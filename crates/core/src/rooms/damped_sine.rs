//! Damped sinusoid gallery: exponential envelope on a pure wave.
//!
//! DRAG: TUNE DECAY. See `docs/ROOMS.md`.

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

fn decay(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        0.2 + x * 3.5 + s
    } else {
        0.5 + phase_unit(t) * 2.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.4;
    let omega = 8.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.4
        };
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = (-alpha * x).exp();
        let y = amp * env * (omega * std::f64::consts::TAU * x).sin();
        let px = col as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // envelope guides
    prev = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = (-alpha * x).exp();
        let py = (cy - amp * env).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
    prev = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = (-alpha * x).exp();
        let py = (cy + amp * env).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
}

/// Damped sinusoid room.
#[derive(Debug, Default)]
pub struct DampedSine {
    seed: u64,
}

impl DampedSine {
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

impl Room for DampedSine {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "damped-sine",
            title: "Damped Sine",
            wing: "Waves & Sound",
            blurb: "Exponential envelope on a pure oscillation. t and DRAG: TUNE DECAY.",
            accent: [40, 160, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, decay(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "damped sine",
            root: 349.23,
            tempo: 72,
            line: &[0, 5, 0, 7, 0, 12, 0, 5],
            encodes: "ring-down of a free oscillator under friction",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DECAY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = decay(t, None, self.seed);
        Some(format!("a={a:.2}  damp  DRAG:DECAY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = decay(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = decay(t, hands.last().copied(), self.seed).max(1e-6);
        let half = std::f64::consts::LN_2 / a;
        let end = (-a).exp();
        Some(format!("a={a:.2}  half={half:.2}  end={end:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "A damped sinusoid is e^{-a t} sin(omega t): the free ring-down of a \
         harmonic oscillator with friction. The envelope is pure exponential; \
         the zeros stay evenly spaced."
    }
}

#[cfg(test)]
mod tests {
    use super::DampedSine;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DampedSine::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DECAY") || s.contains("damp"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn decay_changes() {
        let r = DampedSine::new();
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
        DampedSine::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
