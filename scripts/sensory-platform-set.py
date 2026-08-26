#!/usr/bin/env python3
"""Build and verify the closed six-receipt Sensory Lift pacing set."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import stat
import sys
from typing import Any, NoReturn, Sequence


RECEIPT_SCHEMA = "numinous.sensory-app-platform"
RECEIPT_SCHEMA_VERSION = 1
SET_SCHEMA = "numinous.sensory-app-platform-set"
SET_SCHEMA_VERSION = 1
MAX_RECEIPT_BYTES = 262_144
MAX_SET_BYTES = 131_072
SHA256_RE = re.compile(r"[0-9a-f]{64}")
REVISION_RE = re.compile(r"[0-9a-f]{40}")
VERSION_RE = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
)
OPERATING_SYSTEMS = ("linux", "macos", "windows")
TARGETS = {
    (1920, 1080): 33.0,
    (2560, 1440): 50.0,
}
COMPONENTS = [
    "core room render_input",
    "App input feedback",
    "App room chrome",
    "App audio badge",
    "App spectrum meter",
    "core Modern era transform",
]
EXCLUDES = [
    "compositor completion",
    "display scanout",
    "input latency",
    "perceptual quality",
]
SET_CLAIM = (
    "the six named physical reference candidates passed the production App "
    "surface pacing contract at 1080p and 1440p on Windows, macOS, and Linux"
)
SET_LIMITATIONS = [
    "each result applies only to the named physical reference machine",
    "compositor completion and display scanout remain outside the measured boundary",
    "input latency remains outside the measured boundary",
    "perceptual quality and accessibility remain separate review boundaries",
    "six passing candidates do not establish universal hardware performance",
    "receipt hashes bind content but do not authenticate the operator or capture conditions",
]


class PacingSetError(RuntimeError):
    """A pacing receipt or its closed set violated the evidence contract."""


def unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build one JSON object while rejecting duplicate member names."""
    value: dict[str, Any] = {}
    for name, member in pairs:
        if name in value:
            raise PacingSetError(f"JSON object repeats field {name!r}")
        value[name] = member
    return value


def reject_nonfinite_json(value: str) -> NoReturn:
    """Reject nonstandard JSON numeric constants."""
    raise PacingSetError(f"JSON contains non-finite number {value}")


def parse_json(data: str | bytes) -> Any:
    """Parse strict JSON without duplicate fields or non-finite numbers."""
    return json.loads(
        data,
        object_pairs_hook=unique_json_object,
        parse_constant=reject_nonfinite_json,
    )


def require_exact_keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    """Require an object with one exact field inventory."""
    if not isinstance(value, dict) or set(value) != expected:
        raise PacingSetError(f"{label} fields do not match the schema")
    return value


def require_text(value: Any, label: str, *, allow_empty: bool = False) -> str:
    """Require bounded canonical printable text."""
    if (
        not isinstance(value, str)
        or len(value) > 500
        or value != value.strip()
        or (not value and not allow_empty)
        or not all(character.isprintable() for character in value)
    ):
        raise PacingSetError(f"{label} is malformed")
    return value


def require_sha256(value: Any, label: str) -> str:
    """Require a canonical SHA-256 digest."""
    if not isinstance(value, str) or SHA256_RE.fullmatch(value) is None:
        raise PacingSetError(f"{label} is malformed")
    return value


def require_u32(value: Any, label: str) -> int:
    """Require one JSON integer in the unsigned 32-bit range."""
    if type(value) is not int or not 0 <= value <= 0xFFFF_FFFF:
        raise PacingSetError(f"{label} is malformed")
    return value


def require_positive_int(value: Any, label: str) -> int:
    """Require one strictly positive JSON integer."""
    if type(value) is not int or value <= 0:
        raise PacingSetError(f"{label} is malformed")
    return value


def require_duration(value: Any, label: str) -> float:
    """Require one finite nonnegative duration without accepting booleans."""
    if type(value) not in {int, float}:
        raise PacingSetError(f"{label} is malformed")
    duration = float(value)
    if not math.isfinite(duration) or duration < 0.0:
        raise PacingSetError(f"{label} is malformed")
    return duration


def read_bounded_regular(path: Path, limit: int) -> bytes:
    """Read one bounded ordinary file without following a final link."""
    try:
        metadata = path.lstat()
    except OSError as error:
        raise PacingSetError(f"cannot inspect {path}: {error}") from error
    if not stat.S_ISREG(metadata.st_mode) or path.is_symlink():
        raise PacingSetError(f"evidence path is not an ordinary file: {path}")
    if metadata.st_size > limit:
        raise PacingSetError(f"evidence file exceeds {limit} bytes: {path}")
    try:
        data = path.read_bytes()
    except OSError as error:
        raise PacingSetError(f"cannot read {path}: {error}") from error
    if len(data) != metadata.st_size:
        raise PacingSetError(f"evidence file changed while being read: {path}")
    return data


def nearest_rank(values: Sequence[float], quantile: float) -> float:
    """Match the Rust probe's nearest-rank percentile calculation."""
    ordered = sorted(values)
    if not ordered:
        raise PacingSetError("sample summary is empty")
    rank = math.ceil(quantile * len(ordered))
    return ordered[max(rank - 1, 0)]


def validate_summary(value: Any, samples: int, label: str) -> list[float]:
    """Recompute every retained timing summary."""
    summary = require_exact_keys(value, {"raw", "p50", "p95", "maximum"}, label)
    raw = summary["raw"]
    if not isinstance(raw, list) or len(raw) != samples:
        raise PacingSetError(f"{label} does not contain exactly {samples} raw samples")
    durations = [require_duration(item, f"{label} sample") for item in raw]
    expected = {
        "p50": nearest_rank(durations, 0.50),
        "p95": nearest_rank(durations, 0.95),
        "maximum": max(durations),
    }
    for field, result in expected.items():
        actual = require_duration(summary[field], f"{label} {field}")
        if actual != result:
            raise PacingSetError(f"{label} {field} disagrees with its raw samples")
    return durations


def validate_receipt(value: Any) -> dict[str, Any]:
    """Validate one complete physical pacing receipt and recompute its claims."""
    receipt = require_exact_keys(
        value,
        {
            "schema",
            "schemaVersion",
            "evidence",
            "build",
            "platform",
            "adapter",
            "surface",
            "source",
            "warmups",
            "samples",
            "presentedFrames",
            "skippedFrames",
            "suboptimalFrames",
            "acquireMs",
            "renderAndPresentMs",
            "boundaryMs",
            "boundaryBudgetMs",
            "checkEnforced",
            "failures",
            "verdict",
        },
        "receipt",
    )
    if (
        receipt["schema"] != RECEIPT_SCHEMA
        or type(receipt["schemaVersion"]) is not int
        or receipt["schemaVersion"] != RECEIPT_SCHEMA_VERSION
    ):
        raise PacingSetError("receipt schema is unsupported")

    evidence = require_exact_keys(
        receipt["evidence"],
        {"class", "timingAuthority", "correctnessClaim", "pacingClaim", "excludes"},
        "receipt evidence",
    )
    if (
        evidence["class"] != "physical-reference-pacing"
        or evidence["timingAuthority"] != "physical-reference-candidate"
        or evidence["correctnessClaim"]
        != "the deterministic fully composed App frame completed through the production direct surface presenter on this runtime"
        or evidence["pacingClaim"]
        != "the recorded acquire-through-present-request samples are a candidate result for this named physical reference only"
        or evidence["excludes"] != EXCLUDES
    ):
        raise PacingSetError("receipt evidence claim is not the physical pacing contract")

    build = require_exact_keys(
        receipt["build"],
        {"packageVersion", "revision", "profile", "binarySha256"},
        "receipt build",
    )
    package_version = require_text(build["packageVersion"], "package version")
    if VERSION_RE.fullmatch(package_version) is None:
        raise PacingSetError("package version is not canonical semantic versioning")
    if not isinstance(build["revision"], str) or REVISION_RE.fullmatch(build["revision"]) is None:
        raise PacingSetError("build revision is not a full canonical commit")
    if build["profile"] != "release":
        raise PacingSetError("physical pacing receipt is not a release build")
    require_sha256(build["binarySha256"], "binary hash")

    platform = require_exact_keys(
        receipt["platform"],
        {"os", "architecture", "family", "githubActions", "machine", "osVersion", "powerState"},
        "receipt platform",
    )
    if platform["os"] not in OPERATING_SYSTEMS:
        raise PacingSetError("receipt operating system is unsupported")
    expected_family = "windows" if platform["os"] == "windows" else "unix"
    if platform["family"] != expected_family:
        raise PacingSetError("receipt operating system family is inconsistent")
    require_text(platform["architecture"], "platform architecture")
    require_text(platform["machine"], "physical machine")
    require_text(platform["osVersion"], "operating system version")
    if platform["githubActions"] is not False:
        raise PacingSetError("hosted CI timing cannot enter the physical set")
    if platform["powerState"] != "ac":
        raise PacingSetError("physical pacing receipt was not recorded on AC power")

    adapter = require_exact_keys(
        receipt["adapter"],
        {"name", "vendorId", "deviceId", "deviceType", "driver", "driverInfo", "backend", "physicalGpu"},
        "receipt adapter",
    )
    require_text(adapter["name"], "adapter name")
    require_u32(adapter["vendorId"], "adapter vendor id")
    require_u32(adapter["deviceId"], "adapter device id")
    require_text(adapter["driver"], "adapter driver", allow_empty=True)
    require_text(adapter["driverInfo"], "adapter driver information", allow_empty=True)
    require_text(adapter["backend"], "adapter backend")
    if adapter["deviceType"] not in {"IntegratedGpu", "DiscreteGpu"} or adapter["physicalGpu"] is not True:
        raise PacingSetError("physical pacing requires an integrated or discrete GPU")

    surface = require_exact_keys(
        receipt["surface"],
        {
            "requestedWidth",
            "requestedHeight",
            "actualWidth",
            "actualHeight",
            "format",
            "presentMode",
            "desiredMaximumFrameLatency",
        },
        "receipt surface",
    )
    requested = (
        require_positive_int(surface["requestedWidth"], "requested width"),
        require_positive_int(surface["requestedHeight"], "requested height"),
    )
    actual = (
        require_positive_int(surface["actualWidth"], "actual width"),
        require_positive_int(surface["actualHeight"], "actual height"),
    )
    if requested not in TARGETS or actual != requested:
        raise PacingSetError("physical surface is not one exact reference target")
    frame_latency = require_positive_int(
        surface["desiredMaximumFrameLatency"], "maximum frame latency"
    )
    if (
        not isinstance(surface["format"], str)
        or not surface["format"].endswith("Srgb")
        or surface["presentMode"] != "Fifo"
        or frame_latency != 1
    ):
        raise PacingSetError("physical surface did not retain the sRGB FIFO contract")

    source = require_exact_keys(
        receipt["source"],
        {
            "room",
            "variation",
            "phase",
            "width",
            "height",
            "byteLength",
            "litPixels",
            "allAlphaOpaque",
            "firstRenderSha256",
            "repeatRenderSha256",
            "deterministic",
            "components",
        },
        "receipt source",
    )
    source_width = require_positive_int(source["width"], "source width")
    source_height = require_positive_int(source["height"], "source height")
    source_bytes = require_positive_int(source["byteLength"], "source byte length")
    if (
        source["room"] != "times-tables"
        or type(source["variation"]) is not int
        or source["variation"] != 17
        or type(source["phase"]) not in {int, float}
        or source["phase"] != 0.375
        or (source_width, source_height) != requested
        or source_bytes != requested[0] * requested[1] * 4
        or type(source["litPixels"]) is not int
        or source["litPixels"] < 100
        or source["allAlphaOpaque"] is not True
        or source["deterministic"] is not True
        or source["components"] != COMPONENTS
    ):
        raise PacingSetError("receipt source does not match the deterministic App frame contract")
    first_source_hash = require_sha256(source["firstRenderSha256"], "source hash")
    if source["repeatRenderSha256"] != first_source_hash:
        raise PacingSetError("receipt source renders are not byte exact")

    warmups = require_positive_int(receipt["warmups"], "warmup count")
    samples = require_positive_int(receipt["samples"], "sample count")
    if warmups < 30 or samples < 120:
        raise PacingSetError("physical pacing receipt has too few warmups or samples")
    presented_frames = require_positive_int(receipt["presentedFrames"], "presented frame count")
    skipped_frames = require_u32(receipt["skippedFrames"], "skipped frame count")
    suboptimal_frames = require_u32(receipt["suboptimalFrames"], "suboptimal frame count")
    if (
        presented_frames != warmups + samples
        or skipped_frames != 0
        or suboptimal_frames != 0
    ):
        raise PacingSetError("physical pacing receipt has incomplete or transient frames")
    acquire = validate_summary(receipt["acquireMs"], samples, "acquire summary")
    rendered = validate_summary(
        receipt["renderAndPresentMs"], samples, "render and present summary"
    )
    boundary = validate_summary(receipt["boundaryMs"], samples, "boundary summary")
    for index, (acquire_ms, render_ms, boundary_ms) in enumerate(
        zip(acquire, rendered, boundary, strict=True)
    ):
        if abs((acquire_ms + render_ms) - boundary_ms) > 0.000_002:
            raise PacingSetError(f"boundary sample {index} disagrees with its measured segments")
    budget = TARGETS[requested]
    if require_duration(receipt["boundaryBudgetMs"], "boundary budget") != budget:
        raise PacingSetError("receipt boundary budget does not match its target")
    if float(receipt["boundaryMs"]["p95"]) > budget:
        raise PacingSetError("receipt boundary p95 misses its target budget")
    if receipt["checkEnforced"] is not True or receipt["failures"] != [] or receipt["verdict"] != "pass":
        raise PacingSetError("receipt does not carry an enforced passing verdict")
    return receipt


def read_receipt(path: Path) -> tuple[dict[str, Any], bytes]:
    """Read and validate one bounded physical receipt."""
    data = read_bounded_regular(path, MAX_RECEIPT_BYTES)
    try:
        value = parse_json(data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise PacingSetError(f"receipt is not strict JSON: {path}: {error}") from error
    return validate_receipt(value), data


def validator_sha256() -> str:
    """Bind a set manifest to the exact validator source without claiming authentication."""
    return hashlib.sha256(Path(__file__).resolve().read_bytes()).hexdigest()


def target_entry(receipt: dict[str, Any], data: bytes) -> dict[str, Any]:
    """Project one validated receipt into the closed set manifest."""
    platform = receipt["platform"]
    adapter = receipt["adapter"]
    surface = receipt["surface"]
    return {
        "os": platform["os"],
        "architecture": platform["architecture"],
        "width": surface["requestedWidth"],
        "height": surface["requestedHeight"],
        "budgetMs": receipt["boundaryBudgetMs"],
        "machine": platform["machine"],
        "osVersion": platform["osVersion"],
        "adapterName": adapter["name"],
        "adapterVendorId": adapter["vendorId"],
        "adapterDeviceId": adapter["deviceId"],
        "adapterDeviceType": adapter["deviceType"],
        "backend": adapter["backend"],
        "driver": adapter["driver"],
        "driverInfo": adapter["driverInfo"],
        "binarySha256": receipt["build"]["binarySha256"],
        "sourceSha256": receipt["source"]["firstRenderSha256"],
        "receiptSha256": hashlib.sha256(data).hexdigest(),
        "boundaryP95Ms": receipt["boundaryMs"]["p95"],
    }


def build_manifest(records: Sequence[tuple[dict[str, Any], bytes]]) -> dict[str, Any]:
    """Validate exact matrix coverage and build its deterministic manifest."""
    if len(records) != len(OPERATING_SYSTEMS) * len(TARGETS):
        raise PacingSetError("physical pacing set requires exactly six receipts")
    receipts: list[dict[str, Any]] = []
    for receipt, data in records:
        parsed = validate_receipt(parse_json(data))
        if parsed != receipt:
            raise PacingSetError("parsed receipt disagrees with its retained bytes")
        receipts.append(parsed)
    versions = {receipt["build"]["packageVersion"] for receipt in receipts}
    revisions = {receipt["build"]["revision"] for receipt in receipts}
    if len(versions) != 1 or len(revisions) != 1:
        raise PacingSetError("physical pacing set mixes build identities")

    by_cell: dict[tuple[str, int, int], tuple[dict[str, Any], bytes]] = {}
    for record in records:
        receipt, _data = record
        surface = receipt["surface"]
        cell = (receipt["platform"]["os"], surface["requestedWidth"], surface["requestedHeight"])
        if cell in by_cell:
            raise PacingSetError(f"physical pacing set repeats target {cell}")
        by_cell[cell] = record
    expected_cells = {
        (operating_system, width, height)
        for operating_system in OPERATING_SYSTEMS
        for width, height in TARGETS
    }
    missing = sorted(expected_cells - set(by_cell))
    if missing:
        raise PacingSetError(f"physical pacing set is missing target {missing[0]}")

    for operating_system in OPERATING_SYSTEMS:
        pair = [
            by_cell[(operating_system, width, height)][0]
            for width, height in TARGETS
        ]
        identity = (
            "architecture",
            "machine",
            "osVersion",
            "powerState",
        )
        if any(pair[0]["platform"][field] != pair[1]["platform"][field] for field in identity):
            raise PacingSetError(f"{operating_system} target pair mixes platform identity")
        if pair[0]["adapter"] != pair[1]["adapter"]:
            raise PacingSetError(f"{operating_system} target pair mixes adapter identity")
        if pair[0]["build"]["binarySha256"] != pair[1]["build"]["binarySha256"]:
            raise PacingSetError(f"{operating_system} target pair mixes executable identity")

    for width, height in TARGETS:
        sources = [
            by_cell[(operating_system, width, height)][0]["source"]
            for operating_system in OPERATING_SYSTEMS
        ]
        if sources[1:] != sources[:-1]:
            raise PacingSetError(f"{width}x{height} deterministic source differs across operating systems")

    adapter_names = {receipt["adapter"]["name"].casefold() for receipt in receipts}
    if len(adapter_names) < 2:
        raise PacingSetError("physical pacing set requires at least two distinct adapters")
    entries = [target_entry(*by_cell[cell]) for cell in sorted(by_cell)]
    return {
        "schema": SET_SCHEMA,
        "schemaVersion": SET_SCHEMA_VERSION,
        "verdict": "pass",
        "claim": SET_CLAIM,
        "limitations": SET_LIMITATIONS,
        "packageVersion": next(iter(versions)),
        "revision": next(iter(revisions)),
        "receiptCount": len(entries),
        "validatorSha256": validator_sha256(),
        "targets": entries,
    }


def canonical_document(value: Any) -> bytes:
    """Encode one human-readable deterministic evidence document."""
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("utf-8")


def write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    """Write one manifest without replacing an existing path."""
    if path.parent != Path(""):
        path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as destination:
            destination.write(canonical_document(manifest))
    except FileExistsError as error:
        raise PacingSetError(f"manifest path already exists: {path}") from error


def verify_manifest(path: Path, records: Sequence[tuple[dict[str, Any], bytes]]) -> dict[str, Any]:
    """Rebuild a set and require the retained manifest to match byte for byte."""
    data = read_bounded_regular(path, MAX_SET_BYTES)
    try:
        manifest = parse_json(data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise PacingSetError(f"manifest is not strict JSON: {error}") from error
    expected = build_manifest(records)
    if manifest != expected:
        raise PacingSetError("manifest content disagrees with the supplied physical receipts")
    if data != canonical_document(expected):
        raise PacingSetError("manifest JSON is not canonical")
    return expected


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse the explicit build and verification modes."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    build = subparsers.add_parser("build", help="validate receipts and write one closed manifest")
    build.add_argument("--out", type=Path, required=True)
    build.add_argument("receipts", type=Path, nargs="+")
    verify = subparsers.add_parser("verify", help="verify a retained manifest and all receipts")
    verify.add_argument("manifest", type=Path)
    verify.add_argument("receipts", type=Path, nargs="+")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Build or verify one physical pacing set."""
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        records = [read_receipt(path) for path in args.receipts]
        if args.command == "build":
            manifest = build_manifest(records)
            write_manifest(args.out, manifest)
            print(f"physical pacing set pass: {args.out}")
            return 0
        verify_manifest(args.manifest, records)
        print(f"physical pacing set verified: {args.manifest}")
        return 0
    except (OSError, UnicodeError, PacingSetError) as error:
        print(f"physical pacing set failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
