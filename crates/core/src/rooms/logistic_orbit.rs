//! Logistic orbit cobweb: distinct from logistic-map and logistic-cobweb.
//!
//! Long orbit trail with return map. DRAG: TUNE R AND X0. See `docs/ROOMS.md`.
//!
//! Note: logistic-map and logistic-cobweb already exist; this room is a pure
//! return-map portrait with measured period readout.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 200;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, y)) = hand {
        (2.5 + x * 1.5 + s, 0.05 + y * 0.9)
    } else {
        let u = phase_unit(t);
        (3.2 + u * 0.8 + s, 0.2 + u * 0.3)
    }
}

fn period_guess(vals: &[f64]) -> usize {
    if vals.len() < 20 {
        return 0;
    }
    let tail = &vals[vals.len().saturating_sub(64)..];
    let last = *tail.last().unwrap_or(&0.0);
    for p in 1..=16 {
        if tail.len() <= p {
            break;
        }
        let mut ok = true;
        for i in (p..tail.len()).rev().take(p * 2) {
            if (tail[i] - tail[i - p]).abs() > 0.02 {
                ok = false;
                break;
            }
        }
        if ok && (last - tail[tail.len() - 1 - p]).abs() < 0.02 {
            return p;
        }
    }
    0
}

fn draw(canvas: &mut dyn Surface, r: f64, x0: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Parabola r x (1-x) and diagonal.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=width {
        let x = i as f64 / width.saturating_sub(1).max(1) as f64;
        let y = (r * x * (1.0 - x)).clamp(0.0, 1.0);
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
    // Cobweb.
    let mut x = x0.clamp(0.01, 0.99);
    let mut px = (x * width.saturating_sub(1) as f64).round() as i32;
    let mut py = height.saturating_sub(1) as i32;
    for i in 0..ORBIT {
        let y = (r * x * (1.0 - x)).clamp(0.0, 1.0);
        let qx = (x * width.saturating_sub(1) as f64).round() as i32;
        let qy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, py, qx, py, if i % 2 == 0 { '*' } else { '+' });
        canvas.line(qx, py, qx, qy, if i % 2 == 0 { '*' } else { '+' });
        let dx = (y * width.saturating_sub(1) as f64).round() as i32;
        let dy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(qx, qy, dx, dy, '.');
        px = dx;
        py = dy;
        x = y;
    }
}

/// Logistic orbit portrait room.
#[derive(Debug, Default)]
pub struct LogisticOrbit {
    seed: u64,
}

impl LogisticOrbit {
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

impl Room for LogisticOrbit {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "logistic-orbit",
            title: "Logistic Orbit",
            wing: "Motion & Dynamics",
            blurb: "Return-map cobweb of the logistic map with period guess. t and DRAG: TUNE R \
                    AND X0.",
            accent: [220, 60, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (r, x0) = params(t, None, self.seed);
        draw(canvas, r, x0);
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "log orbit",
            root: 349.23,
            tempo: 126,
            line: &[0, 2, 4, 7, 11, 14, 11, 4],
            encodes: "parabola cobweb toward a cycle or chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R AND X0")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (r, x0) = params(t, None, self.seed);
        Some(format!("r={r:.2}  x0={x0:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (r, x0) = params(t, hands.last().copied(), self.seed);
        draw(canvas, r, x0);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (r, x0) = params(t, hands.last().copied(), self.seed);
        let mut x = x0;
        let mut vals = Vec::with_capacity(ORBIT);
        for _ in 0..ORBIT {
            x = r * x * (1.0 - x);
            vals.push(x);
        }
        let p = period_guess(&vals);
        if p == 0 {
            Some(format!("r={r:.3}  chaos?"))
        } else {
            Some(format!("r={r:.3}  period~{p}"))
        }
    }

    fn reveal(&self) -> &'static str {
        "The logistic map x -> r x (1-x) is the minimal model of period doubling. \
         This room's cobweb makes the return map visible; the period guess is a \
         toy readout, not a proof of uniqueness."
    }
}

#[cfg(test)]
mod tests {
    use super::LogisticOrbit;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LogisticOrbit::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = LogisticOrbit::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LogisticOrbit::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(LogisticOrbit::new().motif().unwrap().line.len() >= 6);
    }
}
