#!/usr/bin/env python3
"""Regression tests for offline portable format and stdio conformance checks."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_interoperability", ROOT / "scripts" / "agent-interoperability.py"
)
assert SPEC is not None and SPEC.loader is not None
INTEROP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INTEROP)


class InteroperabilityTests(unittest.TestCase):
    def test_pinned_fixtures_have_source_revisions_and_matching_bytes(self) -> None:
        root = INTEROP.PLUGIN.FIXTURE_ROOT
        provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
        for artifact in provenance["artifacts"]:
            self.assertRegex(artifact["upstreamRevision"], r"^[a-f0-9]{40}$")
            if "path" in artifact:
                data = (root / artifact["path"]).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), artifact["sha256"])

    def test_hashed_fixture_git_blobs_survive_checkout_with_every_autocrlf_mode(
        self,
    ) -> None:
        root = INTEROP.PLUGIN.FIXTURE_ROOT
        provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
        fixtures: list[tuple[str, bytes, str]] = []
        for artifact in provenance["artifacts"]:
            if "path" not in artifact:
                continue
            path = root / artifact["path"]
            relative = path.relative_to(ROOT).as_posix()
            blob = subprocess.check_output(["git", "show", f":{relative}"], cwd=ROOT)
            with self.subTest(path=relative):
                self.assertEqual(hashlib.sha256(blob).hexdigest(), artifact["sha256"])
                self.assertEqual(path.read_bytes(), blob)
            fixtures.append((relative, blob, artifact["sha256"]))

        for mode in ("false", "true", "input"):
            with (
                self.subTest(autocrlf=mode),
                tempfile.TemporaryDirectory() as temporary,
                patch.dict(
                    "os.environ",
                    {
                        "GIT_DIR": "inherited-hook-git-dir",
                        "GIT_INDEX_FILE": "inherited-hook-index",
                        "GIT_WORK_TREE": "inherited-hook-worktree",
                    },
                ),
            ):
                checkout = Path(temporary)
                # Hooks may select the parent repository or a temporary index.
                # Only the isolated checkout drops those inherited selectors.
                environment = {
                    key: value
                    for key, value in os.environ.items()
                    if not key.upper().startswith("GIT_")
                }
                subprocess.run(
                    ["git", "init", "--quiet"],
                    cwd=checkout,
                    check=True,
                    capture_output=True,
                    env=environment,
                )
                (checkout / ".gitattributes").write_bytes(
                    (ROOT / ".gitattributes").read_bytes()
                )
                for relative, blob, _ in fixtures:
                    target = checkout / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(blob)
                git = [
                    "git",
                    "-c",
                    f"core.autocrlf={mode}",
                    "-c",
                    "core.safecrlf=false",
                ]
                subprocess.run(
                    [
                        *git,
                        "add",
                        "--",
                        ".gitattributes",
                        *(path for path, _, _ in fixtures),
                    ],
                    cwd=checkout,
                    check=True,
                    capture_output=True,
                    env=environment,
                )
                for relative, blob, _ in fixtures:
                    indexed = subprocess.check_output(
                        [*git, "show", f":{relative}"], cwd=checkout, env=environment
                    )
                    self.assertEqual(indexed, blob)
                    (checkout / relative).unlink()
                subprocess.run(
                    [*git, "checkout-index", "--all", "--force"],
                    cwd=checkout,
                    check=True,
                    capture_output=True,
                    env=environment,
                )
                for relative, blob, digest in fixtures:
                    with self.subTest(autocrlf=mode, path=relative):
                        data = (checkout / relative).read_bytes()
                        self.assertEqual(data, blob)
                        self.assertEqual(hashlib.sha256(data).hexdigest(), digest)

    def test_pinned_minimal_okf_allows_unknown_types_extensions_and_broken_links(
        self,
    ) -> None:
        fixture = json.loads(
            (INTEROP.PLUGIN.FIXTURE_ROOT / "okf-0.2-minimal.json").read_text(
                encoding="utf-8"
            )
        )
        concepts = INTEROP.validate_okf(fixture["files"])
        self.assertEqual(concepts["concept.md"]["type"], "Unregistered Type")
        self.assertEqual(concepts["concept.md"]["extension"], {"local": "preserved"})

    def test_okf_rejects_untyped_concepts_and_bad_reserved_files(self) -> None:
        for path, content in (
            ("concept.md", "No frontmatter."),
            ("concept.md", "---\ntype: false\n---\nBody"),
            ("concept.md", "---\ntype: ''\n---\nBody"),
            ("concept.md", "---\ntype: Attested Computation\n---\nBody"),
            ("nested/index.md", "---\nokf_version: '0.2'\n---\n"),
            ("index.md", "---\nokf_version: '0.1'\n---\n"),
            ("log.md", "---\ntype: log\n---\n"),
            ("log.md", "## yesterday\n* Updated.\n"),
            ("log.md", "## 2026-99-99\n* Updated.\n"),
            ("log.md", "## 2026-09-01\n* Updated.\n## 2026-09-02\n* Updated.\n"),
            ("../outside.md", "---\ntype: note\n---\nBody"),
        ):
            with self.subTest(path=path, content=content):
                with self.assertRaises(ValueError):
                    INTEROP.validate_okf([{"path": path, "content": content}])
        file = {"path": "concept.md", "content": "---\ntype: note\n---\nBody"}
        with self.assertRaisesRegex(INTEROP.ConformanceError, "repeats"):
            INTEROP.validate_okf([file, file])

    def test_utf8_bytes_and_closed_manifest_are_checked_independently(self) -> None:
        text = "---\ntype: note\n---\n日本語\n"
        file = {
            "path": "note.md",
            "content": text,
            "mediaType": "text/markdown",
            "bytes": len(text.encode("utf-8")),
            "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
        }
        manifest = {
            "files": [{key: value for key, value in file.items() if key != "content"}]
        }
        canonical = json.dumps(
            manifest, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        )
        capsule = {
            "files": [file],
            "manifest": manifest,
            "manifestSha256": hashlib.sha256(canonical.encode("utf-8")).hexdigest(),
        }
        INTEROP.validate_capsule(capsule)
        for field, value in (("bytes", len(text)), ("sha256", "0" * 64)):
            broken = json.loads(json.dumps(capsule))
            broken["files"][0][field] = value
            with self.subTest(field=field):
                with self.assertRaisesRegex(
                    INTEROP.ConformanceError, "digest or UTF-8"
                ):
                    INTEROP.validate_capsule(broken)
        broken = json.loads(json.dumps(capsule))
        broken["manifest"]["files"][0]["path"] = "other.md"
        with self.assertRaisesRegex(INTEROP.ConformanceError, "inventory differs"):
            INTEROP.validate_capsule(broken)
        broken = json.loads(json.dumps(capsule))
        broken["manifestSha256"] = "0" * 64
        with self.assertRaisesRegex(INTEROP.ConformanceError, "manifest digest"):
            INTEROP.validate_capsule(broken)

    def test_stdio_failures_and_ambiguous_envelopes_cannot_pass(self) -> None:
        request = INTEROP.call(1, "export_journal", {"format": "okf-0.2"})
        for output in (
            b"",
            b'{"jsonrpc":"2.0","id":2,"result":{}}\n',
            b'{"jsonrpc":"2.0","id":true,"result":{}}\n',
            b'{"jsonrpc":"2.0","id":1,"result":{"isError":true}}\n',
            b'{"jsonrpc":"2.0","id":1,"id":1,"result":{}}\n',
        ):

            def fake_run(
                *_args: object, **kwargs: object
            ) -> subprocess.CompletedProcess[bytes]:
                stream = kwargs["stdout"]
                getattr(stream, "write")(output)
                return subprocess.CompletedProcess(["server"], 0)

            with (
                self.subTest(output=output),
                patch.object(INTEROP.subprocess, "run", side_effect=fake_run),
            ):
                with self.assertRaises(ValueError):
                    INTEROP.session(Path(sys.executable), [request])

    def test_stdio_profile_isolated_and_cleaned_on_failure(self) -> None:
        roots: list[Path] = []

        def fake_run(
            *_args: object, **kwargs: object
        ) -> subprocess.CompletedProcess[bytes]:
            profile = Path(str(kwargs["cwd"]))
            roots.append(profile)
            env = kwargs["env"]
            self.assertIsInstance(env, dict)
            self.assertEqual(
                getattr(env, "get")("NUMINOUS_JOURNAL"), str(profile / "journal.txt")
            )
            (profile / "journal.txt").write_text("test state", encoding="utf-8")
            return subprocess.CompletedProcess(["server"], 1)

        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.dict(
                "os.environ", {"NUMINOUS_JOURNAL": str(Path(temporary) / "player.txt")}
            ),
            patch.object(INTEROP.subprocess, "run", side_effect=fake_run),
        ):
            with self.assertRaisesRegex(INTEROP.ConformanceError, "server failed"):
                INTEROP.session(
                    Path(sys.executable), [INTEROP.call(1, "export_journal", {})]
                )
            self.assertFalse((Path(temporary) / "player.txt").exists())
        self.assertEqual(len(roots), 1)
        self.assertFalse(roots[0].exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
