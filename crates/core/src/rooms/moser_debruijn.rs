//! Moser-de Bruijn sequence: sums of distinct powers of 4.
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
        8.0 + x * 56.0 + s
    } else {
        12.0 + phase_unit(t) * 48.0 + s
    }
}

/// Moser-de Bruijn: replace binary digits of n by base-4 digits (0,1 only).
fn mdb(n: u32) -> u32 {
    let mut x = n;
    let mut r = 0u32;
    let mut p = 1u32;
    while x > 0 {
        r += (x & 1) * p;
        x >>= 1;
        p = p.saturating_mul(4);
    }
    r
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(4.0, 80.0) as u32;
    let mut vals: Vec<u32> = (0..n).map(mdb).collect();
    let max_v = vals.iter().copied().max().unwrap_or(1).max(1) as f64;
    let mut prev: Option<(i32, i32)> = None;
    for (i, v) in vals.iter().enumerate() {
        let x = ((i as f64 / n.saturating_sub(1).max(1) as f64) * width.saturating_sub(1) as f64)
            .round() as i32;
        let y = ((1.0 - (*v as f64 / max_v)) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        canvas.line(x, y - 1, x, y + 1, '*');
        prev = Some((x, y));
    }
    // Mark the "square" property: every natural is uniquely m+2*n with m,n in the set.
    let _ = seed;
    let _ = &mut vals;
}

/// Moser-de Bruijn room.
#[derive(Debug, Default)]
pub struct MoserDebruijn {
    seed: u64,
}

impl MoserDebruijn {
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

impl Room for MoserDebruijn {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "moser-debruijn",
            title: "Moser-de Bruijn",
            wing: "Number & Pattern",
            blurb: "Sums of distinct powers of 4. t and DRAG: TUNE N.",
            accent: [90, 50, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "moser-debruijn",
            root: 38.89,
            tempo: 98,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "Moser-de Bruijn: binary to base-4, unique m+2n splits of N",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round();
        Some(format!("n={n:.0}  base4  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
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
        let n = n_param(t, hands.last().copied(), self.seed).round() as u32;
        let v = mdb(n);
        Some(format!("N={n}  m={v}"))
    }

    fn reveal(&self) -> &'static str {
        "The Moser-de Bruijn sequence lists numbers whose base-4 digits are only \
         0 and 1: 0,1,4,5,16,17,... Every natural number has a unique write-up as \
         m + 2 n with m and n both in the sequence."
    }
}

#[cfg(test)]
mod tests {
    use super::{MoserDebruijn, mdb};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn sequence_values() {
        assert_eq!(mdb(0), 0);
        assert_eq!(mdb(1), 1);
        assert_eq!(mdb(2), 4);
        assert_eq!(mdb(3), 5);
    }

    #[test]
    fn status_invites() {
        let s = MoserDebruijn::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("base4"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = MoserDebruijn::new();
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
        MoserDebruijn::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
