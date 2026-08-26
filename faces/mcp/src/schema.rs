//! Bounded runtime validation for the declared MCP tool schemas.

use serde_json::{Map, Value, json};

use super::{DEFAULT_HEIGHT, DEFAULT_WIDTH, arcade_action, temporal, tools_catalog};

const MAX_SCHEMA_VALIDATION_DEPTH: usize = 16;

/// The most argument names a rejection will offer. A caller needs a nudge, not
/// the schema pasted back at them.
const MAX_ARGUMENT_SUGGESTIONS: usize = 2;

/// Validate the argument object against the bounded JSON Schema subset used by
/// this server. The catalog is the contract: clients that do not pre-validate
/// receive the same guiding errors as clients that do.
pub(super) fn validate_declared_tool_arguments(params: Option<&Value>) -> Result<(), String> {
    let Some(params) = params else {
        return Ok(());
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(schema) = tools_catalog()
        .get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
        })
        .and_then(|tool| tool.get("inputSchema"))
    else {
        // Unknown tools remain JSON-RPC invalid-params errors at dispatch.
        return Ok(());
    };
    let default_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&default_arguments);
    validate_schema_value(arguments, schema, "", 0)?;
    validate_domain_tool_arguments(name, arguments)
}

fn validate_domain_tool_arguments(name: &str, arguments: &Value) -> Result<(), String> {
    if name == "play_room" {
        let width = arguments
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_WIDTH) as usize;
        let height = arguments
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_HEIGHT) as usize;
        temporal::request(arguments, width, height)?;
        temporal::dwell_request(arguments, width, height)?;
    }
    if name == "munch_arcade"
        && let Some(actions) = arguments.get("actions").and_then(Value::as_array)
        && let Some((index, _)) = actions
            .iter()
            .enumerate()
            .find(|(_, action)| arcade_action(action).is_none())
    {
        return Err(format!(
            "Argument 'actions[{index}]' must be up, down, left, right, eat, w, a, s, d, or e."
        ));
    }
    Ok(())
}

fn argument_subject(path: &str) -> String {
    if path.is_empty() {
        "Arguments".to_string()
    } else {
        format!("Argument '{path}'")
    }
}

fn property_path(parent: &str, property: &str) -> String {
    if parent.is_empty() {
        property.to_string()
    } else {
        format!("{parent}.{property}")
    }
}

pub(super) fn validate_schema_value(
    value: &Value,
    schema: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_SCHEMA_VALIDATION_DEPTH {
        return Err(format!(
            "{} exceeds the supported nesting depth of {MAX_SCHEMA_VALIDATION_DEPTH}.",
            argument_subject(path)
        ));
    }

    if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
        let matches = alternatives
            .iter()
            .filter(|alternative| {
                validate_schema_value(value, alternative, path, depth + 1).is_ok()
            })
            .count();
        if matches != 1 {
            return Err(format!(
                "{} must match exactly one declared event shape.",
                argument_subject(path)
            ));
        }
        return Ok(());
    }

    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        let valid_type = match expected_type {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "number" => value.as_f64().is_some_and(f64::is_finite),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "null" => value.is_null(),
            _ => true,
        };
        if !valid_type {
            let subject = argument_subject(path);
            if path == "gesture" && expected_type == "array" {
                return Err(
                    "Argument 'gesture' must be an array, for example [{\"kind\":\"down\",\"x\":0.5,\"y\":0.5,\"t\":0.25},{\"kind\":\"up\",\"x\":0.5,\"y\":0.5,\"t\":0.25}]."
                        .to_string(),
                );
            }
            return Err(format!(
                "{subject} must be {article}{expected_type}.",
                article = if matches!(expected_type, "array" | "integer" | "object") {
                    "an "
                } else {
                    "a "
                }
            ));
        }
    }

    if let Some(allowed) = schema.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        let choices = allowed
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{} must be one of: {choices}.",
            argument_subject(path)
        ));
    }

    if let Some(text) = value.as_str()
        && let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64)
    {
        let count = text.chars().count() as u64;
        if count > maximum {
            return Err(format!(
                "{} must be at most {maximum} characters.",
                argument_subject(path)
            ));
        }
    }

    if let Some(number) = value.as_f64() {
        for (keyword, relation) in [
            ("minimum", "at least"),
            ("maximum", "at most"),
            ("exclusiveMinimum", "greater than"),
            ("exclusiveMaximum", "less than"),
        ] {
            let Some(bound) = schema.get(keyword).and_then(Value::as_f64) else {
                continue;
            };
            let valid = match keyword {
                "minimum" => number >= bound,
                "maximum" => number <= bound,
                "exclusiveMinimum" => number > bound,
                _ => number < bound,
            };
            if !valid {
                if path == "t" && keyword == "exclusiveMaximum" && bound == 1.0 {
                    return Err(
                        "Argument 't' must be less than 1. Use a finite phase from 0.0 through 0.999; the loop endpoint is 0.0."
                            .to_string(),
                    );
                }
                return Err(format!(
                    "{} must be {relation} {bound}.",
                    argument_subject(path)
                ));
            }
        }
    }

    if let Some(object) = value.as_object() {
        let properties = schema.get("properties").and_then(Value::as_object);
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for property in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(property) {
                    let missing = property_path(path, property);
                    return Err(format!("Missing required argument '{missing}'."));
                }
            }
        }
        if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            for property in object.keys() {
                if properties.is_none_or(|known| !known.contains_key(property)) {
                    let hint = nearest_argument_hint(property, properties);
                    if path.is_empty() {
                        return Err(format!(
                            "Unexpected argument '{}'.{hint}",
                            numinous_core::echoable_id(property)
                        ));
                    }
                    return Err(format!(
                        "{} has an unexpected field '{}'.{hint}",
                        argument_subject(path),
                        numinous_core::echoable_id(property)
                    ));
                }
            }
        }
        if let Some(additional_schema) = schema
            .get("additionalProperties")
            .filter(|additional| additional.is_object())
        {
            for (property, property_value) in object {
                if properties.is_none_or(|known| !known.contains_key(property)) {
                    validate_schema_value(
                        property_value,
                        additional_schema,
                        &property_path(path, property),
                        depth + 1,
                    )?;
                }
            }
        }
        if let Some(properties) = properties {
            for (property, property_schema) in properties {
                if let Some(property_value) = object.get(property) {
                    validate_schema_value(
                        property_value,
                        property_schema,
                        &property_path(path, property),
                        depth + 1,
                    )?;
                }
            }
        }
    }

    if let Some(items) = value.as_array() {
        if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64)
            && items.len() < minimum as usize
        {
            return Err(format!(
                "{} must contain at least {minimum} items.",
                argument_subject(path)
            ));
        }
        if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64)
            && items.len() > maximum as usize
        {
            return Err(format!(
                "{} accepts at most {maximum} items.",
                argument_subject(path)
            ));
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate_schema_value(item, item_schema, &format!("{path}[{index}]"), depth + 1)?;
            }
        }
    }

    Ok(())
}

/// A " Did you mean: ..." clause for an argument name the schema rejected, or
/// an empty string when nothing in the schema is close. A caller that misspells
/// `expr` as `expression` should not have to re-read the schema to find out.
fn nearest_argument_hint(property: &str, properties: Option<&Map<String, Value>>) -> String {
    let Some(known) = properties else {
        return String::new();
    };
    let names: Vec<&str> = known.keys().map(String::as_str).collect();
    let suggestions = numinous_core::nearest_names(property, names, MAX_ARGUMENT_SUGGESTIONS);
    if suggestions.is_empty() {
        return String::new();
    }
    format!(" Did you mean: {}?", suggestions.join(", "))
}
