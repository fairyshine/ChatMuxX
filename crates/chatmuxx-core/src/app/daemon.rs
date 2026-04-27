use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::sync::mpsc;

use crate::{
    channel::wechat::{
        conversation_external_id, conversation_id, extract_text, is_group_message, token_ref,
        WeChatClient, WeChatMessage,
    },
    config::{default_config_path, load_config, Config},
    mobile::{parse_mobile_text, MobileCommand, ParsedInbound},
    provider::{LaunchMode, ProviderKind},
    session::{CloseReason, CreateSessionRequest, SessionManager, SessionSummary},
    state::{
        accounts::{AccountState, WeChatAccountRecord},
        atomic::{load_json_or_default, save_json},
        files::{ensure_state_dir, StatePaths},
        history::{append_history, recent_outbound_texts, HistoryEvent},
        monitor::{MonitorSessionState, MonitorSourceState, MonitorState},
        sessions::{
            AppState, BindingRecord, ChannelType, ConversationKind, ConversationRecord, OwnerId,
            SessionId, SessionStatus,
        },
    },
    tmux::TmuxKey,
    ChatMuxXError, Result,
};

const DISPLAY_FLUSH_INTERVAL_MS: u64 = 3_000;
const COMPACT_DEDUPE_MIN_CHARS: usize = 30;

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
                        save_json(&paths.accounts, &accounts).await?;
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
            kind: if is_group_message(message) {
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
                manager.send_text_and_enter(&session_id, &text).await?;
            } else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "还没有绑定的会话。发送 `cmx new /你的项目路径 claude` 创建 Claude 会话，或把 `claude` 换成 `codex` / `shell`。",
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
                    "用法：`cmx new --id main /项目路径 claude`，provider 也可以是 `codex` 或 `shell`。",
                )
                .await?;
                return Ok(());
            };
            if workspace_looks_like_option(&workspace) {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "项目路径解析失败：`--id` 要放在 `cmx new` 后面，示例：`cmx new --id main /项目路径 claude`。",
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
            let current_session_id = active_session_id(paths, &event.conversation_id).await?;
            let text = format_session_list(&sessions, current_session_id.as_ref());
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
            let session_id = manager.resolve_session_id(&session_id).await?;
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
                &format!("已关闭并清理：{}", session_id.0),
            )
            .await?;
        }
        MobileCommand::Rename { session_id, new_id } => {
            let Some(new_id) = new_id else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "用法：`cmx rename [session-id] <new-id>`",
                )
                .await?;
                return Ok(());
            };
            let Some(session_id) =
                session_id.or(active_session_id(paths, &event.conversation_id).await?)
            else {
                send_wechat_reply(
                    paths,
                    &event.account_id,
                    &event.conversation_id,
                    "没有可重命名的活动会话。",
                )
                .await?;
                return Ok(());
            };
            let record = manager.rename_session(&session_id, new_id).await?;
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!("已重命名会话：{}", record.id.0),
            )
            .await?;
        }
        MobileCommand::Prune => {
            let result = manager.prune_inactive_sessions().await?;
            send_wechat_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &format!(
                    "已清理 {} 个 Dead/Closed 会话，{} 条绑定记录。",
                    result.removed_sessions, result.removed_bindings
                ),
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
                "切换 provider 请先用 `cmx close` 关闭当前会话，再用 `cmx new /路径 claude` 或 `cmx new /路径 codex` 创建。",
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
                next_monitor.last_status_text = footer_text
                    .as_deref()
                    .and_then(|footer| extract_footer_value(footer, "状态"))
                    .or(next_monitor.last_status_text);
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
                send_wechat_deduped_reply(
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

async fn send_wechat_reply(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
) -> Result<()> {
    send_wechat_reply_inner(paths, account_id, conversation_id, text, false).await
}

async fn send_wechat_deduped_reply(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
) -> Result<()> {
    send_wechat_reply_inner(paths, account_id, conversation_id, text, true).await
}

async fn send_wechat_reply_inner(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
    dedupe: bool,
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
    let mut recent = recent_outbound_texts(&paths.history, conversation_id, 20).await?;
    let text = if dedupe {
        let Some(text) = suppress_recent_outbound_text(text, &recent) else {
            return Ok(());
        };
        text
    } else {
        text.trim_matches('\n').trim_end().to_owned()
    };
    for part in split_for_chat(&text) {
        if dedupe && is_redundant_outbound(&part, &recent) {
            continue;
        }
        client
            .send_text(
                &account.bot_token,
                &conversation.external_conversation_id,
                &part,
                context_token,
            )
            .await?;
        recent.insert(0, part.clone());
        append_history(
            &paths.history,
            HistoryEvent::OutboundText {
                conversation_id: conversation_id.to_owned(),
                text: part,
                at: now_string(),
            },
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
    "ChatMuxX 命令：\ncmx new --id main /项目路径 claude\ncmx new --id codex /项目路径 codex\ncmx new --id sh /项目路径 shell\ncmx sessions\ncmx switch <session-id>\ncmx rename [session-id] <new-id>\ncmx screenshot\ncmx close\ncmx prune\n普通文字会发送给当前会话。"
}

fn format_session_list(
    sessions: &[SessionSummary],
    current_session_id: Option<&SessionId>,
) -> String {
    if sessions.is_empty() {
        return "会话列表：没有会话。".to_owned();
    }

    let mut lines = vec![format!("会话列表：{} 个", sessions.len())];
    for session in sessions {
        let marker = if current_session_id == Some(&session.id) {
            "当前 "
        } else if current_session_id.is_none() && session.active {
            "活动 "
        } else {
            ""
        };
        lines.push(format!(
            "- {marker}{} · {} · {:?}\n  {}",
            session.id.0,
            session.provider,
            session.status,
            session.workspace.display()
        ));
    }
    lines.join("\n")
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

fn is_redundant_outbound(candidate: &str, recent: &[String]) -> bool {
    let candidate = normalize_for_history_dedupe(candidate);
    if candidate.is_empty() {
        return true;
    }
    let candidate_compact = compact_for_history_dedupe(&candidate);

    recent.iter().any(|previous| {
        let previous = normalize_for_history_dedupe(previous);
        if previous == candidate
            || previous.contains(&candidate)
            || (candidate.contains(&previous) && previous.chars().count() > 80)
        {
            return true;
        }

        let previous_compact = compact_for_history_dedupe(&previous);
        candidate_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
            && (previous_compact == candidate_compact
                || previous_compact.contains(&candidate_compact)
                || (candidate_compact.contains(&previous_compact)
                    && previous_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS))
    })
}

fn suppress_recent_outbound_text(candidate: &str, recent: &[String]) -> Option<String> {
    let mut text = candidate.trim_matches('\n').trim_end().to_owned();
    for previous in recent {
        let next = remove_recent_overlap(&text, previous);
        if normalize_for_history_dedupe(&next).is_empty() {
            return None;
        }
        text = next;
    }

    Some(text)
}

fn remove_recent_overlap(candidate: &str, previous: &str) -> String {
    let candidate_lines = normalized_history_lines(candidate);
    let previous_lines = normalized_history_lines(previous);
    if candidate_lines.is_empty() || previous_lines.is_empty() {
        return candidate.trim_matches('\n').trim_end().to_owned();
    }

    let candidate_compact = compact_lines(&candidate_lines);
    let previous_compact = compact_lines(&previous_lines);
    if candidate_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
        && previous_compact.contains(&candidate_compact)
    {
        return String::new();
    }

    if contains_line_window(&previous_lines, &candidate_lines) {
        return String::new();
    }

    if let Some(start) = find_line_window(&candidate_lines, &previous_lines) {
        let mut kept = Vec::new();
        kept.extend(candidate_lines[..start].iter().cloned());
        kept.extend(
            candidate_lines[start + previous_lines.len()..]
                .iter()
                .cloned(),
        );
        return kept.join("\n");
    }

    let max_overlap = candidate_lines.len().min(previous_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == candidate_lines[..overlap] {
            return candidate_lines[overlap..].join("\n");
        }
    }

    for prefix_len in (1..candidate_lines.len()).rev() {
        if contains_line_window(&previous_lines, &candidate_lines[..prefix_len]) {
            return candidate_lines[prefix_len..].join("\n");
        }

        let prefix_compact = compact_lines(&candidate_lines[..prefix_len]);
        if prefix_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
            && previous_compact.contains(&prefix_compact)
        {
            return candidate_lines[prefix_len..].join("\n");
        }
    }

    candidate.trim_matches('\n').trim_end().to_owned()
}

fn normalized_history_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn contains_line_window(haystack: &[String], needle: &[String]) -> bool {
    find_line_window(haystack, needle).is_some()
}

fn find_line_window(haystack: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn merge_pending_text(previous: Option<String>, delta: &str) -> Option<String> {
    let delta = delta.trim_matches('\n').trim_end();
    if delta.trim().is_empty() {
        return previous;
    }

    let Some(previous) = previous.filter(|text| !text.trim().is_empty()) else {
        return Some(delta.to_owned());
    };

    Some(merge_overlapping_text(&previous, delta))
}

fn merge_overlapping_text(previous: &str, next: &str) -> String {
    let previous = previous.trim_matches('\n').trim_end();
    let next = next.trim_matches('\n').trim_end();

    if previous.is_empty() {
        return next.to_owned();
    }
    if next.is_empty() || previous == next || previous.contains(next) {
        return previous.to_owned();
    }
    if next.contains(previous) {
        return next.to_owned();
    }

    let previous_lines = previous.lines().collect::<Vec<_>>();
    let next_lines = next.lines().collect::<Vec<_>>();
    let max_overlap = previous_lines.len().min(next_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == next_lines[..overlap] {
            let suffix = next_lines[overlap..].join("\n");
            if suffix.trim().is_empty() {
                return previous.to_owned();
            }
            return format!("{previous}\n{suffix}");
        }
    }

    let previous_chars = previous.chars().collect::<Vec<_>>();
    let next_chars = next.chars().collect::<Vec<_>>();
    let max_char_overlap = previous_chars.len().min(next_chars.len()).min(500);
    for overlap in (20..=max_char_overlap).rev() {
        if previous_chars[previous_chars.len() - overlap..] == next_chars[..overlap] {
            let suffix = next_chars[overlap..].iter().collect::<String>();
            if suffix.trim().is_empty() {
                return previous.to_owned();
            }
            return format!("{previous}{suffix}");
        }
    }

    format!("{previous}\n{next}")
}

fn should_flush_pending(state: &MonitorSessionState, now_ms: u64) -> bool {
    if state
        .pending_display_text
        .as_deref()
        .is_none_or(|text| text.trim().is_empty())
    {
        return false;
    }

    state.pending_since_ms.is_some_and(|pending_since| {
        now_ms.saturating_sub(pending_since) >= DISPLAY_FLUSH_INTERVAL_MS
    })
}

fn footer_for_display(pending_footer: Option<&str>, last_footer: Option<&str>) -> Option<String> {
    pending_footer
        .and_then(normalize_footer)
        .or_else(|| last_footer.and_then(normalize_footer))
}

fn normalize_footer(footer: &str) -> Option<String> {
    let footer = footer.trim();
    if footer.is_empty() {
        None
    } else {
        Some(footer.to_owned())
    }
}

fn format_display_message(body: String, footer: Option<&str>) -> String {
    let body = body.trim_matches('\n').trim_end();
    match footer.map(str::trim).filter(|footer| !footer.is_empty()) {
        Some(footer) if !body.is_empty() => format!("{body}\n\n——\n{footer}"),
        Some(footer) => footer.to_owned(),
        None => body.to_owned(),
    }
}

fn normalize_for_history_dedupe(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_for_history_dedupe(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_whitespace() && !is_separator_char(*ch))
        .collect()
}

fn compact_lines(lines: &[String]) -> String {
    compact_for_history_dedupe(&lines.join("\n"))
}

fn trim_for_chat(text: &str) -> String {
    const MAX_LINES: usize = 60;
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(MAX_LINES);
    lines[start..].join("\n")
}

fn pane_delta(previous: Option<&str>, current: &str, provider: ProviderKind) -> Option<String> {
    text_delta(previous, current, |text| {
        normalize_pane_text(text, provider)
    })
}

fn raw_pane_delta(previous: Option<&str>, current: &str) -> Option<String> {
    text_delta(previous, current, normalize_raw_pane_text)
}

fn text_delta(
    previous: Option<&str>,
    current: &str,
    normalize: impl Fn(&str) -> String,
) -> Option<String> {
    let current = normalize(current);
    if current.trim().is_empty() {
        return None;
    }

    let Some(previous) = previous else {
        return Some(current);
    };
    let previous = normalize(previous);

    if current == previous {
        return None;
    }
    let current_compact = compact_for_history_dedupe(&current);
    let previous_compact = compact_for_history_dedupe(&previous);
    if current_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
        && (current_compact == previous_compact || previous_compact.contains(&current_compact))
    {
        return None;
    }
    if let Some(delta) = current.strip_prefix(&previous) {
        return non_empty_delta(delta);
    }

    let previous_lines = previous.lines().collect::<Vec<_>>();
    let current_lines = current.lines().collect::<Vec<_>>();
    let max_overlap = previous_lines.len().min(current_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == current_lines[..overlap] {
            return non_empty_delta(&current_lines[overlap..].join("\n"));
        }
    }

    if let Some(prefix_len) =
        longest_current_prefix_seen_in_previous(&previous_lines, &current_lines)
    {
        return non_empty_delta(&current_lines[prefix_len..].join("\n"));
    }

    Some(current)
}

fn longest_current_prefix_seen_in_previous(
    previous_lines: &[&str],
    current_lines: &[&str],
) -> Option<usize> {
    let max_len = previous_lines.len().min(current_lines.len());
    for len in (1..=max_len).rev() {
        let prefix = &current_lines[..len];
        let prefix_compact = compact_for_history_dedupe(&prefix.join("\n"));
        if len < 3 && prefix_compact.chars().count() < COMPACT_DEDUPE_MIN_CHARS {
            continue;
        }
        if previous_lines.windows(len).any(|window| window == prefix) {
            return Some(len);
        }
    }

    None
}

fn normalize_raw_pane_text(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_pane_text(text: &str, provider: ProviderKind) -> String {
    if provider == ProviderKind::Shell {
        return normalize_shell_pane_text(text);
    }

    let raw_lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let footer_start = raw_lines.len().saturating_sub(8);
    let lines = raw_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| match provider {
            ProviderKind::Codex | ProviderKind::Claude
                if is_noisy_agent_status_line(line)
                    || is_noisy_agent_ui_line(line)
                    || (provider == ProviderKind::Claude && is_noisy_claude_ui_line(line))
                    || (index >= footer_start && is_terminal_footer_line(line)) =>
            {
                None
            }
            _ => Some(*line),
        })
        .collect::<Vec<_>>();
    lines.join("\n")
}

fn normalize_shell_pane_text(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !is_noisy_shell_line(line))
        .filter(|line| !is_shell_prompt_or_echo_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_noisy_shell_line(line: &str) -> bool {
    let line = line.trim();
    line == "The default interactive shell is now zsh."
        || line == "To update your account to use zsh, please run `chsh -s /bin/zsh`."
        || line.starts_with("For more details, please visit https://support.apple.com/kb/")
}

fn is_shell_prompt_or_echo_line(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }

    if line.starts_with("cmx> ") || line == "cmx>" {
        return true;
    }

    if starts_with_prompt_marker(line) {
        return true;
    }

    if let Some(index) = prompt_marker_index(line) {
        let prefix = line[..index].trim();
        return !prefix.is_empty()
            && prefix.chars().count() <= 80
            && looks_like_shell_prompt_prefix(prefix);
    }

    let Some(last) = line.chars().last() else {
        return false;
    };
    matches!(last, '$' | '%' | '#')
        && line.chars().count() <= 80
        && looks_like_shell_prompt_prefix(line.trim_end_matches(['$', '%', '#']).trim())
}

fn starts_with_prompt_marker(line: &str) -> bool {
    ["$ ", "% ", "# ", "> "]
        .iter()
        .any(|marker| line.starts_with(marker))
}

fn prompt_marker_index(line: &str) -> Option<usize> {
    ["$ ", "% ", "# ", "> "]
        .iter()
        .filter_map(|marker| line.find(marker))
        .min()
}

fn looks_like_shell_prompt_prefix(prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    prefix.starts_with("bash-")
        || prefix.starts_with("sh-")
        || prefix.contains('@')
        || prefix.contains(':')
        || prefix.contains('~')
        || prefix.contains('/')
        || prefix.ends_with('>')
}

fn extract_terminal_footer(text: &str, provider: ProviderKind) -> Option<String> {
    match provider {
        ProviderKind::Shell => None,
        ProviderKind::Codex | ProviderKind::Claude => {
            let status = extract_provider_status(text, provider);
            let context = extract_provider_footer_context(text, provider);
            format_terminal_footer(status.as_deref(), context.as_deref())
        }
    }
}

fn format_terminal_footer(status: Option<&str>, context: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
        parts.push(format!("状态：{}", status.trim()));
    }
    if let Some(context) = context.filter(|value| !value.trim().is_empty()) {
        parts.push(context.trim().to_owned());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

fn extract_footer_value(footer: &str, label: &str) -> Option<String> {
    footer.split('·').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(label)
            .and_then(|value| value.trim().strip_prefix('：').or(Some(value.trim())))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn extract_provider_status(text: &str, provider: ProviderKind) -> Option<String> {
    match provider {
        ProviderKind::Shell => None,
        ProviderKind::Codex | ProviderKind::Claude => text
            .lines()
            .rev()
            .find(|line| is_noisy_agent_status_line(line))
            .and_then(clean_status_line),
    }
}

fn extract_provider_footer_context(text: &str, provider: ProviderKind) -> Option<String> {
    match provider {
        ProviderKind::Shell => None,
        ProviderKind::Codex | ProviderKind::Claude => text
            .lines()
            .rev()
            .take(12)
            .find_map(extract_footer_context_from_line),
    }
}

fn extract_footer_context_from_line(line: &str) -> Option<String> {
    let cleaned = clean_footer_line(line);
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.contains('·') && contains_model_token(&cleaned) {
        return Some(cleaned);
    }

    extract_model_from_line(&cleaned)
}

fn clean_footer_line(line: &str) -> String {
    line.trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '│' | '┃' | '┆' | '┊' | '║' | '╎' | '╏' | '─' | '━' | ' ' | '\t'
            )
        })
        .trim()
        .to_owned()
}

fn extract_model_from_line(line: &str) -> Option<String> {
    let tokens = line
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '|' | '│' | '·' | ',' | ';'))
        .map(clean_model_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    for (index, token) in tokens.iter().enumerate() {
        let lower = token.to_ascii_lowercase();
        if lower == "model" || lower == "model:" || lower == "模型" || lower == "模型:" {
            if let Some(next) = tokens.get(index + 1) {
                return Some((*next).to_owned());
            }
        }
        if is_model_token(&lower) {
            return Some((*token).to_owned());
        }
    }

    None
}

fn clean_model_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            ':' | '=' | '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>' | '"' | '\''
        )
    })
}

fn is_model_token(lower: &str) -> bool {
    lower.contains("gpt-")
        || lower.contains("codex")
        || lower.starts_with("claude-")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
}

fn contains_model_token(text: &str) -> bool {
    text.split(|ch: char| ch.is_whitespace() || matches!(ch, '|' | '│' | '·' | ',' | ';'))
        .map(clean_model_token)
        .map(str::to_ascii_lowercase)
        .any(|token| is_model_token(&token))
}

fn is_terminal_footer_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.contains("model")
        || lower.contains("模型")
        || lower.contains("gpt-")
        || lower.contains("claude-")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
        || lower.contains("tokens")
        || lower.contains("context")
        || lower.contains("上下文")
}

fn clean_status_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let without_spinner = trimmed
        .strip_prefix(|ch| {
            matches!(
                ch,
                '⠋' | '⠙' | '⠹' | '⠸' | '⠼' | '⠴' | '⠦' | '⠧' | '⠇' | '⠏'
            )
        })
        .unwrap_or(trimmed)
        .trim();
    if without_spinner.is_empty() {
        None
    } else {
        Some(without_spinner.to_owned())
    }
}

fn is_noisy_agent_status_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("⠋")
        || trimmed.starts_with("⠙")
        || trimmed.starts_with("⠹")
        || trimmed.starts_with("⠸")
        || trimmed.starts_with("⠼")
        || trimmed.starts_with("⠴")
        || trimmed.starts_with("⠦")
        || trimmed.starts_with("⠧")
        || trimmed.starts_with("⠇")
        || trimmed.starts_with("⠏")
        || trimmed.eq_ignore_ascii_case("thinking")
        || trimmed.eq_ignore_ascii_case("working")
}

fn is_noisy_agent_ui_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("› ")
        || trimmed == "›"
        || trimmed.starts_with("❯")
        || trimmed.starts_with("• Working")
        || trimmed.starts_with("• Thinking")
        || trimmed.contains("esc to interrupt")
        || trimmed.contains("ctrl + t to view transcript")
        || trimmed.contains("? for shortcuts")
        || trimmed.starts_with("⚠ Model metadata")
        || trimmed
            .chars()
            .all(|ch| is_separator_char(ch) || ch.is_whitespace())
}

fn is_noisy_claude_ui_line(line: &str) -> bool {
    let cleaned = clean_footer_line(line);
    let lower = cleaned.to_ascii_lowercase();
    lower.contains("claude code v")
        || lower.contains("tips for getting started")
        || lower.contains("welcome back")
        || lower.contains("run /init to create")
        || lower.contains("recent activity")
        || lower.contains("no recent activity")
        || lower.contains("api usage billing")
        || lower.contains("/effort")
        || (cleaned.starts_with("~/") && !cleaned.chars().any(char::is_whitespace))
        || cleaned
            .chars()
            .any(|ch| matches!(ch, '▐' | '▛' | '█' | '▜' | '▌' | '▝' | '▘' | '▗'))
}

fn is_separator_char(ch: char) -> bool {
    matches!(
        ch,
        '─' | '━'
            | '-'
            | '—'
            | '═'
            | '│'
            | '┃'
            | '┆'
            | '┊'
            | '║'
            | '╎'
            | '╏'
            | '╭'
            | '╮'
            | '╰'
            | '╯'
    )
}

fn non_empty_delta(delta: &str) -> Option<String> {
    let delta = delta.trim_matches('\n').trim_end();
    if delta.trim().is_empty() {
        None
    } else {
        Some(delta.to_owned())
    }
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
    fn chat_text_is_split_by_char_count() {
        let parts = split_for_chat(&"a".repeat(1801));

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].chars().count(), 1800);
    }

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

    #[test]
    fn pane_delta_sends_only_appended_text() {
        let delta = pane_delta(
            Some("first\nsecond"),
            "first\nsecond\nthird",
            ProviderKind::Codex,
        );

        assert_eq!(delta, Some("third".to_owned()));
    }

    #[test]
    fn pane_delta_handles_scrolled_overlap() {
        let delta = pane_delta(Some("a\nb\nc"), "b\nc\nd", ProviderKind::Codex);

        assert_eq!(delta, Some("d".to_owned()));
    }

    #[test]
    fn pane_delta_handles_current_tail_embedded_in_previous() {
        let previous = [
            "startup banner",
            "old warning",
            "• 明白，这个仓库 /Users/wumengsong/Code/cmux_test 是专门用来测试 cmux 的。",
            "  你接下来想让我在这里做什么？",
        ]
        .join("\n");
        let current = [
            "old warning",
            "• 明白，这个仓库 /Users/wumengsong/Code/cmux_test 是专门用来测试 cmux 的。",
            "  你接下来想让我在这里做什么？",
            "• 我来快速列一下当前目录内容。",
            "• 当前目录 /Users/wumengsong/Code/cmux_test 里没有项目文件，只有：",
            "  - . 当前目录",
            "  - .. 上级目录",
        ]
        .join("\n");

        let delta = pane_delta(Some(&previous), &current, ProviderKind::Codex);

        assert_eq!(
            delta,
            Some(
                [
                    "• 我来快速列一下当前目录内容。",
                    "• 当前目录 /Users/wumengsong/Code/cmux_test 里没有项目文件，只有：",
                    "  - . 当前目录",
                    "  - .. 上级目录",
                ]
                .join("\n")
            )
        );
    }

    #[test]
    fn pane_delta_suppresses_repeated_text() {
        let delta = pane_delta(Some("same"), "same", ProviderKind::Codex);

        assert_eq!(delta, None);
    }

    #[test]
    fn pane_delta_suppresses_terminal_reflow_only_changes() {
        let delta = pane_delta(
            Some("README.zh-CN.md\n列出会话、创建本地 shell 测试会话"),
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话",
            ProviderKind::Codex,
        );

        assert_eq!(delta, None);
    }

    #[test]
    fn codex_pane_normalization_filters_spinner_status() {
        let text = normalize_pane_text("hello\n⠋ thinking\nworld", ProviderKind::Codex);

        assert_eq!(text, "hello\nworld");
    }

    #[test]
    fn codex_pane_normalization_filters_prompt_echo_and_ui_hints() {
        let text = normalize_pane_text(
            "answer\n› user prompt\n• Working (1s • esc to interrupt)\n› Use /skills to list available skills",
            ProviderKind::Codex,
        );

        assert_eq!(text, "answer");
    }

    #[test]
    fn claude_pane_normalization_filters_welcome_screen() {
        let text = normalize_pane_text(
            "╭─── Claude Code v2.1.119 ─────────────────────────╮\n│            Welcome back!           │ Tips for getting started\n│               ▐▛███▜▌              │ Run /init to create a CLAUDE.md file\n│   Sonnet 4.6 · API Usage Billing   │ Recent activity\n│          ~/Code/ChatMuxX           │ No recent activity\n╰──────────────────────────────────────────────────╯\n❯\n  ? for shortcuts                             ● high · /effort",
            ProviderKind::Claude,
        );

        assert_eq!(text, "");
    }

    #[test]
    fn shell_pane_normalization_filters_prompts_and_command_echo() {
        let text = normalize_pane_text(
            "The default interactive shell is now zsh.\nTo update your account to use zsh, please run `chsh -s /bin/zsh`.\nFor more details, please visit https://support.apple.com/kb/HT208050.\nbash-3.2$ pwd\n/Users/wumengsong/Code/ChatMuxX\nbash-3.2$",
            ProviderKind::Shell,
        );

        assert_eq!(text, "/Users/wumengsong/Code/ChatMuxX");
    }

    #[test]
    fn shell_pane_normalization_handles_zsh_style_prompts() {
        let text = normalize_pane_text(
            "wumengsong@Mac ChatMuxX % ls\nREADME.md\nCargo.toml\nwumengsong@Mac ChatMuxX %",
            ProviderKind::Shell,
        );

        assert_eq!(text, "README.md\nCargo.toml");
    }

    #[test]
    fn footer_is_added_to_display_message() {
        let text = format_display_message(
            "done".to_owned(),
            Some("状态：thinking · 模型：gpt-5.1-codex"),
        );

        assert_eq!(text, "done\n\n——\n状态：thinking · 模型：gpt-5.1-codex");
    }

    #[test]
    fn unchanged_footer_is_included_with_display_body() {
        let footer = footer_for_display(Some("状态：thinking"), Some("状态：thinking"));

        assert_eq!(footer, Some("状态：thinking".to_owned()));
    }

    #[test]
    fn last_footer_is_reused_when_current_pane_has_no_footer() {
        let footer = footer_for_display(None, Some("gpt-5.5 high · ~/Code/ChatMuxX"));

        assert_eq!(footer, Some("gpt-5.5 high · ~/Code/ChatMuxX".to_owned()));
    }

    #[test]
    fn pending_text_waits_for_flush_interval() {
        let state = MonitorSessionState {
            provider: ProviderKind::Codex,
            source: None,
            last_pane_hash: None,
            last_pane_text: None,
            last_status_text: None,
            last_footer_text: None,
            pending_display_text: Some("hello".to_owned()),
            pending_footer_text: None,
            pending_since_ms: Some(1_000),
            last_delivery_at_ms: None,
        };

        assert!(!should_flush_pending(&state, 3_999));
        assert!(should_flush_pending(&state, 4_000));
    }

    #[test]
    fn pending_text_appends_with_newline() {
        let pending = merge_pending_text(Some("one".to_owned()), "two");

        assert_eq!(pending, Some("one\ntwo".to_owned()));
    }

    #[test]
    fn pending_text_drops_contained_duplicate() {
        let pending = merge_pending_text(Some("alpha\nbeta\ngamma".to_owned()), "beta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn pending_text_merges_line_overlap() {
        let pending = merge_pending_text(Some("alpha\nbeta".to_owned()), "beta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn pending_text_replaces_when_next_contains_previous() {
        let pending = merge_pending_text(Some("beta".to_owned()), "alpha\nbeta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn provider_status_is_extracted_from_spinner_line() {
        let status = extract_provider_status("hello\n⠋ thinking", ProviderKind::Codex);

        assert_eq!(status, Some("thinking".to_owned()));
    }

    #[test]
    fn terminal_footer_includes_status_and_model() {
        let footer = extract_terminal_footer(
            "answer\n⠋ thinking\nmodel: gpt-5.1-codex",
            ProviderKind::Codex,
        );

        assert_eq!(footer, Some("状态：thinking · gpt-5.1-codex".to_owned()));
    }

    #[test]
    fn terminal_footer_keeps_codex_bottom_bar_context() {
        let footer = extract_terminal_footer(
            "answer\n\n› Use /skills to list available skills\n\n  gpt-5.5 high · ~/Code/ChatMuxX",
            ProviderKind::Codex,
        );

        assert_eq!(footer, Some("gpt-5.5 high · ~/Code/ChatMuxX".to_owned()));
    }

    #[test]
    fn terminal_footer_keeps_claude_startup_context() {
        let footer = extract_terminal_footer(
            "╭─── Claude Code v2.1.119 ─────────╮\n│   Sonnet 4.6 · API Usage Billing   │\n│          ~/Code/ChatMuxX           │\n❯\n  ? for shortcuts              ● high · /effort",
            ProviderKind::Claude,
        );

        assert_eq!(footer, Some("Sonnet 4.6 · API Usage Billing".to_owned()));
    }

    #[test]
    fn terminal_footer_includes_status_and_bottom_bar_context() {
        let footer = extract_terminal_footer(
            "answer\n⠋ thinking\n  gpt-5.5 high · ~/Code/ChatMuxX",
            ProviderKind::Codex,
        );

        assert_eq!(
            footer,
            Some("状态：thinking · gpt-5.5 high · ~/Code/ChatMuxX".to_owned())
        );
    }

    #[test]
    fn pane_normalization_filters_bottom_model_line() {
        let text = normalize_pane_text("answer\nmodel: gpt-5.1-codex", ProviderKind::Codex);

        assert_eq!(text, "answer");
    }

    #[test]
    fn redundant_outbound_detects_exact_recent_message() {
        assert!(is_redundant_outbound(
            "hello\nworld",
            &[" hello \n\n world ".to_owned()]
        ));
    }

    #[test]
    fn redundant_outbound_detects_candidate_inside_recent_message() {
        assert!(is_redundant_outbound(
            "world",
            &["hello\nworld\nagain".to_owned()]
        ));
    }

    #[test]
    fn redundant_outbound_allows_new_text() {
        assert!(!is_redundant_outbound(
            "new output",
            &["old output".to_owned()]
        ));
    }

    #[test]
    fn recent_outbound_suppression_keeps_only_suffix_after_previous_message() {
        let text = suppress_recent_outbound_text("alpha\nbeta\ngamma", &["alpha\nbeta".to_owned()]);

        assert_eq!(text, Some("gamma".to_owned()));
    }

    #[test]
    fn recent_outbound_suppression_drops_fully_repeated_message() {
        let text = suppress_recent_outbound_text("alpha\nbeta", &["alpha\nbeta\ngamma".to_owned()]);

        assert_eq!(text, None);
    }

    #[test]
    fn recent_outbound_suppression_removes_scrolled_overlap() {
        let text =
            suppress_recent_outbound_text("beta\ngamma\ndelta", &["alpha\nbeta\ngamma".to_owned()]);

        assert_eq!(text, Some("delta".to_owned()));
    }

    #[test]
    fn recent_outbound_suppression_drops_reflowed_repeat() {
        let text = suppress_recent_outbound_text(
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build",
            &[
                "README.zh-CN.md\n列出会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build"
                    .to_owned(),
            ],
        );

        assert_eq!(text, None);
    }

    #[test]
    fn recent_outbound_suppression_keeps_suffix_after_reflowed_prefix() {
        let text = suppress_recent_outbound_text(
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build\n新增内容",
            &[
                "README.zh-CN.md\n列出会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build"
                    .to_owned(),
            ],
        );

        assert_eq!(text, Some("新增内容".to_owned()));
    }
}
