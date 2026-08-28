#!/usr/bin/env python3
"""Regression tests for the tagged release provenance boundary."""

from __future__ import annotations

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release.yml"
PACKAGE_WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release-packages.yml"
ATTEST_WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release-attest.yml"
CI_WORKFLOW_PATH = ROOT / ".github" / "workflows" / "ci.yml"
NIGHTLY_WORKFLOW_PATH = ROOT / ".github" / "workflows" / "nightly.yml"
VERIFY_PATH = ROOT / "VERIFY.md"
CONTRACT_COMMAND = "scripts/test-release-workflow.py"
PIN_CONTRACT_COMMAND = "scripts/test-workflow-pins.py"
SBOM_CONTRACT_COMMAND = "scripts/test-release-sbom.py"
PERFORMANCE_CONTRACT_COMMAND = "scripts/test-dependency-migration-performance.py"
PERFORMANCE_RECEIPT_COMMAND = (
    "scripts/dependency-migration-performance.py --verify-receipt "
    "docs/evidence/dependency-migration-2026-08-02.json"
)
REQUIRED_CI_JOBS = (
    "quality",
    "msrv",
    "house-style",
    "supply-chain",
    "audit",
    "codeql",
    "coverage",
    "build",
    "release-artifacts",
)
ATTEST_ACTION = "actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4.2.2"
DEPENDENCY_REVIEW_ACTION = (
    "actions/dependency-review-action@"
    "a1d282b36b6f3519aa1f3fc636f609c47dddb294 # v5.0.0"
)
CODEQL_INIT_ACTION = (
    "github/codeql-action/init@"
    "cdf488f595d80d6e07e03d4674febd5ab45fa938 # v4.37.9"
)
CODEQL_ANALYZE_ACTION = (
    "github/codeql-action/analyze@"
    "cdf488f595d80d6e07e03d4674febd5ab45fa938 # v4.37.9"
)
INSTALL_ACTION = (
    "taiki-e/install-action@b6ff580856c41316412a0b9b60540fbc6f8c82cc # v2.86.7"
)
RUST_TOOLCHAIN_ACTION = (
    "dtolnay/rust-toolchain@46511b1c83438f0dd37c02d843619ece5a4abb5b # 1.97.1"
)
HOOK_TRIGGER = (
    "'^(\\.github/workflows/(ci|nightly|release|release-attest|"
    "release-packages)\\.yml|"
    "docs/evidence/dependency-migration-[0-9-]+\\.json|"
    "scripts/(check|verify)\\.(ps1|sh)|scripts/hooks/pre-commit|"
    "scripts/(package-release|test-package-release|release-engagement-smoke|"
    "test-release-engagement-smoke|input-hardware-session|"
    "test-input-hardware-session|sensory-platform-set|"
    "test-sensory-platform-(proof|set)|release-sbom|test-release-sbom|"
    "test-release-workflow|test-workflow-pins|dependency-migration-performance|"
    "test-dependency-migration-performance)\\.py)$'"
)


def job_block(workflow: str, name: str) -> str:
    """Return one exact two-space-indented job body."""
    match = re.search(
        rf"(?ms)^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)",
        workflow,
    )
    if match is None:
        raise AssertionError(f"workflow has no {name} job")
    return match.group("body")


class ReleaseWorkflowTests(unittest.TestCase):
    """Keep provenance publication narrow, ordered, and reproducible."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.package_workflow = PACKAGE_WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.attest_workflow = ATTEST_WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.ci_workflow = CI_WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.nightly_workflow = NIGHTLY_WORKFLOW_PATH.read_text(encoding="utf-8")
        cls.verify = VERIFY_PATH.read_text(encoding="utf-8")
        cls.header = cls.workflow.split("\njobs:\n", maxsplit=1)[0]
        cls.package_header = cls.package_workflow.split("\njobs:\n", maxsplit=1)[0]
        cls.validate_release = job_block(
            cls.package_workflow, "validate-release-reference"
        )
        cls.audit = job_block(cls.package_workflow, "audit-artifacts")
        cls.attest_validate = job_block(
            cls.attest_workflow, "validate-release-context"
        )
        cls.attest_packages = job_block(cls.attest_workflow, "package-artifacts")
        cls.attest = job_block(cls.attest_workflow, "attest-artifacts")
        cls.release_artifacts = job_block(cls.workflow, "release-artifacts")
        cls.publish = job_block(cls.workflow, "publish")
        cls.supply_chain = job_block(cls.ci_workflow, "supply-chain")
        cls.codeql = job_block(cls.ci_workflow, "codeql")

    def test_attestation_actions_are_current_immutable_and_exact(self) -> None:
        self.assertEqual(self.attest_workflow.count(ATTEST_ACTION), 2)
        self.assertEqual(self.attest_workflow.count("uses: actions/attest@"), 2)
        self.assertNotIn("actions/attest-build-provenance@", self.attest_workflow)
        self.assertEqual(self.attest.count(f"uses: {ATTEST_ACTION}"), 2)
        self.assertEqual(self.attest.count("      - id: attest-build\n"), 1)
        self.assertEqual(self.attest.count("      - id: attest-sbom\n"), 1)
        self.assertNotRegex(
            self.attest_workflow,
            r"actions/attest@(v|main|master)",
        )
        self.assertNotIn("uses: actions/attest@", self.package_workflow)
        self.assertNotIn("uses: actions/attest@", self.workflow)

    def test_ci_install_actions_are_current_immutable_and_exact(self) -> None:
        self.assertEqual(self.ci_workflow.count(INSTALL_ACTION), 2)
        self.assertEqual(self.ci_workflow.count("uses: taiki-e/install-action@"), 2)
        self.assertNotRegex(
            self.ci_workflow,
            r"taiki-e/install-action@(v|main|master)",
        )

    def test_dependency_review_is_pr_only_read_only_and_strict(self) -> None:
        self.assertEqual(self.ci_workflow.count(DEPENDENCY_REVIEW_ACTION), 1)
        self.assertEqual(
            self.supply_chain.count(f"uses: {DEPENDENCY_REVIEW_ACTION}"), 1
        )
        self.assertIn(
            "        if: github.event_name == 'pull_request'\n", self.supply_chain
        )
        self.assertIn("          fail-on-severity: moderate\n", self.supply_chain)
        self.assertIn(
            "          fail-on-scopes: runtime, development, unknown\n",
            self.supply_chain,
        )
        self.assertIn(
            "          comment-summary-in-pr: never\n", self.supply_chain
        )
        self.assertIn("          show-patched-versions: true\n", self.supply_chain)
        self.assertNotIn("permissions:", self.supply_chain)
        self.assertIn("permissions:\n  contents: read\n", self.ci_workflow)

    def test_codeql_covers_rust_and_workflows_inside_main_ci(self) -> None:
        self.assertEqual(self.codeql.count(f"uses: {CODEQL_INIT_ACTION}"), 1)
        self.assertEqual(self.codeql.count(f"uses: {CODEQL_ANALYZE_ACTION}"), 1)
        self.assertIn("        language: [rust, actions]\n", self.codeql)
        self.assertIn("      fail-fast: false\n", self.codeql)
        self.assertIn("          build-mode: none\n", self.codeql)
        self.assertIn("          queries: security-extended\n", self.codeql)
        self.assertIn(
            "          category: /language:${{ matrix.language }}\n", self.codeql
        )
        self.assertIn("      - if: matrix.language == 'rust'\n", self.codeql)
        permissions = re.search(
            r"(?ms)^    permissions:\n(?P<body>(?:^      [^\n]+\n)+)",
            self.codeql,
        )
        self.assertIsNotNone(permissions)
        self.assertEqual(
            set(permissions.group("body").splitlines()),
            {
                "      actions: read",
                "      contents: read",
                "      security-events: write",
            },
        )
        self.assertIn(
            "          upload: ${{ github.event_name != 'push' || "
            "github.event.head_commit.author.username != 'dependabot[bot]' }}\n",
            self.codeql,
        )

    def test_privileged_authority_and_publication_are_workflow_unique(self) -> None:
        self.assertIn("permissions:\n  contents: read\n", self.header)
        self.assertNotIn("permissions: write-all", self.workflow)
        self.assertNotIn("permissions: write-all", self.package_workflow)
        self.assertNotIn("permissions: write-all", self.attest_workflow)
        self.assertEqual(self.workflow.count("attestations: write"), 1)
        self.assertEqual(self.workflow.count("id-token: write"), 1)
        self.assertEqual(self.workflow.count("artifact-metadata: write"), 1)
        self.assertEqual(self.workflow.count("contents: write"), 1)
        self.assertIn("attestations: write", self.attest)
        self.assertIn("id-token: write", self.attest)
        self.assertIn("artifact-metadata: write", self.attest)
        self.assertIn("contents: write", self.publish)
        self.assertEqual(self.workflow.count("gh release create"), 1)
        self.assertIn("gh release create", self.publish)
        self.assertEqual(self.attest_workflow.count("attestations: write"), 1)
        self.assertEqual(self.attest_workflow.count("id-token: write"), 1)
        self.assertEqual(self.attest_workflow.count("artifact-metadata: write"), 1)
        for permission in (
            "artifact-metadata: write",
            "attestations: write",
            "id-token: write",
            "contents: write",
        ):
            self.assertNotIn(permission, self.package_workflow)
        for job in (
            self.validate_release,
            job_block(self.package_workflow, "package-binaries"),
            job_block(self.package_workflow, "package-soundtrack"),
            self.audit,
        ):
            for permission in (
                "artifact-metadata: write",
                "attestations: write",
                "id-token: write",
                "contents: write",
            ):
                self.assertNotIn(permission, job)

    def test_pull_requests_reuse_the_read_only_package_workflow(self) -> None:
        self.assertIn("  workflow_call:\n", self.package_header)
        self.assertNotIn("  push:\n", self.package_header)
        self.assertNotIn("  workflow_dispatch:\n", self.package_header)
        self.assertNotIn("  workflow_call:\n", self.header)
        self.assertNotIn("  pull_request:\n", self.header)
        release_artifacts = job_block(self.ci_workflow, "release-artifacts")
        self.assertIn("    name: release artifacts\n", release_artifacts)
        self.assertIn(
            "    uses: ./.github/workflows/release-packages.yml\n",
            release_artifacts,
        )
        self.assertEqual(
            self.ci_workflow.count("uses: ./.github/workflows/release-packages.yml"),
            1,
        )
        self.assertNotIn("uses: ./.github/workflows/release.yml", self.ci_workflow)
        for permission in (
            "artifact-metadata: write",
            "attestations: write",
            "id-token: write",
            "contents: write",
        ):
            self.assertNotIn(permission, release_artifacts)
        tag_push = (
            "    if: github.event_name == 'push' && "
            "startsWith(github.ref, 'refs/tags/')\n"
        )
        preview = job_block(self.workflow, "package-preview")
        self.assertIn("    if: github.event_name == 'workflow_dispatch'\n", preview)
        self.assertIn(
            "    uses: ./.github/workflows/release-packages.yml\n", preview
        )
        self.assertIn("      contents: read\n", preview)
        for permission in (
            "artifact-metadata: write",
            "attestations: write",
            "id-token: write",
            "contents: write",
        ):
            self.assertNotIn(permission, preview)
        self.assertIn(tag_push, self.release_artifacts)
        self.assertIn(
            "    uses: ./.github/workflows/release-attest.yml\n",
            self.release_artifacts,
        )
        self.assertEqual(
            self.workflow.count("uses: ./.github/workflows/release-attest.yml"),
            1,
        )
        self.assertIn(
            "    uses: ./.github/workflows/release-packages.yml\n",
            self.attest_packages,
        )
        self.assertIn("    needs: validate-release-context\n", self.attest_packages)
        self.assertIn("      contents: read\n", self.attest_packages)
        for permission in (
            "artifact-metadata: write",
            "attestations: write",
            "id-token: write",
            "contents: write",
        ):
            self.assertNotIn(permission, self.attest_packages)

    def test_main_ci_requires_every_job_to_succeed(self) -> None:
        main_ci = job_block(self.ci_workflow, "main-ci")
        self.assertEqual(self.ci_workflow.count("    name: main CI\n"), 1)
        self.assertIn("    name: main CI\n", main_ci)
        self.assertIn("    if: always()\n", main_ci)
        self.assertEqual(
            set(re.findall(r"^      - ([a-z0-9-]+)$", main_ci, re.MULTILINE)),
            set(REQUIRED_CI_JOBS),
        )
        for job in REQUIRED_CI_JOBS:
            self.assertEqual(main_ci.count(f"${{{{ needs.{job}.result }}}}"), 1)
        self.assertEqual(main_ci.count('test "$result" = success'), 1)

    def test_every_runner_job_has_a_deliberate_timeout(self) -> None:
        expected = {
            CI_WORKFLOW_PATH: {
                "quality": 30,
                "msrv": 15,
                "house-style": 10,
                "supply-chain": 10,
                "audit": 10,
                "codeql": 20,
                "coverage": 20,
                "build": 30,
                "main-ci": 5,
            },
            PACKAGE_WORKFLOW_PATH: {
                "validate-release-reference": 5,
                "package-binaries": 30,
                "package-soundtrack": 15,
                "audit-artifacts": 15,
            },
            ATTEST_WORKFLOW_PATH: {
                "validate-release-context": 5,
                "attest-artifacts": 10,
            },
            WORKFLOW_PATH: {
                "publish": 10,
            },
            NIGHTLY_WORKFLOW_PATH: {
                "am-qa": 30,
                "roundtrip": 45,
            },
        }
        for path, timeouts in expected.items():
            workflow = path.read_text(encoding="utf-8")
            runner_jobs = {
                name
                for name in re.findall(r"(?m)^  ([a-z0-9][a-z0-9-]*):$", workflow)
                if "    runs-on:" in job_block(workflow, name)
            }
            with self.subTest(path=path.relative_to(ROOT)):
                self.assertEqual(runner_jobs, set(timeouts))
            for name, minutes in timeouts.items():
                block = job_block(workflow, name)
                with self.subTest(path=path.relative_to(ROOT), job=name):
                    self.assertEqual(block.count("    timeout-minutes:"), 1)
                    self.assertIn(f"    timeout-minutes: {minutes}\n", block)

    def test_release_reference_is_validated_before_packaging(self) -> None:
        self.assertEqual(
            self.validate_release.count("if: startsWith(github.ref, 'refs/tags/')"),
            2,
        )
        self.assertIn("          fetch-depth: 0\n", self.validate_release)
        self.assertIn("          persist-credentials: false\n", self.validate_release)
        self.assertEqual(
            self.package_workflow.count(
                '--validate-release-reference "${GITHUB_REF_NAME}"'
            ),
            1,
        )
        self.assertIn('--tag-ref "${GITHUB_REF}"', self.validate_release)
        self.assertIn('--expected-sha "${GITHUB_SHA}"', self.validate_release)
        self.assertIn(
            "--main-ref refs/remotes/origin/main", self.validate_release
        )
        for name in ("package-binaries", "package-soundtrack"):
            self.assertIn(
                "    needs: validate-release-reference\n",
                job_block(self.package_workflow, name),
            )

    def test_attestation_is_tag_only_and_follows_the_closed_read_only_audit(
        self,
    ) -> None:
        self.assertIn("  workflow_call:\n", self.attest_workflow)
        self.assertNotIn("  push:\n", self.attest_workflow)
        self.assertNotIn("  workflow_dispatch:\n", self.attest_workflow)
        self.assertIn(
            "    if: github.event_name == 'push' && "
            "startsWith(github.ref, 'refs/tags/')\n",
            self.release_artifacts,
        )
        self.assertIn("    needs: validate-release-context\n", self.attest_packages)
        self.assertIn("    needs: package-artifacts\n", self.attest)
        self.assertEqual(
            self.attest_workflow.count('test "${GITHUB_EVENT_NAME}" = push'), 2
        )
        self.assertEqual(self.attest_workflow.count("            refs/tags/*) ;;"), 2)
        self.assertEqual(
            self.attest_workflow.count(
                '--validate-release-reference "${GITHUB_REF_NAME}"'
            ),
            2,
        )
        self.assertEqual(
            self.attest_workflow.count(
                '--validate-remote-release-tag "${GITHUB_REF_NAME}"'
            ),
            2,
        )
        self.assertIn("          fetch-depth: 0\n", self.attest_validate)
        self.assertIn("          persist-credentials: false\n", self.attest_validate)
        self.assertIn("          name: verified-release-input-set\n", self.attest)
        self.assertIn("          name: verified-release-input-set\n", self.audit)
        self.assertEqual(self.attest.count('--verify-archive "$archive"'), 1)
        self.assertEqual(self.attest.count("scripts/release-sbom.py verify"), 1)
        self.assertEqual(self.attest.count("--verify-file-set dist"), 2)
        self.assertLess(
            self.attest.index(
                "      - name: Revalidate release context and audited contents\n"
            ),
            self.attest.index("      - id: attest-build\n"),
        )

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
        self.assertEqual(self.audit.count("--verify-file-set dist"), 2)
        self.assertEqual(self.audit.count('--file-set-manifest "$input_manifest"'), 2)
        self.assertIn('input_manifest="${RUNNER_TEMP}/release-input-files"', self.audit)
        self.assertIn('test "$SOUNDTRACK_RESULT" = skipped', self.audit)
        self.assertIn('--expected-version "$version"', self.audit)
        self.assertIn('--expected-revision "$revision"', self.audit)
        self.assertNotRegex(self.audit, r'test "\$count" -ge|wc -l|find dist')

    def test_attestation_has_only_required_job_permissions(self) -> None:
        permissions = re.search(
            r"(?ms)^    permissions:\n(?P<body>(?:^      [^\n]+\n)+)",
            self.attest,
        )
        self.assertIsNotNone(permissions)
        self.assertEqual(
            set(permissions.group("body").splitlines()),
            {
                "      artifact-metadata: write",
                "      attestations: write",
                "      contents: read",
                "      id-token: write",
            },
        )

    def test_only_verified_archives_are_attestation_subjects(self) -> None:
        manifests = re.findall(
            r"(?ms)printf '%s\\n' \\\n(?P<body>.*?)^            >\"\$input_manifest\"$",
            self.attest,
        )
        self.assertEqual(len(manifests), 2)
        input_names = re.findall(r'"(numinous-[^\"]+)"', manifests[0])
        final_names = re.findall(r'"(numinous-[^\"]+)"', manifests[1])
        self.assertEqual(len(input_names), 12)
        self.assertEqual(len(final_names), 14)
        self.assertEqual(input_names, sorted(input_names))
        normalized_final_names = [
            name.replace("${GITHUB_REF_NAME}", "v${version}")
            for name in final_names
        ]
        self.assertEqual(normalized_final_names, sorted(normalized_final_names))
        subjects = re.findall(
            r"(?ms)^          subject-path: \|\n(?P<body>(?:^            [^\n]+\n)+)",
            self.attest,
        )
        self.assertEqual(len(subjects), 2)
        self.assertEqual(
            subjects[0].splitlines(),
            ["            dist/*.tar.gz", "            dist/*.zip"],
        )
        self.assertEqual(
            subjects[1].splitlines(),
            [
                "            dist/numinous-${{ github.ref_name }}-aarch64-apple-darwin.tar.gz",
                "            dist/numinous-${{ github.ref_name }}-x86_64-apple-darwin.tar.gz",
                "            dist/numinous-${{ github.ref_name }}-x86_64-pc-windows-msvc.zip",
                "            dist/numinous-${{ github.ref_name }}-x86_64-unknown-linux-gnu.tar.gz",
            ],
        )
        self.assertNotIn("soundtrack", subjects[1])
        self.assertLess(
            self.attest.index(
                "      - name: Revalidate release context and audited contents\n"
            ),
            self.attest.index("      - id: attest-build\n"),
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
            self.audit.index("      - name: Verify final audited input set\n"),
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
            self.attest.count(
                "          sbom-path: "
                "dist/numinous-${{ github.ref_name }}-sbom.spdx.json\n"
            ),
            1,
        )
        self.assertNotIn("predicate-type:", self.attest)
        self.assertNotIn("predicate-path:", self.attest)

    def test_provenance_bundle_is_a_required_release_artifact(self) -> None:
        self.assertIn("${{ steps.attest-build.outputs.bundle-path }}", self.attest)
        self.assertIn("${{ steps.attest-sbom.outputs.bundle-path }}", self.attest)
        self.assertNotIn("name: release-provenance", self.package_workflow)
        self.assertNotIn("name: release-provenance", self.attest_workflow)
        self.assertEqual(self.attest.count('test -s "$bundle"'), 2)
        self.assertIn(
            'test -s "dist/numinous-${GITHUB_REF_NAME}-provenance.jsonl"',
            self.attest,
        )
        self.assertIn(
            'test -s "dist/numinous-${GITHUB_REF_NAME}-sbom-attestation.jsonl"',
            self.attest,
        )
        self.assertIn("      - name: Verify final release evidence set\n", self.attest)
        self.assertIn(
            '"numinous-${GITHUB_REF_NAME}-provenance.jsonl"', self.attest
        )
        self.assertIn(
            '"numinous-${GITHUB_REF_NAME}-sbom-attestation.jsonl"', self.attest
        )
        self.assertIn("if-no-files-found: error", self.attest)
        self.assertLess(
            self.attest.index("      - name: Verify final release evidence set\n"),
            self.attest.index("          name: verified-release-set\n"),
        )

    def test_publication_cannot_bypass_audit_or_attestation(self) -> None:
        self.assertIn("needs: release-artifacts", self.publish)
        self.assertIn(
            "if: github.event_name == 'push' && "
            "startsWith(github.ref, 'refs/tags/')",
            self.publish,
        )
        self.assertEqual(self.publish.count("name: verified-release-set"), 1)
        self.assertNotIn("name: release-provenance", self.publish)
        self.assertIn("    environment: release\n", self.publish)
        for suffix in (
            "sbom.spdx.json",
            "provenance.jsonl",
            "sbom-attestation.jsonl",
        ):
            self.assertIn(
                f'test -s "dist/numinous-${{GITHUB_REF_NAME}}-{suffix}"',
                self.publish,
            )
        remote_validation = (
            '--validate-remote-release-tag "${GITHUB_REF_NAME}"'
        )
        self.assertEqual(self.publish.count(remote_validation), 1)
        self.assertIn('--expected-sha "${GITHUB_SHA}"', self.publish)
        self.assertIn("--remote origin", self.publish)
        self.assertLess(
            self.publish.index(remote_validation),
            self.publish.index('gh release create "${GITHUB_REF_NAME}"'),
        )
        self.assertIn('gh release create "${GITHUB_REF_NAME}" dist/*', self.publish)

    def test_verification_commands_lock_the_reusable_signer_and_spdx_type(
        self,
    ) -> None:
        signer = (
            "--signer-workflow "
            "blisspixel/numinous/.github/workflows/release-attest.yml"
        )
        self.assertEqual(self.verify.count(signer), 4)
        self.assertEqual(
            self.verify.count(
            "--signer-workflow blisspixel/numinous/.github/workflows/release.yml",
            ),
            1,
        )
        self.assertEqual(
            self.verify.count("--predicate-type https://spdx.dev/Document/v2.3"),
            2,
        )
        verification_commands = [
            line
            for line in self.verify.splitlines()
            if line.startswith("gh attestation verify ")
        ]
        self.assertEqual(len(verification_commands), 4)
        for constraint in (
            "--source-ref refs/tags/TAG",
            "--source-digest TAG_COMMIT_SHA",
            "--signer-digest TAG_COMMIT_SHA",
            "--deny-self-hosted-runners",
        ):
            self.assertTrue(
                all(constraint in command for command in verification_commands)
            )
        self.assertIn(
            "Existing attestations through `v0.4.0-alpha.9` use",
            self.verify,
        )
        self.assertIn("`https://spdx.dev/Document`", self.verify)

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
        pin_expected_counts = {
            ROOT / "scripts" / "check.ps1": 1,
            ROOT / "scripts" / "check.sh": 1,
            ROOT / "scripts" / "verify.ps1": 1,
            ROOT / "scripts" / "verify.sh": 1,
            ROOT / "scripts" / "hooks" / "pre-commit": 1,
            ROOT / ".github" / "workflows" / "ci.yml": 1,
            ROOT / ".github" / "workflows" / "nightly.yml": 1,
        }
        for path, expected in pin_expected_counts.items():
            with self.subTest(path=path.relative_to(ROOT), contract="action policy"):
                source = path.read_text(encoding="utf-8")
                self.assertEqual(source.count(PIN_CONTRACT_COMMAND), expected)
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
        self.assertIn("scripts/uninstall-roundtrip.py", self.package_workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
