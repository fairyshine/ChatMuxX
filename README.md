# ChatMuxX

English | [简体中文](README.zh-CN.md)

ChatMuxX lets you control local tmux windows running Codex, Claude Code, or a shell from mobile chat apps. The current v0.1 path focuses on WeChat through Tencent iLink HTTP and Codex running locally inside tmux.

It is a new Rust implementation. It does not depend on OpenClaw at runtime and does not install or modify Codex/Claude hooks, plugins, skills, or config files.

## Quick Start

```bash
cargo build
target/debug/cmx config init
target/debug/cmx doctor
target/debug/cmx login wechat
target/debug/cmx daemon
```

Then send this from WeChat:

```text
cmx new /absolute/path/to/your/project codex
```

After the session is created, keep chatting in the same WeChat conversation. Normal messages are forwarded to Codex, while commands that start with `cmx` control ChatMuxX.

## Why ChatMuxX

ChatMuxX is useful when you want to keep a coding agent running on your own machine, but control it from a phone:

- Start a local Codex session from WeChat.
- Send prompts without exposing your workstation over SSH.
- Keep the real agent process inside tmux for inspection and recovery.
- Route mobile commands separately from provider-native slash commands.
- Store state in local files that are easy to inspect and back up.

## Current Status

The project currently supports a minimal usable WeChat-to-Codex flow:

- QR login to WeChat iLink with `cmx login wechat`.
- Foreground daemon with `cmx daemon`.
- Create a Codex tmux window from WeChat.
- Forward normal WeChat text to the active Codex session.
- Capture tmux pane output and send it back to WeChat.
- List, switch, screenshot, interrupt, Enter/Esc, and close sessions with `cmx ...` commands.

Some parts are still early:

- Codex output is read from tmux pane capture, not structured transcript parsing.
- `cmx screenshot` currently sends text capture, not an image.
- `cmx close` has no confirmation prompt yet.
- The daemon runs in the foreground; no launchd/systemd service is installed yet.
- Claude provider, richer recovery, fake iLink integration tests, and multi-user policies are still in progress.

## Architecture

ChatMuxX keeps the chat channel, routing logic, providers, tmux control, and state storage separated:

```text
WeChat / mobile chat
        |
        v
cmx daemon  -> mobile command router -> session manager
        |                                  |
        |                                  v
        |                              tmux windows
        |                                  |
        v                                  v
 local state files                 Codex / Claude / shell
```

Important boundaries:

- Chat channels know how to receive and send messages, but not tmux internals.
- Provider adapters know how to launch CLIs, but not chat-platform details.
- Session state uses stable ChatMuxX session IDs; tmux IDs are runtime metadata.
- All persistent state lives under `~/.chatmuxx` by default.

## Requirements

- macOS or Linux with `tmux` installed.
- Rust toolchain with `cargo`.
- Codex CLI installed and already logged in locally.
- A WeChat account that can use the iLink Bot API.
- Optional: Claude Code CLI if you want to experiment with the Claude provider path later.

Check local tools:

```bash
tmux -V
codex --version
cargo --version
```

## Basic Operations

Build the development binary:

```bash
cargo build
```

Run the local binary:

```bash
target/debug/cmx --help
```

Install `cmx` into Cargo's local bin directory:

```bash
cargo install --path crates/cmx
```

After installation, use `cmx` directly:

```bash
cmx --help
```

Run a quick local check:

```bash
target/debug/cmx doctor
```

Start the foreground daemon:

```bash
target/debug/cmx daemon
```

List local sessions:

```bash
target/debug/cmx sessions list
```

Create a local test session without WeChat:

```bash
target/debug/cmx sessions new /tmp shell
```

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

CLI subcommands currently exposed by `cmx`:

```text
cmx daemon [--config <path>]
cmx login wechat
cmx doctor
cmx config init [--path <path>]
cmx sessions new <workspace> [provider] [-- <extra-provider-args>...]
cmx sessions list
cmx sessions send <session-id> <text> [--enter]
cmx sessions capture <session-id>
cmx sessions close <session-id>
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

[owner]
wechat_user_id = ""

[wechat]
enabled = true
base_url = "https://ilinkai.weixin.qq.com"
bot_type = "3"
long_poll_timeout_ms = 38000

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

## Security Notes

- Treat `~/.chatmuxx/accounts.json` as a secret file.
- Run ChatMuxX only on machines where local tmux, Codex, Claude, and shell access are acceptable from your mobile chat account.
- Use absolute workspace paths so the provider starts in the intended project directory.
- Review output before sharing logs; transcripts and pane captures may contain code, paths, prompts, or secrets.
- The current v0.1 flow is designed for a single owner and does not yet implement rich multi-user policy controls.

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

Run the full local check:

```bash
target/debug/cmx doctor
```

## Repository Layout

```text
crates/cmx/              # user-facing cmx CLI
crates/chatmuxx-core/    # config, daemon, sessions, tmux, providers, channels, state
docs/design/             # product and architecture design notes
docs/development/        # implementation docs, task index, testing strategy
```

## Development Docs

- [Design docs](docs/design/README.md)
- [Development docs](docs/development/README.md)
- [Repository architecture](docs/development/repository-architecture.md)
- [State and storage](docs/development/state-and-storage.md)
- [Testing strategy](docs/development/testing-strategy.md)
- [Reference projects](docs/development/reference-projects.md)
