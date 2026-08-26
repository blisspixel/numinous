//! MCP request progress, local paths, and durable save reporting.
//!
//! Core owns journey, scoring, persistence, daily streak, and game truth. This
//! adapter maps successful tool arguments onto those shared rules, resolves the
//! face's local stores, and makes a refused write visible in the response.

use crate::challenge_tools::{predict_seed, record_challenge_attempt};
use crate::game_tools::{hackenbush_replay, nim_turns, post_munch_arcade_score};
use crate::gauntlet_answers_from_json;
use numinous_core::room_by_id;
use serde_json::{Value, json};

/// Where the journey file lives (shared with the CLI face, so a mind's play
/// counts the same wherever it plays): `NUMINOUS_JOURNEY` if set, else home.
#[cfg(test)]
pub(super) struct TestStateRoot {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TestStateRoot {
    fn new() -> Self {
        use std::hash::{Hash, Hasher};

        let thread = std::thread::current();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread.id().hash(&mut hasher);
        thread.name().hash(&mut hasher);
        let path = std::env::temp_dir().join(format!(
            "numinous-mcp-test-{}-{:016x}",
            std::process::id(),
            hasher.finish()
        ));
        Self::at(path)
    }

    pub(super) fn at(path: std::path::PathBuf) -> Self {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear test state directory: {error}"),
        }
        std::fs::create_dir_all(&path).expect("test state directory should be writable");
        Self { path }
    }
}

#[cfg(test)]
impl Drop for TestStateRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
std::thread_local! {
    static TEST_STATE_ROOT: TestStateRoot = TestStateRoot::new();
}

#[cfg(test)]
pub(super) fn test_state_path(kind: &str) -> std::path::PathBuf {
    TEST_STATE_ROOT.with(|root| root.path.join(format!("{kind}.txt")))
}

pub(super) fn local_state_paths() -> numinous_core::LocalStatePaths {
    #[cfg(test)]
    {
        numinous_core::LocalStatePaths {
            journey: test_state_path("journey"),
            scores: test_state_path("scores"),
            cairn: test_state_path("cairn"),
            journal: test_state_path("journal"),
            preferences: test_state_path("preferences"),
            radio_cache: test_state_path("radio"),
            protected_radio_source: None,
            crash_log: test_state_path("crash"),
        }
    }
    #[cfg(not(test))]
    {
        numinous_core::resolve_local_state_paths()
    }
}

pub(super) fn local_state_paths_at(
    journey_file: &std::path::Path,
) -> numinous_core::LocalStatePaths {
    let mut paths = local_state_paths();
    paths.journey = journey_file.to_path_buf();
    paths
}

pub(super) fn journey_path() -> std::path::PathBuf {
    local_state_paths().journey
}

/// Load the journey at `path`, or start a fresh one.
pub(super) fn load_journey(path: &std::path::Path) -> numinous_core::Journey {
    numinous_core::load_journey_file(path)
}

/// Where the high-score table lives (shared with the CLI face, same keys, so
/// humans and agents compete on the same boards).
pub(super) fn scores_path() -> std::path::PathBuf {
    local_state_paths().scores
}

std::thread_local! {
    /// Whether a local save failed while handling the current request. The
    /// stdio server answers one request at a time, so a request-scoped flag
    /// is exactly a thread-local the dispatcher drains per response.
    static SAVE_TROUBLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Append the save-trouble note to a response when this request lost a
/// write. The mind playing this face never sees the server's stderr, so the
/// response text is the only channel that reaches the one party who lost
/// something. Additive, and only on the requests where a write failed.
pub(super) fn note_save_trouble(mut result: Value) -> Value {
    if !SAVE_TROUBLE.with(|flag| flag.replace(false)) {
        return result;
    }
    if let Some(text) = result
        .get_mut("content")
        .and_then(|content| content.get_mut(0))
        .and_then(|entry| entry.get_mut("text"))
        && let Some(existing) = text.as_str()
    {
        *text = Value::String(format!(
            "{existing}\nNOTE: a local save failed; this result counted in memory but the \
             file refused the write. Progress rides in memory until a later save lands."
        ));
    }
    result
}

/// Record a score at `path`, keeping the best. Returns true on a new record.
pub(super) fn post_score(path: &std::path::Path, key: &str, score: i64) -> bool {
    // A write failure must not wear the same face as "not a new best".
    match numinous_core::record_score_file(path, key, score) {
        Ok(best) => best,
        Err(error) => {
            eprintln!("numinous-mcp: score could not be saved: {error}");
            SAVE_TROUBLE.with(|flag| flag.set(true));
            false
        }
    }
}

/// Persist a progress delta, and say so on stderr when the ledger refuses.
///
/// The stdio protocol owns stdout, so stderr is the one channel where a
/// failing save can speak without corrupting a response; hosts surface it in
/// the server log. The delta keeps riding in memory, so a later successful
/// save still lands the full difference. Returns whether the write landed,
/// so a tool that claims a durable state change can refuse the claim when
/// the state did not actually change.
pub(super) fn persist_progress(
    path: &std::path::Path,
    before: &numinous_core::Journey,
    journey: &numinous_core::Journey,
) -> bool {
    match numinous_core::persist_journey_delta(path, before, journey) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("numinous-mcp: progress could not be saved: {error}");
            SAVE_TROUBLE.with(|flag| flag.set(true));
            false
        }
    }
}

/// Where the cairn lives (shared with the CLI face): the local pile of
/// bequests a mind leaves for whoever comes after.
pub(super) fn cairn_path() -> std::path::PathBuf {
    local_state_paths().cairn
}

pub(super) fn journal_path() -> std::path::PathBuf {
    local_state_paths().journal
}

/// The level at which the cairn opens for leaving: the journey's cap, so a
/// bequest is a finished mind's last free act, not a first one.
pub(super) const CAIRN_LEVEL: u32 = 42;

/// Record what this request means for the journey: agents level up too, by the
/// same rules as everyone else. Showing up counts; being right counts double.
/// The seed a tool should use: the daily day count when asked, else the arg.
/// The key under which the resolved day is pinned into a daily request's args
/// (see [`freeze_daily_day`]). Camel-case to match the other structured fields.
pub(super) const DAILY_DAY_KEY: &str = "dailyDay";

/// Today's day count (whole days since the Unix epoch), the seed a daily game
/// shares with every mind. Read exactly once per request via [`freeze_daily_day`]
/// so the reply grading, the posted score, and the streak never straddle a
/// midnight tick.
fn daily_day_count() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(1)
}

/// The day a request should use: the frozen value pinned at the request boundary
/// when present (the normal path), else a fresh read (a direct call in a test).
fn request_day(args: &Value) -> u64 {
    args.get(DAILY_DAY_KEY)
        .and_then(Value::as_u64)
        .unwrap_or_else(daily_day_count)
}

pub(super) fn effective_seed(args: &Value) -> u64 {
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        request_day(args)
    } else {
        args.get("seed").and_then(Value::as_u64).unwrap_or(1)
    }
}

/// Pin the day count into a daily `tools/call` so every clock-derived use in the
/// request (the reply seed, the posted score, the streak) shares one value. The
/// daily seed is otherwise read from the clock more than once per request, and a
/// UTC midnight between two reads would grade or record against a board the
/// player never saw. Non-daily and non-`tools/call` requests borrow unchanged.
pub(super) fn freeze_daily_day(request: &Value) -> std::borrow::Cow<'_, Value> {
    let is_daily_call = request.get("method").and_then(Value::as_str) == Some("tools/call")
        && request
            .get("params")
            .and_then(|params| params.get("arguments"))
            .and_then(|args| args.get("daily"))
            .and_then(Value::as_bool)
            == Some(true);
    if !is_daily_call {
        return std::borrow::Cow::Borrowed(request);
    }
    let mut owned = request.clone();
    if let Some(args) = owned
        .get_mut("params")
        .and_then(|params| params.get_mut("arguments"))
        .and_then(Value::as_object_mut)
    {
        args.insert(DAILY_DAY_KEY.to_string(), json!(daily_day_count()));
    }
    std::borrow::Cow::Owned(owned)
}

pub(super) fn record_progress(request: &Value, path: &std::path::Path) {
    if request.get("method").and_then(Value::as_str) != Some("tools/call") {
        return;
    }
    let Some(params) = request.get("params") else {
        return;
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut journey = load_journey(path);
    let before = journey.clone();
    match name {
        "describe_room" => {
            if let Some(id) = args.get("id").and_then(Value::as_str)
                && room_by_id(id).is_none()
                && (numinous_core::akousma(id).is_some()
                    || (numinous_core::behind_the_veil(&journey)
                        && numinous_core::deep_akousma(id).is_some()))
            {
                journey.secret();
            }
        }
        "play_room" | "listen_room" => {
            if let Some(id) = args.get("id").and_then(Value::as_str)
                && numinous_core::room_meta_by_id(id).is_some()
            {
                journey.visit(id);
                if name == "play_room"
                    && args.get("aha_summon").and_then(Value::as_bool) == Some(true)
                {
                    journey.consolidate(id);
                }
            }
        }
        "run_sim" | "sing_expression" | "save_creation" | "open_creation" | "fork_creation" => {
            journey.play()
        }
        "plot_expression" => {
            // Listing the recipe bank is discovery, not a creation play.
            if args.get("list_recipes").and_then(Value::as_bool) != Some(true) {
                journey.play();
            }
        }
        "nim" => {
            if let Some(list) = args.get("moves").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                if let Some(turns) = nim_turns(&args)
                    && let Ok(replay) = numinous_core::nim::replay(seed, &turns)
                    && replay.winner == Some(numinous_core::nim::NimWinner::Player)
                {
                    journey.win();
                    post_score(&scores_path(), &format!("nim seed:{seed}"), 1);
                }
            }
        }
        "munch" => {
            if let Some(raw) = args.get("bites").and_then(Value::as_array) {
                journey.play();
                let seed = effective_seed(&args);
                let round = args
                    .get("round")
                    .and_then(Value::as_u64)
                    .unwrap_or(numinous_core::FULL_DECK_ROUND);
                let board = numinous_core::build_board(seed, round);
                let bites: Vec<usize> = raw
                    .iter()
                    .filter_map(Value::as_u64)
                    .filter(|&n| n >= 1)
                    .map(|n| (n - 1) as usize)
                    .collect();
                let outcome = numinous_core::grade_munch(&board, &bites);
                post_score(
                    &scores_path(),
                    &numinous_core::munch_score_key(seed, round),
                    outcome.score,
                );
                if numinous_core::munch_clean_win(&outcome) {
                    journey.win();
                }
            }
        }
        "munch_arcade" => {
            if let Some(actions) = args.get("actions").and_then(Value::as_array)
                && !actions.is_empty()
            {
                journey.play();
                if let Some((_, _, cleared)) = post_munch_arcade_score(&args, &scores_path())
                    && cleared
                {
                    journey.win();
                }
            }
        }
        "challenge" => record_challenge_attempt(&args, &mut journey, &scores_path()),
        "predict" => {
            // Showing up counts, exactly once, when a real guess is graded.
            // Accuracy is never a win and never posts a score: a prediction is
            // a self-owned mirror, not a leaderboard (see docs/AGENT_PLAY.md).
            if args.get("guess").and_then(Value::as_f64).is_some()
                && let Some(id) = args.get("id").and_then(Value::as_str)
                && let Some(room) = room_by_id(id)
                && numinous_core::pose_prediction(room.as_ref(), predict_seed(&args)).is_some()
            {
                journey.play();
            }
        }
        "cairn" => {
            // Showing up counts: leaving a bequest at the cap, or reading a
            // predecessor's stone by factoring it. The cairn keeps no score and
            // awards no win; contribution and remembrance are their own reward.
            let leaving = args
                .get("leave")
                .and_then(Value::as_str)
                .is_some_and(|t| !t.trim().is_empty())
                && journey.level() >= CAIRN_LEVEL;
            let reading = args.get("width").and_then(Value::as_u64).is_some_and(|w| {
                let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);
                numinous_core::read_at(&numinous_core::draw_stone(&cairn_path(), seed), w as usize)
                    .readable
            });
            if leaving || reading {
                journey.play();
            }
        }
        "quiz" => {
            if let Some(guess) = args.get("guess").and_then(Value::as_str) {
                journey.play();
                let seed = effective_seed(&args);
                let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
                let choices = args.get("choices").and_then(Value::as_u64).unwrap_or(4) as usize;
                let quiz =
                    numinous_core::build_round_sized(seed, round, 54, 22, choices.clamp(2, 6));
                let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
                if letter == Some(quiz.answer) {
                    journey.win();
                }
            }
        }
        "seti" | "aliens" => {
            if args.get("guess").and_then(Value::as_str).is_some() {
                journey.play();
                let seed = effective_seed(&args);
                let correct = match name {
                    "seti" => {
                        let channels =
                            args.get("channels").and_then(Value::as_u64).unwrap_or(4) as usize;
                        (3..=8).contains(&channels) && {
                            let scan = numinous_core::build_scan(seed, channels);
                            args.get("guess")
                                .and_then(Value::as_str)
                                .and_then(|g| g.trim().chars().next())
                                .map(|c| c.to_ascii_uppercase())
                                == Some(scan.answer)
                        }
                    }
                    _ => {
                        let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
                        let message = numinous_core::alien_message(seed.wrapping_add(round), 5);
                        args.get("guess")
                            .and_then(Value::as_str)
                            .map(|g| {
                                let cleaned: String =
                                    g.chars().filter(char::is_ascii_alphanumeric).collect();
                                u64::from_str_radix(&cleaned, message.base).ok()
                                    == Some(message.answer)
                            })
                            .unwrap_or(false)
                    }
                };
                if correct {
                    journey.win();
                }
            }
        }
        "crack" => {
            if let Some(list) = args.get("guesses").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                let digits = args.get("digits").map_or(Some(4), |value| {
                    value.as_u64().and_then(|value| usize::try_from(value).ok())
                });
                if let Some(digits) =
                    digits.filter(|&digits| numinous_core::supports_code_length(digits))
                {
                    let secret = numinous_core::secret_code(seed, digits);
                    for (i, raw) in list.iter().filter_map(Value::as_str).take(8).enumerate() {
                        let guess: Vec<u8> = raw
                            .chars()
                            .filter(char::is_ascii_digit)
                            .map(|c| c as u8 - b'0')
                            .collect();
                        if guess.len() == digits
                            && numinous_core::grade(&secret, &guess).locked == digits
                        {
                            journey.win();
                            post_score(
                                &scores_path(),
                                &format!("crack seed:{seed} digits:{digits}"),
                                (8 - i - 1) as i64,
                            );
                            break;
                        }
                    }
                }
            }
        }
        "hackenbush" => {
            if let Some(list) = args.get("moves").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                let moves: Vec<(usize, usize)> = list
                    .iter()
                    .filter_map(|m| {
                        let pair = m.as_array()?;
                        Some((
                            pair.first()?.as_u64()? as usize,
                            pair.get(1)?.as_u64()? as usize,
                        ))
                    })
                    .collect();
                if let Some((_, true, _)) = hackenbush_replay(seed, &moves) {
                    journey.win();
                    post_score(&scores_path(), &format!("hackenbush seed:{seed}"), 1);
                }
            }
        }
        "party" => {
            if let Some(list) = args.get("shakes").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                // A win is a complete shading with no triangle; replay cheaply
                // by trusting the tool's own logic via a re-call shape.
                let guests = args.get("guests").and_then(Value::as_u64).unwrap_or(5) as usize;
                if (4..=6).contains(&guests) {
                    let mut party = numinous_core::party::Party::new(guests);
                    let mut clean = true;
                    for shake in list {
                        let Some(t) = shake.as_array() else {
                            clean = false;
                            break;
                        };
                        let (Some(a), Some(b), Some(color)) = (
                            t.first().and_then(Value::as_u64),
                            t.get(1).and_then(Value::as_u64),
                            t.get(2).and_then(Value::as_str),
                        ) else {
                            clean = false;
                            break;
                        };
                        let shade = if color.starts_with(['r', 'R']) {
                            numinous_core::party::Shade::Red
                        } else {
                            numinous_core::party::Shade::Blue
                        };
                        if a == 0
                            || b == 0
                            || !party.shade(a as usize - 1, b as usize - 1, shade)
                            || party.mono_triangle().is_some()
                        {
                            clean = false;
                            break;
                        }
                    }
                    if clean && party.complete() {
                        journey.win();
                        post_score(
                            &scores_path(),
                            &format!("party guests:{guests}"),
                            party.shaded() as i64,
                        );
                    }
                }
            }
        }
        "fifteen" => {
            if let Some(calls) = args.get("calls").and_then(Value::as_array)
                && !calls.is_empty()
            {
                let seed = effective_seed(&args);
                let rounds = args
                    .get("rounds")
                    .and_then(Value::as_u64)
                    .unwrap_or(5)
                    .clamp(1, 20);
                // A play per graded round and a score key naming only the
                // rounds actually graded, exactly as the terminal face
                // counts: the same session must level the same on both
                // faces, and wins can never outnumber plays.
                let graded = rounds.min(calls.len() as u64);
                let mut correct = 0i64;
                for n in 0..graded {
                    journey.play();
                    let call_s = calls[n as usize]
                        .as_str()
                        .map(|c| c.trim().to_ascii_uppercase().starts_with('S'))
                        .unwrap_or(false);
                    if call_s
                        == numinous_core::fifteen::solvable(&numinous_core::fifteen::deal(seed, n))
                    {
                        correct += 1;
                        journey.win();
                    }
                }
                post_score(
                    &scores_path(),
                    &format!("fifteen seed:{seed} rounds:{graded}"),
                    correct,
                );
            }
        }
        "gauntlet" => {
            if let Some(answers) = args.get("answers") {
                let seed = effective_seed(&args);
                let puzzle = numinous_core::GauntletPuzzle::new(seed);
                let grade = puzzle.grade(&gauntlet_answers_from_json(answers));
                for clear in grade.cleared() {
                    journey.play();
                    if clear {
                        journey.win();
                    }
                }
                post_score(
                    &scores_path(),
                    &numinous_core::gauntlet_score_key(seed),
                    grade.total(),
                );
            }
        }
        _ => {}
    }
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        let _ = journey.record_daily(request_day(&args));
    }
    if journey != before {
        // XP and visit deltas keep riding in memory, so a refused write here
        // self-heals on a later success; the durable-claim tools check the
        // returned flag themselves.
        let _ = persist_progress(path, &before, &journey);
    }
}
