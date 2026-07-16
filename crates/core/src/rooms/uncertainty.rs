//! The Uncertainty Dial: narrower in time, wider in frequency.
//!
//! A Gaussian window and its Fourier transform: squeeze one, the other spreads.
//! DRAG: SQUEEZE THE WINDOW. See `docs/ROOMS.md`.

use std::f64::consts::PI;

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

fn sigma_t(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((x, _)) = hand {
        0.05 + x * 0.45
    } else {
        0.08 + phase_unit(t) * 0.35
    }
}

fn gaussian(x: f64, sigma: f64) -> f64 {
    let s = sigma.max(1e-6);
    (-0.5 * (x / s).powi(2)).exp()
}

fn draw(canvas: &mut dyn Surface, sigma: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mid = height / 2;
    // Top half: time window.
    let mut prev: Option<(i32, i32)> = None;
    for px in 0..width {
        let x = (px as f64 / width.saturating_sub(1).max(1) as f64 - 0.5) * 4.0;
        let y = gaussian(x, sigma);
        let py = (mid as f64 * (1.0 - y * 0.9)).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px as i32, py, '#');
        }
        prev = Some((px as i32, py));
    }
    // Bottom half: frequency (Fourier of Gaussian is Gaussian with 1/sigma).
    let sigma_f = 1.0 / (sigma * 2.0 * PI).max(1e-6);
    // Rescale for display.
    let sigma_f_plot = sigma_f * 0.15;
    prev = None;
    for px in 0..width {
        let f = (px as f64 / width.saturating_sub(1).max(1) as f64 - 0.5) * 4.0;
        let y = gaussian(f, sigma_f_plot.max(0.05));
        let py = (mid as f64 + mid as f64 * (1.0 - y * 0.85)).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(
                ox,
                oy,
                px as i32,
                py.min(height.saturating_sub(1) as i32),
                '*',
            );
        }
        prev = Some((px as i32, py.min(height.saturating_sub(1) as i32)));
    }
    // Divider.
    canvas.line(
        0,
        mid as i32,
        width.saturating_sub(1) as i32,
        mid as i32,
        '.',
    );
}

/// Uncertainty Dial room.
#[derive(Debug, Default)]
pub struct Uncertainty {
    seed: u64,
}

impl Uncertainty {
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

impl Room for Uncertainty {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "uncertainty",
            title: "The Uncertainty Dial",
            wing: "Waves & Sound",
            blurb: "Narrower in time, wider in frequency: you cannot own both. t and DRAG: SQUEEZE \
                    THE WINDOW.",
            accent: [255, 200, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let s = sigma_t(t, None)
            * if self.seed == 0 {
                1.0
            } else {
                0.9 + (self.seed % 5) as f64 * 0.04
            };
        draw(canvas, s);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "time freq",
            root: 220.0,
            tempo: 90,
            line: &[0, 7, 12, 7, 0, 5, 12, 0],
            encodes: "squeeze time and frequency must spread",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SQUEEZE THE WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = sigma_t(t, None);
        let sf = 1.0 / (s * 2.0 * PI);
        Some(format!("st={s:.2}  sf={sf:.2}  DRAG:SQUEEZE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = sigma_t(t, hands.last().copied());
        draw(canvas, s);
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
        let s = sigma_t(t, hands.last().copied());
        let sf = 1.0 / (s * 2.0 * PI);
        let prod = s * sf;
        Some(format!("st={s:.2} sf={sf:.2}  prod={prod:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "A function and its Fourier transform cannot both be sharply localized. \
         The Gaussian is the equality case: squeeze the window in time and its \
         spectrum spreads, a physical trade you can hear as tone smear."
    }
}

#[cfg(test)]
mod tests {
    use super::Uncertainty;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Uncertainty::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SQUEEZE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn squeeze_changes() {
        let r = Uncertainty::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.1,
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
        Uncertainty::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Uncertainty::new().motif().unwrap().line.len() >= 6);
    }
}
