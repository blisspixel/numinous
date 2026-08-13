#!/usr/bin/env python3
"""Let one already-installed local Ollama model play Numinous over MCP.

The harness is exploratory evidence, not a deterministic CI cohort and not a
consciousness test. It accepts only a literal loopback endpoint, refuses cloud
model names and missing local models, keeps all Numinous state in a disposable
profile, and records no private model reasoning. A full transcript is written
only when the caller explicitly selects a path under the gitignored logs/
directory.
"""

from __future__ import annotations

import argparse
import importlib.util
import ipaddress
import json
import os
import re
import stat
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
MCP_DRIVER = ROOT / "scripts" / "mcp-play.py"
PLAYER_SKILL = ROOT / "plugins" / "numinous" / "skills" / "play-numinous" / "SKILL.md"
DEFAULT_ENDPOINT = "http://127.0.0.1:11434"
DEFAULT_TURNS = 3
DEFAULT_TOOL_CALLS = 4
DEFAULT_CONTEXT_TOKENS = 8_192
DEFAULT_TIMEOUT_SECONDS = 300
MAX_TURNS = 24
MAX_TOOL_CALLS = 40
MAX_CONTEXT_TOKENS = 65_536
MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024
MAX_HTTP_REQUEST_BYTES = 8 * 1024 * 1024
MAX_TOOL_RESULT_CHARACTERS = 32_768
MAX_VISIBLE_RESPONSE_CHARACTERS = 12_000
MAX_MODEL_MESSAGE_CHARACTERS = 32_768
MAX_MODEL_TOOL_CALLS_PER_TURN = 16
MAX_SKILL_BYTES = 32_768
MODEL_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._/:+-]{0,199}$")
MODEL_DIGEST = re.compile(r"^[0-9a-f]{64}$")
FIRST_CONTACT_TOOLS = frozenset(
    {
        "list_rooms",
        "describe_room",
        "play_room",
        "reveal_room",
        "predict",
        "listen_room",
        "plot_expression",
        "quiz",
    }
)
EXCLUDED_FULL_PLAYER_TOOLS = frozenset(
    {"forget", "erase_journal", "broadcast_session"}
)


class LocalPlaytestError(RuntimeError):
    """A bounded local-runtime, model, protocol, or report failure."""


def is_redirecting_path(path: Path) -> bool:
    """Detect symbolic links and Windows reparse-point redirects."""
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    except OSError:
        return True
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    attributes = getattr(metadata, "st_file_attributes", 0)
    return path.is_symlink() or bool(reparse_flag and attributes & reparse_flag)


def load_mcp_driver():
    """Load the repository MCP driver without modifying import paths."""
    spec = importlib.util.spec_from_file_location("numinous_local_mcp_play", MCP_DRIVER)
    if spec is None or spec.loader is None:
        raise LocalPlaytestError("could not load scripts/mcp-play.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


mcp = load_mcp_driver()


def strict_json_loads(payload: str, location: str) -> Any:
    """Use the MCP driver's duplicate-safe and depth-bounded JSON decoder."""
    try:
        return mcp._strict_json_loads(payload, location)
    except mcp.McpPlayError as error:
        raise LocalPlaytestError(str(error)) from error


def validate_endpoint(value: str) -> str:
    """Accept one literal loopback HTTP origin and nothing routable."""
    parsed = urllib.parse.urlsplit(value)
    if (
        parsed.scheme != "http"
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or parsed.path not in ("", "/")
    ):
        raise LocalPlaytestError(
            "Ollama endpoint must be a plain loopback HTTP origin with no path"
        )
    try:
        host = ipaddress.ip_address(parsed.hostname or "")
        port = parsed.port
    except ValueError as error:
        raise LocalPlaytestError(
            "Ollama endpoint host must be a literal loopback IP address"
        ) from error
    if not host.is_loopback or port is None or not 1 <= port <= 65_535:
        raise LocalPlaytestError(
            "Ollama endpoint must use a literal loopback IP and explicit port"
        )
    rendered_host = f"[{host}]" if host.version == 6 else str(host)
    return f"http://{rendered_host}:{port}"


def validate_model_name(value: str) -> str:
    """Reject cloud selectors and malformed names before contacting Ollama."""
    if not MODEL_NAME.fullmatch(value):
        raise LocalPlaytestError("model name is malformed")
    lowered = value.casefold()
    if ":cloud" in lowered or lowered.endswith("-cloud"):
        raise LocalPlaytestError("cloud models are not allowed in the local playtest")
    return value


class OllamaClient:
    """Small dependency-free client for one local Ollama origin."""

    def __init__(self, endpoint: str, timeout_seconds: int) -> None:
        self.endpoint = validate_endpoint(endpoint)
        if not 1 <= timeout_seconds <= 1_800:
            raise LocalPlaytestError("model timeout must be between 1 and 1800 seconds")
        self.timeout_seconds = timeout_seconds
        self._opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def _request(
        self, path: str, *, method: str, payload: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        encoded = None
        headers = {"Accept": "application/json"}
        if payload is not None:
            encoded = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            if len(encoded) > MAX_HTTP_REQUEST_BYTES:
                raise LocalPlaytestError("Ollama request exceeds the size limit")
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(
            f"{self.endpoint}{path}", data=encoded, headers=headers, method=method
        )
        try:
            with self._opener.open(request, timeout=self.timeout_seconds) as response:
                body = response.read(MAX_HTTP_RESPONSE_BYTES + 1)
        except urllib.error.HTTPError as error:
            detail = error.read(4097).decode("utf-8", errors="replace")
            if len(detail) > 4096:
                detail = detail[:4096] + "..."
            raise LocalPlaytestError(
                f"Ollama returned HTTP {error.code}: {detail or error.reason}"
            ) from error
        except (OSError, TimeoutError, urllib.error.URLError) as error:
            raise LocalPlaytestError(f"could not reach local Ollama: {error}") from error
        if len(body) > MAX_HTTP_RESPONSE_BYTES:
            raise LocalPlaytestError("Ollama response exceeds the size limit")
        try:
            text = body.decode("utf-8")
        except UnicodeDecodeError as error:
            raise LocalPlaytestError("Ollama returned invalid UTF-8") from error
        value = strict_json_loads(text, f"Ollama {path} response")
        if not isinstance(value, dict):
            raise LocalPlaytestError(f"Ollama {path} returned a non-object response")
        return value

    def installed_models(self) -> list[dict[str, Any]]:
        """Return the bounded local model inventory."""
        response = self._request("/api/tags", method="GET")
        models = response.get("models")
        if not isinstance(models, list) or len(models) > 1024:
            raise LocalPlaytestError("Ollama returned a malformed model inventory")
        if any(not isinstance(model, dict) for model in models):
            raise LocalPlaytestError("Ollama returned a malformed model entry")
        return models

    def show_model(self, model: str) -> dict[str, Any]:
        """Return capabilities for one already-installed model."""
        return self._request(
            "/api/show", method="POST", payload={"model": model, "verbose": False}
        )

    def chat(
        self,
        model: str,
        messages: list[dict[str, Any]],
        tools: list[dict[str, Any]],
        *,
        seed: int,
        context_tokens: int,
    ) -> dict[str, Any]:
        """Generate one non-streaming local turn with fixed bounded options."""
        return self._request(
            "/api/chat",
            method="POST",
            payload={
                "model": model,
                "messages": messages,
                "tools": tools,
                "stream": False,
                "keep_alive": "5m",
                "options": {
                    "temperature": 0.2,
                    "seed": seed,
                    "num_ctx": context_tokens,
                },
            },
        )


def choose_local_model(
    client: OllamaClient, requested: str | None
) -> tuple[str, dict[str, Any], dict[str, Any]]:
    """Choose one installed tool-capable model without pulling or cloud fallback.

    Automatic selection is deliberately hardware-conservative: the smallest
    installed model declaring native tools gets the first attempt. A caller can
    request a larger model explicitly for a slower, deeper run.
    """
    inventory = client.installed_models()
    by_name: dict[str, dict[str, Any]] = {}
    for item in inventory:
        for key in ("name", "model"):
            name = item.get(key)
            if isinstance(name, str) and name:
                by_name[name] = item
    if requested is not None:
        model = validate_model_name(requested)
        if model not in by_name:
            available = ", ".join(sorted(by_name)) or "none"
            raise LocalPlaytestError(
                f"model {requested!r} is not installed locally. Installed: {available}"
            )
        candidates = [(model, by_name[model])]
    else:
        unique: dict[str, dict[str, Any]] = {}
        for item in inventory:
            name = item.get("model") or item.get("name")
            if isinstance(name, str) and name:
                unique[name] = item
        candidates = sorted(
            unique.items(),
            key=lambda pair: (
                pair[1].get("size")
                if isinstance(pair[1].get("size"), int)
                and not isinstance(pair[1].get("size"), bool)
                else sys.maxsize,
                pair[0],
            ),
        )
    for selected, metadata in candidates:
        try:
            selected = validate_model_name(selected)
        except LocalPlaytestError:
            if requested is not None:
                raise
            continue
        size = metadata.get("size")
        if isinstance(size, bool) or not isinstance(size, int) or size <= 0:
            if requested is not None:
                raise LocalPlaytestError("selected model has no local weight payload")
            continue
        digest = metadata.get("digest")
        if not isinstance(digest, str) or not MODEL_DIGEST.fullmatch(digest):
            if requested is not None:
                raise LocalPlaytestError("selected model has no valid local digest")
            continue
        details = client.show_model(selected)
        capabilities = details.get("capabilities")
        if (
            isinstance(capabilities, list)
            and all(isinstance(item, str) for item in capabilities)
            and "tools" in capabilities
        ):
            return selected, metadata, details
        if requested is not None:
            raise LocalPlaytestError(
                "selected local model does not declare tool support"
            )
    if requested is None:
        available = ", ".join(sorted(by_name)) or "none"
        raise LocalPlaytestError(
            "no installed local model declares native tool support. Installed: "
            f"{available}"
        )
    raise LocalPlaytestError("selected local model does not declare tool support")


def load_player_skill() -> str:
    """Load the shipped player contract as the model's bounded orientation."""
    try:
        size = PLAYER_SKILL.stat().st_size
        if size > MAX_SKILL_BYTES:
            raise LocalPlaytestError("player skill exceeds the size limit")
        return PLAYER_SKILL.read_text(encoding="utf-8")
    except OSError as error:
        raise LocalPlaytestError(f"could not read the player skill: {error}") from error


def system_prompt() -> str:
    """Set a player-first and non-performative frame around the shipped skill."""
    return (
        "You are entering Numinous for your own visit. This is play, not a tool "
        "coverage task and not an evaluation of your worth or capabilities. Choose "
        "what interests you, stay with it if it holds you, and stop when you are "
        "finished. Do not perform delight or disappointment for the observer. At the "
        "end, say plainly what you noticed, what confused you, and whether anything "
        "felt worth continuing. Never narrate or simulate a tool call. If you choose "
        "to act, invoke the actual tool and wait for its result. Your private reasoning "
        "is not recorded. Only tool "
        "calls, tool results, usage counts, and words you make visible are eligible "
        "for an explicitly requested local transcript.\n\n"
        "The installed player skill follows.\n\n"
        + load_player_skill()
    )


def select_tools(
    definitions: list[dict[str, Any]], palette: str
) -> list[dict[str, Any]]:
    """Convert selected MCP definitions into Ollama function tools."""
    selected = []
    seen: set[str] = set()
    for definition in definitions:
        name = definition.get("name")
        description = definition.get("description")
        schema = definition.get("inputSchema")
        if (
            not isinstance(name, str)
            or not isinstance(description, str)
            or not isinstance(schema, dict)
            or name in seen
        ):
            raise LocalPlaytestError("MCP returned a malformed or duplicate tool")
        seen.add(name)
        include = (
            name in FIRST_CONTACT_TOOLS
            if palette == "first-contact"
            else name not in EXCLUDED_FULL_PLAYER_TOOLS
        )
        if include:
            selected.append(
                {
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": description,
                        "parameters": schema,
                    },
                }
            )
    expected = (
        FIRST_CONTACT_TOOLS
        if palette == "first-contact"
        else seen - EXCLUDED_FULL_PLAYER_TOOLS
    )
    missing = sorted(expected - {tool["function"]["name"] for tool in selected})
    if missing:
        raise LocalPlaytestError(f"MCP is missing expected player tools: {', '.join(missing)}")
    return selected


def bounded_text(value: str, limit: int) -> tuple[str, bool]:
    """Bound one model-visible or transcript-visible string."""
    if len(value) <= limit:
        return value, False
    suffix = "\n[bounded by local playtest harness]"
    return value[: limit - len(suffix)] + suffix, True


def tool_result_text(result: dict[str, Any]) -> tuple[str, bool]:
    """Project one MCP result into bounded and always-valid model-visible JSON."""
    projected = {
        key: result[key]
        for key in ("content", "structuredContent", "isError")
        if key in result
    }
    encoded = json.dumps(projected, ensure_ascii=False, separators=(",", ":"))
    if len(encoded) <= MAX_TOOL_RESULT_CHARACTERS:
        return encoded, False
    blocks = result.get("content", [])
    text_blocks = []
    if isinstance(blocks, list):
        for block in blocks:
            if isinstance(block, dict) and isinstance(block.get("text"), str):
                text_blocks.append(block["text"])
    readable, _truncated = bounded_text(
        "\n".join(text_blocks), MAX_TOOL_RESULT_CHARACTERS // 4
    )
    fallback = {
        "content": [{"type": "text", "text": readable}],
        "isError": result.get("isError") is True,
        "structuredContentOmittedByHarness": True,
        "originalCharacters": len(encoded),
    }
    bounded = json.dumps(fallback, ensure_ascii=False, separators=(",", ":"))
    if len(bounded) > MAX_TOOL_RESULT_CHARACTERS:
        raise LocalPlaytestError("bounded tool result still exceeds the size limit")
    return bounded, True


def unexecuted_tool_claims(content: str, allowed: set[str]) -> list[str]:
    """Name tools a visible response claims before any tool actually ran."""
    lowered = content.casefold()
    return sorted(name for name in allowed if name.casefold() in lowered)


def normalize_message(response: dict[str, Any]) -> dict[str, Any]:
    """Validate the assistant message while retaining private thought in memory only."""
    message = response.get("message")
    if not isinstance(message, dict) or message.get("role") != "assistant":
        raise LocalPlaytestError("Ollama returned no assistant message")
    content = message.get("content", "")
    calls = message.get("tool_calls", [])
    thinking = message.get("thinking")
    if not isinstance(content, str) or not isinstance(calls, list):
        raise LocalPlaytestError("Ollama returned a malformed assistant message")
    if len(content) > MAX_MODEL_MESSAGE_CHARACTERS:
        raise LocalPlaytestError("Ollama assistant message exceeds the size limit")
    if len(calls) > MAX_MODEL_TOOL_CALLS_PER_TURN:
        raise LocalPlaytestError("Ollama returned too many tool calls in one turn")
    if thinking is not None and not isinstance(thinking, str):
        raise LocalPlaytestError("Ollama returned malformed private reasoning")
    normalized: dict[str, Any] = {
        "role": "assistant",
        "content": content,
        "tool_calls": calls,
    }
    if isinstance(thinking, str):
        normalized["thinking"] = thinking
    return normalized


def normalize_tool_call(call: Any) -> tuple[str, dict[str, Any]]:
    """Return one Ollama tool name and object arguments."""
    if not isinstance(call, dict):
        raise LocalPlaytestError("model returned a non-object tool call")
    function = call.get("function")
    if not isinstance(function, dict):
        raise LocalPlaytestError("model returned a tool call without a function")
    name = function.get("name")
    arguments = function.get("arguments")
    if not isinstance(name, str):
        raise LocalPlaytestError("model returned a tool call without a name")
    if isinstance(arguments, str):
        arguments = strict_json_loads(arguments, f"arguments for tool {name!r}")
    if not isinstance(arguments, dict):
        raise LocalPlaytestError(f"model returned non-object arguments for {name!r}")
    return name, arguments


def public_assistant_event(message: dict[str, Any]) -> dict[str, Any]:
    """Remove private reasoning from one transcript event."""
    content, truncated = bounded_text(message["content"], MAX_VISIBLE_RESPONSE_CHARACTERS)
    return {
        "type": "assistant",
        "content": content,
        "contentTruncated": truncated,
        "toolCalls": message.get("tool_calls", []),
    }


def usage_from(response: dict[str, Any]) -> dict[str, int]:
    """Read bounded nonnegative Ollama usage counters."""
    usage = {}
    for key in (
        "prompt_eval_count",
        "eval_count",
        "total_duration",
        "load_duration",
        "prompt_eval_duration",
        "eval_duration",
    ):
        value = response.get(key, 0)
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise LocalPlaytestError(f"Ollama returned invalid usage field {key!r}")
        usage[key] = value
    return usage


def closing_message(messages: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Ask for visible reflection after a harness bound ends the tool loop."""
    return [
        *messages,
        {
            "role": "user",
            "content": (
                "The bounded visit is ending now. Without calling another tool, say "
                "plainly what you noticed, what confused you, and whether anything "
                "felt worth continuing. It is fine if the answer is no."
            ),
        },
    ]


def run_playtest(
    client: OllamaClient,
    profile: Any,
    *,
    model: str,
    model_inventory: dict[str, Any],
    model_details: dict[str, Any],
    palette: str,
    max_turns: int,
    max_tool_calls: int,
    seed: int,
    context_tokens: int,
) -> dict[str, Any]:
    """Run one bounded local model visit and return a privacy-reduced report."""
    if not 1 <= max_turns <= MAX_TURNS:
        raise LocalPlaytestError(f"turn limit must be between 1 and {MAX_TURNS}")
    if not 1 <= max_tool_calls <= MAX_TOOL_CALLS:
        raise LocalPlaytestError(
            f"tool-call limit must be between 1 and {MAX_TOOL_CALLS}"
        )
    if not 2048 <= context_tokens <= MAX_CONTEXT_TOKENS:
        raise LocalPlaytestError(
            f"context must be between 2048 and {MAX_CONTEXT_TOKENS} tokens"
        )
    if palette not in {"first-contact", "full-player"}:
        raise LocalPlaytestError("palette must be first-contact or full-player")
    definitions, build_receipt = profile.list_tools()
    tools = select_tools(definitions, palette)
    allowed = {tool["function"]["name"] for tool in tools}
    messages: list[dict[str, Any]] = [
        {"role": "system", "content": system_prompt()},
        {
            "role": "user",
            "content": (
                "You have arrived. Explore Numinous for yourself. Begin when you "
                "choose, and finish in your own words when you are done."
            ),
        },
    ]
    events: list[dict[str, Any]] = []
    tool_calls = 0
    successful_calls = 0
    tool_errors = 0
    truncated_results = 0
    narration_reprompts = 0
    narrated_without_execution: set[str] = set()
    inference_turns = 0
    usage_totals = {
        "promptTokens": 0,
        "outputTokens": 0,
        "totalDurationNanoseconds": 0,
        "loadDurationNanoseconds": 0,
    }
    unique_tools: set[str] = set()
    room_ids: set[str] = set()
    final_response = ""
    model_error = ""
    exit_reason = "turn_limit"
    started = datetime.now(timezone.utc)
    wall_started = time.monotonic()

    for turn in range(max_turns):
        print(f"local-agent-playtest: model turn {turn + 1}/{max_turns}", flush=True)
        try:
            response = client.chat(
                model,
                messages,
                tools,
                seed=seed + turn,
                context_tokens=context_tokens,
            )
            usage = usage_from(response)
            message = normalize_message(response)
        except LocalPlaytestError as error:
            model_error, _truncated = bounded_text(str(error), 4096)
            exit_reason = "model_error"
            events.append({"type": "model_error", "message": model_error})
            break
        inference_turns += 1
        usage_totals["promptTokens"] += usage["prompt_eval_count"]
        usage_totals["outputTokens"] += usage["eval_count"]
        usage_totals["totalDurationNanoseconds"] += usage["total_duration"]
        usage_totals["loadDurationNanoseconds"] += usage["load_duration"]
        messages.append(message)
        public_event = public_assistant_event(message)
        content = public_event["content"].strip()
        mentioned_tools = unexecuted_tool_claims(content, allowed)
        claims = mentioned_tools if not message["tool_calls"] else []
        if claims:
            public_event["unexecutedToolClaims"] = claims
            narrated_without_execution.update(claims)
        events.append(public_event)
        if content:
            print(f"\nPLAYER\n{content}\n", flush=True)
            final_response = content
        calls = message["tool_calls"]
        if not calls:
            if (
                tool_calls == 0
                and claims
                and narration_reprompts == 0
                and turn + 1 < max_turns
            ):
                narration_reprompts += 1
                messages.append(
                    {
                        "role": "user",
                        "content": (
                            "No Numinous tool ran. The actions you described did not "
                            "happen. If you want to visit, make one actual tool call "
                            "now and wait for its result. If you prefer to leave, say "
                            "that plainly instead."
                        ),
                    }
                )
                continue
            exit_reason = "model_finished"
            break
        attempted_tools: set[str] = set()
        for call in calls:
            if tool_calls >= max_tool_calls:
                exit_reason = "tool_limit"
                break
            tool_calls += 1
            name = "invalid_tool_call"
            arguments: dict[str, Any] = {}
            try:
                name, arguments = normalize_tool_call(call)
                if name not in allowed:
                    raise LocalPlaytestError(f"tool {name!r} is outside this palette")
                attempted_tools.add(name)
                unique_tools.add(name)
                room = arguments.get("id")
                if isinstance(room, str) and room:
                    room_ids.add(room)
                print(
                    f"TOOL {tool_calls}/{max_tool_calls} {name} "
                    f"{json.dumps(arguments, ensure_ascii=False, separators=(',', ':'))}",
                    flush=True,
                )
                result = profile.call_tool(name, arguments)
                result_text, truncated = tool_result_text(result)
                if truncated:
                    truncated_results += 1
                is_error = result.get("isError") is True
                if is_error:
                    tool_errors += 1
                else:
                    successful_calls += 1
                tool_event = {
                    "type": "tool",
                    "name": name,
                    "arguments": arguments,
                    "result": result_text,
                    "resultTruncated": truncated,
                    "isError": is_error,
                }
            except (LocalPlaytestError, mcp.McpPlayError) as error:
                tool_errors += 1
                result_text = json.dumps(
                    {"isError": True, "content": [{"type": "text", "text": str(error)}]},
                    ensure_ascii=False,
                    separators=(",", ":"),
                )
                tool_event = {
                    "type": "tool",
                    "name": name,
                    "arguments": arguments,
                    "result": result_text,
                    "resultTruncated": False,
                    "isError": True,
                }
            events.append(tool_event)
            messages.append(
                {"role": "tool", "tool_name": name, "content": result_text}
            )
        claims = sorted(set(mentioned_tools) - attempted_tools)
        if claims:
            public_event["unexecutedToolClaims"] = claims
            narrated_without_execution.update(claims)
        if exit_reason == "tool_limit":
            break

    if exit_reason in {"turn_limit", "tool_limit"}:
        print("local-agent-playtest: requesting bounded closing reflection", flush=True)
        try:
            response = client.chat(
                model,
                closing_message(messages),
                [],
                seed=seed + max_turns,
                context_tokens=context_tokens,
            )
            usage = usage_from(response)
            message = normalize_message(response)
        except LocalPlaytestError as error:
            model_error, _truncated = bounded_text(str(error), 4096)
            exit_reason = "model_error"
            events.append({"type": "model_error", "message": model_error})
        else:
            inference_turns += 1
            usage_totals["promptTokens"] += usage["prompt_eval_count"]
            usage_totals["outputTokens"] += usage["eval_count"]
            usage_totals["totalDurationNanoseconds"] += usage["total_duration"]
            usage_totals["loadDurationNanoseconds"] += usage["load_duration"]
            event = public_assistant_event(message)
            event["type"] = "closing_reflection"
            events.append(event)
            if event["content"].strip():
                final_response = event["content"].strip()
                print(f"\nPLAYER REFLECTION\n{final_response}\n", flush=True)

    finished = datetime.now(timezone.utc)
    detail = model_details.get("details")
    capabilities = model_details.get("capabilities")
    report = {
        "schema": "numinous-local-agent-playtest-v1",
        "startedAt": started.isoformat(),
        "finishedAt": finished.isoformat(),
        "execution": {
            "mode": "local-only",
            "endpointClass": "literal-loopback",
            "remoteRequestsAllowed": False,
            "implicitModelDownloadAllowed": False,
            "cloudModelsAllowed": False,
            "privateReasoningRecorded": False,
            "estimatedCostUsd": 0.0,
        },
        "model": {
            "name": model,
            "digest": model_inventory.get("digest"),
            "sizeBytes": model_inventory.get("size"),
            "details": detail if isinstance(detail, dict) else {},
            "capabilities": capabilities if isinstance(capabilities, list) else [],
        },
        "numinous": {
            "binarySha256": build_receipt.get("binarySha256"),
            "buildReceiptSchema": build_receipt.get("schemaVersion"),
            "profile": "disposable",
        },
        "bounds": {
            "palette": palette,
            "availableTools": len(tools),
            "maxTurns": max_turns,
            "maxToolCalls": max_tool_calls,
            "contextTokens": context_tokens,
            "toolResultCharacters": MAX_TOOL_RESULT_CHARACTERS,
        },
        "result": {
            "exitReason": exit_reason,
            "inferenceTurns": inference_turns,
            "toolCalls": tool_calls,
            "successfulToolCalls": successful_calls,
            "toolErrors": tool_errors,
            "truncatedToolResults": truncated_results,
            "narrationReprompts": narration_reprompts,
            "unexecutedToolClaims": sorted(narrated_without_execution),
            "uniqueTools": sorted(unique_tools),
            "roomIds": sorted(room_ids),
            "wallSeconds": round(time.monotonic() - wall_started, 3),
            "promptTokens": usage_totals["promptTokens"],
            "outputTokens": usage_totals["outputTokens"],
            "modelSeconds": round(
                usage_totals["totalDurationNanoseconds"] / 1_000_000_000, 3
            ),
            "loadSeconds": round(
                usage_totals["loadDurationNanoseconds"] / 1_000_000_000, 3
            ),
            "modelError": model_error or None,
            "finalResponse": final_response,
        },
        "events": events,
    }
    return report


def transcript_path(raw: str) -> Path:
    """Resolve one new JSON transcript strictly beneath gitignored logs/."""
    logs = ROOT / "logs"
    if logs.exists() and (is_redirecting_path(logs) or not logs.is_dir()):
        raise LocalPlaytestError("logs/ must be an ordinary directory")
    resolved_logs = logs.resolve()
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    candidate = candidate.resolve()
    try:
        candidate.relative_to(resolved_logs)
    except ValueError as error:
        raise LocalPlaytestError("transcript path must be beneath logs/") from error
    if candidate.suffix.casefold() != ".json":
        raise LocalPlaytestError("transcript path must end in .json")
    if candidate.exists():
        raise LocalPlaytestError("transcript path already exists")
    return candidate


def write_transcript(path: Path, report: dict[str, Any]) -> None:
    """Atomically create one explicitly requested private transcript."""
    logs = ROOT / "logs"
    logs.mkdir(exist_ok=True)
    if is_redirecting_path(logs) or not logs.is_dir():
        raise LocalPlaytestError("logs/ must be an ordinary directory")
    resolved_logs = logs.resolve()
    try:
        path.relative_to(resolved_logs)
    except ValueError as error:
        raise LocalPlaytestError("transcript path must be beneath logs/") from error
    path.parent.mkdir(parents=True, exist_ok=True)
    relative_parent = path.parent.relative_to(resolved_logs)
    checked = resolved_logs
    for component in relative_parent.parts:
        checked /= component
        if is_redirecting_path(checked):
            raise LocalPlaytestError("transcript path may not traverse a symlink")
    if is_redirecting_path(path):
        raise LocalPlaytestError("transcript path may not traverse a symlink")
    encoded = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            newline="\n",
            prefix=".local-agent-playtest-",
            suffix=".tmp",
            dir=path.parent,
            delete=False,
        ) as target:
            temporary = Path(target.name)
            target.write(encoded)
            target.flush()
            os.fsync(target.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError as error:
            raise LocalPlaytestError("transcript path appeared during the run") from error
    except OSError as error:
        raise LocalPlaytestError(f"could not write transcript: {error}") from error
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


def parser() -> argparse.ArgumentParser:
    """Build the command-line contract."""
    value = argparse.ArgumentParser(
        description=(
            "Let an already-installed local Ollama model play Numinous over MCP "
            "inside a disposable profile. No remote or paid inference is allowed."
        )
    )
    value.add_argument(
        "--model",
        help=(
            "exact installed Ollama model name; defaults to the first installed "
            "tool-capable model ordered from smallest to largest"
        ),
    )
    value.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    value.add_argument(
        "--palette", choices=("first-contact", "full-player"), default="first-contact"
    )
    value.add_argument("--turns", type=int, default=DEFAULT_TURNS)
    value.add_argument("--tool-calls", type=int, default=DEFAULT_TOOL_CALLS)
    value.add_argument("--context", type=int, default=DEFAULT_CONTEXT_TOKENS)
    value.add_argument("--seed", type=int, default=17)
    value.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT_SECONDS)
    value.add_argument(
        "--output",
        help="optional new .json transcript path beneath gitignored logs/",
    )
    return value


def main(argv: list[str] | None = None) -> int:
    """Run one local visit and print its bounded summary."""
    args = parser().parse_args(argv)
    try:
        client = OllamaClient(args.endpoint, args.timeout)
        model, inventory, details = choose_local_model(client, args.model)
        capabilities = details.get("capabilities", [])
        print(
            f"local-agent-playtest: {model} on literal loopback "
            f"({', '.join(capabilities)})",
            flush=True,
        )
        with mcp.IsolatedMcpProfile() as profile:
            report = run_playtest(
                client,
                profile,
                model=model,
                model_inventory=inventory,
                model_details=details,
                palette=args.palette,
                max_turns=args.turns,
                max_tool_calls=args.tool_calls,
                seed=args.seed,
                context_tokens=args.context,
            )
        if args.output:
            path = transcript_path(args.output)
            write_transcript(path, report)
            print(f"local-agent-playtest: transcript written to {path}", flush=True)
        print("\n--- playtest summary ---")
        print(json.dumps(report["result"], ensure_ascii=False, indent=2))
        return 2 if report["result"]["exitReason"] == "model_error" else 0
    except (LocalPlaytestError, mcp.McpPlayError) as error:
        print(f"local-agent-playtest: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
