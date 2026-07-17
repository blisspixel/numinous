//! Site percolation on a square grid: open sites and cluster flood.
//!
//! DRAG: TUNE OPEN PROB. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn open_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        (x * 0.9 + 0.05 + s).clamp(0.05, 0.95)
    } else {
        // Sweep through critical ~0.59 for square site percolation
        (0.35 + phase_unit(t) * 0.4 + s).clamp(0.05, 0.95)
    }
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) -> f64 {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return 0.0;
    }
    let w = width.min(64);
    let h = height.min(32);
    let mut open = vec![false; w * h];
    let mut state = seed ^ 0xC0FF_EE00_D15E_A5E5;
    let mut next_u = || {
        state = state.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1);
        (state >> 33) as f64 / (u32::MAX as f64)
    };
    for cell in &mut open {
        *cell = next_u() < p;
    }
    // Flood from left edge: connected open cluster
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    for y in 0..h {
        let i = y * w;
        if open[i] {
            stack.push(i);
            seen[i] = true;
        }
    }
    let mut cluster = 0usize;
    while let Some(i) = stack.pop() {
        cluster += 1;
        let x = i % w;
        let y = i / w;
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if open[j] && !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    let mut right_touch = false;
    for y in 0..h {
        if seen[y * w + (w - 1)] {
            right_touch = true;
            break;
        }
    }
    for y in 0..height {
        for x in 0..width {
            let gx = (x * w / width.max(1)).min(w - 1);
            let gy = (y * h / height.max(1)).min(h - 1);
            let i = gy * w + gx;
            let ch = if seen[i] {
                if right_touch { '#' } else { '*' }
            } else if open[i] {
                '+'
            } else {
                ' '
            };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
    cluster as f64 / (w * h) as f64
}

/// Site percolation room.
#[derive(Debug, Default)]
pub struct Percolation {
    seed: u64,
}

impl Percolation {
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

impl Room for Percolation {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "percolation",
            title: "Percolation",
            wing: "Chance & Order",
            blurb: "Open sites on a grid until a path crosses. t and DRAG: TUNE OPEN PROB.",
            accent: [40, 120, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let _ = draw(canvas, open_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "percolation",
            root: 164.81,
            tempo: 86,
            line: &[0, 3, 5, 8, 10, 8, 5, 3],
            encodes: "open sites meet a spanning path at criticality",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE OPEN PROB")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = open_p(t, None, self.seed);
        Some(format!("p={p:.2}  perc  DRAG:OPEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = open_p(t, hands.last().copied(), self.seed);
        let _ = draw(canvas, p, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = open_p(t, hands.last().copied(), self.seed);
        // Square site percolation threshold (accepted estimate).
        let pc = 0.592_746_f64;
        let delta = p - pc;
        let side = if delta > 0.02 {
            "above"
        } else if delta < -0.02 {
            "below"
        } else {
            "near"
        };
        Some(format!("p={p:.3}  pc={pc:.3}  {side}"))
    }

    fn reveal(&self) -> &'static str {
        "In site percolation each cell is open with probability p. Below a \
         critical p_c there is no left-right open path; above it, one appears. \
         Square-site p_c is about 0.5927: a sharp phase transition in the plane."
    }
}

#[cfg(test)]
mod tests {
    use super::Percolation;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Percolation::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("OPEN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = Percolation::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Percolation::new().render(&mut c, 0.65);
        assert!(c.ink_count() > 0);
    }
}
