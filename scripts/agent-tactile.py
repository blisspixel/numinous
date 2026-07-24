#!/usr/bin/env python3
"""Agent tactile review of the 0.3 five-flagship cohort over MCP.

Machine evidence for discoverable action and hand consequence on Times Tables,
Double Pendulum, Game of Life, Galton Board, and Formula Jam (Studio plot).
Not a human stranger gate. Writes notes under .agent/tester-cohort/.
"""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "scripts" / "mcp-play.py"
OUT = ROOT / ".agent" / "tester-cohort" / "round-08-tactile-0.3"


@dataclass(frozen=True)
class Probe:
    slug: str
    title: str
    open_tool: str
    open_args: dict[str, Any]
    hand_tool: str
    hand_args: dict[str, Any]
    invite_tokens: tuple[str, ...]
    expect_status_change: bool


PROBES = [
    Probe(
        slug="times-tables",
        title="Times Tables",
        open_tool="play_room",
        open_args={"id": "times-tables", "t": 0.15, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "times-tables",
            "t": 0.15,
            "width": 56,
            "height": 28,
            "pokes": [[0.72, 0.50]],
        },
        invite_tokens=("TURN", "DIAL", "K=", "DRAG", "CLICK"),
        expect_status_change=True,
    ),
    Probe(
        slug="double-pendulum",
        title="Double Pendulum",
        open_tool="play_room",
        open_args={"id": "double-pendulum", "t": 0.25, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "double-pendulum",
            "t": 0.25,
            "width": 56,
            "height": 28,
            "pokes": [[0.30, 0.28]],
        },
        invite_tokens=("CLICK", "RE-DROP", "TWINS"),
        expect_status_change=True,
    ),
    Probe(
        slug="game-of-life",
        title="Game of Life",
        open_tool="play_room",
        open_args={"id": "game-of-life", "t": 0.2, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "game-of-life",
            "t": 0.2,
            "width": 56,
            "height": 28,
            "pokes": [[0.45, 0.45]],
        },
        invite_tokens=("CLICK", "GLIDER", "PLACE", "LIFE", "GEN"),
        expect_status_change=True,
    ),
    Probe(
        slug="galton-board",
        title="Galton Board",
        open_tool="play_room",
        open_args={"id": "galton-board", "t": 0.2, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "galton-board",
            "t": 0.2,
            "width": 56,
            "height": 28,
            "pokes": [[0.20, 0.50]],
        },
        invite_tokens=("CLICK", "DROP", "PICK", "COIN", "BALL", "p="),
        expect_status_change=True,
    ),
    Probe(
        slug="formula-jam",
        title="Formula Jam (Studio plot)",
        open_tool="plot_expression",
        open_args={"expr": "sin(x)"},
        hand_tool="plot_expression",
        hand_args={"expr": "sin(2*x)"},
        invite_tokens=(),
        expect_status_change=True,
    ),
]


def call_tool(tool: str, arguments: dict[str, Any]) -> dict[str, Any]:
    payload = json.dumps(arguments)
    process = subprocess.run(
        [sys.executable, str(DRIVER), "call", tool, payload],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode != 0:
        return {
            "ok": False,
            "stderr": process.stderr.strip(),
            "stdout": process.stdout.strip(),
            "code": process.returncode,
        }
    text = process.stdout
    structured = None
    if "--- structuredContent ---" in text:
        body, _, tail = text.partition("--- structuredContent ---")
        try:
            structured = json.loads(tail.strip())
        except json.JSONDecodeError:
            structured = None
        text = body.strip()
    return {"ok": True, "text": text, "structured": structured}


def status_of(result: dict[str, Any]) -> str:
    structured = result.get("structured") or {}
    for key in ("status", "readout", "message", "expression"):
        value = structured.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    text = result.get("text") or ""
    return text.strip()[:120]


def plate_fingerprint(result: dict[str, Any]) -> str:
    structured = result.get("structured") or {}
    for key in ("plate", "frame", "ascii", "plot"):
        value = structured.get(key)
        if isinstance(value, str) and value.strip():
            return value
    return result.get("text") or ""


def review_probe(probe: Probe) -> dict[str, Any]:
    open_result = call_tool(probe.open_tool, probe.open_args)
    hand_result = call_tool(probe.hand_tool, probe.hand_args)
    open_status = status_of(open_result)
    hand_status = status_of(hand_result)
    open_plate = plate_fingerprint(open_result)
    hand_plate = plate_fingerprint(hand_result)

    defects: list[str] = []
    if not open_result.get("ok"):
        defects.append(f"open failed: {open_result.get('stderr') or open_result.get('stdout')}")
    if not hand_result.get("ok"):
        defects.append(f"hand failed: {hand_result.get('stderr') or hand_result.get('stdout')}")

    if probe.invite_tokens and open_result.get("ok"):
        upper = open_status.upper()
        if not any(token.upper() in upper for token in probe.invite_tokens):
            # Fall back to action field when status is ambient-only.
            action = ""
            structured = open_result.get("structured") or {}
            if isinstance(structured.get("action"), str):
                action = structured["action"]
            combined = f"{open_status} {action}".upper()
            if not any(token.upper() in combined for token in probe.invite_tokens):
                defects.append(
                    f"first contact missing invite tokens {probe.invite_tokens}: "
                    f"status={open_status!r} action={action!r}"
                )

    if probe.expect_status_change and open_result.get("ok") and hand_result.get("ok"):
        if open_status == hand_status and open_plate == hand_plate:
            defects.append("hand left status and plate unchanged")

    if open_status and len(open_status) > 56 and probe.open_tool == "play_room":
        defects.append(f"open status longer than 56 chars: {open_status!r}")
    if hand_status and len(hand_status) > 56 and probe.hand_tool == "play_room":
        defects.append(f"hand status longer than 56 chars: {hand_status!r}")

    passed = not defects
    return {
        "slug": probe.slug,
        "title": probe.title,
        "pass": passed,
        "defects": defects,
        "open_status": open_status,
        "hand_status": hand_status,
        "open_ok": bool(open_result.get("ok")),
        "hand_ok": bool(hand_result.get("ok")),
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    results = [review_probe(probe) for probe in PROBES]
    passed = sum(1 for item in results if item["pass"])
    failed = len(results) - passed

    summary_lines = [
        f"# Tactile agent cohort (0.3 five flagships)",
        "",
        f"Stamp: {stamp}",
        f"Result: {passed}/{len(results)} PASS, {failed} FAIL",
        "Evidence class: agent/MCP machine review, not human stranger hallway.",
        "",
    ]
    for item in results:
        mark = "PASS" if item["pass"] else "FAIL"
        summary_lines.append(f"## {item['title']} ({item['slug']})  {mark}")
        summary_lines.append(f"- open: `{item['open_status']}`")
        summary_lines.append(f"- hand: `{item['hand_status']}`")
        if item["defects"]:
            for defect in item["defects"]:
                summary_lines.append(f"- defect: {defect}")
        summary_lines.append("")

    summary_path = OUT / "SUMMARY.md"
    summary_path.write_text("\n".join(summary_lines), encoding="utf-8")
    raw_path = OUT / "results.json"
    raw_path.write_text(json.dumps(results, indent=2), encoding="utf-8")

    print(f"wrote {summary_path}")
    print(f"wrote {raw_path}")
    print(f"{passed}/{len(results)} PASS")
    for item in results:
        mark = "PASS" if item["pass"] else "FAIL"
        print(f"  {mark}  {item['slug']}")
        for defect in item["defects"]:
            print(f"        {defect}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
