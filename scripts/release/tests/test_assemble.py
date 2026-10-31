"""Draft lifecycle tests use real archives and an in-memory GitHub boundary."""

import copy
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from assemble import ApiError, GitHub, assemble, parse_api_response, preflight, source_marker
from common import TARGETS, archive_inventory, payload_name
from package import archive_payload, refresh_documents
from payload_fixture import MACOS, PayloadFixture


class FakeGitHub:
    def __init__(self, release):
        self.input = release
        self.remote_commit = release["commit"]
        self.release = None
        self.files = {}
        self.uploads = []
        self.creates = 0
        self.fail_on = None
        self.publish_after_upload = False
        self.lookup_error = None

    def resolve_tag(self, tag):
        return self.remote_commit

    def get_release(self, tag):
        if self.lookup_error:
            raise self.lookup_error
        return copy.deepcopy(self.release)

    def read_manifest(self, release):
        data = self.files.get("release.json")
        return json.loads(data) if data else None

    def create(self, release, notes):
        self.creates += 1
        self.release = {"id": 1, "draft": True, "prerelease": release["prerelease"], "tag_name": release["tag"],
                        "body": notes.read_text(), "html_url": "https://github.com/example/repo/releases/tag/v0.1.0"}

    def upload(self, tag, path):
        # gh release upload --clobber deletes the old asset first, even if
        # the replacement upload subsequently fails.
        self.files.pop(path.name, None)
        if path.name == self.fail_on:
            raise ApiError("simulated interrupted upload")
        self.files[path.name] = path.read_bytes()
        self.uploads.append(path.name)
        if self.publish_after_upload:
            self.release["draft"] = False

    def assets(self, release):
        return [{"id": name, "name": name, "state": "uploaded", "size": len(data),
                 "digest": "sha256:" + hashlib.sha256(data).hexdigest()} for name, data in self.files.items()]

    def delete(self, asset):
        self.files.pop(asset["name"])


class AssemblyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release assembly ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.assets = self.root / "downloaded artifacts"
        self.assets.mkdir()
        self.release = None
        self.archives = {}
        self.fixtures = {}
        for target, platform in TARGETS.items():
            source = self.root / target
            source.mkdir()
            fixture = PayloadFixture(source, target)
            self.fixtures[target] = fixture
            fixture.stage()
            manifest = json.loads((fixture.staging / "release.json").read_text())
            manifest["signing_mode"] = platform.signing_mode
            manifest["runtime_audit"] = {"system": platform.system}
            if platform.system == "windows":
                signing = fixture.staging / "signing"
                signing.mkdir()
                (signing / "manyhands.cer").write_bytes(b"public certificate fixture")
                (signing / "README.md").write_text("self-signed fixture")
            if target == MACOS:
                (fixture.staging / "Manyhands.app/Contents/Info.plist").write_text("<plist>fixture</plist>")
            refresh_documents(fixture.staging, manifest)
            artifact = self.assets / f"manyhands-release-{target}"
            artifact.mkdir()
            if target == MACOS:
                # Structural archive fixture, not a native signature test.
                archive = artifact / f"{payload_name(fixture.release, target)}.zip"
                with zipfile.ZipFile(archive, "w") as packed:
                    for path in fixture.staging.rglob("*"):
                        if path.is_file():
                            packed.write(path, f"{payload_name(fixture.release, target)}/{path.relative_to(fixture.staging).as_posix()}")
            else:
                archive = archive_payload(fixture.release, target, fixture.staging, artifact)
            inventory = archive_inventory(archive, manifest)
            archive.with_name(archive.name + ".json").write_text(json.dumps(inventory))
            self.archives[target] = archive
            self.release = fixture.release
        self.github = FakeGitHub(self.release)

    def run_assembly(self):
        return assemble(self.release, self.assets, self.github)

    def existing_draft(self):
        self.github.release = {"id": 1, "draft": True, "prerelease": False, "tag_name": self.release["tag"],
                               "body": source_marker(self.release) + "\nOwner-written notes", "html_url": "draft-url"}

    def test_complete_candidate_creates_only_a_draft_with_sorted_checksums(self):
        result = self.run_assembly()
        self.assertTrue(self.github.release["draft"])
        self.assertEqual(self.github.creates, 1)
        self.assertEqual(result, self.github.release["html_url"])
        checksums = self.github.files["SHA256SUMS"].decode().splitlines()
        self.assertEqual(len(checksums), 5)
        self.assertEqual(checksums, sorted(checksums, key=lambda line: line.split("  ")[1]))
        for line in checksums:
            digest, name = line.split("  ")
            self.assertEqual(digest, hashlib.sha256(self.github.files[name]).hexdigest())
        aggregate = json.loads(self.github.files["release.json"])
        self.assertEqual(aggregate["commit"], self.release["commit"])
        self.assertEqual({package["target"] for package in aggregate["packages"]}, set(TARGETS))

    def test_missing_target_prevents_any_mutation(self):
        archive = next(iter(self.archives.values()))
        archive.unlink()
        with self.assertRaisesRegex(ValueError, "candidate|archive|asset"):
            self.run_assembly()
        self.assertEqual(self.github.creates, 0)

    def test_duplicate_candidate_is_rejected(self):
        archive = next(iter(self.archives.values()))
        duplicate = self.assets / "duplicate"
        duplicate.mkdir()
        shutil.copyfile(archive, duplicate / archive.name)
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.run_assembly()

    def test_changed_archive_bytes_are_rejected(self):
        next(iter(self.archives.values())).write_bytes(b"corrupt archive")
        with self.assertRaisesRegex(ValueError, "checksum|size"):
            self.run_assembly()
        self.assertEqual(self.github.creates, 0)

    def test_inventory_cannot_claim_another_revision(self):
        archive = next(iter(self.archives.values()))
        path = archive.with_name(archive.name + ".json")
        data = json.loads(path.read_text())
        data["commit"] = "b" * 40
        path.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "commit|identity"):
            self.run_assembly()

    def test_same_revision_rerun_preserves_notes_and_unrelated_assets(self):
        self.existing_draft()
        notes = self.github.release["body"]
        self.github.files["owner-attachment.txt"] = b"keep this"
        self.run_assembly()
        self.run_assembly()
        self.assertEqual(self.github.release["body"], notes)
        self.assertEqual(self.github.files["owner-attachment.txt"], b"keep this")
        self.assertEqual(self.github.creates, 0)

    def test_edited_notes_without_marker_use_existing_manifest_identity(self):
        self.run_assembly()
        self.github.release["body"] = "Fully rewritten owner notes"
        self.run_assembly()
        self.assertEqual(self.github.release["body"], "Fully rewritten owner notes")

    def test_different_revision_draft_is_rejected(self):
        self.existing_draft()
        self.github.release["body"] = source_marker({**self.release, "commit": "b" * 40})
        with self.assertRaisesRegex(ValueError, "source|revision"):
            self.run_assembly()
        self.assertEqual(self.github.uploads, [])

    def test_published_release_rejected_by_preflight_and_assembly(self):
        self.existing_draft()
        self.github.release["draft"] = False
        for action in (lambda: preflight(self.release, self.github), self.run_assembly):
            with self.assertRaisesRegex(ValueError, "published"):
                action()
        self.assertEqual(self.github.uploads, [])

    def test_moved_tag_is_rejected(self):
        self.github.remote_commit = "b" * 40
        with self.assertRaisesRegex(ValueError, "tag"):
            preflight(self.release, self.github)

    def test_lookup_failure_is_not_treated_as_release_absence(self):
        self.github.lookup_error = ApiError("HTTP 403")
        with self.assertRaises(ApiError):
            self.run_assembly()
        self.assertEqual(self.github.creates, 0)

    def test_interrupted_draft_upload_can_be_repaired(self):
        self.github.fail_on = sorted(path.name for path in self.archives.values())[1]
        with self.assertRaises(ApiError):
            self.run_assembly()
        self.assertTrue(self.github.release["draft"])
        self.assertNotIn("SHA256SUMS", self.github.files)
        self.github.fail_on = None
        self.run_assembly()
        self.assertIn("SHA256SUMS", self.github.files)
        self.assertEqual(self.github.creates, 1)

    def test_publication_mid_run_stops_further_uploads(self):
        self.existing_draft()
        self.github.publish_after_upload = True
        with self.assertRaisesRegex(ValueError, "published"):
            self.run_assembly()
        self.assertEqual(len(self.github.uploads), 1)

    def test_failed_rerun_removes_only_the_stale_checksum_indicator(self):
        self.run_assembly()
        self.github.files["owner-notes.txt"] = b"keep"
        self.github.fail_on = sorted(path.name for path in self.archives.values())[1]
        with self.assertRaises(ApiError):
            self.run_assembly()
        self.assertNotIn("SHA256SUMS", self.github.files)
        self.assertEqual(self.github.files["owner-notes.txt"], b"keep")

    def test_rewritten_notes_cannot_lose_their_immutable_source_anchor(self):
        self.run_assembly()
        original = self.github.files["release.json"]
        self.github.release["body"] = "Fully rewritten owner notes"
        self.github.fail_on = "release.json"
        self.run_assembly()  # No replacement of the existing identity asset.
        self.assertEqual(self.github.files["release.json"], original)
        self.assertEqual(self.github.uploads.count("release.json"), 1)

    def test_rebuild_updates_checksums_without_replacing_source_identity(self):
        self.run_assembly()
        identity = self.github.files["release.json"]
        self.github.release["body"] = "Owner notes without a marker"
        target = "x86_64-unknown-linux-gnu"
        fixture, archive = self.fixtures[target], self.archives[target]
        binary = fixture.staging / "bin/manyhands"
        binary.write_text(binary.read_text() + "# rebuilt binary fixture\n")
        archive.unlink()
        archive_payload(fixture.release, target, fixture.staging, archive.parent)
        manifest = json.loads((fixture.staging / "release.json").read_text())
        archive.with_name(archive.name + ".json").write_text(json.dumps(archive_inventory(archive, manifest)))
        self.github.fail_on = "release.json"
        self.run_assembly()
        self.assertEqual(self.github.files["release.json"], identity)
        self.assertEqual(self.github.files[archive.name], archive.read_bytes())
        self.assertIn(hashlib.sha256(archive.read_bytes()).hexdigest().encode(), self.github.files["SHA256SUMS"])

    def test_wrong_prerelease_classification_is_rejected_without_mutation(self):
        self.existing_draft()
        self.github.release["prerelease"] = True
        with self.assertRaisesRegex(ValueError, "prerelease"):
            self.run_assembly()
        self.assertEqual(self.github.uploads, [])
        preview = {**self.release, "tag": "v0.1.0-rc.1", "version": "0.1.0-rc.1", "prerelease": True}
        self.github.release.update(tag_name=preview["tag"], prerelease=False)
        with self.assertRaisesRegex(ValueError, "prerelease"):
            preflight(preview, self.github)

    def test_malformed_nested_metadata_fails_with_a_validation_error(self):
        archive = next(iter(self.archives.values()))
        path = archive.with_name(archive.name + ".json")
        original = json.loads(path.read_text())
        for field in ("archive", "runtime_audit"):
            for value in (None, []):
                with self.subTest(field=field, value=value):
                    path.write_text(json.dumps({**original, field: value}))
                    with self.assertRaisesRegex(ValueError, "metadata"):
                        self.run_assembly()
        self.assertEqual(self.github.creates, 0)


class ApiResponseTests(unittest.TestCase):
    def test_successful_json_and_empty_delete_responses(self):
        self.assertEqual(parse_api_response('HTTP/2.0 200 OK\n\n{"draft":true}', 0, ""), {"draft": True})
        self.assertIsNone(parse_api_response("HTTP/2.0 204 No Content\n\n", 0, ""))

    def test_only_explicit_404_can_be_treated_as_missing(self):
        self.assertIsNone(parse_api_response('HTTP/2.0 404 Not Found\ncontent-type: application/json\n\n{"message":"Not Found"}', 1, "", missing_ok=True))
        for stdout, stderr in (
            ('HTTP/2.0 403 Forbidden\n\n{"message":"Forbidden"}', ""),
            ("", "network connection failed"),
            ('HTTP/2.0 429 Too Many Requests\n\n{}', ""),
        ):
            with self.subTest(stdout=stdout), self.assertRaises(ApiError):
                parse_api_response(stdout, 1, stderr, missing_ok=True)

    def test_create_command_explicitly_requires_a_draft_and_existing_tag(self):
        release = {"tag": "v0.1.0-rc.1", "commit": "a" * 40, "prerelease": True}
        with patch("assemble.subprocess.run", return_value=SimpleNamespace(returncode=0, stdout="", stderr="")) as run:
            GitHub("example/repo").create(release, Path("notes.md"))
        args = run.call_args.args[0]
        self.assertIn("--draft", args)
        self.assertIn("--verify-tag", args)
        self.assertIn("--prerelease", args)
        self.assertEqual(args[args.index("--target") + 1], release["commit"])

    def test_annotated_remote_tags_are_peeled_to_the_commit(self):
        client = GitHub("example/repo")
        with patch.object(client, "api", side_effect=[
            {"object": {"type": "tag", "sha": "b" * 40}},
            {"object": {"type": "commit", "sha": "a" * 40}},
        ]):
            self.assertEqual(client.resolve_tag("v0.1.0"), "a" * 40)

    def test_draft_lookup_falls_back_to_authenticated_listing(self):
        draft = {"id": 1, "tag_name": "v0.1.0", "draft": True}
        client = GitHub("example/repo")
        with patch.object(client, "api", side_effect=[None, [draft]]):
            self.assertEqual(client.get_release("v0.1.0"), draft)


if __name__ == "__main__":
    unittest.main()
