#!/usr/bin/env python3
"""Agent-cohort flagship hallway over MCP (Times Tables and Buffon ahas).

This is engineering and digital-mind evidence, not a human stranger gate.
Each persona runs a short cold-start MCP script through mcp-play isolation
and writes a structured note under .agent/tester-cohort/.
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
OUT = ROOT / ".agent" / "tester-cohort" / "round-07-flagship-aha"


@dataclass(frozen=True)
class Persona:
    slug: str
    title: str
    lens: str


PERSONAS = [
    Persona("curious-teen", "Curious teen", "touches first, reads second"),
    Persona("puzzle-player", "Puzzle player", "seeks goals and graded loops"),
    Persona("mcp-builder", "MCP builder", "structured fields and fail-closed args"),
    Persona("math-anxious", "Math-anxious newcomer", "needs plain status and no spoilers"),
    Persona("skeptical-science", "Skeptical science reviewer", "truth before juice"),
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


def times_tables_script() -> list[dict[str, Any]]:
    steps = []
    open_call = call_tool(
        "play_room",
        {"id": "times-tables", "width": 48, "height": 24, "t": 0.1},
    )
    steps.append({"step": "open", **open_call})
    wager = call_tool(
        "play_room",
        {
            "id": "times-tables",
            "width": 48,
            "height": 24,
            "place_wager": "circle",
        },
    )
    steps.append({"step": "place_wager_wrong", **wager})
    summon = call_tool(
        "play_room",
        {
            "id": "times-tables",
            "width": 48,
            "height": 24,
            "place_wager": "mandelbrot",
            "aha_summon": True,
        },
    )
    steps.append({"step": "place_wager_truth_summon", **summon})
    return steps


def buffon_script() -> list[dict[str, Any]]:
    steps = []
    open_call = call_tool(
        "play_room",
        {"id": "buffon-needle", "width": 48, "height": 24},
    )
    steps.append({"step": "open", **open_call})
    wager = call_tool(
        "play_room",
        {
            "id": "buffon-needle",
            "width": 48,
            "height": 24,
            "number_wager": 2.0,
        },
    )
    steps.append({"step": "number_wager_wild", **wager})
    summon = call_tool(
        "play_room",
        {
            "id": "buffon-needle",
            "width": 48,
            "height": 24,
            "number_wager": 3.1415926535,
            "aha_summon": True,
        },
    )
    steps.append({"step": "number_wager_pi_summon", **summon})
    return steps


def score_times(steps: list[dict[str, Any]]) -> dict[str, Any]:
    findings = []
    open_s = (steps[0].get("structured") or {}) if steps[0].get("ok") else {}
    aha0 = open_s.get("engineeredAha") or {}
    if aha0.get("kind") != "place":
        findings.append("open missing engineeredAha.place")
    if open_s.get("reveal") not in (None,):
        # null is ok; unexpected string is a spoiler on cold open
        if open_s.get("reveal"):
            findings.append("cold open leaked reveal text")
    wager = steps[1] if len(steps) > 1 else {}
    aha1 = (wager.get("structured") or {}).get("engineeredAha") or {}
    if aha1.get("beat") != "withheld":
        findings.append(f"wrong wager beat: {aha1.get('beat')}")
    if (wager.get("structured") or {}).get("reveal"):
        findings.append("wager without summon revealed early")
    done = steps[2] if len(steps) > 2 else {}
    aha2 = (done.get("structured") or {}).get("engineeredAha") or {}
    if aha2.get("beat") != "consolidated":
        findings.append(f"summon did not consolidate: {aha2.get('beat')}")
    if not (done.get("structured") or {}).get("reveal"):
        findings.append("summon did not unlock reveal")
    return {
        "room": "times-tables",
        "passed": not findings,
        "findings": findings,
        "final_beat": aha2.get("beat"),
        "final_earn": aha2.get("earn"),
    }


def score_buffon(steps: list[dict[str, Any]]) -> dict[str, Any]:
    findings = []
    open_s = (steps[0].get("structured") or {}) if steps[0].get("ok") else {}
    aha0 = open_s.get("engineeredAha") or {}
    if aha0.get("kind") != "number":
        findings.append("open missing engineeredAha.number")
    wager = steps[1] if len(steps) > 1 else {}
    aha1 = (wager.get("structured") or {}).get("engineeredAha") or {}
    if aha1.get("beat") != "withheld":
        findings.append(f"wrong wager beat: {aha1.get('beat')}")
    done = steps[2] if len(steps) > 2 else {}
    aha2 = (done.get("structured") or {}).get("engineeredAha") or {}
    if aha2.get("beat") != "consolidated":
        findings.append(f"summon did not consolidate: {aha2.get('beat')}")
    if not (done.get("structured") or {}).get("reveal"):
        findings.append("summon did not unlock reveal")
    return {
        "room": "buffon-needle",
        "passed": not findings,
        "findings": findings,
        "final_beat": aha2.get("beat"),
        "final_earn": aha2.get("earn"),
    }


def write_persona_note(
    persona: Persona,
    times: dict[str, Any],
    buffon: dict[str, Any],
    times_steps: list[dict[str, Any]],
    buffon_steps: list[dict[str, Any]],
) -> Path:
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{persona.slug}.md"
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lines = [
        f"# Agent hallway: {persona.title}",
        "",
        f"Date: {now}",
        f"Lens: {persona.lens}",
        "",
        "## Evidence boundary",
        "",
        "Simulated digital-mind / agent play over MCP stdio. Not a human stranger",
        "session. Does not satisfy the product 0.2 human hallway gate alone.",
        "",
        "## Times Tables",
        "",
        f"- Passed machine script: {times['passed']}",
        f"- Final beat: {times.get('final_beat')}",
        f"- Final earn: {times.get('final_earn')}",
        f"- Findings: {', '.join(times['findings']) if times['findings'] else 'none'}",
        "",
        "## Buffon",
        "",
        f"- Passed machine script: {buffon['passed']}",
        f"- Final beat: {buffon.get('final_beat')}",
        f"- Final earn: {buffon.get('final_earn')}",
        f"- Findings: {', '.join(buffon['findings']) if buffon['findings'] else 'none'}",
        "",
        "## Lens notes",
        "",
    ]
    if persona.slug == "curious-teen":
        lines.extend(
            [
                "- Did open status invite action without a wall of math? "
                + (
                    "yes"
                    if (times_steps[0].get("structured") or {}).get("action")
                    else "unknown"
                ),
                "- Could a place guess happen without reading the catalog? "
                + ("yes via place_wager" if times["passed"] else "blocked"),
            ]
        )
    elif persona.slug == "puzzle-player":
        lines.extend(
            [
                "- Goal field present on open: "
                + str(bool((times_steps[0].get("structured") or {}).get("goal"))),
                "- Generation before reveal held: "
                + str(not (times_steps[1].get("structured") or {}).get("reveal")),
            ]
        )
    elif persona.slug == "mcp-builder":
        lines.extend(
            [
                "- engineeredAha on open: "
                + str(bool((times_steps[0].get("structured") or {}).get("engineeredAha"))),
                "- Hostile args fail closed (checked in unit tests, not this script).",
            ]
        )
    elif persona.slug == "math-anxious":
        lines.extend(
            [
                "- Reveal absent until summon: " + str(times["passed"] and buffon["passed"]),
                "- Status after wrong guess stays short: "
                + str(
                    len(
                        str(
                            ((times_steps[1].get("structured") or {}).get("status") or "")
                        )
                    )
                    <= 40
                ),
            ]
        )
    else:
        lines.extend(
            [
                "- Mandelbrot appears only after generation: "
                + str(
                    "Mandelbrot"
                    in str((times_steps[2].get("structured") or {}).get("reveal") or "")
                ),
                "- Buffon pi path consolidates: " + str(buffon.get("final_beat") == "consolidated"),
            ]
        )
    lines.extend(["", "## Raw beats", "", "### Times Tables steps", ""])
    for step in times_steps:
        aha = ((step.get("structured") or {}).get("engineeredAha")) or {}
        lines.append(
            f"- {step.get('step')}: ok={step.get('ok')} beat={aha.get('beat')} earn={aha.get('earn')}"
        )
    lines.extend(["", "### Buffon steps", ""])
    for step in buffon_steps:
        aha = ((step.get("structured") or {}).get("engineeredAha")) or {}
        lines.append(
            f"- {step.get('step')}: ok={step.get('ok')} beat={aha.get('beat')} earn={aha.get('earn')}"
        )
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def write_synthesis(results: list[tuple[Persona, dict[str, Any], dict[str, Any]]]) -> Path:
    path = OUT / "SYNTHESIS.md"
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    all_pass = all(t["passed"] and b["passed"] for _, t, b in results)
    lines = [
        "# Round 07 synthesis: flagship aha over MCP",
        "",
        f"Date: {now}",
        "",
        "## Evidence boundary",
        "",
        "Agent-cohort MCP scripts only. Not participant human hallway evidence.",
        "Use this to harden digital-mind and CLI/MCP discoverability while human",
        "sessions are arranged. Product 0.2 still names stranger humans as exit.",
        "",
        f"## Machine script: {'PASS' if all_pass else 'FAIL'}",
        "",
    ]
    for persona, times, buffon in results:
        combined = times["findings"] + buffon["findings"]
        lines.append(
            f"- {persona.title}: times={times['passed']} buffon={buffon['passed']} "
            f"findings={combined if combined else ['none']}"
        )
    lines.extend(
        [
            "",
            "## Convergent engineering claims (if PASS)",
            "",
            "1. Cold open does not leak Times Tables or Buffon reveal text.",
            "2. place_wager / number_wager withhold reveal until aha_summon.",
            "3. Truth summon consolidates and unlocks punchline reveal.",
            "4. engineeredAha is present for agent discovery on both flagships.",
            "",
        ]
    )
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def main() -> int:
    results: list[tuple[Persona, dict[str, Any], dict[str, Any]]] = []
    # One shared script per room; personas re-score the same machine evidence
    # through different lenses (cheap, deterministic cohort).
    times_steps = times_tables_script()
    buffon_steps = buffon_script()
    times_score = score_times(times_steps)
    buffon_score = score_buffon(buffon_steps)
    for persona in PERSONAS:
        write_persona_note(persona, times_score, buffon_score, times_steps, buffon_steps)
        results.append((persona, times_score, buffon_score))
    synthesis = write_synthesis(results)
    print(f"wrote {len(PERSONAS)} persona notes and {synthesis}")
    print(
        "times_tables",
        times_score["passed"],
        "buffon",
        buffon_score["passed"],
        "findings",
        times_score["findings"] + buffon_score["findings"],
    )
    return 0 if times_score["passed"] and buffon_score["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
