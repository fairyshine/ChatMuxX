use crate::{provider::ProviderKind, session::SessionSummary, state::sessions::SessionId};

pub(super) const WECHAT_DISABLED: &str = "WeChat is disabled in config.";
pub(super) const WECHAT_ACCOUNT_MISSING: &str =
    "No WeChat account found. Run `cmx login wechat` first.";
pub(super) const DAEMON_STARTING: &str = "Starting ChatMuxX daemon. Press Ctrl-C to stop.";
pub(super) const DAEMON_STOPPING: &str = "Stopping ChatMuxX daemon.";
pub(super) const UNAUTHORIZED: &str = "This WeChat user is not authorized to use ChatMuxX.";
pub(super) const NO_BOUND_SESSION: &str = "No session is bound to this chat yet. Send `cmx n /your/project/path claude` to create a Claude session, or use `codex` / `shell` instead.";
pub(super) const USAGE_NEW: &str =
    "Usage: `cmx n --id main /your/project/path claude`; provider can also be `codex` or `shell`.";
pub(super) const NEW_ID_POSITION: &str =
    "Invalid workspace: put `--id` after `cmx n`, for example `cmx n --id main /your/project/path claude`.";
pub(super) const USAGE_SWITCH: &str = "Usage: `cmx sw <session-id>`";
pub(super) const USAGE_RENAME: &str = "Usage: `cmx mv [session-id] <new-id>`";
pub(super) const NO_ACTIVE_SESSION: &str = "No active session.";
pub(super) const NO_ACTIVE_TO_CLOSE: &str = "No active session to close.";
pub(super) const NO_ACTIVE_TO_RENAME: &str = "No active session to rename.";
pub(super) const PROVIDER_SWITCH_NOT_SUPPORTED: &str = "Provider switching is not supported in-place yet. Close the current session with `cmx rm`, then create a new one with `cmx n /path claude` or `cmx n /path codex`.";
pub(super) const RECOVER_NOT_SUPPORTED: &str = "Session recovery is not supported yet.";
pub(super) const CONFIRM_EXPIRED_OR_MISSING: &str =
    "There is no pending confirmation, or it has expired.";
pub(super) const CONFIRM_CANCELLED: &str = "Cancelled.";
pub(super) const SESSION_LIST_EMPTY: &str = "Sessions: none.";
pub(super) const SESSION_MARKER_CURRENT: &str = "current ";
pub(super) const SESSION_MARKER_ACTIVE: &str = "active ";
pub(super) const HELP: &str = "ChatMuxX commands:\ncmx n --id main /project/path claude\ncmx n --id codex /project/path codex\ncmx n --id sh /project/path shell\ncmx ls\ncmx sw <session-id>\ncmx mv [session-id] <new-id>\ncmx ss\ncmx rm\ncmx prune\nPlain text is sent to the current session.";

pub(super) fn flow_not_active(text: &str) -> String {
    format!("No interactive flow is active. Received: {text}")
}

pub(super) fn session_created(provider: ProviderKind, session_id: &SessionId) -> String {
    format!("Created {provider} session: {}", session_id.0)
}

pub(super) fn session_switched(session_id: &SessionId) -> String {
    format!("Switched to session: {}", session_id.0)
}

pub(super) fn session_closed(session_id: &SessionId) -> String {
    format!("Closed and cleaned up session: {}", session_id.0)
}

pub(super) fn confirm_close(session_id: &SessionId) -> String {
    format!(
        "Close session `{}`? Reply `yes` to confirm, or `no` to cancel.",
        session_id.0
    )
}

pub(super) fn confirm_interrupt(session_id: &SessionId) -> String {
    format!(
        "Interrupt session `{}`? Reply `yes` to confirm, or `no` to cancel.",
        session_id.0
    )
}

pub(super) fn session_interrupted(session_id: &SessionId) -> String {
    format!("Interrupted session: {}", session_id.0)
}

pub(super) fn session_renamed(session_id: &SessionId) -> String {
    format!("Renamed session: {}", session_id.0)
}

pub(super) fn sessions_pruned(removed_sessions: usize, removed_bindings: usize) -> String {
    format!("Pruned {removed_sessions} Dead/Closed sessions and {removed_bindings} bindings.")
}

pub(super) fn unknown_command(name: &str) -> String {
    format!("Unknown cmx command: {name}")
}

pub(super) fn session_list_header(count: usize) -> String {
    format!("Sessions: {count}")
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
