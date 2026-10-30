"""Generate complete target-specific disclosures for both release binaries."""

import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

from common import TARGETS, target_info, write_json

REPO = Path(__file__).resolve().parents[2]


def gather_disclosures(repo: Path, packages: list, source_ids: set, output: Path) -> list:
    """Copy nested notices and original source archives, not just crate SPDX labels."""
    lock = tomllib.loads((repo / "Cargo.lock").read_text(encoding="utf-8"))
    checksums = {
        (p["name"], p["version"], p.get("source")): p.get("checksum")
        for p in lock["package"]
    }
    inventory = []
    for package in sorted(packages, key=lambda p: (p["name"], p["version"])):
        if package.get("source") is None:
            continue  # The application's own MIT licence is staged separately.
        name, version = package["name"], package["version"]
        if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or not re.fullmatch(r"[A-Za-z0-9.+-]+", version):
            raise ValueError("invalid dependency name/version in licence metadata")
        root = Path(package["manifest_path"]).parent
        entry = {"name": name, "version": version, "notices": []}
        for path in sorted(root.rglob("*")):
            if not re.match(r"^(licen[cs]e|copying|notice|copyright|ofl)([._-]|$)", path.name, re.I):
                continue
            if path.is_symlink():
                raise ValueError(f"licence file is a symbolic link: {name}/{path.name}")
            if not path.is_file():
                continue
            relative = Path("dependencies") / f"{name}-{version}" / path.relative_to(root)
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            entry["notices"].append(relative.as_posix())
        if (name, version) in source_ids:
            if not package["source"].startswith("registry+"):
                raise ValueError(f"original source archive needs a registry package: {name}")
            archive = root.parents[2] / "cache" / root.parent.name / f"{name}-{version}.crate"
            if not archive.is_file():
                raise ValueError(f"missing original source archive for {name} {version}; run cargo fetch --locked")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            if digest != checksums.get((name, version, package["source"])):
                raise ValueError(f"source archive checksum does not match Cargo.lock: {name} {version}")
            relative = Path("sources") / archive.name
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(archive, destination)
            entry["source"] = relative.as_posix()
            entry["source_sha256"] = digest
        inventory.append(entry)
    return inventory


def render_notices(fragments: dict, font_readme: str, font_license: str) -> str:
    sections = "\n".join(
        f"<h2>{html.escape(name)}</h2>\n{fragment}"
        for name, fragment in fragments.items()
    )
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Manyhands third-party notices</title>
<style>body{{font-family:system-ui,sans-serif;max-width:70rem;margin:2rem auto;padding:0 1rem}}pre{{white-space:pre-wrap;overflow-wrap:anywhere}}</style>
</head><body><h1>Manyhands third-party notices</h1>
<p>Manyhands is MIT-licensed. Dependencies and assets retain their own terms.
The CLI and desktop dependency graphs are listed separately. Duplicate notices are intentional.</p>
<p>Additional original notices are under <code>licenses/dependencies/</code>.
Original source archives for MPL-covered dependencies and libgit2 are under
<code>licenses/sources/</code>; see <code>licenses/README.md</code> for extraction instructions.
Version-specific original source download links are also provided below.</p>
<h2>IBM Plex Sans</h2><pre>{html.escape(font_readme)}</pre><pre>{html.escape(font_license)}</pre>
{sections}
</body></html>
"""


def run(args: list, repo: Path) -> str:
    result = subprocess.run(args, cwd=repo, capture_output=True, text=True, check=False)
    if result.returncode:
        raise ValueError(f"{' '.join(args[:3])} failed:\n{result.stderr.strip()}")
    return result.stdout


def generate_notices(repo: Path, target: str, output: Path) -> None:
    target_info(target)
    if output.exists():
        raise ValueError(f"notices output must be a new directory: {output}")
    versions = dict(
        line.split("=", 1) for line in (repo / "packaging/tool-versions.env").read_text().splitlines()
        if line and not line.startswith("#")
    )
    actual = run(["cargo", "about", "--version"], repo).strip()
    if actual != f"cargo-about {versions['CARGO_ABOUT_VERSION']}":
        raise ValueError(f"wrong notices tool version: {actual}")
    fragments, packages, source_ids = {}, {}, set()
    for graph, features in (("CLI", []), ("Desktop", ["--features", "desktop"])):
        print(f"release: gathering {graph} notices for {target}", file=sys.stderr, flush=True)
        base = [
            "cargo", "about", "generate", "--locked", "--fail", "--no-default-features",
            "--target", target, "--config", str(repo / "packaging/licenses/about.toml"),
            *features,
        ]
        data = json.loads(run([*base, "--format", "json"], repo))
        for license_info in data["licenses"]:
            for used_by in license_info["used_by"]:
                package = used_by["crate"]
                key = package["name"], package["version"]
                packages[key] = package
                if license_info["id"] == "MPL-2.0" or package["name"] == "libgit2-sys":
                    source_ids.add(key)
        fragments[graph] = run([*base, str(repo / "packaging/licenses/about.hbs")], repo)
    # Vendored OpenSSL is built by openssl-src, not part of the normal Rust
    # graph. Preserve its original notices as well when distributing Windows.
    if target == "x86_64-pc-windows-msvc":
        metadata = json.loads(run([
            "cargo", "metadata", "--locked", "--format-version", "1",
            "--no-default-features", "--features", "desktop", "--filter-platform", target,
        ], repo))
        for package in metadata["packages"]:
            if package["name"] == "openssl-src":
                packages[package["name"], package["version"]] = package
    with tempfile.TemporaryDirectory(prefix="manyhands-notices-") as temp:
        print(f"release: preserving original notices for {len(packages)} packages", file=sys.stderr, flush=True)
        staging = Path(temp) / "notices"
        licenses = staging / "licenses"
        licenses.mkdir(parents=True)
        inventory = gather_disclosures(repo, list(packages.values()), source_ids, licenses)
        write_json(licenses / "inventory.json", {"schema_version": 1, "target": target, "packages": inventory})
        shutil.copyfile(repo / "packaging/licenses/README.md", licenses / "README.md")
        fonts = licenses / "fonts"
        fonts.mkdir()
        for name in ("README.md", "LICENSE.txt"):
            shutil.copyfile(repo / "assets/fonts" / name, fonts / name)
        document = render_notices(
            fragments, (fonts / "README.md").read_text(encoding="utf-8"),
            (fonts / "LICENSE.txt").read_text(encoding="utf-8"),
        )
        (staging / "THIRD-PARTY-NOTICES.html").write_text(document, encoding="utf-8")
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(staging, output)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        generate_notices(args.repo.resolve(), args.target, args.output.resolve())
    except (OSError, ValueError, KeyError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
