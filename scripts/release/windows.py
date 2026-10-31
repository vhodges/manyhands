"""Record verified Windows signing/runtime metadata, without private material."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys

from common import read_release, write_json
from package import refresh_documents
from verify import smoke_packaged_cli, validate_payload

TARGET = "x86_64-pc-windows-msvc"
REPO = Path(__file__).resolve().parents[2]


def validate_runtime(runtime: dict) -> None:
    if not isinstance(runtime, dict) or (
        runtime.get("system"), runtime.get("architecture"), runtime.get("crt")
    ) != ("windows", "x86_64", "static"):
        raise ValueError("invalid Windows runtime audit: expected x86_64 with static CRT")
    binaries = runtime.get("binaries")
    if not isinstance(binaries, dict) or set(binaries) != {"manyhands.exe", "manyhands-cli.exe"}:
        raise ValueError("Windows runtime audit must cover both executables")
    for dependencies in binaries.values():
        if not isinstance(dependencies, list) or not dependencies or not all(
            isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_-][A-Za-z0-9_.-]*\.[A-Za-z0-9]+", name)
            for name in dependencies
        ):
            raise ValueError("invalid dependency list in Windows runtime audit")


def validate_signer(root: Path, signer: dict) -> dict:
    certificate = (root / "signing/manyhands.cer").read_bytes()
    expected = {
        "thumbprint_sha1": hashlib.sha1(certificate, usedforsecurity=False).hexdigest().upper(),
        "fingerprint_sha256": hashlib.sha256(certificate).hexdigest().upper(),
    }
    if not isinstance(signer, dict) or any(signer.get(key) != value for key, value in expected.items()):
        raise ValueError("public certificate does not match the recorded signer fingerprints")
    for field in ("subject", "expires_utc", "timestamp_url"):
        if not isinstance(signer.get(field), str) or not signer[field]:
            raise ValueError(f"missing signer certificate metadata: {field}")
    return {key: signer[key] for key in (*expected, "subject", "expires_utc", "timestamp_url")}


def finalize_windows(root: Path, runtime: dict, signer: dict) -> None:
    manifest = read_release(root / "release.json")
    validate_payload(root, manifest, TARGET, final=False)
    if manifest["signing_mode"] != "pending":
        raise ValueError("Windows payload has already been finalized")
    validate_runtime(runtime)
    signer = validate_signer(root, signer)
    crt_notice = root / "licenses/runtime/msvc/CRT-NOTICE.md"
    crt_notice.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(REPO / "packaging/windows/CRT-NOTICE.md", crt_notice)
    inventory_path = root / "licenses/inventory.json"
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
    inventory["packages"].append({"name": "Microsoft MSVC CRT", "version": "static runtime",
                                  "notices": ["runtime/msvc/CRT-NOTICE.md"]})
    write_json(inventory_path, inventory)
    (root / "runtime").mkdir(exist_ok=True)
    write_json(root / "runtime/windows-runtime.json", runtime)
    (root / "signing/README.md").write_text(
        "# Windows self-signing\n\nThese executables are signed with a persistent self-signed Authenticode identity.\n"
        "This publisher is not trusted by Windows by default. Self-signing does not remove SmartScreen warnings.\n"
        "No certificate is installed automatically. The accompanying manyhands.cer contains only the public certificate.\n\n"
        f"Subject: {signer['subject']}\n\nSHA-256 certificate fingerprint: {signer['fingerprint_sha256']}\n\n"
        f"SHA-1 store identifier (not the signature digest): {signer['thumbprint_sha1']}\n\n"
        f"Certificate expires: {signer['expires_utc']}\n\n"
        f"RFC 3161 timestamp service: {signer['timestamp_url']}\n\n"
        "File and timestamp digests use SHA-256. Native signature validation is separate from comparing these fingerprints.\n"
        "The release verifier temporarily trusts this public certificate only on its disposable Windows runner, then removes that trust.\n",
        encoding="utf-8",
    )
    manifest["runtime_audit"] = runtime
    manifest["signer"] = signer
    manifest["runtime_requirements"] = [
        "Windows x86_64 with working graphics drivers; native release checks run on Windows Server 2022",
        "Windows system ICU common C runtime (icuuc.dll), provided since Windows 10 version 1703",
        "MSVC CRT is statically linked; no Visual Studio runtime installation is required",
        "Git available on PATH",
    ]
    manifest["signing_mode"] = "self-signed-authenticode"
    refresh_documents(root, manifest)


def verify_windows(root: Path) -> dict:
    manifest = read_release(root / "release.json")
    validate_payload(root, manifest, TARGET)
    validate_runtime(manifest.get("runtime_audit"))
    validate_signer(root, manifest.get("signer"))
    if json.loads((root / "runtime/windows-runtime.json").read_text()) != manifest["runtime_audit"]:
        raise ValueError("Windows runtime report differs from the manifest")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("finalize", "verify", "smoke"))
    parser.add_argument("--staging", type=Path, required=True)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--signer", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "finalize":
            if args.runtime is None or args.signer is None:
                raise ValueError("Windows finalization needs runtime and signer reports")
            finalize_windows(args.staging, json.loads(args.runtime.read_text()), json.loads(args.signer.read_text()))
        else:
            verify_windows(args.staging)
            if args.command == "smoke":
                smoke_packaged_cli(args.staging, TARGET)
    except (OSError, ValueError, KeyError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
