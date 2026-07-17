//! Zipf law: rank-frequency power law p(k) ~ 1/k^s.
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

fn exponent(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 1.8 + s
    } else {
        0.7 + phase_unit(t) * 1.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, s: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let s = s.clamp(0.4, 2.5);
    let n = 40 + if seed == 0 { 0 } else { (seed % 10) as i32 };
    // Normalize Zipf over 1..n
    let mut z = 0.0;
    for k in 1..=n {
        z += (k as f64).powf(-s);
    }
    let mut prev: Option<(i32, i32)> = None;
    for k in 1..=n {
        let p = (k as f64).powf(-s) / z;
        let x = ((k as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - p / ((1.0_f64).powf(-s) / z)) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        // bar
        let base = height.saturating_sub(2) as i32;
        canvas.line(x, base, x, y, if k <= 3 { '#' } else { '*' });
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '.');
        }
        prev = Some((x, y));
    }
}

/// Zipf law room.
#[derive(Debug, Default)]
pub struct Zipf {
    seed: u64,
}

impl Zipf {
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

impl Room for Zipf {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "zipf",
            title: "Zipf Law",
            wing: "Chance & Order",
            blurb: "Rank-frequency power law 1/k^s. t and DRAG: TUNE S.",
            accent: [90, 50, 110],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, exponent(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "zipf",
            root: 20.6,
            tempo: 98,
            line: &[0, 7, 12, 5, 0, 12, 7, 3],
            encodes: "Zipf: frequency falls as power of rank in language and cities",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE S")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = exponent(t, None, self.seed);
        Some(format!("s={s:.2}  1/k^s  DRAG:S"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = exponent(t, hands.last().copied(), self.seed);
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
        let s = exponent(t, hands.last().copied(), self.seed).clamp(0.4, 2.5);
        let n = 40
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 10) as i32
            };
        let mut z = 0.0_f64;
        for k in 1..=n {
            z += (k as f64).powf(-s);
        }
        let p1 = (1.0_f64).powf(-s) / z;
        let p1_pct = (p1 * 100.0).round() as i32;
        Some(format!("s={s:.2}  P1={p1_pct}%  n={n}"))
    }

    fn reveal(&self) -> &'static str {
        "Zipf's law says the frequency of the k-th most common word (or city, \
         or website) falls roughly as 1/k^s. A single exponent captures heavy \
         tails across language, geography, and the web."
    }
}

#[cfg(test)]
mod tests {
    use super::Zipf;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Zipf::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("1/k"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn s_changes() {
        let r = Zipf::new();
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
        Zipf::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
