"""Installer metadata and shared Pascal policy; native installation is separate."""

import copy
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from installer import compiler_config, installer_inventory, numeric_version, validate_installer, verify_installer_payload
from package import archive_payload, refresh_documents
from payload_fixture import REPO, WINDOWS, PayloadFixture


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='installer metadata ')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.release = {'schema_version': 1, 'tag': 'v0.1.0', 'version': '0.1.0',
                        'commit': 'a' * 40, 'prerelease': False}
        self.archive = self.root / 'manyhands-v0.1.0-x86_64-pc-windows-msvc.zip'
        self.archive.write_bytes(b'verified ZIP boundary fixture')
        self.setup = self.root / 'manyhands-v0.1.0-x86_64-pc-windows-msvc-setup.exe'
        self.setup.write_bytes(b'signed Inno Setup boundary fixture')
        self.package = {**self.release, 'target': WINDOWS, 'signing_mode': 'self-signed-authenticode',
                        'signer': {'thumbprint_sha1': 'A' * 40, 'fingerprint_sha256': 'B' * 64}}

    def inventory(self):
        return installer_inventory(self.release, self.archive, self.package, self.setup)

    def test_full_semver_maps_to_bounded_numeric_file_metadata(self):
        for version, expected in [('0.1.6', '0.1.6.0'), ('1.2.3-rc.10', '1.2.3.0'), ('65535.65535.65535', '65535.65535.65535.0')]:
            self.assertEqual(numeric_version(version), expected)
        for version in ('65536.1.0', '0.999999999999999999999.0', '01.2.3', '1.2.3+build'):
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, 'version|65535|tag'):
                numeric_version(version)

    def test_compiler_download_is_versioned_https_and_sha256_pinned(self):
        config = compiler_config()
        self.assertEqual(config['version'], '7.1.1')
        self.assertEqual(config['url'], 'https://github.com/jrsoftware/issrc/releases/download/is-7_1_1/innosetup-7.1.1-x64.exe')
        self.assertEqual(config['sha256'], '8a5a6711c3766a6e5fb74f280ba0e3551e719ce94538d901fdbd1f8f9920b3f0')

    def test_inventory_binds_setup_bytes_to_the_exact_zip_and_signer(self):
        inventory = self.inventory()
        self.assertEqual(inventory['kind'], 'manyhands-windows-installer')
        self.assertEqual(inventory['app_id'], 'io.github.vhodges.manyhands')
        self.assertEqual(inventory['numeric_version'], '0.1.0.0')
        self.assertEqual(inventory['source_archive']['sha256'], hashlib.sha256(self.archive.read_bytes()).hexdigest())
        self.assertEqual(inventory['installer']['sha256'], hashlib.sha256(self.setup.read_bytes()).hexdigest())
        validate_installer(self.release, self.setup, inventory, self.archive, self.package)

    def test_wrong_identity_signer_zip_or_setup_bytes_are_rejected(self):
        inventory = self.inventory()
        for field, value in [('commit', 'b' * 40), ('signer', {}), ('target', 'aarch64-unknown-linux-gnu'), ('app_id', 'other'), ('numeric_version', '0.1.1.0'), ('compiler', {})]:
            bad = copy.deepcopy(inventory)
            bad[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_installer(self.release, self.setup, bad, self.archive, self.package)
        self.archive.write_bytes(b'new ZIP build')
        with self.assertRaisesRegex(ValueError, 'archive'):
            validate_installer(self.release, self.setup, inventory, self.archive, self.package)
        self.archive.write_bytes(b'verified ZIP boundary fixture')
        self.setup.write_bytes(b'changed setup')
        with self.assertRaisesRegex(ValueError, 'installer'):
            validate_installer(self.release, self.setup, inventory, self.archive, self.package)

    def test_incomplete_or_unsigned_metadata_cannot_produce_inventory(self):
        for field, value in [('signing_mode', 'pending'), ('signer', None), ('commit', 'b' * 40)]:
            original = self.package[field]
            self.package[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.inventory()
            self.package[field] = original

    def test_compiler_payload_must_match_every_file_in_the_verified_zip(self):
        root = self.root / 'payload fixture'
        root.mkdir()
        fixture = PayloadFixture(root, WINDOWS)
        fixture.stage()
        signing = fixture.staging / 'signing'
        signing.mkdir()
        (signing / 'manyhands.cer').write_bytes(b'public fixture')
        (signing / 'README.md').write_text('public fixture explanation')
        manifest = json.loads((fixture.staging / 'release.json').read_text())
        manifest['signing_mode'] = 'self-signed-authenticode'
        refresh_documents(fixture.staging, manifest)
        archive = archive_payload(fixture.release, WINDOWS, fixture.staging, fixture.dist)
        verify_installer_payload(fixture.staging, archive, fixture.release)
        binary = fixture.staging / 'bin/manyhands.exe'
        binary.write_bytes(b'different binary with the same manifest')
        with self.assertRaisesRegex(ValueError, 'payload'):
            verify_installer_payload(fixture.staging, archive, fixture.release)

    def test_committed_icon_contains_standard_sizes_and_a_256_pixel_image(self):
        data = (REPO / 'packaging/windows/manyhands.ico').read_bytes()
        reserved, kind, count = struct.unpack_from('<HHH', data)
        self.assertEqual((reserved, kind), (0, 1))
        widths = []
        for index in range(count):
            width, height, _, _, _, _, length, offset = struct.unpack_from('<BBBBHHII', data, 6 + index * 16)
            self.assertEqual(width, height)
            self.assertLessEqual(offset + length, len(data))
            widths.append(width or 256)
        self.assertTrue({16, 32, 48, 256}.issubset(widths))

    @unittest.skipUnless(shutil.which('fpc'), 'Local Pascal execution needs FPC; Windows uses the actual Inno engine')
    def test_actual_installer_pascal_policy(self):
        source = REPO / 'scripts/release/tests/installer_policy_fpc.pas'
        result = subprocess.run(['fpc', f'-Fi{REPO / "packaging/windows"}', f'-Fi{source.parent}',
                                 f'-FE{self.root}', f'-FU{self.root}', str(source)], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        result = subprocess.run([str(self.root / 'installer_policy_fpc')], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
