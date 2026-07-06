//! Engine B: the radio. Stations, like a car stereo for a mathematical world.
//!
//! A station is a GENRE with a house identity, not a song: its brief carries
//! texture, mood, and taboos, and each track on its rotation gets its own
//! tempo, key, and subgenre from the station's deck, so tuning in feels like
//! real radio, one station, many records. The core owns only the pure part;
//! fetching and playback are the faces' business. See `docs/MUSIC.md`.

/// One radio station: a dial position in the world's soundtrack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Station {
    /// The dial name, lowercase, stable (used for cache filenames).
    pub id: &'static str,
    /// How the dial reads on screen.
    pub name: &'static str,
    /// The station's identity: genre, texture, mood, and what it never
    /// plays. Deliberately free of tempo and key; those belong to tracks.
    pub brief: &'static str,
    /// The rotation deck: per-track direction (subgenre, tempo, key, arc).
    /// Track n gets deck entry n mod len, so a playlist never repeats a brief.
    pub deck: &'static [&'static str],
}

/// The dial, in order. Comedy waits for its writer (see `docs/MUSIC.md`).
pub const STATIONS: [Station; 3] = [
    Station {
        id: "trance",
        name: "NUMINA FM",
        brief: "An instrumental trance and EDM radio station. House identity: hypnotic \
                melodic electronic music, clean and euphoric rather than aggressive, wide \
                stereo, space and depth, the feeling of flying over an endless glowing \
                grid of mathematics. Always instrumental. Never: vocals, white-noise \
                riser screech, dubstep drops, harsh distortion.",
        deck: &[
            " This track: classic uplifting trance, 138 BPM, A minor, long breakdown into a euphoric supersaw peak.",
            " This track: progressive trance, 128 BPM, F sharp minor, patient evolving arpeggio, no big drop, a slow tide.",
            " This track: driving peak-time techno-trance, 134 BPM, C minor, rolling bassline, hypnotic and dark but never harsh.",
            " This track: dreamy anthem trance, 136 BPM, E minor, plucked melody over warm pads, one triumphant key change.",
            " This track: deep progressive house, 122 BPM, D minor, groovy and understated, for the long drive.",
            " This track: psytrance-leaning, 142 BPM, G minor, squelchy rubber bassline, playful acid accents, controlled energy.",
        ],
    },
    Station {
        id: "chill",
        name: "THE ATTRACTOR",
        brief: "An instrumental chill radio station. House identity: warm, dreamy, \
                unhurried electronic music with analog texture, tape hiss and vinyl \
                warmth, the mood of watching a slow fractal zoom at 2am, curious and \
                completely calm. Always instrumental. Never: vocals, EDM builds, sudden \
                transitions, brightness that startles.",
        deck: &[
            " This track: downtempo chillwave, 84 BPM, C major seventh chords, a slow melodic motif returning like a tide.",
            " This track: lo-fi hip hop, 74 BPM, jazzy Rhodes chords, brushed drums, rain-on-a-window texture.",
            " This track: ambient drift, no percussion at all, slow pads in D flat major, shimmer and air.",
            " This track: mellow deep dub, 92 BPM, sub bass and sparse echoing chords, patient and warm.",
            " This track: gentle downtempo with soft guitar over synth pads, 80 BPM, golden-hour warmth.",
        ],
    },
    Station {
        id: "arcade",
        name: "EIGHT BIT SUNRISE",
        brief: "An instrumental retro-electronic radio station. House identity: chiptune \
                and synthwave joy, square-wave leads and warm analog low end together, \
                the optimism of a 1986 arcade at golden hour with the depth of a modern \
                mix. Always instrumental. Never: vocals, harsh bitcrush, horror synth, \
                anything cold.",
        deck: &[
            " This track: bright synthwave, 118 BPM, C major, small joyful melodic runs, crisp retro drum machine.",
            " This track: pure chiptune, 150 BPM, A major, fast arpeggios and a heroic lead, boss-battle energy kept friendly.",
            " This track: outrun cruise, 100 BPM, F major, gated snare, long chords, night-highway glide.",
            " This track: cozy puzzle-game groove, 110 BPM, G major, bouncy bassline, playful call-and-answer melodies.",
            " This track: victory-lap fanfare into a half-time outro, 126 BPM, D major, one triumphant key change.",
        ],
    },
];

/// Track lengths per station, in seconds, cycled alongside the deck so a
/// station plays records of different sizes: radio, not a loop of singles.
/// Trance stretches out; chill wanders; arcade keeps it punchy.
#[must_use]
pub fn length_for(station: &Station, track: usize) -> u64 {
    let cycle: &[u64] = match station.id {
        "trance" => &[247, 331, 178, 296, 152, 363],
        "chill" => &[214, 293, 358, 176, 269],
        _ => &[148, 203, 124, 237, 172],
    };
    cycle[track % cycle.len()]
}

/// The full brief for a station's nth track: the house identity plus that
/// track's card from the rotation deck.
#[must_use]
pub fn brief_for(station: &Station, track: usize) -> String {
    format!(
        "{}{}",
        station.brief,
        station.deck[track % station.deck.len()]
    )
}

/// Find a station by its dial id.
#[must_use]
pub fn station(id: &str) -> Option<&'static Station> {
    STATIONS.iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::{STATIONS, brief_for, station};

    #[test]
    fn stations_are_genres_and_tracks_carry_the_tempo() {
        assert!(STATIONS.len() >= 3);
        for s in &STATIONS {
            assert!(!s.id.is_empty() && s.id == s.id.to_lowercase());
            assert!(!s.name.is_empty());
            assert!(s.brief.len() > 150, "{}: a real house identity", s.id);
            assert!(
                !s.brief.contains("BPM"),
                "{}: the station is a genre; tempo belongs to tracks",
                s.id
            );
            assert!(
                s.brief.contains("instrumental"),
                "{}: stays instrumental",
                s.id
            );
            assert!(s.deck.len() >= 5, "{}: a real rotation", s.id);
            for card in s.deck {
                assert!(card.contains("BPM") || card.contains("no percussion"));
            }
        }
        let mut ids: Vec<_> = STATIONS.iter().map(|s| s.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), STATIONS.len(), "ids are unique");
    }

    #[test]
    fn rotation_never_repeats_until_the_deck_wraps() {
        let st = &STATIONS[0];
        let briefs: Vec<String> = (0..st.deck.len()).map(|i| brief_for(st, i)).collect();
        for (i, a) in briefs.iter().enumerate() {
            assert!(a.starts_with(st.brief), "the house sound holds");
            for b in briefs.iter().skip(i + 1) {
                assert_ne!(a, b, "every card in the deck is distinct");
            }
        }
        assert_eq!(brief_for(st, 0), brief_for(st, st.deck.len()), "then wraps");
    }

    #[test]
    fn track_lengths_vary_like_real_radio() {
        for st in &STATIONS {
            let lengths: Vec<u64> = (0..6).map(|i| super::length_for(st, i)).collect();
            let distinct: std::collections::BTreeSet<_> = lengths.iter().collect();
            assert!(
                distinct.len() >= 4,
                "{}: a real spread, not one runtime",
                st.id
            );
            for &secs in &lengths {
                assert!((120..=600).contains(&secs), "{}: {secs}s is a song", st.id);
            }
        }
    }

    #[test]
    fn the_dial_finds_by_id() {
        assert_eq!(station("trance").map(|s| s.name), Some("NUMINA FM"));
        assert!(station("polka").is_none());
    }
}
