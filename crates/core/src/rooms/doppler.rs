//! Doppler shift sketch: source moving along a line, wavefronts compress.
//!
//! DRAG: TUNE SPEED. See `docs/ROOMS.md`.

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

fn speed(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    // fraction of wave speed c
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        (x * 0.95 + s).clamp(0.0, 0.98)
    } else {
        (0.1 + phase_unit(t) * 0.75 + s).clamp(0.0, 0.98)
    }
}

fn draw(canvas: &mut dyn Surface, beta: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let c = 1.0; // wave speed in units
    let v = beta * c;
    let pulses = 8;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // source path
    canvas.line(
        0,
        cy.round() as i32,
        width.saturating_sub(1) as i32,
        cy.round() as i32,
        '.',
    );
    for p in 0..pulses {
        let emit_t = p as f64 / pulses as f64;
        let sx = (0.1 + emit_t * 0.7) * width as f64;
        // radius grown since emission: proportional to (1 - emit_t)
        let age = 1.0 - emit_t + j * 0.05;
        // Doppler: front radius smaller ahead, larger behind via center shift
        // Use expanding circle centered at emission point, but draw denser ahead
        let r = age * (width as f64) * 0.35 * (1.0 - 0.3 * beta);
        let steps = 48;
        for i in 0..steps {
            let th = std::f64::consts::TAU * (i as f64 / steps as f64);
            // compress ahead (direction of motion +x)
            let compress = if th.cos() > 0.0 {
                1.0 - beta * 0.7
            } else {
                1.0 + beta * 0.5
            };
            let px = (sx + r * compress * th.cos()).round() as i32;
            let py = (cy + r * 0.55 * th.sin()).round() as i32;
            canvas.plot(px, py, if p % 2 == 0 { '#' } else { '*' });
        }
        canvas.plot(sx.round() as i32, cy.round() as i32, 'o');
        let _ = v;
    }
}

/// Doppler room.
#[derive(Debug, Default)]
pub struct Doppler {
    seed: u64,
}

impl Doppler {
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

impl Room for Doppler {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "doppler",
            title: "Doppler",
            wing: "Waves & Sound",
            blurb: "Moving source packs wavefronts ahead. t and DRAG: TUNE SPEED.",
            accent: [200, 100, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, speed(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "doppler",
            root: 554.37,
            tempo: 110,
            line: &[0, 3, 7, 12, 15, 12, 7, 3],
            encodes: "frequency rise ahead of a moving source",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SPEED")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = speed(t, None, self.seed);
        Some(format!("v/c={b:.2}  doppler  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = speed(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
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
        let b = speed(t, hands.last().copied(), self.seed);
        let ratio = (1.0 + b) / (1.0 - b).max(1e-3);
        Some(format!("v/c={b:.3}  f'/f~{ratio:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Doppler effect is the change in observed frequency when source and \
         listener move. Ahead of a source, wavefronts pack tighter: pitch rises. \
         Behind, they stretch: pitch falls."
    }
}

#[cfg(test)]
mod tests {
    use super::Doppler;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Doppler::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("doppler"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn speed_changes() {
        let r = Doppler::new();
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
        Doppler::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
