#!/usr/bin/env python3
"""Check portable formats against pinned standards and live production exports.

This is an offline format conformance gate. It does not claim that an external
agent host has discovered the plugin or selected its skill.
"""

from __future__ import annotations

import argparse
from datetime import date
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile
from typing import Any

from gate_cli import build_and_locate

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_interoperability_plugin", ROOT / "scripts" / "validate-agent-plugin.py"
)
assert SPEC is not None and SPEC.loader is not None
PLUGIN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLUGIN)
MAX_SESSION_BYTES = 2 * 1024 * 1024


class ConformanceError(ValueError):
    """A producer or transport response violates its declared portable boundary."""


def validate_okf(files: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    """Check the OKF v0.2 required floor, without rejecting optional extensions."""
    concepts: dict[str, dict[str, Any]] = {}
    paths: set[str] = set()
    for file in files:
        path, content = file.get("path"), file.get("content")
        if not isinstance(path, str) or not isinstance(content, str):
            raise ConformanceError("OKF file must have a string path and content")
        parts = path.split("/")
        if (
            path.startswith("/")
            or "\\" in path
            or any(part in {"", ".", ".."} for part in parts)
        ):
            raise ConformanceError("OKF file path must stay within the bundle")
        if path in paths:
            raise ConformanceError("OKF bundle repeats a file path")
        paths.add(path)
        if not path.endswith(".md"):
            continue
        name = PurePosixPath(path).name
        if name == "index.md":
            if content.startswith("---\n"):
                fields, _ = PLUGIN.parse_frontmatter(content, path)
                if path != "index.md" or set(fields) != {"okf_version"}:
                    raise ConformanceError(
                        "only the root index may declare OKF frontmatter"
                    )
                if fields["okf_version"] != "0.2":
                    raise ConformanceError("root index must declare OKF 0.2")
            continue
        if name == "log.md":
            if content.startswith("---\n"):
                raise ConformanceError("reserved log must not contain frontmatter")
            dates = re.findall(r"^## (.+)$", content, flags=re.MULTILINE)
            if any(re.fullmatch(r"\d{4}-\d{2}-\d{2}", date) is None for date in dates):
                raise ConformanceError("OKF log headings must use ISO dates")
            try:
                for value in dates:
                    date.fromisoformat(value)
            except ValueError as error:
                raise ConformanceError(
                    "OKF log headings must use valid ISO dates"
                ) from error
            if dates != sorted(dates, reverse=True):
                raise ConformanceError("OKF log dates must be newest first")
            continue
        fields, _ = PLUGIN.parse_frontmatter(content, path)
        kind = fields.get("type")
        if not isinstance(kind, str) or not kind.strip():
            raise ConformanceError("OKF concept type must be a nonempty string")
        if kind == "Attested Computation":
            runtime = fields.get("runtime")
            if not isinstance(runtime, str) or not runtime.strip():
                raise ConformanceError("Attested Computation must declare runtime")
        concepts[path] = fields
    return concepts


def validate_capsule(capsule: dict[str, Any]) -> None:
    """Verify UTF-8 lengths, every digest, and the manifest's closed payload set."""
    files = capsule.get("files")
    manifest = capsule.get("manifest")
    if not isinstance(files, list) or not isinstance(manifest, dict):
        raise ConformanceError("portable capsule has no files or manifest")
    inventory = manifest.get("files")
    if not isinstance(inventory, list) or len(inventory) != len(files):
        raise ConformanceError("portable manifest inventory differs")
    for file, entry in zip(files, inventory, strict=True):
        if not isinstance(file, dict) or not isinstance(entry, dict):
            raise ConformanceError("portable manifest file must be an object")
        content = file.get("content")
        if not isinstance(content, str):
            raise ConformanceError("portable payload content must be UTF-8 text")
        encoded = content.encode("utf-8")
        if (
            file.get("bytes") != len(encoded)
            or file.get("sha256") != hashlib.sha256(encoded).hexdigest()
        ):
            raise ConformanceError(
                "portable payload digest or UTF-8 byte length differs"
            )
        if any(
            file.get(key) != entry.get(key)
            for key in ("path", "mediaType", "bytes", "sha256")
        ):
            raise ConformanceError("portable manifest inventory differs")
    canonical = json.dumps(
        manifest, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    )
    if (
        capsule.get("manifestSha256")
        != hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    ):
        raise ConformanceError("portable manifest digest differs")
    validate_okf(files)


def call(identifier: int, tool: str, arguments: dict[str, Any]) -> dict[str, Any]:
    """Build a protocol request without owning any room or journal rules."""
    return {
        "jsonrpc": "2.0",
        "id": identifier,
        "method": "tools/call",
        "params": {"name": tool, "arguments": arguments},
    }


def session(binary: Path, requests: list[dict[str, Any]]) -> dict[int, dict[str, Any]]:
    """Drive one bounded stdio session in a disposable player profile."""
    with tempfile.TemporaryDirectory(prefix="numinous-interoperability-") as temporary:
        profile = Path(temporary)
        env = {
            key: value
            for key, value in os.environ.items()
            if key.upper() in {"SYSTEMROOT", "WINDIR", "COMSPEC", "PATH", "PATHEXT"}
        }
        env.update(
            {
                "HOME": temporary,
                "USERPROFILE": temporary,
                "TEMP": temporary,
                "TMP": temporary,
                "TMPDIR": temporary,
            }
        )
        for name, filename in (
            ("JOURNEY", "journey.txt"),
            ("SCORES", "scores.txt"),
            ("CAIRN", "cairn.json"),
            ("JOURNAL", "journal.txt"),
            ("PROJECT", "project.txt"),
        ):
            env[f"NUMINOUS_{name}"] = str(profile / filename)
        payload = "".join(
            json.dumps(request, ensure_ascii=False) + "\n" for request in requests
        )
        if len(payload.encode("utf-8")) > MAX_SESSION_BYTES:
            raise ConformanceError("stdio request exceeds conformance session bound")
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as diagnostic:
            process = subprocess.run(
                [str(binary.resolve())],
                input=payload.encode("utf-8"),
                stdout=output,
                stderr=diagnostic,
                env=env,
                cwd=profile,
                timeout=30,
                check=False,
            )
            if process.returncode != 0:
                diagnostic.seek(0)
                raise ConformanceError(
                    f"MCP server failed: {diagnostic.read(2048).decode('utf-8', errors='replace')}"
                )
            if output.tell() > MAX_SESSION_BYTES:
                raise ConformanceError(
                    "stdio response exceeds conformance session bound"
                )
            output.seek(0)
            responses = [
                json.loads(line, object_pairs_hook=PLUGIN.object_without_duplicates)
                for line in output
                if line.strip()
            ]
    expected = [request["id"] for request in requests if "id" in request]
    if len(responses) != len(expected):
        raise ConformanceError("MCP response count differs from the submitted requests")
    result: dict[int, dict[str, Any]] = {}
    for response, identifier in zip(responses, expected, strict=True):
        if (
            not isinstance(response, dict)
            or response.get("id") != identifier
            or isinstance(response.get("id"), bool)
            or response.get("jsonrpc") != "2.0"
            or set(response) != {"jsonrpc", "id", "result"}
        ):
            raise ConformanceError("invalid MCP response envelope")
        value = response.get("result")
        if not isinstance(value, dict) or value.get("isError") or "error" in response:
            raise ConformanceError(f"MCP request {identifier} failed: {response}")
        result[identifier] = value
    return result


def production_conformance(binary: Path) -> dict[str, Any]:
    """Read live OKF and portable capsules, including correction and pagination."""
    kind = 'thought: "quoted" \\ path\nnext\tline'
    subject = 'math: "quoted" \\ 日本語'
    requests: list[dict[str, Any]] = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "interoperability-gate", "version": "1"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        call(2, "export_journal", {"format": "okf-0.2"}),
        call(
            3,
            "record_journal",
            {
                "kind": kind,
                "subject": subject,
                "text": 'First account: "one".\nSecond line.',
                "affect": "curious",
                "event_time_utc": 1,
            },
        ),
        call(
            4,
            "correct_journal",
            {
                "entry_id": 1,
                "text": "Corrected account.",
                "source": "player-provided",
                "event_time_utc": 2,
            },
        ),
        call(5, "export_journal", {"format": "okf-0.2", "limit": 1}),
        call(
            6, "export_journal", {"format": "okf-0.2", "limit": 1, "after_entry_id": 1}
        ),
        call(7, "export_journal", {"format": "portable-1"}),
    ]
    results = session(binary, requests)
    empty = results[2]["structuredContent"]
    if validate_okf(empty["files"]) or empty["page"]["returned"] != 0:
        raise ConformanceError("empty OKF export contains unexpected concepts")
    first = results[5]["structuredContent"]
    second = results[6]["structuredContent"]
    before = list(validate_okf(first["files"]).values())
    after = list(validate_okf(second["files"]).values())
    if len(before) != 1 or len(after) != 1:
        raise ConformanceError("paged OKF export did not return one concept per page")
    original, corrected = before[0], after[0]
    if original["tags"][-1] != kind or original["numinous"]["entry_id"] != 1:
        raise ConformanceError(
            "real YAML parser did not recover the exact journal metadata"
        )
    if original["status"] != "deprecated" or corrected["status"] != "stable":
        raise ConformanceError(
            "OKF correction lifecycle differs from the native journal"
        )
    if (
        corrected["numinous"]["supersedes"] != 1
        or corrected["sources"][0]["resource"] != "/entries/00000000000000000001.md"
    ):
        raise ConformanceError("OKF correction lost its inspectable lineage")
    if (
        first["page"]["hasMore"] is not True
        or first["page"]["nextAfterEntryId"] != 1
        or second["page"]["hasMore"] is not False
    ):
        raise ConformanceError("OKF pagination does not match the native journal page")
    for page in (empty, first, second):
        if (
            page.get("createdFile") is not False
            or page.get("containsHostPath") is not False
        ):
            raise ConformanceError("OKF export crossed its in-memory privacy boundary")
    capsule = results[7]["structuredContent"]
    validate_capsule(capsule)
    native = next(
        file for file in capsule["files"] if file["path"] == "native/journal-page.json"
    )
    if json.loads(native["content"])["entries"][0]["subject"] != subject:
        raise ConformanceError("portable native journal changed Unicode subject text")
    return {
        "passed": True,
        "schema": "numinous.interoperability-conformance",
        "agentPluginsVersion": "1.0.0",
        "okfVersion": "0.2",
        "okfPagesChecked": 3,
        "portableCapsulesChecked": 1,
        "externalHostDiscoveryProven": False,
    }


def main() -> int:
    """Validate the package and current compiled producer with no paid services."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="explicit prebuilt MCP executable")
    args = parser.parse_args()
    try:
        PLUGIN.validate_package(PLUGIN.DEFAULT_PLUGIN_ROOT)
        binary = args.binary or build_and_locate(("numinous-mcp",))[0]
        summary = production_conformance(binary)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
