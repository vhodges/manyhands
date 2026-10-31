import io
import json
import os
from pathlib import Path
import stat
import sys
import tarfile
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from package import archive_payload
from verify import extract_archive, smoke_environment, verify_archive
from payload_fixture import LINUX, MACOS, WINDOWS, PayloadFixture


class VerificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release verification ")
        self.addCleanup(self.temp.cleanup)
        self.fixture = PayloadFixture(self.temp.name)
        self.fixture.stage()

    def archive(self):
        return archive_payload(
            self.fixture.release, LINUX, self.fixture.staging, self.fixture.dist,
        )

    def test_linux_round_trip_preserves_execute_bits_and_smokes_extracted_cli(self):
        archive = self.archive()
        result = verify_archive(archive, self.fixture.release, LINUX, smoke_cli=True)
        self.assertEqual(result["commit"], self.fixture.release["commit"])
        with tarfile.open(archive) as packed:
            binary = packed.getmember(f"{archive.name.removesuffix('.tar.gz')}/bin/manyhands-cli")
            self.assertTrue(binary.mode & stat.S_IXUSR)

    def test_missing_execute_permission_is_rejected(self):
        (self.fixture.staging / "bin/manyhands-cli").chmod(0o644)
        with self.assertRaisesRegex(ValueError, "execut"):
            self.archive()

    def test_changed_package_identity_is_rejected(self):
        path = self.fixture.staging / "release.json"
        original = json.loads(path.read_text())
        for field, value in (("target", WINDOWS), ("commit", "b" * 40), ("version", "0.2.0")):
            data = {**original, field: value}
            path.write_text(json.dumps(data))
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.archive()

    def test_missing_required_notice_is_rejected(self):
        (self.fixture.staging / "THIRD-PARTY-NOTICES.html").unlink()
        with self.assertRaisesRegex(ValueError, "NOTICES"):
            self.archive()

    def test_malformed_licence_inventory_fails_with_a_clear_validation_error(self):
        path = self.fixture.staging / "licenses/inventory.json"
        for inventory in (
            {"schema_version": True, "target": LINUX, "packages": []},
            {"schema_version": 1, "target": LINUX, "packages": None},
            {"schema_version": 1, "target": LINUX,
             "packages": [{"name": "example", "version": "1.0.0", "notices": "file"}]},
        ):
            path.write_text(json.dumps(inventory))
            with self.subTest(inventory=inventory), self.assertRaisesRegex(ValueError, "inventory"):
                self.archive()

    def test_cannot_smoke_an_executable_that_reports_failure(self):
        cli = self.fixture.staging / "bin/manyhands-cli"
        cli.write_text(f"#!{sys.executable}\nraise SystemExit(9)\n")
        with self.assertRaisesRegex(ValueError, "smoke"):
            verify_archive(self.archive(), self.fixture.release, LINUX, smoke_cli=True)

    @unittest.skipIf(os.name == "nt", "symlink creation requires Windows privilege")
    def test_symlink_in_staging_is_rejected(self):
        (self.fixture.staging / "outside").symlink_to(self.fixture.root)
        with self.assertRaisesRegex(ValueError, "link"):
            self.archive()


class ExtractionTests(unittest.TestCase):
    def test_windows_smoke_environment_drops_build_tool_search_paths(self):
        env = smoke_environment(Path("isolated home"), WINDOWS)
        root = os.environ.get("SystemRoot", r"C:\Windows")
        from pathlib import PureWindowsPath
        self.assertEqual(env["PATH"], f"{PureWindowsPath(root) / 'System32'};{root}")
        self.assertNotIn("LD_LIBRARY_PATH", env)
        self.assertNotIn("DYLD_LIBRARY_PATH", env)

    def test_windows_zip_preserves_both_executables_and_documents(self):
        with tempfile.TemporaryDirectory(prefix="Windows ZIP fixture ") as temp:
            fixture = PayloadFixture(temp, WINDOWS)
            fixture.stage()
            # Only a structural fixture: native Authenticode verification is
            # a separate Windows step, not something this fake EXE can test.
            manifest = fixture.staging / "release.json"
            data = json.loads(manifest.read_text())
            data["signing_mode"] = "self-signed-authenticode"
            manifest.write_text(json.dumps(data))
            signing = fixture.staging / "signing"
            signing.mkdir()
            (signing / "manyhands.cer").write_text("public certificate fixture")
            (signing / "README.md").write_text("self-signing explanation")
            archive = archive_payload(fixture.release, WINDOWS, fixture.staging, fixture.dist)
            extracted = fixture.root / "extracted copy"
            verify_archive(archive, fixture.release, WINDOWS, extract_to=extracted)
            root = extracted / archive.stem
            self.assertEqual((root / "bin/manyhands.exe").read_bytes(),
                             (fixture.binaries / "manyhands.exe").read_bytes())
            self.assertTrue((root / "bin/manyhands-cli.exe").is_file())
            self.assertTrue((root / "licenses/fonts/LICENSE.txt").is_file())

    def test_macos_appledouble_metadata_is_not_treated_as_payload(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / "macos.zip"
            with zipfile.ZipFile(archive, "w") as packed:
                packed.writestr("expected/file", "payload")
                packed.writestr("__MACOSX/expected/._file", "resource metadata")
                packed.writestr("__MACOSX/._expected", "root directory resource metadata")
            extracted = extract_archive(archive, root / "extracted", "expected", MACOS)
            self.assertEqual((extracted / "file").read_text(), "payload")
            self.assertFalse((root / "extracted/__MACOSX").exists())

    def test_tar_cannot_write_outside_expected_root(self):
        for name in ("../escaped", "/absolute", "expected/../../escaped", "expected/../escaped",
                     "other/file", "expected/dir\\file", "expected/C:/file"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                archive = root / "bad.tar.gz"
                with tarfile.open(archive, "w:gz") as packed:
                    member = tarfile.TarInfo(name)
                    member.size = 1
                    packed.addfile(member, io.BytesIO(b"x"))
                with self.assertRaisesRegex(ValueError, "path"):
                    extract_archive(archive, root / "extracted", "expected", LINUX)
                self.assertFalse((root / "escaped").exists())

    def test_tar_links_are_rejected(self):
        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                archive = root / "bad.tar.gz"
                with tarfile.open(archive, "w:gz") as packed:
                    member = tarfile.TarInfo("expected/link")
                    member.type = kind
                    member.linkname = "../../outside"
                    packed.addfile(member)
                with self.assertRaisesRegex(ValueError, "link"):
                    extract_archive(archive, root / "extracted", "expected", LINUX)

    def test_zip_traversal_and_symlinks_are_rejected(self):
        for name, mode in (("expected/../escaped", stat.S_IFREG),
                           ("expected/link", stat.S_IFLNK)):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                archive = root / "bad.zip"
                with zipfile.ZipFile(archive, "w") as packed:
                    member = zipfile.ZipInfo(name)
                    member.external_attr = (mode | 0o644) << 16
                    packed.writestr(member, "../../outside")
                with self.assertRaises(ValueError):
                    extract_archive(archive, root / "extracted", "expected", WINDOWS)


if __name__ == "__main__":
    unittest.main()
