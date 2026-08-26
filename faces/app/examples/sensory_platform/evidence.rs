use std::fs::OpenOptions;
use std::io::{Read, Write};

use serde::Serialize;
use sha2::{Digest, Sha256};
use winit::dpi::PhysicalSize;

use super::super::{Probe, presentation};
use super::contract::{Config, EvidenceClass};
use super::source::{SourceReceipt, hex_digest};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterReceipt {
    name: String,
    vendor_id: u32,
    device_id: u32,
    device_type: String,
    driver: String,
    driver_info: String,
    backend: String,
    physical_gpu: bool,
}

impl AdapterReceipt {
    pub(crate) fn from_info(info: &presentation::GpuPresentationInfo<'_>) -> Self {
        let device_type = info.adapter.device_type().to_owned();
        Self {
            name: info.adapter.name().to_owned(),
            vendor_id: info.adapter.vendor(),
            device_id: info.adapter.device(),
            device_type: device_type.clone(),
            driver: info.adapter.driver().to_owned(),
            driver_info: info.adapter.driver_info().to_owned(),
            backend: info.adapter.backend().to_owned(),
            physical_gpu: matches!(device_type.as_str(), "IntegratedGpu" | "DiscreteGpu"),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SurfaceReceipt {
    requested_width: u32,
    requested_height: u32,
    actual_width: u32,
    actual_height: u32,
    format: String,
    present_mode: String,
    desired_maximum_frame_latency: u32,
}

impl SurfaceReceipt {
    pub(crate) fn from_info(
        info: presentation::GpuPresentationInfo<'_>,
        requested: PhysicalSize<u32>,
        actual: PhysicalSize<u32>,
    ) -> Self {
        Self {
            requested_width: requested.width,
            requested_height: requested.height,
            actual_width: actual.width,
            actual_height: actual.height,
            format: info.surface_format,
            present_mode: info.present_mode,
            desired_maximum_frame_latency: 1,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SampleSummary {
    raw: Vec<f64>,
    p50: Option<f64>,
    p95: Option<f64>,
    maximum: Option<f64>,
}

impl SampleSummary {
    fn new(raw: &[f64]) -> Self {
        let mut ordered = raw.to_vec();
        ordered.sort_by(f64::total_cmp);
        Self {
            raw: raw.to_vec(),
            p50: percentile(&ordered, 0.50),
            p95: percentile(&ordered, 0.95),
            maximum: ordered.last().copied(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildReceipt {
    package_version: &'static str,
    revision: Option<String>,
    profile: &'static str,
    binary_sha256: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformReceipt {
    os: &'static str,
    architecture: &'static str,
    family: &'static str,
    github_actions: bool,
    machine: Option<String>,
    os_version: Option<String>,
    power_state: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceReceipt {
    class: EvidenceClass,
    timing_authority: &'static str,
    correctness_claim: &'static str,
    pacing_claim: &'static str,
    excludes: [&'static str; 4],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    schema: &'static str,
    schema_version: u32,
    evidence: EvidenceReceipt,
    build: BuildReceipt,
    platform: PlatformReceipt,
    adapter: Option<AdapterReceipt>,
    surface: Option<SurfaceReceipt>,
    source: SourceReceipt,
    warmups: usize,
    samples: usize,
    presented_frames: usize,
    skipped_frames: usize,
    suboptimal_frames: usize,
    acquire_ms: SampleSummary,
    render_and_present_ms: SampleSummary,
    boundary_ms: SampleSummary,
    boundary_budget_ms: Option<f64>,
    check_enforced: bool,
    failures: Vec<String>,
    verdict: &'static str,
}

pub(crate) fn write_probe_receipt(
    probe: &Probe,
    binary_sha256: Option<String>,
    github_actions: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let failures = evaluate(probe, binary_sha256.as_deref());
    let passed = failures.is_empty();
    let receipt = build_receipt(probe, binary_sha256, github_actions, failures);
    if let Some(parent) = probe.config.output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe.config.output)?;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    Ok(passed)
}

fn evaluate(probe: &Probe, binary_sha256: Option<&str>) -> Vec<String> {
    let mut failures = Vec::new();
    if let Some(failure) = &probe.failure {
        failures.push(failure.clone());
    }
    if binary_sha256.is_none() {
        failures.push("running executable identity could not be hashed".to_owned());
    }
    if !probe.source.receipt.deterministic {
        failures.push("the repeated App source composition was not byte exact".to_owned());
    }
    if !probe.source.receipt.all_alpha_opaque {
        failures.push("the App source frame contains nonopaque alpha".to_owned());
    }
    if probe.source.receipt.lit_pixels < 100 {
        failures.push("the App source frame is effectively blank".to_owned());
    }
    if probe.adapter.is_none() || probe.surface.is_none() {
        failures.push("the production presenter did not expose a direct surface".to_owned());
    }
    if let Some(surface) = &probe.surface {
        if !surface.format.ends_with("Srgb") {
            failures.push(format!("surface format is not sRGB: {}", surface.format));
        }
        if surface.present_mode != "Fifo" {
            failures.push(format!(
                "surface present mode is not FIFO: {}",
                surface.present_mode
            ));
        }
    }
    if probe.warmups_completed != probe.config.warmups {
        failures.push(format!(
            "completed {} of {} warmups",
            probe.warmups_completed, probe.config.warmups
        ));
    }
    validate_samples(probe, &mut failures);
    if probe.config.evidence_class == EvidenceClass::PhysicalReferencePacing {
        validate_physical_result(probe, &mut failures);
    }
    failures
}

fn validate_samples(probe: &Probe, failures: &mut Vec<String>) {
    for (label, values) in [
        ("acquire", &probe.acquire_ms),
        ("render and present", &probe.render_and_present_ms),
        ("boundary", &probe.boundary_ms),
    ] {
        if values.len() != probe.config.samples {
            failures.push(format!(
                "retained {} of {} {label} samples",
                values.len(),
                probe.config.samples
            ));
        }
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            failures.push(format!("{label} samples contain an invalid duration"));
        }
    }
}

fn validate_physical_result(probe: &Probe, failures: &mut Vec<String>) {
    if !probe
        .adapter
        .as_ref()
        .is_some_and(|adapter| adapter.physical_gpu)
    {
        failures.push("physical pacing requires an integrated or discrete GPU".to_owned());
    }
    if probe.skipped_frames != 0 {
        failures.push(format!(
            "physical pacing observed {} skipped frames",
            probe.skipped_frames
        ));
    }
    if probe.suboptimal_frames != 0 {
        failures.push(format!(
            "physical pacing observed {} suboptimal frames",
            probe.suboptimal_frames
        ));
    }
    if let Some(surface) = &probe.surface
        && (surface.actual_width, surface.actual_height)
            != (surface.requested_width, surface.requested_height)
    {
        failures.push(format!(
            "physical pacing surface is {}x{}, requested {}x{}",
            surface.actual_width,
            surface.actual_height,
            surface.requested_width,
            surface.requested_height
        ));
    }
    if let (Some(p95), Some(budget)) = (
        SampleSummary::new(&probe.boundary_ms).p95,
        probe.config.budget_ms,
    ) && p95 > budget
    {
        failures.push(format!(
            "boundary p95 {p95:.6} ms exceeds the {budget:.6} ms budget"
        ));
    }
}

fn build_receipt(
    probe: &Probe,
    binary_sha256: Option<String>,
    github_actions: bool,
    failures: Vec<String>,
) -> Receipt {
    let physical = probe.config.evidence_class == EvidenceClass::PhysicalReferencePacing;
    Receipt {
        schema: "numinous.sensory-app-platform",
        schema_version: 1,
        evidence: EvidenceReceipt {
            class: probe.config.evidence_class,
            timing_authority: if physical {
                "physical-reference-candidate"
            } else {
                "informational-only"
            },
            correctness_claim: "the deterministic fully composed App frame completed through the production direct surface presenter on this runtime",
            pacing_claim: if physical {
                "the recorded acquire-through-present-request samples are a candidate result for this named physical reference only"
            } else {
                "hosted or unclassified runtime timings are retained for diagnosis and cannot promote the feature"
            },
            excludes: [
                "compositor completion",
                "display scanout",
                "input latency",
                "perceptual quality",
            ],
        },
        build: build(probe, binary_sha256),
        platform: platform(&probe.config, github_actions),
        adapter: probe.adapter.clone(),
        surface: probe.surface.clone(),
        source: probe.source.receipt.clone(),
        warmups: probe.config.warmups,
        samples: probe.config.samples,
        presented_frames: probe.presented_frames,
        skipped_frames: probe.skipped_frames,
        suboptimal_frames: probe.suboptimal_frames,
        acquire_ms: SampleSummary::new(&probe.acquire_ms),
        render_and_present_ms: SampleSummary::new(&probe.render_and_present_ms),
        boundary_ms: SampleSummary::new(&probe.boundary_ms),
        boundary_budget_ms: probe.config.budget_ms,
        check_enforced: probe.config.check,
        verdict: if failures.is_empty() { "pass" } else { "fail" },
        failures,
    }
}

fn build(probe: &Probe, binary_sha256: Option<String>) -> BuildReceipt {
    BuildReceipt {
        package_version: env!("CARGO_PKG_VERSION"),
        revision: probe.config.revision.clone(),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        binary_sha256,
    }
}

fn platform(config: &Config, github_actions: bool) -> PlatformReceipt {
    PlatformReceipt {
        os: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        family: std::env::consts::FAMILY,
        github_actions,
        machine: config.machine.clone(),
        os_version: config.os_version.clone(),
        power_state: config.power_state.clone(),
    }
}

fn percentile(ordered: &[f64], quantile: f64) -> Option<f64> {
    if ordered.is_empty() {
        return None;
    }
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered.get(rank.saturating_sub(1)).copied()
}

pub(crate) fn round_ms(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub(crate) fn executable_sha256() -> Result<String, String> {
    let path = std::env::current_exe()
        .map_err(|error| format!("cannot locate running executable: {error}"))?;
    let mut file = std::fs::File::open(&path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex_digest(&digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::SampleSummary;

    #[test]
    fn nearest_rank_summary_handles_empty_and_ordered_samples() {
        let empty = SampleSummary::new(&[]);
        assert_eq!(empty.p95, None);
        let summary = SampleSummary::new(&[4.0, 1.0, 3.0, 2.0]);
        assert_eq!(summary.p50, Some(2.0));
        assert_eq!(summary.p95, Some(4.0));
        assert_eq!(summary.maximum, Some(4.0));
    }
}
