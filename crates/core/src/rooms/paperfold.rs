//! Regular paperfolding sequence: fold a strip, read mountain/valley bits.
//!
//! DRAG: TUNE LENGTH. See `docs/ROOMS.md`.

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

fn length(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 16) as f64 };
    if let Some((x, _)) = hand {
        16.0 + x * 120.0 + s
    } else {
        24.0 + phase_unit(t) * 100.0 + s
    }
}

/// Paperfold bit at 1-based index n: 1 if floor((n mod 2^{v+1})/2^v)=1 for v=v2(n).
fn paperfold_bit(n: u32) -> u8 {
    if n == 0 {
        return 0;
    }
    // OEIS A014577: for n>=1, write n = m*2^k with m odd; bit is 1 if m ≡ 1 mod 4.
    let mut m = n;
    while m.is_multiple_of(2) {
        m /= 2;
    }
    if m % 4 == 1 { 1 } else { 0 }
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 160.0) as u32;
    // Dragon-like polyline: turn by paperfold bits.
    let mut x = width as f64 * 0.15;
    let mut y = height as f64 * 0.5;
    let mut dir = 0i32; // 0E 1N 2W 3S
    let step = (width.min(height) as f64) * 0.9 / (n as f64).sqrt().max(4.0);
    let mut prev = (x.round() as i32, y.round() as i32);
    for i in 1..=n {
        let bit = paperfold_bit(i);
        // turn left on 1, right on 0 (regular paperfold dragon)
        dir = if bit == 1 {
            (dir + 1).rem_euclid(4)
        } else {
            (dir - 1).rem_euclid(4)
        };
        match dir {
            0 => x += step,
            1 => y -= step,
            2 => x -= step,
            _ => y += step,
        }
        let px = x.round() as i32;
        let py = y.round() as i32;
        canvas.line(prev.0, prev.1, px, py, if bit == 1 { '#' } else { '*' });
        prev = (px, py);
    }
    let _ = seed;
}

/// Paperfold sequence room.
#[derive(Debug, Default)]
pub struct Paperfold {
    seed: u64,
}

impl Paperfold {
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

impl Room for Paperfold {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "paperfold",
            title: "Paperfold Sequence",
            wing: "Number & Pattern",
            blurb: "Regular fold bits draw a dragon path. t and DRAG: TUNE LENGTH.",
            accent: [50, 90, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, length(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "paperfold",
            root: 29.14,
            tempo: 94,
            line: &[0, 7, 5, 12, 3, 7, 0, 12],
            encodes: "paperfold sequence: odd part mod 4 steers dragon turns",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LENGTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = length(t, None, self.seed).round();
        Some(format!("n={n:.0}  fold  DRAG:LEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = length(t, hands.last().copied(), self.seed);
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
        let n = length(t, hands.last().copied(), self.seed).round() as u32;
        let n = n.max(1);
        let b = paperfold_bit(n);
        // Dragon/paperfold: fold bit is the ruler-like sequence; count 1s up to n.
        let mut ones = 0u32;
        for i in 1..=n.min(256) {
            ones += u32::from(paperfold_bit(i));
        }
        Some(format!("n={n}  bit={b}  ones<={ones}"))
    }

    fn reveal(&self) -> &'static str {
        "The regular paperfolding sequence records mountain versus valley creases \
         when you fold a strip in half repeatedly. Its turns generate the Heighway \
         dragon curve, a space-filling fractal limit."
    }
}

#[cfg(test)]
mod tests {
    use super::{Paperfold, paperfold_bit};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn bits_start() {
        // 1,1,0,1,1,0,0,... for n=1,2,3,4,5,6,7
        assert_eq!(paperfold_bit(1), 1);
        assert_eq!(paperfold_bit(2), 1);
        assert_eq!(paperfold_bit(3), 0);
        assert_eq!(paperfold_bit(4), 1);
    }

    #[test]
    fn status_invites() {
        let s = Paperfold::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("fold"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn length_changes() {
        let r = Paperfold::new();
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
        Paperfold::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
