//! Read-only MCP access to shared room study, without Journey admission.

use numinous_core::{
    echoable_id, room_by_id,
    study::{
        MAX_STUDY_BLOCK_ID_BYTES, MAX_STUDY_LOCALE_BYTES, StudyDepth, StudyFallback, StudyRequest,
        StudyTranslationStatus,
    },
};
use serde_json::{Value, json};

use super::{study_json, tool_error, tool_structured};

pub(super) fn catalog_entry() -> Value {
    json!({
        "name": "study_room",
        "title": "Study a room",
        "description": "Read optional room study directly. No visit, level, wager, consolidation, or reading progress is required or awarded. Choose explanation (default), notes, or mathematics, or open one returned stable block id. Unavailable depths and blocks return an error; notes never substitute for mathematics. Text and structured content preserve scientific notation and references. Language and translation availability are explicit for the document and each block; Japanese is a reviewed draft for the Lissajous pilot, with English fallback elsewhere.",
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        },
        "inputSchema": {
            "type": "object",
            "properties": {
                "room": {
                    "type": "string", "minLength": 1,
                    "maxLength": super::MAX_TOOL_ID_CHARS,
                    "description": "Explicit catalog room id, for example lissajous."
                },
                "locale": {
                    "type": "string", "minLength": 1,
                    "maxLength": MAX_STUDY_LOCALE_BYTES,
                    "default": "en",
                    "description": "Explicit study-language request, for example en or ja-JP. Core validates its bounded language-tag grammar. Each block reports the actual text language and any fallback."
                },
                "depth": {
                    "type": "string",
                    "enum": StudyDepth::ALL.into_iter().map(study_json::depth_name).collect::<Vec<_>>(),
                    "description": "Explanation is the default when neither depth nor block is supplied. Cannot be combined with block. Missing mathematics is an error."
                },
                "block": {
                    "type": "string", "minLength": 1, "maxLength": MAX_STUDY_BLOCK_ID_BYTES,
                    "description": "One exact stable room-qualified id from availableBlocks, for example lissajous.recurrence. No earlier reading is required. Cannot be combined with depth."
                }
            },
            "required": ["room"],
            "not": { "required": ["depth", "block"] },
            "additionalProperties": false
        },
        "outputSchema": output_schema()
    })
}

fn locale_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "requested": { "type": "string" },
            "resolved": { "type": "string" },
            "fallback": { "oneOf": [
                { "type": "null" },
                { "type": "string", "enum": [
                    study_json::fallback_name(StudyFallback::ParentLanguage),
                    study_json::fallback_name(StudyFallback::TranslationUnavailable)
                ] }
            ] }
        },
        "required": ["requested", "resolved", "fallback"],
        "additionalProperties": false
    })
}

fn depth_schema() -> Value {
    json!({
        "type": "string",
        "enum": StudyDepth::ALL.into_iter().map(study_json::depth_name).collect::<Vec<_>>()
    })
}

fn part_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["paragraph"] },
                    "runs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": { "type": "string", "enum": ["text", "math"] },
                                "text": { "type": "string" }
                            },
                            "required": ["kind", "text"], "additionalProperties": false
                        }
                    }
                },
                "required": ["kind", "runs"], "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["equation"] },
                    "notation": { "type": "string" }
                },
                "required": ["kind", "notation"], "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["reference"] },
                    "description": { "type": "string" },
                    "source": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "url": { "type": "string" }
                        },
                        "required": ["id", "title", "url"], "additionalProperties": false
                    }
                },
                "required": ["kind", "description", "source"], "additionalProperties": false
            }
        ]
    })
}

fn block_schema(include_parts: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "title": { "type": "string" },
            "depth": depth_schema(),
            "locale": locale_schema(),
            "translation": { "type": "string", "enum": [
                study_json::translation_name(StudyTranslationStatus::Original),
                study_json::translation_name(StudyTranslationStatus::ReviewedDraft)
            ] }
        },
        "required": ["id", "title", "depth", "locale", "translation"],
        "additionalProperties": false
    });
    if include_parts {
        schema["properties"]["parts"] = json!({ "type": "array", "items": part_schema() });
        schema["required"]
            .as_array_mut()
            .expect("fixed schema required array")
            .push(json!("parts"));
    }
    schema
}

fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "schema": { "type": "string", "enum": [study_json::STUDY_SCHEMA] },
            "schemaVersion": { "type": "integer", "enum": [study_json::STUDY_SCHEMA_VERSION] },
            "room": { "type": "string" },
            "selection": { "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["depth"] },
                        "depth": depth_schema()
                    },
                    "required": ["kind", "depth"], "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["block"] },
                        "block": { "type": "string" }
                    },
                    "required": ["kind", "block"], "additionalProperties": false
                }
            ] },
            "locale": {
                "description": "Preferred document language; each block reports its actual text language separately.",
                "type": "object",
                "properties": locale_schema()["properties"].clone(),
                "required": ["requested", "resolved", "fallback"],
                "additionalProperties": false
            },
            "contentLocales": { "type": "array", "items": { "type": "string" } },
            "availableDepths": { "type": "array", "items": depth_schema() },
            "availableBlocks": { "type": "array", "items": block_schema(false) },
            "blocks": { "type": "array", "minItems": 1, "items": block_schema(true) }
        },
        "required": ["schema", "schemaVersion", "room", "selection", "locale", "contentLocales", "availableDepths", "availableBlocks", "blocks"],
        "additionalProperties": false
    })
}

fn optional_string<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("Argument '{key}' must be a string."))
        })
        .transpose()
}

fn response(args: &Value) -> Result<(String, Value), String> {
    let room_id = args
        .get("room")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing required string argument 'room'.".to_string())?;
    let request = StudyRequest::parse(
        optional_string(args, "locale")?,
        optional_string(args, "depth")?,
        optional_string(args, "block")?,
    )
    .map_err(|error| error.to_string())?;
    let room = room_by_id(room_id).ok_or_else(|| {
        format!(
            "Unknown study room '{}'. Use list_rooms for catalog ids.",
            echoable_id(room_id)
        )
    })?;
    let selected = request
        .read(room.as_ref())
        .map_err(|error| error.to_string())?;
    Ok((selected.plain_text(), study_json::response_json(&selected)?))
}

pub(super) fn tool(args: &Value) -> Value {
    match response(args) {
        Ok((text, structured)) => tool_structured(&text, structured),
        Err(message) => tool_error(&message),
    }
}
