# Packaging Review Follow-Up Execution

## Approved Scope

Starting point: `0871a0e0` on `phase-ii-packaging`. The owner approved fixing
the follow-up findings and explicitly selected **Document limitation** for
recovered panics. The design and plan are paired with this record. No Rust
behavior change, native workflow dispatch, merge, release, credentials,
licence-original, dependency, version/tag or signing-identity changes.

Latest main was fetched before implementation and remains an ancestor of
this branch (ten branch commits ahead, zero main commits missing).

## Reparse Boundary

The actual pinned Inno 7.1.1 engine first rejected both fresh and upgrade
installations through an ancestor profile junction. The logs identify the
ordinary application directory's parent junction as the rejected path.
The separate root/descendant rejection cases already passed.

`CheckManagedAncestors` now canonicalizes the application root and argument,
requires root-or-descendant membership with a separator boundary, checks the
application root itself, then stops. It retains fresh owned-tree, per-file
and mutation-boundary checks; redirected ancestors are not installer-owned.

The two new engine tests then passed in 48.412 seconds, covering successful
fresh/upgrade through a junction and rejection at/below the application root.
No native Windows installation acceptance is implied.

Red evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_19940be3d001ou84d5GosLMT7p.out`.

## Author Placeholders

Four new tests cover singular/plural author labels in angle, square, single-
and double-curly delimiters, case/multiline variants, canonical-tool text,
complete original grants, bound supplements and raw-CRLF scoped evidence.
They produced **56 expected failing subcases** before the one-label-vocabulary
change, then all **103 notice tests passed** in 0.233 seconds. Ordinary author
names remain valid; original bytes and owner approvals are unchanged.

Red evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_199414e83001vzuOen0puMH5hR.out`.

## Recovered Panics

The operational guide now documents the selected limitation: the fixed-text
Windows release dialog runs before `catch_unwind` can recover, blocks its
thread until dismissed, and does not release already-held locks. Restart
guidance is conservative, not proof that the process terminated. No Rust
source change or global-hook swapping/suppression was added.

## Validation Cost And Immutable Expectations

The permanent 1,000-entry engine regression first failed with per-pass times
of 14,725–48,644 ms (the original quadratic implementation). Its seven-pass
fixture intentionally repeats the old upgrade pattern. The baseline run was
concurrent with notice generation, so use the separate original 13.877-second
single-pass / 63.804-second actual-payload measurements for a cleaner baseline,
not a claimed constant speedup across different load conditions.

The new policy validates each path once and emits sorted directory/file-prefix
nodes. Stable bottom-up merge sorting and adjacent equal-prefix checks reject
case aliases, duplicate files and file/directory collisions, including the
non-neighbour `foo` / `foo-bar` / `foo\child` trap. Input file/digest ordering
is preserved. A separate sorted incoming lookup supports binary pruning
membership; immutable embedded tables are validated only when loaded, while
destination snapshots and installed content checks remain fresh.

FPC/shared policy tests first failed on the missing sorting/lookup functions,
then all 16 Python installer tests passed with compiled policy execution.
The pinned-engine scale regression passed after the correction: seven passes
of 807–969 ms, with a generous 5,000 ms per-pass / 20,000 ms total budget.

Using the fresh actual Windows disclosure bundle's 920 managed paths, a
separate pinned-engine measurement returned 564–854 ms per validation,
**4,944 ms total for seven equivalent validations**, and **48 ms** for the two
binary membership loops (previously 1,278 ms). These are validation benchmarks,
not native Windows or whole-install timings; production now also avoids the
three redundant embedded-table revalidations.

Malformed/colliding embedded-input fixtures now reject at loading, before any
checkpoint or copying, rather than mutating supposedly immutable cached
tables after initialization. Wrong installed bytes, forged installed/temporary
metadata, repair-floor, cancellation and incomplete-completion cases remain.

Evidence:
- Red scale regression:
  `/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_1994411e5001b8fTTlWjlaxKO2.out`.
- Actual-payload measurement:
  `/tmp/opencode/packaging-follow-up-actual-scale.json`.

Independent read-only scoped review accepted all follow-up fixes with no
actionable findings. Its lightweight algorithm model matched the original
collision policy for 1,820 lists and stable sorting for 1,625 tagged/untagged
cases; this model check is separate from the parent Pascal-engine results.

## Fresh Disclosure And Static Verification

All four targets regenerated successfully offline under the stricter author
placeholder vocabulary. The complete licence trees match the previously
audited originals byte-for-byte. Every inventoried notice, locked source
archive hash, applicable supplement/binding, raw scope and complete-original
HTML was checked. Counts remain unchanged:

| Target | Packages | Notice paths | Supplements | Source archives |
| --- | ---: | ---: | ---: | ---: |
| Linux x86_64 | 583 | 1169 | 48 | 3 |
| Linux ARM64 | 580 | 1163 | 48 | 3 |
| macOS ARM64 | 498 | 932 | 67 | 2 |
| Windows x86_64 | 483 | 910 | 29 | 2 |

Fresh outputs and audit evidence:
`/tmp/opencode/packaging-follow-up-notice-audit.json`.
Both-architecture flake evaluation, x86_64 source-policy/transitive isolation
and desktop-loader checks passed. No application binaries were rebuilt.

All five PowerShell helper suites passed (process 11, signing 8, trust 14),
with real Windows store APIs stubbed/guarded. All nine PowerShell files parse;
both workflows pass actionlint/ShellCheck. All 29 Python files parse with
Python 3.11 grammar; all 79 supplemental lock bindings and 17 exact originals
verify. No native signing or trust-store operation was performed.

## Integrated Verification And Handoff

The first integrated run completed 264 tests with two failing subcases in the
invalid-embedded-input fixture. Production correctly rejected both inputs in
`InitializeSetup` before copying/checkpointing, but fixture teardown expanded
the not-yet-initialized application constant and failed to write its report.
Teardown now uses its independently known owned fixture path. A subsequent
compile caught a nested-brace comment typo, which was corrected. The focused
two invalid-input cases then passed in 28.721 seconds. Production code is
unchanged by this fixture correction. Independent re-review accepted the
fixture correction without new findings.

The final integrated rerun passed **264 tests in 238.263 seconds, no skips**,
including Ruby, FPC and all ten actual Inno-engine tests. The final scale
passes were 820–1,003 ms for 1,000 paths. The production definition compiled
with output disabled only; no production installer was signed or generated.
All follow-up implementation, regression, scoped-review and local verification
checkpoints are complete.

First integrated evidence:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_1994a6054001S6gk1lIEUEEExG.out`.
Final rerun output:
`/home/vhodges/.local/share/opencode/shell/e43aca20d3f93876ea1415f1cf6b174b61edefd3/sh_1994fbb99001WKmyF3AycZWKpn.out`.

Final handoff retains the existing worktree and branch-only commit/push scope.
Native Windows signing/install/upgrade/repair/uninstall/junction/dialog and
Homebrew acceptance remain unperformed; the guide now includes the native
engine-suite command and explicit first-run checks. Existing Rust and
application-build evidence remains historical: no Rust source/dependency
change, full Rust rerun or application rebuild is part of this follow-up.
Main, existing tags, licence originals/approvals, signing identity, credentials,
native workflows, releases and the external tap are untouched.
