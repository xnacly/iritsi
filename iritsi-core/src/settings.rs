use std::time::Duration;

pub const DEFAULT_ACCENT: &str = "#ff6505";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateInterval {
    Every(Duration),
    Manual,
}

impl UpdateInterval {
    pub const DEFAULT: UpdateInterval = UpdateInterval::hours(12);

    pub const CHOICES: [UpdateInterval; 7] = [
        UpdateInterval::hours(1),
        UpdateInterval::hours(6),
        UpdateInterval::hours(12),
        UpdateInterval::hours(24),
        UpdateInterval::hours(72),
        UpdateInterval::hours(168),
        UpdateInterval::Manual,
    ];

    const fn hours(hours: u64) -> Self {
        UpdateInterval::Every(Duration::from_secs(hours * 60 * 60))
    }

    pub(crate) fn to_setting(self) -> String {
        match self {
            UpdateInterval::Every(every) => every.as_secs().to_string(),
            UpdateInterval::Manual => "manual".to_owned(),
        }
    }

    fn from_setting(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(UpdateInterval::Manual),
            secs => secs
                .parse()
                .ok()
                .filter(|&secs| secs > 0)
                .map(|secs| UpdateInterval::Every(Duration::from_secs(secs))),
        }
    }
}

impl Default for UpdateInterval {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub const DEPARTURE_CHOICES: [u32; 4] = [5, 10, 20, 30];
pub const DEFAULT_DEPARTURES: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeDisplay {
    #[default]
    Both,
    Clock,
    Countdown,
}

impl TimeDisplay {
    pub const ALL: [TimeDisplay; 3] = [
        TimeDisplay::Both,
        TimeDisplay::Clock,
        TimeDisplay::Countdown,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TimeDisplay::Both => "both",
            TimeDisplay::Clock => "clock",
            TimeDisplay::Countdown => "countdown",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|display| display.name() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub update_interval: UpdateInterval,
    pub accent: String,
    pub departures: u32,
    pub time_display: TimeDisplay,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            update_interval: UpdateInterval::default(),
            accent: DEFAULT_ACCENT.to_owned(),
            departures: DEFAULT_DEPARTURES,
            time_display: TimeDisplay::default(),
        }
    }
}

pub(crate) const UPDATE_INTERVAL: &str = "update_interval";
pub(crate) const ACCENT: &str = "accent";
pub(crate) const DEPARTURES: &str = "departures";
pub(crate) const TIME_DISPLAY: &str = "time_display";
pub(crate) const KEYS: [&str; 4] = [UPDATE_INTERVAL, ACCENT, DEPARTURES, TIME_DISPLAY];

impl Settings {
    pub(crate) fn from_values(get: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Settings::default();
        Self {
            update_interval: get(UPDATE_INTERVAL)
                .and_then(|v| UpdateInterval::from_setting(&v))
                .unwrap_or(defaults.update_interval),
            accent: get(ACCENT)
                .filter(|v| is_hex_color(v))
                .unwrap_or(defaults.accent),
            departures: get(DEPARTURES)
                .and_then(|v| v.parse().ok())
                .filter(|v| DEPARTURE_CHOICES.contains(v))
                .unwrap_or(defaults.departures),
            time_display: get(TIME_DISPLAY)
                .and_then(|v| TimeDisplay::from_name(&v))
                .unwrap_or(defaults.time_display),
        }
    }
}

pub fn is_hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stored_values_and_falls_back_to_defaults() {
        let settings = Settings::from_values(|key| match key {
            UPDATE_INTERVAL => Some("manual".into()),
            ACCENT => Some("#00ff00".into()),
            DEPARTURES => Some("20".into()),
            TIME_DISPLAY => Some("countdown".into()),
            _ => None,
        });
        assert_eq!(settings.update_interval, UpdateInterval::Manual);
        assert_eq!(settings.accent, "#00ff00");
        assert_eq!(settings.departures, 20);
        assert_eq!(settings.time_display, TimeDisplay::Countdown);

        let broken = Settings::from_values(|key| match key {
            UPDATE_INTERVAL => Some("0".into()),
            ACCENT => Some("orange".into()),
            DEPARTURES => Some("7".into()),
            TIME_DISPLAY => Some("sundial".into()),
            _ => None,
        });
        assert_eq!(broken, Settings::default());
    }

    #[test]
    fn roundtrips_intervals() {
        for interval in UpdateInterval::CHOICES {
            assert_eq!(
                UpdateInterval::from_setting(&interval.to_setting()),
                Some(interval)
            );
        }
    }
}
