//! Langton's Ant: chaos for ten thousand steps, then a highway forever.
//!
//! An ant on a grid follows two rules: on a white square turn right, flip it to
//! black, step forward; on a black square turn left, flip it to white, step
//! forward. For about ten thousand steps it makes a symmetric mess, and then,
//! with no change to the rules, it starts building a straight highway that never
//! ends. `t` runs the clock. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Simulation grid side (fixed, toroidal), independent of the surface.
const GRID: usize = 100;
/// The most steps `t` reaches (the highway emerges around 10,000).
const MAX_STEPS: usize = 12_000;
/// A visible opening state, before the clock grows the path toward its highway.
const ENTRY_STEPS: usize = 2_500;

/// Seed base for deterministic variation (nonzero only; var=0 path is empty start).
const SEED: u64 = 0xA4A4_0000_5EED_0001;

/// The Langton's Ant room.
#[derive(Debug, Default)]
pub struct LangtonsAnt {
    seed: u64,
}

impl LangtonsAnt {
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

    /// Steps simulated at phase `t`.
    fn steps_for(t: f64) -> usize {
        ENTRY_STEPS + (t.clamp(0.0, 1.0) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize
    }
}

/// Initialize the grid. For variation==0: completely empty (preserves all historical
/// renders, tests, and postcards exactly). For nonzero: scatter a few deterministic
/// black cells using the seed so different visits evolve differently.
fn init_grid(variation: u64) -> Vec<bool> {
    let mut grid = vec![false; GRID * GRID];
    if variation != 0 {
        let mut rng = SplitMix64::new(SEED ^ variation);
        let pre = 2 + (variation % 5) as usize;
        for _ in 0..pre {
            let idx = (rng.next_u64() as usize) % (GRID * GRID);
            grid[idx] = true;
        }
    }
    grid
}

fn starting_ant(variation: u64) -> (i32, i32, i32) {
    let side = GRID as i32;
    let dx = if variation == 0 {
        0
    } else {
        ((variation % 7) as i32) - 3
    };
    let dy = if variation == 0 {
        0
    } else {
        (((variation / 7) % 5) as i32) - 2
    };
    (side / 2 + dx, side / 2 + dy, 0)
}

/// Advance the ant `steps` from the provided starting grid (mutates and returns).
/// For nonzero variation, start position is deterministically offset so paths diverge visibly.
fn run_ant(mut grid: Vec<bool>, steps: usize, variation: u64) -> Vec<bool> {
    let side = GRID as i32;
    let (mut x, mut y, mut dir) = starting_ant(variation);
    for _ in 0..steps {
        let index = (y as usize) * GRID + x as usize;
        let black = grid[index];
        // Black: turn left; white: turn right.
        dir = if black { (dir + 3) % 4 } else { (dir + 1) % 4 };
        grid[index] = !black;
        match dir {
            0 => y -= 1,
            1 => x += 1,
            2 => y += 1,
            _ => x -= 1,
        }
        x = (x + side) % side;
        y = (y + side) % side;
    }
    grid
}

/// Run the ant for `steps` from a clean start (variation 0 path).
#[allow(dead_code)]
fn simulate(steps: usize) -> Vec<bool> {
    run_ant(init_grid(0), steps, 0)
}

/// Run with per-visit variation (pre-scatters starting squares for nonzero seeds).
fn simulate_varied(steps: usize, variation: u64) -> Vec<bool> {
    run_ant(init_grid(variation), steps, variation)
}

fn normalized_grid_cell(value: f64) -> usize {
    ((value.clamp(0.0, 1.0) * GRID as f64) as usize).min(GRID - 1)
}

fn poked_cells(pokes: &[(f64, f64)]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(|&(px, py)| {
        if px.is_finite() && py.is_finite() {
            Some((normalized_grid_cell(px), normalized_grid_cell(py)))
        } else {
            None
        }
    })
}

fn apply_poked_flips(grid: &mut [bool], pokes: &[(f64, f64)]) {
    for (gx, gy) in poked_cells(pokes) {
        let idx = gy * GRID + gx;
        grid[idx] = !grid[idx];
    }
}

fn draw_poked_cells(canvas: &mut dyn Surface, pokes: &[(f64, f64)], width: usize, height: usize) {
    let radius = (width.min(height) / 28).clamp(6, 18) as i32;
    for (gx, gy) in poked_cells(pokes) {
        let x = ((gx as f64 + 0.5) * width as f64 / GRID as f64) as i32;
        let y = ((gy as f64 + 0.5) * height as f64 / GRID as f64) as i32;
        canvas.line(x - radius, y - radius, x + radius, y - radius, '+');
        canvas.line(x - radius, y + radius, x + radius, y + radius, '+');
        canvas.line(x - radius, y - radius, x - radius, y + radius, '+');
        canvas.line(x + radius, y - radius, x + radius, y + radius, '+');
        canvas.line(x - radius, y, x + radius, y, '+');
        canvas.line(x, y - radius, x, y + radius, '+');
    }
}

/// Draw the grid state to the canvas (shared to avoid duplication between render and poked).
fn draw_grid(canvas: &mut dyn Surface, grid: &[bool], width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }
    for gy in 0..GRID {
        for gx in 0..GRID {
            if !grid[gy * GRID + gx] {
                continue;
            }
            let left = gx * width / GRID;
            let right = (((gx + 1) * width / GRID).max(left + 1)).min(width);
            let top = gy * height / GRID;
            let bottom = (((gy + 1) * height / GRID).max(top + 1)).min(height);
            for py in top..bottom {
                for px in left..right {
                    canvas.plot(px as i32, py as i32, '#');
                }
            }
        }
    }
}

impl Room for LangtonsAnt {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "langtons-ant",
            title: "Langton's Ant",
            wing: "Emergence",
            blurb: "One ant, two rules: turn on the color under you, flip it, step. It makes chaos \
                    for ten thousand steps and then builds a highway forever. t runs the clock.",
            accent: [120, 200, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let steps = Self::steps_for(t);
        let grid = simulate_varied(steps, self.seed);
        draw_grid(canvas, &grid, width, height);
    }

    fn postcard_t(&self) -> f64 {
        0.97
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "F minor highway",
            root: 174.61,
            tempo: 140,
            line: &[0, 1, 5, 6, 0, 1, 5, 12, 13, 12],
            encodes: "left-right turns suddenly locking into a highway",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FLIP A CELL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let steps = Self::steps_for(t);
        // Start from varied initial (scattered squares per seed), apply user flips to
        // the *starting* config so the ant's entire history reacts (not a post-render overlay).
        let mut grid = init_grid(self.seed);
        apply_poked_flips(&mut grid, pokes);
        let grid = run_ant(grid, steps, self.seed);
        draw_grid(canvas, &grid, width, height);
        draw_poked_cells(canvas, pokes, width, height);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let (x, y) = inputs.iter().rev().find_map(|input| match *input {
            RoomInput::PointerDown { x, y, .. } if x.is_finite() && y.is_finite() => {
                Some((normalized_grid_cell(x), normalized_grid_cell(y)))
            }
            _ => None,
        })?;
        Some(format!(
            "CELL {x},{y} FLIPPED   ANT REPLAYED {} STEPS",
            Self::steps_for(t)
        ))
    }

    fn reveal(&self) -> &'static str {
        "No matter how you scatter the starting squares, the ant always ends up \
         building the same highway. That its path is always eventually orderly is \
         proven; why it must be is still, in the deepest sense, a mystery."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GRID, LangtonsAnt, apply_poked_flips, init_grid, poked_cells, run_ant, simulate,
        simulate_varied,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};

    #[test]
    fn one_step_paints_exactly_one_cell() {
        let grid = simulate(1);
        assert_eq!(grid.iter().filter(|&&c| c).count(), 1);
    }

    #[test]
    fn the_first_four_steps_make_a_small_loop() {
        // The classic opening: after four steps four cells are black.
        let grid = simulate(4);
        assert_eq!(grid.iter().filter(|&&c| c).count(), 4);
    }

    #[test]
    fn more_steps_change_the_board() {
        assert_ne!(simulate(100), simulate(2_000));
    }

    #[test]
    fn simulate_stays_on_the_grid() {
        // A large run must not have panicked on an out-of-bounds cell.
        assert_eq!(simulate(GRID * GRID).len(), GRID * GRID);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = LangtonsAnt::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 5);

        let mut entry = Canvas::new(80, 40);
        room.render(&mut entry, 0.0);
        assert!(
            entry.ink_count() >= 30,
            "the opening state shows the ant's first visible pattern"
        );
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = LangtonsAnt::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
        }
    }

    #[test]
    fn reveal_mentions_the_highway() {
        assert!(LangtonsAnt::new().reveal().contains("highway"));
    }

    #[test]
    fn poked_cells_preserve_order_clamp_and_filter() {
        let cells: Vec<_> =
            poked_cells(&[(0.2, 0.3), (f64::NAN, 0.5), (2.0, -1.0), (0.4, 0.6)]).collect();
        assert_eq!(cells, vec![(20, 30), (GRID - 1, 0), (40, 60)]);
    }

    #[test]
    fn duplicate_flips_are_replayed_not_deduplicated() {
        let mut grid = init_grid(0);
        apply_poked_flips(&mut grid, &[(0.4, 0.6), (0.4, 0.6)]);
        assert_eq!(grid, init_grid(0));
    }

    #[test]
    fn entry_click_draws_a_legible_marker_and_status() {
        let room = LangtonsAnt::new();
        let mut base = Canvas::new(80, 40);
        let mut poked = Canvas::new(80, 40);
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[(0.5, 0.5)]);
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];

        assert!(poked.ink_count() >= base.ink_count() + 20);
        assert_eq!(
            room.status_input(0.0, &input).as_deref(),
            Some("CELL 50,50 FLIPPED   ANT REPLAYED 2500 STEPS")
        );
    }

    #[test]
    fn poked_flips_use_the_newest_bounded_finite_points() {
        let room = LangtonsAnt::new();
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.3))
            .collect();
        let mut old = vec![(0.9, 0.9); MAX_ROOM_POKES + 11];
        old.extend(newest.clone());

        let mut expected = Canvas::new(50, 30);
        let mut actual = Canvas::new(50, 30);
        room.render_poked(&mut expected, 0.5, &newest);
        room.render_poked(&mut actual, 0.5, &old);
        assert_eq!(actual.to_text(), expected.to_text());

        let mut uncapped = Canvas::new(50, 30);
        let all = old;
        room.render_poked(&mut uncapped, 0.5, &all[..all.len() - 1]);
        assert_ne!(uncapped.to_text(), expected.to_text());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_flip_identity() {
        let room = LangtonsAnt::new();
        let finite = [(0.4, 0.6)];
        let with_bad_points = [
            (f64::NAN, 0.1),
            (f64::INFINITY, 0.2),
            finite[0],
            (0.3, f64::NEG_INFINITY),
        ];
        let mut expected = Canvas::new(50, 30);
        let mut actual = Canvas::new(50, 30);
        room.render_poked(&mut expected, 0.5, &finite);
        room.render_poked(&mut actual, 0.5, &with_bad_points);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = LangtonsAnt::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES - 1];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        let mut expected = Canvas::new(50, 30);
        let mut actual = Canvas::new(50, 30);
        room.render(&mut expected, 0.5);
        room.render_poked(&mut actual, 0.5, &with_invalid_tail);
        assert_eq!(actual.to_text(), expected.to_text());

        let mut filter_first = Canvas::new(50, 30);
        room.render_poked(
            &mut filter_first,
            0.5,
            &with_invalid_tail[..MAX_ROOM_POKES - 1],
        );
        assert_ne!(filter_first.to_text(), expected.to_text());
    }

    #[test]
    fn poked_flips_are_applied_before_the_ant_runs() {
        let steps = LangtonsAnt::steps_for(0.2);
        let pokes = [(0.5, 0.5)];
        let mut pre_poked = init_grid(0);
        apply_poked_flips(&mut pre_poked, &pokes);
        let expected = run_ant(pre_poked, steps, 0);

        let mut post_poked = simulate_varied(steps, 0);
        apply_poked_flips(&mut post_poked, &pokes);

        let mut via_room_start = init_grid(0);
        apply_poked_flips(&mut via_room_start, &pokes);
        let actual = run_ant(via_room_start, steps, 0);
        assert_eq!(actual, expected);
        assert_ne!(actual, post_poked);
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = LangtonsAnt::new_with(0);
        let r_def = LangtonsAnt::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(50, 30);
        r0.render_poked(&mut cp, 0.5, &[(0.5, 0.5)]);
        assert_ne!(
            cp.to_text(),
            a.to_text(),
            "poked must visibly change output"
        );
        assert!(cp.ink_count() != a.ink_count() || cp.to_text() != a.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation_and_poked_differs() {
        let r0 = LangtonsAnt::new_with(0);
        let r42 = LangtonsAnt::new_with(42);
        let mut a = Canvas::new(50, 30);
        let mut c = Canvas::new(50, 30);
        r0.render(&mut a, 0.5);
        r42.render(&mut c, 0.5);
        assert_ne!(a.to_text(), c.to_text());

        // Pokes applied pre-run affect the evolved grid visibly.
        let mut p = Canvas::new(50, 30);
        r0.render_poked(&mut p, 0.5, &[(0.4, 0.6)]);
        assert_ne!(p.to_text(), a.to_text());
        assert!(p.ink_count() > 0);
    }
}
