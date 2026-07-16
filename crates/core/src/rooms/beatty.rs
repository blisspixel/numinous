//! Beatty sequence: floor(n r) partitions N when 1/r + 1/s = 1.
//!
//! DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn r_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    // r > 1; classic golden: phi = (1+sqrt5)/2
    if let Some((x, _)) = hand {
        1.2 + x * 2.0 + s
    } else {
        1.3 + phase_unit(t) * 1.8 + s
    }
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = r.clamp(1.15, 3.5);
    let s = r / (r - 1.0); // complementary: 1/r + 1/s = 1
    let n_max = width.max(24);
    let y_a = (height as f64 * 0.35).round() as i32;
    let y_b = (height as f64 * 0.7).round() as i32;
    let mut prev_a: Option<(i32, i32)> = None;
    let mut prev_b: Option<(i32, i32)> = None;
    for n in 1..=n_max {
        let ax = ((n as f64 / n_max as f64) * width.saturating_sub(1) as f64).round() as i32;
        let av = (n as f64 * r).floor() as i32;
        let bv = (n as f64 * s).floor() as i32;
        let ay = y_a - (av % (height as i32 / 4 + 1));
        let by = y_b - (bv % (height as i32 / 4 + 1));
        if let Some((ox, oy)) = prev_a {
            canvas.line(ox, oy, ax, ay, '#');
        }
        if let Some((ox, oy)) = prev_b {
            canvas.line(ox, oy, ax, by, '*');
        }
        prev_a = Some((ax, ay));
        prev_b = Some((ax, by));
    }
    let _ = seed;
}

/// Beatty sequence room.
#[derive(Debug, Default)]
pub struct Beatty {
    seed: u64,
}

impl Beatty {
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

impl Room for Beatty {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "beatty",
            title: "Beatty Sequence",
            wing: "Number & Pattern",
            blurb: "floor(n r) and floor(n s) partition N. t and DRAG: TUNE R.",
            accent: [100, 60, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, r_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "beatty",
            root: 49.0,
            tempo: 90,
            line: &[0, 5, 7, 12, 5, 0, 7, 12],
            encodes: "Beatty: complementary irrationals tile the natural numbers",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None, self.seed);
        let s = r / (r - 1.0);
        Some(format!("r={r:.2}  s={s:.2}  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
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
        let r = r_param(t, hands.last().copied(), self.seed).clamp(1.15, 3.5);
        let s = r / (r - 1.0);
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let dphi = (r - phi).abs();
        // Sample partition: unique floor values among first 40 of each seq.
        // Cap at 320 so floor(40 * s) for s up to ~8 still fits.
        let mut seen = [false; 320];
        let mut hits = 0u32;
        for n in 1..=40 {
            let a = (n as f64 * r).floor() as usize;
            let b = (n as f64 * s).floor() as usize;
            if a < seen.len() && !seen[a] {
                seen[a] = true;
                hits += 1;
            }
            if b < seen.len() && !seen[b] {
                seen[b] = true;
                hits += 1;
            }
        }
        Some(format!("r={r:.2}  s={s:.2}  |r-phi|={dphi:.3}  u={hits}"))
    }

    fn reveal(&self) -> &'static str {
        "Beatty's theorem: if r and s are irrationals greater than 1 with \
         1/r + 1/s = 1, then floor(n r) and floor(n s) partition the positive \
         integers with no overlaps and no gaps. The golden ratio is the classic pair."
    }
}

#[cfg(test)]
mod tests {
    use super::Beatty;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Beatty::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("r="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = Beatty::new();
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
        Beatty::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
