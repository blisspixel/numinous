//! Nightly regional window-mean report for the automatic catalog tours.

use std::io::Write;
use std::path::Path;
use std::thread;

use super::{
    FLASH_REFERENCE, KNOWN_OVER_FLASH_AREA, REGIONAL_FLASH_SPEEDS as SPEEDS, visit_era_frames,
};
use crate::photosensitivity::MAX_FLASHES_PER_SECOND;
use crate::photosensitivity::regional::{RegionalFlashCapture, RegionalFlashes};
use crate::{Era, ROOM_CATALOG, RoomMeta, room_by_id};

struct Measurement {
    room: &'static str,
    era: Era,
    speeds: [RegionalFlashes; 4],
}

fn measure_catalog(rooms: &[RoomMeta], size: (usize, usize)) -> Vec<Measurement> {
    let fps = crate::APP_FRAMES_PER_SECOND as usize;
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(rooms.len());
    let mut results = thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..workers {
            handles.push(scope.spawn(move || {
                let mut rows = Vec::new();
                for meta in rooms.iter().skip(worker).step_by(workers) {
                    let room = room_by_id(meta.id).expect("catalog room");
                    let mut captures = Era::ALL.map(|era| {
                        (
                            era,
                            RegionalFlashCapture::new(size.0, size.1)
                                .expect("valid reference raster"),
                        )
                    });
                    visit_era_frames(room.as_ref(), size, &mut captures, |capture, frame| {
                        assert!(
                            capture.push(frame),
                            "render dimensions must match the capture"
                        );
                    });
                    rows.extend(captures.map(|(era, capture)| Measurement {
                        room: meta.id,
                        era,
                        speeds: SPEEDS.map(|step| {
                            capture
                                .at_step(step, fps)
                                .expect("nonempty capture and frame rate")
                        }),
                    }));
                }
                rows
            }));
        }
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("regional report worker"))
            .collect::<Vec<_>>()
    });
    results.sort_by_key(|r| r.room);
    results
}

fn write_report(
    mut output: impl Write,
    rows: &[Measurement],
    (width, height): (usize, usize),
) -> std::io::Result<()> {
    writeln!(
        output,
        "room,seed,era,speed,frame_width,frame_height,fps,window_width,window_height,general_peak,general_x,general_y,red_peak,red_x,red_y,extended_general,extended_red"
    )?;
    for row in rows {
        for (speed, m) in SPEEDS.iter().zip(&row.speeds) {
            let general = &m.general;
            let red = &m.red;
            writeln!(
                output,
                "{},0,{},{speed},{width},{height},{},{},{},{},{},{},{},{},{},{},{}",
                row.room,
                row.era.name(),
                crate::APP_FRAMES_PER_SECOND,
                general.window.width,
                general.window.height,
                general.rate,
                general.window.x,
                general.window.y,
                red.rate,
                red.window.x,
                red.window.y,
                m.extended_general,
                m.extended_red
            )?;
        }
    }
    Ok(())
}

type BaselineRow = (&'static str, Era, [u8; 4], [u8; 4]);

fn overage(rate: usize) -> u8 {
    if rate as f64 > MAX_FLASHES_PER_SECOND {
        u8::try_from(rate).expect("one second of frames bounds its flash count")
    } else {
        0
    }
}

fn over_budget(rows: &[Measurement]) -> Vec<BaselineRow> {
    let mut entries = Vec::new();
    for row in rows {
        let general = row.speeds.each_ref().map(|p| overage(p.general.rate));
        let red = row.speeds.each_ref().map(|p| overage(p.red.rate));
        if general != [0; 4] || red != [0; 4] {
            entries.push((row.room, row.era, general, red));
        }
    }
    entries
}

fn baseline_changes(measured: &[BaselineRow], known: &[BaselineRow]) -> Vec<String> {
    let mut changes = Vec::new();
    for row in measured {
        if !known.contains(row) {
            changes.push(format!("measured {row:?}"));
        }
    }
    for row in known {
        if !measured.contains(row) {
            changes.push(format!("recorded {row:?}"));
        }
    }
    changes
}

#[test]
#[ignore = "full regional catalog report at the reference raster; run nightly"]
fn automatic_room_regional_flash_report() {
    assert_eq!(
        SPEEDS.last().map(|&s| s as f64),
        Some(crate::MAX_TIME_SCALE)
    );
    let rows = measure_catalog(ROOM_CATALOG, FLASH_REFERENCE);
    assert_eq!(rows.len(), ROOM_CATALOG.len() * Era::ALL.len());
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/qol");
    std::fs::create_dir_all(&directory).expect("report directory");
    let mut file = std::io::BufWriter::new(
        std::fs::File::create(directory.join("regional-flash.csv"))
            .expect("create regional report"),
    );
    write_report(&mut file, &rows, FLASH_REFERENCE).expect("write regional report");
    file.flush().expect("flush regional report");
    let changes = baseline_changes(&over_budget(&rows), KNOWN_OVER_FLASH_AREA);
    assert!(
        changes.is_empty(),
        "regional flash measurements changed:\n{}",
        changes.join("\n")
    );
}

#[test]
fn sample_report_records_each_era_speed_and_window_location() {
    let room = crate::room_meta_by_id("times-tables").unwrap();
    let rows = measure_catalog(std::slice::from_ref(&room), (12, 9));
    let mut bytes = Vec::new();
    write_report(&mut bytes, &rows, (12, 9)).unwrap();
    let csv = String::from_utf8(bytes).unwrap();
    let (header, data) = csv.split_once('\n').unwrap();
    assert_eq!(header.split(',').count(), 17);
    let lines: Vec<_> = data.lines().collect();
    assert_eq!(lines.len(), Era::ALL.len() * SPEEDS.len());
    for (era, block) in Era::ALL
        .into_iter()
        .zip(lines.as_chunks::<{ SPEEDS.len() }>().0)
    {
        for (speed, line) in SPEEDS.into_iter().zip(block) {
            let fields: Vec<_> = line.split(',').collect();
            assert_eq!(
                &fields[..9],
                &[
                    "times-tables",
                    "0",
                    era.name(),
                    &speed.to_string(),
                    "12",
                    "9",
                    "60",
                    "4",
                    "3"
                ]
            );
            for (x, y) in [(10, 11), (13, 14)] {
                assert!(fields[x].parse::<usize>().unwrap() + 4 <= 12);
                assert!(fields[y].parse::<usize>().unwrap() + 3 <= 9);
            }
        }
    }
}

#[test]
fn regional_baseline_tracks_each_era_speed_and_flash_type() {
    let mut capture = RegionalFlashCapture::new(1, 1).unwrap();
    assert!(capture.push(&[0, 0, 0, 255]));
    let quiet = capture.at_step(1, 60).unwrap();
    let mut rows = [Era::Modern, Era::Vector].map(|era| Measurement {
        room: "r",
        era,
        speeds: [quiet; 4],
    });
    rows[0].speeds[0].red.rate = 3;
    rows[1].speeds[2].general.rate = 4;
    rows[1].speeds[3].general.rate = 5;
    let known = [("r", Era::Vector, [0, 0, 4, 5], [0; 4])];
    assert_eq!(over_budget(&rows), known);
    rows[1].speeds[1].red.rate = 4;
    assert_eq!(over_budget(&rows)[0].3, [0, 4, 0, 0]);
    assert!(baseline_changes(&known, &known).is_empty());
    for change in [
        ("r", Era::Modern, [0, 0, 4, 5], [0; 4]),
        ("r", Era::Vector, [0, 4, 4, 5], [0; 4]),
        ("r", Era::Vector, [0, 0, 4, 4], [0; 4]),
        ("r", Era::Vector, [0, 0, 4, 5], [0, 0, 0, 4]),
    ] {
        assert_eq!(baseline_changes(&[change], &known).len(), 2);
    }
    assert_eq!(baseline_changes(&[], &known).len(), 1);
    assert_eq!(baseline_changes(&known, &[]).len(), 1);
}
