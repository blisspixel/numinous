//! Bounded room rendering input for the terminal face.

use numinous_core::{Room, RoomInput};

const MAX_CLI_RENDER_WIDTH: usize = 4096;
const MAX_CLI_RENDER_HEIGHT: usize = 4096;
const MAX_CLI_RENDER_PIXELS: usize = 16 * 1024 * 1024;

pub(crate) type ParsedRoomInputs = (Vec<(f64, f64)>, Vec<RoomInput>);

pub(crate) fn parse_poke_arg(raw: &str) -> Result<(f64, f64), String> {
    let Some((x, y)) = raw.split_once(',') else {
        return Err(format!(
            "Bad --poke '{}'. Use normalized coordinates like --poke 0.4,0.6.\n",
            numinous_core::display_safe(raw)
        ));
    };
    let x = x.trim().parse::<f64>().map_err(|_| {
        format!(
            "Bad --poke '{}'. The x coordinate must be a number.\n",
            numinous_core::display_safe(raw)
        )
    })?;
    let y = y.trim().parse::<f64>().map_err(|_| {
        format!(
            "Bad --poke '{}'. The y coordinate must be a number.\n",
            numinous_core::display_safe(raw)
        )
    })?;
    if x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
        Ok((x, y))
    } else {
        Err(format!(
            "Bad --poke '{}'. Coordinates must be finite numbers in [0,1].\n",
            numinous_core::display_safe(raw)
        ))
    }
}

/// Parse one --gesture value: `down:x,y,t`, `move:x,y,t`, `up:x,y,t`, or
/// `cancel`, with finite coordinates in `[0, 1]`.
pub(crate) fn parse_gesture_arg(raw: &str) -> Result<RoomInput, String> {
    if raw == "cancel" {
        return Ok(RoomInput::PointerCancel);
    }
    let Some((kind, coords)) = raw.split_once(':') else {
        return Err(format!(
            "Bad --gesture '{}'. Use down:x,y,t, move:x,y,t, up:x,y,t, or cancel.\n",
            numinous_core::display_safe(raw)
        ));
    };
    let parts: Vec<&str> = coords.split(',').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Bad --gesture '{}'. Pointer events need x,y,t like down:0.3,0.4,0.1.\n",
            numinous_core::display_safe(raw)
        ));
    }
    let mut values = [0.0_f64; 3];
    for (slot, part) in values.iter_mut().zip(&parts) {
        let value: f64 = part.trim().parse().map_err(|_| {
            format!(
                "Bad --gesture '{}'. Coordinates must be numbers.\n",
                numinous_core::display_safe(raw)
            )
        })?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "Bad --gesture '{}'. Coordinates must be finite numbers in [0,1].\n",
                numinous_core::display_safe(raw)
            ));
        }
        *slot = value;
    }
    let (x, y, t) = (values[0], values[1], values[2]);
    match kind {
        "down" => Ok(RoomInput::PointerDown { x, y, t }),
        "move" => Ok(RoomInput::PointerMove { x, y, t }),
        "up" => Ok(RoomInput::PointerUp { x, y, t }),
        other => Err(format!(
            "Bad --gesture '{}'. Pointer kinds are down, move, and up; cancel takes no coordinates; got '{}'.\n",
            numinous_core::display_safe(raw),
            numinous_core::display_safe(other)
        )),
    }
}

pub(crate) fn parse_gestures(raw: &[String]) -> Result<Vec<RoomInput>, String> {
    if raw.len() > numinous_core::MAX_ROOM_INPUTS {
        return Err(format!(
            "Too many --gesture events: got {}, maximum is {}.\n",
            raw.len(),
            numinous_core::MAX_ROOM_INPUTS
        ));
    }
    raw.iter().map(|event| parse_gesture_arg(event)).collect()
}

pub(crate) fn validate_render_dimensions(width: usize, height: usize) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("Render width and height must both be positive.\n".to_string());
    }
    if width > MAX_CLI_RENDER_WIDTH || height > MAX_CLI_RENDER_HEIGHT {
        return Err(format!(
            "Render size {width}x{height} exceeds the CLI limit of {}x{}.\n",
            MAX_CLI_RENDER_WIDTH, MAX_CLI_RENDER_HEIGHT
        ));
    }
    if width.saturating_mul(height) > MAX_CLI_RENDER_PIXELS {
        return Err(format!(
            "Render size {width}x{height} exceeds the {}-pixel allocation limit.\n",
            MAX_CLI_RENDER_PIXELS
        ));
    }
    Ok(())
}

pub(crate) fn validate_render_request(width: usize, height: usize, t: f64) -> Result<(), String> {
    validate_render_dimensions(width, height)?;
    if !t.is_finite() || !(0.0..1.0).contains(&t) {
        return Err(format!(
            "Render phase must be a finite number in [0,1); got {t}.\n"
        ));
    }
    Ok(())
}

pub(crate) fn parse_pokes(raw: &[String]) -> Result<Vec<(f64, f64)>, String> {
    if raw.len() > numinous_core::MAX_ROOM_POKES {
        return Err(format!(
            "Too many --poke values: got {}, maximum is {}.\n",
            raw.len(),
            numinous_core::MAX_ROOM_POKES
        ));
    }
    raw.iter().map(|poke| parse_poke_arg(poke)).collect()
}

pub(crate) fn parse_room_inputs(
    raw_pokes: &[String],
    raw_gestures: &[String],
) -> Result<ParsedRoomInputs, String> {
    let pokes = parse_pokes(raw_pokes)?;
    let gestures = parse_gestures(raw_gestures)?;
    if !pokes.is_empty() && !gestures.is_empty() {
        return Err(
            "Use either --poke (static hand points) or --gesture (a pointer trail), not both.\n"
                .to_string(),
        );
    }
    Ok((pokes, gestures))
}

#[derive(Clone, Copy)]
pub(crate) struct RoomRenderInput<'a> {
    pub(crate) variation: u64,
    pub(crate) pokes: &'a [(f64, f64)],
    pub(crate) gesture: &'a [RoomInput],
}

impl<'a> RoomRenderInput<'a> {
    pub(crate) fn new(variation: u64, pokes: &'a [(f64, f64)]) -> Self {
        Self {
            variation,
            pokes,
            gesture: &[],
        }
    }

    pub(crate) fn with_gesture(variation: u64, gesture: &'a [RoomInput]) -> Self {
        Self {
            variation,
            pokes: &[],
            gesture,
        }
    }

    pub(crate) fn has_interaction(self) -> bool {
        !self.pokes.is_empty() || !self.gesture.is_empty()
    }
}

#[cfg(test)]
impl RoomRenderInput<'static> {
    pub(crate) fn plain() -> Self {
        Self {
            variation: 0,
            pokes: &[],
            gesture: &[],
        }
    }
}

pub(crate) fn visible_status(
    room: &dyn Room,
    t: f64,
    input: RoomRenderInput<'_>,
) -> Option<String> {
    let base = room.status(t);
    if !input.has_interaction() {
        return base;
    }
    if !input.gesture.is_empty() {
        room.status_input(t, input.gesture)
    } else {
        let inputs = numinous_core::inputs_from_pokes(input.pokes, t);
        room.status_input(t, &inputs)
    }
    .or(base)
}
