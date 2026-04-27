use std::path::PathBuf;

use crate::provider::ProviderKind;

#[derive(Debug, thiserror::Error)]
pub enum ChatMuxXError {
    #[error("home directory could not be determined")]
    HomeDirUnavailable,

    #[error("config already exists: {0}")]
    ConfigAlreadyExists(PathBuf),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize TOML config: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("failed to serialize JSON at {path}: {source}")]
    JsonSerialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse JSON at {path}: {source}")]
    JsonDeserialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse TOML config at {path}: {source}")]
    TomlDeserialize {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("external command failed: {command} ({code_text})", code_text = code.map_or_else(|| "terminated by signal".to_owned(), |code| format!("exit code {code}")))]
    ExternalCommandFailed { command: String, code: Option<i32> },

    #[error("tmux command failed: {command} ({stderr})")]
    TmuxCommandFailed { command: String, stderr: String },

    #[error("failed to parse tmux output: {0}")]
    TmuxParse(String),

    #[error("invalid workspace: {0}")]
    InvalidWorkspace(PathBuf),

    #[error("unknown provider: {0}")]
    UnknownProvider(ProviderKind),

    #[error("invalid provider: {0}")]
    InvalidProvider(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("ambiguous session id prefix `{prefix}` matches: {matches}")]
    AmbiguousSessionId { prefix: String, matches: String },

    #[error("invalid session id `{0}`; use 1-32 letters, numbers, '.', '_' or '-'")]
    InvalidSessionId(String),

    #[error("session id already exists: {0}")]
    SessionIdAlreadyExists(String),

    #[error(
        "ChatMuxX source directory is missing or is not a git repo: {0}; run the installer first"
    )]
    UpdateSourceMissing(PathBuf),

    #[error("session has no tmux attachment: {0}")]
    SessionNotAttached(String),

    #[error("wechat account is not logged in; run `cmx login wechat`")]
    WeChatAccountMissing,

    #[error("wechat account expired; run `cmx login wechat` again")]
    WeChatAccountExpired,

    #[error("wechat context token is missing for conversation: {0}")]
    WeChatMissingContextToken(String),

    #[error("wechat HTTP request failed with status {status}: {body}")]
    WeChatHttpStatus { status: u16, body: String },

    #[error("wechat protocol error: {0}")]
    WeChatProtocol(String),

    #[error("wechat request failed: {0}")]
    WeChatRequest(#[from] reqwest::Error),

    #[error("unsupported launch mode for {provider}: {mode}")]
    UnsupportedLaunchMode {
        provider: ProviderKind,
        mode: &'static str,
    },

    #[error("feature is not implemented yet: {0}")]
    NotImplemented(&'static str),
}

pub type Result<T> = std::result::Result<T, ChatMuxXError>;

impl ChatMuxXError {
    pub fn is_missing_tmux_target(&self) -> bool {
        let Self::TmuxCommandFailed { stderr, .. } = self else {
            return false;
        };

        stderr.contains("no server running")
            || stderr.contains("can't find")
            || stderr.contains("can't find pane")
            || stderr.contains("can't find window")
            || stderr.contains("can't find session")
    }
}

pub(crate) trait IoContext<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T> {
        let path = path.into();
        self.map_err(|source| ChatMuxXError::Io { path, source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tmux_server_is_detected() {
        let err = ChatMuxXError::TmuxCommandFailed {
            command: "tmux capture-pane -p -t %1".to_owned(),
            stderr: "no server running on /private/tmp/tmux-501/default".to_owned(),
        };

        assert!(err.is_missing_tmux_target());
    }
}
