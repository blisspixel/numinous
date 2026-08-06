#!/usr/bin/env python3
"""No `#[ignore]`d test is an orphan.

An ignored test does not run in the ordinary suite. That is the right choice
for a sweep too slow for every pull request, and it is only safe if something
else names and runs it. An ignored test that nothing names never runs at all:
it reads like a gate, it counts as code, and it checks nothing. Worse, it rots
quietly, so the day somebody does run it they find a failure of unknown age.

So every ignored test must be named by a workflow or by a gate script, and this
says which ones are not.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST_ROOTS = (ROOT / "crates", ROOT / "faces")
WORKFLOWS = ROOT / ".github" / "workflows"
SCRIPTS = ROOT / "scripts"

# `#[ignore ...]` then attributes, then `fn name(`
IGNORED = re.compile(r"#\[ignore[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(?P<name>\w+)")

# An ignored test that is deliberately not wired anywhere, with the reason. It
# still has to be listed here, so the choice is visible rather than an
# oversight that looks identical to one.
DELIBERATELY_UNWIRED: dict[str, str] = {}


def ignored_tests() -> list[tuple[Path, str]]:
    found: list[tuple[Path, str]] = []
    for root in RUST_ROOTS:
        for path in sorted(root.rglob("*.rs")):
            if "target" in path.parts:
                continue
            for match in IGNORED.finditer(path.read_text(encoding="utf-8")):
                found.append((path, match.group("name")))
    return found


def runner_text() -> str:
    parts = [path.read_text(encoding="utf-8") for path in sorted(WORKFLOWS.glob("*.yml"))]
    parts += [
        path.read_text(encoding="utf-8")
        for path in sorted(SCRIPTS.glob("*.py"))
        # Skip this file, or it would count its own prose as wiring.
        if path.name != Path(__file__).name
    ]
    return "\n".join(parts)


class OrphanTests(unittest.TestCase):
    def test_the_scan_finds_ignored_tests_at_all(self):
        # A regex that matched nothing would pass every check below by default.
        self.assertGreaterEqual(
            len(ignored_tests()),
            2,
            "no ignored tests found, so this gate is checking nothing",
        )

    def test_every_ignored_test_is_named_by_something_that_runs_it(self):
        runners = runner_text()
        orphans = [
            f"{path.relative_to(ROOT).as_posix()}::{name}"
            for path, name in ignored_tests()
            if name not in runners and name not in DELIBERATELY_UNWIRED
        ]
        self.assertEqual(
            orphans,
            [],
            "these tests are ignored and named by no workflow or gate, so they never "
            "run anywhere. Wire them, delete them, or record why in "
            "DELIBERATELY_UNWIRED: " + ", ".join(orphans),
        )

    def test_an_exemption_always_carries_a_reason(self):
        for name, reason in DELIBERATELY_UNWIRED.items():
            self.assertTrue(
                reason.strip(),
                f"{name} is exempt with no reason, which is the same as an oversight",
            )

    def test_an_exemption_names_a_test_that_exists(self):
        # A stale exemption would silently cover a future test that happened to
        # take the same name.
        names = {name for _, name in ignored_tests()}
        for name in DELIBERATELY_UNWIRED:
            self.assertIn(
                name,
                names,
                f"{name} is exempt but no ignored test has that name any more",
            )


if __name__ == "__main__":
    unittest.main()
