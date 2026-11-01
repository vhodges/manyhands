# Release packaging

The manual [Release workflow](../.github/workflows/release.yml) builds the
four native archive targets and assembles an unpublished GitHub Release.
The Build workflow supplies the separate native build/test checks.
Stage 2 now generates a personal Homebrew cask and signed Windows Inno Setup
installer alongside the archives. The root Nix flake separately builds Linux
consumer packages from source. Native channel acceptance remains.

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

With the Homebrew and installer checkpoints, the draft has eight workflow-owned downloads:

- Two Linux `.tar.gz` archives (x86_64 and ARM64).
- One Apple Silicon macOS ZIP with `Manyhands.app` and the CLI.
- One Windows x86_64 ZIP with both executables.
- `manyhands-v<VERSION>-x86_64-pc-windows-msvc-setup.exe`: per-user installer.
- `manyhands.rb`: a version/checksum-specific personal Homebrew cask.
- `release.json`: immutable source identity, target/archive and channel descriptors.
- `SHA256SUMS`: checksums for the four archives, installer, cask, and `release.json`.

Detailed compiler, signing, and runtime data lives in each archive's own
`release.json`. The release-level source manifest is uploaded first and
retained unchanged on same-source reruns. This lets recovery identify the
draft even when its owner has completely rewritten the release notes.
Do not delete or edit that source asset. The initial notes also contain a
hidden source marker; if both identity records are removed, preflight
cannot safely identify the draft and fails before mutation.

Reruns preserve owner-written notes and unrelated attachments. After all
new candidates are verified, assembly clears any stale `SHA256SUMS`,
replaces only the four owned archives, installer, and generated cask, and uploads fresh
checksums last. The cask's archive hash may change after a native rebuild;
only source-determined asset names/targets belong in the immutable manifest.
An interrupted upload leaves a repairable draft with no completion checksum
file. A source mismatch, moved tag, incorrect prerelease classification,
or published release fails rather than updating that release.

GitHub uploads are not transactional with manual publication. The helper
checks draft state before each mutation and stops when it observes
publication, but cannot atomically lock the release UI. Publish only after
the workflow has completed successfully and the downloads have been
reviewed. The workflow itself never publishes a draft.

The new asset contract applies to a new source version/tag containing this
checkpoint, not the existing `v0.1.6` draft. Never retrofit or clobber an
existing immutable source manifest to add channels. The observed initial
draft-visibility failure still requires an assembly-only failed-job rerun;
automatic post-create retries are deliberately deferred during Stage 2.

## Homebrew personal cask

Assembly renders `manyhands.rb` from the verified Apple Silicon macOS ZIP,
using its SHA-256, full version, source repository, and actual deployment
metadata. It checks that the app's `LSMinimumSystemVersion` agrees with the
archive manifest. Current Homebrew uses named macOS major releases in
`depends_on`; an unknown major or a nonzero point-release requirement fails
generation instead of being silently rounded down. Big Sur uses the bare
macOS requirement (Homebrew's own floor); newer releases use named minimums.

The cask moves the versioned directory's `Manyhands.app` into the configured
application directory and links `bin/manyhands-cli` from the retained
versioned Caskroom payload. The sibling `lib/` and disclosures stay beside
the CLI; do not copy the executable out on its own. The app carries its own
disclosures. The cask requires Apple Silicon, does not
alter quarantine/Gatekeeper, and makes no Developer ID/notarization claim.
It defines no user-data removal (`zap`) action; normal uninstall preserves
settings, credentials, repositories, and Manyhands data.

For local generation from an existing matching release input and ZIP:

```sh
devenv shell -- python3 -B scripts/release/homebrew.py --input release-input.json --archive "$MACOS_ARCHIVE" --repo vhodges/manyhands --output /tmp/opencode/manyhands.rb
nix shell nixpkgs#ruby -c ruby -c /tmp/opencode/manyhands.rb
```

`--repo` is the repository hosting the release, **not** the tap. Assembly
generates the cask itself before any draft mutation; native candidates stay
the exact four archives plus inventories. Prebuilt casks injected into the
candidate directory are rejected. No cross-repository write token is needed.

### Manual tap update and native acceptance

The proposed repository is `vhodges/homebrew-tap`, but its name, existence,
and access still need owner confirmation. No external tap is created or
modified by this implementation. After an authorized release run, review
and publish the release manually before updating the tap: draft downloads
are not publicly installable.

After publication, download `manyhands.rb`, `SHA256SUMS`, and the matching
macOS archive, and verify their checksums. With separate tap authorization,
copy the generated file to `Casks/m/manyhands.rb` in the confirmed tap.
On Apple Silicon macOS, set `TAP` to that tap's Homebrew name and run:

```sh
brew style --cask Casks/m/manyhands.rb
brew audit --cask --online "$TAP/manyhands"
brew install --cask "$TAP/manyhands"
manyhands-cli --help
open -a Manyhands
brew uninstall --cask "$TAP/manyhands"
```

Validate the CLI **through Homebrew's symlink**, with inherited `DYLD_*`
overrides unset, to check real relative-library loading. Confirm installed
app/CLI notices are readable, normal macOS approval/security behavior is
accurately documented, and uninstall preserves user data. These native
checks, `brew style`, and the cask audit are pending; Linux generation,
synthetic archive tests, and Ruby syntax do not establish Homebrew acceptance.

## Windows per-user installer

The Inno Setup EXE installs the same files as the verified portable ZIP into
`%LOCALAPPDATA%\Programs\Manyhands`, plus the brand icon, Inno licence and
managed-file manifest.
It requires no elevation, adds a current-user Start menu shortcut, and keeps
the stable AppId **`io.github.vhodges.manyhands`**. The initial target is native
x86_64 Windows, with the same Windows 10 1703/system ICU floor as the ZIP.
The desktop's release build uses the GUI PE subsystem; the featureless CLI
keeps the console subsystem. Both are checked from actual native PE headers.
Windows release desktop startup failures and panics display a fixed-text
native error dialog without exposing panic payloads or paths. Existing stderr
diagnostics/default panic-hook behavior remain; the CLI and debug/non-Windows
behavior are unchanged. Injected-dialog tests on Linux do not establish
native Windows dialog linkage or visibility.

User PATH integration is an **opt-in** `clipath` task (off for a new install).
Reinstall/upgrade retains Inno's previous task selection. The installer reads
only the registry's user PATH, never copies the process's merged system/user
PATH, and preserves unrelated entries and REG_SZ/REG_EXPAND_SZ type. Quoted,
case-varied, trailing-backslash, and environment-expanded existing entries
are not claimed. Uninstall or deselecting the task removes at most one exact
entry recorded as installer-owned. PATH changes apply to **new terminals**.
Directories containing a semicolon or quote cannot opt into PATH integration.

Installer-owned metadata lives at `HKCU\Software\Manyhands\Installer` in the
64-bit view. Full SemVer precedence rejects downgrades, including stable to
older prerelease transitions, and permits same-version reinstall. Numeric
Windows file metadata is `MAJOR.MINOR.PATCH.0`; fields above 65535 fail before
compilation. Preparation is read-only. A `PendingVersion` floor is written at
the actual install-start boundary, before copying, and cleared only after
completion; a checkpoint failure prevents copying. Cancelling preparation
does not establish a new floor. After an interrupted install or
upgrade, rerun the **same or newer** installer; an older payload cannot be
used to overwrite partially upgraded files. Once files may have changed,
cancellation retains the floor: Inno rollback does not generally restore
overwritten pre-existing files. Invalid/unreadable metadata
fails closed. Changing the installation directory requires uninstall first.
An incomplete post-install operation returns exit code **42**, even when
Inno has handled/suppressed its error dialog; repair with the same/newer
installer rather than treating it as a successful silent install.

After successful copying and before completion, the installer removes stale
files only within its owned `bin` and `licenses` trees. The compiler embeds
the incoming managed-file list derived from the exact verified ZIP plus the
Inno licence and manifest itself; it does not trust an older installed list.
Before any pruning, it validates the complete list, expected files, both owned
trees and reparse-free ancestors. It also checks every expected file's SHA-256
against a separate compiler-embedded digest table derived from the verified
ZIP and exact installer additions, including the installed manifest itself.
The digest table is not installed; expectations are cached at startup and
never reloaded from mutable installed or temporary metadata. Old/wrong bytes
left by Inno's Keep existing/Skip paths cannot authorize pruning or completion.
Unsafe paths, missing/mismatched files or deletion
failures leave installation incomplete, return 42 and retain the repair floor.
No pre-install tree deletion is performed. Root/user files are not pruned;
empty obsolete directories may remain. Treat `bin` and `licenses` as
installer-owned, not as locations for personal files.

Normal uninstall removes tracked installation files, its shortcut, owned
PATH addition, and installer metadata only. It does not delete settings,
credentials, project repositories, or Manyhands user data. There is no
certificate-import action; self-signing still does not establish public
publisher trust or remove SmartScreen warnings. Local Git operations use the
linked libgit2 library; a client-side Git executable is not required.

### Build and signing boundary

`windows/inno-setup.json` pins the upstream **Inno Setup 7.1.1 x64** compiler
installer and SHA-256. The Windows job verifies the download before running
it and checks `ISCC --version`; it never uses a drifting preinstalled compiler.
The upstream licence is retained as `licenses/installer/INNO-LICENSE.txt` in
the installation. See [Inno's licence](https://github.com/jrsoftware/issrc/blob/is-7_1_1/license.txt)
and [commercial-use FAQ](https://jrsoftware.org/isorder.php#required).

After extracted ZIP verification, the job imports the existing signing
identity again inside the guarded cleanup scope. `installer.py` compares
**every payload file** with the verified ZIP, before and after compilation.
Inno's mandatory signing callback signs/timestamps/verifies the embedded
uninstaller, setup self-copies, and final EXE with the same publisher identity.
The parent independently verifies the final EXE. No signing secret is passed
on the compiler command line, and no unsigned/untimestamped fallback exists.
The internal installer inventory binds its bytes to the source identity,
exact Windows ZIP hash, signer, and pinned compiler. Assembly requires and
validates both that installer and its inventory before draft mutation.
ISCC has an explicit **1,800-second** deadline, including mandatory signing
and verification, with **two signing retries / 2,000 ms delay**. The installer
workflow step allows **45 minutes**, leaving 15 minutes for surrounding
verification, CLI smoke testing and cleanup. Ordinary callback/native tools
retain 180-second deadlines; application signing and extracted-verification
steps retain their ten-minute limits. No timeout or retry permits unsigned or
untimestamped output. See [signing deadlines](windows/SIGNING.md#progress-and-deadlines).

The `.ico` is derived from the committed 1024 PNG with ImageMagick 7.1.2-34:

```sh
nix shell nixpkgs#imagemagick -c magick packaging/icons/manyhands-1024.png -define icon:auto-resize=256,128,64,48,32,24,16 packaging/windows/manyhands.ico
```

### Installer checks and remaining acceptance

Python tests validate version/file metadata, source binding, compiler pins,
and assembly/recovery. With FPC available, they compile and run the exact
shared Pascal SemVer/PATH policy, not a Python rewrite:

```sh
nix shell nixpkgs#ruby nixpkgs#fpc -c devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v
```

The Windows helper step runs the same policy/cases through Inno's real Pascal
Script engine before release secrets. Its registry adapter fixture uses a
fresh, guarded HKCU GUID subtree for version/ownership/environment tests,
cleans it in `finally`, and exits before installation. It never edits the
runner's actual PATH. A separate no-payload completion fixture exercises
Inno's real post-install event boundary and requires exit 42 on failure,
0 on success; it writes only its owned result file. These fixtures and the
production definition were also checked locally under Wine; this is not
native Windows acceptance. The production definition was compiled with
output disabled, so that check did not produce or sign an installer.

Additional opt-in pinned-engine tests exercise the actual lifecycle, not just
policy functions: preparation cancellation, fatal checkpoint failures before
copying, mid-install cancellation with overwritten bytes retained, malformed
managed lists, missing expected files, reparse points, pruning failures, and
successful fresh/upgrade completion. Additional cases cover old/wrong bytes
at existing paths and forged installed/temporary digest metadata. A signing-
failure fixture proves three bounded attempts without unsigned output. The
complete local suite passed **255 tests with no skips**, including Ruby, FPC
and all five Inno-engine tests after the content-validation correction. To reproduce
the engine subset on Linux with an already verified compiler and a fresh,
owned Wine prefix:

```sh
nix shell nixpkgs#wineWow64Packages.stable nixpkgs#xvfb-run --command \
  env INNO_COMPILER=/path/to/verified/ISCC.exe \
  MANYHANDS_INNO_WINEPREFIX=/tmp/opencode/manyhands-owned-fresh-wine-prefix \
  WINEDLLOVERRIDES=mscoree,mshtml= \
  xvfb-run -a python3 -B scripts/release/tests/test_installer_engine.py
```

On an authorized disposable native Windows machine, still exercise interactive
and silent install, app launch, CLI PATH enabled/disabled, same-version reinstall,
stable/prerelease upgrade, rejected downgrade, interrupted-upgrade repair,
uninstall, readable installed disclosures, and retention of user data and
pre-existing PATH entries. Verify the installed uninstaller's signature too.
For silent operation, Inno supports `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`;
`/TASKS=clipath` opts into PATH and `/TASKS=` selects no optional tasks.
No new release workflow run or native signing/install acceptance has occurred
for this checkpoint.

## Linux Nix source packages

The consumer flake and lock live at the project root; supporting expressions
and the [installation/runtime guide](nix/README.md) live in `packaging/nix/`.
They do not modify `devenv.nix`, `devenv.yaml`, or `devenv.lock`.
For `x86_64-linux` and `aarch64-linux`, `default`/`manyhands` provides desktop
plus a separately built featureless CLI; `manyhands-cli` provides just the
CLI, without desktop fonts or graphics runtime libraries. Do not install
both variants into one profile: their CLI executable paths overlap.

Nixpkgs revision `c7def046b9a883d46974757852106483d741586f` supplies matching
Rust/Cargo **1.98.1**. `packaging/nix/cargo-about.nix` separately pins **0.9.3** and its
vendor hash. Evaluation asserts agreement with `tool-versions.env`.
Cargo dependencies and original MPL/libgit2 `.crate` archives are fixed
inputs verified against `Cargo.lock`; notice generation runs frozen/offline
inside the sandbox, without developer Cargo caches or remote licence lookup.
Reviewed upstream licence originals absent from published crates are retained
as checksum-pinned offline supplements; unresolved canonical copyright
placeholders fail notice generation. The vendor lockfile is an isolated,
content-only store input, so its symlink cannot retain the unfiltered flake
snapshot. Source-policy checks also inspect transitive vendor references.
Installed terms/source archives are under `share/doc/manyhands`. The CLI's
disclosures omit the desktop graph and fonts; separate Nix runtime library
packages retain their own pinned Nixpkgs sources and terms.

The desktop wrapper supplies GPUI's dynamic loader paths; the CLI is installed
directly without a wrapper. Neither package includes a runtime Git executable;
local operations and SSH transport use linked libraries. The source filter
excludes private project state, worktrees, build outputs, and hidden files.
Flake checks cover these boundaries, CLI help/repository creation with an
empty PATH and Git-/graphics-free closure, desktop installed
files, and required dynamic libraries. Loader success does not prove a
Wayland window or working host graphics driver, and ARM64 evaluation alone
does not establish an ARM64 build. Those acceptance checks and exact remote
tag examples remain separate from local evaluation/build verification.

No Nix binary cache, automatic Nixpkgs submission, release-asset upload, or
signing-identity change is introduced. These packages build source rather
than consuming the signed native archives. Existing `v0.1.6` predates the
flake; consumer tag-pinned testing needs a later published source tag.

**Implementation checkpoint:** shared-disclosure remediation now passes
offline generation and original-file/source integrity checks for all four
targets. Six exact release declarations have the owner's dated, checksum-
bound grant-by-reference approval; unknown cases still fail closed. Two
upstream licensing uncertainties remain documented: whether KDE's LGPL
protocol definitions cover generated bindings, and the objc2 project's
Apple SDK caveat (the same caveat across 18 dependencies). These are not
established violations or prerequisites to reviewing this packaging PR;
preserving notices is not a claim to resolve every possible binary-
distribution obligation. See the Task 12
[execution record](../docs/plans/2026-10-30-release-workflow-execution.md)
for historical package-build/runtime evidence and remaining distribution
decisions. The current [review-fix execution record](../docs/plans/2026-11-01-packaging-review-fixes-execution.md)
records rebuilt x86_64 CLI/desktop packages, all current-system flake checks,
installed original disclosures and empty-PATH repository creation with both
installed CLIs. The owner confirmed an isolated NixOS/COSMIC Wayland launch
and correct text rendering on the earlier pre-review build; that manual window
check was not repeated for the review-fix outputs. ARM64 executable/runtime
and later tagged-consumer checks remain; evaluation/loader success alone
does not establish those outcomes.

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
Both workflows export `OPENSSL_DIR` from `brew --prefix openssl@3` before
building. Installing OpenSSL 3 alone is insufficient: the locked
`openssl-sys` also detects preinstalled OpenSSL 4 and otherwise prefers it.
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

Both modes require an elevated GitHub-hosted Windows runner. Only the public
certificate is temporarily trusted in `LocalMachine\Root`; its private signing
identity remains in `CurrentUser\My`. Existing trust is preserved and only
owned additions are removed. The native helper fixture runs before release
secrets. Application signing and extracted-verification steps have ten-minute
limits; the installer step has a 45-minute backstop around its explicit
1,800-second compiler deadline, as detailed above.
