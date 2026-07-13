#!/usr/bin/env python3
"""Drive a freshly built Numinous MCP server over stdio.

Each invocation owns a unique temporary profile containing Journey, scores,
and Cairn state. The profile is removed before the process exits, so QA calls
cannot contaminate a player or another concurrent tester.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
STATE_DIR_PREFIX = "numinous-mcp-play-"


class McpPlayError(RuntimeError):
    """A readable protocol, server, or tool failure."""


def _binary() -> str:
    """Build and return the fresh debug server without replacing a live release."""
    subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "numinous-mcp"],
        cwd=ROOT,
        check=True,
    )
    binary = ROOT / "target" / "debug" / "numinous-mcp"
    windows_binary = binary.with_suffix(".exe")
    return str(windows_binary if windows_binary.exists() else binary)


def _session(requests: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Send requests through one fresh server and one disposable profile."""
    with tempfile.TemporaryDirectory(prefix=STATE_DIR_PREFIX) as state_dir:
        state_root = Path(state_dir)
        env = dict(os.environ)
        env.update(
            {
                "NUMINOUS_JOURNEY": str(state_root / "journey.txt"),
                "NUMINOUS_SCORES": str(state_root / "scores.txt"),
                "NUMINOUS_CAIRN": str(state_root / "cairn.json"),
            }
        )
        payload = "".join(json.dumps(request) + "\n" for request in requests)
        process = subprocess.run(
            [_binary()],
            input=payload,
            capture_output=True,
            text=True,
            cwd=ROOT,
            env=env,
            check=False,
        )
        if process.returncode != 0:
            detail = process.stderr.strip() or process.stdout.strip() or "no diagnostic output"
            raise McpPlayError(
                f"server exited with status {process.returncode}: {detail}"
            )

        responses: list[dict[str, Any]] = []
        for line_number, line in enumerate(process.stdout.splitlines(), start=1):
            line = line.strip()
            if not line:
                continue
            try:
                response = json.loads(line)
            except json.JSONDecodeError as error:
                raise McpPlayError(
                    f"server returned invalid JSON on line {line_number}: {error}"
                ) from error
            if not isinstance(response, dict):
                raise McpPlayError(
                    f"server returned a non-object response on line {line_number}"
                )
            responses.append(response)
        if len(responses) != len(requests):
            raise McpPlayError(
                f"server returned {len(responses)} response(s) for {len(requests)} request(s)"
            )
        return responses


def _response_result(response: dict[str, Any], operation: str) -> dict[str, Any]:
    """Return one successful JSON-RPC result or raise a readable failure."""
    error = response.get("error")
    if isinstance(error, dict):
        code = error.get("code", "unknown")
        message = error.get("message", "no error message")
        raise McpPlayError(f"{operation} failed ({code}): {message}")
    result = response.get("result")
    if not isinstance(result, dict):
        raise McpPlayError(f"{operation} returned no object result")
    return result


def _tool_text(result: dict[str, Any]) -> str:
    """Collect every textual content block from a tool result."""
    content = result.get("content", [])
    if not isinstance(content, list):
        return ""
    blocks = []
    for item in content:
        if isinstance(item, dict) and isinstance(item.get("text"), str):
            blocks.append(item["text"])
    return "\n".join(blocks)


def _init(extra: list[dict[str, Any]]) -> list[dict[str, Any]]:
    initialize = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "mcp-play", "version": "1"},
        },
    }
    responses = _session([initialize, *extra])
    _response_result(responses[0], "initialize")
    return responses


def _call_tool(tool: str, arguments: dict[str, Any]) -> int:
    responses = _init(
        [
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": tool, "arguments": arguments},
            }
        ]
    )
    result = _response_result(responses[-1], f"tool '{tool}'")
    text = _tool_text(result)
    if result.get("isError") is True:
        raise McpPlayError(f"tool '{tool}' failed: {text or 'no error message'}")
    if text:
        print(text)
    structured = result.get("structuredContent")
    if structured is not None:
        print("\n--- structuredContent ---")
        print(json.dumps(structured, indent=2))
    return 0


def _list_tools() -> int:
    responses = _init(
        [{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}]
    )
    result = _response_result(responses[-1], "tools/list")
    tools = result.get("tools")
    if not isinstance(tools, list):
        raise McpPlayError("tools/list returned no tool array")
    for tool in tools:
        if not isinstance(tool, dict):
            raise McpPlayError("tools/list returned a malformed tool entry")
        name = tool.get("name", "<unnamed>")
        description = tool.get("description", "No description provided.")
        print(name)
        print(
            textwrap.fill(
                str(description),
                width=88,
                initial_indent="  ",
                subsequent_indent="  ",
                break_long_words=False,
                break_on_hyphens=False,
            )
        )
        print()
    print(f"{len(tools)} tools.")
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Build the latest Numinous MCP server and exercise it through an "
            "isolated, automatically cleaned test profile."
        ),
        epilog=textwrap.dedent(
            """
            examples:
              python scripts/mcp-play.py list
              python scripts/mcp-play.py tools
              python scripts/mcp-play.py call play_room '{"id":"lorenz","t":0.5}'
              python scripts/mcp-play.py call predict '{"id":"slope-rider","seed":4}'
              '{"id":"cult-of-pi"}' | python scripts/mcp-play.py call describe_room -

            Each command starts with empty Journey, score, and Cairn state. Use a
            direct MCP session when a test intentionally needs persistent state.
            Pass - to read JSON from stdin, which avoids shell quoting differences.
            """
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("list", help="list all catalog rooms")
    subcommands.add_parser(
        "tools", help="show every tool with its complete description"
    )
    call = subcommands.add_parser("call", help="call one MCP tool")
    call.add_argument("tool", help="tool name, for example play_room")
    call.add_argument(
        "arguments",
        nargs="?",
        default="{}",
        help="JSON object of tool arguments, or - to read it from stdin (default: {})",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _parser()
    args = parser.parse_args(argv)
    try:
        if args.command == "tools":
            return _list_tools()
        if args.command == "list":
            return _call_tool("list_rooms", {})
        raw_arguments = sys.stdin.read() if args.arguments == "-" else args.arguments
        try:
            arguments = json.loads(raw_arguments)
        except json.JSONDecodeError as error:
            print(f"mcp-play: bad JSON arguments: {error}", file=sys.stderr)
            return 2
        if not isinstance(arguments, dict):
            print("mcp-play: tool arguments must be a JSON object", file=sys.stderr)
            return 2
        return _call_tool(args.tool, arguments)
    except McpPlayError as error:
        print(f"mcp-play: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        print(
            f"mcp-play: could not build the latest server (status {error.returncode})",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
