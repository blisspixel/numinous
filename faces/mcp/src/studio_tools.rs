//! MCP projection for Formula Jam and portable Studio creations.
//!
//! Parsing, evaluation, rendering, melody construction, capsule identity, and
//! lineage remain in core. This module owns the MCP-facing argument boundary,
//! portable result shape, optional audio attachment, and encounter receipt.

use serde_json::{Value, json};

use crate::encounter::{
    issue_receipt, receipt_json, request as encounter_request,
    sing_action as encounter_sing_action, sing_action_json, sing_result as encounter_sing_result,
};
use crate::{MAX_TOOL_HEIGHT, MAX_TOOL_WIDTH, audible, note_name, tool_error, tool_structured};

/// Formula Jam discovery and still plots.
pub(super) fn plot_expression_tool(args: &Value) -> Value {
    if args.get("list_recipes").and_then(Value::as_bool) == Some(true) {
        let recipes: Vec<Value> = numinous_core::STUDIO_RECIPES
            .iter()
            .enumerate()
            .map(|(i, source)| json!({ "index": i, "expr": source }))
            .collect();
        let lines: Vec<String> = numinous_core::STUDIO_RECIPES
            .iter()
            .enumerate()
            .map(|(i, source)| format!("  {i}: {source}"))
            .collect();
        return tool_structured(
            &format!(
                "Formula Jam curated recipes ({}):\n{}",
                numinous_core::studio_recipe_count(),
                lines.join("\n")
            ),
            json!({
                "discovery": "list",
                "recipeCount": numinous_core::studio_recipe_count(),
                "recipes": recipes,
                "valid": true
            }),
        );
    }

    let has_expr = args.get("expr").and_then(Value::as_str).is_some();
    let has_recipe = args.get("recipe").is_some();
    let has_seed = args.get("seed").is_some();
    let has_auto_step = args.get("auto_step").is_some();
    let mode_count = usize::from(has_expr) + usize::from(has_recipe) + usize::from(has_seed);
    if mode_count != 1 {
        return tool_error(
            "Provide exactly one of: expr (manual), recipe (index), or seed (random bank). Use list_recipes true to inspect the bank.",
        );
    }
    if has_auto_step && !has_seed {
        return tool_error("auto_step requires seed (stateless Auto walk over the curated bank).");
    }

    let source = if has_expr {
        numinous_core::PlotSource::Manual(
            args.get("expr")
                .and_then(Value::as_str)
                .expect("expr present")
                .to_string(),
        )
    } else if has_recipe {
        let Some(index) = args.get("recipe").and_then(Value::as_u64) else {
            return tool_error("Argument 'recipe' must be a non-negative integer.");
        };
        numinous_core::PlotSource::Recipe(index)
    } else {
        let Some(seed) = args.get("seed").and_then(Value::as_u64) else {
            return tool_error("Argument 'seed' must be a non-negative integer.");
        };
        let step = args.get("auto_step").and_then(Value::as_u64).unwrap_or(0);
        numinous_core::PlotSource::Seeded {
            seed,
            auto_step: has_auto_step.then_some(step),
        }
    };

    let request = match numinous_core::PlotRequest::new(
        source,
        args.get("xmin").and_then(Value::as_f64),
        args.get("xmax").and_then(Value::as_f64),
        args.get("a").and_then(Value::as_f64),
        None,
        None,
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    match request.execute() {
        Ok(result) => {
            let expr = request.source();
            let discovery = request.discovery().as_str();
            let xmin = request.xmin();
            let xmax = request.xmax();
            let a = request.parameter();
            let summary = format!(
                "y = {expr}    x in [{xmin:.3}, {xmax:.3}]    y in [{:.3}, {:.3}]\nDiscovery: {discovery}\n\n{}",
                result.ymin, result.ymax, result.text
            );
            tool_structured(
                &summary,
                json!({
                    "expression": expr,
                    "discovery": discovery,
                    "recipeIndex": request.recipe_index(),
                    "recipeCount": numinous_core::studio_recipe_count(),
                    "a": a,
                    "xmin": xmin,
                    "xmax": xmax,
                    "ymin": result.ymin,
                    "ymax": result.ymax,
                    "valid": true,
                    "plot": result.text
                }),
            )
        }
        Err(numinous_core::StudioRequestError::Undefined) => {
            tool_error("Nothing to plot: the function is undefined across this range.")
        }
        Err(error) => tool_error(&error.to_string()),
    }
}

/// Build a portable Studio capsule without granting the MCP face filesystem
/// access. The complete `.num` document and native link travel in the result.
pub(super) fn save_creation_tool(args: &Value) -> Value {
    let Some(source) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let mut creation = match numinous_core::StudioCreation::new(
        source,
        args.get("xmin")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN),
        args.get("xmax")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX),
        args.get("a")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_PARAMETER),
    ) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&error),
    };
    if let Some(title) = args.get("title").and_then(Value::as_str) {
        creation = match creation.with_title(title) {
            Ok(creation) => creation,
            Err(error) => return tool_error(&error),
        };
    }
    if let Some(author) = args.get("author").and_then(Value::as_str) {
        creation = match creation.with_author(author) {
            Ok(creation) => creation,
            Err(error) => return tool_error(&error),
        };
    }
    if let Some(raw_era) = args.get("era").and_then(Value::as_str) {
        let Some(era) = numinous_core::Era::parse(raw_era) else {
            return tool_error("Argument 'era' must be phosphor, 8-bit, vector, or modern.");
        };
        creation = creation.with_era(era);
    }
    studio_creation_result("save", &creation, None, args)
}

/// Open caller-supplied capsule data. A path-shaped string remains data and is
/// refused by the capsule parser rather than becoming an ambient file read.
pub(super) fn open_creation_tool(args: &Value) -> Value {
    let Some(capsule) = args.get("capsule").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'capsule'.");
    };
    let creation = match numinous_core::StudioCreation::from_capsule(capsule) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&format!("Could not open Studio capsule: {error}")),
    };
    studio_creation_result("open", &creation, None, args)
}

/// Make one child through the same core fork constructor the CLI uses.
pub(super) fn fork_creation_tool(args: &Value) -> Value {
    let Some(capsule) = args.get("parent").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'parent'.");
    };
    let parent = match numinous_core::StudioCreation::from_capsule(capsule) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&format!("Could not open parent capsule: {error}")),
    };
    let parent_link = parent.to_link();
    let child = match parent.fork(
        args.get("expr").and_then(Value::as_str),
        args.get("title").and_then(Value::as_str),
        args.get("author").and_then(Value::as_str),
    ) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&error),
    };
    studio_creation_result("fork", &child, Some(&parent_link), args)
}

fn studio_preview_size(args: &Value) -> Result<(usize, usize), String> {
    let read = |name: &str, default: usize, maximum: usize| {
        let Some(value) = args.get(name) else {
            return Ok(default);
        };
        let value = value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| format!("Argument '{name}' must be a non-negative integer."))?;
        if !(2..=maximum).contains(&value) {
            return Err(format!(
                "Argument '{name}' must be an integer from 2 through {maximum}."
            ));
        }
        Ok(value)
    };
    Ok((
        read(
            "width",
            numinous_core::DEFAULT_PLOT_WIDTH,
            MAX_TOOL_WIDTH as usize,
        )?,
        read(
            "height",
            numinous_core::DEFAULT_PLOT_HEIGHT,
            MAX_TOOL_HEIGHT as usize,
        )?,
    ))
}

fn studio_creation_result(
    action: &str,
    creation: &numinous_core::StudioCreation,
    parent_link: Option<&str>,
    args: &Value,
) -> Value {
    let (width, height) = match studio_preview_size(args) {
        Ok(size) => size,
        Err(error) => return tool_error(&error),
    };
    let request = match numinous_core::PlotRequest::new(
        numinous_core::PlotSource::Manual(creation.source().to_string()),
        Some(creation.xmin()),
        Some(creation.xmax()),
        Some(creation.a()),
        Some(width),
        Some(height),
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    let preview = match request.execute() {
        Ok(preview) => preview,
        Err(numinous_core::StudioRequestError::Undefined) => {
            return tool_error(&format!(
                "Cannot {action} this Studio creation: the function is undefined across its saved range."
            ));
        }
        Err(error) => return tool_error(&error.to_string()),
    };

    let num_file = creation.to_num_file();
    let link = creation.to_link();
    if link.chars().count() > numinous_core::MAX_JOURNAL_SUBJECT_CHARS {
        return tool_error("The canonical Studio link exceeds the journal subject bound.");
    }
    let capsule_format_version = if num_file.starts_with("NUMINOUS_STUDIO 2\n") {
        2
    } else {
        1
    };
    let verb = match action {
        "save" => "Saved",
        "open" => "Opened",
        "fork" => "Forked",
        _ => "Prepared",
    };
    let mut structured = json!({
        "schema": "numinous.studio-creation",
        "schemaVersion": 1,
        "action": action,
        "capsuleFormatVersion": capsule_format_version,
        "expression": creation.source(),
        "xmin": creation.xmin(),
        "xmax": creation.xmax(),
        "a": creation.a(),
        "title": creation.title(),
        "author": creation.author(),
        "era": creation.era().map(numinous_core::Era::name),
        "descends": creation.descends(),
        "numFile": num_file,
        "link": link,
        "journalSubject": link,
        "createdFile": false,
        "readHostFile": false,
        "containsHostPath": false,
        "preview": {
            "width": width,
            "height": height,
            "ymin": preview.ymin,
            "ymax": preview.ymax,
            "render": preview.text,
        }
    });
    if let Some(parent_link) = parent_link {
        structured["parentLink"] = json!(parent_link);
    }
    tool_structured(
        &format!(
            "{verb} Studio creation as portable capsule data. No host file was read or created.\nExpression: {}\nLink: {}\n\n{}",
            creation.source(),
            creation.to_link(),
            preview.text
        ),
        structured,
    )
}

/// Turn an agent's function into readable music and optional audio.
pub(super) fn sing_expression_tool(args: &Value) -> Value {
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let Some(source) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let notes = match args.get("notes").and_then(Value::as_u64) {
        Some(notes @ 1..=64) => Some(notes as usize),
        Some(_) => return tool_error("Argument 'notes' must be an integer from 1 through 64."),
        None => None,
    };
    let request = match numinous_core::SingRequest::new(
        source,
        args.get("xmin").and_then(Value::as_f64),
        args.get("xmax").and_then(Value::as_f64),
        args.get("a").and_then(Value::as_f64),
        notes,
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    let spec = match request.execute() {
        Ok(spec) => spec,
        Err(numinous_core::StudioRequestError::Undefined) => {
            return tool_error("Nothing to sing: the function is undefined across this range.");
        }
        Err(error) => return tool_error(&error.to_string()),
    };
    let mut lines = vec![format!(
        "y = {source} as a melody: {:.1}s, {} notes. Each line names the step \
         taken to reach it: the size measured in cents, the equal-tempered \
         name when one is near enough, and the whole number ratio when one is, \
         with how far off it sits.",
        spec.duration,
        spec.notes.len()
    )];
    let mut steps = Vec::with_capacity(spec.notes.len().saturating_sub(1));
    for (i, note) in spec.notes.iter().enumerate() {
        let step = i
            .checked_sub(1)
            .and_then(|previous| spec.notes.get(previous))
            .and_then(|previous| {
                numinous_core::Interval::between(f64::from(previous.freq), f64::from(note.freq))
            });
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s{}",
            i + 1,
            note.freq,
            note_name(note.freq),
            note.start,
            match step.as_ref() {
                Some(step) => format!("  [{}]", step.describe()),
                None => String::new(),
            }
        ));
        if let Some(step) = step {
            steps.push(interval_value(&step));
        }
    }
    let audible = match audible::requested(args) {
        Ok(true) => match audible::block(&spec) {
            Ok(rendered) => Some(rendered),
            Err(message) => return tool_error(&message),
        },
        Ok(false) => None,
        Err(message) => return tool_error(&message),
    };
    if audible.is_some() {
        lines.push(
            "A WAV of this melody follows as an audio attachment. It is \
             the only part of this reply that is not a description of the \
             melody, and it is a sound sent rather than a sound heard: \
             whether your client can surface it is its answer to give."
                .to_string(),
        );
    }
    let mut structured = json!({
        "expr": source,
        "duration_seconds": spec.duration,
        "notes": spec.notes.iter().enumerate().map(|(index, note)| json!({
            "index": index + 1,
            "frequency_hz": note.freq,
            "name": note_name(note.freq),
            "start_seconds": note.start,
            "duration_seconds": note.dur,
            "amplitude": note.amp,
        })).collect::<Vec<_>>(),
        "steps": steps,
        "audio": audible.as_ref().map(|(_, described)| described.clone()),
    });
    if want_receipt {
        let audio_asked = args.get("audio").and_then(Value::as_bool).unwrap_or(false);
        let action = encounter_sing_action(
            source,
            args.get("xmin")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN),
            args.get("xmax")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX),
            args.get("a")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_PARAMETER),
            notes.unwrap_or(numinous_core::DEFAULT_MELODY_NOTES) as u64,
            audio_asked,
        );
        let result = encounter_sing_result(
            source,
            spec.duration.into(),
            spec.notes.len() as u64,
            structured
                .get("audio")
                .and_then(|value| value.get("encodedBytes"))
                .and_then(Value::as_u64),
        );
        match issue_receipt(
            numinous_core::EncounterTool::SingExpression,
            &action.canonical_bytes(),
            &result.canonical_bytes(),
        ) {
            Ok(receipt) => {
                structured["encounter"] = receipt_json(&receipt, sing_action_json(&action))
            }
            Err(message) => return tool_error(&message),
        }
        lines.push("Encounter receipt attached.".to_string());
    }
    let result = tool_structured(&lines.join("\n"), structured);
    match audible {
        Some((block, _)) => audible::attach(result, block),
        None => result,
    }
}

/// Project one measured step between notes into typed evidence.
fn interval_value(step: &numinous_core::Interval) -> Value {
    json!({
        "cents": (step.cents * 10.0).round() / 10.0,
        "direction": step.direction.label(),
        "name": step.name,
        "ratio": step.ratio.map(|ratio| json!({
            "numerator": ratio.numerator,
            "denominator": ratio.denominator,
            "centsOff": ratio.cents_off,
        })),
    })
}
