#!/usr/bin/env python3
"""Agent tactile review of the 0.3 five-flagship cohort over MCP.

Machine evidence for discoverable action and hand consequence on Times Tables,
Double Pendulum, Game of Life, Galton Board, and Formula Jam (Studio plot).
Not a human stranger gate. Writes notes under .agent/tester-cohort/.
"""

from __future__ import annotations

import hashlib
import json
import math
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
    hand_tokens: tuple[str, ...]
    expect_status_change: bool
    # Optional sonic gate: hand signature must differ from open.
    # Default tool is listen_room; Formula Jam uses sing_expression.
    sonic_tool: str = "listen_room"
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
        hand_tokens=("K", "MORPH", "LOBES"),
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
            "t": 0.35,
            "width": 56,
            "height": 28,
            "gesture": [
                {"kind": "down", "x": 0.3, "y": 0.5, "t": 0.10},
                {"kind": "move", "x": 0.3, "y": 0.5, "t": 0.147},
                {"kind": "up", "x": 0.6, "y": 0.5, "t": 0.15},
            ],
        },
        # Opening must lead with re-drop, not twins telemetry alone.
        invite_tokens=("CLICK", "RE-DROP"),
        hand_tokens=("FLUNG", "TWINS"),
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
            "t": 0.37,
            "width": 56,
            "height": 28,
            "gesture": [{"kind": "down", "x": 0.24, "y": 0.71, "t": 0.08}],
        },
        # Opening must invite the plant; GEN alone is ambient soup.
        invite_tokens=("CLICK", "GLIDER"),
        hand_tokens=("GEN 52", "GLIDER 1"),
        expect_status_change=True,
        # Early plant must change birth notes by a later snapshot (not t=0).
        sonic_open_args={"id": "game-of-life", "t": 0.37},
        sonic_hand_args={
            "id": "game-of-life",
            "t": 0.37,
            "gesture": [{"kind": "down", "x": 0.24, "y": 0.71, "t": 0.08}],
        },
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
        hand_tokens=("DROP", "P.40", "M6.4"),
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
        hand_tokens=(),
        expect_status_change=True,
        sonic_tool="sing_expression",
        sonic_open_args={"expr": "sin(x)"},
        sonic_hand_args={"expr": "sin(2*x)"},
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


def canonical_plate(result: dict[str, Any], tool: str) -> str:
    """Return the required structured visual field without a text fallback."""
    structured = result.get("structured") or {}
    key = "render" if tool == "play_room" else "plot" if tool == "plot_expression" else ""
    value = structured.get(key) if key else None
    return value if isinstance(value, str) and value.strip() else ""


def digest(text: str) -> str:
    """Return a compact stable identity for potentially large evidence."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def render_delta(result: dict[str, Any]) -> dict[str, Any] | None:
    """Return a complete MCP render delta when one is present and well typed."""
    structured = result.get("structured") or {}
    delta = structured.get("delta")
    if not isinstance(delta, dict):
        return None
    required = (
        "cells_changed",
        "ink_added",
        "ink_removed",
        "ink_reshaped",
        "total_cells",
    )
    if not all(type(delta.get(key)) is int for key in required):
        return None
    return delta


def notes_signature(result: dict[str, Any]) -> str:
    """Stable fingerprint of mathematical notes from listen_room or melody text."""
    structured = result.get("structured") or {}
    notes = structured.get("notes")
    if isinstance(notes, list) and notes:
        parts: list[str] = []
        for note in notes:
            if not isinstance(note, dict):
                continue
            parts.append(
                f"{note.get('frequency_hz')}:{note.get('amplitude')}:{note.get('duration_seconds')}"
            )
        if parts:
            return "|".join(parts)
    # sing_expression returns readable melody lines in text, not notes JSON.
    text = result.get("text") or ""
    freqs = []
    for line in text.splitlines():
        if "Hz" not in line:
            continue
        token = line.split("Hz", 1)[0].strip().split()
        if token:
            freqs.append(token[-1])
    return "|".join(freqs)


def note_values(result: dict[str, Any]) -> list[tuple[float, float | None, float | None]]:
    """Extract numeric note evidence without depending on presentation text."""
    structured = result.get("structured") or {}
    notes = structured.get("notes")
    values: list[tuple[float, float | None, float | None]] = []
    if isinstance(notes, list):
        for note in notes:
            if not isinstance(note, dict):
                continue
            frequency = note.get("frequency_hz")
            amplitude = note.get("amplitude")
            duration = note.get("duration_seconds")
            if isinstance(frequency, (int, float)):
                values.append(
                    (
                        float(frequency),
                        float(amplitude) if isinstance(amplitude, (int, float)) else None,
                        float(duration) if isinstance(duration, (int, float)) else None,
                    )
                )
        if values:
            return values

    for line in (result.get("text") or "").splitlines():
        if "Hz" not in line:
            continue
        token = line.split("Hz", 1)[0].strip().split()
        if not token:
            continue
        try:
            values.append((float(token[-1]), None, None))
        except ValueError:
            continue
    return values


def close(left: float, right: float, tolerance: float = 1e-5) -> bool:
    """Compare deterministic floating evidence with a bounded relative error."""
    return abs(left - right) <= tolerance * max(1.0, abs(left), abs(right))


def sonic_congruence(
    probe: Probe,
    open_result: dict[str, Any],
    hand_result: dict[str, Any],
    open_status: str,
    hand_status: str,
) -> tuple[bool, str]:
    """Check a room-specific mathematical invariant in the changed notes."""
    open_notes = note_values(open_result)
    hand_notes = note_values(hand_result)
    if probe.slug == "times-tables":
        try:
            hand_k = float(hand_status.split("K ", 1)[1].split()[0])
        except (IndexError, ValueError):
            return False, "status did not expose the selected multiplier"
        ok = (
            len(open_notes) == 2
            and len(hand_notes) == 2
            and close(hand_k, 7.76)
            and hand_notes[0][0] > 0.0
            and close(open_notes[0][0], hand_notes[0][0])
            and close(hand_notes[1][0] / hand_notes[0][0], hand_k / (hand_k - 1.0))
        )
        return ok, "dial x=0.72 selects K 7.76 and retunes to the K/(K-1) interval"
    if probe.slug == "double-pendulum":
        ok = (
            len(hand_notes) == 2
            and close(hand_notes[0][0], hand_notes[1][0])
            and "FLUNG" in hand_status
            and "TWINS 0.000" in hand_status
        )
        return ok, "the symmetric fling leaves coincident twins and two equal release pitches"
    if probe.slug == "game-of-life":
        open_frequencies = [note[0] for note in open_notes]
        hand_frequencies = [note[0] for note in hand_notes]
        open_amplitudes = [note[1] for note in open_notes]
        hand_amplitudes = [note[1] for note in hand_notes]
        try:
            births = int(hand_status.split("BORN ", 1)[1].split()[0])
        except (IndexError, ValueError):
            return False, "evolved status did not expose the birth count"
        activity = 0.45 + 0.55 * min(math.sqrt(births / 256.0), 1.0)
        row_births = [
            round(births * ((amplitude or 0.0) / (0.06 * activity)) ** 2)
            for _, amplitude, _ in hand_notes
        ]
        ok = (
            len(open_notes) == 12
            and len(hand_notes) == 12
            and all(
                close(left, right)
                for left, right in zip(open_frequencies, hand_frequencies, strict=True)
            )
            and open_amplitudes != hand_amplitudes
            and births > 0
            and sum(row_births) == births
            and "GEN 52" in hand_status
            and "GLIDER 1" in hand_status
        )
        return ok, "12 pitch-row amplitudes reconstruct the evolved status birth count"
    if probe.slug == "galton-board":
        ratio = hand_notes[1][0] / hand_notes[0][0] if len(hand_notes) == 2 else 0.0
        ok = len(hand_notes) == 2 and close(ratio, 1.5) and "P.40" in hand_status
        return ok, "the selected p=0.40 drop resolves to the encoded 3:2 landing interval"
    if probe.slug == "formula-jam":
        open_frequencies = [round(note[0], 6) for note in open_notes]
        hand_frequencies = [round(note[0], 6) for note in hand_notes]
        ok = (
            open_status == "sin(x)"
            and hand_status == "sin(2*x)"
            and len(open_frequencies) == 24
            and sorted(open_frequencies) == sorted(hand_frequencies)
            and open_frequencies != hand_frequencies
        )
        return ok, "sin(2*x) traverses the same sampled pitch set in doubled phase order"
    return False, "probe has no declared sonic invariant"


def review_probe(probe: Probe) -> dict[str, Any]:
    open_result = call_tool(probe.open_tool, probe.open_args)
    hand_result = call_tool(probe.hand_tool, probe.hand_args)
    open_status = status_of(open_result)
    hand_status = status_of(hand_result)
    open_plate = canonical_plate(open_result, probe.open_tool)
    hand_plate = canonical_plate(hand_result, probe.hand_tool)

    defects: list[str] = []
    if not open_result.get("ok"):
        defects.append(f"open failed: {open_result.get('stderr') or open_result.get('stdout')}")
    if not hand_result.get("ok"):
        defects.append(f"hand failed: {hand_result.get('stderr') or hand_result.get('stdout')}")
    if open_result.get("ok") and not open_plate:
        defects.append(f"open result omitted canonical structured {probe.open_tool} visual")
    if hand_result.get("ok") and not hand_plate:
        defects.append(f"hand result omitted canonical structured {probe.hand_tool} visual")

    if probe.invite_tokens and open_result.get("ok"):
        # Status must carry the invite. Action-only chrome is not enough for
        # first contact: agents and footers read status first.
        upper = open_status.upper()
        if not all(token.upper() in upper for token in probe.invite_tokens):
            action = ""
            structured = open_result.get("structured") or {}
            if isinstance(structured.get("action"), str):
                action = structured["action"]
            defects.append(
                f"first contact status missing required invite tokens {probe.invite_tokens}: "
                f"status={open_status!r} action={action!r}"
            )

    if probe.hand_tokens and hand_result.get("ok"):
        upper = hand_status.upper()
        if not all(token.upper() in upper for token in probe.hand_tokens):
            defects.append(
                f"hand status missing mathematical consequence tokens {probe.hand_tokens}: "
                f"status={hand_status!r}"
            )

    if probe.expect_status_change and open_result.get("ok") and hand_result.get("ok"):
        if open_status == hand_status and open_plate == hand_plate:
            defects.append("hand left status and plate unchanged")

    hand_delta = None
    if probe.hand_tool == "play_room" and hand_result.get("ok"):
        hand_delta = render_delta(hand_result)
        if hand_delta is None:
            defects.append("hand result omitted a complete structured render delta")
        else:
            width = probe.hand_args.get("width")
            height = probe.hand_args.get("height")
            components = (
                hand_delta["ink_added"]
                + hand_delta["ink_removed"]
                + hand_delta["ink_reshaped"]
            )
            region = hand_delta.get("changed_region")
            if hand_delta["cells_changed"] <= 0:
                defects.append("hand changed no render cells")
            if hand_delta["cells_changed"] != components:
                defects.append("render delta components do not sum to cells_changed")
            if not isinstance(width, int) or not isinstance(height, int):
                defects.append("probe omitted integer render dimensions")
            elif hand_delta["total_cells"] != width * height:
                defects.append("render delta total_cells does not match probe dimensions")
            if (
                not isinstance(region, list)
                or len(region) != 4
                or not all(type(value) is int for value in region)
            ):
                defects.append("nonzero render delta has no four-integer changed region")
            elif isinstance(width, int) and isinstance(height, int):
                x0, y0, x1, y1 = region
                if not (0 <= x0 <= x1 < width and 0 <= y0 <= y1 < height):
                    defects.append("render delta changed region is outside probe dimensions")

    if open_result.get("ok") and hand_result.get("ok") and open_plate == hand_plate:
        defects.append("hand left the canonical structured render unchanged")

    if open_status and len(open_status) > 56 and probe.open_tool == "play_room":
        defects.append(f"open status longer than 56 chars: {open_status!r}")
    if hand_status and len(hand_status) > 56 and probe.hand_tool == "play_room":
        defects.append(f"hand status longer than 56 chars: {hand_status!r}")

    sonic_diff = None
    sonic_math = None
    sonic_math_claim = ""
    open_note_signature = ""
    hand_note_signature = ""
    if probe.sonic_open_args is not None and probe.sonic_hand_args is not None:
        sonic_open = call_tool(probe.sonic_tool, probe.sonic_open_args)
        sonic_hand = call_tool(probe.sonic_tool, probe.sonic_hand_args)
        if not sonic_open.get("ok"):
            defects.append(
                f"sonic open failed: {sonic_open.get('stderr') or sonic_open.get('stdout')}"
            )
        elif not sonic_hand.get("ok"):
            defects.append(
                f"sonic hand failed: {sonic_hand.get('stderr') or sonic_hand.get('stdout')}"
            )
        else:
            open_note_signature = notes_signature(sonic_open)
            hand_note_signature = notes_signature(sonic_hand)
            sonic_diff = (
                open_note_signature != hand_note_signature
                and bool(open_note_signature)
                and bool(hand_note_signature)
            )
            if not sonic_diff:
                defects.append(
                    "hand left mathematical notes unchanged "
                    f"(open={open_note_signature[:80]!r} hand={hand_note_signature[:80]!r})"
                )
            sonic_math, sonic_math_claim = sonic_congruence(
                probe,
                sonic_open,
                sonic_hand,
                open_status,
                hand_status,
            )
            if not sonic_math:
                defects.append(f"sonic consequence violated invariant: {sonic_math_claim}")

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
        "sonic_math": sonic_math,
        "sonic_math_claim": sonic_math_claim,
        "open_render_sha256": digest(open_plate) if open_plate else None,
        "hand_render_sha256": digest(hand_plate) if hand_plate else None,
        "hand_delta": hand_delta,
        "open_notes_sha256": digest(open_note_signature) if open_note_signature else None,
        "hand_notes_sha256": digest(hand_note_signature) if hand_note_signature else None,
        "open_note_signature": open_note_signature,
        "hand_note_signature": hand_note_signature,
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    tracked_dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=no"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    )
    checker_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    results = [review_probe(probe) for probe in PROBES]
    passed = sum(1 for item in results if item["pass"])
    failed = len(results) - passed

    summary_lines = [
        f"# Tactile agent cohort (0.3 five flagships)",
        "",
        f"Stamp: {stamp}",
        f"Revision: `{revision}`",
        f"Checker SHA-256: `{checker_sha256}`",
        f"Tracked worktree dirty: `{str(tracked_dirty).lower()}`",
        f"Result: {passed}/{len(results)} PASS, {failed} FAIL",
        "Evidence class: agent/MCP machine review, not human stranger hallway.",
        "",
    ]
    for item in results:
        mark = "PASS" if item["pass"] else "FAIL"
        summary_lines.append(f"## {item['title']} ({item['slug']})  {mark}")
        summary_lines.append(f"- open: `{item['open_status']}`")
        summary_lines.append(f"- hand: `{item['hand_status']}`")
        if item.get("open_render_sha256") and item.get("hand_render_sha256"):
            summary_lines.append(
                "- visual hashes: "
                f"`{item['open_render_sha256'][:12]}` to "
                f"`{item['hand_render_sha256'][:12]}`"
            )
        if item.get("hand_delta") is not None:
            summary_lines.append(
                f"- visual: {item['hand_delta']['cells_changed']} cells changed, "
                f"region {item['hand_delta'].get('changed_region')}"
            )
        if item.get("sonic_diff") is True:
            summary_lines.append("- sonic: hand notes differ from open")
        elif item.get("sonic_diff") is False:
            summary_lines.append("- sonic: hand notes match open")
        if item.get("sonic_math") is not None:
            summary_lines.append(
                f"- congruence: {'PASS' if item['sonic_math'] else 'FAIL'}, "
                f"{item['sonic_math_claim']}"
            )
        if item["defects"]:
            for defect in item["defects"]:
                summary_lines.append(f"- defect: {defect}")
        summary_lines.append("")

    summary_path = OUT / "SUMMARY.md"
    summary_path.write_text("\n".join(summary_lines), encoding="utf-8")
    raw_path = OUT / "results.json"
    raw_path.write_text(
        json.dumps(
            {
                "revision": revision,
                "checker_sha256": checker_sha256,
                "tracked_worktree_dirty": tracked_dirty,
                "results": results,
            },
            indent=2,
        ),
        encoding="utf-8",
    )

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
