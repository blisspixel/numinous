//! Bogdanov map: planar discrete chaos with a classic parameter gallery.
//!
//! x' = y; y' = x + e y + k y (1-y) + mu x (x-1). DRAG: TUNE K.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 7_000;

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

fn k_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.5 + x * 2.0 + s
    } else {
        1.2 + phase_unit(t) * 0.8 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Soft-bounded Bogdanov-inspired iteration: tanh keeps the portrait filled.
    let e = 0.08;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS * 3);
    for (sx, sy) in [(0.1f64, 0.0), (0.3, 0.1), (-0.2, 0.2)] {
        let mut x = sx;
        let mut y = sy;
        for _ in 0..ITERS {
            let nx = (y + e * x + k * x * (1.0 - x.abs()) * 0.25).tanh();
            let ny = (-x + 0.15 * y).tanh();
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            pts.push((x, y));
        }
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 9 == 0 { '#' } else { '*' });
    }
}

/// Bogdanov map room.
#[derive(Debug, Default)]
pub struct Bogdanov {
    seed: u64,
}

impl Bogdanov {
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

impl Room for Bogdanov {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bogdanov",
            title: "Bogdanov Map",
            wing: "Motion & Dynamics",
            blurb: "Planar discrete map with a classic chaotic gallery. t and DRAG: TUNE K.",
            accent: [180, 60, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, k_param(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bogdanov",
            root: 311.13,
            tempo: 102,
            line: &[0, 4, 8, 11, 15, 11, 8, 4],
            encodes: "k-tuned planar fold into a strange cloud",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = k_param(t, None, self.seed);
        Some(format!("k={k:.2}  bogdanov  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = k_param(t, hands.last().copied(), self.seed);
        draw(canvas, k);
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
        let k = k_param(t, hands.last().copied(), self.seed);
        // Match soft-bounded Bogdanov iteration used in draw.
        let e = 0.08;
        let mut x = 0.1_f64;
        let mut y = 0.0_f64;
        let mut max_r = 0.0_f64;
        for _ in 0..400 {
            let nx = (y + e * x + k * x * (1.0 - x.abs()) * 0.25).tanh();
            let ny = (-x + 0.15 * y).tanh();
            x = nx;
            y = ny;
            max_r = max_r.max((x * x + y * y).sqrt());
        }
        let band = if max_r > 0.9 {
            "edge"
        } else if max_r > 0.5 {
            "wide"
        } else {
            "core"
        };
        Some(format!("k={k:.2}  r={max_r:.2}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "The Bogdanov map is a planar discrete dynamical system used as a \
         gallery of bifurcations and strange attractors. One parameter k steers \
         the fold strength."
    }
}

#[cfg(test)]
mod tests {
    use super::Bogdanov;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bogdanov::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Bogdanov::new();
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
        Bogdanov::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Bogdanov::new().motif().unwrap().line.len() >= 6);
    }
}
