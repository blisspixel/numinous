//! Shared JSON presentation for the CLI and MCP study adapters.
//!
//! Core owns request meaning, selection, availability, and plain text. This
//! module only projects the resulting typed content into the common wire shape.

use numinous_core::study::{
    StudyBlock, StudyDepth, StudyFallback, StudyInline, StudyLocaleResolution, StudyPart,
    StudyResponse, StudySelection, StudyTranslationStatus,
};
use serde_json::{Value, json};

pub(crate) const STUDY_SCHEMA: &str = "numinous.room-study";
pub(crate) const STUDY_SCHEMA_VERSION: u32 = 1;

pub(crate) fn depth_name(depth: StudyDepth) -> &'static str {
    depth.as_str()
}

pub(crate) fn fallback_name(fallback: StudyFallback) -> &'static str {
    fallback.as_str()
}

pub(crate) fn translation_name(translation: StudyTranslationStatus) -> &'static str {
    translation.as_str()
}

fn locale_json(locale: &StudyLocaleResolution) -> Value {
    json!({
        "requested": locale.requested.as_str(),
        "resolved": locale.resolved,
        "fallback": locale.fallback.map(fallback_name),
    })
}

fn block_metadata(block: &StudyBlock) -> Value {
    json!({
        "id": block.id,
        "title": block.title,
        "depth": depth_name(block.depth),
        "locale": locale_json(&block.locale),
        "translation": translation_name(block.translation),
    })
}

fn inline_json(inline: &StudyInline) -> Result<Value, String> {
    match inline {
        StudyInline::Text(text) => Ok(json!({ "kind": "text", "text": text })),
        StudyInline::Math(text) => Ok(json!({ "kind": "math", "text": text })),
        _ => Err("This face cannot represent an unsupported study inline role.".to_string()),
    }
}

fn part_json(part: &StudyPart) -> Result<Value, String> {
    match part {
        StudyPart::Paragraph(runs) => Ok(json!({
            "kind": "paragraph",
            "runs": runs.iter().map(inline_json).collect::<Result<Vec<_>, _>>()?,
        })),
        StudyPart::Equation(notation) => Ok(json!({
            "kind": "equation",
            "notation": notation,
        })),
        StudyPart::Reference {
            source,
            description,
        } => Ok(json!({
            "kind": "reference",
            "source": { "id": source.id, "title": source.title, "url": source.url },
            "description": description,
        })),
        _ => Err("This face cannot represent an unsupported study content part.".to_string()),
    }
}

fn block_json(block: &StudyBlock) -> Result<Value, String> {
    let mut value = block_metadata(block);
    value["parts"] = json!(
        block
            .parts
            .iter()
            .map(part_json)
            .collect::<Result<Vec<_>, _>>()?
    );
    Ok(value)
}

pub(crate) fn response_json(response: &StudyResponse) -> Result<Value, String> {
    let document = response.document();
    let selection = match response.selection() {
        StudySelection::Depth(depth) => json!({ "kind": "depth", "depth": depth_name(*depth) }),
        StudySelection::Block(id) => json!({ "kind": "block", "block": id }),
        _ => return Err("This face cannot represent an unsupported study selection.".to_string()),
    };
    Ok(json!({
        "schema": STUDY_SCHEMA,
        "schemaVersion": STUDY_SCHEMA_VERSION,
        "room": document.room_id,
        "selection": selection,
        "locale": locale_json(&document.locale),
        "contentLocales": document.content_locales,
        "availableDepths": StudyDepth::ALL.into_iter()
            .filter(|depth| document.has_depth(*depth))
            .map(depth_name).collect::<Vec<_>>(),
        "availableBlocks": document.blocks.iter().map(block_metadata).collect::<Vec<_>>(),
        "blocks": response.selected_blocks().map(block_json).collect::<Result<Vec<_>, _>>()?,
    }))
}
