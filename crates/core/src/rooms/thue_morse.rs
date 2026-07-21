//! Thue-Morse sequence: overlap-free binary weather.
//!
//! t_n = sum of binary digits of n mod 2. Cube-free, overlap-free. Drawn as a
//! staircase and beat tape. DRAG: SET THE WINDOW. See `docs/ROOMS.md`.

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

fn window(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (32 + (x * 220.0) as usize).clamp(32, 256)
    } else {
        (48 + (phase_unit(t) * 160.0) as usize).clamp(32, 220)
    }
}

fn thue_bit(n: u32) -> u8 {
    n.count_ones() as u8 & 1
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || n == 0 {
        return;
    }
    let off = if seed == 0 { 0u32 } else { (seed % 17) as u32 };
    // Staircase path: 0 = east, 1 = north-east step.
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut prev: Option<(i32, i32)> = None;
    let scale_x = width.saturating_sub(4) as f64 / n as f64;
    let scale_y = height.saturating_sub(6) as f64 / (n as f64 * 0.5 + 1.0);
    for i in 0..n {
        let bit = thue_bit(i as u32 + off);
        x += 1.0;
        if bit == 1 {
            y += 1.0;
        }
        let px = (2.0 + x * scale_x).round() as i32;
        let py = (height as f64 - 3.0 - y * scale_y).round() as i32;
        if let Some(o) = prev {
            let ch = if bit == 1 { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
    // Beat tape along the bottom.
    let yb = height.saturating_sub(2) as i32;
    let show = n.min(width.saturating_sub(2));
    for (i, idx) in (0..show).enumerate() {
        let bit = thue_bit(idx as u32 + off);
        canvas.plot((1 + i) as i32, yb, if bit == 1 { '|' } else { '.' });
    }
}

/// Thue-Morse room.
#[derive(Debug, Default)]
pub struct ThueMorse {
    seed: u64,
}

impl ThueMorse {
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

impl Room for ThueMorse {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "thue-morse",
            title: "Thue-Morse Weather",
            wing: "Number & Pattern",
            blurb: "Parity of binary digit sum: cube-free automatic sequence. t and DRAG: SET THE \
                    WINDOW.",
            accent: [80, 180, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, window(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "thue morse",
            root: 185.0,
            tempo: 112,
            line: &[0, 0, 5, 5, 7, 7, 12, 0],
            encodes: "parity of ones in binary indices",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = window(t, None);
        let ones = (0..n).filter(|&i| thue_bit(i as u32) == 1).count();
        Some(format!("N={n}  1s={ones}  DRAG:WINDOW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = window(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = window(t, hands.last().copied());
        let ones = (0..n).filter(|&i| thue_bit(i as u32) == 1).count();
        let dens = ones as f64 / n.max(1) as f64;
        Some(format!("WIN N={n}  dens1={dens:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Thue-Morse sequence t_n is the parity of the number of 1-bits in n. \
         It is overlap-free and cube-free: no block XXX appears, and it refuses \
         the patterns that free monoids would allow. Prouhet used it to partition \
         powers; paper-folders meet it as the paperfolding cousin of automatic sequences."
    }
}

#[cfg(test)]
mod tests {
    use super::{ThueMorse, thue_bit};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ThueMorse::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("WINDOW"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn window_changes() {
        let r = ThueMorse::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn bits_known() {
        assert_eq!(thue_bit(0), 0);
        assert_eq!(thue_bit(1), 1);
        assert_eq!(thue_bit(2), 1);
        assert_eq!(thue_bit(3), 0);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        ThueMorse::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(ThueMorse::new().motif().unwrap().line.len() >= 6);
    }
}
