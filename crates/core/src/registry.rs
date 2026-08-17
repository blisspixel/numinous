//! The room registry: the catalog every face enumerates.
//!
//! The registry is the only thing a face needs to know about; it never depends
//! on a room's internals (see the dependency rule in `docs/ARCHITECTURE.md`).

use crate::room::Room;
use crate::rooms;

/// All built-in rooms, in catalog order. Default variation 0 pins tests and postcards.
#[must_use]
pub fn all_rooms() -> Vec<Box<dyn Room>> {
    all_rooms_with(0)
}

/// All rooms with a per-visit variation seed (default 0 keeps exact behavior for
/// tests, postcards, and determinism). Rooms that support it read the seed for
/// replayable novelty. See ARCADE.md and DIGITAL_MINDS.md.
#[must_use]
pub fn all_rooms_with(variation: u64) -> Vec<Box<dyn Room>> {
    rooms::construct_all(variation)
}

/// Find a built-in room by its stable id, if it exists.
#[must_use]
pub fn room_by_id(id: &str) -> Option<Box<dyn Room>> {
    room_by_id_with(id, 0)
}

/// Find a built-in room by stable id with a replayable variation seed.
#[must_use]
pub fn room_by_id_with(id: &str, variation: u64) -> Option<Box<dyn Room>> {
    rooms::construct_by_id(id, variation)
}

/// Rooms measured over the WCAG 2.3.1 flash budget on 2026-08-05.
///
/// This list is a record of a real defect, not a permission slip. It exists
/// so the budget can be enforced on the other rooms today instead of waiting
/// for these to be redesigned, and tests fail if the list grows, if an entry
/// stops violating and is not removed, or if a room outside it starts
/// flashing. It is public so the accessibility report can name these rooms
/// to the player from the same list the tests enforce: a count that lives
/// in prose drifts, a count that lives here cannot.
pub const KNOWN_OVER_FLASH_BUDGET: [&str; 3] = ["coupled-tent", "gauss-map", "ricker"];

/// Rooms whose answer to a touch the color-free renderer cannot show,
/// measured 2026-08-05.
///
/// The first three need the room to answer with shape rather than only
/// brightness; `magnet-fractal` changes both-lit cells by too little
/// luminance to survive the block-character floor. A record of a real
/// defect, not a permission slip, public for the same reason as
/// [`KNOWN_OVER_FLASH_BUDGET`]: the player-facing report and the enforcing
/// tests must read one list.
pub const RESPONSE_INVISIBLE_WITHOUT_COLOR: [&str; 4] =
    ["hilbert", "magnet-fractal", "percolation", "wireworld"];

/// The longest rejected id a not-found message will echo back. Beyond this the
/// tail is dropped, so a hostile or accidental megabyte cannot become the
/// message.
pub const MAX_ECHOED_ID: usize = 48;

/// The most candidates a not-found message offers. Small on purpose: the
/// catalog is discovered by playing, not by reading a list (see `PLAY.md`).
pub const MAX_ROOM_SUGGESTIONS: usize = 3;

/// Names from `candidates` closest to `query`, nearest first.
///
/// Returns nothing when nothing is genuinely close, so a wrong guess stays
/// silent rather than pointing somewhere misleading. Used wherever a face has
/// to reject a name it recognizes the shape of: a room id, a tool argument.
#[must_use]
pub fn nearest_names<'a, I>(query: &str, candidates: I, limit: usize) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }
    // Measured once: inside the loop these would be recomputed for every
    // candidate, and the loop runs the length of the catalog.
    let query_chars = query.chars().count();
    let tolerance = close_enough(query_chars);
    let mut scored: Vec<(usize, usize, &'a str)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let lowered = candidate.to_ascii_lowercase();
            let distance = edit_distance(&query, &lowered);
            // Containment ranks ahead of any edit distance: someone who typed
            // "mandel" wants "mandelbrot", however many edits separate them.
            // It takes a real fragment to count, or short names like "id" and
            // "t" would match any word that happens to spell them.
            let contained = (lowered.contains(&query) || query.contains(&lowered))
                && query_chars.min(lowered.chars().count()) >= MIN_CONTAINED_CHARS;
            if !contained && distance > tolerance {
                return None;
            }
            Some((usize::from(!contained), distance, candidate))
        })
        .collect();
    // Rank, then distance, then name: a total order, so equal candidates
    // resolve the same way on every run and every platform.
    scored.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(b.2)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, name)| name)
        .collect()
}

/// Catalog ids closest to `id`, nearest first.
///
/// The result never grows with the catalog, which keeps a not-found message a
/// fixed size no matter how many rooms ship.
#[must_use]
pub fn nearest_room_ids(id: &str, limit: usize) -> Vec<&'static str> {
    nearest_names(
        id,
        rooms::ROOM_CATALOG.iter().map(|metadata| metadata.id),
        limit,
    )
}

/// The shortest fragment that may stand in for a whole name. Below this,
/// containment stops meaning anything: "id" and "t" are spelled inside a great
/// many words that have nothing to do with them.
const MIN_CONTAINED_CHARS: usize = 3;

/// How far a typo may stray and still count as the same word. One edit for a
/// short name, growing slowly with length, so long ids tolerate a slip without
/// nonsense matching everything.
fn close_enough(length: usize) -> usize {
    (length / 4).clamp(1, 3)
}

/// Optimal string alignment distance: Levenshtein plus adjacent transposition.
///
/// Transposing two characters is the most common typing slip there is, and
/// plain Levenshtein charges two edits for it, which puts "widht" further from
/// "width" than a threshold this tight will allow. Counting it as one edit is
/// what makes the suggestion useful on short names.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    // Three rolling rows: the transposition case needs the row before last.
    let mut before_previous = vec![0usize; b_chars.len() + 1];
    let mut previous: Vec<usize> = (0..=b_chars.len()).collect();
    let mut current = vec![0usize; b_chars.len() + 1];
    for (i, &a_char) in a_chars.iter().enumerate() {
        current[0] = i + 1;
        for (j, &b_char) in b_chars.iter().enumerate() {
            let substitution = previous[j] + usize::from(a_char != b_char);
            let mut best = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
            if i > 0 && j > 0 && a_char == b_chars[j - 1] && a_chars[i - 1] == b_char {
                best = best.min(before_previous[j - 1] + 1);
            }
            current[j + 1] = best;
        }
        std::mem::swap(&mut before_previous, &mut previous);
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b_chars.len()]
}

/// Whether a character must be escaped before it is echoed back to a person.
///
/// Four families qualify, and none of them are what `char::is_control` alone
/// would catch:
///
/// - C0 and C1 controls, which can drive a terminal.
/// - The bidirectional formatting characters, which reorder how a line
///   displays without changing what it contains, so a diagnostic can be made
///   to read as something other than what it says (the Trojan Source problem).
/// - The line and paragraph separators, which are not control characters but
///   which many renderers break lines on, so untrusted input could otherwise
///   forge extra lines in a diagnostic or a transcript.
/// - The zero-width characters, which are the reason an id that looks exactly
///   like `times-tables` can fail to match it: escaping them turns a baffling
///   rejection into a visible cause.
#[must_use]
pub fn must_escape_for_display(character: char) -> bool {
    character.is_control()
        // Zl and Zp: line and paragraph separators. Not controls, but treated
        // as hard breaks by enough renderers to be a forgery risk.
        || matches!(character, '\u{2028}' | '\u{2029}')
        || matches!(character,
            '\u{00AD}'                 // soft hyphen
            | '\u{061C}'               // Arabic letter mark
            | '\u{180E}'               // Mongolian vowel separator
            | '\u{200B}'..='\u{200F}'  // zero-width space through RTL mark
            | '\u{202A}'..='\u{202E}'  // bidi embeddings and overrides
            | '\u{2060}'..='\u{2064}'  // word joiner and invisible operators
            | '\u{2066}'..='\u{2069}'  // bidi isolates
            | '\u{FEFF}'               // zero-width no-break space
        )
}

/// Text rendered safe to show a person: every character that could drive a
/// terminal, reorder the line, or hide inside it is escaped to its printable
/// form. Length is not bounded here; see [`echoable_id`] for that.
#[must_use]
pub fn display_safe(text: &str) -> String {
    let mut safe = String::with_capacity(text.len());
    for character in text.chars() {
        if must_escape_for_display(character) {
            safe.extend(character.escape_default());
        } else {
            safe.push(character);
        }
    }
    safe
}

/// A rejected id rendered safe to echo: escaped for display and length
/// bounded, so untrusted input cannot corrupt a terminal or a client
/// transcript and cannot inflate the message it appears in.
#[must_use]
pub fn echoable_id(id: &str) -> String {
    let mut safe = String::with_capacity(id.len().min(MAX_ECHOED_ID));
    for character in id.chars().take(MAX_ECHOED_ID) {
        if must_escape_for_display(character) {
            safe.extend(character.escape_default());
        } else {
            safe.push(character);
        }
    }
    // Ask only whether a character exists past the bound, never how many do:
    // counting would scan the whole input, which is the cost this bound exists
    // to avoid.
    if id.chars().nth(MAX_ECHOED_ID).is_some() {
        safe.push_str("...");
    }
    safe
}

/// The rooms that are not in the catalog. Never listed, never announced; the
/// faces decide who may enter (by rank, see `crate::journey`). Calling this a
/// registry function is already saying too much.
#[must_use]
pub fn hidden_room_by_id(id: &str) -> Option<Box<dyn Room>> {
    rooms::construct_hidden_by_id(id)
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_ECHOED_ID, MAX_ROOM_SUGGESTIONS, all_rooms, display_safe, echoable_id,
        must_escape_for_display, nearest_names, nearest_room_ids, room_by_id, room_by_id_with,
    };
    use crate::canvas::Canvas;
    use crate::room::Room;

    fn render_text(room: &dyn Room, t: f64) -> String {
        let mut canvas = Canvas::new(48, 28);
        room.render(&mut canvas, t);
        canvas.to_text()
    }

    fn render_poked_text(room: &dyn Room, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 28);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    fn room_text(rooms: &[Box<dyn Room>], id: &str, t: f64) -> String {
        let room = rooms
            .iter()
            .find(|room| room.meta().id == id)
            .unwrap_or_else(|| panic!("{id} must be registered"));
        render_text(room.as_ref(), t)
    }

    #[test]
    fn registry_is_non_empty() {
        assert!(!all_rooms().is_empty());
    }

    #[test]
    fn a_near_miss_id_suggests_the_room_that_was_meant() {
        // The typos a 354-room hyphenated catalog actually produces.
        for (typo, intended) in [
            ("times-table", "times-tables"),
            ("mandelbrott", "mandelbrot"),
            ("game-of-live", "game-of-life"),
            ("lorenzo", "lorenz"),
            ("galtonboard", "galton-board"),
        ] {
            let suggestions = nearest_room_ids(typo, MAX_ROOM_SUGGESTIONS);
            assert!(
                suggestions.contains(&intended),
                "{typo} should suggest {intended}, got {suggestions:?}"
            );
        }
    }

    #[test]
    fn a_partial_id_suggests_the_rooms_containing_it() {
        let suggestions = nearest_room_ids("mandel", MAX_ROOM_SUGGESTIONS);
        assert!(
            suggestions.contains(&"mandelbrot"),
            "a prefix should reach the room it names, got {suggestions:?}"
        );
    }

    #[test]
    fn suggestions_are_capped_and_never_list_the_catalog() {
        // The bound is the point: this message must not grow with the catalog.
        assert!(nearest_room_ids("a", 3).len() <= 3);
        assert!(nearest_room_ids("mandel", 2).len() <= 2);
        assert!(nearest_room_ids("times-tables", 0).is_empty());
        assert!(nearest_room_ids("", MAX_ROOM_SUGGESTIONS).is_empty());
        assert!(nearest_room_ids("   ", MAX_ROOM_SUGGESTIONS).is_empty());
    }

    #[test]
    fn a_transposition_counts_as_one_slip() {
        // The most common typo there is. Plain Levenshtein charges two for it,
        // which would put a short name out of reach of its own correction.
        assert_eq!(
            nearest_names("widht", ["width", "height", "id", "t"], 2)
                .first()
                .copied(),
            Some("width")
        );
    }

    #[test]
    fn a_short_name_is_not_matched_by_merely_being_spelled_inside_a_word() {
        // "widht" spells both "id" and "t"; neither is what was meant.
        let suggestions = nearest_names("widht", ["id", "t"], 2);
        assert!(suggestions.is_empty(), "got {suggestions:?}");
    }

    #[test]
    fn a_real_fragment_still_reaches_the_name_it_names() {
        assert_eq!(
            nearest_names("expression", ["expr", "recipe", "seed"], 1)
                .first()
                .copied(),
            Some("expr")
        );
    }

    #[test]
    fn nonsense_suggests_nothing_rather_than_misleading() {
        let suggestions = nearest_room_ids("qqqqzzzzxxxxwwww", MAX_ROOM_SUGGESTIONS);
        assert!(
            suggestions.is_empty(),
            "unrelated input should stay silent, got {suggestions:?}"
        );
    }

    #[test]
    fn suggestions_are_deterministic() {
        let first = nearest_room_ids("mandel", MAX_ROOM_SUGGESTIONS);
        let second = nearest_room_ids("mandel", MAX_ROOM_SUGGESTIONS);
        assert_eq!(first, second);
    }

    #[test]
    fn suggestion_matching_ignores_case() {
        assert_eq!(
            nearest_room_ids("TIMES-TABLES", MAX_ROOM_SUGGESTIONS)
                .first()
                .copied(),
            Some("times-tables")
        );
    }

    #[test]
    fn text_shown_to_a_person_cannot_reorder_or_hide_itself() {
        // A bidirectional override reorders how the rest of the line displays
        // without changing what it contains, so a diagnostic can be made to
        // read as something other than what it says. is_control() does not
        // cover these: they are format characters, not control characters.
        for (name, hostile) in [
            ("right-to-left override", "safe\u{202e}dnammoc"),
            ("left-to-right override", "a\u{202d}b"),
            ("first strong isolate", "a\u{2068}b"),
            ("pop directional isolate", "a\u{2069}b"),
            ("left-to-right mark", "a\u{200e}b"),
            ("right-to-left mark", "a\u{200f}b"),
            ("zero width space", "times\u{200b}tables"),
            ("zero width joiner", "a\u{200d}b"),
            ("word joiner", "a\u{2060}b"),
            ("soft hyphen", "times\u{00ad}tables"),
            ("byte order mark", "a\u{feff}b"),
            ("Arabic letter mark", "a\u{061c}b"),
            ("escape", "a\u{1b}[2Jb"),
            ("bell", "a\u{7}b"),
            ("line separator", "a\u{2028}b"),
            ("paragraph separator", "a\u{2029}b"),
            ("Mongolian vowel separator", "a\u{180e}b"),
        ] {
            let shown = display_safe(hostile);
            assert!(
                !shown.chars().any(must_escape_for_display),
                "{name} survived display_safe: {shown:?}"
            );
            assert!(
                echoable_id(hostile)
                    .chars()
                    .all(|c| !must_escape_for_display(c)),
                "{name} survived echoable_id"
            );
        }
    }

    #[test]
    fn a_diagnostic_cannot_be_forged_into_extra_lines() {
        // U+2028 and U+2029 are not control characters, so is_control misses
        // them, but enough renderers break lines on them that untrusted input
        // could otherwise append a convincing second line to a message.
        for forgery in [
            "room\u{2028}Everything is fine.",
            "room\u{2029}Everything is fine.",
        ] {
            let shown = display_safe(forgery);
            assert_eq!(shown.lines().count(), 1, "forged a line break: {shown:?}");
            assert!(!shown.contains('\u{2028}') && !shown.contains('\u{2029}'));
        }
    }

    #[test]
    fn escaping_leaves_ordinary_text_alone() {
        // Including text that is not English. Escaping is about characters
        // that lie about the line, not about characters that are unfamiliar.
        for ordinary in [
            "times-tables",
            "Times Tables",
            "salle des maths",
            "数学の部屋",
            "комната",
            "غرفة",
            "y = sin(a*x) + 1",
        ] {
            assert_eq!(display_safe(ordinary), ordinary, "{ordinary} was altered");
        }
    }

    #[test]
    fn an_invisible_character_makes_a_lookalike_id_visibly_different() {
        // The player's complaint this answers: "I typed times-tables and it
        // says there is no such room." The message now shows why.
        let lookalike = "times-tables\u{200b}";
        assert_ne!(echoable_id(lookalike), "times-tables");
        assert!(echoable_id(lookalike).contains("\\u{200b}"));
    }

    #[test]
    fn an_echoed_id_is_escaped_and_bounded() {
        assert_eq!(echoable_id("times-table"), "times-table");
        assert_eq!(echoable_id("a\nb\tc"), "a\\nb\\tc");
        let long = "z".repeat(MAX_ECHOED_ID * 4);
        let echoed = echoable_id(&long);
        assert!(echoed.ends_with("..."));
        assert_eq!(echoed.chars().count(), MAX_ECHOED_ID + 3);
        // A hostile id cannot smuggle an escape sequence into a terminal or a
        // client transcript.
        assert!(!echoable_id("\u{1b}[2J\u{1b}[H").contains('\u{1b}'));
    }

    #[test]
    fn every_room_has_a_unique_id() {
        let rooms = all_rooms();
        let mut ids: Vec<&str> = rooms.iter().map(|r| r.meta().id).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(unique, ids.len(), "room ids must be unique");
    }

    /// Each of these renders a chaotic map whose point density changes
    /// sharply with phase. Fixing them means changing what they draw, which
    /// is a mathematical-truth decision and not something to rush inside an
    /// accessibility cycle. Tracked in `docs/ROADMAP.md` under 0.5 Sensory;
    /// the list itself is the public one the access report prints.
    use super::KNOWN_OVER_FLASH_BUDGET;

    #[test]
    fn every_room_waiting_on_a_decision_is_named_where_the_owner_reads() {
        // These lists are the am-track's record of what it cannot decide, and
        // for a long time that record lived only in a working note that is not
        // committed. Somebody reading the repository saw a scattering of room
        // names in prose and no single place saying what the track is waiting
        // on.
        //
        // The roadmap now has that place, and this keeps it honest: every room
        // the code holds on a known-failure list has to be named there. A list
        // that grows without the roadmap following would otherwise leave a
        // decision recorded nowhere a person looks.
        // Matched inside backticks, as the section writes them. A bare
        // substring would accept `zipff` for `zipf`, which is the same hole a
        // reviewer found in an earlier rule of mine.
        let section = crate::roadmap_decisions();

        for (list, rooms) in [
            ("KNOWN_OVER_FLASH_BUDGET", &KNOWN_OVER_FLASH_BUDGET[..]),
            (
                "RESPONSE_INVISIBLE_WITHOUT_COLOR",
                &RESPONSE_INVISIBLE_WITHOUT_COLOR[..],
            ),
        ] {
            assert!(!rooms.is_empty(), "{list} is empty, so this checks nothing");
            for room in rooms {
                assert!(
                    section.contains(&format!("`{room}`")),
                    "{room} is on {list} but is not named in the roadmap's decisions \
                     section, so nobody reading the repository knows it is waiting"
                );
            }
        }
    }

    #[test]
    #[ignore = "full-catalog sweep of 35,400 renders; run by the nightly and release gates"]
    fn no_catalog_room_flashes_past_the_photosensitivity_budget() {
        // WCAG 2.3.1: no more than three flashes in any one-second window. The
        // terminal loops step phase by 0.01 per frame at 30 frames per second,
        // so a full cycle is 100 frames and that is the fastest a shipped face
        // advances a room. The worst window is what the standard bounds, not
        // the average, so a room that strobes for half a second still fails.
        //
        // Measured at a declared reference size. Mean whole-frame luminance is
        // a proxy: this does not implement the flashing-area rule, and at very
        // small rasters a dense plot saturates the frame and reads as brighter
        // than it would on screen. Smaller sizes therefore report more
        // violations than this, which is recorded rather than hidden.
        const FPS: f64 = 30.0;
        const STEP: f64 = 0.01;
        const REFERENCE: (usize, usize) = (240, 140);
        let frames = (1.0 / STEP).round() as usize;

        let mut over = Vec::new();
        let mut over_red = Vec::new();
        // Widest luminance swing seen anywhere in the catalog. This proves the
        // sweep measured something. Counting flashes instead would be wrong:
        // a catalog of gentle fades produces no qualifying flashes at all, and
        // that is a pass, not an empty measurement.
        let mut widest_swing = 0.0f64;
        // Whether any room is ever a saturated red state at all. The red
        // assertions below pass trivially on a catalog that never goes red, and
        // "no room flashes red" would then be a claim about nothing.
        let mut reddest = 0.0f64;
        for room in all_rooms() {
            // Both measurements come off the same renders. They are different
            // questions, luminance against chromaticity, but rendering the
            // catalog twice to ask them separately would double the most
            // expensive part of the sweep for nothing.
            let (series, red_series): (Vec<f64>, Vec<crate::photosensitivity::RedState>) = (0
                ..frames)
                .map(|frame| {
                    let mut raster = crate::raster::Raster::with_accent(
                        REFERENCE.0,
                        REFERENCE.1,
                        room.meta().accent,
                    );
                    room.render(&mut raster, frame as f64 * STEP);
                    let rgba = raster.to_rgba();
                    (
                        crate::photosensitivity::frame_luminance(&rgba),
                        crate::photosensitivity::frame_red_state(&rgba),
                    )
                })
                .unzip();
            let low = series.iter().copied().fold(f64::MAX, f64::min);
            let high = series.iter().copied().fold(f64::MIN, f64::max);
            widest_swing = widest_swing.max(high - low);
            let peak = crate::photosensitivity::peak_flashes_per_second(&series, FPS);
            if peak > crate::photosensitivity::MAX_FLASHES_PER_SECOND {
                over.push((room.meta().id, peak));
            }

            reddest = red_series
                .iter()
                .fold(reddest, |worst, state| worst.max(state.saturation));
            let red_peak = crate::photosensitivity::peak_red_flashes_per_second(&red_series, FPS);
            if red_peak > crate::photosensitivity::MAX_FLASHES_PER_SECOND {
                over_red.push((room.meta().id, red_peak));
            }
        }

        // A catalog whose luminance never changed would pass every assertion
        // below while measuring nothing at all.
        assert!(
            widest_swing > 0.01,
            "no room's luminance varied by more than {widest_swing:.4} across a full \
             cycle, so the sweep measured nothing"
        );

        let mut unexpected: Vec<String> = over
            .iter()
            .filter(|(id, _)| !KNOWN_OVER_FLASH_BUDGET.contains(id))
            .map(|(id, peak)| format!("{id} at {peak:.2}/s"))
            .collect();
        unexpected.sort();
        assert!(
            unexpected.is_empty(),
            "rooms newly over the {:.0} flash per second budget: {}",
            crate::photosensitivity::MAX_FLASHES_PER_SECOND,
            unexpected.join(", ")
        );

        let mut fixed: Vec<&str> = KNOWN_OVER_FLASH_BUDGET
            .iter()
            .filter(|id| !over.iter().any(|(over_id, _)| over_id == *id))
            .copied()
            .collect();
        fixed.sort_unstable();
        assert!(
            fixed.is_empty(),
            "these no longer exceed the budget and must leave KNOWN_OVER_FLASH_BUDGET: {}",
            fixed.join(", ")
        );

        // What the red half of this sweep actually found, stated plainly
        // because it is easy to mistake for a stronger claim than it is: no
        // room in the catalog ever reaches the saturated-red ratio at all. The
        // reddest whole-frame mean anywhere is burning-ship at 0.658, against a
        // threshold of 0.80, with ising next at 0.617. So the budget assertion
        // below passes with room to spare rather than by a narrow margin, and
        // the flash-counting itself is proven by the unit tests in
        // `crate::photosensitivity` rather than by this catalog.
        //
        // This guard is the red counterpart of the luminance one above. A
        // catalog rendered entirely in greys would satisfy the budget assertion
        // while the red path never ran, and that would be an empty measurement
        // reported as a pass. The bar sits below the measured 0.658 so that
        // ordinary drift does not trip it, and far enough above zero that a
        // catalog which stopped drawing warm colors would.
        assert!(
            reddest > 0.5,
            "the reddest frame in the catalog measured {reddest:.4}, so the red sweep did \
             not look at anything meaningfully red"
        );

        let mut red_offenders: Vec<String> = over_red
            .iter()
            .map(|(id, peak)| format!("{id} at {peak:.2}/s"))
            .collect();
        red_offenders.sort();
        assert!(
            red_offenders.is_empty(),
            "rooms over the {:.0} red flash per second budget (reddest frame measured \
             {reddest:.4} against a {:.2} ratio): {}",
            crate::photosensitivity::MAX_FLASHES_PER_SECOND,
            crate::photosensitivity::RED_SATURATION,
            red_offenders.join(", ")
        );
    }

    #[test]
    fn every_room_postcard_has_ink() {
        // The beauty-QA invariant: no room may present an empty postcard.
        for room in all_rooms() {
            let mut canvas = Canvas::new(60, 40);
            room.render(&mut canvas, room.postcard_t());
            assert!(
                canvas.ink_count() > 10,
                "{} is blank at its postcard phase",
                room.meta().id
            );
        }
    }

    #[test]
    fn no_reveal_carries_internal_qa_chrome() {
        // The reveal is the payload: it ends on the idea, never on checkbox
        // homework. Source provenance and review checklists live as code
        // comments beside each reveal and as `citations::for_room` entries;
        // if they leak back into player-facing prose, this test names the
        // room that broke the voice.
        const CHROME_TOKENS: &[&str] = &["Provenance", "Checklist", "- [x]", "\n---"];
        for room in all_rooms() {
            let reveal = room.reveal();
            for token in CHROME_TOKENS {
                assert!(
                    !reveal.contains(token),
                    "{} lets QA chrome ({token:?}) ride its reveal",
                    room.meta().id
                );
            }
        }
    }

    #[test]
    fn no_blurb_carries_a_lever_note_fragment() {
        // A blurb describes the mathematics in prose; the touch verb rides
        // `verb()` and each face renders it honestly for its own inputs.
        // The old template tail (". t and DRAG: TUNE X.") read as broken
        // copy to a stranger, so colon-caps lever fragments are banned from
        // every blurb. Hand-written prose lever notes (Morley's "t wobbles
        // vertices") remain welcome; the fragment grammar does not.
        for room in all_rooms() {
            let blurb = room.meta().blurb;
            for fragment in ["DRAG:", "HOLD:", "CLICK:"] {
                assert!(
                    !blurb.contains(fragment),
                    "{} still carries a lever-note fragment ({fragment:?}) in its blurb",
                    room.meta().id
                );
            }
        }
    }

    #[test]
    fn no_doorway_prints_a_number_its_own_reveal_repeats() {
        // A packaged playtest read three ordinary doorways and found the
        // answer already sitting in them: Kaprekar named 6174, the First Rain
        // named the percolation threshold, the Busy Beaver named BB(5). The
        // door promises the explanation comes later and only if you ask, so a
        // doorway that states a value its own reveal states has spent the
        // room before the picture draws.
        //
        // Numbers are the mechanical half of that rule: a multi-digit value in
        // both places is an answer handed over, not a description. Single
        // digits are ordinary prose ("two primes", "three arcs") and are not
        // read as answers. The named-answer half needs judgment and lives in
        // the catalog's own `no_doorway_sells_a_staged_rooms_answer`.

        /// Numbers a doorway and a reveal may legitimately share, with why.
        const SHARED_BY_RIGHT: &[(&str, &str, &str)] = &[
            (
                "starbow",
                "1979",
                "the citation year of the transform it draws",
            ),
            ("phantom-jam", "2008", "the citation year of the experiment"),
            (
                "wet-oracle",
                "2010",
                "the citation year of the slime-mold result",
            ),
            ("morley", "1899", "the citation year of the theorem"),
            (
                "galton-board",
                "16",
                "the peg rows the player drops balls through",
            ),
            (
                "cellular-automata",
                "30",
                "a rule number on the tour, not its punchline",
            ),
            ("rule-30", "30", "the room's own name"),
            ("truchet", "10", "part of the program name 10 PRINT"),
            (
                "upside-ruler",
                "10",
                "the base of the number system the room lives in",
            ),
            (
                "legendre",
                "11",
                "the interval [-1,1] the polynomials are defined on",
            ),
        ];

        fn multi_digit_values(text: &str) -> Vec<String> {
            let mut found = Vec::new();
            let mut current = String::new();
            for character in text.chars() {
                if character.is_ascii_digit()
                    || (!current.is_empty() && matches!(character, '.' | ','))
                {
                    current.push(character);
                } else {
                    push_value(&mut found, &current);
                    current.clear();
                }
            }
            push_value(&mut found, &current);
            found
        }

        fn push_value(found: &mut Vec<String>, raw: &str) {
            let cleaned: String = raw
                .trim_end_matches(['.', ','])
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            if cleaned.len() >= 2 && !found.contains(&cleaned) {
                found.push(cleaned);
            }
        }

        let mut leaks = Vec::new();
        for room in all_rooms() {
            let id = room.meta().id;
            let in_reveal = multi_digit_values(room.reveal());
            for value in multi_digit_values(room.meta().blurb) {
                let allowed = SHARED_BY_RIGHT
                    .iter()
                    .any(|&(room_id, number, _)| room_id == id && number == value);
                if !allowed && in_reveal.contains(&value) {
                    leaks.push(format!(
                        "{id} names {value} at the door and again in its reveal"
                    ));
                }
            }
        }
        leaks.sort();
        assert!(
            leaks.is_empty(),
            "doorways that spend their own room before it draws: {}",
            leaks.join("; ")
        );
    }

    #[test]
    fn every_catalog_room_has_first_contact_status() {
        // The kid-principle invariant: first contact always names something
        // readable before the player acts. Empty status is not an invitation.
        for room in all_rooms() {
            let status = room.status(0.0);
            assert!(
                status.as_ref().is_some_and(|s| !s.trim().is_empty()),
                "{} opens silent; first contact needs a status line",
                room.meta().id
            );
        }
    }

    #[test]
    fn first_contact_status_names_an_action_or_goal_when_the_room_has_a_verb() {
        // Rooms that publish a touch verb should invite play on first contact:
        // either a direct action token (CLICK/DRAG/...) or a clear measured
        // goal (TARGET/FOUND/GOAL) so the status is not ambient-only prose.
        const INVITE_TOKENS: &[&str] = &[
            "CLICK", "DRAG", "HOLD", "DROP", "PLANT", "FLIP", "TRY", "SEED", "THROW", "TEST",
            "DIVE", "TOUCH", "PIN", "TURN", "MOVE", "PAINT", "TRACE", "BRUSH", "TUNE", "POUR",
            "RIDE", "SOW", "SCRUB", "PICK", "PUSH", "PULL", "PERTURB", "MORPH", "DIAL", "HAND",
            "COIN", "WAVE", "BET", "FIX", "PLACE", "PRINT", "NEST", "WELL", "STORM", "GLIDER",
            "WIDTH", "ORBIT", "TAP", "SWEEP", "STEER", "AIM", "REPLAY", "LAUNCH", "STRIKE", "CUT",
            "DRAW", "SPIN", "ZOOM", "FOCUS", "POINT", "TARGET", "GOAL", "OPEN", "INVITE", "CHOOSE",
        ];
        let mut shallow = Vec::new();
        for room in all_rooms() {
            let Some(verb) = room.verb() else {
                continue;
            };
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let upper = open.to_ascii_uppercase();
            let hit = INVITE_TOKENS.iter().any(|token| upper.contains(token));
            if !hit {
                shallow.push(format!("{id}: verb={verb:?} status={open:?}"));
            }
        }
        assert!(
            shallow.is_empty(),
            "first-contact invite missing for:\n{}",
            shallow.join("\n")
        );
    }

    /// Rooms whose touch response is still invisible in the color-free
    /// renderer, remeasured 2026-08-05 at 120 by 70 after `to_mono` learned to
    /// shade a cell whose halves are both lit.
    ///
    /// This list was 21. Shading a both-lit cell recovered 15 of them, and
    /// moving the shade thresholds onto the catalog's measured ink recovered
    /// two more. These four fail for two different reasons, measured at the
    /// level of individual cells rather than guessed at:
    ///
    /// `hilbert`, `percolation` and `wireworld` change only cells with one
    /// half below the lit floor `crate::ansi` uses. Such a cell renders as a half
    /// block, which says which half is lit and nothing about how brightly.
    /// The change can be large and still invisible: one `hilbert` cell moves
    /// its lit half from 174 to 251 and keeps the same glyph. No threshold
    /// helps, because no threshold is consulted.
    ///
    /// `magnet-fractal` does change both-lit cells, but by about 22 luminance
    /// inside the widest band, which spans the floor up to 128.
    ///
    /// So the first three need the room to answer with shape rather than only
    /// brightness. Encoding brightness into a half-lit cell would need a glyph
    /// that means "lower half, dimly", and the block characters do not have
    /// one; the nearest candidates encode how much of the cell is filled,
    /// which would say something false about where the ink is.
    ///
    /// The list is a record of a real defect, not a permission slip. The test
    /// below fails if it grows, if an entry starts responding and is not
    /// removed, or if a room outside it goes quiet. Tracked in
    /// `docs/ROADMAP.md` under 0.5 Sensory; the list itself is the public
    /// one the access report prints.
    use super::RESPONSE_INVISIBLE_WITHOUT_COLOR;

    /// Rooms whose picture does not change at all under a center poke,
    /// measured 2026-08-05 at 120 by 70. They still answer on the status line,
    /// which `poke_changes_status_for_every_catalog_room` enforces, but the
    /// plate itself is unmoved. Same shrink-only contract as above.
    const NO_VISUAL_RESPONSE_TO_A_POKE: [&str; 7] = [
        "brusselator",
        "cesaro",
        "koch-snowflake",
        "laplace-clock",
        "slingshot",
        "sylvester",
        "the-lens",
    ];

    #[test]
    fn a_touch_answers_without_relying_on_color() {
        // A player who cannot use color, or who set NO_COLOR, must still see
        // that the room heard them. The check is mechanical rather than a
        // matter of taste: render the room before and after a center poke,
        // strip the color with the same renderer NO_COLOR selects, and require
        // the two to differ.
        use crate::ansi::{to_ansi, to_mono};
        const SIZE: (usize, usize) = (120, 70);

        let mut invisible_without_color = Vec::new();
        let mut unmoved = Vec::new();
        for room in all_rooms() {
            let id = room.meta().id;
            let mut base = crate::raster::Raster::with_accent(SIZE.0, SIZE.1, room.meta().accent);
            room.render(&mut base, 0.35);
            let mut poked = crate::raster::Raster::with_accent(SIZE.0, SIZE.1, room.meta().accent);
            room.render_poked(&mut poked, 0.35, &[(0.5, 0.5)]);

            if to_ansi(&base) == to_ansi(&poked) {
                unmoved.push(id);
            } else if to_mono(&base) == to_mono(&poked) {
                invisible_without_color.push(id);
            }
        }

        for (measured, known, label) in [
            (
                &invisible_without_color,
                &RESPONSE_INVISIBLE_WITHOUT_COLOR[..],
                "answer in a way the color-free renderer cannot show",
            ),
            (
                &unmoved,
                &NO_VISUAL_RESPONSE_TO_A_POKE[..],
                "do not change their picture at all",
            ),
        ] {
            let mut fresh: Vec<&str> = measured
                .iter()
                .copied()
                .filter(|id| !known.contains(id))
                .collect();
            fresh.sort_unstable();
            assert!(
                fresh.is_empty(),
                "rooms that newly {label}: {}",
                fresh.join(", ")
            );

            let mut fixed: Vec<&str> = known
                .iter()
                .copied()
                .filter(|id| !measured.contains(id))
                .collect();
            fixed.sort_unstable();
            assert!(
                fixed.is_empty(),
                "these no longer {label} and must leave their list: {}",
                fixed.join(", ")
            );
        }
    }

    #[test]
    fn poke_changes_status_for_every_catalog_room() {
        // Every catalog room must speak after a center poke: first contact and
        // action consequence stay distinct on the status line.
        use crate::room::RoomInput;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert_ne!(
                after, open,
                "{id} is touchable but status does not change after a poke"
            );
        }
    }

    #[test]
    fn action_status_reports_a_measured_quantity() {
        // After a center poke, status must carry at least one digit: a measured
        // consequence (count, coordinate, rule number, ratio), not only words.
        use crate::room::RoomInput;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert!(
                after.chars().any(|c| c.is_ascii_digit()),
                "{id} action status has no measured quantity: {after:?}"
            );
        }
    }

    #[test]
    fn action_status_fits_compact_footer() {
        // Compact App footers have a tight character budget beside fixed
        // controls. Center-poke status should stay within a short line.
        use crate::room::RoomInput;
        const MAX_CHARS: usize = 56;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert!(
                after.chars().count() <= MAX_CHARS,
                "{id} action status is too long for compact footer ({}): {after:?}",
                after.chars().count()
            );
        }
    }

    #[test]
    fn first_contact_status_fits_compact_footer() {
        // Open status shares the same footer budget as action status.
        const MAX_CHARS: usize = 56;
        let mut long = Vec::new();
        for room in all_rooms() {
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let len = open.chars().count();
            if len > MAX_CHARS {
                long.push(format!("{id} ({len}): {open:?}"));
            }
        }
        assert!(
            long.is_empty(),
            "first-contact status too long for compact footer:\n{}",
            long.join("\n")
        );
    }

    #[test]
    fn lookup_by_id_works_and_misses_are_none() {
        assert!(room_by_id("times-tables").is_some());
        assert!(room_by_id("no-such-room").is_none());
    }

    #[test]
    fn varied_lookup_constructs_only_the_requested_replay() {
        let canonical = room_by_id_with("lsystem-garden", 0).expect("canonical room");
        let varied = room_by_id_with("lsystem-garden", 1).expect("varied room");
        assert_ne!(
            render_text(canonical.as_ref(), 0.5),
            render_text(varied.as_ref(), 0.5)
        );
        assert!(room_by_id_with("no-such-room", 1).is_none());
    }

    #[test]
    fn all_rooms_with_variation_produces_different_lsystem() {
        use super::all_rooms_with;
        let r0 = all_rooms_with(0);
        let r1 = all_rooms_with(1);
        assert_eq!(r0.len(), r1.len());
        assert_ne!(
            room_text(&r0, "lsystem-garden", 0.5),
            room_text(&r1, "lsystem-garden", 0.5),
            "registry variation must reach the L-System room"
        );
        assert_ne!(
            room_text(&r0, "quine", 0.6),
            room_text(&r1, "quine", 0.6),
            "registry variation must reach the Quine room"
        );
        assert_ne!(
            room_text(&r0, "double-pendulum", 0.75),
            room_text(&r1, "double-pendulum", 0.75),
            "registry variation must reach animated double-pendulum motion"
        );
        assert_ne!(
            room_text(&r0, "times-tables", 0.2),
            room_text(&r1, "times-tables", 0.2),
            "registry variation must reach Times Tables"
        );
        assert_ne!(
            room_text(&r0, "prime-spirals", 0.3),
            room_text(&r1, "prime-spirals", 0.3),
            "registry variation must reach Prime Spirals"
        );
    }

    #[test]
    fn all_rooms_with_variation_reaches_the_late_variation_rooms() {
        use super::all_rooms_with;
        let r0 = all_rooms_with(0);
        let r42 = all_rooms_with(42);
        for (id, phase) in [
            ("lissajous", 0.35),
            ("harmonograph", 0.4),
            ("logistic-map", 0.3),
            ("the-pour", 0.45),
            ("slope-rider", 0.55),
            ("mobius", 0.35),
            ("zeno", 0.75),
        ] {
            assert_ne!(
                room_text(&r0, id, phase),
                room_text(&r42, id, phase),
                "registry variation must reach {id}"
            );
        }
    }

    #[test]
    fn late_variation_room_seed_zero_matches_default() {
        use crate::rooms::{
            harmonograph::Harmonograph, lissajous::Lissajous, logistic_map::LogisticMap,
            mobius::Mobius, slope_rider::SlopeRider, the_pour::ThePour, zeno::Zeno,
        };
        for (id, phase, default, seeded) in [
            (
                "lissajous",
                0.35,
                Box::new(Lissajous::new()) as Box<dyn Room>,
                Box::new(Lissajous::new_with(0)) as Box<dyn Room>,
            ),
            (
                "harmonograph",
                0.4,
                Box::new(Harmonograph::new()) as Box<dyn Room>,
                Box::new(Harmonograph::new_with(0)) as Box<dyn Room>,
            ),
            (
                "logistic-map",
                0.3,
                Box::new(LogisticMap::new()) as Box<dyn Room>,
                Box::new(LogisticMap::new_with(0)) as Box<dyn Room>,
            ),
            (
                "the-pour",
                0.45,
                Box::new(ThePour::new()) as Box<dyn Room>,
                Box::new(ThePour::new_with(0)) as Box<dyn Room>,
            ),
            (
                "slope-rider",
                0.55,
                Box::new(SlopeRider::new()) as Box<dyn Room>,
                Box::new(SlopeRider::new_with(0)) as Box<dyn Room>,
            ),
            (
                "mobius",
                0.35,
                Box::new(Mobius::new()) as Box<dyn Room>,
                Box::new(Mobius::new_with(0)) as Box<dyn Room>,
            ),
            (
                "zeno",
                0.75,
                Box::new(Zeno::new()) as Box<dyn Room>,
                Box::new(Zeno::new_with(0)) as Box<dyn Room>,
            ),
        ] {
            assert_eq!(
                render_text(default.as_ref(), phase),
                render_text(seeded.as_ref(), phase),
                "{id} seed 0 must preserve the default postcard path"
            );
        }
    }

    #[test]
    fn dynamic_rooms_expose_poke_through_trait_objects() {
        let rooms = all_rooms();
        let julia = rooms
            .iter()
            .find(|room| room.meta().id == "julia")
            .expect("julia must be registered");
        assert_eq!(julia.verb(), Some("CLICK: MORPH C"));
        assert_ne!(
            render_text(julia.as_ref(), 0.35),
            render_poked_text(julia.as_ref(), 0.35, &[(0.9, 0.1)]),
            "Julia poke must dispatch through dyn Room"
        );
    }

    #[test]
    fn every_catalog_room_has_a_structured_motif() {
        for room in all_rooms() {
            let meta = room.meta();
            let motif = room
                .motif()
                .unwrap_or_else(|| panic!("{} must have an Engine A2 motif", meta.id));
            assert!(
                !motif.key.trim().is_empty(),
                "{} motif must name a key",
                meta.id
            );
            assert!(
                motif.root.is_finite() && motif.root > 0.0,
                "{} motif root must be a positive finite frequency",
                meta.id
            );
            assert!(
                (40..=220).contains(&motif.tempo),
                "{} motif tempo must stay playable",
                meta.id
            );
            assert!(
                motif.line.len() >= 6,
                "{} motif must be a phrase, not a sting",
                meta.id
            );
            assert!(
                motif.line.iter().any(|&step| step != 0),
                "{} motif must carry melodic movement",
                meta.id
            );
            assert!(
                !motif.encodes.trim().is_empty(),
                "{} motif must explain the mathematical mapping",
                meta.id
            );
            assert_eq!(
                motif.notation().len(),
                motif.line.len(),
                "{} motif notation must cover the whole phrase",
                meta.id
            );
            assert!(
                motif.pattern().seconds() > 0.0,
                "{} motif must render to a nonempty pattern",
                meta.id
            );
        }
    }

    #[test]
    fn all_rooms_with_variation_affects_poke_rooms() {
        use crate::rooms::{
            chaos_game::ChaosGame, game_of_life::GameOfLife, golden_angle::GoldenAngle,
            langtons_ant::LangtonsAnt, sandpile::Sandpile, strange_loop::StrangeLoop,
            voronoi::Voronoi,
        };
        let c0 = ChaosGame::new_with(0);
        let c1 = ChaosGame::new_with(1);
        let mut ca0 = crate::canvas::Canvas::new(32, 16);
        let mut ca1 = crate::canvas::Canvas::new(32, 16);
        c0.render(&mut ca0, 0.5);
        c1.render(&mut ca1, 0.5);
        assert_ne!(ca0.to_text(), ca1.to_text());
        let g0 = GameOfLife::new_with(0);
        let g1 = GameOfLife::new_with(1);
        let mut ga0 = crate::canvas::Canvas::new(32, 16);
        let mut ga1 = crate::canvas::Canvas::new(32, 16);
        g0.render(&mut ga0, 0.3);
        g1.render(&mut ga1, 0.3);
        assert_ne!(ga0.to_text(), ga1.to_text());
        let v0 = Voronoi::new_with(0);
        let v1 = Voronoi::new_with(1);
        let mut va0 = crate::canvas::Canvas::new(32, 16);
        let mut va1 = crate::canvas::Canvas::new(32, 16);
        v0.render(&mut va0, 0.3);
        v1.render(&mut va1, 0.3);
        assert_ne!(va0.to_text(), va1.to_text());
        // Verify StrangeLoop (self-ref) variation affects render (seed-driven rotation)
        let s0 = StrangeLoop::new_with(0);
        let s1 = StrangeLoop::new_with(1);
        let mut sa0 = crate::canvas::Canvas::new(32, 16);
        let mut sa1 = crate::canvas::Canvas::new(32, 16);
        s0.render(&mut sa0, 0.5);
        s1.render(&mut sa1, 0.5);
        assert_ne!(sa0.to_text(), sa1.to_text());
        // GoldenAngle: variation rotates + jitters seed count for visible per-visit novelty (poke plants respect seed too)
        let ga0 = GoldenAngle::new_with(0);
        let ga42 = GoldenAngle::new_with(42);
        let mut gaa0 = crate::canvas::Canvas::new(32, 16);
        let mut gaa42 = crate::canvas::Canvas::new(32, 16);
        ga0.render(&mut gaa0, 0.0);
        ga42.render(&mut gaa42, 0.0);
        assert_ne!(gaa0.to_text(), gaa42.to_text());
        // LangtonsAnt now has functional variation (initial scatter) + poke pre-integration
        let la0 = LangtonsAnt::new_with(0);
        let la1 = LangtonsAnt::new_with(1);
        let mut laa0 = crate::canvas::Canvas::new(32, 16);
        let mut laa1 = crate::canvas::Canvas::new(32, 16);
        la0.render(&mut laa0, 0.5);
        la1.render(&mut laa1, 0.5);
        assert_ne!(laa0.to_text(), laa1.to_text());
        // Sandpile: variation drifts the ambient pour site so the mandala offsets.
        let sp0 = Sandpile::new_with(0);
        let sp1 = Sandpile::new_with(1);
        let mut spa0 = crate::canvas::Canvas::new(32, 16);
        let mut spa1 = crate::canvas::Canvas::new(32, 16);
        sp0.render(&mut spa0, 0.55);
        sp1.render(&mut spa1, 0.55);
        assert_ne!(spa0.to_text(), spa1.to_text());
    }
}
