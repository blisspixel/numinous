//! The fifth authored treatment: Fermat's spiral and equal-area windings.
//!
//! English only. The Lissajous Japanese draft was independently reviewed and
//! this text was not, so a Japanese request resolves to English and says so.
//!
//! The polar equation and the two opposite signs of r are classical. The
//! equal-area identity between successive turns is derived from that equation.
//! Discrete sunflower packing is a separate construction and is not proved here;
//! the Golden Angle room carries it.

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
            "fermat-spiral.model",
            "The polar equation, and both signs of r",
            vec![
                text("Fermat's spiral, also called the parabolic spiral, is the polar curve"),
                StudyPart::Equation("r^2 = a^2 * theta"),
                mixed(&[
                    ("for a scale ", false),
                    ("a > 0", true),
                    (" and polar angle ", false),
                    ("theta >= 0", true),
                    (". Equivalently ", false),
                    ("r = a * sqrt(theta)", true),
                    (
                        " or the opposite sign. For each positive theta there are two \
                         real radii, one the negative of the other. Negative r at angle \
                         theta is the same point as positive r at angle theta + pi, so \
                         the two signs are two arms through the origin.",
                        false,
                    ),
                ]),
                text(
                    "The room draws both arms. One follows theta; the other is the same \
                     radius rotated by pi. Ambient phase unfurls them together from the \
                     centre. A drag sets how many turns the parameter runs, and the \
                     drawing scale is chosen so the outermost sample fits the viewport. \
                     That scale is a presentation map. It is not a second definition of a.",
                ),
            ],
        ),
        block(
            locale,
            "fermat-spiral.area",
            "Equal area between successive turns",
            vec![
                mixed(&[
                    (
                        "The polar area between two radii at the same angle, in a slice ",
                        false,
                    ),
                    ("d theta", true),
                    (", is", false),
                ]),
                StudyPart::Equation("dA = (1/2) * (r_outer^2 - r_inner^2) * d theta"),
                mixed(&[
                    ("Take the same branch one full turn later: ", false),
                    ("r(theta + 2*pi)^2 - r(theta)^2 = a^2 * 2*pi", true),
                    (". Then", false),
                ]),
                StudyPart::Equation("dA / d theta = pi * a^2"),
                text(
                    "The area between adjacent windings, per radian, does not depend on \
                     theta. That is the equal-share property of the curve: equal annuli \
                     in equal turns. It is not the sector area from the origin out to the \
                     curve, which grows with theta.",
                ),
                mixed(&[
                    ("Worked values at ", false),
                    ("a = 1", true),
                    (": at ", false),
                    ("theta = 2*pi", true),
                    (", ", false),
                    ("r = sqrt(2*pi)", true),
                    (". One turn later ", false),
                    ("theta = 4*pi", true),
                    (" and ", false),
                    ("r = sqrt(4*pi)", true),
                    (". In both cases ", false),
                    ("r^2 / theta = 1", true),
                    (". The area per radian between those windings is ", false),
                    ("pi", true),
                    (
                        ", and the same number appears between windings at theta = 1 \
                         and at theta = 100.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "fermat-spiral.packing",
            "A curve is not a packing",
            vec![
                mixed(&[
                    ("Vogel's sunflower places discs at ", false),
                    ("r(k) = c * sqrt(k)", true),
                    (" with a separate step angle. Because ", false),
                    ("r(k)^2", true),
                    (" is linear in the index ", false),
                    ("k", true),
                    (
                        ", the annulus between k and k+1 has area pi*c^2, the same for \
                         every seed. That radial law is this curve sampled at integer k. \
                         The azimuth of each seed is not this curve's job.",
                        false,
                    ),
                ]),
                text(
                    "The Golden Angle room takes those discrete samples and turns them. \
                     This room draws the continuous two-armed curve and does not place \
                     florets. Equal-area rings and a golden step are independent choices. \
                     A sunflower model needs both; each room owns one of them.",
                ),
            ],
        ),
        block(
            locale,
            "fermat-spiral.limits",
            "Limits of this treatment",
            vec![
                text(
                    "The identities are exact for the ideal polar curve. The room draws a \
                     finite polyline on a grid, ghosts the full double arm, and brightens \
                     a prefix whose length is the unfurl fraction. Character cells are not \
                     square, so an aspect map keeps a round head round. Those are \
                     presentation maps.",
                ),
                mixed(&[
                    (
                        "The poked status prints a polar radius r~sqrt(theta) with the \
                         dimensionless choice ",
                        false,
                    ),
                    ("a = 1", true),
                    (
                        ", not a pixel length. Binary64 evaluation of r^2 / theta at the \
                         worked angles agrees with a^2 to ordinary rounding error. That is \
                         a machine check of the algebra, not a measurement of a plant.",
                        false,
                    ),
                ]),
                text(
                    "This treatment does not derive curvature or arc length, does not prove \
                     that sunflowers optimize packing, and does not identify the square-root \
                     radius with a unique biological mechanism. Archimedean spirals with \
                     other exponents are different curves; the room is the m = 2 case.",
                ),
            ],
        ),
        block(
            locale,
            "fermat-spiral.references",
            "References",
            vec![
                text(
                    "MathWorld is cited for the polar equation r^2 = a^2 theta and for the \
                     two opposite signs of r. MacTutor is cited for the same equation and \
                     for Fermat's 1636 discussion. The equal-area identity between \
                     successive turns is derived above.",
                ),
                StudyPart::Reference {
                    source: &MATHWORLD,
                    description: "Defines Fermat's spiral as the Archimedean spiral r^2 = a^2 \
                                  theta, with two opposite signs of r at each positive theta, \
                                  matching the two arms the room draws.",
                },
                StudyPart::Reference {
                    source: &MACTUTOR,
                    description: "States the polar equation r^2 = a^2 theta and records that \
                                  Fermat discussed the curve in 1636.",
                },
            ],
        ),
    ]
}

static MATHWORLD: StudySource = StudySource {
    id: "mathworld-fermats-spiral",
    title: "Wolfram MathWorld, Fermat's Spiral: r^2 = a^2 theta",
    url: "https://mathworld.wolfram.com/FermatsSpiral.html",
};

static MACTUTOR: StudySource = StudySource {
    id: "mactutor-fermats-spiral",
    title: "MacTutor History of Mathematics Archive, Fermat's Spiral (1636)",
    url: "https://mathshistory.st-andrews.ac.uk/Curves/Fermats/",
};

#[cfg(test)]
mod tests {
    use super::super::{StudyDepth, StudyLocale, StudyLocaleResolution, StudyPart};
    use std::f64::consts::PI;

    fn locale() -> StudyLocaleResolution {
        StudyLocaleResolution::new(&StudyLocale::parse("en").expect("en parses"), "en")
    }

    fn polar_r(scale: f64, theta: f64) -> f64 {
        scale * theta.sqrt()
    }

    fn area_per_radian_between_turns(scale: f64) -> f64 {
        0.5 * scale * scale * 2.0 * PI
    }

    #[test]
    fn r_squared_over_theta_is_the_scale_squared() {
        let scale = 1.0_f64;
        for theta in [2.0 * PI, 4.0 * PI, 1.0_f64, 100.0] {
            let radius = polar_r(scale, theta);
            assert!(
                (radius * radius / theta - scale * scale).abs() < 1e-12,
                "theta={theta}: r^2/theta = {}",
                radius * radius / theta
            );
        }
        let four = polar_r(2.0, PI);
        assert!((four * four / PI - 4.0).abs() < 1e-12);
    }

    #[test]
    fn the_worked_radii_are_the_values_the_formula_gives() {
        assert!((polar_r(1.0, 2.0 * PI) - (2.0 * PI).sqrt()).abs() < 1e-12);
        assert!((polar_r(1.0, 4.0 * PI) - (4.0 * PI).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn area_between_turns_does_not_depend_on_theta() {
        assert!((area_per_radian_between_turns(1.0) - PI).abs() < 1e-12);
        assert!((area_per_radian_between_turns(2.0) - 4.0 * PI).abs() < 1e-12);
        for theta in [1.0_f64, 2.0 * PI, 100.0] {
            let inner = polar_r(1.0, theta);
            let outer = polar_r(1.0, theta + 2.0 * PI);
            let slice = 0.5 * (outer * outer - inner * inner);
            assert!(
                (slice - PI).abs() < 1e-12,
                "theta={theta}: area per radian {slice}"
            );
        }
    }

    #[test]
    fn the_negative_radius_is_the_arm_rotated_by_pi() {
        let theta = 0.7_f64;
        let radius = polar_r(1.0, theta);
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        let x_arm = radius * (theta + PI).cos();
        let y_arm = radius * (theta + PI).sin();
        assert!((x + x_arm).abs() < 1e-12);
        assert!((y + y_arm).abs() < 1e-12);
        let x_neg = (-radius) * theta.cos();
        let y_neg = (-radius) * theta.sin();
        assert!((x_neg - x_arm).abs() < 1e-12);
        assert!((y_neg - y_arm).abs() < 1e-12);
    }

    #[test]
    fn the_treatment_is_mathematics_only_and_cites_what_it_leans_on() {
        let blocks = super::blocks(&locale());
        assert!(!blocks.is_empty());
        for block in &blocks {
            assert_eq!(block.depth, StudyDepth::Mathematics);
            assert!(block.id.starts_with("fermat-spiral."));
            assert!(!block.parts.is_empty(), "{} is empty", block.id);
        }
        assert!(!blocks.iter().any(|b| b.depth == StudyDepth::Explanation));
        let references = blocks
            .iter()
            .flat_map(|block| &block.parts)
            .filter(|part| matches!(part, StudyPart::Reference { .. }))
            .count();
        assert_eq!(references, 2, "both cited sources must survive");
        let packing = blocks
            .iter()
            .find(|block| block.id == "fermat-spiral.packing")
            .expect("packing block");
        let packing_text: String = packing.parts.iter().map(StudyPart::plain_text).collect();
        assert!(
            packing_text.contains("Golden Angle"),
            "the complementary room must be named: {packing_text}"
        );
        assert!(
            !packing_text.to_lowercase().contains("prove that sunflower"),
            "must not claim a biological proof: {packing_text}"
        );
    }
}
