use crate::config::Config;
use crate::config::current_minutes;
use crate::effects::EffectContext;
use crate::effects::profile::{build_profile, render};
use crate::effects::schedule::{ProfileSelection, resolve_profile};
use crate::metrics::Metrics;
use crate::mqtt::{LightCommand, MqttClient};
use crate::openrgb::OpenRgbClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::{sleep, timeout};
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

        let mut mqtt: Option<MqttClient> = None;
        let mut mqtt_override: Option<LightCommand> = None;
        if self.config.mqtt.enabled {
            match MqttClient::connect(&self.config.mqtt).await {
                Ok(client) => {
                    info!("MQTT enabled");
                    mqtt = Some(client);
                }
                Err(e) => warn!("MQTT connection failed: {e}"),
            }
        }

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
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.poll_interval));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            if let Some(ref mut mqtt_client) = mqtt {
                match timeout(Duration::from_millis(10), mqtt_client.recv_command()).await {
                    Ok(Some(cmd)) => {
                        mqtt_override = Some(cmd);
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            }

            let snapshot = self.metrics.snapshot();
            let ctx = EffectContext::with_telemetry(snapshot.cpu_temp, snapshot.cpu_usage);

            let computed_target = match resolve_profile(&self.config.effects, current_minutes()) {
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

            let (target, light_on) = match mqtt_override {
                Some(cmd) if !cmd.on => ([0u8; 3], false),
                Some(cmd) => (cmd.color, true),
                None => (computed_target, true),
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

                if let Some(ref mqtt_client) = mqtt {
                    mqtt_client.publish_states(
                        snapshot.cpu_temp,
                        snapshot.cpu_usage,
                        color,
                        light_on,
                    );
                }

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

            interval.tick().await;
        }

        info!("daemon shutting down");
    }
}
