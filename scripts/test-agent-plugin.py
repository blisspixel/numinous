#!/usr/bin/env python3
"""Regression tests for the portable Agent Plugins package."""

from __future__ import annotations

import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


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
            with self.assertRaisesRegex(
                PLUGIN.PluginValidationError, "required boundary"
            ):
                PLUGIN.validate_package(package, PLUGIN.workspace_version())

            skill.write_bytes(
                (
                    PLUGIN.DEFAULT_PLUGIN_ROOT / "skills/play-numinous/SKILL.md"
                ).read_bytes()
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

    def test_canonical_schemas_accept_standard_variants_and_reject_bad_types(
        self,
    ) -> None:
        # Standard validity and this product's stricter package posture differ.
        PLUGIN.validate_schema(
            {"$schema": PLUGIN.PLUGIN_SCHEMA, "name": "other-plugin"},
            "plugin.schema.json",
        )
        valid = {
            "$schema": PLUGIN.MCP_SCHEMA,
            "mcpServers": {
                "remote": {
                    "type": "streamable-http",
                    "url": "https://example.com/mcp",
                    "headers": {"Authorization": "test-token"},
                }
            },
        }
        PLUGIN.validate_schema(valid, "mcp.schema.json")
        for server in (
            {"type": "stdio", "command": 42},
            {"type": "stdio", "command": "server", "env": {"PLUGIN_ROOT": "shadow"}},
            {"type": "stdio", "command": "server", "cwd": "/ambient/root"},
            {
                "type": "streamable-http",
                "url": "https://example.com",
                "headers": {"X": 1},
            },
        ):
            with self.subTest(server=server):
                with self.assertRaisesRegex(PLUGIN.PluginValidationError, "canonical"):
                    PLUGIN.validate_schema(
                        {"$schema": PLUGIN.MCP_SCHEMA, "mcpServers": {"other": server}},
                        "mcp.schema.json",
                    )

    def test_schema_fixture_drift_fails_without_fetching_a_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixtures = Path(temporary)
            for name in ("provenance.json", "plugin.schema.json"):
                (fixtures / name).write_bytes((PLUGIN.FIXTURE_ROOT / name).read_bytes())
            with (fixtures / "plugin.schema.json").open("ab") as output:
                output.write(b"\n")
            with patch.object(PLUGIN, "FIXTURE_ROOT", fixtures):
                with self.assertRaisesRegex(
                    PLUGIN.PluginValidationError, "digest differs"
                ):
                    PLUGIN.validate_schema({}, "plugin.schema.json")

    def test_external_schema_reference_fails_without_outbound_retrieval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixtures = Path(temporary)
            provenance = PLUGIN.read_json(PLUGIN.FIXTURE_ROOT / "provenance.json")
            schema = PLUGIN.read_json(PLUGIN.FIXTURE_ROOT / "plugin.schema.json")
            schema["$ref"] = "https://example.invalid/unavailable-schema.json"
            encoded = json.dumps(schema).encode("utf-8")
            (fixtures / "plugin.schema.json").write_bytes(encoded)
            for artifact in provenance["artifacts"]:
                if artifact.get("path") == "plugin.schema.json":
                    artifact["sha256"] = hashlib.sha256(encoded).hexdigest()
            (fixtures / "provenance.json").write_text(
                json.dumps(provenance), encoding="utf-8"
            )
            with (
                patch.object(PLUGIN, "FIXTURE_ROOT", fixtures),
                patch(
                    "urllib.request.urlopen",
                    side_effect=AssertionError("outbound retrieval"),
                ) as retrieve,
            ):
                with self.assertRaisesRegex(
                    PLUGIN.PluginValidationError, "offline reference"
                ):
                    PLUGIN.validate_schema({}, "plugin.schema.json")
                retrieve.assert_not_called()

    def test_yaml_recovers_multiline_and_quoted_values_and_rejects_ambiguous_input(
        self,
    ) -> None:
        fields, body = PLUGIN.parse_frontmatter(
            "---\nname: play-numinous\ndescription: >-\n  Enter as a player:\n"
            "  choose a Watch Agent session.\nmetadata:\n  version: '0.2'\n"
            '  note: "quote: \\"yes\\""\n---\nInstructions.\n',
            "test skill",
        )
        self.assertEqual(
            fields["description"], "Enter as a player: choose a Watch Agent session."
        )
        self.assertEqual(fields["metadata"], {"version": "0.2", "note": 'quote: "yes"'})
        self.assertEqual(body, "Instructions.")
        for document in (
            "name: first\nname: second",
            "metadata:\n  name: first\n  name: second",
            "description: invalid: colon",
            "name: &shared value\ndescription: *shared",
            "name: !!python/object/apply:os.system ['invalid']",
            "[name, description]",
            "metadata: {true: value}",
        ):
            with self.subTest(document=document):
                with self.assertRaises(PLUGIN.PluginValidationError):
                    PLUGIN.parse_frontmatter(f"---\n{document}\n---\nBody", "test")

    def test_reference_skill_validator_is_required(self) -> None:
        import subprocess

        failed = subprocess.CompletedProcess(
            ["reference"], 1, "reference rejection", ""
        )
        with patch.object(PLUGIN.subprocess, "run", return_value=failed):
            with self.assertRaisesRegex(
                PLUGIN.PluginValidationError, "reference validation failed"
            ):
                PLUGIN.validate_skill(
                    PLUGIN.DEFAULT_PLUGIN_ROOT / "skills/play-numinous/SKILL.md"
                )
        with patch.object(
            PLUGIN.subprocess,
            "run",
            side_effect=subprocess.TimeoutExpired("reference", 15),
        ):
            with self.assertRaisesRegex(
                PLUGIN.PluginValidationError, "could not complete"
            ):
                PLUGIN.validate_skill(
                    PLUGIN.DEFAULT_PLUGIN_ROOT / "skills/play-numinous/SKILL.md"
                )

    def test_author_identity_is_canonical(self) -> None:
        manifest = PLUGIN.read_json(PLUGIN.DEFAULT_PLUGIN_ROOT / "plugin.json")
        manifest["author"] = {"name": "Someone else"}
        with self.assertRaisesRegex(
            PLUGIN.PluginValidationError, "repository identity"
        ):
            PLUGIN.validate_manifest(manifest, PLUGIN.workspace_version())


if __name__ == "__main__":
    unittest.main(verbosity=2)
