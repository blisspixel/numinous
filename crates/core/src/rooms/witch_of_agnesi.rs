//! Witch of Agnesi: classical cubic curve y = 8 a^3 / (x^2 + 4 a^2).
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

fn a_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.3 + x * 1.2 + s
    } else {
        0.5 + phase_unit(t) * 0.8 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = a.clamp(0.2, 2.0);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let x_span = 6.0 * a;
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -x_span + 2.0 * x_span * (col as f64 / width.saturating_sub(1).max(1) as f64) + j;
        let y = 8.0 * a * a * a / (x * x + 4.0 * a * a);
        let u = (col as f64 / width.saturating_sub(1).max(1) as f64).clamp(0.0, 1.0);
        let v = (y / (2.0 * a)).clamp(0.0, 1.0); // peak is 2a at x=0
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v * 0.85 - 0.05) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // guiding circle of diameter 2a at origin of construction
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) as f64) * 0.7;
    let rad = (height as f64) * 0.15 * a.clamp(0.5, 1.5);
    for i in 0..48 {
        let th = std::f64::consts::TAU * (i as f64 / 48.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        canvas.plot(px, py, '.');
    }
}

/// Witch of Agnesi room.
#[derive(Debug, Default)]
pub struct WitchOfAgnesi {
    seed: u64,
}

impl WitchOfAgnesi {
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

impl Room for WitchOfAgnesi {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "witch-of-agnesi",
            title: "Witch of Agnesi",
            wing: "Shape & Space",
            blurb: "Maria Agnesi's classical cubic bell curve. t and DRAG: TUNE A.",
            accent: [160, 80, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, a_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "witch of agnesi",
            root: 698.46,
            tempo: 80,
            line: &[0, 4, 7, 12, 7, 4, 0, 7],
            encodes: "a circle construction that became a cubic bell",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("a={a:.2}  agnesi  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = a_param(t, hands.last().copied(), self.seed).clamp(0.2, 2.0);
        // y = 8 a^3 / (x^2 + 4 a^2); peak 2a; total area under the witch is 4 pi a^2.
        let peak = 2.0 * a;
        let area = 4.0 * std::f64::consts::PI * a * a;
        Some(format!("a={a:.2}  peak={peak:.2}  A={area:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "The witch of Agnesi is the cubic y = 8a^3/(x^2+4a^2), built by Maria \
         Gaetana Agnesi from a circle and a sliding line. The odd English name \
         is a mistranslation of the Italian versiera."
    }
}

#[cfg(test)]
mod tests {
    use super::WitchOfAgnesi;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = WitchOfAgnesi::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("A"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = WitchOfAgnesi::new();
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
        WitchOfAgnesi::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
