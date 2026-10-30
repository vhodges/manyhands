"""macOS policy and bundle metadata; real codesign runs on macOS."""

from pathlib import Path
import plistlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from macos import bundle_dependencies, minimum_os, parse_dependencies, plist_document, resolve_dependency


class MacPolicyTests(unittest.TestCase):
    def test_dependency_paths_preserve_spaces(self):
        output = """/some path/manyhands:
    /usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1345.0.0)
    /some path/libssl.3.dylib (compatibility version 3.0.0, current version 3.6.0)
"""
        self.assertEqual(parse_dependencies(output), ["/usr/lib/libSystem.B.dylib", "/some path/libssl.3.dylib"])

    def test_new_and_old_macos_deployment_commands(self):
        self.assertEqual(minimum_os("cmd LC_BUILD_VERSION\nplatform 1\nminos 14.2\nsdk 15.0\n"), (14, 2, 0))
        self.assertEqual(minimum_os("cmd LC_VERSION_MIN_MACOSX\nversion 11.0\nsdk 14.0\n"), (11, 0, 0))

    def test_non_macos_or_missing_deployment_target_is_rejected(self):
        for output in ("cmd LC_BUILD_VERSION\nplatform 2\nminos 14.0\n", "cmd LC_UUID\n"):
            with self.subTest(output=output), self.assertRaisesRegex(ValueError, "macOS"):
                minimum_os(output)

    def test_app_metadata_has_stable_identity_and_numeric_versions(self):
        info = plistlib.loads(plist_document("0.2.0-rc.1", (14, 0, 0)))
        self.assertEqual(info["CFBundleIdentifier"], "io.github.vhodges.manyhands")
        self.assertEqual(info["CFBundleExecutable"], "manyhands")
        self.assertEqual(info["CFBundleShortVersionString"], "0.2.0")
        self.assertEqual(info["LSMinimumSystemVersion"], "14.0.0")

    def test_rpath_resolution_is_explicit(self):
        with tempfile.TemporaryDirectory(prefix="macOS library paths ") as temp:
            root = Path(temp)
            binary = root / "bin/manyhands"
            binary.parent.mkdir()
            library = root / "lib/libssl.3.dylib"
            library.parent.mkdir()
            library.write_bytes(b"dylib fixture")
            self.assertEqual(
                resolve_dependency("@rpath/libssl.3.dylib", binary, binary.parent, ["@loader_path/../lib"]),
                library.resolve(),
            )
            with self.assertRaisesRegex(ValueError, "resolve"):
                resolve_dependency("@rpath/missing.dylib", binary, binary.parent, [])

    def test_openssl_is_copied_in_dependency_order_and_relocated(self):
        with tempfile.TemporaryDirectory(prefix="macOS bundle fixture ") as temp:
            root = Path(temp)
            prefix = root / "brew openssl"
            (prefix / "lib").mkdir(parents=True)
            for name in ("libssl.3.dylib", "libcrypto.3.dylib"):
                (prefix / "lib" / name).write_bytes(name.encode())
            binary = root / "bin/manyhands"
            binary.parent.mkdir()
            binary.write_bytes(b"Mach-O fixture")
            commands = []

            def native(args):
                commands.append(args)
                if args[:2] == ["otool", "-L"]:
                    name = Path(args[-1]).name
                    dependencies = {
                        "manyhands": [str(prefix / "lib/libssl.3.dylib")],
                        "libssl.3.dylib": [str(prefix / "lib/libssl.3.dylib"), str(prefix / "lib/libcrypto.3.dylib")],
                        "libcrypto.3.dylib": [str(prefix / "lib/libcrypto.3.dylib"), "/usr/lib/libSystem.B.dylib"],
                    }[name]
                    return "file:\n" + "".join(f"  {path} (compatibility version 1.0.0, current version 1.0.0)\n" for path in dependencies)
                return ""

            destination = root / "lib"
            copied = bundle_dependencies(binary, destination, "@executable_path/../lib", prefix, run=native)
            self.assertEqual([p.name for p in copied], ["libcrypto.3.dylib", "libssl.3.dylib"])
            self.assertEqual((destination / "libssl.3.dylib").read_bytes(), b"libssl.3.dylib")
            self.assertIn(["install_name_tool", "-change", str(prefix / "lib/libssl.3.dylib"),
                           "@executable_path/../lib/libssl.3.dylib", str(binary)], commands)

    def test_unknown_external_library_requires_a_distribution_policy(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            unknown = root / "unreviewed.dylib"
            unknown.write_bytes(b"library fixture")

            def native(args):
                return f"file:\n  {unknown} (compatibility version 1.0.0, current version 1.0.0)\n"

            with self.assertRaisesRegex(ValueError, "unreviewed"):
                bundle_dependencies(root / "manyhands", root / "dest", "@executable_path/../lib", root / "openssl", run=native)


if __name__ == "__main__":
    unittest.main()
