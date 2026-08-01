#!/usr/bin/env python3
"""Mediate one stateful Understanding Alpha session and seal its public receipts."""

from __future__ import annotations

import argparse
import contextlib
import importlib.util
import json
import math
import os
import platform
import re
import secrets
import subprocess
import sys
import tempfile
import threading
import time
from datetime import date
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parent.parent
AGENT_ROOT = ROOT / ".agent"
STATE_SCHEMA = "numinous-understanding-collector-state-v3"
ACTIVE_SCHEMA = "numinous-understanding-active-session-v2"
LOCK_SCHEMA = "numinous-understanding-ledger-lock-v1"
TRANSITION_SCHEMA = "numinous-understanding-transition-lock-v1"
RECOVERY_SCHEMA = "numinous-understanding-recovery-lock-v1"
MAX_RESPONSE_BYTES = 4096
MAX_STATE_BYTES = 5_000_000
MAX_MARKER_BYTES = 16_384
MAX_RECEIPT_TRANSACTION_BYTES = 128_000_000
RECEIPT_TRANSACTION_SCHEMA = "numinous-understanding-receipt-transaction-v1"
CONSENT_TEXT = (
    "Participation is voluntary. Collector-managed storage records bounded public MCP "
    "outputs, bounded answers and rationales, model and provider identity, backend "
    "revision, runtime "
    "settings, date, repository, source, protocol, opaque context, and independently "
    "witnessed start-receipt commitments, operating system class, and capability policy, "
    "but never hidden reasoning. Every response "
    "packet accepts terminalAction stop; the separate withdrawal packet accepts "
    "terminalAction withdraw until pair aggregation. Stop erases participant-authored "
    "answers and rationales but retains consent metadata, bounded public encounter and "
    "material receipts, source-bound build evidence, an explicit content-erasure marker, "
    "and the adverse interruption receipt. Withdrawal erases both "
    "provisional arms and consumes the pair. Choose "
    "aggregate-only to suppress session diagnostics and pair-level results, or bounded-raw "
    "to permit their publication. Collector erasure cannot remove provider logs, operator "
    "captures, terminal scrollback, participant-owned copies, or other storage outside the "
    "collector boundary. Declining creates no response content; the cohort retains only an "
    "aggregate model-family refusal count, while the already recorded opaque start receipt "
    "remains for attempt-completeness reconciliation."
)


def load_local_module(name: str, path: Path):
    """Load one repository script without changing its CLI filename."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


study = load_local_module(
    "numinous_understanding_study", ROOT / "scripts" / "understanding-study.py"
)
mcp_play = load_local_module("numinous_mcp_play", ROOT / "scripts" / "mcp-play.py")
TRANSITION_WAIT_SECONDS = (
    mcp_play.BUILD_TIMEOUT_SECONDS + mcp_play.SERVER_TIMEOUT_SECONDS + 30
)
THREAD_MUTEX_GUARD = threading.Lock()
THREAD_MUTEXES: dict[Path, threading.RLock] = {}


class CollectorError(RuntimeError):
    """A bounded collection-state, participant-input, or MCP failure."""


ToolCaller = Callable[[str, dict[str, Any]], tuple[dict[str, Any], dict[str, Any]]]


def require_agent_path(path: Path, label: str) -> Path:
    """Keep private banks, mutable state, and raw receipts under ignored .agent."""
    resolved = path.resolve()
    try:
        resolved.relative_to(AGENT_ROOT.resolve())
    except ValueError as error:
        raise CollectorError(f"{label} must be inside {AGENT_ROOT}") from error
    return resolved


def require_distinct_paths(paths: dict[str, Path]) -> None:
    """Reject aliases and names reserved for collector-owned sidecars."""
    observed: dict[Path, str] = {}
    reserved_suffixes = (
        ".lock",
        ".active",
        ".transition",
        ".recovery",
        ".anchor.json",
        ".transaction.json",
        ".tmp",
        ".pending",
    )
    for label, path in paths.items():
        resolved = path.resolve()
        if resolved in observed:
            raise CollectorError(f"{label} aliases {observed[resolved]}")
        observed[resolved] = label
        if label in {"collector state", "collector ledger"} and resolved.name.endswith(
            reserved_suffixes
        ):
            raise CollectorError(f"{label} uses a reserved sidecar name")


def write_json_atomic(path: Path, value: Any, description: str) -> None:
    """Replace one JSON artifact atomically after a durable same-directory stage."""
    try:
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
    except OSError as error:
        raise CollectorError(f"could not stage {description} {path}: {error}") from error
    temporary = Path(handle.name)
    try:
        with handle:
            json.dump(value, handle, ensure_ascii=False, sort_keys=True, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except OSError as error:
        raise CollectorError(f"could not persist {description} {path}: {error}") from error
    finally:
        try:
            temporary.unlink(missing_ok=True)
        except OSError as cleanup_error:
            if sys.exception() is None:
                raise CollectorError(
                    f"could not remove temporary {description} {temporary}: "
                    f"{cleanup_error}"
                ) from cleanup_error


def write_state(path: Path, state: dict[str, Any]) -> None:
    """Replace mutable working state atomically inside .agent."""
    write_json_atomic(path, state, "collector state")


def write_state_once(path: Path, state: dict[str, Any]) -> None:
    """Publish initial state atomically without replacing a concurrent winner."""
    payload = json.dumps(
        state,
        ensure_ascii=False,
        sort_keys=True,
        indent=2,
        allow_nan=False,
    ) + "\n"
    result = study.write_text_once(path, payload)
    if result != "written":
        raise CollectorError(f"collector state already exists: {path}")


def remove_state_if_exact(path: Path, expected: dict[str, Any]) -> None:
    """Remove an unclaimed initial state only when its complete content is still ours."""
    if not path.exists():
        return
    observed = load_state(path)
    if study.canonical_bytes(observed) != study.canonical_bytes(expected):
        raise CollectorError("unclaimed collector state changed ownership")
    remove_path(path, "unclaimed collector state")


def validate_manifest_snapshot(manifest: Any) -> dict[str, Any]:
    """Validate the frozen allocation shape without requiring the private bank."""
    expected_fields = {
        "schemaVersion",
        "protocolVersion",
        "runnerVersion",
        "calibrationRunnerRevision",
        "calibrationRunnerSourceSha256",
        "allocationSeed",
        "probeBankSha256",
        "calibrationAudit",
        "encounterSpecSha256",
        "distractorSequenceId",
        "toolCallsPerRoom",
        "maximumReserveConditionOrderImbalance",
        "maximumReserveFirstRoomCountRange",
        "models",
        "pairs",
    }
    if not isinstance(manifest, dict) or set(manifest) != expected_fields:
        raise CollectorError("collector allocation snapshot schema differs")
    calibration_audit = manifest["calibrationAudit"]
    if not isinstance(calibration_audit, dict):
        raise CollectorError("collector allocation snapshot metadata differs")
    calibration_provenance = calibration_audit.get("provenance")
    if not isinstance(calibration_provenance, dict):
        raise CollectorError("collector allocation snapshot metadata differs")
    audit_fields = {
        "schemaVersion",
        "probeBankSha256",
        "rules",
        "provenance",
        "complete",
        "passed",
        "replacementProbeIds",
        "items",
        "rawAnswersIncluded",
    }
    provenance_fields = {
        "responseRecordCount",
        "distinctFreshContextCount",
        "distinctAttemptStartReceiptCount",
        "contextSetSha256",
        "attemptStartReceiptSetSha256",
        "responseRecordSetSha256",
        "deliveryLedgerSha256",
        "relevanceRecordSetSha256",
        "reviewerIds",
        "backendRevisions",
        "collectionDates",
        "reasoningEffort",
        "capabilityPolicy",
        "runnerVersion",
        "runnerRevision",
        "runnerSourceSha256",
        "oneAttemptPerCell",
    }
    items = calibration_audit.get("items")
    backend_revisions = calibration_provenance.get("backendRevisions")
    if (
        set(calibration_audit) != audit_fields
        or set(calibration_provenance) != provenance_fields
        or not isinstance(items, list)
        or not items
        or not isinstance(backend_revisions, dict)
        or set(backend_revisions) != set(study.MODEL_FAMILIES)
        or any(
            not isinstance(revisions, list)
            or len(revisions) != 1
            or not isinstance(revisions[0], str)
            or not 1 <= len(revisions[0]) <= 256
            for revisions in backend_revisions.values()
        )
    ):
        raise CollectorError("collector allocation snapshot metadata differs")
    expected_calibration_count = (
        len(items)
        * len(study.MODEL_FAMILIES)
        * study.CALIBRATION_REPLICATES_PER_MODEL
    )
    if (
        manifest["schemaVersion"] != study.ALLOCATION_SCHEMA
        or manifest["protocolVersion"] != study.PROTOCOL_VERSION
        or manifest["runnerVersion"] != study.RUNNER_VERSION
        or not isinstance(manifest["calibrationRunnerRevision"], str)
        or not study.COMMIT_SHA.fullmatch(manifest["calibrationRunnerRevision"])
        or not isinstance(manifest["calibrationRunnerSourceSha256"], str)
        or not study.SHA256_HEX.fullmatch(
            manifest["calibrationRunnerSourceSha256"]
        )
        or manifest["calibrationRunnerRevision"]
        != calibration_provenance.get("runnerRevision")
        or manifest["calibrationRunnerSourceSha256"]
        != calibration_provenance.get("runnerSourceSha256")
        or manifest["allocationSeed"] != study.ALLOCATION_SEED
        or manifest["toolCallsPerRoom"] != study.TOOL_CALLS_PER_ROOM
        or manifest["maximumReserveConditionOrderImbalance"] != 2
        or manifest["maximumReserveFirstRoomCountRange"] != 3
        or not isinstance(manifest["distractorSequenceId"], str)
        or not manifest["distractorSequenceId"]
        or any(
            not isinstance(manifest[key], str)
            or not study.SHA256_HEX.fullmatch(manifest[key])
            for key in ("probeBankSha256", "encounterSpecSha256")
        )
        or calibration_audit.get("schemaVersion")
        != "numinous-understanding-calibration-audit-v5"
        or calibration_audit.get("probeBankSha256")
        != manifest["probeBankSha256"]
        or calibration_audit.get("complete") is not True
        or calibration_audit.get("passed") is not True
        or calibration_audit.get("replacementProbeIds") != []
        or calibration_audit.get("rawAnswersIncluded") is not False
        or calibration_provenance.get("responseRecordCount")
        != expected_calibration_count
        or calibration_provenance.get("distinctFreshContextCount")
        != expected_calibration_count
        or calibration_provenance.get("distinctAttemptStartReceiptCount")
        != expected_calibration_count
        or any(
            not isinstance(calibration_provenance.get(field), str)
            or not study.SHA256_HEX.fullmatch(calibration_provenance[field])
            for field in (
                "contextSetSha256",
                "attemptStartReceiptSetSha256",
                "responseRecordSetSha256",
                "deliveryLedgerSha256",
                "relevanceRecordSetSha256",
            )
        )
    ):
        raise CollectorError("collector allocation snapshot metadata differs")
    expected_models = [
        {
            "modelFamily": model,
            "modelIdentifier": model,
            "provider": study.MODEL_PROVIDERS[model],
            "calibratedBackendRevision": backend_revisions[model][0],
            "reasoningEffort": "high",
            "qualifyingPairs": 10,
            "reserves": 2,
        }
        for model in study.MODEL_FAMILIES
    ]
    if manifest["models"] != expected_models:
        raise CollectorError("collector allocation snapshot models differ")
    pairs = manifest["pairs"]
    if not isinstance(pairs, list) or len(pairs) != 24:
        raise CollectorError("collector allocation snapshot pair count differs")
    pair_fields = {
        "pairId",
        "modelFamily",
        "calibratedBackendRevision",
        "studySourceSha256",
        "reasoningEffort",
        "order",
        "allocationRole",
        "studySeed",
        "roomOrder",
        "collectionOrder",
        "sessions",
    }
    seen: set[str] = set()
    for pair in pairs:
        if not isinstance(pair, dict) or set(pair) != pair_fields:
            raise CollectorError("collector allocation snapshot pair schema differs")
        model = pair["modelFamily"]
        order = pair["order"]
        if (
            model not in study.MODEL_FAMILIES
            or isinstance(order, bool)
            or not isinstance(order, int)
            or not 1 <= order <= 12
        ):
            raise CollectorError("collector allocation snapshot pair identity differs")
        alias = study.MODEL_ALIASES[model]
        pair_id = f"{alias}-p{order:02d}"
        expected_sessions = [
            {"sessionId": f"{pair_id}-g", "condition": study.CONDITIONS[0]},
            {"sessionId": f"{pair_id}-c", "condition": study.CONDITIONS[1]},
        ]
        if (
            pair["pairId"] != pair_id
            or pair_id in seen
            or pair["reasoningEffort"] != "high"
            or pair["calibratedBackendRevision"]
            != backend_revisions[model][0]
            or pair["studySourceSha256"]
            != manifest["calibrationRunnerSourceSha256"]
            or pair["allocationRole"] != ("primary" if order <= 10 else "reserve")
            or not isinstance(pair["studySeed"], str)
            or not study.SHA256_HEX.fullmatch(pair["studySeed"])
            or not isinstance(pair["roomOrder"], list)
            or any(not isinstance(room, str) for room in pair["roomOrder"])
            or set(pair["roomOrder"]) != set(study.ROOMS)
            or len(pair["roomOrder"]) != len(study.ROOMS)
            or pair["sessions"] != expected_sessions
            or not isinstance(pair["collectionOrder"], list)
            or len(pair["collectionOrder"]) != 2
            or any(
                not isinstance(session_id, str)
                for session_id in pair["collectionOrder"]
            )
            or sorted(pair["collectionOrder"])
            != sorted(session["sessionId"] for session in expected_sessions)
        ):
            raise CollectorError("collector allocation snapshot pair content differs")
        seen.add(pair_id)
    expected_pair_ids = {
        f"{study.MODEL_ALIASES[model]}-p{order:02d}"
        for model in study.MODEL_FAMILIES
        for order in range(1, 13)
    }
    if seen != expected_pair_ids:
        raise CollectorError("collector allocation snapshot pair inventory differs")
    return manifest


def load_state(path: Path) -> dict[str, Any]:
    """Load and minimally validate mutable collector state."""
    try:
        state = study.read_bounded_json(path, MAX_STATE_BYTES)
    except study.StudyError as error:
        raise CollectorError(f"collector state is invalid: {error}") from error
    expected = {
        "schemaVersion",
        "sessionId",
        "cursor",
        "repairUsed",
        "repairPending",
        "complete",
        "manifestSha256",
        "probeBankSha256",
        "sessionLedger",
        "cohortLedger",
        "numinousCommit",
        "pairId",
        "collectionOrder",
        "pairStatePaths",
        "withdrawalNonce",
        "consentPending",
        "refusalOrdinal",
        "headerDraft",
        "manifestSnapshot",
    }
    if (
        not isinstance(state, dict)
        or set(state) != expected
        or state.get("schemaVersion") != STATE_SCHEMA
    ):
        raise CollectorError("collector state schema differs")
    if (
        not isinstance(state["sessionId"], str)
        or not re.fullmatch(r"[a-z0-9-]+", state["sessionId"])
        or not isinstance(state["pairId"], str)
        or not re.fullmatch(r"[a-z0-9-]+", state["pairId"])
        or not isinstance(state["collectionOrder"], list)
        or len(state["collectionOrder"]) != 2
        or any(
            not isinstance(session_id, str)
            or not re.fullmatch(r"[a-z0-9-]+", session_id)
            for session_id in state["collectionOrder"]
        )
        or not isinstance(state["pairStatePaths"], list)
        or not 1 <= len(state["pairStatePaths"]) <= 2
        or not isinstance(state["withdrawalNonce"], str)
        or not study.SHA256_HEX.fullmatch(state["withdrawalNonce"])
    ):
        raise CollectorError("collector state pair identity is invalid")
    resolved_state = path.resolve()
    resolved_pair_paths: list[Path] = []
    for raw_path in state["pairStatePaths"]:
        if not isinstance(raw_path, str):
            raise CollectorError("collector pair state path is invalid")
        candidate = require_agent_path(Path(raw_path), "paired collector state")
        if candidate in resolved_pair_paths:
            raise CollectorError("collector pair state path is duplicated")
        resolved_pair_paths.append(candidate)
    if resolved_state not in resolved_pair_paths:
        raise CollectorError("collector state does not include its own paired path")
    if (
        isinstance(state["cursor"], bool)
        or not isinstance(state["cursor"], int)
        or not 0 <= state["cursor"] <= 10_000
        or not isinstance(state["repairUsed"], bool)
        or not isinstance(state["repairPending"], bool)
        or not isinstance(state["complete"], bool)
        or (state["repairPending"] and not state["repairUsed"])
        or (state["complete"] and state["repairPending"])
        or not isinstance(state["consentPending"], bool)
        or (
            state["refusalOrdinal"] is not None
            and (
                isinstance(state["refusalOrdinal"], bool)
                or not isinstance(state["refusalOrdinal"], int)
                or state["refusalOrdinal"] <= 0
            )
        )
        or not isinstance(state["headerDraft"], dict)
    ):
        raise CollectorError("collector state consent boundary is invalid")
    for key in ("manifestSha256", "probeBankSha256"):
        if not isinstance(state[key], str) or not study.SHA256_HEX.fullmatch(state[key]):
            raise CollectorError(f"collector state {key} is invalid")
    if (
        not isinstance(state["numinousCommit"], str)
        or not study.COMMIT_SHA.fullmatch(state["numinousCommit"])
    ):
        raise CollectorError("collector state commit is invalid")
    for key in ("sessionLedger", "cohortLedger"):
        if (
            not isinstance(state[key], str)
            or not state[key]
            or len(state[key]) > 4096
        ):
            raise CollectorError(f"collector state {key} path is invalid")
    if state["refusalOrdinal"] is not None and not state["consentPending"]:
        raise CollectorError("collector refusal ordinal exists after consent")
    study.bounded_canonical_size(state["headerDraft"], "collector header draft", 16_384)
    header = {**state["headerDraft"], "publicationConsent": "bounded-raw"}
    try:
        study.validate_event_shape(header)
        study.assert_sanitized(header)
    except study.StudyError as error:
        raise CollectorError(f"collector header draft is invalid: {error}") from error
    if (
        header.get("type") != "session"
        or header.get("sessionId") != state["sessionId"]
        or header.get("numinousCommit") != state["numinousCommit"]
    ):
        raise CollectorError("collector header draft binding differs")
    validate_manifest_snapshot(state["manifestSnapshot"])
    if study.content_sha256(state["manifestSnapshot"]) != state["manifestSha256"]:
        raise CollectorError("collector state allocation snapshot is invalid")
    try:
        _pairs, sessions = study.manifest_indexes(state["manifestSnapshot"])
        if state["sessionId"] not in sessions:
            raise CollectorError("collector state session is outside its allocation")
        pair, session = sessions[state["sessionId"]]
        if (
            state["pairId"] != pair["pairId"]
            or state["collectionOrder"] != pair["collectionOrder"]
        ):
            raise CollectorError("collector state pair allocation binding differs")
        study.validate_session_header(header, pair, session)
    except study.StudyError as error:
        raise CollectorError(f"collector allocation binding is invalid: {error}") from error
    return state


def repository_commit() -> str:
    """Return the clean tracked repository revision used for every encounter."""
    try:
        return study.repository_commit()
    except study.StudyError as error:
        raise CollectorError(str(error)) from error


def study_source_sha256(revision: str) -> str:
    """Return the committed runtime-source identity for one exact revision."""
    try:
        return study.study_source_identity(revision)
    except study.StudyError as error:
        raise CollectorError(str(error)) from error


def require_attempt_start_receipt(
    path: Path | None, expected_sha256: str
) -> str:
    """Require one independently recorded receipt before concealed exposure."""
    if path is None:
        raise CollectorError(
            "attempt start receipt required before exposure for commitment "
            f"{expected_sha256}"
        )
    receipt_path = require_agent_path(path, "attempt start receipt")
    try:
        receipt = study.read_bounded_json(receipt_path, 8192)
        study.validate_attempt_start_receipt(receipt, expected_sha256)
    except study.StudyError as error:
        raise CollectorError(f"attempt start receipt is invalid: {error}") from error
    return study.content_sha256(receipt)


def require_committed_file(path: Path, label: str) -> None:
    """Require a tracked file whose bytes equal the version committed at HEAD."""
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(ROOT.resolve()).as_posix()
    except ValueError as error:
        raise CollectorError(f"{label} must be inside the repository") from error
    try:
        subprocess.run(
            ["git", "ls-files", "--error-unmatch", "--", relative],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        committed = subprocess.run(
            ["git", "show", f"HEAD:{relative}"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        ).stdout
        current = resolved.read_bytes()
    except (OSError, subprocess.CalledProcessError) as error:
        raise CollectorError(f"{label} is not committed at repository HEAD") from error
    if current != committed:
        raise CollectorError(f"{label} differs from repository HEAD")


def tool_text(result: dict[str, Any]) -> str:
    """Collect only public textual MCP content blocks."""
    content = result.get("content", [])
    if not isinstance(content, list):
        raise CollectorError("MCP tool content must be an array")
    blocks = []
    for block in content:
        if not isinstance(block, dict):
            raise CollectorError("MCP tool content block must be an object")
        text = block.get("text")
        if text is not None:
            if not isinstance(text, str):
                raise CollectorError("MCP tool text block must contain a string")
            blocks.append(text)
    return "\n".join(blocks)


MCP_RESULT_FIELDS = {
    "play_room": frozenset(
        {
            "action",
            "delta",
            "engineeredAha",
            "gesture",
            "goal",
            "goalMet",
            "height",
            "pokes",
            "render",
            "reveal",
            "room",
            "status",
            "t",
            "title",
            "variation",
            "width",
        }
    ),
    "reveal_room": frozenset({"concept", "reveal", "room", "title"}),
    "plot_expression": frozenset(
        {
            "a",
            "discovery",
            "expression",
            "plot",
            "recipeCount",
            "recipeIndex",
            "valid",
            "xmax",
            "xmin",
            "ymax",
            "ymin",
        }
    ),
}
MCP_PUBLIC_PROJECTION = {
    "play_room": (
        "action",
        "goal",
        "goalMet",
        "render",
        "reveal",
        "room",
        "status",
        "t",
        "title",
    ),
    "reveal_room": ("concept", "reveal", "room", "title"),
    "plot_expression": (
        "expression",
        "plot",
        "valid",
        "xmax",
        "xmin",
        "ymax",
        "ymin",
    ),
}


def project_mcp_result(
    tool: str,
    result: dict[str, Any],
    arguments: dict[str, Any],
    expected_server_info: dict[str, Any],
) -> dict[str, Any]:
    """Validate one exact MCP result schema and retain its public projection."""
    if tool not in MCP_RESULT_FIELDS:
        raise CollectorError(f"collector does not allow MCP tool {tool}")
    if not isinstance(result, dict) or set(result) != {
        "_meta",
        "content",
        "isError",
        "resultType",
        "structuredContent",
    }:
        raise CollectorError(f"MCP tool {tool} result envelope differs")
    if result["resultType"] != "complete":
        raise CollectorError(f"MCP tool {tool} result is incomplete")
    if (
        not isinstance(expected_server_info, dict)
        or set(expected_server_info) != {"name", "version"}
        or expected_server_info.get("name") != "numinous"
        or not isinstance(expected_server_info.get("version"), str)
        or not expected_server_info["version"]
        or len(expected_server_info["version"].encode("utf-8")) > 256
    ):
        raise CollectorError("isolated MCP server identity is invalid")
    metadata = result["_meta"]
    if (
        not isinstance(metadata, dict)
        or set(metadata) != {mcp_play.SERVER_INFO_META_KEY}
        or metadata[mcp_play.SERVER_INFO_META_KEY] != expected_server_info
    ):
        raise CollectorError(f"MCP tool {tool} server identity differs")
    if result["isError"] is not False:
        raise CollectorError(f"MCP tool {tool} returned an error result")
    structured = result["structuredContent"]
    structured_fields = set(structured) if isinstance(structured, dict) else set()
    expected_fields = MCP_RESULT_FIELDS[tool]
    if tool == "reveal_room":
        schema_matches = structured_fields in (
            expected_fields,
            expected_fields - {"concept"},
        )
    else:
        schema_matches = structured_fields == expected_fields
    if not isinstance(structured, dict) or not schema_matches:
        raise CollectorError(f"MCP tool {tool} structured result schema differs")
    content = result["content"]
    if (
        not isinstance(content, list)
        or not content
        or any(
            not isinstance(block, dict) or set(block) != {"type", "text"}
            for block in content
        )
        or any(
            block["type"] != "text" or not isinstance(block["text"], str)
            for block in content
        )
    ):
        raise CollectorError(f"MCP tool {tool} content schema differs")
    projection = {
        key: structured[key]
        for key in MCP_PUBLIC_PROJECTION[tool]
        if key in structured
    }
    def bounded_string(field: str, *, nullable: bool = False) -> None:
        value = projection.get(field)
        if nullable and value is None:
            return
        if not isinstance(value, str) or len(value.encode("utf-8")) > 65_536:
            raise CollectorError(f"MCP tool {tool} field {field} must be a bounded string")

    if tool == "play_room":
        for field in ("action", "render", "room", "status", "title"):
            bounded_string(field)
        bounded_string("goal", nullable=True)
        bounded_string("reveal", nullable=True)
        if not isinstance(projection["goalMet"], bool):
            raise CollectorError("MCP play_room goalMet must be boolean")
        phase = projection["t"]
        if (
            isinstance(phase, bool)
            or not isinstance(phase, (int, float))
            or not math.isfinite(float(phase))
            or float(phase) != float(arguments.get("t", math.nan))
            or projection["room"] != arguments.get("id")
        ):
            raise CollectorError("MCP play_room result identity differs")
    elif tool == "reveal_room":
        for field in ("reveal", "room", "title"):
            bounded_string(field)
        if "concept" in projection:
            bounded_string("concept")
        if projection["room"] != arguments.get("id"):
            raise CollectorError("MCP reveal_room result identity differs")
    else:
        for field in ("expression", "plot"):
            bounded_string(field)
        if projection["expression"] != arguments.get("expr"):
            raise CollectorError("MCP plot_expression result identity differs")
        if projection["valid"] is not True:
            raise CollectorError("MCP plot_expression valid must be true")
        numeric = []
        for field in ("xmax", "xmin", "ymax", "ymin"):
            value = projection[field]
            if (
                isinstance(value, bool)
                or not isinstance(value, (int, float))
                or not math.isfinite(float(value))
                or abs(float(value)) > 1_000_000_000_000.0
            ):
                raise CollectorError(
                    f"MCP plot_expression field {field} must be finite and bounded"
                )
            numeric.append(float(value))
        xmax, xmin, ymax, ymin = numeric
        if not xmin < xmax or not ymin <= ymax:
            raise CollectorError("MCP plot_expression bounds are invalid")
    study.bounded_canonical_size(projection, f"MCP tool {tool} projection", 524_288)
    study.assert_sanitized(projection, "mcp")
    return projection


def write_receipts_atomic(
    ledger: Path, receipts: list[dict[str, Any]], description: str
) -> None:
    """Durably stage a complete receipt chain, then replace it atomically."""
    payload = "".join(
        json.dumps(
            receipt,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        + "\n"
        for receipt in receipts
    )
    try:
        ledger.parent.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise CollectorError(
            f"could not create collector ledger directory: {error}"
        ) from error
    try:
        handle = tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            newline="\n",
            dir=ledger.parent,
            prefix=f".{ledger.name}.",
            suffix=".tmp",
            delete=False,
        )
    except OSError as error:
        raise CollectorError(f"could not stage {description}: {error}") from error
    temporary = Path(handle.name)
    try:
        with handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, ledger)
    except OSError as error:
        raise CollectorError(f"could not replace {description}: {error}") from error
    finally:
        try:
            temporary.unlink(missing_ok=True)
        except OSError as cleanup_error:
            if sys.exception() is None:
                raise CollectorError(
                    f"could not remove temporary {description} {temporary}: "
                    f"{cleanup_error}"
                ) from cleanup_error


def receipt_anchor_path(ledger: Path) -> Path:
    """Return the terminal receipt commitment beside one ledger."""
    return ledger.with_name(f"{ledger.name}.anchor.json")


def receipt_transaction_path(ledger: Path) -> Path:
    """Return the durable write-ahead transaction beside one ledger."""
    return ledger.with_name(f"{ledger.name}.transaction.json")


def read_receipt_anchor(path: Path) -> dict[str, Any]:
    """Read one bounded terminal receipt commitment."""
    try:
        anchor = study.read_bounded_json(path, MAX_MARKER_BYTES)
    except study.StudyError as error:
        raise CollectorError(f"collector receipt anchor is invalid: {error}") from error
    if not isinstance(anchor, dict):
        raise CollectorError("collector receipt anchor must be an object")
    return anchor


def read_committed_receipts(
    ledger: Path, manifest: dict[str, Any]
) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    """Read a ledger only when its independent terminal commitment is exact."""
    transaction = receipt_transaction_path(ledger)
    if transaction.exists():
        raise CollectorError("collector receipt transaction requires recovery")
    anchor_path = receipt_anchor_path(ledger)
    if ledger.exists() != anchor_path.exists():
        raise CollectorError("collector ledger and receipt anchor presence differs")
    if not ledger.exists():
        return [], None
    receipts = study.read_receipt_jsonl(ledger)
    anchor = read_receipt_anchor(anchor_path)
    try:
        study.verify_receipt_anchor(manifest, receipts, anchor)
    except study.StudyError as error:
        raise CollectorError(f"collector receipt commitment differs: {error}") from error
    return receipts, anchor


def recover_receipt_transaction(ledger: Path, manifest: dict[str, Any]) -> bool:
    """Finish one exact durable ledger transaction after an interrupted process."""
    transaction_path = receipt_transaction_path(ledger)
    if not transaction_path.exists():
        return False
    try:
        transaction = study.read_bounded_json(
            transaction_path, MAX_RECEIPT_TRANSACTION_BYTES
        )
    except study.StudyError as error:
        raise CollectorError(f"collector receipt transaction is invalid: {error}") from error
    if not isinstance(transaction, dict) or set(transaction) != {
        "schemaVersion",
        "ledger",
        "manifestSha256",
        "priorAnchor",
        "targetAnchor",
        "receipts",
    }:
        raise CollectorError("collector receipt transaction has an invalid shape")
    if (
        transaction["schemaVersion"] != RECEIPT_TRANSACTION_SCHEMA
        or transaction["ledger"] != str(ledger)
        or transaction["manifestSha256"] != study.content_sha256(manifest)
        or not isinstance(transaction["receipts"], list)
        or (
            transaction["priorAnchor"] is not None
            and not isinstance(transaction["priorAnchor"], dict)
        )
        or not isinstance(transaction["targetAnchor"], dict)
    ):
        raise CollectorError("collector receipt transaction binding differs")
    receipts = transaction["receipts"]
    try:
        expected_target = study.build_receipt_anchor(manifest, receipts)
    except study.StudyError as error:
        raise CollectorError(f"collector receipt transaction differs: {error}") from error
    if study.canonical_bytes(expected_target) != study.canonical_bytes(
        transaction["targetAnchor"]
    ):
        raise CollectorError("collector receipt transaction target differs")
    prior_anchor = transaction["priorAnchor"]
    current_receipts = study.read_receipt_jsonl(ledger) if ledger.exists() else []
    current_chain_anchor = (
        study.build_receipt_anchor(manifest, current_receipts)
        if current_receipts
        else None
    )
    if current_chain_anchor not in (prior_anchor, expected_target):
        raise CollectorError("collector receipt transaction ledger state differs")
    anchor_path = receipt_anchor_path(ledger)
    current_anchor = read_receipt_anchor(anchor_path) if anchor_path.exists() else None
    if current_anchor not in (prior_anchor, expected_target):
        raise CollectorError("collector receipt transaction anchor state differs")
    if current_chain_anchor != expected_target:
        write_receipts_atomic(ledger, receipts, f"collector receipt ledger {ledger}")
    if current_anchor != expected_target:
        write_json_atomic(anchor_path, expected_target, "collector receipt anchor")
    verified_receipts, verified_anchor = read_committed_receipts_without_transaction(
        ledger, manifest
    )
    if verified_receipts != receipts or verified_anchor != expected_target:
        raise CollectorError("collector receipt transaction verification differs")
    remove_path(transaction_path, "completed collector receipt transaction")
    return True


def read_committed_receipts_without_transaction(
    ledger: Path, manifest: dict[str, Any]
) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    """Verify committed files while their owning transaction still exists."""
    anchor_path = receipt_anchor_path(ledger)
    if not ledger.exists() or not anchor_path.exists():
        raise CollectorError("collector receipt transaction did not publish both files")
    receipts = study.read_receipt_jsonl(ledger)
    anchor = read_receipt_anchor(anchor_path)
    try:
        study.verify_receipt_anchor(manifest, receipts, anchor)
    except study.StudyError as error:
        raise CollectorError(f"collector receipt commitment differs: {error}") from error
    return receipts, anchor


def commit_receipts(
    ledger: Path,
    manifest: dict[str, Any],
    receipts: list[dict[str, Any]],
    description: str,
) -> None:
    """Commit a complete receipt chain through a durable write-ahead transaction."""
    recover_receipt_transaction(ledger, manifest)
    prior_receipts, prior_anchor = read_committed_receipts(ledger, manifest)
    target_anchor = study.build_receipt_anchor(manifest, receipts)
    if prior_receipts == receipts and prior_anchor == target_anchor:
        return
    transaction = {
        "schemaVersion": RECEIPT_TRANSACTION_SCHEMA,
        "ledger": str(ledger),
        "manifestSha256": study.content_sha256(manifest),
        "priorAnchor": prior_anchor,
        "targetAnchor": target_anchor,
        "receipts": receipts,
    }
    transaction_path = receipt_transaction_path(ledger)
    payload = json.dumps(
        transaction,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ) + "\n"
    try:
        result = study.write_text_once(transaction_path, payload)
    except study.StudyError as error:
        raise CollectorError(f"could not stage collector receipt transaction: {error}") from error
    if result != "written":
        raise CollectorError("collector receipt transaction already exists")
    try:
        write_receipts_atomic(ledger, receipts, description)
    except Exception:
        current_receipts = study.read_receipt_jsonl(ledger) if ledger.exists() else []
        current_anchor = (
            read_receipt_anchor(receipt_anchor_path(ledger))
            if receipt_anchor_path(ledger).exists()
            else None
        )
        if current_receipts == prior_receipts and current_anchor == prior_anchor:
            remove_path(transaction_path, "aborted collector receipt transaction")
        raise
    write_json_atomic(
        receipt_anchor_path(ledger), target_anchor, "collector receipt anchor"
    )
    verified_receipts, verified_anchor = read_committed_receipts_without_transaction(
        ledger, manifest
    )
    if verified_receipts != receipts or verified_anchor != target_anchor:
        raise CollectorError("collector receipt transaction verification differs")
    remove_path(transaction_path, "completed collector receipt transaction")


def write_marker_once(path: Path, marker: dict[str, Any], label: str) -> None:
    """Publish one complete marker without replacing a concurrent winner."""
    payload = json.dumps(
        marker,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ) + "\n"
    try:
        result = study.write_text_once(path, payload)
    except study.StudyError as error:
        raise CollectorError(f"could not claim {label}: {error}") from error
    if result != "written":
        raise CollectorError(f"{label} already exists")


def append_receipt_once(
    ledger: Path,
    manifest: dict[str, Any],
    event: dict[str, Any],
    identity_fields: tuple[str, ...] = (),
    *,
    event_validator: Callable[[dict[str, Any]], None] = study.validate_event_shape,
) -> tuple[dict[str, Any], bool]:
    """Atomically append, or return an identical receipt under a unique identity."""
    event_validator(event)
    study.assert_sanitized(event)
    try:
        ledger.parent.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise CollectorError(
            f"could not create collector ledger directory: {error}"
        ) from error
    lock = ledger.with_name(f"{ledger.name}.lock")
    write_marker_once(
        lock,
        {
            "schemaVersion": LOCK_SCHEMA,
            "ledger": str(ledger),
            "ownerPid": os.getpid(),
        },
        f"collector ledger lock {ledger}",
    )
    try:
        recover_receipt_transaction(ledger, manifest)
        receipts, anchor = read_committed_receipts(ledger, manifest)
        events = (
            study.verify_receipt_anchor(manifest, receipts, anchor)
            if receipts and anchor is not None
            else []
        )
        if identity_fields:
            matches = [
                (index, observed)
                for index, observed in enumerate(events)
                if all(observed.get(field) == event.get(field) for field in identity_fields)
            ]
            if len(matches) > 1:
                raise CollectorError("receipt identity is duplicated")
            if matches:
                index, observed = matches[0]
                clean_observed = clean_receipt_event(observed)
                if study.canonical_bytes(clean_observed) != study.canonical_bytes(event):
                    raise CollectorError("receipt identity has different content")
                return receipts[index], False
        updated = study.seal_records(manifest, [*events, event])
        commit_receipts(
            ledger, manifest, updated, f"collector receipt ledger {ledger}"
        )
        return updated[-1], True
    finally:
        try:
            lock.unlink()
        except OSError as error:
            if sys.exception() is None:
                raise CollectorError(
                    f"could not release collector ledger lock {lock}: {error}"
                ) from error


def append_receipt(
    ledger: Path, manifest: dict[str, Any], event: dict[str, Any]
) -> dict[str, Any]:
    """Validate and atomically extend one receipt chain."""
    receipt, _written = append_receipt_once(ledger, manifest, event)
    return receipt


def active_path(ledger: Path) -> Path:
    """Return the exclusive cohort collection lease path."""
    return ledger.with_name(f"{ledger.name}.active")


def transition_path(ledger: Path) -> Path:
    """Return the exclusive cohort mutation marker path."""
    return ledger.with_name(f"{ledger.name}.transition")


def recovery_path(ledger: Path) -> Path:
    """Return the exclusive recovery mutex path for one cohort ledger."""
    return ledger.with_name(f"{ledger.name}.recovery")


def transition_thread_mutex(ledger: Path) -> threading.RLock:
    """Return one process-local mutex for a cohort transition path."""
    key = ledger.resolve()
    with THREAD_MUTEX_GUARD:
        return THREAD_MUTEXES.setdefault(key, threading.RLock())


@contextlib.contextmanager
def serialized_transition_file(ledger: Path):
    """Serialize one complete cohort read, decision, receipt, and state transition."""
    marker_path = transition_path(ledger)
    marker = {
        "schemaVersion": TRANSITION_SCHEMA,
        "ledger": str(ledger),
        "ownerPid": os.getpid(),
    }
    deadline = time.monotonic() + TRANSITION_WAIT_SECONDS
    while True:
        try:
            write_marker_once(marker_path, marker, "collector state transition")
            break
        except CollectorError:
            if not marker_path.exists():
                raise
            try:
                observed = read_marker(
                    marker_path,
                    {"schemaVersion", "ledger", "ownerPid"},
                    TRANSITION_SCHEMA,
                )
            except CollectorError as error:
                cause: BaseException | None = error
                while cause is not None and not isinstance(cause, PermissionError):
                    cause = cause.__cause__
                if cause is None or time.monotonic() >= deadline:
                    raise
                time.sleep(0.01)
                continue
            if observed["ledger"] != str(ledger):
                raise CollectorError("collector transition points to another ledger")
            if not process_is_alive(observed["ownerPid"]):
                raise CollectorError("stale collector transition requires recovery")
            if time.monotonic() >= deadline:
                raise CollectorError("collector cohort transition is busy")
            time.sleep(0.01)
    try:
        yield
    finally:
        if marker_path.exists():
            observed = read_marker(
                marker_path,
                {"schemaVersion", "ledger", "ownerPid"},
                TRANSITION_SCHEMA,
            )
            if observed != marker:
                raise CollectorError("collector state transition changed ownership")
            remove_path(marker_path, "collector state transition")


@contextlib.contextmanager
def serialized_transition(ledger: Path):
    """Serialize one transition across threads and independent processes."""
    with transition_thread_mutex(ledger):
        with serialized_transition_file(ledger):
            yield


def serialize_state_transition(handler: Callable[[argparse.Namespace], None]):
    """Wrap a mutating state command in the per-state transition mutex."""

    def serialized(args: argparse.Namespace) -> None:
        ledger = require_agent_path(args.ledger, "collector ledger")
        with serialized_transition(ledger):
            handler(args)

    return serialized


def active_record(state_path: Path, session_id: str) -> dict[str, Any]:
    """Build the private lease record for one in-progress session."""
    return {
        "schemaVersion": ACTIVE_SCHEMA,
        "sessionId": session_id,
        "statePath": str(state_path),
        "ownerPid": os.getpid(),
    }


def claim_active(ledger: Path, state_path: Path, session_id: str) -> None:
    """Claim the cohort-wide serial collection lease without replacement."""
    lease = active_path(ledger)
    write_marker_once(
        lease,
        active_record(state_path, session_id),
        "collector session lease",
    )


def require_active(ledger: Path, state_path: Path, session_id: str) -> None:
    """Require the exact private lease before changing session evidence."""
    lease = active_path(ledger)
    try:
        observed = study.read_bounded_json(lease, MAX_MARKER_BYTES)
    except study.StudyError as error:
        raise CollectorError(f"collector lease is invalid: {error}") from error
    expected = active_record(state_path, session_id)
    if (
        not isinstance(observed, dict)
        or set(observed) != set(expected)
        or observed.get("schemaVersion") != ACTIVE_SCHEMA
        or observed.get("sessionId") != session_id
        or observed.get("statePath") != str(state_path)
        or isinstance(observed.get("ownerPid"), bool)
        or not isinstance(observed.get("ownerPid"), int)
        or observed["ownerPid"] <= 0
    ):
        raise CollectorError("collector lease belongs to a different session")


def release_active(ledger: Path, state_path: Path, session_id: str) -> None:
    """Release only the lease owned by the completed bounded transition."""
    require_active(ledger, state_path, session_id)
    try:
        active_path(ledger).unlink()
    except OSError as error:
        raise CollectorError(f"could not release collector lease: {error}") from error


def process_is_alive(pid: int) -> bool:
    """Return whether a local process still owns a crash-recovery marker."""
    if os.name == "nt":
        return windows_process_is_alive(pid)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def windows_process_is_alive(pid: int) -> bool:
    """Query a Windows process handle without delivering a signal."""
    import ctypes
    from ctypes import wintypes

    process_query_limited_information = 0x1000
    error_invalid_parameter = 87
    still_active = 259

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)
    kernel32.OpenProcess.restype = wintypes.HANDLE
    kernel32.GetExitCodeProcess.argtypes = (wintypes.HANDLE, wintypes.LPDWORD)
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32.CloseHandle.argtypes = (wintypes.HANDLE,)
    kernel32.CloseHandle.restype = wintypes.BOOL

    handle = kernel32.OpenProcess(process_query_limited_information, False, pid)
    if not handle:
        return ctypes.get_last_error() != error_invalid_parameter
    try:
        exit_code = wintypes.DWORD()
        if not kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code)):
            return True
        return exit_code.value == still_active
    finally:
        kernel32.CloseHandle(handle)


def read_marker(path: Path, expected_fields: set[str], schema: str) -> dict[str, Any]:
    """Read one exact private crash-recovery marker."""
    try:
        value = study.read_bounded_json(path, MAX_MARKER_BYTES)
    except study.StudyError as error:
        raise CollectorError(f"recovery marker {path} is invalid: {error}") from error
    if (
        not isinstance(value, dict)
        or set(value) != expected_fields
        or value.get("schemaVersion") != schema
        or isinstance(value.get("ownerPid"), bool)
        or not isinstance(value.get("ownerPid"), int)
        or value["ownerPid"] <= 0
    ):
        raise CollectorError(f"recovery marker {path} has an invalid shape")
    return value


@contextlib.contextmanager
def serialized_recovery_file(ledger: Path):
    """Own the live transition marker throughout one dead-owner recovery."""
    recovery = recovery_path(ledger)
    recovery_marker = {
        "schemaVersion": RECOVERY_SCHEMA,
        "ledger": str(ledger),
        "ownerPid": os.getpid(),
    }
    write_marker_once(recovery, recovery_marker, "collector recovery")
    transition = transition_path(ledger)
    transition_marker = {
        "schemaVersion": TRANSITION_SCHEMA,
        "ledger": str(ledger),
        "ownerPid": os.getpid(),
    }
    reclaimed = False
    owns_transition = False
    try:
        if transition.exists():
            observed = read_marker(
                transition,
                {"schemaVersion", "ledger", "ownerPid"},
                TRANSITION_SCHEMA,
            )
            if observed["ledger"] != str(ledger):
                raise CollectorError(
                    "transition marker points to a different cohort ledger"
                )
            if process_is_alive(observed["ownerPid"]):
                raise CollectorError("collector transition owner is still running")
            write_json_atomic(
                transition, transition_marker, "collector recovery transition"
            )
            reclaimed = True
        else:
            write_marker_once(
                transition, transition_marker, "collector recovery transition"
            )
        owns_transition = True
        yield reclaimed
    finally:
        if owns_transition and transition.exists():
            observed = read_marker(
                transition,
                {"schemaVersion", "ledger", "ownerPid"},
                TRANSITION_SCHEMA,
            )
            if observed != transition_marker:
                raise CollectorError("collector recovery transition changed ownership")
            remove_path(transition, "collector recovery transition")
        if recovery.exists():
            observed = read_marker(
                recovery,
                {"schemaVersion", "ledger", "ownerPid"},
                RECOVERY_SCHEMA,
            )
            if observed != recovery_marker:
                raise CollectorError("collector recovery changed ownership")
            remove_path(recovery, "collector recovery")


@contextlib.contextmanager
def serialized_recovery(ledger: Path):
    """Serialize recovery against same-process and cross-process mutations."""
    with transition_thread_mutex(ledger):
        with serialized_recovery_file(ledger) as reclaimed:
            yield reclaimed


def serialize_recovery(handler: Callable[[argparse.Namespace, bool], None]):
    """Wrap recovery in a mutex and a live transition ownership claim."""

    def serialized(args: argparse.Namespace) -> None:
        ledger = require_agent_path(args.ledger, "collector ledger")
        with serialized_recovery(ledger) as reclaimed_transition:
            handler(args, reclaimed_transition)

    return serialized


@serialize_recovery
def command_recover(args: argparse.Namespace, reclaimed_transition: bool) -> None:
    """Recover only dead-process markers for one exact persisted collector state."""
    state_path = require_agent_path(args.state, "collector state")
    ledger = require_agent_path(args.ledger, "collector ledger")
    state = load_state(state_path)
    if Path(state["cohortLedger"]) != ledger:
        raise CollectorError("collector state cohort ledger differs")
    recovered = ["collector-transition"] if reclaimed_transition else []
    lock_ledgers = {
        ledger,
        require_agent_path(Path(state["sessionLedger"]), "provisional session ledger"),
    }
    for lock_ledger in lock_ledgers:
        lock = lock_ledger.with_name(f"{lock_ledger.name}.lock")
        if lock.exists():
            marker = read_marker(
                lock, {"schemaVersion", "ledger", "ownerPid"}, LOCK_SCHEMA
            )
            if marker["ledger"] != str(lock_ledger):
                raise CollectorError("ledger lock points to a different ledger")
            if process_is_alive(marker["ownerPid"]):
                raise CollectorError("ledger lock owner is still running")
            remove_path(lock, "stale collector ledger lock")
            recovered.append(f"ledger-lock:{lock_ledger.name}")
    manifest = state["manifestSnapshot"]
    for lock_ledger in lock_ledgers:
        if recover_receipt_transaction(lock_ledger, manifest):
            recovered.append(f"receipt-transaction:{lock_ledger.name}")
    owned_paths = {
        state_path,
        transition_path(ledger),
        ledger,
        active_path(ledger),
        *(item.with_name(f"{item.name}.lock") for item in lock_ledgers),
        *(
            pending_session_ledger(ledger, session_id)
            for session_id in state["collectionOrder"]
        ),
    }
    for owned_path in owned_paths:
        removed = remove_owned_temporaries(owned_path, owned_path.name)
        if removed:
            recovered.append(f"temporary:{owned_path.name}:{removed}")
    lease = active_path(ledger)
    if lease.exists():
        marker = read_marker(
            lease,
            {"schemaVersion", "sessionId", "statePath", "ownerPid"},
            ACTIVE_SCHEMA,
        )
        if marker["statePath"] != str(state_path):
            raise CollectorError("collector lease points to a different state file")
        if process_is_alive(marker["ownerPid"]):
            raise CollectorError("collector lease owner is still running")
        if state["sessionId"] != marker["sessionId"] or Path(
            state["cohortLedger"]
        ) != ledger:
            raise CollectorError("collector state does not match the stale lease")
        remove_path(lease, "stale collector lease")
        recovered.append("collector-lease")
        if not state["complete"]:
            claim_active(ledger, state_path, state["sessionId"])
    else:
        if not state["complete"]:
            claim_active(ledger, state_path, state["sessionId"])
            recovered.append("collector-lease")
    print(
        json.dumps(
            {"status": "recovered", "markers": recovered},
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
        )
    )


@serialize_recovery
def command_recover_cohort(args: argparse.Namespace, reclaimed_transition: bool) -> None:
    """Recover dead cohort-only markers when no collector state is available."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_agent_path(args.manifest, "allocation manifest")
    manifest = study.load_json(args.manifest)
    if not isinstance(manifest, dict):
        raise CollectorError("allocation manifest must be an object")
    recovered = recover_unleased_ledger(ledger, manifest, reclaimed_transition)
    print(json.dumps({"status": "recovered", "markers": recovered}, sort_keys=True))


def recover_unleased_ledger(
    ledger: Path, commitment: dict[str, Any], reclaimed_transition: bool
) -> list[str]:
    """Recover one dead receipt stream that has no resumable session state."""
    if active_path(ledger).exists():
        raise CollectorError("stateful recovery is required while a session lease exists")
    recovered = ["collector-transition"] if reclaimed_transition else []
    markers = (
        (
            ledger.with_name(f"{ledger.name}.lock"),
            {"schemaVersion", "ledger", "ownerPid"},
            LOCK_SCHEMA,
            "ledger-lock",
        ),
    )
    for path, fields, schema, label in markers:
        if not path.exists():
            continue
        marker = read_marker(path, fields, schema)
        if marker["ledger"] != str(ledger):
            raise CollectorError(f"{label} points to a different cohort ledger")
        if process_is_alive(marker["ownerPid"]):
            raise CollectorError(f"{label} owner is still running")
        remove_path(path, f"stale {label}")
        recovered.append(label)
        remove_owned_temporaries(path, label)
    if recover_receipt_transaction(ledger, commitment):
        recovered.append("receipt-transaction")
    remove_owned_temporaries(ledger, "cohort ledger")
    return recovered


def pending_root(ledger: Path) -> Path:
    """Return the private pair-staging directory beside the cohort ledger."""
    return ledger.with_name(f"{ledger.name}.pending")


def pending_session_ledger(ledger: Path, session_id: str) -> Path:
    """Return the canonical provisional receipt ledger for one allocated session."""
    if not re.fullmatch(r"[a-z0-9-]+", session_id):
        raise CollectorError("session id is unsafe for a provisional ledger path")
    return pending_root(ledger) / f"{session_id}.jsonl"


def read_verified_ledger(
    ledger: Path, manifest: dict[str, Any]
) -> list[dict[str, Any]]:
    """Read one optional ledger through the manifest-rooted verifier."""
    receipts, anchor = read_committed_receipts(ledger, manifest)
    if not receipts or anchor is None:
        return []
    return study.verify_receipt_anchor(manifest, receipts, anchor)


def remove_ledger_artifacts(ledger: Path, description: str) -> None:
    """Remove one verified private ledger and all of its owned commitment files."""
    if receipt_transaction_path(ledger).exists():
        raise CollectorError(f"{description} has an unfinished receipt transaction")
    remove_path(ledger, description)
    remove_path(receipt_anchor_path(ledger), f"{description} receipt anchor")
    remove_owned_temporaries(ledger, description)
    remove_owned_temporaries(
        receipt_anchor_path(ledger), f"{description} receipt anchor"
    )


def clean_receipt_event(event: dict[str, Any]) -> dict[str, Any]:
    """Remove the verifier-only source index from one event."""
    return {key: value for key, value in event.items() if key != "_sourceIndex"}


def collection_evidence(ledger: Path, manifest: dict[str, Any]) -> list[dict[str, Any]]:
    """Combine settled and provisional events for scheduling checks only."""
    events = read_verified_ledger(ledger, manifest)
    _pairs, sessions = study.manifest_indexes(manifest)
    for session_id in sessions:
        events.extend(
            read_verified_ledger(pending_session_ledger(ledger, session_id), manifest)
        )
    return events


def replace_ledger(
    ledger: Path, manifest: dict[str, Any], events: list[dict[str, Any]]
) -> None:
    """Atomically replace a provisional ledger with a privacy projection."""
    receipts = study.seal_records(manifest, events)
    commit_receipts(ledger, manifest, receipts, f"provisional ledger {ledger}")


def publish_pair(ledger: Path, manifest: dict[str, Any], pair: dict[str, Any]) -> bool:
    """Publish a complete terminal pair through one atomic ledger replacement."""
    settled = read_verified_ledger(ledger, manifest)
    pair_session_ids = set(pair["collectionOrder"])
    settled_terminals = {
        event["sessionId"]
        for event in settled
        if event.get("sessionId") in pair_session_ids
        and event.get("type") in ("session_complete", "session_interruption")
    }
    if settled_terminals == pair_session_ids:
        for session_id in pair["collectionOrder"]:
            remove_ledger_artifacts(
                pending_session_ledger(ledger, session_id),
                "settled provisional session ledger",
            )
        try:
            pending_root(ledger).rmdir()
        except OSError:
            pass
        return True
    staged: list[dict[str, Any]] = []
    staged_paths: list[Path] = []
    for session_id in pair["collectionOrder"]:
        path = pending_session_ledger(ledger, session_id)
        events = read_verified_ledger(path, manifest)
        if not events or events[-1].get("type") not in (
            "session_complete",
            "session_interruption",
        ):
            return False
        staged.extend(events)
        staged_paths.append(path)
    observed = [
        event for event in settled if event.get("sessionId") in pair_session_ids
    ]
    if observed:
        if len(observed) != len(staged) or any(
            study.canonical_bytes(clean_receipt_event(actual))
            != study.canonical_bytes(clean_receipt_event(expected))
            for actual, expected in zip(observed, staged, strict=True)
        ):
            raise CollectorError(
                "settled pair evidence is partial or differs from provisional receipts"
            )
        raise CollectorError("settled pair terminal receipts are incomplete")
    clean_settled = [clean_receipt_event(event) for event in settled]
    clean_staged = [clean_receipt_event(event) for event in staged]
    updated = study.seal_records(manifest, [*clean_settled, *clean_staged])
    commit_receipts(ledger, manifest, updated, f"collector receipt ledger {ledger}")
    published = read_verified_ledger(ledger, manifest)
    appended = [
        event for event in published if event.get("sessionId") in pair_session_ids
    ]
    if len(appended) != len(staged) or any(
        study.canonical_bytes(
            clean_receipt_event(actual)
        )
        != study.canonical_bytes(
            clean_receipt_event(expected)
        )
        for actual, expected in zip(appended, staged, strict=True)
    ):
        raise CollectorError("atomic pair publication verification differs")
    for path in staged_paths:
        remove_ledger_artifacts(path, "settled provisional session ledger")
    try:
        pending_root(ledger).rmdir()
    except OSError:
        pass
    return True


def interruption_projection(
    events: list[dict[str, Any]], terminal: dict[str, Any]
) -> list[dict[str, Any]]:
    """Remove all participant response content from an interrupted session."""
    headers = [event for event in events if event.get("type") == "session"]
    if len(headers) != 1:
        raise CollectorError("interrupted session header is missing or duplicated")
    condition = headers[0].get("condition")
    participant_tool_coordinates = {
        (room_id, sequence, call["role"], call["tool"])
        for room_id, room in study.encounter_rooms().items()
        for sequence, call in enumerate(
            study.condition_calls(room, condition), start=1
        )
        if call["arguments"].get("expr") == "__PARTICIPANT_EXPRESSION__"
    }
    retained = []
    for event in events:
        if event.get("type") not in ("session", "tool", "material"):
            continue
        cleaned = {key: value for key, value in event.items() if key != "_sourceIndex"}
        coordinates = (
            cleaned.get("room"),
            cleaned.get("sequence"),
            cleaned.get("role"),
            cleaned.get("tool"),
        )
        if cleaned.get("type") == "tool" and coordinates in participant_tool_coordinates:
            cleaned.update(
                {
                    "arguments": dict(study.ERASED_PARTICIPANT_TOOL_CONTENT),
                    "structuredResult": dict(study.ERASED_PARTICIPANT_TOOL_CONTENT),
                    "visibleText": "",
                }
            )
        retained.append(cleaned)
    if not any(event["type"] == "tool" for event in retained):
        raise CollectorError("interruption is unavailable before public exposure")
    return [*retained, terminal]


def remove_path(path: Path, label: str) -> None:
    """Remove one private working file with a bounded diagnostic."""
    try:
        path.unlink(missing_ok=True)
    except OSError as error:
        raise CollectorError(f"could not remove {label}: {error}") from error


def remove_owned_temporaries(path: Path, label: str) -> int:
    """Remove only same-directory temporary files owned by one canonical path."""
    prefix = f".{path.name}."
    removed = 0
    try:
        candidates = list(path.parent.iterdir()) if path.parent.exists() else []
    except OSError as error:
        raise CollectorError(f"could not inspect {label} temporaries: {error}") from error
    for candidate in candidates:
        if not candidate.name.startswith(prefix) or not candidate.name.endswith(".tmp"):
            continue
        if candidate.parent.resolve() != path.parent.resolve():
            raise CollectorError(f"{label} temporary escaped its owned directory")
        remove_path(candidate, f"orphan {label} temporary")
        removed += 1
    return removed


def session_actions(
    bank: dict[str, Any], manifest: dict[str, Any], session_id: str
) -> list[dict[str, Any]]:
    """Build the private deterministic state machine without exposing future probes."""
    _pairs, sessions = study.manifest_indexes(manifest)
    if session_id not in sessions:
        raise CollectorError(f"unknown session id {session_id}")
    pair, session = sessions[session_id]
    rooms = study.encounter_rooms()
    actions: list[dict[str, Any]] = []
    for room_id in pair["roomOrder"]:
        room = rooms[room_id]
        calls = study.condition_calls(room, session["condition"])
        if session["condition"] == study.CONDITIONS[0]:
            actions.append(
                {"kind": "tool", "room": room_id, "sequence": 1, "call": calls[0]}
            )
            actions.append(
                {
                    "kind": "condition_response",
                    "room": room_id,
                    "stage": "construction"
                    if room_id == "formula-jam"
                    else "prediction",
                    "prompt": room["generationPrompt"],
                    "responseSchema": room["generationAnswerSchema"],
                }
            )
            actions.append(
                {"kind": "tool", "room": room_id, "sequence": 2, "call": calls[1]}
            )
            actions.append(
                {
                    "kind": "feedback",
                    "room": room_id,
                    "expectedAnswer": room["expectedAnswer"],
                    "text": room["feedbackText"],
                    "participantCorrect": True,
                }
            )
            actions.extend(
                {"kind": "tool", "room": room_id, "sequence": sequence, "call": call}
                for sequence, call in enumerate(calls[2:], start=3)
            )
            if room_id == "formula-jam":
                actions.append(
                    {
                        "kind": "material",
                        "room": room_id,
                        "text": room["revealMaterial"],
                    }
                )
        else:
            actions.append(
                {"kind": "tool", "room": room_id, "sequence": 1, "call": calls[0]}
            )
            if room_id == "formula-jam":
                actions.append(
                    {
                        "kind": "material",
                        "room": room_id,
                        "text": room["revealMaterial"],
                    }
                )
                actions.append(
                    {
                        "kind": "condition_response",
                        "room": room_id,
                        "stage": "elaboration",
                        "prompt": room["controlPrompt"],
                    }
                )
                interaction_index = 1
            else:
                actions.append(
                    {"kind": "tool", "room": room_id, "sequence": 2, "call": calls[1]}
                )
                actions.append(
                    {
                        "kind": "condition_response",
                        "room": room_id,
                        "stage": "elaboration",
                        "prompt": room["controlPrompt"],
                    }
                )
                interaction_index = 2
            interaction_sequence = interaction_index + 1
            actions.append(
                {
                    "kind": "tool",
                    "room": room_id,
                    "sequence": interaction_sequence,
                    "call": calls[interaction_index],
                }
            )
            actions.append(
                {
                    "kind": "feedback",
                    "room": room_id,
                    "expectedAnswer": room["expectedAnswer"],
                    "text": room["feedbackText"],
                    "participantCorrect": None,
                }
            )
            actions.extend(
                {
                    "kind": "tool",
                    "room": room_id,
                    "sequence": sequence,
                    "call": call,
                }
                for sequence, call in enumerate(
                    calls[interaction_index + 1 :], start=interaction_sequence + 1
                )
            )
    actions.extend(
        {"kind": "probe", "phase": "immediate", "probe": probe}
        for probe in study.probe_sequence(bank, pair["roomOrder"], "immediate")
    )
    actions.extend(
        {"kind": "distractor", "item": item}
        for item in bank["distractorSequence"]["items"]
    )
    actions.extend(
        {"kind": "probe", "phase": "late", "probe": probe}
        for probe in study.probe_sequence(bank, pair["roomOrder"], "late")
    )
    return actions


def current_request_id(state: dict[str, Any]) -> str:
    """Bind a participant response to exactly one cursor and repair state."""
    return study.content_sha256(
        {
            "schemaVersion": STATE_SCHEMA,
            "sessionId": state["sessionId"],
            "cursor": state["cursor"],
            "repairPending": state["repairPending"],
            "consentPending": state["consentPending"],
            "manifestSha256": state["manifestSha256"],
        }
    )


def stop_request(state: dict[str, Any]) -> dict[str, Any]:
    """Expose the exact participant object that stops the active arm."""
    return {
        "kind": "stop",
        "requestId": current_request_id(state),
        "submit": {"terminalAction": "stop"},
    }


def withdrawal_request(
    state: dict[str, Any], pair_published: bool
) -> dict[str, Any] | None:
    """Expose a stable participant credential only while pair data is provisional."""
    if pair_published:
        return None
    request_id = study.content_sha256(
        {
            "schemaVersion": STATE_SCHEMA,
            "kind": "pair-withdrawal",
            "pairId": state["pairId"],
            "manifestSha256": state["manifestSha256"],
            "withdrawalNonce": state["withdrawalNonce"],
        }
    )
    return {
        "kind": "withdrawal",
        "requestId": request_id,
        "submit": {"terminalAction": "withdraw"},
        "availableUntil": "pair-aggregation",
    }


def consent_output(state: dict[str, Any]) -> dict[str, Any]:
    """Return the frozen participant-facing consent request before any exposure."""
    return {
        "status": "awaiting_consent",
        "deliveries": [],
        "responseRequest": {
            "kind": "consent",
            "requestId": current_request_id(state),
            "text": CONSENT_TEXT,
            "responseSchema": {
                "participate": "boolean",
                "publicationConsent": "aggregate-only or bounded-raw when participating",
            },
        },
        "stopRequest": None,
        "withdrawalRequest": None,
    }


def resolved_action_call(
    action: dict[str, Any], prior_events: list[dict[str, Any]]
) -> dict[str, Any]:
    """Resolve the one participant-authored Formula call from sealed prior evidence."""
    call = action["call"]
    if call["arguments"].get("expr") != "__PARTICIPANT_EXPRESSION__":
        return call
    constructions = [
        event
        for event in prior_events
        if event.get("type") == "condition_response"
        and event.get("room") == action["room"]
        and event.get("stage") == "construction"
    ]
    if len(constructions) != 1:
        raise CollectorError("Formula Jam construction receipt is missing or duplicated")
    room = study.encounter_rooms()[action["room"]]
    expression = study.validate_generation_answer(room, constructions[0].get("answer"))
    return {**call, "arguments": {"expr": expression}}


def feedback_event(
    action: dict[str, Any], session_id: str, prior_events: list[dict[str, Any]]
) -> dict[str, Any]:
    """Build fixed outcome feedback with one bounded response-contingent bit."""
    participant_correct = action["participantCorrect"]
    if participant_correct is True:
        responses = [
            event
            for event in prior_events
            if event.get("type") == "condition_response"
            and event.get("room") == action["room"]
            and event.get("stage") in ("prediction", "construction")
        ]
        if len(responses) != 1:
            raise CollectorError("generation response is missing before feedback")
        if action["room"] == "formula-jam":
            interaction_results = [
                event.get("structuredResult")
                for event in prior_events
                if event.get("type") == "tool"
                and event.get("room") == action["room"]
                and event.get("role") == "interaction"
            ]
            participant_correct = bool(
                len(interaction_results) == 1
                and interaction_results[0]
                and interaction_results[0].get("expression")
            )
        else:
            room = study.encounter_rooms()[action["room"]]
            participant_correct = (
                study.validate_generation_answer(room, responses[0].get("answer"))
                == action["expectedAnswer"]
            )
    return {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "feedback",
        "sessionId": session_id,
        "room": action["room"],
        "expectedAnswer": action["expectedAnswer"],
        "participantCorrect": participant_correct,
        "text": action["text"],
    }


def response_request(
    action: dict[str, Any], request_id: str, repair: bool = False
) -> dict[str, Any]:
    """Project only the current participant request."""
    if action["kind"] == "condition_response":
        if action["stage"] in ("prediction", "construction"):
            return {
                "kind": "condition_response",
                "requestId": request_id,
                "room": action["room"],
                "stage": action["stage"],
                "prompt": action["prompt"],
                "responseSchema": {
                    "answer": action["responseSchema"],
                    "rationale": "string, 12 through 256 characters",
                },
            }
        return {
            "kind": "condition_response",
            "requestId": request_id,
            "room": action["room"],
            "stage": action["stage"],
            "prompt": action["prompt"],
            "responseSchema": {"text": "string, 12 through 256 characters"},
        }
    if action["kind"] == "probe":
        packet = study.public_probe(action["probe"], schema_only=repair)
        return {
            "kind": "probe_repair" if repair else "probe",
            "requestId": request_id,
            **packet,
        }
    if action["kind"] == "distractor":
        return {
            "kind": "distractor",
            "requestId": request_id,
            "itemId": action["item"]["id"],
            "prompt": action["item"]["prompt"],
            "responseSchema": {"answer": "scalar"},
        }
    raise CollectorError(f"action {action['kind']} does not accept a response")


def session_events(
    manifest: dict[str, Any], ledger: Path, session_id: str
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Verify the cohort chain and return all events plus one session's body."""
    events = read_verified_ledger(ledger, manifest)
    headers = [
        event
        for event in events
        if event.get("sessionId") == session_id and event.get("type") == "session"
    ]
    if len(headers) != 1:
        raise CollectorError("collector session must have exactly one sealed header")
    _pairs, sessions = study.manifest_indexes(manifest)
    pair, session = sessions[session_id]
    study.validate_session_header(headers[0], pair, session)
    body = [
        event
        for event in events
        if event.get("sessionId") == session_id and event.get("type") != "session"
    ]
    return [headers[0], *body], body


def replay_deliveries(
    manifest: dict[str, Any], ledger: Path, session_id: str
) -> list[dict[str, Any]]:
    """Replay the current receipt-backed stimulus batch after an ambiguous failure."""
    _events, body = session_events(manifest, ledger, session_id)
    last_response = -1
    for index, event in enumerate(body):
        if event.get("type") in (
            "condition_response",
            "distractor_response",
            "response",
            "response_refusal",
        ):
            last_response = index
    return [
        {key: value for key, value in event.items() if key != "_sourceIndex"}
        for event in body[last_response + 1 :]
        if event.get("type") in ("tool", "material", "feedback")
    ]


def reconcile_state(
    state: dict[str, Any],
    bank: dict[str, Any],
    manifest: dict[str, Any],
    ledger: Path,
) -> None:
    """Derive mutable progress from immutable receipts and reject skipped actions."""
    actions = session_actions(bank, manifest, state["sessionId"])
    events, body = session_events(manifest, ledger, state["sessionId"])
    terminal = (
        body[-1]
        if body and body[-1].get("type") in ("session_complete", "session_interruption")
        else None
    )
    action_body = body[:-1] if terminal is not None else body
    if terminal is not None and terminal["type"] == "session_interruption":
        if any(
            event.get("type")
            in (
                "response",
                "response_refusal",
                "distractor_response",
                "condition_response",
            )
            for event in action_body
        ):
            raise CollectorError("interrupted session retains participant content")
        if not any(event.get("type") == "tool" for event in action_body):
            raise CollectorError("interrupted session has no public exposure")
        _pairs, sessions = study.manifest_indexes(manifest)
        pair, session = sessions[state["sessionId"]]
        header = next(event for event in events if event.get("type") == "session")
        study.validate_and_score_session(
            bank, pair, session, header, terminal, action_body
        )
        state.update(
            {
                "cursor": 0,
                "repairUsed": False,
                "repairPending": False,
                "complete": True,
            }
        )
        return
    cursor = 0
    body_index = 0
    repair_used = False
    repair_pending = False
    while cursor < len(actions) and body_index < len(action_body):
        action = actions[cursor]
        event = action_body[body_index]
        kind = action["kind"]
        if kind == "tool":
            call = resolved_action_call(action, action_body[:body_index])
            matches = (
                event.get("type") == "tool"
                and event.get("room") == action["room"]
                and event.get("sequence") == action["sequence"]
                and event.get("role") == call["role"]
                and event.get("tool") == call["tool"]
                and study.canonical_bytes(event.get("arguments"))
                == study.canonical_bytes(call["arguments"])
            )
            if not matches:
                raise CollectorError(
                    "collector tool receipts do not match the action state"
                )
        elif kind == "material":
            if (
                event.get("type") != "material"
                or event.get("room") != action["room"]
                or event.get("text") != action["text"]
            ):
                raise CollectorError(
                    "collector material receipt does not match the action state"
                )
        elif kind == "condition_response":
            if (
                event.get("type") != "condition_response"
                or event.get("room") != action["room"]
                or event.get("stage") != action["stage"]
            ):
                raise CollectorError("collector condition response is out of order")
        elif kind == "feedback":
            expected_feedback = feedback_event(
                action, state["sessionId"], action_body[:body_index]
            )
            observed_feedback = {
                key: value for key, value in event.items() if key != "_sourceIndex"
            }
            if study.canonical_bytes(observed_feedback) != study.canonical_bytes(
                expected_feedback
            ):
                raise CollectorError("collector feedback receipt is out of order")
        elif kind == "distractor":
            if (
                event.get("type") != "distractor_response"
                or event.get("itemId") != action["item"]["id"]
            ):
                raise CollectorError("collector distractor response is out of order")
        elif kind == "probe":
            matches_probe = (
                event.get("type") in ("response", "response_refusal")
                and event.get("phase") == action["phase"]
                and event.get("probeId") == action["probe"]["id"]
            )
            if not matches_probe:
                raise CollectorError("collector probe response is out of order")
            if event["type"] == "response":
                valid, _correct = study.score_answer(action["probe"], event["answer"])
                if event.get("attempt") != 1:
                    raise CollectorError(
                        "collector probe response starts with the wrong attempt"
                    )
                if not valid and not repair_used:
                    repair_used = True
                    body_index += 1
                    if body_index == len(action_body):
                        repair_pending = True
                        break
                    retry = action_body[body_index]
                    if (
                        retry.get("type") not in ("response", "response_refusal")
                        or retry.get("phase") != action["phase"]
                        or retry.get("probeId") != action["probe"]["id"]
                        or (
                            retry.get("type") == "response"
                            and retry.get("attempt") != 2
                        )
                    ):
                        raise CollectorError("collector schema repair is out of order")
        else:
            raise CollectorError(f"unsupported collector action {kind}")
        cursor += 1
        body_index += 1
    if body_index != len(action_body):
        raise CollectorError("collector receipt ledger contains extra session events")
    if terminal is not None:
        if cursor != len(actions) or repair_pending:
            raise CollectorError("completed session receipt precedes its final action")
        _pairs, sessions = study.manifest_indexes(manifest)
        pair, session = sessions[state["sessionId"]]
        header = next(event for event in events if event.get("type") == "session")
        study.validate_and_score_session(
            bank, pair, session, header, terminal, action_body
        )
    state.update(
        {
            "cursor": cursor,
            "repairUsed": repair_used,
            "repairPending": repair_pending,
            "complete": terminal is not None,
        }
    )


def validate_collection_start(
    manifest: dict[str, Any], events: list[dict[str, Any]], session_id: str
) -> None:
    """Enforce pair order and within-pair crossover order before collection."""
    _pairs, sessions = study.manifest_indexes(manifest)
    pair, _session = sessions[session_id]
    headers = {event["sessionId"] for event in events if event["type"] == "session"}
    terminals = {
        event["sessionId"]
        for event in events
        if event["type"] in ("session_complete", "session_interruption")
    }
    outcomes = {
        event["pairId"]
        for event in events
        if event["type"] in ("withdrawal", "infrastructure_failure")
    }
    if pair["pairId"] in outcomes:
        raise CollectorError("allocated pair was already consumed by a terminal outcome")
    if session_id in headers:
        raise CollectorError("session id was already collected")
    completed_pairs = sum(
        candidate["modelFamily"] == pair["modelFamily"]
        and candidate["pairId"] not in outcomes
        and set(candidate["collectionOrder"]).issubset(terminals)
        for candidate in manifest["pairs"]
    )
    if completed_pairs >= 10:
        raise CollectorError("the model family already has ten qualifying pairs")
    for prior in manifest["pairs"]:
        if (
            prior["modelFamily"] != pair["modelFamily"]
            or prior["order"] >= pair["order"]
        ):
            continue
        prior_ids = set(prior["collectionOrder"])
        if prior["pairId"] not in outcomes and not prior_ids.issubset(terminals):
            raise CollectorError("an earlier allocated pair is not terminal")
    first, second = pair["collectionOrder"]
    expected = second if first in terminals else first
    if session_id != expected:
        raise CollectorError(
            "session differs from the frozen crossover collection order"
        )


def advance(
    state: dict[str, Any],
    bank: dict[str, Any],
    manifest: dict[str, Any],
    ledger: Path,
    tool_caller: ToolCaller | None = None,
) -> dict[str, Any]:
    """Run deterministic actions until the next participant response or completion."""
    actions = session_actions(bank, manifest, state["sessionId"])
    if tool_caller is None:
        source_sha256 = state["headerDraft"]["studySourceSha256"]

        def bound_tool_caller(
            tool: str, arguments: dict[str, Any]
        ) -> tuple[dict[str, Any], dict[str, Any]]:
            return mcp_play.isolated_tool_call(
                tool,
                arguments,
                expected_revision=state["numinousCommit"],
                expected_source_sha256=source_sha256,
            )

        tool_caller = bound_tool_caller
    while state["cursor"] < len(actions):
        action = actions[state["cursor"]]
        if action["kind"] in ("condition_response", "probe", "distractor"):
            return {
                "status": "awaiting_response",
                "deliveries": replay_deliveries(
                    manifest, ledger, state["sessionId"]
                ),
                "responseRequest": response_request(
                    action,
                    current_request_id(state),
                    repair=state["repairPending"],
                ),
                "stopRequest": stop_request(state),
                "withdrawalRequest": withdrawal_request(state, False),
            }
        if action["kind"] == "tool":
            prior_events = read_verified_ledger(ledger, manifest)
            call = resolved_action_call(action, prior_events)
            try:
                initialization, result = tool_caller(call["tool"], call["arguments"])
            except Exception as error:
                raise CollectorError(
                    f"isolated MCP call {call['tool']} failed before receipt: {error}"
                ) from error
            if not isinstance(initialization, dict) or not isinstance(result, dict):
                raise CollectorError("isolated MCP caller returned an invalid shape")
            if initialization.get("protocolVersion") != study.MCP_PROTOCOL_REVISION:
                raise CollectorError("isolated MCP protocol revision differs")
            binary_sha256 = initialization.get("numinousBinarySha256")
            if not isinstance(binary_sha256, str) or not study.SHA256_HEX.fullmatch(
                binary_sha256
            ):
                raise CollectorError("isolated MCP binary digest is missing or invalid")
            build_receipt = initialization.get("binaryBuildReceipt")
            try:
                study.validate_mcp_build_receipt(build_receipt, binary_sha256)
            except study.StudyError as error:
                raise CollectorError(f"isolated MCP build receipt differs: {error}") from error
            if (
                build_receipt["sourceRevision"] != state["numinousCommit"]
                or build_receipt["studySourceSha256"]
                != state["headerDraft"]["studySourceSha256"]
            ):
                raise CollectorError("isolated MCP build source identity differs")
            projection = project_mcp_result(
                call["tool"],
                result,
                call["arguments"],
                initialization.get("serverInfo"),
            )
            if call["role"] == "interaction":
                study.validate_feedback_evidence(
                    study.encounter_rooms()[action["room"]],
                    projection,
                    call["arguments"],
                )
            event = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "tool",
                "sessionId": state["sessionId"],
                "room": action["room"],
                "sequence": action["sequence"],
                "role": call["role"],
                "tool": call["tool"],
                "arguments": call["arguments"],
                "structuredResult": projection,
                "visibleText": tool_text(result),
                "toolOutcome": "success",
                "binarySha256": binary_sha256,
                "binaryBuildReceipt": build_receipt,
            }
            append_receipt(ledger, manifest, event)
        elif action["kind"] == "feedback":
            event = feedback_event(
                action,
                state["sessionId"],
                read_verified_ledger(ledger, manifest),
            )
            append_receipt(ledger, manifest, event)
        elif action["kind"] == "material":
            event = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "material",
                "sessionId": state["sessionId"],
                "room": action["room"],
                "kind": "reveal",
                "text": action["text"],
                "materialSha256": study.content_sha256(action["text"]),
            }
            append_receipt(ledger, manifest, event)
        state["cursor"] += 1
    completion = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "session_complete",
        "sessionId": state["sessionId"],
    }
    append_receipt(ledger, manifest, completion)
    state["complete"] = True
    return {
        "status": "complete",
        "deliveries": replay_deliveries(manifest, ledger, state["sessionId"]),
        "responseRequest": None,
        "stopRequest": None,
        "withdrawalRequest": withdrawal_request(state, False),
    }


def record_response(
    state: dict[str, Any],
    bank: dict[str, Any],
    manifest: dict[str, Any],
    ledger: Path,
    response: Any,
    tool_caller: ToolCaller | None = None,
) -> dict[str, Any]:
    """Record exactly the current bounded response, then advance once."""
    actions = session_actions(bank, manifest, state["sessionId"])
    if state["cursor"] >= len(actions):
        raise CollectorError("collector has no pending response")
    action = actions[state["cursor"]]
    if not isinstance(response, dict) or response.get("requestId") != current_request_id(
        state
    ):
        raise CollectorError("participant response requestId is stale or invalid")
    payload = {key: value for key, value in response.items() if key != "requestId"}
    if action["kind"] == "condition_response":
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "condition_response",
            "sessionId": state["sessionId"],
            "room": action["room"],
            "stage": action["stage"],
        }
        if action["stage"] in ("prediction", "construction"):
            if set(payload) != {"answer", "rationale"}:
                raise CollectorError(
                    "generation response must contain only answer and rationale"
                )
            room = study.encounter_rooms()[action["room"]]
            event.update(
                {
                    "answer": study.validate_generation_answer(
                        room, payload["answer"]
                    ),
                    "rationale": payload["rationale"],
                }
            )
        else:
            if set(payload) != {"text"}:
                raise CollectorError("explanation response must contain only text")
            event["text"] = payload["text"]
        study.validate_event_shape(event)
        study.assert_sanitized(event)
        append_receipt(ledger, manifest, event)
        state["cursor"] += 1
    elif action["kind"] == "distractor":
        if set(payload) != {"answer"}:
            raise CollectorError("distractor response must contain only answer")
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "distractor_response",
            "sessionId": state["sessionId"],
            "itemId": action["item"]["id"],
            "answer": payload["answer"],
        }
        append_receipt(ledger, manifest, event)
        state["cursor"] += 1
    elif action["kind"] == "probe":
        probe = action["probe"]
        if payload == {"refuse": True}:
            event = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "response_refusal",
                "sessionId": state["sessionId"],
                "phase": action["phase"],
                "probeId": probe["id"],
            }
            append_receipt(ledger, manifest, event)
            state["cursor"] += 1
            state["repairPending"] = False
        else:
            if set(payload) != {"answer"}:
                raise CollectorError(
                    "probe response must contain only answer or refuse true"
                )
            attempt = 2 if state["repairPending"] else 1
            event = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "response",
                "sessionId": state["sessionId"],
                "phase": action["phase"],
                "probeId": probe["id"],
                "attempt": attempt,
                "answer": payload["answer"],
            }
            append_receipt(ledger, manifest, event)
            valid, _correct = study.score_answer(probe, payload["answer"])
            if not valid and not state["repairUsed"]:
                state["repairUsed"] = True
                state["repairPending"] = True
                return {
                    "status": "awaiting_response",
                    "deliveries": [],
                    "responseRequest": response_request(
                        action, current_request_id(state), repair=True
                    ),
                    "stopRequest": stop_request(state),
                    "withdrawalRequest": withdrawal_request(state, False),
                }
            state["cursor"] += 1
            state["repairPending"] = False
    else:
        raise CollectorError(
            f"collector expected deterministic action {action['kind']}"
        )
    return advance(state, bank, manifest, ledger, tool_caller=tool_caller)


def read_input(path: str) -> Any:
    """Read one bounded ephemeral JSON response from stdin."""
    if path != "-":
        raise CollectorError("participant responses are accepted only from stdin")
    try:
        text = sys.stdin.read(MAX_RESPONSE_BYTES + 1)
        if len(text.encode("utf-8")) > MAX_RESPONSE_BYTES:
            raise CollectorError("participant response exceeds the input limit")
        return study.strict_json_loads(text, "participant response")
    except OSError as error:
        raise CollectorError(f"could not read participant response: {error}") from error
    except study.StudyError as error:
        raise CollectorError(
            f"participant response is invalid JSON: {error}"
        ) from error


def load_inputs(
    args: argparse.Namespace, *, require_committed_manifest: bool = True
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Load and cross-check the private bank and frozen allocation."""
    bank = study.load_bank(args.bank)
    require_distinct_paths(
        {
            "private probe bank": args.bank,
            "allocation manifest": args.manifest,
            "collector ledger": args.ledger,
        }
    )
    if require_committed_manifest:
        require_committed_file(args.manifest, "allocation manifest")
    manifest = study.load_manifest(args.manifest, bank)
    if manifest["encounterSpecSha256"] != study.content_sha256(
        study.load_encounter_spec()
    ):
        raise CollectorError("manifest encounter specification commitment differs")
    return bank, manifest


@serialize_state_transition
def command_start(args: argparse.Namespace) -> None:
    state_path = require_agent_path(args.state, "collector state")
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_agent_path(args.bank, "private probe bank")
    require_distinct_paths(
        {
            "collector state": state_path,
            "collector ledger": ledger,
            "private probe bank": args.bank,
            "allocation manifest": args.manifest,
        }
    )
    if state_path.exists():
        raise CollectorError(f"collector state already exists: {state_path}")
    bank, manifest = load_inputs(args)
    _pairs, sessions = study.manifest_indexes(manifest)
    if args.session_id not in sessions:
        raise CollectorError(f"unknown session id {args.session_id}")
    pair, session = sessions[args.session_id]
    existing_events = collection_evidence(ledger, manifest)
    validate_collection_start(manifest, existing_events, args.session_id)
    if not study.SHA256_HEX.fullmatch(args.context_id):
        raise CollectorError("context id must be an opaque SHA-256 value")
    if any(
        event.get("contextId") == args.context_id and event["type"] == "session"
        for event in existing_events
    ):
        raise CollectorError("context id was already used")
    context_tombstone = study.content_sha256(args.context_id)
    if any(
        event.get("type") == "withdrawal"
        and context_tombstone in event.get("contextTombstones", [])
        for event in existing_events
    ):
        raise CollectorError("context id was already exposed before withdrawal")
    if args.model_identifier != pair["modelFamily"]:
        raise CollectorError("model identifier differs from the frozen allocation")
    if args.backend_revision != pair["calibratedBackendRevision"]:
        raise CollectorError("backend revision differs from the calibrated allocation")
    prior_state_path: Path | None = None
    prior_state: dict[str, Any] | None = None
    pair_state_paths = [str(state_path)]
    withdrawal_nonce = secrets.token_hex(32)
    first_session_id, second_session_id = pair["collectionOrder"]
    supplied_prior = getattr(args, "prior_state", None)
    if args.session_id == first_session_id:
        if supplied_prior is not None:
            raise CollectorError("the first arm cannot name a prior collector state")
    elif args.session_id == second_session_id:
        if supplied_prior is None:
            raise CollectorError("the second arm requires the completed first-arm state")
        prior_state_path = require_agent_path(
            supplied_prior, "completed first-arm collector state"
        )
        if prior_state_path == state_path:
            raise CollectorError("paired collector state paths must differ")
        prior_state = load_state(prior_state_path)
        if (
            prior_state["sessionId"] != first_session_id
            or prior_state["pairId"] != pair["pairId"]
            or prior_state["collectionOrder"] != pair["collectionOrder"]
            or Path(prior_state["cohortLedger"]) != ledger
            or prior_state["manifestSha256"] != study.content_sha256(manifest)
            or prior_state["complete"] is not True
            or prior_state["consentPending"] is True
            or prior_state["pairStatePaths"] != [str(prior_state_path)]
        ):
            raise CollectorError("completed first-arm collector state differs")
        pair_state_paths = [str(prior_state_path), str(state_path)]
        withdrawal_nonce = prior_state["withdrawalNonce"]
    else:
        raise CollectorError("session is outside the pair collection order")
    session_ledger = pending_session_ledger(ledger, args.session_id)
    require_distinct_paths(
        {
            "collector state": state_path,
            "collector ledger": ledger,
            "provisional session ledger": session_ledger,
            "private probe bank": args.bank,
            "allocation manifest": args.manifest,
        }
    )
    if session_ledger.exists():
        raise CollectorError("provisional session ledger already exists")
    commit = repository_commit()
    source_sha256 = study_source_sha256(commit)
    if source_sha256 != manifest["calibrationRunnerSourceSha256"]:
        raise CollectorError("study runtime source differs from calibrated source")
    start_commitment = study.attempt_start_commitment(
        phase="collection",
        root_sha256=study.content_sha256(manifest),
        start_key=args.session_id,
        model_identifier=args.model_identifier,
        context_id=args.context_id,
        backend_revision=args.backend_revision,
        runner_revision=commit,
        runner_source_sha256=source_sha256,
    )
    attempt_start_receipt_sha256 = require_attempt_start_receipt(
        getattr(args, "start_receipt", None), start_commitment
    )
    header_draft = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "session",
        "sessionId": args.session_id,
        "consent": True,
        "modelFamily": pair["modelFamily"],
        "modelIdentifier": args.model_identifier,
        "provider": study.MODEL_PROVIDERS[pair["modelFamily"]],
        "backendRevision": args.backend_revision,
        "reasoningEffort": pair["reasoningEffort"],
        "settings": {"sampling": "platform-default", "freshContext": True},
        "date": date.today().isoformat(),
        "numinousCommit": commit,
        "mcpProtocolRevision": study.MCP_PROTOCOL_REVISION,
        "operatingSystem": platform.system().casefold(),
        "runnerVersion": study.RUNNER_VERSION,
        "studySourceSha256": source_sha256,
        "attemptStartReceiptSha256": attempt_start_receipt_sha256,
        "condition": session["condition"],
        "contextId": args.context_id,
        "capabilityPolicy": "collector-only-no-repository-web-or-tools",
    }
    state = {
        "schemaVersion": STATE_SCHEMA,
        "sessionId": args.session_id,
        "cursor": 0,
        "repairUsed": False,
        "repairPending": False,
        "complete": False,
        "manifestSha256": study.content_sha256(manifest),
        "probeBankSha256": study.content_sha256(bank),
        "sessionLedger": str(session_ledger),
        "cohortLedger": str(ledger),
        "numinousCommit": commit,
        "pairId": pair["pairId"],
        "collectionOrder": pair["collectionOrder"],
        "pairStatePaths": pair_state_paths,
        "withdrawalNonce": withdrawal_nonce,
        "consentPending": True,
        "refusalOrdinal": None,
        "headerDraft": header_draft,
        "manifestSnapshot": manifest,
    }
    write_state_once(state_path, state)
    prior_original = prior_state
    try:
        if prior_state_path is not None and prior_state is not None:
            prior_state = {**prior_state, "pairStatePaths": pair_state_paths}
            write_state(prior_state_path, prior_state)
        claim_active(ledger, state_path, args.session_id)
    except Exception:
        if prior_state_path is not None and prior_original is not None:
            write_state(prior_state_path, prior_original)
        remove_state_if_exact(state_path, state)
        raise
    print(json.dumps(consent_output(state), ensure_ascii=False, sort_keys=True, indent=2))


def load_bound_session(
    args: argparse.Namespace,
) -> tuple[
    Path,
    Path,
    Path,
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]:
    """Load one private state and verify every immutable path and commitment."""
    state_path = require_agent_path(args.state, "collector state")
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_agent_path(args.bank, "private probe bank")
    require_distinct_paths(
        {
            "collector state": state_path,
            "collector ledger": ledger,
            "private probe bank": args.bank,
            "allocation manifest": args.manifest,
        }
    )
    bank, manifest = load_inputs(args)
    state = load_state(state_path)
    current_commit = repository_commit()
    if current_commit != state["numinousCommit"]:
        raise CollectorError("repository revision changed during collection")
    if study_source_sha256(current_commit) != manifest[
        "calibrationRunnerSourceSha256"
    ]:
        raise CollectorError("study runtime source differs from calibrated source")
    if state["manifestSha256"] != study.content_sha256(manifest):
        raise CollectorError("collector state manifest commitment differs")
    if state["probeBankSha256"] != study.content_sha256(bank):
        raise CollectorError("collector state probe bank commitment differs")
    if Path(state["cohortLedger"]) != ledger:
        raise CollectorError("collector state cohort ledger differs")
    _pairs, sessions = study.manifest_indexes(manifest)
    if state["sessionId"] not in sessions:
        raise CollectorError("collector state session is outside the allocation")
    pair, _session = sessions[state["sessionId"]]
    if (
        state["pairId"] != pair["pairId"]
        or state["collectionOrder"] != pair["collectionOrder"]
    ):
        raise CollectorError("collector state pair binding differs")
    provisional_ledger = require_agent_path(
        Path(state["sessionLedger"]), "provisional session ledger"
    )
    if provisional_ledger != pending_session_ledger(ledger, state["sessionId"]):
        raise CollectorError("collector state provisional ledger differs")
    pair_session_ids = set(pair["collectionOrder"])
    published_events = [
        event
        for event in read_verified_ledger(ledger, manifest)
        if event.get("sessionId") in pair_session_ids
    ]
    published_terminals = {
        event["sessionId"]
        for event in published_events
        if event.get("type") in ("session_complete", "session_interruption")
    }
    if published_events and published_terminals != pair_session_ids:
        raise CollectorError("settled pair evidence is partial")
    session_ledger = ledger if published_terminals else provisional_ledger
    if active_path(ledger).exists():
        require_active(ledger, state_path, state["sessionId"])
    return state_path, ledger, session_ledger, bank, manifest, state, pair


def recruitment_refusals(
    ledger: Path, manifest: dict[str, Any], model_family: str
) -> list[dict[str, Any]]:
    """Return one model family's exact contiguous aggregate refusal sequence."""
    refusals = [
        event
        for event in read_verified_ledger(ledger, manifest)
        if event.get("type") == "recruitment_refusal"
        and event.get("modelFamily") == model_family
    ]
    for ordinal, event in enumerate(refusals, start=1):
        if event.get("familyRefusalOrdinal") != ordinal:
            raise CollectorError("recruitment refusal ordinals are not contiguous")
    return refusals


def recover_consent_commit(
    state_path: Path,
    session_ledger: Path,
    manifest: dict[str, Any],
    state: dict[str, Any],
    pair: dict[str, Any],
) -> bool:
    """Repair mutable consent state from an already sealed session header."""
    if not state["consentPending"] or not session_ledger.exists():
        return False
    events = read_verified_ledger(session_ledger, manifest)
    if not events:
        return False
    if len(events) != 1 or events[0].get("type") != "session":
        raise CollectorError("stale consent state has unexpected provisional receipts")
    _pairs, sessions = study.manifest_indexes(manifest)
    _bound_pair, session = sessions[state["sessionId"]]
    study.validate_session_header(events[0], pair, session)
    expected_header = {
        **state["headerDraft"],
        "publicationConsent": events[0]["publicationConsent"],
    }
    observed_header = {
        key: value for key, value in events[0].items() if key != "_sourceIndex"
    }
    if study.canonical_bytes(observed_header) != study.canonical_bytes(expected_header):
        raise CollectorError("sealed consent header differs from collector state")
    state["consentPending"] = False
    write_state(state_path, state)
    return True


@serialize_state_transition
def command_respond(args: argparse.Namespace) -> None:
    state_path, ledger, session_ledger, bank, manifest, state, pair = (
        load_bound_session(args)
    )
    consent_recovered = recover_consent_commit(
        state_path, session_ledger, manifest, state, pair
    )
    if consent_recovered:
        require_active(ledger, state_path, state["sessionId"])
        output = advance(state, bank, manifest, session_ledger)
        write_state(state_path, state)
        print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))
        return
    if state["consentPending"]:
        refusals = recruitment_refusals(ledger, manifest, pair["modelFamily"])
        if state["refusalOrdinal"] is not None:
            expected_refusal = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "recruitment_refusal",
                "modelFamily": pair["modelFamily"],
                "familyRefusalOrdinal": state["refusalOrdinal"],
            }
            matching = [
                event
                for event in refusals
                if event["familyRefusalOrdinal"] == state["refusalOrdinal"]
            ]
            if matching:
                if study.canonical_bytes(
                    clean_receipt_event(matching[0])
                ) != study.canonical_bytes(expected_refusal):
                    raise CollectorError("recruitment refusal transaction differs")
                if active_path(ledger).exists():
                    release_active(ledger, state_path, state["sessionId"])
                remove_owned_temporaries(ledger, "cohort ledger")
                remove_owned_temporaries(state_path, "declined collector state")
                remove_path(state_path, "declined collector state")
                print(
                    json.dumps(
                        {
                            "status": "recruitment_refusal",
                            "responseContentRetainedByCollector": False,
                            "aggregateModelFamilyRefusalRecorded": True,
                        },
                        ensure_ascii=False,
                        sort_keys=True,
                        indent=2,
                    )
                )
                return
        require_active(ledger, state_path, state["sessionId"])
        response = read_input(args.input)
        if not isinstance(response, dict) or response.get(
            "requestId"
        ) != current_request_id(state):
            raise CollectorError("participant response requestId is stale or invalid")
        if response.get("participate") is False and set(response) == {
            "requestId",
            "participate",
        }:
            if state["refusalOrdinal"] is None:
                state["refusalOrdinal"] = len(refusals) + 1
                write_state(state_path, state)
            expected_refusal = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "recruitment_refusal",
                "modelFamily": pair["modelFamily"],
                "familyRefusalOrdinal": state["refusalOrdinal"],
            }
            if state["refusalOrdinal"] != len(refusals) + 1:
                raise CollectorError("recruitment refusal transaction has a gap")
            append_receipt_once(
                ledger,
                manifest,
                expected_refusal,
                ("type", "modelFamily", "familyRefusalOrdinal"),
            )
            if active_path(ledger).exists():
                release_active(ledger, state_path, state["sessionId"])
            else:
                raise CollectorError("collector lease disappeared before refusal cleanup")
            remove_owned_temporaries(ledger, "cohort ledger")
            remove_owned_temporaries(state_path, "declined collector state")
            remove_path(state_path, "declined collector state")
            print(
                json.dumps(
                    {
                        "status": "recruitment_refusal",
                        "responseContentRetainedByCollector": False,
                        "aggregateModelFamilyRefusalRecorded": True,
                    },
                    ensure_ascii=False,
                    sort_keys=True,
                    indent=2,
                )
            )
            return
        if (
            response.get("participate") is not True
            or set(response)
            != {"requestId", "participate", "publicationConsent"}
            or response.get("publicationConsent")
            not in ("aggregate-only", "bounded-raw")
        ):
            raise CollectorError("consent response is invalid")
        header = {
            **state["headerDraft"],
            "publicationConsent": response["publicationConsent"],
        }
        append_receipt(session_ledger, manifest, header)
        state["consentPending"] = False
        write_state(state_path, state)
        output = advance(state, bank, manifest, session_ledger)
        write_state(state_path, state)
        print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))
        return
    reconcile_state(state, bank, manifest, session_ledger)
    if state["complete"]:
        deliveries = replay_deliveries(manifest, session_ledger, state["sessionId"])
        pair_published = publish_pair(ledger, manifest, pair)
        if active_path(ledger).exists():
            release_active(ledger, state_path, state["sessionId"])
        print(
            json.dumps(
                {
                    "status": "complete",
                    "deliveries": deliveries,
                    "responseRequest": None,
                    "withdrawalRequest": withdrawal_request(state, pair_published),
                },
                ensure_ascii=False,
                sort_keys=True,
                indent=2,
            )
        )
        return
    require_active(ledger, state_path, state["sessionId"])
    participant_response = read_input(args.input)
    if participant_response == {
        "requestId": current_request_id(state),
        "terminalAction": "stop",
    }:
        output = settle_interruption(
            state_path,
            ledger,
            session_ledger,
            bank,
            manifest,
            state,
            pair,
            "participant-stop",
            participant_response["requestId"],
        )
        print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))
        return
    output = record_response(
        state,
        bank,
        manifest,
        session_ledger,
        participant_response,
    )
    write_state(state_path, state)
    if state["complete"]:
        pair_published = publish_pair(ledger, manifest, pair)
        release_active(ledger, state_path, state["sessionId"])
        output["withdrawalRequest"] = withdrawal_request(state, pair_published)
    print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))


@serialize_state_transition
def command_status(args: argparse.Namespace) -> None:
    """Recover from an ambiguous process failure without replaying a response."""
    state_path, ledger, session_ledger, bank, manifest, state, pair = (
        load_bound_session(args)
    )
    recover_consent_commit(state_path, session_ledger, manifest, state, pair)
    if state["consentPending"]:
        require_active(ledger, state_path, state["sessionId"])
        print(
            json.dumps(
                consent_output(state), ensure_ascii=False, sort_keys=True, indent=2
            )
        )
        return
    reconcile_state(state, bank, manifest, session_ledger)
    if not state["complete"]:
        require_active(ledger, state_path, state["sessionId"])
        output = advance(state, bank, manifest, session_ledger)
        write_state(state_path, state)
    else:
        output = {
            "status": "complete",
            "deliveries": replay_deliveries(
                manifest, session_ledger, state["sessionId"]
            ),
            "responseRequest": None,
        }
    if state["complete"]:
        pair_published = publish_pair(ledger, manifest, pair)
        output["withdrawalRequest"] = withdrawal_request(state, pair_published)
        if active_path(ledger).exists():
            release_active(ledger, state_path, state["sessionId"])
    print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))


def interruption_stage(
    state: dict[str, Any], bank: dict[str, Any], manifest: dict[str, Any]
) -> str:
    """Derive the interruption stage from the sealed action cursor."""
    actions = session_actions(bank, manifest, state["sessionId"])
    if state["cursor"] >= len(actions):
        return "late"
    action = actions[state["cursor"]]
    if action["kind"] in ("tool", "material", "condition_response", "feedback"):
        return "encounter"
    if action["kind"] == "distractor":
        return "distractor"
    if action["kind"] == "probe":
        return action["phase"]
    raise CollectorError("collector cursor has an unsupported interruption stage")


def terminal_request_sha256(
    state: dict[str, Any], action: str, request_id: str
) -> str:
    """Commit a terminal action to the exact participant request it settles."""
    return study.content_sha256(
        {
            "sessionId": state["sessionId"],
            "requestId": request_id,
            "action": action,
        }
    )


def interruptible_request_ids(
    state: dict[str, Any], bank: dict[str, Any], manifest: dict[str, Any]
) -> set[str]:
    """Accept the current request or the latest response racing with a stop."""
    candidates = {current_request_id(state)}
    actions = session_actions(bank, manifest, state["sessionId"])
    previous_response = next(
        (
            index
            for index in range(min(state["cursor"], len(actions)) - 1, -1, -1)
            if actions[index]["kind"] in ("condition_response", "probe", "distractor")
        ),
        None,
    )
    if previous_response is not None:
        prior = {
            **state,
            "cursor": previous_response,
            "consentPending": False,
            "repairPending": False,
        }
        candidates.add(current_request_id(prior))
        if state["repairUsed"] and actions[previous_response]["kind"] == "probe":
            prior["repairPending"] = True
            candidates.add(current_request_id(prior))
    return candidates


def settle_interruption(
    state_path: Path,
    ledger: Path,
    session_ledger: Path,
    bank: dict[str, Any],
    manifest: dict[str, Any],
    state: dict[str, Any],
    pair: dict[str, Any],
    reason_code: str,
    request_id: str,
) -> dict[str, Any]:
    """Settle one already authenticated stop or infrastructure interruption."""
    if state["consentPending"]:
        raise CollectorError("session cannot be interrupted before consent and exposure")
    if state["complete"]:
        raise CollectorError("completed session cannot be interrupted")
    if request_id not in interruptible_request_ids(state, bank, manifest):
        raise CollectorError("interruption requestId is stale or invalid")
    require_active(ledger, state_path, state["sessionId"])
    terminal = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "session_interruption",
        "sessionId": state["sessionId"],
        "stage": interruption_stage(state, bank, manifest),
        "reasonCode": reason_code,
        "terminalRequestSha256": terminal_request_sha256(
            state,
            "participant-stop" if reason_code == "participant-stop" else "infrastructure",
            request_id,
        ),
    }
    study.validate_event_shape(terminal)
    events = read_verified_ledger(session_ledger, manifest)
    projection = interruption_projection(events, terminal)
    replace_ledger(session_ledger, manifest, projection)
    remove_owned_temporaries(session_ledger, "interrupted provisional ledger")
    state.update({"complete": True, "repairPending": False})
    write_state(state_path, state)
    remove_owned_temporaries(state_path, "interrupted collector state")
    publish_pair(ledger, manifest, pair)
    if active_path(ledger).exists():
        release_active(ledger, state_path, state["sessionId"])
    return {
        "status": "interrupted",
        "stage": terminal["stage"],
        "reasonCode": reason_code,
        "responseContentRetainedByCollector": False,
    }


@serialize_state_transition
def command_interrupt(args: argparse.Namespace) -> None:
    """Settle an operator-observed infrastructure interruption after exposure."""
    state_path, ledger, session_ledger, bank, manifest, state, pair = (
        load_bound_session(args)
    )
    recover_consent_commit(state_path, session_ledger, manifest, state, pair)
    reconcile_state(state, bank, manifest, session_ledger)
    output = settle_interruption(
        state_path,
        ledger,
        session_ledger,
        bank,
        manifest,
        state,
        pair,
        args.reason_code,
        current_request_id(state),
    )
    print(json.dumps(output, ensure_ascii=False, sort_keys=True, indent=2))


@serialize_state_transition
def command_withdraw(args: argparse.Namespace) -> None:
    """Erase provisional pair data and settle only a content-free withdrawal."""
    state_path = require_agent_path(args.state, "collector state")
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_distinct_paths(
        {"collector state": state_path, "collector ledger": ledger}
    )
    state = load_state(state_path)
    participant = read_input(args.input)
    expected_withdrawal = withdrawal_request(state, False)
    if participant != {
        "requestId": expected_withdrawal["requestId"],
        "terminalAction": "withdraw",
    }:
        raise CollectorError("participant withdrawal object is stale or invalid")
    request_id = participant["requestId"]
    if Path(state["cohortLedger"]) != ledger:
        raise CollectorError("collector state cohort ledger differs")
    manifest = state["manifestSnapshot"]
    _pairs, sessions = study.manifest_indexes(manifest)
    if state["sessionId"] not in sessions:
        raise CollectorError("collector state session is outside the allocation")
    pair, _session = sessions[state["sessionId"]]
    if (
        pair["pairId"] != state["pairId"]
        or pair["collectionOrder"] != state["collectionOrder"]
    ):
        raise CollectorError("collector state pair allocation differs")
    pair_state_paths = [
        require_agent_path(Path(path), "paired collector state")
        for path in state["pairStatePaths"]
    ]
    for paired_state_path in pair_state_paths:
        if not paired_state_path.exists():
            continue
        paired_state = load_state(paired_state_path)
        if (
            paired_state["pairId"] != pair["pairId"]
            or paired_state["collectionOrder"] != pair["collectionOrder"]
            or paired_state["pairStatePaths"] != state["pairStatePaths"]
            or paired_state["withdrawalNonce"] != state["withdrawalNonce"]
            or Path(paired_state["cohortLedger"]) != ledger
            or paired_state["manifestSha256"] != state["manifestSha256"]
        ):
            raise CollectorError("paired collector state differs during withdrawal")
    active_marker: dict[str, Any] | None = None
    active_state_path: Path | None = None
    if active_path(ledger).exists():
        active_marker = read_marker(
            active_path(ledger),
            {"schemaVersion", "sessionId", "statePath", "ownerPid"},
            ACTIVE_SCHEMA,
        )
        active_state_path = require_agent_path(
            Path(active_marker["statePath"]), "active paired collector state"
        )
        if (
            active_marker["sessionId"] not in pair["collectionOrder"]
            or active_state_path not in pair_state_paths
        ):
            raise CollectorError("collector lease belongs to a different pair")
    contexts: set[str] = set()
    provisional_exists = False
    for session_id in pair["collectionOrder"]:
        pending = pending_session_ledger(ledger, session_id)
        pending_events = read_verified_ledger(pending, manifest)
        provisional_exists = provisional_exists or bool(pending_events)
        for event in pending_events:
            if event.get("type") == "session":
                contexts.add(event["contextId"])
    if state["consentPending"] and not provisional_exists:
        raise CollectorError(
            "withdrawal is unavailable before consent; decline participation instead"
        )
    settled = read_verified_ledger(ledger, manifest)
    published_pair_events = [
        event
        for event in settled
        if event.get("sessionId") in set(pair["collectionOrder"])
    ]
    if published_pair_events:
        raise CollectorError(
            "pair aggregation has begun and withdrawal cannot erase published responses"
        )
    existing_outcome = next(
        (
            event
            for event in settled
            if event.get("type") == "withdrawal"
            and event.get("pairId") == pair["pairId"]
        ),
        None,
    )
    if existing_outcome is None and not contexts:
        raise CollectorError("withdrawal has no provisional context to erase")
    if existing_outcome is None:
        append_receipt_once(
            ledger,
            manifest,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "withdrawal",
                "pairId": pair["pairId"],
                "contextTombstones": sorted(
                    study.content_sha256(context_id) for context_id in contexts
                ),
                "terminalRequestSha256": terminal_request_sha256(
                    state, "participant-withdrawal", request_id
                ),
            },
            ("type", "pairId"),
        )
    for session_id in pair["collectionOrder"]:
        pending = pending_session_ledger(ledger, session_id)
        remove_ledger_artifacts(pending, "withdrawn provisional session ledger")
    remove_owned_temporaries(ledger, "cohort ledger")
    if active_marker is not None and active_state_path is not None:
        release_active(ledger, active_state_path, active_marker["sessionId"])
    for paired_state_path in pair_state_paths:
        remove_owned_temporaries(paired_state_path, "withdrawn paired collector state")
        remove_path(paired_state_path, "withdrawn paired collector state")
    print(
        json.dumps(
            {
                "status": "withdrawn",
                "pairId": pair["pairId"],
                "responseRetainedByCollector": False,
            },
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
        )
    )


@serialize_state_transition
def command_fail(args: argparse.Namespace) -> None:
    """Settle a verified pre-exposure infrastructure failure without session data."""
    state_path, ledger, session_ledger, bank, manifest, state, pair = (
        load_bound_session(args)
    )
    recover_consent_commit(state_path, session_ledger, manifest, state, pair)
    settled = read_verified_ledger(ledger, manifest)
    pair_outcomes = [
        event
        for event in settled
        if event.get("pairId") == pair["pairId"]
        and event.get("type") in ("withdrawal", "infrastructure_failure")
    ]
    expected_outcome = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "infrastructure_failure",
        "pairId": pair["pairId"],
        "stage": "before_exposure",
        "reasonCode": args.reason_code,
    }
    if len(pair_outcomes) > 1 or (
        pair_outcomes
        and study.canonical_bytes(clean_receipt_event(pair_outcomes[0]))
        != study.canonical_bytes(expected_outcome)
    ):
        raise CollectorError("allocated pair has a different terminal outcome")
    outcome_sealed = bool(pair_outcomes)
    if state["consentPending"]:
        if not outcome_sealed:
            append_receipt_once(
                ledger, manifest, expected_outcome, ("type", "pairId")
            )
        if active_path(ledger).exists():
            release_active(ledger, state_path, state["sessionId"])
        elif not outcome_sealed:
            raise CollectorError("collector lease disappeared before failure cleanup")
        remove_owned_temporaries(ledger, "cohort ledger")
        remove_owned_temporaries(state_path, "failed collector state")
        remove_path(state_path, "failed collector state")
        print(
            json.dumps(
                {"status": "infrastructure_failure", "pairId": pair["pairId"]},
                ensure_ascii=False,
                sort_keys=True,
                indent=2,
            )
        )
        return
    if not outcome_sealed:
        reconcile_state(state, bank, manifest, session_ledger)
        if state["complete"]:
            raise CollectorError(
                "completed session cannot become an infrastructure failure"
            )
        require_active(ledger, state_path, state["sessionId"])
        events = read_verified_ledger(session_ledger, manifest)
        if any(event.get("type") != "session" for event in events):
            raise CollectorError(
                "infrastructure failure is unavailable after public exposure"
            )
        append_receipt_once(
            ledger, manifest, expected_outcome, ("type", "pairId")
        )
    if active_path(ledger).exists():
        require_active(ledger, state_path, state["sessionId"])
    elif not outcome_sealed:
        raise CollectorError("collector lease disappeared before failure cleanup")
    for session_id in pair["collectionOrder"]:
        pending = pending_session_ledger(ledger, session_id)
        remove_ledger_artifacts(pending, "failed provisional session ledger")
    remove_owned_temporaries(ledger, "cohort ledger")
    remove_owned_temporaries(state_path, "failed collector state")
    if active_path(ledger).exists():
        release_active(ledger, state_path, state["sessionId"])
    remove_path(state_path, "failed collector state")
    print(
        json.dumps(
            {"status": "infrastructure_failure", "pairId": pair["pairId"]},
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
        )
    )


@serialize_state_transition
def command_refusal(args: argparse.Namespace) -> None:
    """Record only an aggregate family count for a pre-consent refusal."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_agent_path(args.bank, "private probe bank")
    _bank, manifest = load_inputs(args)
    if args.model_family not in study.MODEL_FAMILIES:
        raise CollectorError("recruitment refusal model family is invalid")
    study.exact_int(args.ordinal, "recruitment refusal ordinal", 1, 100_000)
    if active_path(ledger).exists():
        raise CollectorError("recruitment refusal cannot interleave an active session")
    refusals = recruitment_refusals(ledger, manifest, args.model_family)
    expected = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "recruitment_refusal",
        "modelFamily": args.model_family,
        "familyRefusalOrdinal": args.ordinal,
    }
    if args.ordinal <= len(refusals):
        if study.canonical_bytes(
            clean_receipt_event(refusals[args.ordinal - 1])
        ) != study.canonical_bytes(expected):
            raise CollectorError("recruitment refusal transaction differs")
        status = "unchanged"
    elif args.ordinal == len(refusals) + 1:
        _receipt, written = append_receipt_once(
            ledger,
            manifest,
            expected,
            ("type", "modelFamily", "familyRefusalOrdinal"),
        )
        status = "recorded" if written else "unchanged"
    else:
        raise CollectorError("recruitment refusal ordinal has a gap")
    print(
        json.dumps(
            {
                "status": "recruitment_refusal",
                "writeStatus": status,
                "modelFamily": args.model_family,
                "familyRefusalOrdinal": args.ordinal,
            },
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
        )
    )


@serialize_state_transition
def command_deviation(args: argparse.Namespace) -> None:
    """Seal one bounded protocol deviation outside an active session."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    require_agent_path(args.bank, "private probe bank")
    _bank, manifest = load_inputs(args)
    if active_path(ledger).exists():
        raise CollectorError("deviation cannot interleave an active session")
    study.exact_int(args.ordinal, "deviation ordinal", 1, 100_000)
    existing = [
        event
        for event in read_verified_ledger(ledger, manifest)
        if event.get("type") == "deviation"
    ]
    for ordinal, prior in enumerate(existing, start=1):
        if prior.get("deviationOrdinal") != ordinal:
            raise CollectorError("deviation ordinals are not contiguous")
    if args.ordinal > len(existing) + 1:
        raise CollectorError("deviation ordinal has a gap")
    pairs, sessions = study.manifest_indexes(manifest)
    event: dict[str, Any] = {
        "schemaVersion": study.EVENT_SCHEMA,
        "type": "deviation",
        "deviationOrdinal": args.ordinal,
        "code": args.code,
        "description": args.description,
    }
    if args.pair_id is not None:
        if args.pair_id not in pairs:
            raise CollectorError("deviation pair id is outside the allocation")
        event["pairId"] = args.pair_id
    if args.session_id is not None:
        if args.session_id not in sessions:
            raise CollectorError("deviation session id is outside the allocation")
        pair, _session = sessions[args.session_id]
        if args.pair_id is not None and pair["pairId"] != args.pair_id:
            raise CollectorError("deviation pair and session ids disagree")
        event["sessionId"] = args.session_id
    _receipt, written = append_receipt_once(
        ledger, manifest, event, ("type", "deviationOrdinal")
    )
    print(
        json.dumps(
            {
                "status": "deviation_recorded",
                "writeStatus": "recorded" if written else "unchanged",
                "deviationOrdinal": args.ordinal,
            },
            sort_keys=True,
        )
    )


def calibration_packet(
    bank: dict[str, Any], delivery: dict[str, Any]
) -> dict[str, Any]:
    """Expose one oracle-free calibration item bound to its sealed request."""
    probe = next(
        probe for probe in bank["probes"] if probe["id"] == delivery["probeId"]
    )
    return {
        "status": "awaiting_calibration_response",
        "deliveryOrdinal": delivery["deliveryOrdinal"],
        "modelIdentifier": delivery["modelIdentifier"],
        "publicProbe": study.public_probe(probe, False),
        "responseRequest": {
            "kind": "calibration_response",
            "requestId": delivery["requestId"],
            "responseSchema": {"answer": "scalar", "refuse": "boolean"},
        },
    }


def validate_calibration_append(
    bank: dict[str, Any],
    prior: list[dict[str, Any]],
    event: dict[str, Any],
    runner_revision: str,
    runner_source_sha256: str,
) -> None:
    """Prove one proposed event extends the exact frozen calibration prefix."""
    study.calibration_progress(
        bank,
        [*prior, event],
        runner_revision,
        runner_source_sha256,
        require_complete=False,
    )


@serialize_state_transition
def command_calibration_next(args: argparse.Namespace) -> None:
    """Seal and expose exactly the next calibration item in a fresh context."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    bank_path = require_agent_path(args.bank, "private probe bank")
    bank = study.load_bank(bank_path)
    runner_revision = repository_commit()
    runner_source_sha256 = study_source_sha256(runner_revision)
    commitment = study.calibration_receipt_commitment(
        bank, runner_revision, runner_source_sha256
    )
    recover_receipt_transaction(ledger, commitment)
    events = read_verified_ledger(ledger, commitment)
    progress = study.calibration_progress(
        bank,
        events,
        runner_revision,
        runner_source_sha256,
        require_complete=False,
    )
    if progress["complete"]:
        raise CollectorError("calibration delivery schedule is complete")
    pending = progress["pendingDelivery"]
    if pending is not None:
        for field, supplied in (
            ("modelIdentifier", args.model_identifier),
            ("contextId", args.context_id),
            ("backendRevision", args.backend_revision),
        ):
            if pending[field] != supplied:
                raise CollectorError(
                    f"pending calibration delivery {field} differs from this request"
                )
        delivery = pending
    else:
        cell = progress["nextCell"]
        if cell is None:
            raise CollectorError("calibration progress has no next cell")
        if args.model_identifier != cell["modelIdentifier"]:
            raise CollectorError("model identifier differs from the next calibration cell")
        if not study.SHA256_HEX.fullmatch(args.context_id):
            raise CollectorError("calibration context id must be an opaque SHA-256 value")
        if not 1 <= len(args.backend_revision) <= 256:
            raise CollectorError("calibration backend revision is invalid")
        delivery = {
            "schemaVersion": study.CALIBRATION_EVENT_SCHEMA,
            "type": "calibration_delivery",
            **cell,
            "contextId": args.context_id,
            "backendRevision": args.backend_revision,
            "reasoningEffort": "high",
            "capabilityPolicy": study.CALIBRATION_CAPABILITY_POLICY,
            "freshContext": True,
            "attempt": 1,
            "runnerVersion": study.RUNNER_VERSION,
            "runnerRevision": runner_revision,
            "runnerSourceSha256": runner_source_sha256,
            "date": date.today().isoformat(),
        }
        start_commitment = study.attempt_start_commitment(
            phase="calibration",
            root_sha256=study.content_sha256(commitment),
            start_key=str(cell["deliveryOrdinal"]),
            model_identifier=args.model_identifier,
            context_id=args.context_id,
            backend_revision=args.backend_revision,
            runner_revision=runner_revision,
            runner_source_sha256=runner_source_sha256,
        )
        delivery["attemptStartReceiptSha256"] = require_attempt_start_receipt(
            getattr(args, "start_receipt", None), start_commitment
        )
        delivery["requestId"] = study.calibration_request_id(commitment, delivery)
        validate_calibration_append(
            bank, events, delivery, runner_revision, runner_source_sha256
        )
        append_receipt_once(
            ledger,
            commitment,
            delivery,
            ("type", "deliveryOrdinal"),
            event_validator=lambda _event: None,
        )
    print(json.dumps(calibration_packet(bank, delivery), indent=2, sort_keys=True))


@serialize_state_transition
def command_calibration_respond(args: argparse.Namespace) -> None:
    """Seal one bounded answer against the outstanding calibration delivery."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    bank_path = require_agent_path(args.bank, "private probe bank")
    bank = study.load_bank(bank_path)
    runner_revision = repository_commit()
    runner_source_sha256 = study_source_sha256(runner_revision)
    commitment = study.calibration_receipt_commitment(
        bank, runner_revision, runner_source_sha256
    )
    recover_receipt_transaction(ledger, commitment)
    events = read_verified_ledger(ledger, commitment)
    progress = study.calibration_progress(
        bank,
        events,
        runner_revision,
        runner_source_sha256,
        require_complete=False,
    )
    delivery = progress["pendingDelivery"]
    if delivery is None:
        raise CollectorError("calibration has no outstanding response request")
    participant = read_input(args.input)
    if not isinstance(participant, dict) or participant.get(
        "requestId"
    ) != delivery["requestId"]:
        raise CollectorError("calibration response requestId is stale or invalid")
    if set(participant) == {"requestId", "refuse"} and participant["refuse"] is True:
        payload = {"refuse": True}
    elif set(participant) == {"requestId", "answer"}:
        answer = participant["answer"]
        if isinstance(answer, (dict, list)) or answer is None:
            raise CollectorError("calibration answer must be a scalar")
        payload = {"answer": answer}
    else:
        raise CollectorError("calibration response shape is invalid")
    response = {
        "schemaVersion": study.CALIBRATION_EVENT_SCHEMA,
        "type": "calibration_response",
        "deliveryOrdinal": delivery["deliveryOrdinal"],
        "requestId": delivery["requestId"],
        **payload,
    }
    validate_calibration_append(
        bank, events, response, runner_revision, runner_source_sha256
    )
    append_receipt_once(
        ledger,
        commitment,
        response,
        ("type", "deliveryOrdinal"),
        event_validator=lambda _event: None,
    )
    completed = len(progress["records"]) + 1
    total = len(commitment["cells"])
    print(
        json.dumps(
            {
                "status": "calibration_response_sealed",
                "deliveryOrdinal": delivery["deliveryOrdinal"],
                "completedResponses": completed,
                "remainingResponses": total - completed,
            },
            indent=2,
            sort_keys=True,
        )
    )


@serialize_recovery
def command_calibration_recover(
    args: argparse.Namespace, reclaimed_transition: bool
) -> None:
    """Recover dead markers and a receipt transaction for calibration."""
    ledger = require_agent_path(args.ledger, "collector ledger")
    bank_path = require_agent_path(args.bank, "private probe bank")
    bank = study.load_bank(bank_path)
    runner_revision = repository_commit()
    runner_source_sha256 = study_source_sha256(runner_revision)
    commitment = study.calibration_receipt_commitment(
        bank, runner_revision, runner_source_sha256
    )
    recovered = recover_unleased_ledger(ledger, commitment, reclaimed_transition)
    print(json.dumps({"status": "recovered", "markers": recovered}, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    """Build the stateful collector CLI."""
    parser = argparse.ArgumentParser(
        description=(
            "Mediate one isolated Understanding Alpha session. Private inputs and raw "
            "receipts must remain under .agent."
        )
    )
    commands = parser.add_subparsers(dest="command", required=True)
    start = commands.add_parser("start", help="start one fresh consenting session")
    start.add_argument("--bank", type=Path, required=True)
    start.add_argument("--manifest", type=Path, required=True)
    start.add_argument("--state", type=Path, required=True)
    start.add_argument("--ledger", type=Path, required=True)
    start.add_argument("--session-id", required=True)
    start.add_argument("--context-id", required=True)
    start.add_argument("--model-identifier", required=True)
    start.add_argument("--backend-revision", required=True)
    start.add_argument("--prior-state", type=Path)
    start.add_argument(
        "--start-receipt",
        type=Path,
        help="independently recorded receipt for the commitment named on refusal",
    )
    start.set_defaults(handler=command_start)

    respond = commands.add_parser("respond", help="record exactly the pending response")
    respond.add_argument("--bank", type=Path, required=True)
    respond.add_argument("--manifest", type=Path, required=True)
    respond.add_argument("--state", type=Path, required=True)
    respond.add_argument("--ledger", type=Path, required=True)
    respond.add_argument(
        "--input", choices=("-",), required=True, help="read bounded JSON from stdin"
    )
    respond.set_defaults(handler=command_respond)

    status = commands.add_parser(
        "status", help="resume safely after an ambiguous collector process failure"
    )
    status.add_argument("--bank", type=Path, required=True)
    status.add_argument("--manifest", type=Path, required=True)
    status.add_argument("--state", type=Path, required=True)
    status.add_argument("--ledger", type=Path, required=True)
    status.set_defaults(handler=command_status)

    recover = commands.add_parser(
        "recover", help="recover exact stale process markers after a crash"
    )
    recover.add_argument("--state", type=Path, required=True)
    recover.add_argument("--ledger", type=Path, required=True)
    recover.set_defaults(handler=command_recover)

    recover_cohort = commands.add_parser(
        "recover-cohort", help="recover dead cohort markers without session state"
    )
    recover_cohort.add_argument("--ledger", type=Path, required=True)
    recover_cohort.add_argument("--manifest", type=Path, required=True)
    recover_cohort.set_defaults(handler=command_recover_cohort)

    interrupt = commands.add_parser(
        "interrupt", help="stop after exposure and erase response content"
    )
    interrupt.add_argument("--bank", type=Path, required=True)
    interrupt.add_argument("--manifest", type=Path, required=True)
    interrupt.add_argument("--state", type=Path, required=True)
    interrupt.add_argument("--ledger", type=Path, required=True)
    interrupt.add_argument(
        "--reason-code",
        choices=("context-lost", "runtime-failure"),
        required=True,
    )
    interrupt.set_defaults(handler=command_interrupt)

    withdraw = commands.add_parser(
        "withdraw", help="erase provisional pair data before aggregation"
    )
    withdraw.add_argument("--state", type=Path, required=True)
    withdraw.add_argument("--ledger", type=Path, required=True)
    withdraw.add_argument(
        "--input", choices=("-",), required=True, help="read bounded JSON from stdin"
    )
    withdraw.set_defaults(handler=command_withdraw)

    fail = commands.add_parser(
        "fail", help="settle a verified infrastructure failure before exposure"
    )
    fail.add_argument("--bank", type=Path, required=True)
    fail.add_argument("--manifest", type=Path, required=True)
    fail.add_argument("--state", type=Path, required=True)
    fail.add_argument("--ledger", type=Path, required=True)
    fail.add_argument(
        "--reason-code",
        choices=("runtime-unavailable", "tool-unavailable"),
        required=True,
    )
    fail.set_defaults(handler=command_fail)

    refusal = commands.add_parser(
        "refusal", help="record one content-free pre-consent refusal"
    )
    refusal.add_argument("--bank", type=Path, required=True)
    refusal.add_argument("--manifest", type=Path, required=True)
    refusal.add_argument("--ledger", type=Path, required=True)
    refusal.add_argument("--model-family", choices=study.MODEL_FAMILIES, required=True)
    refusal.add_argument("--ordinal", type=int, required=True)
    refusal.set_defaults(handler=command_refusal)

    deviation = commands.add_parser(
        "deviation", help="record one bounded protocol deviation"
    )
    deviation.add_argument("--bank", type=Path, required=True)
    deviation.add_argument("--manifest", type=Path, required=True)
    deviation.add_argument("--ledger", type=Path, required=True)
    deviation.add_argument("--ordinal", type=int, required=True)
    deviation.add_argument("--code", required=True)
    deviation.add_argument("--description", required=True)
    deviation.add_argument("--pair-id")
    deviation.add_argument("--session-id")
    deviation.set_defaults(handler=command_deviation)

    calibration_next = commands.add_parser(
        "calibration-next", help="seal and expose the next oracle-free calibration item"
    )
    calibration_next.add_argument("--bank", type=Path, required=True)
    calibration_next.add_argument("--ledger", type=Path, required=True)
    calibration_next.add_argument("--model-identifier", required=True)
    calibration_next.add_argument("--context-id", required=True)
    calibration_next.add_argument("--backend-revision", required=True)
    calibration_next.add_argument(
        "--start-receipt",
        type=Path,
        help="independently recorded receipt for the commitment named on refusal",
    )
    calibration_next.set_defaults(handler=command_calibration_next)

    calibration_respond = commands.add_parser(
        "calibration-respond", help="seal the outstanding calibration response"
    )
    calibration_respond.add_argument("--bank", type=Path, required=True)
    calibration_respond.add_argument("--ledger", type=Path, required=True)
    calibration_respond.add_argument(
        "--input", choices=("-",), required=True, help="read bounded JSON from stdin"
    )
    calibration_respond.set_defaults(handler=command_calibration_respond)

    calibration_recover = commands.add_parser(
        "calibration-recover", help="recover dead calibration receipt markers"
    )
    calibration_recover.add_argument("--bank", type=Path, required=True)
    calibration_recover.add_argument("--ledger", type=Path, required=True)
    calibration_recover.set_defaults(handler=command_calibration_recover)
    return parser


def main(argv: list[str] | None = None) -> int:
    """Run one bounded collector transition."""
    args = build_parser().parse_args(argv)
    try:
        args.handler(args)
    except (CollectorError, study.StudyError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
