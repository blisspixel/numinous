#!/usr/bin/env python3
"""Agent-and-machine soak: multi-room CLI walk under an isolated profile.

Renders, sonifies room beds, runs a short game open, and exercises forget
preview without erasing anything outside the temp profile. Exit non-zero on any
failure. Not a performance benchmark and not a human long-session claim.

The outputs are judged on what is in them, not on their size. A room bed is
uncompressed PCM, so its length depends on how long it plays and not at all on
whether it makes a sound: forty seconds of silence weighs the same two and a
half megabytes as forty seconds of music, and would have satisfied a check that
the file was over a thousand bytes. The peak and RMS the CLI already prints are
read instead, so the measurement is the product's own rather than a second one
written here. A picture is checked for being a real PNG of the size that was
asked for, which its header states.
"""

from __future__ import annotations

import json
import math
import os
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

# The signal line the CLI prints beneath a room-bed export, in the same shape
# `flagship-goldens.py` reads it.
SIGNAL_RE = re.compile(
    r"peak\s+(?P<peak>[-+0-9.eE]+),\s+RMS\s+(?P<rms>[-+0-9.eE]+)",
    re.IGNORECASE,
)

# Floors for a bed that is actually playing. Measured across the soak's own
# rooms, whose quietest sits at peak 0.133 and RMS 0.031, so these sit an order
# of magnitude below the real thing: they separate sound from silence, and are
# not a judgement about how a bed should be mixed.
MIN_PEAK = 0.01
MIN_RMS = 0.001

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "am-soak"

# The gates share one way of getting the binaries they test; see gate_cli.py
# for why there is only one copy of it.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import resolve_cli  # noqa: E402

# Stratified sample across wings and the five tactile flagships.
ROOMS = (
    "times-tables",
    "double-pendulum",
    "game-of-life",
    "galton-board",
    "buffon-needle",
    "lorenz",
    "mandelbrot",
    "cellular-automata",
    "lissajous",
    "golden-angle",
    "cult-of-pi",
    "conjecture-mill",
)


def run_cli(cli: list[str], args: list[str], env: dict[str, str]) -> tuple[int, str, str]:
    """Run one probe with its input already closed.

    Every command here is meant to be non-interactive, and a probe that
    inherits an open stdin can wait on it forever: `bench` plays gauntlets that
    read a line, so against a pipe nobody writes to it blocks until the timeout
    and the gate reports a hang instead of a result. That made this depend on
    how the caller was started, passing under a closed stdin and failing under
    an open one, which is the worst kind of gate to own.
    """
    process = subprocess.run(
        [*cli, *args],
        input="",
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        env=env,
        timeout=120,
    )
    return process.returncode, process.stdout, process.stderr


def png_complaint(path: Path, width: int, height: int) -> str | None:
    """Why this is not the picture that was asked for, or None if it is.

    Only the header is read. That is enough to tell a real PNG of the right
    size from a truncated write or an empty file, and it needs no decoder, so
    this cannot become a second and disagreeing implementation of how Numinous
    encodes an image. What the pixels contain is covered elsewhere: every room
    is checked for ink before encoding by `every_room_postcard_has_ink`, and
    the encoder itself by the committed flagship goldens.
    """
    if not path.is_file():
        return "no file was written"
    # Twenty four bytes off the handle, not the whole file. A render that went
    # wrong can leave something large behind, and there is no reason for a
    # check on the first two fields to read any of it.
    with path.open("rb") as handle:
        header = handle.read(24)
    if len(header) < 24 or not header.startswith(PNG_SIGNATURE):
        return f"not a PNG: first bytes were {header[:8]!r}"
    # Bytes 8 to 16 are the IHDR chunk header: a four byte length, then the
    # type. A real IHDR is always exactly 13 bytes long, so a different length
    # means these are not the dimensions and should not be read as them.
    length, kind = struct.unpack(">I4s", header[8:16])
    if kind != b"IHDR":
        return f"PNG did not open with IHDR: {kind!r}"
    if length != 13:
        return f"IHDR declares {length} bytes, not 13, so its fields cannot be trusted"
    actual = struct.unpack(">II", header[16:24])
    if actual != (width, height):
        return f"PNG says it is {actual[0]}x{actual[1]}, not {width}x{height}"
    return None


def bed_complaint(path: Path, report: str) -> str | None:
    """Why this room bed is not audible, or None if it is.

    Size proves nothing here. A room bed is uncompressed PCM, so its length is
    set by how long it plays; silence and music of the same duration weigh the
    same. The CLI measures the signal itself and prints it, so that measurement
    is read rather than recomputed, which also means this cannot drift away from
    what the product reports.
    """
    if not path.is_file():
        return "no file was written"
    match = SIGNAL_RE.search(report)
    if match is None:
        return f"the export reported no signal line to judge: {report[:200]!r}"
    # A floor only rejects numbers. Anything that is not one has to be caught
    # before the comparison, because the comparison would wave it through: an
    # exponent the pattern happily matches, like 1e999, becomes infinity, and
    # every "less than the floor" test on infinity or a NaN is false. That is
    # a check that cannot fail, which is the one kind not worth having.
    try:
        peak = float(match.group("peak"))
        rms = float(match.group("rms"))
    except ValueError:
        return (
            f"the signal line did not carry numbers: peak {match.group('peak')!r}, "
            f"RMS {match.group('rms')!r}"
        )
    if not (math.isfinite(peak) and math.isfinite(rms)):
        return f"the signal line was not finite: peak {peak}, RMS {rms}"
    if peak < MIN_PEAK or rms < MIN_RMS:
        return f"the bed is effectively silent: peak {peak}, RMS {rms}"
    return None


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    cli = resolve_cli()
    checks: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-am-soak-") as tmp:
        work = Path(tmp)
        profile = work / "profile"
        profile.mkdir()
        env = dict(os.environ)
        env.update(
            {
                "NUMINOUS_JOURNEY": str(profile / "journey.txt"),
                "NUMINOUS_SCORES": str(profile / "scores.txt"),
                "NUMINOUS_CAIRN": str(profile / "cairn.json"),
                "NUMINOUS_MUTE": "1",
            }
        )
        code, stdout, stderr = run_cli(cli, ["rooms"], env)
        # The detail used to read "catalog listed" whenever the command exited
        # zero, so a listing that came back empty reported the success sentence
        # while failing. A failure has to say what went wrong or it sends the
        # reader looking in the wrong place.
        if code != 0:
            listing = f"rooms exited {code}: {(stderr or stdout)[:200]}"
        elif "times-tables" not in (stdout + stderr):
            combined = (stdout + stderr).strip()
            listing = (
                "the catalog listing did not contain times-tables: "
                f"{combined[:200]!r}"
            )
        else:
            listing = "catalog listed"
        checks.append(
            {
                "name": "rooms_list",
                "passed": listing == "catalog listed",
                "detail": listing,
            }
        )
        for room_id in ROOMS:
            png = work / f"{room_id}.png"
            wav = work / f"{room_id}.wav"
            code_r, out_r, err_r = run_cli(
                cli,
                [
                    "render",
                    room_id,
                    "--width",
                    "120",
                    "--height",
                    "80",
                    "--out",
                    str(png),
                ],
                env,
            )
            png_reason = (
                f"render exited {code_r}: {(err_r or out_r)[:200]}"
                if code_r != 0
                else png_complaint(png, 120, 80)
            )
            checks.append(
                {
                    "name": f"render:{room_id}",
                    "passed": png_reason is None,
                    "detail": png_reason or "png ok, 120x80 by its own header",
                }
            )
            code_s, out_s, err_s = run_cli(
                cli,
                ["sonify", room_id, "--layer", "room-bed", "--out", str(wav)],
                env,
            )
            wav_reason = (
                f"sonify exited {code_s}: {(err_s or out_s)[:200]}"
                if code_s != 0
                else bed_complaint(wav, out_s)
            )
            checks.append(
                {
                    "name": f"sonify:{room_id}",
                    "passed": wav_reason is None,
                    "detail": wav_reason or "wav ok and the bed is audible",
                }
            )
        # Prefer non-interactive surfaces. Games that wait on stdin are covered
        # by pure core tests and MCP tools, not this soak.
        for cmd in (
            ["describe", "times-tables"],
            ["sims"],
            ["plot", "--list-recipes"],
            ["bench"],
        ):
            code_g, out_g, err_g = run_cli(cli, list(cmd), env)
            text = out_g + err_g
            ok_g = code_g == 0 and len(text.strip()) > 20
            checks.append(
                {
                    "name": f"cli:{'-'.join(cmd[:2])}",
                    "passed": ok_g,
                    "detail": "ok" if ok_g else (err_g or out_g)[:200],
                }
            )
        code_f, out_f, err_f = run_cli(cli, ["forget"], env)
        checks.append(
            {
                "name": "forget_preview",
                "passed": code_f == 0,
                "detail": "preview ok" if code_f == 0 else (err_f or out_f)[:200],
            }
        )

    failed = [c for c in checks if not c["passed"]]
    summary = {
        "suite": "am-soak",
        "passed": not failed,
        "check_count": len(checks),
        "failed_count": len(failed),
        "failed": [{"name": c["name"], "detail": c["detail"]} for c in failed],
        "rooms": list(ROOMS),
        "evidence_class": "agent-machine-soak",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{summary['check_count'] - summary['failed_count']}/{summary['check_count']} PASS")
    for item in checks:
        if item["passed"] and item["name"].startswith(("render:", "sonify:")):
            continue
        mark = "PASS" if item["passed"] else "FAIL"
        print(f"  {mark}  {item['name']}: {item['detail']}")
    if failed:
        for item in failed:
            if item["name"].startswith(("render:", "sonify:")):
                print(f"  FAIL  {item['name']}: {item['detail']}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                "suite": summary["suite"],
                "passed": summary["passed"],
                "check_count": summary["check_count"],
                "failed_count": summary["failed_count"],
                "failed": summary["failed"],
                "evidence_class": summary["evidence_class"],
            },
            sort_keys=True,
        )
    )
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
