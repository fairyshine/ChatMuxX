use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::provider::ProviderKind;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AppState {
    pub schema_version: u32,
    pub owner: Option<OwnerState>,
    pub sessions: Vec<SessionRecord>,
    pub bindings: Vec<BindingRecord>,
    pub conversations: Vec<ConversationRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OwnerState {
    pub id: OwnerId,
    pub wechat_user_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
pub struct OwnerId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self(format!("s-{}", base36(millis)))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

fn base36(mut value: u128) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_owned();
    }

    let mut chars = Vec::new();
    while value > 0 {
        chars.push(DIGITS[(value % 36) as usize] as char);
        value /= 36;
    }
    chars.iter().rev().collect()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub status: SessionStatus,
    pub tmux: Option<TmuxAttachment>,
    pub owner: OwnerId,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum SessionStatus {
    Starting,
    Running,
    WaitingInput,
    Done,
    Dead,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TmuxAttachment {
    pub session_name: String,
    pub window_id: String,
    pub pane_id: String,
    pub created_by_chatmuxx: bool,
    pub adopted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindingRecord {
    pub conversation_id: String,
    pub session_id: SessionId,
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConversationRecord {
    pub id: String,
    pub channel_type: ChannelType,
    pub account_id: String,
    pub external_conversation_id: String,
    pub kind: ConversationKind,
    pub latest_context_token_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ChannelType {
    WeChat,
    Telegram,
    External(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ConversationKind {
    Direct,
    Group,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_app_state_has_schema_zero_until_initialized() {
        let state = AppState::default();

        assert_eq!(state.schema_version, 0);
        assert!(state.sessions.is_empty());
    }
}
