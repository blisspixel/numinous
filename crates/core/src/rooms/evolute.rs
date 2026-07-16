//! Evolute of an ellipse: envelope of normals (astroid-like for the ellipse).
//!
//! DRAG: TUNE ECCENTRICITY. See `docs/ROOMS.md`.

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

fn ecc(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.2 + x * 0.75 + s
    } else {
        0.3 + phase_unit(t) * 0.6 + s
    }
}

fn draw(canvas: &mut dyn Surface, e: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.38;
    let e = e.clamp(0.15, 0.95);
    let b = a * (1.0 - e * e).sqrt();
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // ellipse
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=120 {
        let th = std::f64::consts::TAU * (i as f64 / 120.0) + j;
        let px = (cx + a * th.cos()).round() as i32;
        let py = (cy - b * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // evolute of ellipse: (a^2-b^2)/a cos^3, (b^2-a^2)/b sin^3
    // = ae^2 cos^3, -a e^2 /sqrt(1-e^2) wait use standard
    // x = (a^2 - b^2)/a cos^3 t, y = (b^2 - a^2)/b sin^3 t
    prev = None;
    for i in 0..=180 {
        let th = std::f64::consts::TAU * (i as f64 / 180.0);
        let x = ((a * a - b * b) / a) * th.cos().powi(3);
        let y = ((b * b - a * a) / b) * th.sin().powi(3);
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Ellipse evolute room.
#[derive(Debug, Default)]
pub struct Evolute {
    seed: u64,
}

impl Evolute {
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

impl Room for Evolute {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "evolute",
            title: "Ellipse Evolute",
            wing: "Shape & Space",
            blurb: "Envelope of normals to an ellipse. t and DRAG: TUNE ECCENTRICITY.",
            accent: [80, 100, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ecc(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "evolute",
            root: 293.66,
            tempo: 78,
            line: &[0, 4, 7, 12, 7, 4, 0, 9],
            encodes: "normals envelope into a four-cusped diamond",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ECCENTRICITY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = ecc(t, None, self.seed);
        Some(format!("e={e:.2}  evolute  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = ecc(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
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
        let e = ecc(t, hands.last().copied(), self.seed);
        Some(format!("E={e:.3}  normals"))
    }

    fn reveal(&self) -> &'static str {
        "The evolute of a curve is the envelope of its normals, the locus of \
         centers of curvature. For an ellipse it is a stretched astroid with \
         four cusps: a classical conjugate of the ellipse itself."
    }
}

#[cfg(test)]
mod tests {
    use super::Evolute;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Evolute::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("evolute"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ecc_changes() {
        let r = Evolute::new();
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
        Evolute::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
