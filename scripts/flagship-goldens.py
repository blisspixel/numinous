#!/usr/bin/env python3
"""Flagship visual and room-bed audio golden regression (agent-and-machine track).

Renders deterministic CLI PNG plates and room-bed WAV files for the five
tactile flagships, then compares content hashes and coarse signal metrics to a
committed manifest. Use --update to rewrite goldens after intentional product
changes. This is machine regression evidence, not human sensory judgment.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "docs" / "evidence" / "goldens" / "flagship-manifest.json"
GOLDEN_DIR = ROOT / "docs" / "evidence" / "goldens" / "flagship"
WIDTH = 64
HEIGHT = 40
FLAGSHIPS = (
    "times-tables",
    "double-pendulum",
    "game-of-life",
    "galton-board",
    "buffon-needle",
)
SIGNAL_RE = re.compile(
    r"peak\s+(?P<peak>[-+0-9.eE]+),\s+RMS\s+(?P<rms>[-+0-9.eE]+)",
    re.IGNORECASE,
)


def resolve_cli() -> list[str]:
    """Prefer a built binary; fall back to cargo run."""
    candidates = [
        ROOT / "target" / "debug" / "numinous.exe",
        ROOT / "target" / "debug" / "numinous",
        ROOT / "target" / "release" / "numinous.exe",
        ROOT / "target" / "release" / "numinous",
    ]
    for path in candidates:
        if path.is_file():
            return [str(path)]
    return [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "--bin",
        "numinous",
        "--",
    ]


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


def capture_room(cli: list[str], room_id: str, work: Path) -> dict[str, Any]:
    png = work / f"{room_id}.png"
    wav = work / f"{room_id}.wav"
    render_out = run_capture(
        [
            *cli,
            "render",
            room_id,
            "--width",
            str(WIDTH),
            "--height",
            str(HEIGHT),
            "--out",
            str(png),
        ]
    )
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
    if not png.is_file() or not wav.is_file():
        raise RuntimeError(f"missing artifacts for {room_id}")
    match = SIGNAL_RE.search(sonify_out)
    if match is None:
        raise RuntimeError(f"missing signal line for {room_id}: {sonify_out[-400:]}")
    peak = float(match.group("peak"))
    rms = float(match.group("rms"))
    visual = png_metrics(png)
    return {
        "id": room_id,
        "width": WIDTH,
        "height": HEIGHT,
        "png_sha256": sha256_file(png),
        "wav_sha256": sha256_file(wav),
        "png_bytes": int(visual["bytes"]),
        "png_mean_byte": visual["mean_byte"],
        "wav_bytes": wav.stat().st_size,
        "audio_peak": peak,
        "audio_rms": rms,
        "render_status_line": next(
            (line for line in render_out.splitlines() if line.startswith("Status:")),
            "",
        ),
    }


def load_manifest() -> dict[str, Any]:
    if not MANIFEST.is_file():
        raise RuntimeError(f"missing golden manifest: {MANIFEST}")
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def compare_entry(expected: dict[str, Any], actual: dict[str, Any]) -> list[str]:
    defects: list[str] = []
    for key in ("png_sha256", "wav_sha256", "width", "height", "png_bytes", "wav_bytes"):
        if expected.get(key) != actual.get(key):
            defects.append(f"{key}: expected {expected.get(key)!r} got {actual.get(key)!r}")
    # Tiny float guards for metric drift while hashes remain authoritative.
    for key, tol in (("png_mean_byte", 1e-6), ("audio_peak", 1e-9), ("audio_rms", 1e-9)):
        exp = float(expected[key])
        got = float(actual[key])
        if abs(exp - got) > tol * max(1.0, abs(exp)):
            defects.append(f"{key}: expected {exp} got {got}")
    return defects


def update_goldens(cli: list[str]) -> dict[str, Any]:
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    rooms: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-goldens-") as tmp:
        work = Path(tmp)
        for room_id in FLAGSHIPS:
            entry = capture_room(cli, room_id, work)
            # Persist compact PNG plates for offline visual review. WAV bytes stay
            # out of the repository; the manifest binds exact content hashes.
            (GOLDEN_DIR / f"{room_id}.png").write_bytes((work / f"{room_id}.png").read_bytes())
            rooms.append(entry)
    manifest = {
        "schemaVersion": "numinous-flagship-goldens-v1",
        "evidenceClass": "agent-machine-regression",
        "width": WIDTH,
        "height": HEIGHT,
        "layer": "room-bed",
        "rooms": rooms,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def verify_goldens(cli: list[str]) -> dict[str, Any]:
    manifest = load_manifest()
    expected_rooms = {room["id"]: room for room in manifest["rooms"]}
    defects: list[dict[str, Any]] = []
    actual_rooms: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-goldens-verify-") as tmp:
        work = Path(tmp)
        for room_id in FLAGSHIPS:
            actual = capture_room(cli, room_id, work)
            actual_rooms.append(actual)
            expected = expected_rooms.get(room_id)
            if expected is None:
                defects.append({"id": room_id, "defects": ["missing from manifest"]})
                continue
            room_defects = compare_entry(expected, actual)
            if room_defects:
                defects.append({"id": room_id, "defects": room_defects})
    return {
        "suite": "flagship-goldens",
        "passed": not defects,
        "room_count": len(FLAGSHIPS),
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
                print(
                    f"  {room['id']}: png={room['png_sha256'][:12]} "
                    f"wav={room['wav_sha256'][:12]} peak={room['audio_peak']}"
                )
            return 0
        summary = verify_goldens(cli)
    except RuntimeError as error:
        print(f"flagship-goldens: {error}", file=sys.stderr)
        return 1
    print(
        f"{summary['room_count'] - len(summary['defects'])}/{summary['room_count']} PASS"
    )
    for room in summary["rooms"]:
        print(
            f"  {room['id']}: png={room['png_sha256'][:12]} "
            f"wav={room['wav_sha256'][:12]} peak={room['audio_peak']}"
        )
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
