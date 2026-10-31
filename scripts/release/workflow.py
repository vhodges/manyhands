"""Shell-neutral argument handling for the native Release workflow steps."""

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys

from common import target_info

REPO = Path(__file__).resolve().parents[2]
STAGES = ("prepare", "tools", "build-cli", "build-desktop", "install-about", "notices", "stage",
          "linux-audit", "macos-package", "windows-sign", "archive", "verify",
          "macos-verify", "windows-verify", "linux-smoke")


def tool_versions(repo: Path) -> dict:
    versions = {}
    for line in (repo / "packaging/tool-versions.env").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        match = re.fullmatch(r"(RUST_VERSION|CARGO_ABOUT_VERSION)=([0-9]+\.[0-9]+\.[0-9]+)", line)
        if not match or match[1] in versions:
            raise ValueError("invalid or duplicate pinned release tool version")
        versions[match[1]] = match[2]
    if set(versions) != {"RUST_VERSION", "CARGO_ABOUT_VERSION"}:
        raise ValueError("missing pinned release tools")
    return versions


def emit_tools(repo: Path, output: Path) -> None:
    versions = tool_versions(repo)
    with output.open("a", encoding="utf-8") as stream:
        stream.write(f"rust={versions['RUST_VERSION']}\ncargo-about={versions['CARGO_ABOUT_VERSION']}\n")


def required(env: dict, name: str) -> str:
    if not env.get(name):
        raise ValueError(f"missing workflow environment variable: {name}")
    return env[name]


def command_for_stage(stage: str, env: dict, repo: Path) -> list[str]:
    def value(name):
        return required(env, name)

    def python(script, *args):
        return [sys.executable, "-B", str(repo / "scripts/release" / script), *args]

    def target():
        selected = value("TARGET")
        target_info(selected)
        return selected

    if stage == "prepare":
        return python("prepare.py", "--manifest-env", "RELEASE_INPUT", "--output", "release-input.json",
                      "--target", target(), "--github-env", value("GITHUB_ENV"))
    if stage in ("build-cli", "build-desktop"):
        flags = ["--features", "desktop"] if stage == "build-desktop" else []
        binary = "manyhands" if stage == "build-desktop" else "manyhands-cli"
        return ["cargo", "build", "--locked", "--release", "--no-default-features", *flags,
                "--bin", binary, "--target", target()]
    if stage == "install-about":
        return ["cargo", "install", "cargo-about", "--version", tool_versions(repo)["CARGO_ABOUT_VERSION"],
                "--locked", "--features", "cli"]
    if stage == "notices":
        return python("notices.py", "--target", target(), "--output", "release-notices")
    if stage == "stage":
        selected = target()
        return python("package.py", "stage", "--input", "release-input.json", "--target", selected,
                      "--binaries", f"target/{selected}/release", "--notices", "release-notices", "--output", value("STAGING"))
    if stage == "archive":
        return python("package.py", "archive", "--input", "release-input.json", "--target", target(),
                      "--staging", value("STAGING"), "--output", "release-dist")
    if stage == "verify":
        return python("verify.py", "--input", "release-input.json", "--target", target(),
                      "--archive", value("ARCHIVE"), "--extract-to", "release-extracted",
                      "--inventory", value("INVENTORY"), "--smoke-cli")
    if stage == "linux-audit":
        return python("linux_runtime.py", "audit", "--staging", value("STAGING"), "--target", target())
    if stage == "linux-smoke":
        return python("linux_runtime.py", "smoke", "--input", "release-input.json",
                      "--target", target(), "--archive", value("ARCHIVE"))
    if stage in ("macos-package", "macos-verify"):
        mode, variable = ("package", "STAGING") if stage == "macos-package" else ("verify", "EXTRACTED")
        return python("macos.py", mode, "--staging", value(variable))
    if stage in ("windows-sign", "windows-verify"):
        mode, variable = ("Sign", "STAGING") if stage == "windows-sign" else ("Verify", "EXTRACTED")
        return ["pwsh", "-NoLogo", "-NoProfile", "-File", str(repo / "scripts/release/windows-sign.ps1"),
                "-Mode", mode, "-Staging", value(variable)]
    raise ValueError(f"unknown executable workflow stage: {stage}")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=STAGES)
    args = parser.parse_args(argv)
    try:
        if args.stage == "tools":
            emit_tools(REPO, Path(required(os.environ, "GITHUB_OUTPUT")))
        else:
            subprocess.run(command_for_stage(args.stage, os.environ, REPO), cwd=REPO, check=True)
    except subprocess.CalledProcessError as error:
        print(f"release: {args.stage} failed with exit code {error.returncode}", file=sys.stderr)
        return error.returncode
    except (OSError, ValueError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
