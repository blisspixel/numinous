//! Further-reading citations keyed by room id (panel item 7).
//!
//! Short, durable references a curious mind can chase offline. Not every
//! catalog room has a bespoke line yet; [`for_room`] returns a wing-level
//! fallback so faces always have something honest to show with reveal.

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
        "julia" => {
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
        "logistic-map" => {
            "See also: May, Simple mathematical models with very complicated dynamics \
             (Nature, 1976); Feigenbaum constants."
        }
        "collatz" => {
            "See also: Lagarias, The 3x+1 problem and its generalizations; \
             the Collatz conjecture remains open."
        }
        "goldbach" => "See also: Goldbach's conjecture; Hardy & Littlewood circle method surveys.",
        "twin-primes" => {
            "See also: twin prime conjecture; Zhang's bounded gaps work (2013) and \
             Polymath8 surveys."
        }
        "prime-spirals" => {
            "See also: Ulam spiral; Stein, S. & Ulam; popular expositions on primes \
             on polar plots."
        }
        "epicycles" => "See also: Fourier series and epicycles; Needham, Visual Complex Analysis.",
        "lissajous" => "See also: Lissajous curves; classical harmonic motion texts.",
        "buffon-needle" => "See also: Buffon's needle problem; geometric probability in Feller.",
        "cult-of-pi" => "See also: Arndt & Haenel, Pi Unleashed; Borwein on pi algorithms.",
        "barnsley-fern" => "See also: Barnsley, Fractals Everywhere (IFS attractors).",
        "lsystem" | "l-system" | "l-system-garden" => {
            "See also: Prusinkiewicz & Lindenmayer, The Algorithmic Beauty of Plants."
        }
        "voronoi" => "See also: Aurenhammer, Voronoi diagrams; computational geometry surveys.",
        "conjecture-mill" => {
            "See also: experimental mathematics; Borwein & Bailey, Mathematics by Experiment."
        }
        "langtons-ant" => "See also: Langton's ant; Gajardo et al. on recurrent behavior.",
        "cellular-automata" => {
            "See also: Wolfram, A New Kind of Science (with critical reading); \
             Cook on Rule 110 universality."
        }
        "quine" => "See also: Hofstadter, Gödel, Escher, Bach; quines and self-reference.",
        "strange-loop" => "See also: Hofstadter, I Am a Strange Loop; tangled hierarchies.",
        "arecibo" => "See also: the Arecibo message (1974); SETI Institute historical notes.",
        "fourier" | "fourier-epicycles" => {
            "See also: Fourier, Théorie analytique de la chaleur; modern DFT primers."
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
        _ => {
            "See also: explore the room's wing in a standard reference library, then \
             return with a better citation for this phenomenon."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::for_room;

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
    }
}
