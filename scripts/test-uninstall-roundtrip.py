#!/usr/bin/env python3
"""Focused regressions for the uninstall roundtrip contract.

These cover the parts that decide pass or fail without needing an install, so a
mistake in the judgment is caught even when no packaged archive is around. The
roundtrip itself is exercised by `uninstall-roundtrip.py` in the release gate.
"""

from __future__ import annotations

import importlib.util
import json
import os
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
from pathlib import Path
from typing import cast
from unittest import mock

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

    def test_seeded_state_completes_the_full_preservation_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            profile = Path(raw)
            journey = profile / ".numinous-journey"
            scores = profile / ".numinous-scores"
            journey.write_text("visited lorenz\n", encoding="utf-8")
            scores.write_text("munch\t7\n", encoding="utf-8")
            MODULE.seed_state_not_reached_by_roundtrip(profile)
            state = MODULE.player_state(profile)
            self.assertEqual(set(state), set(MODULE.PLAYER_STATE))
            self.assertEqual(journey.read_text(encoding="utf-8"), "visited lorenz\n")
            self.assertEqual(scores.read_text(encoding="utf-8"), "munch\t7\n")

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
            first_module = patched["PSModulePath"].split(os.pathsep)[0]
            self.assertTrue(
                first_module.lower().endswith(
                    os.path.join("windowspowershell", "v1.0", "modules")
                ),
                f"first PowerShell module directory was {first_module}",
            )
            self.assertIn("/usr/bin", patched["PATH"])
        else:
            self.assertEqual(patched, {"PATH": "/usr/bin"})

    def test_the_original_environment_is_not_mutated(self) -> None:
        original = {"PATH": "/usr/bin"}
        MODULE.native_tool_env(original)
        self.assertEqual(original, {"PATH": "/usr/bin"})


def _stop_recorded_process(pid_file: Path) -> None:
    if not pid_file.is_file():
        return
    text = pid_file.read_text(encoding="ascii").strip()
    if not text.isdecimal():
        return
    try:
        os.kill(int(text), signal.SIGTERM)
    except OSError:
        return


def _holder_script(exit_code: int) -> str:
    """Print, leave a child holding stdout, record that child's pid, and exit.

    The child inherits this process's standard output and outlives it. That is
    the shape of the maintenance helper inheriting a capturing parent's pipes.
    """
    return (
        "import subprocess, sys\n"
        "from pathlib import Path\n"
        "print('command-returned', flush=True)\n"
        "child = subprocess.Popen(\n"
        "    [sys.executable, '-c', 'import time; time.sleep(30)']\n"
        ")\n"
        "Path(sys.argv[1]).write_text(str(child.pid), encoding='ascii')\n"
        f"raise SystemExit({exit_code})\n"
    )


class ProcessRunnerTests(unittest.TestCase):
    @mock.patch.object(MODULE.subprocess, "run")
    def test_each_step_can_choose_a_shorter_timeout(self, runner: mock.Mock) -> None:
        runner.return_value = subprocess_result = MODULE.subprocess.CompletedProcess(
            args=["bounded"], returncode=0, stdout="finished\n", stderr=""
        )
        output = MODULE.run(
            ["bounded"], {"PATH": ""}, "bounded step", timeout_seconds=7.0
        )
        self.assertEqual(output, subprocess_result.stdout)
        self.assertEqual(runner.call_args.kwargs["timeout"], 7.0)

    @mock.patch.object(MODULE.subprocess, "run")
    def test_timeout_is_reported_as_a_roundtrip_failure(self, runner: mock.Mock) -> None:
        runner.side_effect = MODULE.subprocess.TimeoutExpired(["stuck"], 3.0)
        with self.assertRaisesRegex(
            MODULE.RoundtripError, "stuck step exceeded 3 seconds"
        ):
            MODULE.run(["stuck"], {"PATH": ""}, "stuck step", timeout_seconds=3.0)

    def test_a_spawned_child_keeps_stdout_open_after_its_parent_exits(self) -> None:
        # The tests below only lock the gate if this process really does keep
        # the pipe open. A fast pass against a pipe that already closed would
        # say nothing.
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            script = directory / "holder.py"
            pid_file = directory / "holder.pid"
            script.write_text(_holder_script(0), encoding="utf-8")
            process = subprocess.Popen(
                [sys.executable, str(script), str(pid_file)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            try:
                process.wait(timeout=8)
                self.assertEqual(process.returncode, 0)
                stdout = process.stdout
                assert stdout is not None
                os.set_blocking(stdout.fileno(), False)
                collected = bytearray()
                held = False
                for _ in range(20):
                    try:
                        chunk: object = stdout.read(65536)
                    except BlockingIOError:
                        held = True
                        break
                    if chunk is None:
                        held = True
                        break
                    if chunk == b"":
                        self.fail(
                            "stdout closed when the spawning process exited, "
                            "so the child is not holding the pipe"
                        )
                    if not isinstance(chunk, bytes):
                        self.fail(f"unexpected read result {chunk!r}")
                    collected.extend(chunk)
                self.assertTrue(held, "stdout did not stay open")
                self.assertIn(b"command-returned", bytes(collected))
            finally:
                _stop_recorded_process(pid_file)
                try:
                    process.kill()
                except OSError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
                if process.stdout is not None:
                    process.stdout.close()
                if process.stderr is not None:
                    process.stderr.close()

    def test_a_helper_holding_stdout_does_not_delay_process_exit(self) -> None:
        # The maintenance helper inherits a capturing parent's pipes and
        # outlives `numinous uninstall`. The gate has to notice the command
        # returning, with the text it printed, then judge the helper apart.
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            script = directory / "holder.py"
            pid_file = directory / "holder.pid"
            script.write_text(_holder_script(0), encoding="utf-8")
            started = time.monotonic()
            try:
                output = MODULE.run_until_the_process_exits(
                    [sys.executable, str(script), str(pid_file)],
                    {"PATH": os.environ.get("PATH", "")},
                    "held pipes",
                    8.0,
                )
                elapsed = time.monotonic() - started
            finally:
                _stop_recorded_process(pid_file)
        self.assertLess(elapsed, 6.0)
        self.assertIn("command-returned", output)

    def test_failure_text_is_visible_while_a_child_holds_the_pipe(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            script = directory / "holder.py"
            pid_file = directory / "holder.pid"
            script.write_text(_holder_script(3), encoding="utf-8")
            started = time.monotonic()
            try:
                with self.assertRaises(MODULE.RoundtripError) as caught:
                    MODULE.run_until_the_process_exits(
                        [sys.executable, str(script), str(pid_file)],
                        {"PATH": os.environ.get("PATH", "")},
                        "held pipes",
                        8.0,
                    )
                elapsed = time.monotonic() - started
            finally:
                _stop_recorded_process(pid_file)
        message = str(caught.exception)
        self.assertLess(elapsed, 6.0)
        self.assertIn("exit 3", message)
        self.assertIn("command-returned", message)

    def test_a_process_that_does_not_exit_is_still_bounded(self) -> None:
        started = time.monotonic()
        with self.assertRaisesRegex(
            MODULE.RoundtripError, "still running exceeded 1 seconds"
        ):
            MODULE.run_until_the_process_exits(
                [sys.executable, "-c", "import time; time.sleep(30)"],
                {"PATH": os.environ.get("PATH", "")},
                "still running",
                1.0,
            )
        self.assertLess(time.monotonic() - started, 5.0)


class WindowsRegistrationTests(unittest.TestCase):
    def test_other_platforms_do_not_touch_the_registry(self) -> None:
        with (
            mock.patch.object(MODULE.platform, "system", return_value="Linux"),
            mock.patch.object(MODULE, "_reg") as reg,
        ):
            saved = MODULE.SavedWindowsRegistration(r"HKCU\Software\Example")
            saved.capture()
            saved.restore()
        reg.assert_not_called()

    def test_a_missing_key_is_deleted_again_on_restore(self) -> None:
        calls: list[list[str]] = []

        def fake_reg(args: list[str]) -> subprocess.CompletedProcess[str]:
            calls.append(list(args))
            if args[0] == "query":
                return subprocess.CompletedProcess(
                    args,
                    1,
                    "",
                    "ERROR: The system was unable to find "
                    "the specified registry key or value.",
                )
            return subprocess.CompletedProcess(args, 0, "", "")

        with (
            mock.patch.object(MODULE.platform, "system", return_value="Windows"),
            mock.patch.object(MODULE, "_reg", side_effect=fake_reg),
        ):
            saved = MODULE.SavedWindowsRegistration(r"HKCU\Software\Example")
            saved.capture()
            saved.restore()
        self.assertEqual(calls[0][:2], ["query", r"HKCU\Software\Example"])
        self.assertEqual(calls[1], ["delete", r"HKCU\Software\Example", "/f"])

    def test_an_exported_key_is_imported_unchanged(self) -> None:
        blob = b"Windows Registry Editor Version 5.00\r\n"
        imported: list[bytes] = []

        def fake_reg(args: list[str]) -> subprocess.CompletedProcess[str]:
            if args[0] == "query":
                return subprocess.CompletedProcess(args, 0, "HKEY", "")
            if args[0] == "export":
                Path(args[2]).write_bytes(blob)
                return subprocess.CompletedProcess(args, 0, "", "")
            if args[0] == "import":
                imported.append(Path(args[1]).read_bytes())
                return subprocess.CompletedProcess(args, 0, "", "")
            return subprocess.CompletedProcess(args, 1, "", "unexpected")

        with (
            mock.patch.object(MODULE.platform, "system", return_value="Windows"),
            mock.patch.object(MODULE, "_reg", side_effect=fake_reg),
        ):
            saved = MODULE.SavedWindowsRegistration(r"HKCU\Software\Example")
            saved.capture()
            saved.restore()
        self.assertEqual(imported, [blob])

    def test_a_read_failure_is_not_treated_as_absence(self) -> None:
        def fake_reg(args: list[str]) -> subprocess.CompletedProcess[str]:
            return subprocess.CompletedProcess(args, 1, "", "ERROR: Access is denied.")

        with (
            mock.patch.object(MODULE.platform, "system", return_value="Windows"),
            mock.patch.object(MODULE, "_reg", side_effect=fake_reg),
        ):
            saved = MODULE.SavedWindowsRegistration(r"HKCU\Software\Example")
            with self.assertRaises(MODULE.RoundtripError):
                saved.capture()
        self.assertFalse(saved._captured)

    @unittest.skipUnless(
        platform.system() == "Windows", "registry roundtrip is Windows-only"
    )
    def test_a_private_key_roundtrips_through_reg(self) -> None:
        key = r"HKCU\Software\NuminousRoundtripTest" + uuid.uuid4().hex
        subprocess.run(["reg", "delete", key, "/f"], capture_output=True, check=False)
        try:
            missing = MODULE.SavedWindowsRegistration(key)
            missing.capture()
            subprocess.run(
                ["reg", "add", key, "/v", "DisplayName", "/d", "temporary", "/f"],
                check=True,
                capture_output=True,
                text=True,
            )
            missing.restore()
            probe = subprocess.run(
                ["reg", "query", key], capture_output=True, text=True, check=False
            )
            self.assertNotEqual(probe.returncode, 0)

            subprocess.run(
                ["reg", "add", key, "/v", "DisplayName", "/d", "kept", "/f"],
                check=True,
                capture_output=True,
                text=True,
            )
            present = MODULE.SavedWindowsRegistration(key)
            present.capture()
            subprocess.run(
                ["reg", "add", key, "/v", "DisplayName", "/d", "changed", "/f"],
                check=True,
                capture_output=True,
                text=True,
            )
            present.restore()
            shown = subprocess.run(
                ["reg", "query", key, "/v", "DisplayName"],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("kept", shown.stdout)
            self.assertNotIn("changed", shown.stdout)
        finally:
            subprocess.run(["reg", "delete", key, "/f"], capture_output=True, check=False)


class IsolatedProfileEnvTests(unittest.TestCase):
    def test_player_overrides_and_launcher_roots_are_confined(self) -> None:
        original = {
            "PATH": "/usr/bin",
            "XDG_DATA_HOME": "/real/data",
            "APPDATA": "/real/roaming",
            "LOCALAPPDATA": "/real/local",
            "NUMINOUS_JOURNAL": "/real/journal",
        }
        profile = Path("isolated-profile")
        install = Path("isolated-install")
        patched = MODULE.isolated_profile_env(original, profile, install)
        self.assertEqual(patched["NUMINOUS_HOME"], str(install))
        self.assertEqual(patched["HOME"], str(profile))
        self.assertEqual(
            patched["XDG_DATA_HOME"], str(profile / ".local" / "share")
        )
        self.assertNotIn("NUMINOUS_JOURNAL", patched)
        if platform.system() == "Windows":
            self.assertEqual(
                patched["APPDATA"], str(profile / "AppData" / "Roaming")
            )
            self.assertEqual(
                patched["LOCALAPPDATA"], str(profile / "AppData" / "Local")
            )
        else:
            self.assertEqual(patched["APPDATA"], "/real/roaming")
            self.assertEqual(patched["LOCALAPPDATA"], "/real/local")
        self.assertEqual(original["XDG_DATA_HOME"], "/real/data")


class LauncherArtifactTests(unittest.TestCase):
    def test_launchers_stay_inside_the_isolated_profile(self) -> None:
        profile = Path("profile")
        launchers = MODULE.launcher_artifacts(profile)
        self.assertTrue(launchers)
        for launcher in launchers:
            self.assertEqual(launcher.parts[0], profile.name)

    def test_a_dangling_shortcut_still_counts_as_an_artifact(self) -> None:
        if platform.system() == "Windows":
            self.skipTest("creating symbolic links is not a stable Windows test contract")
        with tempfile.TemporaryDirectory() as raw:
            link = Path(raw) / "Numinous"
            link.symlink_to(Path(raw) / "missing")
            self.assertTrue(MODULE.path_or_link_exists(link))


class PrivateWorkspaceTests(unittest.TestCase):
    def test_other_platforms_do_not_invoke_the_installer(self) -> None:
        with (
            mock.patch.object(MODULE.platform, "system", return_value="Linux"),
            mock.patch.object(MODULE, "run") as runner,
        ):
            MODULE.protect_private_workspace(Path("workspace"))
        runner.assert_not_called()

    @unittest.skipUnless(platform.system() == "Windows", "the ACL walk is Windows-only")
    def test_protect_removes_a_replacement_grant(self) -> None:
        # The same place the roundtrip uses: a new directory under the profile,
        # not a shared temp folder. The grant is the right the ancestor check
        # refuses. Afterwards the directory must be one the check can accept.
        workspace = Path(tempfile.mkdtemp(
            prefix=".numinous-protect-test-", dir=Path.home()
        ))
        try:
            self._grant_everyone_delete(workspace)
            before = self._replacement_report(workspace)
            self.assertTrue(before["bad"], f"the planted grant did not stick: {before}")
            MODULE.protect_private_workspace(workspace)
            after = self._replacement_report(workspace)
        finally:
            shutil.rmtree(workspace, ignore_errors=True)
        self.assertTrue(after["protected"], after)
        self.assertEqual(after["owner"], after["current"], after)
        self.assertEqual(after["bad"], [], after)

    @unittest.skipUnless(platform.system() == "Windows", "the ACL walk is Windows-only")
    def test_protect_refuses_to_become_an_install(self) -> None:
        completed = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(ROOT / "scripts" / "install.ps1"),
                "-ProtectDirectory",
                str(Path.home()),
                "-Uninstall",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=MODULE.native_tool_env(dict(os.environ)),
        )
        self.assertNotEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        self.assertIn("takes no other", completed.stdout + completed.stderr)

    def _powershell(self, script: str, path: Path) -> dict[str, object]:
        completed = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=MODULE.native_tool_env({**os.environ, "NUMINOUS_PROTECT_PATH": str(path)}),
        )
        if completed.returncode != 0:
            self.fail(completed.stdout + completed.stderr)
        parsed = json.loads(completed.stdout)
        if not isinstance(parsed, dict):
            self.fail(f"expected an access report object, got {parsed!r}")
        return cast(dict[str, object], parsed)

    def _grant_everyone_delete(self, path: Path) -> None:
        completed = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                r"""
                $ErrorActionPreference = 'Stop'
                $path = $env:NUMINOUS_PROTECT_PATH
                $everyone = New-Object Security.Principal.SecurityIdentifier('S-1-1-0')
                $acl = Get-Acl -LiteralPath $path
                $rule = New-Object Security.AccessControl.FileSystemAccessRule(
                    $everyone,
                    [Security.AccessControl.FileSystemRights]::Delete,
                    [Security.AccessControl.AccessControlType]::Allow)
                $acl.AddAccessRule($rule)
                if ($PSVersionTable.PSEdition -eq 'Core') {
                    [IO.FileSystemAclExtensions]::SetAccessControl((Get-Item -LiteralPath $path), $acl)
                } else {
                    [IO.Directory]::SetAccessControl($path, $acl)
                }
                """,
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=MODULE.native_tool_env({**os.environ, "NUMINOUS_PROTECT_PATH": str(path)}),
        )
        if completed.returncode != 0:
            self.fail(completed.stdout + completed.stderr)

    def _replacement_report(self, path: Path) -> dict[str, object]:
        # The same trusted identities and replacement rights as
        # Assert-PrivateInstallAncestors. This probe does not decide the
        # install; it shows the grant the installer itself refuses.
        return self._powershell(
            r"""
            $ErrorActionPreference = 'Stop'
            $path = $env:NUMINOUS_PROTECT_PATH
            $acl = Get-Acl -LiteralPath $path
            $trusted = @(
                [Security.Principal.WindowsIdentity]::GetCurrent().User.Value,
                'S-1-5-18',
                'S-1-5-32-544',
                'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464'
            )
            $replacement = [int](
                [Security.AccessControl.FileSystemRights]::Delete -bor
                [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
                [Security.AccessControl.FileSystemRights]::ChangePermissions -bor
                [Security.AccessControl.FileSystemRights]::TakeOwnership -bor
                0x10000000
            )
            $inheritOnly = [int][Security.AccessControl.PropagationFlags]::InheritOnly
            $bad = New-Object System.Collections.Generic.List[string]
            foreach ($rule in $acl.GetAccessRules(
                $true, $true, [Security.Principal.SecurityIdentifier])) {
                $rights = [int]$rule.FileSystemRights
                $propagation = [int]$rule.PropagationFlags
                if ($rule.AccessControlType -eq 'Allow' -and
                    (($propagation -band $inheritOnly) -eq 0) -and
                    ($rule.IdentityReference.Value -notin $trusted) -and
                    (($rights -band $replacement) -ne 0)) {
                    $bad.Add($rule.IdentityReference.Value)
                }
            }
            @{
                protected = [bool]$acl.AreAccessRulesProtected
                owner = $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value
                current = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
                bad = @($bad)
            } | ConvertTo-Json -Compress
            """,
            path,
        )


if __name__ == "__main__":
    unittest.main()
