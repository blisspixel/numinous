//! Resume one explicit project chain from the terminal.
//!
//! The preview comes from the core. Applying it writes this process's
//! workspace intention and, when the next call names a present room, its
//! place. Nothing is saved, and the next tool is not called.

use std::path::Path;

use numinous_core::{
    CreationStatus, EvidenceStatus, NextPreview, ProjectArgumentValue, ProjectCall, ReceiptCheck,
    ResumePreview, RoomStatus, SessionWorkspace, WorkspacePlace, WorkspacePlaceDraft,
    WorkspaceUpdate, display_safe, try_load_journal_file, try_load_project_file,
};

/// Text for one resume, plus a refusal when `--apply` could not write.
#[derive(Debug)]
pub(super) struct ResumeOutcome {
    pub text: String,
    /// Set when the player asked to apply and this process wrote nothing.
    pub refusal: Option<String>,
}

/// Preview one revision. `apply` writes this process only.
///
/// # Errors
///
/// Returns an error when the chain or journal cannot be read, or when the
/// chosen revision cannot be previewed. A refusal to apply is [`ResumeOutcome::refusal`],
/// with the preview still in [`ResumeOutcome::text`].
pub(super) fn resume_report(
    project_path: &Path,
    journal_path: &Path,
    revision: Option<u64>,
    apply: bool,
) -> Result<ResumeOutcome, String> {
    let project_before = std::fs::read(project_path).ok();
    let journal_before = std::fs::read(journal_path).ok();
    let chain = try_load_project_file(project_path).map_err(|error| {
        format!(
            "Could not read the project chain at {}: {}",
            display_safe(&project_path.to_string_lossy()),
            display_safe(&error.to_string())
        )
    })?;
    let journal = try_load_journal_file(journal_path).map_err(|error| {
        format!(
            "Could not read the journal at {}: {}",
            display_safe(&journal_path.to_string_lossy()),
            display_safe(&error.to_string())
        )
    })?;
    let preview = chain
        .preview(revision, &journal, ReceiptCheck::NotSupplied)
        .map_err(|error| {
            format!(
                "Could not preview the project: {}",
                display_safe(&error.to_string())
            )
        })?;
    let process = if apply {
        match apply_workspace(&preview) {
            Ok(workspace) => ProcessWorkspace::Written(Box::new(workspace)),
            Err(refusal) => ProcessWorkspace::Refused(refusal),
        }
    } else {
        ProcessWorkspace::PreviewOnly
    };
    if std::fs::read(project_path).ok() != project_before {
        return Err("The project file changed during resume.".to_string());
    }
    if std::fs::read(journal_path).ok() != journal_before {
        return Err("The journal file changed during resume.".to_string());
    }
    Ok(render(&preview, process))
}

enum ProcessWorkspace {
    PreviewOnly,
    Refused(String),
    Written(Box<SessionWorkspace>),
}

fn apply_workspace(preview: &ResumePreview) -> Result<SessionWorkspace, String> {
    if !preview.will_return {
        return Err("Apply refused: this revision will not return.".to_string());
    }
    let NextPreview::Ready(call) = &preview.next else {
        return Err("Apply refused: the next call is not ready.".to_string());
    };
    let place = match call.tool {
        "play_room" => Some(place_from_call(preview, call, "id", true)?),
        "study_room" => Some(place_from_call(preview, call, "room", false)?),
        "open_creation" | "fork_creation" => None,
        other => {
            return Err(format!(
                "Apply refused: {other} is outside the process workspace rules."
            ));
        }
    };
    let mut workspace = SessionWorkspace::new();
    workspace
        .edit(WorkspaceUpdate {
            place,
            intention: Some(preview.question.clone()),
            ..WorkspaceUpdate::default()
        })
        .map_err(|error| format!("Apply refused: {error}"))?;
    Ok(workspace)
}

fn place_from_call(
    preview: &ResumePreview,
    call: &ProjectCall,
    room_argument: &str,
    with_phase: bool,
) -> Result<WorkspacePlaceDraft, String> {
    let room = text_argument(call, room_argument)?;
    let present = preview
        .rooms
        .iter()
        .any(|fact| fact.id == room && fact.status == RoomStatus::Present);
    if !present {
        return Err("Apply refused: the next room is not a present catalog room.".to_string());
    }
    let t = if with_phase {
        phase_argument(call)?
    } else if call.arguments.iter().any(|argument| argument.name == "t") {
        return Err("Apply refused: study_room has no phase.".to_string());
    } else {
        None
    };
    Ok(WorkspacePlaceDraft {
        room,
        t,
        variation: None,
    })
}

fn text_argument(call: &ProjectCall, name: &str) -> Result<String, String> {
    let Some(argument) = call.arguments.iter().find(|argument| argument.name == name) else {
        return Err(format!("Apply refused: {} is missing {name}.", call.tool));
    };
    match &argument.value {
        ProjectArgumentValue::Text(text) => Ok(text.clone()),
        ProjectArgumentValue::Number(_) => Err(format!(
            "Apply refused: {name} on {} must be text.",
            call.tool
        )),
    }
}

fn phase_argument(call: &ProjectCall) -> Result<Option<f64>, String> {
    let Some(argument) = call.arguments.iter().find(|argument| argument.name == "t") else {
        return Ok(None);
    };
    let ProjectArgumentValue::Number(token) = &argument.value else {
        return Err("Apply refused: play_room phase must be a number.".to_string());
    };
    let phase = token
        .parse::<f64>()
        .map_err(|_| "Apply refused: play_room phase is not a number.".to_string())?;
    if !phase.is_finite() || !(0.0..1.0).contains(&phase) {
        return Err("Apply refused: play_room phase must be in [0, 1).".to_string());
    }
    Ok(Some(phase))
}

fn render(preview: &ResumePreview, process: ProcessWorkspace) -> ResumeOutcome {
    let mut text = String::new();
    text.push_str(&format!(
        "Schema: {}\nSchema version: {}\nRevision: {}\nQuestion: {}\n",
        preview.schema,
        preview.version,
        preview.revision_id,
        display_safe(&preview.question)
    ));
    text.push_str(&format!(
        "Interpreted: {}\nWill return: {}\nPreview not applied: {}\nPreview workspace changed: {}\nPreview journal changed: {}\n",
        word(preview.interpreted),
        word(preview.will_return),
        word(preview.not_applied),
        word(preview.workspace_changed),
        word(preview.journal_changed)
    ));
    text.push_str(&next_section(&preview.next));
    text.push_str(&rooms_section(preview));
    text.push_str(&evidence_section(preview));
    text.push_str(&creation_section(preview));
    text.push_str(&format!(
        "Superseded by: {}\nParent resolved: {}\n",
        match preview.superseded_by {
            Some(id) => id.to_string(),
            None => "none".to_string(),
        },
        match preview.parent_resolved {
            Some(resolved) => word(resolved).to_string(),
            None => "absent".to_string(),
        }
    ));
    let refusal = match process {
        ProcessWorkspace::PreviewOnly => {
            text.push_str("Process workspace: preview only\n");
            None
        }
        ProcessWorkspace::Refused(refusal) => {
            text.push_str("Process workspace: left empty\n");
            text.push_str(&refusal);
            text.push('\n');
            Some(refusal)
        }
        ProcessWorkspace::Written(workspace) => {
            text.push_str("Process workspace: this process only\n");
            text.push_str(&workspace_section(&workspace, next_tool(&preview.next)));
            None
        }
    };
    ResumeOutcome { text, refusal }
}

fn next_tool(next: &NextPreview) -> &str {
    match next {
        NextPreview::Ready(call) => call.tool,
        NextPreview::Incompatible(next) => next.tool.as_str(),
    }
}

fn next_section(next: &NextPreview) -> String {
    match next {
        NextPreview::Ready(call) => {
            let mut text = format!("Next: {}\n", call.tool);
            for argument in &call.arguments {
                let value = match &argument.value {
                    ProjectArgumentValue::Text(value) | ProjectArgumentValue::Number(value) => {
                        display_safe(value)
                    }
                };
                text.push_str(&format!("  {}: {value}\n", argument.name));
            }
            text
        }
        NextPreview::Incompatible(next) => format!(
            "Next: incompatible\n  tool: {}\n  reason: {}\n",
            display_safe(&next.tool),
            display_safe(&next.reason)
        ),
    }
}

fn rooms_section(preview: &ResumePreview) -> String {
    if preview.rooms.is_empty() {
        return "Rooms: none\n".to_string();
    }
    let mut text = String::from("Rooms:\n");
    for room in &preview.rooms {
        text.push_str(&format!(
            "  {} {}\n",
            display_safe(&room.id),
            room_word(room.status)
        ));
    }
    text
}

fn evidence_section(preview: &ResumePreview) -> String {
    if preview.evidence.is_empty() {
        return "Evidence: none\n".to_string();
    }
    let mut text = String::from("Evidence:\n");
    for fact in &preview.evidence {
        text.push_str(&format!(
            "  {} {} {}\n",
            fact.kind,
            evidence_word(fact.status),
            fact.digest_hex
        ));
    }
    text
}

fn creation_section(preview: &ResumePreview) -> String {
    let status = match preview.creation.status {
        CreationStatus::Missing => "missing",
        CreationStatus::Present => "present",
        CreationStatus::Incompatible => "incompatible",
    };
    let mut text = format!("Creation: {status}");
    if let Some(period) = &preview.creation.period_text {
        text.push_str(&format!(" period {}", display_safe(period)));
    }
    text.push_str(&format!(
        "\nLineage was not in the link: {}\n",
        word(preview.creation.lineage_was_not_in_the_link)
    ));
    text
}

fn workspace_section(workspace: &SessionWorkspace, tool: &str) -> String {
    let intention = workspace
        .intention()
        .map(display_safe)
        .unwrap_or_else(|| "none".to_string());
    let pending = workspace
        .pending_prediction()
        .map(display_safe)
        .unwrap_or_else(|| "none".to_string());
    let place = match workspace.place() {
        Some(place) => place_line(place),
        None => format!("Place: none\n{tool} names no room, so this process has no place."),
    };
    let unfinished = match workspace.unfinished() {
        Some(_) => "set",
        None => "none",
    };
    let recent = if workspace.recent().is_empty() {
        "none".to_string()
    } else {
        workspace.recent().len().to_string()
    };
    let retrieved = if workspace.retrieved().is_empty() {
        "none".to_string()
    } else {
        workspace.retrieved().len().to_string()
    };
    format!(
        "Intention: {intention}\n{place}\nPending prediction: {pending}\nUnfinished: {unfinished}\nRecent notes: {recent}\nRetrieved handles: {retrieved}\n"
    )
}

fn place_line(place: &WorkspacePlace) -> String {
    let mut line = format!("Place: {}", display_safe(place.room()));
    if let Some(phase) = place.t() {
        line.push_str(&format!(" t={}", phase_text(phase)));
    }
    if let Some(variation) = place.variation() {
        line.push_str(&format!(" variation={variation}"));
    }
    line
}

fn phase_text(phase: f64) -> String {
    let text = format!("{phase}");
    if text.parse::<f64>() == Ok(phase) {
        text
    } else {
        format!("{phase:?}")
    }
}

fn word(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn room_word(status: RoomStatus) -> &'static str {
    match status {
        RoomStatus::Present => "present",
        RoomStatus::Missing => "missing",
        RoomStatus::Incompatible => "incompatible",
    }
}

fn evidence_word(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Present => "present",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Corrected => "corrected",
        EvidenceStatus::Collided => "collided",
        EvidenceStatus::Incompatible => "incompatible",
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use numinous_core::{
        JOURNAL_SOURCE_SELF_AUTHORED, JournalRecord, ProjectDraft, ProjectEvidence, ProjectNext,
        keep_project_file, record_journal_file,
    };

    use super::resume_report;

    fn root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "numinous_cli_project_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("fixture directory");
        path
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut found = std::fs::read_dir(dir)
            .expect("fixture directory")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        found.sort();
        found
    }

    fn draft(
        question: &str,
        next: ProjectNext,
        rooms: &[&str],
        creation: Option<&str>,
        evidence: Vec<ProjectEvidence>,
    ) -> ProjectDraft {
        ProjectDraft {
            recorded_at_utc: 1_700_000_000,
            question: question.to_string(),
            next,
            rooms: rooms.iter().map(|room| (*room).to_string()).collect(),
            evidence,
            creation: creation.map(str::to_string),
        }
    }

    fn require(text: &str, needle: &str) {
        assert!(text.contains(needle), "missing {needle}\n{text}");
    }

    #[test]
    fn project_resume_apply_sets_a_play_room_and_leaves_both_files() {
        let root = root("play");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        let question = r#"{"tool":"play_room","arguments":{"id":"lorenz"}}"#;
        keep_project_file(
            &project,
            &draft(
                question,
                ProjectNext::PlayRoom {
                    room: "lissajous".to_string(),
                    phase: Some("0.25".to_string()),
                },
                &["lissajous"],
                None,
                Vec::new(),
            ),
        )
        .expect("keep");
        let project_bytes = std::fs::read(&project).expect("project bytes");
        let before = names(&root);
        let outcome = resume_report(&project, &journal, None, true).expect("resume");
        assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
        require(&outcome.text, "Schema: numinous.project-resume-preview");
        require(&outcome.text, "Schema version: 1");
        require(&outcome.text, "Interpreted: false");
        require(&outcome.text, "Will return: true");
        require(&outcome.text, "Preview not applied: true");
        require(&outcome.text, "Preview workspace changed: false");
        require(&outcome.text, "Preview journal changed: false");
        require(&outcome.text, &format!("Question: {question}"));
        require(
            &outcome.text,
            "Next: play_room\n  id: lissajous\n  t: 0.25\n",
        );
        require(&outcome.text, "Process workspace: this process only");
        require(&outcome.text, &format!("Intention: {question}"));
        require(&outcome.text, "Place: lissajous t=0.25");
        require(&outcome.text, "Pending prediction: none");
        require(&outcome.text, "Unfinished: none");
        require(&outcome.text, "Recent notes: none");
        require(&outcome.text, "Retrieved handles: none");
        require(&outcome.text, "Evidence: none");
        assert!(
            !outcome.text.contains("id: lorenz"),
            "the question must not be dispatched\n{}",
            outcome.text
        );
        assert!(
            !outcome
                .text
                .contains(root.file_name().unwrap().to_str().unwrap())
        );
        assert_eq!(
            std::fs::read(&project).expect("project after"),
            project_bytes
        );
        assert!(!journal.exists(), "resume must not create a journal");
        assert_eq!(names(&root), before);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_without_apply_prints_the_preview_only() {
        let root = root("preview");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        keep_project_file(
            &project,
            &draft(
                "Which room holds the curve?",
                ProjectNext::StudyRoom {
                    room: "lissajous".to_string(),
                },
                &["lissajous"],
                None,
                Vec::new(),
            ),
        )
        .expect("keep");
        let outcome = resume_report(&project, &journal, Some(1), false).expect("resume");
        assert!(outcome.refusal.is_none());
        require(&outcome.text, "Next: study_room\n  room: lissajous\n");
        require(&outcome.text, "Process workspace: preview only");
        assert!(!outcome.text.contains("Intention:"));
        assert!(!outcome.text.contains("Place:"));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_apply_sets_a_study_room_without_a_phase() {
        let root = root("study");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        keep_project_file(
            &project,
            &draft(
                "Read the curve.",
                ProjectNext::StudyRoom {
                    room: "lissajous".to_string(),
                },
                &["lissajous"],
                None,
                Vec::new(),
            ),
        )
        .expect("keep");
        let outcome = resume_report(&project, &journal, None, true).expect("resume");
        assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
        require(&outcome.text, "Place: lissajous\n");
        assert!(!outcome.text.contains("t="));
        require(&outcome.text, "Intention: Read the curve.");
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_apply_of_open_creation_sets_intention_and_no_place() {
        let root = root("open");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        keep_project_file(
            &project,
            &draft(
                "What period do these two oscillators share?",
                ProjectNext::OpenCreation {
                    capsule: "closing-voices".to_string(),
                },
                &["lissajous"],
                Some("full-return"),
                Vec::new(),
            ),
        )
        .expect("keep");
        let outcome = resume_report(&project, &journal, None, true).expect("resume");
        assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
        require(
            &outcome.text,
            "Next: open_creation\n  capsule: closing-voices\n",
        );
        require(
            &outcome.text,
            "Intention: What period do these two oscillators share?",
        );
        require(&outcome.text, "Place: none");
        require(
            &outcome.text,
            "open_creation names no room, so this process has no place.",
        );
        require(&outcome.text, "Creation: present period 12");
        assert!(!outcome.text.contains("Place: lissajous"));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_apply_of_fork_creation_sets_intention_and_no_place() {
        let root = root("fork");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        keep_project_file(
            &project,
            &draft(
                "Remix the return.",
                ProjectNext::ForkCreation,
                &["lissajous"],
                Some("full-return"),
                Vec::new(),
            ),
        )
        .expect("keep");
        let outcome = resume_report(&project, &journal, None, true).expect("resume");
        assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
        require(&outcome.text, "Next: fork_creation\n  parent: ");
        require(&outcome.text, "Intention: Remix the return.");
        require(&outcome.text, "Place: none");
        require(
            &outcome.text,
            "fork_creation names no room, so this process has no place.",
        );
        assert!(!outcome.text.contains("Place: lissajous"));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_apply_of_a_superseded_revision_is_refused() {
        let root = root("superseded");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        keep_project_file(
            &project,
            &draft(
                "First question.",
                ProjectNext::PlayRoom {
                    room: "lissajous".to_string(),
                    phase: None,
                },
                &["lissajous"],
                None,
                Vec::new(),
            ),
        )
        .expect("keep");
        numinous_core::correct_project_file(
            &project,
            1,
            &draft(
                "Second question.",
                ProjectNext::PlayRoom {
                    room: "lissajous".to_string(),
                    phase: None,
                },
                &["lissajous"],
                None,
                Vec::new(),
            ),
        )
        .expect("correct");
        let project_bytes = std::fs::read(&project).expect("project bytes");
        let refused = resume_report(&project, &journal, Some(1), true).expect("old revision");
        require(&refused.text, "Revision: 1");
        require(&refused.text, "Question: First question.");
        require(&refused.text, "Will return: false");
        require(&refused.text, "Superseded by: 2");
        require(&refused.text, "Process workspace: left empty");
        require(
            &refused.text,
            "Apply refused: this revision will not return.",
        );
        assert_eq!(
            refused.refusal.as_deref(),
            Some("Apply refused: this revision will not return.")
        );
        assert!(!refused.text.contains("Intention:"));
        let current = resume_report(&project, &journal, None, true).expect("current");
        assert!(current.refusal.is_none(), "{:?}", current.refusal);
        require(&current.text, "Revision: 2");
        require(&current.text, "Intention: Second question.");
        require(&current.text, "Place: lissajous\n");
        assert_eq!(
            std::fs::read(&project).expect("project after"),
            project_bytes
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_of_an_empty_or_malformed_chain_writes_nothing() {
        let root = root("empty");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        let error = resume_report(&project, &journal, None, true).expect_err("empty");
        require(&error, "project chain is empty");
        assert!(!project.exists());
        assert!(!journal.exists());
        std::fs::write(&project, b"not a chain\n").expect("malformed");
        let before = std::fs::read(&project).expect("bytes");
        let error = resume_report(&project, &journal, None, false).expect_err("malformed");
        require(&error, "Could not read the project chain");
        assert_eq!(std::fs::read(&project).expect("unchanged"), before);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn project_resume_apply_does_not_copy_journal_text() {
        let root = root("evidence");
        let project = root.join("project.txt");
        let journal = root.join("journal.txt");
        let entry = record_journal_file(
            &journal,
            JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 10,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lissajous",
                text: "quartz-journal-token",
                affect: None,
            },
        )
        .expect("journal");
        keep_project_file(
            &project,
            &draft(
                "What did the room do?",
                ProjectNext::PlayRoom {
                    room: "lissajous".to_string(),
                    phase: None,
                },
                &["lissajous"],
                None,
                vec![ProjectEvidence::Journal {
                    digest: entry.identity_digest(),
                    entry_id: Some(entry.entry_id),
                }],
            ),
        )
        .expect("keep");
        let digest: String = entry
            .identity_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let project_bytes = std::fs::read(&project).expect("project bytes");
        let journal_bytes = std::fs::read(&journal).expect("journal bytes");
        let outcome = resume_report(&project, &journal, None, true).expect("resume");
        assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
        require(
            &outcome.text,
            &format!("Evidence:\n  journal present {digest}\n"),
        );
        require(&outcome.text, "Retrieved handles: none");
        assert!(!outcome.text.contains("quartz-journal-token"));
        assert!(
            !project_bytes
                .windows("quartz-journal-token".len())
                .any(|window| { window == b"quartz-journal-token" })
        );
        assert_eq!(
            std::fs::read(&project).expect("project after"),
            project_bytes
        );
        assert_eq!(
            std::fs::read(&journal).expect("journal after"),
            journal_bytes
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
