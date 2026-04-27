use std::path::PathBuf;
use std::str::FromStr;

use crate::{
    config::{default_config_path, load_config, Config},
    provider::{LaunchMode, ProviderKind},
    session::{CloseReason, CreateSessionRequest, SessionManager},
    state::{
        files::{ensure_state_dir, StatePaths},
        sessions::{OwnerId, SessionId},
    },
    tmux::TmuxKey,
    ChatMuxXError, Result,
};

pub async fn new(
    id: Option<String>,
    workspace: PathBuf,
    provider: String,
    extra_args: Vec<String>,
) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    ensure_state_dir(&paths.root).await?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let provider =
        ProviderKind::from_str(&provider).map_err(|_| ChatMuxXError::InvalidProvider(provider))?;
    let record = manager
        .create_session(CreateSessionRequest {
            id: id.map(SessionId),
            provider,
            workspace,
            launch_mode: LaunchMode::Fresh,
            extra_args,
            owner: OwnerId("local-owner".to_owned()),
            conversation: None,
        })
        .await?;

    println!(
        "created session {} ({})",
        record.id.0,
        record
            .tmux
            .as_ref()
            .map(|tmux| tmux.window_id.as_str())
            .unwrap_or("no tmux window")
    );
    Ok(())
}

pub async fn list() -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let sessions = manager.list_sessions().await?;

    if sessions.is_empty() {
        println!("No sessions.");
        return Ok(());
    }

    let mark_single_session_current =
        sessions.len() == 1 && sessions.iter().all(|session| !session.active);
    for session in sessions {
        let marker = if session.active || mark_single_session_current {
            "current"
        } else {
            "-"
        };
        println!(
            "{}\t{}\t{}\t{:?}\t{}",
            marker,
            session.id.0,
            session.provider,
            session.status,
            session.workspace.display()
        );
    }
    Ok(())
}

pub async fn close(session_id: String) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let record = manager
        .close_session(&SessionId(session_id.clone()), CloseReason::UserRequested)
        .await?;

    println!("closed and pruned session {}", record.id.0);
    Ok(())
}

pub async fn rename(session_id: String, new_id: String) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let record = manager
        .rename_session(&SessionId(session_id), SessionId(new_id))
        .await?;

    println!("renamed session to {}", record.id.0);
    Ok(())
}

pub async fn prune() -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let result = manager.prune_inactive_sessions().await?;

    println!(
        "pruned {} sessions and {} bindings",
        result.removed_sessions, result.removed_bindings
    );
    Ok(())
}

pub async fn send(session_id: String, text: String, enter: bool) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let session_id = SessionId(session_id);

    if enter {
        manager.send_text_and_enter(&session_id, &text).await?;
    } else {
        manager.send_text(&session_id, &text).await?;
    }

    println!("sent text to {}", session_id.0);
    Ok(())
}

pub async fn capture(session_id: String) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let text = manager.capture_pane(&SessionId(session_id)).await?;

    println!("{text}");
    Ok(())
}

pub async fn esc(session_id: String) -> Result<()> {
    send_key(session_id, TmuxKey::Escape, "Esc").await
}

pub async fn interrupt(session_id: String) -> Result<()> {
    send_key(session_id, TmuxKey::CtrlC, "Ctrl-C").await
}

pub async fn enter(session_id: String) -> Result<()> {
    send_key(session_id, TmuxKey::Enter, "Enter").await
}

async fn send_key(session_id: String, key: TmuxKey, label: &str) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let session_id = SessionId(session_id);
    manager.send_key(&session_id, key).await?;

    println!("sent {label} to {}", session_id.0);
    Ok(())
}

async fn load_config_or_default() -> Result<Config> {
    let path = default_config_path()?;
    match load_config(&path).await {
        Ok(config) => Ok(config),
        Err(ChatMuxXError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Config::default())
        }
        Err(err) => Err(err),
    }
}
