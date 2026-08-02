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
import struct
import subprocess
import tarfile
from typing import BinaryIO
import zipfile


ROOT = Path(__file__).resolve().parent.parent
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
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
MAX_TAR_TRAILING_BYTES = 16 * 1024
ZIP_END_RECORD_SIZE = 22
ZIP_MAX_COMMENT_BYTES = 65_535
ZIP_CENTRAL_HEADER_SIZE = 46
TAR_BLOCK_BYTES = 512


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
    if not VERSION_RE.fullmatch(version):
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
    if not any(
        name.startswith("radio/") and name.endswith(".mp3") for name in payload
    ):
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
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
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
            raise ValueError("binary packaging requires a supported target and binary directory")
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
    if "\\" in name or "//" in name:
        raise ValueError(f"unsafe archive member: {name!r}")
    path = PurePosixPath(name)
    if path.is_absolute() or not path.parts or any(part in ("", ".", "..") for part in path.parts):
        raise ValueError(f"unsafe archive member: {name!r}")
    return path


def admit_archive_entry(entry_count: int) -> int:
    """Admit one archive entry within the metadata work budget."""
    if entry_count >= MAX_ARCHIVE_ENTRIES:
        raise ValueError("release archive contains too many entries")
    return entry_count + 1


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


def zip_archive_entry_count(path: Path) -> int:
    """Validate and count classic ZIP metadata before ZipFile constructs it."""
    size = path.stat().st_size
    tail_size = min(size, ZIP_END_RECORD_SIZE + ZIP_MAX_COMMENT_BYTES)
    with path.open("rb") as source:
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
        with path.open("rb") as source:
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
    with path.open("rb") as source:
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
        with path.open("rb") as source:
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
            local_name_size, local_extra_size = struct.unpack_from(
                "<HH", local_header, 26
            )
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
            position + ZIP_CENTRAL_HEADER_SIZE :
            position + ZIP_CENTRAL_HEADER_SIZE + name_size
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


def tar_files(path: Path) -> dict[str, bytes]:
    """Read the canonical ustar subset without hidden extension processing."""
    files: dict[str, bytes] = {}
    entry_count = 0
    total_bytes = 0
    zero_blocks = 0
    with gzip.open(path, "rb") as source:
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
            raw_name = header[:100].split(b"\0", 1)[0]
            raw_prefix = header[345:500].split(b"\0", 1)[0]
            try:
                name = raw_name.decode("utf-8")
                prefix = raw_prefix.decode("utf-8")
            except UnicodeDecodeError as error:
                raise ValueError("release tar member name is not UTF-8") from error
            if prefix:
                name = f"{prefix}/{name}"
            normalized = safe_member_name(name).as_posix()
            entry_count = admit_archive_entry(entry_count)
            entry_type = header[156:157]
            if entry_type == b"5":
                if declared_size != 0:
                    raise ValueError(f"tar directory has a payload: {name}")
                continue
            if entry_type not in (b"\0", b"0"):
                raise ValueError(f"unsupported tar entry type: {entry_type!r}")
            if normalized in files:
                raise ValueError(f"duplicate archive member: {normalized}")
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


def archive_files(path: Path) -> dict[str, bytes]:
    files: dict[str, bytes] = {}
    entry_count = 0
    total_bytes = 0
    if path.name.endswith(".zip"):
        declared_entries = zip_archive_entry_count(path)
        with zipfile.ZipFile(path) as archive:
            infos = archive.infolist()
            if len(infos) != declared_entries:
                raise ValueError("release ZIP entry count mismatch")
            for info in infos:
                name = safe_member_name(info.filename)
                entry_count = admit_archive_entry(entry_count)
                if info.is_dir():
                    continue
                mode = (info.external_attr >> 16) & 0o170000
                if mode not in (0, 0o100000):
                    raise ValueError(f"non-file ZIP member: {info.filename}")
                normalized = name.as_posix()
                if normalized in files:
                    raise ValueError(f"duplicate archive member: {normalized}")
                total_bytes = admit_archive_payload(total_bytes, info.file_size)
                data = archive.read(info)
                if len(data) != info.file_size:
                    raise ValueError(f"archive member size mismatch: {info.filename}")
                files[normalized] = data
    elif path.name.endswith(".tar.gz"):
        files = tar_files(path)
    else:
        raise ValueError(f"unsupported archive extension: {path.name}")
    return files


def parse_checksum(checksum_path: Path, expected_name: str) -> str:
    text = checksum_path.read_text(encoding="ascii")
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)\n?", text)
    if not match or match.group(2) != expected_name:
        raise ValueError("checksum sidecar is malformed or names another archive")
    return match.group(1)


def parse_soundtrack_content_checksum(checksum_path: Path) -> str:
    text = checksum_path.read_text(encoding="ascii")
    match = re.fullmatch(
        rf"([0-9a-f]{{64}})  {SOUNDTRACK_CONTENT_LABEL}\n?", text
    )
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


def verify_archive(path: Path, checksum_path: Path) -> dict[str, bytes]:
    """Verify one closed archive and return the exact admitted member snapshot."""
    expected_archive_hash = parse_checksum(checksum_path, path.name)
    if sha256_file(path) != expected_archive_hash:
        raise ValueError(f"archive checksum mismatch: {path.name}")
    files = archive_files(path)
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
    metadata = json.loads(payload["RELEASE.json"])
    if metadata.get("schema") != "numinous.release" or metadata.get("schemaVersion") != 1:
        raise ValueError("release metadata schema is unsupported")
    if metadata.get("kind") == "binaries":
        suffix = TARGETS.get(metadata.get("target"), (None, None))[0]
        if suffix is None:
            raise ValueError("release metadata names an unsupported target")
        expected = {f"bin/{name}{suffix}" for name in BINARIES}
        if not expected.issubset(payload):
            raise ValueError("binary payload is incomplete")
    elif metadata.get("kind") == "soundtrack":
        if sum(name.startswith("radio/") and name.endswith(".mp3") for name in payload) < 1:
            raise ValueError("soundtrack payload contains no MP3 tracks")
        content_checksum = Path(f"{path}.content.sha256")
        if not content_checksum.is_file() or content_checksum.is_symlink():
            raise ValueError("soundtrack archive has no content checksum")
        expected_content_hash = parse_soundtrack_content_checksum(content_checksum)
        if soundtrack_content_hash(payload) != expected_content_hash:
            raise ValueError("soundtrack content checksum mismatch")
    else:
        raise ValueError("release metadata kind is unsupported")
    if sha256_file(path) != expected_archive_hash:
        raise ValueError("release archive changed during verification")
    return files


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print-version", action="store_true")
    parser.add_argument("--validate-tag")
    parser.add_argument("--verify-archive", type=Path)
    parser.add_argument("--checksum", type=Path)
    parser.add_argument("--kind", choices=("binaries", "soundtrack"), default="binaries")
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
        verify_archive(args.verify_archive, args.checksum)
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
