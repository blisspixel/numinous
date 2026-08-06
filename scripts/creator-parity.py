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
is not covered here; its curve sampling, discards and framing are held to the
same core rule by `numinous_app::studio_render`'s own tests.

`sing` is covered now, and the sentence that used to sit here said it could not
be: one face returns a WAV and the other a note list, so there was no single
artifact to compare. That was the wrong conclusion from a true observation. The
two describe the same melody, so the WAV is measured for the pitch it actually
holds at each onset and checked against the frequency the other face names.

Skipping it had hidden a real defect in both faces at once. The terminal face
fixed the knob at 0 and offered no way to set it, so `sin(a*x)` sang a flat
line; this face fixed it at 1 and rejected the argument, so it could only ever
sing `sin(x)`. Both faces plot with a settable knob defaulting to 1, and
neither could sing with one. Note that a duration and a note count would not
have caught it: those are identical either way, so a gate built on what the CLI
prints would have watched it go past.

Known gap this gate cannot close, recorded rather than worked around: MCP has no
way to open a saved `.num` document at all. A human can save a creation and an
MCP peer cannot read it, so the "remix the same musical document" half of the
0.7 exit is not merely untested, it is unbuilt. Adding a tool for it changes the
pinned tool inventory and is a product decision.
"""

from __future__ import annotations

import json
import os
import re
import struct
import subprocess
import sys
import tempfile
import wave
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "creator-parity"

# The gates share one way of getting the binaries they test; see gate_cli.py
# for why there is only one copy of it.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import GateError, build_and_locate  # noqa: E402

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


def isolated_env(home: Path) -> dict[str, str]:
    """An environment whose player state cannot reach the person running this.

    `plot_expression` is a tool call, and tool calls can persist journey and
    score deltas. Without this, running the gate from `scripts/check` would
    write into a developer's own play history.

    The caller owns `home` and is expected to remove it. This used to make its
    own directory and never delete it, leaving one behind per case on every run.
    """
    env = dict(os.environ)
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["NUMINOUS_HOME"] = str(home / "install")
    env["NUMINOUS_JOURNEY"] = str(home / "journey")
    env["NUMINOUS_SCORES"] = str(home / "scores")
    env["NUMINOUS_JOURNAL"] = str(home / "journal")
    env["NUMINOUS_CAIRN"] = str(home / "cairn")
    return env


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
    return mcp_tool(mcp, "plot_expression", arguments, env)


def mcp_tool(
    mcp: str, tool: str, arguments: dict[str, Any], env: dict[str, str]
) -> str:
    """Call one MCP tool and return its text, or raise with the reason.

    The plumbing lives here rather than in each caller so a second tool cannot
    end up with its own slightly different idea of what an error looks like.
    """
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"_meta": META, "name": tool, "arguments": arguments},
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


# Sing cases: the same expression, note count and knob through both faces.
# `sin(a*x)` is first on purpose. It is the case that found the defect: the CLI
# fixed the knob at 0 and offered no way to set it, so it sang a flat line while
# MCP sang `sin(x)`, and this gate used to skip `sing` entirely.
SING_CASES: tuple[tuple[str, str, int, float | None], ...] = (
    ("knob expression", "sin(a*x)", 24, 1.0),
    ("plain expression", "sin(x)", 24, 1.0),
    ("knob at another value", "sin(a*x)", 16, 2.5),
    ("knob that flattens it", "sin(a*x)", 16, 0.0),
    # A knob of None asks both faces without naming one, so the two DEFAULTS
    # have to agree. Every case above passes the knob explicitly, which means
    # they would all still pass if one face's default drifted, and a player who
    # never names the knob is the common case rather than the rare one.
    ("neither face is told the knob", "sin(a*x)", 24, None),
    ("neither face is told the knob, plain", "sin(x)", 24, None),
)

# Ranges, which this face could not vary at all until now: it sang one window
# and a person could ask for any span. Each entry is a label and the window.
SING_RANGES: tuple[tuple[str, float, float], ...] = (
    ("narrow window", -1.0, 1.0),
    ("offset window", 0.0, 10.0),
    ("wide window", -20.0, 20.0),
)

# note  1:   440.0 Hz ( A4)  at  0.00s
MCP_NOTE = re.compile(r"note\s+(\d+):\s+([\d.]+) Hz.*?at\s+([\d.]+)s")


def mcp_sing(mcp: str, arguments: dict[str, Any], env: dict[str, str]) -> list[tuple[float, float]]:
    """The frequency and onset of every note MCP reports."""
    text = mcp_tool(mcp, "sing_expression", arguments, env)
    notes = [(float(hz), float(at)) for _, hz, at in MCP_NOTE.findall(text)]
    if not notes:
        raise ParityError(f"MCP reported no notes for {arguments}: {text[:300]}")
    return notes


def wav_note_frequencies(path: Path, onsets: list[float]) -> list[float]:
    """The dominant frequency the WAV actually holds at each onset.

    Read from the audio rather than from anything the CLI says about it. The
    CLI prints a duration and a note count, and those two are identical whether
    the knob is 0 or 1, so a gate built on them would have watched this defect
    go by. The notes are pure tones, so counting zero crossings inside a window
    is enough and needs no dependency.
    """
    with wave.open(str(path), "rb") as handle:
        if handle.getsampwidth() != 2:
            raise ParityError(f"{path} is not 16-bit PCM")
        rate = handle.getframerate()
        channels = handle.getnchannels()
        frames = handle.readframes(handle.getnframes())
    samples = struct.unpack(f"<{len(frames) // 2}h", frames)
    if channels > 1:
        samples = samples[::channels]

    measured = []
    for index, onset in enumerate(onsets):
        end = onsets[index + 1] if index + 1 < len(onsets) else onset + (onsets[1] - onsets[0])
        # Trim both edges: the envelope's attack and release cross zero in ways
        # that have nothing to do with pitch.
        span = end - onset
        start_frame = int((onset + span * 0.25) * rate)
        stop_frame = int((onset + span * 0.75) * rate)
        window = samples[start_frame:stop_frame]
        if len(window) < 8:
            raise ParityError(f"{path} has no audio for the note at {onset}s")
        crossings = sum(
            1
            for first, second in zip(window, window[1:])
            if (first < 0) != (second < 0)
        )
        seconds = len(window) / rate
        measured.append(crossings / (2.0 * seconds))
    return measured


def check_sing(
    cli: str,
    mcp: str,
    label: str,
    source: str,
    notes: int,
    knob: float | None,
    env: dict[str, str],
    xmin: float | None = None,
    xmax: float | None = None,
) -> dict[str, Any]:
    """One face's audio must hold the pitches the other face names."""
    window = "" if xmin is None else f" over [{xmin}, {xmax}]"
    name = (
        f"sing {label}: {source} a={'default' if knob is None else knob} "
        f"notes={notes}{window}"
    )
    try:
        arguments: dict[str, Any] = {"expr": source, "notes": notes}
        if knob is not None:
            arguments["a"] = knob
        if xmin is not None:
            arguments["xmin"] = xmin
            arguments["xmax"] = xmax
        reported = mcp_sing(mcp, arguments, env)
        with tempfile.TemporaryDirectory(
            prefix="numinous-sing-parity-", ignore_cleanup_errors=True
        ) as workspace:
            wav = Path(workspace) / "sung.wav"
            command = [cli, "sing", source, "--notes", str(notes)]
            if knob is not None:
                command += [f"--a={knob}"]
            if xmin is not None:
                command += [f"--xmin={xmin}", f"--xmax={xmax}"]
            command += ["--out", str(wav)]
            result = subprocess.run(
                command,
                env=env, capture_output=True, text=True, encoding="utf-8", timeout=120,
            )
            if result.returncode != 0:
                raise ParityError(f"the CLI rejected it: {result.stderr[-300:]}")
            measured = wav_note_frequencies(wav, [onset for _, onset in reported])

        if len(measured) != len(reported):
            raise ParityError(
                f"MCP named {len(reported)} notes and the WAV held {len(measured)}"
            )
        worst = 0.0
        worst_note = 0
        for index, ((expected, _), actual) in enumerate(zip(reported, measured), start=1):
            error = abs(actual - expected) / max(expected, 1.0)
            if error > worst:
                worst, worst_note = error, index
        # Zero-crossing counting over a short window is coarse; 6 percent is
        # far tighter than the gap this catches, where a silenced knob moves
        # every note to one pitch.
        if worst > 0.06:
            raise ParityError(
                f"note {worst_note} is {worst * 100:.1f} percent away from the "
                f"frequency MCP named"
            )
    except ParityError as error:
        return {"name": name, "passed": False, "detail": str(error)}
    return {
        "name": name,
        "passed": True,
        "detail": f"{len(reported)} notes, worst pitch error {worst * 100:.1f} percent",
    }


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
        built = build_and_locate(("numinous", "numinous-mcp"))
        cli, mcp = str(built[0]), str(built[1])
    except (ParityError, GateError) as error:
        results = [{"name": "build", "passed": False, "detail": str(error)}]
    else:
        # A fresh profile per case, so no case can change what another sees,
        # and each one is removed once its case is done.
        results = []
        for label, mcp_args, cli_args in CASES:
            with tempfile.TemporaryDirectory(
                prefix="numinous-creator-parity-", ignore_cleanup_errors=True
            ) as home:
                results.append(
                    check(cli, mcp, label, mcp_args, cli_args, isolated_env(Path(home)))
                )
        for label, source, notes, knob in SING_CASES:
            with tempfile.TemporaryDirectory(
                prefix="numinous-creator-parity-", ignore_cleanup_errors=True
            ) as home:
                results.append(
                    check_sing(
                        cli, mcp, label, source, notes, knob, isolated_env(Path(home))
                    )
                )
        for label, xmin, xmax in SING_RANGES:
            with tempfile.TemporaryDirectory(
                prefix="numinous-creator-parity-", ignore_cleanup_errors=True
            ) as home:
                results.append(
                    check_sing(
                        cli, mcp, label, "sin(a*x)", 16, 1.5,
                        isolated_env(Path(home)), xmin=xmin, xmax=xmax,
                    )
                )
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
