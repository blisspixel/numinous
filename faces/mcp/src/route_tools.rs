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
        "description":"Choose a delivery order and compare its round-trip cost with nearest-next and the exact minimum for a street network. Start with no arguments, or carry the returned working state (snapshot) into another call. Roads are two-way; declare each connection once. Actions replace network with current, or edit road_cost, road_open, stops (depot first), depot, or order; greedy chooses the nearest next stop, improve accepts one cheaper reorder, and undo restores a prior edit. Evaluation preserves your order. Each leg follows the cheapest open road path and may pass or revisit other stops. If required stops cannot reach one another, the network stays editable with an infeasible comparison; disconnected unused junctions do not prevent a round trip. trace records a shortest-path search at cursor zero, and step reveals events with steps (default one) or an absolute cursor. Event costs are cumulative from the starting junction: settled is final, relaxed is tentative. The path result appears on completion. next is a followable call carrying snapshot. Editing state stays private and caller-carried. Scalar action save returns a NUMINOUS_ROUTE 1 creation from the current snapshot; an optional existing capsule preserves its parent identity while saving edits. Explicit remix creates the child. Scalar open or remix accepts capsule text and starts a fresh session. Portable creations carry the authored network, delivery order, and parent identity, with no undo or search playback. creation.next offers an explicit remix; save next reopens the saved creation. The project tool can keep that capsule using existing project persistence.",
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
        object_schema(json!({"status":{"type":"string","enum":["feasible"]},"current":tour,"greedy":tour,"exact":object_schema(json!({"tour":tour,"statesCompleted":integer}),&["tour","statesCompleted"]),"proposal":{"oneOf":[{"type":"null"},proposal]}}),&["status","current","greedy","exact","proposal"]),
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
    let trace = object_schema(
        json!({"from":bounded_index(),"to":bounded_index(),"cursor":integer,"eventCount":integer,"completed":{"type":"boolean"},"events":{"type":"array","items":event},"result":result}),
        &[
            "from",
            "to",
            "cursor",
            "eventCount",
            "completed",
            "events",
            "result",
        ],
    );
    let next = object_schema(
        json!({"tool":{"type":"string","enum":["route_lab"]},"arguments":request_schema()}),
        &["tool", "arguments"],
    );
    object_schema(
        json!({"schema":{"type":"string","enum":["numinous.route-workbench"]},"schemaVersion":{"type":"integer","enum":[1]},"changed":{"type":"boolean"},"snapshot":request_schema()["properties"]["snapshot"],"comparison":comparison,"trace":{"oneOf":[{"type":"null"},trace]},"next":next,"creation":object_schema(json!({"capsule":{"type":"string"},"identityHex":{"type":"string"},"parentIdentityHex":{"oneOf":[{"type":"null"},{"type":"string"}]},"next":next}),&["capsule","identityHex","parentIdentityHex","next"])}),
        &[
            "schema",
            "schemaVersion",
            "changed",
            "snapshot",
            "comparison",
            "trace",
            "next",
        ],
    )
}

pub(super) fn tool(arguments: &Value) -> Value {
    match route_json::response(arguments) {
        Ok(result) => {
            let text = if result["comparison"]["status"] == "feasible" {
                format!(
                    "Route Lab round-trip cost: yours {}, nearest next {}, minimum {}. Each delivery leg follows the cheapest open road path. Carry snapshot into the next call to continue.",
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
            tool_structured(&text, result)
        }
        Err(error) => tool_error(&format!("Invalid Route Lab request: {error}")),
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
        json!({"revision":{"type":"integer","minimum":0},"current":town_schema(),"undo":{"type":"array","maxItems":MAX_ROUTE_UNDO,"items":town_schema()},"trace":{"oneOf":[{"type":"null"},object_schema(json!({"revision":{"type":"integer","minimum":0},"townIdentity":{"type":"array","minItems":32,"maxItems":32,"items":{"type":"integer","minimum":0,"maximum":255}},"from":bounded_index(),"to":bounded_index(),"cursor":{"type":"integer","minimum":0}}),&["revision","townIdentity","from","to","cursor"])]}}),
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
