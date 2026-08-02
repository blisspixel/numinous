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
# Times Tables only: era matrix proves palette identity without exploding the set.
ERA_PROBE_ROOM = "times-tables"
ERAS = ("modern", "phosphor", "8bit", "vector")
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
    return entry


def load_manifest() -> dict[str, Any]:
    if not MANIFEST.is_file():
        raise RuntimeError(f"missing golden manifest: {MANIFEST}")
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def compare_entry(expected: dict[str, Any], actual: dict[str, Any]) -> list[str]:
    defects: list[str] = []
    keys = ["png_sha256", "width", "height", "png_bytes"]
    if "wav_sha256" in expected:
        keys.extend(["wav_sha256", "wav_bytes"])
    for key in keys:
        if expected.get(key) != actual.get(key):
            defects.append(f"{key}: expected {expected.get(key)!r} got {actual.get(key)!r}")
    # Tiny float guards for metric drift while hashes remain authoritative.
    float_keys = [("png_mean_byte", 1e-6)]
    if "audio_peak" in expected:
        float_keys.extend([("audio_peak", 1e-9), ("audio_rms", 1e-9)])
    for key, tol in float_keys:
        exp = float(expected[key])
        got = float(actual[key])
        if abs(exp - got) > tol * max(1.0, abs(exp)):
            defects.append(f"{key}: expected {exp} got {got}")
    return defects


def entry_key(entry: dict[str, Any]) -> str:
    era = entry.get("era") or "modern"
    return f"{entry['id']}@{era}"


def update_goldens(cli: list[str]) -> dict[str, Any]:
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    rooms: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-goldens-") as tmp:
        work = Path(tmp)
        for room_id in FLAGSHIPS:
            entry = capture_room(cli, room_id, work, era="modern", audio=True)
            (GOLDEN_DIR / f"{room_id}.png").write_bytes((work / f"{room_id}.png").read_bytes())
            rooms.append(entry)
        for era in ERAS:
            if era == "modern":
                continue
            entry = capture_room(
                cli, ERA_PROBE_ROOM, work, era=era, audio=False
            )
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
        "schemaVersion": "numinous-flagship-goldens-v2",
        "evidenceClass": "agent-machine-regression",
        "width": WIDTH,
        "height": HEIGHT,
        "layer": "room-bed",
        "eras": list(ERAS),
        "rooms": rooms,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def verify_goldens(cli: list[str]) -> dict[str, Any]:
    manifest = load_manifest()
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
