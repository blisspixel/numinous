#!/usr/bin/env python3
"""Independent automated auditors for Understanding Alpha method artifacts.

Two criterion packs (A and B) re-read the same registration / ledger JSON and
fail closed on method violations. These replace human dual-review for the
agent-and-machine track only. They do not score comprehension and they do not
claim a qualifying cohort result by themselves.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent

REQUIRED_REGISTRATION_KEYS = (
    "schemaVersion",
    "protocolVersion",
    "track",
    "runnerVersion",
    "bankCommitmentSha256",
    "allocationSeed",
    "registeredAt",
    "attestation",
)


def load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("artifact must be a JSON object")
    return data


def auditor_a(artifact: dict[str, Any]) -> list[str]:
    """Pack A: identity, track labeling, and registration completeness."""
    defects: list[str] = []
    for key in REQUIRED_REGISTRATION_KEYS:
        if key not in artifact:
            defects.append(f"missing registration field {key}")
    if artifact.get("track") != "agent-and-machine":
        defects.append("track must be agent-and-machine")
    if artifact.get("protocolVersion") != "0.4-v5":
        defects.append("protocolVersion must be 0.4-v5")
    if artifact.get("schemaVersion") != "numinous-understanding-registration-v1":
        defects.append("unexpected schemaVersion")
    commitment = artifact.get("bankCommitmentSha256")
    if not isinstance(commitment, str) or len(commitment) != 64:
        defects.append("bankCommitmentSha256 must be 64 hex chars")
    elif any(ch not in "0123456789abcdef" for ch in commitment):
        defects.append("bankCommitmentSha256 must be lowercase hex")
    attestation = artifact.get("attestation")
    if not isinstance(attestation, str) or "before" not in attestation.lower():
        defects.append("attestation must declare pre-exposure recording")
    if artifact.get("qualifyingCohortComplete") is True:
        # Explicit ban: registration alone cannot claim completion.
        defects.append("registration must not claim qualifyingCohortComplete")
    return defects


def auditor_b(artifact: dict[str, Any]) -> list[str]:
    """Pack B: allocation shape, isolation claims, and privacy boundary."""
    defects: list[str] = []
    planned = artifact.get("plannedPrimaryPairs")
    if planned != 20:
        defects.append("plannedPrimaryPairs must be 20")
    reserves = artifact.get("plannedReservePairs")
    if reserves != 4:
        defects.append("plannedReservePairs must be 4")
    isolation = artifact.get("isolation")
    if isolation != "fresh-mcp-process-per-public-call":
        defects.append("isolation must be fresh-mcp-process-per-public-call")
    privacy = artifact.get("privacyBoundary")
    if not isinstance(privacy, list) or "no-hidden-reasoning" not in privacy:
        defects.append("privacyBoundary must list no-hidden-reasoning")
    if "no-host-paths" not in (privacy or []):
        defects.append("privacyBoundary must list no-host-paths")
    reviewers = artifact.get("automatedReviewers")
    if not isinstance(reviewers, list) or sorted(reviewers) != ["A", "B"]:
        defects.append("automatedReviewers must be exactly A and B")
    if artifact.get("humanPanelRequired") is not False:
        defects.append("humanPanelRequired must be false on the am-track")
    return defects


def run_auditor(name: str, path: Path) -> dict[str, Any]:
    artifact = load_json(path)
    if name == "A":
        defects = auditor_a(artifact)
    elif name == "B":
        defects = auditor_b(artifact)
    else:
        raise ValueError("auditor name must be A or B")
    return {
        "auditor": name,
        "path": str(path),
        "passed": not defects,
        "defects": defects,
        "evidence_class": "agent-machine-method-audit",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("auditor", choices=("A", "B", "both"))
    parser.add_argument(
        "artifact",
        type=Path,
        help="registration JSON path under docs/evidence or fixtures",
    )
    args = parser.parse_args(argv)
    names = ("A", "B") if args.auditor == "both" else (args.auditor,)
    results = [run_auditor(name, args.artifact) for name in names]
    passed = all(item["passed"] for item in results)
    print(json.dumps({"passed": passed, "results": results}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
