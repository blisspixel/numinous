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
import subprocess
import tarfile
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
    "scripts/install.ps1",
    "scripts/install.sh",
)


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
    if kind == "binaries":
        if target not in TARGETS or binary_dir is None:
            raise ValueError("binary packaging requires a supported target and binary directory")
        payload = release_payload(version, target, binary_dir, root)
        archive_format = TARGETS[target][1]
    else:
        if target != "all":
            raise ValueError("soundtrack packaging uses target 'all'")
        payload = soundtrack_payload(version, radio_dir, root)
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
    verify_archive(archive_path, checksum_path)
    return archive_path, checksum_path


def safe_member_name(name: str) -> PurePosixPath:
    if "\\" in name or "//" in name:
        raise ValueError(f"unsafe archive member: {name!r}")
    path = PurePosixPath(name)
    if path.is_absolute() or not path.parts or any(part in ("", ".", "..") for part in path.parts):
        raise ValueError(f"unsafe archive member: {name!r}")
    return path


def archive_files(path: Path) -> dict[str, bytes]:
    files: dict[str, bytes] = {}
    if path.name.endswith(".zip"):
        with zipfile.ZipFile(path) as archive:
            for info in archive.infolist():
                name = safe_member_name(info.filename)
                if info.is_dir():
                    continue
                mode = (info.external_attr >> 16) & 0o170000
                if mode not in (0, 0o100000):
                    raise ValueError(f"non-file ZIP member: {info.filename}")
                normalized = name.as_posix()
                if normalized in files:
                    raise ValueError(f"duplicate archive member: {normalized}")
                files[normalized] = archive.read(info)
    elif path.name.endswith(".tar.gz"):
        with tarfile.open(path, "r:gz") as archive:
            for info in archive.getmembers():
                name = safe_member_name(info.name)
                if info.isdir():
                    continue
                if not info.isfile():
                    raise ValueError(f"non-file tar member: {info.name}")
                source = archive.extractfile(info)
                if source is None:
                    raise ValueError(f"unreadable tar member: {info.name}")
                normalized = name.as_posix()
                if normalized in files:
                    raise ValueError(f"duplicate archive member: {normalized}")
                files[normalized] = source.read()
    else:
        raise ValueError(f"unsupported archive extension: {path.name}")
    return files


def parse_checksum(checksum_path: Path, expected_name: str) -> str:
    text = checksum_path.read_text(encoding="ascii")
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)\n?", text)
    if not match or match.group(2) != expected_name:
        raise ValueError("checksum sidecar is malformed or names another archive")
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


def verify_archive(path: Path, checksum_path: Path) -> None:
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
    else:
        raise ValueError("release metadata kind is unsupported")


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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
