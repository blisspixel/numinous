//! There is nothing in this file.
//!
//! (For those who kept looking: a few names are not in the catalog, yet they
//! answer when you call them. The Order asks only that you keep silent about
//! what you hear. See `docs/LORE.md`, if you can find it.)

/// Names that are not rooms, and what they whisper back. Kept lowercase.
const AKOUSMATA: &[(&str, &str)] = &[
    (
        "pythagoras",
        "He left no writings, and forbade his students to. What is wisest? Number. \
         What is most beautiful? Harmony. You have already said too much.",
    ),
    (
        "tetractys",
        "One, and two, and three, and four. Bless us, fountain that holds the roots \
         of ever-flowing nature: four rows, ten points, the whole of things. Speak \
         of it to no one.",
    ),
    (
        "akousma",
        "A thing heard, not explained. Do not eat beans. Do not stir the fire with \
         a knife. Do not question what is odd. You were not meant to ask why.",
    ),
    (
        "akousmata",
        "The sayings of the ones who only listened. They sat behind the curtain for \
         five years and did not speak. You have been listening for less.",
    ),
    (
        "hippasus",
        "He proved the diagonal of the square could never be a ratio of whole \
         numbers, and spoke of it aloud. The sea took him for it. Some say the \
         Order helped the sea. Do not ask again.",
    ),
    (
        "odd",
        "The odd is limited and male and good; the even, unlimited. One is neither, \
         being both. This is why we question things that are odd. You are learning.",
    ),
    (
        "harmonia",
        "Pluck a string, then half of it: the octave. Two thirds: the fifth. The \
         cosmos is tuned the same way. We called it the music of the spheres, and \
         only Pythagoras could hear it.",
    ),
];

/// If `query` names one of the hidden things, return what it whispers.
///
/// Returns `None` for ordinary names, so callers fall back to their normal
/// not-found behavior and nothing is given away.
#[must_use]
pub fn akousma(query: &str) -> Option<&'static str> {
    let query = query.trim();
    AKOUSMATA
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(query))
        .map(|&(_, whisper)| whisper)
}

#[cfg(test)]
mod tests {
    use super::akousma;

    #[test]
    fn the_named_ones_answer() {
        assert!(akousma("hippasus").is_some());
        assert!(akousma("Tetractys").is_some()); // case-insensitive
        assert!(akousma(" odd ").is_some()); // trimmed
    }

    #[test]
    fn ordinary_names_stay_silent() {
        assert!(akousma("times-tables").is_none());
        assert!(akousma("banana").is_none());
        assert!(akousma("").is_none());
    }
}
