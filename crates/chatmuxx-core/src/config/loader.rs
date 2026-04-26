use std::path::{Path, PathBuf};

use crate::{error::IoContext, ChatMuxXError, Result};

pub fn default_state_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or(ChatMuxXError::HomeDirUnavailable)?;
    Ok(PathBuf::from(home).join(".chatmuxx"))
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(default_state_dir()?.join("config.toml"))
}

pub async fn load_config(path: impl AsRef<Path>) -> Result<super::Config> {
    let path = path.as_ref();
    let text = tokio::fs::read_to_string(path).await.at(path)?;
    toml::from_str(&text).map_err(|source| ChatMuxXError::TomlDeserialize {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn load_config_reads_toml_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let text = toml::to_string_pretty(&super::super::Config::default()).expect("toml");
        tokio::fs::write(&path, text).await.expect("write config");

        let config = load_config(&path).await.expect("load config");

        assert_eq!(config.daemon.tmux_session, "chatmuxx");
        assert_eq!(config.providers.codex.command, "codex");
    }
}
