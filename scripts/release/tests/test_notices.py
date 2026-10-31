"""Disclosures must include nested notices and checksum-verified source."""

import hashlib
import json
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from notices import gather_disclosures, generate_notices, render_notices


class NoticeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release notices ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.output = self.root / "output"
        self.crate = self.root / "registry/src/example-index/option-ext-0.2.0"
        self.crate.mkdir(parents=True)
        self.cache = self.root / "registry/cache/example-index"
        self.cache.mkdir(parents=True)
        self.source = self.cache / "option-ext-0.2.0.crate"
        self.source.write_bytes(b"original published source archive")
        checksum = hashlib.sha256(self.source.read_bytes()).hexdigest()
        (self.repo / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "option-ext"\nversion = "0.2.0"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{checksum}"\n', encoding="utf-8",
        )
        self.package = {
            "name": "option-ext", "version": "0.2.0",
            "source": "registry+https://github.com/rust-lang/crates.io-index",
            "manifest_path": str(self.crate / "Cargo.toml"),
        }

    def test_copies_nested_native_notices_without_flattening(self):
        nested = self.crate / "vendor/library/COPYING"
        nested.parent.mkdir(parents=True)
        nested.write_text("Native library copyright and complete terms", encoding="utf-8")
        gather_disclosures(self.repo, [self.package], set(), self.output)
        copied = self.output / "dependencies/option-ext-0.2.0/vendor/library/COPYING"
        self.assertEqual(copied.read_text(), nested.read_text())

    def test_original_source_is_verified_against_the_lockfile(self):
        gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)
        self.assertEqual(
            (self.output / "sources/option-ext-0.2.0.crate").read_bytes(),
            self.source.read_bytes(),
        )

    def test_corrupt_original_source_fails(self):
        self.source.write_bytes(b"corrupt cached download")
        with self.assertRaisesRegex(ValueError, "checksum"):
            gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)

    def test_missing_original_source_fails(self):
        self.source.unlink()
        with self.assertRaisesRegex(ValueError, "source archive"):
            gather_disclosures(self.repo, [self.package], {("option-ext", "0.2.0")}, self.output)

    def test_font_terms_are_complete_and_html_escaped(self):
        rendered = render_notices(
            {"CLI": "<section>CLI terms</section>", "Desktop": "<section>Desktop terms</section>"},
            "IBM Plex Sans <script>not executable</script>", "SIL OFL\ncomplete terms & copyright",
        )
        self.assertIn("CLI terms", rendered)
        self.assertIn("Desktop terms", rendered)
        self.assertIn("&lt;script&gt;not executable&lt;/script&gt;", rendered)
        self.assertIn("SIL OFL\ncomplete terms &amp; copyright", rendered)
        self.assertNotIn("<script>", rendered)

    def test_generator_reads_utf8_files_instead_of_capturing_about_stdout(self):
        packaging = self.repo / "packaging/licenses"
        packaging.mkdir(parents=True)
        (self.repo / "packaging/tool-versions.env").write_text("CARGO_ABOUT_VERSION=0.9.3\n", encoding="utf-8")
        (packaging / "README.md").write_text("Source and licence guide", encoding="utf-8")
        fonts = self.repo / "assets/fonts"
        fonts.mkdir(parents=True)
        (fonts / "README.md").write_text("IBM Plex Sans — provenance", encoding="utf-8")
        (fonts / "LICENSE.txt").write_text("Copyright © IBM; OFL terms", encoding="utf-8")
        generated = []

        def cargo(args, repo):
            if args == ["cargo", "about", "--version"]:
                return "cargo-about 0.9.3\n"
            if args[:2] == ["cargo", "metadata"]:
                return '{"packages":[]}'
            if "--output-file" not in args:
                raise ValueError("cargo-about rejects stdout when PowerShell is an ancestor")
            destination = Path(args[args.index("--output-file") + 1])
            self.assertNotEqual(str(destination), "-")
            generated.append(args)
            text = json.dumps({"licenses": [{"id": "MPL-2.0", "used_by": [{"crate": self.package}]}]}) if "--format" in args else "<section>Licence © café — complete terms</section>"
            destination.write_bytes(text.encode("utf-8"))
            return "stdout is not the licence document"

        for target in ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"):
            with self.subTest(target=target), patch("notices.run", side_effect=cargo), contextlib.redirect_stderr(io.StringIO()):
                output = self.output / target
                generate_notices(self.repo, target, output)
                html = (output / "THIRD-PARTY-NOTICES.html").read_text(encoding="utf-8")
                self.assertIn("Licence © café — complete terms", html)
                self.assertNotIn("stdout is not", html)
        self.assertEqual(len(generated), 8)  # JSON and HTML for both front ends on both targets.


if __name__ == "__main__":
    unittest.main()
