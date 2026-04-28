use crate::{
    config::Config,
    state::{
        accounts::AccountState,
        atomic::{load_json_or_default, save_json},
        files::StatePaths,
        sessions::{
            AppState, BindingRecord, ConfirmationAction, ConfirmationRecord, ConversationRecord,
            SessionId, SessionStatus,
        },
    },
    ChatMuxXError, Result,
};

pub(super) async fn is_authorized(
    paths: &StatePaths,
    config: &Config,
    from_user_id: &str,
) -> Result<bool> {
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

pub(super) async fn active_session_id(
    paths: &StatePaths,
    conversation_id: &str,
) -> Result<Option<SessionId>> {
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

pub(super) async fn bind_conversation(
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

pub(super) fn upsert_conversation(state: &mut AppState, record: ConversationRecord) {
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

pub(super) async fn set_confirmation(
    paths: &StatePaths,
    conversation_id: &str,
    action: ConfirmationAction,
    created_at_ms: u64,
) -> Result<()> {
    let mut state: AppState = load_json_or_default(&paths.state).await?;
    state.schema_version = 1;
    state
        .confirmations
        .retain(|confirmation| confirmation.conversation_id != conversation_id);
    state.confirmations.push(ConfirmationRecord {
        conversation_id: conversation_id.to_owned(),
        action,
        created_at_ms,
    });
    save_json(&paths.state, &state).await
}

pub(super) async fn take_confirmation(
    paths: &StatePaths,
    conversation_id: &str,
    now_ms: u64,
    ttl_ms: u64,
) -> Result<Option<ConfirmationAction>> {
    let mut state: AppState = load_json_or_default(&paths.state).await?;
    let mut action = None;
    state.confirmations.retain(|confirmation| {
        let expired = now_ms.saturating_sub(confirmation.created_at_ms) > ttl_ms;
        let matched = confirmation.conversation_id == conversation_id;
        if matched && !expired {
            action = Some(confirmation.action.clone());
        }
        !(matched || expired)
    });
    save_json(&paths.state, &state).await?;
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn confirmation_is_stored_and_consumed_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StatePaths::from_root(dir.path());
        let session_id = SessionId("main".to_owned());

        set_confirmation(
            &paths,
            "conv-1",
            ConfirmationAction::CloseSession {
                session_id: session_id.clone(),
            },
            1_000,
        )
        .await
        .expect("set confirmation");

        assert_eq!(
            take_confirmation(&paths, "conv-1", 2_000, 60_000)
                .await
                .expect("take confirmation"),
            Some(ConfirmationAction::CloseSession { session_id })
        );
        assert_eq!(
            take_confirmation(&paths, "conv-1", 2_001, 60_000)
                .await
                .expect("take confirmation again"),
            None
        );
    }

    #[tokio::test]
    async fn expired_confirmation_is_removed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StatePaths::from_root(dir.path());

        set_confirmation(
            &paths,
            "conv-1",
            ConfirmationAction::InterruptSession {
                session_id: SessionId("main".to_owned()),
            },
            1_000,
        )
        .await
        .expect("set confirmation");

        assert_eq!(
            take_confirmation(&paths, "conv-1", 62_000, 60_000)
                .await
                .expect("take expired confirmation"),
            None
        );
    }
}
