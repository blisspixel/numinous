//! Photosensitivity budget: how hard a surface is allowed to flash.
//!
//! The standard is WCAG 2.3.1, Three Flashes or Below Threshold. Its terms are
//! the basis for the following measurements:
//!
//! - A **flash** is a *pair of opposing* transitions in relative luminance, an
//!   increase followed by a decrease or the reverse. A single transition is not
//!   a flash, so a long fade in one direction must never be counted as one.
//! - A transition counts only when the change is at least
//!   [`GENERAL_FLASH_DELTA`] of maximum relative luminance **and** the darker of
//!   the two images is below [`DARK_CEILING`]. That second condition is the one
//!   most often dropped, and dropping it inflates the count on bright scenes
//!   that are not a hazard.
//! - Relative luminance runs 0 for black to 1 for white and comes from
//!   sRGB with the usual linearization. It is not the Rec. 601 luma used by
//!   `crate::ansi::to_mono`, which answers a different question and must not be
//!   substituted here.
//! - The limit is [`MAX_FLASHES_PER_SECOND`] within any one-second window.
//!   Each general flash uses one qualifying rise and one qualifying fall,
//!   in either order. A leg is used at most once.
//!
//! 2.3.1 bounds red flashing separately and more strictly, because sensitivity
//! to saturated red is higher. The red screen measures linear RGB saturation
//! and CIE chromaticity distance. [`count_red_flashes`] describes how the
//! sampled color path is divided into opposing excursions.
//!
//! Whole-frame means can dilute a localized strobe. The [regional] screen
//! also measures overlapping windows at a declared raster geometry. Its
//! window means have different biases from measuring the actual area of
//! concurrent flashing pixels.

pub mod regional;

/// Minimum change in relative luminance for a transition to count, as a
/// fraction of maximum relative luminance.
pub const GENERAL_FLASH_DELTA: f64 = 0.10;

/// A transition counts only when the darker of its two images is below this
/// relative luminance. Bright-on-bright changes are excluded by the standard.
pub const DARK_CEILING: f64 = 0.80;

/// More than this many flashes inside any one-second window is a violation.
pub const MAX_FLASHES_PER_SECOND: f64 = 3.0;

/// Relative luminance of one sRGB color, 0 for black through 1 for white.
///
/// The WCAG definition: linearize each channel, then weight by how much the
/// eye gets from it.
#[must_use]
pub fn relative_luminance(red: u8, green: u8, blue: u8) -> f64 {
    let table = linear_channel_table();
    0.2126 * table[red as usize] + 0.7152 * table[green as usize] + 0.0722 * table[blue as usize]
}

/// The sRGB-to-linear curve for all 256 channel values.
///
/// Built once. A channel is a `u8`, so the whole domain fits in a table, and
/// the alternative is three `powf` calls per pixel: measuring a full catalog
/// sweep that way spends minutes inside the exponential rather than doing the
/// work. This is exact, not an approximation.
///
/// Shared with `crate::dichromacy` rather than copied into it. Two copies of
/// the same curve are two things that can drift, and a linearization that
/// differs between two accessibility measurements would make their numbers
/// quietly incomparable.
pub(crate) fn linear_channel_table() -> &'static [f64; 256] {
    static TABLE: std::sync::OnceLock<[f64; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0f64; 256];
        for (channel, slot) in table.iter_mut().enumerate() {
            let value = channel as f64 / 255.0;
            *slot = if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            };
        }
        table
    })
}

// Every nonzero channel in the sRGB table is at least 2^-12. A binary64
// value at that magnitude has a spacing of 2^-64, so this scale preserves
// every table entry exactly. Even a 64-bit slice contains fewer than 2^62
// RGBA pixels, leaving room in u128 for the sum of all three channels.
const LINEAR_SCALE: f64 = (1u128 << 64) as f64;

fn linear_counts() -> &'static [u128; 256] {
    static COUNTS: std::sync::OnceLock<[u128; 256]> = std::sync::OnceLock::new();
    COUNTS.get_or_init(|| linear_channel_table().map(|value| (value * LINEAR_SCALE) as u128))
}

/// Mean relative luminance of one RGBA frame.
///
/// This whole-frame reading can dilute a local strobe. The [regional] screen
/// measures smaller windows separately.
#[must_use]
pub fn frame_luminance(rgba: &[u8]) -> f64 {
    let pixels = rgba.len() / 4;
    if pixels == 0 {
        return 0.0;
    }
    let total: f64 = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| relative_luminance(pixel[0], pixel[1], pixel[2]))
        .sum();
    total / pixels as f64
}

/// Visit the direction of each qualifying leg between adjacent extrema.
/// Flat runs preserve the current leg, so a gradual change is measured once.
fn visit_general_transitions(series: &[f64], mut visit: impl FnMut(bool)) {
    let Some((&first, rest)) = series.split_first() else {
        return;
    };
    let mut record = |from: f64, to: f64| {
        if (to - from).abs() >= GENERAL_FLASH_DELTA && from.min(to) < DARK_CEILING {
            visit(to > from);
        }
    };
    let mut start = first;
    let mut end = first;
    for &next in rest {
        if (end > start && next < end) || (end < start && next > end) {
            record(start, end);
            start = end;
        }
        end = next;
    }
    record(start, end);
}

/// How many transitions in this series are large enough and dark enough to
/// count toward the flash budget.
#[must_use]
pub fn qualifying_transitions(series: &[f64]) -> usize {
    let mut legs = 0;
    visit_general_transitions(series, |_| legs += 1);
    legs
}

/// Flashes in a luminance series.
///
/// A qualifying rise can close a pending fall and vice versa. Several rises
/// separated by shallow dips still need a qualifying fall to complete a flash.
/// Closing each pair at the first opportunity leaves the longest possible
/// suffix for subsequent disjoint pairs.
#[must_use]
pub fn count_flashes(series: &[f64]) -> usize {
    let mut pending = None;
    let mut flashes = 0;
    visit_general_transitions(series, |rise| {
        if pending == Some(!rise) {
            flashes += 1;
            pending = None;
        } else {
            pending = Some(rise);
        }
    });
    flashes
}

/// Flashes per second for a series sampled at `fps`.
///
/// Returns 0 when there is nothing to measure, so an empty capture reads as
/// "no evidence of flashing" rather than as a divide by zero. A caller that
/// needs to distinguish "safe" from "not measured" must check the sample count
/// itself.
#[must_use]
pub fn flashes_per_second(series: &[f64], fps: f64) -> f64 {
    if series.len() < 2 || !fps.is_finite() || fps <= 0.0 {
        return 0.0;
    }
    let seconds = series.len() as f64 / fps;
    if seconds <= 0.0 {
        return 0.0;
    }
    count_flashes(series) as f64 / seconds
}

/// The most complete flashes observed in any sampled one-second window.
///
/// The standard bounds flashing in *any* one-second window, so averaging over
/// a whole cycle would let a half-second strobe hide behind quiet neighbours.
/// At 60 fps, 61 sample instants span one second. A shorter capture contributes
/// the complete flashes observed during that capture.
#[must_use]
pub fn peak_flashes_per_second(series: &[f64], fps: f64) -> f64 {
    peak_count_per_second(series, fps, count_flashes)
}

fn peak_count_per_second<T>(series: &[T], fps: f64, count: impl Fn(&[T]) -> usize) -> f64 {
    if series.len() < 3 || !fps.is_finite() || fps <= 0.0 {
        return 0.0;
    }
    let samples = (fps.floor() as usize).saturating_add(1).min(series.len());
    series.windows(samples).map(count).max().unwrap_or(0) as f64
}

/// Whether a series sampled at `fps` stays inside the budget, judged on its
/// worst one-second window rather than its average.
#[must_use]
pub fn within_budget(series: &[f64], fps: f64) -> bool {
    peak_flashes_per_second(series, fps) <= MAX_FLASHES_PER_SECOND
}

/// A state at or above this `R / (R + G + B)` is a saturated red.
pub const RED_SATURATION: f64 = 0.80;

/// Two states must differ by more than this in CIE 1976 UCS chromaticity for a
/// color excursion to count toward a red flash.
pub const RED_CHROMATICITY_DELTA: f64 = 0.20;

/// A frame's color state for the red-flash test.
///
/// Two numbers decide a red flash and they answer different questions, so both
/// travel together: how red the frame is, and where its color sits. Saturation
/// alone would call every move between two different reds a flash, and distance
/// alone would call a move between two greens one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RedState {
    /// `R / (R + G + B)` over mean linear-light sRGB channels.
    pub saturation: f64,
    /// CIE 1976 UCS `u'` of the frame's mean color.
    pub u: f64,
    /// CIE 1976 UCS `v'` of the frame's mean color.
    pub v: f64,
}

impl RedState {
    /// Whether this frame counts as a saturated red state.
    #[must_use]
    pub fn is_saturated_red(&self) -> bool {
        self.saturation >= RED_SATURATION
    }

    /// Distance to another state in the CIE 1976 UCS chromaticity diagram.
    ///
    /// That diagram rather than raw RGB distance because it is roughly uniform:
    /// a step of 0.2 means about the same amount of visible color change
    /// wherever on it the step is taken, which is what lets one threshold apply
    /// everywhere.
    #[must_use]
    pub fn chromaticity_distance(&self, other: &Self) -> f64 {
        (self.u - other.u).hypot(self.v - other.v)
    }
}

/// The red-flash state of one RGBA frame.
///
/// As with [`frame_luminance`], local color excursions can average away.
/// The [regional] measurement uses the same red-state calculation per window.
///
/// This proxy has no luminance floor: dim red ink on black can yield a
/// saturated state. See [`RedState::saturation`] for its channel convention.
#[must_use]
pub fn frame_red_state(rgba: &[u8]) -> RedState {
    let table = linear_counts();
    let mut light = [0u128; 3];
    for pixel in rgba.as_chunks::<4>().0 {
        for (total, &channel) in light.iter_mut().zip(pixel) {
            *total += table[usize::from(channel)];
        }
    }
    red_state_from_totals(light)
}

fn red_state_from_totals(totals: [u128; 3]) -> RedState {
    // Chromaticity depends only on the proportions. Dimming a field or
    // repeating its pixels must give identical coordinates: a rounded
    // displacement on a constant-hue plateau can reverse the next chord.
    let [red, green, blue] = totals;
    let divisor = crate::numerics::gcd(crate::numerics::gcd(red, green), blue);
    if divisor == 0 {
        // Place black at the origin to screen returns between red and black.
        return RedState {
            saturation: 0.0,
            u: 0.0,
            v: 0.0,
        };
    }
    let [red, green, blue] = totals.map(|channel| channel / divisor);
    let other = green + blue;
    let sum = red + other;
    // R/(R+G+B) >= 4/5 iff R >= 4(G+B). Compare integer totals before
    // rounding the public ratio, including values within one ulp of 0.8.
    let ratio = red as f64 / sum as f64;
    let saturation = if red / 4 >= other {
        ratio.max(RED_SATURATION)
    } else {
        ratio.min(RED_SATURATION.next_down())
    };
    let [r, g, b] = [red, green, blue].map(|channel| channel as f64);
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = 0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b;
    let denominator = x + 15.0 * y + 3.0 * z;
    RedState {
        saturation,
        u: 4.0 * x / denominator,
        v: 9.0 * y / denominator,
    }
}

#[derive(Default)]
struct RedCounts {
    transitions: usize,
    flashes: usize,
}

fn displacement(from: RedState, to: RedState) -> [f64; 2] {
    [to.u - from.u, to.v - from.v]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

// Each leg ends when the next step turns back along the accumulated chord.
// Compare the chords of the qualifying legs again: skipping a small leg
// must never turn two displacements in the same direction into a flash.
fn red_counts_in_order(mut states: impl Iterator<Item = RedState>) -> RedCounts {
    let mut counts = RedCounts::default();
    let Some(mut start) = states.next() else {
        return counts;
    };
    let mut tip = start;
    let mut pending: Option<([f64; 2], bool)> = None;
    let mut finish_leg = |from: RedState, to: RedState| {
        let chord = displacement(from, to);
        let distance_squared = dot(chord, chord);
        if !distance_squared.is_finite()
            || distance_squared <= RED_CHROMATICITY_DELTA * RED_CHROMATICITY_DELTA
        {
            return;
        }
        let red = from.is_saturated_red() || to.is_saturated_red();
        counts.transitions += usize::from(red);
        if let Some((previous, previous_red)) = pending
            && (red || previous_red)
            && dot(previous, chord) < 0.0
        {
            counts.flashes += 1;
            pending = None;
        } else {
            pending = Some((chord, red));
        }
    };
    for next in states {
        if dot(displacement(start, tip), displacement(tip, next)) < 0.0 {
            finish_leg(start, tip);
            start = tip;
        }
        tip = next;
    }
    finish_leg(start, tip);
    counts
}

fn red_counts(states: &[RedState]) -> RedCounts {
    if !states.iter().any(RedState::is_saturated_red) {
        return RedCounts::default();
    }
    // A bent path can have different chord endpoints from either end. Each
    // orientation must witness its own complete pairs; their counts never add.
    let forward = red_counts_in_order(states.iter().copied());
    let backward = red_counts_in_order(states.iter().rev().copied());
    RedCounts {
        transitions: forward.transitions.max(backward.transitions),
        flashes: forward.flashes.max(backward.flashes),
    }
}

/// Count individual above-threshold color excursions with a saturated endpoint.
///
/// A flash can include a return leg whose own endpoints are both unsaturated.
/// Use [`count_red_flashes`] to measure complete pairs.
#[must_use]
pub fn qualifying_red_transitions(states: &[RedState]) -> usize {
    red_counts(states).transitions
}

/// Count disjoint pairs of opposing color excursions in a sequence of frame means.
///
/// [WCAG 2.2](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html)
/// defines the saturation and CIE 1976 UCS distance tests. This
/// screen locates a turn in the two-dimensional color path when the next
/// sample moves back along the chord from the previous turn (negative dot
/// product). Each paired chord must exceed 0.20 in length, the two chords
/// must point back against each other, and at least one of their endpoints
/// must be saturated. Short legs cannot supply either half of a pair.
/// Bent color paths are screened from both ends and the larger count is used.
/// This turning convention is an operational choice for sampled colors.
#[must_use]
pub fn count_red_flashes(states: &[RedState]) -> usize {
    red_counts(states).flashes
}

/// Red flashes per second for a series sampled at `fps`.
///
/// Empty and unmeasurable inputs read as zero, on the same terms as
/// [`flashes_per_second`].
#[must_use]
pub fn red_flashes_per_second(states: &[RedState], fps: f64) -> f64 {
    if states.len() < 2 || !fps.is_finite() || fps <= 0.0 {
        return 0.0;
    }
    let seconds = states.len() as f64 / fps;
    if seconds <= 0.0 {
        return 0.0;
    }
    count_red_flashes(states) as f64 / seconds
}

/// The worst one-second window of red flashing in a longer series.
#[must_use]
pub fn peak_red_flashes_per_second(states: &[RedState], fps: f64) -> f64 {
    peak_count_per_second(states, fps, count_red_flashes)
}

/// Whether a series stays inside the red-flash budget.
///
/// The standard allows the same count for red flashes as for general ones; red
/// is stricter in what *counts* as a flash, not in how many are permitted.
#[must_use]
pub fn within_red_budget(states: &[RedState], fps: f64) -> bool {
    peak_red_flashes_per_second(states, fps) <= MAX_FLASHES_PER_SECOND
}

#[cfg(test)]
mod tests {
    use super::{
        DARK_CEILING, GENERAL_FLASH_DELTA, MAX_FLASHES_PER_SECOND, RED_CHROMATICITY_DELTA,
        RED_SATURATION, RedState, count_flashes, count_red_flashes, flashes_per_second,
        frame_luminance, frame_red_state, peak_flashes_per_second, peak_red_flashes_per_second,
        qualifying_red_transitions, qualifying_transitions, relative_luminance, within_budget,
        within_red_budget,
    };

    /// A frame of one flat color, as RGBA.
    fn flat(red: u8, green: u8, blue: u8, pixels: usize) -> Vec<u8> {
        [red, green, blue, 255].repeat(pixels)
    }

    #[test]
    fn chromaticity_matches_the_published_srgb_primaries() {
        // The whole red test rests on this transform, so it is checked against
        // numbers from outside this codebase rather than against itself. The
        // sRGB primaries in CIE 1976 UCS are red u'=0.4507 v'=0.5229, green
        // u'=0.1250 v'=0.5625, blue u'=0.1755 v'=0.1579, and the D65 white
        // point is u'=0.1978 v'=0.4683.
        for (label, rgb, expected) in [
            ("red", (255u8, 0u8, 0u8), (0.4507, 0.5229)),
            ("green", (0, 255, 0), (0.1250, 0.5625)),
            ("blue", (0, 0, 255), (0.1755, 0.1579)),
            ("white", (255, 255, 255), (0.1978, 0.4683)),
        ] {
            let state = frame_red_state(&flat(rgb.0, rgb.1, rgb.2, 4));
            assert!(
                (state.u - expected.0).abs() < 5e-4 && (state.v - expected.1).abs() < 5e-4,
                "{label}: got u'={} v'={}, expected u'={} v'={}",
                state.u,
                state.v,
                expected.0,
                expected.1
            );
        }
    }

    #[test]
    fn saturated_red_is_the_ratio_and_not_the_brightness() {
        // The proxy's ratio has no luminance floor, so dim red is as
        // saturated as bright red. This is deliberate and is documented on
        // `frame_red_state`; pinning it here means a later "fix" that adds a
        // floor has to argue with a test rather than slip through.
        let bright = frame_red_state(&flat(255, 0, 0, 4));
        let faint = frame_red_state(&flat(12, 0, 0, 4));
        assert!((bright.saturation - 1.0).abs() < 1e-12);
        assert!((faint.saturation - 1.0).abs() < 1e-12);
        assert!(bright.is_saturated_red() && faint.is_saturated_red());

        // Just either side of the threshold, built from the ratio rather than
        // from a color chosen by eye.
        let under = frame_red_state(&flat(200, 107, 0, 4));
        assert!(under.saturation < RED_SATURATION && !under.is_saturated_red());
        let over = frame_red_state(&flat(200, 105, 0, 4));
        assert!(over.saturation > RED_SATURATION && over.is_saturated_red());

        // Black has no color to be red, and must not divide by zero.
        let black = frame_red_state(&flat(0, 0, 0, 4));
        assert!(black.saturation.abs() < 1e-12 && !black.is_saturated_red());
        assert!(black.u.abs() < 1e-12 && black.v.abs() < 1e-12);
        // Nothing to measure is not a red frame either.
        assert!(!frame_red_state(&[]).is_saturated_red());
    }

    #[test]
    fn a_red_transition_needs_both_the_ratio_and_the_distance() {
        let red = frame_red_state(&flat(255, 0, 0, 4));
        let white = frame_red_state(&flat(255, 255, 255, 4));
        let green = frame_red_state(&flat(0, 255, 0, 4));

        // Red to white crosses the ratio and clears the distance, so it counts.
        // It clears it by little: 0.259 against a threshold of 0.2, which is
        // why the distance is measured rather than assumed.
        let far = red.chromaticity_distance(&white);
        assert!(
            far > RED_CHROMATICITY_DELTA && far < 0.30,
            "red to white measured {far}"
        );
        assert_eq!(qualifying_red_transitions(&[red, white]), 1);

        // Green to blue crosses more of the diagram than red to white does, and
        // is not a red transition, because neither end is red. Distance alone
        // decides nothing.
        let blue = frame_red_state(&flat(0, 0, 255, 4));
        assert!(green.chromaticity_distance(&blue) > far);
        assert_eq!(qualifying_red_transitions(&[green, blue]), 0);
        // The return from blue to green opposes the earlier red-to-blue leg.
        // Only the first leg has a saturated endpoint; the pair involves it.
        assert_eq!(qualifying_red_transitions(&[red, blue, green]), 1);
        assert_eq!(count_red_flashes(&[red, blue, green]), 1);

        // Two intensities of the same red have the same chromaticity.
        let dim_red = frame_red_state(&flat(40, 0, 0, 4));
        assert!(dim_red.is_saturated_red());
        assert_eq!(qualifying_red_transitions(&[red, dim_red]), 0);

        // A crossing that barely moves on the diagram is not a flash. These two
        // sit either side of the ratio and are almost the same color.
        let over = frame_red_state(&flat(200, 105, 0, 4));
        let under = frame_red_state(&flat(200, 107, 0, 4));
        assert!(over.is_saturated_red() != under.is_saturated_red());
        assert!(over.chromaticity_distance(&under) < RED_CHROMATICITY_DELTA);
        assert_eq!(qualifying_red_transitions(&[over, under]), 0);
    }

    #[test]
    fn linear_saturation_catches_the_warm_red_and_blue_pair() {
        let warm = frame_red_state(&flat(200, 60, 0, 4));
        let blue = frame_red_state(&flat(0, 0, 255, 4));
        // Half as much light in each channel preserves both ratios.
        let mixed = frame_red_state(&[200, 0, 0, 255, 0, 60, 0, 255]);
        assert!((warm.saturation - mixed.saturation).abs() < 1e-12);
        assert!(warm.chromaticity_distance(&mixed) < 1e-12);
        // The encoded ratio is 200/260, below 0.8. These independent
        // references apply the W3C linearization and full XYZ coefficients.
        assert!((warm.saturation - 0.927_442_799).abs() < 1e-9);
        assert!((warm.chromaticity_distance(&blue) - 0.428_480_457).abs() < 1e-9);
        let light = relative_luminance(200, 60, 0) - relative_luminance(0, 0, 255);
        assert!((light - 0.082_910_775).abs() < 1e-9 && light < GENERAL_FLASH_DELTA);
        assert_eq!(count_red_flashes(&[blue, warm, blue]), 1);
        assert_eq!(count_red_flashes(&[warm, blue, warm]), 1);
        let strobe: Vec<_> = [warm, blue, warm, blue]
            .into_iter()
            .cycle()
            .take(61)
            .collect();
        assert_eq!(peak_red_flashes_per_second(&strobe, 60.0), 30.0);
    }

    #[test]
    fn constant_hue_plateaus_cannot_turn_a_color_chord() {
        let red = frame_red_state(&[255, 0, 0, 255]);
        let green = frame_red_state(&[0, 255, 0, 255]);
        let black = frame_red_state(&[0, 0, 0, 255]);
        for intensity in 1..=255 {
            let dim_green = frame_red_state(&[0, intensity, 0, 255]);
            assert_eq!(dim_green, green);
            assert_eq!(
                count_red_flashes(&[red, green, dim_green, green, black, green]),
                1
            );
        }
        let warm = super::red_state_from_totals([5, 3, 19]);
        for scale in [2, 7, (1 << 100) + 5] {
            assert_eq!(
                warm,
                super::red_state_from_totals([5 * scale, 3 * scale, 19 * scale])
            );
        }
    }

    #[test]
    fn red_saturation_is_inclusive_and_ucs_distance_is_strict() {
        let green = 1u128 << 80;
        for (red, saturated) in [
            (4 * green - 1, false),
            (4 * green, true),
            (4 * green + 1, true),
        ] {
            assert_eq!(
                super::red_state_from_totals([red, green, 0]).is_saturated_red(),
                saturated
            );
        }
        // Synthetic coordinates isolate the temporal rule from the sRGB gamut.
        let a = RedState {
            saturation: 0.8,
            u: 0.0,
            v: 0.0,
        };
        let boundary = RedState {
            u: RED_CHROMATICITY_DELTA,
            ..a
        };
        let far = RedState {
            u: RED_CHROMATICITY_DELTA + 0.001,
            ..a
        };
        assert_eq!(count_red_flashes(&[a, boundary, a]), 0);
        assert_eq!(count_red_flashes(&[a, far, a]), 1);
        let red_green = frame_red_state(&flat(255, 136, 0, 4));
        let red_blue = frame_red_state(&flat(255, 0, 136, 4));
        assert!(red_green.is_saturated_red() && red_blue.is_saturated_red());
        assert!(red_green.chromaticity_distance(&red_blue) < RED_CHROMATICITY_DELTA);
        assert_eq!(count_red_flashes(&[red_green, red_blue, red_green]), 0);
    }

    #[test]
    fn red_pairs_agree_with_an_independent_three_sample_oracle() {
        // Coordinates are in quarter units. Every nonzero displacement
        // clears 0.2. The law of cosines lets integer arithmetic decide
        // whether the two successive displacements point back at each other.
        let points = [
            (0_i32, 0_i32, true),
            (1, 0, false),
            (1, 1, false),
            (2, 1, true),
            (2, 2, false),
            (0, 2, false),
        ];
        let color = |(x, y, red)| RedState {
            u: f64::from(x) / 4.0,
            v: f64::from(y) / 4.0,
            saturation: if red { 1.0 } else { 0.0 },
        };
        let squared = |(x, y, _): (i32, i32, bool), (u, v, _): (i32, i32, bool)| {
            (x - u).pow(2) + (y - v).pow(2)
        };
        for a in points {
            for b in points {
                for c in points {
                    let ab = squared(a, b);
                    let bc = squared(b, c);
                    let flash = ab > 0 && bc > 0 && squared(a, c) < ab + bc && (a.2 || b.2 || c.2);
                    assert_eq!(
                        count_red_flashes(&[color(a), color(b), color(c)]),
                        usize::from(flash)
                    );
                }
            }
        }
        // The first and last states can bound different chords on a bent
        // color path. Reversing playback preserves the larger witnessed count.
        let bent = [points[0], points[1], points[2], points[5]].map(color);
        assert_eq!(count_red_flashes(&bent), 1);
        let mut backward = bent;
        backward.reverse();
        assert_eq!(count_red_flashes(&backward), 1);
    }

    #[test]
    fn two_qualifying_color_legs_in_the_same_direction_do_not_make_a_flash() {
        let states = [0.0, 0.3, 0.25, 0.55].map(|u| RedState {
            saturation: 1.0,
            u,
            v: 0.0,
        });
        assert_eq!(qualifying_red_transitions(&states), 2);
        assert_eq!(count_red_flashes(&states), 0);
    }

    #[test]
    fn a_red_flash_is_a_pair_of_transitions_not_a_single_one() {
        let red = frame_red_state(&flat(255, 0, 0, 4));
        let white = frame_red_state(&flat(255, 255, 255, 4));
        // One crossing is half a flash, and half a flash is none.
        assert_eq!(count_red_flashes(&[white, red]), 0);
        // Out and back is one.
        assert_eq!(count_red_flashes(&[white, red, white]), 1);
        assert_eq!(count_red_flashes(&[white, red, white, red, white]), 2);
    }

    #[test]
    fn a_slow_slide_into_red_is_still_measured_by_where_it_arrives() {
        // Frame to frame this ramp moves a tiny distance, so comparing adjacent
        // samples would find nothing. What matters is the excursion, and the
        // excursion crosses the diagram.
        let ramp: Vec<RedState> = (0..=10)
            .map(|step| {
                let other = u8::try_from(255 - step * 25).expect("in range");
                frame_red_state(&flat(255, other, other, 4))
            })
            .collect();
        let adjacent_moves: Vec<f64> = ramp
            .windows(2)
            .map(|pair| pair[0].chromaticity_distance(&pair[1]))
            .collect();
        assert!(
            adjacent_moves
                .iter()
                .all(|move_| *move_ < RED_CHROMATICITY_DELTA),
            "no single step should clear the threshold: {adjacent_moves:?}"
        );
        let mut there_and_back = ramp.clone();
        there_and_back.extend(ramp.iter().rev().skip(1).copied());
        assert_eq!(count_red_flashes(&there_and_back), 1);
        let mut out_of_red: Vec<_> = ramp.iter().rev().copied().collect();
        assert_eq!(count_red_flashes(&out_of_red), 0);
        out_of_red.extend(ramp.iter().skip(1).copied());
        assert_eq!(count_red_flashes(&out_of_red), 1);
        assert_eq!(count_red_flashes(&ramp), 0);
    }

    #[test]
    fn the_red_budget_is_judged_on_the_worst_second() {
        let red = frame_red_state(&flat(255, 0, 0, 4));
        let white = frame_red_state(&flat(255, 255, 255, 4));
        let fps = 10.0;

        // Three flashes inside one second is allowed; the standard bounds more
        // than three.
        let mut at_limit = Vec::new();
        for _ in 0..3 {
            at_limit.extend([white, white, red]);
        }
        at_limit.push(white);
        assert_eq!(count_red_flashes(&at_limit), 3);
        assert!(within_red_budget(&at_limit, fps));

        // A strobe in the first second must not be excused by a quiet second
        // after it.
        let mut strobe = Vec::new();
        for _ in 0..5 {
            strobe.extend([white, red]);
        }
        strobe.extend(std::iter::repeat_n(white, 10));
        assert!(peak_red_flashes_per_second(&strobe, fps) > MAX_FLASHES_PER_SECOND);
        assert!(!within_red_budget(&strobe, fps));
        // Averaged over both seconds it would have squeaked through, which is
        // the mistake the sliding window exists to prevent.
        assert!(count_red_flashes(&strobe) as f64 / 2.0 <= MAX_FLASHES_PER_SECOND);

        // A steady picture never flashes, whatever its color.
        assert!(within_red_budget(&[red; 30], fps));
        // Unmeasurable input reads as no evidence rather than as a pass earned.
        assert!(peak_red_flashes_per_second(&[], fps).abs() < 1e-12);
        assert!(peak_red_flashes_per_second(&[white, red], 0.0).abs() < 1e-12);
        assert!(peak_red_flashes_per_second(&[white, red], f64::NAN).abs() < 1e-12);
    }

    #[test]
    fn fixed_point_counts_preserve_every_linear_channel() {
        for (&channel, &count) in super::linear_channel_table()
            .iter()
            .zip(super::linear_counts())
        {
            assert_eq!(channel, count as f64 / super::LINEAR_SCALE);
        }
    }

    #[test]
    fn relative_luminance_anchors_at_black_and_white() {
        assert!(relative_luminance(0, 0, 0).abs() < 1e-12);
        assert!((relative_luminance(255, 255, 255) - 1.0).abs() < 1e-12);
        // Green carries most of the luminance, blue the least. This is the
        // property that separates the WCAG formula from a plain average.
        let green = relative_luminance(0, 255, 0);
        let red = relative_luminance(255, 0, 0);
        let blue = relative_luminance(0, 0, 255);
        assert!(green > red && red > blue, "{green} {red} {blue}");
    }

    #[test]
    fn frame_luminance_averages_the_whole_frame() {
        let black = [0u8, 0, 0, 255];
        let white = [255u8, 255, 255, 255];
        let mut half = Vec::new();
        half.extend_from_slice(&black);
        half.extend_from_slice(&white);
        let mean = frame_luminance(&half);
        assert!((mean - 0.5).abs() < 1e-9, "{mean}");
        assert_eq!(frame_luminance(&[]), 0.0);
    }

    #[test]
    fn a_steady_image_never_flashes() {
        let steady = vec![0.4; 60];
        assert_eq!(count_flashes(&steady), 0);
        assert!(within_budget(&steady, 30.0));
    }

    #[test]
    fn a_single_long_fade_is_not_a_flash() {
        // One transition, however large, is not a pair of opposing ones.
        let fade: Vec<f64> = (0..=60).map(|step| f64::from(step) / 60.0).collect();
        assert_eq!(qualifying_transitions(&fade), 1);
        assert_eq!(count_flashes(&fade), 0);
    }

    #[test]
    fn a_full_dark_bright_dark_cycle_is_one_flash() {
        let cycle = [0.0, 0.5, 0.0];
        assert_eq!(qualifying_transitions(&cycle), 2);
        assert_eq!(count_flashes(&cycle), 1);
    }

    #[test]
    fn only_opposing_qualified_legs_complete_disjoint_flashes() {
        for (samples, legs, flashes) in [
            (&[0.00, 0.15, 0.10, 0.25][..], 2, 0),
            (&[0.00, 0.15, 0.10, 0.25, 0.25, 0.00][..], 3, 1),
            (&[0.00, 0.15, 0.10, 0.25, 0.00, 0.40, 0.00][..], 5, 2),
            (&[0.00, 0.08, 0.16, 0.16, 0.08, 0.00, 0.30][..], 3, 1),
        ] {
            let reversed: Vec<_> = samples.iter().copied().rev().collect();
            let inverted: Vec<_> = samples.iter().map(|v| 0.7 - v).collect();
            for series in [samples, &reversed, &inverted] {
                assert_eq!(qualifying_transitions(series), legs, "{series:?}");
                assert_eq!(count_flashes(series), flashes, "{series:?}");
            }
        }
    }

    #[test]
    fn one_second_includes_both_endpoints_but_never_extrapolates_a_short_clip() {
        // Four complete flashes need eight transitions. The last transition
        // falls exactly one second after the start at eight samples a second.
        let alternating = [0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        assert_eq!(peak_flashes_per_second(&alternating, 8.0), 4.0);
        assert!(!within_budget(&alternating, 8.0));
        assert_eq!(peak_flashes_per_second(&alternating, 7.5), 3.0);
        assert_eq!(peak_flashes_per_second(&alternating, 8.5), 4.0);
        assert_eq!(peak_flashes_per_second(&alternating, 0.5), 0.0);
        assert_eq!(peak_flashes_per_second(&alternating[..3], 60.0), 1.0);
    }

    #[test]
    fn a_ten_hertz_strobe_is_caught() {
        // The case the budget exists to prevent: alternating black and white
        // every other frame at 30fps is 15 flashes per second.
        let strobe: Vec<f64> = (0..30)
            .map(|frame| if frame % 2 == 0 { 0.0 } else { 1.0 })
            .collect();
        let rate = flashes_per_second(&strobe, 30.0);
        assert!(rate > MAX_FLASHES_PER_SECOND, "{rate} should exceed budget");
        assert!(!within_budget(&strobe, 30.0));
    }

    #[test]
    fn a_slow_pulse_stays_inside_the_budget() {
        // Two full cycles per second is within three.
        let mut series = Vec::new();
        for _ in 0..2 {
            series.extend(std::iter::repeat_n(0.1, 7));
            series.extend(std::iter::repeat_n(0.5, 8));
        }
        let rate = flashes_per_second(&series, 30.0);
        assert!(rate <= MAX_FLASHES_PER_SECOND, "{rate}");
        assert!(within_budget(&series, 30.0));
    }

    #[test]
    fn a_change_below_the_delta_does_not_count() {
        let subtle = [0.40, 0.40 + GENERAL_FLASH_DELTA / 2.0, 0.40];
        assert_eq!(qualifying_transitions(&subtle), 0);
        assert_eq!(count_flashes(&subtle), 0);
        assert_eq!(count_flashes(&[0.0, GENERAL_FLASH_DELTA, 0.0]), 1);
    }

    #[test]
    fn bright_on_bright_is_excluded_by_the_dark_ceiling() {
        // Both images above the ceiling: a large change, but not a hazard the
        // standard counts. Dropping this condition is the common mistake.
        let bright = [DARK_CEILING + 0.05, 1.0, DARK_CEILING + 0.05];
        assert_eq!(qualifying_transitions(&bright), 0);
        assert_eq!(count_flashes(&[DARK_CEILING, 1.0, DARK_CEILING]), 0);
        // The same swing with a dark image in it does count.
        let dark = [0.0, 0.5, 0.0];
        assert_eq!(qualifying_transitions(&dark), 2);
    }

    #[test]
    fn a_short_strobe_cannot_hide_behind_quiet_neighbours() {
        // Three seconds of stillness with half a second of strobe inside it.
        // The average is comfortably under budget; the worst second is not,
        // and the standard bounds the worst second.
        let mut series = vec![0.2; 45];
        for frame in 0..15 {
            series.push(if frame % 2 == 0 { 0.0 } else { 1.0 });
        }
        series.extend(std::iter::repeat_n(0.2, 30));
        let average = flashes_per_second(&series, 30.0);
        let peak = peak_flashes_per_second(&series, 30.0);
        assert!(
            average <= MAX_FLASHES_PER_SECOND,
            "average {average} looks safe"
        );
        assert!(peak > MAX_FLASHES_PER_SECOND, "peak {peak} must catch it");
        assert!(!within_budget(&series, 30.0));
    }

    #[test]
    fn nothing_to_measure_is_not_a_divide_by_zero() {
        assert_eq!(flashes_per_second(&[], 30.0), 0.0);
        assert_eq!(flashes_per_second(&[0.5], 30.0), 0.0);
        assert_eq!(flashes_per_second(&[0.0, 1.0, 0.0], 0.0), 0.0);
        assert_eq!(flashes_per_second(&[0.0, 1.0, 0.0], f64::NAN), 0.0);
        assert_eq!(peak_flashes_per_second(&[], 30.0), 0.0);
        assert_eq!(peak_flashes_per_second(&[0.5], 30.0), 0.0);
        assert_eq!(peak_flashes_per_second(&[0.0, 1.0, 0.0], 0.0), 0.0);
    }
}
