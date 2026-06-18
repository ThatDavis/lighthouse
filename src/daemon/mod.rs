use crate::config::Config;
use crate::metrics::Metrics;
use crate::openrgb::OpenRgbClient;
use std::collections::HashMap;
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
                    match OpenRgbClient::new(
                        &self.config.openrgb_host,
                        self.config.openrgb_port,
                        true,
                    )
                    .connect()
                    .await
                    {
                        Ok(conn) => conn,
                        Err(_) => {
                            error!("dry-run: failed to create dry-run connection; exiting");
                            return;
                        }
                    }
                } else {
                    error!("failed to connect to OpenRGB: {e}; exiting");
                    return;
                }
            }
        };

        let mut ticker = interval(Duration::from_secs(self.config.poll_interval));
        info!("daemon started");

        let mut zone_led_counts: HashMap<u32, u32> = HashMap::new();
        let controller = match connection
            .query_controller_data(self.config.openrgb_device_id)
            .await
        {
            Ok(controller) => {
                for (idx, zone) in controller.zones.iter().enumerate() {
                    info!("zone {}: {} ({} leds)", idx, zone.name, zone.led_count);
                    zone_led_counts.insert(idx as u32, zone.led_count);
                }
                Some(controller)
            }
            Err(e) => {
                warn!("failed to query controller data: {e}; using fallback led count of 1");
                for zone_id in &self.config.openrgb_zone_ids {
                    zone_led_counts.insert(*zone_id, 1);
                }
                None
            }
        };

        if let Some(ref controller) = controller {
            if let Err(e) = connection
                .set_direct_mode(self.config.openrgb_device_id, controller)
                .await
            {
                warn!("failed to set device to Direct mode: {e}");
            }
        } else {
            warn!("cannot set Direct mode without controller data");
        }

        while !self.shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;

            let snapshot = self.metrics.snapshot();
            if let Some(temp) = snapshot.cpu_temp {
                let color = self.config.color_for_temperature(temp);
                info!(
                    "cpu_temp={:.1}°C cpu_usage={:.1}% color=rgb({},{},{})",
                    temp, snapshot.cpu_usage, color[0], color[1], color[2]
                );

                for zone_id in &self.config.openrgb_zone_ids {
                    let led_count = zone_led_counts.get(zone_id).copied().unwrap_or(1);
                    if let Err(e) = connection
                        .set_zone_color(self.config.openrgb_device_id, *zone_id, led_count, color)
                        .await
                    {
                        warn!("failed to set color for zone {}: {}", zone_id, e);
                    }
                }
            } else {
                warn!("no CPU temperature available");
            }
        }

        info!("daemon shutting down");
    }
}
