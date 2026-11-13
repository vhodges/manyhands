# Windows Verification Trust Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove current-user-root verification imports from release automation while preserving exact identity checks and owned cleanup.

**Architecture:** Pure policy selection plus real Windows token/context discovery guard machine-root trust. Shared import/removal helpers track an exact owned certificate path through both signing modes; portable tests stub only store/context boundaries, with a gated native fixture registered before secrets.

**Tech Stack:** PowerShell 7, Windows PKI cmdlets, .NET X509/WindowsPrincipal, Python unittest, Devenv, GitHub Actions.

---

## Starting Point And Authorization

Work in `.worktrees/release-windows-trust` on `release-windows-trust`, based on
`b411e208` after fetch/merge of origin/main. Follow the
[approved design](2026-10-31-windows-verification-trust-design.md). At planning,
only the patch was authorized, so edits were left uncommitted for review. The
owner subsequently approved committing/merging into main, pushing origin, and
preparing/pushing a new matching release tag. Workflow dispatch is not included.
All Rust tooling goes through Devenv; no Rust behavior changes are required.
Run Devenv invocations sequentially.

## Task 1: Guard And Own Temporary Verification Trust

**Files:** `scripts/release/windows-tools.psm1`, new
`scripts/release/tests/test_windows_trust.ps1`.

1. Establish the existing 16 focused Python, DLL/endpoint, 11 process, and five
   signature-progress tests as a clean baseline.
2. Add failing PowerShell cases for hosted Windows administrator policy and
   exact machine-root selection. Exercise the real import/removal helper bodies
   with stubbed context/store APIs and a real ephemeral public certificate:
   existing trust preserved, ownership recorded before import, partial failure
   cleaned, wrong/public-private identity rejected before mutation, wrong import
   result rejected, and removal failure propagated. Observe expected failures.
3. Implement `Get-VerificationTrustStore` (pure policy) and
   `Get-HostedVerificationTrustStore` (actual OS/environment/elevation check).
   Implement `Add-VerificationTrust(CertificatePath, Thumbprint, [ref] OwnedPath)`
   and `Remove-VerificationTrust(OwnedPath)` with fixed machine-root scope,
   public-certificate validation, before-import ownership, preservation, exact
   cleanup path validation, and checked postconditions. Export the helpers.
4. Rerun until green. Add a real Windows-only hosted/admin fixture for import,
   preserving pre-existing trust, and removal in caller `finally`. Never read
   publisher secrets or write fixture private keys to disk.

## Task 2: Integrate Both Modes And Native Test Registration

**Files:** `scripts/release/windows-sign.ps1`,
`scripts/release/tests/test_workflow.py`, `.github/workflows/release.yml`.

1. Add a failing registration assertion for the new suite and a ten-minute
   helper-test step backstop. Add portable script-wiring checks for early
   preflight, shared guarded import, owned cleanup, and no current-user root.
2. Run the real context preflight before PFX handling in the signing script.
   Replace the inline root import with the shared import helper; call exact
   owned removal from `finally`, accumulating cleanup errors while continuing
   private-key/file cleanup. Do not change signature/timestamp verification.
3. Register the new suite before secrets and add its helper-step backstop.
   Rerun portable tests and existing process/signature tests.

## Task 3: Documentation, Review, And Verification

**Files:** `packaging/windows/SIGNING.md`,
`docs/plans/2026-10-30-release-workflow-design.md`,
`docs/plans/2026-10-30-release-workflow-execution.md`.

1. Document runner-only machine trust, early failure outside the allowed
   context, unchanged private-key location, owned cleanup, fixture scope, and
   native checks still pending. Preserve historical execution findings.
2. Run affected Python tests and all four PowerShell suites through Devenv.
   Parse PowerShell and inline Windows workflow bodies, run actionlint with
   ShellCheck, and check tracked/new-file whitespace.
3. Request scoped read-only review, address concrete findings with regressions,
   fetch/merge main and check mergeability. Record actual evidence and leave
   the branch/worktree intact for owner integration instructions.

## Execution Status

All tasks are implemented and locally verified. The new portable trust suite
passes 14 cases; the 16 focused Python tests, existing PowerShell suites,
syntax checks, workflow lint, and whitespace checks pass. Scoped read-only
review reported no actionable issues. See the
[execution record](2026-10-30-release-workflow-execution.md#v015-native-trial--hosted-machine-root-verification-trust)
for red/green evidence and native limitations. The real Windows trust fixture
is registered but not run on Linux. At review edits were uncommitted; the
owner then approved committing/merging, pushing main, and preparing/pushing
`v0.1.6` with matching Cargo metadata and CLI version fixtures. Native workflow
dispatch remains separately authorized; publisher identity and secrets stay
unchanged, and existing tags must not move.
