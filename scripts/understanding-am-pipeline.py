#!/usr/bin/env python3
"""Agent-and-machine Understanding Alpha registration dry-run.

Writes a registration commitment document for the am-track, runs dual automated
auditors, and exits non-zero on method failure. This is preparation evidence,
not a calibrated bank and not a completed 20-pair cohort result.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "docs" / "evidence" / "understanding-0.4"
FIXTURE_BANK = ROOT / "scripts" / "understanding-probes.fixture.json"
AUDITOR = ROOT / "scripts" / "understanding-am-auditor.py"
RUNNER = ROOT / "scripts" / "understanding-study.py"


def bank_commitment(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_registration(*, dry_run: bool) -> dict[str, Any]:
    commitment = bank_commitment(FIXTURE_BANK)
    # Fixture bank is public and not admissible for qualifying collection.
    # Dry-run registration documents the method surface only.
    return {
        "schemaVersion": "numinous-understanding-registration-v1",
        "protocolVersion": "0.4-v6",
        "track": "agent-and-machine",
        "runnerVersion": "numinous-understanding-runner-v6",
        "bankCommitmentSha256": commitment,
        "bankKind": "public-fixture-dry-run" if dry_run else "concealed",
        "allocationSeed": "numinous-understanding-alpha-v1",
        "plannedPrimaryPairs": 20,
        "plannedReservePairs": 4,
        "isolation": "fresh-mcp-process-per-public-call",
        "privacyBoundary": [
            "no-hidden-reasoning",
            "no-host-paths",
            "no-private-progression",
            "bounded-public-mcp-projections-only",
        ],
        "automatedReviewers": ["A", "B"],
        "humanPanelRequired": False,
        "qualifyingCohortComplete": False,
        # Fixed for dry-run reproducibility; qualifying registration uses live time.
        "registeredAt": "2026-08-02T00:00:00Z",
        "attestation": (
            "This method commitment was recorded before any qualifying stimulus "
            "exposure on the agent-and-machine track."
        ),
        "limitations": [
            "Dry-run uses the public fixture bank, which cannot authorize qualifying collection.",
            "Calibration deliveries and real model-family sessions remain open.",
            "Dual automated auditors replace human dual-review only on the am-track.",
        ],
    }


def run_auditors(path: Path) -> dict[str, Any]:
    process = subprocess.run(
        [sys.executable, str(AUDITOR), "both", str(path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    payload: dict[str, Any]
    try:
        payload = json.loads(process.stdout)
    except json.JSONDecodeError:
        payload = {
            "passed": False,
            "results": [],
            "stderr": process.stderr,
            "stdout": process.stdout,
        }
    payload["exitCode"] = process.returncode
    return payload


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--write",
        action="store_true",
        help="write registration JSON under docs/evidence/understanding-0.4/",
    )
    parser.add_argument(
        "--check-only",
        type=Path,
        help="audit an existing registration path without rewriting",
    )
    args = parser.parse_args(argv)

    if args.check_only is not None:
        audit = run_auditors(args.check_only)
        print(json.dumps(audit, indent=2))
        return 0 if audit.get("passed") and audit.get("exitCode") == 0 else 1

    registration = build_registration(dry_run=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    path = EVIDENCE / "registration-dry-run.json"
    if args.write or not path.is_file():
        path.write_text(json.dumps(registration, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {path}")
    else:
        # Keep committed dry-run stable unless --write forces refresh of timestamp.
        path = EVIDENCE / "registration-dry-run.json"
        print(f"using existing {path}")

    # Always re-read the on-disk artifact so auditors see committed bytes.
    audit = run_auditors(path)
    summary = {
        "suite": "understanding-am-pipeline-dry-run",
        "passed": bool(audit.get("passed")),
        "registration": str(path.relative_to(ROOT)).replace("\\", "/"),
        "audit": audit,
        "evidence_class": "agent-machine-method-prep",
        "qualifyingCohortComplete": False,
    }
    summary_path = EVIDENCE / "pipeline-dry-run-summary.json"
    if args.write:
        summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {summary_path}")
    print(json.dumps(summary, indent=2))
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
