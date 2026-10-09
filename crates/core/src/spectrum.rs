//! Spectrum bands for the music visualizer path (panel item 8).
//!
//! Pure offline math over interleaved PCM. OS loopback capture remains a later
//! platform concern; this module is the shared band-energy driver that rooms
//! and the App can feed from any f32 sample buffer (room beds, radio decode,
//! or a future loopback ring).

/// Number of fixed frequency bands reported by [`band_energies`].
pub const BAND_COUNT: usize = 7;

/// Human-readable band names in low-to-high order.
pub const BAND_NAMES: [&str; BAND_COUNT] =
    ["sub", "bass", "low-mid", "mid", "high-mid", "treble", "air"];

/// Approximate center frequencies (Hz) for the seven bands.
const BAND_CENTERS_HZ: [f32; BAND_COUNT] =
    [60.0, 150.0, 400.0, 1_000.0, 2_500.0, 6_000.0, 12_000.0];

/// Analyze interleaved f32 samples into seven non-negative band energies.
///
/// `channels` must be 1 or 2. Empty input, zero rate, or unsupported channel
/// counts return zeros. Energies are relative mean-square magnitudes at the
/// nearest DFT bin to each band center, not calibrated dB SPL. A non-finite
/// sample contributes nothing, and a non-finite energy is stored as zero.
#[must_use]
pub fn band_energies(samples: &[f32], channels: usize, sample_rate: u32) -> [f32; BAND_COUNT] {
    let mut out = [0.0f32; BAND_COUNT];
    if samples.is_empty() || sample_rate == 0 || !(channels == 1 || channels == 2) {
        return out;
    }
    let frames = samples.len() / channels;
    if frames < 16 {
        return out;
    }
    // Cap the analysis window so a full radio buffer stays cheap.
    let window = frames.min(2_048);
    let start = frames.saturating_sub(window);
    let rate = sample_rate as f32;
    let denom = window.saturating_sub(1).max(1) as f32;
    for (band, &center) in BAND_CENTERS_HZ.iter().enumerate() {
        if center >= rate * 0.48 {
            continue;
        }
        // Nearest positive DFT bin for this center frequency.
        let k = ((center * window as f32 / rate).round() as usize).clamp(1, window / 2);
        let omega = std::f32::consts::TAU * k as f32 / window as f32;
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        let (mut c, mut s) = (1.0f32, 0.0f32);
        let (dc, ds) = (omega.cos(), omega.sin());
        for i in 0..window {
            let frame = start + i;
            let mixed = if channels == 1 {
                samples[frame]
            } else {
                let o = frame * 2;
                0.5 * (samples[o] + samples[o + 1])
            };
            // A non-finite sample is silence. It must not poison the accumulator.
            let mono = if mixed.is_finite() { mixed } else { 0.0 };
            // Hann window softens spectral leakage without a full FFT table.
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / denom).cos();
            let x = mono * w;
            re += x * c;
            im += x * s;
            let nc = c * dc - s * ds;
            let ns = c * ds + s * dc;
            c = nc;
            s = ns;
        }
        let energy = (re * re + im * im) / (window as f32 * window as f32);
        out[band] = if energy.is_finite() {
            energy.max(0.0)
        } else {
            0.0
        };
    }
    out
}

/// Collapse seven bands into a coarse bass / mid / treble triple for lever maps.
///
/// A non-finite band contributes nothing, so the triple stays finite.
#[must_use]
pub fn bass_mid_treble(bands: &[f32; BAND_COUNT]) -> (f32, f32, f32) {
    let bass = add_finite(bands[0], bands[1]);
    let mid = add_finite(add_finite(bands[2], bands[3]), bands[4]);
    let treble = add_finite(bands[5], bands[6]);
    (bass, mid, treble)
}

fn add_finite(left: f32, right: f32) -> f32 {
    let sum = finite_band(left) + finite_band(right);
    finite_band(sum)
}

fn finite_band(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Normalize band energies so the loudest finite positive band is 1.0.
///
/// A non-finite or negative energy contributes nothing. All zeros, and a
/// vector with no finite positive energy, stay zero.
#[must_use]
pub fn normalize_bands(bands: &[f32; BAND_COUNT]) -> [f32; BAND_COUNT] {
    let mut finite = [0.0f32; BAND_COUNT];
    for (slot, &energy) in finite.iter_mut().zip(bands) {
        if energy.is_finite() && energy > 0.0 {
            *slot = energy;
        }
    }
    let peak = finite.iter().copied().fold(0.0f32, f32::max);
    if peak <= f32::EPSILON {
        return [0.0; BAND_COUNT];
    }
    let mut out = [0.0f32; BAND_COUNT];
    for (index, &energy) in finite.iter().enumerate() {
        out[index] = (energy / peak).clamp(0.0, 1.0);
    }
    out
}

/// Coarse onset proxy: low-band energy of this frame over the previous frame.
///
/// Values near 1.0 mean little change; above ~1.5 suggests a low-band attack.
/// Either side empty, silent, or non-finite returns 1.0 (no onset).
#[must_use]
pub fn low_band_onset(previous: &[f32; BAND_COUNT], current: &[f32; BAND_COUNT]) -> f32 {
    let prev = previous[0] + previous[1];
    let curr = current[0] + current[1];
    if !prev.is_finite() || !curr.is_finite() {
        return 1.0;
    }
    if prev <= f32::EPSILON {
        return if curr > f32::EPSILON { 2.0 } else { 1.0 };
    }
    (curr / prev).clamp(0.0, 8.0)
}

/// Normalized spectrum of a stereo arrangement render at the room-bed rate.
///
/// Offline path for room beds and CLI/MCP listen exports. Live App paths also
/// feed output-mix or loopback captures through the same band pipeline.
/// This is the visualizer window: only the last 2048 frames.
#[must_use]
pub fn arrangement_spectrum(samples: &[f32], sample_rate: u32) -> [f32; BAND_COUNT] {
    normalize_bands(&band_energies(samples, 2, sample_rate))
}

/// Whole-buffer spectral fingerprint on the same seven band centers.
///
/// [`band_energies`] measures only its last 2048 frames, which is the live
/// visualizer window. A room bed can change timbre before that tail, so this
/// averages the raw energies of every non-overlapping window and then
/// normalizes. A buffer of at most 2048 frames matches [`normalize_bands`] of
/// [`band_energies`] on that same buffer. A remainder shorter than 16 frames
/// is omitted, matching the analyzer's own minimum. Silence, fewer than 16
/// frames, a zero rate, or an unsupported channel count returns zeros.
///
/// The air band stays zero when its center is at or above 0.48 of the sample
/// rate. At the 16 kHz room-bed rate that center is above Nyquist, so the air
/// band of a room bed is zero. The result is gain-invariant: scaling a finite
/// non-silent buffer does not change the normalized bands. A non-finite sample
/// contributes nothing, and a non-finite window energy does not survive.
#[must_use]
pub fn spectral_fingerprint(
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
) -> [f32; BAND_COUNT] {
    if samples.is_empty() || sample_rate == 0 || !(channels == 1 || channels == 2) {
        return [0.0; BAND_COUNT];
    }
    let frames = samples.len() / channels;
    if frames < 16 {
        return [0.0; BAND_COUNT];
    }
    const WINDOW: usize = 2_048;
    if frames <= WINDOW {
        return normalize_bands(&band_energies(samples, channels, sample_rate));
    }
    let mut sums = [0.0f32; BAND_COUNT];
    let mut windows = 0u32;
    let mut start = 0usize;
    while start < frames {
        let remaining = frames - start;
        if remaining < 16 {
            break;
        }
        let window = remaining.min(WINDOW);
        let sample_start = start * channels;
        let sample_end = (start + window) * channels;
        let energies = band_energies(&samples[sample_start..sample_end], channels, sample_rate);
        for (sum, energy) in sums.iter_mut().zip(energies) {
            *sum += energy;
        }
        windows += 1;
        start += window;
    }
    if windows == 0 {
        return [0.0; BAND_COUNT];
    }
    let scale = windows as f32;
    for sum in &mut sums {
        *sum /= scale;
        if !sum.is_finite() {
            *sum = 0.0;
        }
    }
    normalize_bands(&sums)
}

/// Lever-style controls derived from spectrum bands (visualizer to room params).
///
/// Values are normalized 0..=1. Faces may map them onto zoom, scatter, rule
/// flips, or other room inputs without inventing a second analysis path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectrumLevers {
    /// Sub + bass energy.
    pub bass: f32,
    /// Low-mid through high-mid energy.
    pub mid: f32,
    /// Treble + air energy.
    pub treble: f32,
    /// Onset strength vs the previous frame (1.0 is steady).
    pub onset: f32,
}

/// Map seven bands into bass / mid / treble levers plus an onset proxy.
#[must_use]
pub fn levers_from_bands(
    previous: &[f32; BAND_COUNT],
    current: &[f32; BAND_COUNT],
) -> SpectrumLevers {
    let (bass, mid, treble) = bass_mid_treble(current);
    let total = (bass + mid + treble).max(f32::EPSILON);
    SpectrumLevers {
        bass: (bass / total).clamp(0.0, 1.0),
        mid: (mid / total).clamp(0.0, 1.0),
        treble: (treble / total).clamp(0.0, 1.0),
        onset: low_band_onset(previous, current),
    }
}

/// Onset strength above which a beat is considered to "hit".
pub const ONSET_HIT: f32 = 1.55;

/// Scale room time from spectrum bass (panel visualizer: bass pumps motion).
///
/// `base` is the player's ordinary time scale. Bass 0 keeps a calm floor;
/// bass 1 nearly doubles pace. Clamped so rooms never freeze or race.
#[must_use]
pub fn spectrum_time_scale(base: f64, levers: &SpectrumLevers) -> f64 {
    let pump = 0.55 + f64::from(levers.bass) * 0.95;
    (base * pump).clamp(0.25, 2.25)
}

/// Micro phase nudge from treble and onset (treble scatters time; beats kick).
#[must_use]
pub fn spectrum_phase_nudge(levers: &SpectrumLevers) -> f64 {
    let treble = f64::from(levers.treble) * 0.012;
    let beat = if levers.onset >= ONSET_HIT {
        0.018 * f64::from((levers.onset - 1.0).clamp(0.0, 3.0) / 3.0)
    } else {
        0.0
    };
    (treble + beat).clamp(0.0, 0.04)
}

/// Normalized hand point driven by spectrum (mid -> x, bass -> y).
///
/// Used as an optional soft poke target when a beat hits, so chaos rooms and
/// planting rooms can answer the music without scolding quiet mixes.
#[must_use]
pub fn spectrum_hand_point(levers: &SpectrumLevers) -> (f64, f64) {
    let x = (0.15 + f64::from(levers.mid) * 0.70).clamp(0.05, 0.95);
    let y = (0.20 + f64::from(levers.bass) * 0.60).clamp(0.05, 0.95);
    (x, y)
}

/// True when this frame should plant a soft spectrum poke.
#[must_use]
pub fn spectrum_should_poke(levers: &SpectrumLevers) -> bool {
    levers.onset >= ONSET_HIT && levers.bass + levers.mid + levers.treble > 0.05
}

/// Layout for [`draw_spectrum_bars`].
#[derive(Debug, Clone, Copy)]
pub struct SpectrumBarLayout {
    /// Left edge of the first bar, in pixels.
    pub left: usize,
    /// Bottom edge of the bars, in pixels.
    pub bottom: usize,
    /// Width of each bar in pixels.
    pub bar_width: usize,
    /// Maximum bar height in pixels.
    pub max_height: usize,
}

/// Draw seven vertical spectrum bars into an RGBA buffer (visualizer chrome).
///
/// Bars sit at the bottom of the given layout. Empty bands draw a one-pixel
/// baseline so the meter stays readable when the bed is quiet. Modern-era
/// identity is preserved for the rest of the frame: this only writes the bar
/// region.
pub fn draw_spectrum_bars(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    bands: &[f32; BAND_COUNT],
    layout: SpectrumBarLayout,
) {
    if width == 0 || height == 0 || rgba.len() < width * height * 4 {
        return;
    }
    let bar_width = layout.bar_width.max(1);
    let max_height = layout.max_height.max(1).min(height);
    let gap = 1usize;
    for (i, &level) in bands.iter().enumerate() {
        let h = ((level.clamp(0.0, 1.0) * max_height as f32).round() as usize).max(1);
        let x0 = layout.left + i * (bar_width + gap);
        let y1 = layout.bottom.min(height.saturating_sub(1));
        let y0 = y1.saturating_sub(h.saturating_sub(1));
        for y in y0..=y1 {
            for x in x0..x0.saturating_add(bar_width).min(width) {
                let o = (y * width + x) * 4;
                if o + 3 >= rgba.len() {
                    return;
                }
                // Soft cyan meter that reads on both phosphor green and modern.
                rgba[o] = 40;
                rgba[o + 1] = 220;
                rgba[o + 2] = 255;
                rgba[o + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BAND_COUNT, BAND_NAMES, SpectrumBarLayout, SpectrumLevers, arrangement_spectrum,
        band_energies, bass_mid_treble, draw_spectrum_bars, levers_from_bands, low_band_onset,
        normalize_bands, spectral_fingerprint, spectrum_hand_point, spectrum_phase_nudge,
        spectrum_should_poke, spectrum_time_scale,
    };

    fn sine_stereo(freq: f32, rate: u32, frames: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let t = i as f32 / rate as f32;
            let s = (std::f32::consts::TAU * freq * t).sin() * 0.5;
            out.push(s);
            out.push(s);
        }
        out
    }

    #[test]
    fn empty_and_hostile_inputs_are_quiet() {
        assert_eq!(band_energies(&[], 2, 16_000), [0.0; BAND_COUNT]);
        assert_eq!(band_energies(&[0.1, 0.1], 3, 16_000), [0.0; BAND_COUNT]);
        assert_eq!(band_energies(&[0.1; 32], 2, 0), [0.0; BAND_COUNT]);
    }

    #[test]
    fn a_low_sine_lights_the_bass_side() {
        let samples = sine_stereo(120.0, 16_000, 1_024);
        let bands = band_energies(&samples, 2, 16_000);
        let (bass, mid, treble) = bass_mid_treble(&bands);
        assert!(bass > mid, "bass {bass} should exceed mid {mid}");
        assert!(bass > treble, "bass {bass} should exceed treble {treble}");
        assert_eq!(BAND_NAMES.len(), BAND_COUNT);
    }

    #[test]
    fn a_high_sine_lights_the_treble_side() {
        let samples = sine_stereo(4_000.0, 16_000, 2_048);
        let bands = band_energies(&samples, 2, 16_000);
        let (bass, mid, treble) = bass_mid_treble(&bands);
        assert!(
            treble + mid > bass * 2.0,
            "high sine should leave bass: bass={bass} mid={mid} treble={treble} bands={bands:?}"
        );
        let mut peak = 0usize;
        let mut peak_e = -1.0f32;
        for (i, &e) in bands.iter().enumerate() {
            if e > peak_e {
                peak_e = e;
                peak = i;
            }
        }
        assert!(
            peak >= 3,
            "peak band {peak} should sit mid or higher for 4 kHz: {bands:?}"
        );
    }

    #[test]
    fn normalize_and_onset_are_stable() {
        let quiet = [0.0; BAND_COUNT];
        assert_eq!(normalize_bands(&quiet), quiet);
        assert_eq!(low_band_onset(&quiet, &quiet), 1.0);
        let mut loud = [0.0; BAND_COUNT];
        loud[0] = 4.0;
        loud[3] = 1.0;
        let n = normalize_bands(&loud);
        assert!((n[0] - 1.0).abs() < 1e-5);
        assert!((n[3] - 0.25).abs() < 1e-5);
        let attack = low_band_onset(&quiet, &loud);
        assert!(
            attack >= 1.5,
            "silent-to-loud should read as onset: {attack}"
        );
    }

    #[test]
    fn analysis_is_deterministic() {
        let samples = sine_stereo(440.0, 16_000, 512);
        assert_eq!(
            band_energies(&samples, 2, 16_000),
            band_energies(&samples, 2, 16_000)
        );
    }

    #[test]
    fn arrangement_spectrum_and_bars_are_safe() {
        let samples = sine_stereo(150.0, 16_000, 1_024);
        let bands = arrangement_spectrum(&samples, 16_000);
        assert!(bands.iter().all(|b| (0.0..=1.0).contains(b)));
        let mut rgba = vec![0u8; 64 * 32 * 4];
        draw_spectrum_bars(
            &mut rgba,
            64,
            32,
            &bands,
            SpectrumBarLayout {
                left: 2,
                bottom: 30,
                bar_width: 3,
                max_height: 20,
            },
        );
        assert!(rgba.iter().any(|&v| v > 0), "bars should light some pixels");
        // Hostile geometry must not panic.
        draw_spectrum_bars(
            &mut [],
            0,
            0,
            &bands,
            SpectrumBarLayout {
                left: 0,
                bottom: 0,
                bar_width: 0,
                max_height: 0,
            },
        );
        let levers = levers_from_bands(&[0.0; BAND_COUNT], &bands);
        assert!((0.0..=1.0).contains(&levers.bass));
        assert!((0.0..=1.0).contains(&levers.mid));
        assert!((0.0..=1.0).contains(&levers.treble));
        assert!(levers.onset >= 1.0);
        let quiet = SpectrumLevers {
            bass: 0.1,
            mid: 0.2,
            treble: 0.1,
            onset: 1.0,
        };
        let loud = SpectrumLevers {
            bass: 0.9,
            mid: 0.5,
            treble: 0.8,
            onset: 2.5,
        };
        assert!(spectrum_time_scale(1.0, &loud) > spectrum_time_scale(1.0, &quiet));
        assert!(spectrum_phase_nudge(&loud) > spectrum_phase_nudge(&quiet));
        assert!(spectrum_should_poke(&loud));
        assert!(!spectrum_should_poke(&quiet));
        let (x, y) = spectrum_hand_point(&loud);
        assert!((0.05..=0.95).contains(&x) && (0.05..=0.95).contains(&y));
    }

    fn peak_band(bands: &[f32; BAND_COUNT]) -> usize {
        bands
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    #[test]
    fn spectral_fingerprint_is_quiet_for_silence_and_hostile_input() {
        assert_eq!(spectral_fingerprint(&[], 2, 16_000), [0.0; BAND_COUNT]);
        assert_eq!(
            spectral_fingerprint(&[0.0; 30], 2, 16_000),
            [0.0; BAND_COUNT]
        );
        assert_eq!(
            spectral_fingerprint(&[0.1; 64], 3, 16_000),
            [0.0; BAND_COUNT]
        );
        assert_eq!(spectral_fingerprint(&[0.1; 64], 2, 0), [0.0; BAND_COUNT]);
    }

    #[test]
    fn a_short_buffer_matches_the_visualizer_window() {
        let samples = sine_stereo(440.0, 16_000, 2_048);
        assert_eq!(
            spectral_fingerprint(&samples, 2, 16_000),
            normalize_bands(&band_energies(&samples, 2, 16_000))
        );
        let shorter = sine_stereo(440.0, 16_000, 512);
        assert_eq!(
            spectral_fingerprint(&shorter, 2, 16_000),
            normalize_bands(&band_energies(&shorter, 2, 16_000))
        );
    }

    #[test]
    fn distinct_tones_do_not_share_a_fingerprint() {
        let bass = spectral_fingerprint(&sine_stereo(150.0, 16_000, 2_048), 2, 16_000);
        let mid = spectral_fingerprint(&sine_stereo(1_000.0, 16_000, 2_048), 2, 16_000);
        assert_ne!(
            peak_band(&bass),
            peak_band(&mid),
            "bass {bass:?} mid {mid:?}"
        );
        assert_eq!(bass[BAND_COUNT - 1], 0.0, "air is above Nyquist at 16 kHz");
        assert_eq!(mid[BAND_COUNT - 1], 0.0, "air is above Nyquist at 16 kHz");
        assert!(
            bass.iter()
                .all(|band| band.is_finite() && (0.0..=1.0).contains(band)),
            "{bass:?}"
        );
        let quiet: Vec<f32> = sine_stereo(1_000.0, 16_000, 2_048)
            .into_iter()
            .map(|sample| sample * 0.25)
            .collect();
        let gained = spectral_fingerprint(&quiet, 2, 16_000);
        for (index, (left, right)) in mid.iter().zip(gained).enumerate() {
            assert!(
                (left - right).abs() < 1e-5,
                "band {index} moved under gain: {left} vs {right}"
            );
        }
        assert_eq!(
            spectral_fingerprint(&sine_stereo(1_000.0, 16_000, 2_048), 2, 16_000),
            mid
        );
    }

    #[test]
    fn a_long_buffer_keeps_timbre_from_before_the_tail() {
        let mut samples = sine_stereo(150.0, 16_000, 2_048);
        samples.extend(sine_stereo(1_000.0, 16_000, 2_048));
        let tail = arrangement_spectrum(&samples, 16_000);
        let whole = spectral_fingerprint(&samples, 2, 16_000);
        assert_ne!(
            tail, whole,
            "the tail window must not stand in for the whole bed"
        );
        let (bass, mid, _) = bass_mid_treble(&tail);
        assert!(mid > bass, "the tail is the later tone: {tail:?}");
        assert!(
            whole[1] > 0.2,
            "the earlier bass tone must survive in the fingerprint: {whole:?}"
        );
        assert_eq!(whole[BAND_COUNT - 1], 0.0);
    }

    #[test]
    fn a_non_finite_energy_does_not_survive_normalization() {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = f32::NAN;
        bands[1] = 4.0;
        bands[2] = f32::INFINITY;
        bands[3] = -2.0;
        let normalized = normalize_bands(&bands);
        assert!(
            normalized.iter().all(|band| band.is_finite()),
            "{normalized:?}"
        );
        assert_eq!(normalized[1], 1.0);
        assert_eq!(normalized[0], 0.0);
        assert_eq!(normalized[2], 0.0);
        assert_eq!(normalized[3], 0.0);
        assert_eq!(normalize_bands(&[f32::NAN; BAND_COUNT]), [0.0; BAND_COUNT]);
        assert_eq!(
            normalize_bands(&[f32::NEG_INFINITY; BAND_COUNT]),
            [0.0; BAND_COUNT]
        );
    }

    #[test]
    fn a_non_finite_sample_stays_out_of_the_fingerprint() {
        let mut samples = sine_stereo(440.0, 16_000, 512);
        samples[10] = f32::NAN;
        samples[11] = f32::INFINITY;
        let fingerprint = spectral_fingerprint(&samples, 2, 16_000);
        assert!(
            fingerprint
                .iter()
                .all(|band| band.is_finite() && (0.0..=1.0).contains(band)),
            "{fingerprint:?}"
        );
        let energies = band_energies(&samples, 2, 16_000);
        assert!(
            energies.iter().all(|band| band.is_finite() && *band >= 0.0),
            "{energies:?}"
        );
        let levers = levers_from_bands(&energies, &energies);
        assert!(
            levers.bass.is_finite()
                && levers.mid.is_finite()
                && levers.treble.is_finite()
                && levers.onset.is_finite()
        );
        assert_eq!(
            spectral_fingerprint(&[f32::NAN; 64], 2, 16_000),
            [0.0; BAND_COUNT]
        );
        let mut quiet = [0.0; BAND_COUNT];
        quiet[0] = f32::NAN;
        assert_eq!(low_band_onset(&quiet, &quiet), 1.0);
    }

    #[test]
    fn a_non_finite_band_does_not_move_the_levers() {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = f32::NAN;
        bands[1] = 1.0;
        bands[3] = f32::INFINITY;
        let levers = levers_from_bands(&bands, &bands);
        assert!(
            (0.0..=1.0).contains(&levers.bass)
                && (0.0..=1.0).contains(&levers.mid)
                && (0.0..=1.0).contains(&levers.treble),
            "{levers:?}"
        );
        assert_eq!(levers.onset, 1.0);
        assert!(spectrum_time_scale(1.0, &levers).is_finite());
        assert!(spectrum_phase_nudge(&levers).is_finite());
        let (x, y) = spectrum_hand_point(&levers);
        assert!(x.is_finite() && y.is_finite());
        let (bass, mid, treble) = bass_mid_treble(&bands);
        assert!(bass.is_finite() && mid.is_finite() && treble.is_finite());
        assert_eq!(mid, 0.0);
    }
}
