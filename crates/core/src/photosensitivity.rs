//! Photosensitivity budget: how hard a surface is allowed to flash.
//!
//! The standard is WCAG 2.3.1, Three Flashes or Below Threshold. Its terms are
//! specific and easy to approximate wrongly, so they are implemented here
//! rather than eyeballed:
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
//!
//! 2.3.1 bounds red flashing separately and more strictly, because sensitivity
//! to saturated red is higher, and that test is implemented too. It is a
//! different measurement rather than the same one on the red channel: see
//! [`frame_red_state`].
//!
//! One part of 2.3.1 is deliberately **not** implemented, and the evidence must
//! say so rather than implying full conformance: the flashing **area** rule,
//! which bounds how much of central vision may flash. This module measures
//! whole frames, which is the conservative direction for a full-screen flash (a
//! whole-frame flash is at least as large as a partial one) but which cannot
//! see a small patch strobing inside an otherwise steady picture. Both the
//! general and the red measurement share that limit.

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
            *slot = if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            };
        }
        table
    })
}

/// Mean relative luminance of one RGBA frame.
///
/// Whole-frame rather than per-region: this module does not implement the
/// flashing-area rule, and averaging the whole frame cannot understate a
/// full-screen flash, which is the case that matters most.
#[must_use]
pub fn frame_luminance(rgba: &[u8]) -> f64 {
    let pixels = rgba.len() / 4;
    if pixels == 0 {
        return 0.0;
    }
    let total: f64 = rgba
        .chunks_exact(4)
        .map(|pixel| relative_luminance(pixel[0], pixel[1], pixel[2]))
        .sum();
    total / pixels as f64
}

/// The peaks and valleys of a luminance series, in order.
///
/// Transitions are measured between adjacent extrema, not between adjacent
/// samples, so a gradual ramp is one transition rather than many small ones.
/// Flat runs are not turning points and are passed over.
fn turning_points(series: &[f64]) -> Vec<f64> {
    if series.len() < 2 {
        return series.to_vec();
    }
    let mut points = vec![series[0]];
    let mut direction: i8 = 0;
    for window in series.windows(2) {
        let (previous, next) = (window[0], window[1]);
        let step: i8 = if next > previous {
            1
        } else if next < previous {
            -1
        } else {
            continue;
        };
        if direction != 0 && step != direction {
            points.push(previous);
        }
        direction = step;
    }
    if let Some(&last) = series.last() {
        points.push(last);
    }
    points
}

/// How many transitions in this series are large enough and dark enough to
/// count toward the flash budget.
#[must_use]
pub fn qualifying_transitions(series: &[f64]) -> usize {
    turning_points(series)
        .windows(2)
        .filter(|pair| {
            let change = (pair[1] - pair[0]).abs();
            let darker = pair[0].min(pair[1]);
            change >= GENERAL_FLASH_DELTA && darker < DARK_CEILING
        })
        .count()
}

/// Flashes in a luminance series.
///
/// A flash is two opposing transitions, so a full bright-dark-bright cycle is
/// one flash rather than two. Counting transitions instead would double every
/// measurement and fail surfaces that are within the budget.
#[must_use]
pub fn count_flashes(series: &[f64]) -> usize {
    qualifying_transitions(series) / 2
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

/// The worst one-second window in a longer series.
///
/// The standard bounds flashing in *any* one-second window, so averaging over
/// a whole cycle would let a half-second strobe hide behind quiet neighbours.
/// A series shorter than a full window is measured whole rather than skipped.
#[must_use]
pub fn peak_flashes_per_second(series: &[f64], fps: f64) -> f64 {
    if series.len() < 2 || !fps.is_finite() || fps <= 0.0 {
        return 0.0;
    }
    let window = (fps.ceil() as usize).max(2);
    if series.len() <= window {
        return flashes_per_second(series, fps);
    }
    series
        .windows(window)
        .map(|slice| flashes_per_second(slice, fps))
        .fold(0.0f64, f64::max)
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
/// crossing between them to count as a red transition.
pub const RED_CHROMATICITY_DELTA: f64 = 0.20;

/// A frame's color state for the red-flash test.
///
/// Two numbers decide a red flash and they answer different questions, so both
/// travel together: how red the frame is, and where its color sits. Saturation
/// alone would call every move between two different reds a flash, and distance
/// alone would call a move between two greens one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RedState {
    /// `R / (R + G + B)` over the frame's mean displayed color.
    ///
    /// The standard reads this off the displayed values, so it is taken before
    /// linearization. Relative luminance is the quantity that gets linearized.
    /// This one is not, and linearizing it anyway would move the threshold.
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
/// Whole-frame, on the same basis and with the same limitation as
/// [`frame_luminance`]: without the flashing-area rule, a small patch strobing
/// saturated red inside a steady picture averages away here. What this does
/// measure is a full-frame red flash.
///
/// Note for anyone reading the evidence this produces: the standard puts no
/// luminance floor under the ratio, so a frame of faint red ink on black is a
/// saturated red state, exactly as a bright red fill is. That is the standard
/// as written, and adding a floor it does not have would be inventing a
/// threshold rather than implementing one.
#[must_use]
pub fn frame_red_state(rgba: &[u8]) -> RedState {
    let pixels = rgba.len() / 4;
    if pixels == 0 {
        return RedState {
            saturation: 0.0,
            u: 0.0,
            v: 0.0,
        };
    }
    // Two means, because the two quantities are read off different scales. The
    // ratio comes off the displayed values, which is where the standard takes
    // it. The chromaticity comes off linear light, the only scale on which
    // averaging a picture means anything: averaging encoded values and
    // linearizing afterwards names a color no part of the frame contains.
    let table = linear_channel_table();
    let mut encoded = [0.0f64; 3];
    let mut light = [0.0f64; 3];
    for pixel in rgba.chunks_exact(4) {
        for (index, channel) in pixel[..3].iter().enumerate() {
            encoded[index] += f64::from(*channel);
            light[index] += table[*channel as usize];
        }
    }
    let encoded = encoded.map(|total| total / pixels as f64);
    let linear = light.map(|total| total / pixels as f64);
    let sum = encoded[0] + encoded[1] + encoded[2];
    // A black frame has no color to be red and no chromaticity at all. Zero for
    // both is the honest answer rather than a division producing NaN, which
    // would silently disable every later comparison. Placing black at the
    // origin also makes any red-to-black crossing look maximally distant, which
    // errs toward counting a flash rather than missing one.
    let saturation = if sum > 0.0 { encoded[0] / sum } else { 0.0 };
    let x = 0.4124 * linear[0] + 0.3576 * linear[1] + 0.1805 * linear[2];
    let y = 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    let z = 0.0193 * linear[0] + 0.1192 * linear[1] + 0.9505 * linear[2];
    let denominator = x + 15.0 * y + 3.0 * z;
    let (u, v) = if denominator > 0.0 {
        (4.0 * x / denominator, 9.0 * y / denominator)
    } else {
        (0.0, 0.0)
    };
    RedState { saturation, u, v }
}

/// One representative state per run of same-redness frames, in order.
///
/// A run is a stretch of frames that are all saturated red or all not, so
/// consecutive runs always differ in redness and every boundary between them is
/// a candidate transition. The representative is the run's most extreme frame,
/// the reddest inside a red run and the least red outside one, so a gradual
/// slide into red is judged by where it arrives rather than by how small each
/// step was. Comparing adjacent frames instead would miss a ramp that crosses
/// the diagram over half a second, which is well inside flashing speed.
///
/// This is the red counterpart of `turning_points`, and it exists for the same
/// reason: to measure excursions rather than samples.
fn red_run_extremes(states: &[RedState]) -> Vec<RedState> {
    let mut runs: Vec<RedState> = Vec::new();
    let mut current: Option<(bool, RedState)> = None;
    for state in states {
        match current {
            Some((red, best)) if red == state.is_saturated_red() => {
                let further = if red {
                    state.saturation > best.saturation
                } else {
                    state.saturation < best.saturation
                };
                current = Some((red, if further { *state } else { best }));
            }
            Some((_, best)) => {
                runs.push(best);
                current = Some((state.is_saturated_red(), *state));
            }
            None => current = Some((state.is_saturated_red(), *state)),
        }
    }
    if let Some((_, best)) = current {
        runs.push(best);
    }
    runs
}

/// How many crossings into or out of saturated red are large enough to count.
///
/// The series is first reduced to one representative frame per run of
/// same-redness frames, so every boundary left is already a crossing and the
/// only question is whether its two sides are far enough apart on the diagram.
/// That second condition is what keeps a drift between two nearly identical
/// colors, one just over the ratio and one just under, from reading as a flash.
#[must_use]
pub fn qualifying_red_transitions(states: &[RedState]) -> usize {
    red_run_extremes(states)
        .windows(2)
        .filter(|pair| pair[0].chromaticity_distance(&pair[1]) > RED_CHROMATICITY_DELTA)
        .count()
}

/// Red flashes in a series of frame states.
///
/// A flash is a pair of opposing transitions, in and back out, exactly as for
/// general flashes.
#[must_use]
pub fn count_red_flashes(states: &[RedState]) -> usize {
    qualifying_red_transitions(states) / 2
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
    if states.len() < 2 || !fps.is_finite() || fps <= 0.0 {
        return 0.0;
    }
    let window = (fps.ceil() as usize).max(2);
    if states.len() <= window {
        return red_flashes_per_second(states, fps);
    }
    states
        .windows(window)
        .map(|slice| red_flashes_per_second(slice, fps))
        .fold(0.0f64, f64::max)
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
        // The standard's ratio has no luminance floor, so dim red is as
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
        let under = frame_red_state(&flat(79, 21, 0, 4));
        assert!(under.saturation < RED_SATURATION && !under.is_saturated_red());
        let over = frame_red_state(&flat(81, 19, 0, 4));
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

        // Two saturated reds are both red states, so there is no crossing.
        let dim_red = frame_red_state(&flat(40, 0, 0, 4));
        assert!(dim_red.is_saturated_red());
        assert_eq!(qualifying_red_transitions(&[red, dim_red]), 0);

        // A crossing that barely moves on the diagram is not a flash. These two
        // sit either side of the ratio and are almost the same color.
        let over = frame_red_state(&flat(81, 19, 0, 4));
        let under = frame_red_state(&flat(79, 21, 0, 4));
        assert!(over.is_saturated_red() != under.is_saturated_red());
        assert!(over.chromaticity_distance(&under) < RED_CHROMATICITY_DELTA);
        assert_eq!(qualifying_red_transitions(&[over, under]), 0);
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
    }

    #[test]
    fn bright_on_bright_is_excluded_by_the_dark_ceiling() {
        // Both images above the ceiling: a large change, but not a hazard the
        // standard counts. Dropping this condition is the common mistake.
        let bright = [DARK_CEILING + 0.05, 1.0, DARK_CEILING + 0.05];
        assert_eq!(qualifying_transitions(&bright), 0);
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
