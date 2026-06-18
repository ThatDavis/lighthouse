use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub openrgb_host: String,
    pub openrgb_port: u16,
    #[serde(default)]
    pub openrgb_device_id: u32,
    #[serde(default = "default_zone_ids")]
    pub openrgb_zone_ids: Vec<u32>,
    pub poll_interval: u64,
    #[serde(default = "default_temp_smoothing")]
    pub temp_smoothing: f32,
    #[serde(default = "default_transition_steps")]
    pub transition_steps: u32,
    #[serde(default = "default_transition_interval_ms")]
    pub transition_interval_ms: u64,
    pub temperature: TemperatureConfig,
    pub colors: ColorConfig,
    #[serde(default)]
    pub effects: EffectsConfig,
    #[serde(default)]
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub dry_run: bool,
}

fn default_zone_ids() -> Vec<u32> {
    vec![0]
}

fn default_temp_smoothing() -> f32 {
    1.0
}

fn default_transition_steps() -> u32 {
    1
}

fn default_transition_interval_ms() -> u64 {
    100
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct TemperatureConfig {
    pub cold: f32,
    pub warm: f32,
    pub hot: f32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct ColorConfig {
    pub cold: [u8; 3],
    pub warm: [u8; 3],
    pub hot: [u8; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct MqttConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mqtt_broker")]
    pub broker_host: String,
    #[serde(default = "default_mqtt_port")]
    pub broker_port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_mqtt_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default = "default_mqtt_discovery_prefix")]
    pub discovery_prefix: String,
}

fn default_mqtt_broker() -> String {
    "127.0.0.1".to_string()
}

fn default_mqtt_port() -> u16 {
    1883
}

fn default_mqtt_topic_prefix() -> String {
    "lighthouse".to_string()
}

fn default_mqtt_discovery_prefix() -> String {
    "homeassistant".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct EffectsConfig {
    #[serde(default = "default_active_profile")]
    pub active_profile: String,
    #[serde(default)]
    pub profiles: Vec<EffectProfile>,
    #[serde(default)]
    pub schedules: Vec<Schedule>,
}

fn default_active_profile() -> String {
    "temperature".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Schedule {
    pub start: String,
    pub end: String,
    pub profile: String,
    #[serde(default)]
    pub outside_range: OutsideRange,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutsideRange {
    #[default]
    ActiveProfile,
    Off,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EffectProfile {
    pub name: String,
    #[serde(default)]
    pub effects: Vec<EffectEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectEntry {
    Temperature,
    CpuUsage {
        #[serde(default = "default_usage_low")]
        low: f32,
        #[serde(default = "default_usage_high")]
        high: f32,
        #[serde(default = "default_usage_low_color")]
        low_color: [u8; 3],
        #[serde(default = "default_usage_high_color")]
        high_color: [u8; 3],
    },
    Pulse {
        color: [u8; 3],
        #[serde(default = "default_effect_speed")]
        speed: f32,
    },
    Breathe {
        #[serde(default = "default_effect_speed")]
        speed: f32,
    },
    Cycle {
        #[serde(default = "default_effect_speed")]
        speed: f32,
    },
}

fn default_usage_low() -> f32 {
    0.0
}

fn default_usage_high() -> f32 {
    100.0
}

fn default_usage_low_color() -> [u8; 3] {
    [0, 255, 0]
}

fn default_usage_high_color() -> [u8; 3] {
    [255, 0, 0]
}

fn default_effect_speed() -> f32 {
    1.0
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid thresholds: cold ({cold}) must be <= warm ({warm}) <= hot ({hot})")]
    InvalidThresholds { cold: f32, warm: f32, hot: f32 },
    #[error("invalid temp_smoothing: {value}; must be between 0.0 and 1.0")]
    InvalidTempSmoothing { value: f32 },
    #[error("invalid transition_steps: {value}; must be >= 1")]
    InvalidTransitionSteps { value: u32 },
    #[error("invalid transition_interval_ms: {value}; must be >= 10")]
    InvalidTransitionInterval { value: u64 },
    #[error("unknown active_profile: {name}")]
    UnknownProfile { name: String },
    #[error("invalid cpu_usage range: low ({low}) must be < high ({high})")]
    InvalidUsageRange { low: f32, high: f32 },
    #[error("invalid schedule time: {value}; expected HH:MM")]
    InvalidScheduleTime { value: String },
    #[error("schedule end ({end}) must be after start ({start})")]
    InvalidScheduleRange { start: String, end: String },
    #[error("unknown schedule profile: {name}")]
    UnknownScheduleProfile { name: String },
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(self.temperature.cold <= self.temperature.warm
            && self.temperature.warm <= self.temperature.hot)
        {
            return Err(ConfigError::InvalidThresholds {
                cold: self.temperature.cold,
                warm: self.temperature.warm,
                hot: self.temperature.hot,
            });
        }
        if !(0.0..=1.0).contains(&self.temp_smoothing) {
            return Err(ConfigError::InvalidTempSmoothing {
                value: self.temp_smoothing,
            });
        }
        if self.transition_steps < 1 {
            return Err(ConfigError::InvalidTransitionSteps {
                value: self.transition_steps,
            });
        }
        if self.transition_interval_ms < 10 {
            return Err(ConfigError::InvalidTransitionInterval {
                value: self.transition_interval_ms,
            });
        }

        let profile_names: std::collections::HashSet<_> =
            self.effects.profiles.iter().map(|p| &p.name).collect();
        if !profile_names.contains(&self.effects.active_profile)
            && !self.effects.profiles.is_empty()
        {
            return Err(ConfigError::UnknownProfile {
                name: self.effects.active_profile.clone(),
            });
        }

        for profile in &self.effects.profiles {
            for effect in &profile.effects {
                if let EffectEntry::CpuUsage { low, high, .. } = effect {
                    if *low >= *high {
                        return Err(ConfigError::InvalidUsageRange {
                            low: *low,
                            high: *high,
                        });
                    }
                }
            }
        }

        for schedule in &self.effects.schedules {
            let start = parse_time(&schedule.start).map_err(|_| ConfigError::InvalidScheduleTime {
                value: schedule.start.clone(),
            })?;
            let end = parse_time(&schedule.end).map_err(|_| ConfigError::InvalidScheduleTime {
                value: schedule.end.clone(),
            })?;
            if start == end {
                return Err(ConfigError::InvalidScheduleRange {
                    start: schedule.start.clone(),
                    end: schedule.end.clone(),
                });
            }
            if !profile_names.contains(&schedule.profile) {
                return Err(ConfigError::UnknownScheduleProfile {
                    name: schedule.profile.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn color_for_temperature(&self, temp: f32) -> [u8; 3] {
        let t = &self.temperature;
        if temp <= t.cold {
            self.colors.cold
        } else if temp >= t.hot {
            self.colors.hot
        } else if temp <= t.warm {
            let ratio = (temp - t.cold) / (t.warm - t.cold);
            interpolate(self.colors.cold, self.colors.warm, ratio)
        } else {
            let ratio = (temp - t.warm) / (t.hot - t.warm);
            interpolate(self.colors.warm, self.colors.hot, ratio)
        }
    }

    pub fn interpolate_color(a: [u8; 3], b: [u8; 3], ratio: f32) -> [u8; 3] {
        interpolate(a, b, ratio)
    }

    pub fn default_path() -> PathBuf {
        let etc_path = PathBuf::from("/etc/lighthouse/config.toml");
        if etc_path.exists() {
            return etc_path;
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lighthouse")
            .join("config.toml")
    }
}

fn parse_time(value: &str) -> Result<u16, ()> {
    let mut parts = value.split(':');
    let hour: u16 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    let minute: u16 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    if parts.next().is_some() || hour >= 24 || minute >= 60 {
        return Err(());
    }
    Ok(hour * 60 + minute)
}

pub fn current_minutes() -> u16 {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = since_epoch.as_secs();
    ((secs / 3600 % 24) * 60 + (secs / 60 % 60)) as u16
}

fn interpolate(a: [u8; 3], b: [u8; 3], ratio: f32) -> [u8; 3] {
    let r = (a[0] as f32 + (b[0] as f32 - a[0] as f32) * ratio) as u8;
    let g = (a[1] as f32 + (b[1] as f32 - a[1] as f32) * ratio) as u8;
    let b_ = (a[2] as f32 + (b[2] as f32 - a[2] as f32) * ratio) as u8;
    [r, g, b_]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            openrgb_host: "127.0.0.1".to_string(),
            openrgb_port: 6742,
            openrgb_device_id: 0,
            openrgb_zone_ids: vec![0],
            poll_interval: 1,
            temp_smoothing: 1.0,
            transition_steps: 1,
            transition_interval_ms: 100,
            temperature: TemperatureConfig {
                cold: 35.0,
                warm: 55.0,
                hot: 75.0,
            },
            colors: ColorConfig {
                cold: [0, 0, 255],
                warm: [255, 255, 0],
                hot: [255, 0, 0],
            },
            effects: EffectsConfig::default(),
            mqtt: MqttConfig::default(),
            dry_run: false,
        }
    }

    #[test]
    fn validates_thresholds() {
        let mut config = test_config();
        assert!(config.validate().is_ok());

        config.temperature.cold = 80.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validates_smoothing_options() {
        let mut config = test_config();
        config.temp_smoothing = 1.5;
        assert!(config.validate().is_err());

        config.temp_smoothing = 0.5;
        config.transition_steps = 0;
        assert!(config.validate().is_err());

        config.transition_steps = 5;
        config.transition_interval_ms = 5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validates_active_profile() {
        let mut config = test_config();
        config.effects.active_profile = "missing".to_string();
        config.effects.profiles.push(EffectProfile {
            name: "existing".to_string(),
            effects: vec![EffectEntry::Temperature],
        });
        assert!(config.validate().is_err());
    }

    #[test]
    fn validates_cpu_usage_range() {
        let mut config = test_config();
        config.effects.profiles.push(EffectProfile {
            name: "load".to_string(),
            effects: vec![EffectEntry::CpuUsage {
                low: 100.0,
                high: 0.0,
                low_color: [0, 0, 0],
                high_color: [255, 0, 0],
            }],
        });
        config.effects.active_profile = "load".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn color_interpolates_between_thresholds() {
        let config = test_config();

        let mid_cold_warm = config.color_for_temperature(45.0);
        assert!(mid_cold_warm[0] > 0);
        assert!(mid_cold_warm[2] > 0);

        let mid_warm_hot = config.color_for_temperature(65.0);
        assert!(mid_warm_hot[0] > 0);
        assert!(mid_warm_hot[1] < 255);
    }

    #[test]
    fn interpolate_color_is_linear() {
        let a = [0, 0, 0];
        let b = [100, 200, 50];
        let mid = Config::interpolate_color(a, b, 0.5);
        assert_eq!(mid, [50, 100, 25]);
    }

    #[test]
    fn loads_zone_ids_from_file() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
openrgb_host = "127.0.0.1"
openrgb_port = 6742
openrgb_zone_ids = [0, 1]
poll_interval = 2

temp_smoothing = 0.8
transition_steps = 10
transition_interval_ms = 50

[temperature]
cold = 30.0
warm = 50.0
hot = 70.0

[colors]
cold = [0, 0, 255]
warm = [255, 255, 0]
hot = [255, 0, 0]

[effects]
active_profile = "load"

[[effects.profiles]]
name = "load"

[[effects.profiles.effects]]
type = "cpu_usage"
low = 0.0
high = 100.0
low_color = [0, 255, 0]
high_color = [255, 0, 0]
"#
        )
        .unwrap();

        let config = Config::from_file(tmp.path()).unwrap();
        assert_eq!(config.openrgb_zone_ids, vec![0, 1]);
        assert_eq!(config.temp_smoothing, 0.8);
        assert_eq!(config.transition_steps, 10);
        assert_eq!(config.transition_interval_ms, 50);
        assert_eq!(config.effects.active_profile, "load");
        assert_eq!(config.effects.profiles.len(), 1);
    }
}
