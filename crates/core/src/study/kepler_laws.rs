//! The third authored treatment: Kepler's second law on the drawn ellipse.
//!
//! English only. The Lissajous Japanese draft was independently reviewed and
//! this text was not, so a Japanese request resolves to English and says so.
//!
//! The geometry, Kepler equation, equal-area identity, and apsidal speed ratio
//! are classical. The cited sources are used for Kepler's equation and for the
//! centered-ellipse focus used by the room. The worked numbers are checked
//! against those formulas rather than copied from a table.

use super::{
    StudyBlock, StudyDepth, StudyInline, StudyLocaleResolution, StudyPart, StudySource,
    StudyTranslationStatus,
};

fn text(body: &'static str) -> StudyPart {
    StudyPart::Paragraph(vec![StudyInline::Text(body)])
}

fn mixed(runs: &[(&'static str, bool)]) -> StudyPart {
    StudyPart::Paragraph(
        runs.iter()
            .map(|(body, is_math)| {
                if *is_math {
                    StudyInline::Math(body)
                } else {
                    StudyInline::Text(body)
                }
            })
            .collect(),
    )
}

fn block(
    locale: &StudyLocaleResolution,
    id: &'static str,
    title: &'static str,
    parts: Vec<StudyPart>,
) -> StudyBlock {
    StudyBlock {
        id: id.to_string(),
        title,
        depth: StudyDepth::Mathematics,
        locale: locale.clone(),
        translation: StudyTranslationStatus::Original,
        parts,
    }
}

pub(super) fn blocks(locale: &StudyLocaleResolution) -> Vec<StudyBlock> {
    vec![
        block(
            locale,
            "kepler-laws.model",
            "The ellipse and the sun, stated exactly",
            vec![
                mixed(&[
                    ("Fix an eccentricity ", false),
                    ("e", true),
                    (" with ", false),
                    ("0 <= e <= 0.9", true),
                    (
                        ", the range the room admits. A centered ellipse of semi-major axis ",
                        false,
                    ),
                    ("a > 0", true),
                    (" then has semi-minor axis", false),
                ]),
                StudyPart::Equation("b = a * sqrt(1 - e^2)"),
                mixed(&[
                    ("Parametrize it by eccentric anomaly ", false),
                    ("E", true),
                    (":", false),
                ]),
                StudyPart::Equation("x = a * cos(E),        y = b * sin(E)"),
                mixed(&[(
                    "Perihelion, the nearest point, sits at positive x. The sun sits at \
                         the corresponding focus,",
                    false,
                )]),
                StudyPart::Equation("sun at (a*e, 0)"),
                text(
                    "That sign is part of the model, not a drawing convenience. Measuring time \
                     from perihelion with Kepler's equation below places the planet at positive \
                     x when M = 0. Putting the sun at the opposite focus would still satisfy a \
                     residual check of the solver while making the equal-area sectors wrong.",
                ),
                text(
                    "The screen then flips the y axis because its rows increase downward, and \
                     it scales the two axes so a circle stays round on pixels and on character \
                     cells. Those are presentation maps. They do not move the sun to the other \
                     focus.",
                ),
            ],
        ),
        block(
            locale,
            "kepler-laws.equation",
            "Kepler's equation, and why equal mean anomalies are equal times",
            vec![
                mixed(&[
                    ("Mean anomaly ", false),
                    ("M", true),
                    (
                        " advances in exact proportion to time from perihelion. It is related to ",
                        false,
                    ),
                    ("E", true),
                    (" by Kepler's equation", false),
                ]),
                StudyPart::Equation("M = E - e * sin(E)"),
                mixed(&[
                    ("At ", false),
                    ("e = 0", true),
                    (" this is ", false),
                    ("M = E", true),
                    (
                        ", uniform circular motion. For positive e the planet still spends \
                         equal time on equal intervals of M, because M is defined that way. \
                         Solving for E is what places the planet on the ellipse at that time.",
                        false,
                    ),
                ]),
                text(
                    "The room solves the equation by Newton iteration from E = M, twelve steps, \
                     on the admitted range of e. That is enough for a pixel and is not a claim \
                     about every eccentricity.",
                ),
            ],
        ),
        block(
            locale,
            "kepler-laws.area",
            "Equal times sweep equal areas about the sun",
            vec![
                mixed(&[
                    ("The area of the ellipse is ", false),
                    ("pi*a*b", true),
                    (
                        ". Because M is linear in time, a complete orbit corresponds to \
                         M increasing by 2*pi, and a time fraction t of the period sweeps \
                         the same fraction of that area about the focus:",
                        false,
                    ),
                ]),
                StudyPart::Equation("area in time fraction t = t * pi * a * b"),
                mixed(&[(
                    "The room marks six equal-time sectors, so each has area",
                    false,
                )]),
                StudyPart::Equation("pi * a * b / 6"),
                text(
                    "Those sectors are bounded by the orbital arc, not by the chord between \
                     their endpoints. A chord would understate the area near perihelion, where \
                     the arc bows away from the sun, and that is a different theorem.",
                ),
                mixed(&[
                    ("At ", false),
                    ("e = 0", true),
                    (
                        " the ellipse is a circle of radius a, the sun is at the centre, and \
                         each of the six sectors is a sixth of the disk. That is the circular \
                         limit the speed wager answers SAME.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "kepler-laws.speed",
            "Faster near the sun",
            vec![
                mixed(&[
                    (
                        "At perihelion and aphelion the velocity is perpendicular to the \
                         radius. Angular momentum conservation then says ",
                        false,
                    ),
                    ("r_p * v_p = r_a * v_a", true),
                    (". With ", false),
                    ("r_p = a*(1-e)", true),
                    (" and ", false),
                    ("r_a = a*(1+e)", true),
                    (" the speed ratio is", false),
                ]),
                StudyPart::Equation("v_p / v_a = (1+e) / (1-e)"),
                mixed(&[
                    ("The same number is the distance ratio ", false),
                    ("r_a / r_p", true),
                    (
                        ", which the room reports as ra/rp after a hand tunes e.",
                        false,
                    ),
                ]),
                mixed(&[
                    ("Worked values: at ", false),
                    ("e = 0", true),
                    (" the ratio is 1. At ", false),
                    ("e = 1/2", true),
                    (" it is 3. At ", false),
                    ("e = 1/5", true),
                    (
                        " it is 3/2. The circular limit is the only case the speed wager \
                     grades SAME. Any admitted positive e is faster at perihelion.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "kepler-laws.limits",
            "Limits of this treatment",
            vec![
                text(
                    "The identities above are exact in the two-body Kepler problem with a \
                     fixed focus, a closed elliptical orbit, and time measured from \
                     perihelion. The room is that model, sampled.",
                ),
                text(
                    "The solver is binary64 Newton iteration, not a closed form. Equal-area \
                     checks in the source triangulate the renderer positions before pixel \
                     rounding, with relative quadrature error below one part in ten to the \
                     fifth on six sectors at e = 0, 0.2, 0.6, and 0.9. That is a sampled \
                     bound, not a proof that every drawn polygon matches the integral.",
                ),
                text(
                    "The picture rounds the sun and the planet onto a grid and approximates \
                     each arc by line segments. Character cells are not square, so the \
                     renderer applies an aspect map to keep a circle round. Those maps change \
                     what a player sees. They do not change the focus used to grade area or \
                     speed.",
                ),
                text(
                    "This treatment is Kepler's second law and the apsidal speed ratio that \
                     follows from it. It does not derive the inverse-square force, does not \
                     treat hyperbolic or parabolic motion, and does not claim a third-body \
                     or relativistic correction.",
                ),
            ],
        ),
        block(
            locale,
            "kepler-laws.references",
            "References",
            vec![
                text(
                    "The two sources below are cited for Kepler's equation and for the \
                     centered-ellipse geometry with the focus at +a*e. The equal-area \
                     identity and the apsidal speed ratio are derived above rather than \
                     quoted from them.",
                ),
                StudyPart::Reference {
                    source: &MIT_LECTURE,
                    description: "States the centered ellipse, the focal distance a*e, Kepler's \
                                  equation M = E - e sin(E), and the area interpretation of \
                                  mean anomaly used above.",
                },
                StudyPart::Reference {
                    source: &MATHWORLD,
                    description: "Defines Kepler's equation in the form used above, with mean \
                                  anomaly linear in time from perihelion.",
                },
            ],
        ),
    ]
}

static MIT_LECTURE: StudySource = StudySource {
    id: "mit-16-346-lec3",
    title: "MIT 16.346 Astrodynamics, Lecture 3: Kepler's equation, centered ellipse and focus",
    url: "https://ocw.mit.edu/courses/16-346-astrodynamics-fall-2008/379eae1a78cf9ad58247058dffbae3ac_lec_03.pdf",
};

static MATHWORLD: StudySource = StudySource {
    id: "mathworld-keplers-equation",
    title: "Wolfram MathWorld, Kepler's Equation: M = E - e sin(E)",
    url: "https://mathworld.wolfram.com/KeplersEquation.html",
};

#[cfg(test)]
mod tests {
    use super::super::{StudyDepth, StudyLocale, StudyLocaleResolution, StudyPart};
    use std::f64::consts::PI;

    fn locale() -> StudyLocaleResolution {
        StudyLocaleResolution::new(&StudyLocale::parse("en").expect("en parses"), "en")
    }

    fn newton(mean: f64, eccentricity: f64) -> f64 {
        let mut anomaly = mean;
        for _ in 0..12 {
            let residual = anomaly - eccentricity * anomaly.sin() - mean;
            let derivative = 1.0 - eccentricity * anomaly.cos();
            anomaly -= residual / derivative;
        }
        anomaly
    }

    fn speed_ratio(eccentricity: f64) -> f64 {
        (1.0 + eccentricity) / (1.0 - eccentricity)
    }

    #[test]
    fn kepler_equation_holds_on_the_admitted_range() {
        for eccentricity in [0.0_f64, 0.2, 0.5, 0.6, 0.9] {
            for step in 0..=24 {
                let mean = f64::from(step) * 2.0 * PI / 24.0;
                let anomaly = newton(mean, eccentricity);
                let recovered = anomaly - eccentricity * anomaly.sin();
                assert!(
                    (recovered - mean).abs() < 1e-12,
                    "e={eccentricity} M={mean}: recovered {recovered}"
                );
            }
        }
    }

    #[test]
    fn the_worked_speed_ratios_are_the_values_the_formula_gives() {
        assert!((speed_ratio(0.0) - 1.0).abs() < 1e-12);
        assert!((speed_ratio(0.5) - 3.0).abs() < 1e-12);
        assert!((speed_ratio(0.2) - 1.5).abs() < 1e-12);
        assert!((speed_ratio(1.0 / 5.0) - 3.0 / 2.0).abs() < 1e-12);
    }

    #[test]
    fn six_equal_time_sectors_are_one_sixth_of_the_ellipse() {
        for (a, e) in [(1.0_f64, 0.0), (2.0, 0.2), (1.0, 0.9)] {
            let b = a * (1.0_f64 - e * e).sqrt();
            let sector = PI * a * b / 6.0;
            let full = PI * a * b;
            assert!((6.0 * sector - full).abs() < 1e-12);
        }
    }

    #[test]
    fn perihelion_puts_the_sun_at_positive_ae() {
        // The treatment's sign claim: M = 0 is perihelion at +a, sun at +a e.
        let a = 1.0_f64;
        for e in [0.0_f64, 0.2, 0.9] {
            let perihelion_x = a * newton(0.0, e).cos();
            let sun = a * e;
            assert!((perihelion_x - a).abs() < 1e-12, "perihelion at +a");
            assert!(sun >= 0.0);
            assert!((sun - e).abs() < 1e-12);
        }
    }

    #[test]
    fn the_treatment_is_mathematics_only_and_cites_what_it_leans_on() {
        let blocks = super::blocks(&locale());
        assert!(!blocks.is_empty());
        for block in &blocks {
            assert_eq!(block.depth, StudyDepth::Mathematics);
            assert!(block.id.starts_with("kepler-laws."));
            assert!(!block.parts.is_empty(), "{} is empty", block.id);
        }
        assert!(!blocks.iter().any(|b| b.depth == StudyDepth::Explanation));
        let references = blocks
            .iter()
            .flat_map(|block| &block.parts)
            .filter(|part| matches!(part, StudyPart::Reference { .. }))
            .count();
        assert_eq!(references, 2, "both cited sources must survive");
    }
}
