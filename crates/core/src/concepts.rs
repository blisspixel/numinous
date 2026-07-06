//! Concepts: the math each game secretly is, hidden by default.
//!
//! Every game answers "?" with the concept it has been teaching all along.
//! Never shown uninvited, never required, never a gate: the play carries
//! itself, and this door is for the moment curiosity arrives on its own.

/// The concept behind a game, by its id. None for ids that are not games.
#[must_use]
pub fn concept(game: &str) -> Option<&'static str> {
    Some(match game {
        "munch" => {
            "THE CONCEPT: set membership. Every rule carves the numbers into two \
             worlds, inside and outside, and your bites are membership tests. \
             Primes, squares, digit sums: mathematics largely IS deciding what \
             belongs to what, fast. The traps (91 looks prime; it is 7 x 13) are \
             where real number theory lives."
        }
        "quiz" => {
            "THE CONCEPT: recognizing structure. A trained eye reads a picture's \
             generating rule the way you read a face. Spirals whisper growth \
             rates, lattices whisper multiplication, dust whispers iteration. \
             You are training the pattern-matcher mathematicians actually use \
             before any formula appears."
        }
        "nim" => {
            "THE CONCEPT: invariants. The Order tracks one number you cannot see: \
             the xor of the heap sizes. Every winning move restores it to zero, \
             every losing position cannot escape it. Finding the quantity that \
             does not change while everything else does is half of physics and \
             most of game theory. Beat the Order once and it tells you plainly."
        }
        "crack" => {
            "THE CONCEPT: information. Each wire's locked and loose counts are \
             bits that split the ten thousand possible codes into smaller and \
             smaller worlds. Guess to LEARN, not just to hit: a wire that cannot \
             be right can still cut the possibilities in half. That is deduction, \
             and it is measurable, Shannon showed how."
        }
        "seti" => {
            "THE CONCEPT: signatures of mind. Nature makes rhythms, echoes, and \
             noise, all describable by simple recurrences. Counting 2, 3, 5, 7 \
             is different: primes have no rhythm, so nothing mindless drums them \
             out. That is why real SETI would treat a prime beacon as a hello: \
             some patterns only intention can make."
        }
        "aliens" => {
            "THE CONCEPT: representation versus meaning. The sequence is the same \
             in any base; only its clothes change. Fibonacci in binary is still \
             Fibonacci. Separating what a number IS from how it is WRITTEN is the \
             move that unlocks binary, hex, and eventually the idea that math is \
             about structures, not symbols."
        }
        "gauntlet" => {
            "THE CONCEPT: compound performance. The combo is multiplication where \
             each stage's stake is everything you built: one miss resets the \
             factor to one. Streaks, interest, and reliability engineering all \
             run on this same asymmetry, which is why consistency beats \
             brilliance across almost any long run."
        }
        "hackenbush" => {
            "THE CONCEPT: games as numbers. Every garden here has an exact value: \
             one red blade is 1, red with blue on top is exactly 1/2. Add the \
             stalks: positive means Red wins, and the best move is the one that \
             keeps the sum on your side. Conway pushed this until games became a \
             number system bigger than the reals, the surreal numbers."
        }
        "party" => {
            "THE CONCEPT: Ramsey theory. Total disorder is impossible: among any \
             six guests there must be three mutual friends or three mutual \
             strangers, however the handshakes fall. Five guests can escape (the \
             pentagon knows the trick); six cannot, and no cleverness saves you. \
             Structure is not optional at scale; it is a law."
        }
        "fifteen" => {
            "THE CONCEPT: parity, the invisible coin. Count the pairs of tiles \
             that are out of order and where the hole sits: their combined \
             evenness never changes, no matter how you slide. Half of all \
             scrambles are on the wrong side of that coin and no sequence of \
             legal moves can ever cross over. Invariants are how you prove a \
             thing is impossible without trying everything."
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::concept;

    #[test]
    fn every_game_has_its_concept_and_says_its_name() {
        for game in [
            "munch",
            "quiz",
            "nim",
            "crack",
            "seti",
            "aliens",
            "gauntlet",
            "hackenbush",
            "party",
            "fifteen",
        ] {
            let text = concept(game).expect(game);
            assert!(text.starts_with("THE CONCEPT:"), "{game} names its idea");
            assert!(text.len() > 150, "{game}: a real idea, not a caption");
        }
        assert!(concept("chess").is_none());
    }
}
