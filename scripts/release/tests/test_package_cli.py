"""Exercise the actual stage/archive/verify entry points with a small payload."""

import json
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from payload_fixture import LINUX, PayloadFixture

SCRIPTS = Path(__file__).resolve().parents[1]


class PackagingCliTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "linux", "Linux executable fixture")
    def test_stage_archive_and_verify_commands(self):
        with tempfile.TemporaryDirectory(prefix="release CLI commands ") as temp:
            fixture = PayloadFixture(temp)
            release_input = fixture.root / "release input.json"
            release_input.write_text(json.dumps(fixture.release))
            identity = ["--input", str(release_input), "--target", LINUX]
            archive = fixture.dist / f"manyhands-v0.1.0-{LINUX}.tar.gz"
            inventory = fixture.root / "archive inventory.json"
            commands = (
                ["package.py", "stage", *identity, "--binaries", str(fixture.binaries),
                 "--notices", str(fixture.notices), "--output", str(fixture.staging)],
                ["package.py", "archive", *identity, "--staging", str(fixture.staging),
                 "--output", str(fixture.dist)],
                ["verify.py", *identity, "--archive", str(archive), "--smoke-cli", "--inventory", str(inventory)],
            )
            for script, *args in commands:
                with self.subTest(script=script, args=args):
                    result = subprocess.run(
                        [sys.executable, "-B", str(SCRIPTS / script), *args],
                        cwd=temp, capture_output=True, text=True,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(archive.is_file())
            data = json.loads(inventory.read_text())
            self.assertEqual(data["archive"]["sha256"], hashlib.sha256(archive.read_bytes()).hexdigest())
            self.assertEqual(data["target"], LINUX)


if __name__ == "__main__":
    unittest.main()
