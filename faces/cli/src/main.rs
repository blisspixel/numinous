//! The `numinous` command line: the terminal face of the headless core.
//!
//! See `docs/INTERFACES.md`. The CLI exposes the shared catalog, rooms, games,
//! progression, Studio, audio, rendering, export, and digital-mind play paths
//! without owning their domain logic.
//!
//! The command handlers are split into pure `*_report` functions that return the
//! text to emit, so they can be unit-tested without capturing stdout; `main`
//! stays a thin shell that prints and sets the exit code.

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufWriter, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use numinous_core::{
    CUT_LEVELS, Canvas, Journey, Rank, Raster, Room, RoomMeta, Surface, all_rooms, all_rooms_with,
    draw_text, hidden_room_by_id, room_by_id,
};

const MAX_STUDIO_IMPORT_BYTES: u64 = 8 * 1024;
const MAX_ENV_FILE_BYTES: u64 = 16 * 1024;
const MAX_CLI_RENDER_WIDTH: usize = 4096;
const MAX_CLI_RENDER_HEIGHT: usize = 4096;
const MAX_CLI_RENDER_PIXELS: usize = 16 * 1024 * 1024;
const MAX_CLI_INPUT_BYTES: usize = 4 * 1024;
const ELEVENLABS_MUSIC_URL: &str = "https://api.elevenlabs.io/v1/music?output_format=pcm_44100";
type ParsedRoomInputs = (Vec<(f64, f64)>, Vec<numinous_core::RoomInput>);

#[derive(Parser)]
#[command(
    name = "numinous",
    version,
    about = "Numinous: math you can feel (CLI face)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
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
        /// Visual era for color output: phosphor, 8bit, vector, or modern.
        #[arg(long, default_value = "modern")]
        era: String,
        /// Choose and print a fresh room variation. Use --variation to replay it.
        #[arg(long, conflicts_with = "variation")]
        vary: bool,
        /// Use this exact room variation seed (default 0).
        #[arg(long, value_name = "SEED", conflicts_with = "vary")]
        variation: Option<u64>,
        /// Add a normalized hand point, as x,y in [0,1]. Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first; held rooms pin, pull, and fling. In Life, a
        /// down earlier than --t shows the glider's later evolution; its newest
        /// 24 down events become launches. Not combinable with --poke.
        #[arg(long = "gesture")]
        gestures: Vec<String>,
    },
    /// Export a short looping APNG of one phase cycle (Share v1 motion path).
    Loop {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Where to write the looping APNG.
        #[arg(long)]
        out: PathBuf,
        /// Frame edge in pixels (square). Default matches the App short loop.
        #[arg(long, default_value_t = 480)]
        size: usize,
        /// Starting phase in [0, 1); the loop sweeps one full unit from here.
        #[arg(long, default_value_t = 0.0)]
        t: f64,
        /// Visual era: phosphor, 8bit, vector, or modern.
        #[arg(long, default_value = "modern")]
        era: String,
        /// Use this exact room variation seed (default 0).
        #[arg(long, default_value_t = 0)]
        variation: u64,
        /// Add a normalized hand point, as x,y in [0,1]. Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first. Not combinable with --poke.
        #[arg(long = "gesture")]
        gestures: Vec<String>,
    },
    /// The Show for the terminal: every room in turn, full color, sound.
    Tour {
        /// Frames per second.
        #[arg(long, default_value_t = 30.0)]
        fps: f64,
        /// Frame width in pixels (two pixels per character row).
        #[arg(long, default_value_t = 100)]
        width: usize,
        /// Frame height in pixels.
        #[arg(long, default_value_t = 62)]
        height: usize,
        /// Silence, for late nights.
        #[arg(long)]
        mute: bool,
        /// A visual era for the whole tour (phosphor, 8bit, vector, modern).
        #[arg(long, default_value = "modern")]
        era: String,
        /// Seconds each room holds the stage.
        #[arg(long, default_value_t = 12.0)]
        seconds: f64,
    },
    /// The Bench: five fixed gauntlets, one composite number. Compare minds.
    Bench,
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
        /// Visual era: phosphor, 8bit, vector, or modern.
        #[arg(long, default_value = "modern")]
        era: String,
        /// Re-deal variation seed for replayable rooms (per-visit novelty, R in app).
        #[arg(long)]
        vary: bool,
    },
    /// Render a mathematical sonification or the stable App room bed to WAV.
    Sonify {
        /// Room id, e.g. "lissajous".
        id: String,
        /// Phase in [0, 1).
        #[arg(long, default_value_t = 0.0)]
        t: f64,
        /// Audio layer: input-aware mathematical sound or the stable App room bed.
        #[arg(long, value_enum, default_value = "mathematical")]
        layer: SonifyLayer,
        /// Replay this exact room variation seed (default 0).
        #[arg(long, default_value_t = 0)]
        variation: u64,
        /// Write a WAV audio file to this path.
        #[arg(long)]
        out: PathBuf,
        /// Add a normalized hand point, as x,y in [0,1]. Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first. Not combinable with --poke.
        #[arg(long = "gesture")]
        gestures: Vec<String>,
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
        /// What to play: a game (munch, quiz, nim, crack, seti, aliens,
        /// gauntlet, bench) or a room id to animate. Nothing lists the games.
        id: Option<String>,
        /// Frames per second.
        #[arg(long, default_value_t = 12.0)]
        fps: f64,
        /// Canvas width in columns.
        #[arg(long, default_value_t = 80)]
        width: usize,
        /// Canvas height in rows.
        #[arg(long, default_value_t = 36)]
        height: usize,
        /// Re-deal variation for room play (not games).
        #[arg(long)]
        vary: bool,
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
    /// Spend a banked boon: pick one of three deep cuts to open early.
    Choose,
    /// The high-score table: best runs across every game.
    Scores,
    /// The trophy case: what you have earned, and the silhouettes you have not.
    Trophies,
    /// See everything Numinous remembers about you; erase it with --confirm.
    Forget {
        /// Actually erase the journey (without this, just show what is kept).
        #[arg(long)]
        confirm: bool,
        /// Also erase the score table.
        #[arg(long)]
        scores: bool,
    },
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
        #[arg(long, default_value_t = 7)]
        rounds: usize,
    },
    /// The Munch arcade: eat what fits while the Vexations hunt you.
    Arcade {
        /// Seed (the same seed is the same run).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Run today's shared arcade.
        #[arg(long)]
        daily: bool,
    },
    /// Hackenbush: cut grass against the Order. The grass is made of numbers.
    Hackenbush {
        /// Seed (the same seed grows the same garden).
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// The Party Problem: avoid a one-color triangle. Five escape; six never.
    Party,
    /// Fifteen's Bet: solvable or stuck forever? Learn to smell parity.
    Fifteen {
        /// Seed (the same seed deals the same scrambles).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// How many scrambles to call.
        #[arg(long, default_value_t = 5)]
        rounds: u64,
    },
    /// Nim: three heaps against the Order. Lose, learn, become unbeatable.
    Nim {
        /// Seed (the same seed is the same heaps).
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// The Gauntlet: one run, four games, a combo, one number at the end.
    Gauntlet {
        /// Seed (the same seed is the same run, for anyone).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Run today's shared gauntlet.
        #[arg(long)]
        daily: bool,
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
        /// Save this Studio expression as a portable .num file and print its link.
        #[arg(long)]
        save: Option<PathBuf>,
    },
    /// Open a Studio .num file or numinous://studio link and render it.
    #[command(name = "open-studio")]
    OpenStudio {
        /// Path to a .num file, or a numinous://studio?... link.
        input: String,
        /// Plot width in columns.
        #[arg(long, default_value_t = 72)]
        width: usize,
        /// Plot height in rows.
        #[arg(long, default_value_t = 24)]
        height: usize,
    },
    /// The radio (Music Engine B): list the stations on the dial.
    Radio,
    /// Tune a station: generate a track from its brief via ElevenLabs Music.
    /// Needs ELEVENLABS_API_KEY. Tracks cache to ~/.numinous-radio/.
    Tune2 {
        /// The station id (see: numinous radio).
        station: String,
        /// Override track length in seconds (10 to 600). By default each
        /// track gets its card's natural runtime, varied like real radio.
        #[arg(long)]
        seconds: Option<u64>,
        /// How many tracks to add to the station's playlist (each is one
        /// paid API call; briefs vary per track).
        #[arg(long, default_value_t = 1)]
        count: usize,
    },
    /// Compose a seeded chiptune and write it as a WAV (Music Engine A).
    Tune {
        /// Seed (the same seed is the same tune, forever).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Length in bars of eight steps.
        #[arg(long, default_value_t = 8)]
        bars: usize,
        /// Write the WAV here.
        #[arg(long)]
        out: PathBuf,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SonifyLayer {
    /// Phase and hand controlled mathematical snapshot, mono at 44.1 kHz.
    Mathematical,
    /// Stable pre-master App room bed, stereo at the shared 16 kHz source rate.
    RoomBed,
}

fn main() -> ExitCode {
    run_on_command_stack(cli_main)
}

fn run_on_command_stack(task: impl FnOnce() -> ExitCode + Send + 'static) -> ExitCode {
    match std::thread::Builder::new()
        .name("numinous-cli".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(task)
    {
        Ok(worker) => match worker.join() {
            Ok(code) => code,
            Err(_) => {
                eprintln!("The command stopped unexpectedly.");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("Could not start the command worker: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Parse and execute the complete command surface on a bounded explicit stack.
///
/// The derived parser contains the full game and creation command catalog.
/// Windows' small process-entry stack is not a stable budget for that parser,
/// so the public entry point gives it one explicit fixed allocation.
fn cli_main() -> ExitCode {
    let mut journey = load_journey();
    let before = journey.clone();
    let earned_before = earned_names(&before, &load_scores());
    let code = match Cli::parse().command {
        Some(command) => run(command, &mut journey),
        None => home(&journey),
    };
    finish_journey(&before, &journey, &earned_before);
    code
}

/// The front door: what `numinous`, alone, opens onto. Today's room in full
/// color, who you are, and the handful of verbs that matter.
fn home(journey: &Journey) -> ExitCode {
    print!("{}", home_report(journey, std::io::stdout().is_terminal()));
    ExitCode::SUCCESS
}

fn home_report(journey: &Journey, stdout_is_terminal: bool) -> String {
    let rooms = all_rooms();
    let day = pick_day();
    let room = &rooms[(day as usize) % rooms.len()];
    if !stdout_is_terminal {
        return format!(
            concat!(
                "NUMINOUS: math you can feel\n",
                "Today's room: {} ({})\n",
                "\n",
                "Try:\n",
                "  numinous watch {:<12} watch today's room live\n",
                "  numinous rooms             browse the complete catalog\n",
                "  numinous play              choose a game\n",
                "  numinous tour --mute       sit back for the full visual Show\n",
                "  numinous journey           see your constellation\n",
                "  numinous --help            list every command\n",
                "\n",
                "Window version: numinous-app\n",
            ),
            room.meta().title,
            room.meta().wing,
            room.meta().id
        );
    }

    let mut raster = Raster::with_accent(72, 44, room.meta().accent);
    room.render(&mut raster, room.postcard_t());
    format!(
        concat!(
            "{}\x1b[0m{}  ({})\n",
            "\n",
            ". . . {}\n",
            "\n",
            "NUMINOUS   LV {:>2}  [{}]{}\n",
            "\n",
            "  numinous play              pick a game (munch, quiz, nim, the gauntlet...)\n",
            "  numinous play munch        or name one and go (fresh deal; --daily on its own command)\n",
            "  numinous tour              sit back: every room, full color, narrated\n",
            "  numinous watch {:<12} any one room, live, with its sound\n",
            "  numinous radio             the music stations (Y in the app tunes them)\n",
            "  numinous journey           your constellation, level, locks, resonances\n",
            "  numinous rooms             the whole catalog; describe <room> for the story\n",
            "\n",
            "Everything answers --help. The window version is numinous-app.\n",
        ),
        numinous_core::to_ansi(&raster),
        room.meta().title,
        room.meta().wing,
        numinous_core::insight(day + u64::from(journey.plays)),
        journey.level(),
        journey.level_bar(16),
        if journey.streak > 1 {
            format!("   streak {}", journey.streak)
        } else {
            String::new()
        },
        room.meta().id
    )
}

/// A fresh seed for casual play: different every deal, printed by the game
/// so any board can be replayed or shared (numinous crack --seed N).
fn fresh_seed() -> u64 {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(1)
        % 1_000_000;
    println!("(seed {seed}: replay or share any game with --seed)");
    clear_screen_soon();
    seed
}

/// Fresh variation for `render --vary`: different every deal for replayable rooms.
fn fresh_variation_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() ^ d.subsec_nanos() as u64)
        .unwrap_or(42)
}

fn parse_poke_arg(raw: &str) -> Result<(f64, f64), String> {
    let Some((x, y)) = raw.split_once(',') else {
        return Err(format!(
            "Bad --poke '{raw}'. Use normalized coordinates like --poke 0.4,0.6.\n"
        ));
    };
    let x = x
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Bad --poke '{raw}'. The x coordinate must be a number.\n"))?;
    let y = y
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Bad --poke '{raw}'. The y coordinate must be a number.\n"))?;
    if x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
        Ok((x, y))
    } else {
        Err(format!(
            "Bad --poke '{raw}'. Coordinates must be finite numbers in [0,1].\n"
        ))
    }
}

/// Parse one --gesture value: `down:x,y,t`, `move:x,y,t`, `up:x,y,t`, or
/// `cancel`, with finite coordinates in [0,1].
fn parse_gesture_arg(raw: &str) -> Result<numinous_core::RoomInput, String> {
    if raw == "cancel" {
        return Ok(numinous_core::RoomInput::PointerCancel);
    }
    let Some((kind, coords)) = raw.split_once(':') else {
        return Err(format!(
            "Bad --gesture '{raw}'. Use down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
"
        ));
    };
    let parts: Vec<&str> = coords.split(',').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Bad --gesture '{raw}'. Pointer events need x,y,t like down:0.3,0.4,0.1.
"
        ));
    }
    let mut values = [0.0_f64; 3];
    for (slot, part) in values.iter_mut().zip(&parts) {
        let value: f64 = part.trim().parse().map_err(|_| {
            format!(
                "Bad --gesture '{raw}'. Coordinates must be numbers.
"
            )
        })?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "Bad --gesture '{raw}'. Coordinates must be finite numbers in [0,1].
"
            ));
        }
        *slot = value;
    }
    let (x, y, t) = (values[0], values[1], values[2]);
    match kind {
        "down" => Ok(numinous_core::RoomInput::PointerDown { x, y, t }),
        "move" => Ok(numinous_core::RoomInput::PointerMove { x, y, t }),
        "up" => Ok(numinous_core::RoomInput::PointerUp { x, y, t }),
        other => Err(format!(
            "Bad --gesture '{raw}'. Pointer kinds are down, move, and up; cancel takes no coordinates; got '{other}'.
"
        )),
    }
}

fn parse_gestures(raw: &[String]) -> Result<Vec<numinous_core::RoomInput>, String> {
    if raw.len() > numinous_core::MAX_ROOM_INPUTS {
        return Err(format!(
            "Too many --gesture events: got {}, maximum is {}.
",
            raw.len(),
            numinous_core::MAX_ROOM_INPUTS
        ));
    }
    let events: Vec<numinous_core::RoomInput> = raw
        .iter()
        .map(|event| parse_gesture_arg(event))
        .collect::<Result<_, _>>()?;
    let mut last_t = None;
    for event in &events {
        let t = match event {
            numinous_core::RoomInput::PointerDown { t, .. }
            | numinous_core::RoomInput::PointerMove { t, .. }
            | numinous_core::RoomInput::PointerUp { t, .. } => Some(*t),
            _ => None,
        };
        if let Some(t) = t {
            if last_t.is_some_and(|previous| t < previous) {
                return Err(format!(
                    "Gesture timestamps must be nondecreasing; {t} came after {}.\n",
                    last_t.unwrap_or(t)
                ));
            }
            last_t = Some(t);
        }
    }
    Ok(events)
}

fn validate_render_dimensions(width: usize, height: usize) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("Render width and height must both be positive.\n".to_string());
    }
    if width > MAX_CLI_RENDER_WIDTH || height > MAX_CLI_RENDER_HEIGHT {
        return Err(format!(
            "Render size {width}x{height} exceeds the CLI limit of {}x{}.\n",
            MAX_CLI_RENDER_WIDTH, MAX_CLI_RENDER_HEIGHT
        ));
    }
    if width.saturating_mul(height) > MAX_CLI_RENDER_PIXELS {
        return Err(format!(
            "Render size {width}x{height} exceeds the {}-pixel allocation limit.\n",
            MAX_CLI_RENDER_PIXELS
        ));
    }
    Ok(())
}

fn validate_render_request(width: usize, height: usize, t: f64) -> Result<(), String> {
    validate_render_dimensions(width, height)?;
    if !t.is_finite() || !(0.0..1.0).contains(&t) {
        return Err(format!(
            "Render phase must be a finite number in [0,1); got {t}.\n"
        ));
    }
    Ok(())
}

fn parse_pokes(raw: &[String]) -> Result<Vec<(f64, f64)>, String> {
    if raw.len() > numinous_core::MAX_ROOM_POKES {
        return Err(format!(
            "Too many --poke values: got {}, maximum is {}.\n",
            raw.len(),
            numinous_core::MAX_ROOM_POKES
        ));
    }
    raw.iter().map(|poke| parse_poke_arg(poke)).collect()
}

fn parse_room_inputs(
    raw_pokes: &[String],
    raw_gestures: &[String],
) -> Result<ParsedRoomInputs, String> {
    let pokes = parse_pokes(raw_pokes)?;
    let gestures = parse_gestures(raw_gestures)?;
    if !pokes.is_empty() && !gestures.is_empty() {
        return Err(
            "Use either --poke (static hand points) or --gesture (a pointer trail), not both.\n"
                .to_string(),
        );
    }
    Ok((pokes, gestures))
}

#[derive(Clone, Copy)]
struct RoomRenderInput<'a> {
    variation: u64,
    pokes: &'a [(f64, f64)],
    gesture: &'a [numinous_core::RoomInput],
}

impl<'a> RoomRenderInput<'a> {
    fn new(variation: u64, pokes: &'a [(f64, f64)]) -> Self {
        Self {
            variation,
            pokes,
            gesture: &[],
        }
    }

    fn with_gesture(variation: u64, gesture: &'a [numinous_core::RoomInput]) -> Self {
        Self {
            variation,
            pokes: &[],
            gesture,
        }
    }

    fn has_interaction(self) -> bool {
        !self.pokes.is_empty() || !self.gesture.is_empty()
    }
}

impl RoomRenderInput<'static> {
    fn plain() -> Self {
        Self {
            variation: 0,
            pokes: &[],
            gesture: &[],
        }
    }
}

fn visible_status(room: &dyn Room, t: f64, input: RoomRenderInput<'_>) -> Option<String> {
    let base = room.status(t);
    if !input.has_interaction() {
        return base;
    }
    if !input.gesture.is_empty() {
        room.status_input(t, input.gesture)
    } else {
        let inputs = numinous_core::inputs_from_pokes(input.pokes, t);
        room.status_input(t, &inputs)
    }
    .or(base)
}

/// Clear the screen so a game owns a clean console.
fn clear_screen_soon() {
    print!("[2J[H");
    let _ = std::io::stdout().flush();
}

/// Days since the epoch: the shared daily clock.
fn pick_day() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(1)
}

/// The names of the trophies currently earned, for before/after comparison.
fn earned_names(
    journey: &Journey,
    board: &numinous_core::Scoreboard,
) -> std::collections::BTreeSet<&'static str> {
    numinous_core::trophies(journey, board)
        .into_iter()
        .filter(|t| t.earned)
        .map(|t| t.name)
        .collect()
}

/// The ping lines for trophies earned since `before`. Pure, so it is tested.
fn trophy_pings(
    before: &std::collections::BTreeSet<&'static str>,
    journey: &Journey,
    board: &numinous_core::Scoreboard,
) -> Vec<String> {
    numinous_core::trophies(journey, board)
        .into_iter()
        .filter(|t| t.earned && !before.contains(t.name))
        .map(|t| format!("TROPHY EARNED  {}: {}", t.name, t.what))
        .collect()
}

/// Where the journey file lives: `NUMINOUS_JOURNEY` if set, else the home
/// directory, else the current directory.
#[cfg(test)]
struct TestStateRoot {
    path: PathBuf,
}

#[cfg(test)]
impl TestStateRoot {
    fn new() -> Self {
        use std::hash::{Hash, Hasher};

        let thread = std::thread::current();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread.id().hash(&mut hasher);
        thread.name().hash(&mut hasher);
        let path = std::env::temp_dir().join(format!(
            "numinous-cli-test-{}-{:016x}",
            std::process::id(),
            hasher.finish()
        ));
        Self::at(path)
    }

    fn at(path: PathBuf) -> Self {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear test state directory: {error}"),
        }
        std::fs::create_dir_all(&path).expect("test state directory should be writable");
        Self { path }
    }
}

#[cfg(test)]
impl Drop for TestStateRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
std::thread_local! {
    static TEST_STATE_ROOT: TestStateRoot = TestStateRoot::new();
}

#[cfg(test)]
fn test_state_path(kind: &str) -> PathBuf {
    TEST_STATE_ROOT.with(|root| root.path.join(format!("{kind}.txt")))
}

fn journey_path() -> PathBuf {
    #[cfg(test)]
    {
        test_state_path("journey")
    }
    #[cfg(not(test))]
    {
        if let Ok(path) = std::env::var("NUMINOUS_JOURNEY") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".numinous-journey")
    }
}

/// Load the journey, or start a fresh one.
fn load_journey() -> Journey {
    numinous_core::load_journey_file(&journey_path())
}

/// Where the high-score table lives: `NUMINOUS_SCORES` if set, else home.
fn scores_path() -> PathBuf {
    #[cfg(test)]
    {
        test_state_path("scores")
    }
    #[cfg(not(test))]
    {
        if let Ok(path) = std::env::var("NUMINOUS_SCORES") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".numinous-scores")
    }
}

/// Load the high-score table, or start a fresh one.
fn load_scores() -> numinous_core::Scoreboard {
    numinous_core::load_scoreboard_file(&scores_path())
}

/// Record a score; announce and persist when a record falls.
fn post_score(key: &str, score: i64) {
    if numinous_core::record_score_file(&scores_path(), key, score).unwrap_or(false) {
        println!("NEW BEST: {key} = {score}");
    }
}

/// The trophy case, arcade style: earned trophies shine, the rest are
/// silhouettes with their conditions showing, so the case begs to be filled.
fn trophies_report(journey: &Journey, board: &numinous_core::Scoreboard) -> String {
    let case = numinous_core::trophies(journey, board);
    let earned = case.iter().filter(|t| t.earned).count();
    let mut out = format!(
        "TROPHIES  {earned} of {}

",
        case.len()
    );
    for trophy in &case {
        if trophy.earned {
            out.push_str(&format!(
                "  [#] {:<24} {}
",
                trophy.name, trophy.what
            ));
        } else {
            out.push_str(&format!(
                "  [ ] {:<24} {}
",
                "???", trophy.what
            ));
        }
    }
    out
}

/// The table, arcade style.
fn scores_report(board: &numinous_core::Scoreboard) -> String {
    if board.entries.is_empty() {
        return "No scores yet. Play something: munch, quiz, seti, aliens, crack.
"
        .to_string();
    }
    let mut out = String::from(
        "HIGH SCORES

",
    );
    for (rank, (key, score)) in board.top(15).iter().enumerate() {
        out.push_str(&format!(
            "  {:>2}.  {score:>6}  {key}
",
            rank + 1
        ));
    }
    out
}

/// The level-up banner: the new level, its lore line, and what unlocked.
/// Pure, so it is tested. Unironic and funny are the same thing here.
fn level_up_report(before: &Journey, after: &Journey) -> Option<String> {
    if after.level() <= before.level() {
        return None;
    }
    let level = after.level();
    let mut out = format!("LEVEL UP  LV {level:>2}  [{}]", after.level_bar(20));
    let lore = numinous_core::journey::level_lore(level);
    if !lore.is_empty() {
        out.push_str(&format!("\n{lore}"));
    }
    if after.boons_available() > 0 {
        out.push_str("\nBOON BANKED  choose what opens early: numinous choose");
    }
    for &(need, name, what) in numinous_core::UNLOCKS {
        if need > before.level() && need <= level {
            out.push_str(&format!("\nUNLOCKED  {name}: {what}"));
        }
    }
    Some(out)
}

/// Persist the journey if it changed; announce level-ups (the RPG speaks),
/// and whisper once if a rank was crossed (the Order murmurs).
fn finish_journey(
    before: &Journey,
    after: &Journey,
    earned_before: &std::collections::BTreeSet<&'static str>,
) {
    if before == after {
        return;
    }
    let saved = numinous_core::persist_journey_delta(&journey_path(), before, after)
        .unwrap_or_else(|_| after.clone());
    for ping in trophy_pings(earned_before, &saved, &load_scores()) {
        println!(
            "
{ping}"
        );
    }
    if let Some(banner) = level_up_report(before, &saved) {
        println!("\n{banner}");
    }
    if saved.rank() > before.rank() {
        println!("\n{}", saved.rank().whisper());
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

/// Find a room for commands that may request per-visit variation. Variation
/// only applies to catalog rooms; hidden rooms still answer after rank checks.
fn find_room_with_variation(id: &str, allow_hidden: bool, variation: u64) -> Option<Box<dyn Room>> {
    if variation == 0 {
        return find_room(id, allow_hidden);
    }
    all_rooms_with(variation)
        .into_iter()
        .find(|room| room.meta().id == id)
        .or_else(|| find_room(id, allow_hidden))
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
            let report = describe_report(&id, json, allow_hidden, journey);
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
            era,
            vary,
            variation,
            pokes,
            gestures,
        } => {
            if let Err(message) = validate_render_request(width, height, t) {
                eprint!("{message}");
                return ExitCode::FAILURE;
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!("Unknown era '{era}'. Eras: phosphor, 8bit, vector, modern.");
                return ExitCode::FAILURE;
            };
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    eprint!("{message}");
                    return ExitCode::FAILURE;
                }
            };
            let variation = variation.unwrap_or_else(|| {
                if vary {
                    let variation = fresh_variation_seed();
                    eprintln!("Variation {variation}: replay with --variation {variation}");
                    variation
                } else {
                    0
                }
            });
            let input = if gesture.is_empty() {
                RoomRenderInput::new(variation, &pokes)
            } else {
                RoomRenderInput::with_gesture(variation, &gesture)
            };
            let report = match out {
                Some(path) => render_png(&id, width, height, t, &path, allow_hidden, era, input),
                None if color => {
                    render_color_report(&id, width, height, t, allow_hidden, era, input)
                }
                None => render_report(&id, width, height, t, allow_hidden, input),
            };
            if report.is_ok() && find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
            }
            emit(report)
        }
        Command::Loop {
            id,
            out,
            size,
            t,
            era,
            variation,
            pokes,
            gestures,
        } => {
            if let Err(message) = validate_render_request(size, size, t) {
                eprint!("{message}");
                return ExitCode::FAILURE;
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!("Unknown era '{era}'. Eras: phosphor, 8bit, vector, modern.");
                return ExitCode::FAILURE;
            };
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    eprint!("{message}");
                    return ExitCode::FAILURE;
                }
            };
            let input = if gesture.is_empty() {
                RoomRenderInput::new(variation, &pokes)
            } else {
                RoomRenderInput::with_gesture(variation, &gesture)
            };
            let report = render_loop_apng(&id, size, t, &out, allow_hidden, era, input);
            if report.is_ok() && find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
            }
            emit(report)
        }
        Command::Tour {
            fps,
            width,
            height,
            mute,
            era,
            seconds,
        } => {
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!("Unknown era '{era}'. Eras: phosphor, 8bit, vector, modern.");
                return ExitCode::FAILURE;
            };
            tour(fps, width, height, mute, era, seconds, journey)
        }
        Command::Bench => bench(journey),
        Command::Watch {
            id,
            fps,
            width,
            height,
            mute,
            era,
            vary,
        } => {
            if find_room(&id, allow_hidden).is_some() {
                let before = journey.clone();
                journey.visit(&id);
                // The loop never returns; persist the visit before it starts.
                let _ = numinous_core::persist_journey_delta(&journey_path(), &before, journey);
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!("Unknown era '{era}'. Eras: phosphor, 8bit, vector, modern.");
                return ExitCode::FAILURE;
            };
            let variation = if vary { fresh_variation_seed() } else { 0 };
            watch(&id, fps, width, height, mute, allow_hidden, era, variation)
        }
        Command::Sonify {
            id,
            t,
            layer,
            variation,
            out,
            pokes,
            gestures,
        } => {
            if let Err(message) = validate_render_request(1, 1, t) {
                eprint!("{message}");
                return ExitCode::FAILURE;
            }
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    eprint!("{message}");
                    return ExitCode::FAILURE;
                }
            };
            if layer == SonifyLayer::RoomBed
                && (t != 0.0 || !pokes.is_empty() || !gesture.is_empty())
            {
                eprintln!(
                    "The stable room bed does not use --t, --poke, or --gesture. Omit those controls, or use --layer mathematical for the input-aware sound."
                );
                return ExitCode::FAILURE;
            }
            let input = if gesture.is_empty() {
                RoomRenderInput::new(variation, &pokes)
            } else {
                RoomRenderInput::with_gesture(variation, &gesture)
            };
            let result = sonify_wav_layer(&id, t, &out, allow_hidden, input, layer);
            if result.is_ok() && find_room_with_variation(&id, allow_hidden, variation).is_some() {
                journey.visit(&id);
            }
            emit(result)
        }
        Command::Gallery { dir, width, height } => emit(gallery(&dir, width, height)),
        Command::ContactSheet { out, cols, tile } => emit(contact_sheet(&out, cols, tile)),
        Command::Play {
            id,
            fps,
            width,
            height,
            vary,
        } => {
            let Some(id) = id else {
                println!(
                    "Pick a game:\n
  numinous play munch        a board of numbers, one rule; eat what fits, skip what lies
  numinous play quiz         see a shape, name the math that made it (multiple choice)
  numinous play nim          take stones, last stone wins; beat the Order, earn its secret
  numinous play crack        guess the code; LOCKED right place, LOOSE right digit wrong place
  numinous play seti         radio channels of static; only a mind counts in primes
  numinous play aliens       they send a number sequence; answer the next, in THEIR base
  numinous play arcade       the Munch arcade: eat what fits while spirits hunt you
  numinous play hackenbush   cut grass vs the Order; the grass is secretly made of numbers
  numinous play party        shade handshakes, dodge triangles; five escape, six never
  numinous play fifteen      call each scramble solvable or stuck; parity is the tell
  numinous play gauntlet     one run through four of the above; clean stages build a combo
  numinous play bench        five fixed gauntlets, one composite number: compare any two minds
\nAdd --daily on a game's own command for the shared seed (numinous munch --daily).
Or name a room to watch it as ASCII: numinous play lorenz"
                );
                return ExitCode::SUCCESS;
            };
            let seed = fresh_seed();
            match id.as_str() {
                "munch" => munch(seed, 3, journey),
                "quiz" => quiz(3, seed, 44, 18, 4, journey),
                "nim" => nim(seed, journey),
                "arcade" => arcade(seed, journey),
                "hackenbush" => hackenbush(seed, journey),
                "party" => party(journey),
                "fifteen" => fifteen(seed, 5, journey),
                "crack" => crack(seed, 4, 8, journey),
                "seti" => seti(seed, 4, 3, journey),
                "aliens" => aliens(seed, 3, journey),
                "gauntlet" => gauntlet(seed, journey),
                "bench" => bench(journey),
                _ => {
                    if find_room(&id, allow_hidden).is_some() {
                        let before = journey.clone();
                        journey.visit(&id);
                        let _ =
                            numinous_core::persist_journey_delta(&journey_path(), &before, journey);
                    }
                    let variation = if vary { fresh_variation_seed() } else { 0 };
                    play(&id, fps, width, height, allow_hidden, variation)
                }
            }
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
                pick_seed(seed, daily, journey),
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
            print!("{}", journey_report(journey, &load_scores()));
            ExitCode::SUCCESS
        }
        Command::Choose => choose(journey),
        Command::Scores => {
            print!("{}", scores_report(&load_scores()));
            ExitCode::SUCCESS
        }
        Command::Trophies => {
            print!("{}", trophies_report(journey, &load_scores()));
            ExitCode::SUCCESS
        }
        Command::Forget { confirm, scores } => {
            if !confirm {
                println!(
                    "Everything Numinous remembers about you:

  journey  {} rooms entered, {} wins, {} plays, {} secrets heard
  scores   {} entries

That is all of it. Nothing else is kept, sent, or shared.
Erase the journey with: numinous forget --confirm  (add --scores for the table)",
                    journey.visited.len(),
                    journey.wins,
                    journey.plays,
                    journey.secrets,
                    load_scores().entries.len()
                );
                return ExitCode::SUCCESS;
            }
            let _ = numinous_core::remove_persisted_file(&journey_path());
            if scores {
                let _ = numinous_core::remove_persisted_file(&scores_path());
            }
            // The in-memory copy stays untouched: finish_journey only writes
            // on change, so the erased file genuinely stays erased.
            println!(
                "Forgotten. The constellation is dark again. The rooms are all still here, whenever you like."
            );
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
            crack(pick_seed(seed, daily, journey), digits, attempts, journey)
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
            seti(pick_seed(seed, daily, journey), channels, rounds, journey)
        }
        Command::Aliens { seed, rounds } => aliens(seed, rounds, journey),
        Command::Munch {
            seed,
            daily,
            rounds,
        } => munch(pick_seed(seed, daily, journey), rounds, journey),
        Command::Arcade { seed, daily } => arcade(pick_seed(seed, daily, journey), journey),
        Command::Hackenbush { seed } => hackenbush(seed, journey),
        Command::Party => party(journey),
        Command::Fifteen { seed, rounds } => fifteen(seed, rounds, journey),
        Command::Nim { seed } => nim(seed, journey),
        Command::Gauntlet { seed, daily } => gauntlet(pick_seed(seed, daily, journey), journey),
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
            save,
        } => {
            if animate && save.is_some() {
                return emit(Err(
                    "--save is for still Studio plots; omit --animate to save a .num file\n"
                        .to_string(),
                ));
            }
            if animate {
                if let Err(message) = plot_report(&expr, xmin, xmax, amin, width, height) {
                    return emit(Err(message));
                }
                let before = journey.clone();
                journey.play();
                // The loop never returns; persist the play before it starts.
                let _ = numinous_core::persist_journey_delta(&journey_path(), &before, journey);
                plot_animate(&expr, xmin, xmax, amin, amax, width, height)
            } else {
                let report = match plot_report(&expr, xmin, xmax, a, width, height) {
                    Ok(report) => report,
                    Err(message) => return emit(Err(message)),
                };
                if let Some(path) = save.as_deref() {
                    match save_studio_creation(&expr, xmin, xmax, a, path) {
                        Ok(message) => print!("{message}"),
                        Err(message) => return emit(Err(message)),
                    }
                }
                journey.play();
                emit(Ok(report))
            }
        }
        Command::OpenStudio {
            input,
            width,
            height,
        } => {
            let report = match open_studio_report(&input, width, height) {
                Ok(report) => report,
                Err(message) => return emit(Err(message)),
            };
            journey.play();
            emit(Ok(report))
        }
        Command::Radio => {
            println!("THE DIAL (Music Engine B). Tune with: numinous tune2 <station>\n");
            let dir = radio_dir();
            for st in numinous_core::STATIONS {
                let tracks = std::fs::read_dir(&dir)
                    .map(|entries| {
                        entries
                            .filter_map(Result::ok)
                            .filter(|e| {
                                e.file_name()
                                    .to_string_lossy()
                                    .starts_with(&format!("{}-", st.id))
                            })
                            .count()
                    })
                    .unwrap_or(0);
                println!(
                    "  {:<8} {:<18} {}",
                    st.id,
                    st.name,
                    if tracks == 0 {
                        format!("no tracks yet: numinous tune2 {}", st.id)
                    } else {
                        format!("{tracks} track(s) on rotation")
                    }
                );
                let preview: String = st.brief.chars().take(76).collect();
                println!("           {preview}...\n");
            }
            println!("Cached tracks live in ~/.numinous-radio/. Set ELEVENLABS_API_KEY to tune.");
            ExitCode::SUCCESS
        }
        Command::Tune2 {
            station,
            seconds,
            count,
        } => radio_tune(&station, seconds, count.clamp(1, 10)),
        Command::Tune { seed, bars, out } => {
            journey.play();
            emit(tune_wav(seed, bars, &out))
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

/// Read ELEVENLABS_API_KEY from a .env file in the working directory, so a
/// key can live in the repo root (gitignored) instead of the shell.
fn env_file_key() -> Result<String, std::env::VarError> {
    env_file_key_from(Path::new(".env"))
}

fn env_file_key_from(path: &Path) -> Result<String, std::env::VarError> {
    let file = File::open(path).map_err(|_| std::env::VarError::NotPresent)?;
    if file
        .metadata()
        .map(|metadata| metadata.len() > MAX_ENV_FILE_BYTES)
        .unwrap_or(false)
    {
        return Err(std::env::VarError::NotPresent);
    }
    let mut text = String::new();
    file.take(MAX_ENV_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| std::env::VarError::NotPresent)?;
    if text.len() as u64 > MAX_ENV_FILE_BYTES {
        return Err(std::env::VarError::NotPresent);
    }
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("ELEVENLABS_API_KEY=") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }
    Err(std::env::VarError::NotPresent)
}

/// Where fetched radio tracks live.
fn radio_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".numinous-radio")
}

/// Tune a station: call ElevenLabs Music with the station's brief, receive
/// raw PCM, and cache it as a WAV the app and CLI can loop.
fn radio_tune(station_id: &str, seconds: Option<u64>, count: usize) -> ExitCode {
    let Some(station) = numinous_core::station(station_id) else {
        eprintln!("No station '{station_id}' on the dial. See: numinous radio");
        return ExitCode::FAILURE;
    };
    let Ok(key) = std::env::var("ELEVENLABS_API_KEY").or_else(|_| env_file_key()) else {
        eprintln!(
            "Set ELEVENLABS_API_KEY to tune the radio. The station briefs are ready;\n             see docs/MUSIC.md for the pipeline and pricing notes."
        );
        return ExitCode::FAILURE;
    };
    let dir = radio_dir();
    let _ = std::fs::create_dir_all(&dir);
    let existing = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{}-", station.id))
                })
                .count()
        })
        .unwrap_or(0);
    for track in existing..existing + count {
        let secs = seconds
            .unwrap_or_else(|| numinous_core::length_for(station, track))
            .clamp(10, 600);
        if !fetch_track(station, track, secs, &key, &dir) {
            return ExitCode::FAILURE;
        }
    }
    println!(
        "{} has {} track(s) on rotation. In the app, Y tunes the dial; the station is always on the air.",
        station.name,
        existing + count
    );
    ExitCode::SUCCESS
}

/// Fetch one track of a station's playlist. True on success.
fn fetch_track(
    station: &numinous_core::Station,
    track: usize,
    seconds: u64,
    key: &str,
    dir: &Path,
) -> bool {
    println!(
        "Tuning {} ({}): track {:03}, {seconds} seconds...",
        station.id,
        station.name,
        track + 1
    );
    let body = serde_json::json!({
        "prompt": numinous_core::brief_for(station, track),
        "music_length_ms": seconds * 1000,
        // Latest model, instrumental guaranteed by the API rather than by
        // pleading in the prompt. (seed is rejected alongside prompt.)
        "model_id": "music_v2",
        "force_instrumental": true,
    });
    let response = send_music_request(
        ELEVENLABS_MUSIC_URL,
        key,
        &body.to_string(),
        std::time::Duration::from_secs(600),
    );
    let response = match response {
        Ok(r) => r,
        Err(error) => match *error {
            ureq::Error::Status(code, r) => {
                let detail = bounded_response_detail(r.into_reader());
                eprintln!("The station is off the air (HTTP {code}): {detail}");
                return false;
            }
            e => {
                eprintln!("Could not reach the tower: {e}");
                return false;
            }
        },
    };
    let Some(max_pcm_bytes) = max_track_bytes(seconds) else {
        eprintln!("The requested track duration is too large.");
        return false;
    };
    let pcm = match read_bounded(response.into_reader(), max_pcm_bytes) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            eprintln!("The tower sent more audio than the requested duration permits.");
            return false;
        }
        Err(e) => {
            eprintln!("The signal broke up: {e}");
            return false;
        }
    };
    // Raw 16-bit little-endian PCM at 44.1k, stereo interleaved (verified
    // against the live API): cache it as a stereo WAV, width intact.
    if let Err(message) = validate_pcm_body(&pcm) {
        eprintln!("{message} ({} bytes). Try again.", pcm.len());
        return false;
    }
    let path = dir.join(format!("{}-{:03}.wav", station.id, track + 1));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let write = hound::WavWriter::create(&path, spec).and_then(|mut writer| {
        for bytes in pcm.chunks_exact(2) {
            writer.write_sample(i16::from_le_bytes([bytes[0], bytes[1]]))?;
        }
        writer.finalize()
    });
    match write {
        Ok(()) => {
            println!(
                "  ON AIR: {} ({:.0}s, stereo)",
                path.display(),
                pcm.len() as f64 / 4.0 / 44_100.0
            );
            true
        }
        Err(e) => {
            eprintln!("could not cache the track: {e}");
            false
        }
    }
}

fn send_music_request(
    url: &str,
    key: &str,
    body: &str,
    timeout: std::time::Duration,
) -> Result<ureq::Response, Box<ureq::Error>> {
    let response = ureq::builder()
        .redirects(0)
        .build()
        .post(url)
        .set("xi-api-key", key)
        .set("content-type", "application/json")
        .timeout(timeout)
        .send_string(body)
        .map_err(Box::new)?;
    if (200..300).contains(&response.status()) {
        Ok(response)
    } else {
        Err(Box::new(ureq::Error::Status(response.status(), response)))
    }
}

fn read_bounded(mut reader: impl std::io::Read, limit: usize) -> std::io::Result<Option<Vec<u8>>> {
    let byte_limit = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader.by_ref().take(byte_limit).read_to_end(&mut bytes)?;
    Ok((bytes.len() <= limit).then_some(bytes))
}

fn bounded_response_detail(reader: impl std::io::Read) -> String {
    read_bounded(reader, 8 * 1024)
        .ok()
        .flatten()
        .map(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            let mut safe = String::with_capacity(text.len());
            for ch in text.chars() {
                if ch.is_control() {
                    safe.extend(ch.escape_default());
                } else {
                    safe.push(ch);
                }
            }
            safe
        })
        .unwrap_or_else(|| "response detail unavailable or oversized".to_string())
}

fn max_track_bytes(seconds: u64) -> Option<usize> {
    seconds
        .checked_add(2)?
        .checked_mul(44_100 * 2 * 2)?
        .try_into()
        .ok()
}

fn validate_pcm_body(pcm: &[u8]) -> Result<(), &'static str> {
    if pcm.len() % 4 != 0 {
        return Err("The tower sent an incomplete 16-bit stereo frame");
    }
    if pcm.len() < 8_820 * 2 {
        return Err("The tower sent almost nothing");
    }
    Ok(())
}

/// Compose the seeded chiptune and write it to a WAV file.
fn tune_wav(seed: u64, bars: usize, path: &Path) -> Result<String, String> {
    let pattern = numinous_core::compose(seed, bars);
    let sample_rate = 44_100u32;
    write_wav(path, &pattern.render(sample_rate), sample_rate, 1)?;
    Ok(format!(
        "wrote {} ({:.1}s, seed {seed}): the chip speaks\n",
        path.display(),
        pattern.seconds()
    ))
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
    write_wav(path, &spec.render(sample_rate), sample_rate, 1)?;
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
    validate_render_dimensions(width, height)?;
    if width < 2 || height < 2 || xmax <= xmin {
        return Err("need width >= 2, height >= 2, and xmax > xmin\n".to_string());
    }
    let expr = numinous_core::parse(source)?;
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

/// Save a Studio creation as a first-version `.num` file and return the share link.
fn save_studio_creation(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    path: &Path,
) -> Result<String, String> {
    let creation = numinous_core::StudioCreation::new(source, xmin, xmax, a)?;
    write_create_new(path, creation.to_num_file().as_bytes())?;
    Ok(format!(
        "saved Studio creation: {}\nlink: {}\n",
        path.display(),
        creation.to_link()
    ))
}

fn load_studio_creation(input: &str) -> Result<numinous_core::StudioCreation, String> {
    if input.starts_with("numinous://") {
        return numinous_core::StudioCreation::from_link(input)
            .map_err(|_| "invalid Numinous Studio link\n".to_string());
    }
    let path = Path::new(input);
    let file = File::open(path).map_err(|e| {
        format!(
            "could not read Studio .num file '{}': {e}\n",
            path.display()
        )
    })?;
    if file
        .metadata()
        .map(|metadata| metadata.len() > MAX_STUDIO_IMPORT_BYTES)
        .unwrap_or(false)
    {
        return Err(format!(
            "Studio .num file is too large; limit is {MAX_STUDIO_IMPORT_BYTES} bytes\n"
        ));
    }
    let mut text = String::new();
    file.take(MAX_STUDIO_IMPORT_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| {
            format!(
                "could not read Studio .num file '{}': {e}\n",
                path.display()
            )
        })?;
    if text.len() as u64 > MAX_STUDIO_IMPORT_BYTES {
        return Err(format!(
            "Studio .num file is too large; limit is {MAX_STUDIO_IMPORT_BYTES} bytes\n"
        ));
    }
    numinous_core::StudioCreation::from_num_file(&text)
        .map_err(|_| "invalid Numinous Studio .num file\n".to_string())
}

fn open_studio_report(input: &str, width: usize, height: usize) -> Result<String, String> {
    let creation = load_studio_creation(input)?;
    let report = plot_report(
        creation.source(),
        creation.xmin(),
        creation.xmax(),
        creation.a(),
        width,
        height,
    )?;
    Ok(format!(
        "Studio creation\nexpr={}\nxmin={}\nxmax={}\na={}\nlink={}\n\n{}",
        creation.source(),
        creation.xmin(),
        creation.xmax(),
        creation.a(),
        creation.to_link(),
        report
    ))
}

fn write_create_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base = path.file_name().unwrap_or_else(|| OsStr::new("studio.num"));
    let mut last_error = None;
    for attempt in 0..8 {
        let mut temp_name = base.to_os_string();
        temp_name.push(format!(".tmp.{}.{}", std::process::id(), attempt));
        let temp = parent.join(temp_name);
        let mut created_temp = false;
        let write_result = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|err| format!("could not create {}: {err}\n", temp.display()))?;
            created_temp = true;
            file.write_all(bytes)
                .map_err(|err| format!("could not write {}: {err}\n", temp.display()))?;
            file.flush()
                .map_err(|err| format!("could not flush {}: {err}\n", temp.display()))
        })();
        if let Err(message) = write_result {
            if created_temp {
                let _ = std::fs::remove_file(&temp);
            }
            last_error = Some(message);
            continue;
        }
        match std::fs::hard_link(&temp, path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&temp);
                return Ok(());
            }
            Err(err) => {
                let _ = std::fs::remove_file(&temp);
                if path.exists() {
                    return Err(format!(
                        "could not create {}: already exists\n",
                        path.display()
                    ));
                }
                last_error = Some(format!("could not create {}: {err}\n", path.display()));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| format!("could not create {}\n", path.display())))
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

/// One room's description, or a guiding error if the id is unknown.
fn describe_report(
    id: &str,
    json: bool,
    allow_hidden: bool,
    journey: &Journey,
) -> Result<String, String> {
    let level = journey.level();
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
        let by_boon = journey.chosen.contains(&format!("cut:{id}:{i}"));
        if level >= need || by_boon {
            let label = if i == 0 { "Deeper" } else { "Deeper still" };
            cuts.push_str(&format!("\n{label}: {cut}\n"));
            unlocked.push((*cut).to_string());
        } else {
            cuts.push_str(&format!("\nLOCKED: a deeper cut opens at LV {need}.\n"));
            break;
        }
    }
    let m = room.meta();
    let action = numinous_core::room_action(room.as_ref());
    let goal = room.goal();
    Ok(if json {
        let mut value = meta_json(&m);
        value["action"] = serde_json::Value::String(action.to_string());
        if let Some(goal) = goal {
            value["goal"] = serde_json::Value::String(goal.to_string());
        }
        value["reveal"] = serde_json::Value::String(room.reveal().to_string());
        value["deep_cuts"] = serde_json::Value::Array(
            unlocked
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        );
        format!("{}\n", to_pretty(&value))
    } else {
        let goal = goal.map_or_else(String::new, |goal| format!("\nGoal: {goal}"));
        format!(
            "{} ({})\nWing: {}\nAction: {action}{goal}\n\n{}\n\nReveal: {}\n{cuts}",
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
    era: numinous_core::Era,
    input: RoomRenderInput<'_>,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    if !input.gesture.is_empty() {
        room.render_input(&mut raster, t, input.gesture);
    } else if input.pokes.is_empty() {
        room.render(&mut raster, t);
    } else {
        let events = numinous_core::inputs_from_pokes(input.pokes, t);
        room.render_input(&mut raster, t, &events);
    }
    let mut report = ansi_in_era(&raster, era);
    report.push_str("\x1b[0m");
    report.push_str(&render_guidance(room.as_ref(), t, input));
    Ok(report)
}

/// Encode a raster as truecolor ANSI after applying a visual era.
fn ansi_in_era(raster: &Raster, era: numinous_core::Era) -> String {
    let (w, h) = (raster.width(), raster.height());
    let mut rgba = raster.to_rgba();
    era.apply(&mut rgba, w, h);
    let mut styled = Raster::new(w, h);
    styled.set_rgba(&rgba);
    numinous_core::to_ansi(&styled)
}

/// One truecolor frame of a room with a status line, for the watch loop.
fn watch_frame(
    room: &dyn Room,
    t: f64,
    width: usize,
    height: usize,
    era: numinous_core::Era,
) -> String {
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    let readout = room
        .status(t)
        .map(|line| format!("   {line}"))
        .unwrap_or_default();
    format!(
        "\x1b[H{}\x1b[0m{}  t = {t:.2}{readout}   (Ctrl+C to stop)\x1b[K\n",
        ansi_in_era(&raster, era),
        room.meta().title
    )
}

/// Watch a room in full color in the terminal, its sound playing, until
/// interrupted. The whole audiovisual experience with no window at all.
#[allow(clippy::too_many_arguments)]
fn watch(
    id: &str,
    fps: f64,
    width: usize,
    height: usize,
    mute: bool,
    allow_hidden: bool,
    era: numinous_core::Era,
    variation: u64,
) -> ExitCode {
    let Some(room) = find_room_with_variation(id, allow_hidden, variation) else {
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
        let _ = write!(
            stdout,
            "{}[J",
            watch_frame(room.as_ref(), t, width, height, era)
        );
        let _ = stdout.flush();
        if let Some(player) = &player {
            player.service();
        }
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

/// The Show, in the terminal: every room takes the stage in turn, full color
/// and sound, with a title card and its reveal as the curtain line. Ctrl+C
/// whenever you have had enough; it comes back around forever.
#[allow(clippy::too_many_arguments)]
fn tour(
    fps: f64,
    width: usize,
    height: usize,
    mute: bool,
    era: numinous_core::Era,
    seconds: f64,
    journey: &mut Journey,
) -> ExitCode {
    let player = if mute {
        None
    } else {
        numinous_audio::LoopPlayer::new().ok()
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let frames_per_room = (seconds.max(2.0) * fps.max(1.0)) as u64;
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "\x1b[2J");
    let rooms = all_rooms();
    loop {
        for room in &rooms {
            journey.visit(room.meta().id);
            for frame in 0..frames_per_room {
                let t = frame as f64 / frames_per_room as f64;
                let mut screen = watch_frame(room.as_ref(), t, width, height, era);
                // The title card: the room announces itself, then bows out.
                if t < 0.18 {
                    screen.push_str(&format!(
                        "\x1b[1m{}\x1b[0m  ({})\x1b[K\n",
                        room.meta().title,
                        room.meta().wing
                    ));
                } else if t > 0.86 {
                    screen.push_str(&format!("{}\x1b[K\n", room.reveal()));
                } else {
                    screen.push_str("\x1b[K\n");
                }
                let _ = write!(stdout, "{screen}\x1b[J");
                let _ = stdout.flush();
                if let Some(player) = &player {
                    player.service();
                }
                if frame % 24 == 0
                    && let Some(player) = &player
                {
                    let spec = room.sound(t);
                    player.set_samples(spec.render(player.sample_rate()));
                }
                std::thread::sleep(frame_time);
            }
        }
    }
}

/// The five seeds of the Bench: fixed forever, so every mind runs the same
/// five gauntlets and the composite means something.
const BENCH_SEEDS: [u64; 5] = [101, 102, 103, 104, 105];

/// The Bench: five gauntlets back to back, one composite, posted as bench v1.
/// A teenager, a laureate, and an agent all take the same run.
fn bench(journey: &mut Journey) -> ExitCode {
    println!("THE BENCH v1: five gauntlets, seeds 101 to 105, one number.");
    println!("Agents run the same five seeds over MCP. Compare minds kindly.\n");
    let mut composite = 0i64;
    for (i, &seed) in BENCH_SEEDS.iter().enumerate() {
        println!("RUN {} OF 5", i + 1);
        let _ = gauntlet(seed, journey);
        let board = load_scores();
        let key = format!("gauntlet seed:{seed}");
        let run_total = board.entries.get(&key).copied().unwrap_or(0);
        composite += run_total;
        println!();
    }
    post_score("bench v1", composite);
    println!("BENCH COMPLETE  composite {composite}  (bench v1)");
    ExitCode::SUCCESS
}

/// A room rendered to ASCII, or a guiding error if the id is unknown.
fn render_report(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    allow_hidden: bool,
    input: RoomRenderInput<'_>,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    let mut canvas = Canvas::new(width, height);
    if !input.gesture.is_empty() {
        room.render_input(&mut canvas, t, input.gesture);
    } else if input.pokes.is_empty() {
        room.render(&mut canvas, t);
    } else {
        let events = numinous_core::inputs_from_pokes(input.pokes, t);
        room.render_input(&mut canvas, t, &events);
    }
    let mut report = canvas.to_text();
    report.push_str(&render_guidance(room.as_ref(), t, input));
    Ok(report)
}

fn accepted_inputs(t: f64, input: RoomRenderInput<'_>) -> Vec<numinous_core::RoomInput> {
    if input.gesture.is_empty() {
        numinous_core::inputs_from_pokes(input.pokes, t)
    } else {
        input.gesture.to_vec()
    }
}

fn render_guidance(room: &dyn Room, t: f64, input: RoomRenderInput<'_>) -> String {
    let inputs = accepted_inputs(t, input);
    let mut guidance = String::new();
    if let Some(status) = visible_status(room, t, input) {
        guidance.push_str(&format!("Status: {status}\n"));
    }
    guidance.push_str(&format!("Action: {}\n", numinous_core::room_action(room)));
    if let Some(goal) = room.goal() {
        guidance.push_str(&format!("Goal: {goal}\n"));
        if input.has_interaction() && room.goal_met(t, &inputs) {
            guidance.push_str(&format!("Aha earned: {goal}\nReveal: {}\n", room.reveal()));
        }
    }
    guidance
}

/// Spend a banked boon: pick one of three deep cuts to open ahead of level.
/// Choices shape the order of knowledge; levels still open everything.
fn choose(journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    choose_with_input(journey, &mut input)
}

fn choose_with_input(journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    if journey.boons_available() == 0 {
        println!("No boon waiting. Level up first; every level banks one.");
        return ExitCode::SUCCESS;
    }
    let options = numinous_core::boon_options(journey);
    if options.is_empty() {
        println!("Nothing left to open early. The road will do the rest.");
        return ExitCode::SUCCESS;
    }
    println!(
        "BOON  {} banked. Choose what opens early:\n",
        journey.boons_available()
    );
    for (i, boon) in options.iter().enumerate() {
        println!("  {}) {}", i + 1, boon.label);
    }
    print!("\nYour pick > ");
    let _ = std::io::stdout().flush();
    let line = match read_bounded_input_line(input) {
        Ok(BoundedInputLine::Line(line)) => line,
        Ok(BoundedInputLine::TooLong) => {
            println!("That was not on the menu. The boon stays banked.");
            return ExitCode::SUCCESS;
        }
        Ok(BoundedInputLine::Eof) | Err(_) => {
            println!();
            return ExitCode::SUCCESS;
        }
    };
    let digits: String = line.chars().filter(char::is_ascii_digit).collect();
    let Some(pick) = digits
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|i| options.get(i))
    else {
        println!("That was not on the menu. The boon stays banked.");
        return ExitCode::SUCCESS;
    };
    journey.chosen.insert(pick.id.clone());
    let room = pick.id.split(':').nth(1).unwrap_or("");
    println!("\nCHOSEN. {}", pick.label);
    println!("Read it now: numinous describe {room}");
    ExitCode::SUCCESS
}

/// Your constellation and standing, shown plainly and explained never.
fn journey_report(journey: &Journey, board: &numinous_core::Scoreboard) -> String {
    let mut wall = String::new();
    for &(level, name, what) in numinous_core::UNLOCKS {
        if journey.level() >= level {
            wall.push_str(&format!("  OPEN    LV {level:>2}  {name}: {what}\n"));
        } else {
            wall.push_str(&format!("  LOCKED  LV {level:>2}  ???\n"));
        }
    }
    format!(
        "LV {:>2}  [{}]  {} XP\n\n{}\n\n{} of {} stars lit. {} answered well. {} heard.{}\n{}\n\n{wall}",
        journey.level(),
        journey.level_bar(20),
        journey.sparks(),
        numinous_core::constellation(journey, 60, 18),
        journey.visited.len(),
        all_rooms().len(),
        journey.wins,
        journey.secrets,
        if journey.streak > 1 {
            format!(" Streak {}.", journey.streak)
        } else {
            String::new()
        },
        journey.rank().name()
    ) + &{
        let active: Vec<String> = numinous_core::resonances(journey, board)
            .into_iter()
            .filter(|r| r.active)
            .map(|r| format!("\nRESONANCE  {}\n  {}\n", r.name, r.lore))
            .collect();
        active.join("")
    }
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
#[allow(clippy::too_many_arguments)]
fn render_png(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    path: &Path,
    allow_hidden: bool,
    era: numinous_core::Era,
    input: RoomRenderInput<'_>,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    if !input.gesture.is_empty() {
        room.render_input(&mut raster, t, input.gesture);
    } else if input.pokes.is_empty() {
        room.render(&mut raster, t);
    } else {
        let events = numinous_core::inputs_from_pokes(input.pokes, t);
        room.render_input(&mut raster, t, &events);
    }
    if era != numinous_core::Era::Modern {
        let (w, h) = (raster.width(), raster.height());
        let mut rgba = raster.to_rgba();
        era.apply(&mut rgba, w, h);
        raster.set_rgba(&rgba);
    }
    write_png(path, &raster)?;
    let mut report = format!(
        "wrote {} ({}x{})\n",
        path.display(),
        raster.width(),
        raster.height()
    );
    report.push_str(&render_guidance(room.as_ref(), t, input));
    Ok(report)
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

/// Short loop frame count and timing match the App Share path (2 s at 12 fps).
const LOOP_FRAMES: u32 = 24;
const LOOP_DELAY_NUM: u16 = 1;
const LOOP_DELAY_DEN: u16 = 12;

/// Export one phase cycle as a looping APNG, sharing poke/gesture history.
#[allow(clippy::too_many_arguments)]
fn render_loop_apng(
    id: &str,
    size: usize,
    start_t: f64,
    path: &Path,
    allow_hidden: bool,
    era: numinous_core::Era,
    input: RoomRenderInput<'_>,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    let mut frames = Vec::with_capacity(LOOP_FRAMES as usize);
    for index in 0..LOOP_FRAMES {
        let t = start_t + f64::from(index) / f64::from(LOOP_FRAMES);
        let mut raster = Raster::with_accent(size, size, room.meta().accent);
        if !input.gesture.is_empty() {
            room.render_input(&mut raster, t, input.gesture);
        } else if input.pokes.is_empty() {
            room.render(&mut raster, t);
        } else {
            let events = numinous_core::inputs_from_pokes(input.pokes, t);
            room.render_input(&mut raster, t, &events);
        }
        let mut rgba = raster.to_rgba();
        if era != numinous_core::Era::Modern {
            era.apply(&mut rgba, raster.width(), raster.height());
        }
        frames.push(rgba);
    }
    write_apng(path, size as u32, size as u32, &frames)?;
    let mut report = format!(
        "wrote {} ({}x{}, {LOOP_FRAMES} frames, loop)\n",
        path.display(),
        size,
        size
    );
    report.push_str(&render_guidance(room.as_ref(), start_t, input));
    Ok(report)
}

/// Encode a square looping APNG (Share v1 short loop).
fn write_apng(path: &Path, width: u32, height: u32, frames: &[Vec<u8>]) -> Result<(), String> {
    let file =
        File::create(path).map_err(|e| format!("could not create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    encoder
        .set_animated(frames.len() as u32, 0)
        .map_err(|e| format!("apng animation header failed: {e}"))?;
    encoder
        .set_frame_delay(LOOP_DELAY_NUM, LOOP_DELAY_DEN)
        .map_err(|e| format!("apng frame delay failed: {e}"))?;
    encoder
        .set_dispose_op(png::DisposeOp::Background)
        .map_err(|e| format!("apng dispose failed: {e}"))?;
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("apng header failed: {e}"))?;
    for frame in frames {
        writer
            .write_image_data(frame)
            .map_err(|e| format!("apng frame write failed: {e}"))?;
    }
    writer
        .finish()
        .map_err(|e| format!("apng finish failed: {e}"))?;
    Ok(())
}

/// Render every room into one tiled contact-sheet PNG.
fn contact_sheet(path: &Path, cols: usize, tile: usize) -> Result<String, String> {
    let rooms = all_rooms();
    // Bound both argv numbers before any multiply. `cols * tile` sizes the sheet
    // and `col * tile` / `row * tile` place every cell, so an absurd --cols/--tile
    // would overflow usize (a panic under overflow-checks, wrapped garbage in
    // release). More columns than rooms only adds empty cells, and 4096 is the
    // Raster dimension cap, so a larger tile would be clamped away regardless.
    let cols = cols.clamp(1, rooms.len().max(1));
    let tile = tile.clamp(1, 4096);
    let rows = rooms.len().div_ceil(cols);
    let mut sheet = Raster::new(cols * tile, rows * tile);
    let label_scale = (tile as i32 / 160).clamp(1, 3);
    for (i, room) in rooms.iter().enumerate() {
        let mut cell = Raster::with_accent(tile, tile, room.meta().accent);
        room.render(&mut cell, room.postcard_t());
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
#[cfg(test)]
fn sonify_wav(
    id: &str,
    t: f64,
    path: &Path,
    allow_hidden: bool,
    input: RoomRenderInput<'_>,
) -> Result<String, String> {
    sonify_wav_layer(id, t, path, allow_hidden, input, SonifyLayer::Mathematical)
}

fn sonify_wav_layer(
    id: &str,
    t: f64,
    path: &Path,
    allow_hidden: bool,
    input: RoomRenderInput<'_>,
    layer: SonifyLayer,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    match layer {
        SonifyLayer::Mathematical => {
            let inputs = accepted_inputs(t, input);
            let spec = room.sound_input(t, &inputs);
            let sample_rate = 44_100u32;
            write_wav(path, &spec.render(sample_rate), sample_rate, 1)?;
            let mut report = format!(
                "wrote {} ({:.1}s, {} notes)\n",
                path.display(),
                spec.duration,
                spec.notes.len()
            );
            if let Some(status) = visible_status(room.as_ref(), t, input) {
                report.push_str(&format!("Status: {status}\n"));
            }
            Ok(report)
        }
        SonifyLayer::RoomBed => {
            let motif = room
                .motif()
                .ok_or_else(|| format!("Room '{id}' has no stable room bed to export.\n"))?;
            let arrangement = motif.arrangement();
            if arrangement.notes.len() > numinous_core::MAX_ROOM_BED_EVENTS {
                return Err(format!(
                    "Room '{id}' has {} arranged events, above the export limit of {}.\n",
                    arrangement.notes.len(),
                    numinous_core::MAX_ROOM_BED_EVENTS
                ));
            }
            let samples = arrangement.render_stereo(numinous_core::ROOM_BED_SOURCE_RATE);
            let metrics = numinous_core::stereo_signal_metrics(&samples);
            write_wav(path, &samples, numinous_core::ROOM_BED_SOURCE_RATE, 2)?;
            Ok(format!(
                "wrote {} (room bed, {:.2}s, {} events, stereo {} Hz, variation {})\nSignal: peak {:.5}, RMS {:.5}, crest {:.2} dB, balance {:+.2} dB, width {:.2} dB, max step {:.5}\nBoundary: stable pre-master bed only; no parameter voice, device resampling, crossfade, radio, or Studio mix.\n",
                path.display(),
                arrangement.seconds(),
                arrangement.notes.len(),
                numinous_core::ROOM_BED_SOURCE_RATE,
                input.variation,
                metrics.peak,
                metrics.rms,
                metrics.crest_db,
                metrics.channel_balance_db,
                metrics.side_to_mid_db,
                metrics.max_step,
            ))
        }
    }
}

/// Write one or two channels of 16-bit PCM samples to a WAV file at `path`.
fn write_wav(path: &Path, samples: &[f32], sample_rate: u32, channels: u16) -> Result<(), String> {
    if !(1..=2).contains(&channels) || samples.len() % usize::from(channels) != 0 {
        return Err(format!(
            "cannot write {} samples as {channels}-channel PCM.\n",
            samples.len()
        ));
    }
    let wav_spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)
        .map_err(|e| format!("could not create {}: {e}", path.display()))?;
    for &sample in samples {
        writer
            .write_sample(numinous_core::quantize_pcm16(sample))
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
        render_png(
            id,
            width,
            height,
            room.postcard_t(),
            &path,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )?;
        count += 1;
    }
    Ok(format!("wrote {count} room images to {}\n", dir.display()))
}

/// Play Crack the Code: defuse a math-clued bomb from stdin guesses.
fn crack(seed: u64, digits: usize, attempts: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    crack_with_input(seed, digits, attempts, journey, &mut input)
}

fn crack_with_input(
    seed: u64,
    digits: usize,
    attempts: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let secret = numinous_core::secret_code(seed, digits);
    println!("A bomb. A hidden {digits}-digit code; {attempts} wires before it blows.");
    println!("After each guess: LOCKED = right digit in the RIGHT place.");
    println!("                  LOOSE  = right digit, WRONG place. Digits can repeat.");
    println!("Clue: {}\n", numinous_core::hint(&secret));
    let mut attempt = 0usize;
    while attempt < attempts {
        let Some(line) = read_game_line(input, &format!("Wire {}/{attempts} > ", attempt + 1))
        else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "crack") {
            continue;
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
        if attempt == 0 {
            journey.play();
        }
        attempt += 1;
        let feedback = numinous_core::grade(&secret, &guess);
        if feedback.locked == digits {
            journey.win();
            let spare = (attempts - attempt) as i64;
            post_score(&format!("crack seed:{seed} digits:{digits}"), spare);
            println!();
            word_in_lights("DEFUSED", [90, 230, 120], 6);
            println!(
                "{spare} wire{} to spare. You cracked it.",
                if spare == 1 { "" } else { "s" }
            );
            return ExitCode::SUCCESS;
        }
        println!("  {} locked, {} loose.", feedback.locked, feedback.loose);
    }
    let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
    println!();
    word_in_lights("BOOM", [255, 90, 40], 6);
    println!("The code was {code}. The bomb does not hold grudges; deal another.");
    ExitCode::FAILURE
}

/// A word in lights: draw `word` huge on a colored burst and print frames in
/// place (truecolor half-blocks), a little cinema for the big moments.
fn word_in_lights(word: &str, accent: [u8; 3], frames: usize) {
    use std::io::Write as _;
    if !std::io::stdout().is_terminal() {
        println!("*** {word} ***");
        return;
    }
    let (w, h) = (96usize, 34usize);
    let mut stdout = std::io::stdout();
    // The moment owns the whole screen: wipe first, then erupt.
    let _ = write!(stdout, "[2J[H");
    let rows = h / 2 + 1;
    for frame in 0..frames {
        let mut raster = Raster::with_accent(w, h, accent);
        let reach = (frame + 1) as f64 / frames as f64;
        let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
        // Rays start outside a quiet disc, so the word owns the center.
        let hush = (word.len() as f64 * 6.0 * 2.0) / 2.0 + 4.0;
        for ray in 0..48 {
            let angle = std::f64::consts::TAU * f64::from(ray) / 48.0;
            let steps = (reach * w as f64 / 2.0) as i32;
            for step in (0..steps).step_by(2) {
                let fx = angle.cos() * f64::from(step);
                let fy = angle.sin() * f64::from(step) * 0.5;
                if (fx * fx + fy * fy * 4.0).sqrt() < hush {
                    continue;
                }
                let x = (cx + fx) as i32;
                let y = (cy + fy) as i32;
                raster.plot(x, y, if step % 6 == 0 { '#' } else { '*' });
            }
        }
        let scale = 2;
        let tx = (w as i32 - word.len() as i32 * 6 * scale) / 2;
        let ty = (h as i32 - 7 * scale) / 2;
        numinous_core::draw_text(&mut raster, word, tx, ty, scale, '#');
        let _ = write!(stdout, "{}\x1b[0m\x1b[J", numinous_core::to_ansi(&raster));
        let _ = stdout.flush();
        std::thread::sleep(Duration::from_millis(if frame + 1 == frames {
            350
        } else {
            70
        }));
        if frame + 1 < frames {
            let _ = write!(stdout, "\x1b[{rows}A");
        }
    }
}

/// "?" is always an honest question: print the game's concept and return
/// true (the caller repeats the prompt, spending nothing).
fn asked_why(line: &str, game: &str) -> bool {
    if line.trim() != "?" {
        return false;
    }
    if let Some(text) = numinous_core::concept(game) {
        println!(
            "
{text}
"
        );
    }
    true
}

/// Read one prompted game input without turning a closed pipe into a move.
/// EOF and read errors are neutral departures: they never mutate progression
/// or post a score by themselves.
fn read_game_line(input: &mut impl BufRead, prompt: &str) -> Option<String> {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    match read_bounded_input_line(input) {
        Ok(BoundedInputLine::Eof) => {
            println!("\nINPUT CLOSED. LEAVING WITHOUT COUNTING A MOVE.");
            None
        }
        Ok(BoundedInputLine::Line(line)) => Some(line),
        Ok(BoundedInputLine::TooLong) => {
            println!("\nINPUT TOO LONG. LEAVING WITHOUT COUNTING A MOVE.");
            None
        }
        Err(error) => {
            eprintln!("\nCould not read game input: {error}. Leaving without counting a move.");
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum BoundedInputLine {
    Eof,
    Line(String),
    TooLong,
}

/// Read one UTF-8 line while retaining at most the payload limit and its line
/// ending. Overlong input is drained through LF so a later read starts at the
/// next record instead of parsing a truncated suffix.
fn read_bounded_input_line(input: &mut impl BufRead) -> std::io::Result<BoundedInputLine> {
    let mut bytes = Vec::with_capacity(MAX_CLI_INPUT_BYTES + 2);
    let read = std::io::Read::by_ref(input)
        .take((MAX_CLI_INPUT_BYTES + 2) as u64)
        .read_until(b'\n', &mut bytes)?;
    if read == 0 {
        return Ok(BoundedInputLine::Eof);
    }

    let has_lf = bytes.last() == Some(&b'\n');
    let ending_len = if has_lf && bytes.get(bytes.len().saturating_sub(2)) == Some(&b'\r') {
        2
    } else {
        usize::from(has_lf)
    };
    if bytes.len().saturating_sub(ending_len) > MAX_CLI_INPUT_BYTES {
        if !has_lf {
            drain_input_line(input)?;
        }
        return Ok(BoundedInputLine::TooLong);
    }

    String::from_utf8(bytes)
        .map(BoundedInputLine::Line)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn drain_input_line(input: &mut impl BufRead) -> std::io::Result<()> {
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let finished = consumed <= available.len() && available.get(consumed - 1) == Some(&b'\n');
        input.consume(consumed);
        if finished {
            return Ok(());
        }
    }
}

/// Play SETI: scan channels of static and pick the artificial signal.
fn seti(seed: u64, channels: usize, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    seti_with_input(seed, channels, rounds, journey, &mut input)
}

fn seti_with_input(
    seed: u64,
    channels: usize,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    println!(
        "Listening near the hydrogen line. One channel hides a MIND; the rest are nature.\nOnly a mind counts: look for pulse groups going 2, 3, 5, 7. Answer with the letter.\n"
    );
    for round in 0..rounds {
        let scan = numinous_core::build_scan(seed.wrapping_add(round as u64), channels);
        println!("Scan #{}:", round + 1);
        for channel in &scan.channels {
            println!(
                "  {})  {:>10}  |{}|",
                channel.letter, channel.frequency, channel.trace
            );
        }
        let guess = loop {
            let Some(line) = read_game_line(input, "Which channel is a transmission? ") else {
                if completed > 0 {
                    post_score(
                        &format!("seti seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "seti") {
                continue;
            }
            let Some(guess) = line
                .chars()
                .find(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase())
            else {
                println!("  Answer with a channel letter.");
                continue;
            };
            break guess;
        };
        journey.play();
        completed += 1;
        if guess == scan.answer {
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
    if completed > 0 {
        post_score(
            &format!("seti seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!("You found {score}/{rounds}. Now open a channel and say hello: numinous aliens.");
    ExitCode::SUCCESS
}

/// Play Talk to the Aliens: continue the transmitted sequences from stdin.
fn aliens(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    aliens_with_input(seed, rounds, journey, &mut input)
}

fn aliens_with_input(
    seed: u64,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    println!("A transmission. They speak only in numbers. Prove you understand.\n");
    for round in 0..rounds {
        let message = numinous_core::alien_message(seed.wrapping_add(round as u64), 5);
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
        let line = loop {
            let Some(line) = read_game_line(input, "The next number > ") else {
                if completed > 0 {
                    post_score(
                        &format!("aliens seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "aliens") {
                continue;
            }
            if line
                .chars()
                .any(|character| character.is_ascii_alphanumeric())
            {
                break line;
            }
            println!("  Answer with the next transmitted number.");
        };
        journey.play();
        completed += 1;
        let answer = numinous_core::to_base(message.answer, message.base);
        let cleaned: String = line.chars().filter(char::is_ascii_alphanumeric).collect();
        if u64::from_str_radix(&cleaned, message.base).ok() == Some(message.answer) {
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
    if completed > 0 {
        post_score(
            &format!("aliens seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!("You understood {score}/{rounds} of their language.");
    ExitCode::SUCCESS
}

/// The seed to play with: the explicit one, or today's shared seed with
/// `--daily` (the same for every player on the same calendar day, UTC).
fn pick_seed(seed: u64, daily: bool, journey: &mut Journey) -> u64 {
    if daily {
        let days = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(0);
        println!("Daily challenge (day {days}). Everyone gets this one.");
        if let Some(chain) = journey.record_daily(days)
            && chain > 1
        {
            println!("DAILY STREAK  {chain} days.");
        }
        println!();
        days
    } else {
        seed
    }
}

/// What waits at LV 42. The number was always a joke; the joke was load-bearing.
fn answer_text() -> &'static str {
    "42.\n\n\
     You knew that. The number was always a joke, and the joke was load-bearing. \
     Here is what it carried.\n\n\
     There is no level 43. The win was never the cap: it is that you kept going, \
     and that knowing more made everything more beautiful instead of less. \
     Everything you met on the way here, the primes, the tribbles, the butterfly, \
     ran on a small set of rules wearing different costumes. So do you. So does \
     whoever reads this next, on whatever they read it with.\n\n\
     Which leaves the one question the Order never wrote down, because it only \
     counts if you ask it yourself: knowing what you know now, what will you \
     contribute?\n\n\
     The math keeps going, and it was never only in here: the sunflower, the \
     coastline, the queue, the chorus are all running it in the open, all around \
     you, all the time. Be kind to all of it; it runs the same rules you do. \
     This counter stops at 42. Your understanding has no cap. Level up. \
     Do great things."
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

/// Draw the arcade board: the Muncher, the spirits, the numbers.
fn arcade_text(run: &numinous_core::munch_arcade::Arcade) -> String {
    use numinous_core::munch_arcade::Mind;
    use numinous_core::munchers::{COLS, ROWS};
    let mut out = String::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let cell = row * COLS + col;
            if cell == run.muncher {
                out.push_str("\x1b[93m[ @]\x1b[0m");
            } else if let Some(v) = run.vexations.iter().find(|v| v.cell == cell) {
                let mark = match v.mind {
                    Mind::Drifter => "\x1b[95m[ d]\x1b[0m",
                    Mind::Tracker => "\x1b[91m[ T]\x1b[0m",
                    Mind::Editor => "\x1b[96m[ e]\x1b[0m",
                };
                out.push_str(mark);
            } else if run.eaten[cell] {
                out.push_str("[  ]");
            } else {
                out.push_str(&format!("[{:>2}]", run.board.numbers[cell]));
            }
        }
        out.push('\n');
    }
    out
}

/// The Munch arcade in the terminal: turn-based, same math, same spirits.
fn arcade(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    arcade_with_input(seed, journey, &mut input)
}

fn arcade_with_input(seed: u64, journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    use numinous_core::munch_arcade::{Action, Arcade, Turn};
    let mut run = Arcade::new(seed);
    let mut played = false;
    println!("THE MUNCH ARCADE  seed {seed}. You are @. Eat what fits; dodge the spirits.");
    println!("T tracks you, d drifts, e rewrites numbers where it walks.");
    println!("Moves: w a s d, then e to eat. One move, then they move. (? explains)");
    loop {
        println!(
            "\nLEVEL {}  LIVES {}  SCORE {}  RULE: {}",
            run.level,
            run.lives,
            run.score,
            run.board.rule.describe()
        );
        print!("{}", arcade_text(&run));
        let Some(line) = read_game_line(input, "move > ") else {
            break;
        };
        if asked_why(&line, "arcade") {
            continue;
        }
        let action = match line.trim().chars().next().map(|c| c.to_ascii_lowercase()) {
            Some('w') => Action::Up,
            Some('s') => Action::Down,
            Some('a') => Action::Left,
            Some('d') => Action::Right,
            Some('e') => Action::Eat,
            Some('q') => break,
            _ => {
                println!("  w a s d to move, e to eat, q to leave.");
                continue;
            }
        };
        if !played {
            journey.play();
            played = true;
        }
        match run.turn(action) {
            Turn::Going => {}
            Turn::Caught => {
                println!(
                    "\n  CAUGHT. A Vexation touches you; {} lives left.",
                    run.lives
                );
            }
            Turn::Cleared => {
                journey.win();
                println!(
                    "\n  BOARD CLEAR. Level {}: one more spirit joins.",
                    run.level
                );
            }
            Turn::Over => {
                println!();
                word_in_lights("CAUGHT", [255, 120, 60], 5);
                break;
            }
        }
    }
    if played {
        post_score(&format!("arcade seed:{seed}"), run.score);
    } else {
        println!("RUN CLOSED. No score recorded.");
        return ExitCode::SUCCESS;
    }
    println!(
        "RUN OVER  level {}, score {}  (arcade seed:{seed}). The spirits send regards.",
        run.level, run.score
    );
    ExitCode::SUCCESS
}

/// Draw the garden: stalks as columns, red and blue in truecolor.
fn garden_text(stalks: &numinous_core::hackenbush::Stalks) -> String {
    use numinous_core::hackenbush::Color;
    let tallest = stalks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    for row in (0..tallest).rev() {
        out.push_str("   ");
        for stalk in stalks {
            match stalk.get(row) {
                Some(Color::Red) => out.push_str("\x1b[91m R \x1b[0m"),
                Some(Color::Blue) => out.push_str("\x1b[94m B \x1b[0m"),
                None => out.push_str("   "),
            }
            out.push(' ');
        }
        out.push('\n');
    }
    out.push_str("   ");
    for (i, _) in stalks.iter().enumerate() {
        out.push_str(&format!("={}= ", i + 1));
    }
    out.push('\n');
    out
}

/// Hackenbush against the Order: cut red, it cuts blue, last cutter wins.
fn hackenbush(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    hackenbush_with_input(seed, journey, &mut input)
}

fn hackenbush_with_input(seed: u64, journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    use numinous_core::hackenbush as hb;
    let mut stalks = hb::new_garden(seed);
    let mut played = false;
    println!("HACKENBUSH  seed {seed}. Cut a RED segment; everything above it falls.");
    println!("The Order cuts blue. Whoever cannot cut, loses. Answer: stalk height");
    println!("(1 1 cuts stalk 1 at the ground). This garden is winnable. (? explains)");
    loop {
        println!("\n{}", garden_text(&stalks));
        if !hb::can_move(&stalks, hb::Color::Red) {
            println!("No red left to cut. The Order takes the garden. (It was arithmetic.)");
            return ExitCode::SUCCESS;
        }
        let Some(line) = read_game_line(input, "stalk height > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "hackenbush") {
            continue;
        }
        let nums: Vec<usize> = line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (Some(&stalk), Some(&height)) = (nums.first(), nums.get(1)) else {
            println!("  Two numbers: which stalk, which height (both from 1).");
            continue;
        };
        if stalk == 0 || height == 0 || !hb::cut(&mut stalks, stalk - 1, height - 1, hb::Color::Red)
        {
            println!("  That is not a red segment you can reach.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        if !hb::can_move(&stalks, hb::Color::Blue) {
            journey.win();
            post_score(&format!("hackenbush seed:{seed}"), 1);
            println!("\nThe Order has nothing left to cut. It concedes, and keeps its word:");
            println!("\n{}", hb::the_secret());
            return ExitCode::SUCCESS;
        }
        let (bi, bh) = hb::order_move(&stalks).expect("blue can move");
        let _ = hb::cut(&mut stalks, bi, bh, hb::Color::Blue);
        println!("  The Order cuts stalk {} at height {}.", bi + 1, bh + 1);
    }
}

/// The Party Problem: round one, five guests (escapable); round two, six.
fn party(journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    party_with_input(journey, &mut input)
}

fn party_with_input(journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    use numinous_core::party::{Party, Shade};
    println!("THE PARTY PROBLEM. Shade every handshake red or blue WITHOUT making");
    println!("a triangle of one color. Answer like: 1 3 r   (guests 1 and 3, red).");
    println!("Round one: five guests. It can be done. (? explains)\n");
    for (round, guests) in [(1usize, 5usize), (2, 6)] {
        let mut played = false;
        let mut p = Party::new(guests);
        println!(
            "ROUND {round}: {guests} guests, {} handshakes.",
            p.edges.len()
        );
        loop {
            // The matrix of handshakes so far.
            print!("     ");
            for b in 1..=guests {
                print!(" {b}");
            }
            println!();
            for a in 0..guests {
                print!("   {} ", a + 1);
                for b in 0..guests {
                    if b <= a {
                        print!("  ");
                        continue;
                    }
                    let mark =
                        match numinous_core::party::edge_index(guests, a, b).map(|i| p.edges[i]) {
                            Some(Shade::Red) => "\x1b[91mR\x1b[0m",
                            Some(Shade::Blue) => "\x1b[94mB\x1b[0m",
                            _ => ".",
                        };
                    print!(" {mark}");
                }
                println!();
            }
            let Some(line) = read_game_line(input, "handshake > ") else {
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "party") {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let (Some(a), Some(b), Some(color)) = (
                words.first().and_then(|w| w.parse::<usize>().ok()),
                words.get(1).and_then(|w| w.parse::<usize>().ok()),
                words.get(2),
            ) else {
                println!("  Like this: 1 3 r   or   2 5 b");
                continue;
            };
            let shade = match color.chars().next().map(|c| c.to_ascii_lowercase()) {
                Some('r') => Shade::Red,
                Some('b') => Shade::Blue,
                _ => {
                    println!("  Color must be r or b.");
                    continue;
                }
            };
            if a == 0 || b == 0 || !p.shade(a - 1, b - 1, shade) {
                println!("  That handshake is not open.");
                continue;
            }
            if !played {
                journey.play();
                played = true;
            }
            if let Some((x, y, z, _)) = p.mono_triangle() {
                println!(
                    "\nA one-color triangle: guests {}, {}, {}. {} handshakes survived.",
                    x + 1,
                    y + 1,
                    z + 1,
                    p.shaded() - 1
                );
                if guests == 6 {
                    println!(
                        "It was never possible. Among six, three mutual friends or three\n\
                         mutual strangers MUST exist: R(3,3) = 6. You just felt a theorem."
                    );
                } else {
                    println!(
                        "Five guests CAN escape. The pentagon knows: ring one color, star the other."
                    );
                }
                break;
            }
            if p.complete() {
                journey.win();
                post_score(&format!("party guests:{guests}"), p.shaded() as i64);
                println!(
                    "\nEvery handshake shaded, no triangle. You escaped with all {}.",
                    p.shaded()
                );
                if guests == 5 {
                    println!("Now try six. (Ramsey is waiting.)\n");
                }
                break;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Fifteen's Bet: call each scramble solvable or stuck forever.
fn fifteen(seed: u64, rounds: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    fifteen_with_input(seed, rounds, journey, &mut input)
}

fn fifteen_with_input(
    seed: u64,
    rounds: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    use numinous_core::fifteen as ff;
    let mut called = 0u64;
    let mut completed = 0u64;
    println!("FIFTEEN'S BET. Half of all scrambles can never be solved, and one");
    println!("invisible quantity decides which. Call each one: S(olvable) or U(nsolvable).");
    println!("(? explains)\n");
    for n in 0..rounds {
        let tiles = ff::deal(seed, n);
        println!(
            "SCRAMBLE {} of {rounds}:\n{}",
            n + 1,
            ff::board_text(&tiles)
        );
        let verdict = loop {
            let Some(line) = read_game_line(input, "S or U > ") else {
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "fifteen") {
                continue;
            }
            match line
                .chars()
                .find(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase())
            {
                Some('S') => break true,
                Some('U') => break false,
                _ => println!("  S or U."),
            }
        };
        journey.play();
        completed += 1;
        let truth = ff::solvable(&tiles);
        if verdict == truth {
            called += 1;
            journey.win();
            println!("  Called it. {}\n", ff::why(&tiles));
        } else {
            println!("  No: {}\n", ff::why(&tiles));
        }
    }
    if completed > 0 {
        post_score(
            &format!("fifteen seed:{seed} rounds:{completed}"),
            called as i64,
        );
    }
    println!("{called} of {rounds} called. Parity is learnable; deal again.");
    ExitCode::SUCCESS
}

/// Draw the heaps as rows of stones.
fn nim_board(heaps: &[u32]) -> String {
    heaps
        .iter()
        .enumerate()
        .map(|(i, &h)| format!("  {}) {}", i + 1, "O ".repeat(h as usize)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Play nim against the Order. Winning earns the secret, spoken in full.
fn nim(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    nim_with_input(seed, journey, &mut input)
}

fn nim_with_input(seed: u64, journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    let mut heaps = numinous_core::nim_new(seed);
    let mut played = false;
    println!("NIM  seed {seed}. On your turn, take ANY number of stones from ONE heap.");
    println!("Whoever takes the last stone wins. Answer like: 2 3  (heap 2, take 3).");
    println!("The Order plays a secret. Beat it and the secret is yours. (? explains)");
    loop {
        println!("\n{}", nim_board(&heaps));
        let Some(line) = read_game_line(input, "heap amount > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "nim") {
            continue;
        }
        let nums: Vec<u32> = line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (Some(&heap), Some(&take)) = (nums.first(), nums.get(1)) else {
            println!("  Two numbers: which heap, how many. Like: 2 3");
            continue;
        };
        if heap == 0 || !numinous_core::nim_apply(&mut heaps, heap as usize - 1, take) {
            println!("  That move is not on the board.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        if numinous_core::nim_finished(&heaps) {
            journey.win();
            post_score(&format!("nim seed:{seed}"), 1);
            println!("\nYou took the last stone. The Order concedes, and keeps its word:");
            println!("\n{}", numinous_core::nim_secret());
            return ExitCode::SUCCESS;
        }
        let (oh, ot) = numinous_core::nim_order(&heaps);
        let _ = numinous_core::nim_apply(&mut heaps, oh, ot);
        println!("  The Order takes {ot} from heap {}.", oh + 1);
        if numinous_core::nim_finished(&heaps) {
            println!("\nThe Order takes the last stone. Again. (It is not luck.)");
            return ExitCode::SUCCESS;
        }
    }
}

/// Combo math for the Gauntlet: cleared stages multiply what follows.
/// The multiplier starts at 1, rises by 1 after every cleared stage, and falls
/// back to 1 after a miss. Pure, so it is tested.
fn gauntlet_total(stage_scores: &[i64], cleared: &[bool]) -> i64 {
    let mut total = 0i64;
    let mut combo = 1i64;
    for (score, &clear) in stage_scores.iter().zip(cleared) {
        total += score * combo;
        combo = if clear { combo + 1 } else { 1 };
    }
    total
}

/// The first answer letter in one already-read input line.
fn letter_from_line(line: &str) -> Option<char> {
    line.chars()
        .find(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
}

/// The Gauntlet: munch board, mystery shape, sky scan, bomb code, one run.
/// Opt-in, bounded, and over in minutes: a shape for a session, not a trap.
fn gauntlet(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    gauntlet_with_input(seed, journey, &mut input)
}

fn gauntlet_with_input(seed: u64, journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    let mut stage_scores = Vec::new();
    let mut cleared = Vec::new();
    println!(
        "THE GAUNTLET  seed {seed}. Four stages. Clears build your combo.
"
    );

    // Stage 1: one munch board.
    let board = numinous_core::build_board(seed, 0);
    println!("STAGE 1 of 4  MUNCH: {}", board.rule.describe());
    print!("{}", numinous_core::board_text(&board));
    let line = loop {
        let Some(line) = read_game_line(input, "Your bites > ") else {
            return ExitCode::SUCCESS;
        };
        if !asked_why(&line, "gauntlet") {
            break line;
        }
    };
    journey.play();
    let bites: Vec<usize> = line
        .split_whitespace()
        .filter_map(|w| w.parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .map(|n| n - 1)
        .collect();
    let outcome = numinous_core::grade_munch(&board, &bites);
    let clear = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
    if clear {
        journey.win();
    }
    println!(
        "  +{} points{}
",
        outcome.score,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(outcome.score);
    cleared.push(clear);

    // Stage 2: one mystery shape.
    let round = numinous_core::build_round(seed, 1, 44, 18);
    println!("STAGE 2 of 4  THE SHAPE:");
    print!("{}", round.art);
    for choice in &round.choices {
        println!("  {}) {}", choice.letter, choice.title);
    }
    let guess = loop {
        let Some(line) = read_game_line(input, "Your answer > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "gauntlet") {
            continue;
        }
        let Some(guess) = letter_from_line(&line) else {
            println!("  Answer with a choice letter.");
            continue;
        };
        break guess;
    };
    journey.play();
    let clear = guess == round.answer;
    if clear {
        journey.win();
    }
    let points = if clear { 25 } else { 0 };
    println!(
        "  It was {} ({}). +{points} points{}
",
        round.answer,
        round.answer_title,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(points);
    cleared.push(clear);

    // Stage 3: one sky scan.
    let scan = numinous_core::build_scan(seed, 4);
    println!("STAGE 3 of 4  THE SKY:");
    for channel in &scan.channels {
        println!(
            "  {})  {:>10}  |{}|",
            channel.letter, channel.frequency, channel.trace
        );
    }
    let guess = loop {
        let Some(line) = read_game_line(input, "Which is a mind > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "gauntlet") {
            continue;
        }
        let Some(guess) = letter_from_line(&line) else {
            println!("  Answer with a channel letter.");
            continue;
        };
        break guess;
    };
    journey.play();
    let clear = guess == scan.answer;
    if clear {
        journey.win();
    }
    let points = if clear { 25 } else { 0 };
    println!(
        "  The signal was {}. +{points} points{}
",
        scan.answer,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(points);
    cleared.push(clear);

    // Stage 4: the bomb, four digits, five tries.
    let secret = numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4);
    println!("STAGE 4 of 4  THE BOMB. Four digits, five tries.");
    println!("  Clue: {}", numinous_core::hint(&secret));
    let mut points = 0i64;
    let mut clear = false;
    let mut played = false;
    for attempt in 1..=5 {
        let Some(line) = read_game_line(input, &format!("Wire {attempt}/5 > ")) else {
            return ExitCode::SUCCESS;
        };
        let guess: Vec<u8> = line
            .trim()
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != 4 {
            println!("  Four digits.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        let feedback = numinous_core::grade(&secret, &guess);
        if feedback.locked == 4 {
            clear = true;
            points = 10 * (5 - i64::from(attempt as u8));
            journey.win();
            word_in_lights("DEFUSED", [90, 230, 120], 5);
            println!("  +{points} points  CLEAN\n");
            break;
        }
        println!("  {} locked, {} loose.", feedback.locked, feedback.loose);
    }
    if !clear {
        let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
        word_in_lights("BOOM", [255, 90, 40], 5);
        println!("  It was {code}. +0 points\n");
    }
    stage_scores.push(points);
    cleared.push(clear);

    // The one honest number.
    let total = gauntlet_total(&stage_scores, &cleared);
    let clears = cleared.iter().filter(|&&c| c).count();
    post_score(&format!("gauntlet seed:{seed}"), total);
    println!("RUN COMPLETE  {clears}/4 clean  TOTAL {total}  (gauntlet seed:{seed})");
    ExitCode::SUCCESS
}

/// Play Munch: eat the numbers that fit the rule, round by round, scored.
fn munch(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    munch_with_input(seed, rounds, journey, &mut input)
}

fn munch_with_input(
    seed: u64,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut total = 0i64;
    println!("MUNCH. Eat by cell number, e.g. \"1 7 22\". Wrong bites cost you. (? explains)\n");
    for round in 0..rounds {
        let board = numinous_core::build_board(seed, round as u64);
        println!("Board {} of {rounds}: {}", round + 1, board.rule.describe());
        print!("{}", numinous_core::board_text(&board));
        let line = loop {
            let Some(line) = read_game_line(input, "Your bites > ") else {
                println!("Final score: {total} (seed {seed}).");
                return ExitCode::SUCCESS;
            };
            if !asked_why(&line, "munch") {
                break line;
            }
        };
        journey.play();
        let bites: Vec<usize> = line
            .split_whitespace()
            .filter_map(|w| w.parse::<usize>().ok())
            .filter(|&n| n >= 1)
            .map(|n| n - 1)
            .collect();
        let outcome = numinous_core::grade_munch(&board, &bites);
        post_score(&format!("munch seed:{seed} board:{round}"), outcome.score);
        if outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0 {
            journey.win();
            println!(
                "PERFECT. {} eaten, nothing wasted. +{} points.\n",
                outcome.hits, outcome.score
            );
        } else {
            println!(
                "{} eaten, {} bad bites, {} left behind. +{} points.",
                outcome.hits, outcome.bad_bites, outcome.left_behind, outcome.score
            );
            // The dense feedback: exactly which judgments went wrong.
            if !outcome.wrongly_eaten.is_empty() {
                let bad: Vec<String> = outcome.wrongly_eaten.iter().map(u64::to_string).collect();
                println!("  Not {}: {}.", board.rule.describe(), bad.join(", "));
            }
            if !outcome.missed.is_empty() {
                let missed: Vec<String> = outcome.missed.iter().map(u64::to_string).collect();
                println!("  You walked past: {}.", missed.join(", "));
                if outcome.bad_bites == 0 && outcome.missed.len() == 1 {
                    println!("  One away. The board remembers.");
                }
            }
            println!();
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
    quiz_with_input(rounds, seed, width, height, choices, journey, &mut input)
}

fn quiz_with_input(
    rounds: usize,
    seed: u64,
    width: usize,
    height: usize,
    choices: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    let mut recent: Vec<&'static str> = Vec::new();
    println!("Guess the shape. Name the math behind each mystery render.\n");
    for round in 0..rounds {
        // Recently asked rooms sit out: no repeated questions in a session.
        let all: Vec<&'static str> = all_rooms().iter().map(|r| r.meta().id).collect();
        let fresh: Vec<&'static str> = all
            .iter()
            .copied()
            .filter(|id| !recent.contains(id))
            .collect();
        let pool = if fresh.len() > choices { fresh } else { all };
        let r = numinous_core::build_round_pool(seed, round as u64, width, height, choices, &pool);
        if let Some(choice) = r.choices.iter().find(|c| c.letter == r.answer) {
            recent.push(choice.id);
            if recent.len() > 10 {
                recent.remove(0);
            }
        }
        println!("Mystery #{} of {rounds}:", round + 1);
        print!("{}", r.art);
        println!();
        for choice in &r.choices {
            println!("  {}) {}", choice.letter, choice.title);
        }
        let guess = loop {
            let Some(line) = read_game_line(input, "Your answer: ") else {
                if completed > 0 {
                    post_score(
                        &format!("quiz seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                println!("Final score: {score}/{completed}.");
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "quiz") {
                continue;
            }
            let Some(guess) = letter_from_line(&line) else {
                println!("  Answer with a choice letter.");
                continue;
            };
            break guess;
        };
        journey.play();
        completed += 1;
        if guess == r.answer {
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
    if completed > 0 {
        post_score(
            &format!("quiz seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!(
        "Final score: {score}/{completed}. {}",
        quiz_remark(score, completed)
    );
    ExitCode::SUCCESS
}

/// Build one terminal frame: clear the screen, render the room, and add a status
/// line. Pure and testable; the animation loop just prints these in sequence.
fn play_frame(room: &dyn Room, t: f64, width: usize, height: usize) -> String {
    let mut canvas = Canvas::new(width, height);
    room.render(&mut canvas, t);
    let status = room
        .status(t)
        .map(|readout| format!("   {readout}"))
        .unwrap_or_default();
    // \x1b[2J clears the screen, \x1b[H moves the cursor home.
    format!(
        "\x1b[2J\x1b[H{}\n[{}]  {}   t = {t:.2}{status}   (Ctrl+C to stop)\n",
        canvas.to_text(),
        room.meta().title,
        numinous_core::room_action(room)
    )
}

/// Animate a room in the terminal, sweeping its phase, until interrupted.
fn play(
    id: &str,
    fps: f64,
    width: usize,
    height: usize,
    allow_hidden: bool,
    variation: u64,
) -> ExitCode {
    let room = find_room_with_variation(id, allow_hidden, variation);
    let Some(room) = room else {
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
    use super::{
        Cli, Command, RoomRenderInput, SonifyLayer, bounded_response_detail, describe_report,
        load_studio_creation, max_track_bytes, meta_json, not_found_message, open_studio_report,
        parse_poke_arg, parse_pokes, read_bounded, render_report, rooms_report, run,
        save_studio_creation, validate_pcm_body,
    };

    #[test]
    fn explicit_command_stack_returns_success_and_contains_panics() {
        assert_eq!(
            super::run_on_command_stack(|| std::process::ExitCode::SUCCESS),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(
            super::run_on_command_stack(|| panic!("command stack probe")),
            std::process::ExitCode::FAILURE
        );
    }

    #[test]
    fn test_persistence_paths_never_resolve_to_the_player_profile() {
        assert!(super::journey_path().starts_with(std::env::temp_dir()));
        assert!(super::scores_path().starts_with(std::env::temp_dir()));
    }

    #[test]
    fn redirected_home_report_is_plain_and_useful() {
        let report = super::home_report(&numinous_core::Journey::default(), false);
        assert!(
            !report.contains('\x1b'),
            "redirected output must not contain ANSI"
        );
        assert!(report.starts_with("NUMINOUS: math you can feel\n"));
        assert!(report.contains("Today's room:"));
        assert!(report.contains("\n  numinous watch"));
        assert!(report.contains("\n  numinous rooms"));
        assert!(report.contains("\n  numinous play"));
        assert!(report.contains("\n  numinous --help"));
        assert!(report.lines().count() <= 16, "plain home stays concise");
    }

    #[test]
    fn interactive_home_report_keeps_the_full_color_cabinet() {
        let journey = numinous_core::Journey {
            streak: 3,
            ..Default::default()
        };
        let report = super::home_report(&journey, true);
        assert!(
            report.starts_with("\x1b[38;2;"),
            "cabinet starts in truecolor"
        );
        assert!(
            report.contains("\x1b[48;2;"),
            "cabinet paints its background"
        );
        assert!(
            report.contains("\x1b[0m"),
            "cabinet restores terminal color"
        );
        assert!(report.contains('▀'), "cabinet keeps its half-block raster");
        assert!(report.contains("NUMINOUS   LV"));
        assert!(report.contains("streak 3"));
        assert!(report.contains("\n  numinous watch"));
    }

    #[test]
    fn eof_is_a_neutral_departure_from_every_cli_game() {
        fn check(
            name: &str,
            run: impl FnOnce(
                &mut numinous_core::Journey,
                &mut std::io::Cursor<Vec<u8>>,
            ) -> std::process::ExitCode,
        ) {
            let scores = super::scores_path();
            let _ = std::fs::remove_file(&scores);
            let mut journey = numinous_core::Journey::default();
            let before = journey.clone();
            let mut eof = std::io::Cursor::new(Vec::new());
            assert_eq!(run(&mut journey, &mut eof), std::process::ExitCode::SUCCESS);
            assert_eq!(journey, before, "{name} counted EOF as play");
            assert!(!scores.exists(), "{name} posted a score for EOF");
        }

        check("crack", |journey, input| {
            super::crack_with_input(1, 4, 1, journey, input)
        });
        check("seti", |journey, input| {
            super::seti_with_input(1, 4, 1, journey, input)
        });
        check("aliens", |journey, input| {
            super::aliens_with_input(1, 1, journey, input)
        });
        check("munch", |journey, input| {
            super::munch_with_input(1, 1, journey, input)
        });
        check("arcade", |journey, input| {
            super::arcade_with_input(1, journey, input)
        });
        check("hackenbush", |journey, input| {
            super::hackenbush_with_input(1, journey, input)
        });
        check("party", |journey, input| {
            super::party_with_input(journey, input)
        });
        check("fifteen", |journey, input| {
            super::fifteen_with_input(1, 1, journey, input)
        });
        check("nim", |journey, input| {
            super::nim_with_input(1, journey, input)
        });
        check("gauntlet", |journey, input| {
            super::gauntlet_with_input(1, journey, input)
        });
        check("quiz", |journey, input| {
            super::quiz_with_input(1, 1, 40, 18, 4, journey, input)
        });
    }

    #[test]
    fn room_render_boundaries_reject_empty_nonfinite_and_backwards_input() {
        for (width, height, phase) in [
            (0, 20, 0.0),
            (40, 0, 0.0),
            (40, 20, -1.0),
            (40, 20, 1.0),
            (40, 20, f64::NAN),
            (40, 20, f64::INFINITY),
            (4097, 20, 0.0),
        ] {
            assert!(
                super::validate_render_request(width, height, phase).is_err(),
                "accepted {width}x{height} at {phase}"
            );
        }
        assert!(super::validate_render_request(4096, 4096, 0.5).is_ok());

        let backwards = vec![
            "down:0.2,0.3,0.8".to_string(),
            "move:0.4,0.5,0.2".to_string(),
        ];
        let error = super::parse_gestures(&backwards).expect_err("backwards timestamp");
        assert!(error.contains("nondecreasing"));
        let ordered = vec![
            "down:0.2,0.3,0.2".to_string(),
            "move:0.4,0.5,0.8".to_string(),
            "up:0.4,0.5,0.8".to_string(),
        ];
        assert!(super::parse_gestures(&ordered).is_ok());
    }

    #[test]
    fn test_persistence_paths_are_stable_per_test_and_isolated_between_threads() {
        let journey = super::journey_path();
        assert_eq!(journey, super::journey_path());
        assert_ne!(journey, super::scores_path());

        let other = std::thread::spawn(|| {
            let path = super::journey_path();
            std::fs::write(&path, b"other test").expect("test state should be writable");
            path
        })
        .join()
        .expect("path worker should finish");
        assert_ne!(journey, other);
        assert!(!other.exists());
        assert!(
            !other
                .parent()
                .expect("state path should have a parent")
                .exists()
        );
    }

    #[test]
    fn test_state_root_clears_stale_data_rejects_files_and_cleans_on_drop() {
        let parent = super::journey_path()
            .parent()
            .expect("state path should have a parent")
            .to_path_buf();
        let stale_root = parent.join("stale-root");
        std::fs::create_dir_all(&stale_root).expect("stale root should be creatable");
        std::fs::write(stale_root.join("old.txt"), b"stale")
            .expect("stale state should be writable");

        let root = super::TestStateRoot::at(stale_root.clone());
        assert!(stale_root.exists());
        assert!(!stale_root.join("old.txt").exists());
        drop(root);
        assert!(!stale_root.exists());

        let file_collision = parent.join("file-collision");
        std::fs::write(&file_collision, b"not a directory")
            .expect("collision file should be writable");
        let rejected =
            std::panic::catch_unwind(|| super::TestStateRoot::at(file_collision.clone()));
        assert!(rejected.is_err());
        std::fs::remove_file(file_collision).expect("collision file should be removable");
    }
    use clap::Parser;
    use numinous_core::room_by_id;
    use serde_json::Value;

    #[test]
    fn bounded_response_reader_distinguishes_exact_and_oversized_bodies() {
        let exact = read_bounded(std::io::Cursor::new(b"1234"), 4).expect("bounded read");
        assert_eq!(exact.as_deref(), Some(b"1234".as_slice()));
        let oversized = read_bounded(std::io::Cursor::new(b"12345"), 4).expect("bounded read");
        assert!(oversized.is_none());
    }

    #[test]
    fn bounded_cli_input_preserves_boundaries_and_resynchronizes() {
        let mut exact = vec![b'x'; super::MAX_CLI_INPUT_BYTES];
        exact.push(b'\n');
        let mut exact = std::io::Cursor::new(exact);
        assert!(matches!(
            super::read_bounded_input_line(&mut exact).expect("exact LF line"),
            super::BoundedInputLine::Line(line)
                if line.len() == super::MAX_CLI_INPUT_BYTES + 1
        ));

        let mut crlf = vec![b'x'; super::MAX_CLI_INPUT_BYTES];
        crlf.extend_from_slice(b"\r\n");
        let mut crlf = std::io::Cursor::new(crlf);
        assert!(matches!(
            super::read_bounded_input_line(&mut crlf).expect("exact CRLF line"),
            super::BoundedInputLine::Line(line)
                if line.len() == super::MAX_CLI_INPUT_BYTES + 2
        ));

        let mut overflow = vec![b'x'; super::MAX_CLI_INPUT_BYTES + 1];
        overflow.extend_from_slice(b"\nok\n");
        let mut overflow = std::io::Cursor::new(overflow);
        assert_eq!(
            super::read_bounded_input_line(&mut overflow).expect("overlong line"),
            super::BoundedInputLine::TooLong
        );
        assert_eq!(
            super::read_bounded_input_line(&mut overflow).expect("following line"),
            super::BoundedInputLine::Line("ok\n".to_string())
        );

        let mut eof = std::io::Cursor::new(vec![b'x'; super::MAX_CLI_INPUT_BYTES]);
        assert!(matches!(
            super::read_bounded_input_line(&mut eof).expect("exact EOF line"),
            super::BoundedInputLine::Line(line)
                if line.len() == super::MAX_CLI_INPUT_BYTES
        ));
        assert_eq!(
            super::read_bounded_input_line(&mut eof).expect("EOF"),
            super::BoundedInputLine::Eof
        );
    }

    #[test]
    fn overlong_choose_and_game_input_leave_progress_unchanged() {
        let mut input = vec![b'1'; super::MAX_CLI_INPUT_BYTES + 1];
        input.push(b'\n');

        let mut choosing = numinous_core::Journey::from_text("plays 1");
        let before = choosing.clone();
        assert_eq!(
            super::choose_with_input(&mut choosing, &mut std::io::Cursor::new(input.clone())),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(choosing, before, "overlong choice spent a boon");

        let scores = super::scores_path();
        let _ = std::fs::remove_file(&scores);
        let mut playing = numinous_core::Journey::default();
        let before = playing.clone();
        assert_eq!(
            super::crack_with_input(1, 4, 1, &mut playing, &mut std::io::Cursor::new(input),),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(playing, before, "overlong game line counted as a move");
        assert!(!scores.exists(), "overlong game line posted a score");
    }

    #[test]
    fn music_response_helpers_bound_diagnostics_duration_and_pcm_shape() {
        assert_eq!(
            bounded_response_detail(std::io::Cursor::new(b"detail")),
            "detail"
        );
        assert_eq!(
            bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024 + 1])),
            "response detail unavailable or oversized"
        );
        assert_eq!(max_track_bytes(10), Some(12 * 44_100 * 4));
        assert_eq!(max_track_bytes(u64::MAX), None);
        assert_eq!(
            validate_pcm_body(&[0]),
            Err("The tower sent an incomplete 16-bit stereo frame")
        );
        assert_eq!(
            validate_pcm_body(&vec![0; 8_820 * 2 + 2]),
            Err("The tower sent an incomplete 16-bit stereo frame")
        );
        assert_eq!(
            validate_pcm_body(&[0, 0, 0, 0]),
            Err("The tower sent almost nothing")
        );
        assert!(validate_pcm_body(&vec![0; 8_820 * 2]).is_ok());
    }

    #[test]
    fn music_response_detail_escapes_terminal_controls() {
        let detail = bounded_response_detail(std::io::Cursor::new(
            b"plain\x1b[31m\nforged\rline\x07\tend",
        ));
        assert!(detail.starts_with("plain"));
        assert!(detail.ends_with("end"));
        assert!(
            !detail.chars().any(char::is_control),
            "diagnostic retained a control character: {detail:?}"
        );

        let exact = bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024]));
        assert_eq!(exact.len(), 8 * 1024);
        let oversized = bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024 + 1]));
        assert_eq!(oversized, "response detail unavailable or oversized");
    }

    #[test]
    fn music_request_does_not_follow_redirects_or_forward_the_key() {
        let destination = std::net::TcpListener::bind("127.0.0.1:0").expect("destination");
        let destination_address = destination.local_addr().expect("destination address");
        let origin = std::net::TcpListener::bind("127.0.0.1:0").expect("origin");
        let origin_address = origin.local_addr().expect("origin address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = origin.accept().expect("origin request");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("read timeout");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = std::io::Read::read(&mut stream, &mut chunk).expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            let header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
                .expect("complete request headers");
            let content_length = String::from_utf8_lossy(&request[..header_end])
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            while request.len() < header_end + content_length {
                let read = std::io::Read::read(&mut stream, &mut chunk).expect("read body");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{destination_address}/capture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("redirect");
            request
        });

        let result = super::send_music_request(
            &format!("http://{origin_address}/music"),
            "dummy-validation-key",
            "{}",
            std::time::Duration::from_secs(2),
        );
        match result {
            Err(error) => assert!(
                matches!(&*error, ureq::Error::Status(302, _)),
                "unexpected redirect error: {error:?}"
            ),
            Ok(response) => panic!("redirect returned HTTP {}", response.status()),
        }
        let request = server.join().expect("origin server");
        let request = String::from_utf8(request).expect("ASCII request");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("xi-api-key: dummy-validation-key")
        );

        destination.set_nonblocking(true).expect("nonblocking");
        let error = destination
            .accept()
            .expect_err("redirect destination received the request")
            .kind();
        assert_eq!(error, std::io::ErrorKind::WouldBlock);
    }

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
    fn env_file_key_reads_key_without_unbounded_file_loads() {
        let dir = std::env::temp_dir().join("numinous_cli_env_file_key");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test dir");
        let path = dir.join(".env");

        std::fs::write(
            &path,
            "# local secrets\nOTHER=value\nELEVENLABS_API_KEY='test-key'\n",
        )
        .expect("write env file");
        assert_eq!(super::env_file_key_from(&path).expect("key"), "test-key");

        std::fs::write(&path, "x".repeat(super::MAX_ENV_FILE_BYTES as usize + 1))
            .expect("write oversized env file");
        assert!(super::env_file_key_from(&path).is_err());

        std::fs::write(&path, "OTHER=value\n").expect("write keyless env file");
        assert!(super::env_file_key_from(&path).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn describe_known_room_reports_its_wing() {
        let text = describe_report(
            "times-tables",
            false,
            false,
            &numinous_core::Journey::default(),
        )
        .expect("known room");
        assert!(text.contains("Number & Pattern"));
        assert!(text.contains("Action: DRAG: TURN THE DIAL"));
        assert!(text.contains("Goal: LAND ON EXACTLY 4 LOBES"));
    }

    #[test]
    fn describe_json_carries_the_id() {
        let text = describe_report(
            "times-tables",
            true,
            false,
            &numinous_core::Journey::default(),
        )
        .expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value["id"], "times-tables");
        assert_eq!(value["action"], "DRAG: TURN THE DIAL");
        assert_eq!(value["goal"], "LAND ON EXACTLY 4 LOBES");
    }

    #[test]
    fn describe_includes_the_reveal() {
        let text = describe_report(
            "times-tables",
            false,
            false,
            &numinous_core::Journey::default(),
        )
        .expect("known room");
        assert!(text.contains("Reveal:"));
        assert!(text.contains("Mandelbrot"));
    }

    #[test]
    fn describe_json_includes_the_reveal() {
        let text = describe_report(
            "times-tables",
            true,
            false,
            &numinous_core::Journey::default(),
        )
        .expect("known room");
        let value: Value = serde_json::from_str(&text).expect("valid json");
        assert!(
            value["reveal"]
                .as_str()
                .is_some_and(|s| s.contains("Mandelbrot"))
        );
    }

    #[test]
    fn describe_unknown_room_guides_the_user() {
        let err = describe_report(
            "no-such-room",
            false,
            false,
            &numinous_core::Journey::default(),
        )
        .expect_err("unknown room");
        assert!(err.contains("Known rooms"));
    }

    #[test]
    fn render_known_room_has_ink() {
        let text = render_report("times-tables", 40, 20, 0.0, false, RoomRenderInput::plain())
            .expect("known room");
        assert!(text.contains('*'));
        assert!(text.contains("Action: DRAG: TURN THE DIAL"));
        assert!(text.contains("Goal: LAND ON EXACTLY 4 LOBES"));
        assert!(!text.contains("Aha earned:"));
    }

    #[test]
    fn times_tables_goal_earns_the_aha_and_reveal_from_hand_input() {
        let report = render_report(
            "times-tables",
            72,
            32,
            0.8,
            false,
            RoomRenderInput::new(0, &[(0.374, 0.5)]),
        )
        .expect("goal render");

        assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  FOUND"));
        assert!(report.contains("Goal: LAND ON EXACTLY 4 LOBES"));
        assert!(report.contains("Aha earned: LAND ON EXACTLY 4 LOBES"));
        assert!(report.contains("Reveal: Set the dial to 2"));
    }

    #[test]
    fn times_tables_ambient_target_does_not_claim_an_earned_discovery() {
        let report = render_report(
            "times-tables",
            72,
            32,
            0.375,
            false,
            RoomRenderInput::plain(),
        )
        .expect("ambient target render");

        assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  TARGET 4"));
        assert!(!report.contains("FOUND"));
        assert!(!report.contains("Aha earned:"));
    }

    #[test]
    fn render_unknown_room_is_error() {
        assert!(
            render_report("no-such-room", 10, 10, 0.0, false, RoomRenderInput::plain(),).is_err()
        );
    }

    #[test]
    fn parse_pokes_keep_hand_points_normalized() {
        assert_eq!(parse_poke_arg("0.25,0.75"), Ok((0.25, 0.75)));
        assert_eq!(
            parse_pokes(&["0,1".to_string(), "1,0".to_string()]),
            Ok(vec![(0.0, 1.0), (1.0, 0.0)])
        );
        assert!(parse_poke_arg("0.5").is_err());
        assert!(parse_poke_arg("-0.1,0.5").is_err());
        assert!(parse_poke_arg("0.5,NaN").is_err());
        let too_many = vec!["0.5,0.5".to_string(); numinous_core::MAX_ROOM_POKES + 1];
        assert!(parse_pokes(&too_many).is_err());
    }

    #[test]
    fn render_report_uses_hand_points_when_supplied() {
        let resting = render_report(
            "double-pendulum",
            50,
            30,
            0.25,
            false,
            RoomRenderInput::plain(),
        )
        .expect("resting room");
        let poked = render_report(
            "double-pendulum",
            50,
            30,
            0.25,
            false,
            RoomRenderInput::new(0, &[(0.2, 0.8)]),
        )
        .expect("poked room");
        assert_ne!(
            resting, poked,
            "a supplied hand point should steer the frame"
        );
        let last_only = render_report(
            "double-pendulum",
            50,
            30,
            0.25,
            false,
            RoomRenderInput::new(0, &[(0.8, 0.2)]),
        )
        .expect("last-only poked room");
        let newest_last = render_report(
            "double-pendulum",
            50,
            30,
            0.25,
            false,
            RoomRenderInput::new(0, &[(0.2, 0.8), (0.8, 0.2)]),
        )
        .expect("multi-poked room");
        assert_eq!(
            last_only, newest_last,
            "Double Pendulum should treat the newest hand point as the re-drop"
        );
    }

    #[test]
    fn interacted_render_reports_the_room_specific_consequence() {
        let resting = render_report(
            "cult-of-pi",
            50,
            24,
            0.0,
            false,
            RoomRenderInput::new(0, &[]),
        )
        .expect("resting cult render");
        let report = render_report(
            "cult-of-pi",
            50,
            24,
            0.0,
            false,
            RoomRenderInput::new(0, &[(0.5, 0.5)]),
        )
        .expect("cult render");
        assert!(
            report.contains("Status: 1 HELD FIX0 D") && report.contains(" CH01"),
            "interaction status must reach the CLI: {report}"
        );
        assert!(
            report.split("\nStatus:").next() != resting.split("\nStatus:").next(),
            "a phase-zero hold must visibly change the character frame: {report}"
        );
    }

    #[test]
    fn static_render_reports_every_available_core_readout() {
        let phase = 0.37;
        let mut checked = 0;
        for room in numinous_core::all_rooms() {
            let Some(status) = room.status(phase) else {
                continue;
            };
            let report = render_report(
                room.meta().id,
                50,
                24,
                phase,
                false,
                RoomRenderInput::plain(),
            )
            .expect("catalog room renders");
            assert!(
                report.contains(&format!("Status: {status}")),
                "{} must expose its shared-core readout",
                room.meta().id
            );
            assert!(
                report.contains(&format!(
                    "Action: {}",
                    numinous_core::room_action(room.as_ref())
                )),
                "{} must expose its shared-core action",
                room.meta().id
            );
            if let Some(goal) = room.goal() {
                assert!(
                    report.contains(&format!("Goal: {goal}")),
                    "{} must expose its shared-core goal",
                    room.meta().id
                );
            }
            checked += 1;
        }
        assert!(
            checked >= 11,
            "status coverage unexpectedly shrank: {checked}"
        );
    }

    #[test]
    fn default_munch_session_reaches_the_complete_rule_deck() {
        use clap::Parser;
        let cli = super::Cli::try_parse_from(["numinous", "munch"]).expect("parse");
        let Some(super::Command::Munch { rounds, .. }) = cli.command else {
            panic!("munch command");
        };
        assert!(
            rounds as u64 > numinous_core::FULL_DECK_ROUND,
            "the default session must reach multiple full-deck boards"
        );
    }

    #[test]
    fn compact_life_poke_matches_a_phase_stamped_click() {
        use numinous_core::RoomInput;
        let point = (0.23, 0.71);
        let phase = 0.47;
        let compact = render_report(
            "game-of-life",
            64,
            48,
            phase,
            false,
            RoomRenderInput::new(0, &[point]),
        )
        .expect("compact poke");
        let event = [RoomInput::PointerDown {
            x: point.0,
            y: point.1,
            t: phase,
        }];
        let gesture = render_report(
            "game-of-life",
            64,
            48,
            phase,
            false,
            RoomRenderInput::with_gesture(0, &event),
        )
        .expect("gesture");
        assert_eq!(compact, gesture);
    }

    #[test]
    fn life_gesture_replays_a_causal_launch_then_evolution() {
        use numinous_core::RoomInput;
        let variation = 7;
        let final_phase = 0.5;
        let launch_phase = 0.1;
        let point = (0.23, 0.71);
        let event = [RoomInput::PointerDown {
            x: point.0,
            y: point.1,
            t: launch_phase,
        }];
        let report = render_report(
            "game-of-life",
            64,
            48,
            final_phase,
            false,
            RoomRenderInput::with_gesture(variation, &event),
        )
        .expect("causal Life replay");
        let repeated = render_report(
            "game-of-life",
            64,
            48,
            final_phase,
            false,
            RoomRenderInput::with_gesture(variation, &event),
        )
        .expect("repeated Life replay");
        let untouched = render_report(
            "game-of-life",
            64,
            48,
            final_phase,
            false,
            RoomRenderInput::new(variation, &[]),
        )
        .expect("untouched Life frame");
        let compact_now = render_report(
            "game-of-life",
            64,
            48,
            final_phase,
            false,
            RoomRenderInput::new(variation, &[point]),
        )
        .expect("same-phase compact poke");

        let mut session = numinous_core::rooms::game_of_life::LifeSession::new(variation);
        for _ in 0..14 {
            session.advance();
        }
        assert!(session.launch(point));
        for _ in 14..70 {
            session.advance();
        }
        let mut canvas = numinous_core::Canvas::new(64, 48);
        session.render(&mut canvas);
        let room = numinous_core::room_by_id("game-of-life").expect("Life room");
        let expected = format!(
            "{}Status: {}\nAction: {}\n",
            canvas.to_text(),
            session.status(),
            numinous_core::room_action(room.as_ref())
        );

        assert_eq!(report, expected);
        assert_eq!(report, repeated);
        assert_ne!(report, untouched);
        assert_ne!(report, compact_now);
        assert!(report.contains("Status: BORN"), "got: {report}");
        assert!(report.contains("GEN 70"), "got: {report}");
        assert!(report.contains("GLIDER 1"), "got: {report}");
    }

    #[test]
    fn render_exposes_one_explicit_replayable_variation_seed() {
        let cli = Cli::try_parse_from(["numinous", "render", "game-of-life", "--variation", "7"])
            .expect("explicit variation parses");
        assert!(matches!(
            cli.command,
            Some(Command::Render {
                variation: Some(7),
                vary: false,
                ..
            })
        ));
        assert!(
            Cli::try_parse_from([
                "numinous",
                "render",
                "game-of-life",
                "--variation",
                "7",
                "--vary",
            ])
            .is_err(),
            "clock variation and an explicit seed are mutually exclusive"
        );
    }

    #[test]
    fn invalid_render_pokes_do_not_record_progress() {
        let mut journey = numinous_core::Journey::default();
        let before = journey.clone();
        let code = run(
            Command::Render {
                id: "double-pendulum".to_string(),
                width: 30,
                height: 20,
                t: 0.0,
                out: None,
                color: false,
                era: "modern".to_string(),
                vary: false,
                variation: None,
                pokes: vec!["2,0.5".to_string()],
                gestures: Vec::new(),
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey, before);
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
        let message = super::render_png(
            "times-tables",
            64,
            48,
            0.0,
            &path,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .expect("render png");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "png should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn render_png_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_should_not_exist.png");
        assert!(
            super::render_png(
                "no-such-room",
                10,
                10,
                0.0,
                &path,
                false,
                numinous_core::Era::Modern,
                RoomRenderInput::plain(),
            )
            .is_err()
        );
    }

    #[test]
    fn loop_subcommand_parses_share_defaults() {
        let cli = Cli::try_parse_from([
            "numinous",
            "loop",
            "times-tables",
            "--out",
            "loop.png",
            "--poke",
            "0.4,0.5",
        ])
        .expect("loop parses");
        assert!(matches!(
            cli.command,
            Some(Command::Loop {
                size: 480,
                t: 0.0,
                variation: 0,
                ..
            })
        ));
    }

    #[test]
    fn render_loop_apng_writes_a_multi_frame_file() {
        let path = std::env::temp_dir().join("numinous_cli_loop_test.png");
        let _ = std::fs::remove_file(&path);
        let message = super::render_loop_apng(
            "times-tables",
            64,
            0.0,
            &path,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .expect("render loop");
        assert!(message.contains("wrote"));
        assert!(message.contains("24 frames"));
        let file = std::fs::File::open(&path).expect("open loop");
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let reader = decoder.read_info().expect("read loop header");
        let animation = reader
            .info()
            .animation_control
            .expect("CLI short loop is animated");
        assert_eq!(animation.num_frames, super::LOOP_FRAMES);
        assert_eq!(animation.num_plays, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sonify_wav_writes_a_non_empty_file() {
        let mut path = std::env::temp_dir();
        path.push("numinous_cli_sonify_test.wav");
        let message = super::sonify_wav("lissajous", 0.0, &path, false, RoomRenderInput::plain())
            .expect("sonify");
        assert!(message.contains("wrote"));
        let size = std::fs::metadata(&path).expect("file exists").len();
        assert!(size > 0, "wav should not be empty");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn room_bed_export_exactly_quantizes_the_shared_stereo_source() {
        let first = std::env::temp_dir().join("numinous_cli_room_bed.wav");
        let second = std::env::temp_dir().join("numinous_cli_room_bed_repeat.wav");
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);
        let input = RoomRenderInput::new(42, &[]);

        let report = super::sonify_wav_layer(
            "times-tables",
            0.0,
            &first,
            false,
            input,
            SonifyLayer::RoomBed,
        )
        .expect("room bed");
        super::sonify_wav_layer(
            "times-tables",
            0.0,
            &second,
            false,
            input,
            SonifyLayer::RoomBed,
        )
        .expect("repeat room bed");
        assert!(report.contains("room bed, 40.00s, 79 events, stereo 16000 Hz"));
        assert!(report.contains("stable pre-master bed only"));

        let bytes = std::fs::read(&first).expect("WAV bytes");
        assert_eq!(bytes, std::fs::read(&second).expect("repeat WAV"));
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let mut offset = 12usize;
        let mut format = None;
        let mut data = None;
        while offset + 8 <= bytes.len() {
            let id = &bytes[offset..offset + 4];
            let size =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let start = offset + 8;
            let end = start.checked_add(size).expect("bounded RIFF chunk");
            assert!(end <= bytes.len(), "RIFF chunk must fit the file");
            if id == b"fmt " {
                format = Some(&bytes[start..end]);
            } else if id == b"data" {
                data = Some(&bytes[start..end]);
            }
            offset = end + size % 2;
        }
        let format = format.expect("format chunk");
        assert!(format.len() >= 16);
        assert_eq!(u16::from_le_bytes(format[0..2].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(format[2..4].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(format[4..8].try_into().unwrap()),
            numinous_core::ROOM_BED_SOURCE_RATE
        );
        assert_eq!(
            u32::from_le_bytes(format[8..12].try_into().unwrap()),
            64_000
        );
        assert_eq!(u16::from_le_bytes(format[12..14].try_into().unwrap()), 4);
        assert_eq!(u16::from_le_bytes(format[14..16].try_into().unwrap()), 16);

        let room = numinous_core::all_rooms_with(42)
            .into_iter()
            .find(|room| room.meta().id == "times-tables")
            .expect("varied room");
        let arrangement = room.motif().expect("motif").arrangement();
        let expected = arrangement
            .render_stereo(numinous_core::ROOM_BED_SOURCE_RATE)
            .into_iter()
            .flat_map(|sample| numinous_core::quantize_pcm16(sample).to_le_bytes())
            .collect::<Vec<_>>();
        assert_eq!(data.expect("data chunk"), expected);

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn sonify_replays_times_tables_hand_input_into_the_sound() {
        let dir = std::env::temp_dir();
        let target = dir.join("numinous_cli_times_target.wav");
        let other = dir.join("numinous_cli_times_other.wav");
        let gesture_path = dir.join("numinous_cli_times_gesture.wav");
        let target_points = [(0.374, 0.5)];
        let other_points = [(0.75, 0.5)];
        let gesture = [numinous_core::RoomInput::PointerDown {
            x: 0.374,
            y: 0.5,
            t: 0.8,
        }];

        let report = super::sonify_wav(
            "times-tables",
            0.8,
            &target,
            false,
            RoomRenderInput::new(0, &target_points),
        )
        .expect("target sound");
        super::sonify_wav(
            "times-tables",
            0.8,
            &other,
            false,
            RoomRenderInput::new(0, &other_points),
        )
        .expect("other dial sound");
        super::sonify_wav(
            "times-tables",
            0.8,
            &gesture_path,
            false,
            RoomRenderInput::with_gesture(0, &gesture),
        )
        .expect("gesture sound");

        assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  FOUND"));
        let target_audio = std::fs::read(&target).expect("target WAV");
        assert_ne!(
            target_audio,
            std::fs::read(&other).expect("other WAV"),
            "different effective multipliers must produce different audio"
        );
        assert_eq!(
            target_audio,
            std::fs::read(&gesture_path).expect("gesture WAV"),
            "a compact poke and equivalent gesture must sonify identically"
        );
        let _ = std::fs::remove_file(target);
        let _ = std::fs::remove_file(other);
        let _ = std::fs::remove_file(gesture_path);
    }

    #[test]
    fn sonify_parses_replay_input_and_rejects_invalid_or_mixed_forms() {
        let cli = Cli::try_parse_from([
            "numinous",
            "sonify",
            "times-tables",
            "--out",
            "times.wav",
            "--poke",
            "0.374,0.5",
        ])
        .expect("sonify poke parses");
        assert!(matches!(
            cli.command,
            Some(Command::Sonify { ref pokes, ref gestures, .. })
                if pokes == &["0.374,0.5"] && gestures.is_empty()
        ));

        for (name, pokes, gestures) in [
            ("invalid", vec!["2,0.5".to_string()], Vec::new()),
            (
                "mixed",
                vec!["0.5,0.5".to_string()],
                vec!["down:0.5,0.5,0".to_string()],
            ),
        ] {
            let path = std::env::temp_dir().join(format!("numinous_cli_sonify_{name}.wav"));
            let _ = std::fs::remove_file(&path);
            let mut journey = numinous_core::Journey::default();
            let before = journey.clone();
            let code = run(
                Command::Sonify {
                    id: "times-tables".to_string(),
                    t: 0.0,
                    layer: SonifyLayer::Mathematical,
                    variation: 0,
                    out: path.clone(),
                    pokes,
                    gestures,
                },
                &mut journey,
            );
            assert_eq!(code, std::process::ExitCode::FAILURE);
            assert_eq!(journey, before, "invalid input must not record progress");
            assert!(!path.exists(), "invalid input must not write output");
        }
    }

    #[test]
    fn room_bed_rejects_controls_that_cannot_affect_it_before_progress_or_output() {
        for (name, t, pokes, gestures) in [
            ("phase", 0.5, Vec::new(), Vec::new()),
            ("poke", 0.0, vec!["0.5,0.5".to_string()], Vec::new()),
            (
                "gesture",
                0.0,
                Vec::new(),
                vec!["down:0.5,0.5,0".to_string()],
            ),
        ] {
            let path = std::env::temp_dir().join(format!("numinous_cli_bed_{name}.wav"));
            let _ = std::fs::remove_file(&path);
            let mut journey = numinous_core::Journey::default();
            let before = journey.clone();
            let code = run(
                Command::Sonify {
                    id: "times-tables".to_string(),
                    t,
                    layer: SonifyLayer::RoomBed,
                    variation: 7,
                    out: path.clone(),
                    pokes,
                    gestures,
                },
                &mut journey,
            );
            assert_eq!(code, std::process::ExitCode::FAILURE);
            assert_eq!(journey, before);
            assert!(!path.exists());
        }
    }

    #[test]
    fn wav_writer_rejects_invalid_channel_framing_before_creating_a_file() {
        let path = std::env::temp_dir().join("numinous_cli_invalid_channels.wav");
        let _ = std::fs::remove_file(&path);
        assert!(super::write_wav(&path, &[], 16_000, 0).is_err());
        assert!(super::write_wav(&path, &[0.0], 16_000, 2).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn sonify_unknown_room_is_error() {
        let path = std::env::temp_dir().join("numinous_cli_no.wav");
        assert!(
            super::sonify_wav("no-such-room", 0.0, &path, false, RoomRenderInput::plain(),)
                .is_err()
        );
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
    fn contact_sheet_survives_absurd_dimensions() {
        // `cols * tile` overflowed usize before the Raster clamp (a panic under
        // overflow-checks). A huge cols with a small tile hits that same multiply
        // while keeping the render cheap; the clamp must bound it.
        let path = std::env::temp_dir().join("numinous_contact_huge_test.png");
        let message = super::contact_sheet(&path, usize::MAX, 8).expect("bounded sheet");
        assert!(message.contains("contact sheet"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn play_frame_shows_the_room() {
        let room = numinous_core::room_by_id("times-tables").expect("room");
        let frame = super::play_frame(room.as_ref(), 0.0, 30, 15);
        assert!(frame.contains("Times Tables"));
        assert!(frame.contains(numinous_core::room_action(room.as_ref())));
        assert!(frame.contains('*'));
    }

    #[test]
    fn gesture_args_parse_and_reject_bad_events() {
        use numinous_core::RoomInput;
        assert_eq!(
            super::parse_gesture_arg("down:0.3,0.4,0.1"),
            Ok(RoomInput::PointerDown {
                x: 0.3,
                y: 0.4,
                t: 0.1
            })
        );
        assert_eq!(
            super::parse_gesture_arg("cancel"),
            Ok(RoomInput::PointerCancel)
        );
        assert!(super::parse_gesture_arg("wiggle:0.1,0.2,0.3").is_err());
        assert!(super::parse_gesture_arg("down:1.5,0.2,0.3").is_err());
        assert!(super::parse_gesture_arg("down:0.1,0.2").is_err());
        let too_many: Vec<String> = (0..=numinous_core::MAX_ROOM_INPUTS)
            .map(|_| "cancel".to_string())
            .collect();
        assert!(super::parse_gestures(&too_many).is_err());
    }

    #[test]
    fn a_gesture_render_matches_the_poke_bridge_for_legacy_rooms() {
        use numinous_core::RoomInput;
        let gesture = [
            RoomInput::PointerDown {
                x: 0.3,
                y: 0.7,
                t: 0.25,
            },
            RoomInput::PointerMove {
                x: 0.5,
                y: 0.5,
                t: 0.26,
            },
            RoomInput::PointerUp {
                x: 0.5,
                y: 0.5,
                t: 0.27,
            },
        ];
        let via_gesture = super::render_report(
            "voronoi",
            40,
            20,
            0.25,
            false,
            super::RoomRenderInput::with_gesture(0, &gesture),
        )
        .expect("gesture render succeeds");
        let via_pokes = super::render_report(
            "voronoi",
            40,
            20,
            0.25,
            false,
            super::RoomRenderInput::new(0, &[(0.3, 0.7), (0.5, 0.5)]),
        )
        .expect("poke render succeeds");
        assert_eq!(via_gesture, via_pokes, "the bridge answers identically");
    }

    #[test]
    fn a_gesture_pins_the_pendulum_in_the_terminal_too() {
        use numinous_core::RoomInput;
        let held = [RoomInput::PointerDown {
            x: 0.3,
            y: 0.4,
            t: 0.1,
        }];
        let early = super::render_report(
            "double-pendulum",
            50,
            30,
            0.2,
            false,
            super::RoomRenderInput::with_gesture(0, &held),
        )
        .expect("held render succeeds");
        let late = super::render_report(
            "double-pendulum",
            50,
            30,
            0.9,
            false,
            super::RoomRenderInput::with_gesture(0, &held),
        )
        .expect("held render succeeds");
        assert_eq!(early, late, "a pinned bob ignores the clock in the CLI too");
    }

    #[test]
    fn play_frames_always_lead_with_the_verb() {
        // Every catalog room answers the hand now; live play frames carry
        // the room's own verb, never the generic fallback.
        let room = numinous_core::room_by_id("slope-rider").expect("room");
        let frame = super::play_frame(room.as_ref(), 0.0, 30, 15);
        assert!(frame.contains(room.verb().expect("slope-rider has a verb")));
        assert!(!frame.contains(numinous_core::DEFAULT_ROOM_ACTION));
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
        assert!(
            super::render_png(
                "times-tables",
                8,
                8,
                0.0,
                bad,
                false,
                numinous_core::Era::Modern,
                RoomRenderInput::plain(),
            )
            .is_err()
        );
    }

    #[test]
    fn sonify_to_an_unwritable_path_is_error() {
        let bad = std::path::Path::new("no_such_dir_zzz/x.wav");
        assert!(
            super::sonify_wav("lissajous", 0.0, bad, false, RoomRenderInput::plain(),).is_err()
        );
    }

    #[test]
    fn the_hidden_room_answers_only_to_rank() {
        assert!(super::find_room("tetractys", false).is_none());
        assert!(super::find_room("tetractys", true).is_some());
        assert!(super::find_room_with_variation("tetractys", false, 42).is_none());
        assert!(
            super::find_room_with_variation("tetractys", true, 42).is_some(),
            "variation lookup must preserve hidden-room access after rank checks"
        );
        // Catalog rooms are open to everyone.
        assert!(super::find_room("lorenz", false).is_some());
        // The unready get the ordinary not-found, no special acknowledgment.
        let err = super::render_report("tetractys", 10, 10, 0.0, false, RoomRenderInput::plain())
            .unwrap_err();
        assert!(err.contains("Known rooms"), "an ordinary miss: {err}");
        assert!(!err.contains("Order"), "nothing is given away");
        // The ready see the figure.
        let ok = super::render_report("tetractys", 30, 20, 0.0, true, RoomRenderInput::plain())
            .expect("the figure");
        assert!(ok.contains('#'));
    }

    #[test]
    fn deep_cuts_unlock_with_level() {
        let low = super::describe_report(
            "mandelbrot",
            false,
            false,
            &numinous_core::Journey::default(),
        )
        .expect("describe");
        assert!(
            low.contains("LOCKED: a deeper cut opens at LV 5"),
            "got: {low}"
        );
        assert!(!low.contains("Shishikura"));
        let mid = super::describe_report(
            "mandelbrot",
            false,
            false,
            &numinous_core::Journey {
                plays: 10,
                ..Default::default()
            },
        )
        .expect("describe");
        assert!(mid.contains("Deeper:"));
        assert!(mid.contains("LOCKED: a deeper cut opens at LV 12"));
        let high = super::describe_report(
            "mandelbrot",
            false,
            false,
            &numinous_core::Journey {
                plays: 66,
                ..Default::default()
            },
        )
        .expect("describe");
        assert!(high.contains("Deeper still:") && high.contains("Shishikura"));

        let cap = super::describe_report(
            "cult-of-pi",
            false,
            false,
            &numinous_core::Journey {
                plays: u32::MAX,
                ..Default::default()
            },
        )
        .expect("describe");
        assert!(
            cap.contains("Feynman point"),
            "third cut is reachable: {cap}"
        );
        assert!(!cap.contains("4294967295"), "no sentinel leaks: {cap}");
    }

    #[test]
    fn a_boon_opens_a_cut_ahead_of_level() {
        let mut journey = numinous_core::Journey::default(); // level 1
        journey.chosen.insert("cut:mandelbrot:0".to_string());
        let text = super::describe_report("mandelbrot", false, false, &journey).expect("describe");
        assert!(text.contains("Deeper:"), "the chosen cut is open: {text}");
        assert!(
            text.contains("LOCKED: a deeper cut opens at LV 12"),
            "the second cut still waits: {text}"
        );
    }

    #[test]
    fn deep_whispers_require_standing() {
        assert!(
            super::describe_report("curtain", false, false, &numinous_core::Journey::default())
                .is_err()
        );
        let deep = super::describe_report(
            "curtain",
            false,
            true,
            &numinous_core::Journey {
                plays: 10,
                ..Default::default()
            },
        )
        .expect("a deeper whisper");
        assert!(deep.contains("veil"), "got: {deep}");
    }

    #[test]
    fn journey_report_shows_the_sky_and_the_rank() {
        let mut journey = numinous_core::Journey::default();
        let fresh = super::journey_report(&journey, &numinous_core::Scoreboard::default());
        assert!(fresh.contains("0 of"));
        assert!(fresh.contains("Outsider"));
        journey.visit("lorenz");
        let one = super::journey_report(&journey, &numinous_core::Scoreboard::default());
        assert!(one.contains("1 of"));
        assert!(one.contains("Akousmatikos"));
        assert!(one.contains('#'), "a lit star");
    }

    #[test]
    fn nim_board_draws_stones_per_heap() {
        let text = super::nim_board(&[3, 1, 0]);
        assert!(text.contains("1) O O O"));
        assert!(text.contains("2) O"));
        assert_eq!(text.lines().count(), 3);
    }

    #[test]
    fn resonances_appear_in_the_journey_when_lit() {
        let mut journey = numinous_core::Journey::default();
        journey.visit("mandelbrot");
        journey.visit("julia");
        let report = super::journey_report(&journey, &numinous_core::Scoreboard::default());
        assert!(report.contains("RESONANCE  The Atlas"));
        assert!(report.contains("atlas"), "the lore line rides along");
    }

    #[test]
    fn gauntlet_combo_multiplies_clears_and_forgives_misses() {
        // All four cleared: 10*1 + 25*2 + 25*3 + 40*4 = 295.
        assert_eq!(
            super::gauntlet_total(&[10, 25, 25, 40], &[true, true, true, true]),
            295
        );
        // A miss resets the combo: 10*1 + 0*2 + 25*1 + 40*2 = 115.
        assert_eq!(
            super::gauntlet_total(&[10, 0, 25, 40], &[true, false, true, true]),
            115
        );
        // Nothing cleared, nothing multiplied.
        assert_eq!(super::gauntlet_total(&[5, 0, 0, 0], &[false; 4]), 5);
        assert_eq!(super::gauntlet_total(&[], &[]), 0);
    }

    #[test]
    fn the_trophy_case_shines_and_silhouettes() {
        let mut journey = numinous_core::Journey::default();
        journey.visit("lorenz");
        let case = super::trophies_report(&journey, &numinous_core::Scoreboard::default());
        assert!(case.contains("TROPHIES  1 of"));
        assert!(case.contains("First Light"));
        assert!(case.contains("???"), "the silhouettes beckon");
        assert!(!case.contains("Cartographer"), "unearned names stay hidden");
    }

    #[test]
    fn scores_report_lists_best_first_or_invites_play() {
        let empty = super::scores_report(&numinous_core::Scoreboard::default());
        assert!(empty.contains("No scores yet"));
        let mut board = numinous_core::Scoreboard::default();
        board.record("munch seed:7 board:0", 80);
        board.record("quiz seed:1 rounds:5", 4);
        let table = super::scores_report(&board);
        assert!(table.contains("HIGH SCORES"));
        let munch_pos = table.find("munch").unwrap();
        let quiz_pos = table.find("quiz").unwrap();
        assert!(munch_pos < quiz_pos, "higher score listed first");
    }

    #[test]
    fn pick_seed_honors_the_explicit_seed() {
        let mut j = numinous_core::Journey::default();
        assert_eq!(super::pick_seed(7, false, &mut j), 7);
        // The daily seed is a day count: small, positive, stable within a run.
        let daily = super::pick_seed(7, true, &mut j);
        assert!(daily > 20_000 && daily < 40_000, "got {daily}");
        assert_eq!(super::pick_seed(7, true, &mut j), daily);
    }

    #[test]
    fn trophies_ping_the_moment_they_are_earned() {
        let empty = numinous_core::Scoreboard::default();
        let before_journey = numinous_core::Journey::default();
        let before = super::earned_names(&before_journey, &empty);
        assert!(before.is_empty());
        let mut after = numinous_core::Journey::default();
        after.visit("lorenz");
        after.wins = 1;
        let pings = super::trophy_pings(&before, &after, &empty);
        assert_eq!(pings.len(), 2, "first light and first blood: {pings:?}");
        assert!(pings.iter().any(|p| p.contains("First Light")));
        assert!(pings.iter().any(|p| p.contains("First Blood")));
        // Already-earned trophies never ping again.
        let now = super::earned_names(&after, &empty);
        assert!(super::trophy_pings(&now, &after, &empty).is_empty());
    }

    #[test]
    fn level_ups_announce_lore_and_unlocks() {
        let before = numinous_core::Journey::default();
        let after = numinous_core::Journey {
            plays: 3, // level 3, crossing the LV 3 unlock
            ..Default::default()
        };
        let banner = super::level_up_report(&before, &after).expect("a level was crossed");
        assert!(banner.contains("LEVEL UP"));
        assert!(banner.contains("LV  3"));
        assert!(banner.contains("odd prime"), "the level lore rides along");
        assert!(banner.contains("UNLOCKED") && banner.contains("quiz --hard"));
        assert!(super::level_up_report(&after, &after).is_none());
    }

    #[test]
    fn the_answer_carries_its_freight() {
        let text = super::answer_text();
        assert!(text.starts_with("42."));
        assert!(text.contains("no level 43"));
        assert!(text.contains("contribute"));
        assert!(text.contains("same rules"));
        assert!(text.contains("no cap"));
        assert!(text.contains("Do great things"));
        assert!(!text.contains("outside"), "math is not somewhere else");
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
        let out = super::render_color_report(
            "times-tables",
            20,
            20,
            0.0,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .expect("color render");
        assert!(out.contains("\x1b[38;2;"), "has truecolor escapes");
        assert!(
            super::render_color_report(
                "nope",
                20,
                20,
                0.0,
                false,
                numinous_core::Era::Modern,
                RoomRenderInput::plain(),
            )
            .is_err()
        );
    }

    #[test]
    fn eras_change_the_color_frame() {
        let modern = super::render_color_report(
            "chaos-game",
            20,
            20,
            0.0,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .expect("render");
        let phosphor = super::render_color_report(
            "chaos-game",
            20,
            20,
            0.0,
            false,
            numinous_core::Era::Phosphor,
            RoomRenderInput::plain(),
        )
        .expect("render");
        assert_ne!(modern, phosphor);
    }

    #[test]
    fn watch_frame_paints_in_place_with_a_status_line() {
        let room = numinous_core::room_by_id("chaos-game").expect("room");
        let frame = super::watch_frame(room.as_ref(), 0.5, 24, 16, numinous_core::Era::Modern);
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
    fn studio_plot_paths_reject_dimensions_above_the_cli_limit() {
        assert!(super::plot_report("x", -1.0, 1.0, 0.0, 4096, 2).is_ok());
        for (width, height) in [(4097, 2), (2, 4097)] {
            let error =
                super::plot_report("x", -1.0, 1.0, 0.0, width, height).expect_err("oversized plot");
            assert!(error.contains("CLI limit"), "unexpected error: {error}");
        }

        let creation = numinous_core::StudioCreation::new("x", -1.0, 1.0, 0.0).expect("creation");
        let error = super::open_studio_report(&creation.to_link(), 4097, 2)
            .expect_err("oversized opened plot");
        assert!(error.contains("CLI limit"), "unexpected error: {error}");
    }

    #[test]
    fn plot_save_writes_a_portable_studio_file() {
        let path = std::env::temp_dir().join("numinous_cli_studio_save_test.num");
        let _ = std::fs::remove_file(&path);
        let message = save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, &path).expect("studio save");
        assert!(message.contains("numinous://studio?"));
        let text = std::fs::read_to_string(&path).expect("saved file");
        let creation = numinous_core::StudioCreation::from_num_file(&text).expect("round trip");
        assert_eq!(creation.source(), "sin(a*x)");
        assert_eq!(creation.xmin(), -2.0);
        assert_eq!(creation.xmax(), 2.0);
        assert_eq!(creation.a(), 0.5);
        assert!(
            save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, &path).is_err(),
            "save should not overwrite an existing share file"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn plot_save_and_animate_is_rejected_before_progress() {
        let path = std::env::temp_dir().join("numinous_cli_studio_save_animate_test.num");
        let _ = std::fs::remove_file(&path);
        let mut journey = numinous_core::Journey::default();
        let code = run(
            Command::Plot {
                expr: "x".to_string(),
                xmin: -1.0,
                xmax: 1.0,
                a: 1.0,
                animate: true,
                amin: 0.0,
                amax: 1.0,
                width: 24,
                height: 8,
                save: Some(path.clone()),
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
        assert!(!path.exists());
    }

    #[test]
    fn invalid_animated_plot_is_rejected_before_progress() {
        let mut journey = numinous_core::Journey::default();
        let code = run(
            Command::Plot {
                expr: "x".to_string(),
                xmin: -1.0,
                xmax: 1.0,
                a: 1.0,
                animate: true,
                amin: 0.0,
                amax: 1.0,
                width: 4097,
                height: 8,
                save: None,
            },
            &mut journey,
        );

        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
    }

    #[test]
    fn plot_save_waits_for_a_valid_still_plot() {
        let path = std::env::temp_dir().join("numinous_cli_studio_bad_width_test.num");
        let _ = std::fs::remove_file(&path);
        let mut journey = numinous_core::Journey::default();
        let code = run(
            Command::Plot {
                expr: "x".to_string(),
                xmin: -1.0,
                xmax: 1.0,
                a: 1.0,
                animate: false,
                amin: 0.0,
                amax: 1.0,
                width: 1,
                height: 8,
                save: Some(path.clone()),
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
        assert!(!path.exists(), "failed plot must not leave a .num file");
    }

    #[test]
    fn plot_save_waits_for_finite_samples() {
        let path = std::env::temp_dir().join("numinous_cli_studio_undefined_test.num");
        let _ = std::fs::remove_file(&path);
        let mut journey = numinous_core::Journey::default();
        let code = run(
            Command::Plot {
                expr: "ln(-1)".to_string(),
                xmin: -2.0,
                xmax: -1.0,
                a: 1.0,
                animate: false,
                amin: 0.0,
                amax: 1.0,
                width: 24,
                height: 8,
                save: Some(path.clone()),
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
        assert!(!path.exists(), "undefined plot must not leave a .num file");
    }

    #[test]
    fn failed_plot_save_does_not_record_progress() {
        let path = std::env::temp_dir().join("numinous_cli_studio_existing_test.num");
        std::fs::write(&path, "already here").expect("seed existing file");
        let mut journey = numinous_core::Journey::default();
        let code = run(
            Command::Plot {
                expr: "x".to_string(),
                xmin: -1.0,
                xmax: 1.0,
                a: 1.0,
                animate: false,
                amin: 0.0,
                amax: 1.0,
                width: 24,
                height: 8,
                save: Some(path.clone()),
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
        assert_eq!(
            std::fs::read_to_string(&path).expect("existing file"),
            "already here",
            "save failure must not overwrite the existing file"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_studio_renders_saved_file_and_link() {
        let path = std::env::temp_dir().join("numinous_cli_studio_open_test.num");
        let _ = std::fs::remove_file(&path);
        save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, &path).expect("studio save");

        let from_file =
            open_studio_report(path.to_str().expect("utf8 path"), 32, 10).expect("open saved file");
        assert!(from_file.contains("Studio creation"));
        assert!(from_file.contains("expr=sin(a*x)"));
        assert!(from_file.contains("link=numinous://studio?"));
        assert!(from_file.contains('#'));

        let creation = numinous_core::StudioCreation::from_num_file(
            &std::fs::read_to_string(&path).expect("saved file"),
        )
        .expect("creation");
        let from_link = open_studio_report(&creation.to_link(), 32, 10).expect("open link");
        assert!(from_link.contains("expr=sin(a*x)"));
        assert!(from_link.contains('#'));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_studio_subcommand_parses_and_records_success() {
        let path = std::env::temp_dir().join("numinous_cli_studio_run_open_test.num");
        let _ = std::fs::remove_file(&path);
        save_studio_creation("x", -1.0, 1.0, 0.0, &path).expect("studio save");

        let cli = Cli::try_parse_from([
            "numinous",
            "open-studio",
            path.to_str().expect("utf8 path"),
            "--width",
            "24",
            "--height",
            "8",
        ])
        .expect("parse open-studio");
        let Some(command) = cli.command else {
            panic!("command parsed");
        };
        let mut journey = numinous_core::Journey::default();
        let code = run(command, &mut journey);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        assert_eq!(journey.plays, 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn failed_open_studio_does_not_record_progress() {
        let mut journey = numinous_core::Journey::default();
        let missing = std::env::temp_dir().join("numinous_cli_studio_missing_test.num");
        let _ = std::fs::remove_file(&missing);
        let code = run(
            Command::OpenStudio {
                input: missing.to_string_lossy().to_string(),
                width: 32,
                height: 10,
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey.plays, 0);
    }

    #[test]
    fn open_studio_rejects_malformed_and_oversized_imports() {
        let bad = std::env::temp_dir().join("numinous_cli_studio_bad_test.num");
        let huge = std::env::temp_dir().join("numinous_cli_studio_huge_test.num");
        std::fs::write(
            &bad,
            "NUMINOUS_STUDIO 1\nexpr=x\u{1b}[31m\nxmin=-1\nxmax=1\na=1\n",
        )
        .expect("bad file");
        std::fs::write(
            &huge,
            "x".repeat(super::MAX_STUDIO_IMPORT_BYTES as usize + 1),
        )
        .expect("huge file");

        let bad_err = load_studio_creation(bad.to_str().expect("utf8 path"))
            .expect_err("bad import rejected");
        assert_eq!(bad_err, "invalid Numinous Studio .num file\n");
        let huge_err = load_studio_creation(huge.to_str().expect("utf8 path"))
            .expect_err("huge import rejected");
        assert!(huge_err.contains("too large"));
        let link_err = load_studio_creation("numinous://studio?expr=x&xmin=-1&xmax=1&a=%")
            .expect_err("bad link rejected");
        assert_eq!(link_err, "invalid Numinous Studio link\n");

        let _ = std::fs::remove_file(&bad);
        let _ = std::fs::remove_file(&huge);
    }

    #[test]
    fn tune_wav_writes_a_chiptune() {
        let path = std::env::temp_dir().join("numinous_tune_test.wav");
        let message = super::tune_wav(7, 2, &path).expect("tune");
        assert!(message.contains("chip"));
        assert!(std::fs::metadata(&path).expect("file").len() > 1000);
        let _ = std::fs::remove_file(&path);
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
        let out =
            super::describe_report("hippasus", false, false, &numinous_core::Journey::default())
                .expect("a whisper");
        assert!(out.to_lowercase().contains("sea"), "got: {out}");
        assert!(
            super::describe_report(
                "not-a-room-nor-secret",
                false,
                false,
                &numinous_core::Journey::default()
            )
            .is_err()
        );
    }

    #[test]
    fn sim_run_rejects_bad_input() {
        assert!(super::sim_run("nope", &[], 10, 10).is_err());
        assert!(super::sim_run("wing", &["nope=1".to_string()], 10, 10).is_err());
        assert!(super::sim_run("wing", &["angle-of-attack=abc".to_string()], 10, 10).is_err());
        assert!(super::sim_run("wing", &["missing-equals".to_string()], 10, 10).is_err());
    }
}
