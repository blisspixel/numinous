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


def elf_fixture() -> bytes:
    data = bytearray(512)
    identity = b"\x7fELF\x02\x01\x01" + b"\0" * 9
    struct.pack_into(
        "<16sHHIQQQIHHHHHH",
        data,
        0,
        identity,
        2,
        62,
        1,
        0x400010,
        64,
        0,
        0,
        64,
        56,
        3,
        0,
        0,
        0,
    )
    struct.pack_into("<IIQQQQQQ", data, 64, 1, 5, 0, 0x400000, 0, 512, 512, 8)
    struct.pack_into("<IIQQQQQQ", data, 120, 2, 4, 256, 0x400100, 0, 80, 80, 8)
    struct.pack_into("<IIQQQQQQ", data, 176, 3, 4, 352, 0x400160, 0, 8, 8, 1)
    data[352:359] = b"/ld.so\0"
    strings = b"\0libc.so.6\0libm.so.6\0"
    data[384 : 384 + len(strings)] = strings
    entries = ((5, 0x400180), (10, len(strings)), (1, 1), (1, 11), (0, 0))
    for index, entry in enumerate(entries):
        struct.pack_into("<QQ", data, 256 + index * 16, *entry)
    return bytes(data)


def mach_fixture(cpu_type: int = 0x01000007) -> bytes:
    commands = bytearray()
    for name in (b"/usr/lib/libSystem.B.dylib", b"@rpath/libexample.dylib"):
        command_size = (24 + len(name) + 1 + 7) & ~7
        command = bytearray(command_size)
        struct.pack_into("<IIIIII", command, 0, 0xC, command_size, 24, 0, 0, 0)
        command[24 : 24 + len(name)] = name
        commands.extend(command)
    header = struct.pack(
        "<IIIIIIII",
        0xFEEDFACF,
        cpu_type,
        3,
        2,
        2,
        len(commands),
        0,
        0,
    )
    return header + commands


def pe_fixture() -> bytes:
    data = bytearray(1024)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    coff = 0x84
    struct.pack_into("<HHIIIHH", data, coff, 0x8664, 1, 0, 0, 0, 240, 0x22)
    optional = coff + 20
    struct.pack_into("<H", data, optional, 0x20B)
    struct.pack_into("<Q", data, optional + 24, 0x140000000)
    struct.pack_into("<I", data, optional + 60, 0x200)
    struct.pack_into("<I", data, optional + 108, 16)
    struct.pack_into("<II", data, optional + 120, 0x1000, 60)
    struct.pack_into("<II", data, optional + 216, 0x1080, 64)
    section = optional + 240
    struct.pack_into(
        "<8sIIIIIIHHI",
        data,
        section,
        b".rdata\0\0",
        0x200,
        0x1000,
        0x200,
        0x200,
        0,
        0,
        0,
        0,
        0x40000040,
    )
    struct.pack_into("<IIIII", data, 0x200, 0x1060, 0, 0, 0x1040, 0x1070)
    struct.pack_into("<IIIII", data, 0x214, 0x1060, 0, 0, 0x1050, 0x1070)
    data[0x240:0x24D] = b"KERNEL32.dll\0"
    data[0x250:0x25B] = b"USER32.dll\0"
    struct.pack_into("<IIIIIIII", data, 0x280, 1, 0x10C0, 0, 0, 0, 0, 0, 0)
    data[0x2C0:0x2CC] = b"VERSION.dll\0"
    return bytes(data)


class ReleasePackageTests(unittest.TestCase):
    def test_mac_launcher_icon_is_a_bounded_native_icns_container(self) -> None:
        data = (ROOT / "assets" / "logo.icns").read_bytes()
        self.assertLess(len(data), 1024 * 1024)
        self.assertEqual(data[:4], b"icns")
        self.assertEqual(int.from_bytes(data[4:8], "big"), len(data))
        self.assertEqual(data[8:12], b"ic08")
        self.assertEqual(int.from_bytes(data[12:16], "big"), len(data) - 8)
        self.assertEqual(data[16:24], b"\x89PNG\r\n\x1a\n")

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
                name.split("/", maxsplit=1)[1] for name in files if "/" in name
            }
            self.assertTrue(
                {
                    "VERIFY.md",
                    "assets/logo.icns",
                    "assets/logo.png",
                    "plugins/numinous/plugin.json",
                    "plugins/numinous/mcp.json",
                    "plugins/numinous/skills/play-numinous/SKILL.md",
                    "scripts/input-hardware-session.py",
                    "scripts/package-release.py",
                    "scripts/release-engagement-smoke.py",
                }.issubset(relative_files)
            )

    def test_native_binary_parsers_report_exact_imports(self) -> None:
        self.assertEqual(
            PACKAGE.exact_imports(
                ["KERNEL32.dll", "kernel32.dll"], "PE", lowercase=True
            ),
            ["kernel32.dll"],
        )
        fixtures = (
            (
                elf_fixture(),
                "x86_64-unknown-linux-gnu",
                "ELF",
                "x86_64",
                ["libc.so.6", "libm.so.6"],
            ),
            (
                mach_fixture(),
                "x86_64-apple-darwin",
                "Mach-O",
                "x86_64",
                ["/usr/lib/libSystem.B.dylib", "@rpath/libexample.dylib"],
            ),
            (
                mach_fixture(0x0100000C),
                "aarch64-apple-darwin",
                "Mach-O",
                "aarch64",
                ["/usr/lib/libSystem.B.dylib", "@rpath/libexample.dylib"],
            ),
            (
                pe_fixture(),
                "x86_64-pc-windows-msvc",
                "PE",
                "x86_64",
                ["kernel32.dll", "user32.dll", "version.dll"],
            ),
        )
        for data, target, binary_format, architecture, imports in fixtures:
            with self.subTest(target=target):
                result = PACKAGE.inspect_native_binary(data, target)
                self.assertEqual(result["format"], binary_format)
                self.assertEqual(result["architecture"], architecture)
                self.assertEqual(result["imports"], sorted(imports))
                self.assertEqual(
                    result["sha1"],
                    PACKAGE.hashlib.sha1(data, usedforsecurity=False).hexdigest(),
                )
                self.assertEqual(result["sha256"], PACKAGE.sha256_bytes(data))

    def test_native_binary_parsers_reject_wrong_targets_and_malformed_tables(
        self,
    ) -> None:
        with self.assertRaisesRegex(ValueError, "does not match"):
            PACKAGE.inspect_native_binary(
                mach_fixture(0x0100000C), "x86_64-apple-darwin"
            )
        malformed_elf = bytearray(elf_fixture())
        struct.pack_into("<Q", malformed_elf, 32, len(malformed_elf))
        with self.assertRaisesRegex(ValueError, "program headers"):
            PACKAGE.inspect_native_binary(
                bytes(malformed_elf), "x86_64-unknown-linux-gnu"
            )
        non_executable_elf = bytearray(elf_fixture())
        struct.pack_into("<H", non_executable_elf, 16, 0)
        with self.assertRaisesRegex(ValueError, "not an executable"):
            PACKAGE.inspect_native_binary(
                bytes(non_executable_elf), "x86_64-unknown-linux-gnu"
            )
        ambiguous_dynamic_elf = bytearray(elf_fixture())
        struct.pack_into("<H", ambiguous_dynamic_elf, 16, 3)
        struct.pack_into("<I", ambiguous_dynamic_elf, 176, 4)
        with self.assertRaisesRegex(ValueError, "no unique interpreter"):
            PACKAGE.inspect_native_binary(
                bytes(ambiguous_dynamic_elf), "x86_64-unknown-linux-gnu"
            )
        malformed_mach = bytearray(mach_fixture())
        struct.pack_into("<I", malformed_mach, 36, 7)
        with self.assertRaisesRegex(ValueError, "command size"):
            PACKAGE.inspect_native_binary(bytes(malformed_mach), "x86_64-apple-darwin")
        malformed_pe = bytearray(pe_fixture())
        malformed_pe[0x240:0x400] = b"A" * (0x400 - 0x240)
        with self.assertRaisesRegex(ValueError, "terminator"):
            PACKAGE.inspect_native_binary(bytes(malformed_pe), "x86_64-pc-windows-msvc")
        missing_pe_name = bytearray(pe_fixture())
        struct.pack_into("<I", missing_pe_name, 0x20C, 0)
        with self.assertRaisesRegex(ValueError, "missing a name"):
            PACKAGE.inspect_native_binary(
                bytes(missing_pe_name), "x86_64-pc-windows-msvc"
            )
        non_executable_pe = bytearray(pe_fixture())
        struct.pack_into("<H", non_executable_pe, 0x96, 0x20)
        with self.assertRaisesRegex(ValueError, "not an executable"):
            PACKAGE.inspect_native_binary(
                bytes(non_executable_pe), "x86_64-pc-windows-msvc"
            )

    def test_verified_native_archive_inventory_binds_every_executable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            binaries = temp / "binaries"
            binaries.mkdir()
            for name in PACKAGE.BINARIES:
                (binaries / f"{name}.exe").write_bytes(pe_fixture())
            version = PACKAGE.workspace_version()
            archive, checksum = PACKAGE.build_archive(
                version,
                "x86_64-pc-windows-msvc",
                "binaries",
                binaries,
                ROOT / "assets" / "radio",
                temp / "dist",
            )
            original_open = Path.open
            archive_open_count = 0

            def record_open(path: Path, *args: object, **kwargs: object) -> object:
                nonlocal archive_open_count
                if path == archive:
                    archive_open_count += 1
                return original_open(path, *args, **kwargs)

            with mock.patch.object(
                Path, "open", autospec=True, side_effect=record_open
            ):
                inventory = PACKAGE.native_archive_inventory(archive, checksum)
            self.assertEqual(archive_open_count, 1)
            self.assertEqual(len(inventory), 3)
            self.assertEqual(
                {item["fileName"] for item in inventory},
                {f"bin/{name}.exe" for name in PACKAGE.BINARIES},
            )
            self.assertTrue(
                all(
                    item["target"] == "x86_64-pc-windows-msvc"
                    and item["version"] == version
                    and item["imports"] == ["kernel32.dll", "user32.dll", "version.dll"]
                    for item in inventory
                )
            )

    def test_binary_archive_rejects_an_extra_manifested_payload(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            binaries = temp / "binaries"
            binaries.mkdir()
            for name in PACKAGE.BINARIES:
                (binaries / f"{name}.exe").write_bytes(pe_fixture())
            version = PACKAGE.workspace_version()
            payload = PACKAGE.release_payload(
                version, "x86_64-pc-windows-msvc", binaries
            )
            payload["extra.bin"] = b"unexpected"
            archive = temp / f"numinous-v{version}-x86_64-pc-windows-msvc.zip"
            PACKAGE.write_zip(
                archive,
                PACKAGE.archive_root(version, "x86_64-pc-windows-msvc", "binaries"),
                PACKAGE.add_manifest(payload),
            )
            checksum = Path(f"{archive}.sha256")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "inventory is not exact"):
                PACKAGE.verify_archive(archive, checksum)
            payload.pop("extra.bin")
            PACKAGE.write_zip(archive, "wrong-root", PACKAGE.add_manifest(payload))
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "members do not match"):
                PACKAGE.verify_archive(archive, checksum)
            mismatched_payload = dict(payload)
            mismatched_payload["RELEASE.json"] = PACKAGE.release_metadata(
                "9.9.9", "x86_64-pc-windows-msvc", "binaries", "a" * 40
            )
            PACKAGE.write_zip(
                archive,
                PACKAGE.archive_root("9.9.9", "x86_64-pc-windows-msvc", "binaries"),
                PACKAGE.add_manifest(mismatched_payload),
            )
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "name does not match"):
                PACKAGE.verify_archive(archive, checksum)
            expected_root = PACKAGE.archive_root(
                version, "x86_64-pc-windows-msvc", "binaries"
            )
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            with zipfile.ZipFile(archive, "a") as release_zip:
                directory = zipfile.ZipInfo("wrong-root/", (1980, 1, 1, 0, 0, 0))
                directory.external_attr = (0o40755 << 16) | 0x10
                release_zip.writestr(directory, b"")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "members do not match"):
                PACKAGE.verify_archive(archive, checksum)
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            with zipfile.ZipFile(archive, "a") as release_zip:
                collision = zipfile.ZipInfo(
                    f"{expected_root}/README.md/", (1980, 1, 1, 0, 0, 0)
                )
                collision.external_attr = (0o40755 << 16) | 0x10
                release_zip.writestr(collision, b"")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "duplicate archive member"):
                PACKAGE.verify_archive(archive, checksum)
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            with zipfile.ZipFile(archive, "a") as release_zip:
                descendant = zipfile.ZipInfo(
                    f"{expected_root}/README.md/child/", (1980, 1, 1, 0, 0, 0)
                )
                descendant.external_attr = (0o40755 << 16) | 0x10
                release_zip.writestr(descendant, b"")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "path collides with a file"):
                PACKAGE.verify_archive(archive, checksum)
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            with zipfile.ZipFile(archive, "a") as release_zip:
                casefold_descendant = zipfile.ZipInfo(
                    f"{expected_root}/readme.md/child/", (1980, 1, 1, 0, 0, 0)
                )
                casefold_descendant.external_attr = (0o40755 << 16) | 0x10
                release_zip.writestr(casefold_descendant, b"")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "path collides with a file"):
                PACKAGE.verify_archive(archive, checksum)
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            with zipfile.ZipFile(archive, "a") as release_zip:
                disguised_link = zipfile.ZipInfo(
                    f"{expected_root}/extra/", (1980, 1, 1, 0, 0, 0)
                )
                disguised_link.external_attr = (0o120777 << 16) | 0x10
                release_zip.writestr(disguised_link, b"")
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "invalid ZIP directory"):
                PACKAGE.verify_archive(archive, checksum)
            for file_first in (True, False):
                admitted_names: set[str] = set()
                admitted_files: set[str] = set()
                first = "root/README.md" if file_first else "root/readme.md/child"
                second = "root/readme.md/child" if file_first else "root/README.md"
                PACKAGE.admit_archive_name(
                    admitted_names,
                    admitted_files,
                    first,
                    is_directory=not file_first,
                )
                with self.assertRaisesRegex(ValueError, "path collides with a file"):
                    PACKAGE.admit_archive_name(
                        admitted_names,
                        admitted_files,
                        second,
                        is_directory=file_first,
                    )
            PACKAGE.write_zip(archive, expected_root, PACKAGE.add_manifest(payload))
            checksum.write_text(
                f"{PACKAGE.sha256_file(archive)}  {archive.name}\n", encoding="ascii"
            )
            with self.assertRaisesRegex(ValueError, "commit does not match"):
                PACKAGE.verify_archive(
                    archive,
                    checksum,
                    expected_version=version,
                    expected_revision="0" * 40,
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
        for invalid in ("v0.2.0", "../0.2.0", "0.2.0/escape", "0.2", None):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    PACKAGE.validate_version(invalid)
        for invalid in (
            "/absolute/file",
            "root/../escape",
            "root//file",
            "root/./file",
            "./root/file",
            "root/file/.",
            "root/.",
            "root/CON/file",
            "root/name./file",
            "root/has space/file",
            "root/file:stream",
            "root/caf\N{LATIN SMALL LETTER E WITH ACUTE}/file",
        ):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    PACKAGE.safe_member_name(invalid)

    def test_release_metadata_is_exact_and_duplicate_safe(self) -> None:
        exact = PACKAGE.release_metadata(
            "1.2.3", "x86_64-pc-windows-msvc", "binaries", "a" * 40
        )
        self.assertEqual(PACKAGE.parse_release_metadata(exact)["version"], "1.2.3")
        duplicate = exact.replace(
            b'  "version": "1.2.3"',
            b'  "version": "1.2.3",\n  "version": "1.2.3"',
        )
        with self.assertRaisesRegex(ValueError, "repeats key"):
            PACKAGE.parse_release_metadata(duplicate)
        extra = exact.replace(b"{\n", b'{\n  "extra": true,\n', 1)
        with self.assertRaisesRegex(ValueError, "shape is not exact"):
            PACKAGE.parse_release_metadata(extra)
        inconsistent = exact.replace(b'"signed": false', b'"signed": true')
        with self.assertRaisesRegex(ValueError, "identity is inconsistent"):
            PACKAGE.parse_release_metadata(inconsistent)
        boolean_schema_version = exact.replace(
            b'"schemaVersion": 1', b'"schemaVersion": true'
        )
        with self.assertRaisesRegex(ValueError, "schema is unsupported"):
            PACKAGE.parse_release_metadata(boolean_schema_version)
        for field, original, replacement in (
            ("kind", b'"kind": "binaries"', b'"kind": []'),
            (
                "target",
                b'"target": "x86_64-pc-windows-msvc"',
                b'"target": {}',
            ),
        ):
            malformed_type = exact.replace(original, replacement)
            with self.subTest(field=field):
                with self.assertRaisesRegex(ValueError, "kind or target is malformed"):
                    PACKAGE.parse_release_metadata(malformed_type)

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
                    with self.assertRaisesRegex(
                        ValueError, "unsupported tar entry type"
                    ):
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
            with gzip.GzipFile(
                filename=archive_path, mode="wb", mtime=0
            ) as destination:
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
