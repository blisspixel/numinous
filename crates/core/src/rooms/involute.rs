//! Circle involute: path of a point on a taut string unwrapping from a circle.
//!
//! DRAG: TUNE TURNS. See `docs/ROOMS.md`.

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

fn turns(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        0.5 + x * 3.5 + s
    } else {
        1.0 + phase_unit(t) * 2.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, n_turns: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.08;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.04
    };
    // base circle
    for i in 0..64 {
        let th = std::f64::consts::TAU * (i as f64 / 64.0);
        let px = (cx + a * th.cos()).round() as i32;
        let py = (cy - a * th.sin()).round() as i32;
        canvas.plot(px, py, '.');
    }
    // involute: x = a (cos t + t sin t), y = a (sin t - t cos t)
    let steps = 400;
    let t_max = n_turns.clamp(0.3, 5.0) * std::f64::consts::TAU;
    let mut max_r = a;
    for i in 0..=steps {
        let t = t_max * (i as f64 / steps as f64);
        let x = a * (t.cos() + t * t.sin());
        let y = a * (t.sin() - t * t.cos());
        max_r = max_r.max(x.hypot(y));
    }
    let scale = (width.min(height) as f64) * 0.42 / max_r.max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let t = t_max * (i as f64 / steps as f64) + rot;
        let x = a * (t.cos() + t * t.sin());
        let y = a * (t.sin() - t * t.cos());
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Circle involute room.
#[derive(Debug, Default)]
pub struct Involute {
    seed: u64,
}

impl Involute {
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

impl Room for Involute {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "involute",
            title: "Involute",
            wing: "Shape & Space",
            blurb: "Unwrapping a taut string from a circle. t and DRAG: TUNE TURNS.",
            accent: [100, 140, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, turns(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "involute",
            root: 277.18,
            tempo: 82,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "string peel from a circle draws gear flanks",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TURNS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = turns(t, None, self.seed);
        Some(format!("turns={n:.1}  invol  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = turns(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = turns(t, hands.last().copied(), self.seed);
        // Circle involute arc length from cusp is (1/2) a theta^2; a~1 here.
        let theta = n * std::f64::consts::TAU;
        let arc = 0.5 * theta * theta;
        Some(format!("turns={n:.2}  s~{arc:.1}  gear"))
    }

    fn reveal(&self) -> &'static str {
        "A circle involute is the path of the free end of a taut string as it \
         unwinds from a circle. Gear teeth use involute flanks so contact stays \
         smooth at constant pressure angle."
    }
}

#[cfg(test)]
mod tests {
    use super::Involute;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Involute::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("invol"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn turns_change() {
        let r = Involute::new();
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
        Involute::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
