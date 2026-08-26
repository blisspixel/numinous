use std::path::PathBuf;

use serde::Serialize;

const DEFAULT_WIDTH: u32 = 900;
const DEFAULT_HEIGHT: u32 = 700;
const DEFAULT_WARMUPS: usize = 3;
const DEFAULT_SAMPLES: usize = 12;
const MAX_SAMPLES: usize = 1_000;
const PHYSICAL_MIN_WARMUPS: usize = 30;
const PHYSICAL_MIN_SAMPLES: usize = 120;
const PHYSICAL_1080P: (u32, u32, f64) = (1920, 1080, 33.0);
const PHYSICAL_1440P: (u32, u32, f64) = (2560, 1440, 50.0);

pub(crate) const HELP: &str = "Numinous Sensory Lift App platform probe
\
\n\
Usage: sensory_platform [options]\n\
\n\
Options:\n\
  --out PATH          Exclusive JSON receipt path, default sensory-app-platform.json\n\
  --width N           Requested frame width, default 900\n\
  --height N          Requested frame height, default 700\n\
  --warmups N         Warmup presentations, default 3\n\
  --samples N         Retained presentations, default 12\n\
  --check             Exit unsuccessfully when the receipt verdict fails\n\
  --physical          Require the physical reference pacing contract\n\
  --budget-ms N       Required p95 boundary budget for --physical\n\
  --machine TEXT      Required physical reference machine description\n\
  --os-version TEXT   Required physical reference OS version\n\
  --power-state ac    Required physical reference power state\n\
  --revision SHA      Source revision, otherwise GITHUB_SHA when available\n\
  --help              Show this help\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EvidenceClass {
    PortableRuntimeCorrectness,
    PhysicalReferencePacing,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Config {
    pub(crate) output: PathBuf,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) warmups: usize,
    pub(crate) samples: usize,
    pub(crate) check: bool,
    pub(crate) evidence_class: EvidenceClass,
    pub(crate) budget_ms: Option<f64>,
    pub(crate) machine: Option<String>,
    pub(crate) os_version: Option<String>,
    pub(crate) power_state: Option<String>,
    pub(crate) revision: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: PathBuf::from("sensory-app-platform.json"),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            warmups: DEFAULT_WARMUPS,
            samples: DEFAULT_SAMPLES,
            check: false,
            evidence_class: EvidenceClass::PortableRuntimeCorrectness,
            budget_ms: None,
            machine: None,
            os_version: None,
            power_state: None,
            revision: std::env::var("GITHUB_SHA").ok(),
        }
    }
}

pub(crate) fn parse_args<I>(args: I) -> Result<Option<Config>, String>
where
    I: IntoIterator<Item = String>,
{
    let mut config = Config::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => config.output = PathBuf::from(required("--out", args.next())?),
            "--width" => config.width = parse_u32("--width", args.next())?,
            "--height" => config.height = parse_u32("--height", args.next())?,
            "--warmups" => config.warmups = parse_usize("--warmups", args.next())?,
            "--samples" => config.samples = parse_usize("--samples", args.next())?,
            "--check" => config.check = true,
            "--physical" => config.evidence_class = EvidenceClass::PhysicalReferencePacing,
            "--budget-ms" => config.budget_ms = Some(parse_f64("--budget-ms", args.next())?),
            "--machine" => config.machine = Some(required("--machine", args.next())?),
            "--os-version" => config.os_version = Some(required("--os-version", args.next())?),
            "--power-state" => config.power_state = Some(required("--power-state", args.next())?),
            "--revision" => config.revision = Some(required("--revision", args.next())?),
            unknown => return Err(format!("unknown option: {unknown}")),
        }
    }
    Ok(Some(config))
}

pub(crate) fn validate_config(
    config: &Config,
    release_build: bool,
    github_actions: bool,
) -> Result<(), String> {
    for (name, dimension) in [("--width", config.width), ("--height", config.height)] {
        if !(64..=numinous_gpu::MAX_FRAME_DIMENSION).contains(&dimension) {
            return Err(format!(
                "{name} must be between 64 and {}, got {dimension}",
                numinous_gpu::MAX_FRAME_DIMENSION
            ));
        }
    }
    if !(1..=MAX_SAMPLES).contains(&config.samples) {
        return Err(format!(
            "--samples must be between 1 and {MAX_SAMPLES}, got {}",
            config.samples
        ));
    }
    if config.warmups > MAX_SAMPLES {
        return Err(format!(
            "--warmups must be at most {MAX_SAMPLES}, got {}",
            config.warmups
        ));
    }
    if config.output.as_os_str().is_empty() {
        return Err("--out must not be empty".to_owned());
    }
    if config.evidence_class == EvidenceClass::PortableRuntimeCorrectness {
        if config.budget_ms.is_some()
            || config.machine.is_some()
            || config.os_version.is_some()
            || config.power_state.is_some()
        {
            return Err("physical metadata and a pacing budget require --physical".to_owned());
        }
        return Ok(());
    }
    validate_physical(config, release_build, github_actions)
}

fn validate_physical(
    config: &Config,
    release_build: bool,
    github_actions: bool,
) -> Result<(), String> {
    if !release_build {
        return Err("--physical requires a release build".to_owned());
    }
    if github_actions {
        return Err("--physical refuses GitHub Actions timing as physical evidence".to_owned());
    }
    if config.warmups < PHYSICAL_MIN_WARMUPS || config.samples < PHYSICAL_MIN_SAMPLES {
        return Err(format!(
            "--physical requires at least {PHYSICAL_MIN_WARMUPS} warmups and {PHYSICAL_MIN_SAMPLES} samples"
        ));
    }
    for (name, value) in [
        ("--machine", config.machine.as_deref()),
        ("--os-version", config.os_version.as_deref()),
    ] {
        validate_label(
            name,
            value.ok_or_else(|| format!("--physical requires {name}"))?,
        )?;
    }
    if config.power_state.as_deref() != Some("ac") {
        return Err("--physical requires --power-state ac".to_owned());
    }
    let revision = config
        .revision
        .as_deref()
        .ok_or("--physical requires --revision")?;
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--revision must be a full 40-character hexadecimal commit".to_owned());
    }
    validate_physical_target(config)
}

fn validate_physical_target(config: &Config) -> Result<(), String> {
    let budget = config.budget_ms.ok_or("--physical requires --budget-ms")?;
    let expected = match (config.width, config.height) {
        (width, height) if (width, height) == (PHYSICAL_1080P.0, PHYSICAL_1080P.1) => {
            PHYSICAL_1080P.2
        }
        (width, height) if (width, height) == (PHYSICAL_1440P.0, PHYSICAL_1440P.1) => {
            PHYSICAL_1440P.2
        }
        _ => {
            return Err(
                "--physical accepts only 1920x1080 or 2560x1440 reference frames".to_owned(),
            );
        }
    };
    if budget != expected {
        return Err(format!(
            "--physical requires the declared {expected:.0} ms budget for {}x{}",
            config.width, config.height
        ));
    }
    Ok(())
}

fn validate_label(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(format!("{name} must be 1 to 256 non-control characters"));
    }
    Ok(())
}

fn required(name: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("{name} requires a value"))
}

fn parse_u32(name: &str, value: Option<String>) -> Result<u32, String> {
    let value = required(name, value)?;
    value
        .parse()
        .map_err(|_| format!("{name} requires an unsigned integer, got {value}"))
}

fn parse_usize(name: &str, value: Option<String>) -> Result<usize, String> {
    let value = required(name, value)?;
    value
        .parse()
        .map_err(|_| format!("{name} requires an unsigned integer, got {value}"))
}

fn parse_f64(name: &str, value: Option<String>) -> Result<f64, String> {
    let value = required(name, value)?;
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a number, got {value}"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{name} must be finite and positive, got {value}"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{Config, EvidenceClass, parse_args, validate_config};

    #[test]
    fn portable_arguments_stay_bounded_and_nonauthoritative() {
        let config = parse_args([
            "--samples".to_owned(),
            "8".to_owned(),
            "--warmups".to_owned(),
            "0".to_owned(),
            "--check".to_owned(),
        ])
        .expect("parse portable arguments")
        .expect("run action");
        assert_eq!(config.samples, 8);
        assert_eq!(config.warmups, 0);
        assert!(config.check);
        assert_eq!(
            config.evidence_class,
            EvidenceClass::PortableRuntimeCorrectness
        );
        validate_config(&config, false, true).expect("portable CI contract");

        let mut invalid = config;
        invalid.budget_ms = Some(33.0);
        assert!(validate_config(&invalid, false, true).is_err());
    }

    #[test]
    fn physical_pacing_requires_release_hardware_metadata_and_declared_targets() {
        let config = Config {
            evidence_class: EvidenceClass::PhysicalReferencePacing,
            width: 1920,
            height: 1080,
            warmups: 30,
            samples: 120,
            budget_ms: Some(33.0),
            machine: Some("reference laptop".to_owned()),
            os_version: Some("reference OS".to_owned()),
            power_state: Some("ac".to_owned()),
            revision: Some("a".repeat(40)),
            ..Config::default()
        };
        assert!(validate_config(&config, false, false).is_err());
        assert!(validate_config(&config, true, true).is_err());
        validate_config(&config, true, false).expect("physical reference contract");

        let mut wrong_budget = config.clone();
        wrong_budget.budget_ms = Some(50.0);
        assert!(validate_config(&wrong_budget, true, false).is_err());
        let mut wrong_size = config;
        wrong_size.width = 1280;
        wrong_size.height = 720;
        assert!(validate_config(&wrong_size, true, false).is_err());
    }
}
