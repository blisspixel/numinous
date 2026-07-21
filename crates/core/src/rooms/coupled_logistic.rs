//! Coupled logistic maps: two populations with weak cross terms.
//!
//! DRAG: TUNE COUPLING. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 400;

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

fn coupling(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        x * 0.5 + s
    } else {
        phase_unit(t) * 0.35 + s
    }
}

fn draw(canvas: &mut dyn Surface, eps: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = 3.7
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.02
        };
    let mut x = 0.2;
    let mut y = 0.7;
    // burn-in
    for _ in 0..80 {
        let nx = (1.0 - eps) * r * x * (1.0 - x) + eps * r * y * (1.0 - y);
        let ny = (1.0 - eps) * r * y * (1.0 - y) + eps * r * x * (1.0 - x);
        x = nx.clamp(0.0, 1.0);
        y = ny.clamp(0.0, 1.0);
    }
    for i in 0..STEPS {
        let nx = (1.0 - eps) * r * x * (1.0 - x) + eps * r * y * (1.0 - y);
        let ny = (1.0 - eps) * r * y * (1.0 - y) + eps * r * x * (1.0 - x);
        x = nx.clamp(0.0, 1.0);
        y = ny.clamp(0.0, 1.0);
        let col = ((i as f64 / STEPS as f64) * width.saturating_sub(1) as f64).round() as i32;
        let px = (x * height.saturating_sub(1) as f64).round() as i32;
        let py = (y * height.saturating_sub(1) as f64).round() as i32;
        // time series of x (top half metaphor via #) and y (*)
        let yx = ((1.0 - x) * height.saturating_sub(1) as f64).round() as i32;
        let yy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(col, yx, '#');
        canvas.plot(col, yy, '*');
        // phase plane point on left margin strip
        let ix = (x * (width / 4) as f64).round() as i32;
        let iy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, '+');
        let _ = (px, py);
    }
}

/// Coupled logistic maps room.
#[derive(Debug, Default)]
pub struct CoupledLogistic {
    seed: u64,
}

impl CoupledLogistic {
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

impl Room for CoupledLogistic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coupled-logistic",
            title: "Coupled Logistic",
            wing: "Motion & Dynamics",
            blurb: "Two logistic maps cross-talk into sync or chaos. t and DRAG: TUNE COUPLING.",
            accent: [200, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, coupling(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "coupled logistic",
            root: 103.83,
            tempo: 110,
            line: &[0, 5, 0, 7, 0, 12, 0, 7],
            encodes: "two logistic clocks exchanging a fraction each step",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE COUPLING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = coupling(t, None, self.seed);
        Some(format!("e={e:.2}  couple  DRAG:E"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = coupling(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let e = coupling(t, hands.last().copied(), self.seed);
        // Match render_poked: draw XORs the seed with poke count for r variation.
        let seed = self.seed ^ hands.len() as u64;
        let r = 3.7
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.02
            };
        let mut x = 0.2;
        let mut y = 0.7;
        for _ in 0..80 {
            let nx = (1.0 - e) * r * x * (1.0 - x) + e * r * y * (1.0 - y);
            let ny = (1.0 - e) * r * y * (1.0 - y) + e * r * x * (1.0 - x);
            x = nx.clamp(0.0, 1.0);
            y = ny.clamp(0.0, 1.0);
        }
        let mut sum = 0.0;
        let n = 120usize;
        for _ in 0..n {
            let nx = (1.0 - e) * r * x * (1.0 - x) + e * r * y * (1.0 - y);
            let ny = (1.0 - e) * r * y * (1.0 - y) + e * r * x * (1.0 - x);
            x = nx.clamp(0.0, 1.0);
            y = ny.clamp(0.0, 1.0);
            sum += (x - y).abs();
        }
        let md = sum / n as f64;
        let tag = if md < 0.04 { "SYNC" } else { "split" };
        Some(format!("e={e:.3}  mean|dx|={md:.3}  {tag}"))
    }

    fn reveal(&self) -> &'static str {
        "Two logistic maps with weak cross-coupling can lock into synchrony or \
         stay chaotic and independent. Coupling strength is the dial between \
         private chaos and shared rhythm."
    }
}

#[cfg(test)]
mod tests {
    use super::CoupledLogistic;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CoupledLogistic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("couple"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn couple_changes() {
        let r = CoupledLogistic::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        CoupledLogistic::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
