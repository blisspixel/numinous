//! Hopf Fibration: linked rings, none touching (the qubit's shadow).
//!
//! Stereographic projection of fibers of S^3 -> S^2 fills space with circles
//! that are all linked and pairwise non-intersecting. SPIN: THE FIBER.
//! See `docs/ROOMS.md`.

use std::f64::consts::TAU;

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

/// Stereographic Hopf fiber for base point (theta, phi) on S^2, parameter s on the fiber.
fn fiber_point(theta: f64, phi: f64, s: f64) -> (f64, f64, f64) {
    // Standard: (cos(theta/2) e^{i s}, sin(theta/2) e^{i (s+phi)}) in C^2, then stereo from S^3.
    let ct = (theta * 0.5).cos();
    let st = (theta * 0.5).sin();
    let z0r = ct * s.cos();
    let z0i = ct * s.sin();
    let z1r = st * (s + phi).cos();
    let z1i = st * (s + phi).sin();
    // Stereographic from north pole of S^3 in R^4: drop w=z0i or use (x,y,z)/(1-w).
    let w = z0i;
    let den = (1.0 - w).max(1e-6);
    let x = z0r / den;
    let y = z1r / den;
    let z = z1i / den;
    (x, y, z)
}

fn project(x: f64, y: f64, z: f64, yaw: f64, pitch: f64) -> (f64, f64) {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    let x1 = x * cy - z * sy;
    let z1 = x * sy + z * cy;
    let y1 = y * cp - z1 * sp;
    let z2 = y * sp + z1 * cp;
    let depth = 2.5 + z2;
    let s = 0.35 / depth.max(0.5);
    (0.5 + x1 * s, 0.5 - y1 * s)
}

fn draw(canvas: &mut dyn Surface, n_fibers: usize, yaw: f64, pitch: f64, phase: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |u: f64, v: f64| -> (i32, i32) {
        (
            (u.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (v.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let n_fibers = n_fibers.clamp(3, 14);
    for f in 0..n_fibers {
        let theta = 0.35 + 0.9 * f as f64 / n_fibers as f64;
        let phi = phase + TAU * f as f64 / n_fibers as f64;
        let steps = 48;
        let mut prev: Option<(i32, i32)> = None;
        for s in 0..=steps {
            let ang = TAU * s as f64 / steps as f64;
            let (x, y, z) = fiber_point(theta, phi, ang);
            let (u, v) = project(x, y, z, yaw, pitch);
            let p = to_px(u, v);
            if let Some(o) = prev {
                let ch = if f % 3 == 0 { '#' } else { '*' };
                canvas.line(o.0, o.1, p.0, p.1, ch);
            }
            prev = Some(p);
        }
    }
}

/// Hopf Fibration room.
#[derive(Debug, Default)]
pub struct Hopf {
    seed: u64,
}

impl Hopf {
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

impl Room for Hopf {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hopf",
            title: "The Linked Rings",
            wing: "Shape & Space",
            blurb: "Hopf fibration: space filled with circles all linked, none touching. The \
                    shadow of S^3 and a picture of a qubit. t grows fibers; DRAG: SPIN THE FIBER.",
            accent: [180, 120, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = 5 + (phase_unit(t) * 6.0) as usize;
        let yaw = phase_unit(t) * 1.2
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 5) as f64 * 0.1
            };
        let pitch = 0.4;
        draw(canvas, n, yaw, pitch, phase_unit(t) * TAU * 0.2);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hopf links",
            root: 349.23,
            tempo: 93,
            line: &[0, 7, 10, 14, 17, 14, 10, 7],
            encodes: "circles all linked, none touching: S^3 to S^2",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SPIN THE FIBER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = 5 + (phase_unit(t) * 6.0) as usize;
        Some(format!("fibers={n}  Hopf  DRAG:SPIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        let (yaw, pitch) = hands
            .last()
            .map(|&(x, y)| ((x - 0.5) * TAU, (y - 0.5) * 2.0))
            .unwrap_or((0.5, 0.4));
        draw(canvas, n, yaw, pitch, phase_unit(t));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        let (x, y) = *hands.last().unwrap();
        // Hopf map sample: hand selects a base point; n fibers drawn.
        Some(format!("fibers={n}  base=({x:.2},{y:.2})"))
    }

    fn reveal(&self) -> &'static str {
        "The Hopf fibration maps each great circle in S^3 to a point of S^2. \
         Stereographic projection of those fibers yields a space-filling family \
         of linked circles: the classical picture of a quantum two-state system."
    }
}

#[cfg(test)]
mod tests {
    use super::Hopf;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hopf::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("SPIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn spin_changes() {
        let r = Hopf::new();
        let o = r.status(0.4).unwrap();
        let a = r
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Hopf::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Hopf::new().motif().unwrap().line.len() >= 6);
    }
}
