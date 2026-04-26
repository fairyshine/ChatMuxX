# Core Interfaces

Status: draft. Rust snippets are interface sketches, not final code.

## Conventions

Snippets assume project-local imports and aliases such as:

```rust
pub type Result<T> = std::result::Result<T, ChatMuxXError>;
```

Common external types used in sketches:

- `DateTime<Utc>` from `chrono`.
- `Serialize` and `Deserialize` from `serde`.
- `BTreeMap` from `std::collections`.
- `PathBuf` from `std::path`.
- `Arc` from `std::sync`.
- `Bytes` from `bytes`.

## ID Types

Use newtype wrappers instead of raw strings across module boundaries.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ChannelAccountId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TmuxWindowId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TmuxPaneId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OwnerId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SenderId(pub String);
```

## Common Message Models

Common models are the contract between channel adapters, router, delivery, and state. Platform-specific fields are wrapped instead of leaking raw protocol structs across modules.

```rust
#[derive(Clone, Debug)]
pub struct ConversationRef {
    pub id: ConversationId,
    pub account_id: ChannelAccountId,
    pub channel_type: ChannelType,
    pub external_id: String,
    pub kind: ConversationKind,
}

#[derive(Clone, Debug)]
pub enum ConversationKind {
    Direct,
    Group,
}

#[derive(Clone, Debug)]
pub struct SenderRef {
    pub id: SenderId,
    pub display_name: Option<String>,
    pub is_owner_hint: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PlatformMeta {
    pub wechat: Option<WeChatMeta>,
    pub telegram: Option<TelegramMeta>,
    pub external: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct WeChatMeta {
    pub from_user_id: String,
    pub to_user_id: String,
    pub group_id: Option<String>,
    pub context_token_ref: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TelegramMeta {
    pub chat_id: String,
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
}
```

Sensitive values such as a WeChat `context_token` should be stored by reference or in channel state. Logs and history must not print the raw token.

## Outbound Message Models

Delivery produces channel-independent outbound messages; channel adapters render them with platform-specific APIs.

```rust
#[derive(Clone, Debug)]
pub enum OutboundMessage {
    Text(OutboundText),
    ActionPrompt(OutboundActionPrompt),
    Image(OutboundImage),
    File(OutboundFile),
}

#[derive(Clone, Debug)]
pub struct OutboundText {
    pub text: String,
    pub formatting: TextFormatting,
    pub importance: MessageImportance,
}

#[derive(Clone, Debug)]
pub enum TextFormatting {
    Plain,
    Markdown,
}

#[derive(Clone, Debug)]
pub enum MessageImportance {
    Normal,
    Status,
    Error,
}

#[derive(Clone, Debug)]
pub struct OutboundActionPrompt {
    pub title: String,
    pub body: String,
    pub actions: Vec<ActionChoice>,
    pub fallback_text: String,
}

#[derive(Clone, Debug)]
pub struct ActionChoice {
    pub id: String,
    pub label: String,
    pub style: ActionStyle,
}

#[derive(Clone, Debug)]
pub enum ActionStyle {
    Default,
    Destructive,
    Cancel,
}

#[derive(Clone, Debug)]
pub struct DeliveryReceipt {
    pub platform_message_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OutboundImage {
    pub bytes: Bytes,
    pub mime_type: String,
    pub caption: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OutboundFile {
    pub path: PathBuf,
    pub display_name: Option<String>,
    pub caption: Option<String>,
}
```

v0.1 only requires text and screenshot image delivery. `OutboundFile` is reserved for future file workflows.

## Channel Adapter

Channel adapters normalize platform events and send platform messages.

```rust
#[async_trait::async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn channel_type(&self) -> ChannelType;
    fn capabilities(&self) -> ChannelCapabilities;

    async fn run(&self, sink: ChannelEventSink) -> Result<()>;
    async fn send_message(
        &self,
        conversation: &ConversationId,
        message: OutboundMessage,
    ) -> Result<DeliveryReceipt>;
}
```

`run()` owns the platform receive loop. For WeChat this means long polling, retry/backoff, cursor advancement, and account-expired detection. The daemon receives normalized events through `ChannelEventSink`.

```rust
#[derive(Clone)]
pub struct ChannelEventSink {
    tx: tokio::sync::mpsc::Sender<ChannelEvent>,
}

impl ChannelEventSink {
    pub async fn send(&self, event: ChannelEvent) -> Result<()> {
        self.tx.send(event).await.map_err(Into::into)
    }
}
```

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChannelType {
    WeChat,
    Telegram,
    External(String),
}

#[derive(Clone, Debug)]
pub struct ChannelCapabilities {
    pub buttons: bool,
    pub edit_message: bool,
    pub images: bool,
    pub files: bool,
    pub max_text_len: usize,
}
```

```rust
#[derive(Clone, Debug)]
pub enum ChannelEvent {
    InboundText(InboundText),
    InboundAction(InboundAction),
    InboundFile(InboundFile),
    ConversationStarted(ConversationRef),
    ChannelAccountExpired(ChannelAccountId),
}

#[derive(Clone, Debug)]
pub struct InboundText {
    pub account_id: ChannelAccountId,
    pub conversation: ConversationRef,
    pub sender: SenderRef,
    pub text: String,
    pub received_at: DateTime<Utc>,
    pub platform_meta: PlatformMeta,
}

#[derive(Clone, Debug)]
pub struct InboundAction {
    pub account_id: ChannelAccountId,
    pub conversation: ConversationRef,
    pub sender: SenderRef,
    pub action_id: String,
    pub received_at: DateTime<Utc>,
    pub platform_meta: PlatformMeta,
}

#[derive(Clone, Debug)]
pub struct InboundFile {
    pub account_id: ChannelAccountId,
    pub conversation: ConversationRef,
    pub sender: SenderRef,
    pub file_name: Option<String>,
    pub media_type: MediaType,
    pub platform_meta: PlatformMeta,
}

#[derive(Clone, Debug)]
pub enum MediaType {
    Image,
    Voice,
    File,
    Video,
    Unknown,
}
```

`platform_meta` stores channel-only metadata such as WeChat `context_token`, but core logic should not inspect raw protocol fields except through typed helpers.

## WeChat iLink Adapter

The WeChat adapter uses direct HTTP calls.

```rust
pub struct WeChatClient {
    base_url: Url,
    http: reqwest::Client,
}

impl WeChatClient {
    pub async fn get_bot_qrcode(&self) -> Result<QrLoginStart>;
    pub async fn poll_qrcode_status(&self, qrcode: &str) -> Result<QrLoginStatus>;
    pub async fn get_updates(
        &self,
        token: &BotToken,
        cursor: Option<&GetUpdatesBuf>,
    ) -> Result<GetUpdatesResponse>;
    pub async fn send_message(
        &self,
        token: &BotToken,
        message: WeChatSendMessage,
    ) -> Result<()>;
}
```

v0.1 WeChat adapter responsibilities:

- QR login.
- Persist token through `state`.
- Long-poll `getupdates`.
- Advance `get_updates_buf`.
- Convert text messages into `ChannelEvent::InboundText`.
- Preserve latest `context_token` in conversation metadata.
- Send text replies with the correct `context_token`.

Do not implement in v0.1 unless needed for text flow:

- CDN media upload/download.
- typing indicators.
- multiple simultaneous WeChat accounts.

## Mobile Command Parser

`cmx ...` is the ChatMuxX mobile command space.

```rust
pub enum MobileCommand {
    Help,
    New(NewSessionArgs),
    Sessions,
    Switch { session_id: Option<SessionId> },
    Close { session_id: Option<SessionId> },
    Provider { provider: Option<ProviderKind> },
    Screenshot,
    Esc,
    Interrupt,
    Enter,
    Recover { session_id: Option<SessionId> },
}

pub struct NewSessionArgs {
    pub workspace: Option<PathBuf>,
    pub provider: Option<ProviderKind>,
    pub extra_args: Vec<String>,
}

pub enum ParsedInbound {
    BridgeCommand(MobileCommand),
    ProviderSlashCommand(String),
    PlainText(String),
    FlowReply(String),
}

pub fn parse_mobile_text(text: &str, flow: Option<&ActiveFlow>) -> ParsedInbound;
```

Routing rules:

- `cmx ...` becomes `BridgeCommand`.
- `/...` becomes `ProviderSlashCommand`.
- plain text becomes `PlainText` unless an onboarding/confirmation flow is active.
- active flows may interpret numbered replies or short tokens as `FlowReply`.

## Router

The router converts authorized channel events into application actions.

```rust
#[async_trait::async_trait]
pub trait Router: Send + Sync {
    async fn handle_channel_event(&self, event: ChannelEvent) -> Result<Vec<AppAction>>;
}

pub enum AppAction {
    StartFlow {
        conversation: ConversationId,
        flow: ActiveFlow,
    },
    SendToProvider {
        session_id: SessionId,
        input: ProviderInput,
    },
    ExecuteBridgeCommand {
        conversation: ConversationId,
        command: MobileCommand,
    },
    Deliver(DeliveryEvent),
    RecordHistory(HistoryEvent),
}

pub enum ProviderInput {
    Text(String),
    SlashCommand(String),
    Key(TmuxKey),
}
```

Router responsibilities:

- authorize sender before producing side-effect actions.
- parse `cmx ...` bridge commands.
- route `/...` to the active provider as provider-native slash commands.
- route plain text to the active provider when a conversation is bound.
- start onboarding when a conversation is unbound.
- interpret confirmation/session-selection replies through `ActiveFlow`.

## Flows and Confirmations

Flows hold short-lived conversation state for onboarding, provider replacement, recovery, and destructive confirmations.

```rust
pub enum ActiveFlow {
    NewSession(NewSessionFlow),
    SwitchSession(SwitchSessionFlow),
    Confirm(ConfirmationFlow),
    Recover(RecoveryFlow),
}

pub struct NewSessionFlow {
    pub conversation: ConversationId,
    pub workspace: Option<PathBuf>,
    pub provider: Option<ProviderKind>,
    pub extra_args: Vec<String>,
    pub step: NewSessionStep,
}

pub enum NewSessionStep {
    AskWorkspace,
    AskProvider,
    ConfirmCreate,
}

pub struct SwitchSessionFlow {
    pub conversation: ConversationId,
    pub candidates: Vec<SessionSummary>,
}

pub struct RecoveryFlow {
    pub conversation: ConversationId,
    pub session_id: SessionId,
    pub choices: Vec<RecoveryChoice>,
}

pub enum RecoveryChoice {
    Fresh,
    Continue,
    Resume { provider_session_id: String },
    Cancel,
}

pub struct ConfirmationFlow {
    pub id: String,
    pub conversation: ConversationId,
    pub action: PendingAction,
    pub expires_at: DateTime<Utc>,
}

pub enum PendingAction {
    CloseSession { session_id: SessionId },
    ReplaceProvider {
        old_session_id: SessionId,
        provider: ProviderKind,
        workspace: PathBuf,
        extra_args: Vec<String>,
    },
    Interrupt { session_id: SessionId },
    RecoverFresh { session_id: SessionId },
}

pub struct ConfirmationRequest {
    pub conversation: ConversationId,
    pub flow_id: String,
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub destructive: bool,
}
```

v0.1 confirmation rules:

- `cmx close`, `cmx interrupt`, provider replacement, and recovery replacement/fresh actions require confirmation.
- `cmx esc` and `cmx enter` do not require confirmation.
- flows must work through plain text replies such as `1`, `yes`, `no`, or `cancel`.

`cmx new` text behavior:

- `cmx new /absolute/path codex` should create a Codex session in the given workspace when authorization and validation pass.
- `cmx new /absolute/path claude -- --model opus` may pass provider extra args after a delimiter if supported by the parser.
- `cmx new` without enough arguments starts a text-first onboarding flow.
- The onboarding flow must work without buttons: show numbered provider choices, recent/default workspace suggestions, and allow direct path input.
- Complex directory browsing is deferred.

## Session Manager

```rust
#[async_trait::async_trait]
pub trait SessionManager: Send + Sync {
    async fn create_session(&self, req: CreateSessionRequest) -> Result<ChatMuxXSession>;
    async fn bind_conversation(
        &self,
        conversation: ConversationId,
        session_id: SessionId,
    ) -> Result<()>;
    async fn switch_conversation(
        &self,
        conversation: ConversationId,
        session_id: SessionId,
    ) -> Result<()>;
    async fn close_session(&self, session_id: SessionId, reason: CloseReason) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>>;
    async fn active_session_for(
        &self,
        conversation: &ConversationId,
    ) -> Result<Option<ChatMuxXSession>>;
}
```

```rust
pub struct CreateSessionRequest {
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub launch_mode: LaunchMode,
    pub extra_args: Vec<String>,
    pub owner: OwnerId,
    pub conversation: Option<ConversationId>,
}

pub enum LaunchMode {
    Fresh,
    Continue,
    Resume { provider_session_id: String },
}

pub enum CloseReason {
    UserRequested,
    ProviderReplacement,
    RecoveryReplacement,
    DeadSessionCleanup,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMuxXSession {
    pub id: SessionId,
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub status: SessionStatus,
    pub tmux: Option<TmuxAttachment>,
    pub owner: OwnerId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TmuxAttachment {
    pub session_name: String,
    pub window_id: TmuxWindowId,
    pub pane_id: TmuxPaneId,
    pub created_by_chatmuxx: bool,
    pub adopted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SessionStatus {
    Starting,
    Running,
    WaitingInput,
    Done,
    Dead,
    Closed,
}

pub struct SessionSummary {
    pub id: SessionId,
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub status: SessionStatus,
    pub display_name: String,
}
```

Rules:

- `ChatMuxXSession.id` is stable.
- tmux window/pane ids are metadata.
- closing only affects managed/adopted windows.
- provider replacement confirms, closes old window, creates a new session/window, then updates binding.

## Tmux Boundary

Only this module may run `tmux`.

```rust
#[async_trait::async_trait]
pub trait TmuxClient: Send + Sync {
    async fn ensure_managed_session(&self, name: &str) -> Result<()>;
    async fn list_windows(&self, session_name: &str) -> Result<Vec<TmuxWindow>>;
    async fn create_window(&self, req: CreateTmuxWindow) -> Result<TmuxWindow>;
    async fn send_text(&self, pane: &TmuxPaneId, text: &str) -> Result<()>;
    async fn send_key(&self, pane: &TmuxPaneId, key: TmuxKey) -> Result<()>;
    async fn capture_pane(&self, pane: &TmuxPaneId) -> Result<String>;
    async fn close_window(&self, window: &TmuxWindowId) -> Result<()>;
}
```

```rust
pub struct CreateTmuxWindow {
    pub session_name: String,
    pub window_name: String,
    pub cwd: PathBuf,
    pub command: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub struct TmuxWindow {
    pub session_name: String,
    pub window_id: TmuxWindowId,
    pub pane_id: TmuxPaneId,
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub active: bool,
}
```

`cwd` should be passed to tmux window creation directly. Higher-level modules should not build ad-hoc `cd ... && command` shell strings.

```rust
pub enum TmuxKey {
    Enter,
    Escape,
    CtrlC,
    Tab,
}
```

Implementation can call `tmux` CLI through `tokio::process::Command`.

## Provider Adapter

```rust
#[async_trait::async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> ProviderCapabilities;
    fn launch_command(&self, req: &ProviderLaunchRequest) -> Result<ProviderLaunchCommand>;
    async fn discover_output(&self, session: &ChatMuxXSession) -> Result<OutputSource>;
    async fn read_events(
        &self,
        session: &ChatMuxXSession,
        cursor: Option<ProviderCursor>,
    ) -> Result<ProviderReadResult>;
    async fn parse_pane_status(&self, pane_text: &str) -> Result<Option<ProviderEvent>>;
}
```

```rust
pub enum ProviderKind {
    Codex,
    Claude,
    Shell,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Codex => "codex",
            ProviderKind::Claude => "claude",
            ProviderKind::Shell => "shell",
        }
    }
}

pub struct ProviderCapabilities {
    pub structured_output: bool,
    pub resume: bool,
    pub continue_last: bool,
    pub prompt_detection: bool,
}

pub trait ProviderRegistry: Send + Sync {
    fn get(&self, kind: ProviderKind) -> Result<Arc<dyn ProviderAdapter>>;
    fn list(&self) -> Vec<ProviderKind>;
}
```

```rust
pub struct ProviderLaunchRequest {
    pub workspace: PathBuf,
    pub launch_mode: LaunchMode,
    pub extra_args: Vec<String>,
}

pub struct ProviderLaunchCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub display_name: String,
}

pub enum OutputSource {
    Structured {
        kind: OutputSourceKind,
        path: PathBuf,
    },
    PaneOnly,
}

pub enum OutputSourceKind {
    CodexJsonl,
    ClaudeTranscript,
    TmuxPane,
}

pub struct ProviderCursor {
    pub source_kind: OutputSourceKind,
    pub offset: Option<u64>,
    pub last_seen_id: Option<String>,
    pub pane_hash: Option<String>,
}

pub struct ProviderReadResult {
    pub events: Vec<ProviderEvent>,
    pub next_cursor: ProviderCursor,
}
```

Provider launch rules:

- Codex, Claude, and Shell start in a concrete workspace directory.
- Provider default command, args, and env come from `config.toml`.
- Provider adapters own provider-specific arguments for fresh/continue/resume modes.
- User-configured extra args are appended according to provider-specific rules.
- tmux owns the actual cwd setup through `CreateTmuxWindow.cwd`.
- Avoid constructing shell command strings unless the Shell provider explicitly needs shell semantics.

v0.1 provider behavior:

- Codex: structured transcript/JSONL when available, pane fallback.
- Claude: transcript/status parsing and pane fallback, no hook/plugin install.
- Shell: raw text/key interaction, pane capture primary.

## Provider Events

```rust
pub enum ProviderEvent {
    AssistantMessage {
        session_id: SessionId,
        text: String,
        source: OutputSourceKind,
    },
    StatusChanged {
        session_id: SessionId,
        status: ProviderStatus,
    },
    PromptRequested {
        session_id: SessionId,
        prompt: Prompt,
    },
    ToolActivity {
        session_id: SessionId,
        label: String,
        status: ToolStatus,
    },
    CommandOutput {
        session_id: SessionId,
        text: String,
    },
    SessionFinished {
        session_id: SessionId,
    },
    SessionFailed {
        session_id: SessionId,
        reason: String,
    },
}

pub enum ProviderStatus {
    Starting,
    Running,
    WaitingInput,
    Done,
    Failed,
}

pub struct Prompt {
    pub title: String,
    pub body: String,
    pub choices: Vec<PromptChoice>,
}

pub struct PromptChoice {
    pub id: String,
    pub label: String,
}

pub enum ToolStatus {
    Started,
    Running,
    Finished,
    Failed,
}
```

## Monitor

```rust
#[async_trait::async_trait]
pub trait MonitorRunner: Send + Sync {
    async fn tick(&self) -> Result<Vec<ProviderEvent>>;
}
```

v0.1 can start with polling:

- poll active sessions every configurable interval.
- read structured provider source if available.
- use pane fallback where needed.
- update `monitor_state.json`.
- emit deduped `ProviderEvent` values.

## Delivery

```rust
#[async_trait::async_trait]
pub trait DeliveryService: Send + Sync {
    async fn deliver(&self, event: DeliveryEvent) -> Result<()>;
}

pub enum DeliveryEvent {
    Provider(ProviderEvent),
    SystemMessage {
        conversation: ConversationId,
        text: String,
    },
    ConfirmationRequest(ConfirmationRequest),
    SessionList {
        conversation: ConversationId,
        sessions: Vec<SessionSummary>,
    },
}
```

Delivery rules:

- use channel capabilities.
- split before truncate.
- throttle status.
- no automatic summary in v0.1.

Screenshot delivery:

- `cmx screenshot` is modeled as a bridge command that asks `tmux` to capture pane text and a renderer to produce an image.
- The first implementation may send text capture if image rendering is not ready.
- Once image rendering exists, delivery sends `OutboundMessage::Image`.

## History Events

History is semantic and redacted before state persistence.

```rust
pub enum HistoryEvent {
    UserInput {
        conversation: ConversationId,
        session_id: Option<SessionId>,
        text: String,
        at: DateTime<Utc>,
    },
    ProviderOutput {
        session_id: SessionId,
        text: String,
        at: DateTime<Utc>,
    },
    SessionEvent {
        session_id: SessionId,
        event: SessionHistoryKind,
        at: DateTime<Utc>,
    },
    BridgeCommand {
        conversation: ConversationId,
        command: String,
        at: DateTime<Utc>,
    },
}

pub enum SessionHistoryKind {
    Created,
    Bound,
    Switched,
    Closed,
    Recovered,
    ProviderReplaced,
}
```

History must not contain credentials, raw WeChat `context_token`, authorization headers, upload URLs, or raw account payloads.

## State Store

```rust
#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    async fn load_state(&self) -> Result<AppState>;
    async fn save_state(&self, state: &AppState) -> Result<()>;
    async fn load_accounts(&self) -> Result<AccountState>;
    async fn save_accounts(&self, accounts: &AccountState) -> Result<()>;
    async fn load_monitor_state(&self) -> Result<MonitorState>;
    async fn save_monitor_state(&self, monitor: &MonitorState) -> Result<()>;
    async fn append_history(&self, event: HistoryEvent) -> Result<()>;
}
```

Only `state` should know file paths and JSON layout.
