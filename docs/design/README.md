# ChatMuxX Design Docs

Status: draft, awaiting product decisions.

This directory records the design of ChatMuxX: a bridge that lets mobile social apps such as Telegram and WeChat control local tmux windows running coding agents such as Codex and Claude Code.

## Documents

- [current-state.md](current-state.md): current repository facts and reference-project notes.
- [product-scope.md](product-scope.md): product goals, users, first release scope, and non-goals.
- [architecture.md](architecture.md): proposed runtime architecture, modules, flows, and state boundaries.
- [open-questions.md](open-questions.md): decisions that need owner confirmation before implementation design is considered stable.

## Design Principles

- Keep tmux as the source of truth for agent sessions.
- Support multiple chat channels through a common channel interface.
- Keep provider-specific agent behavior behind explicit provider interfaces.
- Keep state small, inspectable, and recoverable from local files.
- Favor modularity over copying one large reference implementation directly.
- Make the phone UX practical for real work: status, prompts, screenshots, files, and recovery should be first-class.

## Working Process

1. Capture known facts from this repository and references.
2. Mark unclear decisions in `open-questions.md`.
3. Confirm decisions with the project owner.
4. Update scope and architecture docs after each confirmed decision.
5. Only then turn design into development tasks under `docs/development`.
