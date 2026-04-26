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

pub async fn new(workspace: PathBuf, provider: String, extra_args: Vec<String>) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    ensure_state_dir(&paths.root).await?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let provider =
        ProviderKind::from_str(&provider).map_err(|_| ChatMuxXError::InvalidProvider(provider))?;
    let record = manager
        .create_session(CreateSessionRequest {
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

    for session in sessions {
        println!(
            "{}\t{}\t{:?}\t{}",
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

    println!("closed session {}", record.id.0);
    Ok(())
}

pub async fn send(session_id: String, text: String, enter: bool) -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    let config = load_config_or_default().await?;
    let manager = SessionManager::new(config, paths);
    let session_id = SessionId(session_id);

    manager.send_text(&session_id, &text).await?;
    if enter {
        manager.send_key(&session_id, TmuxKey::Enter).await?;
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
