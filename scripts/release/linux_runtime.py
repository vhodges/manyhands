"""Audit GNU/Linux releases against the Ubuntu 24.04 runtime baseline."""

import argparse
import ctypes
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from common import payload_name, read_release, target_info
from package import refresh_documents
from verify import validate_payload, verify_archive

REPO = Path(__file__).resolve().parents[2]
LINUX_TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu")


def check_elf(target: str, header: str, segments: str, dynamic: str,
              versions: str, linkage: str) -> dict:
    if target not in LINUX_TARGETS:
        raise ValueError("Linux runtime audit requires a supported GNU/Linux target")
    machine = "AArch64" if target.startswith("aarch64") else "Advanced Micro Devices X86-64"
    if not re.search(rf"Machine:\s+{machine}\s*(?:\n|$)", header) or "ELF64" not in header or "little endian" not in header:
        raise ValueError(f"ELF architecture is not {target}")
    expected = "/lib/ld-linux-aarch64.so.1" if target.startswith("aarch64") else "/lib64/ld-linux-x86-64.so.2"
    match = re.search(r"Requesting program interpreter:\s*([^\]]+)\]", segments)
    if not match or match.group(1) != expected:
        raise ValueError(f"ELF interpreter is not the Ubuntu loader {expected}")
    for search in re.findall(r"\((?:RUNPATH|RPATH)\)[^\n]*\[([^\]]*)\]", dynamic):
        for directory in search.split(":"):
            if directory != "$ORIGIN/../lib":
                raise ValueError(f"ELF has an unsupported/private search path: {directory}")
    needed = sorted(set(re.findall(r"\(NEEDED\)[^\n]*\[([^\]]+)\]", dynamic)))
    if not needed or "not found" in linkage or "not a dynamic executable" in linkage:
        raise ValueError("ELF has unresolved runtime dependencies")
    resolved = sorted(set(re.findall(r"(?<!\S)(/[^\s()]+)", linkage)))
    for library in resolved:
        if not library.startswith(("/lib/", "/lib64/", "/usr/lib/", "/usr/lib64/")):
            raise ValueError(f"ELF loads a non-system dependency: {library}")
    if not resolved or any(name not in linkage for name in needed):
        raise ValueError("ELF has unresolved runtime dependencies")
    required = {tuple(map(int, version.split("."))) for version in re.findall(r"GLIBC_([0-9]+\.[0-9]+)", versions)}
    if not required or max(required) > (2, 39):
        raise ValueError("ELF requires glibc newer than the Ubuntu 24.04 baseline (2.39), or has no version information")
    return {"architecture": target, "interpreter": expected, "needed": needed,
            "resolved": resolved, "glibc_required": ".".join(map(str, max(required)))}


def command(args: list[str]) -> str:
    env = {key: value for key, value in os.environ.items() if not key.startswith("LD_")}
    result = subprocess.run(args, env=env, capture_output=True, text=True, timeout=120, check=False)
    if result.returncode:
        raise ValueError(f"{args[0]} failed: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout


def configuration(path: Path) -> list[str]:
    return [line.strip() for line in path.read_text().splitlines() if line.strip() and not line.startswith("#")]


def audit_payload(root: Path, target: str, *, config: Path, update: bool = True,
                  dynamic_load: bool = False) -> dict:
    manifest = read_release(root / "release.json")
    validate_payload(root, manifest, target)
    binaries = {}
    for name in ("manyhands", "manyhands-cli"):
        binary = str((root / "bin" / name).resolve())
        binaries[name] = check_elf(
            target, command(["readelf", "-h", binary]), command(["readelf", "-l", "--wide", binary]),
            command(["readelf", "-d", binary]), command(["readelf", "--version-info", binary]),
            command(["ldd", binary]),
        )
    libraries = configuration(config / "dynamic-libraries.txt")
    if dynamic_load:
        for library in libraries:
            ctypes.CDLL(library)
    report = {"system": "linux", "baseline": "Ubuntu 24.04", "glibc_baseline": "2.39",
              "binaries": binaries, "dynamic_libraries": libraries,
              "ubuntu_packages": configuration(config / "runtime-libraries.txt")}
    if update:
        runtime = root / "runtime"
        runtime.mkdir(exist_ok=True)
        for name in ("runtime-libraries.txt", "dynamic-libraries.txt"):
            shutil.copyfile(config / name, runtime / name)
        manifest["runtime_audit"] = report
        manifest["runtime_requirements"] = [
            *target_info(target).runtime,
            "Install the Ubuntu runtime packages listed in runtime/runtime-libraries.txt; graphics drivers are supplied by the host",
            "glibc 2.39 baseline; NixOS requires a Nix-native build",
        ]
        refresh_documents(root, manifest)
    return report


CONTAINER_SETUP = r"""
set -euo pipefail
packages=()
while IFS= read -r package; do
    [[ -z "$package" || "$package" == \#* ]] || packages+=("$package")
done < /config/runtime-libraries.txt
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends python3 binutils "${packages[@]}"
exec python3 -B /release/scripts/release/linux_runtime.py runtime-check --archive /input/archive.tar.gz --input /input/release.json --target "$1" --config /config
"""


def container_smoke(archive: Path, release_input: Path, target: str) -> None:
    if target not in LINUX_TARGETS:
        raise ValueError("container smoke test requires a Linux target")
    if not archive.is_file():
        raise ValueError(f"missing Linux archive: {archive}")
    release = read_release(release_input)
    if archive.name != f"{payload_name(release, target)}.tar.gz":
        raise ValueError("Linux archive filename does not match its release input")
    architecture = "arm64" if target.startswith("aarch64") else "amd64"
    subprocess.run([
        "docker", "run", "--rm", "--platform", f"linux/{architecture}",
        "-v", f"{REPO / 'scripts/release'}:/release/scripts/release:ro",
        "-v", f"{REPO / 'packaging/linux'}:/config:ro",
        "-v", f"{archive.resolve()}:/input/archive.tar.gz:ro",
        "-v", f"{release_input.resolve()}:/input/release.json:ro",
        "ubuntu:24.04", "bash", "-c", CONTAINER_SETUP, "bash", target,
    ], check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    audit = commands.add_parser("audit")
    audit.add_argument("--staging", type=Path, required=True)
    audit.add_argument("--check-only", action="store_true")
    smoke = commands.add_parser("smoke")
    check = commands.add_parser("runtime-check", help=argparse.SUPPRESS)
    for sub in (audit, smoke, check):
        sub.add_argument("--target", choices=LINUX_TARGETS, required=True)
        sub.add_argument("--config", type=Path, default=REPO / "packaging/linux")
    for sub in (smoke, check):
        sub.add_argument("--archive", type=Path, required=True)
        sub.add_argument("--input", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "audit":
            print(json.dumps(audit_payload(args.staging, args.target, config=args.config, update=not args.check_only), indent=2))
        elif args.command == "smoke":
            container_smoke(args.archive, args.input, args.target)
        else:
            release = read_release(args.input)
            # The bind-mounted archive has a generic name; preserve the
            # identity filename inside this isolated temporary directory.
            with tempfile.TemporaryDirectory(prefix="manyhands-runtime-") as temp:
                archive = Path(temp) / f"{payload_name(release, args.target)}.tar.gz"
                shutil.copyfile(args.archive, archive)
                extracted = Path(temp) / "extracted"
                verify_archive(archive, release, args.target, smoke_cli=True, extract_to=extracted)
                report = audit_payload(extracted / payload_name(release, args.target), args.target,
                                       config=args.config, update=False, dynamic_load=True)
                print(json.dumps(report, indent=2))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
