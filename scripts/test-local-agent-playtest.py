#!/usr/bin/env python3
"""Regression tests for the zero-cost local model playtest harness."""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
HARNESS_PATH = ROOT / "scripts" / "local-agent-playtest.py"


def load_harness():
    spec = importlib.util.spec_from_file_location(
        "numinous_local_agent_playtest", HARNESS_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load local-agent-playtest.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


HARNESS = load_harness()


def tool_definition(name: str) -> dict:
    return {
        "name": name,
        "description": f"Use {name} as a bounded player action.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": False,
            "properties": {},
        },
    }


def assistant_response(
    *, content: str = "", calls: list[dict] | None = None, thinking: str | None = None
) -> dict:
    message = {
        "role": "assistant",
        "content": content,
        "tool_calls": calls or [],
    }
    if thinking is not None:
        message["thinking"] = thinking
    return {
        "message": message,
        "prompt_eval_count": 100,
        "eval_count": 20,
        "total_duration": 2_000_000_000,
        "load_duration": 100_000_000,
        "prompt_eval_duration": 800_000_000,
        "eval_duration": 1_100_000_000,
    }


def call(name: str, arguments: dict) -> dict:
    return {"function": {"name": name, "arguments": arguments}}


class FakeClient:
    def __init__(self, responses: list[dict | Exception]) -> None:
        self.responses = list(responses)
        self.requests: list[dict] = []

    def chat(self, model, messages, tools, **options):
        self.requests.append(
            {
                "model": model,
                "messages": copy.deepcopy(messages),
                "tools": copy.deepcopy(tools),
                "options": copy.deepcopy(options),
            }
        )
        if not self.responses:
            raise AssertionError("fake model received an unexpected turn")
        response = self.responses.pop(0)
        if isinstance(response, Exception):
            raise response
        return response


class FakeProfile:
    def __init__(self) -> None:
        names = set(HARNESS.FIRST_CONTACT_TOOLS)
        names.update({"forget", "erase_journal", "broadcast_session", "scores"})
        self.definitions = [tool_definition(name) for name in sorted(names)]
        self.calls: list[tuple[str, dict]] = []

    def list_tools(self):
        return self.definitions, {
            "schemaVersion": "numinous-mcp-development-build-receipt-v1",
            "binarySha256": "a" * 64,
        }

    def call_tool(self, name, arguments):
        self.calls.append((name, arguments))
        return {
            "content": [{"type": "text", "text": f"{name} answered."}],
            "structuredContent": {"tool": name, "arguments": arguments},
            "isError": False,
        }


class BoundaryTests(unittest.TestCase):
    def test_windows_reparse_points_count_as_redirects(self) -> None:
        reparse_flag = 1024

        class ReparsePath:
            @staticmethod
            def lstat():
                return type(
                    "Metadata",
                    (),
                    {"st_file_attributes": reparse_flag},
                )()

            @staticmethod
            def is_symlink():
                return False

        with mock.patch.object(
            HARNESS.stat,
            "FILE_ATTRIBUTE_REPARSE_POINT",
            reparse_flag,
            create=True,
        ):
            self.assertTrue(HARNESS.is_redirecting_path(ReparsePath()))

    def test_endpoint_is_literal_loopback_only(self) -> None:
        self.assertEqual(
            HARNESS.validate_endpoint("http://127.0.0.1:11434"),
            "http://127.0.0.1:11434",
        )
        self.assertEqual(
            HARNESS.validate_endpoint("http://[::1]:11434/"),
            "http://[::1]:11434",
        )
        for invalid in (
            "https://127.0.0.1:11434",
            "http://localhost:11434",
            "http://192.168.1.2:11434",
            "http://127.0.0.1:11434/api",
            "http://user@127.0.0.1:11434",
            "http://127.0.0.1",
        ):
            with self.subTest(invalid=invalid):
                with self.assertRaises(HARNESS.LocalPlaytestError):
                    HARNESS.validate_endpoint(invalid)

    def test_cloud_and_malformed_model_names_are_rejected(self) -> None:
        self.assertEqual(
            HARNESS.validate_model_name("devstral-small-2:24b"),
            "devstral-small-2:24b",
        )
        for invalid in (
            "gpt-oss:120b-cloud",
            "gpt-oss:cloud",
            "../model",
            "model name",
            "",
        ):
            with self.subTest(invalid=invalid):
                with self.assertRaises(HARNESS.LocalPlaytestError):
                    HARNESS.validate_model_name(invalid)

    def test_model_must_be_installed_with_weights_and_tools(self) -> None:
        class InventoryClient:
            def installed_models(self):
                return [
                    {
                        "name": "larger:24b",
                        "model": "larger:24b",
                        "size": 15_000_000_001,
                        "digest": "c" * 64,
                    },
                    {
                        "name": "devstral-small-2:24b",
                        "model": "devstral-small-2:24b",
                        "size": 15_000_000_000,
                        "digest": "b" * 64,
                    }
                ]

            def show_model(self, model):
                self.model = model
                return {"capabilities": ["completion", "tools"]}

        client = InventoryClient()
        model, inventory, details = HARNESS.choose_local_model(client, None)
        self.assertEqual(model, "devstral-small-2:24b")
        self.assertEqual(inventory["digest"], "b" * 64)
        self.assertIn("tools", details["capabilities"])
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "not installed"):
            HARNESS.choose_local_model(client, "missing:1b")
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "cloud models"):
            HARNESS.choose_local_model(client, "model:cloud")

    def test_selected_model_requires_a_valid_local_digest(self) -> None:
        class InventoryClient:
            @staticmethod
            def installed_models():
                return [{"name": "local:1b", "size": 1, "digest": "bad"}]

            @staticmethod
            def show_model(_model):
                raise AssertionError("an unverified local payload was inspected")

        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "digest"):
            HARNESS.choose_local_model(InventoryClient(), "local:1b")

    def test_tool_palettes_are_closed_and_destructive_controls_stay_absent(self) -> None:
        profile = FakeProfile()
        first = HARNESS.select_tools(profile.definitions, "first-contact")
        first_names = {item["function"]["name"] for item in first}
        self.assertEqual(first_names, HARNESS.FIRST_CONTACT_TOOLS)
        full = HARNESS.select_tools(profile.definitions, "full-player")
        full_names = {item["function"]["name"] for item in full}
        self.assertIn("scores", full_names)
        self.assertTrue(full_names.isdisjoint(HARNESS.EXCLUDED_FULL_PLAYER_TOOLS))

    def test_missing_or_duplicate_tool_definitions_fail_closed(self) -> None:
        definitions = FakeProfile().definitions
        missing = [item for item in definitions if item["name"] != "play_room"]
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "play_room"):
            HARNESS.select_tools(missing, "first-contact")
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "duplicate"):
            HARNESS.select_tools([*definitions, definitions[0]], "full-player")

    def test_tool_results_and_visible_model_text_are_bounded(self) -> None:
        text, truncated = HARNESS.bounded_text(
            "x" * (HARNESS.MAX_VISIBLE_RESPONSE_CHARACTERS + 1),
            HARNESS.MAX_VISIBLE_RESPONSE_CHARACTERS,
        )
        self.assertTrue(truncated)
        self.assertEqual(len(text), HARNESS.MAX_VISIBLE_RESPONSE_CHARACTERS)
        encoded, result_truncated = HARNESS.tool_result_text(
            {"content": [{"type": "text", "text": "y" * 100_000}]}
        )
        self.assertTrue(result_truncated)
        self.assertLessEqual(len(encoded), HARNESS.MAX_TOOL_RESULT_CHARACTERS)
        self.assertTrue(json.loads(encoded)["structuredContentOmittedByHarness"])
        oversized = assistant_response(
            content="z" * (HARNESS.MAX_MODEL_MESSAGE_CHARACTERS + 1)
        )
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "message exceeds"):
            HARNESS.normalize_message(oversized)
        too_many = assistant_response(
            calls=[call("list_rooms", {})]
            * (HARNESS.MAX_MODEL_TOOL_CALLS_PER_TURN + 1)
        )
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "too many tool calls"):
            HARNESS.normalize_message(too_many)


class LoopTests(unittest.TestCase):
    @staticmethod
    def execute_playtest(client: FakeClient, profile: FakeProfile, **overrides):
        arguments = {
            "model": "devstral-small-2:24b",
            "model_inventory": {
                "digest": "b" * 64,
                "size": 15_000_000_000,
            },
            "model_details": {
                "capabilities": ["completion", "tools"],
                "details": {"parameter_size": "24B"},
            },
            "palette": "first-contact",
            "max_turns": 4,
            "max_tool_calls": 4,
            "seed": 17,
            "context_tokens": 4096,
        }
        arguments.update(overrides)
        with mock.patch("builtins.print"):
            return HARNESS.run_playtest(client, profile, **arguments)

    def test_real_tool_loop_reports_play_without_private_reasoning(self) -> None:
        client = FakeClient(
            [
                assistant_response(
                    calls=[call("list_rooms", {"response_mode": "compact"})],
                    thinking="PRIVATE FIRST THOUGHT",
                ),
                assistant_response(
                    calls=[
                        call(
                            "play_room",
                            {"id": "double-pendulum", "t": 0.5},
                        )
                    ],
                    thinking="PRIVATE SECOND THOUGHT",
                ),
                assistant_response(
                    content="The widening gap was worth staying with.",
                    thinking="PRIVATE FINAL THOUGHT",
                ),
            ]
        )
        profile = FakeProfile()
        report = self.execute_playtest(client, profile)
        self.assertEqual(report["result"]["exitReason"], "model_finished")
        self.assertEqual(report["result"]["toolCalls"], 2)
        self.assertEqual(report["result"]["successfulToolCalls"], 2)
        self.assertEqual(
            report["result"]["finalResponse"],
            "The widening gap was worth staying with.",
        )
        self.assertEqual(report["result"]["roomIds"], ["double-pendulum"])
        self.assertEqual(report["execution"]["estimatedCostUsd"], 0.0)
        self.assertFalse(report["execution"]["privateReasoningRecorded"])
        encoded = json.dumps(report)
        self.assertNotIn("PRIVATE", encoded)
        self.assertEqual(
            profile.calls,
            [
                ("list_rooms", {"response_mode": "compact"}),
                ("play_room", {"id": "double-pendulum", "t": 0.5}),
            ],
        )
        self.assertEqual(report["result"]["promptTokens"], 300)
        self.assertEqual(report["result"]["outputTokens"], 60)
        tool_history = client.requests[1]["messages"][-1]
        self.assertEqual(tool_history["role"], "tool")
        self.assertEqual(tool_history["tool_name"], "list_rooms")

    def test_out_of_palette_call_becomes_feedback_instead_of_execution(self) -> None:
        client = FakeClient(
            [
                assistant_response(calls=[call("forget", {"confirm": True})]),
                assistant_response(content="I could not use that control."),
            ]
        )
        profile = FakeProfile()
        report = self.execute_playtest(client, profile)
        self.assertEqual(profile.calls, [])
        self.assertEqual(report["result"]["toolErrors"], 1)
        self.assertEqual(report["result"]["successfulToolCalls"], 0)
        self.assertIn("outside this palette", client.requests[1]["messages"][-1]["content"])

    def test_narrated_calls_are_not_mistaken_for_play_and_get_one_recovery(self) -> None:
        client = FakeClient(
            [
                assistant_response(
                    content="I used list_rooms and then play_room on the pendulum."
                ),
                assistant_response(calls=[call("list_rooms", {})]),
                assistant_response(content="Now I actually saw the room list."),
            ]
        )
        profile = FakeProfile()
        report = self.execute_playtest(client, profile)
        self.assertEqual(report["result"]["narrationReprompts"], 1)
        self.assertEqual(
            report["result"]["unexecutedToolClaims"], ["list_rooms", "play_room"]
        )
        self.assertEqual(profile.calls, [("list_rooms", {})])
        recovery = client.requests[1]["messages"][-1]
        self.assertEqual(recovery["role"], "user")
        self.assertIn("did not happen", recovery["content"])

    def test_narration_beside_a_real_call_does_not_claim_extra_execution(self) -> None:
        client = FakeClient(
            [
                assistant_response(
                    content="I called list_rooms and reveal_room.",
                    calls=[call("list_rooms", {})],
                ),
                assistant_response(content="I am finished."),
            ]
        )
        report = self.execute_playtest(client, FakeProfile())
        self.assertEqual(report["result"]["successfulToolCalls"], 1)
        self.assertEqual(report["result"]["unexecutedToolClaims"], ["reveal_room"])

    def test_turn_bound_gets_one_tool_free_closing_reflection(self) -> None:
        client = FakeClient(
            [
                assistant_response(calls=[call("list_rooms", {})]),
                assistant_response(content="I would continue with the pendulum."),
            ]
        )
        report = self.execute_playtest(client, FakeProfile(), max_turns=1)
        self.assertEqual(report["result"]["exitReason"], "turn_limit")
        self.assertEqual(report["result"]["inferenceTurns"], 2)
        self.assertEqual(client.requests[-1]["tools"], [])
        self.assertEqual(
            report["events"][-1]["type"], "closing_reflection"
        )

    def test_model_failure_preserves_prior_witnessed_play(self) -> None:
        client = FakeClient(
            [
                assistant_response(calls=[call("list_rooms", {})]),
                HARNESS.LocalPlaytestError("local inference timed out"),
            ]
        )
        report = self.execute_playtest(client, FakeProfile())
        self.assertEqual(report["result"]["exitReason"], "model_error")
        self.assertEqual(report["result"]["successfulToolCalls"], 1)
        self.assertEqual(report["result"]["modelError"], "local inference timed out")
        self.assertEqual(report["events"][-1]["type"], "model_error")

    def test_invalid_usage_and_limits_fail_closed(self) -> None:
        invalid = assistant_response(content="done")
        invalid["eval_count"] = -1
        report = self.execute_playtest(FakeClient([invalid]), FakeProfile())
        self.assertEqual(report["result"]["exitReason"], "model_error")
        self.assertIn("eval_count", report["result"]["modelError"])
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "turn limit"):
            self.execute_playtest(FakeClient([]), FakeProfile(), max_turns=0)
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "context"):
            self.execute_playtest(FakeClient([]), FakeProfile(), context_tokens=1024)
        with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "palette"):
            self.execute_playtest(FakeClient([]), FakeProfile(), palette="unbounded")


class TranscriptTests(unittest.TestCase):
    def test_transcript_is_opt_in_new_json_beneath_logs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with mock.patch.object(HARNESS, "ROOT", root):
                path = HARNESS.transcript_path("logs/visit.json")
                HARNESS.write_transcript(path, {"schema": "test", "events": []})
                self.assertEqual(
                    json.loads(path.read_text(encoding="utf-8"))["schema"], "test"
                )
                with self.assertRaisesRegex(
                    HARNESS.LocalPlaytestError, "already exists"
                ):
                    HARNESS.transcript_path("logs/visit.json")
                with self.assertRaisesRegex(HARNESS.LocalPlaytestError, "beneath"):
                    HARNESS.transcript_path("outside.json")

    def test_transcript_rejects_a_redirected_logs_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            redirected = root / "redirected"
            redirected.mkdir()
            logs = root / "logs"
            try:
                logs.symlink_to(redirected, target_is_directory=True)
            except OSError:
                self.skipTest("this account cannot create directory symlinks")
            with (
                mock.patch.object(HARNESS, "ROOT", root),
                self.assertRaisesRegex(HARNESS.LocalPlaytestError, "ordinary"),
            ):
                HARNESS.transcript_path("logs/visit.json")

    def test_transcript_creation_never_overwrites_a_racing_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with mock.patch.object(HARNESS, "ROOT", root):
                path = HARNESS.transcript_path("logs/visit.json")
                original_link = HARNESS.os.link

                def occupy_then_link(source, destination):
                    destination.write_text("existing", encoding="utf-8")
                    return original_link(source, destination)

                with (
                    mock.patch.object(HARNESS.os, "link", side_effect=occupy_then_link),
                    self.assertRaisesRegex(HARNESS.LocalPlaytestError, "appeared"),
                ):
                    HARNESS.write_transcript(path, {"schema": "test"})
                self.assertEqual(path.read_text(encoding="utf-8"), "existing")


if __name__ == "__main__":
    unittest.main(verbosity=2)
