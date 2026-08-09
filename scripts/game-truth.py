#!/usr/bin/env python3
"""Machine acceptance that game outcomes depend on the play just made.

Three regressions from the 2026-08 game-flow hunt, each a case where what the
player saw did not depend on what actually happened, held here at the process
boundary because each one lives in the CLI's stdin loops and score
persistence, which unit tests cannot reach without the environment isolation
this harness provides.

1. Fifteen keeps its partial score. A departure after two graded calls posts
   `fifteen seed:N rounds:2`, the way seti, aliens, and quiz always did; it
   used to post nothing and four correct calls could vanish silently.
2. An abandoned bench posts no composite. The bench once rebuilt its number
   from the scoreboard's memory of a better day, so an entirely unplayed
   bench could print a full historical composite. Now a run that ends early
   abandons the bench with its reason named, and nothing posts.
3. The bomb charges no wire for help or typos. Stage four of the gauntlet
   spent one of its five wires on a `?` or a three-digit typo, unlike every
   other stage and standalone crack. Two runs differing only in junk before
   the same first real guess must now post the same total.

Each check runs the real binary in an isolated profile, so nothing here can
reach the player history of whoever runs the gate.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import GateError, build_and_locate  # noqa: E402


class GameTruthError(RuntimeError):
    """A game outcome did not depend on the play just made."""


def isolated_env(home: Path) -> dict[str, str]:
    """An environment whose player state cannot reach the person running this.

    Every command here posts scores and journey deltas; without this, running
    the gate locally would write into a developer's own play history.
    """
    env = dict(os.environ)
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["NUMINOUS_JOURNEY"] = str(home / "journey")
    env["NUMINOUS_SCORES"] = str(home / "scores")
    return env


def run_game(
    cli: Path, home: Path, arguments: list[str], stdin: str
) -> subprocess.CompletedProcess[str]:
    """Run one game command against an isolated profile."""
    return subprocess.run(
        [str(cli), *arguments],
        input=stdin,
        capture_output=True,
        text=True,
        timeout=120,
        env=isolated_env(home),
        check=False,
    )


def scores_text(home: Path) -> str:
    """The isolated profile's score table, or empty when nothing posted."""
    path = home / "scores"
    return path.read_text(encoding="utf-8") if path.exists() else ""


def check_fifteen_partial(cli: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        home = Path(raw)
        run_game(cli, home, ["fifteen", "--seed", "7", "--rounds", "5"], "S\nS\n")
        table = scores_text(home)
        if "fifteen seed:7 rounds:2" not in table:
            raise GameTruthError(
                "fifteen dropped its partial score on departure; the table "
                f"holds: {table!r}"
            )


def check_bench_abandoned(cli: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        home = Path(raw)
        outcome = run_game(cli, home, ["bench"], "")
        if "BENCH ABANDONED" not in outcome.stdout:
            raise GameTruthError(
                "an unplayed bench did not say it was abandoned; stdout ends: "
                f"{outcome.stdout[-300:]!r}"
            )
        table = scores_text(home)
        if "bench v1" in table:
            raise GameTruthError(
                "an unplayed bench posted a composite; history must not wear "
                f"the run's face: {table!r}"
            )


def parse_total(stdout: str) -> int:
    """The gauntlet's one honest number, from its RUN COMPLETE line."""
    match = re.search(r"RUN COMPLETE\s+\d/4 clean\s+TOTAL (-?\d+)", stdout)
    if match is None:
        raise GameTruthError(
            f"no RUN COMPLETE line to read a total from; stdout ends: {stdout[-300:]!r}"
        )
    return int(match.group(1))


def parse_bomb_code(stdout: str) -> str:
    """The four-digit code the BOOM line reveals after a lost bomb stage."""
    match = re.search(r"It was (\d{4})\.", stdout)
    if match is None:
        raise GameTruthError(
            f"no BOOM reveal to learn the code from; stdout ends: {stdout[-300:]!r}"
        )
    return match.group(1)


def check_bomb_charges_no_wire_for_junk(cli: Path) -> None:
    seed = ["gauntlet", "--seed", "11"]
    # Learn the fixed seed's code by losing once: five wrong wires reveal it.
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        lost = run_game(cli, Path(raw), seed, "0\nA\nA\n" + "0000\n" * 5)
        code = parse_bomb_code(lost.stdout)
    # Two runs identical through stage three; the noisy one asks for help and
    # mistypes twice before the same first real guess.
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        clean = run_game(cli, Path(raw), seed, f"0\nA\nA\n{code}\n")
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        noisy = run_game(cli, Path(raw), seed, f"0\nA\nA\n?\nabc\n12\n{code}\n")
    clean_total = parse_total(clean.stdout)
    noisy_total = parse_total(noisy.stdout)
    if clean_total != noisy_total:
        raise GameTruthError(
            "help or a typo burned a bomb wire: a first-guess defuse scored "
            f"{clean_total} clean but {noisy_total} after junk input"
        )


MCP_META = {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientCapabilities": {},
    "io.modelcontextprotocol/clientInfo": {"name": "game-truth", "version": "1"},
}


def mcp_tool_text(mcp: Path, home: Path, tool: str, arguments: dict[str, Any]) -> str:
    """Call one MCP tool in the isolated profile and return its text."""
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"_meta": MCP_META, "name": tool, "arguments": arguments},
    }
    result = subprocess.run(
        [str(mcp)],
        input=json.dumps(request) + "\n",
        capture_output=True,
        text=True,
        timeout=120,
        env=isolated_env(home),
        check=False,
    )
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("id") != 1:
            continue
        # A JSON-RPC error or a missing result is the real failure; hiding it
        # behind an empty string would make a later check fail with a
        # misleading parse message instead of the actual refusal.
        if "error" in message:
            raise GameTruthError(f"MCP refused {tool}: {message['error']!r}")
        payload = message.get("result")
        if not isinstance(payload, dict):
            raise GameTruthError(f"MCP returned no result for {tool}: {message!r}")
        content = payload.get("content") or [{}]
        return str(content[0].get("text", ""))
    raise GameTruthError(
        f"MCP produced no reply for {tool} (exit {result.returncode}): "
        f"{result.stderr[-300:]!r}"
    )


def check_fifteen_levels_the_same_on_both_faces(cli: Path, mcp: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        home = Path(raw)
        reply = mcp_tool_text(
            mcp, home, "fifteen", {"seed": 7, "rounds": 5, "calls": ["S", "S", "S"]}
        )
        if "of 3 called" not in reply:
            raise GameTruthError(
                f"MCP graded rounds it never saw: {reply.splitlines()[-1]!r}"
            )
        # The same three graded calls through the terminal face, in a second
        # profile; both ledgers must tell the same XP story, or the same
        # session levels differently depending on which face heard it.
        with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw_cli:
            cli_home = Path(raw_cli)
            run_game(
                cli, cli_home, ["fifteen", "--seed", "7", "--rounds", "3"], "S\nS\nS\n"
            )
            mcp_journey = mcp_tool_text(mcp, home, "journey", {})
            cli_journey = (cli_home / "journey").read_text(encoding="utf-8")
            mcp_xp = re.search(r"(\d+) XP", mcp_journey)
            if mcp_xp is None:
                raise GameTruthError(f"no XP line in the MCP journey: {mcp_journey!r}")
            plays = re.search(r"plays (\d+)", cli_journey)
            wins = re.search(r"wins (\d+)", cli_journey)
            if plays is None or wins is None:
                raise GameTruthError(f"unreadable CLI journey: {cli_journey!r}")
            cli_xp = int(plays.group(1)) + 2 * int(wins.group(1))
            if int(mcp_xp.group(1)) != cli_xp:
                raise GameTruthError(
                    "the same fifteen session levels differently per face: "
                    f"MCP says {mcp_xp.group(1)} XP, the terminal ledger adds "
                    f"to {cli_xp}"
                )


def check_an_unreadable_journey_is_named_not_faked(cli: Path) -> None:
    """A journey that exists and cannot be read is not a fresh player.

    Treating it as one silently demoted the rank, closed the veil, and
    re-announced the same level on every run, forever, because nothing was
    ever written. The run must say what happened and must not celebrate a
    crossing it cannot see.
    """
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        home = Path(raw)
        journey = home / "journey"
        # Not text: readable bytes that are not valid UTF-8, which is the
        # shape a truncated or clobbered file actually takes.
        journey.write_bytes(b"plays 40\nwins 30\n\xff\xfe not text\n")
        outcome = run_game(cli, home, ["plot", "sin(x)", "--width", "20", "--height", "8"], "")
        said = outcome.stderr
        if "could not be read" not in said:
            raise GameTruthError(
                "an unreadable journey was treated as a fresh player with no "
                f"word to the player; stderr held: {said!r}"
            )
        if "LEVEL UP" in outcome.stdout or "TROPHY" in outcome.stdout:
            raise GameTruthError(
                "a run that cannot see the journey announced a crossing "
                f"anyway: {outcome.stdout!r}"
            )
        if "could not be saved" in said:
            raise GameTruthError(
                "one cause was told twice: the run explained the unreadable "
                "journey and then tried the write anyway, which can only fail "
                f"against the same condition; stderr held: {said!r}"
            )
        if "  " in said:
            raise GameTruthError(
                f"the player's copy carries a run of spaces: {said!r}"
            )
        after = journey.read_bytes()
        if b"plays 40" not in after:
            raise GameTruthError(
                "the unreadable journey was overwritten; the player's history "
                "must survive a run that could not read it"
            )


def check_the_save_note_rides_the_reply_that_lost(mcp: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="numinous-game-truth-") as raw:
        home = Path(raw)
        # The scores path is a directory, so every score write refuses.
        (home / "scores").mkdir(parents=True)
        reply = mcp_tool_text(
            mcp, home, "fifteen", {"seed": 7, "rounds": 3, "calls": ["S", "S", "S"]}
        )
        if "NOTE: a local save failed" not in reply:
            raise GameTruthError(
                "the reply that lost a score write does not say so; it used "
                f"to land on the next reply instead: {reply.splitlines()[-1]!r}"
            )


def main() -> int:
    try:
        [cli, mcp] = build_and_locate(("numinous", "numinous-mcp"))
    except GateError as error:
        print(f"game-truth: {error}", file=sys.stderr)
        return 1
    checks = (
        ("fifteen keeps its partial score", lambda: check_fifteen_partial(cli)),
        ("an abandoned bench posts no composite", lambda: check_bench_abandoned(cli)),
        (
            "the bomb charges no wire for junk",
            lambda: check_bomb_charges_no_wire_for_junk(cli),
        ),
        (
            "fifteen levels the same on both faces",
            lambda: check_fifteen_levels_the_same_on_both_faces(cli, mcp),
        ),
        (
            "a lost save is named on the reply that lost it",
            lambda: check_the_save_note_rides_the_reply_that_lost(mcp),
        ),
        (
            "an unreadable journey is named, never faked",
            lambda: check_an_unreadable_journey_is_named_not_faked(cli),
        ),
    )
    for label, check in checks:
        try:
            check()
        except GameTruthError as error:
            print(f"game-truth: FAIL {label}: {error}", file=sys.stderr)
            return 1
        print(f"game-truth: ok  {label}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
