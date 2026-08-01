#!/usr/bin/env python3
"""Regression tests for the disposable MCP playtest driver."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import stat
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
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def initialize_repository(root: Path) -> str:
    """Create one committed source fixture and return its full revision."""
    (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
    commands = (
        ["git", "init", "--quiet"],
        ["git", "add", "Cargo.toml"],
        [
            "git",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "Initialize fixture",
        ],
    )
    for command in commands:
        subprocess.run(command, cwd=root, check=True, capture_output=True)
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def fake_artifact(driver, path: Path):
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return driver.BuiltArtifact(
        path=path,
        sha256=digest,
        receipt={"binarySha256": digest},
        owner=None,
    )


class McpPlayLifecycleTests(unittest.TestCase):
    def test_cargo_artifact_uses_explicit_fresh_target_and_json_path(self) -> None:
        driver = load_driver()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target_dir = root / "fresh-target"
            executable = target_dir / "fixture" / "debug" / "numinous-mcp"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"fresh executable")
            message = {
                "reason": "compiler-artifact",
                "target": {"name": "numinous-mcp", "kind": ["bin"]},
                "executable": str(executable),
            }
            completed = subprocess.CompletedProcess(
                args=["cargo"], returncode=0, stdout=json.dumps(message), stderr=""
            )
            with mock.patch.object(
                driver.subprocess, "run", return_value=completed
            ) as run:
                observed = driver._cargo_artifact(target_dir, "fixture-target", {})
        self.assertEqual(observed, executable.resolve())
        command = run.call_args.args[0]
        self.assertEqual(command[command.index("--target") + 1], "fixture-target")
        self.assertEqual(command[command.index("--target-dir") + 1], str(target_dir))
        self.assertIn("--no-default-features", command)
        self.assertIn("--message-format=json-render-diagnostics", command)

    def test_qualifying_source_rejects_untracked_cargo_configuration(self) -> None:
        driver = load_driver()
        revision = "1" * 40
        with (
            mock.patch.object(
                driver.source_integrity,
                "verify_source_tree",
                side_effect=driver.source_integrity.SourceIntegrityError(
                    "qualifying runtime source worktree is dirty"
                ),
            ),
            self.assertRaisesRegex(driver.McpPlayError, "worktree is dirty"),
        ):
            driver._require_qualifying_source(revision, "2" * 64, {})

    def test_qualifying_source_rejects_ignored_cargo_configuration(self) -> None:
        driver = load_driver()
        revision = "1" * 40
        with (
            mock.patch.object(
                driver.source_integrity,
                "verify_source_tree",
                side_effect=driver.source_integrity.SourceIntegrityError(
                    "qualifying runtime source contains ignored files"
                ),
            ),
            self.assertRaisesRegex(driver.McpPlayError, "contains ignored files"),
        ):
            driver._require_qualifying_source(revision, "2" * 64, {})

    def test_qualifying_source_rejects_hidden_index_changes(self) -> None:
        driver = load_driver()
        for flag in ("--assume-unchanged", "--skip-worktree"):
            with self.subTest(flag=flag), tempfile.TemporaryDirectory() as temporary:
                repository = Path(temporary)
                revision = initialize_repository(repository)
                subprocess.run(
                    ["git", "update-index", flag, "Cargo.toml"],
                    cwd=repository,
                    check=True,
                    capture_output=True,
                )
                (repository / "Cargo.toml").write_text(
                    "[workspace]\nmembers = []\n", encoding="utf-8"
                )
                with (
                    mock.patch.object(driver, "ROOT", repository),
                    mock.patch.object(
                        driver, "QUALIFYING_SOURCE_PATHS", ("Cargo.toml",)
                    ),
                    self.assertRaisesRegex(
                        driver.McpPlayError, "nonordinary index flags"
                    ),
                ):
                    driver._require_qualifying_source(revision, "2" * 64, {})

    def test_qualifying_build_rejects_ancestor_and_cargo_home_configuration(self) -> None:
        driver = load_driver()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "workspace" / "repository"
            repository.mkdir(parents=True)
            ancestor_config = root / "workspace" / ".cargo" / "config.toml"
            ancestor_config.parent.mkdir()
            ancestor_config.write_text("[build]\nrustflags = []\n", encoding="utf-8")
            cargo_home = root / "cargo-home"
            cargo_home.mkdir()
            with mock.patch.object(driver, "ROOT", repository):
                self.assertTrue(
                    driver._has_unbound_cargo_configuration(
                        {"CARGO_HOME": str(cargo_home)}
                    )
                )
            ancestor_config.unlink()
            cargo_home_config = cargo_home / "config"
            cargo_home_config.write_text("[net]\noffline = true\n", encoding="utf-8")
            with mock.patch.object(driver, "ROOT", repository):
                self.assertTrue(
                    driver._has_unbound_cargo_configuration(
                        {"CARGO_HOME": str(cargo_home)}
                    )
                )
            cargo_home_config.unlink()
            with mock.patch.object(driver, "ROOT", repository):
                self.assertFalse(
                    driver._has_unbound_cargo_configuration(
                        {"CARGO_HOME": str(cargo_home)}
                    )
                )

    def test_build_freezes_exact_cargo_artifact_away_from_stale_default(self) -> None:
        driver = load_driver()
        revision = "1" * 40
        source_digest = "2" * 64
        built_path: list[Path] = []

        def build(target_dir: Path, _target: str, _env: dict[str, str]) -> Path:
            artifact = target_dir / "fixture-target" / "debug" / "numinous-mcp"
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"fresh private executable")
            built_path.append(artifact)
            return artifact

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stale = root / "target" / "debug" / "numinous-mcp"
            stale.parent.mkdir(parents=True)
            stale.write_bytes(b"stale default executable")
            with (
                mock.patch.object(driver, "ROOT", root),
                mock.patch.object(driver, "_build_environment", return_value={}),
                mock.patch.object(driver, "_require_qualifying_source"),
                mock.patch.object(
                    driver, "_has_unbound_cargo_configuration", return_value=False
                ),
                mock.patch.object(driver, "_git_output", return_value=revision),
                mock.patch.object(
                    driver,
                    "_toolchain_metadata",
                    return_value=("cargo fixture", "rustc fixture", "fixture-target"),
                ),
                mock.patch.object(driver, "_cargo_artifact", side_effect=build),
            ):
                artifact = driver._build_artifact(revision, source_digest)
            built_path[0].chmod(stat.S_IWRITE | stat.S_IREAD)
            built_path[0].write_bytes(b"replaced compiler output")
            self.assertEqual(artifact.path.read_bytes(), b"fresh private executable")
            self.assertNotEqual(artifact.path, stale)
            self.assertEqual(artifact.receipt["sourceRevision"], revision)
            self.assertEqual(artifact.receipt["studySourceSha256"], source_digest)
            artifact.owner.cleanup()

    def test_session_detects_concurrent_private_artifact_replacement(self) -> None:
        driver = load_driver()
        request = {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "server"
            binary.write_bytes(b"original executable")
            artifact = fake_artifact(driver, binary)

            def replace_during_run(*_args, **kwargs):
                binary.write_bytes(b"replacement executable")
                response = {"jsonrpc": "2.0", "id": 1, "result": {}}
                kwargs["stdout"].write((json.dumps(response) + "\n").encode())
                return subprocess.CompletedProcess(args=[str(binary)], returncode=0)

            with (
                mock.patch.object(driver, "_binary", return_value=artifact),
                mock.patch.object(
                    driver.subprocess, "run", side_effect=replace_during_run
                ),
                self.assertRaisesRegex(driver.McpPlayError, "changed during execution"),
            ):
                driver._session([request])

    def test_concurrent_sessions_own_distinct_profiles_and_clean_them(self) -> None:
        driver = load_driver()
        workers = 8
        barrier = threading.Barrier(workers)
        lock = threading.Lock()
        captured: list[tuple[Path, Path, Path, Path]] = []

        def fake_run(*_args, **kwargs):
            env = kwargs["env"]
            paths = tuple(
                Path(env[name])
                for name in (
                    "NUMINOUS_JOURNEY",
                    "NUMINOUS_SCORES",
                    "NUMINOUS_CAIRN",
                    "NUMINOUS_JOURNAL",
                )
            )
            self.assertEqual(len({path.parent for path in paths}), 1)
            self.assertEqual(Path(env["HOME"]), paths[0].parent)
            self.assertEqual(Path(env["USERPROFILE"]), paths[0].parent)
            for path in paths:
                path.write_text("isolated", encoding="utf-8")
            (paths[0].parent / ".numinous-radio").mkdir()
            (paths[0].parent / ".numinous-crash.log").write_text(
                "isolated", encoding="utf-8"
            )
            with lock:
                captured.append(paths)
            barrier.wait(timeout=5)
            request = json.loads(kwargs["input"].decode("utf-8").strip())
            response = {
                "jsonrpc": "2.0",
                "id": request["id"],
                "result": {},
            }
            kwargs["stdout"].write((json.dumps(response) + "\n").encode("utf-8"))
            return subprocess.CompletedProcess(
                args=["fake-server"],
                returncode=0,
            )

        request = {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        with tempfile.TemporaryDirectory() as binary_root:
            binary = Path(binary_root) / "fake-server"
            binary.write_bytes(b"stable fake server")
            with mock.patch.object(
                driver, "_binary", return_value=fake_artifact(driver, binary)
            ):
                with mock.patch.object(driver.subprocess, "run", side_effect=fake_run):
                    with ThreadPoolExecutor(max_workers=workers) as executor:
                        results = list(
                            executor.map(
                                lambda _index: driver._session([request]), range(workers)
                            )
                        )

        self.assertEqual(len(results), workers)
        profile_roots = {paths[0].parent for paths in captured}
        self.assertEqual(len(profile_roots), workers)
        self.assertTrue(all(not root.exists() for root in profile_roots))

    def test_session_rejects_oversized_input_before_building(self) -> None:
        driver = load_driver()
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"value": "x" * driver.MAX_REQUEST_LINE_BYTES},
        }
        with mock.patch.object(driver, "_binary") as binary:
            with self.assertRaisesRegex(driver.McpPlayError, "request 1 exceeds"):
                driver._session([request])
        binary.assert_not_called()

    def test_session_rejects_excess_output_before_decoding(self) -> None:
        driver = load_driver()

        def fake_run(*_args, **kwargs):
            kwargs["stdout"].write(b"x" * (driver.MAX_RESPONSE_LINE_BYTES + 2))
            return subprocess.CompletedProcess(args=["fake-server"], returncode=0)

        request = {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        with tempfile.TemporaryDirectory() as binary_root:
            binary = Path(binary_root) / "fake-server"
            binary.write_bytes(b"stable fake server")
            with mock.patch.object(
                driver, "_binary", return_value=fake_artifact(driver, binary)
            ):
                with mock.patch.object(driver.subprocess, "run", side_effect=fake_run):
                    with self.assertRaisesRegex(
                        driver.McpPlayError, "session output exceeds"
                    ):
                        driver._session([request])


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
        self.assertIn("35 tools.", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
