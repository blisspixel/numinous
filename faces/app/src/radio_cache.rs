use std::{
    collections::BinaryHeap,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use symphonia::core::{
    audio::sample::Sample,
    codecs::audio::AudioDecoderOptions,
    errors::Error as SymphoniaError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
};

pub(crate) const MAX_TRACKS: usize = 64;
pub(crate) const MAX_AUDIO_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SOURCE_SAMPLES: usize = MAX_AUDIO_BYTES as usize / 2;
const MAX_OUTPUT_SECONDS: usize = 60 * 8;
const MAX_DECODED_STEREO_BYTES: usize = 128 * 1024 * 1024;
const MAX_SCAN_ENTRIES: usize = 4096;
const MAX_SCAN_METADATA_PROBES: usize = 256;
const MAX_SCAN_AUDIO_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SCAN_FALLBACK_DECODES: usize = 1;

pub(crate) struct LoadedTrack {
    pub(crate) stereo: Arc<Vec<f32>>,
    pub(crate) sample_rate: u32,
    pub(crate) remaining: Duration,
}

#[derive(Clone, Copy)]
struct DiscoveryLimits {
    entries: usize,
    metadata_probes: usize,
    audio_bytes: u64,
    fallback_decodes: usize,
}

const DISCOVERY_LIMITS: DiscoveryLimits = DiscoveryLimits {
    entries: MAX_SCAN_ENTRIES,
    metadata_probes: MAX_SCAN_METADATA_PROBES,
    audio_bytes: MAX_SCAN_AUDIO_BYTES,
    fallback_decodes: MAX_SCAN_FALLBACK_DECODES,
};

struct DiscoveryBudget {
    limits: DiscoveryLimits,
    entries: usize,
    metadata_probes: usize,
    audio_bytes: u64,
    fallback_decodes: usize,
}

impl DiscoveryBudget {
    fn new(limits: DiscoveryLimits) -> Self {
        Self {
            limits,
            entries: 0,
            metadata_probes: 0,
            audio_bytes: 0,
            fallback_decodes: 0,
        }
    }

    fn claim_entry(&mut self) -> bool {
        if self.entries >= self.limits.entries {
            return false;
        }
        self.entries += 1;
        true
    }

    fn claim_metadata_probe(&mut self) -> bool {
        if self.metadata_probes >= self.limits.metadata_probes {
            return false;
        }
        self.metadata_probes += 1;
        true
    }

    fn claim_audio_bytes(&mut self, bytes: u64) -> bool {
        let Some(total) = self.audio_bytes.checked_add(bytes) else {
            return false;
        };
        if total > self.limits.audio_bytes {
            return false;
        }
        self.audio_bytes = total;
        true
    }

    fn claim_fallback_decode(&mut self) -> bool {
        if self.fallback_decodes >= self.limits.fallback_decodes {
            return false;
        }
        self.fallback_decodes += 1;
        true
    }
}

pub(crate) fn default_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("NUMINOUS_RADIO") {
        return PathBuf::from(dir);
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join("assets").join("radio"));
        candidates.push(parent.join("radio"));
    }
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("assets").join("radio"));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("radio"),
    );
    if let Some(dir) = candidates.into_iter().find(|path| path.is_dir()) {
        return dir;
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".numinous-radio")
}

pub(crate) fn station_tracks(dir: &Path, station_id: &str) -> Vec<PathBuf> {
    station_tracks_with_limits(dir, station_id, DISCOVERY_LIMITS)
}

fn station_tracks_with_limits(
    dir: &Path,
    station_id: &str,
    limits: DiscoveryLimits,
) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let prefix = format!("{station_id}-");
    let legacy_wav = format!("{station_id}.wav");
    let legacy_mp3 = format!("{station_id}.mp3");
    let mut candidates = BinaryHeap::new();
    let mut budget = DiscoveryBudget::new(limits);
    for entry in entries {
        if !budget.claim_entry() {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let supported = matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("wav" | "mp3")
        );
        if !(name.starts_with(&prefix) || name == legacy_wav || name == legacy_mp3) || !supported {
            continue;
        }
        if candidates.len() >= MAX_TRACKS
            && candidates.peek().is_some_and(|largest| &path >= largest)
        {
            continue;
        }
        if !budget.claim_metadata_probe() {
            break;
        }
        let Some(bytes) = bounded_audio_len(&path) else {
            continue;
        };
        if !budget.claim_audio_bytes(bytes) {
            break;
        }
        if playable_info(&path, &mut budget).is_some() {
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

#[cfg(test)]
pub(crate) fn audio_is_bounded(path: &Path) -> bool {
    bounded_audio_len(path).is_some()
}

fn bounded_audio_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .filter(|meta| meta.is_file() && meta.len() > 0 && meta.len() <= MAX_AUDIO_BYTES)
        .map(|meta| meta.len())
}

pub(crate) fn duration_seconds(path: &Path) -> Option<f64> {
    let info = track_info(path)?;
    Some(info.frames as f64 / f64::from(info.sample_rate))
}

pub(crate) fn load_track(path: &Path, offset: f64, device_rate: u32) -> Option<LoadedTrack> {
    if device_rate == 0 {
        return None;
    }
    let (info, mut raw) = read_track(path)?;
    let skip_frames = ((offset.max(0.0) * f64::from(info.sample_rate)) as usize)
        .min(info.frames.saturating_sub(1));
    let skip_samples = skip_frames.checked_mul(info.channels)?;
    if skip_samples > 0 {
        raw.drain(..skip_samples);
    }
    let remaining_frames = raw.len() / info.channels;
    let stereo = into_stereo(raw, info.channels)?;
    let remaining = remaining_frames as f64 / f64::from(info.sample_rate);
    Some(LoadedTrack {
        stereo: Arc::new(stereo),
        sample_rate: info.sample_rate,
        remaining: Duration::from_secs_f64(remaining.max(1.0 / f64::from(info.sample_rate))),
    })
}

fn into_stereo(mut raw: Vec<f32>, channels: usize) -> Option<Vec<f32>> {
    if channels == 2 {
        return Some(raw);
    }
    if channels != 1 {
        return None;
    }
    let frames = raw.len();
    let stereo_len = frames.checked_mul(2)?;
    let stereo_bytes = stereo_len.checked_mul(std::mem::size_of::<f32>())?;
    if stereo_bytes > MAX_DECODED_STEREO_BYTES {
        return None;
    }
    raw.try_reserve_exact(frames).ok()?;
    raw.resize(stereo_len, 0.0);
    for frame in (0..frames).rev() {
        let sample = raw[frame];
        raw[frame * 2] = sample;
        raw[frame * 2 + 1] = sample;
    }
    Some(raw)
}

fn playable_info(path: &Path, budget: &mut DiscoveryBudget) -> Option<TrackInfo> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("wav") => {
            let reader = open_bounded_wav_reader(path)?;
            track_info_from_reader(&reader)
        }
        Some("mp3") => mp3_track_info(path).or_else(|| {
            budget
                .claim_fallback_decode()
                .then(|| decode_mp3(path).map(|(info, _)| info))
                .flatten()
        }),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrackInfo {
    sample_rate: u32,
    channels: usize,
    frames: usize,
    samples: usize,
}

impl TrackInfo {
    fn new(sample_rate: u32, channels: usize, frames: usize) -> Option<Self> {
        if sample_rate == 0 || !(1..=2).contains(&channels) || frames < 2 {
            return None;
        }
        let samples = frames.checked_mul(channels)?;
        let stereo_bytes = frames
            .checked_mul(2)?
            .checked_mul(std::mem::size_of::<f32>())?;
        let max_frames =
            usize::try_from(u64::from(sample_rate).checked_mul(MAX_OUTPUT_SECONDS as u64)?).ok()?;
        if frames > max_frames
            || samples > MAX_SOURCE_SAMPLES
            || stereo_bytes > MAX_DECODED_STEREO_BYTES
        {
            return None;
        }
        Some(Self {
            sample_rate,
            channels,
            frames,
            samples,
        })
    }

    fn max_source_samples(sample_rate: u32, channels: usize) -> Option<usize> {
        let max_frames_by_time =
            usize::try_from(u64::from(sample_rate).checked_mul(MAX_OUTPUT_SECONDS as u64)?).ok()?;
        let max_frames_by_stereo =
            MAX_DECODED_STEREO_BYTES.checked_div(2 * std::mem::size_of::<f32>())?;
        max_frames_by_time
            .min(max_frames_by_stereo)
            .checked_mul(channels)
            .map(|samples| samples.min(MAX_SOURCE_SAMPLES))
    }
}

fn track_info(path: &Path) -> Option<TrackInfo> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("wav") => {
            let reader = open_bounded_wav_reader(path)?;
            track_info_from_reader(&reader)
        }
        Some("mp3") => mp3_track_info(path).or_else(|| decode_mp3(path).map(|(info, _)| info)),
        _ => None,
    }
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
    TrackInfo::new(spec.sample_rate, channels, frames)
}

fn read_track(path: &Path) -> Option<(TrackInfo, Vec<f32>)> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("wav") => {
            let info = track_info(path)?;
            let raw = read_wav_samples(path, &info)?;
            Some((info, raw))
        }
        Some("mp3") => decode_mp3(path),
        _ => None,
    }
}

fn read_wav_samples(path: &Path, info: &TrackInfo) -> Option<Vec<f32>> {
    let mut reader = open_bounded_wav_reader(path)?;
    if track_info_from_reader(&reader).as_ref() != Some(info) {
        return None;
    }
    let mut raw = Vec::new();
    raw.try_reserve_exact(info.samples).ok()?;
    for sample in reader.samples::<i16>() {
        raw.push(f32::from(sample.ok()?) / 32_768.0);
    }
    if raw.len() != info.samples || raw.len() % info.channels != 0 {
        return None;
    }
    Some(raw)
}

fn open_bounded_wav_reader(path: &Path) -> Option<hound::WavReader<BufReader<std::fs::File>>> {
    let file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_AUDIO_BYTES {
        return None;
    }
    hound::WavReader::new(BufReader::new(file)).ok()
}

fn mp3_track_info(path: &Path) -> Option<TrackInfo> {
    let (format, track_id) = open_symphonia(path)?;
    let track = format.tracks().iter().find(|track| track.id == track_id)?;
    let audio = track.codec_params.as_ref()?.audio()?;
    let sample_rate = audio.sample_rate?;
    let channels = audio.channels.as_ref()?.count();
    let frames = usize::try_from(track.num_frames?).ok()?;
    TrackInfo::new(sample_rate, channels, frames)
}

fn decode_mp3(path: &Path) -> Option<(TrackInfo, Vec<f32>)> {
    let (mut format, track_id) = open_symphonia(path)?;
    let audio = format
        .tracks()
        .iter()
        .find(|track| track.id == track_id)?
        .codec_params
        .as_ref()?
        .audio()?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio, &AudioDecoderOptions::default())
        .ok()?;
    let mut raw = Vec::new();
    let mut sample_rate = None;
    let mut channels = None;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(_) => return None,
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => return None,
        };
        let spec = decoded.spec();
        let packet_channels = spec.channels().count();
        if !(1..=2).contains(&packet_channels)
            || sample_rate.is_some_and(|rate| rate != spec.rate())
            || channels.is_some_and(|count| count != packet_channels)
        {
            return None;
        }
        sample_rate = Some(spec.rate());
        channels = Some(packet_channels);
        let packet_samples = decoded.samples_interleaved();
        let max_samples = TrackInfo::max_source_samples(spec.rate(), packet_channels)?;
        let packet_range = reserve_decode_packet(&mut raw, packet_samples, max_samples)?;
        decoded.copy_to_slice_interleaved(&mut raw[packet_range]);
    }
    let sample_rate = sample_rate?;
    let channels = channels?;
    if raw.len() % channels != 0 {
        return None;
    }
    let frames = raw.len() / channels;
    if frames < 2 {
        return None;
    }
    Some((TrackInfo::new(sample_rate, channels, frames)?, raw))
}

fn reserve_decode_packet(
    raw: &mut Vec<f32>,
    packet_samples: usize,
    max_samples: usize,
) -> Option<std::ops::Range<usize>> {
    let start = raw.len();
    let next_len = start.checked_add(packet_samples)?;
    if next_len > max_samples {
        return None;
    }
    raw.try_reserve(packet_samples).ok()?;
    raw.resize(next_len, f32::MID);
    Some(start..next_len)
}

fn open_symphonia(path: &Path) -> Option<(Box<dyn symphonia::core::formats::FormatReader>, u32)> {
    let source = BoundedMediaSource::open(path)?;
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()?;
    let track_id = format.default_track(TrackType::Audio)?.id;
    Some((format, track_id))
}

struct BoundedMediaSource {
    file: std::fs::File,
    len: u64,
}

impl BoundedMediaSource {
    fn open(path: &Path) -> Option<Self> {
        let file = std::fs::File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        (metadata.is_file() && metadata.len() <= MAX_AUDIO_BYTES).then_some(Self {
            file,
            len: metadata.len(),
        })
    }
}

impl std::io::Read for BoundedMediaSource {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let position = std::io::Seek::stream_position(&mut self.file)?;
        let remaining = self.len.saturating_sub(position);
        let readable = buffer
            .len()
            .min(usize::try_from(remaining).unwrap_or(usize::MAX));
        std::io::Read::read(&mut self.file, &mut buffer[..readable])
    }
}

impl std::io::Seek for BoundedMediaSource {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        let (base, offset) = match position {
            std::io::SeekFrom::Start(position) => {
                return std::io::Seek::seek(&mut self.file, std::io::SeekFrom::Start(position));
            }
            std::io::SeekFrom::Current(offset) => {
                (std::io::Seek::stream_position(&mut self.file)?, offset)
            }
            std::io::SeekFrom::End(offset) => (self.len, offset),
        };
        let target = i128::from(base) + i128::from(offset);
        let target = u64::try_from(target).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid media seek")
        })?;
        std::io::Seek::seek(&mut self.file, std::io::SeekFrom::Start(target))
    }
}

impl MediaSource for BoundedMediaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_radio_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("radio")
    }

    #[test]
    fn clean_clone_default_finds_the_bundled_soundtrack() {
        if std::env::var_os("NUMINOUS_RADIO").is_some() {
            return;
        }
        let actual = default_dir().canonicalize().expect("default radio dir");
        let expected = bundled_radio_dir()
            .canonicalize()
            .expect("bundled radio dir");
        assert_eq!(actual, expected);
    }

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

    fn write_rate_wav(path: &Path, sample_rate: u32, frames: u32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("write wav");
        for _ in 0..frames {
            writer.write_sample(0i16).expect("sample");
        }
        writer.finalize().expect("finalize");
    }

    #[test]
    fn discovery_budget_bounds_entries_metadata_bytes_and_fallbacks() {
        let limits = DiscoveryLimits {
            entries: 2,
            metadata_probes: 1,
            audio_bytes: 5,
            fallback_decodes: 1,
        };
        let mut budget = DiscoveryBudget::new(limits);

        assert!(budget.claim_entry());
        assert!(budget.claim_entry());
        assert!(!budget.claim_entry());
        assert!(budget.claim_metadata_probe());
        assert!(!budget.claim_metadata_probe());
        assert!(budget.claim_audio_bytes(5));
        assert!(!budget.claim_audio_bytes(1));
        assert!(budget.claim_fallback_decode());
        assert!(!budget.claim_fallback_decode());
    }

    #[test]
    fn station_discovery_stops_at_aggregate_work_limits() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_scan_budget");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        for i in 0..3 {
            write_rate_wav(&dir.join(format!("trance-{i}.wav")), 8_000, 2);
        }
        let limits = DiscoveryLimits {
            entries: 3,
            metadata_probes: 1,
            audio_bytes: MAX_AUDIO_BYTES,
            fallback_decodes: 0,
        };

        let tracks = station_tracks_with_limits(&dir, "trance", limits);

        assert_eq!(tracks.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
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
    fn bundled_mp3_soundtrack_is_complete_and_playable() {
        let dir = bundled_radio_dir();
        let mut total = 0;
        for station in ["trance", "chill", "arcade"] {
            let tracks = station_tracks(&dir, station);
            assert_eq!(tracks.len(), 14, "{station} track count");
            assert!(
                tracks
                    .iter()
                    .all(|path| path.extension().is_some_and(|ext| ext == "mp3"))
            );
            for path in &tracks {
                assert!(
                    duration_seconds(path).is_some_and(|seconds| seconds >= 10.0),
                    "{} has a valid duration",
                    path.display()
                );
            }
            let first = &tracks[0];
            let loaded = load_track(first, 5.0, 44_100).expect("decode bundled MP3");
            assert!(loaded.stereo.iter().any(|sample| sample.abs() > 0.01));
            assert!(loaded.remaining.as_secs_f64() >= 5.0);
            total += tracks.len();
        }
        assert_eq!(total, 42);
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
    fn load_track_keeps_source_rate_stereo_and_rotates_into_the_broadcast() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_load");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("mono.wav");
        write_wav(&path, 1, 3);

        let loaded = load_track(&path, 1.0, 48_000).expect("load");

        assert!(loaded.stereo.len() > 44_100 * 2);
        assert_eq!(loaded.sample_rate, 44_100);
        assert!(loaded.stereo.iter().any(|sample| sample.abs() > 0.1));
        assert!(loaded.remaining.as_secs_f64() >= 1.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_track_does_not_amplify_storage_for_high_rate_devices() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_high_rate");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("mono.wav");
        write_wav(&path, 1, 3);

        let loaded = load_track(&path, 0.0, 96_000).expect("load at high device rate");

        assert_eq!(loaded.sample_rate, 44_100);
        assert_eq!(loaded.stereo.len(), 44_100 * 3 * 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn low_rate_tracks_remain_small_at_high_device_rates() {
        let dir = std::env::temp_dir().join("numinous_radio_cache_output_budget");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("low-rate.wav");
        write_rate_wav(&path, 1, MAX_OUTPUT_SECONDS as u32);

        let complete = load_track(&path, 0.0, 192_000).expect("bounded source-rate track");
        assert_eq!(complete.sample_rate, 1);
        assert_eq!(complete.stereo.len(), MAX_OUTPUT_SECONDS * 2);
        let suffix = load_track(&path, 479.0, 48_000).expect("bounded suffix");
        assert_eq!(suffix.stereo.len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn normal_rate_long_tracks_fit_the_decoded_budget() {
        let frames = 44_100 * 6 * 60;
        let info = TrackInfo::new(44_100, 2, frames).expect("six-minute stereo track");

        assert_eq!(info.frames, frames);
        assert_eq!(info.samples, frames * 2);
        assert!(info.samples * std::mem::size_of::<f32>() <= MAX_DECODED_STEREO_BYTES);
    }

    #[test]
    fn packetized_decode_growth_is_amortized_and_bounded() {
        let packet_samples = 1_152;
        let packet_count = 512;
        let max_samples = packet_samples * packet_count;
        let mut raw = Vec::new();
        let mut capacity_changes = 0;

        for _ in 0..packet_count {
            let before = raw.capacity();
            let range = reserve_decode_packet(&mut raw, packet_samples, max_samples)
                .expect("packet within budget");
            raw[range].fill(0.25);
            capacity_changes += usize::from(raw.capacity() != before);
        }

        assert_eq!(raw.len(), max_samples);
        assert!(
            capacity_changes <= 16,
            "capacity changed {capacity_changes} times"
        );
        assert!(reserve_decode_packet(&mut raw, 1, max_samples).is_none());
        assert_eq!(raw.len(), max_samples);
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

        assert!(audio_is_bounded(&path));
        assert!(duration_seconds(&path).is_none());
        assert!(station_tracks(&dir, "trance").is_empty());
        assert!(load_track(&path, 0.0, 44_100).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn oversized_files_are_rejected_before_decode() {
        let path = std::env::temp_dir().join("numinous_radio_cache_oversized.wav");
        let file = std::fs::File::create(&path).expect("oversized placeholder");
        file.set_len(MAX_AUDIO_BYTES + 1).expect("make sparse file");

        assert!(!audio_is_bounded(&path));
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
        file.set_len(MAX_AUDIO_BYTES + 1)
            .expect("inflate after metadata pass");

        assert!(read_wav_samples(&path, &info).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn symphonia_source_enforces_the_opened_file_length_for_its_lifetime() {
        let path = std::env::temp_dir().join("numinous_radio_cache_swapped_bounds.mp3");
        std::fs::write(&path, b"bounded placeholder").expect("write placeholder");
        assert!(audio_is_bounded(&path));
        let mut source = BoundedMediaSource::open(&path).expect("open bounded source");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("reopen media");
        file.set_len(MAX_AUDIO_BYTES + 1)
            .expect("inflate after metadata pass");

        assert_eq!(
            std::io::Seek::seek(&mut source, std::io::SeekFrom::End(0))
                .expect("seek to bounded end"),
            19
        );
        std::io::Seek::seek(&mut source, std::io::SeekFrom::Start(0)).expect("rewind source");
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut source, &mut bytes).expect("read bounded source");
        assert_eq!(bytes, b"bounded placeholder");
        assert!(open_symphonia(&path).is_none());
        let _ = std::fs::remove_file(path);
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

        assert!(read_wav_samples(&path, &info).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
