#!/usr/bin/env python3
"""Regression tests for the portable Agent Plugins package."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_agent_plugin", ROOT / "scripts" / "validate-agent-plugin.py"
)
assert SPEC is not None and SPEC.loader is not None
PLUGIN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLUGIN)


def copy_package(destination: Path) -> None:
    """Copy the three-file package without importing a recursive copy helper."""
    for relative in PLUGIN.EXPECTED_PLUGIN_FILES:
        source = PLUGIN.DEFAULT_PLUGIN_ROOT / relative
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source.read_bytes())


class AgentPluginTests(unittest.TestCase):
    def test_repository_package_is_valid_and_release_locked(self) -> None:
        PLUGIN.validate_package(PLUGIN.DEFAULT_PLUGIN_ROOT)

    def test_manifest_rejects_unknown_duplicate_and_stale_fields(self) -> None:
        version = PLUGIN.workspace_version()
        manifest = PLUGIN.read_json(PLUGIN.DEFAULT_PLUGIN_ROOT / "plugin.json")
        manifest["unknown"] = True
        with self.assertRaisesRegex(PLUGIN.PluginValidationError, "unsupported field"):
            PLUGIN.validate_manifest(manifest, version)

        stale = PLUGIN.read_json(PLUGIN.DEFAULT_PLUGIN_ROOT / "plugin.json")
        stale["version"] = "9.9.9"
        with self.assertRaisesRegex(PLUGIN.PluginValidationError, "workspace release"):
            PLUGIN.validate_manifest(stale, version)

        with tempfile.TemporaryDirectory() as temporary:
            duplicate = Path(temporary) / "plugin.json"
            duplicate.write_text('{"name":"one","name":"two"}', encoding="utf-8")
            with self.assertRaisesRegex(PLUGIN.PluginValidationError, "repeats field"):
                PLUGIN.read_json(duplicate)

    def test_mcp_entry_is_one_executable_token_with_no_hidden_shell(self) -> None:
        configuration = PLUGIN.read_json(PLUGIN.DEFAULT_PLUGIN_ROOT / "mcp.json")
        PLUGIN.validate_mcp(configuration)
        for command in (
            "numinous-mcp --flag",
            "sh -c numinous-mcp",
            "../numinous-mcp",
        ):
            mutated = json.loads(json.dumps(configuration))
            mutated["mcpServers"]["numinous"]["command"] = command
            with self.subTest(command=command):
                with self.assertRaisesRegex(
                    PLUGIN.PluginValidationError, "one bare token"
                ):
                    PLUGIN.validate_mcp(mutated)

    def test_skill_identity_and_privacy_boundary_are_required(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "numinous"
            copy_package(package)
            skill = package / "skills" / "play-numinous" / "SKILL.md"
            skill.write_text(
                skill.read_text(encoding="utf-8").replace(
                    "prompts, private reasoning", "everything"
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PLUGIN.PluginValidationError, "required boundary"):
                PLUGIN.validate_package(package, PLUGIN.workspace_version())

            skill.write_bytes(
                (PLUGIN.DEFAULT_PLUGIN_ROOT / "skills/play-numinous/SKILL.md").read_bytes()
            )
            renamed = package / "skills" / "renamed"
            renamed.parent.mkdir(parents=True, exist_ok=True)
            (package / "skills" / "play-numinous").rename(renamed)
            with self.assertRaisesRegex(PLUGIN.PluginValidationError, "inventory"):
                PLUGIN.validate_package(package, PLUGIN.workspace_version())

    def test_package_inventory_is_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "numinous"
            copy_package(package)
            (package / "surprise.txt").write_text("not declared", encoding="utf-8")
            with self.assertRaisesRegex(PLUGIN.PluginValidationError, "inventory"):
                PLUGIN.validate_package(package, PLUGIN.workspace_version())


if __name__ == "__main__":
    unittest.main(verbosity=2)
