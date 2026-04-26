use std::path::Path;

use crate::{
    config::{default_config_path, load_config},
    state::files::{ensure_state_dir, is_private_dir, StatePaths},
    Result,
};

pub async fn run() -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    ensure_state_dir(&paths.root).await?;

    let config_path = default_config_path()?;
    println!("ChatMuxX doctor");
    print_path_check("state dir", &paths.root, paths.root.is_dir());
    print_path_check("config", &config_path, config_path.exists());

    match is_private_dir(&paths.root) {
        Ok(true) => println!("- permissions: ok (state dir is private)"),
        Ok(false) => println!("- permissions: warning (state dir should be 0700)"),
        Err(err) => println!("- permissions: warning ({err})"),
    }

    match load_config(&config_path).await {
        Ok(config) => {
            println!("- config parse: ok");
            print_command_check("tmux", "tmux");
            print_command_check("codex", &config.providers.codex.command);
            print_command_check("claude", &config.providers.claude.command);
            print_command_check("shell", &config.providers.shell.command);
        }
        Err(err) if config_path.exists() => {
            println!("- config parse: error ({err})");
        }
        Err(_) => {
            println!("- config parse: skipped (run `cmx config init`)");
            print_command_check("tmux", "tmux");
        }
    }

    Ok(())
}

fn print_path_check(label: &str, path: &Path, exists: bool) {
    let status = if exists { "ok" } else { "missing" };
    println!("- {label}: {status} ({})", path.display());
}

fn print_command_check(label: &str, command: &str) {
    let status = if command_exists(command) {
        "ok"
    } else {
        "missing"
    };
    println!("- {label} command: {status} ({command})");
}

fn command_exists(command: &str) -> bool {
    if command.contains('/') {
        return Path::new(command).is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path_var).any(|dir| dir.join(command).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_exists_finds_absolute_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let command = dir.path().join("tool");
        std::fs::write(&command, "").expect("write command");

        assert!(command_exists(command.to_str().expect("utf8 path")));
    }

    #[test]
    fn command_exists_returns_false_for_missing_command() {
        assert!(!command_exists("definitely-not-a-chatmuxx-command"));
    }
}
