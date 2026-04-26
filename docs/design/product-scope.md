# Product Scope

Status: draft. Items marked `TBD` need confirmation.

## Product Goal

ChatMuxX lets a user operate local coding-agent sessions from mobile chat apps. The user should be able to start or bind a tmux session, send instructions, receive agent output, respond to prompts, inspect terminal state, and recover from interrupted sessions without returning to the desktop.

## Target Users

- Primary: developers who run Codex, Claude Code, or similar agents locally and want mobile control while away from the computer.
- Secondary: developers who coordinate multiple agent sessions from phone chat threads.
- TBD: whether ChatMuxX is intended only for a single trusted owner or for small trusted teams.

## First-Class Channels

- Telegram: expected to be supported, with ccgram as the main UX reference.
- WeChat: expected to be supported, using the iLink/OpenClaw reference.
- TBD: whether both channels are required for the first release or whether one channel should ship first.

## First-Class Agent Providers

- Codex CLI.
- Claude Code.
- Shell sessions.
- TBD: Gemini, Pi, and other providers.

## Core Use Cases

1. Bind a mobile conversation to a tmux window.
2. Send text from mobile chat to the active agent pane.
3. Relay agent responses back to the mobile chat.
4. Show agent status and completion state.
5. Render interactive prompts as mobile actions.
6. Capture and send terminal screenshots.
7. Recover or rebind stale/dead sessions.
8. Send files between chat app and local workspace.
9. Support voice input after transcription.

## First Release Candidate Scope

Recommended minimal first release:

- Single-user local daemon.
- One channel adapter.
- Codex and Claude provider adapters.
- Existing tmux window binding.
- Text in/out.
- Basic status and prompt detection.
- Local JSON state.
- CLI configuration.

Deferred unless explicitly prioritized:

- Multi-user/team authorization.
- Cross-channel session sharing.
- Media upload/download beyond basic files.
- Voice transcription.
- Live terminal view.
- Agent-to-agent messaging.
- Rich command discovery.
- Full web dashboard.

## Non-Goals

- ChatMuxX should not host remote coding agents itself.
- ChatMuxX should not replace tmux as the session source of truth.
- ChatMuxX should not require users to abandon desktop terminal workflows.
- ChatMuxX should not directly expose arbitrary shell access to untrusted users.

## Success Criteria

- A user can walk away from the computer and keep a coding agent session moving from phone chat.
- Desktop and phone remain synchronized because both control the same tmux session.
- A daemon restart does not lose channel-to-window bindings.
- Provider- and channel-specific code can evolve independently.

