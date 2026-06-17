use crate::config::Config;
use crate::openrgb::OpenRgbClient;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub async fn run(config_path: PathBuf) -> Result<()> {
    let config = Config::from_file(&config_path)
        .with_context(|| format!("failed to load config from {:?}", config_path))?;

    let client = OpenRgbClient::new(&config.openrgb_host, config.openrgb_port, config.dry_run);

    let mut connection = match client.connect().await {
        Ok(conn) => conn,
        Err(e) => {
            if config.dry_run {
                warn!("dry-run: continuing without OpenRGB ({e})");
                crate::openrgb::Connection::DryRun
            } else {
                anyhow::bail!("failed to connect to OpenRGB: {e}");
            }
        }
    };

    if let Err(e) = connection
        .set_device_mode(config.openrgb_device_id, 0)
        .await
    {
        warn!("failed to set Direct mode: {e}");
    }

    let mut zone_led_counts: HashMap<u32, u32> = HashMap::new();
    match connection
        .query_controller_data(config.openrgb_device_id)
        .await
    {
        Ok(controller) => {
            for (idx, zone) in controller.zones.iter().enumerate() {
                info!("zone {}: {} ({} leds)", idx, zone.name, zone.led_count);
                zone_led_counts.insert(idx as u32, zone.led_count);
            }
        }
        Err(e) => {
            warn!("failed to query controller data: {e}; using fallback led count of 1");
            for zone_id in &config.openrgb_zone_ids {
                zone_led_counts.insert(*zone_id, 1);
            }
        }
    }

    let colors: Vec<[u8; 3]> = vec![
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 0],
        [0, 255, 255],
        [255, 0, 255],
        [255, 255, 255],
    ];

    info!("starting color test cycle; press Ctrl+C to stop");

    for color in colors.iter().cycle() {
        info!(
            "setting color to rgb({},{},{})",
            color[0], color[1], color[2]
        );

        for zone_id in &config.openrgb_zone_ids {
            let led_count = zone_led_counts.get(zone_id).copied().unwrap_or(1);
            if let Err(e) = connection
                .set_zone_color(config.openrgb_device_id, *zone_id, led_count, *color)
                .await
            {
                warn!("failed to set color for zone {}: {}", zone_id, e);
            }
        }

        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
