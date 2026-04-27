# ChatMuxX

English | [简体中文](README.zh-CN.md)

ChatMuxX lets you control local Codex, Claude Code, or shell sessions from WeChat.

The common flow is simple: run `cmx daemon` on your computer, send a message from WeChat, ChatMuxX forwards it to Codex running inside local tmux, then sends Codex output back to WeChat.

ChatMuxX does not install or modify Codex/Claude hooks, plugins, skills, or config files.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/master/scripts/install.sh | sh
```

If a new terminal cannot find `cmx`, add one of these lines to `~/.zshrc`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
alias cmx="$HOME/.cargo/bin/cmx"
```

Then reload zsh:

```bash
source ~/.zshrc
```

## Try It

You need `tmux`, Codex CLI logged in locally, and a WeChat account that can use the iLink Bot API.

1. Check your local setup:

```bash
cmx doctor
```

2. Connect WeChat:

```bash
cmx login wechat
```

Scan the QR code in your terminal.

3. Start the daemon:

```bash
cmx daemon
```

Keep this terminal window running. The daemon receives WeChat messages, controls tmux, and sends replies back.

4. Send this from WeChat:

```text
cmx n --id main /Users/you/Code/your-project codex
```

Replace the path with your real project directory.

5. Send a normal message from WeChat:

```text
Explain how this repo starts
```

Codex runs on your computer. The answer comes back to WeChat.

## Requirements

Check local tools:

```bash
tmux -V
codex --version
```

Normal WeChat text goes to Codex. Text starting with `cmx` controls ChatMuxX.

Provider-native `/...` commands are forwarded to Codex or Claude. ChatMuxX does not use `/` commands.

## Commands

WeChat commands act on the current bound session when no session id is given. Local CLI commands usually need a session id for pane actions.

| Action | Short | Full |
| --- | --- | --- |
| Help | `cmx h` | `cmx help` |
| Create Codex session | `cmx n --id main /path codex` | `cmx new --id main /path codex` |
| List sessions | `cmx ls` | `cmx list` / `cmx sessions list` |
| Switch session | `cmx sw main` | `cmx switch main` |
| Rename session | WeChat: `cmx mv main2`<br>Local: `cmx mv main main2` | WeChat: `cmx rename main2`<br>Local: `cmx rename main main2` |
| Show terminal text | WeChat: `cmx ss`<br>Local: `cmx ss main` | WeChat: `cmx screenshot`<br>Local: `cmx capture main` |
| Send text | WeChat: normal message<br>Local: `cmx p main "echo hello" --enter` | WeChat: normal message<br>Local: `cmx send main "echo hello" --enter` |
| Interrupt | WeChat: `cmx i`<br>Local: `cmx i main` | WeChat: `cmx interrupt`<br>Local: `cmx interrupt main` |
| Enter | WeChat: `cmx e`<br>Local: `cmx e main` | WeChat: `cmx enter`<br>Local: `cmx enter main` |
| Escape | WeChat: `cmx esc`<br>Local: `cmx esc main` | `cmx esc` |
| Close session | WeChat: `cmx rm`<br>Local: `cmx rm main` | WeChat: `cmx close`<br>Local: `cmx close main` |
| Clean closed/dead sessions | `cmx clean` | `cmx prune` |

## Update

For release installs, rerun the one-line installer to download the latest GitHub Release, including pre-releases.

Use `CHATMUXX_VERSION=latest-stable` if you only want the latest stable release.

## Local State

ChatMuxX stores local state under:

```text
~/.chatmuxx
```

Common files:

- `config.toml`: config
- `accounts.json`: WeChat login data
- `state.json`: sessions and bindings
- `history.jsonl`: message history
- `monitor_state.json`: tmux output tracking

## Status

ChatMuxX can already run the basic WeChat-to-local-Codex flow, but it is still early:

- The daemon runs in the foreground; it is not installed as a system service yet.
- `cmx ss` returns text, not a real screenshot.
- Claude provider, recovery, and multi-user policies are still being improved.

Developer and design docs live in [docs](docs/).
