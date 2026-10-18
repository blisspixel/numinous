//! Private stateless Route Lab transport over the canonical bounded workbench.

use super::{route_json, tool_error, tool_structured};
use numinous_core::route::{
    MAX_ROUTE_JUNCTIONS, MAX_ROUTE_ROAD_COST, MAX_ROUTE_ROADS, MAX_ROUTE_STOPS,
};
use numinous_core::route_creation::MAX_ROUTE_CAPSULE_BYTES;
use numinous_core::route_workbench::MAX_ROUTE_UNDO;
use serde_json::{Value, json};

pub(super) fn catalog_entry() -> Value {
    json!({
        "name":"route_lab", "title":"Explore delivery routes",
        "description":"Choose a delivery order and compare its round-trip cost with nearest-next and the exact minimum for a street network. Start with no arguments, or carry the returned working state (snapshot) into another call. Roads are two-way; declare each connection once. Actions replace network with current, or edit road_cost, road_open, stops (depot first), depot, or order; greedy chooses the nearest next stop, improve accepts one cheaper reorder, and undo restores a prior edit. Evaluation preserves your order. Each leg follows the cheapest open road path and may pass or revisit other stops. If required stops cannot reach one another, the network stays editable with an infeasible comparison; disconnected unused junctions do not prevent a round trip. choices starts read-only nearest-next playback at cursor zero, and choice_step reveals records with steps (default one) or an absolute cursor. A record names the current stop, remaining candidate stops and shortest-path costs, the chosen stop, the expanded street leg, and the accumulated cost, including the return to the depot. Seeking changes only the cursor. The player order stays. Final comparisons stay labeled while later records leave the view. nearestStatement says whether nearest-next costs more than the exact minimum on this network or matches it. trace records a shortest-path search at cursor zero, and step reveals events with steps (default one) or an absolute cursor. Event costs are cumulative from the starting junction: settled is final, relaxed is tentative. trace.view reports each junction as unseen, tentative, settled, or unreachable, with its currently revealed cost and predecessor plus the latest visible event. Unseen becomes unreachable only when playback completes; seeking backwards removes later knowledge. The path result appears on completion. next is a followable call carrying snapshot. Editing state stays private and caller-carried. Scalar action save returns a NUMINOUS_ROUTE 1 creation from the current snapshot; an optional existing capsule preserves its parent identity while saving edits. Explicit remix creates the child. Scalar open or remix accepts capsule text and starts a fresh session. Portable creations carry the authored network, delivery order, and parent identity, with no undo or search playback. creation.next offers an explicit remix; save next reopens the saved creation. The project tool can keep that capsule using existing project persistence.",
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false},
        "inputSchema":request_schema(),
        "outputSchema":output_schema()
    })
}

fn diagnostic_schema() -> Value {
    json!({"oneOf":[
        object_schema(json!({"code":{"type":"string","enum":["size","junction","road","duplicate_stop","tour","arithmetic"]}}),&["code"]),
        object_schema(json!({"code":{"type":"string","enum":["unreachable"]},"from":bounded_index(),"to":bounded_index()}),&["code","from","to"])
    ]})
}

fn output_schema() -> Value {
    let integer = json!({"type":"integer","minimum":0});
    let tour = object_schema(
        json!({"order":{"type":"array","items":bounded_index()},"cost":integer,"walk":{"type":"array","items":bounded_index()}}),
        &["order", "cost", "walk"],
    );
    let proposal = object_schema(
        json!({"first":integer,"second":integer,"delta":{"type":"integer"},"tour":tour}),
        &["first", "second", "delta", "tour"],
    );
    let comparison = json!({"oneOf":[
        object_schema(json!({"status":{"type":"string","enum":["feasible"]},"current":tour,"greedy":tour,"exact":object_schema(json!({"tour":tour,"statesCompleted":integer}),&["tour","statesCompleted"]),"proposal":{"oneOf":[{"type":"null"},proposal]},"nearestStatement":{"type":"string"}}),&["status","current","greedy","exact","proposal","nearestStatement"]),
        object_schema(json!({"status":{"type":"string","enum":["infeasible"]},"diagnosis":{"type":"string"},"diagnostic":diagnostic_schema()}),&["status","diagnosis","diagnostic"])
    ]});
    let distance = json!({"type":"integer","minimum":0,"description":"Cumulative path cost from the search starting junction, not the individual road cost."});
    let event = json!({"oneOf":[
        object_schema(json!({"type":{"type":"string","enum":["settled"]},"junction":bounded_index(),"cost":distance}),&["type","junction","cost"]),
        object_schema(json!({"type":{"type":"string","enum":["relaxed"]},"from":bounded_index(),"to":bounded_index(),"cost":distance}),&["type","from","to","cost"])
    ]});
    let result = json!({"oneOf":[{"type":"null"},
        object_schema(json!({"status":{"type":"string","enum":["reachable"]},"junctions":{"type":"array","items":bounded_index()},"cost":integer}),&["status","junctions","cost"]),
        object_schema(json!({"status":{"type":"string","enum":["unreachable"]},"diagnosis":{"type":"string"},"diagnostic":diagnostic_schema()}),&["status","diagnosis","diagnostic"])
    ]});
    let junction = object_schema(
        json!({"junction":bounded_index(),"cost":{"oneOf":[{"type":"null"},distance]},"predecessor":{"oneOf":[{"type":"null"},bounded_index()]},"state":{"type":"string","enum":["unseen","tentative","settled","unreachable"]}}),
        &["junction", "cost", "predecessor", "state"],
    );
    let view = object_schema(
        json!({"from":bounded_index(),"to":bounded_index(),"cursor":integer,"eventCount":integer,"completed":{"type":"boolean"},"junctions":{"type":"array","maxItems":MAX_ROUTE_JUNCTIONS,"items":junction},"activeEvent":{"oneOf":[{"type":"null"},event]}}),
        &[
            "from",
            "to",
            "cursor",
            "eventCount",
            "completed",
            "junctions",
            "activeEvent",
        ],
    );
    let trace = object_schema(
        json!({"from":bounded_index(),"to":bounded_index(),"cursor":integer,"eventCount":integer,"completed":{"type":"boolean"},"events":{"type":"array","items":event},"view":view,"result":result}),
        &[
            "from",
            "to",
            "cursor",
            "eventCount",
            "completed",
            "events",
            "view",
            "result",
        ],
    );
    let next = object_schema(
        json!({"tool":{"type":"string","enum":["route_lab"]},"arguments":request_schema()}),
        &["tool", "arguments"],
    );
    let candidate = object_schema(
        json!({"stop":bounded_index(),"cost":integer}),
        &["stop", "cost"],
    );
    let choice = object_schema(
        json!({"current":bounded_index(),"candidates":{"type":"array","items":candidate},"chosen":bounded_index(),"leg":{"type":"array","items":bounded_index()},"cost":integer,"accumulated":integer}),
        &[
            "current",
            "candidates",
            "chosen",
            "leg",
            "cost",
            "accumulated",
        ],
    );
    let choices = object_schema(
        json!({"cursor":integer,"choiceCount":integer,"completed":{"type":"boolean"},"prefixCost":integer,"records":{"type":"array","items":choice},"nearestStatement":{"oneOf":[{"type":"null"},{"type":"string"}]}}),
        &[
            "cursor",
            "choiceCount",
            "completed",
            "prefixCost",
            "records",
            "nearestStatement",
        ],
    );
    object_schema(
        json!({"schema":{"type":"string","enum":["numinous.route-workbench"]},"schemaVersion":{"type":"integer","enum":[1]},"changed":{"type":"boolean"},"snapshot":request_schema()["properties"]["snapshot"],"comparison":comparison,"trace":{"oneOf":[{"type":"null"},trace]},"choices":{"oneOf":[{"type":"null"},choices]},"next":next,"creation":object_schema(json!({"capsule":{"type":"string"},"identityHex":{"type":"string"},"parentIdentityHex":{"oneOf":[{"type":"null"},{"type":"string"}]},"next":next}),&["capsule","identityHex","parentIdentityHex","next"])}),
        &[
            "schema",
            "schemaVersion",
            "changed",
            "snapshot",
            "comparison",
            "trace",
            "choices",
            "next",
        ],
    )
}

pub(super) fn tool(arguments: &Value) -> Value {
    match route_json::response(arguments) {
        Ok(result) => {
            let mut text = if result["comparison"]["status"] == "feasible" {
                format!(
                    "Route Lab round-trip cost: yours {}, nearest next {}, minimum {}. Each delivery leg follows the cheapest open road path.",
                    result["comparison"]["current"]["cost"],
                    result["comparison"]["greedy"]["cost"],
                    result["comparison"]["exact"]["tour"]["cost"]
                )
            } else {
                format!(
                    "No round-trip comparison: {}. Your street network is retained for editing or undo.",
                    result["comparison"]["diagnosis"]
                        .as_str()
                        .unwrap_or("infeasible")
                )
            };
            append_choice_prose(&mut text, &result["choices"]);
            text.push_str(" Carry snapshot into the next call to continue.");
            tool_structured(&text, result)
        }
        Err(error) => tool_error(&format!("Invalid Route Lab request: {error}")),
    }
}

/// The same visible choice records the CLI prints, so a text reader and a
/// compact reader see the street leg rather than only the two stop ids.
fn append_choice_prose(text: &mut String, choices: &Value) {
    if choices.is_null() {
        return;
    }
    let Some(cursor) = choices["cursor"].as_u64() else {
        return;
    };
    let Some(count) = choices["choiceCount"].as_u64() else {
        return;
    };
    let Some(prefix) = choices["prefixCost"].as_u64() else {
        return;
    };
    let state = if choices["completed"] == true {
        "complete"
    } else {
        "paused"
    };
    text.push_str(&format!(
        " Nearest choices {cursor} of {} revealed ({state}). Prefix cost: {}.",
        numinous_core::counted(count as usize, "choice"),
        numinous_core::counted(prefix as usize, "travel unit")
    ));
    let Some(records) = choices["records"].as_array() else {
        return;
    };
    for record in records {
        let Some(candidates) = record["candidates"].as_array() else {
            continue;
        };
        let Some(leg) = record["leg"].as_array() else {
            continue;
        };
        let listed = candidates
            .iter()
            .filter_map(|candidate| {
                Some(format!(
                    "{} ({})",
                    candidate["stop"].as_u64()?,
                    candidate["cost"].as_u64()?
                ))
            })
            .collect::<Vec<_>>()
            .join(", ");
        let walk = leg
            .iter()
            .filter_map(Value::as_u64)
            .map(|stop| stop.to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        let (Some(current), Some(chosen), Some(cost), Some(accumulated)) = (
            record["current"].as_u64(),
            record["chosen"].as_u64(),
            record["cost"].as_u64(),
            record["accumulated"].as_u64(),
        ) else {
            continue;
        };
        text.push_str(&format!(
            " At stop {current}, candidates {listed}; chosen {chosen}; leg {walk}; cost {}; accumulated {}.",
            numinous_core::counted(cost as usize, "travel unit"),
            numinous_core::counted(accumulated as usize, "travel unit")
        ));
    }
    if let Some(statement) = choices["nearestStatement"].as_str() {
        text.push(' ');
        text.push_str(statement);
    }
}

fn bounded_index() -> Value {
    json!({"type":"integer","minimum":0,"maximum":MAX_ROUTE_JUNCTIONS-1})
}
fn object_schema(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn town_schema() -> Value {
    let stops = json!({"type":"array","minItems":1,"maxItems":MAX_ROUTE_STOPS,"items":bounded_index(),"description":"Required delivery junctions, including the depot as the first entry."});
    let order = json!({"type":"array","minItems":1,"maxItems":MAX_ROUTE_STOPS,"items":bounded_index(),"description":"Delivery order, starting at the depot and including every required stop once. Road paths between deliveries may pass or revisit other stops."});
    object_schema(
        json!({"junctions":{"type":"integer","minimum":3,"maximum":MAX_ROUTE_JUNCTIONS,"description":"Junction count. Valid junction identifiers run from zero through junctions minus one."},"roads":{"type":"array","maxItems":MAX_ROUTE_ROADS,"description":"Two-way roads. Each connection can be traveled in either direction and must be declared once.","items":object_schema(json!({"from":bounded_index(),"to":bounded_index(),"cost":{"type":"integer","minimum":1,"maximum":MAX_ROUTE_ROAD_COST},"open":{"type":"boolean"}}), &["from","to","cost","open"])},"stops":stops,"order":order}),
        &["junctions", "roads", "stops", "order"],
    )
}

fn request_schema() -> Value {
    let snapshot = object_schema(
        json!({"revision":{"type":"integer","minimum":0},"current":town_schema(),"undo":{"type":"array","maxItems":MAX_ROUTE_UNDO,"items":town_schema()},"trace":{"oneOf":[{"type":"null"},object_schema(json!({"revision":{"type":"integer","minimum":0},"townIdentity":{"type":"array","minItems":32,"maxItems":32,"items":{"type":"integer","minimum":0,"maximum":255}},"from":bounded_index(),"to":bounded_index(),"cursor":{"type":"integer","minimum":0}}),&["revision","townIdentity","from","to","cursor"])]},"choices":{"oneOf":[{"type":"null"},object_schema(json!({"revision":{"type":"integer","minimum":0},"townIdentity":{"type":"array","minItems":32,"maxItems":32,"items":{"type":"integer","minimum":0,"maximum":255}},"cursor":{"type":"integer","minimum":0}}),&["revision","townIdentity","cursor"])]}}),
        &["revision", "current", "undo", "trace"],
    );
    let mut actions = Vec::new();
    for name in ["evaluate", "greedy", "improve", "undo"] {
        actions.push(object_schema(
            json!({"type":{"type":"string","enum":[name]}}),
            &["type"],
        ));
    }
    for (name, field, schema) in [
        (
            "road_cost",
            "cost",
            json!({"type":"integer","minimum":1,"maximum":MAX_ROUTE_ROAD_COST}),
        ),
        ("road_open", "open", json!({"type":"boolean"})),
    ] {
        let mut properties = json!({"type":{"type":"string","enum":[name]},"from":bounded_index(),"to":bounded_index()});
        properties[field] = schema;
        actions.push(object_schema(properties, &["type", "from", "to", field]));
    }
    for (name, field) in [("stops", "stops"), ("order", "order")] {
        let mut properties = json!({"type":{"type":"string","enum":[name]}});
        properties[field] =
            json!({"type":"array","minItems":1,"maxItems":MAX_ROUTE_STOPS,"items":bounded_index()});
        actions.push(object_schema(properties, &["type", field]));
    }
    actions.push(object_schema(
        json!({"type":{"type":"string","enum":["depot"]},"depot":bounded_index()}),
        &["type", "depot"],
    ));
    actions.push(object_schema(json!({"type":{"type":"string","enum":["trace"]},"from":bounded_index(),"to":bounded_index()}),&["type","from","to"]));
    actions.push(object_schema(
        json!({"type":{"type":"string","enum":["choices"]}}),
        &["type"],
    ));
    let mut choice_step = object_schema(
        json!({"type":{"type":"string","enum":["choice_step"]},"steps":{"type":"integer","minimum":0},"cursor":{"type":"integer","minimum":0}}),
        &["type"],
    );
    choice_step["not"] = json!({"required":["steps","cursor"]});
    actions.push(choice_step);
    let mut step = object_schema(
        json!({"type":{"type":"string","enum":["step"]},"steps":{"type":"integer","minimum":0},"cursor":{"type":"integer","minimum":0}}),
        &["type"],
    );
    step["not"] = json!({"required":["steps","cursor"]});
    actions.push(step);
    actions.push(object_schema(
        json!({"type":{"type":"string","enum":["network"]},"current":town_schema()}),
        &["type", "current"],
    ));
    actions.push(json!({"type":"string","enum":["open","remix","save"]}));
    object_schema(
        json!({"snapshot":snapshot,"capsule":{"type":"string","minLength":1,"maxLength":MAX_ROUTE_CAPSULE_BYTES},"action":{"oneOf":actions}}),
        &[],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_view_schema_accepts_real_prefixes_and_refuses_forged_view_fields() {
        let schema = output_schema();
        let mut response =
            route_json::response(&json!({"action":{"type":"trace","from":0,"to":3}})).unwrap();
        let event_count = response["trace"]["eventCount"].as_u64().unwrap();
        crate::validate_schema_value(&response, &schema, "", 0).unwrap();
        for expected_cursor in 1..=event_count {
            assert_eq!(response["next"]["tool"], "route_lab");
            response = route_json::response(&response["next"]["arguments"]).unwrap();
            crate::validate_schema_value(&response, &schema, "", 0).unwrap();
            assert_eq!(response["trace"]["cursor"], expected_cursor);
            assert_eq!(response["trace"]["eventCount"], event_count);
            assert_eq!(
                response["trace"]["completed"],
                expected_cursor == event_count
            );
            assert_eq!(
                response["trace"]["result"].is_null(),
                expected_cursor < event_count
            );
        }
        assert_eq!(response["trace"]["completed"], true);
        for (field, value) in [
            ("state", json!("certified")),
            ("cost", json!(-1)),
            ("predecessor", json!(999)),
            ("secret", json!(true)),
        ] {
            let mut forged = response.clone();
            forged["trace"]["view"]["junctions"][0][field] = value;
            assert!(crate::validate_schema_value(&forged, &schema, "", 0).is_err());
        }
        let mut forged = response;
        forged["trace"]["view"]["path"] = json!([0, 1, 3]);
        assert!(crate::validate_schema_value(&forged, &schema, "", 0).is_err());
    }

    #[test]
    fn choice_step_pointer_reveals_nearest_next_without_changing_the_player_order() {
        let schema = output_schema();
        let mut response = route_json::response(&json!({"action":{"type":"choices"}})).unwrap();
        let player_order = response["comparison"]["current"]["order"].clone();
        assert_eq!(player_order, json!([0, 1, 2, 3]));
        assert!(
            response["choices"]["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        crate::validate_schema_value(&response, &schema, "", 0).unwrap();
        let mut guard = 0;
        while response["choices"]["completed"] != true {
            assert_eq!(response["next"]["tool"], "route_lab");
            assert_eq!(
                response["next"]["arguments"]["action"]["type"],
                "choice_step"
            );
            response = route_json::response(&response["next"]["arguments"]).unwrap();
            crate::validate_schema_value(&response, &schema, "", 0).unwrap();
            guard += 1;
            assert!(guard <= 4, "choice playback did not finish");
        }
        assert_eq!(response["comparison"]["current"]["order"], player_order);
        assert_eq!(response["comparison"]["greedy"]["cost"], 9);
        assert_eq!(response["comparison"]["exact"]["tour"]["cost"], 8);
        assert_eq!(response["choices"]["prefixCost"], 9);
        let records = response["choices"]["records"].as_array().unwrap();
        assert_eq!(
            records
                .iter()
                .map(|record| record["chosen"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 0]
        );
        let at_b = &records[1];
        assert_eq!(at_b["current"], 1);
        assert_eq!(
            at_b["candidates"],
            json!([{"stop":2,"cost":2},{"stop":3,"cost":3}])
        );
        assert_eq!(at_b["chosen"], 2);
        let returning = records.last().unwrap();
        assert_eq!(returning["current"], 3);
        assert_eq!(returning["chosen"], 0);
        assert_eq!(returning["leg"], json!([3, 2, 0]));
        assert_eq!(returning["cost"], 4);
        let statement = response["choices"]["nearestStatement"].as_str().unwrap();
        assert!(statement.contains("costs more"));
        assert!(!statement.to_ascii_lowercase().contains("lost"));
        let shown = tool(&json!({"snapshot": response["snapshot"].clone()}));
        let prose = shown["content"][0]["text"].as_str().unwrap();
        assert!(
            prose.contains("At stop 3, candidates 0 (4); chosen 0; leg 3 -> 2 -> 0;"),
            "{prose}"
        );
        assert!(prose.contains("costs more"), "{prose}");
        let compact = crate::apply_response_mode("route_lab", Some("compact"), shown.clone());
        let compact_prose = compact["content"][0]["text"].as_str().unwrap();
        assert!(compact_prose.contains("leg 3 -> 2 -> 0"), "{compact_prose}");
        assert_eq!(
            compact["structuredContent"]["choices"]["records"],
            shown["structuredContent"]["choices"]["records"]
        );
        let hidden = route_json::response(&json!({"snapshot":response["snapshot"].clone(),"action":{"type":"choice_step","cursor":2}})).unwrap();
        crate::validate_schema_value(&hidden, &schema, "", 0).unwrap();
        assert_eq!(hidden["choices"]["records"].as_array().unwrap().len(), 2);
        assert_eq!(hidden["choices"]["records"][1]["current"], 1);
        assert!(
            hidden["choices"]["records"]
                .as_array()
                .unwrap()
                .iter()
                .all(|record| record["current"] != 3)
        );
        assert_eq!(hidden["comparison"]["greedy"]["cost"], 9);
        assert_eq!(hidden["comparison"]["exact"]["tour"]["cost"], 8);
        assert_eq!(hidden["comparison"]["current"]["order"], player_order);
        let tied =
            route_json::response(&json!({"action":{"type":"road_cost","from":1,"to":3,"cost":5}}))
                .unwrap();
        assert!(tied["choices"].is_null());
        let mut tied =
            route_json::response(&json!({"snapshot":tied["snapshot"],"action":{"type":"choices"}}))
                .unwrap();
        while tied["choices"]["completed"] != true {
            assert_eq!(tied["next"]["arguments"]["action"]["type"], "choice_step");
            tied = route_json::response(&tied["next"]["arguments"]).unwrap();
            crate::validate_schema_value(&tied, &schema, "", 0).unwrap();
        }
        assert_eq!(tied["comparison"]["greedy"]["cost"], 9);
        assert_eq!(tied["comparison"]["exact"]["tour"]["cost"], 9);
        let tie = tied["choices"]["nearestStatement"].as_str().unwrap();
        assert!(tie.contains("matches the exact minimum"));
        assert!(!tie.to_ascii_lowercase().contains("lost"));
        assert!(!tie.to_ascii_lowercase().contains("defeated"));
        let saved = route_json::response(&json!({"snapshot":response["snapshot"],"action":"save"}))
            .unwrap();
        let opened = route_json::response(&saved["next"]["arguments"]).unwrap();
        let remixed = route_json::response(&opened["creation"]["next"]["arguments"]).unwrap();
        let mut opened_choices = route_json::response(
            &json!({"snapshot":opened["snapshot"],"action":{"type":"choices"}}),
        )
        .unwrap();
        let mut remixed_choices = route_json::response(
            &json!({"snapshot":remixed["snapshot"],"action":{"type":"choices"}}),
        )
        .unwrap();
        while opened_choices["choices"]["completed"] != true {
            opened_choices = route_json::response(&opened_choices["next"]["arguments"]).unwrap();
            remixed_choices = route_json::response(&remixed_choices["next"]["arguments"]).unwrap();
        }
        assert_eq!(
            opened_choices["choices"]["records"],
            remixed_choices["choices"]["records"]
        );
        let mut forged = response["snapshot"].clone();
        forged["choices"]["townIdentity"] = json!(vec![7; 32]);
        let error = route_json::response(&json!({"snapshot":forged})).unwrap_err();
        assert!(error.contains("different street network"), "{error}");
        let mut legacy = response["snapshot"].clone();
        legacy.as_object_mut().unwrap().remove("choices");
        assert!(route_json::response(&json!({"snapshot":legacy})).is_ok());
    }
}
