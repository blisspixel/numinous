#!/usr/bin/env python3
"""Validate Numinous's pinned portable Agent Plugins package."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PLUGIN_ROOT = ROOT / "plugins" / "numinous"
PLUGIN_SCHEMA = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"
MCP_SCHEMA = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json"
NAME_RE = re.compile(r"^(?!.*(?:--|\.\.))[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$")
SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
PLUGIN_FIELDS = {
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
}
AUTHOR_FIELDS = {"name", "email", "url"}
SERVER_FIELDS = {"type", "command", "args", "env", "cwd"}
SKILL_FIELDS = {
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
}
EXPECTED_PLUGIN_FILES = {
    "plugin.json",
    "mcp.json",
    "skills/play-numinous/SKILL.md",
}


class PluginValidationError(ValueError):
    """A portable package violates the pinned contract."""


def object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build one JSON object while refusing ambiguous duplicate fields."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PluginValidationError(f"JSON repeats field {key!r}")
        result[key] = value
    return result


def read_json(path: Path) -> dict[str, Any]:
    """Read one bounded ordinary JSON object without duplicate fields."""
    if not path.is_file() or path.is_symlink():
        raise PluginValidationError(f"missing ordinary file: {path}")
    if path.stat().st_size > 64 * 1024:
        raise PluginValidationError(f"JSON file exceeds 64 KiB: {path}")
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=object_without_duplicates
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise PluginValidationError(f"malformed JSON: {path}") from error
    if not isinstance(value, dict):
        raise PluginValidationError(f"JSON root is not an object: {path}")
    return value


def workspace_version(root: Path = ROOT) -> str:
    """Read the release version from the workspace package table."""
    manifest = root / "Cargo.toml"
    in_workspace_package = False
    for line in manifest.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_workspace_package = stripped == "[workspace.package]"
        elif in_workspace_package:
            match = re.fullmatch(r'version\s*=\s*"([^"]+)"', stripped)
            if match:
                version = match.group(1)
                if VERSION_RE.fullmatch(version) is None:
                    raise PluginValidationError("workspace version is malformed")
                return version
    raise PluginValidationError("Cargo.toml has no workspace package version")


def require_string(value: Any, label: str, maximum: int | None = None) -> str:
    """Require one nonempty string with an optional character cap."""
    if not isinstance(value, str) or not value:
        raise PluginValidationError(f"{label} must be a nonempty string")
    if maximum is not None and len(value) > maximum:
        raise PluginValidationError(f"{label} exceeds {maximum} characters")
    return value


def validate_manifest(manifest: dict[str, Any], expected_version: str) -> None:
    """Validate the public Agent Plugins v1 manifest plus Numinous identity."""
    unknown = set(manifest) - PLUGIN_FIELDS
    if unknown:
        raise PluginValidationError(
            f"plugin.json has unsupported field {sorted(unknown)[0]!r}"
        )
    if manifest.get("$schema") != PLUGIN_SCHEMA:
        raise PluginValidationError("plugin.json does not pin Agent Plugins 1.0.0")
    name = require_string(manifest.get("name"), "plugin name", 64)
    if NAME_RE.fullmatch(name) is None or name != "numinous":
        raise PluginValidationError("plugin name is not the canonical 'numinous' name")
    if manifest.get("version") != expected_version:
        raise PluginValidationError("plugin version does not match the workspace release")
    require_string(manifest.get("description"), "plugin description")
    author = manifest.get("author")
    if not isinstance(author, dict) or not author or set(author) - AUTHOR_FIELDS:
        raise PluginValidationError("plugin author object is malformed")
    for field, value in author.items():
        require_string(value, f"plugin author {field}")
    for field in ("homepage", "repository", "license"):
        require_string(manifest.get(field), f"plugin {field}")
    if manifest["repository"] != "https://github.com/blisspixel/numinous":
        raise PluginValidationError("plugin repository is not canonical")
    if manifest["license"] != "Apache-2.0":
        raise PluginValidationError("plugin license must match the repository")
    keywords = manifest.get("keywords")
    if not isinstance(keywords, list) or not keywords:
        raise PluginValidationError("plugin keywords must be a nonempty string list")
    if any(not isinstance(keyword, str) or not keyword for keyword in keywords):
        raise PluginValidationError("plugin keywords contain a non-string or empty value")
    extensions = manifest.get("extensions")
    if extensions is not None and (
        not isinstance(extensions, dict)
        or any(not isinstance(value, dict) for value in extensions.values())
    ):
        raise PluginValidationError("plugin extensions must map names to objects")


def validate_mcp(configuration: dict[str, Any]) -> None:
    """Validate the pinned MCP configuration and its single stdio entry."""
    if set(configuration) != {"$schema", "mcpServers"}:
        raise PluginValidationError("mcp.json must contain only schema and servers")
    if configuration["$schema"] != MCP_SCHEMA:
        raise PluginValidationError("mcp.json does not pin Agent Plugins 1.0.0")
    servers = configuration["mcpServers"]
    if not isinstance(servers, dict) or set(servers) != {"numinous"}:
        raise PluginValidationError("mcp.json must contain exactly one Numinous server")
    server = servers["numinous"]
    if not isinstance(server, dict) or set(server) - SERVER_FIELDS:
        raise PluginValidationError("Numinous MCP server entry is malformed")
    if server != {"type": "stdio", "command": "numinous-mcp"}:
        raise PluginValidationError(
            "Numinous MCP must launch the installed binary as one bare token"
        )


def parse_skill_frontmatter(path: Path) -> tuple[dict[str, Any], str]:
    """Parse the deliberately small Agent Skills frontmatter subset in use."""
    if not path.is_file() or path.is_symlink():
        raise PluginValidationError(f"missing ordinary skill file: {path}")
    if path.stat().st_size > 64 * 1024:
        raise PluginValidationError("SKILL.md exceeds 64 KiB")
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise PluginValidationError("SKILL.md has no opening frontmatter delimiter")
    try:
        closing = lines.index("---", 1)
    except ValueError as error:
        raise PluginValidationError("SKILL.md has no closing frontmatter delimiter") from error
    fields: dict[str, Any] = {}
    current_mapping: str | None = None
    for line in lines[1:closing]:
        if not line.strip():
            continue
        if line.startswith("  "):
            if current_mapping != "metadata" or ":" not in line:
                raise PluginValidationError("SKILL.md has unsupported nested frontmatter")
            key, raw = line.strip().split(":", maxsplit=1)
            metadata = fields.setdefault("metadata", {})
            if key in metadata:
                raise PluginValidationError(f"SKILL.md repeats metadata field {key!r}")
            metadata[key] = raw.strip().strip('"')
            continue
        current_mapping = None
        if ":" not in line:
            raise PluginValidationError("SKILL.md has malformed frontmatter")
        key, raw = line.split(":", maxsplit=1)
        if key in fields:
            raise PluginValidationError(f"SKILL.md repeats field {key!r}")
        if key == "metadata" and not raw.strip():
            fields[key] = {}
            current_mapping = key
        else:
            fields[key] = raw.strip().strip('"')
    return fields, "\n".join(lines[closing + 1 :]).strip()


def validate_skill(path: Path) -> None:
    """Validate one Agent Skills document and Numinous's player-first posture."""
    fields, body = parse_skill_frontmatter(path)
    unknown = set(fields) - SKILL_FIELDS
    if unknown:
        raise PluginValidationError(
            f"SKILL.md has unsupported field {sorted(unknown)[0]!r}"
        )
    name = require_string(fields.get("name"), "skill name", 64)
    if name != path.parent.name or SKILL_NAME_RE.fullmatch(name) is None:
        raise PluginValidationError("skill name must match its lowercase directory")
    description = require_string(fields.get("description"), "skill description", 1024)
    if "player" not in description.lower() or "watch agent" not in description.lower():
        raise PluginValidationError("skill description must name play and Watch Agent")
    if fields.get("license") != "Apache-2.0":
        raise PluginValidationError("skill license must match the repository")
    compatibility = fields.get("compatibility")
    if compatibility is not None:
        require_string(compatibility, "skill compatibility", 500)
    metadata = fields.get("metadata")
    if metadata is not None and (
        not isinstance(metadata, dict)
        or any(
            not isinstance(key, str) or not isinstance(value, str)
            for key, value in metadata.items()
        )
    ):
        raise PluginValidationError("skill metadata must map strings to strings")
    if not body:
        raise PluginValidationError("SKILL.md instructions are empty")
    if len(body.splitlines()) > 500:
        raise PluginValidationError("SKILL.md instructions exceed 500 lines")
    for required in (
        "list_rooms",
        "play_room",
        "reveal_room",
        "broadcast_session",
        "prompts, private reasoning",
    ):
        if required not in body:
            raise PluginValidationError(f"SKILL.md omits required boundary {required!r}")


def validate_package(plugin_root: Path, expected_version: str | None = None) -> None:
    """Validate every portable component and refuse an ambiguous package root."""
    if not plugin_root.is_dir() or plugin_root.is_symlink():
        raise PluginValidationError(f"plugin root is not an ordinary directory: {plugin_root}")
    resolved_root = plugin_root.resolve()
    actual_files: set[str] = set()
    for path in plugin_root.rglob("*"):
        if path.is_symlink():
            raise PluginValidationError(f"plugin package contains a symlink: {path}")
        if path.is_file():
            try:
                relative = path.resolve().relative_to(resolved_root).as_posix()
            except ValueError as error:
                raise PluginValidationError(f"plugin path escapes its root: {path}") from error
            actual_files.add(relative)
    if actual_files != EXPECTED_PLUGIN_FILES:
        missing = sorted(EXPECTED_PLUGIN_FILES - actual_files)
        extra = sorted(actual_files - EXPECTED_PLUGIN_FILES)
        detail = f"missing {missing[0]!r}" if missing else f"unexpected {extra[0]!r}"
        raise PluginValidationError(f"plugin package inventory differs: {detail}")
    version = expected_version or workspace_version()
    validate_manifest(read_json(plugin_root / "plugin.json"), version)
    validate_mcp(read_json(plugin_root / "mcp.json"))
    validate_skill(plugin_root / "skills" / "play-numinous" / "SKILL.md")


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plugin_root", nargs="?", type=Path, default=DEFAULT_PLUGIN_ROOT)
    parser.add_argument("--expected-version")
    args = parser.parse_args()
    try:
        validate_package(args.plugin_root, args.expected_version)
    except (OSError, PluginValidationError) as error:
        parser.error(str(error))
    print(f"Agent Plugins package valid: {args.plugin_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
