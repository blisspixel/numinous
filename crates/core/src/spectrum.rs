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
/// nearest DFT bin to each band center, not calibrated dB SPL.
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
            let mono = if channels == 1 {
                samples[frame]
            } else {
                let o = frame * 2;
                0.5 * (samples[o] + samples[o + 1])
            };
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
        out[band] = (re * re + im * im) / (window as f32 * window as f32);
    }
    out
}

/// Collapse seven bands into a coarse bass / mid / treble triple for lever maps.
#[must_use]
pub fn bass_mid_treble(bands: &[f32; BAND_COUNT]) -> (f32, f32, f32) {
    let bass = bands[0] + bands[1];
    let mid = bands[2] + bands[3] + bands[4];
    let treble = bands[5] + bands[6];
    (bass, mid, treble)
}

/// Normalize band energies so the loudest band is 1.0 (or all zeros stay zero).
#[must_use]
pub fn normalize_bands(bands: &[f32; BAND_COUNT]) -> [f32; BAND_COUNT] {
    let peak = bands.iter().copied().fold(0.0f32, f32::max);
    if peak <= f32::EPSILON {
        return [0.0; BAND_COUNT];
    }
    let mut out = [0.0f32; BAND_COUNT];
    for (i, &e) in bands.iter().enumerate() {
        out[i] = (e / peak).clamp(0.0, 1.0);
    }
    out
}

/// Coarse onset proxy: low-band energy of this frame over the previous frame.
///
/// Values near 1.0 mean little change; above ~1.5 suggests a low-band attack.
/// Either side empty or silent returns 1.0 (no onset).
#[must_use]
pub fn low_band_onset(previous: &[f32; BAND_COUNT], current: &[f32; BAND_COUNT]) -> f32 {
    let prev = previous[0] + previous[1];
    let curr = current[0] + current[1];
    if prev <= f32::EPSILON {
        return if curr > f32::EPSILON { 2.0 } else { 1.0 };
    }
    (curr / prev).clamp(0.0, 8.0)
}

#[cfg(test)]
mod tests {
    use super::{
        BAND_COUNT, BAND_NAMES, band_energies, bass_mid_treble, low_band_onset, normalize_bands,
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
}
