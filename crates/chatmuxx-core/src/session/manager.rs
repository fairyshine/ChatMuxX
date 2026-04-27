use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    config::Config,
    provider::{ProviderLaunchRequest, ProviderRegistry},
    session::model::{CloseReason, CreateSessionRequest, SessionSummary},
    state::{
        atomic::{load_json_or_default, save_json},
        files::StatePaths,
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

        let id = SessionId::new();
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

        let mut state = self.load_state().await?;
        state.schema_version = 1;
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
        let state = self.load_state().await?;
        Ok(state.sessions.iter().map(summary_from_record).collect())
    }

    pub async fn close_session(
        &self,
        session_id: &SessionId,
        _reason: CloseReason,
    ) -> Result<SessionRecord> {
        let mut state = self.load_state().await?;
        let Some(record) = state
            .sessions
            .iter_mut()
            .find(|item| &item.id == session_id)
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

        record.status = SessionStatus::Closed;
        record.updated_at = now_string();
        let updated = record.clone();
        for binding in state
            .bindings
            .iter_mut()
            .filter(|binding| binding.session_id == *session_id)
        {
            binding.active = false;
        }
        self.save_state(&state).await?;
        Ok(updated)
    }

    pub async fn mark_session_dead(&self, session_id: &SessionId) -> Result<()> {
        let mut state = self.load_state().await?;
        let Some(record) = state
            .sessions
            .iter_mut()
            .find(|item| &item.id == session_id)
        else {
            return Err(ChatMuxXError::SessionNotFound(session_id.0.clone()));
        };

        record.status = SessionStatus::Dead;
        record.updated_at = now_string();
        for binding in state
            .bindings
            .iter_mut()
            .filter(|binding| binding.session_id == *session_id)
        {
            binding.active = false;
        }
        self.save_state(&state).await
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
        state
            .sessions
            .into_iter()
            .find(|item| &item.id == session_id)
            .ok_or_else(|| ChatMuxXError::SessionNotFound(session_id.0.clone()))
    }

    async fn load_state(&self) -> Result<AppState> {
        load_json_or_default(&self.paths.state).await
    }

    async fn save_state(&self, state: &AppState) -> Result<()> {
        save_json(&self.paths.state, state).await
    }
}

fn validate_workspace(path: &Path) -> Result<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(ChatMuxXError::InvalidWorkspace(path.to_path_buf()))
    }
}

fn summary_from_record(record: &SessionRecord) -> SessionSummary {
    SessionSummary {
        id: record.id.clone(),
        provider: record.provider,
        workspace: record.workspace.clone(),
        status: record.status.clone(),
        display_name: format!("{}:{}", record.provider, record.workspace.display()),
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
    format!("cmux-{safe_display}-{suffix}")
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
    use crate::state::sessions::OwnerId;

    #[test]
    fn window_name_is_stable_and_tmux_friendly() {
        let name = window_name(&SessionId("sess-123456789".to_owned()), "shell:/tmp/a b");

        assert_eq!(name, "cmux-shell--tmp-a-b-123456789");
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

        let summary = summary_from_record(&record);

        assert_eq!(summary.id, record.id);
        assert_eq!(summary.display_name, "shell:/tmp/project");
    }
}
