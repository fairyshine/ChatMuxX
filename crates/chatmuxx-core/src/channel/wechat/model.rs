use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct QrLoginStart {
    pub qrcode: String,
    #[serde(rename = "qrcode_img_content")]
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QrLoginStatus {
    Waiting,
    Scanned,
    Confirmed(WeChatLoginCredentials),
    Expired,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeChatLoginCredentials {
    pub bot_token: String,
    pub account_id: String,
    pub user_id: Option<String>,
    pub base_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RawQrStatus {
    pub status: String,
    pub bot_token: Option<String>,
    pub ilink_bot_id: Option<String>,
    pub ilink_user_id: Option<String>,
    pub baseurl: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetUpdatesResponse {
    #[serde(default)]
    pub ret: Option<i64>,
    #[serde(default)]
    pub errcode: Option<i64>,
    #[serde(default)]
    pub errmsg: Option<String>,
    #[serde(default)]
    pub msgs: Vec<WeChatMessage>,
    #[serde(default)]
    pub get_updates_buf: Option<String>,
    #[serde(default)]
    pub longpolling_timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatMessage {
    #[serde(default)]
    pub message_id: Option<i64>,
    #[serde(default)]
    pub from_user_id: Option<String>,
    #[serde(default)]
    pub to_user_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub create_time_ms: Option<i64>,
    #[serde(default)]
    pub message_type: Option<i64>,
    #[serde(default)]
    pub message_state: Option<i64>,
    #[serde(default)]
    pub item_list: Vec<WeChatMessageItem>,
    #[serde(default)]
    pub context_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatMessageItem {
    #[serde(default)]
    pub r#type: Option<i64>,
    #[serde(default)]
    pub text_item: Option<WeChatTextItem>,
    #[serde(default)]
    pub voice_item: Option<WeChatVoiceItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatTextItem {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatVoiceItem {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct GetUpdatesRequest<'a> {
    pub get_updates_buf: &'a str,
    pub base_info: BaseInfo,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SendTextRequest<'a> {
    pub msg: SendTextMessage<'a>,
    pub base_info: BaseInfo,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SendTextMessage<'a> {
    pub from_user_id: &'a str,
    pub to_user_id: &'a str,
    pub client_id: &'a str,
    pub message_type: i64,
    pub message_state: i64,
    pub context_token: &'a str,
    pub item_list: Vec<SendTextItem<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SendTextItem<'a> {
    pub r#type: i64,
    pub text_item: SendTextBody<'a>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SendTextBody<'a> {
    pub text: &'a str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct BaseInfo {
    pub channel_version: &'static str,
}

impl Default for BaseInfo {
    fn default() -> Self {
        Self {
            channel_version: env!("CARGO_PKG_VERSION"),
        }
    }
}
