//! Manneville-Pomeau intermittency map (toy).
//!
//! Near-tangent fixed point produces laminar phases then chaotic bursts.
//! DRAG: TUNE EPSILON. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 240;

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

fn epsilon(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.002
    };
    if let Some((x, _)) = hand {
        0.001 + x * 0.05 + s
    } else {
        0.005 + phase_unit(t) * 0.03 + s
    }
}

fn step(x: f64, eps: f64) -> f64 {
    // Classic toy: x + x^2 + eps mod 1 on [0,1)
    (x + x * x + eps).rem_euclid(1.0)
}

fn draw(canvas: &mut dyn Surface, eps: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Graph
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=width {
        let x = i as f64 / width.saturating_sub(1).max(1) as f64;
        let y = step(x, eps);
        let px = i as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '#');
        }
        prev = Some((px, py));
    }
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
    // Time series strip of laminar vs burst
    let mut x = if seed == 0 {
        0.1
    } else {
        0.05 + (seed % 40) as f64 * 0.01
    };
    for i in 0..ORBIT.min(width) {
        x = step(x, eps);
        let px = i as i32;
        let py = ((1.0 - x) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round()
            as i32;
        canvas.plot(px, py, if x < 0.2 { '*' } else { '+' });
    }
}

/// Manneville map room.
#[derive(Debug, Default)]
pub struct Manneville {
    seed: u64,
}

impl Manneville {
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

impl Room for Manneville {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "manneville",
            title: "Manneville Map",
            wing: "Motion & Dynamics",
            blurb: "Intermittency: long laminar waits, then chaotic bursts. t and DRAG: TUNE \
                    EPSILON.",
            accent: [200, 140, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, epsilon(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "manneville",
            root: 110.0,
            tempo: 70,
            line: &[0, 0, 0, 0, 5, 0, 0, 12],
            encodes: "long laminar silence then a chaotic burst",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE EPSILON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = epsilon(t, None, self.seed);
        Some(format!("eps={e:.3}  intermit  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = epsilon(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
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
        let e = epsilon(t, hands.last().copied(), self.seed);
        Some(format!("TUNE eps={e:.4}"))
    }

    fn reveal(&self) -> &'static str {
        "Manneville-Pomeau intermittency appears when a fixed point is almost \
         tangent to the diagonal: the orbit crawls for long laminar stretches, \
         then bursts into chaos. Epsilon opens the channel."
    }
}

#[cfg(test)]
mod tests {
    use super::Manneville;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Manneville::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Manneville::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Manneville::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Manneville::new().motif().unwrap().line.len() >= 6);
    }
}
