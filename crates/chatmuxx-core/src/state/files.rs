use std::path::{Path, PathBuf};

use crate::{
    config::default_state_dir,
    error::{IoContext, Result},
};

#[derive(Clone, Debug)]
pub struct StatePaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub accounts: PathBuf,
    pub monitor_state: PathBuf,
    pub history: PathBuf,
    pub logs_dir: PathBuf,
}

impl StatePaths {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            config: root.join("config.toml"),
            state: root.join("state.json"),
            accounts: root.join("accounts.json"),
            monitor_state: root.join("monitor_state.json"),
            history: root.join("history.jsonl"),
            logs_dir: root.join("logs"),
            root,
        }
    }

    pub fn from_default_root() -> Result<Self> {
        Ok(Self::from_root(default_state_dir()?))
    }
}

pub async fn ensure_state_dir(path: &Path) -> Result<()> {
    tokio::fs::create_dir_all(path).await.at(path)?;
    set_private_dir_permissions(path).await
}

#[cfg(unix)]
async fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(0o700);
    tokio::fs::set_permissions(path, permissions).await.at(path)
}

#[cfg(not(unix))]
async fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn mode(path: &Path) -> Result<u32> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path).at(path)?;
    Ok(metadata.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
pub fn mode(_path: &Path) -> Result<u32> {
    Ok(0)
}

#[cfg(unix)]
pub fn is_private_dir(path: &Path) -> Result<bool> {
    Ok(mode(path)? == 0o700)
}

#[cfg(not(unix))]
pub fn is_private_dir(_path: &Path) -> Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_paths_are_rooted_under_directory() {
        let paths = StatePaths::from_root("/tmp/chatmuxx-test");

        assert_eq!(
            paths.config,
            PathBuf::from("/tmp/chatmuxx-test/config.toml")
        );
        assert_eq!(
            paths.accounts,
            PathBuf::from("/tmp/chatmuxx-test/accounts.json")
        );
        assert_eq!(paths.logs_dir, PathBuf::from("/tmp/chatmuxx-test/logs"));
    }

    #[tokio::test]
    async fn ensure_state_dir_creates_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state_dir = dir.path().join(".chatmuxx");

        ensure_state_dir(&state_dir)
            .await
            .expect("ensure state dir");

        assert!(state_dir.is_dir());
        assert!(is_private_dir(&state_dir).expect("permission check"));
    }
}
