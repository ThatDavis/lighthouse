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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TemperatureConfig {
    pub cold: f32,
    pub warm: f32,
    pub hot: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ColorConfig {
    pub cold: [u8; 3],
    pub warm: [u8; 3],
    pub hot: [u8; 3],
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
"#
        )
        .unwrap();

        let config = Config::from_file(tmp.path()).unwrap();
        assert_eq!(config.openrgb_zone_ids, vec![0, 1]);
        assert_eq!(config.temp_smoothing, 0.8);
        assert_eq!(config.transition_steps, 10);
        assert_eq!(config.transition_interval_ms, 50);
    }
}
