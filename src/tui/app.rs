use crate::config::Config;
use crate::effects::EffectContext;
use crate::metrics::Metrics;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Status,
    Thresholds,
    Colors,
    Effects,
    Daemon,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Screen::Status => "Status",
            Screen::Thresholds => "Thresholds",
            Screen::Colors => "Colors",
            Screen::Effects => "Effects",
            Screen::Daemon => "Daemon",
        }
    }

    pub fn all() -> &'static [Screen] {
        &[
            Screen::Status,
            Screen::Thresholds,
            Screen::Colors,
            Screen::Effects,
            Screen::Daemon,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    Unknown,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Cold,
    Warm,
    Hot,
    Red,
    Green,
    Blue,
    ActiveProfile,
    TempSmoothing,
    TransitionSteps,
    TransitionInterval,
}

pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub metrics: Metrics,
    pub screen: Screen,
    pub daemon_status: DaemonStatus,
    pub status_message: String,
    pub input_mode: InputMode,
    pub selected_field: usize,
    pub thresholds: [String; 3],
    pub colors: [String; 3],
    pub active_profile: String,
    pub temp_smoothing: String,
    pub transition_steps: String,
    pub transition_interval: String,
    pub current_profile: String,
    pub current_color: [u8; 3],
    pub current_temp: Option<f32>,
    pub current_usage: f32,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let thresholds = [
            config.temperature.cold.to_string(),
            config.temperature.warm.to_string(),
            config.temperature.hot.to_string(),
        ];
        let colors = [
            color_to_string(config.colors.cold),
            color_to_string(config.colors.warm),
            color_to_string(config.colors.hot),
        ];
        Self {
            active_profile: config.effects.active_profile.clone(),
            temp_smoothing: config.temp_smoothing.to_string(),
            transition_steps: config.transition_steps.to_string(),
            transition_interval: config.transition_interval_ms.to_string(),
            current_profile: config.effects.active_profile.clone(),
            config,
            config_path,
            metrics: Metrics::new(1.0),
            screen: Screen::Status,
            daemon_status: DaemonStatus::Unknown,
            status_message: String::new(),
            input_mode: InputMode::Normal,
            selected_field: 0,
            thresholds,
            colors,
            current_color: [0; 3],
            current_temp: None,
            current_usage: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let snapshot = self.metrics.snapshot();
        self.current_temp = snapshot.cpu_temp;
        self.current_usage = snapshot.cpu_usage;

        let ctx = EffectContext::with_telemetry(snapshot.cpu_temp, snapshot.cpu_usage);
        self.current_color = crate::effects::profile::build_profile(
            &self.config.effects.active_profile,
            &self.config.effects,
            &self.config.temperature,
            &self.config.colors,
        )
        .map(|p| crate::effects::profile::render(&p, &ctx))
        .unwrap_or_else(|| {
            self.config
                .color_for_temperature(snapshot.cpu_temp.unwrap_or(35.0))
        });

        self.current_profile = crate::effects::schedule::resolve_profile(
            &self.config.effects,
            crate::config::current_minutes(),
        )
        .selected_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| self.config.effects.active_profile.clone());
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }
}

fn color_to_string(color: [u8; 3]) -> String {
    format!("{}, {}, {}", color[0], color[1], color[2])
}

pub fn parse_color(parts: [&str; 3]) -> Option<[u8; 3]> {
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

pub trait ProfileSelectionExt {
    fn selected_name(&self) -> Option<&str>;
}

impl ProfileSelectionExt for crate::effects::schedule::ProfileSelection<'_> {
    fn selected_name(&self) -> Option<&str> {
        match self {
            crate::effects::schedule::ProfileSelection::Scheduled(name) => Some(name),
            _ => None,
        }
    }
}
