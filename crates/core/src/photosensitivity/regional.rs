//! Regional screening of a periodic SDR image sequence.
//!
//! The WCAG reference is a 341 by 256 pixel field at 1024 by 768. This
//! screening geometry scales each side to one third of the supplied raster,
//! rounded to the nearest whole pixel. Windows overlap at quarter-window
//! strides, with the right and bottom boundaries included. Display size,
//! distance, and magnification can change the actual visual angle.
//!
//! Each window uses its mean luminance and the existing Numinous red proxy
//! (encoded-channel saturation, threshold crossings, linear-light chromaticity).
//! A patch covering a fraction
//! `a` of a steady window with a luminance excursion `d` moves the mean by
//! `a*d`. A small, bright patch can cross the 0.10 threshold, while a patch
//! over a quarter of the window with a modest excursion can fall below it.
//! Opposite excursions can cancel. The returned rates describe these sampled
//! windows, not the area of concurrently flashing pixels.
//!
//! References: [WCAG 2.2](https://www.w3.org/TR/WCAG22/#dfn-general-flash-and-red-flash-thresholds)
//! and [the viewing geometry](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html).

use super::{
    DARK_CEILING, GENERAL_FLASH_DELTA, RedState, count_flashes, count_red_flashes,
    linear_channel_table, red_state_from_means,
};
use crate::surface::MAX_DIM;

/// Rectangle in the supplied raster, with the origin at the upper left.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlashWindow {
    /// Horizontal position of the first included pixel.
    pub x: usize,
    /// Vertical position of the first included pixel.
    pub y: usize,
    /// Number of pixel columns.
    pub width: usize,
    /// Number of pixel rows.
    pub height: usize,
}

/// Highest count in one sampled second, and a window that attained it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowPeak {
    /// First such window in top-to-bottom, left-to-right order.
    pub window: FlashWindow,
    /// Number of flashes in a full one-second sample bin.
    pub rate: usize,
}

/// Independent peaks from all windows and all origins on the phase grid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionalFlashes {
    /// Peak general-flash count.
    pub general: WindowPeak,
    /// Peak saturated-red-flash count.
    pub red: WindowPeak,
    /// A window has five consecutive one-second bins of two to three general flashes.
    pub extended_general: bool,
    /// A window has five consecutive one-second bins of two to three red flashes.
    pub extended_red: bool,
}

#[derive(Default)]
struct WindowSeries {
    luminance: Vec<f64>,
    red: Vec<RedState>,
}

struct Sample {
    rect: FlashWindow,
    /// Top left, top right, bottom left and bottom right prefix indices.
    corners: [usize; 4],
    series: WindowSeries,
}

/// Pixel cuts needed to obtain all interval sums along one axis.
struct Axis {
    cuts: Vec<usize>,
    windows: Vec<(usize, usize)>,
}

impl Axis {
    fn new(length: usize) -> Self {
        let size = ((length + 1) / 3).max(1);
        let stride = (size / 4).max(1);
        let last = length - size;
        let origins = (0..last).step_by(stride).chain(std::iter::once(last));
        let mut cuts: Vec<_> = origins.clone().flat_map(|i| [i, i + size]).collect();
        cuts.sort_unstable();
        cuts.dedup();
        let windows = origins
            .map(|start| {
                (
                    cuts.binary_search(&start).expect("window begins on a cut"),
                    cuts.binary_search(&(start + size))
                        .expect("window ends on a cut"),
                )
            })
            .collect();
        Self { cuts, windows }
    }
}

// Fixed-point accumulation gives identical sums for identical color multisets,
// including black regions beside very bright ones. Per-channel roundoff is at
// most 2^-33 in linear light. All sums fit u64 at the raster dimension cap.
const LINEAR_SCALE: f64 = (1u64 << 32) as f64;

fn linear_counts() -> &'static [u64; 256] {
    static COUNTS: std::sync::OnceLock<[u64; 256]> = std::sync::OnceLock::new();
    COUNTS.get_or_init(|| linear_channel_table().map(|s| (s * LINEAR_SCALE).round() as u64))
}

/// Streaming spatial reduction of one base cycle. Stores window readings for
/// each phase, so additional playback speeds reuse the original capture.
pub struct RegionalFlashCapture {
    width: usize,
    height: usize,
    x: Axis,
    y: Axis,
    /// Encoded R,G,B and linear R,G,B sums, at window boundary intersections.
    sums: Vec<[u64; 6]>,
    samples: Vec<Sample>,
}

impl RegionalFlashCapture {
    /// Prepare the reference window grid. Each dimension must fit a nonempty
    /// Numinous raster.
    pub fn new(width: usize, height: usize) -> Option<Self> {
        if !(1..=MAX_DIM).contains(&width) || !(1..=MAX_DIM).contains(&height) {
            return None;
        }
        let x = Axis::new(width);
        let y = Axis::new(height);
        let pitch = x.cuts.len();
        let mut samples = Vec::new();
        for &(top, bottom) in &y.windows {
            for &(left, right) in &x.windows {
                let rect = FlashWindow {
                    x: x.cuts[left],
                    y: y.cuts[top],
                    width: x.cuts[right] - x.cuts[left],
                    height: y.cuts[bottom] - y.cuts[top],
                };
                samples.push(Sample {
                    rect,
                    corners: [
                        top * pitch + left,
                        top * pitch + right,
                        bottom * pitch + left,
                        bottom * pitch + right,
                    ],
                    series: WindowSeries::default(),
                });
            }
        }
        let sums = vec![[0; 6]; pitch * y.cuts.len()];
        Some(Self {
            width,
            height,
            x,
            y,
            sums,
            samples,
        })
    }

    /// Append one tightly packed RGBA frame, using RGB only. Returns false on
    /// a length mismatch and leaves the captured sequence unchanged.
    pub fn push(&mut self, rgba: &[u8]) -> bool {
        if rgba.len() != self.width * self.height * 4 {
            return false;
        }
        let pixels = rgba.as_chunks::<4>().0;
        let table = linear_counts();
        let pitch = self.x.cuts.len();
        self.sums.fill([0; 6]);
        // Accumulate disjoint tiles first. Each pixel is read exactly once.
        // Only the corners needed by the windows need a summed-area entry.
        for (row, bounds) in self.y.cuts.windows(2).enumerate() {
            let start = (row + 1) * pitch + 1;
            let tiles = &mut self.sums[start..start + pitch - 1];
            for y in bounds[0]..bounds[1] {
                let scan = &pixels[y * self.width..(y + 1) * self.width];
                for (xs, total) in self.x.cuts.windows(2).zip(tiles.iter_mut()) {
                    let mut run = [0u64; 6];
                    for &[r, g, b, _] in &scan[xs[0]..xs[1]] {
                        run[0] += u64::from(r);
                        run[1] += u64::from(g);
                        run[2] += u64::from(b);
                        run[3] += table[usize::from(r)];
                        run[4] += table[usize::from(g)];
                        run[5] += table[usize::from(b)];
                    }
                    for (cell, add) in total.iter_mut().zip(run) {
                        *cell += add;
                    }
                }
            }
            let mut row_sum = [0; 6];
            for i in start..start + pitch - 1 {
                for (k, sum) in row_sum.iter_mut().enumerate() {
                    *sum += self.sums[i][k];
                    self.sums[i][k] = *sum + self.sums[i - pitch][k];
                }
            }
        }
        for sample in &mut self.samples {
            let [tl, tr, bl, br] = sample.corners;
            let channel = |k: usize| {
                ((self.sums[br][k] - self.sums[bl][k]) - (self.sums[tr][k] - self.sums[tl][k]))
                    as f64
            };
            let area = (sample.rect.width * sample.rect.height) as f64;
            let encoded = [0, 1, 2].map(|k| channel(k) / area);
            let linear = [3, 4, 5].map(|k| channel(k) / (area * LINEAR_SCALE));
            let luminance = 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
            sample.series.luminance.push(luminance);
            sample
                .series
                .red
                .push(red_state_from_means(encoded, linear));
        }
        true
    }

    /// All spatial windows, with their general and red readings in input order.
    pub fn windows(&self) -> impl Iterator<Item = (FlashWindow, &[f64], &[RedState])> {
        self.samples.iter().map(|s| {
            (
                s.rect,
                s.series.luminance.as_slice(),
                s.series.red.as_slice(),
            )
        })
    }

    /// Replay the base cycle advancing by `step` frames per sample. Every
    /// starting phase gets a full `fps`-sample window, including across wraps.
    /// A zero step holds the sampled image. Empty captures and rates outside
    /// 2 through 1000 samples per second have no reading.
    ///
    /// Extended flags report five adjacent, non-overlapping one-second sample
    /// bins of two or three flashes at a single location, from any sampled
    /// origin of the repeating sequence. They are descriptive cadence flags,
    /// not a duration threshold in WCAG.
    pub fn at_step(&self, step: usize, fps: usize) -> Option<RegionalFlashes> {
        let first = self.samples.first()?;
        let len = first.series.luminance.len();
        if len == 0 || !(2..=1000).contains(&fps) {
            return None;
        }
        let zero = WindowPeak {
            window: first.rect,
            rate: 0,
        };
        let mut report = RegionalFlashes {
            general: zero,
            red: zero,
            extended_general: false,
            extended_red: false,
        };
        let step = step % len;
        if step == 0 {
            return Some(report);
        }
        let mut offset_count = len;
        let mut divisor = step;
        while divisor != 0 {
            (offset_count, divisor) = (divisor, offset_count % divisor);
        }
        let period = len / offset_count;
        for (rect, light, red) in self.windows() {
            let (low, high) = light.iter().fold((1.0_f64, 0.0_f64), |(low, high), &v| {
                (low.min(v), high.max(v))
            });
            if low < DARK_CEILING && high - low >= GENERAL_FLASH_DELTA {
                let (peak, extended) =
                    orbit_counts(light, offset_count, period, step, fps, count_flashes);
                report.extended_general |= extended;
                if peak > report.general.rate {
                    report.general = WindowPeak {
                        window: rect,
                        rate: peak,
                    };
                }
            }
            if red.iter().any(RedState::is_saturated_red)
                && red.iter().any(|s| !s.is_saturated_red())
            {
                let (peak, extended) =
                    orbit_counts(red, offset_count, period, step, fps, count_red_flashes);
                report.extended_red |= extended;
                if peak > report.red.rate {
                    report.red = WindowPeak {
                        window: rect,
                        rate: peak,
                    };
                }
            }
        }
        Some(report)
    }
}

fn orbit_counts<T: Copy>(
    states: &[T],
    offsets: usize,
    period: usize,
    step: usize,
    fps: usize,
    count: impl Fn(&[T]) -> usize,
) -> (usize, bool) {
    let mut peak = 0;
    let mut extended = false;
    let mut orbit = Vec::with_capacity(period + fps - 1);
    let mut counts = Vec::with_capacity(period);
    for offset in 0..offsets {
        orbit.clear();
        let mut index = offset;
        for _ in 0..period + fps - 1 {
            orbit.push(states[index]);
            index = (index + step) % states.len();
        }
        counts.clear();
        counts.extend(orbit.windows(fps).map(&count));
        peak = peak.max(counts.iter().copied().max().unwrap_or(0));
        extended |= (0..period).any(|start| {
            (0..5).all(|second| (2..=3).contains(&counts[(start + fps * second) % period]))
        });
    }
    (peak, extended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photosensitivity::{
        frame_luminance, frame_red_state, peak_flashes_per_second, peak_red_flashes_per_second,
    };

    fn frame(width: usize, height: usize, color: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            for x in 0..width {
                bytes.extend(color(x, y));
                bytes.push(255);
            }
        }
        bytes
    }

    #[test]
    fn regional_geometry_includes_boundaries_and_rejects_invalid_frames() {
        assert!(RegionalFlashCapture::new(0, 140).is_none());
        assert!(RegionalFlashCapture::new(240, usize::MAX).is_none());
        let mut grid = RegionalFlashCapture::new(240, 140).unwrap();
        assert_eq!(grid.windows().count(), 90);
        assert_eq!(
            grid.windows().last().unwrap().0,
            FlashWindow {
                x: 160,
                y: 93,
                width: 80,
                height: 47
            }
        );
        assert!(grid.at_step(1, 60).is_none());
        assert!(!grid.push(&[0; 4]));
        let rgba = frame(240, 140, |x, y| [(x + y) as u8, x as u8, y as u8]);
        assert!(grid.push(&rgba));
        assert!(!grid.push(&rgba[..rgba.len() - 1]));
        assert!(grid.at_step(1, 0).is_none());
        assert_eq!(grid.at_step(0, 60).unwrap().general.rate, 0);
        for (window, light, red) in grid.windows() {
            let mut crop = Vec::new();
            for y in window.y..window.y + window.height {
                let start = 4 * (y * 240 + window.x);
                crop.extend_from_slice(&rgba[start..start + 4 * window.width]);
            }
            assert_eq!(light.len(), 1);
            assert!((light[0] - frame_luminance(&crop)).abs() < 2e-10);
            let reference = frame_red_state(&crop);
            assert!((red[0].saturation - reference.saturation).abs() < 1e-12);
            assert!(red[0].chromaticity_distance(&reference) < 1e-8);
        }
        // The WCAG reference rectangle is reproduced at the cited resolution.
        let reference = RegionalFlashCapture::new(1024, 768)
            .unwrap()
            .windows()
            .next()
            .unwrap()
            .0;
        assert_eq!((reference.width, reference.height), (341, 256));
    }

    #[test]
    fn regional_thirty_percent_patch_strobes_at_ten_hz_and_dilutes_in_whole_frame() {
        let mut grid = RegionalFlashCapture::new(60, 30).unwrap();
        let mut whole = Vec::new();
        for t in 0..60 {
            // Six of twenty columns, across all ten window rows.
            let rgba = frame(60, 30, |x, y| {
                if x < 6 && y < 10 && t % 6 < 3 {
                    [255; 3]
                } else {
                    [0; 3]
                }
            });
            whole.push(frame_luminance(&rgba));
            assert!(grid.push(&rgba));
        }
        assert_eq!(peak_flashes_per_second(&whole, 60.0), 0.0);
        let report = grid.at_step(1, 60).unwrap();
        assert_eq!((report.general.rate, report.red.rate), (10, 0));
        assert_eq!((report.general.window.x, report.general.window.y), (0, 0));
        assert!(!report.extended_general);
    }

    #[test]
    fn regional_one_pixel_moving_line_does_not_flash() {
        let mut grid = RegionalFlashCapture::new(60, 30).unwrap();
        for x in 0..60 {
            let rgba = frame(60, 30, |col, _| if col == x { [255; 3] } else { [0; 3] });
            assert!(grid.push(&rgba));
        }
        for step in [1, 2, 4, 8] {
            let report = grid.at_step(step, 60).unwrap();
            assert_eq!((report.general.rate, report.red.rate), (0, 0));
        }
    }

    #[test]
    fn regional_color_alternation_remains_visible_in_a_steady_surround() {
        let mut grid = RegionalFlashCapture::new(60, 30).unwrap();
        let mut whole = Vec::new();
        for t in 0..60 {
            // The two patch colors differ by less than 0.10 in luminance.
            let rgba = frame(60, 30, |x, y| {
                if x < 20 && y < 10 {
                    if t % 6 < 3 { [130, 0, 0] } else { [0, 0, 190] }
                } else {
                    [90; 3]
                }
            });
            whole.push(frame_red_state(&rgba));
            assert!(grid.push(&rgba));
        }
        let report = grid.at_step(1, 60).unwrap();
        assert_eq!(report.general.rate, 0);
        assert_eq!(report.red.rate, 10);
        assert_eq!((report.red.window.x, report.red.window.y), (0, 0));
        assert_eq!(peak_red_flashes_per_second(&whole, 60.0), 0.0);
    }

    #[test]
    fn regional_cadence_uses_consecutive_seconds_at_the_same_window() {
        for (active, expected) in [(360, true), (120, false)] {
            let mut grid = RegionalFlashCapture::new(1, 1).unwrap();
            for t in 0..360 {
                let color = if t < active && t % 24 < 12 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 0, 255]
                };
                assert!(grid.push(&color));
            }
            let report = grid.at_step(1, 60).unwrap();
            assert!((2..=3).contains(&report.general.rate));
            assert_eq!(
                (report.extended_general, report.extended_red),
                (expected, expected)
            );
        }
    }

    #[test]
    fn regional_rates_match_direct_counts_over_every_starting_phase() {
        let mut grid = RegionalFlashCapture::new(1, 1).unwrap();
        let colors = [
            [0, 0, 0, 255],
            [210, 10, 0, 255],
            [200, 160, 40, 255],
            [0, 0, 80, 255],
            [190, 0, 190, 255],
        ];
        for i in [0, 1, 3, 4, 0, 2, 1, 1, 3, 4, 2, 1, 4, 0, 0, 2] {
            assert!(grid.push(&colors[i]));
        }
        let (_, light, red) = grid.windows().next().unwrap();
        let n = light.len();
        for step in [0, 1, 2, 3, 4, 8, 17, usize::MAX] {
            let fps = 12;
            let report = grid.at_step(step, fps).unwrap();
            let mut general = 0;
            let mut red_peak = 0;
            for start in 0..n {
                let indices = (0..fps).map(|i| (start + i * (step % n)) % n);
                general = general.max(count_flashes(
                    &indices.clone().map(|i| light[i]).collect::<Vec<_>>(),
                ));
                red_peak = red_peak.max(count_red_flashes(
                    &indices.map(|i| red[i]).collect::<Vec<_>>(),
                ));
            }
            assert_eq!((report.general.rate, report.red.rate), (general, red_peak));
        }
    }
}
