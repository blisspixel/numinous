//! MCP projection for the simulation catalog.
//!
//! Simulation definitions, lever bounds, rendering, and readouts remain in
//! core. This module owns only the MCP-facing list and run argument boundary.

use numinous_core::Canvas;
use serde_json::{Value, json};

use crate::{DEFAULT_HEIGHT, DEFAULT_WIDTH, tool_error, tool_structured};

/// The `list_sims` text: each simulation with its levers.
pub(super) fn list_sims_text() -> String {
    numinous_core::all_sims()
        .iter()
        .map(|sim| {
            let meta = sim.meta();
            let levers: Vec<String> = meta
                .levers
                .iter()
                .map(|lever| format!("{}=[{}..{}]", lever.name, lever.min, lever.max))
                .collect();
            format!("{}  {}  levers: {}", meta.id, meta.title, levers.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a simulation at the supplied lever values and project its readout.
pub(super) fn run_sim_tool(args: &Value) -> Value {
    if let Some(map) = args.as_object() {
        for key in map.keys() {
            // Accept "levers" as an alias for "params": list_sims labels these
            // controls "levers:", so a mind that reads there and passes
            // "levers" should not hit a wall over vocabulary.
            if key != "id" && key != "params" && key != "levers" {
                return tool_error(&format!(
                    "Unknown argument '{key}'. Lever values go inside 'params' (also accepted: 'levers'), for example {{\"id\": \"wing\", \"params\": {{\"angle-of-attack\": 12}}}}."
                ));
            }
        }
    }
    // `params` and `levers` are the same slot; if both are given, one would be
    // silently dropped, so guide instead of quietly losing half the settings.
    if args.get("params").is_some() && args.get("levers").is_some() {
        return tool_error(
            "Pass lever values in one of 'params' or 'levers', not both; they are the same argument.",
        );
    }
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(sim) = numinous_core::sim_by_id(id) else {
        return tool_error(&unknown_sim(id));
    };
    let meta = sim.meta();
    let mut params = numinous_core::default_params(&meta);
    if let Some(value) = args.get("params").or_else(|| args.get("levers")) {
        let Some(obj) = value.as_object() else {
            return tool_error("Argument 'params' must be an object of lever names to numbers.");
        };
        for (name, value) in obj {
            let Some((index, lever)) = meta
                .levers
                .iter()
                .enumerate()
                .find(|(_, lever)| lever.name == name)
            else {
                let allowed = meta
                    .levers
                    .iter()
                    .map(|lever| lever.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                return tool_error(&format!(
                    "Unknown lever '{name}' for {id}. Available levers: {allowed}."
                ));
            };
            let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
                return tool_error(&format!("Lever '{name}' must be a finite number."));
            };
            if !(lever.min..=lever.max).contains(&number) {
                return tool_error(&format!(
                    "Lever '{name}' must be between {} and {} {}.",
                    lever.min, lever.max, lever.unit
                ));
            }
            params[index] = number;
        }
    }
    let mut canvas = Canvas::new(DEFAULT_WIDTH as usize, DEFAULT_HEIGHT as usize / 2);
    sim.render(&mut canvas, &params);
    let render = canvas.to_text();
    let readout = sim.readout(&params);
    tool_structured(
        &format!("{}\n\n{render}\n{readout}", meta.title),
        json!({
            "sim": id,
            "title": meta.title,
            // The render and plain readout ride in the structured payload, so
            // a structured-content-only client sees what the levers did.
            "render": render,
            "readout": readout,
            "params": meta
                .levers
                .iter()
                .enumerate()
                .map(|(i, lever)| json!({ "lever": lever.name, "value": params[i] }))
                .collect::<Vec<_>>()
        }),
    )
}

fn unknown_sim(id: &str) -> String {
    let known: Vec<&str> = numinous_core::all_sims()
        .iter()
        .map(|sim| sim.meta().id)
        .collect();
    format!("No sim with id '{id}'. Known sims: {}", known.join(", "))
}
