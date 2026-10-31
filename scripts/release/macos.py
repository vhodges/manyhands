"""Complete, relocate, and ad-hoc sign the Apple Silicon distribution."""

import argparse
import json
from pathlib import Path
import plistlib
import re
import shutil
from string import Template
import subprocess
import sys
import tempfile

from common import read_release, write_json
from package import refresh_documents
from verify import validate_payload

REPO = Path(__file__).resolve().parents[2]
TARGET = "aarch64-apple-darwin"
OPENSSL_LIBRARIES = {"libssl.3.dylib", "libcrypto.3.dylib"}


def command(args, *, include_stderr=False):
    result = subprocess.run(args, capture_output=True, text=True, timeout=120, check=False)
    if result.returncode:
        raise ValueError(f"{args[0]} failed: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout + (result.stderr if include_stderr else "")


def parse_dependencies(output: str) -> list[str]:
    return re.findall(r"^\s+(.+?)\s+\((?:compatibility|current) version [^\n]+", output, re.M)


def rpaths(output: str) -> list[str]:
    return re.findall(r"^\s*path (.+?) \(offset [0-9]+\)", output, re.M)


def minimum_os(output: str) -> tuple[int, int, int]:
    if "LC_BUILD_VERSION" in output:
        platform = re.search(r"^\s*platform\s+(\S+)", output, re.M)
        if not platform or platform.group(1).lower() not in ("1", "macos"):
            raise ValueError("Mach-O does not target macOS")
        version = re.search(r"^\s*minos\s+([0-9.]+)", output, re.M)
    elif "LC_VERSION_MIN_MACOSX" in output:
        version = re.search(r"^\s*version\s+([0-9.]+)", output, re.M)
    else:
        version = None
    if not version or not re.fullmatch(r"[0-9]+\.[0-9]+(?:\.[0-9]+)?", version.group(1)):
        raise ValueError("Mach-O is missing its macOS deployment target")
    parts = tuple(map(int, version.group(1).split(".")))
    return (parts + (0,))[:3]


def version_string(version: tuple[int, int, int]) -> str:
    return ".".join(map(str, version))


def plist_document(version: str, deployment: tuple[int, int, int]) -> bytes:
    template = Template((REPO / "packaging/macos/Info.plist.in").read_text(encoding="utf-8"))
    rendered = template.substitute(VERSION=version.split("-", 1)[0], MINIMUM_OS=version_string(deployment))
    plistlib.loads(rendered.encode())  # Fail before writing a malformed bundle.
    return rendered.encode("utf-8")


def resolve_dependency(name: str, binary: Path, executable_dir: Path, search: list[str]) -> Path:
    def expand(value):
        if value.startswith("@loader_path/"):
            return binary.parent / value.removeprefix("@loader_path/")
        if value.startswith("@executable_path/"):
            return executable_dir / value.removeprefix("@executable_path/")
        if value.startswith("/"):
            return Path(value)
        return None

    candidates = [expand(name)]
    if name.startswith("@rpath/"):
        candidates = [base / name.removeprefix("@rpath/") for value in search if (base := expand(value)) is not None]
    for candidate in candidates:
        if candidate is not None and candidate.is_file():
            return candidate.resolve()
    raise ValueError(f"cannot resolve Mach-O dependency {name} of {binary.name}")


def system_library(name: str) -> bool:
    return name.startswith(("/System/Library/", "/usr/lib/"))


def bundle_dependencies(binary: Path, destination: Path, executable_reference: str,
                        openssl_prefix: Path, *, run=command) -> list[Path]:
    """Copy reviewed OpenSSL libraries and rewrite the complete dependency graph."""
    copied, sources, visiting = [], {}, set()

    def visit(source, output, reference):
        dependencies = parse_dependencies(run(["otool", "-L", str(source)]))
        if source.suffix == ".dylib":
            dependencies = dependencies[1:]  # LC_ID_DYLIB, not a dependency.
        search = rpaths(run(["otool", "-l", str(source)]))
        for old in dependencies:
            if system_library(old):
                continue
            library = resolve_dependency(old, source, binary.parent, search)
            if library.name not in OPENSSL_LIBRARIES or not library.is_relative_to(openssl_prefix.resolve()):
                raise ValueError(f"unreviewed macOS redistribution dependency: {library.name}")
            previous = sources.get(library.name)
            if previous and previous != library:
                raise ValueError(f"conflicting macOS dylib names: {library.name}")
            if library in visiting:
                raise ValueError("cyclic external macOS dylib dependencies")
            target = destination / library.name
            if previous is None:
                sources[library.name] = library
                visiting.add(library)
                destination.mkdir(parents=True, exist_ok=True)
                if target.exists():
                    raise ValueError(f"macOS dylib output already exists: {target}")
                shutil.copy2(library, target)
                visit(library, target, "@loader_path")
                run(["install_name_tool", "-id", f"@rpath/{library.name}", str(target)])
                visiting.remove(library)
                copied.append(target)
            run(["install_name_tool", "-change", old, f"{reference}/{library.name}", str(output)])
        # All non-system references now use direct relative paths; obsolete
        # build-time LC_RPATH entries must not leak into a distribution.
        for search_path in search:
            run(["install_name_tool", "-delete_rpath", search_path, str(output)])

    visit(binary, binary, executable_reference)
    return copied


def openssl_notices(root: Path, prefix: Path) -> str:
    prefix = prefix.resolve()
    notices = []
    for source in sorted(prefix.rglob("*")):
        if source.is_file() and re.match(r"^(licen[cs]e|copying|notice|authors)([._-]|$)", source.name, re.I):
            relative = Path("runtime/openssl@3") / source.relative_to(prefix)
            target = root / "licenses" / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            notices.append(relative.as_posix())
    if not any("license" in Path(name).name.lower() for name in notices):
        raise ValueError("cannot redistribute OpenSSL without its original licence text")
    version = prefix.name
    guide = root / "licenses/runtime/openssl@3/README.md"
    guide.write_text(
        f"# OpenSSL\n\nHomebrew openssl@3 {version}. Original licence/notices accompany this file.\n"
        "Unmodified upstream library code; install names were relocated for this package.\n"
        "Upstream source and releases: https://openssl-library.org/source/\n", encoding="utf-8",
    )
    notices.append("runtime/openssl@3/README.md")
    inventory_path = root / "licenses/inventory.json"
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
    inventory["packages"].append({"name": "openssl@3", "version": version, "notices": notices})
    write_json(inventory_path, inventory)
    return version


def macho_files(root: Path) -> list[Path]:
    return [*sorted((root / "Manyhands.app/Contents/Frameworks").glob("*.dylib")),
            *sorted((root / "lib").glob("*.dylib")), root / "bin/manyhands-cli",
            root / "Manyhands.app/Contents/MacOS/manyhands"]


def verify_native(root: Path) -> None:
    manifest = read_release(root / "release.json")
    validate_payload(root, manifest, TARGET)
    info = plistlib.loads((root / "Manyhands.app/Contents/Info.plist").read_bytes())
    deployment = manifest.get("runtime_audit", {}).get("minimum_macos")
    if not deployment or info.get("LSMinimumSystemVersion") != deployment:
        raise ValueError("macOS minimum version is inconsistent between bundle and manifest")
    for binary in macho_files(root):
        if command(["lipo", "-archs", str(binary)]).strip() != "arm64":
            raise ValueError(f"Mach-O is not Apple Silicon arm64: {binary.name}")
        if minimum_os(command(["otool", "-l", str(binary)])) > tuple(map(int, deployment.split("."))):
            raise ValueError("app minimum macOS version is lower than its native dependencies")
        dependencies = parse_dependencies(command(["otool", "-L", str(binary)]))
        if binary.suffix == ".dylib":
            dependencies = dependencies[1:]
        executable_dir = root / ("Manyhands.app/Contents/MacOS" if binary.is_relative_to(root / "Manyhands.app") else "bin")
        for name in dependencies:
            if system_library(name):
                continue
            resolved = resolve_dependency(name, binary, executable_dir, rpaths(command(["otool", "-l", str(binary)])))
            if not resolved.is_relative_to(root.resolve()):
                raise ValueError(f"Mach-O still loads a private external dependency: {name}")
        command(["codesign", "--verify", "--strict", "--verbose=2", str(binary)])
        if "Signature=adhoc" not in command(["codesign", "--display", "--verbose=4", str(binary)], include_stderr=True):
            raise ValueError(f"Mach-O does not have the expected ad-hoc signature: {binary.name}")
    command(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(root / "Manyhands.app")])


def package_macos(root: Path) -> None:
    manifest = read_release(root / "release.json")
    validate_payload(root, manifest, TARGET, final=False)
    if manifest["signing_mode"] != "pending":
        raise ValueError("macOS payload has already been finalized")
    app = root / "Manyhands.app/Contents"
    prefix = Path(command(["brew", "--prefix", "openssl@3"]).strip())
    libraries = [
        *bundle_dependencies(root / "bin/manyhands-cli", root / "lib", "@executable_path/../lib", prefix),
        *bundle_dependencies(app / "MacOS/manyhands", app / "Frameworks", "@executable_path/../Frameworks", prefix),
    ]
    if libraries:
        manifest["runtime_native_libraries"] = {"openssl@3": openssl_notices(root, prefix)}
    deployment = (11, 0, 0)
    for binary in macho_files(root):
        if command(["lipo", "-archs", str(binary)]).strip() != "arm64":
            raise ValueError(f"wrong macOS architecture: {binary.name}")
        deployment = max(deployment, minimum_os(command(["otool", "-l", str(binary)])))
    (app / "Info.plist").write_bytes(plist_document(manifest["version"], deployment))
    with tempfile.TemporaryDirectory(prefix="manyhands-icon-") as temp:
        iconset = Path(temp) / "Manyhands.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
                command(["sips", "-z", str(size * scale), str(size * scale),
                         str(REPO / "packaging/icons/manyhands-1024.png"), "--out", str(iconset / name)])
        command(["iconutil", "-c", "icns", str(iconset), "-o", str(app / "Resources/Manyhands.icns")])
    manifest["runtime_audit"] = {"system": "macos", "architecture": "arm64",
                                 "minimum_macos": version_string(deployment),
                                 "bundled_libraries": sorted({path.name for path in libraries})}
    manifest["runtime_requirements"] = [f"macOS {version_string(deployment)} or newer on Apple Silicon", "Git available on PATH"]
    # Finish documents before sealing the bundle. A failed codesign leaves
    # the payload pending; no archive may be assembled from it.
    refresh_documents(root, manifest)
    for library in libraries:
        command(["codesign", "--force", "--sign", "-", str(library)])
    command(["codesign", "--force", "--sign", "-", "--identifier", "io.github.vhodges.manyhands.cli", str(root / "bin/manyhands-cli")])
    command(["codesign", "--force", "--sign", "-", "--identifier", "io.github.vhodges.manyhands", str(app / "MacOS/manyhands")])
    manifest["signing_mode"] = "ad-hoc"
    refresh_documents(root, manifest)
    try:
        command(["codesign", "--force", "--sign", "-", str(root / "Manyhands.app")])
        verify_native(root)
    except (OSError, ValueError, subprocess.SubprocessError):
        manifest["signing_mode"] = "pending"
        refresh_documents(root, manifest)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("package", "verify"))
    parser.add_argument("--staging", type=Path, required=True)
    args = parser.parse_args()
    try:
        if sys.platform != "darwin":
            raise ValueError("native macOS packaging/verification must run on macOS")
        (package_macos if args.command == "package" else verify_native)(args.staging.resolve())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
