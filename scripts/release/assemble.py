"""Validate four native artifacts and assemble or repair an unpublished draft."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile
from urllib.parse import quote
import zipfile

from common import IDENTITY_KEYS, TARGETS, hash_file, payload_name, read_release, target_info, validate_release, write_json
from verify import verify_archive


class ApiError(RuntimeError):
    pass


def parse_api_response(stdout: str, returncode: int, stderr: str, *, missing_ok=False):
    response = stdout.replace("\r\n", "\n")
    while True:
        headers, separator, body = response.partition("\n\n")
        status = re.match(r"HTTP/\S+ ([0-9]{3})(?:\s|$)", headers)
        if not separator or not status:
            raise ApiError(f"GitHub returned no HTTP response: {stderr.strip() or stdout.strip()}")
        if body.startswith("HTTP/"):
            response = body
            continue
        break
    code = int(status.group(1))
    if code == 404 and missing_ok:
        return None
    if returncode or not 200 <= code < 300:
        raise ApiError(f"GitHub HTTP {code}: {body.strip() or stderr.strip()}")
    try:
        return json.loads(body) if body.strip() else None
    except ValueError as error:
        raise ApiError("GitHub returned invalid JSON") from error


class GitHub:
    def __init__(self, repo: str):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo) or any(
            part in (".", "..") for part in repo.split("/")
        ):
            raise ValueError("repository must be OWNER/REPO")
        self.repo = repo

    def api(self, endpoint: str, *, method="GET", missing_ok=False, accept="application/vnd.github+json"):
        result = subprocess.run(
            ["gh", "api", "--include", "--method", method,
             "--header", f"Accept: {accept}", "--header", "X-GitHub-Api-Version: 2022-11-28",
             f"repos/{self.repo}/{endpoint}"], capture_output=True, text=True, timeout=120, check=False,
        )
        return parse_api_response(result.stdout, result.returncode, result.stderr, missing_ok=missing_ok)

    def pages(self, endpoint):
        page = 1
        while True:
            items = self.api(f"{endpoint}?per_page=100&page={page}")
            if not isinstance(items, list):
                raise ApiError("GitHub returned an invalid list response")
            yield from items
            if len(items) < 100:
                return
            page += 1

    def resolve_tag(self, tag):
        ref = self.api(f"git/ref/tags/{quote(tag, safe='')}")
        item, seen = ref["object"], set()
        while item["type"] == "tag":
            if item["sha"] in seen or len(seen) >= 32:
                raise ApiError("GitHub returned cyclic or excessively nested annotated tags")
            seen.add(item["sha"])
            item = self.api(f"git/tags/{item['sha']}")["object"]
        if item["type"] != "commit":
            raise ValueError("remote release tag does not identify a commit")
        return item["sha"]

    def get_release(self, tag):
        release = self.api(f"releases/tags/{quote(tag, safe='')}", missing_ok=True)
        if release is not None:
            return release
        # Some API versions omit drafts from by-tag lookup. A successful
        # authenticated listing distinguishes absence from a lookup failure.
        return next((item for item in self.pages("releases") if item["tag_name"] == tag), None)

    def assets(self, release):
        return list(self.pages(f"releases/{release['id']}/assets"))

    def read_manifest(self, release):
        assets = [asset for asset in self.assets(release) if asset["name"] == "release.json" and asset["state"] == "uploaded"]
        if len(assets) > 1:
            raise ApiError("draft contains duplicate release manifests")
        return self.api(f"releases/assets/{assets[0]['id']}", accept="application/octet-stream") if assets else None

    def cli(self, *args):
        result = subprocess.run(["gh", "release", *map(str, args), "--repo", self.repo],
                                capture_output=True, text=True, timeout=1800, check=False)
        if result.returncode:
            raise ApiError(f"GitHub release {args[0]} failed: {result.stderr.strip() or result.stdout.strip()}")

    def create(self, release, notes):
        args = ["create", release["tag"], "--draft", "--verify-tag", "--target", release["commit"],
                "--title", f"Manyhands {release['tag']}", "--notes-file", notes]
        if release["prerelease"]:
            args.append("--prerelease")
        self.cli(*args)

    def upload(self, tag, path):
        self.cli("upload", tag, path.resolve(), "--clobber")

    def delete(self, asset):
        self.api(f"releases/assets/{asset['id']}", method="DELETE")


def source_marker(release: dict) -> str:
    return f"<!-- manyhands-release:v1 {release['commit']} -->"


def preflight(release: dict, github, *, expected_id=None):
    validate_release(release)
    if github.resolve_tag(release["tag"]) != release["commit"]:
        raise ValueError("remote tag moved or does not match the prepared source commit")
    existing = github.get_release(release["tag"])
    if existing is None:
        if expected_id is not None:
            raise ValueError("draft disappeared during assembly")
        return None
    if not existing["draft"]:
        raise ValueError("refusing to modify a published release")
    if expected_id is not None and existing["id"] != expected_id:
        raise ValueError("draft was replaced during assembly")
    if type(existing.get("prerelease")) is not bool or existing["prerelease"] != release["prerelease"]:
        raise ValueError("draft prerelease classification does not match its version tag")
    if existing["tag_name"] != release["tag"]:
        raise ValueError("draft tag does not match the release input")
    body = existing.get("body") or ""
    markers = re.findall(r"<!-- manyhands-release:v1 ([0-9a-f]{40}|[0-9a-f]{64}) -->", body)
    if markers:
        if markers != [release["commit"]]:
            raise ValueError("draft belongs to a different source revision")
    else:
        manifest = github.read_manifest(existing)
        if manifest is None:
            raise ValueError("draft has no source marker or existing manifest to prove its revision")
        validate_release(manifest)
        if any(manifest[key] != release[key] for key in IDENTITY_KEYS):
            raise ValueError("draft manifest belongs to a different source revision")
    return existing


def candidates(release: dict, assets: Path) -> list[tuple[Path, dict]]:
    validate_release(release)
    expected = {payload_name(release, target) + platform.extension: target for target, platform in TARGETS.items()}
    required = set(expected) | {name + ".json" for name in expected}
    found = {}
    for path in assets.rglob("*"):
        if path.is_symlink():
            raise ValueError("candidate artifacts contain a symbolic link")
        if not path.is_file():
            continue
        if path.name in found:
            raise ValueError(f"duplicate candidate asset: {path.name}")
        found[path.name] = path
    if set(found) != required:
        raise ValueError(f"candidate asset set is incomplete or unexpected: missing={sorted(required - set(found))}, extra={sorted(set(found) - required)}")
    result = []
    for name, target in sorted(expected.items()):
        path = found[name]
        inventory = read_release(found[name + ".json"])
        if any(inventory[key] != release[key] for key in IDENTITY_KEYS):
            raise ValueError(f"candidate identity/commit mismatch: {name}")
        if inventory.get("target") != target:
            raise ValueError(f"candidate target mismatch: {name}")
        archive = inventory.get("archive", {})
        if not isinstance(archive, dict):
            raise ValueError(f"candidate archive metadata must be an object: {name}")
        if archive.get("name") != name or archive.get("size") != path.stat().st_size:
            raise ValueError(f"candidate archive name/size mismatch: {name}")
        if archive.get("sha256") != hash_file(path):
            raise ValueError(f"candidate archive checksum mismatch: {name}")
        platform = target_info(target)
        audit = inventory.get("runtime_audit")
        if not isinstance(audit, dict):
            raise ValueError(f"candidate runtime metadata must be an object: {name}")
        if inventory.get("signing_mode") != platform.signing_mode or audit.get("system") != platform.system:
            raise ValueError(f"candidate lacks completed platform signing/runtime metadata: {name}")
        manifest = verify_archive(path, release, target)
        if manifest != {key: value for key, value in inventory.items() if key != "archive"}:
            raise ValueError(f"candidate inventory differs from the archived manifest: {name}")
        result.append((path, inventory))
    return result


def source_manifest(release: dict) -> dict:
    # Only source-determined fields belong here. Per-build hashes, compiler
    # details, and native audits live in SHA256SUMS and the archive manifests.
    # The same source revision therefore always has the same identity bytes.
    return {**release, "kind": "manyhands-release-source", "packages": [
        {"target": target, "archive": payload_name(release, target) + platform.extension,
         "signing_mode": platform.signing_mode} for target, platform in sorted(TARGETS.items())
    ]}


def release_notes(release: dict, packages: list) -> str:
    rows = "\n".join(f"| `{item['target']}` | `{item['archive']['name']}` | {item['signing_mode']} |" for item in packages)
    return f"""{source_marker(release)}
# Manyhands {release['tag']}

Source commit: `{release['commit']}`

| Target | Download | Signing |
| --- | --- | --- |
{rows}

Each archive includes the desktop and CLI, an installation README, the MIT
licence, complete third-party/font notices, and relevant original source.
Linux downloads target Ubuntu 24.04-compatible GNU/Linux and require
Wayland/working graphics drivers for the desktop. macOS downloads target
Apple Silicon; the archive README records the actual minimum OS version.
Windows downloads use a statically linked MSVC CRT. Install Git for operations
that invoke Git helpers.

macOS ad-hoc signatures do not provide Developer ID trust or notarization.
Windows self-signing does not provide public publisher trust or remove
SmartScreen warnings. See the packaged README and public certificate details.

`SHA256SUMS` covers the four archives and release-level `release.json`.
The release-level source manifest is immutable across reruns; each archive
contains its own detailed build/runtime manifest. Leave the source asset intact.
Publish this draft manually only after the Release workflow finishes
successfully and the downloads are reviewed. Do not publish while a run is active.
"""


def assemble(release: dict, assets: Path, github) -> str:
    packages = candidates(release, assets)  # Complete verification before any mutation.
    existing = preflight(release, github)
    with tempfile.TemporaryDirectory(prefix="manyhands-assembly-") as temp:
        work = Path(temp)
        aggregate = work / "release.json"
        identity = source_manifest(release)
        write_json(aggregate, identity)
        files = [path for path, _ in packages] + [aggregate]
        checksums = work / "SHA256SUMS"
        checksums.write_text("".join(f"{hash_file(path)}  {path.name}\n" for path in sorted(files, key=lambda p: p.name)), encoding="utf-8")
        if existing is None:
            notes = work / "notes.md"
            notes.write_text(release_notes(release, [item for _, item in packages]), encoding="utf-8")
            # Recheck immediately before creation; a concurrently created
            # same-source draft is reusable without changing its notes.
            existing = preflight(release, github)
            if existing is None:
                github.create(release, notes)
                existing = preflight(release, github)
                if existing is None:
                    raise ApiError("new draft is not visible after creation")
        release_id = existing["id"]
        previous = github.read_manifest(existing)
        if previous is not None and previous != identity:
            raise ValueError("draft source manifest differs from the immutable source identity")
        if previous is None:
            preflight(release, github, expected_id=release_id)
            github.upload(release["tag"], aggregate)
        # Never clobber a valid source anchor. It remains available even if
        # owner-edited notes omit the marker and a later asset upload fails.
        anchors = [asset for asset in github.assets(existing) if asset["name"] == "release.json"]
        if len(anchors) != 1 or anchors[0]["state"] != "uploaded" or anchors[0]["size"] != aggregate.stat().st_size:
            raise ApiError("immutable source manifest upload is incomplete or has changed")
        if anchors[0].get("digest") and anchors[0]["digest"] != f"sha256:{hash_file(aggregate)}":
            raise ApiError("immutable source manifest bytes have changed")
        # An interrupted rerun must not leave an old checksum file suggesting
        # completion. It is the only asset removed separately from clobber.
        for asset in github.assets(existing):
            if asset["name"] == "SHA256SUMS":
                preflight(release, github, expected_id=release_id)
                github.delete(asset)
        uploads = [*files, checksums]
        for path in [*(path for path, _ in packages), checksums]:
            preflight(release, github, expected_id=release_id)
            github.upload(release["tag"], path)
        current = preflight(release, github, expected_id=release_id)
        uploaded = {asset["name"]: asset for asset in github.assets(current)}
        for path in uploads:
            asset = uploaded.get(path.name)
            if not asset or asset["state"] != "uploaded" or asset["size"] != path.stat().st_size:
                raise ApiError(f"draft asset upload is incomplete: {path.name}")
            if asset.get("digest") and asset["digest"] != f"sha256:{hash_file(path)}":
                raise ApiError(f"GitHub asset digest differs from the candidate: {path.name}")
        preflight(release, github, expected_id=release_id)
        return current["html_url"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("preflight", "assemble"))
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--repo", required=True)
    args = parser.parse_args()
    try:
        release = read_release(args.input)
        github = GitHub(args.repo)
        if args.command == "preflight":
            preflight(release, github)
            print(f"Release preflight passed: {release['tag']} at {release['commit']}")
        else:
            if args.assets is None:
                raise ValueError("assembly requires --assets")
            url = assemble(release, args.assets, github)
            print(f"Draft release: {url}")
            if os.environ.get("GITHUB_STEP_SUMMARY"):
                with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a", encoding="utf-8") as summary:
                    summary.write(f"## Draft Release\n\n[{release['tag']}]({url})\n\nSource: `{release['commit']}`\n\n")
                    for target, platform in TARGETS.items():
                        summary.write(f"- `{payload_name(release, target)}{platform.extension}`\n")
                    summary.write("- `release.json`\n- `SHA256SUMS`\n\nReview downloads before publishing manually.\n")
    except (ApiError, OSError, ValueError, KeyError, subprocess.SubprocessError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
