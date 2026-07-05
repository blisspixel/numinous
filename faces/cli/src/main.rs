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
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use numinous_core::{
    Canvas, Journey, Rank, Raster, Room, RoomMeta, Surface, all_rooms, draw_text,
    hidden_room_by_id, room_by_id,
};

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
        /// Render in full 24-bit color in the terminal (two pixels per cell).
        #[arg(long)]
        color: bool,
    },
    /// Watch a room in full color in the terminal, with its sound, live.
    Watch {
        /// Room id, e.g. "mandelbrot".
        id: String,
        /// Frames per second.
        #[arg(long, default_value_t = 20.0)]
        fps: f64,
        /// Frame width in pixels (columns).
        #[arg(long, default_value_t = 100)]
        width: usize,
        /// Frame height in pixels (two per terminal row).
        #[arg(long, default_value_t = 56)]
        height: usize,
        /// Silence: skip the live audio.
        #[arg(long)]
        mute: bool,
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
    /// Play "guess the shape": name the room behind a mystery render.
    Quiz {
        /// Number of rounds.
        #[arg(long, default_value_t = 5)]
        rounds: usize,
        /// Seed (the same seed gives the same quiz).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Play today's shared puzzle (everyone gets the same one).
        #[arg(long)]
        daily: bool,
        /// Hard mode: six shapes to tell apart (opens at LV 3).
        #[arg(long)]
        hard: bool,
        /// Mystery render width in columns.
        #[arg(long, default_value_t = 54)]
        width: usize,
        /// Mystery render height in rows.
        #[arg(long, default_value_t = 22)]
        height: usize,
    },
    /// The jokes that live in Numinous, dissected (a frog dies for science).
    Jokes {
        /// Which specimen to dissect (omit to list them).
        index: Option<usize>,
    },
    /// Your constellation: where you have been, and what it has made of you.
    Journey,
    /// Crack the Code: defuse a math-clued bomb before your attempts run out.
    Crack {
        /// Seed (the same seed gives the same code).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Play today's shared code (everyone gets the same one).
        #[arg(long)]
        daily: bool,
        /// Number of digits in the code.
        #[arg(long, default_value_t = 4)]
        digits: usize,
        /// Attempts before the bomb blows.
        #[arg(long, default_value_t = 8)]
        attempts: usize,
    },
    /// SETI: scan the static and find the one channel that is not natural.
    Seti {
        /// Seed (the same seed gives the same scan).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Scan today's shared sky (everyone gets the same one).
        #[arg(long)]
        daily: bool,
        /// Channels per scan.
        #[arg(long, default_value_t = 4)]
        channels: usize,
        /// Number of scans.
        #[arg(long, default_value_t = 4)]
        rounds: usize,
    },
    /// Talk to the Aliens: continue the number sequence they transmit.
    Aliens {
        /// Seed (the same seed gives the same transmission).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Number of signals.
        #[arg(long, default_value_t = 5)]
        rounds: usize,
    },
    /// Munch: eat the numbers that fit the rule. Scored; compare with anyone.
    Munch {
        /// Seed (the same seed gives the same boards, human or AI).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Play today's shared boards.
        #[arg(long)]
        daily: bool,
        /// Number of boards.
        #[arg(long, default_value_t = 3)]
        rounds: usize,
    },
    /// The answer. (Opens at LV 42.)
    Answer,
    /// List the sims and their levers.
    Sims,
    /// Run a sim: render it and read the outcome. Set levers with --set name=value.
    Sim {
        /// Sim id, e.g. "tribbles".
        id: String,
        /// Set a lever, repeatable: --set breeding-rate=2.9.
        #[arg(long = "set")]
        set: Vec<String>,
        /// Render width in columns.
        #[arg(long, default_value_t = 70)]
        width: usize,
        /// Render height in rows.
        #[arg(long, default_value_t = 24)]
        height: usize,
    },
    /// Plot a function of x, e.g. numinous plot "sin(a*x)". Use a for a knob.
    Plot {
        /// The expression in x and a (funcs: sin cos tan exp ln abs sqrt; consts pi e).
        expr: String,
        /// Left edge of the x range.
        #[arg(long, default_value_t = -std::f64::consts::TAU)]
        xmin: f64,
        /// Right edge of the x range.
        #[arg(long, default_value_t = std::f64::consts::TAU)]
        xmax: f64,
        /// Value of the parameter a (constant unless animating).
        #[arg(long, default_value_t = 1.0)]
        a: f64,
        /// Animate: sweep a from amin to amax, Ctrl+C to stop.
        #[arg(long)]
        animate: bool,
        /// Start of the a sweep when animating.
        #[arg(long, default_value_t = 0.0)]
        amin: f64,
        /// End of the a sweep when animating.
        #[arg(long, default_value_t = std::f64::consts::TAU)]
        amax: f64,
        /// Plot width in columns.
        #[arg(long, default_value_t = 72)]
        width: usize,
        /// Plot height in rows.
        #[arg(long, default_value_t = 24)]
        height: usize,
    },
    /// Sing a function: turn y = f(x) into a melody and write a WAV.
    Sing {
        /// The expression in x.
        expr: String,
        /// Left edge of the x range.
        #[arg(long, default_value_t = -std::f64::consts::TAU)]
        xmin: f64,
        /// Right edge of the x range.
        #[arg(long, default_value_t = std::f64::consts::TAU)]
        xmax: f64,
        /// Number of notes.
        #[arg(long, default_value_t = 48)]
        notes: usize,
        /// Write a WAV audio file to this path.
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let mut journey = load_journey();
    let before = journey.clone();
    let code = run(Cli::parse().command, &mut journey);
    finish_journey(&before, &journey);
    code
}

/// Where the journey file lives: `NUMINOUS_JOURNEY` if set, else the home
/// directory, else the current directory.
fn journey_path() -> PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_JOURNEY") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".numinous-journey")
}

/// Load the journey, or start a fresh one.
fn load_journey() -> Journey {
    std::fs::read_to_string(journey_path())
        .map(|text| Journey::from_text(&text))
        .unwrap_or_default()
}

/// Persist the journey if it changed, and whisper once if a rank was crossed.
fn finish_journey(before: &Journey, after: &Journey) {
    if before == after {
        return;
    }
    let _ = std::fs::write(journey_path(), after.to_text());
    if after.rank() > before.rank() {
        println!("\n{}", after.rank().whisper());
    }
}

/// Find a room: the catalog always; the unlisted ones only for those judged
/// ready. An unready caller gets an ordinary not-found, no acknowledgment.
fn find_room(id: &str, allow_hidden: bool) -> Option<Box<dyn Room>> {
    room_by_id(id).or_else(|| {
        if allow_hidden {
            hidden_room_by_id(id)
        } else {
            None
        }
    })
}

/// Run one command, recording the journey as it goes.
fn run(command: Command, journey: &mut Journey) -> ExitCode {
    let allow_hidden = journey.rank() >= Rank::Mathematikos;
    match command {
        Command::Rooms { json } => {
            print!("{}", rooms_report(json));
            ExitCode::SUCCESS
        }
        Command::Describe { id, json } => {
            let report = describe_report(&id, json, allow_hidden, journey.level());
            if report.is_ok() && find_room(&id, allow_hidden).is_none() {
                // The name was not a room, yet it answered: a secret heard.
                journey.secret();
            }
            emit(report)
        }
        Command::Render {
            id,
            width,
            height,
            t,
            out,
            color,
        } => {
            if find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
            }
            match out {
                Some(path) => emit(render_png(&id, width, height, t, &path, allow_hidden)),
                None if color => emit(render_color_report(&id, width, height, t, allow_hidden)),
                None => emit(render_report(&id, width, height, t, allow_hidden)),
            }
        }
        Command::Watch {
            id,
            fps,
            width,
            height,
            mute,
        } => {
            if find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
                // The loop never returns; persist the visit before it starts.
                let _ = std::fs::write(journey_path(), journey.to_text());
            }
            watch(&id, fps, width, height, mute, allow_hidden)
        }
        Command::Sonify { id, t, out } => {
            if find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
            }
            emit(sonify_wav(&id, t, &out, allow_hidden))
        }
        Command::Gallery { dir, width, height } => emit(gallery(&dir, width, height)),
        Command::ContactSheet { out, cols, tile } => emit(contact_sheet(&out, cols, tile)),
        Command::Play {
            id,
            fps,
            width,
            height,
        } => {
            if find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
                let _ = std::fs::write(journey_path(), journey.to_text());
            }
            play(&id, fps, width, height, allow_hidden)
        }
        Command::Quiz {
            rounds,
            seed,
            daily,
            hard,
            width,
            height,
        } => {
            if hard && still_locked(journey, 3, "quiz --hard") {
                return ExitCode::FAILURE;
            }
            let choices = if hard { 6 } else { 4 };
            quiz(
                rounds,
                pick_seed(seed, daily),
                width,
                height,
                choices,
                journey,
            )
        }
        Command::Jokes { index } => {
            print!("{}", jokes_report(index));
            ExitCode::SUCCESS
        }
        Command::Journey => {
            print!("{}", journey_report(journey));
            ExitCode::SUCCESS
        }
        Command::Crack {
            seed,
            daily,
            digits,
            attempts,
        } => {
            if digits > 4 && still_locked(journey, 5, "crack --digits 5+") {
                return ExitCode::FAILURE;
            }
            crack(pick_seed(seed, daily), digits, attempts, journey)
        }
        Command::Seti {
            seed,
            daily,
            channels,
            rounds,
        } => {
            if channels > 4 && still_locked(journey, 7, "seti --channels 5+") {
                return ExitCode::FAILURE;
            }
            seti(pick_seed(seed, daily), channels, rounds, journey)
        }
        Command::Aliens { seed, rounds } => aliens(seed, rounds, journey),
        Command::Munch {
            seed,
            daily,
            rounds,
        } => munch(pick_seed(seed, daily), rounds, journey),
        Command::Answer => {
            if still_locked(journey, numinous_core::MAX_LEVEL, "the answer") {
                return ExitCode::FAILURE;
            }
            println!("{}", answer_text());
            ExitCode::SUCCESS
        }
        Command::Sims => {
            print!("{}", sims_report());
            ExitCode::SUCCESS
        }
        Command::Sim {
            id,
            set,
            width,
            height,
        } => {
            journey.play();
            emit(sim_run(&id, &set, width, height))
        }
        Command::Plot {
            expr,
            xmin,
            xmax,
            a,
            animate,
            amin,
            amax,
            width,
            height,
        } => {
            journey.play();
            if animate {
                // The loop never returns; persist the play before it starts.
                let _ = std::fs::write(journey_path(), journey.to_text());
                plot_animate(&expr, xmin, xmax, amin, amax, width, height)
            } else {
                emit(plot_report(&expr, xmin, xmax, a, width, height))
            }
        }
        Command::Sing {
            expr,
            xmin,
            xmax,
            notes,
            out,
        } => {
            journey.play();
            emit(sing_wav(&expr, xmin, xmax, notes, &out))
        }
    }
}

/// Animate a plot in the terminal, sweeping the parameter `a`, until interrupted.
fn plot_animate(
    source: &str,
    xmin: f64,
    xmax: f64,
    amin: f64,
    amax: f64,
    width: usize,
    height: usize,
) -> ExitCode {
    let frame_time = Duration::from_secs_f64(1.0 / 12.0);
    let mut stdout = std::io::stdout();
    let mut phase = 0.0_f64;
    loop {
        let a = amin + (amax - amin) * phase;
        match plot_report(source, xmin, xmax, a, width, height) {
            Ok(text) => {
                let _ = write!(
                    stdout,
                    "\x1b[2J\x1b[H{text}\na = {a:.3}   (Ctrl+C to stop)\n"
                );
                let _ = stdout.flush();
            }
            Err(message) => {
                eprint!("{message}");
                return ExitCode::FAILURE;
            }
        }
        std::thread::sleep(frame_time);
        phase = if phase + 0.02 >= 1.0 {
            0.0
        } else {
            phase + 0.02
        };
    }
}

/// Turn `source` into a melody over `[xmin, xmax]` and write it as a WAV.
fn sing_wav(
    source: &str,
    xmin: f64,
    xmax: f64,
    notes: usize,
    path: &Path,
) -> Result<String, String> {
    let expr = numinous_core::parse(source)?;
    if xmax <= xmin {
        return Err("need xmax > xmin\n".to_string());
    }
    let sample_rate = 44_100u32;
    let spec = numinous_core::to_melody(&expr, xmin, xmax, notes, 0.0);
    write_wav(path, &spec.render(sample_rate), sample_rate)?;
    Ok(format!(
        "wrote {} ({:.1}s, {} notes) from y = {source}\n",
        path.display(),
        spec.duration,
        spec.notes.len()
    ))
}

/// Plot `source` as y = f(x, a) over `[xmin, xmax]`, auto-scaling y.
fn plot_report(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<String, String> {
    let expr = numinous_core::parse(source)?;
    if width < 2 || height < 2 || xmax <= xmin {
        return Err("need width >= 2, height >= 2, and xmax > xmin\n".to_string());
    }
    let samples: Vec<(f64, f64)> = (0..width)
        .map(|i| {
            let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
            (x, numinous_core::eval(&expr, x, a))
        })
        .filter(|(_, y)| y.is_finite())
        .collect();
    if samples.is_empty() {
        return Err("nothing to plot: the function is undefined across this range\n".to_string());
    }
    let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let ymax = samples
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let yspan = (ymax - ymin).max(1e-9);

    let mut canvas = Canvas::new(width, height);
    let to_screen = |x: f64, y: f64| -> (i32, i32) {
        let sx = ((x - xmin) / (xmax - xmin) * (width as f64 - 1.0)) as i32;
        let sy = ((height as f64 - 1.0) - (y - ymin) / yspan * (height as f64 - 1.0)) as i32;
        (sx, sy)
    };
    let mut previous: Option<(i32, i32)> = None;
    for &(x, y) in &samples {
        let (sx, sy) = to_screen(x, y);
        if let Some((px, py)) = previous {
            canvas.line(px, py, sx, sy, '#');
        }
        previous = Some((sx, sy));
    }
    Ok(format!(
        "y = {source}    x in [{xmin:.3}, {xmax:.3}]    y in [{ymin:.3}, {ymax:.3}]\n\n{}",
        canvas.to_text()
    ))
}

/// The list of sims and their levers.
fn sims_report() -> String {
    let lines: Vec<String> = numinous_core::all_sims()
        .iter()
        .map(|sim| {
            let meta = sim.meta();
            let levers: Vec<String> = meta
                .levers
                .iter()
                .map(|l| format!("{}=[{}..{}] {}", l.name, l.min, l.max, l.unit))
                .collect();
            format!(
                "{:<12} {}\n  levers: {}",
                meta.id,
                meta.title,
                levers.join(", ")
            )
        })
        .collect();
    format!("{}\n", lines.join("\n\n"))
}

/// Render a sim with the given lever settings and return its picture and readout.
fn sim_run(id: &str, sets: &[String], width: usize, height: usize) -> Result<String, String> {
    let sim = numinous_core::sim_by_id(id)
        .ok_or_else(|| format!("no sim named '{id}'. Try: numinous sims\n"))?;
    let meta = sim.meta();
    let mut params = numinous_core::default_params(&meta);
    for entry in sets {
        let (name, value) = entry
            .split_once('=')
            .ok_or_else(|| format!("--set expects name=value, got '{entry}'\n"))?;
        let index = meta
            .levers
            .iter()
            .position(|l| l.name == name)
            .ok_or_else(|| format!("'{id}' has no lever '{name}'. Try: numinous sims\n"))?;
        params[index] = value
            .parse()
            .map_err(|_| format!("'{value}' is not a number\n"))?;
    }
    let mut canvas = Canvas::new(width, height);
    sim.render(&mut canvas, &params);
    Ok(format!("{}\n{}\n", canvas.to_text(), sim.readout(&params)))
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

/// The levels at which a room's first and second deep cuts unlock.
const CUT_LEVELS: [u32; 2] = [5, 12];

/// One room's description, or a guiding error if the id is unknown.
fn describe_report(id: &str, json: bool, allow_hidden: bool, level: u32) -> Result<String, String> {
    let Some(room) = find_room(id, allow_hidden) else {
        // Not every name in the world is a room. A few of them answer anyway,
        // and a few more answer only for those who have been listening a while.
        let whisper = numinous_core::akousma(id).or_else(|| {
            allow_hidden
                .then(|| numinous_core::deep_akousma(id))
                .flatten()
        });
        return match whisper {
            Some(whisper) => Ok(format!("{whisper}\n")),
            None => Err(not_found_message(id)),
        };
    };
    // The knowledge is the loot: deeper cuts unlock as the journey deepens.
    let mut cuts = String::new();
    let mut unlocked = Vec::new();
    for (i, cut) in room.deep_cuts().iter().enumerate() {
        let need = CUT_LEVELS.get(i).copied().unwrap_or(u32::MAX);
        if level >= need {
            let label = if i == 0 { "Deeper" } else { "Deeper still" };
            cuts.push_str(&format!("\n{label}: {cut}\n"));
            unlocked.push((*cut).to_string());
        } else {
            cuts.push_str(&format!("\nLOCKED: a deeper cut opens at LV {need}.\n"));
            break;
        }
    }
    let m = room.meta();
    Ok(if json {
        let mut value = meta_json(&m);
        value["reveal"] = serde_json::Value::String(room.reveal().to_string());
        value["deep_cuts"] = serde_json::Value::Array(
            unlocked
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        );
        format!("{}\n", to_pretty(&value))
    } else {
        format!(
            "{} ({})\nWing: {}\n\n{}\n\nReveal: {}\n{cuts}",
            m.title,
            m.id,
            m.wing,
            m.blurb,
            room.reveal()
        )
    })
}

/// A room rendered in truecolor ANSI (two pixels per terminal cell).
fn render_color_report(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    allow_hidden: bool,
) -> Result<String, String> {
    let room = find_room(id, allow_hidden).ok_or_else(|| not_found_message(id))?;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    Ok(numinous_core::to_ansi(&raster))
}

/// One truecolor frame of a room with a status line, for the watch loop.
fn watch_frame(room: &dyn Room, t: f64, width: usize, height: usize) -> String {
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    format!(
        "\x1b[H{}\x1b[0m{}  t = {t:.2}   (Ctrl+C to stop)\x1b[K\n",
        numinous_core::to_ansi(&raster),
        room.meta().title
    )
}

/// Watch a room in full color in the terminal, its sound playing, until
/// interrupted. The whole audiovisual experience with no window at all.
fn watch(
    id: &str,
    fps: f64,
    width: usize,
    height: usize,
    mute: bool,
    allow_hidden: bool,
) -> ExitCode {
    let Some(room) = find_room(id, allow_hidden) else {
        eprint!("{}", not_found_message(id));
        return ExitCode::FAILURE;
    };
    let player = if mute {
        None
    } else {
        numinous_audio::LoopPlayer::new().ok()
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let mut stdout = std::io::stdout();
    // Clear once; frames then repaint in place (no flicker).
    let _ = write!(stdout, "\x1b[2J");
    let mut t = 0.0f64;
    let mut frame = 0u64;
    loop {
        let _ = write!(stdout, "{}", watch_frame(room.as_ref(), t, width, height));
        let _ = stdout.flush();
        // Refresh the room's voice a few times per sweep.
        if frame % 24 == 0
            && let Some(player) = &player
        {
            let spec = room.sound(t);
            player.set_samples(spec.render(player.sample_rate()));
        }
        std::thread::sleep(frame_time);
        t = if t + 0.005 >= 1.0 { 0.0 } else { t + 0.005 };
        frame += 1;
    }
}

/// A room rendered to ASCII, or a guiding error if the id is unknown.
fn render_report(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    allow_hidden: bool,
) -> Result<String, String> {
    let room = find_room(id, allow_hidden).ok_or_else(|| not_found_message(id))?;
    let mut canvas = Canvas::new(width, height);
    room.render(&mut canvas, t);
    Ok(canvas.to_text())
}

/// Your constellation and standing, shown plainly and explained never.
fn journey_report(journey: &Journey) -> String {
    let mut wall = String::new();
    for &(level, name, what) in numinous_core::UNLOCKS {
        if journey.level() >= level {
            wall.push_str(&format!("  OPEN    LV {level:>2}  {name}: {what}\n"));
        } else {
            wall.push_str(&format!("  LOCKED  LV {level:>2}  ???\n"));
        }
    }
    format!(
        "LV {:>2}  [{}]  {} XP\n\n{}\n\n{} of {} stars lit. {} answered well. {} heard.\n{}\n\n{wall}",
        journey.level(),
        journey.level_bar(20),
        journey.sparks(),
        numinous_core::constellation(journey, 60, 18),
        journey.visited.len(),
        all_rooms().len(),
        journey.wins,
        journey.secrets,
        journey.rank().name()
    )
}

/// True (and says so) if `what` is still locked at this journey's level.
fn still_locked(journey: &Journey, need: u32, what: &str) -> bool {
    if journey.level() >= need {
        return false;
    }
    println!(
        "LOCKED. {what} opens at LV {need}. You are LV {}. Keep playing.",
        journey.level()
    );
    true
}

/// Render a room to a PNG image at `path`, returning a status message.
fn render_png(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    path: &Path,
    allow_hidden: bool,
) -> Result<String, String> {
    let room = find_room(id, allow_hidden).ok_or_else(|| not_found_message(id))?;
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
fn sonify_wav(id: &str, t: f64, path: &Path, allow_hidden: bool) -> Result<String, String> {
    let room = find_room(id, allow_hidden).ok_or_else(|| not_found_message(id))?;
    let spec = room.sound(t);
    let sample_rate = 44_100u32;
    write_wav(path, &spec.render(sample_rate), sample_rate)?;
    Ok(format!(
        "wrote {} ({:.1}s, {} notes)\n",
        path.display(),
        spec.duration,
        spec.notes.len()
    ))
}

/// Write mono 16-bit samples to a WAV file at `path`.
fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let wav_spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)
        .map_err(|e| format!("could not create {}: {e}", path.display()))?;
    for sample in samples {
        writer
            .write_sample((sample * f32::from(i16::MAX)) as i16)
            .map_err(|e| format!("wav write failed: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("wav finalize failed: {e}"))
}

/// Render every room to `<dir>/<id>.png`, returning a status message.
fn gallery(dir: &Path, width: usize, height: usize) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut count = 0usize;
    for room in all_rooms() {
        let id = room.meta().id;
        let path = dir.join(format!("{id}.png"));
        render_png(id, width, height, 0.0, &path, false)?;
        count += 1;
    }
    Ok(format!("wrote {count} room images to {}\n", dir.display()))
}

/// Play Crack the Code: defuse a math-clued bomb from stdin guesses.
fn crack(seed: u64, digits: usize, attempts: usize, journey: &mut Journey) -> ExitCode {
    journey.play();
    let secret = numinous_core::secret_code(seed, digits);
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    println!("A bomb. {digits} digits, {attempts} attempts before it blows.");
    println!("Clue: {}\n", numinous_core::hint(&secret));
    let mut attempt = 0usize;
    while attempt < attempts {
        print!("Wire {}/{attempts} > ", attempt + 1);
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let guess: Vec<u8> = line
            .trim()
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != digits {
            println!("  Enter exactly {digits} digits.");
            continue;
        }
        attempt += 1;
        let feedback = numinous_core::grade(&secret, &guess);
        if feedback.locked == digits {
            journey.win();
            println!(
                "\nDEFUSED with {} attempts to spare. You cracked it.",
                attempts - attempt
            );
            return ExitCode::SUCCESS;
        }
        println!("  {} locked, {} loose.", feedback.locked, feedback.loose);
    }
    let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
    println!("\nBOOM. The code was {code}.");
    ExitCode::FAILURE
}

/// Play SETI: scan channels of static and pick the artificial signal.
fn seti(seed: u64, channels: usize, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut score = 0usize;
    println!(
        "Listening near the hydrogen line. Nature makes rhythms; only minds count in primes.\n"
    );
    for round in 0..rounds {
        let scan = numinous_core::build_scan(seed.wrapping_add(round as u64), channels);
        journey.play();
        println!("Scan #{}:", round + 1);
        for channel in &scan.channels {
            println!(
                "  {})  {:>10}  |{}|",
                channel.letter, channel.frequency, channel.trace
            );
        }
        print!("Which channel is a transmission? ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let guess = line.trim().chars().next().map(|c| c.to_ascii_uppercase());
        if guess == Some(scan.answer) {
            score += 1;
            journey.win();
            println!(
                "Contact. {} at {} was counting in primes. That is not nature.\n",
                scan.answer, scan.answer_frequency
            );
        } else {
            println!(
                "Static. The signal was {} at {}, counting 2, 3, 5, 7, 11.\n",
                scan.answer, scan.answer_frequency
            );
        }
    }
    println!("You found {score}/{rounds}. Now open a channel and say hello: numinous aliens.");
    ExitCode::SUCCESS
}

/// Play Talk to the Aliens: continue the transmitted sequences from stdin.
fn aliens(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut score = 0usize;
    println!("A transmission. They speak only in numbers. Prove you understand.\n");
    for round in 0..rounds {
        let message = numinous_core::alien_message(seed.wrapping_add(round as u64), 5);
        journey.play();
        let shown: Vec<String> = message
            .terms
            .iter()
            .map(|&t| numinous_core::to_base(t, message.base))
            .collect();
        let base_note = if message.base == 10 {
            String::new()
        } else {
            format!(" (they count in base {})", message.base)
        };
        println!(
            "Signal #{}{}: {}, ...?",
            round + 1,
            base_note,
            shown.join(", ")
        );
        print!("The next number > ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let answer = numinous_core::to_base(message.answer, message.base);
        if u64::from_str_radix(line.trim(), message.base).ok() == Some(message.answer) {
            score += 1;
            journey.win();
            println!(
                "Contact. It was {answer} ({}).\n  {}\n",
                message.name, message.explanation
            );
        } else {
            println!(
                "Silence. It was {answer} ({}).\n  {}\n",
                message.name, message.explanation
            );
        }
    }
    println!("You understood {score}/{rounds} of their language.");
    ExitCode::SUCCESS
}

/// The seed to play with: the explicit one, or today's shared seed with
/// `--daily` (the same for every player on the same calendar day, UTC).
fn pick_seed(seed: u64, daily: bool) -> u64 {
    if daily {
        let days = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(0);
        println!("Daily challenge (day {days}). Everyone gets this one.\n");
        days
    } else {
        seed
    }
}

/// What waits at LV 42. It was never a red herring for you.
fn answer_text() -> &'static str {
    "42.\n\n\
     You knew that. What you know now that you did not at LV 1: 42 is the third \
     primary pseudoperfect number, the number of partitions of 10, the sum of the \
     first three odd cubes shifted by nothing at all, and the only number the \
     Order refuses to comment on. You were told it was a red herring. It was, \
     until you carried one and two and three and four all the way here.\n\n\
     The answer was the playing. Level cap reached. The math keeps going."
}

/// The jokes, listed or dissected.
fn jokes_report(index: Option<usize>) -> String {
    match index {
        Some(i) => match numinous_core::explain_joke(i) {
            Some(joke) => format!(
                "Specimen {i}: \"{}\"\nHabitat: {}.\nMechanism: {}\n",
                joke.text, joke.habitat, joke.mechanism
            ),
            None => format!(
                "No specimen {i}. There are {} catalogued jokes.\n",
                numinous_core::jokes().len()
            ),
        },
        None => {
            let mut lines =
                vec!["The catalogued jokes (a joke explained is a frog dissected):".to_string()];
            for (i, joke) in numinous_core::jokes().iter().enumerate() {
                lines.push(format!("  {i}: \"{}\"  ({})", joke.text, joke.habitat));
            }
            lines.push("Dissect one with: numinous jokes <index>\n".to_string());
            lines.join("\n")
        }
    }
}

/// A closing remark for a quiz score. Pure, so it is unit-tested.
fn quiz_remark(score: usize, rounds: usize) -> &'static str {
    if rounds == 0 {
        return "Play a round!";
    }
    match score * 100 / rounds {
        100 => "Flawless. You see the math behind the shape.",
        60..=99 => "Sharp eye.",
        _ => "The shapes are sneaky. Play again.",
    }
}

/// Play Munch: eat the numbers that fit the rule, round by round, scored.
fn munch(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut total = 0i64;
    println!("MUNCH. Eat by cell number, e.g. \"1 7 22\". Wrong bites cost you.\n");
    for round in 0..rounds {
        let board = numinous_core::build_board(seed, round as u64);
        journey.play();
        println!("Board {} of {rounds}: {}", round + 1, board.rule.describe());
        print!("{}", numinous_core::board_text(&board));
        print!("Your bites > ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let bites: Vec<usize> = line
            .split_whitespace()
            .filter_map(|w| w.parse::<usize>().ok())
            .filter(|&n| n >= 1)
            .map(|n| n - 1)
            .collect();
        let outcome = numinous_core::grade_munch(&board, &bites);
        if outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0 {
            journey.win();
            println!(
                "PERFECT. {} eaten, nothing wasted. +{} points.\n",
                outcome.hits, outcome.score
            );
        } else {
            println!(
                "{} eaten, {} bad bites, {} left behind. +{} points.\n",
                outcome.hits, outcome.bad_bites, outcome.left_behind, outcome.score
            );
        }
        total += outcome.score;
    }
    println!("Final score: {total} (seed {seed}). Beat that, or make an AI try.");
    ExitCode::SUCCESS
}

/// Play the interactive "guess the shape" quiz, reading guesses from stdin.
fn quiz(
    rounds: usize,
    seed: u64,
    width: usize,
    height: usize,
    choices: usize,
    journey: &mut Journey,
) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut score = 0usize;
    println!("Guess the shape. Name the math behind each mystery render.\n");
    for round in 0..rounds {
        let r = numinous_core::build_round_sized(seed, round as u64, width, height, choices);
        journey.play();
        println!("Mystery #{} of {rounds}:", round + 1);
        print!("{}", r.art);
        println!();
        for choice in &r.choices {
            println!("  {}) {}", choice.letter, choice.title);
        }
        print!("Your answer: ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let guess = line.trim().chars().next().map(|c| c.to_ascii_uppercase());
        if guess == Some(r.answer) {
            score += 1;
            journey.win();
            println!(
                "Correct! It is {}.\n  {}\n",
                r.answer_title, r.answer_reveal
            );
        } else {
            println!(
                "Not quite. It was {} ({}).\n  {}\n",
                r.answer, r.answer_title, r.answer_reveal
            );
        }
    }
    println!(
        "Final score: {score}/{rounds}. {}",
        quiz_remark(score, rounds)
    );
    ExitCode::SUCCESS
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
fn play(id: &str, fps: f64, width: usize, height: usize, allow_hidden: bool) -> ExitCode {
    let Some(room) = find_room(id, allow_hidden) else {
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
        let text = describe_report("times-tables", false, false, 1).expect("known room");
        assert!(text.contains("Number & Pattern"));
    }

    #[test]
    fn describe_json_carries_the_id() {
        let text = describe_report("times-tables", true, false, 1).expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value["id"], "times-tables");
    }

    #[test]
    fn describe_includes_the_reveal() {
        let text = describe_report("times-tables", false, false, 1).expect("known room");
        assert!(text.contains("Reveal:"));
        assert!(text.contains("Mandelbrot"));
    }

    #[test]
    fn describe_json_includes_the_reveal() {
        let text = describe_report("times-tables", true, false, 1).expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert!(
            value["reveal"]
                .as_str()
                .is_some_and(|s| s.contains("Mandelbrot"))
        );
    }

    #[test]
    fn describe_unknown_room_guides_the_user() {
        let err = describe_report("no-such-room", false, false, 1).expect_err("unknown room");
        assert!(err.contains("Known rooms"));
    }

    #[test]
    fn render_known_room_has_ink() {
        let text = render_report("times-tables", 40, 20, 0.0, false).expect("known room");
        assert!(text.contains('*'));
    }

    #[test]
    fn render_unknown_room_is_error() {
        assert!(render_report("no-such-room", 10, 10, 0.0, false).is_err());
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
        let message =
            super::render_png("times-tables", 64, 48, 0.0, &path, false).expect("render png");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "png should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn render_png_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_should_not_exist.png");
        assert!(super::render_png("no-such-room", 10, 10, 0.0, &path, false).is_err());
    }

    #[test]
    fn sonify_wav_writes_a_non_empty_file() {
        let mut path = std::env::temp_dir();
        path.push("numinous_cli_sonify_test.wav");
        let message = super::sonify_wav("lissajous", 0.0, &path, false).expect("sonify");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "wav should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sonify_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_no.wav");
        assert!(super::sonify_wav("no-such-room", 0.0, &path, false).is_err());
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
        assert!(super::render_png("times-tables", 8, 8, 0.0, bad, false).is_err());
    }

    #[test]
    fn sonify_to_an_unwritable_path_is_error() {
        let bad = std::path::Path::new("no_such_dir_zzz/x.wav");
        assert!(super::sonify_wav("lissajous", 0.0, bad, false).is_err());
    }

    #[test]
    fn the_hidden_room_answers_only_to_rank() {
        assert!(super::find_room("tetractys", false).is_none());
        assert!(super::find_room("tetractys", true).is_some());
        // Catalog rooms are open to everyone.
        assert!(super::find_room("lorenz", false).is_some());
        // The unready get the ordinary not-found, no special acknowledgment.
        let err = super::render_report("tetractys", 10, 10, 0.0, false).unwrap_err();
        assert!(err.contains("Known rooms"), "an ordinary miss: {err}");
        assert!(!err.contains("Order"), "nothing is given away");
        // The ready see the figure.
        let ok = super::render_report("tetractys", 30, 20, 0.0, true).expect("the figure");
        assert!(ok.contains('#'));
    }

    #[test]
    fn deep_cuts_unlock_with_level() {
        let low = super::describe_report("mandelbrot", false, false, 1).expect("describe");
        assert!(
            low.contains("LOCKED: a deeper cut opens at LV 5"),
            "got: {low}"
        );
        assert!(!low.contains("Shishikura"));
        let mid = super::describe_report("mandelbrot", false, false, 5).expect("describe");
        assert!(mid.contains("Deeper:"));
        assert!(mid.contains("LOCKED: a deeper cut opens at LV 12"));
        let high = super::describe_report("mandelbrot", false, false, 12).expect("describe");
        assert!(high.contains("Deeper still:") && high.contains("Shishikura"));
    }

    #[test]
    fn deep_whispers_require_standing() {
        assert!(super::describe_report("curtain", false, false, 1).is_err());
        let deep = super::describe_report("curtain", false, true, 10).expect("a deeper whisper");
        assert!(deep.contains("veil"), "got: {deep}");
    }

    #[test]
    fn journey_report_shows_the_sky_and_the_rank() {
        let mut journey = numinous_core::Journey::default();
        let fresh = super::journey_report(&journey);
        assert!(fresh.contains("0 of"));
        assert!(fresh.contains("Outsider"));
        journey.visit("lorenz");
        let one = super::journey_report(&journey);
        assert!(one.contains("1 of"));
        assert!(one.contains("Akousmatikos"));
        assert!(one.contains('#'), "a lit star");
    }

    #[test]
    fn pick_seed_honors_the_explicit_seed() {
        assert_eq!(super::pick_seed(7, false), 7);
        // The daily seed is a day count: small, positive, stable within a run.
        let daily = super::pick_seed(7, true);
        assert!(daily > 20_000 && daily < 40_000, "got {daily}");
        assert_eq!(super::pick_seed(7, true), daily);
    }

    #[test]
    fn jokes_report_lists_and_dissects() {
        let list = super::jokes_report(None);
        assert!(list.contains("frog"));
        let one = super::jokes_report(Some(0));
        assert!(one.contains("Mechanism:"));
        assert!(super::jokes_report(Some(999)).contains("No specimen"));
    }

    #[test]
    fn quiz_remark_scales_with_score() {
        assert!(super::quiz_remark(5, 5).contains("Flawless"));
        assert!(super::quiz_remark(4, 5).contains("Sharp"));
        assert!(super::quiz_remark(0, 5).contains("sneaky"));
        assert_eq!(super::quiz_remark(0, 0), "Play a round!");
    }

    #[test]
    fn sims_report_lists_the_sims_with_levers() {
        let out = super::sims_report();
        assert!(out.contains("tribbles"));
        assert!(out.contains("levers"));
    }

    #[test]
    fn sim_run_renders_and_reads_out() {
        let out = super::sim_run("wing", &["angle-of-attack=20".to_string()], 40, 12).expect("run");
        assert!(out.contains("STALL"), "got: {out}");
    }

    #[test]
    fn render_color_report_emits_truecolor() {
        let out =
            super::render_color_report("times-tables", 20, 20, 0.0, false).expect("color render");
        assert!(out.contains("\x1b[38;2;"), "has truecolor escapes");
        assert!(super::render_color_report("nope", 20, 20, 0.0, false).is_err());
    }

    #[test]
    fn watch_frame_paints_in_place_with_a_status_line() {
        let room = numinous_core::room_by_id("chaos-game").expect("room");
        let frame = super::watch_frame(room.as_ref(), 0.5, 24, 16);
        assert!(
            frame.starts_with("\x1b[H"),
            "repaints from home, no flicker"
        );
        assert!(frame.contains("Chaos Game"));
        assert!(frame.contains("t = 0.50"));
    }

    #[test]
    fn plot_report_draws_a_known_function() {
        let out = super::plot_report("x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
        assert!(out.contains("y = x"));
        assert!(out.contains('#'));
    }

    #[test]
    fn plot_report_uses_the_parameter() {
        // With a=0 the line is flat; with a large a it spans more, so ink differs.
        let flat = super::plot_report("a * x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
        let steep = super::plot_report("a * x", -1.0, 1.0, 5.0, 24, 8).expect("plot");
        assert_ne!(flat, steep);
    }

    #[test]
    fn plot_report_rejects_bad_input() {
        assert!(super::plot_report("sin(", -1.0, 1.0, 0.0, 24, 8).is_err());
        assert!(super::plot_report("x", 1.0, 1.0, 0.0, 24, 8).is_err()); // xmax not > xmin
    }

    #[test]
    fn sing_wav_writes_a_melody() {
        let path = std::env::temp_dir().join("numinous_sing_test.wav");
        let message = super::sing_wav("sin(x)", -3.0, 3.0, 16, &path).expect("sing");
        assert!(message.contains("wrote"));
        assert!(std::fs::metadata(&path).expect("file").len() > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sing_wav_rejects_a_bad_expression() {
        let path = std::env::temp_dir().join("numinous_sing_bad.wav");
        assert!(super::sing_wav("nope(", -1.0, 1.0, 8, &path).is_err());
    }

    #[test]
    fn describe_whispers_for_the_hidden_names() {
        let out = super::describe_report("hippasus", false, false, 1).expect("a whisper");
        assert!(out.to_lowercase().contains("sea"), "got: {out}");
        assert!(super::describe_report("not-a-room-nor-secret", false, false, 1).is_err());
    }

    #[test]
    fn sim_run_rejects_bad_input() {
        assert!(super::sim_run("nope", &[], 10, 10).is_err());
        assert!(super::sim_run("wing", &["nope=1".to_string()], 10, 10).is_err());
        assert!(super::sim_run("wing", &["angle-of-attack=abc".to_string()], 10, 10).is_err());
        assert!(super::sim_run("wing", &["missing-equals".to_string()], 10, 10).is_err());
    }
}
