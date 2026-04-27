# ChatMuxX

English | [简体中文](README.zh-CN.md)

ChatMuxX lets you control local tmux windows running Codex, Claude Code, or a shell from mobile chat apps. The current v0.1 path focuses on WeChat through Tencent iLink HTTP and Codex running locally inside tmux.

It is a new Rust implementation. It does not depend on OpenClaw at runtime and does not install or modify Codex/Claude hooks, plugins, skills, or config files.

## Quick Start

```bash
cargo build
target/debug/cmux config init
target/debug/cmux doctor
target/debug/cmux login wechat
target/debug/cmux daemon
```

Then send this from WeChat:

```text
cmux new /absolute/path/to/your/project codex
```

After the session is created, keep chatting in the same WeChat conversation. Normal messages are forwarded to Codex, while commands that start with `cmux` control ChatMuxX.

## Why ChatMuxX

ChatMuxX is useful when you want to keep a coding agent running on your own machine, but control it from a phone:

- Start a local Codex session from WeChat.
- Send prompts without exposing your workstation over SSH.
- Keep the real agent process inside tmux for inspection and recovery.
- Route mobile commands separately from provider-native slash commands.
- Store state in local files that are easy to inspect and back up.

## Current Status

The project currently supports a minimal usable WeChat-to-Codex flow:

- QR login to WeChat iLink with `cmux login wechat`.
- Foreground daemon with `cmux daemon`.
- Create a Codex tmux window from WeChat.
- Forward normal WeChat text to the active Codex session.
- Capture tmux pane output and send it back to WeChat.
- List, switch, screenshot, interrupt, Enter/Esc, and close sessions with `cmux ...` commands.

Some parts are still early:

- Codex output is read from tmux pane capture, not structured transcript parsing.
- `cmux screenshot` currently sends text capture, not an image.
- `cmux close` has no confirmation prompt yet.
- The daemon runs in the foreground; no launchd/systemd service is installed yet.
- Claude provider, richer recovery, fake iLink integration tests, and multi-user policies are still in progress.

## Architecture

ChatMuxX keeps the chat channel, routing logic, providers, tmux control, and state storage separated:

```text
WeChat / mobile chat
        |
        v
cmux daemon  -> mobile command router -> session manager
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
target/debug/cmux --help
```

Install `cmux` into Cargo's local bin directory:

```bash
cargo install --path crates/cmux
```

After installation, use `cmux` directly:

```bash
cmux --help
```

Run a quick local check:

```bash
target/debug/cmux doctor
```

Start the foreground daemon:

```bash
target/debug/cmux daemon
```

List local sessions:

```bash
target/debug/cmux sessions list
```

Create a local test session without WeChat:

```bash
target/debug/cmux sessions new /tmp shell
```

## First Run

Create the default config:

```bash
target/debug/cmux config init
```

This creates:

```text
~/.chatmuxx/config.toml
```

Run a local health check:

```bash
target/debug/cmux doctor
```

Log in to WeChat:

```bash
target/debug/cmux login wechat
```

The command prints a QR code in the terminal. Scan it with WeChat and confirm login. ChatMuxX stores the account token in:

```text
~/.chatmuxx/accounts.json
```

Start the daemon:

```bash
target/debug/cmux daemon
```

Keep this process running. It owns the WeChat long-poll loop, command routing, tmux input, and output delivery.

## Use From WeChat

Send ChatMuxX commands with the `cmux` prefix.

Create a Codex session in a real project directory:

```text
cmux new --id main /Users/wumengsong/Code/ChatMuxX codex
```

After the session is created, send normal messages to the same WeChat conversation. They will be forwarded to Codex. If `--id` is omitted, ChatMuxX creates a short id such as `s-mogm4ctu`.

Useful commands:

```text
cmux help
cmux sessions
cmux switch <session-id>
cmux rename [session-id] <new-id>
cmux screenshot
cmux interrupt
cmux enter
cmux esc
cmux close
```

Provider-native slash commands are forwarded to the active CLI, so `/...` is not used for ChatMuxX commands.

## Local CLI Session Testing

You can test the tmux/provider path without WeChat:

```bash
target/debug/cmux sessions new --id main /tmp codex
target/debug/cmux sessions list
target/debug/cmux sessions send <session-id> "hello" --enter
target/debug/cmux sessions capture <session-id>
target/debug/cmux sessions rename <session-id> <new-id>
target/debug/cmux sessions close <session-id>
```

Shell is also supported:

```bash
target/debug/cmux sessions new /tmp shell
```

CLI subcommands currently exposed by `cmux`:

```text
cmux daemon [--config <path>]
cmux login wechat
cmux doctor
cmux config init [--path <path>]
cmux sessions new [--id <id>] <workspace> [provider] [-- <extra-provider-args>...]
cmux sessions list
cmux sessions rename <session-id> <new-id>
cmux sessions send <session-id> <text> [--enter]
cmux sessions capture <session-id>
cmux sessions close <session-id>
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

Provider CLIs must be started in a concrete workspace directory. Use `cmux new /absolute/path codex` from WeChat or `cmux sessions new /absolute/path codex` locally.

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

If `cmux daemon` says no WeChat account exists, run:

```bash
target/debug/cmux login wechat
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
target/debug/cmux doctor
```

## Repository Layout

```text
crates/cmux/              # user-facing cmux CLI
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
