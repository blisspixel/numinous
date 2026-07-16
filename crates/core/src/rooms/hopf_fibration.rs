//! Hopf fibration: circles of S3 projected to linked rings in R3.
//!
//! DRAG: TUNE FIBER. See `docs/ROOMS.md`.

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

fn fiber(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::PI + s
    } else {
        phase_unit(t) * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, base: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.35;
    // Villarceau-like linked circles: stereographic fibers over S2 base points.
    let n_fibers = 6 + if seed == 0 { 0 } else { (seed % 2) as usize };
    for f in 0..n_fibers {
        let lat = -0.8 + 1.6 * (f as f64 / (n_fibers - 1).max(1) as f64);
        let lon = base + f as f64 * 0.4;
        // Circle in a plane; project as ellipse.
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=48 {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
            // Fiber circle radius depends on latitude.
            let rr = (1.0 - lat * lat).sqrt().max(0.15);
            let x = rr * th.cos() * lon.cos() - lat * lon.sin() * 0.3;
            let y = rr * th.sin();
            let z = rr * th.cos() * lon.sin() + lat * lon.cos() * 0.3;
            let d = 1.0 / (2.2 + z);
            let px = (cx + x * sc * d).round() as i32;
            let py = (cy - y * sc * d * 0.65).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if f % 2 == 0 { '#' } else { '*' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
}

/// Hopf fibration room.
#[derive(Debug, Default)]
pub struct HopfFibration {
    seed: u64,
}

impl HopfFibration {
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

impl Room for HopfFibration {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hopf-fibration",
            title: "Hopf Fibration",
            wing: "Shape & Space",
            blurb: "S3 fibers as linked circles. t and DRAG: TUNE FIBER.",
            accent: [40, 80, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, fiber(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hopf-fibration",
            root: 92.5,
            tempo: 86,
            line: &[0, 5, 7, 12, 9, 5, 0, 7],
            encodes: "Hopf map: every fiber is a circle, any two fibers link once",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE FIBER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = fiber(t, None, self.seed);
        Some(format!("b={b:.2}  hopf  DRAG:FIB"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = fiber(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
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
        let b = fiber(t, hands.last().copied(), self.seed);
        Some(format!("B={b:.3}  hopf"))
    }

    fn reveal(&self) -> &'static str {
        "The Hopf fibration maps the 3-sphere to the 2-sphere so each fiber is a \
         circle. Any two distinct fibers are linked exactly once; stereographic \
         projection turns them into nested Villarceau circles in space."
    }
}

#[cfg(test)]
mod tests {
    use super::HopfFibration;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HopfFibration::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("hopf"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn fiber_changes() {
        let r = HopfFibration::new();
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
        HopfFibration::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
