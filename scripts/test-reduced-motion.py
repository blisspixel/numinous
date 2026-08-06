#!/usr/bin/env python3
"""Focused regressions for the reduced-motion contract.

These cover the judgement without spawning the CLI, so a mistake in what counts
as reduced motion working is caught even when nothing is built. The live probes
are `reduced-motion.py`, which CI runs against a real binary.

The judgement is the part worth testing on its own. A gate can be wrong in two
directions and only one of them is loud: a gate that fails a working feature is
noticed within the hour, and a gate that passes a broken one is noticed when a
player writes in. Every case below is a way for The Show to be broken, and each
asserts the judgement says so.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "reduced_motion", ROOT / "scripts" / "reduced-motion.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

DREW = "a room, and the prompt beneath it"


def run(ended: bool, rooms: int, output: str = DREW) -> "MODULE.ShowRun":
    return MODULE.ShowRun(ended=ended, rooms=rooms, output=output)


def healthy() -> tuple["MODULE.ShowRun", "MODULE.ShowRun", "MODULE.ShowRun"]:
    """The three runs a working Show produces."""
    return (
        # Ordinary: never asks, never ends.
        run(ended=False, rooms=0),
        # Held with nobody there: one room, then it stops.
        run(ended=True, rooms=1),
        # Held with a player: one room per answer.
        run(ended=True, rooms=MODULE.ASKED_TWICE_ROOMS),
    )


class ShowJudgementTests(unittest.TestCase):
    def test_a_working_show_is_judged_to_pass(self) -> None:
        # Without this, every assertion below could be satisfied by a judgement
        # that fails everything.
        self.assertEqual(MODULE.judge_show(*healthy()), [])

    def test_a_gallery_that_advances_while_held_fails(self) -> None:
        # The defect the whole feature exists to prevent: the phase is held so
        # the picture stops moving, and the gallery changes rooms anyway.
        ordinary, eof, asked = healthy()
        marched_on = run(ended=False, rooms=0)
        reasons = MODULE.judge_show(ordinary, marched_on, asked)
        self.assertTrue(reasons)
        self.assertTrue(any("redraw forever" in reason for reason in reasons))

    def test_a_closed_stdin_that_never_ends_fails(self) -> None:
        # Held, The Show blocks on the player. If end of input does not leave,
        # a piped or closed stdin turns waiting into a loop that draws forever.
        ordinary, _, asked = healthy()
        spinning = run(ended=False, rooms=975)
        reasons = MODULE.judge_show(ordinary, spinning, asked)
        self.assertTrue(any("redraw forever" in reason for reason in reasons))
        self.assertTrue(any("975 rooms" in reason for reason in reasons))

    def test_ignoring_the_player_fails_in_either_direction(self) -> None:
        ordinary, eof, _ = healthy()
        for rooms in (1, 2, 4, 20):
            reasons = MODULE.judge_show(ordinary, eof, run(ended=True, rooms=rooms))
            self.assertTrue(
                any(str(MODULE.ASKED_TWICE_ROOMS) in reason for reason in reasons),
                f"{rooms} rooms for two Enters and a q should not pass",
            )

    def test_a_show_that_never_advances_at_all_fails(self) -> None:
        # Considerate and broken look the same from a distance. A gallery stuck
        # on one room no matter what the player does is not reduced motion.
        ordinary, eof, _ = healthy()
        stuck = run(ended=True, rooms=1)
        self.assertTrue(MODULE.judge_show(ordinary, eof, stuck))

    def test_ordinary_motion_must_not_wait_for_anyone(self) -> None:
        _, eof, asked = healthy()
        prompting = run(ended=False, rooms=2)
        reasons = MODULE.judge_show(prompting, eof, asked)
        self.assertTrue(any("ordinary motion asked" in reason for reason in reasons))

    def test_ordinary_motion_that_ends_by_itself_fails(self) -> None:
        # It runs until stopped. One that returns on its own was not running
        # the gallery, so the comparison against the held run proves nothing.
        _, eof, asked = healthy()
        reasons = MODULE.judge_show(run(ended=True, rooms=0), eof, asked)
        self.assertTrue(any("ended on its own" in reason for reason in reasons))

    def test_drawing_nothing_is_not_holding_still(self) -> None:
        # The counterpart of the frame probes' blank-frame check: a Show that
        # emits only its prompt has stopped dead rather than held a picture.
        ordinary, _, asked = healthy()
        blank = run(ended=True, rooms=1, output="   \n  \n")
        reasons = MODULE.judge_show(ordinary, blank, asked)
        self.assertTrue(any("drew nothing" in reason for reason in reasons))


class ProbeTableTests(unittest.TestCase):
    def test_the_keys_sent_match_the_rooms_expected(self) -> None:
        # Two Enters and a q is three rooms: the two answered plus the one the
        # player was looking at when they quit. If these ever drift apart the
        # gate measures nothing, and it would still be green.
        self.assertEqual(MODULE.ASKED_TWICE.count(b"\n"), MODULE.ASKED_TWICE_ROOMS)
        self.assertTrue(MODULE.ASKED_TWICE.endswith(b"q\n"))

    def test_the_show_is_driven_fast_enough_to_finish_in_the_deadline(self) -> None:
        # Ordinary motion has to actually run the gallery inside the probe's
        # deadline, or the ordinary run proves nothing about advancing.
        args = MODULE.SHOW_ARGS
        seconds = float(args[args.index("--seconds") + 1])
        self.assertLess(seconds * 2, MODULE.DEADLINE_SECONDS)
        self.assertIn("--mute", args, "the gate must not open an audio device")

    def test_every_frame_probe_names_a_marker(self) -> None:
        for label, args, marker in MODULE.PROBES:
            self.assertTrue(args, f"{label} runs the CLI with no arguments")
            self.assertTrue(marker.startswith("\x1b["), f"{label} has no frame marker")


if __name__ == "__main__":
    unittest.main()
