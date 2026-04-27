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

WeChat commands act on the current bound session when no session id is given. Local commands usually need a session id.

| Action | WeChat short | WeChat full | Local CLI |
| --- | --- | --- | --- |
| Help | `cmx h` | `cmx help` | `cmx help` |
| Create Codex session | `cmx n --id main /path codex` | `cmx new --id main /path codex` | `cmx n --id main /path codex` |
| List sessions | `cmx ls` | `cmx list` / `cmx sessions list` | `cmx ls` / `cmx list` |
| Switch session | `cmx sw main` | `cmx switch main` | use the session id in local commands |
| Rename session | `cmx mv main2` | `cmx rename main2` | `cmx mv main main2` / `cmx rename main main2` |
| Show terminal text | `cmx ss` | `cmx screenshot` / `cmx capture` | `cmx ss main` / `cmx capture main` |
| Send text | normal message | normal message | `cmx p main "echo hello" --enter` / `cmx send main "echo hello" --enter` |
| Interrupt | `cmx i` | `cmx interrupt` | `cmx i main` / `cmx interrupt main` |
| Enter | `cmx e` | `cmx enter` | `cmx e main` / `cmx enter main` |
| Escape | `cmx esc` | `cmx esc` | `cmx esc main` |
| Close session | `cmx rm` | `cmx close` | `cmx rm main` / `cmx close main` |
| Clean closed/dead sessions | `cmx clean` | `cmx prune` | `cmx clean` / `cmx prune` |

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
