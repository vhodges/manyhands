"""Installed Nix source disclosures follow selected Cargo.lock versions."""

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


HELPER = Path(__file__).resolve().parents[3] / 'packaging/nix/check_disclosures.py'


class NixDisclosureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='nix disclosure checks ')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.locked = []
        self.packages = []

    def add_package(self, name='option-ext', version='9.8.7'):
        raw = f'{name} {version} original source fixture'.encode()
        digest = hashlib.sha256(raw).hexdigest()
        source = f'sources/{name}-{version}.crate'
        path = self.root / source
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(raw)
        package = {'name': name, 'version': version, 'source': source,
                   'source_sha256': digest, 'notices': []}
        self.packages.append(package)
        self.locked.append({'name': name, 'version': version, 'checksum': digest})
        if name == 'wayland-protocols-plasma':
            prefix = f'dependencies/{name}-{version}/plasma-wayland-protocols'
            for member in ('src/protocols/blur.xml', 'COPYING.LIB'):
                relative = f'{prefix}/{member}'
                original = self.root / relative
                original.parent.mkdir(parents=True, exist_ok=True)
                original.write_text('original protocol/licence fixture')
                package['notices'].append(relative)
        return package

    def check(self, *, desktop=False):
        (self.root / 'inventory.json').write_text(json.dumps({'schema_version': 1, 'packages': self.packages}))
        self.assertTrue(HELPER.is_file(), 'Nix disclosure verifier is missing')
        spec = importlib.util.spec_from_file_location('nix_disclosure_verifier', HELPER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.validate_disclosures(self.root, self.locked, desktop=desktop)

    def test_new_locked_versions_need_no_duplicated_version_literal(self):
        self.add_package()
        self.add_package('libgit2-sys', '12.3.4+5.6.7')
        self.add_package('wayland-protocols-plasma', '8.7.6')
        self.check(desktop=True)

    def test_unused_locked_version_does_not_require_an_unselected_source_archive(self):
        self.add_package()
        self.locked.append({'name': 'option-ext', 'version': '1.2.3', 'checksum': 'a' * 64})
        self.check()

    def test_locked_source_package_cannot_disappear_from_the_inventory(self):
        self.add_package()
        self.locked.append({'name': 'libgit2-sys', 'version': '12.3.4', 'checksum': 'a' * 64})
        with self.assertRaisesRegex(ValueError, 'source.*inventory'):
            self.check()

    def test_empty_inventory_cannot_clear_existing_locked_source_requirements(self):
        self.add_package()
        self.packages.clear()
        with self.assertRaisesRegex(ValueError, 'source.*inventory'):
            self.check()

    def test_missing_selected_source_archive_is_rejected(self):
        package = self.add_package()
        (self.root / package['source']).unlink()
        with self.assertRaisesRegex(ValueError, 'source'):
            self.check()

    def test_stale_version_is_not_accepted_as_the_locked_package(self):
        self.add_package()['version'] = '0.0.1'
        with self.assertRaisesRegex(ValueError, 'locked'):
            self.check()

    def test_source_bytes_must_match_both_inventory_and_lock(self):
        package = self.add_package()
        (self.root / package['source']).write_bytes(b'changed source')
        package['source_sha256'] = hashlib.sha256(b'changed source').hexdigest()
        with self.assertRaisesRegex(ValueError, 'checksum'):
            self.check()

    def test_inventoried_source_path_must_match_the_exact_locked_identity(self):
        self.add_package()['source'] = '../outside.crate'
        with self.assertRaisesRegex(ValueError, 'source.*path'):
            self.check()

    def test_selected_package_cannot_omit_its_source_record(self):
        self.add_package().pop('source')
        with self.assertRaisesRegex(ValueError, 'source.*path'):
            self.check()

    def test_source_symlink_is_not_an_original_installed_archive(self):
        package = self.add_package()
        source = self.root / package['source']
        target = self.root / 'original.crate'
        source.rename(target)
        source.symlink_to(target)
        with self.assertRaisesRegex(ValueError, 'linked|symbolic'):
            self.check()

    def test_desktop_protocol_paths_follow_the_selected_version(self):
        package = self.add_package('wayland-protocols-plasma', '8.7.6')
        (self.root / package['notices'][0]).unlink()
        with self.assertRaisesRegex(ValueError, 'protocol'):
            self.check(desktop=True)

    def test_protocol_files_must_belong_to_the_selected_inventory(self):
        self.add_package('wayland-protocols-plasma', '8.7.6')['notices'] = []
        with self.assertRaisesRegex(ValueError, 'protocol'):
            self.check(desktop=True)

    def test_cli_does_not_silently_acquire_desktop_protocol_material(self):
        self.add_package('wayland-protocols-plasma', '8.7.6')
        with self.assertRaisesRegex(ValueError, 'CLI|desktop'):
            self.check()


if __name__ == '__main__':
    unittest.main()
