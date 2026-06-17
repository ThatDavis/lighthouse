use crate::config::Config;
use crate::metrics::Metrics;
use crate::openrgb::OpenRgbClient;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct Daemon {
    config: Config,
    metrics: Metrics,
    shutdown: Arc<AtomicBool>,
}

impl Daemon {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            metrics: Metrics::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    pub async fn run(mut self) {
        let client = OpenRgbClient::new(
            &self.config.openrgb_host,
            self.config.openrgb_port,
            self.config.dry_run,
        );

        let mut connection = match client.connect().await {
            Ok(conn) => conn,
            Err(e) => {
                if self.config.dry_run {
                    warn!("dry-run: continuing without OpenRGB ({e})");
                    crate::openrgb::Connection::DryRun
                } else {
                    error!("failed to connect to OpenRGB: {e}; exiting");
                    return;
                }
            }
        };

        let mut ticker = interval(Duration::from_secs(self.config.poll_interval));
        info!("daemon started");

        while !self.shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;

            let snapshot = self.metrics.snapshot();
            if let Some(temp) = snapshot.cpu_temp {
                let color = self.config.color_for_temperature(temp);
                info!(
                    "cpu_temp={:.1}°C cpu_usage={:.1}% color=rgb({},{},{})",
                    temp, snapshot.cpu_usage, color[0], color[1], color[2]
                );

                if let Err(e) = connection
                    .set_all_color(self.config.openrgb_device_id, 1, color)
                    .await
                {
                    warn!("failed to set color: {e}");
                }
            } else {
                warn!("no CPU temperature available");
            }
        }

        info!("daemon shutting down");
    }
}
