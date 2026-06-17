pub mod config;
pub mod daemon;
pub mod metrics;
pub mod openrgb;
pub mod tui;

use crate::config::Config;
use crate::daemon::Daemon;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "lighthouse")]
#[command(about = "Map system telemetry to OpenRGB motherboard lighting")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the lighting daemon
    Daemon {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    /// Launch the interactive TUI
    Tui {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    /// Validate a config file
    Validate {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { config } => {
            let config_path = config.unwrap_or_else(Config::default_path);
            let config = Config::from_file(&config_path)
                .with_context(|| format!("failed to load config from {:?}", config_path))?;

            let daemon = Daemon::new(config);
            let shutdown = daemon.shutdown_handle();

            setup_signal_handler(shutdown);
            daemon.run().await;
            info!("exited cleanly");
            Ok(())
        }
        Commands::Tui { config } => {
            let config_path = config.unwrap_or_else(Config::default_path);
            info!("starting TUI with config {:?}", config_path);
            crate::tui::run(config_path).await.context("TUI failed")
        }
        Commands::Validate { config } => {
            let config_path = config.unwrap_or_else(Config::default_path);
            match Config::from_file(&config_path) {
                Ok(_) => {
                    info!("config {:?} is valid", config_path);
                    Ok(())
                }
                Err(e) => {
                    error!("config {:?} is invalid: {}", config_path, e);
                    Err(e.into())
                }
            }
        }
    }
}

fn setup_signal_handler(shutdown: Arc<AtomicBool>) {
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        info!("received shutdown signal");
        shutdown.store(true, Ordering::Relaxed);
    });
}
