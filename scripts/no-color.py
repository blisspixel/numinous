#!/usr/bin/env python3
"""Machine acceptance for NO_COLOR across the whole terminal face (0.5-am).

`NO_COLOR` is honored surface by surface, and every surface has a Rust test.
What no test covered was the binary as a whole: a new subcommand can emit color
and every existing test stays green, because none of them knows it exists. That
is not hypothetical. The room pictures honored the setting for several releases
while the Munch arcade, the Hackenbush garden and the Party matrix did not, and
each was found by hand rather than by a gate.

So this sweeps. Every subcommand the CLI advertises is either driven here or
named in SKIPPED with a reason, and the list of subcommands is read from the
binary's own help rather than written down twice. Adding a subcommand therefore
fails this gate until someone decides which it is, which is the only way a sweep
stays a sweep.

Each probe runs twice. With `NO_COLOR` set it must emit no SGR escape at all.
Without it, the run is measured but not required to be colorful, since plenty of
surfaces are plain text either way. What is required is that the sweep as a whole
sees real color somewhere, or it would pass just as happily against a binary that
had lost the ability to draw in color at all.

Cursor control is not color. `\x1b[H`, `\x1b[2J` and `\x1b[K` position and clear,
and a `NO_COLOR` surface is still allowed to paint in place, so only SGR
sequences (those ending in `m`) count.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, NamedTuple

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "no-color"

# Select Graphic Rendition: the escapes that set color, weight and so on. A
# sequence ending in any other letter moves or clears the cursor and is allowed.
SGR = re.compile(r"\x1b\[[0-9;]*m")

# Bytes to keep from a probe. The live loops emit megabytes a second and the
# question is whether any color appears, not how much.
READ_LIMIT = 400_000

# Loops that never exit are killed at this point and judged on what they wrote.
LOOP_DEADLINE_SECONDS = 3.0

# Anything that has not answered by now is a hang, not a slow machine.
ONE_SHOT_TIMEOUT_SECONDS = 60.0

# At least this many probes must show color when color is allowed. Without this
# the whole gate would pass against a binary that emitted no color anywhere,
# which is the one way "no color found" means nothing.
MIN_COLORFUL_PROBES = 3


class Probe(NamedTuple):
    """One way of driving the CLI, and how long to let it run."""

    argv: list[str]
    # None for anything that exits on its own. A number for the live loops,
    # which never do.
    deadline: float | None = None

    @property
    def label(self) -> str:
        return " ".join(self.argv)

    @property
    def subcommand(self) -> str:
        return self.argv[0]


# Games read a line and leave on end of input, so every probe is fed a closed
# stdin. That also keeps a probe from waiting on a player who is not there.
PROBES: tuple[Probe, ...] = (
    Probe(["access"]),
    Probe(["rooms"]),
    Probe(["describe", "lorenz"]),
    Probe(["render", "lorenz", "--width", "40", "--height", "20"]),
    Probe(["render", "lorenz", "--color", "--width", "40", "--height", "20"]),
    Probe(["plot", "sin(x)"]),
    Probe(["sims"]),
    Probe(["sim", "logistic"]),
    Probe(["jokes"]),
    Probe(["journey"]),
    Probe(["choose"]),
    Probe(["scores"]),
    Probe(["trophies"]),
    Probe(["forget"]),
    Probe(["answer"]),
    Probe(["radio"]),
    Probe(["bench"]),
    Probe(["help"]),
    Probe(["open-studio", "no-such-file.num"]),
    # The games, which are where the escapes actually were.
    Probe(["arcade"]),
    Probe(["hackenbush"]),
    Probe(["party"]),
    Probe(["munch"]),
    Probe(["nim"]),
    Probe(["fifteen"]),
    Probe(["crack"]),
    Probe(["seti"]),
    Probe(["aliens"]),
    Probe(["quiz"]),
    Probe(["gauntlet"]),
    # The live loops, which paint in truecolor and never exit.
    Probe(["watch", "lorenz"], deadline=LOOP_DEADLINE_SECONDS),
    Probe(["play", "lorenz"], deadline=LOOP_DEADLINE_SECONDS),
    Probe(
        ["tour", "--mute", "--width", "24", "--height", "12", "--seconds", "1", "--fps", "5"],
        deadline=LOOP_DEADLINE_SECONDS,
    ),
)

# Not driven here, and why. A reason is required: an unexplained skip is how a
# surface stops being checked without anyone deciding that it should.
SKIPPED: dict[str, str] = {
    "update": "downloads and installs a GitHub release",
    "tune2": "needs ELEVENLABS_API_KEY and the network",
    "gallery": "renders all 354 rooms to disk; covered by the goldens gate",
    "contact-sheet": "renders all 354 rooms into one sheet; same cost, same cover",
    "share": "writes a share package; covered by the packaging gates",
    "loop": "writes an APNG; its terminal output is one line, and slow to reach",
    "sonify": "writes a WAV; covered by the room-bed audio goldens",
    "sing": "writes a WAV; covered by the creator parity gate",
    "tune": "writes a WAV; covered by the soundtrack packaging gate",
}


class SweepError(RuntimeError):
    """The CLI could not be driven."""


def resolve_cli() -> str:
    """Build the CLI, then return the binary that build produced.

    This observes live behaviour, so it has to observe the behaviour of the
    current source. Picking up whichever binary happened to be on disk would
    let a stale artifact answer for code that no longer exists.
    """
    build = subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "numinous"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        raise SweepError("cannot build the CLI under test:\n" + build.stderr)
    configured = os.environ.get("CARGO_TARGET_DIR")
    target_root = Path(configured) if configured else ROOT / "target"
    if not target_root.is_absolute():
        target_root = ROOT / target_root
    for name in ("numinous.exe", "numinous"):
        candidate = target_root / "debug" / name
        if candidate.is_file():
            return str(candidate)
    raise SweepError(
        f"cargo build reported success but no numinous binary is under {target_root / 'debug'}"
    )


def isolated_env(home: Path, no_color: bool) -> dict[str, str]:
    """An environment whose play history cannot reach the person running this.

    Several probes are games, and games record scores and journey progress.
    Without this, running the gate would write into a developer's own history.

    The caller owns `home` and is expected to remove it. This used to make its
    own directory and never delete it, which left two behind per probe, sixty
    six per sweep, in everybody's temp directory forever.
    """
    env = dict(os.environ)
    env.pop("NO_COLOR", None)
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["NUMINOUS_HOME"] = str(home / "install")
    env["NUMINOUS_JOURNEY"] = str(home / "journey")
    env["NUMINOUS_SCORES"] = str(home / "scores")
    env["NUMINOUS_JOURNAL"] = str(home / "journal")
    env["NUMINOUS_CAIRN"] = str(home / "cairn")
    if no_color:
        env["NO_COLOR"] = "1"
    return env


def advertised_subcommands(cli: str) -> set[str]:
    """Every subcommand the CLI lists in its own help.

    Read from the binary rather than kept in a list here, so a subcommand that
    is added and forgotten cannot also be forgotten by this gate.
    """
    result = subprocess.run(
        [cli, "--help"], capture_output=True, text=True, timeout=ONE_SHOT_TIMEOUT_SECONDS
    )
    if result.returncode != 0:
        raise SweepError(f"the CLI would not print its help: {result.stderr[-400:]}")
    names: set[str] = set()
    in_commands = False
    for line in result.stdout.splitlines():
        if line.startswith("Commands:"):
            in_commands = True
            continue
        if in_commands:
            if not line.startswith("  "):
                break
            word = line.split()
            if word:
                names.add(word[0])
    if not names:
        raise SweepError("no subcommands found in the CLI help; the sweep would cover nothing")
    return names


def sgr_count(cli: str, probe: Probe, no_color: bool) -> int:
    """How many SGR escapes this probe emits under the given setting."""
    # The profile lives exactly as long as the run that needs it. Cleanup
    # errors are ignored because the live loops are killed rather than asked to
    # stop, and a killed process on Windows can still be holding a file open;
    # failing the sweep over a leftover temporary file would report a colour
    # defect that does not exist.
    with tempfile.TemporaryDirectory(
        prefix="numinous-no-color-", ignore_cleanup_errors=True
    ) as home:
        proc = subprocess.Popen(
            [cli, *probe.argv],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            env=isolated_env(Path(home), no_color),
        )
        try:
            data, _ = proc.communicate(
                input=b"", timeout=probe.deadline or ONE_SHOT_TIMEOUT_SECONDS
            )
        except subprocess.TimeoutExpired:
            if probe.deadline is None:
                proc.kill()
                proc.communicate()
                raise SweepError(f"{probe.label} never answered") from None
            # Expected for the live loops: kill, then read what the pipe holds.
            proc.kill()
            data, _ = proc.communicate()
    return len(SGR.findall(data[:READ_LIMIT].decode("utf-8", "replace")))


def check(cli: str, probe: Probe) -> dict[str, Any]:
    try:
        held = sgr_count(cli, probe, no_color=True)
        allowed = sgr_count(cli, probe, no_color=False)
    except SweepError as error:
        return {"name": probe.label, "passed": False, "colorful": False, "detail": str(error)}
    if held:
        return {
            "name": probe.label,
            "passed": False,
            "colorful": bool(allowed),
            "detail": f"emitted {held} color escapes with NO_COLOR set",
        }
    return {
        "name": probe.label,
        "passed": True,
        "colorful": bool(allowed),
        "detail": (
            f"{allowed} color escapes allowed, none with NO_COLOR set"
            if allowed
            else "plain either way"
        ),
    }


def judge_coverage(advertised: set[str], driven: set[str], skipped: set[str]) -> list[str]:
    """Why this sweep does not cover the binary, or nothing if it does.

    Separated from reading the binary's help so the judgement can be tested on
    its own. This is the part that keeps the sweep a sweep, so it is the part
    most worth being sure about.
    """
    reasons = []
    missing = sorted(advertised - driven - skipped)
    if missing:
        reasons.append(
            "these subcommands are neither driven nor skipped, so nothing checks "
            f"whether they honor NO_COLOR: {', '.join(missing)}"
        )
    # A skip for something that no longer exists is stale bookkeeping, and it
    # hides the fact that the list was never revisited.
    stale = sorted(skipped - advertised)
    if stale:
        reasons.append(f"these are skipped but no longer exist: {', '.join(stale)}")
    # A subcommand in both lists is a contradiction, and whichever way it is
    # read, someone believed the other one.
    both = sorted(driven & skipped)
    if both:
        reasons.append(f"these are both driven and skipped: {', '.join(both)}")
    return reasons


def coverage(cli: str) -> dict[str, Any]:
    """Every advertised subcommand is either driven or skipped for a reason."""
    try:
        advertised = advertised_subcommands(cli)
    except (SweepError, subprocess.TimeoutExpired) as error:
        return {"name": "coverage", "passed": False, "colorful": False, "detail": str(error)}
    driven = {probe.subcommand for probe in PROBES}
    reasons = judge_coverage(advertised, driven, set(SKIPPED))
    return {
        "name": "coverage",
        "passed": not reasons,
        "colorful": False,
        "detail": (
            "; ".join(reasons)
            if reasons
            else f"{len(advertised)} subcommands: {len(driven)} driven, {len(SKIPPED)} skipped"
        ),
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    try:
        cli = resolve_cli()
    except SweepError as error:
        results = [{"name": "build", "passed": False, "colorful": False, "detail": str(error)}]
    else:
        results = [coverage(cli)] + [check(cli, probe) for probe in PROBES]
        colorful = [item for item in results if item["colorful"]]
        if len(colorful) < MIN_COLORFUL_PROBES:
            results.append(
                {
                    "name": "the sweep saw real color",
                    "passed": False,
                    "colorful": False,
                    "detail": (
                        f"only {len(colorful)} probes emitted color when it was allowed, "
                        f"need {MIN_COLORFUL_PROBES}; a binary that cannot draw in color "
                        "would pass every check above without honoring anything"
                    ),
                }
            )
        else:
            results.append(
                {
                    "name": "the sweep saw real color",
                    "passed": True,
                    "colorful": True,
                    "detail": f"{len(colorful)} probes emitted color when it was allowed",
                }
            )

    failed = [item for item in results if not item["passed"]]
    summary = {
        "suite": "no-color",
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
    print(
        json.dumps(
            {key: summary[key] for key in ("suite", "passed", "check_count", "failed_count")},
            sort_keys=True,
        )
    )
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
