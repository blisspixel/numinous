//! Devil's curve: figure-eight algebraic curve of degree four.
//!
//! DRAG: TUNE RATIO. See `docs/ROOMS.md`.

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

fn ratio(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.5 + x * 1.5 + s
    } else {
        0.8 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, ab: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    // y^2 (y^2 - a^2) = x^2 (x^2 - b^2) with a/b = ab-ish.
    let a = 1.0;
    let b = ab.clamp(0.4, 2.2);
    let scale = (width.min(height) as f64) * 0.28;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.05
    };
    let cos_r = rot.cos();
    let sin_r = rot.sin();
    // Polar form: r^2 = (a^2 sin^2 - b^2 cos^2) / (sin^2 - cos^2)
    let steps = 360;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let s2 = th.sin().powi(2);
        let c2 = th.cos().powi(2);
        let den = s2 - c2;
        if den.abs() < 0.05 {
            prev = None;
            continue;
        }
        let num = a * a * s2 - b * b * c2;
        let r2 = num / den;
        if r2 <= 0.0 || !r2.is_finite() {
            prev = None;
            continue;
        }
        let r = r2.sqrt();
        let x = r * th.cos();
        let y = r * th.sin();
        let xr = x * cos_r - y * sin_r;
        let yr = x * sin_r + y * cos_r;
        let px = (cx + xr * scale).round() as i32;
        let py = (cy - yr * scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Devil's curve room.
#[derive(Debug, Default)]
pub struct DevilCurve {
    seed: u64,
}

impl DevilCurve {
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

impl Room for DevilCurve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "devil-curve",
            title: "Devil Curve",
            wing: "Shape & Space",
            blurb: "Quartic figure-eight of Gabriele. t and DRAG: TUNE RATIO.",
            accent: [120, 30, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "devil-curve",
            root: 185.0,
            tempo: 80,
            line: &[0, 5, 3, 7, 12, 7, 3, 5],
            encodes: "devil curve: y^2(y^2-a^2)=x^2(x^2-b^2) quartic lobes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ab = ratio(t, None, self.seed);
        Some(format!("b={ab:.2}  lobes  DRAG:RATIO"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ab = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, ab, self.seed ^ hands.len() as u64);
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
        let ab = ratio(t, hands.last().copied(), self.seed);
        Some(format!("B={ab:.3}  devil"))
    }

    fn reveal(&self) -> &'static str {
        "The devil's curve is a quartic figure-eight studied by Cramer and later \
         named for its forked silhouette. In polar form it shows how two squared \
         terms balance: y^2(y^2-a^2) equals x^2(x^2-b^2)."
    }
}

#[cfg(test)]
mod tests {
    use super::DevilCurve;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DevilCurve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lobes"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = DevilCurve::new();
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
        DevilCurve::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
