//! Chebyshev nodes: min-max interpolation points kill Runge oscillation.
//!
//! DRAG: TUNE DEGREE. See `docs/ROOMS.md`.

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

fn degree(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 10.0 + s
    } else {
        3.0 + phase_unit(t) * 8.0 + s
    }
}

fn runge_f(x: f64) -> f64 {
    1.0 / (1.0 + 25.0 * x * x)
}

fn cheb_node(i: usize, n: usize) -> f64 {
    // Roots of T_{n+1}: cos( (2i+1)pi / (2n+2) )
    let nn = n + 1;
    ((2.0 * i as f64 + 1.0) * std::f64::consts::PI / (2.0 * nn as f64)).cos()
}

fn lagrange_cheb(x: f64, n: usize) -> f64 {
    if n < 2 {
        return runge_f(x);
    }
    let mut y = 0.0;
    for i in 0..=n {
        let xi = cheb_node(i, n);
        let yi = runge_f(xi);
        let mut li = 1.0;
        for j in 0..=n {
            if i == j {
                continue;
            }
            let xj = cheb_node(j, n);
            li *= (x - xj) / (xi - xj);
        }
        y += yi * li;
    }
    y
}

fn draw(canvas: &mut dyn Surface, deg: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = deg.round().clamp(2.0, 14.0) as usize;
    let y0 = height as f64 * 0.75;
    let y_scale = height as f64 * 0.35;
    // True f.
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -1.0 + 2.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = runge_f(x);
        let py = (y0 - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
    // Chebyshev interpolant.
    prev = None;
    for col in 0..width {
        let x = -1.0 + 2.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = lagrange_cheb(x, n).clamp(-0.5, 1.5);
        let py = (y0 - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Nodes denser near edges.
    for i in 0..=n {
        let x = cheb_node(i, n);
        let y = runge_f(x);
        let px = (((x + 1.0) * 0.5) * width.saturating_sub(1) as f64).round() as i32;
        let py = (y0 - y * y_scale).round() as i32;
        canvas.line(px, py - 1, px, py + 1, '*');
    }
    let _ = seed;
}

/// Chebyshev nodes room.
#[derive(Debug, Default)]
pub struct Chebyshev {
    seed: u64,
}

impl Chebyshev {
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

impl Room for Chebyshev {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chebyshev",
            title: "Chebyshev Nodes",
            wing: "Number & Pattern",
            blurb: "Min-max nodes tame Runge edges. t and DRAG: TUNE DEGREE.",
            accent: [40, 140, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, degree(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chebyshev",
            root: 698.46,
            tempo: 104,
            line: &[0, 4, 7, 11, 12, 11, 7, 4],
            encodes: "Chebyshev nodes cluster at edges and kill Runge blow-up",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DEGREE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = degree(t, None, self.seed);
        Some(format!("n={:.0}  Tnodes  DRAG:DEG", d.round()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = degree(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let d = degree(t, hands.last().copied(), self.seed);
        Some(format!("N={:.0}  cheb", d.round()))
    }

    fn reveal(&self) -> &'static str {
        "Chebyshev nodes are the projections of equal angles on a circle. They \
         pack denser near the ends of the interval, so high-degree polynomial \
         fits stay stable where equispaced nodes explode."
    }
}

#[cfg(test)]
mod tests {
    use super::Chebyshev;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Chebyshev::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("Tnodes"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn degree_changes() {
        let r = Chebyshev::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Chebyshev::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 0);
    }
}
