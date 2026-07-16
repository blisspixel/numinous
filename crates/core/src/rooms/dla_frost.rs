//! Diffusion-Limited Aggregation: the frost that grows from random walkers.
//!
//! Walkers freeze on contact and build lightning, coral, and frost. The
//! sibling Random Walk is begging for. CLICK: PLANT A SEED. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0xD1A0_F105_7001;
const GRID_W: usize = 48;
const GRID_H: usize = 28;
const WALKERS: usize = 400;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn idx(x: usize, y: usize) -> usize {
    y * GRID_W + x
}

fn grow(seed: u64, n_walkers: usize, extra_seeds: &[(f64, f64)]) -> Vec<u8> {
    let mut grid = vec![0u8; GRID_W * GRID_H];
    // Seed cluster at bottom center.
    let sx = GRID_W / 2;
    let sy = GRID_H - 2;
    grid[idx(sx, sy)] = 1;
    for &(px, py) in extra_seeds {
        let x = ((px * (GRID_W - 1) as f64).round() as usize).min(GRID_W - 1);
        let y = ((py * (GRID_H - 1) as f64).round() as usize).min(GRID_H - 1);
        grid[idx(x, y)] = 1;
    }
    let mut rng = SplitMix64::new(SEED ^ seed);
    let mut stuck = 1 + extra_seeds.len();
    for _ in 0..n_walkers {
        // Spawn on top edge.
        let mut x = (rng.next_f64() * (GRID_W - 1) as f64) as usize;
        let mut y = 0usize;
        for _ in 0..GRID_W * GRID_H * 2 {
            // Neighbor frozen?
            let mut freeze = false;
            for (dx, dy) in [
                (-1i32, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, 1),
                (-1, 1),
                (1, -1),
            ] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < GRID_W
                    && (ny as usize) < GRID_H
                    && grid[idx(nx as usize, ny as usize)] > 0
                {
                    freeze = true;
                    break;
                }
            }
            if freeze {
                grid[idx(x, y)] = 1;
                stuck += 1;
                break;
            }
            // Random step.
            match (rng.next_u64() % 4) as u32 {
                0 => x = x.saturating_add(1).min(GRID_W - 1),
                1 => x = x.saturating_sub(1),
                2 => y = y.saturating_add(1).min(GRID_H - 1),
                _ => y = y.saturating_sub(1),
            }
        }
        let _ = stuck;
    }
    grid
}

fn draw(canvas: &mut dyn Surface, grid: &[u8]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for gy in 0..GRID_H {
        for gx in 0..GRID_W {
            if grid[idx(gx, gy)] == 0 {
                continue;
            }
            let x0 = (gx as f64 / GRID_W as f64 * width as f64).round() as i32;
            let y0 = (gy as f64 / GRID_H as f64 * height as f64).round() as i32;
            let x1 = (((gx + 1) as f64 / GRID_W as f64) * width as f64).round() as i32;
            let y1 = (((gy + 1) as f64 / GRID_H as f64) * height as f64).round() as i32;
            for yy in y0..y1.max(y0 + 1) {
                for xx in x0..x1.max(x0 + 1) {
                    canvas.plot(xx, yy, if (gx + gy) % 3 == 0 { '#' } else { '*' });
                }
            }
        }
    }
}

/// DLA Frost room.
#[derive(Debug, Default)]
pub struct DlaFrost {
    seed: u64,
}

impl DlaFrost {
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
}

impl Room for DlaFrost {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "dla-frost",
            title: "The Frost",
            wing: "Emergence",
            blurb: "Diffusion-limited aggregation: random walkers freeze on contact and grow \
                    lightning and coral. t grows the swarm; CLICK: PLANT A SEED.",
            accent: [180, 220, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = 80 + (phase_unit(t) * WALKERS as f64) as usize;
        let grid = grow(self.seed, n, &[]);
        draw(canvas, &grid);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dla frost",
            root: 123.47,
            tempo: 71,
            line: &[0, 0, 3, 8, 12, 15, 8, 3],
            encodes: "walkers freeze into branching frost and lightning",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLANT A SEED")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = 80 + (phase_unit(t) * WALKERS as f64) as usize;
        let grid = grow(self.seed, n, &[]);
        let stuck = grid.iter().filter(|&&c| c > 0).count();
        Some(format!("stuck={stuck}  walk={n}  CLICK:SEED"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = 100 + (phase_unit(t) * WALKERS as f64) as usize;
        let grid = grow(self.seed ^ hands.len() as u64, n, &hands);
        draw(canvas, &grid);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = 100 + (phase_unit(t) * WALKERS as f64) as usize;
        let grid = grow(self.seed ^ hands.len() as u64, n, &hands);
        let stuck = grid.iter().filter(|&&c| c > 0).count();
        Some(format!("SEED x{}  stuck={stuck}  walk={n}", hands.len()))
    }

    fn reveal(&self) -> &'static str {
        "Diffusion-limited aggregation lets particles wander until they touch \
         the cluster, then stick. The resulting fractal frost appears in mineral \
         dendrites, dielectric breakdown, and coral, one rule for many forms."
    }
}

#[cfg(test)]
mod tests {
    use super::DlaFrost;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DlaFrost::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("SEED"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn seed_changes() {
        let r = DlaFrost::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.6,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        DlaFrost::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(DlaFrost::new().motif().unwrap().line.len() >= 6);
    }
}
