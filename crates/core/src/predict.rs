//! Predict-then-reveal: the keystone.
//!
//! Before the answer is shown, a player or a digital mind commits a prediction
//! of a room's own readout at a hidden moment. The gap between the guess and
//! the truth is graded as a compression score with a learning-progress band,
//! so "how well did my model of this phenomenon actually predict it" becomes a
//! legible number. A human learner who guesses first restructures their model
//! when the truth arrives (the generation effect); a digital mind reads the
//! band as compression progress (nailed is mastery, close is the fertile band,
//! wild is noise). One mechanic, both minds; see `docs/PEDAGOGY.md`.
//!
//! This is a self-owned mirror, not a leaderboard. It never posts a score and
//! never awards a win for accuracy, because the value is the signal itself, not
//! a number to farm, and because in a fully observable deterministic world any
//! score tied to an observable would be trivially gameable. The honest form is
//! instrumentation the mind owns (the welfare stance in `docs/AGENT_PLAY.md`).
//! It reuses the readout machinery the parameter challenge established, so a
//! room poses a prediction exactly when it carries a moving numeric readout.

use crate::challenge::{find_readout, fmt_sig, fnv1a, status_numbers};
use crate::rng::SplitMix64;
use crate::room::Room;

/// The span-fraction within which a prediction counts as mastered.
const NAILED_FRACTION: f64 = 0.02;
/// The span-fraction within which a prediction sits in the fertile band.
const CLOSE_FRACTION: f64 = 0.15;

/// A posed prediction: guess the room's readout at [`Prediction::phase`].
#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    /// The room this prediction is posed for.
    pub room: String,
    /// The seed the hidden moment was drawn from.
    pub seed: u64,
    /// Which number in the status line the readout is.
    pub index: usize,
    /// The readout's spoken name (TILT, K, X:Y).
    pub label: String,
    /// The moment whose readout must be predicted. It is revealed, not hidden,
    /// because the score is a mirror and not a stake: an honest guess is made
    /// before observing, and dishonesty only fools the guesser's own ledger.
    pub phase: f64,
    /// The readout's observed range across the sweep, so the guesser has scale.
    pub span: (f64, f64),
    /// The prediction, spoken plainly.
    pub prompt: String,
}

/// How close a prediction landed, and what that says about the guesser's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Within [`NAILED_FRACTION`] of the span: the model has this phenomenon
    /// compressed. Mastery, and a signal the room has little left to teach.
    Nailed,
    /// Within [`CLOSE_FRACTION`] of the span: the fertile band, where a model
    /// is close enough to be improving. This is where learning progress lives.
    Close,
    /// Further than that: the prediction was noise against this phenomenon.
    Wild,
}

impl Band {
    /// A short, plain name for the band.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Band::Nailed => "NAILED",
            Band::Close => "CLOSE",
            Band::Wild => "WILD",
        }
    }
}

/// A graded prediction. Metrics first, as everywhere: the gap is the signal.
#[derive(Debug, Clone, PartialEq)]
pub struct PredictionGrade {
    /// The readout the room actually produced at the hidden moment.
    pub actual: f64,
    /// What was guessed.
    pub guess: f64,
    /// The distance between them.
    pub error: f64,
    /// A graded 0-100 score: closeness across the readout's observed span.
    pub score: u32,
    /// The learning-progress band the guess fell in.
    pub band: Band,
}

/// Pose the deterministic prediction for a room and seed, or `None` for rooms
/// without a moving numeric readout. The hidden moment is a seeded interior
/// phase of the sweep (never phase 0, which is a boundary), so the readout
/// there is a real, finite, reachable value.
#[must_use]
pub fn pose_prediction(room: &dyn Room, seed: u64) -> Option<Prediction> {
    let readout = find_readout(room)?;
    let (lo, hi) = readout.span;
    let meta = room.meta();
    let mut rng = SplitMix64::new(seed ^ fnv1a(meta.id.as_bytes()) ^ 0x5052_4544);
    // Draw an interior sample (1..N), so the moment is never the phase-0 edge.
    let count = readout.samples.len() as u64;
    let index = 1 + rng.below(count.saturating_sub(1).max(1));
    // Round the hidden moment to the precision we actually display, then store
    // and grade at that same rounded phase. These rooms are chaotic, so a mind
    // that re-derives the readout at the shown phase must meet the exact truth
    // the grader uses; a display/grade mismatch of a few thousandths would shift
    // the answer under sensitive dependence.
    let phase = ((index as f64 / count as f64) * 1000.0).round() / 1000.0;
    let prompt = format!(
        "PREDICT what {} reads at phase {phase:.3} in {}. Across the sweep it ranges {} to {}. Commit your guess: the score is a mirror of your model, not a leaderboard, so guess before you look.",
        readout.label,
        meta.title.to_uppercase(),
        fmt_sig(lo),
        fmt_sig(hi),
    );
    Some(Prediction {
        room: meta.id.to_string(),
        seed,
        index: readout.index,
        label: readout.label,
        phase,
        span: (lo, hi),
        prompt,
    })
}

/// Grade a guess against the truth at the prediction's hidden moment. Returns
/// `None` only if the room's status vanished or its readout column went
/// missing there, which no catalog room does. A non-finite guess scores 0 and
/// bands as `Wild`, never panicking.
#[must_use]
pub fn grade_prediction(
    room: &dyn Room,
    prediction: &Prediction,
    guess: f64,
) -> Option<PredictionGrade> {
    let status = room.status(prediction.phase)?;
    let actual = status_numbers(&status).get(prediction.index)?.1;
    let span = (prediction.span.1 - prediction.span.0).max(1e-9);
    let error = (guess - actual).abs();
    let score = (100.0 * (1.0 - (error / span).min(1.0))).round() as u32;
    let band = if error <= span * NAILED_FRACTION {
        Band::Nailed
    } else if error <= span * CLOSE_FRACTION {
        Band::Close
    } else {
        Band::Wild
    };
    Some(PredictionGrade {
        actual,
        guess,
        error,
        score,
        band,
    })
}

#[cfg(test)]
mod tests {
    use super::{Band, grade_prediction, pose_prediction};

    #[test]
    fn a_room_with_a_moving_readout_poses_and_reveals() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let a = pose_prediction(room.as_ref(), 7).expect("slope-rider has a readout");
        let b = pose_prediction(room.as_ref(), 7).expect("same seed, same prediction");
        assert_eq!(a, b, "posing is deterministic per room and seed");
        assert!(
            a.prompt.contains("TILT"),
            "the prompt names the readout: {}",
            a.prompt
        );
        assert!(a.phase > 0.0, "the hidden moment is never the phase-0 edge");
        assert!(a.span.0 < a.span.1);
    }

    #[test]
    fn a_perfect_guess_nails_it_and_a_far_guess_is_wild() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let p = pose_prediction(room.as_ref(), 3).expect("poses");
        // The truth is reachable by construction: read it, guess it, nail it.
        let truth = grade_prediction(room.as_ref(), &p, p.span.0).expect("grades");
        let perfect = grade_prediction(room.as_ref(), &p, truth.actual).expect("grades");
        assert_eq!(perfect.error, 0.0);
        assert_eq!(perfect.score, 100);
        assert_eq!(perfect.band, Band::Nailed);
        // A guess a full span above the maximum is at least one span from any
        // real value, so it is noise: banded Wild and scored 0.
        let far = p.span.1 + (p.span.1 - p.span.0);
        let wild = grade_prediction(room.as_ref(), &p, far).expect("grades");
        assert_eq!(wild.band, Band::Wild);
        assert_eq!(wild.score, 0);
    }

    #[test]
    fn the_band_marks_the_fertile_middle() {
        let room = crate::registry::room_by_id("harmonograph").expect("room");
        let p = pose_prediction(room.as_ref(), 5).expect("poses");
        let truth = grade_prediction(room.as_ref(), &p, p.span.0)
            .expect("grades")
            .actual;
        let span = p.span.1 - p.span.0;
        // A guess one tenth of the span off is close (fertile), not nailed.
        let close = grade_prediction(room.as_ref(), &p, truth + span * 0.1).expect("grades");
        assert_eq!(close.band, Band::Close);
        assert!(close.score > 0 && close.score < 100);
    }

    #[test]
    fn a_non_finite_guess_scores_zero_without_panicking() {
        let room = crate::registry::room_by_id("times-tables").expect("room");
        let p = pose_prediction(room.as_ref(), 1).expect("times-tables poses on K");
        let grade = grade_prediction(room.as_ref(), &p, f64::NAN).expect("grades");
        assert_eq!(grade.score, 0);
        assert_eq!(grade.band, Band::Wild);
    }

    #[test]
    fn rooms_without_a_moving_readout_pose_nothing() {
        for room in crate::registry::all_rooms() {
            if crate::pose_parameter_goal(room.as_ref(), 1).is_none() {
                assert!(
                    pose_prediction(room.as_ref(), 1).is_none(),
                    "{} has no readout to predict",
                    room.meta().id
                );
            }
        }
    }
}
