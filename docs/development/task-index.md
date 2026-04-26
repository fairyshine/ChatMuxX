# Task Index

Status: draft.

This is the short execution index for [implementation-plan.md](implementation-plan.md).

## Build Order

1. A1: Create Rust Workspace
2. A2: Config Loader
3. A3: Logging and Redaction Foundation
4. B1: State Directory and Permissions
5. B2: Typed JSON Stores
6. B3: History JSONL
7. C1: Tmux Command Runner
8. C2: Managed Session Operations
9. D1: Provider Registry and Models
10. D2: Shell Provider
11. E1: Session Manager
12. E2: Mobile Command Parser
13. E3: Flow and Confirmation Engine
14. E4: Router
15. G2: Delivery Service
16. G3: Screenshot Command
17. D3: Codex Provider
18. D4: Claude Provider
19. F1: iLink Models and Client
20. F2: QR Login
21. F3: Long Polling and Inbound Text
22. F4: Outbound Text
23. G1: Monitor Runner
24. H1: App Wiring
25. H2: Action Executor
26. H3: End-to-End Manual Path
27. I1: Doctor Checks
28. I2: Error Messages and Backoff
29. I3: User Docs

## Early Usable Slice

The smallest non-WeChat slice that proves the local architecture:

1. workspace + config + state
2. tmux boundary
3. Shell provider
4. session manager
5. mobile command parser
6. fake channel event harness
7. delivery as terminal/stdout or test sink

This slice should prove that ChatMuxX can create a managed tmux window, send text, capture output, and close the session before WeChat is wired in.

## First Real WeChat Slice

The smallest WeChat slice:

1. `cmx login wechat`
2. WeChat long polling
3. owner authorization
4. `cmx help`
5. `cmx new /tmp shell`
6. send raw shell command
7. receive shell output

## Do Not Start Before v0.1

- Telegram adapter.
- file workflows.
- voice transcription.
- live view.
- natural-language-to-command generation.
- Claude/Codex hook/plugin/skill installation.
- SQLite storage backend.
- launchd/systemd service installation.
- web UI/dashboard.

