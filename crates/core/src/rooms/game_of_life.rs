//! Conway's Game of Life: a universe from four rules.
//!
//! Each cell lives or dies based only on how many of its eight neighbors are
//! alive. From a random soup, gliders, oscillators, and still lifes emerge. `t`
//! sweeps the generation shown, so the life evolves as you scrub. The simulation
//! runs on a fixed toroidal grid and is sampled onto the surface, so the work is
//! bounded no matter how large the surface is. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::{MAX_DIM, Surface};

/// Simulation grid width and height (fixed, independent of the surface).
const GRID_W: usize = 96;
const GRID_H: usize = 96;
/// Fixed seed so the soup reproduces exactly.
const SEED: u64 = 0x11FE_0DED_5EED_600D;
/// Fraction of cells alive in the initial soup.
const DENSITY: f64 = 0.32;
/// Let the raw random soup open into legible colonies before first contact.
const SETTLE_GENERATIONS: usize = 12;
/// The most generations `t` reaches.
const MAX_GEN: usize = 140;
/// The last meaningful change to a persistent Life session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifeEvent {
    /// The untouched opening universe.
    Opening,
    /// A hand cleared a local patch and planted one five-cell glider.
    Launch {
        /// Exact cells placed by the glider stamp.
        planted: usize,
        /// Previously living cells removed to make the glider legible.
        cleared: usize,
    },
    /// One B3/S23 generation completed.
    Step {
        /// Dead cells born with exactly three neighbors.
        births: usize,
        /// Living cells that did not survive with two or three neighbors.
        deaths: usize,
    },
}

/// Bounded incremental state for one visit to Conway's Game of Life.
///
/// The App advances this state over elapsed time, so the universe does not jump
/// back when the gallery's normalized phase wraps. Stateless faces replay the
/// same state machine from their bounded input histories.
#[derive(Clone, Debug)]
pub struct LifeSession {
    generation: u64,
    launches: u64,
    live_cells: usize,
    event: LifeEvent,
    recent_launches: Vec<bool>,
    recent_births: Vec<bool>,
    step_sound: crate::life_sound::LifeStepSound,
    baseline: Vec<bool>,
    world: Vec<bool>,
    next_baseline: Vec<bool>,
    next_world: Vec<bool>,
}

impl LifeSession {
    /// Start one deterministic visit with the selected variation.
    #[must_use]
    pub fn new(variation: u64) -> Self {
        let world = seed(variation);
        let live_cells = world.iter().filter(|&&alive| alive).count();
        Self {
            generation: 0,
            launches: 0,
            live_cells,
            event: LifeEvent::Opening,
            recent_launches: vec![false; GRID_W * GRID_H],
            recent_births: vec![false; GRID_W * GRID_H],
            step_sound: crate::life_sound::LifeStepSound::default(),
            baseline: world.clone(),
            world,
            next_baseline: vec![false; GRID_W * GRID_H],
            next_world: vec![false; GRID_W * GRID_H],
        }
    }

    /// Advance one exact B3/S23 generation.
    pub fn advance(&mut self) {
        step_into(&self.world, GRID_W, GRID_H, &mut self.next_world);
        step_into(&self.baseline, GRID_W, GRID_H, &mut self.next_baseline);
        let mut births = 0;
        let mut deaths = 0;
        self.recent_births.fill(false);
        for index in 0..self.world.len() {
            let born = !self.world[index] && self.next_world[index];
            self.recent_births[index] = born;
            births += usize::from(born);
            deaths += usize::from(self.world[index] && !self.next_world[index]);
        }
        self.step_sound =
            crate::life_sound::LifeStepSound::from_birth_mask(&self.recent_births, GRID_W, GRID_H);
        std::mem::swap(&mut self.world, &mut self.next_world);
        std::mem::swap(&mut self.baseline, &mut self.next_baseline);
        self.live_cells = self.live_cells + births - deaths;
        self.generation = self.generation.saturating_add(1);
        self.event = LifeEvent::Step { births, deaths };
        self.recent_launches.fill(false);
    }

    /// Clear a small local patch and place one five-cell glider at `point`.
    ///
    /// Nonfinite points are rejected without changing the session.
    pub fn launch(&mut self, point: (f64, f64)) -> bool {
        let Some(cells) = sown_glider_cells(point) else {
            return false;
        };
        let (cx, cy) = cells[0];
        let mut cleared = 0;
        for dy in -5_i32..=5 {
            for dx in -5_i32..=5 {
                let x = (cx as i32 + dx).rem_euclid(GRID_W as i32) as usize;
                let y = (cy as i32 + dy).rem_euclid(GRID_H as i32) as usize;
                cleared += usize::from(self.world[y * GRID_W + x] && !cells.contains(&(x, y)));
            }
        }
        let newly_living = cells
            .iter()
            .filter(|&&(x, y)| !self.world[y * GRID_W + x])
            .count();
        if !plant_glider(&mut self.world, point) {
            return false;
        }
        self.live_cells = self.live_cells + newly_living - cleared;
        self.launches = self.launches.saturating_add(1);
        self.recent_births.fill(false);
        self.step_sound = crate::life_sound::LifeStepSound::default();
        for &(x, y) in &cells {
            self.recent_launches[y * GRID_W + x] = true;
        }
        self.event = LifeEvent::Launch {
            planted: 5,
            cleared,
        };
        true
    }

    /// Generations completed since this visit began.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Gliders launched during this visit.
    #[must_use]
    pub fn launches(&self) -> u64 {
        self.launches
    }

    /// Fixed-size sonic reduction of births in the newest exact generation.
    #[must_use]
    pub fn step_sound(&self) -> &crate::life_sound::LifeStepSound {
        &self.step_sound
    }

    /// A truthful readout for wide App layouts and agent faces.
    #[must_use]
    pub fn status(&self) -> String {
        let glider = if self.launches == 1 {
            "GLIDER"
        } else {
            "GLIDERS"
        };
        match self.event {
            LifeEvent::Opening => format!(
                "B3/S23  GEN {}  LIVE {}  {glider} {}",
                self.generation, self.live_cells, self.launches
            ),
            LifeEvent::Launch { planted, cleared } => format!(
                "PLANTED {planted}  CLEARED {cleared}  GEN {}  LIVE {}  {glider} {}",
                self.generation, self.live_cells, self.launches
            ),
            LifeEvent::Step { births, deaths } => format!(
                "BORN {births}  DIED {deaths}  GEN {}  LIVE {}  {glider} {}",
                self.generation, self.live_cells, self.launches
            ),
        }
    }

    /// A compact footer readout that keeps the causal fields on small windows.
    #[must_use]
    pub fn compact_status(&self) -> String {
        match self.event {
            LifeEvent::Opening => format!(
                "RULE B3/S23 G{} L{} GL{}",
                self.generation, self.live_cells, self.launches
            ),
            LifeEvent::Launch { planted, cleared } => format!(
                "PLANT{planted} CLEAR{cleared} G{} L{} GL{}",
                self.generation, self.live_cells, self.launches
            ),
            LifeEvent::Step { births, deaths } => format!(
                "BORN{births} DIED{deaths} G{} L{} GL{}",
                self.generation, self.live_cells, self.launches
            ),
        }
    }

    /// Draw the untouched universe normally and causal deviations brightly.
    pub fn render(&self, canvas: &mut dyn Surface) {
        draw_grid_state(
            canvas,
            (self.launches > 0).then_some(self.baseline.as_slice()),
            &self.world,
            &self.recent_launches,
            &self.recent_births,
        );
    }
}

fn drawing_dims(canvas: &dyn Surface) -> Option<(usize, usize)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width.min(MAX_DIM), height.min(MAX_DIM)))
    }
}

/// The Game of Life room.
#[derive(Debug, Default)]
pub struct GameOfLife {
    seed: u64,
}

impl GameOfLife {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The generation shown at phase `t`.
    fn generation_for(t: f64) -> usize {
        let phase = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        (phase * MAX_GEN as f64).round() as usize
    }

    fn ambient_sound(&self) -> SoundSpec {
        self.motif().map_or_else(
            || SoundSpec::tone(130.81, 0.5, 0.1),
            |motif| SoundSpec::from_motif(&motif),
        )
    }
}

impl Room for GameOfLife {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "game-of-life",
            title: "Game of Life",
            wing: "Emergence",
            blurb: "Aim at a quiet patch and place five living cells. Birth with 3 neighbors and \
                    survival with 2 or 3 make that glider move by itself.",
            accent: [90, 210, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let mut session = LifeSession::new(self.seed);
        advance_session(&mut session, Self::generation_for(t));
        session.render(canvas);
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C major, sparse",
            root: 130.81,
            tempo: 112,
            line: &[0, 0, 4, 0, 7, 0, 4, 0],
            encodes: "pulses of birth against silence: cells live and die on a clock",
        })
    }

    fn status(&self, t: f64) -> Option<String> {
        let mut session = LifeSession::new(self.seed);
        advance_session(&mut session, Self::generation_for(t));
        Some(session.status())
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        Some(
            session_with_launches(Self::generation_for(t), self.seed, &launch_events(inputs))
                .status(),
        )
    }

    fn sound(&self, t: f64) -> SoundSpec {
        let mut session = LifeSession::new(self.seed);
        advance_session(&mut session, Self::generation_for(t));
        session
            .step_sound()
            .snapshot()
            .unwrap_or_else(|| self.ambient_sound())
    }

    fn sound_input(&self, t: f64, inputs: &[RoomInput]) -> SoundSpec {
        session_with_launches(Self::generation_for(t), self.seed, &launch_events(inputs))
            .step_sound()
            .snapshot()
            .unwrap_or_else(|| self.ambient_sound())
    }

    fn verb(&self) -> Option<&'static str> {
        Some("AIM + CLICK: PLACE A 5-CELL GLIDER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let generations = Self::generation_for(t);
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        let launches = pokes[start..]
            .iter()
            .filter_map(|&(x, y)| {
                (x.is_finite() && y.is_finite()).then_some(GliderLaunch {
                    point: (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
                    generation: generations,
                })
            })
            .collect::<Vec<_>>();
        if launches.is_empty() {
            self.render(canvas, t);
            return;
        }
        session_with_launches(generations, self.seed, &launches).render(canvas);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let launches = launch_events(inputs);
        if launches.is_empty() {
            self.render(canvas, t);
            return;
        }
        session_with_launches(Self::generation_for(t), self.seed, &launches).render(canvas);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn reveal(&self) -> &'static str {
        "Those four rules are enough to build a working computer. People have \
         built Tetris, and the Game of Life itself, running inside the Game of \
         Life. It is not a toy, it is a universe."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Conway bet fifty dollars that no pattern could grow forever. Bill \
             Gosper's glider gun, found in 1970, fires a glider every thirty \
             generations, forever. Conway paid.",
            "Whether a Life pattern eventually dies is undecidable: no algorithm can \
             answer it for every pattern, for the same reason no program can decide \
             whether every other program halts. The toy grid inherits the deepest \
             limit in computer science.",
        ]
    }
}

fn draw_grid_state(
    canvas: &mut dyn Surface,
    baseline: Option<&[bool]>,
    world: &[bool],
    recent_launches: &[bool],
    recent_births: &[bool],
) {
    let Some((width, height)) = drawing_dims(canvas) else {
        return;
    };
    for py in 0..height {
        for px in 0..width {
            let gx = px * GRID_W / width;
            let gy = py * GRID_H / height;
            let index = gy * GRID_W + gx;
            if world[index] {
                let mark = if recent_births[index] {
                    '@'
                } else if recent_launches[index] || baseline.is_some_and(|opening| !opening[index])
                {
                    '#'
                } else {
                    '*'
                };
                canvas.plot(px as i32, py as i32, mark);
            }
        }
    }
}

fn sown_glider_cells(point: (f64, f64)) -> Option<[(usize, usize); 5]> {
    let (x, y) = point;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let cx = (x.clamp(0.0, 1.0) * (GRID_W - 1) as f64).round() as i32;
    let cy = (y.clamp(0.0, 1.0) * (GRID_H - 1) as f64).round() as i32;
    let mut cells = [(0, 0); 5];
    for (cell, (dx, dy)) in cells
        .iter_mut()
        .zip([(0, 0), (1, 0), (2, 0), (2, -1), (1, -2)])
    {
        *cell = (
            (cx + dx).rem_euclid(GRID_W as i32) as usize,
            (cy + dy).rem_euclid(GRID_H as i32) as usize,
        );
    }
    Some(cells)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GliderLaunch {
    point: (f64, f64),
    generation: usize,
}

fn plant_glider(grid: &mut [bool], point: (f64, f64)) -> bool {
    let Some(cells) = sown_glider_cells(point) else {
        return false;
    };
    let (cx, cy) = cells[0];
    for dy in -5_i32..=5 {
        for dx in -5_i32..=5 {
            let x = (cx as i32 + dx).rem_euclid(GRID_W as i32) as usize;
            let y = (cy as i32 + dy).rem_euclid(GRID_H as i32) as usize;
            grid[y * GRID_W + x] = false;
        }
    }
    for (x, y) in cells {
        grid[y * GRID_W + x] = true;
    }
    true
}

#[cfg(test)]
fn sow_pokes(grid: &mut [bool], pokes: &[(f64, f64)]) {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    for &point in &pokes[start..] {
        plant_glider(grid, point);
    }
}

fn launch_events(inputs: &[RoomInput]) -> Vec<GliderLaunch> {
    let raw: Vec<_> = inputs
        .iter()
        .filter_map(|input| match *input {
            RoomInput::PointerDown { x, y, t } => Some((x, y, t)),
            _ => None,
        })
        .collect();
    let start = raw.len().saturating_sub(MAX_ROOM_POKES);
    raw[start..]
        .iter()
        .filter_map(|&(x, y, t)| {
            (x.is_finite() && y.is_finite() && t.is_finite()).then_some(GliderLaunch {
                point: (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
                generation: GameOfLife::generation_for(t),
            })
        })
        .collect()
}

#[cfg(test)]
fn simulate_with_launches(
    generations: usize,
    variation: u64,
    launches: &[GliderLaunch],
) -> Vec<bool> {
    session_with_launches(generations, variation, launches).world
}

fn session_with_launches(
    generations: usize,
    variation: u64,
    launches: &[GliderLaunch],
) -> LifeSession {
    let generations = generations.min(MAX_GEN);
    let mut session = LifeSession::new(variation);
    let mut ordered = launches
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, launch)| launch.generation <= generations)
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(index, launch)| (launch.generation, *index));
    let mut current = 0;
    for (_, launch) in ordered {
        let target = launch.generation;
        advance_session(&mut session, target - current);
        session.launch(launch.point);
        current = target;
    }
    advance_session(&mut session, generations - current);
    session
}

fn advance_session(session: &mut LifeSession, generations: usize) {
    for _ in 0..generations {
        session.advance();
    }
}

#[cfg(test)]
fn simulate_with_pokes(generations: usize, variation: u64, pokes: &[(f64, f64)]) -> Vec<bool> {
    let mut grid = seed(variation);
    sow_pokes(&mut grid, pokes);
    for _ in 0..generations.min(MAX_GEN) {
        grid = step(&grid, GRID_W, GRID_H);
    }
    grid
}

#[cfg(test)]
fn glider_on_empty_grid(point: (f64, f64), generations: usize) -> Vec<bool> {
    let mut grid = vec![false; GRID_W * GRID_H];
    sow_pokes(&mut grid, &[point]);
    for _ in 0..generations.min(MAX_GEN) {
        grid = step(&grid, GRID_W, GRID_H);
    }
    grid
}

/// The initial soup, seeded deterministically.
fn seed(variation: u64) -> Vec<bool> {
    let mut rng = SplitMix64::new(SEED ^ variation);
    let mut grid = (0..GRID_W * GRID_H)
        .map(|_| rng.next_f64() < DENSITY)
        .collect::<Vec<_>>();
    for _ in 0..SETTLE_GENERATIONS {
        grid = step(&grid, GRID_W, GRID_H);
    }
    grid
}

/// Run the Game of Life for `generations` steps from the seed.
#[cfg(test)]
fn simulate(generations: usize, variation: u64) -> Vec<bool> {
    let mut grid = seed(variation);
    for _ in 0..generations.min(MAX_GEN) {
        grid = step(&grid, GRID_W, GRID_H);
    }
    grid
}

/// Advance one generation on a toroidal grid (rules B3/S23).
fn step(grid: &[bool], w: usize, h: usize) -> Vec<bool> {
    let mut next = vec![false; w * h];
    step_into(grid, w, h, &mut next);
    next
}

fn step_into(grid: &[bool], w: usize, h: usize, next: &mut [bool]) {
    next.fill(false);
    for y in 0..h {
        for x in 0..w {
            let mut neighbors = 0u8;
            for dy in [-1i32, 0, 1] {
                for dx in [-1i32, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = (x as i32 + dx).rem_euclid(w as i32) as usize;
                    let ny = (y as i32 + dy).rem_euclid(h as i32) as usize;
                    if grid[ny * w + nx] {
                        neighbors += 1;
                    }
                }
            }
            // Rules B3/S23: born on 3 neighbors, survives on 2 or 3.
            let alive = grid[y * w + x];
            next[y * w + x] = neighbors == 3 || (alive && neighbors == 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GRID_W, GameOfLife, GliderLaunch, LifeEvent, LifeSession, advance_session,
        glider_on_empty_grid, launch_events, simulate, simulate_with_launches, simulate_with_pokes,
        sow_pokes, sown_glider_cells, step,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::surface::{MAX_DIM, Surface};

    fn grid_with(w: usize, h: usize, live: &[(usize, usize)]) -> Vec<bool> {
        let mut g = vec![false; w * h];
        for &(x, y) in live {
            g[y * w + x] = true;
        }
        g
    }

    #[test]
    fn a_block_is_a_still_life() {
        let live = [(2, 2), (3, 2), (2, 3), (3, 3)];
        let g = grid_with(6, 6, &live);
        assert_eq!(step(&g, 6, 6), g, "a 2x2 block should not change");
    }

    #[test]
    fn a_blinker_oscillates_with_period_two() {
        let horizontal = grid_with(5, 5, &[(1, 2), (2, 2), (3, 2)]);
        let vertical = grid_with(5, 5, &[(2, 1), (2, 2), (2, 3)]);
        let a = step(&horizontal, 5, 5);
        assert_eq!(a, vertical, "a horizontal blinker becomes vertical");
        assert_eq!(step(&a, 5, 5), horizontal, "and back after two steps");
    }

    #[test]
    fn b3_s23_truth_table_is_exhaustive() {
        let neighbors = [
            (1, 1),
            (2, 1),
            (3, 1),
            (1, 2),
            (3, 2),
            (1, 3),
            (2, 3),
            (3, 3),
        ];
        for alive in [false, true] {
            for count in 0..=8 {
                let mut live = neighbors[..count].to_vec();
                if alive {
                    live.push((2, 2));
                }
                let grid = grid_with(5, 5, &live);
                let next = step(&grid, 5, 5);
                let expected = count == 3 || alive && count == 2;
                assert_eq!(
                    next[2 * 5 + 2],
                    expected,
                    "alive={alive}, neighbors={count}"
                );
            }
        }
    }

    #[test]
    fn life_session_advances_the_exact_b3_s23_world() {
        let mut session = LifeSession::new(7);
        let opening = session.world.clone();
        let expected = step(&opening, GRID_W, super::GRID_H);
        let births = opening
            .iter()
            .zip(&expected)
            .filter(|(before, after)| !**before && **after)
            .count();
        let deaths = opening
            .iter()
            .zip(&expected)
            .filter(|(before, after)| **before && !**after)
            .count();

        session.advance();
        assert_eq!(session.world, expected);
        assert_eq!(session.generation(), 1);
        assert_eq!(session.event, LifeEvent::Step { births, deaths });
        assert_eq!(
            opening.iter().filter(|&&alive| alive).count() + births,
            session.live_cells + deaths,
            "population change equals births minus deaths"
        );
        assert_eq!(session.step_sound().birth_count(), births);
        assert_eq!(
            session.recent_births.iter().filter(|&&born| born).count(),
            births
        );
        for (index, (&before, &after)) in opening.iter().zip(&expected).enumerate() {
            assert_eq!(session.recent_births[index], !before && after);
        }

        let audio = session.step_sound().render_stereo(48_000);
        assert!(!audio.is_empty());
        assert_eq!(audio.len() % 2, 0);
        assert!(audio.iter().all(|sample| sample.is_finite()));
        assert!(audio.iter().all(|sample| sample.abs() <= 0.2));

        let mut canvas = Canvas::new(GRID_W, super::GRID_H);
        session.render(&mut canvas);
        for (index, &born) in session.recent_births.iter().enumerate() {
            if born {
                assert_eq!(canvas.cell(index % GRID_W, index / GRID_W), Some('@'));
            }
        }
    }

    #[test]
    fn life_snapshot_sound_is_the_last_exact_generation_event() {
        let room = GameOfLife::new_with(9);
        let t = 0.37;
        let inputs = [crate::room::RoomInput::PointerDown {
            x: 0.24,
            y: 0.71,
            t: 0.08,
        }];
        let session = super::session_with_launches(
            GameOfLife::generation_for(t),
            9,
            &super::launch_events(&inputs),
        );
        let expected = session
            .step_sound()
            .snapshot()
            .expect("evolving Life has births");
        let actual = room.sound_input(t, &inputs);

        assert_eq!(actual.notes, expected.notes);
        assert_eq!(actual.duration, expected.duration);
    }

    #[test]
    fn a_launch_at_the_snapshot_boundary_does_not_invent_unseen_births() {
        let room = GameOfLife::new_with(9);
        let t = 0.37;
        let inputs = [crate::room::RoomInput::PointerDown {
            x: 0.24,
            y: 0.71,
            t,
        }];
        let actual = room.sound_input(t, &inputs);
        let ambient = room.ambient_sound();

        assert_eq!(actual.notes, ambient.notes);
        assert_eq!(actual.duration, ambient.duration);
    }

    #[test]
    fn life_session_launch_sequence_preserves_derived_state() {
        let opening = LifeSession::new(3).world;
        let mut session = LifeSession::new(3);
        let points = [(0.02, 0.02), (0.70, 0.70), (0.02, 0.02)];
        for point in points {
            assert!(session.launch(point));
            assert_eq!(session.baseline, opening, "launches do not rewrite history");
            assert_eq!(
                session.live_cells,
                session.world.iter().filter(|&&alive| alive).count(),
                "population remains derived after {point:?}"
            );
        }
        let mut canvas = Canvas::new(super::GRID_W, super::GRID_H);
        session.render(&mut canvas);
        for point in &points[..2] {
            for (x, y) in sown_glider_cells(*point).expect("finite glider") {
                if session.world[y * super::GRID_W + x] {
                    assert_eq!(canvas.cell(x, y), Some('#'));
                }
            }
        }

        session.advance();
        assert_eq!(
            session.baseline,
            step(&opening, super::GRID_W, super::GRID_H)
        );
        assert!(!session.recent_launches.iter().any(|&recent| recent));
        assert_eq!(
            session.live_cells,
            session.world.iter().filter(|&&alive| alive).count()
        );
    }

    #[test]
    fn life_session_launch_is_causal_and_survives_phase_sized_runs() {
        let mut session = LifeSession::new(11);
        assert!(session.launch((0.5, 0.5)));
        assert_eq!(session.generation(), 0);
        assert_eq!(session.launches(), 1);
        assert!(matches!(
            session.event,
            LifeEvent::Launch {
                planted: 5,
                cleared: _
            }
        ));
        for (x, y) in sown_glider_cells((0.5, 0.5)).expect("finite glider") {
            assert!(session.world[y * GRID_W + x]);
        }

        advance_session(&mut session, super::MAX_GEN);
        assert_eq!(session.generation(), super::MAX_GEN as u64);
        assert_eq!(session.launches(), 1);
        session.advance();
        assert_eq!(
            session.generation(),
            super::MAX_GEN as u64 + 1,
            "a normalized gallery sweep is not a session reset"
        );
    }

    #[test]
    fn life_session_reset_restores_the_selected_variation() {
        let opening = LifeSession::new(3).world;
        let mut session = LifeSession::new(3);
        session.launch((0.2, 0.8));
        advance_session(&mut session, 12);
        session = LifeSession::new(3);

        assert_eq!(session.world, opening);
        assert_eq!(session.baseline, session.world);
        assert_eq!(session.generation(), 0);
        assert_eq!(session.launches(), 0);
        assert_eq!(session.event, LifeEvent::Opening);
    }

    #[test]
    fn life_session_status_names_the_action_then_the_rule_consequence() {
        let mut session = LifeSession::new(0);
        session.launch((0.4, 0.6));
        let launched = session.status();
        assert!(launched.contains("GLIDER 1"), "got: {launched}");
        assert!(launched.contains("PLANTED 5"), "got: {launched}");

        session.advance();
        let advanced = session.status();
        assert!(advanced.starts_with("BORN "), "got: {advanced}");
        assert!(advanced.contains("GEN 1"), "got: {advanced}");
        assert!(advanced.contains("GLIDER 1"), "got: {advanced}");
        assert!(advanced.contains("LIVE "), "got: {advanced}");
        assert!(advanced.contains("BORN "), "got: {advanced}");
        assert!(advanced.contains("DIED "), "got: {advanced}");
        let compact = session.compact_status();
        assert!(compact.starts_with("BORN"), "got: {compact}");
        assert!(compact.contains("DIED"), "got: {compact}");
        assert!(compact.contains("G1"), "got: {compact}");
        assert!(compact.contains("GL1"), "got: {compact}");
    }

    #[test]
    fn nonfinite_launch_is_a_bit_for_bit_no_op() {
        let mut session = LifeSession::new(5);
        advance_session(&mut session, 7);
        let before_world = session.world.clone();
        let before_baseline = session.baseline.clone();
        let before_status = session.status();
        let before_event = session.event;

        assert!(!session.launch((f64::NAN, 0.5)));
        assert_eq!(session.world, before_world);
        assert_eq!(session.baseline, before_baseline);
        assert_eq!(session.status(), before_status);
        assert_eq!(session.event, before_event);
    }

    #[test]
    fn render_and_status_are_pure_session_observations() {
        let mut session = LifeSession::new(9);
        session.launch((0.4, 0.6));
        advance_session(&mut session, 141);
        let generation = session.generation();
        let status = session.status();
        let mut first = Canvas::new(72, 36);
        let mut second = Canvas::new(72, 36);

        session.render(&mut first);
        session.render(&mut second);

        assert_eq!(first.to_text(), second.to_text());
        assert_eq!(session.generation(), generation);
        assert_eq!(session.status(), status);
    }

    #[test]
    fn generation_maps_zero_to_the_soup() {
        assert_eq!(GameOfLife::generation_for(0.0), 0);
    }

    #[test]
    fn nonfinite_phase_falls_back_to_the_first_generation() {
        assert_eq!(GameOfLife::generation_for(f64::NAN), 0);
        assert_eq!(GameOfLife::generation_for(f64::NEG_INFINITY), 0);
        assert_eq!(GameOfLife::generation_for(f64::INFINITY), super::MAX_GEN);
    }

    #[test]
    fn render_is_deterministic() {
        let room = GameOfLife::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn sown_glider_uses_both_coordinates() {
        let center = sown_glider_cells((0.25, 0.75)).expect("finite point");
        let moved_x = sown_glider_cells((0.75, 0.75)).expect("finite point");
        let moved_y = sown_glider_cells((0.25, 0.25)).expect("finite point");

        assert_ne!(center, moved_x, "x moves the planted cells");
        assert_ne!(center, moved_y, "y moves the planted cells");
        assert!(sown_glider_cells((f64::NAN, 0.5)).is_none());

        let mut grid = vec![false; GRID_W * super::GRID_H];
        sow_pokes(&mut grid, &[(0.25, 0.75)]);
        for (x, y) in center {
            assert!(grid[y * GRID_W + x], "sown cell ({x},{y}) is alive");
        }
    }

    #[test]
    fn sown_life_evolves_under_the_same_rules() {
        let point = [(0.33, 0.66)];
        let base = simulate(4, 0);
        let sown_start = simulate_with_pokes(0, 0, &point);
        let sown_evolved = simulate_with_pokes(4, 0, &point);
        let sown_after_one = simulate_with_pokes(1, 0, &point);

        assert_ne!(base, sown_evolved, "the planted cells affect the future");
        assert_ne!(sown_start, sown_evolved, "the planted pattern evolves");
        assert_eq!(
            sown_after_one,
            step(&sown_start, GRID_W, super::GRID_H),
            "sown cells advance through the same B3/S23 transition"
        );
        assert_eq!(simulate(4, 0), simulate_with_pokes(4, 0, &[]));
    }

    #[test]
    fn public_render_poked_visibly_changes_the_room() {
        let room = GameOfLife::new();
        let mut base = Canvas::new(72, 36);
        let mut poked = Canvas::new(72, 36);

        room.render(&mut base, 0.12);
        room.render_poked(&mut poked, 0.12, &[(0.18, 0.82)]);

        assert_ne!(base.to_text(), poked.to_text());
        assert!(poked.ink_count() > 10);
    }

    #[test]
    fn planted_glider_moves_on_the_toroidal_grid() {
        let mut start_cells = sown_glider_cells((0.5, 0.5)).expect("center glider");
        start_cells.sort_unstable();
        let mut after_four = live_cells(&glider_on_empty_grid((0.5, 0.5), 4));
        after_four.sort_unstable();
        let expected = start_cells.map(|(x, y)| ((x + 1) % GRID_W, (y + 1) % super::GRID_H));
        let mut expected = expected.to_vec();
        expected.sort_unstable();

        assert_eq!(after_four, expected);

        let edge = sown_glider_cells((1.0, 0.0)).expect("edge glider");
        assert!(
            edge.iter().any(|&(x, _)| x == 0),
            "right edge wraps to column 0"
        );
        assert!(
            edge.iter().any(|&(_, y)| y == super::GRID_H - 1),
            "top edge wraps upward"
        );
    }

    #[test]
    fn phase_stamped_launch_clears_and_plants_at_the_clicked_generation() {
        let phase = 0.5;
        let generation = GameOfLife::generation_for(phase);
        let inputs = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: phase,
        }];
        let launches = launch_events(&inputs);
        assert_eq!(
            launches,
            vec![GliderLaunch {
                point: (0.5, 0.5),
                generation
            }]
        );
        let grid = simulate_with_launches(generation, 0, &launches);
        let cells = sown_glider_cells((0.5, 0.5)).expect("glider");
        let (cx, cy) = cells[0];
        for dy in -4_i32..=4 {
            for dx in -4_i32..=4 {
                let x = (cx as i32 + dx).rem_euclid(GRID_W as i32) as usize;
                let y = (cy as i32 + dy).rem_euclid(super::GRID_H as i32) as usize;
                assert_eq!(
                    grid[y * GRID_W + x],
                    cells.contains(&(x, y)),
                    "launch neighborhood differs at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn a_future_launch_is_absent_when_scrubbing_before_it() {
        let launch = GliderLaunch {
            point: (0.5, 0.5),
            generation: 90,
        };
        assert_eq!(
            simulate_with_launches(40, 0, &[launch]),
            simulate(40, 0),
            "an event cannot change the universe before its recorded generation"
        );
    }

    #[test]
    fn stateless_replay_orders_decreasing_timestamps_chronologically() {
        let late = GliderLaunch {
            point: (0.8, 0.2),
            generation: 80,
        };
        let early = GliderLaunch {
            point: (0.2, 0.8),
            generation: 20,
        };

        assert_eq!(
            simulate_with_launches(100, 3, &[late, early]),
            simulate_with_launches(100, 3, &[early, late]),
            "timestamps define the stateless causal order, not array position"
        );
    }

    #[test]
    fn a_drag_launches_only_once_at_pointer_down() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.4,
                y: 0.6,
                t: 0.2,
            },
            RoomInput::PointerMove {
                x: 0.5,
                y: 0.5,
                t: 0.21,
            },
            RoomInput::PointerMove {
                x: 0.6,
                y: 0.4,
                t: 0.22,
            },
            RoomInput::PointerUp {
                x: 0.6,
                y: 0.4,
                t: 0.23,
            },
        ];
        let launches = launch_events(&inputs);
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].point, (0.4, 0.6));
    }

    #[test]
    fn compact_poke_and_phase_stamped_click_render_identically() {
        let room = GameOfLife::new();
        let phase = 0.47;
        let point = (0.23, 0.71);
        let mut compact = Canvas::new(64, 48);
        let mut event = Canvas::new(64, 48);
        room.render_poked(&mut compact, phase, &[point]);
        room.render_input(
            &mut event,
            phase,
            &[RoomInput::PointerDown {
                x: point.0,
                y: point.1,
                t: phase,
            }],
        );
        assert_eq!(compact.to_text(), event.to_text());
    }

    fn live_cells(grid: &[bool]) -> Vec<(usize, usize)> {
        grid.iter()
            .enumerate()
            .filter_map(|(i, &alive)| alive.then_some((i % GRID_W, i / GRID_W)))
            .collect()
    }

    #[test]
    fn sowed_cells_use_the_newest_bounded_raw_tail() {
        let newest = vec![(0.85, 0.15); MAX_ROOM_POKES];
        let mut all = vec![(0.15, 0.85); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();
        let mut expected = vec![false; GRID_W * super::GRID_H];
        let mut actual = vec![false; GRID_W * super::GRID_H];
        let mut prefix_only = vec![false; GRID_W * super::GRID_H];

        sow_pokes(&mut expected, &newest);
        sow_pokes(&mut actual, &all);
        sow_pokes(&mut prefix_only, &discarded_prefix);

        assert_eq!(actual, expected);
        assert_ne!(actual, prefix_only);
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);
        let mut grid = vec![false; GRID_W * super::GRID_H];

        sow_pokes(&mut grid, &with_invalid_tail);

        assert!(grid.iter().all(|&alive| !alive));
    }

    #[test]
    fn all_invalid_newest_tail_discards_older_valid_gliders() {
        let mut with_valid_prefix = vec![(0.5, 0.5); MAX_ROOM_POKES];
        with_valid_prefix.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert_eq!(
            simulate_with_pokes(2, 0, &with_valid_prefix),
            simulate(2, 0)
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_glider_identity() {
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            simulate_with_pokes(2, 0, &with_bad_points),
            simulate_with_pokes(2, 0, &finite)
        );
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = GameOfLife::new_with(0);
        let r_def = GameOfLife::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        r0.render(&mut a, 0.3);
        r_def.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = GameOfLife::new_with(42);
        let mut c = Canvas::new(48, 24);
        r42.render(&mut c, 0.3);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = GameOfLife::new();
        let mut canvas = Canvas::new(48, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = GameOfLife::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(6, 6);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
        }
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_cells() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = GameOfLife::new();
        for (width, height) in [(usize::MAX, 12), (12, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 0.0, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_DIM * 12);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn reveal_calls_it_a_universe() {
        assert!(GameOfLife::new().reveal().contains("universe"));
    }
}
