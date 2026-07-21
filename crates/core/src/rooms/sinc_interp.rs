//! Cardinal sine (sinc) interpolation kernel and reconstruction.
//!
//! DRAG: TUNE B. See `docs/ROOMS.md`.

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

fn bandwidth(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        0.5 + x * 3.5 + s
    } else {
        0.8 + phase_unit(t) * 2.8 + s
    }
    .clamp(0.4, 4.5)
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-10 {
        1.0
    } else {
        let z = std::f64::consts::PI * x;
        z.sin() / z
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // samples of a low-frequency signal
    let n = 7usize;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let phase = if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.2
        };
        let y = (2.0 * std::f64::consts::PI * (i as f64) / (n as f64 - 1.0) * b * 0.35 + phase)
            .sin()
            * 0.7;
        samples.push(y);
    }
    // reconstructed curve via sinc sum
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = (col as f64) / width.saturating_sub(1).max(1) as f64 * (n as f64 - 1.0);
        let mut y = 0.0;
        for (i, &s) in samples.iter().enumerate() {
            y += s * sinc(x - i as f64);
        }
        let py = ((0.5 - y * 0.45) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // sample stems
    for (i, &s) in samples.iter().enumerate() {
        let px = ((i as f64) / (n as f64 - 1.0) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((0.5 - s * 0.45) * height.saturating_sub(1) as f64).round() as i32;
        let mid = (height / 2) as i32;
        canvas.line(px, mid, px, py, '|');
        canvas.line(px - 1, py, px + 1, py, 'o');
    }
    // one sinc kernel outline
    let kcx = width as f64 * 0.5;
    prev = None;
    for col in 0..width {
        let x = (col as f64 - kcx) / (width as f64 / (2.0 * b));
        let y = sinc(x) * 0.25;
        let py = ((0.85 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
}

/// Sinc interpolation room.
#[derive(Debug, Default)]
pub struct SincInterp {
    seed: u64,
}

impl SincInterp {
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

impl Room for SincInterp {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sinc-interp",
            title: "Sinc Interpolation",
            wing: "Analysis",
            blurb: "Whittaker-Shannon reconstruction from samples. t and DRAG: TUNE B.",
            accent: [50, 90, 110],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, bandwidth(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sinc-interp",
            root: 246.94,
            tempo: 76,
            line: &[0, 3, 5, 7, 10, 7, 5, 3],
            encodes: "sinc kernel: bandlimited interpolation Whittaker-Shannon",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = bandwidth(t, None, self.seed);
        Some(format!("b={b:.2}  sinc  DRAG:B"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = bandwidth(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = bandwidth(t, hands.last().copied(), self.seed);
        // Nyquist spacing ~ 1/(2B) for bandlimit B.
        let nyq = 1.0 / (2.0 * b.max(1e-6));
        Some(format!("B={b:.2}  Nyq={nyq:.2}  recon"))
    }

    fn reveal(&self) -> &'static str {
        "If a signal is bandlimited, samples at or above the Nyquist rate capture \
         it completely. The Whittaker-Shannon formula rebuilds the continuous wave \
         as a sum of shifted sinc kernels, one per sample."
    }
}

#[cfg(test)]
mod tests {
    use super::SincInterp;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SincInterp::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("sinc"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn b_changes() {
        let r = SincInterp::new();
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
        SincInterp::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
