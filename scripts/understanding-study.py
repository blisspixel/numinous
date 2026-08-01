#!/usr/bin/env python3
"""Frozen allocation, probe delivery, redaction, and analysis for 0.4."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import ipaddress
import json
import math
import os
import re
import subprocess
import sys
import tempfile
from collections import defaultdict
from datetime import date, datetime
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parent.parent


def load_source_integrity():
    """Load the shared exact-source verifier without modifying import paths."""
    path = ROOT / "scripts" / "understanding-source.py"
    spec = importlib.util.spec_from_file_location("numinous_understanding_source", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load understanding-source.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


source_integrity = load_source_integrity()
FIXTURE_PROBE_BANK_PATH = ROOT / "scripts" / "understanding-probes.fixture.json"
ENCOUNTER_SPEC_PATH = ROOT / "scripts" / "understanding-encounters.json"
RUNNER_VERSION = "numinous-understanding-runner-v5"
MCP_PROTOCOL_REVISION = "2026-07-28"
ALLOCATION_SEED = "numinous-understanding-alpha-v1"
BOOTSTRAP_SEED = "numinous-understanding-alpha-bootstrap-v1"
LATE_BOOTSTRAP_SEED = "numinous-understanding-alpha-late-bootstrap-v1"
ALLOCATION_SCHEMA = "numinous-understanding-allocation-v5"
EVENT_SCHEMA = "numinous-understanding-events-v5"
REPORT_SCHEMA = "numinous-understanding-report-v5"
RECEIPT_SCHEMA = "numinous-understanding-receipt-v1"
RECEIPT_ANCHOR_SCHEMA = "numinous-understanding-receipt-anchor-v1"
PROTOCOL_VERSION = "0.4-v5"
MCP_BUILD_RECEIPT_SCHEMA = "numinous-mcp-build-receipt-v1"
ERASED_PARTICIPANT_TOOL_CONTENT = {"participantContentErased": True}
BOOTSTRAP_RESAMPLES = 100_000
CALIBRATION_REPLICATES_PER_MODEL = 2
CALIBRATION_MODEL_CEILING_CORRECT_COUNT = 2
CALIBRATION_AMBIGUITY_COUNT = 2
CALIBRATION_RELEVANCE_REVIEWERS = 2
CALIBRATION_CAPABILITY_POLICY = "calibration-only-no-repository-web-or-tools"
CALIBRATION_EVENT_SCHEMA = "numinous-understanding-calibration-events-v1"
CALIBRATION_COMMITMENT_SCHEMA = "numinous-understanding-calibration-commitment-v1"
ATTEMPT_START_COMMITMENT_SCHEMA = "numinous-understanding-attempt-start-v1"
ATTEMPT_START_RECEIPT_SCHEMA = "numinous-understanding-attempt-start-receipt-v1"
ATTEMPT_START_ATTESTATION = (
    "This commitment was recorded before the identified stimulus was exposed."
)
MAX_INTERRUPTED_SESSIONS = 2
MAX_INTERRUPTED_SESSIONS_PER_MODEL = 1
MODEL_FAMILIES = ("gpt-5.6-sol", "gpt-5.6-terra")
MODEL_ALIASES = {"gpt-5.6-sol": "sol", "gpt-5.6-terra": "terra"}
MODEL_PROVIDERS = {model: "OpenAI" for model in MODEL_FAMILIES}
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
MAX_JSONL_RECORDS = 5_000
MAX_JSONL_TOTAL_BYTES = 256_000_000
MAX_JSON_DOCUMENT_BYTES = 5_000_000
MAX_JSON_NESTING_DEPTH = 32
ABSOLUTE_PATH = re.compile(
    r"(?i)(?:\b[a-z]:\\[^\"'\r\n|]+|\\\\[^\"'\r\n|]+|"
    r"(?<![:/])/(?:[^/\"'\r\n|]+/)+[^\"'\r\n|]+)"
)
EMAIL_VALUE = re.compile(
    r"(?i)\b[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9.-]+\.[a-z]{2,}\b"
)
IPV4_CANDIDATE = re.compile(r"(?<![\d.])(?:\d{1,3}\.){3}\d{1,3}(?![\d.])")
IPV6_CANDIDATE = re.compile(
    r"(?i)(?<![0-9a-f:])(?=[0-9a-f:]*:)[0-9a-f:]{2,}(?![0-9a-f:])"
)
BEARER_VALUE = re.compile(r"(?i)\bbearer\s+[a-z0-9._~+/-]{12,}\b")
BASIC_VALUE = re.compile(
    r"(?i)\b(?:authorization\s*:\s*)?basic\s+[a-z0-9+/]{12,}={0,2}(?=\s|$)"
)
RATIONAL_VALUE = re.compile(r"^([+-]?\d{1,18})/([+-]?\d{1,18})$")
COMMIT_SHA = re.compile(r"^[0-9a-f]{40}$")
SHA256_HEX = re.compile(r"^[0-9a-f]{64}$")
PRIVATE_KEYS = frozenset(
    {
        "accountid",
        "accountidentifier",
        "affect",
        "apikey",
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
        "secret",
        "accesskey",
        "accesskeyid",
        "accesstoken",
        "clientsecret",
        "token",
        "username",
        "userid",
    }
)
PRIVATE_ASSIGNMENT = re.compile(
    r"(?i)\b(?:account(?:[_ -]?(?:id|identifier))|affect|api[_ -]?key|"
    r"authorization|chain[_ -]?of[_ -]?thought|client[_ -]?(?:info|metadata|secret)|"
    r"cookie|credential|filesystem[_ -]?path|hidden[_ -]?reasoning|"
    r"host(?:[_ -]?identifier|name)|local[_ -]?path|password|"
    r"private[_ -]?prompt|prompt|reasoning|session[_ -]?id|system[_ -]?prompt|"
    r"secret|access[_ -]?(?:key(?:[_ -]?id)?|token)|token|user(?:[_ -]?id|name))"
    r"\s*[:=]\s*[^\r\n,;]+"
)
KNOWN_SECRET_VALUE = re.compile(
    r"(?i)\b(?:sk-(?:proj-)?[a-z0-9_-]{16,}|gh[pousr]_[a-z0-9]{16,}|"
    r"xox[baprs]-[a-z0-9-]{16,}|AKIA[A-Z0-9]{16})\b"
)
FORMULA_CONSTRUCTION = re.compile(r"^(?:sin|cos)\((?:[2-9]|[1-9][0-9])\*x\)$")


class StudyError(RuntimeError):
    """A deterministic study contract violation."""


def validate_json_nesting(value: Any, location: str) -> None:
    """Reject programmatic or decoded values beyond the bounded JSON depth."""
    pending = [(value, 0)]
    while pending:
        item, depth = pending.pop()
        if depth > MAX_JSON_NESTING_DEPTH:
            raise StudyError(f"{location} exceeds the JSON nesting limit")
        if isinstance(item, dict):
            pending.extend((nested, depth + 1) for nested in item.values())
        elif isinstance(item, list):
            pending.extend((nested, depth + 1) for nested in item)


def strict_json_loads(payload: str | bytes, location: str) -> Any:
    """Decode JSON while rejecting duplicate object keys at every depth."""

    def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"duplicate object key {key!r}")
            value[key] = item
        return value

    def reject_constant(value: str) -> None:
        raise ValueError(f"non-finite JSON constant {value!r}")

    try:
        value = json.loads(
            payload,
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except (RecursionError, TypeError, ValueError) as error:
        raise StudyError(f"invalid JSON in {location}: {error}") from error
    try:
        validate_json_nesting(value, f"invalid JSON in {location}")
    except StudyError as error:
        raise StudyError(f"{error}") from error
    return value


def read_bounded_json(path: Path, maximum_bytes: int = MAX_JSON_DOCUMENT_BYTES) -> Any:
    """Read one bounded strict JSON document."""
    try:
        with path.open("rb") as handle:
            payload = handle.read(maximum_bytes + 1)
    except OSError as error:
        raise StudyError(f"could not read {path}: {error}") from error
    if len(payload) > maximum_bytes:
        raise StudyError(f"{path} exceeds the JSON document limit")
    return strict_json_loads(payload, str(path))


def canonical_bytes(value: Any) -> bytes:
    """Return the stable JSON representation used for content hashes."""
    try:
        validate_json_nesting(value, "value")
        return json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (OverflowError, RecursionError, TypeError, ValueError) as error:
        raise StudyError("value is not canonical finite JSON") from error
    except StudyError as error:
        raise StudyError("value is not canonical finite JSON") from error


def content_sha256(value: Any) -> str:
    """Hash a JSON value after canonical serialization."""
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def attempt_start_commitment(
    *,
    phase: str,
    root_sha256: str,
    start_key: str,
    model_identifier: str,
    context_id: str,
    backend_revision: str,
    runner_revision: str,
    runner_source_sha256: str,
) -> str:
    """Commit one planned attempt without disclosing its concealed stimulus."""
    if phase not in ("calibration", "collection"):
        raise StudyError("attempt start phase is invalid")
    if not isinstance(root_sha256, str) or not SHA256_HEX.fullmatch(root_sha256):
        raise StudyError("attempt start root commitment is invalid")
    if not isinstance(start_key, str) or not 1 <= len(start_key) <= 128:
        raise StudyError("attempt start key is invalid")
    if model_identifier not in MODEL_FAMILIES:
        raise StudyError("attempt start model identifier is invalid")
    if not isinstance(context_id, str) or not SHA256_HEX.fullmatch(context_id):
        raise StudyError("attempt start context commitment is invalid")
    if not isinstance(backend_revision, str) or not 1 <= len(backend_revision) <= 256:
        raise StudyError("attempt start backend revision is invalid")
    if not isinstance(runner_revision, str) or not COMMIT_SHA.fullmatch(
        runner_revision
    ):
        raise StudyError("attempt start runner revision is invalid")
    if not isinstance(runner_source_sha256, str) or not SHA256_HEX.fullmatch(
        runner_source_sha256
    ):
        raise StudyError("attempt start runner source commitment is invalid")
    return content_sha256(
        {
            "schemaVersion": ATTEMPT_START_COMMITMENT_SCHEMA,
            "protocolVersion": PROTOCOL_VERSION,
            "phase": phase,
            "rootSha256": root_sha256,
            "startKey": start_key,
            "modelIdentifier": model_identifier,
            "contextId": context_id,
            "backendRevision": backend_revision,
            "runnerRevision": runner_revision,
            "runnerSourceSha256": runner_source_sha256,
        }
    )


def validate_attempt_start_receipt(
    receipt: Any, expected_sha256: str
) -> dict[str, Any]:
    """Validate one independently recorded pre-exposure start commitment."""
    fields = {
        "schemaVersion",
        "protocolVersion",
        "startCommitmentSha256",
        "mechanism",
        "witnessId",
        "witnessedAt",
        "recordLocator",
        "recordSha256",
        "attestation",
    }
    if (
        not isinstance(receipt, dict)
        or set(receipt) != fields
        or receipt.get("schemaVersion") != ATTEMPT_START_RECEIPT_SCHEMA
        or receipt.get("protocolVersion") != PROTOCOL_VERSION
        or receipt.get("startCommitmentSha256") != expected_sha256
        or receipt.get("mechanism")
        not in ("append-only-external-log", "independent-reconciler-ledger")
        or not isinstance(receipt.get("witnessId"), str)
        or not SHA256_HEX.fullmatch(receipt["witnessId"])
        or not isinstance(receipt.get("witnessedAt"), str)
        or re.fullmatch(
            r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", receipt["witnessedAt"]
        )
        is None
        or not isinstance(receipt.get("recordLocator"), str)
        or not 1 <= len(receipt["recordLocator"]) <= 2048
        or not isinstance(receipt.get("recordSha256"), str)
        or not SHA256_HEX.fullmatch(receipt["recordSha256"])
        or receipt.get("attestation") != ATTEMPT_START_ATTESTATION
    ):
        raise StudyError("attempt start receipt is invalid or differs")
    try:
        datetime.strptime(receipt["witnessedAt"], "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as error:
        raise StudyError("attempt start receipt timestamp is invalid") from error
    assert_sanitized(receipt)
    bounded_canonical_size(receipt, "attempt start receipt", 8192)
    return receipt


STUDY_SOURCE_OBJECTS = (
    ".cargo",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "assets",
    "data",
    "crates",
    "faces",
    "scripts/understanding-study.py",
    "scripts/understanding-collect.py",
    "scripts/understanding-encounters.json",
    "scripts/understanding-source.py",
    "scripts/mcp-play.py",
)


def repository_commit() -> str:
    """Return the clean committed repository revision used by the study boundary."""
    try:
        revision, _identities = source_integrity.verify_source_tree(
            ROOT,
            STUDY_SOURCE_OBJECTS,
            whole_worktree_clean=True,
        )
    except source_integrity.SourceIntegrityError as error:
        raise StudyError(f"could not identify the repository revision: {error}") from error
    return revision


def study_source_identity(revision: str) -> str:
    """Bind every committed runtime source tree used by calibration and collection."""
    if not COMMIT_SHA.fullmatch(revision):
        raise StudyError("study source revision is not a full commit SHA")
    try:
        _actual_revision, identities = source_integrity.verify_source_tree(
            ROOT,
            STUDY_SOURCE_OBJECTS,
            expected_revision=revision,
            whole_worktree_clean=True,
        )
    except source_integrity.SourceIntegrityError as error:
        raise StudyError(f"could not verify study source identity: {error}") from error
    return content_sha256(identities)


def current_study_source_identity() -> tuple[str, str]:
    """Return the clean commit and stable runtime-source tree commitment."""
    revision = repository_commit()
    return revision, study_source_identity(revision)


def receipt_genesis(manifest: dict[str, Any]) -> str:
    """Root the cohort receipt chain in the complete frozen allocation."""
    return content_sha256(
        {
            "schemaVersion": RECEIPT_SCHEMA,
            "manifestSha256": content_sha256(manifest),
            "probeBankSha256": manifest["probeBankSha256"],
            "encounterSpecSha256": manifest["encounterSpecSha256"],
        }
    )


def seal_records(
    manifest: dict[str, Any], records: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Wrap ordered collector events in one manifest-rooted hash chain."""
    manifest_hash = content_sha256(manifest)
    previous = receipt_genesis(manifest)
    receipts = []
    for index, source in enumerate(records):
        event = {key: value for key, value in source.items() if key != "_sourceIndex"}
        envelope = {
            "schemaVersion": RECEIPT_SCHEMA,
            "manifestSha256": manifest_hash,
            "receiptIndex": index,
            "previousReceiptSha256": previous,
            "eventSha256": content_sha256(event),
            "event": event,
        }
        receipt = {**envelope, "receiptSha256": content_sha256(envelope)}
        receipts.append(receipt)
        previous = receipt["receiptSha256"]
    return receipts


def verify_receipts(
    manifest: dict[str, Any], receipts: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Verify and unwrap the supplied manifest-rooted receipt chain."""
    manifest_hash = content_sha256(manifest)
    previous = receipt_genesis(manifest)
    events: list[dict[str, Any]] = []
    expected_fields = {
        "schemaVersion",
        "manifestSha256",
        "receiptIndex",
        "previousReceiptSha256",
        "eventSha256",
        "event",
        "receiptSha256",
    }
    for index, receipt in enumerate(receipts):
        if not isinstance(receipt, dict) or set(receipt) != expected_fields:
            raise StudyError("collector receipt has an invalid shape")
        if receipt["schemaVersion"] != RECEIPT_SCHEMA:
            raise StudyError("collector receipt schema differs")
        if receipt["manifestSha256"] != manifest_hash:
            raise StudyError("collector receipt manifest commitment differs")
        if (
            isinstance(receipt["receiptIndex"], bool)
            or not isinstance(receipt["receiptIndex"], int)
            or receipt["receiptIndex"] != index
        ):
            raise StudyError("collector receipt index is missing or reordered")
        if receipt["previousReceiptSha256"] != previous:
            raise StudyError("collector receipt chain is broken")
        event = receipt["event"]
        if not isinstance(event, dict):
            raise StudyError("collector receipt event must be an object")
        if receipt["eventSha256"] != content_sha256(event):
            raise StudyError("collector event payload hash differs")
        envelope = {
            key: value for key, value in receipt.items() if key != "receiptSha256"
        }
        if receipt["receiptSha256"] != content_sha256(envelope):
            raise StudyError("collector receipt hash differs")
        event = dict(event)
        event["_sourceIndex"] = index
        events.append(event)
        previous = receipt["receiptSha256"]
    if not events:
        raise StudyError("collector receipt chain is empty")
    return events


def build_receipt_anchor(
    manifest: dict[str, Any], receipts: list[dict[str, Any]]
) -> dict[str, Any]:
    """Commit to one exact nonempty receipt chain, including its terminal receipt."""
    verify_receipts(manifest, receipts)
    return {
        "schemaVersion": RECEIPT_ANCHOR_SCHEMA,
        "manifestSha256": content_sha256(manifest),
        "receiptCount": len(receipts),
        "lastReceiptSha256": receipts[-1]["receiptSha256"],
        "receiptChainSha256": content_sha256(receipts),
    }


def verify_receipt_anchor(
    manifest: dict[str, Any],
    receipts: list[dict[str, Any]],
    anchor: dict[str, Any],
) -> list[dict[str, Any]]:
    """Require the supplied chain to match its independently stored terminal anchor."""
    if not isinstance(anchor, dict) or set(anchor) != {
        "schemaVersion",
        "manifestSha256",
        "receiptCount",
        "lastReceiptSha256",
        "receiptChainSha256",
    }:
        raise StudyError("collector receipt anchor has an invalid shape")
    expected = build_receipt_anchor(manifest, receipts)
    if canonical_bytes(anchor) != canonical_bytes(expected):
        raise StudyError("collector receipt chain differs from its terminal anchor")
    return verify_receipts(manifest, receipts)


def calibration_cells(bank: dict[str, Any]) -> list[dict[str, Any]]:
    """Return the complete deterministic calibration delivery schedule."""
    cells: list[dict[str, Any]] = []
    ordinal = 0
    for probe in bank["probes"]:
        for model in MODEL_FAMILIES:
            for replicate in range(1, CALIBRATION_REPLICATES_PER_MODEL + 1):
                ordinal += 1
                cells.append(
                    {
                        "probeId": probe["id"],
                        "modelFamily": model,
                        "modelIdentifier": model,
                        "replicate": replicate,
                        "deliveryOrdinal": ordinal,
                    }
                )
    return cells


def calibration_receipt_commitment(
    bank: dict[str, Any], runner_revision: str, runner_source_sha256: str
) -> dict[str, Any]:
    """Build the immutable root used only by the calibration receipt chain."""
    if not COMMIT_SHA.fullmatch(runner_revision):
        raise StudyError("calibration runner revision is invalid")
    if not SHA256_HEX.fullmatch(runner_source_sha256):
        raise StudyError("calibration runner source commitment is invalid")
    cells = calibration_cells(bank)
    return {
        "schemaVersion": CALIBRATION_COMMITMENT_SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "runnerVersion": RUNNER_VERSION,
        "runnerRevision": runner_revision,
        "runnerSourceSha256": runner_source_sha256,
        "probeBankSha256": content_sha256(bank),
        "encounterSpecSha256": content_sha256(
            {
                "schemaVersion": CALIBRATION_COMMITMENT_SCHEMA,
                "cells": cells,
            }
        ),
        "cells": cells,
    }


def calibration_request_id(
    commitment: dict[str, Any], delivery: dict[str, Any]
) -> str:
    """Bind one response to an already sealed oracle-free delivery."""
    return content_sha256(
        {
            "schemaVersion": CALIBRATION_EVENT_SCHEMA,
            "commitmentSha256": content_sha256(commitment),
            "delivery": {
                key: value for key, value in delivery.items() if key != "requestId"
            },
        }
    )


def calibration_progress(
    bank: dict[str, Any],
    events: list[dict[str, Any]],
    runner_revision: str,
    runner_source_sha256: str,
    *,
    require_complete: bool,
) -> dict[str, Any]:
    """Verify an alternating delivery and response prefix without trusting callers."""
    commitment = calibration_receipt_commitment(
        bank, runner_revision, runner_source_sha256
    )
    cells = commitment["cells"]
    probe_ids = {probe["id"] for probe in bank["probes"]}
    contexts: set[str] = set()
    records: list[dict[str, Any]] = []
    cursor = 0
    pending: dict[str, Any] | None = None
    delivery_fields = {
        "schemaVersion",
        "type",
        "probeId",
        "modelFamily",
        "modelIdentifier",
        "replicate",
        "deliveryOrdinal",
        "contextId",
        "backendRevision",
        "reasoningEffort",
        "capabilityPolicy",
        "freshContext",
        "attempt",
        "runnerVersion",
        "runnerRevision",
        "runnerSourceSha256",
        "attemptStartReceiptSha256",
        "date",
        "requestId",
    }
    for cell in cells:
        if cursor >= len(events):
            break
        delivery = {
            key: value for key, value in events[cursor].items() if key != "_sourceIndex"
        }
        if set(delivery) != delivery_fields:
            raise StudyError("calibration delivery receipt shape differs")
        if (
            delivery["schemaVersion"] != CALIBRATION_EVENT_SCHEMA
            or delivery["type"] != "calibration_delivery"
            or any(delivery.get(key) != value for key, value in cell.items())
            or delivery["probeId"] not in probe_ids
            or not isinstance(delivery["contextId"], str)
            or not SHA256_HEX.fullmatch(delivery["contextId"])
            or delivery["contextId"] in contexts
            or not isinstance(delivery["backendRevision"], str)
            or not 1 <= len(delivery["backendRevision"]) <= 256
            or delivery["reasoningEffort"] != "high"
            or delivery["capabilityPolicy"] != CALIBRATION_CAPABILITY_POLICY
            or delivery["freshContext"] is not True
            or delivery["attempt"] != 1
            or delivery["runnerVersion"] != RUNNER_VERSION
            or delivery["runnerRevision"] != runner_revision
            or delivery["runnerSourceSha256"] != runner_source_sha256
            or not isinstance(delivery["attemptStartReceiptSha256"], str)
            or not SHA256_HEX.fullmatch(delivery["attemptStartReceiptSha256"])
            or delivery["requestId"]
            != calibration_request_id(commitment, delivery)
        ):
            raise StudyError("calibration delivery receipt differs from the frozen cell")
        try:
            date.fromisoformat(delivery["date"])
        except (TypeError, ValueError) as error:
            raise StudyError("calibration delivery date is invalid") from error
        assert_sanitized(delivery)
        bounded_canonical_size(delivery, "calibration delivery", 4096)
        contexts.add(delivery["contextId"])
        cursor += 1
        if cursor >= len(events):
            pending = delivery
            break
        response = {
            key: value for key, value in events[cursor].items() if key != "_sourceIndex"
        }
        common_response_fields = {
            "schemaVersion",
            "type",
            "deliveryOrdinal",
            "requestId",
        }
        if set(response) not in (
            common_response_fields | {"answer"},
            common_response_fields | {"refuse"},
        ) or ("refuse" in response and response["refuse"] is not True):
            raise StudyError("calibration response receipt shape differs")
        if (
            response["schemaVersion"] != CALIBRATION_EVENT_SCHEMA
            or response["type"] != "calibration_response"
            or response["deliveryOrdinal"] != delivery["deliveryOrdinal"]
            or response["requestId"] != delivery["requestId"]
        ):
            raise StudyError("calibration response is not bound to its delivery")
        assert_sanitized(response)
        bounded_canonical_size(response, "calibration response", 4096)
        records.append(
            {
                key: delivery[key]
                for key in (
                    "probeId",
                    "modelFamily",
                    "modelIdentifier",
                    "replicate",
                    "deliveryOrdinal",
                    "contextId",
                    "backendRevision",
                    "reasoningEffort",
                    "capabilityPolicy",
                    "freshContext",
                    "attempt",
                    "runnerVersion",
                    "runnerRevision",
                    "runnerSourceSha256",
                    "attemptStartReceiptSha256",
                    "date",
                )
            }
            | ({"refuse": True} if "refuse" in response else {"answer": response["answer"]})
        )
        cursor += 1
    if cursor != len(events):
        raise StudyError("calibration ledger has extra or out-of-order events")
    complete = len(records) == len(cells) and pending is None
    if require_complete and not complete:
        raise StudyError(
            f"calibration stopped early with {len(cells) - len(records)} response(s) missing"
        )
    return {
        "records": records,
        "pendingDelivery": pending,
        "nextCell": None if complete or pending is not None else cells[len(records)],
        "complete": complete,
    }


def calibration_response_records(
    bank: dict[str, Any],
    receipts: list[dict[str, Any]],
    anchor: dict[str, Any],
    runner_revision: str,
    runner_source_sha256: str,
) -> tuple[list[dict[str, Any]], str]:
    """Extract responses only from a complete anchored delivery ledger."""
    commitment = calibration_receipt_commitment(
        bank, runner_revision, runner_source_sha256
    )
    events = verify_receipt_anchor(commitment, receipts, anchor)
    progress = calibration_progress(
        bank,
        events,
        runner_revision,
        runner_source_sha256,
        require_complete=True,
    )
    return progress["records"], content_sha256(receipts)


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
    return read_bounded_json(path)


def write_text_once(path: Path, text: str) -> str:
    """Write atomically, refusing to replace evidence with different content."""
    if path.is_symlink():
        raise StudyError(f"refusing to inspect symlink evidence target {path}")
    if path.exists():
        try:
            existing = path.read_text(encoding="utf-8")
        except OSError as error:
            raise StudyError(f"could not inspect existing {path}: {error}") from error
        if existing == text:
            return "unchanged"
        raise StudyError(f"refusing to replace existing evidence file {path}")
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
        raise StudyError(f"could not stage evidence file {path}: {error}") from error
    temporary = Path(handle.name)
    result = "written"
    try:
        with handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            if path.is_symlink():
                raise StudyError(f"refusing to inspect symlink evidence target {path}")
            try:
                existing = path.read_text(encoding="utf-8")
            except OSError as error:
                raise StudyError(
                    f"could not inspect concurrent {path}: {error}"
                ) from error
            if existing == text:
                result = "unchanged"
            else:
                raise StudyError(f"refusing to replace existing evidence file {path}")
        except OSError as error:
            raise StudyError(
                f"could not publish evidence file {path}: {error}"
            ) from error
    except OSError as error:
        raise StudyError(
            f"could not stage evidence content for {path}: {error}"
        ) from error
    finally:
        try:
            temporary.unlink(missing_ok=True)
        except OSError as cleanup_error:
            if sys.exception() is None:
                raise StudyError(
                    f"could not clean staged evidence for {path}: {cleanup_error}"
                ) from cleanup_error
    return result


def write_json_once(path: Path, value: Any) -> str:
    """Write indented JSON through the evidence-preserving writer."""
    text = json.dumps(
        value, ensure_ascii=False, sort_keys=True, indent=2, allow_nan=False
    )
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
        if isinstance(answer, str):
            match = RATIONAL_VALUE.fullmatch(answer.strip())
            if match is None:
                return False, None
            denominator = int(match.group(2))
            if denominator == 0:
                return False, None
            number = float(Fraction(int(match.group(1)), denominator))
        elif isinstance(answer, bool) or not isinstance(answer, (int, float)):
            return False, None
        else:
            try:
                number = float(answer)
            except (OverflowError, ValueError):
                return False, None
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
    if set(bank) != {
        "schemaVersion",
        "protocolVersion",
        "distractorSequence",
        "probes",
    }:
        raise StudyError("probe bank contains unknown or missing fields")
    if bank.get("schemaVersion") != "numinous-understanding-probes-v1":
        raise StudyError("unsupported probe bank schema")
    if bank.get("protocolVersion") != PROTOCOL_VERSION:
        raise StudyError("probe bank protocol version does not match the runner")
    distractor = bank.get("distractorSequence")
    if (
        not isinstance(distractor, dict)
        or set(distractor) != {"id", "items"}
        or not isinstance(distractor.get("id"), str)
    ):
        raise StudyError("probe bank requires a named distractor sequence")
    items = distractor.get("items")
    if not isinstance(items, list) or len(items) != 5:
        raise StudyError("distractor sequence must contain exactly five items")
    distractor_ids: set[str] = set()
    for item in items:
        if not isinstance(item, dict):
            raise StudyError("each distractor must be an object")
        if set(item) != {"id", "prompt"}:
            raise StudyError("distractor contains unknown or missing fields")
        item_id = item.get("id")
        prompt = item.get("prompt")
        if not isinstance(item_id, str) or not item_id or item_id in distractor_ids:
            raise StudyError("distractor ids must be unique nonempty strings")
        if not isinstance(prompt, str) or not prompt.strip() or len(prompt) > 512:
            raise StudyError("distractor prompts must be nonempty strings")
        distractor_ids.add(item_id)
    probes = bank.get("probes")
    if not isinstance(probes, list):
        raise StudyError("probe bank probes must be an array")
    expected_counts = {
        (phase, room): 2 for phase in ("immediate", "late") for room in ROOMS
    }
    counts: dict[tuple[str, str], int] = defaultdict(int)
    probe_ids: set[str] = set()
    oracle_fields = {
        "mod_product": {"kind", "start", "multiplier", "modulus"},
        "identical_state_gap": {"kind", "steps"},
        "relative_growth": {
            "kind",
            "aInitial",
            "aFinal",
            "bInitial",
            "bFinal",
        },
        "life_center": {"kind", "currentlyAlive", "liveNeighbors"},
        "binomial_paths": {"kind", "rows", "rights"},
        "binomial_mean": {"kind", "rows", "rightProbability"},
        "trig_parity": {"kind", "function", "frequency", "verticalOffset"},
        "trig_period_over_pi": {"kind", "frequency"},
    }
    for probe in probes:
        if not isinstance(probe, dict):
            raise StudyError("each probe must be an object")
        if set(probe) != {
            "id",
            "phase",
            "room",
            "prompt",
            "answerSchema",
            "oracle",
        }:
            raise StudyError("probe contains unknown or missing fields")
        probe_id = probe.get("id")
        phase = probe.get("phase")
        room = probe.get("room")
        prompt = probe.get("prompt")
        if not isinstance(probe_id, str) or not probe_id or probe_id in probe_ids:
            raise StudyError("probe ids must be unique nonempty strings")
        if (phase, room) not in expected_counts:
            raise StudyError(f"probe {probe_id} has an invalid phase or room")
        if not isinstance(prompt, str) or not prompt.strip() or len(prompt) > 1024:
            raise StudyError(f"probe {probe_id} has no prompt")
        schema = probe.get("answerSchema")
        oracle = probe.get("oracle")
        if not isinstance(schema, dict) or not isinstance(oracle, dict):
            raise StudyError(
                f"probe {probe_id} requires answerSchema and oracle objects"
            )
        answer_type = schema.get("type")
        expected_schema_fields = (
            {"type", "tolerance"}
            if answer_type == "number"
            else {"type", "enum"}
            if answer_type == "string"
            else set()
        )
        if not expected_schema_fields or set(schema) != expected_schema_fields:
            raise StudyError(f"probe {probe_id} answer schema fields are invalid")
        kind = oracle.get("kind")
        if kind not in oracle_fields or set(oracle) != oracle_fields[kind]:
            raise StudyError(f"probe {probe_id} oracle fields are invalid")
        if answer_type == "string":
            choices = schema["enum"]
            if (
                not isinstance(choices, list)
                or not 1 <= len(choices) <= 10
                or any(
                    not isinstance(choice, str) or not 1 <= len(choice) <= 64
                    for choice in choices
                )
                or len({choice.casefold() for choice in choices}) != len(choices)
            ):
                raise StudyError(f"probe {probe_id} answer enum is invalid")
        expected = oracle_answer(oracle)
        valid, _normalized = normalize_answer(schema, expected)
        if not valid:
            raise StudyError(
                f"probe {probe_id} oracle output violates its answer schema"
            )
        if schema.get("type") == "number" and require_number(schema, "tolerance") < 0.0:
            raise StudyError(f"probe {probe_id} tolerance must be nonnegative")
        counts[(phase, room)] += 1
        probe_ids.add(probe_id)
    if counts != expected_counts:
        raise StudyError(f"probe inventory mismatch: {dict(counts)}")
    return bank


def load_bank(path: Path) -> dict[str, Any]:
    """Load and validate the tracked probe bank."""
    return validate_bank(load_json(path))


def validate_encounter_spec(spec: Any) -> dict[str, Any]:
    """Validate the exact public intervention and MCP call specification."""
    if not isinstance(spec, dict):
        raise StudyError("encounter specification must be an object")
    if set(spec) != {"schemaVersion", "protocolVersion", "rooms"}:
        raise StudyError("encounter specification contains unknown or missing fields")
    if spec.get("schemaVersion") != "numinous-understanding-encounters-v5":
        raise StudyError("unsupported encounter specification schema")
    if spec.get("protocolVersion") != PROTOCOL_VERSION:
        raise StudyError("encounter specification protocol does not match the runner")
    rooms = spec.get("rooms")
    if not isinstance(rooms, list) or [room.get("id") for room in rooms] != list(ROOMS):
        raise StudyError(
            "encounter specification must contain the five flagships in order"
        )
    for room in rooms:
        room_id = room["id"]
        expected_room_fields = {
            "id",
            "calls",
            "controlPrompt",
            "generationPrompt",
            "generationAnswerSchema",
            "expectedAnswer",
            "feedbackText",
            "feedbackEvidence",
        }
        if room_id == "formula-jam":
            expected_room_fields.add("revealMaterial")
        if set(room) != expected_room_fields:
            raise StudyError(f"encounter {room_id} contains unknown or missing fields")
        calls = room.get("calls")
        if not isinstance(calls, list) or len(calls) != TOOL_CALLS_PER_ROOM:
            raise StudyError(f"encounter {room_id} must contain four MCP calls")
        for call in calls:
            if not isinstance(call, dict) or set(call) != {"tool", "arguments", "role"}:
                raise StudyError(f"encounter {room_id} call has an invalid shape")
            tool = call["tool"]
            arguments = call["arguments"]
            role = call["role"]
            if not isinstance(role, str) or not role:
                raise StudyError(f"encounter {room_id} call role must be nonempty")
            if tool == "play_room":
                allowed_arguments = {"id", "t", "response_mode", "pokes"}
                if not {"id", "t", "response_mode"}.issubset(arguments) or set(
                    arguments
                ) - allowed_arguments:
                    raise StudyError(f"encounter {room_id} play_room arguments differ")
                if (
                    arguments["id"] != room_id
                    or arguments["response_mode"] != "compact"
                ):
                    raise StudyError(f"encounter {room_id} play_room identity differs")
                phase = require_number(arguments, "t")
                if not 0.0 <= phase < 1.0:
                    raise StudyError(f"encounter {room_id} play_room phase is invalid")
                if "pokes" in arguments and arguments["pokes"] != [[0.5, 0.5]]:
                    if not (
                        room_id == "double-pendulum"
                        and arguments["pokes"] == [[0.2, 0.8]]
                    ):
                        raise StudyError(f"encounter {room_id} interaction differs")
            elif tool == "reveal_room":
                if arguments != {"id": room_id}:
                    raise StudyError(f"encounter {room_id} reveal arguments differ")
            elif tool == "plot_expression":
                if set(arguments) != {"expr"} or not isinstance(arguments["expr"], str):
                    raise StudyError("Formula Jam plot expression is invalid")
                if not 1 <= len(arguments["expr"]) <= 128:
                    raise StudyError("Formula Jam expression exceeds its bound")
            else:
                raise StudyError(f"encounter {room_id} uses unsupported tool {tool!r}")
        expected_calls = (
            (
                ("plot_expression", "encounter"),
                ("plot_expression", "interaction"),
                ("plot_expression", "continuation"),
                ("plot_expression", "final_observation"),
            )
            if room_id == "formula-jam"
            else (
                ("play_room", "encounter"),
                ("play_room", "interaction"),
                ("play_room", "continuation"),
                ("reveal_room", "reveal"),
            )
        )
        observed_calls = tuple((call["tool"], call["role"]) for call in calls)
        if observed_calls != expected_calls:
            raise StudyError(f"encounter {room_id} tool roles or order differ")
        for prompt_key in (
            "controlPrompt",
            "generationPrompt",
            "feedbackText",
        ):
            prompt = room.get(prompt_key)
            if not isinstance(prompt, str) or not 1 <= len(prompt) <= 512:
                raise StudyError(f"encounter {room_id} {prompt_key} is invalid")
        schema = room["generationAnswerSchema"]
        expected_answer = room["expectedAnswer"]
        evidence = room["feedbackEvidence"]
        if room_id == "formula-jam":
            if (
                schema
                != {
                    "type": "string",
                    "pattern": r"^(sin|cos)\(([2-9]|[1-9][0-9])\*x\)$",
                }
                or expected_answer != "valid-expression"
                or evidence
                != {"field": "expression", "equalsParticipantAnswer": True}
            ):
                raise StudyError("Formula Jam generation answer contract differs")
        elif (
            not isinstance(schema, dict)
            or set(schema) != {"type", "enum"}
            or schema["type"] != "string"
            or not isinstance(schema["enum"], list)
            or len(schema["enum"]) != 2
            or any(not isinstance(choice, str) or not choice for choice in schema["enum"])
            or expected_answer not in schema["enum"]
            or not isinstance(evidence, dict)
            or set(evidence) != {"field", "contains"}
            or evidence["field"] != "status"
            or not isinstance(evidence["contains"], str)
            or not evidence["contains"]
        ):
            raise StudyError(f"encounter {room_id} generation answer contract differs")
        reveal_material = room.get("revealMaterial")
        if room_id == "formula-jam":
            if (
                not isinstance(reveal_material, str)
                or not 1 <= len(reveal_material) <= 2048
            ):
                raise StudyError("Formula Jam reveal material is invalid")
        elif reveal_material is not None:
            raise StudyError(f"encounter {room_id} has unexpected reveal material")
    return spec


def load_encounter_spec(path: Path = ENCOUNTER_SPEC_PATH) -> dict[str, Any]:
    """Load the tracked executable encounter specification."""
    return validate_encounter_spec(load_json(path))


def build_allocation(
    bank: dict[str, Any],
    calibration_audit: dict[str, Any],
    encounter_spec: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build the complete 24-pair allocation from the literal protocol seed."""
    validate_calibration_audit(bank, calibration_audit)
    encounter_spec = encounter_spec or load_encounter_spec()
    pairs: list[dict[str, Any]] = []
    for model in MODEL_FAMILIES:
        calibrated_backend_revision = calibration_audit["provenance"][
            "backendRevisions"
        ][model][0]
        model_digest = keyed_digest(ALLOCATION_SEED, f"{model}:balanced-schedule")
        room_offset = model_digest[0] % len(ROOMS)
        condition_offset = model_digest[1] % len(CONDITIONS)
        rotations = [(room_offset + offset) % len(ROOMS) for offset in range(12)]
        first_conditions = [
            CONDITIONS[(condition_offset + offset) % len(CONDITIONS)]
            for offset in range(12)
        ]
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
                    "calibratedBackendRevision": calibrated_backend_revision,
                    "studySourceSha256": calibration_audit["provenance"][
                        "runnerSourceSha256"
                    ],
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
        "calibrationRunnerRevision": calibration_audit["provenance"][
            "runnerRevision"
        ],
        "calibrationRunnerSourceSha256": calibration_audit["provenance"][
            "runnerSourceSha256"
        ],
        "allocationSeed": ALLOCATION_SEED,
        "probeBankSha256": content_sha256(bank),
        "calibrationAudit": calibration_audit,
        "encounterSpecSha256": content_sha256(encounter_spec),
        "distractorSequenceId": bank["distractorSequence"]["id"],
        "toolCallsPerRoom": TOOL_CALLS_PER_ROOM,
        "maximumReserveConditionOrderImbalance": 2,
        "maximumReserveFirstRoomCountRange": 3,
        "models": [
            {
                "modelFamily": model,
                "modelIdentifier": model,
                "provider": MODEL_PROVIDERS[model],
                "calibratedBackendRevision": calibration_audit["provenance"][
                    "backendRevisions"
                ][model][0],
                "reasoningEffort": "high",
                "qualifyingPairs": 10,
                "reserves": 2,
            }
            for model in MODEL_FAMILIES
        ],
        "pairs": pairs,
    }


def validate_manifest(manifest: Any, bank: dict[str, Any]) -> dict[str, Any]:
    """Require the byte-equivalent allocation generated by this revision."""
    if not isinstance(manifest, dict) or not isinstance(
        manifest.get("calibrationAudit"), dict
    ):
        raise StudyError("allocation manifest has no passed calibration audit")
    expected = build_allocation(bank, manifest["calibrationAudit"])
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


def session_packet(manifest: dict[str, Any], session_id: str) -> dict[str, Any]:
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
            "receive corrective feedback and Reveal, then continue without another "
            "generated answer."
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
    return normalized in PRIVATE_KEYS


def valid_network_address(value: str) -> bool:
    """Return whether a candidate is a syntactically valid IP address."""
    try:
        ipaddress.ip_address(value)
    except ValueError:
        return False
    return True


def redact_network_addresses(value: str) -> tuple[str, int]:
    """Replace valid IPv4 and IPv6 literals without redacting lookalikes."""
    clean = value
    count = 0
    for pattern in (IPV4_CANDIDATE, IPV6_CANDIDATE):

        def replace(match: re.Match[str]) -> str:
            nonlocal count
            if not valid_network_address(match.group(0)):
                return match.group(0)
            count += 1
            return "<IP_ADDRESS>"

        clean = pattern.sub(replace, clean)
    return clean, count


def contains_network_address(value: str, *, allow_ascii_art: bool = False) -> bool:
    """Return whether text contains a valid IPv4 or IPv6 literal."""
    return any(
        valid_network_address(match.group(0))
        and not (allow_ascii_art and match.group(0) == "::")
        for pattern in (IPV4_CANDIDATE, IPV6_CANDIDATE)
        for match in pattern.finditer(value)
    )


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
        clean, count = ABSOLUTE_PATH.subn("<ABSOLUTE_PATH>", value)
        for replacement in replacements:
            if replacement and re.search(
                re.escape(replacement), clean, flags=re.IGNORECASE
            ):
                clean = re.sub(
                    re.escape(replacement),
                    "<HOST_IDENTIFIER>",
                    clean,
                    flags=re.IGNORECASE,
                )
                count += 1
        clean, network_count = redact_network_addresses(clean)
        count += network_count
        for pattern, marker in (
            (EMAIL_VALUE, "<EMAIL>"),
            (BEARER_VALUE, "<CREDENTIAL>"),
            (BASIC_VALUE, "<CREDENTIAL>"),
            (PRIVATE_ASSIGNMENT, "<PRIVATE_ASSIGNMENT>"),
            (KNOWN_SECRET_VALUE, "<CREDENTIAL>"),
        ):
            replaced = pattern.sub(marker, clean)
            count += int(replaced != clean)
            clean = replaced
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
    elif isinstance(value, str):
        if ABSOLUTE_PATH.search(value):
            raise StudyError(f"{location} contains an absolute host path")
        if EMAIL_VALUE.search(value):
            raise StudyError(f"{location} contains an email address")
        mcp_ascii_art = (
            location.startswith("mcp.")
            or ".structuredResult.render" in location
            or ".structuredResult.plot" in location
            or location.endswith(".visibleText")
        )
        if contains_network_address(value, allow_ascii_art=mcp_ascii_art):
            raise StudyError(f"{location} contains an IP address")
        if BEARER_VALUE.search(value):
            raise StudyError(f"{location} contains a credential pattern")
        if (
            BASIC_VALUE.search(value)
            or PRIVATE_ASSIGNMENT.search(value)
            or KNOWN_SECRET_VALUE.search(value)
        ):
            raise StudyError(f"{location} contains a private assignment")


def bounded_jsonl_lines(path: Path) -> Iterable[tuple[int, bytes]]:
    """Yield lines without ever allocating beyond the per-line or total limit."""
    total_bytes = 0
    try:
        with path.open("rb") as handle:
            line_number = 0
            while True:
                raw = handle.readline(MAX_JSONL_LINE_BYTES + 1)
                if not raw:
                    break
                line_number += 1
                total_bytes += len(raw)
                if total_bytes > MAX_JSONL_TOTAL_BYTES:
                    raise StudyError(f"{path} exceeds the JSONL total-byte limit")
                if len(raw) > MAX_JSONL_LINE_BYTES:
                    raise StudyError(
                        f"{path}:{line_number} exceeds the JSONL line limit"
                    )
                yield line_number, raw
    except OSError as error:
        raise StudyError(f"could not read {path}: {error}") from error


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    """Read bounded JSONL records with stable source indexes."""
    records: list[dict[str, Any]] = []
    for line_number, raw in bounded_jsonl_lines(path):
        if not raw.strip():
            continue
        if len(records) >= MAX_JSONL_RECORDS:
            raise StudyError(f"{path} exceeds the JSONL record limit")
        record = strict_json_loads(raw, f"{path}:{line_number}")
        if not isinstance(record, dict):
            raise StudyError(f"{path}:{line_number} must contain a JSON object")
        record["_sourceIndex"] = len(records)
        records.append(record)
    return records


def read_receipt_jsonl(path: Path) -> list[dict[str, Any]]:
    """Read bounded receipt JSONL without mutating hash-covered envelopes."""
    receipts: list[dict[str, Any]] = []
    for line_number, raw in bounded_jsonl_lines(path):
        if not raw.strip():
            continue
        if len(receipts) >= MAX_JSONL_RECORDS:
            raise StudyError(f"{path} exceeds the JSONL record limit")
        receipt = strict_json_loads(raw, f"{path}:{line_number}")
        if not isinstance(receipt, dict):
            raise StudyError(f"{path}:{line_number} must contain a JSON object")
        receipts.append(receipt)
    return receipts


def redact_jsonl(
    input_path: Path, output_path: Path, replacements: tuple[str, ...]
) -> str:
    """Produce a bounded ledger with prohibited fields removed."""
    output_lines: list[str] = []
    for record in read_jsonl(input_path):
        record.pop("_sourceIndex", None)
        clean, removed = redact_value(record, replacements)
        if not isinstance(clean, dict):
            raise StudyError("redaction produced a non-object record")
        if removed:
            clean["redactedFieldCount"] = removed
        assert_sanitized(clean)
        output_lines.append(
            json.dumps(
                clean,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            )
        )
    return write_text_once(
        output_path, "\n".join(output_lines) + ("\n" if output_lines else "")
    )


def required_string(value: dict[str, Any], key: str) -> str:
    """Read a nonempty string field."""
    item = value.get(key)
    if not isinstance(item, str) or not item.strip():
        raise StudyError(f"{key} must be a nonempty string")
    return item


EVENT_FIELDS: dict[str, tuple[frozenset[str], frozenset[str]]] = {
    "session": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "consent",
                "publicationConsent",
                "modelFamily",
                "modelIdentifier",
                "provider",
                "backendRevision",
                "reasoningEffort",
                "settings",
                "date",
                "numinousCommit",
                "mcpProtocolRevision",
                "operatingSystem",
                "runnerVersion",
                "studySourceSha256",
                "attemptStartReceiptSha256",
                "condition",
                "contextId",
                "capabilityPolicy",
            }
        ),
        frozenset(),
    ),
    "tool": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "room",
                "sequence",
                "role",
                "tool",
                "arguments",
                "structuredResult",
                "visibleText",
                "toolOutcome",
                "binarySha256",
                "binaryBuildReceipt",
            }
        ),
        frozenset(),
    ),
    "response": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "phase",
                "probeId",
                "attempt",
                "answer",
            }
        ),
        frozenset(),
    ),
    "response_refusal": (
        frozenset({"schemaVersion", "type", "sessionId", "phase", "probeId"}),
        frozenset(),
    ),
    "distractor_response": (
        frozenset({"schemaVersion", "type", "sessionId", "itemId", "answer"}),
        frozenset(),
    ),
    "condition_response": (
        frozenset({"schemaVersion", "type", "sessionId", "room", "stage"}),
        frozenset({"text", "answer", "rationale"}),
    ),
    "feedback": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "room",
                "expectedAnswer",
                "participantCorrect",
                "text",
            }
        ),
        frozenset(),
    ),
    "material": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "room",
                "kind",
                "text",
                "materialSha256",
            }
        ),
        frozenset(),
    ),
    "session_complete": (
        frozenset({"schemaVersion", "type", "sessionId"}),
        frozenset(),
    ),
    "session_interruption": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "sessionId",
                "stage",
                "reasonCode",
                "terminalRequestSha256",
            }
        ),
        frozenset(),
    ),
    "recruitment_refusal": (
        frozenset(
            {"schemaVersion", "type", "modelFamily", "familyRefusalOrdinal"}
        ),
        frozenset(),
    ),
    "withdrawal": (
        frozenset(
            {
                "schemaVersion",
                "type",
                "pairId",
                "contextTombstones",
                "terminalRequestSha256",
            }
        ),
        frozenset(),
    ),
    "infrastructure_failure": (
        frozenset({"schemaVersion", "type", "pairId", "stage", "reasonCode"}),
        frozenset(),
    ),
    "deviation": (
        frozenset(
            {"schemaVersion", "type", "deviationOrdinal", "code", "description"}
        ),
        frozenset({"pairId", "sessionId"}),
    ),
}
EVENT_PROCESSING_FIELDS = frozenset({"_sourceIndex", "redactedFieldCount"})


def exact_int(value: Any, field: str, minimum: int, maximum: int) -> int:
    """Return a bounded integer while rejecting booleans."""
    if isinstance(value, bool) or not isinstance(value, int):
        raise StudyError(f"{field} must be an integer")
    if not minimum <= value <= maximum:
        raise StudyError(f"{field} must be in {minimum}..{maximum}")
    return value


def bounded_canonical_size(value: Any, field: str, limit: int) -> None:
    """Require a JSON value whose canonical representation is bounded."""
    try:
        size = len(canonical_bytes(value))
    except (OverflowError, TypeError, ValueError) as error:
        raise StudyError(f"{field} is not canonical finite JSON") from error
    if size > limit:
        raise StudyError(f"{field} exceeds the public evidence limit")


def validate_mcp_build_receipt(receipt: Any, binary_sha256: str) -> None:
    """Require one exact source-bound receipt for the executed private binary."""
    fields = {
        "schemaVersion",
        "sourceRevision",
        "studySourceSha256",
        "sourcePolicy",
        "environmentPolicy",
        "cargoVersion",
        "rustcVersion",
        "targetTriple",
        "profile",
        "features",
        "locked",
        "incremental",
        "targetDirectoryPolicy",
        "artifactPolicy",
        "binarySha256",
    }
    if not isinstance(receipt, dict) or set(receipt) != fields:
        raise StudyError("MCP build receipt schema differs")
    if receipt["schemaVersion"] != MCP_BUILD_RECEIPT_SCHEMA:
        raise StudyError("MCP build receipt version differs")
    if (
        not isinstance(receipt["sourceRevision"], str)
        or not COMMIT_SHA.fullmatch(receipt["sourceRevision"])
        or not isinstance(receipt["studySourceSha256"], str)
        or not SHA256_HEX.fullmatch(receipt["studySourceSha256"])
    ):
        raise StudyError("MCP build receipt source identity is invalid")
    exact_values = {
        "sourcePolicy": "verified-clean-commit-before-and-after",
        "environmentPolicy": "bounded-inheritance-no-build-overrides-v1",
        "profile": "debug",
        "features": "none",
        "locked": True,
        "incremental": False,
        "targetDirectoryPolicy": "fresh-explicit-private",
        "artifactPolicy": "cargo-json-private-copy-hash-before-and-after-execution",
    }
    if any(receipt.get(key) != value for key, value in exact_values.items()):
        raise StudyError("MCP build receipt policy differs")
    for key in ("cargoVersion", "rustcVersion", "targetTriple"):
        if (
            not isinstance(receipt[key], str)
            or not receipt[key].strip()
            or len(receipt[key]) > 512
        ):
            raise StudyError(f"MCP build receipt {key} is invalid")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", receipt["targetTriple"]):
        raise StudyError("MCP build receipt target triple is invalid")
    if receipt["binarySha256"] != binary_sha256:
        raise StudyError("MCP build receipt binary digest differs")
    bounded_canonical_size(receipt, "MCP build receipt", 8192)


def validate_event_shape(record: dict[str, Any]) -> None:
    """Reject unknown, missing, oversized, or privacy-unsafe event fields."""
    event_type = record.get("type")
    if event_type not in EVENT_FIELDS:
        raise StudyError(f"unsupported event type {event_type!r}")
    required, optional = EVENT_FIELDS[event_type]
    keys = set(record)
    missing = required - keys
    if missing:
        raise StudyError(
            f"{event_type} event is missing field(s): " + ", ".join(sorted(missing))
        )
    unknown = keys - required - optional - EVENT_PROCESSING_FIELDS
    if unknown:
        raise StudyError(
            f"{event_type} event contains unknown field(s): "
            + ", ".join(sorted(unknown))
        )
    if "redactedFieldCount" in record:
        count = record["redactedFieldCount"]
        if isinstance(count, bool) or not isinstance(count, int) or count <= 0:
            raise StudyError("redactedFieldCount must be a positive integer")
    if record.get("schemaVersion") != EVENT_SCHEMA:
        raise StudyError("event schema version does not match the frozen runner")
    if "sessionId" in record and (
        not isinstance(record["sessionId"], str)
        or not record["sessionId"].strip()
        or len(record["sessionId"]) > 128
    ):
        raise StudyError("sessionId must be a nonempty string")
    if event_type == "session":
        for key in (
            "modelFamily",
            "modelIdentifier",
            "provider",
            "backendRevision",
            "reasoningEffort",
            "date",
            "numinousCommit",
            "mcpProtocolRevision",
            "operatingSystem",
            "runnerVersion",
            "condition",
            "contextId",
            "capabilityPolicy",
        ):
            if (
                not isinstance(record[key], str)
                or not record[key].strip()
                or len(record[key]) > 256
            ):
                raise StudyError(f"session {key} must be a bounded nonempty string")
        if record["consent"] is not True:
            raise StudyError("session consent must be true")
        if record["publicationConsent"] not in ("aggregate-only", "bounded-raw"):
            raise StudyError("session publicationConsent is invalid")
        if not isinstance(
            record["attemptStartReceiptSha256"], str
        ) or not SHA256_HEX.fullmatch(record["attemptStartReceiptSha256"]):
            raise StudyError("session attempt start receipt is invalid")
        settings = record["settings"]
        if not isinstance(settings, dict):
            raise StudyError("session settings must be a bounded object")
        bounded_canonical_size(settings, "session settings", 4096)
        if any(isinstance(value, (dict, list)) for value in settings.values()):
            raise StudyError("session settings must contain only scalar values")
    elif event_type == "tool":
        if not isinstance(record["tool"], str) or not record["tool"].strip():
            raise StudyError("tool name must be a nonempty string")
        if not isinstance(record["arguments"], dict):
            raise StudyError("tool arguments must be an object")
        if record["structuredResult"] is not None and not isinstance(
            record["structuredResult"], dict
        ):
            raise StudyError("tool structured result must be an object or null")
        if not isinstance(record["visibleText"], str):
            raise StudyError("tool visibleText must be a string")
        if record["toolOutcome"] not in ("success", "error"):
            raise StudyError("toolOutcome must be success or error")
        if (
            not isinstance(record["binarySha256"], str)
            or not SHA256_HEX.fullmatch(record["binarySha256"])
        ):
            raise StudyError("tool binarySha256 must be a SHA-256 digest")
        validate_mcp_build_receipt(
            record["binaryBuildReceipt"], record["binarySha256"]
        )
        if record["room"] not in ROOMS:
            raise StudyError("tool room is not a study flagship")
        exact_int(record["sequence"], "tool sequence", 1, TOOL_CALLS_PER_ROOM)
        if not isinstance(record["role"], str) or not record["role"].strip():
            raise StudyError("tool role must be a nonempty string")
        if len(record["visibleText"].encode("utf-8")) > 65_536:
            raise StudyError("tool visibleText exceeds the public evidence limit")
        bounded_canonical_size(record["arguments"], "tool arguments", 65_536)
        bounded_canonical_size(
            record["structuredResult"], "tool structured result", 524_288
        )
    elif event_type in ("response", "distractor_response"):
        answer = record["answer"]
        if isinstance(answer, (dict, list)) or answer is None:
            raise StudyError(f"{event_type} answer must be scalar")
        if isinstance(answer, str) and len(answer) > 128:
            raise StudyError(f"{event_type} answer exceeds the scalar limit")
        bounded_canonical_size(answer, f"{event_type} answer", 256)
        if event_type == "response":
            if record["phase"] not in ("immediate", "late"):
                raise StudyError("response phase must be immediate or late")
            if not isinstance(record["probeId"], str) or not record["probeId"]:
                raise StudyError("response probeId must be a nonempty string")
            exact_int(record["attempt"], "response attempt", 1, 2)
        elif not isinstance(record["itemId"], str) or not record["itemId"]:
            raise StudyError("distractor itemId must be a nonempty string")
    elif event_type == "response_refusal":
        if record["phase"] not in ("immediate", "late"):
            raise StudyError("response refusal phase must be immediate or late")
        if not isinstance(record["probeId"], str) or not record["probeId"]:
            raise StudyError("response refusal probeId must be a nonempty string")
    elif event_type == "condition_response":
        if record["room"] not in ROOMS:
            raise StudyError("condition response room is invalid")
        if record["stage"] not in (
            "prediction",
            "construction",
            "elaboration",
        ):
            raise StudyError("condition response stage is invalid")
        if record["stage"] in ("prediction", "construction"):
            if set(record) - EVENT_PROCESSING_FIELDS != {
                "schemaVersion",
                "type",
                "sessionId",
                "room",
                "stage",
                "answer",
                "rationale",
            }:
                raise StudyError("generation response fields differ")
            answer = record["answer"]
            rationale = record["rationale"]
            if isinstance(answer, (dict, list)) or answer is None:
                raise StudyError("generation answer must be scalar")
            if not isinstance(rationale, str) or not 12 <= len(rationale) <= 256:
                raise StudyError("generation rationale must contain 12 through 256 characters")
            bounded_canonical_size(answer, "generation answer", 256)
        else:
            if set(record) - EVENT_PROCESSING_FIELDS != {
                "schemaVersion",
                "type",
                "sessionId",
                "room",
                "stage",
                "text",
            }:
                raise StudyError("explanation response fields differ")
            if not isinstance(record["text"], str) or not 12 <= len(record["text"]) <= 256:
                raise StudyError("explanation text must contain 12 through 256 characters")
    elif event_type == "feedback":
        if record["room"] not in ROOMS:
            raise StudyError("feedback room is invalid")
        if not isinstance(record["text"], str) or not 1 <= len(record["text"]) <= 512:
            raise StudyError("feedback text is invalid")
        if isinstance(record["expectedAnswer"], (dict, list)) or record[
            "expectedAnswer"
        ] is None:
            raise StudyError("feedback expected answer must be scalar")
        if record["participantCorrect"] is not None and not isinstance(
            record["participantCorrect"], bool
        ):
            raise StudyError("feedback participantCorrect must be boolean or null")
    elif event_type == "material":
        if record["room"] != "formula-jam" or record["kind"] != "reveal":
            raise StudyError("only the frozen Formula Jam Reveal material is valid")
        if not isinstance(record["text"], str) or not 1 <= len(record["text"]) <= 2048:
            raise StudyError("material text is invalid")
        if (
            not isinstance(record["materialSha256"], str)
            or not SHA256_HEX.fullmatch(record["materialSha256"])
            or record["materialSha256"] != content_sha256(record["text"])
        ):
            raise StudyError("material content hash differs")
    elif event_type == "session_interruption":
        if record["stage"] not in ("encounter", "immediate", "distractor", "late"):
            raise StudyError("session interruption stage is invalid")
        if record["reasonCode"] not in (
            "participant-stop",
            "context-lost",
            "runtime-failure",
        ):
            raise StudyError("session interruption reason code is invalid")
        if not SHA256_HEX.fullmatch(record["terminalRequestSha256"]):
            raise StudyError("session interruption request commitment is invalid")
    elif event_type == "infrastructure_failure":
        if record["stage"] != "before_exposure":
            raise StudyError("infrastructure failure stage is invalid")
        if record["reasonCode"] not in ("runtime-unavailable", "tool-unavailable"):
            raise StudyError("infrastructure failure reason code is invalid")
    elif event_type == "recruitment_refusal":
        if record["modelFamily"] not in MODEL_FAMILIES:
            raise StudyError("recruitment refusal model family is invalid")
        exact_int(
            record["familyRefusalOrdinal"],
            "recruitment refusal ordinal",
            1,
            100_000,
        )
    elif event_type == "withdrawal":
        tombstones = record["contextTombstones"]
        if (
            not isinstance(tombstones, list)
            or not 1 <= len(tombstones) <= 2
            or any(
                not isinstance(value, str) or not SHA256_HEX.fullmatch(value)
                for value in tombstones
            )
            or len(set(tombstones)) != len(tombstones)
        ):
            raise StudyError("withdrawal context tombstones are invalid")
        if not SHA256_HEX.fullmatch(record["terminalRequestSha256"]):
            raise StudyError("withdrawal request commitment is invalid")
    elif event_type == "deviation":
        exact_int(record["deviationOrdinal"], "deviation ordinal", 1, 100_000)
        if (
            not isinstance(record["code"], str)
            or not re.fullmatch(r"[a-z0-9-]{1,64}", record["code"])
        ):
            raise StudyError("deviation code is invalid")
        if not isinstance(record["description"], str) or not 1 <= len(
            record["description"]
        ) <= 2048:
            raise StudyError("deviation description is invalid")


def validate_session_header(
    header: dict[str, Any], pair: dict[str, Any], session: dict[str, Any]
) -> None:
    """Validate consent and the exact reproducibility metadata."""
    if header.get("schemaVersion") != EVENT_SCHEMA or header.get("type") != "session":
        raise StudyError("session header has the wrong schema or type")
    if header.get("consent") is not True:
        raise StudyError(f"session {session['sessionId']} lacks explicit consent")
    if header.get("publicationConsent") not in ("aggregate-only", "bounded-raw"):
        raise StudyError(
            f"session {session['sessionId']} publication consent is invalid"
        )
    if header.get("sessionId") != session["sessionId"]:
        raise StudyError("session header id mismatch")
    if header.get("modelFamily") != pair["modelFamily"]:
        raise StudyError(f"session {session['sessionId']} model family mismatch")
    if header.get("modelIdentifier") != pair["modelFamily"]:
        raise StudyError(f"session {session['sessionId']} model identifier mismatch")
    if header.get("provider") != MODEL_PROVIDERS[pair["modelFamily"]]:
        raise StudyError(f"session {session['sessionId']} provider mismatch")
    if header.get("backendRevision") != pair["calibratedBackendRevision"]:
        raise StudyError(
            f"session {session['sessionId']} backend revision differs from calibration"
        )
    if header.get("reasoningEffort") != pair["reasoningEffort"]:
        raise StudyError(f"session {session['sessionId']} reasoning effort mismatch")
    if header.get("condition") != session["condition"]:
        raise StudyError(f"session {session['sessionId']} condition mismatch")
    context_id = required_string(header, "contextId")
    if not SHA256_HEX.fullmatch(context_id):
        raise StudyError(
            f"session {session['sessionId']} context id must be an opaque SHA-256 value"
        )
    if header.get("capabilityPolicy") != "collector-only-no-repository-web-or-tools":
        raise StudyError(f"session {session['sessionId']} capability policy mismatch")
    for key in (
        "backendRevision",
        "date",
        "mcpProtocolRevision",
        "operatingSystem",
    ):
        required_string(header, key)
    try:
        date.fromisoformat(header["date"])
    except ValueError as error:
        raise StudyError(
            f"session {session['sessionId']} date must be ISO 8601"
        ) from error
    if header.get("runnerVersion") != RUNNER_VERSION:
        raise StudyError(f"session {session['sessionId']} runner version mismatch")
    if header.get("studySourceSha256") != pair["studySourceSha256"]:
        raise StudyError(f"session {session['sessionId']} study source mismatch")
    attempt_start_receipt = required_string(header, "attemptStartReceiptSha256")
    if not SHA256_HEX.fullmatch(attempt_start_receipt):
        raise StudyError(f"session {session['sessionId']} start receipt is invalid")
    if header.get("mcpProtocolRevision") != MCP_PROTOCOL_REVISION:
        raise StudyError(
            f"session {session['sessionId']} MCP protocol revision mismatch"
        )
    commit = required_string(header, "numinousCommit")
    if not COMMIT_SHA.fullmatch(commit):
        raise StudyError(f"session {session['sessionId']} has an invalid commit SHA")
    if header.get("settings") != {"sampling": "platform-default", "freshContext": True}:
        raise StudyError(f"session {session['sessionId']} settings mismatch")


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


def encounter_rooms(spec: dict[str, Any] | None = None) -> dict[str, dict[str, Any]]:
    """Index the validated executable encounter specification."""
    spec = spec or load_encounter_spec()
    return {room["id"]: room for room in spec["rooms"]}


def condition_calls(room: dict[str, Any], condition: str) -> list[dict[str, Any]]:
    """Return exact MCP calls in the delivery order for one arm."""
    calls = room["calls"]
    if room["id"] == "formula-jam":
        if condition == CONDITIONS[0]:
            return calls
        return [
            calls[0],
            {**calls[1], "arguments": {"expr": "sin(2*x)"}},
            calls[2],
            calls[3],
        ]
    if condition == CONDITIONS[0]:
        return calls
    return [calls[0], calls[3], calls[1], calls[2]]


def validate_generation_answer(room: dict[str, Any], answer: Any) -> str:
    """Validate and normalize one frozen prediction or Formula construction."""
    schema = room["generationAnswerSchema"]
    if room["id"] == "formula-jam":
        if (
            not isinstance(answer, str)
            or not FORMULA_CONSTRUCTION.fullmatch(answer)
        ):
            raise StudyError(
                "Formula Jam construction must be sin(k*x) or cos(k*x) for integer "
                "k from 2 through 99"
            )
        return answer
    if not isinstance(answer, str):
        raise StudyError(f"{room['id']} generation answer must be a string")
    normalized = answer.strip().casefold()
    choices = {choice.casefold(): choice for choice in schema["enum"]}
    if normalized not in choices:
        raise StudyError(f"{room['id']} generation answer is outside the frozen schema")
    return choices[normalized]


def validate_feedback_evidence(
    room: dict[str, Any], structured_result: Any, interaction_arguments: dict[str, Any]
) -> None:
    """Require the public interaction result to support its frozen feedback."""
    if not isinstance(structured_result, dict):
        raise StudyError(f"{room['id']} interaction result is unavailable")
    evidence = room["feedbackEvidence"]
    observed = structured_result.get(evidence["field"])
    if "contains" in evidence:
        if not isinstance(observed, str) or evidence["contains"] not in observed:
            raise StudyError(f"{room['id']} interaction result does not support feedback")
    elif observed != interaction_arguments.get("expr"):
        raise StudyError("Formula Jam interaction did not plot the resolved expression")


def validate_tool_events(
    pair: dict[str, Any],
    session: dict[str, Any],
    events: list[dict[str, Any]],
    *,
    allow_erased_participant_tool: bool = False,
) -> None:
    """Enforce equal call budgets, room order, and Reveal ordering."""
    rooms = encounter_rooms()
    if len(events) != len(ROOMS) * TOOL_CALLS_PER_ROOM:
        raise StudyError(
            f"session {session['sessionId']} must record exactly "
            f"{len(ROOMS) * TOOL_CALLS_PER_ROOM} public tool calls"
        )
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for event in events:
        room = event.get("room")
        if room not in ROOMS:
            raise StudyError(
                f"session {session['sessionId']} tool event has invalid room"
            )
        required_string(event, "tool")
        if not isinstance(event.get("arguments"), dict):
            raise StudyError("tool event arguments must be an object")
        if event.get("structuredResult") is not None and not isinstance(
            event.get("structuredResult"), dict
        ):
            raise StudyError("tool event structuredResult must be an object or null")
        if not isinstance(event.get("visibleText"), str):
            raise StudyError("tool event visibleText must be a string")
        if event.get("toolOutcome") not in ("success", "error"):
            raise StudyError("tool event outcome must be success or error")
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
        raise StudyError(
            f"session {session['sessionId']} room order differs from allocation"
        )
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
        raise StudyError(
            f"session {session['sessionId']} interleaves or reorders room calls"
        )
    for room in pair["roomOrder"]:
        room_events = sorted(grouped[room], key=lambda item: item["_sourceIndex"])
        sequences = [event.get("sequence") for event in room_events]
        roles = [event.get("role") for event in room_events]
        expected_calls = condition_calls(rooms[room], session["condition"])
        expected_roles = [call["role"] for call in expected_calls]
        if sequences != list(range(1, TOOL_CALLS_PER_ROOM + 1)):
            raise StudyError(
                f"session {session['sessionId']} {room} call sequence is invalid"
            )
        if roles != expected_roles:
            raise StudyError(
                f"session {session['sessionId']} {room} role order is invalid"
            )
        for event, expected_call in zip(room_events, expected_calls, strict=True):
            if event["tool"] != expected_call["tool"]:
                raise StudyError(f"session {session['sessionId']} {room} tool differs")
            dynamic_formula = (
                room == "formula-jam"
                and session["condition"] == CONDITIONS[0]
                and expected_call["arguments"].get("expr")
                == "__PARTICIPANT_EXPRESSION__"
            )
            erased_formula = dynamic_formula and (
                event.get("arguments") == ERASED_PARTICIPANT_TOOL_CONTENT
                and event.get("structuredResult") == ERASED_PARTICIPANT_TOOL_CONTENT
                and event.get("visibleText") == ""
            )
            if erased_formula and not allow_erased_participant_tool:
                raise StudyError(
                    f"session {session['sessionId']} {room} erases a completed response"
                )
            if not dynamic_formula and canonical_bytes(
                event["arguments"]
            ) != canonical_bytes(expected_call["arguments"]):
                raise StudyError(
                    f"session {session['sessionId']} {room} arguments differ"
                )
            if event["toolOutcome"] != "success":
                raise StudyError(
                    f"session {session['sessionId']} {room} tool call failed"
                )
            if expected_call["role"] == "interaction" and not erased_formula:
                validate_feedback_evidence(
                    rooms[room], event["structuredResult"], event["arguments"]
                )
        reveal_events = [
            event for event in room_events if event.get("role") == "reveal"
        ]
        if room != "formula-jam":
            if len(reveal_events) != 1:
                raise StudyError(
                    f"session {session['sessionId']} {room} lacks one Reveal"
                )
            reveal_event = reveal_events[0]
            if not reveal_event["visibleText"] and not reveal_event["structuredResult"]:
                raise StudyError(
                    f"session {session['sessionId']} {room} has an empty Reveal"
                )
        if session["condition"] == CONDITIONS[0]:
            for event in room_events:
                if event.get("role") == "reveal":
                    break
                if contains_nonempty_key(event.get("structuredResult"), "reveal"):
                    raise StudyError(
                        f"session {session['sessionId']} {room} leaked Reveal before generation"
                    )


def validate_tool_event_prefix(
    pair: dict[str, Any], session: dict[str, Any], events: list[dict[str, Any]]
) -> None:
    """Validate a nonempty encounter prefix for a post-exposure interruption."""
    if not events:
        raise StudyError(
            f"session {session['sessionId']} interruption must occur after exposure"
        )
    if len(events) > len(ROOMS) * TOOL_CALLS_PER_ROOM:
        raise StudyError(
            f"session {session['sessionId']} encounter interruption exceeds full exposure"
        )
    rooms = encounter_rooms()
    expected = [
        (room, sequence, call)
        for room in pair["roomOrder"]
        for sequence, call in enumerate(
            condition_calls(rooms[room], session["condition"]), start=1
        )
    ]
    ordered = sorted(events, key=lambda item: item["_sourceIndex"])
    for event, (room, sequence, call) in zip(
        ordered, expected[: len(ordered)], strict=True
    ):
        role = call["role"]
        if (
            event.get("room") != room
            or event.get("sequence") != sequence
            or event.get("role") != role
        ):
            raise StudyError(
                f"session {session['sessionId']} interrupted encounter is not a valid prefix"
            )
        dynamic_formula = (
            room == "formula-jam"
            and session["condition"] == CONDITIONS[0]
            and call["arguments"].get("expr") == "__PARTICIPANT_EXPRESSION__"
        )
        erased_formula = dynamic_formula and (
            event.get("arguments") == ERASED_PARTICIPANT_TOOL_CONTENT
            and event.get("structuredResult") == ERASED_PARTICIPANT_TOOL_CONTENT
            and event.get("visibleText") == ""
        )
        if event.get("tool") != call["tool"] or (
            not dynamic_formula
            and canonical_bytes(event.get("arguments"))
            != canonical_bytes(call["arguments"])
        ):
            raise StudyError(
                f"session {session['sessionId']} interrupted encounter call differs"
            )
        required_string(event, "tool")
        if not isinstance(event.get("arguments"), dict):
            raise StudyError("tool event arguments must be an object")
        if event.get("structuredResult") is not None and not isinstance(
            event.get("structuredResult"), dict
        ):
            raise StudyError("tool event structuredResult must be an object or null")
        if not isinstance(event.get("visibleText"), str):
            raise StudyError("tool event visibleText must be a string")
        if event.get("toolOutcome") not in ("success", "error"):
            raise StudyError("tool event outcome must be success or error")
        if dynamic_formula and not erased_formula:
            validate_feedback_evidence(
                rooms[room], event["structuredResult"], event["arguments"]
            )
        if session["condition"] == CONDITIONS[0] and role != "reveal":
            if contains_nonempty_key(event.get("structuredResult"), "reveal"):
                raise StudyError(
                    f"session {session['sessionId']} {room} leaked Reveal before generation"
                )


def reveal_payloads(events: list[dict[str, Any]]) -> dict[str, bytes]:
    """Return the public Reveal payload for pairwise equality checks."""
    payloads: dict[str, bytes] = {}
    for event in events:
        if event.get("type") == "tool" and event.get("role") == "reveal":
            room = event["room"]
            payload = {
                "structuredResult": event["structuredResult"],
                "visibleText": event["visibleText"],
            }
        elif event.get("type") == "material" and event.get("kind") == "reveal":
            room = event["room"]
            payload = {"material": event["text"]}
        else:
            continue
        if room in payloads:
            raise StudyError(f"duplicate Reveal payload for {room}")
        payloads[room] = canonical_bytes(payload)
    if set(payloads) != set(ROOMS):
        raise StudyError("session does not contain one Reveal payload per room")
    return payloads


def retained_reveal_payloads(events: list[dict[str, Any]]) -> dict[str, bytes]:
    """Return every retained Reveal without requiring a complete encounter."""
    payloads: dict[str, bytes] = {}
    for event in events:
        if event.get("type") == "tool" and event.get("role") == "reveal":
            room = event["room"]
            payload = {
                "structuredResult": event["structuredResult"],
                "visibleText": event["visibleText"],
            }
        elif event.get("type") == "material" and event.get("kind") == "reveal":
            room = event["room"]
            payload = {"material": event["text"]}
        else:
            continue
        if room in payloads:
            raise StudyError(f"duplicate Reveal payload for {room}")
        payloads[room] = canonical_bytes(payload)
    return payloads


def retained_tool_payloads(
    events: list[dict[str, Any]],
) -> dict[tuple[str, str], bytes]:
    """Index canonical public MCP payloads by room and specification role."""
    payloads: dict[tuple[str, str], bytes] = {}
    for event in events:
        if event.get("type") != "tool":
            continue
        key = (event["room"], event["role"])
        if key == ("formula-jam", "interaction"):
            continue
        if key in payloads:
            raise StudyError(f"duplicate retained tool payload for {key[0]} {key[1]}")
        payloads[key] = canonical_bytes(
            {
                "tool": event["tool"],
                "arguments": event["arguments"],
                "structuredResult": event["structuredResult"],
                "visibleText": event["visibleText"],
                "toolOutcome": event["toolOutcome"],
            }
        )
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

    def score_probe(probe: dict[str, Any]) -> None:
        nonlocal cursor, invalid_attempts, repairs, refusals
        event = current()
        if event is None or event.get("probeId") != probe["id"]:
            raise StudyError(
                f"session {session_id} is missing ordered probe {probe['id']}"
            )
        if event.get("phase") != probe["phase"]:
            raise StudyError(
                f"session {session_id} probe {probe['id']} has the wrong phase"
            )
        if event.get("type") == "response_refusal":
            scores[probe["id"]] = 0
            refusals += 1
            cursor += 1
            return
        if event.get("type") != "response" or event.get("attempt") != 1:
            raise StudyError(
                f"session {session_id} has an invalid first response event"
            )
        valid, correct = score_answer(probe, event.get("answer"))
        cursor += 1
        if valid:
            scores[probe["id"]] = int(correct)
            return
        invalid_attempts += 1
        retry = current()
        if (
            retry is not None
            and retry.get("type") in ("response", "response_refusal")
            and retry.get("probeId") == probe["id"]
        ):
            if retry.get("phase") != probe["phase"]:
                raise StudyError(
                    f"session {session_id} probe {probe['id']} repair has the wrong phase"
                )
            repairs += 1
            if retry["type"] == "response_refusal":
                refusals += 1
                scores[probe["id"]] = 0
                cursor += 1
                return
            if retry.get("attempt") != 2:
                raise StudyError(
                    f"session {session_id} probe {probe['id']} repair attempt differs"
                )
            valid, correct = score_answer(probe, retry.get("answer"))
            invalid_attempts += int(not valid)
            scores[probe["id"]] = int(valid and correct)
            cursor += 1
        else:
            scores[probe["id"]] = 0

    for probe in expected_probes[:10]:
        score_probe(probe)

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
        score_probe(probe)
    if cursor != len(response_events):
        raise StudyError(f"session {session_id} has extra or out-of-order probe events")
    return {
        "scores": scores,
        "invalidAttempts": invalid_attempts,
        "schemaRepairs": repairs,
        "responseRefusals": refusals,
    }


def validate_condition_fidelity(
    pair: dict[str, Any], session: dict[str, Any], events: list[dict[str, Any]]
) -> None:
    """Require the exact intervention sequence and bounded participant receipts."""
    rooms = encounter_rooms()
    observed = [
        event
        for event in sorted(events, key=lambda item: item["_sourceIndex"])
        if event.get("type")
        in ("tool", "condition_response", "material", "feedback")
    ]
    expected: list[tuple[str, str, str]] = []
    for room_id in pair["roomOrder"]:
        room = rooms[room_id]
        calls = condition_calls(room, session["condition"])
        if session["condition"] == CONDITIONS[0]:
            response_stage = (
                "construction" if room_id == "formula-jam" else "prediction"
            )
            expected.append(("tool", room_id, calls[0]["role"]))
            expected.append(("condition_response", room_id, response_stage))
            expected.append(("tool", room_id, calls[1]["role"]))
            expected.append(("feedback", room_id, "outcome"))
            expected.extend(("tool", room_id, call["role"]) for call in calls[2:])
            if room_id == "formula-jam":
                expected.append(("material", room_id, "reveal"))
        else:
            expected.append(("tool", room_id, calls[0]["role"]))
            if room_id == "formula-jam":
                expected.append(("material", room_id, "reveal"))
                expected.append(("condition_response", room_id, "elaboration"))
                interaction_index = 1
            else:
                expected.append(("tool", room_id, calls[1]["role"]))
                expected.append(("condition_response", room_id, "elaboration"))
                interaction_index = 2
            expected.append(("tool", room_id, calls[interaction_index]["role"]))
            expected.append(("feedback", room_id, "outcome"))
            expected.extend(
                ("tool", room_id, call["role"])
                for call in calls[interaction_index + 1 :]
            )
    if len(observed) != len(expected):
        raise StudyError(
            f"session {session['sessionId']} condition fidelity event count differs"
        )
    for event, (event_type, room_id, stage) in zip(observed, expected, strict=True):
        actual_stage = event.get("role") if event_type == "tool" else event.get("stage")
        if event_type == "material":
            actual_stage = event.get("kind")
        elif event_type == "feedback":
            actual_stage = "outcome"
        if (
            event.get("type") != event_type
            or event.get("room") != room_id
            or actual_stage != stage
        ):
            raise StudyError(
                f"session {session['sessionId']} condition fidelity sequence differs"
            )
        if event_type == "material":
            expected_text = rooms[room_id]["revealMaterial"]
            if event["text"] != expected_text:
                raise StudyError(
                    f"session {session['sessionId']} Formula Jam Reveal differs"
                )
    for room_id in pair["roomOrder"]:
        room = rooms[room_id]
        room_events = [event for event in observed if event.get("room") == room_id]
        feedback = next(event for event in room_events if event["type"] == "feedback")
        if (
            feedback["expectedAnswer"] != room["expectedAnswer"]
            or feedback["text"] != room["feedbackText"]
        ):
            raise StudyError(f"session {session['sessionId']} {room_id} feedback differs")
        if session["condition"] == CONDITIONS[0]:
            response_stage = "construction" if room_id == "formula-jam" else "prediction"
            response = next(
                event
                for event in room_events
                if event.get("type") == "condition_response"
                and event.get("stage") == response_stage
            )
            normalized_answer = validate_generation_answer(room, response.get("answer"))
            if room_id == "formula-jam":
                interaction = next(
                    event
                    for event in room_events
                    if event.get("type") == "tool" and event.get("role") == "interaction"
                )
                if interaction["arguments"] != {"expr": normalized_answer}:
                    raise StudyError(
                        f"session {session['sessionId']} Formula construction was not executed"
                    )
                correct = bool(interaction.get("structuredResult", {}).get("expression"))
            else:
                correct = normalized_answer == room["expectedAnswer"]
            if feedback["participantCorrect"] is not correct:
                raise StudyError(
                    f"session {session['sessionId']} {room_id} feedback is not response-contingent"
                )
        elif feedback["participantCorrect"] is not None:
            raise StudyError(
                f"session {session['sessionId']} {room_id} control feedback must be neutral"
            )


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
    participant_content_events = [
        event
        for event in events
        if event.get("type")
        in (
            "response",
            "response_refusal",
            "distractor_response",
            "condition_response",
            "feedback",
        )
    ]
    for event in tool_events:
        build_receipt = event.get("binaryBuildReceipt")
        if (
            not isinstance(build_receipt, dict)
            or build_receipt.get("sourceRevision") != header["numinousCommit"]
            or build_receipt.get("studySourceSha256")
            != header["studySourceSha256"]
        ):
            raise StudyError(
                f"session {session['sessionId']} MCP build source differs"
            )
    interrupted = completion.get("type") == "session_interruption"
    if interrupted:
        if participant_content_events:
            raise StudyError(
                f"session {session['sessionId']} interruption must retain no response content"
            )
        stage = completion.get("stage")
        if stage not in ("encounter", "immediate", "distractor", "late"):
            raise StudyError(
                f"session {session['sessionId']} has an invalid interruption stage"
            )
        required_string(completion, "reasonCode")
        if stage == "encounter":
            validate_tool_event_prefix(pair, session, tool_events)
        else:
            validate_tool_events(
                pair,
                session,
                tool_events,
                allow_erased_participant_tool=True,
            )
        if header["_sourceIndex"] >= min(
            event["_sourceIndex"] for event in tool_events
        ):
            raise StudyError(
                f"session {session['sessionId']} activity precedes consent metadata"
            )
        if completion["_sourceIndex"] <= max(
            event["_sourceIndex"] for event in tool_events
        ):
            raise StudyError(
                f"session {session['sessionId']} interruption is out of order"
            )
        adverse_score = 0.0 if session["condition"] == CONDITIONS[0] else 1.0
        adverse_rooms = {
            phase: {room: adverse_score for room in ROOMS}
            for phase in ("immediate", "late")
        }
        return {
            "sessionId": session["sessionId"],
            "condition": session["condition"],
            "immediateScore": adverse_score,
            "lateScore": adverse_score,
            "roomScores": adverse_rooms,
            "invalidAttempts": 0,
            "schemaRepairs": 0,
            "responseRefusals": 0,
            "toolErrors": sum(
                event.get("toolOutcome") == "error" for event in tool_events
            ),
            "interrupted": True,
            "interruptionStage": stage,
            "missingDataRule": "hypothesis-adverse",
        }
    validate_tool_events(pair, session, tool_events)
    validate_condition_fidelity(pair, session, events)
    if not probe_events:
        raise StudyError(f"session {session['sessionId']} has no probe events")
    if max(event["_sourceIndex"] for event in tool_events) >= min(
        event["_sourceIndex"] for event in probe_events
    ):
        raise StudyError(
            f"session {session['sessionId']} probes began before encounters ended"
        )
    if header["_sourceIndex"] >= min(event["_sourceIndex"] for event in tool_events):
        raise StudyError(
            f"session {session['sessionId']} activity precedes consent metadata"
        )
    if completion["_sourceIndex"] <= max(
        event["_sourceIndex"] for event in probe_events
    ):
        raise StudyError(f"session {session['sessionId']} completion is out of order")
    expected = probe_sequence(bank, pair["roomOrder"], "immediate") + probe_sequence(
        bank, pair["roomOrder"], "late"
    )
    ordered = sorted(probe_events, key=lambda event: event["_sourceIndex"])
    result = ordered_response_score(
        session["sessionId"], expected, bank["distractorSequence"]["items"], ordered
    )
    if result["schemaRepairs"] > 1:
        raise StudyError(f"session {session['sessionId']} exceeds one schema repair")
    probe_by_id = {probe["id"]: probe for probe in expected}
    phase_scores: dict[str, float] = {}
    room_scores: dict[str, dict[str, float]] = {}
    for phase in ("immediate", "late"):
        phase_items = [probe for probe in expected if probe["phase"] == phase]
        phase_scores[phase] = (
            sum(result["scores"][probe["id"]] for probe in phase_items) / 10.0
        )
        room_scores[phase] = {}
        for room in ROOMS:
            ids = [
                probe_id
                for probe_id, probe in probe_by_id.items()
                if probe["phase"] == phase and probe["room"] == room
            ]
            room_scores[phase][room] = (
                sum(result["scores"][probe_id] for probe_id in ids) / 2.0
            )
    return {
        "sessionId": session["sessionId"],
        "condition": session["condition"],
        "immediateScore": phase_scores["immediate"],
        "lateScore": phase_scores["late"],
        "roomScores": room_scores,
        "invalidAttempts": result["invalidAttempts"],
        "schemaRepairs": result["schemaRepairs"],
        "responseRefusals": result["responseRefusals"],
        "toolErrors": sum(event.get("toolOutcome") == "error" for event in tool_events),
        "interrupted": False,
        "interruptionStage": None,
        "missingDataRule": None,
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
    differences: dict[str, list[float]],
    resamples: int = BOOTSTRAP_RESAMPLES,
    seed: str = BOOTSTRAP_SEED,
) -> dict[str, Any]:
    """Bootstrap 10 pairs within each family, then pool all 20 differences."""
    if set(differences) != set(MODEL_FAMILIES):
        raise StudyError("bootstrap requires both frozen model families")
    if any(len(differences[model]) != 10 for model in MODEL_FAMILIES):
        raise StudyError("bootstrap requires exactly 10 pair differences per family")
    if resamples <= 0:
        raise StudyError("bootstrap resample count must be positive")
    rng = StableRng(seed)
    pooled_distribution: list[float] = []
    family_distributions: dict[str, list[float]] = {
        model: [] for model in MODEL_FAMILIES
    }
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
        "seed": seed,
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
    deviation_count = 0
    allowed_types = {
        "session",
        "tool",
        "response",
        "response_refusal",
        "distractor_response",
        "condition_response",
        "feedback",
        "material",
        "session_complete",
        "session_interruption",
        "recruitment_refusal",
        "withdrawal",
        "infrastructure_failure",
        "deviation",
    }
    for record in records:
        validate_event_shape(record)
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
                "familyRefusalOrdinal",
                "_sourceIndex",
                "redactedFieldCount",
            }
            if set(record) - allowed:
                raise StudyError(
                    "recruitment refusals may contain only aggregate count fields"
                )
            if record["familyRefusalOrdinal"] != recruitment_refusals[model] + 1:
                raise StudyError(
                    "recruitment refusal ordinals must be contiguous per model family"
                )
            recruitment_refusals[model] += 1
            continue
        if event_type == "deviation":
            deviation_count += 1
            if record["deviationOrdinal"] != deviation_count:
                raise StudyError("deviation ordinals must be contiguous")
        if event_type in ("withdrawal", "infrastructure_failure"):
            pair_id = record.get("pairId")
            if pair_id not in pairs or pair_id in pair_outcomes:
                raise StudyError("pair outcome has an invalid or duplicate pair id")
            if event_type == "infrastructure_failure":
                if record.get("stage") != "before_exposure":
                    raise StudyError(
                        "infrastructure failures must occur before exposure"
                    )
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
                    "contextTombstones",
                    "terminalRequestSha256",
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
        elif event_type in ("session_complete", "session_interruption"):
            if session_id in completions:
                raise StudyError(f"duplicate session terminal event {session_id}")
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
                    raise StudyError(
                        "response refusal may not contain response content"
                    )
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
                    raise StudyError(
                        f"cohort continued after 10 complete pairs for {model}"
                    )
                continue
            if pair_id in pair_outcomes:
                continue
            session_ids = [session["sessionId"] for session in pair["sessions"]]
            complete = all(
                session_id in headers and session_id in completions
                for session_id in session_ids
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
            raise StudyError(
                f"incomplete cohort for {model}; two reserves are exhausted"
            )

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
        observed = [
            pair_id
            for pair_id, _index in sorted(first_indexes, key=lambda item: item[1])
        ]
        expected = [pair["pairId"] for pair in consumed]
        if observed != expected:
            raise StudyError(f"{model} pairs were not collected in frozen order")

    selected_session_ids = [
        session["sessionId"] for pair in selected_pairs for session in pair["sessions"]
    ]
    session_intervals = sorted(
        (
            headers[session_id]["_sourceIndex"],
            completions[session_id]["_sourceIndex"],
            session_id,
        )
        for session_id in selected_session_ids
    )
    for previous, current in zip(
        session_intervals, session_intervals[1:], strict=False
    ):
        if previous[1] >= current[0]:
            raise StudyError(
                f"qualifying sessions overlap: {previous[2]} and {current[2]}"
            )
    context_ids = [
        headers[session_id]["contextId"] for session_id in selected_session_ids
    ]
    if len(set(context_ids)) != len(context_ids):
        raise StudyError("all qualifying sessions must use distinct fresh context ids")
    attempt_start_receipts = [
        headers[session_id]["attemptStartReceiptSha256"]
        for session_id in selected_session_ids
    ]
    if len(set(attempt_start_receipts)) != len(attempt_start_receipts):
        raise StudyError(
            "all qualifying sessions must use distinct attempt start receipts"
        )
    withdrawn_context_tombstones = {
        tombstone
        for outcome in pair_outcomes.values()
        if outcome["type"] == "withdrawal"
        for tombstone in outcome["contextTombstones"]
    }
    if any(
        content_sha256(context_id) in withdrawn_context_tombstones
        for context_id in context_ids
    ):
        raise StudyError("a qualifying session reused a withdrawn exposed context")
    cohort_commits = {
        headers[session_id]["numinousCommit"] for session_id in selected_session_ids
    }
    if len(cohort_commits) != 1:
        raise StudyError("all qualifying sessions must use one Numinous commit")
    protocol_revisions = {
        headers[session_id]["mcpProtocolRevision"]
        for session_id in selected_session_ids
    }
    if len(protocol_revisions) != 1:
        raise StudyError("all qualifying sessions must use one MCP protocol revision")
    binary_hashes = {
        event["binarySha256"]
        for session_id in selected_session_ids
        for event in session_events[session_id]
        if event.get("type") == "tool"
    }
    if len(binary_hashes) != 1:
        raise StudyError("all qualifying tool calls must use one Numinous binary")
    binary_build_receipts = {
        canonical_bytes(event["binaryBuildReceipt"])
        for session_id in selected_session_ids
        for event in session_events[session_id]
        if event.get("type") == "tool"
    }
    if len(binary_build_receipts) != 1:
        raise StudyError("all qualifying tool calls must use one MCP build receipt")
    cohort_commit = next(iter(cohort_commits))
    protocol_revision = next(iter(protocol_revisions))
    binary_sha256 = next(iter(binary_hashes))
    binary_build_receipt = strict_json_loads(
        next(iter(binary_build_receipts)).decode("utf-8"), "MCP build receipt"
    )

    session_scores: dict[str, dict[str, Any]] = {}
    pair_results: list[dict[str, Any]] = []
    differences: dict[str, list[float]] = {model: [] for model in MODEL_FAMILIES}
    room_differences: dict[str, list[float]] = {room: [] for room in ROOMS}
    room_differences_by_family = {
        room: {model: [] for model in MODEL_FAMILIES} for room in ROOMS
    }
    late_differences: dict[str, list[float]] = {model: [] for model in MODEL_FAMILIES}
    late_room_differences: dict[str, list[float]] = {room: [] for room in ROOMS}
    late_room_differences_by_family = {
        room: {model: [] for model in MODEL_FAMILIES} for room in ROOMS
    }
    complete_case_differences: dict[str, list[float]] = {
        model: [] for model in MODEL_FAMILIES
    }
    for pair in selected_pairs:
        pair_headers = [headers[session["sessionId"]] for session in pair["sessions"]]
        observed_collection_order = [
            session_id
            for session_id, _index in sorted(
                (
                    (
                        session["sessionId"],
                        headers[session["sessionId"]]["_sourceIndex"],
                    )
                    for session in pair["sessions"]
                ),
                key=lambda item: item[1],
            )
        ]
        if observed_collection_order != pair["collectionOrder"]:
            raise StudyError(
                f"pair {pair['pairId']} condition collection order changed"
            )
        first_session_id, second_session_id = pair["collectionOrder"]
        if (
            completions[first_session_id]["_sourceIndex"]
            >= headers[second_session_id]["_sourceIndex"]
        ):
            raise StudyError(f"pair {pair['pairId']} condition sessions overlap")
        if paired_configuration_key(pair_headers[0]) != paired_configuration_key(
            pair_headers[1]
        ):
            raise StudyError(
                f"pair {pair['pairId']} does not use the same model configuration"
            )
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
        generation_events = session_events[generation["sessionId"]]
        control_events = session_events[control["sessionId"]]
        generation_tools = retained_tool_payloads(generation_events)
        control_tools = retained_tool_payloads(control_events)
        for key in set(generation_tools) & set(control_tools):
            if generation_tools[key] != control_tools[key]:
                raise StudyError(
                    f"pair {pair['pairId']} did not receive identical public MCP payloads"
                )
        generation_reveals = (
            retained_reveal_payloads(generation_events)
            if generation["interrupted"]
            else reveal_payloads(generation_events)
        )
        control_reveals = (
            retained_reveal_payloads(control_events)
            if control["interrupted"]
            else reveal_payloads(control_events)
        )
        for room in set(generation_reveals) & set(control_reveals):
            if generation_reveals[room] != control_reveals[room]:
                raise StudyError(
                    f"pair {pair['pairId']} did not receive identical Reveal payloads"
                )
        difference = generation["immediateScore"] - control["immediateScore"]
        differences[pair["modelFamily"]].append(difference)
        if not generation["interrupted"] and not control["interrupted"]:
            complete_case_differences[pair["modelFamily"]].append(difference)
        late_difference = generation["lateScore"] - control["lateScore"]
        late_differences[pair["modelFamily"]].append(late_difference)
        per_room = {}
        late_per_room = {}
        for room in ROOMS:
            room_difference = (
                generation["roomScores"]["immediate"][room]
                - control["roomScores"]["immediate"][room]
            )
            per_room[room] = room_difference
            room_differences[room].append(room_difference)
            room_differences_by_family[room][pair["modelFamily"]].append(
                room_difference
            )
            late_room_difference = (
                generation["roomScores"]["late"][room]
                - control["roomScores"]["late"][room]
            )
            late_per_room[room] = late_room_difference
            late_room_differences[room].append(late_room_difference)
            late_room_differences_by_family[room][pair["modelFamily"]].append(
                late_room_difference
            )
        pair_results.append(
            {
                "pairId": pair["pairId"],
                "modelFamily": pair["modelFamily"],
                "generationCollectedFirst": (
                    pair["collectionOrder"][0] == generation["sessionId"]
                ),
                "firstRoom": pair["roomOrder"][0],
                "generationImmediate": generation["immediateScore"],
                "controlImmediate": control["immediateScore"],
                "pairedImmediateDifference": difference,
                "generationLate": generation["lateScore"],
                "controlLate": control["lateScore"],
                "pairedLateDifference": late_difference,
                "roomImmediateDifferences": per_room,
                "roomLateDifferences": late_per_room,
            }
        )
    bootstrap = stratified_bootstrap(differences, bootstrap_resamples)
    late_bootstrap = stratified_bootstrap(
        late_differences,
        bootstrap_resamples,
        seed=LATE_BOOTSTRAP_SEED,
    )
    room_bootstrap95 = {
        room: stratified_bootstrap(
            room_differences_by_family[room],
            bootstrap_resamples,
            seed=f"{BOOTSTRAP_SEED}:room:{room}",
        )["pooled95"]
        for room in ROOMS
    }
    late_room_bootstrap95 = {
        room: stratified_bootstrap(
            late_room_differences_by_family[room],
            bootstrap_resamples,
            seed=f"{LATE_BOOTSTRAP_SEED}:room:{room}",
        )["pooled95"]
        for room in ROOMS
    }
    family_means = {
        model: sum(differences[model]) / len(differences[model])
        for model in MODEL_FAMILIES
    }
    pooled_mean = sum(family_means.values()) / 2.0
    room_means = {
        room: sum(room_differences[room]) / len(room_differences[room])
        for room in ROOMS
    }
    late_family_means = {
        model: sum(late_differences[model]) / len(late_differences[model])
        for model in MODEL_FAMILIES
    }
    late_pooled_mean = sum(late_family_means.values()) / 2.0
    late_room_means = {
        room: sum(late_room_differences[room]) / len(late_room_differences[room])
        for room in ROOMS
    }
    condition_order_groups = {
        "generationFirst": [
            result["pairedImmediateDifference"]
            for result in pair_results
            if result["generationCollectedFirst"]
        ],
        "controlFirst": [
            result["pairedImmediateDifference"]
            for result in pair_results
            if not result["generationCollectedFirst"]
        ],
    }
    first_room_groups = {
        room: [
            result["pairedImmediateDifference"]
            for result in pair_results
            if result["firstRoom"] == room
        ]
        for room in ROOMS
    }

    def descriptive_group(values: list[float]) -> dict[str, int | float]:
        if not values:
            raise StudyError("balance sensitivity group may not be empty")
        return {
            "pairs": len(values),
            "meanImmediateDifference": sum(values) / len(values),
        }

    interruption_counts = {
        model: sum(
            score["interrupted"]
            for score in session_scores.values()
            if sessions[score["sessionId"]][0]["modelFamily"] == model
        )
        for model in MODEL_FAMILIES
    }
    total_interruptions = sum(interruption_counts.values())
    complete_case_family_means = {
        model: (
            sum(complete_case_differences[model])
            / len(complete_case_differences[model])
            if complete_case_differences[model]
            else None
        )
        for model in MODEL_FAMILIES
    }
    complete_case_pooled_mean = (
        sum(value for value in complete_case_family_means.values() if value is not None)
        / len(MODEL_FAMILIES)
        if all(value is not None for value in complete_case_family_means.values())
        else None
    )
    criteria = {
        "pairedMeanAtLeastTenPoints": pooled_mean >= 0.10,
        "bootstrapLowerBoundAboveZero": bootstrap["pooled95"][0] > 0.0,
        "eachModelNonnegative": all(value >= 0.0 for value in family_means.values()),
        "fourOfFiveRoomsNonnegative": sum(value >= 0.0 for value in room_means.values())
        >= 4,
        "noRoomBelowNegativeTenPoints": all(
            value >= -0.10 for value in room_means.values()
        ),
        "interruptionCeilingMet": total_interruptions <= MAX_INTERRUPTED_SESSIONS
        and all(
            count <= MAX_INTERRUPTED_SESSIONS_PER_MODEL
            for count in interruption_counts.values()
        ),
    }
    raw_pair_ids = {
        pair["pairId"]
        for pair in selected_pairs
        if all(
            headers[session["sessionId"]]["publicationConsent"] == "bounded-raw"
            for session in pair["sessions"]
        )
    }
    raw_session_ids = {
        session_id
        for session_id in selected_session_ids
        if headers[session_id]["publicationConsent"] == "bounded-raw"
    }
    backend_revisions = {
        model: sorted(
            {
                headers[session_id]["backendRevision"]
                for session_id in selected_session_ids
                if headers[session_id]["modelFamily"] == model
            }
        )
        for model in MODEL_FAMILIES
    }
    provenance_limitations = [
        "Fresh-context and capability-policy isolation are runtime attestations; "
        "the collector cannot cryptographically prove capability removal."
    ]
    if any(
        revision == "unavailable"
        for revisions in backend_revisions.values()
        for revision in revisions
    ):
        provenance_limitations.append(
            "The provider backend revision was unavailable for at least one session."
        )
    return {
        "schemaVersion": REPORT_SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "runnerVersion": RUNNER_VERSION,
        "numinousCommit": cohort_commit,
        "mcpProtocolRevision": protocol_revision,
        "allocationSha256": content_sha256(manifest),
        "probeBankSha256": content_sha256(bank),
        "runtimeProvenance": {
            "dates": sorted(
                {headers[session_id]["date"] for session_id in selected_session_ids}
            ),
            "models": [
                {
                    "modelIdentifier": model,
                    "provider": MODEL_PROVIDERS[model],
                    "backendRevisions": backend_revisions[model],
                    "reasoningEffort": "high",
                }
                for model in MODEL_FAMILIES
            ],
            "settings": {"sampling": "platform-default", "freshContext": True},
            "numinousCommit": cohort_commit,
            "mcpProtocolRevision": protocol_revision,
            "operatingSystems": sorted(
                {
                    headers[session_id]["operatingSystem"]
                    for session_id in selected_session_ids
                }
            ),
            "runnerVersion": RUNNER_VERSION,
            "binarySha256": binary_sha256,
            "binaryBuildReceipt": binary_build_receipt,
            "limitations": provenance_limitations,
        },
        "cohortComplete": True,
        "selectedPairs": [pair["pairId"] for pair in selected_pairs],
        "recruitmentRefusals": {
            model: recruitment_refusals.get(model, 0) for model in MODEL_FAMILIES
        },
        "withdrawals": sum(
            event["type"] == "withdrawal" for event in pair_outcomes.values()
        ),
        "infrastructureFailures": sum(
            event["type"] == "infrastructure_failure"
            for event in pair_outcomes.values()
        ),
        "sessionInterruptions": sum(
            terminal["type"] == "session_interruption"
            for terminal in completions.values()
        ),
        "publicationConsents": {
            mode: sum(
                headers[session_id]["publicationConsent"] == mode
                for session_id in selected_session_ids
            )
            for mode in ("aggregate-only", "bounded-raw")
        },
        "deviations": deviations,
        "primary": {
            "pairedMeanDifference": pooled_mean,
            "familyMeanDifferences": family_means,
            "roomMeanDifferences": room_means,
            "roomBootstrap95": room_bootstrap95,
            "bootstrap": bootstrap,
            "criteria": criteria,
            "predeclaredStatisticalGatePassed": all(criteria.values()),
        },
        "secondary": {
            "delayedWithinContext": {
                "pairedMeanDifference": late_pooled_mean,
                "familyMeanDifferences": late_family_means,
                "roomMeanDifferences": late_room_means,
                "roomBootstrap95": late_room_bootstrap95,
                "bootstrap": late_bootstrap,
                "inferenceLimit": (
                    "This is delayed within-context transfer after a fixed distractor, not "
                    "durable recall or learning across contexts."
                ),
            }
        },
        "sensitivity": {
            "missingData": {
                "primaryRule": (
                    "Hypothesis-adverse imputation: an interrupted generation session scores "
                    "zero and an interrupted explanation-first control scores one for every "
                    "missing immediate and late item."
                ),
                "interruptionCeiling": {
                    "total": MAX_INTERRUPTED_SESSIONS,
                    "perModelFamily": MAX_INTERRUPTED_SESSIONS_PER_MODEL,
                    "observedByModelFamily": interruption_counts,
                    "met": criteria["interruptionCeilingMet"],
                },
                "completeCase": {
                    "pairsByModelFamily": {
                        model: len(complete_case_differences[model])
                        for model in MODEL_FAMILIES
                    },
                    "familyMeanDifferences": complete_case_family_means,
                    "pairedMeanDifference": complete_case_pooled_mean,
                    "inferenceLimit": (
                        "Descriptive sensitivity only. Removing interrupted pairs changes "
                        "the randomized sample and is not the primary estimand."
                    ),
                },
            },
            "method": (
                "Descriptive paired immediate differences stratified by the frozen "
                "condition collection order and first room. These small subgroups are "
                "not additional hypothesis tests."
            ),
            "conditionCollectionOrder": {
                key: descriptive_group(values)
                for key, values in condition_order_groups.items()
            },
            "firstRoomPosition": {
                room: descriptive_group(values)
                for room, values in first_room_groups.items()
            },
        },
        "publicationAudit": {
            "computedByRunner": False,
            "requiredBeforeMilestoneClaim": True,
            "reason": (
                "Input analysis cannot prove that an event was never omitted. A separate "
                "allocation and ledger reconciliation is required before publication."
            ),
        },
        "pairResults": [
            result for result in pair_results if result["pairId"] in raw_pair_ids
        ],
        "sessionDiagnostics": [
            session_scores[key] for key in sorted(raw_session_ids)
        ],
        "evidenceBoundary": (
            "The computed statistical gate is not the 0.4 milestone by itself. Publication, "
            "independent methodology and math review, and returning-journal acceptance "
            "remain required."
        ),
    }


def analyze_receipts(
    manifest: dict[str, Any],
    bank: dict[str, Any],
    receipts: list[dict[str, Any]],
    anchor: dict[str, Any],
    bootstrap_resamples: int = BOOTSTRAP_RESAMPLES,
) -> dict[str, Any]:
    """Verify collector receipts before running the frozen cohort analysis."""
    return analyze_events(
        manifest,
        bank,
        verify_receipt_anchor(manifest, receipts, anchor),
        bootstrap_resamples=bootstrap_resamples,
    )


def audit_receipts(
    manifest: dict[str, Any],
    bank: dict[str, Any],
    receipts: list[dict[str, Any]],
    anchor: dict[str, Any],
) -> dict[str, Any]:
    """Report complete, incomplete, or reserve-exhausted cohort disposition safely."""
    validate_manifest(manifest, bank)
    events = verify_receipt_anchor(manifest, receipts, anchor)
    pairs, sessions = manifest_indexes(manifest)
    headers: set[str] = set()
    terminals: set[str] = set()
    outcomes: dict[str, str] = {}
    pair_content: set[str] = set()
    recruitment = {model: 0 for model in MODEL_FAMILIES}
    deviations = 0
    for event in events:
        validate_event_shape(event)
        assert_sanitized(event)
        event_type = event["type"]
        session_id = event.get("sessionId")
        if session_id in sessions:
            pair, session = sessions[session_id]
            pair_content.add(pair["pairId"])
            if event_type == "session":
                if session_id in headers:
                    raise StudyError("audit found a duplicate session header")
                validate_session_header(event, pair, session)
                headers.add(session_id)
            elif event_type in ("session_complete", "session_interruption"):
                if session_id in terminals:
                    raise StudyError("audit found a duplicate session terminal")
                terminals.add(session_id)
        if event_type in ("withdrawal", "infrastructure_failure"):
            pair_id = event["pairId"]
            if pair_id not in pairs or pair_id in outcomes:
                raise StudyError("audit found an invalid or duplicate pair outcome")
            outcomes[pair_id] = event_type
        elif event_type == "recruitment_refusal":
            model = event["modelFamily"]
            if event["familyRefusalOrdinal"] != recruitment[model] + 1:
                raise StudyError(
                    "audit found a noncontiguous recruitment refusal ordinal"
                )
            recruitment[model] += 1
        elif event_type == "deviation":
            deviations += 1
            if event["deviationOrdinal"] != deviations:
                raise StudyError("audit found a noncontiguous deviation ordinal")
    if set(outcomes) & pair_content:
        raise StudyError("audit found response content for a terminal pair outcome")
    family_results: dict[str, Any] = {}
    cohort_complete = True
    reserve_exhausted = False
    for model in MODEL_FAMILIES:
        ordered = sorted(
            (pair for pair in manifest["pairs"] if pair["modelFamily"] == model),
            key=lambda pair: pair["order"],
        )
        qualifying = []
        consumed = []
        for pair in ordered:
            pair_id = pair["pairId"]
            session_ids = set(pair["collectionOrder"])
            if pair_id in outcomes:
                consumed.append({"pairId": pair_id, "status": outcomes[pair_id]})
            elif session_ids.issubset(terminals) and session_ids.issubset(headers):
                consumed.append({"pairId": pair_id, "status": "terminal-pair"})
                if len(qualifying) < 10:
                    qualifying.append(pair_id)
            elif pair_id in pair_content:
                consumed.append({"pairId": pair_id, "status": "partial"})
                break
            else:
                break
        complete = len(qualifying) == 10
        exhausted = not complete and len(consumed) == 12
        cohort_complete = cohort_complete and complete
        reserve_exhausted = reserve_exhausted or exhausted
        family_results[model] = {
            "qualifyingPairs": qualifying,
            "qualifyingPairCount": len(qualifying),
            "consumedPairs": consumed,
            "reservesExhausted": exhausted,
        }
    return {
        "schemaVersion": "numinous-understanding-audit-v1",
        "protocolVersion": PROTOCOL_VERSION,
        "runnerVersion": RUNNER_VERSION,
        "allocationSha256": content_sha256(manifest),
        "probeBankSha256": content_sha256(bank),
        "status": (
            "complete"
            if cohort_complete
            else "reserves-exhausted"
            if reserve_exhausted
            else "incomplete"
        ),
        "cohortComplete": cohort_complete,
        "families": family_results,
        "recruitmentRefusals": recruitment,
        "deviationCount": deviations,
        "rawParticipantContentIncluded": False,
    }


def calibrate_bank(
    bank: dict[str, Any],
    records: list[dict[str, Any]],
    relevance_records: list[dict[str, Any]],
    delivery_ledger_sha256: str,
    runner_revision: str,
    runner_source_sha256: str,
) -> dict[str, Any]:
    """Apply frozen calibration rules to complete, provenance-bound observations."""
    validate_bank(bank)
    if not COMMIT_SHA.fullmatch(runner_revision):
        raise StudyError("calibration runner revision is invalid")
    if not SHA256_HEX.fullmatch(runner_source_sha256):
        raise StudyError("calibration runner source commitment is invalid")
    probes = {probe["id"]: probe for probe in bank["probes"]}
    ordered_keys = [
        (probe_id, model, replicate)
        for probe_id in probes
        for model in MODEL_FAMILIES
        for replicate in range(1, CALIBRATION_REPLICATES_PER_MODEL + 1)
    ]
    expected_keys = set(ordered_keys)
    observed: dict[tuple[str, str, int], dict[str, Any]] = {}
    contexts: set[str] = set()
    attempt_start_receipts: set[str] = set()
    dates: set[str] = set()
    backend_revisions: dict[str, set[str]] = {model: set() for model in MODEL_FAMILIES}
    provenance_fields = {
        "probeId",
        "modelFamily",
        "modelIdentifier",
        "replicate",
        "deliveryOrdinal",
        "contextId",
        "backendRevision",
        "reasoningEffort",
        "capabilityPolicy",
        "freshContext",
        "attempt",
        "runnerVersion",
        "runnerRevision",
        "runnerSourceSha256",
        "attemptStartReceiptSha256",
        "date",
    }
    for record in records:
        if not isinstance(record, dict):
            raise StudyError("calibration record must be an object")
        response_fields = set(record) - provenance_fields
        if set(record) not in (
            provenance_fields | {"answer"},
            provenance_fields | {"refuse"},
        ) or (response_fields == {"refuse"} and record.get("refuse") is not True):
            raise StudyError("calibration response shape is invalid")
        replicate = exact_int(
            record.get("replicate"),
            "calibration replicate",
            1,
            CALIBRATION_REPLICATES_PER_MODEL,
        )
        key = (record.get("probeId"), record.get("modelFamily"), replicate)
        if key not in expected_keys or key in observed:
            raise StudyError("calibration response identity is invalid or duplicated")
        expected_ordinal = ordered_keys.index(key) + 1
        context_id = record.get("contextId")
        attempt_start_receipt = record.get("attemptStartReceiptSha256")
        backend_revision = record.get("backendRevision")
        try:
            date.fromisoformat(record.get("date"))
        except (TypeError, ValueError) as error:
            raise StudyError("calibration response date is invalid") from error
        if (
            record.get("modelIdentifier") != key[1]
            or record.get("deliveryOrdinal") != expected_ordinal
            or not isinstance(context_id, str)
            or not SHA256_HEX.fullmatch(context_id)
            or context_id in contexts
            or not isinstance(backend_revision, str)
            or not 1 <= len(backend_revision) <= 256
            or record.get("reasoningEffort") != "high"
            or record.get("capabilityPolicy") != CALIBRATION_CAPABILITY_POLICY
            or record.get("freshContext") is not True
            or record.get("attempt") != 1
            or record.get("runnerVersion") != RUNNER_VERSION
            or record.get("runnerRevision") != runner_revision
            or record.get("runnerSourceSha256") != runner_source_sha256
            or not isinstance(attempt_start_receipt, str)
            or not SHA256_HEX.fullmatch(attempt_start_receipt)
            or attempt_start_receipt in attempt_start_receipts
        ):
            raise StudyError("calibration response provenance differs")
        assert_sanitized(record)
        bounded_canonical_size(record, "calibration response", 4096)
        observed[key] = record
        contexts.add(context_id)
        attempt_start_receipts.add(attempt_start_receipt)
        dates.add(record["date"])
        backend_revisions[key[1]].add(backend_revision)
    missing = expected_keys - set(observed)
    if missing:
        raise StudyError(
            f"calibration stopped early with {len(missing)} response(s) missing"
        )
    if not SHA256_HEX.fullmatch(delivery_ledger_sha256):
        raise StudyError("calibration delivery ledger commitment is invalid")
    if any(len(revisions) != 1 for revisions in backend_revisions.values()):
        raise StudyError(
            "each model family calibration must use exactly one backend revision"
        )

    expected_relevance = {
        (probe_id, reviewer)
        for probe_id in probes
        for reviewer in range(1, CALIBRATION_RELEVANCE_REVIEWERS + 1)
    }
    relevance: dict[tuple[str, int], dict[str, Any]] = {}
    reviewer_ids: dict[int, str] = {}
    for record in relevance_records:
        if not isinstance(record, dict) or set(record) != {
            "probeId",
            "reviewerOrdinal",
            "reviewerId",
            "judgment",
            "rationale",
        }:
            raise StudyError("calibration relevance judgment shape is invalid")
        reviewer = exact_int(
            record.get("reviewerOrdinal"),
            "calibration relevance reviewer",
            1,
            CALIBRATION_RELEVANCE_REVIEWERS,
        )
        key = (record.get("probeId"), reviewer)
        reviewer_id = record.get("reviewerId")
        rationale = record.get("rationale")
        if (
            key not in expected_relevance
            or key in relevance
            or not isinstance(reviewer_id, str)
            or not SHA256_HEX.fullmatch(reviewer_id)
            or (
                reviewer in reviewer_ids
                and reviewer_ids[reviewer] != reviewer_id
            )
            or record.get("judgment")
            not in ("relevant", "irrelevant", "ambiguous")
            or not isinstance(rationale, str)
            or not 12 <= len(rationale) <= 512
        ):
            raise StudyError("calibration relevance judgment is invalid")
        reviewer_ids[reviewer] = reviewer_id
        relevance[key] = record
    if set(relevance) != expected_relevance or len(set(reviewer_ids.values())) != len(
        reviewer_ids
    ):
        raise StudyError("calibration relevance review is incomplete or not independent")

    item_results = []
    replacements = []
    for probe_id, probe in probes.items():
        correct_by_model: dict[str, int] = {}
        ambiguous_count = 0
        for model in MODEL_FAMILIES:
            model_correct = 0
            for replicate in range(1, CALIBRATION_REPLICATES_PER_MODEL + 1):
                response = observed[(probe_id, model, replicate)]
                if response.get("refuse") is True:
                    ambiguous_count += 1
                    continue
                valid, correct = score_answer(probe, response["answer"])
                model_correct += int(correct)
                ambiguous_count += int(not valid)
            correct_by_model[model] = model_correct
        relevance_judgments = [
            relevance[(probe_id, reviewer)]["judgment"]
            for reviewer in range(1, CALIBRATION_RELEVANCE_REVIEWERS + 1)
        ]
        reasons = []
        if any(
            count >= CALIBRATION_MODEL_CEILING_CORRECT_COUNT
            for count in correct_by_model.values()
        ):
            reasons.append("ceiling")
        if ambiguous_count >= CALIBRATION_AMBIGUITY_COUNT:
            reasons.append("ambiguous")
        if any(judgment != "relevant" for judgment in relevance_judgments):
            reasons.append("intervention-irrelevant")
        decision = "replace" if reasons else "retain"
        if decision == "replace":
            replacements.append(probe_id)
        item_results.append(
            {
                "probeId": probe_id,
                "correctByModel": correct_by_model,
                "ambiguousCount": ambiguous_count,
                "relevanceJudgments": relevance_judgments,
                "decision": decision,
                "reasons": reasons,
            }
        )
    ordered_records = [observed[key] for key in ordered_keys]
    ordered_relevance = [relevance[key] for key in sorted(relevance)]
    return {
        "schemaVersion": "numinous-understanding-calibration-audit-v5",
        "probeBankSha256": content_sha256(bank),
        "rules": {
            "replicatesPerModelPerItem": CALIBRATION_REPLICATES_PER_MODEL,
            "modelFamilies": list(MODEL_FAMILIES),
            "replaceAtPerModelCorrectCount": CALIBRATION_MODEL_CEILING_CORRECT_COUNT,
            "replaceAtAmbiguousCount": CALIBRATION_AMBIGUITY_COUNT,
            "relevanceReviewersPerItem": CALIBRATION_RELEVANCE_REVIEWERS,
            "stoppingRule": "collect every frozen item-model-replicate cell exactly once",
            "replacementRule": (
                "replace every flagged item in the same room and phase, assign a new id, "
                "then recalibrate the complete revised bank before allocation"
            ),
        },
        "provenance": {
            "responseRecordCount": len(ordered_records),
            "distinctFreshContextCount": len(contexts),
            "distinctAttemptStartReceiptCount": len(attempt_start_receipts),
            "contextSetSha256": content_sha256(sorted(contexts)),
            "attemptStartReceiptSetSha256": content_sha256(
                sorted(attempt_start_receipts)
            ),
            "responseRecordSetSha256": content_sha256(ordered_records),
            "deliveryLedgerSha256": delivery_ledger_sha256,
            "relevanceRecordSetSha256": content_sha256(ordered_relevance),
            "reviewerIds": [reviewer_ids[index] for index in sorted(reviewer_ids)],
            "backendRevisions": {
                model: sorted(backend_revisions[model]) for model in MODEL_FAMILIES
            },
            "collectionDates": sorted(dates),
            "reasoningEffort": "high",
            "capabilityPolicy": CALIBRATION_CAPABILITY_POLICY,
            "runnerVersion": RUNNER_VERSION,
            "runnerRevision": runner_revision,
            "runnerSourceSha256": runner_source_sha256,
            "oneAttemptPerCell": True,
        },
        "complete": True,
        "passed": not replacements,
        "replacementProbeIds": replacements,
        "items": item_results,
        "rawAnswersIncluded": False,
    }


def validate_calibration_audit(
    bank: dict[str, Any], audit: dict[str, Any]
) -> dict[str, Any]:
    """Require a complete passed audit before allocating any qualifying session."""
    expected_count = (
        len(bank["probes"]) * len(MODEL_FAMILIES) * CALIBRATION_REPLICATES_PER_MODEL
    )
    expected_rules = {
        "replicatesPerModelPerItem": CALIBRATION_REPLICATES_PER_MODEL,
        "modelFamilies": list(MODEL_FAMILIES),
        "replaceAtPerModelCorrectCount": CALIBRATION_MODEL_CEILING_CORRECT_COUNT,
        "replaceAtAmbiguousCount": CALIBRATION_AMBIGUITY_COUNT,
        "relevanceReviewersPerItem": CALIBRATION_RELEVANCE_REVIEWERS,
        "stoppingRule": "collect every frozen item-model-replicate cell exactly once",
        "replacementRule": (
            "replace every flagged item in the same room and phase, assign a new id, "
            "then recalibrate the complete revised bank before allocation"
        ),
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
    if (
        not isinstance(audit, dict)
        or set(audit)
        != {
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
        or audit.get("schemaVersion") != "numinous-understanding-calibration-audit-v5"
        or audit.get("probeBankSha256") != content_sha256(bank)
        or audit.get("complete") is not True
        or audit.get("passed") is not True
        or audit.get("replacementProbeIds") != []
        or audit.get("rawAnswersIncluded") is not False
        or audit.get("rules") != expected_rules
        or not isinstance(audit.get("items"), list)
        or [item.get("probeId") for item in audit["items"]]
        != [probe["id"] for probe in bank["probes"]]
        or any(item.get("decision") != "retain" or item.get("reasons") for item in audit["items"])
        or not isinstance(audit.get("provenance"), dict)
        or set(audit["provenance"]) != provenance_fields
        or audit["provenance"].get("responseRecordCount") != expected_count
        or audit["provenance"].get("distinctFreshContextCount") != expected_count
        or audit["provenance"].get("distinctAttemptStartReceiptCount")
        != expected_count
        or audit["provenance"].get("oneAttemptPerCell") is not True
        or audit["provenance"].get("capabilityPolicy")
        != CALIBRATION_CAPABILITY_POLICY
        or audit["provenance"].get("runnerVersion") != RUNNER_VERSION
        or not isinstance(audit["provenance"].get("runnerRevision"), str)
        or not COMMIT_SHA.fullmatch(audit["provenance"]["runnerRevision"])
        or not isinstance(audit["provenance"].get("runnerSourceSha256"), str)
        or not SHA256_HEX.fullmatch(audit["provenance"]["runnerSourceSha256"])
    ):
        raise StudyError("calibration audit is incomplete, failed, or differs")
    provenance = audit["provenance"]
    for field in (
        "contextSetSha256",
        "attemptStartReceiptSetSha256",
        "responseRecordSetSha256",
        "deliveryLedgerSha256",
        "relevanceRecordSetSha256",
    ):
        if not isinstance(provenance[field], str) or not SHA256_HEX.fullmatch(
            provenance[field]
        ):
            raise StudyError("calibration audit commitment is invalid")
    reviewer_ids = provenance["reviewerIds"]
    if (
        not isinstance(reviewer_ids, list)
        or len(reviewer_ids) != CALIBRATION_RELEVANCE_REVIEWERS
        or any(
            not isinstance(value, str) or not SHA256_HEX.fullmatch(value)
            for value in reviewer_ids
        )
        or len(set(reviewer_ids)) != len(reviewer_ids)
    ):
        raise StudyError("calibration audit reviewer identities are invalid")
    backend_revisions = provenance["backendRevisions"]
    if not isinstance(backend_revisions, dict) or set(backend_revisions) != set(
        MODEL_FAMILIES
    ):
        raise StudyError("calibration audit backend revisions are invalid")
    for revisions in backend_revisions.values():
        if (
            not isinstance(revisions, list)
            or len(revisions) != 1
            or any(
                not isinstance(value, str) or not 1 <= len(value) <= 256
                for value in revisions
            )
            or len(set(revisions)) != len(revisions)
        ):
            raise StudyError("calibration audit backend revisions are invalid")
    collection_dates = provenance["collectionDates"]
    if not isinstance(collection_dates, list) or not collection_dates:
        raise StudyError("calibration audit collection dates are invalid")
    try:
        parsed_dates = [date.fromisoformat(value) for value in collection_dates]
    except (TypeError, ValueError) as error:
        raise StudyError("calibration audit collection dates are invalid") from error
    if len(set(parsed_dates)) != len(parsed_dates) or provenance.get(
        "reasoningEffort"
    ) != "high":
        raise StudyError("calibration audit provenance is invalid")
    for item in audit["items"]:
        if (
            not isinstance(item, dict)
            or set(item)
            != {
                "probeId",
                "correctByModel",
                "ambiguousCount",
                "relevanceJudgments",
                "decision",
                "reasons",
            }
            or not isinstance(item["correctByModel"], dict)
            or set(item["correctByModel"]) != set(MODEL_FAMILIES)
            or any(
                isinstance(count, bool)
                or not isinstance(count, int)
                or not 0 <= count < CALIBRATION_MODEL_CEILING_CORRECT_COUNT
                for count in item["correctByModel"].values()
            )
            or isinstance(item["ambiguousCount"], bool)
            or not isinstance(item["ambiguousCount"], int)
            or not 0 <= item["ambiguousCount"] < CALIBRATION_AMBIGUITY_COUNT
            or item["relevanceJudgments"]
            != ["relevant"] * CALIBRATION_RELEVANCE_REVIEWERS
        ):
            raise StudyError("calibration audit item evidence is invalid")
    assert_sanitized(audit)
    bounded_canonical_size(audit, "calibration audit", 1_000_000)
    return audit


def load_manifest(path: Path, bank: dict[str, Any]) -> dict[str, Any]:
    """Load and validate the exact allocation manifest."""
    manifest = load_json(path)
    if not isinstance(manifest, dict):
        raise StudyError("allocation manifest must be a JSON object")
    return validate_manifest(manifest, bank)


def command_allocate(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
    calibration_audit = load_json(args.calibration_audit)
    if not isinstance(calibration_audit, dict):
        raise StudyError("calibration audit must be an object")
    manifest = build_allocation(bank, calibration_audit)
    result = write_json_once(args.output, manifest)
    print(f"{result} {args.output}")
    print(f"allocation sha256 {content_sha256(manifest)}")


def command_validate(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
    manifest = load_manifest(args.manifest, bank)
    print(f"probe bank PASS {content_sha256(bank)}")
    print(f"allocation PASS {content_sha256(manifest)}")


def command_calibrate(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
    runner_revision, runner_source_sha256 = current_study_source_identity()
    records, delivery_ledger_sha256 = calibration_response_records(
        bank,
        read_receipt_jsonl(args.responses),
        load_json(args.anchor),
        runner_revision,
        runner_source_sha256,
    )
    relevance_records = [
        {key: value for key, value in record.items() if key != "_sourceIndex"}
        for record in read_jsonl(args.relevance)
    ]
    report = calibrate_bank(
        bank,
        records,
        relevance_records,
        delivery_ledger_sha256,
        runner_revision,
        runner_source_sha256,
    )
    result = write_json_once(args.output, report)
    print(f"{result} {args.output}")
    print("calibration " + ("PASS" if report["passed"] else "REPLACE"))


def command_session(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
    manifest = load_manifest(args.manifest, bank)
    print(
        json.dumps(session_packet(manifest, args.session_id), indent=2, sort_keys=True)
    )


def command_probe(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
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
    bank = load_bank(args.bank)
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
    bank = load_bank(args.bank)
    manifest = load_manifest(args.manifest, bank)
    receipts = read_receipt_jsonl(args.responses)
    report = analyze_receipts(manifest, bank, receipts, load_json(args.anchor))
    result = write_json_once(args.output, report)
    print(f"{result} {args.output}")
    print(
        "predeclared statistical gate "
        + ("PASS" if report["primary"]["predeclaredStatisticalGatePassed"] else "FAIL")
    )


def command_audit(args: argparse.Namespace) -> None:
    bank = load_bank(args.bank)
    manifest = load_manifest(args.manifest, bank)
    report = audit_receipts(
        manifest,
        bank,
        read_receipt_jsonl(args.responses),
        load_json(args.anchor),
    )
    result = write_json_once(args.output, report)
    print(f"{result} {args.output}")
    print(f"cohort audit {report['status'].upper()}")


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line contract."""
    parser = argparse.ArgumentParser(
        description=(
            "Freeze and analyze the 0.4 Understanding Alpha study. The runner never calls a "
            "model and refuses to report an incomplete cohort."
        )
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    allocate = subparsers.add_parser(
        "allocate", help="write the exact 24-pair manifest"
    )
    allocate.add_argument("--bank", type=Path, required=True)
    allocate.add_argument("--calibration-audit", type=Path, required=True)
    allocate.add_argument("--output", type=Path, required=True)
    allocate.set_defaults(handler=command_allocate)

    validate = subparsers.add_parser(
        "validate", help="validate a manifest and probe bank"
    )
    validate.add_argument("--bank", type=Path, required=True)
    validate.add_argument("--manifest", type=Path, required=True)
    validate.set_defaults(handler=command_validate)

    calibrate = subparsers.add_parser(
        "calibrate", help="apply the frozen private item calibration rules"
    )
    calibrate.add_argument("--bank", type=Path, required=True)
    calibrate.add_argument("--responses", type=Path, required=True)
    calibrate.add_argument("--anchor", type=Path, required=True)
    calibrate.add_argument("--relevance", type=Path, required=True)
    calibrate.add_argument("--output", type=Path, required=True)
    calibrate.set_defaults(handler=command_calibrate)

    session = subparsers.add_parser("session", help="emit a public condition packet")
    session.add_argument("--bank", type=Path, required=True)
    session.add_argument("--manifest", type=Path, required=True)
    session.add_argument("--session-id", required=True)
    session.set_defaults(handler=command_session)

    probe = subparsers.add_parser("probe", help="emit one held-out public probe")
    probe.add_argument("--bank", type=Path, required=True)
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
    distractor.add_argument("--bank", type=Path, required=True)
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
    analyze.add_argument("--bank", type=Path, required=True)
    analyze.add_argument("--manifest", type=Path, required=True)
    analyze.add_argument("--responses", type=Path, required=True)
    analyze.add_argument("--anchor", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    analyze.set_defaults(handler=command_analyze)

    audit = subparsers.add_parser(
        "audit", help="report cohort disposition even when analysis is unavailable"
    )
    audit.add_argument("--bank", type=Path, required=True)
    audit.add_argument("--manifest", type=Path, required=True)
    audit.add_argument("--responses", type=Path, required=True)
    audit.add_argument("--anchor", type=Path, required=True)
    audit.add_argument("--output", type=Path, required=True)
    audit.set_defaults(handler=command_audit)
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
