//! Hilbert curve: a continuous path that fills the square.
//!
//! The limit of recursive U-turns visits every point of the unit square. Finite
//! generations approximate space-filling without crossings. DRAG: DEEPEN THE
//! FOLD. See `docs/ROOMS.md`.

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

fn order(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (1 + (x * 5.0) as u32).clamp(1, 6)
    } else {
        (2 + (phase_unit(t) * 3.0) as u32).clamp(1, 5)
    }
}

/// d2xy from Hilbert curve Wikipedia / Butz.
fn d2xy(n: u32, mut d: u32) -> (u32, u32) {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut s = 1u32;
    while s < n {
        let rx = 1 & (d / 2);
        let ry = 1 & (d ^ rx);
        // rotate
        if ry == 0 {
            if rx == 1 {
                x = s - 1 - x;
                y = s - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        x += s * rx;
        y += s * ry;
        d /= 4;
        s *= 2;
    }
    (x, y)
}

fn curve(order: u32) -> Vec<(f64, f64)> {
    let n = 1u32 << order;
    let total = n * n;
    let mut pts = Vec::with_capacity(total as usize);
    for d in 0..total {
        let (x, y) = d2xy(n, d);
        let u = (x as f64 + 0.5) / n as f64;
        let v = (y as f64 + 0.5) / n as f64;
        pts.push((0.08 + 0.84 * u, 0.08 + 0.84 * v));
    }
    pts
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)], highlight: Option<usize>) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    for (i, w) in pts.windows(2).enumerate() {
        let a = to_px(w[0]);
        let b = to_px(w[1]);
        let ch = if highlight == Some(i) { '#' } else { '*' };
        canvas.line(a.0, a.1, b.0, b.1, ch);
    }
}

/// Hilbert curve room.
#[derive(Debug, Default)]
pub struct Hilbert {
    seed: u64,
}

impl Hilbert {
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

impl Room for Hilbert {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hilbert",
            title: "The Space-Filling Path",
            wing: "Shape & Space",
            blurb: "Hilbert curve: a continuous path that fills the square in the limit. Finite \
                    folds approximate without crossing. t and DRAG: DEEPEN THE FOLD.",
            accent: [180, 140, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let o = order(t, None);
        let pts = curve(o);
        let hi = if self.seed == 0 {
            None
        } else {
            Some((self.seed as usize) % pts.len().saturating_sub(1).max(1))
        };
        draw(canvas, &pts, hi);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hilbert fold",
            root: 246.94,
            tempo: 100,
            line: &[0, 2, 5, 7, 12, 7, 5, 2],
            encodes: "a path that folds until the square is full",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DEEPEN THE FOLD")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let cells = 1u32 << (2 * o);
        Some(format!("order={o}  cells={cells}  DRAG:FOLD"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        let pts = curve(o);
        let hi = hands.last().map(|&(x, y)| {
            // Nearest point index.
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, p) in pts.iter().enumerate() {
                let d = (p.0 - x).hypot(p.1 - y);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            best.min(pts.len().saturating_sub(2))
        });
        draw(canvas, &pts, hi);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let o = order(t, hands.last().copied());
        let cells = 1u32 << (2 * o);
        Some(format!("FOLD order={o}  cells={cells}"))
    }

    fn reveal(&self) -> &'static str {
        "A space-filling curve is continuous and surjective onto the square. \
         Hilbert's construction nests U-shaped turns so neighborhoods stay \
         local: a one-dimensional thread that becomes the plane."
    }
}

#[cfg(test)]
mod tests {
    use super::{Hilbert, curve, d2xy};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hilbert::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("FOLD"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn fold_changes() {
        let r = Hilbert::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.85,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn order2_has_16() {
        assert_eq!(curve(2).len(), 16);
        assert_eq!(d2xy(4, 0), (0, 0));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Hilbert::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Hilbert::new().motif().unwrap().line.len() >= 6);
    }
}
