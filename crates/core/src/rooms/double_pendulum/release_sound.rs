//! Bounded stereo sonification of one released twin-pendulum experiment.

use std::f32::consts::{FRAC_PI_4, TAU};

const SOUND_MILLISECONDS: usize = 720;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 192_000;
const PULSE_HORIZONS: [usize; 7] = [0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000];
const PULSE_START_SECONDS: f32 = 0.02;
const PULSE_STEP_SECONDS: f32 = 0.095;
const PULSE_SECONDS: f32 = 0.08;

fn stereo_gains(pan: f32) -> (f32, f32) {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (angle.cos(), angle.sin())
}

struct Tone {
    start: f32,
    frequency: f32,
    amplitude: f32,
    pan: f32,
}

fn add_tone(samples: &mut [f32], sample_rate: u32, tone: Tone) {
    let rate = sample_rate as f32;
    let start_frame = (tone.start * rate) as usize;
    let frame_count = (PULSE_SECONDS * rate) as usize;
    let (left_gain, right_gain) = stereo_gains(tone.pan);
    for frame in 0..frame_count {
        let target = start_frame.saturating_add(frame);
        let offset = target.saturating_mul(2);
        let Some(stereo) = samples.get_mut(offset..offset.saturating_add(2)) else {
            break;
        };
        let seconds = frame as f32 / rate;
        let progress = seconds / PULSE_SECONDS;
        let attack = (seconds / 0.004).clamp(0.0, 1.0);
        let envelope = attack * (1.0 - progress).max(0.0).powi(3);
        let phase = TAU * tone.frequency * seconds;
        let timbre = phase.sin().mul_add(0.82, (phase * 2.0).sin() * 0.18);
        let sample = timbre * tone.amplitude * envelope;
        stereo[0] += sample * left_gain;
        stereo[1] += sample * right_gain;
    }
}

fn divergence_mapping(gap: f64) -> (f32, f32) {
    let orders = (gap / super::SHADOW_OFFSET)
        .max(1.0)
        .log10()
        .clamp(0.0, 4.0) as f32;
    let amount = orders / 4.0;
    let interval_ratio = 2.0_f32.powf(amount);
    (interval_ratio, amount * 0.85)
}

pub(super) fn supports_sample_rate(sample_rate: u32) -> bool {
    (MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate)
}

pub(super) fn render(
    first: f64,
    second: f64,
    w1: f64,
    w2: f64,
    root_hz: f32,
    gesture_gain: f32,
    sample_rate: u32,
) -> Vec<f32> {
    debug_assert!(supports_sample_rate(sample_rate));
    let frames = sample_rate as usize * SOUND_MILLISECONDS / 1_000;
    let mut samples = vec![0.0; frames.saturating_mul(2)];
    let amplitude = (gesture_gain * 1.35).clamp(0.035, 0.075);

    let gaps = super::divergence_gaps(first, second, w1, w2, PULSE_HORIZONS);
    for (pulse, gap) in gaps.into_iter().enumerate() {
        let (interval_ratio, spread) = divergence_mapping(gap);
        let start = PULSE_START_SECONDS + pulse as f32 * PULSE_STEP_SECONDS;
        add_tone(
            &mut samples,
            sample_rate,
            Tone {
                start,
                frequency: root_hz,
                amplitude,
                pan: -spread,
            },
        );
        add_tone(
            &mut samples,
            sample_rate,
            Tone {
                start,
                frequency: root_hz * interval_ratio,
                amplitude,
                pan: spread,
            },
        );
    }

    for sample in &mut samples {
        *sample = sample.clamp(-1.0, 1.0);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::divergence_mapping;

    #[test]
    fn four_orders_of_twin_separation_reach_one_octave_and_full_width() {
        assert_eq!(divergence_mapping(0.0), (1.0, 0.0));
        let (ratio, spread) = divergence_mapping(super::super::SHADOW_OFFSET * 10_000.0);
        assert_eq!(ratio, 2.0);
        assert_eq!(spread, 0.85);
    }

    #[test]
    fn the_mapping_is_bounded_for_extreme_finite_gaps() {
        for gap in [f64::MIN_POSITIVE, 1.0, f64::MAX] {
            let (ratio, spread) = divergence_mapping(gap);
            assert!(ratio.is_finite() && (1.0..=2.0).contains(&ratio));
            assert!(spread.is_finite() && (0.0..=0.85).contains(&spread));
        }
    }
}
