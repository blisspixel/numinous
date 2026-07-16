//! Arnold circle map: mode locking and devil's staircase of winding numbers.
//!
//! theta' = theta + omega - (K/2pi) sin(2pi theta). DRAG: TUNE K AND OMEGA.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 400;
const BURN: usize = 80;

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
    // K coupling, omega drive frequency in [0,1)
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.01
    };
    if let Some((x, y)) = hand {
        (x * 2.5 + s, y)
    } else {
        let u = phase_unit(t);
        (0.5 + u * 1.5 + s, 0.3 + u * 0.2)
    }
}

fn step(theta: f64, omega: f64, k: f64) -> f64 {
    let two_pi = std::f64::consts::TAU;
    (theta + omega - (k / two_pi) * (two_pi * theta).sin()).rem_euclid(1.0)
}

fn winding(omega: f64, k: f64) -> f64 {
    let mut th = 0.1;
    for _ in 0..BURN {
        th = step(th, omega, k);
    }
    let start = th;
    let mut sum = 0.0;
    for _ in 0..ITERS {
        let next = step(th, omega, k);
        // Unwrapped increment (handle wrap).
        let mut d = next - th;
        if d < -0.5 {
            d += 1.0;
        } else if d > 0.5 {
            d -= 1.0;
        }
        sum += d;
        th = next;
    }
    let _ = start;
    sum / ITERS as f64
}

fn draw(canvas: &mut dyn Surface, k: f64, omega: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Top: staircase of winding number vs omega at fixed K.
    let samples = width.max(40);
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=samples {
        let w = i as f64 / samples as f64;
        let wn = winding(w, k).clamp(0.0, 1.0);
        let px = (w * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - wn) * (height as f64 * 0.55)).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '#');
        }
        prev = Some((px, py));
    }
    // Bottom: orbit on a circle for the chosen omega.
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.78;
    let r = height as f64 * 0.16;
    for i in 0..64 {
        let a = std::f64::consts::TAU * i as f64 / 64.0;
        canvas.plot(
            (cx + r * a.cos()).round() as i32,
            (cy + r * a.sin()).round() as i32,
            '.',
        );
    }
    let mut th = if seed == 0 {
        0.1
    } else {
        (seed % 50) as f64 * 0.02
    };
    for i in 0..120 {
        th = step(th, omega, k);
        let a = th * std::f64::consts::TAU;
        let px = (cx + r * a.cos()).round() as i32;
        let py = (cy + r * a.sin()).round() as i32;
        canvas.plot(px, py, if i + 20 > 120 { '@' } else { '*' });
    }
    // Mark current omega on the staircase axis.
    let ox = (omega.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(ox, 0, ox, (height as f64 * 0.55).round() as i32, '+');
}

/// Arnold circle map room.
#[derive(Debug, Default)]
pub struct CircleMap {
    seed: u64,
}

impl CircleMap {
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

impl Room for CircleMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "circle-map",
            title: "Arnold Circle Map",
            wing: "Motion & Dynamics",
            blurb: "Mode locking and winding-number staircase on the circle. t and DRAG: TUNE K \
                    AND OMEGA.",
            accent: [80, 100, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (k, omega) = params(t, None, self.seed);
        draw(canvas, k, omega, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "arnold",
            root: 196.0,
            tempo: 96,
            line: &[0, 0, 5, 7, 7, 12, 12, 5],
            encodes: "winding plateaus of mode-locked rotation",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K AND OMEGA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (k, omega) = params(t, None, self.seed);
        let w = winding(omega, k);
        Some(format!("K={k:.2}  w~{w:.3}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (k, omega) = params(t, hands.last().copied(), self.seed);
        draw(canvas, k, omega, self.seed ^ hands.len() as u64);
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
        let (k, omega) = params(t, hands.last().copied(), self.seed);
        let w = winding(omega, k);
        let drift = (w - omega).abs();
        let band = if k >= 1.0 && drift < 0.02 {
            "LOCK"
        } else if drift < 0.05 {
            "near"
        } else {
            "free"
        };
        Some(format!("w={w:.3}  dOm={drift:.3}  K={k:.2}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "Arnold's circle map models two coupled oscillators. When the coupling \
         K is large enough, the winding number locks to rational plateaus: the \
         devil's staircase of mode locking, with chaotic bands between."
    }
}

#[cfg(test)]
mod tests {
    use super::CircleMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CircleMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("K="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = CircleMap::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.1,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CircleMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(CircleMap::new().motif().unwrap().line.len() >= 6);
    }
}
