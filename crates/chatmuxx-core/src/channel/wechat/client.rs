use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, Method};

use crate::{
    channel::wechat::model::{
        BaseInfo, GetUpdatesRequest, GetUpdatesResponse, QrLoginStart, QrLoginStatus, RawQrStatus,
        SendTextBody, SendTextItem, SendTextMessage, SendTextRequest, WeChatLoginCredentials,
    },
    ChatMuxXError, Result,
};

const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const MESSAGE_TYPE_BOT: i64 = 2;
const MESSAGE_STATE_FINISH: i64 = 2;
const MESSAGE_ITEM_TEXT: i64 = 1;

#[derive(Clone, Debug)]
pub struct WeChatClient {
    base_url: String,
    http: Client,
}

impl WeChatClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base_url(&base_url.into()),
            http: Client::new(),
        }
    }

    pub async fn get_bot_qrcode(&self, bot_type: &str) -> Result<QrLoginStart> {
        let url = self.url(&format!(
            "ilink/bot/get_bot_qrcode?bot_type={}",
            url_encode(bot_type)
        ));
        let response = self.http.get(url).send().await?;
        parse_json_response(response).await
    }

    pub async fn poll_qrcode_status(&self, qrcode: &str) -> Result<QrLoginStatus> {
        let url = self.url(&format!(
            "ilink/bot/get_qrcode_status?qrcode={}",
            url_encode(qrcode)
        ));
        let response = self
            .http
            .get(url)
            .header("iLink-App-ClientVersion", "1")
            .send()
            .await?;
        let raw: RawQrStatus = parse_json_response(response).await?;

        match raw.status.as_str() {
            "wait" => Ok(QrLoginStatus::Waiting),
            "scaned" => Ok(QrLoginStatus::Scanned),
            "expired" => Ok(QrLoginStatus::Expired),
            "cancelled" | "canceled" => Ok(QrLoginStatus::Cancelled),
            "confirmed" => {
                let token = raw.bot_token.ok_or_else(|| {
                    ChatMuxXError::WeChatProtocol("confirmed QR login missing bot_token".to_owned())
                })?;
                let account_id = raw.ilink_bot_id.ok_or_else(|| {
                    ChatMuxXError::WeChatProtocol(
                        "confirmed QR login missing ilink_bot_id".to_owned(),
                    )
                })?;
                Ok(QrLoginStatus::Confirmed(WeChatLoginCredentials {
                    bot_token: token,
                    account_id,
                    user_id: raw.ilink_user_id,
                    base_url: raw
                        .baseurl
                        .unwrap_or_else(|| self.base_url.trim_end_matches('/').to_owned()),
                }))
            }
            other => Err(ChatMuxXError::WeChatProtocol(format!(
                "unknown QR status: {other}"
            ))),
        }
    }

    pub async fn get_updates(
        &self,
        token: &str,
        get_updates_buf: Option<&str>,
        timeout_ms: u64,
    ) -> Result<GetUpdatesResponse> {
        let body = GetUpdatesRequest {
            get_updates_buf: get_updates_buf.unwrap_or_default(),
            base_info: BaseInfo::default(),
        };
        let response = self
            .request_json(Method::POST, "ilink/bot/getupdates", Some(token), &body)
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(err) if err.is_timeout() => {
                return Ok(GetUpdatesResponse {
                    ret: Some(0),
                    errcode: None,
                    errmsg: None,
                    msgs: Vec::new(),
                    get_updates_buf: get_updates_buf.map(str::to_owned),
                    longpolling_timeout_ms: None,
                });
            }
            Err(err) => return Err(err.into()),
        };

        let parsed: GetUpdatesResponse = parse_json_response(response).await?;
        if parsed.errcode == Some(-14) {
            return Err(ChatMuxXError::WeChatAccountExpired);
        }
        Ok(parsed)
    }

    pub async fn send_text(
        &self,
        token: &str,
        to_user_id: &str,
        text: &str,
        context_token: &str,
    ) -> Result<()> {
        let client_id = client_id();
        let body = SendTextRequest {
            msg: SendTextMessage {
                from_user_id: "",
                to_user_id,
                client_id: &client_id,
                message_type: MESSAGE_TYPE_BOT,
                message_state: MESSAGE_STATE_FINISH,
                context_token,
                item_list: vec![SendTextItem {
                    r#type: MESSAGE_ITEM_TEXT,
                    text_item: SendTextBody { text },
                }],
            },
            base_info: BaseInfo::default(),
        };
        let response = self
            .request_json(Method::POST, "ilink/bot/sendmessage", Some(token), &body)
            .send()
            .await?;
        ensure_success(response).await?;
        Ok(())
    }

    fn request_json<T: serde::Serialize + ?Sized>(
        &self,
        method: Method,
        endpoint: &str,
        token: Option<&str>,
        body: &T,
    ) -> reqwest::RequestBuilder {
        let bytes = serde_json::to_vec(body).expect("serialize request body");
        let mut builder = self
            .http
            .request(method, self.url(endpoint))
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("Content-Length", bytes.len().to_string())
            .header("X-WECHAT-UIN", random_wechat_uin())
            .body(bytes);
        if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
            builder = builder.header("Authorization", format!("Bearer {}", token.trim()));
        }
        builder
    }

    fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint.trim_start_matches('/'))
    }
}

async fn parse_json_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ChatMuxXError::WeChatHttpStatus {
            status: status.as_u16(),
            body,
        });
    }

    serde_json::from_str(&body)
        .map_err(|err| ChatMuxXError::WeChatProtocol(format!("invalid JSON response: {err}")))
}

async fn ensure_success(response: reqwest::Response) -> Result<()> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ChatMuxXError::WeChatHttpStatus {
            status: status.as_u16(),
            body,
        });
    }
    Ok(())
}

fn normalize_base_url(url: &str) -> String {
    let trimmed = if url.trim().is_empty() {
        DEFAULT_BASE_URL
    } else {
        url.trim()
    };
    if trimmed.ends_with('/') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}/")
    }
}

fn random_wechat_uin() -> String {
    STANDARD.encode(rand::random::<u32>().to_string())
}

fn client_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("chatmuxx-{nanos}")
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_is_normalized() {
        let client = WeChatClient::new("https://example.test");

        assert_eq!(
            client.url("ilink/bot/getupdates"),
            "https://example.test/ilink/bot/getupdates"
        );
    }

    #[test]
    fn random_uin_is_base64_decimal_timestamp_fragment() {
        let value = random_wechat_uin();

        assert!(!value.is_empty());
        assert!(STANDARD.decode(value).is_ok());
    }

    #[test]
    fn url_encode_escapes_qrcode_values() {
        assert_eq!(url_encode("a b+c"), "a%20b%2Bc");
    }
}
