"""Release identity must come from a real tag, never the working copy."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

RELEASE_SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(RELEASE_SCRIPTS))

from prepare import materialize_release, parse_release_tag, prepare_release


class TagTests(unittest.TestCase):
    def test_stable_and_prerelease_versions(self):
        for tag, expected in (
            ("v0.1.0", ("0.1.0", False)),
            ("v12.34.56", ("12.34.56", False)),
            ("v0.2.0-rc.1", ("0.2.0-rc.1", True)),
            ("v0.2.0-alpha-beta.0", ("0.2.0-alpha-beta.0", True)),
        ):
            with self.subTest(tag=tag):
                self.assertEqual(parse_release_tag(tag), expected)

    def test_invalid_tags_are_rejected(self):
        for tag in (
            "0.1.0", "main", "refs/tags/v0.1.0", "v01.1.0", "v1.01.0",
            "v1.0.01", "v1.0", "v1.0.0-", "v1.0.0-rc..1", "v1.0.0-01",
            "v1.0.0-rc.01", "v1.0.0+build.1", "v1.0.0 ", "v1.0.0\n",
            "v1.0.0;touch marker", "$(touch marker)", "v1.0.0/other",
        ):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                parse_release_tag(tag)


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release repository ")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git("init", "--quiet")
        self.write_manifests("0.1.0")
        self.commit()
        self.sha = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return subprocess.run(
            ["git", "-c", "user.name=Release Test", "-c",
             "user.email=release-test@example.invalid", "-C", str(self.repo), *args],
            check=True, capture_output=True, text=True,
        ).stdout.strip()

    def write_manifests(self, version, lock_version=None):
        (self.repo / "Cargo.toml").write_text(
            f'[package]\nname = "manyhands"\nversion = "{version}"\n',
            encoding="utf-8",
        )
        (self.repo / "Cargo.lock").write_text(
            f'version = 4\n[[package]]\nname = "manyhands"\n'
            f'version = "{lock_version or version}"\n', encoding="utf-8",
        )

    def commit(self):
        self.git("add", "Cargo.toml", "Cargo.lock")
        self.git("commit", "--quiet", "-m", "fixture")

    def test_lightweight_and_annotated_tags_resolve_to_commits(self):
        for annotated in (False, True):
            with self.subTest(annotated=annotated):
                if annotated:
                    self.git("tag", "-f", "-a", "v0.1.0", "-m", "release")
                else:
                    self.git("tag", "-f", "v0.1.0")
                self.assertEqual(prepare_release(self.repo, "v0.1.0"), {
                    "schema_version": 1, "tag": "v0.1.0", "version": "0.1.0",
                    "commit": self.sha, "prerelease": False,
                })

    def test_branch_named_like_a_version_is_not_a_release_tag(self):
        self.git("branch", "v0.1.0")
        with self.assertRaisesRegex(ValueError, "tag"):
            prepare_release(self.repo, "v0.1.0")

    def test_missing_tag_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "tag"):
            prepare_release(self.repo, "v0.1.0")

    def test_uses_tagged_manifests_not_head_or_working_copy(self):
        self.git("tag", "v0.1.0")
        self.write_manifests("0.2.0")
        self.commit()
        self.write_manifests("0.3.0")
        self.assertEqual(prepare_release(self.repo, "v0.1.0")["commit"], self.sha)

    def test_version_mismatch_is_rejected(self):
        self.git("tag", "v0.2.0")
        with self.assertRaisesRegex(ValueError, "Cargo.toml"):
            prepare_release(self.repo, "v0.2.0")

    def test_lockfile_mismatch_is_rejected(self):
        self.write_manifests("0.1.0", lock_version="0.0.1")
        self.commit()
        self.git("tag", "v0.1.0")
        with self.assertRaisesRegex(ValueError, "Cargo.lock"):
            prepare_release(self.repo, "v0.1.0")

    def test_cli_writes_manifest_and_safe_github_outputs(self):
        self.git("tag", "v0.1.0")
        manifest = self.repo / "release input.json"
        outputs = self.repo / "github output"
        result = subprocess.run(
            [sys.executable, "-B", str(RELEASE_SCRIPTS / "prepare.py"),
             "--repo", str(self.repo), "--tag", "v0.1.0", "--output", str(manifest),
             "--github-output", str(outputs)], capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(manifest.read_text())["commit"], self.sha)
        self.assertIn("prerelease=false\n", outputs.read_text())
        self.assertIn(f"commit={self.sha}\n", outputs.read_text())

    def test_invalid_cli_input_writes_no_manifest(self):
        manifest = self.repo / "release.json"
        result = subprocess.run(
            [sys.executable, "-B", str(RELEASE_SCRIPTS / "prepare.py"),
             "--repo", str(self.repo), "--tag", "v0.1.0;touch marker",
             "--output", str(manifest)], capture_output=True, text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("release:", result.stderr)
        self.assertFalse(manifest.exists())
        self.assertFalse((self.repo / "marker").exists())

    def test_handoff_checks_out_the_prepared_sha_without_resolving_the_tag(self):
        self.git("tag", "v0.1.0")
        release = prepare_release(self.repo, "v0.1.0")
        self.git("tag", "-d", "v0.1.0")
        self.assertEqual(materialize_release(self.repo, release), release)

    def test_handoff_rejects_a_different_checkout(self):
        release = {"schema_version": 1, "tag": "v0.1.0", "version": "0.1.0",
                   "commit": "b" * 40, "prerelease": False}
        with self.assertRaisesRegex(ValueError, "checkout"):
            materialize_release(self.repo, release)

    def test_release_tag_must_be_merged_when_workflow_requires_it(self):
        self.write_manifests("0.2.0")
        self.commit()
        self.git("tag", "v0.2.0")
        self.git("checkout", "--quiet", self.sha)
        with self.assertRaisesRegex(ValueError, "merged"):
            prepare_release(self.repo, "v0.2.0", require_merged=True)

    def test_handoff_cli_emits_target_paths_and_manifest_output(self):
        release = {"schema_version": 1, "tag": "v0.1.0", "version": "0.1.0",
                   "commit": self.sha, "prerelease": False}
        output, environment, github_output = (self.repo / name for name in ("input.json", "environment", "outputs"))
        result = subprocess.run(
            [sys.executable, "-B", str(RELEASE_SCRIPTS / "prepare.py"), "--repo", str(self.repo),
             "--manifest-env", "RELEASE_INPUT", "--output", str(output),
             "--target", "aarch64-apple-darwin", "--github-env", str(environment),
             "--github-output", str(github_output)],
            env={**os.environ, "RELEASE_INPUT": json.dumps(release)}, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("ARCHIVE=release-dist/manyhands-v0.1.0-aarch64-apple-darwin.zip\n", environment.read_text())
        manifest_line = next(line for line in github_output.read_text().splitlines() if line.startswith("manifest="))
        self.assertEqual(json.loads(manifest_line.removeprefix("manifest=")), release)


if __name__ == "__main__":
    unittest.main()
