//! Prediction and parameter challenge posing, grading, and attempt accounting.

use numinous_broadcast::{
    PLAY_ROOM_DEFAULT_HEIGHT as DEFAULT_HEIGHT, PLAY_ROOM_DEFAULT_WIDTH as DEFAULT_WIDTH,
};
use numinous_core::room_by_id;
use serde_json::{Value, json};

use super::room_input::parse_room_pokes;
use super::{post_score, tool_error, tool_structured, unknown_room};

/// The challenge seed: always the explicit argument, never the daily clock.
///
/// Challenges are graded twice per request (once for the reply, once for
/// progress recording), so a clock-derived seed could pose two different
/// goals across a midnight boundary. An explicit seed cannot drift; agents
/// who want a shared daily goal can pass today's day number themselves.
fn challenge_seed(args: &Value) -> u64 {
    args.get("seed").and_then(Value::as_u64).unwrap_or(1)
}

/// The requested challenge kind, exactly as passed (validation happens in
/// the tool so bad values earn a guiding error, not a silent default).
fn challenge_kind(args: &Value) -> Option<&str> {
    args.get("kind").and_then(Value::as_str)
}

/// Read an optional challenge phase without depending on schema validation.
/// Internal progress helpers call the same tools directly in tests and replay
/// paths, so the domain boundary must reject invalid phases on its own.
fn challenge_phase(args: &Value) -> Result<Option<f64>, &'static str> {
    let Some(value) = args.get("t") else {
        return Ok(None);
    };
    let Some(t) = value.as_f64() else {
        return Err("Argument 't' must be a phase in [0,1).");
    };
    if !(0.0..1.0).contains(&t) {
        return Err("Argument 't' must be a phase in [0,1).");
    }
    Ok(Some(t))
}

/// The prediction seed: always explicit, never the clock, so posing and
/// recording cannot pick two different moments across a midnight boundary.
pub(super) fn predict_seed(args: &Value) -> u64 {
    args.get("seed").and_then(Value::as_u64).unwrap_or(1)
}

/// The `predict` tool: commit a guess of a room's readout at a hidden moment,
/// graded as a gap with a learning-progress band. Call without `guess` to pose
/// (the moment, the readout's name, its range); call again with `guess` to see
/// the truth and how close your model came.
///
/// Deliberately not a leaderboard: it never posts a score and never awards a
/// win for accuracy. The score is a mirror of the guesser's model, so guessing
/// after observing only fools your own ledger. This is the honest form in a
/// fully observable deterministic world, and the welfare stance for digital
/// minds (see docs/AGENT_PLAY.md and docs/PEDAGOGY.md).
pub(super) fn predict_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    // Predict the room the player actually played: honor the same `variation`
    // as play_room, so the graded truth is the readout they saw, not a different
    // (canonical) universe. In a chaotic room even a small parameter shift moves
    // the answer, so grading variation 5's model against variation 0 would call a
    // faithful prediction wrong. Pass the same seed and variation to both calls.
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let room = numinous_core::room_by_id_with(id, variation);
    let Some(room) = room else {
        return tool_error(&unknown_room(id));
    };
    let seed = predict_seed(args);
    let Some(prediction) = numinous_core::pose_prediction(room.as_ref(), seed) else {
        return tool_error(&format!(
            "{id} has no moving numeric readout to predict, so no prediction can be posed. Predictions need a room whose status line carries a number that changes with phase; describe_room names each room's readout."
        ));
    };
    let (lo, hi) = prediction.span;
    let guess = match args.get("guess") {
        Some(value) => match value.as_f64() {
            Some(guess) => Some(guess),
            None => return tool_error("'guess' must be a number."),
        },
        None => None,
    };
    let rate_guess = match args.get("rate") {
        Some(value) => match value.as_f64() {
            Some(rate) => Some(rate),
            None => return tool_error("'rate' must be a number."),
        },
        None => None,
    };
    if rate_guess.is_some() && guess.is_none() {
        return tool_error("'rate' requires a numeric 'guess' to anchor the prediction line.");
    }
    let Some(guess) = guess else {
        let rate_window = numinous_core::prediction_rate_window(&prediction);
        return tool_structured(
            &format!(
                "{}\n\nCall predict again with the same seed and variation ({variation}), your `guess`, and optionally your `rate` to see the truth, local rate, and signed residual shape.",
                prediction.prompt
            ),
            json!({
                "game": "predict",
                "room": prediction.room,
                "seed": seed,
                "variation": variation,
                "label": prediction.label,
                "phase": prediction.phase,
                "span": [lo, hi],
                "rate_window": rate_window,
                "prompt": prediction.prompt,
            }),
        );
    };
    let Some(grade) = numinous_core::grade_prediction(room.as_ref(), &prediction, guess) else {
        return tool_error(&format!("{id}'s readout vanished at the posed moment."));
    };
    if let Some(rate_guess) = rate_guess {
        let curve = match numinous_core::grade_prediction_curve(
            room.as_ref(),
            &prediction,
            guess,
            rate_guess,
        ) {
            Ok(curve) => curve,
            Err(numinous_core::PredictionCurveError::ReadoutUnavailable) => {
                return tool_error(&format!(
                    "{id}'s readout vanished inside the posed rate window."
                ));
            }
            Err(numinous_core::PredictionCurveError::NonFiniteModel) => {
                return tool_error(
                    "The committed guess and rate produce values outside the numeric range. Use smaller finite values.",
                );
            }
            Err(numinous_core::PredictionCurveError::NonFiniteFeedback) => {
                return tool_error(
                    "The room truth and committed model produce feedback outside the numeric range. Pose a different window or use smaller finite values.",
                );
            }
            Err(numinous_core::PredictionCurveError::InvalidWindow) => {
                return tool_error("The posed rate window is invalid; pose a new prediction.");
            }
        };
        let residuals = curve
            .samples
            .iter()
            .map(|sample| sample.residual)
            .collect::<Vec<_>>();
        let error_shape = curve
            .samples
            .iter()
            .map(|sample| {
                json!({
                    "phase": sample.phase,
                    "predicted": sample.predicted,
                    "actual": sample.actual,
                    "residual": sample.residual,
                })
            })
            .collect::<Vec<_>>();
        return tool_structured(
            &format!(
                "{}. Point guess {:.3}; actual {:.3} at phase {:.3} ({:.3} off, score {}/100). Your rate {:.3}; actual local rate {:.3} ({:.3} off). Signed residual shape, actual minus predicted: {:?}. This is model feedback, not a leaderboard.",
                grade.band.name(),
                grade.guess,
                grade.actual,
                prediction.phase,
                grade.error,
                grade.score,
                curve.rate_guess,
                curve.actual_rate,
                curve.rate_error,
                residuals,
            ),
            json!({
                "game": "predict",
                "room": prediction.room,
                "seed": seed,
                "variation": variation,
                "label": prediction.label,
                "phase": prediction.phase,
                "guess": grade.guess,
                "actual": grade.actual,
                "error": grade.error,
                "score": grade.score,
                "band": grade.band.name(),
                "rate_guess": curve.rate_guess,
                "actual_rate": curve.actual_rate,
                "rate_error": curve.rate_error,
                "mean_absolute_residual": curve.mean_absolute_residual,
                "error_shape": error_shape,
            }),
        );
    }
    tool_structured(
        &format!(
            "{}. You guessed {:.3}; {} actually read {:.3} at phase {:.3} ({:.3} off, score {}/100, seed {seed}). The score is a mirror of your model, not a leaderboard.",
            grade.band.name(),
            grade.guess,
            prediction.label,
            grade.actual,
            prediction.phase,
            grade.error,
            grade.score
        ),
        json!({
            "game": "predict",
            "room": prediction.room,
            "seed": seed,
            "variation": variation,
            "label": prediction.label,
            "phase": prediction.phase,
            "guess": grade.guess,
            "actual": grade.actual,
            "error": grade.error,
            "score": grade.score,
            "band": grade.band.name(),
        }),
    )
}

/// Record a parameter attempt: showing up counts (play), landing within
/// tolerance counts double (win), and the graded score posts under
/// `challenge <room> parameter seed:N`. Pose-only calls (no `t`) record
/// nothing, mirroring the touch kind's pose/grade split.
fn record_parameter_attempt(
    args: &Value,
    journey: &mut numinous_core::Journey,
    scores: &std::path::Path,
) {
    let Ok(Some(t)) = challenge_phase(args) else {
        return;
    };
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(room) = room_by_id(id) else {
        return;
    };
    let seed = challenge_seed(args);
    let Some(goal) = numinous_core::pose_parameter_goal(room.as_ref(), seed) else {
        return;
    };
    let Some(grade) = numinous_core::grade_parameter(room.as_ref(), &goal, t) else {
        return;
    };
    journey.play();
    post_score(
        scores,
        &format!("challenge {id} parameter seed:{seed}"),
        i64::from(grade.score),
    );
    if grade.within {
        journey.win();
    }
}

/// Record what a challenge attempt means for progress: showing up counts
/// (play), clearing the threshold counts double (win), and the graded score
/// posts under `challenge <room> seed:N`. Pose-only calls record nothing.
/// Separated from `record_progress` so the semantics are testable against
/// explicit temp paths, like the arcade replay path.
pub(super) fn record_challenge_attempt(
    args: &Value,
    journey: &mut numinous_core::Journey,
    scores: &std::path::Path,
) {
    if challenge_kind(args) == Some("parameter") {
        record_parameter_attempt(args, journey, scores);
        return;
    }
    let Ok(pokes) = parse_room_pokes(args) else {
        return;
    };
    if pokes.is_empty() {
        return;
    }
    let Ok(t) = challenge_phase(args) else {
        return;
    };
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(room) = room_by_id(id) else {
        return;
    };
    let seed = challenge_seed(args);
    let Some(challenge) = numinous_core::pose_challenge(
        room.as_ref(),
        seed,
        DEFAULT_WIDTH as usize,
        DEFAULT_HEIGHT as usize,
    ) else {
        return;
    };
    journey.play();
    let t = t.unwrap_or(0.0);
    let grade = numinous_core::grade_challenge(room.as_ref(), &challenge, t, &pokes);
    post_score(
        scores,
        &format!("challenge {id} seed:{seed}"),
        i64::from(grade.score),
    );
    if grade.passed {
        journey.win();
    }
}

/// The `challenge` tool: pose a seeded touch goal, or grade an attempt.
///
/// Pose and grade run on the server's default frame so goals are comparable
/// across minds. Grading recomputes deterministically from (room, seed, t,
/// pokes), so the same attempt always earns the same numbers.
pub(super) fn challenge_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(room) = room_by_id(id) else {
        return tool_error(&unknown_room(id));
    };
    let seed = challenge_seed(args);
    let kind = match args.get("kind") {
        None => "touch",
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => {
            return tool_error("Argument 'kind' must be a string: \"touch\" or \"parameter\".");
        }
    };
    match kind {
        "touch" => {}
        "parameter" => return parameter_challenge_tool(room.as_ref(), id, seed, args),
        other => {
            return tool_error(&format!(
                "Unknown challenge kind '{other}'. Valid kinds: touch (change cells in a target box, graded on your pokes) and parameter (land the room's status readout on a target number, graded on your t)."
            ));
        }
    }
    let t = match challenge_phase(args) {
        Ok(t) => t.unwrap_or(0.0),
        Err(message) => return tool_error(message),
    };
    let Some(challenge) = numinous_core::pose_challenge(
        room.as_ref(),
        seed,
        DEFAULT_WIDTH as usize,
        DEFAULT_HEIGHT as usize,
    ) else {
        return tool_error(&format!(
            "{id} does not answer the hand yet, so no challenge can be posed. Challenges need a room with a touch verb; describe_room names each room's action."
        ));
    };
    let pokes = match parse_room_pokes(args) {
        Ok(pokes) => pokes,
        Err(message) => return tool_error(&message),
    };
    let (x0, y0, x1, y1) = challenge.target;
    if pokes.is_empty() {
        return tool_structured(
            &format!(
                "{}\n\nCall challenge again with the same seed and your pokes ([[x,y], ...] in [0,1]) to be graded. Every attempt gets metrics, not pass/fail: cells changed in the target, cells changed overall, centroid distance, and a 0-100 score to climb.",
                challenge.goal
            ),
            json!({
                "game": "challenge",
                "room": challenge.room,
                "seed": seed,
                "goal": challenge.goal,
                "target": [x0, y0, x1, y1],
                "minCells": challenge.min_cells,
                "width": challenge.width,
                "height": challenge.height,
            }),
        );
    }
    let grade = numinous_core::grade_challenge(room.as_ref(), &challenge, t, &pokes);
    let verdict = if grade.passed { "PASSED. " } else { "" };
    tool_structured(
        &format!(
            "{verdict}Score {}/100: {} of {} needed cells changed inside the target, {} changed overall, centroid {:.1} cells from target center (seed {seed}).",
            grade.score,
            grade.cells_in_target,
            challenge.min_cells,
            grade.cells_changed,
            grade.center_distance
        ),
        json!({
            "game": "challenge",
            "room": challenge.room,
            "seed": seed,
            "target": [x0, y0, x1, y1],
            "minCells": challenge.min_cells,
            "cellsInTarget": grade.cells_in_target,
            "cellsChanged": grade.cells_changed,
            "thresholdFraction": grade.threshold_fraction,
            "centerDistance": grade.center_distance,
            "passed": grade.passed,
            "score": grade.score,
        }),
    )
}

/// The parameter kind of the `challenge` tool: pose a readout target, or
/// grade an attempted phase.
///
/// The goal targets the room's own status line, the same instrument the
/// player reads, so posing and grading can never disagree with the screen.
/// Omitting `t` poses; passing it grades, because for this kind the phase
/// IS the attempt.
fn parameter_challenge_tool(
    room: &dyn numinous_core::Room,
    id: &str,
    seed: u64,
    args: &Value,
) -> Value {
    let Some(goal) = numinous_core::pose_parameter_goal(room, seed) else {
        return tool_error(&format!(
            "{id} has no moving numeric readout, so no parameter goal can be posed. Parameter goals need a room whose status line carries a number that changes with phase; try the touch kind, or another room."
        ));
    };
    let (lo, hi) = goal.span;
    let t = match challenge_phase(args) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return tool_structured(
                &format!(
                    "{}\n\nThe readout ranges roughly {lo:.3} to {hi:.3} across the sweep. Call challenge again with the same seed and kind plus your t in [0,1) to be graded. Every attempt gets metrics, not pass/fail: the readout you landed on, its distance from the target, and a 0-100 score to climb.",
                    goal.goal
                ),
                json!({
                    "game": "challenge",
                    "kind": "parameter",
                    "room": goal.room,
                    "seed": seed,
                    "goal": goal.goal,
                    "label": goal.label,
                    "target": goal.target,
                    "tolerance": goal.tolerance,
                    "span": [lo, hi],
                }),
            );
        }
        Err(message) => return tool_error(message),
    };
    let Some(grade) = numinous_core::grade_parameter(room, &goal, t) else {
        return tool_error(&format!(
            "{id}'s readout vanished at t={t}; try a different phase."
        ));
    };
    let verdict = if grade.within { "LANDED. " } else { "" };
    tool_structured(
        &format!(
            "{verdict}Score {}/100: {} read {:.3} at t={t}, {:.3} from the target (seed {seed}); structuredContent carries the exact target and tolerance.",
            grade.score, goal.label, grade.value, grade.distance
        ),
        json!({
            "game": "challenge",
            "kind": "parameter",
            "room": goal.room,
            "seed": seed,
            "label": goal.label,
            "target": goal.target,
            "tolerance": goal.tolerance,
            "value": grade.value,
            "distance": grade.distance,
            "within": grade.within,
            "score": grade.score,
        }),
    )
}
