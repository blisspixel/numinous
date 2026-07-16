//! The Learning Clock: continual learning; new task, old skill fades or holds.
//!
//! Two tasks on a shared weight. Train A, then B: measure retention of A.
//! TRAIN: TASK A, THEN B. See `docs/ROOMS.md`.

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

/// Loss for task with target weight w*.
fn loss(w: f64, target: f64) -> f64 {
    (w - target).powi(2)
}

/// Train toward target for n steps from w0.
fn train(mut w: f64, target: f64, steps: usize, lr: f64) -> f64 {
    for _ in 0..steps {
        w -= lr * 2.0 * (w - target);
    }
    w
}

fn schedule(t: f64, hand: Option<(f64, f64)>) -> (usize, usize) {
    // steps_A, steps_B
    if let Some((x, y)) = hand {
        let a = 5 + (x * 40.0) as usize;
        let b = 5 + (y * 40.0) as usize;
        (a, b)
    } else {
        let u = phase_unit(t);
        (10 + (u * 30.0) as usize, 5 + ((1.0 - u) * 35.0) as usize)
    }
}

fn run(seed: u64, steps_a: usize, steps_b: usize) -> (f64, f64, f64, f64) {
    let target_a = 1.0;
    let target_b = -1.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.1
        };
    let w0 = 0.0;
    let w_after_a = train(w0, target_a, steps_a, 0.15);
    let loss_a1 = loss(w_after_a, target_a);
    let w_after_b = train(w_after_a, target_b, steps_b, 0.15);
    let loss_a2 = loss(w_after_b, target_a);
    let loss_b = loss(w_after_b, target_b);
    (w_after_b, loss_a1, loss_a2, loss_b)
}

fn draw(canvas: &mut dyn Surface, w: f64, loss_a1: f64, loss_a2: f64, loss_b: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Weight on a line.
    let y = (0.35 * height as f64).round() as i32;
    canvas.line(
        (0.1 * width as f64).round() as i32,
        y,
        (0.9 * width as f64).round() as i32,
        y,
        '.',
    );
    let u = ((w + 1.5) / 3.0).clamp(0.0, 1.0);
    let wx = ((0.1 + u * 0.8) * width as f64).round() as i32;
    canvas.plot(wx, y, 'W');
    // Target marks
    let ta = ((0.1 + (1.0 + 1.5) / 3.0 * 0.8) * width as f64).round() as i32;
    let tb = ((0.1 + (-1.0 + 1.5) / 3.0 * 0.8) * width as f64).round() as i32;
    canvas.plot(ta, y - 1, 'A');
    canvas.plot(tb, y - 1, 'B');
    // Loss bars for A before/after B and B.
    let bars = [
        (loss_a1, 0.55, '1'),
        (loss_a2, 0.7, '2'),
        (loss_b, 0.85, 'B'),
    ];
    for (l, yf, ch) in bars {
        let yy = (yf * height as f64).round() as i32;
        let h = (l.min(2.0) / 2.0 * 0.6).clamp(0.02, 0.6);
        let x0 = (0.2 * width as f64).round() as i32;
        let x1 = ((0.2 + h) * width as f64).round() as i32;
        canvas.line(x0, yy, x1, yy, '=');
        canvas.plot(x0 - 2, yy, ch);
    }
}

/// Learning Clock room.
#[derive(Debug, Default)]
pub struct LearningClock {
    seed: u64,
}

impl LearningClock {
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

impl Room for LearningClock {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "learning-clock",
            title: "The Learning Clock",
            wing: "Number & Pattern",
            blurb: "Train task A, then B: does A survive? Continual learning as a felt trade. t and \
                    DRAG: TRAIN A THEN B.",
            accent: [80, 200, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (sa, sb) = schedule(t, None);
        let (w, la1, la2, lb) = run(self.seed, sa, sb);
        draw(canvas, w, la1, la2, lb);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "continual",
            root: 155.56,
            tempo: 104,
            line: &[0, 5, 7, 12, 7, 5, 0, 5],
            encodes: "a second task overwriting or sparing the first",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TRAIN A THEN B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (sa, sb) = schedule(t, None);
        let (_, la1, la2, _) = run(self.seed, sa, sb);
        let retain = (1.0 - (la2 / la1.max(1e-6)).min(2.0)).max(0.0);
        Some(format!("retainA={retain:.2}  steps={sa}/{sb}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (sa, sb) = schedule(t, hands.last().copied());
        let (w, la1, la2, lb) = run(self.seed ^ hands.len() as u64, sa, sb);
        draw(canvas, w, la1, la2, lb);
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
        let (sa, sb) = schedule(t, hands.last().copied());
        let (_, la1, la2, lb) = run(self.seed ^ hands.len() as u64, sa, sb);
        Some(format!(
            "A:{sa} B:{sb}  LA={la2:.2} LB={lb:.2}  was={la1:.2}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "Continual learning is the clock that never resets: a new task moves \
         shared weights, and old skill may fade. Plasticity and stability trade; \
         the same tension lives in every mind that keeps going."
    }
}

#[cfg(test)]
mod tests {
    use super::{LearningClock, run};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LearningClock::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("retain"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn train_changes() {
        let r = LearningClock::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.9,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn more_b_hurts_a() {
        let (_, _, la_short, _) = run(0, 30, 5);
        let (_, _, la_long, _) = run(0, 30, 40);
        assert!(la_long >= la_short - 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LearningClock::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(LearningClock::new().motif().unwrap().line.len() >= 6);
    }
}
