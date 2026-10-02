//! Versioned App preferences shared with local-state persistence.

use std::fmt;

use crate::{Era, study::StudyLocale};

/// Current on-disk preferences schema.
pub const PREFERENCES_SCHEMA_VERSION: u8 = 3;

/// Every schema this build reads, oldest first.
///
/// Older documents stay readable so an upgrade never discards a player's
/// settings. A field a schema did not have takes its default, and a field it
/// did not have is rejected if it appears, so a damaged or hand-edited file
/// cannot pass as a newer one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Schema {
    /// Master volume, mute, Visual Era, and window mode.
    V1,
    /// Adds the study language.
    V2,
    /// Adds the music, room, and effect levels and the text scale.
    V3,
}

impl Schema {
    fn from_header(header: &str) -> Option<Self> {
        match header {
            "NUMINOUS_PREFERENCES 1" => Some(Self::V1),
            "NUMINOUS_PREFERENCES 2" => Some(Self::V2),
            "NUMINOUS_PREFERENCES 3" => Some(Self::V3),
            _ => None,
        }
    }
}

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

/// The player's text size, as a percentage of the size the window chooses.
///
/// Exactly five sizes exist: 100, 125, 150, 175, and 200 percent. Text never
/// shrinks below the viewport's own choice, and each step is large enough to be
/// a visible change in whole-pixel type, so any other value is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextScale(u8);

impl TextScale {
    /// The window's own text size, unchanged.
    pub const BASE: Self = Self(100);
    /// The largest admitted text size.
    pub const LARGEST: Self = Self(200);
    /// Percentage points between adjacent sizes.
    pub const STEP_PERCENT: u8 = 25;

    /// The admitted size at `percent`, or `None` for any other value.
    #[must_use]
    pub const fn from_percent(percent: u8) -> Option<Self> {
        if percent >= Self::BASE.0
            && percent <= Self::LARGEST.0
            && percent.is_multiple_of(Self::STEP_PERCENT)
        {
            Some(Self(percent))
        } else {
            None
        }
    }

    /// The size as a percentage of the window's own choice.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }

    /// One step larger, holding at [`TextScale::LARGEST`].
    #[must_use]
    pub const fn larger(self) -> Self {
        if self.0 >= Self::LARGEST.0 {
            self
        } else {
            Self(self.0 + Self::STEP_PERCENT)
        }
    }

    /// One step smaller, holding at [`TextScale::BASE`].
    #[must_use]
    pub const fn smaller(self) -> Self {
        if self.0 <= Self::BASE.0 {
            self
        } else {
            Self(self.0 - Self::STEP_PERCENT)
        }
    }
}

impl Default for TextScale {
    fn default() -> Self {
        Self::BASE
    }
}

/// Player-selected App preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPreferences {
    /// Master volume as an exact percentage.
    pub volume_percent: u8,
    /// Recorded music, the radio, as a percentage beneath the master level.
    pub music_volume_percent: u8,
    /// Room sound as a percentage beneath the master level: the room score,
    /// its mathematical voices and events, Studio, and Watch Agent replay.
    pub room_volume_percent: u8,
    /// Game cues as a percentage beneath the master level.
    pub effect_volume_percent: u8,
    /// Whether all App sound is muted.
    pub muted: bool,
    /// Visual treatment applied to the rendered room.
    pub era: Era,
    /// Window presentation requested for the next launch.
    pub window_mode: WindowModePreference,
    /// Requested language for optional room study, separate from shell translation.
    pub study_locale: StudyLocale,
    /// Player text size.
    pub text_scale: TextScale,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            volume_percent: 45,
            music_volume_percent: 100,
            room_volume_percent: 100,
            effect_volume_percent: 100,
            muted: false,
            era: Era::Modern,
            window_mode: WindowModePreference::Windowed,
            study_locale: StudyLocale::default(),
            text_scale: TextScale::BASE,
        }
    }
}

impl AppPreferences {
    /// Serialize the complete current schema in stable key order.
    #[must_use]
    pub fn to_text(&self) -> String {
        let era = match self.era {
            Era::Phosphor => "phosphor",
            Era::EightBit => "8-bit",
            Era::Vector => "vector",
            Era::Modern => "modern",
        };
        format!(
            "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent {}\nmusic_volume_percent {}\nroom_volume_percent {}\neffect_volume_percent {}\nmuted {}\nera {era}\nwindow_mode {}\nstudy_locale {}\ntext_scale_percent {}\n",
            self.volume_percent,
            self.music_volume_percent,
            self.room_volume_percent,
            self.effect_volume_percent,
            self.muted,
            self.window_mode.name(),
            self.study_locale,
            self.text_scale.percent(),
        )
    }

    /// Parse one complete preferences document.
    ///
    /// Unknown, duplicate, missing, or out-of-range fields are rejected as one
    /// unit so a damaged file cannot apply a surprising partial configuration.
    /// Schemas 1 and 2 stay readable: schema 1 defaults study to English, and
    /// both default the music, room, and effect levels to 100 percent and the
    /// text scale to its base, so an upgraded install sounds and reads exactly
    /// as it did.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error when the schema or any field is invalid.
    pub fn try_from_text(text: &str) -> Result<Self, PreferencesError> {
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| PreferencesError::new("preferences file is empty"))?;
        let schema = Schema::from_header(header)
            .ok_or_else(|| PreferencesError::new("preferences schema is missing or unsupported"))?;

        let mut volume_percent = None;
        let mut music_volume_percent = None;
        let mut room_volume_percent = None;
        let mut effect_volume_percent = None;
        let mut muted = None;
        let mut era = None;
        let mut window_mode = None;
        let mut study_locale = None;
        let mut text_scale = None;
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
                    parse_percent(value, "volume_percent must be between 0 and 100")?,
                )?,
                "music_volume_percent" if schema >= Schema::V3 => set_once(
                    &mut music_volume_percent,
                    parse_percent(value, "music_volume_percent must be between 0 and 100")?,
                )?,
                "room_volume_percent" if schema >= Schema::V3 => set_once(
                    &mut room_volume_percent,
                    parse_percent(value, "room_volume_percent must be between 0 and 100")?,
                )?,
                "effect_volume_percent" if schema >= Schema::V3 => set_once(
                    &mut effect_volume_percent,
                    parse_percent(value, "effect_volume_percent must be between 0 and 100")?,
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
                "study_locale" if schema >= Schema::V2 => set_once(
                    &mut study_locale,
                    StudyLocale::parse(value)
                        .map_err(|error| PreferencesError::new(error.to_string()))?,
                )?,
                "text_scale_percent" if schema >= Schema::V3 => set_once(
                    &mut text_scale,
                    value
                        .parse::<u8>()
                        .ok()
                        .and_then(TextScale::from_percent)
                        .ok_or_else(|| {
                            PreferencesError::new(
                                "text_scale_percent must be 100, 125, 150, 175, or 200",
                            )
                        })?,
                )?,
                _ => {
                    return Err(PreferencesError::new(
                        "preferences contain an unknown field",
                    ));
                }
            }
        }

        let current = schema >= Schema::V3;
        let full = |value: Option<u8>, message| {
            if current {
                required(value, message)
            } else {
                Ok(100)
            }
        };
        Ok(Self {
            volume_percent: required(volume_percent, "volume_percent is missing")?,
            music_volume_percent: full(music_volume_percent, "music_volume_percent is missing")?,
            room_volume_percent: full(room_volume_percent, "room_volume_percent is missing")?,
            effect_volume_percent: full(effect_volume_percent, "effect_volume_percent is missing")?,
            muted: required(muted, "muted is missing")?,
            era: required(era, "era is missing")?,
            window_mode: required(window_mode, "window_mode is missing")?,
            study_locale: if schema >= Schema::V2 {
                required(study_locale, "study_locale is missing")?
            } else {
                StudyLocale::default()
            },
            text_scale: if current {
                required(text_scale, "text_scale_percent is missing")?
            } else {
                TextScale::BASE
            },
        })
    }
}

fn parse_percent(value: &str, message: &'static str) -> Result<u8, PreferencesError> {
    value
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 100)
        .ok_or_else(|| PreferencesError::new(message))
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
    use super::{
        AppPreferences, PREFERENCES_SCHEMA_VERSION, Schema, TextScale, WindowModePreference,
    };
    use crate::Era;

    const CURRENT_PREFIX: &str = "NUMINOUS_PREFERENCES 3\nvolume_percent 45\nmusic_volume_percent 100\nroom_volume_percent 100\neffect_volume_percent 100\nmuted false\nera modern\nwindow_mode windowed\nstudy_locale en\n";

    #[test]
    fn current_preferences_round_trip_in_stable_order() {
        let preferences = AppPreferences {
            volume_percent: 70,
            music_volume_percent: 30,
            room_volume_percent: 0,
            effect_volume_percent: 85,
            muted: true,
            era: Era::Vector,
            window_mode: WindowModePreference::Exclusive,
            study_locale: "ja-JP".parse().unwrap(),
            text_scale: TextScale::from_percent(175).unwrap(),
        };
        let text = preferences.to_text();
        assert_eq!(
            text,
            format!(
                "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent 70\nmusic_volume_percent 30\nroom_volume_percent 0\neffect_volume_percent 85\nmuted true\nera vector\nwindow_mode exclusive\nstudy_locale ja-jp\ntext_scale_percent 175\n"
            )
        );
        assert_eq!(AppPreferences::try_from_text(&text), Ok(preferences));
    }

    #[test]
    fn the_written_header_is_the_newest_schema_this_build_reads() {
        let text = AppPreferences::default().to_text();
        let header = text.lines().next().expect("header");
        assert_eq!(Schema::from_header(header), Some(Schema::V3));
        assert_eq!(
            AppPreferences::try_from_text(&text),
            Ok(AppPreferences::default())
        );
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
            "NUMINOUS_PREFERENCES 4\nvolume_percent 45\nmuted false\nera modern\nwindow_mode windowed\n",
        ] {
            assert!(AppPreferences::try_from_text(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn the_current_schema_requires_and_bounds_every_level_and_the_text_scale() {
        let complete = format!("{CURRENT_PREFIX}text_scale_percent 100\n");
        assert_eq!(
            AppPreferences::try_from_text(&complete),
            Ok(AppPreferences::default())
        );
        for key in [
            "music_volume_percent",
            "room_volume_percent",
            "effect_volume_percent",
            "text_scale_percent",
        ] {
            let missing = complete
                .lines()
                .filter(|line| !line.starts_with(key))
                .map(|line| format!("{line}\n"))
                .collect::<String>();
            assert!(
                AppPreferences::try_from_text(&missing).is_err(),
                "{key} must be required"
            );
            let duplicated = format!("{complete}{key} 100\n");
            assert!(
                AppPreferences::try_from_text(&duplicated).is_err(),
                "{key} must not repeat"
            );
        }
        for level in ["101", "255", "-1", "50.5", "loud"] {
            for key in [
                "music_volume_percent",
                "room_volume_percent",
                "effect_volume_percent",
            ] {
                let text = complete.replace(&format!("{key} 100"), &format!("{key} {level}"));
                assert!(
                    AppPreferences::try_from_text(&text).is_err(),
                    "{key} {level} must be rejected"
                );
            }
        }
        for scale in ["0", "99", "110", "225", "250", "big"] {
            let text = complete.replace(
                "text_scale_percent 100",
                &format!("text_scale_percent {scale}"),
            );
            assert!(
                AppPreferences::try_from_text(&text).is_err(),
                "text scale {scale} must be rejected"
            );
        }
        let edges = complete
            .replace("music_volume_percent 100", "music_volume_percent 0")
            .replace("text_scale_percent 100", "text_scale_percent 200");
        let parsed = AppPreferences::try_from_text(&edges).expect("edge values are legal");
        assert_eq!(parsed.music_volume_percent, 0);
        assert_eq!(parsed.text_scale, TextScale::LARGEST);
    }

    #[test]
    fn legacy_preferences_keep_every_existing_setting_and_default_study_language() {
        let text = "NUMINOUS_PREFERENCES 1\nvolume_percent 17\nmuted true\nera phosphor\nwindow_mode borderless\n";
        let preferences = AppPreferences::try_from_text(text).unwrap();
        assert_eq!(preferences.volume_percent, 17);
        assert!(preferences.muted);
        assert_eq!(preferences.era, Era::Phosphor);
        assert_eq!(preferences.window_mode, WindowModePreference::Borderless);
        assert_eq!(preferences.study_locale.as_str(), "en");
        assert_eq!(
            AppPreferences::try_from_text(&preferences.to_text()),
            Ok(preferences)
        );
        assert!(AppPreferences::try_from_text(&format!("{text}study_locale ja\n")).is_err());
    }

    #[test]
    fn legacy_schemas_upgrade_to_full_levels_and_base_text_without_admitting_new_fields() {
        let version_one = "NUMINOUS_PREFERENCES 1\nvolume_percent 17\nmuted true\nera phosphor\nwindow_mode borderless\n";
        let version_two = format!(
            "{}study_locale ja\n",
            version_one.replace("PREFERENCES 1", "PREFERENCES 2")
        );
        for legacy in [version_one.to_string(), version_two] {
            let upgraded = AppPreferences::try_from_text(&legacy).expect("legacy schema");
            assert_eq!(upgraded.volume_percent, 17, "master level survives");
            assert_eq!(upgraded.music_volume_percent, 100);
            assert_eq!(upgraded.room_volume_percent, 100);
            assert_eq!(upgraded.effect_volume_percent, 100);
            assert_eq!(upgraded.text_scale, TextScale::BASE);
            let rewritten = upgraded.to_text();
            assert!(rewritten.starts_with("NUMINOUS_PREFERENCES 3\n"));
            assert_eq!(AppPreferences::try_from_text(&rewritten), Ok(upgraded));
            for newer in [
                "music_volume_percent 50\n",
                "room_volume_percent 50\n",
                "effect_volume_percent 50\n",
                "text_scale_percent 150\n",
            ] {
                assert!(
                    AppPreferences::try_from_text(&format!("{legacy}{newer}")).is_err(),
                    "an older schema must not carry {newer:?}"
                );
            }
        }
    }

    #[test]
    fn study_language_is_bounded_validated_and_required_in_the_current_schema() {
        let prefix = "NUMINOUS_PREFERENCES 2\nvolume_percent 17\nmuted true\nera phosphor\nwindow_mode borderless\n";
        for language in ["haw", "tlh", "zz-unknown", "en-US"] {
            let parsed =
                AppPreferences::try_from_text(&format!("{prefix}study_locale {language}\n"))
                    .unwrap();
            assert_eq!(parsed.study_locale.as_str(), language.to_ascii_lowercase());
            assert_eq!(parsed.volume_percent, 17);
        }
        for suffix in [
            "",
            "study_locale \n",
            "study_locale ja_JP\n",
            "study_locale ja\nstudy_locale en\n",
            "study_locale ja extra\n",
        ] {
            assert!(AppPreferences::try_from_text(&format!("{prefix}{suffix}")).is_err());
        }
        let without_study =
            format!("{CURRENT_PREFIX}text_scale_percent 100\n").replace("study_locale en\n", "");
        assert!(AppPreferences::try_from_text(&without_study).is_err());
    }

    #[test]
    fn text_scale_admits_exactly_five_sizes_and_steps_between_them() {
        let admitted = (0..=u8::MAX)
            .filter_map(TextScale::from_percent)
            .map(TextScale::percent)
            .collect::<Vec<_>>();
        assert_eq!(admitted, [100, 125, 150, 175, 200]);
        assert_eq!(TextScale::default(), TextScale::BASE);

        let mut climbed = vec![TextScale::BASE.percent()];
        let mut scale = TextScale::BASE;
        for _ in 0..6 {
            scale = scale.larger();
            climbed.push(scale.percent());
        }
        assert_eq!(climbed, [100, 125, 150, 175, 200, 200, 200]);
        for _ in 0..6 {
            scale = scale.smaller();
        }
        assert_eq!(scale, TextScale::BASE, "the base is the floor");
        assert_eq!(TextScale::BASE.smaller(), TextScale::BASE);
        assert_eq!(TextScale::LARGEST.larger(), TextScale::LARGEST);
    }
}
