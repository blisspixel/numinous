//! The `numinous` command line: the terminal face of the headless core.
//!
//! See `docs/INTERFACES.md`. This increment lists the catalog, describes a room,
//! and renders a room as ASCII in the terminal (the Teletype face). GPU preview,
//! audio, and the Studio REPL arrive in later increments.
//!
//! The command handlers are split into pure `*_report` functions that return the
//! text to emit, so they can be unit-tested without capturing stdout; `main`
//! stays a thin shell that prints and sets the exit code.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use numinous_core::{Canvas, Raster, Room, RoomMeta, Surface, all_rooms, draw_text, room_by_id};

#[derive(Parser)]
#[command(
    name = "numinous",
    version,
    about = "Numinous: math you can feel (CLI face)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all rooms in the catalog.
    Rooms {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Describe a single room by id.
    Describe {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Render a room as ASCII in the terminal, or as a PNG image with --out.
    Render {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Width in columns (ASCII) or pixels (PNG).
        #[arg(long, default_value_t = 80)]
        width: usize,
        /// Height in rows (ASCII) or pixels (PNG).
        #[arg(long, default_value_t = 40)]
        height: usize,
        /// Phase in [0, 1): for Times Tables this sweeps the multiplier.
        #[arg(long, default_value_t = 0.0)]
        t: f64,
        /// Write a PNG image to this path instead of ASCII to the terminal.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Render a room's sound to a WAV file (everything is an instrument).
    Sonify {
        /// Room id, e.g. "lissajous".
        id: String,
        /// Phase in [0, 1).
        #[arg(long, default_value_t = 0.0)]
        t: f64,
        /// Write a WAV audio file to this path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Render every room to a PNG image in a directory (a showcase and beauty-QA).
    Gallery {
        /// Directory to write the images into.
        #[arg(long, default_value = "renders")]
        dir: PathBuf,
        /// Image width in pixels.
        #[arg(long, default_value_t = 800)]
        width: usize,
        /// Image height in pixels.
        #[arg(long, default_value_t = 800)]
        height: usize,
    },
    /// Render every room into one tiled contact-sheet image.
    ContactSheet {
        /// Where to write the sheet.
        #[arg(long, default_value = "renders/contact.png")]
        out: PathBuf,
        /// Number of columns in the grid.
        #[arg(long, default_value_t = 3)]
        cols: usize,
        /// Size of each room tile in pixels.
        #[arg(long, default_value_t = 320)]
        tile: usize,
    },
    /// Play a room live in the terminal, animating its phase (Ctrl+C to stop).
    Play {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Frames per second.
        #[arg(long, default_value_t = 12.0)]
        fps: f64,
        /// Canvas width in columns.
        #[arg(long, default_value_t = 80)]
        width: usize,
        /// Canvas height in rows.
        #[arg(long, default_value_t = 36)]
        height: usize,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Rooms { json } => {
            print!("{}", rooms_report(json));
            ExitCode::SUCCESS
        }
        Command::Describe { id, json } => emit(describe_report(&id, json)),
        Command::Render {
            id,
            width,
            height,
            t,
            out,
        } => match out {
            Some(path) => emit(render_png(&id, width, height, t, &path)),
            None => emit(render_report(&id, width, height, t)),
        },
        Command::Sonify { id, t, out } => emit(sonify_wav(&id, t, &out)),
        Command::Gallery { dir, width, height } => emit(gallery(&dir, width, height)),
        Command::ContactSheet { out, cols, tile } => emit(contact_sheet(&out, cols, tile)),
        Command::Play {
            id,
            fps,
            width,
            height,
        } => play(&id, fps, width, height),
    }
}

/// Print a report to stdout, or its error to stderr, and map to an exit code.
fn emit(report: Result<String, String>) -> ExitCode {
    match report {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprint!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// The catalog listing, as human text or JSON.
fn rooms_report(json: bool) -> String {
    let rooms = all_rooms();
    if json {
        let arr: Vec<serde_json::Value> = rooms.iter().map(|r| meta_json(&r.meta())).collect();
        format!("{}\n", to_pretty(&serde_json::Value::Array(arr)))
    } else {
        let lines: Vec<String> = rooms
            .iter()
            .map(|r| {
                let m = r.meta();
                format!("{:<16} {:<20} {}", m.id, m.wing, m.title)
            })
            .collect();
        format!("{}\n", lines.join("\n"))
    }
}

/// One room's description, or a guiding error if the id is unknown.
fn describe_report(id: &str, json: bool) -> Result<String, String> {
    let room = room_by_id(id).ok_or_else(|| not_found_message(id))?;
    let m = room.meta();
    Ok(if json {
        let mut value = meta_json(&m);
        value["reveal"] = serde_json::Value::String(room.reveal().to_string());
        format!("{}\n", to_pretty(&value))
    } else {
        format!(
            "{} ({})\nWing: {}\n\n{}\n\nReveal: {}\n",
            m.title,
            m.id,
            m.wing,
            m.blurb,
            room.reveal()
        )
    })
}

/// A room rendered to ASCII, or a guiding error if the id is unknown.
fn render_report(id: &str, width: usize, height: usize, t: f64) -> Result<String, String> {
    let room = room_by_id(id).ok_or_else(|| not_found_message(id))?;
    let mut canvas = Canvas::new(width, height);
    room.render(&mut canvas, t);
    Ok(canvas.to_text())
}

/// Render a room to a PNG image at `path`, returning a status message.
fn render_png(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    path: &Path,
) -> Result<String, String> {
    let room = room_by_id(id).ok_or_else(|| not_found_message(id))?;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    write_png(path, &raster)?;
    Ok(format!(
        "wrote {} ({}x{})\n",
        path.display(),
        raster.width(),
        raster.height()
    ))
}

/// Encode a raster as an RGBA PNG at `path`.
fn write_png(path: &Path, raster: &Raster) -> Result<(), String> {
    let (w, h) = (raster.width(), raster.height());
    let file =
        File::create(path).map_err(|e| format!("could not create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header failed: {e}"))?;
    writer
        .write_image_data(&raster.to_rgba())
        .map_err(|e| format!("png write failed: {e}"))?;
    Ok(())
}

/// Render every room into one tiled contact-sheet PNG.
fn contact_sheet(path: &Path, cols: usize, tile: usize) -> Result<String, String> {
    let rooms = all_rooms();
    let cols = cols.max(1);
    let rows = rooms.len().div_ceil(cols);
    let mut sheet = Raster::new(cols * tile, rows * tile);
    let label_scale = (tile as i32 / 160).clamp(1, 3);
    for (i, room) in rooms.iter().enumerate() {
        let mut cell = Raster::with_accent(tile, tile, room.meta().accent);
        room.render(&mut cell, 0.0);
        let (x, y) = ((i % cols) * tile, (i / cols) * tile);
        sheet.blit(&cell, x, y);
        draw_text(
            &mut sheet,
            &room.meta().title.to_uppercase(),
            x as i32 + 8,
            y as i32 + 8,
            label_scale,
            '#',
        );
    }
    write_png(path, &sheet)?;
    Ok(format!(
        "wrote contact sheet {} ({} rooms, {}x{})\n",
        path.display(),
        rooms.len(),
        cols * tile,
        rows * tile
    ))
}

/// Render a room's sound to a 16-bit mono WAV at `path`, returning a status message.
fn sonify_wav(id: &str, t: f64, path: &Path) -> Result<String, String> {
    let room = room_by_id(id).ok_or_else(|| not_found_message(id))?;
    let spec = room.sound(t);
    let sample_rate = 44_100u32;
    let samples = spec.render(sample_rate);

    let wav_spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)
        .map_err(|e| format!("could not create {}: {e}", path.display()))?;
    for sample in &samples {
        writer
            .write_sample((sample * f32::from(i16::MAX)) as i16)
            .map_err(|e| format!("wav write failed: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("wav finalize failed: {e}"))?;
    Ok(format!(
        "wrote {} ({:.1}s, {} notes)\n",
        path.display(),
        spec.duration,
        spec.notes.len()
    ))
}

/// Render every room to `<dir>/<id>.png`, returning a status message.
fn gallery(dir: &Path, width: usize, height: usize) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut count = 0usize;
    for room in all_rooms() {
        let id = room.meta().id;
        let path = dir.join(format!("{id}.png"));
        render_png(id, width, height, 0.0, &path)?;
        count += 1;
    }
    Ok(format!("wrote {count} room images to {}\n", dir.display()))
}

/// Build one terminal frame: clear the screen, render the room, and add a status
/// line. Pure and testable; the animation loop just prints these in sequence.
fn play_frame(room: &dyn Room, t: f64, width: usize, height: usize) -> String {
    let mut canvas = Canvas::new(width, height);
    room.render(&mut canvas, t);
    // \x1b[2J clears the screen, \x1b[H moves the cursor home.
    format!(
        "\x1b[2J\x1b[H{}\n[{}]  t = {t:.2}   (Ctrl+C to stop)\n",
        canvas.to_text(),
        room.meta().title
    )
}

/// Animate a room in the terminal, sweeping its phase, until interrupted.
fn play(id: &str, fps: f64, width: usize, height: usize) -> ExitCode {
    let Some(room) = room_by_id(id) else {
        eprint!("{}", not_found_message(id));
        return ExitCode::FAILURE;
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let mut stdout = std::io::stdout();
    let mut t = 0.0f64;
    loop {
        let _ = write!(stdout, "{}", play_frame(room.as_ref(), t, width, height));
        let _ = stdout.flush();
        std::thread::sleep(frame_time);
        t = if t + 0.01 >= 1.0 { 0.0 } else { t + 0.01 };
    }
}

fn not_found_message(id: &str) -> String {
    let known: Vec<&str> = all_rooms().iter().map(|r| r.meta().id).collect();
    format!(
        "No room with id '{id}'. Known rooms: {}\n",
        known.join(", ")
    )
}

fn meta_json(m: &RoomMeta) -> serde_json::Value {
    serde_json::json!({
        "id": m.id,
        "title": m.title,
        "wing": m.wing,
        "blurb": m.blurb,
    })
}

fn to_pretty(value: &serde_json::Value) -> String {
    // Pretty-print, falling back to the compact form. Serializing an
    // already-constructed Value does not fail in practice; this avoids any
    // explicit panic in a production path.
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{describe_report, meta_json, not_found_message, render_report, rooms_report};
    use numinous_core::room_by_id;
    use serde_json::Value;

    #[test]
    fn rooms_report_lists_times_tables() {
        assert!(rooms_report(false).contains("times-tables"));
    }

    #[test]
    fn rooms_report_json_is_a_non_empty_array() {
        let text = rooms_report(true);
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert!(value.as_array().is_some_and(|a| !a.is_empty()));
    }

    #[test]
    fn describe_known_room_reports_its_wing() {
        let text = describe_report("times-tables", false).expect("known room");
        assert!(text.contains("Number & Pattern"));
    }

    #[test]
    fn describe_json_carries_the_id() {
        let text = describe_report("times-tables", true).expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value["id"], "times-tables");
    }

    #[test]
    fn describe_includes_the_reveal() {
        let text = describe_report("times-tables", false).expect("known room");
        assert!(text.contains("Reveal:"));
        assert!(text.contains("Mandelbrot"));
    }

    #[test]
    fn describe_json_includes_the_reveal() {
        let text = describe_report("times-tables", true).expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert!(
            value["reveal"]
                .as_str()
                .is_some_and(|s| s.contains("Mandelbrot"))
        );
    }

    #[test]
    fn describe_unknown_room_guides_the_user() {
        let err = describe_report("no-such-room", false).expect_err("unknown room");
        assert!(err.contains("Known rooms"));
    }

    #[test]
    fn render_known_room_has_ink() {
        let text = render_report("times-tables", 40, 20, 0.0).expect("known room");
        assert!(text.contains('*'));
    }

    #[test]
    fn render_unknown_room_is_error() {
        assert!(render_report("no-such-room", 10, 10, 0.0).is_err());
    }

    #[test]
    fn meta_json_has_expected_fields() {
        let room = room_by_id("times-tables").expect("known room");
        let value = meta_json(&room.meta());
        for key in ["id", "title", "wing", "blurb"] {
            assert!(value.get(key).is_some(), "missing key {key}");
        }
    }

    #[test]
    fn not_found_message_lists_known_rooms() {
        assert!(not_found_message("x").contains("times-tables"));
    }

    #[test]
    fn render_png_writes_a_non_empty_file() {
        let mut path = std::env::temp_dir();
        path.push("numinous_cli_render_test.png");
        let message = super::render_png("times-tables", 64, 48, 0.0, &path).expect("render png");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "png should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn render_png_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_should_not_exist.png");
        assert!(super::render_png("no-such-room", 10, 10, 0.0, &path).is_err());
    }

    #[test]
    fn sonify_wav_writes_a_non_empty_file() {
        let mut path = std::env::temp_dir();
        path.push("numinous_cli_sonify_test.wav");
        let message = super::sonify_wav("lissajous", 0.0, &path).expect("sonify");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "wav should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sonify_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_no.wav");
        assert!(super::sonify_wav("no-such-room", 0.0, &path).is_err());
    }

    #[test]
    fn gallery_writes_one_image_per_room() {
        let dir = std::env::temp_dir().join("numinous_gallery_test");
        let _ = std::fs::remove_dir_all(&dir);
        let message = super::gallery(&dir, 40, 40).expect("gallery");
        assert!(message.contains("wrote"));
        let files = std::fs::read_dir(&dir).expect("dir exists").count();
        assert_eq!(files, numinous_core::all_rooms().len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn contact_sheet_writes_a_non_empty_file() {
        let path = std::env::temp_dir().join("numinous_contact_test.png");
        let message = super::contact_sheet(&path, 3, 32).expect("contact sheet");
        assert!(message.contains("contact sheet"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn play_frame_shows_the_room() {
        let room = numinous_core::room_by_id("times-tables").expect("room");
        let frame = super::play_frame(room.as_ref(), 0.0, 30, 15);
        assert!(frame.contains("Times Tables"));
        assert!(frame.contains('*'));
    }

    #[test]
    fn play_frame_changes_with_phase() {
        let room = numinous_core::room_by_id("times-tables").expect("room");
        let a = super::play_frame(room.as_ref(), 0.0, 40, 20);
        let b = super::play_frame(room.as_ref(), 0.6, 40, 20);
        assert_ne!(a, b, "the frame should animate as t changes");
    }

    #[test]
    fn render_png_to_an_unwritable_path_is_error() {
        let bad = std::path::Path::new("no_such_dir_zzz/x.png");
        assert!(super::render_png("times-tables", 8, 8, 0.0, bad).is_err());
    }

    #[test]
    fn sonify_to_an_unwritable_path_is_error() {
        let bad = std::path::Path::new("no_such_dir_zzz/x.wav");
        assert!(super::sonify_wav("lissajous", 0.0, bad).is_err());
    }
}
