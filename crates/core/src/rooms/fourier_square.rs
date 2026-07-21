//! Fourier square wave: partial sums and the Gibbs overshoot.
//!
//! Sum of odd harmonics toward a square wave. DRAG: SET THE TERM COUNT.
//! See `docs/ROOMS.md`.

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

fn terms(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 40.0) as usize).clamp(1, 48)
    } else {
        (2 + (phase_unit(t) * 30.0) as usize).clamp(1, 36)
    }
}

fn partial(x: f64, n: usize) -> f64 {
    // (4/pi) sum_{k=0}^{n-1} sin((2k+1)x)/(2k+1)
    let mut s = 0.0f64;
    for k in 0..n {
        let odd = (2 * k + 1) as f64;
        s += (odd * x).sin() / odd;
    }
    4.0 * s / std::f64::consts::PI
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.05
    };
    // Target square wave (dashed).
    let mid = height as f64 * 0.5;
    let amp = height as f64 * 0.35;
    for col in 0..width {
        let u = col as f64 / width.saturating_sub(1).max(1) as f64;
        let x = (u * 2.0 * std::f64::consts::PI) + shift;
        // Square: +1 on [0,pi), -1 on [pi,2pi)
        let phase = x.rem_euclid(std::f64::consts::TAU);
        let sq = if phase < std::f64::consts::PI {
            1.0
        } else {
            -1.0
        };
        let py = (mid - sq * amp * 0.9).round() as i32;
        canvas.plot(col as i32, py, '.');
    }
    // Partial sum.
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..=width.saturating_mul(2) {
        let u = col as f64 / (width.saturating_mul(2).max(1) as f64);
        let x = (u * 2.0 * std::f64::consts::PI) + shift;
        let y = partial(x, n).clamp(-1.5, 1.5);
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = (mid - y * amp).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if n < 4 { '*' } else { '#' });
        }
        prev = Some((px, py));
    }
}

/// Fourier square wave room.
#[derive(Debug, Default)]
pub struct FourierSquare {
    seed: u64,
}

impl FourierSquare {
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

impl Room for FourierSquare {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fourier-square",
            title: "Gibbs Overshoot",
            wing: "Waves & Sound",
            blurb: "Odd-harmonic Fourier sums toward a square wave; ringing refuses to die. t and \
                    DRAG: SET THE TERM COUNT.",
            accent: [40, 100, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, terms(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gibbs",
            root: 196.0,
            tempo: 120,
            line: &[0, 12, 0, 12, 7, 12, 0, 7],
            encodes: "odd harmonics climbing a square with ringing",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE TERM COUNT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = terms(t, None);
        Some(format!("terms={n}  Gibbs  DRAG:TERMS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = terms(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = terms(t, hands.last().copied());
        // Peak near discontinuity estimates Gibbs ~9% overshoot independent of n.
        let peak = (0..200)
            .map(|i| {
                let x = std::f64::consts::PI * (0.5 + i as f64 * 0.002);
                partial(x, n).abs()
            })
            .fold(0.0f64, f64::max);
        Some(format!("TERMS={n}  peak~{peak:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Partial sums of the square-wave Fourier series ring near jumps. The \
         overshoot approaches about 9% of the jump height and does not vanish as \
         more terms are added: Gibbs phenomenon."
    }
}

#[cfg(test)]
mod tests {
    use super::FourierSquare;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FourierSquare::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("terms"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn terms_change() {
        let r = FourierSquare::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        FourierSquare::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(FourierSquare::new().motif().unwrap().line.len() >= 6);
    }
}
