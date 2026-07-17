//! Phantom Jam: one brake births a jam that rolls backward forever.
//!
//! Cars on a ring follow a simple follow-the-leader rule. A single slowdown
//! nucleates a dense cluster that propagates upstream against the traffic: the
//! phantom jam (Sugiyama 2008). HOLD: BRAKE plants the seed; `t` runs the ring.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const CARS: usize = 48;
const RING: f64 = 1.0;
const V_MAX: f64 = 0.018;
const SAFE: f64 = 0.028;
const DT: f64 = 1.0;
const MAX_STEPS: usize = 220;
const ENTRY_STEPS: usize = 40;
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

/// Optimal-velocity style update on a ring; one braked car seeds the jam.
fn simulate(steps: usize, brake_at: f64, seed: u64) -> (Vec<f64>, f64, f64) {
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
    let mut vel = vec![V_MAX * 0.7; CARS];
    let brake_i = ((brake_at.clamp(0.0, 0.999) * CARS as f64) as usize).min(CARS - 1);

    for step in 0..steps {
        let mut next_v = vel.clone();
        for i in 0..CARS {
            let j = (i + 1) % CARS;
            let gap = (pos[j] - pos[i]).rem_euclid(RING);
            let target = if gap < SAFE {
                0.0
            } else {
                V_MAX * (1.0 - SAFE / gap).clamp(0.0, 1.0)
            };
            let mut v = vel[i] + 0.35 * (target - vel[i]);
            // Seed brake for a few early steps at the chosen car.
            if i == brake_i && step < 12 {
                v *= 0.15;
            }
            next_v[i] = v.clamp(0.0, V_MAX);
        }
        vel = next_v;
        for i in 0..CARS {
            pos[i] = (pos[i] + vel[i] * DT).rem_euclid(RING);
        }
    }

    let mean_v = vel.iter().sum::<f64>() / CARS as f64;
    // Jam position: densest local cluster center (min average gap neighborhood).
    let mut best_i = 0usize;
    let mut best_dense = f64::INFINITY;
    for i in 0..CARS {
        let mut local = 0.0;
        for k in 0..5 {
            let a = (i + k) % CARS;
            let b = (a + 1) % CARS;
            local += (pos[b] - pos[a]).rem_euclid(RING);
        }
        if local < best_dense {
            best_dense = local;
            best_i = i;
        }
    }
    let jam_x = pos[best_i];
    (pos, mean_v, jam_x)
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
}

impl Room for PhantomJam {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "phantom-jam",
            title: "Phantom Jam",
            wing: "Emergence",
            blurb: "One brake on a ring of cars births a dense jam that rolls backward against \
                    traffic. No accident, no bottleneck: just follow-the-leader (Sugiyama 2008). \
                    t runs the ring; HOLD: BRAKE plants the seed.",
            accent: [230, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (pos, _, jam) = simulate(steps, 0.15 + (self.seed % 5) as f64 * 0.05, self.seed);
        draw_ring(canvas, &pos, jam);
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
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (_, mean_v, jam) = simulate(steps, 0.15, self.seed);
        Some(format!(
            "V={mean_v:.3}  JAM@{:.0}%  HOLD:BRAKE",
            jam * 100.0
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (x, _) = *hands.last().expect("nonempty");
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (pos, _, jam) = simulate(steps, x, self.seed);
        draw_ring(canvas, &pos, jam);
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
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (_, mean_v, jam) = simulate(steps, x, self.seed);
        Some(format!(
            "BRAKE@{:.0}%  V={mean_v:.3}  JAM@{:.0}%",
            x * 100.0,
            jam * 100.0
        ))
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
        let (pos, mean_v, jam) = simulate(80, 0.2, 0);
        assert_eq!(pos.len(), super::CARS);
        assert!(mean_v >= 0.0);
        assert!((0.0..1.0).contains(&jam));
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
