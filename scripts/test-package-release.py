#!/usr/bin/env python3
"""Regression tests for release archive construction and verification."""

from __future__ import annotations

import importlib.util
import gzip
import io
from pathlib import Path
import struct
import tarfile
import tempfile
import unittest
from unittest import mock
import zipfile


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
            files = PACKAGE.archive_files(first_archive)
            relative_files = {
                name.split("/", maxsplit=1)[1]
                for name in files
                if "/" in name
            }
            self.assertTrue(
                {
                    "VERIFY.md",
                    "scripts/input-hardware-session.py",
                    "scripts/package-release.py",
                    "scripts/release-engagement-smoke.py",
                }.issubset(relative_files)
            )

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
            content_checksum = Path(f"{archive}.content.sha256")
            self.assertTrue(content_checksum.is_file())

            other_archive, other_checksum = PACKAGE.build_archive(
                "0.2.0-alpha.999",
                "all",
                "soundtrack",
                None,
                radio,
                temp / "other-dist",
            )
            PACKAGE.verify_archive(other_archive, other_checksum)
            self.assertNotEqual(archive.read_bytes(), other_archive.read_bytes())
            self.assertEqual(
                content_checksum.read_bytes(),
                Path(f"{other_archive}.content.sha256").read_bytes(),
            )
            content_checksum.write_text(
                f"{'0' * 64}  {PACKAGE.SOUNDTRACK_CONTENT_LABEL}\n",
                encoding="ascii",
            )
            with self.assertRaisesRegex(ValueError, "content checksum mismatch"):
                PACKAGE.verify_archive(archive, checksum)

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

    def test_archive_expansion_budget_fails_before_member_read(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            archives = (temp / "release.zip", temp / "release.tar.gz")
            payload = {"first": b"a", "second": b"bc"}
            PACKAGE.write_zip(archives[0], "root", payload)
            PACKAGE.write_tar_gz(archives[1], "root", payload)

            zip_reads: list[str] = []
            original_zip_read = zipfile.ZipFile.read

            def record_zip_read(
                archive: zipfile.ZipFile,
                member: zipfile.ZipInfo,
                *args: object,
                **kwargs: object,
            ) -> bytes:
                zip_reads.append(member.filename)
                return original_zip_read(archive, member, *args, **kwargs)

            tar_reads: list[str] = []
            original_tar_read = PACKAGE.read_tar_payload

            def record_tar_read(
                source: object,
                declared_size: int,
                member_name: str,
            ) -> bytes:
                tar_reads.append(member_name)
                return original_tar_read(source, declared_size, member_name)

            with mock.patch.object(
                zipfile.ZipFile,
                "read",
                autospec=True,
                side_effect=record_zip_read,
            ):
                with mock.patch.object(PACKAGE, "MAX_ARCHIVE_ENTRIES", 1):
                    with self.assertRaisesRegex(ValueError, "too many entries"):
                        PACKAGE.archive_files(archives[0])
                self.assertEqual(zip_reads, [])
                for limit, value, message in [
                    ("MAX_ARCHIVE_MEMBER_BYTES", 1, "member is too large"),
                    ("MAX_ARCHIVE_TOTAL_BYTES", 2, "payload is too large"),
                ]:
                    zip_reads.clear()
                    with mock.patch.object(PACKAGE, limit, value):
                        with self.assertRaisesRegex(ValueError, message):
                            PACKAGE.archive_files(archives[0])
                    self.assertEqual(zip_reads, ["root/first"])

            with mock.patch.object(
                PACKAGE,
                "read_tar_payload",
                side_effect=record_tar_read,
            ):
                with mock.patch.object(PACKAGE, "MAX_ARCHIVE_ENTRIES", 1):
                    with self.assertRaisesRegex(ValueError, "too many entries"):
                        PACKAGE.archive_files(archives[1])
                self.assertEqual(tar_reads, ["root/first"])
                for limit, value, message in [
                    ("MAX_ARCHIVE_MEMBER_BYTES", 1, "member is too large"),
                    ("MAX_ARCHIVE_TOTAL_BYTES", 2, "payload is too large"),
                ]:
                    tar_reads.clear()
                    with mock.patch.object(PACKAGE, limit, value):
                        with self.assertRaisesRegex(ValueError, message):
                            PACKAGE.archive_files(archives[1])
                    self.assertEqual(tar_reads, ["root/first"])

    def test_archive_entry_budget_rejects_directory_floods(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            zip_path = temp / "directories.zip"
            with zipfile.ZipFile(zip_path, "w") as archive:
                for index in range(PACKAGE.MAX_ARCHIVE_ENTRIES + 1):
                    archive.writestr(f"root/d{index}/", b"")
            with mock.patch.object(
                PACKAGE.zipfile,
                "ZipFile",
                wraps=PACKAGE.zipfile.ZipFile,
            ) as zip_open:
                with self.assertRaisesRegex(ValueError, "too many entries"):
                    PACKAGE.archive_files(zip_path)
                zip_open.assert_not_called()

            tar_path = temp / "directories.tar.gz"
            with tarfile.open(tar_path, "w:gz") as archive:
                for index in range(PACKAGE.MAX_ARCHIVE_ENTRIES + 1):
                    info = tarfile.TarInfo(f"root/d{index}/")
                    info.type = tarfile.DIRTYPE
                    archive.addfile(info)
            with self.assertRaisesRegex(ValueError, "too many entries"):
                PACKAGE.archive_files(tar_path)

    def test_zip_metadata_preflight_rejects_forged_counts_and_zip64(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            forged = temp / "forged-count.zip"
            with zipfile.ZipFile(forged, "w") as archive:
                for index in range(PACKAGE.MAX_ARCHIVE_ENTRIES + 1):
                    archive.writestr(f"root/d{index}/", b"")
            forged_bytes = bytearray(forged.read_bytes())
            end_offset = forged_bytes.rfind(b"PK\x05\x06")
            self.assertGreaterEqual(end_offset, 0)
            struct.pack_into("<HH", forged_bytes, end_offset + 8, 1, 1)
            forged.write_bytes(forged_bytes)
            with mock.patch.object(
                PACKAGE.zipfile,
                "ZipFile",
                wraps=PACKAGE.zipfile.ZipFile,
            ) as zip_open:
                with self.assertRaisesRegex(ValueError, "too many entries"):
                    PACKAGE.archive_files(forged)
                zip_open.assert_not_called()

            zip64 = temp / "zip64.zip"
            PACKAGE.write_zip(zip64, "root", {"file": b"payload"})
            zip64_bytes = zip64.read_bytes()
            end_offset = zip64_bytes.rfind(b"PK\x05\x06")
            zip64_end = b"PK\x06\x06" + struct.pack("<Q", 44) + bytes(44)
            zip64_locator = struct.pack("<4sIQI", b"PK\x06\x07", 0, end_offset, 1)
            zip64.write_bytes(
                zip64_bytes[:end_offset]
                + zip64_end
                + zip64_locator
                + zip64_bytes[end_offset:]
            )
            with mock.patch.object(
                PACKAGE.zipfile,
                "ZipFile",
                wraps=PACKAGE.zipfile.ZipFile,
            ) as zip_open:
                with self.assertRaisesRegex(ValueError, "ZIP64"):
                    PACKAGE.archive_files(zip64)
                zip_open.assert_not_called()

            local_zip64 = temp / "local-zip64.zip"
            with zipfile.ZipFile(local_zip64, "w") as archive:
                with archive.open("root/file", "w", force_zip64=True) as member:
                    member.write(b"payload")
            with mock.patch.object(
                PACKAGE.zipfile,
                "ZipFile",
                wraps=PACKAGE.zipfile.ZipFile,
            ) as zip_open:
                with self.assertRaisesRegex(ValueError, "ZIP64"):
                    PACKAGE.archive_files(local_zip64)
                zip_open.assert_not_called()

    def test_tar_rejects_hidden_extension_metadata_before_expansion(self) -> None:
        def pax_record(key: bytes, value: bytes) -> bytes:
            body = b" " + key + b"=" + value + b"\n"
            length = len(body) + 1
            while True:
                encoded = str(length).encode() + body
                if len(encoded) == length:
                    return encoded
                length = len(encoded)

        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            cases = [
                (
                    "pax",
                    tarfile.XHDTYPE,
                    pax_record(b"comment", b"a" * 1_000_000),
                ),
                ("gnu-long-name", tarfile.GNUTYPE_LONGNAME, b"a" * 1_000_000 + b"\0"),
            ]
            for label, entry_type, body in cases:
                archive_path = temp / f"{label}.tar.gz"
                with tarfile.open(archive_path, "w:gz") as archive:
                    extension = tarfile.TarInfo("extension")
                    extension.type = entry_type
                    extension.size = len(body)
                    archive.addfile(extension, io.BytesIO(body))
                    regular = tarfile.TarInfo("root/file")
                    regular.size = 1
                    archive.addfile(regular, io.BytesIO(b"x"))
                with self.subTest(label=label):
                    with self.assertRaisesRegex(ValueError, "unsupported tar entry type"):
                        PACKAGE.archive_files(archive_path)

    def test_tar_rejects_non_ustar_headers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "non-ustar.tar.gz"
            PACKAGE.write_tar_gz(archive_path, "root", {"file": b"payload"})
            with gzip.open(archive_path, "rb") as source:
                contents = bytearray(source.read())
            contents[257:265] = bytes(8)
            contents[148:156] = b"        "
            checksum = sum(contents[:512])
            contents[148:156] = f"{checksum:06o}\0 ".encode()
            with gzip.GzipFile(filename=archive_path, mode="wb", mtime=0) as destination:
                destination.write(contents)
            with self.assertRaisesRegex(ValueError, "not canonical ustar"):
                PACKAGE.archive_files(archive_path)

    def test_archive_declared_and_actual_sizes_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            zip_path = temp / "release.zip"
            tar_path = temp / "release.tar.gz"
            payload = {"file": b"payload"}
            PACKAGE.write_zip(zip_path, "root", payload)
            PACKAGE.write_tar_gz(tar_path, "root", payload)

            with mock.patch.object(zipfile.ZipFile, "read", return_value=b""):
                with self.assertRaisesRegex(ValueError, "size mismatch"):
                    PACKAGE.archive_files(zip_path)
            with mock.patch.object(PACKAGE, "read_tar_payload", return_value=b""):
                with self.assertRaisesRegex(ValueError, "size mismatch"):
                    PACKAGE.archive_files(tar_path)

if __name__ == "__main__":
    unittest.main(verbosity=2)
