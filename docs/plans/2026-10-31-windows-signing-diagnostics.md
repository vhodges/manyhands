# Windows Signing Diagnostics Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the next Windows signing stall identify its operation and fail within bounded time without weakening release verification.

**Architecture:** A PowerShell information-stream progress helper marks signing boundaries. The existing native-tool wrapper becomes a shell-free .NET process runner with concurrent output drains and a 180-second deadline; ten-minute workflow-step limits cover in-process cmdlets.

**Tech Stack:** PowerShell 7/.NET, Python standard-library fixtures and unittest, GitHub Actions, Devenv, actionlint.

---

## Scope And Starting Point

Follow the [approved design](2026-10-31-windows-signing-diagnostics-design.md).
Worktree `.worktrees/release-windows-signing-diagnostics` starts at
`25a8174d`, after fetching and merging current `origin/main`. No Rust behavior
or dependency changes are needed. At planning, only the patch was authorized,
so edits were left uncommitted for review. The owner subsequently selected
committing and merging into local `main`; push, version/tag preparation, and
another native run remain separately authorized operations.
Run Devenv commands sequentially to avoid its shared-state race.

## Task 1: Bound Native Tools And Preserve Their Interface

**Files:**
- Modify: `scripts/release/windows-tools.psm1`
- Create: `scripts/release/tests/test_windows_process.ps1`
- Create: `scripts/release/tests/windows_process_fixture.py`

1. Establish baseline Windows Python and PowerShell policy tests.
2. Add real child-process tests: argument vectors (spaces/quotes/empty/Unicode),
   separate large stdout/stderr, failure diagnostics, default 180-second and
   rejected invalid deadlines, short hung-tree expiry, and caller cleanup.
   Test UTC progress on the information stream, separated from result strings
   and without exposing an argument sentinel. Observe expected missing-feature
   failures before changing production code; keep fixture waits bounded even
   against the old runner.
3. Add/export `Write-ReleaseProgress([string]$Message)`, using UTC ISO timestamps
   and `Write-Host`. Add `[ValidateRange(1, 3600)][int]$TimeoutSeconds = 180` to
   `Invoke-ReleaseTool`, preserving the existing first two positional arguments.
4. Construct `ProcessStartInfo` with `UseShellExecute = $false`, redirected
   stdout/stderr, and one `ArgumentList.Add()` per argument. Start both
   `ReadToEndAsync()` tasks before waiting. Use one stopwatch deadline for
   `WaitForExit()` and bounded output completion. On expiry, use `Kill($true)`
   for a live process tree, bounded reap, then throw `TimeoutException`.
   Nonzero exits still throw with captured stdout/stderr; dispose in `finally`.
5. Rerun the new process tests and existing policy suite. Expect success; do
   not claim native SignTool behavior from these Linux fixtures.

## Task 2: Identify Every Signing Boundary

**Files:**
- Modify: `scripts/release/windows-sign.ps1`
- Create: `scripts/release/tests/test_windows_signing.ps1`

1. Add failing tests that execute the real `Confirm-Signatures` function with
   stubbed native/Authenticode APIs. Require progress around both checks,
   preserved rejection of missing timestamps/unexpected identities, and no
   success message after rejection. Touch no Windows certificate stores.
2. Add timestamped progress around PFX loading/validation, owned store import,
   public export, runtime audit, signing/timestamping each executable, temporary
   trust import, both verification calls, metadata/inventory work, smoke test,
   and cleanup. Keep secret-bearing operations and ownership bookkeeping in
   their original scope; do not wrap them in new assignment scopes.
3. Rerun the verification-progress tests and confirm all security checks and
   cleanup failures still propagate unchanged.

## Task 3: Add Workflow Backstops And Record Evidence

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/release/tests/test_workflow.py`
- Modify: `packaging/windows/SIGNING.md`
- Modify: `docs/plans/2026-10-30-release-workflow-execution.md`

1. Add a failing workflow contract test for a ten-minute timeout on signing
   and extracted Windows verification, then add both `timeout-minutes: 10`
   fields. Keep the four-target matrix and manual/draft contract unchanged.
2. Document progress interpretation, both deadlines, cleanup on native timeout
   versus hard-cancellation limitations, the cancelled trial, and pending native
   acceptance. No certificate regeneration or unsigned fallback.
3. Run affected Python tests and all three PowerShell test scripts through
   Devenv; parse modified PowerShell and inline workflow bodies; run actionlint
   with ShellCheck for both workflows and `git diff --check`.
4. Obtain scoped read-only code review, resolve concrete findings with tests,
   fetch/merge main and check mergeability. Record actual evidence and leave
   the branch/worktree intact for the owner's integration choice.

## Execution Status

All tasks are implemented and locally verified. A review finding about
registering the new PowerShell suites in native Windows validation was
reproduced with a failing workflow test and corrected; scoped re-review found
no additional actionable issues. The
[execution record](2026-10-30-release-workflow-execution.md#v014-native-trial--bounded-signing-diagnostics)
records validation scope and native limitations. At the review checkpoint,
edits were uncommitted; the owner then authorized committing and merging into
local `main`, with focused checks repeated on the merged tree. No new
version/tag, signing identity, secrets, push, or workflow dispatch is included.
