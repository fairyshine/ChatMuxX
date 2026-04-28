use crate::{
    app::daemon::{
        active_session_id, bind_conversation, confirm_interrupt_active, format_session_list,
        help_text, messages, now_millis, send_key_to_active, set_confirmation, wechat,
        workspace_looks_like_option, InboundWeChatText,
    },
    mobile::MobileCommand,
    provider::{display::trim_for_chat, LaunchMode, ProviderKind},
    session::{CreateSessionRequest, SessionManager},
    state::{
        files::StatePaths,
        sessions::{ConfirmationAction, OwnerId},
    },
    tmux::TmuxKey,
    Result,
};

pub(super) async fn handle_bridge_command(
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
            set_confirmation(
                paths,
                &event.conversation_id,
                ConfirmationAction::CloseSession {
                    session_id: session_id.clone(),
                },
                now_millis(),
            )
            .await?;
            wechat::send_reply(
                paths,
                &event.account_id,
                &event.conversation_id,
                &messages::confirm_close(&session_id),
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
        MobileCommand::Interrupt => confirm_interrupt_active(paths, manager, event).await?,
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
