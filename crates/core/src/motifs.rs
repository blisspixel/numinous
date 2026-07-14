//! Engine A2: motifs. Every room a phrase, not a tone.
//!
//! A motif is a room's musical identity: a key, a tempo, and a short line of
//! scale degrees that encodes the room's core idea (the walk wanders, the
//! attractor never resolves, the territories ring open fifths). Motifs render
//! through the chiptune engine, so the same phrase is the app's bed for the
//! room and structured notation over MCP. See `docs/MUSIC.md` and the July
//! review, finding 6.

use crate::chiptune::{Arrangement, ChipNote, Pattern, Step, Voice, pitch};

const PHRASE_STEPS: usize = 32;
const MELODY_ONSETS: [usize; 15] = [0, 2, 4, 7, 8, 10, 13, 15, 16, 18, 21, 23, 24, 27, 30];

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
    /// Render the motif as a sparse four-bar lead with a final resolution.
    /// Room ambience uses the softer triangle voice; explicit seeded chiptune
    /// exports retain the square lead when that brighter timbre is the point.
    #[must_use]
    pub fn pattern(&self) -> Pattern {
        let step_seconds = 60.0 / self.tempo.max(1) as f32 / 2.0; // eighth notes
        let mut steps: Vec<Step> = vec![None; PHRASE_STEPS];
        for (event, &step) in MELODY_ONSETS.iter().enumerate() {
            let degree = self.phrase_degree(event);
            steps[step] = Some((
                fold_frequency(pitch(self.root, degree), 110.0, 660.0),
                Voice::Triangle,
                0.16,
            ));
        }
        Pattern {
            steps,
            step_seconds,
        }
    }

    /// Arrange the motif as a deterministic stereo room bed.
    ///
    /// The mathematical line remains the lead, with rests and a small formal
    /// variation across four bars. Quiet root and fifth drones provide a stable
    /// consonant reference without competing with room interaction sounds.
    #[must_use]
    pub fn arrangement(&self) -> Arrangement {
        let pattern = self.pattern();
        let mut notes = Vec::with_capacity(MELODY_ONSETS.len() + 6);
        for (step, event) in pattern.steps.iter().enumerate() {
            if let Some((frequency, voice, level)) = event {
                notes.push(ChipNote {
                    frequency: *frequency,
                    start_step: step,
                    step_count: 1,
                    voice: *voice,
                    level: *level,
                    pan: if step % 4 == 0 { -0.22 } else { 0.22 },
                });
            }
        }

        let anchor = fold_frequency(self.root, 55.0, 110.0);
        for start_step in [0, 16] {
            notes.push(ChipNote {
                frequency: anchor,
                start_step,
                step_count: 16,
                voice: Voice::Sine,
                level: 0.075,
                pan: -0.12,
            });
            notes.push(ChipNote {
                frequency: anchor * 1.5,
                start_step,
                step_count: 16,
                voice: Voice::Sine,
                level: 0.04,
                pan: 0.12,
            });
        }

        Arrangement {
            notes,
            steps: PHRASE_STEPS,
            step_seconds: pattern.step_seconds,
        }
    }

    fn phrase_degree(&self, event: usize) -> i32 {
        if event + 1 == MELODY_ONSETS.len() || self.line.is_empty() {
            return 0;
        }
        let len = self.line.len();
        let index = match event {
            0..=6 => event % len,
            7..=10 => (event + 2) % len,
            _ => len - 1 - event % len,
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
        self.line
            .iter()
            .map(|&degree| {
                let semis_from_a4 = (12.0 * (self.root / 440.0).log2()).round() as i32 + degree;
                let index = semis_from_a4.rem_euclid(12) as usize;
                let octave = 4 + (semis_from_a4 + 9).div_euclid(12);
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
    use super::{MELODY_ONSETS, Motif, PHRASE_STEPS};

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
        assert_eq!(
            pattern.steps.iter().filter(|step| step.is_some()).count(),
            MELODY_ONSETS.len()
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
            pattern.steps[MELODY_ONSETS[MELODY_ONSETS.len() - 1]]
                .expect("final onset")
                .0,
            TEST_MOTIF.root
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
    fn arrangement_has_a_quiet_anchor_real_stereo_and_clean_seam() {
        let arrangement = TEST_MOTIF.arrangement();
        assert!(
            arrangement
                .notes
                .iter()
                .any(|note| note.voice == super::Voice::Sine && note.step_count == 16)
        );
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

            let samples = motif.arrangement().render_stereo(8_000);
            assert!(!samples.is_empty(), "{} must make sound", meta.id);
            assert_eq!(&samples[..2], &[0.0, 0.0], "{} attack seam", meta.id);
            assert_eq!(
                &samples[samples.len() - 2..],
                &[0.0, 0.0],
                "{} release seam",
                meta.id
            );
            assert!(
                samples
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() < 0.45),
                "{} must retain quiet mix headroom",
                meta.id
            );
            for channel in 0..2 {
                let mean = samples
                    .iter()
                    .skip(channel)
                    .step_by(2)
                    .copied()
                    .sum::<f32>()
                    / (samples.len() / 2) as f32;
                assert!(mean.abs() < 0.003, "{} channel {channel} DC", meta.id);
            }
        }
    }
}
