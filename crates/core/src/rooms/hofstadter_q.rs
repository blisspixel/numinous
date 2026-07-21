//! Hofstadter Q-sequence: chaotic integer recursion plotted as a skyline.
//!
//! DRAG: SET THE LENGTH. See `docs/ROOMS.md`.

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

fn length(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (20 + (x * 200.0) as usize).clamp(20, 240)
    } else {
        (40 + (phase_unit(t) * 140.0) as usize).clamp(20, 200)
    }
}

fn q_seq(n: usize) -> Vec<u32> {
    let mut q = vec![0u32; n + 1];
    if n >= 1 {
        q[1] = 1;
    }
    if n >= 2 {
        q[2] = 1;
    }
    for i in 3..=n {
        let a = q[i - 1] as usize;
        let b = q[i - 2] as usize;
        let i1 = i.saturating_sub(a).max(1);
        let i2 = i.saturating_sub(b).max(1);
        q[i] = q[i1].saturating_add(q[i2]);
    }
    q
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n.clamp(3, 300);
    let q = q_seq(n);
    let max_q = q.iter().copied().max().unwrap_or(1).max(1) as f64;
    let shift = if seed == 0 { 0 } else { (seed % 5) as usize };
    for col in 0..width {
        let i = (1 + (col + shift) * n / width.max(1)).clamp(1, n);
        let v = q[i] as f64 / max_q;
        let top = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let bot = height.saturating_sub(1) as i32;
        canvas.line(
            col as i32,
            top,
            col as i32,
            bot,
            if v > 0.5 { '#' } else { '*' },
        );
    }
}

/// Hofstadter Q room.
#[derive(Debug, Default)]
pub struct HofstadterQ {
    seed: u64,
}

impl HofstadterQ {
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

impl Room for HofstadterQ {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hofstadter-q",
            title: "Hofstadter Q",
            wing: "Number & Pattern",
            blurb: "Chaotic integer recursion as a skyline. t and DRAG: SET THE LENGTH.",
            accent: [100, 40, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, length(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hofstadter q",
            root: 69.3,
            tempo: 72,
            line: &[0, 0, 3, 7, 0, 12, 5, 0],
            encodes: "Q(n) looks back at itself and refuses a closed form",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE LENGTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = length(t, None);
        Some(format!("n={n}  Q-seq  DRAG:LEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = length(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = length(t, hands.last().copied());
        let q = q_seq(n);
        let last = q[n];
        // Mean of first n terms vs n/2 (meta-Fibonacci chaos).
        let mean = if n > 0 {
            q[1..=n].iter().map(|&v| v as f64).sum::<f64>() / n as f64
        } else {
            0.0
        };
        Some(format!("n={n}  Q={last}  mean={mean:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "Hofstadter's Q-sequence is Q(n) = Q(n-Q(n-1)) + Q(n-Q(n-2)) with \
         Q(1)=Q(2)=1. It grows on average like n/2 yet looks chaotic: a \
         recursive integer skyline with no known closed form."
    }
}

#[cfg(test)]
mod tests {
    use super::HofstadterQ;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HofstadterQ::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("LEN") || s.contains("n="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn length_changes() {
        let r = HofstadterQ::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        HofstadterQ::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
