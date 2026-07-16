//! Brownian motion: discrete random walk path converging to Wiener process.
//!
//! DRAG: TUNE STEPS. See `docs/ROOMS.md`.

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

fn steps(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 40) as f64 };
    if let Some((x, _)) = hand {
        40.0 + x * 200.0 + s
    } else {
        60.0 + phase_unit(t) * 160.0 + s
    }
}

fn next_u01(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 33) as f64) / ((1u64 << 31) as f64)
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(20.0, 320.0) as usize;
    let mut state = if seed == 0 { 0xcafef00d_u64 } else { seed };
    let dt = 1.0 / n as f64;
    let sigma = dt.sqrt();
    let mut w = 0.0_f64;
    let mut max_abs = 1e-6_f64;
    let mut path = Vec::with_capacity(n + 1);
    path.push(0.0);
    for _ in 0..n {
        // Box-Muller half
        let u1 = next_u01(&mut state).clamp(1e-12, 1.0 - 1e-12);
        let u2 = next_u01(&mut state);
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        w += sigma * z;
        max_abs = max_abs.max(w.abs());
        path.push(w);
    }
    let cy = height as f64 * 0.5;
    let y_scale = height as f64 * 0.42 / max_abs;
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
    let mut prev: Option<(i32, i32)> = None;
    for (i, &wi) in path.iter().enumerate() {
        let x = ((i as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = (cy - wi * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
}

/// Brownian motion room.
#[derive(Debug, Default)]
pub struct Brownian {
    seed: u64,
}

impl Brownian {
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

impl Room for Brownian {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "brownian",
            title: "Brownian Motion",
            wing: "Chance & Order",
            blurb: "Wiener path from Gaussian steps. t and DRAG: TUNE STEPS.",
            accent: [80, 80, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, steps(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "brownian",
            root: 24.5,
            tempo: 86,
            line: &[0, 2, 5, 3, 7, 12, 5, 0],
            encodes: "Brownian: continuous nowhere-smooth path, variance grows as t",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE STEPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = steps(t, None, self.seed).round();
        Some(format!("n={n:.0}  W(t)  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = steps(t, hands.last().copied(), self.seed);
        // Reseed from poke so path visibly changes.
        let seed = self.seed
            ^ hands
                .last()
                .map(|&(x, y)| ((x * 1e6) as u64) ^ ((y * 1e6) as u64))
                .unwrap_or(0)
            ^ hands.len() as u64;
        draw(canvas, n, seed);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let n = steps(t, hands.last().copied(), self.seed).round();
        Some(format!("N={n:.0}  brown"))
    }

    fn reveal(&self) -> &'static str {
        "Brownian motion (the Wiener process) is the scaling limit of random \
         walks: continuous paths that are almost surely nowhere differentiable. \
         Variance grows like time; it is the backbone of diffusion and Ito calculus."
    }
}

#[cfg(test)]
mod tests {
    use super::Brownian;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Brownian::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("W(t)"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn steps_change() {
        let r = Brownian::new();
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
        Brownian::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
