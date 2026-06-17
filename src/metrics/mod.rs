use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

pub struct Metrics {
    system: System,
    components: Components,
    running: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub cpu_temp: Option<f32>,
    pub cpu_usage: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
            ),
            components: Components::new_with_refreshed_list(),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn snapshot(&mut self) -> Snapshot {
        self.system.refresh_cpu_all();
        self.components.refresh(false);

        let cpu_usage = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .fold(0.0, |acc, x| acc + x)
            / self.system.cpus().len().max(1) as f32;

        let cpu_temp = self
            .components
            .iter()
            .filter(|c| c.label().to_lowercase().contains("cpu"))
            .filter_map(|c| c.temperature())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .or_else(|| {
                self.components
                    .iter()
                    .filter_map(|c| c.temperature())
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            });

        Snapshot {
            cpu_temp,
            cpu_usage,
        }
    }

    pub fn shutdown_trigger(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_returns_values() {
        let mut metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        assert!(snapshot.cpu_usage >= 0.0);
    }

    #[test]
    fn snapshot_temperature_is_reasonable() {
        let mut metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        if let Some(temp) = snapshot.cpu_temp {
            assert!(temp > 0.0);
            assert!(temp < 120.0);
        }
    }
}
