# Cross-Channel Disclosure Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve exact, offline original dependency disclosures across all channels without hiding unresolved source-licensing conditions.

**Architecture:** Keep locked dependencies unchanged. Apply checksum-/version-bound supplemental originals and source-member notices through the shared generator, validating any grant-only provenance narrowly. Preserve separately licensed protocol inputs while retaining the owner-review distribution hold.

**Tech Stack:** Python 3.11+, pinned cargo-about 0.9.3, Cargo.lock, Nix flakes and Devenv; original immutable upstream text inputs.

---

## Execution Rules

Continue in the existing `phase-ii-packaging` worktree, preserving accumulated
edits. The owner initially approved data-first execution without commits.
On 2026-11-01 the owner separately authorized finishing local verification,
committing, pushing `phase-ii-packaging`, and opening a PR. That supersedes
this plan's original leave-uncommitted instruction, but does not authorize
merge, tags, workflow dispatch, publication or external tap changes. No
dependency or licence-selection changes. Editor feasibility is outside scope.

### Task 1: Complete Original Source-Member Harvesting

**Files:** `scripts/release/notices.py`, `scripts/release/tests/test_notices.py`.

1. Add regressions for `AUTHORS` referenced by a licence, arbitrary filenames
   under `.licenses`/licence directories, and XML copyright blocks. Run
   `nix shell nixpkgs#python3 -c python3 -B -m unittest discover -s scripts/release/tests -p test_notices.py`
   and verify the new cases fail before changing harvesting.
2. Expand the existing preserved-notice predicate without flattening paths
   or following symlinks. Treat embedded notices as original source material,
   not evidence that all source code is linked. Preserve exact file bytes.
3. Run the same regressions and existing notice suite to green. Check the
   real granit-parser and sha1_smol members against verified audit evidence.

### Task 2: Retain Reviewed Supplemental Originals

**Files:** `packaging/licenses/supplemental.json`,
`packaging/licenses/supplemental/*`, `nix/source-filter.nix`,
`nix/tests/source-filter.nix`, `scripts/release/tests/test_notices.py`.

1. Add failing tests for full reviewed input coverage, exact Cargo.lock
   checksums, text SHA-256, immutable URLs/source linkage, CLI exclusion,
   missing/unsafe files, and unknown evidence kinds.
2. Populate original texts from the verified audit/provenance maps, not from
   current HEAD. Preserve distinct source-member copyright headers where
   full upstream licence files alone are insufficient. Do not select later
   malloc_buf/SeaHash notices merely to make a check green.
3. Record grant-only source facts separately from named copyright notices;
   keep any unproved applicability explicitly unresolved. Enumerate safe
   supplemental filenames in the desktop source filter, excluding the whole
   set from the CLI input while no CLI dependency needs supplementation.
4. Verify every byte hash and rerun Python/source-filter tests.

### Task 3: Validate And Render Resolutions Without Blanket Exceptions

**Files:** `scripts/release/notices.py`, `scripts/release/tests/test_notices.py`.

1. Add failing tests for exact reviewed grant-only evidence, a stale crate
   checksum, missing supporting evidence, invented/canonical holders,
   unresolved conditions, and complete HTML/inventory provenance.
2. Resolve only matching immutable inputs. Preserve original texts and
   explanations in HTML and inventory. Continue rejecting unknown/unresolved
   canonical fallbacks, aggregate their package/version diagnostics, and
   leave no final output after failure. No network fallback.
3. Run targeted tests and real graph audits; report unresolved cases instead
   of suppressing them or treating copyright-mention boilerplate as a holder.

### Task 4: Preserve Separately Licensed Protocol Material

**Files:** `nix/package.nix`, `scripts/release/notices.py`, notice tests,
`packaging/licenses/README.md`, `nix/README.md`, `packaging/README.md`.

1. Test preservation/checksums of the original Plasma source crate and XML/
   LGPL notices and exclusion from unrelated CLI/target graphs.
2. Supply its original `.crate` checksum as a Nix fixed input and retain its
   full separately licensed source notices. Clearly distinguish protocol
   definition provenance from the unresolved generated-binding decision.
3. Do not alter `about.toml` to grant a blanket exemption or claim the
   application is cleared for Linux desktop binary redistribution. Record
   the required owner decision, including the objc2 original caveat.

### Task 5: Regenerate And Review

**Files:** the existing execution record and consumer/operations guides.

1. Regenerate actual notices offline for all four native target graphs with
   pinned tools; use available checksum-verified source caches/inputs only.
   A graph failure is a blocker, not a successful complete bundle.
2. Run `nix shell nixpkgs#ruby nixpkgs#fpc nixpkgs#python3 -c python3 -B -m unittest discover -s scripts/release/tests`,
   workflow actionlint/ShellCheck, Python 3.11 grammar parsing, and
   `git diff --check`.
3. Run `nix flake check --all-systems --no-build`; build current x86_64 CLI,
   desktop and installed checks where unblocked. ARM64 evaluation is not a
   build; do not run native workflows or claim Wayland acceptance.
4. Request scoped read-only review of supplemental applicability, unsafe
   input rejection, licence/source completeness, graph isolation, and the
   protocol hold. Correct findings with red/green tests.
5. Record actual results and unresolved owner decisions. Complete the
   authorized commit/push/PR handoff after verification, without merge,
   tags, external contact, publication, or signing changes.
