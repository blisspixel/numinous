//! Headless visual QA matrix for the windowed app.
//!
//! The matrix exercises every catalog room before and after interaction, every
//! app game state, overlays, progression, reset flow, and small viewports using
//! the same room, HUD, overlay, and game drawing modules as the live app.
//! Run: `cargo run -p numinous-app --example screens`.
//!
//! The public README gallery is a separate, small set of those same frames:
//! `cargo run -p numinous-app --example screens -- --readme` writes
//! `assets/screens/`.
//!
//! `--room <id>` inspects one room in `renders/qa-room/<id>/` without replacing
//! the full matrix. Its frames use the same composed drawing and domain checks.

use std::collections::{BTreeSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use numinous_app::route_authoring::{
    Action as RouteAction, Page as RoutePage, Panel as RoutePanel,
};
use numinous_core::{Journey, Raster, Room, RoomInput, Scoreboard, Surface, all_rooms};

fn draw_cabinet_menu(raster: &mut Raster, mode: numinous_app::input_legend::InputMode) {
    let state = numinous_app::menu::MenuState::launch();
    draw_cabinet_menu_state(raster, &state, mode);
}

fn draw_cabinet_menu_state(
    raster: &mut Raster,
    state: &numinous_app::menu::MenuState,
    mode: numinous_app::input_legend::InputMode,
) {
    draw_cabinet_menu_state_with_display(raster, state, mode, false);
}

fn draw_cabinet_menu_state_with_display(
    raster: &mut Raster,
    state: &numinous_app::menu::MenuState,
    mode: numinous_app::input_legend::InputMode,
    fullscreen: bool,
) {
    let audio = audio_state::describe(
        audio_state::Program::RoomScore,
        None,
        0.45,
        audio_state::SourceLevels::default(),
        false,
        true,
        true,
    );
    hud::draw_audio_state(raster, &audio, raster.width());
    let _ = numinous_app::menu::draw_menu(
        raster,
        state,
        mode,
        numinous_app::input_legend::ControllerFace::Generic.into(),
        numinous_app::menu::MenuReadout {
            volume_percent: 45,
            music_percent: 100,
            room_percent: 100,
            effect_percent: 100,
            muted: false,
            era: "phosphor",
            window_mode: if fullscreen { "borderless" } else { "windowed" },
            fullscreen,
        },
    );
}

#[path = "../src/audio_state.rs"]
mod audio_state;
#[allow(dead_code)]
#[path = "../src/feedback.rs"]
mod feedback;
#[allow(dead_code)]
#[path = "../src/gallery.rs"]
mod gallery;
#[allow(dead_code)]
#[path = "../src/game_draw.rs"]
mod game_draw;
#[path = "../src/hud.rs"]
mod hud;
#[path = "../src/input_feedback.rs"]
mod input_feedback;
#[allow(dead_code)]
#[path = "../src/input_legend.rs"]
mod input_legend;
#[allow(dead_code)]
#[path = "../src/nim_render.rs"]
mod nim_render;
#[allow(dead_code)]
#[path = "../src/overlays.rs"]
mod overlays;
#[allow(dead_code)]
#[path = "../src/play.rs"]
mod play;
// The core's census of current documents, for the screen-matrix count only
// this example can state.
#[cfg(test)]
#[path = "../../../crates/core/src/prose_census.rs"]
mod prose_census;
#[allow(dead_code)]
#[path = "../src/studio_panel.rs"]
mod studio_panel;

const OUTPUT: &str = "renders/qa-app";
const README_SCREENS: &str = "assets/screens";
const DEFAULT_SIZE: (usize, usize) = (900, 700);
const README_PLATES: [&str; 9] = [
    "menu.png",
    "times-tables.png",
    "golden-angle.png",
    "mandelbrot.png",
    "double-pendulum.png",
    "studio.png",
    "kepler-laws.png",
    "lissajous.png",
    "route-lab.png",
];
const FULLSCREEN_SIZE: (usize, usize) = (1920, 1080);
const ROOM_SIZE: (usize, usize) = DEFAULT_SIZE;
const SMALL_SIZE: (usize, usize) = (360, 240);
const DEFAULT_MIN_CHANGED_PIXELS: usize = 100;
const ABSOLUTE_MIN_CHANGED_PIXELS: usize = 32;
const DEFAULT_MIN_DOMAIN_CHANGED_PIXELS: usize = 8;
const ABSOLUTE_MIN_DOMAIN_CHANGED_PIXELS: usize = 4;
const MIN_CHANGED_SUPPORT_PERMILLE: usize = 10;
const MIN_SUPPORT_DENSITY_PERMILLE: usize = 1;
const SPATIAL_TILE_SIZE: usize = 32;
const MIN_COHERENT_TILES: usize = 2;
const MIN_MEAN_CHANNEL_DELTA: usize = 4;
const ROUTE_AUTHORING_STATES: [&str; 27] = [
    "opening",
    "custom-network",
    "dense-network",
    "new-road-draft",
    "disconnected",
    "search-opening",
    "search-prefix",
    "search-improved",
    "search-rewind",
    "search-complete",
    "search-disconnected",
    "search-dense",
    "random-options",
    "random-map",
    "random-search-prefix",
    "random-search-complete",
    "random-pick-end",
    "keep-question",
    "shared-question",
    "remix-preview",
    "received-question",
    "received-kept",
    "received-dense",
    "received-long-question",
    "received-unicode-question",
    "received-unicode-last-page",
    "keep-unicode-question",
];
const ROUTE_GALLERY_STATES: [&str; 2] = ["question", "studio"];
const SHARED_SCREEN_COUNT: usize =
    105 + ROUTE_AUTHORING_STATES.len() * 4 + ROUTE_GALLERY_STATES.len() * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InteractionKind {
    Boundary,
    Click,
    DragRelease,
    Held,
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
    let state = audio_state::describe(
        program,
        None,
        0.45,
        audio_state::SourceLevels::default(),
        false,
        true,
        true,
    );
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
    if !relative.starts_with("menu/") && !relative.starts_with("overlays/launch-help") {
        hud::draw_audio_state(&mut presented, &state, raster.width());
    }
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
        Some("games" | "overlays" | "flows" | "menu") => {
            if relative.contains("-fullscreen-") {
                FULLSCREEN_SIZE
            } else if relative.contains("-small-") {
                SMALL_SIZE
            } else {
                DEFAULT_SIZE
            }
        }
        _ => panic!("unknown QA capture category: {relative}"),
    }
}

fn expected_paths(rooms: &[Box<dyn Room>]) -> BTreeSet<String> {
    assert_eq!(
        rooms.len(),
        numinous_core::ROOM_CATALOG.len(),
        "the QA matrix must cover the complete canonical catalog"
    );
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
        "watch-agent",
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
        "studio-morph",
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
    for state in ROUTE_AUTHORING_STATES {
        for mode in ["keyboard", "controller"] {
            for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
                expected.insert(format!(
                    "overlays/route-editor-{state}-{mode}-{label}-{}x{}.png",
                    size.0, size.1
                ));
            }
        }
    }
    for state in ROUTE_GALLERY_STATES {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "overlays/route-gallery-{state}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    assert_eq!(
        expected.len(),
        rooms.len() * 8 + SHARED_SCREEN_COUNT,
        "eight states per room plus the shared QA inventory"
    );
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

/// Art-first: pointer feedback must never ink the room. The domain response
/// oracle still checks that interaction moves the math; this only guards that
/// the overlay layer stays empty.
fn feedback_chrome_error(id: &str, state: &str, before: &Raster, after: &Raster) -> Option<String> {
    let diff = difference(before, after);
    if diff.changed != 0 {
        return Some(format!(
            "{id} {state} paints {} chrome pixels over the art; feedback must be empty",
            diff.changed
        ));
    }
    None
}

fn assert_feedback_chrome(id: &str, state: &str, before: &Raster, after: &Raster) {
    if let Some(message) = feedback_chrome_error(id, state, before, after) {
        panic!("{message}");
    }
}

fn domain_response_error(
    id: &str,
    viewport: &str,
    immediate_base: &Raster,
    immediate: &Raster,
    delayed_base: &Raster,
    delayed: &Raster,
) -> Option<String> {
    let immediate = difference(immediate_base, immediate);
    let delayed = difference(delayed_base, delayed);
    let strongest = if immediate.changed >= delayed.changed {
        &immediate
    } else {
        &delayed
    };
    let area = immediate_base.width() * immediate_base.height();
    let default_area = ROOM_SIZE.0 * ROOM_SIZE.1;
    let minimum_changed = DEFAULT_MIN_DOMAIN_CHANGED_PIXELS
        .saturating_mul(area)
        .div_ceil(default_area)
        .max(ABSOLUTE_MIN_DOMAIN_CHANGED_PIXELS);
    if strongest.changed < minimum_changed {
        return Some(format!(
            "{id} {viewport} room renderer changes only {} pixels, below the {minimum_changed}-pixel domain floor",
            strongest.changed
        ));
    }
    if strongest.mean_channel_delta < MIN_MEAN_CHANNEL_DELTA {
        return Some(format!(
            "{id} {viewport} room renderer mean channel delta {} is too faint",
            strongest.mean_channel_delta
        ));
    }
    None
}

fn assert_domain_response(
    id: &str,
    viewport: &str,
    immediate_base: &Raster,
    immediate: &Raster,
    delayed_base: &Raster,
    delayed: &Raster,
) {
    if let Some(message) = domain_response_error(
        id,
        viewport,
        immediate_base,
        immediate,
        delayed_base,
        delayed,
    ) {
        panic!("{message}");
    }
}

fn life_cause_error(state: &str, before: &Raster, after: &Raster) -> Option<String> {
    let diff = difference(before, after);
    let area = before.width() * before.height();
    let default_area = ROOM_SIZE.0 * ROOM_SIZE.1;
    let minimum_changed = DEFAULT_MIN_CHANGED_PIXELS
        .saturating_mul(area)
        .div_ceil(default_area)
        .max(ABSOLUTE_MIN_CHANGED_PIXELS);
    if diff.changed < minimum_changed {
        return Some(format!(
            "Life {state} changes only {} pixels, below the {minimum_changed}-pixel floor",
            diff.changed
        ));
    }
    if diff.support * 100 > area * 8 {
        return Some(format!(
            "Life {state} spreads across {} of {area} pixels before one glider can be followed",
            diff.support
        ));
    }
    if diff.largest_tile_cluster == 0 {
        return Some(format!("Life {state} has no coherent changed tile"));
    }
    if diff.mean_channel_delta < MIN_MEAN_CHANNEL_DELTA {
        return Some(format!("Life {state} is too faint"));
    }
    None
}

fn assert_life_cause_is_local_and_visible(state: &str, before: &Raster, after: &Raster) {
    if let Some(message) = life_cause_error(state, before, after) {
        panic!("{message}");
    }
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

fn scenario_for_verb(room: &dyn Room) -> RoomScenario {
    use InteractionKind::{Click, DragRelease, Held};
    let action = room.verb().unwrap_or("DRAG: SCRUB TIME");
    if action.starts_with("CLICK") {
        scenario(
            Click,
            (0.82, 0.80),
            0.06,
            click(0.82, 0.80),
            SemanticOracle::StatusChanges,
        )
    } else if action.starts_with("HOLD") {
        scenario(
            Held,
            (0.82, 0.80),
            0.40,
            vec![down(0.82, 0.80, 0.0)],
            SemanticOracle::StatusChanges,
        )
    } else if action.starts_with("DRAG") || action.starts_with("PULL") || action.starts_with("AIM")
    {
        scenario(
            DragRelease,
            (0.18, 0.20),
            0.55,
            drag((0.18, 0.20), (0.50, 0.50), (0.82, 0.80)),
            SemanticOracle::StatusChanges,
        )
    } else {
        panic!(
            "{} has unsupported interaction prefix {prefix:?}",
            room.meta().id,
            prefix = action.split(':').next().unwrap_or_default()
        )
    }
}

fn room_scenario(room: &dyn Room) -> RoomScenario {
    use InteractionKind::{Boundary, Click, DragRelease, Repeated};
    use SemanticOracle::{ActionContains, StatusChanges};
    let id = room.meta().id;
    match id {
        "times-tables" => scenario(
            DragRelease,
            (0.18, 0.50),
            0.35,
            drag((0.18, 0.50), (0.52, 0.50), (0.88, 0.50)),
            StatusChanges,
        ),
        "laplace-clock" => scenario(
            DragRelease,
            (0.02, 0.50),
            0.85,
            drag((0.02, 0.50), (0.50, 0.50), (0.98, 0.50)),
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
            (0.50, 0.50),
            0.40,
            repeated(&[(0.04, 0.88), (0.96, 0.88), (0.50, 0.50)]),
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
            Click,
            (0.82, 0.50),
            0.40,
            click(0.82, 0.50),
            ActionContains("INTERVAL"),
        ),
        "prime-spirals" => scenario(Click, (0.82, 0.76), 0.50, click(0.82, 0.76), StatusChanges),
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
        "julia" => scenario(Click, (0.78, 0.70), 0.45, click(0.78, 0.70), StatusChanges),
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
            Click,
            (0.82, 0.68),
            0.45,
            click(0.82, 0.68),
            ActionContains("RETUNE"),
        ),
        "logistic-map" => scenario(Click, (0.84, 0.36), 0.45, click(0.84, 0.36), StatusChanges),
        "langtons-ant" => scenario(
            Repeated,
            (0.32, 0.36),
            0.45,
            repeated(&[(0.32, 0.36), (0.52, 0.50), (0.70, 0.64)]),
            StatusChanges,
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
            click(0.96, 0.50),
            ActionContains("WIDTH"),
        ),
        "the-pour" => scenario(
            Click,
            (0.82, 0.28),
            0.45,
            click(0.82, 0.28),
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
            Click,
            (0.80, 0.72),
            0.50,
            click(0.80, 0.72),
            ActionContains("RE-DROP"),
        ),
        "epicycles" => scenario(Click, (0.78, 0.68), 0.45, click(0.78, 0.68), StatusChanges),
        "random-walk" => scenario(
            Repeated,
            (0.28, 0.32),
            0.45,
            repeated(&[(0.28, 0.32), (0.50, 0.50), (0.72, 0.68)]),
            StatusChanges,
        ),
        "voronoi" => scenario(
            Boundary,
            (0.50, 0.50),
            0.45,
            repeated(&[(0.04, 0.08), (0.96, 0.92), (0.50, 0.50)]),
            ActionContains("WELL"),
        ),
        "mobius" => scenario(Click, (0.80, 0.64), 0.45, click(0.80, 0.64), StatusChanges),
        "zeno" => scenario(Click, (0.80, 0.66), 0.45, click(0.80, 0.66), StatusChanges),
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
        "strange-loop" => scenario(Click, (0.78, 0.70), 0.45, click(0.78, 0.70), StatusChanges),
        _ => scenario_for_verb(room),
    }
}

fn assert_scenario_shape(id: &str, scenario: &RoomScenario) {
    assert!(
        (0.0..=1.0).contains(&scenario.delayed_phase),
        "{id} delayed phase is normalized"
    );
    let [RoomInput::PointerDown { x, y, t }] = scenario.immediate.as_slice() else {
        panic!("{id} immediate scenario must be exactly one pointer down");
    };
    assert!(
        x.is_finite()
            && y.is_finite()
            && t.is_finite()
            && (0.0..=1.0).contains(x)
            && (0.0..=1.0).contains(y)
            && *t == 0.0,
        "{id} immediate pointer down is normalized at capture phase zero"
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
    assert!(downs > 0, "{id} scenario starts its input");
    if scenario.kind != InteractionKind::Held {
        assert!(releases > 0, "{id} completed scenario closes its input");
    }
    match scenario.kind {
        InteractionKind::Boundary => assert!(touches_boundary, "{id} reaches a boundary"),
        InteractionKind::Click => {
            assert_eq!((downs, moves, releases), (1, 0, 1), "{id} is one click")
        }
        InteractionKind::DragRelease => {
            assert!(moves >= 2, "{id} drag samples its path");
            assert_eq!(releases, 1, "{id} drag has one release");
        }
        InteractionKind::Held => assert_eq!(
            (downs, moves, releases),
            (1, 0, 0),
            "{id} held capture has one active press"
        ),
        InteractionKind::Repeated => assert!(downs >= 3, "{id} repeats its action"),
    }
}

fn assert_scenario_matches_verb(room: &dyn Room, scenario: &RoomScenario) {
    use InteractionKind::{Boundary, Click, DragRelease, Held, Repeated};
    let id = room.meta().id;
    let action = room.verb().unwrap_or("DRAG: SCRUB TIME");
    let matches_click =
        action.contains("CLICK") && matches!(scenario.kind, Click | Repeated | Boundary);
    if matches_click {
        assert!(
            !scenario
                .delayed
                .iter()
                .any(|input| matches!(input, RoomInput::PointerMove { .. })),
            "{id} declared click scenario cannot synthesize a drag"
        );
    }
    let matches = matches_click
        || (action.starts_with("DRAG") && matches!(scenario.kind, DragRelease | Boundary))
        || (action.starts_with("HOLD") && scenario.kind == Held)
        || ((action.starts_with("PULL") || action.starts_with("AIM"))
            && scenario.kind == DragRelease);
    assert!(
        matches,
        "{id} scenario {:?} must exercise declared action {action}",
        scenario.kind
    );
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

fn assert_hold_release_contract(room: &dyn Room, scenario: &RoomScenario) {
    if scenario.kind != InteractionKind::Held {
        return;
    }
    let [RoomInput::PointerDown { x, y, .. }] = scenario.immediate.as_slice() else {
        unreachable!("scenario shape already proves one pointer down");
    };
    let held = [down(*x, *y, 0.0)];
    let released = [down(*x, *y, 0.0), up(*x, *y, 0.1)];
    let cancelled = [down(*x, *y, 0.0), RoomInput::PointerCancel];
    let phase = scenario.delayed_phase;
    let base = room_content(room, phase, &[], SMALL_SIZE);
    let active = room_content(room, phase, &held, SMALL_SIZE);
    let after_release = room_content(room, phase, &released, SMALL_SIZE);
    let after_cancel = room_content(room, phase, &cancelled, SMALL_SIZE);
    assert!(
        domain_response_error(
            room.meta().id,
            "held compact",
            &base,
            &active,
            &base,
            &active
        )
        .is_none(),
        "{} active hold must have a perceptible domain consequence",
        room.meta().id
    );
    assert_eq!(
        after_release.to_rgba(),
        base.to_rgba(),
        "{} release must end its hold effect",
        room.meta().id
    );
    assert_eq!(
        after_cancel.to_rgba(),
        base.to_rgba(),
        "{} cancel must end its hold effect",
        room.meta().id
    );
    let base_status = room.status(phase);
    assert_ne!(
        room.status_input(phase, &held),
        base_status,
        "{} active hold status must name its consequence",
        room.meta().id
    );
    assert_eq!(
        room.status_input(phase, &released),
        base_status,
        "{} released status must return to ambient",
        room.meta().id
    );
    assert_eq!(
        room.status_input(phase, &cancelled),
        base_status,
        "{} cancelled status must return to ambient",
        room.meta().id
    );
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
    let mut raster = room_content_with_feedback(room, t, inputs, size);
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
            controller_face: input_legend::ControllerFace::Generic.into(),
            motion: numinous_core::Motion::Full,
        },
        inputs,
        None,
        width,
        height,
    );
    // Offline visualizer meter keeps the screens matrix honest about spectrum chrome.
    let bands = [0.15, 0.35, 0.7, 0.45, 0.25, 0.1, 0.05];
    hud::draw_spectrum_meter(&mut raster, &bands, width, height);
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
            controller_face: input_legend::ControllerFace::Generic.into(),
            motion: numinous_core::Motion::Full,
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

fn room_content_with_feedback(
    room: &dyn Room,
    t: f64,
    inputs: &[RoomInput],
    size: (usize, usize),
) -> Raster {
    apply_input_feedback(room_content(room, t, inputs, size), inputs)
}

fn apply_input_feedback(mut raster: Raster, inputs: &[RoomInput]) -> Raster {
    input_feedback::draw(&mut raster, inputs);
    raster
}

/// Feedback chrome only draws while the hand is held. For matrix checks that
/// still need a visible reticle, drop a terminal up/cancel so the latest
/// gesture stays open without inventing a new hand point.
fn feedback_hold_inputs(inputs: &[RoomInput]) -> Vec<RoomInput> {
    match inputs.last() {
        Some(RoomInput::PointerUp { .. } | RoomInput::PointerCancel) if inputs.len() > 1 => {
            inputs[..inputs.len() - 1].to_vec()
        }
        _ => inputs.to_vec(),
    }
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
            controller_face: input_legend::ControllerFace::Generic.into(),
            motion: numinous_core::Motion::Full,
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
            controller_face: input_legend::ControllerFace::Generic.into(),
            motion: numinous_core::Motion::Full,
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
    studio_panel::StudioPanel::default().draw_with_controller(
        &mut raster,
        input_mode,
        input_legend::ControllerFace::Generic.into(),
        width,
        height,
    );
    raster
}

fn studio_morph_screen(width: usize, height: usize) -> Raster {
    let mut panel = studio_panel::StudioPanel::default();
    panel.toggle_help();
    assert!(panel.load_random_recipe().is_some());
    panel.advance_morph(studio_panel::RECIPE_MORPH_SECONDS / 2.0);
    let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
    panel.draw_with_controller(
        &mut raster,
        input_legend::InputMode::KeyboardMouse,
        input_legend::ControllerFace::Generic.into(),
        width,
        height,
    );
    raster
}

fn write_png(raster: &Raster, path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create screenshot directory");
    }
    let file = File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(
        BufWriter::new(file),
        raster.width() as u32,
        raster.height() as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer
        .write_image_data(&raster.to_rgba())
        .expect("png data");
    println!("wrote {}", path.display());
}

fn readme_room_screen(room: &dyn Room, t: f64, inputs: &[RoomInput]) -> Raster {
    let (width, height) = DEFAULT_SIZE;
    let mut raster = room_content(room, t, inputs, DEFAULT_SIZE);
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
            the_show: false,
            studio: false,
            muted: false,
            level: 1,
            input_mode: input_legend::InputMode::KeyboardMouse,
            controller_face: input_legend::ControllerFace::Generic.into(),
            motion: numinous_core::Motion::Full,
        },
        inputs,
        None,
        width,
        height,
    );
    raster
}

fn present_readme_plate(raster: &Raster, name: &str) -> Raster {
    assert_eq!(
        (raster.width(), raster.height()),
        DEFAULT_SIZE,
        "{name} uses the public App window size"
    );
    assert!(raster.lit_count() > 20, "{name} is not a blank screen");
    let mut presented = raster.clone();
    if name != "menu.png" {
        let program = if name == "studio.png" {
            audio_state::Program::Studio
        } else {
            audio_state::Program::RoomScore
        };
        let state = audio_state::describe(
            program,
            None,
            0.45,
            audio_state::SourceLevels::default(),
            false,
            true,
            true,
        );
        hud::draw_audio_state(&mut presented, &state, raster.width());
    }
    presented
}

fn write_readme_screens(output: &Path) {
    std::fs::create_dir_all(output).expect("create README screenshot directory");
    if let Ok(entries) = std::fs::read_dir(output) {
        for entry in entries {
            let path = entry.expect("readme screen directory entry").path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.extension().is_some_and(|ext| ext == "png") && !README_PLATES.contains(&name) {
                std::fs::remove_file(&path)
                    .unwrap_or_else(|error| panic!("remove stale {name}: {error}"));
            }
        }
    }

    let rooms = all_rooms();
    let times = room_by_id(&rooms, "times-tables");
    let mut menu = Raster::with_accent(DEFAULT_SIZE.0, DEFAULT_SIZE.1, times.meta().accent);
    let mut state = numinous_app::menu::MenuState::launch();
    state.set_experiment_available(numinous_core::is_engineered_aha_room(times.meta().id));
    draw_cabinet_menu_state(
        &mut menu,
        &state,
        numinous_app::input_legend::InputMode::KeyboardMouse,
    );
    write_png(
        &present_readme_plate(&menu, "menu.png"),
        &output.join("menu.png"),
    );

    write_png(
        &present_readme_plate(&readme_room_screen(times, 0.0, &[]), "times-tables.png"),
        &output.join("times-tables.png"),
    );

    let golden = room_by_id(&rooms, "golden-angle");
    write_png(
        &present_readme_plate(&readme_room_screen(golden, 0.28, &[]), "golden-angle.png"),
        &output.join("golden-angle.png"),
    );

    let mandelbrot = room_by_id(&rooms, "mandelbrot");
    write_png(
        &present_readme_plate(&readme_room_screen(mandelbrot, 0.0, &[]), "mandelbrot.png"),
        &output.join("mandelbrot.png"),
    );

    let pendulum = room_by_id(&rooms, "double-pendulum");
    write_png(
        &present_readme_plate(
            &readme_room_screen(pendulum, 0.86, &[]),
            "double-pendulum.png",
        ),
        &output.join("double-pendulum.png"),
    );

    for (room_id, phase, inputs) in [
        ("kepler-laws", 0.6, Vec::new()),
        (
            "lissajous",
            0.15,
            vec![RoomInput::PointerDown {
                x: 0.5625,
                y: 0.1875,
                t: 0.15,
            }],
        ),
    ] {
        let name = format!("{room_id}.png");
        let room = room_by_id(&rooms, room_id);
        write_png(
            &present_readme_plate(&readme_room_screen(room, phase, &inputs), &name),
            &output.join(name),
        );
    }

    let mut panel = studio_panel::StudioPanel::new("x(t)=a*cos(3*t); y(t)=sin(2*t)")
        .expect("parametric Formula Jam source");
    panel.toggle_help();
    let mut studio = Raster::with_accent(DEFAULT_SIZE.0, DEFAULT_SIZE.1, [120, 220, 190]);
    panel.draw_with_controller(
        &mut studio,
        input_legend::InputMode::KeyboardMouse,
        input_legend::ControllerFace::Generic.into(),
        DEFAULT_SIZE.0,
        DEFAULT_SIZE.1,
    );
    write_png(
        &present_readme_plate(&studio, "studio.png"),
        &output.join("studio.png"),
    );
    let search = route_authoring_panels()
        .into_iter()
        .find(|(state, _)| *state == "search-improved")
        .expect("authored search improvement fixture")
        .1;
    let route = search.draw(DEFAULT_SIZE.0, DEFAULT_SIZE.1, None);
    write_png(
        &present_readme_plate(&route, "route-lab.png"),
        &output.join("route-lab.png"),
    );
}

fn gauntlet(seed: u64) -> play::GauntletPlay {
    let puzzle = numinous_core::GauntletPuzzle::new(seed);
    let secret = puzzle.bomb_code().to_vec();
    play::GauntletPlay {
        seed,
        stage: 0,
        munch: play::MunchPlay {
            board: puzzle.munch,
            seed,
            round: 0,
            cursor: 0,
            bites: BTreeSet::new(),
            graded: None,
            bite_flash: None,
        },
        quiz: play::QuizPlay {
            round: puzzle.shape,
            flash: None,
        },
        scan: puzzle.sky,
        secret,
        wire: "314".to_string(),
        wire_lines: vec!["1 LOCKED  1 LOOSE".to_string()],
        scores: vec![80, 100, 60, 50],
        cleared: vec![true, true, false, true],
        message: "STAGE REVIEW  COMBO AND CONSEQUENCES STAY VISIBLE".to_string(),
    }
}

fn authored_route_workbench() -> numinous_core::route_workbench::RouteWorkbench {
    use numinous_core::route::Road;
    use numinous_core::route_workbench::{
        EditableRoad, RouteTownSnapshot, RouteWorkbench, RouteWorkbenchSnapshot,
    };
    RouteWorkbench::from_snapshot(RouteWorkbenchSnapshot {
        revision: 0,
        current: RouteTownSnapshot {
            junctions: 6,
            roads: [
                (0, 1, 2),
                (1, 2, 3),
                (2, 3, 4),
                (3, 4, 5),
                (4, 5, 6),
                (0, 5, 25),
            ]
            .into_iter()
            .map(|(from, to, cost)| EditableRoad {
                road: Road { from, to, cost },
                open: true,
            })
            .collect(),
            stops: vec![0, 2, 3, 5],
            order: vec![0, 5, 2, 3],
        },
        undo: Vec::new(),
        trace: None,
    })
    .expect("bounded authored route fixture")
}

fn route_qa_directory(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("App QA runs inside its workspace")
        .join(".agent")
        .join(name)
        .join(std::process::id().to_string())
}

fn route_authoring_panels() -> Vec<(&'static str, RoutePanel)> {
    use numinous_core::route::{MAX_ROUTE_JUNCTIONS, MAX_ROUTE_ROADS, Road, RouteEvent};
    use numinous_core::route_workbench::RouteSearchState;
    use numinous_core::route_workbench::{EditableRoad, RouteEdit, RouteWorkbench};
    let custom = authored_route_workbench();
    let comparison = custom.compare().expect("connected authored route fixture");
    assert_eq!(comparison.current.cost, 48);
    // The endpoints are distance 20 apart, so any closed delivery route costs
    // at least 40. A monotone visit followed by a return along the line attains it.
    assert_eq!(comparison.exact.tour.cost, 40);
    let mut dense_network = custom.snapshot();
    dense_network.current.junctions = MAX_ROUTE_JUNCTIONS;
    dense_network.current.roads = (0..MAX_ROUTE_JUNCTIONS)
        .flat_map(|from| ((from + 1)..MAX_ROUTE_JUNCTIONS).map(move |to| (from, to)))
        .take(MAX_ROUTE_ROADS)
        .map(|(from, to)| EditableRoad {
            road: Road {
                from,
                to,
                cost: ((from + to) % 9 + 1) as u32,
            },
            open: true,
        })
        .collect();
    dense_network.current.stops = vec![0, 4, 9, 15, 22, 31];
    dense_network.current.order = vec![0, 31, 9, 22, 4, 15];
    let dense = RouteWorkbench::from_snapshot(dense_network).expect("bounded dense route fixture");
    let mut draft = RoutePanel::new(RouteWorkbench::first_town());
    let before_draft = draft.workbench().snapshot();
    draft.act(RouteAction::Page(RoutePage::Roads));
    draft.act(RouteAction::NewRoad);
    draft.act(RouteAction::To(2));
    draft.act(RouteAction::Cost(3));
    assert_eq!(draft.workbench().snapshot(), before_draft);
    let mut disconnected = custom.clone();
    for (from, to) in [(0, 1), (0, 5)] {
        disconnected
            .apply(RouteEdit::RoadOpen {
                from,
                to,
                open: false,
            })
            .unwrap();
    }
    assert!(disconnected.compare().is_err());
    let disconnected_workbench = disconnected.clone();
    let mut disconnected = RoutePanel::new(disconnected);
    disconnected.act(RouteAction::Page(RoutePage::Roads));
    let search_panel = |complete: bool| {
        let mut panel = RoutePanel::new(RouteWorkbench::first_town());
        panel.act(RouteAction::Page(RoutePage::Search));
        for _ in 0..3 {
            panel.act(RouteAction::Next);
        }
        panel.act(RouteAction::Search);
        let count = if complete {
            panel.workbench().trace().unwrap().events().len()
        } else {
            3
        };
        for _ in 0..count {
            panel.act(RouteAction::Step);
        }
        assert_eq!(panel.workbench().trace().unwrap().completed(), complete);
        panel
    };
    let start_search = |workbench: RouteWorkbench, target: usize| {
        let mut panel = RoutePanel::new(workbench);
        panel.act(RouteAction::Page(RoutePage::Search));
        for _ in 0..target {
            panel.act(RouteAction::Next);
        }
        panel.act(RouteAction::Search);
        panel
    };
    let opening_search = start_search(RouteWorkbench::first_town(), 3);
    let opening = opening_search.workbench().trace().unwrap().view();
    assert_eq!(opening.cursor, 0);
    assert_eq!(opening.junctions[0].cost, Some(0));
    assert!(
        opening.junctions[1..]
            .iter()
            .all(|junction| junction.cost.is_none())
    );
    let mut improved_search = start_search(custom.clone(), 5);
    let improvement_cursor = improved_search
        .workbench()
        .trace()
        .unwrap()
        .events()
        .iter()
        .position(|event| {
            matches!(
                event,
                RouteEvent::Relaxed {
                    from: 4,
                    to: 5,
                    cost: 20
                }
            )
        })
        .expect("line route replaces the expensive direct tentative cost")
        + 1;
    for _ in 0..improvement_cursor {
        improved_search.act(RouteAction::Step);
    }
    let improved = improved_search.workbench().trace().unwrap().view();
    assert_eq!(improved.junctions[5].cost, Some(20));
    assert_eq!(improved.junctions[5].predecessor, Some(4));
    assert_eq!(improved.junctions[5].state, RouteSearchState::Tentative);
    assert!(!improved.completed);
    let mut rewound_search = RoutePanel::new(improved_search.workbench().clone());
    rewound_search.act(RouteAction::Page(RoutePage::Search));
    for _ in 0..5 {
        rewound_search.act(RouteAction::Next);
    }
    rewound_search.act(RouteAction::Back);
    let rewound = rewound_search.workbench().trace().unwrap().view();
    assert_eq!(rewound.junctions[5].cost, Some(25));
    assert_eq!(rewound.junctions[5].predecessor, Some(0));
    assert_eq!(rewound.cursor + 1, improved.cursor);
    let mut disconnected_search = start_search(disconnected_workbench, 5);
    disconnected_search.act(RouteAction::Step);
    let isolated = disconnected_search.workbench().trace().unwrap().view();
    assert!(isolated.completed);
    assert!(
        isolated.junctions[1..]
            .iter()
            .all(|junction| junction.state == RouteSearchState::Unreachable)
    );
    let mut dense_search = start_search(dense.clone(), 31);
    for _ in 0..12 {
        dense_search.act(RouteAction::Step);
    }
    assert_eq!(
        dense_search
            .workbench()
            .trace()
            .unwrap()
            .view()
            .junctions
            .len(),
        MAX_ROUTE_JUNCTIONS
    );
    let source = numinous_core::RouteCreation::new(custom.town().clone()).unwrap();
    let mut question = RoutePanel::opened(source.clone());
    question.act(RouteAction::Confirm);
    question.act(RouteAction::Page(RoutePage::Keep));
    question.push_text("Can a new delivery make the nearest-next order cheaper?");
    let child = source.remix(source.town().clone()).unwrap();
    assert_eq!(child.parent_identity(), Some(source.identity()));
    let portable = numinous_core::ProjectDocument::from_draft(
        &question
            .project_draft(0)
            .expect("portable authored question"),
    )
    .expect("canonical portable question without a local revision");
    let received = RoutePanel::received(portable.clone()).expect("paused received route");
    assert!(received.paused);
    assert_eq!(received.question, question.question);
    assert_eq!(received.workbench().town(), question.workbench().town());
    assert!(received.workbench().snapshot().undo.is_empty());
    assert!(received.workbench().trace().is_none());
    let mut shared = RoutePanel::received(portable.clone()).expect("route sharing receipt");
    shared.act(RouteAction::Confirm);
    shared.act(RouteAction::Page(RoutePage::Keep));
    let share_path = route_qa_directory("qa-route-sharing")
        .join("Café-東京")
        .join(format!(
            "numinous-route-question-{}-000.project",
            portable.identity_hex()
        ));
    numinous_core::export_project_document_file(&share_path, &portable)
        .expect("immutable portable sharing receipt");
    assert_eq!(
        numinous_core::read_project_document_file(&share_path)
            .unwrap()
            .identity_hex(),
        portable.identity_hex()
    );
    shared.shared_path = Some(share_path.display().to_string());
    shared.message = "QUESTION SHARED. GALLERY OPENS IT PAUSED.".into();
    let mut received_kept =
        RoutePanel::received(portable).expect("paused received route after deliberate keep");
    received_kept.mark_received_kept();
    assert!(received_kept.paused);
    let mut dense_question = RoutePanel::new(dense.clone());
    dense_question.act(RouteAction::Page(RoutePage::Keep));
    dense_question.push_text("Which road closure changes this delivery order most?");
    let dense_document = numinous_core::ProjectDocument::from_draft(
        &dense_question
            .project_draft(0)
            .expect("portable dense question"),
    )
    .expect("canonical dense question");
    let received_dense = RoutePanel::received(dense_document).expect("paused dense route");
    assert!(received_dense.paused);
    assert_eq!(
        received_dense.workbench().town().junctions,
        MAX_ROUTE_JUNCTIONS
    );
    let mut long_draft = question.project_draft(0).expect("portable question draft");
    let extended_question = "If a road closes after the first delivery, which alternate street walk keeps every required stop reachable, what changes in the total return cost, and can a different delivery order recover some of that cost without reopening the road? Compare the saved order against your choice.";
    long_draft.question = extended_question.to_string();
    let received_long = RoutePanel::received(
        numinous_core::ProjectDocument::from_draft(&long_draft).expect("long portable question"),
    )
    .expect("paused long route question");
    assert_eq!(
        received_long.question.chars().count(),
        numinous_core::MAX_WORKSPACE_TEXT_CHARS
    );
    let mut unicode_draft = long_draft;
    unicode_draft.question = "道路が閉鎖された後でも、すべての配達先を訪問して出発点に戻れる経路を探してください。保存された訪問順と最も近い配達先を選ぶ順を比較し、通過する交差点、使う道路、往復の総費用を説明してください。高い道路の費用を下げた場合と、配達先を一つ追加した場合では、どの選択が変わるでしょうか。探索を一歩ずつ進め、仮の費用が更新される場所を確認してください。同じネットワークで順序だけを変えた実験と、道路そのものを変えた実験を区別してください。最初の経路を保存したまま別の案を試し、改善した理由と残った制約を説明してください。Caféから戻る道も含めてください。理由も確認。".to_string();
    assert_eq!(
        unicode_draft.question.chars().count(),
        numinous_core::MAX_WORKSPACE_TEXT_CHARS
    );
    let unicode_document = numinous_core::ProjectDocument::from_draft(&unicode_draft)
        .expect("bounded Japanese and accented portable question");
    let received_unicode =
        RoutePanel::received(unicode_document.clone()).expect("paused shaped question");
    let received_unicode_last =
        RoutePanel::received(unicode_document.clone()).expect("paged shaped question");
    let mut keep_unicode =
        RoutePanel::received(unicode_document).expect("shaped question retained after opening");
    keep_unicode.act(RouteAction::Confirm);
    keep_unicode.act(RouteAction::Page(RoutePage::Keep));
    let random_panel = |page| {
        let mut panel = RoutePanel::new(RouteWorkbench::first_town());
        panel.act(RouteAction::Page(RoutePage::Maps));
        panel.act(RouteAction::RandomMap);
        panel.act(RouteAction::Page(page));
        panel
    };
    let mut random_prefix = random_panel(RoutePage::Search);
    random_prefix.act(RouteAction::SearchStart(2));
    random_prefix.act(RouteAction::Search);
    for _ in 0..3 {
        random_prefix.act(RouteAction::Step);
    }
    assert_eq!(random_prefix.workbench().trace().unwrap().view().from, 2);
    assert!(!random_prefix.workbench().trace().unwrap().completed());
    let mut random_complete = random_panel(RoutePage::Search);
    random_complete.act(RouteAction::Search);
    while !random_complete.workbench().trace().unwrap().completed() {
        random_complete.act(RouteAction::Step);
    }
    let mut random_picker = random_panel(RoutePage::Search);
    random_picker.act(RouteAction::PickEnd);
    let panels = vec![
        ("opening", RoutePanel::new(RouteWorkbench::first_town())),
        ("custom-network", RoutePanel::new(custom)),
        ("dense-network", RoutePanel::new(dense)),
        ("new-road-draft", draft),
        ("disconnected", disconnected),
        ("search-opening", opening_search),
        ("search-prefix", search_panel(false)),
        ("search-improved", improved_search),
        ("search-rewind", rewound_search),
        ("search-complete", search_panel(true)),
        ("search-disconnected", disconnected_search),
        ("search-dense", dense_search),
        ("random-options", random_panel(RoutePage::Maps)),
        ("random-map", random_panel(RoutePage::View)),
        ("random-search-prefix", random_prefix),
        ("random-search-complete", random_complete),
        ("random-pick-end", random_picker),
        ("keep-question", question),
        ("shared-question", shared),
        ("remix-preview", RoutePanel::opened(child)),
        ("received-question", received),
        ("received-kept", received_kept),
        ("received-dense", received_dense),
        ("received-long-question", received_long),
        ("received-unicode-question", received_unicode),
        ("received-unicode-last-page", received_unicode_last),
        ("keep-unicode-question", keep_unicode),
    ];
    assert!(
        panels
            .iter()
            .map(|(name, _)| *name)
            .eq(ROUTE_AUTHORING_STATES)
    );
    panels
}

fn route_authoring_frames(sizes: &[(&str, (usize, usize))]) -> Vec<(String, Raster)> {
    let mut frames = Vec::new();
    for (mode, hint) in [
        ("keyboard", None),
        ("controller", Some("DPAD SELECT. SOUTH ACTS. BACK LEAVES.")),
    ] {
        for (label, (width, height)) in sizes {
            for (state, mut panel) in route_authoring_panels() {
                if state == "received-unicode-last-page" {
                    // Paging uses the layout computed for this actual viewport.
                    panel.draw(*width, *height, hint);
                    for _ in 0..numinous_core::MAX_WORKSPACE_TEXT_CHARS {
                        panel.act(RouteAction::QuestionPage(1));
                    }
                }
                let raster = panel.draw(*width, *height, hint);
                assert_eq!((raster.width(), raster.height()), (*width, *height));
                assert!(raster.lit_count() > 20, "route editor {state} is not blank");
                frames.push((
                    format!("route-editor-{state}-{mode}-{label}-{width}x{height}.png"),
                    raster,
                ));
            }
        }
    }
    frames
}

fn route_gallery_frames(sizes: &[(&str, (usize, usize))]) -> Vec<(String, Raster)> {
    let directory = route_qa_directory("qa-route-gallery");
    std::fs::create_dir_all(&directory).expect("isolated Gallery fixture directory");
    let mut question = RoutePanel::new(authored_route_workbench());
    question.act(RouteAction::Page(RoutePage::Keep));
    question.question = "道路を閉じると、Caféへの配達順と往復費用はどう変わりますか？".into();
    let document = numinous_core::ProjectDocument::from_draft(
        &question.project_draft(0).expect("Gallery question draft"),
    )
    .expect("Gallery route document");
    numinous_core::export_project_document_file(&directory.join("route.project"), &document)
        .expect("immutable Gallery route fixture");
    let circle = numinous_core::StudioCreation::from_capsule("uniform-circle")
        .expect("canonical Studio Gallery fixture");
    std::fs::write(directory.join("circle.num"), circle.to_num_file())
        .expect("Gallery Studio fixture");
    let mut panel = gallery::GalleryPanel::open(&directory);
    let mut frames = Vec::new();
    let mut observed = BTreeSet::new();
    for _ in 0..ROUTE_GALLERY_STATES.len() {
        let state = if let Some(received) = panel.selected_project() {
            assert_eq!(received.identity_hex(), document.identity_hex());
            "question"
        } else {
            assert_eq!(panel.selected_creation(), Some(&circle));
            "studio"
        };
        assert!(
            observed.insert(state),
            "Gallery fixtures remain distinct tiles"
        );
        for (label, (width, height)) in sizes {
            let mut raster = Raster::new(*width, *height);
            panel.draw(&mut raster, *width, *height);
            assert!(raster.lit_count() > 20, "Gallery {state} is not blank");
            frames.push((
                format!("route-gallery-{state}-{label}-{width}x{height}.png"),
                raster,
            ));
        }
        panel.move_selection(1, 0);
    }
    assert_eq!(observed, ROUTE_GALLERY_STATES.into_iter().collect());
    frames
}

fn write_route_gallery_previews(output: &Path) {
    let mut manifest = Vec::new();
    for (name, raster) in route_gallery_frames(&[
        ("default", DEFAULT_SIZE),
        ("small", SMALL_SIZE),
        ("wide", (1600, 700)),
        ("wide-short", (1280, 400)),
    ]) {
        write_png(&raster, &output.join(&name));
        manifest.push(name);
    }
    manifest.sort();
    std::fs::write(
        output.join("MANIFEST.txt"),
        format!("{}\n", manifest.join("\n")),
    )
    .expect("write focused Gallery manifest");
    println!(
        "wrote {}",
        numinous_core::counted(manifest.len(), "Gallery preview")
    );
}

fn write_route_authoring_previews(output: &Path) {
    let mut manifest = Vec::new();
    for (name, mut raster) in route_authoring_frames(&[
        ("default", DEFAULT_SIZE),
        ("small", SMALL_SIZE),
        ("wide", (1600, 700)),
        ("wide-short", (1280, 400)),
    ]) {
        let audio = audio_state::describe(
            audio_state::Program::RoomScore,
            None,
            0.45,
            audio_state::SourceLevels::default(),
            false,
            true,
            true,
        );
        let width = raster.width();
        hud::draw_audio_state(&mut raster, &audio, width);
        write_png(&raster, &output.join(&name));
        manifest.push(name);
    }
    manifest.sort();
    std::fs::write(
        output.join("MANIFEST.txt"),
        format!("{}\n", manifest.join("\n")),
    )
    .expect("write focused route editor manifest");
    println!(
        "wrote {} for native route authoring",
        numinous_core::counted(manifest.len(), "preview")
    );
}

fn write_room_previews(output: &Path, room: &dyn Room) {
    let scenario = room_scenario(room);
    let id = room.meta().id;
    assert_scenario_shape(id, &scenario);
    assert_scenario_matches_verb(room, &scenario);
    assert_semantics(room, &scenario);
    assert_hold_release_contract(room, &scenario);
    let mut manifest = Vec::new();
    let mut captures = vec![
        ("base", 0.0, Vec::new(), 0),
        ("arrival", 0.0, Vec::new(), 1),
        ("interacted", 0.0, scenario.immediate.clone(), 0),
        (
            "delayed",
            scenario.delayed_phase,
            scenario.delayed.clone(),
            0,
        ),
    ];
    if id == "route-lab" {
        for (name, keys) in [
            ("closed-street", "c".to_string()),
            ("disconnected", "c.c".to_string()),
            ("undo", "c.cz".to_string()),
            ("trace-start", "t".to_string()),
            ("trace-prefix", "t...".to_string()),
            ("trace-complete", format!("t{}", ".".repeat(32))),
            ("trace-unreachable", "c.ct.".to_string()),
        ] {
            captures.push((
                name,
                0.0,
                keys.chars().map(|ch| RoomInput::Key { ch }).collect(),
                0,
            ));
        }
    }
    for (size_label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        assert_domain_response(
            id,
            size_label,
            &room_content(room, 0.0, &[], size),
            &room_content(room, 0.0, &scenario.immediate, size),
            &room_content(room, scenario.delayed_phase, &[], size),
            &room_content(room, scenario.delayed_phase, &scenario.delayed, size),
        );
        for (label, phase, inputs, card) in &captures {
            let mut raster = room_screen(room, *phase, inputs, size, *card, false, 7);
            let audio = audio_state::describe(
                audio_state::Program::RoomScore,
                None,
                0.45,
                audio_state::SourceLevels::default(),
                false,
                true,
                true,
            );
            hud::draw_audio_state(&mut raster, &audio, size.0);
            assert!(raster.lit_count() > 20, "{id}/{label} is not blank");
            let relative = format!("{label}-{size_label}-{}x{}.png", size.0, size.1);
            write_png(&raster, &output.join(&relative));
            manifest.push(relative);
        }
    }
    manifest.sort();
    std::fs::write(
        output.join("MANIFEST.txt"),
        format!("room={id}\n{}\n", manifest.join("\n")),
    )
    .expect("write focused room manifest");
    println!(
        "wrote {} for {id}",
        numinous_core::counted(manifest.len(), "preview")
    );
}

fn main() {
    let _generation_lock = GenerationLock::acquire(Path::new("renders/.qa-app.lock"))
        .expect("another App screenshot generator is already writing renders");
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments == ["--route-gallery"] {
        write_route_gallery_previews(Path::new("renders/qa-route-gallery"));
        return;
    }
    if arguments == ["--route-editor"] {
        write_route_authoring_previews(Path::new("renders/qa-route-editor"));
        return;
    }
    if let Some(index) = arguments.iter().position(|argument| argument == "--room") {
        if arguments.len() != 2 || index != 0 {
            eprintln!("Usage: screens --room <catalog-room-id>");
            std::process::exit(2);
        }
        let rooms = all_rooms();
        let Some(room) = rooms.iter().find(|room| room.meta().id == arguments[1]) else {
            eprintln!("Unknown catalog room: {}", arguments[1]);
            std::process::exit(2);
        };
        let output = Path::new("renders/qa-room").join(room.meta().id);
        write_room_previews(&output, room.as_ref());
        return;
    }
    if std::env::args().any(|argument| argument == "--readme") {
        write_readme_screens(Path::new(README_SCREENS));
        println!("wrote {} README plates", README_PLATES.len());
        return;
    }
    let output = Path::new(OUTPUT);
    if output.exists() {
        std::fs::remove_dir_all(output).expect("remove stale screenshot matrix");
    }
    let rooms = all_rooms();
    let mut manifest = Vec::new();
    if std::env::args().any(|argument| argument == "--menu-only") {
        let room = room_by_id(&rooms, "times-tables");
        let mut states = vec![("home", numinous_app::menu::MenuState::launch())];
        let mut modes = numinous_app::menu::MenuState::launch();
        let _ = modes.activate_shortcut('m');
        states.push(("modes", modes));
        let mut games = numinous_app::menu::MenuState::launch();
        let _ = games.activate_shortcut('g');
        states.push(("games", games));
        let mut options = numinous_app::menu::MenuState::launch();
        let _ = options.activate_shortcut('s');
        states.push(("options", options));
        let mut pause = numinous_app::menu::MenuState::launch();
        pause.open_pause(numinous_app::menu::ActivityKind::Quiz);
        states.push(("pause", pause));
        for (label, state) in states {
            let mut frame = room_screen(room, 0.12, &[], DEFAULT_SIZE, 0, false, 1);
            draw_cabinet_menu_state(
                &mut frame,
                &state,
                numinous_app::input_legend::InputMode::KeyboardMouse,
            );
            save(
                &frame,
                &format!("menu/{label}-{}x{}.png", DEFAULT_SIZE.0, DEFAULT_SIZE.1),
                &mut manifest,
            );
        }
        let fullscreen_state = numinous_app::menu::MenuState::launch();
        let mut fullscreen = room_screen(room, 0.12, &[], FULLSCREEN_SIZE, 0, false, 1);
        draw_cabinet_menu_state_with_display(
            &mut fullscreen,
            &fullscreen_state,
            numinous_app::input_legend::InputMode::KeyboardMouse,
            true,
        );
        save(
            &fullscreen,
            &format!(
                "menu/home-fullscreen-{}x{}.png",
                FULLSCREEN_SIZE.0, FULLSCREEN_SIZE.1
            ),
            &mut manifest,
        );
        let mut compact = room_screen(room, 0.12, &[], SMALL_SIZE, 0, false, 1);
        draw_cabinet_menu(
            &mut compact,
            numinous_app::input_legend::InputMode::Controller,
        );
        save(
            &compact,
            &format!(
                "menu/home-controller-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
        println!("wrote {} menu previews", manifest.len());
        return;
    }
    let mut interaction_kinds = BTreeSet::new();
    let mut changed_status_oracles = 0;
    let mut explained_action_oracles = 0;

    for room in &rooms {
        let id = room.meta().id;
        let phase = 0.0;
        let scenario = room_scenario(room.as_ref());
        assert_scenario_shape(id, &scenario);
        assert_scenario_matches_verb(room.as_ref(), &scenario);
        assert_semantics(room.as_ref(), &scenario);
        assert_hold_release_contract(room.as_ref(), &scenario);
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
        let immediate_hold = feedback_hold_inputs(&scenario.immediate);
        let delayed_hold = feedback_hold_inputs(&scenario.delayed);
        let raw_interacted_hold = room_content(room.as_ref(), phase, &immediate_hold, ROOM_SIZE);
        let raw_delayed_hold = room_content(
            room.as_ref(),
            scenario.delayed_phase,
            &delayed_hold,
            ROOM_SIZE,
        );
        let raw_small_interacted_hold =
            room_content(room.as_ref(), phase, &immediate_hold, SMALL_SIZE);
        let raw_small_delayed_hold = room_content(
            room.as_ref(),
            scenario.delayed_phase,
            &delayed_hold,
            SMALL_SIZE,
        );
        let feedback_interacted =
            apply_input_feedback(raw_interacted_hold.clone(), &immediate_hold);
        let feedback_delayed = apply_input_feedback(raw_delayed_hold.clone(), &delayed_hold);
        let feedback_small_interacted =
            apply_input_feedback(raw_small_interacted_hold.clone(), &immediate_hold);
        let feedback_small_delayed =
            apply_input_feedback(raw_small_delayed_hold.clone(), &delayed_hold);
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
        assert_domain_response(
            id,
            "default",
            &raw_base,
            &raw_interacted,
            &raw_delayed_base,
            &raw_delayed,
        );
        assert_domain_response(
            id,
            "compact",
            &raw_small_base,
            &raw_small_interacted,
            &raw_small_delayed_base,
            &raw_small_delayed,
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
            assert_feedback_chrome(
                id,
                "immediate feedback",
                &raw_interacted_hold,
                &feedback_interacted,
            );
            assert_feedback_chrome(id, "delayed feedback", &raw_delayed_hold, &feedback_delayed);
            assert_feedback_chrome(
                id,
                "compact immediate feedback",
                &raw_small_interacted_hold,
                &feedback_small_interacted,
            );
            assert_feedback_chrome(
                id,
                "compact delayed feedback",
                &raw_small_delayed_hold,
                &feedback_small_delayed,
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
            InteractionKind::Held,
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
        draw_cabinet_menu(
            &mut help,
            numinous_app::input_legend::InputMode::KeyboardMouse,
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
        overlays::draw_journey_overlay_with_controller(
            &mut journey_screen,
            &journey,
            &Scoreboard::default(),
            rooms.len(),
            (width, height),
            input_legend::InputMode::KeyboardMouse,
            input_legend::ControllerFace::Generic.into(),
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
            "watch-agent",
            audio_state::Program::WatchAgent,
            None,
            0.45,
            false,
            true,
            true,
            "WATCH AGENT: VOL 45%",
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
        let state = audio_state::describe(
            program,
            station,
            volume,
            audio_state::SourceLevels::default(),
            muted,
            active,
            output,
        );
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
    save_sizes("studio-morph", &mut manifest, studio_morph_screen);

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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
        game_draw::draw_nim(
            &nim,
            input_legend::InputMode::KeyboardMouse,
            input_legend::ControllerFace::Generic,
            width,
            height,
        )
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
            input_legend::ControllerFace::Generic,
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
            input_legend::ControllerFace::Generic,
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
                    input_legend::ControllerFace::Generic,
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
    draw_cabinet_menu(
        &mut controller_help,
        numinous_app::input_legend::InputMode::Controller,
    );
    save(
        &controller_help,
        "overlays/controller-help-small-360x240.png",
        &mut manifest,
    );

    let mut keyboard_paused = room_screen(golden, 0.42, &[], SMALL_SIZE, 0, false, 7);
    overlays::draw_pause_overlay_with_controller(
        &mut keyboard_paused,
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        input_legend::InputMode::KeyboardMouse,
        input_legend::ControllerFace::Generic.into(),
    );
    save(
        &keyboard_paused,
        "overlays/keyboard-paused-small-360x240.png",
        &mut manifest,
    );
    let mut controller_paused =
        room_screen_with_mode(golden, 0.42, &[], SMALL_SIZE, 0, false, 7, controller);
    overlays::draw_pause_overlay_with_controller(
        &mut controller_paused,
        SMALL_SIZE.0,
        SMALL_SIZE.1,
        controller,
        input_legend::ControllerFace::Generic.into(),
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
    overlays::draw_journey_overlay_with_controller(
        &mut controller_journey,
        &journey,
        &Scoreboard::default(),
        rooms.len(),
        SMALL_SIZE,
        controller,
        input_legend::ControllerFace::Generic.into(),
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
            input_legend::ControllerFace::Generic,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-quiz-result-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_munch(
            &munch,
            20,
            controller,
            input_legend::ControllerFace::Generic,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-munch-result-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_arcade(
            &arcade_over,
            controller,
            input_legend::ControllerFace::Generic,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-arcade-over-small-360x240.png",
        &mut manifest,
    );
    save(
        &game_draw::draw_nim(
            &nim_over,
            controller,
            input_legend::ControllerFace::Generic,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
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
            input_legend::ControllerFace::Generic,
            SMALL_SIZE.0,
            SMALL_SIZE.1,
        ),
        "games/controller-gauntlet-bomb-small-360x240.png",
        &mut manifest,
    );

    for (relative, raster) in
        route_authoring_frames(&[("default", DEFAULT_SIZE), ("small", SMALL_SIZE)])
    {
        save(&raster, &format!("overlays/{relative}"), &mut manifest);
    }
    for (relative, raster) in
        route_gallery_frames(&[("default", DEFAULT_SIZE), ("small", SMALL_SIZE)])
    {
        save(&raster, &format!("overlays/{relative}"), &mut manifest);
    }
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
        DEFAULT_MIN_CHANGED_PIXELS, DEFAULT_SIZE, GenerationLock, MIN_CHANGED_SUPPORT_PERMILLE,
        MIN_COHERENT_TILES, MIN_MEAN_CHANNEL_DELTA, MIN_SUPPORT_DENSITY_PERMILLE, README_PLATES,
        ROOM_SIZE, SHARED_SCREEN_COUNT, SMALL_SIZE, apply_input_feedback,
        assert_hold_release_contract, assert_scenario_matches_verb, assert_scenario_shape,
        assert_semantics, difference, domain_response_error, expected_paths, feedback_chrome_error,
        feedback_hold_inputs, life_cause_error, room_by_id, room_content, room_scenario,
        room_screen, write_readme_screens,
    };
    use numinous_core::{Raster, RoomInput, Surface, all_rooms};
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    #[test]
    fn scenario_and_inventory_contracts_track_the_catalog() {
        let rooms = all_rooms();
        for room in &rooms {
            let id = room.meta().id;
            let scenario = room_scenario(room.as_ref());
            assert_scenario_shape(id, &scenario);
            assert_scenario_matches_verb(room.as_ref(), &scenario);
            assert_semantics(room.as_ref(), &scenario);
            assert_hold_release_contract(room.as_ref(), &scenario);
        }
        assert_eq!(
            expected_paths(&rooms).len(),
            rooms.len() * 8 + SHARED_SCREEN_COUNT,
            "eight room states plus the exact shared evidence inventory"
        );
        let count = numinous_core::counted(expected_paths(&rooms).len(), "screen");
        for (name, prose) in [
            ("docs/ROADMAP.md", include_str!("../../../docs/ROADMAP.md")),
            ("VERIFY.md", include_str!("../../../VERIFY.md")),
        ] {
            assert!(
                prose.contains(&count),
                "{name} must describe the live QA inventory as {count}"
            );
        }
        // And no current document states a different matrix size.
        let documents = crate::prose_census::current_documents(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        );
        let (wrong, matched) =
            crate::prose_census::misstated(&documents, "# screen", expected_paths(&rooms).len());
        assert!(matched > 0, "no current document states the screen count");
        assert!(
            wrong.is_empty(),
            "stale screen counts:\n{}",
            wrong.join("\n")
        );
    }

    #[test]
    #[ignore = "release matrix diagnostic"]
    fn catalog_visual_contract_report() {
        let rooms = all_rooms();
        let mut errors = Vec::new();
        for room in &rooms {
            let id = room.meta().id;
            let scenario = room_scenario(room.as_ref());
            let base = room_content(room.as_ref(), 0.0, &[], ROOM_SIZE);
            let immediate = room_content(room.as_ref(), 0.0, &scenario.immediate, ROOM_SIZE);
            let immediate_hold = feedback_hold_inputs(&scenario.immediate);
            let delayed_hold = feedback_hold_inputs(&scenario.delayed);
            let immediate_hold_raw = room_content(room.as_ref(), 0.0, &immediate_hold, ROOM_SIZE);
            let immediate_feedback =
                apply_input_feedback(immediate_hold_raw.clone(), &immediate_hold);
            let delayed_base = room_content(room.as_ref(), scenario.delayed_phase, &[], ROOM_SIZE);
            let delayed = room_content(
                room.as_ref(),
                scenario.delayed_phase,
                &scenario.delayed,
                ROOM_SIZE,
            );
            let delayed_hold_raw = room_content(
                room.as_ref(),
                scenario.delayed_phase,
                &delayed_hold,
                ROOM_SIZE,
            );
            let delayed_feedback = apply_input_feedback(delayed_hold_raw.clone(), &delayed_hold);
            let small_base = room_content(room.as_ref(), 0.0, &[], SMALL_SIZE);
            let small_immediate = room_content(room.as_ref(), 0.0, &scenario.immediate, SMALL_SIZE);
            let small_immediate_hold =
                room_content(room.as_ref(), 0.0, &immediate_hold, SMALL_SIZE);
            let small_immediate_feedback =
                apply_input_feedback(small_immediate_hold.clone(), &immediate_hold);
            let small_delayed_base =
                room_content(room.as_ref(), scenario.delayed_phase, &[], SMALL_SIZE);
            let small_delayed = room_content(
                room.as_ref(),
                scenario.delayed_phase,
                &scenario.delayed,
                SMALL_SIZE,
            );
            let small_delayed_hold = room_content(
                room.as_ref(),
                scenario.delayed_phase,
                &delayed_hold,
                SMALL_SIZE,
            );
            let small_delayed_feedback =
                apply_input_feedback(small_delayed_hold.clone(), &delayed_hold);
            for error in [
                domain_response_error(id, "default", &base, &immediate, &delayed_base, &delayed),
                domain_response_error(
                    id,
                    "compact",
                    &small_base,
                    &small_immediate,
                    &small_delayed_base,
                    &small_delayed,
                ),
            ]
            .into_iter()
            .flatten()
            {
                errors.push(error);
            }
            if id == "game-of-life" {
                for error in [
                    life_cause_error("immediate", &base, &immediate),
                    life_cause_error("generation 4", &delayed_base, &delayed),
                    life_cause_error("compact immediate", &small_base, &small_immediate),
                    life_cause_error("compact generation 4", &small_delayed_base, &small_delayed),
                ]
                .into_iter()
                .flatten()
                {
                    errors.push(error);
                }
            } else {
                for error in [
                    feedback_chrome_error(
                        id,
                        "immediate feedback",
                        &immediate_hold_raw,
                        &immediate_feedback,
                    ),
                    feedback_chrome_error(
                        id,
                        "delayed feedback",
                        &delayed_hold_raw,
                        &delayed_feedback,
                    ),
                    feedback_chrome_error(
                        id,
                        "compact immediate feedback",
                        &small_immediate_hold,
                        &small_immediate_feedback,
                    ),
                    feedback_chrome_error(
                        id,
                        "compact delayed feedback",
                        &small_delayed_hold,
                        &small_delayed_feedback,
                    ),
                ]
                .into_iter()
                .flatten()
                {
                    errors.push(error);
                }
            }
        }
        assert!(
            errors.is_empty(),
            "{} visual contract failure(s):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }

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

    #[test]
    fn one_changed_pixel_does_not_satisfy_the_domain_oracle() {
        let before = Raster::with_accent(ROOM_SIZE.0, ROOM_SIZE.1, [255, 255, 255]);
        let mut after = before.clone();
        after.plot((ROOM_SIZE.0 / 2) as i32, (ROOM_SIZE.1 / 2) as i32, '#');
        assert_eq!(difference(&before, &after).changed, 1);
        let error = domain_response_error(
            "one-pixel-probe",
            "default",
            &before,
            &after,
            &before,
            &after,
        )
        .expect("one changed pixel is below the domain floor");
        assert!(error.contains("domain floor"), "{error}");
    }

    #[test]
    fn formerly_sparse_domain_responses_clear_the_floor() {
        let rooms = all_rooms();
        for id in [
            "laplace-clock",
            "message-heals",
            "recaman",
            "wireworld",
            "learning-clock",
            "serpentine",
        ] {
            let room = room_by_id(&rooms, id);
            let scenario = room_scenario(room);
            let base = room_content(room, 0.0, &[], ROOM_SIZE);
            let immediate = room_content(room, 0.0, &scenario.immediate, ROOM_SIZE);
            let delayed_base = room_content(room, scenario.delayed_phase, &[], ROOM_SIZE);
            let delayed = room_content(room, scenario.delayed_phase, &scenario.delayed, ROOM_SIZE);
            if let Some(err) =
                domain_response_error(id, "default", &base, &immediate, &delayed_base, &delayed)
            {
                panic!("{id} must retain a perceptible domain consequence: {err}");
            }
        }
    }

    #[test]
    fn lissajous_curve_survives_composed_app_chrome_at_both_window_sizes() {
        let rooms = all_rooms();
        let room = room_by_id(&rooms, "lissajous");
        let readme_tuning = [RoomInput::PointerDown {
            x: 0.5625,
            y: 0.1875,
            t: 0.15,
        }];
        // Untouched traces isolate the oscillator from clicked-cell markers.
        // The tuned case uses the public README plate's interior hand point.
        let cases: [(&str, f64, &[RoomInput]); 3] = [
            ("equal-frequency circle", 1.0 / 3.0, &[]),
            ("untuned sweep", 0.5, &[]),
            ("README 2:5 tuning", 0.15, &readme_tuning),
        ];
        for size in [DEFAULT_SIZE, SMALL_SIZE] {
            let background = Raster::with_accent(size.0, size.1, room.meta().accent).to_rgba();
            for (name, t, inputs) in cases {
                let source = room_content(room, t, inputs, size);
                assert!(
                    source.lit_count() > size.1,
                    "{name} at {size:?} must contain a visible oscillator trace"
                );
                let mut composed = room_screen(room, t, inputs, size, 0, false, 1);
                let audio = super::audio_state::describe(
                    super::audio_state::Program::RoomScore,
                    None,
                    0.45,
                    super::audio_state::SourceLevels::default(),
                    false,
                    true,
                    true,
                );
                super::hud::draw_audio_state(&mut composed, &audio, size.0);
                let source = source.to_rgba();
                let composed = composed.to_rgba();
                assert_ne!(source, composed, "the App chrome must actually be drawn");
                for (index, ((source_pixel, composed_pixel), background_pixel)) in source
                    .chunks_exact(4)
                    .zip(composed.chunks_exact(4))
                    .zip(background.chunks_exact(4))
                    .enumerate()
                {
                    if source_pixel != background_pixel {
                        assert_eq!(
                            composed_pixel,
                            source_pixel,
                            "{name} at {size:?}: chrome changed curve pixel ({}, {})",
                            index % size.0,
                            index / size.0
                        );
                    }
                }
            }
        }
    }

    fn png_size(path: &Path) -> (u32, u32) {
        let decoder =
            png::Decoder::new(BufReader::new(File::open(path).expect("open README plate")));
        let reader = decoder.read_info().expect("decode README plate");
        let info = reader.info();
        (info.width, info.height)
    }

    #[test]
    fn readme_gallery_writes_current_cabinet_and_play_chrome() {
        let output =
            std::env::temp_dir().join(format!("numinous-readme-screens-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&output);
        write_readme_screens(&output);

        for name in README_PLATES {
            let path = output.join(name);
            assert!(path.is_file(), "{name} is written");
            assert_eq!(
                png_size(&path),
                (DEFAULT_SIZE.0 as u32, DEFAULT_SIZE.1 as u32),
                "{name} matches the public App window"
            );
            let bytes = std::fs::metadata(&path)
                .expect("readme plate metadata")
                .len();
            assert!(
                bytes > 5_000,
                "{name} is a real plate, not a stub ({bytes} B)"
            );
        }

        let rooms = all_rooms();
        let golden = room_by_id(&rooms, "golden-angle");
        let play = room_screen(golden, 0.28, &[], DEFAULT_SIZE, 0, false, 1);
        let inspect = room_screen(golden, 0.28, &[], DEFAULT_SIZE, 0, true, 1);
        assert_ne!(
            play.to_rgba(),
            inspect.to_rgba(),
            "the public Golden Angle plate is play, not the inspect overlay"
        );

        std::fs::remove_dir_all(&output).expect("clean README plate temp dir");
    }
}
