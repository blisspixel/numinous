//! Public CLI study access must remain independent of all player state.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static NEXT_STUDY: AtomicU64 = AtomicU64::new(0);

struct State(PathBuf);

impl State {
    fn new() -> Self {
        let serial = NEXT_STUDY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "numinous-cli-study-{}-{serial}",
            std::process::id()
        ));
        assert!(!root.exists(), "study fixture must start absent");
        Self(root)
    }

    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_numinous"))
            .args(args)
            .env("NUMINOUS_JOURNEY", self.0.join("journey"))
            .env("NUMINOUS_SCORES", self.0.join("scores"))
            .env("NUMINOUS_CAIRN", self.0.join("cairn"))
            .env("NUMINOUS_JOURNAL", self.0.join("journal"))
            .env("NUMINOUS_PREFERENCES", self.0.join("preferences"))
            .output()
            .expect("launch public study command")
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).expect("remove isolated study fixture");
        }
    }
}

fn successful_json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "study should not consult an unavailable profile"
    );
    serde_json::from_slice(&output.stdout).expect("study response is JSON")
}

#[test]
fn public_study_is_available_before_any_visit_and_reports_japanese() {
    let state = State::new();
    let value = successful_json(state.command(&["study", "lissajous", "--locale", "ja", "--json"]));
    assert_eq!(value["selection"]["depth"], "explanation");
    assert_eq!(value["locale"]["requested"], "ja");
    assert_eq!(value["locale"]["resolved"], "ja");
    assert!(value["locale"]["fallback"].is_null());
    let blocks = value["blocks"].as_array().expect("selected blocks");
    assert!(
        blocks
            .iter()
            .all(|block| block["locale"]["resolved"] == "ja")
    );
    assert!(
        blocks
            .iter()
            .all(|block| block["translation"] == "reviewed_draft")
    );
    assert!(
        value["availableDepths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|depth| depth == "mathematics")
    );
    assert!(
        value["availableBlocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|block| block["id"] == "lissajous.recurrence")
    );
    assert!(!state.0.exists(), "study must not create player state");
}

#[test]
fn public_study_opens_a_deep_block_directly_and_preserves_scientific_content() {
    let state = State::new();
    let japanese = successful_json(state.command(&[
        "study",
        "lissajous",
        "--block",
        "lissajous.recurrence",
        "--locale",
        "ja-JP",
        "--json",
    ]));
    let english = successful_json(state.command(&[
        "study",
        "lissajous",
        "--block",
        "lissajous.recurrence",
        "--locale",
        "en",
        "--json",
    ]));
    assert_eq!(japanese["blocks"].as_array().unwrap().len(), 1);
    assert_eq!(japanese["blocks"][0]["depth"], "mathematics");
    assert_eq!(japanese["locale"]["requested"], "ja-jp");
    assert_eq!(japanese["locale"]["fallback"], "parent_language");
    let equations = |value: &Value| {
        value["blocks"][0]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["kind"] == "equation")
            .map(|part| part["notation"].clone())
            .collect::<Vec<_>>()
    };
    assert!(!equations(&japanese).is_empty());
    assert_eq!(equations(&japanese), equations(&english));
    let references = |value: &Value| {
        value["blocks"][0]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["kind"] == "reference")
            .map(|part| part["source"].clone())
            .collect::<Vec<_>>()
    };
    assert!(!references(&japanese).is_empty());
    assert_eq!(references(&japanese), references(&english));
    assert!(!state.0.exists());
}

#[test]
fn public_study_reports_english_fallback_and_leaves_existing_state_untouched() {
    let state = State::new();
    std::fs::create_dir(&state.0).expect("state fixture");
    let files = ["journey", "scores", "cairn", "journal", "preferences"];
    for (index, name) in files.iter().enumerate() {
        std::fs::write(state.0.join(name), format!("preserve {index}\n")).expect("state bytes");
    }
    let value =
        successful_json(state.command(&["study", "times-tables", "--locale", "ja", "--json"]));
    assert_eq!(value["locale"]["requested"], "ja");
    assert_eq!(value["locale"]["resolved"], "en");
    assert_eq!(value["locale"]["fallback"], "translation_unavailable");
    assert_eq!(value["blocks"][0]["translation"], "original");
    for (index, name) in files.iter().enumerate() {
        assert_eq!(
            std::fs::read(state.0.join(name)).unwrap(),
            format!("preserve {index}\n").as_bytes()
        );
    }
    assert_eq!(std::fs::read_dir(&state.0).unwrap().count(), files.len());
}

#[test]
fn public_study_does_not_need_a_readable_journey_file() {
    let state = State::new();
    std::fs::create_dir_all(state.0.join("journey")).expect("invalid Journey path fixture");
    let output = state.command(&[
        "study",
        "lissajous",
        "--depth",
        "mathematics",
        "--locale",
        "ja",
    ]);
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "the Journey path is irrelevant to study"
    );
    let text = String::from_utf8(output.stdout).expect("Japanese is UTF-8");
    assert!(
        text.chars()
            .any(|ch| ('\u{3040}'..='\u{30ff}').contains(&ch))
    );
    assert!(state.0.join("journey").is_dir());
    assert_eq!(std::fs::read_dir(&state.0).unwrap().count(), 1);
}

#[test]
fn public_study_rejects_invalid_or_unavailable_requests_without_state() {
    let state = State::new();
    let cases: &[&[&str]] = &[
        &["study"],
        &["study", "missing-room"],
        &["study", "lissajous", "--locale", "ja_JP"],
        &["study", "lissajous", "--depth", "deep"],
        &["study", "golden-angle", "--depth", "mathematics"],
        &["study", "lissajous", "--block", "lissajous.missing"],
        &["study", "lissajous", "--block", "../recurrence"],
        &["study", "golden-angle", "--block", "lissajous.recurrence"],
        &[
            "study",
            "lissajous",
            "--depth",
            "mathematics",
            "--block",
            "lissajous.recurrence",
        ],
    ];
    for args in cases {
        let output = state.command(args);
        assert!(
            !output.status.success(),
            "request unexpectedly succeeded: {args:?}"
        );
        assert!(!output.stderr.is_empty());
        assert!(
            output.stdout.is_empty(),
            "invalid study must not substitute other content"
        );
    }
    assert!(!state.0.exists());
    let help = state.command(&["study", "--help"]);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for flag in ["--locale", "--depth", "--block", "--json"] {
        assert!(help.contains(flag));
    }
    assert!(help.contains("mathematics"));
}

#[test]
fn refusing_a_depth_names_where_it_is_written_and_offers_it_without_a_requirement() {
    // The chore this removes: with only "unavailable for this room", the sole
    // way to find a written treatment is to ask again for the next room, and
    // the next, across the whole catalog. It also reads as a broken feature.
    let state = State::new();
    let refused = state.command(&["study", "golden-angle", "--depth", "mathematics"]);
    assert!(!refused.status.success());
    let message = String::from_utf8(refused.stderr).unwrap();
    assert!(message.contains("unavailable for this room"));
    for named in numinous_core::AUTHORED_MATHEMATICS_ROOMS {
        assert!(
            message.contains(named),
            "refusal must name {named}: {message}"
        );
    }
    assert!(message.contains("requires nothing"));
    assert!(!message.to_lowercase().contains("unlock"));

    // Following that advice must actually work, from a clean profile.
    for named in numinous_core::AUTHORED_MATHEMATICS_ROOMS {
        let followed = state.command(&["study", named, "--depth", "mathematics"]);
        assert!(
            followed.status.success(),
            "the refusal advertised {named}, which then refused too"
        );
    }
    assert!(!state.0.exists(), "a refusal must not create player state");
}

#[test]
fn structured_study_reports_catalog_coverage_so_no_client_probes_room_by_room() {
    let state = State::new();
    let value = successful_json(state.command(&["study", "golden-angle", "--json"]));
    let coverage = value["authoredDepthRooms"]
        .as_object()
        .expect("coverage must be an object");
    let mathematics = coverage["mathematics"]
        .as_array()
        .expect("mathematics coverage must be a list");
    let named: Vec<&str> = mathematics.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(named, numinous_core::AUTHORED_MATHEMATICS_ROOMS);
    // Every room has these two, so listing rooms for them would restate the
    // catalog and tell a reader nothing.
    assert!(!coverage.contains_key("explanation"));
    assert!(!coverage.contains_key("notes"));
    // Coverage is reported even from a room that lacks the depth, which is the
    // case where a caller would otherwise start hunting.
    assert!(
        !value["availableDepths"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("mathematics"))
    );
    assert!(!state.0.exists());
}
