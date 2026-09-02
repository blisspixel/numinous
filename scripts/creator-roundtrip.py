#!/usr/bin/env python3
"""Machine acceptance for the local Studio make / save / reopen loop (CLI).

Creates a .num document, reopens it, and checks deterministic identity of the
saved document and reopened expression text. The version 3 case also proves
one parametric pair, its pitch scale, exact WAV voice, and atomic fork.

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

# The gates share one way of getting the binaries they test; see gate_cli.py
# for why there is only one copy of it.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from gate_cli import resolve_cli  # noqa: E402

EXPRESSIONS = (
    "sin(x)",
    "sin(2*x)",
    "x*x",
    "sin(a*x)",
    "floor(3*sin(x))/3",
    "mod(x + pi, 2*pi) - pi",
    "min(max(x, -2), 2)",
    "max(abs(x) - a, 0)",
)


def run_cli(cli: list[str], args: list[str], env: dict[str, str]) -> tuple[int, str, str]:
    process = subprocess.run(
        [*cli, *args],
        input="",
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
        and not line.startswith(
            (
                "Studio creation",
                "kind=",
                "expr=",
                "xexpr=",
                "yexpr=",
                "xmin=",
                "xmax=",
                "tmin=",
                "tmax=",
                "a=",
                "scale=",
                "title=",
                "author=",
                "link=",
                "remix it:",
                "y = ",
                "x(t) = ",
                "t in [",
            )
        )
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


def check_parametric_pair(
    cli: list[str], work: Path, env: dict[str, str]
) -> dict[str, Any]:
    """Version 3 must keep one pair and its sound through save, open and fork."""
    parent = work / "lissajous.num"
    arguments = [
        "plot",
        "--x-expr=cos(3*t)",
        "--y-expr=sin(2*t)",
        "--tmin=0",
        "--tmax=6.283185307179586",
        "--a=0.25",
        "--scale=pentatonic",
        "--title=Five Petals",
        "--save",
        str(parent),
    ]
    code, stdout, stderr = run_cli(cli, arguments, env)
    if code != 0 or not parent.is_file():
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"save failed: {(stderr or stdout)[:400]}",
        }
    body = parent.read_text(encoding="utf-8")
    required = (
        "NUMINOUS_STUDIO 3\n",
        "kind=parametric\n",
        "xexpr=cos(3*t)\n",
        "yexpr=sin(2*t)\n",
        "tmin=0\n",
        "tmax=6.283185307179586\n",
        "a=0.25\n",
        "scale=pentatonic\n",
        "title=Five Petals\n",
    )
    missing = [field.rstrip() for field in required if field not in body]
    if missing:
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"version 3 capsule omitted {missing}",
        }
    link = next(
        (line.removeprefix("link: ") for line in stdout.splitlines() if line.startswith("link: ")),
        "",
    )
    if not link:
        return {"name": "parametric pair", "passed": False, "detail": "save omitted link"}

    code, reopened, error = run_cli(cli, ["open-studio", str(parent)], env)
    if code != 0 or not drawing(reopened).strip():
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"reopen failed or drew nothing: {(error or reopened)[:400]}",
        }
    for spoken in (
        "kind=parametric",
        "xexpr=cos(3*t)",
        "yexpr=sin(2*t)",
        "scale=pentatonic",
    ):
        if spoken not in reopened:
            return {
                "name": "parametric pair",
                "passed": False,
                "detail": f"reopen omitted {spoken}",
            }

    repeated = work / "lissajous-repeat.num"
    repeat_args = [*arguments]
    repeat_args[-1] = str(repeated)
    code, _, error = run_cli(cli, repeat_args, env)
    if code != 0 or not repeated.is_file() or sha256_file(repeated) != sha256_file(parent):
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"repeated version 3 save drifted: {error[:300]}",
        }

    capsule_wav = work / "capsule.wav"
    raw_wav = work / "raw.wav"
    code, _, error = run_cli(
        cli, ["sing", str(parent), "--notes=24", "--out", str(capsule_wav)], env
    )
    code_raw, _, error_raw = run_cli(
        cli,
        [
            "sing",
            "sin(2*t)",
            "--xmin=0",
            "--xmax=6.283185307179586",
            "--a=0.25",
            "--notes=24",
            "--scale=pentatonic",
            "--out",
            str(raw_wav),
        ],
        env,
    )
    if (
        code != 0
        or code_raw != 0
        or not capsule_wav.is_file()
        or not raw_wav.is_file()
        or sha256_file(capsule_wav) != sha256_file(raw_wav)
    ):
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"stored scale voice drifted: {(error or error_raw)[:300]}",
        }

    child = work / "lissajous-child.num"
    code, _, error = run_cli(
        cli,
        [
            "fork",
            str(parent),
            "--x-expr=cos(5*t)",
            "--y-expr=sin(4*t)",
            "--scale=minor",
            "--out",
            str(child),
        ],
        env,
    )
    child_body = child.read_text(encoding="utf-8") if child.is_file() else ""
    if code != 0 or any(
        field not in child_body
        for field in (
            "xexpr=cos(5*t)\n",
            "yexpr=sin(4*t)\n",
            "scale=minor\n",
            f"descends={link}\n",
        )
    ):
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": f"atomic fork lost pair, scale, or lineage: {(error or child_body)[:400]}",
        }

    refused = work / "partial.num"
    partial, _, partial_error = run_cli(
        cli,
        ["plot", "--x-expr=t", "--save", str(refused)],
        env,
    )
    if partial == 0 or refused.exists() or "both --x-expr and --y-expr" not in partial_error:
        return {
            "name": "parametric pair",
            "passed": False,
            "detail": "a half-pair was not refused atomically",
        }

    return {
        "name": "parametric pair",
        "passed": True,
        "detail": "version 3 pair, scale, drawing, voice, deterministic save, and fork agree",
        "sha256": sha256_file(parent),
    }


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
        results.append(check_parametric_pair(cli, work, env))
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
