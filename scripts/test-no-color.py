#!/usr/bin/env python3
"""Focused regressions for the NO_COLOR sweep.

These cover the judgment without spawning the CLI, so a mistake in what counts
as covered is caught even when nothing is built. The live sweep is
`no-color.py`, which CI runs against a real binary.

Coverage is the part worth testing hardest. Whether one surface emits an escape
is measured directly and fails loudly. Whether the sweep still covers the binary
is a claim about a list, and a list can quietly stop describing the thing it was
written for while every check it does run stays green.
"""

from __future__ import annotations

import importlib.util
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("no_color", ROOT / "scripts" / "no-color.py")
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CoverageJudgementTests(unittest.TestCase):
    def test_a_covered_binary_is_judged_to_pass(self) -> None:
        # Without this, every assertion below could be met by a judgment that
        # objects to everything.
        self.assertEqual(
            MODULE.judge_coverage({"rooms", "update"}, {"rooms"}, {"update"}), []
        )

    def test_a_new_subcommand_is_not_quietly_uncovered(self) -> None:
        # The whole point. A subcommand added next year is checked or is
        # explicitly excused, and either way somebody decided.
        reasons = MODULE.judge_coverage({"rooms", "brand-new"}, {"rooms"}, set())
        self.assertTrue(reasons)
        self.assertIn("brand-new", reasons[0])

    def test_a_skip_for_something_gone_is_reported(self) -> None:
        # Stale bookkeeping hides that the list was never revisited, and the
        # next person reads it as a considered decision.
        reasons = MODULE.judge_coverage({"rooms"}, {"rooms"}, {"removed-long-ago"})
        self.assertTrue(any("no longer exist" in reason for reason in reasons))

    def test_driving_and_skipping_the_same_subcommand_is_a_contradiction(self) -> None:
        reasons = MODULE.judge_coverage({"rooms"}, {"rooms"}, {"rooms"})
        self.assertTrue(any("both driven and skipped" in reason for reason in reasons))

    def test_several_uncovered_subcommands_are_all_named(self) -> None:
        # A report that names one of four sends the reader back three times.
        reasons = MODULE.judge_coverage({"a", "b", "c", "d"}, {"a"}, {"b"})
        self.assertTrue(reasons)
        self.assertIn("c, d", reasons[0])


class ProbeTableTests(unittest.TestCase):
    def test_every_skip_carries_a_reason(self) -> None:
        for name, reason in MODULE.SKIPPED.items():
            self.assertTrue(reason.strip(), f"{name} is skipped with no reason given")

    def test_no_probe_is_listed_twice(self) -> None:
        labels = [probe.label for probe in MODULE.PROBES]
        self.assertEqual(len(labels), len(set(labels)))

    def test_every_probe_names_a_subcommand_first(self) -> None:
        for probe in MODULE.PROBES:
            self.assertTrue(probe.argv, "a probe runs the CLI with no arguments")
            self.assertFalse(
                probe.subcommand.startswith("-"),
                f"{probe.label} leads with a flag rather than a subcommand",
            )

    def test_the_live_loops_carry_a_deadline(self) -> None:
        # watch, play and tour never exit. A probe without a deadline waits the
        # full one-shot timeout and then reports a hang, which would turn a
        # working feature into a slow failure.
        for name in ("watch", "play", "tour"):
            probes = [probe for probe in MODULE.PROBES if probe.subcommand == name]
            self.assertTrue(probes, f"{name} is not driven at all")
            for probe in probes:
                self.assertIsNotNone(probe.deadline, f"{probe.label} would hang the sweep")

    def test_one_shot_probes_do_not_carry_a_deadline(self) -> None:
        # A deadline on something that exits by itself would cut its output
        # short and could hide an escape printed at the end.
        loops = {"watch", "play", "tour"}
        for probe in MODULE.PROBES:
            if probe.subcommand not in loops:
                self.assertIsNone(probe.deadline, f"{probe.label} is cut short for no reason")

    def test_the_games_that_carried_raw_escapes_are_all_driven(self) -> None:
        # These three wrote SGR inline and ignored the setting. They are the
        # reason this sweep exists, so losing one from the table would be
        # losing the regression.
        driven = {probe.subcommand for probe in MODULE.PROBES}
        for game in ("arcade", "hackenbush", "party"):
            self.assertIn(game, driven)

    def test_at_least_one_probe_asks_for_color_explicitly(self) -> None:
        # Plain `render` is ASCII either way, so a sweep holding only that would
        # never see the renderer's color at all.
        self.assertTrue(
            any("--color" in probe.argv for probe in MODULE.PROBES),
            "nothing drives the color renderer",
        )


class EscapeMatchingTests(unittest.TestCase):
    def test_sgr_is_matched_and_cursor_control_is_not(self) -> None:
        # The instrument the whole sweep measures with. One that matched cursor
        # moves would fail every surface; one that matched nothing would pass a
        # binary painting in full color under NO_COLOR.
        self.assertEqual(MODULE.SGR.findall("\x1b[91mR\x1b[0m"), ["\x1b[91m", "\x1b[0m"])
        self.assertEqual(MODULE.SGR.findall("\x1b[38;2;1;2;3mx"), ["\x1b[38;2;1;2;3m"])
        self.assertEqual(MODULE.SGR.findall("\x1b[1;91mx"), ["\x1b[1;91m"])
        for allowed in ("\x1b[H", "\x1b[2J", "\x1b[K", "\x1b[J", "plain text"):
            self.assertEqual(MODULE.SGR.findall(allowed), [], f"{allowed!r} is not color")

    def test_the_pattern_is_the_one_the_rust_tests_use(self) -> None:
        # Same rule as `sgr_codes` in the CLI's own tests: escape, bracket,
        # digits and semicolons, terminated by m. Written out here so the two
        # cannot drift into disagreeing about what color is.
        self.assertEqual(MODULE.SGR.pattern, r"\x1b\[[0-9;]*m")
        self.assertIsInstance(MODULE.SGR, re.Pattern)


class ThresholdTests(unittest.TestCase):
    def test_the_sweep_demands_evidence_that_it_saw_color(self) -> None:
        # Without a floor here, a binary that had lost the ability to draw in
        # color at all would pass every check in the sweep.
        self.assertGreaterEqual(MODULE.MIN_COLORFUL_PROBES, 3)

    def test_loop_probes_end_well_inside_the_one_shot_timeout(self) -> None:
        self.assertLess(MODULE.LOOP_DEADLINE_SECONDS, MODULE.ONE_SHOT_TIMEOUT_SECONDS)


if __name__ == "__main__":
    unittest.main()
