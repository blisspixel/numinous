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



class MelodyDefaultTests(unittest.TestCase):
    """The three faces disagree about a default melody length, on purpose.

    Recorded so it cannot drift silently while an owner decides. Each face's
    default is read from its own source, so this fails when a face changes and
    the record does not follow, and it fails when a face is added without one.
    """

    CORE = ROOT / "crates" / "core" / "src" / "studio.rs"

    def recorded(self) -> dict[str, int]:
        text = self.CORE.read_text(encoding="utf-8")
        block = re.search(
            r"DEFAULT_MELODY_NOTES_PER_FACE:.*?=\s*\[(.*?)\];", text, re.S
        )
        self.assertIsNotNone(block, "the record is gone from studio.rs")
        return {
            name: int(count)
            for name, count in re.findall(r'\("([a-z-]+)",\s*(\d+)\)', block.group(1))
        }

    def measured(self) -> dict[str, int]:
        app = (ROOT / "faces" / "app" / "src" / "studio_panel.rs").read_text(encoding="utf-8")
        cli = (ROOT / "faces" / "cli" / "src" / "main.rs").read_text(encoding="utf-8")
        mcp = (ROOT / "faces" / "mcp" / "src" / "main.rs").read_text(encoding="utf-8")

        app_notes = re.search(r"to_melody\(expr,\s*-TAU,\s*TAU,\s*(\d+),", app)
        self.assertIsNotNone(app_notes, "the App Studio panel no longer sings a fixed count")

        # The `notes` default sits in the Sing command block, not anywhere else
        # in a file this large.
        sing = re.search(r"Sing \{(.*?)\n    \},", cli, re.S)
        self.assertIsNotNone(sing, "the CLI Sing command block moved")
        cli_notes = re.search(
            r"Number of notes\.\s*\n\s*#\[arg\(long, default_value_t = (\d+)\)\]", sing.group(1)
        )
        self.assertIsNotNone(cli_notes, "the CLI sing note default moved")

        mcp_notes = re.search(
            r'let notes = args\.get\("notes"\)\.and_then\(Value::as_u64\)\.unwrap_or\((\d+)\)',
            mcp,
        )
        self.assertIsNotNone(mcp_notes, "the MCP sing note default moved")
        return {
            "app-studio-panel": int(app_notes.group(1)),
            "cli-sing": int(cli_notes.group(1)),
            "mcp-sing-expression": int(mcp_notes.group(1)),
        }

    def test_the_record_matches_what_each_face_actually_does(self):
        self.assertEqual(self.recorded(), self.measured())

    def test_the_record_is_deleted_once_the_faces_agree(self):
        values = set(self.measured().values())
        self.assertGreater(
            len(values),
            1,
            "every face now sings the same default length, so delete "
            "DEFAULT_MELODY_NOTES_PER_FACE and this test rather than keeping a "
            "record of a disagreement that is over",
        )

    def test_the_disagreement_is_named_where_the_owner_reads(self):
        section = (ROOT / "docs" / "ROADMAP.md").read_text(encoding="utf-8")
        start = section.find("### Decisions the am-track is waiting on")
        self.assertNotEqual(start, -1, "the roadmap has no decisions section")
        end = section.find("\n### ", start + 10)
        decisions = section[start:end]
        for face in self.recorded():
            self.assertIn(
                face,
                decisions,
                f"{face} sings its own default length and is not named in the "
                f"roadmap's decisions section",
            )

if __name__ == "__main__":
    unittest.main()
