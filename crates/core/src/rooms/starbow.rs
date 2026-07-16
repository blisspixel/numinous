//! The Starbow: burn toward lightspeed and watch the sky pour into a ring.
//!
//! Relativistic aberration (McKinley 1979 closed form) maps every rest-frame
//! star angle theta into a forward cone: cos theta' = (cos theta + beta) /
//! (1 + beta cos theta), with beta = v/c. As beta climbs toward 1 the whole
//! celestial sphere floods into a burning ring ahead. `t` burns ambient speed;
//! HOLD: BURN under the hand. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Fixed catalog of rest-frame stars (deterministic).
const STAR_COUNT: usize = 220;
/// Base seed for the star field.
const FIELD_SEED: u64 = 0x57A2_B0B0_5EED_0001;
/// Salt for nonzero variation field remix.
const VARIATION_SALT: u64 = 0x57A2_B0B0_5EED_0002;
/// Ambient beta at t = 0 (a gentle cruise).
const BETA_MIN: f64 = 0.05;
/// Ambient beta at t = 1 (near lightspeed).
const BETA_MAX: f64 = 0.97;

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

/// Ambient speed as a fraction of light.
fn ambient_beta(t: f64) -> f64 {
    BETA_MIN + phase_unit(t) * (BETA_MAX - BETA_MIN)
}

/// Hand burn: y low (top of screen) is faster, matching a throttle push.
fn beta_from_hand(y: f64) -> f64 {
    let u = 1.0 - y.clamp(0.0, 1.0);
    BETA_MIN + u * (0.995 - BETA_MIN)
}

/// McKinley / special-relativity aberration: rest angle -> forward angle.
fn aberrate(cos_theta: f64, beta: f64) -> f64 {
    let beta = beta.clamp(0.0, 0.999_999);
    let c = cos_theta.clamp(-1.0, 1.0);
    let num = c + beta;
    let den = 1.0 + beta * c;
    if den.abs() < 1e-15 {
        1.0
    } else {
        (num / den).clamp(-1.0, 1.0)
    }
}

/// Rest-frame stars as (cos_theta, azimuth), theta from the boost axis.
fn star_field(seed: u64) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(FIELD_SEED ^ seed.wrapping_mul(VARIATION_SALT | 1));
    (0..STAR_COUNT)
        .map(|_| {
            // Uniform on the sphere: cos_theta uniform in [-1, 1], azimuth uniform.
            let cos_theta = rng.next_f64() * 2.0 - 1.0;
            let az = rng.next_f64() * 2.0 * PI;
            (cos_theta, az)
        })
        .collect()
}

fn draw_starbow(canvas: &mut dyn Surface, beta: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let radius = width.min(height) as f64 * 0.46;
    // Crosshair: direction of travel (dead ahead).
    canvas.plot(cx.round() as i32, cy.round() as i32, '+');

    for (cos_theta, az) in star_field(seed) {
        let cos_p = aberrate(cos_theta, beta);
        // Project onto the sky disk: radial distance from center encodes polar angle.
        // theta' = 0 (ahead) at center; theta' = pi (behind) at the rim.
        let theta_p = cos_p.clamp(-1.0, 1.0).acos();
        let r = (theta_p / PI) * radius;
        let px = (cx + r * az.cos()).round() as i32;
        let py = (cy + r * az.sin()).round() as i32;
        // Brighter (denser mark) when packed near the ring at high beta.
        let mark = if beta > 0.85 && (0.35..0.75).contains(&(r / radius)) {
            '#'
        } else if beta > 0.6 {
            '*'
        } else {
            '.'
        };
        canvas.plot(px, py, mark);
    }
}

/// Fraction of stars packed into the forward half-cone (theta' < pi/2).
fn forward_fraction(beta: f64, seed: u64) -> f64 {
    let mut forward = 0usize;
    for (cos_theta, _) in star_field(seed) {
        if aberrate(cos_theta, beta) > 0.0 {
            forward += 1;
        }
    }
    forward as f64 / STAR_COUNT as f64
}

/// The Starbow room.
#[derive(Debug, Default)]
pub struct Starbow {
    seed: u64,
}

impl Starbow {
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

    fn beta_at(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        let hands = finite_pokes(pokes);
        if let Some(&(_, y)) = hands.last() {
            beta_from_hand(y)
        } else {
            ambient_beta(t)
        }
    }
}

impl Room for Starbow {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "starbow",
            title: "The Starbow",
            wing: "Shape & Space",
            blurb: "Burn toward lightspeed; relativistic aberration pours the whole sky into a \
                    burning ring ahead. One closed-form transform per star (McKinley 1979). t \
                    burns ambient beta; HOLD: BURN under the hand.",
            accent: [255, 200, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw_starbow(canvas, ambient_beta(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.92
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lightspeed ring",
            root: 233.08,
            tempo: 128,
            line: &[0, 7, 12, 14, 12, 7, 0, 12],
            encodes: "stars flooding forward into a single bright ring",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("HOLD: BURN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let beta = ambient_beta(t);
        let fwd = forward_fraction(beta, self.seed);
        Some(format!(
            "v={:.0}%c  FWD {:.0}%  HOLD:BURN",
            beta * 100.0,
            fwd * 100.0
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let beta = self.beta_at(t, pokes);
        draw_starbow(canvas, beta, self.seed);
        let (width, height) = canvas.draw_bounds();
        if width > 0 && height > 0 {
            for &(x, y) in &hands {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.plot(px, py, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let beta = self.beta_at(t, &pokes);
        let fwd = forward_fraction(beta, self.seed);
        let gamma = 1.0 / (1.0 - beta * beta).max(1e-12).sqrt();
        Some(format!(
            "BURN v={:.0}%c  g={:.1}  FWD {:.0}%",
            beta * 100.0,
            gamma,
            fwd * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "At rest the stars fill the sky. Burn toward lightspeed and aberration \
         pours every direction into a bright ring dead ahead: cos theta' = \
         (cos theta + beta) / (1 + beta cos theta). One closed form per star, \
         no simulation, just the geometry of Minkowski space (McKinley 1979)."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BETA_MAX, BETA_MIN, Starbow, aberrate, ambient_beta, beta_from_hand, forward_fraction,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn zero_beta_is_identity() {
        for c in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            assert!((aberrate(c, 0.0) - c).abs() < 1e-12);
        }
    }

    #[test]
    fn high_beta_pushes_side_stars_forward() {
        // A star at 90 degrees (cos = 0) swings forward under boost.
        let c = aberrate(0.0, 0.9);
        assert!(c > 0.5, "side star should appear ahead, got cos={c}");
    }

    #[test]
    fn forward_fraction_grows_with_beta() {
        let slow = forward_fraction(0.1, 0);
        let fast = forward_fraction(0.95, 0);
        assert!(fast > slow);
        assert!(fast > 0.7);
    }

    #[test]
    fn ambient_beta_sweeps() {
        assert!((ambient_beta(0.0) - BETA_MIN).abs() < 1e-12);
        assert!((ambient_beta(1.0) - BETA_MAX).abs() < 1e-12);
    }

    #[test]
    fn hand_top_burns_harder() {
        assert!(beta_from_hand(0.0) > beta_from_hand(1.0));
    }

    #[test]
    fn first_contact_status_invites_burn() {
        let room = Starbow::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("HOLD"), "{open}");
        assert!(open.contains("BURN") || open.contains("v="), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn burn_changes_status() {
        let room = Starbow::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.05,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("burn");
        assert_ne!(after, open);
        assert!(after.contains("BURN"), "{after}");
        assert!(after.contains("g="), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Starbow::new();
        let mut a = Canvas::new(50, 36);
        let mut b = Canvas::new(50, 36);
        room.render(&mut a, 0.8);
        room.render(&mut b, 0.8);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 40);
    }

    #[test]
    fn burn_changes_the_sky() {
        let room = Starbow::new();
        let mut slow = Canvas::new(48, 32);
        let mut fast = Canvas::new(48, 32);
        room.render_poked(&mut slow, 0.0, &[(0.5, 0.95)]);
        room.render_poked(&mut fast, 0.0, &[(0.5, 0.05)]);
        assert_ne!(slow.to_text(), fast.to_text());
    }

    #[test]
    fn variation_remixes_the_field() {
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        Starbow::new_with(0).render(&mut a, 0.7);
        Starbow::new_with(3).render(&mut b, 0.7);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(48, 32);
        Starbow::new().render(&mut zero, 0.7);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Starbow::new();
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
    fn reveal_names_aberration_or_mckinley() {
        let text = Starbow::new().reveal().to_ascii_lowercase();
        assert!(text.contains("aberrat") || text.contains("beta"));
        assert!(text.contains("mckinley") || text.contains("ring") || text.contains("light"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = Starbow::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
