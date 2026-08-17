//! Berry phase: holonomy after a closed loop in parameter space.
//!
//! DRAG: TUNE LOOP. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
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

/// How far round the loop the picture is sampled.
const LOOP_SAMPLES: usize = 64;

/// The tilt of the loop's own pole away from the sphere's.
fn loop_tilt(seed: u64) -> f64 {
    0.4 + if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.08
    }
}

/// Draw the sphere, the area the loop encloses, and the turn that area costs.
///
/// A packaged playtest read the old picture exactly right: "the picture is
/// still a circle and a number." It drew the sphere, the loop, and one chord
/// whose screen angle happened to be the phase, which is a number wearing a
/// line. Nothing in it showed the thing the room is named for.
///
/// A Berry phase is a mismatch you can point at. Carry a vector round a closed
/// loop, keeping it as parallel as the sphere allows, and it comes back
/// pointing somewhere else. The angle it missed by is the phase, and it is half
/// the area the loop enclosed. So the picture now draws both halves of that
/// sentence: the cap is shaded, and the vector is drawn twice, as it set out
/// and as it came back. Turning the dial opens the cap and fans the two apart
/// together, which is the theorem happening rather than the theorem quoted.
///
/// The two arms are drawn in the picture plane rather than tangent to the
/// sphere. What is exact is the angle between them.
fn draw(canvas: &mut dyn Surface, rho: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rho = rho.clamp(0.2, 1.0);
    let r_sphere = (width.min(height) as f64) * 0.4;
    let project = |x: f64, y: f64, z: f64| {
        (
            (cx + r_sphere * x).round() as i32,
            (cy - r_sphere * (y * 0.55 + 0.15 * z)).round() as i32,
        )
    };

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

    // The parameter loop, a circle of radius rho about its own tilted pole.
    let tilt = loop_tilt(seed);
    let ring: Vec<(i32, i32)> = (0..=LOOP_SAMPLES)
        .map(|i| {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / LOOP_SAMPLES as f64);
            let (x, y, z) = bloch_loop_point(rho, tilt, th);
            project(x, y, z)
        })
        .collect();
    let (pole_x, pole_y, pole_z) = bloch_loop_point(0.0, tilt, 0.0);
    let pole = project(pole_x, pole_y, pole_z);

    // The enclosed cap, fanned from the loop's pole out to the loop, so the
    // area is a thing on screen and not only a term in a formula.
    for &(x, y) in &ring {
        canvas.line(pole.0, pole.1, x, y, ':');
    }
    for pair in ring.windows(2) {
        canvas.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, '#');
    }

    // The vector as it set out and as it came back, from the point on the loop
    // where the journey began. They start together at a tiny loop and swing
    // apart as the cap opens.
    let (_, phase) = berry_phase_magnitude(rho);
    let base = ring[0];
    let arm = r_sphere * 0.62;
    let away = (
        f64::from(base.0 - pole.0),
        f64::from(base.1 - pole.1),
    );
    let reach = away.0.hypot(away.1).max(1.0);
    let heading = (away.0 / reach, away.1 / reach);
    let turned = (
        heading.0 * phase.cos() - heading.1 * phase.sin(),
        heading.0 * phase.sin() + heading.1 * phase.cos(),
    );
    let tip = |direction: (f64, f64)| {
        (
            base.0 + (direction.0 * arm).round() as i32,
            base.1 + (direction.1 * arm * 0.55).round() as i32,
        )
    };
    let (set_out_x, set_out_y) = tip(heading);
    let (came_back_x, came_back_y) = tip(turned);
    canvas.line(base.0, base.1, set_out_x, set_out_y, '=');
    canvas.line(base.0, base.1, came_back_x, came_back_y, '*');
    canvas.plot(base.0, base.1, 'O');
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
        let (solid, ph) = berry_phase_magnitude(r);
        // The area enclosed and the angle it cost, each once. Bars rather than
        // a sign, because the room measures a magnitude and the direction
        // belongs to the state and the orientation of the loop. Steradians
        // beside degrees, so the reading does not spell out the factor between
        // them: watching the cap open while the two arms fan is the way to meet
        // that, and the reveal is where it gets said.
        Some(format!(
            "AREA {solid:.2}sr  |g| {:.0}deg  DRAG:LOOP",
            ph.to_degrees()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = loop_r(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = loop_r(t, hands.last().copied(), self.seed);
        let (solid, phase) = berry_phase_magnitude(r);
        Some(format!(
            "LOOP {r:.2}  AREA Om={solid:.2}  |gamma|={phase:.2}rad"
        ))
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
    use super::{Berry, berry_phase_magnitude, bloch_loop_point, loop_r};
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
    fn the_loop_shows_the_turn_it_costs() {
        // A packaged playtest read the old picture exactly right: "the picture
        // is still a circle and a number." The area a loop encloses and the
        // angle the carried vector misses by are the two halves of the room,
        // and both have to be on screen, moving together as the dial turns.
        let room = Berry::new();
        let mut frames = std::collections::HashSet::new();
        for step in 0..=12 {
            let mut canvas = Canvas::new(66, 26);
            room.render(&mut canvas, f64::from(step) / 12.0);
            frames.insert(canvas.to_text());
        }
        assert!(
            frames.len() >= 10,
            "thirteen turns of the dial drew only {} pictures",
            frames.len()
        );
        // A small loop earns almost no turn, a large one earns a lot, and the
        // second is drawn with more of the cap shaded than the first.
        let ink = |phase: f64| {
            let mut canvas = Canvas::new(66, 26);
            room.render(&mut canvas, phase);
            canvas.ink_count()
        };
        assert!(
            ink(1.0) > ink(0.0),
            "opening the loop has to shade more of the sphere"
        );
        let (small_area, small_turn) = berry_phase_magnitude(loop_r(0.0, None, 0));
        let (large_area, large_turn) = berry_phase_magnitude(loop_r(1.0, None, 0));
        assert!(large_area > small_area && large_turn > small_turn);
        // The status reports both measurements and never the relation between
        // them, which is the discovery the room exists to let a player make.
        let reading = room.status(1.0).expect("status");
        assert!(reading.contains("AREA") && reading.contains("|g|"), "{reading}");
        assert!(
            !reading.to_ascii_lowercase().contains("half"),
            "the status must not hand over the theorem: {reading}"
        );
        // Each quantity once, in its own unit. Printing the turn in radians
        // beside an area in steradians would spell out the factor between them,
        // which is the sentence the reveal is holding.
        assert!(
            !reading.contains("rad"),
            "the reading says the same thing twice: {reading}"
        );
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Berry::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

