#!/usr/bin/env python3
"""Focused regressions for agent hallway and tactile machine cohorts.

These tests cover pure scoring and summary contracts without spawning MCP.
Live cohort scripts remain separate CI steps that exercise real binaries.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def load_module(name: str, relative: str):
    path = ROOT / relative
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {relative}")
    module = importlib.util.module_from_spec(spec)
    # dataclasses need the module present in sys.modules during class body exec
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


hallway = load_module("numinous_agent_hallway", "scripts/agent-hallway.py")
tactile = load_module("numinous_agent_tactile", "scripts/agent-tactile.py")
first_contact = load_module(
    "numinous_agent_first_contact", "scripts/agent-first-contact.py"
)


def aha_step(beat: str | None, earn=None, reveal=None, kind: str | None = None) -> dict:
    engineered: dict = {}
    if beat is not None:
        engineered["beat"] = beat
    if earn is not None:
        engineered["earn"] = earn
    if kind is not None:
        engineered["kind"] = kind
    structured: dict = {"engineeredAha": engineered}
    if reveal is not None:
        structured["reveal"] = reveal
    return {"ok": True, "structured": structured, "text": ""}


class AgentHallwayTests(unittest.TestCase):
    def test_score_times_accepts_consolidated_summon(self) -> None:
        steps = [
            aha_step("open", kind="place"),
            aha_step("withheld", kind="place"),
            aha_step("consolidated", earn="four-lobe", reveal="Mandelbrot", kind="place"),
        ]
        # open engineeredAha must advertise place kind at cold start
        steps[0]["structured"]["engineeredAha"] = {"kind": "place"}
        score = hallway.score_times(steps)
        self.assertTrue(score["passed"], score["findings"])

    def test_score_times_rejects_early_reveal(self) -> None:
        steps = [
            aha_step(None, kind="place", reveal="spoiler"),
            aha_step("withheld"),
            aha_step("consolidated", reveal="ok"),
        ]
        steps[0]["structured"]["engineeredAha"] = {"kind": "place"}
        steps[0]["structured"]["reveal"] = "spoiler"
        score = hallway.score_times(steps)
        self.assertFalse(score["passed"])
        self.assertTrue(any("leak" in finding for finding in score["findings"]))

    def test_score_buffon_requires_number_kind(self) -> None:
        steps = [
            aha_step(None),
            aha_step("withheld"),
            aha_step("consolidated", reveal="pi"),
        ]
        steps[0]["structured"]["engineeredAha"] = {"kind": "place"}
        score = hallway.score_buffon(steps)
        self.assertFalse(score["passed"])
        self.assertTrue(any("number" in finding for finding in score["findings"]))

    def test_cohort_summary_fails_closed(self) -> None:
        summary = hallway.cohort_summary(
            {"ok": True},
            {"passed": True, "findings": []},
            {"passed": False, "findings": ["summon did not consolidate"]},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
        )
        self.assertFalse(summary["passed"])
        self.assertEqual(summary["suite"], "agent-hallway")
        self.assertEqual(summary["personas"], len(hallway.PERSONAS))
        self.assertIn("summon did not consolidate", summary["findings"])

    def test_cohort_summary_passes_when_all_green(self) -> None:
        summary = hallway.cohort_summary(
            {"ok": True},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
            {"passed": True, "findings": []},
        )
        self.assertTrue(summary["passed"])
        self.assertEqual(summary["findings"], [])

    def test_score_kepler_requires_typed_truth_and_visual_evidence(self) -> None:
        steps = [
            aha_step("explore", kind="speed"),
            aha_step("withheld", kind="speed"),
            aha_step(
                "consolidated",
                earn="call:faster:right",
                reveal="Equal areas",
                kind="speed",
            ),
        ]
        steps[2]["structured"]["engineeredAha"].update(
            {"truth": "faster", "wager": "faster"}
        )
        steps[2]["structured"]["render"] = "orbit O O O"
        score = hallway.score_kepler(steps)
        self.assertTrue(score["passed"], score["findings"])

        steps[2]["structured"]["render"] = "orbit only"
        score = hallway.score_kepler(steps)
        self.assertFalse(score["passed"])
        self.assertTrue(any("equal-time" in finding for finding in score["findings"]))

    def test_score_parrondo_requires_exact_typed_truth_and_three_paths(self) -> None:
        steps = [
            aha_step("explore", kind="policy"),
            aha_step("withheld", kind="policy"),
            aha_step(
                "consolidated",
                earn="call:abb:right",
                reveal="Two losing games can combine",
                kind="policy",
            ),
        ]
        steps[2]["structured"]["engineeredAha"].update(
            {
                "truth": "abb",
                "wager": "abb",
                "expectedEnd": {"a": -1.2, "b": -1.5, "abb": 7.0},
            }
        )
        steps[2]["structured"]["render"] = "A B O"
        score = hallway.score_parrondo(steps)
        self.assertTrue(score["passed"], score["findings"])

        steps[2]["structured"]["engineeredAha"]["expectedEnd"]["abb"] = -0.1
        score = hallway.score_parrondo(steps)
        self.assertFalse(score["passed"])
        self.assertTrue(any("expectations" in finding for finding in score["findings"]))

    def test_score_nontransitive_requires_exact_cycle_and_outcome_grid(self) -> None:
        steps = [
            aha_step("explore", kind="counter"),
            aha_step("withheld", kind="counter"),
            aha_step(
                "consolidated",
                earn="call:c:right",
                reveal="No best die",
                kind="counter",
            ),
        ]
        steps[2]["structured"]["engineeredAha"].update(
            {
                "chosen": "a",
                "truth": "c",
                "wager": "c",
                "counterWins": 20,
                "exactCycle": {
                    "aOverB": 24,
                    "bOverC": 24,
                    "cOverA": 20,
                    "outcomesPerPair": 36,
                },
            }
        )
        steps[2]["structured"]["render"] = "C vs A\n20 W / 16 L"
        score = hallway.score_nontransitive(steps)
        self.assertTrue(score["passed"], score["findings"])

        steps[2]["structured"]["engineeredAha"]["counterWins"] = 19
        score = hallway.score_nontransitive(steps)
        self.assertFalse(score["passed"])
        self.assertTrue(any("C beats A" in finding for finding in score["findings"]))


class AgentTactileTests(unittest.TestCase):
    def test_probe_inventory_covers_five_flagships(self) -> None:
        slugs = {probe.slug for probe in tactile.PROBES}
        self.assertEqual(
            slugs,
            {
                "times-tables",
                "double-pendulum",
                "game-of-life",
                "galton-board",
                "formula-jam",
            },
        )

    def test_status_of_prefers_structured_fields(self) -> None:
        status = tactile.status_of(
            {
                "ok": True,
                "text": "fallback text",
                "structured": {"status": "DRAG:DIAL K 2"},
            }
        )
        self.assertEqual(status, "DRAG:DIAL K 2")

    def test_canonical_plate_requires_structured_visual(self) -> None:
        empty = tactile.canonical_plate({"ok": True, "text": "ascii", "structured": {}}, "play_room")
        self.assertEqual(empty, "")
        plate = tactile.canonical_plate(
            {"ok": True, "structured": {"render": "###"}},
            "play_room",
        )
        self.assertEqual(plate, "###")

    def test_digest_is_stable(self) -> None:
        self.assertEqual(tactile.digest("same"), tactile.digest("same"))
        self.assertNotEqual(tactile.digest("a"), tactile.digest("b"))

    def test_close_accepts_relative_tolerance(self) -> None:
        self.assertTrue(tactile.close(1.0, 1.0 + 1e-9))
        self.assertFalse(tactile.close(1.0, 1.1))


class AgentFirstContactTests(unittest.TestCase):
    def test_contact_rooms_include_flagships(self) -> None:
        ids = {room_id for room_id, _ in first_contact.CONTACT_ROOMS}
        for required in (
            "times-tables",
            "double-pendulum",
            "game-of-life",
            "galton-board",
            "buffon-needle",
        ):
            self.assertIn(required, ids)

    def test_expected_tool_count_matches_public_surface(self) -> None:
        self.assertEqual(first_contact.EXPECTED_TOOL_COUNT, 36)

    def test_status_of_reads_structured_status(self) -> None:
        status = first_contact.status_of(
            {"ok": True, "structured": {"status": "CLICK:DROP"}, "text": "x"}
        )
        self.assertEqual(status, "CLICK:DROP")


if __name__ == "__main__":
    raise SystemExit(unittest.main())
