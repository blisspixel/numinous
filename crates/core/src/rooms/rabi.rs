//! Rabi oscillation: two-level quantum flopping under a drive.
//!
//! DRAG: TUNE DETUNE. See `docs/ROOMS.md`.

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

fn detune(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        (x - 0.5) * 4.0 + s
    } else {
        (phase_unit(t) - 0.5) * 3.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, delta: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let omega0 = 2.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.2
        };
    let omega_r = (omega0 * omega0 + delta * delta).sqrt();
    // Excited-state probability P_e(t) = (Omega0/Omega_r)^2 sin^2(Omega_r t / 2)
    let amp = (omega0 / omega_r.max(1e-6)).powi(2);
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let u = col as f64 / width.saturating_sub(1).max(1) as f64;
        let tt = u * 4.0 * std::f64::consts::PI;
        let pe = amp * (0.5 * omega_r * tt).sin().powi(2);
        let py = ((1.0 - pe) * height.saturating_sub(1) as f64 * 0.85 + height as f64 * 0.08)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Ground line and excited line.
    let y_g = (height as f64 * 0.93).round() as i32;
    let y_e = (height as f64 * 0.08).round() as i32;
    canvas.line(0, y_g, width.saturating_sub(1) as i32, y_g, '.');
    canvas.line(0, y_e, width.saturating_sub(1) as i32, y_e, '.');
}

/// Rabi oscillation room.
#[derive(Debug, Default)]
pub struct Rabi {
    seed: u64,
}

impl Rabi {
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

impl Room for Rabi {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "rabi",
            title: "Rabi Flopping",
            wing: "Waves & Sound",
            blurb: "Two-level drive: detune slows full flips. t and DRAG: TUNE DETUNE.",
            accent: [80, 40, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, detune(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rabi",
            root: 587.33,
            tempo: 100,
            line: &[0, 7, 5, 12, 0, 7, 12, 5],
            encodes: "driven two-level system oscillates at generalized Rabi frequency",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DETUNE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = detune(t, None, self.seed);
        Some(format!("d={d:.2}  flip  DRAG:DET"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = detune(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = detune(t, hands.last().copied(), self.seed);
        let om0 = 2.5_f64;
        let om = (om0 * om0 + d * d).sqrt();
        // Max excited population (Om0/Om)^2; Rabi period 2 pi / Om.
        let pop = (om0 / om).powi(2);
        Some(format!("D={d:.2}  Om={om:.2}  Pmax={pop:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "A driven two-level quantum system does not sit still: population flops \
         between ground and excited states at the Rabi frequency. Detuning raises \
         that rate and caps how much population ever reaches the top."
    }
}

#[cfg(test)]
mod tests {
    use super::Rabi;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Rabi::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("flip"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn detune_changes() {
        let r = Rabi::new();
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
        Rabi::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
