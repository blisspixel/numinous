//! Solid torus sections: meridional and longitudinal cuts.
//!
//! DRAG: TUNE PHI. See `docs/ROOMS.md`.

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

fn phi(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.2
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::TAU + s
    } else {
        phase_unit(t) * std::f64::consts::TAU + s
    }
}

fn draw(canvas: &mut dyn Surface, phi: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r0 = (width.min(height) as f64) * 0.28;
    let r1 = r0 * 0.42;
    let tilt = if seed == 0 {
        0.35
    } else {
        0.25 + (seed % 4) as f64 * 0.05
    };
    // outer torus silhouette (two circles perspective)
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=120 {
        let th = std::f64::consts::TAU * (i as f64) / 120.0;
        let x = (r0 + r1) * th.cos();
        let y = (r0 + r1) * th.sin() * tilt;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    prev = None;
    for i in 0..=120 {
        let th = std::f64::consts::TAU * (i as f64) / 120.0;
        let x = (r0 - r1) * th.cos();
        let y = (r0 - r1) * th.sin() * tilt;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // meridional disk at angle phi
    let mx = cx + r0 * phi.cos();
    let my = cy - r0 * phi.sin() * tilt;
    for i in 0..=40 {
        let a = std::f64::consts::TAU * (i as f64) / 40.0;
        let px = (mx + r1 * a.cos()).round() as i32;
        let py = (my - r1 * a.sin() * 0.9).round() as i32;
        if i == 0 {
            prev = Some((px, py));
        } else if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            prev = Some((px, py));
        }
    }
    // core circle
    prev = None;
    for i in 0..=80 {
        let th = std::f64::consts::TAU * (i as f64) / 80.0;
        let x = r0 * th.cos();
        let y = r0 * th.sin() * tilt;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '=');
        }
        prev = Some((px, py));
    }
}

/// Solid torus room.
#[derive(Debug, Default)]
pub struct SolidTorus {
    seed: u64,
}

impl SolidTorus {
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

impl Room for SolidTorus {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "solid-torus",
            title: "Solid Torus",
            wing: "Shape & Space",
            blurb: "Meridian disk spinning inside a doughnut. t and DRAG: TUNE PHI.",
            accent: [70, 100, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phi(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "solid-torus",
            root: 146.83,
            tempo: 70,
            line: &[0, 2, 7, 9, 7, 2, 0, 5],
            encodes: "solid torus: D2 x S1, meridian and longitude generate pi1",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHI")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phi(t, None, self.seed);
        Some(format!("phi={p:.2}  torus  DRAG:PHI"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = phi(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = phi(t, hands.last().copied(), self.seed);
        Some(format!("PHI={p:.3}  solid"))
    }

    fn reveal(&self) -> &'static str {
        "A solid torus is a disk times a circle: D2 x S1. Cutting along a meridian \
         yields a solid cylinder; gluing two solid tori along their boundaries \
         builds every closed orientable 3-manifold of genus one (a lens space)."
    }
}

#[cfg(test)]
mod tests {
    use super::SolidTorus;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SolidTorus::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("torus"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phi_changes() {
        let r = SolidTorus::new();
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
        SolidTorus::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}
