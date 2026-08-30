//! The room threshold: three useful choices in front of the complete catalog.

use serde_json::{Value, json};

use super::tool_structured;

/// Compatibility doorway retained beside the three choices.
pub(super) const STARTER_ROOM_IDS: [&str; 4] = [
    "times-tables",
    "double-pendulum",
    "kepler-laws",
    "mandelbrot",
];

const TOUCH_ROOM_ID: &str = "times-tables";

fn catalog_row(metadata: &numinous_core::RoomMeta) -> Value {
    json!({
        "id": metadata.id,
        "title": metadata.title,
        "wing": metadata.wing,
    })
}

fn catalog_rows() -> Vec<Value> {
    numinous_core::ROOM_CATALOG
        .iter()
        .map(catalog_row)
        .collect()
}

fn starter_rows() -> Vec<Value> {
    STARTER_ROOM_IDS
        .iter()
        .filter_map(|id| numinous_core::room_meta_by_id(id))
        .map(|metadata| catalog_row(&metadata))
        .collect()
}

fn strange_loop_chain() -> Value {
    let steps = numinous_core::STRANGE_LOOP_WALK
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let metadata = numinous_core::room_meta_by_id(step.room_id)
                .expect("the core walk is locked to canonical catalog rooms");
            json!({
                "position": index + 1,
                "id": metadata.id,
                "title": metadata.title,
                "wing": metadata.wing,
                "question": step.question,
                "next": {
                    "tool": "describe_room",
                    "arguments": { "id": metadata.id },
                },
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "numinous.room-walk",
        "schemaVersion": 1,
        "id": numinous_core::STRANGE_LOOP_WALK.id,
        "title": numinous_core::STRANGE_LOOP_WALK.title,
        "invitation": numinous_core::STRANGE_LOOP_WALK.invitation,
        "steps": steps,
    })
}

fn wing_rows() -> Vec<Value> {
    // The wing reading is core's, shared with every other face, because a wing
    // list built twice is two lists that can disagree about the catalog.
    numinous_core::wings()
        .into_iter()
        .map(|wing| {
            let doorway = numinous_core::ROOM_CATALOG[wing.doorway()];
            json!({
                "wing": wing.name,
                "count": wing.len(),
                "doorway": catalog_row(&doorway),
                "next": {
                    "tool": "describe_room",
                    "arguments": { "id": doorway.id },
                },
            })
        })
        .collect()
}

fn threshold() -> Value {
    let touch = numinous_core::room_meta_by_id(TOUCH_ROOM_ID)
        .expect("the flagship threshold room is in the catalog");
    let walk_start = numinous_core::STRANGE_LOOP_WALK
        .steps
        .first()
        .expect("the Strange Loop walk has an entrance");
    json!({
        "schema": "numinous.room-threshold",
        "schemaVersion": 1,
        "doors": [
            {
                "id": "touch",
                "kind": "room",
                "title": "Touch one astonishing thing",
                "invitation": "Turn one dial and watch multiplication draw a living curve.",
                "next": {
                    "tool": "describe_room",
                    "arguments": { "id": touch.id },
                },
            },
            {
                "id": "strange-loop",
                "kind": "walk",
                "title": numinous_core::STRANGE_LOOP_WALK.title,
                "invitation": numinous_core::STRANGE_LOOP_WALK.invitation,
                "chain": numinous_core::STRANGE_LOOP_WALK.id,
                "next": {
                    "tool": "describe_room",
                    "arguments": { "id": walk_start.room_id },
                },
            },
            {
                "id": "wander",
                "kind": "wings",
                "title": "Wander by wing",
                "invitation": "Choose a field of curiosity before choosing one room.",
                "next": { "field": "wings" },
            },
        ],
    })
}

fn threshold_text(chain: &Value, wings: &[Value]) -> String {
    let touch = numinous_core::room_meta_by_id(TOUCH_ROOM_ID)
        .expect("the flagship threshold room is in the catalog");
    let step_text = chain["steps"]
        .as_array()
        .expect("the local chain projection has steps")
        .iter()
        .map(|step| {
            format!(
                "{}. {} ({}): {}",
                step["position"].as_u64().expect("step position"),
                step["title"].as_str().expect("step title"),
                step["id"].as_str().expect("step id"),
                step["question"].as_str().expect("step question")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let wing_text = wings
        .iter()
        .map(|wing| {
            format!(
                "{} ({} rooms), doorway: {} ({})",
                wing["wing"].as_str().expect("wing name"),
                wing["count"].as_u64().expect("wing count"),
                wing["doorway"]["title"].as_str().expect("doorway title"),
                wing["doorway"]["id"].as_str().expect("doorway id")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Three doors. Choose one.\n\n1. Touch one astonishing thing\n{} ({}) in {}. Turn one dial and watch multiplication draw a living curve.\n\n2. Walk the Strange Loop\n{}\n\n3. Wander by wing\n{}\n\nThe complete typed catalog remains in structuredContent.rooms.",
        touch.title, touch.id, touch.wing, step_text, wing_text,
    )
}

/// Return the three-door threshold and the complete compatible catalog.
pub(super) fn list_tool() -> Value {
    let rooms = catalog_rows();
    let chain = strange_loop_chain();
    let wings = wing_rows();
    let text = threshold_text(&chain, &wings);
    tool_structured(
        &text,
        json!({
            "count": rooms.len(),
            "starters": starter_rows(),
            "rooms": rooms,
            "threshold": threshold(),
            "chain": chain,
            "wings": wings,
        }),
    )
}

/// Give compact clients the same three choices without repeating their detail.
pub(super) fn compact_summary(structured: &Value) -> Option<String> {
    let doors = structured.get("threshold")?.get("doors")?.as_array()?;
    let choices = doors
        .iter()
        .map(|door| {
            Some(format!(
                "{} ({})",
                door.get("title")?.as_str()?,
                door.get("id")?.as_str()?
            ))
        })
        .collect::<Option<Vec<String>>>()?;
    Some(format!(
        "{} rooms, three doors: {}. Follow each door's next field. Read structuredContent.chain for the Strange Loop walk, wings to wander, and rooms for the complete catalog.",
        structured.get("count")?.as_u64()?,
        choices.join(", "),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::{Value, json};

    fn structured() -> Value {
        super::list_tool()["structuredContent"].clone()
    }

    #[test]
    fn threshold_has_exactly_three_valid_nonspoiling_doors() {
        let structured = structured();
        let doors = structured["threshold"]["doors"]
            .as_array()
            .expect("three doors");
        assert_eq!(doors.len(), 3);
        assert_eq!(
            doors
                .iter()
                .map(|door| door["id"].as_str().expect("door id"))
                .collect::<Vec<_>>(),
            ["touch", "strange-loop", "wander"]
        );
        for door in &doors[..2] {
            let room = door["next"]["arguments"]["id"]
                .as_str()
                .expect("room door next call");
            assert_eq!(door["next"]["tool"], "describe_room");
            assert!(numinous_core::room_meta_by_id(room).is_some());
        }
        assert_eq!(doors[2]["next"]["field"], "wings");
        let projected = serde_json::to_string(&json!({
            "threshold": structured["threshold"],
            "chain": structured["chain"],
            "wings": structured["wings"],
        }))
        .expect("serialize threshold");
        for forbidden in [
            "reveal",
            "concept",
            "deepCuts",
            "deep_cuts",
            "citation",
            "blurb",
        ] {
            assert!(!projected.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn strange_loop_projection_keeps_the_core_order() {
        let structured = structured();
        let ids = structured["chain"]["steps"]
            .as_array()
            .expect("walk steps")
            .iter()
            .map(|step| step["id"].as_str().expect("step id"))
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            numinous_core::STRANGE_LOOP_WALK
                .steps
                .iter()
                .map(|step| step.room_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn wing_wander_is_complete_unique_and_actionable() {
        let structured = structured();
        let wings = structured["wings"].as_array().expect("wing summaries");
        assert_eq!(
            wings
                .iter()
                .map(|wing| wing["count"].as_u64().expect("wing count"))
                .sum::<u64>(),
            numinous_core::ROOM_CATALOG.len() as u64
        );
        assert_eq!(
            wings
                .iter()
                .map(|wing| wing["wing"].as_str().expect("wing name"))
                .collect::<HashSet<_>>()
                .len(),
            wings.len()
        );
        for wing in wings {
            assert_eq!(wing["doorway"]["wing"], wing["wing"]);
            assert_eq!(wing["next"]["tool"], "describe_room");
            assert_eq!(wing["next"]["arguments"]["id"], wing["doorway"]["id"]);
        }
    }

    #[test]
    fn complete_catalog_and_starters_remain_compatible() {
        let result = super::list_tool();
        let structured = &result["structuredContent"];
        let expected_rooms = numinous_core::ROOM_CATALOG
            .iter()
            .map(super::catalog_row)
            .collect::<Vec<_>>();
        assert_eq!(structured["rooms"], json!(expected_rooms));
        assert_eq!(structured["starters"], json!(super::starter_rows()));
        assert_eq!(structured["count"], expected_rooms.len());
        let text = result["content"][0]["text"]
            .as_str()
            .expect("threshold text");
        assert!(text.contains("Three doors"));
        assert!(text.contains("structuredContent.rooms"));
        assert!(text.len() < 8_192, "threshold became another catalog dump");
    }
}
