//! Bounded stereo percussion for the highlighted Galton ball path.

use std::f32::consts::{FRAC_PI_4, TAU};

/// Fixed half-second event keeps repeated drops responsive and bounded.
const SOUND_SECONDS: f32 = 0.5;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 192_000;
const PEG_STEP_SECONDS: f32 = 0.024;
const PEG_SECONDS: f32 = 0.038;
const LANDING_START_SECONDS: f32 = 0.4;
const LANDING_SECONDS: f32 = 0.095;
const PEG_SCALE_STEPS: [i32; 8] = [12, 9, 7, 4, 2, 0, 2, 4];
const LANDING_SCALE_STEPS: [i32; 5] = [0, 2, 4, 7, 9];

fn stereo_gains(pan: f32) -> (f32, f32) {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (angle.cos(), angle.sin())
}

struct Tone {
    start: f32,
    duration: f32,
    frequency: f32,
    amplitude: f32,
    pan: f32,
    decay_power: i32,
}

fn add_tone(samples: &mut [f32], sample_rate: u32, tone: Tone) {
    let rate = sample_rate as f32;
    let start_frame = (tone.start * rate) as usize;
    let frame_count = (tone.duration * rate) as usize;
    let (left_gain, right_gain) = stereo_gains(tone.pan);
    for frame in 0..frame_count {
        let target = start_frame.saturating_add(frame);
        let offset = target.saturating_mul(2);
        let Some(stereo) = samples.get_mut(offset..offset.saturating_add(2)) else {
            break;
        };
        let seconds = frame as f32 / rate;
        let progress = seconds / tone.duration;
        let attack = (seconds / 0.003).clamp(0.0, 1.0);
        let envelope = attack * (1.0 - progress).max(0.0).powi(tone.decay_power);
        let phase = TAU * tone.frequency * seconds;
        let timbre = phase.sin().mul_add(0.78, (phase * 2.01).sin() * 0.22);
        let sample = timbre * tone.amplitude * envelope;
        stereo[0] += sample * left_gain;
        stereo[1] += sample * right_gain;
    }
}

pub(super) fn supports_sample_rate(sample_rate: u32) -> bool {
    (MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate)
}

pub(super) fn render(root: f32, trace: &[usize], rows: usize, sample_rate: u32) -> Vec<f32> {
    debug_assert!(supports_sample_rate(sample_rate));
    let frames = (SOUND_SECONDS * sample_rate as f32) as usize;
    let mut samples = vec![0.0; frames.saturating_mul(2)];
    let rows = rows.max(1);

    for (row, edge) in trace.windows(2).take(rows).enumerate() {
        let went_right = edge[1] > edge[0];
        let degree = PEG_SCALE_STEPS[row % PEG_SCALE_STEPS.len()] + i32::from(went_right) * 2;
        let destination_row = row + 1;
        add_tone(
            &mut samples,
            sample_rate,
            Tone {
                start: row as f32 * PEG_STEP_SECONDS,
                duration: PEG_SECONDS,
                frequency: crate::chiptune::pitch(root, degree),
                amplitude: 0.09,
                pan: (2.0 * edge[1] as f32 - destination_row as f32) / rows as f32,
                decay_power: 4,
            },
        );
    }

    if let Some(&rights) = trace.last() {
        add_tone(
            &mut samples,
            sample_rate,
            Tone {
                start: LANDING_START_SECONDS,
                duration: LANDING_SECONDS,
                frequency: crate::chiptune::pitch(
                    root,
                    LANDING_SCALE_STEPS[rights % LANDING_SCALE_STEPS.len()],
                ),
                amplitude: 0.12,
                pan: (2.0 * rights as f32 - rows as f32) / rows as f32,
                decay_power: 2,
            },
        );
    }

    for sample in &mut samples {
        *sample = sample.clamp(-1.0, 1.0);
    }
    samples
}
