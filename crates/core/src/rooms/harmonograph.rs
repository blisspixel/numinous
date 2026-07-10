//! The harmonograph: the drawings a Victorian pendulum machine made.
//!
//! Two decaying oscillations on each axis trace a curve that spirals slowly
//! inward as the swings die away, like a Lissajous figure caught in amber. `t`
//! detunes the frequencies, opening and closing the weave. See `docs/ROOMS.md`.

use super::variation_signed;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Points along the traced curve.
const STEPS: usize = 4_000;
/// How far the pendulum's parameter runs (many swings).
const S_MAX: f64 = 60.0;
/// Damping: how fast the swings decay.
const DAMP: f64 = 0.012;

/// The harmonograph room.
#[derive(Debug, Default)]
pub struct Harmonograph {
    seed: u64,
}

impl Harmonograph {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn detune_for(&self, t: f64) -> f64 {
        (t.clamp(0.0, 1.0) - 0.5) * 0.06
            + variation_signed(self.seed, 0x4841_524D_4F4E_0001) * 0.035
    }
}

/// The pen position at parameter `s`, with a small frequency `detune`. In
/// `[-1, 1]` on each axis.
fn point(s: f64, detune: f64) -> (f64, f64) {
    let decay = (-DAMP * s).exp();
    let x = ((2.0 * s).sin() + ((3.0 + detune) * s + 1.0).sin()) * 0.5 * decay;
    let y = ((3.0 * s).sin() + ((2.0 - detune) * s + 0.7).sin()) * 0.5 * decay;
    (x, y)
}

impl Room for Harmonograph {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "harmonograph",
            title: "Harmonograph",
            wing: "Waves & Sound",
            blurb: "Two dying oscillations on each axis draw a curve that spirals inward as the \
                    pendulums lose energy. t detunes the frequencies to open and close the weave.",
            accent: [200, 80, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let detune = self.detune_for(t);
        let aspect = canvas.char_aspect();
        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let radius = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.9;

        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let s = S_MAX * i as f64 / STEPS as f64;
            let (x, y) = point(s, detune);
            let sx = (center_x + x * radius) as i32;
            let sy = (center_y - y * radius * aspect) as i32;
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
        }
    }

    fn reveal(&self) -> &'static str {
        "Before computers, this is how people saw a chord. Two frequencies in a \
         simple ratio draw a clean closed figure; nudge them apart and it blooms \
         into a rose that never quite repeats."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "F fading rose",
            root: 174.61,
            tempo: 84,
            line: &[0, 4, 7, 11, 9, 7, 4, 0],
            encodes: "pendulum harmonics blooming outward then decaying home",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DAMP, Harmonograph, point};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_pen_stays_in_range() {
        for i in 0..=100 {
            let (x, y) = point(i as f64 * 0.6, 0.0);
            assert!(x.abs() <= 1.0 && y.abs() <= 1.0);
        }
    }

    #[test]
    fn the_swing_decays() {
        let (x_early, _) = point(1.0, 0.0);
        let envelope_late = (-DAMP * 50.0_f64).exp();
        assert!(envelope_late < 1.0);
        // Late motion is bounded by the (smaller) envelope.
        let (x_late, y_late) = point(50.0, 0.0);
        assert!(x_late.abs() <= envelope_late + 1e-9);
        assert!(y_late.abs() <= envelope_late + 1e-9);
        assert!(x_early.abs() <= 1.0);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Harmonograph::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.4);
        room.render(&mut b, 0.4);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Harmonograph::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_a_chord() {
        assert!(Harmonograph::new().reveal().contains("chord"));
    }
}
