//! Validate the research atlas and rebuild its offline viewer and typed graph.
//!
//! Run from a checkout with `cargo run -p numinous-cli --example math_atlas`.
//! Append `-- --check` to reject stale outputs without writing them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use numinous_core::all_rooms;
use serde_json::{Value, json};

type Result<T> = std::result::Result<T, String>;

#[derive(Parser)]
#[command(about = "Validate research records and rebuild the offline atlas and typed graph")]
struct Args {
    /// Validate without writing, and reject stale outputs.
    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Atlas error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("Cannot locate the repository root")?;
    let atlas = root.join("docs/evidence/math-atlas");
    let mut data = load(&atlas.join("papers.json"))?;
    let sources = load(&atlas.join("sources.json"))?;
    data["opportunities"] = load(&atlas.join("opportunities.json"))?["items"].clone();
    validate(&data, &sources)?;
    let graph = graph_for(&data)?;
    let template = read(&root.join("scripts/templates/math-atlas.html"))?;
    let html = render_html(&template, &data, &graph)?;
    let graph_text = serde_json::to_string_pretty(&graph)
        .map_err(|error| format!("Cannot serialize graph: {error}"))?
        + "\n";
    let coverage = render_coverage(&data)?;
    let mut outputs = vec![
        ("graph.json".to_owned(), graph_text),
        ("index.html".to_owned(), html),
        ("coverage.md".to_owned(), coverage),
    ];
    for paper in array(&data, "papers")? {
        outputs.push((
            format!("papers/{}.md", text(paper, "paper_id")?),
            render_paper_note(paper, text(&data, "source_commit")?)?,
        ));
    }
    let note_dir = atlas.join("papers");
    let expected_notes: BTreeSet<_> = array(&data, "papers")?
        .iter()
        .map(|paper| text(paper, "paper_id").map(|id| format!("{id}.md")))
        .collect::<Result<_>>()?;
    if note_dir.is_dir() {
        let mut actual_notes = BTreeSet::new();
        for entry in fs::read_dir(&note_dir).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|extension| extension == "md") {
                actual_notes.insert(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .ok_or("Sidecar filename is not UTF-8")?
                        .to_owned(),
                );
            }
        }
        validate_note_names(&actual_notes, &expected_notes)?;
    }
    if !args.check {
        fs::create_dir_all(atlas.join("papers"))
            .map_err(|error| format!("Cannot create paper note directory: {error}"))?;
    }
    for (name, contents) in outputs {
        let path = atlas.join(&name);
        if args.check {
            if fs::read_to_string(&path).ok().as_deref() != Some(contents.as_str()) {
                return Err(format!("{name} is missing or stale; rebuild the atlas"));
            }
        } else {
            fs::write(&path, contents)
                .map_err(|error| format!("Cannot write {}: {error}", path.display()))?;
        }
    }
    println!(
        "Atlas valid: {} papers, {} nodes, {} typed edges",
        array(&data, "papers")?.len(),
        array(&graph, "nodes")?.len(),
        array(&graph, "edges")?.len()
    );
    Ok(())
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))
}

fn validate_note_names(actual: &BTreeSet<String>, expected: &BTreeSet<String>) -> Result<()> {
    if let Some(name) = actual.difference(expected).next() {
        return Err(format!(
            "Orphan paper sidecar {name}; reconcile it with the source catalog before rebuilding"
        ));
    }
    Ok(())
}

fn load(path: &Path) -> Result<Value> {
    serde_json::from_str(&read(path)?)
        .map_err(|error| format!("Invalid JSON in {}: {error}", path.display()))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{key} must be a nonempty string"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn strings<'a>(value: &'a Value, key: &str) -> Result<Vec<&'a str>> {
    array(value, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| format!("{key} must contain nonempty strings"))
        })
        .collect()
}

fn unique_index<'a>(items: &'a [Value], key: &str) -> Result<BTreeMap<&'a str, &'a Value>> {
    let mut index = BTreeMap::new();
    for item in items {
        let id = text(item, key)?;
        if index.insert(id, item).is_some() {
            return Err(format!("Duplicate {key}: {id}"));
        }
    }
    Ok(index)
}

fn validate(data: &Value, sources: &Value) -> Result<()> {
    let papers = unique_index(array(data, "papers")?, "paper_id")?;
    let expected = unique_index(array(sources, "papers")?, "paper_id")?;
    if !papers.keys().eq(expected.keys()) {
        return Err("Paper records must cover every indexed manuscript exactly once".into());
    }
    if text(data, "source_commit")? != text(sources, "source_commit")? {
        return Err("Source revisions disagree".into());
    }
    let rooms: BTreeSet<_> = all_rooms()
        .iter()
        .map(|room| room.meta().id.to_owned())
        .collect();
    let mut families = BTreeSet::new();
    let opportunities = unique_index(array(data, "opportunities")?, "id")?;
    validate_opportunities(&opportunities)?;
    for (pid, paper) in &papers {
        validate_paper(paper, expected[*pid], &rooms).map_err(|error| format!("{pid}: {error}"))?;
        validate_audit(paper, expected[*pid], &papers, &opportunities)
            .map_err(|error| format!("{pid}: {error}"))?;
        families.insert(text(paper, "family_id")?);
    }
    for (id, capability) in unique_index(array(data, "capabilities")?, "id")? {
        text(capability, "label")?;
        text(capability, "rationale")?;
        for family in strings(capability, "families")? {
            if !families.contains(family) {
                return Err(format!("{id}: unknown family {family}"));
            }
        }
    }
    Ok(())
}

fn validate_opportunities(items: &BTreeMap<&str, &Value>) -> Result<()> {
    for (id, item) in items {
        for key in [
            "title",
            "stage",
            "core_seam",
            "gesture",
            "game",
            "expert_workflow",
            "validation",
            "scope",
            "revisit_or_reject",
            "status",
        ] {
            text(item, key)?;
        }
        for dependency in strings(item, "dependencies")? {
            if dependency == *id || !items.contains_key(dependency) {
                return Err(format!("{id}: invalid dependency {dependency}"));
            }
        }
    }
    // A dependency cycle would leave a proposed slice impossible to select.
    let mut pending: BTreeSet<_> = items.keys().copied().collect();
    loop {
        let ready: Vec<_> = pending
            .iter()
            .copied()
            .filter(|id| {
                items[id]["dependencies"].as_array().is_some_and(|deps| {
                    deps.iter()
                        .all(|dep| !pending.contains(dep.as_str().unwrap_or("")))
                })
            })
            .collect();
        if ready.is_empty() {
            return if pending.is_empty() {
                Ok(())
            } else {
                Err("Opportunity dependencies contain a cycle".into())
            };
        }
        for id in ready {
            pending.remove(id);
        }
    }
}

fn validate_audit(
    paper: &Value,
    source: &Value,
    papers: &BTreeMap<&str, &Value>,
    opportunities: &BTreeMap<&str, &Value>,
) -> Result<()> {
    let audit = &paper["insight_audit"];
    if text(audit, "status")? != "source-anchored-review" {
        return Err("Missing source-anchored review".into());
    }
    for key in [
        "reusable_asset",
        "playful_loop",
        "expert_workflow",
        "disposition_reason",
    ] {
        text(audit, key)?;
    }
    if strings(audit, "open_questions")?.is_empty() {
        return Err("Review needs an explicit open question".into());
    }
    let retrieved: BTreeSet<_> = array(source, "retrieved_sources")?
        .iter()
        .map(|entry| text(entry, "path"))
        .collect::<Result<_>>()?;
    let sections = array(audit, "source_sections")?;
    if sections.is_empty() || array(audit, "extracted_insights")?.is_empty() {
        return Err("Review needs source sections and extracted insights".into());
    }
    for section in sections {
        let path = text(section, "source_path")?;
        if !retrieved.contains(path) {
            return Err(format!("Reviewed source lacks provenance: {path}"));
        }
        text(section, "locator")?;
        text(section, "read_for")?;
    }
    for insight in array(audit, "extracted_insights")? {
        for key in [
            "kind",
            "insight",
            "source_locator",
            "numinous_application",
            "limitation",
        ] {
            text(insight, key)?;
        }
    }
    let mut linked = BTreeSet::new();
    for link in array(audit, "cross_paper_connections")? {
        let target = text(link, "paper_id")?;
        let relation = text(link, "relation")?;
        if target == text(paper, "paper_id")?
            || !papers.contains_key(target)
            || !linked.insert((target, relation))
        {
            return Err(format!(
                "Invalid or duplicate cross-paper connection: {target}"
            ));
        }
        text(link, "rationale")?;
        text(link, "transfer_boundary")?;
    }
    let assignment = &paper["roadmap_assignment"];
    if !["candidate", "research-lens", "deferred"].contains(&text(assignment, "status")?) {
        return Err("Invalid roadmap disposition".into());
    }
    text(assignment, "rationale")?;
    let targets = strings(assignment, "work_item_ids")?;
    if targets.is_empty() || targets.iter().collect::<BTreeSet<_>>().len() != targets.len() {
        return Err("Missing or duplicate roadmap targets".into());
    }
    for target in &targets {
        if !opportunities.contains_key(target) {
            return Err(format!("Unknown roadmap item {target}"));
        }
    }
    if !targets.contains(&text(assignment, "primary_work_item")?) {
        return Err("Primary roadmap item is not among assigned items".into());
    }
    Ok(())
}

fn validate_paper(paper: &Value, source: &Value, rooms: &BTreeSet<String>) -> Result<()> {
    if !text(paper, "paper_id")?
        .strip_prefix('p')
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("Paper ID must be p followed by digits, safe for a sidecar filename".into());
    }
    for key in [
        "paper_id",
        "family_id",
        "family_title",
        "title",
        "source_path",
        "source_url",
        "reading_scope",
        "mathematical_hook",
        "playful_entry",
        "expert_use",
        "visual_sound",
        "implementation",
        "contribution_kind",
        "priority",
        "caveat",
        "next_question",
    ] {
        text(paper, key)?;
    }
    for key in ["family_id", "source_path", "source_url"] {
        if text(paper, key)? != text(source, key)? {
            return Err(format!("Source identity mismatch in {key}"));
        }
    }
    for room in strings(paper, "existing_room_ids")? {
        if !rooms.contains(room) {
            return Err(format!("Unknown room identifier {room}"));
        }
    }
    let concepts = strings(paper, "connections")?;
    if concepts.is_empty() || concepts.iter().collect::<BTreeSet<_>>().len() != concepts.len() {
        return Err("Concept tags missing or duplicated".into());
    }
    for (key, allowed) in [
        (
            "reading_scope",
            &[
                "abstract",
                "introduction-and-statement",
                "selected-construction",
            ][..],
        ),
        ("priority", &["prototype", "explore", "background"][..]),
        (
            "implementation",
            &["small", "medium", "large", "research"][..],
        ),
    ] {
        if !allowed.contains(&text(paper, key)?) {
            return Err(format!("Unsupported {key}"));
        }
    }
    for retrieved in array(source, "retrieved_sources")? {
        let hash = text(retrieved, "sha256")?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Invalid source checksum".into());
        }
    }
    Ok(())
}

#[derive(Default)]
struct Graph {
    nodes: BTreeMap<String, Value>,
    edges: BTreeMap<(String, String, String), Value>,
}

impl Graph {
    fn node(&mut self, node: Value) -> Result<()> {
        let id = text(&node, "id")?.to_owned();
        if self
            .nodes
            .get(&id)
            .is_some_and(|existing| existing != &node)
        {
            return Err(format!("Conflicting graph node {id}"));
        }
        self.nodes.insert(id, node);
        Ok(())
    }

    fn edge(
        &mut self,
        source: &str,
        target: &str,
        relation: &str,
        evidence: &str,
    ) -> Result<&mut Value> {
        let key = (source.into(), target.into(), relation.into());
        let edge =
            json!({"source": source, "target": target, "relation": relation, "evidence": evidence});
        match self.edges.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => Ok(entry.insert(edge)),
            std::collections::btree_map::Entry::Occupied(_) => {
                Err(format!("Duplicate edge {source} -> {target} ({relation})"))
            }
        }
    }

    fn finish(self, source_commit: &str) -> Result<Value> {
        for (source, target, _) in self.edges.keys() {
            if !self.nodes.contains_key(source) || !self.nodes.contains_key(target) {
                return Err(format!("Missing endpoint for {source} -> {target}"));
            }
        }
        Ok(json!({
            "schema": "numinous.research-graph.v1",
            "source_commit": source_commit,
            "semantics": "Room, concept, capability, cross-paper and roadmap edges are research interpretations and proposed reuse, not proof dependencies or shipped features. Cross-paper edges retain their rationale and transfer boundary.",
            "nodes": self.nodes.into_values().collect::<Vec<_>>(),
            "edges": self.edges.into_values().collect::<Vec<_>>(),
        }))
    }
}

fn graph_for(data: &Value) -> Result<Value> {
    let mut graph = Graph::default();
    for paper in array(data, "papers")? {
        let pid = text(paper, "paper_id")?;
        let family = format!("family:{}", text(paper, "family_id")?);
        graph.node(json!({"id": pid, "kind": "paper", "label": text(paper, "title")?, "priority": text(paper, "priority")?, "note_path": format!("papers/{pid}.md")}))?;
        graph.node(json!({"id": family, "kind": "family", "label": format!("{}: {}", text(paper, "family_id")?, text(paper, "family_title")?)}))?;
        graph.edge(pid, &family, "catalog-member", "pinned-source-catalog")?;
        for link in array(&paper["insight_audit"], "cross_paper_connections")? {
            let edge = graph.edge(
                pid,
                text(link, "paper_id")?,
                text(link, "relation")?,
                "design-inference",
            )?;
            edge["rationale"] = link["rationale"].clone();
            edge["transfer_boundary"] = link["transfer_boundary"].clone();
        }
        for target in strings(&paper["roadmap_assignment"], "work_item_ids")? {
            let edge = graph.edge(
                pid,
                &format!("work:{target}"),
                "roadmap-disposition",
                "design-inference",
            )?;
            edge["status"] = paper["roadmap_assignment"]["status"].clone();
            edge["rationale"] = paper["roadmap_assignment"]["rationale"].clone();
        }
        for (field, prefix, kind, relation, evidence) in [
            (
                "existing_room_ids",
                "room:",
                "room",
                "possible-room-connection",
                "design-inference",
            ),
            (
                "connections",
                "concept:",
                "concept",
                "conceptual-connection",
                "survey-interpretation",
            ),
        ] {
            for label in strings(paper, field)? {
                let id = format!("{prefix}{label}");
                graph.node(json!({"id": id, "kind": kind, "label": label}))?;
                graph.edge(pid, &id, relation, evidence)?;
            }
        }
    }
    for capability in array(data, "capabilities")? {
        let id = format!("capability:{}", text(capability, "id")?);
        graph.node(json!({"id": id, "kind": "capability", "label": text(capability, "label")?, "rationale": text(capability, "rationale")?}))?;
        for family in strings(capability, "families")? {
            graph.edge(
                &format!("family:{family}"),
                &id,
                "possible-shared-capability",
                "design-inference",
            )?;
        }
    }
    for item in array(data, "opportunities")? {
        let id = format!("work:{}", text(item, "id")?);
        graph.node(json!({"id": id, "kind": "work-item", "label": text(item, "title")?, "stage": text(item, "stage")?, "status": text(item, "status")?}))?;
        for dependency in strings(item, "dependencies")? {
            graph.edge(
                &id,
                &format!("work:{dependency}"),
                "proposed-prerequisite",
                "design-inference",
            )?;
        }
    }
    graph.finish(text(data, "source_commit")?)
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn render_paper_note(paper: &Value, revision: &str) -> Result<String> {
    let id = text(paper, "paper_id")?;
    let audit = &paper["insight_audit"];
    let assignment = &paper["roadmap_assignment"];
    let mut out = format!(
        "# {id}: {}\n\nResearch sidecar, rebuilt from [the canonical records](../papers.json).\nEdit that record and run the Rust atlas builder; this note and the graph share\nthe same evidence. Proposals are Designed; manuscript proofs are not verified here.\n\n[Open the graph at this paper](../index.html#{id}) | [Read the pinned manuscript]({})\n\nSource revision: `{revision}`. Family: {}. Reading scope: {}.\n\n{}\n\n",
        text(paper, "title")?,
        text(paper, "source_url")?,
        text(paper, "family_id")?,
        text(paper, "reading_scope")?,
        text(paper, "reading_note")?
    );
    for (heading, key) in [
        ("The mathematical question", "mathematical_hook"),
        ("Visual and sonic possibility", "visual_sound"),
        ("What makes it hard", "caveat"),
    ] {
        out.push_str(&format!("## {heading}\n\n{}\n\n", text(paper, key)?));
    }
    out.push_str("## Inside the construction\n\n");
    for insight in array(audit, "extracted_insights")? {
        out.push_str(&format!(
            "### {}\n\n{}\n\n- **Source locator:** {}\n- **Numinous use:** {}\n- **Limit:** {}\n\n",
            text(insight, "kind")?,
            text(insight, "insight")?,
            text(insight, "source_locator")?,
            text(insight, "numinous_application")?,
            text(insight, "limitation")?
        ));
    }
    for (heading, key) in [
        ("Reusable asset", "reusable_asset"),
        ("Play and optional game", "playful_loop"),
        ("Useful expert workflow", "expert_workflow"),
    ] {
        out.push_str(&format!("## {heading}\n\n{}\n\n", text(audit, key)?));
    }
    out.push_str("## Connections to investigate\n\n");
    let connections = array(audit, "cross_paper_connections")?;
    if connections.is_empty() {
        out.push_str("No specific paper-to-paper transfer is asserted yet.\n\n");
    }
    for link in connections {
        let target = text(link, "paper_id")?;
        out.push_str(&format!(
            "- **[{target}]({target}.md), {}:** {} Boundary: {}\n",
            text(link, "relation")?,
            text(link, "rationale")?,
            text(link, "transfer_boundary")?
        ));
    }
    out.push_str(&format!(
        "\nExisting room prerequisites or possible connections: {}. These are proposed\nconnections, not claims that a room implements this manuscript.\n\n",
        match strings(paper, "existing_room_ids")? {
            rooms if rooms.is_empty() => "none identified".to_owned(),
            rooms => rooms.join(", "),
        }
    ));
    out.push_str("## Open questions\n\n");
    for question in strings(audit, "open_questions")? {
        out.push_str(&format!("- {question}\n"));
    }
    out.push_str(&format!(
        "\n## Roadmap disposition\n\n**{}:** {}\n\n",
        text(assignment, "status")?,
        text(assignment, "rationale")?
    ));
    for target in strings(assignment, "work_item_ids")? {
        out.push_str(&format!("- [{target}](../coverage.md#{target})\n"));
    }
    out.push_str("\n## Source sections inspected\n\n");
    for section in array(audit, "source_sections")? {
        out.push_str(&format!(
            "- [{}](https://github.com/openai/math/blob/{revision}/{}): {}\n",
            text(section, "locator")?,
            text(section, "source_path")?,
            text(section, "read_for")?
        ));
    }
    Ok(out)
}

fn render_coverage(data: &Value) -> Result<String> {
    let papers = array(data, "papers")?;
    let mut result = String::from(
        "# Research opportunity coverage\n\nThis file is rebuilt from the atlas records. It accounts for the pinned corpus;\nit does not claim that every proof is verified or every useful insight found.\nSelection and acceptance rules live in [Mathematical play](../../MATHEMATICAL_PLAY.md).\nAll work items below are Designed. The active release sequence remains in\n[the roadmap](../../ROADMAP.md).\n\n",
    );
    result.push_str(&format!(
        "Source revision: `{}`. Indexed manuscripts: {}.\n\n",
        text(data, "source_commit")?,
        papers.len()
    ));
    result.push_str("## Reading coverage\n\n| Scope | Manuscripts |\n| --- | --- |\n");
    let mut scopes = BTreeMap::new();
    for paper in papers {
        *scopes
            .entry(text(paper, "reading_scope")?)
            .or_insert(0usize) += 1;
    }
    for (scope, count) in scopes {
        result.push_str(&format!("| {scope} | {count} |\n"));
    }
    result.push_str("\n## Work items\n\nCounts overlap when a paper offers more than one possible destination. A link\nis an opportunity assignment, not a commitment to implement the entire paper.\n\n");
    for item in array(data, "opportunities")? {
        let id = text(item, "id")?;
        let assigned: Vec<_> = papers
            .iter()
            .filter(|p| {
                p["roadmap_assignment"]["work_item_ids"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|v| v.as_str() == Some(id)))
            })
            .collect();
        result.push_str(&format!(
            "### {id}\n\n**{}.** Stage: {}. Assigned manuscripts: {}.\n\n",
            text(item, "title")?,
            text(item, "stage")?,
            assigned.len()
        ));
        for (label, key) in [
            ("First gesture", "gesture"),
            ("Game", "game"),
            ("Expert workflow", "expert_workflow"),
            ("Core seam", "core_seam"),
            ("Validation", "validation"),
            ("Scope", "scope"),
            ("Revisit or reject", "revisit_or_reject"),
        ] {
            result.push_str(&format!("- **{label}:** {}\n", text(item, key)?));
        }
        let deps = strings(item, "dependencies")?;
        result.push_str(&format!(
            "- **Depends on:** {}.\n\n",
            if deps.is_empty() {
                "No proposed work-item prerequisite".into()
            } else {
                deps.iter()
                    .map(|id| format!("[{id}](#{id})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
    }
    result.push_str("## Every manuscript\n\nPaper links open the Markdown research sidecars. Each note links back to its\ngraph neighborhood and contains source sections, extracted constructions,\nlimitations and open questions.\n\n| Paper | Disposition | Primary destination | Other destinations | Reason |\n| --- | --- | --- | --- | --- |\n");
    for paper in papers {
        let id = text(paper, "paper_id")?;
        let assignment = &paper["roadmap_assignment"];
        let primary = text(assignment, "primary_work_item")?;
        let other = strings(assignment, "work_item_ids")?
            .into_iter()
            .filter(|id| *id != primary)
            .map(|id| format!("[{id}](#{id})"))
            .collect::<Vec<_>>()
            .join(", ");
        result.push_str(&format!(
            "| [{id}: {}](papers/{id}.md) | {} | [{primary}](#{primary}) | {other} | {} |\n",
            markdown_cell(text(paper, "title")?),
            text(assignment, "status")?,
            markdown_cell(text(assignment, "rationale")?)
        ));
    }
    Ok(result)
}

fn render_html(template: &str, data: &Value, graph: &Value) -> Result<String> {
    if template.matches("__ATLAS_DATA__").count() != 1 {
        return Err("Viewer template must contain one data marker".into());
    }
    let payload = serde_json::to_string(&json!({"atlas": data, "graph": graph}))
        .map_err(|error| format!("Cannot serialize viewer data: {error}"))?;
    // Source text must not terminate the inert JSON script element.
    let payload = payload
        .replace('<', "\\u003c")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(template
        .replace("\r\n", "\n")
        .replace("__ATLAS_DATA__", &payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> Result<(Value, Value)> {
        let atlas = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/math-atlas");
        let mut data = load(&atlas.join("papers.json"))?;
        data["opportunities"] = load(&atlas.join("opportunities.json"))?["items"].clone();
        Ok((data, load(&atlas.join("sources.json"))?))
    }

    #[test]
    fn audit_rejects_missing_provenance_and_dangling_references() -> Result<()> {
        let (data, sources) = inputs()?;
        validate(&data, &sources)?;
        let mut broken = data.clone();
        broken["papers"][0]["insight_audit"]["source_sections"][0]["source_path"] =
            json!("unretrieved.tex");
        assert!(
            validate(&broken, &sources)
                .unwrap_err()
                .contains("lacks provenance")
        );
        let mut broken = data.clone();
        broken["papers"][0]["roadmap_assignment"]["work_item_ids"] = json!(["unknown-work"]);
        assert!(
            validate(&broken, &sources)
                .unwrap_err()
                .contains("Unknown roadmap item")
        );
        let mut broken = data;
        broken["papers"][0]["insight_audit"]["cross_paper_connections"] = json!([{
            "paper_id":"p9999", "relation":"proposed-transfer", "rationale":"test", "transfer_boundary":"test"
        }]);
        assert!(
            validate(&broken, &sources)
                .unwrap_err()
                .contains("cross-paper connection")
        );
        Ok(())
    }

    #[test]
    fn opportunity_dependencies_reject_cycles() -> Result<()> {
        let (mut data, _) = inputs()?;
        let first = data["opportunities"][0]["id"].clone();
        let second = data["opportunities"][1]["id"].clone();
        data["opportunities"][0]["dependencies"] = json!([second]);
        data["opportunities"][1]["dependencies"] = json!([first]);
        let index = unique_index(array(&data, "opportunities")?, "id")?;
        assert!(
            validate_opportunities(&index)
                .unwrap_err()
                .contains("cycle")
        );
        Ok(())
    }

    #[test]
    fn removed_paper_cannot_leave_an_unchecked_sidecar() {
        let expected = BTreeSet::from(["p001.md".to_owned()]);
        let actual = BTreeSet::from(["p001.md".to_owned(), "p002.md".to_owned()]);
        assert!(
            validate_note_names(&actual, &expected)
                .unwrap_err()
                .contains("p002.md")
        );
        assert!(validate_note_names(&expected, &expected).is_ok());
    }

    #[test]
    fn repository_atlas_covers_its_sources_and_rebuilds_without_drift() -> Result<()> {
        run(Args { check: true })
    }

    #[test]
    fn viewer_data_cannot_close_its_script_element() -> Result<()> {
        let data = json!({"text": "</script>\u{2028}\u{2029}"});
        let html = render_html("__ATLAS_DATA__", &data, &Value::Null)?;
        assert!(!html.contains('<'));
        assert!(!html.contains('\u{2028}'));
        assert!(!html.contains('\u{2029}'));
        let decoded: Value = serde_json::from_str(&html).map_err(|error| error.to_string())?;
        assert_eq!(decoded["atlas"], data);
        assert!(render_html("__ATLAS_DATA____ATLAS_DATA__", &data, &Value::Null).is_err());
        Ok(())
    }

    #[test]
    fn graph_rejects_duplicate_edges_and_missing_endpoints() -> Result<()> {
        let mut graph = Graph::default();
        graph.edge("a", "b", "relation", "evidence")?;
        assert!(graph.edge("a", "b", "relation", "evidence").is_err());
        assert!(graph.finish("revision").is_err());
        Ok(())
    }

    #[test]
    fn graph_order_is_independent_of_insertion_order() -> Result<()> {
        let build = |order: [&str; 2]| -> Result<Value> {
            let mut graph = Graph::default();
            for id in order {
                graph.node(json!({"id": id, "kind": "paper", "label": id}))?;
                graph.edge(id, id, "relation", "evidence")?;
            }
            graph.finish("revision")
        };
        assert_eq!(build(["a", "b"])?, build(["b", "a"])?);
        Ok(())
    }
}
