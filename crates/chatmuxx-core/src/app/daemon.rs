use std::path::PathBuf;

use crate::{config::default_config_path, Result};

pub async fn run(config_path: Option<PathBuf>) -> Result<()> {
    let path = config_path.unwrap_or(default_config_path()?);
    tracing::info!(config = %path.display(), "starting ChatMuxX daemon");
    println!(
        "cmx daemon is not implemented yet. Expected config: {}",
        path.display()
    );
    Ok(())
}
