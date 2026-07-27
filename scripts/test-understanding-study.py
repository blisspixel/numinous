#!/usr/bin/env python3
"""Regression tests for the frozen Understanding Alpha study runner."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "scripts" / "understanding-study.py"
sys.dont_write_bytecode = True


def load_runner():
    spec = importlib.util.spec_from_file_location("numinous_understanding_study", RUNNER)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load understanding-study.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


study = load_runner()


def wrong_answer(probe: dict) -> int | float | str:
    schema = probe["answerSchema"]
    expected = study.oracle_answer(probe["oracle"])
    if schema["type"] == "number":
        return float(expected) + 1.0
    return next(choice for choice in schema["enum"] if choice != expected)


def session_records(
    bank: dict,
    pair: dict,
    session: dict,
    correct: bool,
) -> list[dict]:
    session_id = session["sessionId"]
    condition = session["condition"]
    roles = (
        ["encounter", "generation", "interaction", "reveal"]
        if condition == study.CONDITIONS[0]
        else ["reveal", "explanation", "interaction", "continuation"]
    )
    records = [
        {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session",
            "sessionId": session_id,
            "consent": True,
            "modelFamily": pair["modelFamily"],
            "modelIdentifier": pair["modelFamily"],
            "provider": "frozen-test-runtime",
            "backendRevision": "revision-1",
            "reasoningEffort": "high",
            "settings": {"sampling": "platform-default"},
            "date": "2026-07-27",
            "numinousCommit": "1" * 40,
            "mcpProtocolRevision": "2025-11-25",
            "operatingSystem": "test-os",
            "runnerVersion": study.RUNNER_VERSION,
            "condition": condition,
        }
    ]
    for room in pair["roomOrder"]:
        for sequence, role in enumerate(roles, start=1):
            structured = {"reveal": "same public Reveal"} if role == "reveal" else {}
            records.append(
                {
                    "schemaVersion": study.EVENT_SCHEMA,
                    "type": "tool",
                    "sessionId": session_id,
                    "room": room,
                    "sequence": sequence,
                    "role": role,
                    "tool": "study-fixture",
                    "arguments": {"room": room, "sequence": sequence},
                    "structuredResult": structured,
                    "visibleText": f"public {role}",
                }
            )
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
            answer = study.oracle_answer(probe["oracle"]) if correct else wrong_answer(probe)
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
        self.bank = study.load_bank()
        self.manifest = study.build_allocation(self.bank)

    def test_frozen_hashes_and_balanced_allocation(self) -> None:
        self.assertEqual(
            study.content_sha256(self.bank),
            "4ac647fdfc4559b26ab417ece0eb01a021fc53d0decda35d8d5c798fd000cbc0",
        )
        self.assertEqual(
            study.content_sha256(self.manifest),
            "8a241287e91589d18e877f75d07d9fc03cb98dbd518e63b7cd26aa8922fd1a01",
        )
        self.assertEqual(len(self.manifest["pairs"]), 24)
        for model in study.MODEL_FAMILIES:
            pairs = [pair for pair in self.manifest["pairs"] if pair["modelFamily"] == model]
            self.assertEqual(sum(pair["allocationRole"] == "primary" for pair in pairs), 10)
            self.assertEqual(sum(pair["allocationRole"] == "reserve" for pair in pairs), 2)
            first_conditions = [
                next(
                    session["condition"]
                    for session in pair["sessions"]
                    if session["sessionId"] == pair["collectionOrder"][0]
                )
                for pair in pairs
            ]
            self.assertEqual(first_conditions.count(study.CONDITIONS[0]), 6)
            self.assertEqual(first_conditions.count(study.CONDITIONS[1]), 6)
            room_counts = {
                room: sum(pair["roomOrder"][0] == room for pair in pairs)
                for room in study.ROOMS
            }
            self.assertLessEqual(max(room_counts.values()) - min(room_counts.values()), 1)

    def test_public_packets_never_expose_oracles(self) -> None:
        packet = study.public_probe(self.bank["probes"][0], schema_only=False)
        repair = study.public_probe(self.bank["probes"][0], schema_only=True)
        self.assertNotIn("oracle", json.dumps(packet))
        self.assertNotIn("prompt", repair)
        self.assertEqual(set(repair), {"schemaVersion", "probeId", "answerSchema"})

    def test_manifest_mutation_is_rejected(self) -> None:
        changed = json.loads(json.dumps(self.manifest))
        changed["pairs"][0]["roomOrder"].reverse()
        with self.assertRaisesRegex(study.StudyError, "differs from the frozen"):
            study.validate_manifest(changed, self.bank)

    def test_independent_oracles_cover_every_probe(self) -> None:
        for probe in self.bank["probes"]:
            expected = study.oracle_answer(probe["oracle"])
            valid, correct = study.score_answer(probe, expected)
            self.assertTrue(valid, probe["id"])
            self.assertTrue(correct, probe["id"])


class RedactionTests(unittest.TestCase):
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
                        "visibleText": "C:\\Users\\PrivateName\\journal.txt PrivateName",
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


class CohortAnalysisTests(unittest.TestCase):
    def setUp(self) -> None:
        self.bank = study.load_bank()
        self.manifest = study.build_allocation(self.bank)

    def test_complete_cohort_passes_frozen_analysis(self) -> None:
        report = study.analyze_events(
            self.manifest,
            self.bank,
            cohort_records(self.bank, self.manifest),
            bootstrap_resamples=2_000,
        )
        self.assertTrue(report["cohortComplete"])
        self.assertEqual(len(report["selectedPairs"]), 20)
        self.assertEqual(report["primary"]["pairedMeanDifference"], 1.0)
        self.assertEqual(report["primary"]["bootstrap"]["pooled95"], [1.0, 1.0])
        self.assertTrue(report["primary"]["predeclaredStatisticalGatePassed"])

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
        with self.assertRaisesRegex(study.StudyError, "identical Reveal payloads"):
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
        records[indexes[1]], records[indexes[4]] = records[indexes[4]], records[indexes[1]]
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
        first_block = [record for record in records if record.get("sessionId") == first_id]
        second_block = [record for record in records if record.get("sessionId") == second_id]
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

    def test_complete_cohort_uses_one_numinous_commit(self) -> None:
        records = cohort_records(self.bank, self.manifest)
        for record in records:
            if record.get("type") == "session" and str(record.get("sessionId")).startswith(
                "sol-p02-"
            ):
                record["numinousCommit"] = "2" * 40
        with self.assertRaisesRegex(study.StudyError, "one Numinous commit"):
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
        probe = next(probe for probe in self.bank["probes"] if probe["id"] == target["probeId"])
        target["answer"] = {"invalid": True}
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
            item for item in report["sessionDiagnostics"] if item["sessionId"] == "sol-p01-g"
        )
        self.assertEqual(diagnostic["schemaRepairs"], 1)
        self.assertEqual(diagnostic["invalidAttempts"], 1)
        self.assertEqual(diagnostic["immediateScore"], 1.0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
