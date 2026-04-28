use std::{
    collections::HashSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    config::Config,
    provider::{ProviderLaunchRequest, ProviderRegistry},
    session::model::{CloseReason, CreateSessionRequest, PruneSessionsResult, SessionSummary},
    state::{
        atomic::{load_json_or_default, save_json},
        files::StatePaths,
        monitor::MonitorState,
        sessions::{
            AppState, BindingRecord, SessionId, SessionRecord, SessionStatus, TmuxAttachment,
        },
    },
    tmux::TmuxKey,
    tmux::{CreateTmuxWindow, TmuxClient},
    ChatMuxXError, Result,
};

#[derive(Clone)]
pub struct SessionManager {
    config: Config,
    paths: StatePaths,
    providers: ProviderRegistry,
    tmux: TmuxClient,
}

impl SessionManager {
    pub fn new(config: Config, paths: StatePaths) -> Self {
        let providers = ProviderRegistry::from_configs(&config.providers);
        Self {
            config,
            paths,
            providers,
            tmux: TmuxClient::new(),
        }
    }

    pub async fn configure_managed_tmux_session(&self) -> Result<()> {
        self.tmux
            .ensure_managed_session(&self.config.daemon.tmux_session)
            .await
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<SessionRecord> {
        validate_workspace(&req.workspace)?;

        let provider = self.providers.get(req.provider)?;
        let launch = provider.launch_command(&ProviderLaunchRequest {
            workspace: req.workspace.clone(),
            launch_mode: req.launch_mode,
            extra_args: req.extra_args,
        })?;

        self.tmux
            .ensure_managed_session(&self.config.daemon.tmux_session)
            .await?;

        let mut state = self.load_state().await?;
        state.schema_version = 1;
        let id = match req.id {
            Some(id) => {
                validate_session_id(&id)?;
                if state.sessions.iter().any(|session| session.id == id) {
                    return Err(ChatMuxXError::SessionIdAlreadyExists(id.0));
                }
                id
            }
            None => unique_session_id(&state),
        };
        let tmux_window = self
            .tmux
            .create_window(CreateTmuxWindow {
                session_name: self.config.daemon.tmux_session.clone(),
                window_name: window_name(&id, &launch.display_name),
                cwd: req.workspace.clone(),
                command: launch.argv(),
                env: launch.env,
            })
            .await?;

        let now = now_string();
        let record = SessionRecord {
            id: id.clone(),
            provider: req.provider,
            workspace: req.workspace,
            status: SessionStatus::Running,
            tmux: Some(TmuxAttachment {
                session_name: tmux_window.session_name,
                window_id: tmux_window.window_id,
                pane_id: tmux_window.pane_id,
                created_by_chatmuxx: true,
                adopted: false,
            }),
            owner: req.owner,
            created_at: now.clone(),
            updated_at: now,
        };

        state.sessions.push(record.clone());
        if let Some(conversation_id) = req.conversation {
            for binding in state
                .bindings
                .iter_mut()
                .filter(|binding| binding.conversation_id == conversation_id)
            {
                binding.active = false;
            }
            state.bindings.push(BindingRecord {
                conversation_id,
                session_id: id,
                active: true,
            });
        }
        self.save_state(&state).await?;

        Ok(record)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.prune_inactive_sessions().await?;
        let state = self.load_state().await?;
        let active_session_ids = state
            .bindings
            .iter()
            .filter(|binding| binding.active)
            .map(|binding| binding.session_id.clone())
            .collect::<HashSet<_>>();
        Ok(state
            .sessions
            .iter()
            .map(|record| summary_from_record(record, active_session_ids.contains(&record.id)))
            .collect())
    }

    pub async fn resolve_session_id(&self, session_id: &SessionId) -> Result<SessionId> {
        let state = self.load_state().await?;
        resolve_session_id_in_state(&state, session_id)
    }

    pub async fn rename_session(
        &self,
        session_id: &SessionId,
        new_id: SessionId,
    ) -> Result<SessionRecord> {
        validate_session_id(&new_id)?;
        let mut state = self.load_state().await?;
        let session_id = resolve_session_id_in_state(&state, session_id)?;
        if state.sessions.iter().any(|session| session.id == new_id) {
            return Err(ChatMuxXError::SessionIdAlreadyExists(new_id.0));
        }

        let Some(record) = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return Err(ChatMuxXError::SessionNotFound(session_id.0));
        };
        record.id = new_id.clone();
        record.updated_at = now_string();
        let updated = record.clone();

        for binding in state
            .bindings
            .iter_mut()
            .filter(|binding| binding.session_id == session_id)
        {
            binding.session_id = new_id.clone();
        }
        self.save_state(&state).await?;
        self.rename_monitor_record(&session_id, &new_id).await?;
        Ok(updated)
    }

    pub async fn prune_inactive_sessions(&self) -> Result<PruneSessionsResult> {
        let mut state = self.load_state().await?;
        let removed_ids = state
            .sessions
            .iter()
            .filter(|session| matches!(session.status, SessionStatus::Dead | SessionStatus::Closed))
            .map(|session| session.id.clone())
            .collect::<HashSet<_>>();

        if removed_ids.is_empty() {
            return Ok(PruneSessionsResult::default());
        }

        let result = remove_session_records(&mut state, &removed_ids);
        self.save_state(&state).await?;
        self.remove_monitor_records(&removed_ids).await?;

        Ok(result)
    }

    pub async fn prune_missing_tmux_sessions(&self) -> Result<PruneSessionsResult> {
        let mut state = self.load_state().await?;
        let mut removed_ids = HashSet::new();
        for session in state.sessions.iter().filter(|session| {
            matches!(
                session.status,
                SessionStatus::Starting | SessionStatus::Running | SessionStatus::WaitingInput
            )
        }) {
            let Some(tmux) = &session.tmux else {
                continue;
            };
            if !self.tmux.has_pane(&tmux.pane_id).await? {
                removed_ids.insert(session.id.clone());
            }
        }

        if removed_ids.is_empty() {
            return Ok(PruneSessionsResult::default());
        }

        let result = remove_session_records(&mut state, &removed_ids);
        self.save_state(&state).await?;
        self.remove_monitor_records(&removed_ids).await?;
        Ok(result)
    }

    pub async fn close_session(
        &self,
        session_id: &SessionId,
        _reason: CloseReason,
    ) -> Result<SessionRecord> {
        let mut state = self.load_state().await?;
        let session_id = resolve_session_id_in_state(&state, session_id)?;
        let Some(record) = state
            .sessions
            .iter()
            .find(|item| item.id == session_id)
            .cloned()
        else {
            return Err(ChatMuxXError::SessionNotFound(session_id.0.clone()));
        };

        if let Some(tmux) = &record.tmux {
            if tmux.created_by_chatmuxx || tmux.adopted {
                if let Err(err) = self.tmux.close_window(&tmux.window_id).await {
                    if !err.is_missing_tmux_target() {
                        return Err(err);
                    }
                }
            }
        }

        let removed_ids = HashSet::from([session_id]);
        remove_session_records(&mut state, &removed_ids);
        self.save_state(&state).await?;
        self.remove_monitor_records(&removed_ids).await?;
        Ok(record)
    }

    pub async fn mark_session_dead(&self, session_id: &SessionId) -> Result<()> {
        let mut state = self.load_state().await?;
        let session_id = resolve_session_id_in_state(&state, session_id)?;
        if !state.sessions.iter().any(|item| item.id == session_id) {
            return Err(ChatMuxXError::SessionNotFound(session_id.0.clone()));
        }

        let removed_ids = HashSet::from([session_id]);
        remove_session_records(&mut state, &removed_ids);
        self.save_state(&state).await?;
        self.remove_monitor_records(&removed_ids).await
    }

    pub async fn send_text(&self, session_id: &SessionId, text: &str) -> Result<()> {
        let record = self.session_record(session_id).await?;
        let pane_id = pane_id(&record)?;
        self.tmux.send_text(pane_id, text).await
    }

    pub async fn send_text_and_enter(&self, session_id: &SessionId, text: &str) -> Result<()> {
        let record = self.session_record(session_id).await?;
        let pane_id = pane_id(&record)?;
        self.tmux.send_text(pane_id, text).await?;
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        self.tmux.send_key(pane_id, TmuxKey::Enter).await
    }

    pub async fn send_key(&self, session_id: &SessionId, key: TmuxKey) -> Result<()> {
        let record = self.session_record(session_id).await?;
        let pane_id = pane_id(&record)?;
        self.tmux.send_key(pane_id, key).await
    }

    pub async fn capture_pane(&self, session_id: &SessionId) -> Result<String> {
        let record = self.session_record(session_id).await?;
        let pane_id = pane_id(&record)?;
        self.tmux.capture_pane(pane_id).await
    }

    async fn session_record(&self, session_id: &SessionId) -> Result<SessionRecord> {
        let state = self.load_state().await?;
        let session_id = resolve_session_id_in_state(&state, session_id)?;
        state
            .sessions
            .into_iter()
            .find(|item| item.id == session_id)
            .ok_or_else(|| ChatMuxXError::SessionNotFound(session_id.0.clone()))
    }

    async fn load_state(&self) -> Result<AppState> {
        load_json_or_default(&self.paths.state).await
    }

    async fn save_state(&self, state: &AppState) -> Result<()> {
        save_json(&self.paths.state, state).await
    }

    async fn remove_monitor_records(&self, session_ids: &HashSet<SessionId>) -> Result<()> {
        let mut monitor: MonitorState = load_json_or_default(&self.paths.monitor_state).await?;
        for session_id in session_ids {
            monitor.sessions.remove(session_id);
        }
        save_json(&self.paths.monitor_state, &monitor).await
    }

    async fn rename_monitor_record(&self, old_id: &SessionId, new_id: &SessionId) -> Result<()> {
        let mut monitor: MonitorState = load_json_or_default(&self.paths.monitor_state).await?;
        if let Some(state) = monitor.sessions.remove(old_id) {
            monitor.sessions.insert(new_id.clone(), state);
        }
        save_json(&self.paths.monitor_state, &monitor).await
    }
}

fn remove_session_records(
    state: &mut AppState,
    session_ids: &HashSet<SessionId>,
) -> PruneSessionsResult {
    let before_sessions = state.sessions.len();
    state
        .sessions
        .retain(|session| !session_ids.contains(&session.id));

    let before_bindings = state.bindings.len();
    state
        .bindings
        .retain(|binding| !session_ids.contains(&binding.session_id));

    PruneSessionsResult {
        removed_sessions: before_sessions - state.sessions.len(),
        removed_bindings: before_bindings - state.bindings.len(),
    }
}

fn validate_workspace(path: &Path) -> Result<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(ChatMuxXError::InvalidWorkspace(path.to_path_buf()))
    }
}

fn validate_session_id(id: &SessionId) -> Result<()> {
    let text = id.0.as_str();
    let valid_len = (1..=32).contains(&text.chars().count());
    let valid_chars = text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if valid_len && valid_chars {
        Ok(())
    } else {
        Err(ChatMuxXError::InvalidSessionId(id.0.clone()))
    }
}

fn unique_session_id(state: &AppState) -> SessionId {
    for index in 0..1000 {
        let candidate = if index == 0 {
            SessionId::new()
        } else {
            SessionId(format!("{}-{index}", SessionId::new().0))
        };
        if state.sessions.iter().all(|session| session.id != candidate) {
            return candidate;
        }
    }

    SessionId::new()
}

fn resolve_session_id_in_state(state: &AppState, input: &SessionId) -> Result<SessionId> {
    if state.sessions.iter().any(|session| session.id == *input) {
        return Ok(input.clone());
    }

    let matches = state
        .sessions
        .iter()
        .filter(|session| session.id.0.starts_with(&input.0))
        .map(|session| session.id.0.clone())
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(ChatMuxXError::SessionNotFound(input.0.clone())),
        [id] => Ok(SessionId(id.clone())),
        _ => Err(ChatMuxXError::AmbiguousSessionId {
            prefix: input.0.clone(),
            matches: matches.join(", "),
        }),
    }
}

fn summary_from_record(record: &SessionRecord, active: bool) -> SessionSummary {
    SessionSummary {
        id: record.id.clone(),
        provider: record.provider,
        workspace: record.workspace.clone(),
        status: record.status.clone(),
        display_name: format!("{}:{}", record.provider, record.workspace.display()),
        active,
    }
}

fn pane_id(record: &SessionRecord) -> Result<&str> {
    record
        .tmux
        .as_ref()
        .map(|tmux| tmux.pane_id.as_str())
        .ok_or_else(|| ChatMuxXError::SessionNotAttached(record.id.0.clone()))
}

fn window_name(id: &SessionId, display_name: &str) -> String {
    let suffix = id.0.rsplit('-').next().unwrap_or(&id.0);
    let safe_display = display_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("cmx-{safe_display}-{suffix}")
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::provider::ProviderKind;
    use crate::state::sessions::{BindingRecord, OwnerId};

    #[test]
    fn window_name_is_stable_and_tmux_friendly() {
        let name = window_name(&SessionId("sess-123456789".to_owned()), "shell:/tmp/a b");

        assert_eq!(name, "cmx-shell--tmp-a-b-123456789");
    }

    #[test]
    fn generated_session_id_is_short() {
        let id = SessionId::new();

        assert!(id.0.starts_with("s-"));
        assert!(id.0.len() <= 12);
    }

    #[test]
    fn custom_session_id_validation_accepts_friendly_names() {
        assert!(validate_session_id(&SessionId("main".to_owned())).is_ok());
        assert!(validate_session_id(&SessionId("work-1".to_owned())).is_ok());
    }

    #[test]
    fn custom_session_id_validation_rejects_spaces() {
        assert!(validate_session_id(&SessionId("my session".to_owned())).is_err());
    }

    #[test]
    fn summary_uses_provider_and_workspace() {
        let record = SessionRecord {
            id: SessionId("sess-1".to_owned()),
            provider: ProviderKind::Shell,
            workspace: PathBuf::from("/tmp/project"),
            status: SessionStatus::Running,
            tmux: None,
            owner: OwnerId("owner".to_owned()),
            created_at: "1".to_owned(),
            updated_at: "1".to_owned(),
        };

        let summary = summary_from_record(&record, true);

        assert_eq!(summary.id, record.id);
        assert_eq!(summary.display_name, "shell:/tmp/project");
        assert!(summary.active);
    }

    #[test]
    fn remove_session_records_removes_sessions_and_bindings() {
        let remove_id = SessionId("sess-dead".to_owned());
        let keep_id = SessionId("sess-running".to_owned());
        let mut state = AppState {
            schema_version: 1,
            owner: None,
            sessions: vec![
                SessionRecord {
                    id: remove_id.clone(),
                    provider: ProviderKind::Codex,
                    workspace: PathBuf::from("/tmp/dead"),
                    status: SessionStatus::Dead,
                    tmux: None,
                    owner: OwnerId("owner".to_owned()),
                    created_at: "1".to_owned(),
                    updated_at: "1".to_owned(),
                },
                SessionRecord {
                    id: keep_id.clone(),
                    provider: ProviderKind::Codex,
                    workspace: PathBuf::from("/tmp/running"),
                    status: SessionStatus::Running,
                    tmux: None,
                    owner: OwnerId("owner".to_owned()),
                    created_at: "1".to_owned(),
                    updated_at: "1".to_owned(),
                },
            ],
            bindings: vec![
                BindingRecord {
                    conversation_id: "conv-1".to_owned(),
                    session_id: remove_id.clone(),
                    active: false,
                },
                BindingRecord {
                    conversation_id: "conv-2".to_owned(),
                    session_id: keep_id.clone(),
                    active: true,
                },
            ],
            conversations: Vec::new(),
            confirmations: Vec::new(),
        };

        let result = remove_session_records(&mut state, &HashSet::from([remove_id]));

        assert_eq!(
            result,
            PruneSessionsResult {
                removed_sessions: 1,
                removed_bindings: 1
            }
        );
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].id, keep_id);
        assert_eq!(state.bindings.len(), 1);
    }

    #[test]
    fn session_id_resolution_accepts_unique_prefix() {
        let state = AppState {
            schema_version: 1,
            owner: None,
            sessions: vec![SessionRecord {
                id: SessionId("main-session".to_owned()),
                provider: ProviderKind::Codex,
                workspace: PathBuf::from("/tmp/project"),
                status: SessionStatus::Running,
                tmux: None,
                owner: OwnerId("owner".to_owned()),
                created_at: "1".to_owned(),
                updated_at: "1".to_owned(),
            }],
            bindings: Vec::new(),
            conversations: Vec::new(),
            confirmations: Vec::new(),
        };

        let id = resolve_session_id_in_state(&state, &SessionId("main".to_owned())).unwrap();

        assert_eq!(id, SessionId("main-session".to_owned()));
    }

    #[test]
    fn session_id_resolution_rejects_ambiguous_prefix() {
        let state = AppState {
            schema_version: 1,
            owner: None,
            sessions: vec![
                SessionRecord {
                    id: SessionId("main-a".to_owned()),
                    provider: ProviderKind::Codex,
                    workspace: PathBuf::from("/tmp/a"),
                    status: SessionStatus::Running,
                    tmux: None,
                    owner: OwnerId("owner".to_owned()),
                    created_at: "1".to_owned(),
                    updated_at: "1".to_owned(),
                },
                SessionRecord {
                    id: SessionId("main-b".to_owned()),
                    provider: ProviderKind::Codex,
                    workspace: PathBuf::from("/tmp/b"),
                    status: SessionStatus::Running,
                    tmux: None,
                    owner: OwnerId("owner".to_owned()),
                    created_at: "1".to_owned(),
                    updated_at: "1".to_owned(),
                },
            ],
            bindings: Vec::new(),
            conversations: Vec::new(),
            confirmations: Vec::new(),
        };

        assert!(matches!(
            resolve_session_id_in_state(&state, &SessionId("main".to_owned())),
            Err(ChatMuxXError::AmbiguousSessionId { .. })
        ));
    }
}
