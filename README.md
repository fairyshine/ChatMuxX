# ChatMuxX

English | [简体中文](README.zh-CN.md)

ChatMuxX lets you control local Codex, Claude Code, or shell sessions from WeChat.

The common flow is simple: run `cmux daemon` on your computer, send a message from WeChat, ChatMuxX forwards it to Codex running inside local tmux, then sends Codex output back to WeChat.

ChatMuxX does not install or modify Codex/Claude hooks, plugins, skills, or config files.

## Try It

Send this from WeChat:

```text
cmux n --id main /Users/you/Code/your-project codex
```

Then send a normal message:

```text
Explain how this repo starts
```

Codex runs on your computer. The answer comes back to WeChat.

## Install

Recommended one-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh | sh
```

The installer downloads the source and runs `cargo install`. After that, you get the `cmux` command.

If you want to inspect the installer first:

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh -o /tmp/chatmuxx-install.sh
less /tmp/chatmuxx-install.sh
sh /tmp/chatmuxx-install.sh
```

If a new terminal cannot find `cmux`, add this to your shell profile:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Requirements

You need:

- `tmux`
- Rust / Cargo
- Codex CLI, already logged in locally
- A WeChat account that can use the iLink Bot API

Check them:

```bash
tmux -V
cargo --version
codex --version
```

## First Run

1. Check your local setup:

```bash
cmux doctor
```

2. Log in to WeChat:

```bash
cmux login wechat
```

Scan the QR code in your terminal.

3. Start ChatMuxX:

```bash
cmux daemon
```

Keep this process running. It receives WeChat messages, controls tmux, and sends replies.

4. Create a Codex session from WeChat:

```text
cmux n --id main /Users/you/Code/your-project codex
```

Replace the path with your real project directory.

5. Chat normally:

```text
Explain this project
```

Normal text goes to Codex. Text starting with `cmux` controls ChatMuxX.

## WeChat Commands

```text
cmux h                         help
cmux n <project-path> codex     create a Codex session
cmux ls                        list sessions
cmux sw <session-id>           switch session
cmux mv <new-id>               rename current session
cmux ss                        show current terminal text
cmux i                         interrupt current task, like Ctrl-C
cmux e                         send Enter
cmux esc                       send Esc
cmux rm                        close current session
```

Provider-native `/...` commands are forwarded to Codex or Claude. ChatMuxX does not use `/` commands.

## Local Commands

You can test without WeChat:

```bash
cmux n --id test /tmp shell
cmux ls
cmux p test "echo hello" --enter
cmux cap test
cmux rm test
```

Local pane-control commands usually need a session id:

```bash
cmux i main
cmux e main
cmux esc main
```

## Update

Run the installer again:

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh | sh
```

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
- `cmux ss` returns text, not a real screenshot.
- Claude provider, recovery, and multi-user policies are still being improved.

Developer and design docs live in [docs](docs/).
