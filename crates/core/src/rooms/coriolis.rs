//! Coriolis deflection: free path curves in a rotating frame.
//!
//! DRAG: TUNE SPIN. See `docs/ROOMS.md`.

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

fn spin(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 2.0 + s
    } else {
        0.6 + phase_unit(t) * 1.6 + s
    }
}

fn draw(canvas: &mut dyn Surface, omega: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64) * 0.44;
    let omega = omega.clamp(0.25, 3.0);
    // Inertial straight shot from left, viewed in a frame rotating at omega.
    let v = 1.2;
    let x0 = -1.0;
    let steps = 320;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let tt = u * 1.7;
        let xi = x0 + v * tt;
        let yi = 0.05
            * if seed == 0 {
                0.0
            } else {
                ((seed % 5) as f64 - 2.0) * 0.02
            };
        let ang = -omega * tt;
        let xr = xi * ang.cos() - yi * ang.sin();
        let yr = xi * ang.sin() + yi * ang.cos();
        let px = (cx + xr * scale).round() as i32;
        let py = (cy - yr * scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Inertial reference chord.
    let rx0 = (cx - scale).round() as i32;
    let rx1 = (cx + scale).round() as i32;
    canvas.line(rx0, cy.round() as i32, rx1, cy.round() as i32, '.');
    // Frame spin indicator (filled hub, not a lone reticle).
    let tip_ang = omega * 0.45;
    let tx = (cx + 0.18 * scale * tip_ang.cos()).round() as i32;
    let ty = (cy - 0.18 * scale * tip_ang.sin()).round() as i32;
    canvas.line(cx.round() as i32, cy.round() as i32, tx, ty, '+');
    for dy in -1..=1 {
        for dx in -1..=1 {
            canvas.plot(cx.round() as i32 + dx, cy.round() as i32 + dy, 'o');
        }
    }
}

/// Coriolis room.
#[derive(Debug, Default)]
pub struct Coriolis {
    seed: u64,
}

impl Coriolis {
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

impl Room for Coriolis {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coriolis",
            title: "Coriolis Path",
            wing: "Motion & Dynamics",
            blurb: "Inertial straight line curves under frame spin. t and DRAG: TUNE SPIN.",
            accent: [30, 130, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, spin(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "coriolis",
            root: 440.0,
            tempo: 90,
            line: &[0, 2, 7, 9, 12, 9, 7, 2],
            encodes: "rotating frame turns free straight flight into a curve",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SPIN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let w = spin(t, None, self.seed);
        Some(format!("w={w:.2}  deflect  DRAG:SPIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let w = spin(t, hands.last().copied(), self.seed);
        draw(canvas, w, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let w = spin(t, hands.last().copied(), self.seed).clamp(0.1, 3.0);
        // Path time ~1.7; total rotation of the frame is omega * t.
        let t_path = 1.7;
        let rot = w * t_path;
        let deg = rot.to_degrees();
        Some(format!("w={w:.2}  rot={rot:.2}  {deg:.0}deg"))
    }

    fn reveal(&self) -> &'static str {
        "In a rotating frame, free motion that is straight inertially appears to \
         curve. The fictitious Coriolis force is -2 m omega x v. Storms, \
         artillery, and long-range shells all feel this geometric bias."
    }
}

#[cfg(test)]
mod tests {
    use super::Coriolis;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Coriolis::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("deflect"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn spin_changes() {
        let r = Coriolis::new();
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
        Coriolis::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
