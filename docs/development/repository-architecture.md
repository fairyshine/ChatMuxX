# Repository Architecture

Status: draft.

## Workspace Shape

ChatMuxX should start as a small Rust workspace with one binary crate and one core library crate:

```text
ChatMuxX/
  Cargo.toml
  crates/
    cmx/
      Cargo.toml
      src/
        main.rs
        cli.rs
    chatmuxx-core/
      Cargo.toml
      src/
        lib.rs
        app/
        channel/
        command/
        config/
        delivery/
        error.rs
        history/
        monitor/
        provider/
        router/
        security/
        session/
        state/
        tmux/
        types/
  docs/
    design/
    development/
  references/
    ccgram/
    openclaw-weixin/
```

Why two crates:

- `crates/cmx` owns CLI parsing, terminal output, and process entrypoints.
- `crates/chatmuxx-core` owns reusable daemon logic, traits, models, adapters, and tests.
- `chatmuxx-core` is the shared core for the foreground `cmx daemon` now and for future launchd/systemd service wrappers later.
- Future external connectors or libraries can depend on `chatmuxx-core` without depending on CLI code.

Naming notes:

- `chatmuxx-core` follows common Rust workspace naming for a reusable core library crate.
- Rust code imports this crate as `chatmuxx_core`.
- `crates/cmx` is named after the user-facing binary command.
- Future crates can follow the same pattern, for example `chatmuxx-connector-protocol` or `chatmuxx-test-support`.

## Root `Cargo.toml`

The root should be a workspace:

```toml
[workspace]
members = [
  "crates/cmx",
  "crates/chatmuxx-core",
]
resolver = "2"
```

## Binary Crate: `crates/cmx`

Responsibilities:

- Parse CLI commands.
- Initialize logging.
- Load config.
- Call into `chatmuxx-core`.
- Convert user-facing errors into readable terminal output.

Initial commands:

```text
cmx daemon
cmx login wechat
cmx doctor
cmx config init
cmx sessions list
cmx sessions close <session-id>
```

Suggested dependencies:

- `clap`: CLI parsing.
- `tokio`: async runtime.
- `tracing` and `tracing-subscriber`: logs.
- `chatmuxx-core`: core logic.

## Core Crate: `crates/chatmuxx-core`

Suggested top-level modules:

```text
src/
  lib.rs
  error.rs
  types/
    ids.rs
    time.rs
  app/
    daemon.rs
    login.rs
    doctor.rs
  config/
    model.rs
    loader.rs
  channel/
    mod.rs
    wechat/
      mod.rs
      auth.rs
      client.rs
      model.rs
      polling.rs
      send.rs
      conversation.rs
  connector/
    mod.rs
    protocol.rs
  router/
    mod.rs
    mobile_command.rs
    authorization.rs
  session/
    mod.rs
    manager.rs
    model.rs
    recovery.rs
  tmux/
    mod.rs
    command.rs
    model.rs
    parser.rs
  provider/
    mod.rs
    registry.rs
    model.rs
    codex.rs
    claude.rs
    shell.rs
  monitor/
    mod.rs
    runner.rs
    offsets.rs
  delivery/
    mod.rs
    renderer.rs
    rate_limit.rs
  state/
    mod.rs
    files.rs
    atomic.rs
    schema.rs
    accounts.rs
    sessions.rs
    monitor.rs
    history.rs
  history/
    mod.rs
    event.rs
  security/
    mod.rs
    redact.rs
    permissions.rs
```

Some modules should be split further once implementation starts:

```text
channel/wechat/
  auth.rs
  client.rs
  model.rs
  polling.rs
  send.rs
  conversation.rs

session/
  manager.rs
  model.rs
  recovery.rs
  switching.rs
  confirmation.rs

provider/codex/
  mod.rs
  launch.rs
  transcript.rs
  status.rs

provider/claude/
  mod.rs
  launch.rs
  transcript.rs
  status.rs

provider/shell/
  mod.rs
  launch.rs
  pane.rs

state/
  files.rs
  atomic.rs
  schema.rs
  accounts.rs
  sessions.rs
  monitor.rs
  history.rs
```

## Module Ownership

`app`

- Owns high-level workflows: daemon startup, login, doctor checks.
- Wires channel adapters, router, session manager, monitor, delivery, and state store together.

`channel`

- Defines channel adapter traits and common inbound/outbound event models.
- `channel/wechat` implements direct iLink HTTP login, long polling, and text sending.
- Must not call tmux or provider modules directly.
- Should not own generic delivery policy such as long-text splitting, status throttling, or action fallback wording.

`router`

- Parses mobile `cmx ...` commands.
- Authorizes inbound events.
- Resolves the active `ChatMuxXSession`.
- Starts onboarding, session switching, and confirmation flows.
- Owns message intent routing: whether an inbound message is a ChatMuxX command, provider slash command, plain provider input, or active flow reply.
- Does not create tmux windows directly.

`session`

- Owns stable ChatMuxX session lifecycle.
- Creates, binds, switches, closes, and recovers sessions.
- Calls `tmux` through the tmux boundary only.
- Persists state through `state` only.
- Does not parse channel text or know WeChat command syntax.

`tmux`

- The only module allowed to execute `tmux` commands.
- Creates/attaches the managed `chatmuxx` tmux session.
- Lists windows, creates windows, sends keys, captures panes, closes adopted windows.

`provider`

- Defines provider traits and implementations for Codex, Claude Code, and Shell.
- Includes `provider/registry.rs` to resolve adapters by `ProviderKind`.
- Builds launch commands.
- Finds/reads structured output when available.
- Parses provider status and prompts.
- Must not know WeChat, Telegram, or channel message formats.
- Launch behavior must include workspace/cwd and configurable provider arguments.

`monitor`

- Polls provider output sources and pane fallback.
- Tracks offsets.
- Emits normalized `ProviderEvent` values.

`delivery`

- Converts internal outbound events into channel messages.
- Splits long text, throttles status, and renders action fallback.
- Owns generic outbound policy: what text to send, when to throttle, how to split, and how to render actions when buttons are unavailable.
- Calls channel adapters only through their outbound message interface.

`state`

- Owns all local files under `~/.chatmuxx`.
- Performs atomic writes, schema-version checks, and permission checks.
- Business modules must not edit JSON/TOML files directly.
- Includes `state/history.rs` as the storage implementation for appending `history.jsonl`.

`history`

- Records local chat/session history events in `history.jsonl`.
- Excludes credentials and sensitive protocol fields.
- Owns history semantics: event types, what should be recorded, and how sensitive fields are removed before persistence.
- Does not write files directly; it persists through `state`.

`security`

- Owner authorization, redaction, file-permission helpers.

## Dependency Direction

Allowed direction:

```text
cmx -> app -> router/session/monitor/delivery
channel -> common models only
provider -> tmux model/common models only
session -> tmux/provider/state
monitor -> provider/tmux/state
delivery -> channel/common models
state -> filesystem only
```

Avoid:

- `channel/wechat` importing `tmux`.
- `provider/*` importing `channel`.
- random modules writing files under `~/.chatmuxx`.
- shelling out to `tmux` outside `tmux`.
