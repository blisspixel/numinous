#!/usr/bin/env python3
"""Drive the LATEST Numinous MCP server over stdio, for playtesting the real
build instead of a stale, already-running session server.

Why this exists: an editor's MCP server is a long-running process holding the
release binary locked, so it cannot be hot-swapped and it tests old code. This
driver builds a fresh `numinous-mcp` from current source and speaks JSON-RPC to
it directly, so a playtester (human or agent) always exercises the latest tools,
reveals, and structured output. Each call runs against isolated temp journey and
score files, so play never touches the real save.

Usage:
    python scripts/mcp-play.py list
    python scripts/mcp-play.py tools
    python scripts/mcp-play.py call play_room '{"id":"lorenz","t":0.5,"width":60,"height":30}'
    python scripts/mcp-play.py call predict '{"id":"slope-rider","seed":4}'

The `call` form prints the human text and, when present, the structuredContent
as pretty JSON, so a playtester sees exactly what a structured-content client
would surface.
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _binary() -> str:
    """Build (if needed) and return the path to a fresh debug numinous-mcp.

    Debug, not release, so it never collides with a running release server that
    holds the release binary locked.
    """
    subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "numinous-mcp"],
        cwd=ROOT,
        check=True,
    )
    exe = ROOT / "target" / "debug" / "numinous-mcp"
    win = exe.with_suffix(".exe")
    return str(win if win.exists() else exe)


def _session(requests: list[dict]) -> list[dict]:
    """Send a batch of JSON-RPC requests to a fresh server, return responses."""
    tmp = Path(tempfile.gettempdir())
    env = {
        "NUMINOUS_JOURNEY": str(tmp / "numinous_mcp_play_journey.txt"),
        "NUMINOUS_SCORES": str(tmp / "numinous_mcp_play_scores.txt"),
    }
    import os

    full_env = dict(os.environ)
    full_env.update(env)
    payload = "".join(json.dumps(r) + "\n" for r in requests)
    proc = subprocess.run(
        [_binary()],
        input=payload,
        capture_output=True,
        text=True,
        cwd=ROOT,
        env=full_env,
    )
    out = []
    for line in proc.stdout.splitlines():
        line = line.strip()
        if line:
            out.append(json.loads(line))
    return out


def _init(extra: list[dict]) -> list[dict]:
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "mcp-play", "version": "1"},
        },
    }
    return _session([init, *extra])


def main() -> int:
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 2
    verb = args[0]
    if verb == "tools":
        resp = _init([{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}])
        tools = resp[-1]["result"]["tools"]
        for t in tools:
            print(f"{t['name']}: {t['description'][:100]}")
        print(f"\n{len(tools)} tools.")
        return 0
    if verb == "list":
        return main_call("list_rooms", "{}")
    if verb == "call":
        if len(args) < 2:
            print("usage: mcp-play.py call <tool> '<json-args>'")
            return 2
        tool = args[1]
        tool_args = args[2] if len(args) > 2 else "{}"
        return main_call(tool, tool_args)
    print(f"unknown verb '{verb}'; try tools, list, or call")
    return 2


def main_call(tool: str, tool_args: str) -> int:
    try:
        arguments = json.loads(tool_args)
    except json.JSONDecodeError as exc:
        print(f"bad JSON arguments: {exc}")
        return 2
    resp = _init(
        [
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": tool, "arguments": arguments},
            }
        ]
    )
    result = resp[-1].get("result", resp[-1])
    text = ""
    if isinstance(result, dict):
        content = result.get("content", [])
        if content and isinstance(content, list):
            text = content[0].get("text", "")
    print(text)
    sc = result.get("structuredContent") if isinstance(result, dict) else None
    if sc is not None:
        print("\n--- structuredContent ---")
        print(json.dumps(sc, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
