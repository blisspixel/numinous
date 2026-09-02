#!/usr/bin/env python3
"""Invariant tests for the stateful Understanding Alpha collector."""

from __future__ import annotations

import importlib.util
import argparse
import concurrent.futures
import contextlib
import hashlib
import io
import json
import os
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
COLLECTOR = ROOT / "scripts" / "understanding-collect.py"
sys.dont_write_bytecode = True


def load_collector():
    spec = importlib.util.spec_from_file_location(
        "numinous_understanding_collect", COLLECTOR
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load understanding-collect.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


collector = load_collector()
study = collector.study
RUNNER_REVISION = "1" * 40
RUNNER_SOURCE_SHA256 = "2" * 64


def binary_build_receipt() -> dict:
    return {
        "schemaVersion": study.MCP_BUILD_RECEIPT_SCHEMA,
        "sourceRevision": RUNNER_REVISION,
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


def fake_driver_artifact(path: Path):
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return collector.mcp_play.BuiltArtifact(
        path=path,
        sha256=digest,
        receipt={"binarySha256": digest},
        owner=None,
    )


def fake_tool_call(
    tool: str,
    arguments: dict,
    *,
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
) -> tuple[dict, dict]:
    """Return a stable public result without starting the Rust server."""
    visible = f"public result for {tool}"
    server_info = {"name": "numinous", "version": "0.2.0"}
    if tool == "play_room":
        interaction_status = {
            "times-tables": "K 6.00",
            "double-pendulum": "TWINS 0.000",
            "game-of-life": "GLIDER 1",
            "galton-board": "P.50",
        }
        structured = {
            "action": "observe",
            "delta": None,
            "engineeredAha": {},
            "gesture": None,
            "goal": "understand",
            "goalMet": False,
            "height": 1,
            "pokes": [],
            "render": visible,
            "reveal": None,
            "room": arguments["id"],
            "status": interaction_status.get(arguments["id"], "public")
            if "pokes" in arguments
            else "public",
            "t": arguments["t"],
            "title": arguments["id"],
            "variation": 0,
            "width": 1,
        }
    elif tool == "reveal_room":
        structured = {
            "concept": "public concept",
            "reveal": "same public reveal",
            "room": arguments["id"],
            "title": arguments["id"],
        }
    elif tool == "plot_expression":
        structured = {
            "a": 1.0,
            "discovery": "manual",
            "expression": arguments["expr"],
            "kind": "graph",
            "plot": visible,
            "recipeCount": 1,
            "recipeIndex": None,
            "valid": True,
            "xmax": 1.0,
            "xmin": -1.0,
            "ymax": 1.0,
            "ymin": -1.0,
        }
    else:
        raise AssertionError(f"unexpected tool {tool}")
    build_receipt = binary_build_receipt()
    if expected_revision is not None:
        build_receipt["sourceRevision"] = expected_revision
    if expected_source_sha256 is not None:
        build_receipt["studySourceSha256"] = expected_source_sha256
    return (
        {
            "protocolVersion": study.MCP_PROTOCOL_REVISION,
            "numinousBinarySha256": "2" * 64,
            "binaryBuildReceipt": build_receipt,
            "serverInfo": server_info,
        },
        {
            "_meta": {collector.mcp_play.SERVER_INFO_META_KEY: dict(server_info)},
            "structuredContent": structured,
            "content": [{"type": "text", "text": visible}],
            "isError": False,
            "resultType": "complete",
        },
    )


def passing_calibration_audit(bank: dict) -> dict:
    records = []
    ordinal = 0
    for probe in bank["probes"]:
        expected = study.oracle_answer(probe["oracle"])
        wrong = (
            float(expected) + 1.0
            if probe["answerSchema"]["type"] == "number"
            else next(
                choice
                for choice in probe["answerSchema"]["enum"]
                if choice != expected
            )
        )
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
                            f"collector fixture:{probe['id']}:{model}:{replicate}"
                        ),
                        "backendRevision": "unavailable",
                        "reasoningEffort": "high",
                        "capabilityPolicy": study.CALIBRATION_CAPABILITY_POLICY,
                        "freshContext": True,
                        "attempt": 1,
                        "runnerVersion": study.RUNNER_VERSION,
                        "runnerRevision": RUNNER_REVISION,
                        "runnerSourceSha256": RUNNER_SOURCE_SHA256,
                        "attemptStartReceiptSha256": study.content_sha256(
                            f"collector fixture start receipt:{ordinal}"
                        ),
                        "date": "2026-07-31",
                        "answer": wrong,
                    }
                )
    reviewer_ids = (
        study.content_sha256("collector fixture relevance reviewer one"),
        study.content_sha256("collector fixture relevance reviewer two"),
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
    return study.calibrate_bank(
        bank,
        records,
        relevance,
        "5" * 64,
        RUNNER_REVISION,
        RUNNER_SOURCE_SHA256,
    )


class CollectorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.bank = study.load_bank(study.FIXTURE_PROBE_BANK_PATH)
        self.manifest = study.build_allocation(
            self.bank, passing_calibration_audit(self.bank)
        )
        self.pair = self.manifest["pairs"][0]
        self.session_id = next(
            session["sessionId"]
            for session in self.pair["sessions"]
            if session["condition"] == study.CONDITIONS[0]
        )
        self.session = next(
            session
            for session in self.pair["sessions"]
            if session["sessionId"] == self.session_id
        )
        agent_root = ROOT / ".agent"
        agent_root.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(
            prefix="understanding-collector-test-", dir=agent_root
        )
        self.ledger = Path(self.temporary.name) / "nested" / "receipts.jsonl"
        self.state = {
            "schemaVersion": collector.STATE_SCHEMA,
            "sessionId": self.session_id,
            "cursor": 0,
            "repairUsed": False,
            "repairPending": False,
            "complete": False,
            "manifestSha256": study.content_sha256(self.manifest),
            "probeBankSha256": study.content_sha256(self.bank),
            "sessionLedger": str(self.ledger),
            "cohortLedger": str(Path(self.temporary.name) / "cohort.jsonl"),
            "numinousCommit": "1" * 40,
            "pairId": self.pair["pairId"],
            "collectionOrder": self.pair["collectionOrder"],
            "pairStatePaths": [str(self.ledger.with_name("state.json"))],
            "withdrawalNonce": "6" * 64,
            "consentPending": False,
            "refusalOrdinal": None,
            "headerDraft": {
                key: value
                for key, value in self.header().items()
                if key != "publicationConsent"
            },
            "manifestSnapshot": self.manifest,
        }
        collector.append_receipt(self.ledger, self.manifest, self.header())

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def header(self) -> dict:
        return {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session",
            "sessionId": self.session_id,
            "consent": True,
            "publicationConsent": "bounded-raw",
            "modelFamily": self.pair["modelFamily"],
            "modelIdentifier": self.pair["modelFamily"],
            "provider": study.MODEL_PROVIDERS[self.pair["modelFamily"]],
            "backendRevision": "unavailable",
            "reasoningEffort": self.pair["reasoningEffort"],
            "settings": {"sampling": "platform-default", "freshContext": True},
            "date": "2026-07-28",
            "numinousCommit": "1" * 40,
            "mcpProtocolRevision": study.MCP_PROTOCOL_REVISION,
            "operatingSystem": "test",
            "runnerVersion": study.RUNNER_VERSION,
            "studySourceSha256": RUNNER_SOURCE_SHA256,
            "attemptStartReceiptSha256": "7" * 64,
            "condition": self.session["condition"],
            "contextId": study.content_sha256(f"{self.session_id}-fresh"),
            "capabilityPolicy": "collector-only-no-repository-web-or-tools",
        }

    def answer_pending(self, action: dict, state: dict | None = None) -> dict:
        state = state or self.state
        request_id = collector.current_request_id(state)
        if action["kind"] == "condition_response":
            if action["stage"] in ("prediction", "construction"):
                room = study.encounter_rooms()[action["room"]]
                return {
                    "requestId": request_id,
                    "answer": "sin(4*x)"
                    if action["room"] == "formula-jam"
                    else room["expectedAnswer"],
                    "rationale": "A bounded rationale for this committed answer.",
                }
            return {"requestId": request_id, "text": "bounded public response"}
        if action["kind"] == "distractor":
            return {"requestId": request_id, "answer": 0}
        if action["kind"] == "probe":
            return {
                "requestId": request_id,
                "answer": study.oracle_answer(action["probe"]["oracle"]),
            }
        raise AssertionError(f"unexpected pending action {action['kind']}")

    def advance_to_probe(self) -> dict:
        output = collector.advance(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            tool_caller=fake_tool_call,
        )
        actions = collector.session_actions(self.bank, self.manifest, self.session_id)
        while actions[self.state["cursor"]]["kind"] != "probe":
            action = actions[self.state["cursor"]]
            output = collector.record_response(
                self.state,
                self.bank,
                self.manifest,
                self.ledger,
                self.answer_pending(action),
                tool_caller=fake_tool_call,
            )
        return output

    def withdrawal_case(
        self, name: str, *, include_response: bool = True
    ) -> tuple[Path, Path, Path, argparse.Namespace]:
        cohort = Path(self.temporary.name) / f"{name}-cohort.jsonl"
        pending = collector.pending_session_ledger(cohort, self.session_id)
        collector.append_receipt(pending, self.manifest, self.header())
        if include_response:
            room_id = self.pair["roomOrder"][0]
            collector.append_receipt(
                pending,
                self.manifest,
                {
                    "schemaVersion": study.EVENT_SCHEMA,
                    "type": "condition_response",
                    "sessionId": self.session_id,
                    "room": room_id,
                    "stage": "prediction",
                    "answer": study.encounter_rooms()[room_id]["expectedAnswer"],
                    "rationale": "A bounded private rationale that must be erased.",
                },
            )
        state_path = Path(self.temporary.name) / f"{name}-state.json"
        state = {
            **self.state,
            "sessionLedger": str(pending),
            "cohortLedger": str(cohort),
            "pairStatePaths": [str(state_path)],
            "manifestSnapshot": self.manifest,
        }
        collector.write_state(state_path, state)
        collector.claim_active(cohort, state_path, self.session_id)
        return (
            cohort,
            pending,
            state_path,
            argparse.Namespace(
                state=state_path,
                ledger=cohort,
                input="-",
            ),
        )

    @staticmethod
    def withdrawal_payload(state_path: Path) -> str:
        state = collector.load_state(state_path)
        request = collector.withdrawal_request(state, False)
        return json.dumps(
            {
                "requestId": request["requestId"],
                "terminalAction": "withdraw",
            }
        )

    def start_case(
        self, name: str
    ) -> tuple[Path, Path, Path, Path, argparse.Namespace]:
        root = Path(self.temporary.name)
        bank_path = root / f"{name}-bank.json"
        manifest_path = root / f"{name}-allocation.json"
        state_path = root / f"{name}-state.json"
        cohort = root / f"{name}-cohort.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")
        args = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            session_id=self.pair["collectionOrder"][0],
            context_id=study.content_sha256(f"{name}-fresh-context"),
            model_identifier=self.pair["modelFamily"],
            backend_revision="unavailable",
        )
        with (
            mock.patch.object(collector, "repository_commit", return_value="1" * 40),
            mock.patch.object(
                collector, "study_source_sha256", return_value=RUNNER_SOURCE_SHA256
            ),
            mock.patch.object(
                collector, "require_attempt_start_receipt", return_value="7" * 64
            ),
            mock.patch.object(collector, "require_committed_file"),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_start(args)
        return bank_path, manifest_path, state_path, cohort, args

    def command_patches(
        self, *, tool: bool = False, receipt: bool = True
    ) -> contextlib.ExitStack:
        stack = contextlib.ExitStack()
        stack.enter_context(
            mock.patch.object(collector, "repository_commit", return_value="1" * 40)
        )
        stack.enter_context(
            mock.patch.object(
                collector, "study_source_sha256", return_value=RUNNER_SOURCE_SHA256
            )
        )
        if receipt:
            stack.enter_context(
                mock.patch.object(
                    collector,
                    "require_attempt_start_receipt",
                    return_value="7" * 64,
                )
            )
        stack.enter_context(mock.patch.object(collector, "require_committed_file"))
        if tool:
            stack.enter_context(
                mock.patch.object(
                    collector.mcp_play,
                    "isolated_tool_call",
                    side_effect=fake_tool_call,
                )
            )
        return stack

    def test_only_current_probe_is_projected(self) -> None:
        output = self.advance_to_probe()
        actions = collector.session_actions(self.bank, self.manifest, self.session_id)
        current = actions[self.state["cursor"]]["probe"]
        future = actions[self.state["cursor"] + 1]["probe"]
        public = json.dumps(output, sort_keys=True)
        self.assertEqual(output["responseRequest"]["probeId"], current["id"])
        self.assertNotIn("oracle", public.casefold())
        self.assertNotIn(future["id"], public)

    def test_conditions_have_one_participant_response_per_room(self) -> None:
        stages_by_condition = {}
        for session in self.pair["sessions"]:
            actions = collector.session_actions(
                self.bank, self.manifest, session["sessionId"]
            )
            first_probe = next(
                index
                for index, action in enumerate(actions)
                if action["kind"] == "probe"
            )
            stages_by_condition[session["condition"]] = [
                action["stage"]
                for action in actions[:first_probe]
                if action["kind"] == "condition_response"
            ]
        self.assertEqual(
            stages_by_condition[study.CONDITIONS[0]],
            [
                "construction" if room == "formula-jam" else "prediction"
                for room in self.pair["roomOrder"]
            ],
        )
        self.assertEqual(
            stages_by_condition[study.CONDITIONS[1]],
            ["elaboration"] * len(study.ROOMS),
        )
        self.assertEqual(
            len(stages_by_condition[study.CONDITIONS[0]]),
            len(stages_by_condition[study.CONDITIONS[1]]),
        )
        self.assertNotIn("actions", self.state)

    def test_stale_response_cannot_bind_to_the_next_probe(self) -> None:
        self.advance_to_probe()
        action = collector.session_actions(
            self.bank, self.manifest, self.session_id
        )[self.state["cursor"]]
        response = self.answer_pending(action)
        collector.record_response(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            response,
            tool_caller=fake_tool_call,
        )
        with self.assertRaisesRegex(collector.CollectorError, "requestId is stale"):
            collector.record_response(
                self.state,
                self.bank,
                self.manifest,
                self.ledger,
                response,
                tool_caller=fake_tool_call,
            )

    def test_full_session_is_exactly_replayable_from_receipts(self) -> None:
        output = collector.advance(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            tool_caller=fake_tool_call,
        )
        actions = collector.session_actions(self.bank, self.manifest, self.session_id)
        while output["status"] != "complete":
            action = actions[self.state["cursor"]]
            output = collector.record_response(
                self.state,
                self.bank,
                self.manifest,
                self.ledger,
                self.answer_pending(action),
                tool_caller=fake_tool_call,
            )
        receipts = study.read_receipt_jsonl(self.ledger)
        records = study.verify_receipts(self.manifest, receipts)
        header = records[0]
        completion = records[-1]
        body = records[1:-1]
        score = study.validate_and_score_session(
            self.bank,
            self.pair,
            self.session,
            header,
            completion,
            body,
        )
        self.assertEqual(score["immediateScore"], 1.0)
        self.assertEqual(score["lateScore"], 1.0)
        self.assertEqual(self.state["cursor"], len(actions))
        self.state["complete"] = False
        collector.reconcile_state(self.state, self.bank, self.manifest, self.ledger)
        self.assertTrue(self.state["complete"])

    def test_one_schema_only_repair_is_enforced(self) -> None:
        self.advance_to_probe()
        first_probe_cursor = self.state["cursor"]
        repair = collector.record_response(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            {
                "requestId": collector.current_request_id(self.state),
                "answer": "not-valid-for-this-schema",
            },
            tool_caller=fake_tool_call,
        )
        self.assertEqual(repair["responseRequest"]["kind"], "probe_repair")
        self.assertNotIn("prompt", repair["responseRequest"])
        self.assertEqual(self.state["cursor"], first_probe_cursor)
        collector.record_response(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            {
                "requestId": collector.current_request_id(self.state),
                "refuse": True,
            },
            tool_caller=fake_tool_call,
        )
        next_cursor = self.state["cursor"]
        output = collector.record_response(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            {
                "requestId": collector.current_request_id(self.state),
                "answer": "not-valid-for-this-schema",
            },
            tool_caller=fake_tool_call,
        )
        self.assertGreater(self.state["cursor"], next_cursor)
        self.assertNotEqual(output["responseRequest"]["kind"], "probe_repair")

    def test_mutable_state_is_reconstructed_from_receipts(self) -> None:
        first_output = collector.advance(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            tool_caller=fake_tool_call,
        )
        expected_cursor = self.state["cursor"]
        self.state.update({"cursor": 999, "repairUsed": True, "repairPending": True})
        collector.reconcile_state(self.state, self.bank, self.manifest, self.ledger)
        self.assertEqual(self.state["cursor"], expected_cursor)
        self.assertFalse(self.state["repairUsed"])
        self.assertFalse(self.state["repairPending"])
        replay = collector.advance(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            tool_caller=fake_tool_call,
        )
        self.assertEqual(replay["deliveries"], first_output["deliveries"])

    def test_collection_order_rejects_skips(self) -> None:
        first, second = self.pair["collectionOrder"]
        collector.validate_collection_start(self.manifest, [], first)
        with self.assertRaisesRegex(
            collector.CollectorError, "crossover collection order"
        ):
            collector.validate_collection_start(self.manifest, [], second)
        second_pair_first = self.manifest["pairs"][1]["collectionOrder"][0]
        with self.assertRaisesRegex(collector.CollectorError, "earlier allocated pair"):
            collector.validate_collection_start(self.manifest, [], second_pair_first)
        terminal = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_complete",
            "sessionId": first,
        }
        collector.validate_collection_start(self.manifest, [terminal], second)

    def test_consumed_pair_and_exhausted_reserve_are_rejected(self) -> None:
        first = self.pair["collectionOrder"][0]
        with self.assertRaisesRegex(collector.CollectorError, "already consumed"):
            collector.validate_collection_start(
                self.manifest,
                [
                    {
                        "type": "withdrawal",
                        "pairId": self.pair["pairId"],
                        "contextTombstones": ["3" * 64],
                        "terminalRequestSha256": "4" * 64,
                    }
                ],
                first,
            )
        family_pairs = [
            pair
            for pair in self.manifest["pairs"]
            if pair["modelFamily"] == self.pair["modelFamily"]
        ]
        events = [
            {"type": "session_complete", "sessionId": session_id}
            for pair in family_pairs[:10]
            for session_id in pair["collectionOrder"]
        ]
        with self.assertRaisesRegex(collector.CollectorError, "ten qualifying pairs"):
            collector.validate_collection_start(
                self.manifest, events, family_pairs[10]["collectionOrder"][0]
            )

    def test_response_input_is_bounded_stdin_only(self) -> None:
        with self.assertRaisesRegex(collector.CollectorError, "only from stdin"):
            collector.read_input(str(Path(self.temporary.name) / "response.json"))
        with mock.patch("sys.stdin", io.StringIO("x" * 4097)):
            with self.assertRaisesRegex(collector.CollectorError, "input limit"):
                collector.read_input("-")
        with mock.patch("sys.stdin", io.StringIO('{"answer":1,"answer":2}')):
            with self.assertRaisesRegex(collector.CollectorError, "duplicate object key"):
                collector.read_input("-")
        deeply_nested = "[" * 40 + "0" + "]" * 40
        with mock.patch("sys.stdin", io.StringIO(deeply_nested)):
            with self.assertRaisesRegex(collector.CollectorError, "nesting limit"):
                collector.read_input("-")

    def test_receipt_append_is_atomic_on_replace_failure(self) -> None:
        before = self.ledger.read_bytes()
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_interruption",
            "sessionId": self.session_id,
            "stage": "encounter",
            "reasonCode": "runtime-failure",
            "terminalRequestSha256": "4" * 64,
        }
        with (
            mock.patch.object(collector.os, "replace", side_effect=OSError("disk")),
            self.assertRaisesRegex(collector.CollectorError, "could not replace"),
        ):
            collector.append_receipt(self.ledger, self.manifest, event)
        self.assertEqual(self.ledger.read_bytes(), before)
        self.assertEqual(
            [item["type"] for item in collector.read_verified_ledger(
                self.ledger, self.manifest
            )],
            ["session"],
        )

    def test_receipt_anchor_rejects_tail_deletion(self) -> None:
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_interruption",
            "sessionId": self.session_id,
            "stage": "encounter",
            "reasonCode": "runtime-failure",
            "terminalRequestSha256": "4" * 64,
        }
        collector.append_receipt(self.ledger, self.manifest, event)
        receipts = study.read_receipt_jsonl(self.ledger)
        collector.write_receipts_atomic(
            self.ledger, receipts[:-1], "simulated truncated ledger"
        )
        with self.assertRaisesRegex(collector.CollectorError, "commitment differs"):
            collector.read_verified_ledger(self.ledger, self.manifest)

    def test_receipt_transaction_recovers_anchor_publication_failure(self) -> None:
        event = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_interruption",
            "sessionId": self.session_id,
            "stage": "encounter",
            "reasonCode": "runtime-failure",
            "terminalRequestSha256": "4" * 64,
        }
        original_write = collector.write_json_atomic

        def fail_anchor(path: Path, value: object, description: str) -> None:
            if description == "collector receipt anchor":
                raise collector.CollectorError("simulated anchor failure")
            original_write(path, value, description)

        with (
            mock.patch.object(collector, "write_json_atomic", side_effect=fail_anchor),
            self.assertRaisesRegex(collector.CollectorError, "anchor failure"),
        ):
            collector.append_receipt(self.ledger, self.manifest, event)
        self.assertTrue(collector.receipt_transaction_path(self.ledger).exists())
        with self.assertRaisesRegex(collector.CollectorError, "requires recovery"):
            collector.read_verified_ledger(self.ledger, self.manifest)
        self.assertTrue(
            collector.recover_receipt_transaction(self.ledger, self.manifest)
        )
        events = collector.read_verified_ledger(self.ledger, self.manifest)
        self.assertEqual(events[-1]["type"], "session_interruption")
        self.assertFalse(collector.receipt_transaction_path(self.ledger).exists())

    def test_concurrent_starts_cannot_remove_the_winning_state(self) -> None:
        bank_path, manifest_path, state_path, cohort, args = self.start_case(
            "concurrent-primer"
        )
        collector.release_active(cohort, state_path, args.session_id)
        state_path.unlink()
        def attempt() -> Exception | None:
            try:
                collector.command_start(args)
            except Exception as error:
                return error
            return None

        with (
            self.command_patches(),
            mock.patch("builtins.print"),
            concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor,
        ):
            results = list(executor.map(lambda _index: attempt(), range(2)))
        self.assertEqual(sum(result is None for result in results), 1)
        self.assertEqual(sum(isinstance(result, Exception) for result in results), 1)
        self.assertTrue(state_path.exists())
        self.assertTrue(collector.active_path(cohort).exists())

    def test_consent_commit_recovers_without_duplicate_header(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "consent-crash"
        )
        initial = collector.load_state(state_path)
        response = {
            "requestId": collector.current_request_id(initial),
            "participate": True,
            "publicationConsent": "bounded-raw",
        }
        original_write = collector.write_state
        failed = False

        def fail_consent_state(path: Path, state: dict) -> None:
            nonlocal failed
            if not failed and state["consentPending"] is False:
                failed = True
                raise collector.CollectorError("simulated state crash")
            original_write(path, state)

        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        with (
            self.command_patches(),
            mock.patch.object(collector, "write_state", side_effect=fail_consent_state),
            mock.patch("sys.stdin", io.StringIO(json.dumps(response))),
            self.assertRaisesRegex(collector.CollectorError, "state crash"),
        ):
            collector.command_respond(bound)
        pending = collector.pending_session_ledger(cohort, _start.session_id)
        self.assertTrue(collector.load_state(state_path)["consentPending"])
        with (
            self.command_patches(),
            mock.patch.object(
                collector.mcp_play, "isolated_tool_call", side_effect=fake_tool_call
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_status(bound)
        events = collector.read_verified_ledger(pending, self.manifest)
        self.assertEqual(sum(event["type"] == "session" for event in events), 1)
        self.assertFalse(collector.load_state(state_path)["consentPending"])

    def test_stale_consent_state_withdrawal_erases_header_and_orphan_temps(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "consent-withdraw"
        )
        initial = collector.load_state(state_path)
        response = {
            "requestId": collector.current_request_id(initial),
            "participate": True,
            "publicationConsent": "bounded-raw",
        }
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        with (
            self.command_patches(),
            mock.patch.object(
                collector, "write_state", side_effect=collector.CollectorError("crash")
            ),
            mock.patch("sys.stdin", io.StringIO(json.dumps(response))),
            self.assertRaisesRegex(collector.CollectorError, "crash"),
        ):
            collector.command_respond(bound)
        pending = collector.pending_session_ledger(cohort, _start.session_id)
        orphan_paths = [
            state_path.parent / f".{state_path.name}.dead.tmp",
            pending.parent / f".{pending.name}.dead.tmp",
            cohort.parent / f".{cohort.name}.dead.tmp",
        ]
        for path in orphan_paths:
            path.write_text("private response content", encoding="utf-8")
        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(state_path))
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_withdraw(
                argparse.Namespace(
                    state=state_path,
                    ledger=cohort,
                    input="-",
                )
            )
        self.assertFalse(pending.exists())
        self.assertFalse(state_path.exists())
        self.assertTrue(all(not path.exists() for path in orphan_paths))
        outcome = collector.read_verified_ledger(cohort, self.manifest)
        self.assertEqual([event["type"] for event in outcome], ["withdrawal"])

    def test_declined_consent_is_transparent_and_exactly_once(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "consent-decline"
        )
        state = collector.load_state(state_path)
        consent = collector.consent_output(state)
        self.assertIsNone(consent["withdrawalRequest"])
        self.assertIn("aggregate model-family refusal count", collector.CONSENT_TEXT)
        self.assertIn("start receipt", collector.CONSENT_TEXT)
        self.assertIn("bounded answers and rationales", collector.CONSENT_TEXT)
        self.assertIn("provider logs", collector.CONSENT_TEXT)
        self.assertIn("both provisional arms", collector.CONSENT_TEXT)
        self.assertIn(
            "erases participant-authored answers and rationales", collector.CONSENT_TEXT
        )
        self.assertIn("retains consent metadata", collector.CONSENT_TEXT)
        self.assertIn(
            "bounded public encounter and material receipts", collector.CONSENT_TEXT
        )
        self.assertIn("adverse interruption receipt", collector.CONSENT_TEXT)
        self.assertIn("source-bound build evidence", collector.CONSENT_TEXT)
        self.assertIn("explicit content-erasure marker", collector.CONSENT_TEXT)
        self.assertNotIn("content-free adverse", collector.CONSENT_TEXT)
        response = {
            "requestId": collector.current_request_id(state),
            "participate": False,
        }
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(state_path))
            ),
            self.assertRaisesRegex(
                collector.CollectorError, "withdrawal is unavailable before consent"
            ),
        ):
            collector.command_withdraw(
                argparse.Namespace(state=state_path, ledger=cohort, input="-")
            )
        self.assertTrue(state_path.exists())
        self.assertTrue(collector.active_path(cohort).exists())
        self.assertFalse(cohort.exists())
        original_release = collector.release_active

        def release_then_crash(ledger: Path, path: Path, session_id: str) -> None:
            original_release(ledger, path, session_id)
            raise collector.CollectorError("simulated cleanup crash")

        with (
            self.command_patches(),
            mock.patch.object(
                collector,
                "release_active",
                side_effect=release_then_crash,
            ),
            mock.patch("sys.stdin", io.StringIO(json.dumps(response))),
            self.assertRaisesRegex(collector.CollectorError, "cleanup crash"),
        ):
            collector.command_respond(bound)
        with (
            self.command_patches(),
            mock.patch("sys.stdin", io.StringIO(json.dumps(response))),
            contextlib.redirect_stdout(io.StringIO()) as output,
        ):
            collector.command_respond(bound)
        events = collector.read_verified_ledger(cohort, self.manifest)
        self.assertEqual(sum(event["type"] == "recruitment_refusal" for event in events), 1)
        result = json.loads(output.getvalue())
        self.assertFalse(result["responseContentRetainedByCollector"])
        self.assertTrue(result["aggregateModelFamilyRefusalRecorded"])

    def test_infrastructure_failure_is_atomic_and_cleanup_is_idempotent(self) -> None:
        cohort, pending, state_path, _withdraw = self.withdrawal_case(
            "infrastructure-failure", include_response=False
        )
        root = Path(self.temporary.name)
        bank_path = root / "failure-bank.json"
        manifest_path = root / "failure-allocation.json"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")
        args = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            reason_code="runtime-unavailable",
        )
        with (
            self.command_patches(),
            mock.patch.object(
                collector,
                "append_receipt_once",
                side_effect=collector.CollectorError("disk"),
            ),
            self.assertRaisesRegex(collector.CollectorError, "disk"),
        ):
            collector.command_fail(args)
        self.assertTrue(pending.exists())

        original_remove = collector.remove_path
        crashed = False

        def fail_after_outcome(path: Path, label: str) -> None:
            nonlocal crashed
            if not crashed and "failed provisional" in label:
                crashed = True
                raise collector.CollectorError("cleanup crash")
            original_remove(path, label)

        with (
            self.command_patches(),
            mock.patch.object(collector, "remove_path", side_effect=fail_after_outcome),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(collector.CollectorError, "cleanup crash"),
        ):
            collector.command_fail(args)
        self.assertTrue(pending.exists())
        self.assertEqual(
            sum(
                event["type"] == "infrastructure_failure"
                for event in collector.read_verified_ledger(cohort, self.manifest)
            ),
            1,
        )
        with self.command_patches(), contextlib.redirect_stdout(io.StringIO()):
            collector.command_fail(args)
        self.assertFalse(pending.exists())
        self.assertFalse(state_path.exists())
        self.assertEqual(
            sum(
                event["type"] == "infrastructure_failure"
                for event in collector.read_verified_ledger(cohort, self.manifest)
            ),
            1,
        )

    def test_failure_retry_after_lease_release_finishes_cleanup(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "failure-release"
        )
        args = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            reason_code="runtime-unavailable",
        )
        original_remove = collector.remove_path
        crashed = False

        def fail_state_remove(path: Path, label: str) -> None:
            nonlocal crashed
            if not crashed and label == "failed collector state":
                crashed = True
                raise collector.CollectorError("state cleanup crash")
            original_remove(path, label)

        with (
            self.command_patches(),
            mock.patch.object(collector, "remove_path", side_effect=fail_state_remove),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(collector.CollectorError, "state cleanup crash"),
        ):
            collector.command_fail(args)
        self.assertFalse(collector.active_path(cohort).exists())
        self.assertTrue(state_path.exists())
        with self.command_patches(), contextlib.redirect_stdout(io.StringIO()):
            collector.command_fail(args)
        self.assertFalse(state_path.exists())

    def test_state_pair_tampering_cannot_delete_another_pair(self) -> None:
        cohort, pending, state_path, args = self.withdrawal_case("tampered-pair")
        withdrawal_payload = self.withdrawal_payload(state_path)
        state = collector.load_state(state_path)
        other = self.manifest["pairs"][1]
        state["pairId"] = other["pairId"]
        state["collectionOrder"] = other["collectionOrder"]
        collector.write_state(state_path, state)
        with (
            mock.patch(
                "sys.stdin", io.StringIO(withdrawal_payload)
            ),
            self.assertRaisesRegex(collector.CollectorError, "pair allocation"),
        ):
            collector.command_withdraw(args)
        self.assertTrue(pending.exists())

    def test_state_and_receipt_staging_failures_are_bounded(self) -> None:
        path = Path(self.temporary.name) / "stage-state.json"
        with (
            mock.patch.object(
                collector.tempfile,
                "NamedTemporaryFile",
                side_effect=OSError("denied"),
            ),
            self.assertRaisesRegex(collector.CollectorError, "could not stage"),
        ):
            collector.write_state(path, self.state)
        with (
            mock.patch.object(
                collector.tempfile,
                "NamedTemporaryFile",
                side_effect=OSError("denied"),
            ),
            self.assertRaisesRegex(collector.CollectorError, "could not stage"),
        ):
            collector.write_receipts_atomic(
                Path(self.temporary.name) / "stage-ledger.jsonl", [], "test ledger"
            )

    def test_malformed_state_fields_fail_closed(self) -> None:
        path = Path(self.temporary.name) / "malformed-state.json"
        cases = {
            "sessionId": {},
            "cursor": False,
            "repairUsed": 1,
            "complete": "yes",
            "manifestSha256": "bad",
            "sessionLedger": {},
            "numinousCommit": "bad",
            "headerDraft": {},
        }
        for field, value in cases.items():
            with self.subTest(field=field):
                changed = {**self.state, field: value}
                path.write_text(json.dumps(changed), encoding="utf-8")
                with self.assertRaises(collector.CollectorError):
                    collector.load_state(path)
        changed = {
            **self.state,
            "pairStatePaths": [str(path)],
            "manifestSnapshot": {},
            "manifestSha256": study.content_sha256({}),
        }
        path.write_text(json.dumps(changed), encoding="utf-8")
        with self.assertRaisesRegex(collector.CollectorError, "snapshot schema"):
            collector.load_state(path)

    def test_standalone_refusal_is_concurrent_and_idempotent(self) -> None:
        root = Path(self.temporary.name)
        bank_path = root / "refusal-bank.json"
        manifest_path = root / "refusal-allocation.json"
        cohort = root / "refusal-cohort.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")
        args = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            ledger=cohort,
            model_family=self.pair["modelFamily"],
            ordinal=1,
        )

        def record() -> Exception | None:
            try:
                collector.command_refusal(args)
            except Exception as error:
                return error
            return None

        with (
            mock.patch.object(collector, "require_committed_file"),
            mock.patch("builtins.print"),
            concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor,
        ):
            results = list(executor.map(lambda _index: record(), range(2)))
        self.assertEqual(results, [None, None])
        events = collector.read_verified_ledger(cohort, self.manifest)
        self.assertEqual([event["familyRefusalOrdinal"] for event in events], [1])

    @unittest.skipUnless(os.name == "nt", "Windows process query regression")
    def test_process_liveness_check_never_uses_windows_signals(self) -> None:
        with mock.patch.object(
            collector.os,
            "kill",
            side_effect=AssertionError("Windows liveness must not deliver a signal"),
        ):
            self.assertTrue(collector.process_is_alive(os.getpid()))

    def test_path_aliases_and_reserved_sidecars_are_rejected(self) -> None:
        root = Path(self.temporary.name)
        shared = root / "shared.jsonl"
        with self.assertRaisesRegex(collector.CollectorError, "aliases"):
            collector.require_distinct_paths(
                {"collector state": shared, "collector ledger": shared}
            )
        with self.assertRaisesRegex(collector.CollectorError, "reserved"):
            collector.require_distinct_paths(
                {"collector state": root / "state.lock"}
            )

    def test_concurrent_response_is_recorded_once(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "concurrent-response"
        )
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        consent_state = collector.load_state(state_path)
        consent = {
            "requestId": collector.current_request_id(consent_state),
            "participate": True,
            "publicationConsent": "bounded-raw",
        }
        with (
            self.command_patches(tool=True),
            mock.patch.object(collector, "read_input", return_value=consent),
            mock.patch("builtins.print"),
        ):
            collector.command_respond(bound)
        state = collector.load_state(state_path)
        action = collector.session_actions(
            self.bank, self.manifest, state["sessionId"]
        )[state["cursor"]]
        response = self.answer_pending(action, state)

        def respond() -> Exception | None:
            try:
                collector.command_respond(bound)
            except Exception as error:
                return error
            return None

        with (
            self.command_patches(tool=True),
            mock.patch.object(collector, "read_input", return_value=response),
            mock.patch("builtins.print"),
        ):
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                results = list(executor.map(lambda _index: respond(), range(2)))
        self.assertEqual(sum(result is None for result in results), 1)
        pending = collector.pending_session_ledger(cohort, state["sessionId"])
        events = collector.read_verified_ledger(pending, self.manifest)
        self.assertEqual(
            sum(event["type"] == "condition_response" for event in events), 1
        )

    def test_interrupt_race_cannot_reintroduce_response_content(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "interrupt-race"
        )
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        consent_state = collector.load_state(state_path)
        consent = {
            "requestId": collector.current_request_id(consent_state),
            "participate": True,
            "publicationConsent": "bounded-raw",
        }
        with (
            self.command_patches(tool=True),
            mock.patch.object(collector, "read_input", return_value=consent),
            mock.patch("builtins.print"),
        ):
            collector.command_respond(bound)
        state = collector.load_state(state_path)
        action = collector.session_actions(
            self.bank, self.manifest, state["sessionId"]
        )[state["cursor"]]
        response = self.answer_pending(action, state)
        interrupt_args = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            reason_code="context-lost",
        )

        def invoke(handler, args) -> Exception | None:
            try:
                handler(args)
            except Exception as error:
                return error
            return None

        with (
            self.command_patches(tool=True),
            mock.patch.object(collector, "read_input", return_value=response),
            mock.patch("builtins.print"),
        ):
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                futures = (
                    executor.submit(invoke, collector.command_respond, bound),
                    executor.submit(invoke, collector.command_interrupt, interrupt_args),
                )
                results = [future.result() for future in futures]
        self.assertTrue(all(result is None for result in results), results)
        final_state = collector.load_state(state_path)
        pending = collector.pending_session_ledger(cohort, final_state["sessionId"])
        events = collector.read_verified_ledger(pending, self.manifest)
        self.assertEqual(events[-1]["type"], "session_interruption")
        self.assertTrue(
            all(
                event["type"]
                in {"session", "tool", "material", "session_interruption"}
                for event in events
            )
        )

    def test_participant_stop_is_sealed_through_the_response_channel(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "participant-stop"
        )
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )
        consent_state = collector.load_state(state_path)
        with (
            self.command_patches(tool=True),
            mock.patch(
                "sys.stdin",
                io.StringIO(
                    json.dumps(
                        {
                            "requestId": collector.current_request_id(consent_state),
                            "participate": True,
                            "publicationConsent": "bounded-raw",
                        }
                    )
                ),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_respond(bound)
        state = collector.load_state(state_path)
        request_id = collector.current_request_id(state)
        with (
            self.command_patches(tool=True),
            mock.patch(
                "sys.stdin",
                io.StringIO(
                    json.dumps(
                        {"requestId": request_id, "terminalAction": "stop"}
                    )
                ),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_respond(bound)
        pending = collector.pending_session_ledger(cohort, state["sessionId"])
        events = collector.read_verified_ledger(pending, self.manifest)
        terminal = events[-1]
        self.assertEqual(terminal["type"], "session_interruption")
        self.assertEqual(terminal["reasonCode"], "participant-stop")
        self.assertEqual(
            terminal["terminalRequestSha256"],
            collector.terminal_request_sha256(
                state, "participant-stop", request_id
            ),
        )

    def test_formula_stop_erases_derived_content_before_pair_publication(self) -> None:
        bank_path, manifest_path, state_path, cohort, _start = self.start_case(
            "formula-stop"
        )
        bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=state_path,
            ledger=cohort,
            input="-",
        )

        def respond(payload: dict, target: argparse.Namespace = bound) -> dict:
            output = io.StringIO()
            with (
                self.command_patches(tool=True),
                mock.patch("sys.stdin", io.StringIO(json.dumps(payload))),
                contextlib.redirect_stdout(output),
            ):
                collector.command_respond(target)
            return json.loads(output.getvalue())

        state = collector.load_state(state_path)
        respond(
            {
                "requestId": collector.current_request_id(state),
                "participate": True,
                "publicationConsent": "bounded-raw",
            }
        )
        state = collector.load_state(state_path)
        respond(
            {
                "requestId": collector.current_request_id(state),
                "terminalAction": "stop",
            }
        )

        second_session_id = self.pair["collectionOrder"][1]
        second_state_path = Path(self.temporary.name) / "formula-stop-second.json"
        second_start = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=second_state_path,
            ledger=cohort,
            session_id=second_session_id,
            context_id=study.content_sha256("formula-stop-second-context"),
            model_identifier=self.pair["modelFamily"],
            backend_revision="unavailable",
            prior_state=state_path,
        )
        with self.command_patches(), contextlib.redirect_stdout(io.StringIO()):
            collector.command_start(second_start)
        second_bound = argparse.Namespace(
            bank=bank_path,
            manifest=manifest_path,
            state=second_state_path,
            ledger=cohort,
            input="-",
        )
        second_state = collector.load_state(second_state_path)
        respond(
            {
                "requestId": collector.current_request_id(second_state),
                "participate": True,
                "publicationConsent": "bounded-raw",
            },
            second_bound,
        )
        expression = "sin(4*x)"
        second_pending = collector.pending_session_ledger(cohort, second_session_id)
        second_session = next(
            session
            for session in self.pair["sessions"]
            if session["sessionId"] == second_session_id
        )
        self.assertEqual(second_session["condition"], study.CONDITIONS[0])
        actions = collector.session_actions(
            self.bank, self.manifest, second_session_id
        )
        while True:
            second_state = collector.load_state(second_state_path)
            events = collector.read_verified_ledger(second_pending, self.manifest)
            if any(
                event.get("room") == "formula-jam"
                and event.get("role") == "interaction"
                for event in events
            ):
                break
            respond(
                self.answer_pending(actions[second_state["cursor"]], second_state),
                second_bound,
            )
        self.assertIn(expression, second_pending.read_text(encoding="utf-8"))
        second_state = collector.load_state(second_state_path)
        stop_result = respond(
            {
                "requestId": collector.current_request_id(second_state),
                "terminalAction": "stop",
            },
            second_bound,
        )
        self.assertFalse(stop_result["responseContentRetainedByCollector"])
        published = collector.read_verified_ledger(cohort, self.manifest)
        self.assertNotIn(expression, json.dumps(published, sort_keys=True))
        self.assertEqual(
            sum(
                event.get("arguments") == study.ERASED_PARTICIPANT_TOOL_CONTENT
                for event in published
            ),
            1,
        )
        expression_bytes = expression.encode("utf-8")
        for path in Path(self.temporary.name).rglob("*"):
            if path.is_file():
                self.assertNotIn(expression_bytes, path.read_bytes(), path)

    def test_mcp_driver_rejects_misattributed_responses_and_timeouts(self) -> None:
        binary = Path(self.temporary.name) / "numinous-mcp-test"
        binary.write_bytes(b"test-binary")

        def completed_with_output(payload):
            def run(*_args, **kwargs):
                kwargs["stdout"].write(payload.encode("utf-8"))
                return collector.subprocess.CompletedProcess(
                    args=[str(binary)], returncode=0
                )

            return run

        malformed = completed_with_output(
            '{"jsonrpc":"2.0","id":2,"result":{}}\n'
            '{"jsonrpc":"2.0","id":1,"result":{}}\n'
        )
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
        ]
        with (
            mock.patch.object(
                collector.mcp_play,
                "_binary",
                return_value=fake_driver_artifact(binary),
            ),
            mock.patch.object(collector.mcp_play.subprocess, "run", side_effect=malformed),
            self.assertRaisesRegex(collector.mcp_play.McpPlayError, "request id 1"),
        ):
            collector.mcp_play._session(requests)
        for payload in (
            '{"jsonrpc":"2.0","id":1,"id":1,"result":{}}\n',
            '{"jsonrpc":"2.0","id":1,"result":{"value":NaN}}\n',
            '{"jsonrpc":"2.0","id":1,"result":'
            + "[" * 40
            + "0"
            + "]" * 40
            + "}\n",
        ):
            malformed_json = completed_with_output(payload)
            with (
                mock.patch.object(
                    collector.mcp_play,
                    "_binary",
                    return_value=fake_driver_artifact(binary),
                ),
                mock.patch.object(
                    collector.mcp_play.subprocess,
                    "run",
                    side_effect=malformed_json,
                ),
                self.assertRaises(collector.mcp_play.McpPlayError),
            ):
                collector.mcp_play._session(requests)
        with (
            mock.patch.object(
                collector.mcp_play.subprocess,
                "run",
                side_effect=collector.subprocess.TimeoutExpired("cargo", 120),
            ),
            self.assertRaisesRegex(collector.mcp_play.McpPlayError, "build exceeded"),
        ):
            collector.mcp_play._cargo_artifact(
                Path(self.temporary.name) / "timeout-target", "fixture-target", {}
            )

    def test_withdrawal_is_private_idempotent_and_bank_independent(self) -> None:
        cohort, pending, state_path, args = self.withdrawal_case("withdraw")
        original_remove = collector.remove_path
        interrupted = False

        def fail_first_cleanup(path: Path, label: str) -> None:
            nonlocal interrupted
            if not interrupted and "provisional" in label:
                interrupted = True
                raise collector.CollectorError("simulated cleanup crash")
            original_remove(path, label)

        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(state_path))
            ),
            mock.patch.object(collector, "remove_path", side_effect=fail_first_cleanup),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(collector.CollectorError, "cleanup crash"),
        ):
            collector.command_withdraw(args)
        outcome = collector.read_verified_ledger(cohort, self.manifest)
        self.assertEqual([event["type"] for event in outcome], ["withdrawal"])
        self.assertTrue(pending.exists())
        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(state_path))
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_withdraw(args)
        self.assertFalse(pending.exists())
        self.assertFalse(state_path.exists())
        outcome = collector.read_verified_ledger(cohort, self.manifest)
        self.assertEqual(sum(event["type"] == "withdrawal" for event in outcome), 1)
        self.assertNotIn("bounded private rationale", json.dumps(outcome))

    def test_withdrawal_append_failure_preserves_private_receipts(self) -> None:
        _cohort, pending, _state_path, args = self.withdrawal_case("safe-withdraw")
        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(_state_path))
            ),
            mock.patch.object(
                collector,
                "append_receipt_once",
                side_effect=collector.CollectorError("disk"),
            ),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(collector.CollectorError, "disk"),
        ):
            collector.command_withdraw(args)
        self.assertTrue(pending.exists())

    def test_withdrawal_never_claims_erasure_after_pair_publication(self) -> None:
        cohort, pending, _state_path, args = self.withdrawal_case("published-withdraw")
        pending_events = collector.read_verified_ledger(pending, self.manifest)
        for event in pending_events:
            collector.append_receipt(
                cohort, self.manifest, collector.clean_receipt_event(event)
            )
        with (
            mock.patch(
                "sys.stdin", io.StringIO(self.withdrawal_payload(_state_path))
            ),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(collector.CollectorError, "aggregation has begun"),
        ):
            collector.command_withdraw(args)
        published = collector.read_verified_ledger(cohort, self.manifest)
        self.assertIn("bounded private rationale", json.dumps(published))
        self.assertTrue(pending.exists())

    def test_recover_reclaims_exact_stale_lock_and_lease(self) -> None:
        cohort, pending, state_path, _args = self.withdrawal_case(
            "recover", include_response=False
        )
        collector.active_path(cohort).write_text(
            json.dumps(
                {
                    "schemaVersion": collector.ACTIVE_SCHEMA,
                    "sessionId": self.session_id,
                    "statePath": str(state_path),
                    "ownerPid": 2147483647,
                }
            ),
            encoding="utf-8",
        )
        lock = cohort.with_name(f"{cohort.name}.lock")
        lock.write_text(
            json.dumps(
                {
                    "schemaVersion": collector.LOCK_SCHEMA,
                    "ledger": str(cohort),
                    "ownerPid": 2147483647,
                }
            ),
            encoding="utf-8",
        )
        pending_lock = pending.with_name(f"{pending.name}.lock")
        pending_lock.write_text(
            json.dumps(
                {
                    "schemaVersion": collector.LOCK_SCHEMA,
                    "ledger": str(pending),
                    "ownerPid": 2147483647,
                }
            ),
            encoding="utf-8",
        )
        with (
            mock.patch.object(collector, "process_is_alive", return_value=False),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_recover(
                argparse.Namespace(state=state_path, ledger=cohort)
            )
        self.assertFalse(lock.exists())
        self.assertFalse(pending_lock.exists())
        lease = json.loads(collector.active_path(cohort).read_text(encoding="utf-8"))
        self.assertEqual(lease["ownerPid"], collector.os.getpid())

    def test_recovery_owns_transition_until_cleanup_finishes(self) -> None:
        cohort = Path(self.temporary.name) / "serialized-recovery.jsonl"
        transition = collector.transition_path(cohort)
        transition.write_text(
            json.dumps(
                {
                    "schemaVersion": collector.TRANSITION_SCHEMA,
                    "ledger": str(cohort),
                    "ownerPid": 2147483647,
                }
            ),
            encoding="utf-8",
        )
        recovery_entered = threading.Event()
        release_recovery = threading.Event()
        mutation_entered = threading.Event()
        thread_errors: list[BaseException] = []

        def recover() -> None:
            try:
                with collector.serialized_recovery(cohort) as reclaimed:
                    self.assertTrue(reclaimed)
                    recovery_entered.set()
                    self.assertTrue(release_recovery.wait(timeout=5))
            except BaseException as error:
                thread_errors.append(error)

        def mutate() -> None:
            try:
                with collector.serialized_transition(cohort):
                    mutation_entered.set()
            except BaseException as error:
                thread_errors.append(error)

        recovery_thread = threading.Thread(target=recover)
        mutation_thread = threading.Thread(target=mutate)
        recovery_thread.start()
        self.assertTrue(recovery_entered.wait(timeout=5))
        mutation_thread.start()
        self.assertFalse(mutation_entered.wait(timeout=0.1))
        release_recovery.set()
        recovery_thread.join(timeout=5)
        mutation_thread.join(timeout=15)
        self.assertFalse(recovery_thread.is_alive())
        self.assertFalse(mutation_thread.is_alive())
        self.assertEqual(thread_errors, [])
        self.assertTrue(mutation_entered.is_set())
        self.assertFalse(transition.exists())
        self.assertFalse(collector.recovery_path(cohort).exists())

    def test_allocated_runtime_identity_is_exact(self) -> None:
        header = self.header()
        header["modelIdentifier"] = "different-runtime"
        with self.assertRaisesRegex(study.StudyError, "model identifier mismatch"):
            study.validate_session_header(header, self.pair, self.session)
        header = self.header()
        header["studySourceSha256"] = "3" * 64
        with self.assertRaisesRegex(study.StudyError, "study source mismatch"):
            study.validate_session_header(header, self.pair, self.session)

    def test_interruption_projection_destroys_all_response_content(self) -> None:
        self.advance_to_probe()
        action = collector.session_actions(self.bank, self.manifest, self.session_id)[
            self.state["cursor"]
        ]
        collector.record_response(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            self.answer_pending(action),
            tool_caller=fake_tool_call,
        )
        events = collector.read_verified_ledger(self.ledger, self.manifest)
        expression = "sin(4*x)"
        terminal = {
            "schemaVersion": study.EVENT_SCHEMA,
            "type": "session_interruption",
            "sessionId": self.session_id,
            "stage": "immediate",
            "reasonCode": "participant-stop",
            "terminalRequestSha256": "4" * 64,
        }
        projection = collector.interruption_projection(events, terminal)
        retained_types = {event["type"] for event in projection}
        self.assertEqual(
            retained_types, {"session", "tool", "material", "session_interruption"}
        )
        serialized = json.dumps(projection, sort_keys=True)
        self.assertNotIn("bounded public response", serialized)
        self.assertNotIn('"answer"', serialized)
        self.assertNotIn(expression, serialized)
        erased = [
            event
            for event in projection
            if event.get("arguments") == study.ERASED_PARTICIPANT_TOOL_CONTENT
        ]
        self.assertEqual(len(erased), 1)
        self.assertEqual(
            erased[0]["structuredResult"], study.ERASED_PARTICIPANT_TOOL_CONTENT
        )
        self.assertEqual(erased[0]["visibleText"], "")

    def test_mcp_result_projection_rejects_schema_drift(self) -> None:
        initialization, result = fake_tool_call("reveal_room", {"id": "times-tables"})
        result["structuredContent"]["unexpected"] = "value"
        with self.assertRaisesRegex(collector.CollectorError, "schema differs"):
            collector.project_mcp_result(
                "reveal_room",
                result,
                {"id": "times-tables"},
                initialization["serverInfo"],
            )

        arguments = {"expr": "sin(2*x)"}
        initialization, result = fake_tool_call("plot_expression", arguments)
        result["structuredContent"]["kind"] = "parametric"
        with self.assertRaisesRegex(collector.CollectorError, "kind must be graph"):
            collector.project_mcp_result(
                "plot_expression",
                result,
                arguments,
                initialization["serverInfo"],
            )

    def test_mcp_result_projection_binds_current_server_identity(self) -> None:
        arguments = {"id": "times-tables"}
        initialization, result = fake_tool_call("reveal_room", arguments)
        result["_meta"][collector.mcp_play.SERVER_INFO_META_KEY]["version"] = "0.3.0"
        with self.assertRaisesRegex(collector.CollectorError, "server identity differs"):
            collector.project_mcp_result(
                "reveal_room",
                result,
                arguments,
                initialization["serverInfo"],
            )

        initialization, result = fake_tool_call("reveal_room", arguments)
        result["resultType"] = "stream"
        with self.assertRaisesRegex(collector.CollectorError, "result is incomplete"):
            collector.project_mcp_result(
                "reveal_room",
                result,
                arguments,
                initialization["serverInfo"],
            )

    def test_mcp_result_projection_rejects_type_and_identity_drift(self) -> None:
        cases = []
        for tool, arguments in (
            ("play_room", {"id": "times-tables", "t": 0.25}),
            ("reveal_room", {"id": "times-tables"}),
            ("plot_expression", {"expr": "sin(2*x)"}),
        ):
            initialization, result = fake_tool_call(tool, arguments)
            cases.append((tool, arguments, initialization, result))
        cases[0][3]["structuredContent"]["room"] = 42
        cases[1][3]["structuredContent"]["reveal"] = []
        cases[2][3]["structuredContent"]["valid"] = "yes"
        for tool, arguments, initialization, result in cases:
            with self.subTest(tool=tool):
                with self.assertRaises(collector.CollectorError):
                    collector.project_mcp_result(
                        tool, result, arguments, initialization["serverInfo"]
                    )

        initialization, result = fake_tool_call(
            "plot_expression", {"expr": "sin(2*x)"}
        )
        result["structuredContent"]["xmax"] = float("inf")
        with self.assertRaisesRegex(collector.CollectorError, "finite"):
            collector.project_mcp_result(
                "plot_expression",
                result,
                {"expr": "sin(2*x)"},
                initialization["serverInfo"],
            )

    def test_formula_construction_schema_matches_the_real_mcp_parser(self) -> None:
        room = study.encounter_rooms()["formula-jam"]
        accepted = ("sin(2*x)", "cos(2*x)", "sin(99*x)", "cos(99*x)")
        requests = [
            {
                "jsonrpc": "2.0",
                "id": index,
                "method": "tools/call",
                "params": {
                    "name": "plot_expression",
                    "arguments": {"expr": expression},
                },
            }
            for index, expression in enumerate(accepted, start=2)
        ]
        responses, _binary_sha256 = collector.mcp_play._discover(requests)
        discovery = collector.mcp_play._response_result(responses[0], "server/discover")
        server_info = discovery["_meta"][collector.mcp_play.SERVER_INFO_META_KEY]
        for expression, response in zip(accepted, responses[1:], strict=True):
            result = collector.mcp_play._response_result(response, "plot_expression")
            self.assertFalse(result["isError"])
            self.assertEqual(study.validate_generation_answer(room, expression), expression)
            projection = collector.project_mcp_result(
                "plot_expression", result, {"expr": expression}, server_info
            )
            self.assertEqual(projection["expression"], expression)
        for expression in ("sinx", "sin(x) x", "sin(x)^", "cos(x)/0", "sincosx"):
            with self.subTest(expression=expression):
                with self.assertRaises(study.StudyError):
                    study.validate_generation_answer(room, expression)

    def test_full_session_uses_twenty_real_isolated_tool_processes(self) -> None:
        calls = []

        def counted_isolated_call(tool: str, arguments: dict) -> tuple[dict, dict]:
            initialization, result = collector.mcp_play.isolated_tool_call(
                tool, arguments
            )
            build_receipt = dict(initialization["binaryBuildReceipt"])
            build_receipt.update(
                {
                    "schemaVersion": collector.mcp_play.BUILD_RECEIPT_SCHEMA,
                    "sourceRevision": RUNNER_REVISION,
                    "studySourceSha256": RUNNER_SOURCE_SHA256,
                    "sourcePolicy": "verified-clean-commit-before-and-after",
                    "targetDirectoryPolicy": "fresh-explicit-private",
                }
            )
            initialization = {
                **initialization,
                "binaryBuildReceipt": build_receipt,
            }
            calls.append(
                (tool, arguments, initialization["numinousBinarySha256"])
            )
            return initialization, result

        output = collector.advance(
            self.state,
            self.bank,
            self.manifest,
            self.ledger,
            tool_caller=counted_isolated_call,
        )
        actions = collector.session_actions(
            self.bank, self.manifest, self.session_id
        )
        while output["status"] == "awaiting_response":
            action = actions[self.state["cursor"]]
            output = collector.record_response(
                self.state,
                self.bank,
                self.manifest,
                self.ledger,
                self.answer_pending(action),
                tool_caller=counted_isolated_call,
            )

        self.assertEqual(output["status"], "complete")
        events = collector.read_verified_ledger(self.ledger, self.manifest)
        tool_events = [event for event in events if event["type"] == "tool"]
        self.assertEqual(len(calls), 20)
        self.assertEqual(len(tool_events), 20)
        self.assertEqual(len({call[2] for call in calls}), 1)
        self.assertEqual(
            sum("goal" in event["structuredResult"] for event in tool_events), 16
        )
        self.assertEqual(
            sum(
                event["structuredResult"].get("goal", object()) is None
                for event in tool_events
            ),
            12,
        )

    def test_attempt_start_receipt_is_required_and_exactly_bound(self) -> None:
        commitment = study.attempt_start_commitment(
            phase="calibration",
            root_sha256="3" * 64,
            start_key="1",
            model_identifier=study.MODEL_FAMILIES[0],
            context_id="4" * 64,
            backend_revision="calibrated-r1",
            runner_revision=RUNNER_REVISION,
            runner_source_sha256=RUNNER_SOURCE_SHA256,
        )
        with self.assertRaisesRegex(
            collector.CollectorError, f"commitment {commitment}"
        ):
            collector.require_attempt_start_receipt(None, commitment)
        receipt = {
            "schemaVersion": study.ATTEMPT_START_RECEIPT_SCHEMA,
            "protocolVersion": study.PROTOCOL_VERSION,
            "startCommitmentSha256": commitment,
            "mechanism": "independent-reconciler-ledger",
            "witnessId": study.content_sha256("independent fixture witness"),
            "witnessedAt": "2026-07-31T12:00:00Z",
            "recordLocator": "reconciler-ledger-record-0001",
            "recordSha256": study.content_sha256("witness record payload"),
            "attestation": study.ATTEMPT_START_ATTESTATION,
        }
        receipt_path = Path(self.temporary.name) / "attempt-start-receipt.json"
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        self.assertEqual(
            collector.require_attempt_start_receipt(receipt_path, commitment),
            study.content_sha256(receipt),
        )
        with self.assertRaisesRegex(collector.CollectorError, "invalid or differs"):
            collector.require_attempt_start_receipt(receipt_path, "5" * 64)

        bank_path = Path(self.temporary.name) / "start-gated-bank.json"
        ledger = Path(self.temporary.name) / "start-gated-calibration.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        calibration_root = study.calibration_receipt_commitment(
            self.bank, RUNNER_REVISION, RUNNER_SOURCE_SHA256
        )
        first_cell = calibration_root["cells"][0]
        context_id = study.content_sha256("start-gated fresh context")
        command_commitment = study.attempt_start_commitment(
            phase="calibration",
            root_sha256=study.content_sha256(calibration_root),
            start_key=str(first_cell["deliveryOrdinal"]),
            model_identifier=first_cell["modelIdentifier"],
            context_id=context_id,
            backend_revision="calibrated-r1",
            runner_revision=RUNNER_REVISION,
            runner_source_sha256=RUNNER_SOURCE_SHA256,
        )
        args = argparse.Namespace(
            bank=bank_path,
            ledger=ledger,
            model_identifier=first_cell["modelIdentifier"],
            context_id=context_id,
            backend_revision="calibrated-r1",
        )
        with (
            self.command_patches(receipt=False),
            self.assertRaisesRegex(collector.CollectorError, command_commitment),
        ):
            collector.command_calibration_next(args)
        self.assertFalse(ledger.exists())
        self.assertFalse(collector.transition_path(ledger).exists())
        self.assertFalse(ledger.with_name(f"{ledger.name}.lock").exists())
        receipt["startCommitmentSha256"] = command_commitment
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        args.start_receipt = receipt_path
        with (
            self.command_patches(receipt=False),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_calibration_next(args)
        events = collector.read_verified_ledger(ledger, calibration_root)
        self.assertEqual(
            events[0]["attemptStartReceiptSha256"], study.content_sha256(receipt)
        )

    def test_calibration_delivery_is_sealed_before_one_bound_response(self) -> None:
        root = Path(self.temporary.name)
        bank_path = root / "calibration-bank.json"
        ledger = root / "calibration-ledger.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        first_cell = study.calibration_cells(self.bank)[0]
        context_id = study.content_sha256("fresh calibration context")
        next_args = argparse.Namespace(
            bank=bank_path,
            ledger=ledger,
            model_identifier=first_cell["modelIdentifier"],
            context_id=context_id,
            backend_revision="calibrated-r1",
        )
        packet_output = io.StringIO()
        with self.command_patches(), contextlib.redirect_stdout(packet_output):
            collector.command_calibration_next(next_args)
        packet = json.loads(packet_output.getvalue())
        serialized_packet = json.dumps(packet, sort_keys=True)
        self.assertNotIn("oracle", serialized_packet)
        self.assertEqual(packet["publicProbe"]["probeId"], first_cell["probeId"])

        with (
            mock.patch.object(
                collector, "repository_commit", return_value="3" * 40
            ),
            mock.patch.object(
                collector, "study_source_sha256", return_value="4" * 64
            ),
            self.assertRaisesRegex(
                collector.CollectorError, "receipt commitment differs"
            ),
        ):
            collector.command_calibration_respond(
                argparse.Namespace(bank=bank_path, ledger=ledger, input="-")
            )

        with (
            self.command_patches(),
            mock.patch(
                "sys.stdin",
                io.StringIO(
                    json.dumps(
                        {
                            "requestId": packet["responseRequest"]["requestId"],
                            "answer": 0,
                        }
                    )
                ),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_calibration_respond(
                argparse.Namespace(bank=bank_path, ledger=ledger, input="-")
            )
        commitment = study.calibration_receipt_commitment(
            self.bank, RUNNER_REVISION, RUNNER_SOURCE_SHA256
        )
        progress = study.calibration_progress(
            self.bank,
            collector.read_verified_ledger(ledger, commitment),
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
            require_complete=False,
        )
        self.assertEqual(len(progress["records"]), 1)
        self.assertEqual(progress["records"][0]["contextId"], context_id)
        self.assertEqual(progress["records"][0]["runnerRevision"], RUNNER_REVISION)
        self.assertEqual(
            progress["records"][0]["runnerSourceSha256"],
            RUNNER_SOURCE_SHA256,
        )
        self.assertEqual(progress["nextCell"]["deliveryOrdinal"], 2)
        self.assertTrue(collector.receipt_anchor_path(ledger).exists())

    def test_calibration_recovery_finishes_one_dead_receipt_transaction(self) -> None:
        root = Path(self.temporary.name)
        bank_path = root / "calibration-recovery-bank.json"
        ledger = root / "calibration-recovery-ledger.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        first_cell = study.calibration_cells(self.bank)[0]
        next_args = argparse.Namespace(
            bank=bank_path,
            ledger=ledger,
            model_identifier=first_cell["modelIdentifier"],
            context_id=study.content_sha256("recoverable calibration context"),
            backend_revision="calibrated-r1",
        )
        packet_output = io.StringIO()
        with self.command_patches(), contextlib.redirect_stdout(packet_output):
            collector.command_calibration_next(next_args)
        packet = json.loads(packet_output.getvalue())
        original_write = collector.write_json_atomic

        def fail_anchor(path: Path, value: object, description: str) -> None:
            if description == "collector receipt anchor":
                raise collector.CollectorError("simulated calibration anchor failure")
            original_write(path, value, description)

        with (
            self.command_patches(),
            mock.patch.object(collector, "write_json_atomic", side_effect=fail_anchor),
            mock.patch(
                "sys.stdin",
                io.StringIO(
                    json.dumps(
                        {
                            "requestId": packet["responseRequest"]["requestId"],
                            "answer": 0,
                        }
                    )
                ),
            ),
            self.assertRaisesRegex(collector.CollectorError, "anchor failure"),
        ):
            collector.command_calibration_respond(
                argparse.Namespace(bank=bank_path, ledger=ledger, input="-")
            )
        dead_pid = 2**31 - 1
        collector.write_json_atomic(
            collector.transition_path(ledger),
            {
                "schemaVersion": collector.TRANSITION_SCHEMA,
                "ledger": str(ledger),
                "ownerPid": dead_pid,
            },
            "simulated dead calibration transition",
        )
        collector.write_json_atomic(
            ledger.with_name(f"{ledger.name}.lock"),
            {
                "schemaVersion": collector.LOCK_SCHEMA,
                "ledger": str(ledger),
                "ownerPid": dead_pid,
            },
            "simulated dead calibration ledger lock",
        )
        recovery_output = io.StringIO()
        with self.command_patches(), contextlib.redirect_stdout(recovery_output):
            collector.command_calibration_recover(
                argparse.Namespace(bank=bank_path, ledger=ledger)
            )
        result = json.loads(recovery_output.getvalue())
        self.assertEqual(
            result["markers"],
            ["collector-transition", "ledger-lock", "receipt-transaction"],
        )
        commitment = study.calibration_receipt_commitment(
            self.bank, RUNNER_REVISION, RUNNER_SOURCE_SHA256
        )
        progress = study.calibration_progress(
            self.bank,
            collector.read_verified_ledger(ledger, commitment),
            RUNNER_REVISION,
            RUNNER_SOURCE_SHA256,
            require_complete=False,
        )
        self.assertEqual(len(progress["records"]), 1)
        self.assertFalse(collector.transition_path(ledger).exists())
        self.assertFalse(collector.receipt_transaction_path(ledger).exists())

    def test_hostile_numeric_response_is_a_bounded_study_error(self) -> None:
        self.advance_to_probe()
        with self.assertRaisesRegex(study.StudyError, "canonical finite JSON"):
            collector.record_response(
                self.state,
                self.bank,
                self.manifest,
                self.ledger,
                {
                    "requestId": collector.current_request_id(self.state),
                    "answer": 10**10_000,
                },
                tool_caller=fake_tool_call,
            )

    def test_terminal_pair_is_published_in_frozen_order(self) -> None:
        cohort = Path(self.temporary.name) / "settled.jsonl"
        expected_session_ids = self.pair["collectionOrder"]
        for session_id in expected_session_ids:
            pair, session = study.manifest_indexes(self.manifest)[1][session_id]
            path = collector.pending_session_ledger(cohort, session_id)
            header = self.header()
            header.update(
                {
                    "sessionId": session_id,
                    "condition": session["condition"],
                    "contextId": study.content_sha256(f"{session_id}-settlement"),
                }
            )
            first_action = collector.session_actions(
                self.bank, self.manifest, session_id
            )[0]
            call = first_action["call"]
            initialization, result = fake_tool_call(call["tool"], call["arguments"])
            tool = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "tool",
                "sessionId": session_id,
                "room": first_action["room"],
                "sequence": first_action["sequence"],
                "role": call["role"],
                "tool": call["tool"],
                "arguments": call["arguments"],
                "structuredResult": collector.project_mcp_result(
                    call["tool"],
                    result,
                    call["arguments"],
                    initialization["serverInfo"],
                ),
                "visibleText": collector.tool_text(result),
                "toolOutcome": "success",
                "binarySha256": "2" * 64,
                "binaryBuildReceipt": binary_build_receipt(),
            }
            terminal = {
                "schemaVersion": study.EVENT_SCHEMA,
                "type": "session_interruption",
                "sessionId": session_id,
                "stage": "encounter",
                "reasonCode": "participant-stop",
                "terminalRequestSha256": "4" * 64,
            }
            for event in (header, tool, terminal):
                collector.append_receipt(path, self.manifest, event)
        with (
            mock.patch.object(
                collector,
                "write_receipts_atomic",
                side_effect=collector.CollectorError("simulated publication failure"),
            ),
            self.assertRaisesRegex(
                collector.CollectorError, "simulated publication failure"
            ),
        ):
            collector.publish_pair(cohort, self.manifest, self.pair)
        self.assertFalse(cohort.exists())
        self.assertTrue(
            all(
                collector.pending_session_ledger(cohort, session_id).exists()
                for session_id in expected_session_ids
            )
        )
        self.assertTrue(collector.publish_pair(cohort, self.manifest, self.pair))
        settled = collector.read_verified_ledger(cohort, self.manifest)
        observed = [
            event["sessionId"] for event in settled if event["type"] == "session"
        ]
        self.assertEqual(observed, expected_session_ids)
        self.assertFalse(collector.pending_root(cohort).exists())
        self.assertTrue(collector.publish_pair(cohort, self.manifest, self.pair))

    def test_command_lifecycle_settles_only_after_both_sessions(self) -> None:
        root = Path(self.temporary.name)
        bank_path = root / "private-bank.json"
        manifest_path = root / "allocation.json"
        cohort = root / "cohort.jsonl"
        bank_path.write_text(json.dumps(self.bank), encoding="utf-8")
        manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")

        def collect(
            session_id: str, ordinal: int, prior_state: Path | None = None
        ) -> tuple[Path, dict]:
            state_path = root / f"state-{ordinal}.json"
            start = argparse.Namespace(
                bank=bank_path,
                manifest=manifest_path,
                state=state_path,
                ledger=cohort,
                session_id=session_id,
                consent=True,
                publication_consent="bounded-raw",
                context_id=study.content_sha256(f"fresh-command-context-{ordinal}"),
                model_identifier=self.pair["modelFamily"],
                backend_revision="unavailable",
                prior_state=prior_state,
            )
            with (
                mock.patch.object(
                    collector, "repository_commit", return_value="1" * 40
                ),
                mock.patch.object(
                    collector,
                    "study_source_sha256",
                    return_value=RUNNER_SOURCE_SHA256,
                ),
                mock.patch.object(
                    collector,
                    "require_attempt_start_receipt",
                    return_value="7" * 64,
                ),
                mock.patch.object(collector, "require_committed_file"),
                mock.patch.object(
                    collector.mcp_play, "isolated_tool_call", side_effect=fake_tool_call
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                collector.command_start(start)
                consent_state = collector.load_state(state_path)
                with mock.patch(
                    "sys.stdin",
                    io.StringIO(
                        json.dumps(
                            {
                                "requestId": collector.current_request_id(
                                    consent_state
                                ),
                                "participate": True,
                                "publicationConsent": "bounded-raw",
                            }
                        )
                    ),
                ):
                    collector.command_respond(
                        argparse.Namespace(
                            bank=bank_path,
                            manifest=manifest_path,
                            state=state_path,
                            ledger=cohort,
                            input="-",
                        )
                    )
                final_output: dict = {}
                while True:
                    state = collector.load_state(state_path)
                    if state["complete"]:
                        break
                    action = collector.session_actions(
                        self.bank, self.manifest, session_id
                    )[state["cursor"]]
                    response_output = io.StringIO()
                    with (
                        mock.patch(
                            "sys.stdin",
                            io.StringIO(json.dumps(self.answer_pending(action, state))),
                        ),
                        contextlib.redirect_stdout(response_output),
                    ):
                        collector.command_respond(
                            argparse.Namespace(
                                bank=bank_path,
                                manifest=manifest_path,
                                state=state_path,
                                ledger=cohort,
                                input="-",
                            )
                        )
                    final_output = json.loads(response_output.getvalue())
                return state_path, final_output

        first, second = self.pair["collectionOrder"]
        first_state_path, first_output = collect(first, 1)
        self.assertFalse(cohort.exists())
        self.assertTrue(collector.pending_session_ledger(cohort, first).exists())
        withdrawal = first_output["withdrawalRequest"]
        self.assertEqual(withdrawal["availableUntil"], "pair-aggregation")

        source_pending = collector.pending_session_ledger(cohort, first)
        for exposed in (False, True):
            suffix = "exposed" if exposed else "pre-exposure"
            withdrawal_cohort = root / f"withdrawal-{suffix}.jsonl"
            withdrawal_pending = collector.pending_session_ledger(
                withdrawal_cohort, first
            )
            withdrawal_pending.parent.mkdir(parents=True)
            withdrawal_pending.write_bytes(source_pending.read_bytes())
            collector.receipt_anchor_path(withdrawal_pending).write_bytes(
                collector.receipt_anchor_path(source_pending).read_bytes()
            )
            withdrawal_state_path = root / f"withdrawal-{suffix}-first-state.json"
            withdrawal_state = collector.load_state(first_state_path)
            withdrawal_state.update(
                {
                    "sessionLedger": str(withdrawal_pending),
                    "cohortLedger": str(withdrawal_cohort),
                    "pairStatePaths": [str(withdrawal_state_path)],
                }
            )
            collector.write_state(withdrawal_state_path, withdrawal_state)
            second_withdrawal_state = root / f"withdrawal-{suffix}-second-state.json"
            second_start = argparse.Namespace(
                bank=bank_path,
                manifest=manifest_path,
                state=second_withdrawal_state,
                ledger=withdrawal_cohort,
                session_id=second,
                context_id=study.content_sha256(
                    f"withdrawal-{suffix}-second-context"
                ),
                model_identifier=self.pair["modelFamily"],
                backend_revision="unavailable",
                prior_state=withdrawal_state_path,
            )
            with self.command_patches(tool=True), contextlib.redirect_stdout(
                io.StringIO()
            ):
                collector.command_start(second_start)
                if exposed:
                    consent_state = collector.load_state(second_withdrawal_state)
                    with mock.patch(
                        "sys.stdin",
                        io.StringIO(
                            json.dumps(
                                {
                                    "requestId": collector.current_request_id(
                                        consent_state
                                    ),
                                    "participate": True,
                                    "publicationConsent": "bounded-raw",
                                }
                            )
                        ),
                    ):
                        collector.command_respond(
                            argparse.Namespace(
                                bank=bank_path,
                                manifest=manifest_path,
                                state=second_withdrawal_state,
                                ledger=withdrawal_cohort,
                                input="-",
                            )
                        )
            with (
                mock.patch(
                    "sys.stdin",
                    io.StringIO(
                        json.dumps(
                            {
                                "requestId": withdrawal["requestId"],
                                "terminalAction": "withdraw",
                            }
                        )
                    ),
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                collector.command_withdraw(
                    argparse.Namespace(
                        state=withdrawal_state_path,
                        ledger=withdrawal_cohort,
                        input="-",
                    )
                )
            self.assertFalse(withdrawal_pending.exists())
            self.assertFalse(second_withdrawal_state.exists())
            self.assertFalse(collector.active_path(withdrawal_cohort).exists())
            self.assertEqual(
                collector.read_verified_ledger(
                    withdrawal_cohort, self.manifest
                )[0]["type"],
                "withdrawal",
            )

        second_state_path, second_output = collect(second, 2, first_state_path)
        self.assertIsNone(second_output["withdrawalRequest"])
        events = collector.read_verified_ledger(cohort, self.manifest)
        headers = [event["sessionId"] for event in events if event["type"] == "session"]
        self.assertEqual(headers, [first, second])
        self.assertFalse(collector.pending_root(cohort).exists())
        self.assertFalse(collector.active_path(cohort).exists())

        collector.active_path(cohort).write_text(
            json.dumps(
                {
                    "schemaVersion": collector.ACTIVE_SCHEMA,
                    "sessionId": second,
                    "statePath": str(second_state_path),
                    "ownerPid": 2147483647,
                }
            ),
            encoding="utf-8",
        )
        with (
            mock.patch.object(collector, "process_is_alive", return_value=False),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            collector.command_recover(
                argparse.Namespace(state=second_state_path, ledger=cohort)
            )
        self.assertFalse(collector.active_path(cohort).exists())
        status_output = io.StringIO()
        with self.command_patches(), contextlib.redirect_stdout(status_output):
            collector.command_status(
                argparse.Namespace(
                    bank=bank_path,
                    manifest=manifest_path,
                    state=second_state_path,
                    ledger=cohort,
                )
            )
        recovered_status = json.loads(status_output.getvalue())
        self.assertEqual(recovered_status["status"], "complete")
        self.assertIsNone(recovered_status["withdrawalRequest"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
