#!/usr/bin/env python3
"""The game-truth gate's own parsers, held to their jobs.

The gate reads a total from RUN COMPLETE and a bomb code from the BOOM
reveal; a parser that quietly matched nothing would let the gate pass while
checking nothing, which is the exact disease the gate exists to catch.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "game_truth", Path(__file__).resolve().parent / "game-truth.py"
)
assert SPEC is not None and SPEC.loader is not None
game_truth = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(game_truth)


class ParserTests(unittest.TestCase):
    def test_the_total_is_read_from_the_run_complete_line(self) -> None:
        stdout = "noise\nRUN COMPLETE  2/4 clean  TOTAL 65  (gauntlet seed:11)\n"
        self.assertEqual(game_truth.parse_total(stdout), 65)

    def test_a_missing_total_is_a_loud_failure_not_a_zero(self) -> None:
        with self.assertRaises(game_truth.GameTruthError):
            game_truth.parse_total("the run never finished")

    def test_the_bomb_code_is_read_from_the_boom_reveal(self) -> None:
        stdout = "BOOM\n  It was 4711. +0 points\n"
        self.assertEqual(game_truth.parse_bomb_code(stdout), "4711")

    def test_a_missing_reveal_is_a_loud_failure(self) -> None:
        with self.assertRaises(game_truth.GameTruthError):
            game_truth.parse_bomb_code("DEFUSED")

    def test_the_isolated_profile_redirects_both_stores(self) -> None:
        env = game_truth.isolated_env(Path("scratch"))
        self.assertTrue(env["NUMINOUS_SCORES"].endswith("scores"))
        self.assertTrue(env["NUMINOUS_JOURNEY"].endswith("journey"))
        self.assertEqual(Path(env["HOME"]), Path("scratch"))


if __name__ == "__main__":
    unittest.main()
