#!/usr/bin/env python3
"""Regression tests for the frozen Understanding Alpha study runner."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "scripts" / "understanding-study.py"
sys.dont_write_bytecode = True


def load_runner():
    spec = importlib.util.spec_from_file_location(
        "numinous_understanding_study", RUNNER
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load understanding-study.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


study = load_runner()
RUNNER_REVISION = "1" * 40
RUNNER_SOURCE_SHA256 = "2" * 64


def initialize_repository(root: Path) -> str:
    """Create one committed source fixture and return its full revision."""
    (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
    commands = (
        ["git", "init", "--quiet"],
        ["git", "add", "Cargo.toml"],
        [
            "git",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "Initialize fixture",
        ],
    )
    for command in commands:
        subprocess.run(command, cwd=root, check=True, capture_output=True)
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def binary_build_receipt() -> dict:
    return {
        "schemaVersion": study.MCP_BUILD_RECEIPT_SCHEMA,
        "sourceRevision": "1" * 40,
        "studySourceSha256": RUNNER_SOURCE_SHA256,
        "sourcePolicy": "verified-clean-commit-before-and-after",
        "environmentPolicy": "bounded-inheritance-no-build-overrides-v1",
        "cargoVersion": "cargo 1.92.0 fixture",
        "rustcVersion": "rustc 1.92.0 fixture",
        "targetTriple": "fixture-target",
        "profile": "debug",
        "features": "none",
        "locked": True,
        "incremental": False,
        "targetDirectoryPolicy": "fresh-explicit-private",
        "artifactPolicy": "cargo-json-private-copy-hash-before-and-after-execution",
        "binarySha256": "2" * 64,
    }


def wrong_answer(probe: dict) -> int | float | str:
    schema = probe["answerSchema"]
    expected = study.oracle_answer(probe["oracle"])
    if schema["type"] == "number":
        return float(expected) + 1.0
    return next(choice for choice in schema["enum"] if choice != expected)


def calibration_inputs(bank: dict) -> tuple[list[dict], list[dict]]:
    records = []
    ordinal = 0
    for probe in bank["probes"]:
        for model in study.MODEL_FAMILIES:
            for replicate in range(1, study.CALIBRATION_REPLICATES_PER_MODEL + 1):
                ordinal += 1
                records.append(
                    {
                        "probeId": probe["id"],
                        "modelFamily": model,
                        "modelIdentifier": model,
                        "replicate": replicate,
                        "deliveryOrdinal": ordinal,
                        "contextId": study.content_sha256(
                            f"fixture:{probe['id']}:{model}:{replicate}"
                        ),
                        "backendRevision": "revision-1",
                        "reasoningEffort": "high",
                        "capabilityPolicy": study.CALIBRATION_CAPABILITY_POLICY,
                        "freshContext": True,
                        "attempt": 1,
                        "runnerVersion": study.RUNNER_VERSION,
                        "runnerRevision": RUNNER_REVISION,
                        "runnerSourceSha256": RUNNER_SOURCE_SHA256,
                        "attemptStartReceiptSha256": study.content_sha256(
                            f"fixture start receipt:{ordinal}"
                        ),
                        "date": "2026-07-31",
                        "answer": wrong_answer(probe),
                    }
                )
    reviewer_ids = (
        study.content_sha256("fixture relevance reviewer one"),
        study.content_sha256("fixture relevance reviewer two"),
    )
    relevance = [
        {
            "probeId": probe["id"],
            "reviewerOrdinal": reviewer,
            "reviewerId": reviewer_ids[reviewer - 1],
            "judgment": "relevant",
            "rationale": "The required intervention changes the state needed by this probe.",
        }
        for probe in bank["probes"]
        for reviewer in range(1, study.CALIBRATION_RELEVANCE_REVIEWERS + 1)
    ]
    return records, relevance


def passing_calibration_audit(bank: dict) -> dict:
    records, relevance = calibration_inputs(bank)
    return study.calibrate_bank(
        bank,
        records,
        relevance,
        "5" * 64,
        RUNNER_REVISION,
        RUNNER_SOURCE_SHA256,
    )


def session_records(
    bank: dict,
    pair: dict,
    session: dict,
    correct: bool,
) -> list[dict]:
    session_id = session["sessionId"]
    condition = session["condition"]
    records = [
        {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session",
            "sessionId": session_id,
            "consent": True,
            "publicationConsent": "bounded-raw",
            "modelFamily": pair["modelFamily"],
            "modelIdentifier": pair["modelFamily"],
            "provider": "OpenAI",
            "backendRevision": "revision-1",
            "reasoningEffort": "high",
            "settings": {"sampling": "platform-default", "freshContext": True},
            "date": "2026-07-27",
            "numinousCommit": "1" * 40,
            "mcpProtocolRevision": study.MCP_PROTOCOL_REVISION,
            "operatingSystem": "test-os",
            "runnerVersion": study.RUNNER_VERSION,
            "studySourceSha256": RUNNER_SOURCE_SHA256,
            "attemptStartReceiptSha256": study.content_sha256(
                f"{session_id}-start-receipt"
            ),
            "condition": condition,
            "contextId": study.content_sha256(f"{session_id}-fresh-context"),
            "capabilityPolicy": "collector-only-no-repository-web-or-tools",
        }
    ]
    room_specs = study.encounter_rooms()

    def append_tool(room: str, sequence: int, call: dict) -> None:
        role = call["role"]
        if role == "reveal":
            structured = {"reveal": "same public Reveal"}
        elif call["tool"] == "plot_expression":
            structured = {"expression": call["arguments"]["expr"]}
        elif role == "interaction":
            structured = {
                "status": room_specs[room]["feedbackEvidence"]["contains"]
            }
        else:
            structured = {"ok": True}
        records.append(
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "tool",
                "sessionId": session_id,
                "room": room,
                "sequence": sequence,
                "role": role,
                "tool": call["tool"],
                "arguments": call["arguments"],
                "structuredResult": structured,
                "visibleText": f"public {room} {role}",
                "toolOutcome": "success",
                "binarySha256": "2" * 64,
                "binaryBuildReceipt": binary_build_receipt(),
            }
        )

    def append_response(room: str, stage: str) -> None:
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "condition_response",
            "sessionId": session_id,
            "room": room,
            "stage": stage,
        }
        if stage in ("prediction", "construction"):
            event.update(
                {
                    "answer": "sin(4*x)"
                    if room == "formula-jam"
                    else room_specs[room]["expectedAnswer"],
                    "rationale": "A bounded rationale for this committed answer.",
                }
            )
        else:
            event["text"] = f"A bounded public {stage} response."
        records.append(event)

    def append_feedback(room: str, participant_correct: bool | None) -> None:
        records.append(
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "feedback",
                "sessionId": session_id,
                "room": room,
                "expectedAnswer": room_specs[room]["expectedAnswer"],
                "participantCorrect": participant_correct,
                "text": room_specs[room]["feedbackText"],
            }
        )

    def append_material(room: str) -> None:
        text = room_specs[room]["revealMaterial"]
        records.append(
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "material",
                "sessionId": session_id,
                "room": room,
                "kind": "reveal",
                "text": text,
                "materialSha256": study.content_sha256(text),
            }
        )

    for room in pair["roomOrder"]:
        calls = study.condition_calls(room_specs[room], condition)
        if condition == study.CONDITIONS[0]:
            append_tool(room, 1, calls[0])
            append_response(
                room, "construction" if room == "formula-jam" else "prediction"
            )
            interaction = calls[1]
            if room == "formula-jam":
                interaction = {**interaction, "arguments": {"expr": "sin(4*x)"}}
            append_tool(room, 2, interaction)
            append_feedback(room, True)
            for sequence, call in enumerate(calls[2:], start=3):
                append_tool(room, sequence, call)
            if room == "formula-jam":
                append_material(room)
        else:
            append_tool(room, 1, calls[0])
            if room == "formula-jam":
                append_material(room)
                append_response(room, "elaboration")
                interaction_index = 1
            else:
                append_tool(room, 2, calls[1])
                append_response(room, "elaboration")
                interaction_index = 2
            interaction_sequence = interaction_index + 1
            append_tool(room, interaction_sequence, calls[interaction_index])
            append_feedback(room, None)
            for sequence, call in enumerate(
                calls[interaction_index + 1 :], start=interaction_sequence + 1
            ):
                append_tool(room, sequence, call)
    for phase in ("immediate", "late"):
        if phase == "late":
            for item in bank["distractorSequence"]["items"]:
                records.append(
                    {
                        "schemaVersion": study.EVENT_SCHEMA,
                        "type": "distractor_response",
                        "sessionId": session_id,
                        "itemId": item["id"],
                        "answer": "fixture",
                    }
                )
        for probe in study.probe_sequence(bank, pair["roomOrder"], phase):
            answer = (
                study.oracle_answer(probe["oracle"]) if correct else wrong_answer(probe)
            )
            records.append(
                {
                    "schemaVersion": study.EVENT_SCHEMA,
                    "type": "response",
                    "sessionId": session_id,
                    "phase": phase,
                    "probeId": probe["id"],
                    "attempt": 1,
                    "answer": answer,
                }
            )
    records.append(
        {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_complete",
            "sessionId": session_id,
        }
    )
    return records


def cohort_records(
    bank: dict,
    manifest: dict,
    selected_orders: dict[str, range] | None = None,
) -> list[dict]:
    selected_orders = selected_orders or {
        "gpt-5.6-sol": range(1, 11),
        "gpt-5.6-terra": range(1, 11),
    }
    records: list[dict] = []
    for pair in manifest["pairs"]:
        if pair["order"] not in selected_orders[pair["modelFamily"]]:
            continue
        sessions_by_id = {session["sessionId"]: session for session in pair["sessions"]}
        for session_id in pair["collectionOrder"]:
            session = sessions_by_id[session_id]
            records.extend(
                session_records(
                    bank,
                    pair,
                    session,
                    correct=session["condition"] == study.CONDITIONS[0],
                )
            )
    for index, record in enumerate(records):
        record["_sourceIndex"] = index
    return records


class ProbeBankTests(unittest.TestCase):
    def setUp(self) -> None:
        self.bank = study.load_bank(study.FIXTURE_PROBE_BANK_PATH)
        self.manifest = study.build_allocation(
            self.bank, passing_calibration_audit(self.bank)
        )

    def test_frozen_hashes_and_balanced_allocation(self) -> None:
        self.assertRegex(study.content_sha256(self.bank), r"^[0-9a-f]{64}$")
        self.assertRegex(study.content_sha256(self.manifest), r"^[0-9a-f]{64}$")
        self.assertEqual(len(self.manifest["pairs"]), 24)
        for model in study.MODEL_FAMILIES:
            pairs = [
                pair for pair in self.manifest["pairs"] if pair["modelFamily"] == model
            ]
            self.assertEqual(
                sum(pair["allocationRole"] == "primary" for pair in pairs), 10
            )
            self.assertEqual(
                sum(pair["allocationRole"] == "reserve" for pair in pairs), 2
            )
            primary = [pair for pair in pairs if pair["allocationRole"] == "primary"]
            first_conditions = [
                next(
                    session["condition"]
                    for session in pair["sessions"]
                    if session["sessionId"] == pair["collectionOrder"][0]
                )
                for pair in primary
            ]
            self.assertEqual(first_conditions.count(study.CONDITIONS[0]), 5)
            self.assertEqual(first_conditions.count(study.CONDITIONS[1]), 5)
            room_counts = {
                room: sum(pair["roomOrder"][0] == room for pair in primary)
                for room in study.ROOMS
            }
            self.assertEqual(set(room_counts.values()), {2})

    def test_study_source_identity_binds_every_declared_git_object(self) -> None:
        expected_objects = {
            relative: f"{index:040x}"
            for index, relative in enumerate(study.STUDY_SOURCE_OBJECTS, start=1)
        }
        with mock.patch.object(
            study.source_integrity,
            "verify_source_tree",
            return_value=(RUNNER_REVISION, expected_objects),
        ) as verify:
            identity = study.study_source_identity(RUNNER_REVISION)
        self.assertEqual(identity, study.content_sha256(expected_objects))
        verify.assert_called_once_with(
            study.ROOT,
            study.STUDY_SOURCE_OBJECTS,
            expected_revision=RUNNER_REVISION,
            whole_worktree_clean=True,
        )

    def test_repository_identity_rejects_a_dirty_tracked_worktree(self) -> None:
        with (
            mock.patch.object(
                study.source_integrity,
                "verify_source_tree",
                side_effect=study.source_integrity.SourceIntegrityError(
                    "qualifying runtime source worktree is dirty"
                ),
            ),
            self.assertRaisesRegex(study.StudyError, "worktree is dirty"),
        ):
            study.repository_commit()

    def test_repository_identity_rejects_ignored_runtime_source(self) -> None:
        with (
            mock.patch.object(
                study.source_integrity,
                "verify_source_tree",
                side_effect=study.source_integrity.SourceIntegrityError(
                    "qualifying runtime source contains ignored files"
                ),
            ),
            self.assertRaisesRegex(study.StudyError, "contains ignored files"),
        ):
            study.repository_commit()

    def test_repository_identity_rejects_hidden_index_changes(self) -> None:
        for flag in ("--assume-unchanged", "--skip-worktree"):
            with self.subTest(flag=flag), tempfile.TemporaryDirectory() as temporary:
                repository = Path(temporary)
                initialize_repository(repository)
                subprocess.run(
                    ["git", "update-index", flag, "Cargo.toml"],
                    cwd=repository,
                    check=True,
                    capture_output=True,
                )
                (repository / "Cargo.toml").write_text(
                    "[workspace]\nmembers = []\n", encoding="utf-8"
                )
                with (
                    mock.patch.object(study, "ROOT", repository),
                    mock.patch.object(
                        study, "STUDY_SOURCE_OBJECTS", ("Cargo.toml",)
                    ),
                    self.assertRaisesRegex(
                        study.StudyError, "nonordinary index flags"
                    ),
                ):
                    study.repository_commit()

    def test_repository_identity_compares_actual_bytes_beyond_git_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            initialize_repository(repository)
            (repository / "Cargo.toml").write_text(
                "[workspace]\nmembers = []\n", encoding="utf-8"
            )
            real_git = study.source_integrity._git

            def incomplete_git(
                root: Path,
                arguments: list[str],
                environment: dict[str, str] | None,
            ) -> bytes:
                if arguments[0] == "status":
                    return b""
                if arguments[:2] == ["ls-files", "-v"]:
                    return b"H Cargo.toml\0"
                return real_git(root, arguments, environment)

            with (
                mock.patch.object(study, "ROOT", repository),
                mock.patch.object(study, "STUDY_SOURCE_OBJECTS", ("Cargo.toml",)),
                mock.patch.object(
                    study.source_integrity, "_git", side_effect=incomplete_git
                ),
                self.assertRaisesRegex(study.StudyError, "differs from commit"),
            ):
                study.repository_commit()

    def test_repository_identity_rejects_redirected_git_environment(self) -> None:
        with (
            tempfile.TemporaryDirectory() as actual_temporary,
            tempfile.TemporaryDirectory() as alternate_temporary,
        ):
            actual = Path(actual_temporary)
            alternate = Path(alternate_temporary)
            initialize_repository(actual)
            initialize_repository(alternate)
            (actual / "Cargo.toml").write_text(
                "[workspace]\nmembers = []\n", encoding="utf-8"
            )
            hostile = {
                "GIT_DIR": str(alternate / ".git"),
                "GIT_WORK_TREE": str(alternate),
            }
            with (
                mock.patch.object(study, "ROOT", actual),
                mock.patch.object(study, "STUDY_SOURCE_OBJECTS", ("Cargo.toml",)),
                mock.patch.dict(os.environ, hostile),
                self.assertRaisesRegex(study.StudyError, "worktree is dirty"),
            ):
                study.repository_commit()

    def test_every_allowed_reserve_path_has_bounded_order_and_room_imbalance(
        self,
    ) -> None:
        for model in study.MODEL_FAMILIES:
            pairs = sorted(
                (
                    pair
                    for pair in self.manifest["pairs"]
                    if pair["modelFamily"] == model
                ),
                key=lambda pair: pair["order"],
            )
            for first_missing in range(-1, 11):
                for second_missing in range(first_missing + 1, 12):
                    missing = {
                        index for index in (first_missing, second_missing) if index >= 0
                    }
                    selected = [
                        pair for index, pair in enumerate(pairs) if index not in missing
                    ][:10]
                    if len(selected) != 10:
                        continue
                    starts = [
                        next(
                            session["condition"]
                            for session in pair["sessions"]
                            if session["sessionId"] == pair["collectionOrder"][0]
                        )
                        for pair in selected
                    ]
                    condition_gap = abs(
                        starts.count(study.CONDITIONS[0])
                        - starts.count(study.CONDITIONS[1])
                    )
                    room_counts = [
                        sum(pair["roomOrder"][0] == room for pair in selected)
                        for room in study.ROOMS
                    ]
                    self.assertLessEqual(condition_gap, 2)
                    self.assertLessEqual(
                        max(room_counts) - min(room_counts),
                        self.manifest["maximumReserveFirstRoomCountRange"],
                    )

    def test_public_packets_never_expose_oracles(self) -> None:
        packet = study.public_probe(self.bank["probes"][0], schema_only=False)
        repair = study.public_probe(self.bank["probes"][0], schema_only=True)
        self.assertNotIn("oracle", json.dumps(packet))
        self.assertNotIn("prompt", repair)
        self.assertEqual(set(repair), {"schemaVersion", "probeId", "answerSchema"})

    def test_answer_schema_cannot_smuggle_an_oracle(self) -> None:
        changed = json.loads(json.dumps(self.bank))
        changed["probes"][0]["answerSchema"]["correctAnswer"] = 4
        with self.assertRaisesRegex(study.StudyError, "answer schema fields"):
            study.validate_bank(changed)

    def test_manifest_mutation_is_rejected(self) -> None:
        changed = json.loads(json.dumps(self.manifest))
        changed["pairs"][0]["roomOrder"].reverse()
        with self.assertRaisesRegex(study.StudyError, "differs from the frozen"):
            study.validate_manifest(changed, self.bank)

    def test_encounter_role_swap_is_rejected(self) -> None:
        spec = json.loads(json.dumps(study.load_encounter_spec()))
        spec["rooms"][0]["calls"][1]["role"] = "continuation"
        with self.assertRaisesRegex(study.StudyError, "roles or order"):
            study.validate_encounter_spec(spec)

    def test_calibration_rules_are_complete_and_deterministic(self) -> None:
        records, relevance = calibration_inputs(self.bank)
        audit = study.calibrate_bank(
            self.bank,
            records,
            relevance,
            "5" * 64,
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
        )
        self.assertTrue(audit["passed"])
        self.assertEqual(
            audit["provenance"]["distinctFreshContextCount"], len(records)
        )
        altered_rules = json.loads(json.dumps(audit))
        altered_rules["rules"]["replaceAtPerModelCorrectCount"] = 3
        with self.assertRaisesRegex(
            study.StudyError, "incomplete, failed, or differs"
        ):
            study.validate_calibration_audit(self.bank, altered_rules)
        malformed_reviewer = json.loads(json.dumps(audit))
        malformed_reviewer["provenance"]["reviewerIds"][0] = {}
        with self.assertRaisesRegex(study.StudyError, "reviewer identities"):
            study.validate_calibration_audit(self.bank, malformed_reviewer)
        for field, replacement in (
            ("runnerRevision", "f" * 39),
            ("runnerSourceSha256", "f" * 63),
        ):
            altered_source = json.loads(json.dumps(audit))
            altered_source["provenance"][field] = replacement
            with self.subTest(field=field), self.assertRaisesRegex(
                study.StudyError, "incomplete, failed, or differs"
            ):
                study.validate_calibration_audit(self.bank, altered_source)
        first_probe = self.bank["probes"][0]
        for record in records:
            if (
                record["probeId"] == first_probe["id"]
                and record["modelFamily"] == study.MODEL_FAMILIES[0]
            ):
                record["answer"] = study.oracle_answer(first_probe["oracle"])
        audit = study.calibrate_bank(
            self.bank,
            records,
            relevance,
            "5" * 64,
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
        )
        self.assertFalse(audit["passed"])
        self.assertEqual(audit["replacementProbeIds"], [first_probe["id"]])
        with self.assertRaisesRegex(study.StudyError, "stopped early"):
            study.calibrate_bank(
                self.bank,
                records[:-1],
                relevance,
                "5" * 64,
                RUNNER_REVISION,
                RUNNER_SOURCE_SHA256,
            )

        records, relevance = calibration_inputs(self.bank)
        relevance[0]["judgment"] = "irrelevant"
        audit = study.calibrate_bank(
            self.bank,
            records,
            relevance,
            "5" * 64,
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
        )
        self.assertFalse(audit["passed"])
        self.assertIn("intervention-irrelevant", audit["items"][0]["reasons"])

        records, relevance = calibration_inputs(self.bank)
        records[1]["contextId"] = records[0]["contextId"]
        with self.assertRaisesRegex(study.StudyError, "provenance differs"):
            study.calibrate_bank(
                self.bank,
                records,
                relevance,
                "5" * 64,
                RUNNER_REVISION,
                RUNNER_SOURCE_SHA256,
            )

        records, relevance = calibration_inputs(self.bank)
        records[0]["backendRevision"] = "different-revision"
        with self.assertRaisesRegex(study.StudyError, "exactly one backend revision"):
            study.calibrate_bank(
                self.bank,
                records,
                relevance,
                "5" * 64,
                RUNNER_REVISION,
                RUNNER_SOURCE_SHA256,
            )

        records, relevance = calibration_inputs(self.bank)
        records[1]["attemptStartReceiptSha256"] = records[0][
            "attemptStartReceiptSha256"
        ]
        with self.assertRaisesRegex(study.StudyError, "provenance differs"):
            study.calibrate_bank(
                self.bank,
                records,
                relevance,
                "5" * 64,
                RUNNER_REVISION,
                RUNNER_SOURCE_SHA256,
            )

    def test_complete_calibration_receipt_chain_drives_the_audit(self) -> None:
        records, relevance = calibration_inputs(self.bank)
        commitment = study.calibration_receipt_commitment(
            self.bank, RUNNER_REVISION, RUNNER_SOURCE_SHA256
        )
        events = []
        for cell, record in zip(commitment["cells"], records, strict=True):
            delivery = {
                "schemaVersion": study.CALIBRATION_EVENT_SCHEMA,
                "type": "calibration_delivery",
                **cell,
                **{
                    key: record[key]
                    for key in (
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
                },
            }
            delivery["requestId"] = study.calibration_request_id(
                commitment, delivery
            )
            events.extend(
                (
                    delivery,
                    {
                        "schemaVersion": study.CALIBRATION_EVENT_SCHEMA,
                        "type": "calibration_response",
                        "deliveryOrdinal": delivery["deliveryOrdinal"],
                        "requestId": delivery["requestId"],
                        "answer": record["answer"],
                    },
                )
            )
        receipts = study.seal_records(commitment, events)
        anchor = study.build_receipt_anchor(commitment, receipts)
        extracted, ledger_sha256 = study.calibration_response_records(
            self.bank,
            receipts,
            anchor,
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
        )
        self.assertEqual(extracted, records)
        self.assertEqual(ledger_sha256, study.content_sha256(receipts))
        audit = study.calibrate_bank(
            self.bank,
            extracted,
            relevance,
            ledger_sha256,
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
        )
        self.assertTrue(audit["passed"])
        self.assertEqual(
            audit["provenance"]["deliveryLedgerSha256"], ledger_sha256
        )

    def test_independent_oracles_cover_every_probe(self) -> None:
        for probe in self.bank["probes"]:
            expected = study.oracle_answer(probe["oracle"])
            valid, correct = study.score_answer(probe, expected)
            self.assertTrue(valid, probe["id"])
            self.assertTrue(correct, probe["id"])

    def test_numeric_answers_accept_exact_rationals_and_reasonable_decimals(
        self,
    ) -> None:
        probe = next(
            probe
            for probe in self.bank["probes"]
            if probe["id"] == "formula-immediate-2"
        )
        self.assertEqual(study.score_answer(probe, "1/3"), (True, True))
        self.assertEqual(study.score_answer(probe, 0.333333333333), (True, True))
        self.assertEqual(study.score_answer(probe, "1/0"), (False, False))


class RedactionTests(unittest.TestCase):
    def test_write_once_never_replaces_a_concurrent_winner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            barrier = threading.Barrier(3)
            outcomes: list[tuple[str, str]] = []

            def writer(text: str) -> None:
                barrier.wait()
                try:
                    outcomes.append((text, study.write_text_once(path, text)))
                except study.StudyError:
                    outcomes.append((text, "rejected"))

            workers = [
                threading.Thread(target=writer, args=("first",)),
                threading.Thread(target=writer, args=("second",)),
            ]
            for worker in workers:
                worker.start()
            barrier.wait()
            for worker in workers:
                worker.join()
            self.assertIn(path.read_text(encoding="utf-8"), {"first", "second"})
            self.assertEqual(sum(result == "written" for _text, result in outcomes), 1)
            self.assertEqual(sum(result == "rejected" for _text, result in outcomes), 1)

    def test_write_once_translates_publication_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            with mock.patch.object(
                study.os, "link", side_effect=PermissionError("denied")
            ):
                with self.assertRaisesRegex(study.StudyError, "publish"):
                    study.write_text_once(path, "content")

    def test_write_once_rejects_symlink_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target.json"
            target.write_text("content", encoding="utf-8")
            link = root / "link.json"
            try:
                link.symlink_to(target)
            except OSError:
                self.skipTest("symlinks are unavailable on this host")
            with self.assertRaisesRegex(study.StudyError, "symlink"):
                study.write_text_once(link, "content")

    def test_redaction_removes_private_fields_paths_and_host_values(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = root / "raw.jsonl"
            clean = root / "clean.jsonl"
            raw.write_text(
                json.dumps(
                    {
                        "schemaVersion": study.EVENT_SCHEMA,
                        "type": "tool",
                        "systemPrompt": "private",
                        "token": "secret",
                        "visibleText": "PrivateName C:\\Users\\PrivateName\\journal.txt",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            result = study.redact_jsonl(raw, clean, ("PrivateName",))
            self.assertEqual(result, "written")
            record = json.loads(clean.read_text(encoding="utf-8"))
            self.assertNotIn("systemPrompt", record)
            self.assertNotIn("token", record)
            self.assertIn("<ABSOLUTE_PATH>", record["visibleText"])
            self.assertIn("<HOST_IDENTIFIER>", record["visibleText"])
            study.assert_sanitized(record)

    def test_analysis_rejects_unsanitized_absolute_path(self) -> None:
        with self.assertRaisesRegex(study.StudyError, "absolute host path"):
            study.assert_sanitized({"visibleText": "/home/person/private.txt"})

    def test_analysis_rejects_identity_and_network_values(self) -> None:
        for value in (
            "person@example.com",
            "connect to 192.0.2.8",
            "Bearer abcdefghijklmnop",
            "Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==",
            "token=abcdefghijklmnopqrstuvwxyz",
        ):
            with self.subTest(value=value):
                with self.assertRaises(study.StudyError):
                    study.assert_sanitized({"visibleText": value})

    def test_analysis_rejects_private_assignments_and_known_secret_prefixes(self) -> None:
        values = (
            "password=hunter2longpassword",
            "password: hunter2longpassword",
            "client_secret=abcdefghijklmnop",
            "authorization=abcdefghijklmnop",
            "cookie=sessionidabcdefghijklmnop",
            "credential: abcdefghijklmnop",
            "access_token=abcdefghijklmnop",
            "username=nickseal",
            "user_id: 123456789",
            "accountId=acct_123456789",
            "hostname: NICKS-LAPTOP",
            "host_identifier=nicks-machine",
            "system_prompt=private instructions",
            "privatePrompt: private instructions",
            "chain_of_thought=private reasoning",
            "hiddenReasoning: private reasoning",
            "sk-proj-abcdefghijklmnop",
            "ghp_abcdefghijklmnop",
            "xoxb-abcdefghijklmnop",
            "AKIAABCDEFGHIJKLMNOP",
        )
        for value in values:
            with self.subTest(value=value):
                with self.assertRaisesRegex(study.StudyError, "private"):
                    study.assert_sanitized({"visibleText": value})
                clean, removed = study.redact_value(value, ())
                self.assertGreater(removed, 0)
                self.assertNotIn(value.split("=", 1)[-1].split(":", 1)[-1], clean)
                study.assert_sanitized(clean)

    def test_strict_json_rejects_duplicate_keys_nesting_and_unbounded_jsonl(self) -> None:
        with self.assertRaisesRegex(study.StudyError, "duplicate object key"):
            study.strict_json_loads('{"outer":{"type":1,"type":2}}', "test")
        nested = "[" * 40 + "0" + "]" * 40
        with self.assertRaisesRegex(study.StudyError, "nesting limit"):
            study.strict_json_loads(nested, "test")
        deep_value: object = 0
        for _index in range(2_000):
            deep_value = [deep_value]
        with self.assertRaisesRegex(study.StudyError, "canonical finite JSON"):
            study.canonical_bytes(deep_value)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "records.jsonl"
            path.write_text('{}\n{}\n', encoding="utf-8")
            with mock.patch.object(study, "MAX_JSONL_RECORDS", 1):
                with self.assertRaisesRegex(study.StudyError, "record limit"):
                    study.read_jsonl(path)
            with mock.patch.object(study, "MAX_JSONL_TOTAL_BYTES", 2):
                with self.assertRaisesRegex(study.StudyError, "total-byte limit"):
                    study.read_jsonl(path)
            path.write_bytes(b"{" + b"x" * 32)
            with mock.patch.object(study, "MAX_JSONL_LINE_BYTES", 8):
                with self.assertRaisesRegex(study.StudyError, "line limit"):
                    study.read_jsonl(path)

    def test_receipt_reader_rejects_duplicate_keys_and_boolean_index(self) -> None:
        bank = study.load_bank(study.FIXTURE_PROBE_BANK_PATH)
        manifest = study.build_allocation(bank, passing_calibration_audit(bank))
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "recruitment_refusal",
            "modelFamily": study.MODEL_FAMILIES[0],
            "familyRefusalOrdinal": 1,
        }
        receipt = study.seal_records(manifest, [event])[0]
        changed = dict(receipt)
        changed["receiptIndex"] = False
        with self.assertRaisesRegex(study.StudyError, "index"):
            study.verify_receipts(manifest, [changed])
        raw = json.dumps(receipt, separators=(",", ":"))
        duplicate = raw[:-1] + ',"schemaVersion":"duplicate"}'
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"
            path.write_text(duplicate + "\n", encoding="utf-8")
            with self.assertRaisesRegex(study.StudyError, "duplicate object key"):
                study.read_receipt_jsonl(path)

    def test_analysis_rejects_credential_field_variants(self) -> None:
        for key in ("accountId", "apiKey", "api_key", "secret", "accessKeyId"):
            with self.subTest(key=key):
                with self.assertRaises(study.StudyError):
                    study.assert_sanitized({"arguments": {key: "credential-value"}})

    def test_private_key_rules_preserve_unrelated_public_fields(self) -> None:
        value = {
            "unaffectedCells": 12,
            "effectSize": 0.2,
            "publicPromptLabel": "keep",
        }
        clean, removed = study.redact_value(value, ())
        self.assertEqual(clean, value)
        self.assertEqual(removed, 0)

    def test_redaction_removes_spaced_paths_and_valid_network_addresses(self) -> None:
        value = (
            "C:\\Users\\Nick Seal\\secret\\capture.json | "
            "/home/Nick Seal/secret/capture.json | 2001:db8::1 | 192.0.2.8"
        )
        clean, removed = study.redact_value(value, ())
        self.assertGreaterEqual(removed, 4)
        self.assertNotIn("Nick Seal", clean)
        self.assertNotIn("2001:db8::1", clean)
        self.assertNotIn("192.0.2.8", clean)
        study.assert_sanitized(clean)

    def test_invalid_ipv4_like_value_is_not_classified_as_an_address(self) -> None:
        value = "999.999.999.999"
        clean, removed = study.redact_value(value, ())
        self.assertEqual(clean, value)
        self.assertEqual(removed, 0)

    def test_unspecified_ipv6_is_private_outside_mcp_ascii_art(self) -> None:
        with self.assertRaisesRegex(study.StudyError, "IP address"):
            study.assert_sanitized({"text": "my address is ::"})
        study.assert_sanitized({"render": "::"}, "mcp")

    def test_malformed_tool_text_is_a_study_error(self) -> None:
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "tool",
            "sessionId": "sol-p01-g",
            "room": "times-tables",
            "sequence": 1,
            "role": "encounter",
            "tool": "play_room",
            "arguments": {},
            "structuredResult": {},
            "visibleText": 42,
            "toolOutcome": "success",
            "binarySha256": "2" * 64,
            "binaryBuildReceipt": binary_build_receipt(),
            "_sourceIndex": 0,
        }
        with self.assertRaises(study.StudyError):
            study.validate_event_shape(event)


class CohortAnalysisTests(unittest.TestCase):
    def setUp(self) -> None:
        self.bank = study.load_bank(study.FIXTURE_PROBE_BANK_PATH)
        self.manifest = study.build_allocation(
            self.bank, passing_calibration_audit(self.bank)
        )

    def test_complete_cohort_passes_frozen_analysis(self) -> None:
        report = study.analyze_events(
            self.manifest,
            self.bank,
            cohort_records(self.bank, self.manifest),
            bootstrap_resamples=2_000,
        )
        self.assertTrue(report["cohortComplete"])
        self.assertEqual(len(report["selectedPairs"]), 20)
        self.assertEqual(report["publicationConsents"]["bounded-raw"], 40)
        self.assertEqual(report["primary"]["pairedMeanDifference"], 1.0)
        self.assertEqual(report["primary"]["bootstrap"]["pooled95"], [1.0, 1.0])
        self.assertTrue(report["primary"]["predeclaredStatisticalGatePassed"])
        self.assertTrue(
            any(
                "cannot cryptographically prove capability removal" in limitation
                for limitation in report["runtimeProvenance"]["limitations"]
            )
        )
        self.assertNotIn(
            "completeFailureAndDeviationLedger", report["primary"]["criteria"]
        )
        self.assertFalse(report["publicationAudit"]["computedByRunner"])
        self.assertEqual(
            report["secondary"]["delayedWithinContext"]["pairedMeanDifference"],
            1.0,
        )
        self.assertEqual(
            report["secondary"]["delayedWithinContext"]["bootstrap"]["pooled95"],
            [1.0, 1.0],
        )
        self.assertEqual(
            report["primary"]["roomBootstrap95"]["times-tables"], [1.0, 1.0]
        )
        self.assertEqual(
            report["secondary"]["delayedWithinContext"]["roomBootstrap95"][
                "times-tables"
            ],
            [1.0, 1.0],
        )
        self.assertEqual(
            sum(
                group["pairs"]
                for group in report["sensitivity"][
                    "conditionCollectionOrder"
                ].values()
            ),
            20,
        )

    def test_aggregate_only_consent_suppresses_pair_and_session_details(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        pair = self.manifest["pairs"][0]
        session_ids = {session["sessionId"] for session in pair["sessions"]}
        for record in records:
            if record.get("type") == "session" and record.get("sessionId") in session_ids:
                record["publicationConsent"] = "aggregate-only"
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=100
        )
        self.assertNotIn(pair["pairId"], {item["pairId"] for item in report["pairResults"]})
        self.assertTrue(
            session_ids.isdisjoint(
                {item["sessionId"] for item in report["sessionDiagnostics"]}
            )
        )

    def test_incomplete_receipts_produce_a_content_free_audit(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        records = [
            record
            for record in records
            if not str(record.get("sessionId", "")).startswith("sol-p10-")
        ]
        receipts = study.seal_records(self.manifest, records)
        audit = study.audit_receipts(
            self.manifest,
            self.bank,
            receipts,
            study.build_receipt_anchor(self.manifest, receipts),
        )
        self.assertEqual(audit["status"], "incomplete")
        self.assertFalse(audit["cohortComplete"])
        self.assertFalse(audit["rawParticipantContentIncluded"])

    def test_withdrawn_context_tombstone_cannot_be_reused(self) -> None:
        records = cohort_records(
            self.bank,
            self.manifest,
            selected_orders={
                "gpt-5.6-sol": range(2, 12),
                "gpt-5.6-terra": range(1, 11),
            },
        )
        reused_context = next(
            record["contextId"]
            for record in records
            if record.get("type") == "session"
            and str(record.get("sessionId", "")).startswith("sol-p02-")
        )
        records.insert(
            0,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "withdrawal",
                "pairId": "sol-p01",
                "contextTombstones": [study.content_sha256(reused_context)],
                "terminalRequestSha256": "4" * 64,
                "_sourceIndex": 0,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "reused a withdrawn"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_manifest_rooted_receipts_reject_mutation_reorder_and_deletion(
        self,
    ) -> None:
        events = cohort_records(self.bank, self.manifest)
        receipts = study.seal_records(self.manifest, events)
        anchor = study.build_receipt_anchor(self.manifest, receipts)
        report = study.analyze_receipts(
            self.manifest,
            self.bank,
            receipts,
            anchor,
            bootstrap_resamples=100,
        )
        self.assertTrue(report["cohortComplete"])

        changed = json.loads(json.dumps(receipts))
        changed[3]["event"]["type"] = "deviation"
        with self.assertRaisesRegex(study.StudyError, "payload hash"):
            study.analyze_receipts(
                self.manifest,
                self.bank,
                changed,
                anchor,
                bootstrap_resamples=100,
            )

        reordered = json.loads(json.dumps(receipts))
        reordered[2], reordered[3] = reordered[3], reordered[2]
        with self.assertRaisesRegex(study.StudyError, "index"):
            study.analyze_receipts(
                self.manifest,
                self.bank,
                reordered,
                anchor,
                bootstrap_resamples=100,
            )

        with self.assertRaisesRegex(study.StudyError, "index|chain"):
            study.analyze_receipts(
                self.manifest,
                self.bank,
                receipts[:2] + receipts[3:],
                anchor,
                bootstrap_resamples=100,
            )

        with self.assertRaisesRegex(study.StudyError, "terminal anchor"):
            study.analyze_receipts(
                self.manifest,
                self.bank,
                receipts[:-1],
                anchor,
                bootstrap_resamples=100,
            )

        changed_manifest = json.loads(json.dumps(self.manifest))
        changed_manifest["pairs"][0]["studySeed"] = "0" * 64
        with self.assertRaisesRegex(study.StudyError, "manifest commitment"):
            study.verify_receipts(changed_manifest, receipts)
        self.assertEqual(
            sum(
                group["pairs"]
                for group in report["sensitivity"]["firstRoomPosition"].values()
            ),
            20,
        )

    def test_balance_sensitivity_uses_observed_non_degenerate_pair_results(
        self,
    ) -> None:
        records = cohort_records(self.bank, self.manifest)
        target_pair = self.manifest["pairs"][0]
        generation_id = next(
            session["sessionId"]
            for session in target_pair["sessions"]
            if session["condition"] == study.CONDITIONS[0]
        )
        probes = {probe["id"]: probe for probe in self.bank["probes"]}
        for record in records:
            if (
                record.get("sessionId") == generation_id
                and record.get("type") == "response"
            ):
                record["answer"] = wrong_answer(probes[record["probeId"]])
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=200
        )
        first_condition = (
            "generationFirst"
            if target_pair["collectionOrder"][0] == generation_id
            else "controlFirst"
        )
        order_group = report["sensitivity"]["conditionCollectionOrder"][first_condition]
        room_group = report["sensitivity"]["firstRoomPosition"][
            target_pair["roomOrder"][0]
        ]
        self.assertLess(order_group["meanImmediateDifference"], 1.0)
        self.assertLess(room_group["meanImmediateDifference"], 1.0)

    def test_unknown_event_fields_are_rejected(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        records[0]["email"] = "person@example.com"
        with self.assertRaisesRegex(study.StudyError, "unknown field"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_response_phase_must_match_the_frozen_probe(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        response = next(
            record for record in records if record.get("type") == "response"
        )
        response["phase"] = "late" if response["phase"] == "immediate" else "immediate"
        with self.assertRaisesRegex(study.StudyError, "phase"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_boolean_sequence_and_attempt_are_rejected(self) -> None:
        for field, event_type in (("sequence", "tool"), ("attempt", "response")):
            records = cohort_records(self.bank, self.manifest)
            event = next(
                record for record in records if record.get("type") == event_type
            )
            event[field] = True
            with self.subTest(field=field):
                with self.assertRaises(study.StudyError):
                    study.analyze_events(
                        self.manifest, self.bank, records, bootstrap_resamples=100
                    )

    def test_huge_numeric_answer_scores_invalid_without_traceback(self) -> None:
        probe = next(
            probe
            for probe in self.bank["probes"]
            if probe["answerSchema"]["type"] == "number"
        )
        self.assertEqual(study.score_answer(probe, 10**10_000), (False, False))

    def test_generation_interruption_uses_hypothesis_adverse_zero_imputation(
        self,
    ) -> None:
        records = cohort_records(self.bank, self.manifest)
        session_id = "sol-p01-g"
        session_indexes = [
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        ]
        keep_through = session_indexes[1]
        records = [
            record
            for index, record in enumerate(records)
            if record.get("sessionId") != session_id
            or (
                index <= keep_through
                and record.get("type")
                not in (
                    "response",
                    "response_refusal",
                    "distractor_response",
                    "condition_response",
                    "feedback",
                )
            )
        ]
        insert_at = max(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        ) + 1
        records.insert(
            insert_at,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "session_interruption",
                "sessionId": session_id,
                "stage": "encounter",
                "reasonCode": "context-lost",
                "terminalRequestSha256": "4" * 64,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=100
        )
        self.assertIn("sol-p01", report["selectedPairs"])
        self.assertEqual(report["sessionInterruptions"], 1)
        diagnostic = next(
            item
            for item in report["sessionDiagnostics"]
            if item["sessionId"] == session_id
        )
        self.assertTrue(diagnostic["interrupted"])
        self.assertEqual(diagnostic["immediateScore"], 0.0)
        self.assertEqual(diagnostic["lateScore"], 0.0)
        self.assertEqual(diagnostic["missingDataRule"], "hypothesis-adverse")

    def test_control_interruption_cannot_inflate_the_generation_difference(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        session_id = "sol-p01-c"
        session_indexes = [
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        ]
        keep_through = session_indexes[1]
        records = [
            record
            for index, record in enumerate(records)
            if record.get("sessionId") != session_id
            or (
                index <= keep_through
                and record.get("type")
                not in (
                    "response",
                    "response_refusal",
                    "distractor_response",
                    "condition_response",
                    "feedback",
                )
            )
        ]
        insert_at = max(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        ) + 1
        records.insert(
            insert_at,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "session_interruption",
                "sessionId": session_id,
                "stage": "encounter",
                "reasonCode": "context-lost",
                "terminalRequestSha256": "4" * 64,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=100
        )
        diagnostic = next(
            item
            for item in report["sessionDiagnostics"]
            if item["sessionId"] == session_id
        )
        self.assertEqual(diagnostic["immediateScore"], 1.0)
        pair = next(
            item for item in report["pairResults"] if item["pairId"] == "sol-p01"
        )
        self.assertLessEqual(pair["pairedImmediateDifference"], 0.0)
        missing = report["sensitivity"]["missingData"]
        self.assertTrue(missing["interruptionCeiling"]["met"])
        self.assertEqual(missing["completeCase"]["pairsByModelFamily"]["gpt-5.6-sol"], 9)

    def test_interrupted_retained_reveal_must_match_the_pair(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        session_id = "sol-p01-g"
        keep_through = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id and record.get("role") == "reveal"
        )
        reveal = records[keep_through]
        self.assertEqual(reveal["role"], "reveal")
        reveal["visibleText"] = "different retained Reveal"
        records = [
            record
            for index, record in enumerate(records)
            if record.get("sessionId") != session_id
            or (
                index <= keep_through
                and record.get("type")
                not in (
                    "response",
                    "response_refusal",
                    "distractor_response",
                    "condition_response",
                    "feedback",
                )
            )
        ]
        insert_at = max(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        ) + 1
        records.insert(
            insert_at,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "session_interruption",
                "sessionId": session_id,
                "stage": "encounter",
                "reasonCode": "context-lost",
                "terminalRequestSha256": "4" * 64,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(
            study.StudyError, "identical (?:public MCP|Reveal) payloads"
        ):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_interruption_before_any_exposure_is_rejected(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        session_id = "sol-p01-g"
        records = [
            record
            for record in records
            if record.get("sessionId") != session_id or record.get("type") == "session"
        ]
        header_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == session_id
        )
        records.insert(
            header_index + 1,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "session_interruption",
                "sessionId": session_id,
                "stage": "encounter",
                "reasonCode": "context-lost",
                "terminalRequestSha256": "4" * 64,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "after exposure"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_incomplete_cohort_refuses_to_report(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        records = [
            record
            for record in records
            if not str(record.get("sessionId", "")).startswith("sol-p10-")
        ]
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "incomplete cohort at sol-p10"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_ordered_reserve_replaces_withdrawn_pair(self) -> None:
        records = cohort_records(
            self.bank,
            self.manifest,
            selected_orders={
                "gpt-5.6-sol": range(2, 12),
                "gpt-5.6-terra": range(1, 11),
            },
        )
        records.insert(
            0,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "withdrawal",
                "pairId": "sol-p01",
                "contextTombstones": ["3" * 64],
                "terminalRequestSha256": "4" * 64,
                "_sourceIndex": 0,
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=200
        )
        self.assertEqual(report["withdrawals"], 1)
        self.assertIn("sol-p11", report["selectedPairs"])
        self.assertNotIn("sol-p01", report["selectedPairs"])

    def test_generation_condition_reveal_leak_is_rejected(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        leaked = next(
            record
            for record in records
            if record.get("sessionId") == "sol-p01-g"
            and record.get("type") == "tool"
            and record.get("sequence") == 1
        )
        leaked["structuredResult"] = {"reveal": "early"}
        with self.assertRaisesRegex(study.StudyError, "leaked Reveal"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_pair_must_receive_the_same_reveal(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        changed = next(
            record
            for record in records
            if record.get("sessionId") == "sol-p01-c"
            and record.get("type") == "tool"
            and record.get("role") == "reveal"
        )
        changed["visibleText"] = "different public Reveal"
        with self.assertRaisesRegex(
            study.StudyError, "identical (?:public MCP|Reveal) payloads"
        ):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_room_tool_calls_may_not_interleave(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        indexes = [
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "sol-p01-g" and record.get("type") == "tool"
        ]
        records[indexes[1]], records[indexes[4]] = (
            records[indexes[4]],
            records[indexes[1]],
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "interleaves or reorders"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_condition_collection_order_is_frozen(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        first_pair = self.manifest["pairs"][0]
        first_id, second_id = first_pair["collectionOrder"]
        first_block = [
            record for record in records if record.get("sessionId") == first_id
        ]
        second_block = [
            record for record in records if record.get("sessionId") == second_id
        ]
        insertion = min(record["_sourceIndex"] for record in first_block + second_block)
        records = [
            record
            for record in records
            if record.get("sessionId") not in (first_id, second_id)
        ]
        records[insertion:insertion] = second_block + first_block
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "condition collection order"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_condition_sessions_may_not_overlap(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        first_id, second_id = self.manifest["pairs"][0]["collectionOrder"]
        second_header_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == second_id and record.get("type") == "session"
        )
        second_header = records.pop(second_header_index)
        first_header_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == first_id and record.get("type") == "session"
        )
        records.insert(first_header_index + 1, second_header)
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "overlap"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_sessions_from_different_families_may_not_overlap(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        foreign_header_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "terra-p01-g"
            and record.get("type") == "session"
        )
        foreign_header = records.pop(foreign_header_index)
        target_header_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "sol-p01-g"
            and record.get("type") == "session"
        )
        records.insert(target_header_index + 1, foreign_header)
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "qualifying sessions overlap"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_complete_cohort_uses_one_numinous_commit(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        for record in records:
            if record.get("type") == "session" and str(
                record.get("sessionId")
            ).startswith("sol-p02-"):
                record["numinousCommit"] = "2" * 40
        with self.assertRaisesRegex(study.StudyError, "one Numinous commit"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_qualifying_sessions_require_distinct_start_receipts(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        headers = [record for record in records if record.get("type") == "session"]
        headers[1]["attemptStartReceiptSha256"] = headers[0][
            "attemptStartReceiptSha256"
        ]
        with self.assertRaisesRegex(study.StudyError, "distinct attempt start"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )

    def test_one_schema_repair_is_scored(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        target_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "sol-p01-g"
            and record.get("type") == "response"
            and record.get("phase") == "immediate"
        )
        target = records[target_index]
        probe = next(
            probe for probe in self.bank["probes"] if probe["id"] == target["probeId"]
        )
        target["answer"] = "invalid"
        retry = dict(target)
        retry["attempt"] = 2
        retry["answer"] = study.oracle_answer(probe["oracle"])
        records.insert(target_index + 1, retry)
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=200
        )
        diagnostic = next(
            item
            for item in report["sessionDiagnostics"]
            if item["sessionId"] == "sol-p01-g"
        )
        self.assertEqual(diagnostic["schemaRepairs"], 1)
        self.assertEqual(diagnostic["invalidAttempts"], 1)
        self.assertEqual(diagnostic["immediateScore"], 1.0)

    def test_probe_may_be_refused_after_schema_repair(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        target_index = next(
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "sol-p01-g"
            and record.get("type") == "response"
            and record.get("phase") == "immediate"
        )
        target = records[target_index]
        target["answer"] = "invalid"
        records.insert(
            target_index + 1,
            {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "response_refusal",
                "sessionId": target["sessionId"],
                "phase": target["phase"],
                "probeId": target["probeId"],
            },
        )
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        report = study.analyze_events(
            self.manifest, self.bank, records, bootstrap_resamples=100
        )
        diagnostic = next(
            item
            for item in report["sessionDiagnostics"]
            if item["sessionId"] == "sol-p01-g"
        )
        self.assertEqual(diagnostic["schemaRepairs"], 1)
        self.assertEqual(diagnostic["responseRefusals"], 1)
        self.assertLess(diagnostic["immediateScore"], 1.0)

    def test_second_schema_repair_in_one_session_is_rejected(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        targets = [
            index
            for index, record in enumerate(records)
            if record.get("sessionId") == "sol-p01-g"
            and record.get("type") == "response"
            and record.get("phase") == "immediate"
        ][:2]
        for target_index in reversed(targets):
            target = records[target_index]
            probe = next(
                probe
                for probe in self.bank["probes"]
                if probe["id"] == target["probeId"]
            )
            target["answer"] = "invalid"
            retry = dict(target)
            retry["attempt"] = 2
            retry["answer"] = study.oracle_answer(probe["oracle"])
            records.insert(target_index + 1, retry)
        for index, record in enumerate(records):
            record["_sourceIndex"] = index
        with self.assertRaisesRegex(study.StudyError, "one schema repair"):
            study.analyze_events(
                self.manifest, self.bank, records, bootstrap_resamples=100
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
