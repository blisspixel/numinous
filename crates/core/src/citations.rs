//! Further-reading citations keyed by room id (panel item 7).
//!
//! Short, durable references a curious mind can chase offline. Not every
//! catalog room has a bespoke line yet; [`for_room`] returns a wing-level
//! fallback so faces always have something honest to show with reveal.
//! Citations unlock with the first deep cut (see [`CUT_LEVELS`]).

use crate::journey::CUT_LEVELS;

/// Look up a further-reading citation for a catalog room id.
#[must_use]
pub fn for_room(id: &str) -> &'static str {
    match id {
        "times-tables" => {
            "See also: modular multiplication on the circle; H. Rademacher, \
             Topics in Analytic Number Theory (cardioid link with Mandelbrot lore)."
        }
        "mandelbrot" => {
            "See also: Mandelbrot, The Fractal Geometry of Nature; Douady & Hubbard \
             on the main cardioid."
        }
        "julia" | "julia-set" => {
            "See also: Julia and Fatou sets in Devaney, An Introduction to Chaotic \
             Dynamical Systems."
        }
        "game-of-life" => {
            "See also: Gardner, Mathematical Games (Sci. Am., 1970); Berlekamp, \
             Conway, Guy, Winning Ways, for cellular automata."
        }
        "galton-board" => {
            "See also: Galton, Natural Inheritance; Feller, An Introduction to \
             Probability Theory and Its Applications (binomial / normal)."
        }
        "double-pendulum" => {
            "See also: classical chaos texts on the double pendulum; Strogatz, \
             Nonlinear Dynamics and Chaos."
        }
        "lorenz" => {
            "See also: Lorenz, Deterministic Nonperiodic Flow (1963); Sparrow, \
             The Lorenz Equations."
        }
        "logistic-map" | "logistic-cobweb" | "logistic-orbit" => {
            "See also: May, Simple mathematical models with very complicated dynamics \
             (Nature, 1976); Feigenbaum constants."
        }
        "collatz" | "collatz-tree" => {
            "See also: Lagarias, The 3x+1 problem and its generalizations; \
             the Collatz conjecture remains open."
        }
        "goldbach" => "See also: Goldbach's conjecture; Hardy & Littlewood circle method surveys.",
        "twin-primes" => {
            "See also: twin prime conjecture; Zhang's bounded gaps work (2013) and \
             Polymath8 surveys."
        }
        "prime-spirals" | "ulam-spiral" => {
            "See also: Ulam spiral; Stein, S. & Ulam; popular expositions on primes \
             on polar plots."
        }
        "epicycles" | "fourier-epicycles" | "fourier" => {
            "See also: Fourier series and epicycles; Needham, Visual Complex Analysis."
        }
        "lissajous" | "lissajous-3d" => {
            "See also: Lissajous curves; classical harmonic motion texts."
        }
        "buffon-needle" => "See also: Buffon's needle problem; geometric probability in Feller.",
        "cult-of-pi" => "See also: Arndt & Haenel, Pi Unleashed; Borwein on pi algorithms.",
        "barnsley-fern" => "See also: Barnsley, Fractals Everywhere (IFS attractors).",
        "lsystem" | "l-system" | "l-system-garden" => {
            "See also: Prusinkiewicz & Lindenmayer, The Algorithmic Beauty of Plants."
        }
        "voronoi" | "delaunay" => {
            "See also: Aurenhammer, Voronoi diagrams; computational geometry surveys."
        }
        "conjecture-mill" => {
            "See also: experimental mathematics; Borwein & Bailey, Mathematics by Experiment."
        }
        "langtons-ant" => "See also: Langton's ant; Gajardo et al. on recurrent behavior.",
        "cellular-automata" | "rules30" | "wolfram-110" => {
            "See also: Wolfram, A New Kind of Science (with critical reading); \
             Cook on Rule 110 universality."
        }
        "quine" => "See also: Hofstadter, Godel, Escher, Bach; quines and self-reference.",
        "strange-loop" => "See also: Hofstadter, I Am a Strange Loop; tangled hierarchies.",
        "arecibo" => "See also: the Arecibo message (1974); SETI Institute historical notes.",
        "random-walk" | "brownian" => {
            "See also: Feller on random walks; Einstein on Brownian motion (1905)."
        }
        "mobius" => "See also: Mobius strip topology; any first course in surfaces.",
        "zeno" => "See also: Zeno's paradoxes; modern treatments via infinite series.",
        "golden-angle" => {
            "See also: phyllotaxis and the golden angle; Vogel's model of sunflower packing."
        }
        "the-pour" => {
            "See also: conservation and free-surface flow primers; fluid mechanics notes."
        }
        "slope-rider" => {
            "See also: calculus of slopes; related-rates and differential geometry intros."
        }
        "harmonograph" => "See also: historical harmonographs; Lissajous figures under damping.",
        "bifurcation" | "feigenbaum" => {
            "See also: Feigenbaum, Universal behavior in nonlinear systems; period doubling."
        }
        "henon" | "henon-heiles" => "See also: Henon map; classical dissipative chaos surveys.",
        "rossler" => "See also: Rossler attractor; continuous chaos beyond Lorenz.",
        "chua" => "See also: Chua's circuit; electronic chaos realization papers.",
        "sierpinski-tri" | "sierpinski-carpet" | "sierpinski-arrowhead" => {
            "See also: Sierpinski gasket; Falconer, Fractal Geometry."
        }
        "koch" | "koch-snowflake" => "See also: Koch curve; early fractal constructions.",
        "dragon-curve" | "levy-c" | "hilbert" | "peano-curve" => {
            "See also: space-filling and fractal curves; classic recreational math."
        }
        "penrose" => "See also: Penrose tilings; aperiodic order surveys.",
        "apollonian" => "See also: Apollonian circle packings; integral packings literature.",
        "buddhabrot" => "See also: the Buddhabrot rendering of the Mandelbrot set (Melinda Green).",
        "burning-ship" => "See also: the Burning Ship fractal; escape-time variants.",
        "multibrot" | "nova" | "phoenix" | "tricorn" => {
            "See also: complex dynamics escape portraits; Devaney surveys."
        }
        "blackbody" => "See also: Planck's law; Wien's displacement; thermal radiation texts.",
        "bayes-update" => "See also: Bayes' theorem; any standard probability text.",
        "shannon-entropy" | "mutual-info" => {
            "See also: Shannon, A Mathematical Theory of Communication (1948)."
        }
        "huffman-tree" => {
            "See also: Huffman coding; Cover & Thomas, Elements of Information Theory."
        }
        "markov-chain" => "See also: Markov chains; Norris or any stochastic processes intro.",
        "central-limit" => "See also: the central limit theorem; Feller volume II sketches.",
        "birthday" => "See also: birthday paradox; elementary probability texts.",
        "benford" => "See also: Benford's law; natural digit distributions.",
        "zipf" => "See also: Zipf's law; power laws in language and cities.",
        "poisson" => "See also: Poisson process; Feller on rare events.",
        "basel" => "See also: Basel problem; Euler's solution for sum 1/n^2.",
        "wallis" => "See also: Wallis product for pi; classical analysis.",
        "gamma-func" => "See also: the gamma function; Artin or any special functions text.",
        "bessel" | "airy" | "hermite" | "legendre" => {
            "See also: special functions handbooks; Watson on Bessel functions."
        }
        "error-function" => "See also: the error function and Gaussian integrals.",
        "dirichlet-eta" | "zeta-walk" => {
            "See also: Riemann zeta and Dirichlet eta; Titchmarsh on the zeta function."
        }
        "pell-path" => "See also: Pell equations; continued fractions for quadratic irrationals.",
        "egyptian-frac" => {
            "See also: Egyptian fractions; greedy algorithms and Sylvester's sequence."
        }
        "continued-frac" => "See also: continued fractions; Khinchin's classic monograph.",
        "farey" | "stern-brocot" | "calkin-wilf" => {
            "See also: Farey sequences and Calkin-Wilf trees; rational enumeration."
        }
        "euclid-algorithm" => {
            "See also: Euclidean algorithm; Knuth, The Art of Computer Programming."
        }
        "euler-totient" => "See also: Euler's totient; Hardy & Wright number theory.",
        "perfect-num" => "See also: perfect numbers; Euclid-Euler theorem for even perfects.",
        "prime-gaps" => "See also: prime gaps; bounded gaps after Zhang and Maynard.",
        "gaussian-primes" => "See also: Gaussian integers; Hardy & Wright on unique factorization.",
        "quadratic-residues" => "See also: quadratic reciprocity; Ireland & Rosen.",
        "zeckendorf" | "fibonacci-word" => {
            "See also: Zeckendorf's theorem; Fibonacci numeration systems."
        }
        "thue-morse" | "paperfold" | "kolakoski" => {
            "See also: automatic sequences; Allouche & Shallit."
        }
        "busy-beaver" => "See also: the Busy Beaver function; Rado; open computability frontiers.",
        "wireworld" => "See also: Wireworld cellular automaton; recreational CA literature.",
        "sandpile" => {
            "See also: abelian sandpile model; Bak, Tang, Wiesenfeld self-organized criticality."
        }
        "percolation" => "See also: percolation theory; Grimmett.",
        "ising" => "See also: Ising model; statistical mechanics intros.",
        "sir" => "See also: SIR epidemic models; Kermack & McKendrick.",
        "lotka-volterra" => "See also: Lotka-Volterra equations; classical population dynamics.",
        "kuramoto" => "See also: Kuramoto model; synchronization surveys.",
        "van-der-pol" => "See also: van der Pol oscillator; nonlinear oscillations.",
        "duffing" => "See also: Duffing equation; forced nonlinear oscillators.",
        "standard-map" | "circle-map" => {
            "See also: Chirikov standard map; circle maps and mode locking."
        }
        "baker" | "horseshoe" | "cat-map" => {
            "See also: Smale horseshoe; Anosov systems; symbolic dynamics primers."
        }
        "tent-map" | "doubling-map" | "gauss-map" => {
            "See also: one-dimensional maps; Devaney on chaotic dynamics."
        }
        "weierstrass" | "blancmange" => {
            "See also: Weierstrass continuous nowhere-differentiable function."
        }
        "cantor-set" | "menger" | "menger-sponge" | "menger-slice" => {
            "See also: Cantor set and Menger sponge; classical fractal constructions."
        }
        "klein-bottle" | "boy-surface" | "cross-cap" | "roman-surface" => {
            "See also: non-orientable surfaces; Hilbert and Cohn-Vossen, Geometry and the Imagination."
        }
        "hopf" | "hopf-fibration" | "hopf-link" => {
            "See also: Hopf fibration; visual topology expositions."
        }
        "poincare-disc" | "hyperbolic-tiling" | "pseudosphere" => {
            "See also: hyperbolic geometry; Thurston notes; Daina Taimina crochet models."
        }
        "riemann-sphere" => {
            "See also: Riemann sphere and stereographic projection; Needham, \
             Visual Complex Analysis; Ahlfors, Complex Analysis."
        }
        "sphere-eversion" => "See also: sphere eversion; Smale's paradox and Outside In.",
        "trefoil" | "unknot" | "figure-eight-knot" | "borromean" => {
            "See also: knot theory intros; Adams, The Knot Book."
        }
        "snell" | "snell-prism" | "brewster" | "bragg" | "diffraction" => {
            "See also: geometric and wave optics; Hecht, Optics."
        }
        "polarization" | "malus" => "See also: Malus's law; polarization of light.",
        "doppler" => "See also: Doppler effect; classical wave physics.",
        "interference" | "standing-wave" | "beats" => {
            "See also: wave interference; any introductory waves chapter."
        }
        "smith-chart" => {
            "See also: P. H. Smith, Transmission Line Calculator (Electronics, 1939); \
             Pozar, Microwave Engineering (Smith chart and matching)."
        }
        "sawtooth" | "triangle-wave" | "fourier-square" | "gibbs-square" => {
            "See also: Fourier series of discontinuous waves; Gibbs phenomenon."
        }
        "am-modulation" | "fm-modulation" => {
            "See also: amplitude and frequency modulation; communication systems texts."
        }
        "uncertainty" => {
            "See also: time-frequency uncertainty; Gabor; quantum uncertainty as analogy only."
        }
        "attention" | "learning-clock" | "gradient-valley" => {
            "See also: optimization landscapes; high-level ML intuition primers (not research claims)."
        }
        "concentration" | "curse-dimension" => {
            "See also: concentration of measure; high-dimensional probability surveys."
        }
        "kepler-laws" | "kepler-loom" => "See also: Kepler's laws; Newton Principia modern notes.",
        "escape-velocity" | "slingshot" => "See also: orbital mechanics primers; gravity assists.",
        "simple-pendulum" | "tautochrone" | "fastest-fall" => {
            "See also: classical mechanics of the pendulum and brachistochrone."
        }
        "catenary" | "catenoid" | "soap-film" => {
            "See also: minimal surfaces and the catenary; calculus of variations."
        }
        "cardioid" | "nephroid" | "deltoid" | "astroid" | "epitrochoid" | "hypotrochoid" => {
            "See also: classical roulette curves; Lockwood, A Book of Curves."
        }
        "lemniscate" | "lemniscate-gerono" | "cassini" => {
            "See also: lemniscates and Cassini ovals; classical algebraic curves."
        }
        "clothoid" | "log-spiral" | "archimedean" | "fermat-spiral" => {
            "See also: spirals in nature and design; Bernoulli and Euler spiral notes."
        }
        "reuleaux" | "superellipse" => {
            "See also: curves of constant width; Lamé curves / superellipses."
        }
        "witch-of-agnesi" | "witch-caustic" => {
            "See also: the witch of Agnesi; classical cubic curves."
        }
        "nontransitive" | "parrondo" => "See also: nontransitive dice; Parrondo's paradox papers.",
        "braess" => "See also: Braess's paradox; network routing counterexamples.",
        "josephus" => {
            "See also: Josephus problem; concrete mathematics (Graham, Knuth, Patashnik)."
        }
        "hilbert-hotel" => "See also: Hilbert's hotel; infinity popularizations with care.",
        "soft-proof" | "wet-oracle" => {
            "See also: experimental mathematics; computer-assisted proof culture."
        }
        "message-heals" => {
            "See also: error-correcting codes; Hamming; Shannon channel coding theorem."
        }
        "the-lens" | "the-magnet" | "the-stretch" => {
            "See also: geometric transformations; linear algebra visual texts."
        }
        other if other.contains("prime") => {
            "See also: Hardy & Wright, An Introduction to the Theory of Numbers."
        }
        other if other.contains("pendulum") || other.contains("chaos") => {
            "See also: Strogatz, Nonlinear Dynamics and Chaos."
        }
        other if other.contains("fractal") || other.contains("julia") || other.contains("nova") => {
            "See also: Falconer, Fractal Geometry; classic complex dynamics surveys."
        }
        other
            if other.contains("wave") || other.contains("fourier") || other.contains("bessel") =>
        {
            "See also: classical texts on Fourier analysis and special functions."
        }
        other if other.contains("graph") || other.contains("network") => {
            "See also: Bondy & Murty, Graph Theory; network science primers."
        }
        other if other.contains("knot") || other.contains("link") => {
            "See also: Adams, The Knot Book; introductory knot theory."
        }
        other if other.contains("map") || other.contains("attractor") => {
            "See also: Strogatz or Devaney on maps and attractors."
        }
        _ => {
            "See also: explore the room's wing in a standard reference library, then \
             return with a better citation for this phenomenon."
        }
    }
}

/// Citation text if the journey has earned the first deep cut, else `None`.
///
/// Panel item 7: further reading unlocks with the first deep cut (level
/// [`CUT_LEVELS`]`[0]`, or earlier via a spent boon on cut index 0).
#[must_use]
pub fn for_room_unlocked(id: &str, level: u32, cut0_by_boon: bool) -> Option<&'static str> {
    let need = CUT_LEVELS.first().copied().unwrap_or(5);
    if level >= need || cut0_by_boon {
        Some(for_room(id))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{for_room, for_room_unlocked};
    use crate::journey::CUT_LEVELS;

    #[test]
    fn every_catalog_room_has_a_nonempty_citation() {
        for room in crate::all_rooms() {
            let line = for_room(room.meta().id);
            assert!(!line.is_empty(), "{} citation empty", room.meta().id);
            assert!(
                line.contains("See also:"),
                "{} citation should invite further reading",
                room.meta().id
            );
        }
    }

    #[test]
    fn flagship_citations_are_specific() {
        assert!(for_room("times-tables").contains("modular"));
        assert!(for_room("galton-board").contains("Galton"));
        assert!(
            for_room("game-of-life").contains("Conway")
                || for_room("game-of-life").contains("Gardner")
        );
        assert!(for_room("shannon-entropy").contains("Shannon"));
        assert!(for_room("penrose").contains("Penrose"));
    }

    #[test]
    fn citations_unlock_with_the_first_deep_cut() {
        let need = CUT_LEVELS[0];
        assert!(for_room_unlocked("mandelbrot", need.saturating_sub(1), false).is_none());
        assert!(for_room_unlocked("mandelbrot", need, false).is_some());
        assert!(for_room_unlocked("mandelbrot", 1, true).is_some());
    }
}
