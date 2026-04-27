# Open Questions

Status: active decisions only. Confirmed decisions are kept here briefly so the remaining unknowns have context.

## Confirmed Decisions

- ChatMuxX is a complete new implementation, not a ccgram fork.
- ccgram is only an experience and design reference.
- WeChat is the first implementation priority for v0.1.
- Telegram and more chat apps should be supported by the architecture later.
- Some future chat apps may connect through external connector processes if they lack usable bot APIs.
- v0.1 targets one trusted owner, with identity and authorization fields reserved for future multi-user support.
- ChatMuxX core should be written in Rust.
- The CLI binary should be `cmux`.
- WeChat should use direct iLink HTTP calls. ChatMuxX must not depend on OpenClaw runtime, gateway, plugin installation, or OpenClaw account storage.
- v0.1 WeChat support focuses on QR login, token persistence, long polling, `sendmessage`, and text conversation flow.
- v0.1 providers are Codex CLI, Claude Code, and Shell.
- Gemini, Pi, richer provider adapters, voice, live view, file workflows, and agent-to-agent messaging are future enhancements.
- Mobile ChatMuxX commands use the `cmux ...` prefix. Provider-native `/...` commands are forwarded to the active CLI.
- ChatMuxX uses stable internal `session_id` values; tmux window/pane IDs are runtime metadata.
- v0.1 uses one managed tmux session, defaulting to `chatmuxx`.
- Mobile chat must support listing, switching, closing, and creating managed windows/sessions.
- Default provider switch behavior replaces the current managed window after confirmation.
- Output monitoring should prefer structured provider sources for Codex/Claude and use tmux pane capture as fallback. Shell uses pane capture as primary source.
- v0.1 state storage uses local files with `schema_version`, atomic writes, and a state module boundary. SQLite and OS keychain are future options.
- v0.1 runs as foreground `cmux daemon`; launchd/systemd service installation is deferred.
- `cmux daemon` should create or attach the managed `chatmuxx` tmux session from any terminal.
- ChatMuxX should not install Claude/Codex hooks, plugins, skills, or configuration changes in v0.1. Provider integrations should keep external CLI app configurations clean.
- Claude and Codex monitoring should use transcript/status parsing and tmux pane fallback instead of installed hooks/plugins.
- Shell provider uses raw shell command/text interaction. Natural-language-to-command generation is not a planned goal unless explicitly reopened later.
- Provider replacement requires confirmation, then closes the old managed tmux window and creates/binds a new provider window. v0.1 does not need an archive/keep branch; users can use `cmux new` or `cmux sessions` to keep old work explicit.
- v0.1 sends primary assistant output as original text where possible. Status updates should be throttled. Long output is split first, then truncated with a hint to use `cmux screenshot`; automatic summarization is not used.
- Mandatory v0.1 mobile commands: `cmux help`, `cmux new`, `cmux sessions`, `cmux switch`, `cmux close`, `cmux provider`, `cmux screenshot`, `cmux esc`, `cmux interrupt`, `cmux enter`, and `cmux recover`.
- v0.1 must prevent unapproved users from controlling ChatMuxX. Owner identity may be initialized during WeChat login/pairing and may also be configured explicitly. Once an owner exists, all inbound commands/messages must be checked against that owner identity.
- High-impact actions require confirmation in v0.1: `cmux close`, provider replacement, recovery replacement/fresh actions, and `cmux interrupt`. `cmux esc` and `cmux enter` do not require confirmation.
- v0.1 may save local chat/session history for history, recovery, and debugging. Runtime logs, chat history, and credentials must be stored separately. Credentials and sensitive protocol fields must not be written into normal logs or chat history.
- Major decisions stay in the design docs for now; ADR files are deferred.
- `docs/development` is the v0.1 task breakdown and interface-design area.

## Remaining Questions

No active open questions at this checkpoint. New questions should be added here when implementation planning reveals unclear decisions.
