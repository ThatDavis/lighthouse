use crate::config::Config;
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

pub async fn run(config_path: PathBuf) -> Result<()> {
    info!("TUI not yet implemented; config path: {:?}", config_path);
    println!("Lighthouse TUI placeholder. Config: {:?}", config_path);
    Ok(())
}
