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
///
/// One. The stone turns at most twice, so a second pass can never help: it can
/// only add twist back. Offering two made the winning corner of the room a
/// narrow band a player had to find by accident, and bought nothing.
const MAX_LOOPS: u32 = 1;

/// Turns one pass of the belt over the stone removes.
///
/// Two, and this is the whole room. It is why an odd number of turns can never
/// be cleared and an even number always can.
const TURNS_PER_LOOP: f64 = 2.0;

/// The stretch of the room a hand turns the stone across, as fractions of the
/// width: from just off the wall to where the stone hangs.
///
/// Two full turns land on the stone, which is a place a player can see and put
/// a hand. The mapping used to run the whole width, and because an exact
/// landing is quantized to [`HAND_STEP`], the only hand position that read as
/// two turns was the outermost thirty-second of the window. A packaged
/// playtest dragged across onto the stone, read `TURNS 1.62`, and reported the
/// advertised trick as a near miss it could not finish.
const SPIN_FROM: f64 = 0.10;
/// The far end of that stretch. See [`SPIN_FROM`].
const SPIN_TO: f64 = 0.80;

/// How far upward a multi-point gesture must travel to carry the belt over.
///
/// A horizontal drag may sit above the middle of the room without being a
/// lift. Requiring visible upward travel keeps ACROSS and UP as different
/// actions while the one-point stone shortcut remains available.
const MIN_LIFT: f64 = 0.20;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 }
}

/// Every point of the hand's gesture, including the one where it let go.
///
/// The shared poke reading drops release points, because in most rooms a lift
/// paints nothing. Here it is half the trick: the belt is carried over the
/// stone by a hand that goes up and lets go at the top, and a two-point drag
/// (down low, up high) is the plainest way to ask for that. Reading only the
/// painted points made that gesture register as a touch that never rose, so
/// the room heard a spin where a player had done a lift.
fn hand_points(inputs: &[RoomInput]) -> Vec<(f64, f64)> {
    inputs
        .iter()
        .filter_map(|input| match *input {
            RoomInput::PointerDown { x, y, .. }
            | RoomInput::PointerMove { x, y, .. }
            | RoomInput::PointerUp { x, y, .. } => Some((x, y)),
            _ => None,
        })
        .collect()
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

/// The spin a hand at this column has given the stone, in full turns.
///
/// Quantized to [`HAND_STEP`] so an exact landing on two turns is something a
/// player can do rather than approach, and measured across [`SPIN_FROM`] to
/// [`SPIN_TO`] so that landing sits on the stone.
fn spin_at(x: f64) -> f64 {
    let along = ((x - SPIN_FROM) / (SPIN_TO - SPIN_FROM)).clamp(0.0, 1.0);
    ((along * MAX_TURNS / HAND_STEP).round() * HAND_STEP).clamp(0.0, MAX_TURNS)
}

/// Read a hand path as a stone spin and, when present, one upward pass.
fn hand_belt(hands: &[(f64, f64)]) -> Belt {
    let [(x, y)] = hands else {
        let mut lowest_y = hands[0].1;
        let mut spin_x = hands[0].0;
        let mut lifted = false;
        for &(x, y) in &hands[1..] {
            if !lifted && lowest_y - y >= MIN_LIFT && y < 0.5 {
                lifted = true;
            }
            if !lifted {
                if y >= lowest_y {
                    lowest_y = y;
                    spin_x = x;
                } else if lowest_y - y < MIN_LIFT {
                    // Before a meaningful rise begins, the newest horizontal
                    // position is still where the stone was turned.
                    spin_x = x;
                }
            }
        }
        if !lifted {
            spin_x = hands.last().map_or(spin_x, |&(x, _)| x);
        }
        return Belt {
            turns: spin_at(spin_x),
            loops: u32::from(lifted),
        };
    };

    Belt {
        turns: spin_at(*x),
        // A poke on the visible stone remains the compact form of the whole
        // trick. Multi-point gestures have to earn OVER through upward travel.
        loops: u32::from(*y < 0.5),
    }
}

/// Read the stone and the belt from the dial, or from the hand when there is one.
///
/// With no hand the dial walks the stone through both turns, so a player who
/// only scrubs still watches the belt braid up. A hand takes both: across is the
/// spin, and lifting toward the top of the room carries the belt over the stone.
fn belt_from(t: f64, pokes: &[(f64, f64)], seed: u64) -> Belt {
    let hands = finite_pokes(pokes);
    let Some(_) = hands.first() else {
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
    let mut belt = hand_belt(&hands);
    belt.loops = belt.loops.min(MAX_LOOPS);
    belt
}

/// The belt that is hanging now.
const BELT: char = '*';

/// The twist a pass took off, kept in the frame beside the belt.
///
/// Both marks paint the room's plain accent, so the two readings separate by
/// glyph and never by color alone.
const GHOST: char = '.';

/// Draw one ribbon's two edges across the span, given the twist along it.
fn draw_edges(
    canvas: &mut dyn Surface,
    wall: f64,
    cy: f64,
    span: f64,
    half: f64,
    twist: f64,
    mark: char,
) {
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
            canvas.line(previous_x, previous_top, x, top, mark);
            canvas.line(previous_x, previous_bottom, x, bottom, mark);
        }
        previous = Some((x, top, bottom));
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
    //
    // Once the belt has been carried over the stone it is drawn twice: the
    // twist that was there before the pass, in the fainter mark, and the twist
    // that is there now. A packaged playtest did the trick and saw nothing,
    // because it is true that two turns come off and leave a flat belt, and it
    // is also true that a flat belt is what the room starts as. Ending where
    // you began is the whole point and it is invisible from the endpoint
    // alone, so the room keeps the braid a player cleared in the frame beside
    // the belt that is hanging now. From one turn the two lie on top of each
    // other, which is the other half of the same fact.
    if belt.loops > 0 {
        draw_edges(canvas, wall, cy, span, half, belt.turns, GHOST);
    }
    draw_edges(canvas, wall, cy, span, half, belt.twist(), BELT);

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
            // A pass is named rather than counted. `LOOPS 1` read as a tally a
            // player had to interpret, and a packaged playtest could not tell a
            // lift that registered from one that did nothing. `OVER` says the
            // belt went over the stone, which is what the hand did, and it says
            // nothing about what that is worth.
            let carried = if belt.loops > 0 { "OVER  " } else { "" };
            format!("TURNS {:.2}  {carried}{state}", belt.turns)
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
        let belt = belt_from(t, &hand_points(inputs), self.seed);
        belt.turns >= 1.0 && belt.is_flat()
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.readout(belt_from(t, &[], self.seed), false))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw_belt(canvas, belt_from(t, pokes, self.seed));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let hands = hand_points(inputs);
        if finite_pokes(&hands).is_empty() {
            return self.status(t);
        }
        Some(self.readout(belt_from(t, &hands, self.seed), true))
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        // The picture reads the same gesture the status does, release included,
        // so a lift that the scoreboard reports is a lift the belt shows.
        draw_belt(canvas, belt_from(t, &hand_points(inputs), self.seed));
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
    use super::{Belt, Degree720, MAX_LOOPS, MAX_TURNS, SPIN_FROM, SPIN_TO, belt_from, spin_at};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    /// The column that turns the stone exactly once. Midway across the stretch
    /// the hand spins over, which is where one turn has to be.
    const ONE_TURN_X: f64 = (SPIN_FROM + SPIN_TO) / 2.0;

    fn hand(x: f64, y: f64) -> [RoomInput; 1] {
        [RoomInput::PointerDown { x, y, t: 0.0 }]
    }

    /// A two-point drag: the hand lands, then lets go somewhere else.
    fn drag(from: (f64, f64), to: (f64, f64)) -> [RoomInput; 2] {
        [
            RoomInput::PointerDown {
                x: from.0,
                y: from.1,
                t: 0.0,
            },
            RoomInput::PointerUp {
                x: to.0,
                y: to.1,
                t: 0.0,
            },
        ]
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
    fn the_trick_survives_being_done_as_one_gesture() {
        // A packaged playtest spun the stone, lifted the belt, and could not
        // flatten it. Both the spin and the lift were read off the newest
        // point, so lifting straight up overwrote the spin with its own x and
        // the two halves of the trick could never be held at once. The room
        // advertised a thing a player could not do.
        let flatten = [(0.10, 0.80), (0.98, 0.80), (0.98, 0.30)];
        let belt = belt_from(0.0, &flatten, 0);
        assert_eq!(belt.turns, MAX_TURNS, "the spin has to survive the lift");
        assert_eq!(belt.loops, 1);
        assert!(belt.is_flat(), "spin then lift has to hang the belt flat");

        // The same gesture from one turn cannot flatten, however it is drawn.
        // This is the fact the room exists for, so it has to hold on the path a
        // hand actually takes and not only on a single tap.
        let odd = [(0.10, 0.80), (ONE_TURN_X, 0.80), (ONE_TURN_X, 0.30)];
        let belt = belt_from(0.0, &odd, 0);
        assert_eq!(belt.turns, 1.0);
        assert!(!belt.is_flat());

        // Carrying the belt over is something the hand did, not somewhere the
        // hand is, so bringing it back down afterwards does not undo the pass.
        let returned = [(0.10, 0.80), (0.98, 0.80), (0.98, 0.20), (0.98, 0.80)];
        assert!(belt_from(0.0, &returned, 0).is_flat());
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
    fn winning_does_not_look_like_never_having_played() {
        // A packaged playtest did the trick and reported an empty win: "0 of
        // 2304 cells answered", the ASCII the same as the untouched room. It
        // was right, and the room was right too. Two turns come off and leave
        // a flat belt, and a flat belt is what the room starts as, so the
        // endpoint of the trick is the picture of nobody having done it. That
        // is the whole point and it cannot be seen from the endpoint alone.
        let room = Degree720::new();
        let mut untouched = Canvas::new(96, 40);
        let mut won = Canvas::new(96, 40);
        room.render(&mut untouched, 0.0);
        room.render_poked(&mut won, 0.0, &[(SPIN_TO, 0.45)]);
        assert!(room.goal_met(0.0, &hand(SPIN_TO, 0.45)));
        assert_ne!(
            untouched.to_text(),
            won.to_text(),
            "a finished trick has to look different from an untouched room"
        );
        assert!(
            won.to_text().contains(super::GHOST),
            "the twist that came off is what makes the win visible"
        );

        // The same lift from one turn takes nothing off, so the belt that
        // hangs now covers the belt that hung before and the frame says so.
        let mut refused = Canvas::new(96, 40);
        room.render_poked(&mut refused, 0.0, &[(ONE_TURN_X, 0.45)]);
        assert!(!room.goal_met(0.0, &hand(ONE_TURN_X, 0.45)));
        assert!(
            !refused.to_text().contains(super::GHOST),
            "from one turn a pass changes nothing, and the picture must agree"
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
    fn the_hand_lets_go_at_the_top_and_the_belt_goes_with_it() {
        // A packaged playtest asked for the trick the plainest way there is:
        // put a hand on the belt, carry it up, let go above the stone. The room
        // read only the points a drag paints, so the release at the top was
        // never seen, the lift never happened, and the scoreboard reported a
        // spin the player had not asked for. Two points are a gesture.
        let room = Degree720::new();
        let lifted = drag((0.90, 0.85), (0.90, 0.15));
        assert!(
            room.status_input(0.0, &lifted).unwrap().contains("OVER"),
            "letting go above the stone has to carry the belt over it"
        );
        assert!(room.goal_met(0.0, &lifted));

        // And the picture agrees with the scoreboard, which it cannot do while
        // the default render reads a different half of the same gesture.
        let mut held = Canvas::new(96, 40);
        let mut carried = Canvas::new(96, 40);
        room.render_input(&mut held, 0.0, &drag((0.90, 0.85), (0.90, 0.75)));
        room.render_input(&mut carried, 0.0, &lifted);
        assert_ne!(held.to_text(), carried.to_text());
    }

    #[test]
    fn a_lift_that_landed_reads_differently_from_one_that_did_not_happen() {
        // The same tester could not tell a lift that registered from a lift the
        // room ignored: both scoreboards said the same thing. A pass is now
        // named on the line, and naming it says nothing about what it is worth,
        // so the room still keeps the discovery to itself.
        let room = Degree720::new();
        let held = room.status_input(0.0, &hand(ONE_TURN_X, 0.85)).unwrap();
        let carried = room
            .status_input(0.0, &drag((ONE_TURN_X, 0.85), (ONE_TURN_X, 0.15)))
            .unwrap();
        assert!(!held.contains("OVER"));
        assert!(carried.contains("OVER"));
        assert_ne!(held, carried);
        assert!(carried.chars().count() <= 56);

        // Carrying the belt over from one turn is a real pass that leaves the
        // belt twisted. Saying so is the honest half; the reason is the reveal.
        assert!(carried.contains("TWIST"));
        assert!(!room.goal_met(0.0, &drag((ONE_TURN_X, 0.85), (ONE_TURN_X, 0.15))));
    }

    #[test]
    fn two_turns_land_on_the_stone_a_hand_can_reach() {
        // The winning spin used to sit in the outermost thirty-second of the
        // window: a tester who dragged across onto the stone read TURNS 1.62
        // and called the advertised trick a near miss. Two turns now land where
        // the stone is drawn, and the whole stretch stays ordered and exact.
        assert_eq!(spin_at(SPIN_TO), MAX_TURNS);
        assert_eq!(spin_at(0.84), MAX_TURNS, "past the stone is still two turns");
        assert_eq!(spin_at(SPIN_FROM), 0.0);
        assert_eq!(spin_at(0.0), 0.0, "behind the wall is still no turn");
        assert_eq!(spin_at(ONE_TURN_X), 1.0);
        let mut previous = 0.0;
        for step in 0..=100 {
            let turns = spin_at(f64::from(step) / 100.0);
            assert!(turns >= previous, "the stone must not turn backwards");
            previous = turns;
        }

        // The gesture the tester reported as their near miss now finishes.
        let room = Degree720::new();
        let across_then_up = [(0.20, 0.50), (0.80, 0.50), (0.80, 0.10)];
        assert!(belt_from(0.0, &across_then_up, 0).is_flat());
        assert!(room.goal_met(
            0.0,
            &crate::room::inputs_from_pokes(&across_then_up, 0.0)
        ));
    }

    #[test]
    fn horizontal_stone_row_drag_spins_without_claiming_a_lift() {
        // A packaged playtest dragged horizontally onto the visible stone. The
        // old reading treated every point above mid-height as a lift, so this
        // motion said OVER at one quarter turn even though the hand never went
        // up. ACROSS must use the landing column and UP must require a rise.
        let room = Degree720::new();
        let across = drag((0.20, 0.45), (0.85, 0.45));
        let belt = belt_from(0.0, &[(0.20, 0.45), (0.85, 0.45)], 0);
        assert_eq!(belt.turns, MAX_TURNS);
        assert_eq!(belt.loops, 0);
        assert!(!room.goal_met(0.0, &across));
        let status = room.status_input(0.0, &across).unwrap();
        assert!(status.contains("TURNS 2.00"), "{status}");
        assert!(status.contains("TWIST +2.00"), "{status}");
        assert!(!status.contains("OVER"), "{status}");

        // The compact stone poke and the release-inclusive lift stay intact.
        assert!(room.goal_met(0.0, &hand(0.85, 0.45)));
        let across_then_up = [
            RoomInput::PointerDown {
                x: 0.20,
                y: 0.45,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: 0.85,
                y: 0.45,
                t: 0.0,
            },
            RoomInput::PointerUp {
                x: 0.85,
                y: 0.10,
                t: 0.0,
            },
        ];
        assert!(room.goal_met(0.0, &across_then_up));
        assert!(
            room.status_input(0.0, &across_then_up)
                .unwrap()
                .contains("OVER")
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
        let mut big = Canvas::new(96, 40);
        Degree720::new().render_poked(&mut big, f64::INFINITY, &[(f64::NAN, f64::NAN)]);
    }
}
