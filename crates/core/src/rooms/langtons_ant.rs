//! Langton's Ant: chaos for ten thousand steps, then a highway forever.
//!
//! An ant on a grid follows two rules: on a white square turn right, flip it to
//! black, step forward; on a black square turn left, flip it to white, step
//! forward. For about ten thousand steps it makes a symmetric mess, and then,
//! with no change to the rules, it starts building a straight highway that never
//! ends. `t` runs the clock. See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Simulation grid side (fixed, toroidal), independent of the surface.
const GRID: usize = 100;
/// The most steps `t` reaches (the highway emerges around 10,000).
const MAX_STEPS: usize = 12_000;

/// The Langton's Ant room.
#[derive(Debug, Default)]
pub struct LangtonsAnt;

impl LangtonsAnt {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Steps simulated at phase `t`.
    fn steps_for(t: f64) -> usize {
        (t.clamp(0.0, 1.0) * MAX_STEPS as f64) as usize
    }
}

/// Run the ant for `steps` and return the grid of black cells.
fn simulate(steps: usize) -> Vec<bool> {
    let mut grid = vec![false; GRID * GRID];
    let side = GRID as i32;
    let (mut x, mut y) = (side / 2, side / 2);
    // Direction: 0 up, 1 right, 2 down, 3 left.
    let mut dir = 0i32;
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
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let grid = simulate(Self::steps_for(t));
        for py in 0..height {
            for px in 0..width {
                let gx = px * GRID / width;
                let gy = py * GRID / height;
                if grid[gy * GRID + gx] {
                    canvas.plot(px as i32, py as i32, '#');
                }
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "No matter how you scatter the starting squares, the ant always ends up \
         building the same highway. That its path is always eventually orderly is \
         proven; why it must be is still, in the deepest sense, a mystery."
    }
}

#[cfg(test)]
mod tests {
    use super::{GRID, LangtonsAnt, simulate};
    use crate::canvas::Canvas;
    use crate::room::Room;

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
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = LangtonsAnt::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_the_highway() {
        assert!(LangtonsAnt::new().reveal().contains("highway"));
    }
}
