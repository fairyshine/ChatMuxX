use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    config::{default_config_path, load_config, Config},
    mobile::{parse_mobile_text, MobileCommand, ParsedInbound},
    provider::{
        display::{
            extract_footer_value, extract_terminal_footer, footer_for_display,
            format_display_message, normalize_footer, normalize_pane_text, normalize_raw_pane_text,
            trim_for_chat,
        },
        LaunchMode, ProviderKind,
    },
    session::{CloseReason, CreateSessionRequest, SessionManager, SessionSummary},
    state::{
        accounts::AccountState,
        atomic::{load_json_or_default, save_json},
        files::{ensure_state_dir, StatePaths},
        history::{append_history, HistoryEvent},
        monitor::{MonitorSessionState, MonitorSourceState, MonitorState},
        sessions::{AppState, OwnerId, SessionId, SessionStatus},
    },
    tmux::TmuxKey,
    ChatMuxXError, Result,
};

mod messages;
mod state;
mod text;
mod wechat;

use state::{active_session_id, bind_conversation, is_authorized};
use text::{merge_pending_text, should_flush_pending, text_delta};

#[derive(Clone, Debug)]
struct InboundWeChatText {
    account_id: String,
    conversation_id: String,
    from_user_id: String,
    text: String,
}

pub async fn run(config_path: Option<PathBuf>) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    ensure_state_dir(&paths.root).await?;
    let config = load_config_or_default(config_path).await?;
    let manager = SessionManager::new(config.clone(), paths.clone());
    manager.configure_managed_tmux_session().await?;

    if !config.wechat.enabled {
        println!("{}", messages::WECHAT_DISABLED);
        return Ok(());
    }

    let accounts: AccountState = load_json_or_default(&paths.accounts).await?;
    if accounts.wechat.is_empty() {
        println!("{}", messages::WECHAT_ACCOUNT_MISSING);
        return Ok(());
    }

    println!("{}", messages::DAEMON_STARTING);
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);
    let poll_paths = paths.clone();
    let poll_config = config.clone();
    tokio::spawn(async move {
        if let Err(err) = wechat::poll_loop(poll_paths, poll_config, tx).await {
            tracing::error!(error = %err, "wechat polling stopped");
        }
    });

    let mut monitor_interval =
        tokio::time::interval(Duration::from_millis(config.daemon.poll_interval_ms));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("{}", messages::DAEMON_STOPPING);
                return Ok(());
            }
            Some(event) = rx.recv() => {
                if let Err(err) = handle_wechat_text(&paths, &config, &manager, event).await {
                    tracing::error!(error = %err, "failed to handle wechat event");
                }
            }
            _ = monitor_interval.tick() => {
                if let Err(err) = monitor_sessions(&paths, &manager).await {
                    tracing::warn!(error = %err, "monitor tick failed");
                }
            }
        }
    }
}

async fn handle_wechat_text(
    paths: &StatePaths,
    config: &Config,
    manager: &SessionManager,
    event: InboundWeChatText,
) -> Result<()> {
    if !is_authorized(paths, config, &event.from_user_id).await? {
        wechat::send_reply(paths, &event.account_id, &event.conversation_id, messages::UNAUTHORIZED)
        .await?;
        return Ok(());
    }

    match parse_mobile_text(&event.text, false) {
        ParsedInbound::BridgeCommand(command) => {
            handle_bridge_command(paths, manager, &event, command).await?;
        }
        ParsedInbound::ProviderSlashCommand(text) | ParsedInbound::PlainText(text) => {
            if let Some(session_id) = active_session_id(paths, &event.conversation_id).await? {
                manager.send_text_and_enter(&session_id, &text).await?;
            } else {
                wechat::send_reply(paths, &event.account_id, &event.conversation_id, messages::NO_BOUND_SESSION)
                .await?;
            }
        }
        ParsedInbound::FlowReply(text) => {
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::flow_not_active(&text),
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_bridge_command(
    paths: &StatePaths,
    manager: &SessionManager,
    event: &InboundWeChatText,
    command: MobileCommand,
) -> Result<()> {
    match command {
        MobileCommand::Help => {
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                help_text(),
            )
            .await?;
        }
        MobileCommand::New(args) => {
            let Some(workspace) = args.workspace else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::USAGE_NEW,
                )
                .await?;
                return Ok(());
            };
            if workspace_looks_like_option(&workspace) {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::NEW_ID_POSITION,
                )
                .await?;
                return Ok(());
            }
            let provider = args.provider.unwrap_or(ProviderKind::Codex);
            let session = manager
                .create_session(CreateSessionRequest {
                    id: args.id,
                    provider,
                    workspace,
                    launch_mode: LaunchMode::Fresh,
                    extra_args: args.extra_args,
                    owner: OwnerId("owner-local".to_owned()),
                    conversation: Some(event.conversation_id.clone()),
                })
                .await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::session_created(provider, &session.id),
            )
            .await?;
        }
        MobileCommand::Sessions => {
            let sessions = manager.list_sessions().await?;
            let current_session_id = active_session_id(paths, &event.conversation_id).await?;
            let text = format_session_list(&sessions, current_session_id.as_ref());
            wechat::send_reply(paths, &event.account_id, &event.conversation_id, &text).await?;
        }
        MobileCommand::Switch { session_id } => {
            let Some(session_id) = session_id else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::USAGE_SWITCH,
                )
                .await?;
                return Ok(());
            };
            let session_id = manager.resolve_session_id(&session_id).await?;
            bind_conversation(paths, &event.conversation_id, &session_id).await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::session_switched(&session_id),
            )
            .await?;
        }
        MobileCommand::Close { session_id } => {
            let Some(session_id) =
                session_id.or(active_session_id(paths, &event.conversation_id).await?)
            else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::NO_ACTIVE_TO_CLOSE,
                )
                .await?;
                return Ok(());
            };
            manager
                .close_session(&session_id, CloseReason::UserRequested)
                .await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::session_closed(&session_id),
            )
            .await?;
        }
        MobileCommand::Rename { session_id, new_id } => {
            let Some(new_id) = new_id else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::USAGE_RENAME,
                )
                .await?;
                return Ok(());
            };
            let Some(session_id) =
                session_id.or(active_session_id(paths, &event.conversation_id).await?)
            else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::NO_ACTIVE_TO_RENAME,
                )
                .await?;
                return Ok(());
            };
            let record = manager.rename_session(&session_id, new_id).await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::session_renamed(&record.id),
            )
            .await?;
        }
        MobileCommand::Prune => {
            let result = manager.prune_inactive_sessions().await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::sessions_pruned(result.removed_sessions, result.removed_bindings),
            )
            .await?;
        }
        MobileCommand::Screenshot => {
            let Some(session_id) = active_session_id(paths, &event.conversation_id).await? else {
                wechat::send_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    messages::NO_ACTIVE_SESSION,
                )
                .await?;
                return Ok(());
            };
            let text = manager.capture_pane(&session_id).await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &trim_for_chat(&text),
            )
            .await?;
        }
        MobileCommand::Esc => send_key_to_active(paths, manager, event, TmuxKey::Escape).await?,
        MobileCommand::Enter => send_key_to_active(paths, manager, event, TmuxKey::Enter).await?,
        MobileCommand::Interrupt => {
            send_key_to_active(paths, manager, event, TmuxKey::CtrlC).await?
        }
        MobileCommand::Provider { .. } => {
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                messages::PROVIDER_SWITCH_NOT_SUPPORTED,
            )
            .await?;
        }
        MobileCommand::Recover { .. } => {
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                messages::RECOVER_NOT_SUPPORTED,
            )
            .await?;
        }
        MobileCommand::Unknown { name, .. } => {
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::unknown_command(&name),
            )
            .await?;
        }
    }
    Ok(())
}

fn workspace_looks_like_option(workspace: &Path) -> bool {
    workspace
        .as_os_str()
        .to_str()
        .is_some_and(|value| value.starts_with('-'))
}

async fn send_key_to_active(
    paths: &StatePaths,
    manager: &SessionManager,
    event: &InboundWeChatText,
    key: TmuxKey,
) -> Result<()> {
    let Some(session_id) = active_session_id(paths, &event.conversation_id).await? else {
        wechat::send_reply(
            paths,
            &event.account_id,
            &event.conversation_id,
            messages::NO_ACTIVE_SESSION,
        )
        .await?;
        return Ok(());
    };
    manager.send_key(&session_id, key).await
}

async fn monitor_sessions(paths: &StatePaths, manager: &SessionManager) -> Result<()> {
    let pruned = manager.prune_missing_tmux_sessions().await?;
    if pruned.removed_sessions > 0 || pruned.removed_bindings > 0 {
        tracing::debug!(
            removed_sessions = pruned.removed_sessions,
            removed_bindings = pruned.removed_bindings,
            "pruned sessions whose tmux panes disappeared"
        );
    }

    let state: AppState = load_json_or_default(&paths.state).await?;
    let mut monitor: MonitorState = load_json_or_default(&paths.monitor_state).await?;
    monitor.schema_version = 1;
    let now_ms = now_millis();

    for binding in state.bindings.iter().filter(|binding| binding.active) {
        let Some(session) = state.sessions.iter().find(|session| {
            session.id == binding.session_id
                && matches!(
                    session.status,
                    SessionStatus::Running | SessionStatus::WaitingInput
                )
        }) else {
            continue;
        };

        let pane = match manager.capture_pane(&session.id).await {
            Ok(pane) => pane,
            Err(err) if err.is_missing_tmux_target() => {
                tracing::debug!(
                    session_id = %session.id.0,
                    error = %err,
                    "tmux target disappeared; marking session dead"
                );
                monitor.sessions.remove(&session.id);
                if let Err(mark_err) = manager.mark_session_dead(&session.id).await {
                    if !matches!(mark_err, ChatMuxXError::SessionNotFound(_)) {
                        return Err(mark_err);
                    }
                }
                continue;
            }
            Err(err) => return Err(err),
        };
        let pane = trim_for_chat(&pane);
        if pane.trim().is_empty() {
            continue;
        }
        let hash = hash_text(&pane);
        let previous_monitor = monitor.sessions.get(&session.id);
        let mut next_monitor = monitor_session_state_from_previous(
            previous_monitor,
            session.provider,
            Some(hash.clone()),
            Some(pane.clone()),
        );

        let pane_changed = previous_monitor.and_then(|state| state.last_pane_hash.as_deref())
            != Some(hash.as_str());
        if pane_changed {
            let previous_text = previous_monitor.and_then(|state| state.last_pane_text.as_deref());
            let footer_text = extract_terminal_footer(&pane, session.provider);
            if let Some(footer) = footer_text.as_deref().and_then(normalize_footer) {
                next_monitor.last_footer_text = Some(footer);
            }
            next_monitor.last_status_text = footer_text
                .as_deref()
                .and_then(|footer| extract_footer_value(footer, "状态"))
                .or(next_monitor.last_status_text);
            if let Some(raw_delta) = raw_pane_delta(previous_text, &pane) {
                append_history(
                    &paths.history,
                    HistoryEvent::ProviderOutput {
                        session_id: session.id.clone(),
                        text: raw_delta,
                        at: now_string(),
                    },
                )
                .await?;
            }

            if let Some(delta) = pane_delta(previous_text, &pane, session.provider) {
                next_monitor.pending_display_text =
                    merge_pending_text(next_monitor.pending_display_text.take(), &delta);
                if next_monitor.pending_since_ms.is_none() {
                    next_monitor.pending_since_ms = Some(now_ms);
                }
                if footer_text.is_some() {
                    next_monitor.pending_footer_text = footer_text.clone();
                }
            }
        }

        let delivery = if should_flush_pending(&next_monitor, now_ms) {
            let body = next_monitor.pending_display_text.take().unwrap_or_default();
            let footer = footer_for_display(
                next_monitor.pending_footer_text.as_deref(),
                next_monitor.last_footer_text.as_deref(),
            );
            let message = format_display_message(body, footer.as_deref());
            if let Some(pending_footer) = next_monitor.pending_footer_text.as_deref() {
                next_monitor.last_footer_text = normalize_footer(pending_footer);
            }
            next_monitor.pending_footer_text = None;
            next_monitor.pending_since_ms = None;
            next_monitor.last_delivery_at_ms = Some(now_ms);
            Some(message)
        } else {
            None
        };

        if let Some(conversation) = state
            .conversations
            .iter()
            .find(|conversation| conversation.id == binding.conversation_id)
        {
            if let Some(message) = delivery {
                wechat::send_deduped_reply(
                    paths,
                    &conversation.account_id,
                    &conversation.id,
                    &message,
                )
                .await?;
            }
        }

        monitor.sessions.insert(session.id.clone(), next_monitor);
    }

    save_json(&paths.monitor_state, &monitor).await
}

fn monitor_session_state_from_previous(
    previous: Option<&MonitorSessionState>,
    provider: ProviderKind,
    last_pane_hash: Option<String>,
    last_pane_text: Option<String>,
) -> MonitorSessionState {
    MonitorSessionState {
        provider,
        source: Some(MonitorSourceState {
            kind: crate::provider::OutputSourceKind::TmuxPane,
            path: None,
            offset: None,
            last_seen_id: None,
        }),
        last_pane_hash,
        last_pane_text,
        last_status_text: previous.and_then(|state| state.last_status_text.clone()),
        last_footer_text: previous.and_then(|state| state.last_footer_text.clone()),
        pending_display_text: previous.and_then(|state| state.pending_display_text.clone()),
        pending_footer_text: previous.and_then(|state| state.pending_footer_text.clone()),
        pending_since_ms: previous.and_then(|state| state.pending_since_ms),
        last_delivery_at_ms: previous.and_then(|state| state.last_delivery_at_ms),
    }
}

fn help_text() -> &'static str {
    messages::HELP
}

fn format_session_list(
    sessions: &[SessionSummary],
    current_session_id: Option<&SessionId>,
) -> String {
    if sessions.is_empty() {
        return messages::SESSION_LIST_EMPTY.to_owned();
    }

    let mut lines = vec![messages::session_list_header(sessions.len())];
    for session in sessions {
        let marker = if current_session_id == Some(&session.id) {
            messages::SESSION_MARKER_CURRENT
        } else if current_session_id.is_none() && session.active {
            messages::SESSION_MARKER_ACTIVE
        } else {
            ""
        };
        lines.push(messages::session_list_item(session, marker));
    }
    lines.join("\n")
}

fn pane_delta(previous: Option<&str>, current: &str, provider: ProviderKind) -> Option<String> {
    text_delta(previous, current, |text| {
        normalize_pane_text(text, provider)
    })
}

fn raw_pane_delta(previous: Option<&str>, current: &str) -> Option<String> {
    text_delta(previous, current, normalize_raw_pane_text)
}

fn hash_text(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish().to_string()
}

async fn load_config_or_default(path: Option<PathBuf>) -> Result<Config> {
    let path = match path {
        Some(path) => path,
        None => default_config_path()?,
    };
    match load_config(&path).await {
        Ok(config) => Ok(config),
        Err(ChatMuxXError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Config::default())
        }
        Err(err) => Err(err),
    }
}

fn now_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_list_is_formatted_for_chat() {
        let text = format_session_list(
            &[SessionSummary {
                id: SessionId("sess-1".to_owned()),
                provider: ProviderKind::Codex,
                workspace: PathBuf::from("/tmp/project"),
                status: SessionStatus::Running,
                display_name: "codex:/tmp/project".to_owned(),
                active: true,
            }],
            Some(&SessionId("sess-1".to_owned())),
        );

        assert_eq!(
            text,
            "会话列表：1 个\n- 当前 sess-1 · codex · Running\n  /tmp/project"
        );
    }

    #[test]
    fn session_list_can_show_globally_active_session() {
        let text = format_session_list(
            &[SessionSummary {
                id: SessionId("sess-1".to_owned()),
                provider: ProviderKind::Codex,
                workspace: PathBuf::from("/tmp/project"),
                status: SessionStatus::Running,
                display_name: "codex:/tmp/project".to_owned(),
                active: true,
            }],
            None,
        );

        assert_eq!(
            text,
            "会话列表：1 个\n- 活动 sess-1 · codex · Running\n  /tmp/project"
        );
    }


}
