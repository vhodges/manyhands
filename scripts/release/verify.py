"""Verify package contents and smoke-test the extracted CLI in isolation."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile

from common import IDENTITY_KEYS, TARGETS, archive_inventory, payload_name, read_release, target_info, validate_release, write_json

DISCLOSURES = (
    "README.md", "LICENSE", "THIRD-PARTY-NOTICES.html", "release.json",
    "licenses/README.md", "licenses/inventory.json",
    "licenses/fonts/README.md", "licenses/fonts/LICENSE.txt",
)


def safe_path(name: str) -> PurePosixPath:
    if not isinstance(name, str):
        raise ValueError("archive/disclosure path must be a string")
    path = PurePosixPath(name)
    if not name or path.is_absolute() or "\\" in name or ":" in name or any(
        part in ("", ".", "..") for part in name.split("/")
    ):
        raise ValueError(f"unsafe archive/disclosure path: {name}")
    return path


def require_file(root: Path, relative: str) -> Path:
    path = root.joinpath(*safe_path(relative).parts)
    if not path.is_file() or not path.stat().st_size:
        raise ValueError(f"missing or empty package file: {relative}")
    return path


def validate_payload(root: Path, release: dict, target: str, *, final: bool = True) -> dict:
    validate_release(release)
    platform = target_info(target)
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"package contains a symbolic link: {path.relative_to(root)}")
        if not path.is_file():
            continue
        if path.suffix.lower() in (".pfx", ".p12", ".key", ".pem"):
            raise ValueError(f"package contains private signing material: {path.relative_to(root)}")
        if path.suffix.lower() not in (".crate", ".exe", ".dll", ".dylib", ".so") and not (
            path.is_relative_to(root / "bin") or path.is_relative_to(root / "Manyhands.app/Contents/MacOS")
        ):
            if re.search(rb"(?m)^-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----\r?$", path.read_bytes()):
                raise ValueError(f"package contains a private key: {path.relative_to(root)}")
    for relative in DISCLOSURES:
        require_file(root, relative)
    manifest = read_release(root / "release.json")
    for key in IDENTITY_KEYS:
        if manifest[key] != release[key]:
            raise ValueError(f"package {key} does not match the release input")
    if manifest.get("target") != target:
        raise ValueError("package target does not match the release target")
    if not isinstance(manifest.get("compiler"), str) or not manifest["compiler"].startswith("rustc "):
        raise ValueError("package is missing its compiler version")
    requirements = manifest.get("runtime_requirements")
    if not isinstance(requirements, list) or not requirements or not all(
        isinstance(requirement, str) and requirement for requirement in requirements
    ):
        raise ValueError("package is missing its runtime requirements")
    expected_modes = {platform.signing_mode} if final else {platform.signing_mode, "pending"}
    if manifest.get("signing_mode") not in expected_modes:
        raise ValueError(f"package signing is incomplete or wrong for {target}")
    inventory = json.loads((root / "licenses/inventory.json").read_text(encoding="utf-8"))
    if not isinstance(inventory, dict) or type(inventory.get("schema_version")) is not int or (
        inventory["schema_version"] != 1 or inventory.get("target") != target
    ):
        raise ValueError("licence inventory schema/target does not match the package")
    if not isinstance(inventory.get("packages"), list):
        raise ValueError("licence inventory packages must be a list")
    for package in inventory["packages"]:
        if not isinstance(package, dict) or not all(
            isinstance(package.get(key), str) and package[key] for key in ("name", "version")
        ) or not isinstance(package.get("notices"), list):
            raise ValueError("invalid package in licence inventory")
        for notice in package["notices"]:
            require_file(root / "licenses", notice)
        if "source" in package:
            source = require_file(root / "licenses", package["source"])
            if hashlib.sha256(source.read_bytes()).hexdigest() != package.get("source_sha256"):
                raise ValueError(f"packaged source checksum mismatch: {package['name']}")
    binaries = [f"bin/manyhands-cli{platform.suffix}"]
    if platform.system == "macos":
        binaries.append("Manyhands.app/Contents/MacOS/manyhands")
        resources = root / "Manyhands.app/Contents/Resources"
        for relative in DISCLOSURES:
            copied = require_file(resources, relative)
            if copied.read_bytes() != (root / relative).read_bytes():
                raise ValueError(f"app bundle disclosure differs from package: {relative}")
        if final:
            require_file(root, "Manyhands.app/Contents/Info.plist")
    else:
        binaries.append(f"bin/manyhands{platform.suffix}")
    for binary in binaries:
        path = require_file(root, binary)
        if platform.system != "windows" and not path.stat().st_mode & stat.S_IXUSR:
            raise ValueError(f"package binary is not executable: {binary}")
    if final and platform.system == "windows":
        require_file(root, "signing/manyhands.cer")
        require_file(root, "signing/README.md")
    return manifest


def extraction_path(name: str, root_name: str, target: str) -> PurePosixPath | None:
    path = safe_path(name.rstrip("/"))
    if path.parts[0] == "__MACOSX" and target_info(target).system == "macos":
        if len(path.parts) == 1 or path.parts[1] == root_name or (
            len(path.parts) == 2 and path.parts[1] == f"._{root_name}"
        ):
            return None  # ditto's AppleDouble resource metadata, never executable content.
    if path.parts[0] != root_name:
        raise ValueError(f"archive path is outside expected root {root_name}: {name}")
    return path


def extract_archive(archive: Path, destination: Path, root_name: str, target: str) -> Path:
    if destination.exists():
        raise ValueError(f"archive extraction output already exists: {destination}")
    platform = target_info(target)
    seen = set()
    if platform.system == "linux":
        with tarfile.open(archive, "r:gz") as packed:
            entries = []
            for member in packed.getmembers():
                relative = extraction_path(member.name, root_name, target)
                if not member.isfile() and not member.isdir():
                    raise ValueError(f"archive contains a link or special file: {member.name}")
                if relative in seen:
                    raise ValueError(f"duplicate archive path: {member.name}")
                seen.add(relative)
                entries.append((member, relative))
            destination.mkdir(parents=True)
            for member, relative in entries:
                path = destination.joinpath(*relative.parts)
                if member.isdir():
                    path.mkdir(parents=True, exist_ok=True)
                    continue
                path.parent.mkdir(parents=True, exist_ok=True)
                with packed.extractfile(member) as source, path.open("xb") as output:
                    shutil.copyfileobj(source, output)
                path.chmod(0o755 if member.mode & 0o111 else 0o644)
    else:
        with zipfile.ZipFile(archive) as packed:
            entries = []
            for member in packed.infolist():
                relative = extraction_path(member.filename, root_name, target)
                mode = member.external_attr >> 16
                if stat.S_ISLNK(mode):
                    raise ValueError(f"archive contains a symbolic link: {member.filename}")
                if relative is None:
                    continue
                if relative in seen:
                    raise ValueError(f"duplicate archive path: {member.filename}")
                seen.add(relative)
                entries.append((member, relative, mode))
            destination.mkdir(parents=True)
            for member, relative, mode in entries:
                path = destination.joinpath(*relative.parts)
                if member.is_dir():
                    path.mkdir(parents=True, exist_ok=True)
                    continue
                path.parent.mkdir(parents=True, exist_ok=True)
                with packed.open(member) as source, path.open("xb") as output:
                    shutil.copyfileobj(source, output)
                path.chmod(0o755 if mode & 0o111 else 0o644)
    return destination / root_name


def smoke_environment(home: Path, target: str) -> dict:
    env = os.environ.copy()
    for key in tuple(env):
        if key.startswith(("LD_", "DYLD_")):
            env.pop(key)
    if target_info(target).system == "windows":
        system = env.get("SystemRoot", r"C:\Windows")
        # Do not satisfy app dependencies from Visual Studio, SDK, Cargo,
        # Git-for-Windows, or any other developer-only PATH directory.
        for key in tuple(env):
            if key.upper() == "PATH":
                env.pop(key)
        env["PATH"] = f"{PureWindowsPath(system) / 'System32'};{system}"
    env.update({
        "HOME": str(home), "USERPROFILE": str(home),
        "XDG_CONFIG_HOME": str(home / "config"), "XDG_DATA_HOME": str(home / "data"),
        "XDG_CACHE_HOME": str(home / "cache"), "APPDATA": str(home / "AppData/Roaming"),
        "LOCALAPPDATA": str(home / "AppData/Local"),
    })
    return env


def smoke_packaged_cli(root: Path, target: str) -> None:
    binary = (root / "bin" / f"manyhands-cli{target_info(target).suffix}").resolve()
    with tempfile.TemporaryDirectory(prefix="manyhands-smoke-") as temp:
        home = Path(temp) / "home"
        home.mkdir()
        env = smoke_environment(home, target)
        try:
            result = subprocess.run(
                [str(binary), "--help"], cwd=temp, env=env, capture_output=True,
                text=True, timeout=60, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ValueError(f"packaged CLI smoke test failed: {error}") from error
        if result.returncode:
            raise ValueError(f"packaged CLI smoke test failed ({result.returncode}): {result.stderr.strip()}")


def verify_archive(archive: Path, release: dict, target: str, *, smoke_cli: bool = False,
                   extract_to: Path | None = None) -> dict:
    name = payload_name(release, target)
    if archive.name != f"{name}{target_info(target).extension}":
        raise ValueError("archive filename does not match the release identity/target")

    def verify(destination):
        root = extract_archive(archive, destination, name, target)
        manifest = validate_payload(root, release, target)
        if smoke_cli:
            smoke_packaged_cli(root, target)
        return manifest

    if extract_to is not None:
        return verify(extract_to)
    with tempfile.TemporaryDirectory(prefix="manyhands-verify-") as temp:
        return verify(Path(temp) / "extracted")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--smoke-cli", action="store_true")
    parser.add_argument("--extract-to", type=Path, help="new directory to retain for native signature checks")
    parser.add_argument("--inventory", type=Path, help="write the verified archive identity/checksum for job handoff")
    args = parser.parse_args()
    try:
        manifest = verify_archive(
            args.archive, read_release(args.input), args.target,
            smoke_cli=args.smoke_cli, extract_to=args.extract_to,
        )
        if args.inventory:
            write_json(args.inventory, archive_inventory(args.archive, manifest))
        print(f"Verified {args.archive.name}: {manifest['commit']}")
    except (OSError, ValueError, KeyError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
