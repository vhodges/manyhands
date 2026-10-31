# Release Workflow Execution Record

Date: 2026-10-30

## First Checkpoint — Tasks 1–3

Branch: `release-workflow`, worktree `.worktrees/release-workflow`.
Fetched origin and merged `origin/main` before implementation; it was
already up to date at `48ffc058`. Work is uncommitted and has not been
pushed. No workflow has been dispatched and no release/tag/certificate
or external repository has been created.

### Implemented

1. **Release identity:** existing lightweight/annotated tag resolution,
   strict v-prefixed SemVer parsing, tagged `Cargo.toml`/`Cargo.lock`
   validation, JSON handoff, and GitHub scalar outputs. Missing tags,
   branch-only references, inconsistent versions, and malformed input
   fail without creating a new manifest.
2. **Licensing/documents:** pinned cargo-about 0.9.3, separate normal CLI
   and desktop graphs, HTML-escaped complete licence notices, nested
   upstream/native/asset notices, IBM Plex Sans provenance and OFL, and
   original source archives for selected MPL-covered dependencies plus
   libgit2. Source archives are checked against Cargo.lock SHA-256 hashes.
3. **Shared packaging:** target-specific staging, macOS app resource
   disclosures, strict README rendering, tar/ZIP creation, safe extraction,
   package identity/disclosure/source verification, and extracted CLI
   smoke tests with an isolated home and loader-path cleanup. Pending
   Windows/macOS signing cannot be archived as a completed payload.

The initial Devenv lacked Python. Added `python3` to its packages and
ignored bytecode caches. No Rust source, Cargo dependencies, or Devenv
inputs/lockfile changed.

### Verification Evidence

```text
devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v
Ran 39 tests ... OK
```

Tests include temporary Git repositories; tag and version errors; nested
notices; missing/corrupt source archives; absent binaries/licences;
private signing material; tar execution bits; Linux and Windows archive
round trips; macOS AppleDouble handling; path/link rejection; malformed
inventory input; CLI smoke failure; and the actual stage/archive/verify
command entry points using synthetic executable fixtures.

Generated real dependency notice bundles successfully for:

| Target | Output |
| --- | --- |
| `x86_64-unknown-linux-gnu` | `/tmp/opencode/manyhands-notices-linux` |
| `aarch64-unknown-linux-gnu` | `/tmp/opencode/manyhands-notices-linux-arm` |
| `aarch64-apple-darwin` | `/tmp/opencode/manyhands-notices-macos` |
| `x86_64-pc-windows-msvc` | `/tmp/opencode/manyhands-notices-windows` |

Manually inspected generated IBM copyright/OFL, full MIT/MPL texts,
libgit2's COPYING/linking exception, and Lucide/Feather icon terms. The
Linux bundle contains original `option-ext-0.2.0.crate` and
`libgit2-sys-0.18.8+1.9.7.crate` source archives. Normal dependency
notice generation required no crate-specific licence-expression override.

Checked upstream Rust 1.98.1 distribution metadata: all four native
target packages are marked available. Pinned that version and cargo-about
in `packaging/tool-versions.env`.

Tracked and new-file whitespace checks passed. Rust build/test gates are
not applicable to this batch's Python, Nix package-list, and documentation
changes. No native compilation, signature verification, or GUI launch is
claimed from these results.

### Observations And Refinements

- cargo-about 0.9.3 requires `--features cli` at installation; a plain
  `cargo install` succeeds without installing an executable. The documented
  installation command includes the feature.
- Crate metadata alone omits nested native/asset notices. Preserve original
  nested files and source archives in addition to generated Rust notices.
- Concurrent Devenv invocations in one worktree raced on generated
  `load-exports` state and timed out. Repeating the notice generation
  sequentially completed successfully. Avoid concurrent Devenv shell
  invocations sharing that state.
- Windows ZIP fixtures verify structure only; dummy certificate/signing
  metadata in tests is not a cryptographic signature test.
- New output directories are required. Helpers do not overwrite arbitrary
  existing staging/extraction/archive outputs.

## First Checkpoint Handoff (Completed Below)

After owner feedback, implement Tasks 4–6: the Linux runtime audit, macOS
bundle metadata/icon/linkage and ad-hoc signing, and persistent Windows
self-signing/provisioning instructions. Native steps must complete the
manifest's signing state, keep the app's resource manifest in sync, and
verify extracted signatures. Follow with Tasks 7–8 for draft lifecycle and
the manual workflow. Windows secrets remain an owner-provisioned input;
native dispatch still needs fresh authorization after merge.

## Second Checkpoint — Tasks 4–6

The owner approved the first checkpoint in conversation. Implemented the
next three tasks on the same branch/worktree. No commit, push, workflow
dispatch, tag/release creation, or real signing identity provisioning has
occurred. Fetched/merged main again before this batch and before this
checkpoint; main remained at `48ffc058`. `git merge-tree --write-tree HEAD
origin/main` also succeeded (implementation is still uncommitted).

### Implemented

- **Linux:** ELF architecture/loader/RPATH/glibc/linkage auditing, explicit
  Ubuntu 24.04 runtime-package and dynamic-library lists, metadata/README
  updates, and a Docker-based isolated runtime check. The container checks
  both binaries' linkage, loads the declared dynamic libraries, and runs
  the extracted CLI without a development environment. Missing input files
  are rejected before Docker can create bind-mount paths.
- **macOS:** generated 1024-square icon (resvg 0.48.1), app plist with stable
  `io.github.vhodges.manyhands` identity, minimum-OS derivation across
  packaged Mach-O files, reviewed OpenSSL 3 relocation into app/CLI runtime
  layouts, original native notices, inside-out ad-hoc signing, and native
  extracted verification. The public README/manifest and app resource
  copies are refreshed before sealing. Failed sealing leaves signing
  marked pending.
- **Windows:** import/header auditing with MSVC tools and an exact reviewed
  OS/API-set list, static-CRT build policy, persistent PFX/secret support,
  SHA-256 Authenticode/RFC 3161 timestamping, exact signer/timestamp checks,
  temporary current-user trust, owned private-key/file/certificate cleanup,
  public certificate/fingerprint documentation, and public-only extracted
  verification. Packaged CLI tests now strip developer-tool PATH entries.

The Windows workflow must set `RUSTFLAGS=-C target-feature=+crt-static`
for both builds. The initial policy forbids non-system module graphs;
unknown modules need explicit review rather than automatic runner copying.
The macOS initial external-library policy covers Homebrew OpenSSL 3 only.

### Local Verification

Built both actual release binaries from the worktree through Devenv,
reusing the primary checkout's ignored target cache:

```sh
CARGO_TARGET_DIR=/home/vhodges/work/src/manyhands/target devenv shell -- cargo build --locked --release --no-default-features --bin manyhands-cli
CARGO_TARGET_DIR=/home/vhodges/work/src/manyhands/target devenv shell -- cargo build --locked --release --no-default-features --features desktop --bin manyhands
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

All three commands passed. Inspected both binaries' dynamic sections and
checked the real desktop binary through the ELF policy: it was correctly
rejected as an Ubuntu release because its loader/RPATHs are Nix-specific.
These local builds are development outputs, not distribution assets.

Final local checks:

| Check | Result |
| --- | --- |
| Python packaging/policy suite through Devenv | **60 tests passed** |
| PowerShell import-policy tests (PowerShell 7.6.6 via Nix) | Passed |
| PowerShell parser: signing script, module, and policy tests | Passed |
| ShellCheck 0.11.0: both native shell entry points | Passed |
| `bash -n` on the Ubuntu container setup script | Passed |
| Generated PNG visual inspection | Correct 1024-square brand artwork |

Python tests cover metadata/versions, dependency relocation orchestration,
unknown external libraries, ELF baseline failures, missing Docker inputs,
certificate-fingerprint metadata, static-CRT policy, Windows environment
cleanup, and archive handling. macOS command-boundary fixtures do not run
real Mach-O rewriting or codesign. PowerShell policy tests run without
accessing a Windows certificate store. Real signature/timestamp validation
is not claimed from Linux-hosted checks.

### Code Review

Requested a read-only review of the native helpers. Two P2 findings were
reproduced with failing regressions and fixed:

1. Accept the exact `__MACOSX/._<root>` AppleDouble root sidecar produced by
   metadata-preserving ZIP tools, while retaining path restrictions.
2. Require an exact reviewed Windows OS/API-set name as well as ordinary
   module presence in System32. A vendor DLL in System32 no longer passes.
   Dumpbin parsing also captures non-`.dll` import names such as
   `WINSPOOL.DRV` instead of silently dropping them.

The targeted re-review confirmed both findings resolved and reported no
remaining concrete issues in that recheck. Non-system dependency graphs
are prohibited, so OS-managed transitive dependencies remain Windows'
responsibility.

### Native Acceptance Still Pending

- Ubuntu native archives and isolated runtime checks on x86_64 and ARM64.
- Real macOS `sips`/`iconutil`/Mach-O relocation/codesign and extracted
  signature checks, plus Finder launch/Gatekeeper behavior.
- Real Windows static-CRT builds, SignTool/timestamp/WinTrust checks,
  certificate-store cleanup, and extracted execution on a native runner.
- GUI launches on Windows, macOS, and Linux Wayland.

Those run after merge with fresh owner authorization. Windows signing
secrets still need owner provisioning using `packaging/windows/SIGNING.md`.
No Rust source or Cargo dependency/lockfile changed in this batch; the
full Rust source-change gates are not required for the new packaging code.

## Second Checkpoint Handoff (Completed Below)

After owner feedback, implement draft creation/recovery/checksums and the
manual `Release` workflow. Wire all three native helpers into each matrix
job before artifact handoff, retain the CLI dependency-boundary check,
set Windows static-CRT flags, and preserve the independent Build workflow.
Task 9's real native acceptance and Stage 2's installation channels remain
separate follow-on checkpoints.

## Stage 1 Final Checkpoint — Tasks 7–8 And Local Handoff

The owner approved the second checkpoint. The manual Release workflow and
draft assembly are now implemented on `release-workflow`; all work remains
uncommitted in `.worktrees/release-workflow`. Native acceptance is still
post-merge and requires fresh owner authorization. No workflow was
dispatched, real release/tag created, signing secrets provisioned, or
GitHub data mutated during implementation.

### Workflow

`.github/workflows/release.yml` has only `workflow_dispatch`, with a
required existing tag. Preparation requires main invocation, an already
merged source revision, matching tagged Cargo manifests, and remote
tag/release preflight. It emits a compact source manifest. Every later job
checks out and verifies that exact SHA without resolving the tag anew.

The four native matrix rows build featureless CLI and production desktop
separately. They generate notices, run the platform runtime/signing
helpers, archive, extract, smoke-test the CLI, perform extracted native
verification, and hand off only successful outputs. Windows secrets are
scoped to signing, not extracted verification; Windows builds set static
CRT flags. Linux runs the isolated Ubuntu check. Development-only desktop
features are excluded. Only the final assembly job has `contents: write`.

### Draft Assembly And Recovery

Added archive size/SHA-256 inventories and complete candidate validation,
including comparison with each extracted archive manifest. `gh` supplies
all GitHub operations. A read-only 404 probe confirmed its `--include`
HTTP response format; tests distinguish explicit absence from API/auth or
network failures. Annotated tags are peeled to commits and rechecked.

Assembly produces six owned assets: four archives, immutable source-level
`release.json`, and `SHA256SUMS`. The source asset identifies the revision
and target/archive/signing descriptors; detailed per-build information
remains inside each archive. Upload the source anchor first, retain it on
reruns, clear stale checksums before replacing archives, and upload new
checksums last. Verify final uploaded states/sizes and available GitHub
digests. Never publish automatically or modify a published release.

Owner-written notes and unrelated attachments are preserved. Existing
drafts must match both source identity and prerelease classification.
When notes no longer contain their original source marker, the immutable
source asset proves identity without risking deletion during clobber.
Same-source rebuilds can replace timestamped binaries/archives while the
identity anchor stays unchanged.

### Final Local Verification

| Check | Result |
| --- | --- |
| Full release-helper suite through Devenv | **87 tests passed** |
| actionlint 1.7.12 with ShellCheck available | Passed |
| PowerShell parser on every inline pwsh workflow body | **19 steps passed** |
| Prior native-helper ShellCheck/PowerShell policy checks | Passed in second checkpoint |
| Fetch/merge origin/main | Already up to date at `48ffc058` |
| `git merge-tree --write-tree HEAD origin/main` | Succeeded; source remains uncommitted |

The added tests exercise real small four-target archive fixtures and an
in-memory GitHub boundary. They cover API error classification, exact draft
creation flags, tag peeling, missing/duplicate/corrupt candidates, mismatched
inventories, source/prerelease mismatches, interrupted clobber semantics,
same-source rebuilds, preserved notes/attachments, and publication observed
mid-run. They do not create real GitHub releases or validate native code
signatures on Linux. No Rust source/dependency changes were made in this
batch; broad Rust source-change gates were not rerun.

### Final Code Review

Requested review of workflow/assembly integration. Reproduced and fixed:

1. Existing draft prerelease flags were not checked. Both incorrect flag
   directions now fail preflight before mutation.
2. Clobbering the only source manifest after owner notes were rewritten
   could make interrupted runs unrecoverable. Source identity is now a
   deterministic immutable asset, established first and never replaced.
   The fake boundary now deletes an old asset before simulating upload
   failure, matching real `gh --clobber` behavior.
3. Null/list nested handoff metadata produced tracebacks. Explicit object
   validation now reports ordinary release-validation errors.

The targeted re-review confirmed all three findings resolved with no
remaining concrete related defect; its 23 assembly tests passed. The
subsequent full parent-run suite passed all 87 tests.

### Handoff

Local implementation and review for Stage 1 are complete. Preserve the
worktree and obtain explicit commit/push/merge instructions. After merge,
the owner provisions Windows signing secrets, creates a matching merged
tag, and authorizes the first native Release run. Inspect real downloads,
signature checks, runtime checks, and desktop launches under Task 9; any
post-merge native failures are follow-on work under repository policy.
Homebrew, Inno Setup, and Nix flake delivery remain Stage 2.

## Integration Authorization

The owner selected committing Stage 1 and merging it into local `main`.
Verify the merged result before removing the completed worktree. Pushing
and native workflow dispatch require their own authorization; native
acceptance and Stage 2 remain follow-on work.

## First Native Trial And Follow-Up Fixes

With owner authorization, pushed main and annotated tag `v0.1.0` at
`055353f8f62ac053638bd64aefbd3b2c14551b81`, provisioned the two persistent
Windows signing secrets, and dispatched Release run
[41139543604](https://github.com/vhodges/manyhands/actions/runs/41139543604).
Signing material is backed up outside the repository; no private values
are recorded here.

### Observed Result

- Tag/version/remote-state preparation passed.
- Both CLI and desktop release builds passed on Windows x86_64 and Linux
  x86_64/ARM64. All three failed later during notice generation.
- cargo-about 0.9.3 explicitly rejected captured stdout because `pwsh`
  appeared in its process ancestry, even though Python captured the pipe.
- macOS never started: GitHub cancelled the job during the scheduled
  macOS 14 image brownout ahead of retirement on November 2, 2026.
- Draft assembly was skipped; no draft assets were produced.

### Follow-Up Scope

The owner approved a separate follow-up on branch `release-ci-fixes` in
`.worktrees/release-ci-fixes`, based on `055353f8`:

1. Write cargo-about JSON and HTML to temporary files with `--output-file`,
   then read them explicitly as UTF-8. Decode remaining Cargo subprocess
   output explicitly as UTF-8 as well.
2. Select Bash for Linux/macOS and PowerShell for Windows. Move the shared
   argument lists and pinned-tool output into a small Python workflow
   adapter; preserve checked child execution, production features, native
   Windows compilation, and all packaging/signature/runtime checks.
3. Move Build and Release to `macos-15`, confirmed ARM64 in GitHub's
   [official runner catalog](https://github.com/actions/runner-images#available-images).
   This avoids the [macOS 14 brownout/retirement](https://github.com/actions/runner-images/issues/13518)
   without switching to a moving `macos-latest` alias.

### Follow-Up Verification

- Baseline: 87 helper tests passed.
- Reproduced the real cargo-about failure locally under PowerShell before
  changing output handling; the new contract regression also failed first.
- After the fix, generated complete real notice bundles for all four
  target graphs under PowerShell using pinned cargo-about 0.9.3. This is a
  cross-target metadata test on Linux, not Windows/macOS native acceptance.
- The first combined invocation timed out during the macOS desktop graph.
  A standalone verbose command and a full macOS rerun then completed.
- The nested local Nix/PowerShell tool environment needed its inherited
  `LD_LIBRARY_PATH` cleared for the smoke command to avoid a Nix library ABI
  collision. No project runtime library configuration was changed.
- Added regressions for UTF-8 file output, shell-neutral paths, production
  Cargo arguments, Windows PowerShell selection, immutable source handoff,
  retained smoke/inventory checks, pinned-tool output validation, and child
  exit-code propagation.
- Scoped read-only code review reported no actionable findings. Its
  complete helper suite passed 97 tests; actionlint passed for both workflows.

At the follow-up review checkpoint these fixes were uncommitted and no
version/tag change or native rerun had been authorized. The owner then
approved committing/merging the fixes, bumping Cargo metadata to `0.1.1`,
pushing main and a new `v0.1.1` tag, and starting one new Release trial.
The existing `v0.1.0` tag remains unchanged. Stage 2 remains separate work.

For the version-only metadata change, check/fmt/clippy passed. An unnecessary
full-suite run first exceeded the command time limit; an extended run then
found the CLI version golden still pinned to `0.1.0`. Both JSON and text
version goldens are updated to `0.1.1`. The owner corrected the verification
scope: use targeted metadata/CLI version checks on a known-compiling tree
instead of repeating full gates merely for a version bump. Remaining
verification follows that direction; the broad suite is not reported as passed.
The targeted CLI version test then passed, as did all 97 release-helper
tests, actionlint for both workflows, and whitespace checks. Integration
and the newly authorized native trial proceed on those scoped checks.

## v0.1.1 Native Trial — Dependency Audit Follow-Up

Merged/pushed the shell and notice fixes, tagged `v0.1.1` at
`676ff1936fc8cc60eb758f873e8883e4b5a51e8d`, and dispatched
[41158651149](https://github.com/vhodges/manyhands/actions/runs/41158651149)
with owner authorization. Both Linux targets completed packaging and
isolated runtime verification successfully. macOS and Windows passed
compilation, notices, and staging, then failed their native dependency
audits; assembly was skipped.

The follow-up branch `release-native-deps` contains these scoped fixes:

- **macOS:** the runner had Homebrew `openssl@3` installed, but the locked
  `openssl-sys 0.9.117` discovery code prefers an installed `openssl@4`
  unless `OPENSSL_DIR` is supplied. The resulting binary imported
  `libssl.4.dylib`, outside the packager's reviewed OpenSSL 3 policy. Both
  workflows now persist `OPENSSL_DIR=$(brew --prefix openssl@3)` before
  cache restoration/building. The crate tracks that environment variable
  for rebuilds; the packager continues to accept only the reviewed family.
- **Windows:** the audit rejected
  `api-ms-win-shcore-scaling-l1-1-1.dll`. Microsoft's official
  [GetDpiForMonitor documentation](https://learn.microsoft.com/en-us/windows/win32/api/shellscalingapi/nf-shellscalingapi-getdpiformonitor)
  lists that exact standard OS forwarding alias. Add only that name to the
  reviewed list and cover it with a regression. Batch rejected imports
  per executable into one diagnostic without weakening any rejection rule.

Verification was restricted to the affected packaging code: the Windows
policy regression failed before the allowlist fix; the aggregate diagnostic
regression also failed before its implementation. Both now pass, alongside
the seven macOS packaging-policy tests, actionlint for both workflows, and
whitespace checks. A scoped read-only review found no actionable issues
and independently checked exact/case-insensitive recognition, rejection
of neighboring unknown contracts and CRT imports, and diagnostic behavior.
No full Rust gates were run for these workflow/policy-only changes.

The fixes are uncommitted pending integration instructions. Cargo versions
and existing tags are unchanged, and no further native run has been
dispatched. The next Release trial needs a matching new source version/tag
containing these fixes; native signing and Windows/macOS runtime acceptance
are still pending.

The owner subsequently approved committing/merging these dependency fixes,
bumping package metadata and CLI version goldens to `0.1.2`, pushing main
and a new `v0.1.2` tag, and starting one new Release trial. Validation stays
scoped to the affected policy/packaging code and the CLI version test;
existing tags are not moved.

## v0.1.2 Native Trial — Windows Desktop OS Components

The authorized `v0.1.2` trial
[41161342157](https://github.com/vhodges/manyhands/actions/runs/41161342157)
completed macOS and both Linux packages, including their native/extracted
verification. Windows reached its dependency audit and reported three
missing reviewed names for `manyhands.exe`: `gdiplus.dll`, `icuuc.dll`, and
`uiautomationcore.dll`. Draft assembly was skipped.

The `release-windows-dlls` follow-up adds only those exact OS DLL names.
Microsoft documents them respectively as GDI+, the OS ICU common C runtime,
and UI Automation; source links are recorded beside the policy entries.
ICU common C APIs were introduced as `icuuc.dll` in Windows 10 version
1703, and that prerequisite is recorded in package runtime metadata without
claiming it as the application's overall minimum supported OS.

The regression reproduced all three rejected imports before the policy
change. It now verifies combined acceptance while the DLLs exist in the
fixture System directory, and individual rejection after each file is
removed. The ordinary-system-file presence check, exact/case-insensitive
name matching, unknown/vendor rejection, and dynamic-CRT rejection remain.
PowerShell policy tests, the four Windows metadata tests, and whitespace
checks passed. Scoped read-only review found no actionable issues and
independently reran those checks. No broad Rust gates were run.

The fix is uncommitted pending integration instructions. Cargo version
remains `0.1.2`; existing tags and signing secrets are unchanged, and no new
native run was dispatched. A subsequent trial needs an approved matching
new source version/tag containing the correction.

The owner subsequently approved committing/merging this Windows correction,
preparing package metadata and CLI version goldens for `0.1.3`, pushing
main and a new `v0.1.3` tag, and starting one new Release trial. Existing
tags and the persistent signing identity are retained. Checks remain scoped
to the Windows policy/metadata and CLI version behavior.
