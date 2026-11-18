"""Resolve and validate an existing release tag without changing the repository."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

from common import IDENTITY_KEYS, TARGETS, parse_release_tag, payload_name, target_info, validate_release, write_json


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise ValueError(f"Git {args[0]} failed: {result.stderr.strip()}")
    return result.stdout


def prepare_release(repo: Path, tag: str, *, require_merged=False) -> dict:
    version, prerelease = parse_release_tag(tag)
    try:
        commit = git(repo, "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}").strip()
    except ValueError as error:
        raise ValueError(f"release tag {tag} is missing or is not a commit: {error}") from error
    if require_merged:
        merged = subprocess.run(["git", "-C", str(repo), "merge-base", "--is-ancestor", commit, "HEAD"],
                                capture_output=True, text=True, check=False)
        if merged.returncode:
            raise ValueError("release tag must be merged into the workflow's main revision")
    manifest = tomllib.loads(git(repo, "show", f"{commit}:Cargo.toml"))
    package = manifest.get("package", {})
    if not isinstance(package, dict) or (
        package.get("name") != "manyhands" or package.get("version") != version
    ):
        raise ValueError(f"tag {tag} does not match the manyhands version in Cargo.toml")
    lock = tomllib.loads(git(repo, "show", f"{commit}:Cargo.lock"))
    root_packages = [
        p for p in lock.get("package", [])
        if p.get("name") == "manyhands" and "source" not in p
    ]
    if len(root_packages) != 1 or root_packages[0].get("version") != version:
        raise ValueError(f"tag {tag} does not match the manyhands version in Cargo.lock")
    return {
        "schema_version": 1, "tag": tag, "version": version,
        "commit": commit, "prerelease": prerelease,
    }


def materialize_release(repo: Path, data: dict) -> dict:
    validate_release(data)
    if git(repo, "rev-parse", "HEAD").strip() != data["commit"]:
        raise ValueError("checkout does not match the prepared source commit")
    return {key: data[key] for key in IDENTITY_KEYS}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    identity = parser.add_mutually_exclusive_group(required=True)
    identity.add_argument("--tag")
    identity.add_argument("--manifest-env", help="materialize prepared JSON from this environment variable; verify HEAD")
    parser.add_argument("--require-merged", action="store_true", help="tag must be an ancestor of the current main checkout")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--github-env", type=Path)
    args = parser.parse_args()
    try:
        if args.require_merged and args.manifest_env:
            raise ValueError("--require-merged applies to initial --tag preparation only")
        if args.github_env and args.target is None:
            raise ValueError("--github-env requires --target")
        if args.manifest_env:
            if args.manifest_env not in os.environ:
                raise ValueError(f"missing prepared manifest environment variable: {args.manifest_env}")
            release = materialize_release(args.repo, json.loads(os.environ[args.manifest_env]))
        else:
            release = prepare_release(args.repo, args.tag, require_merged=args.require_merged)
        write_json(args.output, release)
        if args.github_output:
            with args.github_output.open("a", encoding="utf-8") as output:
                for key in ("tag", "version", "commit", "prerelease"):
                    value = release[key]
                    if isinstance(value, bool):
                        value = str(value).lower()
                    output.write(f"{key}={value}\n")
                output.write(f"manifest={json.dumps(release, separators=(',', ':'))}\n")
        if args.github_env:
            name = payload_name(release, args.target)
            archive = name + target_info(args.target).extension
            values = {"PAYLOAD_NAME": name, "STAGING": f"release-staging/{name}",
                      "ARCHIVE": f"release-dist/{archive}", "INVENTORY": f"release-dist/{archive}.json",
                      "EXTRACTED": f"release-extracted/{name}"}
            with args.github_env.open("a", encoding="utf-8") as environment:
                for key, value in values.items():
                    environment.write(f"{key}={value}\n")
    except (ValueError, OSError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
