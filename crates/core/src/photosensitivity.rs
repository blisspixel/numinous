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
//! Two parts of 2.3.1 are deliberately **not** implemented, and the evidence
//! must say so rather than implying full conformance: the red-flash test, which
//! is stricter because sensitivity to saturated red is higher, and the flashing
//! **area** rule, which bounds how much of central vision may flash. This
//! module measures the whole frame, which is the conservative direction for
//! area (a whole-frame flash is at least as large as a partial one) and no
//! statement at all about red.

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
fn linear_channel_table() -> &'static [f64; 256] {
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

#[cfg(test)]
mod tests {
    use super::{
        DARK_CEILING, GENERAL_FLASH_DELTA, MAX_FLASHES_PER_SECOND, count_flashes,
        flashes_per_second, frame_luminance, peak_flashes_per_second, qualifying_transitions,
        relative_luminance, within_budget,
    };

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
