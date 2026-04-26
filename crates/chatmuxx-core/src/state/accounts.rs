use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountState {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub wechat: Vec<WeChatAccountRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatAccountRecord {
    pub account_id: String,
    #[serde(default)]
    pub bot_user_id: Option<String>,
    pub bot_token: String,
    pub base_url: String,
    #[serde(default)]
    pub get_updates_buf: Option<String>,
    #[serde(default)]
    pub context_tokens: BTreeMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}
