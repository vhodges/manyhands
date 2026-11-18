# Packaging Review Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Address the eight reviewed packaging findings without weakening installation recovery or release verification.

**Architecture:** Preserve the existing channels and shared notice generator. Add bounded release operations, safe install-start/post-copy lifecycle hooks, visible Windows errors and narrow validation regressions; derive Nix checks from locked metadata and remove unjustified runtime Git dependencies.

**Tech Stack:** Python unittest, PowerShell, Inno Setup 7.1.1/Pascal fixtures, Rust/Devenv, Nix flakes.

---

The owner approved execution and follow-up commits/push in this session.
Continue in the existing worktree. Independent tasks may be delegated with
disjoint file ownership; agents do not commit or run competing full builds.

## Task 1: Installer Lifecycle And Release Budget (findings 1–3, 8)

**Files:** `packaging/windows/manyhands.iss`, `installer-completion.iss`,
new installer lifecycle/managed-file includes as needed;
`scripts/release/windows-installer.ps1`, `windows-sign.ps1`,
`windows-tools.psm1`, `installer.py`, Windows/Pascal/installer tests;
`.github/workflows/release.yml`.

1. Add failing real-function/engine regressions for explicit compiler timeout
   and mandatory retry behavior, read-only preparation, install-start failure,
   early cancellation versus mid-install repair floor, and stale-file pruning.
2. Generate a canonical incoming managed-file list only after verified-ZIP
   payload equality. Reject malformed lists, missing expected files, unsafe
   paths and reparse traversal before pruning; preserve current files and
   user state, fail completion if stale-file removal fails.
3. Checkpoint immediately before installation and prune only after successful
   copy. Retain exit 42 for incomplete completion and the repair floor after
   possible writes. Exercise the actual pinned Inno engine under isolated
   fixtures; compile production definition with output disabled, not unsigned.
4. Apply explicit time/retry budgets; remove the dead archive guard and pass
   validated timestamp configuration without duplicate reads. Keep pre/post
   compilation payload matching and exact-identity verification.
5. Run Python installer tests, PowerShell helper tests and Inno/FPC fixtures.
   Record red/green evidence and native limitations.

## Task 2: Visible Windows Desktop Failures (finding 4)

**Files:** `src/main.rs`, desktop-only startup module/tests as needed,
`Cargo.toml` Windows API feature only if required.

1. Add failing tests for startup-failure and panic reporting with fixed,
   secret-free user-facing messages and unaffected non-Windows/debug paths.
2. Add a native Windows dialog path and panic hook before fallible startup.
   Keep the CLI unchanged, avoid new dependencies or sensitive persistent logs,
   and preserve the original panic hook/exit semantics where safe.
3. Run targeted Rust tests via Devenv; parent owns whole-tree Rust gates.

## Task 3: Reject Curly-Brace Placeholder Holders (finding 5)

**Files:** `scripts/release/notices.py`, `scripts/release/tests/test_notices.py`.

1. Add failing direct and full MIT-grant cases for `{year} {fullname}` and
   `{{year}} {{fullname}}`, including multiline and scoped/bound evidence.
2. Extend only placeholder recognition; retain valid ordinary holders,
   complete-grant checks, scoped original bytes and six approved declarations.
3. Run the notice tests red/green. Do not alter any original/licence inputs.

## Task 4: Lock-Derived Checks And No Local Runtime Git (findings 6–7)

**Files:** `packaging/nix/package.nix`, `checks.nix`, policy tests;
`packaging/homebrew/manyhands.rb.in`, Homebrew tests;
`scripts/release/common.py`, `macos.py`, `windows.py`, `assemble.py` and
their tests as needed for runtime requirements/notes.

1. Add failing tests for changed locked source-package versions and absence
   of a local Git runtime requirement in casks/native metadata.
2. Derive source-check paths from exact locked records while retaining
   selected-graph coverage; remove `gitMinimal` and CLI PATH wrapping, keeping
   desktop graphics wrapping. Keep test/build tools distinct from runtime.
3. Run evaluation/source-policy tests, helper tests and an installed CLI
   repository operation with isolated HOME/XDG and a PATH without Git.

## Final Verification And Handoff

1. Integrate targeted results and update current operational docs/PR text;
   preserve historical results. Request scoped review and fix verified gaps.
2. Run the full portable helper suite, PowerShell syntax/helpers, workflow
   lint, Python grammar, exact-original hashes and whitespace checks.
3. Run Devenv `cargo fmt --check`, `cargo check --all-features --locked`,
   `cargo clippy --all-targets --all-features --locked -- -D warnings`, and
   `cargo test --all-features --locked`. Avoid overlapping Cargo invocations.
4. Evaluate both flake architectures; build current x86_64 packages/checks
   after inputs stabilize, inspect no-Git CLI closure and installed disclosures.
   Generate target disclosures as relevant; do not claim native acceptance.
5. Record results, commit logical groups and push the existing branch; keep
   worktree and browser PR handoff. No merge/release/native dispatch.
