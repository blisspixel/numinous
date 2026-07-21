//! Frequency modulation: cos(omega_c t + beta sin omega_m t).
//!
//! DRAG: TUNE BETA. See `docs/ROOMS.md`.

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

fn beta(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * 8.0 + s
    } else {
        0.5 + phase_unit(t) * 6.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.4;
    let fc = 14.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.5
        };
    let fm = 1.5;
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let phase = std::f64::consts::TAU * fc * x + b * (std::f64::consts::TAU * fm * x).sin();
        let y = amp * phase.cos();
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
}

/// FM modulation room.
#[derive(Debug, Default)]
pub struct FmModulation {
    seed: u64,
}

impl FmModulation {
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

impl Room for FmModulation {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fm-modulation",
            title: "FM Modulation",
            wing: "Waves & Sound",
            blurb: "Instantaneous frequency wiggles: radio FM. t and DRAG: TUNE BETA.",
            accent: [140, 60, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, beta(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "fm modulation",
            root: 493.88,
            tempo: 106,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "Bessel sidebands grow with modulation index beta",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE BETA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = beta(t, None, self.seed);
        Some(format!("b={b:.1}  FM  DRAG:BETA"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = beta(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = beta(t, hands.last().copied(), self.seed);
        // Carson rule rough bandwidth ~ 2(beta+1)fm
        let bw = 2.0 * (b + 1.0) * 1.5;
        Some(format!("B={b:.2}  BW~{bw:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "Frequency modulation writes the message into instantaneous frequency. \
         Modulation index beta spreads energy into Bessel sidebands; Carson's \
         rule estimates the occupied bandwidth."
    }
}

#[cfg(test)]
mod tests {
    use super::FmModulation;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FmModulation::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("FM") || s.contains("BETA"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn beta_changes() {
        let r = FmModulation::new();
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
        FmModulation::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
