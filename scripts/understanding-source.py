#!/usr/bin/env python3
"""Verify that qualifying study source exactly matches one Git commit."""

from __future__ import annotations

import hashlib
import os
import re
import stat
import subprocess
from pathlib import Path, PurePosixPath
from typing import Mapping, Sequence

COMMIT_SHA = re.compile(r"^[0-9a-f]{40}$")
MAX_GIT_OUTPUT_BYTES = 16 * 1024 * 1024
WINDOWS_REPARSE_POINT = 0x0400


class SourceIntegrityError(ValueError):
    """Raised when a qualifying source tree cannot be proven exact."""


def _git_environment(environment: Mapping[str, str] | None) -> dict[str, str]:
    """Remove caller-controlled Git repository and configuration selection."""
    inherited = os.environ if environment is None else environment
    bounded = {
        key: value
        for key, value in inherited.items()
        if not key.upper().startswith("GIT_")
    }
    bounded["GIT_CONFIG_NOSYSTEM"] = "1"
    bounded["GIT_CONFIG_GLOBAL"] = os.devnull
    bounded["GIT_OPTIONAL_LOCKS"] = "0"
    return bounded


def _git(
    root: Path,
    arguments: Sequence[str],
    environment: Mapping[str, str] | None,
) -> bytes:
    """Run one bounded read-only Git query anchored to the selected worktree."""
    try:
        result = subprocess.run(
            ["git", *arguments],
            cwd=root,
            env=_git_environment(environment),
            check=True,
            capture_output=True,
            text=False,
            timeout=60,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
        raise SourceIntegrityError(f"source identity Git query failed: {error}") from error
    if len(result.stdout) > MAX_GIT_OUTPUT_BYTES:
        raise SourceIntegrityError("source identity Git output exceeds its limit")
    return result.stdout


def _text_output(
    root: Path,
    arguments: Sequence[str],
    environment: Mapping[str, str] | None,
) -> str:
    try:
        return _git(root, arguments, environment).decode("utf-8").strip()
    except UnicodeDecodeError as error:
        raise SourceIntegrityError("source identity Git output is not UTF-8") from error


def _has_reparse_point(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise SourceIntegrityError(f"runtime source path is unavailable: {path}") from error
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & WINDOWS_REPARSE_POINT
    )


def _require_plain_path(root: Path, path: Path, checked: set[Path]) -> None:
    """Reject links and reparse points between the worktree and one source file."""
    current = path
    while current != root:
        if current in checked:
            return
        if _has_reparse_point(current):
            raise SourceIntegrityError(f"runtime source path is indirect: {current}")
        checked.add(current)
        current = current.parent
    if _has_reparse_point(root):
        raise SourceIntegrityError("runtime source worktree is indirect")
    checked.add(root)


def _blob_sha1(path: Path) -> str:
    """Hash one plain file as a Git blob without loading it all into memory."""
    try:
        size = path.stat().st_size
        digest = hashlib.sha1(usedforsecurity=False)
        digest.update(f"blob {size}\0".encode("ascii"))
        observed = 0
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                observed += len(chunk)
                digest.update(chunk)
    except OSError as error:
        raise SourceIntegrityError(f"runtime source file is unavailable: {path}") from error
    if observed != size:
        raise SourceIntegrityError(f"runtime source file changed while reading: {path}")
    return digest.hexdigest()


def _tree_entries(tree: bytes) -> list[tuple[str, str, str]]:
    entries: list[tuple[str, str, str]] = []
    for record in tree.split(b"\0"):
        if not record:
            continue
        try:
            metadata, encoded_path = record.split(b"\t", 1)
            mode, kind, object_id = metadata.decode("ascii").split(" ")
            relative = encoded_path.decode("utf-8")
        except (UnicodeDecodeError, ValueError) as error:
            raise SourceIntegrityError("committed source tree entry is invalid") from error
        if kind != "blob" or mode not in {"100644", "100755"}:
            raise SourceIntegrityError(f"unsupported runtime source entry: {relative}")
        if not COMMIT_SHA.fullmatch(object_id):
            raise SourceIntegrityError("committed runtime source object is invalid")
        entries.append((relative, mode, object_id))
    return entries


def _require_exact_filesystem(
    root: Path,
    revision: str,
    paths: Sequence[str],
    environment: Mapping[str, str] | None,
) -> None:
    tree = _git(
        root,
        ["ls-tree", "-r", "-z", "--full-tree", revision, "--", *paths],
        environment,
    )
    entries = _tree_entries(tree)
    represented = set()
    checked: set[Path] = set()
    for relative, _mode, expected_object in entries:
        parts = PurePosixPath(relative).parts
        if not parts or any(part in {"", ".", ".."} for part in parts):
            raise SourceIntegrityError("committed runtime source path is invalid")
        source = root.joinpath(*parts)
        _require_plain_path(root, source, checked)
        if _blob_sha1(source) != expected_object:
            raise SourceIntegrityError(
                f"runtime source file differs from commit: {relative}"
            )
        for declared in paths:
            if relative == declared or relative.startswith(f"{declared}/"):
                represented.add(declared)
    missing = sorted(set(paths) - represented)
    if missing:
        raise SourceIntegrityError(
            f"declared runtime source is absent from commit: {missing[0]}"
        )


def verify_source_tree(
    root: Path,
    paths: Sequence[str],
    *,
    expected_revision: str | None = None,
    whole_worktree_clean: bool,
    environment: Mapping[str, str] | None = None,
) -> tuple[str, dict[str, str]]:
    """Verify repository anchoring, cleanliness, index state, and actual bytes."""
    root = root.resolve()
    top_level = Path(_text_output(root, ["rev-parse", "--show-toplevel"], environment))
    if top_level.resolve() != root:
        raise SourceIntegrityError("source identity Git worktree differs from repository root")
    revision = _text_output(root, ["rev-parse", "HEAD"], environment)
    if not COMMIT_SHA.fullmatch(revision):
        raise SourceIntegrityError("repository HEAD is not a full commit SHA")
    if expected_revision is not None and revision != expected_revision:
        raise SourceIntegrityError("qualifying source revision differs from HEAD")

    status_arguments = ["status", "--porcelain=v1", "--untracked-files=all"]
    if not whole_worktree_clean:
        status_arguments.extend(["--", *paths])
    if _git(root, status_arguments, environment).strip():
        raise SourceIntegrityError("qualifying runtime source worktree is dirty")
    if _git(
        root,
        [
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "-z",
            "--",
            *paths,
        ],
        environment,
    ).strip(b"\0"):
        raise SourceIntegrityError("qualifying runtime source contains ignored files")

    indexed = _git(root, ["ls-files", "-v", "-z", "--", *paths], environment)
    if any(record and not record.startswith(b"H ") for record in indexed.split(b"\0")):
        raise SourceIntegrityError("qualifying runtime source has nonordinary index flags")

    _require_exact_filesystem(root, revision, paths, environment)
    identities = {}
    for relative in paths:
        identity = _text_output(
            root, ["rev-parse", f"{revision}:{relative}"], environment
        )
        if not COMMIT_SHA.fullmatch(identity):
            raise SourceIntegrityError(
                f"study source {relative} has an invalid object identity"
            )
        identities[relative] = identity
    return revision, identities
