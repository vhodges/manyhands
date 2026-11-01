"""Generate complete target-specific disclosures for both release binaries."""

import argparse
from datetime import date
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
OWNER_APPROVED_DECLARATION = "owner-approved-release-declaration"
# A double-curly template also contains an inner single-curly token. Keep
# recognizing only these placeholder labels, not arbitrary brace-wrapped names.
ATTRIBUTION_PLACEHOLDERS = re.compile(
    r"[<\[{]\s*(?:years?|yyyy|copyright\s+holders?|fullname|owner|name\s+of\s+(?:the\s+)?copyright\s+owner)\s*[>\]}]", re.I,
)
COPYRIGHT_LINE = re.compile(r"^[ \t]*(?://|/\*|\*)?[ \t]*copyright[ \t]+(?:\(c\)|©)?[ \t]*(.*)$", re.I | re.M)
# These delimit complete standard grants in cargo-about's selected licence
# text. Unknown/nonstandard variants fail closed rather than using filenames.
GRANT_BOUNDARIES = {
    "MIT": ("permission is hereby granted", "use or other dealings in the software."),
    "ISC": ("permission to use, copy, modify", "use or performance of this software."),
    "BSD-2-Clause": ("redistribution and use in source and binary forms", "even if advised of the possibility of such damage."),
    "BSD-3-Clause": ("redistribution and use in source and binary forms", "even if advised of the possibility of such damage."),
    "Zlib": ("this software is provided", "this notice may not be removed or altered from any source distribution."),
    "NCSA": ("permission is hereby granted", "even if advised of the possibility of such damage."),
}


def normalized_notice(text: str) -> str:
    # BSD lists use either bullets or numbers. Only layout/case are ignored;
    # every word of the selected complete grant must still be present.
    text = re.sub(r"(?m)^\s*(?:[1-9]\.|\*)\s+", "", text)
    return " ".join(text.casefold().split())


def selected_grant(license_info: dict) -> tuple[str, str] | None:
    """Identify a complete grant body supplied by the pinned notice tool."""
    boundaries = GRANT_BOUNDARIES.get(license_info["id"])
    if boundaries is None:
        return None
    start, end = boundaries
    reference = normalized_notice(license_info.get("text", ""))
    first = reference.find(start)
    last = reference.find(end, first)
    if first < 0 or last < 0:
        return None
    return start, reference[first:last + len(end)]


def grant_prefix(text: str, grants: set) -> str | None:
    """Return the text before a complete selected grant, never a name hint."""
    if ATTRIBUTION_PLACEHOLDERS.search(text):
        return None
    for start, body in sorted(grants):
        pattern = r"\s+".join(re.escape(word) for word in start.split())
        for match in re.finditer(pattern, text, re.I):
            candidate = normalized_notice(text[match.start():])
            # Microsoft's published MIT originals omit only the final full
            # stop. Permit that exact complete-body variant at end of text,
            # not truncated words, joined suffixes, or missing grant clauses.
            if candidate.startswith(body) or (body.endswith(".") and candidate == body[:-1]):
                return text[:match.start()]
    return None


def named_attribution(text: str) -> bool:
    """Recognize an original named header, not a bare year or a template."""
    if ATTRIBUTION_PLACEHOLDERS.search(text):
        return False
    for match in COPYRIGHT_LINE.finditer(text):
        holder = re.sub(r"^[0-9\s,./\-–—]+", "", match[1]).strip()
        if holder and holder.casefold() not in ("all rights reserved.", "*/"):
            return True
    return False


def grant_header(prefix: str) -> str:
    # Take only the adjacent copyright block, not copyright mentions in a
    # different licence earlier in a compound root document.
    lines = prefix.rstrip().splitlines()
    header = []
    for line in reversed(lines):
        if COPYRIGHT_LINE.fullmatch(line) or line.strip().casefold() == "all rights reserved.":
            header.append(line)
        else:
            break
    return "\n".join(reversed(header))


def scoped_evidence(original: bytes, supplement: dict) -> bytes:
    """Validate an explicit raw-byte line scope without trimming the original."""
    if "scope" not in supplement:
        return original
    binding = supplement.get("binding")
    resolution = supplement.get("resolution", {})
    if (not isinstance(binding, dict)
        or binding.get("component") != f"{supplement['name']}@{supplement['version']}"
        or not isinstance(binding.get("license"), str) or not binding["license"].strip()
        or (resolution and resolution.get("license") != binding["license"])):
        raise ValueError("supplemental scope needs a matching licence/component binding")
    scope = supplement["scope"]
    if (not isinstance(scope, dict) or set(scope) != {"start_line", "end_line", "sha256"}
        or type(scope.get("start_line")) is not int or type(scope.get("end_line")) is not int
        or not isinstance(scope.get("sha256"), str) or not re.fullmatch(r"[a-f0-9]{64}", scope["sha256"])):
        raise ValueError("invalid supplemental evidence scope")
    lines = original.splitlines(keepends=True)
    if not 1 <= scope["start_line"] <= scope["end_line"] <= len(lines):
        raise ValueError("supplemental evidence scope is outside the original line bounds")
    selected = b"".join(lines[scope["start_line"] - 1:scope["end_line"]])
    if hashlib.sha256(selected).hexdigest() != scope["sha256"]:
        raise ValueError(f"supplemental scope checksum mismatch: {supplement['name']} {supplement['version']}")
    return selected


def approved_release_declaration(original: bytes, supplement: dict) -> bool:
    """Validate a dated owner decision for one exact release manifest, not a policy exemption."""
    resolution = supplement.get("resolution") or {}
    if resolution.get("kind") != OWNER_APPROVED_DECLARATION:
        return False
    binding = supplement.get("binding", {})
    component = f"{supplement['name']}@{supplement['version']}"
    if (binding.get("component") != component or binding.get("license") != resolution.get("license")
        or resolution.get("license") != "MIT" or binding.get("role") != "grant-by-reference"
        or not isinstance(binding.get("reason"), str) or not binding["reason"].strip()):
        raise ValueError("owner-approved release declaration needs a matching MIT grant-by-reference binding")
    decision = resolution.get("owner_decision")
    if (not isinstance(decision, dict) or decision.get("decision") != "Accept scoped declarations"
        or not isinstance(decision.get("provenance"), str) or not decision["provenance"].strip()
        or not isinstance(decision.get("date"), str) or not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", decision["date"])):
        raise ValueError("owner-approved release declaration needs explicit dated owner-decision provenance")
    try:
        date.fromisoformat(decision["date"])
    except ValueError as error:
        raise ValueError("invalid owner-decision date for release declaration") from error
    if supplement.get("source_member") not in ("Cargo.toml", "Cargo.toml.orig") or "file" in supplement:
        raise ValueError("owner-approved release declaration needs the original release manifest proof")
    expected = {key: supplement[key] for key in ("crate_sha256", "source_member", "sha256")}
    expected.update(component=component, license=resolution["license"])
    if "scope" in supplement:
        expected["scope"] = supplement["scope"]
    if decision.get("proof") != expected:
        raise ValueError("owner-decision proof does not match the exact component/licence/version/checksums")
    if hashlib.sha256(original).hexdigest() != supplement["sha256"]:
        raise ValueError("owner-approved release declaration original proof checksum mismatch")
    try:
        manifest = tomllib.loads(original.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise ValueError("invalid original release manifest declaration proof") from error
    package = manifest.get("package")
    if (not isinstance(package, dict) or package.get("name") != supplement["name"]
        or package.get("version") != supplement["version"] or not isinstance(package.get("license"), str)):
        raise ValueError("original release manifest does not match the approved package name/version")
    options = set(re.split(r"\s*(?:/|\bOR\b)\s*", package["license"].strip()))
    if "MIT" not in options or not options <= {"MIT", "Apache-2.0"}:
        raise ValueError("original release manifest does not declare the approved selected MIT licence")
    return True


def gather_disclosures(repo: Path, packages: list, source_ids: set, output: Path,
                       *, source_archives: Path | None = None) -> list:
    """Copy nested notices and original source archives, not just crate SPDX labels."""
    lock = tomllib.loads((repo / "Cargo.lock").read_text(encoding="utf-8"))
    checksums = {
        (p["name"], p["version"], p.get("source")): p.get("checksum")
        for p in lock["package"]
    }
    supplement_manifest = repo / "packaging/licenses/supplemental.json"
    supplements = []
    if supplement_manifest.exists():
        data = json.loads(supplement_manifest.read_text(encoding="utf-8"))
        if data["schema_version"] != 1:
            raise ValueError("unsupported supplemental notice schema")
        supplements = data["notices"]
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
            relative_source = path.relative_to(root)
            named_notice = re.match(r"^(licen[cs]e|copying|notice|copyright|ofl|authors|contributors)([._-]|$)", path.name, re.I)
            licence_directory = any(re.fullmatch(r"\.?licen[cs]es?", part, re.I) for part in relative_source.parts)
            xml_notice = (path.suffix.lower() == ".xml" and not path.is_symlink() and path.is_file()
                          and b"<copyright" in path.read_bytes())
            if not (named_notice or licence_directory or xml_notice):
                continue
            if path.is_symlink():
                raise ValueError(f"licence file is a symbolic link: {name}/{path.name}")
            if not path.is_file():
                continue
            relative = Path("dependencies") / f"{name}-{version}" / relative_source
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            entry["notices"].append(relative.as_posix())
        for supplement in supplements:
            if (supplement["name"], supplement["version"]) != (name, version):
                continue
            if supplement["crate_sha256"] != checksums.get((name, version, package["source"])):
                raise ValueError(f"supplemental notice does not match Cargo.lock: {name} {version}")
            if ("file" in supplement) == ("source_member" in supplement):
                raise ValueError("supplemental notice needs exactly one input path")
            if "source_member" in supplement:
                member = supplement["source_member"]
                parts = member.split("/")
                if any(part in ("", ".", "..") for part in parts) or any(c in member for c in ("\\", ":", "\0")):
                    raise ValueError("invalid supplemental source-member path")
                original = root.joinpath(*parts)
                current = root
                for part in parts:
                    current = current / part
                    if current.is_symlink():
                        raise ValueError("supplemental source-member path is a symbolic link")
                relative = Path("dependencies") / f"{name}-{version}" / "supplemental/source" / Path(*parts)
            else:
                filename = supplement["file"]
                if not re.fullmatch(r"[A-Za-z0-9_+.-]+", filename) or filename in (".", ".."):
                    raise ValueError("invalid supplemental notice filename")
                original = repo / "packaging/licenses/supplemental" / filename
                relative = Path("dependencies") / f"{name}-{version}" / "supplemental" / filename
            if original.is_symlink() or not original.is_file():
                raise ValueError(f"missing original supplemental notice: {name} {version}")
            original_bytes = original.read_bytes()
            digest = hashlib.sha256(original_bytes).hexdigest()
            if digest != supplement["sha256"]:
                raise ValueError(f"supplemental notice checksum mismatch: {name} {version}")
            resolution = supplement.get("resolution")
            if resolution is not None and (
                not isinstance(resolution, dict)
                or resolution.get("kind") not in ("original-grant-no-named-header", "declaration-only", OWNER_APPROVED_DECLARATION)
                or not isinstance(resolution.get("license"), str) or not resolution["license"]
                or not isinstance(resolution.get("reason"), str) or not resolution["reason"].strip()
            ):
                raise ValueError(f"invalid supplemental attribution resolution: {name} {version}")
            if "binding" in supplement:
                binding = supplement["binding"]
                if (not isinstance(binding, dict)
                    or not all(isinstance(binding.get(key), str) and binding[key].strip()
                               for key in ("license", "component"))
                    or binding.get("role") not in ("grant", "grant-by-reference", "attribution", "declaration", "notice")
                    or (binding["role"] == "grant-by-reference" and (
                        not isinstance(binding.get("reason"), str) or not binding["reason"].strip()
                    ))):
                    raise ValueError(f"invalid supplemental licence/component binding: {name} {version}")
            scoped_evidence(original_bytes, supplement)
            approved_release_declaration(original_bytes, supplement)
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, destination)
            entry["notices"].append(relative.as_posix())
            entry.setdefault("supplemental", []).append({**supplement, "path": relative.as_posix()})
        if (name, version) in source_ids:
            if not package["source"].startswith("registry+"):
                raise ValueError(f"original source archive needs a registry package: {name}")
            archive = (source_archives / f"{name}-{version}.crate" if source_archives is not None else
                       root.parents[2] / "cache" / root.parent.name / f"{name}-{version}.crate")
            if not archive.is_file():
                raise ValueError(f"missing original source archive for {name} {version}; supply pinned sources or run cargo fetch --locked")
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


def validate_attributions(data: dict, inventory: list, licenses: Path) -> None:
    """Canonical SPDX placeholders cannot substitute for original attribution."""
    attribution_licenses = {"MIT", "ISC", "BSD-2-Clause", "BSD-3-Clause", "Zlib", "NCSA", "bzip2-1.0.6"}
    # SPDX ids can have several complete text variants. Reuse their grant
    # clauses only: every header/holder must still come from this package's
    # original evidence, never from another package's cargo-about text.
    grants = {}
    for license_info in data["licenses"]:
        grant = selected_grant(license_info)
        if grant is not None:
            grants.setdefault(license_info["id"], set()).add(grant)
    entries = {(entry["name"], entry["version"]): entry for entry in inventory}
    unresolved = set()
    for license_info in data["licenses"]:
        if license_info["id"] not in attribution_licenses or not ATTRIBUTION_PLACEHOLDERS.search(license_info.get("text", "")):
            continue
        for used_by in license_info["used_by"]:
            package = used_by["crate"]
            if package.get("source") is None:
                continue
            entry = entries[package["name"], package["version"]]
            selected = grants.get(license_info["id"], set())
            prefix = Path("dependencies") / f"{package['name']}-{package['version']}"
            has_grant, has_attribution, reviewed_unnamed_grant = False, False, False
            for relative in entry["notices"]:
                # Only an actual root-package grant gets implicit component
                # scope. Nested licences (including licence-document copyrights)
                # need a reviewed, hash-pinned supplemental binding.
                path = Path(relative)
                if path.parent != prefix or not re.fullmatch(r"(?:licen[cs]e|copying)(?:[._-][A-Za-z0-9_.+-]+)?", path.name, re.I):
                    continue
                text = (licenses / relative).read_text(encoding="utf-8", errors="replace")
                before_grant = grant_prefix(text, selected)
                if before_grant is not None:
                    has_grant = True
                    # A copyright after a different licence's grant cannot
                    # provide the missing header of this grant.
                    has_attribution |= named_attribution(grant_header(before_grant))
            for supplement in entry.get("supplemental", []):
                binding = supplement.get("binding", {})
                resolution = supplement.get("resolution", {})
                if (binding.get("license") != license_info["id"]
                    or binding.get("component") != f"{package['name']}@{package['version']}"
                    or resolution.get("kind") == "declaration-only"):
                    continue
                original = (licenses / supplement["path"]).read_bytes()
                text = scoped_evidence(original, supplement).decode("utf-8")
                if not text.strip() or ATTRIBUTION_PLACEHOLDERS.search(text):
                    continue
                if resolution.get("kind") == OWNER_APPROVED_DECLARATION:
                    # The original declaration supplies no named licence notice.
                    # This reviewed acceptance is usable only alongside complete
                    # selected MIT terms, never instead of those canonical terms.
                    if approved_release_declaration(original, supplement) and selected_grant(license_info) is not None:
                        has_grant = True
                        reviewed_unnamed_grant = True
                    continue
                role = binding["role"]
                if role == "attribution":
                    has_attribution |= named_attribution(text)
                    continue
                if role not in ("grant", "grant-by-reference"):
                    continue
                before_grant = grant_prefix(text, selected)
                if role == "grant" and before_grant is None:
                    continue
                has_grant = True
                has_attribution |= named_attribution(text if before_grant is None else grant_header(before_grant))
                reviewed_unnamed_grant |= (
                    resolution.get("kind") == "original-grant-no-named-header"
                    and resolution.get("license") == license_info["id"]
                )
            if not (has_grant and (has_attribution or reviewed_unnamed_grant)):
                unresolved.add(f"{package['name']} {package['version']}")
    if unresolved:
        raise ValueError("unresolved canonical licence attribution: " + ", ".join(sorted(unresolved))
                         + "; supply pinned original notices or resolve the upstream declaration explicitly")


def render_notices(fragments: dict, font_readme: str | None, font_license: str | None) -> str:
    sections = "\n".join(
        f"<h2>{html.escape(name)}</h2>\n{fragment}"
        for name, fragment in fragments.items()
    )
    fonts = (f"<h2>IBM Plex Sans</h2><pre>{html.escape(font_readme)}</pre><pre>{html.escape(font_license)}</pre>"
             if font_readme is not None and font_license is not None else "")
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Manyhands third-party notices</title>
<style>body{{font-family:system-ui,sans-serif;max-width:70rem;margin:2rem auto;padding:0 1rem}}pre{{white-space:pre-wrap;overflow-wrap:anywhere}}</style>
</head><body><h1>Manyhands third-party notices</h1>
<p>Manyhands is MIT-licensed. Dependencies and assets retain their own terms.
The packaged front ends' dependency graphs are listed separately. Duplicate notices are intentional.</p>
<p>Additional original notices are under <code>licenses/dependencies/</code>.
Original source archives for MPL-covered dependencies, libgit2, and separately licensed protocol definitions are under
<code>licenses/sources/</code>; see <code>licenses/README.md</code> for extraction instructions.
Version-specific original source download links are also provided below.</p>
{fonts}
{sections}
</body></html>
"""


def run(args: list, repo: Path) -> str:
    result = subprocess.run(args, cwd=repo, capture_output=True, text=True, encoding="utf-8", check=False)
    if result.returncode:
        raise ValueError(f"{' '.join(args[:3])} failed:\n{result.stderr.strip()}")
    return result.stdout


def about_output(args: list, repo: Path) -> str:
    # cargo-about rejects stdout if PowerShell is anywhere in its ancestor
    # chain, including Python capturing a child pipe. Let the tool write
    # UTF-8 itself, then read the result independently of the invoking shell.
    with tempfile.TemporaryDirectory(prefix="manyhands-about-") as temporary:
        output = Path(temporary) / "notices.txt"
        run([*args, "--output-file", str(output)], repo)
        return output.read_text(encoding="utf-8")


def generate_notices(repo: Path, target: str, output: Path, *, cli_only: bool = False,
                     offline: bool = False, source_archives: Path | None = None) -> None:
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
    fragments, packages, source_ids, license_graphs = {}, {}, set(), []
    graphs = [("CLI", [])]
    if not cli_only:
        graphs.append(("Desktop", ["--features", "desktop"]))
    for graph, features in graphs:
        print(f"release: gathering {graph} notices for {target}", file=sys.stderr, flush=True)
        base = [
            "cargo", "about", "generate", "--frozen" if offline else "--locked", "--fail", "--no-default-features",
            "--target", target, "--config", str(repo / "packaging/licenses/about.toml"),
            *features,
        ]
        data = json.loads(about_output([*base, "--format", "json"], repo))
        license_graphs.append(data)
        for license_info in data["licenses"]:
            for used_by in license_info["used_by"]:
                package = used_by["crate"]
                key = package["name"], package["version"]
                packages[key] = package
                if license_info["id"] == "MPL-2.0" or package["name"] in ("libgit2-sys", "wayland-protocols-plasma"):
                    source_ids.add(key)
        fragments[graph] = about_output([*base, str(repo / "packaging/licenses/about.hbs")], repo)
    # Vendored OpenSSL is built by openssl-src, not part of the normal Rust
    # graph. Preserve its original notices as well when distributing Windows.
    if target == "x86_64-pc-windows-msvc":
        metadata = json.loads(run([
            "cargo", "metadata", "--locked", "--format-version", "1",
            "--no-default-features", *([] if cli_only else ["--features", "desktop"]),
            *(["--offline"] if offline else []), "--filter-platform", target,
        ], repo))
        for package in metadata["packages"]:
            if package["name"] == "openssl-src":
                packages[package["name"], package["version"]] = package
    with tempfile.TemporaryDirectory(prefix="manyhands-notices-") as temp:
        print(f"release: preserving original notices for {len(packages)} packages", file=sys.stderr, flush=True)
        staging = Path(temp) / "notices"
        licenses = staging / "licenses"
        licenses.mkdir(parents=True)
        inventory = gather_disclosures(repo, list(packages.values()), source_ids, licenses, source_archives=source_archives)
        for data in license_graphs:
            validate_attributions(data, inventory, licenses)
        supplemental_html = []
        for entry in inventory:
            for supplement in entry.get("supplemental", []):
                text = (licenses / supplement["path"]).read_text(encoding="utf-8")
                explanation = ""
                if "scope" in supplement:
                    scope = supplement["scope"]
                    binding = supplement["binding"]
                    explanation += (
                        f"<p>Pinned evidence scope: lines {scope['start_line']}-{scope['end_line']}; "
                        f"SHA-256 <code>{html.escape(scope['sha256'])}</code>. "
                        f"Bound to <code>{html.escape(binding['component'])}</code> under "
                        f"<code>{html.escape(binding['license'])}</code>. The complete original follows.</p>"
                    )
                if "resolution" in supplement:
                    explanation += f"<p>Original evidence: {html.escape(supplement['resolution']['reason'])}</p>"
                    if supplement["resolution"]["kind"] == OWNER_APPROVED_DECLARATION:
                        decision = supplement["resolution"]["owner_decision"]
                        explanation += (
                            f"<p>Owner decision: {html.escape(decision['decision'])} ({html.escape(decision['date'])}). "
                            f"Provenance: {html.escape(decision['provenance'])}</p>"
                            "<p>No named licence notice was supplied. Canonical MIT terms and the exact original "
                            "release declaration are preserved; no copyright holder is inferred. This declaration "
                            "decision does not resolve KDE generated-binding or Apple SDK conditions.</p>"
                        )
                for condition in supplement.get("conditions", []):
                    explanation += f"<p>Unresolved condition: {html.escape(str(condition))}</p>"
                supplemental_html.append(
                    f"<h3>{html.escape(entry['name'])} {html.escape(entry['version'])}</h3>"
                    f"<p>Original upstream notice: <a href=\"{html.escape(supplement['url'], quote=True)}\">source</a></p>"
                    f"{explanation}"
                    f"<pre>{html.escape(text)}</pre>"
                )
        if supplemental_html:
            fragments["Supplemental original upstream notices"] = "\n".join(supplemental_html)
        write_json(licenses / "inventory.json", {"schema_version": 1, "target": target, "packages": inventory})
        guide = "README-cli.md" if cli_only else "README.md"
        shutil.copyfile(repo / "packaging/licenses" / guide, licenses / "README.md")
        font_readme, font_license = None, None
        if not cli_only:
            fonts = licenses / "fonts"
            fonts.mkdir()
            for name in ("README.md", "LICENSE.txt"):
                shutil.copyfile(repo / "assets/fonts" / name, fonts / name)
            font_readme = (fonts / "README.md").read_text(encoding="utf-8")
            font_license = (fonts / "LICENSE.txt").read_text(encoding="utf-8")
        document = render_notices(fragments, font_readme, font_license)
        (staging / "THIRD-PARTY-NOTICES.html").write_text(document, encoding="utf-8")
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(staging, output)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cli-only", action="store_true", help="exclude the desktop graph and font disclosures")
    parser.add_argument("--offline", action="store_true", help="use frozen Cargo resolution and no remote licence lookup")
    parser.add_argument("--source-archives", type=Path, help="directory of original .crate files, checked against Cargo.lock")
    args = parser.parse_args()
    try:
        generate_notices(args.repo.resolve(), args.target, args.output.resolve(), cli_only=args.cli_only,
                         offline=args.offline, source_archives=args.source_archives)
    except (OSError, ValueError, KeyError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
