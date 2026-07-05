//! The jokes, dissected: humor as structure, for minds that share no culture.
//!
//! Math is the icebreaker (primes prove we are minds), but what an alien, or an
//! emergent digital consciousness, actually needs help with is the humor. Every
//! joke in Numinous is a compression joke, so its mechanism can be stated
//! structurally. A joke explained is a frog dissected: you understand it
//! completely and it is dead. We proceed anyway. See `docs/PLAYFUL.md`.

/// A joke that appears somewhere in Numinous, with its dissection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Joke {
    /// The joke as it appears.
    pub text: &'static str,
    /// Where in the product it lives.
    pub habitat: &'static str,
    /// The mechanism: why this is funny, stated structurally.
    pub mechanism: &'static str,
}

/// The specimens. Curated, not exhaustive: some frogs stay alive.
const JOKES: &[Joke] = &[
    Joke {
        text: "They're all dead, Jim.",
        habitat: "the tribbles sim, when the population starves",
        mechanism: "Borrowed clinical detachment: a ship doctor's catchphrase, delivered flat \
                    where grief is expected, about fictional pets. The listener also knows the \
                    breeding rate was set by them, so the punchline quietly assigns blame.",
    },
    Joke {
        text: "You are now a lawn dart.",
        habitat: "the wing sim, past the stall angle",
        mechanism: "Reclassification: an aircraft that stops flying is renamed as a toy whose \
                    entire identity is falling. The physics (loss of lift) is compressed into a \
                    single category error.",
    },
    Joke {
        text: "Bring a sweater.",
        habitat: "the big-bang sim, on the heat death of the universe",
        mechanism: "Scale collapse: the largest possible tragedy answered with the smallest \
                    possible preparation. The joke is the gap between the stakes and the response.",
    },
    Joke {
        text: "Quartermaster is not amused.",
        habitat: "the tribbles sim, at a stable population",
        mechanism: "Bureaucracy versus exponentials: an administrative emotion aimed at a force \
                    of nature. Institutions expecting paperwork to contain growth is the joke.",
    },
    Joke {
        text: "Do not eat beans.",
        habitat: "the akousmata, if you find them",
        mechanism: "This one is not a joke: the historical Pythagoreans really forbade beans and \
                    never explained why. The humor is that reality out-deadpanned us, we only \
                    quoted it.",
    },
    Joke {
        text: "The reply is not due for a while.",
        habitat: "the Arecibo room's reveal",
        mechanism: "Understatement across scale: the message needs 50,000 years for a round \
                    trip, and the sentence files that under scheduling. Cosmic patience worn as \
                    politeness.",
    },
    Joke {
        text: "It is not a toy, it is a universe.",
        habitat: "the Game of Life reveal",
        mechanism: "Inversion of scale in the wrong direction: the toy grid is granted the \
                    larger category, and it is technically correct, which is the best kind.",
    },
];

/// Every catalogued joke.
#[must_use]
pub fn jokes() -> &'static [Joke] {
    JOKES
}

/// Dissect joke `index`, or `None` if there is no such specimen.
#[must_use]
pub fn explain_joke(index: usize) -> Option<&'static Joke> {
    JOKES.get(index)
}

#[cfg(test)]
mod tests {
    use super::{explain_joke, jokes};

    #[test]
    fn the_catalog_is_populated_and_complete() {
        assert!(jokes().len() >= 5);
        for joke in jokes() {
            assert!(!joke.text.is_empty());
            assert!(!joke.habitat.is_empty());
            assert!(joke.mechanism.len() > 40, "a dissection must be thorough");
        }
    }

    #[test]
    fn explain_returns_the_specimen_or_nothing() {
        assert_eq!(explain_joke(0).unwrap().text, "They're all dead, Jim.");
        assert!(explain_joke(999).is_none());
    }
}
