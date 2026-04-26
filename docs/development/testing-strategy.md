# Testing Strategy

Status: draft.

## Test Layers

## Unit Tests

Run with `cargo test`.

Focus:

- mobile command parsing.
- config loading.
- state serialization and migration.
- redaction.
- provider launch command construction.
- provider transcript parsers with fixtures.
- delivery text splitting.
- authorization checks.

## Integration Tests

Integration tests may require local `tmux`.

Focus:

- ensure managed tmux session.
- create tmux window.
- send keys/text.
- capture pane.
- close managed window.
- session manager create/switch/close behavior against tmux.

Tests should use a unique tmux session name such as `chatmuxx-test-<pid>` and clean up after themselves.

## Fake WeChat iLink Server

Use a local HTTP test server to exercise the WeChat adapter without real network credentials.

Scenarios:

- QR login success.
- QR login timeout.
- `getupdates` returns no messages and advances cursor correctly.
- `getupdates` returns text message.
- adapter preserves `context_token`.
- `sendmessage` includes correct target user and context token.
- expired token maps to `ChannelAccountExpired`.

Suggested crates:

- `wiremock`
- `httpmock`
- or a small `axum` test server

## Fixture Tests

Store provider output fixtures under:

```text
crates/chatmuxx-core/tests/fixtures/
  codex/
  claude/
  shell/
```

Fixtures should be sanitized and must not include real tokens or private project data.

## Manual Smoke Tests

Before v0.1 is considered usable:

1. `cmx config init`
2. `cmx doctor`
3. `cmx login wechat`
4. `cmx daemon`
5. Send `cmx help` from WeChat.
6. Send `cmx new`.
7. Start a Shell session.
8. Run a simple command.
9. Start a Codex or Claude session.
10. Send normal text and provider `/help`.
11. Send `cmx screenshot`.
12. Send `cmx sessions`.
13. Send `cmx close` and confirm.

## CI Shape

Initial CI can run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Tmux integration tests may be gated behind an environment variable:

```bash
CHATMUXX_TEST_TMUX=1 cargo test --test tmux_integration
```

