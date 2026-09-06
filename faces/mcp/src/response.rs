//! Compact projection for complete typed MCP tool results.

use serde_json::{Value, json};

use super::{room_door, show, workspace::compact_workspace_summary};

/// Compact mode removes only prose that duplicates a complete typed result.
/// Guiding errors and text-only tools stay untouched because their text is the
/// result, not presentation overhead.
pub(super) fn apply_response_mode(
    name: &str,
    response_mode: Option<&str>,
    mut result: Value,
) -> Value {
    if response_mode != Some("compact")
        || result.get("isError").and_then(Value::as_bool) == Some(true)
        || result.get("structuredContent").is_none()
    {
        return result;
    }

    let Some(summary) = compact_result_summary(name, &result["structuredContent"]) else {
        return result;
    };
    let Some(current_text) = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
    else {
        return result;
    };
    if summary.len() >= current_text.len() {
        return result;
    }
    if let Some(first) = result
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .and_then(|content| content.first_mut())
    {
        *first = json!({
            "type": "text",
            "text": summary,
        });
    }
    result
}

fn compact_result_summary(name: &str, structured: &Value) -> Option<String> {
    match name {
        // A compact reply that lists bare ids sends a reader looking them up
        // in the very catalog the short mode exists to spare them, so the
        // prose names its starters the same way the structured array does.
        "list_rooms" => room_door::compact_summary(structured),
        "watch_show" => show::compact_summary(structured),
        "describe_room" => {
            let mut summary = format!(
                "{} ({}) in {}. Action: {}.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("wing")?.as_str()?,
                structured.get("action")?.as_str()?
            );
            match structured
                .get("journalCue")
                .and_then(|cue| cue.get("status"))
                .and_then(Value::as_str)
            {
                Some("remembered") => summary.push_str(
                    " This local player profile kept something here; no journal text was opened.",
                ),
                Some("unavailable") => summary
                    .push_str(" The local journal could not be read; no contents were returned."),
                _ => {}
            }
            summary.push_str(" Read structuredContent for the goal, blurb, next play call, and optional journal cue.");
            Some(summary)
        }
        "play_room" => {
            let mut summary = format!(
                "{} ({}) at t={:.3}, {}x{}. Action: {}.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("t")?.as_f64()?,
                structured.get("width")?.as_u64()?,
                structured.get("height")?.as_u64()?,
                structured.get("action")?.as_str()?
            );
            if let Some(status) = structured.get("status").and_then(Value::as_str) {
                summary.push_str(&format!(" Status: {status}."));
            }
            if let Some(cells) = structured
                .get("delta")
                .and_then(|delta| delta.get("cells_changed"))
                .and_then(Value::as_u64)
            {
                summary.push_str(&format!(
                    " Touch changed {}.",
                    numinous_core::counted(cells as usize, "cell")
                ));
            }
            if let Some(temporal) = structured.get("temporal") {
                summary.push_str(&format!(
                    " Temporal from t={:.3} to t={:.3}, {} cells changed.",
                    temporal.get("fromT")?.as_f64()?,
                    temporal.get("toT")?.as_f64()?,
                    temporal.get("delta")?.get("cells_changed")?.as_u64()?
                ));
            }
            if let Some(dwell) = structured.get("dwell") {
                summary.push_str(&format!(
                    " Dwell across {} looks, {} of {} cells held still.",
                    dwell.get("looks")?.as_u64()?,
                    dwell.get("held")?.get("unchanged_cells")?.as_u64()?,
                    dwell.get("held")?.get("total_cells")?.as_u64()?
                ));
            }
            if structured.get("encounter").is_some() {
                summary.push_str(" Encounter receipt attached.");
            }
            if let Some(beat) = structured
                .get("engineeredAha")
                .and_then(|aha| aha.get("beat"))
                .and_then(Value::as_str)
            {
                summary.push_str(&format!(" Aha beat: {beat}."));
            }
            let optional_fields = match (
                structured.get("temporal").is_some(),
                structured.get("dwell").is_some(),
                structured.get("encounter").is_some(),
            ) {
                (true, true, true) => "render, temporal, dwell, encounter, ",
                (true, true, false) => "render, temporal, dwell, ",
                (true, false, true) => "render, temporal, encounter, ",
                (true, false, false) => "render, temporal, ",
                (false, true, true) => "render, dwell, encounter, ",
                (false, true, false) => "render, dwell, ",
                (false, false, true) => "render, encounter, ",
                (false, false, false) => "render, ",
            };
            summary.push_str(&format!(
                " Read structuredContent.{optional_fields}pokes, gesture, status, delta, goal, goalMet, and engineeredAha for the complete result; ask reveal_room for the explanation."
            ));
            Some(summary)
        }
        "listen_room" => {
            let mut summary = format!(
                "{} ({}) at t={:.3}: {} of {} mathematical notes returned over {:.2}s.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("t")?.as_f64()?,
                structured.get("returned_note_count")?.as_u64()?,
                structured.get("note_count")?.as_u64()?,
                structured.get("duration_seconds")?.as_f64()?
            );
            if structured.get("encounter").is_some() {
                summary.push_str(" Encounter receipt attached.");
            }
            summary.push_str(" Read structuredContent.pokes, gesture, motif, ambient_bed, notes, and sound_roles for the complete typed layers.");
            Some(summary)
        }
        "run_sim" => Some(format!(
            "{} ({}): {} Read structuredContent.params, readout, and render for the complete result.",
            structured.get("title")?.as_str()?,
            structured.get("sim")?.as_str()?,
            structured.get("readout")?.as_str()?
        )),
        "quiz" => {
            let seed = structured.get("seed")?.as_u64()?;
            let round = structured.get("round")?.as_u64()?;
            let choice_count = structured.get("choiceCount")?.as_u64()?;
            if let Some(correct) = structured.get("correct").and_then(Value::as_bool) {
                Some(format!(
                    "Quiz seed {seed}, round {round}, choices {choice_count}: {} Answer {} ({}). Read structuredContent.why for the explanation.",
                    if correct { "correct." } else { "not quite." },
                    structured.get("answer")?.as_str()?,
                    structured.get("answerTitle")?.as_str()?
                ))
            } else {
                let choices = structured.get("choices")?.as_array()?;
                Some(format!(
                    "Quiz seed {seed}, round {round}, choices {choice_count}: choose A through {}. Call quiz again with seed {seed}, round {round}, choices {choice_count}, and guess. Read structuredContent.art and choices for the complete puzzle.",
                    choices.last()?.get("letter")?.as_str()?
                ))
            }
        }
        "gauntlet" => {
            let seed = structured.get("seed")?.as_u64()?;
            if let Some(total) = structured.get("total").and_then(Value::as_i64) {
                Some(format!(
                    "Gauntlet seed {seed}: {}/4 clean, total {total}. Read structuredContent.stageScores and reveals for the complete grade.",
                    structured.get("clean")?.as_u64()?
                ))
            } else {
                Some(format!(
                    "Gauntlet seed {seed}: four stages. Call again with answers.bites, shape, sky, and wires. Read structuredContent.munch, shape, sky, and bomb for the complete run."
                ))
            }
        }
        "trophies" => Some(format!(
            "Trophy case: {} of {} earned. Read structuredContent.trophies for every name, condition, and earned state.",
            structured.get("earned")?.as_u64()?,
            structured.get("total")?.as_u64()?
        )),
        "workspace" => compact_workspace_summary(structured),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_touch_that_answers_one_cell_says_cell_not_cells() {
        // Same class as the wing doorway an external playtester reported. A
        // poke that moves a single cell is ordinary play, and this sentence is
        // prose rather than a data label, so it has to agree with its number.
        let one = json!({
            "title": "Times Tables", "room": "times-tables", "t": 0.25,
            "width": 40, "height": 20, "action": "TURN THE DIAL",
            "delta": { "cells_changed": 1 }
        });
        let summary = super::compact_result_summary("play_room", &one).expect("a summary");
        assert!(summary.contains("Touch changed 1 cell."), "{summary}");
        assert!(!summary.contains("1 cells"), "{summary}");

        let many = json!({
            "title": "Times Tables", "room": "times-tables", "t": 0.25,
            "width": 40, "height": 20, "action": "TURN THE DIAL",
            "delta": { "cells_changed": 7 }
        });
        let summary = super::compact_result_summary("play_room", &many).expect("a summary");
        assert!(summary.contains("Touch changed 7 cells."), "{summary}");

        // Zero stays plural, which is the other half of the rule.
        let none = json!({
            "title": "Times Tables", "room": "times-tables", "t": 0.25,
            "width": 40, "height": 20, "action": "TURN THE DIAL",
            "delta": { "cells_changed": 0 }
        });
        let summary = super::compact_result_summary("play_room", &none).expect("a summary");
        assert!(summary.contains("Touch changed 0 cells."), "{summary}");
    }
}
