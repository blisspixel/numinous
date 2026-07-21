//! Stern-Brocot tree: every positive rational appears once.
//!
//! Mediant walk from 0/1 and 1/0. DRAG: WALK THE TREE. See `docs/ROOMS.md`.

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

fn levels(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (3 + (x * 7.0) as usize).clamp(3, 10)
    } else {
        (4 + (phase_unit(t) * 5.0) as usize).clamp(3, 9)
    }
}

struct Mediant {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    level: usize,
    max: usize,
    lo: f64,
    hi: f64,
}

/// Collect Stern-Brocot fractions up to depth as (p,q, depth, index).
fn tree(depth: usize) -> Vec<(u32, u32, usize, f64)> {
    let mut out = Vec::new();
    // Start with neighbors 0/1 and 1/0; place mediants recursively.
    fn rec(out: &mut Vec<(u32, u32, usize, f64)>, m: Mediant) {
        if m.level > m.max || out.len() > 4_000 {
            return;
        }
        let p = m.a + m.c;
        let q = m.b + m.d;
        let mid = (m.lo + m.hi) * 0.5;
        out.push((p, q, m.level, mid));
        rec(
            out,
            Mediant {
                a: m.a,
                b: m.b,
                c: p,
                d: q,
                level: m.level + 1,
                max: m.max,
                lo: m.lo,
                hi: mid,
            },
        );
        rec(
            out,
            Mediant {
                a: p,
                b: q,
                c: m.c,
                d: m.d,
                level: m.level + 1,
                max: m.max,
                lo: mid,
                hi: m.hi,
            },
        );
    }
    rec(
        &mut out,
        Mediant {
            a: 0,
            b: 1,
            c: 1,
            d: 0,
            level: 1,
            max: depth,
            lo: 0.0,
            hi: 1.0,
        },
    );
    out
}

fn draw(canvas: &mut dyn Surface, depth: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let nodes = tree(depth);
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    // Root markers 0/1 and 1/0 as ends of baseline.
    let base_y = (0.12 * height as f64).round() as i32;
    canvas.plot(2, base_y, '0');
    canvas.plot(width.saturating_sub(3) as i32, base_y, '1');
    for &(p, q, level, u) in &nodes {
        let uu = (u + shift).clamp(0.0, 1.0);
        let px = (uu * width.saturating_sub(1) as f64).round() as i32;
        let py = ((level as f64 / (depth as f64 + 1.0)) * height.saturating_sub(2) as f64).round()
            as i32;
        let ch = if q == 1 {
            '#'
        } else if p == 1 {
            '+'
        } else {
            '*'
        };
        canvas.plot(px, py, ch);
        // Thin stem up from parent band.
        if level > 1 {
            canvas.plot(px, py.saturating_sub(1), '.');
        }
        let _ = (p, q);
    }
}

/// Stern-Brocot tree room.
#[derive(Debug, Default)]
pub struct SternBrocot {
    seed: u64,
}

impl SternBrocot {
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

impl Room for SternBrocot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "stern-brocot",
            title: "Stern-Brocot Tree",
            wing: "Number & Pattern",
            blurb: "Every positive rational once, via mediants of 0/1 and 1/0. t and DRAG: WALK \
                    THE TREE.",
            accent: [80, 140, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, levels(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "stern brocot",
            root: 261.63,
            tempo: 98,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "mediants placing every positive rational once",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: WALK THE TREE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = levels(t, None);
        let n = tree(d).len();
        Some(format!("depth={d}  fracs={n}  DRAG:WALK"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = levels(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = levels(t, hands.last().copied());
        let nodes = tree(d);
        let u = hands.last().map(|&(x, _)| x).unwrap_or(0.5);
        let nearest = nodes
            .iter()
            .min_by(|a, b| {
                (a.3 - u)
                    .abs()
                    .partial_cmp(&(b.3 - u).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied();
        if let Some((p, q, _, _)) = nearest {
            Some(format!("WALK ~{p}/{q}  depth={d}"))
        } else {
            Some(format!("WALK depth={d}"))
        }
    }

    fn reveal(&self) -> &'static str {
        "The Stern-Brocot tree enumerates every positive rational exactly once \
         by repeated mediants of 0/1 and 1/0. Adjacent fractions a/b, c/d always \
         satisfy |ad-bc|=1: best approximations sit next door."
    }
}

#[cfg(test)]
mod tests {
    use super::{SternBrocot, tree};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SternBrocot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("WALK"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn walk_changes() {
        let r = SternBrocot::new();
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
    fn tree_grows() {
        assert!(tree(3).len() < tree(5).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        SternBrocot::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(SternBrocot::new().motif().unwrap().line.len() >= 6);
    }
}
