//! The `numinous` command line: the terminal face of the headless core.
//!
//! See `docs/INTERFACES.md`. The CLI exposes the shared catalog, rooms, games,
//! progression, Studio, audio, rendering, export, and digital-mind play paths
//! without owning their domain logic.
//!
//! The command handlers are split into pure `*_report` functions that return the
//! text to emit, so they can be unit-tested without capturing stdout; `main`
//! stays a thin shell that prints and sets the exit code.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufWriter, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use numinous_core::{
    CUT_LEVELS, Canvas, Journey, PlotRequest, Raster, Room, RoomMeta, SingRequest, Surface,
    all_rooms, draw_text, hidden_room_by_id, room_by_id,
};

mod access;
mod game_input;
mod game_runtime;
mod local_state;
mod render_input;
mod studio;

#[cfg(test)]
use access::color_allowed_for;
use access::{access_report, access_settings, color_allowed};
#[cfg(test)]
use game_input::MAX_CLI_INPUT_BYTES;
use game_input::{BoundedInputLine, read_bounded_input_line};
use game_runtime::{
    aliens, arcade, bench, crack, fifteen, gauntlet, hackenbush, munch, nim, party, pick_seed,
    quiz, seti,
};
#[cfg(test)]
use game_runtime::{
    aliens_with_input, arcade_text, arcade_with_input, bench_with_input, crack_with_input,
    fifteen_with_input, garden_text, gauntlet_with_input, hackenbush_with_input, munch_with_input,
    nim_board, nim_with_input, painted, party_board_text, party_with_input, pick_seed_for_day,
    quiz_remark, quiz_with_input, seti_with_input,
};
use local_state::forget_local_state;
use render_input::{RoomRenderInput, parse_room_inputs, validate_render_request, visible_status};
#[cfg(test)]
use render_input::{parse_gesture_arg, parse_gestures, parse_poke_arg, parse_pokes};
#[cfg(test)]
use studio::load_studio_creation;
use studio::{
    CreationIdentity, ForkEdits, StudioParameters, fork_studio_creation_extended,
    open_studio_report, parametric_report, plot_report, plot_request_error, resolve_plot_source,
    resolve_sing_input, save_parametric_creation, save_studio_creation_with_scale,
    sing_request_error,
};
#[cfg(test)]
use studio::{fork_studio_creation, save_studio_creation};

const MAX_ENV_FILE_BYTES: u64 = 16 * 1024;
const ELEVENLABS_MUSIC_URL: &str = "https://api.elevenlabs.io/v1/music?output_format=pcm_44100";
#[cfg(windows)]
const UPDATE_INSTALLER: &str = include_str!("../../../scripts/install.ps1");
#[cfg(not(windows))]
const UPDATE_INSTALLER: &str = include_str!("../../../scripts/install.sh");
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
    /// Show the accessibility switches and which of them are on right now.
    Access,
    /// Install the latest verified GitHub release without touching play history.
    Update,
    /// Remove the managed installation without touching play history.
    Uninstall,
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
    /// Open a room's explanation after playing it.
    Reveal {
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
        /// Add a normalized hand point, as x,y in `[0, 1]`.
        /// Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first; phase time wraps from 1 to 0. Held rooms pin,
        /// pull, and fling. In Life, a down earlier than --t shows the glider's
        /// later evolution; its newest 24 down events become launches. Not
        /// combinable with --poke.
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
        /// Add a normalized hand point, as x,y in `[0, 1]`.
        /// Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first; phase time wraps from 1 to 0. Not combinable
        /// with --poke.
        #[arg(long = "gesture")]
        gestures: Vec<String>,
    },
    /// Package a postcard, short loop, and README into one share folder.
    Share {
        /// Room id, e.g. "times-tables".
        id: String,
        /// Parent directory for the share folder (created if missing).
        #[arg(long)]
        out: PathBuf,
        /// Visual era: phosphor, 8bit, vector, or modern.
        #[arg(long, default_value = "modern")]
        era: String,
        /// Use this exact room variation seed (default 0).
        #[arg(long, default_value_t = 0)]
        variation: u64,
        /// Starting phase in [0, 1) for still and loop.
        #[arg(long, default_value_t = 0.0)]
        t: f64,
        /// Still/loop edge in pixels (square). Default matches App postcard scale.
        #[arg(long, default_value_t = 480)]
        size: usize,
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
        /// Add a normalized hand point, as x,y in `[0, 1]`.
        /// Repeat for multiple points.
        #[arg(long = "poke")]
        pokes: Vec<String>,
        /// Add a gesture event: down:x,y,t, move:x,y,t, up:x,y,t, or cancel.
        /// Repeat, oldest first; phase time wraps from 1 to 0. Not combinable
        /// with --poke.
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
    /// Inventory Numinous-managed local state; erase selected stores with --confirm.
    Forget {
        /// Actually erase the journey (without this, just show what is kept).
        #[arg(long)]
        confirm: bool,
        /// Also erase the score table.
        #[arg(long)]
        scores: bool,
        /// Also erase player-owned local Cairn drafts.
        #[arg(long)]
        cairn: bool,
        /// Also erase the opt-in experience journal.
        #[arg(long)]
        journal: bool,
        /// Also erase generated radio tracks in the managed cache.
        #[arg(long)]
        radio_cache: bool,
        /// Also erase the managed App crash diagnostic.
        #[arg(long)]
        crash_log: bool,
        /// Erase every inventoried managed store, including App preferences.
        #[arg(long)]
        all_local: bool,
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
    /// Discovery: pass an expression, or --recipe N, or --seed N (curated bank).
    Plot {
        /// Manual expression in x and a. Unary: sin cos tan exp ln abs sqrt floor.
        /// Pair functions: mod min max. Constants: pi e.
        /// Omit when using --x-expr/--y-expr, --recipe, --seed, or --list-recipes.
        expr: Option<String>,
        /// Parametric x(t) expression. Requires --y-expr and excludes graph discovery.
        #[arg(long)]
        x_expr: Option<String>,
        /// Parametric y(t) expression. Requires --x-expr and excludes graph discovery.
        #[arg(long)]
        y_expr: Option<String>,
        /// Curated Formula Jam recipe index (wraps). Mutually exclusive with expr and --seed.
        #[arg(long)]
        recipe: Option<u64>,
        /// Random discovery seed into the curated bank. Mutually exclusive with expr and --recipe.
        #[arg(long)]
        seed: Option<u64>,
        /// With --seed: bank entry at seed+step (stateless Auto walk).
        #[arg(long, default_value_t = 0)]
        auto_step: u64,
        /// List curated recipes and exit without plotting.
        #[arg(long)]
        list_recipes: bool,
        /// Left edge of the x range.
        #[arg(long)]
        xmin: Option<f64>,
        /// Right edge of the x range.
        #[arg(long)]
        xmax: Option<f64>,
        /// Left edge of parametric time (default -tau).
        #[arg(long)]
        tmin: Option<f64>,
        /// Right edge of parametric time (default tau).
        #[arg(long)]
        tmax: Option<f64>,
        /// Value of the parameter a (constant unless animating).
        #[arg(long, default_value_t = numinous_core::DEFAULT_STUDIO_PARAMETER)]
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
        #[arg(long, default_value_t = numinous_core::DEFAULT_PLOT_WIDTH)]
        width: usize,
        /// Plot height in rows.
        #[arg(long, default_value_t = numinous_core::DEFAULT_PLOT_HEIGHT)]
        height: usize,
        /// Save this Studio expression as a portable .num file and print its link.
        #[arg(long)]
        save: Option<PathBuf>,
        /// With --save: name the creation (printable ASCII, 64 characters).
        #[arg(long)]
        title: Option<String>,
        /// With --save: sign the creation (printable ASCII, 64 characters).
        #[arg(long)]
        author: Option<String>,
        /// With --save: prose credit (printable ASCII, 160 characters). Empty or whitespace clears it.
        #[arg(long)]
        credit: Option<String>,
        /// Portable pitch map stored with a saved creation.
        #[arg(long, value_enum, default_value_t)]
        scale: StudioScaleArg,
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
        /// Confirm the paid API spend. Without this flag the command names
        /// the key source and the cost, spends nothing, and stops.
        #[arg(long)]
        yes: bool,
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
    /// Call a room's readout: name the number before you look, then meet it.
    Call {
        /// Room id, e.g. "lorenz".
        id: String,
        /// Your call. Omit to hear the question first, then answer it.
        #[arg(long)]
        guess: Option<f64>,
        /// Which question. Defaults to today's, so a day has one call.
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Sing a function: turn y = f(x) into a melody and write WAV or MIDI.
    Sing {
        /// The same Studio expression grammar, a .num file path, or a numinous:// link.
        expr: String,
        /// Left edge of the x range (default -tau; a Studio input supplies its own).
        #[arg(long)]
        xmin: Option<f64>,
        /// Right edge of the x range (default tau; a Studio input supplies its own).
        #[arg(long)]
        xmax: Option<f64>,
        /// Number of notes.
        #[arg(long, default_value_t = numinous_core::DEFAULT_MELODY_NOTES)]
        notes: usize,
        /// Value of the parameter a, matching what `plot` uses (default 1; a
        /// Studio input supplies its own).
        #[arg(long)]
        a: Option<f64>,
        /// Override the capsule pitch map, or quantize a raw expression.
        #[arg(long, value_enum)]
        scale: Option<StudioScaleArg>,
        /// Write a WAV (.wav) or Standard MIDI File (.mid) here.
        #[arg(long)]
        out: PathBuf,
    },
    /// Remix a Studio creation: copy it, record its lineage, save the child.
    Fork {
        /// The parent: a Studio .num file path or a numinous:// link.
        parent: String,
        /// Write the forked .num here (never replaces an existing file).
        #[arg(long)]
        out: PathBuf,
        /// Replace the expression in the fork (the remix itself).
        #[arg(long)]
        expr: Option<String>,
        /// Replace the parametric x(t) coordinate. Requires --y-expr.
        #[arg(long)]
        x_expr: Option<String>,
        /// Replace the parametric y(t) coordinate. Requires --x-expr.
        #[arg(long)]
        y_expr: Option<String>,
        /// Replace the parent's stored pitch map.
        #[arg(long, value_enum)]
        scale: Option<StudioScaleArg>,
        /// Title for the fork.
        #[arg(long)]
        title: Option<String>,
        /// Author signature for the fork.
        #[arg(long)]
        author: Option<String>,
        /// Prose credit for the fork. Omit to keep the parent's identity suggestion; empty or whitespace clears it.
        #[arg(long)]
        credit: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum StudioScaleArg {
    #[default]
    Continuous,
    Chromatic,
    Major,
    Minor,
    Pentatonic,
}

impl From<StudioScaleArg> for numinous_core::StudioScale {
    fn from(value: StudioScaleArg) -> Self {
        match value {
            StudioScaleArg::Continuous => Self::Continuous,
            StudioScaleArg::Chromatic => Self::Chromatic,
            StudioScaleArg::Major => Self::Major,
            StudioScaleArg::Minor => Self::Minor,
            StudioScaleArg::Pentatonic => Self::Pentatonic,
        }
    }
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
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return report_cli_parse_error(error),
    };
    let (mut journey, readable) = load_journey();
    let before = journey.clone();
    let earned_before = earned_names(&before, &load_scores());
    let code = match cli.command {
        Some(command) => run(command, &mut journey),
        None => home(&journey),
    };
    finish_journey(&before, &journey, &earned_before, readable);
    code
}

/// A stranger's most natural move is typing the thing they want to see.
/// When the unknown token names a room, or lands near one, answer in the
/// house voice with the bridge to it instead of a stock parser error.
fn room_bridge_message(token: &str) -> Option<String> {
    let id = numinous_core::echoable_id(token);
    if numinous_core::room_by_id(token).is_some() {
        return Some(format!(
            "{id} is a room, not a command. Step inside: numinous watch {id}\n\
             The story: numinous describe {id}\n"
        ));
    }
    let suggestions = numinous_core::nearest_room_ids(token, numinous_core::MAX_ROOM_SUGGESTIONS);
    let first = suggestions.first()?;
    Some(format!(
        "No command or room named '{id}'. Near rooms: {}. Step inside: numinous watch {first}\n",
        suggestions.join(", "),
    ))
}

/// Route an unknown subcommand that names a room to the bridge; leave every
/// other parse outcome (help, version, command typos with clap's own
/// did-you-mean) exactly as clap prints it, stream and exit code included.
fn report_cli_parse_error(error: clap::Error) -> ExitCode {
    // An exact room id outranks any guess: clap offering `render` for
    // `mandelbrot` is a worse answer than the bridge, and letting the guess
    // win silenced the bridge for scores of real rooms. Only a near miss
    // defers, because there clap's own did-you-mean may be the better one.
    let token = match error.get(clap::error::ContextKind::InvalidSubcommand) {
        Some(clap::error::ContextValue::String(token)) => Some(token.clone()),
        _ => None,
    };
    let names_a_room = token
        .as_deref()
        .is_some_and(|token| numinous_core::room_by_id(token).is_some());
    let clap_suggests_a_command = error
        .get(clap::error::ContextKind::SuggestedSubcommand)
        .is_some();
    if error.kind() == clap::error::ErrorKind::InvalidSubcommand
        && (names_a_room || !clap_suggests_a_command)
        && let Some(token) = token.as_deref()
        && let Some(message) = room_bridge_message(token)
    {
        report_diagnostic(&message);
        // clap's usage-error code, so scripts see the same contract.
        return ExitCode::from(2);
    }
    error.exit()
}

fn managed_install_root(executable: &Path) -> Result<PathBuf, String> {
    let expected_name = if cfg!(windows) {
        "numinous.exe"
    } else {
        "numinous"
    };
    if executable.file_name() != Some(std::ffi::OsStr::new(expected_name)) {
        return Err("The running command is not the managed Numinous CLI.".to_string());
    }
    let Some(binary_dir) = executable.parent() else {
        return Err("The running command has no installation directory.".to_string());
    };
    if binary_dir.file_name() != Some(std::ffi::OsStr::new("bin")) {
        return Err("The running command is outside a managed Numinous bin directory.".to_string());
    }
    let Some(root) = binary_dir.parent() else {
        return Err("The running command has no managed installation root.".to_string());
    };
    let marker = root.join(".numinous-install-root");
    let metadata = marker
        .symlink_metadata()
        .map_err(|_| "The installation marker is missing.".to_string())?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096
    {
        return Err("The installation marker is not a bounded ordinary file.".to_string());
    }
    let marker_text = std::fs::read_to_string(&marker)
        .map_err(|_| "The installation marker could not be read.".to_string())?;
    if marker_text.lines().next() != Some("Numinous install root v2") {
        return Err("The installation marker is not current.".to_string());
    }
    Ok(root.to_path_buf())
}

fn write_update_installer() -> Result<PathBuf, String> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce)
        .map_err(|error| format!("Could not choose a private updater name: {error}"))?;
    let id = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let extension = if cfg!(windows) { "ps1" } else { "sh" };
    let path = std::env::temp_dir().join(format!("numinous-update-{id}.{extension}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("Could not stage the update helper: {error}"))?;
    if let Err(error) = file
        .write_all(UPDATE_INSTALLER.as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = std::fs::remove_file(&path);
        return Err(format!("Could not write the update helper: {error}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o700);
        if let Err(error) = std::fs::set_permissions(&path, permissions) {
            let _ = std::fs::remove_file(&path);
            return Err(format!("Could not protect the update helper: {error}"));
        }
    }
    Ok(path)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaintenanceAction {
    Update,
    Uninstall,
}

fn maintenance_process(installer: &Path, pid: &str, action: MaintenanceAction) -> ProcessCommand {
    #[cfg(windows)]
    let mut command = {
        let windows = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let mut command =
            ProcessCommand::new(windows.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
        command
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(installer)
            .arg("-NoModifyPath")
            .arg("-WaitForProcessId")
            .arg(pid)
            .arg("-DeleteInstaller")
            .arg(installer);
        if action == MaintenanceAction::Uninstall {
            command.arg("-Uninstall");
        }
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = ProcessCommand::new("/bin/sh");
        command
            .arg(installer)
            .arg("--no-modify-path")
            .arg("--wait-for-pid")
            .arg(pid)
            .arg("--delete-installer")
            .arg(installer);
        if action == MaintenanceAction::Uninstall {
            command.arg("--uninstall");
        }
        command
    };
    if let Some(parent) = installer
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        command.current_dir(parent);
    }
    command
}

fn maintain_installation(action: MaintenanceAction) -> ExitCode {
    let result = (|| {
        let executable = std::env::current_exe()
            .map_err(|error| format!("Could not locate the running command: {error}"))?;
        let root = managed_install_root(&executable)?;
        let installer = write_update_installer()?;
        let pid = std::process::id().to_string();
        let mut process = maintenance_process(&installer, &pid, action);
        process.env("NUMINOUS_HOME", &root).stdin(Stdio::null());
        // A helper inherits anonymous pipes too. It waits for this process, so
        // keeping a redirected handle open would make a capturing parent wait
        // for the helper in turn.
        if std::io::stdout().is_terminal() {
            process.stdout(Stdio::inherit());
        } else {
            process.stdout(Stdio::null());
        }
        if std::io::stderr().is_terminal() {
            process.stderr(Stdio::inherit());
        } else {
            process.stderr(Stdio::null());
        }
        if let Err(error) = process.spawn() {
            let _ = std::fs::remove_file(&installer);
            return Err(format!("Could not start the maintenance helper: {error}"));
        }
        Ok(root)
    })();
    match result {
        Ok(root) => {
            let verb = match action {
                MaintenanceAction::Update => "Updating",
                MaintenanceAction::Uninstall => "Uninstalling",
            };
            println!(
                "{verb} the managed installation at {}. The helper will continue after this command closes.",
                terminal_safe_path(&root),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "Install a managed release first with the one-line installer in https://github.com/blisspixel/numinous/blob/main/PLAY.md"
            );
            ExitCode::FAILURE
        }
    }
}

/// The front door: what `numinous`, alone, opens onto. Today's room in full
/// color, who you are, and the handful of verbs that matter.
fn home(journey: &Journey) -> ExitCode {
    print!(
        "{}",
        home_report(journey, std::io::stdout().is_terminal(), color_allowed())
    );
    ExitCode::SUCCESS
}

fn home_report(journey: &Journey, stdout_is_terminal: bool, color: bool) -> String {
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
            // No reset after the frame: a colored one already ends every line
            // with one, and a mono one must stay free of escapes.
            "{}{}  ({})\n",
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
        numinous_core::to_terminal(&raster, color),
        room.meta().title,
        room.meta().wing,
        numinous_core::insight(day + u64::from(journey.plays)),
        journey.level(),
        journey.level_bar(16),
        match journey.live_streak(day) {
            // A dead chain is not shown as alive; the ambient home stays
            // quiet about records and lets the journey report hold them.
            Some(chain) if chain > 1 => format!("   streak {chain}"),
            _ => String::new(),
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
    local_state_paths().journey
}

/// Load the journey, or start a fresh one.
///
/// A file that exists and cannot be read is not a fresh player. Treating it
/// as one silently demotes a rank, closes the veil, hides earned trophies,
/// and re-announces the same level every run, so this says what happened
/// once, on stderr, and returns a default marked unreadable. Nothing is at
/// risk of being written over it: the delta writer fails against the same
/// condition rather than replacing a file it could not read.
fn load_journey() -> (Journey, bool) {
    let path = journey_path();
    match numinous_core::read_journey_file(&path) {
        Ok(journey) => (journey, true),
        Err(error) => {
            let where_it_lives = terminal_safe_path(&path);
            let explanation = if path.is_dir() {
                format!("NUMINOUS_JOURNEY must name a file, but {where_it_lives} is a directory")
            } else {
                format!(
                    "your progress file could not be read, so this run cannot see your journey: {error}"
                )
            };
            report_diagnostic(&terminal_safe(&explanation));
            report_diagnostic(&format!(
                "nothing will be written over it. Fix or move {where_it_lives}, then play on."
            ));
            (Journey::default(), false)
        }
    }
}

/// Where the high-score table lives: `NUMINOUS_SCORES` if set, else home.
fn scores_path() -> PathBuf {
    local_state_paths().scores
}

fn local_state_paths() -> numinous_core::LocalStatePaths {
    #[cfg(test)]
    {
        numinous_core::LocalStatePaths {
            journey: test_state_path("journey"),
            scores: test_state_path("scores"),
            cairn: test_state_path("cairn"),
            journal: test_state_path("journal"),
            preferences: test_state_path("preferences"),
            radio_cache: test_state_path("radio"),
            protected_radio_source: None,
            crash_log: test_state_path("crash"),
        }
    }
    #[cfg(not(test))]
    {
        numinous_core::resolve_local_state_paths()
    }
}

/// Load the high-score table, or start a fresh one.
fn load_scores() -> numinous_core::Scoreboard {
    numinous_core::load_scoreboard_file(&scores_path())
}

/// Warn on stderr that a local write failed, naming what was lost: journey
/// progress or a score. The play still happened and the delta keeps riding
/// in memory, but silence here would let output celebrate what the disk
/// never received.
fn warn_progress_unsaved(what: &str, error: &std::io::Error) {
    eprintln!(
        "{}",
        terminal_safe(&format!("{what} could not be saved: {error}"))
    );
}

/// Persist a journey delta and say so on stderr when the ledger refuses.
/// The one shared door for the commands that persist and keep going.
fn persist_progress_or_warn(before: &Journey, journey: &Journey) {
    if let Err(error) = numinous_core::persist_journey_delta(&journey_path(), before, journey) {
        warn_progress_unsaved("progress", &error);
    }
}

/// Record a score; announce when a record falls, and say so when the write
/// fails rather than wearing the "not a new best" costume.
fn post_score(key: &str, score: i64) {
    match numinous_core::record_score_file(&scores_path(), key, score) {
        // Zero is never a best worth announcing: an aborted run's nothing
        // must not wear a celebration.
        Ok(true) if score > 0 => println!("NEW BEST: {key} = {score}"),
        Ok(true) => {}
        Ok(false) => {}
        Err(error) => warn_progress_unsaved("score", &error),
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
    readable: bool,
) {
    if before == after {
        return;
    }
    // A run that could not read the ledger has already said so, once, at the
    // door. Writing now could only fail against the same condition and would
    // repeat that news in different words, so this stops here: one cause,
    // one telling.
    if !readable {
        return;
    }
    // The play still happened; if the ledger refuses, say so and stop. The
    // banners below would otherwise celebrate a level the disk never
    // received, and because nothing was written they would celebrate the
    // same one again on the next run, and the next: a trophy that arrives
    // every time is not a trophy, it is noise wearing one.
    let saved = match numinous_core::persist_journey_delta(&journey_path(), before, after) {
        Ok(saved) => saved,
        Err(error) => {
            warn_progress_unsaved("progress", &error);
            report_diagnostic("what you earned this run was not recorded, so it is not announced.");
            return;
        }
    };

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
    numinous_core::room_by_id_with(id, variation).or_else(|| find_room(id, allow_hidden))
}

/// Run one command, recording the journey as it goes.
fn run(command: Command, journey: &mut Journey) -> ExitCode {
    let allow_hidden = numinous_core::behind_the_veil(journey);
    match command {
        Command::Access => {
            print!(
                "{}",
                access_report(&access_settings(
                    std::env::var_os(numinous_core::REDUCED_MOTION_VAR).as_deref(),
                    std::env::var_os(numinous_audio::MONO_AUDIO_VAR).as_deref(),
                    std::env::var_os("NO_COLOR").as_deref(),
                ))
            );
            ExitCode::SUCCESS
        }
        Command::Update => maintain_installation(MaintenanceAction::Update),
        Command::Uninstall => maintain_installation(MaintenanceAction::Uninstall),
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
        Command::Reveal { id, json } => emit(reveal_report(&id, json, allow_hidden, journey)),
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
                report_diagnostic(&message);
                return ExitCode::FAILURE;
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!(
                    "Unknown era '{}'. Eras: phosphor, 8bit, vector, modern.",
                    terminal_safe(&era)
                );
                return ExitCode::FAILURE;
            };
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    report_diagnostic(&message);
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
                None if color => render_color_report(
                    &id,
                    width,
                    height,
                    t,
                    allow_hidden,
                    TerminalStyle {
                        era,
                        color: color_allowed(),
                    },
                    input,
                ),
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
                report_diagnostic(&message);
                return ExitCode::FAILURE;
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!(
                    "Unknown era '{}'. Eras: phosphor, 8bit, vector, modern.",
                    terminal_safe(&era)
                );
                return ExitCode::FAILURE;
            };
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    report_diagnostic(&message);
                    return ExitCode::FAILURE;
                }
            };
            let input = if gesture.is_empty() {
                RoomRenderInput::new(variation, &pokes)
            } else {
                RoomRenderInput::with_gesture(variation, &gesture)
            };
            let report = render_loop_apng(&id, size, t, &out, allow_hidden, era, input, false);
            if report.is_ok() && find_room(&id, allow_hidden).is_some() {
                journey.visit(&id);
            }
            emit(report)
        }
        Command::Share {
            id,
            out,
            era,
            variation,
            t,
            size,
        } => {
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!(
                    "Unknown era '{}'. Eras: phosphor, 8bit, vector, modern.",
                    terminal_safe(&era)
                );
                return ExitCode::FAILURE;
            };
            let report = render_share_bundle(&id, &out, size, t, allow_hidden, era, variation);
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
                eprintln!(
                    "Unknown era '{}'. Eras: phosphor, 8bit, vector, modern.",
                    terminal_safe(&era)
                );
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
                // The loop never returns; persist the visit before it starts,
                // and say so if the ledger refuses, since no exit will.
                persist_progress_or_warn(&before, journey);
            }
            let Some(era) = numinous_core::Era::parse(&era) else {
                eprintln!(
                    "Unknown era '{}'. Eras: phosphor, 8bit, vector, modern.",
                    terminal_safe(&era)
                );
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
                report_diagnostic(&message);
                return ExitCode::FAILURE;
            }
            let (pokes, gesture) = match parse_room_inputs(&pokes, &gestures) {
                Ok(input) => input,
                Err(message) => {
                    report_diagnostic(&message);
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
                        persist_progress_or_warn(&before, journey);
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
            print!("{}", journey_report(journey, &load_scores(), pick_day()));
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
        Command::Forget {
            confirm,
            scores,
            cairn,
            journal,
            radio_cache,
            crash_log,
            all_local,
        } => {
            let selection = if all_local {
                numinous_core::LocalStateEraseSelection::complete()
            } else {
                numinous_core::LocalStateEraseSelection {
                    journey: true,
                    scores,
                    cairn,
                    journal,
                    preferences: false,
                    radio_cache,
                    crash_log,
                }
            };
            match forget_local_state(&local_state_paths(), confirm, selection, all_local) {
                Ok(report) => {
                    println!("{report}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Crack {
            seed,
            daily,
            digits,
            attempts,
        } => {
            if !numinous_core::supports_code_length(digits) {
                eprintln!(
                    "Codes run {} to {} digits.",
                    numinous_core::MIN_CODE_DIGITS,
                    numinous_core::MAX_CODE_DIGITS
                );
                return ExitCode::FAILURE;
            }
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
            x_expr,
            y_expr,
            recipe,
            seed,
            auto_step,
            list_recipes,
            xmin,
            xmax,
            tmin,
            tmax,
            a,
            animate,
            amin,
            amax,
            width,
            height,
            save,
            title,
            author,
            credit,
            scale,
        } => {
            if list_recipes {
                let mut lines = vec![format!(
                    "Formula Jam curated recipes ({}):",
                    numinous_core::studio_recipe_count()
                )];
                for (i, source) in numinous_core::STUDIO_RECIPES.iter().enumerate() {
                    lines.push(format!("  {i}: {source}"));
                }
                lines.push(String::new());
                return emit(Ok(lines.join("\n")));
            }
            if animate && save.is_some() {
                return emit(Err(
                    "--save is for still Studio plots; omit --animate to save a .num file\n"
                        .to_string(),
                ));
            }
            if save.is_none() && (title.is_some() || author.is_some() || credit.is_some()) {
                return emit(Err(
                    "a title, author, or credit names a saved creation; add --save\n".to_string(),
                ));
            }
            let scale = numinous_core::StudioScale::from(scale);
            if save.is_none() && scale != numinous_core::StudioScale::Continuous {
                return emit(Err(
                    "--scale is stored with a creation; add --save\n".to_string()
                ));
            }
            let parametric = match (x_expr.as_deref(), y_expr.as_deref()) {
                (Some(x_expr), Some(y_expr)) => Some((x_expr, y_expr)),
                (None, None) => None,
                _ => {
                    return emit(Err(
                        "a parametric plot needs both --x-expr and --y-expr\n".to_string()
                    ));
                }
            };
            if let Some((x_expr, y_expr)) = parametric {
                if xmin.is_some() || xmax.is_some() {
                    return emit(Err(
                        "a parametric plot uses --tmin and --tmax, not --xmin and --xmax\n"
                            .to_string(),
                    ));
                }
                if expr.is_some() || recipe.is_some() || seed.is_some() || auto_step != 0 {
                    return emit(Err(
                        "--x-expr/--y-expr cannot be combined with a graph expression, --recipe, --seed, or --auto-step\n"
                            .to_string(),
                    ));
                }
                let tmin = tmin.unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN);
                let tmax = tmax.unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX);
                if animate {
                    if let Err(message) = parametric_report(
                        x_expr,
                        y_expr,
                        StudioParameters {
                            minimum: tmin,
                            maximum: tmax,
                            a: amin,
                            scale,
                        },
                        (width, height),
                    ) {
                        return emit(Err(message));
                    }
                    let before = journey.clone();
                    journey.play();
                    persist_progress_or_warn(&before, journey);
                    return plot_parametric_animate(
                        x_expr,
                        y_expr,
                        StudioParameters {
                            minimum: tmin,
                            maximum: tmax,
                            a: amin,
                            scale,
                        },
                        amax,
                        (width, height),
                    );
                }
                let report = match parametric_report(
                    x_expr,
                    y_expr,
                    StudioParameters {
                        minimum: tmin,
                        maximum: tmax,
                        a,
                        scale,
                    },
                    (width, height),
                ) {
                    Ok(report) => report,
                    Err(message) => return emit(Err(message)),
                };
                if let Some(path) = save.as_deref() {
                    match save_parametric_creation(
                        x_expr,
                        y_expr,
                        StudioParameters {
                            minimum: tmin,
                            maximum: tmax,
                            a,
                            scale,
                        },
                        CreationIdentity {
                            title: title.as_deref(),
                            author: author.as_deref(),
                            credit: credit.as_deref(),
                        },
                        path,
                    ) {
                        Ok(message) => print!("{message}"),
                        Err(message) => return emit(Err(message)),
                    }
                }
                journey.play();
                return emit(Ok(report));
            }
            if tmin.is_some() || tmax.is_some() {
                return emit(Err(
                    "--tmin and --tmax are only valid with --x-expr/--y-expr\n".to_string(),
                ));
            }
            let source = match resolve_plot_source(expr.as_deref(), recipe, seed, auto_step) {
                Ok(source) => source,
                Err(message) => return emit(Err(message)),
            };
            let xmin = xmin.unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN);
            let xmax = xmax.unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX);
            let request = match PlotRequest::new(
                source,
                Some(xmin),
                Some(xmax),
                Some(if animate { amin } else { a }),
                Some(width),
                Some(height),
            ) {
                Ok(request) => request,
                Err(error) => return emit(Err(plot_request_error(error))),
            };
            let expr = request.source().to_string();
            if animate {
                if let Err(message) = plot_report(&expr, xmin, xmax, amin, width, height) {
                    return emit(Err(message));
                }
                let before = journey.clone();
                journey.play();
                // The loop never returns; persist the play before it starts,
                // and say so if the ledger refuses, since no exit will.
                persist_progress_or_warn(&before, journey);
                plot_animate(&expr, xmin, xmax, amin, amax, width, height)
            } else {
                let report = match plot_report(&expr, xmin, xmax, a, width, height) {
                    Ok(report) => report,
                    Err(message) => return emit(Err(message)),
                };
                if let Some(path) = save.as_deref() {
                    match save_studio_creation_with_scale(
                        &expr,
                        StudioParameters {
                            minimum: xmin,
                            maximum: xmax,
                            a,
                            scale,
                        },
                        CreationIdentity {
                            title: title.as_deref(),
                            author: author.as_deref(),
                            credit: credit.as_deref(),
                        },
                        path,
                    ) {
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
                        format!(
                            "no tracks yet: numinous tune2 {} (paid API; free: numinous tune)",
                            st.id
                        )
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
            yes,
        } => radio_tune(&station, seconds, count.clamp(1, 10), yes),
        Command::Tune { seed, bars, out } => {
            journey.play();
            emit(tune_wav(seed, bars, &out))
        }
        Command::Call { id, guess, seed } => {
            journey.play();
            emit(call_report(&id, guess, seed.unwrap_or_else(pick_day)))
        }
        Command::Sing {
            expr,
            xmin,
            xmax,
            notes,
            a,
            scale,
            out,
        } => {
            journey.play();
            emit(resolve_sing_input(&expr, xmin, xmax, a).and_then(
                |(source, xmin, xmax, a, stored_scale)| {
                    sing_to_path(
                        &source,
                        xmin,
                        xmax,
                        notes,
                        a,
                        scale.map(Into::into).unwrap_or(stored_scale),
                        &out,
                    )
                },
            ))
        }
        Command::Fork {
            parent,
            out,
            expr,
            x_expr,
            y_expr,
            scale,
            title,
            author,
            credit,
        } => {
            journey.play();
            emit(fork_studio_creation_extended(
                &parent,
                ForkEdits {
                    expr: expr.as_deref(),
                    x_expr: x_expr.as_deref(),
                    y_expr: y_expr.as_deref(),
                    scale: scale.map(Into::into),
                    identity: CreationIdentity {
                        title: title.as_deref(),
                        author: author.as_deref(),
                        credit: credit.as_deref(),
                    },
                },
                &out,
            ))
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
                report_diagnostic(&message);
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

fn plot_parametric_animate(
    x_source: &str,
    y_source: &str,
    parameters: StudioParameters,
    amax: f64,
    size: (usize, usize),
) -> ExitCode {
    let frame_time = Duration::from_secs_f64(1.0 / 12.0);
    let mut stdout = std::io::stdout();
    let mut phase = 0.0_f64;
    loop {
        let a = parameters.a + (amax - parameters.a) * phase;
        match parametric_report(
            x_source,
            y_source,
            StudioParameters { a, ..parameters },
            size,
        ) {
            Ok(text) => {
                let _ = write!(
                    stdout,
                    "\x1b[2J\x1b[H{text}\na = {a:.3}   (Ctrl+C to stop)\n"
                );
                let _ = stdout.flush();
            }
            Err(message) => {
                report_diagnostic(&message);
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
    local_state_paths().radio_cache
}

/// Tune a station: call ElevenLabs Music with the station's brief, receive
/// raw PCM, and cache it as a WAV the app and CLI can loop.
fn radio_tune(station_id: &str, seconds: Option<u64>, count: usize, yes: bool) -> ExitCode {
    let Some(station) = numinous_core::station(station_id) else {
        eprintln!(
            "No station '{}' on the dial. See: numinous radio",
            terminal_safe(station_id)
        );
        return ExitCode::FAILURE;
    };
    // Money never moves silently: the player is told where the key came
    // from and what will be spent, and nothing happens without --yes. A key
    // quietly discovered in a .env file once funded a track the player
    // never knowingly ordered.
    let (key, key_source) = match std::env::var("ELEVENLABS_API_KEY") {
        Ok(key) => (key, "the ELEVENLABS_API_KEY environment variable"),
        Err(_) => match env_file_key() {
            Ok(key) => (key, "the .env file in this directory"),
            Err(_) => {
                eprintln!(
                    "Set ELEVENLABS_API_KEY to tune the radio. The station briefs are ready;
             see docs/MUSIC.md for the pipeline and pricing notes.
             The free local engine needs no key: numinous tune"
                );
                return ExitCode::FAILURE;
            }
        },
    };
    println!(
        "Tuning {count} track(s) for {} via ElevenLabs, a paid API call per track.",
        station.name
    );
    println!("Key source: {key_source}.");
    if !yes {
        println!(
            "Nothing was spent. Add --yes to proceed, or compose free and local instead: numinous tune"
        );
        return ExitCode::SUCCESS;
    }
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
            MusicRequestError::HttpStatus(response) => {
                let code = response.status();
                let detail = bounded_response_detail(response.into_body().into_reader());
                eprintln!("The station is off the air (HTTP {code}): {detail}");
                return false;
            }
            MusicRequestError::Request(e) => {
                eprintln!("Could not reach the tower: {e}");
                return false;
            }
        },
    };
    let Some(max_pcm_bytes) = max_track_bytes(seconds) else {
        eprintln!("The requested track duration is too large.");
        return false;
    };
    let pcm = match read_bounded(response.into_body().into_reader(), max_pcm_bytes) {
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
    let _cache_lock = match numinous_core::lock_local_state(dir) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("could not lock the radio cache: {error}");
            return false;
        }
    };
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
                terminal_safe_path(&path),
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

#[derive(Debug)]
enum MusicRequestError {
    HttpStatus(ureq::http::Response<ureq::Body>),
    Request(ureq::Error),
}

fn send_music_request(
    url: &str,
    key: &str,
    body: &str,
    timeout: std::time::Duration,
) -> Result<ureq::http::Response<ureq::Body>, Box<MusicRequestError>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(timeout))
        .build()
        .into();
    let response = agent
        .post(url)
        .header("xi-api-key", key)
        .content_type("application/json")
        .send(body)
        .map_err(|error| Box::new(MusicRequestError::Request(error)))?;
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(Box::new(MusicRequestError::HttpStatus(response)))
    }
}

fn read_bounded(mut reader: impl std::io::Read, limit: usize) -> std::io::Result<Option<Vec<u8>>> {
    let byte_limit = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader.by_ref().take(byte_limit).read_to_end(&mut bytes)?;
    Ok((bytes.len() <= limit).then_some(bytes))
}

/// Write a diagnostic to stderr, guaranteeing it ends its line.
///
/// Diagnostics are built in dozens of places across this file. Terminating
/// them here rather than trusting every author to remember a trailing newline
/// means a new message cannot strand the next shell prompt mid-row.
fn report_diagnostic(message: &str) {
    // Strip every trailing line ending, not just one, so a message that
    // accumulated two newlines or arrived with CRLF still prints as one line.
    eprintln!("{}", message.trim_end_matches(['\r', '\n']));
}

/// Untrusted text rendered safe to print. Shared with the other faces so one
/// definition of "safe to show a person" covers all of them.
fn terminal_safe(text: &str) -> String {
    numinous_core::display_safe(text)
}

fn terminal_safe_path(path: &Path) -> String {
    terminal_safe(&path.to_string_lossy())
}

fn bounded_response_detail(reader: impl std::io::Read) -> String {
    read_bounded(reader, 8 * 1024)
        .ok()
        .flatten()
        .map(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            terminal_safe(&text)
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
    if !pcm.len().is_multiple_of(4) {
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
        terminal_safe_path(path),
        pattern.seconds()
    ))
}

/// Turn `source` into a melody over `[xmin, xmax]` and write it as a WAV.
#[cfg(test)]
fn sing_wav(
    source: &str,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    path: &Path,
) -> Result<String, String> {
    sing_to_path(
        source,
        xmin,
        xmax,
        notes,
        a,
        numinous_core::StudioScale::Continuous,
        path,
    )
}

/// Turn `source` into a melody and write WAV (.wav) or MIDI (.mid/.midi).
fn sing_to_path(
    source: &str,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    scale: numinous_core::StudioScale,
    path: &Path,
) -> Result<String, String> {
    let request = SingRequest::new(source, Some(xmin), Some(xmax), Some(a), Some(notes))
        .map_err(sing_request_error)?;
    let spec = request
        .execute_with_scale(scale)
        .map_err(sing_request_error)?;
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("wav")
        .to_ascii_lowercase();
    match ext.as_str() {
        "wav" => {
            write_wav(path, &spec.render(44_100), 44_100, 1)?;
        }
        "mid" | "midi" => {
            std::fs::write(path, spec.midi()).map_err(|error| {
                format!("could not create {}: {error}", terminal_safe_path(path))
            })?;
        }
        other => {
            return Err(format!("sing writes .wav or .mid, not .{other}\n"));
        }
    }
    Ok(format!(
        "wrote {} ({:.1}s, {} notes) from y = {} on the {} scale\n",
        terminal_safe_path(path),
        spec.duration,
        spec.notes.len(),
        terminal_safe(source),
        scale.name()
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
        .ok_or_else(|| format!("no sim named '{}'. Try: numinous sims\n", terminal_safe(id)))?;
    let meta = sim.meta();
    let mut params = numinous_core::default_params(&meta);
    for entry in sets {
        let (name, value) = entry.split_once('=').ok_or_else(|| {
            format!(
                "--set expects name=value, got '{}'.\n",
                terminal_safe(entry)
            )
        })?;
        let index = meta
            .levers
            .iter()
            .position(|l| l.name == name)
            .ok_or_else(|| {
                format!(
                    "'{}' has no lever '{}'. Try: numinous sims\n",
                    terminal_safe(id),
                    terminal_safe(name)
                )
            })?;
        params[index] = value
            .parse()
            .map_err(|_| format!("'{}' is not a number\n", terminal_safe(value)))?;
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
            report_diagnostic(&message);
            ExitCode::FAILURE
        }
    }
}

/// The catalog listing, as human text or JSON.
fn rooms_report(json: bool) -> String {
    if json {
        let arr: Vec<serde_json::Value> =
            numinous_core::ROOM_CATALOG.iter().map(meta_json).collect();
        format!("{}\n", to_pretty(&serde_json::Value::Array(arr)))
    } else {
        let lines: Vec<String> = numinous_core::ROOM_CATALOG
            .iter()
            .map(|metadata| {
                format!(
                    "{:<16} {:<20} {}",
                    metadata.id, metadata.wing, metadata.title
                )
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
    _journey: &Journey,
) -> Result<String, String> {
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
    let m = room.meta();
    let action = numinous_core::room_action(room.as_ref());
    let goal = room.goal();
    Ok(if json {
        let mut value = meta_json(&m);
        value["action"] = serde_json::Value::String(action.to_string());
        if let Some(goal) = goal {
            value["goal"] = serde_json::Value::String(goal.to_string());
        }
        value["next"] = serde_json::json!({
            "command": format!("numinous render {}", m.id),
        });
        format!("{}\n", to_pretty(&value))
    } else {
        let goal = goal.map_or_else(String::new, |goal| format!("\nGoal: {goal}"));
        format!(
            "{} ({})\nWing: {}\nAction: {}{goal}\n\n{}\n\nPlay: numinous render {}\n",
            m.title,
            m.id,
            m.wing,
            terminal_action_line(room.as_ref()),
            m.blurb,
            m.id,
        )
    })
}

/// One room's earned explanation, or a guiding error when play is incomplete.
fn reveal_report(
    id: &str,
    json: bool,
    allow_hidden: bool,
    journey: &Journey,
) -> Result<String, String> {
    let Some(room) = find_room(id, allow_hidden) else {
        return Err(not_found_message(id));
    };
    let m = room.meta();
    if numinous_core::is_engineered_aha_room(m.id) && !journey.has_consolidated(m.id) {
        return Err(
            "This explanation is still closed. Complete the room's wager and summon in the App or through play_room first."
                .to_string(),
        );
    }
    if !numinous_core::is_engineered_aha_room(m.id) && !journey.visited.contains(m.id) {
        return Err(
            "This explanation is still closed. Render or play the room once, then ask again."
                .to_string(),
        );
    }

    let level = journey.level();
    let mut cuts = String::new();
    let mut structured_cuts = Vec::new();
    for (i, cut) in room.deep_cuts().iter().enumerate() {
        let need = CUT_LEVELS.get(i).copied().unwrap_or(u32::MAX);
        let by_boon = journey.chosen.contains(&format!("cut:{}:{i}", m.id));
        if level >= need || by_boon {
            let label = if i == 0 { "Deeper" } else { "Deeper still" };
            cuts.push_str(&format!("\n{label}: {cut}\n"));
            structured_cuts.push(serde_json::json!({
                "index": i,
                "status": "available",
                "unlock_level": need,
                "text": cut,
            }));
        } else {
            cuts.push_str(&format!("\nLOCKED: a deeper cut opens at LV {need}.\n"));
            structured_cuts.push(serde_json::json!({
                "index": i,
                "status": "locked",
                "unlock_level": need,
            }));
            break;
        }
    }
    let cut0_by_boon = journey.chosen.contains(&format!("cut:{}:0", m.id));
    let citation = numinous_core::room_citation_unlocked(m.id, level, cut0_by_boon);

    Ok(if json {
        let mut value = meta_json(&m);
        value["reveal"] = serde_json::Value::String(room.reveal().to_string());
        if let Some(concept) = room.concept() {
            value["concept"] = serde_json::Value::String(concept.to_string());
        }
        value["deep_cuts"] = serde_json::Value::Array(structured_cuts);
        if let Some(citation) = citation {
            value["citation"] = serde_json::Value::String(citation.to_string());
        }
        format!("{}\n", to_pretty(&value))
    } else {
        let mut text = numinous_core::explain_text(m.id, room.reveal());
        text.push_str(&cuts);
        if let Some(citation) = citation {
            text.push_str(&format!("\n{citation}\n"));
        }
        format!("{text}\n")
    })
}

/// A room rendered in truecolor ANSI (two pixels per terminal cell).
fn render_color_report(
    id: &str,
    width: usize,
    height: usize,
    t: f64,
    allow_hidden: bool,
    style: TerminalStyle,
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
    // No trailing reset: a colored frame already ends every line with one, and
    // a mono frame must stay free of escapes entirely.
    let mut report = ansi_in_era(&raster, style.era, style.color);
    report.push_str(&render_guidance(room.as_ref(), t, input));
    Ok(report)
}

/// How a room is presented in the terminal: which visual era, and whether
/// color may be added at all. Both are presentation, so they travel together.
#[derive(Clone, Copy)]
struct TerminalStyle {
    era: numinous_core::Era,
    color: bool,
}

/// Encode a raster for the terminal after applying a visual era.
fn ansi_in_era(raster: &Raster, era: numinous_core::Era, color: bool) -> String {
    let (w, h) = (raster.width(), raster.height());
    let mut rgba = raster.to_rgba();
    era.apply(&mut rgba, w, h);
    let mut styled = Raster::new(w, h);
    styled.set_rgba(&rgba);
    numinous_core::to_terminal(&styled, color)
}

/// The touch verbs this face cannot hear. A terminal has no mouse route, so
/// printing these is an advertisement with no way to act; the honest copy
/// names this face's own hands instead (`--poke x,y` and `--t`). MOVE rides
/// along because at least one room (the Galton bet) advertises the pointer
/// hover as its own lever.
const UNHEARD_TOUCH_VERBS: [&str; 4] = ["DRAG", "CLICK", "HOLD", "MOVE"];

/// Drop gesture fragments (DRAG:TUNE, CLICK: SEED A GAP, a bare DRAG) from
/// a status readout. Fragments run from the verb to the next double-space
/// column gap or the end of the line, matching how room statuses lay out
/// their columns, and a column that is nothing but the verb goes with them:
/// scrubbing only the colon form left dozens of rooms still advertising a
/// gesture this face cannot hear.
fn scrub_touch_fragments(status: &str) -> String {
    let mut out = status.to_string();
    for verb in UNHEARD_TOUCH_VERBS {
        let marker = format!("{verb}:");
        while let Some(start) = out.find(&marker) {
            let end = out[start..].find("  ").map_or(out.len(), |gap| start + gap);
            out.replace_range(start..end, "");
        }
    }
    // Column-wise pass for the bare verb. Whole columns only, so a reading
    // that merely contains the letters (OVERLAP, CLICKS) keeps its place.
    let kept: Vec<&str> = out
        .split("  ")
        .filter(|column| {
            let bare = column.trim().trim_end_matches(['.', ',', ';', '!']);
            !UNHEARD_TOUCH_VERBS
                .iter()
                .any(|verb| bare.eq_ignore_ascii_case(verb))
        })
        .collect();
    let mut tidy = kept.join("  ").trim().to_string();
    while tidy.contains("   ") {
        tidy = tidy.replace("   ", "  ");
    }
    tidy
}

/// The Action line, translated for a keyboard face: the lever keeps its
/// name, and the hand becomes the flag that actually moves it here.
fn terminal_action_line(room: &dyn Room) -> String {
    if room.meta().id == "times-tables" {
        return "TURN THE DIAL (phase here: numinous render times-tables --t 0.375; --poke x,y is a second hand)".to_string();
    }
    match room.verb() {
        Some(verb) => {
            let lever = verb.split_once(':').map_or(verb, |(_, lever)| lever.trim());
            format!(
                "{lever} (the hand here: numinous render {} --poke x,y)",
                room.meta().id
            )
        }
        None => numinous_core::room_action(room).to_string(),
    }
}

/// Arm Ctrl+C to flip a latch instead of killing the process mid-frame.
/// Installing can fail (a prior handler, an exotic host); the loops then
/// keep the old die-on-signal behavior rather than refusing to run.
fn interrupt_latch() -> Option<std::sync::Arc<std::sync::atomic::AtomicBool>> {
    let latch = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&latch);
    ctrlc::set_handler(move || flag.store(true, std::sync::atomic::Ordering::SeqCst))
        .ok()
        .map(|()| latch)
}

fn interrupted(latch: &Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) -> bool {
    latch
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::SeqCst))
}

/// The first whole sentence of a passage.
///
/// A period alone does not end a sentence: reveals carry decimals (pi is
/// 3.14159), ellipses, and abbreviations, and cutting at the first dot
/// leaves a fragment that can state something false. A terminator counts
/// only when a digit does not sit on both sides of it and the passage
/// moves on with a space.
fn first_sentence(text: &str) -> String {
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if !matches!(byte, b'.' | b'!' | b'?') {
            continue;
        }
        let before = index.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(index + 1).copied();
        // Inside a number, or inside an ellipsis: keep reading.
        if before.is_some_and(|b| b.is_ascii_digit()) && after.is_some_and(|b| b.is_ascii_digit()) {
            continue;
        }
        if after == Some(b'.') || before == Some(b'.') {
            continue;
        }
        match after {
            None => return text[..=index].trim().to_string(),
            Some(next) if next.is_ascii_whitespace() => {
                return text[..=index].trim().to_string();
            }
            _ => continue,
        }
    }
    text.trim().to_string()
}

/// The two-line exit that completes the staircase when Ctrl+C ends a live
/// view: the first sentence of the reveal as a tease, then the route to the
/// whole story. Leaving is the player's verb here, not an error.
fn viewing_epilogue(room: &dyn Room) -> String {
    let tease = first_sentence(room.reveal());
    format!(
        "\n{tease}\nThe story: numinous describe {}\n",
        room.meta().id
    )
}

/// One truecolor frame of a room with a status line, for the watch loop.
fn watch_frame(
    room: &dyn Room,
    t: f64,
    width: usize,
    height: usize,
    style: TerminalStyle,
) -> String {
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    let readout = room
        .status(t)
        .map(|line| format!("   {}", scrub_touch_fragments(&line)))
        .unwrap_or_default();
    format!(
        // Cursor home and erase-line are cursor control, not color, so they
        // stay in both modes. The reset does not: the frame already ended one.
        "\x1b[H{}{}  t = {t:.2}{readout}   (Ctrl+C to stop)\x1b[K\n",
        ansi_in_era(&raster, style.era, style.color),
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
        report_diagnostic(&not_found_message(id));
        return ExitCode::FAILURE;
    };
    let player = if mute {
        None
    } else {
        // Silence must never be a mystery: a device that will not open is
        // named once on stderr, then the room plays visual-only.
        match numinous_audio::LoopPlayer::new() {
            Ok(player) => Some(player),
            Err(error) => {
                eprintln!(
                    "{}",
                    terminal_safe(&format!("sound unavailable, playing silent: {error}"))
                );
                None
            }
        }
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let motion = numinous_core::Motion::from_env();
    let mut stdout = std::io::stdout();
    // Clear once; frames then repaint in place (no flicker).
    let _ = write!(stdout, "\x1b[2J");
    let latch = interrupt_latch();
    let mut t = 0.0f64;
    let mut frame = 0u64;
    loop {
        if interrupted(&latch) {
            println!("{}", viewing_epilogue(room.as_ref()));
            return ExitCode::SUCCESS;
        }
        let _ = write!(
            stdout,
            "{}[J",
            watch_frame(
                room.as_ref(),
                t,
                width,
                height,
                TerminalStyle {
                    era,
                    color: color_allowed()
                },
            )
        );
        let _ = stdout.flush();
        if let Some(player) = &player {
            player.service();
        }
        // Refresh the room's voice a few times per sweep.
        if frame.is_multiple_of(24)
            && let Some(player) = &player
        {
            let spec = room.sound(t);
            player.set_samples(spec.render(player.sample_rate()));
        }
        std::thread::sleep(frame_time);
        // Reduced motion holds the phase, so the loop keeps drawing and
        // keeps responding; only the movement stops.
        t = motion.next_phase(t, 0.005);
        frame += 1;
    }
}

/// How The Show leaves one room for the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Advance {
    /// Animate the room, then move on after this many frames.
    Timer(u64),
    /// Hold the room still and move on when the player asks for it.
    Player,
}

/// What moves The Show along, given the player's motion setting.
///
/// The App already decides this, by different means: reduced motion zeroes its
/// ambient tick, so the sweep never completes, and it is the completed sweep
/// that carries the gallery into the next room. The terminal had no equivalent.
/// It held each room's phase still, which stopped the picture moving, and then
/// advanced anyway on a frame count that never consulted the setting. Two faces
/// disagreeing about one preference is worse than either answer alone, because
/// a player who sets it once cannot tell which face will honor it.
fn show_advance(motion: numinous_core::Motion, seconds: f64, fps: f64) -> Advance {
    match motion {
        numinous_core::Motion::Reduced => Advance::Player,
        numinous_core::Motion::Full => Advance::Timer((seconds.max(2.0) * fps.max(1.0)) as u64),
    }
}

/// What the player asked for at a held room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowStep {
    /// Move to the next room.
    Next,
    /// Leave The Show.
    Quit,
}

/// Read one instruction from a held room.
///
/// End of input is a quit and not a repeat. A held gallery blocks on the
/// player, so a closed or piped stdin would otherwise turn waiting into a loop
/// that draws forever and never yields, which is a worse failure than stopping.
fn show_step(line: Option<&str>) -> ShowStep {
    match line.map(str::trim) {
        None => ShowStep::Quit,
        Some(text) if text.eq_ignore_ascii_case("q") || text.eq_ignore_ascii_case("quit") => {
            ShowStep::Quit
        }
        Some(_) => ShowStep::Next,
    }
}

/// One room's screen in The Show: the picture, then its title card or reveal.
///
/// The chrome is gated on the same `color` decision as the picture. It used to
/// be written with a hardcoded bold and reset, so a `NO_COLOR` player got a
/// color-free picture underneath two escape sequences.
fn tour_screen(
    room: &dyn Room,
    t: f64,
    width: usize,
    height: usize,
    style: TerminalStyle,
) -> String {
    let mut screen = watch_frame(room, t, width, height, style);
    let meta = room.meta();
    if t < 0.18 {
        if style.color {
            screen.push_str(&format!(
                "\x1b[1m{}\x1b[0m  ({})\x1b[K\n",
                meta.title, meta.wing
            ));
        } else {
            screen.push_str(&format!("{}  ({})\x1b[K\n", meta.title, meta.wing));
        }
    } else if t > 0.86 {
        screen.push_str(&format!("{}\x1b[K\n", room.reveal()));
    } else {
        screen.push_str("\x1b[K\n");
    }
    screen
}

/// The size and styling one room of The Show is drawn at.
#[derive(Clone, Copy)]
struct TourFrame {
    width: usize,
    height: usize,
    style: TerminalStyle,
}

/// The Show held still: one room at a time, at rest, until the player asks for
/// the next one.
///
/// Returns the rooms shown, in order, so a test can prove that nothing advanced
/// on its own and that the gallery stopped when the input did.
fn tour_held(
    rooms: &[Box<dyn Room>],
    journey: &mut Journey,
    frame: TourFrame,
    player: Option<&numinous_audio::LoopPlayer>,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Vec<&'static str> {
    let TourFrame {
        width,
        height,
        style,
    } = frame;
    let mut shown = Vec::new();
    if rooms.is_empty() {
        return shown;
    }
    for room in rooms.iter().cycle() {
        let meta = room.meta();
        // Held tours can also end by Ctrl+C rather than q, so each visit
        // persists as it happens, like the timer tour's.
        let before = journey.clone();
        journey.visit(meta.id);
        persist_progress_or_warn(&before, journey);
        shown.push(meta.id);
        // The postcard phase: the frame the room itself considers its best
        // face. A held room rests on a chosen picture rather than on whichever
        // frame the clock happened to stop at.
        let t = room.postcard_t();
        let screen = tour_screen(room.as_ref(), t, width, height, style);
        let _ = write!(out, "{screen}\x1b[J");
        let _ = writeln!(out, "Enter for the next room, q to leave.\x1b[K");
        let _ = out.flush();
        if let Some(player) = player {
            player.set_samples(room.sound(t).render(player.sample_rate()));
            player.service();
        }
        let mut line = String::new();
        let read = input.read_line(&mut line).unwrap_or(0);
        let answer = if read == 0 { None } else { Some(line.as_str()) };
        if let Some(player) = player {
            player.service();
        }
        if show_step(answer) == ShowStep::Quit {
            break;
        }
    }
    // The held tour's designed exits (q and end of input) earn the same
    // epilogue as the timer tour's Ctrl+C, so a reduced-motion player is
    // not the one audience whose staircase stops short of the reveal.
    // Ctrl+C itself stays die-on-signal here: a latch cannot be polled
    // in the middle of a blocking line read, and a Ctrl+C that silently
    // waits for Enter would read as a broken key.
    if let Some(last) = shown.last()
        && let Some(room) = rooms.iter().find(|room| room.meta().id == *last)
    {
        let _ = writeln!(out, "{}", viewing_epilogue(room.as_ref()));
    }
    shown
}

/// The Show, in the terminal: every room takes the stage in turn, full color
/// and sound, with a title card and its reveal as the curtain line. Ctrl+C
/// whenever you have had enough; it comes back around forever.
///
/// Under reduced motion it does not come around by itself: each room is held at
/// its postcard phase and the player says when to move on.
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
        // Silence must never be a mystery: a device that will not open is
        // named once on stderr, then the room plays visual-only.
        match numinous_audio::LoopPlayer::new() {
            Ok(player) => Some(player),
            Err(error) => {
                eprintln!(
                    "{}",
                    terminal_safe(&format!("sound unavailable, playing silent: {error}"))
                );
                None
            }
        }
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let motion = numinous_core::Motion::from_env();
    let style = TerminalStyle {
        era,
        color: color_allowed(),
    };
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "\x1b[2J");
    let rooms = all_rooms();

    let frames_per_room = match show_advance(motion, seconds, fps) {
        Advance::Player => {
            let stdin = std::io::stdin();
            let mut input = stdin.lock();
            tour_held(
                &rooms,
                journey,
                TourFrame {
                    width,
                    height,
                    style,
                },
                player.as_ref(),
                &mut input,
                &mut stdout,
            );
            return ExitCode::SUCCESS;
        }
        Advance::Timer(frames) => frames,
    };

    let latch = interrupt_latch();
    loop {
        for room in &rooms {
            // Ctrl+C now lands on the epilogue below, but a latch that could
            // not install still dies on the signal, so each visit persists as
            // it happens or it never persists at all: a whole gallery watched
            // and zero stars lit is a silent loss.
            let before = journey.clone();
            journey.visit(room.meta().id);
            persist_progress_or_warn(&before, journey);
            for frame in 0..frames_per_room {
                if interrupted(&latch) {
                    println!("{}", viewing_epilogue(room.as_ref()));
                    return ExitCode::SUCCESS;
                }
                let t = frame as f64 / frames_per_room as f64;
                let screen = tour_screen(room.as_ref(), t, width, height, style);
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
        guidance.push_str(&format!("Status: {}\n", scrub_touch_fragments(&status)));
    }
    guidance.push_str(&format!("Action: {}\n", terminal_action_line(room)));
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
fn journey_report(journey: &Journey, board: &numinous_core::Scoreboard, today: u64) -> String {
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
        numinous_core::ROOM_CATALOG.len(),
        journey.wins,
        journey.secrets,
        match journey.live_streak(today) {
            Some(chain) if chain > 1 => format!(" Streak {chain}."),
            // A dead chain becomes a record, not a claim: honesty and the
            // no-scolding law can hold the same line.
            _ if journey.streak > 1 => format!(" Best chain {}.", journey.streak),
            _ => String::new(),
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
        terminal_safe_path(path),
        raster.width(),
        raster.height()
    );
    report.push_str(&render_guidance(room.as_ref(), t, input));
    Ok(report)
}

/// Encode a raster as an RGBA PNG at `path`.
fn write_png(path: &Path, raster: &Raster) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(path)))?;
    write_png_to(file, raster)
}

fn write_png_new(path: &Path, raster: &Raster) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(path)))?;
    write_png_to(file, raster)
}

fn write_derived_png(path: &Path, raster: &Raster) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            std::fs::remove_file(path)
                .map_err(|e| format!("could not replace {}: {e}", terminal_safe_path(path)))?;
        }
        Ok(_) => {
            return Err(format!(
                "refusing to replace non-file gallery member {}",
                terminal_safe_path(path)
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect {}: {error}",
                terminal_safe_path(path)
            ));
        }
    }
    write_png_new(path, raster)
}

fn write_png_to(file: File, raster: &Raster) -> Result<(), String> {
    let (w, h) = (raster.width(), raster.height());
    let mut encoder = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header failed: {e}"))?;
    writer
        .write_image_data(&raster.to_rgba())
        .map_err(|e| format!("png write failed: {e}"))?;
    // Finish explicitly: the drop path swallows the IEND write and the final
    // flush, and a truncated PNG announced as written is a lie with a
    // filename. The APNG and WAV writers already surface this moment.
    writer
        .finish()
        .map_err(|e| format!("png finish failed: {e}"))?;
    Ok(())
}

/// Short loop frame count and timing match the App Share path (2 s at 12 fps).
const LOOP_FRAMES: u32 = 24;
const LOOP_DELAY_NUM: u16 = 1;
const LOOP_DELAY_DEN: u16 = 12;
const MAX_APNG_FRAME_BYTES: usize = 64 * 1024 * 1024;

fn apng_frame_bytes(width: usize, height: usize) -> Result<usize, String> {
    let bytes = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "APNG frame dimensions overflow the allocation size.".to_string())?;
    if bytes > MAX_APNG_FRAME_BYTES {
        return Err(format!(
            "APNG frame needs {bytes} bytes; the limit is {MAX_APNG_FRAME_BYTES}."
        ));
    }
    Ok(bytes)
}

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
    exclusive: bool,
) -> Result<String, String> {
    let room = find_room_with_variation(id, allow_hidden, input.variation)
        .ok_or_else(|| not_found_message(id))?;
    let expected_frame_bytes = apng_frame_bytes(size, size)?;
    let frames = (0..LOOP_FRAMES).map(|index| {
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
        if rgba.len() != expected_frame_bytes {
            return Err(format!(
                "APNG frame has {} bytes; expected {expected_frame_bytes}.",
                rgba.len()
            ));
        }
        Ok(rgba)
    });
    write_apng(
        path,
        size as u32,
        size as u32,
        LOOP_FRAMES,
        frames,
        exclusive,
    )?;
    let mut report = format!(
        "wrote {} ({}x{}, {LOOP_FRAMES} frames, loop)\n",
        terminal_safe_path(path),
        size,
        size
    );
    report.push_str(&render_guidance(room.as_ref(), start_t, input));
    Ok(report)
}

/// Package still + loop + README into one share folder.
fn render_share_bundle(
    id: &str,
    parent: &Path,
    size: usize,
    t: f64,
    allow_hidden: bool,
    era: numinous_core::Era,
    variation: u64,
) -> Result<String, String> {
    // The still, the loop, and the recorded metadata must be one visit: a
    // postcard rendered from the base deal beside a loop of variation N is
    // a bundle that disagrees with itself and can never be replayed.
    let Some(room) = find_room_with_variation(id, allow_hidden, variation) else {
        return Err(not_found_message(id));
    };
    std::fs::create_dir_all(parent).map_err(|e| format!("could not create share parent: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| format!("could not generate share name: {e}"))?;
    let dir = numinous_core::create_share_bundle_dir(parent, room.meta().id, stamp, nonce)
        .map_err(|e| format!("could not create share folder: {e}"))?;

    let postcard_path = dir.join("postcard.png");
    let loop_path = dir.join("loop.png");
    let input = RoomRenderInput::new(variation, &[]);
    // Still: one frame at t.
    let mut raster = Raster::with_accent(size, size, room.meta().accent);
    room.render(&mut raster, t);
    let mut rgba = raster.to_rgba();
    if era != numinous_core::Era::Modern {
        era.apply(&mut rgba, raster.width(), raster.height());
    }
    write_png_file(&postcard_path, size as u32, size as u32, &rgba)?;
    let _ = numinous_core::write_share_sidecar(
        &postcard_path,
        &numinous_core::ShareMeta {
            room_id: room.meta().id.to_string(),
            era: era.name().to_string(),
            kind: numinous_core::ShareKind::Postcard,
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );
    // Loop APNG into the bundle.
    render_loop_apng(id, size, t, &loop_path, allow_hidden, era, input, true)?;
    let _ = numinous_core::write_share_sidecar(
        &loop_path,
        &numinous_core::ShareMeta {
            room_id: room.meta().id.to_string(),
            era: era.name().to_string(),
            kind: numinous_core::ShareKind::Loop,
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );
    numinous_core::write_share_bundle_readme(
        &dir,
        &numinous_core::ShareBundleMeta {
            room_id: room.meta().id.to_string(),
            era: era.name().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            variation,
        },
        true,
        true,
    )
    .map_err(|e| format!("could not write share README: {e}"))?;
    Ok(format!(
        "wrote share bundle {}\n  postcard.png  loop.png  README.share.txt\n",
        terminal_safe_path(&dir)
    ))
}

/// Write a single-frame PNG (Share still inside a bundle).
fn write_png_file(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(path)))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header failed: {e}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("png write failed: {e}"))?;
    // Finish explicitly, as write_png_to does: the drop path swallows the
    // IEND write and the final flush.
    writer
        .finish()
        .map_err(|e| format!("png finish failed: {e}"))?;
    Ok(())
}

/// Encode a square looping APNG (Share v1 short loop).
fn write_apng(
    path: &Path,
    width: u32,
    height: u32,
    frame_count: u32,
    frames: impl IntoIterator<Item = Result<Vec<u8>, String>>,
    exclusive: bool,
) -> Result<(), String> {
    let expected_frame_bytes = apng_frame_bytes(width as usize, height as usize)?;
    let file = if exclusive {
        OpenOptions::new().write(true).create_new(true).open(path)
    } else {
        File::create(path)
    }
    .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(path)))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    encoder
        .set_animated(frame_count, 0)
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
    let mut written = 0_u32;
    for frame in frames {
        let frame = frame?;
        if frame.len() != expected_frame_bytes {
            return Err(format!(
                "APNG frame has {} bytes; expected {expected_frame_bytes}.",
                frame.len()
            ));
        }
        writer
            .write_image_data(&frame)
            .map_err(|e| format!("apng frame write failed: {e}"))?;
        written += 1;
    }
    if written != frame_count {
        return Err(format!(
            "APNG frame source produced {written} frames; expected {frame_count}."
        ));
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
        terminal_safe_path(path),
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
                terminal_safe_path(path),
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
                terminal_safe_path(path),
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
    if !(1..=2).contains(&channels) || !samples.len().is_multiple_of(usize::from(channels)) {
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
        .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(path)))?;
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
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create {}: {e}", terminal_safe_path(dir)))?;
    let mut count = 0usize;
    for room in all_rooms() {
        let id = room.meta().id;
        let path = dir.join(format!("{id}.png"));
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, room.postcard_t());
        write_derived_png(&path, &raster)?;
        count += 1;
    }
    Ok(format!(
        "wrote {count} room images to {}\n",
        terminal_safe_path(dir)
    ))
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

/// Build one terminal frame: clear the screen, render the room, and add a status
/// line. Pure and testable; the animation loop just prints these in sequence.
fn play_frame(room: &dyn Room, t: f64, width: usize, height: usize) -> String {
    let mut canvas = Canvas::new(width, height);
    room.render(&mut canvas, t);
    let status = room
        .status(t)
        .map(|readout| format!("   {}", scrub_touch_fragments(&readout)))
        .unwrap_or_default();
    // \x1b[2J clears the screen, \x1b[H moves the cursor home.
    format!(
        "\x1b[2J\x1b[H{}\n[{}]  {}   t = {t:.2}{status}   (Ctrl+C to stop)\n",
        canvas.to_text(),
        room.meta().title,
        terminal_action_line(room)
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
        report_diagnostic(&not_found_message(id));
        return ExitCode::FAILURE;
    };
    let frame_time = Duration::from_secs_f64(1.0 / fps.max(1.0));
    let motion = numinous_core::Motion::from_env();
    let latch = interrupt_latch();
    let mut stdout = std::io::stdout();
    let mut t = 0.0f64;
    loop {
        if interrupted(&latch) {
            println!("{}", viewing_epilogue(room.as_ref()));
            return ExitCode::SUCCESS;
        }
        let _ = write!(stdout, "{}", play_frame(room.as_ref(), t, width, height));
        let _ = stdout.flush();
        std::thread::sleep(frame_time);
        t = motion.next_phase(t, 0.01);
    }
}

/// The universal call, on the terminal's own terms.
///
/// The App aims a band with a hand or an arrow key; a terminal has neither,
/// so the same commitment is made the way a terminal makes commitments: ask
/// once to hear the question, answer with a number. Both halves are
/// deterministic and stateless, so the question keeps its answer between the
/// two runs, and the day's seed means a day has one call worth comparing.
fn call_report(id: &str, guess: Option<f64>, seed: u64) -> Result<String, String> {
    let Some(room) = find_room(id, false) else {
        return Err(not_found_message(id));
    };
    let Some(posed) = numinous_core::pose_prediction(room.as_ref(), seed) else {
        return Err(format!(
            "{} reads no moving number to call. Rooms with a readout can be called; this one answers in shape alone.\n",
            terminal_safe(room.meta().title)
        ));
    };
    let (lo, hi) = posed.span;
    let Some(guess) = guess else {
        // The shared prompt is written for a tool caller and offers a rate
        // commitment this command does not accept; printing it here would
        // advertise a verb this face cannot hear. The question is the same
        // question, said in the terminal's own words.
        let label = terminal_safe(&posed.label);
        let title = terminal_safe(room.meta().title);
        let phase = posed.phase;
        return Ok([
            format!(
                "Call it before you look. What does {label} read at phase {phase:.3} in {title}?"
            ),
            format!("Across the sweep it runs {lo} to {hi}."),
            format!("Answer with: numinous call {id} --guess <number> --seed {seed}"),
            String::new(),
        ]
        .join("\n"));
    };
    let Some(grade) = numinous_core::grade_prediction(room.as_ref(), &posed, guess) else {
        return Err(format!(
            "{} lost its readout at that phase.
",
            terminal_safe(id)
        ));
    };
    // The bands predict already speaks: a miss is fertile, never punished,
    // and the truth is named whichever way the call went.
    let verdict = match grade.band {
        numinous_core::Band::Nailed => "Nailed.",
        numinous_core::Band::Close => "Close: the fertile band.",
        numinous_core::Band::Wild => "A wild swing; the gap is the lesson.",
    };
    Ok(format!(
        "You called {} for {}; it reads {}. {verdict}
",
        grade.guess,
        terminal_safe(&posed.label),
        grade.actual,
    ))
}

/// Answer an unknown room id with the rooms it was probably meant to be, then
/// one pointer to the browse command. Listing the whole catalog would both bury
/// the answer and hand over the map this project deliberately withholds
/// (`PLAY.md`: finding your own way is the point).
fn not_found_message(id: &str) -> String {
    let suggestions = numinous_core::nearest_room_ids(id, numinous_core::MAX_ROOM_SUGGESTIONS);
    let mut message = format!("No room with id '{}'.", numinous_core::echoable_id(id));
    if !suggestions.is_empty() {
        message.push_str(" Did you mean: ");
        message.push_str(&suggestions.join(", "));
        message.push('?');
    }
    message.push_str(" Run 'numinous rooms' to browse the catalog.\n");
    message
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
mod tests;
