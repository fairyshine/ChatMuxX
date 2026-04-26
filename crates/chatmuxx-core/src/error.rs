use std::path::PathBuf;

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

    #[error("failed to parse TOML config at {path}: {source}")]
    TomlDeserialize {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
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
