use std::time::Duration;

use tokio::sync::mpsc;

use super::{now_string, InboundWeChatText};
use crate::{
    app::daemon::{state::upsert_conversation, text::{is_redundant_outbound, suppress_recent_outbound_text}},
    channel::wechat::{
        conversation_external_id, conversation_id, extract_text, is_group_message, token_ref,
        WeChatClient, WeChatMessage,
    },
    config::Config,
    provider::display::split_for_chat,
    state::{
        accounts::{AccountState, WeChatAccountRecord},
        atomic::{load_json_or_default, save_json},
        files::StatePaths,
        history::{append_history, recent_outbound_texts, HistoryEvent},
        sessions::{AppState, ChannelType, ConversationKind, ConversationRecord},
    },
    ChatMuxXError, Result,
};

pub(super) async fn poll_loop(
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

pub(super) async fn send_reply(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
) -> Result<()> {
    send_reply_inner(paths, account_id, conversation_id, text, false).await
}

pub(super) async fn send_deduped_reply(
    paths: &StatePaths,
    account_id: &str,
    conversation_id: &str,
    text: &str,
) -> Result<()> {
    send_reply_inner(paths, account_id, conversation_id, text, true).await
}

async fn send_reply_inner(
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
