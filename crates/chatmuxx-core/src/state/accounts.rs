use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountState {
    pub schema_version: u32,
    pub wechat: Vec<WeChatAccountRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatAccountRecord {
    pub account_id: String,
    pub bot_user_id: Option<String>,
    pub bot_token: String,
    pub base_url: String,
    pub get_updates_buf: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
