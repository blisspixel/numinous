#!/usr/bin/env python3
"""Agent-cohort flagship hallway over MCP room-owned ahas.

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

# The gates share one way of getting the binaries they test; see gate_cli.py
# for why there is only one copy of it.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import build_and_locate  # noqa: E402
DRIVER = ROOT / "scripts" / "mcp-play.py"
OUT = ROOT / ".agent" / "tester-cohort" / "round-08-flagship-aha"


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


def initialize_script() -> dict[str, Any]:
    """Confirm agents are taught safe discovery and earned reveal at session start."""
    import os
    import tempfile

    # Builds and locates in one step. This used to build here and then assemble
    # its own path, which built twice, did the first build without --locked,
    # and ignored CARGO_TARGET_DIR so it looked in the wrong place on any
    # layout that sets one. See gate_cli.py for why there is only one copy.
    path = str(build_and_locate(("numinous-mcp",))[0])
    with tempfile.TemporaryDirectory(prefix="numinous-mcp-play-") as state_dir:
        env = dict(os.environ)
        env.update(
            {
                "NUMINOUS_JOURNEY": str(Path(state_dir) / "journey.txt"),
                "NUMINOUS_SCORES": str(Path(state_dir) / "scores.txt"),
                "NUMINOUS_CAIRN": str(Path(state_dir) / "cairn.json"),
            }
        )
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                    "io.modelcontextprotocol/clientInfo": {
                        "name": "agent-hallway",
                        "version": "1",
                    },
                }
            },
        }
        proc = subprocess.run(
            [path],
            input=json.dumps(request) + "\n",
            capture_output=True,
            text=True,
            cwd=ROOT,
            env=env,
            check=False,
        )
        if proc.returncode != 0:
            return {"ok": False, "error": proc.stderr.strip() or proc.stdout.strip()}
        try:
            response = json.loads(proc.stdout.strip().splitlines()[0])
        except (ValueError, IndexError) as error:
            return {"ok": False, "error": str(error)}
        instructions = ((response.get("result") or {}).get("instructions")) or ""
        ok = (
            "place_wager" in instructions
            and "number_wager" in instructions
            and "speed_wager" in instructions
            and "policy_wager" in instructions
            and "die_choice" in instructions
            and "counter_wager" in instructions
            and "describe_room is a safe doorway" in instructions
            and "reveal_room opens only after" in instructions
        )
        return {
            "ok": ok,
            "has_place_wager": "place_wager" in instructions,
            "has_number_wager": "number_wager" in instructions,
            "has_speed_wager": "speed_wager" in instructions,
            "has_policy_wager": "policy_wager" in instructions,
            "has_die_choice": "die_choice" in instructions,
            "has_counter_wager": "counter_wager" in instructions,
            "safe_describe": "describe_room is a safe doorway" in instructions,
            "earned_reveal": "reveal_room opens only after" in instructions,
        }


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


def kepler_script() -> list[dict[str, Any]]:
    steps = []
    open_call = call_tool(
        "play_room",
        {"id": "kepler-laws", "width": 48, "height": 24},
    )
    steps.append({"step": "open", **open_call})
    wager = call_tool(
        "play_room",
        {
            "id": "kepler-laws",
            "width": 48,
            "height": 24,
            "pokes": [[0.8, 0.5]],
            "speed_wager": "same",
        },
    )
    steps.append({"step": "speed_wager_wrong", **wager})
    summon = call_tool(
        "play_room",
        {
            "id": "kepler-laws",
            "width": 48,
            "height": 24,
            "pokes": [[0.8, 0.5]],
            "speed_wager": "faster",
            "aha_summon": True,
        },
    )
    steps.append({"step": "speed_wager_truth_summon", **summon})
    return steps


def parrondo_script() -> list[dict[str, Any]]:
    steps = []
    open_call = call_tool(
        "play_room",
        {"id": "parrondo", "width": 48, "height": 24},
    )
    steps.append({"step": "open", **open_call})
    wager = call_tool(
        "play_room",
        {
            "id": "parrondo",
            "width": 48,
            "height": 24,
            "pokes": [[0.5, 0.5]],
            "policy_wager": "a",
        },
    )
    steps.append({"step": "policy_wager_wrong", **wager})
    summon = call_tool(
        "play_room",
        {
            "id": "parrondo",
            "width": 48,
            "height": 24,
            "pokes": [[0.5, 0.5]],
            "policy_wager": "abb",
            "aha_summon": True,
        },
    )
    steps.append({"step": "policy_wager_truth_summon", **summon})
    return steps


def nontransitive_script() -> list[dict[str, Any]]:
    steps = []
    open_call = call_tool(
        "play_room",
        {"id": "nontransitive", "width": 48, "height": 24},
    )
    steps.append({"step": "open", **open_call})
    wager = call_tool(
        "play_room",
        {
            "id": "nontransitive",
            "width": 48,
            "height": 24,
            "die_choice": "a",
            "counter_wager": "b",
        },
    )
    steps.append({"step": "counter_wager_wrong", **wager})
    summon = call_tool(
        "play_room",
        {
            "id": "nontransitive",
            "width": 48,
            "height": 24,
            "die_choice": "a",
            "counter_wager": "c",
            "aha_summon": True,
        },
    )
    steps.append({"step": "counter_wager_truth_summon", **summon})
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


def score_kepler(steps: list[dict[str, Any]]) -> dict[str, Any]:
    findings = []
    open_s = (steps[0].get("structured") or {}) if steps[0].get("ok") else {}
    aha0 = open_s.get("engineeredAha") or {}
    if aha0.get("kind") != "speed":
        findings.append("open missing engineeredAha.speed")
    if open_s.get("reveal"):
        findings.append("cold open leaked reveal text")
    wager = steps[1] if len(steps) > 1 else {}
    wager_s = wager.get("structured") or {}
    aha1 = wager_s.get("engineeredAha") or {}
    if aha1.get("beat") != "withheld":
        findings.append(f"wrong wager beat: {aha1.get('beat')}")
    if wager_s.get("reveal"):
        findings.append("wager without summon revealed early")
    done = steps[2] if len(steps) > 2 else {}
    done_s = done.get("structured") or {}
    aha2 = done_s.get("engineeredAha") or {}
    if aha2.get("beat") != "consolidated":
        findings.append(f"summon did not consolidate: {aha2.get('beat')}")
    if aha2.get("truth") != "faster" or aha2.get("wager") != "faster":
        findings.append("selected ellipse did not answer the speed call")
    if "O" not in str(done_s.get("render") or ""):
        findings.append("consolidated render lacks equal-time marks")
    if not done_s.get("reveal"):
        findings.append("summon did not unlock reveal")
    return {
        "room": "kepler-laws",
        "passed": not findings,
        "findings": findings,
        "final_beat": aha2.get("beat"),
        "final_earn": aha2.get("earn"),
    }


def score_parrondo(steps: list[dict[str, Any]]) -> dict[str, Any]:
    findings = []
    open_s = (steps[0].get("structured") or {}) if steps[0].get("ok") else {}
    aha0 = open_s.get("engineeredAha") or {}
    if aha0.get("kind") != "policy":
        findings.append("open missing engineeredAha.policy")
    if open_s.get("reveal"):
        findings.append("cold open leaked reveal text")
    wager = steps[1] if len(steps) > 1 else {}
    wager_s = wager.get("structured") or {}
    aha1 = wager_s.get("engineeredAha") or {}
    if aha1.get("beat") != "withheld":
        findings.append(f"wrong wager beat: {aha1.get('beat')}")
    if wager_s.get("reveal"):
        findings.append("wager without summon revealed early")
    done = steps[2] if len(steps) > 2 else {}
    done_s = done.get("structured") or {}
    aha2 = done_s.get("engineeredAha") or {}
    if aha2.get("beat") != "consolidated":
        findings.append(f"summon did not consolidate: {aha2.get('beat')}")
    if aha2.get("truth") != "abb" or aha2.get("wager") != "abb":
        findings.append("exact expectation did not answer the policy call")
    expected = aha2.get("expectedEnd") or {}
    if not (expected.get("a", 0) < 0 and expected.get("b", 0) < 0 < expected.get("abb", 0)):
        findings.append("typed expectations do not prove two losers and one winner")
    render = str(done_s.get("render") or "")
    if not all(mark in render for mark in ("A", "B", "O")):
        findings.append("consolidated render lacks three policy paths")
    if not done_s.get("reveal"):
        findings.append("summon did not unlock reveal")
    return {
        "room": "parrondo",
        "passed": not findings,
        "findings": findings,
        "final_beat": aha2.get("beat"),
        "final_earn": aha2.get("earn"),
    }


def score_nontransitive(steps: list[dict[str, Any]]) -> dict[str, Any]:
    findings = []
    open_s = (steps[0].get("structured") or {}) if steps[0].get("ok") else {}
    aha0 = open_s.get("engineeredAha") or {}
    if aha0.get("kind") != "counter":
        findings.append("open missing engineeredAha.counter")
    if open_s.get("reveal"):
        findings.append("cold open leaked reveal text")
    wager = steps[1] if len(steps) > 1 else {}
    wager_s = wager.get("structured") or {}
    aha1 = wager_s.get("engineeredAha") or {}
    if aha1.get("beat") != "withheld":
        findings.append(f"wrong wager beat: {aha1.get('beat')}")
    if wager_s.get("reveal"):
        findings.append("wager without summon revealed early")
    done = steps[2] if len(steps) > 2 else {}
    done_s = done.get("structured") or {}
    aha2 = done_s.get("engineeredAha") or {}
    if aha2.get("beat") != "consolidated":
        findings.append(f"summon did not consolidate: {aha2.get('beat')}")
    if (
        aha2.get("chosen") != "a"
        or aha2.get("truth") != "c"
        or aha2.get("wager") != "c"
    ):
        findings.append("chosen die did not answer the counter call")
    cycle = aha2.get("exactCycle") or {}
    if cycle != {
        "aOverB": 24,
        "bOverC": 24,
        "cOverA": 20,
        "outcomesPerPair": 36,
    }:
        findings.append("typed outcome counts do not prove the exact cycle")
    if aha2.get("counterWins") != 20:
        findings.append("typed counter does not prove C beats A in 20 of 36 outcomes")
    render = str(done_s.get("render") or "")
    if "C vs A" not in render or "20 W / 16 L" not in render:
        findings.append("consolidated render lacks the exact outcome grid")
    if not done_s.get("reveal"):
        findings.append("summon did not unlock reveal")
    return {
        "room": "nontransitive",
        "passed": not findings,
        "findings": findings,
        "final_beat": aha2.get("beat"),
        "final_earn": aha2.get("earn"),
    }


def write_persona_note(
    persona: Persona,
    times: dict[str, Any],
    buffon: dict[str, Any],
    kepler: dict[str, Any],
    parrondo: dict[str, Any],
    nontransitive: dict[str, Any],
    times_steps: list[dict[str, Any]],
    buffon_steps: list[dict[str, Any]],
    kepler_steps: list[dict[str, Any]],
    parrondo_steps: list[dict[str, Any]],
    nontransitive_steps: list[dict[str, Any]],
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
        "## Kepler Areas",
        "",
        f"- Passed machine script: {kepler['passed']}",
        f"- Final beat: {kepler.get('final_beat')}",
        f"- Final earn: {kepler.get('final_earn')}",
        f"- Findings: {', '.join(kepler['findings']) if kepler['findings'] else 'none'}",
        "",
        "## Parrondo's Trap",
        "",
        f"- Passed machine script: {parrondo['passed']}",
        f"- Final beat: {parrondo.get('final_beat')}",
        f"- Final earn: {parrondo.get('final_earn')}",
        f"- Findings: {', '.join(parrondo['findings']) if parrondo['findings'] else 'none'}",
        "",
        "## Nontransitive Dice",
        "",
        f"- Passed machine script: {nontransitive['passed']}",
        f"- Final beat: {nontransitive.get('final_beat')}",
        f"- Final earn: {nontransitive.get('final_earn')}",
        f"- Findings: {', '.join(nontransitive['findings']) if nontransitive['findings'] else 'none'}",
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
                "- Reveal absent until summon: "
                + str(
                    times["passed"]
                    and buffon["passed"]
                    and kepler["passed"]
                    and parrondo["passed"]
                    and nontransitive["passed"]
                ),
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
                "- Kepler speed path consolidates: "
                + str(kepler.get("final_beat") == "consolidated"),
                "- Parrondo exact policy path consolidates: "
                + str(parrondo.get("final_beat") == "consolidated"),
                "- Nontransitive exact counter path consolidates: "
                + str(nontransitive.get("final_beat") == "consolidated"),
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
    lines.extend(["", "### Kepler Areas steps", ""])
    for step in kepler_steps:
        aha = ((step.get("structured") or {}).get("engineeredAha")) or {}
        lines.append(
            f"- {step.get('step')}: ok={step.get('ok')} beat={aha.get('beat')} earn={aha.get('earn')}"
        )
    lines.extend(["", "### Parrondo's Trap steps", ""])
    for step in parrondo_steps:
        aha = ((step.get("structured") or {}).get("engineeredAha")) or {}
        lines.append(
            f"- {step.get('step')}: ok={step.get('ok')} beat={aha.get('beat')} earn={aha.get('earn')}"
        )
    lines.extend(["", "### Nontransitive Dice steps", ""])
    for step in nontransitive_steps:
        aha = ((step.get("structured") or {}).get("engineeredAha")) or {}
        lines.append(
            f"- {step.get('step')}: ok={step.get('ok')} beat={aha.get('beat')} earn={aha.get('earn')}"
        )
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def write_synthesis(
    results: list[
        tuple[
            Persona,
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
        ]
    ],
    initialize: dict[str, Any],
) -> Path:
    path = OUT / "SYNTHESIS.md"
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    all_pass = all(
        times["passed"]
        and buffon["passed"]
        and kepler["passed"]
        and parrondo["passed"]
        and nontransitive["passed"]
        for _, times, buffon, kepler, parrondo, nontransitive in results
    ) and initialize.get("ok")
    lines = [
        "# Round 07 synthesis: flagship aha over MCP",
        "",
        f"Date: {now}",
        "",
        "## Evidence boundary",
        "",
        "Agent-cohort MCP scripts only. Not participant human hallway evidence.",
        "This suite is a required CI gate on the agent-and-machine track to 1.0.",
        "Optional human stranger panels remain a parallel claim, not an am-exit.",
        "",
        f"## Machine script: {'PASS' if all_pass else 'FAIL'}",
        "",
        f"- initialize instructions: {initialize.get('ok')}",
        f"- place_wager taught: {initialize.get('has_place_wager')}",
        f"- number_wager taught: {initialize.get('has_number_wager')}",
        f"- speed_wager taught: {initialize.get('has_speed_wager')}",
        f"- policy_wager taught: {initialize.get('has_policy_wager')}",
        f"- die_choice taught: {initialize.get('has_die_choice')}",
        f"- counter_wager taught: {initialize.get('has_counter_wager')}",
        f"- describe_room stays safe: {initialize.get('safe_describe')}",
        f"- reveal_room is earned: {initialize.get('earned_reveal')}",
        "",
    ]
    for persona, times, buffon, kepler, parrondo, nontransitive in results:
        combined = (
            times["findings"]
            + buffon["findings"]
            + kepler["findings"]
            + parrondo["findings"]
            + nontransitive["findings"]
        )
        lines.append(
            f"- {persona.title}: times={times['passed']} buffon={buffon['passed']} "
            f"kepler={kepler['passed']} "
            f"parrondo={parrondo['passed']} "
            f"nontransitive={nontransitive['passed']} "
            f"findings={combined if combined else ['none']}"
        )
    lines.extend(
        [
            "",
            "## Convergent engineering claims (if PASS)",
            "",
            "1. Cold open does not leak sampled flagship reveal text.",
            "2. Named wager fields withhold reveal until aha_summon.",
            "3. Truth summon consolidates and unlocks punchline reveal.",
            "4. engineeredAha is present for agent discovery on all five sampled flagships.",
            "5. initialize teaches safe discovery and earned reveal for digital minds.",
            "",
        ]
    )
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def cohort_summary(
    initialize: dict[str, Any],
    times_score: dict[str, Any],
    buffon_score: dict[str, Any],
    kepler_score: dict[str, Any],
    parrondo_score: dict[str, Any],
    nontransitive_score: dict[str, Any],
) -> dict[str, Any]:
    """Return a machine-readable pass/fail summary for CI and audits."""
    passed = bool(
        initialize.get("ok")
        and times_score.get("passed")
        and buffon_score.get("passed")
        and kepler_score.get("passed")
        and parrondo_score.get("passed")
        and nontransitive_score.get("passed")
    )
    findings = list(times_score.get("findings") or []) + list(
        buffon_score.get("findings") or []
    ) + list(kepler_score.get("findings") or []) + list(
        parrondo_score.get("findings") or []
    ) + list(nontransitive_score.get("findings") or [])
    return {
        "suite": "agent-hallway",
        "passed": passed,
        "initialize_ok": bool(initialize.get("ok")),
        "times_tables_passed": bool(times_score.get("passed")),
        "buffon_passed": bool(buffon_score.get("passed")),
        "kepler_passed": bool(kepler_score.get("passed")),
        "parrondo_passed": bool(parrondo_score.get("passed")),
        "nontransitive_passed": bool(nontransitive_score.get("passed")),
        "findings": findings,
        "personas": len(PERSONAS),
        "evidence_class": "agent-mcp-machine",
    }


def main() -> int:
    results: list[
        tuple[
            Persona,
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
            dict[str, Any],
        ]
    ] = []
    # One shared script per room; personas re-score the same machine evidence
    # through different lenses (cheap, deterministic cohort).
    initialize = initialize_script()
    times_steps = times_tables_script()
    buffon_steps = buffon_script()
    kepler_steps = kepler_script()
    parrondo_steps = parrondo_script()
    nontransitive_steps = nontransitive_script()
    times_score = score_times(times_steps)
    buffon_score = score_buffon(buffon_steps)
    kepler_score = score_kepler(kepler_steps)
    parrondo_score = score_parrondo(parrondo_steps)
    nontransitive_score = score_nontransitive(nontransitive_steps)
    for persona in PERSONAS:
        write_persona_note(
            persona,
            times_score,
            buffon_score,
            kepler_score,
            parrondo_score,
            nontransitive_score,
            times_steps,
            buffon_steps,
            kepler_steps,
            parrondo_steps,
            nontransitive_steps,
        )
        results.append(
            (
                persona,
                times_score,
                buffon_score,
                kepler_score,
                parrondo_score,
                nontransitive_score,
            )
        )
    synthesis = write_synthesis(results, initialize)
    summary = cohort_summary(
        initialize,
        times_score,
        buffon_score,
        kepler_score,
        parrondo_score,
        nontransitive_score,
    )
    summary_path = OUT / "summary.json"
    OUT.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(PERSONAS)} persona notes and {synthesis}")
    print(f"wrote {summary_path}")
    print(
        "initialize",
        initialize.get("ok"),
        "times_tables",
        times_score["passed"],
        "buffon",
        buffon_score["passed"],
        "kepler",
        kepler_score["passed"],
        "parrondo",
        parrondo_score["passed"],
        "nontransitive",
        nontransitive_score["passed"],
        "findings",
        times_score["findings"]
        + buffon_score["findings"]
        + kepler_score["findings"]
        + parrondo_score["findings"]
        + nontransitive_score["findings"],
    )
    print("--- summary.json ---")
    print(json.dumps(summary, sort_keys=True))
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
