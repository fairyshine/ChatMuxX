# Product Scope

Status: draft.

## Product Goal

ChatMuxX lets a user operate local CLI applications and coding-agent sessions from mobile chat apps. The user should be able to start or bind a tmux session, send instructions, receive output, respond to prompts, inspect terminal state, and recover from interrupted sessions without returning to the desktop.

## Target Users

- Primary: developers who run Codex, Claude Code, or similar agents locally and want mobile control while away from the computer.
- Secondary: developers who coordinate multiple agent sessions from phone chat threads.
- v0.1 targets one trusted owner.
- The architecture should reserve user identity, authorization, and ownership fields so multiple authorized users can be added later without rewriting the core model.

## Channel Direction

- WeChat is the first implementation priority for v0.1.
- Telegram is an early target and should be supported by the architecture even if it ships after WeChat.
- More chat applications should be able to integrate later through the same channel-adapter model.
- Some chat applications may not expose a usable official bot API. ChatMuxX should leave room for self-hosted bot bridges or custom connector processes in those cases.

## First-Class Agent Providers

- v0.1 providers:
  - Codex CLI.
  - Claude Code.
  - Shell sessions.
- Future providers:
  - Gemini.
  - Pi.
  - Other CLI applications with provider adapters.

## v0.1 Core Use Cases

1. Bind a mobile conversation to a tmux window.
2. Create a new tmux window from the mobile chat and launch a selected CLI/provider.
3. Send text from mobile chat to the active agent pane.
4. Relay agent responses back to the mobile chat.
5. Show agent status and completion state.
6. Render interactive prompts as mobile actions.
7. Capture and send terminal screenshots.
8. Recover or rebind stale/dead sessions.

## Future Use Cases

- Send files between chat app and local workspace.
- Support voice input after transcription.
- Provide live terminal view.
- Offer richer provider command discovery.
- Coordinate multiple agent sessions with agent-to-agent messaging.

## First Release Candidate Scope

Recommended minimal first release:

- Single-user local daemon.
- Authorization boundary designed around an owner identity, with room for future authorized users.
- WeChat channel adapter.
- Channel adapter interface that can later support Telegram and other chat apps.
- Codex, Claude Code, and Shell provider adapters.
- Mobile-initiated tmux window creation and provider launch.
- Existing tmux window binding.
- Text in/out.
- Basic status and prompt detection.
- Basic remote controls: screenshot, Esc, Ctrl-C, Enter.
- Basic session recovery for dead or stale windows.
- Local JSON state.
- CLI configuration.

Deferred unless explicitly prioritized:

- Full multi-user/team authorization UI and workflows.
- Cross-channel session sharing.
- Chat file upload/download workflows.
- Advanced media upload/download beyond basic files.
- File browser and rich file search.
- Voice transcription.
- Live terminal view.
- Agent-to-agent messaging.
- Rich command discovery.
- Gemini, Pi, and other provider adapters.
- Full web dashboard.
- Natural-language-to-command generation for Shell.

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
