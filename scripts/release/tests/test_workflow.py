"""Shared workflow arguments must not depend on Bash/PowerShell syntax."""

import contextlib
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from workflow import command_for_stage, emit_tools, main


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="workflow arguments ")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.env = {"TARGET": "x86_64-pc-windows-msvc", "STAGING": "staging path with spaces",
                    "EXTRACTED": "extracted path with spaces", "ARCHIVE": "dist path/archive.zip",
                    "INVENTORY": "dist path/archive.zip.json", "GITHUB_ENV": "job environment"}

    def test_shared_paths_are_single_arguments_without_shell_expansion(self):
        command = command_for_stage("stage", self.env, self.repo)
        self.assertEqual(command[command.index("--output") + 1], self.env["STAGING"])
        self.assertEqual(command[command.index("--target") + 1], self.env["TARGET"])
        self.assertFalse(any("$env:" in argument or "$TARGET" in argument for argument in command))

    def test_cargo_builds_use_the_native_tool_and_production_features(self):
        cli = command_for_stage("build-cli", self.env, self.repo)
        desktop = command_for_stage("build-desktop", self.env, self.repo)
        self.assertEqual(cli[0], "cargo")
        self.assertIn("--release", cli)
        self.assertNotIn("--features", cli)
        self.assertEqual(desktop[desktop.index("--features") + 1], "desktop")
        self.assertNotIn("--all-features", desktop)

    def test_windows_signing_and_extracted_verification_use_powershell(self):
        for stage, mode, variable in (("windows-sign", "Sign", "STAGING"), ("windows-verify", "Verify", "EXTRACTED")):
            with self.subTest(stage=stage):
                command = command_for_stage(stage, self.env, self.repo)
                self.assertEqual(command[0], "pwsh")
                self.assertEqual(command[command.index("-Mode") + 1], mode)
                self.assertEqual(command[-1], self.env[variable])

    def test_handoff_does_not_resolve_the_tag_again(self):
        command = command_for_stage("prepare", self.env, self.repo)
        self.assertIn("--manifest-env", command)
        self.assertNotIn("--tag", command)
        self.assertEqual(command[command.index("--github-env") + 1], self.env["GITHUB_ENV"])

    def test_archive_verification_retains_smoke_and_inventory_checks(self):
        command = command_for_stage("verify", self.env, self.repo)
        self.assertIn("--smoke-cli", command)
        self.assertEqual(command[command.index("--archive") + 1], self.env["ARCHIVE"])
        self.assertEqual(command[command.index("--inventory") + 1], self.env["INVENTORY"])

    def test_invalid_tool_config_does_not_emit_partial_outputs(self):
        packaging = self.repo / "packaging"
        packaging.mkdir()
        (packaging / "tool-versions.env").write_text("RUST_VERSION=1.98.1\nCARGO_ABOUT_VERSION=not-a-version\n", encoding="utf-8")
        output = self.repo / "outputs"
        with self.assertRaisesRegex(ValueError, "version"):
            emit_tools(self.repo, output)
        self.assertFalse(output.exists())

    def test_missing_required_environment_fails_before_running_a_tool(self):
        with self.assertRaisesRegex(ValueError, "TARGET"):
            command_for_stage("stage", {}, self.repo)

    def test_pinned_tools_are_emitted_without_shell_specific_redirection(self):
        packaging = self.repo / "packaging"
        packaging.mkdir()
        (packaging / "tool-versions.env").write_text("# Pinned tools\nRUST_VERSION=1.98.1\nCARGO_ABOUT_VERSION=0.9.3\n", encoding="utf-8")
        output = self.repo / "outputs"
        emit_tools(self.repo, output)
        self.assertEqual(output.read_text(), "rust=1.98.1\ncargo-about=0.9.3\n")

    def test_failed_child_exit_code_is_preserved(self):
        with patch.dict(os.environ, self.env), patch("workflow.subprocess.run", side_effect=subprocess.CalledProcessError(23, ["cargo"])):
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(main(["build-cli"]), 23)


if __name__ == "__main__":
    unittest.main()
