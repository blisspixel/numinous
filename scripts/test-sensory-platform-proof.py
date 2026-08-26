#!/usr/bin/env python3
"""The App direct-surface proof stays cross-platform and claim-safe."""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
APP_MANIFEST = ROOT / "faces" / "app" / "Cargo.toml"


class SensoryPlatformProofTests(unittest.TestCase):
    def setUp(self) -> None:
        self.workflow = WORKFLOW.read_text(encoding="utf-8")
        self.manifest = APP_MANIFEST.read_text(encoding="utf-8")

    def test_probe_target_requires_the_disabled_feature(self):
        self.assertIn('name = "sensory_platform"', self.manifest)
        self.assertIn('required-features = ["gpu-post"]', self.manifest)

    def test_the_existing_build_matrix_owns_all_three_runtime_proofs(self):
        self.assertIn("os: [ubuntu-latest, macos-latest, windows-latest]", self.workflow)
        self.assertIn("Linux direct App surface proof", self.workflow)
        self.assertIn("Native direct App surface proof", self.workflow)
        commands = [
            line.strip()
            for line in self.workflow.splitlines()
            if "--example sensory_platform" in line
        ]
        self.assertEqual(len(commands), 2, "expected Linux and native probe commands")
        for command in commands:
            self.assertIn("--check", command)
            self.assertNotIn(
                "--physical",
                command,
                "hosted CI must not claim physical pacing authority",
            )

    def test_linux_has_a_window_server_and_vulkan_runtime(self):
        self.assertIn("mesa-vulkan-drivers xvfb", self.workflow)
        self.assertIn("WGPU_BACKEND: vulkan", self.workflow)
        self.assertIn("xvfb-run -a cargo run", self.workflow)

    def test_each_platform_retains_a_failure_receipt(self):
        marker = "- name: Retain App surface proof"
        start = self.workflow.find(marker)
        self.assertNotEqual(start, -1)
        block = self.workflow[start : start + 500]
        self.assertIn("if: always()", block)
        self.assertIn("name: sensory-app-platform-${{ runner.os }}", block)
        self.assertIn("path: sensory-app-platform.json", block)
        self.assertIn("if-no-files-found: error", block)


if __name__ == "__main__":
    unittest.main()
