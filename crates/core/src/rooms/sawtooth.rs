//! Sawtooth Fourier partials: all harmonics with 1/k amplitudes.
//!
//! DRAG: SET HARMONICS. See `docs/ROOMS.md`.

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

fn harmonics(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 20.0) as usize).clamp(1, 24)
    } else {
        (2 + (phase_unit(t) * 16.0) as usize).clamp(1, 20)
    }
}

fn saw_partial(x: f64, n: usize) -> f64 {
    // (2/pi) sum_{k=1}^n (-1)^{k+1} sin(k pi x) / k   for period 2 on [-1,1]
    // use period 1: (2/pi) sum sin(2 pi k x)/k with sign
    let mut s = 0.0;
    for k in 1..=n {
        let sign = if k % 2 == 0 { -1.0 } else { 1.0 };
        s += sign * (std::f64::consts::TAU * k as f64 * x).sin() / k as f64;
    }
    2.0 * s / std::f64::consts::PI
}

fn draw(canvas: &mut dyn Surface, n: usize, scroll: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.4;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    // Scroll so the ramp marches; partials chase the ideal flyback.
    let scroll = scroll.rem_euclid(1.0);
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64 + j + scroll;
        let y = saw_partial(x.fract(), n).clamp(-1.4, 1.4);
        let py = (cy - amp * y / 1.2).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
            canvas.line(ox, oy + 1, col as i32, py + 1, '*');
        }
        prev = Some((col as i32, py));
    }
    // Ideal sawtooth ghost for the harmonic limit shape.
    let mut prev_g: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = (col as f64 / width.saturating_sub(1).max(1) as f64 + j + scroll).fract();
        let y = 2.0 * x - 1.0;
        let py = (cy - amp * y * 0.9 / 1.2).round() as i32;
        if let Some((ox, oy)) = prev_g {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev_g = Some((col as i32, py));
    }
}

/// Sawtooth Fourier room.
#[derive(Debug, Default)]
pub struct Sawtooth {
    seed: u64,
}

impl Sawtooth {
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

impl Room for Sawtooth {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sawtooth",
            title: "Sawtooth",
            wing: "Waves & Sound",
            blurb: "Fourier partials of a ramp: all harmonics. t and DRAG: SET HARMONICS.",
            accent: [180, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        // Ambient scrolls the wave; hand retunes harmonic count.
        draw(canvas, harmonics(0.45, None), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sawtooth",
            root: 415.3,
            tempo: 100,
            line: &[0, 2, 4, 7, 9, 12, 14, 12],
            encodes: "every integer harmonic with falling 1/k weights",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET HARMONICS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = harmonics(0.45, None);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("n={n}  scroll={p}%  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = harmonics(t, hands.last().copied());
        draw(canvas, n, phase_unit(t), self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = harmonics(t, hands.last().copied());
        // Harmonic energy of 1/k amplitudes is sum 1/k^2, total pi^2/6.
        let mut energy = 0.0_f64;
        for k in 1..=n {
            let kk = k as f64;
            energy += 1.0 / (kk * kk);
        }
        let total = std::f64::consts::PI * std::f64::consts::PI / 6.0;
        let pct = ((energy / total) * 100.0).round() as i32;
        Some(format!("n={n}  E={pct}%  Gibbs~9%"))
    }

    fn reveal(&self) -> &'static str {
        "A sawtooth needs every integer harmonic, amplitudes falling as 1/k. \
         That bright spectrum is why synth saws cut; partial sums climb the \
         ramp with Gibbs horns at the flyback."
    }
}

#[cfg(test)]
mod tests {
    use super::Sawtooth;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Sawtooth::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("saw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Sawtooth::new();
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
        Sawtooth::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
