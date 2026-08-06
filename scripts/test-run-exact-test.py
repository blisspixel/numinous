#!/usr/bin/env python3
"""Test twin for run-exact-test.py: does it judge a run correctly?

The helper exists for one reason, that `cargo test -- --exact a::name::that::is::gone`
runs nothing and exits 0. A helper that got that wrong would be worse than no
helper, because a workflow would report success while checking nothing. So the
cases that matter here are the ones where cargo itself succeeds.

Cargo is not invoked. `run_one` is called with a stub that returns prepared
output, which is what makes these tests fast and lets them cover results a real
run would be tedious to provoke.
"""

from __future__ import annotations

import importlib.util
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "run_exact_test", ROOT / "scripts" / "run-exact-test.py"
)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class Finished:
    """Enough of `subprocess.CompletedProcess` for the helper to read."""

    def __init__(self, stdout: str, returncode: int = 0) -> None:
        self.stdout = stdout
        self.stderr = ""
        self.returncode = returncode


def with_output(stdout: str, returncode: int = 0):
    """Run the helper against prepared cargo output."""
    real = subprocess.run
    subprocess.run = lambda *args, **kwargs: Finished(stdout, returncode)  # type: ignore[assignment]
    try:
        return GATE.run_one("numinous-core", ["--lib"], "some::test", ignored=False)
    finally:
        subprocess.run = real  # type: ignore[assignment]


ONE_PASSED = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 90 filtered out\n"
NONE_RAN = "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out\n"
ONE_FAILED = "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 90 filtered out\n"
TWO_PASSED = "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 89 filtered out\n"


class JudgementTests(unittest.TestCase):
    def test_one_passing_test_is_accepted(self):
        self.assertIsNone(with_output(ONE_PASSED))

    def test_a_name_that_matches_nothing_is_rejected_even_though_cargo_succeeded(self):
        # The whole reason this helper exists. Cargo exits 0 here.
        complaint = with_output(NONE_RAN, returncode=0)
        self.assertIsNotNone(complaint)
        self.assertIn("0 tests ran", complaint)

    def test_a_failing_test_is_rejected(self):
        complaint = with_output(ONE_FAILED, returncode=101)
        self.assertIsNotNone(complaint)
        self.assertIn("failed", complaint)

    def test_a_name_matching_more_than_one_test_is_rejected(self):
        # --exact should match one. Two means the name is not what was meant,
        # and a gate that pins a test should know exactly which test it pinned.
        complaint = with_output(TWO_PASSED)
        self.assertIsNotNone(complaint)
        self.assertIn("2 tests ran", complaint)

    def test_output_without_any_result_line_is_rejected(self):
        # A build failure prints no test result. Reading that as success would
        # be the same hole in a different shape.
        complaint = with_output("error: could not compile numinous-core\n", returncode=101)
        self.assertIsNotNone(complaint)
        self.assertIn("no test result", complaint)

    def test_a_nonzero_exit_with_a_passing_line_is_still_rejected(self):
        # Belt and braces: if cargo says it failed, believe it, even when the
        # summary it printed looks fine.
        complaint = with_output(ONE_PASSED, returncode=101)
        self.assertIsNotNone(complaint)
        self.assertIn("exited 101", complaint)


class WiringTests(unittest.TestCase):
    def test_the_nightly_only_pins_tests_through_this_helper(self):
        # A step that calls `cargo test -- --exact` directly carries the hole
        # this helper closes, so the workflow must not do that.
        nightly = (ROOT / ".github" / "workflows" / "nightly.yml").read_text(encoding="utf-8")
        for line in nightly.splitlines():
            stripped = line.strip()
            if stripped.startswith("#"):
                continue
            if "--exact" in stripped and "cargo test" in stripped:
                self.fail(f"nightly pins a test without the helper: {stripped}")

    def test_the_helper_refuses_to_sweep_every_target(self):
        # Without --lib or --bin, cargo would run every target and the count
        # check would mean nothing.
        finished = subprocess.run(
            ["python", str(ROOT / "scripts" / "run-exact-test.py"), "--package", "x", "a::b"],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("--lib or --bin", finished.stdout + finished.stderr)


if __name__ == "__main__":
    unittest.main()
