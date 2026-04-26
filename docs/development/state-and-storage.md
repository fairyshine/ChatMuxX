# State and Storage

Status: draft.

## Directory Layout

Default state directory:

```text
~/.chatmuxx/
  config.toml
  state.json
  accounts.json
  monitor_state.json
  history.jsonl
  logs/
    cmx.log
```

Permissions:

- `~/.chatmuxx`: `0700`
- `accounts.json`: `0600`
- files containing tokens or credentials: `0600`
- non-sensitive state files: no broader than `0644`; prefer `0600` for simplicity in v0.1.

`cmx doctor` should warn when permissions are too broad and may offer a fix later.

## `config.toml`

Human-authored configuration.

```toml
[daemon]
tmux_session = "chatmuxx"
poll_interval_ms = 1500

[owner]
# Optional. If empty, owner can be initialized during first WeChat pairing.
wechat_user_id = ""

[wechat]
enabled = true

[providers.codex]
command = "codex"
args = []
env = {}

[providers.claude]
command = "claude"
args = []
env = {}

[providers.shell]
command = "bash"
args = []
env = {}
```

Suggested Rust model:

```rust
pub struct Config {
    pub daemon: DaemonConfig,
    pub owner: OwnerConfig,
    pub wechat: WeChatConfig,
    pub providers: ProviderConfigs,
}

pub struct DaemonConfig {
    pub tmux_session: String,
    pub poll_interval_ms: u64,
}

pub struct OwnerConfig {
    pub wechat_user_id: Option<String>,
}

pub struct ProviderConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub struct ProviderConfigs {
    pub codex: ProviderConfig,
    pub claude: ProviderConfig,
    pub shell: ProviderConfig,
}
```

Config defaults should be applied by `config::loader`, not scattered through business modules.

## `state.json`

Runtime state owned by ChatMuxX.

```json
{
  "schema_version": 1,
  "owner": {
    "id": "owner-local-1",
    "wechat_user_id": "masked-or-real-wechat-user-id"
  },
  "sessions": [
    {
      "id": "ses_...",
      "provider": "codex",
      "workspace": "/Users/example/Code/project",
      "status": "running",
      "tmux": {
        "session_name": "chatmuxx",
        "window_id": "@1",
        "pane_id": "%3",
        "created_by_chatmuxx": true,
        "adopted": false
      },
      "created_at": "2026-04-26T00:00:00Z",
      "updated_at": "2026-04-26T00:00:00Z"
    }
  ],
  "bindings": [
    {
      "conversation_id": "conv_wechat_...",
      "session_id": "ses_...",
      "active": true
    }
  ],
  "conversations": [
    {
      "id": "conv_wechat_...",
      "channel_type": "wechat",
      "account_id": "acct_wechat_...",
      "external_conversation_id": "wechat-peer-or-group",
      "latest_context_token_ref": "ctx_..."
    }
  ]
}
```

Notes:

- `session.id` is stable.
- tmux metadata may change during recovery.
- `context_token` should not be written into general logs or history. If it must be persisted for WeChat replies, keep it in state/accounts with redaction on log output.

Suggested Rust model:

```rust
pub struct AppState {
    pub schema_version: u32,
    pub owner: Option<OwnerState>,
    pub sessions: Vec<SessionRecord>,
    pub bindings: Vec<BindingRecord>,
    pub conversations: Vec<ConversationRecord>,
    pub active_flows: Vec<FlowRecord>,
}

pub struct OwnerState {
    pub id: OwnerId,
    pub wechat_user_id: Option<String>,
}

pub struct SessionRecord {
    pub id: SessionId,
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub status: SessionStatus,
    pub tmux: Option<TmuxAttachment>,
    pub owner: OwnerId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct BindingRecord {
    pub conversation_id: ConversationId,
    pub session_id: SessionId,
    pub active: bool,
}

pub struct ConversationRecord {
    pub id: ConversationId,
    pub channel_type: ChannelType,
    pub account_id: ChannelAccountId,
    pub external_conversation_id: String,
    pub kind: ConversationKind,
    pub latest_context_token_ref: Option<String>,
}
```

`active_flows` may be persisted in v0.1 so confirmation/onboarding can survive a daemon restart. If that is too much for the first implementation, flows can be in-memory first, but the state shape should reserve the concept.

## `accounts.json`

Channel account and credential metadata.

```json
{
  "schema_version": 1,
  "wechat": [
    {
      "account_id": "acct_wechat_...",
      "bot_user_id": "xxx@im.bot",
      "bot_token": "secret",
      "base_url": "https://ilinkai.weixin.qq.com",
      "get_updates_buf": "opaque-cursor",
      "created_at": "2026-04-26T00:00:00Z",
      "updated_at": "2026-04-26T00:00:00Z"
    }
  ]
}
```

Rules:

- `accounts.json` is sensitive.
- Never print `bot_token`.
- Store with `0600`.
- Future versions may move tokens to OS keychain and leave only references here.

Suggested Rust model:

```rust
pub struct AccountState {
    pub schema_version: u32,
    pub wechat: Vec<WeChatAccountRecord>,
}

pub struct WeChatAccountRecord {
    pub account_id: ChannelAccountId,
    pub bot_user_id: Option<String>,
    pub bot_token: SecretString,
    pub base_url: String,
    pub get_updates_buf: Option<String>,
    pub context_tokens: BTreeMap<String, SecretString>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`context_tokens` maps a stable token reference such as `ctx_...` to the raw WeChat `context_token`. This lets conversations store references while keeping raw tokens in the sensitive account file.

Use a secret wrapper such as `secrecy::SecretString` or an equivalent project type so debug output cannot accidentally expose credentials.

## `monitor_state.json`

Provider output offsets and cursors.

```json
{
  "schema_version": 1,
  "sessions": {
    "ses_...": {
      "provider": "codex",
      "source": {
        "kind": "jsonl",
        "path": "/Users/example/.codex/sessions/...",
        "offset": 12345
      },
      "last_pane_hash": "sha256..."
    }
  }
}
```

Suggested Rust model:

```rust
pub struct MonitorState {
    pub schema_version: u32,
    pub sessions: BTreeMap<SessionId, MonitorSessionState>,
}

pub struct MonitorSessionState {
    pub provider: ProviderKind,
    pub source: Option<MonitorSourceState>,
    pub last_pane_hash: Option<String>,
}

pub struct MonitorSourceState {
    pub kind: OutputSourceKind,
    pub path: Option<PathBuf>,
    pub offset: Option<u64>,
    pub last_seen_id: Option<String>,
}
```

## `history.jsonl`

Append-only local history for user input, provider output, and lifecycle events.

Example lines:

```json
{"schema_version":1,"type":"user_input","session_id":"ses_...","conversation_id":"conv_...","text":"run tests","at":"2026-04-26T00:00:00Z"}
{"schema_version":1,"type":"provider_output","session_id":"ses_...","text":"Tests passed","at":"2026-04-26T00:00:01Z"}
{"schema_version":1,"type":"session_event","session_id":"ses_...","event":"created","provider":"codex","at":"2026-04-26T00:00:02Z"}
```

Do not store:

- WeChat `bot_token`
- `Authorization` headers
- `context_token`
- `typing_ticket`
- upload URLs
- raw credential payloads

Suggested Rust model:

```rust
pub struct HistoryLine {
    pub schema_version: u32,
    pub event: HistoryEvent,
}
```

`history` should redact before passing to `state.append_history`.

## Atomic Writes

For JSON state files:

1. Serialize to bytes.
2. Write to a temp file in the same directory.
3. Flush.
4. fsync where practical.
5. Rename over the target.

History is append-only JSONL. Appends should be line-buffered and tolerate partial trailing lines during recovery.

## Schema Versions

Every persisted structured file has `schema_version`.

Rules:

- Unknown newer schema should fail with a clear error.
- Older schema should migrate in `state::schema`.
- Business modules should use typed models, not raw `serde_json::Value`.
