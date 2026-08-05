#!/usr/bin/env python3
"""Focused regressions for the uninstall roundtrip contract.

These cover the parts that decide pass or fail without needing an install, so a
mistake in the judgement is caught even when no packaged archive is around. The
roundtrip itself is exercised by `uninstall-roundtrip.py` in the release gate.
"""

from __future__ import annotations

import importlib.util
import os
import platform
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "uninstall_roundtrip", ROOT / "scripts" / "uninstall-roundtrip.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PlayerStateTests(unittest.TestCase):
    def test_only_existing_player_files_are_hashed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            profile = Path(raw)
            (profile / ".numinous-journey").write_text("lv 2", encoding="utf-8")
            state = MODULE.player_state(profile)
        self.assertEqual(sorted(state), [".numinous-journey"])

    def test_a_changed_byte_changes_the_hash(self) -> None:
        # The whole point of hashing rather than checking presence: an
        # uninstall that rewrote the player's history would still leave a file
        # there, and presence alone would call that a pass.
        with tempfile.TemporaryDirectory() as raw:
            profile = Path(raw)
            journey = profile / ".numinous-journey"
            journey.write_text("lv 2", encoding="utf-8")
            before = MODULE.player_state(profile)
            journey.write_text("lv 3", encoding="utf-8")
            after = MODULE.player_state(profile)
        self.assertNotEqual(before[".numinous-journey"], after[".numinous-journey"])

    def test_an_empty_profile_hashes_to_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            self.assertEqual(MODULE.player_state(Path(raw)), {})

    def test_the_state_list_matches_what_the_uninstaller_promises(self) -> None:
        # The uninstaller tells the player which files it keeps. If that list
        # and this one drift apart, the gate stops covering the promise.
        for installer in ("install.ps1", "install.sh"):
            text = (ROOT / "scripts" / installer).read_text(encoding="utf-8")
            for name in MODULE.PLAYER_STATE:
                self.assertIn(
                    name,
                    text,
                    f"{installer} never mentions {name}, so the promise and the "
                    "gate may have drifted apart",
                )


class InstallerCommandTests(unittest.TestCase):
    SOUNDTRACK = (Path("s.tar.gz"), Path("s.tar.gz.sha256"), Path("s.tar.gz.content.sha256"))

    def test_install_never_modifies_path(self) -> None:
        # The roundtrip runs on a real machine, including a developer's. It
        # must not edit their PATH to test itself.
        for uninstall in (False, True):
            command = MODULE.installer_command(
                Path("a.zip"), Path("a.zip.sha256"), "v1.2.3", self.SOUNDTRACK, uninstall
            )
            joined = " ".join(command)
            self.assertTrue(
                "-NoModifyPath" in joined or "--no-modify-path" in joined,
                f"missing the no-modify-path switch: {joined}",
            )

    def test_uninstall_passes_no_archive(self) -> None:
        command = MODULE.installer_command(
            Path("a.zip"), Path("a.zip.sha256"), "v1.2.3", self.SOUNDTRACK, uninstall=True
        )
        joined = " ".join(command)
        self.assertNotIn("a.zip", joined)
        self.assertTrue(
            "-Uninstall" in command or "--uninstall" in command, joined
        )

    def test_install_carries_the_local_soundtrack(self) -> None:
        # Without these the installer downloads, and the gate stops being
        # hermetic and starts depending on a published release.
        command = MODULE.installer_command(
            Path("a.zip"), Path("a.zip.sha256"), "v1.2.3", self.SOUNDTRACK, uninstall=False
        )
        joined = " ".join(command)
        for part in self.SOUNDTRACK:
            self.assertIn(str(part), joined)


class NativeToolEnvTests(unittest.TestCase):
    def test_windows_puts_system32_first(self) -> None:
        patched = MODULE.native_tool_env({"PATH": "/usr/bin"})
        if platform.system() == "Windows":
            # Not every Windows installs on C:, so assert the shape rather than
            # the drive letter.
            first = patched["PATH"].split(os.pathsep)[0]
            self.assertTrue(
                first.lower().endswith(os.sep + "system32"),
                f"first PATH entry was {first}",
            )
            self.assertIn("/usr/bin", patched["PATH"])
        else:
            self.assertEqual(patched, {"PATH": "/usr/bin"})

    def test_the_original_environment_is_not_mutated(self) -> None:
        original = {"PATH": "/usr/bin"}
        MODULE.native_tool_env(original)
        self.assertEqual(original, {"PATH": "/usr/bin"})


if __name__ == "__main__":
    unittest.main()
