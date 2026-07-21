//! Capture ring and optional input/loopback streams for the visualizer path.
//!
//! The ring is fixed-capacity and allocation-free after construction so the
//! real-time mixer or an input callback can push frames safely. Control-thread
//! code copies a recent window for spectrum analysis (core band-energy path).

use std::sync::{Arc, Mutex};

use cpal::Sample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Fixed interleaved-stereo ring written by audio callbacks.
#[derive(Debug)]
pub struct CaptureRing {
    samples: Box<[f32]>,
    /// Next write index into `samples` (always even: left, right).
    head: usize,
    /// How many samples currently hold valid data (0..=samples.len()).
    filled: usize,
    sample_rate: u32,
}

impl CaptureRing {
    /// Create a ring holding about `frame_capacity` stereo frames.
    #[must_use]
    pub fn new(frame_capacity: usize, sample_rate: u32) -> Self {
        let frames = frame_capacity.max(2);
        let samples = vec![0.0f32; frames.saturating_mul(2)].into_boxed_slice();
        Self {
            samples,
            head: 0,
            filled: 0,
            sample_rate: sample_rate.max(1),
        }
    }

    /// Device rate the ring was sized for.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Push one stereo frame. Overwrites oldest data when full.
    pub fn push_frame(&mut self, left: f32, right: f32) {
        if self.samples.len() < 2 {
            return;
        }
        self.samples[self.head] = left.clamp(-1.0, 1.0);
        self.samples[self.head + 1] = right.clamp(-1.0, 1.0);
        self.head += 2;
        if self.head >= self.samples.len() {
            self.head = 0;
        }
        self.filled = (self.filled + 2).min(self.samples.len());
    }

    /// Copy the most recent `max_frames` stereo frames (interleaved).
    ///
    /// Returns an empty vec when the ring is empty. Order is chronological
    /// oldest-to-newest within the retained window.
    #[must_use]
    pub fn snapshot_frames(&self, max_frames: usize) -> Vec<f32> {
        if self.filled < 2 || max_frames == 0 || self.samples.is_empty() {
            return Vec::new();
        }
        let want = max_frames.saturating_mul(2).min(self.filled);
        // Align to full frames.
        let want = want - (want % 2);
        if want == 0 {
            return Vec::new();
        }
        let mut out = vec![0.0f32; want];
        let len = self.samples.len();
        for (i, sample) in out.iter_mut().enumerate() {
            let idx = (self.head + len - want + i) % len;
            *sample = self.samples[idx];
        }
        out
    }
}

/// Whether a device name looks like a system loopback / stereo-mix source.
#[must_use]
pub fn looks_like_loopback_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("loopback")
        || lower.contains("stereo mix")
        || lower.contains("what u hear")
        || lower.contains("wave out mix")
        || lower.contains("waveout mix")
        || lower.contains("mixed output")
        || lower.contains("system audio")
        || lower.contains("cable output")
        || lower.contains("blackhole")
        || lower.contains("soundflower")
        || lower.contains("monitor of")
}

/// Optional input stream writing into a shared capture ring.
pub struct InputCapture {
    _stream: cpal::Stream,
    ring: Arc<Mutex<CaptureRing>>,
    device_name: String,
    sample_rate: u32,
}

impl InputCapture {
    /// Open the first input device whose name looks like system loopback.
    ///
    /// # Errors
    /// Returns a guiding error when no loopback-like input is available or the
    /// stream cannot be built. Callers should fall back to output-mix tap or
    /// room-bed analysis.
    pub fn try_open_loopback() -> Result<Self, String> {
        let host = cpal::default_host();
        let mut devices = host
            .input_devices()
            .map_err(|e| format!("could not enumerate input devices: {e}"))?;
        let mut chosen: Option<(cpal::Device, String)> = None;
        for device in devices.by_ref() {
            let name = device
                .description()
                .map(|d| d.name().to_owned())
                .unwrap_or_else(|_| "unknown".to_string());
            if looks_like_loopback_name(&name) {
                chosen = Some((device, name));
                break;
            }
        }
        let (device, device_name) = chosen.ok_or_else(|| {
            "no loopback-like input device found (enable Stereo Mix, or install a virtual cable)"
                .to_string()
        })?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("loopback device has no default input config: {e}"))?;
        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;
        if sample_rate == 0 || channels == 0 {
            return Err("loopback device reported degenerate rate or channels".to_string());
        }
        let ring = Arc::new(Mutex::new(CaptureRing::new(4_096, sample_rate)));
        let ring_cb = Arc::clone(&ring);
        let stream_config: cpal::StreamConfig = config.into();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                build_input_stream::<f32>(&device, stream_config, channels, ring_cb)
            }
            cpal::SampleFormat::I16 => {
                build_input_stream::<i16>(&device, stream_config, channels, ring_cb)
            }
            cpal::SampleFormat::U16 => {
                build_input_stream::<u16>(&device, stream_config, channels, ring_cb)
            }
            other => {
                return Err(format!(
                    "loopback device sample format not supported for capture: {other:?}"
                ));
            }
        }
        .map_err(|e| format!("could not build loopback input stream: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("could not start loopback input stream: {e}"))?;
        Ok(Self {
            _stream: stream,
            ring,
            device_name,
            sample_rate,
        })
    }

    /// Human-readable capture device name.
    #[must_use]
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Capture sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Shared ring for spectrum analysis on the control thread.
    #[must_use]
    pub fn ring(&self) -> Arc<Mutex<CaptureRing>> {
        Arc::clone(&self.ring)
    }

    /// Copy recent frames for analysis.
    #[must_use]
    pub fn snapshot_frames(&self, max_frames: usize) -> Vec<f32> {
        self.ring
            .lock()
            .map(|ring| ring.snapshot_frames(max_frames))
            .unwrap_or_default()
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    ring: Arc<Mutex<CaptureRing>>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            if let Ok(mut ring) = ring.lock() {
                for frame in data.chunks(channels.max(1)) {
                    let left = frame.first().map(|s| f32::from_sample(*s)).unwrap_or(0.0);
                    let right = if channels >= 2 {
                        frame.get(1).map(|s| f32::from_sample(*s)).unwrap_or(left)
                    } else {
                        left
                    };
                    ring.push_frame(left, right);
                }
            }
        },
        |error| eprintln!("loopback capture error: {error}"),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::{CaptureRing, looks_like_loopback_name};

    #[test]
    fn ring_overwrite_and_snapshot_are_stable() {
        let mut ring = CaptureRing::new(4, 16_000);
        assert!(ring.snapshot_frames(8).is_empty());
        ring.push_frame(0.1, -0.1);
        ring.push_frame(0.2, -0.2);
        ring.push_frame(0.3, -0.3);
        let snap = ring.snapshot_frames(2);
        assert_eq!(snap, vec![0.2, -0.2, 0.3, -0.3]);

        let mut ring = CaptureRing::new(4, 16_000);
        for i in 0..8 {
            let v = (i + 1) as f32;
            ring.push_frame(v, -v);
        }
        let full = ring.snapshot_frames(4);
        assert_eq!(full.len(), 8, "four stereo frames");
        // Capacity 4: after frames 1..8 the retained window is 5..8, clamped to [-1, 1].
        assert_eq!(full, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]);
        // Sub-unit amplitudes retain order without clamping.
        let mut ring = CaptureRing::new(4, 16_000);
        for i in 0..8 {
            let v = (i + 1) as f32 * 0.1;
            ring.push_frame(v, -v);
        }
        let full = ring.snapshot_frames(4);
        assert_eq!(full, vec![0.5, -0.5, 0.6, -0.6, 0.7, -0.7, 0.8, -0.8]);
    }

    #[test]
    fn loopback_name_heuristics_cover_common_drivers() {
        assert!(looks_like_loopback_name("Stereo Mix (Realtek)"));
        assert!(looks_like_loopback_name("CABLE Output (VB-Audio)"));
        assert!(looks_like_loopback_name("BlackHole 2ch"));
        assert!(looks_like_loopback_name("Monitor of Built-in Audio"));
        assert!(!looks_like_loopback_name("Microphone Array"));
        assert!(!looks_like_loopback_name("Speakers"));
    }
}
