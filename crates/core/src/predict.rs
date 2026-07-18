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

use crate::challenge::{PARAMETER_SAMPLES, find_readout, fmt_sig, fnv1a, status_numbers};
use crate::rng::SplitMix64;
use crate::room::Room;

/// The span-fraction within which a prediction counts as mastered.
const NAILED_FRACTION: f64 = 0.02;
/// The span-fraction within which a prediction sits in the fertile band.
const CLOSE_FRACTION: f64 = 0.15;
/// Samples on each side of the posed moment when a rate model is graded.
const RATE_RADIUS: u64 = 2;
/// The fixed window is small enough to stay local and large enough to expose
/// curvature instead of reducing the model to one secant endpoint.
const RATE_WINDOW_LEN: usize = RATE_RADIUS as usize * 2 + 1;

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
    /// Within `NAILED_FRACTION` of the span: the model has this phenomenon
    /// compressed. Mastery, and a signal the room has little left to teach.
    Nailed,
    /// Within `CLOSE_FRACTION` of the span: the fertile band, where a model
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

/// One point in the revealed shape of a committed linear prediction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionCurveSample {
    /// The phase sampled from the room.
    pub phase: f64,
    /// The room's actual readout at this phase.
    pub actual: f64,
    /// The committed linear model's value at this phase.
    pub predicted: f64,
    /// Actual minus predicted. The sign preserves which side the model missed.
    pub residual: f64,
}

/// Rate and residual feedback for a committed linear prediction.
#[derive(Debug, Clone, PartialEq)]
pub struct PredictionCurveGrade {
    /// The committed slope, in readout units per full phase unit.
    pub rate_guess: f64,
    /// The room's secant slope from the first to last sampled phase.
    pub actual_rate: f64,
    /// Absolute distance between the committed and actual rates.
    pub rate_error: f64,
    /// Mean absolute residual across the five sampled phases.
    pub mean_absolute_residual: f64,
    /// The signed error shape, ordered from earliest to latest phase.
    pub samples: Vec<PredictionCurveSample>,
}

/// Why a rate-and-residual model could not be graded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionCurveError {
    /// The room did not provide the posed numeric readout at every sample.
    ReadoutUnavailable,
    /// Model inputs were non-finite or overflowed while constructing values.
    NonFiniteModel,
    /// Room truth and model values were finite, but derived feedback overflowed.
    NonFiniteFeedback,
    /// The supplied prediction did not describe a valid increasing window.
    InvalidWindow,
}

/// Return the five displayed phases used to grade a prediction's local rate.
///
/// The existing posed phase remains the center, including at the edges of the
/// original seed range. The step contracts near phase 0 or 1 so every sample
/// stays inside the visible sweep without changing established seed meanings.
#[must_use]
pub fn prediction_rate_window(prediction: &Prediction) -> [f64; RATE_WINDOW_LEN] {
    rate_window_for_phase(prediction.phase)
}

fn rate_window_for_phase(phase: f64) -> [f64; RATE_WINDOW_LEN] {
    let edge_limit = 0.999;
    let step = (1.0_f64 / PARAMETER_SAMPLES as f64)
        .min(phase / RATE_RADIUS as f64)
        .min((edge_limit - phase) / RATE_RADIUS as f64);
    std::array::from_fn(|sample| {
        let offset = sample as f64 - RATE_RADIUS as f64;
        ((phase + offset * step) * 1000.0).round() / 1000.0
    })
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
    // Preserve the original seed-to-phase mapping. Stateless clients may pose
    // before an upgrade and grade afterward, so a shared seed must keep naming
    // the same hidden moment across compatible releases.
    let count = readout.samples.len() as u64;
    let index = 1 + rng.below(count.saturating_sub(1).max(1));
    // Round the hidden moment to the precision we actually display, then store
    // and grade at that same rounded phase. These rooms are chaotic, so a mind
    // that re-derives the readout at the shown phase must meet the exact truth
    // the grader uses; a display/grade mismatch of a few thousandths would shift
    // the answer under sensitive dependence.
    let phase = ((index as f64 / count as f64) * 1000.0).round() / 1000.0;
    let rate_window = rate_window_for_phase(phase);
    let prompt = format!(
        "PREDICT what {} reads at phase {phase:.3} in {}, and optionally commit its rate in readout units per phase. Across the sweep it ranges {} to {}. A rate reveals your signed residual shape from phase {:.3} to {:.3}. The score is a mirror of your model, not a leaderboard, so guess before you look.",
        readout.label,
        meta.title.to_uppercase(),
        fmt_sig(lo),
        fmt_sig(hi),
        rate_window[0],
        rate_window[RATE_WINDOW_LEN - 1],
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

/// Reveal how a committed point and rate behave across the posed local window.
///
/// The model is the line `guess + rate * (sample_phase - posed_phase)`. Each
/// residual is actual minus predicted, preserving error direction instead of
/// collapsing the result into another score. The actual rate is the secant
/// slope over the same visible window. Errors distinguish unavailable room
/// truth from invalid windows, invalid model arithmetic, and overflow in the
/// derived feedback itself.
pub fn grade_prediction_curve(
    room: &dyn Room,
    prediction: &Prediction,
    guess: f64,
    rate_guess: f64,
) -> Result<PredictionCurveGrade, PredictionCurveError> {
    if !guess.is_finite() || !rate_guess.is_finite() {
        return Err(PredictionCurveError::NonFiniteModel);
    }
    let rate_window = prediction_rate_window(prediction);
    if !rate_window
        .windows(2)
        .all(|pair| pair[0].is_finite() && pair[0] < pair[1])
        || !rate_window[RATE_WINDOW_LEN - 1].is_finite()
    {
        return Err(PredictionCurveError::InvalidWindow);
    }
    let mut samples = Vec::with_capacity(rate_window.len());
    for phase in rate_window {
        let status = room
            .status(phase)
            .ok_or(PredictionCurveError::ReadoutUnavailable)?;
        let actual = status_numbers(&status)
            .get(prediction.index)
            .ok_or(PredictionCurveError::ReadoutUnavailable)?
            .1;
        let predicted = guess + rate_guess * (phase - prediction.phase);
        let residual = actual - predicted;
        if !actual.is_finite() {
            return Err(PredictionCurveError::ReadoutUnavailable);
        }
        if !predicted.is_finite() {
            return Err(PredictionCurveError::NonFiniteModel);
        }
        if !residual.is_finite() {
            return Err(PredictionCurveError::NonFiniteFeedback);
        }
        samples.push(PredictionCurveSample {
            phase,
            actual,
            predicted,
            residual,
        });
    }
    let first = samples.first().ok_or(PredictionCurveError::InvalidWindow)?;
    let last = samples.last().ok_or(PredictionCurveError::InvalidWindow)?;
    let phase_width = last.phase - first.phase;
    if phase_width <= 0.0 || !phase_width.is_finite() {
        return Err(PredictionCurveError::InvalidWindow);
    }
    let actual_rate = (last.actual - first.actual) / phase_width;
    let rate_error = (rate_guess - actual_rate).abs();
    if !actual_rate.is_finite() || !rate_error.is_finite() {
        return Err(PredictionCurveError::NonFiniteFeedback);
    }
    let mean_absolute_residual = samples
        .iter()
        .enumerate()
        .fold(0.0, |mean, (index, sample)| {
            mean + (sample.residual.abs() - mean) / (index + 1) as f64
        });
    if !mean_absolute_residual.is_finite() {
        return Err(PredictionCurveError::NonFiniteFeedback);
    }
    Ok(PredictionCurveGrade {
        rate_guess,
        actual_rate,
        rate_error,
        mean_absolute_residual,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Band, Prediction, PredictionCurveError, grade_prediction, grade_prediction_curve,
        pose_prediction, prediction_rate_window,
    };

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
        let rate_window = prediction_rate_window(&a);
        assert_eq!(rate_window.len(), 5);
        assert_eq!(rate_window[2], a.phase);
        assert!(
            rate_window.windows(2).all(|pair| pair[0] < pair[1]),
            "the rate window must move forward through phase"
        );
        assert!(rate_window[0] >= 0.0 && rate_window[4] < 1.0);
    }

    #[test]
    fn established_seeds_keep_their_original_phases_including_edges() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        for (seed, expected) in [(4, 0.141), (22, 0.984), (53, 0.016)] {
            let prediction = pose_prediction(room.as_ref(), seed).expect("poses");
            assert_eq!(prediction.phase, expected, "seed {seed} changed meaning");
            let window = prediction_rate_window(&prediction);
            assert_eq!(window[2], expected);
            assert!(window.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(window[0] >= 0.0 && window[4] < 1.0);
        }
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
    fn curve_grade_exposes_the_rate_and_signed_residual_shape() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let prediction = pose_prediction(room.as_ref(), 11).expect("poses");
        let actual = grade_prediction(room.as_ref(), &prediction, prediction.span.0)
            .expect("grades")
            .actual;
        let curve = grade_prediction_curve(room.as_ref(), &prediction, actual, 0.0)
            .expect("finite model grades");

        assert_eq!(
            curve.samples.len(),
            prediction_rate_window(&prediction).len()
        );
        assert_eq!(curve.samples[2].phase, prediction.phase);
        assert!(curve.samples[2].residual.abs() < 1e-12);
        for sample in &curve.samples {
            assert!((sample.predicted - actual).abs() < 1e-12);
            assert!((sample.residual - (sample.actual - sample.predicted)).abs() < 1e-12);
        }
        let first = curve.samples.first().expect("first sample");
        let last = curve.samples.last().expect("last sample");
        let expected_rate = (last.actual - first.actual) / (last.phase - first.phase);
        assert!((curve.actual_rate - expected_rate).abs() < 1e-12);
        assert!((curve.rate_error - expected_rate.abs()).abs() < 1e-12);
        let expected_mean = curve
            .samples
            .iter()
            .map(|sample| sample.residual.abs())
            .sum::<f64>()
            / curve.samples.len() as f64;
        assert!((curve.mean_absolute_residual - expected_mean).abs() < 1e-12);
    }

    #[test]
    fn changing_the_rate_tilts_only_the_committed_model() {
        let room = crate::registry::room_by_id("harmonograph").expect("room");
        let prediction = pose_prediction(room.as_ref(), 13).expect("poses");
        let actual = grade_prediction(room.as_ref(), &prediction, prediction.span.0)
            .expect("grades")
            .actual;
        let falling =
            grade_prediction_curve(room.as_ref(), &prediction, actual, -2.0).expect("grades");
        let rising =
            grade_prediction_curve(room.as_ref(), &prediction, actual, 2.0).expect("grades");

        assert_eq!(
            falling
                .samples
                .iter()
                .map(|sample| sample.actual)
                .collect::<Vec<_>>(),
            rising
                .samples
                .iter()
                .map(|sample| sample.actual)
                .collect::<Vec<_>>(),
            "the committed rate must not alter the room truth"
        );
        assert_eq!(falling.samples[2].predicted, rising.samples[2].predicted);
        assert!(falling.samples[0].predicted > rising.samples[0].predicted);
        assert!(falling.samples[4].predicted < rising.samples[4].predicted);
    }

    #[test]
    fn a_non_finite_curve_model_is_rejected_without_panicking() {
        let room = crate::registry::room_by_id("times-tables").expect("room");
        let prediction = pose_prediction(room.as_ref(), 1).expect("poses");
        assert_eq!(
            grade_prediction_curve(room.as_ref(), &prediction, f64::NAN, 0.0),
            Err(PredictionCurveError::NonFiniteModel)
        );
        assert_eq!(
            grade_prediction_curve(room.as_ref(), &prediction, 1.0, f64::INFINITY),
            Err(PredictionCurveError::NonFiniteModel)
        );
    }

    #[test]
    fn extreme_finite_model_parameters_keep_aggregate_feedback_finite() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let prediction = pose_prediction(room.as_ref(), 17).expect("poses");
        let curve = grade_prediction_curve(room.as_ref(), &prediction, 1.0e308, 0.0)
            .expect("finite residuals remain gradeable without sum overflow");
        assert!(curve.actual_rate.is_finite());
        assert!(curve.rate_error.is_finite());
        assert!(curve.mean_absolute_residual.is_finite());
        assert!(
            curve
                .samples
                .iter()
                .all(|sample| sample.residual.is_finite())
        );
    }

    #[test]
    fn overflowing_prediction_line_reports_a_model_error() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let prediction = pose_prediction(room.as_ref(), 17).expect("poses");
        assert_eq!(
            grade_prediction_curve(room.as_ref(), &prediction, f64::MAX, -f64::MAX,),
            Err(PredictionCurveError::NonFiniteModel)
        );
    }

    #[test]
    fn overflowing_truth_rate_reports_a_feedback_error() {
        struct OverflowRoom;

        impl crate::room::Room for OverflowRoom {
            fn meta(&self) -> crate::room::RoomMeta {
                crate::room::RoomMeta {
                    id: "overflow-room",
                    title: "Overflow Room",
                    wing: "Test",
                    blurb: "Exercises feedback arithmetic.",
                    accent: [0, 0, 0],
                }
            }

            fn render(&self, _surface: &mut dyn crate::surface::Surface, _t: f64) {}

            fn reveal(&self) -> &'static str {
                "Test room."
            }

            fn status(&self, t: f64) -> Option<String> {
                let actual = if t < 0.5 { f64::MAX } else { -f64::MAX };
                Some(format!("VALUE = {actual:.0}"))
            }
        }

        let prediction = Prediction {
            room: "overflow-room".to_string(),
            seed: 0,
            index: 0,
            label: "VALUE".to_string(),
            phase: 0.5,
            span: (-f64::MAX, f64::MAX),
            prompt: String::new(),
        };
        assert_eq!(
            grade_prediction_curve(&OverflowRoom, &prediction, 0.0, 0.0),
            Err(PredictionCurveError::NonFiniteFeedback)
        );
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
