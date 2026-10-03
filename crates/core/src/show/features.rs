//! What the director knows about a room, derived rather than authored.
//!
//! Every feature comes from something the room already owns: hue from its
//! accent, tempo and key from its motif, and ink and motion from its own ASCII
//! renders across the window the Show would perform. Nothing here is a second
//! hand-maintained description of a room that could drift from the room.
//!
//! Ink and motion are measured on the [`Canvas`], not the pixel raster, so a
//! change to the raster pipeline cannot move them. Every feature is reduced to
//! a small integer before the director scores it, so ordering is integer
//! arithmetic over these values.

use std::sync::OnceLock;

use super::profiles::director_profile;
use crate::{Canvas, ROOM_CATALOG, Room, RoomMeta, room_by_id};

/// Columns of the canvas the director measures on, the MCP default width.
pub(crate) const MEASURE_WIDTH: usize = 72;
/// Rows of the canvas the director measures on, the MCP default height.
pub(crate) const MEASURE_HEIGHT: usize = 32;
/// Evenly spaced looks across a room's window, both ends included.
pub(crate) const MEASURE_LOOKS: usize = 4;
/// Mean changed cells between neighboring looks below which a room is
/// near-static: under one percent of the measured canvas.
pub(crate) const MOTION_FLOOR: u32 = 23;
/// Reference pitch for pitch classes: C4 in twelve-tone equal temperament.
const C4_HZ: f64 = 261.625_565_300_598_6;
/// Upper bounds, in thousandths of the canvas, of the first four ink levels.
const INK_LEVELS: [u32; 4] = [50, 100, 175, 275];
/// Upper bounds, in thousandths of the canvas, of the first four motion levels.
const MOTION_LEVELS: [u32; 4] = [10, 40, 100, 200];
/// Upper bounds, in beats per minute, of the first four tempo levels.
const TEMPO_LEVELS: [u32; 4] = [80, 90, 100, 110];

/// Derived, integer features of one catalog room under its director profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RoomFeatures {
    pub(crate) catalog_index: usize,
    pub(crate) room_id: &'static str,
    pub(crate) wing: &'static str,
    /// Accent hue in whole degrees, or `None` for a gray accent.
    pub(crate) hue: Option<u16>,
    /// Motif tempo in beats per minute, when the room has a motif.
    pub(crate) tempo: Option<u32>,
    /// Motif root as a pitch class from C (0) to B (11).
    pub(crate) pitch_class: Option<u8>,
    /// Mean inked cells across the window's looks.
    pub(crate) ink: u32,
    /// Mean changed cells between neighboring looks across the window.
    pub(crate) motion: u32,
}

impl RoomFeatures {
    /// Whether the window barely moves, so a hands-off cue would park on a
    /// still frame.
    pub(crate) const fn is_near_static(self) -> bool {
        self.motion < MOTION_FLOOR
    }

    /// Ink coverage from sparse (0) to dense (4).
    pub(crate) fn ink_level(self) -> u32 {
        level(per_mille_of_canvas(self.ink), &INK_LEVELS)
    }

    /// Energy from a breather (1) to a peak (5): tempo and motion together.
    pub(crate) fn energy(self) -> u32 {
        let tempo = self.tempo.map_or(2, |tempo| level(tempo, &TEMPO_LEVELS));
        let motion = level(per_mille_of_canvas(self.motion), &MOTION_LEVELS);
        1 + (tempo + motion) / 2
    }
}

/// The derived features of every catalog room, in catalog order.
///
/// Measuring renders each room a few times, so the table is built once per
/// process and shared.
pub(crate) fn feature_table() -> &'static [RoomFeatures] {
    static TABLE: OnceLock<Vec<RoomFeatures>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ROOM_CATALOG
            .iter()
            .enumerate()
            .filter_map(|(index, metadata)| measure(index, *metadata))
            .collect()
    })
}

fn measure(catalog_index: usize, metadata: RoomMeta) -> Option<RoomFeatures> {
    let room = room_by_id(metadata.id)?;
    let profile = director_profile(metadata.id)?;
    let looks = profile
        .window()
        .samples(MEASURE_LOOKS)
        .into_iter()
        .map(|phase| render(room.as_ref(), phase))
        .collect::<Vec<_>>();
    let ink = looks.iter().map(Canvas::ink_count).sum::<usize>() / looks.len();
    let changed = looks
        .windows(2)
        .filter_map(|pair| pair[0].delta(&pair[1]))
        .map(|delta| delta.cells_changed)
        .sum::<usize>()
        / (looks.len() - 1);
    let motif = room.motif();
    Some(RoomFeatures {
        catalog_index,
        room_id: metadata.id,
        wing: metadata.wing,
        hue: accent_hue(metadata.accent),
        tempo: motif.map(|motif| motif.tempo),
        pitch_class: motif.and_then(|motif| pitch_class(f64::from(motif.root))),
        ink: u32::try_from(ink).unwrap_or(u32::MAX),
        motion: u32::try_from(changed).unwrap_or(u32::MAX),
    })
}

fn render(room: &dyn Room, phase: f64) -> Canvas {
    let mut canvas = Canvas::new(MEASURE_WIDTH, MEASURE_HEIGHT);
    room.render(&mut canvas, phase);
    canvas
}

/// The hue of an sRGB accent in whole degrees, computed in integers.
///
/// Integer arithmetic gives every platform the same answer. A gray accent has
/// no hue, which is reported rather than guessed.
pub(crate) fn accent_hue([red, green, blue]: [u8; 3]) -> Option<u16> {
    let (r, g, b) = (i32::from(red), i32::from(green), i32::from(blue));
    let max = r.max(g).max(b);
    let chroma = max - r.min(g).min(b);
    if chroma == 0 {
        return None;
    }
    // Each sector spans sixty degrees; the offset inside it is the signed
    // difference of the other two channels over the chroma.
    let (sector_start, numerator) = if max == r {
        (0, g - b)
    } else if max == g {
        (120, b - r)
    } else {
        (240, r - g)
    };
    let degrees = (sector_start + (60 * numerator + chroma / 2).div_euclid(chroma)).rem_euclid(360);
    u16::try_from(degrees).ok()
}

/// The pitch class of a frequency, from C (0) to B (11).
///
/// Motif roots are tuned notes, so the semitone count sits next to a whole
/// number and the rounding is never a coin toss.
pub(crate) fn pitch_class(frequency: f64) -> Option<u8> {
    if !frequency.is_finite() || frequency <= 0.0 {
        return None;
    }
    let semitones = (12.0 * (frequency / C4_HZ).log2()).round();
    // Any audible root is within a few octaves of C4, far inside i64.
    let class = (semitones as i64).rem_euclid(12);
    u8::try_from(class).ok()
}

/// Circular distance between two pitch classes around the circle of fifths.
pub(crate) fn fifths_distance(a: u8, b: u8) -> u8 {
    let position = |class: u8| (u16::from(class) * 7) % 12;
    let gap = position(a).abs_diff(position(b));
    // Both positions are below twelve, so the gap and its complement fit a u8.
    gap.min(12 - gap) as u8
}

fn per_mille_of_canvas(cells: u32) -> u32 {
    let total = (MEASURE_WIDTH * MEASURE_HEIGHT) as u32;
    cells.saturating_mul(1000) / total
}

fn level(value: u32, bounds: &[u32; 4]) -> u32 {
    bounds.iter().take_while(|&&bound| value >= bound).count() as u32
}

#[cfg(test)]
mod tests {
    use super::{
        MOTION_FLOOR, accent_hue, feature_table, fifths_distance, level, measure, pitch_class,
    };
    use crate::{ROOM_CATALOG, catalog_index, room_meta_by_id};

    #[test]
    fn integer_hue_matches_the_six_primary_and_secondary_corners() {
        assert_eq!(accent_hue([255, 0, 0]), Some(0));
        assert_eq!(accent_hue([255, 255, 0]), Some(60));
        assert_eq!(accent_hue([0, 255, 0]), Some(120));
        assert_eq!(accent_hue([0, 255, 255]), Some(180));
        assert_eq!(accent_hue([0, 0, 255]), Some(240));
        assert_eq!(accent_hue([255, 0, 255]), Some(300));
        // A hue just below red wraps to the top of the circle, rounded.
        assert_eq!(accent_hue([255, 0, 10]), Some(358));
        assert_eq!(accent_hue([90, 90, 90]), None);
        // Times Tables' teal: max blue, so the 240 sector, pulled toward cyan.
        assert_eq!(accent_hue([40, 150, 190]), Some(196));
    }

    #[test]
    fn motif_roots_resolve_to_their_named_pitch_classes() {
        assert_eq!(pitch_class(261.63), Some(0));
        assert_eq!(pitch_class(146.83), Some(2));
        assert_eq!(pitch_class(220.0), Some(9));
        assert_eq!(pitch_class(185.0), Some(6));
        assert_eq!(pitch_class(65.41), Some(0));
        assert_eq!(pitch_class(0.0), None);
        assert_eq!(pitch_class(f64::NAN), None);
        assert_eq!(fifths_distance(0, 7), 1);
        assert_eq!(fifths_distance(0, 2), 2);
        assert_eq!(fifths_distance(0, 6), 6);
        assert_eq!(fifths_distance(9, 9), 0);
    }

    #[test]
    fn levels_count_the_bounds_a_value_has_reached() {
        let bounds = [10, 20, 30, 40];
        assert_eq!(level(0, &bounds), 0);
        assert_eq!(level(10, &bounds), 1);
        assert_eq!(level(39, &bounds), 3);
        assert_eq!(level(400, &bounds), 4);
    }

    #[test]
    fn the_table_covers_the_catalog_and_measuring_is_repeatable() {
        let table = feature_table();
        assert_eq!(table.len(), ROOM_CATALOG.len());
        for (index, features) in table.iter().enumerate() {
            assert_eq!(features.catalog_index, index);
            assert_eq!(features.room_id, ROOM_CATALOG[index].id);
            assert!(features.energy() >= 1 && features.energy() <= 5);
            assert!(features.ink_level() <= 4);
        }
        for id in ["times-tables", "galton-board", "lorenz"] {
            let index = catalog_index(id).expect("catalog room");
            let metadata = room_meta_by_id(id).expect("metadata");
            assert_eq!(measure(index, metadata), Some(table[index]), "{id}");
        }
    }

    #[test]
    fn phase_invariant_rooms_measure_as_near_static() {
        // The Galton Board waits for a hand: phase alone never moves it.
        let galton = feature_table()[catalog_index("galton-board").expect("room")];
        assert_eq!(galton.motion, 0);
        assert!(galton.is_near_static());
        let times = feature_table()[catalog_index("times-tables").expect("room")];
        assert!(times.motion >= MOTION_FLOOR, "{times:?}");
        assert!(!times.is_near_static());
    }
}
