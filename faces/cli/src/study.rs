//! Direct room study without player-state admission or mutation.

use numinous_core::{echoable_id, room_by_id, study::StudyRequest};

pub(super) fn report(
    room_id: &str,
    locale: Option<&str>,
    depth: Option<&str>,
    block: Option<&str>,
    json: bool,
) -> Result<String, String> {
    let request = StudyRequest::parse(locale, depth, block).map_err(|error| error.to_string())?;
    let room = room_by_id(room_id).ok_or_else(|| {
        format!(
            "Unknown study room '{}'. Use numinous rooms for catalog ids.",
            echoable_id(room_id)
        )
    })?;
    let response = request
        .read(room.as_ref())
        .map_err(|error| error.to_string())?;
    let flash = numinous_core::flash_limit_line(room.meta().id);
    if json {
        let mut value = crate::study_json::response_json(&response)?;
        if let Some(line) = flash {
            value["flashLimit"] = serde_json::Value::String(line);
        }
        serde_json::to_string_pretty(&value)
            .map(|text| format!("{text}\n"))
            .map_err(|error| format!("Could not encode study content: {error}"))
    } else {
        let mut text = response.plain_text();
        if let Some(line) = flash {
            text.push_str(&format!("{line}\n"));
        }
        Ok(text)
    }
}
