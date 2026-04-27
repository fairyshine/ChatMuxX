# ChatMuxX

[English](README.md) | 简体中文

ChatMuxX 让你在微信里控制本机的 Codex、Claude Code 或 shell。

最常见的用法是：电脑上启动 `cmux daemon`，手机微信里发送一句话，ChatMuxX 把这句话转发给本机 tmux 里的 Codex，再把 Codex 的回复发回微信。

ChatMuxX 不会安装或修改 Codex/Claude 的 hooks、插件、skills 或配置文件。

## 先看效果

微信里发送：

```text
cmux n --id main /Users/you/Code/your-project codex
```

然后继续在微信里发普通消息，例如：

```text
帮我看一下这个仓库怎么启动
```

Codex 会在你的电脑上运行，回复会发回微信。

## 安装

推荐一键安装：

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh | sh
```

安装脚本会下载源码并执行 `cargo install`。安装完成后会得到 `cmux` 命令。

如果想先看脚本内容再执行：

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh -o /tmp/chatmuxx-install.sh
less /tmp/chatmuxx-install.sh
sh /tmp/chatmuxx-install.sh
```

如果新终端里找不到 `cmux`，把下面这行加到你的 shell 配置里：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## 需要提前准备

电脑上需要有：

- `tmux`
- Rust / Cargo
- Codex CLI，并且已经在本机登录
- 可以使用 iLink Bot API 的微信账号

可以先检查：

```bash
tmux -V
cargo --version
codex --version
```

## 第一次使用

1. 检查本机环境：

```bash
cmux doctor
```

2. 登录微信：

```bash
cmux login wechat
```

终端里会显示二维码，用微信扫码确认。

3. 启动 ChatMuxX：

```bash
cmux daemon
```

这个窗口要保持运行。后续它负责收微信消息、控制 tmux、发送回复。

4. 从微信创建 Codex 会话：

```text
cmux n --id main /Users/you/Code/your-project codex
```

把路径换成你真实的项目目录。

5. 开始聊天：

```text
解释一下这个项目
```

普通文字会发送给 Codex。以 `cmux` 开头的是 ChatMuxX 控制命令。

## 常用微信命令

```text
cmux h                         帮助
cmux n <项目路径> codex        新建 Codex 会话
cmux ls                        查看会话
cmux sw <session-id>           切换会话
cmux mv <new-id>               重命名当前会话
cmux ss                        查看当前终端文本
cmux i                         中断当前任务，等同 Ctrl-C
cmux e                         发送 Enter
cmux esc                       发送 Esc
cmux rm                        关闭当前会话
```

Provider 自己的 `/...` 命令会直接发给 Codex 或 Claude，不会被 ChatMuxX 拦截。

## 本地命令

不经过微信也可以直接测试：

```bash
cmux n --id test /tmp shell
cmux ls
cmux p test "echo hello" --enter
cmux cap test
cmux rm test
```

本地控制命令通常需要写 session id，例如：

```bash
cmux i main
cmux e main
cmux esc main
```

## 更新

再次运行安装命令即可：

```bash
curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/main/scripts/install.sh | sh
```

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
- `cmux ss` 现在返回文本，不是真截图。
- Claude provider、恢复能力、多用户策略还会继续完善。

开发和设计文档在 [docs](docs/) 目录。
