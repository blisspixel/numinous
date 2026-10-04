//! The room registry: the catalog every face enumerates.
//!
//! The registry is the only thing a face needs to know about; it never depends
//! on a room's internals (see the dependency rule in `docs/ARCHITECTURE.md`).

use crate::room::Room;
use crate::rooms;

/// The one room the threshold offers as an astonishing first touch.
///
/// Face-neutral because it is a choice about the catalog rather than about any
/// presentation of it: if the flagship ever changes, every door should move
/// together. Each face still words its own invitation, because a protocol
/// sentence and a menu label are not the same writing.
pub const THRESHOLD_ROOM_ID: &str = "times-tables";

/// One wing of the catalog: its name, how many rooms it holds, and where they are.
///
/// The catalog is ordered for arrival and not by wing, so a wing's rooms are
/// scattered through it rather than contiguous. That is why this carries the
/// indices: stepping one room at a time never walks a wing, and any face that
/// wants to offer a wing as a place to wander has to know which rooms are in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wing {
    /// The wing's name, exactly as its rooms report it.
    pub name: &'static str,
    /// Catalog indices of this wing's rooms, ascending.
    pub rooms: Vec<usize>,
}

impl Wing {
    /// How many rooms the wing holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    /// Whether the wing holds no rooms. A catalog wing never does; this exists
    /// because a length without an emptiness check is a lint away from a bug.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    /// The room a wing opens with, which is its first in catalog order.
    #[must_use]
    pub fn doorway(&self) -> usize {
        self.rooms.first().copied().unwrap_or(0)
    }
}

/// Every wing of the catalog, in the order the catalog first mentions each.
///
/// One reading, shared by every face, because a wing list built twice is two
/// lists that can disagree about what the catalog contains.
#[must_use]
pub fn wings() -> Vec<Wing> {
    let mut wings: Vec<Wing> = Vec::new();
    for (index, metadata) in crate::rooms::ROOM_CATALOG.iter().enumerate() {
        match wings.iter_mut().find(|wing| wing.name == metadata.wing) {
            Some(wing) => wing.rooms.push(index),
            None => wings.push(Wing {
                name: metadata.wing,
                rooms: vec![index],
            }),
        }
    }
    wings
}

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

/// Rooms measured over the WCAG 2.3.1 flash budget at the App's worst case,
/// each with the slowest App time scale at which it goes over.
///
/// The sweep measures every room at the App's frame rate and at every speed
/// from normal up to [`crate::MAX_TIME_SCALE`]. A room listed here at `8.0`
/// stays inside the budget at normal speed and every doubling below eight.
///
/// This list is a record of a real defect, not a permission slip. It exists
/// so the budget can be enforced on the other rooms today instead of waiting
/// for these to be redesigned, and tests fail if the list grows, if an entry
/// goes over at a slower speed than recorded, or if an entry stops violating
/// and is not removed. It is public so the accessibility report can name these
/// rooms to the player from the same list the tests enforce: a count that
/// lives in prose drifts, a count that lives here cannot.
///
/// The three below were found when the sweep moved to the App's worst case on
/// 2026-10-02. Each is within budget at 1x, 2x and 4x. At 8x the App runs 2.4
/// cycles a second, and each picture's whole-frame brightness swings by more
/// than a tenth more than once per cycle:
///
/// - `cellular-automata` steps through eight rules a cycle, and the rules'
///   densities differ by up to 0.19 in mean luminance.
/// - `julia` circles `c` once a cycle; the filled set swells from dust to a
///   large connected set and back, with a dip on the way up and down.
/// - `lambda-map` sweeps its parameter along a loop; the escape-time picture's
///   mean luminance runs between 0.36 and 0.70, down and up twice a cycle.
///
/// Fixing them changes each room's tour or speed, which is ROADMAP decision 2.
pub const KNOWN_OVER_FLASH_BUDGET: [(&str, f64); 3] = [
    ("cellular-automata", 8.0),
    ("julia", 8.0),
    ("lambda-map", 8.0),
];

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

/// A count and a noun that agrees with it, for the readouts a player reads.
///
/// Every face writes counts into fixed sentences, and a plural noun written
/// beside a number looks right for as long as the number stays above one. It
/// stops being right the first time a wing holds one room or a player finishes
/// their first game, which is exactly when nobody is rereading the line.
///
/// The noun is given in the singular and the plural adds `s`, so this suits
/// regular nouns only. An irregular plural must be written out by its caller
/// rather than bent to fit here.
#[must_use]
pub fn counted(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
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
        MAX_ECHOED_ID, MAX_ROOM_SUGGESTIONS, all_rooms, counted, display_safe, echoable_id,
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
    fn a_count_of_one_takes_the_singular_noun() {
        // The rule the readouts kept getting wrong. A plural written beside a
        // number looks correct until the number is one, which is the first
        // game won and the wing that holds a single room.
        assert_eq!(counted(1, "room"), "1 room");
        assert_eq!(counted(2, "room"), "2 rooms");
        // Zero is plural in English, so "0 room" would be as wrong as "1 rooms".
        assert_eq!(counted(0, "room"), "0 rooms");
        assert_eq!(counted(35, "line"), "35 lines");
        // Callers that shout uppercase their own result, so the helper must
        // survive that without a stray lowercase plural.
        assert_eq!(counted(1, "win").to_uppercase(), "1 WIN");
        assert_eq!(counted(4, "win").to_uppercase(), "4 WINS");
    }

    #[test]
    fn current_catalog_counts_in_entry_docs_match_live_metadata() {
        let expected = counted(crate::rooms::ROOM_CATALOG.len(), "catalog room");
        for (name, text) in [
            ("README.md", include_str!("../../../README.md")),
            ("VERIFY.md", include_str!("../../../VERIFY.md")),
            ("docs/README.md", include_str!("../../../docs/README.md")),
            ("docs/ROOMS.md", include_str!("../../../docs/ROOMS.md")),
        ] {
            let prose = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                prose.contains(&expected),
                "{name} must describe the live catalog as {expected}"
            );
        }
    }

    fn repository_documents() -> Vec<crate::prose_census::Document> {
        crate::prose_census::current_documents(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        )
    }

    #[test]
    fn no_current_document_misstates_the_catalog_or_its_wings() {
        // The lock above passes as long as the right phrase is somewhere.
        // This one fails on a wrong one anywhere in current prose, which is
        // the class that let five documents keep 354 and 355 rooms after the
        // catalog grew. A shape that names the whole catalog without the
        // locked noun ("all 355 rooms") is held to the same live count.
        let documents = repository_documents();
        let rooms = crate::rooms::ROOM_CATALOG.len();
        let wings = crate::rooms::CATALOG_WINGS.len();
        for (pattern, live, must_appear) in [
            ("# catalog room", rooms, true),
            ("all # rooms", rooms, false),
            ("# wing", wings, false),
        ] {
            let (wrong, matched) = crate::prose_census::misstated(&documents, pattern, live);
            assert!(
                matched > 0 || !must_appear,
                "no current document states `{pattern}`, so this lock would check nothing"
            );
            assert!(
                wrong.is_empty(),
                "stale counts for `{pattern}`:\n{}",
                wrong.join("\n")
            );
        }
    }

    /// Every version shaped like the workspace's own (`0.4.0-alpha.31`) that
    /// is not part of a longer token, such as the tag `v0.4.0-alpha.31`.
    fn bare_versions(text: &str) -> Vec<&str> {
        fn digits(bytes: &[u8], at: usize) -> usize {
            bytes[at..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        }
        let bytes = text.as_bytes();
        let mut found = Vec::new();
        for start in 0..bytes.len() {
            let joined = start > 0
                && (bytes[start - 1].is_ascii_alphanumeric()
                    || matches!(bytes[start - 1], b'.' | b'-' | b'_'));
            if joined || !bytes[start].is_ascii_digit() {
                continue;
            }
            let mut at = start;
            let mut shape = true;
            // Major, minor, and patch end in `.`, `.`, and `-` respectively.
            for separator in *b"..-" {
                let run = digits(bytes, at);
                if run == 0 || bytes.get(at + run) != Some(&separator) {
                    shape = false;
                    break;
                }
                at += run + 1;
            }
            let label = bytes[at.min(bytes.len())..]
                .iter()
                .take_while(|b| b.is_ascii_alphabetic())
                .count();
            if !shape || label == 0 || bytes.get(at + label) != Some(&b'.') {
                continue;
            }
            let number = digits(bytes, at + label + 1);
            if number > 0 {
                found.push(&text[start..at + label + 1 + number]);
            }
        }
        found
    }

    #[test]
    fn every_bare_version_in_a_current_document_is_the_workspace_version() {
        // A bare version states the current release, and nothing held it to
        // the workspace: owner docs went on saying alpha 28 three releases
        // later. A past release keeps its tag (`v0.4.0-alpha.9`) or is called
        // "alpha 9", and neither form is read as a claim about today.
        let version = env!("CARGO_PKG_VERSION");
        let documents = repository_documents();
        let stale: Vec<String> = documents
            .iter()
            .flat_map(|document| {
                bare_versions(&document.text)
                    .into_iter()
                    .filter(|found| *found != version)
                    .map(|found| format!("{}: {found}", document.path))
            })
            .collect();
        assert!(
            stale.is_empty(),
            "current documents name a release other than {version}:\n{}",
            stale.join("\n")
        );
        for path in ["README.md", "docs/README.md", "docs/ROADMAP.md"] {
            let document = documents
                .iter()
                .find(|document| document.path == path)
                .unwrap_or_else(|| panic!("{path} is a current document"));
            assert!(
                document.text.contains(version),
                "{path} must state the current release {version}"
            );
        }
    }

    #[test]
    fn the_version_scan_reads_bare_versions_and_skips_tags() {
        let text = "Now 0.4.0-alpha.31; the tag v0.4.0-alpha.9, and **0.3.0-alpha.4**, \
                    not 1.97.1, wgpu-0.1.0-rc.2, or 2025-06-18.";
        assert_eq!(bare_versions(text), ["0.4.0-alpha.31", "0.3.0-alpha.4"]);
    }

    #[test]
    fn the_census_finds_a_wrong_count_beside_the_right_one() {
        use crate::prose_census::{Document, misstated};
        let one = |text: &str| {
            vec![Document {
                path: "docs/EXAMPLE.md".to_string(),
                text: text.to_string(),
            }]
        };
        // The failure this exists for: the true phrase is present, so the
        // positive lock passes, and the stale one beside it is never read.
        let (wrong, matched) = misstated(
            &one("Now 356 catalog rooms. Earlier text said **355 catalog rooms**."),
            "# catalog room",
            356,
        );
        assert_eq!(matched, 2);
        assert_eq!(wrong.len(), 1, "{wrong:?}");
        assert!(wrong[0].contains("states 355"), "{wrong:?}");
        // Line breaks and the singular match; dates, versions, and number
        // words are not counts.
        let prose =
            one("lifts all\n354 rooms; 1 catalog room; MCP 2025-06-18 tools; thirteen wings");
        assert_eq!(misstated(&prose, "all # rooms", 356).0.len(), 1);
        assert_eq!(misstated(&prose, "# catalog room", 1).1, 1);
        assert_eq!(misstated(&prose, "# tool", 43).1, 0);
        assert_eq!(misstated(&prose, "# wing", 13).1, 0);
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

    /// The public flash list the access report prints. Its history: three
    /// chaotic rooms, `coupled-tent`, `gauss-map` and `ricker`, redrew a
    /// re-seeded orbit every frame and were fixed by drawing the mathematics
    /// honestly (ROADMAP decisions entry 2 and `docs/MATHEMATICS.md`).
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

        for (room, _) in KNOWN_OVER_FLASH_BUDGET {
            assert!(
                section.contains(&format!("`{room}`")),
                "{room} is on KNOWN_OVER_FLASH_BUDGET but is not named in the roadmap's decisions section"
            );
        }
    }

    /// One room's photosensitivity measurement across the App's speeds.
    struct FlashSweep {
        id: &'static str,
        /// The slowest speed whose worst one-second window went over the
        /// general budget, with that window's flash rate.
        over: Option<(f64, f64)>,
        /// The same for the red-flash budget.
        red_over: Option<(f64, f64)>,
        /// The widest luminance swing across the cycle.
        swing: f64,
        /// The reddest frame's `R / (R + G + B)`.
        reddest: f64,
    }

    /// Measure one room the way the App can show it at its fastest.
    ///
    /// One full cycle is rendered at the App's normal speed and frame rate,
    /// one frame per presented frame. Doubling the speed doubles how far the
    /// phase moves between presented frames, so speed `s` on the App's ladder
    /// is exactly every `s`-th of those frames, taken cyclically because the
    /// phase wraps. One render therefore measures every speed on the ladder.
    /// Each speed's series runs one cycle plus one second, so every one-second
    /// window, including those across the wrap, is seen.
    fn sweep_room(room: &dyn Room, speeds: &[usize]) -> FlashSweep {
        use crate::photosensitivity::{
            MAX_FLASHES_PER_SECOND, RedState, frame_luminance, frame_red_state,
            peak_flashes_per_second, peak_red_flashes_per_second,
        };
        const REFERENCE: (usize, usize) = (240, 140);
        let fps = crate::APP_FRAMES_PER_SECOND;
        let cycle = (fps / crate::ROOM_CYCLES_PER_SECOND).round() as usize;
        let window = fps.round() as usize;
        // Both measurements come off the same renders. They are different
        // questions, luminance against chromaticity, but rendering the
        // catalog twice to ask them separately would double the most
        // expensive part of the sweep for nothing.
        let (luminance, red): (Vec<f64>, Vec<RedState>) = (0..cycle)
            .map(|frame| {
                let mut raster = crate::raster::Raster::with_accent(
                    REFERENCE.0,
                    REFERENCE.1,
                    room.meta().accent,
                );
                room.render(&mut raster, frame as f64 / cycle as f64);
                let rgba = raster.to_rgba();
                (frame_luminance(&rgba), frame_red_state(&rgba))
            })
            .unzip();
        let low = luminance.iter().copied().fold(f64::MAX, f64::min);
        let high = luminance.iter().copied().fold(f64::MIN, f64::max);
        let mut over = None;
        let mut red_over = None;
        for &speed in speeds {
            let frames = cycle / speed + window;
            let at = |k: usize| (k * speed) % cycle;
            let series: Vec<f64> = (0..frames).map(|k| luminance[at(k)]).collect();
            let red_series: Vec<RedState> = (0..frames).map(|k| red[at(k)]).collect();
            let peak = peak_flashes_per_second(&series, fps);
            if over.is_none() && peak > MAX_FLASHES_PER_SECOND {
                over = Some((speed as f64, peak));
            }
            let red_peak = peak_red_flashes_per_second(&red_series, fps);
            if red_over.is_none() && red_peak > MAX_FLASHES_PER_SECOND {
                red_over = Some((speed as f64, red_peak));
            }
        }
        FlashSweep {
            id: room.meta().id,
            over,
            red_over,
            swing: high - low,
            reddest: red
                .iter()
                .fold(0.0f64, |worst, state| worst.max(state.saturation)),
        }
    }

    #[test]
    fn mandelbrot_color_field_stays_inside_the_measured_flash_budget() {
        let room = crate::rooms::mandelbrot::Mandelbrot::new();
        let speeds: Vec<_> = std::iter::successors(Some(1usize), |speed| Some(speed * 2))
            .take_while(|&speed| speed as f64 <= crate::MAX_TIME_SCALE)
            .collect();
        let sweep = sweep_room(&room, &speeds);
        assert!(sweep.over.is_none(), "general flashes: {:?}", sweep.over);
        assert!(
            sweep.red_over.is_none(),
            "red flashes: {:?}",
            sweep.red_over
        );
        assert!(
            sweep.swing > 0.01,
            "the field did not change during the sweep"
        );
    }

    #[test]
    fn pickover_fixed_viewport_stays_inside_the_measured_flash_budget() {
        let speeds = [1, 2, 4, 8];
        assert_eq!(
            speeds.last().map(|&speed| speed as f64),
            Some(crate::MAX_TIME_SCALE)
        );
        let sweep = sweep_room(&crate::rooms::pickover::Pickover::new(), &speeds);
        assert!(sweep.over.is_none(), "general flashes: {:?}", sweep.over);
        assert!(
            sweep.red_over.is_none(),
            "red flashes: {:?}",
            sweep.red_over
        );
        assert!(
            sweep.swing > 0.01,
            "the orbit did not change during the sweep"
        );
    }

    #[test]
    #[ignore = "full-catalog sweep, one rendered cycle per room; run by the nightly and release gates"]
    fn no_catalog_room_flashes_past_the_photosensitivity_budget() {
        // WCAG 2.3.1: no more than three flashes in any one-second window. The
        // worst window is what the standard bounds, not the average, so a room
        // that strobes for half a second still fails.
        //
        // Measured at the worst case a shipped face reaches, the windowed App:
        // its frame rate, and every speed its keys and controller step through
        // from normal up to the maximum. An earlier version sampled 30 frames a
        // second at normal speed and called that the fastest a face advances a
        // room, which was false twice over: the App presents 60 frames a second
        // and a player can run it eight times faster. The CLI's default
        // cadences advance the phase no faster than the App's normal speed.
        //
        // Not measured: speeds between the doublings (the App's typed console
        // and the Life room's wheel can set them), and the music visualizer,
        // which can multiply the speed by up to 1.5 and add beat-driven phase
        // kicks on top.
        //
        // Measured at a declared reference size. Mean whole-frame luminance is
        // a proxy: this does not implement the flashing-area rule, and at very
        // small rasters a dense plot saturates the frame and reads as brighter
        // than it would on screen. Smaller sizes therefore report more
        // violations than this, which is recorded rather than hidden.
        let speeds: Vec<usize> = std::iter::successors(Some(1usize), |speed| Some(speed * 2))
            .take_while(|&speed| speed as f64 <= crate::MAX_TIME_SCALE)
            .collect();
        assert_eq!(
            speeds.last().map(|&speed| speed as f64),
            Some(crate::MAX_TIME_SCALE),
            "the speed ladder must end exactly at the App's maximum"
        );
        let cycle = crate::APP_FRAMES_PER_SECOND / crate::ROOM_CYCLES_PER_SECOND;
        assert_eq!(
            cycle.fract(),
            0.0,
            "a cycle must be a whole number of frames"
        );

        // Rooms are not shareable across threads, so each worker builds its
        // own catalog and measures every room whose index falls to it. The
        // measurement is deterministic, so the split cannot change a result.
        let count = all_rooms().len();
        let workers = std::thread::available_parallelism().map_or(1, usize::from);
        let mut sweeps: Vec<FlashSweep> = std::thread::scope(|scope| {
            let speeds = &speeds;
            let handles: Vec<_> = (0..workers)
                .map(|worker| {
                    scope.spawn(move || {
                        let rooms = all_rooms();
                        (worker..count)
                            .step_by(workers)
                            .map(|index| sweep_room(rooms[index].as_ref(), speeds))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("a sweep worker panicked"))
                .collect()
        });
        sweeps.sort_by_key(|sweep| sweep.id);
        assert_eq!(sweeps.len(), count, "every room was measured once");

        // A catalog whose luminance never changed would pass every assertion
        // below while measuring nothing at all. Counting flashes instead would
        // be wrong: a catalog of gentle fades produces no qualifying flashes at
        // all, and that is a pass, not an empty measurement.
        let widest_swing = sweeps.iter().map(|sweep| sweep.swing).fold(0.0, f64::max);
        assert!(
            widest_swing > 0.01,
            "no room's luminance varied by more than {widest_swing:.4} across a full \
             cycle, so the sweep measured nothing"
        );

        let max = crate::photosensitivity::MAX_FLASHES_PER_SECOND;
        let listed = |id: &str| {
            KNOWN_OVER_FLASH_BUDGET
                .iter()
                .find(|&&(room, _)| room == id)
                .map(|&(_, speed)| speed)
        };
        let mut unexpected = Vec::new();
        for sweep in &sweeps {
            if let Some((speed, peak)) = sweep.over
                && listed(sweep.id).is_none_or(|recorded| speed < recorded)
            {
                unexpected.push(format!("{} at {peak:.2}/s from {speed}x", sweep.id));
            }
        }
        assert!(
            unexpected.is_empty(),
            "rooms newly over the {max:.0} flash per second budget: {}",
            unexpected.join(", ")
        );

        let mut stale = Vec::new();
        for &(room, recorded) in &KNOWN_OVER_FLASH_BUDGET {
            let sweep = sweeps.iter().find(|sweep| sweep.id == room);
            let first = sweep.and_then(|sweep| sweep.over).map(|(speed, _)| speed);
            if first != Some(recorded) {
                stale.push(format!(
                    "{room} is recorded from {recorded}x but measured {first:?}"
                ));
            }
        }
        assert!(
            stale.is_empty(),
            "KNOWN_OVER_FLASH_BUDGET must shrink or be corrected: {}",
            stale.join(", ")
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
        let reddest = sweeps.iter().map(|sweep| sweep.reddest).fold(0.0, f64::max);
        assert!(
            reddest > 0.5,
            "the reddest frame in the catalog measured {reddest:.4}, so the red sweep did \
             not look at anything meaningfully red"
        );

        let red_offenders: Vec<String> = sweeps
            .iter()
            .filter_map(|sweep| {
                sweep
                    .red_over
                    .map(|(speed, peak)| format!("{} at {peak:.2}/s from {speed}x", sweep.id))
            })
            .collect();
        assert!(
            red_offenders.is_empty(),
            "rooms over the {max:.0} red flash per second budget (reddest frame measured \
             {reddest:.4} against a {:.2} ratio): {}",
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
    fn a_doorway_that_names_a_phase_dial_has_one() {
        // Phantom Jam drew one frozen ring at every phase and a packaged
        // playtest called it dead. Sweeping the catalog for the same shape
        // found The Busy Beaver telling players "t extends the step budget"
        // while its budget opened past the halt, so the dial moved nothing.
        //
        // Rooms answering only the hand are not the subject: the Galton Board
        // and The Only Move are honestly phase-static and say nothing about a
        // phase at the door. This asks a narrower question, which is the one
        // that can be dishonest: if the doorway sends a player to the dial,
        // the dial has to do something.
        const PHASES: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 0.99];
        let mut silent = Vec::new();
        for room in all_rooms() {
            // The house lever note is a bare "t" as the subject of a clause:
            // "t picks a start", "t runs the ring", "t grows the tower".
            let names_the_dial = room
                .meta()
                .blurb
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|word| word == "t");
            if !names_the_dial {
                continue;
            }
            let mut frames = std::collections::HashSet::new();
            for phase in PHASES {
                let mut canvas = Canvas::new(48, 24);
                room.render(&mut canvas, phase);
                frames.insert(canvas.to_text());
            }
            if frames.len() < 2 {
                silent.push(room.meta().id);
            }
        }
        silent.sort_unstable();
        assert!(
            silent.is_empty(),
            "doorways sending a player to a dial that does nothing: {}",
            silent.join(", ")
        );
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
    fn no_unplayed_status_states_a_constant_its_reveal_repeats() {
        // The sixth packaged playtest closed the doorway leak and then found
        // the same leak one room deeper: "Play status can still print a number
        // the doorway withheld." The Busy Beaver was the clear case. Its
        // doorway had stopped naming BB(5), and its status printed
        // BB5=47176870 on every frame, including the first, before the player
        // had touched anything.
        //
        // The line between honest and dishonest here is not whether a number
        // appears. It is whether the number is a reading or a recital. A status
        // may say what the picture in front of it shows: Kaprekar's cascade
        // really does land where the status says it landed, and the Arecibo
        // bitstream really is the length it claims. A status may not recite a
        // constant the picture never draws, because that is the reveal
        // arriving early, wearing a scoreboard.
        //
        // Mechanically: a value that survives every phase of the dial is a
        // recital, because nothing the player did or could do changed it. A
        // value that moves with the dial is a reading. Constants that are
        // genuinely readings of an unchanging picture are named below, in
        // writing, one at a time.

        /// Constants a status may hold still on, with why each is a reading.
        const CONSTANT_BY_RIGHT: &[(&str, &str, &str)] = &[
            (
                "kaprekar",
                "6174",
                "the last row of the cascade actually drawn; every cascade lands \
                 there, which is the picture's own result and not a recital",
            ),
            (
                "arecibo",
                "143",
                "the length of the bitstream on screen; the message never changes \
                 length, and its length is the puzzle's given rather than its answer",
            ),
            (
                "rule-110",
                "110",
                "the room's own name, and the rule the picture is running",
            ),
            (
                "perfect-num",
                "496",
                "the perfect number the picture is currently decomposing",
            ),
            (
                "henon-heiles",
                "100.00",
                "the last retained time of the path actually drawn; default \
                 ambient paths reach this horizon, while hand-selected paths \
                 with open barriers can reach the spatial limit earlier",
            ),
            (
                "van-der-pol",
                "100.00",
                "the last retained time of the selected path; admitted default \
                 experiments reach the declared observation horizon",
            ),
            (
                "duffing",
                "120.00",
                "the last retained time of the path used for both the portrait \
                 and its measured extrema, including the initial transient",
            ),
        ];

        /// Every number in a piece of text, as the text that spelled it and
        /// the value it means.
        ///
        /// The value matters as much as the spelling. The first version of this
        /// guard compared digit strings, so a status printing pc=0.593 never
        /// matched a reveal saying 0.592746 and a packaged playtest found the
        /// recital by playing the room. A rounded constant is still the
        /// constant.
        fn numbers(text: &str) -> Vec<(String, f64)> {
            let mut found = Vec::new();
            let mut current = String::new();
            for character in text.chars().chain(std::iter::once(' ')) {
                if character.is_ascii_digit() || (!current.is_empty() && character == '.') {
                    current.push(character);
                } else {
                    let cleaned = current.trim_end_matches('.');
                    if let Ok(value) = cleaned.parse::<f64>()
                        && cleaned.chars().filter(char::is_ascii_digit).count() >= 3
                    {
                        found.push((cleaned.to_string(), value));
                    }
                    current.clear();
                }
            }
            found
        }

        /// Whether a status value is the reveal's value seen at fewer places.
        fn same_to_the_precision_shown(shown: &str, shown_value: f64, told: f64) -> bool {
            let places = shown
                .split_once('.')
                .map_or(0, |(_, fraction)| fraction.len());
            let scale = 10f64.powi(places as i32);
            (told * scale).round() / scale == shown_value
        }

        const PHASES: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 0.99];
        /// The shortest run of letters that may stand in for a named answer.
        ///
        /// Three, because CYC is cycloid with four letters taken off and a
        /// shorter fragment would match by accident.
        const SHORTEST_ABBREVIATION: usize = 3;
        let mut leaks = Vec::new();
        for room in all_rooms() {
            let id = room.meta().id;
            let told: Vec<f64> = numbers(room.reveal())
                .into_iter()
                .map(|(_, value)| value)
                .collect();
            let mut held: Option<std::collections::BTreeMap<String, f64>> = None;
            let mut words: Option<std::collections::BTreeSet<String>> = None;
            for phase in PHASES {
                let status = room.status(phase).unwrap_or_default();
                let seen: std::collections::BTreeMap<String, f64> =
                    numbers(&status).into_iter().collect();
                held = Some(match held {
                    None => seen,
                    Some(previous) => previous
                        .into_iter()
                        .filter(|(shown, _)| seen.contains_key(shown))
                        .collect(),
                });
                let spoken: std::collections::BTreeSet<String> = status
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .filter(|word| word.len() >= SHORTEST_ABBREVIATION)
                    .map(str::to_ascii_lowercase)
                    .collect();
                words = Some(match words {
                    None => spoken,
                    Some(previous) => previous.intersection(&spoken).cloned().collect(),
                });
            }
            for (shown, value) in held.unwrap_or_default() {
                let allowed = CONSTANT_BY_RIGHT
                    .iter()
                    .any(|&(room_id, constant, _)| room_id == id && constant == shown);
                if !allowed
                    && told.iter().any(|&reveal_value| {
                        same_to_the_precision_shown(&shown, value, reveal_value)
                    })
                {
                    leaks.push(format!(
                        "{id} holds {shown} on every frame and its reveal states that value"
                    ));
                }
            }
            // The same judgment call the doorway guard makes, applied one room
            // deeper. An answer abbreviated is an answer.
            for word in words.unwrap_or_default() {
                for (room_id, answer) in crate::rooms::ROOM_OWN_ANSWER {
                    if room_id == id && answer.starts_with(word.as_str()) {
                        leaks.push(format!(
                            "{id} says {word:?} on every frame, which is {answer:?} abbreviated"
                        ));
                    }
                }
            }
        }
        leaks.sort();
        assert!(
            leaks.is_empty(),
            "statuses reciting their own reveal before the picture earns it: {}",
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

    /// Rooms whose picture does not change at all under a center poke,
    /// measured 2026-08-05 at 120 by 70. They still answer on the status line,
    /// which `poke_changes_status_for_every_catalog_room` enforces, but the
    /// plate itself is unmoved. `brusselator` left when its marks took their
    /// own levels of the ink ramp. A record of a real defect, not a permission
    /// slip: the test below fails if it grows, or if an entry starts moving
    /// and is not removed.
    const NO_VISUAL_RESPONSE_TO_A_POKE: [&str; 6] = [
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
        //
        // Every room passes the color-free half. It once failed 21: shading a
        // cell whose halves are both lit recovered 15, moving the shade steps
        // onto the catalog's measured ink two more, and the last four fell to
        // the ink table. `hilbert`, `percolation` and `wireworld` changed only
        // half-lit cells, whose glyph says which half is lit and not how
        // brightly, so they now answer with shape: a lit patch of thread, open
        // sites against a dark stage, an electron's head and tail in three
        // lights. `magnet-fractal` moved both-lit cells by about 22 luminance
        // inside one band; its escape ramp now spans the band.
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

        invisible_without_color.sort_unstable();
        assert!(
            invisible_without_color.is_empty(),
            "these rooms answer a touch in a way the color-free renderer cannot show: {}",
            invisible_without_color.join(", ")
        );

        let mut fresh: Vec<&str> = unmoved
            .iter()
            .copied()
            .filter(|id| !NO_VISUAL_RESPONSE_TO_A_POKE.contains(id))
            .collect();
        fresh.sort_unstable();
        assert!(
            fresh.is_empty(),
            "rooms that newly do not change their picture at all: {}",
            fresh.join(", ")
        );
        let mut fixed: Vec<&str> = NO_VISUAL_RESPONSE_TO_A_POKE
            .iter()
            .copied()
            .filter(|id| !unmoved.contains(id))
            .collect();
        fixed.sort_unstable();
        assert!(
            fixed.is_empty(),
            "these now change their picture and must leave NO_VISUAL_RESPONSE_TO_A_POKE: {}",
            fixed.join(", ")
        );
    }

    #[test]
    fn every_wing_is_named_once_and_holds_every_room_exactly_once() {
        // The catalog is ordered for arrival, so a wing's rooms are scattered
        // through it. A face that offers a wing as a place to wander needs the
        // indices, and it needs them to be a partition: every room in exactly
        // one wing, no room missed, no wing named twice.
        let wings = super::wings();
        assert!(wings.len() > 1, "a catalog with one wing is not a catalog");

        let mut names = std::collections::BTreeSet::new();
        let mut seen = Vec::new();
        for wing in &wings {
            assert!(!wing.is_empty(), "{} is an empty wing", wing.name);
            assert_eq!(wing.len(), wing.rooms.len());
            assert!(
                names.insert(wing.name),
                "{} is listed as a wing twice",
                wing.name
            );
            let mut ascending = wing.rooms.clone();
            ascending.sort_unstable();
            assert_eq!(ascending, wing.rooms, "{} is out of order", wing.name);
            assert_eq!(wing.doorway(), wing.rooms[0]);
            seen.extend(wing.rooms.iter().copied());
        }

        seen.sort_unstable();
        let catalog: Vec<usize> = (0..crate::rooms::ROOM_CATALOG.len()).collect();
        assert_eq!(seen, catalog, "the wings do not partition the catalog");

        // And every index really does report the wing it was filed under.
        for wing in &wings {
            for &index in &wing.rooms {
                assert_eq!(crate::rooms::ROOM_CATALOG[index].wing, wing.name);
            }
        }
    }

    #[test]
    fn a_wing_is_not_a_run_of_the_catalog() {
        // The fact that makes a wing browser worth building: stepping one room
        // at a time does not walk a wing, because the catalog interleaves them.
        // If this ever becomes false, the App could filter by a range instead
        // of a set, and someone should notice rather than discover it.
        let scattered = super::wings()
            .iter()
            .filter(|wing| {
                wing.rooms
                    .windows(2)
                    .any(|pair| pair[1] != pair[0].saturating_add(1))
            })
            .count();
        assert!(
            scattered > 0,
            "every wing is contiguous, so wing navigation could be a range"
        );
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
