# ChatMuxX

[English](README.md) | 简体中文

ChatMuxX 可以让你通过移动端聊天应用控制本机 tmux 窗口中的 Codex、Claude Code 或 shell。当前 v0.1 路线重点支持通过腾讯 iLink HTTP 接入微信，并在本机 tmux 中运行 Codex。

ChatMuxX 是一个全新的 Rust 实现。它运行时不依赖 OpenClaw，也不会安装或修改 Codex/Claude 的 hooks、插件、skills 或配置文件。

## 快速开始

```bash
cargo build
target/debug/cmx config init
target/debug/cmx doctor
target/debug/cmx login wechat
target/debug/cmx daemon
```

然后从微信发送：

```text
cmx new /absolute/path/to/your/project codex
```

会话创建后，继续在同一个微信会话里聊天即可。普通消息会转发给 Codex，以 `cmx` 开头的消息用于控制 ChatMuxX。

## 为什么需要 ChatMuxX

当你希望编码 Agent 继续运行在自己的电脑上，但又想用手机远程控制它时，ChatMuxX 会很有用：

- 从微信启动本地 Codex 会话。
- 无需把工作站暴露到 SSH，也能发送提示词。
- 真实的 Agent 进程保留在 tmux 中，便于检查和恢复。
- 移动端控制命令和 provider 原生 slash commands 分离。
- 状态保存在易检查、易备份的本地文件中。

## 当前状态

项目目前已经支持一条最小可用的「微信到 Codex」流程：

- 使用 `cmx login wechat` 通过二维码登录微信 iLink。
- 使用 `cmx daemon` 启动前台守护进程。
- 从微信创建 Codex tmux 窗口。
- 将普通微信文本转发到当前活跃的 Codex 会话。
- 捕获 tmux pane 输出并回传到微信。
- 使用 `cmx ...` 命令列出、切换、截图、中断、发送 Enter/Esc 和关闭会话。

仍处于早期阶段的部分：

- Codex 输出来自 tmux pane capture，还不是结构化 transcript 解析。
- `cmx screenshot` 目前发送的是文本捕获结果，不是图片。
- `cmx close` 还没有确认提示。
- daemon 目前以前台进程运行，尚未安装 launchd/systemd 服务。
- Claude provider、更完整的恢复能力、假的 iLink 集成测试和多用户策略仍在推进中。

## 架构

ChatMuxX 将聊天通道、路由逻辑、provider、tmux 控制和状态存储分离：

```text
微信 / 移动端聊天应用
        |
        v
cmx daemon  -> 移动端命令路由 -> 会话管理器
        |                             |
        |                             v
        |                         tmux 窗口
        |                             |
        v                             v
 本地状态文件                  Codex / Claude / shell
```

重要边界：

- 聊天通道只负责收发消息，不了解 tmux 内部细节。
- Provider adapter 只负责启动 CLI，不了解聊天平台细节。
- 会话状态使用稳定的 ChatMuxX session ID；tmux ID 只是运行时元数据。
- 所有持久化状态默认保存在 `~/.chatmuxx` 下。

## 环境要求

- 安装了 `tmux` 的 macOS 或 Linux。
- Rust 工具链和 `cargo`。
- 已安装并在本机完成登录的 Codex CLI。
- 一个可以使用 iLink Bot API 的微信账号。
- 可选：如果后续想试验 Claude provider 路径，需要 Claude Code CLI。

检查本机工具：

```bash
tmux -V
codex --version
cargo --version
```

## 构建

在仓库根目录执行：

```bash
cargo build
```

开发版二进制文件位于：

```bash
target/debug/cmx
```

也可以安装到 Cargo 本地 bin 目录：

```bash
cargo install --path crates/cmx
```

安装后可以直接使用 `cmx`，无需再输入 `target/debug/cmx`。

## 首次运行

创建默认配置：

```bash
target/debug/cmx config init
```

该命令会创建：

```text
~/.chatmuxx/config.toml
```

执行本地健康检查：

```bash
target/debug/cmx doctor
```

登录微信：

```bash
target/debug/cmx login wechat
```

命令会在终端中打印二维码。使用微信扫码并确认登录。ChatMuxX 会把账号 token 存储在：

```text
~/.chatmuxx/accounts.json
```

启动 daemon：

```bash
target/debug/cmx daemon
```

保持该进程持续运行。它负责微信 long-poll 循环、命令路由、tmux 输入和输出投递。

## 从微信使用

发送以 `cmx` 为前缀的 ChatMuxX 命令。

在真实项目目录中创建一个 Codex 会话：

```text
cmx new /Users/wumengsong/Code/ChatMuxX codex
```

会话创建后，继续在同一个微信会话里发送普通消息。这些消息会被转发给 Codex。

常用命令：

```text
cmx help
cmx sessions
cmx switch <session-id>
cmx screenshot
cmx interrupt
cmx enter
cmx esc
cmx close
```

Provider 原生的 slash commands 会被转发到当前 CLI，因此 `/...` 不会被用作 ChatMuxX 命令。

## 本地 CLI 会话测试

你可以不经过微信，直接测试 tmux/provider 路径：

```bash
target/debug/cmx sessions new /tmp codex
target/debug/cmx sessions list
target/debug/cmx sessions send <session-id> "hello" --enter
target/debug/cmx sessions capture <session-id>
target/debug/cmx sessions close <session-id>
```

也支持 shell：

```bash
target/debug/cmx sessions new /tmp shell
```

当前 `cmx` 暴露的 CLI 子命令：

```text
cmx daemon [--config <path>]
cmx login wechat
cmx doctor
cmx config init [--path <path>]
cmx sessions new <workspace> [provider] [-- <extra-provider-args>...]
cmx sessions list
cmx sessions send <session-id> <text> [--enter]
cmx sessions capture <session-id>
cmx sessions close <session-id>
```

## 配置

默认配置路径：

```text
~/.chatmuxx/config.toml
```

重要默认值：

```toml
[daemon]
tmux_session = "chatmuxx"
poll_interval_ms = 1500

[owner]
wechat_user_id = ""

[wechat]
enabled = true
base_url = "https://ilinkai.weixin.qq.com"
bot_type = "3"
long_poll_timeout_ms = 38000

[providers.codex]
command = "codex"
args = []
env = {}

[providers.claude]
command = "claude"
args = []
env = {}

[providers.shell]
command = "bash"
args = []
env = {}
```

Provider CLI 必须在明确的工作目录中启动。从微信使用 `cmx new /absolute/path codex`，或在本地使用 `cmx sessions new /absolute/path codex`。

## 状态文件

ChatMuxX 会把本地状态存储在：

```text
~/.chatmuxx
```

常见文件：

```text
config.toml
accounts.json
state.json
monitor_state.json
history.jsonl
```

`accounts.json` 包含敏感的微信凭据，不应分享给他人。

## 安全说明

- 将 `~/.chatmuxx/accounts.json` 视为密钥文件。
- 只在你接受移动端聊天账号控制本机 tmux、Codex、Claude 和 shell 的机器上运行 ChatMuxX。
- 使用绝对工作区路径，确保 provider 在预期项目目录中启动。
- 分享日志前先检查内容；transcript 和 pane capture 可能包含代码、路径、提示词或密钥。
- 当前 v0.1 流程面向单 owner 使用，还没有实现完整的多用户策略控制。

## 故障排查

如果 `cmx daemon` 提示没有微信账号，请运行：

```bash
target/debug/cmx login wechat
```

如果微信命令被忽略，请确认消息来自扫描登录二维码的同一个微信用户。

如果 Codex 无法启动，请检查：

```bash
codex --version
```

如果 tmux 窗口没有创建，请检查：

```bash
tmux -V
```

运行完整本地检查：

```bash
target/debug/cmx doctor
```

## 仓库结构

```text
crates/cmx/              # 面向用户的 cmx CLI
crates/chatmuxx-core/    # config、daemon、sessions、tmux、providers、channels、state
docs/design/             # 产品和架构设计说明
docs/development/        # 实现文档、任务索引、测试策略
```

## 开发文档

- [设计文档](docs/design/README.md)
- [开发文档](docs/development/README.md)
- [仓库架构](docs/development/repository-architecture.md)
- [状态与存储](docs/development/state-and-storage.md)
- [测试策略](docs/development/testing-strategy.md)
- [参考项目](docs/development/reference-projects.md)
