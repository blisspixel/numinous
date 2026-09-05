#!/usr/bin/env python3
"""Focused regressions for the creator parity contract.

These cover the judgment without spawning either face, so a mistake in what
counts as agreement is caught even when nothing is built. The live comparison
is `creator-parity.py`, which CI runs against real binaries.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "creator_parity", ROOT / "scripts" / "creator-parity.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PlotBodyTests(unittest.TestCase):
    def test_each_face_may_keep_its_own_voice(self) -> None:
        # The CLI prints a header; MCP prints a header and a discovery line.
        # Requiring those to match would require the faces to be one thing
        # rather than to agree about the mathematics.
        cli = "y = sin(x)    x in [-6.283, 6.283]\n\n  ##  \n ####\n"
        mcp = "y = sin(x)    x in [-6.283, 6.283]\nDiscovery: manual\n\n  ##  \n ####\n"
        self.assertEqual(MODULE.plot_body(cli), MODULE.plot_body(mcp))

    def test_a_different_drawing_is_not_equal(self) -> None:
        self.assertNotEqual(
            MODULE.plot_body("y = a\n\n ## \n"),
            MODULE.plot_body("y = a\n\n #  \n"),
        )

    def test_trailing_whitespace_is_not_a_disagreement(self) -> None:
        # Two faces padding a row differently is not a difference in the plot.
        self.assertEqual(
            MODULE.plot_body("y = a\n\n ## \n"),
            MODULE.plot_body("y = a\n\n ##\n"),
        )

    def test_an_empty_render_collapses_to_nothing(self) -> None:
        self.assertEqual(MODULE.plot_body("y = a\nDiscovery: manual\n\n"), "")


class GeometryTests(unittest.TestCase):
    def response(self, width: object = 12, height: object = 4) -> dict:
        return {
            "content": [{"text": "x(t) = cos(t)    y(t) = sin(t)\n\n\n    ##\n    ##\n\n"}],
            "structuredContent": {"width": width, "height": height},
        }

    def test_blank_margins_do_not_resize_the_other_faces_request(self) -> None:
        response = self.response()
        cli = response["content"][0]["text"] + "LEVEL UP\n"
        with mock.patch.object(MODULE, "mcp_plot", return_value=response), mock.patch.object(
            MODULE, "cli_plot", return_value=cli
        ) as call:
            result = MODULE.check("cli", "mcp", "centered", {}, [], {})
        self.assertTrue(result["passed"], result)
        call.assert_called_once_with("cli", [], 12, 4, {})

    def test_a_declared_canvas_does_not_excuse_different_ink(self) -> None:
        response = self.response()
        cli = response["content"][0]["text"].replace("    ##", "   ##", 1)
        with mock.patch.object(MODULE, "mcp_plot", return_value=response), mock.patch.object(
            MODULE, "cli_plot", return_value=cli
        ):
            result = MODULE.check("cli", "mcp", "shifted", {}, [], {})
        self.assertFalse(result["passed"])

    def test_missing_invalid_or_inconsistent_geometry_fails_before_cli(self) -> None:
        for width, height in [(None, 4), (True, 4), (0, 4), (12, -1), (12, 5), (5, 4)]:
            with self.subTest(width=width, height=height), mock.patch.object(
                MODULE, "mcp_plot", return_value=self.response(width, height)
            ), mock.patch.object(MODULE, "cli_plot") as call:
                result = MODULE.check("cli", "mcp", "invalid", {}, [], {})
            self.assertFalse(result["passed"], result)
            call.assert_not_called()


class CaseTableTests(unittest.TestCase):
    def test_every_case_drives_both_faces(self) -> None:
        for label, mcp_args, cli_args in MODULE.CASES:
            self.assertTrue(mcp_args, f"{label} sends MCP nothing")
            self.assertTrue(cli_args, f"{label} sends the CLI nothing")

    def test_labels_are_unique_so_a_failure_names_one_case(self) -> None:
        labels = [label for label, _, _ in MODULE.CASES]
        self.assertEqual(len(labels), len(set(labels)))

    def test_the_table_covers_every_discovery_path(self) -> None:
        # Expression, recipe, and seed are the three ways into a creation, and
        # the knob and range are what a player changes once inside one. A gate
        # that only covered expressions would miss a whole face of the surface.
        keys = {key for _, mcp_args, _ in MODULE.CASES for key in mcp_args}
        for required in ("expr", "recipe", "seed", "a", "xmin", "xmax"):
            self.assertIn(required, keys, f"no case exercises {required}")

    def test_the_table_covers_the_current_scalar_grammar_rung(self) -> None:
        expressions = "\n".join(
            str(mcp_args.get("expr", "")) for _, mcp_args, _ in MODULE.CASES
        )
        for function in ("floor(", "mod(", "min(", "max("):
            self.assertIn(function, expressions, f"no parity case exercises {function}")

    def test_range_cases_avoid_the_bare_negative_trap(self) -> None:
        # A bare -2 is read as a flag, not a value, so a case written that way
        # would fail for a reason that has nothing to do with parity.
        for label, _, cli_args in MODULE.CASES:
            for index, argument in enumerate(cli_args):
                if argument in ("--xmin", "--xmax", "--a"):
                    following = cli_args[index + 1] if index + 1 < len(cli_args) else ""
                    self.assertFalse(
                        following.startswith("-"),
                        f"{label} passes {argument} a bare negative; use {argument}=value",
                    )

    def test_one_live_sing_case_omits_the_shared_note_default(self) -> None:
        defaults = [case for case in MODULE.SING_CASES if case[2] is None]
        self.assertEqual(len(defaults), 1)
        self.assertEqual(defaults[0][0], "neither face is told the note count")


if __name__ == "__main__":
    unittest.main()
