//! Finite, seed-zero room response to a moving hand and synthetic spectrum
//! levers. The beat traces exercise the core legacy poke renderer, including
//! rooms whose App presentation does not route spectrum pokes to that renderer.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use super::{
    FLASH_REFERENCE, REGIONAL_FLASH_SPEEDS, flash_overage, measure_rooms, visit_in_each_era,
};
use crate::photosensitivity::regional::{RegionalFlashCapture, TimelineFlashPeaks};
use crate::spectrum::{SpectrumLevers, plant_spectrum_poke};
use crate::{Era, Room, RoomInput, RoomMeta};

const FPS: usize = crate::APP_FRAMES_PER_SECOND as usize;
const FRAMES: usize = 4 * FPS + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trace {
    Drag,
    DenseUnspaced,
    DenseSpaced,
    MusicalSpaced,
}

impl Trace {
    const ALL: [Self; 4] = [
        Self::Drag,
        Self::DenseUnspaced,
        Self::DenseSpaced,
        Self::MusicalSpaced,
    ];

    fn schedule(self) -> &'static str {
        match self {
            Self::Drag => "drag",
            Self::DenseUnspaced | Self::DenseSpaced => "dense-beat",
            Self::MusicalSpaced => "musical-beat",
        }
    }

    fn admission(self) -> &'static str {
        match self {
            Self::Drag => "pointer",
            Self::DenseUnspaced => "unspaced",
            Self::DenseSpaced | Self::MusicalSpaced => "spaced",
        }
    }

    fn baseline_column(self) -> Option<usize> {
        match self {
            Self::Drag => Some(0),
            Self::DenseUnspaced => None,
            Self::DenseSpaced => Some(1),
            Self::MusicalSpaced => Some(2),
        }
    }
}

fn drag_x(frame: usize) -> f64 {
    let period = FPS / 2;
    let frame = frame % period;
    frame.min(period - frame) as f64 / (period / 2) as f64
}

/// Supply normalized lever triples, not an audio recording. Every offered
/// onset moves the target, even when the gate skips several intervening hits.
fn beat_levers(trace: Trace, frame: usize) -> SpectrumLevers {
    let interval = if trace == Trace::MusicalSpaced {
        FPS / 4
    } else {
        1
    };
    if !frame.is_multiple_of(interval) {
        return SpectrumLevers {
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            onset: 1.0,
        };
    }
    let pulse = frame / interval;
    let mid = 0.1 + 0.7 * ((pulse * 17) % 31) as f32 / 30.0;
    SpectrumLevers {
        bass: 0.9 - mid,
        mid,
        treble: 0.1,
        onset: 4.0,
    }
}

#[derive(Default)]
struct Beats {
    pokes: Vec<(f64, f64)>,
    last_accepted: Option<Duration>,
    offered: usize,
}

impl Beats {
    fn offer(&mut self, trace: Trace, levers: &SpectrumLevers, frame: usize) {
        if !crate::spectrum_should_poke(levers) {
            return;
        }
        self.offered += 1;
        let now = Duration::from_secs_f64(frame as f64 / FPS as f64);
        // The historical control preserves the same cap and duplicate rule
        // but presents every new onset as immediately eligible.
        let elapsed = if trace == Trace::DenseUnspaced {
            None
        } else {
            self.last_accepted.map(|then| now - then)
        };
        if plant_spectrum_poke(&mut self.pokes, levers, elapsed) {
            self.last_accepted = Some(now);
        }
    }
}

struct Measurement {
    room: &'static str,
    trace: Trace,
    era: Era,
    speed: usize,
    offered: usize,
    admitted: usize,
    peaks: TimelineFlashPeaks,
}

fn measure_trace(
    room: &dyn Room,
    size: (usize, usize),
    trace: Trace,
    speed: usize,
) -> [Measurement; 4] {
    let (width, height) = size;
    let meta = room.meta();
    let mut captures = Era::ALL.map(|era| {
        (
            era,
            RegionalFlashCapture::new(width, height).expect("valid report raster"),
        )
    });
    let mut transformed = Vec::with_capacity(width * height * 4);
    let mut inputs = Vec::new();
    let mut beats = Beats::default();
    let mut phase = 0.0;
    let mut pump = 1.0;
    let mut pending_nudge = 0.0;
    for frame in 0..FRAMES {
        // Frame zero is the fresh room; later ticks use the previous draw's pump.
        let dt = if frame == 0 { 0.0 } else { 1.0 / FPS as f64 };
        let scale = crate::presented_time_scale(speed as f64, pump, meta.id);
        let step = crate::visualizer_phase_step(
            meta.id,
            dt,
            crate::ROOM_CYCLES_PER_SECOND * dt * scale,
            pending_nudge,
        );
        phase = (phase + step).rem_euclid(1.0);
        let mut raster = crate::raster::Raster::with_accent(width, height, meta.accent);
        if trace == Trace::Drag {
            let x = drag_x(frame);
            room.compact_inputs(&mut inputs);
            // Consecutive points on this continuous horizontal sweep are far
            // enough apart to clear the App's primary-drag trail decimation.
            inputs.push(if frame == 0 {
                RoomInput::PointerDown {
                    x,
                    y: 0.5,
                    t: phase,
                }
            } else {
                RoomInput::PointerMove {
                    x,
                    y: 0.5,
                    t: phase,
                }
            });
            inputs.drain(..inputs.len().saturating_sub(crate::MAX_ROOM_INPUTS));
            room.render_input(&mut raster, phase, &inputs);
        } else {
            room.render_poked(&mut raster, phase, &beats.pokes);
        }
        visit_in_each_era(&raster, &mut transformed, &mut captures, |capture, rgba| {
            assert!(capture.push(rgba));
        });
        // The App draws the room before it reads spectrum levers. The next
        // tick consumes the new pump and nudge together. The final frame has
        // no successor that could show an additional poke.
        if trace != Trace::Drag && frame + 1 < FRAMES {
            let levers = beat_levers(trace, frame);
            pump = crate::spectrum_time_scale(1.0, &levers);
            pending_nudge = crate::spectrum_phase_nudge(&levers);
            beats.offer(trace, &levers, frame);
        }
    }
    let (offered, admitted) = if trace == Trace::Drag {
        (FRAMES, FRAMES)
    } else {
        (beats.offered, beats.pokes.len())
    };
    captures.map(|(era, capture)| Measurement {
        room: meta.id,
        trace,
        era,
        speed,
        offered,
        admitted,
        peaks: capture.timeline_peaks(FPS).expect("full seconds of input"),
    })
}

fn write_report(
    mut out: impl Write,
    rows: &[Measurement],
    size: (usize, usize),
) -> std::io::Result<()> {
    writeln!(
        out,
        "room,seed,schedule,renderer,admission,era,speed,frame_width,frame_height,fps,frames,offered,admitted,whole_general,whole_red,window_width,window_height,general_peak,general_x,general_y,red_peak,red_x,red_y"
    )?;
    for row in rows {
        let p = row.peaks;
        let general = p.general.window;
        let red = p.red.window;
        let renderer = if row.trace == Trace::Drag {
            "render_input"
        } else {
            "render_poked"
        };
        writeln!(
            out,
            "{},0,{},{},{},{},{},{},{},{FPS},{FRAMES},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.room,
            row.trace.schedule(),
            renderer,
            row.trace.admission(),
            row.era.name(),
            row.speed,
            size.0,
            size.1,
            row.offered,
            row.admitted,
            p.whole_general,
            p.whole_red,
            general.width,
            general.height,
            p.general.rate,
            general.x,
            general.y,
            p.red.rate,
            red.x,
            red.y
        )?;
    }
    Ok(())
}

type BaselineRow = (&'static str, [u8; 3], [u8; 3]);

fn baseline_changes(
    rooms: &[RoomMeta],
    rows: &[Measurement],
    traces: &[Trace],
    baseline: &[BaselineRow],
) -> Vec<String> {
    let mut changes = Vec::new();
    for meta in rooms {
        let known = baseline.iter().find(|(id, _, _)| *id == meta.id);
        for &trace in traces {
            let Some(col) = trace.baseline_column() else {
                continue;
            };
            let selected = rows
                .iter()
                .filter(|r| r.room == meta.id && r.trace == trace);
            let (mut general, mut red) = (0, 0);
            for row in selected {
                general = general.max(row.peaks.whole_general.max(row.peaks.general.rate));
                red = red.max(row.peaks.whole_red.max(row.peaks.red.rate));
            }
            let actual = (flash_overage(general), flash_overage(red));
            let expected = known.map_or((0, 0), |(_, g, r)| (g[col], r[col]));
            if actual != expected {
                changes.push(format!(
                    "{} {}: measured {actual:?}, recorded {expected:?}",
                    meta.id,
                    trace.schedule()
                ));
            }
        }
    }
    for (id, _, _) in baseline {
        if !rooms.iter().any(|meta| meta.id == *id) {
            changes.push(format!("retired room in interaction baseline: {id}"));
        }
    }
    changes
}

fn run_report(
    rooms: &[RoomMeta],
    size: (usize, usize),
    directory: &Path,
    selected: Option<&str>,
    baseline: &[BaselineRow],
) -> Vec<String> {
    let traces: Vec<_> = Trace::ALL
        .into_iter()
        .filter(|trace| selected.is_none_or(|name| name == trace.schedule()))
        .collect();
    assert!(
        !traces.is_empty(),
        "unknown interaction schedule: {selected:?}"
    );
    let rows = measure_rooms(rooms, |meta| {
        let room = crate::room_by_id(meta.id).expect("catalog room");
        let mut rows = Vec::new();
        for &trace in &traces {
            for speed in REGIONAL_FLASH_SPEEDS {
                rows.extend(measure_trace(room.as_ref(), size, trace, speed));
            }
        }
        rows
    });
    assert_eq!(
        rows.len(),
        rooms.len() * traces.len() * REGIONAL_FLASH_SPEEDS.len() * Era::ALL.len()
    );
    std::fs::create_dir_all(directory).expect("report directory");
    let filename = selected.map_or_else(
        || "interaction-flash.csv".into(),
        |s| format!("interaction-flash-{s}.csv"),
    );
    let mut out = std::io::BufWriter::new(
        std::fs::File::create(directory.join(filename)).expect("create interaction report"),
    );
    write_report(&mut out, &rows, size).expect("write interaction report");
    out.flush().expect("flush interaction report");
    baseline_changes(rooms, &rows, &traces, baseline)
}

#[test]
#[ignore = "finite interaction flash report across the catalog at the reference raster; run nightly"]
fn interaction_room_flash_report() {
    let selected = std::env::var("NUMINOUS_INTERACTION_SCHEDULE").ok();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/qol");
    let changes = run_report(
        crate::ROOM_CATALOG,
        FLASH_REFERENCE,
        &directory,
        selected.as_deref(),
        super::KNOWN_OVER_INTERACTION_FLASHES,
    );
    assert!(
        changes.is_empty(),
        "interaction measurements changed:\n{}",
        changes.join("\n")
    );
}

#[test]
fn scripted_sweeps_and_beats_span_real_time_at_every_playback_speed() {
    assert_eq!(drag_x(0), 0.0);
    assert_eq!(drag_x(FPS / 4), 1.0);
    assert_eq!(drag_x(FPS / 2), 0.0);
    assert_eq!(drag_x(FRAMES - 1), 0.0);
    for (trace, offered, admitted) in [
        (Trace::DenseUnspaced, 240, crate::MAX_ROOM_POKES),
        (Trace::DenseSpaced, 240, 12),
        (Trace::MusicalSpaced, 16, 8),
    ] {
        let mut beats = Beats::default();
        for frame in 0..FRAMES - 1 {
            beats.offer(trace, &beat_levers(trace, frame), frame);
        }
        assert_eq!(
            (beats.offered, beats.pokes.len()),
            (offered, admitted),
            "{trace:?}"
        );
    }
}

#[test]
fn finite_report_checks_the_selected_schedules_after_writing_their_evidence() {
    let rooms = [crate::room_meta_by_id("rule-30").unwrap()];
    let directory =
        std::env::temp_dir().join(format!("numinous-interaction-flash-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let changes = run_report(&rooms, (12, 9), &directory, None, &[]);
    let stale = [("rule-30", [30, 4, 0], [0; 3]), ("retired", [5; 3], [0; 3])];
    let dense_changes = run_report(&rooms, (12, 9), &directory, Some("dense-beat"), &stale);
    let read_and_remove = |name| {
        let path = directory.join(name);
        let csv = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        csv
    };
    let full = read_and_remove("interaction-flash.csv");
    let dense = read_and_remove("interaction-flash-dense-beat.csv");
    std::fs::remove_dir(directory).unwrap();

    assert_eq!(changes, ["rule-30 drag: measured (30, 0), recorded (0, 0)"]);
    assert_eq!(
        dense_changes,
        [
            "rule-30 dense-beat: measured (0, 0), recorded (4, 0)",
            "retired room in interaction baseline: retired",
        ]
    );

    let mut lines = full.lines();
    let header = lines.next().unwrap();
    let mut expected_dense = vec![header];
    let mut peaks = std::collections::BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    for line in lines {
        let cols: Vec<_> = line.split(',').collect();
        assert_eq!(cols.len(), header.split(',').count());
        assert_eq!(&cols[..2], ["rule-30", "0"]);
        assert_eq!(&cols[7..11], ["12", "9", "60", "241"]);
        let mode = (cols[2], cols[4]);
        let (renderer, counts) = match mode {
            ("drag", "pointer") => ("render_input", ["241", "241"]),
            ("dense-beat", "unspaced") => ("render_poked", ["240", "24"]),
            ("dense-beat", "spaced") => ("render_poked", ["240", "12"]),
            ("musical-beat", "spaced") => ("render_poked", ["16", "8"]),
            _ => panic!("unexpected report mode {mode:?}"),
        };
        assert_eq!(cols[3], renderer);
        assert_eq!(&cols[11..13], counts);
        if cols[2] == "dense-beat" {
            expected_dense.push(line);
        }
        assert!(Era::ALL.iter().any(|era| era.name() == cols[5]));
        assert!(REGIONAL_FLASH_SPEEDS.contains(&cols[6].parse().unwrap()));
        assert!(seen.insert((mode, cols[5], cols[6])));
        let peak = peaks.entry(mode).or_insert(0usize);
        for col in [13, 14, 17, 20] {
            *peak = (*peak).max(cols[col].parse().unwrap());
        }
    }
    assert_eq!(
        seen.len(),
        Trace::ALL.len() * Era::ALL.len() * REGIONAL_FLASH_SPEEDS.len()
    );
    assert!(dense.lines().eq(expected_dense));
    // The unspaced readings survive in the CSV even when the spaced trace
    // stays below the threshold and contributes zero to the lookup.
    assert_eq!(peaks[&("dense-beat", "unspaced")], 8);
    assert_eq!(peaks[&("dense-beat", "spaced")], 1);
}

#[test]
fn beat_history_and_phase_are_visible_on_the_following_frame() {
    use std::cell::RefCell;

    struct Probe(RefCell<Vec<(f64, usize)>>);

    impl crate::RoomMetadata for Probe {
        fn meta(&self) -> RoomMeta {
            crate::room_meta_by_id("julia").unwrap()
        }
    }
    impl Room for Probe {
        fn render(&self, _: &mut dyn crate::Surface, _: f64) {
            unreachable!("the probe only measures the poke renderer")
        }

        fn reveal(&self) -> &'static str {
            unreachable!("rendering does not request a reveal")
        }

        fn render_poked(&self, _: &mut dyn crate::Surface, t: f64, pokes: &[(f64, f64)]) {
            self.0.borrow_mut().push((t, pokes.len()));
        }
    }

    for speed in [1, 8] {
        let room = Probe(RefCell::default());
        measure_trace(&room, (1, 1), Trace::DenseSpaced, speed);
        let frames = room.0.into_inner();
        assert_eq!(frames.len(), FRAMES);
        assert_eq!(frames[0], (0.0, 0), "new onsets land after the draw");
        assert_eq!(frames[1].1, 1);
        let first = beat_levers(Trace::DenseSpaced, 0);
        let pump = crate::spectrum_time_scale(1.0, &first);
        let scale = crate::presented_time_scale(speed as f64, pump, "julia");
        let ambient = crate::ROOM_CYCLES_PER_SECOND / FPS as f64 * scale;
        let cap = crate::ROOM_CYCLES_PER_SECOND / FPS as f64 * crate::flash_budget_scale("julia");
        let expected = (ambient + crate::spectrum_phase_nudge(&first)).min(cap);
        assert!(
            (frames[1].0 - expected).abs() < 1e-12,
            "apply the observed pump and nudge together"
        );
        for pair in frames.windows(2) {
            let advance = (pair[1].0 - pair[0].0).rem_euclid(1.0);
            assert!(
                advance <= cap + 1e-12,
                "adjacent presented phases moved {advance} past {cap}"
            );
        }
    }
}
