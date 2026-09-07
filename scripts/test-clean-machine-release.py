#!/usr/bin/env python3
"""Focused regressions for the clean-machine release gate.

These cover the judgments without downloading anything or installing anything,
so a mistake in what counts as a clean machine is caught even on a laptop with
no network. The live gate is `clean-machine-release.py`, which CI runs against a
published release on a runner that did not build it.

The part worth testing hardest is the refusal. Whether an archive matches its
checksum is arithmetic and fails loudly. Whether the machine running the gate is
entitled to vouch for the artifact is a claim about the environment, and a claim
like that can quietly become false while every other check stays green.
"""

from __future__ import annotations

import importlib.util
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "clean_machine_release", ROOT / "scripts" / "clean-machine-release.py"
)
assert SPEC is not None and SPEC.loader is not None
# A dynamically loaded module has no statically known attributes.
MODULE: Any = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BuildOutputTests(unittest.TestCase):
    def test_a_machine_holding_built_faces_is_not_clean_for_them(self) -> None:
        # The whole premise. A machine that compiled these binaries cannot be
        # the machine that independently vouches for them, however carefully the
        # rest of the run is staged.
        with self.subTest("empty tree"):
            self.assertEqual(MODULE.find_build_output(self.make_tree(built=[])), [])
        with self.subTest("release build present"):
            found = MODULE.find_build_output(self.make_tree(built=["release/numinous"]))
            self.assertEqual([path.name for path in found], ["numinous"])
        with self.subTest("windows debug build present"):
            found = MODULE.find_build_output(self.make_tree(built=["debug/numinous-mcp.exe"]))
            self.assertEqual([path.name for path in found], ["numinous-mcp.exe"])

    def test_an_unrelated_target_directory_is_not_evidence_of_building(self) -> None:
        # A target directory left by any other crate says nothing about this
        # artifact. Treating its mere existence as guilt would make the gate
        # unrunnable on ordinary machines and teach people to skip it.
        root = self.make_tree(built=["release/something-else"])
        self.assertEqual(MODULE.find_build_output(root), [])

    def make_tree(self, built: list[str]) -> Path:
        root = Path(tempfile.mkdtemp(prefix="numinous-clean-machine-test-"))
        self.addCleanup(shutil.rmtree, root, True)
        for relative in built:
            path = root / "target" / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"not a real binary")
        return root


class SidecarTests(unittest.TestCase):
    def sidecar(self, text: str) -> Path:
        directory = Path(tempfile.mkdtemp(prefix="numinous-sidecar-test-"))
        self.addCleanup(shutil.rmtree, directory, True)
        path = directory / "archive.tar.gz.sha256"
        path.write_text(text, encoding="ascii")
        return path

    def test_a_well_formed_sidecar_yields_its_digest(self) -> None:
        digest = "a" * 64
        self.assertEqual(
            MODULE.sidecar_digest(self.sidecar(f"{digest}  archive.tar.gz\n")), digest
        )

    def test_a_sidecar_that_declares_no_digest_is_refused(self) -> None:
        # Refusing beats guessing: an unreadable sidecar must stop the run
        # before anything unpacks, not be treated as an absent constraint.
        for text in ("", "not-a-digest  archive.tar.gz\n", "abc  archive.tar.gz\n", "\n"):
            with self.subTest(text=text):
                with self.assertRaises(MODULE.GateError):
                    MODULE.sidecar_digest(self.sidecar(text))

    def test_an_uppercase_digest_is_refused_rather_than_normalized(self) -> None:
        # The published sidecars are lowercase. Quietly accepting another form
        # would mean this gate no longer checks the shape it claims to.
        with self.assertRaises(MODULE.GateError):
            MODULE.sidecar_digest(self.sidecar(("A" * 64) + "  archive.tar.gz\n"))


class TagTests(unittest.TestCase):
    def test_only_a_release_tag_is_accepted(self) -> None:
        for good in ("v0.4.0-alpha.23", "v1.0.0", "v0.10.2-rc.1"):
            with self.subTest(tag=good):
                self.assertEqual(MODULE.resolve_tag(good), good)

    def test_anything_that_is_not_a_release_tag_is_refused(self) -> None:
        # A tag reaches a shell command and names a download. Refusing an
        # unexpected shape keeps that boundary narrow.
        for bad in ("main", "0.4.0", "v0.4", "v0.4.0-alpha.23; rm -rf /", "--repo"):
            with self.subTest(tag=bad):
                with self.assertRaises(MODULE.GateError):
                    MODULE.resolve_tag(bad)


class ReleaseLookupTests(unittest.TestCase):
    def test_the_newest_release_is_found_by_listing_not_by_latest(self) -> None:
        # Every Numinous release so far is a prerelease, and GitHub excludes
        # prereleases from "latest". `gh release view` with no tag therefore
        # answers "release not found", which is how this gate shipped unable to
        # resolve a tag on its own: every local run passed one explicitly. The
        # nightly caught it on all three platforms the first time it ran.
        source = (ROOT / "scripts" / "clean-machine-release.py").read_text(encoding="utf-8")
        self.assertIn('"gh", "release", "list"', source)
        self.assertNotIn('"gh", "release", "view"', source)


class ArtifactNameTests(unittest.TestCase):
    def test_every_published_target_has_an_archive_shape(self) -> None:
        # The four targets the release workflow publishes must each be
        # installable by this gate, or a platform silently loses its evidence.
        published = {
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        }
        self.assertEqual({target for target, _ in MODULE.TARGETS.values()}, published)
        for target, extension in MODULE.TARGETS.values():
            with self.subTest(target=target):
                expected = "zip" if "windows" in target else "tar.gz"
                self.assertEqual(extension, expected)

    def test_the_soundtrack_names_match_what_the_release_publishes(self) -> None:
        archive, checksum, content = MODULE.soundtrack_paths(Path("/tmp"), "v0.4.0-alpha.23")
        self.assertEqual(archive.name, "numinous-v0.4.0-alpha.23-soundtrack.tar.gz")
        self.assertEqual(checksum.name, f"{archive.name}.sha256")
        self.assertEqual(content.name, f"{archive.name}.content.sha256")


class ReceiptTests(unittest.TestCase):
    def test_the_receipt_keeps_the_limits_it_cannot_prove(self) -> None:
        # The gate reaches "otherwise verifiable", not "signed". A receipt that
        # dropped that distinction would let a reader promote provenance into
        # notarization, which is the exact overclaim this project refuses.
        source = (ROOT / "scripts" / "clean-machine-release.py").read_text(encoding="utf-8")
        for limitation in ("not platform signing", "one platform per run", "no window"):
            self.assertIn(limitation, source)

    def test_a_check_renders_as_the_shape_the_summary_stores(self) -> None:
        check = MODULE.Check("a name", True, "a detail")
        rendered: dict[str, Any] = check.as_json()
        self.assertEqual(rendered, {"name": "a name", "passed": True, "detail": "a detail"})
        self.assertEqual(json.loads(json.dumps(rendered)), rendered)


if __name__ == "__main__":
    unittest.main(verbosity=2 if "-v" in sys.argv else 1)
