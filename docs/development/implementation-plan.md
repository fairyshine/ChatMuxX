# Implementation Plan

Status: draft.

This plan is written as implementation work packages. Each package should be small enough to become one or more issues/PRs.

## Milestone A: Project Foundation

Goal: the Rust workspace builds, `cmx` exists, and local config/state paths are established.

## A1. Create Rust Workspace

Modules/files:

- `Cargo.toml`
- `crates/cmx/Cargo.toml`
- `crates/cmx/src/main.rs`
- `crates/cmx/src/cli.rs`
- `crates/chatmuxx-core/Cargo.toml`
- `crates/chatmuxx-core/src/lib.rs`
- `crates/chatmuxx-core/src/error.rs`

Tasks:

- Create workspace with `cmx` binary crate and `chatmuxx-core` library crate.
- Add baseline dependencies:
  - `tokio`
  - `clap`
  - `serde`
  - `serde_json`
  - `toml`
  - `thiserror`
  - `tracing`
  - `tracing-subscriber`
- Define `ChatMuxXError` and project `Result<T>`.
- Add placeholder CLI commands.

Tests:

- `cargo test`
- `cargo fmt --check`
- `cargo clippy --all-targets`

Acceptance:

- `cmx --help` runs.
- `cmx daemon --help`, `cmx login wechat --help`, `cmx doctor --help` run.
- `chatmuxx-core` can be imported by `cmx`.

## A2. Config Loader

Modules/files:

- `config/model.rs`
- `config/loader.rs`
- `app/config_init.rs`

Tasks:

- Define `Config`, `DaemonConfig`, `OwnerConfig`, `WeChatConfig`, and provider config models.
- Implement defaults:
  - state dir: `~/.chatmuxx`
  - tmux session: `chatmuxx`
  - poll interval: `1500ms`
  - provider commands: `codex`, `claude`, `bash`
- Implement `cmx config init`.
- Refuse to overwrite existing config unless a future `--force` option is added.

Tests:

- default config serialization.
- config load with missing optional fields.
- config init creates parent directory.

Acceptance:

- `cmx config init` writes `~/.chatmuxx/config.toml`.
- loading config applies defaults predictably.

## A3. Logging and Redaction Foundation

Modules/files:

- `security/redact.rs`
- `app/logging.rs`

Tasks:

- Initialize `tracing`.
- Add redaction helpers for tokens, authorization headers, context tokens, upload URLs, and user IDs.
- Establish logging policy: no raw credentials in logs.

Tests:

- redaction helper masks expected fields.
- debug formatting for secret wrappers does not expose secret values.

Acceptance:

- logs are initialized by every CLI command.
- tests prove obvious secret strings are masked.

## Milestone B: State and Storage

Goal: local files under `~/.chatmuxx` are typed, permission-aware, and only accessed through `state`.

## B1. State Directory and Permissions

Modules/files:

- `state/files.rs`
- `security/permissions.rs`
- `app/doctor.rs`

Tasks:

- Create state directory with `0700`.
- Create sensitive files with `0600`.
- Implement permission checks.
- Add `cmx doctor` checks for state dir and account file permissions.

Tests:

- permission helper unit tests.
- doctor reports missing state dir clearly.

Acceptance:

- `cmx doctor` reports state dir status.
- state dir is created with restrictive permissions where supported.

## B2. Typed JSON Stores

Modules/files:

- `state/schema.rs`
- `state/sessions.rs`
- `state/accounts.rs`
- `state/monitor.rs`
- `state/atomic.rs`

Tasks:

- Implement typed models:
  - `AppState`
  - `AccountState`
  - `MonitorState`
- Implement atomic JSON writes.
- Implement schema version checks.
- Implement load-or-default behavior.

Tests:

- load missing files returns default state.
- save/load round trips.
- unknown newer schema errors.
- atomic write leaves valid file after successful save.

Acceptance:

- business modules can load/save typed state through `StateStore`.
- no direct JSON file writes outside `state`.

## B3. History JSONL

Modules/files:

- `history/event.rs`
- `history/redact.rs`
- `state/history.rs`

Tasks:

- Define `HistoryEvent`.
- Redact before persistence.
- Append JSON lines to `history.jsonl`.
- Tolerate a partial trailing line when reading future history.

Tests:

- append writes one JSON object per line.
- raw credential fields are not serialized into history.

Acceptance:

- user input, provider output, bridge commands, and session events can be recorded safely.

## Milestone C: Tmux Boundary

Goal: one module can manage the `chatmuxx` tmux session and windows.

## C1. Tmux Command Runner

Modules/files:

- `tmux/command.rs`
- `tmux/model.rs`
- `tmux/parser.rs`

Tasks:

- Wrap `tokio::process::Command`.
- Check whether `tmux` exists.
- Implement consistent error mapping.
- Parse tmux list output using stable separators.

Tests:

- parser unit tests with fixture output.
- command builder tests do not require tmux.

Acceptance:

- all tmux shelling happens in `tmux`.

## C2. Managed Session Operations

Modules/files:

- `tmux/mod.rs`

Tasks:

- Implement `ensure_managed_session`.
- Implement `list_windows`.
- Implement `create_window` with cwd and command vector.
- Implement `send_text`.
- Implement `send_key` for Enter/Escape/Ctrl-C/Tab.
- Implement `capture_pane`.
- Implement `close_window`.

Tests:

- gated integration test with `CHATMUXX_TEST_TMUX=1`.
- create unique test tmux session.
- create window, send command, capture output, close window.

Acceptance:

- `cmx doctor` can verify tmux availability.
- integration test cleans up its tmux session.

## Milestone D: Provider Layer

Goal: provider adapters can launch and monitor Codex, Claude, and Shell without modifying external app configuration.

## D1. Provider Registry and Models

Modules/files:

- `provider/model.rs`
- `provider/registry.rs`
- `provider/mod.rs`

Tasks:

- Define `ProviderKind`.
- Define `ProviderAdapter`.
- Define `ProviderCapabilities`.
- Implement registry for Codex, Claude, Shell.
- Load provider command/default args/env from config.

Tests:

- registry returns expected providers.
- unknown provider errors cleanly.

Acceptance:

- session manager can request adapter by `ProviderKind`.

## D2. Shell Provider

Modules/files:

- `provider/shell/mod.rs`
- `provider/shell/launch.rs`
- `provider/shell/pane.rs`

Tasks:

- Build shell launch command.
- Use raw text/key interaction.
- Parse minimal pane status.
- Treat pane capture as primary output.
- Do not implement NL-to-command.

Tests:

- launch command construction.
- pane output parsing fixtures.

Acceptance:

- Shell session can be launched in a workspace and receive raw commands.

## D3. Codex Provider

Modules/files:

- `provider/codex/mod.rs`
- `provider/codex/launch.rs`
- `provider/codex/transcript.rs`
- `provider/codex/status.rs`

Tasks:

- Build Codex launch command.
- Discover Codex transcript/JSONL source when available.
- Parse assistant output and basic status from fixtures.
- Fall back to tmux pane capture when transcript is unavailable.
- Do not install plugins or modify Codex config.

Tests:

- launch command construction with extra args.
- transcript fixture parser.
- fallback behavior when no transcript source exists.

Acceptance:

- monitor can emit `ProviderEvent` for Codex output.

## D4. Claude Provider

Modules/files:

- `provider/claude/mod.rs`
- `provider/claude/launch.rs`
- `provider/claude/transcript.rs`
- `provider/claude/status.rs`

Tasks:

- Build Claude launch command.
- Discover Claude transcript/status source when available.
- Parse assistant output and basic status from fixtures.
- Fall back to tmux pane capture when transcript/status source is unavailable.
- Do not install hooks, plugins, skills, or modify Claude config.

Tests:

- launch command construction with extra args.
- transcript/status fixture parser.
- fallback behavior when structured source is missing.

Acceptance:

- monitor can emit `ProviderEvent` for Claude output without installed hooks.

## Milestone E: Session, Router, and Mobile Commands

Goal: internal session lifecycle and text-first mobile command flows work without WeChat.

## E1. Session Manager

Modules/files:

- `session/model.rs`
- `session/manager.rs`
- `session/switching.rs`
- `session/recovery.rs`

Tasks:

- Implement `create_session`.
- Implement conversation binding.
- Implement switch active conversation binding.
- Implement close session.
- Implement list session summaries.
- Persist state through `StateStore`.
- Create tmux windows through `TmuxClient`.
- Launch providers through `ProviderRegistry`.

Tests:

- use fake `TmuxClient`, fake `ProviderRegistry`, in-memory `StateStore`.
- create/bind/switch/close lifecycle.
- provider replacement closes old managed window and creates new one after confirmation.

Acceptance:

- session lifecycle works without any channel adapter.

## E2. Mobile Command Parser

Modules/files:

- `router/mobile_command.rs`

Tasks:

- Parse mandatory v0.1 commands:
  - `cmx help`
  - `cmx new`
  - `cmx sessions`
  - `cmx switch`
  - `cmx close`
  - `cmx provider`
  - `cmx screenshot`
  - `cmx esc`
  - `cmx interrupt`
  - `cmx enter`
  - `cmx recover`
- Parse provider slash commands as `/...`.
- Parse provider extra args after `--`.
- Keep plain text untouched.

Tests:

- direct command parse cases.
- `/help` remains provider slash command.
- `cmx new /path codex -- --foo` preserves extra args.
- unknown `cmx` command returns clear error/help.

Acceptance:

- parser has no WeChat-specific logic.

## E3. Flow and Confirmation Engine

Modules/files:

- `session/confirmation.rs`
- `router/flow.rs`

Tasks:

- Implement active flow model.
- Implement text-first `cmx new` flow.
- Implement session switch flow.
- Implement destructive confirmation flow.
- Support `yes/no`, `cancel`, and numbered choices.
- Expire stale flows.

Tests:

- `cmx new` with missing provider asks provider.
- `cmx close` requires confirmation.
- `cmx interrupt` requires confirmation.
- `cmx esc` and `cmx enter` bypass confirmation.

Acceptance:

- all v0.1 flows work through plain text only.

## E4. Router

Modules/files:

- `router/mod.rs`
- `router/authorization.rs`

Tasks:

- Authorize owner.
- Route unbound conversation to onboarding.
- Route `cmx ...` to bridge command actions.
- Route `/...` to active provider.
- Route plain text to active provider.
- Emit delivery and history actions.

Tests:

- unauthorized sender cannot produce session actions.
- unbound conversation starts onboarding.
- bound plain text becomes provider input.
- provider slash commands are not swallowed by bridge commands.

Acceptance:

- router can be tested with fake state/session dependencies.

## Milestone F: WeChat iLink Adapter

Goal: text-only WeChat channel works through direct iLink HTTP.

## F1. iLink Models and Client

Modules/files:

- `channel/wechat/model.rs`
- `channel/wechat/client.rs`

Tasks:

- Define request/response structs for v0.1 endpoints.
- Implement authenticated headers.
- Generate `X-WECHAT-UIN`.
- Implement typed errors.
- Redact sensitive request/response fields.

Tests:

- header construction redacts in logs.
- request serialization matches fixture JSON.
- error response maps to typed error.

Acceptance:

- client can be tested against fake HTTP server.

## F2. QR Login

Modules/files:

- `channel/wechat/auth.rs`
- `app/login.rs`

Tasks:

- Implement `cmx login wechat`.
- Render QR in terminal.
- Poll login status.
- Persist account credentials.
- Initialize owner if needed.

Tests:

- fake server login success.
- expired QR returns clear error.
- owner initialization rules.

Acceptance:

- real manual QR login can persist `accounts.json`.

## F3. Long Polling and Inbound Text

Modules/files:

- `channel/wechat/polling.rs`
- `channel/wechat/conversation.rs`

Tasks:

- Implement `ChannelAdapter::run`.
- Persist `get_updates_buf`.
- Convert text messages to `InboundText`.
- Derive direct/group conversation ids.
- Store `context_token` by reference.
- Emit `ChannelAccountExpired`.

Tests:

- fake `getupdates` with no messages.
- fake `getupdates` with direct text.
- fake `getupdates` with group text.
- cursor persists after success.
- context token is not emitted raw.

Acceptance:

- adapter can feed normalized events into a channel sink.

## F4. Outbound Text

Modules/files:

- `channel/wechat/send.rs`

Tasks:

- Implement text `sendmessage`.
- Resolve latest context token by reference.
- Return typed error if token is missing.
- Ignore or reject unsupported outbound media in v0.1.

Tests:

- fake server receives expected text payload.
- missing context token produces user-facing error path.

Acceptance:

- delivery can send text replies to WeChat conversations.

## Milestone G: Monitor and Delivery

Goal: provider output is deduped, transformed, and sent back to channel conversations.

## G1. Monitor Runner

Modules/files:

- `monitor/runner.rs`
- `monitor/offsets.rs`

Tasks:

- Poll active sessions.
- Ask provider for structured output source.
- Read provider events and update cursor.
- Use pane fallback where needed.
- Persist monitor state.
- Dedupe repeated pane output with hash.

Tests:

- fake provider emits events once.
- cursor updates.
- pane hash suppresses duplicate output.

Acceptance:

- monitor emits normalized `ProviderEvent`.

## G2. Delivery Service

Modules/files:

- `delivery/mod.rs`
- `delivery/renderer.rs`
- `delivery/rate_limit.rs`

Tasks:

- Map `ProviderEvent` to outbound text.
- Split long text.
- Truncate with `cmx screenshot` hint.
- Throttle status messages.
- Render confirmation requests as plain text fallback.
- Send through channel adapter.

Tests:

- long text split/truncate behavior.
- status throttling.
- confirmation fallback text.

Acceptance:

- delivery has no WeChat-specific business logic.

## G3. Screenshot Command

Modules/files:

- `app/screenshot.rs`
- `delivery/renderer.rs`

Tasks:

- Implement `cmx screenshot` from active session.
- Capture pane.
- v0.1 may send text capture first.
- Add image renderer later within same command path.

Tests:

- fake tmux pane capture delivered as text.
- missing active session returns clear message.

Acceptance:

- owner can inspect current terminal from mobile chat.

## Milestone H: Daemon Orchestration

Goal: `cmx daemon` wires all modules and supports the v0.1 manual workflow.

## H1. App Wiring

Modules/files:

- `app/daemon.rs`

Tasks:

- Load config.
- Initialize state store.
- Ensure managed tmux session.
- Load WeChat accounts.
- Start WeChat adapter tasks.
- Start monitor polling task.
- Receive channel events through `mpsc`.
- Dispatch router actions.
- Dispatch delivery events.
- Handle Ctrl-C shutdown.

Tests:

- unit test app wiring with fake channel/router/delivery.
- graceful shutdown test at component level.

Acceptance:

- `cmx daemon` starts without WeChat account and tells user to run `cmx login wechat`.
- `cmx daemon` can process fake channel events in tests.

## H2. Action Executor

Modules/files:

- `app/action_executor.rs`

Tasks:

- Execute `AppAction` values.
- Send provider input through tmux.
- Start/switch/close sessions.
- Record history.
- Trigger delivery.
- Keep side effects out of router.

Tests:

- fake dependencies validate each action path.

Acceptance:

- router remains side-effect light.

## H3. End-to-End Manual Path

Manual scenario:

1. `cmx config init`
2. `cmx doctor`
3. `cmx login wechat`
4. `cmx daemon`
5. send `cmx help`
6. send `cmx new /tmp shell`
7. send `pwd`
8. receive shell output
9. send `cmx sessions`
10. send `cmx screenshot`
11. send `cmx close`
12. confirm close

Acceptance:

- scenario works on a local machine with tmux and WeChat login.

## Milestone I: Hardening and Documentation

Goal: first usable release is understandable and resilient.

## I1. Doctor Checks

Tasks:

- check tmux availability.
- check managed tmux session status.
- check provider commands exist in PATH or configured path.
- check state dir permissions.
- check WeChat login/account presence.
- check config parse.

Acceptance:

- `cmx doctor` gives actionable output.

## I2. Error Messages and Backoff

Tasks:

- user-facing errors for missing active session, missing provider command, expired WeChat token, missing context token.
- WeChat polling retry/backoff.
- avoid duplicate spam during repeated failures.

Acceptance:

- transient network failures do not kill daemon immediately.
- expired account tells user to run `cmx login wechat`.

## I3. User Docs

Tasks:

- update root README quick start.
- document v0.1 commands.
- document `cmx` vs provider `/...` command separation.
- document no hooks/plugins/config changes for Codex/Claude.

Acceptance:

- a new user can follow README to run a Shell session from WeChat.
