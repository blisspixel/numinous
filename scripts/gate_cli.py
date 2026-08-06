"""One way for a gate to get the binaries it is about to test.

Every gate that drives a real binary needs the same three things: build the
current source, find what the build wrote, and refuse to continue if either
step did not happen. Six gates each had their own copy of that, and three of
those copies were wrong in the same way, quietly testing whichever artifact
happened to be lying in `target/` instead of the code under review.

Copies drift. So this is the only copy, and every gate imports it.

Imported by file name rather than as a package because the gates live beside it
with hyphens in their names, which cannot be imported. Each gate puts this
directory on `sys.path` and asks for what it needs.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class GateError(RuntimeError):
    """The binaries a gate needs could not be produced or found."""


def target_debug() -> Path:
    """Where cargo writes debug binaries for this checkout.

    `CARGO_TARGET_DIR` redirects that, and several CI layouts set it, so a
    build that succeeded can still look missing under `ROOT/target`. A relative
    value is resolved by cargo against its own working directory, which is
    always ROOT here, so it is resolved the same way rather than against
    whatever directory the gate was started from.
    """
    configured = os.environ.get("CARGO_TARGET_DIR")
    target_root = Path(configured) if configured else ROOT / "target"
    if not target_root.is_absolute():
        target_root = ROOT / target_root
    return target_root / "debug"


def build_and_locate(names: tuple[str, ...]) -> list[Path]:
    """Build these binaries from the current source and return their paths.

    A gate observes live behaviour, so it has to observe the behaviour of the
    source it was asked about. Picking up whichever binary happened to be on
    disk lets a stale artifact answer for code that no longer exists, and the
    gate passes while the thing is broken. That is not hypothetical: three
    gates did it, and with `rooms` made to print nothing and the binary left
    alone they reported 30 of 30, 41 of 41 and 6 of 6.

    Cargo is incremental, so on an already-built tree this costs almost
    nothing. It also means no gate needs a `cargo run` fallback, which could
    spend a whole per-command timeout compiling or waiting on the build lock.
    """
    if not names:
        raise GateError("a gate asked for no binaries, so there is nothing to test")
    command = ["cargo", "build", "--quiet", "--locked"]
    for name in names:
        command += ["--bin", name]
    build = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if build.returncode != 0:
        raise GateError(
            "cannot build the binaries under test "
            f"({', '.join(names)}):\n{build.stderr}"
        )
    debug = target_debug()
    found: list[Path] = []
    for name in names:
        for candidate in (debug / f"{name}.exe", debug / name):
            if candidate.is_file():
                found.append(candidate)
                break
        else:
            raise GateError(
                f"cargo build reported success but {name} is not under {debug}"
            )
    return found


def resolve_cli() -> list[str]:
    """The CLI binary, freshly built, as an argv prefix."""
    return [str(build_and_locate(("numinous",))[0])]
