//! Sprott attractor: simple 3D quadratic chaotic flow.
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 5_000;
const DT: f64 = 0.02;

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

fn a_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.2 + x * 0.6 + s
    } else {
        0.3 + phase_unit(t) * 0.3 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.01
    };
    let mut x = 0.1 + j;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(STEPS);
    for _ in 0..200 {
        // Minimal Sprott-like quadratic flow
        let dx = y * z;
        let dy = x - y;
        let dz = a - x * y;
        x += dx * DT;
        y += dy * DT;
        z += dz * DT;
    }
    for _ in 0..STEPS {
        let dx = y * z;
        let dy = x - y;
        let dz = a - x * y;
        x += dx * DT;
        y += dy * DT;
        z += dz * DT;
        if !x.is_finite() || !z.is_finite() {
            break;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(z);
        max_y = max_y.max(z);
        pts.push((x, z));
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 15 == 0 { '#' } else { '*' });
    }
}

/// Sprott attractor room.
#[derive(Debug, Default)]
pub struct Sprott {
    seed: u64,
}

impl Sprott {
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

impl Room for Sprott {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sprott",
            title: "Sprott Attractor",
            wing: "Motion & Dynamics",
            blurb: "Minimal quadratic chaos in three dimensions. t and DRAG: TUNE A.",
            accent: [120, 80, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, a_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sprott",
            root: 190.0,
            tempo: 92,
            line: &[0, 4, 8, 11, 8, 4, 0, 11],
            encodes: "sparse quadratic terms that still make chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("a={a:.2}  sprott  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = a_param(t, hands.last().copied(), self.seed);
        Some(format!("TUNE a={a:.3}  flow"))
    }

    fn reveal(&self) -> &'static str {
        "Sprott catalogued many simple chaotic flows with few terms. This room \
         is one minimal quadratic case: chaos without the usual three-wing \
         Lorenz silhouette, a thin algebraic engine of stretch and fold."
    }
}

#[cfg(test)]
mod tests {
    use super::Sprott;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Sprott::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("A"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Sprott::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Sprott::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
