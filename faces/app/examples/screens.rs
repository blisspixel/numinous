//! Headless visual QA matrix for the windowed app.
//!
//! The matrix exercises every catalog room before and after interaction, every
//! app game state, overlays, progression, reset flow, and small viewports using
//! the same room, HUD, overlay, and game drawing modules as the live app.
//! Run: `cargo run -p numinous-app --example screens`.

use std::collections::{BTreeSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use numinous_core::{Journey, Raster, Room, RoomInput, Scoreboard, Surface, all_rooms};

#[path = "../src/audio_state.rs"]
mod audio_state;
#[allow(dead_code)]
#[path = "../src/feedback.rs"]
mod feedback;
#[allow(dead_code)]
#[path = "../src/game_draw.rs"]
mod game_draw;
#[path = "../src/hud.rs"]
mod hud;
#[allow(dead_code)]
#[path = "../src/input_legend.rs"]
mod input_legend;
#[allow(dead_code)]
#[path = "../src/overlays.rs"]
mod overlays;
#[allow(dead_code)]
#[path = "../src/play.rs"]
mod play;
#[allow(dead_code)]
#[path = "../src/studio_panel.rs"]
mod studio_panel;

const OUTPUT: &str = "renders/qa-app";
const DEFAULT_SIZE: (usize, usize) = (900, 700);
const ROOM_SIZE: (usize, usize) = DEFAULT_SIZE;
const SMALL_SIZE: (usize, usize) = (360, 240);
const DEFAULT_MIN_CHANGED_PIXELS: usize = 100;
const ABSOLUTE_MIN_CHANGED_PIXELS: usize = 32;
const MIN_CHANGED_SUPPORT_PERMILLE: usize = 10;
const MIN_SUPPORT_DENSITY_PERMILLE: usize = 1;
const SPATIAL_TILE_SIZE: usize = 32;
const MIN_COHERENT_TILES: usize = 2;
const MIN_MEAN_CHANNEL_DELTA: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InteractionKind {
    Boundary,
    Click,
    DragRelease,
    Repeated,
}

#[derive(Debug, Clone, Copy)]
enum SemanticOracle {
    ActionContains(&'static str),
    StatusChanges,
}

struct RoomScenario {
    kind: InteractionKind,
    immediate: Vec<RoomInput>,
    delayed_phase: f64,
    delayed: Vec<RoomInput>,
    semantic: SemanticOracle,
}

#[derive(Debug)]
struct Difference {
    changed: usize,
    support: usize,
    largest_tile_cluster: usize,
    mean_channel_delta: usize,
}

struct GenerationLock {
    file: Option<File>,
    path: PathBuf,
}

impl GenerationLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let create = || OpenOptions::new().create_new(true).write(true).open(path);
        let file = create().map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "another App screenshot generator owns the receipt directory; if it was terminated forcibly, remove renders/.qa-app.lock",
                )
            } else {
                error
            }
        })?;
        Ok(Self {
            file: Some(file),
            path: path.to_path_buf(),
        })
    }
}

impl Drop for GenerationLock {
    fn drop(&mut self) {
        drop(self.file.take());
        let _ = std::fs::remove_file(&self.path);
    }
}

fn save(raster: &Raster, relative: &str, manifest: &mut Vec<String>) {
    let program = if relative.contains("studio") {
        audio_state::Program::Studio
    } else {
        audio_state::Program::RoomScore
    };
    let state = audio_state::describe(program, None, 0.45, false, true, true);
    save_with_audio(raster, relative, state, manifest);
}

fn save_with_audio(
    raster: &Raster,
    relative: &str,
    state: hud::AudioState,
    manifest: &mut Vec<String>,
) {
    assert_eq!(
        (raster.width(), raster.height()),
        expected_dimensions(relative),
        "{relative} has its declared dimensions"
    );
    assert!(raster.lit_count() > 20, "{relative} is not a blank screen");
    let mut presented = raster.clone();
    hud::draw_audio_state(&mut presented, &state, raster.width());
    let path = Path::new(OUTPUT).join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create screenshot directory");
    }
    let file = File::create(&path).expect("create png");
    let mut encoder = png::Encoder::new(
        BufWriter::new(file),
        raster.width() as u32,
        raster.height() as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer
        .write_image_data(&presented.to_rgba())
        .expect("png data");
    manifest.push(relative.replace('\\', "/"));
    println!("wrote {}", path.display());
}

fn expected_dimensions(relative: &str) -> (usize, usize) {
    match relative.split('/').next() {
        Some("rooms") => {
            if relative.contains("-small-") {
                SMALL_SIZE
            } else {
                ROOM_SIZE
            }
        }
        Some("games" | "overlays" | "flows") => {
            if relative.contains("-small-") {
                SMALL_SIZE
            } else {
                DEFAULT_SIZE
            }
        }
        _ => panic!("unknown QA capture category: {relative}"),
    }
}

fn expected_paths(rooms: &[Box<dyn Room>]) -> BTreeSet<String> {
    assert_eq!(rooms.len(), 31, "current catalog size");
    let mut expected = BTreeSet::new();
    for room in rooms {
        let id = room.meta().id;
        expected.extend([
            format!("rooms/{id}-base-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-arrival-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-interacted-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-delayed-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!(
                "rooms/{id}-base-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            format!(
                "rooms/{id}-arrival-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            format!(
                "rooms/{id}-interacted-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            format!(
                "rooms/{id}-delayed-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
        ]);
    }
    for landmark in ["k2", "k3", "kpi", "k4", "k5"] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "flows/times-tables-{landmark}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        expected.insert(format!(
            "flows/times-tables-goal-{label}-{}x{}.png",
            size.0, size.1
        ));
    }
    expected.extend([
        "flows/mandelbrot-before-reset.png".to_string(),
        "flows/mandelbrot-after-reset.png".to_string(),
        "flows/game-of-life-session-opening.png".to_string(),
        "flows/game-of-life-launch-immediate.png".to_string(),
        "flows/game-of-life-generation-4.png".to_string(),
        "flows/game-of-life-generation-141.png".to_string(),
        "flows/game-of-life-after-reset.png".to_string(),
    ]);
    for name in [
        "launch-help",
        "room-inspect",
        "journey-level-42",
        "level-up-banner",
    ] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!("overlays/{name}-{label}-{}x{}.png", size.0, size.1));
        }
    }
    for name in ["cult-of-pi-journey-banner", "cult-of-pi-post-banner"] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!("overlays/{name}-{label}-{}x{}.png", size.0, size.1));
        }
    }
    for state in [
        "room-score",
        "radio",
        "radio-off",
        "muted",
        "volume-zero",
        "studio",
        "background-silent",
        "no-device",
    ] {
        expected.insert(format!(
            "overlays/audio-{state}-keyboard-default-{}x{}.png",
            DEFAULT_SIZE.0, DEFAULT_SIZE.1
        ));
        expected.insert(format!(
            "overlays/audio-{state}-controller-small-{}x{}.png",
            SMALL_SIZE.0, SMALL_SIZE.1
        ));
    }
    for phase in ["arrival", "departure"] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "overlays/the-show-{phase}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    for name in [
        "studio",
        "quiz-question",
        "quiz-correct",
        "quiz-wrong",
        "munch-play",
        "munch-result",
        "arcade-live",
        "arcade-caught",
        "arcade-clear",
        "arcade-over",
        "nim-live",
        "nim-win",
        "nim-loss",
    ] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!("games/{name}-{label}-{}x{}.png", size.0, size.1));
        }
    }
    for stage in 0..=4 {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "games/gauntlet-stage-{stage}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    expected.extend(
        [
            "rooms/controller-click-arrival-small-360x240.png",
            "rooms/controller-drag-arrival-small-360x240.png",
            "rooms/game-of-life-controller-launch-small-360x240.png",
            "overlays/controller-help-small-360x240.png",
            "overlays/keyboard-paused-small-360x240.png",
            "overlays/controller-paused-small-360x240.png",
            "overlays/controller-show-small-360x240.png",
            "overlays/controller-journey-small-360x240.png",
            "games/controller-studio-small-360x240.png",
            "games/controller-quiz-result-small-360x240.png",
            "games/controller-munch-result-small-360x240.png",
            "games/controller-arcade-over-small-360x240.png",
            "games/controller-nim-win-small-360x240.png",
            "games/controller-gauntlet-bomb-small-360x240.png",
        ]
        .into_iter()
        .map(str::to_string),
    );
    assert_eq!(expected.len(), 349, "documented QA scenario count");
    expected
}

fn assert_times_tables_spectral_palette(raster: &Raster) {
    let rgba = raster.to_rgba();
    let colors: BTreeSet<[u8; 3]> = rgba
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    for expected in [
        [50, 161, 205],
        [226, 51, 205],
        [66, 235, 147],
        [252, 159, 51],
        [126, 83, 247],
    ] {
        assert!(
            colors.contains(&expected),
            "Times Tables is missing spectral ink {expected:?}"
        );
    }

    let width = raster.width();
    let height = raster.height();
    let x = (width as f64 * 0.08).round() as usize;
    let reserve = ((height as f64 * 0.22).round() as usize).max(72);
    let y = height.saturating_sub(reserve).max(1);
    let offset = (y * width + x) * 4;
    let marker = &rgba[offset..offset + 3];
    assert!(
        marker[0] > 60 && marker[1] > 150,
        "Times Tables dial marker is not visible at {x},{y}: {marker:?}"
    );
}

fn difference(before: &Raster, after: &Raster) -> Difference {
    assert_eq!(
        (before.width(), before.height()),
        (after.width(), after.height()),
        "difference inputs have matching dimensions"
    );
    let before_rgba = before.to_rgba();
    let after_rgba = after.to_rgba();
    let width = before.width();
    let height = before.height();
    let mut changed = 0;
    let mut changed_mask = vec![false; width.saturating_mul(height)];
    let mut channel_delta = 0_usize;
    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = before.height();
    let mut max_y = 0;
    for (index, (left, right)) in before_rgba
        .chunks_exact(4)
        .zip(after_rgba.chunks_exact(4))
        .enumerate()
    {
        if left == right {
            continue;
        }
        changed += 1;
        changed_mask[index] = true;
        let x = index % width;
        let y = index / width;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        channel_delta += left[..3]
            .iter()
            .zip(&right[..3])
            .map(|(&a, &b)| usize::from(a.abs_diff(b)))
            .sum::<usize>();
    }
    Difference {
        changed,
        support: if changed == 0 {
            0
        } else {
            (max_x - min_x + 1) * (max_y - min_y + 1)
        },
        largest_tile_cluster: largest_tile_cluster(&changed_mask, width, height),
        mean_channel_delta: channel_delta / changed.max(1) / 3,
    }
}

fn largest_tile_cluster(changed: &[bool], width: usize, height: usize) -> usize {
    let tile_width = width.div_ceil(SPATIAL_TILE_SIZE);
    let tile_height = height.div_ceil(SPATIAL_TILE_SIZE);
    let mut occupied = vec![false; tile_width.saturating_mul(tile_height)];
    for (index, &is_changed) in changed.iter().enumerate() {
        if is_changed {
            let x = index % width;
            let y = index / width;
            occupied[(y / SPATIAL_TILE_SIZE) * tile_width + x / SPATIAL_TILE_SIZE] = true;
        }
    }
    let mut visited = vec![false; occupied.len()];
    let mut largest = 0;
    for start in 0..occupied.len() {
        if !occupied[start] || visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        let mut size = 0;
        while let Some(index) = queue.pop_front() {
            size += 1;
            let x = index % tile_width;
            let y = index / tile_width;
            let x_min = x.saturating_sub(1);
            let x_max = (x + 1).min(tile_width.saturating_sub(1));
            let y_min = y.saturating_sub(1);
            let y_max = (y + 1).min(tile_height.saturating_sub(1));
            for neighbor_y in y_min..=y_max {
                for neighbor_x in x_min..=x_max {
                    let neighbor = neighbor_y * tile_width + neighbor_x;
                    if occupied[neighbor] && !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        largest = largest.max(size);
    }
    largest
}

fn assert_legible(id: &str, state: &str, before: &Raster, after: &Raster) {
    let diff = difference(before, after);
    let area = before.width() * before.height();
    let default_area = ROOM_SIZE.0 * ROOM_SIZE.1;
    let minimum_changed = DEFAULT_MIN_CHANGED_PIXELS
        .saturating_mul(area)
        .div_ceil(default_area)
        .max(ABSOLUTE_MIN_CHANGED_PIXELS);
    assert!(
        diff.changed >= minimum_changed,
        "{id} {state} response changes only {} pixels, below the {minimum_changed}-pixel floor",
        diff.changed,
    );
    assert!(
        diff.support * 1_000 >= area * MIN_CHANGED_SUPPORT_PERMILLE,
        "{id} {state} response is confined to {} of {area} pixels",
        diff.support
    );
    assert!(
        diff.changed * 1_000 >= diff.support * MIN_SUPPORT_DENSITY_PERMILLE,
        "{id} {state} response scatters only {} changed pixels across {} supported pixels",
        diff.changed,
        diff.support
    );
    assert!(
        diff.largest_tile_cluster >= MIN_COHERENT_TILES,
        "{id} {state} response has no coherent cluster larger than {} spatial tile(s)",
        diff.largest_tile_cluster
    );
    assert!(
        diff.mean_channel_delta >= MIN_MEAN_CHANNEL_DELTA,
        "{id} {state} response mean channel delta {} is too faint",
        diff.mean_channel_delta
    );
}

fn assert_life_cause_is_local_and_visible(state: &str, before: &Raster, after: &Raster) {
    let diff = difference(before, after);
    let area = before.width() * before.height();
    let default_area = ROOM_SIZE.0 * ROOM_SIZE.1;
    let minimum_changed = DEFAULT_MIN_CHANGED_PIXELS
        .saturating_mul(area)
        .div_ceil(default_area)
        .max(ABSOLUTE_MIN_CHANGED_PIXELS);
    assert!(
        diff.changed >= minimum_changed,
        "Life {state} changes only {} pixels, below the {minimum_changed}-pixel floor",
        diff.changed,
    );
    assert!(
        diff.support * 100 <= area * 8,
        "Life {state} spreads across {} of {area} pixels before one glider can be followed",
        diff.support
    );
    assert!(
        diff.largest_tile_cluster >= 1,
        "Life {state} has no coherent changed tile"
    );
    assert!(
        diff.mean_channel_delta >= MIN_MEAN_CHANNEL_DELTA,
        "Life {state} is too faint"
    );
}

fn down(x: f64, y: f64, t: f64) -> RoomInput {
    RoomInput::PointerDown { x, y, t }
}

fn moved(x: f64, y: f64, t: f64) -> RoomInput {
    RoomInput::PointerMove { x, y, t }
}

fn up(x: f64, y: f64, t: f64) -> RoomInput {
    RoomInput::PointerUp { x, y, t }
}

fn click(x: f64, y: f64) -> Vec<RoomInput> {
    vec![down(x, y, 0.0), up(x, y, 0.06)]
}

fn drag(from: (f64, f64), through: (f64, f64), to: (f64, f64)) -> Vec<RoomInput> {
    vec![
        down(from.0, from.1, 0.0),
        moved(through.0, through.1, 0.08),
        moved(to.0, to.1, 0.14),
        up(to.0, to.1, 0.18),
    ]
}

fn repeated(points: &[(f64, f64)]) -> Vec<RoomInput> {
    let mut inputs = Vec::with_capacity(points.len() * 2);
    for (index, &(x, y)) in points.iter().enumerate() {
        let t = index as f64 * 0.05;
        inputs.push(down(x, y, t));
        inputs.push(up(x, y, t + 0.025));
    }
    inputs
}

fn scenario(
    kind: InteractionKind,
    immediate_at: (f64, f64),
    delayed_phase: f64,
    delayed: Vec<RoomInput>,
    semantic: SemanticOracle,
) -> RoomScenario {
    RoomScenario {
        kind,
        immediate: vec![down(immediate_at.0, immediate_at.1, 0.0)],
        delayed_phase,
        delayed,
        semantic,
    }
}

fn with_immediate(mut scenario: RoomScenario, immediate: Vec<RoomInput>) -> RoomScenario {
    scenario.immediate = immediate;
    scenario
}

fn room_scenario(id: &str) -> RoomScenario {
    use InteractionKind::{Boundary, Click, DragRelease, Repeated};
    use SemanticOracle::{ActionContains, StatusChanges};
    match id {
        "times-tables" => scenario(
            DragRelease,
            (0.18, 0.50),
            0.35,
            drag((0.18, 0.50), (0.52, 0.50), (0.88, 0.50)),
            StatusChanges,
        ),
        "cellular-automata" => scenario(
            Repeated,
            (0.22, 0.30),
            0.35,
            repeated(&[(0.22, 0.30), (0.50, 0.46), (0.78, 0.64)]),
            ActionContains("FLIP"),
        ),
        "chaos-game" => scenario(
            Boundary,
            (0.04, 0.88),
            0.40,
            repeated(&[(0.04, 0.88), (0.50, 0.04), (0.96, 0.88)]),
            ActionContains("CORNER"),
        ),
        "golden-angle" => scenario(
            Repeated,
            (0.30, 0.30),
            0.45,
            repeated(&[(0.30, 0.30), (0.64, 0.42), (0.48, 0.72)]),
            StatusChanges,
        ),
        "galton-board" => scenario(
            Repeated,
            (0.50, 0.45),
            0.35,
            repeated(&[(0.50, 0.25), (0.52, 0.50), (0.48, 0.75)]),
            StatusChanges,
        ),
        "lissajous" => scenario(
            DragRelease,
            (0.20, 0.50),
            0.40,
            drag((0.20, 0.50), (0.50, 0.50), (0.82, 0.50)),
            ActionContains("INTERVAL"),
        ),
        "prime-spirals" => scenario(
            DragRelease,
            (0.18, 0.24),
            0.50,
            drag((0.18, 0.24), (0.50, 0.50), (0.82, 0.76)),
            StatusChanges,
        ),
        "cult-of-pi" => scenario(
            Repeated,
            (0.26, 0.34),
            0.40,
            repeated(&[(0.26, 0.34), (0.54, 0.48), (0.76, 0.66)]),
            StatusChanges,
        ),
        "collatz" => scenario(
            Click,
            (0.84, 0.18),
            0.45,
            click(0.84, 0.18),
            ActionContains("PERTURB"),
        ),
        "buffon-needle" => scenario(
            Repeated,
            (0.28, 0.32),
            0.40,
            repeated(&[(0.28, 0.32), (0.52, 0.48), (0.74, 0.66)]),
            StatusChanges,
        ),
        "game-of-life" => scenario(
            Click,
            (0.50, 0.50),
            4.0 / 140.0,
            vec![down(0.50, 0.50, 0.0), up(0.50, 0.50, 0.02)],
            StatusChanges,
        ),
        "mandelbrot" => scenario(
            Boundary,
            (0.96, 0.18),
            0.50,
            click(0.96, 0.18),
            StatusChanges,
        ),
        "julia" => scenario(
            DragRelease,
            (0.20, 0.24),
            0.45,
            drag((0.20, 0.24), (0.48, 0.46), (0.78, 0.70)),
            StatusChanges,
        ),
        "barnsley-fern" => scenario(
            Repeated,
            (0.28, 0.72),
            0.50,
            repeated(&[(0.28, 0.72), (0.50, 0.56), (0.72, 0.38)]),
            StatusChanges,
        ),
        "lsystem-garden" => scenario(
            Repeated,
            (0.28, 0.70),
            0.60,
            repeated(&[(0.28, 0.70), (0.52, 0.56), (0.74, 0.38)]),
            ActionContains("PLANT"),
        ),
        "harmonograph" => scenario(
            DragRelease,
            (0.20, 0.42),
            0.45,
            drag((0.20, 0.42), (0.52, 0.54), (0.82, 0.68)),
            ActionContains("RETUNE"),
        ),
        "logistic-map" => scenario(
            DragRelease,
            (0.20, 0.60),
            0.45,
            drag((0.20, 0.60), (0.52, 0.48), (0.84, 0.36)),
            StatusChanges,
        ),
        "langtons-ant" => with_immediate(
            scenario(
                Repeated,
                (0.32, 0.36),
                0.45,
                repeated(&[(0.32, 0.36), (0.52, 0.50), (0.70, 0.64)]),
                StatusChanges,
            ),
            repeated(&[(0.28, 0.30), (0.52, 0.50), (0.74, 0.70)]),
        ),
        "lorenz" => scenario(
            Repeated,
            (0.28, 0.34),
            0.45,
            repeated(&[(0.28, 0.34), (0.52, 0.48), (0.74, 0.62)]),
            ActionContains("STORM"),
        ),
        "arecibo" => scenario(
            Boundary,
            (0.04, 0.50),
            0.40,
            drag((0.04, 0.50), (0.50, 0.50), (0.96, 0.50)),
            ActionContains("WIDTH"),
        ),
        "the-pour" => scenario(
            DragRelease,
            (0.18, 0.62),
            0.45,
            drag((0.18, 0.62), (0.50, 0.44), (0.82, 0.28)),
            ActionContains("SLOPE"),
        ),
        "slope-rider" => scenario(
            Boundary,
            (0.53, 0.47),
            0.45,
            click(0.95, 0.18),
            ActionContains("RIDER"),
        ),
        "double-pendulum" => scenario(
            DragRelease,
            (0.22, 0.28),
            0.50,
            drag((0.22, 0.28), (0.52, 0.46), (0.80, 0.72)),
            ActionContains("RE-DROP"),
        ),
        "epicycles" => scenario(
            DragRelease,
            (0.22, 0.30),
            0.45,
            drag((0.22, 0.30), (0.50, 0.48), (0.78, 0.68)),
            StatusChanges,
        ),
        "random-walk" => with_immediate(
            scenario(
                Repeated,
                (0.28, 0.32),
                0.45,
                repeated(&[(0.28, 0.32), (0.50, 0.50), (0.72, 0.68)]),
                StatusChanges,
            ),
            repeated(&[(0.24, 0.28), (0.50, 0.50), (0.76, 0.72)]),
        ),
        "voronoi" => scenario(
            Boundary,
            (0.04, 0.08),
            0.45,
            repeated(&[(0.04, 0.08), (0.50, 0.50), (0.96, 0.92)]),
            ActionContains("WELL"),
        ),
        "mobius" => scenario(
            DragRelease,
            (0.20, 0.36),
            0.45,
            drag((0.20, 0.36), (0.50, 0.50), (0.80, 0.64)),
            StatusChanges,
        ),
        "zeno" => with_immediate(
            scenario(
                DragRelease,
                (0.53, 0.47),
                0.45,
                drag((0.20, 0.34), (0.50, 0.50), (0.80, 0.66)),
                StatusChanges,
            ),
            repeated(&[(0.24, 0.28), (0.52, 0.50), (0.76, 0.72)]),
        ),
        "goldbach" => scenario(
            Boundary,
            (0.53, 0.47),
            0.55,
            click(0.96, 0.82),
            StatusChanges,
        ),
        "quine" => scenario(
            Repeated,
            (0.30, 0.32),
            0.45,
            repeated(&[(0.30, 0.32), (0.52, 0.50), (0.72, 0.68)]),
            StatusChanges,
        ),
        "strange-loop" => scenario(
            DragRelease,
            (0.22, 0.30),
            0.45,
            drag((0.22, 0.30), (0.50, 0.50), (0.78, 0.70)),
            StatusChanges,
        ),
        other => panic!("missing interaction scenario for {other}"),
    }
}

fn assert_scenario_shape(id: &str, scenario: &RoomScenario) {
    assert!(
        (0.0..=1.0).contains(&scenario.delayed_phase),
        "{id} delayed phase is normalized"
    );
    assert!(
        scenario
            .immediate
            .iter()
            .any(|input| matches!(input, RoomInput::PointerDown { .. })),
        "{id} immediate scenario touches the room"
    );
    let downs = scenario
        .delayed
        .iter()
        .filter(|input| matches!(input, RoomInput::PointerDown { .. }))
        .count();
    let moves = scenario
        .delayed
        .iter()
        .filter(|input| matches!(input, RoomInput::PointerMove { .. }))
        .count();
    let releases = scenario
        .delayed
        .iter()
        .filter(|input| matches!(input, RoomInput::PointerUp { .. }))
        .count();
    let mut previous_t = 0.0;
    for input in &scenario.delayed {
        let (x, y, t) = match *input {
            RoomInput::PointerDown { x, y, t }
            | RoomInput::PointerMove { x, y, t }
            | RoomInput::PointerUp { x, y, t } => (x, y, t),
            _ => continue,
        };
        assert!(
            x.is_finite()
                && y.is_finite()
                && t.is_finite()
                && (0.0..=1.0).contains(&x)
                && (0.0..=1.0).contains(&y),
            "{id} scenario inputs are normalized and finite"
        );
        assert!(t >= previous_t, "{id} scenario timestamps are ordered");
        assert!(
            t <= scenario.delayed_phase,
            "{id} scenario event cannot follow its capture"
        );
        previous_t = t;
    }
    let touches_boundary = scenario.delayed.iter().any(|input| {
        let (x, y) = match *input {
            RoomInput::PointerDown { x, y, .. }
            | RoomInput::PointerMove { x, y, .. }
            | RoomInput::PointerUp { x, y, .. } => (x, y),
            _ => return false,
        };
        x <= 0.05 || x >= 0.95 || y <= 0.05 || y >= 0.95
    });
    assert!(downs > 0 && releases > 0, "{id} scenario closes its input");
    match scenario.kind {
        InteractionKind::Boundary => assert!(touches_boundary, "{id} reaches a boundary"),
        InteractionKind::Click => {
            assert_eq!((downs, moves, releases), (1, 0, 1), "{id} is one click")
        }
        InteractionKind::DragRelease => {
            assert!(moves >= 2, "{id} drag samples its path");
            assert_eq!(releases, 1, "{id} drag has one release");
        }
        InteractionKind::Repeated => assert!(downs >= 3, "{id} repeats its action"),
    }
}

fn assert_semantics(room: &dyn Room, scenario: &RoomScenario) {
    let id = room.meta().id;
    let before = room.status(scenario.delayed_phase);
    let after = room.status_input(scenario.delayed_phase, &scenario.delayed);
    match scenario.semantic {
        SemanticOracle::StatusChanges => assert_ne!(
            after, before,
            "{id} status must name the interaction consequence"
        ),
        SemanticOracle::ActionContains(term) => assert!(
            numinous_core::room_touch_action(room).contains(term),
            "{id} action must explain its {term} interaction"
        ),
    }
}

fn room_by_id<'a>(rooms: &'a [Box<dyn Room>], id: &str) -> &'a dyn Room {
    rooms
        .iter()
        .find(|room| room.meta().id == id)
        .map(Box::as_ref)
        .unwrap_or_else(|| panic!("missing room {id}"))
}

fn room_screen(
    room: &dyn Room,
    t: f64,
    inputs: &[RoomInput],
    size: (usize, usize),
    room_card: u64,
    show_info: bool,
    level: u32,
) -> Raster {
    room_screen_with_mode(
        room,
        t,
        inputs,
        size,
        room_card,
        show_info,
        level,
        input_legend::InputMode::KeyboardMouse,
    )
}

#[allow(clippy::too_many_arguments)]
fn room_screen_with_mode(
    room: &dyn Room,
    t: f64,
    inputs: &[RoomInput],
    size: (usize, usize),
    room_card: u64,
    show_info: bool,
    level: u32,
    input_mode: input_legend::InputMode,
) -> Raster {
    let (width, height) = size;
    let mut raster = room_content(room, t, inputs, size);
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t,
            room_card,
            show_info,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: false,
            studio: false,
            muted: false,
            level,
            input_mode,
        },
        inputs,
        None,
        width,
        height,
    );
    raster
}

fn room_screen_with_banner(
    room: &dyn Room,
    size: (usize, usize),
    level: u32,
    lines: &[String],
) -> Raster {
    let (width, height) = size;
    let mut raster = room_content(room, 0.0, &[], size);
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t: 0.0,
            room_card: 240,
            show_info: false,
            show_help: false,
            show_journey: false,
            banner_active: true,
            the_show: false,
            studio: false,
            muted: false,
            level,
            input_mode: input_legend::InputMode::KeyboardMouse,
        },
        &[],
        None,
        width,
        height,
    );
    overlays::draw_banner(&mut raster, lines, width, height);
    raster
}

fn room_content(room: &dyn Room, t: f64, inputs: &[RoomInput], size: (usize, usize)) -> Raster {
    let mut raster = Raster::with_accent(size.0, size.1, room.meta().accent);
    room.render_input(&mut raster, t, inputs);
    raster
}

fn life_session_screen(
    room: &dyn Room,
    session: &numinous_core::rooms::game_of_life::LifeSession,
    size: (usize, usize),
    input_mode: input_legend::InputMode,
) -> Raster {
    let (width, height) = size;
    let mut raster = life_session_content(room, session, size);
    let status = if width <= 400 {
        session.compact_status()
    } else {
        session.status()
    };
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t: 0.0,
            room_card: 0,
            show_info: false,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: false,
            studio: false,
            muted: false,
            level: 7,
            input_mode,
        },
        &[],
        Some(&status),
        width,
        height,
    );
    raster
}

fn life_session_content(
    room: &dyn Room,
    session: &numinous_core::rooms::game_of_life::LifeSession,
    size: (usize, usize),
) -> Raster {
    let mut raster = Raster::with_accent(size.0, size.1, room.meta().accent);
    session.render(&mut raster);
    raster
}

fn show_screen(room: &dyn Room, t: f64, size: (usize, usize)) -> Raster {
    show_screen_with_mode(room, t, size, input_legend::InputMode::KeyboardMouse)
}

fn show_screen_with_mode(
    room: &dyn Room,
    t: f64,
    size: (usize, usize),
    input_mode: input_legend::InputMode,
) -> Raster {
    let (width, height) = size;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t,
            room_card: 0,
            show_info: false,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: true,
            studio: false,
            muted: false,
            level: 7,
            input_mode,
        },
        &[],
        None,
        width,
        height,
    );
    raster
}

fn save_sizes(
    name: &str,
    manifest: &mut Vec<String>,
    mut draw: impl FnMut(usize, usize) -> Raster,
) {
    for (label, (width, height)) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        save(
            &draw(width, height),
            &format!("games/{name}-{label}-{width}x{height}.png"),
            manifest,
        );
    }
}

fn studio_screen(width: usize, height: usize) -> Raster {
    studio_screen_with_mode(width, height, input_legend::InputMode::KeyboardMouse)
}

fn studio_screen_with_mode(
    width: usize,
    height: usize,
    input_mode: input_legend::InputMode,
) -> Raster {
    let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
    studio_panel::StudioPanel::default().draw(&mut raster, input_mode, width, height, 0.35);
    raster
}

fn gauntlet(seed: u64) -> play::GauntletPlay {
    play::GauntletPlay {
        seed,
        stage: 0,
        munch: play::MunchPlay {
            board: numinous_core::build_board(seed, 0),
            seed,
            round: 0,
            cursor: 0,
            bites: BTreeSet::new(),
            graded: None,
            bite_flash: None,
        },
        quiz: play::QuizPlay {
            round: numinous_core::build_round(seed, 1, 44, 18),
            flash: None,
        },
        scan: numinous_core::build_scan(seed, 4),
        secret: numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4),
        wire: "314".to_string(),
        wire_lines: vec!["1 LOCKED  1 LOOSE".to_string()],
        scores: vec![80, 100, 60, 50],
        cleared: vec![true, true, false, true],
        message: "STAGE REVIEW  COMBO AND CONSEQUENCES STAY VISIBLE".to_string(),
    }
}

fn main() {
    let _generation_lock = GenerationLock::acquire(Path::new("renders/.qa-app.lock"))
        .expect("another App screenshot generator is already writing renders");
    let output = Path::new(OUTPUT);
    if output.exists() {
        std::fs::remove_dir_all(output).expect("remove stale screenshot matrix");
    }
    let rooms = all_rooms();
    let mut manifest = Vec::new();
    let mut interaction_kinds = BTreeSet::new();
    let mut changed_status_oracles = 0;
    let mut explained_action_oracles = 0;

    for room in &rooms {
        let id = room.meta().id;
        let phase = 0.0;
        let scenario = room_scenario(id);
        assert_scenario_shape(id, &scenario);
        assert_semantics(room.as_ref(), &scenario);
        interaction_kinds.insert(scenario.kind);
        match scenario.semantic {
            SemanticOracle::StatusChanges => changed_status_oracles += 1,
            SemanticOracle::ActionContains(_) => explained_action_oracles += 1,
        }
        let raw_base = room_content(room.as_ref(), phase, &[], ROOM_SIZE);
        let raw_interacted = room_content(room.as_ref(), phase, &scenario.immediate, ROOM_SIZE);
        let raw_delayed_base = room_content(room.as_ref(), scenario.delayed_phase, &[], ROOM_SIZE);
        let raw_delayed = room_content(
            room.as_ref(),
            scenario.delayed_phase,
            &scenario.delayed,
            ROOM_SIZE,
        );
        let raw_small_base = room_content(room.as_ref(), phase, &[], SMALL_SIZE);
        let raw_small_interacted =
            room_content(room.as_ref(), phase, &scenario.immediate, SMALL_SIZE);
        let raw_small_delayed_base =
            room_content(room.as_ref(), scenario.delayed_phase, &[], SMALL_SIZE);
        let raw_small_delayed = room_content(
            room.as_ref(),
            scenario.delayed_phase,
            &scenario.delayed,
            SMALL_SIZE,
        );
        let base = room_screen(room.as_ref(), phase, &[], ROOM_SIZE, 0, false, 7);
        let interacted = room_screen(
            room.as_ref(),
            phase,
            &scenario.immediate,
            ROOM_SIZE,
            0,
            false,
            7,
        );
        let delayed = room_screen(
            room.as_ref(),
            scenario.delayed_phase,
            &scenario.delayed,
            ROOM_SIZE,
            0,
            false,
            7,
        );
        if id == "game-of-life" {
            assert_life_cause_is_local_and_visible("immediate", &raw_base, &raw_interacted);
            assert_life_cause_is_local_and_visible("generation 4", &raw_delayed_base, &raw_delayed);
            assert_life_cause_is_local_and_visible(
                "compact immediate",
                &raw_small_base,
                &raw_small_interacted,
            );
            assert_life_cause_is_local_and_visible(
                "compact generation 4",
                &raw_small_delayed_base,
                &raw_small_delayed,
            );
        } else {
            assert_legible(id, "immediate", &raw_base, &raw_interacted);
            assert_legible(id, "delayed", &raw_delayed_base, &raw_delayed);
            assert_legible(
                id,
                "compact immediate",
                &raw_small_base,
                &raw_small_interacted,
            );
            assert_legible(
                id,
                "compact delayed",
                &raw_small_delayed_base,
                &raw_small_delayed,
            );
        }
        save(
            &base,
            &format!("rooms/{id}-base-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &room_screen(room.as_ref(), phase, &[], ROOM_SIZE, 240, false, 7),
            &format!("rooms/{id}-arrival-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &interacted,
            &format!("rooms/{id}-interacted-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &delayed,
            &format!("rooms/{id}-delayed-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &room_screen(room.as_ref(), phase, &[], SMALL_SIZE, 0, false, 7),
            &format!(
                "rooms/{id}-base-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
        save(
            &room_screen(room.as_ref(), phase, &[], SMALL_SIZE, 240, false, 7),
            &format!(
                "rooms/{id}-arrival-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
        save(
            &room_screen(
                room.as_ref(),
                phase,
                &scenario.immediate,
                SMALL_SIZE,
                0,
                false,
                7,
            ),
            &format!(
                "rooms/{id}-interacted-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
        save(
            &room_screen(
                room.as_ref(),
                scenario.delayed_phase,
                &scenario.delayed,
                SMALL_SIZE,
                0,
                false,
                7,
            ),
            &format!(
                "rooms/{id}-delayed-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
    }
    assert_eq!(manifest.len(), rooms.len() * 8, "eight states per room");
    assert!(
        changed_status_oracles > 0 && explained_action_oracles > 0,
        "room scenarios cover changed status and explanatory action oracles"
    );
    assert_eq!(
        interaction_kinds,
        BTreeSet::from([
            InteractionKind::Boundary,
            InteractionKind::Click,
            InteractionKind::DragRelease,
            InteractionKind::Repeated,
        ]),
        "room scenarios cover every interaction family"
    );
    println!(
        "validated {} room scenarios: {changed_status_oracles} changed-status, \
         {explained_action_oracles} explanatory-action; minimum \
         {DEFAULT_MIN_CHANGED_PIXELS} default or {ABSOLUTE_MIN_CHANGED_PIXELS} compact changed \
         pixels, {MIN_CHANGED_SUPPORT_PERMILLE} permille support, \
         {MIN_SUPPORT_DENSITY_PERMILLE} permille support density, and \
         {MIN_COHERENT_TILES} adjacent spatial tiles, and \
         {MIN_MEAN_CHANNEL_DELTA} mean channel delta",
        rooms.len()
    );

    let times = room_by_id(&rooms, "times-tables");
    let landmarks = [
        ("k2", 0.0),
        ("k3", 0.125),
        ("kpi", (std::f64::consts::PI - 2.0) / 8.0),
        ("k4", 0.25),
        ("k5", 0.375),
    ];
    for (landmark, x) in landmarks {
        let inputs = [down(x, 0.5, 0.0)];
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            let raster = room_screen(times, 0.0, &inputs, size, 0, false, 7);
            if landmark == "k2" {
                assert_times_tables_spectral_palette(&raster);
            }
            save(
                &raster,
                &format!(
                    "flows/times-tables-{landmark}-{label}-{}x{}.png",
                    size.0, size.1
                ),
                &mut manifest,
            );
            if landmark == "k5" {
                let mut earned = raster;
                let banner = feedback::room_goal("LAND ON EXACTLY 4 LOBES");
                overlays::draw_banner(&mut earned, banner.lines(), size.0, size.1);
                save(
                    &earned,
                    &format!("flows/times-tables-goal-{label}-{}x{}.png", size.0, size.1),
                    &mut manifest,
                );
            }
        }
    }

    let mandelbrot = room_by_id(&rooms, "mandelbrot");
    let dive = [RoomInput::PointerDown {
        x: 0.5,
        y: 0.5,
        t: 0.6,
    }];
    save(
        &room_screen(mandelbrot, 0.6, &dive, (900, 700), 0, false, 7),
        "flows/mandelbrot-before-reset.png",
        &mut manifest,
    );
    save(
        &room_screen(mandelbrot, 0.0, &[], (900, 700), 0, false, 7),
        "flows/mandelbrot-after-reset.png",
        &mut manifest,
    );

    let life = room_by_id(&rooms, "game-of-life");
    let mut life_session = numinous_core::rooms::game_of_life::LifeSession::new(0);
    let life_opening_content = life_session_content(life, &life_session, DEFAULT_SIZE);
    let life_opening = life_session_screen(
        life,
        &life_session,
        DEFAULT_SIZE,
        input_legend::InputMode::KeyboardMouse,
    );
    save(
        &life_opening,
        "flows/game-of-life-session-opening.png",
        &mut manifest,
    );
    assert!(life_session.launch((0.5, 0.5)));
    save(
        &life_session_screen(
            life,
            &life_session,
            DEFAULT_SIZE,
            input_legend::InputMode::KeyboardMouse,
        ),
        "flows/game-of-life-launch-immediate.png",
        &mut manifest,
    );
    for _ in 0..4 {
        life_session.advance();
    }
    save(
        &life_session_screen(
            life,
            &life_session,
            DEFAULT_SIZE,
            input_legend::InputMode::KeyboardMouse,
        ),
        "flows/game-of-life-generation-4.png",
        &mut manifest,
    );
    for _ in 4..141 {
        life_session.advance();
    }
    assert_eq!(life_session.generation(), 141);
    let life_generation_141_content = life_session_content(life, &life_session, DEFAULT_SIZE);
    assert_ne!(
        life_generation_141_content.to_rgba(),
        life_opening_content.to_rgba(),
        "generation 141 room content must not wrap to the opening"
    );
    let life_generation_141 = life_session_screen(
        life,
        &life_session,
        DEFAULT_SIZE,
        input_legend::InputMode::KeyboardMouse,
    );
    save(
        &life_generation_141,
        "flows/game-of-life-generation-141.png",
        &mut manifest,
    );
    life_session = numinous_core::rooms::game_of_life::LifeSession::new(0);
    let life_after_reset = life_session_screen(
        life,
        &life_session,
        DEFAULT_SIZE,
        input_legend::InputMode::KeyboardMouse,
    );
    assert_eq!(
        life_after_reset.to_rgba(),
        life_opening.to_rgba(),
        "reset restores the exact opening for the same variation"
    );
    save(
        &life_after_reset,
        "flows/game-of-life-after-reset.png",
        &mut manifest,
    );

    let launch = room_by_id(&rooms, "times-tables");
    let golden = room_by_id(&rooms, "golden-angle");
    let galton = room_by_id(&rooms, "galton-board");
    let mut journey = Journey {
        plays: 1_000,
        wins: 37,
        secrets: 12,
        ..Default::default()
    };
    journey.visited = rooms
        .iter()
        .map(|room| room.meta().id.to_string())
        .collect();
    for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        let (width, height) = size;
        let mut help = room_screen(launch, 0.12, &[], size, 0, false, 1);
        overlays::draw_help_overlay(
            &mut help,
            width,
            height,
            None,
            input_legend::InputMode::KeyboardMouse,
            false,
        );
        save(
            &help,
            &format!("overlays/launch-help-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        save(
            &room_screen(golden, 0.0, &[], size, 0, true, 1),
            &format!("overlays/room-inspect-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        let mut journey_screen = room_screen(golden, 0.0, &[], size, 0, false, 42);
        overlays::draw_journey_overlay(
            &mut journey_screen,
            &journey,
            &Scoreboard::default(),
            rooms.len(),
            width,
            height,
            input_legend::InputMode::KeyboardMouse,
        );
        save(
            &journey_screen,
            &format!("overlays/journey-level-42-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        let mut banner = room_screen(golden, 0.0, &[], size, 0, false, 12);
        let level = feedback::level_up(12, 3);
        overlays::draw_banner(&mut banner, level.lines(), width, height);
        save(
            &banner,
            &format!("overlays/level-up-banner-{label}-{width}x{height}.png"),
            &mut manifest,
        );
    }

    let pi = room_by_id(&rooms, "cult-of-pi");
    let pi_level = feedback::level_up(12, 3);
    for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        let (width, height) = size;
        save(
            &room_screen_with_banner(pi, size, 12, pi_level.lines()),
            &format!("overlays/cult-of-pi-journey-banner-{label}-{width}x{height}.png"),
            &mut manifest,
        );
        save(
            &room_screen(pi, 0.0, &[], size, 240, false, 12),
            &format!("overlays/cult-of-pi-post-banner-{label}-{width}x{height}.png"),
            &mut manifest,
        );
    }

    let audio_states = [
        (
            "room-score",
            audio_state::Program::RoomScore,
            None,
            0.45,
            false,
            true,
            true,
            "ROOM MUSIC: VOL 45%",
        ),
        (
            "radio",
            audio_state::Program::Radio,
            Some("NUMINA FM"),
            0.45,
            false,
            true,
            true,
            "RADIO NUMINA FM: VOL 45%",
        ),
        (
            "radio-off",
            audio_state::Program::RoomScore,
            None,
            0.45,
            false,
            true,
            true,
            "ROOM MUSIC: VOL 45%",
        ),
        (
            "muted",
            audio_state::Program::RoomScore,
            None,
            0.45,
            true,
            true,
            true,
            "ROOM MUSIC: MUTED",
        ),
        (
            "volume-zero",
            audio_state::Program::RoomScore,
            None,
            0.0,
            false,
            true,
            true,
            "ROOM MUSIC: VOL 0",
        ),
        (
            "studio",
            audio_state::Program::Studio,
            None,
            0.45,
            false,
            true,
            true,
            "STUDIO: VOL 45%",
        ),
        (
            "background-silent",
            audio_state::Program::RoomScore,
            None,
            0.45,
            false,
            false,
            true,
            "ROOM MUSIC: BACKGROUND SILENT",
        ),
        (
            "no-device",
            audio_state::Program::RoomScore,
            None,
            0.45,
            false,
            true,
            false,
            "NO SOUND DEVICE",
        ),
    ];
    for (name, program, station, volume, muted, active, output, expected) in audio_states {
        let state = audio_state::describe(program, station, volume, muted, active, output);
        assert_eq!(state.label(), expected, "{name} label is semantic");
        for (mode_name, size, input_mode) in [
            (
                "keyboard-default",
                DEFAULT_SIZE,
                input_legend::InputMode::KeyboardMouse,
            ),
            (
                "controller-small",
                SMALL_SIZE,
                input_legend::InputMode::Controller,
            ),
        ] {
            let mut raster = if name == "studio" {
                studio_screen_with_mode(size.0, size.1, input_mode)
            } else {
                room_screen_with_mode(golden, 0.42, &[], size, 0, false, 7, input_mode)
            };
            if name == "radio-off" {
                let banner = feedback::radio_off();
                overlays::draw_banner(&mut raster, banner.lines(), size.0, size.1);
            }
            save_with_audio(
                &raster,
                &format!(
                    "overlays/audio-{name}-{mode_name}-{}x{}.png",
                    size.0, size.1
                ),
                state,
                &mut manifest,
            );
        }
    }

    for (phase_name, phase) in [("arrival", 0.05), ("departure", 0.95)] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            save(
                &show_screen(golden, phase, size),
                &format!(
                    "overlays/the-show-{phase_name}-{label}-{}x{}.png",
                    size.0, size.1
                ),
                &mut manifest,
            );
        }
    }

    save_sizes("studio", &mut manifest, studio_screen);

    let quiz_round = numinous_core::build_round(19, 1, 44, 18);
    let quiz_play = play::QuizPlay {
        round: quiz_round,
        flash: None,
    };
    save_sizes("quiz-question", &mut manifest, |width, height| {
        game_draw::draw_quiz(
            &rooms,
            &quiz_play,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let quiz_correct = play::QuizPlay {
        round: numinous_core::build_round(19, 1, 44, 18),
        flash: Some((true, 40)),
    };
    save_sizes("quiz-correct", &mut manifest, |width, height| {
        game_draw::draw_quiz(
            &rooms,
            &quiz_correct,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let quiz_wrong = play::QuizPlay {
        round: numinous_core::build_round(19, 1, 44, 18),
        flash: Some((false, 40)),
    };
    save_sizes("quiz-wrong", &mut manifest, |width, height| {
        game_draw::draw_quiz(
            &rooms,
            &quiz_wrong,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });

    let (munch_round, munch_board) = play::deal_munch(23, numinous_core::FULL_DECK_ROUND, None);
    let mut munch = play::MunchPlay {
        board: munch_board,
        seed: 23,
        round: munch_round,
        cursor: 17,
        bites: BTreeSet::new(),
        graded: None,
        bite_flash: None,
    };
    save_sizes("munch-play", &mut manifest, |width, height| {
        game_draw::draw_munch(
            &munch,
            20,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    munch.bites = (0..munch.board.numbers.len()).collect();
    let bites: Vec<_> = munch.bites.iter().copied().collect();
    munch.graded = Some(numinous_core::grade_munch(&munch.board, &bites));
    save_sizes("munch-result", &mut manifest, |width, height| {
        game_draw::draw_munch(
            &munch,
            20,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });

    let arcade_live = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: None,
        over: false,
    };
    save_sizes("arcade-live", &mut manifest, |width, height| {
        game_draw::draw_arcade(
            &arcade_live,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let arcade_caught = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: Some((true, 40)),
        over: false,
    };
    save_sizes("arcade-caught", &mut manifest, |width, height| {
        game_draw::draw_arcade(
            &arcade_caught,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let arcade_clear = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: Some((false, 40)),
        over: false,
    };
    save_sizes("arcade-clear", &mut manifest, |width, height| {
        game_draw::draw_arcade(
            &arcade_clear,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let arcade_over = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: None,
        over: true,
    };
    save_sizes("arcade-over", &mut manifest, |width, height| {
        game_draw::draw_arcade(
            &arcade_over,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });

    let nim = play::NimPlay {
        heaps: numinous_core::nim_new(31),
        seed: 31,
        selected: 1,
        take: 2,
        message: "THE ORDER TOOK 1 FROM HEAP 1.".to_string(),
        over: None,
    };
    save_sizes("nim-live", &mut manifest, |width, height| {
        game_draw::draw_nim(&nim, input_legend::InputMode::KeyboardMouse, width, height)
    });
    let nim_over = play::NimPlay {
        heaps: vec![0, 0, 0],
        seed: 31,
        selected: 0,
        take: 1,
        message: "THE LAST STONE IS YOURS.".to_string(),
        over: Some(true),
    };
    save_sizes("nim-win", &mut manifest, |width, height| {
        game_draw::draw_nim(
            &nim_over,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });
    let nim_loss = play::NimPlay {
        heaps: nim_over.heaps.clone(),
        seed: nim_over.seed,
        selected: nim_over.selected,
        take: nim_over.take,
        message: "THE ORDER TOOK THE LAST STONE.".to_string(),
        over: Some(false),
    };
    save_sizes("nim-loss", &mut manifest, |width, height| {
        game_draw::draw_nim(
            &nim_loss,
            input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    });

    let mut gauntlet = gauntlet(37);
    for stage in 0..=4 {
        gauntlet.stage = stage;
        save_sizes(
            &format!("gauntlet-stage-{stage}"),
            &mut manifest,
            |width, height| {
                game_draw::draw_gauntlet(
                    &rooms,
                    &gauntlet,
                    20,
                    input_legend::InputMode::KeyboardMouse,
                    width,
                    height,
                )
            },
        );
    }

    let controller = input_legend::InputMode::Controller;
    save(
        &room_screen_with_mode(galton, 0.05, &[], SMALL_SIZE, 240, false, 7, controller),
        "rooms/controller-click-arrival-small-360x240.png",
        &mut manifest,
    );
    save(
        &room_screen_with_mode(times, 0.0, &[], SMALL_SIZE, 240, false, 7, controller),
        "rooms/controller-drag-arrival-small-360x240.png",
        &mut manifest,
    );
    let mut controller_life = numinous_core::rooms::game_of_life::LifeSession::new(0);
    assert!(controller_life.launch((0.5, 0.5)));
    save(
        &life_session_screen(life, &controller_life, SMALL_SIZE, controller),
        "rooms/game-of-life-controller-launch-small-360x240.png",
        &mut manifest,
    );

    let mut controller_help =
        room_screen_with_mode(launch, 0.12, &[], SMALL_SIZE, 0, false, 1, controller);
    overlays::draw_help_overlay(
        &mut controller_help,
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        Some(5),
        controller,
        false,
    );
    save(
        &controller_help,
        "overlays/controller-help-small-360x240.png",
        &mut manifest,
    );

    let mut keyboard_paused = room_screen(golden, 0.42, &[], SMALL_SIZE, 0, false, 7);
    overlays::draw_pause_overlay(
        &mut keyboard_paused,
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        input_legend::InputMode::KeyboardMouse,
    );
    save(
        &keyboard_paused,
        "overlays/keyboard-paused-small-360x240.png",
        &mut manifest,
    );
    let mut controller_paused =
        room_screen_with_mode(golden, 0.42, &[], SMALL_SIZE, 0, false, 7, controller);
    overlays::draw_pause_overlay(
        &mut controller_paused,
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        controller,
    );
    save(
        &controller_paused,
        "overlays/controller-paused-small-360x240.png",
        &mut manifest,
    );
    save(
        &show_screen_with_mode(golden, 0.05, SMALL_SIZE, controller),
        "overlays/controller-show-small-360x240.png",
        &mut manifest,
    );

    let mut controller_journey =
        room_screen_with_mode(golden, 0.0, &[], SMALL_SIZE, 0, false, 42, controller);
    overlays::draw_journey_overlay(
        &mut controller_journey,
        &journey,
        &Scoreboard::default(),
        rooms.len(),
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        controller,
    );
    save(
        &controller_journey,
        "overlays/controller-journey-small-360x240.png",
        &mut manifest,
    );

    save(
        &studio_screen_with_mode(SMALL_SIZE.0, SMALL_SIZE.1, controller),
        "games/controller-studio-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_quiz(
            &rooms,
            &quiz_correct,
            controller,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-quiz-result-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_munch(&munch, 20, controller, SMALL_SIZE.0, SMALL_SIZE.1),
        "games/controller-munch-result-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_arcade(&arcade_over, controller, SMALL_SIZE.0, SMALL_SIZE.1),
        "games/controller-arcade-over-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_nim(&nim_over, controller, SMALL_SIZE.0, SMALL_SIZE.1),
        "games/controller-nim-win-small-360x240.png",
        &mut manifest,
    );
    gauntlet.stage = 3;
    save(
        &game_draw::draw_gauntlet(
            &rooms,
            &gauntlet,
            20,
            controller,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-gauntlet-bomb-small-360x240.png",
        &mut manifest,
    );

    manifest.sort();
    let actual: BTreeSet<_> = manifest.iter().cloned().collect();
    assert_eq!(
        actual.len(),
        manifest.len(),
        "QA scenario paths must be unique"
    );
    assert_eq!(
        actual,
        expected_paths(&rooms),
        "complete exact QA scenario inventory"
    );
    let manifest_path = PathBuf::from(OUTPUT).join("MANIFEST.txt");
    std::fs::write(
        &manifest_path,
        format!("{} screenshots\n{}\n", manifest.len(), manifest.join("\n")),
    )
    .expect("write manifest");
    println!("wrote {}", manifest_path.display());
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_MIN_CHANGED_PIXELS, GenerationLock, MIN_CHANGED_SUPPORT_PERMILLE,
        MIN_COHERENT_TILES, MIN_MEAN_CHANNEL_DELTA, MIN_SUPPORT_DENSITY_PERMILLE, ROOM_SIZE,
        difference,
    };
    use numinous_core::{Raster, Surface};

    #[test]
    fn screenshot_generation_has_one_cross_process_writer() {
        let path = std::env::temp_dir().join(format!(
            "numinous-screenshot-generation-{}.lock",
            std::process::id()
        ));
        let first = GenerationLock::acquire(&path).expect("first writer owns the lock");
        let output =
            std::process::Command::new(std::env::current_exe().expect("current test binary"))
                .args([
                    "--exact",
                    "tests::screenshot_generation_lock_probe",
                    "--nocapture",
                ])
                .env("NUMINOUS_SCREEN_LOCK_PROBE", &path)
                .output()
                .expect("run competing writer probe");
        assert!(
            output.status.success(),
            "competing process probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        drop(first);
        let second = GenerationLock::acquire(&path).expect("lock releases with its process handle");
        drop(second);
        assert!(!path.exists(), "the final owner removes the lock file");

        std::fs::write(&path, "terminated owner").expect("seed stale lock file");
        let stale_error = GenerationLock::acquire(&path)
            .err()
            .expect("stale lock fails closed");
        assert_eq!(stale_error.kind(), std::io::ErrorKind::WouldBlock);
        assert!(
            stale_error
                .to_string()
                .contains("remove renders/.qa-app.lock"),
            "manual recovery names the exact lock file"
        );
        std::fs::remove_file(&path).expect("operator removes confirmed stale lock");
        let recovered = GenerationLock::acquire(&path).expect("generation resumes after recovery");
        drop(recovered);
        assert!(!path.exists(), "recovered owner cleans up normally");
    }

    #[test]
    fn screenshot_generation_lock_probe() {
        let Ok(path) = std::env::var("NUMINOUS_SCREEN_LOCK_PROBE") else {
            return;
        };
        let error = GenerationLock::acquire(std::path::Path::new(&path))
            .err()
            .expect("the parent process owns the lock");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn scattered_corner_markers_do_not_satisfy_the_spatial_oracle() {
        let before = Raster::with_accent(ROOM_SIZE.0, ROOM_SIZE.1, [255, 255, 255]);
        let mut after = Raster::with_accent(ROOM_SIZE.0, ROOM_SIZE.1, [255, 255, 255]);
        for (left, top) in [(0, 0), (630, 0), (0, 470), (630, 470)] {
            for y in top..top + 10 {
                for x in left..left + 10 {
                    after.plot(x, y, '#');
                }
            }
        }

        let diff = difference(&before, &after);
        let area = ROOM_SIZE.0 * ROOM_SIZE.1;
        assert!(diff.changed >= DEFAULT_MIN_CHANGED_PIXELS);
        assert!(diff.support * 1_000 >= area * MIN_CHANGED_SUPPORT_PERMILLE);
        assert!(diff.changed * 1_000 >= diff.support * MIN_SUPPORT_DENSITY_PERMILLE);
        assert!(diff.mean_channel_delta >= MIN_MEAN_CHANNEL_DELTA);
        assert_eq!(diff.largest_tile_cluster, 1);
        assert!(diff.largest_tile_cluster < MIN_COHERENT_TILES);
    }
}
