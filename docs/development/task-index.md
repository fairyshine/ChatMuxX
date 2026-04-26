# Task Index

Status: draft.

This is the short execution index for [implementation-plan.md](implementation-plan.md).

## Build Order

1. A1: Create Rust Workspace
2. A2: Config Loader
3. A3: Logging and Redaction Foundation
4. Create baseline test scaffolding:
   - unit test module layout.
   - temp-dir helpers.
   - fake tmux/provider/state traits where needed.
   - gated tmux integration test placeholder.
5. B1: State Directory and Permissions
6. B2: Typed JSON Stores
7. B3: History JSONL
8. C1: Tmux Command Runner
9. C2: Managed Session Operations
10. D1: Provider Registry and Models
11. D2: Shell Provider
12. E1: Session Manager
13. E2: Mobile Command Parser
14. E3: Flow and Confirmation Engine
15. E4: Router
16. G2: Delivery Service
17. G3: Screenshot Command
18. D3: Codex Provider
19. D4: Claude Provider
20. F1: iLink Models and Client
21. F2: QR Login
22. F3: Long Polling and Inbound Text
23. F4: Outbound Text
24. G1: Monitor Runner
25. H1: App Wiring
26. H2: Action Executor
27. H3: End-to-End Manual Path
28. I1: Doctor Checks
29. I2: Error Messages and Backoff
30. I3: User Docs

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
