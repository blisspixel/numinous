#!/usr/bin/env python3
"""Machine catalog scorecard: stratified wing smoke via CLI.

For every wing, render and describe a sample of rooms. Fail closed on empty
status, missing PNG, or non-zero exit. This is keep/cut engineering signal for
the am-track, not stranger beauty judgment.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "catalog-scorecard"
PER_WING = 3
ALWAYS = (
    "times-tables",
    "double-pendulum",
    "game-of-life",
    "galton-board",
    "buffon-needle",
)


def resolve_cli() -> list[str]:
    for path in (
        ROOT / "target" / "debug" / "numinous.exe",
        ROOT / "target" / "debug" / "numinous",
        ROOT / "target" / "release" / "numinous.exe",
        ROOT / "target" / "release" / "numinous",
    ):
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
        timeout=90,
    )
    return process.returncode, process.stdout, process.stderr


def list_catalog(cli: list[str], env: dict[str, str]) -> list[tuple[str, str]]:
    code, stdout, stderr = run_cli(cli, ["rooms"], env)
    if code != 0:
        raise RuntimeError(f"rooms failed: {(stderr or stdout)[:400]}")
    rows: list[tuple[str, str]] = []
    for line in stdout.splitlines():
        line = line.rstrip()
        if not line or line.lower().startswith("id") or set(line) <= {"-", " "}:
            continue
        # CLI prints: <id> <wing...> <title...> with variable spacing.
        # The room id is always the first whitespace-separated token (a slug).
        parts = line.split(None, 1)
        if not parts:
            continue
        room_id = parts[0].strip()
        if not room_id or "/" in room_id or " " in room_id:
            continue
        rest = parts[1] if len(parts) > 1 else "unknown"
        # Wing ends at a run of 2+ spaces before the title when present.
        if "  " in rest:
            wing = rest.split("  ", 1)[0].strip()
        else:
            # Fall back: take the first two tokens of the wing name.
            wing_tokens = rest.split()
            wing = " ".join(wing_tokens[:3]) if wing_tokens else "unknown"
        rows.append((room_id, wing or "unknown"))
    if len(rows) < 50:
        raise RuntimeError(f"catalog too small: {len(rows)} rows")
    # Unique ids only.
    seen: set[str] = set()
    unique: list[tuple[str, str]] = []
    for room_id, wing in rows:
        if room_id in seen:
            continue
        seen.add(room_id)
        unique.append((room_id, wing))
    return unique


def select_rooms(rows: list[tuple[str, str]]) -> list[str]:
    by_wing: dict[str, list[str]] = defaultdict(list)
    for room_id, wing in rows:
        by_wing[wing].append(room_id)
    selected: list[str] = []
    seen: set[str] = set()
    for room_id in ALWAYS:
        if room_id not in seen:
            selected.append(room_id)
            seen.add(room_id)
    for wing in sorted(by_wing):
        for room_id in by_wing[wing][:PER_WING]:
            if room_id not in seen:
                selected.append(room_id)
                seen.add(room_id)
    return selected


def probe_room(
    cli: list[str], room_id: str, work: Path, env: dict[str, str]
) -> dict[str, Any]:
    png = work / f"{room_id}.png"
    code_r, out_r, err_r = run_cli(
        cli,
        ["render", room_id, "--width", "80", "--height", "48", "--out", str(png)],
        env,
    )
    code_d, out_d, err_d = run_cli(cli, ["describe", room_id], env)
    status_line = next(
        (line for line in (out_r + err_r).splitlines() if line.startswith("Status:")),
        "",
    )
    status = status_line.removeprefix("Status:").strip()
    describe_text = (out_d + err_d).strip()
    defects: list[str] = []
    if code_r != 0 or not png.is_file() or png.stat().st_size == 0:
        defects.append(f"render failed: {(err_r or out_r)[:160]}")
    if not status:
        defects.append("empty status on render")
    if code_d != 0 or len(describe_text) < 20:
        defects.append(f"describe failed: {(err_d or out_d)[:160]}")
    return {
        "id": room_id,
        "passed": not defects,
        "status": status[:80],
        "png_bytes": png.stat().st_size if png.is_file() else 0,
        "defects": defects,
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    cli = resolve_cli()
    env = dict(os.environ)
    env["NUMINOUS_MUTE"] = "1"
    try:
        rows = list_catalog(cli, env)
    except RuntimeError as error:
        print(f"catalog-scorecard: {error}", file=sys.stderr)
        return 1
    selected = select_rooms(rows)
    results: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-scorecard-") as tmp:
        work = Path(tmp)
        for room_id in selected:
            results.append(probe_room(cli, room_id, work, env))
    failed = [item for item in results if not item["passed"]]
    wings = sorted({wing for _, wing in rows})
    summary = {
        "suite": "catalog-scorecard",
        "passed": not failed,
        "catalog_count": len(rows),
        "wing_count": len(wings),
        "sampled": len(results),
        "failed_count": len(failed),
        "failed": [{"id": f["id"], "defects": f["defects"]} for f in failed],
        "wings": wings,
        "evidence_class": "agent-machine-catalog",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(
        f"catalog={summary['catalog_count']} wings={summary['wing_count']} "
        f"sampled={summary['sampled']} "
        f"{summary['sampled'] - summary['failed_count']}/{summary['sampled']} PASS"
    )
    for item in results:
        if item["passed"]:
            continue
        print(f"  FAIL  {item['id']}: {item['defects']}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                "suite": summary["suite"],
                "passed": summary["passed"],
                "catalog_count": summary["catalog_count"],
                "sampled": summary["sampled"],
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
