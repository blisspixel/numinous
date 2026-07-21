//! Sphere geodesics: great-circle arcs are locally straight paths.
//!
//! DRAG: TUNE TILT. See `docs/ROOMS.md`.

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

fn tilt(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    let tilt = if let Some((x, _)) = hand {
        x * std::f64::consts::FRAC_PI_2 + s
    } else {
        phase_unit(t) * std::f64::consts::FRAC_PI_2 + s
    };
    tilt.clamp(0.0, std::f64::consts::FRAC_PI_2)
}

fn great_circle_metrics(radius: f64) -> (f64, f64) {
    let radius = if radius.is_finite() {
        radius.max(0.0)
    } else {
        0.0
    };
    (std::f64::consts::TAU * radius, 0.0)
}

fn great_circle_point(theta: f64, tilt: f64, yaw: f64) -> (f64, f64, f64) {
    let theta = if theta.is_finite() { theta } else { 0.0 };
    let tilt = if tilt.is_finite() {
        tilt.clamp(0.0, std::f64::consts::FRAC_PI_2)
    } else {
        0.0
    };
    let yaw = if yaw.is_finite() { yaw } else { 0.0 };
    let x = tilt.cos() * theta.cos();
    let y = theta.sin();
    let z = -tilt.sin() * theta.cos();
    (
        x * yaw.cos() - y * yaw.sin(),
        x * yaw.sin() + y * yaw.cos(),
        z,
    )
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.42;
    // Sphere outline.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=64 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 64.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Equator.
    prev = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = cy.round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '-');
        }
        prev = Some((px, py));
    }
    // Great circle tilted by alpha about the y-axis, then yawed about z.
    let alpha = alpha.clamp(0.0, std::f64::consts::FRAC_PI_2);
    let n_gc = 3 + if seed == 0 { 0 } else { (seed % 2) as usize };
    for g in 0..n_gc {
        let yaw = g as f64 * std::f64::consts::PI / n_gc as f64;
        prev = None;
        for i in 0..=80 {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / 80.0);
            let (x, y, z) = great_circle_point(th, alpha, yaw);
            // Backface cull lightly: only z >= -0.05
            if z < -0.15 {
                prev = None;
                continue;
            }
            let px = (cx + r * x).round() as i32;
            let py = (cy - r * y * 0.55).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if g == 0 { '#' } else { '*' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
}

/// Sphere geodesic room.
#[derive(Debug, Default)]
pub struct Geodesic {
    seed: u64,
}

impl Geodesic {
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

impl Room for Geodesic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "geodesic",
            title: "Sphere Geodesics",
            wing: "Shape & Space",
            blurb: "Great-circle arcs follow sphere geodesics. t and DRAG: TUNE TILT.",
            accent: [30, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, tilt(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "geodesic",
            root: 196.0,
            tempo: 74,
            line: &[0, 5, 7, 9, 12, 9, 7, 5],
            encodes: "on a sphere the shortest path is always a great circle arc",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TILT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = tilt(t, None, self.seed);
        let deg = a.to_degrees();
        Some(format!("tilt={deg:.0}deg  DRAG:TILT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = tilt(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = tilt(t, hands.last().copied(), self.seed);
        let deg = a.to_degrees();
        let (length, geodesic_curvature) = great_circle_metrics(1.0);
        Some(format!(
            "GREAT C/R={length:.2}  kgR={geodesic_curvature:.0}  T={deg:.1}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "Great circles are the sphere's geodesics: locally straight paths. An arc \
         shorter than a semicircle is the unique shortest path between its endpoints; \
         antipodes have many minimizing semicircles. Parallel transport around a \
         geodesic triangle shows the sphere's curvature as a turn."
    }
}

#[cfg(test)]
mod tests {
    use super::{Geodesic, great_circle_metrics, great_circle_point, tilt};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Geodesic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("tilt"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tilt_changes() {
        let r = Geodesic::new();
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
    fn great_circle_has_zero_geodesic_curvature() {
        let (unit_length, unit_curvature) = great_circle_metrics(1.0);
        let (double_length, double_curvature) = great_circle_metrics(2.0);
        assert!((unit_length - std::f64::consts::TAU).abs() < 1e-12);
        assert!((double_length - 2.0 * std::f64::consts::TAU).abs() < 1e-12);
        assert_eq!(unit_curvature, 0.0);
        assert_eq!(double_curvature, 0.0);
    }

    #[test]
    fn tilt_rotates_a_unit_great_circle() {
        let equator = great_circle_point(0.0, 0.0, 0.0);
        let meridian = great_circle_point(0.0, std::f64::consts::FRAC_PI_2, 0.0);
        assert_ne!(equator, meridian);
        for point in [equator, meridian, great_circle_point(0.7, 0.4, 1.2)] {
            let norm = point.0 * point.0 + point.1 * point.1 + point.2 * point.2;
            assert!((norm - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn seeded_tilt_stays_inside_the_rendered_range() {
        let boundary = tilt(1.0, Some((1.0, 0.5)), 4);
        assert!(boundary <= std::f64::consts::FRAC_PI_2);
        let room = Geodesic::new_with(4);
        let status = room
            .status_input(
                1.0,
                &[RoomInput::PointerDown {
                    x: 1.0,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(status.contains("T=90.0"), "{status}");
    }

    #[test]
    fn copy_distinguishes_geodesics_from_global_minimizers() {
        let reveal = Geodesic::new().reveal();
        assert!(reveal.contains("shorter than a semicircle"));
        assert!(reveal.contains("antipodes"));
    }

    #[test]
    fn duplicate_hand_history_does_not_add_a_great_circle() {
        let room = Geodesic::new();
        let hand = (0.6, 0.4);
        let mut single = Canvas::new(48, 24);
        let mut duplicate = Canvas::new(48, 24);
        room.render_poked(&mut single, 0.3, &[hand]);
        room.render_poked(&mut duplicate, 0.3, &[hand, hand]);
        assert_eq!(single.to_text(), duplicate.to_text());
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Geodesic::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
