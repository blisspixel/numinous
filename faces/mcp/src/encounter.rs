//! Bounded MCP projection for emit-only Numinous Encounter Receipts.

use numinous_broadcast::{BUILD_SEMANTIC_ID, REPLAY_ABI_VERSION, numinous_compatibility};
use numinous_core::{
    CanonicalGesture, ENCOUNTER_RECEIPT_SCHEMA, ENCOUNTER_RECEIPT_SCHEMA_VERSION,
    EncounterDeltaCounts, EncounterDwellCounts, EncounterReceipt, EncounterTool, ListenRoomAction,
    ListenRoomResult, PlayRoomAction, PlayRoomResult, RoomInput, SingExpressionAction,
    SingExpressionResult,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Parse the optional receipt switch. Omitted and false leave the result unchanged.
pub(super) fn request(arguments: &Value) -> Result<bool, String> {
    let Some(value) = arguments.get("receipt") else {
        return Ok(false);
    };
    value.as_bool().ok_or_else(|| {
        "Argument 'receipt' must be a boolean. Pass true for a replay proof; omit it or pass false to leave the result unchanged. Asking does not keep the play."
            .to_string()
    })
}

/// Canonical play_room action after public defaults and room aliases resolve.
#[expect(
    clippy::too_many_arguments,
    reason = "translates one declared play_room argument list into the core action"
)]
pub(super) fn play_action(
    room: &str,
    t: f64,
    width: u64,
    height: u64,
    variation: u64,
    from_t: Option<f64>,
    dwell: Option<Vec<f64>>,
    pokes: &[(f64, f64)],
    gesture: &[RoomInput],
    arguments: &Value,
    aha_summon: bool,
) -> PlayRoomAction {
    PlayRoomAction::new(room)
        .with_t(t)
        .with_size(width, height)
        .with_variation(variation)
        .with_from_t(from_t)
        .with_dwell(dwell)
        .with_pokes(pokes.to_vec())
        .with_gesture(
            gesture
                .iter()
                .copied()
                .filter_map(canonical_gesture)
                .collect(),
        )
        .with_place_wager(string_arg(arguments, "place_wager"))
        .with_number_wager(arguments.get("number_wager").and_then(Value::as_f64))
        .with_bin_wager(arguments.get("bin_wager").and_then(Value::as_u64))
        .with_ending_wager(string_arg(arguments, "ending_wager"))
        .with_speed_wager(string_arg(arguments, "speed_wager"))
        .with_policy_wager(string_arg(arguments, "policy_wager"))
        .with_die_choice(string_arg(arguments, "die_choice"))
        .with_counter_wager(string_arg(arguments, "counter_wager"))
        .with_aha_summon(aha_summon)
}

/// Domain-only play_room result. Prose, render, and audio stay out.
#[expect(
    clippy::too_many_arguments,
    reason = "translates one declared play_room result into the core receipt fields"
)]
pub(super) fn play_result(
    room: &str,
    t: f64,
    width: u64,
    height: u64,
    variation: u64,
    status: Option<String>,
    goal: Option<String>,
    goal_met: bool,
    delta: Option<EncounterDeltaCounts>,
    aha_beat: Option<String>,
    aha_grade: Option<String>,
    aha_allow_reveal: Option<bool>,
    temporal: Option<EncounterDeltaCounts>,
    dwell: Option<EncounterDwellCounts>,
) -> PlayRoomResult {
    PlayRoomResult::new(room)
        .with_t(t)
        .with_size(width, height)
        .with_variation(variation)
        .with_status(status)
        .with_goal(goal)
        .with_goal_met(goal_met)
        .with_delta(delta)
        .with_aha_beat(aha_beat)
        .with_aha_grade(aha_grade)
        .with_aha_allow_reveal(aha_allow_reveal)
        .with_temporal(temporal)
        .with_dwell(dwell)
}

/// Counts from a structured or typed cell delta.
pub(super) fn delta_counts(
    cells_changed: u64,
    ink_added: u64,
    ink_removed: u64,
    ink_reshaped: u64,
    total_cells: u64,
) -> EncounterDeltaCounts {
    EncounterDeltaCounts {
        cells_changed,
        ink_added,
        ink_removed,
        ink_reshaped,
        total_cells,
    }
}

/// Counts from a typed dwell invariant.
pub(super) fn dwell_counts(
    looks: u64,
    unchanged_cells: u64,
    never_ink: u64,
    always_ink: u64,
    never_ink_in_changed_region: u64,
    never_ink_enclosed: u64,
    total_cells: u64,
) -> EncounterDwellCounts {
    EncounterDwellCounts {
        looks,
        unchanged_cells,
        never_ink,
        always_ink,
        never_ink_in_changed_region,
        never_ink_enclosed,
        total_cells,
    }
}

/// Build the live play_room receipt or explain why this binary cannot sign one.
pub(super) fn issue(
    action: &PlayRoomAction,
    result: &PlayRoomResult,
) -> Result<EncounterReceipt, String> {
    issue_receipt(
        EncounterTool::PlayRoom,
        &action.canonical_bytes(),
        &result.canonical_bytes(),
    )
}

/// Additive structured receipt. Compact and full share this object.
pub(super) fn receipt_json(receipt: &EncounterReceipt, action: Value) -> Value {
    json!({
        "schema": receipt.schema(),
        "schemaVersion": receipt.schema_version(),
        "replayAbiVersion": receipt.replay_abi_version(),
        "fingerprint": hex(receipt.fingerprint()),
        "tool": receipt.tool().name(),
        "action": action,
        "actionDigest": hex(receipt.action_digest()),
        "resultDigest": hex(receipt.result_digest()),
        "provenance": {
            "packageVersion": receipt.package_version(),
            "buildSemanticId": hex(receipt.build_semantic_id()),
        },
    })
}

/// Issue a receipt from already-canonical action and result bytes.
pub(super) fn issue_receipt(
    tool: EncounterTool,
    action_bytes: &[u8],
    result_bytes: &[u8],
) -> Result<EncounterReceipt, String> {
    let compatibility = numinous_compatibility()
        .map_err(|_| "A receipt requires a valid compatibility fingerprint.".to_string())?;
    EncounterReceipt::new(
        REPLAY_ABI_VERSION,
        *compatibility.fingerprint.as_bytes(),
        tool,
        digest(action_bytes),
        digest(result_bytes),
        env!("CARGO_PKG_VERSION"),
        BUILD_SEMANTIC_ID,
    )
    .ok_or_else(|| "A receipt requires a nonempty ASCII package version.".to_string())
}

/// The closed action tuple a later call can replay without guessing defaults.
pub(super) fn action_json(action: &PlayRoomAction) -> Value {
    let mut object = json!({
        "room": action.room(),
        "t": action.t(),
        "width": action.width(),
        "height": action.height(),
        "variation": action.variation(),
        "pokes": action
            .pokes()
            .iter()
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>(),
        "ahaSummon": action.aha_summon(),
    });
    if let Some(from_t) = action.from_t() {
        object["fromT"] = json!(from_t);
    }
    if let Some(dwell) = action.dwell() {
        object["dwell"] = json!(dwell);
    }
    if !action.gesture().is_empty() {
        object["gesture"] =
            Value::Array(action.gesture().iter().copied().map(gesture_json).collect());
    }
    if let Some(wager) = action.place_wager() {
        object["placeWager"] = json!(wager);
    }
    if let Some(wager) = action.number_wager() {
        object["numberWager"] = json!(wager);
    }
    if let Some(wager) = action.bin_wager() {
        object["binWager"] = json!(wager);
    }
    if let Some(wager) = action.ending_wager() {
        object["endingWager"] = json!(wager);
    }
    if let Some(wager) = action.speed_wager() {
        object["speedWager"] = json!(wager);
    }
    if let Some(wager) = action.policy_wager() {
        object["policyWager"] = json!(wager);
    }
    if let Some(choice) = action.die_choice() {
        object["dieChoice"] = json!(choice);
    }
    if let Some(wager) = action.counter_wager() {
        object["counterWager"] = json!(wager);
    }
    object
}

/// Rebuild play_room arguments from a receipt action so the server can replay.
pub(super) fn play_args_from_action(action: &PlayRoomAction) -> Value {
    let mut arguments = json!({
        "id": action.room(),
        "t": action.t(),
        "width": action.width(),
        "height": action.height(),
        "variation": action.variation(),
        "pokes": action
            .pokes()
            .iter()
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>(),
        "receipt": true,
    });
    if let Some(from_t) = action.from_t() {
        arguments["from_t"] = json!(from_t);
    }
    if let Some(dwell) = action.dwell() {
        arguments["dwell"] = json!(dwell);
    }
    if !action.gesture().is_empty() {
        arguments["gesture"] =
            Value::Array(action.gesture().iter().copied().map(gesture_json).collect());
    }
    if let Some(wager) = action.place_wager() {
        arguments["place_wager"] = json!(wager);
    }
    if let Some(wager) = action.number_wager() {
        arguments["number_wager"] = json!(wager);
    }
    if let Some(wager) = action.bin_wager() {
        arguments["bin_wager"] = json!(wager);
    }
    if let Some(wager) = action.ending_wager() {
        arguments["ending_wager"] = json!(wager);
    }
    if let Some(wager) = action.speed_wager() {
        arguments["speed_wager"] = json!(wager);
    }
    if let Some(wager) = action.policy_wager() {
        arguments["policy_wager"] = json!(wager);
    }
    if let Some(choice) = action.die_choice() {
        arguments["die_choice"] = json!(choice);
    }
    if let Some(wager) = action.counter_wager() {
        arguments["counter_wager"] = json!(wager);
    }
    if action.aha_summon() {
        arguments["aha_summon"] = json!(true);
    }
    arguments
}

/// Canonical listen_room action after public defaults and room aliases resolve.
pub(super) fn listen_action(
    room: &str,
    t: f64,
    variation: u64,
    ambient_events: bool,
    audio: bool,
    pokes: &[(f64, f64)],
    gesture: &[RoomInput],
) -> ListenRoomAction {
    ListenRoomAction::new(room)
        .with_t(t)
        .with_variation(variation)
        .with_ambient_events(ambient_events)
        .with_audio(audio)
        .with_pokes(pokes.to_vec())
        .with_gesture(
            gesture
                .iter()
                .copied()
                .filter_map(canonical_gesture)
                .collect(),
        )
}

/// Domain-only listen_room result. WAV bytes stay out.
#[expect(
    clippy::too_many_arguments,
    reason = "translates one declared listen_room result into the core receipt fields"
)]
pub(super) fn listen_result(
    room: &str,
    t: f64,
    variation: u64,
    duration_seconds: f64,
    note_count: u64,
    returned_note_count: u64,
    truncated: bool,
    motif_key: Option<String>,
    motif_tempo: Option<u64>,
    motif_encodes: Option<String>,
    bed_duration_seconds: Option<f64>,
    bed_event_count: Option<u64>,
    audio_encoded_bytes: Option<u64>,
) -> ListenRoomResult {
    ListenRoomResult::new(room)
        .with_t(t)
        .with_variation(variation)
        .with_duration_seconds(duration_seconds)
        .with_note_count(note_count)
        .with_returned_note_count(returned_note_count)
        .with_truncated(truncated)
        .with_motif(motif_key, motif_tempo, motif_encodes)
        .with_bed(bed_duration_seconds, bed_event_count)
        .with_audio_encoded_bytes(audio_encoded_bytes)
}

/// The closed listen action a later call can replay.
pub(super) fn listen_action_json(action: &ListenRoomAction) -> Value {
    let mut object = json!({
        "room": action.room(),
        "t": action.t(),
        "variation": action.variation(),
        "ambientEvents": action.ambient_events(),
        "audio": action.audio(),
        "pokes": action
            .pokes()
            .iter()
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>(),
    });
    if !action.gesture().is_empty() {
        object["gesture"] =
            Value::Array(action.gesture().iter().copied().map(gesture_json).collect());
    }
    object
}

/// Rebuild listen_room arguments from a receipt action.
pub(super) fn listen_args_from_action(action: &ListenRoomAction) -> Value {
    let mut arguments = json!({
        "id": action.room(),
        "t": action.t(),
        "variation": action.variation(),
        "ambient_detail": if action.ambient_events() { "events" } else { "summary" },
        "pokes": action
            .pokes()
            .iter()
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>(),
        "receipt": true,
    });
    if action.audio() {
        arguments["audio"] = json!(true);
    }
    if !action.gesture().is_empty() {
        arguments["gesture"] =
            Value::Array(action.gesture().iter().copied().map(gesture_json).collect());
    }
    arguments
}

/// Canonical sing_expression action after Studio defaults resolve.
pub(super) fn sing_action(
    expr: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    notes: u64,
    scale: numinous_core::StudioScale,
    audio: bool,
) -> SingExpressionAction {
    SingExpressionAction::new(expr)
        .with_window(xmin, xmax, a)
        .with_notes(notes)
        .with_scale(scale)
        .with_audio(audio)
}

/// Domain-only sing_expression result. WAV bytes stay out.
pub(super) fn sing_result(
    expr: &str,
    duration_seconds: f64,
    note_count: u64,
    audio_encoded_bytes: Option<u64>,
) -> SingExpressionResult {
    SingExpressionResult::new(expr)
        .with_duration_seconds(duration_seconds)
        .with_note_count(note_count)
        .with_audio_encoded_bytes(audio_encoded_bytes)
}

/// The closed sing action a later call can replay.
pub(super) fn sing_action_json(action: &SingExpressionAction) -> Value {
    json!({
        "expr": action.expr(),
        "xmin": action.xmin(),
        "xmax": action.xmax(),
        "a": action.a(),
        "notes": action.notes(),
        "scale": action.scale().name(),
        "audio": action.audio(),
    })
}

/// Rebuild sing_expression arguments from a receipt action.
pub(super) fn sing_args_from_action(action: &SingExpressionAction) -> Value {
    let mut arguments = json!({
        "expr": action.expr(),
        "xmin": action.xmin(),
        "xmax": action.xmax(),
        "a": action.a(),
        "notes": action.notes(),
        "scale": action.scale().name(),
        "receipt": true,
    });
    if action.audio() {
        arguments["audio"] = json!(true);
    }
    arguments
}

/// Parse a submitted encounter object and refuse anything this binary cannot own.
pub(super) fn parse_submitted_receipt(value: &Value) -> Result<SubmittedReceipt, String> {
    let object = value.as_object().ok_or_else(|| {
        "Argument 'receipt' must be the structuredContent.encounter object.".to_string()
    })?;
    const KNOWN: &[&str] = &[
        "schema",
        "schemaVersion",
        "replayAbiVersion",
        "fingerprint",
        "tool",
        "action",
        "actionDigest",
        "resultDigest",
        "provenance",
    ];
    if let Some(unknown) = object.keys().find(|key| !KNOWN.contains(&key.as_str())) {
        return Err(format!(
            "Argument 'receipt' has an unknown field '{unknown}'. A receipt is a closed object."
        ));
    }
    let schema = object
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| "A receipt must name schema numinous.encounter-receipt.".to_string())?;
    if schema != ENCOUNTER_RECEIPT_SCHEMA {
        return Err("A receipt must name schema numinous.encounter-receipt.".to_string());
    }
    let schema_version = object
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A receipt must name schemaVersion 1.".to_string())?;
    if schema_version != ENCOUNTER_RECEIPT_SCHEMA_VERSION {
        return Err("This receipt names a schema version this binary does not speak.".to_string());
    }
    let replay_abi = object
        .get("replayAbiVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A receipt must name replayAbiVersion.".to_string())?;
    if replay_abi != u64::from(REPLAY_ABI_VERSION) {
        return Err("This receipt names a replay ABI this binary does not speak.".to_string());
    }
    let compatibility = numinous_compatibility()
        .map_err(|_| "A receipt requires a valid compatibility fingerprint.".to_string())?;
    let fingerprint = object
        .get("fingerprint")
        .and_then(Value::as_str)
        .ok_or_else(|| "A receipt must carry a compatibility fingerprint.".to_string())?;
    if fingerprint != hex(compatibility.fingerprint.as_bytes()) {
        return Err("This receipt was issued by a different catalog or build.".to_string());
    }
    let tool = object
        .get("tool")
        .and_then(Value::as_str)
        .and_then(EncounterTool::from_name)
        .ok_or_else(|| {
            "A receipt must name tool play_room, listen_room, or sing_expression.".to_string()
        })?;
    let action_digest = parse_digest(object.get("actionDigest"), "actionDigest")?;
    let result_digest = parse_digest(object.get("resultDigest"), "resultDigest")?;
    let provenance = object
        .get("provenance")
        .and_then(Value::as_object)
        .ok_or_else(|| "A receipt must carry provenance.".to_string())?;
    if provenance
        .get("packageVersion")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("A receipt must carry a nonempty packageVersion.".to_string());
    }
    let build_id = provenance
        .get("buildSemanticId")
        .and_then(Value::as_str)
        .ok_or_else(|| "A receipt must carry buildSemanticId.".to_string())?;
    if build_id != hex(&BUILD_SEMANTIC_ID) {
        return Err("This receipt was issued by a different catalog or build.".to_string());
    }
    let (replay_args, action_bytes) = match tool {
        EncounterTool::PlayRoom => {
            let action = parse_play_action(object.get("action"))?;
            (play_args_from_action(&action), action.canonical_bytes())
        }
        EncounterTool::ListenRoom => {
            let action = parse_listen_action(object.get("action"))?;
            (listen_args_from_action(&action), action.canonical_bytes())
        }
        EncounterTool::SingExpression => {
            let action = parse_sing_action(object.get("action"))?;
            (sing_args_from_action(&action), action.canonical_bytes())
        }
    };
    if hex(&digest(&action_bytes)) != action_digest {
        return Err("This receipt's action does not match its actionDigest.".to_string());
    }
    Ok(SubmittedReceipt {
        tool,
        replay_args,
        action_digest,
        result_digest,
    })
}

/// A receipt the live binary can attempt to replay.
#[derive(Debug)]
pub(super) struct SubmittedReceipt {
    pub tool: EncounterTool,
    pub replay_args: Value,
    pub action_digest: String,
    pub result_digest: String,
}

fn string_arg(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn canonical_gesture(event: RoomInput) -> Option<CanonicalGesture> {
    match event {
        RoomInput::PointerDown { x, y, t } => Some(CanonicalGesture::Down { x, y, t }),
        RoomInput::PointerMove { x, y, t } => Some(CanonicalGesture::Move { x, y, t }),
        RoomInput::PointerUp { x, y, t } => Some(CanonicalGesture::Up { x, y, t }),
        RoomInput::PointerCancel => Some(CanonicalGesture::Cancel),
        _ => None,
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn parse_digest(value: Option<&Value>, field: &str) -> Result<String, String> {
    let text = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("A receipt must carry a 64-character hex {field}."))?;
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("A receipt must carry a 64-character hex {field}."));
    }
    if text.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(format!("A receipt {field} must be lowercase hex."));
    }
    Ok(text.to_string())
}

fn parse_play_action(value: Option<&Value>) -> Result<PlayRoomAction, String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| "A receipt must carry a replayable action.".to_string())?;
    let room = object
        .get("room")
        .and_then(Value::as_str)
        .ok_or_else(|| "A receipt action must name a room.".to_string())?;
    let t = object
        .get("t")
        .and_then(Value::as_f64)
        .ok_or_else(|| "A receipt action must name t.".to_string())?;
    let width = object
        .get("width")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A receipt action must name width.".to_string())?;
    let height = object
        .get("height")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A receipt action must name height.".to_string())?;
    let variation = object
        .get("variation")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A receipt action must name variation.".to_string())?;
    let mut pokes = Vec::new();
    for poke in object
        .get("pokes")
        .and_then(Value::as_array)
        .ok_or_else(|| "A receipt action must name pokes.".to_string())?
    {
        let pair = poke
            .as_array()
            .filter(|values| values.len() == 2)
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        let x = pair[0]
            .as_f64()
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        let y = pair[1]
            .as_f64()
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        pokes.push((x, y));
    }
    let mut gesture = Vec::new();
    if let Some(events) = object.get("gesture") {
        let events = events
            .as_array()
            .ok_or_else(|| "A receipt gesture must be an array.".to_string())?;
        for event in events {
            gesture.push(parse_gesture(event)?);
        }
    }
    let dwell = match object.get("dwell") {
        None => None,
        Some(value) => Some(
            value
                .as_array()
                .ok_or_else(|| "A receipt dwell must be an array of phases.".to_string())?
                .iter()
                .map(|phase| {
                    phase
                        .as_f64()
                        .ok_or_else(|| "A receipt dwell must be an array of phases.".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    };
    Ok(PlayRoomAction::new(room)
        .with_t(t)
        .with_size(width, height)
        .with_variation(variation)
        .with_from_t(object.get("fromT").and_then(Value::as_f64))
        .with_dwell(dwell)
        .with_pokes(pokes)
        .with_gesture(gesture)
        .with_place_wager(
            object
                .get("placeWager")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_number_wager(object.get("numberWager").and_then(Value::as_f64))
        .with_bin_wager(object.get("binWager").and_then(Value::as_u64))
        .with_ending_wager(
            object
                .get("endingWager")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_speed_wager(
            object
                .get("speedWager")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_policy_wager(
            object
                .get("policyWager")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_die_choice(
            object
                .get("dieChoice")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_counter_wager(
            object
                .get("counterWager")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_aha_summon(
            object
                .get("ahaSummon")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ))
}

fn parse_listen_action(value: Option<&Value>) -> Result<ListenRoomAction, String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| "A listen receipt must carry a replayable action.".to_string())?;
    let room = object
        .get("room")
        .and_then(Value::as_str)
        .ok_or_else(|| "A listen receipt action must name a room.".to_string())?;
    let t = object
        .get("t")
        .and_then(Value::as_f64)
        .ok_or_else(|| "A listen receipt action must name t.".to_string())?;
    let variation = object
        .get("variation")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A listen receipt action must name variation.".to_string())?;
    let mut pokes = Vec::new();
    for poke in object
        .get("pokes")
        .and_then(Value::as_array)
        .ok_or_else(|| "A listen receipt action must name pokes.".to_string())?
    {
        let pair = poke
            .as_array()
            .filter(|values| values.len() == 2)
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        let x = pair[0]
            .as_f64()
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        let y = pair[1]
            .as_f64()
            .ok_or_else(|| "A receipt poke must be [x, y].".to_string())?;
        pokes.push((x, y));
    }
    let mut gesture = Vec::new();
    if let Some(events) = object.get("gesture") {
        let events = events
            .as_array()
            .ok_or_else(|| "A receipt gesture must be an array.".to_string())?;
        for event in events {
            gesture.push(parse_gesture(event)?);
        }
    }
    Ok(ListenRoomAction::new(room)
        .with_t(t)
        .with_variation(variation)
        .with_ambient_events(
            object
                .get("ambientEvents")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )
        .with_audio(
            object
                .get("audio")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )
        .with_pokes(pokes)
        .with_gesture(gesture))
}

fn parse_sing_action(value: Option<&Value>) -> Result<SingExpressionAction, String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| "A sing receipt must carry a replayable action.".to_string())?;
    let expr = object
        .get("expr")
        .and_then(Value::as_str)
        .ok_or_else(|| "A sing receipt action must name expr.".to_string())?;
    let xmin = object
        .get("xmin")
        .and_then(Value::as_f64)
        .ok_or_else(|| "A sing receipt action must name xmin.".to_string())?;
    let xmax = object
        .get("xmax")
        .and_then(Value::as_f64)
        .ok_or_else(|| "A sing receipt action must name xmax.".to_string())?;
    let a = object
        .get("a")
        .and_then(Value::as_f64)
        .ok_or_else(|| "A sing receipt action must name a.".to_string())?;
    let notes = object
        .get("notes")
        .and_then(Value::as_u64)
        .ok_or_else(|| "A sing receipt action must name notes.".to_string())?;
    let scale = object
        .get("scale")
        .and_then(Value::as_str)
        .and_then(numinous_core::StudioScale::parse)
        .ok_or_else(|| "A sing receipt action must name a valid scale.".to_string())?;
    Ok(SingExpressionAction::new(expr)
        .with_window(xmin, xmax, a)
        .with_notes(notes)
        .with_scale(scale)
        .with_audio(
            object
                .get("audio")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ))
}

fn parse_gesture(value: &Value) -> Result<CanonicalGesture, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "A receipt gesture event must be an object.".to_string())?;
    match object.get("kind").and_then(Value::as_str) {
        Some("cancel") => Ok(CanonicalGesture::Cancel),
        Some(kind @ ("down" | "move" | "up")) => {
            let x = object
                .get("x")
                .and_then(Value::as_f64)
                .ok_or_else(|| "A receipt gesture event must name x, y, and t.".to_string())?;
            let y = object
                .get("y")
                .and_then(Value::as_f64)
                .ok_or_else(|| "A receipt gesture event must name x, y, and t.".to_string())?;
            let t = object
                .get("t")
                .and_then(Value::as_f64)
                .ok_or_else(|| "A receipt gesture event must name x, y, and t.".to_string())?;
            Ok(match kind {
                "down" => CanonicalGesture::Down { x, y, t },
                "move" => CanonicalGesture::Move { x, y, t },
                _ => CanonicalGesture::Up { x, y, t },
            })
        }
        _ => Err("A receipt gesture kind must be down, move, up, or cancel.".to_string()),
    }
}

fn gesture_json(event: CanonicalGesture) -> Value {
    match event {
        CanonicalGesture::Down { x, y, t } => json!({"kind": "down", "x": x, "y": y, "t": t}),
        CanonicalGesture::Move { x, y, t } => json!({"kind": "move", "x": x, "y": y, "t": t}),
        CanonicalGesture::Up { x, y, t } => json!({"kind": "up", "x": x, "y": y, "t": t}),
        CanonicalGesture::Cancel => json!({"kind": "cancel"}),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        ENCOUNTER_RECEIPT_SCHEMA, ENCOUNTER_RECEIPT_SCHEMA_VERSION, action_json, hex, issue,
        parse_submitted_receipt, play_action, receipt_json, request,
    };
    use numinous_broadcast::{BUILD_SEMANTIC_ID, REPLAY_ABI_VERSION, numinous_compatibility};
    use serde_json::json;

    #[test]
    fn public_play_room_defaults_match_the_action_tuple() {
        assert_eq!(
            numinous_core::PLAY_ROOM_DEFAULT_WIDTH,
            numinous_broadcast::PLAY_ROOM_DEFAULT_WIDTH
        );
        assert_eq!(
            numinous_core::PLAY_ROOM_DEFAULT_HEIGHT,
            numinous_broadcast::PLAY_ROOM_DEFAULT_HEIGHT
        );
    }

    #[test]
    fn omitted_and_false_leave_the_play_unsigned() {
        assert_eq!(request(&json!({"id": "times-tables"})), Ok(false));
        assert_eq!(
            request(&json!({"id": "times-tables", "receipt": false})),
            Ok(false)
        );
    }

    #[test]
    fn true_asks_for_a_receipt() {
        assert_eq!(
            request(&json!({"id": "times-tables", "receipt": true})),
            Ok(true)
        );
    }

    #[test]
    fn a_non_boolean_receipt_is_a_guiding_error() {
        let error = request(&json!({"id": "times-tables", "receipt": "yes"})).expect_err("guide");
        assert!(error.contains("must be a boolean"), "{error}");
        assert!(error.contains("does not keep"), "{error}");
    }

    #[test]
    fn explicit_defaults_match_omitted_defaults_in_the_action_digest() {
        let omitted = play_action(
            "times-tables",
            0.0,
            72,
            32,
            0,
            None,
            None,
            &[],
            &[],
            &json!({"id": "times-tables"}),
            false,
        );
        let explicit = play_action(
            "times-tables",
            0.0,
            72,
            32,
            0,
            None,
            None,
            &[],
            &[],
            &json!({
                "id": "times-tables",
                "t": 0.0,
                "width": 72,
                "height": 32,
                "variation": 0,
                "receipt": true,
                "response_mode": "compact",
            }),
            false,
        );
        assert_eq!(omitted.canonical_bytes(), explicit.canonical_bytes());
    }

    #[test]
    fn issued_receipts_copy_live_abi_and_fingerprint() {
        let action = play_action(
            "times-tables",
            0.0,
            72,
            32,
            0,
            None,
            None,
            &[],
            &[],
            &json!({"id": "times-tables"}),
            false,
        );
        let result = numinous_core::PlayRoomResult::new("times-tables");
        let first = issue(&action, &result).expect("receipt");
        let second = issue(&action, &result).expect("second process");
        assert_eq!(first, second);
        assert_eq!(first.schema(), ENCOUNTER_RECEIPT_SCHEMA);
        assert_eq!(first.schema_version(), ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        assert_eq!(first.replay_abi_version(), REPLAY_ABI_VERSION);
        assert_eq!(
            first.fingerprint(),
            numinous_compatibility()
                .expect("live catalog")
                .fingerprint
                .as_bytes()
        );
        assert_eq!(first.build_semantic_id(), &BUILD_SEMANTIC_ID);
        assert_eq!(first.package_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_live_receipt_parses_and_a_forged_action_digest_does_not() {
        let action = play_action(
            "times-tables",
            0.0,
            72,
            32,
            0,
            None,
            None,
            &[],
            &[],
            &json!({"id": "times-tables"}),
            false,
        );
        let result = numinous_core::PlayRoomResult::new("times-tables");
        let issued = issue(&action, &result).expect("receipt");
        let json = receipt_json(&issued, action_json(&action));
        let parsed = parse_submitted_receipt(&json).expect("live receipt");
        assert_eq!(parsed.action_digest, hex(issued.action_digest()));
        let mut forged_action = json.clone();
        forged_action["actionDigest"] =
            json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        let error = parse_submitted_receipt(&forged_action).expect_err("forged action");
        assert!(error.contains("actionDigest"), "{error}");
        let mut forged_result = json;
        forged_result["resultDigest"] =
            json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        assert!(
            parse_submitted_receipt(&forged_result).is_ok(),
            "resultDigest is checked by live replay, not by the parser"
        );
    }

    #[test]
    fn a_stale_fingerprint_or_unknown_abi_is_refused() {
        let action = play_action(
            "times-tables",
            0.0,
            72,
            32,
            0,
            None,
            None,
            &[],
            &[],
            &json!({"id": "times-tables"}),
            false,
        );
        let result = numinous_core::PlayRoomResult::new("times-tables");
        let issued = issue(&action, &result).expect("receipt");
        let json = receipt_json(&issued, action_json(&action));

        let mut unknown_abi = json.clone();
        unknown_abi["replayAbiVersion"] = json!(99);
        let abi_error = parse_submitted_receipt(&unknown_abi).expect_err("unknown ABI");
        assert!(abi_error.contains("replay ABI"), "{abi_error}");

        let mut stale = json.clone();
        stale["fingerprint"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
        let stale_error = parse_submitted_receipt(&stale).expect_err("stale fingerprint");
        assert!(stale_error.contains("different catalog"), "{stale_error}");

        let mut unknown_schema = json;
        unknown_schema["schemaVersion"] = json!(2);
        let schema_error = parse_submitted_receipt(&unknown_schema).expect_err("unknown schema");
        assert!(schema_error.contains("schema version"), "{schema_error}");
    }
}
