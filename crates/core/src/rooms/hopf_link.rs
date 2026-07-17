//! Hopf link: two unknotted circles linking once.
//!
//! DRAG: TUNE PHI. See `docs/ROOMS.md`.

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
        (seed % 5) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::TAU + s
    } else {
        phase_unit(t) * std::f64::consts::TAU + s
    }
}

fn draw(canvas: &mut dyn Surface, phi: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.28;
    let sep = r * 0.55;
    let tilt = if seed == 0 {
        0.55
    } else {
        0.4 + (seed % 4) as f64 * 0.05
    };
    // circle A in xy
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=100 {
        let th = std::f64::consts::TAU * (i as f64) / 100.0 + phi;
        let x = r * th.cos() - sep * 0.3;
        let y = r * th.sin() * tilt;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // circle B in xz, linked
    prev = None;
    for i in 0..=100 {
        let th = std::f64::consts::TAU * (i as f64) / 100.0;
        let x = r * th.cos() * tilt * 0.3 + sep * 0.2;
        let z = r * th.sin();
        let y = z * 0.85;
        let px = (cx + x + z * 0.25).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '=');
        }
        prev = Some((px, py));
    }
}

/// Hopf link room.
#[derive(Debug, Default)]
pub struct HopfLink {
    seed: u64,
}

impl HopfLink {
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

impl Room for HopfLink {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hopf-link",
            title: "Hopf Link",
            wing: "Shape & Space",
            blurb: "Two circles, each through the other once. t and DRAG: TUNE PHI.",
            accent: [90, 70, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hopf-link",
            root: 138.59,
            tempo: 68,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "Hopf link: simplest nontrivial 2-component link, lk=1",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHI")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = angle(t, None, self.seed);
        Some(format!("phi={p:.2}  lk=1  DRAG:PHI"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = angle(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = angle(t, hands.last().copied(), self.seed);
        let deg = (p.rem_euclid(1.0) * 360.0).floor() as i32;
        Some(format!("ph={deg}deg  Lk=1  hopf"))
    }

    fn reveal(&self) -> &'static str {
        "The Hopf link is two unknotted circles with linking number one: each \
         goes through the other exactly once. It is the simplest nontrivial \
         two-component link and the fiber pair of the Hopf fibration of S3."
    }
}

#[cfg(test)]
mod tests {
    use super::HopfLink;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HopfLink::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lk"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phi_changes() {
        let r = HopfLink::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        HopfLink::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
