#!/usr/bin/env python3
"""Machine acceptance for reduced motion in the terminal face (0.5-am).

Numinous animates by advancing a phase through [0, 1). Reduced motion stops
that advance and nothing else: the picture holds still, and the player's own
input is what moves it. This gate proves both halves, because either one alone
would be a defect. A view that never moves is not reduced motion, it is a
broken view; a view that still drifts is not reduced motion either.

Each live loop is run twice against the real binary, once ordinary and once
with NUMINOUS_REDUCED_MOTION set, and the emitted frames are compared. A frame
is whatever sits between two cursor-home markers, so the leading screen-clear
and the truncated tail are excluded by construction rather than by guesswork.

This is machine evidence for the CLI only. The App and MCP faces, mono audio,
photosensitivity budgets, and any human accessibility session remain separate
and open.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "reduced-motion"

# Enough bytes to hold many frames of the widest supported view.
READ_LIMIT = 400_000
CHUNK = 8192

# Each probe: a label, the argv after the binary, and the marker that starts a
# frame in that loop's output.
PROBES: tuple[tuple[str, list[str], str], ...] = (
    ("watch", ["watch", "lorenz"], "\x1b[H"),
    ("watch-times-tables", ["watch", "times-tables"], "\x1b[H"),
    ("play", ["play", "lorenz"], "\x1b[2J\x1b[H"),
)


def resolve_cli() -> list[str]:
    """Build the CLI, then return the binary that build produced.

    This gate observes live behaviour, so it must observe the behaviour of the
    current source. Picking whichever binary happens to be on disk would let a
    stale artifact answer for code that no longer exists, and the gate would
    pass while the feature was broken. Cargo is incremental, so this costs
    almost nothing when the tree is already built.
    """
    build = subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "numinous"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        raise SystemExit("cannot build the CLI under test:\n" + build.stderr)
    for name in ("numinous.exe", "numinous"):
        path = ROOT / "target" / "debug" / name
        if path.is_file():
            return [str(path)]
    raise SystemExit("cargo build reported success but produced no numinous binary")


def whole_frames(cli: list[str], args: list[str], reduced: bool, marker: str) -> list[str]:
    """Complete frames emitted by one live loop, bounded in time and bytes."""
    env = dict(os.environ)
    env.pop("NUMINOUS_REDUCED_MOTION", None)
    # Colour is irrelevant here and only makes frames larger to compare.
    env["NO_COLOR"] = "1"
    if reduced:
        env["NUMINOUS_REDUCED_MOTION"] = "1"
    proc = subprocess.Popen(
        cli + args,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        env=env,
    )
    data = b""
    try:
        assert proc.stdout is not None
        while len(data) < READ_LIMIT:
            chunk = proc.stdout.read(CHUNK)
            if not chunk:
                break
            data += chunk
    finally:
        proc.kill()
        proc.wait()
    parts = data.decode("utf-8", "replace").split(marker)
    if len(parts) <= 2:
        return []
    # Adjacent markers can yield empty splits; those are not frames.
    return [frame for frame in parts[1:-1] if frame]


def check(cli: list[str], label: str, args: list[str], marker: str) -> dict[str, Any]:
    moving = whole_frames(cli, args, reduced=False, marker=marker)
    held = whole_frames(cli, args, reduced=True, marker=marker)
    moving_distinct = len(set(moving))
    held_distinct = len(set(held))

    reasons = []
    if len(moving) < 2:
        reasons.append(f"captured only {len(moving)} ordinary frames, need at least 2")
    if len(held) < 2:
        reasons.append(f"captured only {len(held)} reduced frames, need at least 2")
    if moving_distinct < 2:
        reasons.append("ordinary motion did not animate, so the comparison proves nothing")
    if held_distinct != 1:
        reasons.append(f"reduced motion still changed: {held_distinct} distinct frames")
    if held and not held[0].strip():
        reasons.append("reduced motion held a blank frame rather than the picture")

    return {
        "name": label,
        "args": args,
        "passed": not reasons,
        "ordinary_frames": len(moving),
        "ordinary_distinct": moving_distinct,
        "reduced_frames": len(held),
        "reduced_distinct": held_distinct,
        "detail": "; ".join(reasons) if reasons else "ordinary animates, reduced holds still",
    }


def main() -> int:
    cli = resolve_cli()
    OUT.mkdir(parents=True, exist_ok=True)
    results = [check(cli, label, args, marker) for label, args, marker in PROBES]
    failed = [item for item in results if not item["passed"]]
    summary = {
        "suite": "reduced-motion",
        "passed": not failed,
        "check_count": len(results),
        "failed_count": len(failed),
        "results": results,
        "evidence_class": "agent-machine",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{summary['check_count'] - summary['failed_count']}/{summary['check_count']} PASS")
    for item in results:
        mark = "PASS" if item["passed"] else "FAIL"
        print(
            f"  {mark}  {item['name']}: ordinary {item['ordinary_distinct']} distinct "
            f"of {item['ordinary_frames']}, reduced {item['reduced_distinct']} "
            f"of {item['reduced_frames']}. {item['detail']}"
        )
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                "suite": summary["suite"],
                "passed": summary["passed"],
                "check_count": summary["check_count"],
                "failed_count": summary["failed_count"],
                "evidence_class": summary["evidence_class"],
            },
            sort_keys=True,
        )
    )
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
