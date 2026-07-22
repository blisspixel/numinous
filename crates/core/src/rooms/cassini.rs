//! Cassini ovals: product of distances to two foci is constant.
//!
//! Ambient phase draws the ovals with a pen. DRAG: TUNE B.
//! See `docs/ROOMS.md`.

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
fn ratio(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.7 + x * 0.8 + s
    } else {
        // Ambient b/a holds a readable oval family; motion lives in the pen.
        1.05 + s
    }
}

fn draw(canvas: &mut dyn Surface, ba: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.24;
    let ba = ba.clamp(0.65, 1.6);
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
    let show = show.clamp(0.0, 1.0);
    // Polar form: r^2 = a^2 cos(2th) +/- sqrt(b^4 - a^4 sin^2(2th))
    let steps = 480;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    let ch = if ba < 1.0 {
        '*'
    } else if (ba - 1.0).abs() < 0.05 {
        '8'
    } else {
        '#'
    };
    // Soft ghost of full branches.
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
            let py = (cy - yr * 0.9).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, '.');
            }
            prev = Some((px, py));
        }
    }
    // Bright path so far.
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for sign in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=drawn {
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
            let py = (cy - yr * 0.9).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, ch);
                canvas.line(ox, oy + 1, px, py + 1, '.');
            }
            if sign > 0.0 {
                tip = (px, py);
            }
            prev = Some((px, py));
        }
    }
    // Foci: filled blots, not reticle crosses.
    let fx = a * cos_r;
    let fy = a * sin_r;
    for (sx, sy) in [(fx, fy), (-fx, -fy)] {
        let px = (cx + sx).round() as i32;
        let py = (cy - sy * 0.9).round() as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                canvas.plot(px + dx, py + dy, 'o');
            }
        }
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
            }
        }
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
            blurb: "Two-foci product curves draw themselves. Watch the pen; DRAG: TUNE B.",
            accent: [140, 60, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), phase_unit(t), self.seed);
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
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("b/a={ba:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ba = ratio(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, ba, show, self.seed ^ hands.len() as u64);
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
        assert!(s.contains("DRAG") || s.contains("draw"));
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
    fn ambient_pen_moves_the_plate() {
        let r = Cassini::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the ovals");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Cassini::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
