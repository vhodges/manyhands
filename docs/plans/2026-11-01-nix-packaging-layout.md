# Nix Packaging Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Relocate supporting Nix files to `packaging/nix/` without changing package behavior.

**Architecture:** Keep the root flake/lock and move the supporting directory directly. Adjust relative paths and documentation; retain all package inputs, public outputs and isolation policy.

**Tech Stack:** Nix flakes, existing source-policy tests, Git.

---

The owner approved this design and execution in the current session, including
a follow-up commit/push on the existing branch. No additional worktree or
execution-choice prompt is needed.

## Task 1: Relocate And Verify

1. Add source-policy cases in `nix/tests/source-filter.nix` for the allowed
   `packaging/nix/README.md`, its ancestor directories, rejection of the old
   `nix/README.md`, and exclusion of private/implementation files beneath
   the new directory. Evaluate the policy and observe failure before editing
   the implementation.
2. Move the six files under `nix/` to `packaging/nix/`. Update `flake.nix`
   imports, `package.nix` README installation, `source-filter.nix` README and
   supplemental-manifest paths, `checks.nix` runtime-manifest path, and the
   test's manifest path. Preserve the root lockfile and all file bytes not
   requiring a path change.
3. Update `README.md`, `packaging/README.md`, current implementation-plan
   paths and the PR description. Append the decision/results to the existing
   execution record, preserving historical paths/store evidence.
4. Mark new files intent-to-add for Git-flake evaluation. Run
   `nix flake check --all-systems --no-build`, then
   `nix build .#checks.x86_64-linux.source-policy .#checks.x86_64-linux.desktop-loaders --no-link --cores 2 --max-jobs 1`.
   Expect successful evaluation/check builds.
5. Compare old/new production derivations and their filtered source trees:
   only the guide's source path, root README link and corresponding install
   command may differ after normalizing source/output store paths. Do not
   label this as a new application build or runtime test.
6. Run the portable packaging suite and whitespace checks, review the scoped
   diff, commit the relocation and documentation, and push the existing
   branch. Preserve the worktree and browser PR handoff.
