//! Integer partitions: p(n) unrestricted partition counts.
//!
//! DRAG: TUNE N. See `docs/ROOMS.md`.

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

fn n_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 8) as f64 };
    if let Some((x, _)) = hand {
        10.0 + x * 50.0 + s
    } else {
        15.0 + phase_unit(t) * 40.0 + s
    }
}

/// p(0)..p(n) by the generating-function recurrence (parts as coins).
fn partitions_up_to(n: usize) -> Vec<u64> {
    let mut p = vec![0u64; n + 1];
    p[0] = 1;
    for part in 1..=n {
        for j in part..=n {
            p[j] = p[j].saturating_add(p[j - part]);
        }
    }
    p
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 70.0) as usize;
    let p = partitions_up_to(n);
    let max_p = *p.iter().max().unwrap_or(&1).max(&1) as f64;
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in p.iter().enumerate().skip(1) {
        let x = ((i as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - v as f64 / max_p) * height.saturating_sub(1) as f64 * 0.88
            + height as f64 * 0.06)
            .round() as i32;
        // Stem under the curve so p(n) is a filled growth, not a wire.
        let base = (height.saturating_sub(2)) as i32;
        canvas.line(x, y, x, base, '.');
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
            canvas.line(ox, oy + 1, x, y + 1, '*');
        }
        prev = Some((x, y));
    }
    let _ = seed;
}

/// Partition function room.
#[derive(Debug, Default)]
pub struct Partition {
    seed: u64,
}

impl Partition {
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

impl Room for Partition {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "partition",
            title: "Partition Function",
            wing: "Number & Pattern",
            blurb: "p(n): ways to write n as unordered sums. t and DRAG: TUNE N.",
            accent: [140, 90, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "partition",
            root: 30.87,
            tempo: 90,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "partition p(n): unrestricted splits of n into positive parts",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as usize;
        let p = partitions_up_to(n);
        let v = p.last().copied().unwrap_or(0);
        Some(format!("n={n}  p={v}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = n_param(t, hands.last().copied(), self.seed).round() as usize;
        let p = partitions_up_to(n);
        let v = p.last().copied().unwrap_or(0);
        // Growth ratio p(n)/p(n-1) when available.
        let growth = if p.len() >= 2 {
            let prev = p[p.len() - 2];
            if prev > 0 {
                v as f64 / prev as f64
            } else {
                0.0
            }
        } else {
            1.0
        };
        Some(format!("n={n}  p={v}  grow={growth:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The partition function p(n) counts ways to write n as a sum of positive \
         integers ignoring order. Euler's pentagonal-number theorem gives a \
         recurrence; Hardy and Ramanujan later found its explosive asymptotic growth."
    }
}

#[cfg(test)]
mod tests {
    use super::{Partition, partitions_up_to};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn small_partitions() {
        let p = partitions_up_to(7);
        assert_eq!(p[0], 1);
        assert_eq!(p[1], 1);
        assert_eq!(p[2], 2);
        assert_eq!(p[3], 3);
        assert_eq!(p[4], 5);
        assert_eq!(p[5], 7);
    }

    #[test]
    fn status_invites() {
        let s = Partition::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("p="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Partition::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Partition::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
