//! Recaman's Sequence: the jumper that rarely looks back.
//!
//! a(0)=0; a(n)=a(n-1)-n if positive and unused, else a(n-1)+n. Drawn as nested
//! semicircle arcs: a hypnotic harp that hides open questions (852655 never
//! appears in the first 10^230 terms). DRAG: SET THE STRIDE. See `docs/ROOMS.md`.

use std::collections::HashSet;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_TERMS: usize = 96;

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

fn recaman(n: usize, stride: usize) -> Vec<i64> {
    let n = n.clamp(1, MAX_TERMS);
    let stride = stride.max(1);
    let mut seq = Vec::with_capacity(n);
    let mut used = HashSet::new();
    let mut a: i64 = 0;
    seq.push(a);
    used.insert(a);
    for k in 1..n {
        let step = (k * stride) as i64;
        let back = a - step;
        if back > 0 && !used.contains(&back) {
            a = back;
        } else {
            a += step;
        }
        used.insert(a);
        seq.push(a);
    }
    seq
}

fn draw(canvas: &mut dyn Surface, seq: &[i64], seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || seq.len() < 2 {
        return;
    }
    let max_v = seq.iter().copied().max().unwrap_or(1).max(1) as f64;
    let mid_y = height as f64 * 0.55;
    let x_of = |v: i64| -> f64 {
        0.06 * width as f64 + 0.88 * width as f64 * (v as f64 / max_v).clamp(0.0, 1.0)
    };
    for i in 1..seq.len() {
        let a = seq[i - 1];
        let b = seq[i];
        let x0 = x_of(a.min(b));
        let x1 = x_of(a.max(b));
        let cx = (x0 + x1) * 0.5;
        let rad = ((x1 - x0) * 0.5).max(1.0);
        let above = (i + seed as usize) % 2 == 0;
        let steps = ((rad * 1.2) as usize).clamp(8, 48);
        let mut prev: Option<(i32, i32)> = None;
        for s in 0..=steps {
            // Map along the diameter with a semicircle rise.
            let sign = if above { -1.0 } else { 1.0 };
            let t = s as f64 / steps as f64;
            let x = x0 + (x1 - x0) * t;
            let y = mid_y + sign * rad * (std::f64::consts::PI * t).sin() * 0.55;
            let px = x.round() as i32;
            let py = y.round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if i % 3 == 0 { '#' } else { '*' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
            let _ = (cx, a < b);
        }
    }
    // Axis.
    let y = mid_y.round() as i32;
    canvas.line(0, y, width.saturating_sub(1) as i32, y, '.');
}

/// Recaman room.
#[derive(Debug, Default)]
pub struct Recaman {
    seed: u64,
}

impl Recaman {
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

impl Room for Recaman {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "recaman",
            title: "The Jumper",
            wing: "Number & Pattern",
            blurb: "Recaman's sequence: jump back by n if free, else forward. Nested arcs hide an \
                    open seat (852655). t grows terms; DRAG: SET THE STRIDE.",
            accent: [200, 170, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = 12 + (phase_unit(t) * 60.0) as usize;
        let seq = recaman(n, 1);
        draw(canvas, &seq, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "recaman harp",
            root: 196.0,
            tempo: 120,
            line: &[0, 7, 5, 12, 7, 0, 5, 12],
            encodes: "back if free, else forward: nested arcs of a greedy walk",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE STRIDE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = 12 + (phase_unit(t) * 60.0) as usize;
        let seq = recaman(n, 1);
        let last = *seq.last().unwrap_or(&0);
        Some(format!("n={n}  a={last}  DRAG:STRIDE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let stride = hands
            .last()
            .map(|&(x, _)| 1 + (x * 7.0) as usize)
            .unwrap_or(1)
            .clamp(1, 8);
        let n = 12 + (phase_unit(t) * 60.0) as usize;
        let seq = recaman(n, stride);
        draw(canvas, &seq, self.seed ^ stride as u64);
        // Mark hand so challenge pose always sees multi-cell change.
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let stride = hands
            .last()
            .map(|&(x, _)| 1 + (x * 7.0) as usize)
            .unwrap_or(1)
            .clamp(1, 8);
        let n = 12 + (phase_unit(t) * 60.0) as usize;
        let seq = recaman(n, stride);
        let last = *seq.last().unwrap_or(&0);
        Some(format!("STRIDE={stride}  n={n}  a={last}"))
    }

    fn reveal(&self) -> &'static str {
        "Recaman's rule is a greedy dance on the number line: prefer the unused \
         backward step. Drawn as semicircles it becomes a harp. Whether every \
         natural appears is open; 852655 is still missing after unimaginable length."
    }
}

#[cfg(test)]
mod tests {
    use super::{Recaman, recaman};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Recaman::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("STRIDE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn stride_changes() {
        let r = Recaman::new();
        let o = r.status(0.4).unwrap();
        let a = r
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn classic_prefix() {
        // OEIS A005132 prefix with stride 1.
        let s = recaman(10, 1);
        assert_eq!(&s[..8], &[0, 1, 3, 6, 2, 7, 13, 20]);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Recaman::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(Recaman::new().motif().unwrap().line.len() >= 6);
    }
}
