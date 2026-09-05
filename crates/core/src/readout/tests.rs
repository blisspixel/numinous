use std::collections::BTreeMap;

use super::{
    DisplayNumber, NumericReadout, PARAMETER_SAMPLES, ReadoutId, ReadoutLookup, find_readout,
    status_numbers,
};
use crate::challenge::{grade_parameter, pose_parameter_goal};
use crate::predict::{
    PredictionCurveError, grade_prediction, grade_prediction_curve, pose_prediction,
    prediction_rate_window,
};
use crate::{Room, RoomMeta, RoomMetadata, Surface};

fn channel(id: usize, label: &'static str, value: f64) -> NumericReadout {
    NumericReadout::new(ReadoutId::new(id), label, value).expect("finite test channel")
}

#[test]
fn finite_measurements_reject_nonfinite_inputs() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(NumericReadout::new(ReadoutId::new(8), "VALUE", value).is_none());
        assert!(DisplayNumber::fixed(value, 2).is_none());
    }
    for value in [0.0, -0.0, f64::from_bits(1), f64::MAX, -f64::MAX] {
        let readout = channel(8, "VALUE", value);
        assert_eq!(readout.id().get(), 8);
        assert_eq!(readout.label(), "VALUE");
        assert_eq!(readout.value().to_bits(), value.to_bits());
    }
}

#[test]
fn decimal_quantization_preserves_display_ties_and_signed_zero() {
    for (value, digits, expected) in [
        (2.5, 0, "2"),
        (3.5, 0, "4"),
        (-2.5, 0, "-2"),
        (1.125, 2, "1.12"),
        (1.375, 2, "1.38"),
        (2.675, 2, "2.67"),
        (1.125_f64.next_down(), 2, "1.12"),
        (1.125_f64.next_up(), 2, "1.13"),
        (-0.0, 2, "-0.00"),
        (-0.0001, 2, "-0.00"),
        (0.0001, 4, "0.0001"),
    ] {
        let display = DisplayNumber::fixed(value, digits).expect("finite display");
        assert_eq!(display.to_string(), expected);
        let expected = expected.parse::<f64>().expect("numeric fixture");
        assert_eq!(display.value().to_bits(), expected.to_bits());
        assert_eq!(
            display
                .readout(ReadoutId::new(3), "VALUE")
                .value()
                .to_bits(),
            expected.to_bits()
        );
    }
}

#[test]
fn decimal_tokens_round_trip_at_extreme_finite_values_and_precision() {
    for value in [
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
    ] {
        for digits in [0, 2, 15, u8::MAX] {
            let display = DisplayNumber::fixed(value, digits).expect("finite fixed decimal");
            let text = display.to_string();
            assert!(text.len() <= 566, "bounded f64 magnitude and u8 precision");
            let parsed = text.parse::<f64>().expect("numeric display token");
            assert!(parsed.is_finite());
            assert_eq!(display.value().to_bits(), parsed.to_bits());
            assert_eq!(parsed.is_sign_negative(), value.is_sign_negative());
        }
    }
}

type Measurements = fn(f64) -> Option<Vec<NumericReadout>>;

struct TestRoom {
    measurements: Measurements,
    status: fn(f64) -> Option<String>,
}

impl RoomMetadata for TestRoom {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "readout-test",
            title: "Readout Test",
            wing: "Tests",
            blurb: "A bounded numeric contract fixture.",
            accent: [0, 0, 0],
        }
    }
}

impl Room for TestRoom {
    fn render(&self, _: &mut dyn Surface, _: f64) {}

    fn reveal(&self) -> &'static str {
        "A numeric contract fixture."
    }

    fn status(&self, phase: f64) -> Option<String> {
        (self.status)(phase)
    }

    fn numeric_readouts(&self, phase: f64) -> Option<Vec<NumericReadout>> {
        (self.measurements)(phase)
    }
}

fn no_status(_: f64) -> Option<String> {
    panic!("typed study operations must not parse status text")
}

#[test]
fn lookup_preserves_a_negative_zero_measurement_at_negative_zero_phase() {
    let room = TestRoom {
        measurements: |phase| Some(vec![channel(2, "VALUE", phase)]),
        status: no_status,
    };
    let lookup = ReadoutLookup::new(&room).expect("typed lookup");
    assert_eq!(lookup.value(2, 0.0).unwrap().to_bits(), 0.0_f64.to_bits());
    assert_eq!(
        lookup.value(2, -0.0).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
}

fn linear_channels(phase: f64) -> Option<Vec<NumericReadout>> {
    Some(vec![
        channel(0, "TUNING", 8.0),
        channel(2, "VALUE", 1.0 + 4.0 * phase),
        channel(9, "LATER", -phase),
    ])
}

fn translated_channels(phase: f64) -> Option<Vec<NumericReadout>> {
    let mut channels = vec![
        channel(9, "otro", -phase),
        channel(
            2,
            if phase < 0.5 { "valeur 2" } else { "Wert" },
            1.0 + 4.0 * phase,
        ),
        channel(0, "reglage", 8.0),
    ];
    if phase > 0.25 {
        channels.reverse();
    }
    Some(channels)
}

#[test]
fn numeric_identity_survives_reordered_channels_and_translated_labels_in_every_grader() {
    let original = TestRoom {
        measurements: linear_channels,
        status: no_status,
    };
    let translated = TestRoom {
        measurements: translated_channels,
        status: no_status,
    };
    let found = find_readout(&translated).expect("a moving stable ID");
    assert_eq!(found.index, 2);
    assert_eq!(found.label, "valeur 2");
    assert_eq!(found.span, (1.0, 4.9375));

    let original_goal = pose_parameter_goal(&original, 17).expect("goal");
    let translated_goal = pose_parameter_goal(&translated, 17).expect("translated goal");
    assert_eq!(original_goal.index, translated_goal.index);
    assert_eq!(original_goal.target, translated_goal.target);
    assert_eq!(original_goal.tolerance, translated_goal.tolerance);
    assert_eq!(original_goal.span, translated_goal.span);
    for phase in [0.0, 0.25, 0.5, 1.0] {
        assert_eq!(
            grade_parameter(&original, &original_goal, phase),
            grade_parameter(&translated, &translated_goal, phase)
        );
    }

    let original_prediction = pose_prediction(&original, 17).expect("prediction");
    let translated_prediction = pose_prediction(&translated, 17).expect("translated prediction");
    assert_eq!(original_prediction.phase, translated_prediction.phase);
    assert_eq!(original_prediction.index, translated_prediction.index);
    let truth = 1.0 + 4.0 * original_prediction.phase;
    assert_eq!(
        grade_prediction(&original, &original_prediction, truth),
        grade_prediction(&translated, &translated_prediction, truth)
    );
    let curve = grade_prediction_curve(&translated, &translated_prediction, truth, 4.0)
        .expect("typed curve");
    assert_eq!(
        Ok(curve.clone()),
        grade_prediction_curve(&original, &original_prediction, truth, 4.0)
    );
    assert!((curve.actual_rate - 4.0).abs() < 1e-12);
    assert!(
        curve
            .samples
            .iter()
            .all(|sample| sample.residual.abs() < 1e-12)
    );
}

fn missing_at_half(phase: f64) -> Option<Vec<NumericReadout>> {
    Some(if phase == 0.5 {
        vec![]
    } else {
        vec![channel(2, "VALUE", phase)]
    })
}

fn duplicate_at_half(phase: f64) -> Option<Vec<NumericReadout>> {
    let mut channels = vec![channel(2, "VALUE", phase)];
    if phase == 0.5 {
        channels.push(channel(2, "OTHER", -phase));
    }
    Some(channels)
}

fn mixed_typed_origin(phase: f64) -> Option<Vec<NumericReadout>> {
    (phase != 0.5).then(|| vec![channel(2, "VALUE", phase)])
}

fn mixed_legacy_origin(phase: f64) -> Option<Vec<NumericReadout>> {
    (phase != 0.0).then(|| vec![channel(2, "VALUE", phase)])
}

fn legacy_status(phase: f64) -> Option<String> {
    Some(format!("VALUE={phase:.2} TUNING=8 OTHER={phase:.2}"))
}

struct OtherRoom<'a>(&'a dyn Room);

impl RoomMetadata for OtherRoom<'_> {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "different-readout-room",
            ..self.0.meta()
        }
    }
}

impl Room for OtherRoom<'_> {
    fn render(&self, surface: &mut dyn Surface, phase: f64) {
        self.0.render(surface, phase);
    }

    fn reveal(&self) -> &'static str {
        self.0.reveal()
    }

    fn status(&self, phase: f64) -> Option<String> {
        self.0.status(phase)
    }

    fn numeric_readouts(&self, phase: f64) -> Option<Vec<NumericReadout>> {
        self.0.numeric_readouts(phase)
    }
}

#[test]
fn every_grader_rejects_another_room_even_with_identical_numeric_channels() {
    for room in [
        TestRoom {
            measurements: linear_channels,
            status: no_status,
        },
        TestRoom {
            measurements: |_| None,
            status: legacy_status,
        },
    ] {
        let other = OtherRoom(&room);
        let goal = pose_parameter_goal(&room, 7).expect("source goal");
        let prediction = pose_prediction(&room, 7).expect("source prediction");
        let source_value = ReadoutLookup::new(&room)
            .unwrap()
            .value(goal.index, prediction.phase);
        let other_value = ReadoutLookup::new(&other)
            .unwrap()
            .value(goal.index, prediction.phase);
        assert!(source_value.is_some());
        assert_eq!(source_value, other_value, "only the room identity differs");
        assert!(grade_parameter(&room, &goal, prediction.phase).is_some());
        assert!(grade_prediction(&room, &prediction, 0.0).is_some());
        assert!(grade_prediction_curve(&room, &prediction, 0.0, 1.0).is_ok());
        assert!(grade_parameter(&other, &goal, prediction.phase).is_none());
        assert!(grade_prediction(&other, &prediction, 0.0).is_none());
        assert_eq!(
            grade_prediction_curve(&other, &prediction, 0.0, 1.0),
            Err(PredictionCurveError::ReadoutUnavailable)
        );
    }
}

#[test]
fn missing_duplicate_and_mixed_typed_providers_fail_closed() {
    let valid = TestRoom {
        measurements: linear_channels,
        status: no_status,
    };
    let goal = pose_parameter_goal(&valid, 7).expect("valid goal");
    let mut prediction = pose_prediction(&valid, 7).expect("valid prediction");
    prediction.phase = 0.5;
    let invalid_providers: [Measurements; 7] = [
        missing_at_half,
        duplicate_at_half,
        mixed_typed_origin,
        mixed_legacy_origin,
        |_| Some(vec![]),
        |phase| Some(vec![channel(2, "A", phase), channel(2, "B", phase)]),
        |phase| {
            Some(if phase == 0.0 {
                vec![]
            } else {
                vec![channel(2, "VALUE", phase)]
            })
        },
    ];
    for measurements in invalid_providers {
        let room = TestRoom {
            measurements,
            status: legacy_status,
        };
        assert!(find_readout(&room).is_none());
        assert!(grade_parameter(&room, &goal, 0.5).is_none());
        assert!(grade_prediction(&room, &prediction, 0.5).is_none());
        assert_eq!(
            grade_prediction_curve(&room, &prediction, 0.5, 1.0),
            Err(PredictionCurveError::ReadoutUnavailable)
        );
    }
}

#[test]
fn prediction_refuses_a_channel_unavailable_at_its_rounded_center() {
    let available = TestRoom {
        measurements: linear_channels,
        status: no_status,
    };
    let seed = 1;
    let prediction = pose_prediction(&available, seed).expect("available prediction");
    assert_eq!(prediction.phase, 0.016);
    let missing_center = TestRoom {
        measurements: |phase| {
            if phase == 0.016 {
                Some(vec![])
            } else {
                linear_channels(phase)
            }
        },
        status: no_status,
    };
    assert!(find_readout(&missing_center).is_some());
    assert!(pose_parameter_goal(&missing_center, seed).is_some());
    assert!(grade_prediction(&missing_center, &prediction, 0.0).is_none());
    assert!(pose_prediction(&missing_center, seed).is_none());
}

#[test]
fn curve_lookup_rechecks_a_channel_missing_between_discovery_samples() {
    let room = TestRoom {
        measurements: |phase| {
            Some(if phase == 0.516 {
                vec![]
            } else {
                vec![channel(2, "VALUE", phase)]
            })
        },
        status: no_status,
    };
    let mut prediction = pose_prediction(&room, 7).expect("finite grid admits the channel");
    prediction.phase = 0.5;
    assert!(grade_prediction(&room, &prediction, 0.5).is_some());
    assert_eq!(
        grade_prediction_curve(&room, &prediction, 0.5, 1.0),
        Err(PredictionCurveError::ReadoutUnavailable)
    );
}

#[test]
fn typed_discovery_does_not_pose_constant_or_overflowing_spans() {
    for measurements in [
        (|_| Some(vec![channel(2, "CONSTANT", 1.0)])) as Measurements,
        |phase| {
            Some(vec![channel(
                2,
                "EXTREME",
                if phase < 0.5 { -f64::MAX } else { f64::MAX },
            )])
        },
    ] {
        let room = TestRoom {
            measurements,
            status: no_status,
        };
        assert!(pose_parameter_goal(&room, 7).is_none());
        assert!(pose_prediction(&room, 7).is_none());
    }
}

#[test]
fn boundary_validation_rejects_an_invalid_internal_measurement() {
    let room = TestRoom {
        measurements: |_| {
            Some(vec![NumericReadout {
                id: ReadoutId::new(2),
                label: "INVALID",
                value: f64::NAN,
            }])
        },
        status: no_status,
    };
    assert!(find_readout(&room).is_none());
    assert!(ReadoutLookup::new(&room).is_none());
}

#[test]
fn status_only_rooms_retain_numeric_column_and_label_stability_rules() {
    let legacy = TestRoom {
        measurements: |_| None,
        status: legacy_status,
    };
    let selected = find_readout(&legacy).expect("legacy moving column");
    assert_eq!(selected.index, 0);
    assert_eq!(selected.label, "VALUE");
    assert_eq!(selected.span, (0.0, 0.98));
    let unstable = TestRoom {
        measurements: |_| None,
        status: |phase| {
            Some(format!(
                "{}={phase:.2}",
                if phase < 0.5 { "FIRST" } else { "SECOND" }
            ))
        },
    };
    assert!(pose_parameter_goal(&unstable, 7).is_none());
    assert!(pose_prediction(&unstable, 7).is_none());
}

/// Cache the actual status lines once so compatibility posing does not replay
/// the Gray-Scott field for each seed, point guess, and curve comparison.
struct StatusSnapshot {
    meta: RoomMeta,
    statuses: BTreeMap<u64, String>,
}

impl RoomMetadata for StatusSnapshot {
    fn meta(&self) -> RoomMeta {
        self.meta
    }
}

impl Room for StatusSnapshot {
    fn render(&self, _: &mut dyn Surface, _: f64) {}
    fn reveal(&self) -> &'static str {
        "Status compatibility fixture."
    }
    fn status(&self, phase: f64) -> Option<String> {
        self.statuses.get(&phase.to_bits()).cloned()
    }
}

struct Migration {
    id: &'static str,
    index: usize,
    label: &'static str,
    span: (f64, f64),
    seed_one_actual: f64,
}

// Canonical variation-zero values recorded from the pre-migration instrument.
// The last field is its answer to prediction seed one, not a variation seed.
const MIGRATED: [Migration; 6] = [
    Migration {
        id: "times-tables",
        index: 0,
        label: "K",
        span: (2.0, 9.88),
        seed_one_actual: 6.5,
    },
    Migration {
        id: "lissajous",
        index: 1,
        label: "X:Y",
        span: (2.0, 4.95),
        seed_one_actual: 2.33,
    },
    Migration {
        id: "gray-scott",
        index: 2,
        label: "ELAPSED TIME",
        span: (0.0, 118.0),
        seed_one_actual: 20.0,
    },
    Migration {
        id: "standing-wave",
        index: 1,
        label: "PHASE PERCENT",
        span: (0.0, 98.0),
        seed_one_actual: 38.0,
    },
    Migration {
        id: "bayes-update",
        index: 1,
        label: "LIKELIHOOD RATIO",
        span: (0.4, 3.8),
        seed_one_actual: 2.7,
    },
    Migration {
        id: "smith-chart",
        index: 1,
        label: "RESISTANCE",
        span: (0.23, 4.41),
        seed_one_actual: 0.28,
    },
];

#[test]
fn migrated_rooms_keep_their_numeric_ids_spans_seeds_and_grades() {
    for Migration {
        id,
        index,
        label,
        span,
        seed_one_actual,
    } in MIGRATED
    {
        let room = crate::room_by_id(id).expect("catalog room");
        let readout = find_readout(room.as_ref()).expect("typed moving readout");
        assert_eq!(
            (readout.index, readout.label.as_str(), readout.span),
            (index, label, span),
            "{id}"
        );
        let seeds = [0, 1, 7, u64::MAX];
        let mut phases: Vec<f64> = (0..PARAMETER_SAMPLES)
            .map(|i| i as f64 / PARAMETER_SAMPLES as f64)
            .collect();
        phases.push(1.0);
        for seed in seeds {
            let prediction = pose_prediction(room.as_ref(), seed).expect("typed prediction");
            phases.extend(prediction_rate_window(&prediction));
        }
        let mut snapshot = StatusSnapshot {
            meta: room.meta(),
            statuses: BTreeMap::new(),
        };
        for phase in phases {
            snapshot
                .statuses
                .entry(phase.to_bits())
                .or_insert_with(|| room.status(phase).expect("existing status"));
        }
        let legacy_readout = find_readout(&snapshot).expect("legacy moving readout");
        assert_eq!(readout.index, legacy_readout.index, "{id}");
        assert_eq!(readout.span, legacy_readout.span, "{id}");
        assert_eq!(readout.samples, legacy_readout.samples, "{id}");

        for seed in seeds {
            let goal = pose_parameter_goal(room.as_ref(), seed).expect("typed goal");
            let old_goal = pose_parameter_goal(&snapshot, seed).expect("legacy goal");
            assert_eq!(
                (goal.index, goal.target, goal.tolerance, goal.span),
                (
                    old_goal.index,
                    old_goal.target,
                    old_goal.tolerance,
                    old_goal.span
                ),
                "{id} seed {seed}"
            );
            for phase in [0.0, 0.125, 0.375, 0.5, 0.875, 0.984375, 1.0] {
                assert_eq!(
                    grade_parameter(room.as_ref(), &goal, phase),
                    grade_parameter(&snapshot, &old_goal, phase),
                    "{id} phase {phase}"
                );
            }
            let prediction = pose_prediction(room.as_ref(), seed).expect("typed prediction");
            let old_prediction = pose_prediction(&snapshot, seed).expect("legacy prediction");
            assert_eq!(
                (prediction.index, prediction.phase, prediction.span),
                (
                    old_prediction.index,
                    old_prediction.phase,
                    old_prediction.span
                ),
                "{id} seed {seed}"
            );
            if seed == 1 {
                let grade =
                    grade_prediction(room.as_ref(), &prediction, 0.0).expect("finite grade");
                assert_eq!(
                    grade.actual, seed_one_actual,
                    "{id} preserves the recorded phase/answer pair"
                );
            }
            for guess in [span.0, span.1, (span.0 + span.1) * 0.5] {
                assert_eq!(
                    grade_prediction(room.as_ref(), &prediction, guess),
                    grade_prediction(&snapshot, &old_prediction, guess),
                    "{id} seed {seed}"
                );
                assert_eq!(
                    grade_prediction_curve(room.as_ref(), &prediction, guess, 2.0),
                    grade_prediction_curve(&snapshot, &old_prediction, guess, 2.0),
                    "{id} seed {seed}"
                );
            }
        }
    }
}

#[test]
fn migrated_values_match_visible_numbers_for_variations_and_hostile_phases() {
    for Migration { id, index, .. } in MIGRATED {
        for variation in [1, u64::MAX] {
            let room = crate::room_by_id_with(id, variation).expect("variation");
            for phase in [
                0.005,
                0.125,
                0.375,
                0.999,
                1.0,
                -f64::MAX,
                f64::MAX,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                let channels = room
                    .numeric_readouts(phase)
                    .expect("typed provider remains supported");
                assert_eq!(
                    channels.len(),
                    1,
                    "this slice exposes only the previously selected quantity"
                );
                let readout = channels[0];
                let status = room.status(phase).expect("status");
                let expected = status_numbers(&status)[index].1;
                assert_eq!(readout.id().get(), index, "{id}");
                assert_eq!(
                    readout.value().to_bits(),
                    expected.to_bits(),
                    "{id} variation {variation} phase {phase}: {status}"
                );
            }
        }
    }
}

#[test]
fn standing_wave_keeps_half_percent_integer_rounding() {
    let room = crate::room_by_id("standing-wave").expect("standing wave");
    let channels = room.numeric_readouts(0.005).expect("typed measurements");
    assert_eq!(channels[0].value(), 1.0);
    assert!(room.status(0.005).expect("status").contains("phase=1%"));
}

#[test]
fn smith_chart_keeps_the_instruments_resistance_clip() {
    // This load's near-real positive reflection coefficient gives a raw
    // normalized resistance above 9.9 near phase 49/64. The existing compact
    // instrument clips it, and studies must continue to grade that shown value.
    let room = crate::room_by_id_with("smith-chart", 35).expect("Smith chart variation");
    let phase = 49.0 / 64.0;
    let channels = room.numeric_readouts(phase).expect("typed measurements");
    assert_eq!(channels[0].id().get(), 1);
    assert_eq!(channels[0].value(), 9.9);
    assert!(room.status(phase).expect("status").contains("z=9.90"));
}
