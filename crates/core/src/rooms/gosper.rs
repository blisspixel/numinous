//! Gosper curve (flowsnake): space-filling heptagonal path.
//!
//! L-system with 60-degree turns. DRAG: SET THE ORDER. See `docs/ROOMS.md`.

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

#[derive(Clone, Copy)]
enum Sym {
    A,
    B,
    L,
    R,
}

fn path(n: usize, seed: u64) -> Vec<(f64, f64)> {
    // A -> A-B--B+A++AA+B-
    // B -> +A-BB--B-A++A+B
    let mut seq = vec![Sym::A];
    for _ in 0..n {
        let mut next = Vec::with_capacity(seq.len() * 8);
        for s in seq {
            match s {
                Sym::A => {
                    next.extend([
                        Sym::A,
                        Sym::R,
                        Sym::B,
                        Sym::R,
                        Sym::R,
                        Sym::B,
                        Sym::L,
                        Sym::A,
                        Sym::L,
                        Sym::L,
                        Sym::A,
                        Sym::A,
                        Sym::L,
                        Sym::B,
                        Sym::R,
                    ]);
                }
                Sym::B => {
                    next.extend([
                        Sym::L,
                        Sym::A,
                        Sym::R,
                        Sym::B,
                        Sym::B,
                        Sym::R,
                        Sym::R,
                        Sym::B,
                        Sym::R,
                        Sym::A,
                        Sym::L,
                        Sym::L,
                        Sym::A,
                        Sym::L,
                        Sym::B,
                    ]);
                }
                other => next.push(other),
            }
        }
        seq = next;
        if seq.len() > 20_000 {
            break;
        }
    }
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut dir = if seed == 0 { 0i32 } else { (seed % 6) as i32 };
    let mut pts = vec![(x, y)];
    for s in seq {
        match s {
            Sym::L => dir = (dir + 1).rem_euclid(6),
            Sym::R => dir = (dir - 1).rem_euclid(6),
            Sym::A | Sym::B => {
                let a = dir as f64 * std::f64::consts::FRAC_PI_3;
                x += a.cos();
                y += a.sin();
                pts.push((x, y));
            }
        }
    }
    pts
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
    for (i, &p) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (p.0 - min_x) / dx;
        let v = 0.08 + 0.84 * (p.1 - min_y) / dy;
        let q = (
            (u * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32,
        );
        if let Some(o) = prev {
            canvas.line(o.0, o.1, q.0, q.1, if i % 5 == 0 { '#' } else { '*' });
        }
        prev = Some(q);
    }
}

/// Gosper curve room.
#[derive(Debug, Default)]
pub struct Gosper {
    seed: u64,
}

impl Gosper {
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

impl Room for Gosper {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gosper",
            title: "Gosper Curve",
            wing: "Fractals",
            blurb: "Flowsnake: space-filling path on a hexagonal lattice. t and DRAG: SET THE \
                    ORDER.",
            accent: [60, 180, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &path(order(t, None), self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gosper",
            root: 185.0,
            tempo: 94,
            line: &[0, 5, 9, 12, 9, 5, 2, 7],
            encodes: "heptagonal rewrite filling a plane patch",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = path(o, self.seed).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &path(o, self.seed ^ hands.len() as u64));
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
        let n = path(o, self.seed).len();
        // Flowsnake: 7-fold replacement, dim = log7/log(sqrt7) = 2.
        let tiles = 7u64.saturating_pow(o as u32);
        Some(format!("o={o}  pts={n}  tiles={tiles}"))
    }

    fn reveal(&self) -> &'static str {
        "The Gosper curve (flowsnake) is a space-filling curve based on a \
         hexagonal lattice. Its L-system tiles the plane with a self-similar \
         path of constant width in the limit."
    }
}

#[cfg(test)]
mod tests {
    use super::{Gosper, path};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Gosper::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = Gosper::new();
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
    fn path_grows() {
        assert!(path(1, 0).len() < path(2, 0).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Gosper::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Gosper::new().motif().unwrap().line.len() >= 6);
    }
}
