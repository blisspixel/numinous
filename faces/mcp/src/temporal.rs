//! Bounded MCP projection for exact two-phase room evidence.

use numinous_broadcast::PLAY_ROOM_MAX_TEMPORAL_CELLS;
use numinous_core::{RenderDelta, TemporalPair};
use serde_json::{Value, json};

pub(super) const TEMPORAL_EVIDENCE_SCHEMA: &str = "numinous.temporal-evidence";
pub(super) const TEMPORAL_EVIDENCE_SCHEMA_VERSION: u64 = 1;

/// Parse the optional origin phase and enforce the public two-render budget.
pub(super) fn request(
    arguments: &Value,
    width: usize,
    height: usize,
) -> Result<Option<TemporalPair>, String> {
    let Some(value) = arguments.get("from_t") else {
        return Ok(None);
    };
    let Some(to_t) = arguments.get("t").and_then(Value::as_f64) else {
        return Err("Argument 'from_t' requires an explicit numeric destination 't'.".to_string());
    };
    let Some(from_t) = value.as_f64().filter(|phase| phase.is_finite()) else {
        return Err("Argument 'from_t' must be a finite phase in [0,1).".to_string());
    };
    let Some(pair) = TemporalPair::new(from_t, to_t) else {
        return Err("Arguments 'from_t' and 't' must be finite phases in [0,1).".to_string());
    };
    let cells = width
        .checked_mul(height)
        .ok_or_else(temporal_budget_error)?;
    if cells > PLAY_ROOM_MAX_TEMPORAL_CELLS as usize {
        return Err(temporal_budget_error());
    }
    Ok(Some(pair))
}

fn temporal_budget_error() -> String {
    format!(
        "Two-phase temporal evidence accepts at most {PLAY_ROOM_MAX_TEMPORAL_CELLS} cells per observation. Reduce width or height so their product is within that bound."
    )
}

/// Stable JSON shape for one cell-level render comparison.
pub(super) fn render_delta_json(delta: RenderDelta) -> Value {
    json!({
        "cells_changed": delta.cells_changed,
        "ink_added": delta.ink_added,
        "ink_removed": delta.ink_removed,
        "ink_reshaped": delta.ink_reshaped,
        "total_cells": delta.total_cells,
        "changed_region": delta.changed_region.map(|(x0, y0, x1, y1)| json!([x0, y0, x1, y1])),
    })
}

/// Additive structured evidence whose destination remains the top-level frame.
pub(super) fn evidence_json(
    pair: TemporalPair,
    from_status: Option<String>,
    from_render: String,
    delta: RenderDelta,
) -> Value {
    json!({
        "schema": TEMPORAL_EVIDENCE_SCHEMA,
        "schemaVersion": TEMPORAL_EVIDENCE_SCHEMA_VERSION,
        "fromT": pair.from_t(),
        "toT": pair.to_t(),
        "fromStatus": from_status,
        "fromRender": from_render,
        "delta": render_delta_json(delta),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        TEMPORAL_EVIDENCE_SCHEMA, TEMPORAL_EVIDENCE_SCHEMA_VERSION, evidence_json, request,
    };
    use numinous_core::{RenderDelta, TemporalPair};
    use serde_json::json;

    #[test]
    fn omitted_origin_preserves_single_observation_mode() {
        assert_eq!(request(&json!({"t": 0.4}), 512, 256), Ok(None));
    }

    #[test]
    fn origin_requires_explicit_valid_destination() {
        assert_eq!(
            request(&json!({"from_t": 0.2}), 72, 32),
            Err("Argument 'from_t' requires an explicit numeric destination 't'.".to_string())
        );
        for arguments in [
            json!({"from_t": -0.1, "t": 0.2}),
            json!({"from_t": 1.0, "t": 0.2}),
            json!({"from_t": 0.1, "t": 1.0}),
            json!({"from_t": "soon", "t": 0.2}),
            json!({"from_t": 0.1, "t": null}),
            json!({"from_t": 0.1, "t": "later"}),
        ] {
            assert!(request(&arguments, 72, 32).is_err());
        }
    }

    #[test]
    fn temporal_budget_is_checked_without_silent_clamping() {
        assert!(request(&json!({"from_t": 0.2, "t": 0.3}), 72, 32).is_ok());
        let error = request(&json!({"from_t": 0.2, "t": 0.3}), 73, 32).expect_err("over budget");
        assert!(error.contains("at most 2304 cells"));
    }

    #[test]
    fn evidence_names_its_version_direction_and_origin() {
        let pair = TemporalPair::new(0.2, 0.35).expect("pair");
        let evidence = evidence_json(
            pair,
            Some("origin".to_string()),
            "frame\n".to_string(),
            RenderDelta {
                cells_changed: 2,
                ink_added: 1,
                ink_removed: 0,
                ink_reshaped: 1,
                total_cells: 4,
                changed_region: Some((0, 0, 1, 0)),
            },
        );
        assert_eq!(evidence["schema"], TEMPORAL_EVIDENCE_SCHEMA);
        assert_eq!(evidence["schemaVersion"], TEMPORAL_EVIDENCE_SCHEMA_VERSION);
        assert_eq!(evidence["fromT"], 0.2);
        assert_eq!(evidence["toT"], 0.35);
        assert_eq!(evidence["fromStatus"], "origin");
        assert_eq!(evidence["fromRender"], "frame\n");
        assert_eq!(evidence["delta"]["cells_changed"], 2);
    }
}
