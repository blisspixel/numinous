//! Probe the system audio output, write a one-second 440 Hz tone to a WAV, and
//! play it through the default device. Run: cargo run -p numinous-audio --example tone

fn main() {
    // Always write a WAV (no device needed), so there is a verifiable artifact.
    let sample_rate = 44_100u32;
    let samples = numinous_audio::synthesize_sine(440.0, sample_rate, sample_rate as usize);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("tone.wav", spec).expect("create tone.wav");
    for s in &samples {
        writer
            .write_sample((s * f32::from(i16::MAX)) as i16)
            .expect("write sample");
    }
    writer.finalize().expect("finalize wav");
    println!(
        "wrote tone.wav ({} samples at {sample_rate} Hz)",
        samples.len()
    );

    // Then adapt to the system default device and actually play it.
    match numinous_audio::AudioContext::new() {
        Ok(ctx) => {
            println!(
                "Output device: {} | {} Hz | {} channels",
                ctx.device_name(),
                ctx.sample_rate(),
                ctx.channels()
            );
            match ctx.play_tone(440.0, 1.0) {
                Ok(()) => println!("played 440 Hz for 1 second"),
                Err(e) => println!("playback failed: {e}"),
            }
        }
        Err(e) => println!("no audio device available: {e}"),
    }
}
