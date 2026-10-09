//! Versioned App preferences shared with local-state persistence.

use std::fmt;

use crate::{Era, study::StudyLocale};

/// Current on-disk preferences schema.
pub const PREFERENCES_SCHEMA_VERSION: u8 = 5;

/// Where one read-only App preference snapshot came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesSource {
    /// No saved file exists, so the App uses its defaults.
    Defaults,
    /// A complete supported saved document was read successfully.
    Saved,
}

impl PreferencesSource {
    /// Stable name for a preferences inspection result.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Defaults => "defaults",
            Self::Saved => "saved",
        }
    }
}

/// App launch preferences and their read provenance, without device state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPreferencesSnapshot {
    /// The same admitted values the App uses on launch.
    pub preferences: AppPreferences,
    /// Whether those values came from a saved document or first-run defaults.
    pub source: PreferencesSource,
}

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
    /// Adds the music, room, and effect levels.
    V3,
    /// Adds the study reader's body text size.
    V4,
    /// Adds Cabinet and room HUD text size.
    V5,
}

impl Schema {
    fn from_header(header: &str) -> Option<Self> {
        match header {
            "NUMINOUS_PREFERENCES 1" => Some(Self::V1),
            "NUMINOUS_PREFERENCES 2" => Some(Self::V2),
            "NUMINOUS_PREFERENCES 3" => Some(Self::V3),
            "NUMINOUS_PREFERENCES 4" => Some(Self::V4),
            "NUMINOUS_PREFERENCES 5" => Some(Self::V5),
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

/// A saved text size of 100, 125, or 150 percent.
///
/// Study body text applies it to the reader's responsive point size. Cabinet
/// menus, room HUD lettering, and the shared pause, banner, and journey
/// overlays apply it to their window pixel scale, then fit so controls stay
/// inside the window. One hundred percent is that window's existing scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudyTextSize {
    /// The surface's own responsive size.
    #[default]
    Standard,
    /// One quarter larger than the responsive size.
    Large,
    /// One half larger than the responsive size.
    ExtraLarge,
}

impl StudyTextSize {
    /// Every admitted size, smallest first.
    pub const ALL: [Self; 3] = [Self::Standard, Self::Large, Self::ExtraLarge];

    /// Exact percentage relative to the reader's responsive body size.
    #[must_use]
    pub const fn percent(self) -> u8 {
        match self {
            Self::Standard => 100,
            Self::Large => 125,
            Self::ExtraLarge => 150,
        }
    }

    /// Stable on-disk percentage.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Standard => "100",
            Self::Large => "125",
            Self::ExtraLarge => "150",
        }
    }

    /// Whole-pixel scale after this percentage.
    ///
    /// One hundred percent returns `base` unchanged. A larger percentage rounds
    /// half up and is at least one step above a positive `base`, so the choice
    /// can show on a one-pixel face. A non-positive base is treated as one.
    /// Callers that must keep controls inside the window still reduce the result.
    #[must_use]
    pub const fn pixel_scale(self, base: i32) -> i32 {
        let base = if base < 1 { 1 } else { base };
        let percent = self.percent() as i32;
        if percent <= 100 {
            return base;
        }
        let rounded = base.saturating_mul(percent).saturating_add(50) / 100;
        if rounded > base {
            rounded
        } else {
            base.saturating_add(1)
        }
    }

    /// Step toward a larger or smaller size, staying at the ends.
    #[must_use]
    pub fn stepped(self, larger: bool) -> Self {
        match (self, larger) {
            (Self::Standard, true) | (Self::ExtraLarge, false) => Self::Large,
            (Self::Large, true) => Self::ExtraLarge,
            (Self::Large, false) => Self::Standard,
            _ => self,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.name() == value)
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
    /// Body size in optional room study, independent of gameplay rendering.
    pub study_text_size: StudyTextSize,
    /// Cabinet menu and room HUD lettering, independent of the study body.
    pub interface_text_size: StudyTextSize,
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
            study_text_size: StudyTextSize::default(),
            interface_text_size: StudyTextSize::default(),
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
            "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent {}\nmusic_volume_percent {}\nroom_volume_percent {}\neffect_volume_percent {}\nmuted {}\nera {era}\nwindow_mode {}\nstudy_locale {}\nstudy_text_size {}\ninterface_text_size {}\n",
            self.volume_percent,
            self.music_volume_percent,
            self.room_volume_percent,
            self.effect_volume_percent,
            self.muted,
            self.window_mode.name(),
            self.study_locale,
            self.study_text_size.name(),
            self.interface_text_size.name(),
        )
    }

    /// Parse one complete preferences document.
    ///
    /// Unknown, duplicate, missing, or out-of-range fields are rejected as one
    /// unit so a damaged file cannot apply a surprising partial configuration.
    /// Schemas 1 and 2 stay readable: schema 1 defaults study to English, and
    /// both default the music, room, and effect levels to 100 percent, so an
    /// upgraded install keeps its existing source and master levels. Schemas
    /// before 4 retain the standard reading size. Schemas before 5 retain the
    /// standard Cabinet and HUD size.
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
        let mut study_text_size = None;
        let mut interface_text_size = None;
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
                "study_text_size" if schema >= Schema::V4 => set_once(
                    &mut study_text_size,
                    StudyTextSize::parse(value).ok_or_else(|| {
                        PreferencesError::new("study_text_size must be 100, 125, or 150")
                    })?,
                )?,
                "interface_text_size" if schema >= Schema::V5 => set_once(
                    &mut interface_text_size,
                    StudyTextSize::parse(value).ok_or_else(|| {
                        PreferencesError::new("interface_text_size must be 100, 125, or 150")
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
            study_text_size: if schema >= Schema::V4 {
                required(study_text_size, "study_text_size is missing")?
            } else {
                StudyTextSize::default()
            },
            interface_text_size: if schema >= Schema::V5 {
                required(interface_text_size, "interface_text_size is missing")?
            } else {
                StudyTextSize::default()
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
        AppPreferences, PREFERENCES_SCHEMA_VERSION, Schema, StudyTextSize, WindowModePreference,
    };
    use crate::Era;

    const CURRENT_PREFIX: &str = "NUMINOUS_PREFERENCES 5\nvolume_percent 45\nmusic_volume_percent 100\nroom_volume_percent 100\neffect_volume_percent 100\nmuted false\nera modern\nwindow_mode windowed\nstudy_locale en\nstudy_text_size 100\ninterface_text_size 100\n";

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
            study_text_size: StudyTextSize::ExtraLarge,
            interface_text_size: StudyTextSize::Large,
        };
        let text = preferences.to_text();
        assert_eq!(
            text,
            format!(
                "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\nvolume_percent 70\nmusic_volume_percent 30\nroom_volume_percent 0\neffect_volume_percent 85\nmuted true\nera vector\nwindow_mode exclusive\nstudy_locale ja-jp\nstudy_text_size 150\ninterface_text_size 125\n"
            )
        );
        assert_eq!(AppPreferences::try_from_text(&text), Ok(preferences));
    }

    #[test]
    fn the_written_header_is_the_newest_schema_this_build_reads() {
        let text = AppPreferences::default().to_text();
        let header = text.lines().next().expect("header");
        assert_eq!(Schema::from_header(header), Some(Schema::V5));
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
            "NUMINOUS_PREFERENCES 5\nvolume_percent 45\nmuted false\nera modern\nwindow_mode windowed\n",
        ] {
            assert!(AppPreferences::try_from_text(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn the_current_schema_requires_and_bounds_every_level() {
        let complete = CURRENT_PREFIX;
        assert_eq!(
            AppPreferences::try_from_text(complete),
            Ok(AppPreferences::default())
        );
        for key in [
            "music_volume_percent",
            "room_volume_percent",
            "effect_volume_percent",
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
        let edges = complete.replace("music_volume_percent 100", "music_volume_percent 0");
        let parsed = AppPreferences::try_from_text(&edges).expect("edge values are legal");
        assert_eq!(parsed.music_volume_percent, 0);
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
    fn legacy_schemas_upgrade_to_full_levels_without_admitting_new_fields() {
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
            let rewritten = upgraded.to_text();
            assert!(rewritten.starts_with(&format!(
                "NUMINOUS_PREFERENCES {PREFERENCES_SCHEMA_VERSION}\n"
            )));
            assert_eq!(AppPreferences::try_from_text(&rewritten), Ok(upgraded));
            for newer in [
                "music_volume_percent 50\n",
                "room_volume_percent 50\n",
                "effect_volume_percent 50\n",
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
        let without_study = CURRENT_PREFIX.replace("study_locale en\n", "");
        assert!(AppPreferences::try_from_text(&without_study).is_err());
    }

    #[test]
    fn reading_size_upgrades_preserve_language_and_every_audio_level() {
        let legacy = "NUMINOUS_PREFERENCES 3\nvolume_percent 17\nmusic_volume_percent 23\nroom_volume_percent 31\neffect_volume_percent 47\nmuted true\nera phosphor\nwindow_mode borderless\nstudy_locale ja\n";
        let upgraded = AppPreferences::try_from_text(legacy).unwrap();
        assert_eq!(upgraded.study_text_size, StudyTextSize::Standard);
        assert_eq!(upgraded.interface_text_size, StudyTextSize::Standard);
        assert_eq!(upgraded.study_locale.as_str(), "ja");
        assert_eq!(upgraded.volume_percent, 17);
        assert_eq!(upgraded.music_volume_percent, 23);
        assert_eq!(upgraded.room_volume_percent, 31);
        assert_eq!(upgraded.effect_volume_percent, 47);
        assert!(upgraded.muted);
        assert_eq!(upgraded.era, Era::Phosphor);
        assert_eq!(upgraded.window_mode, WindowModePreference::Borderless);
        assert_eq!(
            AppPreferences::try_from_text(&upgraded.to_text()),
            Ok(upgraded)
        );
        assert!(AppPreferences::try_from_text(&format!("{legacy}study_text_size 125\n")).is_err());
    }

    #[test]
    fn reading_size_is_required_strict_and_round_trips_at_every_admitted_size() {
        for size in StudyTextSize::ALL {
            let preferences = AppPreferences {
                study_text_size: size,
                ..AppPreferences::default()
            };
            assert_eq!(
                AppPreferences::try_from_text(&preferences.to_text()),
                Ok(preferences)
            );
        }
        let prefix = CURRENT_PREFIX.replace("study_text_size 100\n", "");
        for suffix in [
            "",
            "study_text_size 99\n",
            "study_text_size 200\n",
            "study_text_size 125.0\n",
            "study_text_size large\n",
            "study_text_size 125 extra\n",
            "study_text_size 100\nstudy_text_size 125\n",
        ] {
            assert!(
                AppPreferences::try_from_text(&format!("{prefix}{suffix}")).is_err(),
                "{suffix:?}"
            );
        }
        assert_eq!(
            StudyTextSize::Standard.stepped(false),
            StudyTextSize::Standard
        );
        assert_eq!(
            StudyTextSize::ExtraLarge.stepped(true),
            StudyTextSize::ExtraLarge
        );
        assert_eq!(StudyTextSize::Large.stepped(false), StudyTextSize::Standard);
        assert_eq!(
            StudyTextSize::Large.stepped(true),
            StudyTextSize::ExtraLarge
        );
    }

    #[test]
    fn cabinet_text_upgrades_from_reading_documents_and_rejects_a_field_they_did_not_have() {
        let legacy = "NUMINOUS_PREFERENCES 4\nvolume_percent 45\nmusic_volume_percent 100\nroom_volume_percent 100\neffect_volume_percent 100\nmuted false\nera modern\nwindow_mode windowed\nstudy_locale en\nstudy_text_size 125\n";
        let upgraded = AppPreferences::try_from_text(legacy).unwrap();
        assert_eq!(upgraded.study_text_size, StudyTextSize::Large);
        assert_eq!(upgraded.interface_text_size, StudyTextSize::Standard);
        assert_eq!(
            AppPreferences::try_from_text(&upgraded.to_text()),
            Ok(upgraded)
        );
        assert!(
            AppPreferences::try_from_text(&format!("{legacy}interface_text_size 150\n")).is_err()
        );
        let prefix = CURRENT_PREFIX.replace("interface_text_size 100\n", "");
        for suffix in [
            "",
            "interface_text_size 99\n",
            "interface_text_size 200\n",
            "interface_text_size large\n",
            "interface_text_size 100\ninterface_text_size 125\n",
        ] {
            assert!(
                AppPreferences::try_from_text(&format!("{prefix}{suffix}")).is_err(),
                "{suffix:?}"
            );
        }
    }

    #[test]
    fn pixel_scale_keeps_one_hundred_percent_exact_and_grows_the_larger_sizes() {
        for base in [1, 2, 4, 6, 8, 16] {
            assert_eq!(StudyTextSize::Standard.pixel_scale(base), base);
        }
        assert_eq!(StudyTextSize::Standard.pixel_scale(0), 1);
        assert_eq!(StudyTextSize::Large.pixel_scale(1), 2);
        assert_eq!(StudyTextSize::Large.pixel_scale(2), 3);
        assert_eq!(StudyTextSize::Large.pixel_scale(4), 5);
        assert_eq!(StudyTextSize::Large.pixel_scale(8), 10);
        assert_eq!(StudyTextSize::ExtraLarge.pixel_scale(1), 2);
        assert_eq!(StudyTextSize::ExtraLarge.pixel_scale(2), 3);
        assert_eq!(StudyTextSize::ExtraLarge.pixel_scale(4), 6);
        assert_eq!(StudyTextSize::ExtraLarge.pixel_scale(6), 9);
    }
}
