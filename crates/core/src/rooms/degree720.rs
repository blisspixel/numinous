//! The 720 Degree Room: Dirac's belt, the quaternion double cover.
//!
//! A tethered stone on a ribbon needs two full turns (720 degrees) to untwist,
//! not one. That is Spin(3) covering SO(3): 360 degrees is not enough. DRAG
//! rotates the stone; status counts full turns. See `docs/ROOMS.md`.

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

/// Angle from hand or ambient sweep; returns radians and twist 0..2.
fn angle_from(t: f64, pokes: &[(f64, f64)], seed: u64) -> (f64, f64) {
    let base = phase_unit(t) * TAU * 2.0; // ambient can show full 720 path
    let hands = finite_pokes(pokes);
    let ang = if let Some(&(x, y)) = hands.last() {
        // Hand owns the double cover: x sweeps 0..720deg, y fine-tunes a half turn.
        // Pure atan2 + ambient phase could land on the same angle as idle t, a dead dial.
        let coarse = x.clamp(0.0, 1.0) * TAU * 2.0;
        let fine = (y.clamp(0.0, 1.0) - 0.5) * TAU;
        (coarse + fine).rem_euclid(TAU * 2.0)
    } else {
        base + if seed == 0 {
            0.0
        } else {
            ((seed % 8) as f64) * 0.2
        }
    };
    let twist = (ang / TAU).rem_euclid(2.0); // 0..2 full turns
    (ang, twist)
}

fn draw_belt(canvas: &mut dyn Surface, ang: f64, twist: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = width.min(height) as f64 * 0.34;
    // Always draw a full double-cover guide so t=0 is a belt, not a speck.
    let guide_steps = 96;
    let guide_span = TAU * 2.0;
    let mut prev_g: Option<(i32, i32)> = None;
    for i in 0..=guide_steps {
        let u = i as f64 / guide_steps as f64;
        let a = guide_span * u;
        let half = 2.0 * PI_F * u;
        let rr = r * (0.5 + 0.35 * (half * 2.0).cos().abs());
        let x = (cx + rr * a.cos()).round() as i32;
        let y = (cy + rr * a.sin() * 0.72).round() as i32;
        if let Some((ox, oy)) = prev_g {
            canvas.line(ox, oy, x, y, '.');
        }
        prev_g = Some((x, y));
    }
    // Live ribbon follows the stone angle (thick so it reads on a large window).
    let span = ang.abs().max(0.4);
    let n = 72;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=n {
        let u = i as f64 / n as f64;
        let a = if ang >= 0.0 { span * u } else { -span * u };
        let half = twist * PI_F * u;
        let rr = r * (0.55 + 0.45 * (half * 2.0).cos().abs());
        let x = (cx + rr * a.cos()).round() as i32;
        let y = (cy + rr * a.sin() * 0.72).round() as i32;
        let mark = if i == n { '#' } else { '*' };
        canvas.plot(x, y, mark);
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '*');
        }
        prev = Some((x, y));
    }
    // Anchor hub.
    for dy in -1..=1 {
        for dx in -1..=1 {
            canvas.plot(cx.round() as i32 + dx, cy.round() as i32 + dy, '+');
        }
    }
    // Stone at tip.
    let tip_a = if ang.abs() < 1e-9 { 0.0 } else { ang };
    let sx = cx + r * tip_a.cos();
    let sy = cy + r * tip_a.sin() * 0.72;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(sx.round() as i32 + dx, sy.round() as i32 + dy, 'O');
            }
        }
    }
}

const PI_F: f64 = std::f64::consts::PI;

/// The 720 Degree Room.
#[derive(Debug, Default)]
pub struct Degree720 {
    seed: u64,
}

impl Degree720 {
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
}

impl Room for Degree720 {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "degree-720",
            title: "The 720 Degree Room",
            wing: "Shape & Space",
            blurb: "A tethered stone needs two full turns to untwist the belt: 360 is not enough, \
                    720 is. Dirac's belt trick; the quaternion double cover of rotations. t spins; \
                    DRAG rotates the stone.",
            accent: [160, 120, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (ang, twist) = angle_from(t, &[], self.seed);
        draw_belt(canvas, ang, twist);
    }

    fn postcard_t(&self) -> f64 {
        0.9
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "double cover",
            root: 196.0,
            tempo: 96,
            line: &[0, 7, 12, 7, 0, 7, 12, 0],
            encodes: "one turn still twisted, two turns free",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ROTATE THE STONE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (ang, twist) = angle_from(t, &[], self.seed);
        let deg = ang.to_degrees().rem_euclid(720.0);
        let free = if twist >= 1.95 { "FREE" } else { "TWIST" };
        Some(format!("{deg:.0}deg  turns={twist:.2}  {free}  DRAG:SPIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let (ang, twist) = angle_from(t, pokes, self.seed);
        draw_belt(canvas, ang, twist);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        if finite_pokes(&pokes).is_empty() {
            return self.status(t);
        }
        let (ang, twist) = angle_from(t, &pokes, self.seed);
        let deg = ang.to_degrees().rem_euclid(720.0);
        let free = if twist >= 1.95 {
            "UNTWISTED"
        } else if twist >= 0.95 {
            "HALF"
        } else {
            "TWIST"
        };
        Some(format!("SPIN {deg:.0}deg  t={twist:.2}  {free}"))
    }

    fn reveal(&self) -> &'static str {
        "Rotate an object 360 degrees and a belt from ceiling to object stays \
         twisted; another 360 degrees undoes the twist. Rotations in 3D form SO(3); \
         spinors live on the double cover Spin(3)≅SU(2). That is why electrons \
         need 720 degrees to look the same."
    }
}

#[cfg(test)]
mod tests {
    use super::Degree720;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites_spin() {
        let s = Degree720::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("SPIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn poke_changes_status() {
        let r = Degree720::new();
        let open = r.status(0.0).unwrap();
        let after = r
            .status_input(
                0.5,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(open, after);
        assert!(after.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Degree720::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn hand_spin_moves_the_belt() {
        let r = Degree720::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        r.render(&mut base, 0.5);
        r.render_poked(&mut poked, 0.5, &[(0.8, 0.5)]);
        assert_ne!(
            base.to_text(),
            poked.to_text(),
            "hand must rotate the stone"
        );
    }

    #[test]
    fn motif_ok() {
        assert!(Degree720::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn extreme_ok() {
        let mut c = Canvas::new(4, 4);
        Degree720::new().render(&mut c, f64::NAN);
    }
}
