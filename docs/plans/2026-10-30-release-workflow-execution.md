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

## v0.1.3 Native Trial — Windows Timestamp Transport

The authorized `v0.1.3` trial
[41163081291](https://github.com/vhodges/manyhands/actions/runs/41163081291)
completed both Linux packages and macOS packaging/native verification.
Windows passed its dependency audit and then failed in **Audit and self-sign
Windows executables** (job `124424744848`), using Windows SDK 10.0.26100.0:

```text
SignTool Error: Invalid Timestamp URL: https://timestamp.digicert.com/
```

Draft assembly was skipped. The error concerns the configured timestamp
transport, not evidence of an unusable signing identity. The helper
normalized the configured HTTPS URL and explicitly required HTTPS.
[Microsoft's SignTool examples](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool#examples)
instead document `http://timestamp.digicert.com` with `/tr` and `/td SHA256`.

The owner approved a focused follow-up on `release-windows-timestamp` in
`.worktrees/release-windows-timestamp`, based on `a52273ff`. Fetched origin
and merged `origin/main`; main had not moved. The
[implementation plan](2026-10-31-windows-timestamp-fix.md) records the scope.

### Correction And Regression Evidence

- Changed only the timestamp configuration to the documented HTTP endpoint,
  with a PowerShell validator that accepts only its normalized exact URL.
  HTTPS, other hosts, credentials, nondefault ports, non-root paths,
  queries/fragments, other schemes, relative URLs, and empty/malformed input
  are rejected. The normalized URL is shared by `/tr` and signing metadata.
- Preserved the publisher identity, secrets, SHA-256 file/timestamp digests,
  mandatory native signature/timestamp verification, and certificate cleanup.
  HTTP carries a cryptographically signed RFC 3161 token; it is not an
  untimestamped or unsigned fallback. Updated the signing guide/design.
- Baseline: all four Windows metadata tests and PowerShell import-policy
  tests passed. The new Python configuration regression then failed on the
  HTTPS value; the PowerShell acceptance regression failed on the missing
  endpoint validator before implementation. Both passed after the fix.
- Final focused Python run through Devenv: **14 Windows metadata/configuration
  and workflow-adapter tests passed**. PowerShell 7.6.9 through Devenv passed
  the import-policy and timestamp endpoint tests; it also parsed all three
  affected PowerShell files successfully. The PowerShell process alone had
  its inherited `LD_LIBRARY_PATH` cleared to avoid the known Nix ABI collision;
  the project's runtime library settings were not changed.
- actionlint 1.7.12 with ShellCheck 0.11.0 passed for Build and Release.
  `git diff --check` passed. No Rust source, dependencies, versions, workflow
  matrix, existing tags, signing material, or release assets changed; broad
  Rust gates were not run for this PowerShell/configuration-only correction.
- A scoped read-only review found no actionable correctness, security, or
  coverage issues. Final fetch confirmed main remained at `a52273ff`;
  `git merge-tree --write-tree HEAD origin/main` succeeded. Edits are still
  uncommitted, so that merge check compares the committed branch baseline.

At the review checkpoint, changes were uncommitted pending integration
instructions. The owner then selected committing the correction and merging
it into local `main`, with focused verification repeated on the merged tree.
Push, version/tag preparation, and native workflow dispatch are not included
in that authorization.

Native Windows signing, extracted signature/runtime verification, and draft
assembly still need an independently authorized post-merge trial with a new
matching source version/tag containing this correction. No workflow was
dispatched, no certificate regenerated, and no existing tag moved.

## v0.1.4 Native Trial — Bounded Signing Diagnostics

The owner subsequently authorized pushing the timestamp correction and a
new Release trial, then explicitly approved preparing `v0.1.4`. The fix is
`5600d7d7`, merged as `87d87dea`; version metadata and both CLI version
fixtures were committed as `25a8174d`. The targeted CLI version test and
tag/merged-source validation passed, then main and the new annotated tag
were pushed. Existing tags were not moved. GitHub CLI credential retrieval
blocked dispatch, so the owner started the trial through GitHub's web UI.

[Release run 41233300191](https://github.com/vhodges/manyhands/actions/runs/41233300191)
validated the tag and completed macOS and both Linux packages. Windows job
`124631826355` remained in **Audit and self-sign Windows executables** from
19:24:20 to 20:26:52 UTC on October 31, 2026: **62 minutes 32 seconds**.
The owner cancelled the run. Its supplied signing log showed only invocation,
masked secrets, and cancellation; cleanup named only an orphan `pwsh`.
This does not identify which signing/certificate/audit operation stalled.
Windows packaging/verification and draft assembly did not complete.

The owner approved a diagnostic follow-up in
`.worktrees/release-windows-signing-diagnostics`, based on `25a8174d` after
fetching/merging `origin/main`. See the
[design](2026-10-31-windows-signing-diagnostics-design.md) and
[implementation plan](2026-10-31-windows-signing-diagnostics.md).

### Diagnostic Changes And Local Evidence

- UTC information-stream progress marks PFX loading/import, public export,
  the runtime audit, each executable's signing/timestamping, SignTool and
  Windows Authenticode verification separately, metadata, smoke testing,
  and cleanup. No private data, environment dumps, or argument lists are
  logged; parsed return values remain separate from progress.
- Native tools use .NET argument lists without a shell, concurrent output
  drains, and a 180-second deadline covering execution and output collection.
  A timeout terminates a live process tree, allows a bounded five-second reap,
  and throws through normal signing cleanup. A detached child whose parent
  has already exited cannot be guaranteed terminated through that old handle;
  its open output pipes still cannot defeat the deadline.
- Both Windows signing and extracted-verification steps have a ten-minute
  backstop for in-process certificate cmdlets and cumulative work. Hard
  workflow cancellation can skip `finally`; disposable-runner teardown is
  the isolation backstop. Signature, timestamp, identity, and cleanup failures
  still fail the release without an unsigned/untimestamped fallback.
- Baseline: 14 focused Python tests and PowerShell DLL/endpoint tests passed.
  New process/progress regressions failed before implementation; the two
  workflow-limit assertions and five verification-progress cases also failed
  before their changes. The inherited-pipe timeout diagnostic was separately
  reproduced as missing before its implementation.
- Final local checks: **16 focused Python tests**, **11 real child-process
  cases**, **5 verification-progress cases with platform APIs stubbed**, and
  existing PowerShell DLL/endpoint tests passed through Devenv. PowerShell
  7.6.9 parsed five source/test files and 21 Windows inline workflow bodies.
  actionlint 1.7.12 with ShellCheck 0.11.0 passed both workflows. Only the
  PowerShell subprocess's inherited loader override was cleared for Nix
  compatibility; project runtime-library settings remain unchanged.
- Scoped read-only review found one test-registration gap: the two new
  PowerShell suites were not yet in the existing Windows helper-test step.
  A new workflow regression failed on that omission before the correction.
  All three suites now run before signing secrets are introduced. Fixture
  deadlines use three seconds (rather than the initial one-second local
  probe) for Windows child-startup headroom; production remains 180 seconds.
  Targeted re-review confirmed the finding resolved with no additional
  actionable issues. Native Windows execution of those tests is still pending.

These results verify diagnostic and timeout behavior on Linux, not native
Windows signing or real certificate-store cleanup. The original stall's
cause remains unproven. No Rust source/dependencies changed, so broad Rust
gates were not run. At review the changes were uncommitted; Cargo version
stays `0.1.4`, and no tags, secrets, workflow matrix, or assets changed.

The owner subsequently selected committing the diagnostic patch and merging
it into local `main`, with focused verification repeated on the merged tree.
Push, version/tag preparation, and another native trial are not included in
that authorization. No new workflow was dispatched.

## v0.1.5 Native Trial — Hosted Machine-Root Verification Trust

The owner authorized committing/merging the diagnostic patch (`55a6a0b1`,
merge `47b5d93f`), then preparing and pushing version metadata and the new
annotated `v0.1.5` tag at `b411e208`. The targeted CLI version test and merged
tag/source validation passed before the atomic main/tag push. No existing
tag moved and no workflow was dispatched by the agent.

The owner supplied the log from
[Release run 41243103652](https://github.com/vhodges/manyhands/actions/runs/41243103652),
which used `b411e208`. On October 31, 2026, PFX loading/validation, import into
`CurrentUser\My`, public export, and the Windows runtime audit completed.
SignTool's signing/RFC 3161 calls then returned success for `manyhands.exe`
in 0.6 seconds and `manyhands-cli.exe` in 0.2 seconds. The next line at
21:28:13 UTC was **Starting temporary verification trust import**. No completion
followed before the ten-minute signing-step timeout. Native signature
verification had not started.

This isolates the stalled call to `Import-Certificate` into
`Cert:\CurrentUser\Root`; it is not a timestamp-service or signing-key failure
in this trial. An inaccessible Windows current-user-root confirmation is the
leading explanation, not a dialog directly observed in the log. Microsoft
documents the machine-root import target and GitHub documents administrator
tokens/UAC-disabled Windows hosted VMs; see the
[approved correction design](2026-10-31-windows-verification-trust-design.md).

The owner approved the CI-only correction in `.worktrees/release-windows-trust`,
based on `b411e208` after fetching/merging origin/main:

- Require actual Windows plus `GITHUB_ACTIONS=true`,
  `RUNNER_ENVIRONMENT=github-hosted`, and an elevated administrator token.
  Preflight runs before private-key handling, and import rechecks the context.
  No current-user-root fallback or silent verification bypass is available.
- Use only the public `.cer` in `LocalMachine\Root`; preserve the persistent
  private identity in `CurrentUser\My`. Validate public identity before mutation,
  preserve existing trust, record exact ownership before import, and check
  imported identity/store presence. Removal accepts only an exact owned root
  thumbprint path, verifies disappearance, and fails the job if it cannot clean.
  A trust-cleanup failure still permits subsequent private-key/file cleanup.
- Both Sign/Verify modes share this lifecycle. Progress, SHA-256, RFC 3161,
  both native verifiers, and the 180-second/ten-minute deadlines remain.
  A registered native fixture tests an independent one-hour public test root
  before release secrets, with private fixture keys only in memory. Its fresh
  child process has the native deadline; its helper-test step now also has a
  ten-minute backstop. It does not replace the publisher identity.
- Red/green evidence: the portable trust tests failed before the new helpers;
  script-wiring and cleanup-continuation cases failed before script integration;
  workflow registration/helper-limit assertions failed before the YAML changes.
- Local checks: **16 focused Python tests**, **14 portable trust cases**,
  existing DLL/endpoint tests, **11 process cases**, and **5 verification-progress
  cases** passed through Devenv. Store/Windows verification APIs are stubbed
  where required on Linux. A separate actual context-discovery probe rejected
  Linux even with hosted-runner environment values; no real store was changed.
  PowerShell 7.6.9 parsed six source/test files and 21 inline Windows workflow
  bodies. actionlint 1.7.12 with ShellCheck 0.11.0 passed both workflows.
- Scoped read-only review found no actionable production, security, cleanup,
  fixture-isolation, or documentation issues. Tracked and new-file whitespace
  checks passed. Final fetch/merge left main unchanged at `b411e208`.

The real Windows root-import/preserve/remove fixture is registered but was
explicitly skipped on Linux; native trust import, full signing verification,
and actual store cleanup are still pending the next separately authorized
Windows run. No Rust source/dependencies changed and broad Rust gates were
not run. At the review checkpoint, changes were uncommitted and Cargo version
remained `0.1.5`, with tags/secrets unchanged and no new run dispatched.

The owner subsequently approved committing/merging this correction into main,
pushing origin, and preparing/pushing a new matching release tag. Prepare
`v0.1.6` with Cargo metadata and both CLI version fixtures updated, validate
the targeted version test and merged tag/source identity, and push main/tag
together. This authorization does not include native workflow dispatch or
moving existing tags; publisher identity and secrets remain unchanged.

## v0.1.6 Native Trial — All Packages And Draft Assembly Passed

The owner ran [Release 41248900448](https://github.com/vhodges/manyhands/actions/runs/41248900448)
from main for `v0.1.6`, source `818ddf88f6c99bc85ad547428058a74c6cdd06fd`.
All four native packages passed, including Windows' real temporary-trust
fixture, signing/timestamping, and extracted verification. Initial assembly
created a draft but failed with `release: new draft is not visible after
creation`, before binary uploads. Authenticated draft-list fallback already
existed; anonymous release lookup returned 404 and could not establish draft
absence. All four successful native artifacts were retained and unexpired.

The owner reran only the failed assembly job and reported the binaries
appeared. API status subsequently confirmed attempt 2 completed successfully,
with all four packages, preparation, and assembly green. Assembly's checks
covered the four archives, release-level `release.json`, and `SHA256SUMS`.
No new tag or rebuild was needed. This supports a transient post-create
visibility delay, without proving its exact cause. No agent publication
occurred; manual downloads/checksums and desktop-launch acceptance remain.

## Stage 2 Task 10 — Personal Homebrew Cask Checkpoint

The owner asked to continue Phase II and explicitly accepted deferring the
post-create lookup retry until after packaging work. The approved Tasks
10–12 were re-grounded against main; work began in a fresh
`.worktrees/phase-ii-packaging` / `phase-ii-packaging` branch from `818ddf88`,
after fetching and merging origin/main. The untracked root handoff file was
left untouched. No ticket/Cycle was assigned and no external tap operation
was authorized. This checkpoint implements Task 10 before installer/Nix work.

- Baseline: all **100 release-helper Python tests** passed through Devenv.
- Added generator cases for version/hash/published URL, actual versioned app
  and CLI paths, retention of sibling libraries/disclosures, architecture/Git
  requirements, actual minimum OS, plist disagreement, unsafe repository
  input, unresolved templates, full prerelease versions, and CLI failure with
  no output file. Observed missing-module failure before implementation.
- Added assembly regressions; before integration they failed on the missing
  cask, old checksum count, absent cask-replacement failure, and acceptance
  of missing minimum-OS metadata. Implemented generation before any draft
  mutation. Added rebuild/recovery and injected-prebuilt-cask safeguards.
- `homebrew.py` verifies the real macOS ZIP and compares the manifest minimum
  to the app plist. Homebrew's current DSL documents named major-release
  requirements, so unknown majors and nonzero point minimums fail rather
  than round down. The template retains the versioned directory in both
  install paths, links rather than copies the CLI, and never bypasses OS
  security or removes user data. The CLI keeps its sibling `lib/` layout.
- Assembly now owns seven assets: the four archives, `manyhands.rb`, immutable
  source `release.json`, and `SHA256SUMS`. The source anchor records only the
  cask's name/kind/target, never its changing archive hash. Rebuilds update
  the cask/checksums, retain source identity, and preserve owner notes and
  unrelated assets. Interrupted cask replacement removes stale completion
  checksums and is repairable. A legacy source contract is never clobbered.
- Local verification: **117 tests passed**, no skips, with Ruby 3.4.11 made
  available for the syntax regression:
  `nix shell nixpkgs#ruby -c devenv shell -- python3 -B -m unittest discover -s scripts/release/tests -v`.
  This includes 11 cask cases and 29 assembly/API cases. No Rust behavior or
  dependencies changed; broad Rust gates are not applicable to this checkpoint.
- PowerShell/workflow sources were not changed. actionlint with ShellCheck
  passed both native workflows; all 24 release Python files parsed with
  Python 3.11 grammar. Tracked/new-file whitespace checks passed. Fetch/merge
  left origin/main at `818ddf88`, and `git merge-tree --write-tree HEAD
  origin/main` succeeded. A fresh public API check reconfirmed the successful
  attempt-2 Phase I baseline. Scoped read-only review found no actionable
  cask, assembly/recovery, security, or documentation issues.

The guide documents manual post-publication tap updates and pending native
acceptance. `vhodges/homebrew-tap` remains a proposal, not a confirmed or
modified external repository. `brew style`, cask audit, actual cask install,
CLI execution via Homebrew's symlink without loader overrides, app launch,
readable installed notices, and uninstall/data preservation remain pending
on Apple Silicon macOS. Windows installer and Linux Nix packages are next;
the source remains `0.1.6` and edits are uncommitted. No new workflow was
dispatched, no tag moved/created, and no release or tap was published.

## Stage 2 Task 11 — Windows Inno Setup Installer Checkpoint

The owner approved proceeding after the Homebrew checkpoint. Work continues
in the same `phase-ii-packaging` branch/worktree, preserving its uncommitted
cask changes. The assembly-visibility retry remains deferred. No commit,
merge, push, version/tag change, workflow dispatch, publication, or external
tap operation was authorized by this request.

### Implementation And Recovery Boundaries

- Pin the official Inno Setup **7.1.1 x64** installer URL and SHA-256
  `8a5a6711c3766a6e5fb74f280ba0e3551e719ce94538d901fdbd1f8f9920b3f0`
  in `packaging/windows/inno-setup.json`. The downloaded bytes were checked
  locally against the upstream release API digest before execution. Native
  provisioning checks the hash before running and verifies `ISCC --version`.
  The upstream licence permits use/redistribution under its stated conditions;
  the commercial-use FAQ says purchase is requested, not strictly required.
  Retain the original licence and unmodified installer copyright notices.
- Stable AppId is **`io.github.vhodges.manyhands`**, matching the macOS bundle
  identity. The installer is per-user/lowest privilege, defaults to
  `%LOCALAPPDATA%\Programs\Manyhands`, restricts the initial payload to x64 OS,
  supplies a branded Start menu shortcut, and offers an unchecked CLI PATH
  task. It retains the portable ZIP and all disclosures; it never installs
  a trust certificate or deletes user data/repositories/credentials.
- Full SemVer ordering is implemented once in shared Pascal and exercised
  by both FPC and the actual Inno Pascal Script engine. Numeric file metadata
  uses `MAJOR.MINOR.PATCH.0` and rejects components above 65535. Reinstall is
  allowed; stable/prerelease downgrades and invalid state fail closed.
  `PendingVersion` is established before file mutation, so interrupted first
  installs/upgrades are repairable by the same/newer installer but cannot
  admit an older payload. Directory changes require uninstall first.
- The registry adapter preserves user PATH content/type, handles quoted,
  case/trailing-backslash and environment-expanded pre-existing entries,
  tracks only a new addition, and removes at most one exact owned token.
  It never copies the process's merged PATH into HKCU or edits system PATH.
  Tests redirect both metadata and environment operations into a unique,
  guarded HKCU fixture subtree and clean it in `finally`.
- `src/main.rs` has a Windows-release-only GUI subsystem attribute; the CLI
  remains a console application. Native import auditing now checks both
  actual PE subsystems, not a mirrored source-text assertion.
- `BuildInstaller` reuses the persistent key and guarded temporary trust
  lifecycle. The verified extraction must match every file in its ZIP before
  and after compilation. Inno's mandatory callback signs/timestamps/verifies
  uninstall/self-copy code and the final EXE with the same exact identity;
  the parent independently verifies the final EXE. No PFX/password is placed
  on a command line; no unsigned or untimestamped fallback is introduced.
  Parent cleanup still owns the key/trust and runs after compiler failure.
- The native job builds only after extracted ZIP verification. Its internal
  installer inventory binds bytes, source/tag/version, target, exact ZIP hash,
  signer, and compiler. Assembly requires it before mutation and now owns
  eight public downloads: four archives, installer, cask, source manifest,
  and checksums. The immutable manifest records source-only channel names/
  targets, not changing build hashes. Interrupted installer uploads remain
  repairable without replacing the source anchor or unrelated attachments.
- The `.ico` derives from the existing 1024 PNG using ImageMagick 7.1.2-34;
  the icon contains 16/24/32/48/64/128/256 sizes. Native helper tests and the
  compiler pin run before signing secrets. The additional build/sign step
  retains ten-minute/180-second workflow/native deadlines.

### Red/Green And Local Evidence

- New installer metadata tests first failed on the missing helper; shared
  Pascal execution failed on the missing policy include; PE policy tests
  failed on the missing subsystem gate. Assembly tests rejected the new
  candidate files before integration; workflow tests failed on missing
  compiler/installer stages and helper registration. The shared-verifier
  export test failed before extracting signing/verification for callbacks.
- All **128 Python tests** passed with Ruby 3.4.11 and FPC 3.2.2 available,
  including exact shared Pascal policy execution, cask syntax, installer
  source-byte binding, checksums, and interrupted-upload recovery.
- All five PowerShell helper suites passed again after the review correction;
  the signing suite now covers seven cases, process tests 11,
  and trust cases 14. The added installer suite checks compiler argument
  vectors, mandatory callback quoting, and rejection of a wrong engine version.
  Linux explicitly skips the native-only branches, which were separately
  compiled/executed under Wine as documented below. The full Rust test suite
  remains pending. Python/PowerShell reruns used Nix-provided interpreters
  while that Devenv process was active, avoiding concurrent Devenv state writes.
- Inno 7.1.1 compiled the **production definition with output disabled** under
  Wine 11.0; no installer or signature was produced by that check. The same
  actual engine compiled and ran the policy/registry fixture under an isolated
  Wine prefix, using fresh fixture keys/reports and exiting before installation.
  It passed upgrade/reinstall/prerelease/corrupt-state, PATH type/ownership,
  environment expansion, and cleanup cases. A fixture assumption about
  `RegWriteStringValue` was corrected after observing that Inno preserves an
  existing REG_EXPAND_SZ type; the test now explicitly creates REG_SZ.
- The interrupted-upgrade regression failed with **Interrupted upgrade allowed
  a downgrade** before `PendingVersion` support, then passed after the floor
  and same/newer repair behavior were implemented. The final production
  definition and policy/state fixture compiled and passed again under Wine.
- Rust `check --all-features --locked`, `fmt --check`, and all-target/all-feature
  Clippy with `-D warnings` passed through Devenv. The full all-feature locked
  Rust suite is running, reusing the root's generated target cache; no Rust
  command was run outside Devenv.
- actionlint with ShellCheck passed both workflows. PowerShell 7.6.9 parsed
  nine source/test files and 23 inline Windows workflow bodies. All 26 release
  Python files parsed with Python 3.11 grammar. Tracked/new-text whitespace
  checks passed. Scoped review produced the one correction documented below;
  targeted re-review accepted it with no new actionable findings.

The scoped reviewer identified one P2 issue in post-install failure reporting.
The pinned Inno source calls `SetStep(ssPostInstall, True)`, handles an event
exception, and can otherwise still exit 0. An actual no-payload fixture
reproduced this: its controlled failing completion operation ran, but returned
**0** before the correction. The installer now shares a completion-event
wrapper with that fixture, records completion only after all registry/PATH/
pending-clear operations succeed, and supplies `GetCustomSetupExitCode`:
**42 on incomplete completion, 0 on success**. Under Wine, the actual Inno
process returned 42 and 0 in the two respective cases after the fix. The
fixture is registered before release secrets and changes no app files or
registry keys; it writes only a fresh owned report. The pending-version floor
is retained on failure. Targeted re-review confirmed the P2 resolved with no
new actionable issues. The pinned uninstaller
source separately confirms `usUninstall` exceptions are fatal, so PATH cleanup
failure there stops before installed-file deletion.

Final Wine verification recompiled the production definition with spaced
payload/output paths and output disabled, then reran both actual Inno fixture
programs in fresh owned directories. The policy/isolated registry fixture
passed with its expected pre-install exit 1/report marker; the completion
fixture returned exactly 42 on failure and 0 on success. The final portable
suite again passed **128 tests**, no skips, with Ruby/FPC available. Both
workflows passed actionlint/ShellCheck again, and all nine PowerShell files
parsed. The full Rust test command has not reported completion yet; do not
call this checkpoint fully locally verified until that result is recorded.

### Native Acceptance Still Pending

Wine and FPC are not native Windows installation/signing acceptance. The
registered Inno registry fixture, actual PE subsystem checks, persistent-key
compiler callbacks, embedded uninstaller signatures, complete install/upgrade/
reinstall/downgrade/repair/remove behavior, app launch, CLI PATH behavior,
readable installed notices, and user-data preservation must be checked on
authorized disposable Windows after merge. Native workflow dispatch needs
fresh permission and a new matching source tag containing this implementation.
No new run was dispatched, no persistent identity changed, and no release or
external tap was published. Source version remains `0.1.6`; edits are uncommitted.

## Stage 2 Task 12 — Root Linux Nix Flake (In Progress)

The owner approved continuing with the Nix flake and explicitly requested
root-level placement. `flake.nix`/`flake.lock` and supporting `nix/` expressions
remain in the same `phase-ii-packaging` worktree alongside the uncommitted
Homebrew/Windows work. Devenv configuration and lock are unchanged. No commit,
merge, push, version/tag change, native workflow, publication, or external tap
operation is included in this authorization.

### Package And Disclosure Boundaries

- Export source-built `default`/`manyhands` (desktop plus separately built
  featureless CLI) and `manyhands-cli`, with desktop/CLI apps for Linux
  x86_64 and ARM64. The CLI build runs before the desktop's distinct
  production-feature build, not as a feature-unified pair.
- Pin Nixpkgs to `c7def046b9a883d46974757852106483d741586f`, providing
  matching Rust/Cargo **1.98.1**. Separately package cargo-about **0.9.3**
  from a checksum-pinned original crate and verified vendor hash
  `sha256-kNGfQQumgvtDPhiQcqc9K2ohhEsTzuySeisHcNcmig8=`. Assert both
  tool versions against `packaging/tool-versions.env` during evaluation.
- Vendor Cargo dependencies as Nix inputs. Generate notices offline with
  frozen resolution, using original source archives fetched from exact
  `Cargo.lock` SHA-256 checksums. Missing/new source-covered dependencies
  fail closed rather than borrowing a developer Cargo cache. Install
  complete normal-dependency texts, nested original notices, inventory,
  and MPL/libgit2 sources under `share/doc/manyhands`; omit desktop graphs
  and font disclosures from the CLI-only variant.
- Both wrappers provide minimal Git. Only the desktop gets GPUI's dynamic
  runtime library path, plus desktop entry/icon and embedded-font provenance.
  The flake supplies loaders, not host graphics drivers or NixGL integration.
- Use an explicit source allowlist excluding `.manyhands`, `.worktrees`,
  hidden/private files, symlinks, and build outputs; omit desktop sources
  and fonts from the CLI input. New Nix inputs are intent-to-add in the
  index solely for Git-based flake evaluation; no commit was made.
- Document checkout/tag-pinned commands, NixOS/Home Manager selection, licence
  locations, architecture/runtime limits, and conflicting CLI profile paths
  in `nix/README.md`, linked from both main and packaging READMEs. A remote
  published-tag check must wait for a tag containing the flake; `v0.1.6`
  predates it.

### Red/Green And Current Verification

- Five notice regressions first failed before offline/CLI-only/fixed-source
  support, then passed. Source-policy evaluation failed before its expression
  existed, then passed with CLI/private-state cases.
- The initial pinned-tool vendor build reported the expected bootstrap hash
  mismatch; installing the observed hash produced a successful vendor build.
- The actual desktop-loader check caught invalid Python quoting, followed
  by a missing sandbox input from a context-free path string. It now embeds
  the policy list directly. The final **source-policy and desktop-loader
  derivations both built successfully** in the sandbox; all declared dynamic
  libraries loaded. This does not establish a Wayland window or driver access.
- `nix flake check --all-systems --no-build` passed for both Linux architectures,
  including all package/app/check outputs. This is evaluation, not an ARM64
  build or runtime result.
- Portable release suite: **133 tests passed**, no skips, with Ruby/FPC/Python
  available through Nix while the existing full Devenv Rust suite remained
  active. actionlint with ShellCheck passed both workflows; `git diff --check`
  passed. No new Rust source/dependency changes are part of Task 12.
- Actual x86_64 CLI package build is running. Desktop package build, installed
  closure/disclosure verification, and remaining package checks await it.
  Scoped read-only Nix review is running. Neither application build is yet
  reported as accepted. The earlier full Rust suite remains pending its
  final result; completed intermediate targets are not full-suite acceptance.

ARM64 builds/runtime, NixOS/Wayland startup without Devenv overrides, and
published-tag consumer examples remain separate acceptance work. The
Homebrew/Windows native checks listed above remain pending too. Signing
identity, secrets, existing tags, and the successful `v0.1.6` draft are untouched;
draft-visibility retry hardening remains deliberately deferred.

### Nix Review Corrections (In Progress)

Scoped review identified two actionable gaps, accepted after verification:

1. `importCargoLock` symlinked the lockfile from the unfiltered flake root,
   retaining tracked `.manyhands` state as a transitive build input despite
   the application's clean source. A new source-policy closure check actually
   built and failed before the fix. The lock is now materialized as a
   content-only, isolated store file; rebuilding the vendor and transitive
   source-policy check passed. It checks both package sources and vendor
   inputs, not merely `cli.src`. This finding concerned tracked project
   state, not evidence that ignored/untracked secrets entered the snapshot.
2. cargo-about's success does not guarantee original copyright attribution
   when a published crate omits its licence file. The reviewer demonstrated
   canonical MIT placeholders for locked desktop crates `taffy 0.13.0` and
   `harfrust 0.5.2`. Pinned offline supplemental originals and a fail-closed
   placeholder check are being added to the shared notice generator, with
   exact original-crate/notice hash validation and disclosure inventory/HTML
   inclusion. New regression cases failed first, including a full-generator
   unresolved-fallback case and a check that permission-clause mentions of
   copyright do not count as actual attribution; they now pass. Exact
   upstream inputs, actual graph regeneration, and re-review are pending.

The running first CLI build predates these review corrections. Even if it
finishes successfully, current-tree package acceptance requires a rebuild.

### Actual CLI Build And Broader Disclosure Blocker

The first x86_64 CLI Nix package completed successfully, including offline
notice generation and installed CLI help. Compilation took 4m07s; the build
phase including notices took 4m32s. Its output is
`/nix/store/cdniqcr0afcs6s91jmwm2gq52r5p51rv-manyhands-cli-0.1.6`.
An independent installed-output check verified isolated help creates no user
state, all inventoried notice/source files and source hashes, 140 disclosed
third-party packages, no desktop/fonts, and a 35-path graphics-free runtime
closure. This is **pre-review-correction evidence**, not current-tree acceptance.
A second build with the isolated lockfile and attribution guard, plus CLI
smoke/closure derivations, is running.

The two initial upstream originals are now retained unmodified under
`packaging/licenses/supplemental/`. `supplemental.json` binds each to its
exact locked crate checksum and commit-pinned upstream URL, and checks the
original text's SHA-256. Tests verify these records. The desktop source
filter includes the supplements; the CLI input excludes them. Supplemental
notices are both inventoried and rendered in the HTML. The final portable
suite passed **143 tests**, no skips; both-architecture flake evaluation
and whitespace checks passed again.

The broader review uncovered more than the original two examples. Its audit
of existing native notice outputs identified **56 desktop package/version
pairs** with canonical MIT placeholders and no retained named attribution
across all platforms (26 in each Linux bundle). Of those, 25 have confirmed
missing upstream copyright notices, six AccessKit originals contain no
named copyright header themselves, and 25 still need provenance resolution.
These are not 56 independently established legal violations. Other omissions
include GPUI Kit's own Longbridge Apache attribution, licence files whose
names do not match the copier's pattern, and referenced AUTHORS files.

More importantly, the original locked
`wayland-protocols-plasma-0.3.12/plasma-wayland-protocols/src/protocols/blur.xml`
declares **LGPL-2.1-or-later** and the 2015 notices for Martin Gräßlin and
Marco Martin. GPUI's locked `gpui-pre-linux-0.3.6` imports and uses the
generated blur client in `src/linux/wayland/{client,window}.rs`. The crate's
top-level MIT licence does not resolve the protocol-specific policy question.
Both the XML and GPUI imports were independently inspected. No conclusion
about generated-binding/source/relinking obligations, licence-policy
exception, or dependency replacement has been authorized or implemented.

Applying the new attribution validator to the existing actual Linux graph
and freshly gathered notices (including the two pinned originals) **failed
closed at `accesskit 0.24.1`**. Resolving a verified upstream licence with no
named header requires reviewed provenance/policy, not an invented holder or
a blanket suppression. Desktop package acceptance is therefore paused;
no full desktop Nix build is claimed. The original P1 review finding remains
open beyond its two corrected examples. Request owner direction for a
focused cross-channel disclosure-remediation checkpoint before continuing
desktop packaging. CLI verification can remain separate.

Audit evidence is under `/tmp/opencode/task12-license-audit-2026-10-31/`:
`requested-licenses.json`, `placeholder-audit.json`,
`missing-mit-attribution.json`, `additional-upstream-files.json`, and
`bundled-supplement-files.json`, with downloaded/extracted originals.
No old tag, draft, signing identity, or secret was changed; no release was
published and no workflow was dispatched. The earlier full Rust test
command remains pending its final result, not inferred successful from
intermediate completed targets.

## Owner-Approved Disclosure Remediation Checkpoint

The owner selected **Remediate disclosures** and then approved the data-first
design: retain exact original notices/grants, complete source-member
harvesting, keep dependencies and licence selections unchanged, verify all
target graphs, and keep the generated-binding interpretation unresolved
unless authoritative evidence or a separately approved compliance approach
resolves it. See `2026-10-31-disclosure-remediation-design.md` and its paired
implementation plan. No integration, upstream contact, native workflow,
publication, or dependency replacement was authorized.

The corrected x86_64 CLI Nix package and its CLI smoke/closure checks completed
successfully at
`/nix/store/1j1j0rlj15v0x29gvbqkakilcflbm03f-manyhands-cli-0.1.6`.
Compilation took 3m44s and the build phase including offline notices 4m07s.
This includes the isolated lockfile and original attribution guard. The
subsequent shared disclosure-remediation changes will require a final rebuild
after their inputs stabilize; do not conflate this checkpoint with that result.

The original full Rust command exited 101 at two editor-feasibility capture
tests because `CARGO_TARGET_DIR` reused the main checkout's cache while the
test helper explicitly requires a plain **relative** `target/` directory in
the worktree. No Rust source defect or packaging-binary failure was observed.
Creating that ignored local directory and rerunning only
`--test editor_feasibility` through Devenv passed all nine tests. The owner
then clarified that feasibility is no longer needed and none of its binaries
ship; no feasibility source is changed and it is outside packaging acceptance.
Other integration targets not reached after the original command stopped,
plus doc-tests, are continuing separately through Devenv. The original full
command is still recorded as failed, not retrospectively relabelled passed.

### Remediation Evidence So Far

- New AUTHORS, arbitrary licence-directory member, and embedded XML notice
  regressions failed first, then passed after preserving those original
  bytes without flattening. Actual verification matched all **10** audited
  original crate members and SHA-256s, including granit-parser's referenced
  `.licenses` files, sha1_smol's AUTHORS, and Plasma blur/COPYING.LIB.
- New regressions for exact crate-member supplements, unsafe paths, explicit
  original grants without a named header, and unknown resolution rejection
  failed before support and now pass. Declaration-only evidence remains
  unresolved rather than becoming a blanket attribution exemption.
- A selected-graph regression first failed on the missing original Plasma
  archive, then passed after adding its checksum-verified source requirement.
  Nix fetches that original archive only for the full desktop package, not
  the CLI. Installed desktop checks now require its source/XML/LGPL files.
- Archive-verification regressions first demonstrated missing supplemental
  original hash/shape checks, then passed with per-file checksum validation
  and notice-inventory membership checks in `verify.py`.

Protocol provenance is established through the original Plasma archive,
Smithay commit `a3d7927d87799b2955bf491b51c7c2a3a82da661`, and its KDE
submodule commit `3d8327f5c8727702c5b14730c32d70b30e2ae106`.
All 49 packaged submodule files matched upstream Git blob identifiers.
The scanner generates Rust client/marshalling/interface tables from XML;
it does not embed KWin's implementation or automatically retain XML's
copyright block. No applicable explicit code-generation exception was found.
Smithay issue #796 records a relevant API-definition interpretation but
is not a rights-holder clarification for this blur definition. No output-
licensing waiver or special source/relinking compliance package was selected.

Further recovered evidence is under
`/tmp/opencode/task12-remediation-provenance/resolved-provenance.json`.
It includes exact newer-objc2 workspace grants (with their Apple SDK caveat),
content-verified originals for older declaration-only crates, Pathfinder's
proved content-equivalent source revision and original source headers, and
SeaHash's distinct later licence-only correction. The later SeaHash/malloc_buf
notices are not silently selected as release-original notices.

The curated supplemental manifest now contains **74 records for all 58
audited package/version pairs**, with 17 deduplicated byte-identical original
files and 20 exact original-crate source members. Four originals have no final
newline; after confirming that the patch tool could not preserve that byte
property, they were copied into new files with exclusive creation and both
pre-/post-copy SHA-256 checks, not normalized. Independent verification
confirmed all 58 original archives/Cargo.lock bindings, all 20 source-member
hashes and all 17 checked-in text originals, with no failures.

There are 24 exact original-grant/no-named-header resolutions, six deliberately
unresolved declaration-only cases, and 18 preserved Apple SDK conditions.
All supplemental `file` inputs are dynamically enumerated safe basenames in
the desktop source filter; no whole-directory catch-all was added. CLI inputs
continue to omit the desktop manifest/originals, and the transitive source
policy and dynamic-loader derivations built successfully again.

Current portable verification: **155 helper tests passed**, no skips;
26 Python files parsed with Python 3.11 grammar; both workflows passed
actionlint/ShellCheck. A sandboxed notices-only verification of the same
production package source/tool/vendor inputs is running for all four target
graphs, without compiling application binaries. It is expected to expose
remaining declaration-only blockers rather than suppress them. A scoped
read-only remediation review is also running. Latest application rebuilds
wait for stabilized inputs and those results; no new desktop build/runtime
acceptance is claimed.

The actual offline notices-only attempts completed for all four target
graphs using the production source, vendor, pinned tool and fixed archives
inside Nix sandboxes. They failed closed on the remaining declaration-only
evidence, not on network access, source hashes, tool versions or missing
supplemental inputs:

| Target | Disclosure metadata packages observed (including application) | Unresolved declarations |
| --- | ---: | --- |
| Linux x86_64 | 584 | mac 0.1.1; seahash 4.1.0 |
| Linux ARM64 | 581 | mac 0.1.1; seahash 4.1.0 |
| macOS ARM64 | 499 | block 0.1.6; leak 0.1.2; leaky-cow 0.1.1; mac 0.1.1; malloc_buf 0.0.6; seahash 4.1.0 |
| Windows x86_64 | 484 | mac 0.1.1; seahash 4.1.0 |

No completed notice bundle or application package acceptance is inferred
from these failures. The source-only verification expression is retained at
`/tmp/opencode/manyhands-notice-verification.nix`; it strips application
compilation from the production phase and retains actual offline generation.

Scoped remediation review found two further P2 correctness gaps. The first,
arbitrary nested copyright text clearing an unrelated canonical licence,
is being corrected with package/licence-bound evidence and whole-notice
placeholder checks. The second concerned macOS app resources: top-level
original hashes were checked but missing/corrupted nested app copies could
pass. Four missing/corrupt notice/source app-copy cases failed before the
fix; all 17 verifier tests now pass after comparing every inventoried app
notice/source with its validated package-root counterpart, including before
signing (`final=False`). These are structural fixtures, not macOS native
signing/install acceptance.

The new originals exposed one intentional whitespace exception: upstream
`zune-image-LICENSE.md` ends its copyright line in a space. `git diff --check`
failed on that exact byte before a **file-specific** `blank-at-eol` allowance
in `.gitattributes`; it now passes without changing the original. All
supplemental originals are also marked `-text` to prevent Windows checkout
line-ending conversion from breaking pinned hashes. `git check-attr`
confirmed the narrow scope; original/archive/member hash verification still
passes. No broad whitespace suppression or notice normalization was added.

### 2026-11-01 Decisions And Continuation Results

The owner explicitly selected **Accept scoped declarations** for only the
six verified declaration-only releases. Their exact original manifests/
source grants and standard terms may be retained as reviewed grants by
reference, with no invented copyright holder and no assertion that a named
notice was supplied. This is package/version/checksum-bound policy, not a
blanket exemption or a finding that the packages were unlicensed. KDE
generated-binding and Apple SDK conditions are expressly outside that
approval. The shared generator/data are being updated to record the decision;
new/unapproved declaration-only evidence must still fail closed.

The remaining Rust integration targets and all **15 doc-tests passed** in
the Devenv continuation. Combined with completed original-run targets and
the scoped nine-test harness rerun, test-target coverage is complete across
the runs. The original broad command still exited 101 at the missing local
capture-directory setup; it is not relabelled as a successful single run.
No feasibility source changed, and the owner excluded it from shipped
packaging scope. All application Rust checks were run through Devenv.

The attribution P2 fix now binds evidence to the selected licence and
component, requires complete matching grants, and rejects placeholders
across the complete candidate text. Its mandatory nested-GNU/FSF and
multiline-template regressions failed first, then passed. This exposed
three compound/nested-source cases needing explicit bindings, not new
licence-policy waivers: chrono's MIT section alongside an Apache template,
libm's actual whole-crate MIT grant without a named header, and granit-parser's
explicitly linked inherited MIT originals/modification notice. Pinned,
inclusive 1-based scopes preserve and hash the selected original lines;
whole original files and other licences remain unmodified in the bundle.
Invalid/boolean/out-of-range/stale scopes fail. After these additions,
69 notice tests passed, and cached Linux/macOS replays again left exactly
the six declaration-only cases awaiting the now-approved policy update.

The manifest at this checkpoint has **79 records for 61 package/version
pairs**, 25 exact original source members, 17 byte-identical checked-in
originals, and two raw-byte scope hashes. These counts include the three
additional proved cases; earlier 74/58/20 counts are historical checkpoints.
Fresh target-graph generation, final package builds, and targeted re-review
remain to be rerun after the explicit declaration-policy change.

The scoped declaration update subsequently passed **84 notice tests**, and
an independent targeted review accepted the bindings/scopes/approval proofs
with no actionable findings. It checked the exact six-version approval set,
all original inputs and scope hashes, preserved conditions/provenance, and a
cached ten-package/fifteen-supplement HTML/inventory replay. This is code-
review acceptance, not a legal/output-licensing clearance.

Fresh offline generation after that update **passed Linux x86_64, Linux
ARM64 and macOS ARM64** in the actual Nix sandbox. Windows exposed a narrow
validator mismatch: its Microsoft `license-mit` originals contain the full
grant and Microsoft attribution but omit the final full stop. A matching
indented original regression failed first. The comparison now permits only
the exact complete canonical body minus that terminal period at candidate
EOF; truncated final words, joined suffixes and missing clauses remain
rejected. All 87 notice tests passed; targeted independent re-review accepted
the three new cases/comparison with no findings. No original was normalized.

The final portable suite passed **213 tests**, no skips. Both workflows
passed actionlint/ShellCheck, all 26 release Python files parsed with Python
3.11 grammar, and whitespace checks passed with the documented exact-file
allowance. All four real offline target graphs are being regenerated once
more after this final comparison fix.

The pre-terminal-period CLI snapshot also built successfully with its smoke/
closure checks at `/nix/store/l2nwv9d3vnc7zdyb6fchcnv425zj82xh-manyhands-cli-0.1.6`.
The corresponding desktop build was deliberately cancelled via its owned
Nix client after the source snapshot became stale, not because of a compiler
failure. Current-tree CLI verification is running; the desktop build will
restart against the stabilized source after that. Do not count the cancelled
attempt as desktop build acceptance. This host is NixOS 26.11 with an active
Wayland session; any actual desktop startup/visual acceptance must still be
recorded separately after a successful package build.

Fresh offline generation after the terminal-period fix **passed all four
target graphs** inside the Nix sandbox. An independent installed-bundle
audit then verified every inventoried notice path, each original-file and
source-archive checksum, all applicable manifest records, the full original
texts in HTML, both raw scope hashes, dated owner approvals and retained
conditions:

| Target | Third-party packages | Notice paths | Supplemental records | Original source archives | Scoped declaration approvals | Retained conditions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Linux x86_64 | 583 | 1169 | 48 | 3 | 2 | 0 |
| Linux ARM64 | 580 | 1163 | 48 | 3 | 2 | 0 |
| macOS ARM64 | 498 | 932 | 67 | 2 | 6 | 18 |
| Windows x86_64 | 483 | 910 | 29 | 2 | 2 | 0 |

All four bundles contain the two scoped original grant sections. Linux
also preserves the exact blur XML hash, original LGPL text and original
Plasma crate; unrelated targets do not acquire that Linux dependency.
These are real cross-target **disclosure-generation** checks on Linux,
not ARM64/macOS/Windows executable builds or native runtime acceptance.
The current successful notice outputs are:

- Linux x86_64: `/nix/store/lv3fgg0q22l2hi40bddxn5mzbpfwdkzp-manyhands-offline-notices-x86_64-unknown-linux-gnu`.
- Linux ARM64: `/nix/store/1sb1hv5cgb3rmnfifdb656nlmxhqqk6l-manyhands-offline-notices-aarch64-unknown-linux-gnu`.
- macOS ARM64: `/nix/store/41wn1nz1yvazpyc4ii7p50p3zd8by1py-manyhands-offline-notices-aarch64-apple-darwin`.
- Windows x86_64: `/nix/store/14sp1m267v52x9z0j3d3vm3qj9msf5h8-manyhands-offline-notices-x86_64-pc-windows-msvc`.

The mechanical disclosure blockers are resolved under the recorded scoped
policy. KDE generated-binding and Apple SDK distribution decisions remain
explicitly open; a successful generator/build is not legal clearance. No
dependency change, later-notice borrowing, signing change, workflow dispatch,
version/tag operation, external contact or publication occurred.

### Authorized PR Handoff

On 2026-11-01 the owner authorized completing the remaining local checks,
committing in logical chunks, pushing `phase-ii-packaging`, and opening a PR.
This supersedes earlier no-commit/no-push instructions for this work. It does
not authorize merge, tags/version changes, native workflow dispatch, release
publication, external tap changes, or signing-identity changes.

The owner also asked to separate technical packaging acceptance from
distribution-policy interpretation. The KDE generated-binding question is
not an established violation; attribution alone would not necessarily
satisfy every obligation if LGPL-covered bindings were conveyed. The objc2
Apple SDK caveat is one upstream uncertainty repeated across 18 dependencies,
not 18 distinct established restrictions. Preserve and document these
uncertainties in the PR without claiming legal clearance or expanding this
packaging task into further licensing research. They do not block code review.

Fresh handoff checks passed: 213 portable helper tests with no skips;
actionlint/ShellCheck on both workflows; Devenv Rust formatting, all-feature
check, and all-target/all-feature Clippy with warnings denied; and evaluation
of both architectures' package/app/check outputs. The latest current-tree
CLI and CLI smoke/graphics-free-closure checks passed at
`/nix/store/gyl0fz6b1wdvw3k2k3ry1y6bk6696g5c-manyhands-cli-0.1.6`.
Source-policy/transitive vendor isolation and desktop-loader checks passed
again. Fetching `origin/main` showed the branch base is still current, with
no ahead/behind commits and a clean `merge-tree` result.

An installed CLI check outside Devenv, with isolated HOME/XDG paths and
loader overrides unset, passed `--help` and verified all 140 dependency
inventory records/source hashes. It created no user state and contained no
desktop binary, fonts, supplemental desktop disclosures or loader override.
Evidence: `/tmp/opencode/phase-ii-final-cli-1d2630fu/`.

Final read-only integration review accepted the eight-asset assembly,
installer payload/signing boundaries, separate GUI/CLI subsystem, root flake
outputs, source isolation and production-only binary selection, with no
P1/P2 findings. Its one P3 stale-status summary has been refreshed. Prior
native/ARM64/tagged-consumer acceptance limits remain; the current desktop
build and installed runtime checks are the remaining local gate.

The final current-tree desktop package and installed-disclosure check then
completed successfully at
`/nix/store/sqnj9v1pyppygv0brag8ynyk40c4pw1s-manyhands-0.1.6`.
Its build phase took 19m11s: the featureless CLI built first (3m36s), then
the desktop (14m12s), followed by actual frozen/offline disclosures and
successful installation/fixup/install checks. `nix flake check --cores 2
--max-jobs 1 -L` subsequently passed all current x86_64 checks (already
realized derivations were reused). ARM64 was evaluated separately, not built.

Installed full-package inspection confirmed only the two production binary
wrappers and their private wrapped executables, no feasibility binary; the
CLI wrapper supplies Git but no graphics loader override. Its isolated help
created no user state. All 583 third-party inventory records matched the
independently audited offline output; supplemental and source hashes passed,
and fonts, desktop entry and icon were present. The desktop wrapper includes
the enumerated Wayland/Vulkan/XCB/XKB/GL/ATK/font/GLib/GTK/OpenSSL loader
paths. Evidence: `/tmp/opencode/phase-ii-final-full-package-w9x8w4zf/`.

The installed desktop wrapper was launched outside Devenv on this host's
NixOS 26.11/COSMIC Wayland session with inherited loader overrides removed
and fresh HOME/config/data/cache/state directories. It exited normally
(code 0) after 14.32s with no stderr. The initial harness classified an exit
before its 20s timer as unaccepted; the owner then explicitly confirmed
**the window opened, text rendered correctly, and they closed it**. This
establishes local visual startup acceptance rather than merely process
survival; the short interval was a manual close, not an application failure.
The registry and Mesa caches were created only in the isolated test roots.
Evidence: `/tmp/opencode/phase-ii-desktop-startup-2joymjjn/`, including the
original timing fields and subsequent owner-confirmation annotation.

GitHub CLI access is an environment limitation, not a packaging failure.
Read-only `gh` requests timed out before API access while waiting on the
desktop Secret Service; direct GitHub HTTPS and Git-over-SSH worked. The
owner also encountered an interactive-parser error on an OSC 11 terminal
background-colour response after an in-place system update. No credential,
keyring, terminal, SSH-key or system change was made to work around it.
Do not keep retrying `gh` or expose tokens. Finish the authorized SSH push
and provide the browser compare/new-PR link and prepared description if API
authentication remains unavailable. No PR is claimed created by that link.

After the owner confirmed the desktop window, the full 213-test portable
suite and whitespace checks passed again. Staged-byte verification confirmed
all 17 supplemental originals retained their pinned hashes, including the
four files without final newlines and Zune's intentional trailing space.
No Cargo manifest/lock, Devenv input, native Build workflow, CLI entry point
or feasibility source was changed.

The authorized implementation commits are:

- `ccac578f`: preserve and validate original dependency disclosures.
- `c21df209`: add Homebrew cask and signed Windows installer.
- `1426f5ef`: source-build Nix desktop and featureless CLI packages.

The remaining handoff documentation is committed separately. The prepared
PR description is retained in `2026-11-01-phase-ii-packaging-pr.md`, rather
than only in a reboot-volatile temporary directory. Push this branch over
SSH and use `https://github.com/vhodges/manyhands/compare/main...phase-ii-packaging?expand=1`
for the owner to submit the PR if `gh` remains unavailable. Keep this
worktree for review/follow-up; no automatic merge or cleanup is authorized.

### Supporting Nix Directory Relocation

On 2026-11-01 the owner revised the earlier placement decision and approved
moving all supporting `nix/` files directly to `packaging/nix/`, while
retaining `flake.nix` and `flake.lock` at the root. This supersedes the
root-level supporting-directory choice recorded above; earlier paths and
store outputs remain historical evidence. No old-path forwarding file or
symlink is retained. The owner also approved a follow-up commit/push on the
same branch; no merge, workflow, tag, release or credential change is included.

The root imports, internal relative licence/runtime-manifest references,
README source-filter/install paths and current documentation links/plans
were updated. Eight new source-policy cases first failed against the old
layout, then passed after the relocation. They verify the relocated guide
and its ancestors are included, the old location is excluded, and private,
implementation and symbolic-link files remain excluded from build sources.

Verification passed: both architectures' package/app/check outputs evaluate;
the x86_64 source-policy/transitive vendor-isolation and desktop-loader
derivations build; and all 213 portable packaging tests pass without skips.
The lockfile, dependencies, toolchain, Devenv, application code, disclosure
inputs, build commands and runtime wrapper logic are unchanged.

Comparison with the successfully built pre-relocation derivations confirmed
the only changed derivation fields are the filtered source, resulting output
address and guide installation path. All 719 desktop-source and 430 CLI-source
file hashes/modes match after accounting for the guide move and the root
README's updated hyperlink. Evidence: `/tmp/opencode/check-nix-layout.py`
and `/tmp/opencode/nix-layout-equivalence.json`. This verifies a layout-only
change; application executables were not rebuilt or relaunched after it.
The preceding successful binary builds, full flake checks and owner-confirmed
Wayland rendering remain explicitly pre-relocation results.

Scoped read-only review accepted the relocation with no actionable findings.
It independently repeated offline both-architecture evaluation and the
719/430-file source/derivation comparison, confirmed current filtered sources
match the working tree, and checked relative references, exclusion policy,
unchanged pinned inputs and documentation consistency. Whitespace checks pass.

### Eight-Finding Review Follow-Up

The owner approved the scoped review fixes on 2026-11-01. Current implementation,
red/green evidence, verification results and remaining native limitations
continue in [the review-fix execution record](2026-11-01-packaging-review-fixes-execution.md).
Earlier package outputs and acceptance results above remain historical;
they do not automatically verify the new review changes.
