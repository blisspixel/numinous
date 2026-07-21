//! Cesaro fractal (torn square): Koch-like with 90 degree turns on a square.
//!
//! DRAG: SET THE ORDER. See `docs/ROOMS.md`.

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

fn order(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 5.0) as usize).clamp(1, 6)
    } else {
        (2 + (phase_unit(t) * 3.0) as usize).clamp(1, 5)
    }
}

fn cesaro_edge(a: (f64, f64), b: (f64, f64), n: usize, out: &mut Vec<(f64, f64)>) {
    if n == 0 || out.len() > 12_000 {
        out.push(b);
        return;
    }
    let (ax, ay) = a;
    let (bx, by) = b;
    let dx = bx - ax;
    let dy = by - ay;
    let p1 = (ax + dx / 3.0, ay + dy / 3.0);
    let p3 = (ax + 2.0 * dx / 3.0, ay + 2.0 * dy / 3.0);
    // 90 degree peak (Cesaro).
    let mx = p3.0 - p1.0;
    let my = p3.1 - p1.1;
    let p2 = (p1.0 - my, p1.1 + mx);
    cesaro_edge(a, p1, n - 1, out);
    cesaro_edge(p1, p2, n - 1, out);
    cesaro_edge(p2, p3, n - 1, out);
    cesaro_edge(p3, b, n - 1, out);
}

fn torn_square(n: usize) -> Vec<(f64, f64)> {
    let a = (0.15, 0.15);
    let b = (0.85, 0.15);
    let c = (0.85, 0.85);
    let d = (0.15, 0.85);
    let mut out = vec![a];
    cesaro_edge(a, b, n, &mut out);
    cesaro_edge(b, c, n, &mut out);
    cesaro_edge(c, d, n, &mut out);
    cesaro_edge(d, a, n, &mut out);
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if i % 5 == 0 { '#' } else { '*' });
        }
        prev = Some((px, py));
    }
}

/// Cesaro fractal room.
#[derive(Debug, Default)]
pub struct Cesaro {
    seed: u64,
}

impl Cesaro {
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

impl Room for Cesaro {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cesaro",
            title: "Cesaro Fractal",
            wing: "Fractals",
            blurb: "Torn square: Koch rewrite with right angles. t and DRAG: SET THE ORDER.",
            accent: [180, 100, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let mut o = order(t, None);
        if self.seed != 0 {
            o = (o + (self.seed % 2) as usize).clamp(1, 6);
        }
        draw(canvas, &torn_square(o));
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cesaro",
            root: 207.65,
            tempo: 92,
            line: &[0, 5, 2, 7, 12, 7, 2, 5],
            encodes: "right-angle Koch rewrite around a square",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = torn_square(o).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &torn_square(o));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let o = order(t, hands.last().copied());
        let n = torn_square(o).len();
        let dim = 4.0_f64.ln() / 3.0_f64.ln();
        Some(format!("o={o}  pts={n}  dim={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Cesaro's fractal is a square-based Koch construction with right-angle \
         peaks. Like the snowflake, the perimeter grows without bound while the \
         region it bounds stays controlled."
    }
}

#[cfg(test)]
mod tests {
    use super::{Cesaro, torn_square};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cesaro::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = Cesaro::new();
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
    fn grows() {
        assert!(torn_square(1).len() < torn_square(3).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Cesaro::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Cesaro::new().motif().unwrap().line.len() >= 6);
    }
}
