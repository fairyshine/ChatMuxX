# WeChat iLink Adapter

Status: draft.

ChatMuxX v0.1 implements WeChat through direct iLink HTTP calls. It must not depend on OpenClaw runtime, gateway, plugin installation, or OpenClaw account storage.

The `references/openclaw-weixin` code is only a protocol reference.

## Scope for v0.1

Implement:

- QR login.
- token persistence.
- `getupdates` long polling.
- cursor persistence through `get_updates_buf`.
- inbound text normalization.
- WeChat conversation identity.
- `context_token` preservation.
- outbound text through `sendmessage`.
- expired-account detection.

Do not implement yet:

- CDN media upload/download.
- image/file/voice/video message handling.
- typing indicators.
- multi-account UI.

## HTTP Client

Suggested module:

```text
channel/wechat/
  auth.rs
  client.rs
  model.rs
  polling.rs
  send.rs
  conversation.rs
```

`client.rs` owns low-level HTTP:

```rust
pub struct WeChatClient {
    base_url: Url,
    http: reqwest::Client,
}
```

All authenticated requests should set iLink headers in one place:

```text
Content-Type: application/json
AuthorizationType: ilink_bot_token
Authorization: Bearer <bot_token>
X-WECHAT-UIN: <random uint32 encoded as base64 string>
```

Never log:

- `Authorization`
- `bot_token`
- `X-WECHAT-UIN`
- `context_token`
- raw request/response bodies containing credentials

## QR Login Flow

`cmux login wechat`:

1. Create `~/.chatmuxx` if needed.
2. Call `get_bot_qrcode`.
3. Render the QR code in terminal.
4. Poll `get_qrcode_status` until confirmed, expired, cancelled, or timed out.
5. Persist account data to `accounts.json`.
6. Initialize owner identity if no owner is configured or persisted.
7. Print a concise success message.

Suggested result model:

```rust
pub struct QrLoginStart {
    pub qrcode: String,
    pub url: String,
}

pub enum QrLoginStatus {
    Waiting,
    Scanned,
    Confirmed(WeChatLoginCredentials),
    Expired,
    Cancelled,
}

pub struct WeChatLoginCredentials {
    pub bot_token: SecretString,
    pub bot_user_id: Option<String>,
    pub base_url: Url,
}
```

## Long Polling

`WeChatAdapter::run(sink)` owns the long-poll loop.

Loop:

1. Load account token and `get_updates_buf`.
2. Call `getupdates`.
3. If response contains new cursor, persist it.
4. Convert supported messages to `ChannelEvent`.
5. Send events to `ChannelEventSink`.
6. Sleep/retry according to server timeout and local backoff policy.

Rules:

- Always advance `get_updates_buf` after a successful response, even when no supported messages are produced.
- If iLink reports session expiration, emit `ChannelEvent::ChannelAccountExpired`.
- Use exponential backoff for transient network errors.
- Do not busy-loop on repeated failures.

Suggested polling config:

```rust
pub struct WeChatPollingConfig {
    pub default_timeout_ms: u64,
    pub min_backoff_ms: u64,
    pub max_backoff_ms: u64,
}
```

## Inbound Text

Only text items are required for v0.1.

Conversion:

```text
WeixinMessage
  -> derive ConversationRef
  -> derive SenderRef
  -> persist context_token as secret context token ref
  -> ChannelEvent::InboundText
```

Conversation identity:

- Direct chat: channel type + account id + `from_user_id`.
- Group chat: channel type + account id + `group_id`.
- Sender authorization still checks `from_user_id` against owner identity.

`context_token` handling:

- Store raw token in sensitive account/channel state.
- Store only `context_token_ref` in `PlatformMeta` and conversation state.
- When sending a reply, resolve the latest token ref for that conversation.
- Never write raw token into history or normal logs.

## Outbound Text

`send.rs` maps `OutboundMessage::Text` to iLink `sendmessage`.

Required fields:

- `to_user_id`: target WeChat peer.
- `context_token`: latest token for the conversation.
- `message_type`: bot message.
- `message_state`: finished message.
- `item_list`: one text item.

If `context_token` is missing:

- return a typed error.
- delivery should send a user-facing message asking the owner to send a new message in that WeChat conversation to refresh context.

## Unsupported Message Types

For v0.1:

- ignore unsupported media by default.
- optionally send a short text response saying media is not supported yet.
- do not download media.
- do not persist media metadata unless needed for debugging.

## Error Mapping

Suggested errors:

```rust
pub enum WeChatError {
    LoginExpired,
    AccountExpired,
    MissingContextToken,
    UnsupportedMessageType,
    HttpStatus(u16),
    Protocol(String),
    Network(String),
}
```

Error handling:

- `AccountExpired` becomes `ChannelAccountExpired`.
- `MissingContextToken` becomes a user-facing delivery error.
- transient `Network` errors use backoff and continue polling.
- protocol errors are logged with redaction.

## Tests

Use a fake iLink server.

Required tests:

- QR login confirmed.
- QR login expired.
- `getupdates` cursor advances.
- text message becomes `InboundText`.
- group message derives group conversation and sender separately.
- `context_token` is stored by reference.
- `sendmessage` resolves context token and sends text.
- account-expired response emits `ChannelAccountExpired`.

