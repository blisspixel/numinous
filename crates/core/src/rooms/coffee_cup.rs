//! The Coffee Cup: one bounce in a circle, and a cardioid condenses.
//!
//! Light leaves a source on the rim of a circular cup. Every ray reflects once
//! with equal angle of incidence and reflection, then is drawn a short way past
//! the bounce. The bright envelope of those reflected rays is a cardioid that
//! kisses the rim at the source. `t` walks the ambient sun; DRAG: SWING THE SUN.
//! Closes the cardioid triangle with Times Tables and Mandelbrot. See
//! `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{
    Gesture, MAX_ROOM_POKES, Room, RoomInput, RoomMeta, latest_gesture, pokes_from_inputs,
};
use crate::surface::Surface;

/// How many source rays are cast around the circle.
const RAY_COUNT: usize = 180;
/// Length of each reflected segment in circle radii (drawn from the bounce).
const REFLECT_LEN: f64 = 1.65;
/// Salt for nonzero variation sun offset.
const VARIATION_SALT: u64 = 0xC0FF_EE00_5EED_0001;

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

/// Ambient sun angle (radians) from phase and optional variation.
fn ambient_sun(t: f64, seed: u64) -> f64 {
    let base = phase_unit(t) * TAU;
    if seed == 0 {
        base
    } else {
        let mix = seed ^ VARIATION_SALT;
        let offset = ((mix % 360) as f64) * TAU / 360.0;
        (base + offset) % TAU
    }
}

/// Map a hand point to a sun angle on the rim (atan2 from center).
fn sun_from_hand(x: f64, y: f64) -> f64 {
    let dx = x - 0.5;
    let dy = y - 0.5;
    if dx.abs() < 1e-12 && dy.abs() < 1e-12 {
        0.0
    } else {
        dy.atan2(dx).rem_euclid(TAU)
    }
}

/// Point on the unit circle at angle `theta` (center origin).
fn on_circle(theta: f64) -> (f64, f64) {
    (theta.cos(), theta.sin())
}

/// Reflect incident direction `inc` off the unit-circle normal at bounce point.
///
/// `inc` points toward the wall; return is the outgoing direction after bounce.
fn reflect(inc: (f64, f64), normal: (f64, f64)) -> (f64, f64) {
    let dot = inc.0 * normal.0 + inc.1 * normal.1;
    (inc.0 - 2.0 * dot * normal.0, inc.1 - 2.0 * dot * normal.1)
}

fn unit(v: (f64, f64)) -> (f64, f64) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 1e-12 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

/// Geometry shared by render and status: circle in the letterboxed plate.
struct PlateGeom {
    cx: f64,
    cy: f64,
    radius: f64,
}

impl PlateGeom {
    fn new(width: usize, height: usize) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let fw = width as f64;
        let fh = height as f64;
        let radius = (fw.min(fh) * 0.42).max(2.0);
        Some(Self {
            cx: fw / 2.0,
            cy: fh / 2.0,
            radius,
        })
    }

    fn to_px(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (self.cx + x * self.radius).round() as i32,
            (self.cy + y * self.radius).round() as i32,
        )
    }
}

fn draw_cup(canvas: &mut dyn Surface, sun: f64) {
    let (width, height) = canvas.draw_bounds();
    let Some(geom) = PlateGeom::new(width, height) else {
        return;
    };

    // Rim of the cup.
    let rim_steps = 240;
    let mut prev = geom.to_px(1.0, 0.0);
    for i in 1..=rim_steps {
        let th = (i as f64 / rim_steps as f64) * TAU;
        let p = geom.to_px(th.cos(), th.sin());
        canvas.line(prev.0, prev.1, p.0, p.1, '.');
        prev = p;
    }

    let sun_pt = on_circle(sun);
    let (sx, sy) = geom.to_px(sun_pt.0, sun_pt.1);
    // Sun mark.
    canvas.plot(sx, sy, '#');
    canvas.plot(sx + 1, sy, '#');
    canvas.plot(sx, sy + 1, '#');
    canvas.plot(sx - 1, sy, '#');
    canvas.plot(sx, sy - 1, '#');

    // Cast rays from the sun, bounce once, draw the reflected segment.
    // Bounce angles skip the sun itself so we do not reflect from the source.
    for i in 0..RAY_COUNT {
        let bounce = (i as f64 / RAY_COUNT as f64) * TAU;
        let delta = (bounce - sun).rem_euclid(TAU);
        if !(0.04..=TAU - 0.04).contains(&delta) {
            continue;
        }
        let bpt = on_circle(bounce);
        // Incident vector: from sun to bounce (toward the wall).
        let inc = unit((bpt.0 - sun_pt.0, bpt.1 - sun_pt.1));
        // Outward normal at bounce is the radius vector.
        let normal = bpt;
        let out = unit(reflect(inc, normal));
        // Draw a short chord of the reflected ray inside the cup.
        let end = (bpt.0 + out.0 * REFLECT_LEN, bpt.1 + out.1 * REFLECT_LEN);
        // Clip the segment to stay mostly inside the visible disk.
        let mut ex = end.0;
        let mut ey = end.1;
        let r2 = ex * ex + ey * ey;
        if r2 > 1.0 {
            // Stop at the far rim so ink stays cup-shaped.
            let scale = 1.0 / r2.sqrt();
            ex *= scale;
            ey *= scale;
        }
        let (bx, by) = geom.to_px(bpt.0, bpt.1);
        let (ex, ey) = geom.to_px(ex, ey);
        canvas.line(bx, by, ex, ey, '*');
    }
}

/// The Coffee Cup room.
#[derive(Debug, Default)]
pub struct CoffeeCup {
    seed: u64,
}

impl CoffeeCup {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// Sun angle for a static poke list (CLI/MCP bridge).
    ///
    /// A hand pins the sun on the rim. Do not add ambient phase here: that
    /// collides with idle t and can paint the same cardioid as no poke (dead
    /// domain). Walking after a release lives in `sun_from_inputs`.
    fn sun_from_pokes(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        let hands = finite_pokes(pokes);
        if let Some(&(x, y)) = hands.last() {
            sun_from_hand(x, y)
        } else {
            ambient_sun(t, self.seed)
        }
    }

    /// Sun angle from a full gesture trail.
    ///
    /// Held: pin exactly to the hand (DRAG: SWING THE SUN). Released or
    /// cancelled: keep walking from the last hand angle so the cup stays alive.
    /// Idle: ambient walk with variation seed.
    fn sun_from_inputs(&self, t: f64, inputs: &[RoomInput]) -> f64 {
        match latest_gesture(inputs) {
            Some(Gesture::Held { at, .. }) => sun_from_hand(at.0, at.1),
            Some(Gesture::Released { at, .. }) | Some(Gesture::Cancelled { at }) => {
                walking_sun(sun_from_hand(at.0, at.1), t)
            }
            None => ambient_sun(t, self.seed),
        }
    }
}

/// Sticky origin plus ambient phase so a finished swing keeps moving.
fn walking_sun(sticky: f64, t: f64) -> f64 {
    (sticky + phase_unit(t) * TAU).rem_euclid(TAU)
}

impl Room for CoffeeCup {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coffee-cup",
            title: "The Coffee Cup",
            wing: "Shape & Space",
            blurb: "Rays bounce once in a circle and condense into a cardioid. t walks the sun on \
                    the rim; DRAG: SWING THE SUN. Same cardioid curve as Times Tables and the \
                    Mandelbrot main bulb.",
            accent: [230, 150, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw_cup(canvas, ambient_sun(t, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.18
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rim cardioid",
            root: 174.61,
            tempo: 100,
            line: &[0, 4, 7, 11, 12, 7, 4, 0],
            encodes: "one bounce folding every ray into a single cusp heart",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SWING THE SUN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let sun = ambient_sun(t, self.seed);
        let deg = (sun * 360.0 / TAU).round() as i32;
        Some(format!("SUN {deg}deg  CARDIOID  DRAG:SWING"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // No hand trail ink: the App reticle owns live chrome, and a trail of
        // + marks over the rays is noise after a swing.
        draw_cup(canvas, self.sun_from_pokes(t, pokes));
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        draw_cup(canvas, self.sun_from_inputs(t, inputs));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let sun = self.sun_from_inputs(t, inputs);
        let deg = (sun * 360.0 / TAU).round() as i32;
        match latest_gesture(inputs) {
            Some(Gesture::Held { at, .. }) => {
                let nx = at.0.clamp(0.0, 1.0);
                let ny = at.1.clamp(0.0, 1.0);
                Some(format!(
                    "SWING {deg}deg  CUSP@{:.0}%{:.0}%  RAYS {RAY_COUNT}",
                    nx * 100.0,
                    ny * 100.0
                ))
            }
            Some(Gesture::Released { .. }) | Some(Gesture::Cancelled { .. }) => {
                // Keep naming the walk after a swing so the room still answers.
                Some(format!("WALK {deg}deg  CARDIOID  RAYS {RAY_COUNT}"))
            }
            None => {
                // Static poke bridges (CLI/MCP) still need a consequence line.
                let pokes = pokes_from_inputs(inputs);
                let hands = finite_pokes(&pokes);
                if hands.is_empty() {
                    return self.status(t);
                }
                let (nx, ny) = *hands.last().expect("nonempty hands");
                Some(format!(
                    "SWING {deg}deg  CUSP@{:.0}%{:.0}%  RAYS {RAY_COUNT}",
                    nx * 100.0,
                    ny * 100.0
                ))
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "Every ray that leaves the sun and bounces once in the circle hugs a \
         cardioid whose cusp sits at the sun. The same curve is the envelope of \
         chords in Times Tables and the main bulb boundary of the Mandelbrot set: \
         one cardioid, three rooms, three verbs."
    }
}

#[cfg(test)]
mod tests {
    use super::{CoffeeCup, ambient_sun, reflect, sun_from_hand, unit};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};
    use std::f64::consts::{FRAC_PI_2, TAU};

    #[test]
    fn reflection_reverses_the_normal_component() {
        // Head-on into the wall (rightward into x=1) reflects back leftward.
        let out = reflect((1.0, 0.0), (1.0, 0.0));
        assert!((out.0 + 1.0).abs() < 1e-12);
        assert!(out.1.abs() < 1e-12);
    }

    #[test]
    fn unit_normalizes() {
        let u = unit((3.0, 4.0));
        assert!((u.0 - 0.6).abs() < 1e-12);
        assert!((u.1 - 0.8).abs() < 1e-12);
    }

    #[test]
    fn sun_from_hand_reads_angle() {
        let east = sun_from_hand(1.0, 0.5);
        assert!(east.abs() < 1e-9 || (east - TAU).abs() < 1e-9);
        let north = sun_from_hand(0.5, 0.0);
        // Hand y=0 is above center (dy = -0.5), atan2(-0.5, 0) = -pi/2, rem_euclid TAU.
        assert!((north - (TAU - FRAC_PI_2)).abs() < 1e-9);
    }

    #[test]
    fn ambient_sun_sweeps_and_variation_offsets() {
        assert!((ambient_sun(0.0, 0) - 0.0).abs() < 1e-12);
        assert!((ambient_sun(1.0, 0) - TAU).abs() < 1e-9 || ambient_sun(1.0, 0).abs() < 1e-9);
        assert_ne!(
            (ambient_sun(0.25, 0) * 1000.0).round(),
            (ambient_sun(0.25, 9) * 1000.0).round()
        );
    }

    #[test]
    fn first_contact_status_invites_a_swing() {
        let room = CoffeeCup::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("SUN"), "{open}");
        assert!(open.contains("DRAG"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn swing_changes_status() {
        let room = CoffeeCup::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.2,
            y: 0.8,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("swing");
        assert_ne!(after, open);
        assert!(after.contains("SWING"), "{after}");
        assert!(after.contains("CUSP@"), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = CoffeeCup::new();
        let mut a = Canvas::new(50, 36);
        let mut b = Canvas::new(50, 36);
        room.render(&mut a, 0.2);
        room.render(&mut b, 0.2);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 80);
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 80);
    }

    #[test]
    fn hand_sun_changes_the_figure() {
        let room = CoffeeCup::new();
        let mut base = Canvas::new(48, 32);
        let mut poked = Canvas::new(48, 32);
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[(0.1, 0.9)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn after_a_swing_the_sun_keeps_walking() {
        let room = CoffeeCup::new();
        let inputs = [
            RoomInput::PointerDown {
                x: 0.95,
                y: 0.5,
                t: 0.0,
            },
            RoomInput::PointerUp {
                x: 0.95,
                y: 0.5,
                t: 0.1,
            },
        ];
        let mut early = Canvas::new(48, 32);
        let mut late = Canvas::new(48, 32);
        room.render_input(&mut early, 0.05, &inputs);
        room.render_input(&mut late, 0.55, &inputs);
        assert_ne!(
            early.to_text(),
            late.to_text(),
            "released swing must not freeze the cardioid"
        );
    }

    #[test]
    fn a_completed_drag_does_not_leave_hand_trail_ink() {
        let room = CoffeeCup::new();
        let mut ambient = Canvas::new(48, 32);
        let mut swung = Canvas::new(48, 32);
        room.render(&mut ambient, 0.0);
        // End on the top rim so the sticky sun is far from ambient east.
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: 0.4,
                y: 0.15,
                t: 0.05,
            },
            RoomInput::PointerMove {
                x: 0.55,
                y: 0.08,
                t: 0.08,
            },
            RoomInput::PointerUp {
                x: 0.5,
                y: 0.0,
                t: 0.1,
            },
        ];
        room.render_input(&mut swung, 0.0, &inputs);
        // No '+' hand trail: ink should only be cup geometry (., *, #).
        assert_eq!(
            swung.to_text().matches('+').count(),
            0,
            "hand trail must not remain as + marks over the cup"
        );
        assert_ne!(
            ambient.to_text(),
            swung.to_text(),
            "the swing still changes the math"
        );
    }

    #[test]
    fn held_hand_pins_the_sun_without_phase_walk() {
        let room = CoffeeCup::new();
        let held = [RoomInput::PointerDown {
            x: 1.0,
            y: 0.5,
            t: 0.0,
        }];
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        room.render_input(&mut a, 0.1, &held);
        room.render_input(&mut b, 0.7, &held);
        assert_eq!(
            a.to_text(),
            b.to_text(),
            "while held, the sun should pin to the hand"
        );
    }

    #[test]
    fn variation_changes_ambient_sun() {
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        CoffeeCup::new_with(0).render(&mut a, 0.3);
        CoffeeCup::new_with(9).render(&mut b, 0.3);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(40, 28);
        CoffeeCup::new().render(&mut zero, 0.3);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = CoffeeCup::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
            room.render_poked(&mut canvas, t, &[(0.5, 0.5)]);
        }
    }

    #[test]
    fn reveal_names_cardioid_and_cross_room_link() {
        let text = CoffeeCup::new().reveal().to_ascii_lowercase();
        assert!(text.contains("cardioid"));
        assert!(text.contains("mandelbrot") || text.contains("times"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = CoffeeCup::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
