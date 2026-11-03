# Packaging Review Follow-Up Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix redirected-profile installation and real-payload validation cost, close author-template recognition gaps, and document the accepted recovered-panic limitation.

**Architecture:** Bound filesystem checks at the application root while retaining fresh destination validation. Validate immutable embedded expectations once; use a sorted directory/file-prefix index and binary membership lookup without changing digest order or completion safety.

**Tech Stack:** Python unittest, Inno Setup 7.1.1/Pascal Script, FPC, Wine/Xvfb, PowerShell and offline Nix disclosure generation.

---

The paired design is approved, including the owner's explicit documentation-
only choice for recovered panics. Execute directly in this existing worktree;
no delegation or competing Rust/application builds are needed.

## Task 1: Reparse Boundary

**Files:** `packaging/windows/installer-managed.iss`,
`scripts/release/tests/installer_lifecycle.iss`,
`scripts/release/tests/test_installer_engine.py`.

1. Add actual-engine junction fixtures above, at and below the application
   root, including fresh/upgrade success through an ancestor junction. Keep
   original reparse rejection and external-file sentinels.
2. Run the new engine cases against current code; observe ancestor-junction
   installation rejection as the expected red failure.
3. Normalize the root/path, require equal-root or separator-delimited
   descendant membership, and stop ancestor traversal after checking the
   root. Do not weaken owned-tree or per-file checks.
4. Rerun the new cases and existing lifecycle/content fixtures; require green
   with the existing checkpoint/floor/42-or-0 assertions intact.

## Task 2: Efficient Validation And Immutable Tables

**Files:** `packaging/windows/installer-managed-policy.pas`,
`packaging/windows/installer-managed.iss`,
`scripts/release/tests/installer_managed_cases.pas`,
`scripts/release/tests/installer_policy_fpc.pas`,
new `scripts/release/tests/installer_scale.iss`,
`scripts/release/tests/installer_lifecycle.iss`,
`scripts/release/tests/test_installer_engine.py`.

1. Add shared policy cases for non-adjacent whole-path file/dir collisions,
   directory-case aliases separated by siblings, duplicates, valid siblings,
   unordered inputs, and binary-lookup misses/boundaries.
2. Add a pinned-engine scale fixture using canonical 1,000-entry file/digest
   inputs and multiple validation passes. Record timing in an owned result
   file; use a generous per-pass budget below the measured 13.877-second
   baseline and a comparison-count bound if needed to avoid machine-speed
   dependence. Observe the performance regression fail before implementation.
3. Implement stable bottom-up merge sorting for strings or tagged prefix
   nodes. Emit `D + directory-prefix` / `F + full-file` nodes after individual
   validation, sort by `CompareText` of the path portion, and accept repeated
   equal nodes only when their raw spellings match and both are directories.
4. Keep the incoming path/digest arrays in their original order. Create a
   sorted path-only lookup after successful input loading; use binary search
   for both pruning membership loops. Validate embedded inputs once, while
   retaining fresh enumeration/path validation for destination snapshots.
5. Move malformed/colliding embedded-input fixtures to loading/preparation
   rejection, instead of mutating supposedly immutable cached tables after
   initialization. Keep completion tampering and installed hash regressions.
6. Run focused Python/FPC tests and all actual-engine fixtures; benchmark the
   actual 920-path manifest and 1,000 entries against the recorded baseline.

Example focused commands (use fresh owned Wine prefixes for engine runs):

```sh
nix shell nixpkgs#fpc --command python3 -B -m unittest discover -s scripts/release/tests -p test_installer.py -v
nix shell nixpkgs#wineWow64Packages.stable nixpkgs#xvfb-run --command env INNO_COMPILER=/tmp/opencode/manyhands-inno-7.1.1/ISCC.exe MANYHANDS_INNO_WINEPREFIX=/tmp/opencode/manyhands-follow-up-engine-red WINEDLLOVERRIDES=mscoree,mshtml= xvfb-run -a python3 -B scripts/release/tests/test_installer_engine.py
```

## Task 3: Author Placeholder Labels

**Files:** `scripts/release/notices.py`,
`scripts/release/tests/test_notices.py`.

1. Add single/double-curly, angle/square, singular/plural, multiline and
   case-insensitive author-template regressions through named attribution,
   complete grants and existing canonical/scoped/supplemental paths.
2. Run `python3 -B -m unittest discover -s scripts/release/tests -p test_notices.py -v`;
   confirm failures show accepted unresolved templates.
3. Add only `authors?` to `ATTRIBUTION_PLACEHOLDERS`; retain ordinary author
   names and all original/binding/approval invariants. Rerun to green.

## Task 4: Documentation And Integrated Verification

**Files:** `packaging/README.md`, `packaging/windows/SIGNING.md` if relevant,
`docs/plans/2026-11-01-phase-ii-packaging-pr.md`, new
`docs/plans/2026-11-01-packaging-review-follow-up-execution.md`.

1. Document the selected recovered-panic limitation without Rust changes.
   Describe the corrected reparse boundary and once-validated embedded
   tables, retaining native acceptance caveats.
2. Run the full release suite with Ruby/FPC/PowerShell/pinned Inno available;
   repeat all five PowerShell helper suites, workflow actionlint/ShellCheck,
   Python 3.11 grammar, supplemental bindings/original hashes and whitespace.
3. Regenerate target disclosures offline for all four targets using the
   existing locked Nix disclosure harness. Verify original/source hashes,
   scopes, applicable records and full-original HTML; do not relabel old
   outputs as verification of the new regex.
4. Recheck current flake evaluation/source isolation as relevant. Do not run
   competing full Rust builds or claim current application rebuilds: no Rust
   source/dependency change is included, and native acceptance remains pending.
5. Record final results and inspect the complete diff and main compatibility.
   Keep worktree/branch and all release/security boundaries; no main merge,
   workflow dispatch or publication.
