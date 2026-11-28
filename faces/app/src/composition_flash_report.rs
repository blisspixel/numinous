//! Paired CPU compositor captures. Freeze the same scene and its chrome for
//! every trial so the meter, the jolt and their interaction can be isolated.

use super::{headless, select_times_tables};
use crate::{App, play};
use numinous_audio::VisualizerSource;
use numinous_core::photosensitivity::{MAX_FLASHES_PER_SECOND, regional::RegionalFlashCapture};
use numinous_core::{BAND_COUNT, Era, Motion, Raster, RoomInput, Surface};
use std::fmt::Write;
use std::path::Path;
use std::time::{Duration, Instant};

fn held_room(app: &mut App, width: usize, height: usize) -> Raster {
    select_times_tables(app);
    app.t = 0.42;
    app.room_card = 0;
    app.inputs.push(RoomInput::PointerDown {
        x: 0.73,
        y: 0.58,
        t: app.t,
    });
    let inputs = app.presented_room_inputs();
    app.cpu_room_frame(width, height, &inputs)
}

fn graded_board(app: &mut App, width: usize, height: usize) -> (Raster, u8) {
    let seed = 7;
    let (round, board) = play::deal_munch(seed, 0, None);
    let wrong = board
        .numbers
        .iter()
        .position(|&value| !board.rule.fits(value))
        .expect("seeded board contains a wrong bite");
    app.munch = Some(play::MunchPlay {
        board,
        seed,
        round,
        cursor: wrong,
        bites: [wrong].into(),
        graded: None,
        bite_flash: None,
    });
    app.munch_grade();
    let jolt = std::mem::take(&mut app.screen_shake);
    assert!(jolt > 0);
    (
        app.modal_frame(width, height).expect("graded Munch board"),
        jolt,
    )
}

struct Trial {
    changed_frames: usize,
    max_changed_pixels: usize,
    transitions: usize,
    rates: [usize; 4],
}

fn capture(
    app: &mut App,
    base: &Raster,
    control: &[u8],
    meter: bool,
    jolt: u8,
    fps: usize,
) -> Trial {
    let (width, height) = (base.width(), base.height());
    let mut regional = RegionalFlashCapture::new(width, height).unwrap();
    let start = Instant::now();
    let mut changed_frames = 0;
    let mut max_changed_pixels = 0;
    let mut previous = None;
    let mut transitions = 0;
    // The first frame is the control. One second of alternating normalized
    // band extrema stresses the meter independently of audio acquisition.
    for tick in 0..=fps {
        if tick == 1 {
            app.screen_shake = jolt;
        }
        let bands = (meter && tick > 0)
            .then_some(([(tick % 2) as f32; BAND_COUNT], VisualizerSource::RoomBed));
        let now = start + Duration::from_secs_f64(tick as f64 / fps as f64);
        let (rgba, _, _) = app.compose_frame_with_bands(base.clone(), width, height, bands, now);
        let changed = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(control.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        changed_frames += usize::from(changed > 0);
        max_changed_pixels = max_changed_pixels.max(changed);
        assert!(regional.push(&rgba));
        transitions += usize::from(previous.as_ref().is_some_and(|last| *last != rgba));
        previous = Some(rgba);
    }
    assert_eq!(app.screen_shake, 0);
    let peaks = regional.timeline_peaks(fps).expect("one complete second");
    Trial {
        changed_frames,
        max_changed_pixels,
        transitions,
        rates: [
            peaks.whole_general,
            peaks.whole_red,
            peaks.general.rate,
            peaks.red.rate,
        ],
    }
}

fn measure(width: usize, height: usize, fps: usize, eras: &[Era]) -> (String, Vec<String>) {
    let mut csv = String::from(
        "scene,era,motion,width,height,fps,meter,jolt_frames,changed_frames,max_changed_pixels,whole_general,whole_red,regional_general,regional_red\n",
    );
    let mut over_budget = Vec::new();
    for (scene, motion) in [
        ("times-tables-held", Motion::Full),
        ("munch-bad-grade", Motion::Full),
        ("munch-bad-grade", Motion::Reduced),
    ] {
        let mut app = headless("composition-flash");
        app.close_menu();
        app.banner = None;
        app.muted = false;
        app.motion = motion;
        let (base, jolt) = if scene == "times-tables-held" {
            (held_room(&mut app, width, height), 0)
        } else {
            graded_board(&mut app, width, height)
        };
        assert!(usize::from(jolt) < fps, "capture must outlast the jolt");
        for &era in eras {
            app.era = era;
            let (control, _, _) =
                app.compose_frame_with_bands(base.clone(), width, height, None, Instant::now());
            for shake in std::iter::once(0).chain((jolt > 0).then_some(jolt)) {
                for meter in [false, true] {
                    let trial = capture(&mut app, &base, &control, meter, shake, fps);
                    let [wg, wr, rg, rr] = trial.rates;
                    let row = format!(
                        "{scene},{},{motion:?},{width},{height},{fps},{meter},{shake},{},{},{wg},{wr},{rg},{rr}",
                        era.name(),
                        trial.changed_frames,
                        trial.max_changed_pixels
                    );
                    writeln!(csv, "{row}").unwrap();
                    if trial
                        .rates
                        .iter()
                        .any(|&rate| rate as f64 > MAX_FLASHES_PER_SECOND)
                    {
                        over_budget.push(row.clone());
                    }
                    let expected_frames = if meter { fps } else { usize::from(shake) };
                    assert_eq!(trial.changed_frames, expected_frames, "{row}");
                    // A moving board alternates positions; a still edge only
                    // appears and disappears. The meter changes every tick.
                    let expected_transitions = if meter {
                        fps
                    } else if shake == 0 {
                        0
                    } else if motion.animates() {
                        usize::from(shake) + 1
                    } else {
                        2
                    };
                    assert_eq!(trial.transitions, expected_transitions, "{row}");
                }
            }
        }
    }
    (csv, over_budget)
}

#[test]
fn room_meter_and_munch_jolt_change_only_their_expected_frames() {
    // One era and a shorter sample sequence keep the ordinary check small.
    // The sequence includes both the first and last frame of a real grade jolt.
    let (csv, over_budget) = measure(160, 96, 16, &[Era::Modern]);
    assert_eq!(csv.lines().skip(1).count(), 10);
    assert!(
        over_budget.is_empty(),
        "over budget:\n{}",
        over_budget.join("\n")
    );
}

#[test]
#[ignore = "paired CPU room and game compositor flash captures; run nightly"]
fn app_composition_flash_report() {
    let (csv, over_budget) = measure(240, 140, 60, &Era::ALL);
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/qol");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("app-composition-flash.csv"), csv).unwrap();
    assert!(
        over_budget.is_empty(),
        "over budget:\n{}",
        over_budget.join("\n")
    );
}
