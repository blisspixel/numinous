//! Error function erf: sigmoid of the Gaussian integral.
//!
//! DRAG: TUNE X. See `docs/ROOMS.md`.

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

fn mark(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        -2.5 + x * 5.0 + s
    } else {
        -2.0 + phase_unit(t) * 4.0 + s
    }
}

/// Abramowitz-Stegun style erf approximation.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let poly = ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t;
    sign * (1.0 - poly * (-x * x).exp())
}

fn draw(canvas: &mut dyn Surface, m: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let x0 = -3.0;
    let x1 = 3.0;
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = x0 + (x1 - x0) * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = erf(x);
        let py = ((0.5 - y * 0.45) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Gaussian density under the curve as dots
    for col in (0..width).step_by(2) {
        let x = x0 + (x1 - x0) * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let g = (-x * x).exp() / std::f64::consts::PI.sqrt();
        let py = ((1.0 - g * 1.4) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(col as i32, py, col as i32, py, '.');
    }
    let mx = ((m - x0) / (x1 - x0) * width.saturating_sub(1) as f64)
        .round()
        .clamp(0.0, width.saturating_sub(1) as f64) as i32;
    let my = ((0.5 - erf(m) * 0.45) * height.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx, 0, mx, height as i32 - 1, '|');
    canvas.line(mx - 2, my, mx + 2, my, 'o');
    let _ = seed;
}

/// Error function room.
#[derive(Debug, Default)]
pub struct ErrorFunction {
    seed: u64,
}

impl ErrorFunction {
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

impl Room for ErrorFunction {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "error-function",
            title: "Error Function",
            wing: "Analysis",
            blurb: "erf(x): signed Gaussian mass. t and DRAG: TUNE X.",
            accent: [60, 100, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mark(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "error-function",
            root: 369.99,
            tempo: 72,
            line: &[0, 3, 7, 10, 7, 3, 0, 5],
            encodes: "erf: integral of Gaussian, asymptotes +/-1",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE X")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = mark(t, None, self.seed);
        let e = erf(x);
        let phi = 0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2));
        Some(format!("x={x:.2}  erf={e:.2}  Phi={phi:.2}  DRAG:X"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = mark(t, hands.last().copied(), self.seed);
        draw(canvas, x, self.seed ^ hands.len() as u64);
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
        let x = mark(t, hands.last().copied(), self.seed);
        let e = erf(x);
        let phi = 0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2));
        Some(format!("erf={e:.3}  Phi={phi:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The error function erf(x) is the signed integral of a Gaussian from 0 to x, \
         scaled so that erf(+inf)=1. It is the workhorse of normal probabilities: \
         P(|Z|<=a) = erf(a/sqrt(2)) for a standard normal Z."
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorFunction;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ErrorFunction::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("erf"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn x_changes() {
        let r = ErrorFunction::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        ErrorFunction::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
