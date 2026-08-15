//! Versioned App preferences shared with local-state persistence.

use std::fmt;

use crate::Era;

/// Current on-disk preferences schema.
pub const PREFERENCES_SCHEMA_VERSION: u8 = 1;

/// The window presentation requested for the next App launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowModePreference {
    /// A normal resizable window.
    #[default]
    Windowed,
    /// Desktop-sized fullscreen without changing the display mode.
    Borderless,
    /// Fullscreen using a monitor video mode when one is available.
    Exclusive,
}

impl WindowModePreference {
    /// Stable serialized name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
            Self::Exclusive => "exclusive",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "windowed" => Some(Self::Windowed),
            "borderless" => Some(Self::Borderless),
            "exclusive" => Some(Self::Exclusive),
            _ => None,
        }
    }
}

/// Player-selected App preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppPreferences {
    /// Master volume as an exact percentage.
    pub volume_percent: u8,
    /// Whether all App sound is muted.
    pub muted: bool,
    /// Visual treatment applied to the rendered room.
    pub era: Era,
    /// Window presentation requested for the next launch.
    pub window_mode: WindowModePreference,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            volume_percent: 45,
            muted: false,
            era: Era::Modern,
            window_mode: WindowModePreference::Windowed,
        }
    }
}

impl AppPreferences {
    /// Serialize the complete current schema in stable key order.
    #[must_use]
    pub fn to_text(self) -> String {
        let era = match self.era {
            Era::Phosphor => "phosphor",
            Era::EightBit => "8-bit",
            Era::Vector => "vector",
            Era::Modern => "modern",
        };
        format!(
            "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent {}\nmuted {}\nera {era}\nwindow_mode {}\n",
            self.volume_percent,
            self.muted,
            self.window_mode.name()
        )
    }

    /// Parse one complete preferences document.
    ///
    /// Unknown, duplicate, missing, or out-of-range fields are rejected as one
    /// unit so a damaged file cannot apply a surprising partial configuration.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error when the schema or any field is invalid.
    pub fn try_from_text(text: &str) -> Result<Self, PreferencesError> {
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| PreferencesError::new("preferences file is empty"))?;
        if header != format!("NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}") {
            return Err(PreferencesError::new(
                "preferences schema is missing or unsupported",
            ));
        }

        let mut volume_percent = None;
        let mut muted = None;
        let mut era = None;
        let mut window_mode = None;
        for line in lines {
            let mut parts = line.split_whitespace();
            let key = parts
                .next()
                .ok_or_else(|| PreferencesError::new("preferences contain an empty line"))?;
            let value = parts
                .next()
                .ok_or_else(|| PreferencesError::new("a preference value is missing"))?;
            if parts.next().is_some() {
                return Err(PreferencesError::new(
                    "a preference line has unexpected trailing data",
                ));
            }
            match key {
                "volume_percent" => set_once(
                    &mut volume_percent,
                    value
                        .parse::<u8>()
                        .ok()
                        .filter(|value| *value <= 100)
                        .ok_or_else(|| {
                            PreferencesError::new("volume_percent must be between 0 and 100")
                        })?,
                )?,
                "muted" => set_once(
                    &mut muted,
                    match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(PreferencesError::new("muted must be true or false")),
                    },
                )?,
                "era" => set_once(
                    &mut era,
                    match value {
                        "phosphor" => Era::Phosphor,
                        "8-bit" => Era::EightBit,
                        "vector" => Era::Vector,
                        "modern" => Era::Modern,
                        _ => return Err(PreferencesError::new("era is not recognized")),
                    },
                )?,
                "window_mode" => set_once(
                    &mut window_mode,
                    WindowModePreference::parse(value)
                        .ok_or_else(|| PreferencesError::new("window_mode is not recognized"))?,
                )?,
                _ => {
                    return Err(PreferencesError::new(
                        "preferences contain an unknown field",
                    ));
                }
            }
        }

        Ok(Self {
            volume_percent: required(volume_percent, "volume_percent is missing")?,
            muted: required(muted, "muted is missing")?,
            era: required(era, "era is missing")?,
            window_mode: required(window_mode, "window_mode is missing")?,
        })
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), PreferencesError> {
    if slot.replace(value).is_some() {
        return Err(PreferencesError::new(
            "preferences contain a duplicate field",
        ));
    }
    Ok(())
}

fn required<T>(value: Option<T>, message: &'static str) -> Result<T, PreferencesError> {
    value.ok_or_else(|| PreferencesError::new(message))
}

/// A preferences document could not be parsed safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferencesError(String);

impl PreferencesError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for PreferencesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PreferencesError {}

#[cfg(test)]
mod tests {
    use super::{AppPreferences, PREFERENCES_SCHEMA_VERSION, WindowModePreference};
    use crate::Era;

    #[test]
    fn current_preferences_round_trip_in_stable_order() {
        let preferences = AppPreferences {
            volume_percent: 70,
            muted: true,
            era: Era::Vector,
            window_mode: WindowModePreference::Exclusive,
        };
        let text = preferences.to_text();
        assert_eq!(
            text,
            format!(
                "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent 70\nmuted true\nera vector\nwindow_mode exclusive\n"
            )
        );
        assert_eq!(AppPreferences::try_from_text(&text), Ok(preferences));
    }

    #[test]
    fn malformed_preferences_never_apply_partially() {
        for text in [
            "",
            "NUMINOUS_PREFERENCES 2\nvolume_percent 45\nmuted false\nera modern\nwindow_mode windowed\n",
            "NUMINOUS_PREFERENCES 1\nvolume_percent 101\nmuted false\nera modern\nwindow_mode windowed\n",
            "NUMINOUS_PREFERENCES 1\nvolume_percent 45\nmuted maybe\nera modern\nwindow_mode windowed\n",
            "NUMINOUS_PREFERENCES 1\nvolume_percent 45\nmuted false\nera modern\n",
            "NUMINOUS_PREFERENCES 1\nvolume_percent 45\nvolume_percent 50\nmuted false\nera modern\nwindow_mode windowed\n",
            "NUMINOUS_PREFERENCES 1\nvolume_percent 45\nmuted false\nera modern\nwindow_mode windowed\nsurprise yes\n",
        ] {
            assert!(AppPreferences::try_from_text(text).is_err(), "{text:?}");
        }
    }
}
