"""Bind a native signed Inno installer to its verified portable ZIP payload."""

import argparse
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


def verify_installer_payload(staging: Path, archive: Path, release: dict) -> None:
    from verify import validate_payload, verify_archive
    validate_payload(staging, release, TARGET)
    with tempfile.TemporaryDirectory(prefix='manyhands-installer-payload-') as temp:
        destination = Path(temp) / 'extracted'
        verify_archive(archive, release, TARGET, extract_to=destination)
        original = destination / payload_name(release, TARGET)

        def files(root):
            return {path.relative_to(root).as_posix(): hash_file(path) for path in root.rglob('*') if path.is_file()}

        if files(staging) != files(original):
            raise ValueError('installer payload files differ from the verified portable ZIP')


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
        verify_installer_payload(args.staging, args.archive, manifest)
        if args.command == 'parameters':
            write_json(args.output, {'version': manifest['version'], 'numeric_version': numeric_version(manifest['version']),
                                     'name': installer_name(manifest), 'app_id': APP_ID, 'compiler': compiler_config()})
        else:
            if args.archive is None or args.installer is None:
                raise ValueError('inventory needs --archive and --installer')
            # Check the archive's structural/source identity as well as matching
            # the retained extraction. Native signature checks run before this.
            write_json(args.output, installer_inventory(manifest, args.archive, manifest, args.installer))
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        print(f'release: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
