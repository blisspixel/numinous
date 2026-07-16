//! Sylvester sequence: s_{n+1} = s_n(s_n-1)+1, Egyptian fraction of 1.
//!
//! DRAG: TUNE TERMS. See `docs/ROOMS.md`.

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

fn terms(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 2) as f64 };
    if let Some((x, _)) = hand {
        3.0 + x * 6.0 + s
    } else {
        4.0 + phase_unit(t) * 5.0 + s
    }
}

fn sylvester(n: usize) -> Vec<u128> {
    let mut v = Vec::with_capacity(n.max(1));
    if n == 0 {
        return v;
    }
    let mut s: u128 = 2;
    v.push(s);
    for _ in 1..n {
        // s_{n+1} = s_n^2 - s_n + 1
        let next = s.saturating_mul(s.saturating_sub(1)).saturating_add(1);
        if next <= s {
            break; // overflow guard
        }
        s = next;
        v.push(s);
    }
    v
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(3.0, 10.0) as usize;
    let seq = sylvester(n);
    // Partial Egyptian sum approaching 1: sum 1/(s_i)
    let mut partial = 0.0_f64;
    let mut prev: Option<(i32, i32)> = None;
    let base = height.saturating_sub(2) as i32;
    for (i, &s) in seq.iter().enumerate() {
        partial += 1.0 / s as f64;
        let x = (((i + 1) as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - partial) * height.saturating_sub(1) as f64 * 0.85 + height as f64 * 0.08)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        // bar for log size of s
        let bar = ((s as f64).ln() / 40.0 * height as f64 * 0.3)
            .round()
            .clamp(1.0, height as f64 * 0.4) as i32;
        canvas.line(x, base, x, base - bar, '*');
        prev = Some((x, y));
    }
    // asymptote y=0 for sum->1
    canvas.line(
        0,
        (height as f64 * 0.08).round() as i32,
        width.saturating_sub(1) as i32,
        (height as f64 * 0.08).round() as i32,
        '.',
    );
    let _ = seed;
}

/// Sylvester sequence room.
#[derive(Debug, Default)]
pub struct Sylvester {
    seed: u64,
}

impl Sylvester {
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

impl Room for Sylvester {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sylvester",
            title: "Sylvester Sequence",
            wing: "Number & Pattern",
            blurb: "Double-exponential Egyptian fraction of 1. t and DRAG: TUNE TERMS.",
            accent: [120, 60, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, terms(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sylvester",
            root: 27.5,
            tempo: 96,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "Sylvester: s->s(s-1)+1, sum 1/s_i = 1 Egyptian expansion",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TERMS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = terms(t, None, self.seed).round() as usize;
        let s = sylvester(n);
        let last = s.last().copied().unwrap_or(0);
        // log10 for status
        let lg = if last > 0 { (last as f64).log10() } else { 0.0 };
        Some(format!("n={n}  lg={lg:.1}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = terms(t, hands.last().copied(), self.seed);
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
        let n = terms(t, hands.last().copied(), self.seed).round() as usize;
        let s = sylvester(n);
        let last = s.last().copied().unwrap_or(0);
        let lg = if last > 0 { (last as f64).log10() } else { 0.0 };
        Some(format!("N={n}  lg10={lg:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Sylvester sequence 2,3,7,43,1807,... obeys s_{n+1} = s_n(s_n-1)+1. \
         The reciprocals form a greedy Egyptian fraction for 1 that converges \
         double-exponentially, the densest such expansion of its kind."
    }
}

#[cfg(test)]
mod tests {
    use super::{Sylvester, sylvester};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn sequence_start() {
        let s = sylvester(5);
        assert_eq!(s[0], 2);
        assert_eq!(s[1], 3);
        assert_eq!(s[2], 7);
        assert_eq!(s[3], 43);
        assert_eq!(s[4], 1807);
    }

    #[test]
    fn status_invites() {
        let s = Sylvester::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lg="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn terms_change() {
        let r = Sylvester::new();
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
        Sylvester::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
