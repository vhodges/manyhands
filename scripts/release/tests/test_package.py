import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from package import archive_payload
from payload_fixture import LINUX, MACOS, WINDOWS, PayloadFixture


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release packaging ")
        self.addCleanup(self.temp.cleanup)
        self.fixture = PayloadFixture(self.temp.name)

    def test_stages_both_frontends_and_readable_disclosures(self):
        self.fixture.stage()
        root = self.fixture.staging
        self.assertTrue((root / "bin/manyhands").is_file())
        self.assertTrue((root / "bin/manyhands-cli").is_file())
        self.assertIn(self.fixture.release["commit"], (root / "README.md").read_text())
        self.assertNotIn("$VERSION", (root / "README.md").read_text())
        self.assertIn("MIT License", (root / "LICENSE").read_text())
        self.assertIn("IBM Plex", (root / "licenses/fonts/README.md").read_text())
        manifest = json.loads((root / "release.json").read_text())
        self.assertEqual(manifest["target"], LINUX)
        self.assertEqual(manifest["signing_mode"], "none")
        self.assertEqual(manifest["compiler"], "rustc 1.98.1 (fixture)")

    def test_missing_cli_fails_without_leaving_a_payload(self):
        (self.fixture.binaries / "manyhands-cli").unlink()
        with self.assertRaisesRegex(ValueError, "manyhands-cli"):
            self.fixture.stage()
        self.assertFalse(self.fixture.staging.exists())

    def test_missing_font_licence_fails_without_leaving_a_payload(self):
        (self.fixture.notices / "licenses/fonts/LICENSE.txt").unlink()
        with self.assertRaisesRegex(ValueError, "LICENSE.txt"):
            self.fixture.stage()
        self.assertFalse(self.fixture.staging.exists())

    def test_missing_nested_notice_is_not_silently_dropped(self):
        (self.fixture.notices / "licenses/dependencies/example/COPYING").unlink()
        with self.assertRaisesRegex(ValueError, "COPYING"):
            self.fixture.stage()

    def test_existing_output_is_not_overwritten(self):
        self.fixture.staging.mkdir()
        marker = self.fixture.staging / "owned by user"
        marker.write_text("keep")
        with self.assertRaisesRegex(ValueError, "exist"):
            self.fixture.stage()
        self.assertEqual(marker.read_text(), "keep")

    def test_notices_for_a_different_target_are_rejected(self):
        inventory = self.fixture.notices / "licenses/inventory.json"
        data = json.loads(inventory.read_text())
        data["target"] = WINDOWS
        inventory.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "target"):
            self.fixture.stage()

    def test_private_signing_material_cannot_enter_payload(self):
        for name, content in (
            ("signing.pfx", b"private certificate fixture"),
            ("credentials.txt", b"-----BEGIN PRIVATE KEY-----\nfixture\n-----END PRIVATE KEY-----\n"),
        ):
            with self.subTest(name=name):
                secret = self.fixture.notices / name
                secret.write_bytes(content)
                with self.assertRaisesRegex(ValueError, "private"):
                    self.fixture.stage()
                self.assertFalse(self.fixture.staging.exists())
                secret.unlink()

    def test_inconsistent_release_identity_fails(self):
        self.fixture.release["version"] = "0.2.0"
        with self.assertRaisesRegex(ValueError, "version"):
            self.fixture.stage()


class PlatformLayoutTests(unittest.TestCase):
    def test_windows_requires_exe_suffixes(self):
        with tempfile.TemporaryDirectory() as temp:
            fixture = PayloadFixture(temp, WINDOWS)
            (fixture.binaries / "manyhands-cli.exe").rename(fixture.binaries / "manyhands-cli")
            with self.assertRaisesRegex(ValueError, "manyhands-cli.exe"):
                fixture.stage()

    def test_unsigned_windows_payload_cannot_be_archived(self):
        with tempfile.TemporaryDirectory() as temp:
            fixture = PayloadFixture(temp, WINDOWS)
            fixture.stage()
            with self.assertRaisesRegex(ValueError, "signing"):
                archive_payload(fixture.release, WINDOWS, fixture.staging, fixture.dist)

    def test_macos_bundle_retains_notices_after_app_is_moved(self):
        with tempfile.TemporaryDirectory() as temp:
            fixture = PayloadFixture(temp, MACOS)
            fixture.stage()
            app = fixture.staging / "Manyhands.app/Contents"
            self.assertTrue((app / "MacOS/manyhands").is_file())
            for relative in ("LICENSE", "README.md", "release.json", "THIRD-PARTY-NOTICES.html",
                             "licenses/fonts/LICENSE.txt", "licenses/fonts/README.md"):
                self.assertTrue((app / "Resources" / relative).is_file(), relative)


if __name__ == "__main__":
    unittest.main()
