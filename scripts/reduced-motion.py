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
# These loops never exit on their own, so the probe ends on a deadline. Long
# enough for a slow machine to emit dozens of frames, short enough that a
# stalled child cannot hold CI open.
DEADLINE_SECONDS = 6.0
# Every probe needs at least this many complete frames before its result means
# anything. Below it the run is inconclusive, which is a different failure from
# the feature being broken, and the report must not confuse the two.
MIN_FRAMES = 4

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
    # CARGO_TARGET_DIR redirects where cargo writes, and several CI layouts set
    # it. Looking only under ROOT/target would fail a build that succeeded.
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR") or (ROOT / "target"))
    for name in ("numinous.exe", "numinous"):
        path = target_dir / "debug" / name
        if path.is_file():
            return [str(path)]
    raise SystemExit(
        "cargo build reported success but no numinous binary is under "
        f"{target_dir / 'debug'}"
    )


def whole_frames(cli: list[str], args: list[str], reduced: bool, marker: str) -> list[str]:
    """Complete frames emitted by one live loop, bounded in time and in bytes.

    The loops under test never exit, so the probe always ends by deadline. A
    plain read loop would block forever on a child that stalls without closing
    its pipe, which would hang the gate rather than fail it.
    """
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
    try:
        data, _ = proc.communicate(timeout=DEADLINE_SECONDS)
    except subprocess.TimeoutExpired:
        # Expected: these loops run until stopped. Kill, then collect what the
        # pipe already holds.
        proc.kill()
        data, _ = proc.communicate()
    parts = data[:READ_LIMIT].decode("utf-8", "replace").split(marker)
    if len(parts) <= 2:
        return []
    # Adjacent markers can yield empty splits; those are not frames.
    return [frame for frame in parts[1:-1] if frame]


def check(cli: list[str], label: str, args: list[str], marker: str) -> dict[str, Any]:
    moving = whole_frames(cli, args, reduced=False, marker=marker)
    held = whole_frames(cli, args, reduced=True, marker=marker)
    moving_distinct = len(set(moving))
    held_distinct = len(set(held))

    # Capture failures and behaviour failures are different problems, and a
    # report that blurs them sends the reader to the wrong place. Only judge
    # behaviour once there is enough evidence to judge it on.
    reasons = []
    captured = True
    if len(moving) < MIN_FRAMES:
        reasons.append(
            f"captured only {len(moving)} ordinary frames, need {MIN_FRAMES}; "
            "the run is inconclusive rather than failing"
        )
        captured = False
    if len(held) < MIN_FRAMES:
        reasons.append(
            f"captured only {len(held)} reduced frames, need {MIN_FRAMES}; "
            "the run is inconclusive rather than failing"
        )
        captured = False
    if captured:
        if moving_distinct < 2:
            reasons.append("ordinary motion did not animate, so the comparison proves nothing")
        if held_distinct != 1:
            reasons.append(f"reduced motion still changed: {held_distinct} distinct frames")
        if not held[0].strip():
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
