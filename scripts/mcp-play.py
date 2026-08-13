#!/usr/bin/env python3
"""Drive a freshly built Numinous MCP server over stdio.

Each invocation owns a unique temporary profile containing every MCP-managed
state path. The profile is removed before the process exits, so QA calls cannot
contaminate a player or another concurrent tester.
"""

from __future__ import annotations

import argparse
import atexit
import hashlib
import importlib.util
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import textwrap
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent


def load_source_integrity():
    """Load the shared exact-source verifier without modifying import paths."""
    path = ROOT / "scripts" / "understanding-source.py"
    spec = importlib.util.spec_from_file_location("numinous_mcp_source", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load understanding-source.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


source_integrity = load_source_integrity()
STATE_DIR_PREFIX = "numinous-mcp-play-"
BUILD_DIR_PREFIX = "numinous-mcp-build-"
BUILD_TIMEOUT_SECONDS = 300
SERVER_TIMEOUT_SECONDS = 30
MAX_REQUEST_LINE_BYTES = 1_048_576
MAX_SESSION_REQUESTS = 64
MAX_PROFILE_OPERATIONS = 128
MAX_RESPONSE_LINE_BYTES = 1_000_000
MAX_JSON_NESTING_DEPTH = 32
MAX_DIAGNOSTIC_CHARACTERS = 4096
MCP_PROTOCOL_VERSION = "2026-07-28"
PROTOCOL_VERSION_META_KEY = "io.modelcontextprotocol/protocolVersion"
CLIENT_INFO_META_KEY = "io.modelcontextprotocol/clientInfo"
CLIENT_CAPABILITIES_META_KEY = "io.modelcontextprotocol/clientCapabilities"
SERVER_INFO_META_KEY = "io.modelcontextprotocol/serverInfo"
BUILD_RECEIPT_SCHEMA = "numinous-mcp-build-receipt-v1"
DEVELOPMENT_BUILD_RECEIPT_SCHEMA = "numinous-mcp-development-build-receipt-v1"
SHA256_HEX = re.compile(r"^[0-9a-f]{64}$")
COMMIT_SHA = re.compile(r"^[0-9a-f]{40}$")
QUALIFYING_SOURCE_PATHS = (
    ".cargo",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "assets",
    "data",
    "crates",
    "faces",
    "scripts/mcp-play.py",
    "scripts/understanding-collect.py",
    "scripts/understanding-encounters.json",
    "scripts/understanding-source.py",
    "scripts/understanding-study.py",
)
BUILD_ENVIRONMENT_KEYS = frozenset(
    {
        "CARGO_HOME",
        "COMSPEC",
        "COMMONPROGRAMFILES",
        "COMMONPROGRAMFILES(X86)",
        "HOME",
        "LOCALAPPDATA",
        "PATH",
        "PATHEXT",
        "PROGRAMDATA",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "RUSTUP_HOME",
        "SYSTEMROOT",
        "TEMP",
        "TMP",
        "TMPDIR",
        "USERPROFILE",
        "WINDIR",
    }
)
WINDOWS_TOOLCHAIN_KEYS = frozenset(
    {
        "DEVENVDIR",
        "FRAMEWORKDIR",
        "FRAMEWORKDIR64",
        "FRAMEWORKVERSION",
        "FRAMEWORKVERSION64",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "NETFXSDKDIR",
        "PATH",
        "UCRTVERSION",
        "UNIVERSALCRTSDKDIR",
        "VCINSTALLDIR",
        "VCTOOLSINSTALLDIR",
        "VCTOOLSREDISTDIR",
        "VSINSTALLDIR",
        "WINDOWSLIBPATH",
        "WINDOWSSDKBINPATH",
        "WINDOWSSDKDIR",
        "WINDOWSSDKLIBVERSION",
        "WINDOWSSDKVERSION",
    }
)


class McpPlayError(RuntimeError):
    """A readable protocol, server, or tool failure."""


@dataclass(frozen=True)
class BuiltArtifact:
    """One private executable and its source-bound reproducibility receipt."""

    path: Path
    sha256: str
    receipt: dict[str, Any]
    owner: tempfile.TemporaryDirectory = field(repr=False, compare=False)


_BUILD_LOCK = threading.Lock()
_BUILD_CACHE: dict[tuple[str | None, str | None], BuiltArtifact] = {}
_BUILD_ENVIRONMENT_LOCK = threading.Lock()
_BUILD_ENVIRONMENT_CACHE: dict[str, str] | None = None


def _cleanup_build_cache() -> None:
    """Close process-cached build owners without implicit-cleanup warnings."""
    with _BUILD_LOCK:
        artifacts = list(_BUILD_CACHE.values())
        _BUILD_CACHE.clear()
    cleaned: set[int] = set()
    for artifact in artifacts:
        identity = id(artifact.owner)
        if identity not in cleaned:
            artifact.owner.cleanup()
            cleaned.add(identity)


atexit.register(_cleanup_build_cache)


def _is_redirecting_path(path: Path) -> bool:
    """Detect symbolic links and Windows reparse-point redirects."""
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    except OSError:
        return True
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    attributes = getattr(metadata, "st_file_attributes", 0)
    return path.is_symlink() or bool(reparse_flag and attributes & reparse_flag)


def _strict_json_loads(payload: str, location: str) -> Any:
    """Decode one protocol value while rejecting duplicate object keys."""

    def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"duplicate object key {key!r}")
            value[key] = item
        return value

    def reject_constant(value: str) -> None:
        raise ValueError(f"non-finite JSON constant {value!r}")

    try:
        value = json.loads(
            payload,
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except (RecursionError, ValueError) as error:
        raise McpPlayError(f"invalid JSON in {location}: {error}") from error
    pending = [(value, 0)]
    while pending:
        item, depth = pending.pop()
        if depth > MAX_JSON_NESTING_DEPTH:
            raise McpPlayError(f"invalid JSON in {location}: nesting limit exceeded")
        if isinstance(item, dict):
            pending.extend((nested, depth + 1) for nested in item.values())
        elif isinstance(item, list):
            pending.extend((nested, depth + 1) for nested in item)
    return value


def _sha256_file(path: Path) -> str:
    """Hash one bounded local artifact without trusting its filename."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise McpPlayError(f"could not hash server artifact: {error}") from error
    return digest.hexdigest()


def _build_environment() -> dict[str, str]:
    """Return the small inherited environment allowed to influence a build."""
    global _BUILD_ENVIRONMENT_CACHE
    with _BUILD_ENVIRONMENT_LOCK:
        if _BUILD_ENVIRONMENT_CACHE is not None:
            return dict(_BUILD_ENVIRONMENT_CACHE)
        env = {
            key: value
            for key, value in os.environ.items()
            if key.upper() in BUILD_ENVIRONMENT_KEYS
        }
    if os.name == "nt":
        program_files_x86 = os.environ.get("ProgramFiles(x86)")
        if program_files_x86 is None:
            raise McpPlayError("Windows build environment lacks ProgramFiles(x86)")
        vswhere = (
            Path(program_files_x86)
            / "Microsoft Visual Studio"
            / "Installer"
            / "vswhere.exe"
        )
        try:
            discovery = subprocess.run(
                [
                    str(vswhere),
                    "-latest",
                    "-products",
                    "*",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-property",
                    "installationPath",
                ],
                env=env,
                capture_output=True,
                text=True,
                check=False,
                timeout=30,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise McpPlayError(f"could not locate the MSVC build environment: {error}") from error
        installation = discovery.stdout.strip()
        if discovery.returncode != 0 or not installation:
            raise McpPlayError("MSVC Build Tools with the x64 compiler are required")
        vcvars = Path(installation) / "VC" / "Auxiliary" / "Build" / "vcvars64.bat"
        comspec = env.get("COMSPEC", "cmd.exe")
        if not vcvars.is_file() or any(character in str(vcvars) for character in '"&|<>^'):
            raise McpPlayError("MSVC vcvars64 path is missing or unsafe")
        vcvars_command = f'"{comspec}" /d /c ""{vcvars}" >nul && set"'
        try:
            configured = subprocess.run(
                vcvars_command,
                env=env,
                capture_output=True,
                text=True,
                check=False,
                timeout=30,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise McpPlayError(
                f"could not initialize the MSVC build environment: {error}"
            ) from error
        if configured.returncode != 0:
            detail = configured.stderr[:MAX_DIAGNOSTIC_CHARACTERS].strip()
            raise McpPlayError(
                "could not initialize the MSVC build environment: "
                f"{detail or 'vcvars64 failed'}"
            )
        for line in configured.stdout.splitlines():
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            if key.upper() in WINDOWS_TOOLCHAIN_KEYS:
                env[key] = value
    env.update(
        {
            "CARGO_INCREMENTAL": "0",
            "CARGO_TERM_COLOR": "never",
        }
    )
    with _BUILD_ENVIRONMENT_LOCK:
        if _BUILD_ENVIRONMENT_CACHE is None:
            _BUILD_ENVIRONMENT_CACHE = dict(env)
        return dict(_BUILD_ENVIRONMENT_CACHE)


def _run_text(
    command: list[str], *, env: dict[str, str], timeout: int = BUILD_TIMEOUT_SECONDS
) -> str:
    """Run one build metadata command and return bounded UTF-8 output."""
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise McpPlayError(f"build command {command[0]!r} failed: {error}") from error
    if result.returncode != 0:
        detail = (result.stderr or result.stdout)[:MAX_DIAGNOSTIC_CHARACTERS].strip()
        raise McpPlayError(
            f"build command {command[0]!r} exited with status {result.returncode}: "
            f"{detail or 'no diagnostic output'}"
        )
    output = result.stdout.strip()
    if len(output.encode("utf-8")) > 64_000:
        raise McpPlayError(f"build command {command[0]!r} output exceeds its limit")
    return output


def _git_output(arguments: list[str], env: dict[str, str]) -> str:
    """Run one read-only Git query through the bounded build command wrapper."""
    return _run_text(["git", *arguments], env=env, timeout=30)


def _require_qualifying_source(
    expected_revision: str, expected_source_sha256: str, env: dict[str, str]
) -> None:
    """Require the exact clean committed source boundary used by the study."""
    if not COMMIT_SHA.fullmatch(expected_revision):
        raise McpPlayError("qualifying source revision is invalid")
    if not SHA256_HEX.fullmatch(expected_source_sha256):
        raise McpPlayError("qualifying study source digest is invalid")
    try:
        source_integrity.verify_source_tree(
            ROOT,
            QUALIFYING_SOURCE_PATHS,
            expected_revision=expected_revision,
            whole_worktree_clean=False,
            environment=env,
        )
    except source_integrity.SourceIntegrityError as error:
        raise McpPlayError(f"qualifying source verification failed: {error}") from error


def _has_unbound_cargo_configuration(env: dict[str, str]) -> bool:
    """Reject Cargo configuration outside the source-bound project directory."""
    candidates = [
        parent / ".cargo" / name
        for parent in ROOT.parents
        for name in ("config", "config.toml")
    ]
    cargo_home = env.get("CARGO_HOME")
    if cargo_home is None:
        home = env.get("USERPROFILE") or env.get("HOME")
        if home is not None:
            cargo_home = str(Path(home) / ".cargo")
    if cargo_home is not None:
        cargo_root = Path(cargo_home)
        if cargo_root.resolve() != (ROOT / ".cargo").resolve():
            candidates.extend(cargo_root / name for name in ("config", "config.toml"))
    return any(path.is_file() for path in candidates)


def _toolchain_metadata(env: dict[str, str]) -> tuple[str, str, str]:
    """Return bounded Cargo, Rust compiler, and explicit host target identities."""
    cargo_version = _run_text(["cargo", "--version", "--verbose"], env=env)
    rustc_details = _run_text(["rustc", "-vV"], env=env)
    rustc_lines = rustc_details.splitlines()
    rustc_version = next((line for line in rustc_lines if line.startswith("rustc ")), "")
    host_target = next(
        (line.removeprefix("host: ") for line in rustc_lines if line.startswith("host: ")),
        "",
    )
    if not rustc_version or not host_target or not re.fullmatch(r"[A-Za-z0-9_.-]+", host_target):
        raise McpPlayError("Rust toolchain metadata is incomplete")
    return cargo_version, rustc_version, host_target


def _cargo_artifact(
    target_dir: Path, host_target: str, env: dict[str, str]
) -> Path:
    """Build once and return Cargo's exact executable artifact from JSON output."""
    command = [
        "cargo",
        "build",
        "--locked",
        "--bin",
        "numinous-mcp",
        "--no-default-features",
        "--target",
        host_target,
        "--target-dir",
        str(target_dir),
        "--message-format=json-render-diagnostics",
    ]
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=BUILD_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        raise McpPlayError(
            f"server build exceeded {BUILD_TIMEOUT_SECONDS} seconds"
        ) from error
    except OSError as error:
        raise McpPlayError(f"server build could not start: {error}") from error
    if result.returncode != 0:
        detail = (result.stderr or result.stdout)[:MAX_DIAGNOSTIC_CHARACTERS].strip()
        raise McpPlayError(
            f"server build exited with status {result.returncode}: "
            f"{detail or 'no diagnostic output'}"
        )
    artifacts: list[Path] = []
    for line_number, line in enumerate(result.stdout.splitlines(), start=1):
        if not line.strip():
            continue
        message = _strict_json_loads(line, f"Cargo build line {line_number}")
        if not isinstance(message, dict) or not isinstance(message.get("reason"), str):
            raise McpPlayError("Cargo build emitted an invalid JSON message")
        target = message.get("target")
        executable = message.get("executable")
        if (
            message["reason"] == "compiler-artifact"
            and isinstance(target, dict)
            and target.get("name") == "numinous-mcp"
            and target.get("kind") == ["bin"]
            and isinstance(executable, str)
        ):
            artifacts.append(Path(executable).resolve())
    if len(artifacts) != 1:
        raise McpPlayError("Cargo did not identify exactly one Numinous MCP executable")
    artifact = artifacts[0]
    try:
        artifact.relative_to(target_dir.resolve())
    except ValueError as error:
        raise McpPlayError("Cargo executable escaped the private target directory") from error
    if not artifact.is_file():
        raise McpPlayError("Cargo executable is missing after a successful build")
    return artifact


def _private_artifact_copy(source: Path, artifact_dir: Path) -> Path:
    """Copy one Cargo artifact once into its private immutable execution path."""
    artifact_dir.mkdir()
    destination = artifact_dir / source.name
    try:
        with source.open("rb") as source_file, destination.open("xb") as destination_file:
            shutil.copyfileobj(source_file, destination_file, length=1024 * 1024)
            destination_file.flush()
            os.fsync(destination_file.fileno())
        destination.chmod(stat.S_IREAD | stat.S_IEXEC)
    except OSError as error:
        raise McpPlayError(f"could not freeze server artifact: {error}") from error
    return destination


def _build_artifact(
    expected_revision: str | None, expected_source_sha256: str | None
) -> BuiltArtifact:
    """Build, freeze, and attest one private executable from the selected source."""
    qualifying = expected_revision is not None or expected_source_sha256 is not None
    if qualifying and (expected_revision is None or expected_source_sha256 is None):
        raise McpPlayError("qualifying source identity is incomplete")
    env = _build_environment()
    if qualifying:
        assert expected_revision is not None and expected_source_sha256 is not None
        _require_qualifying_source(expected_revision, expected_source_sha256, env)
        if _has_unbound_cargo_configuration(env):
            raise McpPlayError("qualifying build refuses unbound Cargo configuration")
    revision = _git_output(["rev-parse", "HEAD"], env)
    cargo_version, rustc_version, host_target = _toolchain_metadata(env)
    owner = tempfile.TemporaryDirectory(prefix=BUILD_DIR_PREFIX)
    build_root = Path(owner.name)
    target_dir = build_root / "target" if qualifying else ROOT / "target"
    cargo_artifact = _cargo_artifact(target_dir, host_target, env)
    frozen_artifact = _private_artifact_copy(cargo_artifact, build_root / "artifact")
    binary_sha256 = _sha256_file(frozen_artifact)
    if qualifying:
        assert expected_revision is not None and expected_source_sha256 is not None
        _require_qualifying_source(expected_revision, expected_source_sha256, env)
    receipt = {
        "schemaVersion": (
            BUILD_RECEIPT_SCHEMA if qualifying else DEVELOPMENT_BUILD_RECEIPT_SCHEMA
        ),
        "sourceRevision": revision,
        "studySourceSha256": expected_source_sha256,
        "sourcePolicy": (
            "verified-clean-commit-before-and-after"
            if qualifying
            else "unbound-working-tree"
        ),
        "environmentPolicy": "bounded-inheritance-no-build-overrides-v1",
        "cargoVersion": cargo_version,
        "rustcVersion": rustc_version,
        "targetTriple": host_target,
        "profile": "debug",
        "features": "none",
        "locked": True,
        "incremental": False,
        "targetDirectoryPolicy": (
            "fresh-explicit-private" if qualifying else "explicit-development-cache"
        ),
        "artifactPolicy": "cargo-json-private-copy-hash-before-and-after-execution",
        "binarySha256": binary_sha256,
    }
    return BuiltArtifact(frozen_artifact, binary_sha256, receipt, owner)


def _binary(
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
) -> BuiltArtifact:
    """Return one process-cached private build bound to the requested source."""
    key = (expected_revision, expected_source_sha256)
    with _BUILD_LOCK:
        artifact = _BUILD_CACHE.get(key)
        if artifact is None:
            artifact = _build_artifact(expected_revision, expected_source_sha256)
            _BUILD_CACHE[key] = artifact
        elif _sha256_file(artifact.path) != artifact.sha256:
            raise McpPlayError("private server artifact changed after it was frozen")
        if expected_revision is not None and expected_source_sha256 is not None:
            _require_qualifying_source(
                expected_revision, expected_source_sha256, _build_environment()
            )
        return artifact


def _session(
    requests: list[dict[str, Any]],
    *,
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
    state_root: Path | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Send requests through one fresh server and one bounded profile."""
    if state_root is None:
        with tempfile.TemporaryDirectory(prefix=STATE_DIR_PREFIX) as state_dir:
            return _session_in_profile(
                requests,
                Path(state_dir),
                expected_revision=expected_revision,
                expected_source_sha256=expected_source_sha256,
            )
    return _session_in_profile(
        requests,
        state_root,
        expected_revision=expected_revision,
        expected_source_sha256=expected_source_sha256,
    )


def _session_in_profile(
    requests: list[dict[str, Any]],
    state_root: Path,
    *,
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Send requests through one fresh server using one caller-owned profile."""
    if len(requests) > MAX_SESSION_REQUESTS:
        raise McpPlayError(
            f"server session exceeds the {MAX_SESSION_REQUESTS}-request limit"
        )
    encoded_requests = []
    for index, request in enumerate(requests, start=1):
        encoded = (json.dumps(request) + "\n").encode("utf-8")
        if len(encoded) > MAX_REQUEST_LINE_BYTES:
            raise McpPlayError(f"server request {index} exceeds the size limit")
        encoded_requests.append(encoded)
    payload = b"".join(encoded_requests)
    expected_responses = sum("id" in request for request in requests)
    maximum_output_bytes = expected_responses * (MAX_RESPONSE_LINE_BYTES + 1)

    if _is_redirecting_path(state_root):
        raise McpPlayError("test profile must be an ordinary directory")
    try:
        state_root = state_root.resolve(strict=True)
    except OSError as error:
        raise McpPlayError(f"test profile is not available: {error}") from error
    if not state_root.is_dir() or _is_redirecting_path(state_root):
        raise McpPlayError("test profile must be an ordinary directory")
    env = {
        key: os.environ[key]
        for key in ("COMSPEC", "SYSTEMROOT", "WINDIR")
        if key in os.environ
    }
    env.update(
        {
            "NUMINOUS_JOURNEY": str(state_root / "journey.txt"),
            "NUMINOUS_SCORES": str(state_root / "scores.txt"),
            "NUMINOUS_CAIRN": str(state_root / "cairn.json"),
            "NUMINOUS_JOURNAL": str(state_root / "journal.txt"),
            "HOME": str(state_root),
            "USERPROFILE": str(state_root),
            "TEMP": str(state_root),
            "TMP": str(state_root),
            "TMPDIR": str(state_root),
        }
    )
    artifact = _binary(expected_revision, expected_source_sha256)
    binary = artifact.path
    if _sha256_file(binary) != artifact.sha256:
        raise McpPlayError("private server artifact changed before execution")
    with tempfile.TemporaryFile(mode="w+b") as stdout_file:
        with tempfile.TemporaryFile(mode="w+b") as stderr_file:
            try:
                process = subprocess.run(
                    [str(binary)],
                    input=payload,
                    stdout=stdout_file,
                    stderr=stderr_file,
                    cwd=ROOT,
                    env=env,
                    check=False,
                    timeout=SERVER_TIMEOUT_SECONDS,
                )
            except subprocess.TimeoutExpired as error:
                raise McpPlayError(
                    f"server session exceeded {SERVER_TIMEOUT_SECONDS} seconds"
                ) from error
            finally:
                if _sha256_file(binary) != artifact.sha256:
                    raise McpPlayError("private server artifact changed during execution")

            stdout_file.seek(0, os.SEEK_END)
            output_bytes = stdout_file.tell()
            if output_bytes > maximum_output_bytes:
                raise McpPlayError("server session output exceeds the size limit")
            if process.returncode != 0:
                stderr_file.seek(0)
                detail_bytes = stderr_file.read(MAX_DIAGNOSTIC_CHARACTERS + 1)
                if not detail_bytes:
                    stdout_file.seek(0)
                    detail_bytes = stdout_file.read(MAX_DIAGNOSTIC_CHARACTERS + 1)
                detail = detail_bytes.decode("utf-8", errors="replace").strip()
                raise McpPlayError(
                    f"server exited with status {process.returncode}: "
                    f"{detail or 'no diagnostic output'}"
                )

            responses: list[dict[str, Any]] = []
            stdout_file.seek(0)
            for line_number, encoded_line in enumerate(stdout_file, start=1):
                if len(encoded_line) > MAX_RESPONSE_LINE_BYTES + 1:
                    raise McpPlayError(
                        f"server response line {line_number} exceeds the size limit"
                    )
                try:
                    line = encoded_line.decode("utf-8").strip()
                except UnicodeDecodeError as error:
                    raise McpPlayError(
                        f"invalid UTF-8 in server response line {line_number}"
                    ) from error
                if not line:
                    continue
                response = _strict_json_loads(
                    line, f"server response line {line_number}"
                )
                if not isinstance(response, dict):
                    raise McpPlayError(
                        f"server returned a non-object response on line {line_number}"
                    )
                responses.append(response)
    if len(responses) != expected_responses:
        raise McpPlayError(
            f"server returned {len(responses)} response(s) for "
            f"{expected_responses} request(s)"
        )
    expected_ids = [request["id"] for request in requests if "id" in request]
    for response, expected_id in zip(responses, expected_ids, strict=True):
        has_result = "result" in response
        has_error = "error" in response
        expected_fields = {"jsonrpc", "id", "result" if has_result else "error"}
        if (
            response.get("jsonrpc") != "2.0"
            or response.get("id") != expected_id
            or isinstance(response.get("id"), bool)
            or has_result == has_error
            or set(response) != expected_fields
            or (has_error and not isinstance(response["error"], dict))
        ):
            raise McpPlayError(
                f"server returned an invalid response for request id {expected_id}"
            )
    return responses, dict(artifact.receipt)


def _response_result(response: dict[str, Any], operation: str) -> dict[str, Any]:
    """Return one successful JSON-RPC result or raise a readable failure."""
    error = response.get("error")
    if isinstance(error, dict):
        code = error.get("code", "unknown")
        message = error.get("message", "no error message")
        raise McpPlayError(f"{operation} failed ({code}): {message}")
    result = response.get("result")
    if not isinstance(result, dict):
        raise McpPlayError(f"{operation} returned no object result")
    return result


def _tool_text(result: dict[str, Any]) -> str:
    """Collect every textual content block from a tool result."""
    content = result.get("content", [])
    if not isinstance(content, list):
        return ""
    blocks = []
    for item in content:
        if isinstance(item, dict) and isinstance(item.get("text"), str):
            blocks.append(item["text"])
    return "\n".join(blocks)


def _modern_request(request: dict[str, Any]) -> dict[str, Any]:
    """Attach the complete stateless protocol metadata to one request."""
    prepared = {**request}
    params = dict(prepared.get("params", {}))
    params["_meta"] = {
        PROTOCOL_VERSION_META_KEY: MCP_PROTOCOL_VERSION,
        CLIENT_INFO_META_KEY: {"name": "mcp-play", "version": "1"},
        CLIENT_CAPABILITIES_META_KEY: {},
    }
    prepared["params"] = params
    return prepared


def _discover(
    extra: list[dict[str, Any]],
    *,
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
    state_root: Path | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Discover the server, then issue independently versioned modern requests."""
    discover = _modern_request(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
        }
    )
    responses, build_receipt = _session(
        [discover, *(_modern_request(request) for request in extra)],
        expected_revision=expected_revision,
        expected_source_sha256=expected_source_sha256,
        state_root=state_root,
    )
    discovery = _response_result(responses[0], "server/discover")
    if (
        discovery.get("resultType") != "complete"
        or MCP_PROTOCOL_VERSION not in discovery.get("supportedVersions", [])
        or not isinstance(discovery.get("capabilities"), dict)
    ):
        raise McpPlayError("server/discover returned an incompatible result")
    return responses, build_receipt


class IsolatedMcpProfile:
    """A disposable profile shared across a bounded sequence of MCP calls."""

    def __init__(self) -> None:
        self._owner = tempfile.TemporaryDirectory(prefix=STATE_DIR_PREFIX)
        self._root = Path(self._owner.name)
        self._closed = False
        self._operations = 0

    def __enter__(self) -> "IsolatedMcpProfile":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def close(self) -> None:
        """Erase the complete disposable profile exactly once."""
        if not self._closed:
            self._owner.cleanup()
            self._closed = True

    def _claim_operation(self) -> None:
        if self._closed:
            raise McpPlayError("test profile is already closed")
        if self._operations >= MAX_PROFILE_OPERATIONS:
            raise McpPlayError(
                f"test profile exceeds the {MAX_PROFILE_OPERATIONS}-operation limit"
            )
        self._operations += 1

    def list_tools(self) -> tuple[list[dict[str, Any]], dict[str, Any]]:
        """Return the server's complete tool definitions and build receipt."""
        self._claim_operation()
        responses, build_receipt = _discover(
            [{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}],
            state_root=self._root,
        )
        result = _response_result(responses[-1], "tools/list")
        tools = result.get("tools")
        if not isinstance(tools, list) or any(not isinstance(tool, dict) for tool in tools):
            raise McpPlayError("tools/list returned a malformed tool array")
        return tools, build_receipt

    def call_tool(self, tool: str, arguments: dict[str, Any]) -> dict[str, Any]:
        """Call one tool while retaining this profile's player-owned state."""
        self._claim_operation()
        responses, _build_receipt = _discover(
            [
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {"name": tool, "arguments": arguments},
                }
            ],
            state_root=self._root,
        )
        return _response_result(responses[-1], f"tool '{tool}'")


def isolated_tool_call(
    tool: str,
    arguments: dict[str, Any],
    *,
    expected_revision: str | None = None,
    expected_source_sha256: str | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Call one tool through a fresh isolated server and return canonical results."""
    responses, build_receipt = _discover(
        [
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": tool, "arguments": arguments},
            }
        ],
        expected_revision=expected_revision,
        expected_source_sha256=expected_source_sha256,
    )
    discovery = _response_result(responses[0], "server/discover")
    server_info = discovery.get("_meta", {}).get(SERVER_INFO_META_KEY)
    initialization = {
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "supportedVersions": discovery.get("supportedVersions"),
        "capabilities": discovery.get("capabilities"),
        "serverInfo": server_info,
        "numinousBinarySha256": build_receipt["binarySha256"],
        "binaryBuildReceipt": build_receipt,
    }
    result = _response_result(responses[-1], f"tool '{tool}'")
    text = _tool_text(result)
    if result.get("isError") is True:
        raise McpPlayError(f"tool '{tool}' failed: {text or 'no error message'}")
    return initialization, result


def _call_tool(tool: str, arguments: dict[str, Any]) -> int:
    _initialization, result = isolated_tool_call(tool, arguments)
    text = _tool_text(result)
    if text:
        print(text)
    structured = result.get("structuredContent")
    if structured is not None:
        print("\n--- structuredContent ---")
        print(json.dumps(structured, indent=2))
    return 0


def _list_tools() -> int:
    responses, _binary_sha256 = _discover(
        [{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}]
    )
    result = _response_result(responses[-1], "tools/list")
    tools = result.get("tools")
    if not isinstance(tools, list):
        raise McpPlayError("tools/list returned no tool array")
    for tool in tools:
        if not isinstance(tool, dict):
            raise McpPlayError("tools/list returned a malformed tool entry")
        name = tool.get("name", "<unnamed>")
        description = tool.get("description", "No description provided.")
        print(name)
        print(
            textwrap.fill(
                str(description),
                width=88,
                initial_indent="  ",
                subsequent_indent="  ",
                break_long_words=False,
                break_on_hyphens=False,
            )
        )
        print()
    print(f"{len(tools)} tools.")
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Build the latest Numinous MCP server and exercise it through an "
            "isolated, automatically cleaned test profile."
        ),
        epilog=textwrap.dedent(
            """
            examples:
              python scripts/mcp-play.py list
              python scripts/mcp-play.py tools
              python scripts/mcp-play.py call play_room '{"id":"lorenz","t":0.5}'
              python scripts/mcp-play.py call predict '{"id":"slope-rider","seed":4}'
              '{"id":"cult-of-pi"}' | python scripts/mcp-play.py call describe_room -

            Each command starts with empty Journey, score, Cairn, journal, radio,
            and crash state. Use a direct MCP session when a test intentionally
            needs persistent state. Pass - to read JSON from stdin, which avoids
            shell quoting differences.
            """
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("list", help="list all catalog rooms")
    subcommands.add_parser(
        "tools", help="show every tool with its complete description"
    )
    call = subcommands.add_parser("call", help="call one MCP tool")
    call.add_argument("tool", help="tool name, for example play_room")
    call.add_argument(
        "arguments",
        nargs="?",
        default="{}",
        help="JSON object of tool arguments, or - to read it from stdin (default: {})",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _parser()
    args = parser.parse_args(argv)
    try:
        if args.command == "tools":
            return _list_tools()
        if args.command == "list":
            return _call_tool("list_rooms", {})
        if args.arguments == "-":
            encoded_arguments = sys.stdin.buffer.read(MAX_REQUEST_LINE_BYTES + 1)
            if len(encoded_arguments) > MAX_REQUEST_LINE_BYTES:
                print("mcp-play: tool arguments exceed the size limit", file=sys.stderr)
                return 2
            try:
                raw_arguments = encoded_arguments.decode("utf-8")
            except UnicodeDecodeError:
                print("mcp-play: tool arguments are not valid UTF-8", file=sys.stderr)
                return 2
        else:
            raw_arguments = args.arguments
            if len(raw_arguments.encode("utf-8")) > MAX_REQUEST_LINE_BYTES:
                print("mcp-play: tool arguments exceed the size limit", file=sys.stderr)
                return 2
        try:
            arguments = _strict_json_loads(raw_arguments, "tool arguments")
        except McpPlayError as error:
            print(f"mcp-play: bad JSON arguments: {error}", file=sys.stderr)
            return 2
        if not isinstance(arguments, dict):
            print("mcp-play: tool arguments must be a JSON object", file=sys.stderr)
            return 2
        return _call_tool(args.tool, arguments)
    except McpPlayError as error:
        print(f"mcp-play: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        print(
            f"mcp-play: could not build the latest server (status {error.returncode})",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
