use crate::config::{EffectsConfig, OutsideRange, Schedule};

pub fn resolve_profile<'a>(effects: &'a EffectsConfig, now_minutes: u16) -> ProfileSelection<'a> {
    for schedule in &effects.schedules {
        if let Some((start, end)) = parse_range(&schedule.start, &schedule.end) {
            if start <= end {
                if now_minutes >= start && now_minutes < end {
                    return ProfileSelection::Scheduled(&schedule.profile);
                }
            } else if now_minutes >= start || now_minutes < end {
                // Wraps past midnight.
                return ProfileSelection::Scheduled(&schedule.profile);
            }
        }
        match schedule.outside_range {
            OutsideRange::Off if is_outside_range(schedule, now_minutes) => {
                return ProfileSelection::Off;
            }
            _ => {}
        }
    }

    ProfileSelection::ActiveProfile
}

fn is_outside_range(schedule: &Schedule, now_minutes: u16) -> bool {
    let (start, end) = match parse_range(&schedule.start, &schedule.end) {
        Some(range) => range,
        None => return false,
    };
    if start <= end {
        now_minutes < start || now_minutes >= end
    } else {
        now_minutes < end && now_minutes >= start
    }
}

fn parse_range(start: &str, end: &str) -> Option<(u16, u16)> {
    let start = parse_time(start)?;
    let end = parse_time(end)?;
    Some((start, end))
}

fn parse_time(value: &str) -> Option<u16> {
    let mut parts = value.split(':');
    let hour: u16 = parts.next()?.parse().ok()?;
    let minute: u16 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || hour >= 24 || minute >= 60 {
        return None;
    }
    Some(hour * 60 + minute)
}

pub enum ProfileSelection<'a> {
    ActiveProfile,
    Scheduled(&'a str),
    Off,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EffectEntry, EffectProfile};

    fn empty_effects() -> EffectsConfig {
        EffectsConfig {
            active_profile: "default".to_string(),
            profiles: vec![EffectProfile {
                name: "day".to_string(),
                effects: vec![EffectEntry::Temperature],
            }],
            schedules: vec![],
        }
    }

    #[test]
    fn no_schedules_uses_active_profile() {
        let effects = empty_effects();
        assert!(matches!(
            resolve_profile(&effects, 720),
            ProfileSelection::ActiveProfile
        ));
    }

    #[test]
    fn schedule_inside_range() {
        let mut effects = empty_effects();
        effects.schedules.push(Schedule {
            start: "08:00".to_string(),
            end: "18:00".to_string(),
            profile: "day".to_string(),
            outside_range: OutsideRange::ActiveProfile,
        });
        assert!(matches!(
            resolve_profile(&effects, 720),
            ProfileSelection::Scheduled(profile) if profile == "day"
        ));
    }

    #[test]
    fn schedule_outside_range_off() {
        let mut effects = empty_effects();
        effects.schedules.push(Schedule {
            start: "08:00".to_string(),
            end: "18:00".to_string(),
            profile: "day".to_string(),
            outside_range: OutsideRange::Off,
        });
        assert!(matches!(
            resolve_profile(&effects, 420),
            ProfileSelection::Off
        ));
    }

    #[test]
    fn schedule_wraps_midnight() {
        let mut effects = empty_effects();
        effects.schedules.push(Schedule {
            start: "22:00".to_string(),
            end: "06:00".to_string(),
            profile: "night".to_string(),
            outside_range: OutsideRange::ActiveProfile,
        });
        assert!(matches!(
            resolve_profile(&effects, 120),
            ProfileSelection::Scheduled(profile) if profile == "night"
        ));
    }
}
