//! Bounded room interaction input for the MCP face.

use numinous_core::{Canvas, Room, RoomInput};
use serde_json::{Value, json};

#[derive(Debug)]
pub(crate) struct ParsedRoomInputs {
    pub(crate) pokes: Vec<(f64, f64)>,
    pub(crate) gesture: Vec<RoomInput>,
}

pub(crate) fn parse_room_pokes(args: &Value) -> Result<Vec<(f64, f64)>, String> {
    let Some(raw) = args.get("pokes") else {
        return Ok(Vec::new());
    };
    let Some(points) = raw.as_array() else {
        return Err("Argument 'pokes' must be an array of [x, y] pairs.".to_string());
    };
    if points.len() > numinous_core::MAX_ROOM_POKES {
        return Err(format!(
            "Argument 'pokes' accepts at most {} points.",
            numinous_core::MAX_ROOM_POKES
        ));
    }
    points
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let Some(pair) = point.as_array() else {
                return Err(format!("Argument 'pokes[{i}]' must be [x, y]."));
            };
            if pair.len() != 2 {
                return Err(format!(
                    "Argument 'pokes[{i}]' must contain exactly two numbers."
                ));
            }
            let Some(x) = pair.first().and_then(Value::as_f64) else {
                return Err(format!("Argument 'pokes[{i}][0]' must be a number."));
            };
            let Some(y) = pair.get(1).and_then(Value::as_f64) else {
                return Err(format!("Argument 'pokes[{i}][1]' must be a number."));
            };
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..=1.0).contains(&x)
                || !(0.0..=1.0).contains(&y)
            {
                return Err(format!(
                    "Argument 'pokes[{i}]' must contain finite coordinates in [0,1]."
                ));
            }
            Ok((x, y))
        })
        .collect()
}

/// Parse the optional `gesture` argument as a replayable pointer trail.
///
/// Down, move, and up events require finite x, y, and t values in `[0, 1]`.
/// Cancel takes no other fields. Unknown fields are rejected, and the trail is
/// bounded by [`numinous_core::MAX_ROOM_INPUTS`].
fn parse_room_gesture(args: &Value) -> Result<Vec<RoomInput>, String> {
    let Some(raw) = args.get("gesture") else {
        return Ok(Vec::new());
    };
    let Some(events) = raw.as_array() else {
        return Err(
            "Argument 'gesture' must be an array, for example [{\"kind\":\"down\",\"x\":0.5,\"y\":0.5,\"t\":0.25},{\"kind\":\"up\",\"x\":0.5,\"y\":0.5,\"t\":0.25}]."
                .to_string(),
        );
    };
    if events.len() > numinous_core::MAX_ROOM_INPUTS {
        return Err(format!(
            "Argument 'gesture' accepts at most {} events.",
            numinous_core::MAX_ROOM_INPUTS
        ));
    }
    events
        .iter()
        .enumerate()
        .map(|(i, event)| {
            let Some(fields) = event.as_object() else {
                return Err(format!("Argument 'gesture[{i}]' must be an object."));
            };
            let kind = fields.get("kind").and_then(Value::as_str).unwrap_or("");
            let allowed: &[&str] = match kind {
                "cancel" => &["kind"],
                "down" | "move" | "up" => &["kind", "x", "y", "t"],
                other => {
                    return Err(format!(
                        "Argument 'gesture[{i}].kind' must be down, move, up, or cancel; got '{other}'."
                    ));
                }
            };
            if let Some(unknown) = fields.keys().find(|key| !allowed.contains(&key.as_str())) {
                return Err(format!(
                    "Argument 'gesture[{i}]' has an unexpected field '{unknown}'."
                ));
            }
            if kind == "cancel" {
                return Ok(RoomInput::PointerCancel);
            }
            let coord = |name: &str| -> Result<f64, String> {
                let value = fields
                    .get(name)
                    .and_then(Value::as_f64)
                    .ok_or(format!("Argument 'gesture[{i}].{name}' must be a number."))?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!(
                        "Argument 'gesture[{i}].{name}' must be finite and in [0,1]."
                    ));
                }
                Ok(value)
            };
            let (x, y, t) = (coord("x")?, coord("y")?, coord("t")?);
            match kind {
                "down" => Ok(RoomInput::PointerDown { x, y, t }),
                "move" => Ok(RoomInput::PointerMove { x, y, t }),
                _ => Ok(RoomInput::PointerUp { x, y, t }),
            }
        })
        .collect()
}

pub(crate) fn parse_room_inputs(args: &Value) -> Result<ParsedRoomInputs, String> {
    let pokes = parse_room_pokes(args)?;
    let gesture = parse_room_gesture(args)?;
    if !pokes.is_empty() && !gesture.is_empty() {
        return Err(
            "Use either 'pokes' (static hand points) or 'gesture' (a pointer trail), not both in one call."
                .to_string(),
        );
    }
    Ok(ParsedRoomInputs { pokes, gesture })
}

/// Echo the canonical JSON form that was accepted, never raw client bytes.
pub(crate) fn gesture_json(gesture: &[RoomInput]) -> Value {
    Value::Array(
        gesture
            .iter()
            .map(|event| match *event {
                RoomInput::PointerDown { x, y, t } => {
                    json!({"kind": "down", "x": x, "y": y, "t": t})
                }
                RoomInput::PointerMove { x, y, t } => {
                    json!({"kind": "move", "x": x, "y": y, "t": t})
                }
                RoomInput::PointerUp { x, y, t } => {
                    json!({"kind": "up", "x": x, "y": y, "t": t})
                }
                _ => json!({"kind": "cancel"}),
            })
            .collect(),
    )
}

pub(crate) fn render_room_observation(
    room: &dyn Room,
    canvas: &mut Canvas,
    t: f64,
    inputs: &[RoomInput],
) {
    if inputs.is_empty() {
        room.render(canvas, t);
    } else {
        room.render_input(canvas, t, inputs);
    }
}

pub(crate) fn room_status_at(room: &dyn Room, t: f64, inputs: &[RoomInput]) -> Option<String> {
    if inputs.is_empty() {
        room.status(t)
    } else {
        room.status_input(t, inputs)
    }
}
