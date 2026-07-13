#!/usr/bin/env python3
"""Regression tests for the disposable MCP playtest driver."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
DRIVER = ROOT / "scripts" / "mcp-play.py"


def load_driver():
    spec = importlib.util.spec_from_file_location("numinous_mcp_play", DRIVER)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load mcp-play.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class McpPlayLifecycleTests(unittest.TestCase):
    def test_concurrent_sessions_own_distinct_profiles_and_clean_them(self) -> None:
        driver = load_driver()
        workers = 8
        barrier = threading.Barrier(workers)
        lock = threading.Lock()
        captured: list[tuple[Path, Path, Path]] = []

        def fake_run(*_args, **kwargs):
            env = kwargs["env"]
            paths = tuple(
                Path(env[name])
                for name in (
                    "NUMINOUS_JOURNEY",
                    "NUMINOUS_SCORES",
                    "NUMINOUS_CAIRN",
                )
            )
            self.assertEqual(len({path.parent for path in paths}), 1)
            for path in paths:
                path.write_text("isolated", encoding="utf-8")
            with lock:
                captured.append(paths)
            barrier.wait(timeout=5)
            request = json.loads(kwargs["input"].strip())
            response = {
                "jsonrpc": "2.0",
                "id": request["id"],
                "result": {},
            }
            return subprocess.CompletedProcess(
                args=["fake-server"],
                returncode=0,
                stdout=json.dumps(response) + "\n",
                stderr="",
            )

        request = {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        with mock.patch.object(driver, "_binary", return_value="fake-server"):
            with mock.patch.object(driver.subprocess, "run", side_effect=fake_run):
                with ThreadPoolExecutor(max_workers=workers) as executor:
                    results = list(
                        executor.map(lambda _index: driver._session([request]), range(workers))
                    )

        self.assertEqual(len(results), workers)
        profile_roots = {paths[0].parent for paths in captured}
        self.assertEqual(len(profile_roots), workers)
        self.assertTrue(all(not root.exists() for root in profile_roots))


class McpPlayCommandTests(unittest.TestCase):
    @staticmethod
    def run_driver(
        *arguments: str, env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(DRIVER), *arguments],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )

    def test_help_is_useful(self) -> None:
        result = self.run_driver("--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("automatically cleaned test profile", result.stdout)
        self.assertIn("python scripts/mcp-play.py call play_room", result.stdout)
        self.assertIn("complete description", result.stdout)

    def test_unknown_tool_is_readable_and_nonzero(self) -> None:
        result = self.run_driver("call", "not_a_tool", "{}")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unknown tool: not_a_tool", result.stderr)

    def test_tool_error_is_readable_and_nonzero(self) -> None:
        arguments = json.dumps({"id": "no-such-room"})
        with tempfile.TemporaryDirectory() as temp_root:
            env = dict(os.environ)
            env.update({"TEMP": temp_root, "TMP": temp_root, "TMPDIR": temp_root})
            result = self.run_driver("call", "play_room", arguments, env=env)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("No room with id 'no-such-room'", result.stderr)
            self.assertEqual(list(Path(temp_root).iterdir()), [])

    def test_json_can_be_read_from_stdin_without_shell_escaping(self) -> None:
        result = subprocess.run(
            [sys.executable, str(DRIVER), "call", "describe_room", "-"],
            cwd=ROOT,
            input=json.dumps({"id": "cult-of-pi"}),
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('"room": "cult-of-pi"', result.stdout)

    def test_tool_descriptions_are_not_truncated(self) -> None:
        result = self.run_driver("tools")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("totals.", result.stdout)
        self.assertIn("29 tools.", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
