//! Conway's Game of Life: a universe from four rules.
//!
//! Each cell lives or dies based only on how many of its eight neighbors are
//! alive. From a random soup, gliders, oscillators, and still lifes emerge. `t`
//! sweeps the generation shown, so the life evolves as you scrub. The simulation
//! runs on a fixed toroidal grid and is sampled onto the surface, so the work is
//! bounded no matter how large the surface is. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Simulation grid width and height (fixed, independent of the surface).
const GRID_W: usize = 96;
const GRID_H: usize = 96;
/// Fixed seed so the soup reproduces exactly.
const SEED: u64 = 0x11FE_0DED_5EED_600D;
/// Fraction of cells alive in the initial soup.
const DENSITY: f64 = 0.32;
/// The most generations `t` reaches.
const MAX_GEN: usize = 140;

/// The Game of Life room.
#[derive(Debug, Default)]
pub struct GameOfLife;

impl GameOfLife {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The generation shown at phase `t`.
    fn generation_for(t: f64) -> usize {
        (t.clamp(0.0, 1.0) * MAX_GEN as f64).round() as usize
    }
}

impl Room for GameOfLife {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "game-of-life",
            title: "Game of Life",
            wing: "Emergence",
            blurb: "A cell lives or dies from four tiny rules about its neighbors; a random soup \
                    breeds gliders and oscillators. t sweeps the generation, so the life evolves.",
            accent: [90, 210, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let grid = simulate(Self::generation_for(t));
        for py in 0..height {
            for px in 0..width {
                let gx = px * GRID_W / width;
                let gy = py * GRID_H / height;
                if grid[gy * GRID_W + gx] {
                    canvas.plot(px as i32, py as i32, '*');
                }
            }
        }
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

/// The initial soup, seeded deterministically.
fn seed() -> Vec<bool> {
    let mut rng = SplitMix64::new(SEED);
    (0..GRID_W * GRID_H)
        .map(|_| rng.next_f64() < DENSITY)
        .collect()
}

/// Run the Game of Life for `generations` steps from the seed.
fn simulate(generations: usize) -> Vec<bool> {
    let mut grid = seed();
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
    use super::{GameOfLife, step};
    use crate::canvas::Canvas;
    use crate::room::Room;

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
    fn render_is_deterministic() {
        let room = GameOfLife::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
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
        }
    }

    #[test]
    fn reveal_calls_it_a_universe() {
        assert!(GameOfLife::new().reveal().contains("universe"));
    }
}
