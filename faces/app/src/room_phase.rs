//! Effective room phase after App presentation rules.

use numinous_core::RoomInput;

/// Whether accepted input contains a finite pointer position that can control
/// a room parameter.
pub fn has_finite_parameter_input(inputs: &[RoomInput]) -> bool {
    inputs.iter().any(|input| match *input {
        RoomInput::PointerDown { x, y, .. }
        | RoomInput::PointerMove { x, y, .. }
        | RoomInput::PointerUp { x, y, .. } => x.is_finite() && y.is_finite(),
        _ => false,
    })
}

/// The phase production rendering gives a room after presentation-specific
/// first-contact policy. Times Tables holds its K=2 opening until a hand arrives
/// outside The Show; every other room and Show frame keeps the gallery phase.
pub fn effective_room_phase(
    room_id: &str,
    phase: f64,
    inputs: &[RoomInput],
    the_show: bool,
) -> f64 {
    let has_finite_pointer = has_finite_parameter_input(inputs);
    if room_id == "times-tables" && !the_show && !has_finite_pointer {
        0.0
    } else {
        phase
    }
}
