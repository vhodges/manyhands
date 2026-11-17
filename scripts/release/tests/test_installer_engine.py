"""Opt-in actual Inno 7.1.1 engine fixtures; never a native acceptance claim.

On Windows INNO_COMPILER selects the provisioned pinned compiler. On Linux use
INNO_COMPILER=/tmp/opencode/manyhands-inno-7.1.1/ISCC.exe and a fresh owned
MANYHANDS_INNO_WINEPREFIX (with wine on PATH); no existing Wine prefix is used.
"""
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import uuid

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'scripts/release'))
from installer import write_managed_digests, write_managed_file_list

COMPILER = os.environ.get('INNO_COMPILER')
WINE = os.environ.get('MANYHANDS_INNO_WINEPREFIX')
ENABLED = COMPILER and (os.name == 'nt' or (WINE and shutil.which('wine')))


@unittest.skipUnless(ENABLED, 'actual Inno fixtures need pinned compiler and Windows or an owned Wine prefix')
class InstallerEngineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        command = [str(COMPILER), '--version']
        environment = {**os.environ, 'WINEDEBUG': '-all'}
        if os.name != 'nt':
            command.insert(0, 'wine')
            environment['WINEPREFIX'] = WINE
        result = subprocess.run(command, env=environment, capture_output=True, text=True, timeout=120)
        if result.returncode or result.stdout.strip() != '7.1.1':
            raise AssertionError('Engine fixtures require exactly pinned Inno 7.1.1: ' + result.stdout + result.stderr)

    def setUp(self):
        # Wine marks its Z: host-filesystem mapping as a reparse point. Keep
        # installed fixture state inside the prefix's real C: tree rather than
        # weakening the production ancestor checks for the test harness.
        parent = str(Path(WINE) / 'drive_c') if os.name != 'nt' else None
        self.temp = tempfile.TemporaryDirectory(prefix='manyhands-installer-engine-', dir=parent)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.env = {**os.environ, 'WINEDEBUG': '-all'}
        if WINE:
            self.env['WINEPREFIX'] = WINE

    def winpath(self, path):
        if os.name == 'nt':
            return str(path)
        if Path(path).is_relative_to(Path(WINE) / 'drive_c'):
            return 'C:\\' + str(Path(path).relative_to(Path(WINE) / 'drive_c')).replace('/', '\\')
        return 'Z:' + str(path).replace('/', '\\')

    def run_tool(self, executable, *args, timeout=120):
        command = [str(executable), *args]
        if os.name != 'nt':
            command.insert(0, 'wine')
        return subprocess.run(command, env=self.env, capture_output=True, text=True, timeout=timeout)

    def test_managed_path_validation_scales_to_a_thousand_entries(self):
        names = ['bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md']
        names += [f'licenses/dependencies/review-crate-{i:04}/native/component/COPYING' for i in range(995)]
        managed, digests = self.write_compiler_inputs({name: '0' * 64 for name in names})
        result = self.run_tool(COMPILER, '/Q', '/O' + self.winpath(self.root),
                               self.winpath(REPO / 'scripts/release/tests/installer_scale.iss'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        report = self.root / 'scale.txt'
        result = self.run_tool(self.root / 'installer-scale-test.exe', '/VERYSILENT', '/SUPPRESSMSGBOXES', '/SP-',
                               '/ManagedFiles=' + self.winpath(managed), '/ManagedDigests=' + self.winpath(digests),
                               '/ResultFile=' + self.winpath(report), timeout=300)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        timings = dict(line.split('=', 1) for line in report.read_text().splitlines())
        self.assertEqual(int(timings.pop('file_count')), 1000)
        self.assertLess(int(timings.pop('membership_ms')), 5000, 'Pruning membership lookups stalled Setup.')
        self.assertEqual(len(timings), 7)
        milliseconds = [int(value) for value in timings.values()]
        print('Pinned-engine 1000-path validation timings (ms): ' + repr(milliseconds), flush=True)
        self.assertLess(max(milliseconds), 5000, 'A validation pass stalled Setup: ' + repr(timings))
        self.assertLess(sum(milliseconds), 20000, 'Repeated validation stalled Setup: ' + repr(timings))

    def test_mandatory_signing_failure_retries_are_bounded_without_unsigned_output(self):
        production = (REPO / 'packaging/windows/manyhands.iss').read_text()
        count = int(re.search(r'(?m)^SignToolRetryCount=(\d+)$', production)[1])
        delay = int(re.search(r'(?m)^SignToolRetryDelay=(\d+)$', production)[1])
        self.assertEqual((count, delay), (2, 2000))
        attempts = self.root / 'attempts.txt'
        callback = self.root / 'fail-signing.cmd'
        callback.write_text('@echo off\n>>"' + self.winpath(attempts) + '" echo attempted\nexit /b 1\n')
        started = time.monotonic()
        result = self.run_tool(COMPILER, '/Q', '/O' + self.winpath(self.root), '/DRetryCount=' + str(count),
                               '/DRetryDelay=' + str(delay), '/Sfixture=cmd.exe /D /C $q$q' + self.winpath(callback) + '$q $f$q',
                               self.winpath(REPO / 'scripts/release/tests/installer_retry.iss'))
        self.assertNotEqual(result.returncode, 0, 'Failed mandatory signing must fail compilation')
        self.assertTrue(attempts.exists(), result.stdout + result.stderr)
        self.assertEqual(attempts.read_text().splitlines(), ['attempted'] * (count + 1), result.stdout + result.stderr)
        self.assertGreaterEqual(time.monotonic() - started, count * delay / 1000)
        self.assertFalse((self.root / 'installer-retry-test.exe').exists(), 'No unsigned fallback output is allowed')

    def test_production_definition_compiles_only_with_output_disabled(self):
        payload = self.root / 'payload with spaces'
        (payload / 'bin').mkdir(parents=True)
        (payload / 'licenses').mkdir()
        (payload / 'LICENSE').write_text('Output-disabled syntax fixture, not a release payload.')
        (payload / 'licenses/README.md').write_text('Output-disabled disclosure fixture.')
        for name in ('manyhands.exe', 'manyhands-cli.exe'):
            (payload / 'bin' / name).write_text('Never executed or released.')
        managed, digests = self.write_compiler_inputs({path.relative_to(payload).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
                                                       for path in payload.rglob('*') if path.is_file()})
        result = self.run_tool(COMPILER, '/Q', '/O-', '/O' + self.winpath(self.root),
                                '/DPayloadDir=' + self.winpath(payload), '/DManagedFiles=' + self.winpath(managed),
                               '/DManagedDigests=' + self.winpath(digests),
                               '/DAppVersion=2.0.0', '/DNumericVersion=2.0.0.0',
                               '/Smanyhands=cmd.exe /D /C exit /b 1',
                               self.winpath(REPO / 'packaging/windows/manyhands.iss'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(list(self.root.glob('*setup.exe')), [])

    def test_existing_policy_and_exit_42_completion_fixtures(self):
        result = self.run_tool(COMPILER, '/Q', '/O' + self.winpath(self.root), '/DFixtureId=' + uuid.uuid4().hex,
                               self.winpath(REPO / 'scripts/release/tests/installer_policy.iss'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        report = self.root / 'policy.txt'
        result = self.run_tool(self.root / 'installer-policy-test.exe', '/VERYSILENT', '/SUPPRESSMSGBOXES',
                               '/NORESTART', '/SP-', '/ResultFile=' + self.winpath(report))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(report.read_text(), 'passed')
        result = self.run_tool(COMPILER, '/Q', '/O' + self.winpath(self.root),
                               self.winpath(REPO / 'scripts/release/tests/installer_completion.iss'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        for failure, expected in [('1', 42), ('0', 0)]:
            report = self.root / ('completion-' + failure + '.txt')
            result = self.run_tool(self.root / 'installer-completion-test.exe', '/VERYSILENT', '/SUPPRESSMSGBOXES',
                                   '/NORESTART', '/SP-', '/Failure=' + failure, '/ResultFile=' + self.winpath(report))
            self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
            self.assertEqual(report.read_text(), 'attempted')

    def write_compiler_inputs(self, payload_hashes, *, directory=None):
        directory = self.root if directory is None else directory
        managed = directory / 'managed-files.txt'
        digests = directory / 'managed-digests.txt'
        write_managed_file_list(managed, list(payload_hashes))
        write_managed_digests(digests, payload_hashes, managed.read_bytes(),
                              (REPO / 'packaging/windows/INNO-LICENSE.txt').read_bytes())
        return managed, digests

    def compile_lifecycle_fixture(self, *, invalid=None):
        source = REPO / 'scripts/release/tests/installer_lifecycle.iss'
        digest = hashlib.sha256(source.read_bytes()).hexdigest()
        directory = self.root
        if invalid:
            directory = self.root / ('inputs-' + invalid)
            directory.mkdir()
        managed, digests = self.write_compiler_inputs({name: digest for name in
                                                      ('bin/manyhands.exe', 'bin/manyhands-cli.exe', 'licenses/README.md')},
                                                     directory=directory)
        if invalid:
            replacement = {'traversal': b'bin\\..\\user-state.txt', 'collision': b'bin\\Manyhands.exe'}[invalid]
            managed.write_bytes(managed.read_bytes().replace(b'bin\\manyhands-cli.exe', replacement))
        result = self.run_tool(COMPILER, '/Q', '/O' + self.winpath(self.root),
                                '/DManagedFiles=' + self.winpath(managed), '/DFixtureId=' + uuid.uuid4().hex,
                               '/DManagedDigests=' + self.winpath(digests), self.winpath(source))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def run_lifecycle_scenario(self, mode, expected, *, ancestor_junction=False):
        scenario = self.root / (mode + ('-ancestor-junction' if ancestor_junction else ''))
        scenario.mkdir()
        fixture = scenario
        if ancestor_junction:
            target = scenario / 'relocated-profile'
            target.mkdir()
            fixture = scenario / 'profile-junction'
            linked = self.run_tool('cmd.exe', '/D', '/C', 'mklink', '/J', self.winpath(fixture), self.winpath(target))
            self.assertEqual(linked.returncode, 0, linked.stdout + linked.stderr)
        report = scenario / 'passed.txt'
        result = self.run_tool(self.root / 'installer-lifecycle-test.exe', '/SILENT', '/SUPPRESSMSGBOXES',
                               '/NORESTART', '/SP-', '/Mode=' + mode, '/FixtureDir=' + self.winpath(fixture),
                               '/ResultFile=' + self.winpath(report), '/LOG=' + self.winpath(scenario / 'setup.log'))
        evidence = result.stdout + result.stderr + (scenario / 'setup.log').read_text(encoding='utf-8-sig', errors='replace')
        evidence += '\nstale_bin_exists=' + str((fixture / 'app/bin/stale.dll').exists())
        evidence += '\nstale_licence_exists=' + str((fixture / 'app/licenses/old-crate/COPYING').exists())
        self.assertEqual(result.returncode, expected, evidence)
        self.assertTrue(report.exists(), 'Engine did not pass deinitialization assertions: ' + mode + '\n' + evidence)
        self.assertEqual(report.read_text(), 'passed')
        if expected == 0:
            self.assertFalse((fixture / 'app/licenses/installer/managed-digests.txt').exists(),
                              'Embedded digest trust metadata must never be installed')

    def test_profile_junction_above_app_allows_fresh_and_upgrade(self):
        self.compile_lifecycle_fixture()
        for mode in ('fresh', 'success'):
            with self.subTest(mode=mode):
                self.run_lifecycle_scenario(mode, 0, ancestor_junction=True)

    def test_reparse_points_at_and_below_app_remain_rejected(self):
        self.compile_lifecycle_fixture()
        for mode in ('root-reparse', 'pre-reparse'):
            with self.subTest(mode=mode):
                self.run_lifecycle_scenario(mode, 7)

    def test_invalid_embedded_paths_are_rejected_before_checkpoint(self):
        for invalid in ('traversal', 'collision'):
            with self.subTest(invalid=invalid):
                self.compile_lifecycle_fixture(invalid=invalid)
                self.run_lifecycle_scenario('invalid-input-' + invalid, 1)

    def test_application_boundary_rejects_siblings_and_dotdot(self):
        self.compile_lifecycle_fixture()
        self.run_lifecycle_scenario('boundary-checks', 7)

    def test_existing_expected_paths_with_wrong_bytes_block_completion(self):
        self.compile_lifecycle_fixture()
        for mode in ('old-binary', 'old-readonly', 'old-cli', 'old-license', 'old-manifest',
                     'forged-installed-digests', 'forged-temporary-digests'):
            with self.subTest(mode=mode):
                self.run_lifecycle_scenario(mode, 42)

    def test_actual_install_lifecycle_and_pruning(self):
        self.compile_lifecycle_fixture()
        for mode, expected in [('preparecancel', 7), ('startfail', 3), ('startpartial', 3),
                               ('midcancel', 5), ('pre-reparse', 7),
                               ('missing', 42), ('post-reparse', 42),
                               ('prunefail', 42), ('success', 0), ('fresh', 0)]:
            with self.subTest(mode=mode):
                self.run_lifecycle_scenario(mode, expected)


if __name__ == '__main__':
    unittest.main(verbosity=2)
