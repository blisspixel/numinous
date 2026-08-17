//! Phantom Jam: one brake births a jam that rolls backward forever.
//!
//! Cars on a ring follow a simple follow-the-leader rule. A single slowdown
//! nucleates a dense cluster that propagates upstream against the traffic: the
//! phantom jam (Sugiyama 2008).
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const CARS: usize = 40;
const RING: f64 = 1.0;
/// How much of the ring one car occupies.
///
/// Without a length, a stopped car and the car behind it converge on the same
/// point and neither can ever move again, so the cluster becomes a permanent
/// standstill instead of a wave. A bumper is what lets a jam discharge at its
/// front while it absorbs arrivals at its back.
const CAR_LENGTH: f64 = 0.008;
/// Speed a driver takes when the road ahead is open.
const V_FREE: f64 = 0.020;
/// Free headway between cars when they are evenly spread around the ring.
const MEAN_ROOM: f64 = RING / CARS as f64 - CAR_LENGTH;
/// Free headway at which a driver has taken half the speed they will take.
///
/// Below the mean, so the even ring sits on the steep part of the response
/// curve where uniform flow is linearly unstable: that instability is the
/// room. Above it, the ring simply flows and there is nothing to see.
const COMFORT: f64 = MEAN_ROOM * 2.0 / 3.0;
/// How sharply the speed choice saturates on either side of [`COMFORT`].
const RESPONSE_WIDTH: f64 = COMFORT / 2.0;
/// How fast a driver closes on the speed their headway invites.
const SENSITIVITY: f64 = 0.35;
const DT: f64 = 1.0;
const MAX_STEPS: usize = 220;
const ENTRY_STEPS: usize = 40;
/// Steps between the two jam sightings whose difference is the drift.
const DRIFT_WINDOW: usize = 20;
/// Cars in the window whose span measures how tight the densest cluster is.
const CLUSTER_CARS: usize = 5;

/// The speed a driver takes for a given free headway (Bando optimal velocity).
///
/// Zero at a closed bumper, rising to [`V_FREE`] on an open road, steepest at
/// [`COMFORT`]. Nothing here knows about jams: the cluster is what this rule
/// does when every driver follows it at once.
fn preferred_speed(room: f64) -> f64 {
    if !room.is_finite() || room <= 0.0 {
        return 0.0;
    }
    let shift = (COMFORT / RESPONSE_WIDTH).tanh();
    V_FREE * (((room - COMFORT) / RESPONSE_WIDTH).tanh() + shift) / (1.0 + shift)
}

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

/// What one run of the ring leaves behind.
struct Ring {
    /// Where every car ended up, as a ring fraction.
    pos: Vec<f64>,
    /// Average speed of the cars, in ring fractions per step.
    mean_speed: f64,
    /// Where the densest cluster of cars sits, as a ring fraction.
    jam_at: f64,
    /// How the cluster is moving, in ring fractions per step. Negative is
    /// upstream, against the direction every car is driving.
    jam_drift: f64,
    /// Span of the densest [`CLUSTER_CARS`] cars. At bumper length times the
    /// window, the cluster is stopped traffic rather than a thick patch.
    jam_span: f64,
}

impl Ring {
    /// The live readout: how the cars are moving, and how the jam is moving.
    ///
    /// Both numbers are percent of the ring per step. The room never says
    /// which way a jam travels; it prints the two signs and lets them
    /// disagree in front of the player.
    fn readout(&self) -> String {
        let packed = self.jam_span <= CLUSTER_CARS as f64 * CAR_LENGTH * 1.02;
        format!(
            "CARS {:+.2}  JAM {:+.2}  {}@{:.0}%",
            self.mean_speed * 100.0,
            self.jam_drift * 100.0,
            if packed { "PACKED" } else { "FORMING" },
            self.jam_at * 100.0
        )
    }
}

/// Where the densest cluster sits and how tight it is.
fn densest_cluster(pos: &[f64]) -> (f64, f64) {
    let mut best_i = 0usize;
    let mut best_span = f64::INFINITY;
    for i in 0..CARS {
        let mut span = 0.0;
        for k in 0..CLUSTER_CARS {
            let a = (i + k) % CARS;
            let b = (a + 1) % CARS;
            span += (pos[b] - pos[a]).rem_euclid(RING);
        }
        if span < best_span {
            best_span = span;
            best_i = i;
        }
    }
    (pos[best_i], best_span)
}

/// How far the ring has run at this phase.
fn steps_at(t: f64) -> usize {
    ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize
}

/// Shortest signed way round the ring from `from` to `to`, in `(-0.5, 0.5]`.
fn ring_delta(from: f64, to: f64) -> f64 {
    let forward = (to - from).rem_euclid(RING);
    if forward > RING / 2.0 {
        forward - RING
    } else {
        forward
    }
}

/// Every driver follows the car ahead; one braked car seeds the disturbance.
///
/// No rule here says "make a jam". The cluster, and the direction it travels,
/// are what the follow-the-leader rule does to an evenly spread ring.
fn simulate(steps: usize, brake_at: f64, seed: u64) -> Ring {
    let steps = steps.min(MAX_STEPS);
    let mut pos: Vec<f64> = (0..CARS)
        .map(|i| {
            let base = i as f64 / CARS as f64;
            if seed == 0 {
                base
            } else {
                (base + ((seed.wrapping_add(i as u64) % 7) as f64) * 0.001).rem_euclid(1.0)
            }
        })
        .collect();
    let mut vel = vec![preferred_speed(MEAN_ROOM); CARS];
    let brake_i = ((brake_at.clamp(0.0, 0.999) * CARS as f64) as usize).min(CARS - 1);
    let sighting_step = steps.saturating_sub(DRIFT_WINDOW);
    let mut earlier_jam = pos[0];

    for step in 0..steps {
        if step == sighting_step {
            earlier_jam = densest_cluster(&pos).0;
        }
        let mut next_v = vel.clone();
        for i in 0..CARS {
            let j = (i + 1) % CARS;
            let room = ((pos[j] - pos[i]).rem_euclid(RING) - CAR_LENGTH).max(0.0);
            let mut v = vel[i] + SENSITIVITY * (preferred_speed(room) - vel[i]);
            // Seed brake for a few early steps at the chosen car.
            if i == brake_i && step < 12 {
                v *= 0.15;
            }
            // A car never drives through the bumper of the car ahead.
            next_v[i] = v.clamp(0.0, room / DT);
        }
        vel = next_v;
        for i in 0..CARS {
            pos[i] = (pos[i] + vel[i] * DT).rem_euclid(RING);
        }
    }

    let (jam_at, jam_span) = densest_cluster(&pos);
    let elapsed = steps - sighting_step;
    let jam_drift = if elapsed == 0 {
        0.0
    } else {
        ring_delta(earlier_jam, jam_at) / elapsed as f64
    };
    Ring {
        mean_speed: vel.iter().sum::<f64>() / CARS as f64,
        pos,
        jam_at,
        jam_drift,
        jam_span,
    }
}

fn draw_ring(canvas: &mut dyn Surface, pos: &[f64], jam_x: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = width.min(height) as f64 * 0.38;
    // Road.
    let steps = 120;
    let mut prev = ((cx + r).round() as i32, cy.round() as i32);
    for i in 1..=steps {
        let a = std::f64::consts::TAU * i as f64 / steps as f64;
        let p = (
            (cx + r * a.cos()).round() as i32,
            (cy + r * a.sin()).round() as i32,
        );
        canvas.line(prev.0, prev.1, p.0, p.1, '.');
        prev = p;
    }
    for &p in pos {
        let a = p * std::f64::consts::TAU;
        let px = (cx + r * a.cos()).round() as i32;
        let py = (cy + r * a.sin()).round() as i32;
        canvas.plot(px, py, '#');
    }
    // Jam marker.
    let a = jam_x * std::f64::consts::TAU;
    let jx = (cx + (r + 4.0) * a.cos()).round() as i32;
    let jy = (cy + (r + 4.0) * a.sin()).round() as i32;
    canvas.plot(jx, jy, '+');
}

fn draw_brake_marker(canvas: &mut dyn Surface, brake_x: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let radius = width.min(height) as f64 * 0.38;
    let half_span = (width.min(height) as f64 * 0.065).clamp(5.0, 46.0);
    let angle = brake_x.clamp(0.0, 1.0) * std::f64::consts::TAU;
    let radial = (angle.cos(), angle.sin());
    let tangent = (-radial.1, radial.0);
    let marker = (cx + radius * radial.0, cy + radius * radial.1);
    for axis in [radial, tangent] {
        canvas.line(
            (marker.0 - half_span * axis.0).round() as i32,
            (marker.1 - half_span * axis.1).round() as i32,
            (marker.0 + half_span * axis.0).round() as i32,
            (marker.1 + half_span * axis.1).round() as i32,
            '!',
        );
    }
}

/// Phantom Jam room.
#[derive(Debug, Default)]
pub struct PhantomJam {
    seed: u64,
}

impl PhantomJam {
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

    /// Which seat on the ring brakes when no hand has chosen one.
    ///
    /// The picture and the readout must run the same ring: they used to
    /// disagree under a variation seed, so the numbers described a jam that
    /// was not the one on screen.
    fn brake_seat(&self) -> f64 {
        0.15 + (self.seed % 5) as f64 * 0.05
    }

}

impl Room for PhantomJam {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let ring = simulate(steps_at(t), self.brake_seat(), self.seed);
        draw_ring(canvas, &ring.pos, ring.jam_at);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "upstream jam",
            root: 155.56,
            tempo: 100,
            line: &[0, 0, 0, 5, 0, 0, 7, 0],
            encodes: "a slow clot rolling against the free flow",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("HOLD: BRAKE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ring = simulate(steps_at(t), self.brake_seat(), self.seed);
        Some(format!("{}  HOLD:BRAKE", ring.readout()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (x, _) = *hands.last().expect("nonempty");
        let ring = simulate(steps_at(t), x, self.seed);
        draw_ring(canvas, &ring.pos, ring.jam_at);
        draw_brake_marker(canvas, x);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        self.render_poked(canvas, t, &crate::held_pokes_from_inputs(inputs));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::held_pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, _) = *hands.last().expect("nonempty");
        let ring = simulate(steps_at(t), x, self.seed);
        Some(format!("BRAKE@{:.0}%  {}", x * 100.0, ring.readout()))
    }

    fn reveal(&self) -> &'static str {
        "A jam can form with no accident and no bottleneck. On a ring, one slow \
         reaction nucleates a dense cluster that travels upstream while cars still \
         drive forward. Sugiyama's 2008 experiment made the phantom jam visible \
         on a real track; the math is follow-the-leader instability."
    }
}

#[cfg(test)]
mod tests {
    use super::{PhantomJam, simulate};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn simulation_places_every_car() {
        let ring = simulate(80, 0.2, 0);
        assert_eq!(ring.pos.len(), super::CARS);
        assert!(ring.mean_speed >= 0.0);
        assert!((0.0..1.0).contains(&ring.jam_at));
    }

    #[test]
    fn the_ring_actually_flows() {
        // Reported dead from packaged play: the same picture at every phase and
        // a mean speed of exactly zero. Forty-eight cars sat closer together
        // than the headway every driver insisted on, so every car chose a
        // target speed of zero on the first step and the ring never moved
        // again. Traffic that never moves cannot have a traffic jam.
        for t in [0.0, 0.2, 0.5, 0.7, 1.0] {
            let ring = simulate(super::steps_at(t), 0.15, 0);
            assert!(
                ring.mean_speed > super::V_FREE * 0.25,
                "the ring is parked at t={t}: mean speed {}",
                ring.mean_speed
            );
        }
    }

    #[test]
    fn the_jam_travels_upstream_while_every_car_drives_forward() {
        // The room's whole claim, measured rather than asserted: the cluster
        // moves the opposite way to the traffic that makes it.
        let ring = simulate(super::MAX_STEPS, 0.15, 0);
        assert!(ring.mean_speed > 0.0, "cars stopped: {}", ring.mean_speed);
        assert!(
            ring.jam_drift < 0.0,
            "the jam is not moving upstream: drift {}",
            ring.jam_drift
        );
        // Cars pass through the cluster: it travels slower than they drive.
        assert!(
            ring.jam_drift.abs() < ring.mean_speed,
            "the jam outruns the traffic: drift {} against speed {}",
            ring.jam_drift,
            ring.mean_speed
        );
        // A jam is stopped traffic, not merely a thicker patch of it.
        assert!(
            ring.jam_span <= super::CLUSTER_CARS as f64 * super::CAR_LENGTH * 1.05,
            "the densest cluster never closed up: span {}",
            ring.jam_span
        );
    }

    #[test]
    fn the_picture_changes_as_the_ring_runs() {
        // Two phases that returned identical text is what "dead" looked like
        // from outside: a player scrubbing the dial learned nothing.
        let room = PhantomJam::new();
        let mut early = Canvas::new(60, 30);
        let mut late = Canvas::new(60, 30);
        room.render(&mut early, 0.2);
        room.render(&mut late, 0.7);
        assert_ne!(early.to_text(), late.to_text());
        assert_ne!(room.status(0.2), room.status(0.7));
    }

    #[test]
    fn an_even_ring_needs_no_brake_to_clot() {
        // The instability is the room: even without the seeded brake the
        // uniform ring is linearly unstable and clots on its own. If this
        // ever passes only because of the brake, the room is a scripted
        // animation rather than a consequence.
        let ring = simulate(super::MAX_STEPS, 0.15, 0);
        let uniform = super::CLUSTER_CARS as f64 / super::CARS as f64;
        assert!(
            ring.jam_span < uniform * 0.6,
            "the ring stayed evenly spread: span {} against uniform {uniform}",
            ring.jam_span
        );
    }

    #[test]
    fn the_readout_describes_the_ring_that_was_drawn() {
        // The status used to simulate a fixed brake seat while the picture
        // used a seed-varied one, so under a variation the numbers described
        // a jam that was not on screen.
        for seed in [0, 3, 4] {
            let room = PhantomJam::new_with(seed);
            let drawn = simulate(super::steps_at(0.5), room.brake_seat(), seed);
            let said = room.status(0.5).expect("status");
            assert!(
                said.contains(&format!("@{:.0}%", drawn.jam_at * 100.0)),
                "seed {seed} readout {said} does not name the drawn jam at {}",
                drawn.jam_at
            );
        }
    }

    #[test]
    fn first_contact_status_invites_brake() {
        let room = PhantomJam::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("HOLD") || open.contains("BRAKE"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn brake_changes_status() {
        let room = PhantomJam::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.6,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("brake");
        assert_ne!(after, open);
        assert!(after.contains("BRAKE"), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = PhantomJam::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.6);
        room.render(&mut b, 0.6);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn brake_site_changes_jam() {
        let room = PhantomJam::new();
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        room.render_poked(&mut a, 0.5, &[(0.1, 0.5)]);
        room.render_poked(&mut b, 0.5, &[(0.8, 0.5)]);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn brake_has_an_immediate_visible_marker() {
        let room = PhantomJam::new();
        let mut open = Canvas::new(80, 50);
        let mut braking = Canvas::new(80, 50);
        room.render(&mut open, 0.0);
        room.render_poked(&mut braking, 0.0, &[(0.82, 0.5)]);
        let changed = (0..50)
            .flat_map(|y| (0..80).map(move |x| (x, y)))
            .filter(|&(x, y)| open.cell(x, y) != braking.cell(x, y))
            .count();
        assert!(changed >= 20, "brake marker changed only {changed} cells");
    }

    #[test]
    fn variation_changes_render() {
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        PhantomJam::new_with(0).render(&mut a, 0.5);
        PhantomJam::new_with(4).render(&mut b, 0.5);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = PhantomJam::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_jam_or_sugiyama() {
        let text = PhantomJam::new().reveal().to_ascii_lowercase();
        assert!(text.contains("jam") || text.contains("sugiyama") || text.contains("upstream"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = PhantomJam::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
