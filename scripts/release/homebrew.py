"""Render a personal-tap cask from a verified Apple Silicon release archive."""

import argparse
from pathlib import Path
import plistlib
import re
from string import Template
import sys
import tempfile
from xml.parsers.expat import ExpatError
import zipfile

from common import hash_file, payload_name, read_release, validate_release
from verify import verify_archive

REPO = Path(__file__).resolve().parents[2]
TARGET = "aarch64-apple-darwin"
MACOS_RELEASES = {12: "monterey", 13: "ventura", 14: "sonoma", 15: "sequoia", 26: "tahoe"}


def render_cask(release: dict, archive: Path, repository: str, *, template: str | None = None) -> str:
    validate_release(release)
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) or any(
        part in (".", "..") for part in repository.split("/")
    ):
        raise ValueError("repository must be OWNER/REPO")
    # Verify the real layout and compare the sealed bundle's deployment target
    # with its manifest. Do not invent requirements from the runner OS version.
    with tempfile.TemporaryDirectory(prefix="manyhands-homebrew-") as temp:
        extracted = Path(temp) / "extracted"
        manifest = verify_archive(archive, release, TARGET, extract_to=extracted)
        root = extracted / payload_name(release, TARGET)
        audit = manifest.get("runtime_audit")
        if not isinstance(audit, dict) or audit.get("system") != "macos" or audit.get("architecture") != "arm64":
            raise ValueError("Homebrew requires completed macOS arm64 runtime metadata")
        minimum = audit.get("minimum_macos")
        # Homebrew's supported depends_on syntax describes named major releases,
        # not point releases. Fail rather than silently round a requirement down.
        match = re.fullmatch(r"([1-9][0-9]*)\.0(?:\.0)?", minimum) if isinstance(minimum, str) else None
        if not match or int(match[1]) not in (11, *MACOS_RELEASES):
            raise ValueError("minimum macOS version needs a reviewed Homebrew release mapping; point releases cannot be rounded down")
        try:
            info = plistlib.loads((root / "Manyhands.app/Contents/Info.plist").read_bytes())
        except (plistlib.InvalidFileException, ExpatError, ValueError) as error:
            raise ValueError("invalid app plist for the minimum macOS check") from error
        if not isinstance(info, dict) or info.get("LSMinimumSystemVersion") != minimum:
            raise ValueError("minimum macOS version differs between the app plist and manifest")
    major = int(match[1])
    requirement = "depends_on :macos" if major == 11 else f"depends_on macos: :{MACOS_RELEASES[major]}"
    if template is None:
        template = (REPO / "packaging/homebrew/manyhands.rb.in").read_text(encoding="utf-8")
    try:
        return Template(template).substitute(
            VERSION=release["version"], SHA256=hash_file(archive), REPOSITORY=repository,
            MACOS_DEPENDENCY=requirement,
        )
    except (KeyError, ValueError) as error:
        raise ValueError(f"invalid or unresolved Homebrew template variable: {error}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--repo", required=True, help="source release repository, not the tap")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        cask = render_cask(read_release(args.input), args.archive, args.repo)
        args.output.write_text(cask, encoding="utf-8")
        print(f"Generated {args.output.name} from {args.archive.name}")
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
