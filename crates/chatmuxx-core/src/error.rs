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

pub(crate) trait IoContext<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T> {
        let path = path.into();
        self.map_err(|source| ChatMuxXError::Io { path, source })
    }
}
