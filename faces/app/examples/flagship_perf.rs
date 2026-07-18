//! Five-flagship App render and dispatch latency baseline.
//!
//! This headless harness measures the software interval from accepted input to
//! a completed room raster. It uses the same room, persistent Life, Studio,
//! and input-feedback paths as the App. It deliberately excludes native event
//! translation and history storage, window presentation, display scan-out,
//! audio submission and callbacks, and human perception. Run the
//! reference-machine gate with:
//!
//! `cargo run --release --locked -p numinous-app --example flagship_perf -- --check`

use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use numinous_core::rooms::game_of_life::LifeSession;
use numinous_core::{Raster, Room, RoomInput, room_by_id};

#[allow(dead_code)]
#[path = "../src/input_feedback.rs"]
mod input_feedback;
#[allow(dead_code)]
#[path = "../src/input_legend.rs"]
mod input_legend;
#[path = "../src/room_phase.rs"]
mod room_phase;
#[allow(dead_code)]
#[path = "../src/studio_panel.rs"]
mod studio_panel;

use input_legend::InputMode;
use room_phase::effective_room_phase;
use studio_panel::StudioPanel;

const DEFAULT_SAMPLES: usize = 40;
const DEFAULT_WARMUP: usize = 5;
const DEFAULT_WIDTH: usize = 900;
const DEFAULT_HEIGHT: usize = 700;
const DEFAULT_BUDGET_MS: f64 = 33.0;
const MIN_DIMENSION: usize = 64;
const MAX_DIMENSION: usize = 4096;
const MAX_SAMPLES: usize = 10_000;
const MEASURED_PATHS: usize = 10;
const MAX_PIXEL_DRAWS: usize = 2_000_000_000;

const HELP: &str = "Five-flagship App performance baseline\n\
\n\
Usage: flagship_perf [options]\n\
\n\
Options:\n\
  --samples N       Measured samples per path, default 40\n\
  --warmup N        Warmup samples per path, default 5\n\
  --width N         Raster width, default 900\n\
  --height N        Raster height, default 700\n\
  --budget-ms N     P95 budget in milliseconds, default 33\n\
  --check           Fail when any P95 exceeds the budget; requires --release\n\
  --help            Show this help\n";

#[derive(Debug, Clone, Copy, PartialEq)]
struct Config {
    samples: usize,
    warmup: usize,
    width: usize,
    height: usize,
    budget_ms: f64,
    check: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            samples: DEFAULT_SAMPLES,
            warmup: DEFAULT_WARMUP,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            budget_ms: DEFAULT_BUDGET_MS,
            check: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Help,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Stats {
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Ambient,
    InputToRaster,
}

impl PathKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Ambient => "ambient raster",
            Self::InputToRaster => "input to room raster",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Measurement {
    category: &'static str,
    flagship: &'static str,
    path: PathKind,
    stats: Stats,
}

fn parse_args<I>(args: I) -> Result<(Action, Config), String>
where
    I: IntoIterator<Item = String>,
{
    let mut config = Config::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => return Ok((Action::Help, config)),
            "--check" => config.check = true,
            "--samples" => {
                config.samples = parse_usize("--samples", args.next())?;
            }
            "--warmup" => {
                config.warmup = parse_usize("--warmup", args.next())?;
            }
            "--width" => {
                config.width = parse_usize("--width", args.next())?;
            }
            "--height" => {
                config.height = parse_usize("--height", args.next())?;
            }
            "--budget-ms" => {
                config.budget_ms = parse_f64("--budget-ms", args.next())?;
            }
            unknown => return Err(format!("unknown option: {unknown}")),
        }
    }
    validate_config(config)?;
    Ok((Action::Run, config))
}

fn parse_usize(name: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("{name} requires a value"))?;
    value
        .parse::<usize>()
        .map_err(|_| format!("{name} requires a nonnegative integer, got {value}"))
}

fn parse_f64(name: &str, value: Option<String>) -> Result<f64, String> {
    let value = value.ok_or_else(|| format!("{name} requires a value"))?;
    value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a number, got {value}"))
}

fn validate_config(config: Config) -> Result<(), String> {
    if !(1..=MAX_SAMPLES).contains(&config.samples) {
        return Err(format!(
            "--samples must be between 1 and {MAX_SAMPLES}, got {}",
            config.samples
        ));
    }
    if config.warmup > MAX_SAMPLES {
        return Err(format!(
            "--warmup must be at most {MAX_SAMPLES}, got {}",
            config.warmup
        ));
    }
    for (name, dimension) in [("--width", config.width), ("--height", config.height)] {
        if !(MIN_DIMENSION..=MAX_DIMENSION).contains(&dimension) {
            return Err(format!(
                "{name} must be between {MIN_DIMENSION} and {MAX_DIMENSION}, got {dimension}"
            ));
        }
    }
    if !config.budget_ms.is_finite() || config.budget_ms <= 0.0 {
        return Err(format!(
            "--budget-ms must be finite and positive, got {}",
            config.budget_ms
        ));
    }
    let pixels = config
        .width
        .checked_mul(config.height)
        .ok_or_else(|| "raster dimensions overflow".to_string())?;
    let draws = config
        .samples
        .checked_add(config.warmup)
        .and_then(|count| count.checked_mul(MEASURED_PATHS))
        .ok_or_else(|| "sample count overflow".to_string())?;
    let pixel_draws = pixels
        .checked_mul(draws)
        .ok_or_else(|| "requested benchmark work overflows".to_string())?;
    if pixel_draws > MAX_PIXEL_DRAWS {
        return Err(format!(
            "requested benchmark work is too large; limit is {MAX_PIXEL_DRAWS} pixel draws"
        ));
    }
    Ok(())
}

fn quantile(sorted_ms: &[f64], proportion: f64) -> f64 {
    let rank = (sorted_ms.len() as f64 * proportion).ceil() as usize;
    sorted_ms[rank.saturating_sub(1).min(sorted_ms.len() - 1)]
}

fn summarize(durations: &[Duration]) -> Stats {
    let mut milliseconds = durations
        .iter()
        .map(|duration| duration.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    milliseconds.sort_by(f64::total_cmp);
    Stats {
        p50_ms: quantile(&milliseconds, 0.50),
        p95_ms: quantile(&milliseconds, 0.95),
        max_ms: *milliseconds.last().expect("validated nonempty samples"),
    }
}

fn measure_rasters<S, Setup, Draw>(
    config: Config,
    mut setup: Setup,
    mut draw: Draw,
) -> Result<Stats, String>
where
    Setup: FnMut() -> S,
    Draw: FnMut(S) -> Raster,
{
    for _ in 0..config.warmup {
        let raster = draw(setup());
        if raster.lit_count() <= 20 {
            return Err("warmup produced an effectively blank raster".to_string());
        }
        black_box(raster);
    }

    let mut durations = Vec::with_capacity(config.samples);
    for _ in 0..config.samples {
        let state = setup();
        let started = Instant::now();
        let raster = draw(state);
        let elapsed = started.elapsed();
        if raster.lit_count() <= 20 {
            return Err("measurement produced an effectively blank raster".to_string());
        }
        black_box(raster);
        durations.push(elapsed);
    }
    Ok(summarize(&durations))
}

fn ensure_changed(name: &str, ambient: &Raster, input: &Raster) -> Result<(), String> {
    if ambient.to_rgba() == input.to_rgba() {
        Err(format!(
            "{name} input fixture does not change the production domain raster"
        ))
    } else {
        Ok(())
    }
}

fn raw_room_raster(
    room: &dyn Room,
    width: usize,
    height: usize,
    phase: f64,
    inputs: &[RoomInput],
) -> Raster {
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render_input(&mut raster, phase, inputs);
    raster
}

fn room_raster(
    room: &dyn Room,
    width: usize,
    height: usize,
    phase: f64,
    inputs: &[RoomInput],
) -> Raster {
    let mut raster = raw_room_raster(room, width, height, phase, inputs);
    input_feedback::draw(&mut raster, inputs);
    raster
}

fn room_measurements(
    config: Config,
    category: &'static str,
    flagship: &'static str,
    room_id: &str,
    phase: f64,
    inputs: &[RoomInput],
) -> Result<[Measurement; 2], String> {
    let room = room_by_id(room_id).ok_or_else(|| format!("missing flagship room: {room_id}"))?;
    let ambient_phase = effective_room_phase(room_id, phase, &[], false);
    let input_phase = effective_room_phase(room_id, phase, inputs, false);
    ensure_changed(
        flagship,
        &raw_room_raster(
            room.as_ref(),
            config.width,
            config.height,
            ambient_phase,
            &[],
        ),
        &raw_room_raster(
            room.as_ref(),
            config.width,
            config.height,
            input_phase,
            inputs,
        ),
    )?;
    let ambient = measure_rasters(
        config,
        || (),
        |_| {
            room_raster(
                room.as_ref(),
                config.width,
                config.height,
                ambient_phase,
                &[],
            )
        },
    )?;
    let input = measure_rasters(
        config,
        || (),
        |_| {
            room_raster(
                room.as_ref(),
                config.width,
                config.height,
                input_phase,
                inputs,
            )
        },
    )?;
    Ok([
        Measurement {
            category,
            flagship,
            path: PathKind::Ambient,
            stats: ambient,
        },
        Measurement {
            category,
            flagship,
            path: PathKind::InputToRaster,
            stats: input,
        },
    ])
}

fn life_measurements(config: Config) -> Result<[Measurement; 2], String> {
    let accent = room_by_id("game-of-life")
        .ok_or_else(|| "missing flagship room: game-of-life".to_string())?
        .meta()
        .accent;
    let mut baseline = LifeSession::new(17);
    for _ in 0..4 {
        baseline.advance();
    }
    let life_raster = |session: &LifeSession| {
        let mut raster = Raster::with_accent(config.width, config.height, accent);
        session.render(&mut raster);
        raster
    };
    let mut input_probe = baseline.clone();
    if !input_probe.launch((0.34, 0.62)) {
        return Err("the fixed Game of Life launch was not admitted".to_string());
    }
    ensure_changed(
        "Game of Life",
        &life_raster(&baseline),
        &life_raster(&input_probe),
    )?;
    let ambient = measure_rasters(config, || baseline.clone(), |session| life_raster(&session))?;
    let hand = [
        RoomInput::PointerDown {
            x: 0.34,
            y: 0.62,
            t: 0.4,
        },
        RoomInput::PointerUp {
            x: 0.34,
            y: 0.62,
            t: 0.4,
        },
    ];
    let input = measure_rasters(
        config,
        || baseline.clone(),
        |mut session| {
            let launched = session.launch((0.34, 0.62));
            assert!(launched, "the fixed flagship launch must be admitted");
            let mut raster = life_raster(&session);
            input_feedback::draw(&mut raster, &hand);
            raster
        },
    )?;
    Ok([
        Measurement {
            category: "Emergence",
            flagship: "Game of Life",
            path: PathKind::Ambient,
            stats: ambient,
        },
        Measurement {
            category: "Emergence",
            flagship: "Game of Life",
            path: PathKind::InputToRaster,
            stats: input,
        },
    ])
}

fn studio_measurements(config: Config) -> Result<[Measurement; 2], String> {
    let baseline = StudioPanel::default();
    let studio_raster = |panel: &StudioPanel| {
        let mut raster = Raster::with_accent(config.width, config.height, [120, 220, 190]);
        panel.draw(
            &mut raster,
            InputMode::KeyboardMouse,
            config.width,
            config.height,
            0.45,
        );
        raster
    };
    let mut input_probe = baseline.clone();
    if input_probe.load_random_recipe().is_none() {
        return Err("the fixed Formula Jam recipe did not load".to_string());
    }
    input_probe.advance_morph(studio_panel::RECIPE_MORPH_SECONDS / 2.0);
    ensure_changed(
        "Formula Jam",
        &studio_raster(&baseline),
        &studio_raster(&input_probe),
    )?;
    let ambient = measure_rasters(config, || baseline.clone(), |panel| studio_raster(&panel))?;
    let input = measure_rasters(
        config,
        || baseline.clone(),
        |mut panel| {
            assert!(
                panel.load_random_recipe().is_some(),
                "the fixed flagship recipe must load"
            );
            panel.advance_morph(studio_panel::RECIPE_MORPH_SECONDS / 2.0);
            studio_raster(&panel)
        },
    )?;
    Ok([
        Measurement {
            category: "Creation",
            flagship: "Formula Jam",
            path: PathKind::Ambient,
            stats: ambient,
        },
        Measurement {
            category: "Creation",
            flagship: "Formula Jam",
            path: PathKind::InputToRaster,
            stats: input,
        },
    ])
}

fn run_suite(config: Config) -> Result<Vec<Measurement>, String> {
    let times_table_hand = [
        RoomInput::PointerDown {
            x: 0.18,
            y: 0.52,
            t: 0.30,
        },
        RoomInput::PointerMove {
            x: 0.58,
            y: 0.52,
            t: 0.34,
        },
        RoomInput::PointerUp {
            x: 0.58,
            y: 0.52,
            t: 0.36,
        },
    ];
    let pendulum_hand = [
        RoomInput::PointerDown {
            x: 0.30,
            y: 0.24,
            t: 0.22,
        },
        RoomInput::PointerMove {
            x: 0.70,
            y: 0.67,
            t: 0.27,
        },
        RoomInput::PointerUp {
            x: 0.70,
            y: 0.67,
            t: 0.30,
        },
    ];
    let galton_hand = [RoomInput::PointerDown {
        x: 0.74,
        y: 0.28,
        t: 0.40,
    }];

    let mut measurements = Vec::with_capacity(10);
    measurements.extend(room_measurements(
        config,
        "Geometry",
        "Times Tables",
        "times-tables",
        0.36,
        &times_table_hand,
    )?);
    measurements.extend(room_measurements(
        config,
        "Chaos",
        "Double Pendulum",
        "double-pendulum",
        0.72,
        &pendulum_hand,
    )?);
    measurements.extend(life_measurements(config)?);
    measurements.extend(room_measurements(
        config,
        "Chance",
        "Galton Board",
        "galton-board",
        0.40,
        &galton_hand,
    )?);
    measurements.extend(studio_measurements(config)?);
    Ok(measurements)
}

fn report(config: Config, measurements: &[Measurement]) -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let logical_cpus = std::thread::available_parallelism().map_or(0, usize::from);
    let mut output = format!(
        "# Five Flagship Performance Baseline\n\n\
         Profile: {profile}  \n\
         Platform: {} {}  \n\
         Logical CPUs: {logical_cpus}  \n\
         Raster: {} by {}  \n\
         Samples: {} after {} warmups  \n\
         P95 budget: {:.2} ms\n\n\
         | Category | Flagship | Path | P50 ms | P95 ms | Max ms | Budget |\n\
         | --- | --- | --- | ---: | ---: | ---: | --- |\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        config.width,
        config.height,
        config.samples,
        config.warmup,
        config.budget_ms
    );
    for measurement in measurements {
        let verdict = if measurement.stats.p95_ms <= config.budget_ms {
            "PASS"
        } else {
            "FAIL"
        };
        output.push_str(&format!(
            "| {} | {} | {} | {:.3} | {:.3} | {:.3} | {} |\n",
            measurement.category,
            measurement.flagship,
            measurement.path.label(),
            measurement.stats.p50_ms,
            measurement.stats.p95_ms,
            measurement.stats.max_ms,
            verdict
        ));
    }
    output.push_str(
        "\nThe input-to-room-raster path starts when an accepted action enters its room or \
         Studio domain handler and ends when the corresponding raster is complete. It includes \
         raster allocation, room or Studio work, persistent Life mutation, and the visible input affordance where \
         applicable. It excludes native event translation and history storage, window presentation, \
         display scan-out, audio submission and callback latency, and human perception.\n",
    );
    output
}

fn check_budget(config: Config, measurements: &[Measurement]) -> Result<(), String> {
    let failures = measurements
        .iter()
        .filter(|measurement| measurement.stats.p95_ms > config.budget_ms)
        .map(|measurement| {
            format!(
                "{} {} p95 {:.3} ms",
                measurement.flagship,
                measurement.path.label(),
                measurement.stats.p95_ms
            )
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} path(s) exceeded the {:.2} ms p95 budget: {}",
            failures.len(),
            config.budget_ms,
            failures.join("; ")
        ))
    }
}

fn run() -> Result<Action, String> {
    let (action, config) = parse_args(std::env::args().skip(1))?;
    if action == Action::Help {
        print!("{HELP}");
        return Ok(action);
    }
    if config.check && cfg!(debug_assertions) {
        return Err("--check requires a --release build".to_string());
    }
    let measurements = run_suite(config)?;
    print!("{}", report(config, &measurements));
    if config.check {
        check_budget(config, &measurements)?;
    }
    Ok(action)
}

fn main() -> ExitCode {
    match run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("flagship_perf: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Config, Stats, check_budget, parse_args, run_suite, summarize};
    use std::time::Duration;

    #[test]
    fn arguments_have_stable_defaults_and_explicit_overrides() {
        assert_eq!(
            parse_args(Vec::new()).unwrap(),
            (Action::Run, Config::default())
        );
        let (_, config) = parse_args(
            [
                "--samples",
                "12",
                "--warmup",
                "2",
                "--width",
                "640",
                "--height",
                "480",
                "--budget-ms",
                "25.5",
                "--check",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();
        assert_eq!(config.samples, 12);
        assert_eq!(config.warmup, 2);
        assert_eq!((config.width, config.height), (640, 480));
        assert_eq!(config.budget_ms, 25.5);
        assert!(config.check);
    }

    #[test]
    fn arguments_reject_missing_unknown_and_hostile_values() {
        for arguments in [
            vec!["--samples"],
            vec!["--samples", "0"],
            vec!["--width", "32"],
            vec!["--budget-ms", "NaN"],
            vec![
                "--samples",
                "10000",
                "--warmup",
                "10000",
                "--width",
                "4096",
                "--height",
                "4096",
            ],
            vec!["--unknown"],
        ] {
            assert!(
                parse_args(arguments.into_iter().map(str::to_string)).is_err(),
                "arguments must fail"
            );
        }
    }

    #[test]
    fn nearest_rank_summary_is_exact() {
        let durations = (1..=20).map(Duration::from_millis).collect::<Vec<_>>();
        assert_eq!(
            summarize(&durations),
            Stats {
                p50_ms: 10.0,
                p95_ms: 19.0,
                max_ms: 20.0,
            }
        );
    }

    #[test]
    fn budget_check_names_only_over_budget_paths() {
        let config = Config {
            budget_ms: 10.0,
            samples: 1,
            warmup: 0,
            width: 160,
            height: 120,
            check: true,
        };
        let mut measurements = run_suite(config).unwrap();
        for measurement in &mut measurements {
            measurement.stats.p95_ms = 1.0;
        }
        assert!(check_budget(config, &measurements).is_ok());
        measurements[3].stats.p95_ms = 11.0;
        let error = check_budget(config, &measurements).unwrap_err();
        assert!(error.contains(measurements[3].flagship));
        assert!(!error.contains(measurements[0].flagship));
    }

    #[test]
    fn every_flagship_path_produces_a_measurement() {
        let measurements = run_suite(Config {
            samples: 1,
            warmup: 0,
            width: 160,
            height: 120,
            budget_ms: 1_000.0,
            check: false,
        })
        .unwrap();
        assert_eq!(measurements.len(), 10);
        assert_eq!(
            measurements
                .iter()
                .map(|measurement| measurement.category)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            5
        );
    }
}
