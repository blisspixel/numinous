//! Public-process smoke coverage for command parsing and its explicit stack.

use std::process::Command;

struct ProjectCliFixture {
    root: std::path::PathBuf,
}

impl ProjectCliFixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "numinous-cli-project-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_numinous"));
        command
            .args(arguments)
            .current_dir(&self.root)
            .env("HOME", &self.root)
            .env("USERPROFILE", &self.root)
            .env("NUMINOUS_PROJECT", self.root.join("chain.txt"))
            .env("NUMINOUS_JOURNAL", self.root.join("journal.txt"))
            .env("NUMINOUS_JOURNEY", self.root.join("journey.txt"))
            .env("NUMINOUS_SCORES", self.root.join("scores.txt"))
            .env("NUMINOUS_CAIRN", self.root.join("cairn.txt"))
            .env("NUMINOUS_PREFERENCES", self.root.join("preferences.txt"))
            .env_remove("NUMINOUS_RADIO");
        command
    }

    fn run(&self, arguments: &[&str]) -> std::process::Output {
        self.command(arguments).output().unwrap()
    }

    fn stdin(&self, arguments: &[&str], input: &[u8]) -> std::process::Output {
        use std::io::Write;
        let mut child = self
            .command(arguments)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }

    fn json(&self, arguments: &[&str]) -> serde_json::Value {
        let output = self.run(arguments);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn draft() -> numinous_core::ProjectDraft {
        numinous_core::ProjectDraft {
            recorded_at_utc: 1,
            question: "Does the saved construction return?".into(),
            next: numinous_core::ProjectNext::PlayRoom {
                room: "lissajous".into(),
                phase: Some("0.250".into()),
            },
            rooms: vec!["lissajous".into()],
            evidence: Vec::new(),
            creation: Some(
                numinous_core::studio_experiment("full-return")
                    .unwrap()
                    .to_num_file(),
            ),
        }
    }
}

impl Drop for ProjectCliFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn public_project_json_reopens_legacy_studio_and_mixed_corrected_route_without_writing() {
    use numinous_core::route_workbench::RouteWorkbench;
    use numinous_core::{ProjectChain, ProjectNext, RouteCreation};
    let fixture = ProjectCliFixture::new("mixed-preview");
    let mut chain = ProjectChain::new();
    chain.keep(&ProjectCliFixture::draft()).unwrap();
    let legacy_document = chain.revision(1).unwrap().to_document();
    assert!(legacy_document.starts_with("NUMINOUS_PROJECT 1\n"));
    std::fs::write(fixture.root.join("chain.txt"), chain.to_text()).unwrap();
    let legacy = fixture.json(&["project", "resume", "--json"]);
    assert_eq!(legacy["next"]["arguments"]["t"], 0.25);
    assert_eq!(legacy["creation"]["kind"], "studio");
    assert_eq!(legacy["creation"]["periodText"], "12");
    assert_eq!(
        legacy["creation"]["capsule"],
        ProjectCliFixture::draft().creation.unwrap()
    );
    assert_eq!(legacy["workspaceChanged"], false);

    let parent = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
    let mut draft = ProjectCliFixture::draft();
    draft.rooms = vec!["route-lab".into()];
    draft.creation = Some(parent.to_capsule());
    draft.next = ProjectNext::RemixRoute;
    chain.keep(&draft).unwrap();
    let mut town = parent.town().clone();
    town.roads[0].road.cost += 4;
    let child = parent.remix(town).unwrap();
    draft.creation = Some(child.to_capsule());
    chain.correct(2, &draft).unwrap();
    let before = chain.to_text();
    assert!(before.starts_with("numinous-project-v2\n"));
    std::fs::write(fixture.root.join("chain.txt"), &before).unwrap();
    let latest = fixture.json(&["project", "resume", "--json"]);
    assert_eq!(latest["creation"]["descends"], parent.identity_hex());
    assert_eq!(latest["parentResolved"], true);
    assert_eq!(latest["next"]["arguments"]["action"], "remix");
    let request = latest["next"]["arguments"].to_string();
    let next = fixture.json(&["route-lab", "--json", "--request", &request]);
    assert_eq!(next["creation"]["parentIdentityHex"], child.identity_hex());
    assert_eq!(
        next["snapshot"]["current"]["roads"][0]["cost"],
        child.town().roads[0].road.cost
    );
    let stale = fixture.json(&["project", "resume", "--revision", "2", "--json"]);
    assert_eq!(stale["supersededBy"], 3);
    assert_eq!(stale["willReturn"], false);
    let refused = fixture.run(&["project", "resume", "--revision", "2", "--apply"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stdout).contains("Apply refused"));
    let old = fixture.run(&["project", "export", "--revision", "1"]);
    assert!(old.status.success());
    assert_eq!(old.stdout, legacy_document.as_bytes());
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("chain.txt")).unwrap(),
        before
    );
    for name in [
        "journal.txt",
        "journey.txt",
        "scores.txt",
        "preferences.txt",
    ] {
        assert!(
            !fixture.root.join(name).exists(),
            "{name} must remain absent"
        );
    }
}

#[test]
fn public_project_reports_unavailable_stored_room_and_creation_without_applying() {
    let fixture = ProjectCliFixture::new("incompatible-preview");
    let mut chain = numinous_core::ProjectChain::new();
    chain.keep(&ProjectCliFixture::draft()).unwrap();
    for (room, status) in [("future-room", "missing"), ("kepler-areas", "incompatible")] {
        let text = chain
            .to_text()
            .replace("lissajous", room)
            .replace("NUMINOUS_STUDIO 3", "NUMINOUS_STUDIO 99");
        std::fs::write(fixture.root.join("chain.txt"), &text).unwrap();
        let preview = fixture.json(&["project", "resume", "--json"]);
        assert_eq!(preview["rooms"][0]["status"], status);
        assert_eq!(preview["creation"]["status"], "incompatible");
        assert_eq!(preview["next"]["status"], "incompatible");
        assert_eq!(preview["willReturn"], false);
        let refused = fixture.run(&["project", "resume", "--apply"]);
        assert!(!refused.status.success());
        let readable = String::from_utf8(refused.stdout).unwrap();
        assert!(readable.contains(&format!("{room} {status}")));
        assert!(readable.contains("Creation: incompatible"));
        assert!(readable.contains("Next: incompatible"));
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("chain.txt")).unwrap(),
            text
        );
    }
    assert!(!fixture.root.join("journal.txt").exists());
}

#[test]
fn public_project_import_collision_confirmation_and_idempotence_preserve_portable_bytes() {
    let fixture = ProjectCliFixture::new("import-admission");
    let mut chain = numinous_core::ProjectChain::new();
    chain.keep(&ProjectCliFixture::draft()).unwrap();
    let document = chain.revision(1).unwrap().to_document();
    let pending = fixture.stdin(&["project", "import", "-"], document.as_bytes());
    assert!(pending.status.success());
    assert!(String::from_utf8_lossy(&pending.stdout).contains("needs --confirm"));
    assert!(!fixture.root.join("chain.txt").exists());
    let confirmed = fixture.stdin(
        &["project", "import", "-", "--confirm"],
        document.as_bytes(),
    );
    assert!(confirmed.status.success());
    assert!(String::from_utf8_lossy(&confirmed.stdout).contains("revision 1 kept"));
    let before = std::fs::read(fixture.root.join("chain.txt")).unwrap();
    let again = fixture.stdin(&["project", "import", "-"], document.as_bytes());
    assert!(again.status.success());
    assert!(String::from_utf8_lossy(&again.stdout).contains("already contains these bytes"));
    assert_eq!(
        std::fs::read(fixture.root.join("chain.txt")).unwrap(),
        before
    );
    let mut different = ProjectCliFixture::draft();
    different.question = "An explicitly different project".into();
    let mut incoming = numinous_core::ProjectChain::new();
    incoming.keep(&different).unwrap();
    let incoming = incoming.revision(1).unwrap().to_document();
    let collision = fixture.stdin(
        &["project", "import", "-", "--origin", "1"],
        incoming.as_bytes(),
    );
    assert!(collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stdout).contains("holds different bytes"));
    assert_eq!(
        std::fs::read(fixture.root.join("chain.txt")).unwrap(),
        before
    );
    let confirmed = fixture.stdin(
        &[
            "project",
            "import",
            "-",
            "--origin",
            "1",
            "--confirm",
            "--json",
        ],
        incoming.as_bytes(),
    );
    assert!(confirmed.status.success());
    let confirmed: serde_json::Value = serde_json::from_slice(&confirmed.stdout).unwrap();
    assert_eq!(confirmed["outcome"], "appended");
    assert_eq!(confirmed["revisionId"], 2);
    assert_eq!(
        fixture
            .run(&["project", "export", "--revision", "1"])
            .stdout,
        document.as_bytes()
    );
    assert_eq!(
        fixture.run(&["project", "export"]).stdout,
        incoming.as_bytes()
    );
    assert!(!fixture.root.join("journal.txt").exists());
}

#[test]
fn public_project_evidence_preview_distinguishes_missing_corrected_and_collided_accounts() {
    use numinous_core::{Journal, JournalRecord, ProjectChain, ProjectEvidence};
    let fixture = ProjectCliFixture::new("evidence-preview");
    let mut journal = Journal::new();
    let record = |text| JournalRecord {
        recorded_at_utc: 1,
        event_at_utc: 1,
        source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
        kind: "encounter",
        subject: "lissajous",
        text,
        affect: None,
    };
    let cited = journal.record(record("Original cited prediction")).unwrap();
    let other = journal
        .record(record("Unrelated private prediction"))
        .unwrap();
    let digest = journal.entry(cited).unwrap().identity_digest();
    let correction = journal
        .correct(
            2,
            None,
            numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            cited,
            "Later corrected prediction",
            None,
        )
        .unwrap();
    let mut draft = ProjectCliFixture::draft();
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
    ];
    let mut chain = ProjectChain::new();
    chain.keep(&draft).unwrap();
    let project_bytes = chain.to_text();
    let journal_bytes = journal.to_text();
    std::fs::write(fixture.root.join("chain.txt"), &project_bytes).unwrap();
    std::fs::write(fixture.root.join("journal.txt"), &journal_bytes).unwrap();
    let json = fixture.json(&["project", "resume", "--json"]);
    assert_eq!(json["evidence"][0]["status"], "corrected");
    assert_eq!(json["evidence"][0]["supersededBy"], correction);
    assert_eq!(json["evidence"][0]["text"], "Original cited prediction");
    assert_eq!(json["evidence"][1]["status"], "collided");
    assert_eq!(json["evidence"][1]["sameBytesElsewhere"], true);
    assert!(json["evidence"][1]["text"].is_null());
    assert_eq!(json["evidence"][2]["status"], "missing");
    assert_eq!(json["willReturn"], false);
    let readable = fixture.run(&["project", "resume"]);
    assert!(readable.status.success());
    let readable = String::from_utf8(readable.stdout).unwrap();
    for status in ["corrected", "collided", "missing"] {
        assert!(readable.contains(&format!("journal {status}")));
    }
    for private_text in [
        "Original cited prediction",
        "Unrelated private prediction",
        "Later corrected prediction",
    ] {
        assert!(!readable.contains(private_text));
    }
    assert!(!json.to_string().contains("Unrelated private prediction"));
    assert!(!json.to_string().contains("Later corrected prediction"));
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("chain.txt")).unwrap(),
        project_bytes
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("journal.txt")).unwrap(),
        journal_bytes
    );
}

#[test]
fn public_project_bad_inputs_and_export_no_clobber_leave_durable_state_unchanged() {
    let fixture = ProjectCliFixture::new("failed-admission");
    let route = numinous_core::RouteCreation::new(
        numinous_core::route_workbench::RouteWorkbench::first_town()
            .town()
            .clone(),
    )
    .unwrap()
    .to_capsule();
    let keep = fixture.stdin(
        &[
            "project",
            "keep",
            "--question",
            "Keep this route",
            "--route",
            "-",
        ],
        route.as_bytes(),
    );
    assert!(keep.status.success());
    let kept = fixture.json(&["project", "resume", "--json"]);
    assert_eq!(kept["next"]["arguments"]["action"], "open");
    let request = kept["next"]["arguments"].to_string();
    let reopened = fixture.json(&["route-lab", "--json", "--request", &request]);
    assert_eq!(reopened["snapshot"]["revision"], 0);
    assert_eq!(
        reopened["creation"]["parentIdentityHex"],
        serde_json::Value::Null
    );
    let before = std::fs::read(fixture.root.join("chain.txt")).unwrap();
    std::fs::write(
        fixture.root.join("journal.txt"),
        numinous_core::Journal::new().to_text(),
    )
    .unwrap();
    let journal = std::fs::read(fixture.root.join("journal.txt")).unwrap();
    let invalid_route = fixture.root.join("input.route");
    for input in [
        vec![0xff],
        vec![b'x'; numinous_core::MAX_ROUTE_CAPSULE_BYTES + 1],
        b"NUMINOUS_ROUTE 9\n".to_vec(),
    ] {
        std::fs::write(&invalid_route, input).unwrap();
        let result = fixture.run(&[
            "project",
            "keep",
            "--question",
            "Invalid route",
            "--route",
            invalid_route.to_str().unwrap(),
        ]);
        assert!(!result.status.success());
        assert_eq!(
            std::fs::read(fixture.root.join("chain.txt")).unwrap(),
            before
        );
    }
    for arguments in [
        vec![
            "project",
            "keep",
            "--question",
            "Missing",
            "--route",
            "missing.route",
        ],
        vec!["project", "import", "missing.project", "--confirm"],
        vec!["project", "resume", "--revision", "99", "--json"],
    ] {
        assert!(!fixture.run(&arguments).status.success());
    }
    let empty_question = fixture.stdin(
        &["project", "keep", "--question", " ", "--route", "-"],
        route.as_bytes(),
    );
    assert!(!empty_question.status.success());
    for input in [
        vec![0xff],
        vec![b'x'; numinous_core::MAX_PROJECT_FILE_BYTES as usize + 1],
        b"NUMINOUS_PROJECT 99\n".to_vec(),
    ] {
        let invalid = fixture.root.join("input.project");
        std::fs::write(&invalid, input).unwrap();
        assert!(
            !fixture
                .run(&["project", "import", invalid.to_str().unwrap(), "--confirm"])
                .status
                .success()
        );
    }
    let target = fixture.root.join("portable.project");
    std::fs::write(&target, "Existing player document").unwrap();
    assert!(
        !fixture
            .run(&["project", "export", "--out", target.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "Existing player document"
    );
    let absent = fixture.root.join("absent.project");
    assert!(
        !fixture
            .run(&[
                "project",
                "export",
                "--revision",
                "99",
                "--out",
                absent.to_str().unwrap()
            ])
            .status
            .success()
    );
    assert!(!absent.exists());
    let missing_parent = fixture
        .root
        .join("missing-directory")
        .join("export.project");
    assert!(
        !fixture
            .run(&[
                "project",
                "export",
                "--out",
                missing_parent.to_str().unwrap()
            ])
            .status
            .success()
    );
    assert!(!missing_parent.exists());
    assert_eq!(
        std::fs::read(fixture.root.join("chain.txt")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(fixture.root.join("journal.txt")).unwrap(),
        journal
    );
    std::fs::write(fixture.root.join("journal.txt"), "malformed journal").unwrap();
    assert!(
        !fixture
            .run(&["project", "resume", "--json"])
            .status
            .success()
    );
    assert!(
        !fixture
            .run(&["project", "resume", "--apply"])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("journal.txt")).unwrap(),
        "malformed journal"
    );
    assert_eq!(
        std::fs::read(fixture.root.join("chain.txt")).unwrap(),
        before
    );
    std::fs::write(fixture.root.join("chain.txt"), "malformed chain").unwrap();
    assert!(
        !fixture
            .run(&["project", "resume", "--json"])
            .status
            .success()
    );
    assert!(!fixture.run(&["project", "export"]).status.success());
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("chain.txt")).unwrap(),
        "malformed chain"
    );
}

#[test]
fn public_route_project_exports_keeps_imports_and_reopens_exact_authored_state() {
    use serde_json::Value;
    let root =
        std::env::temp_dir().join(format!("numinous-cli-route-project-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let capsule = root.join("authored.route");
    let document = root.join("portable.project");
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_numinous"))
            .args(args)
            .env("NUMINOUS_PROJECT", root.join("chain.txt"))
            .env("NUMINOUS_JOURNAL", root.join("journal.txt"))
            .env("NUMINOUS_JOURNEY", root.join("journey.txt"))
            .env("NUMINOUS_SCORES", root.join("scores.txt"))
            .env("NUMINOUS_CAIRN", root.join("cairn.txt"))
            .env("NUMINOUS_PREFERENCES", root.join("preferences.txt"))
            .output()
            .unwrap()
    };
    let exported = invoke(&["route-lab", "--json", "--out", capsule.to_str().unwrap()]);
    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let data = std::fs::read(&capsule).unwrap();
    assert!(data.starts_with(b"NUMINOUS_ROUTE 1\n"));
    let no_clobber = invoke(&["route-lab", "--out", capsule.to_str().unwrap()]);
    assert!(!no_clobber.status.success());
    assert_eq!(std::fs::read(&capsule).unwrap(), data);
    let refused_target = root.join("refused.route");
    let invalid = invoke(&[
        "route-lab",
        "--out",
        refused_target.to_str().unwrap(),
        "--request",
        r#"{"action":{"type":"order","order":[0,1,1,3]}}"#,
    ]);
    assert!(!invalid.status.success());
    assert!(
        !refused_target.exists(),
        "invalid route must fail before file creation"
    );
    let kept = invoke(&[
        "project",
        "keep",
        "--question",
        "Which delivery order is cheaper?",
        "--route",
        capsule.to_str().unwrap(),
        "--remix",
        "--json",
    ]);
    assert!(
        kept.status.success(),
        "{}",
        String::from_utf8_lossy(&kept.stderr)
    );
    let kept: Value = serde_json::from_slice(&kept.stdout).unwrap();
    assert_eq!(kept["outcome"], "appended");
    let exported = invoke(&["project", "export", "--out", document.to_str().unwrap()]);
    assert!(exported.status.success());
    let doc = std::fs::read(&document).unwrap();
    assert!(doc.starts_with(b"NUMINOUS_PROJECT 2\n"));
    let chain = std::fs::read(root.join("chain.txt")).unwrap();
    assert!(chain.starts_with(b"numinous-project-v2\n"));
    let preview = invoke(&["project", "resume", "--json"]);
    assert!(preview.status.success());
    let preview: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["creation"]["kind"], "route");
    assert_eq!(preview["next"]["arguments"]["action"], "remix");
    assert_eq!(std::fs::read(root.join("chain.txt")).unwrap(), chain);
    let request = preview["next"]["arguments"].to_string();
    let reopened = invoke(&["route-lab", "--json", "--request", &request]);
    assert!(reopened.status.success());
    let reopened: Value = serde_json::from_slice(&reopened.stdout).unwrap();
    assert_eq!(reopened["snapshot"]["revision"], 0);
    assert_eq!(reopened["snapshot"]["undo"], serde_json::json!([]));
    assert!(reopened["snapshot"]["trace"].is_null());
    assert!(reopened["creation"]["parentIdentityHex"].is_string());
    std::fs::remove_file(root.join("chain.txt")).unwrap();
    let pending = invoke(&["project", "import", document.to_str().unwrap(), "--json"]);
    assert!(pending.status.success());
    let pending: Value = serde_json::from_slice(&pending.stdout).unwrap();
    assert_eq!(pending["outcome"], "needs_confirm");
    assert!(!root.join("chain.txt").exists());
    let imported = invoke(&[
        "project",
        "import",
        document.to_str().unwrap(),
        "--confirm",
        "--json",
    ]);
    assert!(imported.status.success());
    let roundtrip = invoke(&["project", "export"]);
    assert!(roundtrip.status.success());
    assert_eq!(roundtrip.stdout, doc);
    let malformed = root.join("malformed.route");
    std::fs::write(&malformed, "NUMINOUS_ROUTE 9\n").unwrap();
    let before = std::fs::read(root.join("chain.txt")).unwrap();
    assert!(
        !invoke(&[
            "project",
            "keep",
            "--question",
            "Bad route",
            "--route",
            malformed.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(std::fs::read(root.join("chain.txt")).unwrap(), before);
    assert!(!root.join("journal.txt").exists());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn public_route_workbench_carries_arbitrary_town_and_undo_without_profile_io() {
    use serde_json::{Value, json};
    let root = std::env::temp_dir().join(format!(
        "numinous-cli-route-workbench-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let route = |request: Value| {
        let input = request.to_string();
        let output = isolated_command(&root, &["route-lab", "--json", "--request", &input]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).expect("typed workbench output")
    };
    let first = route(json!({}));
    assert_eq!(first["comparison"]["current"]["cost"], 9);
    let improved = route(first["next"]["arguments"].clone());
    assert_eq!(improved["comparison"]["current"]["cost"], 8);
    let undone = route(json!({"snapshot":improved["snapshot"],"action":{"type":"undo"}}));
    assert_eq!(undone["snapshot"]["current"]["order"], json!([0, 1, 2, 3]));
    let custom = json!({"revision":0,"undo":[],"trace":null,"current":{"junctions":5,"roads":[{"from":0,"to":1,"cost":1,"open":true},{"from":1,"to":2,"cost":1,"open":true},{"from":2,"to":3,"cost":1,"open":true},{"from":3,"to":4,"cost":1,"open":true},{"from":0,"to":4,"cost":2,"open":true}],"stops":[0,2,4],"order":[0,4,2]}});
    let custom = route(json!({"snapshot":custom}));
    assert_eq!(custom["snapshot"]["current"]["order"], json!([0, 4, 2]));
    assert_eq!(custom["comparison"]["current"]["cost"], 6);
    assert_eq!(custom["comparison"]["exact"]["tour"]["cost"], 6);
    let depot = route(json!({"snapshot":custom["snapshot"],"action":{"type":"depot","depot":4}}));
    assert_eq!(depot["snapshot"]["current"]["order"][0], 4);
    assert_eq!(depot["snapshot"]["current"]["stops"][0], 4);
    let rejected = isolated_command(
        &root,
        &[
            "route-lab",
            "--json",
            "--request",
            r#"{"action":{"type":"order","order":[0,1,1,3]}}"#,
        ],
    );
    assert!(!rejected.status.success());
    assert!(
        !root.exists(),
        "caller-carried workbench must not touch profile files"
    );
}

#[test]
fn public_route_workbench_accepts_bounded_stdin_and_refuses_oversized_input() {
    use std::io::Write;
    use std::process::Stdio;
    let run = |input: &[u8]| {
        let mut process = Command::new(env!("CARGO_BIN_EXE_numinous"))
            .args(["route-lab", "--json", "--request", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn CLI");
        process
            .stdin
            .take()
            .unwrap()
            .write_all(input)
            .expect("write route request");
        process.wait_with_output().expect("wait CLI")
    };
    let accepted = run(br#"{"action":{"type":"trace","from":0,"to":3}}"#);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&accepted.stdout).unwrap();
    assert_eq!(output["trace"]["cursor"], 0);
    assert!(output["trace"]["result"].is_null());
    let rejected = run(&vec![b' '; 512 * 1024 + 1]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("byte bound"));
    assert!(!run(&[0xff]).status.success());
}

#[test]
fn public_route_search_view_restores_visible_knowledge_and_rejects_forged_snapshots() {
    use serde_json::{Value, json};
    let fixture = ProjectCliFixture::new("route-search-view");
    let route =
        |request: Value| fixture.json(&["route-lab", "--json", "--request", &request.to_string()]);
    let snapshot = json!({"revision":0,"undo":[],"trace":null,"current":{"junctions":5,"roads":[{"from":0,"to":1,"cost":5,"open":true},{"from":0,"to":2,"cost":1,"open":true},{"from":2,"to":1,"cost":1,"open":true},{"from":1,"to":3,"cost":1,"open":true}],"stops":[0,1,3],"order":[0,3,1]}});
    let start = route(json!({"snapshot":snapshot,"action":{"type":"trace","from":0,"to":3}}));
    assert_eq!(start["trace"]["view"]["junctions"][0]["state"], "tentative");
    assert_eq!(start["trace"]["view"]["junctions"][4]["state"], "unseen");
    assert!(start["trace"]["view"]["activeEvent"].is_null());
    let first = route(start["next"]["arguments"].clone());
    assert_eq!(first["trace"]["view"]["junctions"][0]["state"], "settled");
    let early = route(json!({"snapshot":first["snapshot"],"action":{"type":"step","cursor":2}}));
    assert_eq!(
        early["trace"]["view"]["junctions"][1],
        json!({"junction":1,"state":"tentative","cost":5,"predecessor":0})
    );
    assert!(early["trace"]["result"].is_null());
    let request = json!({"snapshot":early["snapshot"]}).to_string();
    let plain = fixture.run(&["route-lab", "--request", &request]);
    assert!(plain.status.success());
    let plain = String::from_utf8(plain.stdout).unwrap();
    assert!(plain.contains("Junction 1: tentative cost 5 from 0; predecessor 0."));
    assert!(plain.contains("Junction 4: unseen; no cost revealed."));
    assert!(plain.contains("Active event: Tentative cost from 0 to 1 via 0: 5."));
    assert!(!plain.contains("Shortest path:"));
    let restored = route(json!({"snapshot":early["snapshot"]}));
    assert_eq!(restored["trace"], early["trace"]);
    let mut complete = restored;
    let event_count = complete["trace"]["eventCount"].as_u64().unwrap();
    let first_cursor = complete["trace"]["cursor"].as_u64().unwrap() + 1;
    for expected_cursor in first_cursor..=event_count {
        assert_eq!(complete["next"]["tool"], "route_lab");
        complete = route(complete["next"]["arguments"].clone());
        assert_eq!(complete["trace"]["cursor"], expected_cursor);
        assert_eq!(complete["trace"]["eventCount"], event_count);
        assert_eq!(
            complete["trace"]["completed"],
            expected_cursor == event_count
        );
        assert_eq!(
            complete["trace"]["result"].is_null(),
            expected_cursor < event_count
        );
    }
    assert_eq!(complete["trace"]["completed"], true);
    assert_eq!(
        complete["trace"]["view"]["junctions"][1],
        json!({"junction":1,"state":"settled","cost":2,"predecessor":2})
    );
    assert_eq!(
        complete["trace"]["view"]["junctions"][4]["state"],
        "unreachable"
    );
    assert_eq!(
        complete["trace"]["result"]["junctions"],
        json!([0, 2, 1, 3])
    );
    let backwards =
        route(json!({"snapshot":complete["snapshot"],"action":{"type":"step","cursor":2}}));
    assert_eq!(backwards["trace"], early["trace"]);
    assert!(backwards["trace"]["result"].is_null());
    let changed = route(
        json!({"snapshot":complete["snapshot"],"action":{"type":"road_cost","from":0,"to":1,"cost":4}}),
    );
    assert!(changed["trace"].is_null());
    let mut forged = complete["snapshot"].clone();
    forged["trace"]["view"] = complete["trace"]["view"].clone();
    let mut stale = complete["snapshot"].clone();
    stale["current"]["roads"][0]["cost"] = json!(9);
    for snapshot in [forged, stale] {
        let rejected = fixture.run(&[
            "route-lab",
            "--json",
            "--request",
            &json!({"snapshot":snapshot}).to_string(),
        ]);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
    }
    assert!(!fixture.root.join("chain.txt").exists());
    assert!(!fixture.root.join("journal.txt").exists());
}

fn isolated_command(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(args)
        .env("NUMINOUS_JOURNEY", root.join("journey.txt"))
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", root.join("preferences.txt"))
        .output()
        .expect("launch the public CLI binary")
}

#[test]
fn public_binary_crosses_the_explicit_command_stack() {
    let state_root =
        std::env::temp_dir().join(format!("numinous-cli-smoke-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["sonify", "--help"])
        .env("NUMINOUS_JOURNEY", state_root.join("journey.txt"))
        .env("NUMINOUS_SCORES", state_root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", state_root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", state_root.join("preferences.txt"))
        .output()
        .expect("launch the public CLI binary");

    assert!(
        output.status.success(),
        "public help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
    assert!(stdout.contains("--layer <LAYER>"));
    assert!(stdout.contains("room-bed"));
    assert!(stdout.contains("--variation <VARIATION>"));
    assert!(!state_root.exists(), "help must not create player state");
}

#[test]
fn public_route_lab_compares_and_improves_the_same_street_problem() {
    let root = std::env::temp_dir().join(format!(
        "numinous-cli-route-lab-smoke-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let described = isolated_command(&root, &["describe", "route-lab"]);
    assert!(described.status.success());
    let description = String::from_utf8(described.stdout).expect("description is UTF-8");
    assert!(description.contains("Route Lab"));
    assert!(!description.contains("Reveal:"));

    let render = |pokes: &[&str], expected: &str| {
        let mut arguments = vec!["render", "route-lab", "--width", "64", "--height", "28"];
        for poke in pokes {
            arguments.extend(["--poke", poke]);
        }
        let output = isolated_command(&root, &arguments);
        assert!(
            output.status.success(),
            "route render failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).expect("route render is UTF-8");
        assert!(text.contains(expected), "{text}");
    };
    render(&[], "ORDER=ABCD cost=9 opt=8 save=1 BD=3");
    render(&["0.75,0.92"], "ORDER=ABDC cost=8 opt=8 save=0 BD=3");
    render(
        &["0.75,0.92", "0.75,0.92"],
        "ORDER=ABDC cost=8 opt=8 save=0 BD=3",
    );
    render(
        &["0.75,0.92", "0.25,0.92"],
        "ORDER=ABCD cost=9 opt=8 save=1 BD=3",
    );
    render(&["0.5,0.78"], "ORDER=ABCD cost=9 opt=9 save=0 BD=5");
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_describe_is_safe_and_reveal_is_earned() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-reveal-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    let described = isolated_command(&root, &["describe", "kepler-laws"]);
    assert!(described.status.success());
    let described = String::from_utf8(described.stdout).expect("description is UTF-8");
    assert!(described.contains("Kepler Areas"));
    assert!(described.contains("Play: numinous render kepler-laws"));
    assert!(!described.contains("Reveal:"));
    assert!(!described.contains("second law"));

    let early = isolated_command(&root, &["reveal", "lissajous"]);
    assert!(!early.status.success());
    let early = String::from_utf8(early.stderr).expect("early refusal is UTF-8");
    assert!(early.contains("Render or play the room once, then ask again"));

    let played = isolated_command(
        &root,
        &["render", "lissajous", "--width", "24", "--height", "12"],
    );
    assert!(played.status.success());
    let revealed = isolated_command(&root, &["reveal", "lissajous"]);
    assert!(revealed.status.success());
    let revealed = String::from_utf8(revealed.stdout).expect("reveal is UTF-8");
    assert!(revealed.contains("rational frequency ratio"));

    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_journey_override_names_the_file_contract() {
    let root = std::env::temp_dir().join(format!(
        "numinous-cli-directory-contract-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("journey.txt")).expect("Journey directory fixture");

    let output = isolated_command(&root, &["journey"]);
    let stderr = String::from_utf8(output.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("NUMINOUS_JOURNEY must name a file"));
    assert!(stderr.contains("is a directory"));

    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_forget_previews_fails_closed_and_erases_isolated_state() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-forget-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let journey = root.join("journey.txt");
    let scores = root.join("scores.txt");
    let cairn = root.join("cairn.txt");
    let journal = home.join(".numinous-journal");
    let preferences = home.join(".numinous-preferences");
    let radio = home.join(".numinous-radio");
    let crash = home.join(".numinous-crash.log");
    std::fs::create_dir_all(&journey).expect("unexpected Journey directory");
    std::fs::create_dir_all(&radio).expect("radio fixture");
    std::fs::write(&scores, b"50\tmunch seed:1 board:0\n").expect("score fixture");
    std::fs::write(&cairn, b"Ada\ttruth survives inspection\n").expect("Cairn fixture");
    std::fs::create_dir_all(&home).expect("state home fixture");
    std::fs::write(&journal, b"NUMINOUS_JOURNAL\t2\n").expect("journal fixture");
    std::fs::write(
        &preferences,
        numinous_core::AppPreferences::default().to_text(),
    )
    .expect("preferences fixture");
    std::fs::write(radio.join("trance-001.wav"), b"RIFF").expect("radio fixture");
    std::fs::write(&crash, b"isolated diagnostic").expect("crash fixture");

    let command = |args: &[&str], protected_radio: Option<&std::path::Path>| {
        let mut process = Command::new(env!("CARGO_BIN_EXE_numinous"));
        process
            .args(args)
            .env("NUMINOUS_JOURNEY", &journey)
            .env("NUMINOUS_SCORES", &scores)
            .env("NUMINOUS_CAIRN", &cairn)
            .env("NUMINOUS_PREFERENCES", &preferences)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env_remove("NUMINOUS_RADIO");
        if let Some(path) = protected_radio {
            process.env("NUMINOUS_RADIO", path);
        }
        process.output().expect("launch public forget command")
    };

    let preview = command(&["forget"], None);
    assert!(preview.status.success());
    let preview_text = String::from_utf8(preview.stdout).expect("preview is UTF-8");
    assert!(preview_text.contains("unexpected non-file object"));
    assert!(preview_text.contains("journey.txt"));
    assert!(preview_text.contains(".numinous-journal"));
    assert!(preview_text.contains(".numinous-radio"));
    assert!(journey.is_dir(), "preview is non-destructive");

    let blocked = command(&["forget", "--confirm", "--all-local"], None);
    assert!(!blocked.status.success());
    let blocked_text = String::from_utf8(blocked.stderr).expect("failure is UTF-8");
    assert!(blocked_text.contains("Erasure stopped at journey"));
    assert!(scores.is_file(), "global preflight preserves later stores");

    std::fs::remove_dir(&journey).expect("replace invalid Journey object");
    std::fs::write(&journey, b"visited lorenz\nplays 1\n").expect("Journey fixture");

    let protected = command(&["forget", "--confirm", "--all-local"], Some(&radio));
    assert!(!protected.status.success());
    let protected_text = String::from_utf8(protected.stderr).expect("failure is UTF-8");
    assert!(protected_text.contains("selected radio source"));
    for path in [
        &journey,
        &scores,
        &cairn,
        &journal,
        &preferences,
        &radio,
        &crash,
    ] {
        assert!(path.exists(), "{} must be preserved", path.display());
    }

    let journal_erased = command(&["forget", "--confirm", "--journal"], None);
    assert!(journal_erased.status.success());
    assert!(!journey.exists(), "Journey is the baseline selected store");
    assert!(
        !journal.exists(),
        "explicit journal selection must erase it"
    );
    for path in [&scores, &cairn, &preferences, &radio, &crash] {
        assert!(path.exists(), "{} must be preserved", path.display());
    }

    std::fs::write(&journey, b"visited lorenz\nplays 1\n").expect("Journey replacement");
    std::fs::write(&journal, b"NUMINOUS_JOURNAL\t2\n").expect("journal replacement");
    let erased = command(&["forget", "--confirm", "--all-local"], None);
    assert!(
        erased.status.success(),
        "complete erasure failed: {}",
        String::from_utf8_lossy(&erased.stderr)
    );
    let erased_text = String::from_utf8(erased.stdout).expect("receipt is UTF-8");
    assert!(erased_text.contains("0 managed stores and 0 known bytes remain"));
    for path in [
        &journey,
        &scores,
        &cairn,
        &journal,
        &preferences,
        &radio,
        &crash,
    ] {
        assert!(!path.exists(), "{} must be absent", path.display());
    }
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_project_resume_apply_stays_inside_the_process() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-project-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    let project = root.join("project.txt");
    let journal = root.join("journal.txt");
    let journey = root.join("journey.txt");
    numinous_core::keep_project_file(
        &project,
        &numinous_core::ProjectDraft {
            recorded_at_utc: 1_700_000_000,
            question: "Which room is this?".to_string(),
            next: numinous_core::ProjectNext::PlayRoom {
                room: "lissajous".to_string(),
                phase: Some("0.25".to_string()),
            },
            rooms: vec!["lissajous".to_string()],
            evidence: Vec::new(),
            creation: None,
        },
    )
    .expect("keep");
    let project_bytes = std::fs::read(&project).expect("project bytes");
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["project", "resume", "--apply"])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_PROJECT", &project)
        .env("NUMINOUS_JOURNAL", &journal)
        .env("NUMINOUS_JOURNEY", &journey)
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", root.join("preferences.txt"))
        .env_remove("NUMINOUS_RADIO")
        .output()
        .expect("launch the public CLI binary");
    assert!(
        output.status.success(),
        "resume failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("resume is UTF-8");
    assert!(stdout.contains("Intention: Which room is this?"));
    assert!(stdout.contains("Place: lissajous t=0.25"));
    assert!(stdout.contains("Preview workspace changed: false"));
    assert!(stdout.contains("Preview journal changed: false"));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(&project).expect("project after"),
        project_bytes
    );
    assert!(!journal.exists(), "resume must not create a journal");
    assert!(!journey.exists(), "resume must not create a journey");
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_project_resume_of_an_empty_chain_fails_without_creating_files() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-project-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    let project = root.join("project.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["project", "resume", "--apply"])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_PROJECT", &project)
        .env("NUMINOUS_JOURNAL", root.join("journal.txt"))
        .env("NUMINOUS_JOURNEY", root.join("journey.txt"))
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env_remove("NUMINOUS_RADIO")
        .output()
        .expect("launch the public CLI binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("project chain is empty"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !project.exists(),
        "a failed resume must not create the chain"
    );
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}
