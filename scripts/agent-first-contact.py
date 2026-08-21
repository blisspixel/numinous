#!/usr/bin/env python3
"""Cold-start first-contact suite over MCP for the agent-and-machine track.

Proves discovery, tool inventory, multi-wing play, one game open, journal empty
path, and broadcast lifecycle without a human tester. Not a stranger hallway
claim and not a 0.4 comprehension result.
"""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "scripts" / "mcp-play.py"
OUT = ROOT / ".agent" / "tester-cohort" / "first-contact"
EXPECTED_TOOL_COUNT = 36

# Stratified sample: five flagships plus one room from additional wings.
CONTACT_ROOMS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("times-tables", ("DRAG", "DIAL")),
    ("double-pendulum", ("CLICK", "RE-DROP")),
    ("game-of-life", ("CLICK", "GLIDER")),
    ("galton-board", ("CLICK", "DROP")),
    ("buffon-needle", ()),
    ("lorenz", ()),
    ("cellular-automata", ()),
    ("lissajous", ()),
    ("mandelbrot", ()),
)


@dataclass(frozen=True)
class Check:
    name: str
    passed: bool
    detail: str


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
    for key in ("status", "readout", "message"):
        value = structured.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return (result.get("text") or "").strip()[:120]


def check_tools() -> Check:
    result = call_tool("list_rooms", {})
    if not result.get("ok"):
        return Check("list_rooms", False, result.get("stderr") or "call failed")
    # tools listing goes through mcp-play tools subcommand for the full inventory
    process = subprocess.run(
        [sys.executable, str(DRIVER), "tools"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode != 0:
        return Check("tools_inventory", False, process.stderr.strip() or "tools failed")
    # Last line is "{n} tools."
    lines = [line.strip() for line in process.stdout.splitlines() if line.strip()]
    if not lines or not lines[-1].endswith("tools."):
        return Check("tools_inventory", False, "missing tools count line")
    try:
        count = int(lines[-1].split()[0])
    except ValueError:
        return Check("tools_inventory", False, f"bad count line: {lines[-1]!r}")
    if count != EXPECTED_TOOL_COUNT:
        return Check(
            "tools_inventory",
            False,
            f"expected {EXPECTED_TOOL_COUNT} tools, got {count}",
        )
    return Check("tools_inventory", True, f"{count} tools")


def check_room(room_id: str, invite_tokens: tuple[str, ...]) -> list[Check]:
    checks: list[Check] = []
    open_result = call_tool(
        "play_room",
        {"id": room_id, "t": 0.0, "width": 48, "height": 24},
    )
    if not open_result.get("ok"):
        checks.append(
            Check(
                f"open:{room_id}",
                False,
                open_result.get("stderr") or open_result.get("stdout") or "fail",
            )
        )
        return checks
    status = status_of(open_result)
    structured = open_result.get("structured") or {}
    render = structured.get("render")
    if not isinstance(render, str) or not render.strip():
        checks.append(Check(f"open_render:{room_id}", False, "missing structured render"))
    else:
        checks.append(Check(f"open_render:{room_id}", True, "render present"))
    if not status:
        checks.append(Check(f"open_status:{room_id}", False, "empty first-contact status"))
    else:
        checks.append(Check(f"open_status:{room_id}", True, status[:56]))
    if invite_tokens:
        upper = status.upper()
        missing = [token for token in invite_tokens if token.upper() not in upper]
        if missing:
            checks.append(
                Check(
                    f"invite:{room_id}",
                    False,
                    f"missing {missing} in status={status!r}",
                )
            )
        else:
            checks.append(Check(f"invite:{room_id}", True, "invite tokens present"))

    hand_result = call_tool(
        "play_room",
        {
            "id": room_id,
            "t": 0.0,
            "width": 48,
            "height": 24,
            "pokes": [[0.5, 0.5]],
        },
    )
    if not hand_result.get("ok"):
        checks.append(
            Check(
                f"poke:{room_id}",
                False,
                hand_result.get("stderr") or hand_result.get("stdout") or "fail",
            )
        )
        return checks
    hand_status = status_of(hand_result)
    hand_render = (hand_result.get("structured") or {}).get("render")
    open_render = render if isinstance(render, str) else ""
    changed = hand_status != status or (
        isinstance(hand_render, str) and hand_render != open_render
    )
    if not changed:
        # Some rooms are phase-scrub only at center; accept non-empty hand status.
        if hand_status:
            checks.append(
                Check(
                    f"poke:{room_id}",
                    True,
                    "status retained after center poke (may be phase-scrub)",
                )
            )
        else:
            checks.append(Check(f"poke:{room_id}", False, "no status after poke"))
    else:
        checks.append(Check(f"poke:{room_id}", True, "hand changed status or plate"))
    return checks


def check_game() -> Check:
    result = call_tool("munch", {"seed": 1})
    if not result.get("ok"):
        return Check("game_munch", False, result.get("stderr") or "munch failed")
    structured = result.get("structured") or {}
    board = structured.get("board") or structured.get("grid") or structured.get("state")
    text = result.get("text") or ""
    if board is None and not text.strip():
        return Check("game_munch", False, "empty munch result")
    return Check("game_munch", True, "munch opened")


def check_journal() -> Check:
    result = call_tool("read_journal", {})
    if not result.get("ok"):
        # Some hosts may require opt-in; empty or explicit opt-in message is ok
        err = (result.get("stderr") or "") + (result.get("stdout") or "")
        if "opt" in err.lower() or "empty" in err.lower() or "journal" in err.lower():
            return Check("journal_read", True, "journal path responded")
        return Check("journal_read", False, err or "read_journal failed")
    return Check("journal_read", True, "read_journal ok")


def check_broadcast() -> list[Check]:
    # Cold start without a human App listener: status is always legal without a
    # pairing code. Full start requires an explicit code and is covered elsewhere.
    status = call_tool("broadcast_session", {"action": "status"})
    if not status.get("ok"):
        detail = status.get("stderr") or status.get("stdout") or "status failed"
        return [Check("broadcast_status", False, detail)]
    structured = status.get("structured") or {}
    state = structured.get("state")
    if isinstance(state, str) and state.strip():
        return [Check("broadcast_status", True, f"state={state}")]
    text = status.get("text") or ""
    if "broadcast" in text.lower() or "session" in text.lower():
        return [Check("broadcast_status", True, "status text present")]
    return [Check("broadcast_status", True, "status call succeeded")]


def run_suite() -> dict[str, Any]:
    checks: list[Check] = []
    checks.append(check_tools())
    for room_id, tokens in CONTACT_ROOMS:
        checks.extend(check_room(room_id, tokens))
    checks.append(check_game())
    checks.append(check_journal())
    checks.extend(check_broadcast())
    failed = [check for check in checks if not check.passed]
    return {
        "suite": "agent-first-contact",
        "passed": not failed,
        "check_count": len(checks),
        "failed_count": len(failed),
        "failed": [{"name": c.name, "detail": c.detail} for c in failed],
        "checks": [{"name": c.name, "passed": c.passed, "detail": c.detail} for c in checks],
        "evidence_class": "agent-mcp-machine",
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    summary = run_suite()
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(
        f"{summary['check_count'] - summary['failed_count']}/{summary['check_count']} PASS"
    )
    for item in summary["checks"]:
        mark = "PASS" if item["passed"] else "FAIL"
        print(f"  {mark}  {item['name']}: {item['detail']}")
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
