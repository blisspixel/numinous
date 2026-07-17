//! Angle-doubling map on the circle: the Bernoulli shift.
//!
//! theta -> 2 theta mod 1. Expanding chaos, simple symbolics.
//! DRAG: SET THE SEED AND STEPS. See `docs/ROOMS.md`.

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

fn params(t: f64, hand: Option<(f64, f64)>) -> (f64, usize) {
    if let Some((x, y)) = hand {
        (x, (8 + (y * 40.0) as usize).clamp(8, 64))
    } else {
        let u = phase_unit(t);
        (0.1 + u * 0.3, (12 + (u * 30.0) as usize).clamp(8, 48))
    }
}

fn draw(canvas: &mut dyn Surface, theta0: f64, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Graph of y = 2x mod 1 as two lines.
    let y0 = height.saturating_sub(1) as i32;
    let mid = (0.5 * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(0, y0, mid, 0, '#');
    canvas.line(mid, y0, width.saturating_sub(1) as i32, 0, '#');
    canvas.line(0, y0, width.saturating_sub(1) as i32, 0, '.');
    // Orbit on circle at bottom.
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.72;
    let r = height as f64 * 0.18;
    for i in 0..48 {
        let a = std::f64::consts::TAU * i as f64 / 48.0;
        canvas.plot(
            (cx + r * a.cos()).round() as i32,
            (cy + r * a.sin()).round() as i32,
            ':',
        );
    }
    let mut th = if seed == 0 {
        theta0
    } else {
        (theta0 + (seed % 20) as f64 * 0.01).fract()
    };
    let mut bits = 0u64;
    for i in 0..n {
        let a = th * std::f64::consts::TAU;
        let px = (cx + r * a.cos()).round() as i32;
        let py = (cy + r * a.sin()).round() as i32;
        canvas.plot(px, py, if i + 8 > n { '@' } else { '*' });
        // Cobweb sample on the graph.
        let gx = (th * width.saturating_sub(1) as f64).round() as i32;
        let gy =
            ((1.0 - (2.0 * th).fract()) * height.saturating_sub(1) as f64 * 0.45).round() as i32;
        canvas.plot(gx, gy, '+');
        bits = (bits << 1) | if th < 0.5 { 0 } else { 1 };
        th = (2.0 * th).fract();
    }
    let _ = bits;
}

/// Doubling map room.
#[derive(Debug, Default)]
pub struct DoublingMap {
    seed: u64,
}

impl DoublingMap {
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

impl Room for DoublingMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "doubling-map",
            title: "Angle Doubling",
            wing: "Motion & Dynamics",
            blurb: "Bernoulli shift theta -> 2 theta mod 1: expanding chaos. t and DRAG: SET THE \
                    SEED AND STEPS.",
            accent: [40, 180, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (th, n) = params(t, None);
        draw(canvas, th, n, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "doubling",
            root: 523.25,
            tempo: 152,
            line: &[0, 0, 0, 7, 7, 7, 12, 12],
            encodes: "binary expansion revealed by doubling",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE SEED AND STEPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (th, n) = params(t, None);
        Some(format!("th={th:.2}  n={n}  DRAG:SEED"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (th, n) = params(t, hands.last().copied());
        draw(canvas, th, n, self.seed ^ hands.len() as u64);
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
        let (th, n) = params(t, hands.last().copied());
        // Bernoulli map: Lyapunov is ln 2; bits are the itinerary.
        let lyap = std::f64::consts::LN_2;
        let mut x = if self.seed == 0 {
            th
        } else {
            (th + (self.seed % 20) as f64 * 0.01).fract()
        };
        let mut ones = 0u32;
        for _ in 0..n {
            if x >= 0.5 {
                ones += 1;
            }
            x = (2.0 * x).fract();
        }
        let dens = ones as f64 / n.max(1) as f64;
        Some(format!("n={n}  lyap={lyap:.2}  1s={dens:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The angle-doubling map is the Bernoulli shift: each iterate reveals the \
         next binary digit of the starting angle. It is expanding, ergodic, and \
         conjugate to the full shift on two symbols."
    }
}

#[cfg(test)]
mod tests {
    use super::DoublingMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DoublingMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SEED"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn seed_changes() {
        let r = DoublingMap::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        DoublingMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(DoublingMap::new().motif().unwrap().line.len() >= 6);
    }
}
