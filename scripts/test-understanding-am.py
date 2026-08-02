#!/usr/bin/env python3
"""Regressions for Understanding Alpha am-track auditors and dry-run pipeline."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def load(name: str, relative: str):
    path = ROOT / relative
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {relative}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


auditor = load("numinous_understanding_am_auditor", "scripts/understanding-am-auditor.py")
pipeline = load("numinous_understanding_am_pipeline", "scripts/understanding-am-pipeline.py")


class AuditorTests(unittest.TestCase):
    def valid_registration(self) -> dict:
        return {
            "schemaVersion": "numinous-understanding-registration-v1",
            "protocolVersion": "0.4-v5",
            "track": "agent-and-machine",
            "runnerVersion": "numinous-understanding-runner-v5",
            "bankCommitmentSha256": "a" * 64,
            "allocationSeed": "numinous-understanding-alpha-v1",
            "plannedPrimaryPairs": 20,
            "plannedReservePairs": 4,
            "isolation": "fresh-mcp-process-per-public-call",
            "privacyBoundary": ["no-hidden-reasoning", "no-host-paths"],
            "automatedReviewers": ["A", "B"],
            "humanPanelRequired": False,
            "qualifyingCohortComplete": False,
            "registeredAt": "2026-08-02T00:00:00Z",
            "attestation": "Recorded before any qualifying stimulus exposure.",
        }

    def test_auditor_a_accepts_valid(self) -> None:
        self.assertEqual(auditor.auditor_a(self.valid_registration()), [])

    def test_auditor_a_rejects_cohort_claim(self) -> None:
        reg = self.valid_registration()
        reg["qualifyingCohortComplete"] = True
        defects = auditor.auditor_a(reg)
        self.assertTrue(any("qualifyingCohortComplete" in d for d in defects))

    def test_auditor_b_requires_isolation_and_reviewers(self) -> None:
        reg = self.valid_registration()
        reg["isolation"] = "shared-process"
        reg["automatedReviewers"] = ["A"]
        defects = auditor.auditor_b(reg)
        self.assertTrue(any("isolation" in d for d in defects))
        self.assertTrue(any("automatedReviewers" in d for d in defects))

    def test_pipeline_build_registration_is_dry_run(self) -> None:
        reg = pipeline.build_registration(dry_run=True)
        self.assertEqual(reg["track"], "agent-and-machine")
        self.assertEqual(reg["bankKind"], "public-fixture-dry-run")
        self.assertFalse(reg["qualifyingCohortComplete"])
        self.assertEqual(auditor.auditor_a(reg), [])
        self.assertEqual(auditor.auditor_b(reg), [])

    def test_run_auditors_roundtrip_on_temp_file(self) -> None:
        reg = pipeline.build_registration(dry_run=True)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reg.json"
            path.write_text(json.dumps(reg), encoding="utf-8")
            result = pipeline.run_auditors(path)
            self.assertTrue(result.get("passed"))
            self.assertEqual(result.get("exitCode"), 0)


if __name__ == "__main__":
    raise SystemExit(unittest.main())
