#!/usr/bin/env python3
"""Focused regressions for the Python quality ratchet.

The live gate runs two checkers over fifty-odd files and takes about a minute.
These test the judgment around them without running either, so a mistake in what
the ratchet means is caught instantly and on any machine.

The ratchet is the part worth testing hardest. Whether one file passes strict
typing is decided by mypy and fails loudly. Whether the checked set is still
following the directory is a claim about a list, and a list can quietly stop
describing the thing it was written for while every check it does run stays
green. That is exactly how the counted-noun class survived two releases, and the
lesson is worth spending a test on.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "python_quality", ROOT / "scripts" / "python-quality.py"
)
assert SPEC is not None and SPEC.loader is not None
# A dynamically loaded module has no statically known attributes.
MODULE: Any = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class DeclaredSetTests(unittest.TestCase):
    def test_every_declared_file_exists(self) -> None:
        # A declared file that was renamed or deleted means the list has stopped
        # describing the directory, and mypy would silently check less than the
        # gate claims.
        present = {path.name for path in (ROOT / "scripts").glob("*.py")}
        missing = sorted(MODULE.STRICT - present)
        self.assertEqual(missing, [], f"declared but absent: {missing}")

    def test_the_gate_holds_itself_to_the_bar_it_sets(self) -> None:
        # A quality gate exempt from its own standard is an argument for
        # exempting the next thing too.
        self.assertIn("python-quality.py", MODULE.STRICT)

    def test_the_declared_set_is_not_empty(self) -> None:
        # An empty set would make every check below vacuously true, which is the
        # one way this gate could pass while protecting nothing.
        self.assertGreater(len(MODULE.STRICT), 0)


class RatchetTests(unittest.TestCase):
    def test_a_newly_clean_file_must_be_declared(self) -> None:
        # The ratchet itself. Without this rule the list freezes on the day it
        # was written and newly clean files sit unprotected forever.
        source = (ROOT / "scripts" / "python-quality.py").read_text(encoding="utf-8")
        self.assertIn("nothing already strict-clean is left undeclared", source)

    def test_the_ratchet_reports_which_files_to_add(self) -> None:
        # A failure that does not name the files sends the reader to run the
        # checker by hand, which is the moment a gate starts being skipped.
        source = (ROOT / "scripts" / "python-quality.py").read_text(encoding="utf-8")
        self.assertIn("must be added to STRICT", source)

    def test_the_untyped_remainder_is_reported_rather_than_hidden(self) -> None:
        # The gate should say plainly how much is still unchecked. A gate that
        # reports only its successes reads as finished when it is not.
        source = (ROOT / "scripts" / "python-quality.py").read_text(encoding="utf-8")
        self.assertIn("not yet strict, which is expected", source)


class LintTests(unittest.TestCase):
    def test_linting_covers_every_file_not_a_declared_subset(self) -> None:
        # Lint has no ratchet because the whole directory already passes.
        # Narrowing it to the strict set later would quietly stop checking two
        # thirds of the gates.
        source = (ROOT / "scripts" / "python-quality.py").read_text(encoding="utf-8")
        self.assertIn("lint_passed, lint_output = check(lint_command, files)", source)


class ResultTests(unittest.TestCase):
    def test_a_result_renders_as_the_shape_the_summary_stores(self) -> None:
        rendered = MODULE.Result("a name", False, "a detail").as_json()
        self.assertEqual(rendered, {"name": "a name", "passed": False, "detail": "a detail"})

    def test_no_targets_is_a_pass_rather_than_a_silent_error(self) -> None:
        # Running a checker with no files is not a failure, but it must not be
        # reported as though something was verified either.
        passed, detail = MODULE.check(["ruff", "check"], [])
        self.assertTrue(passed)
        self.assertEqual(detail, "nothing to check")


if __name__ == "__main__":
    unittest.main(verbosity=2 if "-v" in sys.argv else 1)
