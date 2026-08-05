#!/usr/bin/env python3
"""Machine acceptance for creator parity between the CLI and MCP (0.7-am).

The 0.7 exit asks that the same creation produce event-identical output through
every face. `creator-roundtrip.py` already proves the CLI can save a `.num` and
reopen it byte-identically. Nothing checked that a second face draws the same
picture from the same inputs, so the two could drift apart and every existing
gate would stay green.

Both faces are driven here with the same expression, recipe, seed, knob, and
range, and their plots must match exactly. Only the plot body is compared:
headers and discovery chrome are each face speaking in its own voice, and
requiring those to match would be requiring the faces to be the same thing
rather than to agree about the mathematics.

This is machine evidence for two faces. The App's Studio panel is the third and
is not covered here. Neither is `sing`, whose two faces return a WAV and a note
list rather than one comparable artifact.

Known gap this gate cannot close, recorded rather than worked around: MCP has no
way to open a saved `.num` document at all. A human can save a creation and an
MCP peer cannot read it, so the "remix the same musical document" half of the
0.7 exit is not merely untested, it is unbuilt. Adding a tool for it changes the
pinned tool inventory and is a product decision.
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "creator-parity"

PROTOCOL_VERSION = "2026-07-28"
META = {
    "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
    "io.modelcontextprotocol/clientCapabilities": {},
    "io.modelcontextprotocol/clientInfo": {"name": "creator-parity", "version": "1"},
}

# Each case: a label, the MCP arguments, and the CLI arguments that mean the
# same thing. Ranges use --flag=value because a bare -2 is read as a flag.
CASES: tuple[tuple[str, dict[str, Any], list[str]], ...] = (
    ("expression", {"expr": "sin(x)"}, ["sin(x)"]),
    ("harmonic", {"expr": "sin(2*x)"}, ["sin(2*x)"]),
    ("parabola", {"expr": "x*x"}, ["x*x"]),
    ("singularity", {"expr": "1/x"}, ["1/x"]),
    ("knob", {"expr": "sin(a*x)", "a": 2.5}, ["sin(a*x)", "--a=2.5"]),
    ("negative knob", {"expr": "sin(a*x)", "a": -3}, ["sin(a*x)", "--a=-3"]),
    ("range", {"expr": "x*x", "xmin": -2, "xmax": 2}, ["x*x", "--xmin=-2", "--xmax=2"]),
    ("offset range", {"expr": "sin(x)", "xmin": 0, "xmax": 10},
     ["sin(x)", "--xmin=0", "--xmax=10"]),
    ("recipe", {"recipe": 0}, ["--recipe=0"]),
    ("later recipe", {"recipe": 3}, ["--recipe=3"]),
    ("seed", {"seed": 7}, ["--seed=7"]),
    ("later seed", {"seed": 42}, ["--seed=42"]),
)


class ParityError(RuntimeError):
    """A face could not be driven."""


def isolated_env() -> dict[str, str]:
    """An environment whose player state cannot reach the person running this.

    `plot_expression` is a tool call, and tool calls can persist journey and
    score deltas. Without this, running the gate from `scripts/check` would
    write into a developer's own play history.
    """
    env = dict(os.environ)
    home = Path(tempfile.mkdtemp(prefix="numinous-creator-parity-"))
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["NUMINOUS_HOME"] = str(home / "install")
    env["NUMINOUS_JOURNEY"] = str(home / "journey")
    env["NUMINOUS_SCORES"] = str(home / "scores")
    env["NUMINOUS_JOURNAL"] = str(home / "journal")
    env["NUMINOUS_CAIRN"] = str(home / "cairn")
    return env


def build_faces() -> tuple[str, str]:
    """Build both faces, then return the binaries that build produced.

    This compares live behaviour, so it has to compare the behaviour of the
    current source. Picking up whichever binary happened to be on disk would
    let a stale artifact answer for code that no longer exists, and the gate
    would agree with itself about a version nobody is running. Cargo is
    incremental, so this costs almost nothing on an already-built tree.
    """
    build = subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "numinous", "--bin", "numinous-mcp"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if build.returncode != 0:
        raise ParityError("cannot build the faces under test:\n" + build.stderr)
    # CARGO_TARGET_DIR redirects where cargo writes, and several CI layouts set
    # it, so a build that succeeded could still look missing under ROOT/target.
    # A relative CARGO_TARGET_DIR is resolved by cargo against its own working
    # directory, which is ROOT here. Resolving it against this process's
    # directory instead would look in the wrong place after a good build.
    configured = os.environ.get("CARGO_TARGET_DIR")
    target_root = Path(configured) if configured else ROOT / "target"
    if not target_root.is_absolute():
        target_root = ROOT / target_root
    target = target_root / "debug"
    found = []
    for name in ("numinous", "numinous-mcp"):
        for candidate in (target / f"{name}.exe", target / name):
            if candidate.is_file():
                found.append(str(candidate))
                break
        else:
            raise ParityError(f"cargo build succeeded but {name} is not under {target}")
    return found[0], found[1]


def plot_body(text: str) -> str:
    """The plot itself, without either face's chrome.

    The header and the discovery line are how each face introduces the result
    in its own voice. What has to agree is the drawing.

    The block is taken by position, not by looking for ink. Both faces print
    their chrome, one blank separator, then exactly the rows they were asked
    for. Trimming blank rows instead would make the height depend on where the
    ink happened to land, and an earlier version did that: it derived the CLI's
    requested height from the trimmed count, so a plot whose top row was empty
    asked the CLI for a shorter plot than MCP had drawn and the two could never
    agree. It passed on Windows and failed on Linux, because which rows come
    out blank moves with the floating point.
    """
    rows = [
        line.rstrip()
        for line in text.split("\n")
        if not line.startswith("y = ") and not line.startswith("Discovery:")
    ]
    # The single separator between the chrome and the drawing.
    if rows and not rows[0]:
        rows.pop(0)
    # The empty string left by the final newline, and nothing more.
    if rows and not rows[-1]:
        rows.pop()
    return "\n".join(rows)


def plot_rows(text: str, expected: int) -> str:
    """Exactly `expected` drawing rows, and nothing that follows them.

    The CLI is a player-facing face: a plot that earns enough experience prints
    LEVEL UP, BOON BANKED, and UNLOCKED lines underneath the drawing. Those are
    the Journey speaking, not the mathematics, and an earlier version compared
    them too. It failed on whichever case happened to cross a level, which is
    why the same twelve cases passed here and failed in CI: the two profiles
    had different amounts of play behind them.
    """
    return "\n".join(plot_body(text).split("\n")[:expected])


def mcp_plot(mcp: str, arguments: dict[str, Any], env: dict[str, str]) -> str:
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"_meta": META, "name": "plot_expression", "arguments": arguments},
    }
    try:
        result = subprocess.run(
            [mcp], input=json.dumps(request) + "\n", env=env,
            capture_output=True, text=True, encoding="utf-8", timeout=120,
        )
    except subprocess.TimeoutExpired as error:
        raise ParityError(f"MCP did not answer {arguments} within its budget") from error
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("id") != 1:
            continue
        payload = message.get("result")
        if payload is None:
            raise ParityError(f"MCP returned no result for {arguments}: {message}")
        if payload.get("isError"):
            raise ParityError(
                f"MCP rejected {arguments}: {payload['content'][0]['text']}"
            )
        return payload["content"][0]["text"]
    raise ParityError(f"MCP produced no reply for {arguments}: {result.stderr[-400:]}")


def cli_plot(
    cli: str, arguments: list[str], width: int, height: int, env: dict[str, str]
) -> str:
    try:
        result = subprocess.run(
            [cli, "plot", *arguments, "--width", str(width), "--height", str(height)],
            env=env, capture_output=True, text=True, encoding="utf-8", timeout=120,
        )
    except subprocess.TimeoutExpired as error:
        raise ParityError(f"the CLI did not answer {arguments} within its budget") from error
    if result.returncode != 0:
        raise ParityError(
            f"CLI rejected {arguments}: exit {result.returncode}, {result.stderr[-400:]}"
        )
    return result.stdout


def check(
    cli: str,
    mcp: str,
    label: str,
    mcp_args: dict[str, Any],
    cli_args: list[str],
    env: dict[str, str],
) -> dict[str, Any]:
    try:
        drawn = plot_body(mcp_plot(mcp, mcp_args, env))
        if not drawn:
            return {"name": label, "passed": False, "detail": "MCP drew nothing to compare"}
        rows = drawn.split("\n")
        # Drive the CLI at whatever geometry MCP chose, since MCP takes no
        # width or height of its own.
        height = len(rows)
        width = max(len(row) for row in rows)
        # Read back exactly the geometry that was asked for, so anything the
        # face prints under the drawing cannot enter the comparison.
        mirrored = plot_rows(cli_plot(cli, cli_args, width, height, env), height)
    except ParityError as error:
        return {"name": label, "passed": False, "detail": str(error)}

    if drawn == mirrored:
        return {
            "name": label,
            "passed": True,
            "detail": f"both faces drew the same {width} by {height} plot",
        }
    first = next(
        (
            index
            for index, (a, b) in enumerate(zip(mirrored.split("\n"), rows))
            if a != b
        ),
        None,
    )
    where = f"row {first}" if first is not None else "line count"
    return {
        "name": label,
        "passed": False,
        "detail": f"the faces disagree at {where} for {width} by {height}",
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    try:
        cli, mcp = build_faces()
    except ParityError as error:
        results = [{"name": "build", "passed": False, "detail": str(error)}]
    else:
        # A fresh profile per case, so no case can change what another sees.
        results = [
            check(cli, mcp, label, mcp_args, cli_args, isolated_env())
            for label, mcp_args, cli_args in CASES
        ]
    failed = [item for item in results if not item["passed"]]
    summary = {
        "suite": "creator-parity",
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
        print(f"  {'PASS' if item['passed'] else 'FAIL'}  {item['name']}: {item['detail']}")
    print("--- summary.json ---")
    print(json.dumps(
        {k: summary[k] for k in ("suite", "passed", "check_count", "failed_count")},
        sort_keys=True,
    ))
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
