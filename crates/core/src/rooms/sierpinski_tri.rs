//! Sierpinski triangle by recursive removal (not the chaos-game room).
//!
//! Midpoint subdivision removes centers. DRAG: SET THE DEPTH. See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 6.0) as usize).clamp(1, 7)
    } else {
        (2 + (phase_unit(t) * 4.0) as usize).clamp(1, 6)
    }
}

fn sierpinski_pixel(ix: usize, iy: usize, w: usize, h: usize, d: usize) -> bool {
    // Map to triangle coords; use bitwise AND test in discrete grid of size 2^d
    let n = 1usize << d.min(8);
    let x = (ix * n) / w.max(1);
    let y = (iy * n) / h.max(1);
    if y >= n || x >= n {
        return false;
    }
    // In upright triangle: x between (n-1-y)/2-ish: use x & y == 0 style for gasket
    // Classic discrete Sierpinski: cell filled if (x & y) == 0 in a square, clip triangle
    if (x & y) != 0 {
        return false;
    }
    // Triangle mask: bottom-heavy
    let half = (n - 1 - y) / 2;
    let mid = n / 2;
    x + half >= mid.saturating_sub(half) && x <= mid + half
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let off = if seed == 0 { 0 } else { (seed % 3) as usize };
    for y in 0..height {
        for x in 0..width {
            if sierpinski_pixel(x + off, y, width, height, d) {
                canvas.plot(x as i32, y as i32, if d > 4 { '#' } else { '*' });
            }
        }
    }
}

/// Sierpinski triangle (recursive) room.
#[derive(Debug, Default)]
pub struct SierpinskiTri {
    seed: u64,
}

impl SierpinskiTri {
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

impl Room for SierpinskiTri {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sierpinski-tri",
            title: "Sierpinski Triangle",
            wing: "Fractals",
            blurb: "Recursive midpoint gasket (not the chaos game). t and DRAG: SET THE DEPTH.",
            accent: [200, 80, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sierpinski tri",
            root: 246.94,
            tempo: 108,
            line: &[0, 0, 7, 0, 12, 7, 0, 12],
            encodes: "bitwise holes in a triangular lattice",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  gasket  DRAG:DEPTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = depth(t, hands.last().copied());
        Some(format!("DEPTH={d}  cells~{}", 3usize.pow(d as u32)))
    }

    fn reveal(&self) -> &'static str {
        "The Sierpinski triangle removes the open middle quarter of each \
         triangle forever. The discrete form (x & y) == 0 is the same gasket: \
         Pascal mod 2 and the chaos game share this set."
    }
}

#[cfg(test)]
mod tests {
    use super::SierpinskiTri;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SierpinskiTri::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = SierpinskiTri::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        SierpinskiTri::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(SierpinskiTri::new().motif().unwrap().line.len() >= 6);
    }
}
