use crate::config::Config;
use crate::config::current_minutes;
use crate::effects::EffectContext;
use crate::effects::profile::{build_profile, render};
use crate::effects::schedule::{ProfileSelection, resolve_profile};
use crate::metrics::Metrics;
use crate::openrgb::OpenRgbClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct Daemon {
    config: Config,
    metrics: Metrics,
    shutdown: Arc<AtomicBool>,
}

impl Daemon {
    pub fn new(config: Config) -> Self {
        Self {
            metrics: Metrics::new(config.temp_smoothing),
            config,
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

        info!("daemon started");

        let profiles: HashMap<String, crate::effects::profile::Profile> = self
            .config
            .effects
            .profiles
            .iter()
            .filter_map(|p| {
                build_profile(
                    &p.name,
                    &self.config.effects,
                    &self.config.temperature,
                    &self.config.colors,
                )
                .map(|built| (p.name.clone(), built))
            })
            .collect();

        let active_profile = build_profile(
            &self.config.effects.active_profile,
            &self.config.effects,
            &self.config.temperature,
            &self.config.colors,
        );

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

        let mut current_color: Option<[u8; 3]> = None;

        while !self.shutdown.load(Ordering::Relaxed) {
            let snapshot = self.metrics.snapshot();
            let ctx = EffectContext::with_telemetry(snapshot.cpu_temp, snapshot.cpu_usage);

            let selection = resolve_profile(&self.config.effects, current_minutes());
            let target = match selection {
                ProfileSelection::Off => [0u8; 3],
                ProfileSelection::Scheduled(name) => profiles
                    .get(name)
                    .map(|p| render(p, &ctx))
                    .unwrap_or_else(|| {
                        warn!("scheduled profile '{}' not found; falling back", name);
                        self.config
                            .color_for_temperature(snapshot.cpu_temp.unwrap_or(35.0))
                    }),
                ProfileSelection::ActiveProfile => active_profile
                    .as_ref()
                    .map(|p| render(p, &ctx))
                    .unwrap_or_else(|| {
                        self.config
                            .color_for_temperature(snapshot.cpu_temp.unwrap_or(35.0))
                    }),
            };

            let start = current_color.unwrap_or(target);
            let steps = self.config.transition_steps.max(1);

            for step in 1..=steps {
                if self.shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let ratio = step as f32 / steps as f32;
                let color = Config::interpolate_color(start, target, ratio);
                current_color = Some(color);

                info!(
                    "cpu_temp={:.1}°C cpu_usage={:.1}% color=rgb({},{},{})",
                    snapshot.cpu_temp.unwrap_or(0.0),
                    snapshot.cpu_usage,
                    color[0],
                    color[1],
                    color[2]
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

                if step < steps {
                    sleep(Duration::from_millis(self.config.transition_interval_ms)).await;
                }
            }

            sleep(Duration::from_secs(self.config.poll_interval)).await;
        }

        info!("daemon shutting down");
    }
}
