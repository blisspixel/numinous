//! Limaçon of Pascal: polar r = b + a cos theta (cardioid when a = b).
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
    // a/b: <1 dimple, =1 cardioid, >1 loop
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.8 + s
    } else {
        0.7 + phase_unit(t) * 1.3 + s
    }
}

fn draw(canvas: &mut dyn Surface, ab: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let b = 1.0;
    let a = ab;
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64) * 0.34 / (a + b).max(1.0);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.05
    };
    let steps = 420;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let r = b + a * th.cos();
        let px = (cx + scale * r * th.cos()).round() as i32;
        let py = (cy - scale * r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            let ch = if ab > 1.0 { '#' } else { '*' };
            canvas.line(ox, oy, px, py, ch);
            canvas.line(ox, oy + 1, px, py + 1, '.');
        }
        prev = Some((px, py));
    }
}

/// Limacon room.
#[derive(Debug, Default)]
pub struct Limacon {
    seed: u64,
}

impl Limacon {
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

impl Room for Limacon {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "limacon",
            title: "Limacon",
            wing: "Shape & Space",
            blurb: "Pascal's snail: dimple, cardioid, or loop. t and DRAG: TUNE RATIO.",
            accent: [200, 80, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "limacon",
            root: 146.8,
            tempo: 92,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "polar snail that becomes a cardioid at a equals b",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ab = ratio(t, None, self.seed);
        Some(format!("a/b={ab:.2}  snail  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ab = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, ab, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let ab = ratio(t, hands.last().copied(), self.seed);
        let kind = if (ab - 1.0).abs() < 0.08 {
            "cardioid"
        } else if ab < 1.0 {
            "dimple"
        } else {
            "loop"
        };
        Some(format!("a/b={ab:.3}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "A limaçon is the polar curve r = b + a cos theta. When a = b it is a \
         cardioid; when a < b it has a dimple; when a > b it loops. Named for \
         Étienne Pascal, it is a roulette of a circle."
    }
}

#[cfg(test)]
mod tests {
    use super::Limacon;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Limacon::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("snail"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Limacon::new();
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
        Limacon::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
