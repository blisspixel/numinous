//! Berry phase: holonomy after a closed loop in parameter space.
//!
//! DRAG: TUNE LOOP. See `docs/ROOMS.md`.

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

fn loop_r(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.25 + x * 0.7 + s
    } else {
        0.35 + phase_unit(t) * 0.5 + s
    }
}

fn berry_phase_magnitude(radius: f64) -> (f64, f64) {
    let radius = if radius.is_finite() {
        radius.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let solid_angle = std::f64::consts::TAU * (1.0 - (1.0 - radius * radius).sqrt());
    (solid_angle, 0.5 * solid_angle)
}

fn bloch_loop_point(radius: f64, tilt: f64, azimuth: f64) -> (f64, f64, f64) {
    let radius = if radius.is_finite() {
        radius.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let tilt = if tilt.is_finite() { tilt } else { 0.0 };
    let azimuth = if azimuth.is_finite() { azimuth } else { 0.0 };
    let z = (1.0 - radius * radius).sqrt();
    let x = radius * azimuth.cos();
    let y = radius * azimuth.sin();
    (
        x,
        y * tilt.cos() - z * tilt.sin(),
        y * tilt.sin() + z * tilt.cos(),
    )
}

fn draw(canvas: &mut dyn Surface, rho: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rho = rho.clamp(0.2, 1.0);
    let r_sphere = (width.min(height) as f64) * 0.4;
    // Bloch sphere outline.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + r_sphere * th.cos()).round() as i32;
        let py = (cy - r_sphere * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Parameter loop as a circle of radius rho about a pole tilt.
    let tilt = 0.4
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.08
        };
    prev = None;
    for i in 0..=64 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 64.0);
        let (x, y, z) = bloch_loop_point(rho, tilt, th);
        let px = (cx + r_sphere * x).round() as i32;
        let py = (cy - r_sphere * (y * 0.55 + 0.15 * z)).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // Solid angle ~ 2pi (1 - cos alpha) toy; mark as chord of state vector.
    let (_, phase) = berry_phase_magnitude(rho);
    let tip_x = (cx + r_sphere * 0.7 * phase.cos()).round() as i32;
    let tip_y = (cy - r_sphere * 0.4 * phase.sin()).round() as i32;
    canvas.line(cx as i32, cy as i32, tip_x, tip_y, '+');
}

/// Berry phase room.
#[derive(Debug, Default)]
pub struct Berry {
    seed: u64,
}

impl Berry {
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

impl Room for Berry {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "berry",
            title: "Berry Phase",
            wing: "Waves & Sound",
            blurb: "Holonomy after a closed parameter loop. t and DRAG: TUNE LOOP.",
            accent: [160, 80, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, loop_r(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "berry",
            root: 622.25,
            tempo: 98,
            line: &[0, 5, 7, 12, 7, 5, 3, 12],
            encodes: "adiabatic loop on Bloch sphere earns geometric Berry phase",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LOOP")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = loop_r(t, None, self.seed);
        let (_, ph) = berry_phase_magnitude(r);
        Some(format!("r={r:.2}  |g|={ph:.2}  DRAG:LOOP"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = loop_r(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed);
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
        let r = loop_r(t, hands.last().copied(), self.seed);
        let (solid, phase) = berry_phase_magnitude(r);
        Some(format!("LOOP Om={solid:.2}  |gamma|={phase:.2}rad"))
    }

    fn reveal(&self) -> &'static str {
        "When a quantum state is steered slowly around a closed loop in parameter \
         space, its geometric Berry phase has magnitude equal to half the solid \
         angle enclosed on the Bloch sphere. The sign depends on the state and \
         loop orientation. Holonomy, not dynamics."
    }
}

#[cfg(test)]
mod tests {
    use super::{Berry, berry_phase_magnitude, bloch_loop_point};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Berry::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("g="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn loop_changes() {
        let r = Berry::new();
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
    fn phase_is_half_the_enclosed_solid_angle() {
        let (small_solid, small_phase) = berry_phase_magnitude(0.25);
        let (large_solid, large_phase) = berry_phase_magnitude(0.9);
        assert!((small_phase * 2.0 - small_solid).abs() < 1e-12);
        assert!((large_phase * 2.0 - large_solid).abs() < 1e-12);
        assert!(large_phase > small_phase);
    }

    #[test]
    fn copy_names_phase_magnitude_without_inventing_a_sign() {
        let room = Berry::new();
        assert!(room.status(0.0).unwrap().contains("|g|"));
        let status = room
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(status.contains("|gamma|"));
        assert!(room.reveal().contains("magnitude"));
    }

    #[test]
    fn parameter_loop_stays_on_the_bloch_sphere() {
        for radius in [0.0, 0.25, 0.9, 1.0] {
            for theta in [0.0, 0.7, std::f64::consts::PI, std::f64::consts::TAU] {
                let (x, y, z) = bloch_loop_point(radius, 0.6, theta);
                assert!((x * x + y * y + z * z - 1.0).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn duplicate_hand_history_does_not_reorient_the_loop() {
        let room = Berry::new_with(2);
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
        Berry::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
