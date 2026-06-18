use crate::config::{ColorConfig, TemperatureConfig};
use std::f32::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod profile;
pub mod schedule;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BlendMode {
    #[default]
    Replace,
    Multiply,
    Add,
}

#[derive(Debug, Clone, Copy)]
pub struct EffectContext {
    pub cpu_temp: Option<f32>,
    pub cpu_usage: f32,
    pub time: f32,
}

impl EffectContext {
    pub fn now() -> Self {
        Self {
            cpu_temp: None,
            cpu_usage: 0.0,
            time: seconds_since_epoch(),
        }
    }

    pub fn with_telemetry(cpu_temp: Option<f32>, cpu_usage: f32) -> Self {
        Self {
            cpu_temp,
            cpu_usage,
            time: seconds_since_epoch(),
        }
    }
}

fn seconds_since_epoch() -> f32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32()
}

pub trait Effect: Send + Sync + std::fmt::Debug {
    fn render(&self, ctx: &EffectContext, base: [u8; 3]) -> [u8; 3];
}

#[derive(Debug, Clone, Copy)]
pub struct TemperatureEffect {
    pub thresholds: TemperatureConfig,
    pub colors: ColorConfig,
}

impl Effect for TemperatureEffect {
    fn render(&self, ctx: &EffectContext, _base: [u8; 3]) -> [u8; 3] {
        let temp = match ctx.cpu_temp {
            Some(t) => t,
            None => return _base,
        };
        color_for_temperature(temp, &self.thresholds, &self.colors)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CpuUsageEffect {
    pub low: f32,
    pub high: f32,
    pub low_color: [u8; 3],
    pub high_color: [u8; 3],
}

impl Effect for CpuUsageEffect {
    fn render(&self, ctx: &EffectContext, _base: [u8; 3]) -> [u8; 3] {
        let t = ((ctx.cpu_usage - self.low) / (self.high - self.low)).clamp(0.0, 1.0);
        interpolate(self.low_color, self.high_color, t)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PulseEffect {
    pub color: [u8; 3],
    pub speed: f32,
}

impl Effect for PulseEffect {
    fn render(&self, ctx: &EffectContext, base: [u8; 3]) -> [u8; 3] {
        let phase = (ctx.time * self.speed * PI * 2.0).sin();
        let on = phase > 0.0;
        if on {
            blend_add(base, self.color)
        } else {
            base
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BreatheEffect {
    pub speed: f32,
}

impl Effect for BreatheEffect {
    fn render(&self, ctx: &EffectContext, base: [u8; 3]) -> [u8; 3] {
        let factor = ((ctx.time * self.speed * PI * 2.0).sin() + 1.0) / 2.0;
        let min_brightness = 0.25;
        let brightness = min_brightness + factor * (1.0 - min_brightness);
        scale_brightness(base, brightness)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CycleEffect {
    pub speed: f32,
}

impl Effect for CycleEffect {
    fn render(&self, ctx: &EffectContext, _base: [u8; 3]) -> [u8; 3] {
        hue_to_rgb(ctx.time * self.speed)
    }
}

fn color_for_temperature(
    temp: f32,
    thresholds: &TemperatureConfig,
    colors: &ColorConfig,
) -> [u8; 3] {
    let t = thresholds;
    if temp <= t.cold {
        colors.cold
    } else if temp >= t.hot {
        colors.hot
    } else if temp <= t.warm {
        let ratio = (temp - t.cold) / (t.warm - t.cold);
        interpolate(colors.cold, colors.warm, ratio)
    } else {
        let ratio = (temp - t.warm) / (t.hot - t.warm);
        interpolate(colors.warm, colors.hot, ratio)
    }
}

fn interpolate(a: [u8; 3], b: [u8; 3], ratio: f32) -> [u8; 3] {
    let r = (a[0] as f32 + (b[0] as f32 - a[0] as f32) * ratio) as u8;
    let g = (a[1] as f32 + (b[1] as f32 - a[1] as f32) * ratio) as u8;
    let b_ = (a[2] as f32 + (b[2] as f32 - a[2] as f32) * ratio) as u8;
    [r, g, b_]
}

fn blend_add(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    [
        (a[0] as u16 + b[0] as u16).min(255) as u8,
        (a[1] as u16 + b[1] as u16).min(255) as u8,
        (a[2] as u16 + b[2] as u16).min(255) as u8,
    ]
}

fn scale_brightness(color: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (color[0] as f32 * factor) as u8,
        (color[1] as f32 * factor) as u8,
        (color[2] as f32 * factor) as u8,
    ]
}

fn hue_to_rgb(hue: f32) -> [u8; 3] {
    let h = hue.rem_euclid(1.0) * 6.0;
    let sector = h as i32;
    let f = h - sector as f32;
    let v = 255u8;
    let p = 0u8;
    let q = (255.0 * (1.0 - f)) as u8;
    let t = (255.0 * f) as u8;

    match sector {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_effect_maps_color() {
        let effect = TemperatureEffect {
            thresholds: TemperatureConfig {
                cold: 30.0,
                warm: 50.0,
                hot: 70.0,
            },
            colors: ColorConfig {
                cold: [0, 0, 255],
                warm: [255, 255, 0],
                hot: [255, 0, 0],
            },
        };
        let ctx = EffectContext::with_telemetry(Some(30.0), 0.0);
        assert_eq!(effect.render(&ctx, [0, 0, 0]), [0, 0, 255]);
    }

    #[test]
    fn cpu_usage_effect_interpolates() {
        let effect = CpuUsageEffect {
            low: 0.0,
            high: 100.0,
            low_color: [0, 0, 0],
            high_color: [100, 0, 0],
        };
        let ctx = EffectContext::with_telemetry(None, 50.0);
        assert_eq!(effect.render(&ctx, [0, 0, 0]), [50, 0, 0]);
    }

    #[test]
    fn cycle_effect_returns_sane_color() {
        let effect = CycleEffect { speed: 1.0 };
        let ctx = EffectContext::now();
        let color = effect.render(&ctx, [0, 0, 0]);
        // Hue-to-RGB should never produce values outside u8; just check it runs.
        let _sum: u16 = color.iter().map(|c| *c as u16).sum();
    }

    #[test]
    fn pulse_and_breathe_do_not_panic() {
        let pulse = PulseEffect {
            color: [255, 0, 0],
            speed: 1.0,
        };
        let breathe = BreatheEffect { speed: 1.0 };
        let ctx = EffectContext::now();
        let _ = pulse.render(&ctx, [0, 0, 0]);
        let _ = breathe.render(&ctx, [255, 255, 255]);
    }
}
