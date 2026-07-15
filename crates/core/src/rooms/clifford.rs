//! Clifford attractor: the classic sin/cos iterated map.
//!
//! x' = sin(a y) + c cos(a x); y' = sin(b x) + d cos(b y).
//! DRAG: TUNE A AND B. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 10_000;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64, f64, f64) {
    // A famous gallery: a=-1.4, b=1.6, c=1.0, d=0.7
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        (-1.8 + x * 0.8 + s, 1.2 + y * 0.8, 1.0, 0.7)
    } else {
        let u = phase_unit(t);
        (-1.4 + u * 0.3 + s, 1.6 - u * 0.2, 1.0, 0.7 + s * 0.1)
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, c: f64, d: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut x: f64 = 0.1;
    let mut y: f64 = 0.1;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let nx = (a * y).sin() + c * (a * x).cos();
        let ny = (b * x).sin() + d * (b * y).cos();
        x = nx;
        y = ny;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        pts.push((x, y));
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let ch = if i % 10 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Clifford attractor room.
#[derive(Debug, Default)]
pub struct Clifford {
    seed: u64,
}

impl Clifford {
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

impl Room for Clifford {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "clifford",
            title: "Clifford Attractor",
            wing: "Motion & Dynamics",
            blurb: "Sin/cos iterated map with dense organic attractors. t and DRAG: TUNE A AND B.",
            accent: [80, 200, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b, c, d) = params(t, None, self.seed);
        draw(canvas, a, b, c, d);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "clifford",
            root: 155.56,
            tempo: 90,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "sin and cos folding a plane into smoke",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b, _c, _d) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b, c, d) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b, c, d);
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
        let (a, b, _c, _d) = params(t, hands.last().copied(), self.seed);
        Some(format!("TUNE a={a:.3}  b={b:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Clifford attractors are simple trigonometric maps that paint dense, \
         organic clouds. Four parameters steer the shape; many settings produce \
         strange attractors with delicate filaments."
    }
}

#[cfg(test)]
mod tests {
    use super::Clifford;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Clifford::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Clifford::new();
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
        Clifford::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Clifford::new().motif().unwrap().line.len() >= 6);
    }
}
