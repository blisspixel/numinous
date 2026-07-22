//! Pseudosphere: surface of constant negative curvature (tractrix revolution).
//!
//! DRAG: TUNE FLARE. See `docs/ROOMS.md`.

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

fn flare(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.5 + x * 1.2 + s
    } else {
        0.7 + phase_unit(t) * 0.9 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    // Fixed scale so flare (a) changes the waist vs rim, not a cancelled self-copy.
    let a = a.clamp(0.55, 1.85);
    let scale = (height as f64) * 0.32;
    // Tractrix: x = a sin u, z = a (cos u + ln tan(u/2)), u in (0, pi).
    let steps = 160;
    let mut prev_l: Option<(i32, i32)> = None;
    let mut prev_r: Option<(i32, i32)> = None;
    let u0 = 0.22
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.03
        };
    let u1 = std::f64::consts::PI - u0;
    for i in 0..=steps {
        let u = u0 + (u1 - u0) * (i as f64 / steps as f64);
        let r = a * u.sin();
        let tan_half = (u * 0.5).tan().abs().max(1e-6);
        let z = a * (u.cos() + tan_half.ln());
        let py = (cy - z * scale * 0.42).round() as i32;
        let pl = (cx - r * scale).round() as i32;
        let pr = (cx + r * scale).round() as i32;
        if let Some((ox, oy)) = prev_l {
            canvas.line(ox, oy, pl, py, '#');
            canvas.line(ox, oy + 1, pl, py + 1, '*');
        }
        if let Some((ox, oy)) = prev_r {
            canvas.line(ox, oy, pr, py, '#');
            canvas.line(ox, oy + 1, pr, py + 1, '*');
        }
        prev_l = Some((pl, py));
        prev_r = Some((pr, py));
    }
    // Latitude rings: denser near the rim so the flare reads as a solid of revolution.
    for k in 0..6 {
        let u = u0 + (u1 - u0) * ((k as f64 + 0.5) / 6.0);
        let r = a * u.sin();
        let tan_half = (u * 0.5).tan().abs().max(1e-6);
        let z = a * (u.cos() + tan_half.ln());
        let py = (cy - z * scale * 0.42).round() as i32;
        let rx = r * scale;
        let ry = rx * 0.28;
        let mut prev: Option<(i32, i32)> = None;
        for j in 0..=48 {
            let th = 2.0 * std::f64::consts::PI * (j as f64 / 48.0);
            let px = (cx + rx * th.cos()).round() as i32;
            let qy = (py as f64 + ry * th.sin()).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, qy, '.');
            }
            prev = Some((px, qy));
        }
    }
}

/// Pseudosphere room.
#[derive(Debug, Default)]
pub struct Pseudosphere {
    seed: u64,
}

impl Pseudosphere {
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

impl Room for Pseudosphere {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pseudosphere",
            title: "Pseudosphere",
            wing: "Shape & Space",
            blurb: "Constant K=-1 from a spun tractrix. t and DRAG: TUNE FLARE.",
            accent: [120, 50, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, flare(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pseudosphere",
            root: 246.94,
            tempo: 76,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "tractrix revolution yields Gaussian curvature constant negative",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE FLARE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = flare(t, None, self.seed);
        Some(format!("a={a:.2}  K=-1  DRAG:FLARE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = flare(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = flare(t, hands.last().copied(), self.seed);
        // Pseudosphere: Gaussian K = -1/a^2; tractrix generator length a.
        let k = -1.0 / (a * a).max(1e-9);
        Some(format!("a={a:.2}  K={k:.2}  tractrix"))
    }

    fn reveal(&self) -> &'static str {
        "Spin a tractrix about its asymptote and you get a pseudosphere: every \
         point has Gaussian curvature -1. It is a local model of hyperbolic \
         geometry, though it cannot cover the whole plane without singularity."
    }
}

#[cfg(test)]
mod tests {
    use super::Pseudosphere;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Pseudosphere::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("K=-1"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn flare_changes() {
        let r = Pseudosphere::new();
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
        Pseudosphere::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
