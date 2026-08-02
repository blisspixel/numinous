#!/usr/bin/env python3
"""Build and verify deterministic Numinous release archives."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import subprocess
import tarfile
import tempfile
from typing import BinaryIO
import zipfile


ROOT = Path(__file__).resolve().parent.parent
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
PORTABLE_ARCHIVE_SEGMENT = re.compile(r"[0-9A-Za-z._-]+")
WINDOWS_RESERVED_SEGMENTS = frozenset(
    {"aux", "con", "nul", "prn"}
    | {f"com{index}" for index in range(1, 10)}
    | {f"lpt{index}" for index in range(1, 10)}
)
TARGETS = {
    "x86_64-pc-windows-msvc": (".exe", "zip"),
    "x86_64-unknown-linux-gnu": ("", "tar.gz"),
    "x86_64-apple-darwin": ("", "tar.gz"),
    "aarch64-apple-darwin": ("", "tar.gz"),
}
BINARIES = ("numinous", "numinous-app", "numinous-mcp")
RELEASE_FILES = (
    "LICENSE",
    "PLAY.md",
    "README.md",
    "VERIFY.md",
    "scripts/input-hardware-session.py",
    "scripts/install.ps1",
    "scripts/install.sh",
    "scripts/package-release.py",
    "scripts/release-engagement-smoke.py",
)
SOUNDTRACK_CONTENT_LABEL = "soundtrack-content-v1"
MAX_ARCHIVE_ENTRIES = 256
MAX_ARCHIVE_MEMBER_BYTES = 256 * 1024 * 1024
MAX_ARCHIVE_TOTAL_BYTES = 512 * 1024 * 1024
MAX_ARCHIVE_METADATA_BYTES = 16 * 1024 * 1024
MAX_ARCHIVE_FILE_BYTES = MAX_ARCHIVE_TOTAL_BYTES + MAX_ARCHIVE_METADATA_BYTES
MAX_TAR_TRAILING_BYTES = 16 * 1024
MAX_NATIVE_IMPORTS = 4096
MAX_NATIVE_NAME_BYTES = 4096
MAX_NATIVE_PROGRAM_HEADERS = 256
MAX_NATIVE_SECTIONS = 96
MAX_MACH_LOAD_COMMANDS = 4096
ZIP_END_RECORD_SIZE = 22
ZIP_MAX_COMMENT_BYTES = 65_535
ZIP_CENTRAL_HEADER_SIZE = 46
TAR_BLOCK_BYTES = 512
NATIVE_TARGETS = {
    "x86_64-pc-windows-msvc": ("PE", "x86_64"),
    "x86_64-unknown-linux-gnu": ("ELF", "x86_64"),
    "x86_64-apple-darwin": ("Mach-O", "x86_64"),
    "aarch64-apple-darwin": ("Mach-O", "aarch64"),
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def workspace_version(root: Path = ROOT) -> str:
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    in_workspace_package = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_workspace_package = stripped == "[workspace.package]"
        elif in_workspace_package:
            match = re.fullmatch(r'version\s*=\s*"([^"]+)"', stripped)
            if match:
                version = match.group(1)
                validate_version(version)
                return version
    raise ValueError("Cargo.toml has no workspace package version")


def validate_version(version: str) -> None:
    if not isinstance(version, str) or not VERSION_RE.fullmatch(version):
        raise ValueError(f"invalid release version: {version!r}")


def commit_sha(root: Path = ROOT) -> str:
    from_environment = os.environ.get("GITHUB_SHA", "").strip()
    if re.fullmatch(r"[0-9a-f]{40}", from_environment):
        return from_environment
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    value = result.stdout.strip()
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("git did not return a complete commit SHA")
    return value


def release_metadata(version: str, target: str, kind: str, commit: str) -> bytes:
    payload = {
        "commit": commit,
        "kind": kind,
        "schema": "numinous.release",
        "schemaVersion": 1,
        "signed": False,
        "tag": f"v{version}",
        "target": target,
        "version": version,
    }
    return (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode()


def parse_release_metadata(data: bytes) -> dict[str, object]:
    """Parse one exact release identity object without duplicate JSON keys."""

    def object_from_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"release metadata repeats key {key!r}")
            result[key] = value
        return result

    try:
        metadata = json.loads(data, object_pairs_hook=object_from_pairs)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("release metadata is malformed") from error
    expected_keys = {
        "commit",
        "kind",
        "schema",
        "schemaVersion",
        "signed",
        "tag",
        "target",
        "version",
    }
    if not isinstance(metadata, dict) or set(metadata) != expected_keys:
        raise ValueError("release metadata shape is not exact")
    if (
        metadata["schema"] != "numinous.release"
        or type(metadata["schemaVersion"]) is not int
        or metadata["schemaVersion"] != 1
    ):
        raise ValueError("release metadata schema is unsupported")
    version = metadata["version"]
    validate_version(version)
    revision = metadata["commit"]
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ValueError("release metadata commit is malformed")
    kind = metadata["kind"]
    target = metadata["target"]
    if (
        not isinstance(kind, str)
        or not 0 < len(kind) <= 32
        or not isinstance(target, str)
        or not 0 < len(target) <= 128
    ):
        raise ValueError("release metadata kind or target is malformed")
    if metadata["signed"] is not False or metadata["tag"] != f"v{version}":
        raise ValueError("release metadata identity is inconsistent")
    return metadata


def release_payload(
    version: str,
    target: str,
    binary_dir: Path,
    root: Path = ROOT,
) -> dict[str, bytes]:
    if target not in TARGETS:
        raise ValueError(f"unsupported release target: {target}")
    suffix, _archive_format = TARGETS[target]
    payload: dict[str, bytes] = {}
    for name in BINARIES:
        source = binary_dir / f"{name}{suffix}"
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"missing ordinary release binary: {source}")
        payload[f"bin/{source.name}"] = source.read_bytes()
    for relative in RELEASE_FILES:
        source = root / relative
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"missing ordinary release file: {source}")
        payload[relative] = source.read_bytes()
    payload["RELEASE.json"] = release_metadata(
        version, target, "binaries", commit_sha(root)
    )
    return payload


def soundtrack_payload(
    version: str,
    radio_dir: Path,
    root: Path = ROOT,
) -> dict[str, bytes]:
    if not radio_dir.is_dir() or radio_dir.is_symlink():
        raise ValueError(f"missing ordinary soundtrack directory: {radio_dir}")
    payload: dict[str, bytes] = {}
    for source in sorted(radio_dir.iterdir(), key=lambda path: path.name):
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"soundtrack entry is not an ordinary file: {source}")
        payload[f"radio/{source.name}"] = source.read_bytes()
    mp3_count = sum(path.endswith(".mp3") for path in payload)
    if radio_dir == root / "assets" / "radio" and mp3_count != 42:
        raise ValueError(f"expected 42 canonical MP3 tracks, found {mp3_count}")
    if "radio/ASSET-LICENSE.txt" not in payload:
        raise ValueError("soundtrack payload is missing ASSET-LICENSE.txt")
    payload["RELEASE.json"] = release_metadata(
        version, "all", "soundtrack", commit_sha(root)
    )
    return payload


def soundtrack_content_hash(payload: dict[str, bytes]) -> str:
    entries = [
        f"{sha256_bytes(data)}  {name}"
        for name, data in sorted(payload.items())
        if name.startswith("radio/")
    ]
    if "radio/ASSET-LICENSE.txt" not in payload:
        raise ValueError("soundtrack content is missing ASSET-LICENSE.txt")
    if not any(name.startswith("radio/") and name.endswith(".mp3") for name in payload):
        raise ValueError("soundtrack content contains no MP3 tracks")
    return sha256_bytes(("\n".join(entries) + "\n").encode("ascii"))


def add_manifest(payload: dict[str, bytes]) -> dict[str, bytes]:
    if "MANIFEST.sha256" in payload:
        raise ValueError("payload reserved MANIFEST.sha256")
    lines = [f"{sha256_bytes(data)}  {name}" for name, data in sorted(payload.items())]
    with_manifest = dict(payload)
    with_manifest["MANIFEST.sha256"] = ("\n".join(lines) + "\n").encode()
    return with_manifest


def archive_root(version: str, target: str, kind: str) -> str:
    if kind == "soundtrack":
        return f"numinous-v{version}-soundtrack"
    return f"numinous-v{version}-{target}"


def file_mode(relative: str) -> int:
    if relative.startswith("bin/") or relative.endswith(".sh"):
        return 0o755
    return 0o644


def write_zip(path: Path, root_name: str, payload: dict[str, bytes]) -> None:
    with zipfile.ZipFile(
        path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for relative, data in sorted(payload.items()):
            info = zipfile.ZipInfo(f"{root_name}/{relative}", (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = file_mode(relative) << 16
            archive.writestr(info, data)


def write_tar_gz(path: Path, root_name: str, payload: dict[str, bytes]) -> None:
    with path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for relative, data in sorted(payload.items()):
                    info = tarfile.TarInfo(f"{root_name}/{relative}")
                    info.size = len(data)
                    info.mode = file_mode(relative)
                    info.mtime = 0
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    archive.addfile(info, io.BytesIO(data))


def build_archive(
    version: str,
    target: str,
    kind: str,
    binary_dir: Path | None,
    radio_dir: Path,
    output_dir: Path,
    root: Path = ROOT,
) -> tuple[Path, Path]:
    validate_version(version)
    content_hash: str | None = None
    if kind == "binaries":
        if target not in TARGETS or binary_dir is None:
            raise ValueError(
                "binary packaging requires a supported target and binary directory"
            )
        payload = release_payload(version, target, binary_dir, root)
        archive_format = TARGETS[target][1]
    else:
        if target != "all":
            raise ValueError("soundtrack packaging uses target 'all'")
        payload = soundtrack_payload(version, radio_dir, root)
        content_hash = soundtrack_content_hash(payload)
        archive_format = "tar.gz"
    payload = add_manifest(payload)
    root_name = archive_root(version, target, kind)
    extension = ".zip" if archive_format == "zip" else ".tar.gz"
    output_dir.mkdir(parents=True, exist_ok=True)
    archive_path = output_dir / f"{root_name}{extension}"
    if archive_format == "zip":
        write_zip(archive_path, root_name, payload)
    else:
        write_tar_gz(archive_path, root_name, payload)
    checksum_path = Path(f"{archive_path}.sha256")
    checksum_path.write_text(
        f"{sha256_file(archive_path)}  {archive_path.name}\n", encoding="ascii"
    )
    if content_hash is not None:
        Path(f"{archive_path}.content.sha256").write_text(
            f"{content_hash}  {SOUNDTRACK_CONTENT_LABEL}\n", encoding="ascii"
        )
    verify_archive(archive_path, checksum_path)
    return archive_path, checksum_path


def safe_member_name(name: str) -> PurePosixPath:
    if not isinstance(name, str) or not name or len(name) > 4096:
        raise ValueError(f"unsafe archive member: {name!r}")
    if "\\" in name or "//" in name:
        raise ValueError(f"unsafe archive member: {name!r}")
    raw_name = name[:-1] if name.endswith("/") else name
    raw_parts = raw_name.split("/")
    if not raw_name or any(part in ("", ".", "..") for part in raw_parts):
        raise ValueError(f"unsafe archive member: {name!r}")
    path = PurePosixPath(raw_name)
    if path.is_absolute() or not path.parts or path.as_posix() != raw_name:
        raise ValueError(f"unsafe archive member: {name!r}")
    for part in path.parts:
        folded_base = part.casefold().split(".", maxsplit=1)[0]
        if (
            len(part) > 255
            or PORTABLE_ARCHIVE_SEGMENT.fullmatch(part) is None
            or part.endswith(".")
            or folded_base in WINDOWS_RESERVED_SEGMENTS
        ):
            raise ValueError(f"nonportable archive member: {name!r}")
    return path


def admit_archive_entry(entry_count: int) -> int:
    """Admit one archive entry within the metadata work budget."""
    if entry_count >= MAX_ARCHIVE_ENTRIES:
        raise ValueError("release archive contains too many entries")
    return entry_count + 1


def admit_archive_name(
    admitted_names: set[str],
    admitted_files: set[str],
    normalized: str,
    *,
    is_directory: bool,
) -> None:
    """Reject duplicate names and file-as-directory hierarchy collisions."""
    collision_key = normalized.casefold()
    if collision_key in admitted_names:
        raise ValueError(f"duplicate archive member: {normalized}")
    if any(
        collision_key.startswith(f"{file_name}/") for file_name in admitted_files
    ) or (
        not is_directory
        and any(name.startswith(f"{collision_key}/") for name in admitted_names)
    ):
        raise ValueError(f"archive member path collides with a file: {normalized}")
    admitted_names.add(collision_key)
    if not is_directory:
        admitted_files.add(collision_key)


def admit_archive_payload(total_bytes: int, declared_size: int) -> int:
    """Admit one regular member within the expanded payload budget."""
    if declared_size < 0 or declared_size > MAX_ARCHIVE_MEMBER_BYTES:
        raise ValueError("release archive member is too large")
    if total_bytes > MAX_ARCHIVE_TOTAL_BYTES - declared_size:
        raise ValueError("release archive payload is too large")
    return total_bytes + declared_size


def reject_zip64_extra(extra: bytes) -> None:
    """Reject ZIP64 and malformed extra fields before archive expansion."""
    position = 0
    while position < len(extra):
        if len(extra) - position < 4:
            raise ValueError("release ZIP extra metadata is malformed")
        header_id, field_size = struct.unpack_from("<HH", extra, position)
        position += 4
        if position + field_size > len(extra):
            raise ValueError("release ZIP extra metadata is truncated")
        if header_id == 0x0001:
            raise ValueError("ZIP64 release archives are unsupported")
        position += field_size


def zip_archive_entry_count(source: BinaryIO) -> int:
    """Validate and count classic ZIP metadata before ZipFile constructs it."""
    size = source.seek(0, os.SEEK_END)
    tail_size = min(size, ZIP_END_RECORD_SIZE + ZIP_MAX_COMMENT_BYTES)
    source.seek(size - tail_size)
    tail = source.read(tail_size)
    offset = tail.rfind(b"PK\x05\x06")
    if offset < 0 or len(tail) - offset < ZIP_END_RECORD_SIZE:
        raise ValueError("release ZIP end record is missing")
    (
        disk_number,
        directory_disk,
        entries_on_disk,
        entry_count,
        directory_size,
        directory_offset,
        comment_size,
    ) = struct.unpack_from("<4H2IH", tail, offset + 4)
    if offset + ZIP_END_RECORD_SIZE + comment_size != len(tail):
        raise ValueError("release ZIP end record is malformed")
    end_offset = size - tail_size + offset
    if end_offset >= 20:
        source.seek(end_offset - 20)
        if source.read(4) == b"PK\x06\x07":
            raise ValueError("ZIP64 release archives are unsupported")
    if disk_number != 0 or directory_disk != 0 or entries_on_disk != entry_count:
        raise ValueError("multi-disk release ZIP archives are unsupported")
    if (
        entry_count == 0xFFFF
        or directory_size == 0xFFFF_FFFF
        or directory_offset == 0xFFFF_FFFF
    ):
        raise ValueError("ZIP64 release archives are unsupported")
    if directory_size > MAX_ARCHIVE_METADATA_BYTES:
        raise ValueError("release ZIP metadata is too large")
    if directory_offset + directory_size != end_offset:
        raise ValueError("release ZIP central directory is malformed")
    source.seek(directory_offset)
    directory = source.read(directory_size)
    if len(directory) != directory_size:
        raise ValueError("release ZIP central directory is truncated")

    actual_count = 0
    position = 0
    while position < len(directory):
        if (
            len(directory) - position < ZIP_CENTRAL_HEADER_SIZE
            or directory[position : position + 4] != b"PK\x01\x02"
        ):
            raise ValueError("release ZIP central directory is malformed")
        central_flags, central_compression = struct.unpack_from(
            "<HH", directory, position + 8
        )
        compressed_size, uncompressed_size = struct.unpack_from(
            "<II", directory, position + 20
        )
        name_size, extra_size, entry_comment_size = struct.unpack_from(
            "<HHH", directory, position + 28
        )
        disk_start = struct.unpack_from("<H", directory, position + 34)[0]
        local_header_offset = struct.unpack_from("<I", directory, position + 42)[0]
        record_size = (
            ZIP_CENTRAL_HEADER_SIZE + name_size + extra_size + entry_comment_size
        )
        if position + record_size > len(directory):
            raise ValueError("release ZIP central directory is truncated")
        if (
            compressed_size == 0xFFFF_FFFF
            or uncompressed_size == 0xFFFF_FFFF
            or disk_start == 0xFFFF
            or local_header_offset == 0xFFFF_FFFF
        ):
            raise ValueError("ZIP64 release archives are unsupported")
        extra_start = position + ZIP_CENTRAL_HEADER_SIZE + name_size
        extra_end = extra_start + extra_size
        reject_zip64_extra(directory[extra_start:extra_end])
        if local_header_offset + 30 > directory_offset:
            raise ValueError("release ZIP local header is malformed")
        source.seek(local_header_offset)
        local_header = source.read(30)
        if len(local_header) != 30 or local_header[:4] != b"PK\x03\x04":
            raise ValueError("release ZIP local header is malformed")
        local_flags, local_compression = struct.unpack_from("<HH", local_header, 6)
        local_compressed_size, local_uncompressed_size = struct.unpack_from(
            "<II", local_header, 18
        )
        local_name_size, local_extra_size = struct.unpack_from("<HH", local_header, 26)
        local_metadata_end = (
            local_header_offset + 30 + local_name_size + local_extra_size
        )
        if local_metadata_end > directory_offset:
            raise ValueError("release ZIP local header is malformed")
        source.seek(local_header_offset + 30)
        local_metadata = source.read(local_name_size + local_extra_size)
        if len(local_metadata) != local_name_size + local_extra_size:
            raise ValueError("release ZIP local metadata is truncated")
        central_name = directory[
            position + ZIP_CENTRAL_HEADER_SIZE : position
            + ZIP_CENTRAL_HEADER_SIZE
            + name_size
        ]
        local_name = local_metadata[:local_name_size]
        local_extra = local_metadata[local_name_size:]
        if (
            local_name != central_name
            or local_flags != central_flags
            or local_compression != central_compression
            or local_metadata_end + compressed_size > directory_offset
        ):
            raise ValueError("release ZIP local header is inconsistent")
        if (
            local_compressed_size == 0xFFFF_FFFF
            or local_uncompressed_size == 0xFFFF_FFFF
        ):
            raise ValueError("ZIP64 release archives are unsupported")
        reject_zip64_extra(local_extra)
        actual_count = admit_archive_entry(actual_count)
        position += record_size
    if actual_count != entry_count:
        raise ValueError("release ZIP entry count mismatch")
    return actual_count


def parse_tar_number(field: bytes, label: str) -> int:
    """Parse one canonical octal tar field and reject GNU base-256 values."""
    if field and field[0] & 0x80:
        raise ValueError(f"unsupported base-256 tar {label}")
    text = field.strip(b" \0")
    if not text:
        return 0
    if any(byte < ord("0") or byte > ord("7") for byte in text):
        raise ValueError(f"malformed tar {label}")
    return int(text, 8)


def read_tar_payload(source: BinaryIO, declared_size: int, member_name: str) -> bytes:
    """Read one already-admitted tar payload exactly."""
    try:
        data = source.read(declared_size)
    except (EOFError, OSError) as error:
        raise ValueError(f"unreadable tar member: {member_name}") from error
    if len(data) != declared_size:
        raise ValueError(f"archive member size mismatch: {member_name}")
    return data


def canonical_tar_text(field: bytes, label: str) -> str:
    """Decode a canonical zero-padded ustar text field."""
    raw, separator, padding = field.partition(b"\0")
    if separator and any(padding):
        raise ValueError(f"tar {label} has nonzero bytes after its terminator")
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"release tar {label} is not UTF-8") from error


def tar_files(
    snapshot: BinaryIO, admitted_directories: set[str] | None = None
) -> dict[str, bytes]:
    """Read the canonical ustar subset without hidden extension processing."""
    files: dict[str, bytes] = {}
    entry_count = 0
    total_bytes = 0
    zero_blocks = 0
    admitted_names: set[str] = set()
    admitted_files: set[str] = set()
    snapshot.seek(0)
    with gzip.GzipFile(fileobj=snapshot, mode="rb") as source:
        while zero_blocks < 2:
            try:
                header = source.read(TAR_BLOCK_BYTES)
            except (EOFError, OSError) as error:
                raise ValueError("release tar stream is unreadable") from error
            if len(header) != TAR_BLOCK_BYTES:
                raise ValueError("release tar terminator is missing")
            if header == bytes(TAR_BLOCK_BYTES):
                zero_blocks += 1
                continue
            if zero_blocks:
                raise ValueError("release tar contains data after its terminator")

            declared_checksum = parse_tar_number(header[148:156], "checksum")
            actual_checksum = sum(header[:148]) + 8 * ord(" ") + sum(header[156:])
            if declared_checksum != actual_checksum:
                raise ValueError("release tar header checksum mismatch")
            if header[257:263] != b"ustar\0" or header[263:265] != b"00":
                raise ValueError("release tar is not canonical ustar")
            declared_size = parse_tar_number(header[124:136], "member size")
            name = canonical_tar_text(header[:100], "member name")
            prefix = canonical_tar_text(header[345:500], "member prefix")
            if prefix:
                name = f"{prefix}/{name}"
            normalized = safe_member_name(name).as_posix()
            entry_count = admit_archive_entry(entry_count)
            entry_type = header[156:157]
            if entry_type not in (b"\0", b"0", b"5"):
                raise ValueError(f"unsupported tar entry type: {entry_type!r}")
            if entry_type != b"5" and name.endswith("/"):
                raise ValueError(f"tar file name ends with a directory marker: {name}")
            admit_archive_name(
                admitted_names,
                admitted_files,
                normalized,
                is_directory=entry_type == b"5",
            )
            if entry_type == b"5":
                if declared_size != 0:
                    raise ValueError(f"tar directory has a payload: {name}")
                if admitted_directories is not None:
                    admitted_directories.add(normalized)
                continue
            total_bytes = admit_archive_payload(total_bytes, declared_size)
            data = read_tar_payload(source, declared_size, name)
            if len(data) != declared_size:
                raise ValueError(f"archive member size mismatch: {name}")
            padding_size = (-declared_size) % TAR_BLOCK_BYTES
            if padding_size:
                padding = source.read(padding_size)
                if len(padding) != padding_size or any(padding):
                    raise ValueError(f"malformed tar member padding: {name}")
            files[normalized] = data

        trailing = source.read(MAX_TAR_TRAILING_BYTES + 1)
        if len(trailing) > MAX_TAR_TRAILING_BYTES or any(trailing):
            raise ValueError("release tar trailing data is too large or nonzero")
    return files


def copy_archive_snapshot(path: Path, snapshot: BinaryIO) -> str:
    """Copy one ordinary archive once and hash the exact immutable snapshot."""
    try:
        before = path.lstat()
    except OSError as error:
        raise ValueError(f"cannot inspect release archive: {path}") from error
    if not stat.S_ISREG(before.st_mode) or path.is_symlink():
        raise ValueError(f"release archive is not an ordinary file: {path}")
    digest = hashlib.sha256()
    total = 0
    try:
        with path.open("rb") as source:
            opened = os.fstat(source.fileno())
            if (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino):
                raise ValueError("release archive changed before it was opened")
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                total += len(chunk)
                if total > MAX_ARCHIVE_FILE_BYTES:
                    raise ValueError("release archive file is too large")
                digest.update(chunk)
                snapshot.write(chunk)
            after = os.fstat(source.fileno())
    except OSError as error:
        raise ValueError(f"cannot snapshot release archive: {path}") from error
    if (
        (opened.st_dev, opened.st_ino) != (after.st_dev, after.st_ino)
        or opened.st_size != after.st_size
        or total != after.st_size
    ):
        raise ValueError("release archive changed while it was read")
    snapshot.seek(0)
    return digest.hexdigest()


def archive_snapshot_files(
    snapshot: BinaryIO,
    archive_name: str,
    admitted_directories: set[str] | None = None,
) -> dict[str, bytes]:
    files: dict[str, bytes] = {}
    entry_count = 0
    total_bytes = 0
    admitted_names: set[str] = set()
    admitted_files: set[str] = set()
    if archive_name.endswith(".zip"):
        declared_entries = zip_archive_entry_count(snapshot)
        snapshot.seek(0)
        with zipfile.ZipFile(snapshot) as archive:
            infos = archive.infolist()
            if len(infos) != declared_entries:
                raise ValueError("release ZIP entry count mismatch")
            for info in infos:
                if info.orig_filename != info.filename:
                    raise ValueError("release ZIP member name contains a null byte")
                name = safe_member_name(info.orig_filename)
                normalized = name.as_posix()
                entry_count = admit_archive_entry(entry_count)
                mode = (info.external_attr >> 16) & 0o170000
                if info.is_dir():
                    if (
                        mode not in (0, 0o040000)
                        or info.file_size != 0
                        or info.compress_size != 0
                    ):
                        raise ValueError(
                            f"invalid ZIP directory entry: {info.filename}"
                        )
                    admit_archive_name(
                        admitted_names,
                        admitted_files,
                        normalized,
                        is_directory=True,
                    )
                    if admitted_directories is not None:
                        admitted_directories.add(normalized)
                    continue
                if mode not in (0, 0o100000):
                    raise ValueError(f"non-file ZIP member: {info.filename}")
                admit_archive_name(
                    admitted_names,
                    admitted_files,
                    normalized,
                    is_directory=False,
                )
                total_bytes = admit_archive_payload(total_bytes, info.file_size)
                data = archive.read(info)
                if len(data) != info.file_size:
                    raise ValueError(f"archive member size mismatch: {info.filename}")
                files[normalized] = data
    elif archive_name.endswith(".tar.gz"):
        files = tar_files(snapshot, admitted_directories)
    else:
        raise ValueError(f"unsupported archive extension: {archive_name}")
    return files


def archive_files(path: Path) -> dict[str, bytes]:
    with tempfile.SpooledTemporaryFile(max_size=MAX_ARCHIVE_METADATA_BYTES) as snapshot:
        copy_archive_snapshot(path, snapshot)
        return archive_snapshot_files(snapshot, path.name)


def parse_checksum(checksum_path: Path, expected_name: str) -> str:
    text = checksum_path.read_text(encoding="ascii")
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)\n?", text)
    if not match or match.group(2) != expected_name:
        raise ValueError("checksum sidecar is malformed or names another archive")
    return match.group(1)


def parse_soundtrack_content_checksum(checksum_path: Path) -> str:
    text = checksum_path.read_text(encoding="ascii")
    match = re.fullmatch(rf"([0-9a-f]{{64}})  {SOUNDTRACK_CONTENT_LABEL}\n?", text)
    if not match:
        raise ValueError("soundtrack content checksum is malformed")
    return match.group(1)


def parse_manifest(data: bytes) -> dict[str, str]:
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise ValueError("payload manifest is not ASCII") from error
    entries: dict[str, str] = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._/-]+)", line)
        if not match:
            raise ValueError(f"malformed payload manifest line: {line!r}")
        relative = safe_member_name(match.group(2)).as_posix()
        if relative in entries:
            raise ValueError(f"duplicate payload manifest entry: {relative}")
        entries[relative] = match.group(1)
    return entries


def verify_archive(
    path: Path,
    checksum_path: Path,
    expected_version: str | None = None,
    expected_revision: str | None = None,
) -> dict[str, bytes]:
    """Verify one closed archive and return the exact admitted member snapshot."""
    expected_archive_hash = parse_checksum(checksum_path, path.name)
    admitted_directories: set[str] = set()
    with tempfile.SpooledTemporaryFile(max_size=MAX_ARCHIVE_METADATA_BYTES) as snapshot:
        archive_hash = copy_archive_snapshot(path, snapshot)
        if archive_hash != expected_archive_hash:
            raise ValueError(f"archive checksum mismatch: {path.name}")
        files = archive_snapshot_files(snapshot, path.name, admitted_directories)
    roots = {PurePosixPath(name).parts[0] for name in files}
    if len(roots) != 1:
        raise ValueError("release archive must contain exactly one root directory")
    root_name = next(iter(roots))
    manifest_name = f"{root_name}/MANIFEST.sha256"
    if manifest_name not in files:
        raise ValueError("release archive has no payload manifest")
    manifest = parse_manifest(files[manifest_name])
    payload = {
        name.removeprefix(f"{root_name}/"): data
        for name, data in files.items()
        if name != manifest_name
    }
    if set(payload) != set(manifest):
        raise ValueError("release payload and manifest inventory differ")
    for relative, data in payload.items():
        if sha256_bytes(data) != manifest[relative]:
            raise ValueError(f"payload checksum mismatch: {relative}")
    try:
        metadata = parse_release_metadata(payload["RELEASE.json"])
    except KeyError as error:
        raise ValueError("release archive has no metadata") from error
    version = metadata["version"]
    target = metadata["target"]
    kind = metadata["kind"]
    if kind == "binaries":
        suffix = TARGETS.get(target, (None, None))[0]
        if suffix is None:
            raise ValueError("release metadata names an unsupported target")
        archive_format = TARGETS[target][1]
        expected = (
            {f"bin/{name}{suffix}" for name in BINARIES}
            | set(RELEASE_FILES)
            | {"RELEASE.json"}
        )
        if set(payload) != expected:
            raise ValueError("binary payload inventory is not exact")
    elif kind == "soundtrack":
        if target != "all":
            raise ValueError("soundtrack metadata target is not all")
        archive_format = "tar.gz"
        if (
            sum(name.startswith("radio/") and name.endswith(".mp3") for name in payload)
            < 1
        ):
            raise ValueError("soundtrack payload contains no MP3 tracks")
        content_checksum = Path(f"{path}.content.sha256")
        if not content_checksum.is_file() or content_checksum.is_symlink():
            raise ValueError("soundtrack archive has no content checksum")
        expected_content_hash = parse_soundtrack_content_checksum(content_checksum)
        if soundtrack_content_hash(payload) != expected_content_hash:
            raise ValueError("soundtrack content checksum mismatch")
    else:
        raise ValueError("release metadata kind is unsupported")
    expected_root = archive_root(version, target, kind)
    if root_name != expected_root or any(
        PurePosixPath(name).parts[0] != expected_root for name in admitted_directories
    ):
        raise ValueError("release archive members do not match the metadata root")
    if path.name != f"{expected_root}.{archive_format}":
        raise ValueError("release archive name does not match its metadata")
    if expected_version is not None:
        validate_version(expected_version)
        if version != expected_version:
            raise ValueError(
                "release archive version does not match the expected release"
            )
    if expected_revision is not None:
        if re.fullmatch(r"[0-9a-f]{40}", expected_revision) is None:
            raise ValueError("expected release revision is malformed")
        if metadata["commit"] != expected_revision:
            raise ValueError(
                "release archive commit does not match the expected release"
            )
    return files


def checked_slice(data: bytes, offset: int, size: int, label: str) -> bytes:
    """Return one bounded binary region without accepting integer wraparound."""
    if type(offset) is not int or type(size) is not int or offset < 0 or size < 0:
        raise ValueError(f"{label} has invalid bounds")
    end = offset + size
    if end < offset or end > len(data):
        raise ValueError(f"{label} is outside the binary")
    return data[offset:end]


def unpack_at(
    data: bytes, format_string: str, offset: int, label: str
) -> tuple[int, ...]:
    size = struct.calcsize(format_string)
    checked_slice(data, offset, size, label)
    return struct.unpack_from(format_string, data, offset)


def native_name(data: bytes, offset: int, end: int, label: str) -> str:
    if not 0 <= offset < end <= len(data):
        raise ValueError(f"{label} has invalid bounds")
    bounded_end = min(end, offset + MAX_NATIVE_NAME_BYTES + 1)
    terminator = data.find(b"\0", offset, bounded_end)
    if terminator < 0:
        raise ValueError(f"{label} is not bounded by a terminator")
    raw = data[offset:terminator]
    if not raw or len(raw) > MAX_NATIVE_NAME_BYTES:
        raise ValueError(f"{label} has an invalid length")
    if any(byte < 0x21 or byte > 0x7E for byte in raw):
        raise ValueError(f"{label} is not printable ASCII")
    return raw.decode("ascii")


def exact_imports(
    imports: list[str], label: str, *, lowercase: bool = False
) -> list[str]:
    if not imports or len(imports) > MAX_NATIVE_IMPORTS:
        raise ValueError(f"{label} import inventory is missing or unbounded")
    normalized = [item.lower() if lowercase else item for item in imports]
    return sorted(set(normalized))


def elf_file_offset(
    address: int,
    size: int,
    load_segments: list[tuple[int, int, int, int]],
    data: bytes,
) -> int:
    candidates: list[int] = []
    for file_offset, virtual_address, file_size, memory_size in load_segments:
        if (
            virtual_address <= address
            and address + size <= virtual_address + memory_size
        ):
            delta = address - virtual_address
            if delta + size <= file_size:
                candidate = file_offset + delta
                checked_slice(data, candidate, size, "ELF virtual address")
                candidates.append(candidate)
    if len(candidates) != 1:
        raise ValueError("ELF virtual address has no unique file mapping")
    return candidates[0]


def inspect_elf(data: bytes) -> tuple[str, list[str]]:
    if len(data) < 64 or data[:4] != b"\x7fELF":
        raise ValueError("release binary is not ELF")
    if data[4] != 2 or data[5] != 1 or data[6] != 1:
        raise ValueError("release ELF must be 64-bit little-endian version 1")
    file_type, machine, version = unpack_at(data, "<HHI", 16, "ELF identity")
    if file_type not in (2, 3) or version != 1:
        raise ValueError(
            "release ELF is not an executable or position-independent executable"
        )
    if machine != 62:
        raise ValueError("release ELF architecture is not x86_64")
    entry_address = unpack_at(data, "<Q", 24, "ELF entry address")[0]
    program_offset = unpack_at(data, "<Q", 32, "ELF program offset")[0]
    program_entry_size = unpack_at(data, "<H", 54, "ELF program entry size")[0]
    program_count = unpack_at(data, "<H", 56, "ELF program count")[0]
    if program_entry_size != 56 or not 1 <= program_count <= MAX_NATIVE_PROGRAM_HEADERS:
        raise ValueError("ELF program header inventory is invalid")
    checked_slice(
        data,
        program_offset,
        program_entry_size * program_count,
        "ELF program headers",
    )
    load_segments: list[tuple[int, int, int, int]] = []
    dynamic_segments: list[tuple[int, int]] = []
    interpreter_segments: list[tuple[int, int]] = []
    executable_ranges: list[tuple[int, int]] = []
    for index in range(program_count):
        offset = program_offset + index * program_entry_size
        program_type, flags = unpack_at(data, "<II", offset, "ELF program identity")
        file_offset, virtual_address = unpack_at(
            data, "<QQ", offset + 8, "ELF program location"
        )
        file_size, memory_size = unpack_at(data, "<QQ", offset + 32, "ELF program size")
        if file_size > memory_size:
            raise ValueError("ELF program file size exceeds memory size")
        checked_slice(data, file_offset, file_size, "ELF program payload")
        if program_type == 1:
            load_segments.append((file_offset, virtual_address, file_size, memory_size))
            if flags & 1:
                executable_ranges.append(
                    (virtual_address, virtual_address + memory_size)
                )
        elif program_type == 2:
            dynamic_segments.append((file_offset, file_size))
        elif program_type == 3:
            interpreter_segments.append((file_offset, file_size))
    if not load_segments or len(dynamic_segments) != 1:
        raise ValueError("ELF load or dynamic segment inventory is invalid")
    if entry_address == 0 or not any(
        start <= entry_address < end for start, end in executable_ranges
    ):
        raise ValueError("ELF entry point is not in an executable load segment")
    if file_type == 3:
        if len(interpreter_segments) != 1:
            raise ValueError("position-independent ELF has no unique interpreter")
        interpreter_offset, interpreter_size = interpreter_segments[0]
        native_name(
            data,
            interpreter_offset,
            interpreter_offset + interpreter_size,
            "ELF interpreter",
        )

    dynamic_offset, dynamic_size = dynamic_segments[0]
    if dynamic_size == 0 or dynamic_size % 16 != 0:
        raise ValueError("ELF dynamic table size is invalid")
    if dynamic_size // 16 > MAX_NATIVE_IMPORTS + 64:
        raise ValueError("ELF dynamic table is unbounded")
    string_addresses: list[int] = []
    string_sizes: list[int] = []
    needed_offsets: list[int] = []
    terminated = False
    for offset in range(dynamic_offset, dynamic_offset + dynamic_size, 16):
        tag, value = unpack_at(data, "<QQ", offset, "ELF dynamic entry")
        if tag == 0:
            terminated = True
            break
        if tag == 1:
            needed_offsets.append(value)
        elif tag == 5:
            string_addresses.append(value)
        elif tag == 10:
            string_sizes.append(value)
    if (
        not terminated
        or len(string_addresses) != 1
        or len(string_sizes) != 1
        or not needed_offsets
        or len(needed_offsets) > MAX_NATIVE_IMPORTS
    ):
        raise ValueError("ELF dynamic string or import inventory is invalid")
    string_size = string_sizes[0]
    if not 1 <= string_size <= len(data):
        raise ValueError("ELF dynamic string table size is invalid")
    string_offset = elf_file_offset(
        string_addresses[0], string_size, load_segments, data
    )
    imports = []
    for needed in needed_offsets:
        if needed >= string_size:
            raise ValueError("ELF import name offset is outside the string table")
        imports.append(
            native_name(
                data,
                string_offset + needed,
                string_offset + string_size,
                "ELF import name",
            )
        )
    return "x86_64", exact_imports(imports, "ELF")


def inspect_mach(data: bytes) -> tuple[str, list[str]]:
    if len(data) < 32 or data[:4] != b"\xcf\xfa\xed\xfe":
        raise ValueError("release binary is not a little-endian 64-bit Mach-O")
    cpu_type = unpack_at(data, "<I", 4, "Mach-O CPU type")[0]
    architectures = {0x01000007: "x86_64", 0x0100000C: "aarch64"}
    architecture = architectures.get(cpu_type)
    if architecture is None:
        raise ValueError("release Mach-O architecture is unsupported")
    file_type, command_count, command_bytes = unpack_at(
        data, "<III", 12, "Mach-O header"
    )
    if file_type != 2:
        raise ValueError("release Mach-O is not an executable")
    if not 1 <= command_count <= MAX_MACH_LOAD_COMMANDS:
        raise ValueError("Mach-O load command count is invalid")
    command_end = 32 + command_bytes
    checked_slice(data, 32, command_bytes, "Mach-O load commands")
    imports: list[str] = []
    dylib_commands = {0xC, 0x20, 0x80000018, 0x8000001F, 0x80000023}
    offset = 32
    for _index in range(command_count):
        command, command_size = unpack_at(data, "<II", offset, "Mach-O load command")
        if (
            command_size < 8
            or command_size % 8 != 0
            or offset + command_size > command_end
        ):
            raise ValueError("Mach-O load command size is invalid")
        if command in dylib_commands:
            if command_size < 24:
                raise ValueError("Mach-O dylib command is truncated")
            name_offset = unpack_at(data, "<I", offset + 8, "Mach-O dylib name offset")[
                0
            ]
            if not 24 <= name_offset < command_size:
                raise ValueError("Mach-O dylib name offset is invalid")
            imports.append(
                native_name(
                    data,
                    offset + name_offset,
                    offset + command_size,
                    "Mach-O dylib name",
                )
            )
        offset += command_size
    if offset != command_end:
        raise ValueError("Mach-O load command bytes are inconsistent")
    return architecture, exact_imports(imports, "Mach-O")


def pe_rva_region(
    rva: int,
    size: int,
    size_of_headers: int,
    sections: list[tuple[int, int, int, int]],
    data: bytes,
) -> tuple[int, int]:
    candidates: list[tuple[int, int]] = []
    if rva < size_of_headers and rva + size <= size_of_headers:
        checked_slice(data, rva, size, "PE header RVA")
        candidates.append((rva, min(size_of_headers, len(data))))
    for virtual_address, virtual_size, raw_offset, raw_size in sections:
        span = max(virtual_size, raw_size)
        if virtual_address <= rva and rva + size <= virtual_address + span:
            delta = rva - virtual_address
            if delta + size <= raw_size:
                candidate = raw_offset + delta
                checked_slice(data, candidate, size, "PE section RVA")
                candidates.append((candidate, raw_offset + raw_size))
    if len(candidates) != 1:
        raise ValueError("PE RVA has no unique file mapping")
    return candidates[0]


def pe_rva_offset(
    rva: int,
    size: int,
    size_of_headers: int,
    sections: list[tuple[int, int, int, int]],
    data: bytes,
) -> int:
    return pe_rva_region(rva, size, size_of_headers, sections, data)[0]


def pe_import_name(
    rva: int,
    size_of_headers: int,
    sections: list[tuple[int, int, int, int]],
    data: bytes,
    label: str,
) -> str:
    offset, end = pe_rva_region(rva, 1, size_of_headers, sections, data)
    return native_name(data, offset, end, label)


def inspect_pe(data: bytes) -> tuple[str, list[str]]:
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError("release binary is not PE")
    pe_offset = unpack_at(data, "<I", 0x3C, "PE header offset")[0]
    if checked_slice(data, pe_offset, 4, "PE signature") != b"PE\0\0":
        raise ValueError("release PE signature is invalid")
    coff_offset = pe_offset + 4
    machine, section_count = unpack_at(data, "<HH", coff_offset, "PE COFF header")
    characteristics = unpack_at(data, "<H", coff_offset + 18, "PE characteristics")[0]
    if machine != 0x8664:
        raise ValueError("release PE architecture is not x86_64")
    if characteristics & 0x0002 == 0 or characteristics & 0x2000:
        raise ValueError("release PE is not an executable image")
    if not 1 <= section_count <= MAX_NATIVE_SECTIONS:
        raise ValueError("PE section inventory is invalid")
    optional_size = unpack_at(data, "<H", coff_offset + 16, "PE optional size")[0]
    optional_offset = coff_offset + 20
    checked_slice(data, optional_offset, optional_size, "PE optional header")
    if (
        optional_size < 112
        or unpack_at(data, "<H", optional_offset, "PE magic")[0] != 0x20B
    ):
        raise ValueError("release PE must use a PE32+ optional header")
    image_base = unpack_at(data, "<Q", optional_offset + 24, "PE image base")[0]
    size_of_headers = unpack_at(data, "<I", optional_offset + 60, "PE header size")[0]
    directory_count = unpack_at(
        data, "<I", optional_offset + 108, "PE directory count"
    )[0]
    if directory_count > 16 or optional_size < 112 + directory_count * 8:
        raise ValueError("PE data directory inventory is invalid")
    section_offset = optional_offset + optional_size
    checked_slice(data, section_offset, section_count * 40, "PE section headers")
    sections: list[tuple[int, int, int, int]] = []
    for index in range(section_count):
        offset = section_offset + index * 40
        virtual_size, virtual_address, raw_size, raw_offset = unpack_at(
            data, "<IIII", offset + 8, "PE section location"
        )
        checked_slice(data, raw_offset, raw_size, "PE section payload")
        sections.append((virtual_address, virtual_size, raw_offset, raw_size))

    def directory(index: int) -> tuple[int, int]:
        if index >= directory_count:
            return 0, 0
        return unpack_at(
            data,
            "<II",
            optional_offset + 112 + index * 8,
            "PE data directory",
        )

    imports: list[str] = []
    import_rva, import_size = directory(1)
    if import_rva == 0 or import_size < 20 or import_size % 20 != 0:
        raise ValueError("PE import directory is missing or malformed")
    if import_size // 20 > MAX_NATIVE_IMPORTS + 1:
        raise ValueError("PE import directory is unbounded")
    import_offset = pe_rva_offset(
        import_rva, import_size, size_of_headers, sections, data
    )
    terminated = False
    for offset in range(import_offset, import_offset + import_size, 20):
        fields = unpack_at(data, "<IIIII", offset, "PE import descriptor")
        if fields == (0, 0, 0, 0, 0):
            terminated = True
            break
        if fields[3] == 0 or fields[4] == 0:
            raise ValueError("PE import descriptor is missing a name or thunk")
        imports.append(
            pe_import_name(fields[3], size_of_headers, sections, data, "PE import name")
        )
    if not terminated:
        raise ValueError("PE import directory has no terminator")

    delay_rva, delay_size = directory(13)
    if delay_rva or delay_size:
        if delay_rva == 0 or delay_size < 32 or delay_size % 32 != 0:
            raise ValueError("PE delay import directory is malformed")
        if delay_size // 32 > MAX_NATIVE_IMPORTS + 1:
            raise ValueError("PE delay import directory is unbounded")
        delay_offset = pe_rva_offset(
            delay_rva, delay_size, size_of_headers, sections, data
        )
        terminated = False
        for offset in range(delay_offset, delay_offset + delay_size, 32):
            fields = unpack_at(data, "<IIIIIIII", offset, "PE delay descriptor")
            if fields == (0, 0, 0, 0, 0, 0, 0, 0):
                terminated = True
                break
            attributes, name_value = fields[:2]
            if attributes & ~1:
                raise ValueError("PE delay import attributes are unsupported")
            if name_value == 0:
                raise ValueError("PE delay import descriptor has no name")
            if attributes & 1:
                name_rva = name_value
            else:
                if name_value < image_base:
                    raise ValueError("PE delay import address precedes the image base")
                name_rva = name_value - image_base
            imports.append(
                pe_import_name(
                    name_rva,
                    size_of_headers,
                    sections,
                    data,
                    "PE delay import name",
                )
            )
        if not terminated:
            raise ValueError("PE delay import directory has no terminator")
    return "x86_64", exact_imports(imports, "PE", lowercase=True)


def inspect_native_binary(data: bytes, target: str) -> dict[str, object]:
    expected = NATIVE_TARGETS.get(target)
    if expected is None:
        raise ValueError(f"unsupported native inventory target: {target}")
    expected_format, expected_architecture = expected
    if expected_format == "ELF":
        architecture, imports = inspect_elf(data)
    elif expected_format == "Mach-O":
        architecture, imports = inspect_mach(data)
    else:
        architecture, imports = inspect_pe(data)
    if architecture != expected_architecture:
        raise ValueError(
            f"release binary architecture {architecture} does not match {target}"
        )
    return {
        "architecture": architecture,
        "format": expected_format,
        "imports": imports,
        "sha1": hashlib.sha1(data, usedforsecurity=False).hexdigest(),
        "sha256": sha256_bytes(data),
    }


def native_archive_inventory(
    path: Path, checksum_path: Path
) -> list[dict[str, object]]:
    """Inspect all executables in one exact verified binary release archive."""
    files = verify_archive(path, checksum_path)
    roots = {PurePosixPath(name).parts[0] for name in files}
    if len(roots) != 1:
        raise ValueError("verified native archive has no unique root")
    root_name = next(iter(roots))
    metadata_name = f"{root_name}/RELEASE.json"
    try:
        metadata = parse_release_metadata(files[metadata_name])
    except (KeyError, UnicodeError, ValueError) as error:
        raise ValueError("verified native archive metadata is malformed") from error
    target = metadata.get("target")
    version = metadata.get("version")
    revision = metadata.get("commit")
    if metadata.get("kind") != "binaries" or target not in NATIVE_TARGETS:
        raise ValueError("verified archive is not a supported binary release")
    validate_version(version)
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ValueError("verified native archive commit is malformed")
    suffix = TARGETS[target][0]
    inventory: list[dict[str, object]] = []
    for binary in BINARIES:
        relative = f"bin/{binary}{suffix}"
        name = f"{root_name}/{relative}"
        try:
            inspected = inspect_native_binary(files[name], target)
        except KeyError as error:
            raise ValueError(f"verified archive is missing {relative}") from error
        inventory.append(
            {
                **inspected,
                "fileName": relative,
                "sourceRevision": revision,
                "target": target,
                "version": version,
            }
        )
    return inventory


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print-version", action="store_true")
    parser.add_argument("--validate-tag")
    parser.add_argument("--verify-archive", type=Path)
    parser.add_argument("--checksum", type=Path)
    parser.add_argument("--expected-version")
    parser.add_argument("--expected-revision")
    parser.add_argument(
        "--kind", choices=("binaries", "soundtrack"), default="binaries"
    )
    parser.add_argument("--version")
    parser.add_argument("--target")
    parser.add_argument("--binary-dir", type=Path)
    parser.add_argument("--radio-dir", type=Path, default=ROOT / "assets" / "radio")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    version = workspace_version()
    if args.print_version:
        print(version)
        return 0
    if args.validate_tag is not None:
        if args.validate_tag != f"v{version}":
            raise ValueError(
                f"release tag {args.validate_tag!r} does not match workspace v{version}"
            )
        print(f"release tag matches workspace version {version}")
        return 0
    if args.verify_archive is not None:
        if args.checksum is None:
            raise ValueError("--verify-archive requires --checksum")
        verify_archive(
            args.verify_archive,
            args.checksum,
            args.expected_version,
            args.expected_revision,
        )
        print(f"verified {args.verify_archive.name}")
        return 0
    requested_version = args.version or version
    target = args.target or ("all" if args.kind == "soundtrack" else None)
    if target is None:
        raise ValueError("binary packaging requires --target")
    binary_dir = args.binary_dir
    if args.kind == "binaries" and binary_dir is None:
        binary_dir = ROOT / "target" / target / "release"
    archive_path, checksum_path = build_archive(
        requested_version,
        target,
        args.kind,
        binary_dir,
        args.radio_dir,
        args.output_dir,
    )
    print(archive_path)
    print(checksum_path)
    content_checksum = Path(f"{archive_path}.content.sha256")
    if content_checksum.exists():
        print(content_checksum)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
