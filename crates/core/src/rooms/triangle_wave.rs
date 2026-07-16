//! Triangle wave Fourier partials: odd harmonics with 1/k^2 amplitudes.
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
        (1 + (x * 16.0) as usize).clamp(1, 18)
    } else {
        (2 + (phase_unit(t) * 12.0) as usize).clamp(1, 16)
    }
}

fn tri_partial(x: f64, n: usize) -> f64 {
    // (8/pi^2) sum_{m=0}^{n-1} (-1)^m sin((2m+1) pi x) / (2m+1)^2
    let mut s = 0.0;
    for m in 0..n {
        let k = 2 * m + 1;
        let sign = if m % 2 == 0 { 1.0 } else { -1.0 };
        s += sign * (k as f64 * std::f64::consts::PI * x).sin() / (k * k) as f64;
    }
    8.0 * s / (std::f64::consts::PI * std::f64::consts::PI)
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
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
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64 + j;
        let y = tri_partial(x.fract(), n).clamp(-1.2, 1.2);
        let py = (cy - amp * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
}

/// Triangle wave room.
#[derive(Debug, Default)]
pub struct TriangleWave {
    seed: u64,
}

impl TriangleWave {
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

impl Room for TriangleWave {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "triangle-wave",
            title: "Triangle Wave",
            wing: "Waves & Sound",
            blurb: "Odd harmonics with 1/k squared: soft corners. t and DRAG: SET HARMONICS.",
            accent: [60, 140, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, harmonics(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "triangle wave",
            root: 440.0,
            tempo: 84,
            line: &[0, 3, 7, 12, 7, 3, 0, 5],
            encodes: "odd harmonics that fall fast enough for continuous slope",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET HARMONICS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = harmonics(t, None);
        Some(format!("n={n}  tri  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = harmonics(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
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
        let n = harmonics(t, hands.last().copied());
        Some(format!("N={n}  1/k^2 odds"))
    }

    fn reveal(&self) -> &'static str {
        "A triangle wave is continuous with discontinuous slope. Its Fourier \
         series uses only odd harmonics with amplitudes 1/k^2: much softer than \
         a square, still not a pure sine."
    }
}

#[cfg(test)]
mod tests {
    use super::TriangleWave;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TriangleWave::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("tri"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = TriangleWave::new();
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
        TriangleWave::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
