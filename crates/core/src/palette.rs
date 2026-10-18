//! The shared palette: the stage every frame sits on, and the lightness band
//! every room accent lives in.
//!
//! A room is free to choose its hue; that is its signature. It is not free to
//! choose how light that hue is, because lightness is what the rest of the
//! system reads. The mark ramp scales an accent to draw faint, secondary and
//! hot marks, the color-free renderer reads brightness, and the dichromacy
//! audits read lightness when hue fails. An accent too dark leaves no room
//! below it for a faint mark to stay visible; an accent too light leaves no
//! room above it for a hot mark to stand out, because the byte scale clamps.
//! Both ends lost a level for players without color, which is the defect the
//! band exists to remove (see `docs/VISUALS.md`, "One accent, one band").

/// The near-black stage every frame is drawn on. Never pure black, so the
/// darkest light still reads as light rather than as a hole.
pub const STAGE: [u8; 3] = [10, 11, 15];

/// The CIELAB lightness every room accent sits within, inclusive.
pub const ACCENT_LIGHTNESS: (f64, f64) = (40.0, 72.0);

/// The least WCAG contrast ratio an accent keeps against [`STAGE`]: the
/// 1.4.11 floor for graphical objects a player has to see.
pub const ACCENT_STAGE_CONTRAST: f64 = 3.0;

/// Whether an accent sits in the band: lightness within
/// [`ACCENT_LIGHTNESS`] and contrast against the stage of at least
/// [`ACCENT_STAGE_CONTRAST`].
#[must_use]
pub fn accent_in_band(accent: [u8; 3]) -> bool {
    let lightness = crate::dichromacy::lightness(accent);
    let (low, high) = ACCENT_LIGHTNESS;
    (low..=high).contains(&lightness) && stage_contrast(accent) >= ACCENT_STAGE_CONTRAST
}

/// Whether `color` sits in the magenta and orchid band.
///
/// Hue is HSV, 285 to 340 degrees inclusive, and saturation is over 0.35.
/// New room accents stay outside this band. Accents already in the catalog
/// are listed beside the test and are not retuned.
#[must_use]
pub fn in_magenta_band(color: [u8; 3]) -> bool {
    let [hue, saturation] = hue_and_saturation(color);
    saturation > 0.35 && (285.0..=340.0).contains(&hue)
}

/// HSV hue in degrees and saturation, both zero for grey.
fn hue_and_saturation(color: [u8; 3]) -> [f64; 2] {
    let channel = |byte: u8| f64::from(byte) / 255.0;
    let (red, green, blue) = (channel(color[0]), channel(color[1]), channel(color[2]));
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let delta = max - min;
    if max == 0.0 || delta == 0.0 {
        return [0.0, 0.0];
    }
    let hue = if max == red {
        60.0 * ((green - blue) / delta)
    } else if max == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    [hue, delta / max]
}

/// The WCAG contrast ratio of a color against [`STAGE`].
#[must_use]
pub fn stage_contrast(color: [u8; 3]) -> f64 {
    let luminance =
        |rgb: [u8; 3]| crate::photosensitivity::relative_luminance(rgb[0], rgb[1], rgb[2]);
    let (a, b) = (luminance(color), luminance(STAGE));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::{
        ACCENT_LIGHTNESS, ACCENT_STAGE_CONTRAST, STAGE, accent_in_band, in_magenta_band,
        stage_contrast,
    };
    use crate::dichromacy;

    /// CIELAB (D65) back to linear sRGB, the inverse of the conversion the
    /// audits use.
    fn lab_to_linear([lightness, a, b]: [f64; 3]) -> [f64; 3] {
        let fy = (lightness + 16.0) / 116.0;
        let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
        let inverse = |t: f64| {
            if t.powi(3) > 216.0 / 24389.0 {
                t.powi(3)
            } else {
                (t - 4.0 / 29.0) * 108.0 / 841.0
            }
        };
        let (x, y, z) = (inverse(fx) * 0.95047, inverse(fy), inverse(fz) * 1.08883);
        [
            3.2406 * x - 1.5372 * y - 0.4986 * z,
            -0.9689 * x + 1.8758 * y + 0.0415 * z,
            0.0557 * x - 0.2040 * y + 1.0570 * z,
        ]
    }

    fn encode(linear: f64) -> u8 {
        let clamped = linear.clamp(0.0, 1.0);
        let coded = if clamped <= 0.003_130_8 {
            12.92 * clamped
        } else {
            1.055 * clamped.powf(1.0 / 2.4) - 0.055
        };
        (coded * 255.0).round() as u8
    }

    /// How far inside the band a moved accent settles, in L*.
    ///
    /// Measured rather than chosen: half a unit in, four warm yellows that
    /// had come down from above kept their hot level only 24 apart for a
    /// tritanope, inside the fold the audits guard; four units in, no moved
    /// accent folds a pair that was apart before.
    const SETTLE: f64 = 4.0;

    /// The rule that moved the catalog's accents into the band, kept as the
    /// fix a failing accent is offered: the same CIELAB hue at the nearest
    /// lightness [`SETTLE`] inside the band, with chroma given up only as far
    /// as the sRGB gamut requires at that lightness.
    fn nearest_in_band(accent: [u8; 3]) -> [u8; 3] {
        if accent_in_band(accent) {
            return accent;
        }
        let [lightness, a, b] = dichromacy::lab(accent);
        let (low, high) = ACCENT_LIGHTNESS;
        let target = lightness.clamp(low + SETTLE, high - SETTLE);
        let fits = |keep: f64| {
            lab_to_linear([target, a * keep, b * keep])
                .iter()
                .all(|channel| (0.0..=1.0).contains(channel))
        };
        let keep = if fits(1.0) {
            1.0
        } else {
            let (mut inside, mut outside) = (0.0, 1.0);
            for _ in 0..48 {
                let middle = (inside + outside) / 2.0;
                if fits(middle) {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            inside
        };
        lab_to_linear([target, a * keep, b * keep]).map(encode)
    }

    #[test]
    fn every_room_accent_sits_in_the_band() {
        let mut outside = Vec::new();
        let rooms = crate::rooms::ROOM_CATALOG
            .iter()
            .chain(crate::rooms::HIDDEN_ROOM_METADATA);
        let mut checked = 0;
        for room in rooms {
            checked += 1;
            if !accent_in_band(room.accent) {
                outside.push(format!(
                    "{} {:?} (L* {:.1}, {:.2}:1) -> {:?}",
                    room.id,
                    room.accent,
                    dichromacy::lightness(room.accent),
                    stage_contrast(room.accent),
                    nearest_in_band(room.accent)
                ));
            }
        }
        assert!(checked > 300, "only {checked} rooms checked");
        assert!(
            outside.is_empty(),
            "{} accents sit outside the lightness band; each is shown with the \
             same hue moved into it:\n{}",
            outside.len(),
            outside.join("\n")
        );
    }

    #[test]
    fn the_band_and_the_contrast_floor_agree() {
        // The lower edge of the band is where the contrast floor lands, so
        // the two rules cannot pull against each other: both read the same
        // relative luminance, and anything light enough for the band is far
        // enough from the stage. Swept over a lattice of the whole cube.
        let (low, _) = ACCENT_LIGHTNESS;
        let levels: Vec<u8> = (0..=255u8).step_by(15).collect();
        let mut light_enough = 0;
        for &r in &levels {
            for &g in &levels {
                for &b in &levels {
                    let color = [r, g, b];
                    if dichromacy::lightness(color) >= low {
                        light_enough += 1;
                        assert!(stage_contrast(color) >= ACCENT_STAGE_CONTRAST, "{color:?}");
                    }
                }
            }
        }
        assert!(
            light_enough > 1_000,
            "the sweep reached only {light_enough} colors"
        );
        assert!((stage_contrast(STAGE) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_rule_keeps_hue_and_lands_inside_the_band() {
        for accent in [
            [40, 40, 40],
            [255, 200, 40],
            [20, 20, 120],
            [240, 220, 120],
            [200, 40, 40],
            [90, 220, 130],
        ] {
            let moved = nearest_in_band(accent);
            assert!(accent_in_band(moved), "{accent:?} -> {moved:?}");
            let [_, a0, b0] = dichromacy::lab(accent);
            let [_, a1, b1] = dichromacy::lab(moved);
            if a0.hypot(b0) > 10.0 {
                let turn = (b1.atan2(a1) - b0.atan2(a0)).abs();
                let turn = turn.min(std::f64::consts::TAU - turn);
                assert!(turn < 0.12, "{accent:?} -> {moved:?} turned {turn:.3} rad");
            }
        }
        // An accent already inside is its own answer.
        assert_eq!(nearest_in_band([40, 150, 190]), [40, 150, 190]);
    }

    /// Catalog accents already inside the magenta band. A new room is not
    /// added here. A room that leaves the band leaves this list.
    const GRANDFATHERED_MAGENTA_ACCENTS: [&str; 40] = [
        "audioactive",
        "basel",
        "berry",
        "bogdanov",
        "butterfly-curve",
        "cantor-set",
        "cassini",
        "cauchy-lorentz",
        "curse-dimension",
        "function-painter",
        "gerono",
        "harmonograph",
        "henon",
        "hopf",
        "lemniscate",
        "liouville",
        "lyapunov",
        "menagerie",
        "multibrot",
        "nova",
        "oregonator",
        "parrondo",
        "pascal-mod",
        "pedal",
        "polarization",
        "prime-spirals",
        "pseudosphere",
        "rabinovich-fabrikant",
        "roman-surface",
        "rose",
        "roulette",
        "sierpinski-carpet",
        "snell-prism",
        "standard-map",
        "the-stretch",
        "tinkerbell",
        "torus-knot",
        "trisectrix",
        "wallis",
        "witch-of-agnesi",
    ];

    #[test]
    fn the_magenta_band_rejects_new_hues_and_keeps_the_catalog_list() {
        assert!(in_magenta_band([216, 40, 190]));
        assert!(in_magenta_band([190, 70, 170]));
        assert!(in_magenta_band([255, 120, 180]));
        let teal = crate::raster::MarkRole::Teal.ink([0, 0, 0]);
        assert_eq!(teal, [7, 118, 140]);
        assert!(!in_magenta_band(teal));
        assert!(
            accent_in_band(teal),
            "the teal ink is outside the accent band, L* {:.1}",
            dichromacy::lightness(teal)
        );
        for role in crate::raster::MarkRole::ALL {
            if role.is_accent_level() {
                continue;
            }
            let ink = role.ink([0, 0, 0]);
            assert!(
                !in_magenta_band(ink),
                "{role:?} ink {ink:?} is in the magenta band"
            );
        }
        let mut found = Vec::new();
        let rooms = crate::rooms::ROOM_CATALOG
            .iter()
            .chain(crate::rooms::HIDDEN_ROOM_METADATA);
        for room in rooms {
            if in_magenta_band(room.accent) {
                found.push(room.id);
            }
        }
        found.sort_unstable();
        let known: Vec<&str> = GRANDFATHERED_MAGENTA_ACCENTS.to_vec();
        let newly: Vec<&&str> = found.iter().filter(|id| !known.contains(*id)).collect();
        assert!(
            newly.is_empty(),
            "new accents in the magenta band: {newly:?}"
        );
        let gone: Vec<&&str> = known.iter().filter(|id| !found.contains(id)).collect();
        assert!(
            gone.is_empty(),
            "these left the magenta band and must leave the grandfather list: {gone:?}"
        );
    }
}
