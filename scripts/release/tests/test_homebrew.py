"""Casks consume verified macOS archives, not guessed install layouts."""

import hashlib
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from common import payload_name
from homebrew import render_cask
from package import refresh_documents
from payload_fixture import MACOS, REPO, PayloadFixture


class HomebrewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="homebrew cask ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.fixture = PayloadFixture(self.root, MACOS)
        self.fixture.stage()
        self.release = self.fixture.release
        self.manifest = json.loads((self.fixture.staging / "release.json").read_text())
        self.manifest.update(signing_mode="ad-hoc", runtime_audit={
            "system": "macos", "architecture": "arm64", "minimum_macos": "14.0.0",
        })
        library = self.fixture.staging / "lib/libcrypto.3.dylib"
        library.parent.mkdir()
        library.write_bytes(b"relocated CLI library fixture")
        self.plist_minimum = "14.0.0"

    def pack(self):
        refresh_documents(self.fixture.staging, self.manifest)
        (self.fixture.staging / "Manyhands.app/Contents/Info.plist").write_bytes(
            plistlib.dumps({"LSMinimumSystemVersion": self.plist_minimum}))
        self.archive = self.root / f"{payload_name(self.release, MACOS)}.zip"
        with zipfile.ZipFile(self.archive, "w") as packed:
            for path in self.fixture.staging.rglob("*"):
                if path.is_file():
                    packed.write(path, f"{payload_name(self.release, MACOS)}/{path.relative_to(self.fixture.staging).as_posix()}")
        return self.archive

    def render(self, **kwargs):
        return render_cask(self.release, self.pack(), "example/repo", **kwargs)

    def test_version_checksum_and_published_url_come_from_the_archive(self):
        cask = self.render()
        self.assertIn('cask "manyhands" do', cask)
        self.assertIn('version "0.1.0"', cask)
        self.assertIn(f'sha256 "{hashlib.sha256(self.archive.read_bytes()).hexdigest()}"', cask)
        expanded = cask.replace("#{version}", self.release["version"])
        self.assertIn(f'https://github.com/example/repo/releases/download/{self.release["tag"]}/{self.archive.name}', expanded)
        self.assertIn('homepage "https://github.com/example/repo"', cask)

    def test_app_and_cli_paths_exist_and_cli_keeps_its_sibling_library(self):
        cask = self.render().replace("#{version}", self.release["version"])
        app = re.search(r'^  app "([^"]+)"$', cask, re.M).group(1)
        cli = re.search(r'^  binary "([^"]+)"$', cask, re.M).group(1)
        with zipfile.ZipFile(self.archive) as packed:
            names = packed.namelist()
        self.assertIn(app + "/Contents/MacOS/manyhands", names)
        self.assertIn(app + "/Contents/Resources/THIRD-PARTY-NOTICES.html", names)
        self.assertIn(cli, names)
        self.assertIn(str(Path(cli).parent.parent / "lib/libcrypto.3.dylib"), names)
        self.assertIn('depends_on arch: :arm64', cask)
        self.assertNotIn('depends_on formula: "git"', cask)
        self.assertIn('depends_on macos: :sonoma', cask)

    def test_minimum_os_is_derived_and_unknown_or_finer_minimums_fail_closed(self):
        for minimum, requirement in (("11.0.0", "depends_on :macos"),
                                     ("12.0.0", "depends_on macos: :monterey"),
                                     ("15.0.0", "depends_on macos: :sequoia"),
                                     ("26.0.0", "depends_on macos: :tahoe")):
            with self.subTest(minimum=minimum):
                self.manifest["runtime_audit"]["minimum_macos"] = minimum
                self.plist_minimum = minimum
                self.assertIn(requirement, self.render())
        for minimum in (None, "", "14.4.0", "14.0.1", "99.0.0", "14.x.0", '14.0.0\";system(\"false\")'):
            with self.subTest(minimum=minimum):
                self.manifest["runtime_audit"]["minimum_macos"] = minimum
                self.plist_minimum = minimum or "14.0.0"
                with self.assertRaisesRegex(ValueError, "minimum macOS"):
                    self.render()

    def test_plist_and_manifest_minimums_must_agree(self):
        self.plist_minimum = "15.0.0"
        with self.assertRaisesRegex(ValueError, "minimum macOS"):
            self.render()

    def test_incomplete_unsigned_wrong_architecture_or_wrong_source_payload_is_rejected(self):
        for field, value in (("signing_mode", "pending"), ("commit", "b" * 40)):
            with self.subTest(field=field):
                original = self.manifest[field]
                self.manifest[field] = value
                with self.assertRaises(ValueError):
                    self.render()
                self.manifest[field] = original
        self.manifest["runtime_audit"]["architecture"] = "x86_64"
        with self.assertRaisesRegex(ValueError, "arm64"):
            self.render()
        self.manifest["runtime_audit"]["architecture"] = "arm64"
        (self.fixture.staging / "bin/manyhands-cli").unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            self.render()

    def test_repository_cannot_inject_ruby_or_url_components(self):
        archive = self.pack()
        for repo in ('example/repo\"', 'example/#{system("false")}', "../repo", "owner/..", "https://github.com/owner/repo", "owner/repo/extra"):
            with self.subTest(repo=repo), self.assertRaisesRegex(ValueError, "repository"):
                render_cask(self.release, archive, repo)

    def test_unresolved_template_variable_fails_before_emitting_a_cask(self):
        with self.assertRaisesRegex(ValueError, "template"):
            self.render(template="$UNDEFINED")

    def test_cask_does_not_remove_user_data_or_bypass_platform_security(self):
        cask = self.render()
        self.assertIn("ad-hoc", cask)
        self.assertIn("notarized", cask)
        for forbidden in ("zap ", "uninstall ", "xattr", "spctl", "no-quarantine", "system_command"):
            self.assertNotIn(forbidden, cask)

    def test_prerelease_version_is_not_truncated(self):
        self.release.update(tag="v0.2.0-rc.1", version="0.2.0-rc.1", prerelease=True)
        self.manifest.update(self.release)
        cask = self.render()
        self.assertIn('version "0.2.0-rc.1"', cask)
        expanded = cask.replace("#{version}", self.release["version"])
        self.assertIn("/releases/download/v0.2.0-rc.1/", expanded)

    def test_cli_renders_to_a_path_with_spaces_and_leaves_no_file_on_failure(self):
        archive = self.pack()
        release_input = self.root / "release input.json"
        release_input.write_text(json.dumps(self.release))
        output = self.root / "generated manyhands.rb"
        command = [sys.executable, "-B", str(REPO / "scripts/release/homebrew.py"),
                   "--input", str(release_input), "--archive", str(archive),
                   "--repo", "example/repo", "--output", str(output)]
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output.read_text(), render_cask(self.release, archive, "example/repo"))
        output.unlink()
        archive.write_bytes(b"not a zip")
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("release:", result.stderr)
        self.assertFalse(output.exists())

    @unittest.skipUnless(shutil.which("ruby"), "Ruby syntax check needs Ruby; native Homebrew checks are separate")
    def test_generated_cask_has_valid_ruby_syntax(self):
        result = subprocess.run(["ruby", "-c"], input=self.render(), text=True, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
