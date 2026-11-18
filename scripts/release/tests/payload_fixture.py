"""Small synthetic distributions; native signing is verified separately."""

import json
from pathlib import Path
import sys

LINUX = "x86_64-unknown-linux-gnu"
WINDOWS = "x86_64-pc-windows-msvc"
MACOS = "aarch64-apple-darwin"
REPO = Path(__file__).resolve().parents[3]


class PayloadFixture:
    def __init__(self, root, target=LINUX):
        self.root = Path(root)
        self.target = target
        self.release = {
            "schema_version": 1, "tag": "v0.1.0", "version": "0.1.0",
            "commit": "a" * 40, "prerelease": False,
        }
        self.binaries = self.root / "release binaries"
        self.binaries.mkdir()
        suffix = ".exe" if target == WINDOWS else ""
        for name in ("manyhands", "manyhands-cli"):
            path = self.binaries / f"{name}{suffix}"
            if target == LINUX:
                path.write_text(
                    f"#!{sys.executable}\n"
                    "import os,sys\n"
                    "assert sys.argv[1:] == ['--help']\n"
                    "assert not os.environ.get('LD_LIBRARY_PATH')\n"
                    "assert not os.environ.get('DYLD_LIBRARY_PATH')\n"
                    "print('Manyhands fixture help')\n", encoding="utf-8",
                )
            else:
                path.write_bytes(b"native executable fixture")
            path.chmod(0o755)
        self.notices = self.root / "notice bundle"
        fonts = self.notices / "licenses/fonts"
        fonts.mkdir(parents=True)
        (self.notices / "THIRD-PARTY-NOTICES.html").write_text("<p>Complete fixture terms</p>")
        (fonts / "README.md").write_text("IBM Plex Sans provenance")
        (fonts / "LICENSE.txt").write_text("SIL OFL fixture terms")
        (self.notices / "licenses/README.md").write_text("Dependency licence/source guide")
        native = self.notices / "licenses/dependencies/example/COPYING"
        native.parent.mkdir(parents=True)
        native.write_text("Native fixture terms")
        (self.notices / "licenses/inventory.json").write_text(json.dumps({
            "schema_version": 1, "target": target,
            "packages": [{"name": "example", "version": "1.0.0",
                          "notices": ["dependencies/example/COPYING"]}],
        }))
        self.staging = self.root / "staging directory"
        self.dist = self.root / "distribution directory"

    def stage(self):
        from package import stage_payload
        return stage_payload(
            self.release, self.target, self.binaries, self.notices, self.staging,
            repo=REPO, compiler="rustc 1.98.1 (fixture)",
        )
