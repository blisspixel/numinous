#!/usr/bin/env python3
"""Measure the exact July 2026 dependency migration on one reference machine."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
import platform
import re
import secrets
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable


SCHEMA_VERSION = "numinous-dependency-migration-performance-v1"
REPOSITORY_URL = "https://github.com/blisspixel/numinous"
BEFORE_REVISION = "b47303d742c795540eb08a9c0e70a7e391a47978"
AFTER_REVISION = "301eac6943fb44ff00316c7b0994e8d8cc505455"
MAX_SAMPLES = 200
MAX_OUTPUT_BYTES = 4 * 1024 * 1024
MAX_TIMING_MS = 60_000.0
EXPECTED_WARMUPS = 3
EXPECTED_SAMPLES = 20
APP_PROBE = "win32-visible-top-level-window-v1"
EXPECTED_GPU_DIMENSIONS = (1200, 900)
SHA256_RE = re.compile(r"[0-9a-f]{64}\Z")
SAFE_TEXT_RE = re.compile(r"[^\x00-\x1f\x7f]{1,256}\Z")
UTC_TIMESTAMP_RE = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\Z")
AUDIO_RECEIPT_RE = re.compile(
    r"audio-ready\t([1-9][0-9]{0,14})\t([1-9][0-9]{0,8})\t([1-9][0-9]{0,3})\t([^\t\r\n]{1,256})\n?\Z"
)

WORKLOADS: dict[str, dict[str, object]] = {
    "cliRequest": {
        "boundary": (
            "fresh process start through a complete deterministic 80 by 30 "
            "Times Tables terminal render captured from stdout"
        ),
        "limit": 1.25,
        "deltaMs": 5.0,
    },
    "gpuPostcard": {
        "boundary": (
            "fresh process start through adapter acquisition, one 1200 by 900 "
            "300-iteration Mandelbrot dispatch, readback, and PNG completion"
        ),
        "limit": 1.25,
        "deltaMs": 50.0,
    },
    "audioDeviceInit": {
        "boundary": (
            "inside one fresh process, from default-host creation through default "
            "output-device and output-configuration acquisition"
        ),
        "limit": 1.50,
        "deltaMs": 10.0,
    },
    "appVisibleWindow": {
        "boundary": (
            "fresh muted App process start through discovery of its visible native "
            "top-level Numinous window, before display scan-out or human perception"
        ),
        "limit": 1.35,
        "deltaMs": 20.0,
    },
}

SIDE_METADATA_FIELDS = {
    "cliRequest": {"outputSha256"},
    "gpuPostcard": {"adapter", "backend", "outputDimensions", "outputSha256"},
    "audioDeviceInit": {"device", "sampleRate", "channels"},
    "appVisibleWindow": set(),
}

REFERENCE_MACHINE = {
    "os": "Windows",
    "osRelease": "10.0.26200",
    "architecture": "AMD64",
    "cpu": "AMD64 Family 25 Model 116 Stepping 1, AuthenticAMD",
    "logicalCpus": 16,
    "memoryBytes": 66_367_377_408,
    "acPower": True,
}

REFERENCE_TOOLCHAINS = {
    "before": {
        "cargoVersion": "cargo 1.96.0 (30a34c682 2026-05-25)",
        "rustcVersion": "rustc 1.96.0 (ac68faa20 2026-05-25)",
    },
    "after": {
        "cargoVersion": "cargo 1.97.1 (c980f4866 2026-06-30)",
        "rustcVersion": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    },
}

REFERENCE_IDENTITIES = {
    "cliRequest": {
        "before": {
            "outputSha256": "a64c231d39e7ec47821034b7e5250b2bedc9bdc47018627323b4e21795777f29",
        },
        "after": {
            "outputSha256": "a64c231d39e7ec47821034b7e5250b2bedc9bdc47018627323b4e21795777f29",
        },
    },
    "gpuPostcard": {
        "before": {
            "adapter": "AMD Radeon(TM) 780M",
            "backend": "Vulkan",
            "outputDimensions": [1200, 900],
            "outputSha256": "9341b0d8fddfb5f2eb243df28b9ccaaa1b1c5eb6788c2f4a9a0c639c4d75cc87",
        },
        "after": {
            "adapter": "AMD Radeon(TM) 780M",
            "backend": "Vulkan",
            "outputDimensions": [1200, 900],
            "outputSha256": "a0880f5b650c92e8733bb6bb6a0527209656292abc5cf4403418a3bd84cab2e1",
        },
    },
    "audioDeviceInit": {
        "before": {
            "device": "Speakers (Realtek(R) Audio)",
            "sampleRate": 48_000,
            "channels": 2,
        },
        "after": {
            "device": "Speakers (Realtek(R) Audio)",
            "sampleRate": 48_000,
            "channels": 2,
        },
    },
    "appVisibleWindow": {"before": {}, "after": {}},
}

AUDIO_PROBE_SOURCE = r'''//! Historical default-output discovery probe.

use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let started = Instant::now();
    let context = match numinous_audio::AudioContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("audio initialization failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let elapsed = started.elapsed().as_nanos();
    let device = context.device_name().replace(['\t', '\r', '\n'], " ");
    println!(
        "audio-ready\t{elapsed}\t{}\t{}\t{device}",
        context.sample_rate(),
        context.channels()
    );
    black_box(context);
    ExitCode::SUCCESS
}
'''


class PerformanceError(RuntimeError):
    """A measurement or evidence contract was not satisfied."""


def canonical_json(value: object) -> bytes:
    """Encode evidence with a stable human-readable JSON representation."""
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode(
        "utf-8"
    )


def parse_json_strict(content: bytes) -> object:
    """Decode UTF-8 JSON while rejecting duplicate keys and nonfinite constants."""
    if len(content) > MAX_OUTPUT_BYTES:
        raise PerformanceError("JSON input exceeds the evidence size bound")
    try:
        text = content.decode("utf-8", "strict")
    except UnicodeDecodeError as error:
        raise PerformanceError("JSON input is not valid UTF-8") from error

    def object_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result = {}
        for key, value in pairs:
            if key in result:
                raise PerformanceError(f"JSON object repeats key {key!r}")
            result[key] = value
        return result

    def reject_constant(value: str) -> object:
        raise PerformanceError(f"JSON contains nonfinite constant {value}")

    try:
        return json.loads(text, object_pairs_hook=object_pairs, parse_constant=reject_constant)
    except json.JSONDecodeError as error:
        raise PerformanceError(f"JSON input is malformed: {error}") from error


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def summarize_ms(samples: list[float]) -> dict[str, float]:
    """Return nearest-rank p50 and p95 plus maximum for positive samples."""
    if not samples or len(samples) > MAX_SAMPLES:
        raise PerformanceError("timing samples must contain between 1 and 200 values")
    if any(not math.isfinite(value) or value <= 0.0 or value > MAX_TIMING_MS for value in samples):
        raise PerformanceError("timing samples must be finite, positive, and at most 60000 ms")
    ordered = sorted(samples)

    def quantile(proportion: float) -> float:
        rank = math.ceil(len(ordered) * proportion)
        return ordered[max(0, rank - 1)]

    return {
        "p50Ms": round(quantile(0.50), 6),
        "p95Ms": round(quantile(0.95), 6),
        "maxMs": round(ordered[-1], 6),
    }


def parse_audio_receipt(output: str) -> dict[str, object]:
    match = AUDIO_RECEIPT_RE.fullmatch(output)
    if match is None:
        raise PerformanceError("audio probe returned a malformed receipt")
    nanoseconds = int(match.group(1))
    sample_rate = int(match.group(2))
    channels = int(match.group(3))
    if nanoseconds > int(MAX_TIMING_MS * 1_000_000):
        raise PerformanceError("audio initialization exceeded the timing bound")
    if not 8_000 <= sample_rate <= 384_000:
        raise PerformanceError("audio probe returned an unsupported sample rate")
    if not 1 <= channels <= 64:
        raise PerformanceError("audio probe returned an unsupported channel count")
    return {
        "durationMs": round(nanoseconds / 1_000_000.0, 6),
        "sampleRate": sample_rate,
        "channels": channels,
        "device": match.group(4),
    }


def parse_gpu_receipt(output: str) -> dict[str, str]:
    matches = []
    for line in output.splitlines():
        if line.startswith("Rendering on: ") and line.endswith(")"):
            payload = line.removeprefix("Rendering on: ")
            separator = payload.rfind(" (")
            if separator > 0:
                matches.append((payload[:separator], payload[separator + 2 : -1]))
    if len(matches) != 1:
        raise PerformanceError("GPU probe must identify exactly one adapter and backend")
    adapter, backend = matches[0]
    if SAFE_TEXT_RE.fullmatch(adapter) is None or SAFE_TEXT_RE.fullmatch(backend) is None:
        raise PerformanceError("GPU probe returned unsafe adapter metadata")
    return {"adapter": adapter, "backend": backend}


def read_png_dimensions(path: Path) -> tuple[int, int]:
    try:
        header = path.read_bytes()[:24]
    except OSError as error:
        raise PerformanceError(f"could not read GPU output: {error}") from error
    if (
        len(header) != 24
        or header[:8] != b"\x89PNG\r\n\x1a\n"
        or header[8:12] != b"\x00\x00\x00\r"
        or header[12:16] != b"IHDR"
    ):
        raise PerformanceError("GPU output is not a canonical PNG with an IHDR first")
    width = int.from_bytes(header[16:20], "big")
    height = int.from_bytes(header[20:24], "big")
    if width <= 0 or height <= 0:
        raise PerformanceError("GPU output has invalid dimensions")
    return width, height


def _expect_exact_keys(value: object, keys: set[str], name: str) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != keys:
        raise PerformanceError(f"{name} must contain exactly {sorted(keys)}")
    return value


def _expect_text(value: object, name: str, maximum: int = 512) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum:
        raise PerformanceError(f"{name} must be a nonempty bounded string")
    if any(ord(character) < 32 or ord(character) == 127 for character in value):
        raise PerformanceError(f"{name} must not contain control characters")
    return value


def _expect_sha256(value: object, name: str) -> str:
    if not isinstance(value, str) or SHA256_RE.fullmatch(value) is None:
        raise PerformanceError(f"{name} must be a lowercase SHA256 digest")
    return value


def _validate_side(
    side: object,
    name: str,
    sample_count: int,
    metadata_fields: set[str],
) -> dict[str, object]:
    side = _expect_exact_keys(
        side,
        {"samplesMs", "stats", "binarySha256"} | metadata_fields,
        name,
    )
    samples = side["samplesMs"]
    if (
        not isinstance(samples, list)
        or len(samples) != sample_count
        or any(isinstance(value, bool) or not isinstance(value, (int, float)) for value in samples)
    ):
        raise PerformanceError(f"{name}.samplesMs has the wrong shape")
    expected_stats = summarize_ms([float(value) for value in samples])
    if side["stats"] != expected_stats:
        raise PerformanceError(f"{name}.stats does not match its raw samples")
    _expect_sha256(side["binarySha256"], f"{name}.binarySha256")
    if "outputSha256" in metadata_fields:
        _expect_sha256(side["outputSha256"], f"{name}.outputSha256")
    return side


def validate_receipt(receipt: object, *, require_pass: bool = True) -> None:
    """Validate a complete evidence receipt and recompute every conclusion."""
    root = _expect_exact_keys(
        receipt,
        {
            "schemaVersion",
            "generatedAt",
            "repository",
            "machine",
            "configuration",
            "revisions",
            "measurements",
            "verdict",
        },
        "receipt",
    )
    if root["schemaVersion"] != SCHEMA_VERSION:
        raise PerformanceError("receipt schema version differs")
    generated_at = _expect_text(root["generatedAt"], "generatedAt", 64)
    if UTC_TIMESTAMP_RE.fullmatch(generated_at) is None:
        raise PerformanceError("generatedAt must be a UTC timestamp with second precision")
    try:
        datetime.strptime(generated_at, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as error:
        raise PerformanceError("generatedAt is not a valid UTC timestamp") from error

    repository = _expect_exact_keys(
        root["repository"], {"url", "runnerSourceSha256"}, "repository"
    )
    if repository["url"] != REPOSITORY_URL:
        raise PerformanceError("repository URL differs")
    _expect_sha256(repository["runnerSourceSha256"], "repository.runnerSourceSha256")

    machine = _expect_exact_keys(
        root["machine"],
        {"os", "osRelease", "architecture", "cpu", "logicalCpus", "memoryBytes", "acPower"},
        "machine",
    )
    if machine != REFERENCE_MACHINE:
        raise PerformanceError("machine identity differs from the reference machine")

    configuration = _expect_exact_keys(
        root["configuration"],
        {"warmupSamplesPerRevision", "measuredSamplesPerRevision", "order", "profile", "locked"},
        "configuration",
    )
    warmups = configuration["warmupSamplesPerRevision"]
    samples = configuration["measuredSamplesPerRevision"]
    if warmups != EXPECTED_WARMUPS:
        raise PerformanceError("warmup sample count differs from the reference contract")
    if samples != EXPECTED_SAMPLES:
        raise PerformanceError("measured sample count differs from the reference contract")
    if configuration["order"] != "alternating-ab-ba":
        raise PerformanceError("measurement order differs")
    if configuration["profile"] != "release" or configuration["locked"] is not True:
        raise PerformanceError("build configuration differs")

    revisions = _expect_exact_keys(root["revisions"], {"before", "after"}, "revisions")
    for label, expected in (("before", BEFORE_REVISION), ("after", AFTER_REVISION)):
        revision = _expect_exact_keys(
            revisions[label], {"commit", "cargoVersion", "rustcVersion"}, f"revisions.{label}"
        )
        if revision["commit"] != expected:
            raise PerformanceError(f"{label} revision differs")
        if revision["cargoVersion"] != REFERENCE_TOOLCHAINS[label]["cargoVersion"]:
            raise PerformanceError(f"{label} Cargo toolchain differs")
        if revision["rustcVersion"] != REFERENCE_TOOLCHAINS[label]["rustcVersion"]:
            raise PerformanceError(f"{label} Rust toolchain differs")

    measurements = root["measurements"]
    if not isinstance(measurements, list) or len(measurements) != len(WORKLOADS):
        raise PerformanceError("receipt must contain every performance workload exactly once")
    found: dict[str, dict[str, object]] = {}
    failed = []
    for index, item_value in enumerate(measurements):
        item = _expect_exact_keys(
            item_value,
            {
                "name",
                "boundary",
                "allowedMedianRatio",
                "allowedMedianDeltaMs",
                "before",
                "after",
                "medianRatio",
                "medianDeltaMs",
                "passed",
            }
            | ({"probe"} if isinstance(item_value, dict) and item_value.get("name") == "appVisibleWindow" else set()),
            f"measurements[{index}]",
        )
        name = item["name"]
        if not isinstance(name, str) or name not in WORKLOADS or name in found:
            raise PerformanceError("measurement names must be known and unique")
        workload = WORKLOADS[name]
        if (
            item["boundary"] != workload["boundary"]
            or item["allowedMedianRatio"] != workload["limit"]
            or item["allowedMedianDeltaMs"] != workload["deltaMs"]
        ):
            raise PerformanceError(f"{name} measurement contract differs")
        metadata_fields = SIDE_METADATA_FIELDS[name]
        before = _validate_side(
            item["before"], f"{name}.before", samples, metadata_fields
        )
        after = _validate_side(
            item["after"], f"{name}.after", samples, metadata_fields
        )
        for label, side in (("before", before), ("after", after)):
            expected_identity = REFERENCE_IDENTITIES[name][label]
            for key, expected_value in expected_identity.items():
                if side[key] != expected_value:
                    raise PerformanceError(f"{name}.{label}.{key} identity differs")
        if name == "cliRequest":
            if before["outputSha256"] != after["outputSha256"]:
                raise PerformanceError("CLI output changed across the migration")
        elif name == "gpuPostcard":
            for side in (before, after):
                if side.get("outputDimensions") != list(EXPECTED_GPU_DIMENSIONS):
                    raise PerformanceError("GPU output dimensions differ")
                _expect_text(side.get("adapter"), "GPU adapter")
                _expect_text(side.get("backend"), "GPU backend")
                _expect_sha256(side.get("outputSha256"), "GPU output digest")
            if (before.get("adapter"), before.get("backend")) != (
                after.get("adapter"),
                after.get("backend"),
            ):
                raise PerformanceError("GPU adapter or backend changed across the comparison")
        elif name == "audioDeviceInit":
            for side in (before, after):
                _expect_text(side.get("device"), "audio device")
                if not isinstance(side.get("sampleRate"), int) or not 8_000 <= side["sampleRate"] <= 384_000:
                    raise PerformanceError("audio sample rate is invalid")
                if not isinstance(side.get("channels"), int) or not 1 <= side["channels"] <= 64:
                    raise PerformanceError("audio channel count is invalid")
            if (
                before.get("device"),
                before.get("sampleRate"),
                before.get("channels"),
            ) != (
                after.get("device"),
                after.get("sampleRate"),
                after.get("channels"),
            ):
                raise PerformanceError("audio device configuration changed across the comparison")
        else:
            if item.get("probe") != APP_PROBE:
                raise PerformanceError("App readiness probe differs")
        expected_ratio = round(
            after["stats"]["p50Ms"] / before["stats"]["p50Ms"], 6
        )
        expected_delta = round(after["stats"]["p50Ms"] - before["stats"]["p50Ms"], 6)
        if item["medianRatio"] != expected_ratio:
            raise PerformanceError(f"{name} median ratio is incorrect")
        if item["medianDeltaMs"] != expected_delta:
            raise PerformanceError(f"{name} median delta is incorrect")
        expected_pass = expected_ratio <= workload["limit"] or expected_delta <= workload["deltaMs"]
        if item["passed"] is not expected_pass:
            raise PerformanceError(f"{name} pass result is incorrect")
        if not expected_pass:
            failed.append(name)
        found[name] = item
    if set(found) != set(WORKLOADS):
        raise PerformanceError("receipt workload set differs")

    verdict = _expect_exact_keys(root["verdict"], {"passed", "failedMeasurements"}, "verdict")
    if verdict["passed"] is not (not failed) or verdict["failedMeasurements"] != failed:
        raise PerformanceError("receipt verdict does not match the measurements")
    if require_pass and failed:
        raise PerformanceError(f"performance regression gate failed: {', '.join(failed)}")


def validate_runner_identity(receipt: dict[str, object], runner_path: Path) -> None:
    """Require a tracked receipt to name the exact verifier and recorder bytes."""
    expected = receipt["repository"]["runnerSourceSha256"]
    actual = sha256_file(runner_path)
    if actual != expected:
        raise PerformanceError(
            "receipt runner digest differs; record fresh evidence after changing the runner"
        )


def _run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    timeout: float,
    capture: bool = True,
) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
        timeout=timeout,
        check=False,
    )
    if result.returncode != 0:
        stderr = (result.stderr or b"")[:4096].decode("utf-8", "replace")
        raise PerformanceError(f"command failed ({result.returncode}): {' '.join(command)}\n{stderr}")
    if capture and (len(result.stdout) > MAX_OUTPUT_BYTES or len(result.stderr) > MAX_OUTPUT_BYTES):
        raise PerformanceError("command output exceeded the evidence bound")
    return result


def _base_environment(target_dir: Path) -> dict[str, str]:
    environment = dict(os.environ)
    for name in list(environment):
        if name.startswith("CARGO_") and name not in {"CARGO_HOME"}:
            environment.pop(name)
        if name.startswith("RUST") and name not in {"RUSTUP_HOME"}:
            environment.pop(name)
    environment["CARGO_TARGET_DIR"] = str(target_dir)
    environment["CARGO_INCREMENTAL"] = "0"
    environment["NUMINOUS_MUTE"] = "1"
    environment["NO_COLOR"] = "1"
    return environment


def _create_worktree(repo: Path, path: Path, revision: str) -> None:
    if path.exists():
        raise PerformanceError(f"worktree path already exists: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    _run(
        ["git", "worktree", "add", "--detach", str(path), revision],
        cwd=repo,
        env=dict(os.environ),
        timeout=120.0,
        capture=False,
    )
    actual = _run(
        ["git", "rev-parse", "HEAD"], cwd=path, env=dict(os.environ), timeout=10.0
    ).stdout.decode("ascii").strip()
    if actual != revision:
        raise PerformanceError(f"worktree revision differs: {actual}")
    marker = {"schemaVersion": SCHEMA_VERSION, "revision": revision}
    (path / ".numinous-performance-worktree.json").write_bytes(canonical_json(marker))
    probe = path / "crates" / "audio" / "examples" / "dependency_migration_perf.rs"
    probe.write_text(AUDIO_PROBE_SOURCE, encoding="utf-8", newline="\n")


def _owned_directory(root: Path, name: str) -> Path:
    """Create one direct scratch child without following a redirected directory."""
    candidate = root / name
    if candidate.exists():
        resolved = candidate.resolve()
        requested = Path(os.path.abspath(candidate))
        if not candidate.is_dir() or resolved != requested or resolved.parent != root:
            raise PerformanceError(f"scratch directory is redirected or invalid: {candidate}")
        return resolved
    candidate.mkdir()
    resolved = candidate.resolve()
    if resolved.parent != root:
        raise PerformanceError(f"scratch directory escaped its owned root: {candidate}")
    return resolved


def _remove_worktree(repo: Path, path: Path, expected_revision: str, work_root: Path) -> None:
    resolved = path.resolve()
    expected_parent = (work_root / "worktrees").resolve()
    if resolved.parent != expected_parent:
        raise PerformanceError("refusing to remove a worktree outside the owned work root")
    marker_path = resolved / ".numinous-performance-worktree.json"
    if not marker_path.is_file():
        raise PerformanceError("refusing to remove a worktree without its ownership marker")
    marker = parse_json_strict(marker_path.read_bytes())
    if marker != {"schemaVersion": SCHEMA_VERSION, "revision": expected_revision}:
        raise PerformanceError("refusing to remove a worktree with a mismatched marker")
    actual = _run(
        ["git", "rev-parse", "HEAD"], cwd=resolved, env=dict(os.environ), timeout=10.0
    ).stdout.decode("ascii").strip()
    if actual != expected_revision:
        raise PerformanceError("refusing to remove a worktree at an unexpected revision")
    _run(
        ["git", "worktree", "remove", "--force", str(resolved)],
        cwd=repo,
        env=dict(os.environ),
        timeout=120.0,
        capture=False,
    )


def _build_revision(checkout: Path, target: Path) -> dict[str, object]:
    environment = _base_environment(target)
    probe = checkout / "crates" / "audio" / "examples" / "dependency_migration_perf.rs"
    if probe.read_text(encoding="utf-8") != AUDIO_PROBE_SOURCE:
        raise PerformanceError("audio probe source differs")
    commands = (
        ["cargo", "build", "--release", "--locked", "-p", "numinous-cli", "--bin", "numinous"],
        ["cargo", "build", "--release", "--locked", "-p", "numinous-app", "--bin", "numinous-app"],
        ["cargo", "build", "--release", "--locked", "-p", "numinous-gpu", "--example", "postcard"],
        [
            "cargo",
            "build",
            "--release",
            "--locked",
            "-p",
            "numinous-audio",
            "--example",
            "dependency_migration_perf",
        ],
    )
    for command in commands:
        _run(command, cwd=checkout, env=environment, timeout=1800.0, capture=False)
    suffix = ".exe" if os.name == "nt" else ""
    binaries = {
        "cliRequest": target / "release" / f"numinous{suffix}",
        "appVisibleWindow": target / "release" / f"numinous-app{suffix}",
        "gpuPostcard": target / "release" / "examples" / f"postcard{suffix}",
        "audioDeviceInit": target
        / "release"
        / "examples"
        / f"dependency_migration_perf{suffix}",
    }
    for name, path in binaries.items():
        if not path.is_file():
            raise PerformanceError(f"{name} binary is missing after build")
    cargo_version = _run(
        ["cargo", "--version"], cwd=checkout, env=environment, timeout=30.0
    ).stdout.decode("utf-8", "strict").strip()
    rustc_version = _run(
        ["rustc", "--version"], cwd=checkout, env=environment, timeout=30.0
    ).stdout.decode("utf-8", "strict").strip()
    return {
        "checkout": checkout,
        "environment": environment,
        "binaries": binaries,
        "cargoVersion": cargo_version,
        "rustcVersion": rustc_version,
    }


def _timed_process(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    timeout: float,
) -> tuple[float, subprocess.CompletedProcess[bytes]]:
    started = time.perf_counter_ns()
    result = _run(command, cwd=cwd, env=env, timeout=timeout)
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000.0
    return round(elapsed_ms, 6), result


def _cli_probe(build: dict[str, object], run_dir: Path) -> dict[str, object]:
    binary = build["binaries"]["cliRequest"]
    duration, result = _timed_process(
        [str(binary), "render", "times-tables", "--width", "80", "--height", "30"],
        cwd=run_dir,
        env=build["environment"],
        timeout=15.0,
    )
    if not 512 <= len(result.stdout) <= MAX_OUTPUT_BYTES:
        raise PerformanceError("CLI benchmark returned an implausible payload")
    return {"durationMs": duration, "outputSha256": sha256_bytes(result.stdout)}


def _gpu_probe(build: dict[str, object], run_dir: Path) -> dict[str, object]:
    output_path = run_dir / "mandelbrot.png"
    output_path.unlink(missing_ok=True)
    binary = build["binaries"]["gpuPostcard"]
    duration, result = _timed_process(
        [str(binary)], cwd=run_dir, env=build["environment"], timeout=60.0
    )
    text = result.stdout.decode("utf-8", "strict")
    identity = parse_gpu_receipt(text)
    dimensions = read_png_dimensions(output_path)
    if dimensions != EXPECTED_GPU_DIMENSIONS:
        raise PerformanceError(f"GPU output dimensions differ: {dimensions}")
    return {
        "durationMs": duration,
        **identity,
        "outputDimensions": list(dimensions),
        "outputSha256": sha256_file(output_path),
    }


def _audio_probe(build: dict[str, object], run_dir: Path) -> dict[str, object]:
    binary = build["binaries"]["audioDeviceInit"]
    result = _run(
        [str(binary)], cwd=run_dir, env=build["environment"], timeout=15.0
    )
    return parse_audio_receipt(result.stdout.decode("utf-8", "strict"))


def _load_user32() -> tuple[ctypes.WinDLL, object]:
    """Load the Win32 calls with pointer-width-correct signatures."""
    if os.name != "nt":
        raise PerformanceError("App readiness measurement currently requires Windows")
    from ctypes import wintypes

    user32 = ctypes.WinDLL("user32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(
        wintypes.BOOL, wintypes.HWND, wintypes.LPARAM
    )
    user32.EnumWindows.argtypes = (callback_type, wintypes.LPARAM)
    user32.EnumWindows.restype = wintypes.BOOL
    user32.GetWindowThreadProcessId.argtypes = (
        wintypes.HWND,
        ctypes.POINTER(wintypes.DWORD),
    )
    user32.GetWindowThreadProcessId.restype = wintypes.DWORD
    user32.IsWindowVisible.argtypes = (wintypes.HWND,)
    user32.IsWindowVisible.restype = wintypes.BOOL
    user32.GetWindowTextLengthW.argtypes = (wintypes.HWND,)
    user32.GetWindowTextLengthW.restype = ctypes.c_int
    user32.GetWindowTextW.argtypes = (
        wintypes.HWND,
        wintypes.LPWSTR,
        ctypes.c_int,
    )
    user32.GetWindowTextW.restype = ctypes.c_int
    user32.PostMessageW.argtypes = (
        wintypes.HWND,
        wintypes.UINT,
        wintypes.WPARAM,
        wintypes.LPARAM,
    )
    user32.PostMessageW.restype = wintypes.BOOL
    return user32, callback_type


def _visible_window_for_process(process_id: int) -> int | None:
    user32, callback_type = _load_user32()
    found: list[int] = []

    def inspect(window: int, _: int) -> bool:
        from ctypes import wintypes

        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(window, ctypes.byref(owner))
        if owner.value != process_id or not user32.IsWindowVisible(window):
            return True
        length = user32.GetWindowTextLengthW(window)
        if not 1 <= length <= 1024:
            return True
        title = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(window, title, len(title))
        if title.value.startswith("Numinous"):
            found.append(int(window))
            return False
        return True

    callback = callback_type(inspect)
    user32.EnumWindows(callback, 0)
    return found[0] if found else None


def _app_probe(build: dict[str, object], run_dir: Path, sequence: int) -> dict[str, object]:
    if os.name != "nt":
        raise PerformanceError("App readiness measurement currently requires Windows")
    profile = run_dir / f"app-profile-{sequence:04d}"
    profile.mkdir()
    environment = dict(build["environment"])
    environment["HOME"] = str(profile)
    environment["USERPROFILE"] = str(profile)
    environment["NUMINOUS_JOURNEY"] = str(profile / "journey.txt")
    environment["NUMINOUS_SCORES"] = str(profile / "scores.txt")
    environment["NUMINOUS_CAIRN"] = str(profile / "cairn.txt")
    binary = build["binaries"]["appVisibleWindow"]
    started = time.perf_counter_ns()
    process = subprocess.Popen(
        [str(binary)],
        cwd=run_dir,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    window = None
    deadline = time.monotonic() + 15.0
    try:
        while time.monotonic() < deadline:
            code = process.poll()
            if code is not None:
                stderr = (process.stderr.read() if process.stderr is not None else b"")[:4096]
                raise PerformanceError(
                    f"App exited before its window became visible ({code}): "
                    f"{stderr.decode('utf-8', 'replace')}"
                )
            window = _visible_window_for_process(process.pid)
            if window is not None:
                break
            time.sleep(0.002)
        if window is None:
            raise PerformanceError("App window did not become visible within 15 seconds")
        duration = round((time.perf_counter_ns() - started) / 1_000_000.0, 6)
        user32, _ = _load_user32()
        if not user32.PostMessageW(window, 0x0010, 0, 0):
            raise PerformanceError("could not request a clean App window close")
        try:
            code = process.wait(timeout=10.0)
        except subprocess.TimeoutExpired as error:
            raise PerformanceError("App did not exit after its window close request") from error
        if code != 0:
            raise PerformanceError(f"App exited with status {code} after its window close request")
        return {"durationMs": duration}
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5.0)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5.0)


def _run_interleaved(
    before: Callable[[], dict[str, object]],
    after: Callable[[], dict[str, object]],
    warmups: int,
    samples: int,
) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    for index in range(warmups):
        order = (before, after) if index % 2 == 0 else (after, before)
        for probe in order:
            probe()
    before_results = []
    after_results = []
    for index in range(samples):
        if index % 2 == 0:
            before_results.append(before())
            after_results.append(after())
        else:
            after_results.append(after())
            before_results.append(before())
    return before_results, after_results


def _same_observation(results: list[dict[str, object]], key: str, name: str) -> object:
    values = {json.dumps(result.get(key), sort_keys=True) for result in results}
    if len(values) != 1:
        raise PerformanceError(f"{name} changed between samples")
    return results[0][key]


def _side_record(
    name: str,
    build: dict[str, object],
    results: list[dict[str, object]],
) -> dict[str, object]:
    samples = [round(float(result["durationMs"]), 6) for result in results]
    record: dict[str, object] = {
        "samplesMs": samples,
        "stats": summarize_ms(samples),
        "binarySha256": sha256_file(build["binaries"][name]),
    }
    keys = {
        "cliRequest": ("outputSha256",),
        "gpuPostcard": ("adapter", "backend", "outputDimensions", "outputSha256"),
        "audioDeviceInit": ("device", "sampleRate", "channels"),
        "appVisibleWindow": (),
    }[name]
    for key in keys:
        record[key] = _same_observation(results, key, f"{name} {key}")
    return record


def _measurement(
    name: str,
    before_build: dict[str, object],
    after_build: dict[str, object],
    before_results: list[dict[str, object]],
    after_results: list[dict[str, object]],
) -> dict[str, object]:
    before = _side_record(name, before_build, before_results)
    after = _side_record(name, after_build, after_results)
    ratio = round(after["stats"]["p50Ms"] / before["stats"]["p50Ms"], 6)
    delta = round(after["stats"]["p50Ms"] - before["stats"]["p50Ms"], 6)
    item = {
        "name": name,
        "boundary": WORKLOADS[name]["boundary"],
        "allowedMedianRatio": WORKLOADS[name]["limit"],
        "allowedMedianDeltaMs": WORKLOADS[name]["deltaMs"],
        "before": before,
        "after": after,
        "medianRatio": ratio,
        "medianDeltaMs": delta,
        "passed": ratio <= WORKLOADS[name]["limit"] or delta <= WORKLOADS[name]["deltaMs"],
    }
    if name == "appVisibleWindow":
        item["probe"] = APP_PROBE
    return item


def _machine_receipt() -> dict[str, object]:
    if os.name != "nt":
        raise PerformanceError("evidence generation currently requires Windows")

    class MemoryStatus(ctypes.Structure):
        _fields_ = [
            ("length", ctypes.c_ulong),
            ("memoryLoad", ctypes.c_ulong),
            ("totalPhysical", ctypes.c_ulonglong),
            ("availablePhysical", ctypes.c_ulonglong),
            ("totalPageFile", ctypes.c_ulonglong),
            ("availablePageFile", ctypes.c_ulonglong),
            ("totalVirtual", ctypes.c_ulonglong),
            ("availableVirtual", ctypes.c_ulonglong),
            ("availableExtendedVirtual", ctypes.c_ulonglong),
        ]

    class PowerStatus(ctypes.Structure):
        _fields_ = [
            ("acLineStatus", ctypes.c_ubyte),
            ("batteryFlag", ctypes.c_ubyte),
            ("batteryLifePercent", ctypes.c_ubyte),
            ("systemStatusFlag", ctypes.c_ubyte),
            ("batteryLifeTime", ctypes.c_ulong),
            ("batteryFullLifeTime", ctypes.c_ulong),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    memory = MemoryStatus()
    memory.length = ctypes.sizeof(memory)
    if not kernel32.GlobalMemoryStatusEx(ctypes.byref(memory)):
        raise PerformanceError("could not read physical memory information")
    power = PowerStatus()
    if not kernel32.GetSystemPowerStatus(ctypes.byref(power)):
        raise PerformanceError("could not read system power status")
    if power.acLineStatus != 1:
        raise PerformanceError("performance evidence requires the machine to be on AC power")
    cpu = os.environ.get("PROCESSOR_IDENTIFIER") or platform.processor() or "unknown Windows CPU"
    return {
        "os": platform.system(),
        "osRelease": platform.version(),
        "architecture": platform.machine(),
        "cpu": cpu,
        "logicalCpus": os.cpu_count() or 1,
        "memoryBytes": int(memory.totalPhysical),
        "acPower": True,
    }


def _parse_count(value: str) -> int:
    try:
        count = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be an integer") from error
    if not 1 <= count <= MAX_SAMPLES:
        raise argparse.ArgumentTypeError(f"must be between 1 and {MAX_SAMPLES}")
    return count


def _repo_root() -> Path:
    script = Path(__file__).resolve()
    root = script.parent.parent
    if not (root / ".git").exists() or not (root / "Cargo.toml").is_file():
        raise PerformanceError("runner must execute from the Numinous repository")
    return root


def _resolve_record_paths(repo: Path, output: Path, work_root: Path) -> tuple[Path, Path]:
    """Resolve and separate the retained receipt from cleanup-owned scratch."""
    agent_root = (repo / ".agent").resolve()
    requested_work_root = Path(os.path.abspath(work_root))
    resolved_work_root = work_root.resolve()
    if work_root.exists() and resolved_work_root != requested_work_root:
        raise PerformanceError("work root must not be a symbolic link or redirected directory")
    if agent_root not in resolved_work_root.parents:
        raise PerformanceError("work root must be inside the ignored .agent directory")
    resolved_output = output.resolve()
    if agent_root not in resolved_output.parents:
        raise PerformanceError("new receipts must first be written inside the ignored .agent directory")
    if resolved_output == resolved_work_root or resolved_work_root in resolved_output.parents:
        raise PerformanceError("receipt output must be outside the cleanup-owned work root")
    return resolved_work_root, resolved_output


def generate_receipt(output: Path, work_root: Path, warmups: int, samples: int) -> dict[str, object]:
    if sys.version_info < (3, 11):
        raise PerformanceError("Python 3.11 or newer is required")
    if warmups != EXPECTED_WARMUPS or samples != EXPECTED_SAMPLES:
        raise PerformanceError("recording requires exactly three warmups and twenty samples")
    repo = _repo_root()
    resolved_work_root, resolved_output = _resolve_record_paths(repo, output, work_root)
    agent_root = (repo / ".agent").resolve()
    resolved_work_root.mkdir(parents=True, exist_ok=True)
    if agent_root not in resolved_work_root.resolve().parents:
        raise PerformanceError("work root is redirected outside the ignored .agent directory")
    worktrees = _owned_directory(resolved_work_root, "worktrees")
    targets = _owned_directory(resolved_work_root, "targets")
    runs = _owned_directory(resolved_work_root, "runs")
    before_path = worktrees / "before"
    after_path = worktrees / "after"
    created: list[tuple[Path, str]] = []
    try:
        _create_worktree(repo, before_path, BEFORE_REVISION)
        created.append((before_path, BEFORE_REVISION))
        _create_worktree(repo, after_path, AFTER_REVISION)
        created.append((after_path, AFTER_REVISION))
        print("Building exact before revision", flush=True)
        before_build = _build_revision(before_path, targets / "before")
        print("Building exact after revision", flush=True)
        after_build = _build_revision(after_path, targets / "after")
        run_root = runs / secrets.token_hex(12)
        before_run = run_root / "before"
        after_run = run_root / "after"
        before_run.mkdir(parents=True, exist_ok=True)
        after_run.mkdir(parents=True, exist_ok=True)
        measurements = []
        app_sequence = {"before": 0, "after": 0}
        probes: dict[str, tuple[Callable[[], dict[str, object]], Callable[[], dict[str, object]]]] = {
            "cliRequest": (
                lambda: _cli_probe(before_build, before_run),
                lambda: _cli_probe(after_build, after_run),
            ),
            "gpuPostcard": (
                lambda: _gpu_probe(before_build, before_run),
                lambda: _gpu_probe(after_build, after_run),
            ),
            "audioDeviceInit": (
                lambda: _audio_probe(before_build, before_run),
                lambda: _audio_probe(after_build, after_run),
            ),
            "appVisibleWindow": (
                lambda: _app_probe(
                    before_build,
                    before_run,
                    app_sequence.__setitem__("before", app_sequence["before"] + 1)
                    or app_sequence["before"],
                ),
                lambda: _app_probe(
                    after_build,
                    after_run,
                    app_sequence.__setitem__("after", app_sequence["after"] + 1)
                    or app_sequence["after"],
                ),
            ),
        }
        for name in WORKLOADS:
            print(f"Measuring {name}", flush=True)
            before_results, after_results = _run_interleaved(
                probes[name][0], probes[name][1], warmups, samples
            )
            measurements.append(
                _measurement(name, before_build, after_build, before_results, after_results)
            )
        failed = [item["name"] for item in measurements if not item["passed"]]
        receipt = {
            "schemaVersion": SCHEMA_VERSION,
            "generatedAt": datetime.now(timezone.utc).isoformat(timespec="seconds").replace(
                "+00:00", "Z"
            ),
            "repository": {
                "url": REPOSITORY_URL,
                "runnerSourceSha256": sha256_file(Path(__file__).resolve()),
            },
            "machine": _machine_receipt(),
            "configuration": {
                "warmupSamplesPerRevision": warmups,
                "measuredSamplesPerRevision": samples,
                "order": "alternating-ab-ba",
                "profile": "release",
                "locked": True,
            },
            "revisions": {
                "before": {
                    "commit": BEFORE_REVISION,
                    "cargoVersion": before_build["cargoVersion"],
                    "rustcVersion": before_build["rustcVersion"],
                },
                "after": {
                    "commit": AFTER_REVISION,
                    "cargoVersion": after_build["cargoVersion"],
                    "rustcVersion": after_build["rustcVersion"],
                },
            },
            "measurements": measurements,
            "verdict": {"passed": not failed, "failedMeasurements": failed},
        }
        validate_receipt(receipt, require_pass=False)
        resolved_output.parent.mkdir(parents=True, exist_ok=True)
        resolved_output.write_bytes(canonical_json(receipt))
        return receipt
    finally:
        cleanup_errors = []
        for path, revision in reversed(created):
            if path.exists():
                try:
                    _remove_worktree(repo, path, revision, resolved_work_root)
                except (OSError, PerformanceError, subprocess.SubprocessError) as error:
                    cleanup_errors.append(str(error))
        if cleanup_errors:
            raise PerformanceError("worktree cleanup failed: " + "; ".join(cleanup_errors))


def _print_summary(receipt: dict[str, object]) -> None:
    for item in receipt["measurements"]:
        before = item["before"]["stats"]
        after = item["after"]["stats"]
        print(
            f"{item['name']}: before p50 {before['p50Ms']:.3f} ms, "
            f"after p50 {after['p50Ms']:.3f} ms, delta {item['medianDeltaMs']:.3f} ms, "
            f"ratio {item['medianRatio']:.3f}, "
            f"{'PASS' if item['passed'] else 'FAIL'}"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--record", type=Path, metavar="RECEIPT")
    mode.add_argument("--verify-receipt", type=Path, metavar="RECEIPT")
    parser.add_argument("--work-dir", type=Path, default=Path(".agent/dependency-migration-performance"))
    parser.add_argument("--warmup", type=_parse_count, default=3)
    parser.add_argument("--samples", type=_parse_count, default=20)
    args = parser.parse_args(argv)
    try:
        if args.verify_receipt is not None:
            content = args.verify_receipt.read_bytes()
            receipt = parse_json_strict(content)
            validate_receipt(receipt)
            validate_runner_identity(receipt, Path(__file__).resolve())
            if content != canonical_json(receipt):
                raise PerformanceError("receipt is not canonical JSON")
            _print_summary(receipt)
            print("Dependency migration performance receipt verified.")
            return 0
        receipt = generate_receipt(args.record, args.work_dir, args.warmup, args.samples)
        _print_summary(receipt)
        if not receipt["verdict"]["passed"]:
            print("Dependency migration performance gate failed.", file=sys.stderr)
            return 1
        print(f"Wrote passing receipt: {args.record}")
        return 0
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError, PerformanceError) as error:
        print(f"dependency migration performance: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
