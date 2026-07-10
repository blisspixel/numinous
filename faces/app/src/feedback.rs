use std::io;
use std::path::PathBuf;

const LEVEL_UP_FRAMES: u64 = 300;
const PLAYTEST_NOTE_FRAMES: u64 = 240;
const RADIO_FRAMES: u64 = 180;
const FULLSCREEN_FRAMES: u64 = 120;
const VOLUME_FRAMES: u64 = 90;
const SOUND_DEVICE_FRAMES: u64 = 600;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Banner {
    lines: Vec<String>,
    frames_left: u64,
}

impl Banner {
    fn new(lines: Vec<String>, frames_left: u64) -> Self {
        Self { lines, frames_left }
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    #[cfg(test)]
    pub(crate) fn frames_left(&self) -> u64 {
        self.frames_left
    }

    pub(crate) fn tick(&mut self) -> bool {
        if self.frames_left == 0 {
            return false;
        }
        self.frames_left -= 1;
        self.frames_left > 0
    }
}

pub(crate) fn level_up(level: u32, boons_available: u32) -> Banner {
    let mut lines = vec![
        format!("LEVEL UP  LV {level}"),
        numinous_core::level_lore(level).to_uppercase(),
    ];
    if boons_available > 0 {
        lines.push("BOON BANKED: NUMINOUS CHOOSE".to_string());
    }
    Banner::new(lines, LEVEL_UP_FRAMES)
}

pub(crate) fn playtest_note(result: io::Result<PathBuf>) -> Banner {
    let lines = match result {
        Ok(path) => {
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("playtest-note")
                .to_ascii_uppercase();
            vec!["PLAYTEST NOTE SAVED".to_string(), label]
        }
        Err(error) => vec![
            "PLAYTEST NOTE FAILED".to_string(),
            format!("WRITE ERROR: {:?}", error.kind()).to_ascii_uppercase(),
        ],
    };
    Banner::new(lines, PLAYTEST_NOTE_FRAMES)
}

pub(crate) fn fullscreen(label: &str) -> Banner {
    Banner::new(vec![format!("FULLSCREEN {label}")], FULLSCREEN_FRAMES)
}

pub(crate) fn volume(volume: f32) -> Banner {
    Banner::new(
        vec![format!("VOLUME {:.0}%", volume.clamp(0.0, 1.0) * 100.0)],
        VOLUME_FRAMES,
    )
}

pub(crate) fn radio(station_name: &str, station_id: &str, track_count: usize) -> Banner {
    let lines = if track_count == 0 {
        vec![
            format!("RADIO: {station_name}"),
            "NO TRACKS CACHED YET".to_string(),
            format!(
                "IN A TERMINAL: NUMINOUS TUNE2 {}",
                station_id.to_uppercase()
            ),
        ]
    } else {
        vec![format!(
            "RADIO: {station_name}  ({track_count} ON ROTATION)"
        )]
    };
    Banner::new(lines, RADIO_FRAMES)
}

pub(crate) fn sound_device_unavailable(error: &str) -> Banner {
    Banner::new(
        vec!["SOUND DEVICE UNAVAILABLE".to_string(), error.to_uppercase()],
        SOUND_DEVICE_FRAMES,
    )
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use super::{fullscreen, level_up, playtest_note, volume};

    #[test]
    fn level_up_banner_names_lore_and_boons() {
        let banner = level_up(2, 1);

        assert_eq!(banner.lines()[0], "LEVEL UP  LV 2");
        assert!(!banner.lines()[1].is_empty());
        assert_eq!(banner.lines()[2], "BOON BANKED: NUMINOUS CHOOSE");
        assert_eq!(banner.frames_left(), 300);
    }

    #[test]
    fn playtest_note_banners_sanitize_to_file_name_or_error_kind() {
        let saved = playtest_note(Ok(PathBuf::from("logs/playtest-77.md")));
        assert_eq!(saved.lines()[0], "PLAYTEST NOTE SAVED");
        assert_eq!(saved.lines()[1], "PLAYTEST-77.MD");

        let failed = playtest_note(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "C:/Users/Alice/private",
        )));
        assert_eq!(failed.lines()[0], "PLAYTEST NOTE FAILED");
        assert_eq!(failed.lines()[1], "WRITE ERROR: PERMISSIONDENIED");
    }

    #[test]
    fn short_status_banners_have_stable_durations() {
        let full = fullscreen("BORDERLESS");
        let audio = volume(0.734);

        assert_eq!(full.lines(), ["FULLSCREEN BORDERLESS"].as_slice());
        assert_eq!(full.frames_left(), 120);
        assert_eq!(audio.lines()[0], "VOLUME 73%");
        assert_eq!(audio.frames_left(), 90);
    }

    #[test]
    fn radio_and_sound_banners_explain_local_state() {
        let empty = super::radio("Axiom FM", "axiom", 0);
        let ready = super::radio("Axiom FM", "axiom", 3);
        let sound = super::sound_device_unavailable("no device");

        assert_eq!(empty.lines()[0], "RADIO: Axiom FM");
        assert_eq!(empty.lines()[2], "IN A TERMINAL: NUMINOUS TUNE2 AXIOM");
        assert_eq!(empty.frames_left(), 180);
        assert_eq!(ready.lines()[0], "RADIO: Axiom FM  (3 ON ROTATION)");
        assert_eq!(sound.lines()[0], "SOUND DEVICE UNAVAILABLE");
        assert_eq!(sound.lines()[1], "NO DEVICE");
        assert_eq!(sound.frames_left(), 600);
    }

    #[test]
    fn tick_reports_whether_banner_should_remain_visible() {
        let mut banner = volume(1.0);

        for _ in 0..89 {
            assert!(banner.tick());
        }
        assert!(!banner.tick());
        assert!(!banner.tick());
    }
}
