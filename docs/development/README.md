# ChatMuxX Development Docs

Status: draft for v0.1 implementation planning.

本目录把 `docs/design` 中已经确认的设计转成可开发的工程文档。当前阶段仍然不写源码，先把仓库结构、模块边界、核心接口、状态文件和任务拆解写清楚。

## Documents

- [repository-architecture.md](repository-architecture.md): Rust workspace、crate、module 和目录结构设计。
- [core-interfaces.md](core-interfaces.md): channel、provider、session、tmux、monitor、delivery 等核心 Rust 接口草案。
- [state-and-storage.md](state-and-storage.md): `~/.chatmuxx` 本地状态文件、schema、权限和迁移策略。
- [wechat-ilink.md](wechat-ilink.md): 微信 iLink HTTP adapter 的 v0.1 实现流程。
- [implementation-plan.md](implementation-plan.md): v0.1 分阶段开发任务和验收点。
- [task-index.md](task-index.md): 可直接开工的任务顺序索引。
- [testing-strategy.md](testing-strategy.md): 单元测试、集成测试、假 iLink server、tmux 测试策略。

## v0.1 Implementation Goal

v0.1 的目标不是功能全，而是让 owner 可以真实用起来：

- `cmx daemon` 前台运行，并创建/连接 `chatmuxx` tmux session。
- `cmx login wechat` 通过 iLink HTTP 完成微信登录并保存账号状态。
- 微信文本消息可以创建、绑定、切换、关闭 ChatMuxX session。
- ChatMuxX session 可以启动 Codex、Claude Code、Shell。
- 手机端可以发送文本、provider slash command、截图、Esc、Enter、Ctrl-C。
- 输出通过 transcript/status 解析和 tmux pane fallback 回传到微信。
- 不安装 Claude/Codex hooks、plugins、skills，不改外部 CLI 配置。

## Coding Rules to Preserve

- Core is Rust-first.
- WeChat adapter uses direct iLink HTTP only; no OpenClaw runtime dependency.
- `cmx ...` is ChatMuxX mobile command space; `/...` is provider command space.
- `ChatMuxXSession.id` is the stable identity; tmux ids are runtime metadata.
- All tmux calls stay inside `tmux`.
- All state file IO stays inside `state`.
- Channel adapters do not know tmux details.
- Provider adapters do not know chat-platform details.

## Decision Workflow

Implementation-level technical details are decided in these development docs and can be refined during coding. Questions should be escalated back to the project owner only when they affect product behavior, security boundaries, v0.1 scope, or user-facing workflows.
