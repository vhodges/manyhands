# Release Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build signed, documented archives for four native targets through a manual, tag-first workflow that assembles draft GitHub Releases, then add Homebrew, a Windows installer, and Linux Nix packages.

**Architecture:** A preparation job resolves an existing tag to an immutable source SHA; four native jobs build, stage, sign, and verify their payloads; a final job assembles a draft only when all targets succeed. Shared packaging metadata and documentation feed the archives and subsequent installation channels. Keep the existing Build workflow independent.

**Tech Stack:** GitHub Actions, Cargo/Rust, Python 3.11+ standard-library packaging helpers and tests, PowerShell, Apple bundle/signing tools, Windows SDK SignTool, cargo-about, Inno Setup, Homebrew Cask, Nix flakes.

---

## Approved Contract And Starting Point

Read [the approved design](2026-10-30-release-workflow-design.md) first.
The approved choices are manual dispatch, an existing version tag, four
current native targets, both front ends, ad-hoc macOS signing, persistent
self-signed Windows signing, draft releases, and staged distribution work.

The planning worktree is `.worktrees/release-workflow`, branch
`release-workflow`, starting at `48ffc058`. It initially contained documentation only.
No workflow has been dispatched and no signing identity has been created.
The design and plan have not been committed or pushed; obtain authorization
before doing so. Commit commands below describe checkpoints to use when
commit authorization is present, not permission to commit automatically.

Stage 1 Tasks 1–8 are now implemented and locally checked; see the
[execution record](2026-10-30-release-workflow-execution.md) for evidence,
implementation refinements, and code-review results. Task 9's review/handoff
record is written; actual native acceptance remains post-merge work with
fresh owner authorization. Stage 2 Tasks 10–12 are not yet implemented.

Before implementation, fetch origin and merge `origin/main` into this
branch. If the owner assigns this work to a Manyhands ticket/Cycle, use its
registered ticket worktree and lifecycle procedure instead, moving the
owned planning documents there before proceeding. Never rebase a published
ticket branch. Consult `AGENTS.md` again in the implementation worktree.

Existing sources worth reading:

- `.github/workflows/build.yml`: four runners and native prerequisites.
- `Cargo.toml`, `Cargo.lock`: versions, desktop gating, locked dependencies.
- `scripts/check-cli-no-gpui.sh`: CLI dependency boundary.
- `src/main.rs`: desktop entry point; currently no Windows GUI subsystem attribute.
- `src/desktop/fonts.rs`, `assets/fonts/LICENSE.txt`: font provenance/licence.
- `brand/manyhands-icon.svg`: source application icon.
- `devenv.nix`: Linux runtime libraries and current development environment.

Initial verified tool versions are Rust `1.98.1` (local Devenv) and
`cargo-about` `0.9.3` (upstream release). Verify rustup offers `1.98.1` on
all four targets before pinning it; if not, select an available compatible
toolchain, validate it locally, and record the reason. Pin tool versions
in `packaging/tool-versions.env`; changing that file is an explicit upgrade.

Local Rust commands run through `devenv shell -- ...`. Native Actions
runners may run Cargo directly under the existing owner exception.
Packaging tests use standard-library `unittest`; do not introduce a Rust
dependency solely to test workflow helpers.

Implementation setup: add `python3` to `devenv.nix`'s packages (the initial
environment had no Python executable), and ignore Python bytecode caches.
Install cargo-about 0.9.3 with `--features cli`; its CLI is not enabled by
default in that release. Run Devenv shell invocations sequentially within
one worktree: concurrent invocations were observed racing on generated
`.devenv/load-exports` state. Matrix jobs use independent native checkouts.

## Stage 1 — Release Foundation

### Task 1: Define Tag And Manifest Validation

**Files:**
- Create: `scripts/release/common.py`
- Create: `scripts/release/prepare.py`
- Create: `scripts/release/tests/test_prepare.py`
- Create: `packaging/tool-versions.env`
- Create: `packaging/README.md`

**Step 1: Add meaningful validation cases.** Use temporary Git repositories
with minimal `Cargo.toml`/`Cargo.lock` fixtures. Cover an existing annotated
tag, a lightweight tag, a missing tag, a branch name passed as a tag,
version mismatch, lockfile mismatch, prerelease parsing, and malformed tag
input. Values containing shell metacharacters must be rejected without
executing anything. Use argument-vector subprocess calls, never `shell=True`.

The parsing interface is:

```python
def parse_release_tag(tag: str) -> tuple[str, bool]:
    """Return (package_version, is_prerelease), or raise ValueError."""

def prepare_release(repo, tag: str) -> dict:
    """Resolve refs/tags/<tag>^{commit}; read and validate tagged manifests."""
```

Test behavior rather than the particular regex. Required accepted examples
are `v0.1.0` and `v0.2.0-rc.1`; reject leading-zero numeric components,
empty prerelease identifiers, leading-zero numeric prerelease identifiers,
and SemVer build metadata. Read files from the resolved commit with Git,
not from an unrelated working copy.

**Step 2: Run the new tests and observe the expected missing-helper failures.**

```sh
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -p 'test_prepare.py' -v
```

**Step 3: Implement preparation.** The command interface is:

```sh
python3 scripts/release/prepare.py --tag "$RELEASE_TAG" --output release-input.json
```

Write a versioned JSON contract with `schema_version: 1`, `tag`, `version`,
`commit`, and `prerelease`. Emit the same scalar job outputs through an
explicit `--github-output` argument when Actions supplies `$GITHUB_OUTPUT`.
Validate the root `manyhands` package version in both tagged manifests.
Fail with a concise diagnostic and no output manifest on invalid input.
Document that tags must include the merged release packaging support.

**Step 4: Repeat the command from Step 2.** Expect all cases to pass. Add
the tool version file with explicit `RUST_VERSION` and
`CARGO_ABOUT_VERSION` values after checking tool availability.

**Step 5: Authorized checkpoint.**

```sh
git add scripts/release/common.py scripts/release/prepare.py scripts/release/tests/test_prepare.py packaging/tool-versions.env packaging/README.md
git commit -m "ci: define tag-first release metadata"
```

### Task 2: Build The Licensing And Release Documentation Payload

**Files:**
- Create: `assets/fonts/README.md`
- Create: `packaging/README-release.md`
- Create: `packaging/licenses/about.toml`
- Create: `packaging/licenses/about.hbs`
- Create: `packaging/licenses/README.md`
- Create: `scripts/release/notices.py`
- Modify: `packaging/README.md`
- Modify: `README.md`

**Step 1: Inventory shipped content.** Resolve normal dependencies for the
featureless CLI and desktop for each target. Inspect their actual licence
expressions/texts and any embedded assets, not only the root crate's MIT
licence. Inspect native libraries separately. Record the source and licence
for each item not covered by Cargo's graph. Resolve source-offer or other
distribution obligations if encountered; do not paper over them with
blanket licence acceptance or guessed licence text.

**Step 2: Add the font README.** Record `@ibm/plex-sans@1.1.0`, the upstream
`fonts/complete/ttf` directory, the six filenames listed in
`src/desktop/fonts.rs`, IBM's copyright and reserved font name, the SIL OFL
1.1, and the fact that the font licence is separate from Manyhands' MIT
licence. Keep the existing font licence intact.

**Step 3: Generate notices.** Install cargo-about through Devenv with the
exact version from `packaging/tool-versions.env`. Start from its documented
configuration and template format for that version. Generate notices for
both feature graphs and combine them in one HTML document; duplicate
attributions are acceptable, dropped licence text is not. Use `--locked`
and the target filter supported by the pinned tool. Explicitly check
`cargo about generate --help` rather than assuming its flags match Cargo.
`notices.py` invokes the tool with argument lists and fails if gathering or
rendering fails. Capture native notices alongside the generated document.

**Step 4: Write the release README template.** Include version/source,
directory layout, desktop/CLI startup, Git requirements, Linux Wayland and
system-library baseline, macOS ad-hoc signing limitations, Windows
self-signed identity limitations, checksums, and licence locations. Only
mention artifacts/channels actually available in that stage. Use explicit
named template variables and fail on unresolved substitutions.

**Step 5: Verify against real locked dependencies.** Generate notices for
the local Linux target through Devenv, inspect representative complete
licence texts and IBM attribution, and ensure the result is HTML-escaped.
Keep generated build output in a temporary directory, not tracked source.
Other target graphs are verified in their native matrix jobs. Review
configuration/clarification entries before an authorized docs checkpoint.

### Task 3: Implement Shared Staging And Archive Verification

**Files:**
- Create: `scripts/release/package.py`
- Create: `scripts/release/verify.py`
- Create: `scripts/release/tests/test_package.py`
- Create: `scripts/release/tests/test_verify.py`
- Modify: `scripts/release/common.py`

**Step 1: Add fixture-driven tests for the distribution boundary.** Build
small synthetic payloads in temporary directories. Verify that absent
binaries or notices fail, Linux execute bits survive tar/extract, Windows
suffixes are required, manifest target/version mismatches fail, and a
private PFX/key cannot enter a payload. Add a malformed archive with
absolute/traversal paths to ensure inspection cannot extract outside its
temporary root. Include paths containing spaces.

**Step 2: Run the new tests to establish the failures.**

```sh
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -p 'test_package.py' -v
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -p 'test_verify.py' -v
```

**Step 3: Implement the shared CLI contracts.**

```sh
python3 scripts/release/package.py stage --input release-input.json --target "$TARGET" --binaries "target/$TARGET/release" --notices "$NOTICES" --output "$STAGING"
python3 scripts/release/package.py archive --input release-input.json --target "$TARGET" --staging "$STAGING" --output "$DIST"
python3 scripts/release/verify.py --archive "$ARCHIVE" --input release-input.json --target "$TARGET" --smoke-cli
```

Staging produces the layout in the design and readable, target-specific
`release.json`. Platform scripts may add audited libraries and signing
material between `stage` and `archive`. No payload mutation occurs after
signing. Use Python's tar support on Linux, ZIP handling on Windows, and
`ditto -c -k --sequesterRsrc --keepParent` on macOS. Account explicitly for
macOS ZIP metadata entries rather than treating them as executables.

Verify the extracted copy, not just the staging directory. Run its CLI
with `--help` from a temporary working directory, with a timeout and no
inherited developer library paths. Provide only the documented runtime
environment. Recheck signatures on extracted native outputs in Tasks 5/6.

**Step 4: Repeat the fixture tests.** Expect successful round trips and
clear failures for intentionally incomplete/invalid archives. Make an
authorized checkpoint containing only shared packaging helpers/tests.

### Task 4: Establish Linux Runtime Compatibility

**Files:**
- Create: `scripts/release/linux-runtime.sh`
- Create: `packaging/linux/runtime-libraries.txt`
- Modify: `packaging/README-release.md`
- Modify: `scripts/release/verify.py`

**Step 1: Build the real local binaries through Devenv.**

```sh
devenv shell -- cargo build --locked --release --no-default-features --bin manyhands-cli
devenv shell -- cargo build --locked --release --no-default-features --features desktop --bin manyhands
```

These Nix-linked local binaries are for local inspection, not the Ubuntu
release assets. Run the boundary script through Devenv as well:

```sh
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

**Step 2: Inspect linkage and dynamic loads.** Record direct/transitive
library dependencies, interpreter, and versioned glibc requirements using
`ldd` and `readelf`; inspect GPUI's known dynamic loads, including Wayland
and Vulkan. Translate dependencies to Ubuntu runtime package names rather
than requiring development packages. Do not bundle glibc, graphics drivers,
or arbitrary libraries recursively from the runner.

**Step 3: Implement the audit.** Fail on unresolved links, accidental
private build paths, wrong architecture, and unaccounted non-system
dependencies. If bundling a redistributable is necessary, stage it under
`lib/`, add its licence/source attribution, establish a relative runtime
search path before packaging, and verify the result. Otherwise explicitly
document it as a system prerequisite.

**Step 4: Verify Ubuntu payloads on an isolated runtime.** In each native
Linux job, install only documented runtime packages in an Ubuntu 24.04
runtime environment and run the extracted CLI. Do not claim this as a
desktop display test. Record GUI startup on a real Wayland machine during
the owner-approved native acceptance step.

### Task 5: Create And Ad-Hoc Sign The macOS Bundle

**Files:**
- Create: `packaging/icons/manyhands-1024.png`
- Create: `packaging/macos/Info.plist.in`
- Create: `scripts/release/macos-package.sh`
- Modify: `scripts/release/package.py`
- Modify: `packaging/README.md`

**Step 1: Generate and inspect the application artwork.** Render
`brand/manyhands-icon.svg` to a 1024-square PNG using a recorded renderer
version. Commit that derived image for native jobs to consume. On macOS,
use `sips` to build the standard iconset sizes and `iconutil` to create
`Manyhands.icns`. Keep generated build-specific iconsets untracked.

**Step 2: Populate the app.** The plist sets `CFBundleExecutable` to
`manyhands`, the application name to `Manyhands`, a stable reverse-DNS
identifier, `CFBundlePackageType` to `APPL`, numeric short/build versions,
and the icon name. Choose the identifier once and document it before the
first distribution. Preserve full SemVer in `release.json` and user docs.
Derive the minimum OS version from the actual executable/dependency load
commands; make the plist agree rather than asserting macOS 14 just
because that is the runner version.

**Step 3: Resolve linkage.** Inspect `otool -L` recursively. Copy necessary
redistributable non-system dylibs into `Contents/Frameworks`, add notices,
and rewrite load paths using `install_name_tool`. Give the standalone CLI
an independently valid runtime layout if it needs non-system dylibs.
Verify neither executable refers to `/opt/homebrew`, `/usr/local/Cellar`,
or a runner-only path. Finish all plist/resources/library edits now.

**Step 4: Sign inside-out.** The essential commands, applied in dependency
order with properly quoted paths, are:

```sh
codesign --force --sign - "$NESTED_MACHO"
codesign --force --sign - "$STAGING/bin/manyhands-cli"
codesign --force --sign - "$STAGING/Manyhands.app"
codesign --verify --strict --verbose=2 "$STAGING/bin/manyhands-cli"
codesign --verify --deep --strict --verbose=2 "$STAGING/Manyhands.app"
```

Do not use `--deep` as a substitute for an inside-out signing strategy.
Verify all nested code as well. Produce the archive with `ditto`, extract
to a different directory, repeat signature checks, and smoke-test the
extracted CLI. Record Finder launch and Gatekeeper behavior during native
acceptance; an ad-hoc signature is not a notarization claim.

### Task 6: Add Persistent Windows Self-Signing

**Files:**
- Create: `scripts/release/windows-sign.ps1`
- Create: `packaging/windows/SIGNING.md`
- Modify: `scripts/release/package.py`
- Modify: `scripts/release/verify.py`
- Modify: `packaging/README.md`

**Step 1: Document maintainer provisioning.** Provide PowerShell commands
using `New-SelfSignedCertificate -Type CodeSigningCert` and
`Export-PfxCertificate` to create a persistent identity on the maintainer's
machine. Use interactive password entry. Describe setting the two secrets
named in the design with `gh secret set`, without printing/committing a
private key, password, or PFX. Provisioning is an owner action, not an
automatic workflow step. Document expiration and certificate replacement.

**Step 2: Implement strict import and cleanup.** Read secrets only from
environment variables. Decode the PFX into the runner's temporary
directory, import into `Cert:\CurrentUser\My`, verify private-key/code
signing capability and validity dates, and export the public `.cer` only.
Use `try`/`finally` to remove private files and imported certificates even
when signing or timestamping fails. Track thumbprints actually imported
so cleanup cannot remove unrelated runner certificates.

**Step 3: Sign both executables.** Discover SignTool from the Windows SDK.
Select the imported identity explicitly and use SHA-256 file digest plus
RFC 3161 SHA-256 timestamping. Keep the timestamp URL in checked-in config.
Select the identity via its store thumbprint rather than exposing the PFX
password on the SignTool command line. Check `$LASTEXITCODE` after native
commands. A missing secret or signing/timestamp failure terminates the job.

**Step 4: Verify signatures accurately.** Verify signature integrity,
expected signer thumbprint, and timestamp. If `signtool verify /pa` needs
the self-signed certificate trusted, install public trust only temporarily
in the disposable runner's current-user certificate store and remove it
after verification. State that this is a controlled runner check, not
public trust. Include `signing/manyhands.cer` and `signing/README.md` in
the distribution; never auto-import trust on the user's machine.

**Step 5: Verify extracted artifacts.** Repeat signature checks and
`manyhands-cli.exe --help` from the extracted ZIP. Inspect native imports
with `dumpbin /dependents` (or equivalent PE inspection), including
transitive dependencies. Stage permitted non-system DLLs next to the EXEs
and include their notices. Verify with build-tool paths removed from PATH.
Do not accept a successful smoke test caused by runner-only DLLs.

### Task 7: Implement Draft Assembly And Rerun Recovery

**Files:**
- Create: `scripts/release/assemble.py`
- Create: `scripts/release/tests/test_assemble.py`
- Modify: `scripts/release/common.py`

**Step 1: Test release lifecycle behavior with a fake GitHub boundary.**
Cover: one missing target; duplicate target; manifest/source mismatch;
checksum mismatch; no prior release; same-SHA draft rerun; published
release rejection; different-SHA draft rejection; interrupted upload then
successful repair; preservation of edited notes and unrelated assets;
and a release published between validation and upload. No test contacts
GitHub or requires a live token.

**Step 2: Establish failures, then implement assembly.**

```sh
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -p 'test_assemble.py' -v
```

The command contract is:

```sh
python3 scripts/release/assemble.py preflight --input release-input.json --repo "$GITHUB_REPOSITORY"
python3 scripts/release/assemble.py assemble --input release-input.json --assets "$DOWNLOADED_ASSETS" --repo "$GITHUB_REPOSITORY"
```

Use `gh` through argument-vector subprocess calls with `GH_TOKEN` in the
environment; parse its JSON outputs. Distinguish a not-found release from
authentication, connectivity, or other API errors. Never create a release
because lookup failed for an unrelated reason. `preflight` performs the
remote tag/release checks without mutation and runs in the preparation job
before the native matrix. Test published-release rejection through this
entry point as well as through final assembly.

**Step 3: Enforce a complete candidate set before mutation.** Require
exactly the expected four archive names and matching target/commit
inventories. Hash archive bytes after all signing and packaging. Write a
release-level `release.json` and `SHA256SUMS`, sorted by asset filename,
then create the draft with `--verify-tag` and the correct prerelease flag.
Set `--target` to the resolved source SHA and record that commit in
machine-readable release metadata and an identifiable marker in initial
notes. A rerun can then check source identity even if the first attempt
was interrupted before the metadata asset finished uploading.

Implemented recovery refinement: the release-level manifest contains only
source-determined identity/target/archive/signing descriptors. Upload it
first and retain its exact bytes across same-source reruns; detailed build
metadata remains inside each archive and its internal handoff inventory.
This immutable anchor must not be replaced with `--clobber`, which deletes
an old asset before uploading its replacement. Archives/checksums remain
replaceable. A rerun also rejects an incorrect GitHub prerelease flag.

**Step 4: Implement repairable draft updates.** Never use a blanket
delete/recreate operation. Revalidate release draft state, tag resolution,
and source identity before replacing owned assets. Use targeted asset
replacement only after the full new set is available. Preserve notes and
unrelated assets. Mark assembly status in the Actions summary; publishing
is permitted only after the complete successful run. Document the API's
non-transactional upload boundary and the prohibition on publishing while
a run is active. Repeat tests and inspect all intended failure diagnostics.

### Task 8: Wire The Manual Native Workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `packaging/README.md`
- Modify: `README.md`

**Step 1: Add the dispatch and concurrency contract.**

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      tag:
        description: Existing release tag, for example v0.1.0
        required: true
        type: string

permissions:
  contents: read

concurrency:
  group: release-${{ inputs.tag }}
  cancel-in-progress: false
```

Pass input values via environment variables into scripts; never interpolate
tag input into shell program text. Require invocation from `main` and
explain that the source tag is a separate input. Checkout for preparation
must include tags/history. Run both local preparation and the non-mutating
GitHub preflight before emitting success. The four matrix jobs check out
the preparation job's resolved SHA and verify it rather than resolving
the tag again.

**Step 2: Add jobs using the design's exact four-entry matrix.** Copy
needed native build prerequisites from current Build, excluding SSH
fixture-only helpers. Install the pinned Rust version, Python 3.11+, the
pinned notices tool, and native packaging tools. Cache Cargo with an
explicit release namespace. Run the two separate build commands from the
design, shared staging, platform audit/signing, archiving, and extracted
verification. Expose Windows secrets only to its signing step.

Use `RUSTFLAGS=-C target-feature=+crt-static` for both Windows builds.
Retain native Windows Perl/`IPC::Cmd` validation for vendored OpenSSL even
though SSH fixture setup itself is omitted. Install macOS `openssl@3` as a
build/runtime-bundling prerequisite, not only as a fixture generator.
Linux jobs must run both the host audit and post-archive Ubuntu container
check. macOS jobs must run extracted `macos-package.sh verify`; Windows
jobs must run extracted `windows-sign.ps1 -Mode Verify` without secrets.

**Step 3: Add job handoff and final assembly.** Upload already-compressed
archives plus inventories with `actions/upload-artifact@v4`, compression
disabled and missing files treated as errors. The final Ubuntu job
depends on prepare plus the entire build matrix, checks out the validated
source, downloads this run's artifacts, and invokes assembly with
job-scoped `contents: write`. Write the resulting draft URL and expected
assets into the summary. A failed matrix must not run final assembly.

**Step 4: Validate static behavior locally.**

```sh
nix shell nixpkgs#actionlint -c actionlint .github/workflows/release.yml
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v
git diff --check
```

Use `shellcheck` for the new shell scripts as available through Nix, and
PowerShell's parser for `.ps1` syntax on a Windows/native validation run.
Confirm Build still has no upload step and Release has no push, tag-push,
schedule, or `release` event trigger. Inspect the complete diff before
an authorized Stage 1 checkpoint.

### Task 9: Stage 1 Review And Authorized Native Acceptance

**Files:**
- Create: `docs/plans/2026-10-30-release-workflow-execution.md`
- Modify: `packaging/README.md` as needed from real runtime findings

**Step 1: Record local results and request review.** Distinguish successful
local tests from platform checks awaiting a native run. If implementation
changes Rust, run all required `AGENTS.md` Rust gates through Devenv. Do
not run the full Rust suite solely for workflow/documentation edits.
Fetch/merge any new main changes and check mergeability before review.

**Step 2: After review/merge and fresh owner permission, run on main.**
Have the owner provision Windows secrets and create/push a tag that
includes the workflow and agrees with the manifests. Then, only when
authorized, dispatch:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0
```

Use the owner's actual agreed tag if different; never create or push a tag
without authorization. A failed native run becomes follow-on work per the
repository's native CI policy. Dispatch of Build remains separately
authorized, not implied by permission for Release.

**Step 3: Inspect a real draft and downloads.** Confirm all four jobs,
archive layouts, checksums, source identity, complete notices, and signing
checks. Launch the desktop manually on macOS, Windows, and Linux Wayland;
exercise the packaged CLI on each target. Record whether each architecture
was actually run. Verify a rerun repairs a draft and a published release
is rejected using a non-production test setup or the fake-boundary tests;
do not publish or damage a real release merely to test refusal behavior.

## Stage 2 — Installation Channels

Proceed after Stage 1's local review and release contract are stable.
Native findings may require revising package runtime details before
channel delivery. Stage 2 changes require their own review and fresh
permission for native runs.

### Task 10: Generate A Homebrew Cask For A Personal Tap

**Files:**
- Create: `packaging/homebrew/manyhands.rb.in`
- Create: `scripts/release/homebrew.py`
- Create: `scripts/release/tests/test_homebrew.py`
- Modify: `scripts/release/assemble.py`
- Modify: `packaging/README.md`
- External, only with owner authorization: `vhodges/homebrew-tap/Casks/m/manyhands.rb`

**Step 1: Add a generation test using actual archive layout.** Verify
version, SHA-256, published download URL, ARM64-only restriction, app path,
CLI path, and actual minimum macOS requirement. Verify that unresolved
template variables fail rather than yielding an unusable cask.

**Step 2: Implement cask rendering.** Use `app` plus `binary` stanzas that
reference the exact directory paths inside the macOS archive. Include
appropriate Git dependency guidance and signing limitations without
disabling quarantine or Gatekeeper. Do not claim official Homebrew cask
acceptance. Emit `manyhands.rb` as an additional draft release asset and
include it in the checksum manifest; update assembly's owned-asset set.

**Step 3: Validate locally and on macOS.** Run generation tests, Ruby
syntax validation, and relevant `brew style`/cask audit checks. After
publication and with permission, install from the personal tap, confirm
the app and CLI install, and check uninstall preserves user data. Request
the intended tap name/access before creating or modifying any external
repository. Initially document a manual tap update; no cross-repository
write token is required in Release.

### Task 11: Add The Windows Installer

**Files:**
- Create: `packaging/windows/manyhands.iss`
- Create: `packaging/windows/manyhands.ico`
- Create: `scripts/release/windows-installer.ps1`
- Modify: `src/main.rs`
- Modify: `scripts/release/windows-sign.ps1`
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/release/assemble.py`
- Modify: `scripts/release/tests/test_assemble.py`
- Modify: `packaging/README.md`

**Step 1: Implement the GUI subsystem boundary.** Add only to the desktop
entry point:

```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
```

The CLI entry point remains a console program. Verify release PE
subsystems on Windows instead of testing this attribute with a mirrored
source-text assertion. Run the required Rust gates below for this change.

**Step 2: Generate the icon and installer definition.** Derive the `.ico`
from the committed brand PNG. Pin the Inno Setup compiler version during
implementation and record installer/tool hashes where downloaded. Use a
single stable `AppId`, current-user install root, no forced elevation,
the existing staged executables and notices, and a desktop shortcut in
the Start menu. Do not offer to install a trust certificate.

**Step 3: Implement safe PATH and upgrade behavior.** PATH integration is
an opt-in task that edits only the user's PATH and removes only the
entry it owns on uninstall. Cover duplicates, pre-existing entries, and
paths containing spaces. Use full SemVer precedence for version checks,
including prerelease ordering; store the installed full version in an
app-owned registry value. Allow reinstall of the same version, reject
downgrades, and preserve user databases, keys, settings, and repositories.
Numeric file version is `MAJOR.MINOR.PATCH.0`; display version is full
SemVer. Provide an explicit diagnostic if numeric components exceed
Windows version-field bounds.

**Step 4: Build/sign the final installer.** Compile from the already
verified/signed payload, sign and timestamp the installer with the same
identity, and verify it. Name it
`manyhands-v<VERSION>-x86_64-pc-windows-msvc-setup.exe`. Upload alongside
the portable ZIP; require it in Stage 2 assembly and checksum tests.

**Step 5: Verify install/upgrade/remove on Windows.** Use a disposable
user/VM and exercise interactive and silent installation, app launch,
CLI discovery with PATH enabled/disabled, reinstall, stable/prerelease
upgrade ordering, rejected downgrade, and uninstall. Verify disclosures
are readable in the install directory and user data is retained.
Record these results; Linux tests cannot validate Inno Setup behavior.

### Task 12: Add Source-Building Linux Nix Packages

**Files:**
- Create: `flake.nix`
- Create: `flake.lock`
- Create: `packaging/nix/package.nix`
- Create: `packaging/linux/manyhands.desktop`
- Modify: `packaging/README.md`
- Modify: `README.md`

**Step 1: Define public outputs.** Use a pinned nixpkgs input and a
small two-system mapping for `x86_64-linux` and `aarch64-linux`. Export
`default` (desktop and CLI) and `manyhands-cli` (featureless CLI). Pin a
nixpkgs/toolchain combination matching the release Rust version and
assert/document that relationship. Keep Devenv inputs and shell behavior
intact. Commit `flake.lock` with `flake.nix` when authorized.

**Step 2: Implement the CLI derivation first.** Use `buildRustPackage`,
`cargoLock.lockFile`, explicit `--no-default-features --bin manyhands-cli`,
and only its native dependencies. Install `bin/manyhands-cli`, MIT licence,
and appropriate complete notices. Include any licence generation inputs
in the Nix dependency closure; builds must not fetch tool binaries or
licence text over the network in the sandbox. Do not include GPUI,
desktop editor crates, fonts, or graphics runtime in the CLI-only closure.

**Step 3: Implement the desktop package.** Build both front ends separately
as in Stage 1, install icons/desktop entry and full documentation, and wrap
the desktop with the same GPUI runtime-library set required in
`devenv.nix`. Include dynamically loaded Wayland/Vulkan libraries, not
only ELF-linked libraries. Keep the CLI independent of the desktop
wrapper. Define the source filter explicitly so `.manyhands`,
`.worktrees`, local build output, and private development state do not
become source dependencies. Add all needed new files to Git's index
before flake evaluation because Git-based flakes omit untracked files.

**Step 4: Check evaluation, builds, and the real runtime boundary.**

```sh
nix flake check --no-build
nix build .#manyhands-cli --out-link /tmp/opencode/manyhands-cli-release
/tmp/opencode/manyhands-cli-release/bin/manyhands-cli --help
nix build .#default --out-link /tmp/opencode/manyhands-desktop-release
nix path-info -r .#manyhands-cli
```

Verify the CLI closure excludes GPUI/graphics dependencies; inspect the
desktop's runtime wrapper and run on NixOS/Wayland without Devenv's
`LD_LIBRARY_PATH`. Evaluate the ARM64 outputs locally and build/run them
on authorized ARM64 hardware or a configured builder; do not report an
ARM64 build from evaluation alone. Use an empty config/data home for
smoke tests. Add flake checks for packaged CLI help and required installed
licence files without duplicating the entire Rust test suite.

**Step 5: Document consumer installation.** Include tag-pinned examples
for `nix run`, `nix profile install`, and NixOS/Home Manager package
selection, distinguishing CLI-only installation. Test the exact examples
against a published tag once the flake has shipped. No binary cache or
automatic nixpkgs submission is required initially.

## Final Verification And Handoff

Run the relevant packaging tests and workflow lint after the last change:

```sh
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v
nix shell nixpkgs#actionlint -c actionlint .github/workflows/release.yml
git diff --check
```

For Rust changes, the repository's standard gates are:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Apply the documented relevant-tests exception only if execution is
actually under `working-on-a-ticket`. Run no native workflow without
fresh approval. Record checks as passed, failed, or pending with actual
evidence. Request code review, inspect the intended diff, and leave the
branch review-ready. Commit, push, merge, tag, external tap changes, and
publication each follow the owner's authorization; the workflow itself
never publishes a release.
