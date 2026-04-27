# Testing Strategy

Status: draft.

Testing should let ChatMuxX evolve quickly without needing real WeChat or real agent CLIs for most checks. The default test suite must be fast, deterministic, and offline.

## Test Layers

## Layer 1: Unit Tests

Run by default:

```bash
cargo test
```

Rules:

- no network.
- no real WeChat credentials.
- no real tmux process.
- no real Codex/Claude requirement.
- use temp directories for config/state.

Coverage:

- mobile command parsing.
- config defaults and loading.
- state serialization, schema versions, and migrations.
- atomic write helpers with temp dirs.
- redaction and secret debug output.
- provider launch command construction.
- provider transcript/status parsers with fixtures.
- delivery text splitting and truncation.
- status throttling.
- authorization checks.
- flow state transitions.

## Layer 2: Component Tests With Fakes

Run by default when they do not require external binaries.

Use fake implementations for:

- `TmuxClient`
- `ProviderAdapter`
- `ProviderRegistry`
- `StateStore`
- `ChannelAdapter`
- `DeliveryService`

Target behavior:

- session manager create/bind/switch/close.
- provider replacement closes old managed window and creates a new one.
- router converts channel events into app actions.
- action executor applies side effects in the correct order.
- monitor dedupes provider events and persists cursors.

Example fake:

```rust
struct FakeTmuxClient {
    windows: Mutex<Vec<TmuxWindow>>,
    sent: Mutex<Vec<(TmuxPaneId, String)>>,
}
```

## Layer 3: Tmux Integration Tests

These require local `tmux` and are opt-in.

Run with:

```bash
CHATMUXX_TEST_TMUX=1 cargo test --test tmux_integration
```

Rules:

- use a unique tmux session name such as `chatmuxx-test-<pid>`.
- clean up the test tmux session on success and failure.
- never touch the real `chatmuxx` session.
- skip with a clear message if `tmux` is unavailable.

Coverage:

- ensure managed tmux session.
- list windows.
- create tmux window with cwd.
- send keys/text.
- capture pane.
- close managed window.
- session manager behavior against real tmux can be added after the tmux boundary is stable.

## Layer 4: Fake WeChat iLink Server Tests

These exercise the WeChat adapter without real credentials.

Suggested implementation:

- use `wiremock`, `httpmock`, or a small local `axum` server.
- bind to localhost on an ephemeral port.
- point `WeChatClient.base_url` to the fake server.

Scenarios:

- QR login success.
- QR login expired.
- QR login cancelled.
- `getupdates` returns no messages and advances cursor.
- `getupdates` returns direct text message.
- `getupdates` returns group text message.
- adapter derives conversation id correctly.
- adapter keeps group conversation identity separate from sender identity.
- adapter stores `context_token` by reference.
- `sendmessage` includes correct target user and context token.
- missing context token returns typed error.
- expired account response maps to `ChannelAccountExpired`.
- transient HTTP/network errors trigger retry/backoff behavior at the adapter loop level.

## Layer 5: Provider Fixture Tests

Store sanitized fixtures under:

```text
crates/chatmuxx-core/tests/fixtures/
  codex/
    session-basic.jsonl
    status-waiting-input.jsonl
  claude/
    transcript-basic.jsonl
    status-approval.txt
  shell/
    pane-command-output.txt
```

Rules:

- fixtures must be synthetic or sanitized.
- no real tokens.
- no private project paths unless replaced with placeholders.
- parser tests should assert normalized `ProviderEvent`, not raw parser internals.

Coverage:

- assistant messages.
- status changes.
- approval/prompt detection when visible.
- malformed entries are ignored or produce clear parser errors.
- pane fallback output extraction.

## Layer 6: Daemon Harness Tests

After app wiring exists, add a local harness that uses fake channel events.

Goal:

- prove the daemon can process events without WeChat.

Scenario:

1. start app with fake channel adapter.
2. inject owner `InboundText("cmx help")`.
3. assert delivery receives help text.
4. inject `InboundText("cmx new /tmp shell")`.
5. assert session manager action is called.
6. inject plain text.
7. assert provider input is sent.

This keeps the core loop testable without real network or tmux.

## Manual Smoke Tests

Before v0.1 is considered usable:

1. `cmx config init`
2. `cmx doctor`
3. `cmx login wechat`
4. `cmx daemon`
5. Send `cmx help` from WeChat.
6. Send `cmx new /tmp shell`.
7. Send `pwd`.
8. Receive shell output.
9. Send provider `/help` in a Codex or Claude session.
10. Send `cmx screenshot`.
11. Send `cmx sessions`.
12. Send `cmx close`.
13. Confirm close.
14. Stop daemon with Ctrl-C.
15. Restart daemon and verify state loads.

## CI Shape

Initial CI:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Optional jobs:

```bash
CHATMUXX_TEST_TMUX=1 cargo test --test tmux_integration
```

Fake iLink tests should run in normal CI because they require no real network credentials.

## Test Data and Secrets

Rules:

- never commit real WeChat tokens.
- never commit real `context_token`.
- never commit real Authorization headers.
- mask user IDs and project paths in fixtures.
- keep fixtures small and explain what each fixture covers.

## Module Test Matrix

| Module | Unit | Fake Component | Integration |
| --- | --- | --- | --- |
| `config` | defaults, parse errors | n/a | n/a |
| `state` | schema, atomic write, redaction | temp-dir store | n/a |
| `tmux` | parsers, command building | n/a | real tmux |
| `provider` | launch, fixtures | fake session | optional real CLI later |
| `router` | command routing, authorization | fake state/session | n/a |
| `session` | model transitions | fake tmux/provider/state | optional real tmux |
| `channel/wechat` | model serialization | fake iLink server | manual real login |
| `monitor` | cursor/dedupe | fake provider/state | n/a |
| `delivery` | splitting/throttling | fake channel | n/a |
| `app` | wiring | fake channel/action executor | manual smoke |

