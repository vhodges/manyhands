"""Shared release identity and serialization helpers (Python 3.11+)."""

import json
import hashlib
from pathlib import Path
import re
from dataclasses import dataclass


@dataclass(frozen=True)
class Target:
    system: str
    suffix: str
    extension: str
    signing_mode: str
    runtime: tuple[str, ...]


TARGETS = {
    "x86_64-unknown-linux-gnu": Target("linux", "", ".tar.gz", "none", (
        "Ubuntu 24.04 or compatible newer GNU/Linux; Wayland for the desktop",
        "Working Vulkan graphics stack and the documented system runtime libraries",
        "Git available on PATH",
    )),
    "aarch64-unknown-linux-gnu": Target("linux", "", ".tar.gz", "none", (
        "Ubuntu 24.04 or compatible newer GNU/Linux; Wayland for the desktop",
        "Working Vulkan graphics stack and the documented system runtime libraries",
        "Git available on PATH",
    )),
    "x86_64-pc-windows-msvc": Target("windows", ".exe", ".zip", "self-signed-authenticode", (
        "Windows x86_64 and the runtime dependencies documented for this release",
        "Git available on PATH",
    )),
    "aarch64-apple-darwin": Target("macos", "", ".zip", "ad-hoc", (
        "Apple Silicon macOS; minimum version is recorded in the app Info.plist",
        "Git available on PATH",
    )),
}

IDENTITY_KEYS = ("schema_version", "tag", "version", "commit", "prerelease")


def parse_release_tag(tag: str) -> tuple[str, bool]:
    """Accept v-prefixed SemVer without build metadata."""
    number = r"(?:0|[1-9][0-9]*)"
    match = re.fullmatch(
        rf"v({number}\.{number}\.{number})(?:-([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*))?",
        tag,
    )
    if not match:
        raise ValueError("tag must be vMAJOR.MINOR.PATCH, optionally with a prerelease")
    prerelease = match.group(2)
    if prerelease and any(
        part.isdecimal() and len(part) > 1 and part.startswith("0")
        for part in prerelease.split(".")
    ):
        raise ValueError("tag has a leading-zero numeric prerelease identifier")
    return tag[1:], prerelease is not None


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def validate_release(data: dict) -> dict:
    if not isinstance(data, dict) or type(data.get("schema_version")) is not int or data["schema_version"] != 1:
        raise ValueError("unsupported release manifest schema")
    if not isinstance(data.get("tag"), str):
        raise ValueError("release manifest is missing its tag")
    version, prerelease = parse_release_tag(data["tag"])
    if data.get("version") != version:
        raise ValueError("release manifest version does not match its tag")
    if type(data.get("prerelease")) is not bool or data["prerelease"] != prerelease:
        raise ValueError("release manifest prerelease flag does not match its tag")
    if not isinstance(data.get("commit"), str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", data["commit"]):
        raise ValueError("release manifest needs a full commit SHA")
    return data


def read_release(path: Path) -> dict:
    return validate_release(json.loads(path.read_text(encoding="utf-8")))


def target_info(target: str) -> Target:
    if target not in TARGETS:
        raise ValueError(f"unsupported release target: {target}")
    return TARGETS[target]


def payload_name(release: dict, target: str) -> str:
    validate_release(release)
    target_info(target)
    return f"manyhands-{release['tag']}-{target}"


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def archive_inventory(archive: Path, manifest: dict) -> dict:
    validate_release(manifest)
    target = manifest["target"]
    if archive.name != payload_name(manifest, target) + target_info(target).extension:
        raise ValueError("archive filename does not match its inventory identity")
    return {**manifest, "archive": {"name": archive.name, "size": archive.stat().st_size,
                                    "sha256": hash_file(archive)}}
