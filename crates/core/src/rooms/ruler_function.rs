//! Ruler function: 2-adic valuation of n, the height of paperfold marks.
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
    let s = if seed == 0 { 0.0 } else { (seed % 16) as f64 };
    if let Some((x, _)) = hand {
        16.0 + x * 112.0 + s
    } else {
        24.0 + phase_unit(t) * 96.0 + s
    }
}

fn v2(mut n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut c = 0u32;
    while n.is_multiple_of(2) {
        n /= 2;
        c += 1;
    }
    c
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 160.0) as u32;
    let base = height.saturating_sub(2) as i32;
    let scale = (height as f64 * 0.7) / 8.0;
    for i in 1..=n {
        let h = v2(i);
        let x = ((i as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let top = base - (h as f64 * scale).round() as i32;
        canvas.line(
            x,
            base,
            x,
            top,
            if h > 2 {
                '#'
            } else if h > 0 {
                '*'
            } else {
                '.'
            },
        );
    }
    let _ = seed;
}

/// Ruler function room.
#[derive(Debug, Default)]
pub struct RulerFunction {
    seed: u64,
}

impl RulerFunction {
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

impl Room for RulerFunction {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ruler-function",
            title: "Ruler Function",
            wing: "Number & Pattern",
            blurb: "2-adic height of n: paper ruler marks. t and DRAG: TUNE N.",
            accent: [40, 90, 70],
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
            key: "ruler-function",
            root: 41.2,
            tempo: 96,
            line: &[0, 7, 0, 5, 0, 12, 0, 7],
            encodes: "ruler function: valuation v2(n) draws paperfold tick heights",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round();
        Some(format!("n={n:.0}  v2  DRAG:N"))
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
        let n = n_param(t, hands.last().copied(), self.seed).round() as u32;
        let h = v2(n.max(1));
        Some(format!("N={n}  v2={h}"))
    }

    fn reveal(&self) -> &'static str {
        "The ruler function r(n) is the exponent of 2 in n: how many times you \
         can halve n. Plot it and you see the ticks of a measuring ruler, the \
         same pattern that appears in the Thue-Morse paperfold and in 2-adic \
         analysis."
    }
}

#[cfg(test)]
mod tests {
    use super::RulerFunction;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = RulerFunction::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("v2"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = RulerFunction::new();
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
        RulerFunction::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
