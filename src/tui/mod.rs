pub mod app;
pub mod daemon;
pub mod events;
pub mod theme;
pub mod ui;

use crate::config::Config;
use crate::tui::app::App;
use std::path::PathBuf;

pub async fn run(config_path: PathBuf) -> anyhow::Result<()> {
    let config = Config::from_file(&config_path)?;
    let app = App::new(config, config_path);
    crate::tui::events::run(app).await
}
