#!/usr/bin/env python3
"""Regression tests for the tagged release provenance boundary."""

from __future__ import annotations

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release.yml"
CONTRACT_COMMAND = "scripts/test-release-workflow.py"
ATTEST_ACTION = (
    "actions/attest@a1948c3f048ba23858d222213b7c278aabede763 # v4.1.1"
)
HOOK_TRIGGER = (
    "'^(\\.github/workflows/(ci|release)\\.yml|"
    "scripts/(check|verify)\\.(ps1|sh)|scripts/hooks/pre-commit|"
    "scripts/(package-release|test-package-release|release-engagement-smoke|"
    "test-release-engagement-smoke|input-hardware-session|"
    "test-input-hardware-session|test-release-workflow)\\.py)$'"
)


def job_block(workflow: str, name: str) -> str:
    """Return one exact two-space-indented job body."""
    match = re.search(
        rf"(?ms)^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)",
        workflow,
    )
    if match is None:
        raise AssertionError(f"release workflow has no {name} job")
    return match.group("body")


class ReleaseWorkflowTests(unittest.TestCase):
    """Keep provenance publication narrow, ordered, and reproducible."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.header = cls.workflow.split("\njobs:\n", maxsplit=1)[0]
        cls.attest = job_block(cls.workflow, "attest-artifacts")
        cls.publish = job_block(cls.workflow, "publish")

    def test_attestation_action_is_immutable_and_unique(self) -> None:
        self.assertEqual(self.workflow.count(ATTEST_ACTION), 1)
        self.assertEqual(self.workflow.count("uses: actions/attest@"), 1)
        self.assertNotIn("actions/attest-build-provenance@", self.workflow)
        self.assertIn(f"uses: {ATTEST_ACTION}", self.attest)
        self.assertNotRegex(
            self.workflow,
            r"actions/attest@(v|main|master)",
        )

    def test_privileged_authority_and_publication_are_workflow_unique(self) -> None:
        self.assertIn("permissions:\n  contents: read\n", self.header)
        self.assertNotIn("permissions: write-all", self.workflow)
        self.assertEqual(self.workflow.count("attestations: write"), 1)
        self.assertEqual(self.workflow.count("id-token: write"), 1)
        self.assertEqual(self.workflow.count("contents: write"), 1)
        self.assertIn("attestations: write", self.attest)
        self.assertIn("id-token: write", self.attest)
        self.assertIn("contents: write", self.publish)
        self.assertEqual(self.workflow.count("gh release create"), 1)
        self.assertIn("gh release create", self.publish)

    def test_attestation_is_tag_only_and_follows_closed_set_audit(self) -> None:
        self.assertIn("    needs: audit-artifacts\n", self.attest)
        self.assertIn("    if: startsWith(github.ref, 'refs/tags/')\n", self.attest)
        self.assertNotIn("github.event_name == 'workflow_dispatch'", self.attest)

    def test_attestation_has_only_required_job_permissions(self) -> None:
        permissions = re.search(
            r"(?ms)^    permissions:\n(?P<body>(?:^      [^\n]+\n)+)",
            self.attest,
        )
        self.assertIsNotNone(permissions)
        self.assertEqual(
            set(permissions.group("body").splitlines()),
            {
                "      attestations: write",
                "      contents: read",
                "      id-token: write",
            },
        )

    def test_only_verified_archives_are_attestation_subjects(self) -> None:
        self.assertIn("          name: verified-release-set\n", self.attest)
        subjects = re.search(
            r"(?ms)^          subject-path: \|\n(?P<body>(?:^            [^\n]+\n)+)",
            self.attest,
        )
        self.assertIsNotNone(subjects)
        self.assertEqual(
            subjects.group("body").splitlines(),
            ["            dist/*.tar.gz", "            dist/*.zip"],
        )

    def test_provenance_bundle_is_a_required_release_artifact(self) -> None:
        self.assertIn("${{ steps.attest.outputs.bundle-path }}", self.attest)
        self.assertIn("name: release-provenance", self.attest)
        self.assertIn("path: dist/*-provenance.jsonl", self.attest)
        self.assertIn("if-no-files-found: error", self.attest)

    def test_publication_cannot_bypass_audit_or_attestation(self) -> None:
        self.assertIn("needs: [audit-artifacts, attest-artifacts]", self.publish)
        self.assertIn("if: startsWith(github.ref, 'refs/tags/')", self.publish)
        self.assertEqual(self.publish.count("name: verified-release-set"), 1)
        self.assertEqual(self.publish.count("name: release-provenance"), 1)
        self.assertIn('gh release create "${GITHUB_REF_NAME}" dist/*', self.publish)

    def test_contract_is_wired_into_every_local_and_ci_gate(self) -> None:
        expected_counts = {
            ROOT / "scripts" / "check.ps1": 1,
            ROOT / "scripts" / "check.sh": 1,
            ROOT / "scripts" / "verify.ps1": 1,
            ROOT / "scripts" / "verify.sh": 1,
            ROOT / "scripts" / "hooks" / "pre-commit": 1,
            ROOT / ".github" / "workflows" / "ci.yml": 3,
        }
        for path, expected in expected_counts.items():
            with self.subTest(path=path.relative_to(ROOT)):
                source = path.read_text(encoding="utf-8")
                self.assertEqual(source.count(CONTRACT_COMMAND), expected)
        hook = (ROOT / "scripts" / "hooks" / "pre-commit").read_text(
            encoding="utf-8"
        )
        self.assertIn(HOOK_TRIGGER, hook)


if __name__ == "__main__":
    unittest.main(verbosity=2)
