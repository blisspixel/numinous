//! Gamma function: Stirling path and poles at non-positive integers.
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

fn center(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        -2.0 + x * 8.0 + s
    } else {
        -1.0 + phase_unit(t) * 6.0 + s
    }
}

/// log|Gamma(x)| via Stirling for x>0.5, reflection for x near poles.
fn log_abs_gamma(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    // poles at 0, -1, -2, ...
    if x <= 0.0 {
        let n = (-x).floor();
        let frac = x + n;
        if frac.abs() < 1e-4 {
            return 8.0; // spike
        }
        // reflection: Gamma(z)Gamma(1-z)=pi/sin(pi z)
        let g1 = log_abs_gamma(1.0 - x);
        let s = (std::f64::consts::PI * x).sin().abs().max(1e-12);
        return std::f64::consts::PI.ln() - s.ln() - g1;
    }
    // Stirling: ln Gamma ~ (x-0.5)ln x - x + 0.5 ln(2pi)
    let x = x.max(0.05);
    (x - 0.5) * x.ln() - x + 0.5 * (2.0 * std::f64::consts::PI).ln()
}

fn draw(canvas: &mut dyn Surface, c: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let x0 = c - 3.5;
    let x1 = c + 3.5;
    let mut prev: Option<(i32, i32)> = None;
    let mut vals = Vec::with_capacity(width);
    for col in 0..width {
        let x = x0 + (x1 - x0) * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = log_abs_gamma(x);
        vals.push(y);
    }
    let ymin = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let ymax = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (ymax - ymin).max(1e-6);
    let pad = if seed == 0 {
        0.0
    } else {
        (seed % 3) as f64 * 0.02
    };
    for (col, &y) in vals.iter().enumerate() {
        let ny = ((y - ymin) / span + pad).clamp(0.0, 1.0);
        let py = ((1.0 - ny) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // mark center
    let mx = ((c - x0) / (x1 - x0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx, 0, mx, height as i32 - 1, '|');
}

/// Gamma function room.
#[derive(Debug, Default)]
pub struct GammaFunc {
    seed: u64,
}

impl GammaFunc {
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

impl Room for GammaFunc {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gamma-func",
            title: "Gamma Function",
            wing: "Analysis",
            blurb: "log|Gamma| with poles at nonpositive integers. t and DRAG: TUNE X.",
            accent: [100, 70, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, center(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gamma-func",
            root: 415.3,
            tempo: 74,
            line: &[0, 5, 7, 12, 10, 7, 5, 0],
            encodes: "Gamma: factorial continuum, poles at 0,-1,-2,...",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE X")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = center(t, None, self.seed);
        let g = log_abs_gamma(x);
        Some(format!("x={x:.2}  lng={g:.1}  DRAG:X"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = center(t, hands.last().copied(), self.seed);
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
        let x = center(t, hands.last().copied(), self.seed);
        Some(format!("X={x:.3}  gamma"))
    }

    fn reveal(&self) -> &'static str {
        "The Gamma function extends the factorial: Gamma(n)=(n-1)! for positive \
         integers. It has simple poles at non-positive integers. Stirling's series \
         approximates log Gamma for large positive arguments; the reflection \
         formula relates Gamma(z) to Gamma(1-z)."
    }
}

#[cfg(test)]
mod tests {
    use super::GammaFunc;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GammaFunc::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lng"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn x_changes() {
        let r = GammaFunc::new();
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
        GammaFunc::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
