# Windows Timestamp Transport Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Correct the SignTool timestamp endpoint rejected by Release v0.1.3 without changing publisher identity or weakening signature verification.

**Architecture:** Validate the one reviewed DigiCert HTTP RFC 3161 URL in the existing PowerShell tools module. Pass its normalized string unchanged to SignTool and signing metadata; retain all digest, timestamp, trust, and cleanup checks.

**Tech Stack:** PowerShell 7, Windows SDK SignTool, Python unittest, Devenv, actionlint.

---

## Scope And Authorization

The owner approved the focused fix in conversation. Work in
`.worktrees/release-windows-timestamp`, based on current `origin/main` at
`a52273ff`. No Rust behavior, workflow matrix, version, tag, identity, or
secrets change. Commit/push/merge and native dispatch need separate approval;
checkpoint commits are deferred until that authorization.

## Task 1: Reproduce The Endpoint Regression

**Files:**
- Modify: `scripts/release/tests/test_windows.py`
- Modify: `scripts/release/tests/test_windows_tools.ps1`

1. Establish the baseline: run the four existing Windows Python tests and
   PowerShell import-policy tests through Devenv.
2. Add a Python assertion that the checked-in endpoint is
   `http://timestamp.digicert.com`, and verify its propagation into package
   signing metadata and public signing documentation.
3. Add PowerShell tests of `Get-SignToolTimestampUrl`: accept the checked-in
   endpoint with or without a root slash and return
   `http://timestamp.digicert.com/`; reject HTTPS, another host, credentials,
   nondefault ports, paths, query strings, fragments, and malformed input.
4. Run each new regression before implementing the fix. Expect the Python
   configuration assertion to fail and the PowerShell helper to be absent.

## Task 2: Apply The Narrow Transport Correction

**Files:**
- Modify: `packaging/windows/signing.json`
- Modify: `scripts/release/windows-tools.psm1`
- Modify: `scripts/release/windows-sign.ps1`

1. Set `timestamp_url` to `http://timestamp.digicert.com`.
2. Add/export `Get-SignToolTimestampUrl([string]$Url)`. Use
   `[Uri]::TryCreate($Url, [UriKind]::Absolute, [ref]$endpoint)` and require
   `$endpoint.AbsoluteUri -ceq 'http://timestamp.digicert.com/'`; otherwise
   throw a clear reviewed-endpoint diagnostic. Return that normalized URL.
3. Replace the HTTPS-only guard with the helper call. Pass the returned
   string to `/tr` and the `timestamp_url` metadata field. Preserve `/fd
   SHA256`, `/td SHA256`, signature/timestamp checks, and cleanup.
4. Rerun the focused Python and PowerShell tests. Expect all to pass.

## Task 3: Document And Verify

**Files:**
- Modify: `packaging/windows/SIGNING.md`
- Modify: `docs/plans/2026-10-30-release-workflow-design.md`
- Modify: `docs/plans/2026-10-30-release-workflow-execution.md`

1. Document the native error, the Microsoft SignTool RFC 3161 example,
   signed-token/HTTP distinction, and preserved verification requirements.
2. Run the Windows Python tests and workflow-adapter tests through Devenv,
   and PowerShell policy tests with PowerShell supplied through Nix.
3. Parse the modified PowerShell files; run actionlint for both workflows
   and `git diff --check`. Inspect the complete diff and mergeability.
4. Record actual results, leave edits uncommitted, and report native Windows
   signing/assembly as pending a separately authorized post-merge run with
   a matching new source version/tag. Existing tags must not move.

## Execution Status

All three tasks are implemented and locally verified, with no actionable
findings from scoped read-only code review. See the
[v0.1.3 execution record](2026-10-30-release-workflow-execution.md#v013-native-trial--windows-timestamp-transport)
for red/green regression evidence, exact validation scope, and pending native
acceptance. At review the changes were uncommitted; the owner subsequently
authorized committing and merging into local `main`. Push, version/tag
preparation, and native dispatch still require separate authorization.
