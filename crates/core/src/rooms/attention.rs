//! Attention as Soft Light: one query lights a few keys; the rest go dim.
//!
//! Softmax over dot products of a query with key vectors. DRAG: MOVE THE QUERY.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const N_KEYS: usize = 8;

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

fn keys(seed: u64) -> [(f64, f64); N_KEYS] {
    let mut out = [(0.0, 0.0); N_KEYS];
    for (i, slot) in out.iter_mut().enumerate() {
        let a = std::f64::consts::TAU * i as f64 / N_KEYS as f64
            + if seed == 0 {
                0.0
            } else {
                (seed % 7) as f64 * 0.02
            };
        let r = 0.28 + (i % 3) as f64 * 0.05;
        *slot = (0.5 + r * a.cos(), 0.5 + r * a.sin());
    }
    out
}

fn softmax_weights(query: (f64, f64), keys: &[(f64, f64); N_KEYS], temp: f64) -> [f64; N_KEYS] {
    let mut dots = [0.0; N_KEYS];
    let mut max = f64::NEG_INFINITY;
    for (i, k) in keys.iter().enumerate() {
        // Similarity: negative distance (local attention).
        let d = (query.0 - k.0).hypot(query.1 - k.1);
        dots[i] = -d / temp.max(0.05);
        max = max.max(dots[i]);
    }
    let mut w = [0.0; N_KEYS];
    let mut sum = 0.0;
    for i in 0..N_KEYS {
        w[i] = (dots[i] - max).exp();
        sum += w[i];
    }
    for wi in &mut w {
        *wi /= sum.max(1e-12);
    }
    w
}

fn entropy(w: &[f64; N_KEYS]) -> f64 {
    let mut h = 0.0;
    for &p in w {
        if p > 1e-12 {
            h -= p * p.ln();
        }
    }
    h
}

fn draw(
    canvas: &mut dyn Surface,
    query: (f64, f64),
    keys: &[(f64, f64); N_KEYS],
    w: &[f64; N_KEYS],
) {
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
    let q = to_px(query);
    for (i, k) in keys.iter().enumerate() {
        let p = to_px(*k);
        let ch = if w[i] > 0.25 {
            '#'
        } else if w[i] > 0.1 {
            '*'
        } else if w[i] > 0.04 {
            '+'
        } else {
            '.'
        };
        canvas.line(q.0, q.1, p.0, p.1, ch);
        canvas.plot(p.0, p.1, if w[i] > 0.2 { 'K' } else { 'k' });
    }
    canvas.plot(q.0, q.1, 'Q');
    // Weight bars along bottom.
    for (i, &wi) in w.iter().enumerate() {
        let x = ((0.1 + i as f64 * 0.1) * width as f64).round() as i32;
        let h = (wi * height as f64 * 0.35).max(1.0);
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - h).round() as i32;
        canvas.line(x, y1, x, y0, '=');
    }
}

/// Attention room.
#[derive(Debug, Default)]
pub struct Attention {
    seed: u64,
}

impl Attention {
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

impl Room for Attention {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "attention",
            title: "Attention as Soft Light",
            wing: "Number & Pattern",
            blurb: "One query lights a few keys; the rest go dim. Softmax weights are the story. t \
                    warms temperature; DRAG: MOVE THE QUERY.",
            accent: [255, 220, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let ks = keys(self.seed);
        let a = phase_unit(t) * std::f64::consts::TAU;
        let query = (0.5 + 0.2 * a.cos(), 0.5 + 0.15 * a.sin());
        let temp = 0.12 + phase_unit(t) * 0.15;
        let w = softmax_weights(query, &ks, temp);
        draw(canvas, query, &ks, &w);
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "soft light",
            root: 329.63,
            tempo: 104,
            line: &[0, 4, 7, 11, 7, 4, 0, 11],
            encodes: "a query lighting a few keys, dimming the rest",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MOVE THE QUERY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ks = keys(self.seed);
        let a = phase_unit(t) * std::f64::consts::TAU;
        let query = (0.5 + 0.2 * a.cos(), 0.5 + 0.15 * a.sin());
        let w = softmax_weights(query, &ks, 0.15);
        let h = entropy(&w);
        Some(format!("H={h:.2}  keys={N_KEYS}  DRAG:QUERY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ks = keys(self.seed);
        let query = hands.last().copied().unwrap_or((0.5, 0.5));
        let temp = 0.1 + phase_unit(t) * 0.2;
        let w = softmax_weights(query, &ks, temp);
        draw(canvas, query, &ks, &w);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let ks = keys(self.seed);
        let query = *hands.last().unwrap();
        let w = softmax_weights(query, &ks, 0.12 + phase_unit(t) * 0.15);
        let h = entropy(&w);
        let top = w
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        Some(format!("QUERY top=K{top}  H={h:.2}  soft"))
    }

    fn reveal(&self) -> &'static str {
        "Attention is a soft spotlight: a query scores keys, softmax turns \
         scores into weights, and values mix under that light. Geometry first; \
         transformers are the same picture with more axes."
    }
}

#[cfg(test)]
mod tests {
    use super::{Attention, keys, softmax_weights};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Attention::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("QUERY"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn query_changes() {
        let r = Attention::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn weights_sum_one() {
        let ks = keys(0);
        let w = softmax_weights((0.5, 0.5), &ks, 0.15);
        let s: f64 = w.iter().sum();
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Attention::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(Attention::new().motif().unwrap().line.len() >= 6);
    }
}
