#!/usr/bin/env python3
"""Lint every Python gate, and hold a growing set of them to strict typing.

The gates in this directory decide whether a release ships. They had no linter
and no type checker, which is a strange place for the project's least-checked
code to be.

Linting is applied to all of them at once, because the whole directory is clean
and keeping it clean costs nothing.

Typing is a ratchet rather than a sweep. Thirty-six of the fifty-four scripts do
not pass `mypy --strict` today, and retrofitting them in one pass would mean
hundreds of hurried annotations and ignore comments, which buys the appearance
of rigor and none of it. So the strict set below starts with every file that
already passes and can only grow:

  - a declared file that stops passing fails this gate, which is the guarantee;
  - an undeclared file that already passes also fails it, which is what stops
    the set going stale while the directory improves around it.

That second rule is the ratchet. Without it the list would freeze on the day it
was written, and a newly clean file would sit unprotected forever. With it, the
only way to make this gate pass is to add the file, so the checked set follows
the code rather than trailing it.

Both rules are decided by running the checker, not by judgment, so there is
nothing here for a reader to rubber-stamp. That is deliberate: a gate whose
entries are opinions is a gate nobody reads.

Run it from a clone:

    python scripts/python-quality.py

Add `--fix` to apply the lint fixes that ruff can make itself.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final, NamedTuple

ROOT: Final[Path] = Path(__file__).resolve().parent.parent
SCRIPTS: Final[Path] = ROOT / "scripts"
OUT: Final[Path] = ROOT / ".agent" / "tester-cohort" / "python-quality"

CHECK_TIMEOUT_SECONDS: Final[float] = 600.0

# Files held to `mypy --strict`. This set only grows; see the module docstring.
# Keep it sorted, so a diff shows what joined rather than where it landed.
STRICT: Final[frozenset[str]] = frozenset(
    {
        "agent-first-contact.py",
        "agent-hallway.py",
        "agent-tactile.py",
        "gate_cli.py",
        "python-quality.py",
        "release-engagement-smoke.py",
        "run-exact-test.py",
        "sensory-platform-set.py",
        "test-agent-plugin.py",
        "test-am-soak.py",
        "test-game-truth.py",
        "test-gate-cli.py",
        "test-no-color.py",
        "test-python-quality.py",
        "test-release-engagement-smoke.py",
        "test-uninstall-roundtrip.py",
        "understanding-am-auditor.py",
        "understanding-am-pipeline.py",
        "understanding-source.py",
        "validate-agent-plugin.py",
    }
)


class QualityError(RuntimeError):
    """A checker could not be run, so this gate has no verdict to give."""


class Result(NamedTuple):
    """One judgment, with enough detail to act on without rerunning anything."""

    name: str
    passed: bool
    detail: str

    def as_json(self) -> dict[str, Any]:
        return {"name": self.name, "passed": self.passed, "detail": self.detail}


def python_files() -> list[str]:
    """Every gate in this directory, by name, in a stable order."""
    return sorted(path.name for path in SCRIPTS.glob("*.py"))


def check(tool: list[str], targets: list[str]) -> tuple[bool, str]:
    """Run one checker over the given files and report the verdict plainly."""
    if not targets:
        return True, "nothing to check"
    command = [sys.executable, "-m", *tool, *[f"scripts/{name}" for name in targets]]
    try:
        completed = subprocess.run(
            command,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            timeout=CHECK_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise QualityError(f"could not run {tool[0]}: {error}") from error
    output = (completed.stdout + completed.stderr).strip()
    return completed.returncode == 0, output


def strict_typing(files: list[str]) -> list[Result]:
    """The declared set must pass, and anything already passing must be declared."""
    results: list[Result] = []

    declared = sorted(name for name in files if name in STRICT)
    missing = sorted(STRICT - set(files))
    if missing:
        results.append(
            Result(
                "every declared file still exists",
                False,
                "declared but absent, so the list has gone stale: " + ", ".join(missing),
            )
        )

    passed, output = check(["mypy", "--strict"], declared)
    results.append(
        Result(
            "the declared set passes strict typing",
            passed,
            f"{len(declared)} files checked"
            if passed
            else output[-1200:] or "mypy reported a failure with no output",
        )
    )

    # The ratchet. Anything that already passes must be declared, so the checked
    # set follows the directory instead of freezing on the day it was written.
    undeclared = [name for name in files if name not in STRICT]
    newly_clean: list[str] = []
    for name in undeclared:
        clean, _ = check(["mypy", "--strict"], [name])
        if clean:
            newly_clean.append(name)
    results.append(
        Result(
            "nothing already strict-clean is left undeclared",
            not newly_clean,
            f"{len(undeclared)} files are not yet strict, which is expected"
            if not newly_clean
            else (
                "these now pass strict typing and must be added to STRICT so they "
                "cannot regress: " + ", ".join(newly_clean)
            ),
        )
    )
    return results


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fix", action="store_true", help="apply the lint fixes ruff can make itself"
    )
    args = parser.parse_args(argv)

    OUT.mkdir(parents=True, exist_ok=True)
    files = python_files()
    results: list[Result]
    try:
        if not files:
            raise QualityError("no Python gates found; this check would cover nothing")
        lint_command = ["ruff", "check", "--fix"] if args.fix else ["ruff", "check"]
        lint_passed, lint_output = check(lint_command, files)
        results = [
            Result(
                "every gate passes the linter",
                lint_passed,
                f"{len(files)} files checked"
                if lint_passed
                else lint_output[-1200:] or "ruff reported a failure with no output",
            )
        ]
        results.extend(strict_typing(files))
    except QualityError as error:
        results = [Result("python quality", False, str(error))]

    failed = [result for result in results if not result.passed]
    summary: dict[str, Any] = {
        "suite": "python-quality",
        "passed": not failed,
        "file_count": len(files),
        "strict_count": len(STRICT),
        "check_count": len(results),
        "failed_count": len(failed),
        "results": [result.as_json() for result in results],
        "evidence_class": "agent-machine",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{len(files)} gates linted, {len(STRICT)} held to strict typing")
    for result in results:
        print(f"  {'PASS' if result.passed else 'FAIL'}  {result.name}: {result.detail}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                key: summary[key]
                for key in ("suite", "passed", "file_count", "strict_count", "failed_count")
            },
            sort_keys=True,
        )
    )
    return 0 if not failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
