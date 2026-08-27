//! MCP projections for the seeded puzzle tools.
//!
//! Core owns puzzle generation, rules, legality, grading, and seeded truth.
//! This module owns bounded request parsing and structured presentation.

use crate::progress::{effective_seed, load_journey};
use crate::{tool_error, tool_structured};
use serde_json::{Value, json};

/// The `crack` tool: replay the guess history against the hidden code.
pub(super) fn crack_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    crack_tool_at_level(args, load_journey(journey_file).level())
}

pub(super) fn crack_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let digits = match args.get("digits") {
        None => 4,
        Some(value) => {
            let Some(value) = value.as_u64() else {
                return tool_error("Code length must be a positive integer.");
            };
            let Ok(value) = usize::try_from(value) else {
                return tool_error("Code length is too large.");
            };
            value
        }
    };
    if !numinous_core::supports_code_length(digits) {
        return tool_error(&format!(
            "Codes run {} to {} digits.",
            numinous_core::MIN_CODE_DIGITS,
            numinous_core::MAX_CODE_DIGITS
        ));
    }
    if digits > 4 && level < 5 {
        return tool_error("Five-digit codes open at LV 5. Play more; the lock knows.");
    }
    let secret = numinous_core::secret_code(seed, digits);
    let clue = numinous_core::hint(&secret);
    let attempts = 8usize;
    let guesses: Vec<String> = args
        .get("guesses")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if guesses.is_empty() {
        return tool_structured(
            &format!(
                "THE BOMB (seed {seed}). A {digits}-digit code, {attempts} tries.\nClue: {clue}\nCall again with your full guesses list."
            ),
            json!({ "game": "crack", "seed": seed, "digits": digits, "attempts": attempts, "clue": clue }),
        );
    }
    let mut lines = Vec::new();
    // The per-guess locked/loose signal is the whole game, so it rides in the
    // structured payload too, not only in the text a structured-content client
    // would drop.
    let mut feedback_rows = Vec::new();
    for (i, raw) in guesses.iter().take(attempts).enumerate() {
        let guess: Vec<u8> = raw
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != digits {
            return tool_error(&format!("Guess {} is not {digits} digits: {raw:?}", i + 1));
        }
        let feedback = numinous_core::grade(&secret, &guess);
        feedback_rows.push(json!({
            "guess": raw,
            "locked": feedback.locked,
            "loose": feedback.loose,
        }));
        if feedback.locked == digits {
            let spare = (attempts - i - 1) as i64;
            return tool_structured(
                &format!(
                    "{}\nDEFUSED on try {} with {spare} to spare. You cracked it.",
                    lines.join("\n"),
                    i + 1
                ),
                json!({ "game": "crack", "seed": seed, "defused": true, "attemptsToSpare": spare, "feedback": feedback_rows }),
            );
        }
        lines.push(format!(
            "{raw}: {} locked, {} loose",
            feedback.locked, feedback.loose
        ));
    }
    if guesses.len() >= attempts {
        let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
        return tool_structured(
            &format!("{}\nBOOM. It was {code}.", lines.join("\n")),
            json!({ "game": "crack", "seed": seed, "defused": false, "code": code, "feedback": feedback_rows }),
        );
    }
    tool_structured(
        &format!(
            "{}\n{} tries left. Clue: {clue}",
            lines.join("\n"),
            attempts - guesses.len()
        ),
        json!({ "game": "crack", "seed": seed, "triesLeft": attempts - guesses.len(), "clue": clue, "feedback": feedback_rows }),
    )
}

/// The `seti` tool: present the scan, or grade the pointed dish.
pub(super) fn seti_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    seti_tool_at_level(args, load_journey(journey_file).level())
}

pub(super) fn seti_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let channels = args.get("channels").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(3..=8).contains(&channels) {
        return tool_error("Scans run 3 to 8 channels.");
    }
    if channels > 4 && level < 7 {
        return tool_error("Wider scans open at LV 7. Keep listening.");
    }
    let scan = numinous_core::build_scan(seed, channels);
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
            let correct = letter == Some(scan.answer);
            let verdict = if correct {
                "Contact. That trace counts the primes; nature does not."
            } else {
                "Static. The mind was elsewhere."
            };
            tool_structured(
                &format!(
                    "{verdict} The signal was {} at {}.",
                    scan.answer, scan.answer_frequency
                ),
                json!({
                    "game": "seti",
                    "seed": seed,
                    "correct": correct,
                    "answer": scan.answer.to_string(),
                    "answerFrequency": scan.answer_frequency,
                    "why": verdict,
                }),
            )
        }
        None => {
            let traces: Vec<String> = scan
                .channels
                .iter()
                .map(|c| format!("{})  {:>10}  |{}|", c.letter, c.frequency, c.trace))
                .collect();
            // The channels a mind must read to answer ride in the structured
            // payload too, so the scan is not lost on a structured-content
            // client. The trace is the puzzle, never text-only.
            let channel_rows: Vec<Value> = scan
                .channels
                .iter()
                .map(|c| json!({ "letter": c.letter.to_string(), "frequency": c.frequency, "trace": c.trace }))
                .collect();
            tool_structured(
                &format!(
                    "THE SKY (seed {seed}). One of these channels is a mind.\n{}\nCall again with your guess letter.",
                    traces.join("\n")
                ),
                json!({ "game": "seti", "seed": seed, "channels": channel_rows }),
            )
        }
    }
}

/// The `aliens` tool: receive a transmission, or answer in their base.
pub(super) fn aliens_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let message = numinous_core::alien_message(seed.wrapping_add(round), 5);
    let shown: Vec<String> = message
        .terms
        .iter()
        .map(|&t| numinous_core::to_base(t, message.base))
        .collect();
    let base_note = if message.base == 10 {
        String::new()
    } else {
        format!(" They count in base {}.", message.base)
    };
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let cleaned: String = guess.chars().filter(char::is_ascii_alphanumeric).collect();
            let correct = u64::from_str_radix(&cleaned, message.base).ok() == Some(message.answer);
            let answer = numinous_core::to_base(message.answer, message.base);
            let verdict = if correct { "Contact." } else { "Silence." };
            tool_structured(
                &format!(
                    "{verdict} It was {answer} ({}).\n{}",
                    message.name, message.explanation
                ),
                // The explanation of the sequence is the teaching, so it rides
                // in structuredContent too, not only in the dropped text block.
                json!({ "game": "aliens", "seed": seed, "round": round, "correct": correct, "answer": answer, "name": message.name, "why": message.explanation }),
            )
        }
        None => tool_structured(
            &format!(
                "A transmission (seed {seed}, signal {round}):{base_note}\n  {}, ...?\nCall again with the next term, written in their base.",
                shown.join(", ")
            ),
            json!({ "game": "aliens", "seed": seed, "round": round, "terms": shown, "base": message.base }),
        ),
    }
}

/// Convert the validated JSON transport shape into the core request type.
pub(super) fn gauntlet_answers_from_json(answers: &Value) -> numinous_core::GauntletAnswers {
    let bites = answers
        .get("bites")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .filter_map(|n| usize::try_from(n - 1).ok())
                .collect()
        })
        .unwrap_or_default();
    let shape = answers
        .get("shape")
        .and_then(Value::as_str)
        .and_then(|guess| guess.trim().chars().next());
    let sky = answers
        .get("sky")
        .and_then(Value::as_str)
        .and_then(|guess| guess.trim().chars().next());
    let wires = answers
        .get("wires")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|raw| {
                    raw.chars()
                        .filter(char::is_ascii_digit)
                        .map(|digit| digit as u8 - b'0')
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    numinous_core::GauntletAnswers {
        bites,
        shape,
        sky,
        wires,
    }
}

/// The `gauntlet` tool: present all four stages, or grade the whole run.
pub(super) fn gauntlet_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let puzzle = numinous_core::GauntletPuzzle::new(seed);
    let Some(answers) = args.get("answers") else {
        let choices: Vec<String> = puzzle
            .shape
            .choices
            .iter()
            .map(|c| format!("{}) {}", c.letter, c.title))
            .collect();
        let traces: Vec<String> = puzzle
            .sky
            .channels
            .iter()
            .map(|c| format!("{})  {:>10}  |{}|", c.letter, c.frequency, c.trace))
            .collect();
        let sky_rows: Vec<Value> = puzzle
            .sky
            .channels
            .iter()
            .map(|c| json!({ "letter": c.letter.to_string(), "frequency": c.frequency, "trace": c.trace }))
            .collect();
        return tool_structured(
            &format!(
                "THE GAUNTLET (seed {seed}). Four stages; clean stages build your combo.\n\nSTAGE 1  MUNCH: {}\n{}\nSTAGE 2  THE SHAPE:\n{}\n{}\nSTAGE 3  THE SKY:\n{}\nSTAGE 4  THE BOMB: four digits, five wires. Clue: {}\n\nCall again with answers: bites, shape, sky, wires.",
                puzzle.munch.rule.describe(),
                numinous_core::board_text(&puzzle.munch),
                puzzle.shape.art,
                choices.join("\n"),
                traces.join("\n"),
                puzzle.bomb_hint()
            ),
            // The whole four-stage puzzle rides in structuredContent, so a mind
            // on a structured-content client can actually play it, not just read
            // that four stages exist.
            json!({
                "game": "gauntlet",
                "seed": seed,
                "stages": numinous_core::GAUNTLET_STAGES,
                "munch": { "rule": puzzle.munch.rule.describe(), "board": numinous_core::board_text(&puzzle.munch) },
                "shape": { "art": puzzle.shape.art, "choices": choices },
                "sky": sky_rows,
                "bomb": { "clue": puzzle.bomb_hint() }
            }),
        );
    };
    let grade = puzzle.grade(&gauntlet_answers_from_json(answers));
    let lines = grade.reveal_lines(&puzzle);
    let scores = grade.stage_scores();
    let total = grade.total();
    let clears = grade.clean_count();
    tool_structured(
        &format!(
            "{}\n\nRUN COMPLETE  {clears}/4 clean  TOTAL {total}  (gauntlet seed:{seed})",
            lines.join("\n")
        ),
        // The per-stage reveals (what the shape, signal, and code were) ride in
        // the structured payload, so the run teaches on any client.
        json!({ "game": "gauntlet", "seed": seed, "total": total, "clean": clears, "stageScores": scores, "reveals": lines }),
    )
}
