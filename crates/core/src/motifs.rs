//! Engine A2: motifs. Every room a phrase, not a tone.
//!
//! A motif is a room's musical identity: a key, a tempo, and a short line of
//! scale degrees that encodes the room's core idea (the walk wanders, the
//! attractor never resolves, the territories ring open fifths). Motifs render
//! through the chiptune engine, so the same phrase is the app's bed for the
//! room and structured notation over MCP. See `docs/MUSIC.md` and the July
//! review, finding 6.

use crate::chiptune::{Pattern, Voice, pitch};

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
    /// Render the phrase as a looping chiptune pattern: the room's bed.
    /// The line plays on the square lead over a root drone every bar.
    #[must_use]
    pub fn pattern(&self) -> Pattern {
        let step_seconds = 60.0 / self.tempo as f32 / 2.0; // eighth notes
        let mut steps = Vec::with_capacity(self.line.len() + self.line.len() % 8);
        for (i, &degree) in self.line.iter().enumerate() {
            if i % 8 == 0 {
                // The bar turns over: the root breathes underneath.
                steps.push(Some((self.root / 2.0, Voice::Triangle, 0.4)));
            }
            steps.push(Some((pitch(self.root, degree), Voice::Square, 0.3)));
        }
        Pattern {
            steps,
            step_seconds,
        }
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

#[cfg(test)]
mod tests {
    use super::Motif;

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
        // Six line steps plus one bar-turn drone.
        assert_eq!(pattern.steps.len(), 7);
        assert!(pattern.seconds() > 1.0, "a phrase, not a blip");
        let samples = pattern.render(22_050);
        assert!(samples.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn notation_reads_as_notes() {
        let notes = TEST_MOTIF.notation();
        assert_eq!(notes.len(), 6);
        assert_eq!(notes[0], "A3", "220 Hz is A3");
        assert_eq!(notes[3], "A4", "an octave up lands on A4");
        assert_eq!(notes[2], "E4", "seven semitones is the fifth");
    }
}
