//! What a color-blind player sees, and whether two marks stay apart for them.
//!
//! `crate::ansi::to_mono` answers the question for a player with no color at
//! all. It does not answer this one. Roughly one man in twelve has a red-green
//! deficiency and sees color, just fewer distinctions, so a cue that survives
//! the color-free renderer can still vanish for them.
//!
//! The standard is WCAG 1.4.1, Use of Color: color must not be the only visual
//! means of conveying information. This module measures the "only" part.
//!
//! Two pieces, both published rather than invented here:
//!
//! - The dichromacy simulation is Vienot, Brettel and Mollon 1999, the same one
//!   accessibility tooling uses. Linear sRGB goes to LMS cone response, the
//!   missing cone's contribution is replaced by a projection along the
//!   confusion axis, and the result comes back to sRGB.
//! - The distance is CIELAB delta-E 1976. It is used because a difference in
//!   raw RGB says nothing about whether an eye can tell two colors apart, and
//!   because these are large flat areas of a single color rather than fine
//!   detail, which is the case delta-E 1976 handles least badly.
//!
//! What the numbers here are NOT: a claim that a passing pair is comfortable,
//! or that a failing pair is invisible. Delta-E is a threshold model and real
//! dichromacy varies between people. The gate is a floor, and the module says
//! so rather than implying a verdict on how the catalog looks.
//!
//! One deliberate omission, stated rather than left to be discovered:
//! anomalous trichromacy (protanomaly, deuteranomaly, tritanomaly), which is
//! more common than the dichromacies simulated here, is not modelled. It is a
//! weakened cone rather than a missing one, so it lies between normal vision
//! and the simulations below. Measuring the dichromacies is therefore the
//! conservative end of the range for those players, not a substitute for
//! measuring them.

/// The three dichromacies: one cone missing, three ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dichromacy {
    /// No long-wavelength cone. Red darkens toward the background.
    Protanopia,
    /// No medium-wavelength cone. The most common, and the one that folds red
    /// and green together.
    Deuteranopia,
    /// No short-wavelength cone. Rare, and it folds blue into green.
    Tritanopia,
}

impl Dichromacy {
    /// All three, so a sweep cannot quietly check only the convenient one.
    pub const ALL: [Self; 3] = [Self::Protanopia, Self::Deuteranopia, Self::Tritanopia];

    /// The name a failing test should print.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
        }
    }
}

/// Linear sRGB to LMS cone response, Vienot 1999.
const RGB_TO_LMS: [[f64; 3]; 3] = [
    [17.8824, 43.5161, 4.11935],
    [3.45565, 27.1554, 3.86714],
    [0.0299566, 0.184309, 1.46709],
];

/// The inverse of [`RGB_TO_LMS`].
const LMS_TO_RGB: [[f64; 3]; 3] = [
    [0.080_944_447_9, -0.130_504_409, 0.116_721_066],
    [-0.010_248_533_5, 0.054_019_326_6, -0.113_614_708],
    [-0.000_365_296_938, -0.004_121_614_69, 0.693_511_405],
];

impl Dichromacy {
    /// The projection in LMS that replaces the missing cone.
    fn projection(self) -> [[f64; 3]; 3] {
        match self {
            Self::Protanopia => [[0.0, 2.023_44, -2.525_81], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            Self::Deuteranopia => [[1.0, 0.0, 0.0], [0.494_207, 0.0, 1.248_27], [0.0, 0.0, 1.0]],
            Self::Tritanopia => [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-0.395_913, 0.801_109, 0.0],
            ],
        }
    }
}

fn apply(matrix: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (row, slot) in matrix.iter().zip(out.iter_mut()) {
        *slot = row[0] * v[0] + row[1] * v[1] + row[2] * v[2];
    }
    out
}

/// What `rgb` looks like to someone with `kind`.
///
/// Round-trips through linear light, so it is not a channel swap: a color that
/// is already on the dichromat's confusion plane comes back unchanged, which is
/// what makes [`simulate`] a projection rather than a filter.
#[must_use]
pub fn simulate(rgb: [u8; 3], kind: Dichromacy) -> [u8; 3] {
    let table = crate::photosensitivity::linear_channel_table();
    let linear = [
        table[rgb[0] as usize],
        table[rgb[1] as usize],
        table[rgb[2] as usize],
    ];
    let lms = apply(&RGB_TO_LMS, linear);
    let projected = apply(&kind.projection(), lms);
    let back = apply(&LMS_TO_RGB, projected);
    [encode(back[0]), encode(back[1]), encode(back[2])]
}

/// Linear light back to one sRGB byte, clamped.
fn encode(linear: f64) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let encoded = if clamped <= 0.003_130_8 {
        12.92 * clamped
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

/// CIELAB, D65, from sRGB.
fn lab(rgb: [u8; 3]) -> [f64; 3] {
    let table = crate::photosensitivity::linear_channel_table();
    let (r, g, b) = (
        table[rgb[0] as usize],
        table[rgb[1] as usize],
        table[rgb[2] as usize],
    );
    let x = 0.4124 * r + 0.3576 * g + 0.1805 * b;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = 0.0193 * r + 0.1192 * g + 0.9505 * b;
    // D65 white, the illuminant sRGB is defined against.
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (841.0 / 108.0) * t + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x / 0.95047), f(y / 1.0), f(z / 1.08883));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// How far apart two colors are, CIELAB delta-E 1976.
///
/// Zero means identical. Around 2.3 is the just-noticeable difference for
/// adjacent patches under good conditions, which is far tighter than anything
/// this module asserts.
#[must_use]
pub fn distance(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (lab(a), lab(b));
    ((la[0] - lb[0]).powi(2) + (la[1] - lb[1]).powi(2) + (la[2] - lb[2]).powi(2)).sqrt()
}

/// The dichromacy that brings two colors closest together, and how close.
#[must_use]
pub fn worst_case(a: [u8; 3], b: [u8; 3]) -> (Dichromacy, f64) {
    let mut worst = (Dichromacy::Protanopia, f64::INFINITY);
    for kind in Dichromacy::ALL {
        let d = distance(simulate(a, kind), simulate(b, kind));
        if d < worst.1 {
            worst = (kind, d);
        }
    }
    worst
}

/// Below this, two large flat areas of color read as one.
pub const TOO_CLOSE: f64 = 25.0;

/// Above this, ordinary color vision separates two colors comfortably.
///
/// The two thresholds together are what make the measurement mean something. A
/// bare "closer than [`TOO_CLOSE`] for a dichromat" also catches pairs that are
/// nearly identical for everyone, and those are a contrast problem tracked
/// elsewhere. A pair that only a dichromat loses is the color-alone cue this
/// module is looking for.
pub const SEPARATE_NORMALLY: f64 = 40.0;

/// Whether a pair carries its meaning in color alone: clear for ordinary
/// vision, gone for at least one dichromat.
#[must_use]
pub fn color_alone(a: [u8; 3], b: [u8; 3]) -> bool {
    distance(a, b) >= SEPARATE_NORMALLY && worst_case(a, b).1 < TOO_CLOSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_survives_every_dichromacy() {
        // The achromatic axis is what all three cone types agree on, so a
        // simulation that shifts grey has its matrices wrong. This is the
        // cheapest check that the transform is a projection of color rather
        // than a tint applied to everything.
        for level in [0u8, 1, 64, 128, 200, 254, 255] {
            for kind in Dichromacy::ALL {
                let out = simulate([level; 3], kind);
                for channel in out {
                    assert!(
                        channel.abs_diff(level) <= 2,
                        "{} moved grey {level} to {out:?}",
                        kind.name()
                    );
                }
            }
        }
    }

    #[test]
    fn simulating_twice_is_the_same_as_simulating_once() {
        // A projection is idempotent by definition: once a color is on the
        // dichromat's plane, projecting again cannot move it. This catches a
        // transposed or mistyped matrix that the eyeball test would pass,
        // because the output would still look plausibly color-blind.
        for color in [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [230, 72, 72],
            [216, 40, 190],
            [56, 224, 132],
            [242, 148, 36],
            [116, 72, 232],
        ] {
            for kind in Dichromacy::ALL {
                let once = simulate(color, kind);
                let twice = simulate(once, kind);
                let drift = distance(once, twice);
                assert!(
                    drift < 1.0,
                    "{} is not a projection: {color:?} settles at {once:?} then moves to \
                     {twice:?}, CIELAB {drift:.2}",
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn blue_survives_a_red_green_deficiency() {
        // Protanopia and deuteranopia leave the short-wavelength cone intact,
        // so blue must come through. If it does not, the projection is
        // touching a row it has no business touching.
        for kind in [Dichromacy::Protanopia, Dichromacy::Deuteranopia] {
            let out = simulate([0, 0, 255], kind);
            assert!(
                distance(out, [0, 0, 255]) < 5.0,
                "{} moved pure blue to {out:?}",
                kind.name()
            );
        }
    }

    #[test]
    fn red_and_green_collapse_for_a_deuteranope_and_not_for_anyone_else() {
        // The one result everybody already knows, used as an external anchor:
        // if the simulation did not reproduce the textbook confusion it would
        // be wrong no matter how self-consistent it was.
        let normal = distance([255, 0, 0], [0, 255, 0]);
        assert!(
            normal > 150.0,
            "red and green are {normal:.1} apart normally"
        );

        let (kind, worst) = worst_case([255, 0, 0], [0, 255, 0]);
        assert_eq!(
            kind,
            Dichromacy::Deuteranopia,
            "the worst case for red against green should be deuteranopia"
        );
        assert!(
            worst < normal / 4.0,
            "deuteranopia should fold red and green together, but {worst:.1} against a \
             normal {normal:.1} is barely a fold"
        );
    }

    #[test]
    fn the_color_alone_rule_needs_both_halves() {
        // Two colors that nobody can tell apart are a contrast defect, not a
        // color-alone cue, and must not be reported here. Two that only a
        // dichromat loses are exactly what this is for.
        assert!(
            !color_alone([100, 100, 100], [104, 104, 104]),
            "a pair that is close for everyone is not a color-alone cue"
        );
        // The positive case is measured rather than assumed. Saturated red
        // against saturated green is the example everybody reaches for and it
        // is the wrong one: a deuteranope sees both as yellow but at clearly
        // different lightnesses, and they stay 30 apart, so the rule correctly
        // lets them through. The pair that actually fails is from the catalog,
        // `times-tables` drawing its spectral `'@'` over its own accent, which
        // ordinary vision separates by 95 and a deuteranope by under 1.
        let magenta = [216, 40, 190];
        let accent = [40, 150, 190];
        assert!(
            color_alone(magenta, accent),
            "the spectral magenta against a blue accent is {:.1} normally and {:.1} for a \
             {}, which is the defect this rule exists to name",
            distance(magenta, accent),
            worst_case(magenta, accent).1,
            worst_case(magenta, accent).0.name()
        );
        assert!(
            !color_alone([0, 0, 0], [255, 255, 255]),
            "black against white survives every dichromacy"
        );
    }
}

/// The room-by-room color-independence audit, written where a person can read
/// it rather than only asserted inside a test.
///
/// The sweeps in `raster.rs` and `registry.rs` decide whether the catalog has
/// regressed. They cannot show what was covered or by how much: a reader has to
/// take the coverage on trust, and a passing test looks identical whether it
/// measured 355 rooms or none. This module writes the measurement out, so the
/// claim is checkable and the margins are visible, not just the failures.
///
/// The file is `docs/evidence/color-independence.json` and it is generated,
/// never hand-edited. Regenerate with `NUMINOUS_UPDATE_EVIDENCE=1 cargo test -p
/// numinous-core --lib color_independence_audit` after an intentional change.
#[cfg(test)]
pub(crate) mod audit {
    use super::{Dichromacy, color_alone, distance, worst_case};

    /// Every mark that paints something other than the plain accent, plus the
    /// one representative for the marks that do paint it.
    const INK_MARKS: [char; 6] = ['#', '!', '@', '%', '&', '~'];

    /// Marks that all paint the plain accent. Recorded as `'*'` whichever one a
    /// room happens to use.
    const ORDINARY: [char; 3] = ['*', '+', '.'];

    /// One room's measured worst pair.
    pub(crate) struct RoomAudit {
        pub id: String,
        pub marks: Vec<char>,
        pub worst: Option<(char, char, f64, f64, Dichromacy)>,
        pub color_alone_pairs: Vec<(char, char)>,
    }

    /// Measure one room's palette: every pair of marks it draws, and which is
    /// closest for a dichromat.
    pub(crate) fn audit_room(id: &str, accent: [u8; 3], drawn: &[char]) -> RoomAudit {
        let raster = crate::raster::Raster::with_accent(1, 1, accent);
        // Collapse the accent-painting marks to one name: a room drawing '*'
        // and '.' is drawing one color, and pairing them with each other would
        // record a distinction the room never made.
        let mut marks: Vec<char> = Vec::new();
        for &mark in drawn {
            let name = if ORDINARY.contains(&mark) { '*' } else { mark };
            if (INK_MARKS.contains(&name) || name == '*') && !marks.contains(&name) {
                marks.push(name);
            }
        }
        marks.sort_unstable();

        let mut worst: Option<(char, char, f64, f64, Dichromacy)> = None;
        let mut color_alone_pairs = Vec::new();
        for (index, &first) in marks.iter().enumerate() {
            for &second in marks.iter().skip(index + 1) {
                let (a, b) = (raster.ink(first), raster.ink(second));
                if a == b {
                    continue;
                }
                let normal = distance(a, b);
                let (kind, folded) = worst_case(a, b);
                if worst.is_none_or(|(_, _, _, seen, _)| folded < seen) {
                    worst = Some((first, second, normal, folded, kind));
                }
                if color_alone(a, b) {
                    color_alone_pairs.push((first, second));
                }
            }
        }
        RoomAudit {
            id: id.to_string(),
            marks,
            worst,
            color_alone_pairs,
        }
    }

    /// The audit as JSON, formatted deterministically.
    ///
    /// Distances are rounded to one decimal before formatting. They are `f64`
    /// derived from a cube root and a square root, and committing full
    /// precision would make the file a record of the host's floating point
    /// rather than of the catalog. One decimal is far finer than any threshold
    /// this module uses.
    pub(crate) fn to_json(rooms: &[RoomAudit]) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schemaVersion\": \"numinous-color-independence-v1\",\n");
        out.push_str("  \"evidenceClass\": \"agent-machine-regression\",\n");
        out.push_str("  \"method\": {\n");
        out.push_str("    \"simulation\": \"Vienot Brettel Mollon 1999\",\n");
        out.push_str("    \"distance\": \"CIELAB delta-E 1976\",\n");
        out.push_str(&format!(
            "    \"separateNormally\": {:.1},\n",
            super::SEPARATE_NORMALLY
        ));
        out.push_str(&format!("    \"tooClose\": {:.1},\n", super::TOO_CLOSE));
        out.push_str("    \"notModelled\": [\n");
        out.push_str("      \"anomalous trichromacy, which is more common than dichromacy\",\n");
        out.push_str("      \"the WCAG 2.3.1 flashing area rule, measured whole-frame instead\"\n");
        out.push_str("    ]\n");
        out.push_str("  },\n");
        out.push_str(&format!("  \"roomsAudited\": {},\n", rooms.len()));
        out.push_str(&format!(
            "  \"roomsWithAColorAlonePair\": {},\n",
            rooms
                .iter()
                .filter(|r| !r.color_alone_pairs.is_empty())
                .count()
        ));
        out.push_str("  \"rooms\": [\n");
        for (index, room) in rooms.iter().enumerate() {
            let marks: String = room.marks.iter().collect();
            out.push_str("    {");
            out.push_str(&format!("\"id\": \"{}\", ", room.id));
            out.push_str(&format!("\"marks\": \"{marks}\", "));
            match room.worst {
                Some((a, b, normal, folded, kind)) => {
                    out.push_str(&format!(
                        "\"closestPair\": \"{a}{b}\", \"normal\": {normal:.1}, \
                         \"folded\": {folded:.1}, \"dichromacy\": \"{}\", ",
                        kind.name()
                    ));
                }
                None => out.push_str("\"closestPair\": null, "),
            }
            let flagged: Vec<String> = room
                .color_alone_pairs
                .iter()
                .map(|(a, b)| format!("\"{a}{b}\""))
                .collect();
            out.push_str(&format!("\"colorAlone\": [{}]", flagged.join(", ")));
            out.push('}');
            if index + 1 < rooms.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
        out.push_str("}\n");
        out
    }
}
