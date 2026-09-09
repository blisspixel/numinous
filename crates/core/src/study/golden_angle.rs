//! The fourth authored treatment: the golden angle and Vogel's packing.
//!
//! English only. The Lissajous Japanese draft was independently reviewed and
//! this text was not, so a Japanese request resolves to English and says so.
//!
//! The identities below are classical. Vogel's 1979 construction is cited for
//! the polar recipe the room draws. The golden-angle identity is checked
//! against phi rather than quoted from a table. Biological packing is a
//! separate claim and is not made here.

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
            "golden-angle.model",
            "Vogel's packing, stated exactly",
            vec![
                text(
                    "Place seeds in the plane, indexed by a non-negative integer k. Seed k \
                     sits at polar coordinates",
                ),
                StudyPart::Equation("theta(k) = k * alpha + phi0,        r(k) = c * sqrt(k)"),
                mixed(&[
                    ("for a fixed step ", false),
                    ("alpha", true),
                    (", an origin phase ", false),
                    ("phi0", true),
                    (", and a scale ", false),
                    ("c > 0", true),
                    (
                        ". The square-root radius is Fermat's spiral: equal area between \
                         successive rings when k increases by one. The room draws that recipe \
                         on a finite disc and then maps it onto pixels or character cells.",
                        false,
                    ),
                ]),
                mixed(&[
                    ("At rest the step is the golden angle ", false),
                    ("alpha = pi*(3 - sqrt(5))", true),
                    (
                        ". The dial adds a detune of at most 0.20 radians as phase runs from \
                         0 to 1, so the picture can leave the golden step without changing the \
                         radius law.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "golden-angle.angle",
            "Why that angle is 2*pi / phi^2",
            vec![
                mixed(&[
                    ("Let ", false),
                    ("phi = (1 + sqrt(5))/2", true),
                    (" be the golden ratio. Then ", false),
                    ("phi^2 = phi + 1", true),
                    (" and", false),
                ]),
                StudyPart::Equation("1/phi^2 = (3 - sqrt(5))/2"),
                mixed(&[
                    ("so the fraction of a turn ", false),
                    ("1/phi^2", true),
                    (" is the smaller golden angle in radians:", false),
                ]),
                StudyPart::Equation("2*pi / phi^2 = pi*(3 - sqrt(5))"),
                mixed(&[
                    ("In degrees that is ", false),
                    ("180*(3 - sqrt(5))", true),
                    (
                        ", which rounds to 137.5 at one decimal place. The room's resting \
                         status prints that one-decimal reading. The exact value is not 137.5.",
                        false,
                    ),
                ]),
                text(
                    "The complementary turn 2*pi/phi is about 222.5 degrees. The room uses the \
                     smaller one. They place the same seeds, in opposite directions.",
                ),
            ],
        ),
        block(
            locale,
            "golden-angle.irrational",
            "Why nearby rationals make spokes",
            vec![
                mixed(&[
                    ("If ", false),
                    ("alpha = 2*pi*p/q", true),
                    (" for integers ", false),
                    ("p", true),
                    (" and ", false),
                    ("q > 0", true),
                    (
                        " in lowest terms, then after q steps the angle has advanced by a \
                         whole number of turns and the seeds lie on q rays. A packing that \
                         wants to fill the disc without preferred directions must therefore \
                         avoid good rational approximations to its fraction of a turn.",
                        false,
                    ),
                ]),
                mixed(&[
                    ("The continued fraction of ", false),
                    ("phi", true),
                    (
                        " is all ones, so phi is the hardest number to approximate by \
                         rationals in the sense of Hurwitz's theorem. Its powers inherit that \
                         property. Consecutive Fibonacci ratios are the best approximations \
                         to 1/phi and to 1/phi^2, which is why a nearby rational step produces \
                         a visible spoke count.",
                        false,
                    ),
                ]),
                text(
                    "The room does not count spirals on a real flower. It detunes the step \
                     and lets the finite sample show gaps. That is a picture of the \
                     approximation fact, not a census of florets.",
                ),
            ],
        ),
        block(
            locale,
            "golden-angle.limits",
            "Limits of this treatment",
            vec![
                text(
                    "The identities are exact in the continuous plane with an infinite \
                     sequence of points. The room draws finitely many discs on a grid, \
                     scales the outermost seed to the viewport, and applies a character-aspect \
                     map so a round head stays round on a terminal cell. Planted extra \
                     clusters use a local detune from the click's y coordinate. Those are \
                     presentation maps.",
                ),
                text(
                    "This treatment does not prove that sunflowers optimize packing, does \
                     not derive a growth hormone model, and does not identify the Fermat \
                     spiral radius law with a unique biological mechanism. The Fermat Spiral \
                     room carries the radial curve as its own object. Equal-area rings and \
                     the golden step are two independent choices; the room combines them.",
                ),
                mixed(&[
                    ("The one-decimal status 137.5 is a rounding of ", false),
                    ("180*(3 - sqrt(5))", true),
                    (
                        ", not a second definition of the angle. Binary64 evaluation of \
                         pi*(3 - sqrt(5)) agrees with 2*pi/phi^2 to ordinary rounding error. \
                         That is a machine check of the algebra, not a measurement of a plant.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "golden-angle.references",
            "References",
            vec![
                text(
                    "Vogel is cited for the polar recipe r proportional to sqrt(k) with a \
                     constant divergence angle. MathWorld is cited for the golden-angle \
                     identity with phi. The continued-fraction comparison and the spoke \
                     count for a rational step are derived above.",
                ),
                StudyPart::Reference {
                    source: &VOGEL,
                    description: "Constructs a sunflower head by placing florets on a Fermat \
                                  spiral at a constant divergence, the recipe the room draws.",
                },
                StudyPart::Reference {
                    source: &MATHWORLD,
                    description: "Defines the golden angle as 2*pi/phi^2, matching \
                                  pi*(3 - sqrt(5)) used above.",
                },
            ],
        ),
    ]
}

static VOGEL: StudySource = StudySource {
    id: "vogel-1979-sunflower",
    title: "H. Vogel, A better way to construct the sunflower head, Mathematical Biosciences 44 (1979)",
    url: "https://doi.org/10.1016/0025-5564(79)90080-4",
};

static MATHWORLD: StudySource = StudySource {
    id: "mathworld-golden-angle",
    title: "Wolfram MathWorld, Golden Angle: 2*pi/phi^2",
    url: "https://mathworld.wolfram.com/GoldenAngle.html",
};

#[cfg(test)]
mod tests {
    use super::super::{StudyDepth, StudyLocale, StudyLocaleResolution, StudyPart};
    use std::f64::consts::PI;

    fn locale() -> StudyLocaleResolution {
        StudyLocaleResolution::new(&StudyLocale::parse("en").expect("en parses"), "en")
    }

    #[test]
    fn the_room_angle_is_two_pi_over_phi_squared() {
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let from_phi = 2.0 * PI / (phi * phi);
        let from_surds = PI * (3.0 - 5.0_f64.sqrt());
        assert!(
            (from_phi - from_surds).abs() < 1e-12,
            "2*pi/phi^2 = {from_phi}, pi*(3-sqrt(5)) = {from_surds}"
        );
        assert!((1.0 / (phi * phi) - (3.0 - 5.0_f64.sqrt()) / 2.0).abs() < 1e-12);
        assert!((phi * phi - phi - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_one_decimal_status_is_a_rounding_of_the_exact_degree_value() {
        let degrees = (PI * (3.0 - 5.0_f64.sqrt())).to_degrees();
        assert!((degrees - 180.0 * (3.0 - 5.0_f64.sqrt())).abs() < 1e-12);
        let shown = (degrees * 10.0).round() / 10.0;
        assert_eq!(shown, 137.5);
        assert!((degrees - 137.5).abs() > 0.001);
    }

    #[test]
    fn a_rational_step_closes_after_q_turns() {
        // alpha = 2*pi*p/q lands on a ray after q steps.
        for (p, q) in [(1, 5), (2, 7), (3, 8)] {
            let alpha = 2.0 * PI * f64::from(p) / f64::from(q);
            let wrap = (f64::from(q) * alpha / (2.0 * PI) - f64::from(p)).abs();
            assert!(wrap < 1e-12, "{p}/{q} did not close: {wrap}");
        }
    }

    #[test]
    fn the_treatment_is_mathematics_only_and_cites_what_it_leans_on() {
        let blocks = super::blocks(&locale());
        assert!(!blocks.is_empty());
        for block in &blocks {
            assert_eq!(block.depth, StudyDepth::Mathematics);
            assert!(block.id.starts_with("golden-angle."));
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
