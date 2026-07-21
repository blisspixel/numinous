//! Trochoid: path of a point fixed to a rolling circle (roulette family).
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
    // h/r: <1 curtate, =1 cycloid, >1 prolate
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.2 + x * 1.8 + s
    } else {
        0.4 + phase_unit(t) * 1.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, hr: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = (height as f64) * 0.12;
    let h = hr * r;
    let n_cups = 3.0;
    let total = n_cups * std::f64::consts::TAU;
    let full_w = r * total;
    let scale_x = (width as f64 * 0.9) / full_w.max(1.0);
    let ox = width as f64 * 0.05;
    let oy = height as f64 * 0.25;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let steps = 400;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = total * (i as f64 / steps as f64) + j;
        // x = r th - h sin th, y = r - h cos th
        let x = r * th - h * th.sin();
        let y = r - h * th.cos();
        let px = (ox + x * scale_x).round() as i32;
        let py = (oy + y * scale_x).round() as i32;
        if let Some((ax, ay)) = prev {
            canvas.line(ax, ay, px, py, if hr > 1.0 { '#' } else { '*' });
        }
        prev = Some((px, py));
    }
    // rolling line
    let ly = (oy + 2.0 * r * scale_x).round() as i32;
    canvas.line(0, ly, width.saturating_sub(1) as i32, ly, '.');
}

/// Trochoid room.
#[derive(Debug, Default)]
pub struct Trochoid {
    seed: u64,
}

impl Trochoid {
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

impl Room for Trochoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "trochoid",
            title: "Trochoid",
            wing: "Shape & Space",
            blurb: "Rolling-circle path: curtate, cycloid, or prolate. t and DRAG: TUNE RATIO.",
            accent: [160, 100, 40],
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
            key: "trochoid",
            root: 220.0,
            tempo: 88,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "point distance to rolling center sets the cup shape",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let hr = ratio(t, None, self.seed);
        Some(format!("h/r={hr:.2}  troch  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let hr = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, hr, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let hr = ratio(t, hands.last().copied(), self.seed);
        let kind = if (hr - 1.0).abs() < 0.08 {
            "cycloid"
        } else if hr < 1.0 {
            "curtate"
        } else {
            "prolate"
        };
        Some(format!("h/r={hr:.3}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "A trochoid is the path of a point fixed relative to a circle rolling \
         on a line. Distance h from the center decides curtate (h<r), cycloid \
         (h=r), or prolate (h>r) loops."
    }
}

#[cfg(test)]
mod tests {
    use super::Trochoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Trochoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("troch"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Trochoid::new();
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
        Trochoid::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
