use crate::config::MqttConfig;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, Publish, QoS, Transport};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy)]
pub struct LightCommand {
    pub on: bool,
    pub color: [u8; 3],
}

pub struct MqttClient {
    client: AsyncClient,
    topic_prefix: String,
    discovery_prefix: String,
    command_rx: mpsc::UnboundedReceiver<LightCommand>,
}

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("mqtt error: {0}")]
    Mqtt(#[from] rumqttc::ClientError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl MqttClient {
    pub async fn connect(config: &MqttConfig) -> Result<Self, MqttError> {
        let mut mqttoptions =
            MqttOptions::new("lighthouse", config.broker_host.clone(), config.broker_port);
        mqttoptions.set_keep_alive(Duration::from_secs(60));

        if config.broker_port == 8883 || config.broker_port == 443 {
            mqttoptions.set_transport(Transport::tls_with_default_config());
        }

        if let Some(username) = &config.username {
            let password = config.password.clone().unwrap_or_default();
            mqttoptions.set_credentials(username, password);
        }

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        let topic_prefix = config.topic_prefix.clone();
        let discovery_prefix = config.discovery_prefix.clone();

        client
            .subscribe(command_topic(&topic_prefix), QoS::AtLeastOnce)
            .await?;
        client
            .subscribe(rgb_command_topic(&topic_prefix), QoS::AtLeastOnce)
            .await?;

        let (command_tx, command_rx) = mpsc::unbounded_channel();

        tokio::spawn(run_event_loop(eventloop, command_tx, topic_prefix.clone()));

        let mqtt = Self {
            client,
            topic_prefix,
            discovery_prefix,
            command_rx,
        };

        mqtt.publish_discovery().await?;
        info!(
            "MQTT connected to {}:{}",
            config.broker_host, config.broker_port
        );

        Ok(mqtt)
    }

    async fn publish_discovery(&self) -> Result<(), MqttError> {
        let device = json!({
            "identifiers": ["lighthouse"],
            "name": "Lighthouse",
            "model": "Lighthouse",
            "manufacturer": "Lighthouse"
        });

        let sensors = [
            (
                "cpu_temp",
                "CPU Temperature",
                "sensor/lighthouse_cpu_temp/state",
                Some("temperature"),
                Some("°C"),
            ),
            (
                "cpu_usage",
                "CPU Usage",
                "sensor/lighthouse_cpu_usage/state",
                None,
                Some("%"),
            ),
        ];

        for (object_id, name, state_topic, device_class, unit) in sensors {
            let payload = json!({
                "name": name,
                "state_topic": format!("{}/{}", self.topic_prefix, state_topic),
                "unique_id": format!("lighthouse_{}", object_id),
                "device": device.clone(),
                "device_class": device_class,
                "unit_of_measurement": unit,
            });
            let topic = format!(
                "{}/sensor/lighthouse_{}/config",
                self.discovery_prefix, object_id
            );
            self.client
                .publish(
                    topic,
                    QoS::AtLeastOnce,
                    true,
                    serde_json::to_string(&payload)?,
                )
                .await?;
        }

        let light_config = json!({
            "name": "Lighthouse",
            "command_topic": command_topic(&self.topic_prefix),
            "state_topic": state_topic(&self.topic_prefix),
            "rgb_command_topic": rgb_command_topic(&self.topic_prefix),
            "rgb_state_topic": rgb_state_topic(&self.topic_prefix),
            "payload_on": "ON",
            "payload_off": "OFF",
            "unique_id": "lighthouse_light",
            "device": device,
        });
        let topic = format!("{}/light/lighthouse/config", self.discovery_prefix);
        self.client
            .publish(
                topic,
                QoS::AtLeastOnce,
                true,
                serde_json::to_string(&light_config)?,
            )
            .await?;

        Ok(())
    }

    pub fn publish_states(&self, temp: Option<f32>, usage: f32, color: [u8; 3], on: bool) {
        let temp_topic = format!("{}/sensor/lighthouse_cpu_temp/state", self.topic_prefix);
        let usage_topic = format!("{}/sensor/lighthouse_cpu_usage/state", self.topic_prefix);
        let state_topic = state_topic(&self.topic_prefix);
        let rgb_state_topic = rgb_state_topic(&self.topic_prefix);

        let temp_payload = temp.map(|t| format!("{:.1}", t)).unwrap_or_default();
        let usage_payload = format!("{:.1}", usage);
        let state_payload = if on { "ON" } else { "OFF" };
        let rgb_payload = format!("{},{},{}", color[0], color[1], color[2]);

        let client = self.client.clone();
        let tp = temp_topic.clone();
        let up = usage_topic.clone();
        tokio::spawn(async move {
            let _ = client
                .publish(tp, QoS::AtLeastOnce, false, temp_payload)
                .await;
            let _ = client
                .publish(up, QoS::AtLeastOnce, false, usage_payload)
                .await;
            let _ = client
                .publish(state_topic, QoS::AtLeastOnce, false, state_payload)
                .await;
            let _ = client
                .publish(rgb_state_topic, QoS::AtLeastOnce, false, rgb_payload)
                .await;
        });
    }

    pub async fn recv_command(&mut self) -> Option<LightCommand> {
        self.command_rx.recv().await
    }
}

async fn run_event_loop(
    mut eventloop: EventLoop,
    command_tx: mpsc::UnboundedSender<LightCommand>,
    topic_prefix: String,
) {
    let on_topic = command_topic(&topic_prefix);
    let rgb_topic = rgb_command_topic(&topic_prefix);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                handle_publish(&publish, &command_tx, &on_topic, &rgb_topic);
            }
            Ok(_) => {}
            Err(e) => {
                warn!("MQTT event loop error: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn handle_publish(
    publish: &Publish,
    command_tx: &mpsc::UnboundedSender<LightCommand>,
    on_topic: &str,
    rgb_topic: &str,
) {
    let payload = String::from_utf8_lossy(&publish.payload);

    if publish.topic == on_topic {
        let on = payload.eq_ignore_ascii_case("ON");
        let _ = command_tx.send(LightCommand {
            on,
            color: [255, 255, 255],
        });
    } else if publish.topic == rgb_topic {
        if let Some(color) = parse_rgb(&payload) {
            let _ = command_tx.send(LightCommand { on: true, color });
        }
    }
}

fn parse_rgb(value: &str) -> Option<[u8; 3]> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    Some([
        parts[0].trim().parse().ok()?,
        parts[1].trim().parse().ok()?,
        parts[2].trim().parse().ok()?,
    ])
}

fn command_topic(prefix: &str) -> String {
    format!("{}/light/set", prefix)
}

fn rgb_command_topic(prefix: &str) -> String {
    format!("{}/light/rgb/set", prefix)
}

fn state_topic(prefix: &str) -> String {
    format!("{}/light/state", prefix)
}

fn rgb_state_topic(prefix: &str) -> String {
    format!("{}/light/rgb/state", prefix)
}
