# Ignored Files Do Not Block Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A context worktree whose only extras are ignored files is cleaned up by ticket closure, document promotion, `context release` and `context retire`, and untracked, modified, hidden-index, submodule, locked and foreign-registered worktrees still block.

**Architecture:** `require_cleanup_clean` in `src/repository/cleanup.rs` is the one precheck all four use. It stops asking libgit2 for ignored entries, so nothing produces `Precheck::IgnoredFiles` any more, and that variant and its three arms in `promotion/run.rs` go. The stored category `ignored_files` stays readable (stored rows, CHECK constraints, `PromotionCleanupPending::IgnoredFiles`, desktop copy), so no registry migration is needed.

**Tech Stack:** Rust, `git2` 0.20.4 (`worktree.prune` with `working_tree(true)` removes ignored files; checked 2026-11-17).

**Spec:** `docs/plans/2026-11-17-ignored-files-cleanup-design.md`. This plan amends one line of it: only the producers of `ignored_files` are removed; `Clean::IgnoredFiles` is the stored `CleanupCategory::IgnoredFiles`, `PromotionCleanupPending::IgnoredFiles` maps from it, and both stay so a row an older build wrote can be read and retried.

## Global Constraints

- Run Cargo only as `devenv shell -- cargo ...`, from this worktree.
- AGENTS.md Testing Strategy: between steps run `cargo check` and only the tests you wrote or changed, by exact name (`--lib <module>::<test>` or `--test <target> <name> -- --exact`). Never run a whole SSH target between steps. The fast tier and the SSH tier run once, in Task 4.
- Stop and narrow the filter if a single command takes more than 15 minutes.
- No `sleep` in tests. Library unit tests first; the SSH harness is for transport behaviour only, and this ticket gets at most one SSH case (Task 3).
- No registry migration, and no change to the `cleanup_category` CHECK constraints.
- Commit after each task and push the ticket branch (`git push origin HEAD`). Commit messages end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Never run `manyhands-cli` or the desktop built from this branch against real app data.

## Review Focus

1. **A modified tracked file that matches an ignore pattern** still blocks (Git reports it modified, not ignored). Pinned in Task 1.
2. **An untracked, non-ignored file next to ignored ones** still blocks, and nothing is removed. Pinned in Task 1.
3. **Ignored files of awkward shapes**: a read-only file, a dangling symlink, a nested directory, all pass and are removed with the worktree. Pinned in Task 1.
4. **A row an older build left at `cleanup_category = 'ignored_files'`** finishes when its request is resent. Pinned in Task 2.
5. **Ignored directory holding a nested Git repository** (for example a clone under an ignored `vendor/`): it must still block, so unpushed commits in it are not deleted with the worktree. Owner decision 2026-11-17. Pinned in Task 1a. An ignored directory that cannot be read blocks too (fail closed).

---

### Task 1: The precheck ignores ignored files

**Files:**
- Modify: `src/repository/cleanup.rs` (`Precheck` enum near line 15; `require_cleanup_clean` status options and result near lines 570-592)
- Modify: `src/repository/promotion/run.rs` (the three `Precheck::IgnoredFiles => return Err(Clean::IgnoredFiles)` arms near lines 1263, 1343, 1436)
- Modify: `src/repository/context_release.rs` (`not_clean`, near line 472)
- Test: `src/repository/cleanup_tests.rs`

**Interfaces:**
- Produces: `Precheck` has `Ok`, `Changed`, `Locked` only. `ContextCleanup::precheck_context` and `precheck_release` keep their signatures. `require_cleanup_clean` is unchanged apart from what it reports.

- [ ] **Step 1: Write the failing tests in `cleanup_tests.rs`.**
  - Replace `cleanup_ignored_and_locked_are_preserved` with two tests. `cleanup_locked_is_preserved` keeps the locked case unchanged. `cleanup_ignored_files_do_not_block` builds the `Fixture::new(AuthoringKind::Document)` worktree, adds `private/` to `info/exclude`, and writes into the linked worktree: `private/keep`, `private/deep/x.o`, a dangling symlink `private/dangling` to `/nonexistent`, and a read-only (`0o444`) `private/ro.bin`. Assert `f.precheck(&c) == Precheck::Ok` and `f.precheck_release(...)` is `ReleaseCheck::Clean` (call it through the fixture the way `precheck` is called; add a small `precheck_release` helper next to `precheck` if there is none), then `f.remove(&c) == WorktreeRemoval::Removed` and the worktree directory and registration are gone.
  - Add table-driven `cleanup_ignored_files_beside_other_changes_still_block` over these cases, each with an ignored `private/keep` present, each asserting `Precheck::Changed`, `files(f.root.path())` unchanged, and `remove` not `Removed`: an untracked non-ignored `new` file; a modified tracked file that matches an ignore pattern (add its path to `info/exclude` after it is committed, then edit it); a staged new file.
  - Rename `cleanup_submodule_ignored_files_are_preserved_recursively` to `cleanup_submodule_ignored_files_do_not_block`, keep its fixture for depth 1 and 2, and assert `f.precheck(&c) == Precheck::Ok` and that `f.remove(&c)` removes only the owned worktree as `cleanup_submodule_clean_recursive_control_preserves_external_git_metadata` already checks for the clean control (the submodule's Git metadata outside the removal roots must survive; if removal is refused for that existing reason, assert the same refusal that test asserts).
- [ ] **Step 2: Run them and confirm they fail.** `devenv shell -- cargo test --lib --locked repository::cleanup_tests::cleanup_ignored_files_do_not_block -- --exact` (and the other two names). Expected: FAIL, `left: IgnoredFiles right: Ok`.
- [ ] **Step 3: Implement.** In `require_cleanup_clean` drop `.include_ignored(true)` and `.recurse_ignored_dirs(true)` and return `Precheck::Changed` for any remaining status entry (delete the `IGNORED` branch). Remove `Precheck::IgnoredFiles` and the three `run.rs` arms. In `not_clean` change the message to "the context worktree holds a change or an untracked file". The `Finding`/`ReleaseCheck` doc comments that name ignored files get the same edit.
- [ ] **Step 4: Run the three tests, then `devenv shell -- cargo check --all-features --locked`.** Expected: PASS, and a clean check (no remaining reference to `Precheck::IgnoredFiles`).
- [ ] **Step 5: Commit** `fix(cleanup): ignored files no longer block a context's cleanup`.

### Task 1a: An ignored directory holding a nested repository still blocks

**Files:**
- Modify: `src/repository/cleanup.rs` (`require_cleanup_clean`, after the status check; one new private function)
- Test: `src/repository/cleanup_tests.rs`

**Interfaces:**
- Consumes: Task 1's `require_cleanup_clean`.
- Produces: `fn ignored_dirs_hold_no_repository(repository: &Repository) -> Result<(), Precheck>` in `cleanup.rs`, called by `require_cleanup_clean` for the repository it checks (each submodule too, as the recursion already calls it). It returns `Err(Precheck::Changed)` when an ignored directory contains an entry named `.git` (file or directory) at any depth, or when a directory in it cannot be read.

- [ ] **Step 1: Write the failing test `cleanup_ignored_directories_holding_a_repository_or_unreadable_block`** in `cleanup_tests.rs`, table-driven over: an ignored `vendor/` with a nested `git2::Repository::init` (a `.git` directory); an ignored `vendor/` with a `.git` file (a gitfile); a `.git` three directories deep under `target/`; an ignored directory with a subdirectory set to mode `0o000` (restore the mode at the end of the case so the tempdir cleans up). Each row asserts `Precheck::Changed`, `files(f.root.path())` unchanged and `f.remove(&c)` not `Removed`. Add the control row: an ignored directory with nested plain files and no `.git` is `Precheck::Ok` (Task 1's test already covers it; do not duplicate it, just keep this table's rows to the blocked cases).
- [ ] **Step 2: Run it by exact name and confirm it fails** (`left: Ok right: Changed`).
- [ ] **Step 3: Implement `ignored_dirs_hold_no_repository`.** Ask libgit2 for statuses with `include_ignored(true)` and `recurse_ignored_dirs(false)`, so an ignored directory is one entry. For each ignored entry that is a directory, walk it with `std::fs::read_dir` and `symlink_metadata`, never following symlinks, iteratively with a stack, and return `Changed` on an entry named `.git` or on any I/O error. The worktree root's own `.git` is outside the ignored entries, so it is never seen. Call it from `require_cleanup_clean` right after the untracked-status check passes.
- [ ] **Step 4: Run the test, then Task 1's three tests again, then `devenv shell -- cargo check --all-features --locked`.** Expected: PASS.
- [ ] **Step 5: Commit** `fix(cleanup): an ignored directory holding a repository still blocks`.

### Task 2: Library tests of promotion and closure follow, and an old row resumes

**Files:**
- Modify: `src/repository/promotion/run_tests.rs` (the two cases that expect `PromotionCleanupPending::IgnoredFiles`, near lines 2275 and 5048)
- Test: `src/repository/promotion/run_tests.rs`

**Interfaces:**
- Consumes: Task 1's precheck.

- [ ] **Step 1: Change the two blocked-by-ignored cases** (the `ignored` flag near 2275 and `mode == 3` near 5048). They must no longer expect `Pending { IgnoredFiles }`. Remove the ignored variants from the preserve-and-supersede tables. Keep their other variants unchanged.
- [ ] **Step 2: Write `an_ignored_file_does_not_stop_the_cleanup_of_a_run`** in the same file, in the style of the neighbouring cases: run a promotion with an ignored file in the context worktree and assert `result.cleanup() == PromotionCleanup::Complete` and that the context worktree no longer exists.
- [ ] **Step 3: Write `a_cleanup_left_at_ignored_files_by_an_older_build_finishes_on_resend`**: stop a run after publication (the fixtures stop at a `FailurePoint`), set the stored row's `cleanup_category` to `'ignored_files'` with an `UPDATE` on the promotion table, add an ignored file to the worktree, resend the same request, and assert `PromotionCleanup::Complete`.
- [ ] **Step 4: Run the three tests by exact name.** Expected: PASS. Run `devenv shell -- cargo test --lib --locked repository::promotion::record_tests::supersession_only_the_three_approved_cleanup_categories repository::closure::record_tests::supersession_only_from_context_changed_ignored_remote_changed -- --exact` once to confirm stored-category behaviour is unchanged; these two must still pass without edits.
- [ ] **Step 5: Commit** `test(promotion): ignored files complete a cleanup, and an old ignored_files row resumes`.

### Task 3: The SSH-harness cases follow, with one end-to-end case

**Files:**
- Modify: `tests/document_promotion/journeys.rs` (entry `preserve_ignored_then_supersede`, line 67, and `preservation` mode 2 near lines 686-740)
- Modify: `tests/ticket_closure/journeys.rs` (the matching mode-2 entry near line 63 and `preservation` mode 2 near lines 840-900)
- Modify: `tests/mutation_remote/c5_lifecycle.rs` (`cleanup_ignored_files`, near line 1524, and its table entry at line 55)

**Interfaces:**
- Consumes: Task 1.

- [ ] **Step 1: Delete mode 2** from both `preservation` functions, its table entry, and the `mode == 2` branch of the `reason:` expression, leaving modes 0, 1 and 3. Renumber nothing that other code depends on: if the table passes mode numbers, keep 3 as 3 and drop 2.
- [ ] **Step 2: Rewrite `cleanup_ignored_files` in `c5_lifecycle.rs`** as the one end-to-end case. Keep its first lines (stop at `FailurePoint::AfterPublication`, write `fixture-ignored` and its `info/exclude` entry). Resend the same request and assert `done.code == ResultCode::Ok` and that `verb.worktree(&w.root)` no longer exists. Update its doc comment to say ignored files do not stop the cleanup. Drop the assertions for `Partial` / `IgnoredFiles` and the second resend.
- [ ] **Step 3: Run each changed case by exact name** (one case each, no whole target): the SSH harness takes `--test mutation_remote cleanup_ignored_files -- --exact` (use the case name the table registers; check the harness's list with `--list` first), and the two modified `preservation` entries by their table names. Expected: PASS. Stop at 15 minutes.
- [ ] **Step 4: Commit** `test(ssh): the cleanup-with-ignored-files case completes`.

### Task 4: Docs, ticket record, and the review-ready gates

**Files:**
- Modify: `docs/plans/2026-11-17-ignored-files-cleanup-design.md` (the "Design" section: only producers go; stored category and `PromotionCleanupPending::IgnoredFiles` stay)

- [ ] **Step 1: Amend the design doc** to match the "Spec" paragraph above and commit.
- [ ] **Step 2: Add a comment to the ticket** (`.manyhands/comments/01M7MKQFCHPEXQ58N77D63S417/<id>.md`, via `manyhands-cli comment add` from main only if the CLI on main is current; otherwise a canonical comment file) recording which tests ran at each checkpoint. One line per paragraph, no hard wrap.
- [ ] **Step 3: Static gates over the whole tree.** `devenv shell -- cargo check --all-features --locked`, `devenv shell -- cargo fmt --check`, `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. Expected: all clean.
- [ ] **Step 4: Fast tier**, exactly the command in AGENTS.md Development step 1. Expected: PASS.
- [ ] **Step 5: SSH tier, once.** Changed paths: `src/repository/cleanup.rs` (shared by retirement, so not in AGENTS.md's table: run it anyway), `src/repository/promotion/` and a `tests/` target each. Run `--test document_promotion`, `--test ticket_closure`, `--test mutation_remote`, `--test remote_polling`, and `--lib repository::context_retirement::`. Expected: PASS. Record each in the ticket.
- [ ] **Step 6: Merge `origin/main`** into the branch (`git fetch && git merge origin/main`, never rebase), check `git merge-tree --write-tree HEAD origin/main`, push, and mark the ticket review-ready in a comment.
