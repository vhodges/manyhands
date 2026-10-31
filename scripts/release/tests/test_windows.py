import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from windows import finalize_windows, verify_windows
from payload_fixture import WINDOWS, PayloadFixture


class WindowsSigningConfigurationTests(unittest.TestCase):
    def test_timestamp_endpoint_uses_documented_signtool_transport(self):
        repo = Path(__file__).resolve().parents[3]
        configuration = json.loads((repo / "packaging/windows/signing.json").read_text(encoding="utf-8"))
        self.assertEqual(configuration["timestamp_url"], "http://timestamp.digicert.com")


class WindowsMetadataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="Windows signing metadata ")
        self.addCleanup(self.temp.cleanup)
        self.fixture = PayloadFixture(self.temp.name, WINDOWS)
        self.fixture.stage()
        signing = self.fixture.staging / "signing"
        signing.mkdir()
        self.certificate = b"public DER fixture; native X509 parsing is tested on Windows"
        (signing / "manyhands.cer").write_bytes(self.certificate)
        self.runtime = {"system": "windows", "architecture": "x86_64", "crt": "static",
                        "binaries": {"manyhands.exe": ["KERNEL32.dll"], "manyhands-cli.exe": ["KERNEL32.dll"]}}
        self.signer = {"subject": "CN=Release fixture", "thumbprint_sha1": hashlib.sha1(self.certificate).hexdigest().upper(),
                       "fingerprint_sha256": hashlib.sha256(self.certificate).hexdigest().upper(),
                       "expires_utc": "2029-10-30T00:00:00Z", "timestamp_url": "http://timestamp.digicert.com/"}

    def test_finalization_records_public_identity_and_runtime_terms(self):
        finalize_windows(self.fixture.staging, self.runtime, self.signer)
        manifest = verify_windows(self.fixture.staging)
        self.assertEqual(manifest["signing_mode"], "self-signed-authenticode")
        self.assertEqual(manifest["runtime_audit"]["crt"], "static")
        self.assertEqual(manifest["signer"]["timestamp_url"], self.signer["timestamp_url"])
        readme = (self.fixture.staging / "signing/README.md").read_text()
        self.assertIn(self.signer["fingerprint_sha256"], readme)
        self.assertIn(f"RFC 3161 timestamp service: {self.signer['timestamp_url']}", readme)
        self.assertIn("not trusted", readme)

    def test_mismatched_public_certificate_cannot_finalize(self):
        self.signer["fingerprint_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "certificate"):
            finalize_windows(self.fixture.staging, self.runtime, self.signer)
        manifest = json.loads((self.fixture.staging / "release.json").read_text())
        self.assertEqual(manifest["signing_mode"], "pending")

    def test_dynamic_crt_cannot_be_stamped_as_portable(self):
        self.runtime["crt"] = "dynamic"
        with self.assertRaisesRegex(ValueError, "runtime"):
            finalize_windows(self.fixture.staging, self.runtime, self.signer)

    def test_changed_public_certificate_fails_verification(self):
        finalize_windows(self.fixture.staging, self.runtime, self.signer)
        (self.fixture.staging / "signing/manyhands.cer").write_bytes(b"different certificate")
        with self.assertRaisesRegex(ValueError, "certificate"):
            verify_windows(self.fixture.staging)


if __name__ == "__main__":
    unittest.main()
