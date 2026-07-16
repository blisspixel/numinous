//! Calkin-Wilf tree: another complete rational enumeration.
//!
//! Left child a/(a+b), right (a+b)/b from root 1/1. Drawn as level bands.
//! DRAG: SET THE DEPTH. See `docs/ROOMS.md`.

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
        (2 + (x * 8.0) as usize).clamp(2, 10)
    } else {
        (3 + (phase_unit(t) * 6.0) as usize).clamp(2, 9)
    }
}

fn enumerate(depth: usize) -> Vec<(u32, u32, usize, f64)> {
    let mut out = Vec::new();
    fn rec(
        out: &mut Vec<(u32, u32, usize, f64)>,
        a: u32,
        b: u32,
        level: usize,
        max: usize,
        u: f64,
    ) {
        if level > max || out.len() > 4_000 {
            return;
        }
        out.push((a, b, level, u));
        if level == max {
            return;
        }
        // Left a/(a+b), right (a+b)/b
        let w = 0.5f64.powi(level as i32 + 1);
        rec(out, a, a + b, level + 1, max, u - w);
        rec(out, a + b, b, level + 1, max, u + w);
    }
    rec(&mut out, 1, 1, 0, depth, 0.5);
    out
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let nodes = enumerate(d);
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.01
    };
    for &(a, b, level, u) in &nodes {
        let uu = (u + shift).clamp(0.05, 0.95);
        let px = (uu * width.saturating_sub(1) as f64).round() as i32;
        let py =
            ((level as f64 / d.max(1) as f64) * height.saturating_sub(2) as f64).round() as i32;
        let ch = if a == b {
            '@'
        } else if a == 1 || b == 1 {
            '#'
        } else {
            '*'
        };
        canvas.plot(px, py, ch);
        if level > 0 {
            canvas.plot(px, py.saturating_sub(1), '.');
        }
        let _ = (a, b);
    }
}

/// Calkin-Wilf tree room.
#[derive(Debug, Default)]
pub struct CalkinWilf {
    seed: u64,
}

impl CalkinWilf {
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

impl Room for CalkinWilf {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "calkin-wilf",
            title: "Calkin-Wilf Tree",
            wing: "Number & Pattern",
            blurb: "Every positive rational once via left a/(a+b) and right (a+b)/b. t and DRAG: \
                    SET THE DEPTH.",
            accent: [40, 160, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "calkin wilf",
            root: 174.61,
            tempo: 110,
            line: &[0, 3, 7, 12, 7, 3, 5, 10],
            encodes: "binary tree of all positive rationals",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        let n = enumerate(d).len();
        Some(format!("depth={d}  nodes={n}  DRAG:DEPTH"))
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
        let nodes = enumerate(d);
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
        if let Some((a, b, _, _)) = nearest {
            Some(format!("DEPTH={d}  ~{a}/{b}"))
        } else {
            Some(format!("DEPTH={d}"))
        }
    }

    fn reveal(&self) -> &'static str {
        "The Calkin-Wilf tree labels every positive rational exactly once: from \
         a/b, the left child is a/(a+b) and the right is (a+b)/b. Reading left \
         to right by levels gives a complete listing without reduction."
    }
}

#[cfg(test)]
mod tests {
    use super::{CalkinWilf, enumerate};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CalkinWilf::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = CalkinWilf::new();
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
        assert!(enumerate(2).len() < enumerate(4).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CalkinWilf::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(CalkinWilf::new().motif().unwrap().line.len() >= 6);
    }
}
