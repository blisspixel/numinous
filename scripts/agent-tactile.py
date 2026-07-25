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
OUT = ROOT / ".agent" / "tester-cohort" / "round-09-tactile-0.3"


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
    # Optional listen_room args: hand notes must differ from open notes.
    # Empty means no sonic gate (visual/status only for this probe).
    sonic_open_args: dict[str, Any] | None = None
    sonic_hand_args: dict[str, Any] | None = None


PROBES = [
    Probe(
        slug="times-tables",
        title="Times Tables",
        open_tool="play_room",
        open_args={"id": "times-tables", "t": 0.0, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "times-tables",
            "t": 0.0,
            "width": 56,
            "height": 28,
            "pokes": [[0.72, 0.50]],
        },
        # Opening must lead with the dial invite, not ambient K alone.
        invite_tokens=("DRAG", "DIAL"),
        expect_status_change=True,
        sonic_open_args={"id": "times-tables", "t": 0.0},
        sonic_hand_args={"id": "times-tables", "t": 0.0, "pokes": [[0.72, 0.50]]},
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
        # Opening must lead with re-drop, not twins telemetry alone.
        invite_tokens=("CLICK", "RE-DROP"),
        expect_status_change=True,
        # Fling release must change mathematical notes vs ambient open.
        sonic_open_args={"id": "double-pendulum", "t": 0.35},
        sonic_hand_args={
            "id": "double-pendulum",
            "t": 0.35,
            "gesture": [
                {"kind": "down", "x": 0.3, "y": 0.5, "t": 0.10},
                {"kind": "move", "x": 0.3, "y": 0.5, "t": 0.147},
                {"kind": "up", "x": 0.6, "y": 0.5, "t": 0.15},
            ],
        },
    ),
    Probe(
        slug="game-of-life",
        title="Game of Life",
        open_tool="play_room",
        open_args={"id": "game-of-life", "t": 0.0, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "game-of-life",
            "t": 0.0,
            "width": 56,
            "height": 28,
            "pokes": [[0.45, 0.45]],
        },
        # Opening must invite the plant; GEN alone is ambient soup.
        invite_tokens=("CLICK", "GLIDER", "PLACE"),
        expect_status_change=True,
        # Plant-at-snapshot is status-first; births sound on later gens only.
    ),
    Probe(
        slug="galton-board",
        title="Galton Board",
        open_tool="play_room",
        open_args={"id": "galton-board", "t": 0.0, "width": 56, "height": 28},
        hand_tool="play_room",
        hand_args={
            "id": "galton-board",
            "t": 0.0,
            "width": 56,
            "height": 28,
            "pokes": [[0.20, 0.50]],
        },
        # Opening must lead with the drop invite, not coin inventory alone.
        invite_tokens=("CLICK", "DROP", "PICK", "COIN", "BET"),
        expect_status_change=True,
        sonic_open_args={"id": "galton-board", "t": 0.0},
        sonic_hand_args={"id": "galton-board", "t": 0.0, "pokes": [[0.20, 0.50]]},
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


def notes_signature(result: dict[str, Any]) -> str:
    """Stable fingerprint of mathematical notes from listen_room."""
    structured = result.get("structured") or {}
    notes = structured.get("notes")
    if not isinstance(notes, list) or not notes:
        return ""
    parts: list[str] = []
    for note in notes:
        if not isinstance(note, dict):
            continue
        parts.append(
            f"{note.get('frequency_hz')}:{note.get('amplitude')}:{note.get('duration_seconds')}"
        )
    return "|".join(parts)


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
        # Status must carry the invite. Action-only chrome is not enough for
        # first contact: agents and footers read status first.
        upper = open_status.upper()
        if not any(token.upper() in upper for token in probe.invite_tokens):
            action = ""
            structured = open_result.get("structured") or {}
            if isinstance(structured.get("action"), str):
                action = structured["action"]
            defects.append(
                f"first contact status missing invite tokens {probe.invite_tokens}: "
                f"status={open_status!r} action={action!r}"
            )

    if probe.expect_status_change and open_result.get("ok") and hand_result.get("ok"):
        if open_status == hand_status and open_plate == hand_plate:
            defects.append("hand left status and plate unchanged")

    if open_status and len(open_status) > 56 and probe.open_tool == "play_room":
        defects.append(f"open status longer than 56 chars: {open_status!r}")
    if hand_status and len(hand_status) > 56 and probe.hand_tool == "play_room":
        defects.append(f"hand status longer than 56 chars: {hand_status!r}")

    sonic_diff = None
    if probe.sonic_open_args is not None and probe.sonic_hand_args is not None:
        sonic_open = call_tool("listen_room", probe.sonic_open_args)
        sonic_hand = call_tool("listen_room", probe.sonic_hand_args)
        if not sonic_open.get("ok"):
            defects.append(
                f"sonic open failed: {sonic_open.get('stderr') or sonic_open.get('stdout')}"
            )
        elif not sonic_hand.get("ok"):
            defects.append(
                f"sonic hand failed: {sonic_hand.get('stderr') or sonic_hand.get('stdout')}"
            )
        else:
            open_sig = notes_signature(sonic_open)
            hand_sig = notes_signature(sonic_hand)
            sonic_diff = open_sig != hand_sig and bool(open_sig) and bool(hand_sig)
            if not sonic_diff:
                defects.append(
                    "hand left mathematical notes unchanged "
                    f"(open={open_sig[:80]!r} hand={hand_sig[:80]!r})"
                )

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
        "sonic_diff": sonic_diff,
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
        if item.get("sonic_diff") is True:
            summary_lines.append("- sonic: hand notes differ from open")
        elif item.get("sonic_diff") is False:
            summary_lines.append("- sonic: hand notes match open")
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
