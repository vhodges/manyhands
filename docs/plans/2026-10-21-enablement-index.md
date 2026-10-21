# Enablement Git Index Implementation Plan

> **For Claude:** Use the executing-plans and test-driven-development skills to implement this plan task-by-task.

**Goal:** Create and enable leave `.manyhands/config.toml` current in the Git index, preserving unrelated user changes.

**Architecture:** Reuse the save fix's locked, path-specific index publication from committed blobs. Support missing indexes for new repositories and conservative repair on domain retries. Preserve read-only finished F2 replay and existing result contracts.

**Tech Stack:** Rust, git2, existing repository lifecycle/F2 request records, Devenv.

## Approved design

Owner approved the chat design and implementation with TDD after investigation
comment `01M5E3V4YT69CJ47ZPCMZDKCWJ`. A post-commit index-write failure follows
save's best-effort policy: the commit and existing outcome remain authoritative,
foreign locks and unrelated state remain intact, and an eligible domain retry
can repair the missing configuration entry. Finished F2 replay is read-only.
No new public outcome, effect or schema is intended. `.git/config` locking is
excluded. Existing clean-worktree admission policy is preserved.

## Task 1: Reproduce and fix successful initialization

Files: `tests/repository_enablement.rs`,
`tests/mutation_confirmation.rs`, `src/repository.rs`.

1. Add regression tests for absent/empty create and born/unborn enable. Assert
   the on-disk index entry's blob and mode equal HEAD and path status is clean.
   Exercise ticket create and remote add/select without staging configuration.
2. Run `devenv shell -- cargo test --locked --test repository_enablement enablement_index`
   and `devenv shell -- cargo test --locked --test mutation_confirmation enablement_index`.
   Expected red: missing entry or `worktree_not_clean`.
3. Extend `refresh_owned_index` to initialize an empty detached snapshot when
   the live index is absent. Continue locking before reading; preserve the
   existing index timestamp when present.
4. Publish only the configuration entry from the initialization commit after
   the commit succeeds and outside the rollback closure, before lifecycle
   handoff. Retain best-effort reporting.
5. Repeat the targeted commands; expect green.

## Task 2: Retry, preservation and failure boundaries

Files: `tests/repository_enablement.rs`,
`tests/mutation_confirmation.rs`, `src/repository.rs`.

1. Add failing recovery tests for a foreign `index.lock` and retry after it is
   removed, interruptions after initialization commit, and partial creation
   after `AfterRepositoryInitialization`.
2. Add preservation assertions for unrelated staging/unstaged files introduced
   after validation; commit only configuration. Test HEAD advancement and
   later configuration staging so repair cannot regress their contents.
3. Implement conservative missing-entry repair for eligible domain enable
   paths using the currently committed snapshot, retaining unrelated staging
   and avoiding arbitrary repair of user-modified configuration entries.
4. Verify completed F2 replay leaves clean initialization state alone, returns
   the same stored result and preserves later staged configuration edits.
5. Run the two targeted commands above plus relevant existing save-index tests
   (`devenv shell -- cargo test --locked --test local_authoring index`).
6. Record TDD evidence and any refinements as a ticket comment generated with
   `devenv shell -- cargo run --locked --bin manyhands-cli -- id new`.

## Task 3: Remove workaround dependencies

Files: `tests/support/items.rs`, `tests/discovery_rebuild.rs`,
`src/repository/comment_publication_tests.rs`,
`src/desktop/test_support/fixtures.rs`, `tests/mutation_confirmation.rs`,
`tests/repository_enablement.rs`, `tests/recovery_foundation_gate.rs`.

1. Remove enable-only configuration re-adds and the desktop RepoCreate reset.
   Audit additional fixture staging; retain intentional dirty-state setup and
   publication-specific repair where needed, with a reason.
2. Make the missing-index negative publication test explicitly remove the
   entry. Keep its existing refusal expectation.
3. Run targeted affected test targets through Devenv. Resolve only regressions
   attributable to changed initialization state; record evidence.

## Task 4: Verification and review

1. Fetch main; if the registration fix has been published, merge it with owner
   authorization before final gates. Preserve its registration behavior.
2. Run once at the end:
   - `devenv shell -- cargo check --all-features --locked`
   - `devenv shell -- cargo fmt --check`
   - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
   - `devenv shell -- cargo test --all-features --locked`
   - `devenv shell -- cargo check --locked --features desktop --bin manyhands`
   - `devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings`
3. Run `git merge-tree --write-tree HEAD origin/main` before review-ready.
4. Obtain independent review, address findings with targeted verification,
   and record results. Request owner approval and publication authorization.
5. Close the ticket only after review/owner approval in the final authorized
   push before merge. Native CI needs separate owner approval after merge.

Implementation proceeds in this session. No push, PR, integration merge or
cleanup is authorized. Commit messages, when commits are authorized, end with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
