//! Bedhead attractor: soft pillow chaos from a trig map.
//!
//! x' = sin(x y / b) y + cos(a x - y); y' = x + sin(y)/b.
//! DRAG: TUNE A AND B. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 8_000;

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
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        (-1.0 + x * 2.0 + s, 0.5 + y * 1.5)
    } else {
        let u = phase_unit(t);
        (-0.81 + u * 0.4 + s, 0.92 + (1.0 - u) * 0.2)
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut x: f64 = 1.0;
    let mut y: f64 = 1.0;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    let b = b.max(0.05);
    for _ in 0..ITERS {
        let nx = (x * y / b).sin() * y + (a * x - y).cos();
        let ny = x + y.sin() / b;
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
        let ch = if i % 11 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Bedhead attractor room.
#[derive(Debug, Default)]
pub struct Bedhead {
    seed: u64,
}

impl Bedhead {
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

impl Room for Bedhead {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bedhead",
            title: "Bedhead Attractor",
            wing: "Motion & Dynamics",
            blurb: "Soft pillow-shaped strange attractor from a trig map. t and DRAG: TUNE A AND \
                    B.",
            accent: [180, 120, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = params(t, None, self.seed);
        draw(canvas, a, b);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bedhead",
            root: 130.81,
            tempo: 84,
            line: &[0, 2, 5, 7, 10, 7, 5, 2],
            encodes: "a soft pillow of chaotic density",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (a, b) = params(t, hands.last().copied(), self.seed);
        let mut x = 1.0_f64;
        let mut y = 1.0_f64;
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        let bb = b.max(0.05);
        for _ in 0..600 {
            let nx = (x * y / bb).sin() * y + (a * x - y).cos();
            let ny = x + y.sin() / bb;
            if !nx.is_finite() || !ny.is_finite() {
                break;
            }
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("a={a:.2} b={b:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Bedhead attractor is a named gallery map: two parameters fold a \
         plane into a soft, pillow-like density of points. It is decorative chaos \
         with honest iteration underneath."
    }
}

#[cfg(test)]
mod tests {
    use super::Bedhead;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bedhead::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Bedhead::new();
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
        Bedhead::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Bedhead::new().motif().unwrap().line.len() >= 6);
    }
}
