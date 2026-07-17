//! Peano curve: space-filling path that visits every square.
//!
//! Recursive nine-subdivision (Hilbert is already a room; this is Peano's
//! original idea). DRAG: SET THE ORDER. See `docs/ROOMS.md`.

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

/// Peano curve points in unit square via recursive 3x3 traversal.
fn peano(n: usize) -> Vec<(f64, f64)> {
    let mut pts = vec![(0.0, 0.0)];
    peano_rec(&mut pts, 0.0, 0.0, 1.0, n, false, false);
    pts
}

fn peano_rec(
    pts: &mut Vec<(f64, f64)>,
    x: f64,
    y: f64,
    s: f64,
    n: usize,
    flip_x: bool,
    flip_y: bool,
) {
    if n == 0 || pts.len() > 20_000 {
        pts.push((x + s * 0.5, y + s * 0.5));
        return;
    }
    let s3 = s / 3.0;
    // Nine cells in a serpentine that fills the square.
    let order = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 2),
        (1, 1),
        (1, 0),
        (2, 0),
        (2, 1),
        (2, 2),
    ];
    for (i, &(cx, cy)) in order.iter().enumerate() {
        let mut ux = cx;
        let mut uy = cy;
        if flip_x {
            ux = 2 - ux;
        }
        if flip_y {
            uy = 2 - uy;
        }
        let nx = x + ux as f64 * s3;
        let ny = y + uy as f64 * s3;
        // Alternate orientation so the path connects.
        let fx = flip_x ^ (i % 2 == 1);
        let fy = flip_y ^ (i / 3 % 2 == 1);
        peano_rec(pts, nx, ny, s3, n - 1, fx, fy);
    }
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let u = 0.06 + 0.88 * x.clamp(0.0, 1.0);
        let v = 0.06 + 0.88 * y.clamp(0.0, 1.0);
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            let ch = if i % 9 == 0 { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Peano curve room.
#[derive(Debug, Default)]
pub struct PeanoCurve {
    seed: u64,
}

impl PeanoCurve {
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

impl Room for PeanoCurve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "peano-curve",
            title: "Peano's Path",
            wing: "Fractals",
            blurb: "A continuous curve that fills the square (order recursion). t and DRAG: SET \
                    THE ORDER.",
            accent: [100, 200, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let mut o = order(t, None);
        if self.seed != 0 {
            o = (o + (self.seed % 2) as usize).clamp(1, 5);
        }
        draw(canvas, &peano(o));
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "peano",
            root: 174.61,
            tempo: 90,
            line: &[0, 5, 2, 7, 4, 9, 6, 11],
            encodes: "a path that learns to fill area",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = peano(o).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &peano(o));
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
        let n = peano(o).len();
        // Space-filling: 9-fold at each step.
        let cells = 9u64.saturating_pow(o as u32);
        Some(format!("o={o}  pts={n}  9^{o}={cells}"))
    }

    fn reveal(&self) -> &'static str {
        "Peano (1890) constructed a continuous surjection from the unit interval \
         onto the unit square: a curve with area. Hilbert simplified the \
         geometry; both prove dimension can jump under continuous maps."
    }
}

#[cfg(test)]
mod tests {
    use super::{PeanoCurve, peano};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PeanoCurve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = PeanoCurve::new();
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
    fn peano_grows() {
        assert!(peano(1).len() > 2);
        assert!(peano(2).len() > peano(1).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        PeanoCurve::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(PeanoCurve::new().motif().unwrap().line.len() >= 6);
    }
}
