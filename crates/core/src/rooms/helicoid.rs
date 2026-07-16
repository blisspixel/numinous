//! Helicoid: ruled minimal surface, a continuous screw.
//!
//! DRAG: TUNE PITCH. See `docs/ROOMS.md`.

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

fn pitch(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.6 + s
    } else {
        0.6 + phase_unit(t) * 1.2 + s
    }
}

fn draw(canvas: &mut dyn Surface, c: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let c = c.clamp(0.3, 2.2);
    let scale = (width.min(height) as f64) * 0.22;
    let turns = 2.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.2
        };
    // Helicoid: x = u cos v, y = u sin v, z = c v. Project (x, z) with y tilt.
    let v_steps = 160;
    let u_rays = 7;
    for ri in 0..u_rays {
        let u = (ri as f64 / (u_rays - 1).max(1) as f64) * 1.4 - 0.7;
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=v_steps {
            let v = (i as f64 / v_steps as f64 - 0.5) * turns * 2.0 * std::f64::consts::PI;
            let x = u * v.cos();
            let y = u * v.sin();
            let z = c * v / std::f64::consts::PI;
            // Simple oblique projection.
            let px = (cx + (x + 0.35 * y) * scale).round() as i32;
            let py = (cy - z * scale * 0.55).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if ri == u_rays / 2 { '#' } else { '.' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    // A few ruling segments at fixed v.
    for k in 0..8 {
        let v = (k as f64 / 7.0 - 0.5) * turns * 2.0 * std::f64::consts::PI;
        let z = c * v / std::f64::consts::PI;
        let x0 = -0.7 * v.cos();
        let y0 = -0.7 * v.sin();
        let x1 = 0.7 * v.cos();
        let y1 = 0.7 * v.sin();
        let p0x = (cx + (x0 + 0.35 * y0) * scale).round() as i32;
        let p0y = (cy - z * scale * 0.55).round() as i32;
        let p1x = (cx + (x1 + 0.35 * y1) * scale).round() as i32;
        let p1y = (cy - z * scale * 0.55).round() as i32;
        canvas.line(p0x, p0y, p1x, p1y, '*');
    }
}

/// Helicoid room.
#[derive(Debug, Default)]
pub struct Helicoid {
    seed: u64,
}

impl Helicoid {
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

impl Room for Helicoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "helicoid",
            title: "Helicoid",
            wing: "Shape & Space",
            blurb: "Ruled minimal screw surface. t and DRAG: TUNE PITCH.",
            accent: [80, 140, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, pitch(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "helicoid",
            root: 329.63,
            tempo: 94,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "helicoid is ruled and minimal; isometric cousin of the catenoid",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PITCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let c = pitch(t, None, self.seed);
        Some(format!("c={c:.2}  screw  DRAG:PITCH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let c = pitch(t, hands.last().copied(), self.seed);
        draw(canvas, c, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let c = pitch(t, hands.last().copied(), self.seed);
        Some(format!("C={c:.3}  helicoid"))
    }

    fn reveal(&self) -> &'static str {
        "The helicoid is a continuous spiral ramp that is also a minimal surface: \
         mean curvature zero everywhere. It is isometric to the catenoid, and a \
         soap film can morph between the two without tearing."
    }
}

#[cfg(test)]
mod tests {
    use super::Helicoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Helicoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("screw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn pitch_changes() {
        let r = Helicoid::new();
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
        Helicoid::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
