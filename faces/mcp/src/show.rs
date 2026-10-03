//! A caller-paced show for minds that meet Numinous through MCP.
//!
//! Each call returns one complete cue. Continuation is explicit data, so the
//! experience is replayable without a session, timer, task, or stored cursor.

use numinous_broadcast::PLAY_ROOM_MAX_DWELL_CELLS;
use numinous_core::{
    Canvas, ExactRatio, LockStep, MINDS_SHOW, Room, SHOW_SCORES, ShowHandoff, ShowMotion,
    ShowScore, counted, room_action, room_by_id_with,
};
use serde_json::{Value, json};

const SHOW_SCHEMA: &str = "numinous.show-segment";
const SHOW_SCHEMA_VERSION: u32 = 2;
const MAX_RETURNED_NOTES: usize = 64;

fn score_ids() -> Vec<&'static str> {
    SHOW_SCORES.iter().map(|score| score.id()).collect()
}

fn cue_counts() -> Vec<usize> {
    SHOW_SCORES.iter().map(|score| score.cue_count()).collect()
}

fn longest_score() -> usize {
    cue_counts().into_iter().max().unwrap_or(1)
}

fn score_summary() -> String {
    SHOW_SCORES
        .iter()
        .map(|score| format!("{} ({})", score.id(), counted(score.cue_count(), "cue")))
        .collect::<Vec<_>>()
        .join(" or ")
}

pub(super) fn catalog_entry() -> Value {
    json!({
        "name": "watch_show",
        "title": "The Show",
        "description": "Watch one bounded cue from a curated show. The default strange-loop score walks six rooms from a tiny local rule toward self-reference. The overture score plays four rooms as one piece, each a single rule repeated, and its last cue hands you the Times Tables dial as a play_room call while the parameter voice converges on an exact 2:1 octave. Every call is deterministic and complete: exact ASCII looks, typed visual deltas, a visual alternative, sound notation, and optional WAV audio. The caller owns timing and must request the next position explicitly. The call reads no journal or workspace, writes no progress, and never reveals an explanation.",
        "annotations": {
            "title": "The Show",
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        },
        "inputSchema": {
            "type": "object",
            "properties": {
                "show": {
                    "type": "string",
                    "enum": score_ids(),
                    "default": MINDS_SHOW.id(),
                    "description": format!("Stable curated score identifier: {}.", score_summary())
                },
                "position": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": longest_score() - 1,
                    "default": 0,
                    "description": "Zero-based cue position inside the chosen score. Use the exact next.arguments returned by the preceding cue."
                },
                "seed": {
                    "type": "integer",
                    "minimum": 0,
                    "default": 0,
                    "description": "Replay seed. Zero preserves each room's canonical variation."
                },
                "width": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": super::MAX_TOOL_WIDTH,
                    "default": super::DEFAULT_WIDTH,
                    "description": "ASCII columns per exact look. The sum of all look cells is bounded."
                },
                "height": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": super::MAX_TOOL_HEIGHT,
                    "default": super::DEFAULT_HEIGHT,
                    "description": "ASCII rows per exact look. The sum of all look cells is bounded."
                },
                "motion": {
                    "type": "string",
                    "enum": ["sampled", "reduced"],
                    "default": "sampled",
                    "description": "sampled returns the cue's three exact looks. reduced returns the cue's still only: the room's postcard on strange-loop, each beat's finished picture on overture."
                },
                "audio": {
                    "type": "boolean",
                    "default": false,
                    "description": "Attach one WAV of the sound at the cue's still. Notation and exact note facts are always returned."
                }
            },
            "additionalProperties": false
        },
        "outputSchema": output_schema()
    })
}

fn output_schema() -> Value {
    let nullable_string = json!({
        "oneOf": [{"type": "null"}, {"type": "string"}]
    });
    let nullable_region = json!({
        "oneOf": [
            {"type": "null"},
            {
                "type": "array",
                "items": {"type": "integer", "minimum": 0},
                "minItems": 4,
                "maxItems": 4
            }
        ]
    });
    let delta = json!({
        "type": "object",
        "properties": {
            "cells_changed": {"type": "integer", "minimum": 0},
            "ink_added": {"type": "integer", "minimum": 0},
            "ink_removed": {"type": "integer", "minimum": 0},
            "ink_reshaped": {"type": "integer", "minimum": 0},
            "total_cells": {"type": "integer", "minimum": 1},
            "changed_region": nullable_region.clone()
        },
        "required": ["cells_changed", "ink_added", "ink_removed", "ink_reshaped", "total_cells", "changed_region"],
        "additionalProperties": false
    });
    let call = json!({
        "type": "object",
        "properties": {
            "tool": {"type": "string", "enum": ["watch_show"]},
            "arguments": replay_arguments_schema()
        },
        "required": ["tool", "arguments"],
        "additionalProperties": false
    });
    let handoff = json!({
        "type": "object",
        "properties": {
            "tool": {"type": "string", "enum": ["play_room"]},
            "arguments": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "t": {"type": "number", "minimum": 0, "exclusiveMaximum": 1},
                    "width": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_WIDTH},
                    "height": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_HEIGHT}
                },
                "required": ["id", "t", "width", "height"],
                "additionalProperties": false
            }
        },
        "required": ["tool", "arguments"],
        "additionalProperties": false
    });
    let exact_ratio = json!({
        "type": "object",
        "properties": {
            "numerator": {"type": "integer", "minimum": 1},
            "denominator": {"type": "integer", "minimum": 1}
        },
        "required": ["numerator", "denominator"],
        "additionalProperties": false
    });
    let octave_lock = json!({
        "oneOf": [
            {"type": "null"},
            {
                "type": "object",
                "properties": {
                    "description": {"type": "string"},
                    "target": {
                        "type": "object",
                        "properties": {
                            "numerator": {"type": "integer", "enum": [2]},
                            "denominator": {"type": "integer", "enum": [1]}
                        },
                        "required": ["numerator", "denominator"],
                        "additionalProperties": false
                    },
                    "steps": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "t": {"type": "number", "minimum": 0, "exclusiveMaximum": 1},
                                "multiplier": exact_ratio.clone(),
                                "voiceRatio": exact_ratio,
                                "centsFromOctave": {"type": "number", "maximum": 0},
                                "isOctave": {"type": "boolean"},
                                "voiceHz": {
                                    "type": "array",
                                    "items": {"type": "number", "exclusiveMinimum": 0},
                                    "minItems": 2,
                                    "maxItems": 2
                                }
                            },
                            "required": ["t", "multiplier", "voiceRatio", "centsFromOctave", "isOctave", "voiceHz"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["description", "target", "steps"],
                "additionalProperties": false
            }
        ]
    });
    json!({
        "type": "object",
        "properties": {
            "schema": {"type": "string", "enum": [SHOW_SCHEMA]},
            "schemaVersion": {"type": "integer", "enum": [SHOW_SCHEMA_VERSION]},
            "show": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "enum": score_ids()},
                    "routeVersion": {"type": "integer", "minimum": 1},
                    "title": {"type": "string"},
                    "invitation": {"type": "string"}
                },
                "required": ["id", "routeVersion", "title", "invitation"],
                "additionalProperties": false
            },
            "timingAuthority": {"type": "string", "enum": ["caller"]},
            "automaticAdvance": {"type": "boolean", "enum": [false]},
            "seed": {"type": "integer", "minimum": 0},
            "position": {"type": "integer", "minimum": 1, "maximum": longest_score()},
            "positionIndex": {"type": "integer", "minimum": 0, "maximum": longest_score() - 1},
            "cueCount": {"type": "integer", "enum": cue_counts()},
            "motion": {"type": "string", "enum": ["sampled", "reduced"]},
            "width": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_WIDTH},
            "height": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_HEIGHT},
            "segment": {
                "type": "object",
                "properties": {
                    "room": {"type": "string"},
                    "title": {"type": "string"},
                    "wing": {"type": "string"},
                    "blurb": {"type": "string"},
                    "question": {"type": "string"},
                    "action": {"type": "string"},
                    "variation": {"type": "integer", "minimum": 0},
                    "looks": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 3,
                        "items": {
                            "type": "object",
                            "properties": {
                                "index": {"type": "integer", "minimum": 1, "maximum": 3},
                                "role": {"type": "string", "enum": ["arrival", "postcard", "passage", "curtain"]},
                                "beat": {"type": "string"},
                                "t": {"type": "number", "minimum": 0, "exclusiveMaximum": 1},
                                "status": nullable_string.clone(),
                                "render": {"type": "string"},
                                "visualAlternative": {"type": "string"},
                                "deltaFromPrevious": {"oneOf": [{"type": "null"}, delta.clone()]}
                            },
                            "required": ["index", "role", "beat", "t", "status", "render", "visualAlternative", "deltaFromPrevious"],
                            "additionalProperties": false
                        }
                    },
                    "held": {
                        "oneOf": [
                            {"type": "null"},
                            {
                                "type": "object",
                                "properties": {
                                    "looks": {"type": "integer", "minimum": 2},
                                    "total_cells": {"type": "integer", "minimum": 1},
                                    "unchanged_cells": {"type": "integer", "minimum": 0},
                                    "never_ink": {"type": "integer", "minimum": 0},
                                    "always_ink": {"type": "integer", "minimum": 0},
                                    "never_ink_in_changed_region": {"type": "integer", "minimum": 0},
                                    "never_ink_enclosed": {"type": "integer", "minimum": 0},
                                    "changed_region": nullable_region
                                },
                                "required": ["looks", "total_cells", "unchanged_cells", "never_ink", "always_ink", "never_ink_in_changed_region", "never_ink_enclosed", "changed_region"],
                                "additionalProperties": false
                            }
                        ]
                    },
                    "sound": {
                        "type": "object",
                        "properties": {
                            "phase": {"type": "number", "minimum": 0, "exclusiveMaximum": 1},
                            "durationSeconds": {"type": "number", "minimum": 0},
                            "noteCount": {"type": "integer", "minimum": 0},
                            "returnedNoteCount": {"type": "integer", "minimum": 0, "maximum": MAX_RETURNED_NOTES},
                            "truncated": {"type": "boolean"},
                            "motif": {
                                "oneOf": [
                                    {"type": "null"},
                                    {
                                        "type": "object",
                                        "properties": {
                                            "key": {"type": "string"},
                                            "tempoBpm": {"type": "integer", "minimum": 1},
                                            "notation": {"type": "array", "items": {"type": "string"}},
                                            "encodes": {"type": "string"}
                                        },
                                        "required": ["key", "tempoBpm", "notation", "encodes"],
                                        "additionalProperties": false
                                    }
                                ]
                            },
                            "notes": {
                                "type": "array",
                                "maxItems": MAX_RETURNED_NOTES,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "index": {"type": "integer", "minimum": 1},
                                        "frequencyHz": {"type": "number", "minimum": 0},
                                        "name": {"type": "string"},
                                        "startSeconds": {"type": "number", "minimum": 0},
                                        "durationSeconds": {"type": "number", "minimum": 0},
                                        "amplitude": {"type": "number", "minimum": 0, "maximum": 1}
                                    },
                                    "required": ["index", "frequencyHz", "name", "startSeconds", "durationSeconds", "amplitude"],
                                    "additionalProperties": false
                                }
                            },
                            "description": {"type": "string"},
                            "octaveLock": octave_lock,
                            "audio": {
                                "oneOf": [
                                    {"type": "null"},
                                    {
                                        "type": "object",
                                        "properties": {
                                            "mimeType": {"type": "string", "enum": ["audio/wav"]},
                                            "sampleRate": {"type": "integer", "minimum": 1},
                                            "channels": {"type": "integer", "enum": [1]},
                                            "bitsPerSample": {"type": "integer", "enum": [16]},
                                            "durationSeconds": {"type": "number", "minimum": 0},
                                            "encodedBytes": {"type": "integer", "minimum": 1}
                                        },
                                        "required": ["mimeType", "sampleRate", "channels", "bitsPerSample", "durationSeconds", "encodedBytes"],
                                        "additionalProperties": false
                                    }
                                ]
                            }
                        },
                        "required": ["phase", "durationSeconds", "noteCount", "returnedNoteCount", "truncated", "motif", "notes", "description", "octaveLock", "audio"],
                        "additionalProperties": false
                    }
                },
                "required": ["room", "title", "wing", "blurb", "question", "action", "variation", "looks", "held", "sound"],
                "additionalProperties": false
            },
            "delivery": {
                "type": "object",
                "properties": {
                    "visual": {"type": "string", "enum": ["ascii"]},
                    "colorUsed": {"type": "boolean", "enum": [false]},
                    "ansiUsed": {"type": "boolean", "enum": [false]},
                    "audioRequested": {"type": "boolean"},
                    "audioContentIndex": {"oneOf": [{"type": "null"}, {"type": "integer", "minimum": 1}]},
                    "hearingClaim": {"type": "null"},
                    "observerAudioOmitted": {"type": "boolean"}
                },
                "required": ["visual", "colorUsed", "ansiUsed", "audioRequested", "audioContentIndex", "hearingClaim", "observerAudioOmitted"],
                "additionalProperties": false
            },
            "effects": {
                "type": "object",
                "properties": {
                    "continuationStored": {"type": "boolean", "enum": [false]},
                    "journeyWritten": {"type": "boolean", "enum": [false]},
                    "journalWritten": {"type": "boolean", "enum": [false]},
                    "workspaceWritten": {"type": "boolean", "enum": [false]}
                },
                "required": ["continuationStored", "journeyWritten", "journalWritten", "workspaceWritten"],
                "additionalProperties": false
            },
            "replay": call.clone(),
            "next": {"oneOf": [{"type": "null"}, call.clone(), handoff]},
            "restart": call,
            "leave": {
                "type": "object",
                "properties": {
                    "tool": {"type": "string", "enum": ["list_rooms"]},
                    "arguments": {"type": "object", "properties": {}, "additionalProperties": false}
                },
                "required": ["tool", "arguments"],
                "additionalProperties": false
            }
        },
        "required": ["schema", "schemaVersion", "show", "timingAuthority", "automaticAdvance", "seed", "position", "positionIndex", "cueCount", "motion", "width", "height", "segment", "delivery", "effects", "replay", "next", "restart", "leave"],
        "additionalProperties": false
    })
}

fn replay_arguments_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "show": {"type": "string", "enum": score_ids()},
            "position": {"type": "integer", "minimum": 0, "maximum": longest_score() - 1},
            "seed": {"type": "integer", "minimum": 0},
            "width": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_WIDTH},
            "height": {"type": "integer", "minimum": 1, "maximum": super::MAX_TOOL_HEIGHT},
            "motion": {"type": "string", "enum": ["sampled", "reduced"]},
            "audio": {"type": "boolean"}
        },
        "required": ["show", "position", "seed", "width", "height", "motion", "audio"],
        "additionalProperties": false
    })
}

pub(super) fn tool(arguments: &Value) -> Value {
    let score = match arguments.get("show") {
        None => MINDS_SHOW,
        Some(Value::String(show)) => match ShowScore::by_id(show) {
            Some(score) => score,
            None => {
                return super::tool_error(&format!(
                    "Argument 'show' must be one of: {}.",
                    score_ids().join(", ")
                ));
            }
        },
        Some(_) => return super::tool_error("Argument 'show' must be a string."),
    };
    let position_value = arguments
        .get("position")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if position_value >= score.cue_count() as u64 {
        return super::tool_error(&position_error(score));
    }
    let position = position_value as usize;
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(0);
    let width = arguments
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(super::DEFAULT_WIDTH) as usize;
    let height = arguments
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(super::DEFAULT_HEIGHT) as usize;
    if width == 0 || width > super::MAX_TOOL_WIDTH as usize {
        return super::tool_error("Argument 'width' must be from 1 through 512.");
    }
    if height == 0 || height > super::MAX_TOOL_HEIGHT as usize {
        return super::tool_error("Argument 'height' must be from 1 through 256.");
    }
    let motion = match arguments.get("motion").and_then(Value::as_str) {
        None | Some("sampled") => ShowMotion::Sampled,
        Some("reduced") => ShowMotion::Reduced,
        Some(_) => return super::tool_error("Argument 'motion' must be sampled or reduced."),
    };
    let audio_requested = match super::audible::requested(arguments) {
        Ok(requested) => requested,
        Err(message) => return super::tool_error(&message),
    };
    let Some(cue) = score.direct(seed, position, motion) else {
        return super::tool_error(&position_error(score));
    };
    let Some(total_cells) = cue
        .looks()
        .len()
        .checked_mul(width)
        .and_then(|cells| cells.checked_mul(height))
    else {
        return super::tool_error(&show_budget_error(cue.looks().len(), width, height));
    };
    if total_cells > PLAY_ROOM_MAX_DWELL_CELLS as usize {
        return super::tool_error(&show_budget_error(cue.looks().len(), width, height));
    }

    let room = room_by_id_with(cue.room_id(), cue.variation())
        .expect("a core-directed show cue names a registered room");
    let meta = room.meta();
    let mut frames = Vec::with_capacity(cue.looks().len());
    let mut looks = Vec::with_capacity(cue.looks().len());
    let mut text = format!(
        "{}\nCue {} of {}: {} ({})\nQuestion: {}\nCaller-paced: nothing advances until you request the next position.\n",
        score.title(),
        position + 1,
        score.cue_count(),
        meta.title,
        meta.id,
        cue.question()
    );
    for (index, directed) in cue.looks().iter().enumerate() {
        let mut canvas = Canvas::new(width, height);
        room.render(&mut canvas, directed.phase());
        let render = canvas.to_text();
        let status = room.status(directed.phase());
        let alternative = visual_alternative(
            meta.title,
            directed.role().as_str(),
            directed.phase(),
            meta.blurb,
            status.as_deref(),
        );
        let delta = frames
            .last()
            .and_then(|previous: &Canvas| previous.delta(&canvas))
            .map(super::render_delta_json);
        text.push_str(&format!(
            "\nLook {}: {} at t={:.3}\n{}\nVisual alternative: {}\n{}",
            index + 1,
            directed.role().as_str(),
            directed.phase(),
            directed.beat(),
            alternative,
            render
        ));
        looks.push(json!({
            "index": index + 1,
            "role": directed.role().as_str(),
            "beat": directed.beat(),
            "t": directed.phase(),
            "status": status,
            "render": render,
            "visualAlternative": alternative,
            "deltaFromPrevious": delta,
        }));
        frames.push(canvas);
    }
    let held = Canvas::invariant(&frames).map(|held| {
        json!({
            "looks": held.looks,
            "total_cells": held.total_cells,
            "unchanged_cells": held.unchanged_cells,
            "never_ink": held.never_ink,
            "always_ink": held.always_ink,
            "never_ink_in_changed_region": held.never_ink_in_changed_region,
            "never_ink_enclosed": held.never_ink_enclosed,
            "changed_region": held.changed_region.map(|(x0, y0, x1, y1)| json!([x0, y0, x1, y1])),
        })
    });

    let sound_phase = cue.sound_phase();
    let sound_spec = room.sound(sound_phase);
    let notes = sound_spec
        .notes
        .iter()
        .take(MAX_RETURNED_NOTES)
        .enumerate()
        .map(|(index, note)| {
            json!({
                "index": index + 1,
                "frequencyHz": note.freq,
                "name": super::note_name(note.freq),
                "startSeconds": note.start,
                "durationSeconds": note.dur,
                "amplitude": note.amp,
            })
        })
        .collect::<Vec<_>>();
    let motif = room.motif().map(|motif| {
        json!({
            "key": motif.key,
            "tempoBpm": motif.tempo,
            "notation": motif.notation(),
            "encodes": motif.encodes,
        })
    });
    let audible = if audio_requested {
        match super::audible::block(&sound_spec) {
            Ok(audio) => Some(audio),
            Err(message) => return super::tool_error(&message),
        }
    } else {
        None
    };
    let sound_description = format!(
        "The sound at the cue's still lasts {:.2} seconds and contains {}. Up to {MAX_RETURNED_NOTES} exact note facts follow whether or not audio was requested; returnedNoteCount and truncated state the boundary.",
        sound_spec.duration,
        counted(sound_spec.notes.len(), "mathematical note")
    );
    text.push_str(&format!("\nSound: {sound_description}"));
    if let Some(motif) = room.motif() {
        text.push_str(&format!(
            " Ambient motif: {} at {} BPM, {}. It encodes: {}.",
            motif.key,
            motif.tempo,
            motif.notation().join(" "),
            motif.encodes
        ));
    }
    if audible.is_some() {
        text.push_str(" A WAV follows. Whether a client surfaces it as sound is outside this result; the notation and note facts remain complete.");
    }
    let octave_lock = cue
        .octave_lock()
        .map(|steps| octave_lock_json(room.as_ref(), steps));
    if let Some(lock) = &octave_lock {
        text.push_str(&format!(
            "\nOctave lock: {}",
            lock["description"].as_str().unwrap_or_default()
        ));
    }

    let call = Call {
        score,
        seed,
        width,
        height,
        motion,
        audio: audio_requested,
    };
    let replay_arguments = call.arguments(position);
    let next = match cue.handoff() {
        Some(handoff) => {
            text.push_str(&format!(
                "\nThe score ends by handing you {} at t={:.3}. next is a play_room call there; turn the dial yourself with t, pokes, or a gesture.",
                handoff.room_id(),
                handoff.phase()
            ));
            Some(handoff_call(handoff, width, height))
        }
        None => (position + 1 < score.cue_count()).then(|| {
            json!({
                "tool": "watch_show",
                "arguments": call.arguments(position + 1)
            })
        }),
    };
    let structured = json!({
        "schema": SHOW_SCHEMA,
        "schemaVersion": SHOW_SCHEMA_VERSION,
        "show": {
            "id": score.id(),
            "routeVersion": score.route_version(),
            "title": score.title(),
            "invitation": score.invitation(),
        },
        "timingAuthority": "caller",
        "automaticAdvance": false,
        "seed": seed,
        "position": position + 1,
        "positionIndex": position,
        "cueCount": score.cue_count(),
        "motion": motion.as_str(),
        "width": width,
        "height": height,
        "segment": {
            "room": meta.id,
            "title": meta.title,
            "wing": meta.wing,
            "blurb": meta.blurb,
            "question": cue.question(),
            "action": room_action(room.as_ref()),
            "variation": cue.variation(),
            "looks": looks,
            "held": held,
            "sound": {
                "phase": sound_phase,
                "durationSeconds": sound_spec.duration,
                "noteCount": sound_spec.notes.len(),
                "returnedNoteCount": notes.len(),
                "truncated": sound_spec.notes.len() > MAX_RETURNED_NOTES,
                "motif": motif,
                "notes": notes,
                "description": sound_description,
                "octaveLock": octave_lock,
                "audio": audible.as_ref().map(|(_, descriptor)| descriptor.clone()),
            }
        },
        "delivery": {
            "visual": "ascii",
            "colorUsed": false,
            "ansiUsed": false,
            "audioRequested": audio_requested,
            "audioContentIndex": audible.as_ref().map(|_| 1),
            "hearingClaim": Value::Null,
            "observerAudioOmitted": false,
        },
        "effects": {
            "continuationStored": false,
            "journeyWritten": false,
            "journalWritten": false,
            "workspaceWritten": false,
        },
        "replay": {"tool": "watch_show", "arguments": replay_arguments},
        "next": next,
        "restart": {
            "tool": "watch_show",
            "arguments": call.arguments(0)
        },
        "leave": {"tool": "list_rooms", "arguments": {}},
    });
    let result = super::tool_structured(&text, structured);
    match audible {
        Some((audio, _)) => super::audible::attach(result, audio),
        None => result,
    }
}

/// The replayable request shape every continuation of one call shares.
struct Call {
    score: ShowScore,
    seed: u64,
    width: usize,
    height: usize,
    motion: ShowMotion,
    audio: bool,
}

impl Call {
    fn arguments(&self, position: usize) -> Value {
        json!({
            "show": self.score.id(),
            "position": position,
            "seed": self.seed,
            "width": self.width,
            "height": self.height,
            "motion": self.motion.as_str(),
            "audio": self.audio,
        })
    }
}

/// The followable call that hands the player the score's last room.
fn handoff_call(handoff: ShowHandoff, width: usize, height: usize) -> Value {
    json!({
        "tool": "play_room",
        "arguments": {
            "id": handoff.room_id(),
            "t": handoff.phase(),
            "width": width,
            "height": height,
        }
    })
}

fn position_error(score: ShowScore) -> String {
    format!(
        "Argument 'position' must be from 0 through {} for the {} score.",
        score.cue_count() - 1,
        score.id()
    )
}

/// The exact octave lock, with the room's own voice at every step beside the
/// arithmetic, so the claim and the sound can be checked against each other.
fn octave_lock_json(room: &dyn Room, steps: &[LockStep]) -> Value {
    let ratio = |ratio: ExactRatio| json!({"numerator": ratio.numerator(), "denominator": ratio.denominator()});
    let fraction = |ratio: ExactRatio| {
        if ratio.denominator() == 1 {
            ratio.numerator().to_string()
        } else {
            format!("{}/{}", ratio.numerator(), ratio.denominator())
        }
    };
    let rows = steps
        .iter()
        .map(|step| {
            let voice_hz = room
                .parameter_sound(step.phase(), &[])
                .map(|voice| {
                    let root = f64::from(voice.root_hz());
                    vec![root, root * f64::from(voice.ratio())]
                })
                .unwrap_or_default();
            json!({
                "t": step.phase(),
                "multiplier": ratio(step.multiplier()),
                "voiceRatio": ratio(step.voice_ratio()),
                "centsFromOctave": step.cents_from_octave(),
                "isOctave": step.is_octave(),
                "voiceHz": voice_hz,
            })
        })
        .collect::<Vec<_>>();
    let readings = steps
        .iter()
        .map(|step| {
            let interval = step.voice_ratio();
            if step.is_octave() {
                format!(
                    "K={} sounds {}:{}, exactly an octave",
                    fraction(step.multiplier()),
                    interval.numerator(),
                    interval.denominator()
                )
            } else {
                format!(
                    "K={} sounds {}:{}, {:.1} cents flat",
                    fraction(step.multiplier()),
                    interval.numerator(),
                    interval.denominator(),
                    -step.cents_from_octave()
                )
            }
        })
        .collect::<Vec<_>>();
    json!({
        "description": format!(
            "The Times Tables voice sounds K/(K-1) above its root. As the dial settles, the interval settles: {}.",
            readings.join("; then ")
        ),
        "target": {"numerator": 2, "denominator": 1},
        "steps": rows,
    })
}

fn visual_alternative(
    title: &str,
    role: &str,
    phase: f64,
    blurb: &str,
    status: Option<&str>,
) -> String {
    match status {
        Some(status) => format!(
            "{title}, {role} look at exact phase {phase:.3}. {blurb} Live readout: {status}"
        ),
        None => format!(
            "{title}, {role} look at exact phase {phase:.3}. {blurb} No live readout is present."
        ),
    }
}

fn show_budget_error(looks: usize, width: usize, height: usize) -> String {
    let per_look = width.saturating_mul(height);
    let requested = looks.saturating_mul(per_look);
    format!(
        "A show cue renders {looks} exact looks, so looks times width times height must stay within {PLAY_ROOM_MAX_DWELL_CELLS} cells. You asked for {width} by {height}, or {per_look} cells a look and {requested} cells total. Pass a smaller width or height, or choose reduced motion."
    )
}

pub(super) fn compact_summary(structured: &Value) -> Option<String> {
    let segment = structured.get("segment")?;
    let next = structured.get("next");
    let mut summary = format!(
        "{} cue {} of {}: {} ({}), {} exact look(s), caller-paced. Read structuredContent.segment for frames, deltas, visual alternatives, and sound facts.",
        structured.get("show")?.get("title")?.as_str()?,
        structured.get("position")?.as_u64()?,
        structured.get("cueCount")?.as_u64()?,
        segment.get("title")?.as_str()?,
        segment.get("room")?.as_str()?,
        segment.get("looks")?.as_array()?.len(),
    );
    let next_tool = next
        .and_then(|value| value.get("tool"))
        .and_then(Value::as_str);
    if next_tool == Some("play_room") {
        summary.push_str(&format!(
            " The score ends by handing you {}: call play_room with the returned next arguments and turn the dial yourself.",
            next?.get("arguments")?.get("id")?.as_str()?
        ));
    } else if let Some(arguments) = next.and_then(|value| value.get("arguments")) {
        summary.push_str(&format!(
            " To continue, call watch_show with position {} and the returned replay arguments.",
            arguments.get("position")?.as_u64()?
        ));
    } else {
        summary
            .push_str(" The score is complete; restart and leave calls are in structuredContent.");
    }
    Some(summary)
}

pub(super) fn viewer_result(result: &Value) -> Value {
    let Some(structured) = result.get("structuredContent") else {
        return result.clone();
    };
    if super::validate_schema_value(structured, &output_schema(), "structuredContent", 0).is_err() {
        return super::tool_error(
            "Public show projection unavailable because the result did not match its declared contract.",
        );
    }
    let mut projected = result.clone();
    let summary = compact_summary(structured).unwrap_or_else(|| "One public show cue.".to_string());
    projected["content"] = json!([{"type": "text", "text": summary}]);
    let audio_requested = projected["structuredContent"]["delivery"]["audioRequested"]
        .as_bool()
        .unwrap_or(false);
    projected["structuredContent"]["segment"]["sound"]["audio"] = Value::Null;
    projected["structuredContent"]["delivery"]["audioContentIndex"] = Value::Null;
    projected["structuredContent"]["delivery"]["observerAudioOmitted"] =
        Value::Bool(audio_requested);
    projected
}

#[cfg(test)]
mod tests {
    use super::{SHOW_SCHEMA, catalog_entry, tool, viewer_result};
    use numinous_broadcast::{MAX_EVENT_BYTES, PublicTool, PublicToolEvent};
    use serde_json::json;

    #[test]
    fn default_cue_is_exact_caller_paced_and_nonrevealing() {
        let result = tool(&json!({}));
        assert_eq!(result["isError"], false, "{result}");
        let structured = &result["structuredContent"];
        assert_eq!(structured["schema"], SHOW_SCHEMA);
        assert_eq!(structured["timingAuthority"], "caller");
        assert_eq!(structured["automaticAdvance"], false);
        assert_eq!(structured["positionIndex"], 0);
        assert_eq!(structured["segment"]["room"], "cellular-automata");
        assert_eq!(structured["segment"]["looks"].as_array().unwrap().len(), 3);
        assert_eq!(structured["effects"]["journeyWritten"], false);
        assert_eq!(structured["next"]["arguments"]["position"], 1);
        let keys = serde_json::to_string(structured).expect("json");
        for private_door in ["\"reveal\"", "\"concept\"", "\"citation\""] {
            assert!(!keys.contains(private_door), "{private_door} leaked");
        }
    }

    #[test]
    fn traversal_is_complete_replayable_and_terminal() {
        let expected = [
            "cellular-automata",
            "game-of-life",
            "rule-110",
            "busy-beaver",
            "quine",
            "strange-loop",
        ];
        for (position, room) in expected.iter().enumerate() {
            let arguments = json!({"position": position, "seed": 29});
            let first = tool(&arguments);
            let replay = tool(&arguments);
            assert_eq!(first["structuredContent"], replay["structuredContent"]);
            assert_eq!(first["structuredContent"]["segment"]["room"], *room);
            assert_eq!(
                first["structuredContent"]["next"].is_null(),
                position == expected.len() - 1
            );
        }
    }

    #[test]
    fn reduced_motion_keeps_the_same_cue_and_sound_facts() {
        let sampled = tool(&json!({"position": 2, "seed": 7, "motion": "sampled"}));
        let reduced = tool(&json!({"position": 2, "seed": 7, "motion": "reduced"}));
        assert_eq!(
            sampled["structuredContent"]["segment"]["room"],
            reduced["structuredContent"]["segment"]["room"]
        );
        assert_eq!(
            sampled["structuredContent"]["segment"]["question"],
            reduced["structuredContent"]["segment"]["question"]
        );
        assert_eq!(
            sampled["structuredContent"]["segment"]["sound"],
            reduced["structuredContent"]["segment"]["sound"]
        );
        assert_eq!(
            reduced["structuredContent"]["segment"]["looks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(reduced["structuredContent"]["segment"]["held"].is_null());
    }

    #[test]
    fn work_and_wire_budgets_fail_closed_and_leave_margin() {
        let refused = tool(&json!({"width": 512, "height": 256}));
        assert_eq!(refused["isError"], true);
        assert!(
            refused["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("must stay within")
        );

        let result = tool(&json!({"width": 512, "height": 12}));
        assert_eq!(result["isError"], false, "{result}");
        let public = viewer_result(&result);
        let event = PublicToolEvent::new(
            PublicTool::WatchShow,
            &json!({"width": 512, "height": 12}),
            &public,
        )
        .expect("public event");
        let bytes = serde_json::to_vec(&event).expect("serialize event").len();
        assert!(
            bytes + 1_024 < MAX_EVENT_BYTES,
            "public cue is {bytes} bytes"
        );
    }

    #[test]
    fn audio_is_opt_in_and_never_crosses_the_viewer_seam() {
        let result = tool(&json!({"audio": true, "motion": "reduced"}));
        assert_eq!(result["content"][1]["type"], "audio");
        assert!(
            result["content"][1]["data"]
                .as_str()
                .unwrap()
                .starts_with("UklGR")
        );
        let projected = viewer_result(&result);
        assert_eq!(projected["content"].as_array().unwrap().len(), 1);
        assert!(projected["structuredContent"]["segment"]["sound"]["audio"].is_null());
        assert!(projected["structuredContent"]["delivery"]["audioContentIndex"].is_null());
        assert_eq!(
            projected["structuredContent"]["delivery"]["observerAudioOmitted"],
            true
        );
        assert!(!serde_json::to_string(&projected).unwrap().contains("UklGR"));
    }

    #[test]
    fn compact_mode_preserves_typed_content_and_audio_bytes() {
        let full = tool(&json!({"audio": true, "motion": "reduced"}));
        let compact =
            super::super::apply_response_mode("watch_show", Some("compact"), full.clone());
        assert_eq!(
            full["structuredContent"], compact["structuredContent"],
            "presentation mode changed the typed result"
        );
        assert_eq!(full["content"][1], compact["content"][1]);
        assert!(
            compact["content"][0]["text"].as_str().unwrap().len()
                < full["content"][0]["text"].as_str().unwrap().len()
        );
    }

    #[test]
    fn successful_show_calls_do_not_create_journey_progress() {
        let journey = super::super::test_state_path("watch-show-progress");
        let _ = std::fs::remove_file(&journey);
        let response = super::super::handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":1,"method":"tools/call",
                "params":{"name":"watch_show","arguments":{"position":1}}
            }),
            &journey,
        )
        .expect("show response");
        assert_eq!(response["result"]["isError"], false, "{response}");
        assert!(
            !journey.exists(),
            "a read-only show call created Journey state"
        );
    }

    #[test]
    fn success_matches_the_declared_closed_output_schema() {
        let entry = catalog_entry();
        for arguments in [
            json!({}),
            json!({"position": 5, "seed": 99}),
            json!({"motion": "reduced", "audio": true}),
            json!({"show": "overture"}),
            json!({"show": "overture", "position": 3}),
            json!({"show": "overture", "position": 3, "motion": "reduced", "audio": true}),
        ] {
            let result = tool(&arguments);
            assert_eq!(result["isError"], false, "{arguments}: {result}");
            super::super::validate_schema_value(
                &result["structuredContent"],
                &entry["outputSchema"],
                "structuredContent",
                0,
            )
            .unwrap_or_else(|error| panic!("{error}: {result}"));
        }
    }

    /// Call a tool through the real dispatcher, exactly as a client would.
    fn dispatch(call: &serde_json::Value) -> serde_json::Value {
        let response = super::super::handle_request(&json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": call["tool"], "arguments": call["arguments"]}
        }))
        .expect("tools/call responds");
        response["result"].clone()
    }

    #[test]
    fn the_overture_followed_verbatim_hands_over_a_playable_dial() {
        // Follow every returned pointer exactly as given. The score's last
        // pointer is not a dead end or a null: it is a play_room call that a
        // client can issue without editing, landing on the K=2 heart.
        let mut call = json!({"tool": "watch_show", "arguments": {"show": "overture", "seed": 5}});
        let mut rooms = Vec::new();
        let final_next = loop {
            let result = dispatch(&call);
            assert_eq!(result["isError"], false, "{result}");
            let structured = &result["structuredContent"];
            assert_eq!(structured["show"]["id"], "overture");
            assert_eq!(structured["cueCount"], 4);
            assert_eq!(structured["effects"]["journeyWritten"], false);
            assert_eq!(structured["segment"]["variation"], 0);
            rooms.push(structured["segment"]["room"].as_str().unwrap().to_string());
            let next = structured["next"].clone();
            assert!(!next.is_null(), "the overture never ends on a null door");
            if next["tool"] != "watch_show" {
                break next;
            }
            assert_eq!(next["arguments"]["show"], "overture");
            call = next;
        };
        assert_eq!(
            rooms,
            ["chaos-game", "mandelbrot", "golden-angle", "times-tables"]
        );
        assert_eq!(final_next["tool"], "play_room");
        assert_eq!(final_next["arguments"]["id"], "times-tables");
        let played = dispatch(&final_next);
        assert_eq!(played["isError"], false, "{played}");
        let text = played["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains("K 2.00") && text.contains("CLOSED  1 LOBE"),
            "{text}"
        );
        assert!(
            text.contains("TURN THE DIAL"),
            "the hand-off names the verb: {text}"
        );
    }

    #[test]
    fn the_overture_lock_converges_on_an_exact_octave_the_voice_really_plays() {
        let result = tool(&json!({"show": "overture", "position": 3}));
        let segment = &result["structuredContent"]["segment"];
        let lock = &segment["sound"]["octaveLock"];
        assert_eq!(lock["target"], json!({"numerator": 2, "denominator": 1}));
        let steps = lock["steps"].as_array().expect("lock steps");
        let ratios = steps
            .iter()
            .map(|step| step["voiceRatio"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ratios,
            [
                json!({"numerator": 43, "denominator": 23}),
                json!({"numerator": 41, "denominator": 21}),
                json!({"numerator": 2, "denominator": 1}),
            ]
        );
        let cents = steps
            .iter()
            .map(|step| step["centsFromOctave"].as_f64().unwrap())
            .collect::<Vec<_>>();
        assert!(cents[0] < cents[1] && cents[1] < cents[2], "{cents:?}");
        assert_eq!(cents[2], 0.0);
        assert_eq!(steps[2]["isOctave"], true);
        let lock_hz = steps[2]["voiceHz"].as_array().expect("voice");
        assert_eq!(
            lock_hz[1].as_f64().unwrap(),
            2.0 * lock_hz[0].as_f64().unwrap()
        );
        // The looks are the lock steps, in order, and the sound facts are the
        // locked octave the still holds.
        let looks = segment["looks"].as_array().expect("looks");
        for (look, step) in looks.iter().zip(steps) {
            assert_eq!(look["t"], step["t"]);
        }
        assert_eq!(segment["sound"]["phase"], steps[2]["t"]);
        let notes = segment["sound"]["notes"].as_array().expect("notes");
        assert_eq!(notes.len(), 2);
        let description = lock["description"].as_str().unwrap();
        assert!(description.contains("exactly an octave"), "{description}");
        assert!(!description.contains("Mandelbrot"), "{description}");
        // Earlier beats carry no lock, and the strange-loop score is unchanged.
        let first = tool(&json!({"show": "overture"}));
        assert!(first["structuredContent"]["segment"]["sound"]["octaveLock"].is_null());
        let walk = tool(&json!({}));
        assert!(walk["structuredContent"]["segment"]["sound"]["octaveLock"].is_null());
        assert_eq!(walk["structuredContent"]["show"]["id"], "strange-loop");
    }

    #[test]
    fn counted_prose_agrees_with_the_live_scores_and_notes() {
        let entry = catalog_entry();
        let description = entry["inputSchema"]["properties"]["show"]["description"]
            .as_str()
            .expect("show description");
        for score in numinous_core::SHOW_SCORES {
            let named = format!(
                "{} ({})",
                score.id(),
                numinous_core::counted(score.cue_count(), "cue")
            );
            assert!(description.contains(&named), "{description}");
        }
        let lock = tool(&json!({"show": "overture", "position": 3}));
        let notes = lock["structuredContent"]["segment"]["sound"]["notes"]
            .as_array()
            .expect("notes")
            .len();
        let sentence = lock["structuredContent"]["segment"]["sound"]["description"]
            .as_str()
            .expect("sound description");
        assert!(
            sentence.contains(&numinous_core::counted(notes, "mathematical note")),
            "{sentence}"
        );
    }

    #[test]
    fn overture_motion_and_bounds_are_exact_and_fail_closed() {
        let reduced = tool(&json!({"show": "overture", "position": 0, "motion": "reduced"}));
        let looks = reduced["structuredContent"]["segment"]["looks"]
            .as_array()
            .expect("looks");
        assert_eq!(looks.len(), 1);
        assert_eq!(
            looks[0]["t"], 0.0,
            "the chaos still is a jump of exactly one half"
        );
        let sampled = tool(&json!({"show": "overture", "position": 0}));
        let roles = sampled["structuredContent"]["segment"]["looks"]
            .as_array()
            .expect("looks")
            .iter()
            .map(|look| look["role"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(roles, ["arrival", "passage", "curtain"]);
        let beyond = tool(&json!({"show": "overture", "position": 4}));
        assert_eq!(beyond["isError"], true);
        assert!(
            beyond["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("0 through 3 for the overture score")
        );
        let unknown = tool(&json!({"show": "finale"}));
        assert_eq!(unknown["isError"], true);
        assert!(
            unknown["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("strange-loop, overture")
        );
        let compact = super::compact_summary(
            &tool(&json!({"show": "overture", "position": 3}))["structuredContent"],
        )
        .expect("summary");
        assert!(compact.contains("call play_room"), "{compact}");
    }

    #[test]
    fn viewer_projection_is_deterministic_and_fails_closed_on_drift() {
        let mut private = tool(&json!({"seed": 3}));
        let baseline = tool(&json!({"seed": 3}));
        let first = viewer_result(&baseline);
        let second = viewer_result(&baseline);
        assert_eq!(first, second);
        private["structuredContent"]["privateField"] = json!("must not cross");
        let refused = viewer_result(&private);
        assert_eq!(refused["isError"], true);
        assert!(
            !serde_json::to_string(&refused)
                .unwrap()
                .contains("must not cross")
        );
    }
}
