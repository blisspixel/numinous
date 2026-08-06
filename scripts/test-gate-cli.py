#!/usr/bin/env python3
"""Regressions for the one resolver, and a rule that keeps it the only one.

`gate_cli.py` exists because eight gates each had their own way of finding the
binaries they test, and three of those quietly tested whichever artifact was
lying in `target/` rather than the code under review. Fixing them one at a time
did not work: the first sweep found six, and two more turned up in the next
cycle, in the gate that guards the flagship regression evidence.

So the sweep is a test now. A written note protects nothing; the checks that
have never been broken here are the ones something else enforces.
"""

from __future__ import annotations

import importlib.util
import os
import re
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = ROOT / "scripts"
SPEC = importlib.util.spec_from_file_location("gate_cli", SCRIPTS / "gate_cli.py")
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

# A path assembled out of this repository's cargo layout: the shape every
# private resolver had. Written as a pattern rather than a literal so
# reordering the parts or swapping quote styles does not slip past it.
#
# Anchored on ROOT, the repository root, on purpose. A gate that builds the
# same path under a temporary directory is making a fixture rather than
# resolving a binary, and one test does exactly that to prove a stale artifact
# gets rejected. Flagging it would be flagging the check for this very defect.
PRIVATE_PATH = re.compile(r"""ROOT\s*/\s*["']target["']\s*/\s*["'](?:debug|release)["']""")

# The fallback that let a gate compile mid-run, spending its per-command
# timeout on a build or on waiting for the cargo lock.
CARGO_RUN = re.compile(r"""["']cargo["']\s*,\s*["']run["']""")


def gate_sources() -> list[Path]:
    """Every script except the one resolver and its own test."""
    skip = {"gate_cli.py", "test-gate-cli.py"}
    return sorted(p for p in SCRIPTS.glob("*.py") if p.name not in skip)


class OnlyOneResolverTests(unittest.TestCase):
    def test_no_gate_builds_its_own_cargo_path(self) -> None:
        # The defect this exists to prevent, in the exact form it kept taking.
        offenders = [
            path.name for path in gate_sources() if PRIVATE_PATH.search(path.read_text("utf-8"))
        ]
        self.assertEqual(
            offenders,
            [],
            "these build their own path into the cargo layout instead of asking "
            f"gate_cli: {offenders}. That is how a stale binary ends up answering "
            "for code that no longer exists.",
        )

    def test_no_gate_keeps_the_cargo_run_fallback(self) -> None:
        offenders = [
            path.name for path in gate_sources() if CARGO_RUN.search(path.read_text("utf-8"))
        ]
        self.assertEqual(
            offenders,
            [],
            f"these can compile mid-run instead of building up front: {offenders}",
        )

    def test_the_rule_would_catch_the_shapes_it_is_written_for(self) -> None:
        # Without this the two tests above could be passing because the patterns
        # match nothing at all, which is the failure they exist to prevent.
        for sample in (
            'ROOT / "target" / "debug" / "numinous"',
            "ROOT / 'target' / 'release' / 'numinous.exe'",
            'ROOT / "target"  /  "debug" / "numinous-mcp"',
            'binary = ROOT / "target" / "debug" / "numinous-mcp"',
        ):
            with self.subTest(sample=sample):
                self.assertIsNotNone(PRIVATE_PATH.search(sample))
        for sample in ('["cargo", "run", "--quiet"]', "['cargo','run']"):
            with self.subTest(sample=sample):
                self.assertIsNotNone(CARGO_RUN.search(sample))
        # The import rule reads a real statement and not a mention of one.
        importing = re.compile(r"(?m)^from gate_cli import\s+\w")
        self.assertIsNotNone(importing.search("from gate_cli import resolve_cli\n"))
        for mention in (
            "# see gate_cli.py, and from gate_cli import resolve_cli\n",
            '"""a gate should use: from gate_cli import resolve_cli"""\n',
            "    from gate_cli import resolve_cli  # indented, inside a branch\n",
        ):
            with self.subTest(mention=mention.strip()):
                self.assertIsNone(importing.search(mention))
        # And that it does not fire on the things that legitimately say target,
        # such as a release triple or a packaging argument.
        for innocent in (
            '{"target": target, "version": version}',
            'parser.add_argument("--target")',
            '"--binary-dir", "target/release"',
            # A fixture built under a temporary root, not a resolver.
            'stale = root / "target" / "debug" / "numinous-mcp"',
        ):
            with self.subTest(innocent=innocent):
                self.assertIsNone(PRIVATE_PATH.search(innocent))

    def test_every_gate_that_drives_a_binary_asks_gate_cli(self) -> None:
        # The positive half. A gate could avoid both patterns above by finding
        # its binary some third way, so the ones that drive the product are
        # required to go through the shared resolver by name.
        for name in (
            "am-soak.py",
            "catalog-scorecard.py",
            "creator-roundtrip.py",
            "creator-parity.py",
            "no-color.py",
            "reduced-motion.py",
            "flagship-goldens.py",
            "agent-hallway.py",
        ):
            with self.subTest(gate=name):
                source = (SCRIPTS / name).read_text("utf-8")
                # Anchored to the start of a line, so the words appearing in a
                # comment or a string do not satisfy it. A gate that only talks
                # about the shared resolver is not using it.
                self.assertRegex(source, r"(?m)^from gate_cli import\s+\w")


class ResolverTests(unittest.TestCase):
    def test_the_target_directory_follows_cargo_target_dir(self) -> None:
        # A build that succeeded can look missing under ROOT/target when this is
        # set, and several CI layouts set it.
        original = os.environ.get("CARGO_TARGET_DIR")
        try:
            with tempfile.TemporaryDirectory(prefix="gate-cli-", ignore_cleanup_errors=True) as tmp:
                os.environ["CARGO_TARGET_DIR"] = tmp
                self.assertEqual(MODULE.target_debug(), Path(tmp) / "debug")
            os.environ.pop("CARGO_TARGET_DIR", None)
            self.assertEqual(MODULE.target_debug(), ROOT / "target" / "debug")
            # A relative value is resolved by cargo against its own working
            # directory, which is always the repository root here, so resolving
            # it against the caller's directory would look in the wrong place.
            os.environ["CARGO_TARGET_DIR"] = "build-elsewhere"
            self.assertEqual(MODULE.target_debug(), ROOT / "build-elsewhere" / "debug")
        finally:
            if original is None:
                os.environ.pop("CARGO_TARGET_DIR", None)
            else:
                os.environ["CARGO_TARGET_DIR"] = original

    def test_asking_for_nothing_is_an_error_rather_than_a_pass(self) -> None:
        with self.assertRaises(MODULE.GateError):
            MODULE.build_and_locate(())

    def test_a_missing_binary_is_reported_by_name(self) -> None:
        original = os.environ.get("CARGO_TARGET_DIR")
        try:
            with tempfile.TemporaryDirectory(prefix="gate-cli-", ignore_cleanup_errors=True) as tmp:
                os.environ["CARGO_TARGET_DIR"] = tmp
                # Building a binary that does not exist fails at the build, and
                # either way the complaint has to name what was being looked for.
                with self.assertRaises(MODULE.GateError) as raised:
                    MODULE.build_and_locate(("numinous-not-a-real-binary",))
                self.assertIn("numinous-not-a-real-binary", str(raised.exception))
        finally:
            if original is None:
                os.environ.pop("CARGO_TARGET_DIR", None)
            else:
                os.environ["CARGO_TARGET_DIR"] = original


if __name__ == "__main__":
    unittest.main()
