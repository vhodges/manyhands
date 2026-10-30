"""Ubuntu runtime policy must reject outputs tied to a build environment."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from linux_runtime import check_elf, container_smoke

TARGET = "x86_64-unknown-linux-gnu"
HEADER = "  Class: ELF64\n  Data: 2's complement, little endian\n  Machine: Advanced Micro Devices X86-64\n"
SEGMENTS = "[Requesting program interpreter: /lib64/ld-linux-x86-64.so.2]"
DYNAMIC = "(NEEDED) Shared library: [libc.so.6]\n"
LINKAGE = "libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x123)\n/lib64/ld-linux-x86-64.so.2 (0x456)"


class LinuxRuntimeTests(unittest.TestCase):
    def check(self, **changes):
        values = dict(target=TARGET, header=HEADER, segments=SEGMENTS, dynamic=DYNAMIC,
                      versions="Name: GLIBC_2.34\nName: GLIBC_2.39", linkage=LINKAGE)
        return check_elf(**(values | changes))

    def test_valid_ubuntu_linkage_reports_required_baseline(self):
        report = self.check()
        self.assertEqual(report["glibc_required"], "2.39")
        self.assertEqual(report["needed"], ["libc.so.6"])

    def test_wrong_architecture_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "architecture"):
            self.check(header=HEADER.replace("Advanced Micro Devices X86-64", "AArch64"))

    def test_arm64_uses_its_own_loader(self):
        report = self.check(
            target="aarch64-unknown-linux-gnu",
            header=HEADER.replace("Advanced Micro Devices X86-64", "AArch64"),
            segments="[Requesting program interpreter: /lib/ld-linux-aarch64.so.1]",
            linkage="libc.so.6 => /lib/aarch64-linux-gnu/libc.so.6 (0x123)",
        )
        self.assertEqual(report["interpreter"], "/lib/ld-linux-aarch64.so.1")

    def test_nix_interpreter_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "interpreter"):
            self.check(segments=SEGMENTS.replace("/lib64/", "/nix/store/private/lib/"))

    def test_private_runtime_paths_are_rejected(self):
        for path in ("/nix/store/private/lib", "/home/runner/work/lib", "/opt/custom/lib", "$ORIGIN/../other"):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "search path"):
                self.check(dynamic=DYNAMIC + f"(RUNPATH) Library runpath: [{path}]\n")

    def test_missing_dependency_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "unresolved"):
            self.check(linkage="libc.so.6 => not found")

    def test_non_system_dependency_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "non-system"):
            self.check(linkage=LINKAGE.replace("/lib/x86_64-linux-gnu/", "/home/runner/lib/"))

    def test_newer_glibc_requirement_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "2.39"):
            self.check(versions="Name: GLIBC_2.40")

    def test_missing_archive_is_rejected_before_docker_can_create_bind_paths(self):
        with tempfile.TemporaryDirectory() as temp, patch(
            "linux_runtime.subprocess.run", side_effect=AssertionError("Docker must not run for missing inputs")
        ):
            with self.assertRaisesRegex(ValueError, "archive"):
                container_smoke(Path(temp) / "missing.tar.gz", Path(temp) / "input.json", TARGET)


if __name__ == "__main__":
    unittest.main()
