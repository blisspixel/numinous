#!/usr/bin/env python3
"""Machine acceptance for the install, play, uninstall roundtrip (0.6-am).

The 0.6 exit asks that a machine installs, plays, saves state, and uninstalls
cleanly. The release workflow already covers install and play. This covers the
end of that sentence, which nothing tested: that uninstalling removes the
program and keeps the player.

The uninstaller prints "Your play history stays" as it finishes. That is a
promise made to someone deciding whether it is safe to remove this, and until
now it was only a sentence. Here it is checked: every player-owned file is
hashed before the uninstall and must still be present and byte-identical after,
while the install root must be gone.

Run from a clone with a packaged archive:

    python scripts/uninstall-roundtrip.py \\
        --release-archive dist/numinous-v0.2.0-alpha.4-<target>.zip

This is machine evidence for one platform per run, the one it executes on. It
says nothing about a genuinely clean machine, about signing, or about the
window opening, which are separate 0.6 gates.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "uninstall-roundtrip"

# Files the player owns. The uninstaller names these as the ones it keeps, so
# these are exactly what the promise covers.
PLAYER_STATE = (
    ".numinous-journey",
    ".numinous-scores",
    ".numinous-cairn",
)

# A room to play so there is state worth keeping. Times Tables is a flagship
# and its render is deterministic.
PLAY_ROOM = "times-tables"


class RoundtripError(RuntimeError):
    """A step of the roundtrip did not hold."""


def native_tool_env(env: dict[str, str]) -> dict[str, str]:
    r"""Make sure the platform's own archive tools win on PATH.

    The installers shell out to `tar`. Run from a Unix-like shell on Windows,
    PATH can put a GNU tar first, and GNU tar reads `C:\...` as a remote
    host and fails with "Cannot connect to C:". Putting System32 in front
    resolves `tar` to the one Windows ships, which understands the paths the
    installer hands it. Everywhere else this is a no-op.
    """
    if platform.system() != "Windows":
        return env
    system32 = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32"
    patched = dict(env)
    patched["PATH"] = str(system32) + os.pathsep + patched.get("PATH", "")
    return patched


def run(command: list[str], env: dict[str, str], step: str) -> str:
    result = subprocess.run(
        command,
        env=native_tool_env(env),
        capture_output=True,
        text=True,
        cwd=ROOT,
        timeout=900,
    )
    if result.returncode != 0:
        raise RoundtripError(
            f"{step} failed with exit {result.returncode}\n"
            f"stdout:\n{result.stdout[-2000:]}\nstderr:\n{result.stderr[-2000:]}"
        )
    return result.stdout


def build_local_soundtrack(output_dir: Path) -> tuple[Path, Path, Path]:
    """Package a one-track soundtrack so the install needs no network.

    Without a soundtrack on disk the installer downloads the shipped one, which
    would make this gate depend on the network and on a published release. CI's
    release smoke solves it the same way, with a single track standing in for
    the full set: what is under test here is the install and uninstall, not the
    music.
    """
    radio = output_dir / "radio"
    radio.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "assets" / "radio" / "ASSET-LICENSE.txt", radio)
    tracks = sorted((ROOT / "assets" / "radio").glob("*.mp3"))
    if not tracks:
        raise RoundtripError("no radio track to package under assets/radio")
    shutil.copy2(tracks[0], radio)
    subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "package-release.py"),
         "--kind", "soundtrack", "--target", "all",
         "--radio-dir", str(radio), "--output-dir", str(output_dir)],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    archive = next(output_dir.glob("numinous-v*-soundtrack.tar.gz"))
    return archive, Path(f"{archive}.sha256"), Path(f"{archive}.content.sha256")


def installer_command(
    archive: Path,
    checksum: Path,
    tag: str,
    soundtrack: tuple[Path, Path, Path],
    uninstall: bool,
) -> list[str]:
    """The platform's installer invocation.

    Windows ships a PowerShell installer and everything else a shell one, and
    they take differently spelled but equivalent switches.
    """
    track, track_sum, track_content = soundtrack
    if platform.system() == "Windows":
        base = ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                str(ROOT / "scripts" / "install.ps1"), "-NoModifyPath"]
        if uninstall:
            return base + ["-Uninstall"]
        return base + [
            "-ReleaseArchive", str(archive),
            "-ReleaseChecksum", str(checksum),
            "-SoundtrackArchive", str(track),
            "-SoundtrackChecksum", str(track_sum),
            "-SoundtrackContentChecksum", str(track_content),
            "-ReleaseTag", tag,
        ]
    base = ["bash", str(ROOT / "scripts" / "install.sh"), "--no-modify-path"]
    if uninstall:
        return base + ["--uninstall"]
    return base + [
        "--release-archive", str(archive),
        "--release-checksum", str(checksum),
        "--soundtrack-archive", str(track),
        "--soundtrack-checksum", str(track_sum),
        "--soundtrack-content-checksum", str(track_content),
        "--release-tag", tag,
    ]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def player_state(profile: Path) -> dict[str, str]:
    """Hash every player-owned file that exists."""
    return {
        name: digest(profile / name)
        for name in PLAYER_STATE
        if (profile / name).is_file()
    }


def installed_cli(install_root: Path) -> Path:
    for name in ("numinous.exe", "numinous"):
        candidate = install_root / "bin" / name
        if candidate.is_file():
            return candidate
    raise RoundtripError(f"no installed CLI under {install_root / 'bin'}")


def roundtrip(archive: Path, checksum: Path, tag: str) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []
    workspace = Path(tempfile.mkdtemp(prefix="numinous-uninstall-roundtrip-"))
    install_root = workspace / "install"
    profile = workspace / "profile"
    profile.mkdir(parents=True)
    soundtrack = build_local_soundtrack(workspace / "soundtrack-src")

    env = dict(os.environ)
    env["NUMINOUS_HOME"] = str(install_root)
    # Isolate the player profile so the roundtrip cannot read or write the
    # history of whoever is running it.
    env["HOME"] = str(profile)
    env["USERPROFILE"] = str(profile)
    env.pop("NUMINOUS_JOURNEY", None)
    env.pop("NUMINOUS_SCORES", None)

    try:
        run(
            installer_command(archive, checksum, tag, soundtrack, uninstall=False),
            env,
            "install",
        )
        checks.append({
            "name": "install",
            "passed": install_root.is_dir(),
            "detail": f"install root present at {install_root}",
        })

        cli = installed_cli(install_root)
        run([str(cli), "render", PLAY_ROOM, "--width", "40", "--height", "20"], env, "play")
        before = player_state(profile)
        checks.append({
            "name": "play saves state",
            "passed": bool(before),
            "detail": f"player files after play: {sorted(before) or 'none, so nothing was saved'}",
        })

        run(
            installer_command(archive, checksum, tag, soundtrack, uninstall=True),
            env,
            "uninstall",
        )

        checks.append({
            "name": "uninstall removes the program",
            "passed": not install_root.exists(),
            "detail": f"install root {'gone' if not install_root.exists() else 'still present'}",
        })

        after = player_state(profile)
        missing = sorted(set(before) - set(after))
        changed = sorted(name for name in set(before) & set(after) if before[name] != after[name])
        reasons = []
        if missing:
            reasons.append(f"uninstall deleted player state: {', '.join(missing)}")
        if changed:
            reasons.append(f"uninstall altered player state: {', '.join(changed)}")
        checks.append({
            "name": "uninstall keeps the player",
            "passed": not reasons,
            "detail": "; ".join(reasons)
            or f"{len(after)} player file(s) survived byte-identical",
        })
    finally:
        shutil.rmtree(workspace, ignore_errors=True)
    return checks


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-archive", required=True, type=Path)
    parser.add_argument(
        "--release-checksum",
        type=Path,
        help="defaults to the archive path plus .sha256",
    )
    parser.add_argument("--release-tag", help="defaults to v plus the packaged version")
    args = parser.parse_args(argv)

    archive = args.release_archive
    checksum = args.release_checksum or Path(str(archive) + ".sha256")
    tag = args.release_tag
    if tag is None:
        version = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "package-release.py"), "--print-version"],
            capture_output=True, text=True, cwd=ROOT, check=True,
        ).stdout.strip()
        tag = f"v{version}"

    OUT.mkdir(parents=True, exist_ok=True)
    try:
        checks = roundtrip(archive, checksum, tag)
    except RoundtripError as error:
        checks = [{"name": "roundtrip", "passed": False, "detail": str(error)}]

    failed = [check for check in checks if not check["passed"]]
    summary = {
        "suite": "uninstall-roundtrip",
        "passed": not failed,
        "platform": platform.system(),
        "check_count": len(checks),
        "failed_count": len(failed),
        "results": checks,
        "evidence_class": "agent-machine",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{summary['check_count'] - summary['failed_count']}/{summary['check_count']} PASS")
    for check in checks:
        print(f"  {'PASS' if check['passed'] else 'FAIL'}  {check['name']}: {check['detail']}")
    print("--- summary.json ---")
    print(json.dumps(
        {k: summary[k] for k in ("suite", "passed", "check_count", "failed_count", "platform")},
        sort_keys=True,
    ))
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
