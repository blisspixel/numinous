#!/usr/bin/env python3
"""Generate and verify the deterministic SPDX release dependency inventory."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tomllib
from typing import Any, NoReturn
from urllib.parse import quote


ROOT = Path(__file__).resolve().parent.parent
MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_PACKAGES = 10_000
METADATA_TIMEOUT_SECONDS = 90
SHA256_HEX = re.compile(r"[0-9a-f]{64}")
COMMIT_SHA = re.compile(r"[0-9a-f]{40}")
RELEASE_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?")
PACKAGE_NAME = re.compile(r"[0-9A-Za-z_-]+")
LICENSE_TOKEN = re.compile(r"[0-9A-Za-z.+-]+(?::[0-9A-Za-z.+-]+)?")
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
SUPPORTED_LICENSE_IDS = frozenset(
    {
        "0BSD",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "CC0-1.0",
        "CDLA-Permissive-2.0",
        "ISC",
        "LGPL-2.1-or-later",
        "MIT",
        "MPL-2.0",
        "Unicode-3.0",
        "Unlicense",
        "Zlib",
    }
)
SUPPORTED_LICENSE_EXCEPTIONS = frozenset({"LLVM-exception"})


class SbomError(ValueError):
    """Raised when release inventory evidence is incomplete or inconsistent."""


def fail(message: str) -> NoReturn:
    raise SbomError(message)


def read_regular_file(path: Path, maximum: int = MAX_INPUT_BYTES) -> bytes:
    """Read one bounded ordinary file without accepting links or special files."""
    try:
        before = path.lstat()
    except OSError as error:
        fail(f"cannot inspect {path}: {error}")
    if not stat.S_ISREG(before.st_mode):
        fail(f"{path} is not an ordinary file")
    if before.st_size > maximum:
        fail(f"{path} exceeds the {maximum}-byte limit")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        fail(f"cannot open {path}: {error}")
    with os.fdopen(descriptor, "rb") as handle:
        opened = os.fstat(handle.fileno())
        if not stat.S_ISREG(opened.st_mode) or (
            before.st_dev,
            before.st_ino,
        ) != (opened.st_dev, opened.st_ino):
            fail(f"{path} changed before it was opened")
        data = handle.read(maximum + 1)
        after = os.fstat(handle.fileno())
    if (
        len(data) > maximum
        or len(data) != before.st_size
        or opened.st_size != before.st_size
        or after.st_size != opened.st_size
        or after.st_mtime_ns != opened.st_mtime_ns
    ):
        fail(f"{path} changed or exceeded its limit while it was read")
    return data


def parse_json_object(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{label} is not valid UTF-8 JSON: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain one JSON object")
    return value


def load_cargo_metadata(root: Path = ROOT) -> dict[str, Any]:
    """Load the exact locked all-feature Cargo graph without running build code."""
    try:
        result = subprocess.run(
            [
                "cargo",
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--all-features",
            ],
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=METADATA_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"cargo metadata failed: {error}")
    if result.returncode != 0:
        detail = result.stderr[:4096].decode("utf-8", errors="replace").strip()
        fail(f"cargo metadata exited {result.returncode}: {detail}")
    if len(result.stdout) > MAX_INPUT_BYTES:
        fail(f"cargo metadata exceeds the {MAX_INPUT_BYTES}-byte limit")
    return parse_json_object(result.stdout, "cargo metadata")


def load_lockfile(data: bytes) -> dict[tuple[str, str, str], str | None]:
    """Return the exact package identities and registry checksums in Cargo.lock."""
    try:
        document = tomllib.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        fail(f"Cargo.lock is invalid UTF-8 TOML: {error}")
    packages = document.get("package")
    if not isinstance(packages, list) or not packages or len(packages) > MAX_PACKAGES:
        fail("Cargo.lock package inventory is missing or unbounded")
    locked: dict[tuple[str, str, str], str | None] = {}
    for package in packages:
        if not isinstance(package, dict):
            fail("Cargo.lock contains a non-table package")
        name = package.get("name")
        version = package.get("version")
        source = package.get("source", "")
        checksum = package.get("checksum")
        if (
            not isinstance(name, str)
            or PACKAGE_NAME.fullmatch(name) is None
            or not isinstance(version, str)
            or not version
            or len(version) > 128
            or not isinstance(source, str)
            or len(source) > 4096
        ):
            fail("Cargo.lock contains an invalid package identity")
        if checksum is not None and (
            not isinstance(checksum, str) or SHA256_HEX.fullmatch(checksum) is None
        ):
            fail(f"Cargo.lock has an invalid checksum for {name} {version}")
        if source.startswith("registry+") and checksum is None:
            fail(f"Cargo.lock has no registry checksum for {name} {version}")
        key = (name, version, source)
        if key in locked:
            fail(f"Cargo.lock repeats {name} {version} from {source or 'workspace'}")
        locked[key] = checksum
    return locked


def require_string(value: Any, label: str, maximum: int = 4096) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum:
        fail(f"{label} must be a nonempty bounded string")
    return value


def package_spdx_id(name: str, version: str, source: str) -> str:
    stable_identity = json.dumps(
        [name, version, source], ensure_ascii=True, separators=(",", ":")
    )
    digest = hashlib.sha256(stable_identity.encode("utf-8")).hexdigest()
    return f"SPDXRef-CargoPackage-{digest}"


def package_download(name: str, version: str, source: str) -> str:
    if source == CRATES_IO_SOURCE:
        return (
            "https://crates.io/api/v1/crates/"
            f"{quote(name, safe='')}/{quote(version, safe='')}/download"
        )
    if source.startswith("git+"):
        return source.removeprefix("git+")
    return "NOASSERTION"


def cargo_purl(name: str, version: str) -> str:
    return f"pkg:cargo/{quote(name, safe='')}@{quote(version, safe='')}"


def normalize_license_expression(value: Any, cargo_id: str) -> str:
    """Normalize Cargo's legacy slash-as-OR form and validate SPDX grammar."""
    if value is None:
        return "NOASSERTION"
    if not isinstance(value, str) or not 0 < len(value) <= 4096:
        fail(f"license declaration for {cargo_id} is invalid")
    expression = re.sub(r"\s*/\s*", " OR ", value)
    tokens = re.findall(r"\(|\)|[0-9A-Za-z.+-]+(?::[0-9A-Za-z.+-]+)?", expression)
    if not tokens or "".join(tokens) != re.sub(r"\s+", "", expression):
        fail(f"license declaration for {cargo_id} is not an SPDX expression")

    position = 0

    def peek() -> str | None:
        return tokens[position] if position < len(tokens) else None

    def take(expected: str | None = None) -> str:
        nonlocal position
        token = peek()
        if token is None or (expected is not None and token != expected):
            fail(f"license declaration for {cargo_id} is not an SPDX expression")
        position += 1
        return token

    def primary() -> bool:
        token = peek()
        if token == "(":
            take("(")
            disjunction()
            take(")")
            return False
        if (
            token in (None, ")", "AND", "OR", "WITH")
            or LICENSE_TOKEN.fullmatch(token) is None
            or token not in SUPPORTED_LICENSE_IDS
        ):
            fail(f"license declaration for {cargo_id} is not an SPDX expression")
        take()
        return True

    def with_exception() -> None:
        is_simple_license = primary()
        if peek() == "WITH":
            if not is_simple_license:
                fail(f"license declaration for {cargo_id} is not an SPDX expression")
            take("WITH")
            exception = take()
            if (
                exception in ("(", ")", "AND", "OR", "WITH")
                or LICENSE_TOKEN.fullmatch(exception) is None
                or exception not in SUPPORTED_LICENSE_EXCEPTIONS
            ):
                fail(f"license declaration for {cargo_id} is not an SPDX expression")

    def conjunction() -> None:
        with_exception()
        while peek() == "AND":
            take("AND")
            with_exception()

    def disjunction() -> None:
        conjunction()
        while peek() == "OR":
            take("OR")
            conjunction()

    disjunction()
    if position != len(tokens):
        fail(f"license declaration for {cargo_id} is not an SPDX expression")
    return " ".join(tokens).replace("( ", "(").replace(" )", ")")


def build_sbom(
    metadata: dict[str, Any],
    lock_data: bytes,
    release_version: str,
    source_revision: str,
    source_date_epoch: int,
) -> dict[str, Any]:
    """Build one canonical SPDX 2.3 document from locked Cargo evidence."""
    if RELEASE_VERSION.fullmatch(release_version) is None:
        fail("release version is invalid")
    if COMMIT_SHA.fullmatch(source_revision) is None:
        fail("source revision must be one lowercase 40-character commit SHA")
    if type(source_date_epoch) is not int or not 0 <= source_date_epoch <= 253402300799:
        fail("source date epoch is outside the supported UTC range")
    try:
        created = datetime.fromtimestamp(source_date_epoch, timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )
    except (OverflowError, OSError, ValueError) as error:
        fail(f"source date epoch cannot be represented: {error}")

    raw_packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    workspace_members = metadata.get("workspace_members")
    if (
        not isinstance(raw_packages, list)
        or not raw_packages
        or len(raw_packages) > MAX_PACKAGES
        or not isinstance(resolve, dict)
        or not isinstance(resolve.get("nodes"), list)
        or not isinstance(workspace_members, list)
        or not workspace_members
    ):
        fail("cargo metadata inventory is missing or unbounded")

    locked = load_lockfile(lock_data)
    packages_by_id: dict[str, dict[str, Any]] = {}
    lock_keys: set[tuple[str, str, str]] = set()
    for raw in raw_packages:
        if not isinstance(raw, dict):
            fail("cargo metadata contains a non-object package")
        cargo_id = require_string(raw.get("id"), "Cargo package id")
        name = require_string(raw.get("name"), f"name for {cargo_id}", 256)
        version = require_string(raw.get("version"), f"version for {cargo_id}", 128)
        source_value = raw.get("source")
        source = (
            ""
            if source_value is None
            else require_string(source_value, f"source for {cargo_id}")
        )
        if PACKAGE_NAME.fullmatch(name) is None:
            fail(f"Cargo package name is invalid: {name}")
        if cargo_id in packages_by_id:
            fail(f"cargo metadata repeats package id {cargo_id}")
        key = (name, version, source)
        if key not in locked:
            fail(f"Cargo.lock has no exact entry for {name} {version}")
        if key in lock_keys:
            fail(f"cargo metadata repeats locked identity {name} {version}")
        lock_keys.add(key)
        checksum = locked[key]
        license_declared = normalize_license_expression(raw.get("license"), cargo_id)
        spdx_package: dict[str, Any] = {
            "SPDXID": package_spdx_id(name, version, source),
            "copyrightText": "NOASSERTION",
            "downloadLocation": package_download(name, version, source),
            "externalRefs": [
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceLocator": cargo_purl(name, version),
                    "referenceType": "purl",
                }
            ],
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": license_declared,
            "name": name,
            "versionInfo": version,
        }
        if checksum is not None:
            spdx_package["checksums"] = [
                {"algorithm": "SHA256", "checksumValue": checksum}
            ]
        packages_by_id[cargo_id] = spdx_package

    if lock_keys != set(locked):
        fail("Cargo.lock and the resolved all-feature package graph differ")
    package_ids = set(packages_by_id)
    workspace_ids: set[str] = set()
    for raw_member in workspace_members:
        member = require_string(raw_member, "Cargo workspace member id")
        if member in workspace_ids:
            fail(f"cargo metadata repeats workspace member {member}")
        workspace_ids.add(member)
    if workspace_ids - package_ids:
        fail("cargo metadata names an unknown workspace package")
    for cargo_id in workspace_ids:
        if packages_by_id[cargo_id]["versionInfo"] != release_version:
            fail("workspace package versions do not match the release version")

    raw_nodes = resolve["nodes"]
    if len(raw_nodes) != len(packages_by_id):
        fail("cargo metadata resolve graph does not cover every package")
    dependencies: dict[str, list[str]] = {}
    for raw_node in raw_nodes:
        if not isinstance(raw_node, dict):
            fail("cargo metadata contains a non-object resolve node")
        cargo_id = require_string(raw_node.get("id"), "Cargo resolve node id")
        raw_dependencies = raw_node.get("dependencies")
        if cargo_id not in package_ids or cargo_id in dependencies:
            fail("cargo metadata has an unknown or repeated resolve node")
        if not isinstance(raw_dependencies, list):
            fail(f"Cargo resolve node {cargo_id} has no dependency list")
        node_dependencies: list[str] = []
        for dependency in raw_dependencies:
            dependency_id = require_string(dependency, f"dependency of {cargo_id}")
            if dependency_id not in package_ids:
                fail(f"Cargo resolve node {cargo_id} names an unknown dependency")
            node_dependencies.append(dependency_id)
        if len(node_dependencies) != len(set(node_dependencies)):
            fail(f"Cargo resolve node {cargo_id} repeats a dependency")
        dependencies[cargo_id] = sorted(node_dependencies)
    if set(dependencies) != package_ids:
        fail("cargo metadata resolve nodes do not match the package inventory")

    release_id = "SPDXRef-Package-numinous-release"
    release_package = {
        "SPDXID": release_id,
        "comment": (
            "This source-derived inventory covers the locked all-feature Rust "
            "workspace graph. It does not inspect platform-native libraries, "
            "emitted binaries, or bundled soundtrack files."
        ),
        "copyrightText": "NOASSERTION",
        "downloadLocation": "NOASSERTION",
        "externalRefs": [
            {
                "referenceCategory": "OTHER",
                "referenceLocator": (
                    f"git+https://github.com/blisspixel/numinous.git@{source_revision}"
                ),
                "referenceType": "vcs",
            }
        ],
        "filesAnalyzed": False,
        "licenseConcluded": "NOASSERTION",
        "licenseDeclared": "NOASSERTION",
        "name": "numinous-release",
        "versionInfo": release_version,
    }
    relationships = [
        {
            "relatedSpdxElement": release_id,
            "relationshipType": "DESCRIBES",
            "spdxElementId": "SPDXRef-DOCUMENT",
        }
    ]
    for cargo_id in sorted(workspace_ids):
        relationships.append(
            {
                "relatedSpdxElement": packages_by_id[cargo_id]["SPDXID"],
                "relationshipType": "CONTAINS",
                "spdxElementId": release_id,
            }
        )
    for cargo_id in sorted(dependencies):
        for dependency_id in dependencies[cargo_id]:
            relationships.append(
                {
                    "relatedSpdxElement": packages_by_id[dependency_id]["SPDXID"],
                    "relationshipType": "DEPENDS_ON",
                    "spdxElementId": packages_by_id[cargo_id]["SPDXID"],
                }
            )
    relationships.sort(
        key=lambda item: (
            item["spdxElementId"],
            item["relationshipType"],
            item["relatedSpdxElement"],
        )
    )
    lock_sha256 = hashlib.sha256(lock_data).hexdigest()
    return {
        "SPDXID": "SPDXRef-DOCUMENT",
        "comment": (
            f"Cargo.lock SHA256: {lock_sha256}. Source revision: {source_revision}."
        ),
        "creationInfo": {
            "created": created,
            "creators": ["Organization: Numinous Project"],
        },
        "dataLicense": "CC0-1.0",
        "documentDescribes": [release_id],
        "documentNamespace": (
            "https://github.com/blisspixel/numinous/sbom/"
            f"{quote(release_version, safe='')}/{source_revision}/{lock_sha256}"
        ),
        "name": f"numinous-v{release_version}-release-sbom",
        "packages": [release_package]
        + sorted(
            packages_by_id.values(),
            key=lambda item: (item["name"], item["versionInfo"], item["SPDXID"]),
        ),
        "relationships": relationships,
        "spdxVersion": "SPDX-2.3",
    }


def render_sbom(document: dict[str, Any]) -> bytes:
    return (
        json.dumps(document, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("utf-8")


def write_exclusive(path: Path, data: bytes) -> None:
    """Create the final evidence file once, without following an existing name."""
    try:
        with path.open("xb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
    except FileExistsError:
        fail(f"refusing to replace existing SBOM {path}")
    except OSError as error:
        fail(f"cannot create SBOM {path}: {error}")


def verify_file(path: Path, expected: bytes) -> None:
    actual = read_regular_file(path)
    if actual != expected:
        fail(f"SBOM does not match the locked source graph: {path}")


def build_expected(args: argparse.Namespace) -> bytes:
    metadata = load_cargo_metadata()
    lock_data = read_regular_file(ROOT / "Cargo.lock")
    return render_sbom(
        build_sbom(
            metadata,
            lock_data,
            args.release_version,
            args.source_revision,
            args.source_date_epoch,
        )
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subcommands = result.add_subparsers(dest="command", required=True)
    for command in ("generate", "verify"):
        subparser = subcommands.add_parser(command)
        subparser.add_argument("--sbom", type=Path, required=True)
        subparser.add_argument("--release-version", required=True)
        subparser.add_argument("--source-revision", required=True)
        subparser.add_argument("--source-date-epoch", type=int, required=True)
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        expected = build_expected(args)
        if args.command == "generate":
            write_exclusive(args.sbom, expected)
            print(f"created {args.sbom}")
        else:
            verify_file(args.sbom, expected)
            print(f"verified {args.sbom}")
    except SbomError as error:
        print(f"release-sbom: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
