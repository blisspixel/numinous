//! Shared JSON projection of the canonical project resume preview.

use numinous_core::CreationKind;
use numinous_core::{
    CreationFact, CreationStatus, EvidenceFact, EvidenceStatus, NextPreview, ProjectArgument,
    ProjectArgumentValue, ResumePreview, RoomStatus,
};
use serde_json::{Map, Value, json};

pub fn preview_json(preview: &ResumePreview) -> Result<Value, String> {
    let (status, tool, arguments, reason) = match &preview.next {
        NextPreview::Ready(call) => ("ready", call.tool, arguments_json(&call.arguments)?, None),
        NextPreview::Incompatible(incompatible) => (
            "incompatible",
            incompatible.tool.as_str(),
            json!({}),
            Some(incompatible.reason.as_str()),
        ),
    };
    Ok(json!({
        "schema": preview.schema,
        "schemaVersion": preview.version,
        "revisionId": preview.revision_id,
        "question": preview.question,
        "interpreted": preview.interpreted,
        "next": {
            "status": status,
            "tool": tool,
            "arguments": arguments,
            "reason": reason,
        },
        "rooms": preview.rooms.iter().map(|room| json!({
            "id": room.id,
            "status": room_status_name(room.status),
        })).collect::<Vec<_>>(),
        "evidence": preview.evidence.iter().map(evidence_json).collect::<Vec<_>>(),
        "creation": creation_json(&preview.creation),
        "willReturn": preview.will_return,
        "notApplied": preview.not_applied,
        "workspaceChanged": preview.workspace_changed,
        "journalChanged": preview.journal_changed,
        "supersededBy": preview.superseded_by,
        "parentResolved": preview.parent_resolved,
    }))
}

fn arguments_json(arguments: &[ProjectArgument]) -> Result<Value, String> {
    let mut object = Map::new();
    for argument in arguments {
        object.insert(argument.name.to_string(), argument_value(&argument.value)?);
    }
    Ok(Value::Object(object))
}

fn argument_value(value: &ProjectArgumentValue) -> Result<Value, String> {
    match value {
        ProjectArgumentValue::Text(text) => Ok(Value::String(text.clone())),
        ProjectArgumentValue::Number(token) => {
            let number: f64 = token
                .parse()
                .map_err(|_| "The previewed phase is not a JSON number.".to_string())?;
            let number = serde_json::Number::from_f64(number)
                .ok_or_else(|| "The previewed phase is not a JSON number.".to_string())?;
            Ok(Value::Number(number))
        }
    }
}

fn evidence_json(fact: &EvidenceFact) -> Value {
    json!({
        "kind": fact.kind,
        "digestHex": fact.digest_hex,
        "status": evidence_status_name(fact.status),
        "entryId": fact.entry_id,
        "supersededBy": fact.superseded_by,
        "subject": fact.subject,
        "text": fact.text,
        "tool": fact.tool,
        "sameBytesElsewhere": fact.same_bytes_elsewhere,
    })
}

fn creation_json(creation: &CreationFact) -> Value {
    json!({
        "kind":creation.kind.map(|kind|match kind {CreationKind::Studio=>"studio",CreationKind::Route=>"route"}),
        "capsule":creation.capsule,
        "status": creation_status_name(creation.status),
        "descends": creation.descends,
        "lineageWasNotInTheLink": creation.lineage_was_not_in_the_link,
        "periodText": creation.period_text,
    })
}

fn room_status_name(status: RoomStatus) -> &'static str {
    match status {
        RoomStatus::Present => "present",
        RoomStatus::Missing => "missing",
        RoomStatus::Incompatible => "incompatible",
    }
}

pub fn evidence_status_name(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Present => "present",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Corrected => "corrected",
        EvidenceStatus::Collided => "collided",
        EvidenceStatus::Incompatible => "incompatible",
    }
}

fn creation_status_name(status: CreationStatus) -> &'static str {
    match status {
        CreationStatus::Missing => "missing",
        CreationStatus::Present => "present",
        CreationStatus::Incompatible => "incompatible",
    }
}

#[cfg(test)]
mod tests {
    use super::preview_json;
    use numinous_core::{
        JOURNAL_SOURCE_SELF_AUTHORED, Journal, JournalRecord, ProjectChain, ProjectDraft,
        ProjectEvidence, ProjectNext, ReceiptCheck,
    };

    fn draft() -> ProjectDraft {
        ProjectDraft {
            recorded_at_utc: 20,
            question: "What survives reopening?".into(),
            next: ProjectNext::PlayRoom {
                room: "lissajous".into(),
                phase: Some("0.250".into()),
            },
            rooms: vec!["lissajous".into()],
            evidence: Vec::new(),
            creation: None,
        }
    }

    #[test]
    fn legacy_preview_preserves_numeric_phase_and_marks_unavailable_stored_data() {
        let mut draft = draft();
        let creation = numinous_core::studio_experiment("full-return").unwrap();
        draft.creation = Some(creation.to_num_file());
        let mut chain = ProjectChain::new();
        chain.keep(&draft).unwrap();
        let preview = chain
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        let json = preview_json(&preview).unwrap();
        assert_eq!(json["next"]["arguments"]["t"], 0.25);
        assert_eq!(json["creation"]["kind"], "studio");
        assert_eq!(json["creation"]["capsule"], creation.to_num_file());
        assert_eq!(json["creation"]["periodText"], "12");
        assert!(json["next"]["reason"].is_null());

        // Stored data can outlive its catalog or supported creation version.
        for (room, status) in [("future-room", "missing"), ("kepler-areas", "incompatible")] {
            let text = chain
                .to_text()
                .replace("lissajous", room)
                .replace("NUMINOUS_STUDIO 3", "NUMINOUS_STUDIO 99");
            let stored = ProjectChain::parse(&text).unwrap();
            let preview = stored
                .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
                .unwrap();
            let json = preview_json(&preview).unwrap();
            assert_eq!(json["rooms"][0]["status"], status);
            assert_eq!(json["next"]["status"], "incompatible");
            assert_eq!(json["next"]["tool"], "play_room");
            assert_eq!(json["next"]["arguments"], serde_json::json!({}));
            assert_eq!(json["next"]["reason"], "room is not available");
            assert_eq!(json["creation"]["status"], "incompatible");
            assert!(json["creation"]["capsule"].is_null());
            assert!(json["creation"]["kind"].is_null());
            assert_eq!(json["willReturn"], false);
            assert_eq!(stored.to_text(), text);
        }
    }

    #[test]
    fn evidence_projection_preserves_cited_correction_and_withholds_collided_or_disagreed_text() {
        let mut journal = Journal::new();
        let record = |text| JournalRecord {
            recorded_at_utc: 1,
            event_at_utc: 1,
            source: JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject: "lissajous",
            text,
            affect: None,
        };
        let cited = journal.record(record("original account")).unwrap();
        let other = journal.record(record("unrelated private account")).unwrap();
        let digest = journal.entry(cited).unwrap().identity_digest();
        let correction = journal
            .correct(
                2,
                None,
                JOURNAL_SOURCE_SELF_AUTHORED,
                cited,
                "corrected account",
                None,
            )
            .unwrap();
        let receipt_subject = format!("receipt:{}", "cd".repeat(32));
        journal
            .record(JournalRecord {
                subject: &receipt_subject,
                text: "private receipt account",
                ..record("unused")
            })
            .unwrap();
        let mut draft = draft();
        draft.evidence = vec![
            ProjectEvidence::Journal {
                digest,
                entry_id: Some(cited),
            },
            ProjectEvidence::Journal {
                digest,
                entry_id: Some(other),
            },
            ProjectEvidence::Journal {
                digest: [0xab; 32],
                entry_id: None,
            },
            ProjectEvidence::Receipt {
                digest: [0xcd; 32],
                tool: None,
            },
        ];
        let mut chain = ProjectChain::new();
        chain.keep(&draft).unwrap();
        let before = journal.to_text();
        let supplied = chain
            .preview(None, &journal, ReceiptCheck::NotSupplied)
            .unwrap();
        let supplied = preview_json(&supplied).unwrap();
        assert_eq!(supplied["evidence"][3]["status"], "present");
        assert_eq!(supplied["evidence"][3]["text"], "private receipt account");
        let preview = chain
            .preview(None, &journal, ReceiptCheck::Disagreed)
            .unwrap();
        let json = preview_json(&preview).unwrap();
        assert_eq!(json["evidence"][0]["status"], "corrected");
        assert_eq!(json["evidence"][0]["text"], "original account");
        assert_eq!(json["evidence"][0]["supersededBy"], correction);
        assert_eq!(json["evidence"][1]["status"], "collided");
        assert_eq!(json["evidence"][1]["sameBytesElsewhere"], true);
        assert_eq!(json["evidence"][2]["status"], "missing");
        assert_eq!(json["evidence"][3]["status"], "incompatible");
        for index in 1..4 {
            assert!(json["evidence"][index]["text"].is_null());
            assert!(json["evidence"][index]["subject"].is_null());
            assert!(json["evidence"][index]["entryId"].is_null());
        }
        assert!(!json.to_string().contains("unrelated private account"));
        assert!(!json.to_string().contains("corrected account"));
        assert!(!json.to_string().contains("private receipt account"));
        assert_eq!(json["creation"]["status"], "missing");
        assert_eq!(json["willReturn"], false);
        assert_eq!(json["journalChanged"], false);
        assert_eq!(journal.to_text(), before);
    }

    #[test]
    fn malformed_numeric_arguments_are_refused_instead_of_projected_as_null_or_text() {
        let mut chain = ProjectChain::new();
        chain.keep(&draft()).unwrap();
        let mut preview = chain
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        for token in ["not-a-number", "NaN", "inf", "-inf"] {
            let numinous_core::NextPreview::Ready(call) = &mut preview.next else {
                panic!("ready")
            };
            call.arguments[1].value = numinous_core::ProjectArgumentValue::Number(token.into());
            assert_eq!(
                preview_json(&preview).unwrap_err(),
                "The previewed phase is not a JSON number."
            );
        }
    }
}
