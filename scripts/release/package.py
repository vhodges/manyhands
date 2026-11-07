"""Stage the common distribution payload and archive a finished native package."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
from string import Template
import zipfile

from common import TARGETS, payload_name, read_release, target_info, validate_release, write_json
from verify import validate_payload

REPO = Path(__file__).resolve().parents[2]


def readme_values(release: dict, target: str) -> dict:
    platform = target_info(target)
    if platform.system == "macos":
        installation = (
            "Move `Manyhands.app` to `/Applications` and launch it from Finder. "
            "Run `bin/manyhands-cli --help` from this directory, or place the CLI "
            "and its accompanying runtime files together in a directory on PATH."
        )
        signing = (
            "The app and CLI are ad-hoc signed. This is not Developer ID signing "
            "or Apple notarization, and does not establish Gatekeeper approval. "
            "macOS may require an explicit user decision before opening a downloaded app."
        )
    elif platform.system == "windows":
        installation = (
            "Extract the ZIP to a directory you control. Launch `bin/manyhands.exe` "
            "for the desktop or run `bin/manyhands-cli.exe --help` in a terminal. "
            "To use the CLI from any directory, add this package's `bin` directory to your user PATH."
        )
        signing = (
            "The executables have self-signed Authenticode signatures. The public "
            "certificate and explanation are under `signing/`. Windows does not "
            "trust this publisher by default; self-signing does not remove "
            "SmartScreen warnings. No certificate is automatically installed on your machine."
        )
    else:
        installation = (
            "Extract the tar archive, preserving executable permissions. Run "
            "`bin/manyhands` from a Wayland desktop session and "
            "`bin/manyhands-cli --help` from a terminal. This dynamically linked "
            "GNU/Linux archive is not a universal static binary. NixOS users need "
            "a Nix-native build rather than running these Ubuntu binaries unchanged."
        )
        signing = "Linux executables are unsigned; verify the downloaded archive's checksum."
    return {
        "VERSION": release["version"], "TAG": release["tag"], "COMMIT": release["commit"],
        "TARGET": target, "INSTALLATION": installation,
        "RUNTIME": "\n".join(f"- {requirement}" for requirement in release.get("runtime_requirements", platform.runtime)),
        "SIGNING": signing,
    }


def refresh_documents(root: Path, manifest: dict, repo: Path = REPO) -> None:
    """Finish metadata and its readable form before native code signing."""
    write_json(root / "release.json", manifest)
    template = Template((repo / "packaging/README-release.md").read_text(encoding="utf-8"))
    (root / "README.md").write_text(template.substitute(readme_values(manifest, manifest["target"])), encoding="utf-8")
    if target_info(manifest["target"]).system == "macos":
        resources = root / "Manyhands.app/Contents/Resources"
        for name in ("README.md", "release.json", "THIRD-PARTY-NOTICES.html", "LICENSE"):
            shutil.copyfile(root / name, resources / name)
        shutil.copytree(root / "licenses", resources / "licenses", dirs_exist_ok=True)


def stage_payload(release: dict, target: str, binaries: Path, notices: Path,
                  output: Path, *, repo: Path = REPO, compiler: str | None = None) -> Path:
    validate_release(release)
    platform = target_info(target)
    if output.exists():
        raise ValueError(f"staging output already exists: {output}")
    for name in ("manyhands", "manyhands-cli"):
        path = binaries / f"{name}{platform.suffix}"
        if not path.is_file() or not path.stat().st_size:
            raise ValueError(f"missing release binary: {path}")
    if compiler is None:
        result = subprocess.run(["rustc", "--version"], capture_output=True, text=True, check=True)
        compiler = result.stdout.strip()
    manifest = {
        **release, "target": target, "compiler": compiler,
        "signing_mode": "none" if platform.system == "linux" else "pending",
        "runtime_requirements": list(platform.runtime),
    }
    with tempfile.TemporaryDirectory(prefix="manyhands-payload-") as temp:
        root = Path(temp) / "payload"
        shutil.copytree(notices, root)
        (root / "bin").mkdir(exist_ok=True)
        shutil.copyfile(repo / "LICENSE", root / "LICENSE")
        template = Template((repo / "packaging/README-release.md").read_text(encoding="utf-8"))
        (root / "README.md").write_text(template.substitute(readme_values(release, target)), encoding="utf-8")
        write_json(root / "release.json", manifest)
        shutil.copy2(binaries / f"manyhands-cli{platform.suffix}", root / "bin" / f"manyhands-cli{platform.suffix}")
        if platform.system == "macos":
            contents = root / "Manyhands.app/Contents"
            (contents / "MacOS").mkdir(parents=True)
            shutil.copy2(binaries / "manyhands", contents / "MacOS/manyhands")
            resources = contents / "Resources"
            resources.mkdir()
            for name in ("README.md", "LICENSE", "THIRD-PARTY-NOTICES.html", "release.json"):
                shutil.copyfile(root / name, resources / name)
            shutil.copytree(root / "licenses", resources / "licenses")
        else:
            shutil.copy2(binaries / f"manyhands{platform.suffix}", root / "bin" / f"manyhands{platform.suffix}")
        validate_payload(root, release, target, final=False)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(root, output)
    return output


def archive_payload(release: dict, target: str, staging: Path, output: Path) -> Path:
    validate_payload(staging, release, target)
    platform = target_info(target)
    if output.resolve().is_relative_to(staging.resolve()):
        raise ValueError("archive output cannot be inside its staging directory")
    output.mkdir(parents=True, exist_ok=True)
    name = payload_name(release, target)
    archive = output / f"{name}{platform.extension}"
    if archive.exists():
        raise ValueError(f"archive output already exists: {archive}")
    try:
        if platform.system == "linux":
            with tarfile.open(archive, "w:gz") as packed:
                packed.add(staging, arcname=name)
        elif platform.system == "windows":
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as packed:
                for path in sorted(staging.rglob("*")):
                    if path.is_file():
                        packed.write(path, (Path(name) / path.relative_to(staging)).as_posix())
        else:
            with tempfile.TemporaryDirectory(prefix="manyhands-macos-zip-") as temp:
                root = Path(temp) / name
                shutil.copytree(staging, root)
                subprocess.run(
                    ["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(root), str(archive.resolve())],
                    check=True,
                )
    except (OSError, subprocess.CalledProcessError, tarfile.TarError, zipfile.BadZipFile):
        archive.unlink(missing_ok=True)
        raise
    return archive


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    stage = commands.add_parser("stage", help="stage documents, manifests, and binaries")
    stage.add_argument("--binaries", type=Path, required=True)
    stage.add_argument("--notices", type=Path, required=True, help="directory generated by notices.py")
    stage.add_argument("--repo", type=Path, default=REPO)
    archive = commands.add_parser("archive", help="archive a completed native payload")
    archive.add_argument("--staging", type=Path, required=True)
    for command in (stage, archive):
        command.add_argument("--input", type=Path, required=True)
        command.add_argument("--target", choices=TARGETS, required=True)
        command.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        release = read_release(args.input)
        if args.command == "stage":
            result = stage_payload(release, args.target, args.binaries, args.notices, args.output, repo=args.repo)
        else:
            result = archive_payload(release, args.target, args.staging, args.output)
        print(result)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
