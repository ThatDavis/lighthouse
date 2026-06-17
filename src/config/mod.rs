use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub openrgb_host: String,
    pub openrgb_port: u16,
    pub poll_interval: u64,
    pub temperature: TemperatureConfig,
    pub colors: ColorConfig,
    #[serde(default)]
    pub dry_run: bool,
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

    pub fn default_path() -> PathBuf {
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
            poll_interval: 1,
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
    fn color_at_boundaries() {
        let config = test_config();
        assert_eq!(config.color_for_temperature(30.0), [0, 0, 255]);
        assert_eq!(config.color_for_temperature(35.0), [0, 0, 255]);
        assert_eq!(config.color_for_temperature(75.0), [255, 0, 0]);
        assert_eq!(config.color_for_temperature(80.0), [255, 0, 0]);
    }
}
