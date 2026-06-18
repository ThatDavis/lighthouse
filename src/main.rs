pub mod config;
pub mod daemon;
pub mod effects;
pub mod metrics;
pub mod mqtt;
pub mod openrgb;
pub mod test_mode;
pub mod tui;

use crate::config::Config;
use crate::daemon::Daemon;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

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
    /// Run a color cycle test
    Test {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

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
                    println!("config {:?} is valid", config_path);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("config {:?} is invalid: {}", config_path, e);
                    Err(e.into())
                }
            }
        }
        Commands::Test { config } => {
            let config_path = config.unwrap_or_else(Config::default_path);
            crate::test_mode::run(config_path)
                .await
                .context("test mode failed")
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
