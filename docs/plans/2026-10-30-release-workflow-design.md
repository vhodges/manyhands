# Release Workflow Design

Date: 2026-10-30

Status: Stage 1 implemented and locally reviewed; native acceptance pending after merge

## Goal

Provide a manually invoked Release workflow that builds the desktop and CLI
for the four supported native targets, signs macOS and Windows outputs,
packages binaries with their documentation and licences, and assembles a
draft GitHub Release. Follow with Homebrew, Windows installer, and Nix
distribution using the same release contract.

The owner approved the two-stage rollout, draft releases, and tag-first
versioning. The existing Build workflow remains the native test/lint workflow.

## Delivery Stages

1. **Release foundation:** tag validation, four native builds, signing,
   archives, licensing, dependency checks, checksums, and draft assembly.
2. **Installation channels:** a personal Homebrew cask, an Inno Setup Windows
   installer, and a source-building Nix flake for Linux.

Stage 2 consumes the Stage 1 payload and metadata. It does not introduce a
second version scheme or rebuild binaries differently for each installer.

## Invocation And Versioning

Add `.github/workflows/release.yml` with display name `Release` and only a
`workflow_dispatch` trigger. Its required string input is `tag`, for example
`v0.1.0`. Dispatch the workflow from `main`, providing the tag separately.

The operator first updates `Cargo.toml` and the root package entry in
`Cargo.lock`, merges the release-ready revision, then creates and pushes its
tag. The workflow never creates or moves a tag and never changes versions.

Accept `vMAJOR.MINOR.PATCH`, optionally followed by a SemVer prerelease
identifier. Reject build metadata in this initial version to keep asset,
bundle, and installer version mappings unambiguous. The value after `v` must
exactly match the package version at the tagged commit. Infer GitHub's
prerelease flag from the SemVer prerelease component.

Resolve the tag to a commit once in a preparation job. Check out that SHA in
each subsequent job and record it in release metadata. Recheck the remote
tag before writing release assets; fail if it no longer resolves to the same
commit. Reject an existing published release before building and recheck
that condition before uploading.

## Workflow Architecture

The workflow has three job boundaries:

1. **Prepare:** resolve and validate the tag, version, and existing release
   state; emit the commit, version, and prerelease flag.
2. **Build/package matrix:** build each native target, stage documents and
   licences, resolve runtime dependencies, sign where applicable, create an
   archive, and verify its extracted contents. Upload completed archives as
   internal Actions artifacts for handoff.
3. **Assemble draft:** download the four archives and their inventories,
   verify the complete expected set, calculate checksums, and create or
   refresh the draft release.

Only the final job needs `contents: write`; other jobs use `contents: read`.
Use a per-tag concurrency group with `cancel-in-progress: false`. Use a
separate release Cargo-cache namespace from Build. Internal Actions
artifacts are transport between jobs; GitHub Release assets are the durable
user downloads. Build does not regain an artifact-upload step.

Release validates packaging and runs a packaged CLI smoke test. It does not
copy Build's full native test matrix. The operator selects a revision whose
normal verification is satisfactory. Native workflow dispatch requires
fresh product-owner approval, including the first Release trial.

## Native Builds

| Runner | Rust target | Archive |
| --- | --- | --- |
| `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| `windows-2022` | `x86_64-pc-windows-msvc` | `.zip` |
| `macos-15` | `aarch64-apple-darwin` | `.zip` |

Use the native prerequisite setup in the current Build workflow as the
starting point. Release does not need its SSH test-fixture provisioning.
Build the two front ends separately:

```sh
cargo build --locked --release --no-default-features --bin manyhands-cli --target "$TARGET"
cargo build --locked --release --no-default-features --features desktop --bin manyhands --target "$TARGET"
```

Do not enable `editor-probe`, `desktop-test-support`, or `--all-features`.
Keep `scripts/check-cli-no-gpui.sh` as a dependency-boundary check. Pin the
release Rust toolchain during implementation and use the same version for
Nix builds, provided it is supported by the existing locked dependencies.
The workflow records the compiler version; byte-for-byte reproducibility
across signing timestamps and native SDK updates is not an initial promise.

Use Bash for Unix runners and PowerShell for Windows, with shared command
arguments constructed in Python. Licence generation writes JSON and HTML
through cargo-about's explicit output-file option so PowerShell ancestry
does not trigger its stdout/encoding guard.

Windows release builds additionally set
`RUSTFLAGS=-C target-feature=+crt-static` for both front ends. Import audits
reject dynamic CRT linkage and accept only the reviewed OS/API-set module
names in `packaging/windows/system-libraries.txt`. No unreviewed DLL is
accepted merely because a build runner has it installed.

## Payloads And Asset Names

Name each archive `manyhands-v<VERSION>-<RUST-TARGET>.<EXT>`. Linux and
Windows archives contain one correspondingly named top-level directory:

```text
manyhands-v<VERSION>-<RUST-TARGET>/
  bin/
    manyhands[.exe]
    manyhands-cli[.exe]
  README.md
  LICENSE
  THIRD-PARTY-NOTICES.html
  release.json
  licenses/
    fonts/
      README.md
      LICENSE.txt
    runtime/                    # When external libraries are redistributed.
  lib/                          # Linux libraries, if explicitly bundled.
  signing/                      # Windows public certificate and explanation.
```

Windows runtime DLLs, when needed, live beside the executables, not in a
separate directory outside the DLL search path. Linux's `lib/` is optional;
the archive is explicitly a dynamically linked GNU/Linux distribution,
not a self-contained executable for every distribution.

The macOS archive contains the same top-level directory, with
`Manyhands.app` and `bin/manyhands-cli` instead of `bin/manyhands`.
`Manyhands.app/Contents/MacOS/manyhands` is the desktop executable. Bundle
resources include the icon, README, MIT licence, third-party notices, font
documents, and `release.json`, so moving the app to `/Applications` retains
its disclosures. Keep shared libraries under `Contents/Frameworks` when
needed. Generate an `.icns` icon from `brand/manyhands-icon.svg`.

The macOS bundle identifier is `io.github.vhodges.manyhands`. OpenSSL 3 is
the initial reviewed external dylib family: copy it into both app and CLI
runtime layouts with original notices, then rewrite install names and
remove obsolete build RPATHs. Other external libraries require an explicit
distribution policy before packaging.

The manifest identifies version, tag, source SHA, target triple, compiler
version, signing mode, and documented runtime requirements. It contains no
absolute runner paths or private signing information.

## Signing

### macOS

Use ad-hoc signing (`codesign --sign -`), which needs no Apple account or
repository secrets. Finish all payload changes and dynamic-library install
name rewrites before signing. Sign nested Mach-O libraries/helpers first,
then executables and the outer app bundle. Verify the CLI and completed
bundle with strict signature verification. Archive with a macOS-aware tool
such as `ditto` that preserves the bundle.

Ad-hoc signing does not provide Developer ID identity, notarization, or
Gatekeeper approval. State that accurately in the release README. Verify
minimum macOS versions from the executable and any bundled dependencies;
the app's `Info.plist` and later cask must agree with the actual requirement.

### Windows

Use a persistent self-signed code-signing certificate, created once by the
maintainer and supplied through repository secrets:

- `WINDOWS_SIGNING_PFX_BASE64`: base64-encoded certificate and private key.
- `WINDOWS_SIGNING_PFX_PASSWORD`: PFX password.

Missing, expired, or unusable signing material fails the Windows job. Never
silently produce an unsigned release or generate a new publisher identity
on each run. Import it into a temporary runner certificate context, sign
both executables with Authenticode/SHA-256, and verify the expected
certificate identity and file integrity. Use RFC 3161 timestamping and fail
on timestamp errors. Stage 2 also signs the completed installer.

The v0.1.3 native trial rejected the configured HTTPS timestamp URL. Use
the reviewed `http://timestamp.digicert.com/` RFC 3161 endpoint, matching
[Microsoft's SignTool example](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool#examples).
Validate and normalize that exact endpoint before invoking SignTool; do not
permit arbitrary HTTP services or silently fall back to untimestamped output.
The timestamp token is cryptographically signed despite HTTP transport;
retain SHA-256 file/timestamp digests and mandatory native verification.
Keep the existing persistent publisher identity and certificate cleanup.

Self-signed certificates are not trusted by Windows users by default.
Cryptographic verification and trusted-publisher validation are distinct;
temporary trust needed for runner verification must not be misreported as
public trust. Include only the public certificate, its fingerprint, and an
explanation in the package. Do not install trust certificates on users'
machines automatically. Clean up imported private material on success or
failure. Self-signing does not remove SmartScreen warnings.

After the v0.1.5 trial isolated a stall at the current-user root import, the
owner approved temporary `LocalMachine\Root` public verification trust on
elevated, disposable GitHub-hosted Windows runners only. Reject other
contexts before private-key handling, retain the private identity in
`CurrentUser\My`, preserve existing trust, and remove only the exact root
certificate added by this invocation. Both signature verifiers and timestamp
requirements remain mandatory. See the
[trust correction design](2026-10-31-windows-verification-trust-design.md).

## Licences And Runtime Dependencies

The repository's MIT licence is the application licence. The bundled IBM
Plex Sans files retain their own SIL Open Font License and IBM copyright.
`src/desktop/fonts.rs` identifies their source as `@ibm/plex-sans@1.1.0`,
`fonts/complete/ttf`. Add `assets/fonts/README.md` with that provenance and
the list of included static faces; distribute it unchanged with
`assets/fonts/LICENSE.txt` in every full package.

Generate dependency notices from the locked normal dependency graphs for
the CLI and desktop on each target, using a pinned `cargo-about` version.
Include licence text and attribution, not just SPDX names. Configure and
review any necessary licence clarifications explicitly. Audit bundled
native libraries and other embedded third-party assets as well; Cargo
metadata alone does not describe all redistributed material. A licence
requiring additional source-delivery obligations needs those obligations
resolved before distribution, not merely an entry in a notice file.

The first implementation batch preserves nested upstream notice files and
includes checksum-verified original `.crate` source archives for selected
MPL-covered dependencies and the libgit2 binding/vendor source. This also
retains vendored native and Lucide/Feather icon notices that a crate-level
SPDX label alone would omit. All four target-specific notice bundles have
been generated locally; this does not substitute for native linkage audits.

Inspect runtime linkage using `ldd`/`readelf`, `otool -L`, and PE import
inspection on the respective platforms. macOS must not depend on runner
Homebrew paths. Windows must not depend on DLLs only present on the build
runner; redistribute required runtime files under their applicable terms.
Keep an explicit documented Linux system-library baseline rather than
copying glibc or graphics drivers into archives. Builds on Ubuntu 24.04
initially target that baseline and compatible newer systems. Linux desktop
use requires Wayland and a working graphics stack. Git remains an external
runtime requirement where the application invokes it.

## Draft Assembly And Recovery

Create the draft only after all four native jobs have succeeded and their
expected archives have been verified. Attach the four archives,
`SHA256SUMS`, and a release-level manifest identifying the commit and target
set. Explain installation requirements and signing modes in the draft's
release notes. No workflow step publishes the draft.

The release-level `release.json` contains immutable source identity and
target/archive/signing descriptors; detailed build/runtime metadata stays
in each archive's own manifest. Establish the source manifest first and
never clobber it on reruns. This retains a source-identity anchor even when
maintainers fully rewrite the notes and an archive replacement fails.

For a rerun, first confirm that an existing release is still a draft,
refers to the same source revision, and has the correct prerelease flag.
Verify the complete candidate set before mutation. Retain the source
manifest, remove stale `SHA256SUMS`, replace only the owned archives, then
upload new checksums last. Preserve maintainer-edited notes and unrelated
assets. An interrupted upload leaves an incomplete draft for a subsequent
same-source run to repair.

GitHub uploads are not transactional with manual publication. Recheck
source/tag/draft state before every mutation and stop when publication is
observed. These checks cannot atomically lock the release UI; maintainers
must not publish while a run is active. The workflow itself never publishes.

## Stage 2 Channels

### Homebrew

Use a personal tap, conventionally `vhodges/homebrew-tap`, with a
`manyhands` cask restricted to Apple Silicon. The cask downloads the
published macOS ZIP, verifies its checksum, installs `Manyhands.app`, and
links `manyhands-cli`. Do not depend on acceptance into the official cask
repository. Ad-hoc signing's installation limitations remain documented.

Generate a version/checksum-specific cask as a release artifact; update the
tap after the release is published so it never points at draft-only assets.
Cross-repository publication needs separate owner authorization and tap
access. No automatic removal of macOS quarantine protections.

### Windows Installer

Use Inno Setup for a per-user installer containing both executables and
the same notices. Provide a Start menu shortcut, optional CLI PATH
integration, a stable application identifier, and predictable upgrade and
uninstall behavior. Preserve user data and repositories on uninstall.
Generate an `.ico` from the existing brand icon. Keep the desktop's GUI
launch free of a console window while retaining the CLI's console behavior.

Use numeric `MAJOR.MINOR.PATCH.0` for Windows file metadata and the full
SemVer value for display. Prereleases use the same install identity; the
installer must prevent an older stable/prerelease payload from silently
replacing a newer one. Include the finished, signed installer in release
checksums. WinGet submission is subsequent distribution work.

### Nix

Add a root `flake.nix` and `flake.lock` with pinned inputs, exposing
`packages.<system>.default` (desktop plus CLI) and
`packages.<system>.manyhands-cli` for `x86_64-linux` and `aarch64-linux`.
Build from source with `Cargo.lock`; do not execute Ubuntu release
binaries on NixOS. Wrap the desktop with the dynamic library paths GPUI
loads at runtime, including Wayland, and install an icon, desktop entry,
and all disclosures under standard share directories.

The CLI-only derivation excludes desktop dependencies. The flake provides
consumer packages without replacing the existing Devenv development
setup. Users pin a published release tag in their flake input or install
from the tagged repository. Native support for Darwin through the Nix
flake is outside the initial scope.

## Acceptance And Verification

- Static workflow validation and local packaging tests pass.
- Tag mismatch, missing tag, and published-release cases fail before
  native builds or asset mutation.
- Each extracted archive contains both front ends, correct metadata,
  readable notices, and the expected file modes.
- The packaged CLI's `--help` command succeeds in an isolated directory.
- Platform signatures verify and the documented trust limitations are
  accurate; macOS's copied app retains its disclosures and signature.
- Runtime dependency inspection finds no unresolved private runner paths.
- Four successful jobs produce exactly the expected owned archive set and
  checksums in an unpublished draft; failed jobs do not start assembly.
- Native trials happen only with fresh owner approval, after merge on main.
- Stage 2 verifies a real cask installation, Windows install/upgrade/remove,
  and both Nix package variants; retain cross-platform manual checks in
  the execution record rather than claiming them from Linux static checks.

## References

- [Implementation plan](2026-10-30-release-workflow-implementation.md)
- [Execution record](2026-10-30-release-workflow-execution.md)
- [Current native Build workflow](../../.github/workflows/build.yml)
- [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Microsoft package signing guidance](https://learn.microsoft.com/en-us/windows/msix/package/signing-package-overview)
- [Apple distribution and notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
