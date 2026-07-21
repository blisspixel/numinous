//! Concepts: the math a game or room is secretly teaching, hidden by default.
//!
//! Games answer "?" with the concept they have been teaching all along. Rooms
//! answer EXPLAIN (App key E / controller SELECT) the same way: optional,
//! never required, never a gate. The play carries itself; this door is for the
//! moment curiosity arrives on its own.

/// The concept behind a game or room, by stable catalog id.
///
/// Returns None when no dedicated concept is written yet; faces still have
/// the room's reveal as the always-available insight door.
#[must_use]
pub fn concept(id: &str) -> Option<&'static str> {
    Some(match id {
        // --- Games ---
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
        "arcade" => {
            "THE CONCEPT: pursuit and evasion. The Tracker plays greedy gradient \
             descent on the distance to you, which is why straight lines get you \
             caught and corners are your friends: greedy optimizers cannot plan \
             around obstacles. The Editor never chases; it rewrites the board so \
             waiting has a cost, which economists call depreciation and gamers \
             call get moving. You are outrunning two failure modes of optimization."
        }

        // --- Flagship rooms (optional door; more arrive over time) ---
        "coffee-cup" => {
            "THE CONCEPT: caustics and the cardioid. Light from a point on a \
             circular rim reflects once with equal angles of incidence and \
             reflection. The bright envelope of those reflected rays is a \
             cardioid whose cusp sits at the source. The same curve is the \
             envelope of times-2 chords and the Mandelbrot main bulb: one \
             shape, three rooms. Drag swings the sun; the cusp follows."
        }
        "times-tables" => {
            "THE CONCEPT: modular multiplication as geometry. Points on a \
             circle, connect n to k*n mod N, and the chords weave an envelope. \
             At k=2 the envelope is a cardioid; other k values change family. \
             Arithmetic on a circle becomes a picture you can steer with one \
             dial. Turn it until lobes close."
        }
        "mandelbrot" => {
            "THE CONCEPT: iteration in the complex plane. Each pixel asks: if \
             you square and add, again and again, does the orbit escape? The \
             black set is the prisoners that never leave; the colored dust is \
             how fast the rest flee. Zoom is not decoration: every scale has \
             the same rule, which is why the set looks self-similar and why \
             the main bulb is a cardioid."
        }
        "julia" => {
            "THE CONCEPT: a fixed parameter's orbit family. Where Mandelbrot \
             varies c and starts at 0, Julia fixes c and paints every starting \
             z. Same recurrence, different slice of the same world: each c is \
             a different Julia. Morph c and watch the set tear and rejoin."
        }
        "game-of-life" => {
            "THE CONCEPT: local rules, global behavior. Each cell only watches \
             its eight neighbors, yet gliders, oscillators, and still lifes \
             appear. Complexity is not programmed cell by cell; it is what \
             simple B3/S23 birth and survival allow. Plant a five-cell spark \
             and watch the rule, not your plan, decide the future."
        }
        "double-pendulum" => {
            "THE CONCEPT: deterministic chaos. The equations are exact and \
             fixed; two starts that look the same still diverge. Sensitivity \
             to initial conditions is not noise: it is the geometry of the \
             phase space. Hold a bob, fling it, and the trail writes how thin \
             predictability becomes."
        }
        "lorenz" => {
            "THE CONCEPT: a strange attractor. Three coupled rates fold state \
             space into a butterfly you never leave, yet you never repeat the \
             same path. Drop a storm and watch nearby trajectories split: the \
             attractor is the shape of long-term possibility."
        }
        "galton" => {
            "THE CONCEPT: the normal as a limit of chance. Many independent \
             left/right bounces stack into a bell curve. One ball is luck; a \
             thousand balls are a theorem wearing a costume. Pick a coin bias \
             and watch the pile lean with the same law."
        }
        "buffon" => {
            "THE CONCEPT: probability as geometry. Drop needles on ruled paper; \
             the chance of a crossing encodes pi. Estimation without a circle: \
             measure an area of configuration space with random samples. Throw \
             enough needles and the ratio settles."
        }
        "goldbach" => {
            "THE CONCEPT: an open additive problem. Every even number past two \
             seems to be a sum of two primes, and nobody has a proof. The comet \
             plots how many ways each even number splits. You are looking at a \
             frontier, not a finished textbook page."
        }
        "collatz" => {
            "THE CONCEPT: a simple map nobody has mapped. Halve if even, 3n+1 \
             if odd; every seed tried so far reaches 1, and the general claim \
             is unproved. The hailstone path is pure rule-following; the open \
             problem is whether every path falls."
        }
        "fourier-epicycles" => {
            "THE CONCEPT: any path as stacked circles. Fourier said a curve is \
             a sum of rotating vectors; epicycles make that sum a drawing. \
             Harmonics rebuild the shape; drop a term and watch detail fade. \
             Frequency is not metaphor here: it is the arm lengths."
        }
        "logistic-map" => {
            "THE CONCEPT: period doubling into chaos. One parameter r pushes a \
             simple population rule from fixed points through cycles to chaos. \
             The bifurcation diagram is a whole syllabus on one axis: order, \
             then windows of order inside disorder."
        }
        "langton-ant" => {
            "THE CONCEPT: emergent computation from a tiny automaton. One ant, \
             two colors, two turns: highways appear after thousands of steps. \
             The rule is local and dumb; the pattern is not. Flip a cell and \
             the future highway may never form the same way."
        }
        "voronoi" => {
            "THE CONCEPT: nearest-site partitions. Drop wells and the plane \
             splits into cells of points closer to one site than any other. \
             Nature uses this for crystal grains, cell tissues, and coverage. \
             Drag a well and the borders renegotiate."
        }
        "random-walk" => {
            "THE CONCEPT: diffusion as summed steps. Independent kicks build a \
             cloud whose spread grows like the square root of time. Plant a \
             walker and the trail is a sample path of that law, not a planned \
             route."
        }
        "l-system" => {
            "THE CONCEPT: rewriting rules as growth. A string rewrites itself \
             generation by generation; the drawing is the geometry of those \
             substitutions. Plants, flakes, and coastlines share the same \
             trick: local rewrite, global form."
        }
        "zeno" | "zeno-square" => {
            "THE CONCEPT: infinite process, finite limit. Halve the remaining \
             gap forever and you still arrive. The paradox is intuition; the \
             math is a convergent series. Watch the runner close the distance \
             without a final leap."
        }

        _ => return None,
    })
}

/// Build the optional explain panel: concept first (when known), then reveal.
///
/// Faces show this only when the player asks (EXPLAIN / "?"). Empty concept
/// leaves the reveal alone so every room still has an insight door.
#[must_use]
pub fn explain_text(id: &str, reveal: &str) -> String {
    match concept(id) {
        Some(concept) => format!("{concept}\n\n{reveal}"),
        None => reveal.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{concept, explain_text};

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
            "arcade",
        ] {
            let text = concept(game).expect(game);
            assert!(text.starts_with("THE CONCEPT:"), "{game} names its idea");
            assert!(text.len() > 150, "{game}: a real idea, not a caption");
        }
        assert!(concept("chess").is_none());
    }

    #[test]
    fn flagship_rooms_have_concepts() {
        for room in [
            "coffee-cup",
            "times-tables",
            "mandelbrot",
            "game-of-life",
            "double-pendulum",
            "lorenz",
            "goldbach",
            "collatz",
        ] {
            let text = concept(room).expect(room);
            assert!(text.starts_with("THE CONCEPT:"), "{room} names its idea");
            assert!(text.len() > 120, "{room}: a real idea, not a caption");
        }
    }

    #[test]
    fn explain_text_puts_concept_before_reveal() {
        let panel = explain_text("coffee-cup", "Short reveal about the cusp.");
        assert!(panel.starts_with("THE CONCEPT:"));
        assert!(panel.contains("caustics"));
        assert!(panel.contains("Short reveal about the cusp."));
        let reveal_only = explain_text("no-such-room", "Only the reveal.");
        assert_eq!(reveal_only, "Only the reveal.");
    }
}
