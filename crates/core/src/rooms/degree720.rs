//! The 720 Degree Room: Dirac's belt, and the trick a player can actually do.
//!
//! A stone hangs on a belt fixed to a wall. Turn the stone once and the belt is
//! twisted; turn it twice and the belt is twisted twice, which looks worse and
//! is not. A twist of two full turns can be carried off the belt by passing it
//! over the stone, without ever turning the stone again. A twist of one cannot,
//! no matter how many times you try. That difference between one turn and two
//! is Spin(3) double covering SO(3), and it is a thing to do rather than a fact
//! to be told: the room hands a player both hands and lets them find out which
//! twists come off.
//!
//! The picture is the belt seen from the side, drawn as its two edges. Where
//! the belt has turned a quarter way the edges meet, and where it has turned
//! half they have swapped sides, so counting crossings is counting half turns.
//! A flat belt is two parallel lines. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

/// Full turns the dial and the width of the room both reach.
///
/// The room is named for this number. Two turns is the smallest twist the belt
/// trick can take off, so it is the far end of every way of driving the stone.
const MAX_TURNS: f64 = 2.0;

/// How finely a hand can set the spin: one eighth of a turn, forty five degrees.
///
/// Quantized rather than continuous so that landing exactly on two turns is
/// something a player can do rather than something they can approach. An exact
/// landing is what lets the belt go exactly flat and say so.
const HAND_STEP: f64 = 0.125;

/// How many times one hand can carry the belt over the stone.
const MAX_LOOPS: u32 = 2;

/// Turns one pass of the belt over the stone removes.
///
/// Two, and this is the whole room. It is why an odd number of turns can never
/// be cleared and an even number always can.
const TURNS_PER_LOOP: f64 = 2.0;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 }
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

/// What the stone and the belt are doing: full turns given, passes made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Belt {
    /// Full turns the stone has been given.
    pub turns: f64,
    /// Times the belt has been carried over the stone.
    pub loops: u32,
}

impl Belt {
    /// Twist left in the belt, in full turns. Sign is which way it winds.
    ///
    /// Every pass takes off two turns and turns the stone not at all, so this
    /// is the one number the trick can change and the stone's own angle is not.
    #[must_use]
    pub fn twist(self) -> f64 {
        self.turns - TURNS_PER_LOOP * f64::from(self.loops)
    }

    /// Whether the belt hangs without a twist in it.
    #[must_use]
    pub fn is_flat(self) -> bool {
        self.twist().abs() < HAND_STEP / 2.0
    }
}

/// Read the stone and the belt from the dial, or from the hand when there is one.
///
/// With no hand the dial walks the stone through both turns, so a player who
/// only scrubs still watches the belt braid up. A hand takes both: across is the
/// spin, and lifting toward the top of the room carries the belt over the stone.
fn belt_from(t: f64, pokes: &[(f64, f64)], seed: u64) -> Belt {
    let hands = finite_pokes(pokes);
    let Some(&(x, y)) = hands.last() else {
        let drift = if seed == 0 {
            0.0
        } else {
            (seed % 8) as f64 * 0.05
        };
        return Belt {
            turns: (phase_unit(t) * MAX_TURNS + drift).min(MAX_TURNS),
            loops: 0,
        };
    };
    let turns = ((x * MAX_TURNS / HAND_STEP).round() * HAND_STEP).clamp(0.0, MAX_TURNS);
    // The bottom half of the room is the belt left alone. Lifting above the
    // middle carries it over the stone, once and then twice.
    let lift = ((0.5 - y).max(0.0)) / 0.5;
    let loops = (lift * f64::from(MAX_LOOPS)).ceil() as u32;
    Belt {
        turns,
        loops: loops.min(MAX_LOOPS),
    }
}

/// Draw the wall, the belt with the twist that is left in it, and the stone.
fn draw_belt(canvas: &mut dyn Surface, belt: Belt) {
    let (width, height) = canvas.draw_bounds();
    if width < 12 || height < 7 {
        return;
    }
    let aspect = canvas.char_aspect().max(0.1);
    let cy = height as f64 / 2.0;
    let wall = (width as f64 * 0.07).round();
    let stone_r = (width.min(height) as f64 * 0.10).max(2.0);
    let stone_x = width as f64 * 0.84 - stone_r;
    let span = (stone_x - wall).max(1.0);
    let half = (height as f64 * 0.24).max(1.0);

    // The wall. Without something the belt cannot turn with, a twist is not a
    // quantity and none of this counts.
    canvas.line(
        wall as i32,
        (cy - half * 1.5) as i32,
        wall as i32,
        (cy + half * 1.5) as i32,
        '|',
    );

    // The belt, drawn as its two edges. A flat ribbon seen from the side shows
    // its edges apart; a quarter turn on and it is edge on and they meet; a
    // half turn on and they have swapped. The crossings are the half turns, so
    // the twist is in the picture and not only on the scoreboard.
    let twist = belt.twist();
    let steps = (span.round() as usize).clamp(24, 400);
    let mut previous: Option<(i32, i32, i32)> = None;
    for step in 0..=steps {
        let along = step as f64 / steps as f64;
        let turned = twist * TAU * along;
        let x = (wall + span * along).round() as i32;
        let offset = half * turned.cos();
        let top = (cy - offset).round() as i32;
        let bottom = (cy + offset).round() as i32;
        if let Some((previous_x, previous_top, previous_bottom)) = previous {
            canvas.line(previous_x, previous_top, x, top, '*');
            canvas.line(previous_x, previous_bottom, x, bottom, '*');
        }
        previous = Some((x, top, bottom));
    }

    // The stone, with a mark on it so its own turning is visible. The mark
    // comes back to where it started every full turn, which is the half of this
    // room that is easy: the stone always returns. The belt is the other half.
    let ring = ((stone_r * 8.0) as usize).max(24);
    for step in 0..ring {
        let angle = TAU * step as f64 / ring as f64;
        let x = (stone_x + stone_r * angle.cos()).round() as i32;
        let y = (cy + stone_r * angle.sin() * aspect).round() as i32;
        canvas.plot(x, y, 'O');
    }
    let facing = belt.turns * TAU;
    canvas.line(
        stone_x.round() as i32,
        cy.round() as i32,
        (stone_x + stone_r * facing.cos()).round() as i32,
        (cy + stone_r * facing.sin() * aspect).round() as i32,
        '#',
    );
}

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

    fn readout(&self, belt: Belt, handled: bool) -> String {
        let state = if belt.is_flat() {
            "FLAT".to_string()
        } else {
            format!("TWIST {:+.2}", belt.twist())
        };
        if handled {
            format!("TURNS {:.2}  LOOPS {}  {state}", belt.turns, belt.loops)
        } else {
            format!("TURNS {:.2}  {state}  DRAG: SPIN AND LIFT", belt.turns)
        }
    }
}

impl Room for Degree720 {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw_belt(canvas, belt_from(t, &[], self.seed));
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
        Some("DRAG: ACROSS SPINS THE STONE, UP LIFTS THE BELT OVER")
    }

    fn goal(&self) -> Option<&'static str> {
        Some("HANG THE BELT FLAT AFTER TURNING THE STONE")
    }

    fn goal_met(&self, t: f64, inputs: &[RoomInput]) -> bool {
        let pokes = crate::pokes_from_inputs(inputs);
        let belt = belt_from(t, &pokes, self.seed);
        belt.turns >= 1.0 && belt.is_flat()
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.readout(belt_from(t, &[], self.seed), false))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw_belt(canvas, belt_from(t, pokes, self.seed));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        if finite_pokes(&pokes).is_empty() {
            return self.status(t);
        }
        Some(self.readout(belt_from(t, &pokes, self.seed), true))
    }

    fn reveal(&self) -> &'static str {
        "Turning the stone once leaves a twist no amount of carrying the belt \
         around can remove. Turning it twice leaves a twist one pass takes off \
         completely, and the stone never moves while you do it, so the two \
         twists were never the same kind of thing. Rotations in 3D form SO(3), \
         and spinors live on its double cover Spin(3), isomorphic to SU(2), \
         where a path of 720 degrees closes and a path of 360 does not. That is \
         why an electron needs two turns to look like itself again, and it is \
         what your hands just did."
    }
}

#[cfg(test)]
mod tests {
    use super::{Belt, Degree720, MAX_LOOPS, MAX_TURNS, belt_from};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    fn hand(x: f64, y: f64) -> [RoomInput; 1] {
        [RoomInput::PointerDown { x, y, t: 0.0 }]
    }

    #[test]
    fn status_invites_spin() {
        let s = Degree720::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("SPIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn poke_changes_status() {
        let r = Degree720::new();
        let open = r.status(0.5).unwrap();
        let after = r.status_input(0.5, &hand(0.8, 0.2)).unwrap();
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
    fn the_dial_visibly_braids_the_belt() {
        // The room was called mute by a packaged playtest: "Status goes TWIST
        // to FREE. The picture barely turns." A room about turning has to turn.
        let room = Degree720::new();
        let mut frames = std::collections::HashSet::new();
        for step in 0..=16 {
            let mut canvas = Canvas::new(96, 40);
            room.render(&mut canvas, f64::from(step) / 16.0);
            frames.insert(canvas.to_text());
        }
        assert!(
            frames.len() >= 12,
            "seventeen turns of the dial drew only {} pictures",
            frames.len()
        );
    }

    #[test]
    fn two_turns_come_off_and_one_never_does() {
        // The room, in one test. A pass of the belt over the stone removes two
        // turns and turns the stone not at all, so an even twist can always be
        // cleared and an odd one can never be, however many passes you make.
        let two = Belt {
            turns: 2.0,
            loops: 1,
        };
        assert!(two.is_flat(), "two turns must come off in one pass");
        assert_eq!(two.turns, 2.0, "the trick must not turn the stone");
        for loops in 0..=MAX_LOOPS {
            let one = Belt { turns: 1.0, loops };
            assert!(
                !one.is_flat(),
                "one turn came off after {loops} passes, which would collapse \
                 the double cover"
            );
        }
    }

    #[test]
    fn a_hand_can_actually_do_the_trick() {
        // A fact nobody can reach is a fact nobody has. Turning the stone all
        // the way across and lifting the belt above the middle has to leave a
        // flat belt and meet the goal, from real pointer input.
        let room = Degree720::new();
        let done = hand(1.0, 0.4);
        let belt = belt_from(0.0, &[(1.0, 0.4)], 0);
        assert_eq!(belt.turns, MAX_TURNS);
        assert_eq!(belt.loops, 1);
        assert!(belt.is_flat());
        assert!(room.goal_met(0.0, &done));
        assert!(room.status_input(0.0, &done).unwrap().contains("FLAT"));

        // The same lift after a single turn leaves the belt twisted, and the
        // goal unmet, which is the discovery the room exists for.
        let half = hand(0.5, 0.4);
        assert!(!room.goal_met(0.0, &half));
        assert!(room.status_input(0.0, &half).unwrap().contains("TWIST"));

        // And leaving the belt alone never flattens a turned stone.
        assert_eq!(belt_from(0.0, &[(1.0, 0.9)], 0).loops, 0);
        assert!(!room.goal_met(0.0, &hand(1.0, 0.9)));
    }

    #[test]
    fn a_pass_of_the_belt_changes_the_picture_without_turning_the_stone() {
        let room = Degree720::new();
        let mut twisted = Canvas::new(96, 40);
        let mut cleared = Canvas::new(96, 40);
        room.render_poked(&mut twisted, 0.0, &[(1.0, 0.9)]);
        room.render_poked(&mut cleared, 0.0, &[(1.0, 0.4)]);
        assert_ne!(
            twisted.to_text(),
            cleared.to_text(),
            "carrying the belt over the stone has to show"
        );
    }

    #[test]
    fn hand_spin_moves_the_belt() {
        let r = Degree720::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        r.render(&mut base, 0.5);
        r.render_poked(&mut poked, 0.5, &[(0.8, 0.9)]);
        assert_ne!(base.to_text(), poked.to_text(), "hand must rotate the stone");
    }

    #[test]
    fn motif_ok() {
        assert!(Degree720::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn extreme_ok() {
        let mut c = Canvas::new(4, 4);
        Degree720::new().render(&mut c, f64::NAN);
        let mut big = Canvas::new(96, 40);
        Degree720::new().render_poked(&mut big, f64::INFINITY, &[(f64::NAN, f64::NAN)]);
    }
}

