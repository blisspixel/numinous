//! Caller-carried Route Lab transport shared by the terminal and protocol faces.

use numinous_core::route::{
    MAX_ROUTE_ROADS, MAX_ROUTE_STOPS, Road, RouteError, RouteEvent, RouteTour,
};
use numinous_core::route_creation::RouteCreation;
use numinous_core::route_workbench::{
    EditableRoad, MAX_ROUTE_UNDO, RouteEdit, RouteSearchState, RouteSearchView, RouteTownSnapshot,
    RouteTraceSnapshot, RouteWorkbench, RouteWorkbenchSnapshot,
};
use serde_json::{Map, Value, json};

pub const MAX_REQUEST_BYTES: usize = 512 * 1024;

fn object<'a>(
    value: &'a Value,
    allowed: &[&str],
    required: &[&str],
) -> Result<&'a Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or("Route request field must be an object.")?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!(
            "Unknown route request field '{}'.",
            numinous_core::echoable_id(key)
        ));
    }
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("Missing route request field '{key}'."));
        }
    }
    Ok(object)
}

fn uint(value: &Value) -> Result<u64, String> {
    value.as_u64().ok_or_else(|| {
        "Route indices, costs, revisions and cursors must be nonnegative integers.".into()
    })
}

fn index(value: &Value) -> Result<usize, String> {
    usize::try_from(uint(value)?).map_err(|_| "Route integer exceeds this platform's bound.".into())
}

fn array(value: &Value, maximum: usize) -> Result<&[Value], String> {
    let values = value
        .as_array()
        .ok_or("Route request field must be an array.")?;
    if values.len() > maximum {
        return Err(format!("Route array exceeds its bound of {maximum}."));
    }
    Ok(values)
}

fn indices(value: &Value) -> Result<Vec<usize>, String> {
    array(value, MAX_ROUTE_STOPS)?.iter().map(index).collect()
}

fn town(value: &Value) -> Result<RouteTownSnapshot, String> {
    let value = object(
        value,
        &["junctions", "roads", "stops", "order"],
        &["junctions", "roads", "stops", "order"],
    )?;
    let roads = array(&value["roads"], MAX_ROUTE_ROADS)?
        .iter()
        .map(|road| {
            let road = object(
                road,
                &["from", "to", "cost", "open"],
                &["from", "to", "cost", "open"],
            )?;
            Ok(EditableRoad {
                road: Road {
                    from: index(&road["from"])?,
                    to: index(&road["to"])?,
                    cost: u32::try_from(uint(&road["cost"])?)
                        .map_err(|_| "Road cost exceeds its integer bound.")?,
                },
                open: road["open"]
                    .as_bool()
                    .ok_or("Road open must be a boolean.")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RouteTownSnapshot {
        junctions: index(&value["junctions"])?,
        roads,
        stops: indices(&value["stops"])?,
        order: indices(&value["order"])?,
    })
}

fn snapshot(value: &Value) -> Result<RouteWorkbenchSnapshot, String> {
    let value = object(
        value,
        &["revision", "current", "undo", "trace"],
        &["revision", "current", "undo", "trace"],
    )?;
    let trace = if value["trace"].is_null() {
        None
    } else {
        let trace = object(
            &value["trace"],
            &["revision", "townIdentity", "from", "to", "cursor"],
            &["revision", "townIdentity", "from", "to", "cursor"],
        )?;
        let identity = array(&trace["townIdentity"], 32)?
            .iter()
            .map(|byte| {
                u8::try_from(uint(byte)?)
                    .map_err(|_| "Trace identity must contain bytes.".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let town_identity: [u8; 32] = identity
            .try_into()
            .map_err(|_| "Trace identity must contain exactly 32 bytes.")?;
        Some(RouteTraceSnapshot {
            revision: uint(&trace["revision"])?,
            town_identity,
            from: index(&trace["from"])?,
            to: index(&trace["to"])?,
            cursor: index(&trace["cursor"])?,
        })
    };
    Ok(RouteWorkbenchSnapshot {
        revision: uint(&value["revision"])?,
        current: town(&value["current"])?,
        undo: array(&value["undo"], MAX_ROUTE_UNDO)?
            .iter()
            .map(town)
            .collect::<Result<Vec<_>, _>>()?,
        trace,
    })
}

fn town_json(town: &RouteTownSnapshot) -> Value {
    json!({"junctions":town.junctions,"roads":town.roads.iter().map(|road| json!({"from":road.road.from,"to":road.road.to,"cost":road.road.cost,"open":road.open})).collect::<Vec<_>>(),"stops":town.stops,"order":town.order})
}

fn snapshot_json(snapshot: &RouteWorkbenchSnapshot) -> Value {
    json!({"revision":snapshot.revision,"current":town_json(&snapshot.current),"undo":snapshot.undo.iter().map(town_json).collect::<Vec<_>>(),"trace":snapshot.trace.as_ref().map(|trace|json!({"revision":trace.revision,"townIdentity":trace.town_identity,"from":trace.from,"to":trace.to,"cursor":trace.cursor}))})
}

fn tour_json(tour: &RouteTour) -> Value {
    json!({"order":tour.order,"cost":tour.cost,"walk":tour.walk})
}

fn event_json(event: &RouteEvent) -> Value {
    match event {
        RouteEvent::Settled { junction, cost } => {
            json!({"type":"settled","junction":junction,"cost":cost})
        }
        RouteEvent::Relaxed { from, to, cost } => {
            json!({"type":"relaxed","from":from,"to":to,"cost":cost})
        }
    }
}

fn search_view_json(view: &RouteSearchView) -> Value {
    json!({
        "from": view.from,
        "to": view.to,
        "cursor": view.cursor,
        "eventCount": view.event_count,
        "completed": view.completed,
        "junctions": view.junctions.iter().map(|junction| json!({
            "junction": junction.junction,
            "cost": junction.cost,
            "predecessor": junction.predecessor,
            "state": match junction.state {
                RouteSearchState::Unseen => "unseen",
                RouteSearchState::Tentative => "tentative",
                RouteSearchState::Settled => "settled",
                RouteSearchState::Unreachable => "unreachable",
            },
        })).collect::<Vec<_>>(),
        "activeEvent": view.active_event.as_ref().map(event_json),
    })
}

fn diagnostic_json(error: &RouteError) -> Value {
    match error {
        RouteError::Size => json!({"code":"size"}),
        RouteError::Junction => json!({"code":"junction"}),
        RouteError::Road => json!({"code":"road"}),
        RouteError::DuplicateStop => json!({"code":"duplicate_stop"}),
        RouteError::Unreachable { from, to } => json!({"code":"unreachable","from":from,"to":to}),
        RouteError::Tour => json!({"code":"tour"}),
        RouteError::Arithmetic => json!({"code":"arithmetic"}),
    }
}

/// Translate wire shape, then delegate all town admission and operations to core.
pub fn response(request: &Value) -> Result<Value, String> {
    if serde_json::to_vec(request)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_REQUEST_BYTES
    {
        return Err(format!("Route request exceeds {MAX_REQUEST_BYTES} bytes."));
    }
    let args = object(request, &["snapshot", "action", "capsule"], &[])?;
    let creation_action = args.get("action").and_then(Value::as_str);
    let capsule = args
        .get("capsule")
        .map(|value| value.as_str().ok_or("Route capsule must be a string."))
        .transpose()?;
    if capsule.is_some() && creation_action.is_none() {
        return Err("A capsule requires action open, remix, or save.".into());
    }
    let mut creation = None;
    let mut workbench = match args.get("snapshot") {
        Some(value) => {
            RouteWorkbench::from_snapshot(snapshot(value)?).map_err(|error| error.to_string())?
        }
        None => RouteWorkbench::first_town(),
    };
    if let Some(action) = creation_action {
        match action {
            "open" | "remix" => {
                if args.contains_key("snapshot") {
                    return Err("Open and remix accept a capsule, not a session snapshot.".into());
                }
                let parent = RouteCreation::from_capsule(
                    capsule.ok_or("Open and remix require a route capsule.")?,
                )
                .map_err(|error| error.to_string())?;
                let authored = if action == "remix" {
                    parent
                        .remix(parent.town().clone())
                        .map_err(|error| error.to_string())?
                } else {
                    parent
                };
                workbench = authored.open();
                creation = Some(authored);
            }
            "save" => {
                creation = Some(if let Some(capsule) = capsule {
                    let existing =
                        RouteCreation::from_capsule(capsule).map_err(|error| error.to_string())?;
                    if !args.contains_key("snapshot") {
                        workbench = existing.open();
                    }
                    existing
                        .with_network(workbench.town().clone())
                        .map_err(|error| error.to_string())?
                } else {
                    RouteCreation::new(workbench.town().clone())
                        .map_err(|error| error.to_string())?
                });
            }
            _ => return Err("Route creation action must be open, remix, or save.".into()),
        }
    }
    let default = json!({"type":"evaluate"});
    let action = if creation_action.is_some() {
        &default
    } else {
        args.get("action").unwrap_or(&default)
    };
    let kind = action
        .get("type")
        .and_then(Value::as_str)
        .ok_or("Route action requires a string type.")?;
    let fields: &[&str] = match kind {
        "evaluate" | "greedy" | "improve" | "undo" => &["type"],
        "road_cost" => &["type", "from", "to", "cost"],
        "road_open" => &["type", "from", "to", "open"],
        "stops" => &["type", "stops"],
        "depot" => &["type", "depot"],
        "order" => &["type", "order"],
        "trace" => &["type", "from", "to"],
        "step" => &["type", "steps", "cursor"],
        "network" => &["type", "current"],
        _ => {
            return Err(format!(
                "Unknown route action '{}'.",
                numinous_core::echoable_id(kind)
            ));
        }
    };
    let required = if kind == "step" {
        &["type"][..]
    } else {
        fields
    };
    let action = object(action, fields, required)?;
    let changed = match kind {
        "evaluate" => false,
        "undo" => workbench.undo().map_err(|error| error.to_string())?,
        "trace" => {
            workbench
                .start_trace(index(&action["from"])?, index(&action["to"])?)
                .map_err(|error| error.to_string())?;
            false
        }
        "step" => {
            if action.contains_key("steps") && action.contains_key("cursor") {
                return Err("Choose steps or cursor, not both.".into());
            }
            let trace = workbench
                .trace()
                .ok_or("Start a trace before stepping it.")?;
            let cursor = if let Some(cursor) = action.get("cursor") {
                index(cursor)?
            } else {
                trace
                    .cursor()
                    .checked_add(action.get("steps").map(index).transpose()?.unwrap_or(1))
                    .ok_or("Trace cursor exceeds its integer bound.")?
            };
            workbench
                .seek_trace(cursor)
                .map_err(|error| error.to_string())?;
            false
        }
        _ => {
            let edit = match kind {
                "road_cost" => RouteEdit::RoadCost {
                    from: index(&action["from"])?,
                    to: index(&action["to"])?,
                    cost: u32::try_from(uint(&action["cost"])?)
                        .map_err(|_| "Road cost exceeds its integer bound.")?,
                },
                "road_open" => RouteEdit::RoadOpen {
                    from: index(&action["from"])?,
                    to: index(&action["to"])?,
                    open: action["open"]
                        .as_bool()
                        .ok_or("Road open must be a boolean.")?,
                },
                "stops" => RouteEdit::RequiredStops(indices(&action["stops"])?),
                "depot" => RouteEdit::Depot(index(&action["depot"])?),
                "order" => RouteEdit::Order(indices(&action["order"])?),
                "greedy" => RouteEdit::Greedy,
                "improve" => RouteEdit::Improve,
                "network" => RouteEdit::Network(town(&action["current"])?),
                _ => return Err("Unsupported route edit.".into()),
            };
            workbench.apply(edit).map_err(|error| error.to_string())?
        }
    };
    let comparison = workbench.compare();
    let offered = comparison
        .as_ref()
        .ok()
        .and_then(|result| result.proposal.as_ref())
        .is_some();
    let comparison_json = match comparison {
        Ok(result) => {
            json!({"status":"feasible","current":tour_json(&result.current),"greedy":tour_json(&result.greedy),"exact":{"tour":tour_json(&result.exact.tour),"statesCompleted":result.exact.states_completed},"proposal":result.proposal.map(|proposal|json!({"first":proposal.first,"second":proposal.second,"delta":proposal.delta,"tour":tour_json(&proposal.tour)}))})
        }
        Err(error) => {
            json!({"status":"infeasible","diagnostic":diagnostic_json(&error),"diagnosis":error.to_string()})
        }
    };
    let snapshot = snapshot_json(&workbench.snapshot());
    let trace = workbench.trace().map(|trace| json!({"from":trace.snapshot().from,"to":trace.snapshot().to,"cursor":trace.cursor(),"eventCount":trace.events().len(),"completed":trace.completed(),"events":trace.visible_events().iter().map(event_json).collect::<Vec<_>>(),"view":search_view_json(&trace.view()),"result":trace.result().map(|result|match result { Ok(path)=>json!({"status":"reachable","junctions":path.junctions,"cost":path.cost}), Err(error)=>json!({"status":"unreachable","diagnostic":diagnostic_json(error),"diagnosis":error.to_string()}) })}));
    let next_action = if workbench.trace().is_some_and(|trace| !trace.completed()) {
        json!({"type":"step","steps":1})
    } else if offered {
        json!({"type":"improve"})
    } else {
        json!({"type":"trace","from":workbench.town().stops[0],"to":workbench.town().stops.get(1).copied().unwrap_or(workbench.town().stops[0])})
    };
    let mut result = json!({"schema":"numinous.route-workbench","schemaVersion":1,"changed":changed,"snapshot":snapshot,"comparison":comparison_json,"trace":trace,"next":{"tool":"route_lab","arguments":{"snapshot":snapshot,"action":next_action}}});
    if let Some(creation) = creation {
        let capsule = creation.to_capsule();
        let parent = creation.parent_identity().map(|digest| {
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        });
        result["creation"] = json!({"capsule":capsule,"identityHex":creation.identity_hex(),"parentIdentityHex":parent,"next":{"tool":"route_lab","arguments":{"capsule":capsule,"action":"remix"}}});
        if creation_action == Some("save") {
            result["next"] =
                json!({"tool":"route_lab","arguments":{"capsule":capsule,"action":"open"}});
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search_snapshot() -> Value {
        json!({"revision":0,"undo":[],"trace":null,"current":{"junctions":5,"roads":[{"from":0,"to":1,"cost":5,"open":true},{"from":0,"to":2,"cost":1,"open":true},{"from":2,"to":1,"cost":1,"open":true},{"from":1,"to":3,"cost":1,"open":true}],"stops":[0,1,3],"order":[0,3,1]}})
    }

    #[test]
    fn search_view_projects_only_prefix_costs_and_replays_after_restore_and_backwards_seek() {
        let start = response(
            &json!({"snapshot":search_snapshot(),"action":{"type":"trace","from":0,"to":3}}),
        )
        .unwrap();
        let view = &start["trace"]["view"];
        assert_eq!(
            view["junctions"][0],
            json!({"junction":0,"state":"tentative","cost":0,"predecessor":null})
        );
        assert_eq!(
            view["junctions"][1],
            json!({"junction":1,"state":"unseen","cost":null,"predecessor":null})
        );
        assert!(view["activeEvent"].is_null());
        assert!(start["trace"]["result"].is_null());
        let first = response(&start["next"]["arguments"]).unwrap();
        assert_eq!(first["trace"]["view"]["junctions"][0]["state"], "settled");
        assert_eq!(
            first["trace"]["view"]["activeEvent"],
            first["trace"]["events"][0]
        );
        let early =
            response(&json!({"snapshot":first["snapshot"],"action":{"type":"step","cursor":2}}))
                .unwrap();
        assert_eq!(
            early["trace"]["view"]["junctions"][1],
            json!({"junction":1,"state":"tentative","cost":5,"predecessor":0})
        );
        assert_eq!(early["trace"]["view"]["junctions"][2]["state"], "unseen");
        let better =
            response(&json!({"snapshot":early["snapshot"],"action":{"type":"step","cursor":5}}))
                .unwrap();
        assert_eq!(
            better["trace"]["view"]["junctions"][1],
            json!({"junction":1,"state":"tentative","cost":2,"predecessor":2})
        );
        assert_eq!(better["trace"]["view"]["junctions"][3]["state"], "unseen");
        assert!(better["trace"]["result"].is_null());
        let restored = response(&json!({"snapshot":better["snapshot"]})).unwrap();
        assert_eq!(restored["trace"], better["trace"]);
        let backwards =
            response(&json!({"snapshot":restored["snapshot"],"action":{"type":"step","cursor":2}}))
                .unwrap();
        assert_eq!(backwards["trace"], early["trace"]);
        let mut cursor = backwards;
        let event_count = cursor["trace"]["eventCount"].as_u64().unwrap();
        let first_cursor = cursor["trace"]["cursor"].as_u64().unwrap() + 1;
        for expected_cursor in first_cursor..=event_count {
            assert_eq!(cursor["next"]["tool"], "route_lab");
            cursor = response(&cursor["next"]["arguments"]).unwrap();
            assert_eq!(cursor["trace"]["cursor"], expected_cursor);
            assert_eq!(cursor["trace"]["eventCount"], event_count);
            assert_eq!(cursor["trace"]["completed"], expected_cursor == event_count);
            assert_eq!(
                cursor["trace"]["result"].is_null(),
                expected_cursor < event_count
            );
        }
        assert_eq!(cursor["trace"]["completed"], true);
        assert_eq!(
            cursor["trace"]["view"]["junctions"][1],
            json!({"junction":1,"state":"settled","cost":2,"predecessor":2})
        );
        assert_eq!(
            cursor["trace"]["view"]["junctions"][3],
            json!({"junction":3,"state":"settled","cost":3,"predecessor":1})
        );
        assert_eq!(
            cursor["trace"]["view"]["junctions"][4],
            json!({"junction":4,"state":"unreachable","cost":null,"predecessor":null})
        );
        assert_eq!(
            cursor["trace"]["result"],
            json!({"status":"reachable","junctions":[0,2,1,3],"cost":3})
        );
        let rewind =
            response(&json!({"snapshot":cursor["snapshot"],"action":{"type":"step","cursor":0}}))
                .unwrap();
        assert_eq!(rewind["trace"], start["trace"]);
        assert_eq!(rewind["schemaVersion"], 1);
        let edited = response(&json!({"snapshot":cursor["snapshot"],"action":{"type":"road_cost","from":0,"to":1,"cost":4}})).unwrap();
        assert!(edited["trace"].is_null());
        assert!(edited["snapshot"]["trace"].is_null());
        let saved = response(&json!({"snapshot":cursor["snapshot"],"action":"save"})).unwrap();
        let saved_without_playback =
            response(&json!({"snapshot":search_snapshot(),"action":"save"})).unwrap();
        assert_eq!(saved["creation"], saved_without_playback["creation"]);
    }

    #[test]
    fn search_view_cannot_be_imported_as_an_authoritative_snapshot_claim() {
        let start = response(
            &json!({"snapshot":search_snapshot(),"action":{"type":"trace","from":0,"to":3}}),
        )
        .unwrap();
        let mut forged = start["snapshot"].clone();
        forged["trace"]["view"] = json!({"junctions":[{"junction":3,"state":"settled","cost":0}]});
        assert!(
            response(&json!({"snapshot":forged}))
                .unwrap_err()
                .contains("Unknown route request field")
        );
        for (field, value) in [("townIdentity", json!(vec![0; 32])), ("cursor", json!(999))] {
            let mut forged = start["snapshot"].clone();
            forged["trace"][field] = value;
            assert!(response(&json!({"snapshot":forged})).is_err());
        }
    }

    #[test]
    fn authored_save_reopens_fresh_and_remix_retains_exact_parent_identity() {
        let first = response(&json!({})).unwrap();
        assert!(first.get("creation").is_none());
        let edited = response(&first["next"]["arguments"]).unwrap();
        assert!(!edited["snapshot"]["undo"].as_array().unwrap().is_empty());
        let saved = response(&json!({"snapshot":edited["snapshot"],"action":"save"})).unwrap();
        let opened = response(&saved["next"]["arguments"]).unwrap();
        assert_eq!(opened["snapshot"]["current"], edited["snapshot"]["current"]);
        assert_eq!(opened["snapshot"]["revision"], 0);
        assert_eq!(opened["snapshot"]["undo"], json!([]));
        assert!(opened["snapshot"]["trace"].is_null());
        let remixed = response(&opened["creation"]["next"]["arguments"]).unwrap();
        assert_eq!(
            remixed["creation"]["parentIdentityHex"],
            opened["creation"]["identityHex"]
        );
        assert_ne!(
            remixed["creation"]["identityHex"],
            opened["creation"]["identityHex"]
        );
        let child=response(&json!({"snapshot":first["snapshot"],"capsule":remixed["creation"]["capsule"],"action":"save"})).unwrap();
        assert_eq!(
            child["creation"]["parentIdentityHex"],
            opened["creation"]["identityHex"]
        );
        for request in [
            json!({"capsule":saved["creation"]["capsule"]}),
            json!({"action":"open"}),
            json!({"action":"unknown"}),
            json!({"snapshot":first["snapshot"],"capsule":saved["creation"]["capsule"],"action":"open"}),
            json!({"action":"open","capsule":"/tmp/route.num"}),
        ] {
            assert!(response(&request).is_err(), "{request}");
        }
    }

    #[test]
    fn atomic_network_edit_changes_structure_and_undo_restores_it() {
        let first = response(&json!({})).unwrap();
        let mut network = first["snapshot"]["current"].clone();
        network["junctions"] = json!(5);
        network["roads"]
            .as_array_mut()
            .unwrap()
            .push(json!({"from":3,"to":4,"cost":2,"open":true}));
        network["stops"] = json!([0, 1, 4]);
        network["order"] = json!([0, 4, 1]);
        let changed = response(
            &json!({"snapshot":first["snapshot"],"action":{"type":"network","current":network}}),
        )
        .unwrap();
        assert_eq!(changed["snapshot"]["current"]["junctions"], 5);
        assert_eq!(changed["snapshot"]["current"]["order"], json!([0, 4, 1]));
        let saved = response(&json!({"snapshot":changed["snapshot"],"action":"save"})).unwrap();
        let remixed = response(&saved["creation"]["next"]["arguments"]).unwrap();
        let saved_again =
            response(&json!({"capsule":remixed["creation"]["capsule"],"action":"save"})).unwrap();
        assert_eq!(
            saved_again["creation"]["identityHex"],
            remixed["creation"]["identityHex"]
        );
        assert_eq!(
            saved_again["snapshot"]["current"],
            changed["snapshot"]["current"]
        );
        assert_eq!(
            saved_again["creation"]["parentIdentityHex"],
            saved["creation"]["identityHex"]
        );
        let reopened = response(&saved_again["next"]["arguments"]).unwrap();
        assert_eq!(
            reopened["snapshot"]["current"],
            changed["snapshot"]["current"]
        );
        let undone =
            response(&json!({"snapshot":changed["snapshot"],"action":{"type":"undo"}})).unwrap();
        assert_eq!(undone["snapshot"]["current"], first["snapshot"]["current"]);
    }

    #[test]
    fn delivery_order_is_not_the_street_walk_and_unused_junctions_need_not_connect() {
        let evaluated = response(&json!({"snapshot": {
            "revision":0,"undo":[],"trace":null,
            "current":{"junctions":5,"roads":[
                {"from":0,"to":1,"cost":1,"open":true},
                {"from":1,"to":2,"cost":2,"open":true},
                {"from":0,"to":2,"cost":4,"open":true}
            ],"stops":[0,1,2],"order":[0,2,1]}
        }}))
        .unwrap();
        assert_eq!(evaluated["comparison"]["status"], "feasible");
        assert_eq!(
            evaluated["comparison"]["current"]["order"],
            json!([0, 2, 1])
        );
        assert_eq!(
            evaluated["comparison"]["current"]["walk"],
            json!([0, 1, 2, 1, 0])
        );
        assert_eq!(evaluated["comparison"]["current"]["cost"], 6);
        assert_eq!(evaluated["snapshot"]["current"]["junctions"], 5);

        let mut traced = response(&json!({"snapshot":evaluated["snapshot"],
            "action":{"type":"trace","from":0,"to":4}}))
        .unwrap();
        let event_count = traced["trace"]["eventCount"].as_u64().unwrap();
        for _ in 0..event_count {
            traced = response(&traced["next"]["arguments"]).unwrap();
        }
        assert_eq!(traced["comparison"]["status"], "feasible");
        assert_eq!(traced["trace"]["completed"], true);
        assert_eq!(traced["trace"]["result"]["status"], "unreachable");
        assert_eq!(traced["trace"]["result"]["diagnostic"]["to"], 4);
    }

    #[test]
    fn edits_keep_closed_costs_and_evaluate_preserves_deliberate_orders() {
        let first = response(&json!({})).unwrap();
        let costly = response(&json!({"snapshot":first["snapshot"],"action":{"type":"road_cost","from":3,"to":1,"cost":5}})).unwrap();
        assert_eq!(costly["comparison"]["current"]["cost"], 9);
        let greedy =
            response(&json!({"snapshot":costly["snapshot"],"action":{"type":"greedy"}})).unwrap();
        assert_eq!(greedy["comparison"]["current"]["cost"], 9);
        let closed = response(&json!({"snapshot":greedy["snapshot"],"action":{"type":"road_open","from":1,"to":3,"open":false}})).unwrap();
        let road = closed["snapshot"]["current"]["roads"]
            .as_array()
            .unwrap()
            .iter()
            .find(|road| road["from"] == 1 && road["to"] == 3)
            .unwrap();
        assert_eq!(road["cost"], 5);
        assert_eq!(road["open"], false);
        let order = response(
            &json!({"snapshot":closed["snapshot"],"action":{"type":"order","order":[0,3,1,2]}}),
        )
        .unwrap();
        let evaluated = response(&json!({"snapshot":order["snapshot"]})).unwrap();
        assert_eq!(
            evaluated["snapshot"]["current"]["order"],
            json!([0, 3, 1, 2])
        );
        assert_eq!(evaluated["changed"], false);
        let stops = response(
            &json!({"snapshot":evaluated["snapshot"],"action":{"type":"stops","stops":[0,2,3]}}),
        )
        .unwrap();
        assert_eq!(stops["snapshot"]["current"]["order"], json!([0, 3, 2]));
        let undone =
            response(&json!({"snapshot":stops["snapshot"],"action":{"type":"undo"}})).unwrap();
        assert_eq!(undone["snapshot"]["current"]["order"], json!([0, 3, 1, 2]));
    }

    #[test]
    fn wire_bounds_and_structure_fail_before_admitting_a_snapshot() {
        for request in [
            json!(null),
            json!({"unknown":true}),
            json!({"action":[]}),
            json!({"action":{"type":"invented"}}),
            json!({"action":{"type":"step"}}),
            json!({"action":{"type":"step","steps":1,"cursor":0}}),
            json!({"action":{"type":"road_cost","from":0,"to":1,"cost":u64::MAX}}),
        ] {
            assert!(response(&request).is_err(), "{request}");
        }
        let first = response(&json!({})).unwrap();
        for (path, value) in [
            ("revision", json!(-1)),
            ("undo", json!([null])),
            ("current", json!({})),
            (
                "trace",
                json!({"revision":0,"from":0,"to":1,"cursor":0,"townIdentity":[0]}),
            ),
        ] {
            let mut snapshot = first["snapshot"].clone();
            snapshot[path] = value;
            assert!(response(&json!({"snapshot":snapshot})).is_err());
        }
        let mut snapshot = first["snapshot"].clone();
        snapshot["undo"] = json!(vec![snapshot["current"].clone(); MAX_ROUTE_UNDO + 1]);
        assert!(response(&json!({"snapshot":snapshot})).is_err());
        let huge = json!({"action":{"type":"x".repeat(MAX_REQUEST_BYTES)}});
        assert!(response(&huge).unwrap_err().contains("exceeds"));
    }
}
