use crate::{ChatMuxXError, Result};

pub async fn list() -> Result<()> {
    println!("No session store is implemented yet.");
    Ok(())
}

pub async fn close(_session_id: String) -> Result<()> {
    Err(ChatMuxXError::NotImplemented("cmx sessions close"))
}
