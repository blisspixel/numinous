"""Compare the built public CLI and MCP study contracts.

Run after building both binaries:
  python scripts/study-parity.py --cli PATH --mcp PATH

Paths are explicit so a focused Cargo test never silently exercises a stale
sibling binary. This harness reads no real player profile.
"""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def cli_call(binary, arguments, environment, *, as_json):
    command = [str(binary), "study", arguments["room"]]
    for key in ("locale", "depth", "block"):
        if key in arguments:
            command.extend((f"--{key}", arguments[key]))
    if as_json:
        command.append("--json")
    return subprocess.run(
        command, env=environment, capture_output=True, timeout=30, check=False
    )


def mcp_call(binary, arguments, environment):
    messages = [
        {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "study-parity", "version": "1"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "study_room", "arguments": arguments},
        },
    ]
    incoming = "".join(json.dumps(message) + "\n" for message in messages)
    output = subprocess.run(
        [str(binary)], input=incoming.encode("utf-8"), env=environment,
        capture_output=True, timeout=30, check=False,
    )
    assert output.returncode == 0, output.stderr.decode("utf-8")
    assert not output.stderr, output.stderr.decode("utf-8")
    replies = [json.loads(line) for line in output.stdout.decode("utf-8").splitlines()]
    response = next(reply for reply in replies if reply.get("id") == 2)
    assert "result" in response, response
    return response["result"]


def compare(cli, mcp, arguments, environment):
    command = cli_call(cli, arguments, environment, as_json=True)
    assert command.returncode == 0, command.stderr.decode("utf-8")
    assert not command.stderr, command.stderr.decode("utf-8")
    structured = json.loads(command.stdout.decode("utf-8"))
    result = mcp_call(mcp, arguments, environment)
    assert result["isError"] is False, result
    assert structured == result["structuredContent"], arguments
    assert structured["schema"] == "numinous.room-study"
    assert structured["schemaVersion"] == 1

    text = cli_call(cli, arguments, environment, as_json=False)
    assert text.returncode == 0, text.stderr.decode("utf-8")
    assert not text.stderr, text.stderr.decode("utf-8")
    assert text.stdout.decode("utf-8") == result["content"][0]["text"], arguments
    return structured


def run(cli, mcp):
    cases = [
        {"room": "lissajous"},
        {"room": "lissajous", "locale": "ja"},
        {"room": "lissajous", "locale": "ja-JP", "depth": "mathematics"},
        {"room": "lissajous", "locale": "ja", "depth": "notes"},
        {"room": "lissajous", "locale": "fr", "block": "lissajous.recurrence"},
        {"room": "times-tables", "locale": "ja"},
        {"room": "lissajous", "locale": "ja", "block": "lissajous.references"},
    ]
    with tempfile.TemporaryDirectory(prefix="numinous-study-parity-") as temporary:
        state = Path(temporary) / "profile"
        environment = dict(os.environ)
        names = ("JOURNEY", "SCORES", "CAIRN", "JOURNAL", "PREFERENCES")
        for name in names:
            environment[f"NUMINOUS_{name}"] = str(state / name.lower())
        outputs = []
        for arguments in cases:
            outputs.append(compare(cli, mcp, arguments, environment))
            assert not state.exists(), "a study call created player state"

        assert outputs[0]["selection"] == {"kind": "depth", "depth": "explanation"}
        assert outputs[1]["locale"] == {"requested": "ja", "resolved": "ja", "fallback": None}
        assert outputs[2]["locale"]["fallback"] == "parent_language"
        assert all(block["locale"]["resolved"] == "en" for block in outputs[3]["blocks"])
        assert outputs[4]["locale"]["fallback"] == "translation_unavailable"
        assert outputs[5]["locale"]["resolved"] == "en"
        assert "mathematics" not in outputs[5]["availableDepths"]
        assert any(part["kind"] == "reference" for part in outputs[6]["blocks"][0]["parts"])

        invalid = [
            {"room": "missing-room"},
            {"room": "lissajous", "locale": "ja_JP"},
            {"room": "times-tables", "depth": "mathematics"},
            {"room": "lissajous", "block": "lissajous.missing"},
            {"room": "lissajous", "block": "lissajous.recurrence", "depth": "mathematics"},
        ]
        for arguments in invalid:
            command = cli_call(cli, arguments, environment, as_json=True)
            result = mcp_call(mcp, arguments, environment)
            assert command.returncode != 0 and command.stderr, arguments
            assert not command.stdout, "CLI substituted content on failure"
            assert result["isError"] is True, arguments
            assert "structuredContent" not in result, "MCP substituted content on failure"
            assert not state.exists()

        state.mkdir()
        before = {name.lower(): f"preserve {name}\n".encode("utf-8") for name in names}
        for name, content in before.items():
            (state / name).write_bytes(content)
        for arguments in (cases[2], cases[5]):
            compare(cli, mcp, arguments, environment)
            after = {path.name: path.read_bytes() for path in state.iterdir()}
            assert after == before, "study changed an existing player profile"
    print("Study parity: 9 successful CLI/MCP comparisons, 5 shared refusals; profile unchanged.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--mcp", required=True, type=Path)
    arguments = parser.parse_args()
    run(arguments.cli.resolve(strict=True), arguments.mcp.resolve(strict=True))
