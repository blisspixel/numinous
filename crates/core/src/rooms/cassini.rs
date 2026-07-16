//! Cassini ovals: product of distances to two foci is constant.
//!
//! DRAG: TUNE B. See `docs/ROOMS.md`.

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

/// Ratio b/a: below 1 two loops, at 1 lemniscate, above 1 single oval.
fn ratio(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.9 + s
    } else {
        0.7 + phase_unit(t) * 0.7 + s
    }
}

fn draw(canvas: &mut dyn Surface, ba: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.18;
    let ba = ba.clamp(0.5, 1.6);
    let b = ba * a;
    let b2 = b * b;
    let b4 = b2 * b2;
    let a2 = a * a;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.04
    };
    let cos_r = rot.cos();
    let sin_r = rot.sin();
    // Polar form: r^2 = a^2 cos(2th) +/- sqrt(b^4 - a^4 sin^2(2th))
    let steps = 360;
    for sign in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
            let c2 = (2.0 * th).cos();
            let s2 = (2.0 * th).sin();
            let disc = b4 - a2 * a2 * s2 * s2;
            if disc < 0.0 {
                prev = None;
                continue;
            }
            let r2 = a2 * c2 + sign * disc.sqrt();
            if r2 <= 0.0 {
                prev = None;
                continue;
            }
            let r = r2.sqrt();
            let x = r * th.cos();
            let y = r * th.sin();
            let xr = x * cos_r - y * sin_r;
            let yr = x * sin_r + y * cos_r;
            let px = (cx + xr).round() as i32;
            let py = (cy - yr).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if ba < 1.0 {
                    '*'
                } else if (ba - 1.0).abs() < 0.05 {
                    '8'
                } else {
                    '#'
                };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    // Foci marks.
    let fx = a * cos_r;
    let fy = a * sin_r;
    for (sx, sy) in [(fx, fy), (-fx, -fy)] {
        let px = (cx + sx).round() as i32;
        let py = (cy - sy).round() as i32;
        canvas.line(px - 1, py, px + 1, py, 'o');
        canvas.line(px, py - 1, px, py + 1, 'o');
    }
}

/// Cassini oval room.
#[derive(Debug, Default)]
pub struct Cassini {
    seed: u64,
}

impl Cassini {
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

impl Room for Cassini {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cassini",
            title: "Cassini Ovals",
            wing: "Shape & Space",
            blurb: "Product of distances to two foci is b^2. t and DRAG: TUNE B.",
            accent: [140, 60, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cassini",
            root: 392.0,
            tempo: 84,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "product of distances to two foci fixed; b=a is lemniscate",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ba = ratio(t, None, self.seed);
        let shape = if ba < 0.95 {
            "2loop"
        } else if ba > 1.05 {
            "oval"
        } else {
            "lemni"
        };
        Some(format!("b/a={ba:.2}  {shape}  DRAG:B"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ba = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, ba, self.seed ^ hands.len() as u64);
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
        let ba = ratio(t, hands.last().copied(), self.seed).clamp(0.5, 1.6);
        // b/a < 1 two loops, =1 lemniscate, >1 single oval.
        let shape = if (ba - 1.0).abs() < 0.03 {
            "lemniscate"
        } else if ba < 1.0 {
            "two loops"
        } else {
            "one oval"
        };
        let dphi = (ba - 1.0).abs();
        Some(format!("b/a={ba:.2}  |b/a-1|={dphi:.2}  {shape}"))
    }

    fn reveal(&self) -> &'static str {
        "Cassini ovals are loci where the product of distances to two foci equals \
         b^2. When b equals the half-distance a, the curve is Bernoulli's \
         lemniscate. Giovanni Cassini once preferred them over Kepler ellipses."
    }
}

#[cfg(test)]
mod tests {
    use super::Cassini;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cassini::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("b/a"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Cassini::new();
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
        Cassini::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
