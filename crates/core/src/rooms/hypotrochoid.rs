//! Hypotrochoid: roulette of a circle rolling inside a fixed circle (Spirograph).
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
    // R/r style via k = R/r
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        2.0 + x * 6.0 + s
    } else {
        3.0 + phase_unit(t) * 4.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.12;
    let big = k * r / (k - 1.0).max(1.1); // fixed circle radius style
    let rr = big / k.max(1.1);
    let d = rr
        * (0.6
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.1
            });
    // x = (R-r) cos t + d cos((R-r)/r t)
    let r_diff = big - rr;
    let scale = (width.min(height) as f64) * 0.4 / (r_diff + d).max(1.0);
    let steps = 800;
    let mut prev: Option<(i32, i32)> = None;
    let turns = 4.0;
    for i in 0..=steps {
        let th = turns * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_diff * th.cos() + d * ((r_diff / rr) * th).cos();
        let y = r_diff * th.sin() - d * ((r_diff / rr) * th).sin();
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Hypotrochoid room.
#[derive(Debug, Default)]
pub struct Hypotrochoid {
    seed: u64,
}

impl Hypotrochoid {
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

impl Room for Hypotrochoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hypotrochoid",
            title: "Hypotrochoid",
            wing: "Shape & Space",
            blurb: "Spirograph: circle rolls inside a circle. t and DRAG: TUNE RATIO.",
            accent: [200, 60, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hypotrochoid",
            root: 261.63,
            tempo: 108,
            line: &[0, 5, 10, 12, 7, 2, 9, 0],
            encodes: "inner rolling ratio sets petal count",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = ratio(t, None, self.seed);
        Some(format!("R/r={k:.1}  spiro  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
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
        let k = ratio(t, hands.last().copied(), self.seed);
        Some(format!("R/r={k:.2}  roulette"))
    }

    fn reveal(&self) -> &'static str {
        "A hypotrochoid is the path of a point attached to a circle rolling \
         inside a fixed circle. Rational radius ratios close into Spirograph \
         petals; irrationals dense-fill a ring."
    }
}

#[cfg(test)]
mod tests {
    use super::Hypotrochoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hypotrochoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("spiro"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Hypotrochoid::new();
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
        Hypotrochoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
