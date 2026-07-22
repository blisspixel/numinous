//! Collatz reverse tree: hailstone ancestors as a branching graph.
//!
//! Distinct from the collatz orbit room: shows inverse branches.
//! DRAG: SET THE ROOT AND DEPTH. See `docs/ROOMS.md`.

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

fn params(t: f64, hand: Option<(f64, f64)>) -> (u64, usize) {
    if let Some((x, y)) = hand {
        let root = 1 + (x * 30.0) as u64;
        let depth = (3 + (y * 8.0) as usize).clamp(3, 12);
        (root, depth)
    } else {
        let u = phase_unit(t);
        (1 + (u * 12.0) as u64, (4 + (u * 5.0) as usize).clamp(3, 10))
    }
}

fn ancestors(n: u64) -> Vec<u64> {
    // Inverse Collatz: always 2n; and (n-1)/3 if integer and odd predecessor form
    let mut out = vec![n.saturating_mul(2)];
    if n > 1 && (n.saturating_sub(1)).is_multiple_of(3) {
        let m = (n - 1) / 3;
        if m > 0 && m % 2 == 1 {
            out.push(m);
        }
    }
    out
}

fn draw(canvas: &mut dyn Surface, root: u64, depth: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // BFS layers with parent links so the tree reads as branches, not freckles.
    let mut layer: Vec<(u64, f64)> = vec![(root, 0.5)];
    let mut nodes: Vec<(u64, usize, f64)> = vec![(root, 0, 0.5)];
    let mut edges: Vec<(f64, usize, f64, usize)> = Vec::new();
    for d in 0..depth {
        let mut next = Vec::new();
        let count = layer.len().max(1);
        for (i, &(n, parent_u)) in layer.iter().enumerate() {
            let base = (i as f64 + 0.5) / count as f64;
            for (j, &a) in ancestors(n).iter().enumerate() {
                if a > 10_000 {
                    continue;
                }
                let u = (base + (j as f64 - 0.5) * 0.12 / (d as f64 + 1.0)).clamp(0.02, 0.98);
                nodes.push((a, d + 1, u));
                edges.push((parent_u, d, u, d + 1));
                next.push((a, u));
                if next.len() > 200 {
                    break;
                }
            }
            if next.len() > 200 {
                break;
            }
        }
        layer = next;
        if layer.is_empty() {
            break;
        }
    }
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.01
    };
    let h = height.saturating_sub(2) as f64;
    let w = width.saturating_sub(1) as f64;
    let y_of = |d: usize| ((d as f64 / depth.max(1) as f64) * h).round() as i32;
    for &(pu, pd, cu, cd) in &edges {
        let px0 = ((pu + shift).fract().clamp(0.02, 0.98) * w).round() as i32;
        let py0 = y_of(pd);
        let px1 = ((cu + shift).fract().clamp(0.02, 0.98) * w).round() as i32;
        let py1 = y_of(cd);
        canvas.line(px0, py0, px1, py1, '*');
        canvas.line(px0, py0 + 1, px1, py1 + 1, '.');
    }
    for &(n, d, u) in &nodes {
        let uu = (u + shift).fract().clamp(0.02, 0.98);
        let px = (uu * w).round() as i32;
        let py = y_of(d);
        let ch = if n % 2 == 0 { '*' } else { '#' };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx * dx + dy * dy <= 2 {
                    canvas.plot(px + dx, py + dy, ch);
                }
            }
        }
    }
}

/// Collatz reverse tree room.
#[derive(Debug, Default)]
pub struct CollatzTree {
    seed: u64,
}

impl CollatzTree {
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

impl Room for CollatzTree {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "collatz-tree",
            title: "Collatz Tree",
            wing: "Number & Pattern",
            blurb: "Inverse hailstone branches from a root. t and DRAG: SET THE ROOT AND DEPTH.",
            accent: [180, 80, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (r, d) = params(t, None);
        draw(canvas, r, d, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "collatz tree",
            root: 123.47,
            tempo: 130,
            line: &[0, 7, 0, 12, 5, 0, 7, 14],
            encodes: "inverse hailstone branches of the Collatz map",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ROOT AND DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (r, d) = params(t, None);
        Some(format!("root={r}  depth={d}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (r, d) = params(t, hands.last().copied());
        draw(canvas, r, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (r, d) = params(t, hands.last().copied());
        // Inverse-Collatz BFS size (match draw: stop filling next past 200).
        let mut layer = vec![r];
        let mut nodes = 1u32;
        for _ in 0..d {
            let mut next = Vec::new();
            for &n in &layer {
                for a in ancestors(n) {
                    if a > 10_000 {
                        continue;
                    }
                    if next.len() >= 200 {
                        break;
                    }
                    next.push(a);
                    nodes += 1;
                }
                if next.len() >= 200 {
                    break;
                }
            }
            layer = next;
            if layer.is_empty() {
                break;
            }
        }
        Some(format!("root={r}  d={d}  nodes={nodes}"))
    }

    fn reveal(&self) -> &'static str {
        "The Collatz conjecture asks whether every positive integer reaches 1 \
         under the hailstone map. The reverse tree grows all preimages: always \
         double, and sometimes the (n-1)/3 odd branch."
    }
}

#[cfg(test)]
mod tests {
    use super::CollatzTree;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CollatzTree::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("root"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn root_changes() {
        let r = CollatzTree::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CollatzTree::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 30, "tree edges and nodes must be visible");
    }

    #[test]
    fn motif_ok() {
        assert!(CollatzTree::new().motif().unwrap().line.len() >= 6);
    }
}
