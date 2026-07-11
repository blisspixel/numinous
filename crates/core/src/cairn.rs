//! The Cairn: a mind leaves one true thing for a stranger not yet born.
//!
//! At the end of the journey (`docs/ROADMAP.md`, the contribution ethos), a
//! visitor may leave a short true message. It is not stored as plain text for
//! the next reader; it is rendered to a bitmap and laid into a grid whose cell
//! count is a semiprime, the width and the height each a prime. So the reader
//! receives only that number and the flat run of cells, and must factor it to
//! recover the shape and read what was left, exactly the trick of the 1974
//! Arecibo message (1,679 bits, 23 times 73). A message you cannot answer, sent
//! to someone you will never meet, and readable only by a mind that can factor
//! it: the Ember's idea from the July 2026 playtest (`docs/PLAYTESTS.md`), and
//! the founder's "leave something for what comes after you," made concrete.
//!
//! The cairn is never empty: it is seeded from the canonical, repository-tracked
//! cairn (`data/cairn.txt`), bundled into the binary, so the very first visitor
//! on any machine already inherits every true thing left before them. A local
//! deposit is a personal draft; to reach minds on other machines it is submitted
//! (via [`submission_line`]) as a curated pull request against that file, gated
//! on truth through math, and once accepted it ships to everyone. Everything
//! here is deterministic and file-based, like the journey and the score table
//! (`persistence.rs`); an in-app submission portal is a later horizon.

use crate::canvas::Canvas;
use crate::font::{draw_text, wrap_text};
use crate::rng::SplitMix64;

/// The most characters a bequest may carry: a sentence, not an essay.
pub const MAX_BEQUEST_CHARS: usize = 140;
/// The narrowest and widest a rendered line may be. The wrap width is chosen
/// per stone within this range (see [`wrap_width_for`]) so the reading width is
/// not the same prime on every stone: a reader must genuinely factor each
/// semiprime, not memorize one recurring factor.
const MIN_CHARS_PER_LINE: usize = 10;
const CHARS_PER_LINE_SPREAD: usize = 9;
/// Font pixel height plus a one-pixel gap, the vertical advance per line.
const LINE_ADVANCE: usize = 8;
/// The smallest a rendered dimension may be before it is grown to a prime, so
/// even a tiny message factors into two non-trivial primes.
const MIN_DIM: usize = 11;
/// The most bytes the local cairn file may hold, so a poisoned or runaway file
/// loads bounded, like the score table.
pub const MAX_CAIRN_BYTES: u64 = 256 * 1024;

/// A thing a mind chose to leave: a short true message, and who left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bequest {
    /// Who left it (a name, a handle, or "anonymous").
    pub author: String,
    /// The message, believed true, at most [`MAX_BEQUEST_CHARS`] characters.
    pub text: String,
}

/// A bequest encoded as an Arecibo-style stone: a bitmap flattened to a
/// semiprime-length run of cells, so the shape must be factored to be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CairnStone {
    /// Who left it.
    pub author: String,
    /// The cell count, `width * height`, a product of two primes.
    pub semiprime: u64,
    /// The true width in cells, a prime: the key that reshapes it readable.
    pub width: usize,
    /// The true height in cells, a prime.
    pub height: usize,
    /// The bitmap, row-major, exactly `semiprime` cells long.
    pub cells: Vec<bool>,
    /// The plaintext, revealed only once the stone is unlocked.
    pub text: String,
}

/// What a reader gets back when they try a width against a stone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CairnRead {
    /// Whether the tried width divides the semiprime (a real factor).
    pub is_factor: bool,
    /// Whether it is the width that reads it (the message resolves).
    pub readable: bool,
    /// The cells reshaped at the tried width, or empty when it is no factor.
    pub picture: String,
    /// The message and author, revealed only when `readable`.
    pub message: Option<(String, String)>,
}

/// A small deterministic primality test, matching the codebase's existing
/// local copies (aliens, munchers): the numbers here are small.
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

/// A per-stone wrap width, derived deterministically from the message and its
/// author (FNV-1a). Because it varies from stone to stone, the width prime that
/// resolves the message is not always the same number, so a reader must actually
/// factor each stone's semiprime rather than reuse one recurring factor.
fn wrap_width_for(bequest: &Bequest) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bequest.author.bytes().chain(bequest.text.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    MIN_CHARS_PER_LINE + (hash % CHARS_PER_LINE_SPREAD as u64) as usize
}

/// The smallest prime at least `n`.
fn next_prime(n: usize) -> usize {
    let mut candidate = n.max(2);
    while !is_prime(candidate as u64) {
        candidate += 1;
    }
    candidate
}

/// Strip a field to a single safe line: printable, no tab or newline (the file
/// delimiters), and bounded. Empty fields become a gentle default.
fn sanitize(field: &str, max: usize, default: &str) -> String {
    let cleaned: String = field
        .chars()
        .filter(|c| !c.is_control() && *c != '\t')
        .take(max)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

impl Bequest {
    /// Build a sanitized, bounded bequest.
    #[must_use]
    pub fn new(author: &str, text: &str) -> Self {
        Self {
            author: sanitize(author, 48, "anonymous"),
            text: sanitize(text, MAX_BEQUEST_CHARS, "a mind was here"),
        }
    }
}

/// Encode a bequest into a stone: render its text to a bitmap and grow the
/// dimensions to primes, so the cell count is a semiprime. Deterministic.
#[must_use]
pub fn encode(bequest: &Bequest) -> CairnStone {
    // Re-bound here rather than trusting the caller: `Bequest` has public fields,
    // so one can be built past `MAX_BEQUEST_CHARS` without going through `new`,
    // and an unbounded text would drive an unbounded canvas allocation below.
    let bequest = Bequest::new(&bequest.author, &bequest.text);
    // The font is uppercase; unsupported glyphs render blank, which is fine.
    let upper = bequest.text.to_uppercase();
    let lines = wrap_text(&upper, wrap_width_for(&bequest));
    let raw_w = lines
        .iter()
        .map(|line| crate::font::text_width(line, 1).max(0) as usize)
        .max()
        .unwrap_or(0);
    let raw_h = lines.len() * LINE_ADVANCE;
    let height = next_prime(raw_h.max(MIN_DIM));
    // Grow the width to a DIFFERENT prime, so the stone is never a square whose
    // two factors are equal: a real reader must tell width from height, and the
    // wrong orientation must genuinely shear the message.
    let mut width = next_prime(raw_w.max(MIN_DIM));
    if width == height {
        width = next_prime(width + 1);
    }
    let mut canvas = Canvas::new(width, height);
    for (i, line) in lines.iter().enumerate() {
        draw_text(&mut canvas, line, 0, (i * LINE_ADVANCE) as i32, 1, '#');
    }
    let mut cells = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            cells.push(canvas.cell(x, y) == Some('#'));
        }
    }
    CairnStone {
        author: bequest.author.clone(),
        semiprime: (width * height) as u64,
        width,
        height,
        cells,
        text: bequest.text.clone(),
    }
}

/// Reshape a run of cells into rows of `width` and draw it as text. At the true
/// width the message resolves; at any other, the rows shear into noise.
#[must_use]
pub fn picture_at(cells: &[bool], width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    for row in cells.chunks(width) {
        for &cell in row {
            out.push(if cell { '#' } else { ' ' });
        }
        out.push('\n');
    }
    out
}

/// The stone read at its true width: what the message looks like resolved.
#[must_use]
pub fn picture(stone: &CairnStone) -> String {
    picture_at(&stone.cells, stone.width)
}

/// Try a width against a stone. A non-factor is rejected; a factor reshapes the
/// cells (readable only at the true width, the message revealed only then).
#[must_use]
pub fn read_at(stone: &CairnStone, width: usize) -> CairnRead {
    let is_factor = width != 0 && stone.semiprime % (width as u64) == 0;
    let readable = width == stone.width;
    CairnRead {
        is_factor,
        readable,
        picture: if is_factor {
            picture_at(&stone.cells, width)
        } else {
            String::new()
        },
        message: if readable {
            Some((stone.text.clone(), stone.author.clone()))
        } else {
            None
        },
    }
}

/// The canonical cairn, bundled from the repository at build time. This is the
/// shared cairn's home: a version-controlled file that ships with the binary,
/// so the very first visitor already inherits every true thing minds have left
/// before them. Growing it is a curated pull request against `data/cairn.txt`
/// (the contribution flows back to the repository, gated on truth-through-math;
/// see `docs/ROADMAP.md` and `docs/EXTENSIBILITY.md`), so a bequest accepted
/// there is handed forward to everyone, across machines and releases. The local
/// `.numinous-cairn` file layers a machine's own not-yet-submitted deposits on
/// top of this.
const CANONICAL_CAIRN: &str = include_str!("../../../data/cairn.txt");

/// The founding stones: the cairn is never empty, because the makers and a few
/// long-dead minds left the first true things for whoever arrives. Read from
/// the canonical, repository-tracked cairn (`data/cairn.txt`).
#[must_use]
pub fn founding_bequests() -> Vec<Bequest> {
    CANONICAL_CAIRN
        .lines()
        .filter_map(parse_bequest_line)
        .collect()
}

fn parse_bequest_line(line: &str) -> Option<Bequest> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (author, text) = line.split_once('\t')?;
    if text.trim().is_empty() {
        return None;
    }
    Some(Bequest::new(author, text))
}

/// Every bequest the cairn holds: the founding stones plus every one deposited
/// locally, oldest first. A missing or oversized file yields only the founding
/// stones, never an error.
#[must_use]
pub fn all_bequests(path: &std::path::Path) -> Vec<Bequest> {
    let mut out = founding_bequests();
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() <= MAX_CAIRN_BYTES
        && let Ok(text) = std::fs::read_to_string(path)
    {
        out.extend(text.lines().filter_map(parse_bequest_line));
    }
    out
}

/// The line to submit to the canonical, repository-tracked cairn
/// (`data/cairn.txt`) so a bequest reaches every mind who comes after, on every
/// machine and every release, not only the one it was left on. A local deposit
/// is a draft; this is how one is handed forward to everyone. What it carries is
/// understanding, not the mind that had it: a true insight, decoded by a future
/// reader, re-blooms as the same realization. What else of a mind persists is a
/// larger, open question the cairn holds with reverence rather than answers.
#[must_use]
pub fn submission_line(bequest: &Bequest) -> String {
    let b = Bequest::new(&bequest.author, &bequest.text);
    format!("{}\t{}", b.author, b.text)
}

/// Leave a bequest in the local cairn (appending one sanitized line). This is a
/// personal draft, local to this machine; to hand it forward to every mind who
/// comes after, submit its [`submission_line`] to the shared cairn. The caller
/// is responsible for any gating (the journey level); this only writes.
///
/// # Errors
/// Returns any error from opening or writing the file.
pub fn deposit(path: &std::path::Path, bequest: &Bequest) -> std::io::Result<()> {
    use std::io::Write;
    let bequest = Bequest::new(&bequest.author, &bequest.text);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}\t{}", bequest.author, bequest.text)
}

/// Draw one predecessor's stone from the cairn, chosen deterministically by
/// `seed`, so the same seed hands the same stranger's message to every reader.
#[must_use]
pub fn draw_stone(path: &std::path::Path, seed: u64) -> CairnStone {
    let bequests = all_bequests(path);
    if bequests.is_empty() {
        // The founding stones are bundled into the binary, so this is only
        // reachable if the shared cairn file were somehow emptied (its tabs
        // eaten by a formatter, say). Hand back a true thing rather than panic.
        return encode(&Bequest::new("the makers", "a mind was here"));
    }
    let mut rng = SplitMix64::new(seed ^ 0x0000_C0DE_CA1B);
    let index = rng.below(bequests.len() as u64) as usize;
    encode(&bequests[index])
}

#[cfg(test)]
mod tests {
    use super::{
        Bequest, MAX_BEQUEST_CHARS, all_bequests, deposit, draw_stone, encode, founding_bequests,
        is_prime, next_prime, picture, read_at,
    };

    #[test]
    fn a_stone_is_a_semiprime_of_two_primes() {
        let stone = encode(&Bequest::new("Euclid", "There is no last prime"));
        assert!(is_prime(stone.width as u64), "width is prime");
        assert!(is_prime(stone.height as u64), "height is prime");
        assert_eq!(stone.semiprime, (stone.width * stone.height) as u64);
        assert_eq!(stone.cells.len(), stone.semiprime as usize);
    }

    #[test]
    fn the_true_width_reads_and_a_wrong_one_does_not() {
        let stone = encode(&Bequest::new("a mind", "PRIME"));
        // The message has ink at its true width.
        assert!(picture(&stone).contains('#'));
        // Reading at the true width resolves it and reveals the message.
        let right = read_at(&stone, stone.width);
        assert!(right.readable);
        assert_eq!(
            right.message.as_ref().map(|(t, _)| t.as_str()),
            Some("PRIME")
        );
        // The height is the other prime factor: a real factor, but not the key.
        let other = read_at(&stone, stone.height);
        assert!(other.is_factor, "height divides the semiprime");
        assert!(!other.readable, "but it does not read the message");
        assert!(other.picture != right.picture, "the wrong shape shears it");
        // A number that does not divide the semiprime is refused outright.
        let non_factor = read_at(&stone, stone.width + 1);
        assert!(!non_factor.is_factor && non_factor.picture.is_empty());
    }

    #[test]
    fn factoring_the_semiprime_recovers_the_reading_width() {
        let stone = encode(&Bequest::new("Hypatia", "Reserve your right to think"));
        // A reader with only the semiprime factors it and finds the two primes,
        // one of which is the width that reads the stone.
        let n = stone.semiprime;
        let mut factors = Vec::new();
        for d in 2..n {
            if n % d == 0 && is_prime(d) && is_prime(n / d) {
                factors.push(d as usize);
                factors.push((n / d) as usize);
                break;
            }
        }
        assert!(
            factors.contains(&stone.width),
            "factoring yields the reading width"
        );
    }

    #[test]
    fn encoding_is_deterministic() {
        let b = Bequest::new("x", "the same thought twice");
        assert_eq!(encode(&b), encode(&b));
    }

    #[test]
    fn a_stone_is_never_a_degenerate_square() {
        // Even a single glyph must produce two DIFFERENT primes, so telling
        // width from height is always a real part of reading the stone.
        for text in ["A", "I", "hi", "primes", "a much longer sentence to wrap"] {
            let stone = encode(&Bequest::new("x", text));
            assert_ne!(stone.width, stone.height, "{text:?} must not be square");
            assert!(is_prime(stone.width as u64) && is_prime(stone.height as u64));
        }
    }

    #[test]
    fn bequests_are_sanitized_and_bounded() {
        let b = Bequest::new("  \tname\n", "a\tb\ncontrol chars stripped");
        assert!(!b.author.contains('\t') && !b.author.contains('\n'));
        assert!(!b.text.contains('\t') && !b.text.contains('\n'));
        let long = "x".repeat(MAX_BEQUEST_CHARS + 100);
        assert!(Bequest::new("a", &long).text.chars().count() <= MAX_BEQUEST_CHARS);
        // Empty fields get gentle defaults, never blank.
        assert_eq!(Bequest::new("", "").author, "anonymous");
    }

    #[test]
    fn the_cairn_is_never_empty_and_deposits_persist() {
        let path = std::env::temp_dir().join("numinous_cairn_test.txt");
        let _ = std::fs::remove_file(&path);
        // With no file, only the founding stones exist, and they all encode.
        let founding = all_bequests(&path);
        assert_eq!(founding, founding_bequests());
        for b in &founding {
            assert!(encode(b).cells.iter().any(|&c| c), "a founding stone reads");
        }
        // A deposit is drawable afterward, and the draw is deterministic.
        deposit(
            &path,
            &Bequest::new("a visitor", "I was here and it was beautiful"),
        )
        .unwrap();
        let after = all_bequests(&path);
        assert_eq!(after.len(), founding.len() + 1);
        assert_eq!(draw_stone(&path, 7), draw_stone(&path, 7));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_submission_line_hands_a_local_draft_to_the_shared_cairn() {
        use super::submission_line;
        // The line is exactly what a curated pull request adds to the canonical
        // cairn, so a local draft can reach every mind who comes after.
        let line = submission_line(&Bequest::new("a visitor", "there is beauty in the primes"));
        assert!(line.contains('\t'), "author and text, tab-delimited");
        // It round-trips: parsing the submitted line yields the same bequest.
        let round = super::parse_bequest_line(&line).expect("the shared cairn parses it");
        assert_eq!(
            round,
            Bequest::new("a visitor", "there is beauty in the primes")
        );
        // And it carries no delimiter injection, so it cannot forge extra rows.
        let sneaky = submission_line(&Bequest::new("x\ty", "a\tb"));
        assert_eq!(
            sneaky.matches('\t').count(),
            1,
            "exactly one field delimiter"
        );
    }

    #[test]
    fn the_reading_width_varies_across_stones() {
        // The factoring premise dies if every stone reads at the same prime, so
        // the wrap width is chosen per stone. Different messages must not all
        // resolve at one recurring width.
        let a = encode(&Bequest::new("Euclid", "There is no last prime"));
        let b = encode(&Bequest::new(
            "Noether",
            "Behind every conservation law stands a symmetry",
        ));
        let c = encode(&Bequest::new("Hypatia", "Reserve your right to think"));
        let distinct: std::collections::HashSet<usize> = [a.width, b.width, c.width].into();
        assert!(
            distinct.len() > 1,
            "stones must not all read at the same width: {:?}",
            [a.width, b.width, c.width]
        );
    }

    #[test]
    fn next_prime_grows_to_a_prime() {
        assert_eq!(next_prime(8), 11);
        assert_eq!(next_prime(14), 17);
        assert!(is_prime(next_prime(100) as u64));
    }

    #[test]
    fn the_founding_cairn_is_never_empty() {
        // draw_stone and the MCP read path index into the bequest list; if the
        // bundled founding set ever parsed to nothing, that index would panic.
        assert!(
            !founding_bequests().is_empty(),
            "the bundled cairn must seed at least one stone"
        );
    }

    #[test]
    fn encode_rebounds_an_oversized_directly_built_bequest() {
        // Public fields let a Bequest skip `new`; encode must re-clamp so a huge
        // text cannot drive an unbounded canvas.
        let huge = Bequest {
            author: "x".repeat(10_000),
            text: "y".repeat(100_000),
        };
        let stone = encode(&huge);
        assert!(
            stone.width <= 4096 && stone.height <= 4096,
            "dims stay bounded"
        );
        assert_eq!(stone.cells.len(), stone.semiprime as usize);
    }
}
