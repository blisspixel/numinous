//! Erdos-Renyi random graph: edges appear with probability p.
//!
//! DRAG: TUNE P. See `docs/ROOMS.md`.

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

fn edge_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        (x * 0.55 + s).clamp(0.0, 0.7)
    } else {
        (phase_unit(t) * 0.45 + s).clamp(0.0, 0.7)
    }
}

fn node_count(seed: u64) -> usize {
    8 + if seed == 0 { 0 } else { (seed % 5) as usize }
}

fn hash_u(a: u64, b: u64, salt: u64) -> f64 {
    let mut x = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add(salt);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    (x as f64) / (u64::MAX as f64)
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = node_count(seed);
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.38;
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let th = 2.0 * std::f64::consts::PI * (i as f64) / n as f64 - std::f64::consts::FRAC_PI_2;
        let x = (cx + r * th.cos()).round() as i32;
        let y = (cy + r * th.sin() * 0.9).round() as i32;
        pts.push((x, y));
        canvas.line(x - 1, y, x + 1, y, 'o');
        canvas.line(x, y - 1, x, y + 1, 'o');
    }
    for i in 0..n {
        for j in (i + 1)..n {
            let u = hash_u(i as u64, j as u64, seed.wrapping_mul(17) + 3);
            if u < p {
                canvas.line(pts[i].0, pts[i].1, pts[j].0, pts[j].1, '#');
            }
        }
    }
    // threshold marker: connectivity ~ log(n)/n
    let crit = (n as f64).ln() / n as f64;
    let mx = (crit.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx, height as i32 - 1, mx, height as i32 - 1, '+');
}

fn edge_count(n: usize, p: f64, seed: u64) -> usize {
    let mut edges = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            let u = hash_u(i as u64, j as u64, seed.wrapping_mul(17) + 3);
            if u < p {
                edges += 1;
            }
        }
    }
    edges
}

/// Erdos-Renyi room.
#[derive(Debug, Default)]
pub struct ErdosRenyi {
    seed: u64,
}

impl ErdosRenyi {
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

impl Room for ErdosRenyi {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "erdos-renyi",
            title: "Erdos-Renyi Graph",
            wing: "Chance & Noise",
            blurb: "Random edges with probability p. t and DRAG: TUNE P.",
            accent: [80, 110, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, edge_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "erdos-renyi",
            root: 185.0,
            tempo: 80,
            line: &[0, 3, 5, 8, 5, 3, 0, 10],
            encodes: "G(n,p): giant component and connectivity at p~log n / n",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = edge_p(t, None, self.seed);
        let n = node_count(self.seed);
        let e = edge_count(n, p, self.seed);
        let crit = (n as f64).ln() / n as f64;
        Some(format!("n={n}  e={e}  p={p:.2}  crit={crit:.2}  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = edge_p(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = edge_p(t, hands.last().copied(), self.seed);
        let n = node_count(self.seed);
        let e = edge_count(n, p, self.seed ^ hands.len() as u64);
        let max_e = n * n.saturating_sub(1) / 2;
        Some(format!("e={e}/{max_e}  p={p:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "In the Erdos-Renyi model G(n,p) each of the binom(n,2) edges appears \
         independently with probability p. Around p ~ log(n)/n the graph becomes \
         connected almost surely; below that it shatters into small pieces."
    }
}

#[cfg(test)]
mod tests {
    use super::ErdosRenyi;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ErdosRenyi::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains('p'));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = ErdosRenyi::new();
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
        ErdosRenyi::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
