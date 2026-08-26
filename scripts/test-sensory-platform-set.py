#!/usr/bin/env python3
"""Regression tests for the physical Sensory Lift pacing set."""

from __future__ import annotations

import copy
from contextlib import redirect_stderr, redirect_stdout
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
from types import ModuleType
import unittest


ROOT = Path(__file__).resolve().parent.parent


def load_script() -> ModuleType:
    specification = importlib.util.spec_from_file_location(
        "numinous_sensory_platform_set",
        ROOT / "scripts" / "sensory-platform-set.py",
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("could not load sensory platform set verifier")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


SET = load_script()
REVISION = "1" * 40


def digest(label: str) -> str:
    return hashlib.sha256(label.encode("utf-8")).hexdigest()


def summary(value: float, samples: int = 120) -> dict[str, object]:
    return {
        "raw": [value] * samples,
        "p50": value,
        "p95": value,
        "maximum": value,
    }


def valid_receipt(operating_system: str, width: int, height: int) -> dict[str, object]:
    budget = SET.TARGETS[(width, height)]
    architecture = "aarch64" if operating_system == "macos" else "x86_64"
    apple = operating_system == "macos"
    adapter_name = "Apple M3" if apple else "AMD Radeon 780M"
    adapter = {
        "name": adapter_name,
        "vendorId": 0 if apple else 4098,
        "deviceId": 0 if apple else 5567,
        "deviceType": "IntegratedGpu",
        "driver": "" if apple else "amdgpu",
        "driverInfo": "" if apple else "24.1",
        "backend": "Metal" if apple else "Vulkan",
        "physicalGpu": True,
    }
    source_hash = digest(f"source-{width}x{height}")
    acquire = 1.0
    rendered = 10.0 if budget == 33.0 else 20.0
    boundary = acquire + rendered
    return {
        "schema": SET.RECEIPT_SCHEMA,
        "schemaVersion": SET.RECEIPT_SCHEMA_VERSION,
        "evidence": {
            "class": "physical-reference-pacing",
            "timingAuthority": "physical-reference-candidate",
            "correctnessClaim": "the deterministic fully composed App frame completed through the production direct surface presenter on this runtime",
            "pacingClaim": "the recorded acquire-through-present-request samples are a candidate result for this named physical reference only",
            "excludes": list(SET.EXCLUDES),
        },
        "build": {
            "packageVersion": "0.4.0-alpha.9",
            "revision": REVISION,
            "profile": "release",
            "binarySha256": digest(f"binary-{operating_system}"),
        },
        "platform": {
            "os": operating_system,
            "architecture": architecture,
            "family": "windows" if operating_system == "windows" else "unix",
            "githubActions": False,
            "machine": f"{operating_system} reference",
            "osVersion": f"{operating_system} 1",
            "powerState": "ac",
        },
        "adapter": adapter,
        "surface": {
            "requestedWidth": width,
            "requestedHeight": height,
            "actualWidth": width,
            "actualHeight": height,
            "format": "Bgra8UnormSrgb",
            "presentMode": "Fifo",
            "desiredMaximumFrameLatency": 1,
        },
        "source": {
            "room": "times-tables",
            "variation": 17,
            "phase": 0.375,
            "width": width,
            "height": height,
            "byteLength": width * height * 4,
            "litPixels": 1000,
            "allAlphaOpaque": True,
            "firstRenderSha256": source_hash,
            "repeatRenderSha256": source_hash,
            "deterministic": True,
            "components": list(SET.COMPONENTS),
        },
        "warmups": 30,
        "samples": 120,
        "presentedFrames": 150,
        "skippedFrames": 0,
        "suboptimalFrames": 0,
        "acquireMs": summary(acquire),
        "renderAndPresentMs": summary(rendered),
        "boundaryMs": summary(boundary),
        "boundaryBudgetMs": budget,
        "checkEnforced": True,
        "failures": [],
        "verdict": "pass",
    }


def record(receipt: dict[str, object]) -> tuple[dict[str, object], bytes]:
    data = (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode("utf-8")
    return receipt, data


def valid_records() -> list[tuple[dict[str, object], bytes]]:
    return [
        record(valid_receipt(operating_system, width, height))
        for operating_system in SET.OPERATING_SYSTEMS
        for width, height in SET.TARGETS
    ]


class SensoryPlatformSetTests(unittest.TestCase):
    def test_contract_runs_once_in_every_local_and_ci_gate(self) -> None:
        command = "scripts/test-sensory-platform-set.py"
        for path in (
            ROOT / "scripts" / "check.ps1",
            ROOT / "scripts" / "check.sh",
            ROOT / "scripts" / "hooks" / "pre-commit",
            ROOT / ".github" / "workflows" / "ci.yml",
        ):
            with self.subTest(path=path):
                self.assertEqual(path.read_text(encoding="utf-8").count(command), 1)

    def test_complete_matrix_builds_a_closed_deterministic_manifest(self) -> None:
        records = valid_records()
        manifest = SET.build_manifest(records)
        self.assertEqual(manifest["schema"], SET.SET_SCHEMA)
        self.assertEqual(manifest["verdict"], "pass")
        self.assertEqual(manifest["receiptCount"], 6)
        self.assertEqual(len(manifest["targets"]), 6)
        self.assertEqual(manifest, SET.build_manifest(list(reversed(records))))
        self.assertEqual(
            {(item["os"], item["width"], item["height"]) for item in manifest["targets"]},
            {
                (operating_system, width, height)
                for operating_system in SET.OPERATING_SYSTEMS
                for width, height in SET.TARGETS
            },
        )

    def test_receipt_validation_recomputes_claims_and_rejects_false_authority(self) -> None:
        mutations = (
            ("unknown field", lambda item: item.update({"extra": True})),
            ("boolean schema version", lambda item: item.update({"schemaVersion": True})),
            ("debug build", lambda item: item["build"].update({"profile": "debug"})),
            ("malformed version", lambda item: item["build"].update({"packageVersion": "latest"})),
            ("hosted run", lambda item: item["platform"].update({"githubActions": True})),
            ("software adapter", lambda item: item["adapter"].update({"deviceType": "Cpu", "physicalGpu": False})),
            ("battery run", lambda item: item["platform"].update({"powerState": "battery"})),
            ("skipped frame", lambda item: item.update({"skippedFrames": 1})),
            ("boolean skipped frame", lambda item: item.update({"skippedFrames": False})),
            ("suboptimal frame", lambda item: item.update({"suboptimalFrames": 1})),
            ("unenforced check", lambda item: item.update({"checkEnforced": False})),
        )
        for label, mutate in mutations:
            with self.subTest(label=label):
                receipt = valid_receipt("windows", 1920, 1080)
                mutate(receipt)
                with self.assertRaises(SET.PacingSetError):
                    SET.validate_receipt(receipt)

    def test_timings_are_recomputed_from_raw_segments_and_budget(self) -> None:
        stale_summary = valid_receipt("windows", 1920, 1080)
        stale_summary["boundaryMs"]["p95"] = 12.0
        with self.assertRaisesRegex(SET.PacingSetError, "disagrees with its raw samples"):
            SET.validate_receipt(stale_summary)

        broken_segment = valid_receipt("windows", 1920, 1080)
        broken_segment["boundaryMs"] = summary(12.0)
        with self.assertRaisesRegex(SET.PacingSetError, "measured segments"):
            SET.validate_receipt(broken_segment)

        missed_budget = valid_receipt("windows", 1920, 1080)
        missed_budget["renderAndPresentMs"] = summary(39.0)
        missed_budget["boundaryMs"] = summary(40.0)
        with self.assertRaisesRegex(SET.PacingSetError, "misses its target budget"):
            SET.validate_receipt(missed_budget)

    def test_matrix_rejects_missing_duplicate_and_mixed_identity_cells(self) -> None:
        records = valid_records()
        with self.assertRaisesRegex(SET.PacingSetError, "exactly six"):
            SET.build_manifest(records[:-1])

        duplicate = records[:-1] + [records[0]]
        with self.assertRaisesRegex(SET.PacingSetError, "repeats target"):
            SET.build_manifest(duplicate)

        mismatched_bytes = list(records)
        mismatched_bytes[0] = (mismatched_bytes[0][0], mismatched_bytes[1][1])
        with self.assertRaisesRegex(SET.PacingSetError, "retained bytes"):
            SET.build_manifest(mismatched_bytes)

        mixed_revision = copy.deepcopy(records)
        mixed_revision[0][0]["build"]["revision"] = "2" * 40
        mixed_revision[0] = record(mixed_revision[0][0])
        with self.assertRaisesRegex(SET.PacingSetError, "mixes build identities"):
            SET.build_manifest(mixed_revision)

    def test_matrix_binds_each_os_pair_and_cross_platform_source(self) -> None:
        mixed_adapter = copy.deepcopy(valid_records())
        mixed_adapter[1][0]["adapter"]["driverInfo"] = "different"
        mixed_adapter[1] = record(mixed_adapter[1][0])
        with self.assertRaisesRegex(SET.PacingSetError, "mixes adapter identity"):
            SET.build_manifest(mixed_adapter)

        changed_source = copy.deepcopy(valid_records())
        changed_hash = digest("changed source")
        changed_source[2][0]["source"]["firstRenderSha256"] = changed_hash
        changed_source[2][0]["source"]["repeatRenderSha256"] = changed_hash
        changed_source[2] = record(changed_source[2][0])
        with self.assertRaisesRegex(SET.PacingSetError, "differs across operating systems"):
            SET.build_manifest(changed_source)

        one_adapter = copy.deepcopy(valid_records())
        for index, (receipt, _data) in enumerate(one_adapter):
            receipt["adapter"] = copy.deepcopy(one_adapter[0][0]["adapter"])
            one_adapter[index] = record(receipt)
        with self.assertRaisesRegex(SET.PacingSetError, "two distinct adapters"):
            SET.build_manifest(one_adapter)

    def test_manifest_write_and_verification_are_exclusive_exact_and_bounded(self) -> None:
        records = valid_records()
        manifest = SET.build_manifest(records)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "set.json"
            SET.write_manifest(path, manifest)
            self.assertEqual(SET.verify_manifest(path, records), manifest)
            with self.assertRaisesRegex(SET.PacingSetError, "already exists"):
                SET.write_manifest(path, manifest)

            changed = copy.deepcopy(records)
            changed[0][0]["boundaryMs"] = summary(10.5)
            changed[0][0]["renderAndPresentMs"] = summary(9.5)
            changed[0] = record(changed[0][0])
            with self.assertRaisesRegex(SET.PacingSetError, "disagrees"):
                SET.verify_manifest(path, changed)

            duplicate_json = root / "duplicate.json"
            duplicate_json.write_text('{"schema":1,"schema":2}', encoding="utf-8")
            with self.assertRaisesRegex(SET.PacingSetError, "repeats field"):
                SET.verify_manifest(duplicate_json, records)

    def test_command_line_builds_and_rechecks_real_receipt_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt_paths = []
            for index, (_receipt, data) in enumerate(valid_records()):
                path = root / f"receipt-{index}.json"
                path.write_bytes(data)
                receipt_paths.append(path)
            manifest = root / "set.json"
            output = io.StringIO()
            with redirect_stdout(output):
                result = SET.main(
                    ["build", "--out", str(manifest), *map(str, receipt_paths)]
                )
                verified = SET.main(
                    ["verify", str(manifest), *map(str, receipt_paths)]
                )
            self.assertEqual((result, verified), (0, 0))
            self.assertIn("physical pacing set pass", output.getvalue())
            self.assertIn("physical pacing set verified", output.getvalue())

            duplicate = root / "duplicate-receipt.json"
            duplicate.write_text('{"schema":1,"schema":2}', encoding="utf-8")
            with redirect_stderr(io.StringIO()):
                self.assertEqual(
                    SET.main(
                        ["build", "--out", str(root / "bad.json"), str(duplicate)]
                    ),
                    1,
                )

            oversized = root / "oversized-receipt.json"
            oversized.write_bytes(b" " * (SET.MAX_RECEIPT_BYTES + 1))
            with self.assertRaisesRegex(SET.PacingSetError, "exceeds"):
                SET.read_receipt(oversized)


if __name__ == "__main__":
    unittest.main()
