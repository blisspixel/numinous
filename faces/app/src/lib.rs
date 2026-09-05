//! Reusable boundaries owned by the windowed Numinous App.
//!
//! Window creation and device routing remain in the binary. This library
//! exposes the read-only local session viewer plus deterministic Studio and Nim
//! renderers, plus bundled study text, so integration tests, the live App, and
//! future App shells exercise the same pairing, retention, and presentation
//! implementations.

pub mod nim_render;
pub mod session_viewer;
pub mod studio_render;
pub mod study_reader;
pub mod study_text;

#[allow(missing_docs)]
pub mod controls;
#[allow(missing_docs)]
pub mod game_draw;
#[allow(missing_docs)]
pub mod input_legend;
#[allow(missing_docs)]
pub mod menu;
mod menu_font;
#[allow(missing_docs)]
pub mod play;
#[allow(missing_docs)]
pub mod room_phase;

/// Whether the App's own surfaces stay readable for a color-blind player.
///
/// The catalog sweeps in `numinous_core` measure rooms. They do not reach here,
/// and this is where the question bites hardest: a terminal player can set
/// `NO_COLOR` and get a picture built from block characters, while the App is
/// pixels and a player with a color deficiency has no such switch.
///
/// The App draws its chrome and its games with the same marks rooms use,
/// against accents of its own. Both are read from these sources rather than
/// listed here, so a new accent or a newly drawn mark is picked up instead of
/// quietly going unchecked.
#[cfg(test)]
mod color_access {
    use numinous_core::Raster;
    use numinous_core::dichromacy;
    use std::path::Path;

    /// Accent and mark pairs the App draws that a dichromat cannot separate,
    /// with what was measured and why each is not a defect.
    ///
    /// One entry, and it is text hierarchy rather than information. On the
    /// gauntlet's bomb stage the heading is drawn with `'#'` and the body with
    /// `'*'`, 49 apart for ordinary vision and 23 for a tritanope. Both are
    /// words, and the words say what they say; what is lost is the heading
    /// looking like a heading, which is a smaller thing than a cue that only
    /// exists in color.
    ///
    /// Shrink-only. A pair arriving here that is not text hierarchy is a real
    /// defect and must be fixed rather than added.
    const KNOWN_BENIGN: [([u8; 3], char, char); 1] = [([255, 140, 120], '#', '*')];

    /// Every mark that could be ink. `'-'` is the near-black one, kept in
    /// because a surface drawing it against a dark accent would be a genuine
    /// find.
    const MARKS: [char; 9] = ['#', '!', '-', '@', '%', '&', '~', '*', '.'];

    /// The calls that put a mark on a raster.
    const DRAWING_CALLS: [&str; 4] = ["draw_text(", ".line(", ".plot(", ".rect("];

    /// The marks a source actually draws with.
    ///
    /// Only a literal inside a drawing call's argument list counts. Matching
    /// the literal anywhere in the file is what a first attempt did, and it
    /// reported 77 collapsing pairs by counting `app.quiz_answer('!')`, which
    /// is a keystroke, and `assert!(fitted.contains('~'))`, which is a test
    /// reading text. Neither is ink, and pairs built from them are collisions
    /// on marks no surface ever draws together.
    ///
    /// The argument list is found by balancing parentheses from the call's own
    /// bracket, so a call spanning fifteen lines is read whole and a call two
    /// lines later is not swept in with it.
    fn marks_drawn_in(source: &str) -> Vec<char> {
        let bytes = source.as_bytes();
        let mut found = Vec::new();
        for call in DRAWING_CALLS {
            for (index, _) in source.match_indices(call) {
                let open = index + call.len() - 1;
                let mut depth = 0usize;
                let mut end = open;
                for (offset, byte) in bytes[open..].iter().enumerate() {
                    match byte {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = open + offset;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let arguments = &source[open..=end.max(open)];
                for mark in MARKS {
                    if arguments.contains(&format!("'{mark}'")) && !found.contains(&mark) {
                        found.push(mark);
                    }
                }
            }
        }
        found
    }

    /// Read the App's own sources, returning the accents it builds rasters with
    /// and the marks it draws.
    ///
    /// A scan rather than a list, for the same reason the catalog sweeps scan:
    /// a list is a second copy of what the code already knows, and it drifts.
    fn accents_and_marks() -> (Vec<[u8; 3]>, Vec<char>) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut accents: Vec<[u8; 3]> = Vec::new();
        let mut marks: Vec<char> = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("the App source directory is readable") {
            let path = entry.expect("a readable directory entry").path();
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a readable App source");
            for (index, _) in source.match_indices("with_accent(") {
                let rest = &source[index..];
                let Some(open) = rest.find('[') else { continue };
                let Some(close) = rest[open..].find(']') else {
                    continue;
                };
                let channels: Vec<u8> = rest[open + 1..open + close]
                    .split(',')
                    .filter_map(|part| part.trim().parse::<u8>().ok())
                    .collect();
                if let Ok(accent) = <[u8; 3]>::try_from(channels.as_slice())
                    && !accents.contains(&accent)
                {
                    accents.push(accent);
                }
            }
            for mark in marks_drawn_in(&source) {
                if !marks.contains(&mark) {
                    marks.push(mark);
                }
            }
        }
        accents.sort();
        marks.sort_unstable();
        (accents, marks)
    }

    #[test]
    fn the_apps_own_marks_stay_apart_for_a_color_blind_player() {
        let (accents, marks) = accents_and_marks();

        // Proof the scan read something. A scan that found no accent would
        // report a clean App by never having looked at one.
        assert!(
            accents.len() >= 8,
            "only {} App accents found, so the scan is broken rather than the App \
             being plain",
            accents.len()
        );
        assert!(
            marks.len() >= 4,
            "only {} App marks found, so the scan is broken",
            marks.len()
        );

        let mut collapsing: Vec<([u8; 3], char, char)> = Vec::new();
        for accent in accents {
            let raster = Raster::with_accent(1, 1, accent);
            for (index, &first) in marks.iter().enumerate() {
                for &second in marks.iter().skip(index + 1) {
                    let (a, b) = (raster.ink(first), raster.ink(second));
                    // Marks that paint the same color are not a distinction the
                    // App is drawing, so they cannot be one it loses.
                    if a != b && dichromacy::color_alone(a, b) {
                        // Every mark that is not special paints the plain
                        // accent, so `'#'` against `'*'` and `'#'` against
                        // `'.'` are one defect. Recording both would say the
                        // App has twice the problem it has.
                        let name = |mark: char| {
                            if raster.ink(mark) == accent {
                                '*'
                            } else {
                                mark
                            }
                        };
                        let entry = (accent, name(first), name(second));
                        if !collapsing.contains(&entry) {
                            collapsing.push(entry);
                        }
                    }
                }
            }
        }
        collapsing.sort();

        let newly: Vec<&([u8; 3], char, char)> = collapsing
            .iter()
            .filter(|it| !KNOWN_BENIGN.contains(it))
            .collect();
        assert!(
            newly.is_empty(),
            "the App newly draws marks a color-blind player cannot separate. Each is a \
             defect unless it is text hierarchy, in which case record why: {newly:?}"
        );
        let gone: Vec<&([u8; 3], char, char)> = KNOWN_BENIGN
            .iter()
            .filter(|it| !collapsing.contains(it))
            .collect();
        assert!(
            gone.is_empty(),
            "these no longer collapse and must leave KNOWN_BENIGN: {gone:?}"
        );
    }
}
