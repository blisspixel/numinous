//! Penrose kite and dart: aperiodic tiling from matching rules.
//!
//! Seed a kite; subdivide by Robinson triangle inflation. The pattern never
//! repeats by translation. CLICK: INFLATE ONCE. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const PHI: f64 = 1.618_033_988_749_895;

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

#[derive(Clone, Copy)]
struct Tri {
    /// Robinson triangle: acute (true) or obtuse (false).
    acute: bool,
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
}

fn gens(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 4.0) as usize).clamp(1, 5)
    } else {
        (1 + (phase_unit(t) * 3.0) as usize).clamp(1, 4)
    }
}

fn mid(p: (f64, f64), q: (f64, f64), t: f64) -> (f64, f64) {
    (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)
}

fn subdivide(t: Tri) -> Vec<Tri> {
    // Standard Robinson P2 subdivision (toy).
    if t.acute {
        // A--B short, A--C long, B--C long; split along golden.
        let p = mid(t.a, t.b, 1.0 / PHI);
        vec![
            Tri {
                acute: true,
                a: t.c,
                b: p,
                c: t.a,
            },
            Tri {
                acute: false,
                a: p,
                b: t.c,
                c: t.b,
            },
        ]
    } else {
        let q = mid(t.b, t.a, 1.0 / PHI);
        vec![
            Tri {
                acute: true,
                a: t.b,
                b: q,
                c: t.c,
            },
            Tri {
                acute: false,
                a: q,
                b: t.c,
                c: t.a,
            },
        ]
    }
}

fn seed_star(seed: u64) -> Vec<Tri> {
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 10) as f64 * 0.05
    };
    let mut out = Vec::new();
    let cx = 0.5;
    let cy = 0.5;
    let r = 0.42;
    for i in 0..5 {
        let a0 = rot + TAU * i as f64 / 5.0 - TAU / 4.0;
        let a1 = rot + TAU * (i + 1) as f64 / 5.0 - TAU / 4.0;
        let b = (cx + r * a0.cos(), cy + r * a0.sin());
        let c = (cx + r * a1.cos(), cy + r * a1.sin());
        out.push(Tri {
            acute: true,
            a: (cx, cy),
            b,
            c,
        });
    }
    out
}

fn inflate(tris: Vec<Tri>, depth: usize) -> Vec<Tri> {
    let mut cur = tris;
    for _ in 0..depth {
        let mut next = Vec::with_capacity(cur.len() * 2);
        for t in cur {
            next.extend(subdivide(t));
        }
        cur = next;
        if cur.len() > 2_000 {
            break;
        }
    }
    cur
}

fn draw(canvas: &mut dyn Surface, tris: &[Tri]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    for t in tris {
        let a = to_px(t.a);
        let b = to_px(t.b);
        let c = to_px(t.c);
        let ch = if t.acute { '*' } else { '#' };
        canvas.line(a.0, a.1, b.0, b.1, ch);
        canvas.line(b.0, b.1, c.0, c.1, ch);
        canvas.line(c.0, c.1, a.0, a.1, ch);
    }
}

/// Penrose tiling room.
#[derive(Debug, Default)]
pub struct Penrose {
    seed: u64,
}

impl Penrose {
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

impl Room for Penrose {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "penrose",
            title: "The Aperiodic Floor",
            wing: "Shape & Space",
            blurb: "Penrose kites from Robinson triangles: inflation never yields a lattice. t and \
                    CLICK: INFLATE ONCE.",
            accent: [220, 180, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let g = gens(t, None);
        let tris = inflate(seed_star(self.seed), g);
        draw(canvas, &tris);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "penrose",
            root: 185.0,
            tempo: 104,
            line: &[0, 5, 7, 12, 16, 12, 7, 5],
            encodes: "golden inflation that forbids a repeating lattice",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: INFLATE ONCE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gens(t, None);
        let n = inflate(seed_star(self.seed), g).len();
        Some(format!("gen={g}  tris={n}  phi  CLICK:INFLATE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gens(t, hands.last().copied());
        let tris = inflate(seed_star(self.seed ^ hands.len() as u64), g);
        draw(canvas, &tris);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let g = gens(t, hands.last().copied());
        let n = inflate(seed_star(self.seed ^ hands.len() as u64), g).len();
        Some(format!("INFLATE gen={g}  tris={n}  aperiodic"))
    }

    fn reveal(&self) -> &'static str {
        "Penrose tilings cover the plane without translational period. Local \
         matching rules force a global quasi-crystal: order without a lattice, \
         the same idea quasicrystals wear in metal."
    }
}

#[cfg(test)]
mod tests {
    use super::{Penrose, inflate, seed_star};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Penrose::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("INFLATE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn inflate_changes() {
        let r = Penrose::new();
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
    fn grows() {
        let a = inflate(seed_star(0), 1).len();
        let b = inflate(seed_star(0), 2).len();
        assert!(b > a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Penrose::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Penrose::new().motif().unwrap().line.len() >= 6);
    }
}
