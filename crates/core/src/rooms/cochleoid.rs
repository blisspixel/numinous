//! Cochleoid: snail curve r = a sin(th)/th.
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

fn scale_a(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.2 + s
    } else {
        0.5 + phase_unit(t) * 0.9 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = a.clamp(0.35, 1.8) * (width.min(height) as f64) * 0.35;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.05
    };
    let steps = 400;
    let mut prev: Option<(i32, i32)> = None;
    for i in 1..=steps {
        let th = 0.15 + 6.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = a * th.sin() / th;
        let ang = th + rot;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Cochleoid room.
#[derive(Debug, Default)]
pub struct Cochleoid {
    seed: u64,
}

impl Cochleoid {
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

impl Room for Cochleoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cochleoid",
            title: "Cochleoid",
            wing: "Shape & Space",
            blurb: "Snail curve r = a sin(th)/th. t and DRAG: TUNE A.",
            accent: [140, 80, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale_a(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cochleoid",
            root: 12.25,
            tempo: 84,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "cochleoid snail: sine over theta winds into a shell",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale_a(t, None, self.seed);
        Some(format!("a={a:.2}  snail  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale_a(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = scale_a(t, hands.last().copied(), self.seed);
        // r = a sin(th)/th; at th=pi/2, r = a * 2/pi.
        let r_half = a * 2.0 / std::f64::consts::PI;
        Some(format!("a={a:.2}  r(pi/2)={r_half:.2}  snail"))
    }

    fn reveal(&self) -> &'static str {
        "The cochleoid (snail) is the polar curve r = a sin(theta)/theta. As the \
         angle unwinds, amplitude falls like 1/theta, coiling into a shell that \
         appears in older geometry texts as a companion to the cochleoid of Pascal."
    }
}

#[cfg(test)]
mod tests {
    use super::Cochleoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cochleoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("snail"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Cochleoid::new();
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
        Cochleoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
