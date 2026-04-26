use std::path::PathBuf;

use crate::{
    config::{default_config_path, Config},
    error::{ChatMuxXError, IoContext, Result},
    state::files::ensure_state_dir,
};

pub async fn run(path: Option<PathBuf>) -> Result<()> {
    let path = path.unwrap_or(default_config_path()?);
    if path.exists() {
        return Err(ChatMuxXError::ConfigAlreadyExists(path));
    }

    if let Some(parent) = path.parent() {
        ensure_state_dir(parent).await?;
    }

    let config = Config::default();
    let text = toml::to_string_pretty(&config)?;
    tokio::fs::write(&path, text).await.at(&path)?;

    println!("Created {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn init_writes_default_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");

        run(Some(path.clone())).await.expect("init config");

        let written = tokio::fs::read_to_string(&path)
            .await
            .expect("read written config");
        assert!(written.contains("tmux_session = \"chatmuxx\""));
        assert!(written.contains("command = \"codex\""));
    }

    #[tokio::test]
    async fn init_refuses_to_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        tokio::fs::write(&path, "existing")
            .await
            .expect("seed config");

        let err = run(Some(path.clone())).await.expect_err("should fail");

        assert!(matches!(err, ChatMuxXError::ConfigAlreadyExists(existing) if existing == path));
    }
}
