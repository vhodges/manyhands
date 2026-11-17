# Packaging Review Fixes Execution

## Approved Scope

The owner approved the reviewed fix pass and follow-up commits/push on
2026-11-01. The starting commit is `79e5df6b` on `phase-ii-packaging`.
See the paired design and implementation plan. No merge, tag, release,
workflow dispatch, tap repository, credentials or signing-identity changes.

## Review Triage

- Findings 1, 3–6 were confirmed in source; curly placeholders were also
  reproduced through both named-attribution and complete-MIT-grant paths.
- Finding 2's unconditional rollback assumption was rejected: pinned Inno
  7.1.1 preserves overwritten pre-existing files on cancellation. Preparation
  should remain read-only, but the repair floor must survive possible writes.
- Finding 3's wholesale pre-install deletion suggestion was rejected because
  it can destroy the previous installation before a new payload succeeds.
  Approved correction is post-copy pruning of the installer-owned trees.
- Production Git subprocesses were not found. Located invocations are
  test-only; production transport uses libgit2 and explicit SSH callbacks.
- Finding 8's dead argument guard and duplicate config read are cleanup;
  pre/post-compile payload equality is an intentional security invariant.

Pinned engine evidence is cached under `/tmp/opencode/inno-7.1.1-Setup.*.pas`;
in particular `Setup.UninstallLog.pas:996–1028` distinguishes rollback from
uninstallation and retains files that existed before installation.

## Completed Independent Checkpoints

### Installer Lifecycle, Managed Files And Budget (1–3, 8)

Preparation now only reads/validates state. The pending-version/install-dir
checkpoint runs at fatal `ssInstall`, before `PerformInstall`, rather than in
preparation or the exception-swallowing post-install event. The floor remains
after files may have changed; same/newer repair is required. Successful
completion still returns 0, incomplete post-install completion returns 42.

The incoming UTF-8-BOM/CRLF managed-file list is derived from the exact
verified ZIP plus the Inno licence and list itself. It is written exclusively
outside staging and embedded by the compiler, not trusted from an earlier
installation. Shared Pascal policy rejects unsafe Win32 paths, aliases and
collisions. The adapter validates the entire list, expected files, both
installer-owned trees and reparse-free ancestors before pruning stale files
within `bin`/`licenses`, after copying and before completion. Root/user files
are not pruned, and empty obsolete directories may remain. Failures retain
the pending floor and return 42; no pre-install wholesale deletion was added.

ISCC has an explicit 1,800-second deadline. Mandatory signing allows two
retries with a 2,000 ms delay; the installer step has 45 minutes, leaving
15 minutes for surrounding verification, smoke testing and cleanup. Ordinary
native tools retain 180-second deadlines; application Sign/Verify steps
retain ten-minute backstops. Mandatory signatures, exact publisher checks and
RFC 3161 timestamps remain unchanged. The dead archive guard was removed and
the validated timestamp URL is passed without duplicate config reads; both
pre- and post-compilation payload-equality checks remain.

Observed red cases covered the missing compiler wrapper/managed inputs,
accepted Win32 aliases, and manifest writes following an existing symlink.
Fourteen installer tests and thirteen workflow tests pass after the fixes.
The actual pinned Inno 7.1.1 engine passed all four fixture tests, including
twelve lifecycle scenarios:

| Scenario | Exit | Verified effect |
| --- | ---: | --- |
| Preparation cancellation | 7 | Old files/version preserved; no new floor |
| First checkpoint write failure | 3 | No copying; no new floor |
| Second checkpoint write failure | 3 | No copying; pending floor retained |
| Mid-install cancellation | 5 | Overwritten bytes remain changed; floor retained |
| Pre-install reparse point | 7 | Rejected before checkpoint/copy |
| Missing expected file, malformed/colliding list, post-copy reparse point, prune failure | 42 | Incomplete; floor retained; version not advanced |
| Successful upgrade/fresh install | 0 | Incoming files retained; completion recorded |

The separate completion fixture still proves 42/0. The retry fixture observes
exactly three failed signing attempts, bounded delays and no unsigned output.
The production definition compiles with output disabled only. These Wine/FPC
results are not native Windows signing/install acceptance.

Independent review then found a P2 gap in destination verification: existing
regular files alone could satisfy the post-copy gate even when Inno retained
old bytes after a read-only-file/copy-error skip. The pinned engine source
confirms both paths (`Setup.Install.pas:990–1000,1353–1369`). This could allow
pruning and false completion.

The correction retains hashes from the verified ZIP after exact payload
equality and generates a separate canonical `managed-digests.txt`, covering
all managed payload files, the exact Inno licence and the installed manifest's
complete bytes. It is compiler-embedded with `dontcopy`, never installed or
self-hashed. Shared Pascal policy requires lowercase SHA-256 and exact
one-to-one count/path/order binding to the file list. Embedded expectations
are cached at startup; completion never reloads installed or temporary trust
metadata. Before the first stale deletion, Inno's `GetSHA256OfFile` verifies
every expected managed file. The producer reserves both embedded basenames
against portable-payload shadowing. Existing path/reparse guards remain.

Six actual-engine regressions first failed against the existence-only gate:
old executable, read-only old executable, old CLI, old licence, old manifest,
and forged installed digest metadata all returned 0, advanced Version,
cleared PendingVersion and removed stale sentinels. After the fix, five engine
tests passed in 235.372 seconds, covering twelve lifecycle scenarios plus
seven content/immutability scenarios (including forged installed and temporary
digest tables). Mismatches return 42, retain the floor and old Version, and
preserve both stale sentinels. The fixture simulates retained/skipped bytes at
the actual `ssPostInstall` boundary; it does not automate native Windows
Keep-existing/Skip dialogs. Sixteen installer tests (including FPC), thirteen
workflow tests and focused PowerShell suites also passed. Independent
read-only re-review accepted the correction with no new actionable issues:
the prior P2 is resolved. Final parent integrated verification passed as
recorded below.

### Windows Error Reporting (4)

The desktop installs its panic-reporting hook before fallible Git runtime
initialization. Windows release builds show a native fixed-ASCII error
dialog without panic payloads or paths. Existing stderr/default-hook behavior
and unwind/exit semantics remain; no persistent log is added. Debug/non-Windows
paths and the CLI are unchanged. Only the existing windows-sys messaging API
feature was enabled; dependency versions and Cargo.lock are unchanged.

Five focused test entries pass through Devenv after two expected red cases,
covering injected dialog reporting, previous-hook chaining and subprocess
panic behavior. This does not compile/run the Windows API branch on Windows;
native dialog linkage/visibility remains a native acceptance item.

### Curly Attribution Placeholders (5)

The original failure was reproduced for both `{year} {fullname}` and
`{{year}} {{fullname}}`. Twelve new tests produced 82 expected failing
subcases before the regex correction, then all **99 notice tests passed**.
Direct, root-grant, canonical-tool, scoped, supplemental and multiline paths
are covered. Original bytes, bindings/scopes, approved declarations and
licence selections are unchanged; no licensing investigation was added.

### Versioned Nix Checks And Local Git Requirement (6–7)

Source/protocol checks now use selected inventory identities and a content-
only manifest derived from Cargo.lock, not duplicated crate versions. They
verify original archive hashes, reject unsafe paths/symlinks, retain protocol
notice coverage, reject missing/empty inventories and allow unused locked
versions. Thirteen focused tests pass after the observed red cases.

The local Git runtime requirement was removed from Nix, the Homebrew cask,
native runtime metadata and generated installation/release guidance. Tests
failed first for all four staged target requirements and for finalized
Windows/macOS metadata, release notes and the cask, then passed. Build/test
Git tools and the server-side Git service are not being removed.

The CLI is installed directly; only the desktop retains its graphics wrapper.
The installed Nix smoke check now creates and commits a repository with an
empty PATH, and the closure check excludes Git as well as graphics packages.
That exact new check body also passed against the prior raw CLI executable,
without invoking its old Git-prepending wrapper. Evidence:
`/tmp/opencode/review-no-git-cli-check-1wspzv_f/`.

Both flake architectures evaluate, and the source-policy/transitive-isolation
and desktop-loader derivations build. Independent scoped review accepted
findings 4–7 with no actionable findings, including the latest disclosure
guards and isolated lock-manifest string context. Its 180-test scoped Python
run passed with one Ruby-only skip; the parent separately ran all 11 cask
tests with Ruby available and no skips.

## Fresh Cross-Target Notice Verification

Offline generation with the stricter brace-placeholder recognition passed
all four targets in Nix sandboxes. An independent bundle audit verified all
applicable supplemental records, original/source SHA-256s, raw scopes and
complete-original HTML. Counts remain unchanged:

| Target | Third-party packages | Notice paths | Supplements | Source archives |
| --- | ---: | ---: | ---: | ---: |
| Linux x86_64 | 583 | 1169 | 48 | 3 |
| Linux ARM64 | 580 | 1163 | 48 | 3 |
| macOS ARM64 | 498 | 932 | 67 | 2 |
| Windows x86_64 | 483 | 910 | 29 | 2 |

Evidence: `/tmp/opencode/packaging-review-notice-audit.json`. Successful
outputs are:

- `/nix/store/jwr3f760gn9rg4z9yd8dzf1x24z7vzmw-manyhands-offline-notices-x86_64-unknown-linux-gnu`.
- `/nix/store/gg1lji4zhpzlgw1yiigvyvjkc8ss3wqr-manyhands-offline-notices-aarch64-unknown-linux-gnu`.
- `/nix/store/96zi416bm7k87v12win0zn895qf5nblc-manyhands-offline-notices-aarch64-apple-darwin`.
- `/nix/store/2vzyvfkincz56q471cqy0caw0wx1f88y-manyhands-offline-notices-x86_64-pc-windows-msvc`.

The one upstream Apple SDK caveat remains retained for 18 dependencies.
No licensing decision or original input changed. Cross-target notice
generation on Linux does not establish native executable acceptance.

## Integrated Local Verification

- The parent full helper run passed **252 tests in 188.453 seconds, no skips**,
  with Ruby, FPC and all four actual Inno-engine tests under Wine/Xvfb. The
  owned prefix is `/tmp/opencode/manyhands-review-parent-inno-wine-20261101`.
- All five PowerShell helper suites passed: tools, process (11 cases), signing
  (8), trust (14) and installer. Actual Windows certificate-store APIs remain
  guarded/stubbed on Linux; these passes are not native certificate acceptance.
- Both workflows passed actionlint/ShellCheck. All 29 Python files parse with
  Python 3.11 grammar. All 79 supplemental lock bindings and 17 exact originals
  verified without changing licence bytes. `git diff --check` passed.

These integrated results precede the destination-content review correction;
the results below supersede them for that correction.

The final parent full helper rerun passed **255 tests in 214.158 seconds,
no skips**, including Ruby syntax, FPC/shared policy and all five actual
Inno-engine tests under Wine/Xvfb. Its fresh owned prefix is
`/tmp/opencode/manyhands-review-content-parent-wine-20261101`.

After that correction, the parent repeated all five PowerShell helper suites
successfully (process 11, signing 8, trust 14), parsed all nine PowerShell files,
reran both workflows through actionlint/ShellCheck, and rechecked all 29 Python
files against Python 3.11 grammar plus all 79 supplemental bindings / 17 exact
original hashes. These checks passed. Windows certificate-store APIs remain
stubbed/guarded; no real trust-store change or native-signing claim is implied.

Parent helper evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_197e2729f001lS7lCcFl2xC2vb.out`.
Final post-correction evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_197fd0d4d0015YwkO8ljmjZXH8.out`.

Both-architecture offline flake evaluation was repeated successfully after
the installer correction. Current x86_64 source-policy/transitive-isolation
and desktop-loader checks also build. Package derivations still match the
completed application-build command (`aklr3801…` desktop, `k50apnjj…` CLI), so
documentation/test updates have not invalidated its package inputs.

### Rust Gate And Timeout Investigation

The sequential Devenv command passed `cargo fmt --check`,
`cargo check --all-features --locked`, and
`cargo clippy --all-targets --all-features --locked -- -D warnings`.
Its full locked all-feature test run passed all 1,478 library tests, then
finished the desktop target with 1,085 passed and two failed. Both failures
were 10-second `until_drawn` waits at `src/desktop/view/harness.rs:301`:

- `desktop::services::drafts::create::tests::save_of_a_restored_not_created_draft_creates_it`
- `desktop::services::drafts::create::tests::the_close_dialogs_save_of_a_not_created_draft_creates_it_and_closes`

Neither those tests nor their desktop implementation/harness are changed by
this branch. Both exact tests passed in isolated Devenv reruns (2.02 and
2.09 seconds), without code changes. This is consistent with load-sensitive
timeouts, not proof that the broad run passed. A fresh complete test run with
`-- --test-threads=2` then passed every target, including those the first
command did not reach: **4,901 standard tests/doc-tests and 857 SSH cases
(5,758 passes total)** across 32 standard targets and 11 SSH targets. This
includes all 1,478 library tests, 1,087 desktop tests, 252 CLI unit tests and
15 doc-tests. One existing on-request performance characterization remains
ignored (`tests/read_characterization.rs`); no new skips or timeout increases
were introduced. The original broad-run failure remains recorded above.

After that success, the parent repeated formatting, all-feature check and
all-target/all-feature Clippy through Devenv; all passed. The prescribed
`cargo run --locked --bin manyhands-cli` smoke also passed, displaying help
without opening a window.

Initial command evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_197c8a7ee001o9d55qRHRlDBWE.out`.
Full rerun evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_197f7fc650016R9qB9ZXpbtbTr.out`.

### Current Nix Package Builds And Installed Audit

The current x86_64 desktop and CLI packages built successfully, including
their offline disclosures and install checks. `nix flake check --offline -L`
then passed every current-system check: source-policy/transitive vendor
isolation, empty-PATH CLI smoke/repository creation, Git-/graphics-free CLI
closure, desktop disclosures, and dynamic library loading. Both-architecture
evaluation also passed; no ARM64 executable/runtime result is claimed.

Current review-fix outputs (not the historical pre-review packages):

- Desktop plus featureless CLI:
  `/nix/store/5cm95f5p9n6nl96lnbyrjjgqddyzvkib-manyhands-0.1.6`.
- CLI only:
  `/nix/store/23h94l5bznjsmyrbyy9rv5wlbdf17abh-manyhands-cli-0.1.6`.

The installed-output audit verified both CLIs are raw ELF executables without
Git-prepending wrappers. With fresh isolated HOME/XDG, no loader overrides and
`PATH=""`, both installed CLIs displayed help without creating user state,
then created and committed a repository. Neither runtime closure contains a
Git executable; the CLI-only closure has eight paths and no graphics packages.
The desktop retains its graphics loader wrapper and private wrapped ELF,
without setting PATH. No desktop window was opened for this review-fix build;
the owner's earlier manual Wayland/rendering acceptance remains historical.

| Installed variant | Third-party packages | Notice paths | Supplements | Source archives |
| --- | ---: | ---: | ---: | ---: |
| Desktop | 583 | 1169 | 48 | 3 |
| CLI only | 140 | 283 | 0 | 2 |

Every inventoried notice exists, and the desktop inventory/notice bytes match
the independently audited fresh offline target bundle. All installed source
archives match Cargo.lock and inventory hashes; all applicable supplemental
records, exact originals, raw scopes and complete-original HTML were checked.
Installed project licence/guide bytes match the repository; desktop fonts and
CLI font absence are correct. Evidence:
`/tmp/opencode/packaging-review-installed-audit.json`.
Build output:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_197cf7fae001LNt4jltP2JCUmO.out`.

## Verified Handoff And Remaining Acceptance

All local implementation, regression, integrated-test, review and package
checkpoints for this fix pass are complete. The owner-authorized handoff is
logical commits and an SSH push of **only `phase-ii-packaging`** to origin,
preserving this worktree and the prepared browser PR description. GitHub CLI
authentication was not retried or reconfigured; browser PR submission remains
the fallback, not a claimed API-created PR.

Latest fetch found no new commits on main: `origin/main` is still `818ddf88`,
already contained by this branch. `git merge-tree --write-tree HEAD
origin/main` succeeds. No merge or push to main, version/tag change, native
workflow dispatch, release publication, tap update, signing-identity change,
licence-original edit or credential configuration is included.

Native Windows signing/install/upgrade/repair/uninstall/dialog acceptance,
native Homebrew acceptance, ARM64 executable/runtime checks and future
published-tag consumer examples remain unperformed. Wine/FPC, injected-dialog
tests, loader checks and cross-target notices do not replace those checks.
The KDE/Apple licensing uncertainties and six exact scoped owner approvals
remain unchanged; no further licensing investigation was undertaken.
