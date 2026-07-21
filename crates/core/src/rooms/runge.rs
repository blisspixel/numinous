//! Runge phenomenon: equispaced polynomial interpolation oscillates at edges.
//!
//! DRAG: TUNE DEGREE. See `docs/ROOMS.md`.

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

fn degree(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 10.0 + s
    } else {
        3.0 + phase_unit(t) * 8.0 + s
    }
}

fn runge_f(x: f64) -> f64 {
    1.0 / (1.0 + 25.0 * x * x)
}

/// Lagrange at equispaced nodes for Runge f.
fn lagrange(x: f64, n: usize) -> f64 {
    if n < 2 {
        return runge_f(x);
    }
    let mut y = 0.0;
    for i in 0..=n {
        let xi = -1.0 + 2.0 * (i as f64) / n as f64;
        let yi = runge_f(xi);
        let mut li = 1.0;
        for j in 0..=n {
            if i == j {
                continue;
            }
            let xj = -1.0 + 2.0 * (j as f64) / n as f64;
            li *= (x - xj) / (xi - xj);
        }
        y += yi * li;
    }
    y
}

fn draw(canvas: &mut dyn Surface, deg: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = deg.round().clamp(2.0, 14.0) as usize;
    let y0 = height as f64 * 0.75;
    let y_scale = height as f64 * 0.35;
    // True Runge function.
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -1.0 + 2.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = runge_f(x);
        let py = (y0 - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
    // Interpolant.
    prev = None;
    for col in 0..width {
        let x = -1.0 + 2.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = lagrange(x, n).clamp(-2.0, 2.0);
        let py = (y0 - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Nodes.
    for i in 0..=n {
        let x = -1.0 + 2.0 * (i as f64) / n as f64;
        let y = runge_f(x);
        let px = (((x + 1.0) * 0.5) * width.saturating_sub(1) as f64).round() as i32;
        let py = (y0 - y * y_scale).round() as i32;
        canvas.line(px, py - 1, px, py + 1, 'o');
    }
    let _ = seed;
}

/// Runge phenomenon room.
#[derive(Debug, Default)]
pub struct Runge {
    seed: u64,
}

impl Runge {
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

impl Room for Runge {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "runge",
            title: "Runge Phenomenon",
            wing: "Number & Pattern",
            blurb: "Equispaced high-degree fit oscillates. t and DRAG: TUNE DEGREE.",
            accent: [180, 50, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, degree(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "runge",
            root: 659.25,
            tempo: 102,
            line: &[0, 7, 12, 5, 0, 12, 7, 3],
            encodes: "higher equispaced degree worsens edge oscillation on Runge f",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DEGREE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = degree(t, None, self.seed);
        Some(format!("n={:.0}  osc  DRAG:DEG", d.round()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = degree(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = degree(t, hands.last().copied(), self.seed);
        let n = d.round().clamp(2.0, 14.0) as usize;
        let mut err = 0.0_f64;
        for i in 0..48 {
            let x = -1.0 + 2.0 * (i as f64) / 47.0;
            err = err.max((lagrange(x, n) - runge_f(x)).abs());
        }
        Some(format!("n={n}  max|e|={err:.2}  Runge"))
    }

    fn reveal(&self) -> &'static str {
        "Interpolating 1/(1+25x^2) at equally spaced points with higher degree \
         polynomials makes the edges worse, not better. That is the Runge \
         phenomenon; Chebyshev nodes fix it."
    }
}

#[cfg(test)]
mod tests {
    use super::Runge;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Runge::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("osc"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn degree_changes() {
        let r = Runge::new();
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
        Runge::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 0);
    }
}
