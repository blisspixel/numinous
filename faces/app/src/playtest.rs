use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use numinous_core::{Journey, Room};

pub(crate) struct PlaytestSnapshot<'a> {
    pub(crate) room: &'a dyn Room,
    pub(crate) journey: &'a Journey,
    pub(crate) room_count: usize,
    pub(crate) phase: f64,
    pub(crate) variation: u64,
    pub(crate) visual_era: &'a str,
    pub(crate) sound_on: bool,
    pub(crate) time_scale: f64,
    pub(crate) poke_points: &'a [(f64, f64)],
    pub(crate) active_mode: &'a str,
}

pub(crate) fn default_log_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("logs"), |root| root.join("logs"))
}

pub(crate) fn build_report(snapshot: &PlaytestSnapshot<'_>, now: SystemTime) -> String {
    let meta = snapshot.room.meta();
    let unix_seconds = unix_seconds(now);
    let phase = if snapshot.phase.is_finite() {
        snapshot.phase.rem_euclid(1.0)
    } else {
        0.0
    };
    let time_scale = if snapshot.time_scale.is_finite() {
        snapshot.time_scale.clamp(0.0, 16.0)
    } else {
        1.0
    };
    let poke_points = format_poke_points(snapshot.poke_points);
    let sound = if snapshot.sound_on { "on" } else { "off" };
    let action = numinous_core::room_touch_action(snapshot.room);
    let mut report = String::new();

    let _ = writeln!(report, "# Numinous Playtest Note");
    let _ = writeln!(report);
    let _ = writeln!(report, "Saved at Unix seconds: {unix_seconds}");
    let _ = writeln!(report);
    let _ = writeln!(report, "## Session Snapshot");
    let _ = writeln!(report);
    let _ = writeln!(report, "- Room: {} (`{}`)", meta.title, meta.id);
    let _ = writeln!(report, "- Wing: {}", meta.wing);
    let _ = writeln!(report, "- Action hint: {action}");
    let _ = writeln!(report, "- Mode: {}", snapshot.active_mode);
    let _ = writeln!(report, "- Phase: {phase:.3}");
    let _ = writeln!(report, "- Variation: {}", snapshot.variation);
    let _ = writeln!(report, "- Visual era: {}", snapshot.visual_era);
    let _ = writeln!(report, "- Sound: {sound}");
    let _ = writeln!(report, "- Time scale: {time_scale:.2}x");
    let _ = writeln!(
        report,
        "- Poke trail: {} point(s)",
        snapshot.poke_points.len()
    );
    let _ = writeln!(report, "- Poke points newest-last: {poke_points}");
    let _ = writeln!(
        report,
        "- Journey: level {}, {} XP, {} of {} rooms, {} play(s), {} win(s)",
        snapshot.journey.level(),
        snapshot.journey.sparks(),
        snapshot.journey.visited.len(),
        snapshot.room_count,
        snapshot.journey.plays,
        snapshot.journey.wins
    );
    let _ = writeln!(report);
    let _ = writeln!(report, "## Facilitator Prompts");
    let _ = writeln!(report);
    let _ = writeln!(
        report,
        "Record observations only. Do not record names, contact details, or sensitive personal data."
    );
    let _ = writeln!(report);
    let _ = writeln!(report, "- First unprompted action:");
    let _ = writeln!(report, "- First confusion:");
    let _ = writeln!(report, "- First unprompted whoa:");
    let _ = writeln!(
        report,
        "- First spontaneous share intent or ask-to-send, without recipient details:"
    );
    let _ = writeln!(report, "- First one-more-run moment:");
    let _ = writeln!(report, "- Where the fun was:");
    let _ = writeln!(report, "- Where the fun stopped:");
    let _ = writeln!(report, "- Room where attention dropped:");
    let _ = writeln!(report, "- Anything that felt like pressure or grind:");
    let _ = writeln!(report, "- What they learned or what surprised them:");
    let _ = writeln!(report, "- One change they would make first:");
    let _ = writeln!(
        report,
        "- Validated instrument: GEQ / FSS-2 / DFS-2 / GUESS / none recorded here:"
    );
    let _ = writeln!(
        report,
        "- Instrument score or external form/file reference:"
    );
    let _ = writeln!(report, "- Would they play again tomorrow:");

    report
}

fn format_poke_points(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return "none".to_string();
    }
    points
        .iter()
        .map(|&(x, y)| {
            let x = if x.is_finite() {
                x.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let y = if y.is_finite() {
                y.clamp(0.0, 1.0)
            } else {
                0.0
            };
            format!("({x:.3},{y:.3})")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn write_report(dir: &Path, now: SystemTime, report: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let stem = format!("playtest-{}", unix_seconds(now));
    for suffix in 0..1000 {
        let filename = if suffix == 0 {
            format!("{stem}.md")
        } else {
            format!("{stem}-{suffix:03}.md")
        };
        let path = dir.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(report.as_bytes())?;
                return Ok(path);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "too many playtest notes in one second",
    ))
}

fn unix_seconds(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn snapshot<'a>(room: &'a dyn Room, journey: &'a Journey) -> PlaytestSnapshot<'a> {
        PlaytestSnapshot {
            room,
            journey,
            room_count: 30,
            phase: 1.25,
            variation: 7,
            visual_era: "Vector",
            sound_on: false,
            time_scale: 2.0,
            poke_points: &[(0.25, 0.75)],
            active_mode: "wander",
        }
    }

    #[test]
    fn report_contains_session_snapshot_and_hallway_prompts() {
        let rooms = numinous_core::all_rooms_with(0);
        let mut journey = Journey::default();
        journey.visit(rooms[0].meta().id);
        journey.play();
        let report = build_report(
            &snapshot(rooms[0].as_ref(), &journey),
            UNIX_EPOCH + Duration::from_secs(42),
        );

        assert!(report.contains("# Numinous Playtest Note"));
        assert!(report.contains("Saved at Unix seconds: 42"));
        assert!(report.contains("Room:"));
        assert!(report.contains("Action hint:"));
        assert!(report.contains("Poke points newest-last: (0.250,0.750)"));
        assert!(report.contains("level"));
        assert!(report.contains("Poke trail: 1 point(s)"));
        assert_lines_in_order(
            &report,
            &[
                "Record observations only. Do not record names, contact details, or sensitive personal data.",
                "- First unprompted action:",
                "- First confusion:",
                "- First unprompted whoa:",
                "- First spontaneous share intent or ask-to-send, without recipient details:",
                "- First one-more-run moment:",
                "- Where the fun was:",
                "- Where the fun stopped:",
                "- Room where attention dropped:",
                "- Anything that felt like pressure or grind:",
                "- What they learned or what surprised them:",
                "- One change they would make first:",
                "- Validated instrument: GEQ / FSS-2 / DFS-2 / GUESS / none recorded here:",
                "- Instrument score or external form/file reference:",
                "- Would they play again tomorrow:",
            ],
        );
        assert!(report.contains("Where the fun stopped"));
        assert!(report.contains("One change they would make first"));
    }

    fn assert_lines_in_order(report: &str, expected: &[&str]) {
        let mut cursor = 0;
        for line in expected {
            let rest = &report[cursor..];
            let Some(offset) = rest.find(line) else {
                panic!("missing report line: {line}");
            };
            cursor += offset + line.len();
        }
    }

    #[test]
    fn poke_points_are_replayable_and_sanitized() {
        let report = format_poke_points(&[(0.2, 0.4), (1.5, f64::NAN)]);

        assert_eq!(report, "(0.200,0.400) (1.000,0.000)");
    }

    #[test]
    fn writer_uses_logs_style_directory_and_unique_filenames() {
        let dir = std::env::temp_dir().join("numinous_playtest_writer");
        let _ = fs::remove_dir_all(&dir);
        let now = UNIX_EPOCH + Duration::from_secs(123);

        let first = write_report(&dir, now, "first").expect("first write");
        let second = write_report(&dir, now, "second").expect("unique write");

        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("playtest-123.md")
        );
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("playtest-123-001.md")
        );
        assert_eq!(fs::read_to_string(first).expect("first report"), "first");
        assert_eq!(fs::read_to_string(second).expect("second report"), "second");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn default_directory_is_repo_root_logs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        assert_eq!(default_log_dir(), root.join("logs"));
    }
}
