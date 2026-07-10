use std::{
    collections::BinaryHeap,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) const MAX_TRACKS: usize = 64;
pub(crate) const MAX_WAV_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SOURCE_SAMPLES: usize = MAX_WAV_BYTES as usize / 2;
const MAX_OUTPUT_SECONDS: usize = 60 * 8;

pub(crate) struct LoadedTrack {
    pub(crate) stereo: Vec<f32>,
    pub(crate) remaining: Duration,
}

pub(crate) fn default_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("NUMINOUS_RADIO") {
        PathBuf::from(dir)
    } else {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".numinous-radio")
    }
}

pub(crate) fn station_tracks(dir: &Path, station_id: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let prefix = format!("{station_id}-");
    let legacy = format!("{station_id}.wav");
    let mut candidates = BinaryHeap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !(name.starts_with(&prefix) || name == legacy) || !wav_is_bounded(&path) {
            continue;
        }
        if candidates.len() >= MAX_TRACKS
            && candidates.peek().is_some_and(|largest| &path >= largest)
        {
            continue;
        }
        if playable_info(&path).is_some() {
            if candidates.len() >= MAX_TRACKS {
                let _ = candidates.pop();
            }
            candidates.push(path);
        }
    }
    candidates.into_sorted_vec()
}

pub(crate) fn live_position(paths: &[PathBuf], now_secs: f64) -> Option<(usize, f64)> {
    if paths.is_empty() {
        return None;
    }
    let durations: Vec<f64> = paths
        .iter()
        .map(|path| duration_seconds(path).unwrap_or(0.0))
        .collect();
    let total: f64 = durations.iter().sum();
    let mut position = if total > 1.0 && now_secs.is_finite() {
        now_secs.rem_euclid(total)
    } else {
        0.0
    };
    for (index, &seconds) in durations.iter().enumerate() {
        if position < seconds || index == durations.len() - 1 {
            return Some((index, position));
        }
        position -= seconds;
    }
    Some((0, 0.0))
}

pub(crate) fn wav_is_bounded(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0 && meta.len() <= MAX_WAV_BYTES)
        .unwrap_or(false)
}

pub(crate) fn duration_seconds(path: &Path) -> Option<f64> {
    let info = track_info(path)?;
    Some(info.frames as f64 / f64::from(info.sample_rate))
}

pub(crate) fn load_track(path: &Path, offset: f64, device_rate: u32) -> Option<LoadedTrack> {
    if device_rate == 0 {
        return None;
    }
    let info = track_info(path)?;
    let src_rate = f64::from(info.sample_rate);
    let channels = info.channels;
    let raw = read_samples(path, &info)?;
    let src_frames = info.frames;
    if src_frames < 2 {
        return None;
    }
    let out_frames = (src_frames as f64 * f64::from(device_rate) / src_rate) as usize;
    let max_output_frames = device_rate as usize * MAX_OUTPUT_SECONDS;
    if out_frames == 0 || out_frames > max_output_frames {
        return None;
    }
    let mut stereo = Vec::with_capacity(out_frames * 2);
    for i in 0..out_frames {
        let src = i as f64 * src_rate / f64::from(device_rate);
        let base = (src as usize).min(src_frames - 2);
        let frac = (src - base as f64) as f32;
        for ch in [0, channels.saturating_sub(1)] {
            let a = raw[base * channels + ch];
            let b = raw[(base + 1) * channels + ch];
            stereo.push(a + (b - a) * frac);
        }
    }
    let skip_frames =
        ((offset.max(0.0) * f64::from(device_rate)) as usize).min(out_frames.saturating_sub(1));
    let stereo = stereo.split_off(skip_frames * 2);
    let remaining = (out_frames - skip_frames) as f64 / f64::from(device_rate);
    Some(LoadedTrack {
        stereo,
        remaining: Duration::from_secs_f64(remaining.max(1.0 / f64::from(device_rate))),
    })
}

fn playable_info(path: &Path) -> Option<TrackInfo> {
    track_info(path)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrackInfo {
    sample_rate: u32,
    channels: usize,
    frames: usize,
    samples: usize,
}

fn track_info(path: &Path) -> Option<TrackInfo> {
    let reader = open_bounded_wav_reader(path)?;
    track_info_from_reader(&reader)
}

fn track_info_from_reader(
    reader: &hound::WavReader<BufReader<std::fs::File>>,
) -> Option<TrackInfo> {
    let spec = reader.spec();
    if spec.sample_rate == 0
        || spec.channels == 0
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return None;
    }
    let channels = usize::from(spec.channels);
    let frames = usize::try_from(reader.duration()).ok()?;
    let samples = frames.checked_mul(channels)?;
    if frames < 2 || samples > MAX_SOURCE_SAMPLES {
        return None;
    }
    Some(TrackInfo {
        sample_rate: spec.sample_rate,
        channels,
        frames,
        samples,
    })
}

fn read_samples(path: &Path, info: &TrackInfo) -> Option<Vec<f32>> {
    let mut reader = open_bounded_wav_reader(path)?;
    if track_info_from_reader(&reader).as_ref() != Some(info) {
        return None;
    }
    let raw_i16: Vec<i16> = reader.samples::<i16>().collect::<Result<_, _>>().ok()?;
    if raw_i16.len() != info.samples || raw_i16.len() % info.channels != 0 {
        return None;
    }
    Some(
        raw_i16
            .into_iter()
            .map(|sample| f32::from(sample) / 32_768.0)
            .collect(),
    )
}

fn open_bounded_wav_reader(path: &Path) -> Option<hound::WavReader<BufReader<std::fs::File>>> {
    let file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_WAV_BYTES {
        return None;
    }
    hound::WavReader::new(BufReader::new(file)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(path: &Path, channels: u16, seconds: u32) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("write wav");
        for i in 0..44_100 * seconds {
            let sample = ((i as f32 * 0.05).sin() * 12_000.0) as i16;
            for channel in 0..channels {
                let signed = if channel % 2 == 0 { sample } else { -sample };
                writer.write_sample(signed).expect("sample");
            }
        }
        writer.finalize().expect("finalize");
    }

    #[test]
    fn station_tracks_are_bounded_sorted_and_station_scoped() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_tracks");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        write_wav(&dir.join("trance-002.wav"), 1, 1);
        write_wav(&dir.join("trance-001.wav"), 1, 1);
        write_wav(&dir.join("chill-001.wav"), 1, 1);
        std::fs::write(
            dir.join("trance-bad.wav"),
            b"RIFF not really a playable wav",
        )
        .expect("bad file");

        let tracks = station_tracks(&dir, "trance");

        assert_eq!(tracks.len(), 2);
        assert!(tracks[0].ends_with("trance-001.wav"));
        assert!(tracks[1].ends_with("trance-002.wav"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn station_tracks_are_sorted_before_the_track_cap() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_sorted_cap");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        for i in (0..MAX_TRACKS + 3).rev() {
            write_wav(&dir.join(format!("trance-{i:03}.wav")), 1, 1);
        }

        let tracks = station_tracks(&dir, "trance");

        assert_eq!(tracks.len(), MAX_TRACKS);
        assert!(tracks[0].ends_with("trance-000.wav"));
        let last_expected = format!("trance-{:03}.wav", MAX_TRACKS - 1);
        assert!(tracks[MAX_TRACKS - 1].ends_with(last_expected.as_str()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_low_sorted_tracks_do_not_consume_the_track_cap() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_invalid_low_names");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        for i in 0..MAX_TRACKS {
            std::fs::write(
                dir.join(format!("trance-{i:03}.wav")),
                b"RIFF not really playable",
            )
            .expect("bad wav");
        }
        write_wav(&dir.join(format!("trance-{:03}.wav", MAX_TRACKS)), 1, 1);

        let tracks = station_tracks(&dir, "trance");

        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].ends_with(format!("trance-{MAX_TRACKS:03}.wav").as_str()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn duration_uses_frames_for_stereo_tracks() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_duration");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("stereo.wav");
        write_wav(&path, 2, 3);

        let duration = duration_seconds(&path).expect("duration");

        assert!(
            (2.9..=3.1).contains(&duration),
            "duration should be about three seconds, got {duration}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_track_resamples_to_stereo_and_rotates_into_the_broadcast() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_load");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("mono.wav");
        write_wav(&path, 1, 3);

        let loaded = load_track(&path, 1.0, 48_000).expect("load");

        assert!(loaded.stereo.len() > 44_100 * 2);
        assert!(loaded.stereo.iter().any(|sample| sample.abs() > 0.1));
        assert!(loaded.remaining.as_secs_f64() >= 1.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_track_accepts_high_rate_devices_within_the_time_cap() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_high_rate");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("mono.wav");
        write_wav(&path, 1, 3);

        let loaded = load_track(&path, 0.0, 96_000).expect("load at high device rate");

        assert!(loaded.stereo.len() > 96_000 * 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_track_does_not_wrap_the_end_of_a_live_track() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_boundary");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("mono.wav");
        write_wav(&path, 1, 3);

        let loaded = load_track(&path, 2.9, 44_100).expect("load near track end");

        assert!(
            (0.05..=0.2).contains(&loaded.remaining.as_secs_f64()),
            "remaining should be only the suffix, got {:?}",
            loaded.remaining
        );
        assert!(loaded.stereo.len() < 44_100);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unsupported_wav_files_are_not_playable_tracks() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_unsupported");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("trance-float.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("write float wav");
        for _ in 0..44_100 {
            writer.write_sample(0.25f32).expect("float sample");
        }
        writer.finalize().expect("finalize");

        assert!(wav_is_bounded(&path));
        assert!(duration_seconds(&path).is_none());
        assert!(station_tracks(&dir, "trance").is_empty());
        assert!(load_track(&path, 0.0, 44_100).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn oversized_files_are_rejected_before_decode() {
        let path = std::env::temp_dir().join("numinous_radio_cache_oversized.wav");
        let file = std::fs::File::create(&path).expect("oversized placeholder");
        file.set_len(MAX_WAV_BYTES + 1).expect("make sparse file");

        assert!(!wav_is_bounded(&path));
        assert!(duration_seconds(&path).is_none());
        assert!(load_track(&path, 0.0, 44_100).is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sample_read_rechecks_file_bounds_on_the_opened_handle() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_swapped_bounds");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("trance-001.wav");
        write_wav(&path, 1, 1);
        let info = track_info(&path).expect("initial bounded info");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("reopen wav");
        file.set_len(MAX_WAV_BYTES + 1)
            .expect("inflate after metadata pass");

        assert!(read_samples(&path, &info).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sample_read_rejects_bounded_swapped_track_info() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_swapped_header");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("trance-001.wav");
        write_wav(&path, 1, 1);
        let info = track_info(&path).expect("initial mono info");
        write_wav(&path, 2, 1);

        assert!(read_samples(&path, &info).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
