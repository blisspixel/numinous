//! Measures the feature-gated Sensory Lift post-processing spike.

use numinous_gpu::{PostFrame, SensoryPostRenderer};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "post_spike/cpu.rs"]
mod cpu;

use cpu::CpuPostRenderer;

const DEFAULT_WARMUPS: usize = 3;
const DEFAULT_SAMPLES: usize = 20;

struct Arguments {
    warmups: usize,
    samples: usize,
    check: bool,
    preview_dir: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt<'a> {
    schema: &'a str,
    schema_version: u32,
    boundary: &'a str,
    adapter: &'a str,
    backend: &'a str,
    timestamp_queries: bool,
    warmups: usize,
    samples: usize,
    workloads: Vec<Workload>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Workload {
    width: u32,
    height: u32,
    source_checksum: String,
    gpu_output_checksum: String,
    cpu_output_checksum: String,
    wall_ms: Samples,
    gpu_ms: Option<Samples>,
    cpu_ms: Samples,
    device_budget_ms: f64,
    cpu_budget_ms: f64,
    validation_boundary_budget_ms: f64,
    gpu_verdict: &'static str,
    cpu_verdict: &'static str,
}

#[derive(Serialize)]
struct Samples {
    raw: Vec<f64>,
    p50: f64,
    p95: f64,
    maximum: f64,
}

impl Samples {
    fn from_raw(raw: Vec<f64>) -> Self {
        let mut ordered = raw.clone();
        ordered.sort_by(f64::total_cmp);
        Self {
            p50: percentile(&ordered, 0.50),
            p95: percentile(&ordered, 0.95),
            maximum: ordered.last().copied().unwrap_or_default(),
            raw,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = arguments()?;
    let warmups = arguments.warmups;
    let samples = arguments.samples;
    let mut renderer = SensoryPostRenderer::new()?;
    let capabilities = renderer.capabilities();
    let adapter = renderer.adapter_name().to_owned();
    let backend = renderer.backend().to_owned();
    let mut workloads = Vec::new();
    for (width, height, device_budget_ms, validation_budget_ms) in
        [(1920, 1080, 8.0, 33.0), (2560, 1440, 12.0, 50.0)]
    {
        workloads.push(measure(
            &mut renderer,
            width,
            height,
            warmups,
            samples,
            device_budget_ms,
            validation_budget_ms,
            arguments.preview_dir.as_deref(),
        )?);
    }
    let failed = workloads
        .iter()
        .any(|workload| workload.gpu_verdict != "pass");
    let receipt = Receipt {
        schema: "numinous.sensory-post-spike",
        schema_version: 2,
        boundary: "the GPU wall boundary spans prebuilt sRGB host frame upload, five render passes, final texture copy, map, and tight host RGBA output; device timing spans only those passes; CPU timing spans equivalent reusable single-threaded f32 buffers through tight host RGBA output",
        adapter: &adapter,
        backend: &backend,
        timestamp_queries: capabilities.timestamp_queries,
        warmups,
        samples,
        workloads,
    };
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    if arguments.check && failed {
        return Err("one or more Sensory Lift GPU budgets failed".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn measure(
    renderer: &mut SensoryPostRenderer,
    width: u32,
    height: u32,
    warmups: usize,
    samples: usize,
    device_budget_ms: f64,
    validation_boundary_budget_ms: f64,
    preview_dir: Option<&Path>,
) -> Result<Workload, Box<dyn std::error::Error>> {
    let source = source_frame(width, height);
    let mut cpu_renderer = CpuPostRenderer::new();
    for _ in 0..warmups {
        renderer.process_rgba(width, height, &source)?;
        cpu_renderer.process_rgba(width, height, &source)?;
    }
    let mut wall_raw = Vec::with_capacity(samples);
    let mut gpu_raw = Vec::with_capacity(samples);
    let mut cpu_raw = Vec::with_capacity(samples);
    let mut output_checksum = None;
    let mut cpu_output_checksum = None;
    for sample_index in 0..samples {
        let started = Instant::now();
        let frame = renderer.process_rgba(width, height, &source)?;
        wall_raw.push(round_ms(started.elapsed().as_secs_f64() * 1_000.0));
        if let Some(gpu_time_ms) = frame.gpu_time_ms {
            gpu_raw.push(round_ms(gpu_time_ms));
        }
        let gpu_checksum = checksum(&frame.rgba);
        match output_checksum {
            Some(expected) if expected != gpu_checksum => {
                return Err("post-processing output changed between identical frames".into());
            }
            None => output_checksum = Some(gpu_checksum),
            _ => {}
        }
        validate_output(&frame, width, height)?;
        if sample_index == 0
            && let Some(directory) = preview_dir
        {
            write_png(
                directory,
                &format!("gpu-{width}x{height}.png"),
                width,
                height,
                &frame.rgba,
            )?;
        }

        let started = Instant::now();
        let cpu_frame = cpu_renderer.process_rgba(width, height, &source)?;
        cpu_raw.push(round_ms(started.elapsed().as_secs_f64() * 1_000.0));
        let cpu_checksum = checksum(cpu_frame);
        match cpu_output_checksum {
            Some(expected) if expected != cpu_checksum => {
                return Err("CPU post output changed between identical frames".into());
            }
            None => cpu_output_checksum = Some(cpu_checksum),
            _ => {}
        }
        if sample_index == 0
            && let Some(directory) = preview_dir
        {
            write_png(
                directory,
                &format!("source-{width}x{height}.png"),
                width,
                height,
                &source,
            )?;
            write_png(
                directory,
                &format!("cpu-{width}x{height}.png"),
                width,
                height,
                cpu_frame,
            )?;
        }
    }
    let wall_ms = Samples::from_raw(wall_raw);
    let gpu_ms = (!gpu_raw.is_empty()).then(|| Samples::from_raw(gpu_raw));
    let cpu_ms = Samples::from_raw(cpu_raw);
    let device_pass = gpu_ms
        .as_ref()
        .is_none_or(|summary| summary.p95 <= device_budget_ms);
    let wall_pass = wall_ms.p95 <= validation_boundary_budget_ms;
    let cpu_pass = cpu_ms.p95 <= validation_boundary_budget_ms;
    Ok(Workload {
        width,
        height,
        source_checksum: format!("fnv1a64:{:016x}", checksum(&source)),
        gpu_output_checksum: format!("fnv1a64:{:016x}", output_checksum.unwrap_or_default()),
        cpu_output_checksum: format!("fnv1a64:{:016x}", cpu_output_checksum.unwrap_or_default()),
        wall_ms,
        gpu_ms,
        cpu_ms,
        device_budget_ms,
        cpu_budget_ms: validation_boundary_budget_ms,
        validation_boundary_budget_ms,
        gpu_verdict: if device_pass && wall_pass {
            "pass"
        } else {
            "fail"
        },
        cpu_verdict: if cpu_pass { "pass" } else { "fail" },
    })
}

fn source_frame(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = vec![0; width as usize * height as usize * 4];
    let width_scale = 1.0 / width.max(1) as f32;
    let height_scale = 1.0 / height.max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let normalized_x = x as f32 * width_scale;
            let normalized_y = y as f32 * height_scale;
            let dx = normalized_x - 0.52;
            let dy = normalized_y - 0.46;
            let glow = (-42.0 * (dx * dx + dy * dy)).exp();
            let index = (y as usize * width as usize + x as usize) * 4;
            rgba[index] = ((0.05 + normalized_x * 0.35 + glow * 0.85).min(1.0) * 255.0) as u8;
            rgba[index + 1] = ((0.03 + normalized_y * 0.22 + glow * 0.70).min(1.0) * 255.0) as u8;
            rgba[index + 2] = ((0.12 + (1.0 - normalized_x) * 0.26 + glow).min(1.0) * 255.0) as u8;
            rgba[index + 3] = 255;
            let vertical_beam =
                x.abs_diff(width * 3 / 8) <= 1 && (height / 5..height * 4 / 5).contains(&y);
            let horizontal_beam =
                y.abs_diff(height * 2 / 3) <= 1 && (width / 7..width * 5 / 7).contains(&x);
            if vertical_beam || horizontal_beam {
                rgba[index] = 255;
                rgba[index + 1] = 232;
                rgba[index + 2] = 176;
            }
        }
    }
    rgba
}

fn validate_output(
    frame: &PostFrame,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = width as usize * height as usize * 4;
    if frame.rgba.len() != expected {
        return Err(format!(
            "post-processing output has {} bytes, expected {expected}",
            frame.rgba.len()
        )
        .into());
    }
    if frame.rgba.iter().all(|byte| *byte == 0) {
        return Err("post-processing output is blank".into());
    }
    Ok(())
}

fn percentile(ordered: &[f64], quantile: f64) -> f64 {
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered[rank.saturating_sub(1).min(ordered.len().saturating_sub(1))]
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn round_ms(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn arguments() -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut warmups = DEFAULT_WARMUPS;
    let mut samples = DEFAULT_SAMPLES;
    let mut check = false;
    let mut preview_dir = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--warmups" => warmups = positive_count(args.next(), "--warmups")?,
            "--samples" => samples = positive_count(args.next(), "--samples")?,
            "--check" => check = true,
            "--preview-dir" => {
                preview_dir = Some(PathBuf::from(
                    args.next().ok_or("--preview-dir requires a path")?,
                ));
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    Ok(Arguments {
        warmups,
        samples,
        check,
        preview_dir,
    })
}

fn write_png(
    directory: &Path,
    name: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let file = std::fs::File::create(directory.join(name))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

fn positive_count(
    value: Option<String>,
    option: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let value = value.ok_or_else(|| format!("{option} requires a value"))?;
    let count = value.parse::<usize>()?;
    if count == 0 {
        return Err(format!("{option} must be positive").into());
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{Samples, checksum, percentile, source_frame};

    #[test]
    fn percentile_uses_nearest_rank() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.50), 2.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.95), 4.0);
        let summary = Samples::from_raw(vec![4.0, 1.0, 3.0, 2.0]);
        assert_eq!(summary.p50, 2.0);
        assert_eq!(summary.maximum, 4.0);
    }

    #[test]
    fn source_is_deterministic_and_opaque() {
        let first = source_frame(8, 6);
        let second = source_frame(8, 6);
        assert_eq!(checksum(&first), checksum(&second));
        assert!(first.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }
}
