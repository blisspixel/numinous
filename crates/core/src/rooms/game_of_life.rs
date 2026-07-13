//! Conway's Game of Life: a universe from four rules.
//!
//! Each cell lives or dies based only on how many of its eight neighbors are
//! alive. From a random soup, gliders, oscillators, and still lifes emerge. `t`
//! sweeps the generation shown, so the life evolves as you scrub. The simulation
//! runs on a fixed toroidal grid and is sampled onto the surface, so the work is
//! bounded no matter how large the surface is. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

/// Simulation grid width and height (fixed, independent of the surface).
const GRID_W: usize = 96;
const GRID_H: usize = 96;
/// Fixed seed so the soup reproduces exactly.
const SEED: u64 = 0x11FE_0DED_5EED_600D;
/// Fraction of cells alive in the initial soup.
const DENSITY: f64 = 0.32;
/// The most generations `t` reaches.
const MAX_GEN: usize = 140;

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
}

impl Room for GameOfLife {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "game-of-life",
            title: "Game of Life",
            wing: "Emergence",
            blurb: "Each dot is a living cell. Click to launch a five-cell glider: cells are born \
                    with 3 neighbors and survive with 2 or 3, so the small shape moves by itself.",
            accent: [90, 210, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let grid = simulate(Self::generation_for(t), self.seed);
        draw_grid(canvas, &grid);
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
        Some(format!(
            "GEN {}   BORN 3   SURVIVES 2 OR 3",
            Self::generation_for(t)
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let gliders = launch_events(inputs).len();
        if gliders == 0 {
            return self.status(t);
        }
        Some(format!(
            "GEN {}   {gliders} GLIDER{} LAUNCHED",
            Self::generation_for(t),
            if gliders == 1 { "" } else { "S" }
        ))
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: LAUNCH A 5-CELL GLIDER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // Keep the random universe as a faint reference and brighten only the
        // living cells the player's launch changed. The glider still obeys the
        // same B3/S23 evolution, but its consequence is now possible to follow.
        let generations = Self::generation_for(t);
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        let launches: Vec<_> = pokes[start..]
            .iter()
            .filter_map(|&(x, y)| {
                (x.is_finite() && y.is_finite()).then_some(GliderLaunch {
                    point: (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
                    generation: generations,
                })
            })
            .collect();
        if launches.is_empty() {
            self.render(canvas, t);
            return;
        }
        let base = simulate(generations, self.seed);
        let grid = simulate_with_launches(generations, self.seed, &launches);
        draw_grid_mark(canvas, &base, '-');
        draw_grid_difference(canvas, &base, &grid);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let launches = launch_events(inputs);
        if launches.is_empty() {
            self.render(canvas, t);
            return;
        }
        let generations = Self::generation_for(t);
        let base = simulate(generations, self.seed);
        let grid = simulate_with_launches(generations, self.seed, &launches);
        draw_grid_mark(canvas, &base, '-');
        draw_grid_difference(canvas, &base, &grid);
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

fn draw_grid(canvas: &mut dyn Surface, grid: &[bool]) {
    draw_grid_mark(canvas, grid, '*');
}

fn draw_grid_mark(canvas: &mut dyn Surface, grid: &[bool], mark: char) {
    let Some((width, height)) = drawing_dims(canvas) else {
        return;
    };
    for py in 0..height {
        for px in 0..width {
            let gx = px * GRID_W / width;
            let gy = py * GRID_H / height;
            if grid[gy * GRID_W + gx] {
                canvas.plot(px as i32, py as i32, mark);
            }
        }
    }
}

fn draw_grid_difference(canvas: &mut dyn Surface, base: &[bool], changed: &[bool]) {
    let Some((width, height)) = drawing_dims(canvas) else {
        return;
    };
    for py in 0..height {
        for px in 0..width {
            let gx = px * GRID_W / width;
            let gy = py * GRID_H / height;
            let index = gy * GRID_W + gx;
            if changed[index] && changed[index] != base[index] {
                canvas.plot(px as i32, py as i32, '#');
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

fn simulate_with_launches(
    generations: usize,
    variation: u64,
    launches: &[GliderLaunch],
) -> Vec<bool> {
    let generations = generations.min(MAX_GEN);
    let mut grid = seed(variation);
    let mut current = 0;
    for launch in launches {
        let target = launch.generation.min(generations).max(current);
        for _ in current..target {
            grid = step(&grid, GRID_W, GRID_H);
        }
        plant_glider(&mut grid, launch.point);
        current = target;
    }
    for _ in current..generations {
        grid = step(&grid, GRID_W, GRID_H);
    }
    grid
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
    (0..GRID_W * GRID_H)
        .map(|_| rng.next_f64() < DENSITY)
        .collect()
}

/// Run the Game of Life for `generations` steps from the seed.
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
    next
}

#[cfg(test)]
mod tests {
    use super::{
        GRID_W, GameOfLife, GliderLaunch, glider_on_empty_grid, launch_events, simulate,
        simulate_with_launches, simulate_with_pokes, sow_pokes, sown_glider_cells, step,
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
