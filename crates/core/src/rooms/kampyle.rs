//! Kampyle of Eudoxus: classical horn curve x^4 = a^2 (x^2 + y^2).
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

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

fn param_a(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.35 + x * 0.6 + s
    } else {
        0.45 + phase_unit(t) * 0.45 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = a.clamp(0.3, 1.05);
    let scale = (width.min(height) as f64) * 0.4 * a;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 8) as f64 * 0.04
    };
    // Polar: r = a / cos^2(th) for |th| < pi/2.
    let steps = 200;
    for sign in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let u = i as f64 / steps as f64;
            let th = (-0.9 + 1.8 * u) * std::f64::consts::FRAC_PI_2;
            let c = th.cos();
            if c.abs() < 0.12 {
                prev = None;
                continue;
            }
            let r = a / (c * c);
            if r > 4.0 {
                prev = None;
                continue;
            }
            let ang = th + rot;
            let x = sign * r * ang.cos();
            let y = r * ang.sin();
            let px = (cx + x * scale).round() as i32;
            let py = (cy - y * scale).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, if sign > 0.0 { '#' } else { '*' });
            }
            prev = Some((px, py));
        }
    }
}

/// Kampyle room.
#[derive(Debug, Default)]
pub struct Kampyle {
    seed: u64,
}

impl Kampyle {
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

impl Room for Kampyle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kampyle",
            title: "Kampyle of Eudoxus",
            wing: "Shape & Space",
            blurb: "Horn curve x^4 = a^2 (x^2+y^2). t and DRAG: TUNE A.",
            accent: [140, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, param_a(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kampyle",
            root: 174.61,
            tempo: 82,
            line: &[0, 2, 7, 9, 12, 9, 7, 2],
            encodes: "Eudoxus horn: r = a sec^2 theta classical quartic",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = param_a(t, None, self.seed);
        Some(format!("a={a:.2}  horn  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = param_a(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = param_a(t, hands.last().copied(), self.seed);
        // Kampyle of Eudoxus: at y=0, x=+-a; asymptotic opening ~ a.
        Some(format!("a={a:.2}  waist={a:.2}  x^4=a^2 r^2"))
    }

    fn reveal(&self) -> &'static str {
        "The kampyle of Eudoxus is a horn-shaped quartic known since antiquity: \
         x^4 = a^2 (x^2 + y^2). In polar form r = a sec^2 theta it flares as the \
         angle nears the asymptotes."
    }
}

#[cfg(test)]
mod tests {
    use super::Kampyle;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Kampyle::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("horn"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Kampyle::new();
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
        Kampyle::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
