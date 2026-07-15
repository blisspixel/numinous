//! Engine A2: motifs. Every room a phrase, not a tone.
//!
//! A motif is a room's musical identity: a key, a tempo, and a short line of
//! scale degrees that encodes the room's core idea (the walk wanders, the
//! attractor never resolves, the territories ring open fifths). Motifs render
//! through the chiptune engine, so the same phrase is the app's bed for the
//! room and structured notation over MCP. See `docs/MUSIC.md` and the July
//! review, finding 6.

use crate::chiptune::{Arrangement, ChipNote, Pattern, Step, Voice, pitch};

/// Fixed source rate for the stable stereo room bed used by every face.
pub const ROOM_BED_SOURCE_RATE: u32 = 16_000;
/// Maximum arranged events a bounded protocol projection may expose.
pub const MAX_ROOM_BED_EVENTS: usize = 96;

const CYCLE_STEPS: usize = 32;
const CYCLE_COUNT: usize = 4;
const PHRASE_STEPS: usize = CYCLE_STEPS * CYCLE_COUNT;
const FORM_OFFSETS: [usize; CYCLE_COUNT] = [0, 3, 5, 0];
const RHYTHM_FAMILIES: [&[usize]; 8] = [
    &[0, 3, 6, 8, 11, 14, 16, 19, 22, 24, 27, 30],
    &[0, 2, 5, 7, 10, 13, 16, 18, 21, 23, 26, 28, 30],
    &[0, 4, 6, 9, 12, 14, 16, 20, 22, 25, 28, 30],
    &[0, 2, 4, 7, 11, 14, 16, 18, 20, 23, 27, 30],
    &[0, 3, 5, 8, 10, 12, 15, 16, 19, 21, 24, 26, 29, 30],
    &[0, 4, 7, 10, 12, 16, 18, 22, 25, 27, 30],
    &[0, 2, 6, 8, 12, 14, 17, 20, 24, 26, 28, 30],
    &[0, 3, 7, 8, 10, 14, 16, 19, 23, 24, 26, 30],
];
const ANCHOR_FAMILIES: [&[usize]; 8] = [
    &[0, 8, 16, 24],
    &[0, 6, 12, 18, 24],
    &[0, 5, 10, 16, 21, 26],
    &[0, 8, 12, 16, 24, 28],
    &[0, 4, 10, 16, 20, 26],
    &[0, 7, 14, 16, 23],
    &[0, 6, 10, 16, 22, 26],
    &[0, 8, 14, 16, 24, 30],
];

/// A room's musical identity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motif {
    /// The key, named for minds that read ("A minor pentatonic").
    pub key: &'static str,
    /// The root frequency in Hz.
    pub root: f32,
    /// Beats per minute.
    pub tempo: u32,
    /// The phrase, as semitone offsets from the root; negative dips below.
    pub line: &'static [i32],
    /// What the phrase encodes, in one clause.
    pub encodes: &'static str,
}

impl Motif {
    /// Render the motif as a sparse sixteen-bar lead in one coherent register.
    /// Room ambience uses a soft sine or triangle voice; explicit seeded
    /// chiptune exports retain the square lead when that brighter timbre is the
    /// point.
    #[must_use]
    pub fn pattern(&self) -> Pattern {
        let step_seconds = 60.0 / self.tempo.max(1) as f32 / 2.0; // eighth notes
        let mut steps: Vec<Step> = vec![None; PHRASE_STEPS];
        let voice = self.lead_voice();
        let level = if voice == Voice::Sine { 0.095 } else { 0.11 };
        let lead_root = self.lead_root();
        for cycle in 0..CYCLE_COUNT {
            let onsets = self.rhythm_onsets_for_cycle(cycle);
            for (event, &relative_step) in onsets.iter().enumerate() {
                let degree = self.phrase_degree_for_cycle(event, onsets.len(), cycle);
                let step = cycle * CYCLE_STEPS + relative_step;
                steps[step] = Some((
                    fold_frequency(pitch(lead_root, degree), 110.0, 660.0),
                    voice,
                    level,
                ));
            }
        }
        Pattern {
            steps,
            step_seconds,
        }
    }

    /// Arrange the motif as a deterministic stereo room bed.
    ///
    /// The mathematical line remains the lead. Its own interval sequence and
    /// tempo select a deterministic four-cycle form from eight restrained
    /// rhythms, so rooms neither share one stencil nor repeat one bar sequence
    /// forever. The literal theme opens and returns around two developments.
    /// Short root and fifth anchors leave explicit gaps instead of one drone.
    #[must_use]
    pub fn arrangement(&self) -> Arrangement {
        let pattern = self.pattern();
        let mut notes = Vec::with_capacity(96);
        for cycle in 0..CYCLE_COUNT {
            let cycle_start = cycle * CYCLE_STEPS;
            let onsets = self.rhythm_onsets_for_cycle(cycle);
            for (melody_event, &relative_step) in onsets.iter().enumerate() {
                let step = cycle_start + relative_step;
                let (frequency, voice, level) = pattern.steps[step].expect("motif onset");
                let next_onset = onsets.get(melody_event + 1).copied().unwrap_or(CYCLE_STEPS);
                let desired_steps = if self.style_index() % 3 == 0 { 2 } else { 1 };
                notes.push(ChipNote {
                    frequency,
                    start_step: step,
                    step_count: desired_steps.min(next_onset - relative_step),
                    voice,
                    level,
                    pan: [-0.24, -0.08, 0.08, 0.24]
                        [(melody_event + cycle + self.style_index()) % 4],
                });
            }

            let anchor = fold_frequency(self.root, 55.0, 110.0);
            let anchor_onsets = self.anchor_onsets_for_cycle(cycle);
            for (event, &relative_step) in anchor_onsets.iter().enumerate() {
                let next_onset = anchor_onsets.get(event + 1).copied().unwrap_or(CYCLE_STEPS);
                let desired_steps = if self.tempo <= 84 { 6 } else { 4 };
                let step_count =
                    desired_steps.min(next_onset.saturating_sub(relative_step + 1).max(2));
                let is_fifth = (event + cycle) % 2 == 1;
                let start_step = cycle_start + relative_step;
                notes.push(ChipNote {
                    frequency: if is_fifth { anchor * 1.5 } else { anchor },
                    start_step,
                    step_count,
                    voice: Voice::Sine,
                    level: if is_fifth { 0.026 } else { 0.045 },
                    pan: if is_fifth { 0.1 } else { -0.1 },
                });
                if relative_step % 16 == 0 {
                    let companion_is_fifth = !is_fifth;
                    notes.push(ChipNote {
                        frequency: if companion_is_fifth {
                            anchor * 1.5
                        } else {
                            anchor
                        },
                        start_step,
                        step_count,
                        voice: Voice::Sine,
                        level: if companion_is_fifth { 0.018 } else { 0.032 },
                        pan: if companion_is_fifth { 0.1 } else { -0.1 },
                    });
                }
            }
        }

        Arrangement {
            notes,
            steps: PHRASE_STEPS,
            step_seconds: pattern.step_seconds,
        }
    }

    fn style_index(&self) -> usize {
        let signature = self.line.iter().fold(self.tempo as u64, |state, degree| {
            let encoded = u64::from(degree.unsigned_abs()) * 2 + u64::from(*degree < 0);
            state.wrapping_mul(16_777_619).wrapping_add(encoded)
        });
        (signature % RHYTHM_FAMILIES.len() as u64) as usize
    }

    fn rhythm_onsets_for_cycle(&self, cycle: usize) -> &'static [usize] {
        RHYTHM_FAMILIES
            [(self.style_index() + FORM_OFFSETS[cycle % CYCLE_COUNT]) % RHYTHM_FAMILIES.len()]
    }

    fn anchor_onsets_for_cycle(&self, cycle: usize) -> &'static [usize] {
        ANCHOR_FAMILIES
            [(self.style_index() + FORM_OFFSETS[cycle % CYCLE_COUNT]) % ANCHOR_FAMILIES.len()]
    }

    fn lead_voice(&self) -> Voice {
        if matches!(self.style_index(), 0 | 3 | 5) {
            Voice::Sine
        } else {
            Voice::Triangle
        }
    }

    fn lead_root(&self) -> f32 {
        const OCTAVE_SEARCH: [i32; 17] =
            [0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 6, -6, 7, -7, 8, -8];
        let lowest = self.line.iter().copied().min().unwrap_or(0);
        let highest = self.line.iter().copied().max().unwrap_or(0);
        for octaves in OCTAVE_SEARCH {
            let candidate = self.root * 2.0_f32.powi(octaves);
            if pitch(candidate, lowest) >= 110.0 && pitch(candidate, highest) <= 660.0 {
                return candidate;
            }
        }
        fold_frequency(self.root, 110.0, 660.0)
    }

    fn phrase_degree_for_cycle(&self, event: usize, event_count: usize, cycle: usize) -> i32 {
        if self.line.is_empty() {
            return 0;
        }
        if event + 1 == event_count {
            return self.line.last().copied().unwrap_or(0);
        }
        let len = self.line.len();
        if event < len {
            let index = match cycle % CYCLE_COUNT {
                0 | 3 => event,
                1 => (event + 1 + self.style_index()) % len,
                _ => len - 1 - event,
            };
            return self.line[index];
        }
        let variation = event - len;
        let index = match variation % 3 {
            0 => (variation * 2 + cycle + self.style_index()) % len,
            1 => len - 1 - (variation + cycle + self.style_index()) % len,
            _ => (variation + cycle + len / 2) % len,
        };
        self.line[index]
    }

    /// The phrase as note names relative to A-440 semitone math, for minds
    /// that read structure rather than hear it.
    #[must_use]
    pub fn notation(&self) -> Vec<String> {
        const NAMES: [&str; 12] = [
            "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
        ];
        let lead_root = self.lead_root();
        self.line
            .iter()
            .map(|&degree| {
                let root_semis = (12.0 * (lead_root / 440.0).log2()).round() as i64;
                let semis_from_a4 = root_semis + i64::from(degree);
                let index = semis_from_a4.rem_euclid(12) as usize;
                let octave = 4_i64 + (semis_from_a4 + 9).div_euclid(12);
                format!("{}{}", NAMES[index], octave)
            })
            .collect()
    }
}

fn fold_frequency(mut frequency: f32, low: f32, high: f32) -> f32 {
    if !frequency.is_finite() || frequency <= 0.0 {
        return low;
    }
    while frequency < low {
        frequency *= 2.0;
    }
    while frequency > high {
        frequency *= 0.5;
    }
    frequency
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        CYCLE_COUNT, CYCLE_STEPS, MAX_ROOM_BED_EVENTS, Motif, PHRASE_STEPS, ROOM_BED_SOURCE_RATE,
        pitch,
    };
    use crate::stereo_signal_metrics;

    const TEST_MOTIF: Motif = Motif {
        key: "A minor",
        root: 220.0,
        tempo: 120,
        line: &[0, 3, 7, 12, 7, 3],
        encodes: "a test phrase",
    };

    #[test]
    fn the_pattern_carries_the_phrase_and_the_breath() {
        let pattern = TEST_MOTIF.pattern();
        assert_eq!(pattern.steps.len(), PHRASE_STEPS);
        let event_count = (0..CYCLE_COUNT)
            .map(|cycle| TEST_MOTIF.rhythm_onsets_for_cycle(cycle).len())
            .sum::<usize>();
        assert_eq!(
            pattern.steps.iter().filter(|step| step.is_some()).count(),
            event_count
        );
        assert!(pattern.seconds() > 6.0, "a complete phrase, not a blip");
        let samples = pattern.render(22_050);
        assert!(samples.iter().any(|&s| s != 0.0));
        assert!(
            pattern
                .steps
                .iter()
                .flatten()
                .all(|(_, voice, _)| *voice != super::Voice::Square)
        );
        let max_step = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_step <= 0.03, "room motif step was {max_step}");
        assert_eq!(
            pattern.steps[(CYCLE_COUNT - 1) * CYCLE_STEPS
                + *TEST_MOTIF
                    .rhythm_onsets_for_cycle(CYCLE_COUNT - 1)
                    .last()
                    .expect("final onset")]
            .expect("final onset")
            .0,
            pitch(
                TEST_MOTIF.lead_root(),
                *TEST_MOTIF.line.last().expect("line")
            )
        );
    }

    #[test]
    fn phrase_schedules_the_declared_line_before_developing_it() {
        let onsets = TEST_MOTIF.rhythm_onsets_for_cycle(0);
        assert!(onsets.len() > TEST_MOTIF.line.len());
        for (event, &degree) in TEST_MOTIF.line.iter().enumerate() {
            assert_eq!(
                TEST_MOTIF.phrase_degree_for_cycle(event, onsets.len(), 0),
                degree,
                "declared degree {event}"
            );
        }
        assert_eq!(
            TEST_MOTIF.phrase_degree_for_cycle(onsets.len() - 1, onsets.len(), 0),
            *TEST_MOTIF.line.last().expect("line"),
            "the final event preserves the motif's authored cadence"
        );
    }

    #[test]
    fn notation_reads_as_notes() {
        let notes = TEST_MOTIF.notation();
        assert_eq!(notes.len(), 6);
        assert_eq!(notes[0], "A3", "220 Hz is A3");
        assert_eq!(notes[3], "A4", "an octave up lands on A4");
        assert_eq!(notes[2], "E4", "seven semitones is the fifth");
    }

    #[test]
    fn hostile_public_motif_degrees_remain_finite_and_total() {
        let hostile = Motif {
            key: "hostile",
            root: f32::NAN,
            tempo: 0,
            line: &[i32::MAX, i32::MIN],
            encodes: "a hostile public value",
        };
        let pattern = hostile.pattern();
        assert!(pattern.steps.iter().flatten().all(|(frequency, _, _)| {
            frequency.is_finite() && (110.0..=660.0).contains(frequency)
        }));
        assert!(
            pattern
                .render(8_000)
                .iter()
                .all(|sample| sample.is_finite())
        );
        assert_eq!(hostile.notation().len(), hostile.line.len());

        let empty = Motif {
            key: "empty",
            root: 220.0,
            tempo: 120,
            line: &[],
            encodes: "an absent authored line",
        };
        assert!(
            empty
                .pattern()
                .steps
                .iter()
                .flatten()
                .all(|(frequency, _, _)| {
                    frequency.is_finite() && (110.0..=660.0).contains(frequency)
                })
        );
        assert_eq!(super::fold_frequency(1.0, 110.0, 660.0), 128.0);
        assert_eq!(super::fold_frequency(1_000.0, 110.0, 660.0), 500.0);
    }

    #[test]
    fn arrangement_has_a_quiet_anchor_real_stereo_and_clean_seam() {
        let arrangement = TEST_MOTIF.arrangement();
        assert!(arrangement.notes.iter().any(|note| {
            note.voice == super::Voice::Sine && (2..=8).contains(&note.step_count)
        }));
        let samples = arrangement.render_stereo(48_000);
        assert!(samples.iter().all(|sample| (-1.0..=1.0).contains(sample)));
        let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak < 0.45, "room bed peak was {peak}");
        assert_eq!(&samples[..2], &[0.0, 0.0]);
        assert_eq!(&samples[samples.len() - 2..], &[0.0, 0.0]);
        assert!(samples.chunks_exact(2).any(|frame| frame[0] != frame[1]));

        for channel in 0..2 {
            let mean = samples
                .iter()
                .skip(channel)
                .step_by(2)
                .copied()
                .sum::<f32>()
                / (samples.len() / 2) as f32;
            assert!(mean.abs() < 0.002, "channel {channel} DC was {mean}");
        }
    }

    #[test]
    fn arrangement_is_deterministic_at_common_device_rates() {
        let arrangement = TEST_MOTIF.arrangement();
        for rate in [44_100, 48_000, 96_000, 192_000] {
            let first = arrangement.render_stereo(rate);
            let second = arrangement.render_stereo(rate);
            assert_eq!(first, second);
            assert_eq!(
                first.len(),
                (arrangement.seconds() * rate as f32) as usize * 2
            );
        }
    }

    #[test]
    fn every_room_bed_is_sparse_low_level_centered_and_seam_safe() {
        let mut event_signatures = HashSet::new();
        for room in crate::all_rooms() {
            let meta = room.meta();
            let motif = room.motif().expect("catalog motif");
            let pattern = motif.pattern();
            assert_eq!(pattern.steps.len(), PHRASE_STEPS, "{} length", meta.id);
            assert!(
                pattern.steps.iter().filter(|step| step.is_none()).count() >= 16,
                "{} needs breathing room",
                meta.id
            );

            let arrangement = motif.arrangement();
            assert!(
                arrangement.notes.len() <= MAX_ROOM_BED_EVENTS,
                "{} exceeds the structured event limit",
                meta.id
            );
            let signature = arrangement
                .notes
                .iter()
                .map(|note| {
                    (
                        note.frequency.to_bits(),
                        note.start_step,
                        note.step_count,
                        note.voice.id(),
                        note.level.to_bits(),
                        note.pan.to_bits(),
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                event_signatures.insert(signature),
                "{} duplicates another room's complete event signature",
                meta.id
            );

            let samples = arrangement.render_stereo(ROOM_BED_SOURCE_RATE);
            assert!(!samples.is_empty(), "{} must make sound", meta.id);
            assert!(samples.len() <= 2_000_000, "{} source allocation", meta.id);
            assert_eq!(&samples[..2], &[0.0, 0.0], "{} attack seam", meta.id);
            assert_eq!(
                &samples[samples.len() - 2..],
                &[0.0, 0.0],
                "{} release seam",
                meta.id
            );
            let metrics = stereo_signal_metrics(&samples);
            assert_eq!(metrics.trailing_samples, 0, "{} stereo frames", meta.id);
            assert_eq!(metrics.non_finite_samples, 0, "{} finite signal", meta.id);
            assert_eq!(metrics.subnormal_samples, 0, "{} subnormal signal", meta.id);
            assert_eq!(metrics.clipped_samples, 0, "{} clipped signal", meta.id);
            assert!(metrics.peak < 0.45, "{} mix headroom", meta.id);
            assert!(
                (0.005..0.12).contains(&metrics.rms),
                "{} room-bed RMS was {}",
                meta.id,
                metrics.rms
            );
            assert!(
                metrics.left_rms > 0.0 && metrics.right_rms > 0.0,
                "{} needs energy in both channels",
                meta.id
            );
            assert!(metrics.left_dc.abs() < 0.003, "{} left DC", meta.id);
            assert!(metrics.right_dc.abs() < 0.003, "{} right DC", meta.id);
            assert!(metrics.max_step < 0.09, "{} sample step", meta.id);
            assert!(
                metrics.channel_balance_db.abs() < 3.0,
                "{} channel balance",
                meta.id
            );
            assert!(
                samples.chunks_exact(2).any(|frame| frame[0] != frame[1]),
                "{} must retain a stereo field",
                meta.id
            );
        }
        assert_eq!(event_signatures.len(), crate::all_rooms().len());
    }

    #[test]
    fn catalog_beds_vary_their_phrase_shapes_and_leave_anchor_breaths() {
        let mut phrase_shapes = HashSet::new();
        let mut anchor_shapes = HashSet::new();

        for room in crate::all_rooms() {
            let meta = room.meta();
            let motif = room.motif().expect("catalog motif");
            let pattern = motif.pattern();
            let cycle_shapes = pattern
                .steps
                .chunks(32)
                .map(|cycle| {
                    cycle
                        .iter()
                        .enumerate()
                        .filter_map(|(step, event)| event.map(|_| step))
                        .collect::<Vec<_>>()
                })
                .collect::<HashSet<_>>();
            assert!(
                pattern.steps.len() >= 96 && cycle_shapes.len() >= 3,
                "{} needs at least three phrase forms in one stable bed",
                meta.id
            );
            let onsets = motif.rhythm_onsets_for_cycle(0);
            assert!(
                onsets.len() > motif.line.len(),
                "{} needs room for its literal line and a cadence",
                meta.id
            );
            for (event, &degree) in motif.line.iter().enumerate() {
                assert_eq!(
                    motif.phrase_degree_for_cycle(event, onsets.len(), 0),
                    degree,
                    "{} declared degree {event}",
                    meta.id
                );
                let actual = pattern.steps[onsets[event]].expect("declared degree").0;
                let expected = pitch(motif.lead_root(), degree);
                assert!(
                    (actual - expected).abs() < 0.001,
                    "{} changed the interval at declared degree {event}",
                    meta.id
                );
                assert!(
                    (110.0..=660.0).contains(&actual),
                    "{} declared degree {event} left the lead register",
                    meta.id
                );
            }
            assert_eq!(
                motif.phrase_degree_for_cycle(onsets.len() - 1, onsets.len(), 0),
                *motif.line.last().expect("catalog line"),
                "{} cadence",
                meta.id
            );
            phrase_shapes.insert(
                pattern
                    .steps
                    .iter()
                    .enumerate()
                    .filter_map(|(step, event)| event.map(|_| step))
                    .collect::<Vec<_>>(),
            );

            let arrangement = motif.arrangement();
            let mut distinct_anchors = HashSet::new();
            for note in arrangement
                .notes
                .iter()
                .filter(|note| note.step_count > 1 && note.level <= 0.045)
            {
                assert!(
                    distinct_anchors.insert((note.start_step, note.frequency.to_bits())),
                    "{} stacks a duplicate anchor at step {}",
                    meta.id,
                    note.start_step
                );
            }
            let anchors = arrangement
                .notes
                .iter()
                .filter(|note| note.step_count > 1 && note.level <= 0.045)
                .map(|note| (note.start_step, note.step_count))
                .collect::<Vec<_>>();
            assert!(
                anchors.iter().all(|(_, steps)| *steps <= 8),
                "{} holds an anchor for more than one bar",
                meta.id
            );
            assert!(
                arrangement.notes.iter().all(|note| note.level <= 0.13),
                "{} exceeds the room-bed voice ceiling",
                meta.id
            );
            anchor_shapes.insert(anchors);
        }

        assert!(
            phrase_shapes.len() >= 6,
            "the catalog needs at least six phrase shapes, found {}",
            phrase_shapes.len()
        );
        assert!(
            anchor_shapes.len() >= 4,
            "the catalog needs at least four accompaniment shapes, found {}",
            anchor_shapes.len()
        );
    }
}
