#!/usr/bin/env python3
"""Run named Rust tests and fail if any of them did not actually run.

`cargo test -- --exact some::test::that::was::renamed` runs nothing and exits
0. A workflow step written that way keeps reporting success after the test it
names is gone, which is worse than not having the step: the board stays green
and the gate is no longer there.

This runs each name and requires the run to report exactly one passing test.
Use it wherever a gate pins a test by name.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# `test result: ok. 1 passed; 0 failed; ...`
RESULT = re.compile(
    r"^test result: (?P<verdict>\w+)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed;",
    re.MULTILINE,
)


def run_one(package: str, target: list[str], name: str, ignored: bool) -> str | None:
    """Return a complaint, or None when exactly one test ran and passed."""
    command = ["cargo", "test", "-p", package, "--release", *target, "--", "--exact"]
    if ignored:
        command.append("--ignored")
    command.append(name)
    finished = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    output = finished.stdout + finished.stderr

    matches = RESULT.findall(output)
    if not matches:
        return f"{name}: cargo printed no test result at all\n{output[-2000:]}"
    passed = sum(int(match[1]) for match in matches)
    failed = sum(int(match[2]) for match in matches)
    if failed:
        return f"{name}: {failed} failed\n{output[-4000:]}"
    if passed != 1:
        return (
            f"{name}: {passed} tests ran, expected exactly 1. A name that matches "
            f"nothing exits 0, so this gate would have reported success while "
            f"checking nothing."
        )
    if finished.returncode != 0:
        return f"{name}: cargo exited {finished.returncode}\n{output[-2000:]}"
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True)
    parser.add_argument("--lib", action="store_true")
    parser.add_argument("--bin")
    parser.add_argument("--ignored", action="store_true")
    parser.add_argument("names", nargs="+")
    args = parser.parse_args()

    target: list[str] = []
    if args.lib:
        target.append("--lib")
    if args.bin:
        target.extend(["--bin", args.bin])
    if not target:
        print("give --lib or --bin so cargo does not sweep every target", file=sys.stderr)
        return 2

    complaints = [
        complaint
        for name in args.names
        if (complaint := run_one(args.package, target, name, args.ignored))
    ]
    for complaint in complaints:
        print(f"FAIL {complaint}", file=sys.stderr)
    if complaints:
        return 1
    print(f"ran {len(args.names)} named test(s) in {args.package}, each exactly once")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
