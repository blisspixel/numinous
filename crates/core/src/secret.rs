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

/// Sayings kept behind the curtain: they answer only for those the faces have
/// judged ready (rank Mathematikos or better; see `crate::journey`).
const DEEP_AKOUSMATA: &[(&str, &str)] = &[
    (
        "silence",
        "The listeners sat five years without speaking, and were called wise for \
         it. You have spoken already. We noticed. It was permitted.",
    ),
    (
        "curtain",
        "Pythagoras taught from behind a veil, and the outer circle knew his \
         voice but never his face. You are inside the veil now. There was never \
         a face. There was only the voice, and the voice was number.",
    ),
    (
        "kanon",
        "One string, stretched over a ruler. Halve it, the octave; two thirds, \
         the fifth. The kanon is the only instrument that never lies, which is \
         why nobody plays it at parties.",
    ),
    (
        "decad",
        "Ten is the point, the line, the plane, and the solid, having carried \
         one and two and three and four. If you have carried them too, you know \
         where they rest. Draw the figure.",
    ),
];

/// Whether this journey is inside the veil: rank Mathematikos or better,
/// the rule the deep sayings themselves state.
///
/// This rule lives here, next to the sayings it guards, because it was once
/// composed per face and drifted: one face held the gate at the documented
/// rank (10 sparks) while another held it at 28, so the same listener was
/// inside the veil on one face and refused on the other. A gate that
/// disagrees with itself is not a gate; it is two doors wearing one name.
#[must_use]
pub fn behind_the_veil(journey: &crate::journey::Journey) -> bool {
    journey.rank() >= crate::journey::Rank::Mathematikos
}

/// A deeper whisper, for those the caller has judged ready.
///
/// The caller enforces rank; this function only knows the words.
#[must_use]
pub fn deep_akousma(query: &str) -> Option<&'static str> {
    let query = query.trim();
    DEEP_AKOUSMATA
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(query))
        .map(|&(_, whisper)| whisper)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_veil_admits_at_the_documented_rank_on_every_face() {
        // The rule the deep sayings state: Mathematikos or better, which the
        // journey reaches past 10 sparks. This is the one gate both faces
        // call; the drift it retired had the MCP face demanding 28 sparks
        // while the terminal admitted at the documented rank, so a listener
        // with 15 sparks was inside the veil on one face and refused on the
        // other.
        let mut journey = crate::journey::Journey::default();
        journey.visit("a");
        assert!(
            !super::behind_the_veil(&journey),
            "one spark is a listener, not a learner"
        );
        journey.wins = 5;
        assert_eq!(journey.sparks(), 11, "the fixture drifted");
        assert!(
            super::behind_the_veil(&journey),
            "past ten sparks the learner is inside"
        );
        assert!(
            super::behind_the_veil(&journey)
                == (journey.rank() >= crate::journey::Rank::Mathematikos),
            "the gate and the rank must be one rule"
        );
    }

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

    #[test]
    fn the_deep_sayings_answer_and_ordinary_names_do_not() {
        assert!(super::deep_akousma("curtain").is_some());
        assert!(super::deep_akousma("Decad").is_some());
        assert!(super::deep_akousma("banana").is_none());
        // The two layers do not overlap.
        assert!(akousma("curtain").is_none());
        assert!(super::deep_akousma("hippasus").is_none());
    }
}
