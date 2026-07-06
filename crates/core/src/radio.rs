//! Engine B: the radio. Stations, like a car stereo for a mathematical world.
//!
//! Each station is a name and a carefully written brief for a music
//! generation model (ElevenLabs Music, or any successor). The core owns only
//! the pure part, what the stations are and what they ask for, so the briefs
//! are versioned, testable, and identical everywhere. Fetching and playback
//! are the faces' business. See `docs/MUSIC.md`.

/// One radio station: a dial position in the world's soundtrack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Station {
    /// The dial name, lowercase, stable (used for cache filenames).
    pub id: &'static str,
    /// How the dial reads on screen.
    pub name: &'static str,
    /// The full brief handed to the music model. Written like a producer's
    /// note: genre, tempo, texture, arc, and what to avoid.
    pub brief: &'static str,
}

/// The dial, in order. Comedy waits for its writer (see `docs/MUSIC.md`).
pub const STATIONS: [Station; 3] = [
    Station {
        id: "trance",
        name: "NUMINA FM",
        brief: "Melodic trance and progressive EDM instrumental, 132 BPM, in A minor. \
                A hypnotic arpeggiated synth line that slowly evolves, warm supersaw pads, \
                a clean four-on-the-floor kick with a rolling bassline, long filter sweeps, \
                one gentle breakdown in the middle that strips to pads and a plucked melody, \
                then rebuilds to a euphoric but not aggressive peak. Space and depth, wide \
                stereo, a feeling of flying over an endless glowing grid of mathematics. \
                No vocals, no risers with white-noise screech, no dubstep drops.",
    },
    Station {
        id: "chill",
        name: "THE ATTRACTOR",
        brief: "Downtempo chillwave instrumental, 84 BPM, dreamy and warm. Soft analog \
                synth chords with slow chorus, a round sub bass, gentle lo-fi drums with \
                brushed texture, tape hiss and vinyl warmth, a slow melodic motif that \
                returns like a tide, subtle field-recording shimmer underneath. The mood of \
                watching a slow fractal zoom at 2am, curious and completely unhurried. \
                No vocals, no EDM builds, no sudden transitions.",
    },
    Station {
        id: "arcade",
        name: "EIGHT BIT SUNRISE",
        brief: "Chiptune-inspired synthwave instrumental, 118 BPM, bright and playful. \
                Square-wave lead melodies in a major key over modern warm analog bass and \
                pads, crisp retro drum machine, small joyful melodic runs and arpeggios, \
                the optimism of a 1986 arcade at golden hour with the depth of a modern \
                mix. Occasional triumphant key change. No vocals, no harsh bitcrush, \
                keep it warm.",
    },
];

/// Find a station by its dial id.
#[must_use]
pub fn station(id: &str) -> Option<&'static Station> {
    STATIONS.iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::{STATIONS, station};

    #[test]
    fn the_dial_has_distinct_working_stations() {
        assert!(STATIONS.len() >= 3);
        for s in &STATIONS {
            assert!(!s.id.is_empty() && s.id == s.id.to_lowercase());
            assert!(!s.name.is_empty());
            assert!(
                s.brief.len() > 200,
                "{} brief is a real producer note",
                s.id
            );
            assert!(s.brief.contains("No vocals"), "{} stays instrumental", s.id);
            assert!(s.brief.contains("BPM"), "{} names its tempo", s.id);
        }
        let mut ids: Vec<_> = STATIONS.iter().map(|s| s.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), STATIONS.len(), "ids are unique");
    }

    #[test]
    fn the_dial_finds_by_id() {
        assert_eq!(station("trance").map(|s| s.name), Some("NUMINA FM"));
        assert!(station("polka").is_none());
    }
}
