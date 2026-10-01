//! Bounded JSON requests to the shared, stateless Route Lab workbench.

use super::route_json;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

#[cfg(test)]
fn report(request: Option<&str>, json_output: bool) -> Result<String, String> {
    report_with_output(request, json_output, None)
}

pub(super) fn report_with_output(
    request: Option<&str>,
    json_output: bool,
    out: Option<&Path>,
) -> Result<String, String> {
    let input = match request {
        Some("-") => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .lock()
                .take((route_json::MAX_REQUEST_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|error| format!("Could not read route request: {error}"))?;
            if bytes.len() > route_json::MAX_REQUEST_BYTES {
                return Err("Route request exceeds its byte bound.".into());
            }
            String::from_utf8(bytes).map_err(|_| "Route request must be UTF-8.")?
        }
        Some(value) => {
            if value.len() > route_json::MAX_REQUEST_BYTES {
                return Err("Route request exceeds its byte bound.".into());
            }
            value.to_string()
        }
        None => "{}".into(),
    };
    let request = serde_json::from_str(&input)
        .map_err(|error| format!("Invalid route request JSON: {error}"))?;
    let result = route_json::response(&request)?;
    if let Some(out) = out {
        let exported = if result.get("creation").is_some() {
            result.clone()
        } else {
            route_json::response(
                &serde_json::json!({"snapshot":result["snapshot"],"action":"save"}),
            )?
        };
        let capsule = exported["creation"]["capsule"]
            .as_str()
            .ok_or("Route export lacks its authored capsule.")?;
        super::studio::write_create_new(out, capsule.as_bytes())?;
    }
    if json_output {
        return serde_json::to_string_pretty(&result).map_err(|error| error.to_string());
    }
    let mut text = readable(&result)?;
    if let Some(out) = out {
        text.push_str(&format!(
            "Authored route exported to {}. Reopening starts a fresh session.\n",
            numinous_core::display_safe(&out.to_string_lossy())
        ));
    }
    Ok(text)
}

fn path_text(values: &Value) -> Result<String, String> {
    let values = values
        .as_array()
        .ok_or("Route report lacks its junction order.")?;
    let parts = values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .map(|value| value.to_string())
                .ok_or("Route report contains an invalid junction.")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join(" -> "))
}

fn diagnosis(value: &Value) -> &str {
    value["diagnosis"]
        .as_str()
        .unwrap_or("No feasible route is available.")
}

fn quantity(value: &Value, noun: &str) -> Result<String, String> {
    let count = value
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or("Route report contains an invalid quantity.")?;
    Ok(numinous_core::counted(count, noun))
}

/// Render the shared result without exposing state envelopes in human output.
fn readable(result: &Value) -> Result<String, String> {
    let network = &result["snapshot"]["current"];
    let roads = network["roads"]
        .as_array()
        .ok_or("Route report lacks its roads.")?;
    let open = roads.iter().filter(|road| road["open"] == true).count();
    let stops = network["stops"]
        .as_array()
        .ok_or("Route report lacks its required stops.")?;
    let mut lines = vec![
        "Route Lab".to_string(),
        format!(
            "Network: {}, {} of {} open, {}; depot {}.",
            numinous_core::counted(
                network["junctions"]
                    .as_u64()
                    .ok_or("Route report lacks its junction count.")? as usize,
                "junction"
            ),
            open,
            numinous_core::counted(roads.len(), "road"),
            numinous_core::counted(stops.len(), "required stop"),
            stops.first().ok_or("Route report lacks its depot.")?
        ),
        format!(
            "Your delivery order: {}; return to depot {}.",
            path_text(&network["order"])?,
            stops.first().ok_or("Route report lacks its depot.")?
        ),
    ];
    let comparison = &result["comparison"];
    if comparison["status"] == "feasible" {
        lines.push(format!(
            "Current round-trip cost: {}.",
            quantity(&comparison["current"]["cost"], "travel unit")?
        ));
        lines.push(format!(
            "Nearest next: {} round trip; delivery order {}.",
            quantity(&comparison["greedy"]["cost"], "travel unit")?,
            path_text(&comparison["greedy"]["order"])?
        ));
        lines.push(format!(
            "Exact minimum round-trip cost: {}; delivery order {}.",
            quantity(&comparison["exact"]["tour"]["cost"], "travel unit")?,
            path_text(&comparison["exact"]["tour"]["order"])?
        ));
        if !comparison["proposal"].is_null() {
            let proposal = &comparison["proposal"];
            lines.push(format!(
                "Improvement offered: {} round trip, cost change {}; delivery order {}.",
                quantity(&proposal["tour"]["cost"], "travel unit")?,
                proposal["delta"],
                path_text(&proposal["tour"]["order"])?
            ));
        } else {
            lines.push("No improving two-edge exchange is available for this order.".into());
        }
        lines.push(format!(
            "Street walk: {}.",
            path_text(&comparison["current"]["walk"])?
        ));
        lines.push(
            "Delivery order names planned visits; the street walk can pass or revisit stops."
                .into(),
        );
    } else {
        lines.push(format!("Route unavailable: {}.", diagnosis(comparison)));
        lines.push(match comparison["diagnostic"]["code"].as_str() {
            Some("unreachable")=>format!("Repair: reopen roads to connect junctions {} and {}, or supply a connected network with --request.",comparison["diagnostic"]["from"],comparison["diagnostic"]["to"]),
            Some("size")=>"Repair: add distinct required stops with the stops action.".into(),
            _=>"Repair: review the network and stop order with --json.".into(),
        });
    }
    let trace = &result["trace"];
    if !trace.is_null() {
        lines.push(format!(
            "Search playback {} -> {}: {} of {} revealed ({}).",
            trace["from"],
            trace["to"],
            trace["cursor"],
            quantity(&trace["eventCount"], "event")?,
            if trace["completed"] == true {
                "complete"
            } else {
                "paused"
            }
        ));
        for event in trace["events"]
            .as_array()
            .ok_or("Route report lacks its recorded events.")?
        {
            lines.push(match event["type"].as_str() {
                Some("settled") => format!(
                    "  Final cost from {} to {}: {}.",
                    trace["from"], event["junction"], event["cost"]
                ),
                Some("relaxed") => format!(
                    "  Tentative cost from {} to {} via {}: {}.",
                    trace["from"], event["to"], event["from"], event["cost"]
                ),
                _ => return Err("Route report contains an unknown recorded event.".into()),
            });
        }
        let outcome = &trace["result"];
        if outcome.is_null() {
            lines.push(
                "The shortest-path result appears after all recorded events are revealed.".into(),
            );
        } else if outcome["status"] == "reachable" {
            lines.push(format!(
                "Shortest path: {}; {}.",
                path_text(&outcome["junctions"])?,
                quantity(&outcome["cost"], "travel unit")?
            ));
        } else {
            lines.push(format!(
                "Shortest path unavailable: {}.",
                diagnosis(outcome)
            ));
        }
    }
    let action = &result["next"]["arguments"]["action"];
    lines.push(match action.as_str().or_else(|| action["type"].as_str()) {
        Some("open") => "Next: reopen the authored route in a fresh editing session.".into(),
        Some("remix") => "Next: create a child route with this creation as its parent.".into(),
        Some("improve") => "Next: accept the offered improvement.".into(),
        Some("step") => "Next: reveal one recorded solver event.".into(),
        Some("trace") => format!(
            "Next: start the shortest-path trace from {} to {}.",
            action["from"], action["to"]
        ),
        _ => return Err("Route report lacks an actionable continuation.".into()),
    });
    if let Some(creation) = result.get("creation") {
        lines.push(format!(
            "Route creation: {}. Opening starts a fresh session without undo or search playback.",
            creation["identityHex"]
                .as_str()
                .ok_or("Route report lacks its creation identity.")?
        ));
        if let Some(parent) = creation["parentIdentityHex"].as_str() {
            lines.push(format!("Parent creation: {parent}."));
        }
    }
    lines.push("Continue: repeat this request with --json, then pass the returned next.arguments JSON to numinous route-lab --request -.".into());
    Ok(format!("{}\n", lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_report_compares_the_player_order_without_dumping_state() {
        let report = report(None, false).unwrap();
        for expected in [
            "Network: 4 junctions, 5 of 5 roads open, 4 required stops; depot 0.",
            "Your delivery order: 0 -> 1 -> 2 -> 3; return to depot 0.",
            "Current round-trip cost: 9 travel units.",
            "Nearest next: 9 travel units",
            "Exact minimum round-trip cost: 8 travel units; delivery order 0 -> 2 -> 3 -> 1.",
            "Improvement offered: 8 travel units round trip, cost change -1",
            "Street walk: 0 -> 1 -> 2 -> 3 -> 2 -> 0.",
            "Delivery order names planned visits; the street walk can pass or revisit stops.",
            "Next: accept the offered improvement.",
            "next.arguments JSON to numinous route-lab --request -.",
        ] {
            assert!(report.contains(expected), "missing {expected}: {report}");
        }
        assert!(!report.contains("schemaVersion"));
        assert!(!report.contains("\"snapshot\""));
        assert!(!report.contains("town"));
        let structured: Value = serde_json::from_str(&super::report(None, true).unwrap()).unwrap();
        assert_eq!(structured, route_json::response(&json!({})).unwrap());
    }

    #[test]
    fn human_report_distinguishes_optimal_order_and_retained_infeasible_network() {
        let first = route_json::response(&json!({})).unwrap();
        let improved = route_json::response(&first["next"]["arguments"]).unwrap();
        let text = readable(&improved).unwrap();
        assert!(text.contains("Current round-trip cost: 8 travel units."));
        assert!(text.contains("No improving two-edge exchange"));
        assert!(text.contains("Next: start the shortest-path trace from 0 to 1."));
        let mut snapshot = first["snapshot"].clone();
        for destination in [1, 2] {
            snapshot=route_json::response(&json!({"snapshot":snapshot,"action":{"type":"road_open","from":0,"to":destination,"open":false}})).unwrap()["snapshot"].clone();
        }
        let text = readable(&route_json::response(&json!({"snapshot":snapshot})).unwrap()).unwrap();
        assert!(text.contains("3 of 5 roads open"));
        assert!(text.contains("Route unavailable:"));
        assert!(text.contains("Repair: reopen roads to connect junctions 0 and 1"));
        assert!(!text.contains("Exact minimum round-trip cost:"));
        let short = route_json::response(&json!({"action":{"type":"stops","stops":[0]}})).unwrap();
        assert!(
            readable(&short)
                .unwrap()
                .contains("Repair: add distinct required stops with the stops action.")
        );
    }

    #[test]
    fn trace_report_reveals_actual_events_and_only_completed_results() {
        let start =
            route_json::response(&json!({"action":{"type":"trace","from":0,"to":3}})).unwrap();
        let text = readable(&start).unwrap();
        assert!(text.contains("events revealed (paused)"));
        assert!(text.contains("Search playback 0 -> 3:"));
        assert!(text.contains("Next: reveal one recorded solver event."));
        assert!(!text.contains("Shortest path:"));
        let step = route_json::response(&start["next"]["arguments"]).unwrap();
        assert!(
            readable(&step)
                .unwrap()
                .contains("Final cost from 0 to 0: 0.")
        );
        let finish=route_json::response(&json!({"snapshot":step["snapshot"],"action":{"type":"step","cursor":step["trace"]["eventCount"]}})).unwrap();
        let text = readable(&finish).unwrap();
        assert!(text.contains("events revealed (complete)"));
        assert!(text.contains("Tentative cost from 0 to 1 via 0: 1."));
        assert!(text.contains("Tentative cost from 0 to 3 via 1: 4."));
        assert!(text.contains("Final cost from 0 to 3: 4."));
        assert!(text.contains("Shortest path: 0 -> 1 -> 3; 4 travel units."));
        let short =
            route_json::response(&json!({"action":{"type":"trace","from":0,"to":1}})).unwrap();
        let short=route_json::response(&json!({"snapshot":short["snapshot"],"action":{"type":"step","cursor":short["trace"]["eventCount"]}})).unwrap();
        assert!(
            readable(&short)
                .unwrap()
                .contains("Shortest path: 0 -> 1; 1 travel unit.")
        );
        let mut disconnected = start["snapshot"].clone();
        disconnected["trace"] = Value::Null;
        for road in disconnected["current"]["roads"].as_array_mut().unwrap() {
            road["open"] = json!(false);
        }
        let trace = route_json::response(
            &json!({"snapshot":disconnected,"action":{"type":"trace","from":0,"to":3}}),
        )
        .unwrap();
        let complete=route_json::response(&json!({"snapshot":trace["snapshot"],"action":{"type":"step","cursor":trace["trace"]["eventCount"]}})).unwrap();
        assert!(
            readable(&complete)
                .unwrap()
                .contains("Shortest path unavailable:")
        );
        assert!(
            readable(&complete)
                .unwrap()
                .contains("1 of 1 event revealed (complete)")
        );
    }
}
