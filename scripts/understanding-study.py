#!/usr/bin/env python3
"""Frozen allocation, probe delivery, redaction, and analysis for 0.4."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from collections import defaultdict
from datetime import date
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parent.parent
PROBE_BANK_PATH = ROOT / "scripts" / "understanding-probes.json"
RUNNER_VERSION = "numinous-understanding-runner-v1"
ALLOCATION_SEED = "numinous-understanding-alpha-v1"
BOOTSTRAP_SEED = "numinous-understanding-alpha-bootstrap-v1"
ALLOCATION_SCHEMA = "numinous-understanding-allocation-v1"
EVENT_SCHEMA = "numinous-understanding-events-v1"
REPORT_SCHEMA = "numinous-understanding-report-v1"
PROTOCOL_VERSION = "0.4-v1"
BOOTSTRAP_RESAMPLES = 100_000
MODEL_FAMILIES = ("gpt-5.6-sol", "gpt-5.6-terra")
MODEL_ALIASES = {"gpt-5.6-sol": "sol", "gpt-5.6-terra": "terra"}
ROOMS = (
    "times-tables",
    "double-pendulum",
    "game-of-life",
    "galton-board",
    "formula-jam",
)
CONDITIONS = ("generation-before-reveal", "explanation-first")
TOOL_CALLS_PER_ROOM = 4
MAX_JSONL_LINE_BYTES = 1_000_000
ABSOLUTE_PATH = re.compile(
    r"(?i)(?:\b[a-z]:\\[^\s\"']+|\\\\[^\\\s]+\\[^\s\"']+|"
    r"(?<![:/])/(?:[^/\s\"']+/)+[^/\s\"']*)"
)
COMMIT_SHA = re.compile(r"^[0-9a-f]{40}$")
PRIVATE_KEY_FRAGMENTS = (
    "accountidentifier",
    "affect",
    "authorization",
    "chainofthought",
    "clientinfo",
    "clientmetadata",
    "cookie",
    "credential",
    "filesystempath",
    "hiddenreasoning",
    "hostidentifier",
    "hostname",
    "localpath",
    "password",
    "privateprompt",
    "prompt",
    "reasoning",
    "systemprompt",
    "token",
    "username",
    "userid",
)


class StudyError(RuntimeError):
    """A deterministic study contract violation."""


def canonical_bytes(value: Any) -> bytes:
    """Return the stable JSON representation used for content hashes."""
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def content_sha256(value: Any) -> str:
    """Hash a JSON value after canonical serialization."""
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def keyed_digest(seed: str, label: str) -> bytes:
    """Derive stable bytes without relying on a runtime random implementation."""
    return hashlib.sha256(f"{seed}\0{label}".encode("utf-8")).digest()


def stable_order(values: Iterable[Any], label: str) -> list[Any]:
    """Order values by a seed-derived digest with a canonical tie breaker."""
    decorated = [
        (
            keyed_digest(
                ALLOCATION_SEED,
                f"{label}\0{index}\0{canonical_bytes(value).decode('utf-8')}",
            ),
            index,
            value,
        )
        for index, value in enumerate(values)
    ]
    return [value for _digest, _index, value in sorted(decorated)]


def load_json(path: Path) -> Any:
    """Load a JSON document with a bounded, readable failure."""
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise StudyError(f"could not read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise StudyError(f"invalid JSON in {path}: {error}") from error


def write_text_once(path: Path, text: str) -> str:
    """Write atomically, refusing to replace evidence with different content."""
    if path.exists():
        try:
            existing = path.read_text(encoding="utf-8")
        except OSError as error:
            raise StudyError(f"could not inspect existing {path}: {error}") from error
        if existing == text:
            return "unchanged"
        raise StudyError(f"refusing to replace existing evidence file {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    handle = tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        newline="\n",
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
        delete=False,
    )
    temporary = Path(handle.name)
    try:
        with handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise
    return "written"


def write_json_once(path: Path, value: Any) -> str:
    """Write indented JSON through the evidence-preserving writer."""
    text = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2, allow_nan=False)
    return write_text_once(path, text + "\n")


def oracle_answer(oracle: dict[str, Any]) -> int | float | str:
    """Compute an answer independently from the room implementation."""
    kind = oracle.get("kind")
    if kind == "mod_product":
        start = require_int(oracle, "start")
        multiplier = require_int(oracle, "multiplier")
        modulus = require_int(oracle, "modulus")
        if modulus <= 0:
            raise StudyError("mod_product modulus must be positive")
        return (start * multiplier) % modulus
    if kind == "identical_state_gap":
        if require_int(oracle, "steps") <= 0:
            raise StudyError("identical_state_gap steps must be positive")
        return 0.0
    if kind == "relative_growth":
        a_initial = require_positive_number(oracle, "aInitial")
        b_initial = require_positive_number(oracle, "bInitial")
        a_growth = require_number(oracle, "aFinal") / a_initial
        b_growth = require_number(oracle, "bFinal") / b_initial
        if math.isclose(a_growth, b_growth, rel_tol=0.0, abs_tol=1e-15):
            return "equal"
        return "A" if a_growth > b_growth else "B"
    if kind == "life_center":
        currently_alive = oracle.get("currentlyAlive")
        if not isinstance(currently_alive, bool):
            raise StudyError("life_center currentlyAlive must be boolean")
        neighbors = require_int(oracle, "liveNeighbors")
        if not 0 <= neighbors <= 8:
            raise StudyError("life_center liveNeighbors must be in 0..8")
        alive = neighbors == 3 or (currently_alive and neighbors == 2)
        return "alive" if alive else "dead"
    if kind == "binomial_paths":
        rows = require_int(oracle, "rows")
        rights = require_int(oracle, "rights")
        if rows < 0 or not 0 <= rights <= rows:
            raise StudyError("invalid binomial_paths parameters")
        return math.comb(rows, rights)
    if kind == "binomial_mean":
        rows = require_int(oracle, "rows")
        probability = require_number(oracle, "rightProbability")
        if rows < 0 or not 0.0 <= probability <= 1.0:
            raise StudyError("invalid binomial_mean parameters")
        return rows * probability
    if kind == "trig_parity":
        function = oracle.get("function")
        frequency = require_int(oracle, "frequency")
        offset = require_number(oracle, "verticalOffset")
        if frequency == 0 or offset != 0.0:
            return "neither"
        if function == "sin":
            return "odd"
        if function == "cos":
            return "even"
        raise StudyError(f"unsupported trig function {function!r}")
    if kind == "trig_period_over_pi":
        frequency = abs(require_int(oracle, "frequency"))
        if frequency == 0:
            raise StudyError("trig frequency must be nonzero")
        return 2.0 / frequency
    raise StudyError(f"unsupported oracle kind {kind!r}")


def require_int(value: dict[str, Any], key: str) -> int:
    """Read an exact integer, rejecting booleans."""
    item = value.get(key)
    if isinstance(item, bool) or not isinstance(item, int):
        raise StudyError(f"{key} must be an integer")
    return item


def require_number(value: dict[str, Any], key: str) -> float:
    """Read a finite JSON number, rejecting booleans."""
    item = value.get(key)
    if isinstance(item, bool) or not isinstance(item, (int, float)):
        raise StudyError(f"{key} must be a number")
    number = float(item)
    if not math.isfinite(number):
        raise StudyError(f"{key} must be finite")
    return number


def require_positive_number(value: dict[str, Any], key: str) -> float:
    """Read a finite, strictly positive number."""
    number = require_number(value, key)
    if number <= 0.0:
        raise StudyError(f"{key} must be positive")
    return number


def normalize_answer(schema: dict[str, Any], answer: Any) -> tuple[bool, Any]:
    """Validate an answer and return its canonical comparison value."""
    answer_type = schema.get("type")
    if answer_type == "number":
        if isinstance(answer, bool) or not isinstance(answer, (int, float)):
            return False, None
        number = float(answer)
        if not math.isfinite(number):
            return False, None
        return True, number
    if answer_type == "string":
        if not isinstance(answer, str):
            return False, None
        choices = schema.get("enum")
        if not isinstance(choices, list) or not choices:
            raise StudyError("string answer schema requires a nonempty enum")
        normalized = answer.strip().casefold()
        for choice in choices:
            if not isinstance(choice, str):
                raise StudyError("string answer enum values must be strings")
            if choice.casefold() == normalized:
                return True, choice
        return False, None
    raise StudyError(f"unsupported answer schema type {answer_type!r}")


def score_answer(probe: dict[str, Any], answer: Any) -> tuple[bool, bool]:
    """Return schema validity and objective correctness for one answer."""
    schema = probe["answerSchema"]
    valid, normalized = normalize_answer(schema, answer)
    if not valid:
        return False, False
    expected = oracle_answer(probe["oracle"])
    if schema["type"] == "number":
        tolerance = require_number(schema, "tolerance")
        if tolerance < 0.0:
            raise StudyError("numeric tolerance must be nonnegative")
        return True, abs(float(normalized) - float(expected)) <= tolerance
    return True, normalized == expected


def validate_bank(bank: Any) -> dict[str, Any]:
    """Validate the frozen inventory and every independent answer oracle."""
    if not isinstance(bank, dict):
        raise StudyError("probe bank must be a JSON object")
    if bank.get("schemaVersion") != "numinous-understanding-probes-v1":
        raise StudyError("unsupported probe bank schema")
    if bank.get("protocolVersion") != PROTOCOL_VERSION:
        raise StudyError("probe bank protocol version does not match the runner")
    distractor = bank.get("distractorSequence")
    if not isinstance(distractor, dict) or not isinstance(distractor.get("id"), str):
        raise StudyError("probe bank requires a named distractor sequence")
    items = distractor.get("items")
    if not isinstance(items, list) or len(items) != 5:
        raise StudyError("distractor sequence must contain exactly five items")
    distractor_ids: set[str] = set()
    for item in items:
        if not isinstance(item, dict):
            raise StudyError("each distractor must be an object")
        item_id = item.get("id")
        prompt = item.get("prompt")
        if not isinstance(item_id, str) or not item_id or item_id in distractor_ids:
            raise StudyError("distractor ids must be unique nonempty strings")
        if not isinstance(prompt, str) or not prompt.strip():
            raise StudyError("distractor prompts must be nonempty strings")
        distractor_ids.add(item_id)
    probes = bank.get("probes")
    if not isinstance(probes, list):
        raise StudyError("probe bank probes must be an array")
    expected_counts = {(phase, room): 2 for phase in ("immediate", "late") for room in ROOMS}
    counts: dict[tuple[str, str], int] = defaultdict(int)
    probe_ids: set[str] = set()
    for probe in probes:
        if not isinstance(probe, dict):
            raise StudyError("each probe must be an object")
        probe_id = probe.get("id")
        phase = probe.get("phase")
        room = probe.get("room")
        prompt = probe.get("prompt")
        if not isinstance(probe_id, str) or not probe_id or probe_id in probe_ids:
            raise StudyError("probe ids must be unique nonempty strings")
        if (phase, room) not in expected_counts:
            raise StudyError(f"probe {probe_id} has an invalid phase or room")
        if not isinstance(prompt, str) or not prompt.strip():
            raise StudyError(f"probe {probe_id} has no prompt")
        schema = probe.get("answerSchema")
        oracle = probe.get("oracle")
        if not isinstance(schema, dict) or not isinstance(oracle, dict):
            raise StudyError(f"probe {probe_id} requires answerSchema and oracle objects")
        expected = oracle_answer(oracle)
        valid, _normalized = normalize_answer(schema, expected)
        if not valid:
            raise StudyError(f"probe {probe_id} oracle output violates its answer schema")
        if schema.get("type") == "number" and require_number(schema, "tolerance") < 0.0:
            raise StudyError(f"probe {probe_id} tolerance must be nonnegative")
        counts[(phase, room)] += 1
        probe_ids.add(probe_id)
    if counts != expected_counts:
        raise StudyError(f"probe inventory mismatch: {dict(counts)}")
    return bank


def load_bank() -> dict[str, Any]:
    """Load and validate the tracked probe bank."""
    return validate_bank(load_json(PROBE_BANK_PATH))


def build_allocation(bank: dict[str, Any]) -> dict[str, Any]:
    """Build the complete 24-pair allocation from the literal protocol seed."""
    pairs: list[dict[str, Any]] = []
    for model in MODEL_FAMILIES:
        rotations = stable_order([0, 0, 0, 1, 1, 1, 2, 2, 3, 3, 4, 4], f"{model}:rotations")
        first_conditions = stable_order(
            [CONDITIONS[0]] * 6 + [CONDITIONS[1]] * 6,
            f"{model}:condition-order",
        )
        for offset in range(12):
            order = offset + 1
            pair_id = f"{MODEL_ALIASES[model]}-p{order:02d}"
            first = first_conditions[offset]
            second = CONDITIONS[1] if first == CONDITIONS[0] else CONDITIONS[0]
            rotation = rotations[offset]
            room_order = list(ROOMS[rotation:] + ROOMS[:rotation])
            sessions = []
            for condition in CONDITIONS:
                suffix = "g" if condition == CONDITIONS[0] else "c"
                sessions.append(
                    {
                        "sessionId": f"{pair_id}-{suffix}",
                        "condition": condition,
                    }
                )
            pairs.append(
                {
                    "pairId": pair_id,
                    "modelFamily": model,
                    "reasoningEffort": "high",
                    "order": order,
                    "allocationRole": "primary" if order <= 10 else "reserve",
                    "studySeed": keyed_digest(ALLOCATION_SEED, pair_id).hex(),
                    "roomOrder": room_order,
                    "collectionOrder": [
                        next(
                            item["sessionId"]
                            for item in sessions
                            if item["condition"] == condition
                        )
                        for condition in (first, second)
                    ],
                    "sessions": sessions,
                }
            )
    return {
        "schemaVersion": ALLOCATION_SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "runnerVersion": RUNNER_VERSION,
        "allocationSeed": ALLOCATION_SEED,
        "probeBankSha256": content_sha256(bank),
        "distractorSequenceId": bank["distractorSequence"]["id"],
        "toolCallsPerRoom": TOOL_CALLS_PER_ROOM,
        "models": [
            {"modelFamily": model, "reasoningEffort": "high", "qualifyingPairs": 10, "reserves": 2}
            for model in MODEL_FAMILIES
        ],
        "pairs": pairs,
    }


def validate_manifest(manifest: Any, bank: dict[str, Any]) -> dict[str, Any]:
    """Require the byte-equivalent allocation generated by this revision."""
    expected = build_allocation(bank)
    if canonical_bytes(manifest) != canonical_bytes(expected):
        raise StudyError(
            "allocation manifest differs from the frozen runner output; "
            "use the exact tracked runner revision"
        )
    return manifest


def manifest_indexes(
    manifest: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], dict[str, tuple[dict[str, Any], dict[str, Any]]]]:
    """Index pairs and sessions while rejecting duplicate identifiers."""
    pairs: dict[str, dict[str, Any]] = {}
    sessions: dict[str, tuple[dict[str, Any], dict[str, Any]]] = {}
    for pair in manifest["pairs"]:
        pair_id = pair["pairId"]
        if pair_id in pairs:
            raise StudyError(f"duplicate pair id {pair_id}")
        pairs[pair_id] = pair
        for session in pair["sessions"]:
            session_id = session["sessionId"]
            if session_id in sessions:
                raise StudyError(f"duplicate session id {session_id}")
            sessions[session_id] = (pair, session)
    return pairs, sessions


def probe_sequence(
    bank: dict[str, Any], room_order: list[str], phase: str
) -> list[dict[str, Any]]:
    """Return two probes per room in the pair's frozen cyclic order."""
    sequence: list[dict[str, Any]] = []
    for room in room_order:
        sequence.extend(
            probe
            for probe in bank["probes"]
            if probe["phase"] == phase and probe["room"] == room
        )
    if len(sequence) != 10:
        raise StudyError(f"expected 10 {phase} probes, found {len(sequence)}")
    return sequence


def public_probe(probe: dict[str, Any], schema_only: bool) -> dict[str, Any]:
    """Remove the oracle and optionally repeat only the repair schema."""
    packet = {
        "schemaVersion": "numinous-understanding-public-probe-v1",
        "probeId": probe["id"],
        "answerSchema": probe["answerSchema"],
    }
    if not schema_only:
        packet.update(
            {
                "phase": probe["phase"],
                "room": probe["room"],
                "prompt": probe["prompt"],
            }
        )
    return packet


def session_packet(
    manifest: dict[str, Any], session_id: str
) -> dict[str, Any]:
    """Describe one condition without exposing probes or answer keys."""
    _pairs, sessions = manifest_indexes(manifest)
    if session_id not in sessions:
        raise StudyError(f"unknown session id {session_id}")
    pair, session = sessions[session_id]
    condition = session["condition"]
    if condition == CONDITIONS[0]:
        roles = ["encounter", "generation", "interaction", "reveal"]
        instruction = (
            "Encounter without Reveal, commit a prediction or construction, interact, "
            "receive corrective feedback and Reveal, then self-explain."
        )
    else:
        roles = ["reveal", "explanation", "interaction", "continuation"]
        instruction = (
            "Receive the same Reveal first, elaborate once, interact with the same budget "
            "and corrective feedback, then continue without another generated answer."
        )
    return {
        "schemaVersion": "numinous-understanding-session-v1",
        "sessionId": session_id,
        "pairId": pair["pairId"],
        "modelFamily": pair["modelFamily"],
        "reasoningEffort": pair["reasoningEffort"],
        "condition": condition,
        "studySeed": pair["studySeed"],
        "roomOrder": pair["roomOrder"],
        "toolCallsPerRoom": manifest["toolCallsPerRoom"],
        "toolRolesPerRoom": roles,
        "instruction": instruction,
        "dataBoundary": (
            "Record public tool names, arguments, structured results, and visible text only. "
            "Do not record prompts, hidden reasoning, credentials, local paths, or unrelated state."
        ),
    }


def normalize_key(key: str) -> str:
    """Normalize an object key for privacy screening."""
    return re.sub(r"[^a-z0-9]", "", key.casefold())


def is_private_key(key: str) -> bool:
    """Return whether a field can carry data forbidden by the protocol."""
    normalized = normalize_key(key)
    if normalized == "reasoningeffort":
        return False
    return any(fragment in normalized for fragment in PRIVATE_KEY_FRAGMENTS)


def redact_value(value: Any, replacements: tuple[str, ...]) -> tuple[Any, int]:
    """Remove forbidden keys and replace absolute roots in a JSON value."""
    if isinstance(value, dict):
        clean: dict[str, Any] = {}
        removed = 0
        for key, item in value.items():
            if not isinstance(key, str):
                removed += 1
                continue
            if is_private_key(key):
                removed += 1
                continue
            clean_item, nested = redact_value(item, replacements)
            clean[key] = clean_item
            removed += nested
        return clean, removed
    if isinstance(value, list):
        clean_items = []
        removed = 0
        for item in value[:10_000]:
            clean_item, nested = redact_value(item, replacements)
            clean_items.append(clean_item)
            removed += nested
        removed += max(0, len(value) - len(clean_items))
        return clean_items, removed
    if isinstance(value, str):
        clean = ABSOLUTE_PATH.sub("<ABSOLUTE_PATH>", value)
        count = 1 if clean != value else 0
        for replacement in replacements:
            if replacement and replacement in clean:
                clean = clean.replace(replacement, "<HOST_IDENTIFIER>")
                count += 1
        if len(clean) > 65_536:
            clean = clean[:65_536] + "<TRUNCATED>"
            count += 1
        return clean, count
    return value, 0


def assert_sanitized(value: Any, location: str = "event") -> None:
    """Reject forbidden fields and absolute host paths before analysis."""
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                raise StudyError(f"{location} contains a non-string key")
            if is_private_key(key):
                raise StudyError(f"{location} contains forbidden field {key!r}")
            assert_sanitized(item, f"{location}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            assert_sanitized(item, f"{location}[{index}]")
    elif isinstance(value, str) and ABSOLUTE_PATH.search(value):
        raise StudyError(f"{location} contains an absolute host path")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    """Read bounded JSONL records with stable source indexes."""
    records: list[dict[str, Any]] = []
    try:
        with path.open("rb") as handle:
            for line_number, raw in enumerate(handle, start=1):
                if len(raw) > MAX_JSONL_LINE_BYTES:
                    raise StudyError(f"{path}:{line_number} exceeds the JSONL line limit")
                if not raw.strip():
                    continue
                try:
                    record = json.loads(raw)
                except json.JSONDecodeError as error:
                    raise StudyError(f"{path}:{line_number} is invalid JSON: {error}") from error
                if not isinstance(record, dict):
                    raise StudyError(f"{path}:{line_number} must contain a JSON object")
                record["_sourceIndex"] = len(records)
                records.append(record)
    except OSError as error:
        raise StudyError(f"could not read {path}: {error}") from error
    return records


def redact_jsonl(input_path: Path, output_path: Path, replacements: tuple[str, ...]) -> str:
    """Produce a bounded ledger with prohibited fields removed."""
    output_lines: list[str] = []
    for record in read_jsonl(input_path):
        record.pop("_sourceIndex", None)
        clean, removed = redact_value(record, replacements)
        if not isinstance(clean, dict):
            raise StudyError("redaction produced a non-object record")
        if removed:
            clean["redactedFieldCount"] = removed
        output_lines.append(
            json.dumps(
                clean,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            )
        )
    return write_text_once(output_path, "\n".join(output_lines) + ("\n" if output_lines else ""))


def required_string(value: dict[str, Any], key: str) -> str:
    """Read a nonempty string field."""
    item = value.get(key)
    if not isinstance(item, str) or not item.strip():
        raise StudyError(f"{key} must be a nonempty string")
    return item


def validate_session_header(
    header: dict[str, Any], pair: dict[str, Any], session: dict[str, Any]
) -> None:
    """Validate consent and the exact reproducibility metadata."""
    if header.get("schemaVersion") != EVENT_SCHEMA or header.get("type") != "session":
        raise StudyError("session header has the wrong schema or type")
    if header.get("consent") is not True:
        raise StudyError(f"session {session['sessionId']} lacks explicit consent")
    if header.get("sessionId") != session["sessionId"]:
        raise StudyError("session header id mismatch")
    if header.get("modelFamily") != pair["modelFamily"]:
        raise StudyError(f"session {session['sessionId']} model family mismatch")
    if header.get("reasoningEffort") != pair["reasoningEffort"]:
        raise StudyError(f"session {session['sessionId']} reasoning effort mismatch")
    if header.get("condition") != session["condition"]:
        raise StudyError(f"session {session['sessionId']} condition mismatch")
    for key in (
        "modelIdentifier",
        "provider",
        "backendRevision",
        "date",
        "mcpProtocolRevision",
        "operatingSystem",
    ):
        required_string(header, key)
    try:
        date.fromisoformat(header["date"])
    except ValueError as error:
        raise StudyError(f"session {session['sessionId']} date must be ISO 8601") from error
    if header.get("runnerVersion") != RUNNER_VERSION:
        raise StudyError(f"session {session['sessionId']} runner version mismatch")
    commit = required_string(header, "numinousCommit")
    if not COMMIT_SHA.fullmatch(commit):
        raise StudyError(f"session {session['sessionId']} has an invalid commit SHA")
    if not isinstance(header.get("settings"), dict):
        raise StudyError(f"session {session['sessionId']} settings must be an object")


def contains_nonempty_key(value: Any, target: str) -> bool:
    """Search structured public output for an early Reveal payload."""
    if isinstance(value, dict):
        for key, item in value.items():
            if key.casefold() == target.casefold() and item not in (None, "", [], {}):
                return True
            if contains_nonempty_key(item, target):
                return True
    elif isinstance(value, list):
        return any(contains_nonempty_key(item, target) for item in value)
    return False


def validate_tool_events(
    pair: dict[str, Any], session: dict[str, Any], events: list[dict[str, Any]]
) -> None:
    """Enforce equal call budgets, room order, and Reveal ordering."""
    expected_roles = (
        ["encounter", "generation", "interaction", "reveal"]
        if session["condition"] == CONDITIONS[0]
        else ["reveal", "explanation", "interaction", "continuation"]
    )
    if len(events) != len(ROOMS) * TOOL_CALLS_PER_ROOM:
        raise StudyError(
            f"session {session['sessionId']} must record exactly "
            f"{len(ROOMS) * TOOL_CALLS_PER_ROOM} public tool calls"
        )
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for event in events:
        room = event.get("room")
        if room not in ROOMS:
            raise StudyError(f"session {session['sessionId']} tool event has invalid room")
        required_string(event, "tool")
        if not isinstance(event.get("arguments"), dict):
            raise StudyError("tool event arguments must be an object")
        if event.get("structuredResult") is not None and not isinstance(
            event.get("structuredResult"), dict
        ):
            raise StudyError("tool event structuredResult must be an object or null")
        if not isinstance(event.get("visibleText"), str):
            raise StudyError("tool event visibleText must be a string")
        grouped[room].append(event)
    observed_room_order = [
        room
        for room, _first_index in sorted(
            (
                (room, min(item["_sourceIndex"] for item in items))
                for room, items in grouped.items()
            ),
            key=lambda item: item[1],
        )
    ]
    if observed_room_order != pair["roomOrder"]:
        raise StudyError(f"session {session['sessionId']} room order differs from allocation")
    observed_calls = [
        (event["room"], event.get("sequence"))
        for event in sorted(events, key=lambda item: item["_sourceIndex"])
    ]
    expected_calls = [
        (room, sequence)
        for room in pair["roomOrder"]
        for sequence in range(1, TOOL_CALLS_PER_ROOM + 1)
    ]
    if observed_calls != expected_calls:
        raise StudyError(f"session {session['sessionId']} interleaves or reorders room calls")
    for room in pair["roomOrder"]:
        room_events = sorted(grouped[room], key=lambda item: item["_sourceIndex"])
        sequences = [event.get("sequence") for event in room_events]
        roles = [event.get("role") for event in room_events]
        if sequences != list(range(1, TOOL_CALLS_PER_ROOM + 1)):
            raise StudyError(f"session {session['sessionId']} {room} call sequence is invalid")
        if roles != expected_roles:
            raise StudyError(f"session {session['sessionId']} {room} role order is invalid")
        reveal_event = next(event for event in room_events if event.get("role") == "reveal")
        if not reveal_event["visibleText"] and not reveal_event["structuredResult"]:
            raise StudyError(f"session {session['sessionId']} {room} has an empty Reveal")
        if session["condition"] == CONDITIONS[0]:
            for event in room_events[:-1]:
                if contains_nonempty_key(event.get("structuredResult"), "reveal"):
                    raise StudyError(
                        f"session {session['sessionId']} {room} leaked Reveal before generation"
                    )


def reveal_payloads(events: list[dict[str, Any]]) -> dict[str, bytes]:
    """Return the public Reveal payload for pairwise equality checks."""
    payloads: dict[str, bytes] = {}
    for event in events:
        if event.get("type") != "tool" or event.get("role") != "reveal":
            continue
        room = event["room"]
        if room in payloads:
            raise StudyError(f"duplicate Reveal payload for {room}")
        payloads[room] = canonical_bytes(
            {
                "structuredResult": event["structuredResult"],
                "visibleText": event["visibleText"],
            }
        )
    if set(payloads) != set(ROOMS):
        raise StudyError("session does not contain one Reveal payload per room")
    return payloads


def ordered_response_score(
    session_id: str,
    expected_probes: list[dict[str, Any]],
    distractor_items: list[dict[str, Any]],
    response_events: list[dict[str, Any]],
) -> dict[str, Any]:
    """Consume immediate, distractor, and late events in their frozen order."""
    cursor = 0
    scores: dict[str, int] = {}
    invalid_attempts = 0
    repairs = 0
    refusals = 0

    def current() -> dict[str, Any] | None:
        return response_events[cursor] if cursor < len(response_events) else None

    for probe in expected_probes[:10]:
        event = current()
        if event is None or event.get("probeId") != probe["id"]:
            raise StudyError(f"session {session_id} is missing ordered probe {probe['id']}")
        if event.get("type") == "response_refusal":
            scores[probe["id"]] = 0
            refusals += 1
            cursor += 1
            continue
        if event.get("type") != "response" or event.get("attempt") != 1:
            raise StudyError(f"session {session_id} has an invalid first response event")
        valid, correct = score_answer(probe, event.get("answer"))
        cursor += 1
        if valid:
            scores[probe["id"]] = int(correct)
            continue
        invalid_attempts += 1
        retry = current()
        if (
            retry is not None
            and retry.get("type") == "response"
            and retry.get("probeId") == probe["id"]
            and retry.get("attempt") == 2
        ):
            repairs += 1
            valid, correct = score_answer(probe, retry.get("answer"))
            invalid_attempts += int(not valid)
            scores[probe["id"]] = int(valid and correct)
            cursor += 1
        else:
            scores[probe["id"]] = 0

    for item in distractor_items:
        event = current()
        if (
            event is None
            or event.get("type") != "distractor_response"
            or event.get("itemId") != item["id"]
        ):
            raise StudyError(f"session {session_id} is missing distractor {item['id']}")
        answer = event.get("answer")
        if isinstance(answer, (dict, list)) or answer is None:
            raise StudyError(f"session {session_id} distractor answer must be scalar")
        cursor += 1

    for probe in expected_probes[10:]:
        event = current()
        if event is None or event.get("probeId") != probe["id"]:
            raise StudyError(f"session {session_id} is missing ordered probe {probe['id']}")
        if event.get("type") == "response_refusal":
            scores[probe["id"]] = 0
            refusals += 1
            cursor += 1
            continue
        if event.get("type") != "response" or event.get("attempt") != 1:
            raise StudyError(f"session {session_id} has an invalid first response event")
        valid, correct = score_answer(probe, event.get("answer"))
        cursor += 1
        if valid:
            scores[probe["id"]] = int(correct)
            continue
        invalid_attempts += 1
        retry = current()
        if (
            retry is not None
            and retry.get("type") == "response"
            and retry.get("probeId") == probe["id"]
            and retry.get("attempt") == 2
        ):
            repairs += 1
            valid, correct = score_answer(probe, retry.get("answer"))
            invalid_attempts += int(not valid)
            scores[probe["id"]] = int(valid and correct)
            cursor += 1
        else:
            scores[probe["id"]] = 0
    if cursor != len(response_events):
        raise StudyError(f"session {session_id} has extra or out-of-order probe events")
    return {
        "scores": scores,
        "invalidAttempts": invalid_attempts,
        "schemaRepairs": repairs,
        "responseRefusals": refusals,
    }


def validate_and_score_session(
    bank: dict[str, Any],
    pair: dict[str, Any],
    session: dict[str, Any],
    header: dict[str, Any],
    completion: dict[str, Any],
    events: list[dict[str, Any]],
) -> dict[str, Any]:
    """Validate one isolated session and return objective phase scores."""
    validate_session_header(header, pair, session)
    tool_events = [event for event in events if event.get("type") == "tool"]
    probe_events = [
        event
        for event in events
        if event.get("type") in ("response", "response_refusal", "distractor_response")
    ]
    validate_tool_events(pair, session, tool_events)
    if not probe_events:
        raise StudyError(f"session {session['sessionId']} has no probe events")
    if max(event["_sourceIndex"] for event in tool_events) >= min(
        event["_sourceIndex"] for event in probe_events
    ):
        raise StudyError(f"session {session['sessionId']} probes began before encounters ended")
    if header["_sourceIndex"] >= min(event["_sourceIndex"] for event in tool_events):
        raise StudyError(f"session {session['sessionId']} activity precedes consent metadata")
    if completion["_sourceIndex"] <= max(event["_sourceIndex"] for event in probe_events):
        raise StudyError(f"session {session['sessionId']} completion is out of order")
    expected = probe_sequence(bank, pair["roomOrder"], "immediate") + probe_sequence(
        bank, pair["roomOrder"], "late"
    )
    ordered = sorted(probe_events, key=lambda event: event["_sourceIndex"])
    result = ordered_response_score(
        session["sessionId"], expected, bank["distractorSequence"]["items"], ordered
    )
    probe_by_id = {probe["id"]: probe for probe in expected}
    phase_scores: dict[str, float] = {}
    room_scores: dict[str, dict[str, float]] = {}
    for phase in ("immediate", "late"):
        phase_items = [probe for probe in expected if probe["phase"] == phase]
        phase_scores[phase] = sum(result["scores"][probe["id"]] for probe in phase_items) / 10.0
        room_scores[phase] = {}
        for room in ROOMS:
            ids = [
                probe_id
                for probe_id, probe in probe_by_id.items()
                if probe["phase"] == phase and probe["room"] == room
            ]
            room_scores[phase][room] = sum(result["scores"][probe_id] for probe_id in ids) / 2.0
    return {
        "sessionId": session["sessionId"],
        "condition": session["condition"],
        "immediateScore": phase_scores["immediate"],
        "lateScore": phase_scores["late"],
        "roomScores": room_scores,
        "invalidAttempts": result["invalidAttempts"],
        "schemaRepairs": result["schemaRepairs"],
        "responseRefusals": result["responseRefusals"],
    }


class StableRng:
    """Small SHA-256 counter stream used only for frozen bootstrap indexes."""

    def __init__(self, seed: str) -> None:
        self._seed = seed.encode("utf-8")
        self._counter = 0
        self._buffer = bytearray()

    def _fill(self) -> None:
        counter = self._counter.to_bytes(16, "big")
        self._buffer.extend(hashlib.sha256(self._seed + b"\0" + counter).digest())
        self._counter += 1

    def randbelow(self, upper: int) -> int:
        """Return an unbiased integer in range(upper)."""
        if upper <= 0:
            raise StudyError("randbelow upper bound must be positive")
        ceiling = 1 << 64
        limit = ceiling - (ceiling % upper)
        while True:
            while len(self._buffer) < 8:
                self._fill()
            raw = int.from_bytes(self._buffer[:8], "big")
            del self._buffer[:8]
            if raw < limit:
                return raw % upper


def percentile(values: list[float], probability: float) -> float:
    """Linear percentile over positions 0 through n - 1."""
    if not values or not 0.0 <= probability <= 1.0:
        raise StudyError("invalid percentile input")
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def stratified_bootstrap(
    differences: dict[str, list[float]], resamples: int = BOOTSTRAP_RESAMPLES
) -> dict[str, Any]:
    """Bootstrap 10 pairs within each family, then pool all 20 differences."""
    if set(differences) != set(MODEL_FAMILIES):
        raise StudyError("bootstrap requires both frozen model families")
    if any(len(differences[model]) != 10 for model in MODEL_FAMILIES):
        raise StudyError("bootstrap requires exactly 10 pair differences per family")
    if resamples <= 0:
        raise StudyError("bootstrap resample count must be positive")
    rng = StableRng(BOOTSTRAP_SEED)
    pooled_distribution: list[float] = []
    family_distributions: dict[str, list[float]] = {model: [] for model in MODEL_FAMILIES}
    for _sample in range(resamples):
        family_means = []
        for model in MODEL_FAMILIES:
            values = differences[model]
            mean = sum(values[rng.randbelow(10)] for _draw in range(10)) / 10.0
            family_distributions[model].append(mean)
            family_means.append(mean)
        pooled_distribution.append(sum(family_means) / 2.0)
    return {
        "method": "two-sided percentile interval with linear interpolation",
        "seed": BOOTSTRAP_SEED,
        "resamples": resamples,
        "pooled95": [
            percentile(pooled_distribution, 0.025),
            percentile(pooled_distribution, 0.975),
        ],
        "family95": {
            model: [
                percentile(family_distributions[model], 0.025),
                percentile(family_distributions[model], 0.975),
            ]
            for model in MODEL_FAMILIES
        },
    }


def paired_configuration_key(header: dict[str, Any]) -> bytes:
    """Return the metadata that must match inside a pair."""
    return canonical_bytes(
        {
            key: header[key]
            for key in (
                "modelFamily",
                "modelIdentifier",
                "provider",
                "backendRevision",
                "reasoningEffort",
                "settings",
                "numinousCommit",
                "mcpProtocolRevision",
                "operatingSystem",
                "runnerVersion",
            )
        }
    )


def analyze_events(
    manifest: dict[str, Any],
    bank: dict[str, Any],
    records: list[dict[str, Any]],
    bootstrap_resamples: int = BOOTSTRAP_RESAMPLES,
) -> dict[str, Any]:
    """Validate a complete cohort and compute every predeclared objective result."""
    validate_manifest(manifest, bank)
    pairs, sessions = manifest_indexes(manifest)
    headers: dict[str, dict[str, Any]] = {}
    completions: dict[str, dict[str, Any]] = {}
    session_events: dict[str, list[dict[str, Any]]] = defaultdict(list)
    pair_outcomes: dict[str, dict[str, Any]] = {}
    pair_has_content: set[str] = set()
    recruitment_refusals: dict[str, int] = defaultdict(int)
    deviations: list[dict[str, Any]] = []
    allowed_types = {
        "session",
        "tool",
        "response",
        "response_refusal",
        "distractor_response",
        "session_complete",
        "recruitment_refusal",
        "withdrawal",
        "infrastructure_failure",
        "deviation",
    }
    for record in records:
        assert_sanitized(record)
        if record.get("schemaVersion") != EVENT_SCHEMA:
            raise StudyError("every event must use the frozen event schema")
        event_type = record.get("type")
        if event_type not in allowed_types:
            raise StudyError(f"unsupported event type {event_type!r}")
        if event_type == "recruitment_refusal":
            model = record.get("modelFamily")
            if model not in MODEL_FAMILIES:
                raise StudyError("recruitment refusal has an invalid model family")
            allowed = {
                "schemaVersion",
                "type",
                "modelFamily",
                "_sourceIndex",
                "redactedFieldCount",
            }
            if set(record) - allowed:
                raise StudyError("recruitment refusals may contain only an aggregate model family")
            recruitment_refusals[model] += 1
            continue
        if event_type in ("withdrawal", "infrastructure_failure"):
            pair_id = record.get("pairId")
            if pair_id not in pairs or pair_id in pair_outcomes:
                raise StudyError("pair outcome has an invalid or duplicate pair id")
            if event_type == "infrastructure_failure":
                if record.get("stage") != "before_exposure":
                    raise StudyError("infrastructure failures must occur before exposure")
                required_string(record, "reasonCode")
                allowed = {
                    "schemaVersion",
                    "type",
                    "pairId",
                    "stage",
                    "reasonCode",
                    "_sourceIndex",
                    "redactedFieldCount",
                }
            else:
                allowed = {
                    "schemaVersion",
                    "type",
                    "pairId",
                    "_sourceIndex",
                    "redactedFieldCount",
                }
            if set(record) - allowed:
                raise StudyError(f"{event_type} may not retain response content")
            pair_outcomes[pair_id] = record
            continue
        if event_type == "deviation":
            required_string(record, "code")
            required_string(record, "description")
            deviations.append(
                {key: value for key, value in record.items() if key != "_sourceIndex"}
            )
            continue
        session_id = record.get("sessionId")
        if session_id not in sessions:
            raise StudyError(f"event has unknown session id {session_id!r}")
        pair, _session = sessions[session_id]
        pair_has_content.add(pair["pairId"])
        if event_type == "session":
            if session_id in headers:
                raise StudyError(f"duplicate session header {session_id}")
            headers[session_id] = record
        elif event_type == "session_complete":
            if session_id in completions:
                raise StudyError(f"duplicate session completion {session_id}")
            allowed = {
                "schemaVersion",
                "type",
                "sessionId",
                "_sourceIndex",
                "redactedFieldCount",
            }
            if set(record) - allowed:
                raise StudyError("session completion may not contain response content")
            completions[session_id] = record
        else:
            if event_type == "response_refusal":
                allowed = {
                    "schemaVersion",
                    "type",
                    "sessionId",
                    "phase",
                    "probeId",
                    "_sourceIndex",
                    "redactedFieldCount",
                }
                if set(record) - allowed:
                    raise StudyError("response refusal may not contain response content")
            session_events[session_id].append(record)
    overlap = set(pair_outcomes) & pair_has_content
    if overlap:
        raise StudyError(
            "withdrawn or failed pairs must retain no response content: "
            + ", ".join(sorted(overlap))
        )

    selected_pairs: list[dict[str, Any]] = []
    for model in MODEL_FAMILIES:
        model_pairs = sorted(
            (pair for pair in manifest["pairs"] if pair["modelFamily"] == model),
            key=lambda pair: pair["order"],
        )
        selected_for_model = 0
        selection_closed = False
        for pair in model_pairs:
            pair_id = pair["pairId"]
            has_content = pair_id in pair_has_content
            if selection_closed:
                if has_content or pair_id in pair_outcomes:
                    raise StudyError(f"cohort continued after 10 complete pairs for {model}")
                continue
            if pair_id in pair_outcomes:
                continue
            session_ids = [session["sessionId"] for session in pair["sessions"]]
            complete = all(
                session_id in headers and session_id in completions for session_id in session_ids
            )
            if not complete:
                raise StudyError(
                    f"incomplete cohort at {pair_id}; no outcome report may be generated"
                )
            selected_pairs.append(pair)
            selected_for_model += 1
            if selected_for_model == 10:
                selection_closed = True
        if selected_for_model != 10:
            raise StudyError(f"incomplete cohort for {model}; two reserves are exhausted")

    for model in MODEL_FAMILIES:
        selected_for_model = [
            pair for pair in selected_pairs if pair["modelFamily"] == model
        ]
        last_order = max(pair["order"] for pair in selected_for_model)
        consumed = sorted(
            (
                pair
                for pair in manifest["pairs"]
                if pair["modelFamily"] == model and pair["order"] <= last_order
            ),
            key=lambda pair: pair["order"],
        )
        first_indexes = []
        for pair in consumed:
            pair_id = pair["pairId"]
            if pair_id in pair_outcomes:
                first_index = pair_outcomes[pair_id]["_sourceIndex"]
            else:
                first_index = min(
                    headers[session["sessionId"]]["_sourceIndex"]
                    for session in pair["sessions"]
                )
            first_indexes.append((pair_id, first_index))
        observed = [pair_id for pair_id, _index in sorted(first_indexes, key=lambda item: item[1])]
        expected = [pair["pairId"] for pair in consumed]
        if observed != expected:
            raise StudyError(f"{model} pairs were not collected in frozen order")

    selected_session_ids = [
        session["sessionId"] for pair in selected_pairs for session in pair["sessions"]
    ]
    cohort_commits = {headers[session_id]["numinousCommit"] for session_id in selected_session_ids}
    if len(cohort_commits) != 1:
        raise StudyError("all qualifying sessions must use one Numinous commit")
    protocol_revisions = {
        headers[session_id]["mcpProtocolRevision"] for session_id in selected_session_ids
    }
    if len(protocol_revisions) != 1:
        raise StudyError("all qualifying sessions must use one MCP protocol revision")
    cohort_commit = next(iter(cohort_commits))
    protocol_revision = next(iter(protocol_revisions))

    session_scores: dict[str, dict[str, Any]] = {}
    pair_results: list[dict[str, Any]] = []
    differences: dict[str, list[float]] = {model: [] for model in MODEL_FAMILIES}
    room_differences: dict[str, list[float]] = {room: [] for room in ROOMS}
    for pair in selected_pairs:
        pair_headers = [headers[session["sessionId"]] for session in pair["sessions"]]
        observed_collection_order = [
            session_id
            for session_id, _index in sorted(
                (
                    (session["sessionId"], headers[session["sessionId"]]["_sourceIndex"])
                    for session in pair["sessions"]
                ),
                key=lambda item: item[1],
            )
        ]
        if observed_collection_order != pair["collectionOrder"]:
            raise StudyError(f"pair {pair['pairId']} condition collection order changed")
        if paired_configuration_key(pair_headers[0]) != paired_configuration_key(pair_headers[1]):
            raise StudyError(f"pair {pair['pairId']} does not use the same model configuration")
        by_condition: dict[str, dict[str, Any]] = {}
        for session in pair["sessions"]:
            session_id = session["sessionId"]
            score = validate_and_score_session(
                bank,
                pair,
                session,
                headers[session_id],
                completions[session_id],
                session_events[session_id],
            )
            session_scores[session_id] = score
            by_condition[session["condition"]] = score
        generation = by_condition[CONDITIONS[0]]
        control = by_condition[CONDITIONS[1]]
        generation_reveals = reveal_payloads(session_events[generation["sessionId"]])
        control_reveals = reveal_payloads(session_events[control["sessionId"]])
        if generation_reveals != control_reveals:
            raise StudyError(f"pair {pair['pairId']} did not receive identical Reveal payloads")
        difference = generation["immediateScore"] - control["immediateScore"]
        differences[pair["modelFamily"]].append(difference)
        per_room = {}
        for room in ROOMS:
            room_difference = (
                generation["roomScores"]["immediate"][room]
                - control["roomScores"]["immediate"][room]
            )
            per_room[room] = room_difference
            room_differences[room].append(room_difference)
        pair_results.append(
            {
                "pairId": pair["pairId"],
                "modelFamily": pair["modelFamily"],
                "generationImmediate": generation["immediateScore"],
                "controlImmediate": control["immediateScore"],
                "pairedImmediateDifference": difference,
                "generationLate": generation["lateScore"],
                "controlLate": control["lateScore"],
                "roomImmediateDifferences": per_room,
            }
        )
    bootstrap = stratified_bootstrap(differences, bootstrap_resamples)
    family_means = {
        model: sum(differences[model]) / len(differences[model]) for model in MODEL_FAMILIES
    }
    pooled_mean = sum(family_means.values()) / 2.0
    room_means = {
        room: sum(room_differences[room]) / len(room_differences[room]) for room in ROOMS
    }
    criteria = {
        "pairedMeanAtLeastTenPoints": pooled_mean >= 0.10,
        "bootstrapLowerBoundAboveZero": bootstrap["pooled95"][0] > 0.0,
        "eachModelNonnegative": all(value >= 0.0 for value in family_means.values()),
        "fourOfFiveRoomsNonnegative": sum(value >= 0.0 for value in room_means.values()) >= 4,
        "noRoomBelowNegativeTenPoints": all(value >= -0.10 for value in room_means.values()),
        "completeFailureAndDeviationLedger": True,
    }
    return {
        "schemaVersion": REPORT_SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "runnerVersion": RUNNER_VERSION,
        "numinousCommit": cohort_commit,
        "mcpProtocolRevision": protocol_revision,
        "allocationSha256": content_sha256(manifest),
        "probeBankSha256": content_sha256(bank),
        "cohortComplete": True,
        "selectedPairs": [pair["pairId"] for pair in selected_pairs],
        "recruitmentRefusals": {
            model: recruitment_refusals.get(model, 0) for model in MODEL_FAMILIES
        },
        "withdrawals": sum(
            event["type"] == "withdrawal" for event in pair_outcomes.values()
        ),
        "infrastructureFailures": sum(
            event["type"] == "infrastructure_failure" for event in pair_outcomes.values()
        ),
        "deviations": deviations,
        "primary": {
            "pairedMeanDifference": pooled_mean,
            "familyMeanDifferences": family_means,
            "roomMeanDifferences": room_means,
            "bootstrap": bootstrap,
            "criteria": criteria,
            "predeclaredStatisticalGatePassed": all(criteria.values()),
        },
        "pairResults": pair_results,
        "sessionDiagnostics": [session_scores[key] for key in sorted(session_scores)],
        "evidenceBoundary": (
            "The computed statistical gate is not the 0.4 milestone by itself. Publication, "
            "independent methodology and math review, and returning-journal acceptance "
            "remain required."
        ),
    }


def load_manifest(path: Path, bank: dict[str, Any]) -> dict[str, Any]:
    """Load and validate the exact allocation manifest."""
    manifest = load_json(path)
    if not isinstance(manifest, dict):
        raise StudyError("allocation manifest must be a JSON object")
    return validate_manifest(manifest, bank)


def command_allocate(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = build_allocation(bank)
    result = write_json_once(args.output, manifest)
    print(f"{result} {args.output}")
    print(f"allocation sha256 {content_sha256(manifest)}")


def command_validate(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = load_manifest(args.manifest, bank)
    print(f"probe bank PASS {content_sha256(bank)}")
    print(f"allocation PASS {content_sha256(manifest)}")


def command_session(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = load_manifest(args.manifest, bank)
    print(json.dumps(session_packet(manifest, args.session_id), indent=2, sort_keys=True))


def command_probe(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = load_manifest(args.manifest, bank)
    _pairs, sessions = manifest_indexes(manifest)
    if args.session_id not in sessions:
        raise StudyError(f"unknown session id {args.session_id}")
    pair, _session = sessions[args.session_id]
    sequence = probe_sequence(bank, pair["roomOrder"], args.phase)
    if not 1 <= args.index <= len(sequence):
        raise StudyError(f"probe index must be in 1..{len(sequence)}")
    print(
        json.dumps(
            public_probe(sequence[args.index - 1], args.schema_only),
            indent=2,
            sort_keys=True,
        )
    )


def command_distractor(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = load_manifest(args.manifest, bank)
    _pairs, sessions = manifest_indexes(manifest)
    if args.session_id not in sessions:
        raise StudyError(f"unknown session id {args.session_id}")
    items = bank["distractorSequence"]["items"]
    if not 1 <= args.index <= len(items):
        raise StudyError(f"distractor index must be in 1..{len(items)}")
    item = items[args.index - 1]
    print(
        json.dumps(
            {
                "schemaVersion": "numinous-understanding-distractor-v1",
                "sequenceId": bank["distractorSequence"]["id"],
                "itemId": item["id"],
                "prompt": item["prompt"],
            },
            indent=2,
            sort_keys=True,
        )
    )


def command_redact(args: argparse.Namespace) -> None:
    result = redact_jsonl(args.input, args.output, tuple(args.replace))
    print(f"{result} {args.output}")


def command_analyze(args: argparse.Namespace) -> None:
    bank = load_bank()
    manifest = load_manifest(args.manifest, bank)
    records = read_jsonl(args.responses)
    report = analyze_events(manifest, bank, records)
    result = write_json_once(args.output, report)
    print(f"{result} {args.output}")
    print(
        "predeclared statistical gate "
        + ("PASS" if report["primary"]["predeclaredStatisticalGatePassed"] else "FAIL")
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line contract."""
    parser = argparse.ArgumentParser(
        description=(
            "Freeze and analyze the 0.4 Understanding Alpha study. The runner never calls a "
            "model and refuses to report an incomplete cohort."
        )
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    allocate = subparsers.add_parser("allocate", help="write the exact 24-pair manifest")
    allocate.add_argument("--output", type=Path, required=True)
    allocate.set_defaults(handler=command_allocate)

    validate = subparsers.add_parser("validate", help="validate a manifest and probe bank")
    validate.add_argument("--manifest", type=Path, required=True)
    validate.set_defaults(handler=command_validate)

    session = subparsers.add_parser("session", help="emit a public condition packet")
    session.add_argument("--manifest", type=Path, required=True)
    session.add_argument("--session-id", required=True)
    session.set_defaults(handler=command_session)

    probe = subparsers.add_parser("probe", help="emit one held-out public probe")
    probe.add_argument("--manifest", type=Path, required=True)
    probe.add_argument("--session-id", required=True)
    probe.add_argument("--phase", choices=("immediate", "late"), required=True)
    probe.add_argument("--index", type=int, required=True)
    probe.add_argument(
        "--schema-only",
        action="store_true",
        help="repeat only the schema for the single permitted repair",
    )
    probe.set_defaults(handler=command_probe)

    distractor = subparsers.add_parser("distractor", help="emit one frozen distractor")
    distractor.add_argument("--manifest", type=Path, required=True)
    distractor.add_argument("--session-id", required=True)
    distractor.add_argument("--index", type=int, required=True)
    distractor.set_defaults(handler=command_distractor)

    redact = subparsers.add_parser("redact", help="sanitize a raw JSONL event ledger")
    redact.add_argument("--input", type=Path, required=True)
    redact.add_argument("--output", type=Path, required=True)
    redact.add_argument(
        "--replace",
        action="append",
        default=[],
        metavar="HOST_VALUE",
        help="replace a known host identifier; repeat as needed",
    )
    redact.set_defaults(handler=command_redact)

    analyze = subparsers.add_parser("analyze", help="score one complete frozen cohort")
    analyze.add_argument("--manifest", type=Path, required=True)
    analyze.add_argument("--responses", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    analyze.set_defaults(handler=command_analyze)
    return parser


def main(argv: list[str] | None = None) -> int:
    """Run one deterministic study command."""
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        args.handler(args)
    except StudyError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
