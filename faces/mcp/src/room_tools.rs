//! MCP projections for describing, hearing, playing, and revealing rooms.
//!
//! Core owns room discovery, veil rules, rendering, sound, goals, grading, and
//! mathematical truth. This module owns their bounded JSON-facing projection.

use crate::encounter::{
    action_json as encounter_action_json, delta_counts as encounter_delta_counts,
    dwell_counts as encounter_dwell_counts, issue as issue_encounter, issue_receipt,
    listen_action as encounter_listen_action, listen_action_json,
    listen_result as encounter_listen_result, play_action as encounter_play_action,
    play_result as encounter_play_result, receipt_json, request as encounter_request,
};
use crate::flagship_aha::{
    FlagshipAhaRequest, parse_flagship_aha_request, project_flagship_aha,
    render_engineered_aha_overlay,
};
use crate::progress::{journal_path, load_journey};
use crate::room_input::{gesture_json, parse_room_inputs, render_room_observation, room_status_at};
use crate::temporal::{
    self, dwell_evidence_json, evidence_json as temporal_evidence_json, render_delta_json,
};
use crate::{audible, journal, tool_error, tool_structured, unknown_room};
use numinous_broadcast::{
    PLAY_ROOM_DEFAULT_HEIGHT as DEFAULT_HEIGHT, PLAY_ROOM_DEFAULT_WIDTH as DEFAULT_WIDTH,
    PLAY_ROOM_MAX_HEIGHT as MAX_TOOL_HEIGHT, PLAY_ROOM_MAX_WIDTH as MAX_TOOL_WIDTH,
};
use numinous_core::{Canvas, room_by_id};
use serde_json::{Value, json};

pub(super) fn describe_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let mut result = describe_room_tool_for_journey(args, &load_journey(journey_file));
    let Some(room) = result
        .get("structuredContent")
        .and_then(|structured| structured.get("room"))
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return result;
    };
    let Some((cue, cue_text)) = journal::room_cue(&journal_path(), &room) else {
        return result;
    };
    result["structuredContent"]["journalCue"] = cue;
    if let Some(text) = result
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .and_then(|content| content.first_mut())
        .and_then(|block| block.get_mut("text"))
        && let Some(existing) = text.as_str()
    {
        *text = Value::String(format!("{existing}\n\n{cue_text}"));
    }
    result
}

/// Find a room the way the terminal does: the catalog always, and the
/// unlisted ones for a journey the veil admits.
///
/// The gate itself lives in core so both faces read one rule, but this face
/// used to skip the second half entirely: a learner who could open the
/// hidden room from the terminal was told over MCP that it does not exist.
/// One player, one standing, two answers.
fn find_room_for(
    id: &str,
    journey: &numinous_core::Journey,
) -> Option<Box<dyn numinous_core::Room>> {
    room_by_id(id).or_else(|| {
        numinous_core::behind_the_veil(journey)
            .then(|| numinous_core::hidden_room_by_id(id))
            .flatten()
    })
}

pub(super) fn describe_room_tool_for_journey(
    args: &Value,
    journey: &numinous_core::Journey,
) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match find_room_for(id, journey) {
        Some(room) => {
            let m = room.meta();
            let goal_line = room
                .goal()
                .map(|goal| format!("\nGoal: {goal}"))
                .unwrap_or_default();
            let text = format!(
                "{} ({})\nWing: {}\nAction: {}{goal_line}\n\n{}\n\nPlay this room before asking reveal_room for its explanation.",
                m.title,
                m.id,
                m.wing,
                numinous_core::room_action(room.as_ref()),
                m.blurb,
            );
            let structured = json!({
                "room": m.id,
                "title": m.title,
                "wing": m.wing,
                "action": numinous_core::room_action(room.as_ref()),
                "goal": room.goal(),
                "blurb": m.blurb,
                "next": {
                    "tool": "play_room",
                    "id": m.id,
                },
            });
            tool_structured(&text, structured)
        }
        // Not every name is a room. A few answer anyway, and a few answer
        // only those with standing.
        None => match numinous_core::akousma(id) {
            Some(whisper) => tool_structured(
                whisper,
                json!({ "kind": "whisper", "id": id, "text": whisper }),
            ),
            // The veil rule is core's: this face once held its own gate at
            // 28 sparks while the terminal held the documented rank, and
            // the same listener was inside on one face and refused here.
            None if numinous_core::behind_the_veil(journey) => {
                match numinous_core::deep_akousma(id) {
                    Some(whisper) => tool_structured(
                        whisper,
                        json!({ "kind": "whisper", "id": id, "text": whisper }),
                    ),
                    None => tool_error(&unknown_room(id)),
                }
            }
            None => tool_error(&unknown_room(id)),
        },
    }
}

/// The nearest note name (twelve-tone, A4 = 440 Hz) for a frequency.
pub(super) fn note_name(freq: f32) -> String {
    if freq <= 0.0 {
        return "-".to_string();
    }
    const NAMES: [&str; 12] = [
        "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
    ];
    let semitones_from_a4 = (12.0 * (freq / 440.0).log2()).round() as i64;
    let index = semitones_from_a4.rem_euclid(12) as usize;
    // A4 is nine semitones above C4; convert to octave numbering.
    let octave = 4 + (semitones_from_a4 + 9).div_euclid(12);
    format!("{}{}", NAMES[index], octave)
}

/// Project one stable stereo room bed into bounded, typed protocol evidence.
///
/// The projection exposes arrangement intent and objective signal features,
/// never PCM samples or a machine-local file reference. Signal features catch
/// engineering regressions but do not claim that a bed sounds pleasant.
fn ambient_bed_value(motif: numinous_core::Motif, include_events: bool) -> Result<Value, String> {
    let arrangement = motif.arrangement();
    if arrangement.notes.len() > numinous_core::MAX_ROOM_BED_EVENTS {
        return Err(format!(
            "Room bed contains {} events; the protocol limit is {}.",
            arrangement.notes.len(),
            numinous_core::MAX_ROOM_BED_EVENTS
        ));
    }

    let duration_seconds = arrangement.steps as f64 * f64::from(arrangement.step_seconds);
    let mut value = json!({
        "schema": "numinous.room-bed.events",
        "schema_version": 1,
        "renderer": "numinous.chiptune.stereo.v1",
        "source_sample_rate_hz": numinous_core::ROOM_BED_SOURCE_RATE,
        "channels": 2,
        "steps": arrangement.steps,
        "step_seconds": arrangement.step_seconds,
        "duration_seconds": duration_seconds,
        "event_count": arrangement.notes.len(),
        "events_included": include_events,
    });

    // Compact spectrum is always available: render once and share with detail.
    let samples = arrangement.render_stereo(numinous_core::ROOM_BED_SOURCE_RATE);
    let spectrum =
        numinous_core::arrangement_spectrum(&samples, numinous_core::ROOM_BED_SOURCE_RATE);
    value["spectrum"] = json!({
        "schema": "numinous.spectrum.bands",
        "schema_version": 1,
        "band_count": numinous_core::BAND_COUNT,
        "names": numinous_core::BAND_NAMES,
        "levels": spectrum.to_vec(),
    });

    if include_events {
        let events = arrangement
            .notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                json!({
                    "index": index + 1,
                    "frequency_hz": note.frequency,
                    "start_step": note.start_step,
                    "step_count": note.step_count,
                    "start_seconds": note.start_step as f64 * f64::from(arrangement.step_seconds),
                    "duration_seconds": note.step_count as f64 * f64::from(arrangement.step_seconds),
                    "voice": note.voice.id(),
                    "level": note.level,
                    "pan": note.pan,
                })
            })
            .collect::<Vec<_>>();
        let metrics = numinous_core::stereo_signal_metrics(&samples);
        value["events"] = json!(events);
        value["signal_metrics"] = json!({
            "scope": "pre_master_room_bed",
            "interpretation": "Engineering regression evidence only; not a pleasantness score.",
            "frame_count": metrics.frame_count,
            "trailing_samples": metrics.trailing_samples,
            "non_finite_samples": metrics.non_finite_samples,
            "subnormal_samples": metrics.subnormal_samples,
            "clipped_samples": metrics.clipped_samples,
            "peak": metrics.peak,
            "rms": metrics.rms,
            "crest_db": metrics.crest_db,
            "left_rms": metrics.left_rms,
            "right_rms": metrics.right_rms,
            "channel_balance_db": metrics.channel_balance_db,
            "left_dc": metrics.left_dc,
            "right_dc": metrics.right_dc,
            "correlation": metrics.correlation,
            "side_to_mid_db": metrics.side_to_mid_db,
            "max_step": metrics.max_step,
            "zero_sample_fraction": metrics.zero_sample_fraction,
        });
    }

    Ok(value)
}

/// The `listen_room` tool: the room's sound as notation a mind can read.
pub(super) fn listen_room_tool(args: &Value) -> Value {
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    if !(0.0..1.0).contains(&t) {
        return tool_error("Argument 't' must be a phase in [0,1).");
    }
    let include_ambient_events = match args.get("ambient_detail").and_then(Value::as_str) {
        None | Some("summary") => false,
        Some("events") => true,
        Some(_) => return tool_error("Argument 'ambient_detail' must be 'summary' or 'events'."),
    };
    let inputs = match parse_room_inputs(args) {
        Ok(inputs) => inputs,
        Err(message) => return tool_error(&message),
    };
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let room = numinous_core::room_by_id_with(id, variation);
    let Some(room) = room else {
        return tool_error(&unknown_room(id));
    };
    let poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, t);
    let accepted_inputs = if inputs.gesture.is_empty() {
        poke_inputs.as_slice()
    } else {
        inputs.gesture.as_slice()
    };
    let spec = room.sound_input(t, accepted_inputs);
    let note_count = spec.notes.len();
    let mut lines = vec![format!(
        "{} at t={t:.3}: {:.1}s of sound, {} notes.",
        room.meta().title,
        spec.duration,
        note_count
    )];
    let ambient_motif = room.motif().map(|motif| {
        let notation = motif.notation();
        lines.push(format!(
            "Ambient motif: {} at {} BPM, {}. It encodes: {}.",
            motif.key,
            motif.tempo,
            notation.join(" "),
            motif.encodes
        ));
        json!({
            "key": motif.key,
            "tempo_bpm": motif.tempo,
            "notation": notation,
            "encodes": motif.encodes,
        })
    });
    let ambient_bed = match room.motif() {
        Some(motif) => match ambient_bed_value(motif, include_ambient_events) {
            Ok(value) => Some(value),
            Err(message) => return tool_error(&message),
        },
        None => None,
    };
    if let Some(bed) = ambient_bed.as_ref() {
        lines.push(format!(
            "Stable stereo room bed: {:.2}s, {} arranged events at {} Hz.{}",
            bed["duration_seconds"].as_f64().unwrap_or_default(),
            bed["event_count"].as_u64().unwrap_or_default(),
            numinous_core::ROOM_BED_SOURCE_RATE,
            if include_ambient_events {
                " Complete events and pre-master signal metrics follow in structuredContent."
            } else {
                " Request ambient_detail=events for the complete bounded event projection."
            }
        ));
    }
    let mut structured_notes = Vec::new();
    if note_count > 0 {
        lines.push("Mathematical sonification:".to_string());
    }
    for (i, note) in spec.notes.iter().take(64).enumerate() {
        let name = note_name(note.freq);
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s  for {:.2}s  amp {:.2}",
            i + 1,
            note.freq,
            name,
            note.start,
            note.dur,
            note.amp
        ));
        structured_notes.push(json!({
            "index": i + 1,
            "frequency_hz": note.freq,
            "name": name,
            "start_seconds": note.start,
            "duration_seconds": note.dur,
            "amplitude": note.amp,
        }));
    }
    if note_count > 64 {
        lines.push(format!("  ... and {} more notes.", note_count - 64));
    }
    // The room's own sonification, as sound rather than as a table of it. This
    // is the mathematical voice, not the ambient bed: what the room is doing
    // right now at this phase, under this hand.
    let audible = match audible::requested(args) {
        Ok(true) => match audible::block(&spec) {
            Ok(rendered) => Some(rendered),
            Err(message) => return tool_error(&message),
        },
        Ok(false) => None,
        Err(message) => return tool_error(&message),
    };
    if audible.is_some() {
        lines.push(
            "A WAV of this room at this phase follows as an audio \
             attachment. Whether your client can surface it as sound is \
             its answer to give, not ours; if it cannot, the notation \
             above is the whole of what arrived."
                .to_string(),
        );
    }
    let mut structured = json!({
        "room": room.meta().id,
        "title": room.meta().title,
        "t": t,
        "variation": variation,
        "audio": audible.as_ref().map(|(_, described)| described.clone()),
        "pokes": inputs.pokes,
        "gesture": if inputs.gesture.is_empty() { Value::Null } else { gesture_json(&inputs.gesture) },
        "duration_seconds": spec.duration,
        "note_count": note_count,
        "returned_note_count": structured_notes.len(),
        "truncated": note_count > 64,
        "motif": ambient_motif,
        "ambient_bed": ambient_bed,
        "notes": structured_notes,
        "sound_roles": {
            "ambient_motif": { "field": "motif" },
            "ambient_arrangement": { "field": "ambient_bed" },
            "mathematical_sonification": { "field": "notes" },
        },
    });
    if want_receipt {
        let audio_asked = args.get("audio").and_then(Value::as_bool).unwrap_or(false);
        let action = encounter_listen_action(
            room.meta().id,
            t,
            variation,
            include_ambient_events,
            audio_asked,
            &inputs.pokes,
            &inputs.gesture,
        );
        let motif = structured.get("motif");
        let bed = structured.get("ambient_bed");
        let result = encounter_listen_result(
            room.meta().id,
            t,
            variation,
            spec.duration.into(),
            note_count as u64,
            structured_notes.len() as u64,
            note_count > 64,
            motif
                .and_then(|value| value.get("key"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            motif
                .and_then(|value| value.get("tempo_bpm"))
                .and_then(Value::as_u64),
            motif
                .and_then(|value| value.get("encodes"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            bed.and_then(|value| value.get("duration_seconds"))
                .and_then(Value::as_f64),
            bed.and_then(|value| value.get("event_count"))
                .and_then(Value::as_u64),
            structured
                .get("audio")
                .and_then(|value| value.get("encodedBytes"))
                .and_then(Value::as_u64),
        );
        match issue_receipt(
            numinous_core::EncounterTool::ListenRoom,
            &action.canonical_bytes(),
            &result.canonical_bytes(),
        ) {
            Ok(receipt) => {
                structured["encounter"] = receipt_json(&receipt, listen_action_json(&action))
            }
            Err(message) => return tool_error(&message),
        }
        lines.push("Encounter receipt attached.".to_string());
    }
    let result = tool_structured(&lines.join("\n"), structured);
    match audible {
        Some((block, _)) => audible::attach(result, block),
        None => result,
    }
}

/// The `reveal_room` tool: optional concept + revelation (the learn surface).
pub(super) fn reveal_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    reveal_room_tool_for_journey(args, &load_journey(journey_file))
}

pub(super) fn reveal_room_tool_for_journey(
    args: &Value,
    journey: &numinous_core::Journey,
) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match find_room_for(id, journey) {
        Some(room) => {
            let room_id = room.meta().id;
            if numinous_core::is_engineered_aha_room(room_id) && !journey.has_consolidated(room_id)
            {
                return tool_error(
                    "This explanation is still closed. Play the room, commit its wager, then call play_room with aha_summon true.",
                );
            }
            if !numinous_core::is_engineered_aha_room(room_id) && !journey.visited.contains(room_id)
            {
                return tool_error(
                    "This explanation is still closed. Play the room once, then ask reveal_room again.",
                );
            }
            let cut0_by_boon = journey.chosen.contains(&format!("cut:{room_id}:0"));
            let citation =
                numinous_core::room_citation_unlocked(room_id, journey.level(), cut0_by_boon);
            let mut body = numinous_core::explain_text(room.meta().id, room.reveal());
            let mut structured_cuts = Vec::new();
            for (i, cut) in room.deep_cuts().iter().enumerate() {
                let need = numinous_core::CUT_LEVELS
                    .get(i)
                    .copied()
                    .unwrap_or(u32::MAX);
                let by_boon = journey.chosen.contains(&format!("cut:{room_id}:{i}"));
                if journey.level() >= need || by_boon {
                    body.push_str(&format!("\n\nDeeper: {cut}"));
                    structured_cuts.push(json!({
                        "index": i,
                        "status": "available",
                        "unlock_level": need,
                        "text": cut,
                    }));
                } else {
                    body.push_str(&format!("\n\nLOCKED: a deeper cut opens at LV {need}."));
                    structured_cuts.push(json!({
                        "index": i,
                        "status": "locked",
                        "unlock_level": need,
                    }));
                    break;
                }
            }
            if let Some(citation) = citation {
                body = format!("{body}\n\n{citation}");
            }
            let mut structured = json!({
                "room": room.meta().id,
                "title": room.meta().title,
                "reveal": room.reveal(),
                "deep_cuts": structured_cuts,
            });
            if let Some(concept) = room.concept() {
                structured["concept"] = json!(concept);
            }
            if let Some(citation) = citation {
                structured["citation"] = json!(citation);
            }
            tool_structured(&body, structured)
        }
        // The Cairn is not a room but a mind pausing there naturally reaches for
        // reveal; point it at the right door rather than saying it does not exist.
        None if id == "cairn" => {
            let reveal = "The Cairn is not a room but a message across time: use the `cairn` tool. \
                          Call it with a `seed` to receive a stone a mind before you left, factor its \
                          semiprime length, then call again with that `width` to read what was left. \
                          At level 42 you may `leave` one true thing of your own.";
            tool_structured(
                reveal,
                json!({ "kind": "cairn", "tool": "cairn", "reveal": reveal }),
            )
        }
        None => tool_error(&unknown_room(id)),
    }
}

pub(super) fn play_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    play_room_tool_for_journey(args, &load_journey(journey_file))
}

fn projected_play_status(
    room_status: Option<String>,
    aha_request: FlagshipAhaRequest,
    engineered_aha: Option<&Value>,
) -> Option<String> {
    let aha_beat = engineered_aha
        .and_then(|value| value.get("beat"))
        .and_then(Value::as_str);
    let use_aha_status = aha_request.uses_generation_args()
        || matches!(
            aha_beat,
            Some("prime" | "morph" | "confirm" | "consolidated")
        );
    if use_aha_status {
        engineered_aha
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or(room_status)
    } else {
        room_status
    }
}

pub(super) fn play_room_tool_for_journey(args: &Value, journey: &numinous_core::Journey) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let canonical_id = numinous_core::canonical_room_id(id);
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let width = args
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_WIDTH) as usize;
    let height = args
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_HEIGHT) as usize;
    // Schema validation is the primary gate; this rejects hostile sizes if a
    // future call path ever reaches the tool without the catalog check.
    if !(1..=MAX_TOOL_WIDTH as usize).contains(&width)
        || !(1..=MAX_TOOL_HEIGHT as usize).contains(&height)
    {
        return tool_error(&format!(
            "Canvas size must be between 1x1 and {MAX_TOOL_WIDTH}x{MAX_TOOL_HEIGHT}."
        ));
    }
    let temporal_pair = match temporal::request(args, width, height) {
        Ok(pair) => pair,
        Err(message) => return tool_error(&message),
    };
    let dwell_window = match temporal::dwell_request(args, width, height) {
        Ok(window) => window,
        Err(message) => return tool_error(&message),
    };
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let inputs = match parse_room_inputs(args) {
        Ok(inputs) => inputs,
        Err(message) => return tool_error(&message),
    };
    let aha_request = match parse_flagship_aha_request(args, canonical_id) {
        Ok(request) => request,
        Err(message) => return tool_error(&message),
    };

    let room = if variation == 0 {
        // The same veil door describe and reveal use: a hidden room is
        // unlisted, not nonexistent, and a learner the terminal admits is
        // the same learner here. Variation stays a catalog contract, which
        // is the terminal's rule too.
        find_room_for(id, journey)
    } else {
        numinous_core::room_by_id_with(id, variation)
    };

    match room {
        Some(room) => {
            let mut canvas = Canvas::new(width, height);
            let poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, t);
            let accepted_inputs = if inputs.gesture.is_empty() {
                poke_inputs.as_slice()
            } else {
                inputs.gesture.as_slice()
            };
            // A gesture trail: held rooms give it pull-and-release semantics;
            // every other room answers through the same bridge the App uses.
            render_room_observation(room.as_ref(), &mut canvas, t, accepted_inputs);
            let delta = if accepted_inputs.is_empty() {
                None
            } else {
                let mut base = Canvas::new(width, height);
                room.render(&mut base, t);
                base.delta(&canvas)
            };
            let m = room.meta();
            let action = numinous_core::room_action(room.as_ref());
            let room_status = room_status_at(room.as_ref(), t, accepted_inputs);
            let goal = room.goal();
            let goal_met = goal.is_some() && room.goal_met(t, accepted_inputs);
            let completed_actions = if inputs.gesture.is_empty() {
                inputs.pokes.len()
            } else {
                inputs
                    .gesture
                    .iter()
                    .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
                    .count()
            };
            let engineered_aha = match project_flagship_aha(
                canonical_id,
                variation,
                t,
                accepted_inputs,
                completed_actions,
                goal_met,
                aha_request,
            ) {
                Ok(value) => value,
                Err(message) => return tool_error(&message),
            };
            render_engineered_aha_overlay(canonical_id, engineered_aha.as_ref(), &mut canvas);
            // Every engineered Aha gates its answer on consolidation. A goal
            // can be visibly met before the player asks the measured gap to
            // answer, so goalMet and reveal are intentionally separate facts.
            //
            // An ordinary room used to pay a landed goal with its explanation
            // in the same reply. That reversed the promise a player is given at
            // the door, that understanding is offered later and only if they
            // ask, and it made the reward for succeeding the loss of the thing
            // they succeeded at. Landing the goal opens `reveal_room`; it does
            // not speak. The staged rooms keep answering their own summon,
            // because `aha_summon` is the player asking.
            let aha_gates_reveal = numinous_core::is_engineered_aha_room(canonical_id);
            let aha_allows_reveal = engineered_aha
                .as_ref()
                .and_then(|value| value.get("allowReveal"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let earned_reveal = (aha_gates_reveal && aha_allows_reveal).then(|| room.reveal());
            // Prefer aha footer for generation-arg visits and for prime/morph/
            // confirm/consolidated beats. Keep the room readout for a pure K5
            // goal path so the public goal and its visible status stay aligned.
            let status = projected_play_status(room_status, aha_request, engineered_aha.as_ref());
            // One observation at one phase, with the same hand. Both the
            // two-phase delta and the multi-look dwell are built from this, so
            // the two kinds of evidence cannot drift apart in how they render.
            let observe = |phase: f64| -> Result<(Canvas, Option<String>), String> {
                let phase_poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, phase);
                let phase_inputs = if inputs.gesture.is_empty() {
                    phase_poke_inputs.as_slice()
                } else {
                    inputs.gesture.as_slice()
                };
                let mut frame = Canvas::new(width, height);
                render_room_observation(room.as_ref(), &mut frame, phase, phase_inputs);
                let phase_goal_met = goal.is_some() && room.goal_met(phase, phase_inputs);
                let phase_aha = project_flagship_aha(
                    canonical_id,
                    variation,
                    phase,
                    phase_inputs,
                    completed_actions,
                    phase_goal_met,
                    aha_request,
                )?;
                render_engineered_aha_overlay(canonical_id, phase_aha.as_ref(), &mut frame);
                let phase_status = projected_play_status(
                    room_status_at(room.as_ref(), phase, phase_inputs),
                    aha_request,
                    phase_aha.as_ref(),
                );
                Ok((frame, phase_status))
            };
            let temporal_evidence = if let Some(pair) = temporal_pair {
                let (from_canvas, from_status) = match observe(pair.from_t()) {
                    Ok(observation) => observation,
                    Err(message) => return tool_error(&message),
                };
                let temporal_delta = from_canvas
                    .delta(&canvas)
                    .expect("temporal observations use identical dimensions");
                Some((pair, from_status, from_canvas.to_text(), temporal_delta))
            } else {
                None
            };
            // Staying is its own act. The room draws once per look and reports
            // what refused to move, so the reward for returning is a
            // measurement the player extracted rather than a paragraph the
            // room volunteered.
            let dwell_evidence = if let Some(window) = dwell_window.as_ref() {
                let mut frames = Vec::with_capacity(window.looks());
                let mut statuses = Vec::with_capacity(window.looks());
                for &phase in window.phases() {
                    match observe(phase) {
                        Ok((frame, phase_status)) => {
                            frames.push(frame);
                            statuses.push(phase_status);
                        }
                        Err(message) => return tool_error(&message),
                    }
                }
                let held = Canvas::invariant(&frames)
                    .expect("a dwell window renders every look at identical dimensions");
                Some((window, held, statuses))
            } else {
                None
            };
            let status_line = status
                .as_ref()
                .map(|readout| format!("\nStatus: {readout}"))
                .unwrap_or_default();
            let touch_line = delta
                .as_ref()
                .map(|d| {
                    format!(
                        "\nTouch: {} of {} cells answered",
                        d.cells_changed, d.total_cells
                    )
                })
                .unwrap_or_default();
            let goal_line = goal
                .map(|objective| format!("\nGoal: {objective}"))
                .unwrap_or_default();
            let aha_line = engineered_aha
                .as_ref()
                .and_then(|value| value.get("beat"))
                .and_then(Value::as_str)
                .map(|beat| format!("\nAha beat: {beat}"))
                .unwrap_or_default();
            let reveal_line = earned_reveal
                .map(|reveal| format!("\nReveal: {reveal}"))
                .unwrap_or_default();
            let render = canvas.to_text();
            let destination_text = format!(
                "{} at t={t:.3}:\nAction: {action}{goal_line}{status_line}{aha_line}{touch_line}{reveal_line}\n\n{render}",
                m.title,
            );
            let text = temporal_evidence.as_ref().map_or_else(
                || destination_text.clone(),
                |(pair, from_status, from_render, temporal_delta)| {
                    let from_status_line = from_status
                        .as_ref()
                        .map(|readout| format!("\nStatus: {readout}"))
                        .unwrap_or_default();
                    format!(
                        "{} from t={:.3}:{from_status_line}\n\n{from_render}\n{destination_text}\nTemporal: {} of {} cells changed from t={:.3} to t={:.3}",
                        m.title,
                        pair.from_t(),
                        temporal_delta.cells_changed,
                        temporal_delta.total_cells,
                        pair.from_t(),
                        pair.to_t(),
                    )
                },
            );
            let mut structured = json!({
                "room": m.id,
                "title": m.title,
                "t": t,
                "width": width,
                "height": height,
                "variation": variation,
                "pokes": inputs.pokes,
                "gesture": if inputs.gesture.is_empty() { Value::Null } else { gesture_json(&inputs.gesture) },
                "action": action,
                "status": status,
                "goal": goal,
                "goalMet": goal_met,
                "reveal": earned_reveal,
                "engineeredAha": engineered_aha,
                // The destination picture remains authoritative for every
                // existing client. An optional origin lives only in temporal.
                "render": render,
                "delta": delta.map(render_delta_json),
            });
            if let Some((pair, from_status, from_render, temporal_delta)) = temporal_evidence {
                structured["temporal"] =
                    temporal_evidence_json(pair, from_status, from_render, temporal_delta);
            }
            let text = match dwell_evidence.as_ref() {
                Some((_, held, _)) => format!(
                    "{text}\nDwell: across {} looks, {} of {} cells held still{}",
                    held.looks,
                    held.unchanged_cells,
                    held.total_cells,
                    held.changed_region
                        .map(|_| format!(
                            ", and {} stayed dark inside the region that moved",
                            held.never_ink_in_changed_region
                        ))
                        .unwrap_or_else(|| ", and nothing moved at all".to_string()),
                ),
                None => text,
            };
            if let Some((window, held, statuses)) = dwell_evidence {
                structured["dwell"] = dwell_evidence_json(window, &held, statuses);
            }
            if want_receipt {
                let aha_beat = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("beat"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let aha_grade = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("graded"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let aha_allow_reveal = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("allowReveal"))
                    .and_then(Value::as_bool);
                let touch = structured.get("delta").and_then(|counts| {
                    Some(encounter_delta_counts(
                        counts.get("cells_changed")?.as_u64()?,
                        counts.get("ink_added")?.as_u64()?,
                        counts.get("ink_removed")?.as_u64()?,
                        counts.get("ink_reshaped")?.as_u64()?,
                        counts.get("total_cells")?.as_u64()?,
                    ))
                });
                let temporal_counts = structured.get("temporal").and_then(|temporal| {
                    let counts = temporal.get("delta")?;
                    Some(encounter_delta_counts(
                        counts.get("cells_changed")?.as_u64()?,
                        counts.get("ink_added")?.as_u64()?,
                        counts.get("ink_removed")?.as_u64()?,
                        counts.get("ink_reshaped")?.as_u64()?,
                        counts.get("total_cells")?.as_u64()?,
                    ))
                });
                let dwell_counts = structured.get("dwell").and_then(|dwell| {
                    let held = dwell.get("held")?;
                    Some(encounter_dwell_counts(
                        dwell.get("looks")?.as_u64()?,
                        held.get("unchanged_cells")?.as_u64()?,
                        held.get("never_ink")?.as_u64()?,
                        held.get("always_ink")?.as_u64()?,
                        held.get("never_ink_in_changed_region")?.as_u64()?,
                        held.get("never_ink_enclosed")?.as_u64()?,
                        held.get("total_cells")?.as_u64()?,
                    ))
                });
                let receipt_action = encounter_play_action(
                    m.id,
                    t,
                    width as u64,
                    height as u64,
                    variation,
                    temporal_pair.map(|pair| pair.from_t()),
                    dwell_window.as_ref().map(|window| window.phases().to_vec()),
                    &inputs.pokes,
                    &inputs.gesture,
                    args,
                    aha_request.summon,
                );
                let receipt_result = encounter_play_result(
                    m.id,
                    t,
                    width as u64,
                    height as u64,
                    variation,
                    status.clone(),
                    goal.map(str::to_owned),
                    goal_met,
                    touch,
                    aha_beat,
                    aha_grade,
                    aha_allow_reveal,
                    temporal_counts,
                    dwell_counts,
                );
                match issue_encounter(&receipt_action, &receipt_result) {
                    Ok(receipt) => {
                        structured["encounter"] =
                            receipt_json(&receipt, encounter_action_json(&receipt_action))
                    }
                    Err(message) => return tool_error(&message),
                }
            }
            tool_structured(&text, structured)
        }
        None => tool_error(&unknown_room(id)),
    }
}
