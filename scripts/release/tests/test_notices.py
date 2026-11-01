"""Disclosures must include nested notices and checksum-verified source."""

import hashlib
import html
import json
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest
from urllib.parse import urlsplit
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from notices import gather_disclosures, generate_notices, render_notices


MIT_GRANT = 'Permission is hereby granted' + (
    Path(__file__).resolve().parents[3] / 'LICENSE'
).read_text(encoding='utf-8').split('Permission is hereby granted', 1)[1]
MIT_ORIGINAL = 'Copyright (c) 2026 Original Authors\n\n' + MIT_GRANT
MIT_CANONICAL = 'Copyright (c) <year> <copyright holders>\n\n' + MIT_GRANT
ISC_GRANT = '''Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
'''


class NoticeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release notices ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.output = self.root / "output"
        self.crate = self.root / "registry/src/example-index/option-ext-0.2.0"
        self.crate.mkdir(parents=True)
        self.cache = self.root / "registry/cache/example-index"
        self.cache.mkdir(parents=True)
        self.source = self.cache / "option-ext-0.2.0.crate"
        self.source.write_bytes(b"original published source archive")
        checksum = hashlib.sha256(self.source.read_bytes()).hexdigest()
        (self.repo / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "option-ext"\nversion = "0.2.0"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{checksum}"\n', encoding="utf-8",
        )
        self.package = {
            "name": "option-ext", "version": "0.2.0",
            "source": "registry+https://github.com/rust-lang/crates.io-index",
            "manifest_path": str(self.crate / "Cargo.toml"),
        }

    def test_copies_nested_native_notices_without_flattening(self):
        nested = self.crate / "vendor/library/COPYING"
        nested.parent.mkdir(parents=True)
        nested.write_text("Native library copyright and complete terms", encoding="utf-8")
        gather_disclosures(self.repo, [self.package], set(), self.output)
        copied = self.output / "dependencies/option-ext-0.2.0/vendor/library/COPYING"
        self.assertEqual(copied.read_text(), nested.read_text())

    def test_preserves_authors_referenced_by_the_original_licence(self):
        (self.crate / 'LICENSE').write_text('Copyright the contributors listed in AUTHORS')
        original = self.crate / 'AUTHORS'
        original.write_bytes('Original Author One\r\nOriginal Author Two\r\n'.encode())
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        relative = 'dependencies/option-ext-0.2.0/AUTHORS'
        self.assertIn(relative, inventory[0]['notices'])
        self.assertEqual((self.output / relative).read_bytes(), original.read_bytes())

    def test_preserves_arbitrary_original_filenames_inside_licence_directories(self):
        original = self.crate / '.licenses/ChenYuheng-MIT'
        original.parent.mkdir()
        original.write_text('Copyright (c) 2015 Chen Yuheng\nComplete original terms')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        relative = 'dependencies/option-ext-0.2.0/.licenses/ChenYuheng-MIT'
        self.assertIn(relative, inventory[0]['notices'])
        self.assertEqual((self.output / relative).read_bytes(), original.read_bytes())

    def test_preserves_protocol_xml_with_embedded_copyright_and_licence(self):
        original = self.crate / 'protocols/blur.xml'
        original.parent.mkdir()
        original.write_bytes(b'<protocol><copyright><![CDATA[SPDX-FileCopyrightText: Original Author\n'
                             b'SPDX-License-Identifier: LGPL-2.1-or-later]]></copyright></protocol>\n')
        (original.parent / 'unlicensed-data.xml').write_text('<unrelated/>')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        relative = 'dependencies/option-ext-0.2.0/protocols/blur.xml'
        self.assertIn(relative, inventory[0]['notices'])
        self.assertEqual((self.output / relative).read_bytes(), original.read_bytes())
        self.assertFalse((self.output / 'dependencies/option-ext-0.2.0/protocols/unlicensed-data.xml').exists())

    def test_original_source_is_verified_against_the_lockfile(self):
        gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)
        self.assertEqual(
            (self.output / "sources/option-ext-0.2.0.crate").read_bytes(),
            self.source.read_bytes(),
        )

    def test_corrupt_original_source_fails(self):
        self.source.write_bytes(b"corrupt cached download")
        with self.assertRaisesRegex(ValueError, "checksum"):
            gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)

    def test_missing_original_source_fails(self):
        self.source.unlink()
        with self.assertRaisesRegex(ValueError, "source archive"):
            gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)

    def test_fixed_output_sources_work_without_a_cargo_registry_cache(self):
        sources = self.root / 'fixed output sources'
        sources.mkdir()
        original = self.source.read_bytes()
        (sources / self.source.name).write_bytes(original)
        self.source.unlink()
        gather_disclosures(self.repo, [self.package], {('option-ext', '0.2.0')}, self.output,
                           source_archives=sources)
        self.assertEqual((self.output / 'sources/option-ext-0.2.0.crate').read_bytes(), original)

    def test_fixed_output_source_selection_never_falls_back_to_a_developer_cache(self):
        sources = self.root / 'fixed output sources'
        sources.mkdir()
        with self.assertRaisesRegex(ValueError, 'source archive'):
            gather_disclosures(self.repo, [self.package], {('option-ext', '0.2.0')}, self.output,
                               source_archives=sources)

    def test_fixed_output_source_hash_is_still_checked_against_cargo_lock(self):
        sources = self.root / 'fixed output sources'
        sources.mkdir()
        (sources / self.source.name).write_bytes(b'changed fixed-output source')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            gather_disclosures(self.repo, [self.package], {('option-ext', '0.2.0')}, self.output,
                               source_archives=sources)

    def supplemental(self, text=MIT_ORIGINAL):
        directory = self.repo / 'packaging/licenses/supplemental'
        directory.mkdir(parents=True)
        license_file = directory / 'option-ext-0.2.0-LICENSE'
        license_file.write_text(text, encoding='utf-8')
        entry = {
            'name': 'option-ext', 'version': '0.2.0', 'file': license_file.name,
            'sha256': hashlib.sha256(license_file.read_bytes()).hexdigest(),
            'crate_sha256': hashlib.sha256(self.source.read_bytes()).hexdigest(),
            'url': 'https://example.test/pinned-commit/LICENSE',
            'binding': {'license': 'MIT', 'component': 'option-ext@0.2.0', 'role': 'grant'},
        }
        manifest = self.repo / 'packaging/licenses/supplemental.json'
        manifest.write_text(json.dumps({'schema_version': 1, 'notices': [entry]}))
        return license_file, manifest

    def test_supplemental_original_notice_is_hash_checked_and_inventoried(self):
        original, _ = self.supplemental()
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        copied = self.output / 'dependencies/option-ext-0.2.0/supplemental/option-ext-0.2.0-LICENSE'
        self.assertTrue(copied.is_file())
        self.assertEqual(copied.read_bytes(), original.read_bytes())
        self.assertIn(copied.relative_to(self.output).as_posix(), inventory[0]['notices'])
        self.assertEqual(inventory[0]['supplemental'][0]['url'], 'https://example.test/pinned-commit/LICENSE')

    def test_corrupt_supplemental_notice_is_rejected(self):
        original, _ = self.supplemental()
        original.write_text('modified copyright and terms')
        with self.assertRaisesRegex(ValueError, 'supplemental.*checksum'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_supplemental_notice_must_match_the_exact_locked_crate(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['crate_sha256'] = '0' * 64
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'supplemental.*Cargo.lock'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_supplement_can_preserve_an_exact_original_crate_member(self):
        original, manifest = self.supplemental()
        member = self.crate / 'src/header.rs'
        member.parent.mkdir()
        member.write_bytes(original.read_bytes())
        data = json.loads(manifest.read_text())
        notice = data['notices'][0]
        del notice['file']
        notice['source_member'] = 'src/header.rs'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        relative = inventory[0]['supplemental'][0]['path']
        self.assertEqual((self.output / relative).read_bytes(), member.read_bytes())
        self.assertEqual(inventory[0]['supplemental'][0]['source_member'], 'src/header.rs')

    def test_supplemental_source_member_cannot_escape_the_locked_crate(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        del data['notices'][0]['file']
        data['notices'][0]['source_member'] = '../outside'
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'supplemental.*path'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_reviewed_grant_without_named_header_preserves_original_not_invented_holder(self):
        from notices import validate_attributions
        original, manifest = self.supplemental(MIT_GRANT)
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {
            'kind': 'original-grant-no-named-header', 'license': 'MIT',
            'reason': 'Exact upstream original grant contains no named copyright header.',
        }
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        about = {'licenses': [{'id': 'MIT', 'text': MIT_CANONICAL,
                               'used_by': [{'crate': self.package}]}]}
        validate_attributions(about, inventory, self.output)
        resolution = inventory[0]['supplemental'][0]['resolution']
        self.assertEqual(resolution, data['notices'][0]['resolution'])
        self.assertEqual((self.output / inventory[0]['notices'][0]).read_bytes(), original.read_bytes())

    def test_declaration_only_evidence_does_not_silently_clear_missing_original(self):
        from notices import validate_attributions
        _, manifest = self.supplemental('license = "MIT"\nauthors = ["Not automatically a copyright holder"]\n')
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {'kind': 'declaration-only', 'license': 'MIT', 'reason': 'Only release-original manifest was recovered.'}
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        about = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>',
                               'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(about, inventory, self.output)

    def approved_declaration(self, expression='MIT', source_member='Cargo.toml'):
        text = ('[package]\nname = "option-ext"\nversion = "0.2.0"\n'
                f'license = "{expression}"\nauthors = ["Not an inferred copyright holder"]\n')
        original, manifest = self.supplemental(text)
        member = self.crate / source_member
        member.write_bytes(original.read_bytes())
        data = json.loads(manifest.read_text())
        notice = data['notices'][0]
        del notice['file']
        notice['source_member'] = source_member
        notice['binding'].update(role='grant-by-reference', reason='Exact original release MIT declaration accepted only by the scoped owner decision.')
        notice['resolution'] = {
            'kind': 'owner-approved-release-declaration', 'license': 'MIT',
            'reason': 'No named licence notice was supplied. Preserve canonical MIT terms and this exact original release declaration.',
            'owner_decision': {
                'decision': 'Accept scoped declarations', 'date': '2026-11-01',
                'provenance': 'Explicit product-owner selection in this remediation session; KDE and Apple conditions remain unresolved.',
                'proof': {key: notice[key] for key in ('crate_sha256', 'source_member', 'sha256')},
            },
        }
        notice['resolution']['owner_decision']['proof'].update(component='option-ext@0.2.0', license='MIT')
        manifest.write_text(json.dumps(data))
        return member, manifest

    def test_owner_approved_exact_release_declaration_resolves_without_inventing_a_holder(self):
        original, _ = self.approved_declaration()
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)
        notice = inventory[0]['supplemental'][0]
        self.assertEqual((self.output / notice['path']).read_bytes(), original.read_bytes())
        self.assertNotIn('Copyright', original.read_text())
        self.assertEqual(notice['resolution']['owner_decision']['date'], '2026-11-01')

    def test_approved_historical_mit_apache_declarations_keep_the_mit_selection(self):
        _, manifest = self.approved_declaration()
        data = json.loads(manifest.read_text())
        original = self.crate / 'Cargo.toml'
        for expression in ('MIT', 'Apache-2.0 OR MIT', 'MIT / Apache-2.0', 'MIT/Apache-2.0'):
            original.write_text(f'[package]\nname="option-ext"\nversion="0.2.0"\nlicense="{expression}"\n')
            digest = hashlib.sha256(original.read_bytes()).hexdigest()
            data['notices'][0]['sha256'] = digest
            data['notices'][0]['resolution']['owner_decision']['proof']['sha256'] = digest
            manifest.write_text(json.dumps(data))
            with self.subTest(expression=expression):
                inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
                self.validate_mit(inventory)
                self.assertEqual(inventory[0]['supplemental'][0]['binding']['license'], 'MIT')

    def test_new_declaration_resolution_requires_an_explicit_dated_owner_decision(self):
        _, manifest = self.approved_declaration()
        original = json.loads(manifest.read_text())
        decision = original['notices'][0]['resolution']['owner_decision']
        invalid = [None, {}, {**decision, 'decision': 'not approved'},
                   {**decision, 'date': ''}, {**decision, 'date': '2026-02-30'},
                   {**decision, 'date': True}, {**decision, 'provenance': ''},
                   {**decision, 'provenance': '   '}, {**decision, 'proof': {}}]
        for approval in invalid:
            data = json.loads(json.dumps(original))
            if approval is None:
                del data['notices'][0]['resolution']['owner_decision']
            else:
                data['notices'][0]['resolution']['owner_decision'] = approval
            manifest.write_text(json.dumps(data))
            with self.subTest(approval=approval), self.assertRaises(ValueError):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_approved_declaration_requires_matching_component_licence_role_and_reason(self):
        _, manifest = self.approved_declaration()
        original = json.loads(manifest.read_text())
        for field, value in (('component', 'other@0.2.0'), ('license', 'ISC'),
                             ('role', 'grant'), ('role', 'declaration'), ('reason', '')):
            data = json.loads(json.dumps(original))
            data['notices'][0]['binding'][field] = value
            manifest.write_text(json.dumps(data))
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_owner_approval_proof_cannot_be_replayed_for_other_record_hashes_or_versions(self):
        _, manifest = self.approved_declaration()
        original = json.loads(manifest.read_text())
        for field, value in (('component', 'option-ext@0.3.0'), ('license', 'ISC'),
                             ('crate_sha256', '0' * 64), ('sha256', '0' * 64),
                             ('source_member', 'README.md')):
            data = json.loads(json.dumps(original))
            data['notices'][0]['resolution']['owner_decision']['proof'][field] = value
            manifest.write_text(json.dumps(data))
            with self.subTest(field=field), self.assertRaises(ValueError):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_new_release_version_does_not_inherit_the_old_owner_decision(self):
        _, manifest = self.approved_declaration()
        data = json.loads(manifest.read_text())
        data['notices'][0]['version'] = '0.3.0'
        data['notices'][0]['binding']['component'] = 'option-ext@0.3.0'
        manifest.write_text(json.dumps(data))
        lock = self.repo / 'Cargo.lock'
        lock.write_text(lock.read_text().replace('0.2.0', '0.3.0'))
        package = {**self.package, 'version': '0.3.0'}
        with self.assertRaises(ValueError):
            gather_disclosures(self.repo, [package], set(), self.output)

    def test_owner_decision_still_requires_actual_unmodified_original_manifest_bytes(self):
        original, manifest = self.approved_declaration()
        content = original.read_bytes()
        original.unlink()
        with self.assertRaisesRegex(ValueError, 'missing original'):
            gather_disclosures(self.repo, [self.package], set(), self.output)
        original.write_bytes(content + b'# changed evidence\n')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            gather_disclosures(self.repo, [self.package], set(), self.output)
        original.write_bytes(content)
        data = json.loads(manifest.read_text())
        data['notices'][0]['crate_sha256'] = '0' * 64
        data['notices'][0]['resolution']['owner_decision']['proof']['crate_sha256'] = '0' * 64
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'Cargo.lock'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_partial_source_member_cannot_be_upgraded_to_an_approved_release_manifest(self):
        self.approved_declaration(source_member='README.md')
        with self.assertRaises(ValueError):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_approved_manifest_must_declare_its_exact_name_version_and_selected_licence(self):
        original, manifest = self.approved_declaration()
        base = json.loads(manifest.read_text())
        invalid = ['[package]\nname="different"\nversion="0.2.0"\nlicense="MIT"\n',
                   '[package]\nname="option-ext"\nversion="0.3.0"\nlicense="MIT"\n',
                   '[package]\nname="option-ext"\nversion="0.2.0"\nlicense="Apache-2.0"\n',
                   'not a TOML manifest', '']
        for content in invalid:
            original.write_text(content)
            data = json.loads(json.dumps(base))
            digest = hashlib.sha256(original.read_bytes()).hexdigest()
            data['notices'][0]['sha256'] = digest
            data['notices'][0]['resolution']['owner_decision']['proof']['sha256'] = digest
            manifest.write_text(json.dumps(data))
            with self.subTest(content=content), self.assertRaises(ValueError):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_approved_declaration_cannot_substitute_for_missing_complete_canonical_terms(self):
        from notices import validate_attributions
        self.approved_declaration()
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        about = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>',
                               'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(about, inventory, self.output)

    def test_approved_mit_declaration_does_not_resolve_a_different_selected_licence(self):
        from notices import validate_attributions
        self.approved_declaration()
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        about = {'licenses': [{'id': 'ISC', 'text': 'Copyright (c) [year] [fullname]\n\n' + ISC_GRANT,
                               'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(about, inventory, self.output)

    def test_declaration_only_remains_blocked_even_if_an_owner_decision_object_is_present(self):
        _, manifest = self.approved_declaration()
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution']['kind'] = 'declaration-only'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_approved_declaration_html_keeps_canonical_terms_original_and_dated_owner_provenance(self):
        original, manifest = self.approved_declaration()
        notice = json.loads(manifest.read_text())['notices'][0]
        (self.repo / 'packaging/tool-versions.env').write_text('CARGO_ABOUT_VERSION=0.9.3\n')
        (self.repo / 'packaging/licenses/README-cli.md').write_text('Source guide')
        about = {'licenses': [{'id': 'MIT', 'text': MIT_CANONICAL, 'used_by': [{'crate': self.package}]}]}
        with patch('notices.run', return_value='cargo-about 0.9.3'), \
             patch('notices.about_output', side_effect=[json.dumps(about), '<pre>' + html.escape(MIT_CANONICAL) + '</pre>']), \
             contextlib.redirect_stderr(io.StringIO()):
            generate_notices(self.repo, 'x86_64-unknown-linux-gnu', self.output, cli_only=True, offline=True)
        document = (self.output / 'THIRD-PARTY-NOTICES.html').read_text()
        self.assertIn(html.escape(MIT_CANONICAL), document)
        self.assertIn(html.escape(original.read_text()), document)
        self.assertIn('Owner decision: Accept scoped declarations', document)
        self.assertIn('2026-11-01', document)
        self.assertIn(html.escape(notice['resolution']['owner_decision']['provenance']), document)
        self.assertIn('No named licence notice was supplied', document)
        installed = json.loads((self.output / 'licenses/inventory.json').read_text())['packages'][0]['supplemental'][0]
        self.assertEqual(installed['resolution'], notice['resolution'])

    def test_unknown_resolution_kind_is_rejected(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {'kind': 'ignore-missing', 'license': 'MIT', 'reason': 'Not a valid resolution.'}
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'resolution'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_unused_desktop_supplement_is_not_shipped_with_cli(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['name'] = 'desktop-only'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.assertEqual(inventory[0]['notices'], [])
        self.assertFalse(self.output.exists())

    def test_canonical_mit_fallback_without_original_attribution_fails(self):
        from notices import validate_attributions
        data = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>\nPermission is hereby granted',
                              'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(data, [{'name': 'option-ext', 'version': '0.2.0', 'notices': []}], self.output)

    def test_pinned_original_attribution_resolves_canonical_fallback(self):
        from notices import validate_attributions
        self.supplemental()
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        data = {'licenses': [{'id': 'MIT', 'text': MIT_CANONICAL,
                              'used_by': [{'crate': self.package}]}]}
        validate_attributions(data, inventory, self.output)

    def validate_mit(self, inventory):
        from notices import validate_attributions
        validate_attributions({'licenses': [{'id': 'MIT', 'text': MIT_CANONICAL,
                                            'used_by': [{'crate': self.package}]}]}, inventory, self.output)

    def scoped_supplemental(self, selected=MIT_ORIGINAL.encode('utf-8'),
                            before=b'Compound licence document.\n\n',
                            after=b'\nApache appendix: Copyright [yyyy] [name of copyright owner]\n'):
        original, manifest = self.supplemental()
        original.write_bytes(before + selected + after)
        data = json.loads(manifest.read_text())
        notice = data['notices'][0]
        notice['sha256'] = hashlib.sha256(original.read_bytes()).hexdigest()
        start = len(before.splitlines(keepends=True)) + 1
        notice['scope'] = {'start_line': start, 'end_line': start + len(selected.splitlines(keepends=True)) - 1,
                           'sha256': hashlib.sha256(selected).hexdigest()}
        manifest.write_text(json.dumps(data))
        return original, manifest

    def test_pinned_compound_document_scope_resolves_mit_without_dropping_other_terms(self):
        original, manifest = self.scoped_supplemental()
        notice = json.loads(manifest.read_text())['notices'][0]
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)
        preserved = inventory[0]['supplemental'][0]
        self.assertEqual(preserved['scope'], notice['scope'])
        self.assertEqual((self.output / preserved['path']).read_bytes(), original.read_bytes())

    def test_scope_checksum_uses_raw_crlf_bytes(self):
        selected = MIT_ORIGINAL.replace('\n', '\r\n').encode('utf-8')
        original, _ = self.scoped_supplemental(selected=selected, before=b'Compound document.\r\n\r\n')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)
        self.assertEqual((self.output / inventory[0]['supplemental'][0]['path']).read_bytes(), original.read_bytes())

    def test_scope_hash_cannot_be_computed_after_normalizing_line_endings(self):
        _, manifest = self.scoped_supplemental(selected=MIT_ORIGINAL.replace('\n', '\r\n').encode('utf-8'))
        data = json.loads(manifest.read_text())
        data['notices'][0]['scope']['sha256'] = hashlib.sha256(MIT_ORIGINAL.encode('utf-8')).hexdigest()
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'scope.*checksum'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_stale_scope_hash_is_rejected_even_when_the_whole_file_hash_matches(self):
        _, manifest = self.scoped_supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['scope']['sha256'] = '0' * 64
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'scope.*checksum'):
            gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_scope_requires_integral_inclusive_one_based_bounds_and_a_valid_hash(self):
        original, manifest = self.scoped_supplemental()
        data = json.loads(manifest.read_text())
        valid = data['notices'][0]['scope']
        invalid = [None, [], {},
                   {**valid, 'start_line': True}, {**valid, 'end_line': True},
                   {**valid, 'start_line': 0}, {**valid, 'start_line': -1},
                   {**valid, 'end_line': valid['start_line'] - 1},
                   {**valid, 'end_line': len(original.read_bytes().splitlines(keepends=True)) + 1},
                   {**valid, 'start_line': '3'}, {**valid, 'end_line': float(valid['end_line'])},
                   {**valid, 'sha256': None}, {**valid, 'sha256': 'wrong'},
                   {**valid, 'sha256': 'A' * 64}]
        for scope in invalid:
            data['notices'][0]['scope'] = scope
            manifest.write_text(json.dumps(data))
            with self.subTest(scope=scope), self.assertRaisesRegex(ValueError, 'scope'):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_scope_requires_a_matching_component_binding(self):
        _, manifest = self.scoped_supplemental()
        original = json.loads(manifest.read_text())
        for binding in (None, {'license': 'MIT', 'component': 'other-component@1.0.0', 'role': 'grant'}):
            data = json.loads(json.dumps(original))
            if binding is None:
                del data['notices'][0]['binding']
            else:
                data['notices'][0]['binding'] = binding
            manifest.write_text(json.dumps(data))
            with self.subTest(binding=binding), self.assertRaisesRegex(ValueError, 'scope.*binding'):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_scope_cannot_turn_a_different_selected_licence_binding_into_mit(self):
        _, manifest = self.scoped_supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding']['license'] = 'ISC'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_every_placeholder_inside_the_entire_selected_scope_still_blocks_attribution(self):
        self.scoped_supplemental(selected=('Copyright (c) 2026 Original Authors\n'
                                          'Copyright (c) 2026\n<copyright holders>\n\n' + MIT_GRANT).encode('utf-8'))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_scoped_unnamed_grant_needs_its_reviewed_resolution_not_an_outside_holder(self):
        _, manifest = self.scoped_supplemental(selected=('This crate as a whole is available under MIT.\n\n' + MIT_GRANT).encode('utf-8'),
                                               before=b'Copyright (c) 2026 Unrelated Component Authors\n\n')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {'kind': 'original-grant-no-named-header', 'license': 'MIT',
                                           'reason': 'The exact pinned whole-crate MIT section contains no named header; later component notices remain separate.'}
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)

    def test_a_valid_scope_does_not_change_declaration_only_into_a_resolution(self):
        _, manifest = self.scoped_supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {'kind': 'declaration-only', 'license': 'MIT',
                                           'reason': 'This review remains unresolved.'}
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_scoped_generator_renders_the_entire_original_and_scope_provenance(self):
        original, manifest = self.scoped_supplemental()
        notice = json.loads(manifest.read_text())['notices'][0]
        (self.repo / 'packaging/tool-versions.env').write_text('CARGO_ABOUT_VERSION=0.9.3\n')
        (self.repo / 'packaging/licenses/README-cli.md').write_text('Source guide')
        about = {'licenses': [{'id': 'MIT', 'text': MIT_CANONICAL, 'used_by': [{'crate': self.package}]}]}
        with patch('notices.run', return_value='cargo-about 0.9.3'), \
             patch('notices.about_output', side_effect=[json.dumps(about), '<p>Canonical MIT</p>']), \
             contextlib.redirect_stderr(io.StringIO()):
            generate_notices(self.repo, 'x86_64-unknown-linux-gnu', self.output, cli_only=True, offline=True)
        document = (self.output / 'THIRD-PARTY-NOTICES.html').read_text(encoding='utf-8')
        self.assertIn(html.escape(original.read_text(encoding='utf-8')), document)
        self.assertIn(f'Pinned evidence scope: lines {notice["scope"]["start_line"]}-{notice["scope"]["end_line"]}', document)
        self.assertIn(notice['scope']['sha256'], document)
        self.assertIn(notice['url'], document)
        installed = json.loads((self.output / 'licenses/inventory.json').read_text())['packages'][0]['supplemental'][0]
        self.assertEqual(installed['scope'], notice['scope'])
        self.assertEqual((self.output / 'licenses' / installed['path']).read_bytes(), original.read_bytes())

    def test_explicit_root_links_bind_nested_mit_grants_without_dropping_other_notices(self):
        _, manifest = self.supplemental()
        root_notice = self.crate / 'LICENSE'
        root_notice.write_text('Inherited code is offered under [MIT](.licenses/Original-MIT) OR Apache-2.0.\n'
                               'Modifications are Copyright (c) 2026 Current Authors under MIT OR Apache-2.0.\n'
                               'Retain every inherited and new notice.\n')
        selected = self.crate / '.licenses/Original-MIT'
        selected.parent.mkdir()
        selected.write_text(MIT_ORIGINAL)
        apache = selected.with_name('Original-Apache')
        apache.write_text('Complete inherited Apache original with its own example [name of copyright owner].\n')
        data = json.loads(manifest.read_text())
        data['notices'][0].pop('file')
        data['notices'][0]['source_member'] = '.licenses/Original-MIT'
        data['notices'][0]['sha256'] = hashlib.sha256(selected.read_bytes()).hexdigest()
        data['notices'][0]['binding']['reason'] = 'The preserved root LICENSE expressly links this full MIT original for inherited package code.'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)
        for original in (root_notice, selected, apache):
            copied = self.output / 'dependencies/option-ext-0.2.0' / original.relative_to(self.crate)
            self.assertEqual(copied.read_bytes(), original.read_bytes())

    def test_nested_gnu_licence_document_copyright_does_not_clear_mit_fallback(self):
        original = self.crate / 'vendor/COPYING'
        original.parent.mkdir()
        original.write_text('GNU LIBRARY GENERAL PUBLIC LICENSE\n'
                            'Copyright (C) 1991, 1999 Free Software Foundation, Inc.\n')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_multiline_holder_placeholder_rejects_the_complete_candidate(self):
        (self.crate / 'LICENSE').write_text('Copyright (c) 2026\n<copyright holders>\n\n' + MIT_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_root_notice_named_mit_but_containing_gnu_terms_is_not_a_mit_grant(self):
        (self.crate / 'LICENSE-MIT').write_text('Copyright (C) 1991 Free Software Foundation, Inc.\n'
                                              'GNU GENERAL PUBLIC LICENSE\n')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_full_mit_grant_of_unrelated_nested_component_does_not_clear_package(self):
        original = self.crate / 'vendor/component/LICENSE-MIT'
        original.parent.mkdir(parents=True)
        original.write_text(MIT_ORIGINAL)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_root_third_party_notice_is_not_implicitly_the_package_licence(self):
        (self.crate / 'NOTICE').write_text('Original notice for a separately licensed bundled component.\n\n' + MIT_ORIGINAL)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_root_full_selected_grant_resolves_without_a_filename_licence_hint(self):
        (self.crate / 'COPYING').write_text(MIT_ORIGINAL)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)

    def test_complete_microsoft_style_mit_grant_can_omit_only_its_terminal_period(self):
        original = MIT_ORIGINAL.rstrip().removesuffix('.')
        (self.crate / 'license-mit').write_text('\n'.join('    ' + line for line in original.splitlines()))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)

    def test_terminal_period_equivalence_does_not_accept_a_truncated_final_word(self):
        (self.crate / 'license-mit').write_text(MIT_ORIGINAL.rstrip().removesuffix('.')[:-1])
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_terminal_period_equivalence_does_not_accept_a_joined_suffix(self):
        (self.crate / 'license-mit').write_text(MIT_ORIGINAL.rstrip().removesuffix('.') + 'X')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_root_full_grant_can_match_another_complete_text_of_the_selected_spdx_licence(self):
        from notices import validate_attributions
        (self.crate / 'COPYING').write_text(MIT_ORIGINAL)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        variant = MIT_CANONICAL.replace('this permission notice', 'this permission notice (including the next paragraph)')
        validate_attributions({'licenses': [
            {'id': 'MIT', 'text': MIT_CANONICAL, 'used_by': []},
            {'id': 'MIT', 'text': variant, 'used_by': [{'crate': self.package}]},
        ]}, inventory, self.output)

    def test_root_copyright_of_other_licence_cannot_supply_an_unnamed_mit_grant(self):
        (self.crate / 'COPYING').write_text('Copyright (C) 1991 Free Software Foundation, Inc.\n\n'
                                          'GNU GENERAL PUBLIC LICENSE\nOther licence terms.\n\n' + MIT_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_year_without_a_holder_is_not_named_attribution(self):
        (self.crate / 'LICENSE').write_text('Copyright (c) 2026\n\n' + MIT_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_root_isc_grant_is_not_mit_evidence(self):
        (self.crate / 'LICENSE-MIT').write_text('Copyright (c) 2026 Original Authors\n\n' + ISC_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_actual_root_isc_grant_resolves_selected_isc_not_just_mit(self):
        from notices import validate_attributions
        (self.crate / 'COPYING').write_text('Copyright (c) 2026 Original Authors\n\n' + ISC_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        validate_attributions({'licenses': [{'id': 'ISC', 'text': 'Copyright (c) [year] [fullname]\n\n' + ISC_GRANT,
                                            'used_by': [{'crate': self.package}]}]}, inventory, self.output)

    def test_bound_supplement_resolves_selected_isc_without_assuming_mit(self):
        from notices import validate_attributions
        _, manifest = self.supplemental('Copyright (c) 2026 Original Authors\n\n' + ISC_GRANT)
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding']['license'] = 'ISC'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        validate_attributions({'licenses': [{'id': 'ISC', 'text': 'Copyright (c) [year] [fullname]\n\n' + ISC_GRANT,
                                            'used_by': [{'crate': self.package}]}]}, inventory, self.output)

    def test_root_header_without_complete_grant_does_not_clear_fallback(self):
        (self.crate / 'LICENSE-MIT').write_text('Copyright (c) 2026 Original Authors\n'
                                              'Permission is hereby granted, free of charge.\n')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_unbound_supplement_does_not_clear_fallback(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        del data['notices'][0]['binding']
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_supplement_bound_to_a_different_component_does_not_clear_package(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding']['component'] = 'nested-library@1.0.0'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_supplement_bound_to_a_different_licence_does_not_clear_mit(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding']['license'] = 'ISC'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_bound_source_header_requires_full_grant_for_the_same_component_and_licence(self):
        _, manifest = self.supplemental('Copyright (c) 2019 Original Component Authors\n// See LICENSE-MIT.\n')
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding']['role'] = 'attribution'
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)
        (self.crate / 'LICENSE-MIT').write_text(MIT_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)

    def test_reviewed_grant_only_resolution_still_requires_matching_binding(self):
        _, manifest = self.supplemental(MIT_GRANT)
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {
            'kind': 'original-grant-no-named-header', 'license': 'MIT',
            'reason': 'Exact reviewed original grant, not an inferred holder.',
        }
        del data['notices'][0]['binding']
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_multiline_holder_placeholder_in_a_bound_supplement_is_not_attribution(self):
        self.supplemental('Copyright (c) 2026\n<copyright holders>\n\n' + MIT_GRANT)
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_a_declaration_only_resolution_cannot_be_cleared_by_binding_its_text_as_a_grant(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        data['notices'][0]['resolution'] = {'kind': 'declaration-only', 'license': 'MIT',
                                           'reason': 'Original attribution is still unresolved.'}
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_reviewed_original_grant_by_reference_is_bound_and_keeps_conditions(self):
        _, manifest = self.supplemental('This exact original workspace grants the package under the MIT licence by reference.\n')
        data = json.loads(manifest.read_text())
        notice = data['notices'][0]
        notice['binding'].update(role='grant-by-reference', reason='Exact reviewed workspace grant at the recorded revision.')
        notice['resolution'] = {'kind': 'original-grant-no-named-header', 'license': 'MIT',
                                'reason': 'Original grant has no named header; no holder is inferred.'}
        notice['conditions'] = [{'kind': 'original-condition', 'status': 'unresolved'}]
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)
        self.assertEqual(inventory[0]['supplemental'][0]['conditions'], notice['conditions'])

    def test_named_original_grant_by_reference_resolves_without_an_unnamed_header_exception(self):
        _, manifest = self.supplemental('Copyright (c) 2026 Original Authors\n'
                                       'Licensed under MIT at https://opensource.org/license/MIT.\n')
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding'].update(role='grant-by-reference', reason='Exact reviewed original names its holder and offers MIT.')
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        self.validate_mit(inventory)

    def test_empty_evidence_cannot_support_a_grant_by_reference_resolution(self):
        _, manifest = self.supplemental(' \n')
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding'].update(role='grant-by-reference', reason='A reason alone is not original evidence.')
        data['notices'][0]['resolution'] = {'kind': 'original-grant-no-named-header', 'license': 'MIT',
                                           'reason': 'This must not clear missing evidence.'}
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_empty_copyright_header_cannot_borrow_a_holder_from_the_grant_paragraph(self):
        _, manifest = self.supplemental('Copyright (c)\n\nLicensed under MIT at https://opensource.org/license/MIT.\n')
        data = json.loads(manifest.read_text())
        data['notices'][0]['binding'].update(role='grant-by-reference', reason='The original offers MIT but has no named header.')
        manifest.write_text(json.dumps(data))
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            self.validate_mit(inventory)

    def test_malformed_supplemental_binding_is_rejected(self):
        _, manifest = self.supplemental()
        data = json.loads(manifest.read_text())
        for binding in (None, [], {}, {'license': 'MIT', 'component': '', 'role': 'grant'},
                        {'license': 'MIT', 'component': 'option-ext@0.2.0', 'role': 'ignore'}):
            data['notices'][0]['binding'] = binding
            manifest.write_text(json.dumps(data))
            with self.subTest(binding=binding), self.assertRaisesRegex(ValueError, 'binding'):
                gather_disclosures(self.repo, [self.package], set(), self.output)

    def test_placeholder_original_file_does_not_resolve_canonical_fallback(self):
        from notices import validate_attributions
        self.supplemental('Copyright (c) <year> <copyright holders>\nPermission terms')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        data = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>',
                              'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(data, inventory, self.output)

    def test_permission_clause_is_not_an_original_copyright_attribution(self):
        from notices import validate_attributions
        self.supplemental('Copyright (c) <year> <copyright holders>\n'
                          'The above copyright notice and this permission notice shall be included in all copies.')
        inventory = gather_disclosures(self.repo, [self.package], set(), self.output)
        data = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>',
                              'used_by': [{'crate': self.package}]}]}
        with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
            validate_attributions(data, inventory, self.output)

    def test_generator_refuses_unresolved_fallback_without_leaving_output(self):
        directory = self.repo / 'packaging/licenses'
        directory.mkdir(parents=True)
        (self.repo / 'packaging/tool-versions.env').write_text('CARGO_ABOUT_VERSION=0.9.3\n')
        (directory / 'README-cli.md').write_text('CLI guide')
        data = {'licenses': [{'id': 'MIT', 'text': 'Copyright (c) <year> <copyright holders>',
                              'used_by': [{'crate': self.package}]}]}
        with patch('notices.run', return_value='cargo-about 0.9.3'), \
             patch('notices.about_output', side_effect=[json.dumps(data), '<p>Canonical MIT</p>']), \
             contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(ValueError, 'unresolved.*option-ext'):
                generate_notices(self.repo, 'x86_64-unknown-linux-gnu', self.output, cli_only=True, offline=True)
        self.assertFalse(self.output.exists())

    def test_selected_protocol_graph_keeps_the_checksum_verified_original_source(self):
        name, version = 'wayland-protocols-plasma', '0.3.12'
        root = self.crate.with_name(f'{name}-{version}')
        self.crate.rename(root)
        source = self.source.with_name(f'{name}-{version}.crate')
        self.source.rename(source)
        (self.repo / 'Cargo.lock').write_text(
            f'version = 4\n[[package]]\nname = "{name}"\nversion = "{version}"\n'
            f'source = "{self.package["source"]}"\nchecksum = "{hashlib.sha256(source.read_bytes()).hexdigest()}"\n')
        package = {**self.package, 'name': name, 'version': version, 'manifest_path': str(root / 'Cargo.toml')}
        directory = self.repo / 'packaging/licenses'
        directory.mkdir(parents=True)
        (directory / 'README-cli.md').write_text('Fixture guide')
        (self.repo / 'packaging/tool-versions.env').write_text('CARGO_ABOUT_VERSION=0.9.3\n')
        data = {'licenses': [{'id': 'MIT', 'text': 'Actual original MIT terms', 'used_by': [{'crate': package}]}]}
        with patch('notices.run', return_value='cargo-about 0.9.3'), \
             patch('notices.about_output', side_effect=[json.dumps(data), '<p>MIT wrapper licence</p>']), \
             contextlib.redirect_stderr(io.StringIO()):
            generate_notices(self.repo, 'x86_64-unknown-linux-gnu', self.output, cli_only=True, offline=True)
        copied = self.output / f'licenses/sources/{name}-{version}.crate'
        self.assertTrue(copied.is_file())
        self.assertEqual(copied.read_bytes(), source.read_bytes())

    def test_font_terms_are_complete_and_html_escaped(self):
        rendered = render_notices(
            {"CLI": "<section>CLI terms</section>", "Desktop": "<section>Desktop terms</section>"},
            "IBM Plex Sans <script>not executable</script>", "SIL OFL\ncomplete terms & copyright",
        )
        self.assertIn("CLI terms", rendered)
        self.assertIn("Desktop terms", rendered)
        self.assertIn("&lt;script&gt;not executable&lt;/script&gt;", rendered)
        self.assertIn("SIL OFL\ncomplete terms &amp; copyright", rendered)
        self.assertNotIn("<script>", rendered)

    def test_cli_notice_rendering_does_not_claim_or_ship_desktop_fonts(self):
        rendered = render_notices({'CLI': '<section>CLI terms</section>'}, None, None)
        self.assertIn('CLI terms', rendered)
        self.assertNotIn('IBM Plex Sans', rendered)
        self.assertNotIn('desktop dependency', rendered)

    def test_cli_offline_generation_uses_one_graph_and_no_font_inputs(self):
        packaging = self.repo / 'packaging/licenses'
        packaging.mkdir(parents=True)
        (self.repo / 'packaging/tool-versions.env').write_text('CARGO_ABOUT_VERSION=0.9.3\n')
        (packaging / 'README-cli.md').write_text('CLI-only complete terms and source guide')
        generated = []

        def cargo(args, repo):
            if args == ['cargo', 'about', '--version']:
                return 'cargo-about 0.9.3\n'
            self.assertIn('--frozen', args)
            self.assertNotIn('--features', args)
            generated.append(args)
            destination = Path(args[args.index('--output-file') + 1])
            text = json.dumps({'licenses': [{'id': 'MPL-2.0', 'used_by': [{'crate': self.package}]}]}) if '--format' in args else '<section>CLI licence terms</section>'
            destination.write_text(text, encoding='utf-8')
            return ''

        with patch('notices.run', side_effect=cargo), contextlib.redirect_stderr(io.StringIO()):
            generate_notices(self.repo, 'x86_64-unknown-linux-gnu', self.output, cli_only=True, offline=True)
        self.assertEqual(len(generated), 2)
        self.assertFalse((self.output / 'licenses/fonts').exists())
        self.assertIn('CLI-only', (self.output / 'licenses/README.md').read_text())
        self.assertNotIn('IBM Plex Sans', (self.output / 'THIRD-PARTY-NOTICES.html').read_text())

    def test_generator_reads_utf8_files_instead_of_capturing_about_stdout(self):
        packaging = self.repo / "packaging/licenses"
        packaging.mkdir(parents=True)
        (self.repo / "packaging/tool-versions.env").write_text("CARGO_ABOUT_VERSION=0.9.3\n", encoding="utf-8")
        (packaging / "README.md").write_text("Source and licence guide", encoding="utf-8")
        fonts = self.repo / "assets/fonts"
        fonts.mkdir(parents=True)
        (fonts / "README.md").write_text("IBM Plex Sans — provenance", encoding="utf-8")
        (fonts / "LICENSE.txt").write_text("Copyright © IBM; OFL terms", encoding="utf-8")
        generated = []

        def cargo(args, repo):
            if args == ["cargo", "about", "--version"]:
                return "cargo-about 0.9.3\n"
            if args[:2] == ["cargo", "metadata"]:
                return '{"packages":[]}'
            if "--output-file" not in args:
                raise ValueError("cargo-about rejects stdout when PowerShell is an ancestor")
            destination = Path(args[args.index("--output-file") + 1])
            self.assertNotEqual(str(destination), "-")
            generated.append(args)
            text = json.dumps({"licenses": [{"id": "MPL-2.0", "used_by": [{"crate": self.package}]}]}) if "--format" in args else "<section>Licence © café — complete terms</section>"
            destination.write_bytes(text.encode("utf-8"))
            return "stdout is not the licence document"

        for target in ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"):
            with self.subTest(target=target), patch("notices.run", side_effect=cargo), contextlib.redirect_stderr(io.StringIO()):
                output = self.output / target
                generate_notices(self.repo, target, output)
                html = (output / "THIRD-PARTY-NOTICES.html").read_text(encoding="utf-8")
                self.assertIn("Licence © café — complete terms", html)
                self.assertNotIn("stdout is not", html)
        self.assertEqual(len(generated), 8)  # JSON and HTML for both front ends on both targets.


class RepositorySupplementTests(unittest.TestCase):
    def test_owner_approved_release_declarations_are_limited_to_the_explicit_six_versions(self):
        repo = Path(__file__).resolve().parents[3]
        notices = json.loads((repo / 'packaging/licenses/supplemental.json').read_text())['notices']
        expected = {('block', '0.1.6'), ('leak', '0.1.2'), ('leaky-cow', '0.1.1'),
                    ('mac', '0.1.1'), ('malloc_buf', '0.0.6'), ('seahash', '4.1.0')}
        approved = [n for n in notices if n.get('resolution', {}).get('kind') == 'owner-approved-release-declaration']
        self.assertEqual({(n['name'], n['version']) for n in approved}, expected)
        self.assertEqual(len(approved), len(expected))
        for n in approved:
            with self.subTest(name=n['name']):
                self.assertIn(n['source_member'], ('Cargo.toml', 'Cargo.toml.orig'))
                self.assertEqual(n['binding']['role'], 'grant-by-reference')
                self.assertEqual(n['binding']['license'], 'MIT')
                decision = n['resolution']['owner_decision']
                self.assertEqual(decision['date'], '2026-11-01')
                self.assertEqual(decision['decision'], 'Accept scoped declarations')
                self.assertTrue(decision['provenance'].strip())
                self.assertEqual(decision['proof'], {**{key: n[key] for key in ('source_member', 'sha256', 'crate_sha256')},
                                                     'component': f'{n["name"]}@{n["version"]}', 'license': 'MIT'})
        for n in notices:
            if (n['name'], n['version']) in expected and n.get('source_member') not in ('Cargo.toml', 'Cargo.toml.orig'):
                self.assertIn(n['binding']['role'], ('notice', 'attribution'))

    def test_apple_sdk_original_conditions_remain_unresolved_after_the_separate_owner_decision(self):
        repo = Path(__file__).resolve().parents[3]
        notices = json.loads((repo / 'packaging/licenses/supplemental.json').read_text())['notices']
        expected = set('''block2@0.6.2 dispatch2@0.3.1 objc2@0.6.4 objc2-encode@4.1.0
            objc2-app-kit@0.3.2 objc2-foundation@0.3.2 objc2-cloud-kit@0.3.2 objc2-core-data@0.3.2
            objc2-core-foundation@0.3.2 objc2-core-graphics@0.3.2 objc2-core-image@0.3.2
            objc2-core-location@0.3.2 objc2-core-media@0.3.2 objc2-core-text@0.3.2 objc2-core-video@0.3.2
            objc2-quartz-core@0.3.2 objc2-screen-capture-kit@0.3.2 objc2-user-notifications@0.3.2'''.split())
        actual = set()
        for n in notices:
            for condition in n.get('conditions', []):
                if condition['kind'] == 'apple-sdk-derived-works':
                    actual.add(f'{n["name"]}@{n["version"]}')
                    self.assertEqual(condition['status'], 'unresolved')
                    self.assertTrue(condition['reason'].strip())
        self.assertTrue(expected <= actual)

    def test_compound_and_linked_original_bindings_cover_the_three_reviewed_packages(self):
        repo = Path(__file__).resolve().parents[3]
        notices = json.loads((repo / 'packaging/licenses/supplemental.json').read_text())['notices']
        by_member = {(n['name'], n['version'], n.get('source_member')): n for n in notices}
        expected = {
            ('chrono', '0.4.45', 'LICENSE.txt'): (11, 31, '7f62263ee3860c67a16082f48472ccc4446c576f7ac2daa2264d019e7ad44d68'),
            ('libm', '0.2.16', 'LICENSE.txt'): (1, 21, '4929c31f30df543d6a400c10a5e6091cbe3fc1b6d7f642ea669f5cc433c66082'),
        }
        required = set(expected) | {('granit-parser', '1.3.0', member)
                                    for member in ('LICENSE', '.licenses/ChenYuheng-MIT', '.licenses/Ethiraric-MIT')}
        self.assertFalse(required - by_member.keys(), f'Missing reviewed original members: {sorted(required - by_member.keys())}')
        for key, (start, end, sha256) in expected.items():
            with self.subTest(package=key):
                self.assertIn(key, by_member)
                notice = by_member[key]
                self.assertEqual(notice['scope'], {'start_line': start, 'end_line': end, 'sha256': sha256})
                self.assertEqual(notice['binding']['role'], 'grant')
                self.assertTrue(notice['binding']['reason'].strip())
        self.assertEqual(by_member['libm', '0.2.16', 'LICENSE.txt']['resolution']['kind'], 'original-grant-no-named-header')
        for member in ('LICENSE', '.licenses/ChenYuheng-MIT', '.licenses/Ethiraric-MIT'):
            key = 'granit-parser', '1.3.0', member
            self.assertIn(key, by_member)
            self.assertTrue(by_member[key]['binding']['reason'].strip())

    def test_every_curated_supplement_has_an_explicit_licence_component_and_evidence_role(self):
        repo = Path(__file__).resolve().parents[3]
        manifest = json.loads((repo / 'packaging/licenses/supplemental.json').read_text())
        for notice in manifest['notices']:
            with self.subTest(name=notice['name'], version=notice['version']):
                binding = notice.get('binding')
                self.assertIsInstance(binding, dict)
                self.assertEqual(binding['component'], f'{notice["name"]}@{notice["version"]}')
                self.assertEqual(binding['license'], 'Apache-2.0' if notice['name'] in ('gpui-kit', 'gpui-kit-assets') else 'MIT')
                self.assertIn(binding['role'], ('grant', 'grant-by-reference', 'attribution', 'declaration', 'notice'))
                if binding['role'] == 'grant-by-reference':
                    self.assertTrue(binding['reason'].strip())
                if notice.get('resolution', {}).get('kind') == 'declaration-only':
                    self.assertEqual(binding['role'], 'declaration')

    def test_reviewed_original_notices_are_pinned_to_locked_crates_and_commits(self):
        repo = Path(__file__).resolve().parents[3]
        manifest = json.loads((repo / 'packaging/licenses/supplemental.json').read_text())
        locked = {(p['name'], p['version']): p.get('checksum')
                  for p in tomllib.loads((repo / 'Cargo.lock').read_text())['package']}
        self.assertEqual(manifest['schema_version'], 1)
        required = set('''accesskit@0.24.1 accesskit_consumer@0.38.0
            lyon@1.0.19 lyon_algorithms@1.0.21 lyon_geom@1.0.19 lyon_path@1.0.19 lyon_tessellation@1.0.22
            mac@0.1.1 profiling@1.0.18 profiling-procmacros@1.0.18 pulp-wasm-simd-flag@0.1.1
            rust-i18n-macro@4.2.2 rust-i18n-support@4.2.2 seahash@4.1.0 svg_fmt@0.4.5
            taffy@0.13.0 zune-inflate@0.2.54 pathfinder_geometry@0.5.1 pathfinder_simd@0.5.6
            accesskit_atspi_common@0.19.1 accesskit_unix@0.22.1 gpu-descriptor@0.3.2 gpu-descriptor-types@0.2.0
            harfrust@0.5.2 xim-ctext@0.3.0 xim-parser@0.2.2 accesskit_windows@0.34.0
            zune-core@0.4.12 zune-jpeg@0.4.21 accesskit_macos@0.26.3 block@0.1.6 block2@0.6.2
            dispatch2@0.3.1 leak@0.1.2 leaky-cow@0.1.1 malloc_buf@0.0.6 objc-sys@0.3.5
            objc2@0.5.2 objc2@0.6.4 objc2-encode@4.1.0 objc2-app-kit@0.2.2 objc2-app-kit@0.3.2
            objc2-foundation@0.2.2 objc2-foundation@0.3.2 objc2-cloud-kit@0.3.2 objc2-core-data@0.3.2
            objc2-core-foundation@0.3.2 objc2-core-graphics@0.3.2 objc2-core-image@0.3.2
            objc2-core-location@0.3.2 objc2-core-media@0.3.2 objc2-core-text@0.3.2 objc2-core-video@0.3.2
            objc2-quartz-core@0.3.2 objc2-screen-capture-kit@0.3.2 objc2-user-notifications@0.3.2
            gpui-kit@0.6.6 gpui-kit-assets@0.6.6 chrono@0.4.45 libm@0.2.16 granit-parser@1.3.0'''.split())
        actual = {f'{p["name"]}@{p["version"]}' for p in manifest['notices']}
        self.assertFalse(required - actual, f'Missing original evidence: {sorted(required - actual)}')
        for notice in manifest['notices']:
            with self.subTest(name=notice['name']):
                self.assertEqual(notice['crate_sha256'], locked[notice['name'], notice['version']])
                url = urlsplit(notice['url'])
                self.assertEqual(url.scheme, 'https')
                self.assertIsNone(url.username)
                self.assertIsNone(url.password)
                self.assertRegex(notice['sha256'], r'^[a-f0-9]{64}$')
                self.assertTrue('.crate' in url.path or any(len(part) == 40 and all(c in '0123456789abcdef' for c in part) for part in url.path.split('/')))
                self.assertNotEqual('file' in notice, 'source_member' in notice)
                if 'file' in notice:
                    self.assertRegex(notice['file'], r'^[A-Za-z0-9_+.-]+$')
                    original = repo / 'packaging/licenses/supplemental' / notice['file']
                    self.assertFalse(original.is_symlink())
                    self.assertEqual(hashlib.sha256(original.read_bytes()).hexdigest(), notice['sha256'])
                else:
                    self.assertFalse(any(part in ('', '.', '..') for part in notice['source_member'].split('/')))
                    self.assertNotIn('\\', notice['source_member'])
                    self.assertNotIn(':', notice['source_member'])
                if 'scope' in notice:
                    scope = notice['scope']
                    self.assertIs(type(scope['start_line']), int)
                    self.assertIs(type(scope['end_line']), int)
                    self.assertGreaterEqual(scope['start_line'], 1)
                    self.assertGreaterEqual(scope['end_line'], scope['start_line'])
                    self.assertRegex(scope['sha256'], r'^[a-f0-9]{64}$')


if __name__ == "__main__":
    unittest.main()
