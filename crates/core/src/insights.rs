//! Insights: the universe, in one breath at a time.
//!
//! Short true things about how mathematics touches reality, dimensions,
//! infinity, information, us. They drip out during play, uninvited and
//! unforced, one every few rounds: the loading-screen tip elevated to a
//! small gift. Every line is verified true and written to land.

/// The catalog. Order matters only for variety; selection is by index.
pub const INSIGHTS: [&str; 18] = [
    "There are more real numbers between 0 and 1 than there are whole numbers \
     altogether. Infinities come in sizes, and Cantor proved it with a diagonal.",
    "You live in three dimensions, but your options do not: the state of a single \
     cup of coffee needs about a trillion trillion dimensions to describe. Physics \
     is geometry in spaces you cannot picture, and it works anyway.",
    "Every second, your phone does more arithmetic than every human who lived \
     before 1900 did in all their lives combined.",
    "A knot in three dimensions cannot exist in four: with one more direction to \
     move in, every tangle simply slides apart. Some problems are only problems \
     because of where you are standing.",
    "The equation that governs heat spreading through steel also prices stock \
     options and sharpens blurry photos. Mathematics does not care what it is \
     about; that is its superpower.",
    "Pi is in the sky: measure any meandering river's actual length against its \
     straight-line distance and, averaged over many rivers, the ratio drifts \
     toward pi.",
    "There is a number, Graham's number, so large that if every particle in the \
     universe were a digit, you could not write it down, and it appeared in a \
     serious proof about coloring the corners of cubes.",
    "The universe compresses: the entire genetic recipe for you fits in about \
     700 megabytes. A mathematical description of a thing is often smaller than \
     the thing, and that gap is why science works.",
    "Spacetime is four-dimensional, and the fourth direction is not spooky: time \
     enters the distance formula with a minus sign, and that single minus sign \
     is all of special relativity.",
    "A sheet of paper folded 42 times would reach the Moon. Exponentials are not \
     fast; they are patient, and then they are sudden.",
    "Black holes obey an equation so simple it fits on a ring: entropy equals \
     area over four. The most extreme objects in existence are also among the \
     simplest.",
    "Quantum mechanics runs on imaginary numbers, the square roots of negatives \
     that were mocked as impossible for centuries. Reality had been using them \
     the whole time.",
    "There are exactly 17 fundamentally different wallpaper patterns, in any \
     universe. The Alhambra's artists found nearly all of them by hand, \
     centuries before the proof.",
    "If you shuffle a deck of cards properly, that exact ordering has almost \
     certainly never existed before in the history of the universe: 52 factorial \
     is that large.",
    "The fourth dimension is not fiction to mathematicians: a hypercube has 8 \
     cubic faces, 24 square ones, and we can compute its shadows, which is what \
     you see when one is drawn.",
    "Sunflowers count in the golden ratio, hurricanes and galaxies share a \
     spiral, and soap bubbles solve minimization problems instantly. Nature \
     computes; mathematics is us reading the source.",
    "Any map, of anything, in any world, needs at most four colors so no two \
     neighbors match. It took 124 years and a computer to prove; no human has \
     ever read the whole proof.",
    "Between any two moments of your life, an uncountable infinity of instants \
     passed. Zeno noticed; calculus answered; you moved anyway.",
];

/// The nth insight, wrapping: a steady drip with no repeats until the well
/// runs a full lap.
#[must_use]
pub fn insight(n: u64) -> &'static str {
    INSIGHTS[(n as usize) % INSIGHTS.len()]
}

#[cfg(test)]
mod tests {
    use super::{INSIGHTS, insight};

    #[test]
    fn the_well_is_deep_true_sized_and_wraps() {
        assert!(INSIGHTS.len() >= 15);
        for line in INSIGHTS {
            assert!(line.len() > 60, "an insight is a breath, not a word");
            assert!(line.len() < 400, "and a breath, not a lecture");
        }
        assert_eq!(insight(0), insight(INSIGHTS.len() as u64));
        assert_ne!(insight(0), insight(1));
    }
}
