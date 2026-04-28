use crate::{provider::ProviderKind, session::SessionSummary, state::sessions::SessionId};

pub(super) const WECHAT_DISABLED: &str = "WeChat is disabled in config.";
pub(super) const WECHAT_ACCOUNT_MISSING: &str =
    "No WeChat account found. Run `cmx login wechat` first.";
pub(super) const DAEMON_STARTING: &str = "Starting ChatMuxX daemon. Press Ctrl-C to stop.";
pub(super) const DAEMON_STOPPING: &str = "Stopping ChatMuxX daemon.";
pub(super) const UNAUTHORIZED: &str = "未授权用户不能使用 ChatMuxX。";
pub(super) const NO_BOUND_SESSION: &str = "还没有绑定的会话。发送 `cmx n /你的项目路径 claude` 创建 Claude 会话，或把 `claude` 换成 `codex` / `shell`。";
pub(super) const USAGE_NEW: &str =
    "用法：`cmx n --id main /项目路径 claude`，provider 也可以是 `codex` 或 `shell`。";
pub(super) const NEW_ID_POSITION: &str =
    "项目路径解析失败：`--id` 要放在 `cmx n` 后面，示例：`cmx n --id main /项目路径 claude`。";
pub(super) const USAGE_SWITCH: &str = "用法：`cmx sw <session-id>`";
pub(super) const USAGE_RENAME: &str = "用法：`cmx mv [session-id] <new-id>`";
pub(super) const NO_ACTIVE_SESSION: &str = "没有活动会话。";
pub(super) const NO_ACTIVE_TO_CLOSE: &str = "没有可关闭的活动会话。";
pub(super) const NO_ACTIVE_TO_RENAME: &str = "没有可重命名的活动会话。";
pub(super) const PROVIDER_SWITCH_NOT_SUPPORTED: &str = "切换 provider 请先用 `cmx rm` 关闭当前会话，再用 `cmx n /路径 claude` 或 `cmx n /路径 codex` 创建。";
pub(super) const RECOVER_NOT_SUPPORTED: &str = "恢复会话稍后支持。";
pub(super) const CONFIRM_EXPIRED_OR_MISSING: &str = "没有待确认操作，或确认已过期。";
pub(super) const CONFIRM_CANCELLED: &str = "已取消。";
pub(super) const SESSION_LIST_EMPTY: &str = "会话列表：没有会话。";
pub(super) const SESSION_MARKER_CURRENT: &str = "当前 ";
pub(super) const SESSION_MARKER_ACTIVE: &str = "活动 ";
pub(super) const HELP: &str = "ChatMuxX 命令：\ncmx n --id main /项目路径 claude\ncmx n --id codex /项目路径 codex\ncmx n --id sh /项目路径 shell\ncmx ls\ncmx sw <session-id>\ncmx mv [session-id] <new-id>\ncmx ss\ncmx rm\ncmx prune\n普通文字会发送给当前会话。";

pub(super) fn flow_not_active(text: &str) -> String {
    format!("暂未进入交互流程，收到：{text}")
}

pub(super) fn session_created(provider: ProviderKind, session_id: &SessionId) -> String {
    format!("已创建 {provider} 会话：{}", session_id.0)
}

pub(super) fn session_switched(session_id: &SessionId) -> String {
    format!("已切换到：{}", session_id.0)
}

pub(super) fn session_closed(session_id: &SessionId) -> String {
    format!("已关闭并清理：{}", session_id.0)
}

pub(super) fn confirm_close(session_id: &SessionId) -> String {
    format!("确认关闭会话 `{}`？回复 `yes` 确认，回复 `no` 取消。", session_id.0)
}

pub(super) fn confirm_interrupt(session_id: &SessionId) -> String {
    format!("确认中断会话 `{}`？回复 `yes` 确认，回复 `no` 取消。", session_id.0)
}

pub(super) fn session_interrupted(session_id: &SessionId) -> String {
    format!("已中断：{}", session_id.0)
}

pub(super) fn session_renamed(session_id: &SessionId) -> String {
    format!("已重命名会话：{}", session_id.0)
}

pub(super) fn sessions_pruned(removed_sessions: usize, removed_bindings: usize) -> String {
    format!("已清理 {removed_sessions} 个 Dead/Closed 会话，{removed_bindings} 条绑定记录。")
}

pub(super) fn unknown_command(name: &str) -> String {
    format!("未知 cmx 命令：{name}")
}

pub(super) fn session_list_header(count: usize) -> String {
    format!("会话列表：{count} 个")
}

pub(super) fn session_list_item(session: &SessionSummary, marker: &str) -> String {
    format!(
        "- {marker}{} · {} · {:?}\n  {}",
        session.id.0,
        session.provider,
        session.status,
        session.workspace.display()
    )
}
