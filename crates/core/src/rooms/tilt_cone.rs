//! Tilt the Cone: boost the frame; simultaneity trades places.
//!
//! A light-cone diagram in 1+1 dimensions. DRAG: BOOST THE FRAME applies a
//! Lorentz boost so planes of simultaneity tip while the light lines stay at
//! 45 degrees. Causality refuses to break. See `docs/ROOMS.md`.

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

fn beta_from(t: f64, pokes: &[(f64, f64)], seed: u64) -> f64 {
    let hands = finite_pokes(pokes);
    let b = if let Some(&(x, _)) = hands.last() {
        (x * 2.0 - 1.0) * 0.9
    } else {
        (phase_unit(t) * 2.0 - 1.0) * 0.75
            + if seed == 0 {
                0.0
            } else {
                ((seed % 5) as f64 - 2.0) * 0.05
            }
    };
    b.clamp(-0.95, 0.95)
}

fn draw(canvas: &mut dyn Surface, beta: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let s = width.min(height) as f64 * 0.4;
    // Light lines x=+-t stay at 45deg.
    canvas.line(
        (cx - s).round() as i32,
        (cy + s).round() as i32,
        (cx + s).round() as i32,
        (cy - s).round() as i32,
        '*',
    );
    canvas.line(
        (cx - s).round() as i32,
        (cy - s).round() as i32,
        (cx + s).round() as i32,
        (cy + s).round() as i32,
        '*',
    );
    // Boosted simultaneity: t' = 0 => t = beta x  (c=1), slope beta in (x,t) with t up.
    // Screen y grows down; draw t upward as -y.
    let gamma = 1.0 / (1.0 - beta * beta).max(1e-9).sqrt();
    let _ = gamma;
    // Horizontal in rest frame (beta=0); tip with beta.
    let dx = s;
    let dt = beta * s; // time tilt
    canvas.line(
        (cx - dx).round() as i32,
        (cy + dt).round() as i32,
        (cx + dx).round() as i32,
        (cy - dt).round() as i32,
        '#',
    );
    // Time axis for boosted observer: x' = 0 => x = beta t.
    canvas.line(
        (cx - beta * s).round() as i32,
        (cy + s).round() as i32,
        (cx + beta * s).round() as i32,
        (cy - s).round() as i32,
        ':',
    );
    canvas.plot(cx.round() as i32, cy.round() as i32, '+');
}

/// Tilt the Cone room.
#[derive(Debug, Default)]
pub struct TiltCone {
    seed: u64,
}

impl TiltCone {
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

impl Room for TiltCone {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tilt-cone",
            title: "Tilt the Cone",
            wing: "Shape & Space",
            blurb: "Boost the frame: planes of simultaneity tip, light stays at 45 degrees, \
                    causality holds. t and DRAG: BOOST THE FRAME set beta. Lorentz pair with Starbow.",
            accent: [100, 180, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, beta_from(t, &[], self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.8
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "light cone",
            root: 196.0,
            tempo: 100,
            line: &[0, 7, 12, 7, 0, 5, 12, 0],
            encodes: "simultaneity tipping while light lines refuse",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: BOOST THE FRAME")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = beta_from(t, &[], self.seed);
        let g = 1.0 / (1.0 - b * b).max(1e-9).sqrt();
        Some(format!("b={b:+.2}  g={g:.2}  DRAG:BOOST"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw(canvas, beta_from(t, pokes, self.seed));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        if finite_pokes(&pokes).is_empty() {
            return self.status(t);
        }
        let b = beta_from(t, &pokes, self.seed);
        let g = 1.0 / (1.0 - b * b).max(1e-9).sqrt();
        Some(format!("BOOST b={b:+.2}  g={g:.2}  LIGHT OK"))
    }

    fn reveal(&self) -> &'static str {
        "A Lorentz boost tips the plane of simultaneity but never the light \
         cone. Events that were simultaneous for one observer become ordered for \
         another, yet no signal can leave the cone: causality is geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::TiltCone;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TiltCone::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("BOOST"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn boost_changes() {
        let r = TiltCone::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
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
        TiltCone::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(TiltCone::new().motif().unwrap().line.len() >= 6);
    }
}
