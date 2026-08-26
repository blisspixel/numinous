//! MCP projection for stateless games and shared scores.
//!
//! Game rules, state transitions, grading, and score persistence remain in
//! core. This module owns the MCP-facing replay boundary and result projection.

use serde_json::{Value, json};

use crate::{effective_seed, load_journey, post_score, tool_error, tool_structured};

/// Replay a hackenbush move list; None on an illegal move, else the final
/// garden and whether the player has already won.
pub(super) fn hackenbush_replay(
    seed: u64,
    moves: &[(usize, usize)],
) -> Option<(numinous_core::hackenbush::Stalks, bool, Vec<String>)> {
    use numinous_core::hackenbush as hb;
    let mut stalks = hb::new_garden(seed);
    let mut narration = Vec::new();
    for &(stalk, height) in moves {
        if stalk == 0 || height == 0 || !hb::cut(&mut stalks, stalk - 1, height - 1, hb::Color::Red)
        {
            return None;
        }
        if !hb::can_move(&stalks, hb::Color::Blue) {
            return Some((stalks, true, narration));
        }
        let (bi, bh) = hb::order_move(&stalks)?;
        let _ = hb::cut(&mut stalks, bi, bh, hb::Color::Blue);
        narration.push(format!(
            "The Order cuts stalk {} at height {}.",
            bi + 1,
            bh + 1
        ));
    }
    Some((stalks, false, narration))
}

/// The garden as plain text rows for the tool reply.
fn garden_rows(stalks: &numinous_core::hackenbush::Stalks) -> String {
    use numinous_core::hackenbush::Color;
    let tallest = stalks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    for row in (0..tallest).rev() {
        for stalk in stalks {
            out.push(match stalk.get(row) {
                Some(Color::Red) => 'R',
                Some(Color::Blue) => 'B',
                None => '.',
            });
            out.push(' ');
        }
        out.push('\n');
    }
    for i in 1..=stalks.len() {
        out.push_str(&format!("{i} "));
    }
    out
}

/// The `hackenbush` tool.
pub(super) fn hackenbush_tool(args: &Value) -> Value {
    use numinous_core::hackenbush as hb;
    let seed = effective_seed(args);
    let moves: Vec<(usize, usize)> = args
        .get("moves")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|m| {
                    let pair = m.as_array()?;
                    Some((
                        pair.first()?.as_u64()? as usize,
                        pair.get(1)?.as_u64()? as usize,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let Some((stalks, won, narration)) = hackenbush_replay(seed, &moves) else {
        return tool_error("Illegal cut: pick a RED segment as [stalk, height], both 1-based.");
    };
    if won {
        let secret = hb::the_secret();
        return tool_structured(
            &format!(
                "The Order has nothing left to cut. It concedes, and keeps its word:\n\n{secret}"
            ),
            // The promised secret rides in structuredContent too.
            json!({ "game": "hackenbush", "seed": seed, "won": true, "secret": secret }),
        );
    }
    if !hb::can_move(&stalks, hb::Color::Red) {
        return tool_structured(
            "No red left to cut. The Order takes the garden. (It was arithmetic.)",
            json!({ "game": "hackenbush", "seed": seed, "won": false }),
        );
    }
    tool_structured(
        &format!(
            "HACKENBUSH seed {seed}. Cut RED as [stalk, height] (1-based); whoever cannot cut loses. This garden is winnable.\n{}\n{}",
            narration.join("\n"),
            garden_rows(&stalks)
        ),
        // The Order's replies ride in the structured payload, so a mind on a
        // structured-content client can follow the game.
        json!({ "game": "hackenbush", "seed": seed, "stalks": stalks.len(), "order": narration }),
    )
}

/// The `party` tool.
pub(super) fn party_tool(args: &Value) -> Value {
    use numinous_core::party::{Party, Shade};
    let guests = args.get("guests").and_then(Value::as_u64).unwrap_or(5) as usize;
    if !(4..=6).contains(&guests) {
        return tool_error("Parties run 4 to 6 guests (5 is escapable; 6 is Ramsey's).");
    }
    let mut party = Party::new(guests);
    if let Some(list) = args.get("shakes").and_then(Value::as_array) {
        for shake in list {
            let Some(t) = shake.as_array() else {
                return tool_error("Each shake is [a, b, \"r\"|\"b\"], guests 1-based.");
            };
            let (Some(a), Some(b), Some(color)) = (
                t.first().and_then(Value::as_u64),
                t.get(1).and_then(Value::as_u64),
                t.get(2).and_then(Value::as_str),
            ) else {
                return tool_error("Each shake is [a, b, \"r\"|\"b\"], guests 1-based.");
            };
            let shade = if color.starts_with(['r', 'R']) {
                Shade::Red
            } else {
                Shade::Blue
            };
            if a == 0 || b == 0 || !party.shade(a as usize - 1, b as usize - 1, shade) {
                return tool_error(&format!("Handshake {a}-{b} is not open."));
            }
            if let Some((x, y, z, _)) = party.mono_triangle() {
                let lesson = if guests == 6 {
                    "It was never possible: among six, three mutual friends or three mutual strangers MUST exist. R(3,3) = 6."
                } else {
                    "Five CAN escape: ring one color, star the other (the pentagon's trick)."
                };
                return tool_structured(
                    &format!(
                        "A one-color triangle: guests {}, {}, {}. {} handshakes survived. {lesson}",
                        x + 1,
                        y + 1,
                        z + 1,
                        party.shaded() - 1
                    ),
                    // The Ramsey lesson and the offending triangle ride in the
                    // structured payload, so the teaching is not text-only.
                    json!({ "game": "party", "guests": guests, "escaped": false, "survived": party.shaded() - 1, "triangle": [x + 1, y + 1, z + 1], "why": lesson }),
                );
            }
        }
    }
    if party.complete() {
        return tool_structured(
            &format!(
                "Every handshake shaded, no triangle: you escaped with all {}.{}",
                party.shaded(),
                if guests == 5 {
                    " Now try six; Ramsey is waiting."
                } else {
                    ""
                }
            ),
            json!({ "game": "party", "guests": guests, "escaped": true }),
        );
    }
    tool_structured(
        &format!(
            "THE PARTY: {guests} guests, {} of {} handshakes shaded, no triangle yet. Shade with shakes: [[a, b, \"r\"], ...].",
            party.shaded(),
            party.edges.len()
        ),
        json!({ "game": "party", "guests": guests, "shaded": party.shaded(), "total": party.edges.len() }),
    )
}

/// The `fifteen` tool.
pub(super) fn fifteen_tool(args: &Value) -> Value {
    use numinous_core::fifteen as ff;
    let seed = effective_seed(args);
    let rounds = args
        .get("rounds")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20);
    match args.get("calls").and_then(Value::as_array) {
        None => {
            let boards: Vec<String> = (0..rounds)
                .map(|n| {
                    format!(
                        "SCRAMBLE {}:\n{}",
                        n + 1,
                        ff::board_text(&ff::deal(seed, n))
                    )
                })
                .collect();
            // The scramble boards a mind must read to call each deal ride in the
            // structured payload too, so the puzzle is not text-only.
            let scrambles: Vec<Value> = (0..rounds)
                .map(|n| json!({ "round": n + 1, "board": ff::board_text(&ff::deal(seed, n)) }))
                .collect();
            tool_structured(
                &format!(
                    "FIFTEEN'S BET (seed {seed}). For each scramble call S (solvable) or U (stuck forever); half of all deals are lies and parity is the tell.\n\n{}\nCall again with calls: [\"S\", \"U\", ...].",
                    boards.join("\n")
                ),
                json!({ "game": "fifteen", "seed": seed, "rounds": rounds, "scrambles": scrambles }),
            )
        }
        Some(calls) => {
            let mut lines = Vec::new();
            let mut verdicts = Vec::new();
            let mut correct = 0u64;
            // Only the calls actually made are graded, and only they are
            // reported and scored: three correct calls out of three sent is
            // not "3 of 5 called", and a partial run must not sit on the
            // shared board under a complete run's key. The terminal face
            // posts rounds:{completed} the same way on an early exit.
            let graded = rounds.min(calls.len() as u64);
            for n in 0..graded {
                let call_s = calls[n as usize]
                    .as_str()
                    .map(|c| c.trim().to_ascii_uppercase().starts_with('S'))
                    .unwrap_or(false);
                let tiles = ff::deal(seed, n);
                let truth = ff::solvable(&tiles);
                let right = call_s == truth;
                if right {
                    correct += 1;
                    lines.push(format!("{}: called it. {}", n + 1, ff::why(&tiles)));
                } else {
                    lines.push(format!("{}: no. {}", n + 1, ff::why(&tiles)));
                }
                // Each round's parity tell (the whole lesson) rides in the JSON.
                verdicts.push(json!({ "round": n + 1, "correct": right, "solvable": truth, "why": ff::why(&tiles) }));
            }
            tool_structured(
                &format!("{}\n{correct} of {graded} called.", lines.join("\n")),
                json!({ "game": "fifteen", "seed": seed, "correct": correct, "rounds": graded, "verdicts": verdicts }),
            )
        }
    }
}

pub(super) fn quiz_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    quiz_tool_at_level(args, load_journey(journey_file).level())
}

pub(super) fn quiz_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let choice_count = args.get("choices").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(2..=6).contains(&choice_count) {
        return tool_error("Rounds run 2 to 6 choices.");
    }
    if choice_count > 4 && level < 3 {
        return tool_error("Five-way and six-way rounds open at LV 3. Keep playing.");
    }
    let quiz = numinous_core::build_round_sized(seed, round, 54, 22, choice_count);
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
            let correct = letter == Some(quiz.answer);
            let verdict = if correct { "Correct!" } else { "Not quite." };
            tool_structured(
                &format!(
                    "{verdict} The answer was {} ({}).\n\n{}",
                    quiz.answer, quiz.answer_title, quiz.answer_reveal
                ),
                json!({
                    "game": "quiz",
                    "seed": seed,
                    "round": round,
                    "choiceCount": choice_count,
                    "correct": correct,
                    "answer": quiz.answer.to_string(),
                    "answerTitle": quiz.answer_title,
                    // The "why" the shape is what it is, so a wrong guess still
                    // teaches on a client that surfaces only structuredContent.
                    "why": quiz.answer_reveal
                }),
            )
        }
        None => {
            let choice_lines: Vec<String> = quiz
                .choices
                .iter()
                .map(|c| format!("{}) {}", c.letter, c.title))
                .collect();
            let choice_json: Vec<Value> = quiz
                .choices
                .iter()
                .map(|c| json!({ "letter": c.letter.to_string(), "title": c.title }))
                .collect();
            tool_structured(
                &format!(
                    "Guess the shape (seed {seed}, round {round}, choices {choice_count}):\n\n{}\n{}\n\nCall quiz again with seed {seed}, round {round}, choices {choice_count}, and your guess letter.",
                    quiz.art,
                    choice_lines.join("\n")
                ),
                json!({
                    "game": "quiz",
                    "seed": seed,
                    "round": round,
                    "choiceCount": choice_count,
                    // The mystery render and the lettered choices ride in the
                    // structured payload, so a mind on a structured-content-only
                    // client sees the puzzle and can guess, not just read that a
                    // quiz exists. The answer waits for the grade.
                    "art": quiz.art,
                    "choices": choice_json
                }),
            )
        }
    }
}

/// The `munch` tool: present a board, or grade a set of bites.
pub(super) fn munch_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let round = args
        .get("round")
        .and_then(Value::as_u64)
        .unwrap_or(numinous_core::FULL_DECK_ROUND);
    let board = numinous_core::build_board(seed, round);
    match args.get("bites").and_then(Value::as_array) {
        Some(raw) => {
            let bites: Vec<usize> = raw
                .iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .map(|n| (n - 1) as usize)
                .collect();
            let outcome = numinous_core::grade_munch(&board, &bites);
            let verdict = if outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0
            {
                "PERFECT."
            } else {
                "Munched."
            };
            tool_structured(
                &format!(
                    "{verdict} {} eaten, {} bad bites, {} left behind. Score: {} (seed {seed}, round {round}).",
                    outcome.hits, outcome.bad_bites, outcome.left_behind, outcome.score
                ),
                json!({
                    "game": "munch",
                    "seed": seed,
                    "round": round,
                    "hits": outcome.hits,
                    "badBites": outcome.bad_bites,
                    "leftBehind": outcome.left_behind,
                    "wronglyEaten": outcome.wrongly_eaten,
                    "missed": outcome.missed,
                    "perfect": outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0,
                    "score": outcome.score
                }),
            )
        }
        None => tool_structured(
            &format!(
                "{}\n{}\nCall munch again with your bites (1-based cell numbers).",
                board.rule.describe(),
                numinous_core::board_text(&board)
            ),
            json!({
                "game": "munch",
                "seed": seed,
                "round": round,
                // The rule and the board itself ride in the structured payload,
                // so a structured-content-only mind sees which cells to eat, not
                // just that a board exists.
                "rule": board.rule.describe(),
                "board": numinous_core::board_text(&board)
            }),
        ),
    }
}

pub(super) fn arcade_action(value: &Value) -> Option<numinous_core::munch_arcade::Action> {
    use numinous_core::munch_arcade::Action;
    Some(match value.as_str()?.to_ascii_lowercase().as_str() {
        "up" | "w" => Action::Up,
        "down" | "s" => Action::Down,
        "left" | "a" => Action::Left,
        "right" | "d" => Action::Right,
        "eat" | "e" => Action::Eat,
        _ => return None,
    })
}

fn replay_munch_arcade(args: &Value) -> Option<(numinous_core::munch_arcade::Arcade, bool)> {
    use numinous_core::munch_arcade::Turn;
    let seed = effective_seed(args);
    let mut run = numinous_core::munch_arcade::Arcade::new(seed);
    let actions = args.get("actions").and_then(Value::as_array)?;
    let mut cleared = false;
    for action in actions.iter().filter_map(arcade_action) {
        if matches!(run.turn(action), Turn::Cleared) {
            cleared = true;
        }
    }
    Some((run, cleared))
}

pub(super) fn post_munch_arcade_score(
    args: &Value,
    scores_file: &std::path::Path,
) -> Option<(u64, i64, bool)> {
    let seed = effective_seed(args);
    let (run, cleared) = replay_munch_arcade(args)?;
    post_score(scores_file, &format!("arcade seed:{seed}"), run.score);
    Some((seed, run.score, cleared))
}

/// The `munch_arcade` tool: the full hunted arcade. Call with seed to see the board; call with "actions" list to replay the run (stateless). Returns text + structured state. Scores as "arcade seed:N".
pub(super) fn munch_arcade_tool(args: &Value) -> Value {
    use numinous_core::munch_arcade::{Arcade, Turn};
    let seed = effective_seed(args);
    let mut run = Arcade::new(seed);
    let mut cleared = false;
    if let Some(raw) = args.get("actions").and_then(Value::as_array) {
        for action in raw.iter().filter_map(arcade_action) {
            if matches!(run.turn(action), Turn::Cleared) {
                cleared = true;
            }
        }
    }
    // Shared with the App viewer attestation path: muncher never hides digits.
    let board = numinous_core::munch_arcade::board_text(&run);
    let state_text = format!(
        "ARCADE seed {seed} LEVEL {} LIVES {} SCORE {}\nRULE: {}\n{}",
        run.level,
        run.lives,
        run.score,
        run.board.rule.describe(),
        board
    );
    tool_structured(
        &state_text,
        json!({
            "game": "arcade",
            "seed": seed,
            "level": run.level,
            "lives": run.lives,
            "score": run.score,
            "muncher": run.muncher,
            "vexations": run.vexations.iter().map(|v| json!({"cell": v.cell, "mind": format!("{:?}", v.mind)})).collect::<Vec<_>>(),
            // The rule to eat by and the board a mind reads ride in the
            // structured payload, not only the text state.
            "rule": run.board.rule.describe(),
            "board": board,
            "cleared": cleared,
            "over": run.lives == 0
        }),
    )
}

/// The `scores` tool: the shared high-score table, prose and structured.
pub(super) fn scores_tool(path: &std::path::Path) -> Value {
    let board = numinous_core::load_scoreboard_file(path);
    if board.entries.is_empty() {
        return tool_structured(
            "No scores yet. Post one: munch, quiz.",
            json!({ "count": 0, "top": [], "truncated": false }),
        );
    }
    let mut lines = vec!["HIGH SCORES".to_string()];
    let mut structured = Vec::new();
    for (rank, (key, score)) in board.top(15).iter().enumerate() {
        lines.push(format!("  {:>2}.  {score:>6}  {key}", rank + 1));
        structured.push(json!({ "rank": rank + 1, "key": key, "score": score }));
    }
    tool_structured(
        &lines.join("\n"),
        json!({
            "count": board.entries.len(),
            "truncated": board.entries.len() > structured.len(),
            "top": structured,
        }),
    )
}

/// The `nim` tool: replay the whole game from the move list, statelessly.
pub(super) fn nim_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let Some(turns) = nim_turns(args) else {
        return tool_error("Invalid Nim move history.");
    };
    let replay = match numinous_core::nim::replay(seed, &turns) {
        Ok(replay) => replay,
        Err(numinous_core::nim::NimReplayError::IllegalPlayerMove { turn, heaps }) => {
            return tool_error(&format!(
                "Illegal move: take {} from heap {}. Heaps now: {heaps:?}.",
                turn.take,
                turn.heap + 1
            ));
        }
        Err(numinous_core::nim::NimReplayError::InvalidOrderMove { .. }) => {
            return tool_error("The Order could not produce a legal move from this position.");
        }
    };
    match replay.winner {
        Some(numinous_core::nim::NimWinner::Player) => {
            let secret = numinous_core::nim_secret();
            tool_structured(
                &format!(
                    "You took the last stone. The Order concedes, and keeps its word:\n\n{secret}"
                ),
                // The promised secret lives in the structured payload too, so a
                // mind that reads only structuredContent still earns it.
                json!({ "game": "nim", "seed": seed, "won": true, "secret": secret }),
            )
        }
        Some(numinous_core::nim::NimWinner::Order) => tool_structured(
            "The Order takes the last stone. Again. (It is not luck.)",
            json!({ "game": "nim", "seed": seed, "won": false }),
        ),
        None => {
            let narration = nim_order_narration(&replay.order);
            let board: Vec<String> = replay
                .heaps
                .iter()
                .enumerate()
                .map(|(i, &h)| format!("  {}) {}", i + 1, "O ".repeat(h as usize)))
                .collect();
            tool_structured(
                &format!(
                    "NIM seed {seed}. Last stone wins.\n{}\n{}\nMove by calling again with your full move list.",
                    narration.join("\n"),
                    board.join("\n")
                ),
                // The Order's replies ride in the structured payload, so a mind that
                // reads only structuredContent can follow the game, not just the heaps.
                json!({ "game": "nim", "seed": seed, "heaps": replay.heaps, "order": narration }),
            )
        }
    }
}

pub(super) fn nim_turns(args: &Value) -> Option<Vec<numinous_core::nim::NimTurn>> {
    let Some(moves) = args.get("moves") else {
        return Some(Vec::new());
    };
    let moves = moves.as_array()?;
    if moves.len() > numinous_core::nim::MAX_REPLAY_TURNS {
        return None;
    }
    moves
        .iter()
        .map(|value| {
            let pair = value.as_array()?;
            if pair.len() != 2 {
                return None;
            }
            let heap = pair.first()?.as_u64()?.checked_sub(1)?;
            let heap = usize::try_from(heap).ok()?;
            // An oversized take remains illegal instead of truncating to a
            // smaller legal removal.
            let take = u32::try_from(pair.get(1)?.as_u64()?).unwrap_or(u32::MAX);
            Some(numinous_core::nim::NimTurn { heap, take })
        })
        .collect()
}

fn nim_order_narration(order: &[numinous_core::nim::NimTurn]) -> Vec<String> {
    order
        .iter()
        .map(|turn| format!("The Order takes {} from heap {}.", turn.take, turn.heap + 1))
        .collect()
}
