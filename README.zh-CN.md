# ChatMuxX

[English](README.md) | 简体中文

ChatMuxX 可以让你通过移动端聊天应用控制本机 tmux 窗口中的 Codex、Claude Code 或 shell。当前 v0.1 路线重点支持通过腾讯 iLink HTTP 接入微信，并在本机 tmux 中运行 Codex。

ChatMuxX 是一个全新的 Rust 实现。它运行时不依赖 OpenClaw，也不会安装或修改 Codex/Claude 的 hooks、插件、skills 或配置文件。

## 当前状态

项目目前已经支持一条最小可用的「微信到 Codex」流程：

- 使用 `cmx login wechat` 通过二维码登录微信 iLink。
- 使用 `cmx daemon` 启动前台守护进程。
- 从微信创建 Codex tmux 窗口。
- 将普通微信文本转发到当前活跃的 Codex 会话。
- 捕获 tmux pane 输出并回传到微信。
- 使用 `cmx ...` 命令列出、切换、截图、中断和关闭会话。

仍处于早期阶段的部分：

- Codex 输出来自 tmux pane capture，还不是结构化 transcript 解析。
- `cmx screenshot` 目前发送的是文本捕获结果，不是图片。
- `cmx close` 还没有确认提示。
- daemon 目前以前台进程运行，尚未安装 launchd/systemd 服务。
- Claude provider、更完整的恢复能力、假的 iLink 集成测试和多用户策略仍在推进中。

## 环境要求

- 安装了 `tmux` 的 macOS 或 Linux。
- Rust 工具链和 `cargo`。
- 已安装并在本机完成登录的 Codex CLI。
- 一个可以使用 iLink Bot API 的微信账号。

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

[wechat]
enabled = true
base_url = "https://ilinkai.weixin.qq.com"
bot_type = "3"
long_poll_timeout_ms = 38000

[providers.codex]
command = "codex"
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

运行健康检查：

```bash
target/debug/cmx doctor
```

## 开发文档

- [设计文档](docs/design/README.md)
- [开发文档](docs/development/README.md)
- [参考项目](docs/development/reference-projects.md)
