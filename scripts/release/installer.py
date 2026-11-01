"""Bind a native signed Inno installer to its verified portable ZIP payload."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import zipfile

from common import IDENTITY_KEYS, hash_file, parse_release_tag, payload_name, read_release, validate_release, write_json

REPO = Path(__file__).resolve().parents[2]
TARGET = 'x86_64-pc-windows-msvc'
APP_ID = 'io.github.vhodges.manyhands'
MANAGED_MANIFEST = 'licenses/installer/managed-files.txt'
INSTALLER_LICENSE = 'licenses/installer/INNO-LICENSE.txt'


def managed_file_list(payload_files: list[str]) -> list[str]:
    """Canonical Windows paths for installer-owned trees, never user/root data.

    Keep the deliberately narrow ASCII/Win32 policy in sync with the Pascal
    reader. Reject aliases at every directory level, not just full filenames.
    """
    # ExtractTemporaryFile identifies the two compiler inputs by basename.
    # No portable entry may shadow either embedded trust input, in any tree.
    if any(isinstance(name, str) and name.rsplit('/', 1)[-1].lower() in
           ('managed-files.txt', 'managed-digests.txt') for name in payload_files):
        raise ValueError('reserved embedded managed-file input basename')
    nodes = {}
    managed = []
    for name in [*payload_files, INSTALLER_LICENSE, MANAGED_MANIFEST]:
        if not isinstance(name, str) or not name or '\\' in name:
            raise ValueError('invalid managed file path')
        parts = name.split('/')
        if parts[0].lower() in ('bin', 'licenses') and parts[0] not in ('bin', 'licenses'):
            raise ValueError('managed path root must use canonical case')
        for index, part in enumerate(parts):
            if not part or part in ('.', '..') or part[-1] in '. ' or any(
                ord(char) < 32 or ord(char) > 126 or char in '<>:"|?*' for char in part
            ) or re.fullmatch(r'CON|CONIN\$|CONOUT\$|PRN|AUX|NUL|COM[1-9]|LPT[1-9]',
                             part.split('.', 1)[0].rstrip(' '), re.I):
                raise ValueError(f'unsafe managed file path: {name}')
            prefix = '/'.join(parts[:index + 1])
            key = prefix.upper()
            is_file = index == len(parts) - 1
            if key in nodes:
                previous, was_file = nodes[key]
                if previous != prefix or was_file or is_file:
                    raise ValueError(f'colliding or reserved managed file path: {name}')
            nodes[key] = (prefix, is_file)
        if parts[0] in ('bin', 'licenses'):
            if len(parts) < 2:
                raise ValueError('managed tree root is not a file')
            managed.append(name.replace('/', '\\'))
    if not {'bin\\manyhands.exe', 'bin\\manyhands-cli.exe', 'licenses\\README.md'}.issubset(managed):
        raise ValueError('managed file list is missing required payload files')
    return sorted(managed, key=str.upper)


def write_managed_file_list(destination: Path, payload_files: list[str]) -> None:
    data = '\ufeff' + '\r\n'.join(managed_file_list(payload_files)) + '\r\n'
    # Production uses a fresh compiler directory. Exclusive creation also
    # prevents an existing symlink from redirecting this write into staging.
    with destination.open('xb') as output:
        output.write(data.encode('utf-8'))


def managed_digest_list(payload_hashes: dict[str, str], manifest: bytes, installer_license: bytes) -> list[str]:
    """Bind every managed file to verified ZIP bytes or exact installer additions.

    This list is embedded only (dontcopy), not installed: the installed file
    manifest has a digest here, without a digest-list self-hash cycle.
    """
    if not isinstance(payload_hashes, dict) or any(
        not isinstance(value, str) or not re.fullmatch(r'[0-9a-f]{64}', value) for value in payload_hashes.values()
    ):
        raise ValueError('managed payload digests must be canonical SHA-256 values')
    paths = managed_file_list(list(payload_hashes))
    canonical = ('\ufeff' + '\r\n'.join(paths) + '\r\n').encode('utf-8')
    if not isinstance(manifest, bytes) or manifest != canonical:
        raise ValueError('managed digests require the exact canonical file manifest bytes')
    if not isinstance(installer_license, bytes) or not installer_license:
        raise ValueError('managed digests require the exact nonempty installer licence bytes')
    hashes = {name.replace('/', '\\'): value for name, value in payload_hashes.items()}
    hashes[MANAGED_MANIFEST.replace('/', '\\')] = hashlib.sha256(manifest).hexdigest()
    hashes[INSTALLER_LICENSE.replace('/', '\\')] = hashlib.sha256(installer_license).hexdigest()
    return [hashes[name] + '  ' + name for name in paths]


def write_managed_digests(destination: Path, payload_hashes: dict[str, str], manifest: bytes, installer_license: bytes) -> None:
    records = managed_digest_list(payload_hashes, manifest, installer_license)
    with destination.open('xb') as output:
        output.write(('\ufeff' + '\r\n'.join(records) + '\r\n').encode('utf-8'))


def installer_name(release: dict) -> str:
    return payload_name(release, TARGET) + '-setup.exe'


def numeric_version(version: str) -> str:
    parse_release_tag('v' + version)
    parts = version.split('-', 1)[0].split('.')
    if any(len(part) > 5 or int(part) > 65535 for part in parts):
        raise ValueError('Windows file version components must not exceed 65535')
    return '.'.join(parts) + '.0'


def compiler_config() -> dict:
    config = json.loads((REPO / 'packaging/windows/inno-setup.json').read_text(encoding='utf-8'))
    version = config.get('version')
    if not isinstance(version, str) or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', version):
        raise ValueError('invalid pinned Inno Setup version')
    url = f'https://github.com/jrsoftware/issrc/releases/download/is-{version.replace(".", "_")}/innosetup-{version}-x64.exe'
    if config.get('url') != url or not re.fullmatch(r'[0-9a-f]{64}', config.get('sha256', '')):
        raise ValueError('Inno Setup needs a versioned upstream HTTPS URL and SHA-256')
    return {key: config[key] for key in ('version', 'url', 'sha256')}


def file_record(path: Path) -> dict:
    if path.is_symlink() or not path.is_file() or not path.stat().st_size:
        raise ValueError(f'missing, empty, or linked installer input: {path.name}')
    return {'name': path.name, 'size': path.stat().st_size, 'sha256': hash_file(path)}


def installer_inventory(release: dict, archive: Path, package: dict, setup: Path) -> dict:
    validate_release(release)
    validate_release(package)
    if any(package[key] != release[key] for key in IDENTITY_KEYS) or package.get('target') != TARGET:
        raise ValueError('installer payload source identity/target mismatch')
    signer = package.get('signer')
    if package.get('signing_mode') != 'self-signed-authenticode' or not isinstance(signer, dict) or any(
        not isinstance(signer.get(key), str) or not re.fullmatch(pattern, signer[key])
        for key, pattern in (('thumbprint_sha1', r'[A-F0-9]{40}'), ('fingerprint_sha256', r'[A-F0-9]{64}'))
    ):
        raise ValueError('installer needs completed payload signing and public signer identity')
    if setup.name != installer_name(release) or archive.name != payload_name(release, TARGET) + '.zip':
        raise ValueError('installer or source archive filename does not match the release')
    return {**{key: release[key] for key in IDENTITY_KEYS}, 'kind': 'manyhands-windows-installer',
            'target': TARGET, 'app_id': APP_ID, 'numeric_version': numeric_version(release['version']),
            'compiler': compiler_config(), 'signing_mode': package['signing_mode'], 'signer': signer,
            'source_archive': file_record(archive), 'installer': file_record(setup)}


def validate_installer(release: dict, setup: Path, inventory: dict, archive: Path, package: dict) -> None:
    expected = installer_inventory(release, archive, package, setup)
    if inventory != expected:
        raise ValueError('installer inventory identity, source archive, signer, compiler, or installer bytes mismatch')


def verify_installer_payload(staging: Path, archive: Path, release: dict) -> dict[str, str]:
    from verify import validate_payload, verify_archive
    validate_payload(staging, release, TARGET)
    with tempfile.TemporaryDirectory(prefix='manyhands-installer-payload-') as temp:
        destination = Path(temp) / 'extracted'
        verify_archive(archive, release, TARGET, extract_to=destination)
        original = destination / payload_name(release, TARGET)

        def files(root):
            return {path.relative_to(root).as_posix(): hash_file(path) for path in root.rglob('*') if path.is_file()}

        original_files = files(original)
        if files(staging) != original_files:
            raise ValueError('installer payload files differ from the verified portable ZIP')
        # Retain the hashes computed from the verified ZIP, not fresh hashes of
        # mutable staging inputs after this equality check.
        return original_files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('parameters', 'inventory'))
    parser.add_argument('--staging', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--installer', type=Path)
    args = parser.parse_args()
    try:
        from windows import verify_windows
        manifest = verify_windows(args.staging)
        if args.archive is None:
            raise ValueError('installer construction needs its verified --archive')
        payload_hashes = verify_installer_payload(args.staging, args.archive, manifest)
        if args.command == 'parameters':
            # Kept outside staging: both pre/post compile comparisons still
            # require exact ZIP equality, including every disclosure byte.
            managed = args.output.resolve().parent / 'managed-files.txt'
            digests = args.output.resolve().parent / 'managed-digests.txt'
            if managed.is_relative_to(args.staging.resolve()):
                raise ValueError('managed file list must be outside the portable payload')
            license_path = REPO / 'packaging/windows/INNO-LICENSE.txt'
            file_record(license_path)
            license_bytes = license_path.read_bytes()
            write_managed_file_list(managed, list(payload_hashes))
            write_managed_digests(digests, payload_hashes, managed.read_bytes(), license_bytes)
            write_json(args.output, {'version': manifest['version'], 'numeric_version': numeric_version(manifest['version']),
                                     'name': installer_name(manifest), 'app_id': APP_ID, 'compiler': compiler_config(),
                                     'managed_files': str(managed), 'managed_digests': str(digests)})
        else:
            if args.installer is None:
                raise ValueError('inventory needs --installer')
            # Check the archive's structural/source identity as well as matching
            # the retained extraction. Native signature checks run before this.
            write_json(args.output, installer_inventory(manifest, args.archive, manifest, args.installer))
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        print(f'release: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
