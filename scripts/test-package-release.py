#!/usr/bin/env python3
"""Regression tests for release archive construction and verification."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_package_release", ROOT / "scripts" / "package-release.py"
)
assert SPEC is not None and SPEC.loader is not None
PACKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGE)


class ReleasePackageTests(unittest.TestCase):
    def test_binary_archives_are_deterministic_and_verified(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            binaries = temp / "binaries"
            binaries.mkdir()
            for name in PACKAGE.BINARIES:
                (binaries / f"{name}.exe").write_bytes(f"binary:{name}".encode())
            first = temp / "first"
            second = temp / "second"
            first_archive, first_checksum = PACKAGE.build_archive(
                PACKAGE.workspace_version(),
                "x86_64-pc-windows-msvc",
                "binaries",
                binaries,
                ROOT / "assets" / "radio",
                first,
            )
            second_archive, second_checksum = PACKAGE.build_archive(
                PACKAGE.workspace_version(),
                "x86_64-pc-windows-msvc",
                "binaries",
                binaries,
                ROOT / "assets" / "radio",
                second,
            )
            self.assertEqual(first_archive.read_bytes(), second_archive.read_bytes())
            self.assertEqual(first_checksum.read_text(), second_checksum.read_text())
            PACKAGE.verify_archive(first_archive, first_checksum)

    def test_soundtrack_archive_has_a_closed_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            radio = temp / "radio"
            radio.mkdir()
            (radio / "ASSET-LICENSE.txt").write_text("fixture license\n")
            (radio / "test-001.mp3").write_bytes(b"ID3 fixture")
            archive, checksum = PACKAGE.build_archive(
                PACKAGE.workspace_version(),
                "all",
                "soundtrack",
                None,
                radio,
                temp / "dist",
            )
            PACKAGE.verify_archive(archive, checksum)
            files = PACKAGE.archive_files(archive)
            self.assertTrue(any(name.endswith("radio/test-001.mp3") for name in files))

    def test_verifier_rejects_a_sidecar_for_another_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            archive = temp / "release.zip"
            archive.write_bytes(b"not an archive")
            checksum = temp / "release.zip.sha256"
            checksum.write_text(f"{'0' * 64}  another.zip\n")
            with self.assertRaisesRegex(ValueError, "names another archive"):
                PACKAGE.verify_archive(archive, checksum)

    def test_version_and_archive_paths_fail_closed(self) -> None:
        for invalid in ("v0.2.0", "../0.2.0", "0.2.0/escape", "0.2"):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    PACKAGE.validate_version(invalid)
        for invalid in ("/absolute/file", "root/../escape", "root//file"):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    PACKAGE.safe_member_name(invalid)


if __name__ == "__main__":
    unittest.main(verbosity=2)
