#!/usr/bin/env python3
"""Focused regressions for the reduced-motion contract.

These cover the judgment without spawning the CLI, so a mistake in what counts
as reduced motion working is caught even when nothing is built. The live probes
are `reduced-motion.py`, which CI runs against a real binary.

The judgment is the part worth testing on its own. A gate can be wrong in two
directions and only one of them is loud: a gate that fails a working feature is
noticed within the hour, and a gate that passes a broken one is noticed when a
player writes in. Every case below is a way for The Show to be broken, and each
asserts the judgment says so.
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
        # Ordinary on a pipe: one postcard, no prompt, then it exits.
        run(ended=True, rooms=0),
        # Held with nobody there: one room, then it stops.
        run(ended=True, rooms=1),
        # Held with a player: one room per answer.
        run(ended=True, rooms=MODULE.ASKED_TWICE_ROOMS),
    )


def healthy_recording() -> "MODULE.Recording":
    """Three frames, two titles, no cursor, and a clean exit."""
    frames = ["picture\nFirst\n", "picture\nFirst\n", "picture\nSecond\n"]
    return MODULE.Recording(ended=True, code=0, frames=frames, output="\x1c".join(frames))


class ShowJudgementTests(unittest.TestCase):
    def test_a_working_show_is_judged_to_pass(self) -> None:
        # Without this, every assertion below could be satisfied by a judgment
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

    def test_a_pipe_that_keeps_streaming_fails(self) -> None:
        # A redirect used to write until the pipe filled. Exiting is the fix,
        # so a run that is still going has the old bug.
        _, eof, asked = healthy()
        reasons = MODULE.judge_show(run(ended=False, rooms=0), eof, asked)
        self.assertTrue(any("did not exit" in reason for reason in reasons))

    def test_ordinary_motion_that_draws_nothing_fails(self) -> None:
        _, eof, asked = healthy()
        reasons = MODULE.judge_show(run(ended=True, rooms=0, output="   \n"), eof, asked)
        self.assertTrue(any("drew nothing" in reason for reason in reasons))

    def test_ordinary_motion_that_moves_the_cursor_fails(self) -> None:
        _, eof, asked = healthy()
        reasons = MODULE.judge_show(run(ended=True, rooms=0, output="\x1b[2Jpicture"), eof, asked)
        self.assertTrue(any("moved the cursor" in reason for reason in reasons))

    def test_drawing_nothing_is_not_holding_still(self) -> None:
        # The counterpart of the frame probes' blank-frame check: a Show that
        # emits no picture has stopped dead rather than held one.
        ordinary, _, asked = healthy()
        for empty in (
            "   \n  \n",
            # Only the prompt. This is the case a bare strip() would pass,
            # because the prompt is non-whitespace and looks like a drawing to
            # anything that merely asks whether the output was blank.
            MODULE.SHOW_PROMPT + "\n",
            # Only cursor moves and clears, which are also non-whitespace.
            "\x1b[H\x1b[2J\x1b[K",
            # Both together, which is exactly what a Show drawing nothing at
            # all would still emit.
            "\x1b[H" + MODULE.SHOW_PROMPT + "\x1b[K\n",
        ):
            reasons = MODULE.judge_show(ordinary, run(ended=True, rooms=1, output=empty), asked)
            self.assertTrue(
                any("drew nothing" in reason for reason in reasons),
                f"{empty!r} should not count as a picture",
            )

    def test_visible_keeps_the_picture_and_drops_the_furniture(self) -> None:
        # The instrument the check above measures with. One that stripped too
        # much would call every room blank, which fails loudly; one that
        # stripped too little passes a Show that drew nothing, which does not.
        self.assertEqual(MODULE.visible("\x1b[H##\x1b[K").strip(), "##")
        self.assertEqual(MODULE.visible(MODULE.SHOW_PROMPT).strip(), "")
        self.assertEqual(MODULE.visible("\x1b[38;2;1;2;3m##\x1b[0m").strip(), "##")
        self.assertEqual(MODULE.visible("plain").strip(), "plain")


class RecordingJudgementTests(unittest.TestCase):
    def test_a_working_recording_is_judged_to_pass(self) -> None:
        self.assertEqual(MODULE.judge_recording(healthy_recording(), 3), [])

    def test_a_recording_that_does_not_exit_fails(self) -> None:
        reasons = MODULE.judge_recording(
            MODULE.Recording(ended=False, code=0, frames=[], output=""), 3
        )
        self.assertTrue(any("did not exit" in reason for reason in reasons))

    def test_a_recording_that_stays_on_one_room_fails(self) -> None:
        frame = "picture\nOnly\n"
        reasons = MODULE.judge_recording(
            MODULE.Recording(ended=True, code=0, frames=[frame, frame, frame], output=frame * 3),
            3,
        )
        self.assertTrue(any("second room" in reason for reason in reasons))

    def test_a_recording_that_moves_the_cursor_or_waits_fails(self) -> None:
        frames = ["a\nOne\n", "a\nOne\n", "b\nTwo\n"]
        cursor = MODULE.judge_recording(
            MODULE.Recording(
                ended=True, code=0, frames=frames, output="\x1b[H" + "\x1c".join(frames)
            ),
            3,
        )
        self.assertTrue(any("cursor" in reason for reason in cursor))
        waiting = MODULE.judge_recording(
            MODULE.Recording(
                ended=True,
                code=0,
                frames=frames,
                output=MODULE.SHOW_PROMPT + "\x1c".join(frames),
            ),
            3,
        )
        self.assertTrue(any("waited" in reason for reason in waiting))


class ProbeTableTests(unittest.TestCase):
    def test_the_keys_sent_match_the_rooms_expected(self) -> None:
        # Two Enters and a q is three rooms: the two answered plus the one the
        # player was looking at when they quit. If these ever drift apart the
        # gate measures nothing, and it would still be green.
        self.assertEqual(MODULE.ASKED_TWICE.count(b"\n"), MODULE.ASKED_TWICE_ROOMS)
        self.assertTrue(MODULE.ASKED_TWICE.endswith(b"q\n"))

    def test_the_show_stays_muted_and_the_recording_reaches_another_room(self) -> None:
        # Full motion holds each room for at least two seconds of frames. The
        # recording asks for one frame past that hold, so a second title is
        # evidence the gallery moved rather than a guess about the clock.
        self.assertIn("--mute", MODULE.SHOW_ARGS, "the gate must not open an audio device")
        self.assertIn("--mute", MODULE.RECORD_ARGS)
        args = MODULE.RECORD_ARGS
        seconds = float(args[args.index("--seconds") + 1])
        fps = float(args[args.index("--fps") + 1])
        count = int(args[args.index("--frames") + 1])
        self.assertEqual(count, MODULE.room_hold_frames(seconds, fps) + 1)
        self.assertLess(seconds, MODULE.DEADLINE_SECONDS)

    def test_every_frame_probe_records_a_finite_run(self) -> None:
        for label, args, marker in MODULE.PROBES:
            self.assertTrue(args, f"{label} runs the CLI with no arguments")
            self.assertIn("--frames", args, f"{label} would stream on a pipe")
            self.assertEqual(
                marker, "\n\x1c\n", f"{label} does not split on the recording separator"
            )


if __name__ == "__main__":
    unittest.main()
