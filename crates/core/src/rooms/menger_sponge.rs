//! Menger sponge face projection: 3D cross-section style carpet with depth cue.
//!
//! Distinct from menger-carpet and menger-slice. DRAG: SET THE DEPTH.
//! See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 4.0) as usize).clamp(1, 5)
    } else {
        (2 + (phase_unit(t) * 2.0) as usize).clamp(1, 4)
    }
}

/// True if (u,v,w) in [0,1]^3 remains in the Menger sponge at depth d.
fn in_sponge(u: f64, v: f64, w: f64, d: usize) -> bool {
    let mut u = u.clamp(0.0, 0.999_999);
    let mut v = v.clamp(0.0, 0.999_999);
    let mut w = w.clamp(0.0, 0.999_999);
    for _ in 0..d {
        let cx = (u * 3.0).floor() as i32;
        let cy = (v * 3.0).floor() as i32;
        let cz = (w * 3.0).floor() as i32;
        let mid_count = i32::from(cx == 1) + i32::from(cy == 1) + i32::from(cz == 1);
        // Remove if at least two coordinates are in the middle third
        if mid_count >= 2 {
            return false;
        }
        u = u * 3.0 - f64::from(cx);
        v = v * 3.0 - f64::from(cy);
        w = w * 3.0 - f64::from(cz);
    }
    true
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Orthographic projection of cube face with w from seed
    let w0 = if seed == 0 {
        0.35
    } else {
        0.2 + (seed % 7) as f64 * 0.08
    };
    for y in 0..height {
        for x in 0..width {
            let u = (x as f64 + 0.5) / width as f64;
            let v = (y as f64 + 0.5) / height as f64;
            // Two slices for depth cue
            let a = in_sponge(u, v, w0, d);
            let b = in_sponge(u, v, (w0 + 0.15).fract(), d);
            let ch = if a && b {
                '#'
            } else if a || b {
                '*'
            } else {
                ' '
            };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
}

/// Menger sponge projection room.
#[derive(Debug, Default)]
pub struct MengerSponge {
    seed: u64,
}

impl MengerSponge {
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

impl Room for MengerSponge {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "menger-sponge",
            title: "Menger Sponge",
            wing: "Fractals",
            blurb: "3D cross-removal fractal in twin slices. t and DRAG: SET THE DEPTH.",
            accent: [100, 100, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "menger sponge",
            root: 87.31,
            tempo: 66,
            line: &[0, 0, 5, 7, 12, 7, 5, 0],
            encodes: "cube tunnels punched until volume is zero",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  sponge  DRAG:D"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = depth(t, hands.last().copied());
        // volume fraction (20/27)^d
        let vol = (20.0_f64 / 27.0).powi(d as i32);
        Some(format!("DEPTH={d}  vol~{vol:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Menger sponge removes the open center cross of each face cube, \
         forever. Volume goes to zero; surface area goes to infinity. Hausdorff \
         dimension is log(20)/log(3)."
    }
}

#[cfg(test)]
mod tests {
    use super::MengerSponge;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MengerSponge::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH") || s.contains("depth"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = MengerSponge::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        MengerSponge::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
