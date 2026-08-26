#!/usr/bin/env python3
"""Exercise the installed CLI and MCP binaries from an isolated profile."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any, BinaryIO


PROTOCOL_VERSION = "2026-07-28"
EXPECTED_TOOL_NAMES = frozenset(
    {
        "aliens",
        "broadcast_session",
        "cairn",
        "challenge",
        "choose",
        "correct_journal",
        "crack",
        "describe_room",
        "erase_journal",
        "explain_joke",
        "export_journal",
        "fifteen",
        "forget",
        "fork_creation",
        "gauntlet",
        "hackenbush",
        "journey",
        "list_rooms",
        "list_sims",
        "listen_room",
        "munch",
        "munch_arcade",
        "nim",
        "open_creation",
        "party",
        "play_room",
        "plot_expression",
        "predict",
        "quiz",
        "read_journal",
        "record_journal",
        "reveal_room",
        "run_sim",
        "save_creation",
        "scores",
        "seti",
        "sing_expression",
        "trophies",
        "workspace",
        "watch_show",
    }
)
EXPECTED_TOOL_COUNT = len(EXPECTED_TOOL_NAMES)
CLI_TIMES_TABLES_ACTION = (
    "Action: TURN THE DIAL (phase here: numinous render times-tables --t 0.375; "
    "--poke x,y is a second hand)"
)
PROCESS_TIMEOUT_SECONDS = 20
READER_SHUTDOWN_TIMEOUT_SECONDS = 1
MAX_OUTPUT_BYTES = 1_048_576
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")


class SmokeError(RuntimeError):
    """A release binary failed its bounded engagement contract."""


def installed_binary(bin_dir: Path, name: str) -> Path:
    """Return one ordinary installed binary for the current platform."""
    directory = bin_dir.resolve(strict=True)
    if not directory.is_dir():
        raise SmokeError(f"binary directory is not a directory: {directory}")
    suffix = ".exe" if os.name == "nt" else ""
    candidate = directory / f"{name}{suffix}"
    if candidate.is_symlink():
        raise SmokeError(f"installed binary is a symbolic link: {candidate.name}")
    try:
        binary = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise SmokeError(f"installed binary is missing: {candidate.name}") from error
    if not binary.is_file():
        raise SmokeError(f"installed binary is not an ordinary file: {candidate.name}")
    return binary


def isolated_environment(state_root: Path) -> dict[str, str]:
    """Return a child environment whose Numinous state stays under state_root."""
    state_root.mkdir(parents=True, exist_ok=False)
    local_data = state_root / "local-data"
    config = state_root / "config"
    local_data.mkdir()
    config.mkdir()
    environment = os.environ.copy()
    environment.update(
        {
            "HOME": str(state_root),
            "USERPROFILE": str(state_root),
            "LOCALAPPDATA": str(local_data),
            "XDG_CONFIG_HOME": str(config),
            "NUMINOUS_JOURNEY": str(state_root / "journey.txt"),
            "NUMINOUS_SCORES": str(state_root / "scores.txt"),
            "NUMINOUS_CAIRN": str(state_root / "cairn.txt"),
            "NUMINOUS_JOURNAL": str(state_root / "journal.txt"),
            "NO_COLOR": "1",
            "TERM": "dumb",
        }
    )
    return environment


def run_process(
    command: list[str],
    *,
    cwd: Path,
    environment: dict[str, str],
    input_text: str | None = None,
) -> str:
    """Run one release binary with bounded time and output."""
    executable = Path(command[0]).name
    with tempfile.TemporaryFile() as standard_input:
        if input_text is not None:
            standard_input.write(input_text.encode("utf-8"))
        standard_input.seek(0)
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=environment,
            stdin=standard_input,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        standard_output = process.stdout
        standard_error = process.stderr
        if standard_output is None or standard_error is None:
            process.kill()
            process.wait()
            raise SmokeError(f"{executable} did not expose output pipes")
        output = bytearray()
        error_output = bytearray()
        output_exceeded = threading.Event()
        stream_failed = threading.Event()
        readers = (
            threading.Thread(
                target=read_bounded_stream,
                args=(standard_output, output, output_exceeded, stream_failed),
                daemon=True,
            ),
            threading.Thread(
                target=read_bounded_stream,
                args=(standard_error, error_output, output_exceeded, stream_failed),
                daemon=True,
            ),
        )
        for reader in readers:
            reader.start()
        deadline = time.monotonic() + PROCESS_TIMEOUT_SECONDS
        failure: str | None = None
        while process.poll() is None:
            if output_exceeded.is_set():
                failure = f"{executable} exceeded the output limit"
                break
            if time.monotonic() >= deadline:
                failure = (
                    f"{executable} exceeded {PROCESS_TIMEOUT_SECONDS} seconds"
                )
                break
            time.sleep(0.01)
        if failure is not None:
            process.kill()
        return_code = process.wait()
        reader_deadline = time.monotonic() + READER_SHUTDOWN_TIMEOUT_SECONDS
        for reader in readers:
            reader.join(timeout=max(0.0, reader_deadline - time.monotonic()))
        if failure is not None or output_exceeded.is_set():
            raise SmokeError(failure or f"{executable} exceeded the output limit")
        if any(reader.is_alive() for reader in readers):
            raise SmokeError(f"{executable} did not close its output streams")
        if stream_failed.is_set():
            raise SmokeError(f"{executable} output could not be read")
        decoded_output = output.decode("utf-8", errors="strict")
        decoded_error = error_output.decode("utf-8", errors="strict")
    if return_code != 0:
        raise SmokeError(
            f"{executable} exited with status {return_code}"
        )
    if decoded_error.strip():
        raise SmokeError(f"{executable} wrote unexpected stderr")
    return decoded_output


def read_bounded_stream(
    stream: BinaryIO,
    destination: bytearray,
    output_exceeded: threading.Event,
    stream_failed: threading.Event,
) -> None:
    """Drain one child pipe without retaining bytes beyond the output cap."""
    try:
        while chunk := stream.read(65_536):
            remaining = MAX_OUTPUT_BYTES - len(destination)
            if len(chunk) > remaining:
                if remaining > 0:
                    destination.extend(chunk[:remaining])
                output_exceeded.set()
                return
            destination.extend(chunk)
    except OSError:
        stream_failed.set()
    finally:
        stream.close()


def validate_render_body(render: str) -> None:
    """Require a substantive bounded Times Tables picture."""
    if not render.strip():
        raise SmokeError("Times Tables render is empty")
    if len(render.encode("utf-8")) > MAX_OUTPUT_BYTES:
        raise SmokeError("Times Tables render exceeded the output limit")
    if len(render.splitlines()) < 20:
        raise SmokeError("Times Tables render has too few rows")
    if "*" not in render or "#" not in render:
        raise SmokeError("Times Tables render has no characteristic ink")


def validate_cli_render(render: str) -> None:
    """Require the installed CLI's picture and semantic room chrome."""
    validate_render_body(render)
    required = (
        "Status:",
        CLI_TIMES_TABLES_ACTION,
        "Goal: LAND ON EXACTLY 4 LOBES",
    )
    missing = [marker for marker in required if marker not in render]
    if missing:
        raise SmokeError(f"Times Tables render is missing {missing[0]}")


def validate_cli_version(output: str) -> str:
    """Return the installed CLI version after strict shape validation."""
    lines = output.splitlines()
    if len(lines) != 1 or not lines[0].startswith("numinous "):
        raise SmokeError("installed CLI returned an unexpected version line")
    version = lines[0].removeprefix("numinous ")
    if VERSION_RE.fullmatch(version) is None:
        raise SmokeError("installed CLI returned a malformed version")
    return version


def parse_mcp_output(output: str) -> list[dict[str, Any]]:
    """Parse one bounded JSON response per nonempty stdout line."""
    responses: list[dict[str, Any]] = []
    for line in output.splitlines():
        if not line.strip():
            continue
        if len(line.encode("utf-8")) > MAX_OUTPUT_BYTES:
            raise SmokeError("MCP response line exceeded the output limit")
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise SmokeError("MCP stdout contains malformed JSON") from error
        if not isinstance(value, dict):
            raise SmokeError("MCP response is not a JSON object")
        responses.append(value)
    return responses


def require_result(responses: dict[int, dict[str, Any]], request_id: int) -> dict[str, Any]:
    """Return one successful object result for request_id."""
    response = responses.get(request_id)
    if response is None:
        raise SmokeError(f"MCP response {request_id} is missing")
    if "error" in response:
        raise SmokeError(f"MCP response {request_id} is an error")
    result = response.get("result")
    if not isinstance(result, dict):
        raise SmokeError(f"MCP response {request_id} has no object result")
    return result


def result_server_version(result: dict[str, Any], label: str) -> str:
    """Return the server version carried by one successful modern result."""
    metadata = result.get("_meta")
    if not isinstance(metadata, dict):
        raise SmokeError(f"{label} omitted result metadata")
    server = metadata.get("io.modelcontextprotocol/serverInfo")
    if not isinstance(server, dict) or server.get("name") != "numinous":
        raise SmokeError(f"{label} returned the wrong server identity")
    version = server.get("version")
    if not isinstance(version, str) or VERSION_RE.fullmatch(version) is None:
        raise SmokeError(f"{label} returned a malformed server version")
    return version


def validate_mcp_responses(
    responses: list[dict[str, Any]], expected_version: str | None = None
) -> None:
    """Validate modern discovery, tool inventory, and one real room play."""
    by_id: dict[int, dict[str, Any]] = {}
    for response in responses:
        if response.get("jsonrpc") != "2.0":
            raise SmokeError("MCP response has the wrong JSON-RPC version")
        request_id = response.get("id")
        if not isinstance(request_id, int) or isinstance(request_id, bool):
            raise SmokeError("MCP response has no integer id")
        if request_id in by_id:
            raise SmokeError(f"MCP response id {request_id} is duplicated")
        by_id[request_id] = response
    if set(by_id) != {1, 2, 3}:
        raise SmokeError("MCP response ids do not match the smoke requests")

    discovery = require_result(by_id, 1)
    if discovery.get("resultType") != "complete":
        raise SmokeError("server/discover did not complete")
    versions = discovery.get("supportedVersions")
    if not isinstance(versions, list) or PROTOCOL_VERSION not in versions:
        raise SmokeError("server/discover omitted the current protocol version")
    observed_versions = [result_server_version(discovery, "server/discover")]

    inventory = require_result(by_id, 2)
    if inventory.get("resultType") != "complete":
        raise SmokeError("tools/list did not complete")
    tools = inventory.get("tools")
    if not isinstance(tools, list) or len(tools) != EXPECTED_TOOL_COUNT:
        raise SmokeError(f"tools/list did not return {EXPECTED_TOOL_COUNT} tools")
    names = [tool.get("name") for tool in tools if isinstance(tool, dict)]
    if len(names) != EXPECTED_TOOL_COUNT or any(not isinstance(name, str) for name in names):
        raise SmokeError("tools/list contains a malformed tool entry")
    if len(set(names)) != EXPECTED_TOOL_COUNT:
        raise SmokeError("tools/list contains duplicate tool names")
    if set(names) != EXPECTED_TOOL_NAMES:
        raise SmokeError("tools/list does not match the exact expected inventory")
    observed_versions.append(result_server_version(inventory, "tools/list"))

    play = require_result(by_id, 3)
    if play.get("resultType") != "complete" or play.get("isError") is not False:
        raise SmokeError("play_room did not complete successfully")
    observed_versions.append(result_server_version(play, "play_room"))
    if len(set(observed_versions)) != 1:
        raise SmokeError("MCP results disagree about the server version")
    if expected_version is not None and observed_versions[0] != expected_version:
        raise SmokeError("CLI and MCP binary versions do not match")
    structured = play.get("structuredContent")
    if not isinstance(structured, dict):
        raise SmokeError("play_room omitted structuredContent")
    expected_fields = {
        "room": "times-tables",
        "width": 40,
        "height": 20,
        "t": 0.25,
        "action": "DRAG: TURN THE DIAL",
        "goal": "LAND ON EXACTLY 4 LOBES",
        "status": "DRAG:DIAL  K 4.00  CLOSED  3 LOBES  TARGET 4",
    }
    for name, expected in expected_fields.items():
        if structured.get(name) != expected:
            raise SmokeError(f"play_room returned an unexpected {name}")
    render = structured.get("render")
    if not isinstance(render, str):
        raise SmokeError("play_room omitted its structured render")
    validate_render_body(render)


def modern_meta() -> dict[str, Any]:
    """Return the stateless metadata required by the current MCP revision."""
    return {
        "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientInfo": {
            "name": "numinous-release-smoke",
            "version": "1.0",
        },
        "io.modelcontextprotocol/clientCapabilities": {},
    }


def mcp_requests() -> str:
    """Return the three deterministic newline-framed smoke requests."""
    requests = (
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": {"_meta": modern_meta()},
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {"_meta": modern_meta()},
        },
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "_meta": modern_meta(),
                "name": "play_room",
                "arguments": {
                    "id": "times-tables",
                    "t": 0.25,
                    "width": 40,
                    "height": 20,
                },
            },
        },
    )
    return "".join(
        f"{json.dumps(request, separators=(',', ':'), sort_keys=True)}\n"
        for request in requests
    )


def run_engagement_smoke(bin_dir: Path) -> str:
    """Exercise installed CLI and MCP binaries and return their shared version."""
    cli = installed_binary(bin_dir, "numinous")
    mcp = installed_binary(bin_dir, "numinous-mcp")
    with tempfile.TemporaryDirectory(prefix="numinous-release-engagement-") as temporary:
        root = Path(temporary)
        environment = isolated_environment(root / "profile")
        version = validate_cli_version(
            run_process(
                [str(cli), "--version"],
                cwd=root,
                environment=environment,
            )
        )
        render = run_process(
            [
                str(cli),
                "render",
                "times-tables",
                "--width",
                "40",
                "--height",
                "20",
                "--t",
                "0.25",
            ],
            cwd=root,
            environment=environment,
        )
        validate_cli_render(render)
        output = run_process(
            [str(mcp)],
            cwd=root,
            environment=environment,
            input_text=mcp_requests(),
        )
        validate_mcp_responses(parse_mcp_output(output), version)
    return version


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bin-dir",
        type=Path,
        required=True,
        help="directory containing the installed Numinous binaries",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        run_engagement_smoke(args.bin_dir)
    except (SmokeError, OSError, UnicodeError) as error:
        print(f"release engagement smoke failed: {error}", file=sys.stderr)
        return 1
    print("release engagement smoke passed: CLI render and MCP discovery, list, and play")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
