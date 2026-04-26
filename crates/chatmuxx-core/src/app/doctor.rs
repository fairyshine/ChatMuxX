use crate::{config::default_config_path, Result};

pub async fn run() -> Result<()> {
    let config_path = default_config_path()?;
    println!("ChatMuxX doctor");
    println!("- config: {}", config_path.display());
    println!("- detailed checks are not implemented yet");
    Ok(())
}
