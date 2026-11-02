"""Installer metadata and shared Pascal policy; native installation is separate."""

import copy
import contextlib
import hashlib
import json
import io
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from installer import compiler_config, installer_inventory, numeric_version, validate_installer, verify_installer_payload
from package import archive_payload, refresh_documents
from payload_fixture import REPO, WINDOWS, PayloadFixture
import installer


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
        verified_files = verify_installer_payload(fixture.staging, archive, fixture.release)
        self.assertEqual(verified_files, {path.relative_to(fixture.staging).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
                                         for path in fixture.staging.rglob('*') if path.is_file()})
        compiler_inputs = self.root / 'compiler inputs'
        compiler_inputs.mkdir()
        parameters = compiler_inputs / 'parameters.json'
        arguments = ['installer.py', 'parameters', '--staging', str(fixture.staging), '--archive', str(archive),
                     '--output', str(parameters)]
        with patch.object(sys, 'argv', arguments), patch('windows.verify_windows', return_value=manifest):
            self.assertEqual(installer.main(), 0)
        managed = Path(json.loads(parameters.read_text())['managed_files'])
        self.assertEqual(managed.read_text(encoding='utf-8-sig').splitlines(), installer.managed_file_list(verified_files))
        digests = Path(json.loads(parameters.read_text())['managed_digests'])
        self.assertEqual(digests.read_text(encoding='utf-8-sig').splitlines(),
                         installer.managed_digest_list(verified_files, managed.read_bytes(),
                                                       (REPO / 'packaging/windows/INNO-LICENSE.txt').read_bytes()))
        # Installer-only inputs must not enter or mutate the portable ZIP tree.
        self.assertFalse((fixture.staging / 'licenses/installer/managed-files.txt').exists())
        self.assertFalse((fixture.staging / 'licenses/installer/managed-digests.txt').exists())
        self.assertEqual(verify_installer_payload(fixture.staging, archive, fixture.release), verified_files)
        binary = fixture.staging / 'bin/manyhands.exe'
        binary.write_bytes(b'different binary with the same manifest')
        with self.assertRaisesRegex(ValueError, 'payload'):
            verify_installer_payload(fixture.staging, archive, fixture.release)
        for command in ('parameters', 'inventory'):
            output = self.root / (command + '.json')
            arguments = ['installer.py', command, '--staging', str(fixture.staging), '--archive', str(archive),
                         '--output', str(output), '--installer', str(self.setup)]
            with self.subTest(command=command), patch.object(sys, 'argv', arguments), \
                    patch('windows.verify_windows', return_value=manifest), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(installer.main(), 1)
                self.assertFalse(output.exists())
                self.assertFalse((self.root / 'managed-files.txt').exists())

    def test_managed_manifest_is_canonical_and_includes_installer_additions(self):
        paths = ['release.json', 'licenses/README.md', 'bin/manyhands.exe', 'bin/manyhands-cli.exe']
        expected = ['bin\\manyhands-cli.exe', 'bin\\manyhands.exe', 'licenses\\installer\\INNO-LICENSE.txt',
                    'licenses\\installer\\managed-files.txt', 'licenses\\README.md']
        self.assertEqual(installer.managed_file_list(paths), expected)
        destination = self.root / 'managed-files.txt'
        installer.write_managed_file_list(destination, paths)
        self.assertEqual(destination.read_bytes(), ('\ufeff' + '\r\n'.join(expected) + '\r\n').encode('utf-8'))

    def test_managed_manifest_cannot_overwrite_an_existing_or_linked_input(self):
        paths = ['bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md']
        target = self.root / 'portable-input.txt'
        target.write_bytes(b'portable bytes must remain unchanged')
        destination = self.root / 'managed-files.txt'
        destination.symlink_to(target)
        with self.assertRaises(OSError):
            installer.write_managed_file_list(destination, paths)
        self.assertEqual(target.read_bytes(), b'portable bytes must remain unchanged')
        with self.assertRaises(OSError):
            installer.write_managed_file_list(target, paths)

    def test_managed_manifest_rejects_windows_aliases_collisions_and_reserved_additions(self):
        base = ['bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md']
        for bad in ('bin/../outside', 'bin//file', '/bin/file', 'bin\\file', 'bin/a:stream',
                    'bin/con.txt', 'bin/COM1', 'bin/CONIN$', 'bin/con .txt', 'bin/file.', 'bin/file ', 'bin/a\nfile',
                    'bin/<file>', 'BIN/manyhands.exe', 'bin/Manyhands.exe',
                    'licenses/installer/managed-files.txt', 'licenses/installer/INNO-LICENSE.txt',
                    'bin/managed-files.txt', 'licenses/other/MANAGED-DIGESTS.txt', 'signing/managed-digests.txt'):
            with self.subTest(path=bad), self.assertRaisesRegex(ValueError, 'managed|path|reserved'):
                installer.managed_file_list(base + [bad])
        for extra in (['licenses/Dir/a', 'licenses/dir/b'], ['bin/extra', 'bin/extra/child']):
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                installer.managed_file_list(base + extra)

    def test_managed_digests_bind_verified_bytes_license_and_manifest_without_self_hash_cycle(self):
        payload = {'licenses/README.md': hashlib.sha256(b'new README').hexdigest(),
                   'bin/manyhands.exe': hashlib.sha256(b'final signed executable').hexdigest(),
                   'bin/manyhands-cli.exe': hashlib.sha256(b'final signed CLI').hexdigest(),
                   'release.json': 'a' * 64}
        original = copy.deepcopy(payload)
        paths = installer.managed_file_list(payload)
        manifest = ('\ufeff' + '\r\n'.join(paths) + '\r\n').encode('utf-8')
        license_bytes = b'exact original Inno licence bytes\r\n'
        expected = {name.replace('/', '\\'): digest for name, digest in payload.items() if name != 'release.json'}
        expected['licenses\\installer\\INNO-LICENSE.txt'] = hashlib.sha256(license_bytes).hexdigest()
        expected['licenses\\installer\\managed-files.txt'] = hashlib.sha256(manifest).hexdigest()
        records = [expected[name] + '  ' + name for name in paths]
        self.assertEqual(installer.managed_digest_list(payload, manifest, license_bytes), records)
        self.assertEqual(payload, original, 'Generating installer additions may not mutate verified ZIP records')
        destination = self.root / 'managed-digests.txt'
        installer.write_managed_digests(destination, payload, manifest, license_bytes)
        self.assertEqual(destination.read_bytes(), ('\ufeff' + '\r\n'.join(records) + '\r\n').encode('utf-8'))
        with self.assertRaises(OSError):
            installer.write_managed_digests(destination, payload, manifest, license_bytes)
        target = self.root / 'portable-input.txt'
        target.write_bytes(b'unchanged portable bytes')
        linked = self.root / 'linked-digests.txt'
        linked.symlink_to(target)
        with self.assertRaises(OSError):
            installer.write_managed_digests(linked, payload, manifest, license_bytes)
        self.assertEqual(target.read_bytes(), b'unchanged portable bytes')
        self.assertNotIn('licenses\\installer\\managed-digests.txt', paths, 'Trust metadata is embedded, not installed/self-hashed')

    def test_managed_digests_reject_noncanonical_hashes_and_manifest_binding(self):
        payload = {'bin/manyhands.exe': 'a' * 64, 'bin/manyhands-cli.exe': 'b' * 64, 'licenses/README.md': 'c' * 64}
        manifest = ('\ufeff' + '\r\n'.join(installer.managed_file_list(payload)) + '\r\n').encode('utf-8')
        for bad in ('A' * 64, 'a' * 63, 'g' * 64, 'a' * 64 + '\n', None):
            with self.subTest(digest=bad), self.assertRaises(ValueError):
                installer.managed_digest_list({**payload, 'bin/manyhands.exe': bad}, manifest, b'licence')
        for bad in (b'', manifest[3:], manifest.replace(b'\r\n', b'\n'), manifest.replace(b'manyhands.exe', b'Manyhands.exe'),
                    manifest + b'bin\\extra.exe\r\n'):
            with self.subTest(manifest=bad), self.assertRaises(ValueError):
                installer.managed_digest_list(payload, bad, b'licence')
        for bad in (b'', None, 'not byte data'):
            with self.subTest(license=bad), self.assertRaises(ValueError):
                installer.managed_digest_list(payload, manifest, bad)
        with self.assertRaises(ValueError):
            installer.managed_digest_list({**payload, 'bin/Manyhands.exe': 'd' * 64}, manifest, b'licence')

    def test_parameters_and_inventory_recheck_payload_before_writing_outputs(self):
        # These are the pre-compile and post-compile entry points, not source-text
        # assertions. A changed payload may never emit a managed list/inventory.
        manifest = {**self.package, 'version': '0.1.0'}
        for command in ('parameters', 'inventory'):
            output = self.root / (command + '.json')
            arguments = ['installer.py', command, '--staging', str(self.root), '--archive', str(self.archive),
                         '--output', str(output), '--installer', str(self.setup)]
            with self.subTest(command=command), patch.object(sys, 'argv', arguments), \
                    patch('windows.verify_windows', return_value=manifest), \
                    patch.object(installer, 'verify_installer_payload', side_effect=ValueError('changed payload')) as verify:
                with contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(installer.main(), 1)
                verify.assert_called_once_with(self.root, self.archive, manifest)
                self.assertFalse(output.exists())
                self.assertFalse((self.root / 'managed-files.txt').exists())

    def test_parameters_derive_managed_list_only_from_verified_payload(self):
        manifest = {**self.package, 'version': '0.1.0'}
        output = self.root / 'parameters.json'
        paths = ['bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md']
        arguments = ['installer.py', 'parameters', '--staging', str(self.root / 'staging'), '--archive', str(self.archive),
                     '--output', str(output)]
        with patch.object(sys, 'argv', arguments), patch('windows.verify_windows', return_value=manifest), \
                patch.object(installer, 'verify_installer_payload', return_value={name: 'a' * 64 for name in paths}):
            self.assertEqual(installer.main(), 0)
        managed = Path(json.loads(output.read_text())['managed_files'])
        self.assertEqual(managed, self.root / 'managed-files.txt')
        self.assertEqual(managed.read_text(encoding='utf-8-sig').splitlines(), installer.managed_file_list(paths))
        digests = Path(json.loads(output.read_text())['managed_digests'])
        self.assertEqual(digests, self.root / 'managed-digests.txt')

    def test_generated_managed_list_may_not_change_the_portable_payload(self):
        output = self.root / 'parameters.json'
        args = ['installer.py', 'parameters', '--staging', str(self.root), '--archive', str(self.archive),
                '--output', str(output)]
        paths = ['bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md']
        with patch.object(sys, 'argv', args), patch('windows.verify_windows', return_value=self.package), \
                patch.object(installer, 'verify_installer_payload', return_value={name: 'a' * 64 for name in paths}), \
                contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(installer.main(), 1)
        self.assertIn('outside the portable payload', errors.getvalue())
        self.assertFalse(output.exists())
        self.assertFalse((self.root / 'managed-files.txt').exists())

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
