//! Cauchy-Lorentz density: heavy tails and no mean.
//!
//! DRAG: TUNE WIDTH. See `docs/ROOMS.md`.

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

fn gamma(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.15 + x * 1.8 + s
    } else {
        0.3 + phase_unit(t) * 1.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, g: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let g = g.clamp(0.12, 2.2);
    let x0 = if seed == 0 {
        0.0
    } else {
        ((seed % 5) as f64 - 2.0) * 0.15
    };
    // f(x) = 1/(pi g (1 + ((x-x0)/g)^2))
    let peak = 1.0 / (std::f64::consts::PI * g);
    let y0 = height as f64 * 0.92;
    let y_scale = height as f64 * 0.8 / peak.max(0.1);
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -4.0 + 8.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let u = (x - x0) / g;
        let f = 1.0 / (std::f64::consts::PI * g * (1.0 + u * u));
        let py = (y0 - f * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Compare a same-height Gaussian (lighter tails) as dots.
    let sig = g;
    let gpeak = 1.0 / (sig * (2.0 * std::f64::consts::PI).sqrt());
    let g_scale = height as f64 * 0.8 / gpeak.max(0.1);
    prev = None;
    for col in (0..width).step_by(2) {
        let x = -4.0 + 8.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let z = (x - x0) / sig;
        let f = gpeak * (-0.5 * z * z).exp();
        let py = (y0 - f * g_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
}

/// Cauchy-Lorentz room.
#[derive(Debug, Default)]
pub struct CauchyLorentz {
    seed: u64,
}

impl CauchyLorentz {
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

impl Room for CauchyLorentz {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cauchy-lorentz",
            title: "Cauchy Lorentz",
            wing: "Chance & Order",
            blurb: "Heavy-tailed density with no mean. t and DRAG: TUNE WIDTH.",
            accent: [120, 40, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, gamma(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cauchy-lorentz",
            root: 116.54,
            tempo: 96,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "Cauchy-Lorentz: heavy tails, undefined mean and variance",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE WIDTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gamma(t, None, self.seed);
        Some(format!("g={g:.2}  heavy  DRAG:WID"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gamma(t, hands.last().copied(), self.seed);
        draw(canvas, g, self.seed ^ hands.len() as u64);
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
        let g = gamma(t, hands.last().copied(), self.seed);
        Some(format!("G={g:.3}  cauchy"))
    }

    fn reveal(&self) -> &'static str {
        "The Cauchy (Lorentz) density falls like 1/x^2, so tails are so heavy the \
         mean and variance do not exist. Sample averages wander forever; spectral \
         lines and ratio-of-normals noise wear this shape."
    }
}

#[cfg(test)]
mod tests {
    use super::CauchyLorentz;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CauchyLorentz::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("heavy"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn width_changes() {
        let r = CauchyLorentz::new();
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
        CauchyLorentz::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
