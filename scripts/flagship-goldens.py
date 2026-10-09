#!/usr/bin/env python3
"""Flagship visual and room-bed audio golden regression (agent-and-machine track).

Renders deterministic CLI PNG plates and room-bed WAV files for the five
tactile flagships, then compares portable gates to a committed manifest.
PNG content hashes must match exactly. Each plate also prints a fixed-grid
luminance signature; the gate allows a maximum block delta of
APPEARANCE_MAX_DELTA. Exact CPU plates are distance 0. That tolerance is for
a future plate that is not byte-identical, and it does not relax the hash.
Room-bed audio gates on peak, RMS, size band, and a normalized spectral
fingerprint. SPECTRUM_ABSOLUTE_TOLERANCE is absolute per band because the
bands are already scaled so the loudest is 1. A non-finite band is a defect.
WAV SHA-256 is recorded as a
host reference only, because float paths can differ across OS targets.
Use --update after intentional product changes. An update refuses to replace
a PNG hash. This is machine regression evidence, not human sensory judgment,
and it does not classify diffs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent

# The gates share one way of getting the binaries they test; see gate_cli.py
# for why there is only one copy of it.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import resolve_cli  # noqa: E402

MANIFEST = ROOT / "docs" / "evidence" / "goldens" / "flagship-manifest.json"
GOLDEN_DIR = ROOT / "docs" / "evidence" / "goldens" / "flagship"
SCHEMA = "numinous-flagship-goldens-v3"
# Written gate. Do not widen it from the manifest: verify rejects a mismatch.
APPEARANCE_MAX_DELTA = 2
SPECTRUM_ABSOLUTE_TOLERANCE = 1e-3
# Two hex digits for each block of the fixed grid. Equal truncated signatures
# must not compare as the same plate.
APPEARANCE_HEX_LENGTH = 80
WIDTH = 64
HEIGHT = 40
FLAGSHIPS = (
    "times-tables",
    "double-pendulum",
    "game-of-life",
    "galton-board",
    "buffon-needle",
)
# Times Tables only: era matrix proves palette identity without exploding the set.
ERA_PROBE_ROOM = "times-tables"
ERAS = ("modern", "phosphor", "8bit", "vector")
SIGNAL_RE = re.compile(
    r"peak\s+(?P<peak>[-+0-9.eE]+),\s+RMS\s+(?P<rms>[-+0-9.eE]+)",
    re.IGNORECASE,
)
APPEARANCE_RE = re.compile(r"^Appearance: (?P<hex>[0-9a-f]+)$", re.MULTILINE)
SPECTRUM_RE = re.compile(
    r"spectrum\s+(?P<bands>[-+0-9.eE]+(?:\s+[-+0-9.eE]+)*)",
    re.IGNORECASE,
)




def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_metrics(path: Path) -> dict[str, float]:
    data = path.read_bytes()
    if len(data) < 32:
        raise RuntimeError(f"PNG too small: {path}")
    # Coarse whole-file stats keep the gate dependency-free (no image codec).
    mean = sum(data) / len(data)
    return {
        "bytes": float(len(data)),
        "mean_byte": round(mean, 6),
    }


def run_capture(command: list[str]) -> str:
    process = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode != 0:
        detail = (process.stderr or process.stdout or "").strip()
        raise RuntimeError(f"command failed ({process.returncode}): {detail[:800]}")
    return process.stdout + process.stderr


def capture_room(
    cli: list[str], room_id: str, work: Path, *, era: str = "modern", audio: bool = True
) -> dict[str, Any]:
    suffix = "" if era == "modern" else f"-{era}"
    png = work / f"{room_id}{suffix}.png"
    render_out = run_capture(
        [
            *cli,
            "render",
            room_id,
            "--width",
            str(WIDTH),
            "--height",
            str(HEIGHT),
            "--era",
            era,
            "--out",
            str(png),
        ]
    )
    if not png.is_file():
        raise RuntimeError(f"missing PNG for {room_id} era {era}")
    visual = png_metrics(png)
    entry: dict[str, Any] = {
        "id": room_id,
        "era": era,
        "width": WIDTH,
        "height": HEIGHT,
        "png_sha256": sha256_file(png),
        "png_bytes": int(visual["bytes"]),
        "png_mean_byte": visual["mean_byte"],
        "render_status_line": next(
            (line for line in render_out.splitlines() if line.startswith("Status:")),
            "",
        ),
    }
    appearance = APPEARANCE_RE.search(render_out)
    if appearance is None or len(appearance.group("hex")) != APPEARANCE_HEX_LENGTH:
        raise RuntimeError(f"missing appearance signature for {room_id} era {era}")
    entry["appearance"] = appearance.group("hex")
    if audio:
        wav = work / f"{room_id}.wav"
        sonify_out = run_capture(
            [
                *cli,
                "sonify",
                room_id,
                "--layer",
                "room-bed",
                "--out",
                str(wav),
            ]
        )
        if not wav.is_file():
            raise RuntimeError(f"missing artifacts for {room_id}")
        match = SIGNAL_RE.search(sonify_out)
        if match is None:
            raise RuntimeError(f"missing signal line for {room_id}: {sonify_out[-400:]}")
        entry["wav_sha256"] = sha256_file(wav)
        entry["wav_bytes"] = wav.stat().st_size
        entry["audio_peak"] = float(match.group("peak"))
        entry["audio_rms"] = float(match.group("rms"))
        spectrum = SPECTRUM_RE.search(sonify_out)
        if spectrum is None:
            raise RuntimeError(f"missing spectral fingerprint for {room_id}: {sonify_out[-400:]}")
        bands = [float(part) for part in spectrum.group("bands").split()]
        if not bands or any(not math.isfinite(band) for band in bands):
            raise RuntimeError(f"non-finite spectral fingerprint for {room_id}")
        entry["spectrum"] = bands
    return entry


def load_manifest() -> dict[str, Any]:
    if not MANIFEST.is_file():
        raise RuntimeError(f"missing golden manifest: {MANIFEST}")
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def compare_entry(expected: dict[str, Any], actual: dict[str, Any]) -> list[str]:
    """Compare goldens with portable rules.

    PNG content hashes must match exactly (they do across OS targets). Room-bed
    WAV *bytes* can differ across toolchains even when the declared signal
    metrics match, so audio gates on peak/RMS/size band rather than SHA-256.
    The manifest still records wav_sha256 from the update host as a reference.
    """
    defects: list[str] = []
    for key in ("png_sha256", "width", "height", "png_bytes"):
        if expected.get(key) != actual.get(key):
            defects.append(f"{key}: expected {expected.get(key)!r} got {actual.get(key)!r}")
    float_keys = [("png_mean_byte", 1e-6)]
    if "audio_peak" in expected:
        # Loose enough for cross-platform float paths; tight enough to catch
        # arrangement or gain regressions. Peak/RMS are the portable contract.
        float_keys.extend([("audio_peak", 1e-4), ("audio_rms", 1e-4)])
    for key, tol in float_keys:
        exp = float(expected[key])
        got = float(actual[key])
        if abs(exp - got) > tol * max(1.0, abs(exp)):
            defects.append(f"{key}: expected {exp} got {got}")
    if "wav_bytes" in expected:
        exp_b = int(expected["wav_bytes"])
        got_b = int(actual["wav_bytes"])
        # Same bed duration should keep size within a small absolute band.
        if abs(exp_b - got_b) > max(64, exp_b // 1000):
            defects.append(f"wav_bytes: expected ~{exp_b} got {got_b}")
    defects.extend(compare_appearance(expected, actual))
    if "audio_peak" in expected:
        defects.extend(compare_spectrum(expected, actual))
    return defects


def compare_appearance(expected: dict[str, Any], actual: dict[str, Any]) -> list[str]:
    """Maximum absolute block delta of the fixed-grid luminance signature."""
    left = expected.get("appearance")
    right = actual.get("appearance")
    if not isinstance(left, str) or not isinstance(right, str):
        return ["appearance: missing fixed-grid signature"]
    if len(left) != APPEARANCE_HEX_LENGTH or len(right) != APPEARANCE_HEX_LENGTH:
        return [
            "appearance: signature is not the fixed-grid length "
            f"({len(left)} and {len(right)})"
        ]
    distance = block_distance(left, right)
    if distance is None:
        return [f"appearance: malformed signature {left!r} vs {right!r}"]
    if distance > APPEARANCE_MAX_DELTA:
        return [
            f"appearance: max block delta {distance} exceeds {APPEARANCE_MAX_DELTA}"
        ]
    return []


def block_distance(left: str, right: str) -> int | None:
    if (
        not left
        or len(left) != len(right)
        or len(left) % 2 != 0
        or any(char not in "0123456789abcdef" for char in left + right)
    ):
        return None
    distance = 0
    for index in range(0, len(left), 2):
        distance = max(
            distance,
            abs(int(left[index:index + 2], 16) - int(right[index:index + 2], 16)),
        )
    return distance


def compare_spectrum(expected: dict[str, Any], actual: dict[str, Any]) -> list[str]:
    """Absolute per-band tolerance on the normalized whole-bed fingerprint."""
    left = expected.get("spectrum")
    right = actual.get("spectrum")
    if not isinstance(left, list) or not isinstance(right, list) or not left:
        return ["spectrum: missing fingerprint"]
    if len(left) != len(right):
        return [f"spectrum: expected {len(left)} bands got {len(right)}"]
    defects: list[str] = []
    for index, (exp, got) in enumerate(zip(left, right, strict=True)):
        try:
            expected_band = float(exp)
            actual_band = float(got)
        except (TypeError, ValueError):
            return [f"spectrum: band {index} was not a finite number"]
        if not math.isfinite(expected_band) or not math.isfinite(actual_band):
            return [f"spectrum: band {index} was not a finite number"]
        delta = abs(expected_band - actual_band)
        if delta > SPECTRUM_ABSOLUTE_TOLERANCE:
            defects.append(
                f"spectrum band {index}: expected {exp} got {got} "
                f"(absolute tolerance {SPECTRUM_ABSOLUTE_TOLERANCE})"
            )
    return defects


def entry_key(entry: dict[str, Any]) -> str:
    era = entry.get("era") or "modern"
    return f"{entry['id']}@{era}"


def keep_recorded_plate(entry: dict[str, Any], previous: dict[str, Any]) -> None:
    """Keep the committed plate bytes when the new render still matches them.

    The luminance signature and spectral fingerprint are added beside the
    existing hash. A PNG hash change is a rebaseline, and this update refuses
    it. Peak, RMS, and size stay on their existing tolerances; when they still
    pass, the recorded numbers stay so a host float does not churn the file.
    """
    name = entry_key(entry)
    if entry["png_sha256"] != previous["png_sha256"]:
        raise RuntimeError(
            f"refusing to rebaseline {name} PNG hash: "
            f"expected {previous['png_sha256']} got {entry['png_sha256']}"
        )
    for key in ("png_bytes", "png_mean_byte", "render_status_line"):
        if entry.get(key) != previous.get(key):
            raise RuntimeError(
                f"refusing to rebaseline {name} {key}: "
                f"expected {previous.get(key)!r} got {entry.get(key)!r}"
            )
    if "audio_peak" in previous:
        for key, tolerance in (("audio_peak", 1e-4), ("audio_rms", 1e-4)):
            expected = float(previous[key])
            actual = float(entry[key])
            if abs(expected - actual) > tolerance * max(1.0, abs(expected)):
                raise RuntimeError(
                    f"refusing to rebaseline {name} {key}: expected {expected} got {actual}"
                )
        expected_bytes = int(previous["wav_bytes"])
        actual_bytes = int(entry["wav_bytes"])
        if abs(expected_bytes - actual_bytes) > max(64, expected_bytes // 1000):
            raise RuntimeError(
                f"refusing to rebaseline {name} wav_bytes: "
                f"expected ~{expected_bytes} got {actual_bytes}"
            )
        for key in ("wav_sha256", "wav_bytes", "audio_peak", "audio_rms"):
            entry[key] = previous[key]


def update_goldens(cli: list[str]) -> dict[str, Any]:
    previous = {
        entry_key(room): room
        for room in (load_manifest().get("rooms", []) if MANIFEST.is_file() else [])
    }
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    rooms: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-goldens-") as tmp:
        work = Path(tmp)
        for room_id in FLAGSHIPS:
            entry = capture_room(cli, room_id, work, era="modern", audio=True)
            old = previous.get(entry_key(entry))
            if old is not None:
                keep_recorded_plate(entry, old)
            (GOLDEN_DIR / f"{room_id}.png").write_bytes((work / f"{room_id}.png").read_bytes())
            rooms.append(entry)
        for era in ERAS:
            if era == "modern":
                continue
            entry = capture_room(
                cli, ERA_PROBE_ROOM, work, era=era, audio=False
            )
            old = previous.get(entry_key(entry))
            if old is not None:
                keep_recorded_plate(entry, old)
            src = work / f"{ERA_PROBE_ROOM}-{era}.png"
            (GOLDEN_DIR / f"{ERA_PROBE_ROOM}-{era}.png").write_bytes(src.read_bytes())
            rooms.append(entry)
    # Eras must not collapse to the same plate for Times Tables.
    era_hashes = {
        entry["era"]: entry["png_sha256"]
        for entry in rooms
        if entry["id"] == ERA_PROBE_ROOM
    }
    if len(set(era_hashes.values())) != len(era_hashes):
        raise RuntimeError(f"era plates are not distinct: {era_hashes}")
    manifest = {
        "schemaVersion": SCHEMA,
        "evidenceClass": "agent-machine-regression",
        "appearanceMaxDelta": APPEARANCE_MAX_DELTA,
        "spectrumAbsoluteTolerance": SPECTRUM_ABSOLUTE_TOLERANCE,
        "width": WIDTH,
        "height": HEIGHT,
        "layer": "room-bed",
        "eras": list(ERAS),
        "rooms": rooms,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def require_manifest_contract(manifest: dict[str, Any]) -> None:
    """Fail closed when the file is an older schema or names a looser gate."""
    if manifest.get("schemaVersion") != SCHEMA:
        raise RuntimeError(
            f"flagship manifest schema {manifest.get('schemaVersion')!r} is not {SCHEMA}"
        )
    if manifest.get("appearanceMaxDelta") != APPEARANCE_MAX_DELTA:
        raise RuntimeError(
            "flagship manifest appearance tolerance does not match "
            f"{APPEARANCE_MAX_DELTA}"
        )
    if manifest.get("spectrumAbsoluteTolerance") != SPECTRUM_ABSOLUTE_TOLERANCE:
        raise RuntimeError(
            "flagship manifest spectrum tolerance does not match "
            f"{SPECTRUM_ABSOLUTE_TOLERANCE}"
        )


def verify_goldens(cli: list[str]) -> dict[str, Any]:
    manifest = load_manifest()
    require_manifest_contract(manifest)
    expected_rooms = {entry_key(room): room for room in manifest["rooms"]}
    defects: list[dict[str, Any]] = []
    actual_rooms: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-goldens-verify-") as tmp:
        work = Path(tmp)
        for room_id in FLAGSHIPS:
            actual = capture_room(cli, room_id, work, era="modern", audio=True)
            actual_rooms.append(actual)
            expected = expected_rooms.get(entry_key(actual))
            if expected is None:
                defects.append({"id": entry_key(actual), "defects": ["missing from manifest"]})
                continue
            room_defects = compare_entry(expected, actual)
            if room_defects:
                defects.append({"id": entry_key(actual), "defects": room_defects})
        for era in ERAS:
            if era == "modern":
                continue
            actual = capture_room(cli, ERA_PROBE_ROOM, work, era=era, audio=False)
            actual_rooms.append(actual)
            expected = expected_rooms.get(entry_key(actual))
            if expected is None:
                defects.append({"id": entry_key(actual), "defects": ["missing from manifest"]})
                continue
            room_defects = compare_entry(expected, actual)
            if room_defects:
                defects.append({"id": entry_key(actual), "defects": room_defects})
    return {
        "suite": "flagship-goldens",
        "passed": not defects,
        "room_count": len(actual_rooms),
        "defects": defects,
        "rooms": actual_rooms,
        "evidence_class": "agent-machine-regression",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--update",
        action="store_true",
        help="rewrite docs/evidence/goldens flagship artifacts and manifest",
    )
    args = parser.parse_args(argv)
    cli = resolve_cli()
    try:
        if args.update:
            manifest = update_goldens(cli)
            print(f"updated {MANIFEST} with {len(manifest['rooms'])} rooms")
            for room in manifest["rooms"]:
                era = room.get("era", "modern")
                line = f"  {room['id']}@{era}: png={room['png_sha256'][:12]}"
                if "wav_sha256" in room:
                    line += f" wav={room['wav_sha256'][:12]} peak={room['audio_peak']}"
                print(line)
            return 0
        summary = verify_goldens(cli)
    except RuntimeError as error:
        print(f"flagship-goldens: {error}", file=sys.stderr)
        return 1
    print(
        f"{summary['room_count'] - len(summary['defects'])}/{summary['room_count']} PASS"
    )
    for room in summary["rooms"]:
        era = room.get("era", "modern")
        line = f"  {room['id']}@{era}: png={room['png_sha256'][:12]}"
        if "wav_sha256" in room:
            line += f" wav={room['wav_sha256'][:12]} peak={room['audio_peak']}"
        print(line)
    for item in summary["defects"]:
        print(f"  FAIL  {item['id']}")
        for defect in item["defects"]:
            print(f"        {defect}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                "suite": summary["suite"],
                "passed": summary["passed"],
                "room_count": summary["room_count"],
                "defects": summary["defects"],
                "evidence_class": summary["evidence_class"],
            },
            sort_keys=True,
        )
    )
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
