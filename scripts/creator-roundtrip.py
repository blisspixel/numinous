#!/usr/bin/env python3
"""Machine acceptance for the local Studio make / save / reopen loop (CLI).

Creates a .num document, reopens it, and checks deterministic identity of the
saved document and reopened expression text.

A creation is more than its expression. A player who narrowed the range or
turned the knob made those part of what they saved, so the range and the knob
are checked too: saved into the document, reported on reopen, and actually
changing the drawing. That last part is the one worth having. A reopen that
echoed the saved numbers and then drew the default curve would satisfy every
other check in this file, which is why the drawings are compared rather than
the whole output.

App gallery and MCP artifact delivery remain separate gates. Not a human
creator usability claim.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".agent" / "tester-cohort" / "creator-roundtrip"

EXPRESSIONS = (
    "sin(x)",
    "sin(2*x)",
    "x*x",
    "sin(a*x)",
)


def resolve_cli() -> list[str]:
    """Build the CLI, then return the binary that build produced.

    This exercises live behaviour, so it has to exercise the behaviour of the
    current source. Picking up whichever binary happened to be on disk lets a
    stale artifact answer for code that no longer exists, and the gate passes
    while the thing is broken. Demonstrated rather than assumed: with `rooms`
    made to print nothing and the binary left alone, this reported a full pass.

    Cargo is incremental, so on an already-built tree this costs almost nothing.
    """
    build = subprocess.run(
        ["cargo", "build", "--quiet", "--locked", "--bin", "numinous"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        raise SystemExit("cannot build the CLI under test:\n" + build.stderr)
    # CARGO_TARGET_DIR redirects where cargo writes and several CI layouts set
    # it, so a build that succeeded could still look missing under ROOT/target.
    configured = os.environ.get("CARGO_TARGET_DIR")
    target_root = Path(configured) if configured else ROOT / "target"
    if not target_root.is_absolute():
        target_root = ROOT / target_root
    for name in ("numinous.exe", "numinous"):
        candidate = target_root / "debug" / name
        if candidate.is_file():
            return [str(candidate)]
    raise SystemExit(
        f"cargo build reported success but no numinous binary is under {target_root / 'debug'}"
    )


def run_cli(cli: list[str], args: list[str], env: dict[str, str]) -> tuple[int, str, str]:
    process = subprocess.run(
        [*cli, *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )
    return process.returncode, process.stdout, process.stderr


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def isolated_env(profile: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(
        {
            "NUMINOUS_JOURNEY": str(profile / "journey.txt"),
            "NUMINOUS_SCORES": str(profile / "scores.txt"),
            "NUMINOUS_CAIRN": str(profile / "cairn.json"),
            "NUMINOUS_MUTE": "1",
        }
    )
    return env


def check_expression(cli: list[str], expr: str, work: Path, env: dict[str, str]) -> dict[str, Any]:
    path = work / f"{hashlib.sha256(expr.encode()).hexdigest()[:12]}.num"
    code, stdout, stderr = run_cli(cli, ["plot", expr, "--save", str(path)], env)
    if code != 0 or not path.is_file():
        return {
            "expression": expr,
            "passed": False,
            "detail": f"save failed: {(stderr or stdout)[:400]}",
        }
    body = path.read_text(encoding="utf-8")
    digest = sha256_file(path)
    code2, stdout2, stderr2 = run_cli(cli, ["open-studio", str(path)], env)
    if code2 != 0:
        return {
            "expression": expr,
            "passed": False,
            "detail": f"open failed: {(stderr2 or stdout2)[:400]}",
            "sha256": digest,
        }
    combined = stdout2 + stderr2
    # Reopen must surface the expression identity, not only succeed.
    if expr not in combined and expr.replace(" ", "") not in combined.replace(" ", ""):
        return {
            "expression": expr,
            "passed": False,
            "detail": f"reopen output omitted expression; got {combined[:200]!r}",
            "sha256": digest,
        }
    # Second save of the same expression must be byte-identical for fixed ranges.
    path_b = work / f"{path.stem}-b.num"
    code3, _, stderr3 = run_cli(cli, ["plot", expr, "--save", str(path_b)], env)
    if code3 != 0 or not path_b.is_file():
        return {
            "expression": expr,
            "passed": False,
            "detail": f"second save failed: {stderr3[:200]}",
            "sha256": digest,
        }
    digest_b = sha256_file(path_b)
    if digest_b != digest:
        return {
            "expression": expr,
            "passed": False,
            "detail": "repeated save was not byte-identical",
            "sha256": digest,
            "sha256_second": digest_b,
            "body": body,
        }
    return {
        "expression": expr,
        "passed": True,
        "detail": "save, reopen, and deterministic rewrite ok",
        "sha256": digest,
        "bytes": path.stat().st_size,
    }


def drawing(reopened: str) -> str:
    """The picture a reopen produced, without the metadata printed above it.

    `open-studio` prints what it restored and then draws it. Comparing whole
    outputs would let a reopen that echoes the saved numbers and then draws the
    default curve look correct, since the echo alone would differ.
    """
    lines = reopened.splitlines()
    body = [
        line
        for line in lines
        if line
        and not line.startswith(("Studio creation", "expr=", "xmin=", "xmax=", "a=", "link=", "y = "))
    ]
    return "\n".join(line.rstrip() for line in body)


def check_settings_survive(cli: list[str], work: Path, env: dict[str, str]) -> dict[str, Any]:
    """A saved creation comes back as itself, not as the default view of itself.

    The expression checks above prove a document saves and reopens. They say
    nothing about the rest of the creation: a player who narrowed the range and
    turned the knob has made those part of what they saved. A reopen that
    restored the expression and then drew the default picture would satisfy
    every other check in this file.

    Two documents that differ only in range must reopen to different drawings,
    and likewise for the knob. Comparing the drawings rather than the whole
    output matters, because the metadata echo would differ on its own even if
    the curve never changed.
    """
    made: dict[str, tuple[str, str]] = {}
    for label, args in (
        ("wide", ["sin(a*x)", "--xmin=-6.2831853", "--xmax=6.2831853", "--a=1"]),
        ("narrow", ["sin(a*x)", "--xmin=-0.5", "--xmax=0.5", "--a=1"]),
        ("knob", ["sin(a*x)", "--xmin=-6.2831853", "--xmax=6.2831853", "--a=7"]),
    ):
        path = work / f"settings-{label}.num"
        code, stdout, stderr = run_cli(cli, ["plot", *args, "--save", str(path)], env)
        if code != 0 or not path.is_file():
            return {
                "name": "settings survive",
                "passed": False,
                "detail": f"{label} save failed: {(stderr or stdout)[:300]}",
            }
        code2, stdout2, stderr2 = run_cli(cli, ["open-studio", str(path)], env)
        if code2 != 0:
            return {
                "name": "settings survive",
                "passed": False,
                "detail": f"{label} reopen failed: {(stderr2 or stdout2)[:300]}",
            }
        made[label] = (path.read_text(encoding="utf-8"), stdout2)

    reasons = []
    for label, wanted in (
        ("narrow", ("xmin=-0.5", "xmax=0.5")),
        ("knob", ("a=7",)),
    ):
        document, reopened = made[label]
        for setting in wanted:
            if setting not in document:
                reasons.append(f"{label} did not save {setting}")
            if setting not in reopened:
                reasons.append(f"{label} did not report {setting} on reopen")

    if drawing(made["wide"][1]) == drawing(made["narrow"][1]):
        reasons.append("a narrowed range reopened to the same drawing as the wide one")
    if drawing(made["wide"][1]) == drawing(made["knob"][1]):
        reasons.append("a turned knob reopened to the same drawing as the untouched one")
    if not drawing(made["wide"][1]).strip():
        reasons.append("reopening drew nothing, so the comparisons prove nothing")

    return {
        "name": "settings survive",
        "passed": not reasons,
        "detail": "; ".join(reasons)
        or "range and knob are saved, reported on reopen, and change the drawing",
    }


def check_recipe_bank(cli: list[str], env: dict[str, str]) -> dict[str, Any]:
    code, stdout, stderr = run_cli(cli, ["plot", "--list-recipes"], env)
    if code != 0:
        return {
            "name": "list_recipes",
            "passed": False,
            "detail": (stderr or stdout)[:300],
        }
    text = stdout + stderr
    if "recipe" not in text.lower() and "sin" not in text.lower():
        return {
            "name": "list_recipes",
            "passed": False,
            "detail": f"unexpected list output: {text[:200]!r}",
        }
    return {"name": "list_recipes", "passed": True, "detail": "recipe bank listed"}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    cli = resolve_cli()
    results: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="numinous-creator-") as tmp:
        work = Path(tmp)
        profile = work / "profile"
        profile.mkdir()
        env = isolated_env(profile)
        results.append(check_recipe_bank(cli, env))
        results.append(check_settings_survive(cli, work, env))
        for expr in EXPRESSIONS:
            results.append(check_expression(cli, expr, work, env))
    failed = [item for item in results if not item.get("passed")]
    summary = {
        "suite": "creator-roundtrip",
        "passed": not failed,
        "check_count": len(results),
        "failed_count": len(failed),
        "results": results,
        "evidence_class": "agent-machine",
    }
    path = OUT / "summary.json"
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
    print(f"{summary['check_count'] - summary['failed_count']}/{summary['check_count']} PASS")
    for item in results:
        mark = "PASS" if item.get("passed") else "FAIL"
        name = item.get("expression") or item.get("name") or "?"
        print(f"  {mark}  {name}: {item.get('detail')}")
    print("--- summary.json ---")
    print(
        json.dumps(
            {
                "suite": summary["suite"],
                "passed": summary["passed"],
                "check_count": summary["check_count"],
                "failed_count": summary["failed_count"],
                "evidence_class": summary["evidence_class"],
            },
            sort_keys=True,
        )
    )
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
