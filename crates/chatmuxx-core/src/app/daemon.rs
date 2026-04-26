use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

use tokio::sync::mpsc;

use crate::{
    channel::wechat::{
        conversation_external_id, conversation_id, extract_text, token_ref, WeChatClient,
        WeChatMessage,
    },
    config::{default_config_path, load_config, Config},
    mobile::{parse_mobile_text, MobileCommand, ParsedInbound},
    provider::{LaunchMode, ProviderKind},
    session::{CloseReason, CreateSessionRequest, SessionManager},
    state::{
        accounts::{AccountState, WeChatAccountRecord},
        atomic::{load_json_or_default, save_json},
        files::{ensure_state_dir, StatePaths},
        monitor::{MonitorSessionState, MonitorSourceState, MonitorState},
        sessions::{
            AppState, BindingRecord, ChannelType, ConversationKind, ConversationRecord, OwnerId,
            SessionId, SessionStatus,
        },
    },
    tmux::TmuxKey,
    ChatMuxXError, Result,
};

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

    if !config.wechat.enabled {
        println!("WeChat is disabled in config.");
        return Ok(());
    }

    let accounts: AccountState = load_json_or_default(&paths.accounts).await?;
    if accounts.wechat.is_empty() {
        println!("No WeChat account found. Run `cmx login wechat` first.");
        return Ok(());
    }

    println!("Starting ChatMuxX daemon. Press Ctrl-C to stop.");
    let (tx, mut rx) = mpsc::channel(128);
    let poll_paths = paths.clone();
    let poll_config = config.clone();
    tokio::spawn(async move {
        if let Err(err) = wechat_poll_loop(poll_paths, poll_config, tx).await {
            tracing::error!(error = %err, "wechat polling stopped");
        }
    });

    let mut monitor_interval =
        tokio::time::interval(Duration::from_millis(config.daemon.poll_interval_ms));
    let manager = SessionManager::new(config.clone(), paths.clone());

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("Stopping ChatMuxX daemon.");
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

async fn wechat_poll_loop(
    paths: StatePaths,
    config: Config,
    tx: mpsc::Sender<InboundWeChatText>,
) -> Result<()> {
    let mut backoff = Duration::from_millis(500);

    loop {
        let mut accounts: AccountState = load_json_or_default(&paths.accounts).await?;
        let Some(account) = accounts.wechat.first().cloned() else {
            return Err(ChatMuxXError::WeChatAccountMissing);
        };

        let client = WeChatClient::new(&account.base_url);
        match client
            .get_updates(
                &account.bot_token,
                account.get_updates_buf.as_deref(),
                config.wechat.long_poll_timeout_ms,
            )
            .await
        {
            Ok(response) => {
                backoff = Duration::from_millis(500);
                if let Some(cursor) = response.get_updates_buf {
                    if let Some(stored) = accounts
                        .wechat
                        .iter_mut()
                        .find(|item| item.account_id == account.account_id)
                    {
                        stored.get_updates_buf = Some(cursor);
                        stored.updated_at = now_string();
                    }
                }

                for message in response.msgs {
                    if let Some(event) =
                        persist_inbound_message(&paths, &mut accounts, &account, &message).await?
                    {
                        if tx.send(event).await.is_err() {
                            return Ok(());
                        }
                    }
                }

                save_json(&paths.accounts, &accounts).await?;
            }
            Err(ChatMuxXError::WeChatAccountExpired) => {
                return Err(ChatMuxXError::WeChatAccountExpired)
            }
            Err(err) => {
                tracing::warn!(error = %err, "wechat getupdates failed; backing off");
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
            }
        }
    }
}

async fn persist_inbound_message(
    paths: &StatePaths,
    accounts: &mut AccountState,
    account: &WeChatAccountRecord,
    message: &WeChatMessage,
) -> Result<Option<InboundWeChatText>> {
    let Some(text) = extract_text(message) else {
        return Ok(None);
    };
    let Some(conversation_id) = conversation_id(&account.account_id, message) else {
        return Ok(None);
    };
    let Some(external_id) = conversation_external_id(message) else {
        return Ok(None);
    };
    let from_user_id = message.from_user_id.clone().unwrap_or_default();
    let token_ref = token_ref(&account.account_id, &conversation_id);

    if let Some(context_token) = &message.context_token {
        if let Some(stored) = accounts
            .wechat
            .iter_mut()
            .find(|item| item.account_id == account.account_id)
        {
            stored
                .context_tokens
                .insert(token_ref.clone(), context_token.clone());
            stored.updated_at = now_string();
        }
    }

    let mut state: AppState = load_json_or_default(&paths.state).await?;
    state.schema_version = 1;
    upsert_conversation(
        &mut state,
        ConversationRecord {
            id: conversation_id.clone(),
            channel_type: ChannelType::WeChat,
            account_id: account.account_id.clone(),
            external_conversation_id: external_id,
            kind: if message.group_id.is_some() {
                ConversationKind::Group
            } else {
                ConversationKind::Direct
            },
            latest_context_token_ref: Some(token_ref),
        },
    );
    save_json(&paths.state, &state).await?;

    Ok(Some(InboundWeChatText {
        account_id: account.account_id.clone(),
        conversation_id,
        from_user_id,
        text,
    }))
}

async fn handle_wechat_text(
    paths: &StatePaths,
    config: &Config,
    manager: &SessionManager,
    event: InboundWeChatText,
) -> Result<()> {
    if !is_authorized(paths, config, &event.from_user_id).await? {
        send_wechat_reply(
            paths,
            &event.account_id,
            &event.conversation_id,
            "未授权用户不能使用 ChatMuxX。",
        )
        .await?;
        return Ok(());
    }

    match parse_mobile_text(&event.text, false) {
        ParsedInbound::BridgeCommand(command) => {
            handle_bridge_command(paths, manager, &event, command).await?;
        }
        ParsedInbound::ProviderSlashCommand(text) | ParsedInbound::PlainText(text) => {
            if let Some(session_id) = active_session_id(paths, &event.conversation_id).await? {
                manager.send_text(&session_id, &text).await?;
                manager.send_key(&session_id, TmuxKey::Enter).await?;
            } else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "还没有绑定的会话。发送 `cmx new /你的项目路径 codex` 创建 Codex 会话。",
                )
                .await?;
            }
        }
        ParsedInbound::FlowReply(text) => {
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("暂未进入交互流程，收到：{text}"),
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
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                help_text(),
            )
            .await?;
        }
        MobileCommand::New(args) => {
            let Some(workspace) = args.workspace else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "用法：`cmx new /项目路径 codex`",
                )
                .await?;
                return Ok(());
            };
            let provider = args.provider.unwrap_or(ProviderKind::Codex);
            let session = manager
                .create_session(CreateSessionRequest {
                    provider,
                    workspace,
                    launch_mode: LaunchMode::Fresh,
                    extra_args: args.extra_args,
                    owner: OwnerId("owner-local".to_owned()),
                    conversation: Some(event.conversation_id.clone()),
                })
                .await?;
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("已创建 {provider} 会话：{}", session.id.0),
            )
            .await?;
        }
        MobileCommand::Sessions => {
            let sessions = manager.list_sessions().await?;
            let text = if sessions.is_empty() {
                "没有会话。".to_owned()
            } else {
                sessions
                    .into_iter()
                    .map(|session| {
                        format!(
                            "{}\t{}\t{:?}\t{}",
                            session.id.0,
                            session.provider,
                            session.status,
                            session.workspace.display()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            send_wechat_reply(paths, &event.account_id, &event.conversation_id, &text).await?;
        }
        MobileCommand::Switch { session_id } => {
            let Some(session_id) = session_id else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "用法：`cmx switch <session-id>`",
                )
                .await?;
                return Ok(());
            };
            bind_conversation(paths, &event.conversation_id, &session_id).await?;
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("已切换到：{}", session_id.0),
            )
            .await?;
        }
        MobileCommand::Close { session_id } => {
            let Some(session_id) =
                session_id.or(active_session_id(paths, &event.conversation_id).await?)
            else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "没有可关闭的活动会话。",
                )
                .await?;
                return Ok(());
            };
            manager
                .close_session(&session_id, CloseReason::UserRequested)
                .await?;
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("已关闭：{}", session_id.0),
            )
            .await?;
        }
        MobileCommand::Screenshot => {
            let Some(session_id) = active_session_id(paths, &event.conversation_id).await? else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "没有活动会话。",
                )
                .await?;
                return Ok(());
            };
            let text = manager.capture_pane(&session_id).await?;
            send_wechat_reply(
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
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                "切换 provider 请先用 `cmx close` 关闭当前会话，再用 `cmx new /路径 codex` 创建。",
            )
            .await?;
        }
        MobileCommand::Recover { .. } => {
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                "恢复会话稍后支持。",
            )
            .await?;
        }
        MobileCommand::Unknown { name, .. } => {
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("未知 cmx 命令：{name}"),
            )
            .await?;
        }
    }
    Ok(())
}

async fn send_key_to_active(
    paths: &StatePaths,
    manager: &SessionManager,
    event: &InboundWeChatText,
    key: TmuxKey,
) -> Result<()> {
    let Some(session_id) = active_session_id(paths, &event.conversation_id).await? else {
        send_wechat_reply(
            paths,
            &event.account_id,
            &event.conversation_id,
            "没有活动会话。",
        )
        .await?;
        return Ok(());
    };
    manager.send_key(&session_id, key).await
}

async fn monitor_sessions(paths: &StatePaths, manager: &SessionManager) -> Result<()> {
    let state: AppState = load_json_or_default(&paths.state).await?;
    let mut monitor: MonitorState = load_json_or_default(&paths.monitor_state).await?;
    monitor.schema_version = 1;

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

        let pane = manager.capture_pane(&session.id).await?;
        let pane = trim_for_chat(&pane);
        if pane.trim().is_empty() {
            continue;
        }
        let hash = hash_text(&pane);
        let last_hash = monitor
            .sessions
            .get(&session.id)
            .and_then(|state| state.last_pane_hash.as_deref());
        if last_hash == Some(hash.as_str()) {
            continue;
        }

        monitor.sessions.insert(
            session.id.clone(),
            MonitorSessionState {
                provider: session.provider,
                source: Some(MonitorSourceState {
                    kind: crate::provider::OutputSourceKind::TmuxPane,
                    path: None,
                    offset: None,
                    last_seen_id: None,
                }),
                last_pane_hash: Some(hash),
            },
        );

        if let Some(conversation) = state
            .conversations
            .iter()
            .find(|conversation| conversation.id == binding.conversation_id)
        {
            send_wechat_reply(paths, &conversation.account_id, &conversation.id, &pane).await?;
        }
    }

    save_json(&paths.monitor_state, &monitor).await
}

async fn send_wechat_reply(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
) -> Result<()> {
    let accounts: AccountState = load_json_or_default(&paths.accounts).await?;
    let state: AppState = load_json_or_default(&paths.state).await?;
    let account = accounts
        .wechat
        .iter()
        .find(|account| account.account_id == account_id)
        .ok_or(ChatMuxXError::WeChatAccountMissing)?;
    let conversation = state
        .conversations
        .iter()
        .find(|conversation| conversation.id == conversation_id)
        .ok_or_else(|| {
            ChatMuxXError::WeChatProtocol(format!("conversation not found: {conversation_id}"))
        })?;
    let token_ref = conversation
        .latest_context_token_ref
        .as_ref()
        .ok_or_else(|| ChatMuxXError::WeChatMissingContextToken(conversation_id.to_owned()))?;
    let context_token = account
        .context_tokens
        .get(token_ref)
        .ok_or_else(|| ChatMuxXError::WeChatMissingContextToken(conversation_id.to_owned()))?;

    let client = WeChatClient::new(&account.base_url);
    for part in split_for_chat(text) {
        client
            .send_text(
                &account.bot_token,
                &conversation.external_conversation_id,
                &part,
                context_token,
            )
            .await?;
    }
    Ok(())
}

async fn is_authorized(paths: &StatePaths, config: &Config, from_user_id: &str) -> Result<bool> {
    let state: AppState = load_json_or_default(&paths.state).await?;
    let accounts: AccountState = load_json_or_default(&paths.accounts).await?;
    let configured = config.owner.wechat_user_id.as_deref();
    let persisted = state
        .owner
        .as_ref()
        .and_then(|owner| owner.wechat_user_id.as_deref());
    let login_owner = accounts
        .wechat
        .first()
        .and_then(|account| account.bot_user_id.as_deref());

    let authorized = [configured, persisted, login_owner]
        .into_iter()
        .flatten()
        .any(|allowed| allowed == from_user_id);
    Ok(authorized)
}

async fn active_session_id(paths: &StatePaths, conversation_id: &str) -> Result<Option<SessionId>> {
    let state: AppState = load_json_or_default(&paths.state).await?;
    Ok(state
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.conversation_id == conversation_id && binding.active)
        .and_then(|binding| {
            state
                .sessions
                .iter()
                .find(|session| {
                    session.id == binding.session_id
                        && matches!(
                            session.status,
                            SessionStatus::Running | SessionStatus::WaitingInput
                        )
                })
                .map(|session| session.id.clone())
        }))
}

async fn bind_conversation(
    paths: &StatePaths,
    conversation_id: &str,
    session_id: &SessionId,
) -> Result<()> {
    let mut state: AppState = load_json_or_default(&paths.state).await?;
    if !state
        .sessions
        .iter()
        .any(|session| &session.id == session_id)
    {
        return Err(ChatMuxXError::SessionNotFound(session_id.0.clone()));
    }
    for binding in state
        .bindings
        .iter_mut()
        .filter(|binding| binding.conversation_id == conversation_id)
    {
        binding.active = false;
    }
    state.bindings.push(BindingRecord {
        conversation_id: conversation_id.to_owned(),
        session_id: session_id.clone(),
        active: true,
    });
    save_json(&paths.state, &state).await
}

fn upsert_conversation(state: &mut AppState, record: ConversationRecord) {
    if let Some(existing) = state
        .conversations
        .iter_mut()
        .find(|conversation| conversation.id == record.id)
    {
        *existing = record;
    } else {
        state.conversations.push(record);
    }
}

fn help_text() -> &'static str {
    "ChatMuxX 命令：\ncmx new /项目路径 codex\ncmx sessions\ncmx switch <session-id>\ncmx screenshot\ncmx close\n普通文字会发送给当前 Codex 会话。"
}

fn split_for_chat(text: &str) -> Vec<String> {
    const MAX_CHARS: usize = 1800;
    let text = if text.trim().is_empty() {
        "(empty)"
    } else {
        text
    };
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if current.chars().count() >= MAX_CHARS {
            parts.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn trim_for_chat(text: &str) -> String {
    const MAX_LINES: usize = 60;
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(MAX_LINES);
    lines[start..].join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_text_is_split_by_char_count() {
        let parts = split_for_chat(&"a".repeat(1801));

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].chars().count(), 1800);
    }

    #[test]
    fn trim_for_chat_keeps_tail() {
        let text = (0..70)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = trim_for_chat(&text);

        assert!(trimmed.starts_with("10\n"));
        assert!(trimmed.ends_with("69"));
    }
}
