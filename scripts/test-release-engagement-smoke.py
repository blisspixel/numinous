#!/usr/bin/env python3
"""Regression tests for the installed release engagement contract."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from typing import Any
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_release_engagement_smoke",
    ROOT / "scripts" / "release-engagement-smoke.py",
)
assert SPEC is not None and SPEC.loader is not None
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)


def terminate_process(process_id: int) -> None:
    """Terminate one regression-test descendant without leaving temp locks."""
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process_id), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=5,
            check=False,
        )
        return
    try:
        os.kill(process_id, signal.SIGTERM)
    except ProcessLookupError:
        pass


def valid_render() -> str:
    rows = ["*" for _ in range(20)]
    rows[0] = "*#*"
    rows.extend(
        (
            "Status: K 4.00  CLOSED  3 LOBES  TARGET 4",
            "Action: TURN THE DIAL (the hand here: numinous render times-tables --poke x,y)",
            "Goal: LAND ON EXACTLY 4 LOBES",
        )
    )
    return "\n".join(rows) + "\n"


def valid_responses() -> list[dict[str, Any]]:
    tools = [{"name": name} for name in sorted(SMOKE.EXPECTED_TOOL_NAMES)]
    return [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "_meta": {
                    "io.modelcontextprotocol/serverInfo": {
                        "name": "numinous",
                        "version": "0.3.0-alpha.1",
                    }
                },
                "resultType": "complete",
                "supportedVersions": [SMOKE.PROTOCOL_VERSION],
            },
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "_meta": {
                    "io.modelcontextprotocol/serverInfo": {
                        "name": "numinous",
                        "version": "0.3.0-alpha.1",
                    }
                },
                "resultType": "complete",
                "tools": tools,
            },
        },
        {
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "_meta": {
                    "io.modelcontextprotocol/serverInfo": {
                        "name": "numinous",
                        "version": "0.3.0-alpha.1",
                    }
                },
                "resultType": "complete",
                "isError": False,
                "structuredContent": {
                    "room": "times-tables",
                    "width": 40,
                    "height": 20,
                    "t": 0.25,
                    "action": "DRAG: TURN THE DIAL",
                    "goal": "LAND ON EXACTLY 4 LOBES",
                    "status": "DRAG:DIAL  K 4.00  CLOSED  3 LOBES  TARGET 4",
                    "render": valid_render(),
                },
            },
        },
    ]


class ReleaseEngagementSmokeTests(unittest.TestCase):
    def test_valid_contract_accepts_discovery_inventory_and_play(self) -> None:
        SMOKE.validate_mcp_responses(valid_responses(), "0.3.0-alpha.1")

    def test_cli_version_requires_one_well_formed_version(self) -> None:
        self.assertEqual(
            SMOKE.validate_cli_version("numinous 0.3.0-alpha.1\n"),
            "0.3.0-alpha.1",
        )
        for invalid in (
            "",
            "numinous v0.2.0\n",
            "numinous 0.2\n",
            "numinous 0.2.0\nextra\n",
        ):
            with self.subTest(invalid=invalid):
                with self.assertRaises(SMOKE.SmokeError):
                    SMOKE.validate_cli_version(invalid)

    def test_binary_versions_must_match_across_faces(self) -> None:
        with self.assertRaisesRegex(SMOKE.SmokeError, "versions do not match"):
            SMOKE.validate_mcp_responses(valid_responses(), "0.2.0-alpha.6")
        inconsistent = valid_responses()
        inconsistent[2]["result"]["_meta"]["io.modelcontextprotocol/serverInfo"][
            "version"
        ] = "0.2.0-alpha.6"
        with self.assertRaisesRegex(SMOKE.SmokeError, "disagree"):
            SMOKE.validate_mcp_responses(inconsistent)

    def test_room_render_requires_substantive_semantic_markers(self) -> None:
        SMOKE.validate_cli_render(valid_render())
        with self.assertRaisesRegex(SMOKE.SmokeError, "Goal"):
            SMOKE.validate_cli_render(valid_render().replace("Goal:", "Missing:"))
        with self.assertRaisesRegex(SMOKE.SmokeError, "too few rows"):
            SMOKE.validate_cli_render(
                "Status:\nAction: TURN THE DIAL (the hand here: numinous render "
                "times-tables --poke x,y)\nGoal: LAND ON EXACTLY 4 LOBES\n"
            )

    def test_inventory_requires_exact_unique_tool_set(self) -> None:
        too_short = valid_responses()
        too_short[1]["result"]["tools"].pop()
        with self.assertRaisesRegex(SMOKE.SmokeError, "35 tools"):
            SMOKE.validate_mcp_responses(too_short)

        duplicate = valid_responses()
        duplicate[1]["result"]["tools"][1] = {"name": "play_room"}
        with self.assertRaisesRegex(SMOKE.SmokeError, "duplicate"):
            SMOKE.validate_mcp_responses(duplicate)

        missing = valid_responses()
        missing[1]["result"]["tools"][0] = {"name": "replacement"}
        with self.assertRaisesRegex(SMOKE.SmokeError, "exact expected inventory"):
            SMOKE.validate_mcp_responses(missing)

    def test_json_rpc_response_envelopes_are_exact(self) -> None:
        for invalid in (None, "1.0", 2):
            responses = valid_responses()
            if invalid is None:
                responses[0].pop("jsonrpc")
            else:
                responses[0]["jsonrpc"] = invalid
            with self.subTest(invalid=invalid):
                with self.assertRaisesRegex(SMOKE.SmokeError, "JSON-RPC version"):
                    SMOKE.validate_mcp_responses(responses)

    def test_errors_missing_ids_and_duplicate_ids_fail_closed(self) -> None:
        error = valid_responses()
        error[2] = {"jsonrpc": "2.0", "id": 3, "error": {"code": -1}}
        with self.assertRaisesRegex(SMOKE.SmokeError, "is an error"):
            SMOKE.validate_mcp_responses(error)

        missing = valid_responses()[:2]
        with self.assertRaisesRegex(SMOKE.SmokeError, "ids do not match"):
            SMOKE.validate_mcp_responses(missing)

        duplicate = valid_responses()
        duplicate[2]["id"] = 2
        with self.assertRaisesRegex(SMOKE.SmokeError, "duplicated"):
            SMOKE.validate_mcp_responses(duplicate)

    def test_output_parser_rejects_malformed_or_nonobject_json(self) -> None:
        encoded = "".join(f"{json.dumps(response)}\n" for response in valid_responses())
        self.assertEqual(len(SMOKE.parse_mcp_output(encoded)), 3)
        with self.assertRaisesRegex(SMOKE.SmokeError, "malformed JSON"):
            SMOKE.parse_mcp_output("{bad}\n")
        with self.assertRaisesRegex(SMOKE.SmokeError, "not a JSON object"):
            SMOKE.parse_mcp_output("[]\n")

    def test_process_runner_enforces_timeout_and_output_limit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with mock.patch.object(SMOKE, "PROCESS_TIMEOUT_SECONDS", 0.05):
                with self.assertRaisesRegex(SMOKE.SmokeError, "exceeded"):
                    SMOKE.run_process(
                        [sys.executable, "-c", "import time; time.sleep(1)"],
                        cwd=root,
                        environment=dict(os.environ),
                    )
            with mock.patch.object(SMOKE, "MAX_OUTPUT_BYTES", 10):
                with self.assertRaisesRegex(SMOKE.SmokeError, "output limit"):
                    SMOKE.run_process(
                        [sys.executable, "-c", "print('x' * 20)"],
                        cwd=root,
                        environment=dict(os.environ),
                    )
            with mock.patch.object(SMOKE, "MAX_OUTPUT_BYTES", 4_096):
                with self.assertRaisesRegex(SMOKE.SmokeError, "output limit"):
                    SMOKE.run_process(
                        [
                            sys.executable,
                            "-u",
                            "-c",
                            (
                                "import os,sys\n"
                                "chunk=b'x'*65536\n"
                                "while True: os.write(sys.stdout.fileno(),chunk)"
                            ),
                        ],
                        cwd=root,
                        environment=dict(os.environ),
                    )

    def test_timeout_is_bounded_when_a_descendant_holds_output_pipes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            child_pid_path = root / "child.pid"
            program = (
                "import pathlib,subprocess,sys,tempfile,time\n"
                "child=subprocess.Popen([sys.executable,'-c',"
                "'import time;time.sleep(10)'],cwd=tempfile.gettempdir())\n"
                f"pathlib.Path({str(child_pid_path)!r}).write_text(str(child.pid))\n"
                "time.sleep(10)"
            )
            started = time.monotonic()
            try:
                with mock.patch.object(SMOKE, "PROCESS_TIMEOUT_SECONDS", 2):
                    with self.assertRaisesRegex(SMOKE.SmokeError, "exceeded"):
                        SMOKE.run_process(
                            [sys.executable, "-c", program],
                            cwd=root,
                            environment=dict(os.environ),
                        )
                self.assertTrue(child_pid_path.exists())
                self.assertLess(time.monotonic() - started, 4.5)
            finally:
                if child_pid_path.exists():
                    child_pid = int(child_pid_path.read_text())
                    terminate_process(child_pid)

    def test_isolated_environment_confines_all_player_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "profile"
            environment = SMOKE.isolated_environment(root)
            for name in (
                "NUMINOUS_JOURNEY",
                "NUMINOUS_SCORES",
                "NUMINOUS_CAIRN",
                "NUMINOUS_JOURNAL",
            ):
                self.assertEqual(Path(environment[name]).parent, root)
            self.assertEqual(Path(environment["HOME"]), root)
            self.assertEqual(Path(environment["USERPROFILE"]), root)
            self.assertEqual(Path(environment["LOCALAPPDATA"]).parent, root)
            self.assertEqual(Path(environment["XDG_CONFIG_HOME"]).parent, root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
