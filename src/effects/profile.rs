use crate::config::{ColorConfig, EffectEntry, EffectsConfig, TemperatureConfig};
use crate::effects::{
    BreatheEffect, CpuUsageEffect, CycleEffect, Effect, EffectContext, PulseEffect,
    TemperatureEffect,
};

pub struct Profile {
    pub name: String,
    pub effects: Vec<Box<dyn Effect>>,
}

pub fn build_profile(
    name: &str,
    config: &EffectsConfig,
    temperature: &TemperatureConfig,
    colors: &ColorConfig,
) -> Option<Profile> {
    let profile_config = config.profiles.iter().find(|p| p.name == name)?;

    let mut effects: Vec<Box<dyn Effect>> = Vec::new();
    for entry in &profile_config.effects {
        let effect: Box<dyn Effect> = match entry {
            EffectEntry::Temperature => Box::new(TemperatureEffect {
                thresholds: *temperature,
                colors: *colors,
            }),
            EffectEntry::CpuUsage {
                low,
                high,
                low_color,
                high_color,
            } => Box::new(CpuUsageEffect {
                low: *low,
                high: *high,
                low_color: *low_color,
                high_color: *high_color,
            }),
            EffectEntry::Pulse { color, speed } => Box::new(PulseEffect {
                color: *color,
                speed: *speed,
            }),
            EffectEntry::Breathe { speed } => Box::new(BreatheEffect { speed: *speed }),
            EffectEntry::Cycle { speed } => Box::new(CycleEffect { speed: *speed }),
        };
        effects.push(effect);
    }

    Some(Profile {
        name: name.to_string(),
        effects,
    })
}

pub fn render(profile: &Profile, ctx: &EffectContext) -> [u8; 3] {
    let mut color = [0u8; 3];
    for effect in &profile.effects {
        color = effect.render(ctx, color);
    }
    color
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config() -> TemperatureConfig {
        TemperatureConfig {
            cold: 30.0,
            warm: 50.0,
            hot: 70.0,
        }
    }

    fn color_config() -> ColorConfig {
        ColorConfig {
            cold: [0, 0, 255],
            warm: [255, 255, 0],
            hot: [255, 0, 0],
        }
    }

    #[test]
    fn builds_and_renders_temperature_profile() {
        let effects = EffectsConfig {
            active_profile: "temp".to_string(),
            profiles: vec![crate::config::EffectProfile {
                name: "temp".to_string(),
                effects: vec![EffectEntry::Temperature],
            }],
        };
        let profile = build_profile("temp", &effects, &temp_config(), &color_config()).unwrap();
        let ctx = EffectContext::with_telemetry(Some(30.0), 0.0);
        assert_eq!(render(&profile, &ctx), [0, 0, 255]);
    }
}
