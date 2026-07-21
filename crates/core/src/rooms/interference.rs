//! Two-source interference: Young's double-slit pattern on a line screen.
//!
//! DRAG: TUNE SPACING. See `docs/ROOMS.md`.

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

fn spacing(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.4 + s
    } else {
        0.08 + phase_unit(t) * 0.3 + s
    }
}

fn draw(canvas: &mut dyn Surface, d: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let wavelength = 0.08
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.005
        };
    let s1x = 0.5 - d * 0.5;
    let s2x = 0.5 + d * 0.5;
    let sy = 0.15;
    // sources
    let sx1 = (s1x * width.saturating_sub(1) as f64).round() as i32;
    let sx2 = (s2x * width.saturating_sub(1) as f64).round() as i32;
    let syi = (sy * height.saturating_sub(1) as f64).round() as i32;
    canvas.plot(sx1, syi, 'o');
    canvas.plot(sx2, syi, 'o');
    // field intensity on grid
    for y in 0..height {
        for x in 0..width {
            let px = x as f64 / width.saturating_sub(1).max(1) as f64;
            let py = y as f64 / height.saturating_sub(1).max(1) as f64;
            let r1 = ((px - s1x).hypot(py - sy)).max(1e-4);
            let r2 = ((px - s2x).hypot(py - sy)).max(1e-4);
            let phase = std::f64::consts::TAU * (r1 - r2) / wavelength;
            // intensity ~ cos^2(delta/2)
            let i = (0.5 * (1.0 + phase.cos())).clamp(0.0, 1.0);
            let ch = if i > 0.75 {
                '#'
            } else if i > 0.45 {
                '*'
            } else if i > 0.2 {
                '+'
            } else if i > 0.05 {
                '.'
            } else {
                ' '
            };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
    canvas.plot(sx1, syi, 'o');
    canvas.plot(sx2, syi, 'o');
}

/// Two-source interference room.
#[derive(Debug, Default)]
pub struct Interference {
    seed: u64,
}

impl Interference {
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

impl Room for Interference {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "interference",
            title: "Interference",
            wing: "Waves & Sound",
            blurb: "Two sources paint bright and dark fringes. t and DRAG: TUNE SPACING.",
            accent: [60, 80, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, spacing(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "interference",
            root: 587.33,
            tempo: 76,
            line: &[0, 5, 0, 7, 0, 12, 0, 7],
            encodes: "path difference decides bright and dark",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SPACING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = spacing(t, None, self.seed);
        Some(format!("d={d:.2}  fringe  DRAG:D"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = spacing(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = spacing(t, hands.last().copied(), self.seed);
        let wavelength = 0.08
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 4) as f64 * 0.005
            };
        // Far-field fringe scale ~ lambda / d (source separation).
        let fringe = wavelength / d.max(1e-6);
        Some(format!("d={d:.2}  lam={wavelength:.3}  fr~{fringe:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Two coherent sources produce interference: bright where path difference \
         is a whole number of wavelengths, dark where it is half. Young's \
         double slit made the wave nature of light undeniable."
    }
}

#[cfg(test)]
mod tests {
    use super::Interference;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Interference::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("fringe"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn spacing_changes() {
        let r = Interference::new();
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
        Interference::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
