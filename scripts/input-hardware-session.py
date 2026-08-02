#!/usr/bin/env python3
"""Record one release-bound physical input session without overstating evidence."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
from pathlib import PurePosixPath
import platform
import re
import stat
import subprocess
import sys
import tempfile
import time
from types import ModuleType
from typing import Any, Callable, Iterator, NoReturn, Sequence


ROOT = Path(__file__).resolve().parent.parent
LOG_ROOT = ROOT / "logs" / "input-sessions"
SCHEMA = "numinous.physical-input-session"
SCHEMA_VERSION = 1
MAX_RECEIPT_BYTES = 262_144
MAX_NOTE_CHARACTERS = 500
MAX_BINARY_ARCHIVE_BYTES = 256 * 1024 * 1024
MAX_INSTALLED_BINARY_BYTES = 128 * 1024 * 1024
APP_STARTUP_SECONDS = 5.0
APP_EXIT_SECONDS = 15.0
SHA256_RE = re.compile(r"[0-9a-f]{64}")
PERSISTENCE_RE = re.compile(r"JOURNEY LV ([1-9][0-9]*) \| ([0-9]+) XP")
LIMITATIONS = (
    "Observations are operator-attested physical session evidence, not automated native event capture.",
    "One host and controller cannot establish broad accessibility, comfort, or hardware compatibility.",
    "The receipt contains no claim about systems or controller models not named in it.",
    "The content identifier detects unresealed changes but is not a signature or external custody proof.",
)


class SessionError(RuntimeError):
    """A physical input session or its receipt violated the evidence contract."""


def unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build one JSON object while rejecting ambiguous duplicate member names."""
    value: dict[str, Any] = {}
    for name, member in pairs:
        if name in value:
            raise SessionError(f"JSON object repeats field {name!r}")
        value[name] = member
    return value


def reject_nonfinite_json(value: str) -> NoReturn:
    """Reject nonstandard JSON numeric constants."""
    raise SessionError(f"JSON contains non-finite number {value}")


def parse_json(data: str | bytes) -> Any:
    """Parse JSON with duplicate-name and non-finite-number rejection."""
    return json.loads(
        data,
        object_pairs_hook=unique_json_object,
        parse_constant=reject_nonfinite_json,
    )


def load_sibling(name: str, filename: str) -> ModuleType:
    """Load one repository script whose filename is not a Python identifier."""
    specification = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    if specification is None or specification.loader is None:
        raise SessionError(f"could not load required script: {filename}")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


PACKAGE = load_sibling("numinous_package_release", "package-release.py")
SMOKE = load_sibling(
    "numinous_release_engagement_smoke", "release-engagement-smoke.py"
)
MATRIX_TARGETS = frozenset(PACKAGE.TARGETS)
MATRIX_CONTROLLER_PROFILES = frozenset({"xbox", "playstation", "generic"})
MATRIX_MIN_CONTROLLER_MODELS = 3


CHECKPOINTS: tuple[dict[str, str], ...] = (
    {
        "id": "keyboard.menu-and-room-navigation",
        "input": "keyboard",
        "action": "Use Escape to open and close the menu, then A and D or arrows to move both ways between rooms.",
        "expected": "The menu transition is clean and each navigation key changes the displayed room exactly once.",
    },
    {
        "id": "keyboard.room-lifecycle",
        "input": "keyboard",
        "action": "Use Space to pause and resume, R to reset, F to enter and leave fullscreen, and M to mute and unmute.",
        "expected": "Pause freezes motion, reset restores the room, fullscreen returns safely, and the audio badge reflects mute state.",
    },
    {
        "id": "pointer.menu",
        "input": "mouse",
        "action": "Hover over two menu destinations, click one, return, and click a different destination.",
        "expected": "Hover selection follows the pointer and each click opens only the selected destination.",
    },
    {
        "id": "pointer.room-gesture",
        "input": "mouse",
        "action": "In Times Tables, drag the dial through a visible change and use the wheel in both directions.",
        "expected": "The held drag is continuous, release ends it, and each wheel direction changes time without a stuck gesture.",
    },
    {
        "id": "controller.identity-and-reconnect",
        "input": "controller",
        "action": "Connect the named controller, make one meaningful input, disconnect during a held South gesture, then reconnect.",
        "expected": "The matching or generic controller legend appears, disconnect cancels the hold, and input resumes after reconnect.",
    },
    {
        "id": "controller.virtual-hand",
        "input": "controller",
        "action": "Move the virtual hand with the left stick and use South to drag the Times Tables dial.",
        "expected": "The hand follows the stick, South produces one bounded press and release, and the room changes causally.",
    },
    {
        "id": "controller.room-navigation",
        "input": "controller",
        "action": "Use both bumpers, both triggers, the right stick, Select, West, and L3 while wandering.",
        "expected": "Rooms, speed, phase, inspect, era, and reset each respond once and agree with the visible controller legend.",
    },
    {
        "id": "controller.game-and-pause",
        "input": "controller",
        "action": "Open the menu with Start, enter a game, make a D-pad and South move, then open and close Start pause.",
        "expected": "The game is playable without keyboard or pointer, pause blocks hidden input, and the live game state is preserved.",
    },
    {
        "id": "controller.audio",
        "input": "controller",
        "action": "Hold North with D-pad up and down, then hold North with South.",
        "expected": "Volume moves in both directions, mute toggles once, and the persistent audio badge reports each state.",
    },
    {
        "id": "lifecycle.persistence-write",
        "input": "lifecycle",
        "action": "From the fresh profile, play until XP rises above zero, open Journey, and record the exact values as JOURNEY LV <level> | <xp> XP.",
        "expected": "The first launch shows a positive earned XP value that can be compared after a clean restart.",
    },
    {
        "id": "app.restart-persistence",
        "input": "lifecycle",
        "action": "After the runner relaunches the same isolated profile, open Journey and record the exact values as JOURNEY LV <level> | <xp> XP.",
        "expected": "The second launch reports exactly the same positive level and XP recorded immediately before the first clean exit.",
    },
)


def target_for_host(system: str, machine: str) -> str:
    """Return the release target for one normalized operating system and CPU."""
    if system == "windows" and machine in {"amd64", "x86_64"}:
        return "x86_64-pc-windows-msvc"
    if system == "linux" and machine in {"amd64", "x86_64"}:
        return "x86_64-unknown-linux-gnu"
    if system == "darwin" and machine in {"amd64", "x86_64"}:
        return "x86_64-apple-darwin"
    if system == "darwin" and machine in {"arm64", "aarch64"}:
        return "aarch64-apple-darwin"
    raise SessionError(f"this host is not a supported release target: {system}/{machine}")


def expected_target() -> str:
    """Return the release target that can honestly describe this host."""
    return target_for_host(platform.system().lower(), platform.machine().lower())


def is_link_like(path: Path) -> bool:
    """Report symbolic links and Windows reparse points without following them."""
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    attributes = getattr(metadata, "st_file_attributes", 0)
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return stat.S_ISLNK(metadata.st_mode) or bool(attributes & reparse_flag)


def copy_regular_snapshot(
    source_path: Path, destination: Path, maximum_bytes: int
) -> tuple[int, str, int]:
    """Copy one bounded ordinary file from a held descriptor into private storage."""
    if is_link_like(source_path):
        raise SessionError(f"source is link-like: {source_path.name}")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(source_path, flags)
    except OSError as error:
        raise SessionError(f"source could not be opened: {source_path.name}") from error
    digest = hashlib.sha256()
    byte_count = 0
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        if not stat.S_ISREG(before.st_mode):
            raise SessionError(f"source is not an ordinary file: {source_path.name}")
        if before.st_size <= 0 or before.st_size > maximum_bytes:
            raise SessionError(f"source size is outside its bound: {source_path.name}")
        with destination.open("xb") as output:
            while chunk := source.read(1024 * 1024):
                byte_count += len(chunk)
                if byte_count > maximum_bytes:
                    raise SessionError(f"source exceeded its bound: {source_path.name}")
                digest.update(chunk)
                output.write(chunk)
            output.flush()
            os.fsync(output.fileno())
        after = os.fstat(source.fileno())
    if (
        byte_count != before.st_size
        or after.st_size != before.st_size
        or after.st_mtime_ns != before.st_mtime_ns
        or (before.st_ino and after.st_ino != before.st_ino)
    ):
        raise SessionError(f"source changed while it was captured: {source_path.name}")
    return byte_count, digest.hexdigest(), stat.S_IMODE(before.st_mode)


def read_bounded_regular(path: Path, maximum_bytes: int) -> bytes:
    """Read one bounded ordinary file from a stable held descriptor."""
    if is_link_like(path):
        raise SessionError(f"source is link-like: {path.name}")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise SessionError(f"source could not be opened: {path.name}") from error
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        if not stat.S_ISREG(before.st_mode):
            raise SessionError(f"source is not an ordinary file: {path.name}")
        if before.st_size <= 0 or before.st_size > maximum_bytes:
            raise SessionError(f"source size is outside its bound: {path.name}")
        data = source.read(maximum_bytes + 1)
        after = os.fstat(source.fileno())
    if (
        len(data) != before.st_size
        or len(data) > maximum_bytes
        or after.st_size != before.st_size
        or after.st_mtime_ns != before.st_mtime_ns
        or (before.st_ino and after.st_ino != before.st_ino)
    ):
        raise SessionError(f"source changed while it was read: {path.name}")
    return data


@contextmanager
def release_install_evidence(
    archive_path: Path, checksum_path: Path, bin_dir: Path
) -> Iterator[tuple[dict[str, Any], dict[str, Path]]]:
    """Yield private snapshots proven to match one verified binary archive."""
    archive_name = archive_path.name
    if not archive_name or PurePosixPath(archive_name).name != archive_name:
        raise SessionError("release archive name is malformed")
    with tempfile.TemporaryDirectory(prefix="numinous-verified-release-") as temporary:
        private_root = Path(temporary)
        pinned_archive = private_root / archive_name
        pinned_checksum = Path(f"{pinned_archive}.sha256")
        copy_regular_snapshot(checksum_path, pinned_checksum, 4096)
        try:
            expected_archive_hash = PACKAGE.parse_checksum(
                pinned_checksum, archive_name
            )
        except (OSError, UnicodeError, ValueError) as error:
            raise SessionError(f"release checksum verification failed: {error}") from error
        archive_bytes, archive_hash, _archive_mode = copy_regular_snapshot(
            archive_path, pinned_archive, MAX_BINARY_ARCHIVE_BYTES
        )
        if archive_hash != expected_archive_hash:
            raise SessionError("release archive checksum mismatch")
        try:
            files = PACKAGE.verify_archive(pinned_archive, pinned_checksum)
        except (
            AttributeError,
            KeyError,
            OSError,
            TypeError,
            UnicodeError,
            ValueError,
            json.JSONDecodeError,
        ) as error:
            raise SessionError(f"release archive verification failed: {error}") from error
        if not isinstance(files, dict):
            raise SessionError("release archive verifier returned no member snapshot")

        roots = {PurePosixPath(name).parts[0] for name in files}
        if len(roots) != 1:
            raise SessionError("verified archive has no unique payload root")
        root_name = next(iter(roots))
        metadata_name = f"{root_name}/RELEASE.json"
        try:
            metadata = parse_json(files[metadata_name])
        except (KeyError, SessionError, UnicodeError, json.JSONDecodeError) as error:
            raise SessionError("verified archive has malformed release metadata") from error
        if not isinstance(metadata, dict):
            raise SessionError("verified archive release metadata is not an object")
        if set(metadata) != {
            "commit",
            "kind",
            "schema",
            "schemaVersion",
            "signed",
            "tag",
            "target",
            "version",
        }:
            raise SessionError("verified archive release metadata is not exact")
        target = metadata.get("target")
        if metadata.get("kind") != "binaries" or target != expected_target():
            raise SessionError("release archive does not match this host target")
        version = metadata.get("version")
        try:
            PACKAGE.validate_version(version)
        except (TypeError, ValueError) as error:
            raise SessionError("release archive has a malformed version") from error
        commit = metadata.get("commit")
        if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
            raise SessionError("release archive has a malformed commit")
        if metadata.get("tag") != f"v{version}" or metadata.get("signed") is not False:
            raise SessionError("release archive metadata is internally inconsistent")

        binary_paths: dict[str, Path] = {}
        binary_evidence: dict[str, Any] = {}
        private_bin = private_root / "bin"
        private_bin.mkdir()
        suffix = ".exe" if os.name == "nt" else ""
        for name in PACKAGE.BINARIES:
            installed = SMOKE.installed_binary(bin_dir, name)
            payload_name = f"{root_name}/bin/{name}{suffix}"
            payload = files.get(payload_name)
            if payload is None:
                raise SessionError(f"verified archive omitted {name}")
            pinned_binary = private_bin / f"{name}{suffix}"
            installed_bytes, installed_hash, installed_mode = copy_regular_snapshot(
                installed, pinned_binary, MAX_INSTALLED_BINARY_BYTES
            )
            payload_hash = hashlib.sha256(payload).hexdigest()
            if installed_hash != payload_hash:
                raise SessionError(
                    f"installed {name} does not match the verified archive"
                )
            if os.name != "nt":
                os.chmod(pinned_binary, installed_mode)
            binary_paths[name] = pinned_binary
            binary_evidence[name] = {
                "bytes": installed_bytes,
                "sha256": installed_hash,
            }

        yield (
            {
                "archive": archive_name,
                "archiveBytes": archive_bytes,
                "archiveSha256": archive_hash,
                "commit": commit,
                "target": target,
                "version": version,
                "binaries": binary_evidence,
            },
            binary_paths,
        )


def prompt_observation(
    checkpoint: dict[str, str], input_fn: Callable[[str], str] = input
) -> dict[str, str]:
    """Collect one explicit pass or fail observation for a named action."""
    print(f"\n[{checkpoint['id']}]\nAction: {checkpoint['action']}\nExpected: {checkpoint['expected']}")
    while True:
        result = input_fn("Result, type PASS or FAIL: ").strip().lower()
        if result in {"pass", "fail"}:
            break
        print("Enter exactly PASS or FAIL.")
    while True:
        note = input_fn("Observed result, 1 to 500 characters: ").strip()
        if (
            1 <= len(note) <= MAX_NOTE_CHARACTERS
            and all(character.isprintable() for character in note)
        ):
            break
        print("Observation must contain 1 to 500 characters.")
    return {
        "checkpoint": checkpoint["id"],
        "input": checkpoint["input"],
        "result": result,
        "observation": note,
    }


def run_app_phase(
    app: Path,
    environment: dict[str, str],
    checkpoints: Sequence[dict[str, str]],
    input_fn: Callable[[str], str] = input,
) -> tuple[list[dict[str, str]], int]:
    """Launch the App, collect observations, and require a clean operator exit."""
    process = subprocess.Popen(
        [str(app)],
        cwd=app.parent,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        startup_deadline = time.monotonic() + APP_STARTUP_SECONDS
        while time.monotonic() < startup_deadline:
            return_code = process.poll()
            if return_code is not None:
                raise SessionError(
                    f"App exited during startup with status {return_code}"
                )
            time.sleep(0.02)
        observations = [prompt_observation(item, input_fn) for item in checkpoints]
        input_fn(
            "Close the App through its normal window or menu control, "
            "then press Enter here: "
        )
        try:
            return_code = process.wait(timeout=APP_EXIT_SECONDS)
        except subprocess.TimeoutExpired as error:
            raise SessionError("App did not close within the bounded exit window") from error
        if return_code != 0:
            raise SessionError(f"App exited with status {return_code}")
        return observations, return_code
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()


def canonical_json(value: Any) -> bytes:
    """Encode one receipt deterministically for its content identifier."""
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def attach_content_id(body: dict[str, Any]) -> dict[str, Any]:
    """Attach a deterministic, non-authenticating identifier to one receipt."""
    receipt = dict(body)
    receipt["contentId"] = hashlib.sha256(canonical_json(body)).hexdigest()
    return receipt


def persistence_value(observation: dict[str, str]) -> dict[str, int]:
    """Parse the exact Journey level and XP recorded at a lifecycle checkpoint."""
    match = PERSISTENCE_RE.fullmatch(observation["observation"])
    if match is None:
        return {"level": 0, "xp": 0}
    return {"level": int(match.group(1)), "xp": int(match.group(2))}


def validate_binary_snapshots(
    release: dict[str, Any], binary_paths: dict[str, Path]
) -> None:
    """Revalidate every private executable snapshot around each process boundary."""
    if set(binary_paths) != set(PACKAGE.BINARIES):
        raise SessionError("private binary snapshot inventory is incomplete")
    for name in PACKAGE.BINARIES:
        data = read_bounded_regular(binary_paths[name], MAX_INSTALLED_BINARY_BYTES)
        expected = release["binaries"][name]
        if len(data) != expected["bytes"] or hashlib.sha256(data).hexdigest() != expected["sha256"]:
            raise SessionError(f"private {name} snapshot changed before execution")


def collect_session(
    release: dict[str, Any],
    binary_paths: dict[str, Path],
    controller_name: str,
    controller_connection: str,
    controller_profile: str,
    input_fn: Callable[[str], str] = input,
) -> dict[str, Any]:
    """Run automated faces plus two physical App phases and identify the receipt."""
    normalized_controller_name = controller_name.strip()
    if not 3 <= len(normalized_controller_name) <= 120 or not all(
        character.isprintable() for character in normalized_controller_name
    ):
        raise SessionError("controller name must contain 3 to 120 characters")
    print("Verifying installed CLI and MCP engagement from an isolated profile.")
    validate_binary_snapshots(release, binary_paths)
    observed_version = SMOKE.run_engagement_smoke(binary_paths["numinous"].parent)
    validate_binary_snapshots(release, binary_paths)
    if observed_version != release["version"]:
        raise SessionError("installed CLI and MCP version does not match the archive")

    with tempfile.TemporaryDirectory(prefix="numinous-input-session-") as temporary:
        state_root = Path(temporary) / "profile"
        environment = SMOKE.isolated_environment(state_root)
        validate_binary_snapshots(release, binary_paths)
        first, first_exit = run_app_phase(
            binary_paths["numinous-app"], environment, CHECKPOINTS[:-1], input_fn
        )
        validate_binary_snapshots(release, binary_paths)
        second, second_exit = run_app_phase(
            binary_paths["numinous-app"], environment, CHECKPOINTS[-1:], input_fn
        )
        validate_binary_snapshots(release, binary_paths)

    observations = first + second
    before_exit = persistence_value(first[-1])
    after_restart = persistence_value(second[-1])
    if before_exit["xp"] <= 0:
        first[-1]["result"] = "fail"
    if after_restart != before_exit:
        second[-1]["result"] = "fail"
    passed = all(item["result"] == "pass" for item in observations)
    host_system = platform.system().lower()
    body: dict[str, Any] = {
        "schema": SCHEMA,
        "schemaVersion": SCHEMA_VERSION,
        "recordedAt": datetime.now(timezone.utc).isoformat(timespec="seconds").replace(
            "+00:00", "Z"
        ),
        "result": "pass" if passed else "fail",
        "release": release,
        "host": {
            "system": host_system,
            "systemRelease": platform.release(),
            "machine": platform.machine().lower(),
        },
        "controller": {
            "name": normalized_controller_name,
            "connection": controller_connection,
            "legendProfile": controller_profile,
        },
        "automated": {
            "archiveVerified": True,
            "installedPayloadMatch": True,
            "cliMcpEngagement": True,
            "cliMcpVersion": observed_version,
        },
        "appLifecycle": {
            "firstLaunchExitCode": first_exit,
            "secondLaunchExitCode": second_exit,
        },
        "persistence": {
            "beforeExit": before_exit,
            "afterRestart": after_restart,
        },
        "observations": observations,
        "limitations": list(LIMITATIONS),
    }
    return attach_content_id(body)


def require_exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    """Reject missing and unknown receipt fields at one object boundary."""
    if set(value) != expected:
        raise SessionError(f"{label} fields do not match the receipt schema")


def validate_receipt(receipt: Any) -> None:
    """Validate a complete receipt and the truth of its aggregate result."""
    if not isinstance(receipt, dict):
        raise SessionError("receipt root is not an object")
    require_exact_keys(
        receipt,
        {
            "schema",
            "schemaVersion",
            "contentId",
            "recordedAt",
            "result",
            "release",
            "host",
            "controller",
            "automated",
            "appLifecycle",
            "persistence",
            "observations",
            "limitations",
        },
        "receipt",
    )
    if (
        receipt["schema"] != SCHEMA
        or type(receipt["schemaVersion"]) is not int
        or receipt["schemaVersion"] != SCHEMA_VERSION
    ):
        raise SessionError("receipt schema is unsupported")
    content_id = receipt["contentId"]
    if not isinstance(content_id, str) or SHA256_RE.fullmatch(content_id) is None:
        raise SessionError("receipt content id is malformed")
    unsealed = {name: value for name, value in receipt.items() if name != "contentId"}
    if hashlib.sha256(canonical_json(unsealed)).hexdigest() != content_id:
        raise SessionError("receipt content does not match its content id")
    if not isinstance(receipt["result"], str) or receipt["result"] not in {"pass", "fail"}:
        raise SessionError("receipt result is malformed")
    try:
        datetime.strptime(receipt["recordedAt"], "%Y-%m-%dT%H:%M:%SZ")
    except (TypeError, ValueError) as error:
        raise SessionError("receipt time is not canonical UTC") from error

    release = receipt["release"]
    if not isinstance(release, dict):
        raise SessionError("release evidence is not an object")
    require_exact_keys(
        release,
        {"archive", "archiveBytes", "archiveSha256", "commit", "target", "version", "binaries"},
        "release",
    )
    if not isinstance(release["archive"], str) or not release["archive"] or Path(release["archive"]).name != release["archive"]:
        raise SessionError("release archive name is malformed")
    if type(release["archiveBytes"]) is not int or release["archiveBytes"] <= 0:
        raise SessionError("release archive size is malformed")
    if not isinstance(release["archiveSha256"], str) or SHA256_RE.fullmatch(release["archiveSha256"]) is None:
        raise SessionError("release archive hash is malformed")
    if not isinstance(release["target"], str) or release["target"] not in PACKAGE.TARGETS:
        raise SessionError("release target is unsupported")
    if not isinstance(release["version"], str):
        raise SessionError("release version is malformed")
    try:
        PACKAGE.validate_version(release["version"])
    except ValueError as error:
        raise SessionError("release version is malformed") from error
    if not isinstance(release["commit"], str) or re.fullmatch(r"[0-9a-f]{40}", release["commit"]) is None:
        raise SessionError("release commit is malformed")
    binaries = release["binaries"]
    if not isinstance(binaries, dict) or set(binaries) != set(PACKAGE.BINARIES):
        raise SessionError("release binary inventory is incomplete")
    for name, evidence in binaries.items():
        if not isinstance(evidence, dict):
            raise SessionError(f"{name} evidence is not an object")
        require_exact_keys(evidence, {"bytes", "sha256"}, f"{name} evidence")
        if type(evidence["bytes"]) is not int or evidence["bytes"] <= 0:
            raise SessionError(f"{name} size is malformed")
        if not isinstance(evidence["sha256"], str) or SHA256_RE.fullmatch(evidence["sha256"]) is None:
            raise SessionError(f"{name} hash is malformed")

    host = receipt["host"]
    if not isinstance(host, dict):
        raise SessionError("host evidence is not an object")
    require_exact_keys(host, {"system", "systemRelease", "machine"}, "host")
    if not all(
        isinstance(host.get(name), str)
        and host[name]
        and all(character.isprintable() for character in host[name])
        for name in host
    ):
        raise SessionError("host evidence contains an empty field")
    try:
        host_target = target_for_host(host["system"], host["machine"])
    except SessionError as error:
        raise SessionError("host evidence names an unsupported release target") from error
    if release["target"] != host_target:
        raise SessionError("host evidence and release target disagree")

    controller = receipt["controller"]
    if not isinstance(controller, dict):
        raise SessionError("controller evidence is not an object")
    require_exact_keys(controller, {"name", "connection", "legendProfile"}, "controller")
    if (
        not isinstance(controller["name"], str)
        or not 3 <= len(controller["name"]) <= 120
        or not all(character.isprintable() for character in controller["name"])
    ):
        raise SessionError("controller name is malformed")
    if not isinstance(controller["connection"], str) or controller["connection"] not in {"wired", "bluetooth", "wireless"}:
        raise SessionError("controller connection is unsupported")
    if not isinstance(controller["legendProfile"], str) or controller["legendProfile"] not in {"xbox", "playstation", "generic"}:
        raise SessionError("controller legend profile is unsupported")
    if controller["name"] != controller["name"].strip():
        raise SessionError("controller name is not canonical")

    automated = receipt["automated"]
    if not isinstance(automated, dict):
        raise SessionError("automated evidence is not an object")
    require_exact_keys(
        automated,
        {
            "archiveVerified",
            "installedPayloadMatch",
            "cliMcpEngagement",
            "cliMcpVersion",
        },
        "automated evidence",
    )
    if any(
        automated[name] is not True
        for name in ("archiveVerified", "installedPayloadMatch", "cliMcpEngagement")
    ):
        raise SessionError("automated evidence is incomplete")
    if (
        not isinstance(automated["cliMcpVersion"], str)
        or automated["cliMcpVersion"] != release["version"]
    ):
        raise SessionError("automated binary version disagrees with the release")

    lifecycle = receipt["appLifecycle"]
    if not isinstance(lifecycle, dict):
        raise SessionError("App lifecycle evidence is not an object")
    require_exact_keys(
        lifecycle, {"firstLaunchExitCode", "secondLaunchExitCode"}, "App lifecycle"
    )
    if any(type(value) is not int or value != 0 for value in lifecycle.values()):
        raise SessionError("App lifecycle did not complete cleanly")

    persistence = receipt["persistence"]
    if not isinstance(persistence, dict):
        raise SessionError("persistence evidence is not an object")
    require_exact_keys(persistence, {"beforeExit", "afterRestart"}, "persistence")
    for label, value in persistence.items():
        if not isinstance(value, dict):
            raise SessionError(f"persistence {label} is not an object")
        require_exact_keys(value, {"level", "xp"}, f"persistence {label}")
        if any(type(number) is not int or number < 0 for number in value.values()):
            raise SessionError(f"persistence {label} values are malformed")

    observations = receipt["observations"]
    if not isinstance(observations, list) or len(observations) != len(CHECKPOINTS):
        raise SessionError("receipt does not contain every checkpoint")
    expected_ids = [item["id"] for item in CHECKPOINTS]
    observed_ids: list[str] = []
    for index, observation in enumerate(observations):
        if not isinstance(observation, dict):
            raise SessionError("checkpoint observation is not an object")
        require_exact_keys(
            observation, {"checkpoint", "input", "result", "observation"}, "checkpoint"
        )
        if not isinstance(observation["checkpoint"], str):
            raise SessionError("checkpoint identity is malformed")
        observed_ids.append(observation["checkpoint"])
        if not isinstance(observation["input"], str):
            raise SessionError("checkpoint input family is malformed")
        if observation["input"] != CHECKPOINTS[index]["input"]:
            raise SessionError("checkpoint input family is inconsistent")
        if not isinstance(observation["result"], str) or observation["result"] not in {"pass", "fail"}:
            raise SessionError("checkpoint result is malformed")
        note = observation["observation"]
        if (
            not isinstance(note, str)
            or not 1 <= len(note) <= MAX_NOTE_CHARACTERS
            or not all(character.isprintable() for character in note)
        ):
            raise SessionError("checkpoint observation note is malformed")
    if observed_ids != expected_ids:
        raise SessionError("checkpoint order or identity is inconsistent")

    limitations = receipt["limitations"]
    if limitations != list(LIMITATIONS):
        raise SessionError("receipt limitations are incomplete")
    aggregate = "pass" if all(item["result"] == "pass" for item in observations) else "fail"
    if receipt["result"] != aggregate:
        raise SessionError("receipt result disagrees with its checkpoints")
    persistence_observations = (
        persistence_value(observations[-2]),
        persistence_value(observations[-1]),
    )
    if persistence_observations != (
        persistence["beforeExit"],
        persistence["afterRestart"],
    ):
        raise SessionError("persistence values disagree with lifecycle observations")
    if receipt["result"] == "pass" and (
        persistence["beforeExit"]["xp"] <= 0
        or persistence["beforeExit"] != persistence["afterRestart"]
    ):
        raise SessionError("passed receipt has no verified persistence mutation")


def read_receipt(path: Path) -> dict[str, Any]:
    """Read one bounded ordinary receipt without following a final symlink."""
    try:
        value = parse_json(read_bounded_regular(path, MAX_RECEIPT_BYTES))
    except (OSError, SessionError, UnicodeError, json.JSONDecodeError) as error:
        raise SessionError(f"receipt could not be read: {error}") from error
    validate_receipt(value)
    return value


def validate_matrix(receipts: Sequence[dict[str, Any]]) -> dict[str, Any]:
    """Require passed, unique sessions across every release target and legend profile."""
    if not receipts:
        raise SessionError("input session matrix contains no receipts")
    content_ids: set[str] = set()
    targets: set[str] = set()
    profiles: set[str] = set()
    controllers: dict[str, str] = {}
    profile_by_controller: dict[str, str] = {}
    versions: set[str] = set()
    commits: set[str] = set()
    for receipt in receipts:
        validate_receipt(receipt)
        if receipt["result"] != "pass":
            raise SessionError("input session matrix contains a failed receipt")
        content_id = receipt["contentId"]
        if content_id in content_ids:
            raise SessionError("input session matrix contains a duplicate receipt")
        content_ids.add(content_id)
        targets.add(receipt["release"]["target"])
        versions.add(receipt["release"]["version"])
        commits.add(receipt["release"]["commit"])
        profile = receipt["controller"]["legendProfile"]
        profiles.add(profile)
        controller_name = receipt["controller"]["name"]
        controller_key = controller_name.casefold()
        prior_profile = profile_by_controller.setdefault(controller_key, profile)
        if prior_profile != profile:
            raise SessionError(
                "input session matrix assigns one controller model to multiple profiles"
            )
        controllers.setdefault(controller_key, controller_name)
    if len(versions) != 1 or len(commits) != 1:
        raise SessionError("input session matrix mixes release identities")
    missing_targets = sorted(MATRIX_TARGETS - targets)
    if missing_targets:
        raise SessionError(
            f"input session matrix is missing release target {missing_targets[0]}"
        )
    missing_profiles = sorted(MATRIX_CONTROLLER_PROFILES - profiles)
    if missing_profiles:
        raise SessionError(
            f"input session matrix is missing controller profile {missing_profiles[0]}"
        )
    if len(controllers) < MATRIX_MIN_CONTROLLER_MODELS:
        raise SessionError(
            f"input session matrix requires {MATRIX_MIN_CONTROLLER_MODELS} distinct controller models"
        )
    return {
        "schema": "numinous.physical-input-matrix",
        "schemaVersion": 1,
        "result": "pass",
        "releaseVersion": next(iter(versions)),
        "releaseCommit": next(iter(commits)),
        "sessionCount": len(receipts),
        "receiptContentIds": sorted(content_ids),
        "releaseTargets": sorted(targets),
        "controllerProfiles": sorted(profiles),
        "controllerModels": sorted(controllers.values()),
    }


def path_is_within(path: str, root: str) -> bool:
    """Compare two normalized absolute paths without prefix ambiguity."""
    try:
        return os.path.commonpath((path, root)) == root
    except ValueError:
        return False


def reject_link_like_ancestry(root: Path, destination: Path) -> None:
    """Reject existing symlink or reparse ancestors from root through destination."""
    try:
        relative = destination.relative_to(root)
    except ValueError as error:
        raise SessionError("receipt output escaped the logs directory") from error
    candidates = [ROOT, root]
    current = root
    for component in relative.parts:
        current /= component
        candidates.append(current)
    for candidate in candidates:
        if is_link_like(candidate):
            raise SessionError(
                f"receipt output ancestry is link-like: {candidate.name}"
            )


def final_path_for_descriptor(descriptor: int, fallback: Path) -> str:
    """Return the operating system's final path for one newly opened receipt."""
    if os.name != "nt":
        return os.path.normcase(str(fallback.resolve(strict=True)))
    import ctypes
    import msvcrt

    get_final_path = ctypes.windll.kernel32.GetFinalPathNameByHandleW
    get_final_path.argtypes = (
        ctypes.c_void_p,
        ctypes.c_wchar_p,
        ctypes.c_uint32,
        ctypes.c_uint32,
    )
    get_final_path.restype = ctypes.c_uint32
    buffer = ctypes.create_unicode_buffer(32_768)
    handle = msvcrt.get_osfhandle(descriptor)
    length = get_final_path(handle, buffer, len(buffer), 0)
    if length == 0 or length >= len(buffer):
        raise SessionError("receipt final path could not be resolved")
    value = buffer.value
    if value.startswith("\\\\?\\UNC\\"):
        value = "\\\\" + value[8:]
    elif value.startswith("\\\\?\\"):
        value = value[4:]
    return os.path.normcase(os.path.abspath(value))


def write_receipt(receipt: dict[str, Any], output_dir: Path = LOG_ROOT) -> Path:
    """Write one validated receipt exclusively inside the ignored logs tree."""
    validate_receipt(receipt)
    logs = Path(os.path.abspath(ROOT / "logs"))
    directory = Path(os.path.abspath(output_dir))
    normalized_logs = os.path.normcase(str(logs))
    normalized_directory = os.path.normcase(str(directory))
    if not path_is_within(normalized_directory, normalized_logs):
        raise SessionError("receipt output must stay inside the repository logs directory")
    reject_link_like_ancestry(logs, directory)
    directory.mkdir(parents=True, exist_ok=True)
    reject_link_like_ancestry(logs, directory)
    filename = f"{receipt['recordedAt'].replace(':', '')}-{receipt['release']['target']}-{receipt['contentId'][:12]}.json"
    destination = directory / filename
    encoded = json.dumps(receipt, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
    encoded_bytes = encoded.encode("utf-8")
    if len(encoded_bytes) > MAX_RECEIPT_BYTES:
        raise SessionError("receipt exceeds the size limit")
    try:
        with destination.open("xb") as output:
            final_path = final_path_for_descriptor(output.fileno(), destination)
            if not path_is_within(final_path, normalized_logs):
                raise SessionError("opened receipt escaped the logs directory")
            output.write(encoded_bytes)
            output.flush()
            os.fsync(output.fileno())
    except FileExistsError as error:
        raise SessionError("receipt destination already exists") from error
    return destination


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    run = subparsers.add_parser("run", help="run one physical release session")
    run.add_argument("--release-archive", type=Path, required=True)
    run.add_argument("--release-checksum", type=Path)
    default_install_root = Path(
        os.environ.get("NUMINOUS_HOME", str(Path.home() / ".numinous"))
    )
    run.add_argument("--bin-dir", type=Path, default=default_install_root / "bin")
    run.add_argument("--controller-name", required=True)
    run.add_argument(
        "--controller-connection",
        choices=("wired", "bluetooth", "wireless"),
        required=True,
    )
    run.add_argument(
        "--controller-profile",
        choices=("xbox", "playstation", "generic"),
        required=True,
    )
    run.add_argument("--output-dir", type=Path, default=LOG_ROOT)
    validate = subparsers.add_parser("validate", help="validate one receipt")
    validate.add_argument("receipt", type=Path)
    matrix = subparsers.add_parser(
        "matrix", help="validate complete target and controller coverage"
    )
    matrix.add_argument("receipts", type=Path, nargs="+")
    subparsers.add_parser("checkpoints", help="print the physical checkpoint inventory")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "validate":
            receipt = read_receipt(args.receipt)
            print(
                f"input session receipt valid: {receipt['result']} "
                f"{receipt['contentId']}"
            )
            return 0
        if args.command == "checkpoints":
            for checkpoint in CHECKPOINTS:
                print(f"{checkpoint['id']}: {checkpoint['action']}")
            return 0
        if args.command == "matrix":
            matrix = validate_matrix([read_receipt(path) for path in args.receipts])
            print(json.dumps(matrix, indent=2, sort_keys=True))
            return 0
        checksum = args.release_checksum or Path(f"{args.release_archive}.sha256")
        with release_install_evidence(
            args.release_archive, checksum, args.bin_dir
        ) as (release, binaries):
            receipt = collect_session(
                release,
                binaries,
                args.controller_name,
                args.controller_connection,
                args.controller_profile,
            )
        destination = write_receipt(receipt, args.output_dir)
        validate_receipt(receipt)
        print(f"input session {receipt['result']}: {destination}")
        return 0 if receipt["result"] == "pass" else 1
    except (SessionError, OSError, UnicodeError, SMOKE.SmokeError) as error:
        print(f"input hardware session failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
