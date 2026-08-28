#!/usr/bin/env python3
"""Every workflow action is pinned, and pinned to the same commit everywhere.

Two failures this catches, both of which have happened here:

- An action referenced by a SHA that was written from memory rather than copied
  from a working pin. The job fails at the point of use, which on a nightly
  means nobody sees it until the next morning, and on a workflow that cannot
  currently run means nobody sees it at all.
- The same action pinned to two different commits in two files, which is how a
  supply-chain review ends up having to reason about two versions and how an
  upgrade quietly leaves one behind.

A tag alone (`actions/checkout@v4`) is not a pin: tags move.
"""

from __future__ import annotations

import collections
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github" / "workflows"

# `uses: owner/name@<sha>  # v1.2.3`
PINNED = re.compile(r"uses:\s*(?P<action>[^@\s]+)@(?P<sha>[0-9a-f]{40})\s*#\s*(?P<version>\S+)")
ANY_USES = re.compile(r"^\s*-?\s*uses:\s*(?P<ref>\S+)")

# Actions allowed to appear at more than one commit, and why. The toolchain is
# pinned twice on purpose: the MSRV job exists to prove the crate still builds
# on the oldest supported compiler, so it must not follow the current one.
MULTI_VERSION_ALLOWED = {
    "dtolnay/rust-toolchain": "the MSRV job pins the oldest supported compiler",
}


def action_step(lines: list[str], start: int) -> list[str]:
    """Return one action step, including its indented configuration."""
    indent = len(lines[start]) - len(lines[start].lstrip())
    end = start + 1
    while end < len(lines):
        line = lines[end]
        if line.strip() and len(line) - len(line.lstrip()) <= indent:
            break
        end += 1
    return lines[start:end]


def workflow_files() -> list[Path]:
    return sorted(WORKFLOWS.glob("*.yml"))


class PinTests(unittest.TestCase):
    def test_there_are_workflows_to_check(self):
        # A glob that matched nothing would pass every test below by default.
        self.assertGreaterEqual(len(workflow_files()), 3, "expected ci, nightly and release")

    def test_every_action_is_pinned_to_a_commit_with_a_version_comment(self):
        for path in workflow_files():
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                match = ANY_USES.match(line)
                if not match:
                    continue
                reference = match.group("ref")
                if reference.startswith("./"):
                    continue  # a local composite action, nothing to pin
                self.assertRegex(
                    line,
                    PINNED,
                    f"{path.name}:{number} is not pinned to a 40-character commit with a "
                    f"version comment: {reference}",
                )

    def test_each_action_resolves_to_one_commit_across_every_workflow(self):
        seen: dict[str, set[tuple[str, str]]] = collections.defaultdict(set)
        for path in workflow_files():
            for match in PINNED.finditer(path.read_text(encoding="utf-8")):
                seen[match.group("action")].add((match.group("sha"), match.group("version")))
        self.assertTrue(seen, "no pinned actions found, so this checks nothing")
        for action, pins in sorted(seen.items()):
            if len(pins) == 1:
                continue
            self.assertIn(
                action,
                MULTI_VERSION_ALLOWED,
                f"{action} is pinned to {len(pins)} different commits "
                f"{sorted(version for _, version in pins)}. If that is deliberate, say why "
                f"in MULTI_VERSION_ALLOWED; otherwise make them agree.",
            )

    def test_a_version_comment_never_disagrees_with_itself(self):
        # Same commit, two different version comments, means one of them is a
        # stale label and a reader cannot tell which.
        by_sha: dict[str, set[str]] = collections.defaultdict(set)
        for path in workflow_files():
            for match in PINNED.finditer(path.read_text(encoding="utf-8")):
                by_sha[match.group("sha")].add(match.group("version"))
        for sha, versions in sorted(by_sha.items()):
            self.assertEqual(
                len(versions),
                1,
                f"{sha[:12]} is labelled {sorted(versions)} in different places",
            )

    def test_checkout_never_persists_workflow_credentials(self):
        checked = 0
        for path in workflow_files():
            lines = path.read_text(encoding="utf-8").splitlines()
            for number, line in enumerate(lines, 1):
                if "uses: actions/checkout@" not in line:
                    continue
                checked += 1
                step = action_step(lines, number - 1)
                self.assertEqual(
                    [part.strip() for part in step].count(
                        "persist-credentials: false"
                    ),
                    1,
                    f"{path.name}:{number} must disable checkout credential persistence",
                )
        self.assertGreater(checked, 0, "no checkout actions found, so this checks nothing")


class RoundtripJobTests(unittest.TestCase):
    """0.6-am asks for the roundtrip on three operating systems."""

    def setUp(self) -> None:
        self.nightly = (WORKFLOWS / "nightly.yml").read_text(encoding="utf-8")

    def test_the_roundtrip_runs_on_all_three_operating_systems(self):
        for os_name in ("ubuntu-latest", "macos-latest", "windows-latest"):
            self.assertIn(
                f"os: {os_name}",
                self.nightly,
                f"the nightly roundtrip matrix does not include {os_name}, so 0.6-am's "
                f"three-platform claim would rest on fewer than three",
            )

    def test_one_platform_failing_does_not_hide_the_others(self):
        # Per-platform evidence is the point. With fail-fast, a Linux failure
        # would cancel Windows and macOS before they reported anything.
        self.assertIn("fail-fast: false", self.nightly)

    def test_each_platform_packages_its_own_archive_format(self):
        for target, archive in (
            ("x86_64-unknown-linux-gnu", "tar.gz"),
            ("aarch64-apple-darwin", "tar.gz"),
            ("x86_64-pc-windows-msvc", "zip"),
        ):
            self.assertIn(f"target: {target}", self.nightly)
            self.assertIn(f"archive: {archive}", self.nightly)

    def test_the_roundtrip_gate_judgment_runs_before_the_roundtrip(self):
        # The twin tests the gate's judgment. Running it after the gate would
        # mean a broken gate reports on the product before anything checks the
        # gate itself.
        twin = self.nightly.find("scripts/test-uninstall-roundtrip.py")
        real = self.nightly.find("scripts/uninstall-roundtrip.py \\")
        self.assertNotEqual(twin, -1, "the roundtrip twin is not wired into the nightly")
        self.assertNotEqual(real, -1, "the roundtrip itself is not wired into the nightly")
        self.assertLess(twin, real)

if __name__ == "__main__":
    unittest.main()
