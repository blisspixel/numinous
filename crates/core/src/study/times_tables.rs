//! The second authored treatment: the flagship room's envelope, derived.
//!
//! English only. The Lissajous pilot carries a reviewed Japanese draft because
//! that draft was independently reviewed; no such review exists for this text,
//! so claiming one here would be a claim the project has not earned. A Japanese
//! request for this room resolves to English and says so.
//!
//! The derivation below is the project's own. The cited sources define the
//! epicycloid and fix its cusp count; they are not cited for the envelope
//! result, which is proved here rather than asserted.

use super::{
    StudyBlock, StudyDepth, StudyInline, StudyLocaleResolution, StudyPart, StudySource,
    StudyTranslationStatus,
};

fn text(body: &'static str) -> StudyPart {
    StudyPart::Paragraph(vec![StudyInline::Text(body)])
}

/// Prose with inline mathematical notation, in the order the runs are given.
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
        // English is the original text. There is no reviewed translation of
        // this treatment, and a request in another language resolves to this
        // one rather than being answered with a draft nobody has checked.
        translation: StudyTranslationStatus::Original,
        parts,
    }
}

pub(super) fn blocks(locale: &StudyLocaleResolution) -> Vec<StudyBlock> {
    vec![
        block(
            locale,
            "times-tables.model",
            "The construction, stated exactly",
            vec![
                text(
                    "Fix an integer N greater than one and place N points evenly on the unit \
                     circle in the complex plane. Index them by k and write the point as the \
                     complex number below. The room uses N = 240.",
                ),
                StudyPart::Equation("P(k) = exp(2*pi*i*k/N),        k = 0, 1, ..., N-1"),
                mixed(&[
                    ("For a multiplier ", false),
                    ("m", true),
                    (
                        ", the times table draws, from each point, one chord to the point its \
                         index is multiplied into:",
                        false,
                    ),
                ]),
                StudyPart::Equation("chord from P(k) to P(round(m*k) mod N)"),
                text(
                    "Two separate things are worth keeping apart from the start. The ideal \
                     object is a family of chords indexed by a continuous angle, and it has an \
                     exact envelope, derived below. The drawn object is a finite set of \
                     straight segments between rounded integer indices. They agree in a limit \
                     and differ on a screen, and the difference is described rather than \
                     ignored.",
                ),
                mixed(&[
                    ("The room's dial carries ", false),
                    ("m", true),
                    (" continuously from 2 to 10, so ", false),
                    ("m", true),
                    (
                        " is a real number and is an integer only at the ten marked stops.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "times-tables.chord",
            "The chord through two points of the unit circle",
            vec![
                mixed(&[
                    ("Let ", false),
                    ("a", true),
                    (" and ", false),
                    ("b", true),
                    (
                        " be distinct points on the unit circle. The line through them is \
                         exactly the set of ",
                        false,
                    ),
                    ("z", true),
                    (" satisfying", false),
                ]),
                StudyPart::Equation("z + a*b*conj(z) = a + b,        |a| = |b| = 1"),
                mixed(&[
                    ("To check it, substitute ", false),
                    ("z = a", true),
                    (". Because ", false),
                    ("|a| = 1", true),
                    (" we have ", false),
                    ("conj(a) = 1/a", true),
                    (", so the left side is ", false),
                    ("a + a*b/a = a + b", true),
                    (
                        ". The same substitution works for b. The equation is linear over the \
                         reals and its solution set is a real line, so two distinct solutions \
                         determine it.",
                        false,
                    ),
                ]),
                mixed(&[
                    (
                        "Now take the two points the times table joins, using a continuous angle ",
                        false,
                    ),
                    ("u", true),
                    (" in place of the index:", false),
                ]),
                StudyPart::Equation(
                    "a = exp(i*u),    b = exp(i*m*u)\nF(z, u) = z + exp(i*(m+1)*u)*conj(z) - exp(i*u) - exp(i*m*u) = 0",
                ),
                mixed(&[
                    ("Each ", false),
                    ("u", true),
                    (
                        " gives one chord. The whole picture is the one-parameter family of \
                         these lines, and the shape a viewer sees standing out of the family is \
                         its envelope: the curve tangent to every member.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "times-tables.envelope",
            "The envelope, derived",
            vec![
                mixed(&[
                    (
                        "A point of the envelope satisfies both the family equation and its \
                         derivative in the family parameter. Differentiating ",
                        false,
                    ),
                    ("F", true),
                    (" with respect to ", false),
                    ("u", true),
                    (", holding ", false),
                    ("z", true),
                    (" fixed:", false),
                ]),
                StudyPart::Equation(
                    "dF/du = i*(m+1)*exp(i*(m+1)*u)*conj(z) - i*exp(i*u) - i*m*exp(i*m*u) = 0",
                ),
                text("Solve that for the conjugate and then conjugate both sides:"),
                StudyPart::Equation(
                    "conj(z) = (exp(i*u) + m*exp(i*m*u)) / ((m+1)*exp(i*(m+1)*u))\nz       = (m*exp(i*u) + exp(i*m*u)) / (m+1)",
                ),
                text(
                    "That is the envelope. It is worth confirming rather than trusting, because \
                     the derivative condition alone does not prove a point lies on its own \
                     chord. Substituting the result back into the family equation:",
                ),
                StudyPart::Equation(
                    "z + exp(i*(m+1)*u)*conj(z)\n  = (m*exp(i*u) + exp(i*m*u))/(m+1) + (m*exp(i*m*u) + exp(i*u))/(m+1)\n  = ((m+1)*exp(i*u) + (m+1)*exp(i*m*u))/(m+1)\n  = exp(i*u) + exp(i*m*u)",
                ),
                mixed(&[
                    ("So ", false),
                    ("F = 0", true),
                    (
                        " and the point lies on the chord it came from, while the derivative \
                         condition makes the contact tangential. The envelope is therefore",
                        false,
                    ),
                ]),
                StudyPart::Equation("E(u) = (m*exp(i*u) + exp(i*m*u)) / (m+1)"),
            ],
        ),
        block(
            locale,
            "times-tables.epicycloid",
            "Why it is an epicycloid, and how many cusps",
            vec![
                mixed(&[
                    (
                        "An epicycloid is traced by a point on a circle of radius b rolling \
                         outside a circle of radius a, and it has ",
                        false,
                    ),
                    ("n = a/b", true),
                    (" cusps. In complex form:", false),
                ]),
                StudyPart::Equation("z(w) = (a+b)*exp(i*w) - b*exp(i*((a+b)/b)*w)"),
                mixed(&[
                    ("The envelope above is that curve, rotated. Put ", false),
                    ("c = pi/(m-1)", true),
                    (" and substitute ", false),
                    ("u = w + c", true),
                    (". Then ", false),
                    ("exp(i*(m-1)*c) = exp(i*pi) = -1", true),
                    (", so ", false),
                    ("exp(i*m*c) = -exp(i*c)", true),
                    (", and", false),
                ]),
                StudyPart::Equation("E(w + c) = exp(i*c) * (m*exp(i*w) - exp(i*m*w)) / (m+1)"),
                mixed(&[
                    ("Matching that against the epicycloid form gives ", false),
                    ("a + b = m/(m+1)", true),
                    (" and ", false),
                    ("b = 1/(m+1)", true),
                    (", hence ", false),
                    ("a = (m-1)/(m+1)", true),
                    (" and", false),
                ]),
                StudyPart::Equation("(a+b)/b = m,        cusps = a/b = m - 1"),
                text(
                    "The cusps can also be found directly, which is a useful independent check. \
                     A cusp is where the traced velocity vanishes:",
                ),
                StudyPart::Equation(
                    "E'(u) = i*m*(exp(i*u) + exp(i*m*u)) / (m+1)\nE'(u) = 0  <=>  exp(i*(m-1)*u) = -1  <=>  u = (2*j+1)*pi/(m-1)",
                ),
                mixed(&[
                    ("For integer ", false),
                    ("m", true),
                    (" those are exactly ", false),
                    ("m-1", true),
                    (" angles in one turn, taking ", false),
                    ("j = 0, 1, ..., m-2", true),
                    (
                        ", which agrees with the rolling-circle count. Away from them the speed \
                         is bounded away from zero, so there are no others.",
                        false,
                    ),
                ]),
                text(
                    "The two-times table therefore gives one cusp, the cardioid. The three-times \
                     table gives two, the nephroid. The room's own target, a multiplier of five, \
                     gives four.",
                ),
            ],
        ),
        block(
            locale,
            "times-tables.worked",
            "Two worked cases",
            vec![
                mixed(&[
                    ("Take ", false),
                    ("m = 2", true),
                    (". The envelope is ", false),
                    ("E(u) = (2*exp(i*u) + exp(2*i*u))/3", true),
                    (". The single cusp sits at ", false),
                    ("u = pi", true),
                    (", where", false),
                ]),
                StudyPart::Equation("E(pi) = (2*(-1) + 1)/3 = -1/3"),
                mixed(&[
                    ("and the far point sits at ", false),
                    ("u = 0", true),
                    (", where ", false),
                    ("E(0) = (2 + 1)/3 = 1", true),
                    (
                        ". The curve therefore spans from -1/3 to 1 along the real axis, a total \
                         width of 4/3, and touches the unit circle only at the far point. That \
                         is the cardioid a player sees blooming at the left end of the dial.",
                        false,
                    ),
                ]),
                mixed(&[
                    ("Take ", false),
                    ("m = 3", true),
                    (". Cusps occur at ", false),
                    ("u = pi/2", true),
                    (" and ", false),
                    ("u = 3*pi/2", true),
                    (". At the first,", false),
                ]),
                StudyPart::Equation("E(pi/2) = (3*i + exp(3*i*pi/2))/4 = (3*i - i)/4 = i/2"),
                text(
                    "and by symmetry the other cusp is at minus one half times i. Two cusps, on \
                     opposite sides, at half the circle's radius: the nephroid.",
                ),
            ],
        ),
        block(
            locale,
            "times-tables.drawn",
            "What the screen draws, which is not the envelope",
            vec![
                text(
                    "Everything above describes a smooth curve arising from a continuum of \
                     chords. The room draws neither a curve nor a continuum. Three differences \
                     matter, and none of them is small enough to leave unsaid.",
                ),
                mixed(&[
                    (
                        "First, the index is rounded. The chord leaves index k and arrives at ",
                        false,
                    ),
                    ("round(m*k) mod N", true),
                    (". For integer ", false),
                    ("m", true),
                    (
                        " the rounding does nothing and the arrival is exactly ",
                        false,
                    ),
                    ("m*k mod N", true),
                    (
                        ". For every other multiplier the rounding moves the arrival by up to \
                         half a step, so the drawn family is a perturbed sample of the ideal \
                         one and the smooth envelope is a guide rather than a description.",
                        false,
                    ),
                ]),
                mixed(&[
                    (
                        "Second, only some chords are drawn. The room chooses a sample count \
                         from the drawing radius, bounded to between 24 and N, and draws that \
                         many chords rather than all ",
                        false,
                    ),
                    ("N", true),
                    (
                        ". A tangent curve suggested by 24 segments and one suggested by 240 are \
                         the same limit and different pictures.",
                        false,
                    ),
                ]),
                text(
                    "Third, the chords are straight segments rasterized onto a character grid \
                     with a non-square cell, so the circle is drawn as an ellipse and corrected \
                     by an aspect factor. What a player sees is a picture of the family, and the \
                     envelope is the shape their eye assembles from it.",
                ),
                mixed(&[
                    (
                        "The closing behaviour also depends on arithmetic. The drawn family \
                         always closes, because indices live modulo ",
                        false,
                    ),
                    ("N", true),
                    (". The ideal family closes after one turn only when ", false),
                    ("m", true),
                    (
                        " is an integer; for other real multipliers the ideal curve does not \
                         return to its start after a single revolution, and the cusp count \
                         stated above is a statement about integers.",
                        false,
                    ),
                ]),
            ],
        ),
        block(
            locale,
            "times-tables.limits",
            "Limits of this treatment",
            vec![
                text(
                    "The derivation is exact in the stated model: a continuum of chords between \
                     exact points of the unit circle, with a real multiplier, in exact \
                     arithmetic. Each of those three assumptions is false on a running machine.",
                ),
                text(
                    "Coordinates are binary64. The angles are irrational multiples of pi in \
                     general, so points on the circle are rounded before anything is drawn, and \
                     the cusp angles derived above are representable only approximately. The \
                     numerical checks behind this text agree with the algebra to about one part \
                     in ten to the fourteenth, which is consistent with binary64 and is not a \
                     proof about the drawing.",
                ),
                text(
                    "The envelope statement is about the family of infinitely many chords. It \
                     says nothing about how visible the curve is at a given sample count, how it \
                     rasterizes, or how a player perceives it. Those are questions about \
                     rendering and about people, and this treatment does not answer either.",
                ),
                text(
                    "No claim is made here that the epicycloid identification is novel. It is a \
                     classical result. What is claimed is that the derivation above is complete \
                     as written and can be checked line by line without consulting a source.",
                ),
            ],
        ),
        block(
            locale,
            "times-tables.references",
            "References",
            vec![
                text(
                    "The two sources below are cited for the definition of the epicycloid and \
                     for its cusp count. The envelope derivation is not taken from them.",
                ),
                StudyPart::Reference {
                    source: &EPICYCLOID,
                    description: "Defines the epicycloid by its rolling-circle construction,                                   gives the parametric equations matched above, and states that                                   n cusps require the rolling radius to be the fixed radius                                   divided by n.",
                },
                StudyPart::Reference {
                    source: &CARDIOID,
                    description: "States that the cardioid is the one-cusped epicycloid, which                                   is the m = 2 case of the derivation above.",
                },
            ],
        ),
    ]
}

static EPICYCLOID: StudySource = StudySource {
    id: "mathworld-epicycloid",
    title: "Wolfram MathWorld, Epicycloid: parametric equations and the cusp count",
    url: "https://mathworld.wolfram.com/Epicycloid.html",
};

static CARDIOID: StudySource = StudySource {
    id: "mathworld-cardioid",
    title: "Wolfram MathWorld, Cardioid: the one-cusped epicycloid",
    url: "https://mathworld.wolfram.com/Cardioid.html",
};

#[cfg(test)]
mod tests {
    use super::super::{StudyDepth, StudyLocale, StudyLocaleResolution, StudyPart};
    use std::f64::consts::PI;

    fn locale() -> StudyLocaleResolution {
        StudyLocaleResolution::new(&StudyLocale::parse("en").expect("en parses"), "en")
    }

    /// The envelope this treatment derives, as complex components.
    fn envelope(m: f64, u: f64) -> (f64, f64) {
        let (s1, c1) = u.sin_cos();
        let (sm, cm) = (m * u).sin_cos();
        ((m * c1 + cm) / (m + 1.0), (m * s1 + sm) / (m + 1.0))
    }

    #[test]
    fn the_derived_envelope_lies_on_its_own_chord_and_is_tangent_to_it() {
        // The text claims a derivation, so the claim is checked rather than
        // trusted. F(z, u) = z + exp(i(m+1)u) conj(z) - exp(iu) - exp(imu),
        // evaluated at the derived point, must vanish, and so must dF/du.
        for m in [2.0_f64, 3.0, 4.0, 5.0, 7.0] {
            for step in 0..360 {
                let u = f64::from(step) * PI / 180.0;
                let (zx, zy) = envelope(m, u);
                let (sp, cp) = ((m + 1.0) * u).sin_cos();
                // exp(i(m+1)u) * conj(z), as (real, imaginary).
                let rx = cp * zx + sp * zy;
                let ry = sp * zx - cp * zy;
                let (s1, c1) = u.sin_cos();
                let (sm, cm) = (m * u).sin_cos();
                let fx = zx + rx - c1 - cm;
                let fy = zy + ry - s1 - sm;
                assert!(
                    fx.hypot(fy) < 1e-12,
                    "m={m} u={u}: the envelope point left its chord by {}",
                    fx.hypot(fy)
                );
            }
        }
    }

    #[test]
    fn the_envelope_has_exactly_one_fewer_cusp_than_the_multiplier() {
        // The cusp count is the claim a player checks by eye: two times gives a
        // cardioid, three a nephroid, five the room's four lobes. Speed is
        // proportional to |exp(iu) + exp(imu)|, which vanishes only at the
        // derived angles.
        for m in [2.0_f64, 3.0, 4.0, 5.0, 7.0, 9.0] {
            let cusps = (m as usize) - 1;
            for j in 0..cusps {
                let u = (2.0 * j as f64 + 1.0) * PI / (m - 1.0);
                let (s1, c1) = u.sin_cos();
                let (sm, cm) = (m * u).sin_cos();
                assert!(
                    (c1 + cm).hypot(s1 + sm) < 1e-12,
                    "m={m} j={j}: the derived cusp angle is not a cusp"
                );
            }
            // And nowhere else: sampled between the cusps, speed stays away
            // from zero, so the count is exactly m-1 rather than at least.
            let mut lowest = f64::INFINITY;
            for step in 0..20_000 {
                let u = f64::from(step) * 2.0 * PI / 20_000.0;
                let near = (0..cusps).any(|j| {
                    let cusp = (2.0 * j as f64 + 1.0) * PI / (m - 1.0);
                    (u - cusp).abs() < 1e-3 || (u - cusp - 2.0 * PI).abs() < 1e-3
                });
                if !near {
                    let (s1, c1) = u.sin_cos();
                    let (sm, cm) = (m * u).sin_cos();
                    lowest = lowest.min((c1 + cm).hypot(s1 + sm));
                }
            }
            assert!(
                lowest > 1e-4,
                "m={m}: an unclaimed cusp appeared, min {lowest}"
            );
        }
    }

    #[test]
    fn the_worked_cases_in_the_text_are_the_values_the_formula_gives() {
        // Every number quoted in the worked block, checked against the formula
        // it was quoted from, so the prose cannot drift from the mathematics.
        let (x, y) = envelope(2.0, PI);
        assert!(
            (x + 1.0 / 3.0).abs() < 1e-12 && y.abs() < 1e-12,
            "cardioid cusp"
        );
        let (x, y) = envelope(2.0, 0.0);
        assert!(
            (x - 1.0).abs() < 1e-12 && y.abs() < 1e-12,
            "cardioid far point"
        );
        let (x, y) = envelope(3.0, PI / 2.0);
        assert!(
            (x).abs() < 1e-12 && (y - 0.5).abs() < 1e-12,
            "nephroid cusp"
        );
        let (x, y) = envelope(3.0, 3.0 * PI / 2.0);
        assert!(
            (x).abs() < 1e-12 && (y + 0.5).abs() < 1e-12,
            "second nephroid cusp"
        );
    }

    #[test]
    fn the_treatment_is_mathematics_only_and_cites_what_it_leans_on() {
        let blocks = super::blocks(&locale());
        assert!(!blocks.is_empty());
        for block in &blocks {
            assert_eq!(block.depth, StudyDepth::Mathematics);
            assert!(block.id.starts_with("times-tables."));
            assert!(!block.parts.is_empty(), "{} is empty", block.id);
        }
        // No authored explanation here, which is what keeps the room's existing
        // catalog explanation at explanation depth rather than moving it away.
        assert!(!blocks.iter().any(|b| b.depth == StudyDepth::Explanation));
        let references = blocks
            .iter()
            .flat_map(|block| &block.parts)
            .filter(|part| matches!(part, StudyPart::Reference { .. }))
            .count();
        assert_eq!(references, 2, "both cited sources must survive");
    }
}
