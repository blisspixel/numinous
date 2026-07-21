//! Brewster's angle: reflection vanishes for p-pol at tan i = n2/n1.
//!
//! DRAG: TUNE ANGLE. See `docs/ROOMS.md`.

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

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.1 + x * 1.3 + s
    } else {
        0.2 + phase_unit(t) * 1.1 + s
    }
}

fn fresnel_r(i: f64, n1: f64, n2: f64) -> f64 {
    // intensity reflectance average of s and p (toy)
    let sin_r = (n1 / n2) * i.sin();
    if sin_r > 1.0 {
        return 1.0;
    }
    let r = sin_r.asin();
    // rp vanishes at brewster
    let rp_num = n2 * i.cos() - n1 * r.cos();
    let rp_den = n2 * i.cos() + n1 * r.cos();
    let rs_num = n1 * i.cos() - n2 * r.cos();
    let rs_den = n1 * i.cos() + n2 * r.cos();
    let rp = if rp_den.abs() < 1e-12 {
        0.0
    } else {
        (rp_num / rp_den).powi(2)
    };
    let rs = if rs_den.abs() < 1e-12 {
        1.0
    } else {
        (rs_num / rs_den).powi(2)
    };
    0.5 * (rp + rs)
}

fn draw(canvas: &mut dyn Surface, i_ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n1 = 1.0;
    let n2 = 1.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.05
        };
    let brewster = (n2 / n1).atan();
    // plot R vs angle as curve
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let u = col as f64 / width.saturating_sub(1).max(1) as f64;
        let ang = u * 1.4; // 0..~80 deg
        let r = fresnel_r(ang, n1, n2).clamp(0.0, 1.0);
        let py = ((1.0 - r) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round()
            as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // brewster mark
    let bu = (brewster / 1.4).clamp(0.0, 1.0);
    let bx = (bu * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(bx, 0, bx, height.saturating_sub(1) as i32, '|');
    // current angle mark
    let iu = (i_ang.clamp(0.0, 1.4) / 1.4).clamp(0.0, 1.0);
    let ix = (iu * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(ix, 0, ix, height.saturating_sub(1) as i32, '+');
}

/// Brewster angle room.
#[derive(Debug, Default)]
pub struct Brewster {
    seed: u64,
}

impl Brewster {
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

impl Room for Brewster {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "brewster",
            title: "Brewster Angle",
            wing: "Waves & Sound",
            blurb: "Fresnel reflectance dips at tan i = n2/n1. t and DRAG: TUNE ANGLE.",
            accent: [160, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "brewster",
            root: 739.99,
            tempo: 82,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "p-pol reflection vanishes at brewster angle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let i = angle(t, None, self.seed);
        let ib = 1.5_f64.atan();
        let d = (i - ib).abs();
        Some(format!("i={i:.2}  iB={ib:.2}  d={d:.2}  DRAG:ANG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let i = angle(t, hands.last().copied(), self.seed);
        draw(canvas, i, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let i = angle(t, hands.last().copied(), self.seed);
        let ib = 1.5_f64.atan();
        let d = (i - ib).abs();
        let pol = if d < 0.05 { "p-pol zero" } else { "Rp>0" };
        Some(format!("i={i:.2}  iB={ib:.2}  {pol}"))
    }

    fn reveal(&self) -> &'static str {
        "At Brewster's angle tan i_B = n2/n1, p-polarized light has zero \
         reflection. Sunglass glare cuts and laser windows exploit this. The \
         reflected ray is then fully s-polarized."
    }
}

#[cfg(test)]
mod tests {
    use super::Brewster;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Brewster::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("brew"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = Brewster::new();
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
        Brewster::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
