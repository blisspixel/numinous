//! The `numinous` command line: the terminal face of the headless core.
//!
//! See `docs/INTERFACES.md`. This increment lists the catalog, describes a room,
//! and renders a room as ASCII in the terminal (the Teletype face). GPU preview,
//! audio, and the Studio REPL arrive in later increments.
//!
//! The command handlers are split into pure `*_report` functions that return the
//! text to emit, so they can be unit-tested without capturing stdout; `main`
//! stays a thin shell that prints and sets the exit code.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use numinous_core::{Canvas, RoomMeta, all_rooms, room_by_id};

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
    /// Render a room as ASCII in the terminal (the Teletype face).
    Render {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Canvas width in columns.
        #[arg(long, default_value_t = 80)]
        width: usize,
        /// Canvas height in rows.
        #[arg(long, default_value_t = 40)]
        height: usize,
        /// Phase in [0, 1): for Times Tables this sweeps the multiplier.
        #[arg(long, default_value_t = 0.0)]
        t: f64,
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
        } => emit(render_report(&id, width, height, t)),
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
    room.render_ascii(&mut canvas, t);
    Ok(canvas.to_text())
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
}
