//! JSON-RPC validation, protocol revision negotiation, and response envelopes.

use serde_json::{Value, json};

use super::{predict_tool, server_info, tool_text};

/// Stateless MCP revision implemented by the per-request metadata path.
pub(super) const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";

/// Every MCP revision this dual-era server implements, newest first.
pub(super) const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &[MODERN_PROTOCOL_VERSION, "2025-11-25", "2025-06-18"];

/// The tool catalog and discovery document are immutable for one binary.
pub(super) const DISCOVERY_CACHE_TTL_MS: u64 = 86_400_000;
pub(super) const TOOLS_CACHE_TTL_MS: u64 = 86_400_000;

pub(super) const PROTOCOL_VERSION_META_KEY: &str = "io.modelcontextprotocol/protocolVersion";
pub(super) const CLIENT_INFO_META_KEY: &str = "io.modelcontextprotocol/clientInfo";
pub(super) const CLIENT_CAPABILITIES_META_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
pub(super) const SERVER_INFO_META_KEY: &str = "io.modelcontextprotocol/serverInfo";
pub(super) const JSON_SCHEMA_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RequestEra {
    Legacy,
    Modern,
}

pub(super) struct ProtocolError {
    code: i64,
    message: &'static str,
    data: Option<Value>,
}

pub(super) fn valid_request_id(id: &Value) -> bool {
    id.is_string() || id.as_i64().is_some() || id.as_u64().is_some()
}

pub(super) fn validate_jsonrpc_envelope(request: &Value) -> Result<(), ProtocolError> {
    let Some(request) = request.as_object() else {
        return Err(ProtocolError {
            code: -32600,
            message: "Invalid Request",
            data: None,
        });
    };
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || !request.get("method").is_some_and(Value::is_string)
        || request.get("id").is_some_and(|id| !valid_request_id(id))
        || request
            .get("params")
            .is_some_and(|params| !params.is_object())
    {
        return Err(ProtocolError {
            code: -32600,
            message: "Invalid Request",
            data: None,
        });
    }
    Ok(())
}

pub(super) fn request_era(request: &Value) -> Result<RequestEra, ProtocolError> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let meta = request.get("params").and_then(|params| params.get("_meta"));
    let has_modern_marker = method == "server/discover"
        || meta.is_some_and(|meta| {
            meta.get(PROTOCOL_VERSION_META_KEY).is_some()
                || meta.get(CLIENT_CAPABILITIES_META_KEY).is_some()
        });
    if !has_modern_marker {
        return Ok(RequestEra::Legacy);
    }

    let Some(meta) = meta.and_then(Value::as_object) else {
        return Err(invalid_params_error(
            "Modern requests require an object at params._meta",
        ));
    };
    let Some(version) = meta.get(PROTOCOL_VERSION_META_KEY).and_then(Value::as_str) else {
        return Err(invalid_params_error(
            "Modern requests require a string protocol version in params._meta",
        ));
    };
    if version != MODERN_PROTOCOL_VERSION {
        return Err(ProtocolError {
            code: -32022,
            message: "Unsupported protocol version",
            data: Some(json!({
                "supported": SUPPORTED_PROTOCOL_VERSIONS,
                "requested": version,
            })),
        });
    }
    if !meta
        .get(CLIENT_CAPABILITIES_META_KEY)
        .is_some_and(Value::is_object)
    {
        return Err(invalid_params_error(
            "Modern requests require client capabilities in params._meta",
        ));
    }
    if meta.get(CLIENT_INFO_META_KEY).is_some_and(|client_info| {
        !client_info.is_object()
            || !client_info.get("name").is_some_and(Value::is_string)
            || !client_info.get("version").is_some_and(Value::is_string)
    }) {
        return Err(invalid_params_error(
            "Modern client info must contain string name and version fields when present",
        ));
    }
    Ok(RequestEra::Modern)
}

fn invalid_params_error(message: &'static str) -> ProtocolError {
    ProtocolError {
        code: -32602,
        message,
        data: None,
    }
}

pub(super) fn protocol_error_response(id: Value, error: &ProtocolError) -> Value {
    let mut response = error_response(id, error.code, error.message);
    if let Some(data) = &error.data {
        response["error"]["data"] = data.clone();
    }
    response
}

fn request_supports_form_elicitation(request: &Value) -> bool {
    let Some(elicitation) = request
        .get("params")
        .and_then(|params| params.get("_meta"))
        .and_then(|meta| meta.get(CLIENT_CAPABILITIES_META_KEY))
        .and_then(|capabilities| capabilities.get("elicitation"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    elicitation.is_empty() || elicitation.get("form").is_some_and(Value::is_object)
}

pub(super) fn prepare_prediction_mrtr(
    request: &Value,
    era: RequestEra,
) -> Result<(Value, Option<Value>), ProtocolError> {
    if era != RequestEra::Modern
        || request.get("method").and_then(Value::as_str) != Some("tools/call")
        || request
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            != Some("predict")
    {
        return Ok((request.clone(), None));
    }

    let params = request
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_params_error("tools/call requires object params"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    if let Some(input_responses) = params.get("inputResponses") {
        if params.get("requestState").is_some() {
            return Err(invalid_params_error(
                "predict does not issue requestState and cannot accept it",
            ));
        }
        let response = input_responses
            .as_object()
            .and_then(|responses| {
                (responses.len() == 1)
                    .then(|| responses.get("prediction"))
                    .flatten()
            })
            .and_then(Value::as_object)
            .ok_or_else(|| {
                invalid_params_error("predict requires one input response named prediction")
            })?;
        let action = response
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params_error("prediction response requires an action"))?;
        if matches!(action, "decline" | "cancel") {
            let text = if action == "decline" {
                "Prediction declined. Nothing was graded or recorded."
            } else {
                "Prediction cancelled. Nothing was graded or recorded."
            };
            return Ok((request.clone(), Some(tool_text(text))));
        }
        if action != "accept" {
            return Err(invalid_params_error(
                "prediction response action must be accept, decline, or cancel",
            ));
        }
        let content = response
            .get("content")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid_params_error("accepted prediction requires form content"))?;
        if content
            .keys()
            .any(|key| !matches!(key.as_str(), "guess" | "rate"))
        {
            return Err(invalid_params_error(
                "prediction form content accepts only guess and rate",
            ));
        }
        let guess = content
            .get("guess")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| invalid_params_error("accepted prediction requires a finite guess"))?;
        let rate = content
            .get("rate")
            .map(|value| {
                value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| invalid_params_error("prediction rate must be finite"))
            })
            .transpose()?;
        let mut merged = arguments
            .as_object()
            .cloned()
            .ok_or_else(|| invalid_params_error("predict arguments must be an object"))?;
        if merged.contains_key("guess") || merged.contains_key("rate") {
            return Err(invalid_params_error(
                "predict accepts the guess in arguments or inputResponses, not both",
            ));
        }
        merged.insert("guess".to_string(), json!(guess));
        if let Some(rate) = rate {
            merged.insert("rate".to_string(), json!(rate));
        }
        let mut prepared = request.clone();
        prepared["params"]["arguments"] = Value::Object(merged);
        prepared["params"]
            .as_object_mut()
            .expect("validated params object")
            .remove("inputResponses");
        return Ok((prepared, None));
    }

    if arguments.get("guess").is_some() || !request_supports_form_elicitation(request) {
        return Ok((request.clone(), None));
    }
    let pose = predict_tool(&arguments);
    if pose.get("isError").and_then(Value::as_bool) == Some(true) {
        return Ok((request.clone(), Some(pose)));
    }
    let message = pose
        .get("structuredContent")
        .and_then(|structured| structured.get("prompt"))
        .and_then(Value::as_str)
        .map(|prompt| format!("{prompt} Commit your guess before seeing the hidden readout."))
        .unwrap_or_else(|| {
            "Commit a prediction before seeing the room's hidden readout.".to_string()
        });
    Ok((
        request.clone(),
        Some(json!({
            "resultType": "input_required",
            "inputRequests": {
                "prediction": {
                    "method": "elicitation/create",
                    "params": {
                        "mode": "form",
                        "message": message,
                        "requestedSchema": {
                            "$schema": JSON_SCHEMA_2020_12,
                            "type": "object",
                            "properties": {
                                "guess": {
                                    "type": "number",
                                    "title": "Predicted readout",
                                    "description": "Your committed value for the hidden readout."
                                },
                                "rate": {
                                    "type": "number",
                                    "title": "Predicted local rate",
                                    "description": "Optional slope in readout units per full phase unit."
                                }
                            },
                            "required": ["guess"]
                        }
                    }
                }
            }
        })),
    ))
}

pub(super) fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub(super) fn result_for_era(mut result: Value, method: &str, era: RequestEra) -> Value {
    if era != RequestEra::Modern {
        return result;
    }
    let Some(object) = result.as_object_mut() else {
        return result;
    };
    object
        .entry("resultType".to_string())
        .or_insert_with(|| json!("complete"));
    let meta = object
        .entry("_meta".to_string())
        .or_insert_with(|| json!({}));
    if !meta.is_object() {
        *meta = json!({});
    }
    if let Some(meta) = meta.as_object_mut() {
        meta.insert(SERVER_INFO_META_KEY.to_string(), server_info());
    }
    if object.get("resultType").and_then(Value::as_str) == Some("complete") {
        match method {
            "server/discover" => {
                object.insert("ttlMs".to_string(), json!(DISCOVERY_CACHE_TTL_MS));
                object.insert("cacheScope".to_string(), json!("public"));
            }
            "tools/list" => {
                object.insert("ttlMs".to_string(), json!(TOOLS_CACHE_TTL_MS));
                object.insert("cacheScope".to_string(), json!("public"));
            }
            _ => {}
        }
    }
    result
}

pub(super) fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
