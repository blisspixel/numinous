//! Huffman coding: merge lightest frequencies into a code tree.
//!
//! DRAG: TUNE S. See `docs/ROOMS.md`.

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

fn skew(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 2.5 + s
    } else {
        0.8 + phase_unit(t) * 2.0 + s
    }
    .clamp(0.4, 3.5)
}

fn freqs(skew: f64, n: usize) -> Vec<f64> {
    // geometric-ish weights
    let mut w: Vec<f64> = (0..n)
        .map(|i| (-skew * (i as f64) / n as f64).exp())
        .collect();
    let sum: f64 = w.iter().sum();
    for v in &mut w {
        *v /= sum.max(1e-12);
    }
    w
}

/// Build Huffman depths by repeatedly merging two lightest.
fn depths(weights: &[f64]) -> Vec<usize> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    // nodes: (weight, leaf_count, depth_sum for leaves tracked separately)
    #[derive(Clone)]
    struct Node {
        w: f64,
        leaves: Vec<usize>,
    }
    let mut forest: Vec<Node> = weights
        .iter()
        .enumerate()
        .map(|(i, &w)| Node { w, leaves: vec![i] })
        .collect();
    let mut depth = vec![0usize; n];
    while forest.len() > 1 {
        forest.sort_by(|a, b| a.w.partial_cmp(&b.w).unwrap_or(std::cmp::Ordering::Equal));
        let a = forest.remove(0);
        let b = forest.remove(0);
        for &i in a.leaves.iter().chain(b.leaves.iter()) {
            depth[i] += 1;
        }
        let mut leaves = a.leaves;
        leaves.extend(b.leaves);
        forest.push(Node {
            w: a.w + b.w,
            leaves,
        });
    }
    depth
}

fn draw(canvas: &mut dyn Surface, skew_v: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = 8usize;
    let w = freqs(skew_v, n);
    let d = depths(&w);
    let max_d = d.iter().copied().max().unwrap_or(1).max(1);
    let bar_w = (width / n).max(2);
    let pad = if seed == 0 { 0i32 } else { (seed % 2) as i32 };
    for i in 0..n {
        let x0 = (i * bar_w) as i32 + pad;
        // weight bar (bottom)
        let wh = (w[i] * height as f64 * 0.45).round() as i32;
        let y1 = height as i32 - 1;
        canvas.line(x0, y1 - wh, x0, y1, '#');
        // depth stem (top)
        let dh = ((d[i] as f64 / max_d as f64) * height as f64 * 0.4).round() as i32;
        canvas.line(x0 + 1, 1, x0 + 1, 1 + dh, '=');
    }
    // average code length line
    let avg: f64 = w
        .iter()
        .zip(d.iter())
        .map(|(&wi, &di)| wi * di as f64)
        .sum();
    let ay = ((avg / max_d as f64) * height as f64 * 0.4).round() as i32;
    canvas.line(0, ay, width as i32 - 1, ay, '-');
}

/// Huffman tree room.
#[derive(Debug, Default)]
pub struct HuffmanTree {
    seed: u64,
}

impl HuffmanTree {
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

impl Room for HuffmanTree {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "huffman-tree",
            title: "Huffman Tree",
            wing: "Chance & Noise",
            blurb: "Optimal prefix codes from frequencies. t and DRAG: TUNE S.",
            accent: [70, 120, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, skew(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "huffman-tree",
            root: 220.0,
            tempo: 76,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "Huffman: merge lightest nodes for shortest expected code",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE S")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = skew(t, None, self.seed);
        let w = freqs(s, 8);
        let d = depths(&w);
        let avg: f64 = w
            .iter()
            .zip(d.iter())
            .map(|(&wi, &di)| wi * di as f64)
            .sum();
        Some(format!("avg={avg:.2}b  DRAG:S"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = skew(t, hands.last().copied(), self.seed);
        draw(canvas, s, self.seed ^ hands.len() as u64);
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
        let s = skew(t, hands.last().copied(), self.seed);
        let w = freqs(s, 8);
        let d = depths(&w);
        let avg: f64 = w
            .iter()
            .zip(d.iter())
            .map(|(&wi, &di)| wi * di as f64)
            .sum();
        Some(format!("avg={avg:.3}  huff"))
    }

    fn reveal(&self) -> &'static str {
        "Huffman coding builds an optimal prefix-free binary code: repeatedly \
         merge the two lightest symbols into a parent. Common symbols get short \
         codes; rare ones get long. Average length sits near the Shannon entropy."
    }
}

#[cfg(test)]
mod tests {
    use super::HuffmanTree;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HuffmanTree::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("avg"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn s_changes() {
        let r = HuffmanTree::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        HuffmanTree::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
