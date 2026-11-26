//! Text and structured readings of the six-guest Sunflower board.

use numinous_core::party::sunflower::{Board, CHALLENGE, EXPLANATION, Inspection, RULES};
use serde_json::{Value, json};

/// A request can always reach any board in at most fifteen toggles.
pub(crate) const MAX_TOGGLES: usize = 15;

fn edge_text([a, b]: [u8; 2]) -> String {
    format!("{a}-{b}")
}

fn describe_selection(inspection: &Inspection) -> String {
    let [a, b, c] = inspection.edges.map(edge_text);
    let [ab, ac, bc] = &inspection.intersections;
    let result = match inspection.core.as_deref() {
        None => "These pairs have different intersections.".to_string(),
        Some([]) => "A sunflower with an empty center.".to_string(),
        Some(center) => format!("A sunflower centered on guest {}.", center[0]),
    };
    format!("A {a}, B {b}, C {c}. Intersections AB {ab:?}, AC {ac:?}, BC {bc:?}. {result}")
}

fn inspection_json(inspection: &Inspection) -> Value {
    json!({"edges": inspection.edges, "intersections": inspection.intersections, "core": inspection.core})
}

pub(crate) fn report(
    code: Option<&str>,
    toggles: &[[u8; 2]],
    inspect: Option<[[u8; 2]; 3]>,
    challenge: bool,
    reveal: bool,
) -> Result<(String, Value), String> {
    if toggles.len() > MAX_TOGGLES {
        return Err(format!(
            "Use at most {MAX_TOGGLES} toggles per request. Continue from the returned board code."
        ));
    }
    let mut board = code.map(Board::from_code).transpose()?.unwrap_or_default();
    for edge in toggles {
        board.toggle(*edge)?;
    }
    let inspection = inspect.map(|edges| board.inspect(edges)).transpose()?;
    let reading = board.reading();
    let code = board.code();
    let strings = reading
        .edges
        .iter()
        .copied()
        .map(edge_text)
        .collect::<Vec<_>>();
    let mut lines = vec![format!(
        "Sunflower board {code}. {}",
        numinous_core::counted(strings.len(), "string")
    )];
    lines.push(RULES.into());
    if strings.is_empty() {
        lines.push(
            "Choose two guests from 1 to 6 to draw a string. Choose them again to erase it.".into(),
        );
    } else {
        lines.push(strings.join("  "));
    }
    lines.push(
        if reading.witness.is_some() {
            "There is a sunflower on this board."
        } else {
            "This board is sunflower-free."
        }
        .into(),
    );
    if let Some(triple) = inspection.as_ref().or(reading.witness.as_ref()) {
        lines.push(describe_selection(triple));
    }
    let mut data = json!({
        "game": "sunflower", "board": code, "edges": reading.edges,
        "witness": reading.witness.as_ref().map(inspection_json), "inspection": inspection.as_ref().map(inspection_json)
    });
    if challenge {
        lines.push(CHALLENGE.into());
        data["challenge"] = json!(CHALLENGE);
    }
    if reveal {
        lines.push(EXPLANATION.into());
        data["explanation"] = json!(EXPLANATION);
    }
    Ok((lines.join("\n"), data))
}
