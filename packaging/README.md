# Release packaging

The manual [Release workflow](../.github/workflows/release.yml) builds the
four native archive targets and assembles an unpublished GitHub Release.
The Build workflow supplies the separate native build/test checks.
Homebrew, a Windows installer, and Nix consumer packages are Stage 2 work.

Release jobs use Bash on Linux/macOS and PowerShell on Windows. Shared
argument construction lives in `scripts/release/workflow.py`, which passes
argument lists directly to Cargo and the existing packaging helpers. Cargo
builds on Windows run natively, preserving the Windows Perl/MSVC environment.
Both native workflows use the pinned ARM64 `macos-15` runner label.

## Tag-first releases

1. Update the package version in `Cargo.toml` and `Cargo.lock` and merge the
   release-ready source, including release tooling.
2. Create and push a matching tag, such as `v0.1.0` or `v0.2.0-rc.1`.
3. Invoke Release **from main**, supplying the existing tag
   as its input. Selecting a tag as the workflow ref is a different action.
4. Inspect the complete draft, verification summary, and checksums before
   publishing manually. Do not publish while a release run is active.

The workflow never creates or moves tags. Numeric SemVer components cannot
have leading zeroes; build metadata (`+...`) is not supported. Prereleases
are marked as such in GitHub. Tags and both Cargo manifests must agree.
The selected commit must already be merged into the dispatch's main
revision. Configure the Windows signing secrets described in
[Windows signing](windows/SIGNING.md) before the first native release run.

A retry can exercise helper fixes only if its source tag contains those
fixes. The failed initial `v0.1.0` tag still identifies the original source;
do not move it. For the next trial, merge the fixes and use a new version/tag
with matching `Cargo.toml` and `Cargo.lock` versions. No version bump or new
tag is made automatically by the workflow.

After merge, with owner approval for the native run:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0
```

Preparation reads manifests at the tagged commit, not from the checkout's
current working files:

```sh
devenv shell -- python3 -B scripts/release/prepare.py --tag v0.1.0 --output /tmp/opencode/release-input.json
```

`release-input.json` is schema version 1 and records the tag, package
version, resolved commit SHA, and prerelease flag. Subsequent jobs build
that SHA, not a branch or a newly resolved tag. They materialize the prepared
JSON and independently verify that their checkout matches its commit.
GitHub release state and the remote tag are checked during preparation and
again before every release mutation.

## Draft assets and reruns

All four native jobs must succeed before assembly. Each produces one
archive and an internal JSON inventory containing its source, build/runtime
metadata, byte size, and SHA-256. Those inventories travel in seven-day
Actions artifacts; assembly checks the complete target set, compares the
inventory with each extracted archive's manifest, and verifies its hash.

The draft has six workflow-owned downloads:

- Two Linux `.tar.gz` archives (x86_64 and ARM64).
- One Apple Silicon macOS ZIP with `Manyhands.app` and the CLI.
- One Windows x86_64 ZIP with both executables.
- `release.json`: immutable source identity and target/archive descriptors.
- `SHA256SUMS`: current checksums for the four archives and `release.json`.

Detailed compiler, signing, and runtime data lives in each archive's own
`release.json`. The release-level source manifest is uploaded first and
retained unchanged on same-source reruns. This lets recovery identify the
draft even when its owner has completely rewritten the release notes.
Do not delete or edit that source asset. The initial notes also contain a
hidden source marker; if both identity records are removed, preflight
cannot safely identify the draft and fails before mutation.

Reruns preserve owner-written notes and unrelated attachments. After all
new candidates are verified, assembly clears any stale `SHA256SUMS`,
replaces only the four owned archives, and uploads fresh checksums last.
An interrupted upload leaves a repairable draft with no completion checksum
file. A source mismatch, moved tag, incorrect prerelease classification,
or published release fails rather than updating that release.

GitHub uploads are not transactional with manual publication. The helper
checks draft state before each mutation and stops when it observes
publication, but cannot atomically lock the release UI. Publish only after
the workflow has completed successfully and the downloads have been
reviewed. The workflow itself never publishes a draft.

## Tool versions and local checks

`tool-versions.env` pins release Rust and cargo-about. Rust 1.98.1's upstream
distribution manifest was checked for all four release targets. Devenv
provides Python for packaging helpers/tests; Python 3.11 or newer is needed.
Run local Rust tooling through Devenv as required by `AGENTS.md`.

```sh
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v
```

Native workflow runs, publication, and repository secrets are owner-managed
operations. Consult the [design](../docs/plans/2026-10-30-release-workflow-design.md)
and [implementation plan](../docs/plans/2026-10-30-release-workflow-implementation.md)
for remaining work and native verification checkpoints.

## Disclosures

See [licence generation](licenses/README.md) for the target-specific notices
bundle and pinned cargo-about installation. The bundle includes complete
licence texts, original nested native/asset notices, IBM Plex Sans
provenance/OFL, and checksum-verified source archives for source-covered
dependencies. It is a directory supplied to the staging helper, not a
single notice file. Native library linkage/distribution is checked in the
platform packaging steps.

## Shared staging and verification

The staging output is a fresh directory containing the two executables,
rendered release README, MIT licence, notice bundle, and `release.json`.
The macOS staging layout also contains `Manyhands.app` with its own copy
of the documentation. Signing/platform metadata are completed by the
native platform steps before archiving; `pending` signing cannot be
archived. Signature verification itself belongs to those native steps.

```sh
devenv shell -- python3 -B scripts/release/package.py stage --input /tmp/opencode/release-input.json --target x86_64-unknown-linux-gnu --binaries target/x86_64-unknown-linux-gnu/release --notices /tmp/opencode/manyhands-notices --output /tmp/opencode/manyhands-staging
devenv shell -- python3 -B scripts/release/package.py archive --input /tmp/opencode/release-input.json --target x86_64-unknown-linux-gnu --staging /tmp/opencode/manyhands-staging --output /tmp/opencode/manyhands-dist
devenv shell -- python3 -B scripts/release/verify.py --input /tmp/opencode/release-input.json --target x86_64-unknown-linux-gnu --archive /tmp/opencode/manyhands-dist/manyhands-v0.1.0-x86_64-unknown-linux-gnu.tar.gz --smoke-cli
```

Use the actual tag/target and binaries built for it. Local Devenv binaries
are Nix-linked development outputs, not the Ubuntu distribution binaries.
The verifier checks the extracted payload, notices and source checksums,
binary permissions, and release identity. It rejects archive traversal,
links, and private signing-file payloads. CLI smoke tests use an isolated
home/current directory and remove inherited loader-path overrides.
`--extract-to <new-directory>` retains extraction for native signature
checks. Native runtime/signing helpers are described below. Native-runner
acceptance remains pending; structural packaging tests alone do not
establish native runtime compatibility or a valid code signature.

## Linux runtime audit

After staging native Ubuntu binaries, before creating the archive:

```sh
bash scripts/release/linux-runtime.sh audit --staging "$STAGING" --target "$TARGET"
```

The audit checks ELF class/architecture, the native GNU loader, runtime
search paths, glibc requirements (Ubuntu 24.04: 2.39), and resolved direct
and transitive linkage. It rejects Nix/build-private runtime paths and
unresolved/non-system libraries. This initial distribution relies on
explicit Ubuntu runtime packages rather than copying glibc or drivers.
`packaging/linux/runtime-libraries.txt` lists those packages;
`dynamic-libraries.txt` also covers loaders such as Wayland and Vulkan.
The package's `runtime/` directory and manifest retain this information.

After archive creation, a native Linux runner with Docker can run:

```sh
bash scripts/release/linux-runtime.sh smoke --archive "$ARCHIVE" --input "$RELEASE_INPUT" --target "$TARGET"
```

This starts the matching Ubuntu 24.04 architecture, installs only the
declared runtime packages plus Python/binutils for verification, extracts
the archive, audits both binaries, loads the declared dynamic libraries,
and runs the CLI in isolation. It does not start a GUI or supply graphics
drivers. Actual Wayland desktop startup remains a native acceptance check.

## macOS bundle and ad-hoc signing

On macOS, after shared staging:

```sh
bash scripts/release/macos-package.sh package --staging "$STAGING"
```

The bundle identifier is **`io.github.vhodges.manyhands`**. The helper
builds `Manyhands.icns` from the committed artwork, derives the minimum
macOS deployment version from every packaged Mach-O file, and records it
consistently in `Info.plist`, the manifest, and README. Full SemVer remains
in release metadata; bundle short/build versions use the numeric core.

Reviewed Homebrew OpenSSL 3 libraries are copied and relocated into the
app's `Contents/Frameworks` and the standalone CLI's sibling `lib/`.
Original OpenSSL notices are collected and installed in the app resources
as well as the archive. Unknown external libraries fail for review rather
than being copied without their distribution terms. Obsolete RPATHs are
removed. Dylibs and executables are ad-hoc signed before the outer bundle
is sealed; the result must verify strictly and retain an ad-hoc signature.

After archiving/extracting through the common helpers, verify the extracted
payload before recording the native result:

```sh
bash scripts/release/macos-package.sh verify --staging "$EXTRACTED_PAYLOAD"
```

`EXTRACTED_PAYLOAD` is the versioned directory inside `verify.py`'s
`--extract-to` destination. This check repeats architecture, relative
linkage, minimum-OS, and signature checks. Ad-hoc signing does not provide
Developer ID trust or notarization.

## Windows self-signing

See [Windows signing](windows/SIGNING.md) for the native build flags,
maintainer certificate provisioning, secrets, timestamp configuration,
signing, cleanup, and public-only extracted verification. The Windows job
must build both binaries with `RUSTFLAGS=-C target-feature=+crt-static`.
Its import policy permits only the reviewed OS/API-set names in
`windows/system-libraries.txt`; ordinary modules must also exist in the
native OS System directory. Presence on the runner alone is insufficient.
Unknown contracts/modules, dynamic CRT imports, and unreviewed bundled
DLLs fail the job. No non-system dependency graph is currently permitted.

`windows-sign.ps1 -Mode Sign` performs the native audit and signing,
then stamps the verified public identity/runtime metadata. `-Mode Verify`
repeats native verification on extracted outputs without access to the
private signing secrets. The CLI test removes developer-tool PATH entries
so Visual Studio/Git/toolchain DLLs cannot satisfy missing app dependencies.
