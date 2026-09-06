use std::collections::BTreeSet;

use super::{
    AUTHORED_MATHEMATICS_ROOMS, MAX_STUDY_BLOCK_ID_BYTES, MAX_STUDY_LOCALE_BYTES, RoomStudy,
    StudyBlock, StudyDepth, StudyDepthError, StudyFallback, StudyInline, StudyLocale,
    StudyLocaleError, StudyPart, StudyRequest, StudyRequestError, StudySelection,
    StudyTranslationStatus, room_study, room_study_for_locale, rooms_with_authored_depth,
};
use crate::{Room, RoomMeta, RoomMetadata, Surface, all_rooms, room_by_id};

const PILOT_IDS: [&str; 11] = [
    "lissajous.try",
    "lissajous.intuition",
    "lissajous.equations",
    "lissajous.state",
    "lissajous.phase",
    "lissajous.torus",
    "lissajous.measure",
    "lissajous.recurrence",
    "lissajous.sound",
    "lissajous.limits",
    "lissajous.references",
];

fn paragraph_text(part: &StudyPart) -> String {
    match part {
        StudyPart::Paragraph(runs) => runs
            .iter()
            .map(|run| match run {
                StudyInline::Text(text) | StudyInline::Math(text) => *text,
            })
            .collect(),
        _ => panic!("expected one source paragraph"),
    }
}

fn only_paragraph(block: &StudyBlock) -> String {
    assert_eq!(block.parts.len(), 1);
    paragraph_text(&block.parts[0])
}

#[derive(Debug, PartialEq, Eq)]
enum ScientificItem {
    Inline(&'static str),
    Equation(&'static str),
    Reference(&'static str, &'static str, &'static str),
}

fn science(block: &StudyBlock) -> Vec<ScientificItem> {
    let mut result = Vec::new();
    for part in &block.parts {
        match part {
            StudyPart::Paragraph(runs) => {
                result.extend(runs.iter().filter_map(|run| match run {
                    StudyInline::Math(text) => Some(ScientificItem::Inline(text)),
                    StudyInline::Text(_) => None,
                }));
            }
            StudyPart::Equation(text) => result.push(ScientificItem::Equation(text)),
            StudyPart::Reference { source, .. } => result.push(ScientificItem::Reference(
                source.id,
                source.title,
                source.url,
            )),
        }
    }
    result
}

fn inline_math(document: &RoomStudy, id: &str) -> Vec<&'static str> {
    science(document.block(id).expect("direct block access"))
        .into_iter()
        .filter_map(|item| match item {
            ScientificItem::Inline(text) => Some(text),
            _ => None,
        })
        .collect()
}

#[test]
fn language_requests_are_bounded_canonical_and_explicit() {
    for (request, canonical, primary) in [
        ("en", "en", "en"),
        ("JA-jP", "ja-jp", "ja"),
        ("zh-Hant-TW", "zh-hant-tw", "zh"),
        ("es-419", "es-419", "es"),
        ("haw", "haw", "haw"),
        ("tlh", "tlh", "tlh"),
        ("ZZ-Custom", "zz-custom", "zz"),
    ] {
        let locale: StudyLocale = request.parse().expect("admitted request syntax");
        assert_eq!(locale.as_str(), canonical);
        assert_eq!(locale.language(), primary);
        assert_eq!(locale.to_string(), canonical);
        assert_eq!(StudyLocale::parse(canonical), Ok(locale));
    }
    assert_eq!(StudyLocale::default().as_str(), "en");
    let at_limit = ["abcdefg"; 8].join("-");
    assert_eq!(at_limit.len(), MAX_STUDY_LOCALE_BYTES);
    assert!(StudyLocale::parse(&at_limit).is_ok());
    assert_eq!(
        StudyLocale::parse(&(at_limit + "h")),
        Err(StudyLocaleError::TooLong)
    );
    assert_eq!(StudyLocale::parse(""), Err(StudyLocaleError::Empty));
    for request in [
        "e",
        "englishhh",
        "en_US",
        " en",
        "en ",
        "ja--JP",
        "en-",
        "-en",
        "eñ",
        "日本語",
        "en-u-ca-japanese",
        "x-private",
        "en-abcdefghi",
        "en-aa-aa-aa-aa-aa-aa-aa-aa",
        "en\0US",
        "en\nUS",
    ] {
        assert_eq!(
            StudyLocale::parse(request),
            Err(StudyLocaleError::InvalidSyntax),
            "request {request:?}"
        );
    }
    for error in [
        StudyLocaleError::Empty,
        StudyLocaleError::TooLong,
        StudyLocaleError::InvalidSyntax,
    ] {
        assert!(!error.to_string().is_empty());
        let typed: &dyn std::error::Error = &error;
        assert!(typed.source().is_none());
    }
}

#[test]
fn fallback_reports_actual_language_per_document_and_block() {
    let room = room_by_id("lissajous").expect("pilot room");
    for (requested, resolved, fallback) in [
        ("en", "en", None),
        ("EN-us", "en", Some(StudyFallback::ParentLanguage)),
        ("ja", "ja", None),
        ("JA-jP", "ja", Some(StudyFallback::ParentLanguage)),
        ("haw", "en", Some(StudyFallback::TranslationUnavailable)),
        ("tlh", "en", Some(StudyFallback::TranslationUnavailable)),
        (
            "zz-Custom",
            "en",
            Some(StudyFallback::TranslationUnavailable),
        ),
    ] {
        let document = room_study(room.as_ref(), requested).expect("bounded request");
        assert_eq!(
            document.locale.requested.as_str(),
            requested.to_ascii_lowercase()
        );
        assert_eq!(document.locale.resolved, resolved);
        assert_eq!(document.locale.fallback, fallback);
        assert_eq!(document.content_locales, &["en", "ja"]);
        assert_eq!(
            document,
            room_study_for_locale(room.as_ref(), &StudyLocale::parse(requested).unwrap())
        );
        for id in PILOT_IDS {
            let block = document.block(id).expect("every pilot depth available");
            assert_eq!(block.locale, document.locale);
            assert_eq!(
                block.translation,
                if resolved == "ja" {
                    StudyTranslationStatus::ReviewedDraft
                } else {
                    StudyTranslationStatus::Original
                }
            );
        }
    }
    let japanese = room_study(room.as_ref(), "ja").unwrap();
    let entrance = japanese
        .blocks_at(StudyDepth::Explanation)
        .collect::<Vec<_>>();
    assert_eq!(entrance.len(), 2);
    assert_eq!(entrance[0].id, "lissajous.try");
    assert_eq!(entrance[1].id, "lissajous.intuition");
    assert_eq!(japanese.blocks[0].id, "lissajous.try");
    assert_eq!(entrance[0].locale.resolved, "ja");
    for block in japanese.blocks_at(StudyDepth::Notes) {
        assert_eq!(block.locale.resolved, "en");
        assert_eq!(
            block.locale.fallback,
            Some(StudyFallback::TranslationUnavailable)
        );
        assert_eq!(block.translation, StudyTranslationStatus::Original);
    }
    let ordinary = room_by_id("times-tables").expect("existing engineered room");
    let untranslated = room_study(ordinary.as_ref(), "ja-JP").unwrap();
    assert_eq!(untranslated.content_locales, &["en"]);
    assert_eq!(untranslated.locale.resolved, "en");
    assert_eq!(
        untranslated.locale.fallback,
        Some(StudyFallback::TranslationUnavailable)
    );
    assert!(untranslated.has_depth(StudyDepth::Explanation));
    assert!(untranslated.has_depth(StudyDepth::Notes));
    assert!(!untranslated.has_depth(StudyDepth::Mathematics));
    assert_eq!(
        room_study(room.as_ref(), "ja_JP"),
        Err(StudyLocaleError::InvalidSyntax)
    );
}

#[test]
fn fresh_rooms_expose_all_existing_sources_without_visits_or_consolidation() {
    let mut ids = BTreeSet::new();
    for room in all_rooms() {
        let room_id = room.meta().id;
        let document = room_study(room.as_ref(), "en").expect("no Journey required");
        assert_eq!(document.room_id, room_id);
        let explanation = document
            .block(&format!("{room_id}.explanation"))
            .expect("existing explanation remains directly addressable");
        let expected = room
            .concept()
            .into_iter()
            .chain([room.reveal()])
            .collect::<Vec<_>>();
        let actual = explanation
            .parts
            .iter()
            .map(paragraph_text)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "source reuse for {room_id}");
        for (index, expected) in room.deep_cuts().iter().enumerate() {
            let note = document
                .block(&format!("{room_id}.note.{index}"))
                .expect("notes require no level or boon");
            assert_eq!(note.depth, StudyDepth::Notes);
            assert_eq!(only_paragraph(note), *expected);
        }
        let citation = document.block(&format!("{room_id}.citation")).unwrap();
        assert_eq!(citation.depth, StudyDepth::Notes);
        assert_eq!(only_paragraph(citation), room.citation());
        assert!(document.has_depth(StudyDepth::Explanation));
        assert!(document.has_depth(StudyDepth::Notes));
        assert_eq!(
            document.has_depth(StudyDepth::Mathematics),
            room_id == "lissajous"
        );
        for block in &document.blocks {
            assert!(
                ids.insert(block.id.clone()),
                "unique stable ID {}",
                block.id
            );
            assert!(block.id.starts_with(&format!("{room_id}.")));
            assert!(block.id.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
            }));
            assert!(!block.title.is_empty());
            assert!(!block.parts.is_empty());
        }
    }
    let pilot = room_by_id("lissajous").unwrap();
    let document = room_study(pilot.as_ref(), "ja").unwrap();
    let first_ids = document
        .blocks
        .iter()
        .take(PILOT_IDS.len())
        .map(|block| block.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(first_ids, PILOT_IDS);
    assert_eq!(
        document.block("lissajous.recurrence").unwrap().depth,
        StudyDepth::Mathematics
    );
    assert!(document.block("lissajous.missing").is_none());
}

#[derive(Debug)]
struct TextOnlyRoom;

impl RoomMetadata for TextOnlyRoom {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "study-source-fixture",
            title: "Study source fixture",
            wing: "Test",
            blurb: "A custom room with no catalog entry",
            accent: [1, 2, 3],
        }
    }
}

impl Room for TextOnlyRoom {
    fn render(&self, _: &mut dyn Surface, _: f64) {
        panic!("reading must not render or advance the room");
    }

    fn reveal(&self) -> &'static str {
        "The room owns its explanation."
    }

    fn concept(&self) -> Option<&'static str> {
        None
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "A first existing note.",
            "A second existing note.",
            "A third existing note.",
        ]
    }

    fn citation(&self) -> &'static str {
        "A custom citation overrides the catalog fallback."
    }

    fn status(&self, _: f64) -> Option<String> {
        panic!("reading must not sample dynamic room state");
    }
}

#[test]
fn custom_room_sources_are_respected_without_sampling_play_state() {
    let document = room_study(&TextOnlyRoom, "ja").unwrap();
    assert_eq!(document.blocks.len(), 5);
    assert_eq!(
        only_paragraph(document.block("study-source-fixture.explanation").unwrap()),
        TextOnlyRoom.reveal()
    );
    assert_eq!(
        only_paragraph(document.block("study-source-fixture.citation").unwrap()),
        TextOnlyRoom.citation()
    );
    assert_eq!(
        only_paragraph(document.block("study-source-fixture.note.2").unwrap()),
        TextOnlyRoom.deep_cuts()[2]
    );
    assert!(!document.has_depth(StudyDepth::Mathematics));
}

#[test]
fn translation_preserves_scientific_ids_equations_numbers_and_references() {
    let room = room_by_id("lissajous").unwrap();
    let english = room_study(room.as_ref(), "en").unwrap();
    let japanese = room_study(room.as_ref(), "ja").unwrap();
    let mut equation_count = 0;
    let mut references = BTreeSet::new();
    for id in PILOT_IDS {
        let original = english.block(id).unwrap();
        let translated = japanese.block(id).unwrap();
        assert_eq!(original.id, translated.id);
        assert_eq!(original.depth, translated.depth);
        assert_ne!(original.title, translated.title);
        assert_eq!(science(original), science(translated), "science in {id}");
        for (left, right) in original.parts.iter().zip(&translated.parts) {
            match (left, right) {
                (StudyPart::Paragraph(a), StudyPart::Paragraph(b)) => {
                    assert_eq!(a.len(), b.len());
                    for (a, b) in a.iter().zip(b) {
                        match (a, b) {
                            (StudyInline::Math(a), StudyInline::Math(b)) => {
                                assert_eq!(a, b);
                                assert_eq!(a.as_ptr(), b.as_ptr(), "one notation source");
                            }
                            (StudyInline::Text(a), StudyInline::Text(b)) => {
                                let digits = |text: &str| {
                                    text.bytes().filter(u8::is_ascii_digit).collect::<Vec<_>>()
                                };
                                assert_eq!(digits(a), digits(b), "prose digits in {id}");
                            }
                            _ => panic!("translation changed inline roles"),
                        }
                    }
                }
                (StudyPart::Equation(a), StudyPart::Equation(b)) => {
                    equation_count += 1;
                    assert_eq!(a.as_ptr(), b.as_ptr(), "one display-equation source");
                }
                (
                    StudyPart::Reference { source: a, .. },
                    StudyPart::Reference { source: b, .. },
                ) => {
                    assert!(std::ptr::eq(*a, *b), "one scientific source object");
                    assert!(a.url.starts_with("https://"));
                    references.insert(a.id);
                }
                _ => panic!("translation changed content structure"),
            }
        }
        assert_eq!(original.parts.len(), translated.parts.len());
    }
    assert_eq!(equation_count, 9);
    assert_eq!(references.len(), 8);
    assert!(inline_math(&english, "lissajous.state").contains(&"theta = 2*pi*t"));
    assert!(inline_math(&english, "lissajous.state").contains(&"x(theta) = cos(2*theta)"));
    assert!(inline_math(&english, "lissajous.measure").contains(&"dmu = du*dv/(2*pi)^2"));
    assert!(inline_math(&english, "lissajous.equations").contains(&"a/(2*pi)"));
    assert!(inline_math(&english, "lissajous.equations").contains(&"b/(2*pi)"));
}

#[test]
fn worked_numerical_examples_match_the_declared_ideal_model() {
    let room = room_by_id("lissajous").unwrap();
    let document = room_study(room.as_ref(), "en").unwrap();
    let recurrence = inline_math(&document, "lissajous.recurrence");
    // Compute distance from the initial normalized y state (0,1), directly
    // from its two coordinates rather than the displayed recurrence bound.
    for cycles in [12.0, 29.0, 70.0] {
        let phase = std::f64::consts::TAU * 2.0_f64.sqrt() * cycles;
        let position = phase.sin();
        let velocity_difference = phase.cos() - 1.0;
        let distance = position.hypot(velocity_difference);
        assert!(recurrence.contains(&format!("{distance:.6}").as_str()));
    }
    let sound = inline_math(&document, "lissajous.sound");
    let cents = 1200.0 * 1.5_f64.log2() - 700.0;
    assert!(sound.contains(&format!("{cents:.3}").as_str()));
    let step = std::f64::consts::TAU / 1500.0;
    let interpolation_bound = 8.0_f64.powi(2) * step.powi(2) / 8.0;
    assert!(interpolation_bound > 0.000140);
    assert!(interpolation_bound < 0.000141);
    assert!(inline_math(&document, "lissajous.limits").contains(&"0.000141"));
}

#[test]
fn request_defaults_and_direct_selection_never_substitute_another_depth() {
    let room = room_by_id("lissajous").unwrap();
    let defaults = StudyRequest::parse(None, None, None).unwrap();
    assert_eq!(defaults.locale().as_str(), "en");
    assert_eq!(
        defaults.selection(),
        &StudySelection::Depth(StudyDepth::Explanation)
    );
    let response = defaults.read(room.as_ref()).unwrap();
    assert_eq!(response.selection(), defaults.selection());
    assert_eq!(response.document().room_id, "lissajous");
    let selected = response
        .selected_blocks()
        .map(|block| block.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(selected, ["lissajous.try", "lissajous.intuition"]);

    let direct = StudyRequest::parse(Some("JA-jP"), None, Some("lissajous.recurrence")).unwrap();
    let response = direct.read(room.as_ref()).unwrap();
    assert_eq!(response.selected_blocks().count(), 1);
    assert_eq!(
        response.selected_blocks().next().unwrap().id,
        "lissajous.recurrence"
    );
    assert_eq!(
        response.selected_blocks().next().unwrap().locale.resolved,
        "ja"
    );
    assert_eq!(
        direct.select(room_study(room.as_ref(), "en").unwrap()),
        Err(StudyRequestError::LocaleMismatch)
    );
    assert_eq!(
        direct
            .select(room_study(room.as_ref(), "ja-jp").unwrap())
            .unwrap(),
        response
    );

    let ordinary = room_by_id("times-tables").unwrap();
    let mathematics = StudyRequest::parse(None, Some("mathematics"), None).unwrap();
    assert_eq!(
        mathematics.read(ordinary.as_ref()),
        Err(StudyRequestError::DepthUnavailable(StudyDepth::Mathematics))
    );
    assert_eq!(
        direct.read(ordinary.as_ref()),
        Err(StudyRequestError::BlockUnavailable(
            "lissajous.recurrence".to_string()
        ))
    );
    let missing = StudyRequest::parse(None, None, Some("lissajous.unknown")).unwrap();
    assert_eq!(
        missing.read(room.as_ref()),
        Err(StudyRequestError::BlockUnavailable(
            "lissajous.unknown".to_string()
        ))
    );
}

#[test]
fn request_validation_uses_shared_depth_names_and_bounds_block_identity() {
    for depth in StudyDepth::ALL {
        assert_eq!(StudyDepth::parse(depth.as_str()), Ok(depth));
        assert_eq!(depth.as_str().parse::<StudyDepth>(), Ok(depth));
    }
    for depth in ["", "math", "Notes", "explanation ", "all"] {
        assert_eq!(StudyDepth::parse(depth), Err(StudyDepthError));
        assert_eq!(
            StudyRequest::parse(None, Some(depth), None),
            Err(StudyRequestError::Depth(StudyDepthError))
        );
    }
    assert_eq!(
        StudyRequest::parse(Some(""), None, None),
        Err(StudyRequestError::Locale(StudyLocaleError::Empty))
    );
    assert_eq!(
        StudyRequest::parse(Some("bad_locale"), Some("notes"), Some("lissajous.try")),
        Err(StudyRequestError::ConflictingSelection)
    );
    for block in [
        "",
        "lissajous",
        "Lissajous.try",
        "lissajous..try",
        "lissajous.-try",
        ".lissajous.try",
        "lissajous.try-",
        "lissajous/try",
        "lissajous.説明",
        "lissajous.try\0",
    ] {
        assert_eq!(
            StudyRequest::parse(None, None, Some(block)),
            Err(StudyRequestError::InvalidBlockSyntax),
            "block {block:?}"
        );
    }
    let limit = format!("{}.try", "a".repeat(MAX_STUDY_BLOCK_ID_BYTES - 4));
    assert_eq!(limit.len(), MAX_STUDY_BLOCK_ID_BYTES);
    assert!(StudyRequest::parse(None, None, Some(&limit)).is_ok());
    assert_eq!(
        StudyRequest::parse(None, None, Some(&(limit + "a"))),
        Err(StudyRequestError::BlockTooLong)
    );
    let errors = [
        StudyRequestError::Locale(StudyLocaleError::InvalidSyntax),
        StudyRequestError::Depth(StudyDepthError),
        StudyRequestError::ConflictingSelection,
        StudyRequestError::BlockTooLong,
        StudyRequestError::InvalidBlockSyntax,
        StudyRequestError::DepthUnavailable(StudyDepth::Mathematics),
        StudyRequestError::BlockUnavailable("lissajous.missing".to_string()),
        StudyRequestError::LocaleMismatch,
    ];
    for (index, error) in errors.iter().enumerate() {
        assert!(!error.to_string().is_empty());
        let typed: &dyn std::error::Error = error;
        assert_eq!(typed.source().is_some(), index < 2);
    }
}

#[test]
fn shared_text_preserves_math_case_and_makes_partial_translation_explicit() {
    let room = room_by_id("lissajous").unwrap();
    let request = StudyRequest::parse(Some("JA-jP"), None, Some("lissajous.torus")).unwrap();
    let response = request.read(room.as_ref()).unwrap();
    let text = response.plain_text();
    assert!(text.contains("Room: lissajous"));
    assert!(text.contains("Study language: requested=ja-jp resolved=ja fallback=parent_language"));
    assert!(text.contains("Languages with content: en, ja"));
    assert!(text.contains("Depths: explanation=available, notes=available, mathematics=available"));
    assert!(text.contains("Selection: block=lissajous.torus"));
    assert!(text.contains("Available blocks: lissajous.try, lissajous.intuition"));
    for available in &response.document().blocks {
        assert!(text.contains(&available.id));
    }
    assert!(text.ends_with('\n'));
    assert!(!text.ends_with("\n\n"));
    assert!(text.contains("Block: lissajous.torus"));
    assert!(text.contains("Translation: reviewed_draft"));
    assert!(text.contains("図形はトーラスの影"));
    let block = response.selected_blocks().next().unwrap();
    for part in &block.parts {
        assert!(text.contains(&part.plain_text()));
        if let StudyPart::Equation(equation) = part {
            assert!(text.contains(equation));
            assert!(!text.contains(&equation.to_uppercase()));
        }
        if let StudyPart::Reference {
            source,
            description,
        } = part
        {
            assert!(text.contains(source.url));
            assert!(text.contains(source.title));
            assert!(text.contains(description));
        }
    }
    let notes = StudyRequest::parse(Some("ja"), Some("notes"), None)
        .unwrap()
        .read(room.as_ref())
        .unwrap()
        .plain_text();
    assert!(notes.contains("Study language: requested=ja resolved=ja fallback=none"));
    assert!(
        notes.contains("Text language: requested=ja resolved=en fallback=translation_unavailable")
    );
    assert!(notes.contains("Translation: original"));
    assert!(!notes.contains("Translation: reviewed_draft"));
    let paragraph = StudyPart::Paragraph(vec![
        StudyInline::Text("そのまま: "),
        StudyInline::Math("x(t) = cos(a*t)"),
        StudyInline::Text(" *literal punctuation*"),
    ]);
    assert_eq!(
        paragraph.plain_text(),
        "そのまま: x(t) = cos(a*t) *literal punctuation*"
    );
    assert_eq!(StudyInline::Text("そのまま").as_str(), "そのまま");
    assert_eq!(
        StudyInline::Math("x(t) = cos(a*t)").as_str(),
        "x(t) = cos(a*t)"
    );
}

#[test]
fn the_advertised_authored_rooms_are_exactly_the_rooms_that_have_the_depth() {
    // The whole point of naming rooms in a refusal is that the names are true.
    // A list maintained by hand beside a separate content check is exactly the
    // kind of pair that drifts, and the drift is only visible to a player who
    // follows the advice and is refused again.
    let advertised: BTreeSet<&str> = AUTHORED_MATHEMATICS_ROOMS.iter().copied().collect();
    assert_eq!(
        advertised.len(),
        AUTHORED_MATHEMATICS_ROOMS.len(),
        "the advertised list must not repeat a room"
    );
    let mut actual = BTreeSet::new();
    for room in all_rooms() {
        let document = room_study(room.as_ref(), "en").expect("en is a valid request");
        if document.has_depth(StudyDepth::Mathematics) {
            actual.insert(room.meta().id);
        }
    }
    assert_eq!(
        actual, advertised,
        "every advertised room must have the depth, and no unadvertised room may"
    );
}

#[test]
fn every_room_always_has_explanation_and_notes_so_neither_is_advertised() {
    // Explanation and notes are rebuilt from catalog text that every room has,
    // so listing rooms for them would name the entire catalog. This test is what
    // makes that empty slice a fact rather than an assumption.
    for room in all_rooms() {
        let document = room_study(room.as_ref(), "en").expect("en is a valid request");
        let id = room.meta().id;
        assert!(
            document.has_depth(StudyDepth::Explanation),
            "{id} has no explanation depth"
        );
        assert!(
            document.has_depth(StudyDepth::Notes),
            "{id} has no notes depth"
        );
    }
    assert!(rooms_with_authored_depth(StudyDepth::Explanation).is_empty());
    assert!(rooms_with_authored_depth(StudyDepth::Notes).is_empty());
    assert_eq!(
        rooms_with_authored_depth(StudyDepth::Mathematics),
        AUTHORED_MATHEMATICS_ROOMS
    );
}

#[test]
fn refusing_a_depth_names_where_it_is_written_and_claims_no_requirement() {
    let room = room_by_id("times-tables").expect("times-tables is in the catalog");
    let request = StudyRequest::parse(None, Some("mathematics"), None).expect("valid request");
    let error = request
        .read(room.as_ref())
        .expect_err("times-tables has no authored treatment");
    assert_eq!(
        error,
        StudyRequestError::DepthUnavailable(StudyDepth::Mathematics)
    );
    let message = error.to_string();
    assert!(
        message.contains("study depth mathematics is unavailable for this room"),
        "the refusal must stay honest about this room: {message}"
    );
    for named in AUTHORED_MATHEMATICS_ROOMS {
        assert!(
            message.contains(named),
            "the refusal must name {named}: {message}"
        );
    }
    // A player must not read the pointer as a gate they have to open.
    assert!(
        message.contains("requires nothing"),
        "the refusal must not imply a requirement: {message}"
    );
    assert!(!message.to_lowercase().contains("unlock"));
}

#[test]
fn a_named_room_actually_answers_the_depth_the_refusal_advertised() {
    // Following the refusal's own advice must work. This closes the loop the
    // drift guard opens: the names are real rooms, and they really answer.
    for named in AUTHORED_MATHEMATICS_ROOMS {
        let room = room_by_id(named).unwrap_or_else(|| panic!("{named} must be a catalog room"));
        let request = StudyRequest::parse(None, Some("mathematics"), None).expect("valid request");
        let response = request
            .read(room.as_ref())
            .unwrap_or_else(|error| panic!("{named} was advertised but refused: {error}"));
        assert!(
            response.selected_blocks().count() > 0,
            "{named} was advertised but returned no mathematics blocks"
        );
    }
}
