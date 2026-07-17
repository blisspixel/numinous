//! Sierpinski arrowhead curve: a path that limits to the gasket.
//!
//! L-system rewrite on 60-degree turns. DRAG: SET THE ORDER.
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
        (2 + (x * 7.0) as usize).clamp(2, 9)
    } else {
        (3 + (phase_unit(t) * 5.0) as usize).clamp(2, 8)
    }
}

/// Build arrowhead as sequence of +1 (left 60), -1 (right 60), 0 (forward).
fn path(n: usize, seed: u64) -> Vec<(f64, f64)> {
    // Start axiom: A; A -> B-A-B; B -> A+B+A
    #[derive(Clone, Copy)]
    enum Sym {
        A,
        B,
        L,
        R,
    }
    let mut seq = vec![Sym::A];
    for _ in 0..n {
        let mut next = Vec::with_capacity(seq.len() * 4);
        for s in seq {
            match s {
                Sym::A => {
                    next.push(Sym::B);
                    next.push(Sym::R);
                    next.push(Sym::A);
                    next.push(Sym::R);
                    next.push(Sym::B);
                }
                Sym::B => {
                    next.push(Sym::A);
                    next.push(Sym::L);
                    next.push(Sym::B);
                    next.push(Sym::L);
                    next.push(Sym::A);
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
            let ch = if i % 5 == 0 { '#' } else { '*' };
            canvas.line(o.0, o.1, q.0, q.1, ch);
        }
        prev = Some(q);
    }
}

/// Sierpinski arrowhead room.
#[derive(Debug, Default)]
pub struct SierpinskiArrowhead {
    seed: u64,
}

impl SierpinskiArrowhead {
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

impl Room for SierpinskiArrowhead {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sierpinski-arrowhead",
            title: "Sierpinski Arrowhead",
            wing: "Fractals",
            blurb: "A continuous path whose limit is the Sierpinski gasket. t and DRAG: SET THE \
                    ORDER.",
            accent: [200, 80, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &path(order(t, None), self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "arrowhead",
            root: 311.13,
            tempo: 104,
            line: &[0, 5, 9, 5, 12, 9, 5, 0],
            encodes: "60-degree rewrite filling the gasket",
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
        // Arrowhead / Sierpinski: dim log2(3) ~ 1.585; 3^o segments.
        let segs = 3u64.saturating_pow(o as u32);
        let dim = 3.0_f64.ln() / 2.0_f64.ln();
        Some(format!("o={o}  pts={n}  3^{o}={segs}  d={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Sierpinski arrowhead is a space-filling path for the gasket: an \
         L-system whose limit set is the same triangular dust as Chaos Game and \
         Pascal mod 2, reached by walking instead of punching holes."
    }
}

#[cfg(test)]
mod tests {
    use super::{SierpinskiArrowhead, path};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SierpinskiArrowhead::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = SierpinskiArrowhead::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn path_grows() {
        assert!(path(2, 0).len() < path(4, 0).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        SierpinskiArrowhead::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(SierpinskiArrowhead::new().motif().unwrap().line.len() >= 6);
    }
}
