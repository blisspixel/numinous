#!/usr/bin/env python3
"""Machine acceptance for the local Studio make / save / reopen loop (CLI).

Creates a .num document, reopens it, and checks deterministic identity of the
saved document and reopened expression text. App gallery and MCP artifact
delivery remain separate gates. Not a human creator usability claim.
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
    candidates = [
        ROOT / "target" / "debug" / "numinous.exe",
        ROOT / "target" / "debug" / "numinous",
        ROOT / "target" / "release" / "numinous.exe",
        ROOT / "target" / "release" / "numinous",
    ]
    for path in candidates:
        if path.is_file():
            return [str(path)]
    return ["cargo", "run", "--quiet", "--locked", "--bin", "numinous", "--"]


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
