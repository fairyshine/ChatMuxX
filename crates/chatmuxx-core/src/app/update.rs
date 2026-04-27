use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::process::Command;

use crate::{config::default_state_dir, error::Result, ChatMuxXError};

const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/fairyshine/ChatMuxX/master/scripts/install.sh";

pub async fn run(
    source_dir: Option<PathBuf>,
    branch: Option<String>,
    version: Option<String>,
) -> Result<()> {
    if source_dir.is_none() {
        return update_from_release(version).await;
    }

    update_from_source(source_dir, branch).await
}

async fn update_from_release(version: Option<String>) -> Result<()> {
    let version = version.or_else(|| std::env::var("CHATMUXX_VERSION").ok());
    let mut command = if let Some(version) = version.as_deref() {
        format!(
            "curl -fsSL -A {} {} | CHATMUXX_VERSION={} sh",
            shell_escape("ChatMuxX updater"),
            shell_escape(INSTALL_SCRIPT_URL),
            shell_escape(version)
        )
    } else {
        format!(
            "curl -fsSL -A {} {} | sh",
            shell_escape("ChatMuxX updater"),
            shell_escape(INSTALL_SCRIPT_URL)
        )
    };

    if let Ok(method) = std::env::var("CHATMUXX_INSTALL_METHOD") {
        command = format!(
            "CHATMUXX_INSTALL_METHOD={} {}",
            shell_escape(&method),
            command
        );
    }
    command = format!("CHATMUXX_SKIP_POST_INSTALL_RUN=1 {command}");

    println!("Updating cmx from GitHub Releases...");
    println!("Running installer: {command}");
    run_inherited("sh", &["-c", &command], Path::new(".")).await?;
    println!("ChatMuxX updated. Restart `cmx daemon` if it is running.");
    Ok(())
}

async fn update_from_source(source_dir: Option<PathBuf>, branch: Option<String>) -> Result<()> {
    let source_dir = source_dir
        .or_else(|| std::env::var_os("CHATMUXX_SRC_DIR").map(PathBuf::from))
        .unwrap_or(default_state_dir()?.join("src").join("ChatMuxX"));
    let branch = branch
        .or_else(|| std::env::var("CHATMUXX_BRANCH").ok())
        .unwrap_or_else(|| "master".to_owned());

    if !source_dir.join(".git").is_dir() {
        return Err(ChatMuxXError::UpdateSourceMissing(source_dir));
    }

    println!("Updating ChatMuxX source in {}...", source_dir.display());
    run_inherited("git", &["fetch", "--prune", "origin"], &source_dir).await?;
    run_inherited("git", &["checkout", &branch], &source_dir).await?;
    run_inherited(
        "git",
        &["pull", "--ff-only", "origin", &branch],
        &source_dir,
    )
    .await?;

    println!("Installing updated cmx...");
    let cmx_crate = source_dir.join("crates").join("cmx").display().to_string();
    run_inherited(
        "cargo",
        &["install", "--path", &cmx_crate, "--force"],
        Path::new("."),
    )
    .await?;

    println!("ChatMuxX updated. Restart `cmx daemon` if it is running.");
    Ok(())
}

fn shell_escape(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

async fn run_inherited(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                ChatMuxXError::CommandNotFound(program.to_owned())
            } else {
                ChatMuxXError::Io {
                    path: PathBuf::from(program),
                    source: err,
                }
            }
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(ChatMuxXError::ExternalCommandFailed {
            command: format!("{} {}", program, args.join(" ")),
            code: status.code(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn update_reports_missing_source_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = run(
            Some(dir.path().join("missing")),
            Some("master".to_owned()),
            None,
        )
        .await
        .expect_err("missing source");

        assert!(matches!(err, ChatMuxXError::UpdateSourceMissing(_)));
    }

    #[test]
    fn shell_escape_quotes_unsafe_values() {
        assert_eq!(shell_escape("v0.0.1-dev1"), "v0.0.1-dev1");
        assert_eq!(shell_escape("bad value"), "'bad value'");
        assert_eq!(shell_escape("bad'value"), "'bad'\\''value'");
    }
}
