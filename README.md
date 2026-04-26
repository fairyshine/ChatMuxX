# ChatMuxX

English | [简体中文](README.zh-CN.md)

ChatMuxX lets you control local tmux windows running Codex, Claude Code, or a shell from mobile chat apps. The current v0.1 path focuses on WeChat through Tencent iLink HTTP and Codex running locally inside tmux.

ChatMuxX is a new Rust implementation. It does not depend on OpenClaw at runtime and does not install or modify Codex/Claude hooks, plugins, skills, or config files.

## Current Status

The project currently supports a minimal usable WeChat-to-Codex flow:

- QR login to WeChat iLink with `cmx login wechat`.
- Foreground daemon with `cmx daemon`.
- Create a Codex tmux window from WeChat.
- Forward normal WeChat text to the active Codex session.
- Capture tmux pane output and send it back to WeChat.
- List, switch, screenshot, interrupt, and close sessions with `cmx ...` commands.

Some parts are still early:

- Codex output is read from tmux pane capture, not structured transcript parsing.
- `cmx screenshot` currently sends text capture, not an image.
- `cmx close` has no confirmation prompt yet.
- The daemon runs in the foreground; no launchd/systemd service is installed yet.
- Claude provider, richer recovery, fake iLink integration tests, and multi-user policies are still in progress.

## Requirements

- macOS or Linux with `tmux` installed.
- Rust toolchain with `cargo`.
- Codex CLI installed and already logged in locally.
- A WeChat account that can use the iLink Bot API.

Check local tools:

```bash
tmux -V
codex --version
cargo --version
```

## Build

From the repository root:

```bash
cargo build
```

The development binary is:

```bash
target/debug/cmx
```

You can also install it into Cargo's local bin directory:

```bash
cargo install --path crates/cmx
```

After installation, use `cmx` directly instead of `target/debug/cmx`.

## First Run

Create the default config:

```bash
target/debug/cmx config init
```

This creates:

```text
~/.chatmuxx/config.toml
```

Run a local health check:

```bash
target/debug/cmx doctor
```

Log in to WeChat:

```bash
target/debug/cmx login wechat
```

The command prints a QR code in the terminal. Scan it with WeChat and confirm login. ChatMuxX stores the account token in:

```text
~/.chatmuxx/accounts.json
```

Start the daemon:

```bash
target/debug/cmx daemon
```

Keep this process running. It owns the WeChat long-poll loop, command routing, tmux input, and output delivery.

## Use From WeChat

Send ChatMuxX commands with the `cmx` prefix.

Create a Codex session in a real project directory:

```text
cmx new /Users/wumengsong/Code/ChatMuxX codex
```

After the session is created, send normal messages to the same WeChat conversation. They will be forwarded to Codex.

Useful commands:

```text
cmx help
cmx sessions
cmx switch <session-id>
cmx screenshot
cmx interrupt
cmx enter
cmx esc
cmx close
```

Provider-native slash commands are forwarded to the active CLI, so `/...` is not used for ChatMuxX commands.

## Local CLI Session Testing

You can test the tmux/provider path without WeChat:

```bash
target/debug/cmx sessions new /tmp codex
target/debug/cmx sessions list
target/debug/cmx sessions send <session-id> "hello" --enter
target/debug/cmx sessions capture <session-id>
target/debug/cmx sessions close <session-id>
```

Shell is also supported:

```bash
target/debug/cmx sessions new /tmp shell
```

## Config

Default config path:

```text
~/.chatmuxx/config.toml
```

Important defaults:

```toml
[daemon]
tmux_session = "chatmuxx"
poll_interval_ms = 1500

[wechat]
enabled = true
base_url = "https://ilinkai.weixin.qq.com"
bot_type = "3"
long_poll_timeout_ms = 38000

[providers.codex]
command = "codex"
args = []
env = {}

[providers.shell]
command = "bash"
args = []
env = {}
```

Provider CLIs must be started in a concrete workspace directory. Use `cmx new /absolute/path codex` from WeChat or `cmx sessions new /absolute/path codex` locally.

## State Files

ChatMuxX stores local state under:

```text
~/.chatmuxx
```

Common files:

```text
config.toml
accounts.json
state.json
monitor_state.json
history.jsonl
```

`accounts.json` contains sensitive WeChat credentials and should not be shared.

## Troubleshooting

If `cmx daemon` says no WeChat account exists, run:

```bash
target/debug/cmx login wechat
```

If WeChat commands are ignored, make sure you are sending from the same WeChat user that scanned the login QR code.

If Codex does not start, verify:

```bash
codex --version
```

If tmux windows are not created, verify:

```bash
tmux -V
```

Run:

```bash
target/debug/cmx doctor
```

## Development Docs

- [Design docs](docs/design/README.md)
- [Development docs](docs/development/README.md)
- [Reference projects](docs/development/reference-projects.md)
