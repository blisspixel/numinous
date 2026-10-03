//! The presented hand: a room's dial follows the pointer on a damped spring.
//!
//! The accepted gesture history, `App.inputs`, is the truth. Replay, MCP
//! parity, postcards, goals, the staged experiments, and the Watch Agent all
//! read it exactly as the hand wrote it. Only the frames the App presents read
//! a copy in which the open gesture's newest point trails the hand on a
//! critically damped [`Spring`], so a dial glides between the points the
//! pointer reports instead of stepping from one to the next. A press lands
//! where the hand lands: easing a dial in from wherever it was would show
//! values the hand never chose. At rest the copy equals the history, so a
//! settled frame is the frame the raw input draws.

use numinous_core::{Motion, RoomInput, Spring};

/// Stiffness of the hand spring, in radians per second.
///
/// Half of a step in about 56 ms and all but one percent in 0.22 s. A steady
/// drag is trailed by `2 / w`, 67 ms, close to the 40 ms smoothing the
/// parameter voice already applies to the same values, so sight and sound
/// move together, the sound at most about 30 ms ahead.
const HAND_STIFFNESS: f64 = 30.0;

/// Points are normalized to the window, so a ten-thousandth is under half a
/// pixel even across a 4K window.
const HAND_TOLERANCE: f64 = 1e-4;

/// The spring the presented hand rides on, one per axis.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HandSpring {
    x: Spring,
    y: Spring,
    /// The newest point of the open gesture when the spring last followed it.
    written: Option<(f64, f64)>,
}

impl Default for HandSpring {
    fn default() -> Self {
        Self {
            x: Spring::new(HAND_STIFFNESS, HAND_TOLERANCE),
            y: Spring::new(HAND_STIFFNESS, HAND_TOLERANCE),
            written: None,
        }
    }
}

impl HandSpring {
    /// Follow the hand the accepted history now holds.
    ///
    /// `hand` is `None` when no gesture is open or the room does not ease its
    /// hand, and the spring then rests. A new gesture starts at rest on its
    /// press; each later point of the same gesture moves the target and leaves
    /// the presented point where it was.
    pub(crate) fn follow(&mut self, hand: Option<(f64, f64)>, motion: Motion) {
        match (self.written, hand) {
            (Some(from), Some(to)) => {
                self.x.retarget(from.0, to.0, motion);
                self.y.retarget(from.1, to.1, motion);
            }
            _ => {
                self.x.settle();
                self.y.settle();
            }
        }
        self.written = hand;
    }

    /// Let `seconds` of presentation time pass.
    pub(crate) fn advance(&mut self, seconds: f64) {
        self.x.advance(seconds);
        self.y.advance(seconds);
    }

    /// Whether the presented hand is the accepted hand.
    pub(crate) fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }

    /// The accepted history as it should be drawn: identical at rest, and
    /// otherwise with the open gesture's newest point where the spring
    /// presents it.
    pub(crate) fn present(&self, inputs: &[RoomInput]) -> Vec<RoomInput> {
        let mut presented = inputs.to_vec();
        if self.is_settled() {
            return presented;
        }
        if let Some((index, _)) = open_hand(inputs)
            && let Some(RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. }) =
                presented.get_mut(index)
        {
            *x = self.x.present(*x).clamp(0.0, 1.0);
            *y = self.y.present(*y).clamp(0.0, 1.0);
        }
        presented
    }
}

/// The newest point of the open gesture, with its place in the history.
///
/// A lift or a cancel closes the gesture, so a history that ends in one has no
/// open hand. Wheel steps and keys inside a gesture do not move it.
pub(crate) fn open_hand(inputs: &[RoomInput]) -> Option<(usize, (f64, f64))> {
    for (index, input) in inputs.iter().enumerate().rev() {
        match *input {
            RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                return Some((index, (x, y)));
            }
            RoomInput::PointerUp { .. } | RoomInput::PointerCancel => return None,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{HandSpring, open_hand};
    use numinous_core::{Motion, RoomInput};

    fn down(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerDown { x, y, t: 0.0 }
    }

    fn moved(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerMove { x, y, t: 0.1 }
    }

    fn newest(inputs: &[RoomInput]) -> (f64, f64) {
        open_hand(inputs).expect("an open gesture").1
    }

    #[test]
    fn the_open_hand_is_the_newest_point_of_an_unfinished_gesture() {
        assert_eq!(open_hand(&[]), None);
        let open = [
            down(0.1, 0.2),
            moved(0.3, 0.4),
            RoomInput::Wheel { delta: 1.0 },
        ];
        assert_eq!(open_hand(&open), Some((1, (0.3, 0.4))));
        let lifted = [
            down(0.1, 0.2),
            RoomInput::PointerUp {
                x: 0.1,
                y: 0.2,
                t: 0.1,
            },
        ];
        assert_eq!(open_hand(&lifted), None);
        assert_eq!(open_hand(&[down(0.5, 0.5), RoomInput::PointerCancel]), None);
    }

    #[test]
    fn a_press_lands_where_the_hand_lands() {
        let mut hand = HandSpring::default();
        let inputs = [down(0.7, 0.2)];
        hand.follow(Some(newest(&inputs)), Motion::Full);
        assert!(hand.is_settled());
        assert_eq!(hand.present(&inputs), inputs);
    }

    #[test]
    fn a_drag_glides_toward_the_hand_and_arrives_exactly() {
        let mut hand = HandSpring::default();
        let mut inputs = vec![down(0.2, 0.5)];
        hand.follow(Some(newest(&inputs)), Motion::Full);
        inputs.push(moved(0.6, 0.5));
        hand.follow(Some(newest(&inputs)), Motion::Full);
        // The presented point has not moved yet; the accepted one has.
        assert_eq!(newest(&hand.present(&inputs)), (0.2, 0.5));
        hand.advance(1.0 / 60.0);
        let (x, y) = newest(&hand.present(&inputs));
        assert!(x > 0.2 && x < 0.6, "between the points, not past them: {x}");
        assert_eq!(y, 0.5, "an axis the hand did not move stays exact");
        for _ in 0..60 {
            hand.advance(1.0 / 60.0);
        }
        assert!(hand.is_settled());
        assert_eq!(hand.present(&inputs), inputs, "at rest the copy is exact");
    }

    #[test]
    fn only_the_newest_point_is_presented_differently() {
        let mut hand = HandSpring::default();
        let mut inputs = vec![down(0.2, 0.5), moved(0.3, 0.5)];
        hand.follow(Some(newest(&inputs)), Motion::Full);
        inputs.push(moved(0.5, 0.6));
        hand.follow(Some(newest(&inputs)), Motion::Full);
        hand.advance(0.01);
        let presented = hand.present(&inputs);
        assert_eq!(presented[..2], inputs[..2]);
        assert_ne!(presented[2], inputs[2]);
        assert!(matches!(presented[2], RoomInput::PointerMove { t, .. } if t == 0.1));
    }

    #[test]
    fn reduced_motion_presents_every_point_where_the_hand_put_it() {
        let mut hand = HandSpring::default();
        let mut inputs = vec![down(0.2, 0.5)];
        hand.follow(Some(newest(&inputs)), Motion::Reduced);
        inputs.push(moved(0.9, 0.1));
        hand.follow(Some(newest(&inputs)), Motion::Reduced);
        assert!(hand.is_settled());
        assert_eq!(hand.present(&inputs), inputs);
    }

    #[test]
    fn a_lift_or_a_room_that_does_not_ease_rests_the_spring() {
        let mut hand = HandSpring::default();
        hand.follow(Some((0.1, 0.1)), Motion::Full);
        hand.follow(Some((0.9, 0.9)), Motion::Full);
        assert!(!hand.is_settled());
        hand.follow(None, Motion::Full);
        assert!(hand.is_settled());
        // The next press starts fresh rather than gliding in from the old one.
        hand.follow(Some((0.4, 0.4)), Motion::Full);
        assert!(hand.is_settled());
    }
}
