//! Coupled oscillators: normal modes of two masses on three springs.
//!
//! DRAG: TUNE K. See `docs/ROOMS.md`.

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

fn couple(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        0.2 + x * 2.5 + s
    } else {
        0.4 + phase_unit(t) * 2.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k_c: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let k: f64 = 1.0;
    let kc = k_c.clamp(0.1, 3.0);
    // Normal modes: omega_s^2 = k/m, omega_a^2 = (k+2kc)/m with m=1
    let w_s = k.sqrt();
    let w_a = (k + 2.0 * kc).sqrt();
    let t_end = 4.0 * std::f64::consts::PI / w_s.min(w_a);
    let amp = height as f64 * 0.18;
    let y1 = height as f64 * 0.35;
    let y2 = height as f64 * 0.7;
    let phase = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.2
    };
    // Symmetric mode both move together
    let mut prev1: Option<(i32, i32)> = None;
    let mut prev2: Option<(i32, i32)> = None;
    for col in 0..width {
        let t = t_end * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let xs = (w_s * t + phase).sin();
        let xa = (w_a * t).sin();
        // show mixture: mostly symmetric with a bit of anti for visual
        let x1 = 0.7 * xs + 0.3 * xa;
        let x2 = 0.7 * xs - 0.3 * xa;
        let p1y = (y1 - x1 * amp).round() as i32;
        let p2y = (y2 - x2 * amp).round() as i32;
        if let Some((ox, oy)) = prev1 {
            canvas.line(ox, oy, col as i32, p1y, '#');
        }
        if let Some((ox, oy)) = prev2 {
            canvas.line(ox, oy, col as i32, p2y, '*');
        }
        prev1 = Some((col as i32, p1y));
        prev2 = Some((col as i32, p2y));
    }
    // mode frequency readout ticks
    canvas.line(2, y1 as i32, 2, y1 as i32, '1');
    canvas.line(2, y2 as i32, 2, y2 as i32, '2');
}

/// Coupled oscillators room.
#[derive(Debug, Default)]
pub struct CoupledOsc {
    seed: u64,
}

impl CoupledOsc {
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

impl Room for CoupledOsc {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coupled-osc",
            title: "Coupled Oscillators",
            wing: "Motion & Dynamics",
            blurb: "Two masses, three springs: normal modes. t and DRAG: TUNE K.",
            accent: [80, 100, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, couple(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "coupled-osc",
            root: 7.29,
            tempo: 92,
            line: &[0, 4, 7, 4, 0, 7, 12, 7],
            encodes: "coupled masses: symmetric and antisymmetric normal modes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let kc = couple(t, None, self.seed);
        let wa = (1.0 + 2.0 * kc).sqrt();
        Some(format!("kc={kc:.2}  wa={wa:.2}  DRAG:K"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let kc = couple(t, hands.last().copied(), self.seed);
        draw(canvas, kc, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let kc = couple(t, hands.last().copied(), self.seed);
        // Two equal masses, spring kc: normal modes sqrt(1) and sqrt(1+2kc).
        let w_s = 1.0_f64;
        let w_a = (1.0 + 2.0 * kc).sqrt();
        let split = w_a - w_s;
        Some(format!("kc={kc:.2}  wa={w_a:.2}  dw={split:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Two equal masses linked by springs have two normal modes: both move \
         together (lower frequency) or opposite (higher, stiffened by the middle \
         spring). General motion is a beat between those pure modes."
    }
}

#[cfg(test)]
mod tests {
    use super::CoupledOsc;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CoupledOsc::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("wa="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = CoupledOsc::new();
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
        CoupledOsc::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
