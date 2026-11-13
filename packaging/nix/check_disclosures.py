"""Check installed source disclosures against selected, checksum-locked crates."""

import argparse
import hashlib
import json
from pathlib import Path
import re


SOURCE_PACKAGES = {'option-ext', 'libgit2-sys', 'wayland-protocols-plasma'}


def original_file(root: Path, relative: str) -> Path:
    current = root
    for part in relative.split('/'):
        current = current / part
        if current.is_symlink():
            raise ValueError(f'linked installed source/protocol file: {relative}')
    if not current.is_file() or not current.stat().st_size:
        raise ValueError(f'missing or empty installed source/protocol file: {relative}')
    return current


def validate_disclosures(licenses: Path, locked_sources: list, *, desktop: bool) -> None:
    locked = {(package['name'], package['version']): package['checksum'] for package in locked_sources}
    inventory = json.loads((licenses / 'inventory.json').read_text(encoding='utf-8'))
    if inventory.get('schema_version') != 1 or not isinstance(inventory.get('packages'), list):
        raise ValueError('invalid installed disclosure inventory')
    selected_names = set()
    for package in inventory['packages']:
        name, version = package['name'], package['version']
        if name not in SOURCE_PACKAGES:
            continue
        if (name, version) not in locked:
            raise ValueError(f'installed source is not a locked package: {name} {version}')
        selected_names.add(name)
        if not re.fullmatch(r'[A-Za-z0-9_-]+', name) or not re.fullmatch(r'[A-Za-z0-9.+-]+', version):
            raise ValueError('invalid locked source identity')
        if name == 'wayland-protocols-plasma' and not desktop:
            raise ValueError('CLI disclosures unexpectedly contain a desktop protocol package')
        relative = f'sources/{name}-{version}.crate'
        if package.get('source') != relative:
            raise ValueError(f'installed source path does not match the locked identity: {name} {version}')
        digest = hashlib.sha256(original_file(licenses, relative).read_bytes()).hexdigest()
        if digest != package.get('source_sha256') or digest != locked[name, version]:
            raise ValueError(f'installed source checksum differs from inventory/Cargo.lock: {name} {version}')
        if name == 'wayland-protocols-plasma':
            prefix = f'dependencies/{name}-{version}/plasma-wayland-protocols'
            for member in ('src/protocols/blur.xml', 'COPYING.LIB'):
                relative = f'{prefix}/{member}'
                if relative not in package.get('notices', []):
                    raise ValueError(f'protocol original is absent from the selected inventory: {relative}')
                original_file(licenses, relative)
    missing = {name for name, _ in locked} - selected_names
    if missing:
        raise ValueError(f'locked source packages are absent from the installed inventory: {", ".join(sorted(missing))}')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--licenses', type=Path, required=True)
    parser.add_argument('--locked-sources', type=Path, required=True)
    parser.add_argument('--desktop', action='store_true')
    args = parser.parse_args()
    validate_disclosures(args.licenses, json.loads(args.locked_sources.read_text()), desktop=args.desktop)


if __name__ == '__main__':
    main()
