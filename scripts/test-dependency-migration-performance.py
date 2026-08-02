#!/usr/bin/env python3
"""Regression tests for the dependency migration performance evidence runner."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import math
import os
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("dependency-migration-performance.py")
SPEC = importlib.util.spec_from_file_location("dependency_migration_performance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
performance = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(performance)


def measurement(
    name: str,
    before: list[float],
    after: list[float],
    limit: float,
) -> dict[str, object]:
    before_stats = performance.summarize_ms(before)
    after_stats = performance.summarize_ms(after)
    ratio = after_stats["p50Ms"] / before_stats["p50Ms"]
    return {
        "name": name,
        "boundary": performance.WORKLOADS[name]["boundary"],
        "allowedMedianRatio": limit,
        "allowedMedianDeltaMs": performance.WORKLOADS[name]["deltaMs"],
        "before": {
            "samplesMs": before,
            "stats": before_stats,
            "binarySha256": "1" * 64,
        },
        "after": {
            "samplesMs": after,
            "stats": after_stats,
            "binarySha256": "2" * 64,
        },
        "medianRatio": round(ratio, 6),
        "medianDeltaMs": round(after_stats["p50Ms"] - before_stats["p50Ms"], 6),
        "passed": ratio <= limit
        or after_stats["p50Ms"] - before_stats["p50Ms"] <= performance.WORKLOADS[name]["deltaMs"],
    }


def valid_receipt() -> dict[str, object]:
    measurements = []
    before_samples = [round(10.0 + index / 10.0, 6) for index in range(20)]
    after_samples = [round(10.5 + index / 10.0, 6) for index in range(20)]
    for name, workload in performance.WORKLOADS.items():
        item = measurement(
            name,
            before_samples,
            after_samples,
            workload["limit"],
        )
        for side in ("before", "after"):
            item[side].update(copy.deepcopy(performance.REFERENCE_IDENTITIES[name][side]))
        if name == "appVisibleWindow":
            item["probe"] = "win32-visible-top-level-window-v1"
        measurements.append(item)
    return {
        "schemaVersion": performance.SCHEMA_VERSION,
        "generatedAt": "2026-08-02T08:00:00Z",
        "repository": {
            "url": performance.REPOSITORY_URL,
            "runnerSourceSha256": "5" * 64,
        },
        "machine": copy.deepcopy(performance.REFERENCE_MACHINE),
        "configuration": {
            "warmupSamplesPerRevision": performance.EXPECTED_WARMUPS,
            "measuredSamplesPerRevision": performance.EXPECTED_SAMPLES,
            "order": "alternating-ab-ba",
            "profile": "release",
            "locked": True,
        },
        "revisions": {
            "before": {
                "commit": performance.BEFORE_REVISION,
                **performance.REFERENCE_TOOLCHAINS["before"],
            },
            "after": {
                "commit": performance.AFTER_REVISION,
                **performance.REFERENCE_TOOLCHAINS["after"],
            },
        },
        "measurements": measurements,
        "verdict": {"passed": True, "failedMeasurements": []},
    }


class StatisticsTests(unittest.TestCase):
    def test_nearest_rank_summary_is_stable(self) -> None:
        self.assertEqual(
            performance.summarize_ms([9.0, 1.0, 5.0, 3.0, 7.0]),
            {"p50Ms": 5.0, "p95Ms": 9.0, "maxMs": 9.0},
        )

    def test_summary_rejects_empty_nonfinite_and_nonpositive_samples(self) -> None:
        for samples in ([], [0.0], [-1.0], [math.inf], [math.nan]):
            with self.subTest(samples=samples):
                with self.assertRaises(performance.PerformanceError):
                    performance.summarize_ms(samples)


class ProbeParsingTests(unittest.TestCase):
    def test_audio_receipt_parses_bounded_fields(self) -> None:
        receipt = performance.parse_audio_receipt(
            "audio-ready\t1250000\t48000\t2\tFixture Audio\n"
        )
        self.assertEqual(receipt["durationMs"], 1.25)
        self.assertEqual(receipt["sampleRate"], 48_000)
        self.assertEqual(receipt["channels"], 2)
        self.assertEqual(receipt["device"], "Fixture Audio")

    def test_audio_receipt_rejects_malformed_or_unbounded_values(self) -> None:
        malformed = (
            "audio-ready\t0\t48000\t2\tDevice\n",
            "audio-ready\t1\t999999\t2\tDevice\n",
            "audio-ready\t1\t48000\t0\tDevice\n",
            "audio-ready\t1\t48000\t2\t\n",
            "audio-ready\t1\t48000\t2\tA\tB\n",
            "noise\n",
        )
        for output in malformed:
            with self.subTest(output=output):
                with self.assertRaises(performance.PerformanceError):
                    performance.parse_audio_receipt(output)

    def test_gpu_receipt_parses_adapter_and_backend(self) -> None:
        self.assertEqual(
            performance.parse_gpu_receipt("Rendering on: Radeon(TM) 780M (Dx12)\nwrote x\n"),
            {"adapter": "Radeon(TM) 780M", "backend": "Dx12"},
        )

    def test_gpu_receipt_rejects_controls_and_ambiguous_lines(self) -> None:
        for output in (
            "Rendering on: GPU (Dx12)\nRendering on: GPU (Dx12)\n",
            "Rendering on: bad\x1bname (Dx12)\n",
            "Rendering on: GPU ()\n",
        ):
            with self.subTest(output=output):
                with self.assertRaises(performance.PerformanceError):
                    performance.parse_gpu_receipt(output)

    def test_png_dimensions_read_exact_ihdr(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "image.png"
            path.write_bytes(
                b"\x89PNG\r\n\x1a\n"
                + struct.pack(">I", 13)
                + b"IHDR"
                + struct.pack(">II", 1200, 900)
                + b"\x08\x06\x00\x00\x00"
                + b"\x00\x00\x00\x00"
            )
            self.assertEqual(performance.read_png_dimensions(path), (1200, 900))

    def test_png_dimensions_reject_truncation_and_wrong_header(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "image.png"
            for content in (b"", b"not a png", b"\x89PNG\r\n\x1a\n" + b"\x00" * 20):
                path.write_bytes(content)
                with self.subTest(content=content):
                    with self.assertRaises(performance.PerformanceError):
                        performance.read_png_dimensions(path)


class ReceiptTests(unittest.TestCase):
    def test_complete_receipt_validates(self) -> None:
        performance.validate_receipt(valid_receipt())

    def test_receipt_rejects_revision_output_and_stat_drift(self) -> None:
        mutations = []

        wrong_revision = valid_receipt()
        wrong_revision["revisions"]["before"]["commit"] = "0" * 40
        mutations.append(wrong_revision)

        changed_cli = valid_receipt()
        changed_cli["measurements"][0]["after"]["outputSha256"] = "9" * 64
        mutations.append(changed_cli)

        missing_cli = valid_receipt()
        del missing_cli["measurements"][0]["before"]["outputSha256"]
        mutations.append(missing_cli)

        malformed_binary = valid_receipt()
        malformed_binary["measurements"][0]["before"]["binarySha256"] = "not-a-digest"
        mutations.append(malformed_binary)

        wrong_stats = valid_receipt()
        wrong_stats["measurements"][0]["after"]["stats"]["p50Ms"] = 99.0
        mutations.append(wrong_stats)

        for receipt in mutations:
            with self.subTest(receipt=receipt):
                with self.assertRaises(performance.PerformanceError):
                    performance.validate_receipt(receipt)

    def test_receipt_rejects_nonreference_environment_and_sampling(self) -> None:
        mutations = []

        changed_machine = valid_receipt()
        changed_machine["machine"]["os"] = "Linux"
        mutations.append(changed_machine)

        no_warmups = valid_receipt()
        no_warmups["configuration"]["warmupSamplesPerRevision"] = 0
        mutations.append(no_warmups)

        one_sample = valid_receipt()
        one_sample["configuration"]["measuredSamplesPerRevision"] = 1
        for item in one_sample["measurements"]:
            for side in ("before", "after"):
                item[side]["samplesMs"] = item[side]["samplesMs"][:1]
                item[side]["stats"] = performance.summarize_ms(item[side]["samplesMs"])
        mutations.append(one_sample)

        date_only = valid_receipt()
        date_only["generatedAt"] = "2026-08-02"
        mutations.append(date_only)

        for receipt in mutations:
            with self.subTest(receipt=receipt):
                with self.assertRaises(performance.PerformanceError):
                    performance.validate_receipt(receipt)

    def test_receipt_rejects_missing_or_duplicate_workloads(self) -> None:
        missing = valid_receipt()
        missing["measurements"].pop()
        duplicate = valid_receipt()
        duplicate["measurements"][-1] = duplicate["measurements"][0]
        for receipt in (missing, duplicate):
            with self.subTest(receipt=receipt):
                with self.assertRaises(performance.PerformanceError):
                    performance.validate_receipt(receipt)

    def test_receipt_rejects_a_failed_or_falsely_passing_verdict(self) -> None:
        failed = valid_receipt()
        failed["measurements"][0]["medianRatio"] = 10.0
        failed["measurements"][0]["passed"] = False
        failed["verdict"] = {"passed": False, "failedMeasurements": ["cliRequest"]}
        with self.assertRaises(performance.PerformanceError):
            performance.validate_receipt(failed)

        false_pass = valid_receipt()
        false_pass["measurements"][0]["after"]["samplesMs"] = [100.0, 110.0, 120.0]
        false_pass["measurements"][0]["after"]["stats"] = performance.summarize_ms(
            [100.0, 110.0, 120.0]
        )
        with self.assertRaises(performance.PerformanceError):
            performance.validate_receipt(false_pass)

    def test_canonical_json_has_stable_digest(self) -> None:
        receipt = valid_receipt()
        encoded = performance.canonical_json(receipt)
        self.assertTrue(encoded.endswith(b"\n"))
        self.assertEqual(encoded, performance.canonical_json(json.loads(encoded)))
        self.assertEqual(
            hashlib.sha256(encoded).hexdigest(),
            "fd3919419f74fb0131230cef9f063e30b625af772f98e71a20eb1e93ef80433a",
        )

    def test_runner_identity_binds_exact_source_bytes(self) -> None:
        receipt = valid_receipt()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "runner.py"
            path.write_bytes(b"exact runner\n")
            receipt["repository"]["runnerSourceSha256"] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()
            performance.validate_runner_identity(receipt, path)
            path.write_bytes(b"changed runner\n")
            with self.assertRaises(performance.PerformanceError):
                performance.validate_runner_identity(receipt, path)

    def test_strict_json_rejects_duplicate_keys_and_nonfinite_constants(self) -> None:
        for content in (b'{"a":1,"a":2}\n', b'{"a":NaN}\n'):
            with self.subTest(content=content):
                with self.assertRaises(performance.PerformanceError):
                    performance.parse_json_strict(content)

    def test_owned_directory_rejects_redirection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            owned = performance._owned_directory(root, "owned")
            self.assertEqual(owned.parent, root)
            redirected = root / "redirected"
            if os.name == "nt":
                result = subprocess.run(
                    ["cmd", "/c", "mklink", "/J", str(redirected), str(owned)],
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            else:
                redirected.symlink_to(owned, target_is_directory=True)
            with self.assertRaises(performance.PerformanceError):
                performance._owned_directory(root, "redirected")

    def test_receipt_output_must_be_outside_cleanup_owned_work_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory).resolve()
            agent = repo / ".agent"
            agent.mkdir()
            work_root = agent / "work"
            for output in (work_root, work_root / "receipt.json"):
                with self.subTest(output=output):
                    with self.assertRaises(performance.PerformanceError):
                        performance._resolve_record_paths(repo, output, work_root)
            retained = agent / "receipt.json"
            self.assertEqual(
                performance._resolve_record_paths(repo, retained, work_root),
                (work_root, retained),
            )


if __name__ == "__main__":
    unittest.main()
