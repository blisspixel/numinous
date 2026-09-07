#!/usr/bin/env python3
"""Machine acceptance for a genuinely clean machine (0.6-am).

The 0.6 exit asks that a clean machine on each supported system installs,
launches, plays a flagship with sound, saves state, and uninstalls cleanly from
a signed or otherwise verifiable artifact. Two gates already cover most of that
sentence, and both say in their own words that they do not cover this part.
`uninstall-roundtrip.py` ends with "It says nothing about a genuinely clean
machine". The release workflow's install smoke runs on the same runner that
built the archive minutes earlier, which is convenient and is not evidence: a
machine cannot vouch for an artifact it produced.

So this gate begins where those stop. It refuses to run on a machine holding
build output for the version under test, downloads the published archive rather
than making one, proves the archive's integrity before anything is unpacked,
and only then hands the verified file to the existing install, play, and
uninstall gates. What is new here is not the playing. It is the provenance of
the thing being played and the emptiness of the machine playing it.

"Otherwise verifiable" is the half of the exit this can honestly reach. The
artifacts are not platform-signed or notarized. They carry a SHA-256 sidecar and
a keyless build-provenance attestation bound to the tag commit, and both are
checked here before install. That is verifiable provenance, not code signing,
and the receipt says so rather than letting a reader assume the stronger claim.

Run against a published release:

    python scripts/clean-machine-release.py --tag v0.4.0-alpha.23

Or against an already-downloaded set, for a machine with no network:

    python scripts/clean-machine-release.py --tag v0.4.0-alpha.23 \\
        --archive-dir .agent/releases/v0.4.0-alpha.23 --skip-download

This proves one platform per run, the one it executes on. It says nothing about
platform signing, notarization, the window opening, controllers, or whether any
of it is enjoyable. Those are separate gates and separate kinds of evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Final, NamedTuple

ROOT: Final[Path] = Path(__file__).resolve().parent.parent
OUT: Final[Path] = ROOT / ".agent" / "tester-cohort" / "clean-machine-release"

REPOSITORY: Final[str] = "blisspixel/numinous"
ATTEST_WORKFLOW: Final[str] = (
    f"{REPOSITORY}/.github/workflows/release-attest.yml"
)
PROVENANCE_PREDICATE: Final[str] = "https://slsa.dev/provenance/v1"

# Which archive this platform installs from, keyed by (system, machine family).
TARGETS: Final[dict[tuple[str, str], tuple[str, str]]] = {
    ("Windows", "x86_64"): ("x86_64-pc-windows-msvc", "zip"),
    ("Linux", "x86_64"): ("x86_64-unknown-linux-gnu", "tar.gz"),
    ("Darwin", "x86_64"): ("x86_64-apple-darwin", "tar.gz"),
    ("Darwin", "arm64"): ("aarch64-apple-darwin", "tar.gz"),
}

TAG_PATTERN: Final[re.Pattern[str]] = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
SHA256_HEX: Final[re.Pattern[str]] = re.compile(r"^[0-9a-f]{64}$")

# Build output that would mean this machine made what it is about to judge.
BUILD_OUTPUT_HINTS: Final[tuple[str, ...]] = ("numinous", "numinous-app", "numinous-mcp")

COMMAND_TIMEOUT_SECONDS: Final[float] = 900.0
READ_CHUNK_BYTES: Final[int] = 1024 * 1024


class GateError(RuntimeError):
    """The clean-machine gate could not reach a verdict it could stand behind."""


class Check(NamedTuple):
    """One recorded judgment, with the detail a reader needs to act on it."""

    name: str
    passed: bool
    detail: str

    def as_json(self) -> dict[str, Any]:
        return {"name": self.name, "passed": self.passed, "detail": self.detail}


def run(command: list[str], purpose: str, cwd: Path | None = None) -> str:
    """Run one command, or explain what was being attempted when it failed."""
    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=COMMAND_TIMEOUT_SECONDS,
            cwd=None if cwd is None else str(cwd),
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise GateError(f"could not {purpose}: {error}") from error
    if completed.returncode != 0:
        detail = (completed.stderr or completed.stdout or "").strip()
        raise GateError(f"could not {purpose}: {detail[-600:]}")
    return completed.stdout


def platform_target() -> tuple[str, str]:
    """The archive target and extension this machine installs from."""
    system = platform.system()
    machine = platform.machine().lower()
    family = "arm64" if machine in {"arm64", "aarch64"} else "x86_64"
    target = TARGETS.get((system, family))
    if target is None:
        raise GateError(f"no published archive for {system} {machine}")
    return target


def file_digest(path: Path) -> str:
    """SHA-256 of a file, read in bounded chunks rather than all at once."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            while chunk := handle.read(READ_CHUNK_BYTES):
                digest.update(chunk)
    except OSError as error:
        raise GateError(f"could not read {path.name}: {error}") from error
    return digest.hexdigest()


def sidecar_digest(path: Path) -> str:
    """The digest a `.sha256` sidecar declares for its archive."""
    try:
        text = path.read_text(encoding="ascii")
    except (OSError, UnicodeError) as error:
        raise GateError(f"could not read {path.name}: {error}") from error
    candidate = text.strip().split(" ", 1)[0]
    if not SHA256_HEX.fullmatch(candidate):
        raise GateError(f"{path.name} does not declare a SHA-256 digest")
    return candidate


def find_build_output(root: Path = ROOT) -> list[Path]:
    """Local build products that would make this machine the artifact's maker.

    A machine that compiled these binaries is not a clean machine for them, no
    matter how carefully the rest of the run is staged. This looks for the
    products themselves rather than for a target directory, because a directory
    left by an unrelated crate says nothing.

    The root is a parameter so this can be judged against a fixture tree without
    reaching into module state, which a test should never have to do.
    """
    found: list[Path] = []
    for profile in ("debug", "release"):
        directory = root / "target" / profile
        if not directory.is_dir():
            continue
        for stem in BUILD_OUTPUT_HINTS:
            for name in (stem, f"{stem}.exe"):
                candidate = directory / name
                if candidate.is_file():
                    found.append(candidate)
    return found


def resolve_tag(requested: str | None) -> str:
    """The release tag under test, either given or the newest published one."""
    if requested is not None:
        if not TAG_PATTERN.fullmatch(requested):
            raise GateError(f"{requested!r} is not a release tag")
        return requested
    latest = run(
        ["gh", "release", "view", "--repo", REPOSITORY, "--json", "tagName", "--jq", ".tagName"],
        "ask for the newest published release",
    ).strip()
    if not TAG_PATTERN.fullmatch(latest):
        raise GateError(f"the newest release is not a release tag: {latest!r}")
    return latest


def soundtrack_paths(directory: Path, tag: str) -> tuple[Path, Path, Path]:
    """The published soundtrack archive and the two checksums that guard it."""
    archive = directory / f"numinous-{tag}-soundtrack.tar.gz"
    return archive, Path(f"{archive}.sha256"), Path(f"{archive}.content.sha256")


def download(tag: str, wanted: list[Path]) -> None:
    """Fetch exactly the files this gate judges, and nothing else."""
    directory = wanted[0].parent
    directory.mkdir(parents=True, exist_ok=True)
    for existing in wanted:
        if existing.exists():
            existing.unlink()
    patterns: list[str] = []
    for path in wanted:
        patterns.extend(["--pattern", path.name])
    run(
        ["gh", "release", "download", tag, "--repo", REPOSITORY, "--dir", str(directory), *patterns],
        f"download the {tag} artifacts",
    )


def verify_integrity(archive: Path, checksum: Path) -> list[Check]:
    """Prove the downloaded bytes before anything unpacks them."""
    declared = sidecar_digest(checksum)
    actual = file_digest(archive)
    return [
        Check(
            "archive matches its published checksum",
            declared == actual,
            f"sidecar {declared[:16]}..., computed {actual[:16]}...",
        )
    ]


def verify_provenance(tag: str, archive: Path, revision: str) -> Check:
    """Check the keyless build-provenance attestation bound to the tag commit.

    This is the "otherwise verifiable" half of the exit. It is provenance, not
    code signing, and the receipt keeps that distinction rather than letting a
    reader promote it.
    """
    try:
        run(
            [
                "gh", "attestation", "verify", str(archive),
                "--predicate-type", PROVENANCE_PREDICATE,
                "--repo", REPOSITORY,
                "--source-ref", f"refs/tags/{tag}",
                "--source-digest", revision,
                "--signer-workflow", ATTEST_WORKFLOW,
                "--signer-digest", revision,
                "--deny-self-hosted-runners",
            ],
            f"verify build provenance for {archive.name}",
        )
    except GateError as error:
        return Check("build provenance is bound to the tag commit", False, str(error))
    return Check(
        "build provenance is bound to the tag commit",
        True,
        f"keyless attestation for {revision[:12]} from the tag-only signer workflow",
    )


def tag_revision(tag: str) -> str:
    """The commit the published release was built from, as GitHub records it."""
    target = run(
        [
            "gh", "api", f"repos/{REPOSITORY}/git/ref/tags/{tag}",
            "--jq", ".object.sha + \" \" + .object.type",
        ],
        f"resolve {tag}",
    ).split()
    if len(target) != 2:
        raise GateError(f"{tag} did not resolve to an object")
    sha, kind = target
    if kind == "commit":
        return sha
    peeled = run(
        [
            "gh", "api", f"repos/{REPOSITORY}/git/tags/{sha}",
            "--jq", ".object.sha",
        ],
        f"peel the annotated tag {tag}",
    ).strip()
    if not re.fullmatch(r"[0-9a-f]{40}", peeled):
        raise GateError(f"{tag} did not peel to a commit")
    return peeled


def delegate(script: str, arguments: list[str], name: str, detail: str) -> Check:
    """Hand the verified archive to a gate that already knows how to judge it."""
    try:
        run([sys.executable, str(ROOT / "scripts" / script), *arguments], f"run {script}")
    except GateError as error:
        return Check(name, False, str(error))
    return Check(name, True, detail)


def gate(tag: str, archive: Path, checksum: Path, skip_download: bool) -> list[Check]:
    """Every judgment this run makes, in the order a reader should read them."""
    checks: list[Check] = []

    built_here = find_build_output()
    checks.append(
        Check(
            "this machine did not build what it is judging",
            not built_here,
            "no local build output for the installed faces"
            if not built_here
            else "found "
            + ", ".join(str(path.relative_to(ROOT)) for path in built_here[:4])
            + "; a machine cannot vouch for an artifact it produced",
        )
    )

    track, track_sum, track_content = soundtrack_paths(archive.parent, tag)
    wanted = [archive, checksum, track, track_sum, track_content]
    if not skip_download:
        download(tag, wanted)
    for required in wanted:
        if not required.is_file():
            raise GateError(f"{required.name} is not present")
    checks.append(
        Check(
            "the artifact came from the published release",
            True,
            f"{archive.name} and the {tag} soundtrack downloaded from the release"
            if not skip_download
            else f"{archive.name} and the {tag} soundtrack supplied from a prior download",
        )
    )

    checks.extend(verify_integrity(archive, checksum))
    if not all(check.passed for check in checks):
        # Nothing unpacks an archive whose bytes are unproven.
        return checks

    revision = tag_revision(tag)
    checks.append(verify_provenance(tag, archive, revision))

    checks.append(
        delegate(
            "uninstall-roundtrip.py",
            [
                "--release-archive", str(archive),
                "--release-checksum", str(checksum),
                "--release-tag", tag,
                "--soundtrack-archive", str(track),
                "--soundtrack-checksum", str(track_sum),
                "--soundtrack-content-checksum", str(track_content),
            ],
            "install, play, save, and uninstall from the verified artifact",
            "the roundtrip installed the published archive and the published "
            "soundtrack, played a room, wrote player state, uninstalled, and "
            "left every player-owned file byte-identical",
        )
    )
    return checks


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", help="release tag; defaults to the newest published release")
    parser.add_argument(
        "--archive-dir",
        type=Path,
        help="where the archive is or should be placed; defaults to a scratch directory",
    )
    parser.add_argument(
        "--skip-download",
        action="store_true",
        help="use an archive already present in --archive-dir",
    )
    args = parser.parse_args(argv)

    OUT.mkdir(parents=True, exist_ok=True)
    checks: list[Check]
    tag = "unknown"
    target = "unknown"
    try:
        tag = resolve_tag(args.tag)
        target, extension = platform_target()
        directory = args.archive_dir or (OUT / "artifact")
        archive = directory / f"numinous-{tag}-{target}.{extension}"
        checks = gate(tag, archive, Path(f"{archive}.sha256"), args.skip_download)
    except GateError as error:
        checks = [Check("clean-machine release gate", False, str(error))]

    failed = [check for check in checks if not check.passed]
    summary: dict[str, Any] = {
        "suite": "clean-machine-release",
        "passed": not failed,
        "tag": tag,
        "platform": platform.system(),
        "target": target,
        "check_count": len(checks),
        "failed_count": len(failed),
        "results": [check.as_json() for check in checks],
        "evidence_class": "agent-machine",
        "limitations": [
            "one platform per run, the one it executed on",
            "verifiable provenance and checksums, not platform signing or notarization",
            "no window, controller, or audible-sound observation",
            "says nothing about whether any of it is enjoyable",
        ],
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{len(checks) - len(failed)}/{len(checks)} PASS on {summary['platform']} ({target})")
    for check in checks:
        print(f"  {'PASS' if check.passed else 'FAIL'}  {check.name}: {check.detail}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {key: summary[key] for key in ("suite", "passed", "tag", "platform", "failed_count")},
            sort_keys=True,
        )
    )
    return 0 if not failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
