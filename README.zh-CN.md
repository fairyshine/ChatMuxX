# ChatMuxX

[English](README.md) | 简体中文

ChatMuxX 让你在微信里控制本机的 Codex、Claude Code 或 shell。

最常见的用法是：电脑上启动 `cmx daemon`，手机微信里发送一句话，ChatMuxX 把这句话转发给本机 tmux 里的 Codex，再把 Codex 的回复发回微信。

ChatMuxX 不会安装或修改 Codex/Claude 的 hooks、插件、skills 或配置文件。

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/master/scripts/install.sh | sh
```

如果新终端里找不到 `cmx`，把下面任意一行加入 `~/.zshrc`：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
alias cmx="$HOME/.cargo/bin/cmx"
```

然后重新加载 zsh：

```bash
source ~/.zshrc
```

## 先试一下

你需要提前准备好 `tmux`、已登录的 Codex CLI，以及可以使用 iLink Bot API 的微信账号。

1. 检查本机环境：

```bash
cmx doctor
```

2. 连接微信：

```bash
cmx login wechat
```

终端里会显示二维码，用微信扫码确认。

3. 启动 daemon：

```bash
cmx daemon
```

这个终端窗口要保持运行。daemon 会负责接收微信消息、控制 tmux，并把回复发回微信。

4. 从微信发送：

```text
cmx n --id main /Users/you/Code/your-project codex
```

把路径换成你真实的项目目录。

5. 继续在微信里发普通消息：

```text
帮我看一下这个仓库怎么启动
```

Codex 会在你的电脑上运行，回复会发回微信。

如果要从微信中断当前任务，发送 `cmx i`，再回复 `yes` 确认。如果要关闭当前会话，发送 `cmx rm`，再回复 `yes` 确认。

## 需要提前准备

检查本地工具：

```bash
tmux -V
codex --version
```

普通微信文字会发送给 Codex。以 `cmx` 开头的是 ChatMuxX 控制命令。

Provider 自己的 `/...` 命令会直接发给 Codex 或 Claude，不会被 ChatMuxX 拦截。

## 常用命令

微信命令不写 session id 时，会默认操作当前绑定会话。本地 CLI 的终端控制命令通常需要写 session id。

微信里的危险操作会先二次确认，例如 `cmx i` 和 `cmx rm`。回复 `yes` 继续，回复 `no` 取消。

| 用途 | 缩写 | 全写 |
| --- | --- | --- |
| 帮助 | `cmx h` | `cmx help` |
| 新建 Codex 会话 | `cmx n --id main /路径 codex` | `cmx new --id main /路径 codex` |
| 新建 Shell 会话 | `cmx n --id sh /路径 shell` | `cmx new --id sh /路径 shell` |
| 新建 Claude 会话 | `cmx n --id claude /路径 claude` | `cmx new --id claude /路径 claude` |
| 查看会话 | `cmx ls` | `cmx list` / `cmx sessions list` |
| 切换会话 | `cmx sw main` | `cmx switch main` |
| 重命名会话 | 微信：`cmx mv main2`<br>本地：`cmx mv main main2` | 微信：`cmx rename main2`<br>本地：`cmx rename main main2` |
| 查看终端文本 | 微信：`cmx ss`<br>本地：`cmx ss main` | 微信：`cmx screenshot`<br>本地：`cmx capture main` |
| 发送文本 | 微信：普通消息<br>本地：`cmx p main "echo hello" --enter` | 微信：普通消息<br>本地：`cmx send main "echo hello" --enter` |
| 中断任务 | 微信：`cmx i`<br>本地：`cmx i main` | 微信：`cmx interrupt`<br>本地：`cmx interrupt main` |
| 发送 Enter | 微信：`cmx e`<br>本地：`cmx e main` | 微信：`cmx enter`<br>本地：`cmx enter main` |
| 发送 Esc | 微信：`cmx esc`<br>本地：`cmx esc main` | `cmx esc` |
| 关闭会话 | 微信：`cmx rm`<br>本地：`cmx rm main` | 微信：`cmx close`<br>本地：`cmx close main` |
| 清理 closed/dead 会话 | `cmx clean` | `cmx prune` |

## 更新

运行：

```bash
cmx update
```

这会下载最新 GitHub Release，包括 pre-release。

如果只想安装最新正式版，可以设置 `CHATMUXX_VERSION=latest-stable`。

## 状态文件

ChatMuxX 的本地状态默认放在：

```text
~/.chatmuxx
```

常见文件：

- `config.toml`：配置
- `accounts.json`：微信登录信息
- `state.json`：会话和绑定
- `history.jsonl`：消息记录
- `monitor_state.json`：tmux 输出捕获状态

## 仍在开发中

当前已经能跑通微信控制本机 Codex 的基本流程，但仍是早期版本：

- daemon 目前以前台进程运行，还没有安装成系统服务。
- `cmx ss` 现在返回文本，不是真截图。
- Claude provider、恢复能力、多用户策略还会继续完善。

开发和设计文档在 [docs](docs/) 目录。
