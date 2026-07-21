//! Mexican hat wavelet: second derivative of a Gaussian, Ricker wavelet.
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        0.3 + x * 1.5 + s
    } else {
        0.45 + phase_unit(t) * 1.2 + s
    }
}

fn ricker(x: f64, sigma: f64) -> f64 {
    let u = x / sigma;
    let u2 = u * u;
    (1.0 - u2) * (-0.5 * u2).exp()
}

fn draw(canvas: &mut dyn Surface, sig: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let sig = sig.clamp(0.25, 2.0);
    let cy = height as f64 * 0.55;
    let y_scale = height as f64
        * 0.35
        * (1.0
            + if seed == 0 {
                0.0
            } else {
                (seed % 3) as f64 * 0.04
            });
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -4.0 + 8.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = ricker(x, sig);
        let py = (cy - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
    // Zero crossings at x = +/- sigma.
    for s in [-1.0, 1.0] {
        let x = s * sig;
        let px = (((x + 4.0) / 8.0) * width.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, cy as i32 - 2, px, cy as i32 + 2, '|');
    }
}

/// Mexican hat room.
#[derive(Debug, Default)]
pub struct MexicanHat {
    seed: u64,
}

impl MexicanHat {
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

impl Room for MexicanHat {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mexican-hat",
            title: "Mexican Hat",
            wing: "Waves & Sound",
            blurb: "Ricker wavelet: second Gaussian derivative. t and DRAG: TUNE SCALE.",
            accent: [160, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "mexican-hat",
            root: 110.0,
            tempo: 98,
            line: &[0, 7, 5, 12, 3, 7, 12, 0],
            encodes: "Mexican hat wavelet: (1-u^2) e^{-u^2/2} multi-scale probe",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = scale_p(t, None, self.seed);
        Some(format!("s={s:.2}  ricker  DRAG:SCALE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = scale_p(t, hands.last().copied(), self.seed);
        draw(canvas, s, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let s = scale_p(t, hands.last().copied(), self.seed).clamp(0.25, 2.0);
        // Ricker wavelet: zeros at +/- sigma, peak 1 at 0.
        let width = 2.0 * s;
        Some(format!("s={s:.2}  zeros=+-s  w={width:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Mexican hat (Ricker) wavelet is the second derivative of a Gaussian: \
         (1 - u^2) e^{-u^2/2}. It is zero-mean, multi-scale, and the workhorse of \
         continuous wavelet transforms and seismic pulse models."
    }
}

#[cfg(test)]
mod tests {
    use super::MexicanHat;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MexicanHat::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ricker"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = MexicanHat::new();
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
        MexicanHat::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
