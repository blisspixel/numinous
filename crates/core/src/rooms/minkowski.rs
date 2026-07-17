//! Minkowski sausage (question-mark related polycurve toy): alternating Koch.
//!
//! A quadratic Koch variant with alternating turns. DRAG: SET THE ORDER.
//! See `docs/ROOMS.md`.

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
        (1 + (x * 4.0) as usize).clamp(1, 5)
    } else {
        (1 + (phase_unit(t) * 3.0) as usize).clamp(1, 4)
    }
}

fn sausage_edge(a: (f64, f64), b: (f64, f64), n: usize, out: &mut Vec<(f64, f64)>) {
    if n == 0 || out.len() > 10_000 {
        out.push(b);
        return;
    }
    let (ax, ay) = a;
    let (bx, by) = b;
    let dx = bx - ax;
    let dy = by - ay;
    // Eight-step Minkowski sausage generator (quadratic Koch type 2).
    let pts = [
        (ax, ay),
        (ax + dx * 0.25, ay + dy * 0.25),
        (ax + dx * 0.25 - dy * 0.25, ay + dy * 0.25 + dx * 0.25),
        (ax + dx * 0.5 - dy * 0.25, ay + dy * 0.5 + dx * 0.25),
        (ax + dx * 0.5, ay + dy * 0.5),
        (ax + dx * 0.5 + dy * 0.25, ay + dy * 0.5 - dx * 0.25),
        (ax + dx * 0.75 + dy * 0.25, ay + dy * 0.75 - dx * 0.25),
        (ax + dx * 0.75, ay + dy * 0.75),
        (bx, by),
    ];
    for w in pts.windows(2) {
        sausage_edge(w[0], w[1], n - 1, out);
    }
}

fn curve(n: usize) -> Vec<(f64, f64)> {
    let mut out = vec![(0.1, 0.5)];
    sausage_edge((0.1, 0.5), (0.9, 0.5), n, &mut out);
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if i % 6 == 0 { '#' } else { '*' });
        }
        prev = Some((px, py));
    }
}

/// Minkowski sausage room.
#[derive(Debug, Default)]
pub struct Minkowski {
    seed: u64,
}

impl Minkowski {
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

impl Room for Minkowski {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "minkowski-sausage",
            title: "Minkowski Sausage",
            wing: "Fractals",
            blurb: "Quadratic Koch sausage: a thick fractal polyline. t and DRAG: SET THE ORDER.",
            accent: [160, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let mut o = order(t, None);
        if self.seed != 0 {
            o = (o + (self.seed % 2) as usize).clamp(1, 5);
        }
        draw(canvas, &curve(o));
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "minkowski",
            root: 146.83,
            tempo: 84,
            line: &[0, 4, 2, 6, 8, 12, 8, 4],
            encodes: "alternating square bumps on a Koch generator",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = curve(o).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &curve(o));
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
        let o = order(t, hands.last().copied());
        let n = curve(o).len();
        // Minkowski sausage dim = ln8/ln4 = 1.5.
        let dim = 8.0_f64.ln() / 4.0_f64.ln();
        Some(format!("o={o}  pts={n}  dim={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Minkowski sausage is a quadratic Koch curve with square-wave \
         bumps. Perimeter grows exponentially with order while the curve stays \
         a continuous polyline."
    }
}

#[cfg(test)]
mod tests {
    use super::{Minkowski, curve};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Minkowski::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = Minkowski::new();
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
        assert!(curve(1).len() < curve(2).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Minkowski::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Minkowski::new().motif().unwrap().line.len() >= 6);
    }
}
