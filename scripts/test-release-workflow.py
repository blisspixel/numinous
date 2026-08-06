#!/usr/bin/env python3
"""Regression tests for the tagged release provenance boundary."""

from __future__ import annotations

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release.yml"
CONTRACT_COMMAND = "scripts/test-release-workflow.py"
SBOM_CONTRACT_COMMAND = "scripts/test-release-sbom.py"
PERFORMANCE_CONTRACT_COMMAND = "scripts/test-dependency-migration-performance.py"
PERFORMANCE_RECEIPT_COMMAND = (
    "scripts/dependency-migration-performance.py --verify-receipt "
    "docs/evidence/dependency-migration-2026-08-02.json"
)
ATTEST_ACTION = "actions/attest@a1948c3f048ba23858d222213b7c278aabede763 # v4.1.1"
RUST_TOOLCHAIN_ACTION = (
    "dtolnay/rust-toolchain@46511b1c83438f0dd37c02d843619ece5a4abb5b # 1.97.1"
)
HOOK_TRIGGER = (
    "'^(\\.github/workflows/(ci|release)\\.yml|"
    "docs/evidence/dependency-migration-[0-9-]+\\.json|"
    "scripts/(check|verify)\\.(ps1|sh)|scripts/hooks/pre-commit|"
    "scripts/(package-release|test-package-release|release-engagement-smoke|"
    "test-release-engagement-smoke|input-hardware-session|"
    "test-input-hardware-session|release-sbom|test-release-sbom|"
    "test-release-workflow|dependency-migration-performance|"
    "test-dependency-migration-performance)\\.py)$'"
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
        cls.audit = job_block(cls.workflow, "audit-artifacts")
        cls.attest = job_block(cls.workflow, "attest-artifacts")
        cls.publish = job_block(cls.workflow, "publish")

    def test_attestation_actions_are_current_immutable_and_exact(self) -> None:
        self.assertEqual(self.workflow.count(ATTEST_ACTION), 2)
        self.assertEqual(self.workflow.count("uses: actions/attest@"), 2)
        self.assertNotIn("actions/attest-build-provenance@", self.workflow)
        self.assertEqual(self.attest.count(f"uses: {ATTEST_ACTION}"), 2)
        self.assertEqual(self.attest.count("      - id: attest-build\n"), 1)
        self.assertEqual(self.attest.count("      - id: attest-sbom\n"), 1)
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

    def test_audit_requires_the_exact_release_artifact_set(self) -> None:
        self.assertIn(
            "          SOUNDTRACK_RESULT: ${{ needs.package-soundtrack.result }}\n",
            self.audit,
        )
        for target, extension in (
            ("aarch64-apple-darwin", "tar.gz"),
            ("x86_64-apple-darwin", "tar.gz"),
            ("x86_64-pc-windows-msvc", "zip"),
            ("x86_64-unknown-linux-gnu", "tar.gz"),
        ):
            archive = f'"dist/numinous-v${{version}}-{target}.{extension}"'
            self.assertEqual(self.audit.count(archive), 1)
        self.assertEqual(
            self.audit.count('"dist/numinous-v${version}-soundtrack.tar.gz"'), 1
        )
        self.assertIn("find dist -mindepth 1 -maxdepth 1", self.audit)
        self.assertIn('test "${#actual[@]}" -eq "${#allowed[@]}"', self.audit)
        self.assertIn('test "${actual[$index]}" = "${allowed[$index]}"', self.audit)
        self.assertIn('test "$SOUNDTRACK_RESULT" = skipped', self.audit)
        self.assertIn('--expected-version "$version"', self.audit)
        self.assertIn('--expected-revision "$revision"', self.audit)
        self.assertNotRegex(self.audit, r'test "\$count" -ge|wc -l')

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
        subjects = re.findall(
            r"(?ms)^          subject-path: \|\n(?P<body>(?:^            [^\n]+\n)+)",
            self.attest,
        )
        self.assertEqual(len(subjects), 2)
        for subject in subjects:
            self.assertEqual(
                subject.splitlines(),
                ["            dist/*.tar.gz", "            dist/*.zip"],
            )

    def test_sbom_is_generated_verified_and_attested_as_spdx(self) -> None:
        self.assertEqual(self.audit.count(f"uses: {RUST_TOOLCHAIN_ACTION}"), 1)
        self.assertEqual(self.audit.count("scripts/release-sbom.py generate"), 1)
        self.assertEqual(self.audit.count("scripts/release-sbom.py verify"), 1)
        self.assertIn('sbom="dist/numinous-v${version}-sbom.spdx.json"', self.audit)
        self.assertLess(
            self.audit.index("scripts/release-sbom.py generate"),
            self.audit.index("scripts/release-sbom.py verify"),
        )
        self.assertLess(
            self.audit.index("scripts/release-sbom.py verify"),
            self.audit.index("          name: verified-release-set"),
        )
        self.assertEqual(self.audit.count('--release-version "$version"'), 2)
        self.assertEqual(self.audit.count('--source-revision "$revision"'), 2)
        self.assertEqual(
            self.audit.count('--source-date-epoch "$source_date_epoch"'), 2
        )
        self.assertEqual(self.audit.count("--release-directory dist"), 2)
        self.assertLess(
            self.audit.index('--verify-archive "$archive"'),
            self.audit.index("scripts/release-sbom.py generate"),
        )
        self.assertIn(
            'revision="$(git rev-parse "${GITHUB_SHA}^{commit}")"', self.audit
        )
        self.assertEqual(
            self.attest.count("          predicate-type: https://spdx.dev/Document\n"),
            1,
        )
        self.assertEqual(
            self.attest.count(
                "          predicate-path: "
                "dist/numinous-${{ github.ref_name }}-sbom.spdx.json\n"
            ),
            1,
        )

    def test_provenance_bundle_is_a_required_release_artifact(self) -> None:
        self.assertIn("${{ steps.attest-build.outputs.bundle-path }}", self.attest)
        self.assertIn("${{ steps.attest-sbom.outputs.bundle-path }}", self.attest)
        self.assertIn("name: release-provenance", self.attest)
        self.assertIn("            dist/*-provenance.jsonl", self.attest)
        self.assertIn("dist/*-sbom-attestation.jsonl", self.attest)
        self.assertEqual(self.attest.count('test -s "$bundle"'), 2)
        self.assertIn(
            'test -s "dist/numinous-${GITHUB_REF_NAME}-provenance.jsonl"',
            self.attest,
        )
        self.assertIn(
            'test -s "dist/numinous-${GITHUB_REF_NAME}-sbom-attestation.jsonl"',
            self.attest,
        )
        self.assertIn("if-no-files-found: error", self.attest)

    def test_publication_cannot_bypass_audit_or_attestation(self) -> None:
        self.assertIn("needs: [audit-artifacts, attest-artifacts]", self.publish)
        self.assertIn("if: startsWith(github.ref, 'refs/tags/')", self.publish)
        self.assertEqual(self.publish.count("name: verified-release-set"), 1)
        self.assertEqual(self.publish.count("name: release-provenance"), 1)
        for suffix in (
            "sbom.spdx.json",
            "provenance.jsonl",
            "sbom-attestation.jsonl",
        ):
            self.assertIn(
                f'test -s "dist/numinous-${{GITHUB_REF_NAME}}-{suffix}"',
                self.publish,
            )
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
                self.assertEqual(source.count(SBOM_CONTRACT_COMMAND), expected)
                self.assertEqual(source.count(PERFORMANCE_CONTRACT_COMMAND), expected)
                self.assertEqual(source.count(PERFORMANCE_RECEIPT_COMMAND), expected)
        hook = (ROOT / "scripts" / "hooks" / "pre-commit").read_text(encoding="utf-8")
        self.assertIn(HOOK_TRIGGER, hook)

    def test_the_live_uninstall_roundtrip_runs_more_often_than_releases(self) -> None:
        # The roundtrip needs a packaged archive, so for a long time it ran only
        # on a tag. That made it the least exercised gate in the repository: a
        # change that broke uninstalling would sit in main until someone cut a
        # release. Nightly now packages an archive of its own and runs it, so
        # this asserts both halves are there rather than only the invocation,
        # which would pass against a step that had nothing to run against.
        nightly = (ROOT / ".github" / "workflows" / "nightly.yml").read_text(encoding="utf-8")
        for needed in (
            "cargo build --release --locked --bin numinous",
            "scripts/package-release.py",
            "scripts/uninstall-roundtrip.py",
            "--release-archive",
        ):
            self.assertIn(needed, nightly, f"nightly no longer {needed!r}")
        # The target used to be spelled out here as x86_64-unknown-linux-gnu,
        # because the roundtrip ran on Linux and nowhere else. It is a matrix
        # now, so pinning one triple would have meant either deleting this
        # check or keeping a Linux-shaped claim about a three-platform gate.
        # Name all three instead: this test got stronger, not looser.
        for target in (
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ):
            self.assertIn(
                f"target: {target}",
                nightly,
                f"the nightly roundtrip no longer packages for {target}, so 0.6-am's "
                f"three-platform claim would be short by one",
            )
        # And the judgment it depends on, which is cheap and has no excuse to
        # be absent from the same run.
        self.assertIn("scripts/test-uninstall-roundtrip.py", nightly)

        # Still on the release path too. Nightly is an addition, not a move: a
        # tag must not be able to publish without the roundtrip having run
        # against the artifact it is actually publishing.
        release = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        self.assertIn("scripts/uninstall-roundtrip.py", release)


if __name__ == "__main__":
    unittest.main(verbosity=2)
