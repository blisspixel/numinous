#!/usr/bin/env python3
"""Regression tests for physical input session evidence."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import platform
import tempfile
from typing import Any
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "numinous_input_hardware_session",
    ROOT / "scripts" / "input-hardware-session.py",
)
assert SPEC is not None and SPEC.loader is not None
SESSION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SESSION)


TARGET_HOSTS = {
    "x86_64-pc-windows-msvc": ("windows", "amd64"),
    "x86_64-unknown-linux-gnu": ("linux", "x86_64"),
    "x86_64-apple-darwin": ("darwin", "x86_64"),
    "aarch64-apple-darwin": ("darwin", "arm64"),
}


def sample_release(target: str | None = None) -> dict[str, Any]:
    binary_hashes = {
        name: {"bytes": index + 1, "sha256": f"{index + 1:064x}"}
        for index, name in enumerate(SESSION.PACKAGE.BINARIES)
    }
    return {
        "archive": "numinous-release.zip",
        "archiveBytes": 123,
        "archiveSha256": "a" * 64,
        "commit": "b" * 40,
        "target": target or SESSION.expected_target(),
        "version": "0.2.0-alpha.4",
        "binaries": binary_hashes,
    }


def sample_receipt(
    result: str = "pass",
    *,
    target: str | None = None,
    profile: str = "xbox",
    controller_name: str = "Example Physical Controller",
) -> dict[str, Any]:
    selected_target = target or SESSION.expected_target()
    host_system, host_machine = TARGET_HOSTS[selected_target]
    observations = [
        {
            "checkpoint": checkpoint["id"],
            "input": checkpoint["input"],
            "result": result,
            "observation": f"Observed {checkpoint['id']} on physical hardware.",
        }
        for checkpoint in SESSION.CHECKPOINTS
    ]
    observations[-2]["observation"] = "JOURNEY LV 2 | 7 XP"
    observations[-1]["observation"] = "JOURNEY LV 2 | 7 XP"
    body = {
        "schema": SESSION.SCHEMA,
        "schemaVersion": SESSION.SCHEMA_VERSION,
        "recordedAt": "2026-08-01T12:34:56Z",
        "result": result,
        "release": sample_release(selected_target),
        "host": {
            "system": host_system,
            "systemRelease": platform.release(),
            "machine": host_machine,
        },
        "controller": {
            "name": controller_name,
            "connection": "wired",
            "legendProfile": profile,
        },
        "automated": {
            "archiveVerified": True,
            "installedPayloadMatch": True,
            "cliMcpEngagement": True,
            "cliMcpVersion": "0.2.0-alpha.4",
        },
        "appLifecycle": {
            "firstLaunchExitCode": 0,
            "secondLaunchExitCode": 0,
        },
        "persistence": {
            "beforeExit": {"level": 2, "xp": 7},
            "afterRestart": {"level": 2, "xp": 7},
        },
        "observations": observations,
        "limitations": list(SESSION.LIMITATIONS),
    }
    return SESSION.attach_content_id(body)


def reidentify(receipt: dict[str, Any]) -> dict[str, Any]:
    body = {name: value for name, value in receipt.items() if name != "contentId"}
    return SESSION.attach_content_id(body)


class InputHardwareSessionTests(unittest.TestCase):
    def test_checkpoint_inventory_covers_each_required_input_boundary(self) -> None:
        identifiers = [checkpoint["id"] for checkpoint in SESSION.CHECKPOINTS]
        self.assertEqual(len(identifiers), len(set(identifiers)))
        self.assertEqual(
            {checkpoint["input"] for checkpoint in SESSION.CHECKPOINTS},
            {"keyboard", "mouse", "controller", "lifecycle"},
        )
        self.assertGreaterEqual(
            sum(checkpoint["input"] == "controller" for checkpoint in SESSION.CHECKPOINTS),
            5,
        )

    def test_complete_pass_and_honest_fail_receipts_validate(self) -> None:
        SESSION.validate_receipt(sample_receipt("pass"))
        SESSION.validate_receipt(sample_receipt("fail"))

    def test_content_identifier_rejects_any_unidentified_mutation(self) -> None:
        receipt = sample_receipt()
        receipt["controller"]["name"] = "Changed Controller"
        with self.assertRaisesRegex(SESSION.SessionError, "content id"):
            SESSION.validate_receipt(receipt)

    def test_checkpoint_inventory_and_aggregate_fail_closed(self) -> None:
        missing = sample_receipt()
        missing["observations"].pop()
        with self.assertRaisesRegex(SESSION.SessionError, "every checkpoint"):
            SESSION.validate_receipt(reidentify(missing))

        reordered = sample_receipt()
        reordered["observations"][0], reordered["observations"][1] = (
            reordered["observations"][1],
            reordered["observations"][0],
        )
        with self.assertRaisesRegex(SESSION.SessionError, "input family|order"):
            SESSION.validate_receipt(reidentify(reordered))

        dishonest = sample_receipt()
        dishonest["observations"][0]["result"] = "fail"
        with self.assertRaisesRegex(SESSION.SessionError, "disagrees"):
            SESSION.validate_receipt(reidentify(dishonest))

    def test_automated_and_lifecycle_claims_cannot_be_partial(self) -> None:
        for section, field, value, message in (
            ("automated", "cliMcpEngagement", False, "incomplete"),
            ("appLifecycle", "secondLaunchExitCode", 1, "cleanly"),
        ):
            receipt = sample_receipt()
            receipt[section][field] = value
            with self.subTest(section=section, field=field):
                with self.assertRaisesRegex(SESSION.SessionError, message):
                    SESSION.validate_receipt(reidentify(receipt))

    def test_archive_and_installed_binaries_must_match_byte_for_byte(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary_dir = root / "bin"
            binary_dir.mkdir()
            suffix = ".exe" if SESSION.os.name == "nt" else ""
            for index, name in enumerate(SESSION.PACKAGE.BINARIES):
                (binary_dir / f"{name}{suffix}").write_bytes(
                    f"binary-{index}".encode("ascii")
                )
            archive, checksum = SESSION.PACKAGE.build_archive(
                "0.2.0-alpha.4",
                SESSION.expected_target(),
                "binaries",
                binary_dir,
                ROOT / "assets" / "radio",
                root / "dist",
                ROOT,
            )
            with SESSION.release_install_evidence(
                archive, checksum, binary_dir
            ) as (release, paths):
                self.assertEqual(release["target"], SESSION.expected_target())
                self.assertEqual(set(paths), set(SESSION.PACKAGE.BINARIES))
                self.assertTrue(all(path.is_file() for path in paths.values()))
                (binary_dir / f"numinous{suffix}").write_bytes(b"swapped-after-pin")
                self.assertNotEqual(
                    paths["numinous"].read_bytes(), b"swapped-after-pin"
                )

            (binary_dir / f"numinous-app{suffix}").write_bytes(b"replacement")
            with self.assertRaisesRegex(SESSION.SessionError, "does not match"):
                with SESSION.release_install_evidence(
                    archive, checksum, binary_dir
                ):
                    self.fail("mismatched installed binary was yielded")

    def test_archive_swap_after_verification_cannot_change_pinned_payload(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            suffix = ".exe" if SESSION.os.name == "nt" else ""
            original_bin = root / "original-bin"
            installed_bin = root / "installed-bin"
            original_bin.mkdir()
            installed_bin.mkdir()
            for name in SESSION.PACKAGE.BINARIES:
                (original_bin / f"{name}{suffix}").write_bytes(
                    f"original-{name}".encode("ascii")
                )
                (installed_bin / f"{name}{suffix}").write_bytes(
                    f"replacement-{name}".encode("ascii")
                )
            original_archive, original_checksum = SESSION.PACKAGE.build_archive(
                "0.2.0-alpha.4",
                SESSION.expected_target(),
                "binaries",
                original_bin,
                ROOT / "assets" / "radio",
                root / "original-dist",
                ROOT,
            )
            replacement_archive, _replacement_checksum = SESSION.PACKAGE.build_archive(
                "0.2.0-alpha.4",
                SESSION.expected_target(),
                "binaries",
                installed_bin,
                ROOT / "assets" / "radio",
                root / "replacement-dist",
                ROOT,
            )
            original_verify = SESSION.PACKAGE.verify_archive

            def verify_then_swap(path: Path, checksum: Path) -> dict[str, bytes]:
                files = original_verify(path, checksum)
                path.write_bytes(replacement_archive.read_bytes())
                return files

            with mock.patch.object(
                SESSION.PACKAGE, "verify_archive", side_effect=verify_then_swap
            ):
                with self.assertRaisesRegex(SESSION.SessionError, "does not match"):
                    with SESSION.release_install_evidence(
                        original_archive, original_checksum, installed_bin
                    ):
                        self.fail("swapped archive payload was yielded")

    def test_prompt_requires_explicit_result_and_bounded_observation(self) -> None:
        answers = iter(("yes", "PASS", "", "Observed exact room navigation."))
        with mock.patch("builtins.print"):
            observation = SESSION.prompt_observation(
                SESSION.CHECKPOINTS[0], lambda _prompt: next(answers)
            )
        self.assertEqual(observation["result"], "pass")
        self.assertEqual(
            observation["observation"], "Observed exact room navigation."
        )

    def test_collection_runs_cli_mcp_and_two_app_lifecycles(self) -> None:
        paths = {
            name: Path("C:/installed") / name for name in SESSION.PACKAGE.BINARIES
        }
        first = [
            {
                "checkpoint": checkpoint["id"],
                "input": checkpoint["input"],
                "result": "pass",
                "observation": "Observed on hardware.",
            }
            for checkpoint in SESSION.CHECKPOINTS[:-1]
        ]
        first[-1]["observation"] = "JOURNEY LV 2 | 7 XP"
        second = [
            {
                "checkpoint": SESSION.CHECKPOINTS[-1]["id"],
                "input": SESSION.CHECKPOINTS[-1]["input"],
                "result": "pass",
                "observation": "JOURNEY LV 2 | 7 XP",
            }
        ]
        with mock.patch("builtins.print"):
            with mock.patch.object(SESSION, "validate_binary_snapshots") as snapshots:
                with mock.patch.object(
                    SESSION.SMOKE,
                    "run_engagement_smoke",
                    return_value="0.2.0-alpha.4",
                ) as smoke:
                    with mock.patch.object(
                        SESSION.SMOKE,
                        "isolated_environment",
                        return_value={"HOME": "isolated"},
                    ):
                        with mock.patch.object(
                            SESSION,
                            "run_app_phase",
                            side_effect=((first, 0), (second, 0)),
                        ) as app_phase:
                            receipt = SESSION.collect_session(
                                sample_release(), paths, "Controller", "wired", "generic"
                            )
        smoke.assert_called_once_with(paths["numinous"].parent)
        self.assertEqual(snapshots.call_count, 5)
        self.assertEqual(app_phase.call_count, 2)
        self.assertEqual(receipt["result"], "pass")
        SESSION.validate_receipt(receipt)

    def test_interrupted_app_observation_terminates_the_launched_process(self) -> None:
        process = mock.Mock()
        process.poll.return_value = None
        process.wait.return_value = 0
        with mock.patch("builtins.print"):
            with mock.patch.object(SESSION.subprocess, "Popen", return_value=process):
                with mock.patch.object(SESSION, "APP_STARTUP_SECONDS", 0):
                    with self.assertRaises(EOFError):
                        SESSION.run_app_phase(
                            Path("numinous-app"),
                            {},
                            SESSION.CHECKPOINTS[:1],
                            lambda _prompt: (_ for _ in ()).throw(EOFError()),
                        )
        process.kill.assert_called_once_with()
        process.wait.assert_called_once_with()

    def test_receipt_writes_only_under_logs_and_is_read_back_strictly(self) -> None:
        receipt = sample_receipt()
        (ROOT / "logs").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "logs") as temporary:
            destination = SESSION.write_receipt(receipt, Path(temporary))
            self.assertEqual(SESSION.read_receipt(destination), receipt)
            with self.assertRaisesRegex(SESSION.SessionError, "already exists"):
                SESSION.write_receipt(receipt, Path(temporary))
        with tempfile.TemporaryDirectory() as outside:
            with self.assertRaisesRegex(SESSION.SessionError, "logs directory"):
                SESSION.write_receipt(receipt, Path(outside))
        with mock.patch.object(
            SESSION,
            "is_link_like",
            side_effect=lambda path: path.name == "logs",
        ):
            with self.assertRaisesRegex(SESSION.SessionError, "link-like"):
                SESSION.write_receipt(receipt)

    def test_unknown_fields_and_oversize_receipts_are_rejected(self) -> None:
        receipt = sample_receipt()
        receipt["unexpected"] = True
        with self.assertRaisesRegex(SESSION.SessionError, "fields"):
            SESSION.validate_receipt(reidentify(receipt))

        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "large.json"
            path.write_bytes(b" " * (SESSION.MAX_RECEIPT_BYTES + 1))
            with self.assertRaisesRegex(SESSION.SessionError, "bound"):
                SESSION.read_receipt(path)
            duplicate = Path(temporary) / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":2}', encoding="utf-8")
            with self.assertRaisesRegex(SESSION.SessionError, "repeats field"):
                SESSION.read_receipt(duplicate)

    def test_matrix_requires_all_targets_profiles_and_unique_passes(self) -> None:
        profiles = ("xbox", "playstation", "generic", "xbox")
        receipts = [
            sample_receipt(
                target=target,
                profile=profile,
                controller_name=f"Physical {profile} controller {index}",
            )
            for index, (target, profile) in enumerate(
                zip(sorted(SESSION.MATRIX_TARGETS), profiles, strict=True)
            )
        ]
        matrix = SESSION.validate_matrix(receipts)
        self.assertEqual(matrix["result"], "pass")
        self.assertEqual(matrix["sessionCount"], 4)

        with self.assertRaisesRegex(SESSION.SessionError, "release target"):
            SESSION.validate_matrix(receipts[:-1])
        with self.assertRaisesRegex(SESSION.SessionError, "duplicate"):
            SESSION.validate_matrix(receipts + [receipts[0]])

        one_model = [
            sample_receipt(
                target=receipt["release"]["target"],
                profile=receipt["controller"]["legendProfile"],
                controller_name="One Controller",
            )
            for receipt in receipts
        ]
        with self.assertRaisesRegex(SESSION.SessionError, "multiple profiles"):
            SESSION.validate_matrix(one_model)

        mixed_release = copy.deepcopy(receipts)
        mixed_release[0]["release"]["version"] = "0.2.0-alpha.5"
        mixed_release[0]["automated"]["cliMcpVersion"] = "0.2.0-alpha.5"
        mixed_release[0] = reidentify(mixed_release[0])
        with self.assertRaisesRegex(SESSION.SessionError, "release identities"):
            SESSION.validate_matrix(mixed_release)

        failed = list(receipts)
        failed[0] = sample_receipt(
            "fail",
            target=failed[0]["release"]["target"],
            profile=failed[0]["controller"]["legendProfile"],
        )
        with self.assertRaisesRegex(SESSION.SessionError, "failed receipt"):
            SESSION.validate_matrix(failed)

    def test_receipt_json_types_and_controller_names_are_strict(self) -> None:
        mutations = (
            ("schemaVersion", True, "schema"),
            ("schemaVersion", 1.0, "schema"),
            ("result", [], "result"),
        )
        for field, value, message in mutations:
            receipt = sample_receipt()
            receipt[field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaisesRegex(SESSION.SessionError, message):
                    SESSION.validate_receipt(reidentify(receipt))

        malformed_target = sample_receipt()
        malformed_target["release"]["target"] = []
        with self.assertRaisesRegex(SESSION.SessionError, "target"):
            SESSION.validate_receipt(reidentify(malformed_target))

        malformed_profile = sample_receipt()
        malformed_profile["controller"]["legendProfile"] = []
        with self.assertRaisesRegex(SESSION.SessionError, "profile"):
            SESSION.validate_receipt(reidentify(malformed_profile))

        blank_name = sample_receipt()
        blank_name["controller"]["name"] = "   "
        with self.assertRaisesRegex(SESSION.SessionError, "canonical"):
            SESSION.validate_receipt(reidentify(blank_name))

    def test_persistence_requires_positive_equal_values_and_exact_notes(self) -> None:
        zero = sample_receipt()
        zero["persistence"]["beforeExit"]["xp"] = 0
        zero["persistence"]["afterRestart"]["xp"] = 0
        zero["observations"][-2]["observation"] = "JOURNEY LV 2 | 0 XP"
        zero["observations"][-1]["observation"] = "JOURNEY LV 2 | 0 XP"
        with self.assertRaisesRegex(SESSION.SessionError, "persistence mutation"):
            SESSION.validate_receipt(reidentify(zero))

        changed = sample_receipt()
        changed["persistence"]["afterRestart"]["xp"] = 6
        changed["observations"][-1]["observation"] = "JOURNEY LV 2 | 6 XP"
        with self.assertRaisesRegex(SESSION.SessionError, "persistence mutation"):
            SESSION.validate_receipt(reidentify(changed))


if __name__ == "__main__":
    unittest.main(verbosity=2)
