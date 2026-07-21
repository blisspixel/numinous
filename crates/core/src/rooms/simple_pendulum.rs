//! Simple pendulum: small-angle and large-angle phase portraits.
//!
//! DRAG: TUNE ENERGY. See `docs/ROOMS.md`.

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

fn energy(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        0.2 + x * 2.5 + s
    } else {
        0.4 + phase_unit(t) * 2.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, e: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let e = e.clamp(0.15, 3.0);
    // Phase plane: theta horizontal, omega vertical. E = omega^2/2 - cos(theta)
    // Contour: omega = +/- sqrt(2(E + cos theta))
    let mut prev_u: Option<(i32, i32)> = None;
    let mut prev_l: Option<(i32, i32)> = None;
    for col in 0..width {
        let th = -std::f64::consts::PI
            + 2.0 * std::f64::consts::PI * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let inside = 2.0 * (e + th.cos());
        if inside < 0.0 {
            prev_u = None;
            prev_l = None;
            continue;
        }
        let om = inside.sqrt();
        let yu = ((0.5 - om / 4.0) * height.saturating_sub(1) as f64).round() as i32;
        let yl = ((0.5 + om / 4.0) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev_u {
            canvas.line(ox, oy, col as i32, yu, '#');
        }
        if let Some((ox, oy)) = prev_l {
            canvas.line(ox, oy, col as i32, yl, '#');
        }
        prev_u = Some((col as i32, yu));
        prev_l = Some((col as i32, yl));
    }
    // Separatrix E=1
    let mut prev_s: Option<(i32, i32)> = None;
    for col in 0..width {
        let th = -std::f64::consts::PI
            + 2.0 * std::f64::consts::PI * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let inside = 2.0 * (1.0 + th.cos());
        if inside < 0.0 {
            prev_s = None;
            continue;
        }
        let om = inside.sqrt();
        let y = ((0.5 - om / 4.0) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev_s {
            canvas.line(ox, oy, col as i32, y, '.');
        }
        prev_s = Some((col as i32, y));
    }
    // Bob snapshot
    let th0 = if e < 1.0 {
        (e.clamp(-0.99, 0.99)).asin() * 0.0 + (1.0 - e * 0.3).acos().min(std::f64::consts::PI)
    } else {
        std::f64::consts::PI * 0.6
    };
    let cx = (width / 2) as i32;
    let cy = (height as f64 * 0.25).round() as i32;
    let len = (height as f64 * 0.35) as i32;
    let bx = cx + (len as f64 * th0.sin() * 0.4).round() as i32;
    let by = cy + (len as f64 * th0.cos().abs().max(0.2)).round() as i32;
    canvas.line(cx, cy, bx, by, '+');
    canvas.line(bx - 1, by, bx + 1, by, 'o');
    let _ = seed;
}

/// Simple pendulum room.
#[derive(Debug, Default)]
pub struct SimplePendulum {
    seed: u64,
}

impl SimplePendulum {
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

impl Room for SimplePendulum {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "simple-pendulum",
            title: "Simple Pendulum",
            wing: "Motion & Dynamics",
            blurb: "Phase portrait: librations and rotations. t and DRAG: TUNE ENERGY.",
            accent: [40, 80, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, energy(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "simple-pendulum",
            root: 9.18,
            tempo: 76,
            line: &[0, 5, 3, 7, 12, 7, 3, 5],
            encodes: "pendulum phase: closed librations below separatrix E=1",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ENERGY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = energy(t, None, self.seed);
        let kind = if e < 1.0 { "lib" } else { "rot" };
        Some(format!("E={e:.2}  {kind}  DRAG:E"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = energy(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let e = energy(t, hands.last().copied(), self.seed);
        let kind = if e < 1.0 {
            "libration"
        } else if (e - 1.0).abs() < 0.05 {
            "separatrix"
        } else {
            "rotation"
        };
        Some(format!("E={e:.3}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "The simple pendulum lives on a phase cylinder: angle and angular speed. \
         Energies below the upright saddle give closed swings (librations); above \
         it the bob loops forever. The figure-eight separatrix divides them."
    }
}

#[cfg(test)]
mod tests {
    use super::SimplePendulum;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SimplePendulum::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lib") || s.contains("rot"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn energy_changes() {
        let r = SimplePendulum::new();
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
        SimplePendulum::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
