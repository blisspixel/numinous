#!/usr/bin/env python3
"""Agent-and-machine soak: multi-room CLI walk under an isolated profile.

Renders, sonifies room beds, runs a short game open, and exercises forget
preview without erasing anything outside the temp profile. Exit non-zero on any
failure. Not a performance benchmark and not a human long-session claim.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "am-soak"

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


def resolve_cli() -> list[str]:
    candidates = [
        ROOT / "target" / "debug" / "numinous.exe",
        ROOT / "target" / "debug" / "numinous",
        ROOT / "target" / "release" / "numinous.exe",
        ROOT / "target" / "release" / "numinous",
    ]
    for path in candidates:
        if path.is_file():
            return [str(path)]
    return ["cargo", "run", "--quiet", "--locked", "--bin", "numinous", "--"]


def run_cli(cli: list[str], args: list[str], env: dict[str, str]) -> tuple[int, str, str]:
    process = subprocess.run(
        [*cli, *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        env=env,
        timeout=120,
    )
    return process.returncode, process.stdout, process.stderr


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
        checks.append(
            {
                "name": "rooms_list",
                "passed": code == 0 and "times-tables" in (stdout + stderr),
                "detail": "catalog listed" if code == 0 else (stderr or stdout)[:200],
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
            ok_r = code_r == 0 and png.is_file() and png.stat().st_size > 0
            checks.append(
                {
                    "name": f"render:{room_id}",
                    "passed": ok_r,
                    "detail": "png ok" if ok_r else (err_r or out_r)[:200],
                }
            )
            code_s, out_s, err_s = run_cli(
                cli,
                ["sonify", room_id, "--layer", "room-bed", "--out", str(wav)],
                env,
            )
            ok_s = code_s == 0 and wav.is_file() and wav.stat().st_size > 1000
            checks.append(
                {
                    "name": f"sonify:{room_id}",
                    "passed": ok_s,
                    "detail": "wav ok" if ok_s else (err_s or out_s)[:200],
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
