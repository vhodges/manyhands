# Fast writes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each write cost about its own work: a write indexes only the contexts it changed, a whole refresh re-reads only what moved, a share uses 4–5 SSH connections, and a close or promotion previews twice and indexes once per exit.

**Architecture:** Each context's index rows come from that context alone, with a stored basis (head, branch, file stats and hashes) and a dirty mark; which copy of an item is presented is decided at read. A write's index step and a whole refresh both observe outside the lease and store under it after a basis re-check. Activity keeps today's values through short per-item walks and an incremental map for the primary. Phase 2 reuses one SSH connection per remote call and drops repeated listings, inspections and the post-push refresh; Phase 3 migrates the registry once per process, audits by use, and cuts a lifecycle's previews and refreshes.

**Tech Stack:** Rust, git2 0.20.4 (libgit2 with libssh2), rusqlite registry, serde_json, the SSH test harness in `tests/support/ssh_*`.

**Spec:** `docs/plans/2026-11-19-fast-writes-design.md` (approved 2026-11-19; decisions 1–14, rulings R-1–R-7, Tables A and B), with the planning amendments P-1–P-5 below, which the owner approves with this plan. Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts): its "How to measure" section and planning comments 1–3.

## Planning amendments to the design (for the owner's approval)

Each was found while grounding the plan in the code; the design is updated to match once approved.

- **P-1. Cleanup checks.** The full `require_cleanup_clean` check runs once, under the lease, inside worktree removal (`remove_context_worktree`, `src/repository/cleanup.rs:265-272`), as the design says. Because a close deletes the remote branch before it removes the worktree, a cheap head-and-status check of the worktree runs before the remote delete (`src/repository/promotion/run.rs` ~1340), so a dirty worktree still stops it. The other two full checks (~1261, ~1432) go. Four full checks become one full and one cheap. Two outcomes change, neither unsafe: a nested repository or unreadable directory that appears after the cheap check now stops the close after the remote branch is deleted (cleanup pending at `remote_deleted`, the work merged, the retry finishes it); and the close records `worktree_removing` before the full check refuses. Task 22 tests both.
- **P-2. Freshness on read keeps today's rule against the context's own time.** The design said reads compare content hashes. Existing tests pin "a file newer than the refresh is stale whatever it holds" (`tests/read_items.rs` `a_file_read_by_path_that_is_newer_than_the_refresh_is_stale_whatever_it_holds`), and `read/comments.rs` ~349-360 uses that rule to decide whether stored context-wide findings stand. So a read reports stale when its context is dirty or the file was modified in a later second than the context's `indexed_at` (stored in nanoseconds, compared in whole seconds as today; taken when that context's observation starts, not when a refresh starts, so an unchanged context that a refresh skips keeps its own time), instead of the repository's `refreshed_at`. Content hashes are used only by a refresh to settle racy basis entries.
- **P-3. A write's index step observes outside the lease and stores under it after a basis re-check**, like the whole refresh, instead of observing under the lease (R-3's "under the lease it holds for the write"). The write still holds its per-item row throughout, so no Manyhands writer of the same item can change the context; other items' writers are not held by the lease while it observes, as vh-bhepg's two-writer tests require (`tests/mutation_replay/reenter.rs` `writer_held_in_its_hand_off`), and no scan runs under the lease, as the promotion tests require (`spec4_assert_unleased_scans`, `src/repository/promotion/run_tests.rs` ~3371). A context whose basis moved between the observation and the store is left dirty, the row keeps owing its hand-off, and the answer is today's `RetryRequired`. The same applies to a sync's index step after its local update (Task 16), which the design put under the lease of the local-update step. The one read under the lease is re-hashing the few racy entries of the contexts being stored (a racy entry is a file modified within 2 s of the observation, typically the file the write just made); the RFC forbids full scans under the lease, not this bounded read.
- **P-4. A sync's index hand-off keeps the `refresh` action** and gets item scope from a per-item target, `synchronization-index-v1/<kind>/<item>/<oid>` (or `synchronization-index-v1/primary/<oid>`), instead of today's `''`. A new action would change the `OperationAction` contract enum (`src/repository/read/dto.rs` ~773); reusing the existing `synchronization-local-v1/` prefix (the sync's local-update rows, sync.rs ~7053) would make `operation list` report a crashed hand-off as `synchronize_*` instead of `refresh` (`read/status.rs` ~357-366). The new prefix is reported as `refresh`, as today.
- **P-5. Previews add no blockers.** Today a preview's discovery yields only the kind mismatch and the item's path (`src/repository/closure/observe.rs` ~239-242, ~300-322; `src/repository/promotion/observe.rs` ~59-62, ~215-240). `item_preview_facts` yields exactly those, from the item's own context and the primary's index rows after the primary's basis check (the primary is observed when `moved_contexts` reports it moved); duplicates in the item's own worktree are still found by its own context observation.

## Owner questions (answered with the plan's approval)

1. **Approve P-1–P-5** (recommended: yes; each keeps a property today's code has, and the design is updated to match).
2. **A primary sync's hand-off row is treated as the primary's own item in Table B** (recommended): a crashed primary sync whose push landed blocks only further primary syncs until resumed; other items' writes, refresh and poll proceed and index the primary. Close, promotion and folder create index under plain whole-repository refresh rows today (target `''`: promotion/run.rs ~1699-1716 under a fresh operation ID, folder.rs ~569-590 under the request's ID) and keep that repository scope: like `context sync-local` (decision 14), a crashed one still blocks refresh and poll until resumed. Close and promotion write the primary, so they also wait behind a crashed primary sync's hand-off row, and are refused while it is stale, until it is resumed. This is a second exception to decision 10, recorded for approval; narrowing those rows is left to a separate ticket.
3. **Measurement in scratch registries** (Task 1): the script runs `repo enable`, `repo identity-set`, `key generate` and the `--confirm` of close and promote only in its own scratch registry and replica; the owner adds the throwaway public key to localhost's `authorized_keys` (or names a key to import). Recommended: OK once, recorded in the ledger.
4. **Approve the named edits E-1–E-11**, and the inventory Task 3 Step 1 brings to the checkpoint.

## Global Constraints

- Run Rust only as `devenv shell -- cargo <cmd>` with `CARGO_TARGET_DIR=/home/vhodges/work/src/manyhands/target/f3wts`, so this branch's binaries never replace `target/debug/manyhands-cli` or `target/release/manyhands-cli` of the main checkout, and no `target/` is created inside the ticket worktree.
- The registry migration is one-way: never run a build from this branch against the owner's registry (`~/.local/share/manyhands`). Every run of this branch's CLI or library outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations (status saves, comments, syncs, close) run only as `devenv shell -- cargo run --locked --bin manyhands-cli -- …` from an up-to-date main checkout, never from a prebuilt binary path.
- Between steps run only the tests you wrote or changed, by exact name (`--lib <module>::<test> -- --exact`, or `--test <target> <name> -- --exact`; an SSH target takes the same filter). Never a whole SSH target between steps. If one test command takes over about 15 minutes, stop, narrow it, and note the target in the ledger.
- Static gates, the fast tier and the SSH tier run once, in the final task. This work touches `src/repository/transport/`, so the SSH tier is every SSH-harness target.
- Tests at the cheapest layer: library unit tests by default, table-driven; non-SSH integration targets for read and CLI contracts; exactly one new SSH case (Task 18), because connection reuse is transport behaviour. No real waits: inject the clock and use the existing hooks to park work.
- Named edits only (section "Named edits"). A test that breaks and is not named stops the work: record it in the ledger and ask the owner.
- No change to JSON shapes, codes, recovery actions or the `OperationAction` enum (design decision 8, P-4); every golden under `tests/fixtures/` stays byte-identical.
- The test hooks keep their contracts: `set_observation_hook_for_testing` fires after observing and before storing, now in the whole refresh between its two steps, in a write's index step, and in local sync's index; `set_refresh_persist_hook_for_testing` fires before a store. This keeps the tests listed under "Re-run, expected unchanged".
- Merge `origin/main` into the ticket worktree (never rebase) at the start of a task if main moved; publish the ticket branch after each task with `item sync` (or the task's `comment add`) from the main checkout.
- Desktop code is not expected to change. If a task touches `src/desktop/`, also run `check` and `clippy` with `--features desktop` on its own.
- Manyhands content (tickets, comments, documents) is one line per paragraph, no hard wrapping.
- Commit trailer: `Co-Authored-By:` the executing model, `<noreply@anthropic.com>`.

## Review Focus

- An item with a live worktree must be listed once, from the worktree, and not read as stale; released or retired, it must come back from main at the next read (Task 3 Table A rows, Tasks 6–7).
- A write must never answer `discovery: current` while one of its contexts is dirty; a crash after the commit and before the index step must leave the context dirty and the row resumable to the same commit (Task 10, `a_crash_after_the_commit_leaves_the_context_dirty_and_resume_completes_it`).
- An editor or an outside `git` command writing while a context is observed must not be stored as current (Task 5 `an_edit_during_observation_keeps_the_previous_rows_and_marks_the_context_dirty`, Task 12 `an_outside_change_between_the_refresh_steps_leaves_the_context_dirty_and_answers_busy`).
- A crashed save whose commit landed must not block refresh, poll or other items, and must close when resumed (Task 3 Table B rows, Tasks 10 and 14).
- A registry written by an older build must open, read as stale where it must, and be brought current by one refresh (Task 2 `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty`, Task 12 `the_first_refresh_after_migration_observes_every_context_once`).

## Named edits

For the owner's approval with this plan. Each changes only as stated.

- **E-1** `tests/discovery_rebuild.rs`: `refresh_scan_race_retains_previous_rows_then_converges_on_retry`, `no_mutation_scan_race_retry_preserves_state_after_external_change`, `refresh_does_not_persist_private_markdown_while_detecting_source_races`, `refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes`, `refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes` and the helper `assert_active_worktree_race_preserves_rows`: only if they fail with the hook between the refresh's two steps, adjust where the race is injected; the asserted outcomes (previous rows kept, `busy`, convergence on retry, no private Markdown persisted) stay. `discovery_public_types_hold_metadata_only`: add `DiscoveryContextKind::Invalid` to the expected public shape.
- **E-2** `tests/read_items.rs`: `refreshed_at_is_set_only_when_a_refresh_completes` (still true for whole refreshes; add that a per-item write does not set it); `a_folder_made_while_a_refresh_observes_makes_the_refresh_try_again` (only if it fails, as E-1).
- **E-3** `tests/read_status.rs`: `index_status_of_an_interrupted_refresh_is_stale_and_says_why` (stale now also from a dirty context; the reason text stays); `index_status_is_stale_when_the_index_holds_an_item_twice` (a primary copy plus its valid worktree copy is not stale; two copies Table A does not explain still are).
- **E-4** Duplicate-copy reads, replaced by Table A: `tests/read_comments.rs` `comments_of_an_item_the_index_holds_twice_come_from_the_chosen_copy_and_are_stale`; `tests/read_items.rs` `an_item_the_index_holds_twice_is_listed_once_and_read_from_the_copy_that_is_there` and `a_ticket_the_index_holds_twice_shows_only_its_effective_rows_relationships`, with their helpers `insert_item_worktree_row` and `insert_second_ticket_row`: "chosen copy" stays; "stale" is asserted only where Table A says.
- **E-5** `tests/mutation_replay/reenter.rs` `writers_on_two_tickets_both_complete_while_one_is_unfinished`: `index refresh` now succeeds and indexes the unfinished ticket instead of answering `recovery_required`; it asserts the unfinished row is still open afterwards; the retry still finishes with the same commit and no pending rows remain.
- **E-6** `src/repository/recovery_scope_tests.rs`: the rows for item rows against whole-repository calls, per Table B.
- **E-7** Checkpoints that mark the repository's `refresh_required`: in `tests/local_authoring.rs`, `document_exact_create_retry_checkpoints_once_then_is_a_no_op`, `document_completed_move_retry_retries_only_registry_invalidation`, `document_noop_registry_failure_returns_pending_then_invalidates_without_commit`, `recovery_ticket_registry_failure_returns_refresh_pending_then_invalidates_without_another_commit` and the helper `assert_registry_refresh_is_the_only_row_change`; in `src/repository/closure/checkpoint_tests.rs`, `hook_writes_closing_then_moves_the_context_and_marks_refresh` and `adopted_synchronization_is_integrated_and_the_checkpoint_kept`; in `src/repository/remote/sync_tests.rs`, `review_resolution_durably_requests_refresh` and `review_resolution_finalizer_and_refresh_sql_faults_retry_without_new_git_effects` (conflict resolve); in `src/repository/context_retirement/execution_tests.rs`, `retirement_absence_index_retry_does_not_reselect_or_touch_new_worktree`: assert the context's dirty mark where they assert `refresh_required`; their failure-injection outcomes stay, because the mark keeps today's place after the commit (Task 8). In `src/repository/read/items_tests.rs`, `the_index_state_is_never_refreshed_only_when_nothing_was_ever_stored` and the helper `index()` pass the new `any_dirty` argument (false).
- **E-8** Transport: `tests/ssh_transport/exact.rs` `tracking_boundaries` modes 0 and 1 change the server between two connections and assert the download used the second advertisement; with one connection per call (the test calls `fetch_exact` directly, exact.rs ~676-680) they assert instead that the download used the one advertisement and the tracking reference holds the advertised OID; the moved-remote case stays with `remote_change_during_fetch`. Modes 2–4, `reference_status_rejection` and `tests/ssh_transport/synchronization.rs` `cancel_before_transfer` move from `Checkpoint::Reconnected` to `Checkpoint::TransferStarted` and keep their assertions.
- **E-9** Per-item holds (`tests/mutation_replay/reenter.rs`): `a_second_writer_held_off_by_the_lease_waits_for_the_live_peer_and_names_it`, `a_second_writer_waits_for_a_live_peer_then_answers_busy_naming_it`, `a_busy_request_completes_when_retried_after_the_holder_finishes` and `a_peer_that_finishes_inside_the_wait_lets_the_second_writer_complete` park a peer of another item in its hand-off; the peer moves to the same item (or a repository-scoped row), so they keep testing wait-then-`busy` and the named blocker. `a_save_that_sets_relationships_answers_busy_at_once_behind_a_live_peer`: its peer is parked in its hand-off (`writer_held_in_its_hand_off`, reenter.rs ~1076-1110), which no longer holds other items; the peer is parked before its commit instead (`first_call_held_before_the_domain`, ~2271), so it is mid-write and the answer stays `busy`. `a_save_that_sets_relationships_is_refused_behind_another_item_s_unfinished_save`: behind a row that owes only its hand-off the save proceeds and is checked against Task 13's overlay (a cycle answers `relationship_cycle`); behind a mid-write row it is still refused.
- **E-10** A sync's hand-off row (P-4): `tests/remote_synchronization.rs` `index_pending` (its scan counts: a retry with an unchanged basis observes nothing), `tests/ssh_transport/synchronization.rs` `authoritative_replay_remote_removed` and `src/repository/comment_publication_tests.rs` `comment_publication_authoritative_child_allows_only_its_plain_refresh_and_reopen` (the row's target is the per-item `synchronization-index-v1/...` target instead of `''`), `src/repository/remote/sync_tests.rs` `local_binding_after_none_rejects_actual_other_target_service_authority` (its count of `synchronization-local-v1/%` rows is unchanged; it gains one `synchronization-index-v1/%` row per sync, push-only included). Task 16's inventory adds any test that pins the target `''` of a post-push hand-off row.
- **E-11** `src/repository/promotion/run_tests.rs` helper `spec4_assert_unleased_scans` counts `observe_context` scans as well as `observe_root` scans, so its `count > 0` assertions keep their meaning once previews stop calling `observe_root`, and its no-scan-under-the-lease check covers both; applied in Task 5, when `observe_context` appears; the six `spec4_*` tests keep their assertions.

**Re-run, expected unchanged** (each in the task that touches it; a failure is recorded and brought to the owner): every user of `set_observation_hook_for_testing` (`src/repository/comment_publication_tests.rs`, `src/repository/remote/sync_tests.rs`, `tests/discovery_rebuild.rs`, `tests/journal_rejection.rs`, `tests/mutation_bridges/folder.rs`, `tests/mutation_confirmation/abandon.rs`, `tests/mutation_replay/local_sync*.rs`, `tests/mutation_replay/reenter.rs`, `tests/read_items/local_sync.rs`, `tests/read_items.rs`, `tests/read_status.rs`, `tests/remote_synchronization.rs`, `tests/ssh_transport/synchronization.rs`); every user of `set_refresh_persist_hook_for_testing`; `tests/remote_synchronization.rs` `remote_change_during_fetch`; the `observe_root` shape tests in `src/repository/discovery.rs` (`observes_valid_document_and_ticket_active_contexts`, `invalid_configuration_suppresses_active_context_discovery`, `wrong_root_branch_remains_unverified_while_discovering_valid_active_contexts`, `reports_a_registered_deterministic_worktree_on_a_nonconforming_branch`, `rejects_an_independent_repository_at_a_deterministic_worktree_path`, `rejects_mismatched_and_malformed_active_context_candidates`) and `src/repository/remote/poll_tests.rs` (`poll_tree_conformity_matrix_rejects_without_creation_and_all_accepted_are_discoverable`, `poll_document_selection_matches_discovery_extensions_including_dotfiles`), because `observe_root` keeps its shape; `a_file_read_by_path_that_is_newer_than_the_refresh_is_stale_whatever_it_holds` and the comment stale cases in `tests/read_comments.rs` (P-2); `a_hand_off_no_operation_owes_is_offered_as_a_refresh` (`outcome_tests.rs` ~1396); the tests that insert `contexts` rows by SQL (they stay clean, because `dirty` defaults to 0, Task 2, and reads fall back to the repository's `refreshed_at` when a context has no `indexed_at`, Task 9); the other users of E-4's helpers (`tests/read_items.rs` ~2749, ~2797, ~2824); the `BeforeRegistryWrite` cases (`recovery_document_registry_failure_returns_refresh_pending_and_retry_does_not_commit_again`, `a_commit_the_domain_names_and_did_not_make_is_not_reported` in `tests/mutation_replay/execute.rs`, fault 3 of `comment_publication_binding_and_file_faults_reopen_without_duplicate_effects` and fault 5 in `tests/comment_publication.rs`), unchanged because the mark stays after the commit; `refresh_records_each_stable_context_persisted_before_completion` (`tests/discovery_rebuild.rs` ~1683-1719); the rebuild hook users (`tests/journal_rejection.rs` ~1019, ~1070; `tests/mutation_confirmation/abandon.rs` ~2311); the lifecycle tests that read `snapshot.refresh_required` (`src/repository/promotion/run_tests.rs` ~1296, ~2579, ~5159; `src/repository/closure/run_tests.rs` ~1422), because it keeps meaning the flag or any dirty mark; the no-new-socket checks in `tests/ticket_closure/journeys.rs` (~222, ~1372) and `tests/document_promotion/advanced.rs` (~247), which cover paths with no remote work; `context_rejects_a_dirty_configuration_path_without_creating_a_context` (`tests/local_authoring.rs`) and `publication_selection_blocks_selected_removal_and_dirty_configuration_path` (`tests/repository_enablement.rs`); `first_parent_history_respects_the_requested_inspection_bound`; every golden.

---

### Task 0: Preflight, ledger, start

**Files:**
- Create: `docs/plans/2026-11-19-fast-writes-execution.md` (execution ledger: per task the base SHA, tests run by name, results, deviations, measurements)

- [ ] **Step 1:** In the ticket worktree, `git fetch origin && git merge origin/main` (never rebase). Record the base SHA in the ledger.
- [ ] **Step 2:** Re-check every file, function and line this plan and the design cite (`grep -n`); record drift in the ledger and use the current names. List by name, in the ledger, the existing tests that pin the checks this plan keeps (push-absence before a merge, the head and status check before a local update and before a push, re-confirmation at `src/repository/promotion/integrate.rs` ~183, a resumed lifecycle's re-check); the tasks that touch those paths re-run them.
- [ ] **Step 3:** From the main checkout, set the ticket's status to `implementing`, then add the start comment: base SHA, the task list, and for each Acceptance item the test or measurement that proves it. Commit the ledger first and publish.

### Task 1: Measurement script and baseline

**Files:**
- Create: `scripts/measure-writes` (bash, executable)

**Interfaces:**
- Produces: `scripts/measure-writes --build-ref <git ref> [--local-only] [--runs N] --out <csv>`. It writes `command,run,seconds,build_ref` rows. Later tasks run it unchanged.

- [ ] **Step 1: Write the script.** It: exports `git archive <build-ref>` into a temporary directory and builds the release CLI there with its own `CARGO_TARGET_DIR` under the temporary root (no `git worktree add` in the owner's repository); creates `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under the temporary root and refuses to run if `MANYHANDS_DATA_DIR` is not under it; clones the main checkout with every `origin/manyhands/*` branch into a replica, and a bare copy as its remote at `ssh://$USER@127.0.0.1/<bare path>` (`--local-only`: an unreachable `ssh://git@127.0.0.1:1/...`); runs `repo enable`, `repo identity-set`, `key generate` for a throwaway key (or `key import` of one the owner names), `key select`, `host approve` for `127.0.0.1`, then `poll once` to materialize every open context (`--local-only`: `context prepare` for each open item instead); then times, N runs each (default 3), on fixed items: `ticket save`, `document save`, `ticket create`, `comment add` (its local-checkpoint time also read from the `progress:` lines on stderr), `item sync`, `document promote --prepare`/`--confirm` and `ticket close --prepare`/`--confirm` on items it creates, `index refresh`, `poll once`. `--local-only` skips the commands that need the remote.
- [ ] **Step 2: The owner's OK** (Owner question 3). The script runs `repo enable`, `repo identity-set`, `key generate` and the `--confirm` of close and promote, all middle-column verbs, but only in its scratch registry and replica. Record the owner's OK in the ledger; show the `127.0.0.1` fingerprint before the first `host approve`; ask the owner to add the throwaway public key to `~/.ssh/authorized_keys` for localhost (or name an existing key to import). Until then, run `--local-only`.
- [ ] **Step 3: Run the baseline** with `--build-ref <base SHA> --runs 3` (SSH mode once the owner's steps are done) and record the medians in the ledger and a ticket comment. Commit the script.

## Phase 1: the index

### Task 2: Registry migration, context keys and the invalid kind

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` ~1555, tables ~1599-1664), `src/repository/recovery.rs` (`migrate_operation_records` ~597-775), `src/repository.rs` (`DiscoveryContextKind`, `context_kind_name` ~6929, kind readers ~7519, ~7548), `src/repository/read/items.rs` (kind reader ~540)
- Test: `src/repository/discovery.rs` tests

**Interfaces:**
- Produces: `contexts` gains `basis TEXT` (JSON; NULL = unknown: the next whole refresh re-observes the context), `indexed_at INTEGER` (unix nanoseconds, `i64`), `dirty INTEGER NOT NULL DEFAULT 0 CHECK (dirty IN (0,1))` (the migration sets every existing row to 1; a row an older build writes later is clean, because an older build stores only after a full observation, and its NULL basis makes the next refresh re-observe it), `activity_base TEXT` (primary only); table `primary_activity(repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE, path TEXT NOT NULL, commit_oid TEXT NOT NULL, committed_at INTEGER NOT NULL, ordinal INTEGER NOT NULL, UNIQUE(repository_id, path))` (`ordinal`: the commit's position on the primary's first-parent chain, larger is newer); `operation_records.owed_contexts TEXT` (JSON array of encoded `ContextKey`s, written when the row is created, before any commit; NULL = unknown, which a resume treats as today's whole-repository refresh with repository scope; the builder may use the legacy `operation_record_contexts` table, recovery.rs ~618, instead if it fits). Marker `cycle_context_basis`, the ALTER ADD COLUMN pattern of `migrate_operation_attribution` (recovery.rs ~775), never through `migrate_item_read_schema`.
- Produces: `DiscoveryContextKind::Invalid` (a new public variant; `Unverified` is root-only, repository.rs ~7548), stored as `invalid` and accepted by every kind reader.
- Produces: `pub(super) enum ContextKey { Primary, Item(canonical::ItemId) }` with `fn encode(&self) -> String` (`"primary"` or the ULID) and `fn decode(&str) -> Option<Self>`, in `src/repository/discovery.rs`.

- [ ] **Step 1: Write failing tests** `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty` (create the tables without the new columns, insert a context, open with the new code: columns exist, the row has `dirty = 1` and NULL basis, the marker is set once, a second open changes nothing) and `the_invalid_context_kind_round_trips_through_every_kind_reader`.
- [ ] **Step 2: Run** both by exact name. Expected: FAIL.
- [ ] **Step 3: Implement.** No reader or writer uses the new columns yet.
- [ ] **Step 4: Run** both and E-1's `discovery_public_types_hold_metadata_only`. Expected: PASS. Commit.

### Task 3: Tables A and B, tests first — design checkpoint

**Files:**
- Test: Table A in `tests/read_items.rs`, `tests/read_comments.rs`, `tests/read_status.rs` (non-SSH targets with real registry fixtures: `enabled_repository`, `create_document_context`, `context_worktree`, and the SQL-row helpers the E-4 tests use), plus `src/repository/remote/state.rs` tests for the remote readers; Table B in `src/repository/recovery_scope_tests.rs` (extend `Seed`, which already has `item_id`, with `completed_step`; add `Call` variants for poll, another item's sync reservation, `context sync-local` admission and retirement), `src/repository/remote/reservation.rs` tests, `src/repository/context_retirement/` tests, `src/repository/local_sync/` tests

- [ ] **Step 1: Inventory** (no code): every existing test that asserts a refusal or a wait behind a pending local row or a remote `index_pending` row (grep tests for `require_no_pending_local`, `blocking_debt`, `LocalOperationPending`, `index_pending`, `insert_local`, `insert_remote_index_pending`, `writer_held_in_its_hand_off`, `first_writer_held_in_its_hand_off`, `LiveIndexOwner`). List each in the ledger with the Table B cell it falls under and whether that cell changes its answer; the ones E-9 already names are confirmed there. These go to the owner at Step 5 as named edits.
- [ ] **Step 2: Write the failing table-driven test** `presentation_across_contexts` in each read target, one row per Table A row (`an_item_only_on_the_primary_is_presented_from_it`, `a_primary_item_with_a_valid_worktree_is_presented_once_from_it_and_not_stale`, `a_released_or_removed_worktree_presents_the_primary_copy_again`, `an_invalid_worktree_presents_the_primary_copy_and_its_problems`, `an_item_only_in_a_worktree_is_presented_from_it`, `an_invalid_configuration_presents_no_item_contexts`, `relationships_and_short_codes_come_from_presented_copies_only`), each row checked over lists, `find`, `show`, the comment reads, graph reads, `index status`, `repo inspect` and the `list_repositories` problem count as the row says, with rows stored as the new design stores them (primary rows unhidden; an invalid context as its own `invalid` row); and `an_invalid_context_row_does_not_refuse_poll_remote_snapshot_or_history_reads` in `src/repository/remote/state.rs` tests.
- [ ] **Step 3: Write one failing test row per Table B cell** against the real checks: `in_the_way` and `is_in_the_way` (`src/repository/recovery.rs` ~2029-2136, including `CallReach::OtherItems`); `require_no_pending_local` (~1119) through its callers' entry points (`src/repository/remote/reservation.rs` ~317, ~1308, ~1557, ~1970, ~3153; `src/repository/remote/poll.rs` ~1674; `src/repository/context_retirement/execution.rs` ~330, `source.rs` ~116, `authority.rs` ~306/~401/~518); `local_sync/record.rs` `admit` (~1008); `context_retirement/pass.rs` `blocking_debt` (~382); `authority.rs` `local_owner` (~135); `reentry.rs` (~144); `record.rs` (~1734); the reservation's `busy` behind another remote row whose only debt is `index_pending` (`reservation.rs` ~1309-1318); a primary sync's hand-off row as the primary's own item (Owner question 2); close, promotion and folder-create rows keeping repository scope; and a comment's local row in `comment_registration_pending` (recovery.rs ~1124), whose local write and index step are done, treated as a row that owes only its hand-off: it holds its own item and nothing else.
- [ ] **Step 4: Run** them. Expected: FAIL where Tables A and B change today's answer. Commit them `#[ignore = "fast writes: design checkpoint"]`, so the tree stays green, and publish.
- [ ] **Step 5: Checkpoint.** Dispatch a fresh reviewer to check the tests against Tables A and B (one row per Table A row and Table B cell, each able to fail). Set the ticket's status to `waiting`; add a comment listing each test row and its Table A row or Table B cell, the reviewer's result, and the Step 1 inventory as named edits for approval; publish and stop. When the owner answers, set the status back to `implementing`, apply any change to the design first, and continue.

### Task 4: Activity without per-item history walks

**Files:**
- Create: `src/repository/activity.rs` (declared in `src/repository.rs`)
- Modify: `src/repository/discovery.rs` (`observe_activity` ~668)
- Test: `src/repository/activity.rs` tests (helpers copied from `discovery.rs` tests: `repository_on_main`, `checkpoint`, `checkpoint_reference`, `active_worktree_at`)

**Interfaces:**
- Produces: `pub(super) struct CommitActivity { pub oid: git2::Oid, pub at: OffsetDateTime, pub ordinal: i64 }` (ordinal: position on the walked first-parent chain, larger is newer).
- Produces: `pub(super) fn item_commit_activity(repository: &git2::Repository, head: git2::Oid, owned: &[PathBuf]) -> Result<ItemWalk, git2::Error>` with `pub(super) struct ItemWalk { pub found: Option<CommitActivity>, pub examined: usize }`: a first-parent walk from `head`, a diff against the first parent restricted to `owned` (`DiffOptions::pathspec` per path with `disable_pathspec_match(true)`), stopping at the first match, at most `MAX_ACTIVITY_HISTORY_COMMITS` (1024) commits; a root commit diffs against the empty tree. An item context stores its found commit in its basis, so an observation with the same head and owned paths reuses it (the design's (head, owned-path set) cache).
- Produces: `pub(super) struct PrimaryActivityMap { base: Option<git2::Oid>, newest: BTreeMap<PathBuf, CommitActivity> }` with `fn load(tx: &rusqlite::Transaction<'_>, repository_id: i64) -> Result<Self, RepositoryError>`, `fn store(&self, tx: &rusqlite::Transaction<'_>, repository_id: i64) -> Result<(), RepositoryError>`, `fn advance(&mut self, repository: &git2::Repository, head: git2::Oid) -> Result<Advance, git2::Error>` and `fn newest_over(&self, owned: &[PathBuf]) -> Option<CommitActivity>` (the entry with the largest ordinal, so it matches today's first-in-walk-order choice when timestamps are backdated); `pub(super) enum Advance { Unchanged, Incremental { diffed: usize }, Rebuilt { diffed: usize } }`. `advance` goes incremental only when `base` is an ancestor of `head` (`graph_descendant_of`) and found on `head`'s first-parent chain; otherwise one full first-parent walk, one unfiltered diff per commit. Only paths under `.manyhands/` and `docs/` are kept. No commit window (R-1).
- `latest_commit_touching`, `first_parent_commits` and `commit_touches_paths` stay in the non-test build until Task 5 switches the primary to the map; then they move under `#[cfg(test)]` as the oracle.

- [ ] **Step 1: Write the failing table test** `activity_matches_latest_commit_touching_for_every_history`: rows for direct commits, a backdated first-parent commit, a backdated commit across two owned paths, a side-branch-only change, a merge of primary into an item branch, a comment added and a comment invalid (parent missing), a mode-only change, a path with `*` and `?` in its name, a root commit, a merged item on main, and a rebuild from no map; each row asserts the activity time and `activity_source` equal the oracle's, for `item_commit_activity` and for the map, and `examined` is at most the oracle's walk.
- [ ] **Step 2: Write the failing tests** `the_primary_map_advances_incrementally_over_new_first_parent_commits` (diffed equals the new commits), `the_primary_map_rebuilds_after_a_reset_amend_or_second_parent_fast_forward` (three rows, `Rebuilt`, values equal the oracle at the new head), `the_primary_map_drops_a_path_that_leaves_the_owned_set` and `the_primary_map_keeps_an_item_older_than_the_window` (1030 commits; today's walk drops the item from the observation, the map keeps its time: R-1).
- [ ] **Step 3: Run** them by exact name. Expected: FAIL.
- [ ] **Step 4: Implement** `activity.rs`; `observe_activity` uses `item_commit_activity` for item contexts. The uncommitted part stays as `observe_activity` computes it.
- [ ] **Step 5: Run** the new tests and the existing activity tests (`observation_uses_a_newer_comment_commit_for_item_activity`, `observation_uses_the_newer_first_parent_commit_when_its_timestamp_is_backdated`, `observation_ignores_a_newer_side_branch_only_item_change`, `observation_uses_newer_uncommitted_item_or_comment_mtime`, `observation_excludes_invalid_comments_from_trees_and_activity`). Expected: PASS. Commit.

### Task 5: Per-context observation with a basis

**Files:**
- Create: `src/repository/basis.rs`
- Modify: `src/repository/discovery.rs` (`observe_root` ~318 and `observe_active_contexts` ~403 composed from per-context observations; `observe_root` keeps its shape, including `leave_active_items_to_their_contexts` ~776, for its current callers)
- Test: `src/repository/basis.rs` and `src/repository/discovery.rs` tests

**Interfaces:**
- Produces in `basis.rs`: `pub(super) struct Basis { head: Option<git2::Oid>, branch: Option<String>, worktree: PathBuf, config_blob: Option<git2::Oid>, item_commit: Option<CommitActivity>, files: Vec<BasisFile>, dirs: Vec<BasisDir> }`, `BasisFile { path, size, inode: Option<u64>, ctime_ns: Option<i64>, mtime_ns: i64, hash: Option<git2::Oid> }`, `BasisDir { path, inode: Option<u64>, ctime_ns: Option<i64>, mtime_ns: i64 }` (inode and ctime `None` on Windows; `hash` filled by an observation, `None` in a stat-only basis); `fn encode(&self) -> String`, `fn decode(&str) -> Option<Self>`; `pub(super) fn check(stored: &Basis, current: &Basis, indexed_at_ns: i64) -> BasisCheck` with `pub(super) enum BasisCheck { Same, Racy(Vec<PathBuf>), Moved }` (racy: an entry whose mtime is within 2 s of `indexed_at_ns`; the caller settles a racy file by hashing it against the stored hash, and the store records it clean by writing a basis whose `indexed_at` is past the window).
- Produces in `discovery.rs`: `pub(super) struct ListedContext { key: ContextKey, worktree: PathBuf }`; `pub(super) fn list_contexts(repository: &git2::Repository, root: &Path) -> Result<Vec<ListedContext>, RootObservationProblem>`; `pub(super) fn stat_basis(repository: &git2::Repository, root: &Path, listed: &ListedContext) -> Basis` (stat only, no file reads, re-lists only a directory whose stat changed); `pub(super) fn observe_context(repository: &git2::Repository, root: &Path, listed: &ListedContext, activity: &mut PrimaryActivityMap) -> ContextObservation`. `ContextObservation` holds today's per-context fields (context, head, sources, validation, items, folders for the primary, problems) plus `basis: Basis`, `started_at_ns: i64` (when this context's observation began; it becomes the context's `indexed_at`), `valid: bool`, `left_dirty: bool`. Stat, read and parse, stat again; on a difference observe once more; if still different, `left_dirty = true` and nothing is to be stored. An invalid item context yields its own observation (`valid = false`, its problems). The primary's items use `PrimaryActivityMap::advance` and `newest_over`, hidden items included; the map is loaded from the registry (read-only for callers that do not store, such as `observe_root` for previews until Task 21), and a fresh map, with its one full walk, only when none is stored. `observe_context` fires its own test counter (`set_context_scan_hook`), not `scan_checkpoint`; E-11 (applied in this task) makes the spec4 helper count both.

- [ ] **Step 1: Write the failing table test** `observation_and_basis_detect_every_kind_of_change`: rows for an editor write during the read (`an_edit_during_observation_keeps_the_previous_rows_and_marks_the_context_dirty`), a same-second same-size edit (racy, settled by hash), an mtime-preserving copy (`#[cfg(unix)]`), a branch switch at the same OID, a new comment file, a removed comment file, a folder made outside Manyhands (primary), a configuration edit, and a racy entry settled once and not hashed again.
- [ ] **Step 2: Write the failing tests** `an_invalid_worktree_is_observed_as_its_own_invalid_context` and `the_primary_observation_does_not_depend_on_active_contexts` (adding, invalidating and removing a worktree leaves the primary's `observe_context` equal).
- [ ] **Step 3: Run** them. Expected: FAIL.
- [ ] **Step 4: Implement.** Storage and reads are unchanged in this task.
- [ ] **Step 5: Run** the new tests, the `observe_root` shape tests listed under "Re-run, expected unchanged", and the six `spec4_*` tests with E-11. Expected: PASS. Commit.

### Task 6: Readers apply Table A

**Files:**
- Modify: `src/repository/read/items.rs` (`effective_rows` ~1567, `effective_copy` ~1616, `row_rank` ~1546, `stored_problems` ~804, `stored_index_state` ~572), `src/repository/read/status.rs` (`index_problems` ~871, context count ~929), `src/repository/read/admin.rs` (`list_repositories` ~216-254), `src/repository/read/graph.rs` (~884-891), `src/repository.rs` (`read_repository_snapshot` ~6983, the "not globally unique" failure ~7218-7225, branch parsing ~7200, ~7555), `src/repository/remote/state.rs` (~2766, ~2816, ~7040, `remote_snapshot` ~7194), `src/repository/local_sync/observe.rs` (~65)
- Test: Task 3's Table A rows

**Interfaces:**
- Produces: `effective_rows` and `effective_copy` are the one place a copy is chosen, and apply Table A to both the old stored shape (primary rows hidden) and the new one; the problem readers drop the primary's problems under a presented worktree's item directories; `read_repository_snapshot` chooses a copy for Table A's pair (a primary item and its valid worktree) instead of failing, and still fails for any other duplicate; every `kind='active'` reader excludes `invalid`.

- [ ] **Step 1:** Remove the `#[ignore]` from the Table A rows that need only the readers (rows over SQL fixtures); rows that need Task 7's storage stay ignored, and the ledger names them. Run them. Expected: FAIL.
- [ ] **Step 2: Implement.** Storage still writes the old shape, so today's tests keep passing.
- [ ] **Step 3: Run** those rows, E-3 and E-4. Expected: PASS. Commit.

### Task 7: Store each context from its own observation

**Files:**
- Modify: `src/repository.rs` (`persist_context_refresh` ~6413, `persist_context` ~6792, `persist_observation` ~6689, `persist_rebuild_observation` ~6202), `src/repository/discovery.rs` (the stored rows no longer apply `leave_active_items_to_their_contexts`; `latest_commit_touching` and its helpers move under `#[cfg(test)]`)
- Test: library tests next to `persist_context_refresh`; the remaining Table A rows

**Interfaces:**
- Consumes: Tasks 4–6.
- Produces: `pub(super) fn persist_context_index(tx: &rusqlite::Transaction<'_>, repository_id: i64, observation: &ContextObservation) -> Result<(), RepositoryError>`: one context's rows from its own observation, its basis, `indexed_at = observation.started_at_ns`, `dirty = 0`; an invalid context as an `invalid` row with its problems; for the primary also configuration observations, folders, `activity_base` and the activity map. Every store of index rows (refresh, rebuild, local sync) goes through it.

- [ ] **Step 1: Write failing tests** `a_store_writes_the_primary_unhidden_and_an_invalid_context_as_its_own_row` and `indexing_a_context_clears_its_dirty_mark_and_stores_its_basis`.
- [ ] **Step 2: Run.** Expected: FAIL. Remove the `#[ignore]` from the remaining Table A rows.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, every Table A row, E-1, E-3, E-4, and `refresh_keeps_primary_and_active_contexts_with_active_item_precedence` (`tests/discovery_rebuild.rs` ~736). Expected: PASS. Commit.

### Task 8: Dirty marks replace the repository flag for writes

**Files:**
- Modify: the writers of `refresh_required`: `src/repository.rs` (`mark_document_refresh_required` ~11284 and callers ~4008, `checkpoint_owned_paths` ~3859-3941, `persist_retry_problem` ~6514-6545), `src/repository/bridges/folder.rs` (~514), `src/repository/closure/checkpoint.rs` (~559), `src/repository/promotion/run.rs` (~1694-1699), `src/repository/promotion/lifecycle_record.rs` (~941), `src/repository/remote/state.rs` (~6530), `src/repository/context_retirement/` (`execution.rs` ~392, `preserve.rs` ~324, `reentry.rs` ~247, `run.rs` ~583, `authority.rs` ~421); its readers: `src/repository/read/mod.rs` (`index_state` ~92), `src/repository/read/items.rs` (~572), `src/repository/read/admin.rs` (~216-254), `src/repository.rs` (snapshot ~6991), `src/repository/mutation/bind/lifecycle.rs` (~1235, ~1386), `src/repository/promotion/run.rs` (~1694), `src/repository/remote/poll.rs` (~2500), `src/repository/context_retirement/handoff.rs` (~531), and the readers in `src/repository/discovery.rs` and `src/repository/recovery.rs`
- Test: library tests next to `mark_document_refresh_required`; E-7

**Interfaces:**
- Produces: `pub(super) fn mark_contexts_dirty(registry_path: &Path, root: &Path, contexts: &[ContextKey], operation: RepositoryOperation) -> Result<(), RepositoryError>`, used by every writer above, at the place each writes `refresh_required` today, so the failure contract stays (a checkpoint marks after `repository.commit`, repository.rs ~3923-3941; a failed mark answers `RefreshPending` with the commit, and a retry marks without committing again). A crash between the commit and the mark cannot leave stale rows reading current, because a context also counts as dirty while a pending row owes it (`owed_contexts`, written at row creation, Task 2); until Task 10 writes `owed_contexts`, that window is today's. `refresh_required` stays only for whole-repository facts (`reconcile_registration` ~11347-11376, the item-read migration ~1845, poll's marks ~2242/~2332, `persist_rebuild_retry` ~6250). `index_state(refresh_required, refreshed_at, has_contexts, any_dirty: bool)`, where `any_dirty` counts dirty marks and contexts owed by pending rows; `snapshot.refresh_required` keeps meaning the flag or any of those.
- Rule for each reader, recorded per reader in the ledger: a reader that decides whether one context is current reads that context's dirty mark; a reader that decides for the repository reads `refresh_required` or any dirty mark.

- [ ] **Step 1: Write failing tests** `a_checkpoint_marks_only_its_context_dirty`, `a_context_owed_by_a_pending_row_reads_dirty`, `index_status_is_stale_while_any_context_is_dirty`, `the_lifecycle_reads_the_dirty_marks_of_the_contexts_it_changed`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, E-7 and E-3. Expected: PASS. Commit.

### Task 9: Freshness on read against the context's time (P-2)

**Files:**
- Modify: `src/repository/read/items.rs` (`ItemFile::newer_than` ~1416, `ItemFile.modified` ~1311-1313, callers ~1792, ~1884), `src/repository/read/comments.rs` (~313, ~349-360)
- Test: library tests in `src/repository/read/`

**Interfaces:**
- Produces: `ItemFile::newer_than(indexed_at_ns: Option<i64>, refreshed_at: Option<i64>)` compares the file's modification second with the second of its context's `indexed_at`, or with the repository's `refreshed_at` when the context has none (rows from an older build, SQL fixtures); callers also report stale when that context is dirty.

- [ ] **Step 1: Write failing tests** `a_write_to_one_item_leaves_other_items_reads_fresh` and `an_outside_edit_after_indexing_reads_stale_from_its_context`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, `a_file_read_by_path_that_is_newer_than_the_refresh_is_stale_whatever_it_holds`, and the comment stale cases in `tests/read_comments.rs` (`a_changed_item_file_or_a_problem_file_that_is_no_longer_a_file_is_stale` and its neighbours). Expected: PASS. Commit.

### Task 10: A write's index step

**Files:**
- Modify: `src/repository.rs` (`handoff_post_authoritative` ~4012; `claim_lifecycle_indexing` ~4069), `src/repository/recovery.rs` (row creation; a new `FailurePoint::AfterCheckpointBeforeIndexStep`), `src/repository/mutation/bind/resume.rs` (`hand_off` ~1111-1132)
- Test: library tests next to `handoff_post_authoritative`

**Interfaces:**
- Consumes: Tasks 5, 7, 8.
- Produces: `fn record_owed_contexts(&self, root: &Path, record: RecoveryRecord, contexts: &[ContextKey]) -> Result<(), RepositoryError>` (written before the commit); `fn index_contexts(&self, root: &Path, record: RecoveryRecord, contexts: &[ContextKey]) -> Result<IndexOutcome, RepositoryError>` with `pub(super) enum IndexOutcome { Current, LeftDirty }`: with the row claimed for indexing and the lease released, observe each context once; fire `set_observation_hook_for_testing`; take the lease, `stat_basis` and `check` each context (hashing racy entries), store those that match with `persist_context_index`, delete the rows of removed contexts, leave moved ones dirty; release; on `Current` complete the row; on `LeftDirty` leave it owing its hand-off and answer today's `RetryRequired`. `fn complete_hand_off_unobserved(&self, root: &Path, record: RecoveryRecord, context: ContextKey) -> Result<IndexOutcome, RepositoryError>`: takes the lease, `stat_basis` and `check` (hashing racy entries); completes the row when the stored basis matches and the context is not dirty, else falls back to `index_contexts` (Task 16 uses it). `hand_off` in `resume.rs` calls `index_contexts` with the row's owed contexts, treating a context that no longer exists as indexed; a row with NULL `owed_contexts` (an older build's, an `insert_local` fixture's) runs today's whole-repository refresh with repository scope. A replay reads the dirty marks of the row's contexts before answering `discovery: current`.
- The indexing row keeps today's hold (in every call's way) until Task 14.

- [ ] **Step 1: Write failing tests** `a_crash_after_the_commit_leaves_the_context_dirty_and_resume_completes_it` (`AfterCheckpointBeforeIndexStep`; `operation resume` completes the row with the same commit; a replay answers `ok`; that a whole refresh proceeds and indexes it meanwhile is Task 14's), `a_crash_between_the_dirty_mark_and_the_commit_is_settled_by_the_next_refresh`, `a_crash_inside_the_index_transaction_stores_nothing` (`BeforeIndexTransactionCommit`), `a_resume_whose_context_was_retired_completes`, `an_abandon_leaves_the_dirty_marks`, `an_index_step_stores_only_after_a_basis_recheck_under_the_lease` (a hook moves the file between observe and store: `LeftDirty`, the row still owes its hand-off, a replay answers `discovery_pending`).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and the `reenter.rs` users of `writer_held_in_its_hand_off` (unchanged until Task 14). Expected: PASS. Commit.

### Task 11: Each command indexes its contexts

**Files:**
- Modify: the callers of `handoff_post_authoritative` (`src/repository.rs` ~1922 `prepare_context`, ~2509, ~2863, ~3376; `src/repository/bridges/repair.rs` ~594; `src/repository/context_release.rs` ~335) and the context-removal paths (release, retirement), `src/repository/local_sync/run.rs` (`index` ~886-907: `persist_context_index` inside its own `owner.transaction`, after the same observe-outside, re-check-under-lease order; it no longer sets the repository's `refresh_required = 0` or `refreshed_at`, ~906)
- Test: library test `each_command_indexes_only_its_contexts`

**Interfaces:**
- Produces: each command calls `record_owed_contexts` and `index_contexts` with the contexts of the design's command table; `remote add`, `remote select`, `remote remove`, `repo enable` and `repo identity-set` keep a whole-repository refresh (callers ~4391-5627), which still releases the lease before observing.

- [ ] **Step 1: Write the failing table test** `each_command_indexes_only_its_contexts` with one row per local command of the design's table (save, create, slug assign, comment checkpoint, `context prepare`, `context sync-local`, `context resolve-local`, `conflict resolve`, republish, repair, move, release, retire, abandon, folder create, the repository-level commands), asserting the contexts observed (the `set_context_scan_hook` counter) and that other contexts' rows and basis are unchanged. Tasks 12, 16 and 22 add the rows for refresh and poll, syncs, and close and promote.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the table test, `a_hand_off_no_operation_owes_is_offered_as_a_refresh`, and the `set_observation_hook_for_testing` users in `tests/journal_rejection.rs`, `tests/mutation_bridges/folder.rs`, `tests/mutation_confirmation/abandon.rs`, `tests/mutation_replay/local_sync*.rs`, `tests/read_items/local_sync.rs` and `src/repository/comment_publication_tests.rs`, by name. Expected: PASS. Commit.

### Task 12: The whole-repository refresh

**Files:**
- Modify: `src/repository.rs` (`refresh_repository_observed` ~1568, `refresh_repository_claimed` ~1512, `refresh_repository_target` ~1458, `reconcile_disappeared_contexts` ~6563, `rebuild_repository` ~1690-1747), `src/repository/mutation/bind/index.rs` (~138)
- Test: library tests next to `refresh_repository_observed`; E-1, E-2

**Interfaces:**
- Produces: the design's two steps, the repository-wide indexing row held from before step 1 to the end: step 1 outside the lease (`list_contexts`, `stat_basis`, `check`, `observe_context` for changed, dirty, new, NULL-basis or transiently-problematic contexts, the primary only when its basis moved); `set_observation_hook_for_testing`; step 2 under the lease, per context, in this order: if its stored row changed after step 1 and matches disk (another store landed), skip it without a mark; else if its disk still matches what step 1 observed, store it; else leave it dirty, with `persist_retry_problem` as today. A context that appeared after step 1 is not stored (no stub row, so Table A presents nothing from it) and makes the answer `RetryRequired`; disappeared ones are deleted. `persisted_context_count` counts the contexts the refresh found current, stored or skipped. `RetryRequired` if any is left dirty; `refreshed_at` = step-1 start; each stored context's `indexed_at` = its own observation start. `rebuild_repository` composes the same functions and keeps its two observations.

- [ ] **Step 1: Write failing tests** `a_refresh_observes_only_changed_contexts` (counted), `a_refresh_of_an_unchanged_repository_diffs_no_commits` and `a_refresh_after_k_new_primary_commits_diffs_k` (the ticket's Acceptance item on commits per refresh, counting `PrimaryActivityMap::advance` and `ItemWalk::examined`), `an_outside_change_between_the_refresh_steps_leaves_the_context_dirty_and_answers_busy`, `a_store_that_landed_after_step_one_is_not_marked_dirty`, `a_refresh_runs_no_observation_under_the_lease`, `a_refresh_reconciles_a_removed_worktree`, `the_first_refresh_after_migration_observes_every_context_once`; add the refresh and poll rows to `each_command_indexes_only_its_contexts`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, E-1, E-2, `refresh_records_each_stable_context_persisted_before_completion`, the rebuild hook users (`tests/journal_rejection.rs` ~1019, ~1070; `tests/mutation_confirmation/abandon.rs` ~2311), and the `set_observation_hook_for_testing` users in `tests/discovery_rebuild.rs`, `tests/read_items.rs` and `tests/read_status.rs`, by name. Expected: PASS. Commit.

### Task 13: The relationship check reads moved contexts from disk

**Files:**
- Modify: `src/repository/read/graph.rs` (`read_tickets_naming` and the comment targets it reads ~884-891), `src/repository/mutation/bind/ticket.rs` (~441)
- Test: library tests next to `read_tickets_naming`

**Interfaces:**
- Produces: `pub(super) fn moved_contexts(repository: &git2::Repository, root: &Path, stored: &[(ContextKey, Option<Basis>)]) -> Vec<ListedContext>` (a NULL basis counts as moved) (stat-only basis compare over every context, the primary included), shared with Task 21; `read_tickets_naming` overlays, for each moved context, its tickets and comment targets parsed from disk, without writing the index.

- [ ] **Step 1: Write the failing test** `a_relationship_check_reads_a_moved_context_from_disk` (an outside commit adds a dependency in B's worktree; a save of A that would close the cycle answers `relationship_cycle`).
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement. Step 4: Run** it and the relationship-check tests in `tests/mutation_relationships.rs` that the change reaches (by name, recorded in the ledger). Expected: PASS. Commit.

### Task 14: Per-item holds for local rows (Table B)

**Files:**
- Modify: the checks listed in Task 3 Step 3; `src/repository/recovery.rs` (`in_the_way` ~2071-2136 with `CallReach`, `Scope::of_row` ~2004, `INDEX_OWNER_STATES` ~2024, `owes_only_its_hand_off` ~2425, `require_no_pending_local` ~1119)
- Test: Table B rows for local rows; E-5, E-6, E-9; the Step 1 inventory of Task 3

**Interfaces:**
- Consumes: Tasks 10 and 12 (every store is basis-checked, so two index owners on different items cannot overwrite each other) and Task 13 (the overlay).
- Produces: one decision, `pub(super) fn holds(row: &PendingRow, call: &CallScope) -> bool`, whose truth table is Table B and nothing else: a row holds the call when the row is of the call's own item; or the row has repository scope; or the call has repository scope and the row does not owe only its hand-off; or the call is `CallReach::OtherItems` and the row is mid-write. `PendingRow` carries the row's state, `completed_step`, action, `item_id` and target (the query at recovery.rs ~2098 also reads `completed_step`); `CallScope` is the call's own item (or the primary), whether it reaches other items, and whether it is repository-wide. Every check of Task 3 Step 3 uses it, through an SQL rendering for the SQL checks; the table test `holds_agrees_with_table_b` enumerates state × step × row scope × call kind against Table B, and the SQL rendering against `holds`. `require_no_pending_local(connection, repository_id, call: &CallScope)` keeps refusing rows of the call's own item.

- [ ] **Step 1:** Remove the `#[ignore]` from the Table B rows for local rows (the primary sync's hand-off rows stay ignored until Task 16). Add `a_whole_refresh_proceeds_past_a_crashed_save_and_indexes_it` (Task 10's crash, then `index refresh`: it succeeds, the item is current, the row is still open). Run them. Expected: FAIL.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run** those rows, `holds_agrees_with_table_b`, the inventoried tests the owner approved as edits, E-5, E-6 and E-9. Expected: PASS. Commit.

### Task 15: Phase 1 close

**Files:**
- Modify: `src/repository.rs` (`first_parent_configuration_entries` ~11204 stops at the first match; the discarded `repository_snapshot` at the end of a hand-off is skipped), `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/RFC/desktop-information-architecture-and-editor.md`, `docs/RFC/cli-contract.md` (the `effects` definition's `discovery` on a write; the `index refresh` and `operation resume` rows), the design (P-1–P-5 and Owner question 2's answer applied)

- [ ] **Step 1:** Make the two code changes; run `context_rejects_a_dirty_configuration_path_without_creating_a_context` and `publication_selection_blocks_selected_removal_and_dirty_configuration_path` by name. Write the Phase 1 RFC amendments, dated, naming this ticket.
- [ ] **Step 2: Measure** with `scripts/measure-writes --build-ref HEAD --runs 3`; record the medians against the baseline and decision 1's targets, the time of one primary observation (R-6's 300 ms) and of the pre-commit safety checks (decision 11's 100 ms). Take sampled profiles of a save, a sync and a close (the ticket's method) and record that `latest_commit_touching` and a second observation are absent. If a threshold is exceeded, set the status to `waiting`, comment, and ask the owner before Phase 2.
- [ ] **Step 3:** A fresh reviewer reviews the Phase 1 diff against the design, P-1–P-5 and Tables A and B; fix what it finds and have the fixes re-reviewed; add a checkpoint comment with the numbers and the review result. Commit and publish.

## Phase 2: publishing

### Task 16: A sync indexes what it changed; no refresh after a push

**Files:**
- Modify: `src/repository/remote/sync.rs` (`synchronization_refresh` ~7081-7118 and its callers ~5916, ~5997, ~6051, ~6817, ~7867; the local-update lease block ~6576-6679; replay of a refresh row ~7069, ~7101-7105), `src/repository/remote/reservation.rs` (`validate_index_identity` ~218 and its calls ~1546, ~3129, ~3643; `finish_synchronization_index` ~3628; the `busy` behind an `index_pending`-only row ~1309-1318), `src/repository/recovery.rs` (`Scope::of_row`, `authored_ids` ~1099, the comment binding check ~180-191), `src/repository/read/status.rs` (`local_action` ~235-275), `src/repository/bridges/abandon.rs` (~267-270), `src/repository/comment_publication.rs` (~392-420)
- Test: `src/repository/remote/sync_tests.rs`, `src/repository/comment_publication_tests.rs`; Table B's sync rows; E-10

**Interfaces:**
- Consumes: Tasks 10 and 14.
- Produces (P-4): the sync's hand-off row, written when the hand-off starts (after the push is verified, or after the local update when nothing is pushed), is a `refresh` row with target `synchronization-index-v1/<kind>/<item>/<oid>` (or `synchronization-index-v1/primary/<oid>`) for every outcome, push-only included, with `item_id` set to the item's ULID (NULL for the primary) and `owed_contexts` set at creation. `Scope::of_row(action, item_id, target)` and `of_call`/`authored_ids` read the target, giving `Scope::Item("<ULID>")` or `Scope::Item("primary")` (a ULID never equals `primary`); the binding check, `validate_index_identity`, `local_action` (reported as `refresh`, as today), abandon and sync replay accept it, and still accept the whole-repository shape (`target = ''`) that older builds wrote. The row does not refuse its own sync's interrupted restart (`require_no_pending_local` at reservation.rs ~1308 receives the sync's own item). The sync's local update (a merge or fast-forward commit) writes its contexts into its local-update row's `owed_contexts` at creation and marks them dirty after the commit, as checkpoints do. Under a lifecycle parent (Task 22) a child sync writes no hand-off row: it records the contexts it changed in the parent's record, and the parent indexes them.
- Produces: `fn synchronization_index_hand_off(&self, request: &SynchronizeRemoteRequest, changed: &[ContextKey], outcome: SynchronizationOutcome) -> Result<SynchronizationResult, SynchronizationError>`: a merge or fast-forward indexes the contexts it changed with `index_contexts` after the local-update block (P-3); a push-only or already-current outcome calls `complete_hand_off_unobserved`. `publish_comment_with_config` skips `prove_comment_receipt` when the receipt is already recorded with the same commit.

- [ ] **Step 1:** Remove the `#[ignore]` from Table B's sync rows. Inventory the tests that pin a post-push hand-off row with target `''` and add them to E-10 in the ledger for the owner. **Write failing tests** `a_sync_that_merged_main_indexes_only_its_context`, `a_push_only_sync_records_its_hand_off_without_observing`, `a_push_only_sync_indexes_a_context_whose_basis_moved`, `a_crashed_sync_hand_off_blocks_no_other_item`, `a_hand_off_row_from_an_older_build_still_replays`, `a_comment_receipt_is_proved_once`; add the sync rows to `each_command_indexes_only_its_contexts`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, Table B's sync rows, E-10, the `set_observation_hook_for_testing` users in `src/repository/remote/sync_tests.rs`, and the push-absence and head-and-status tests listed in Task 0's ledger. Expected: PASS. Commit.

### Task 17: One SSH connection per remote call

**Files:**
- Modify: `src/repository/transport/remote.rs` (`transfer` ~347, `transfer_inner` ~356-450, `fresh_advertisement` ~120, `fetch_exact` ~133, `push_exact` ~201, `delete_exact` ~221-263, `download_push_target` ~275-309), `src/repository/transport/operation.rs` (`connect_ssh` ~289-371), `src/repository/transport/callbacks.rs` (`CallbackAttempt` ~12-73), `src/repository/transport/operation_tests.rs` (`Checkpoint` ~31), `src/repository/promotion/run.rs` (~150-170, the delete's caller)
- Test: `src/repository/transport/callbacks.rs` tests; E-8

**Interfaces:**
- Produces: a remote call lists and transfers on the connection `connect_ssh` opened (the closure already runs over `connection.remote()`, operation.rs ~360-371) and reuses its `CallbackAttempt`, so a connection submits the key once and a reconnect inside libgit2 fails closed. `recheck_ssh` and `recheck_trust` run before each transfer; a failed recheck refuses. `fetch_exact` lists and downloads on one connection. Because `git_remote_push` disconnects when it returns, the post-delete verification listing becomes its own remote call (the delete's caller, promotion/run.rs ~150-170, makes it), and `download_push_target` connects its own `remote_anonymous` explicitly with its own `CallbackAttempt`. No automatic reconnect on error. `Checkpoint::Reconnected` is replaced by `Checkpoint::TransferStarted`.

- [ ] **Step 1: Write the failing unit test** `a_second_key_submission_on_one_connection_fails_closed` on `CallbackAttempt` (no SSH).
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** it and E-8, each by exact name. Expected: PASS. Commit.

### Task 18: Fewer listings and inspections; the SSH connection count

**Files:**
- Modify: `src/repository/remote/sync.rs` (`fetch_and_observe` ~7925-7974: drop the "before" listing, keep "after"; the pre-push listing and the push run in one `with_synchronization_remote` closure, together with `prepare_synchronization_publication`, the `BeforePush` safe point and the inspection, ~6869-6873, ~6953-7007, ~8060-8088; `inspect_synchronization_local` call sites: merge ~6013 with ~6187, drop ~7934/~7944/~7954 inside the fetch window, keep one before each local update and before the push), `src/repository.rs` (`validate_context_tree` ~9741, cached by tree OID with kind, item, configuration blob and build)
- Test: `src/repository/remote/sync_tests.rs`; `tests/remote_synchronization.rs` (the one SSH case)

- [ ] **Step 1: Write failing tests**: `a_tree_validation_is_cached_by_tree_kind_item_configuration_and_build` (library); the SSH case `a_pushing_sync_uses_four_connections_and_a_merging_one_five` in `tests/remote_synchronization.rs`, modelled on `context_first_current_fast_forward_local_ahead` (~386) and `context_divergence(true)` (~706), counting `w.server.accepted_sockets()`. It needs SSH because connection reuse is transport behaviour.
- [ ] **Step 2: Run** them by exact name. Expected: FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, `remote_change_during_fetch`, the `set_observation_hook_for_testing` users in `tests/remote_synchronization.rs` and `tests/ssh_transport/synchronization.rs`, and the push-absence tests from the ledger, by name. Expected: PASS. Commit.

### Task 19: Phase 2 close

- [ ] **Step 1:** Write the `docs/RFC/git-workflow-and-conflict-recovery.md` amendments (the sync refresh steps; the upload-pack count).
- [ ] **Step 2: Measure** with `scripts/measure-writes` in SSH mode; record `item sync` and `comment add` against the 2–4 s target and the connection counts; profile a save, a sync and a close.
- [ ] **Step 3:** A fresh reviewer reviews the Phase 2 diff; fix and re-review; checkpoint comment. Commit and publish.

## Phase 3: promote and close

### Task 20: Registry migration once per process; audits by use

**Files:**
- Modify: `src/repository.rs` (registry open ~4251-4274; `RepositoryService` holds the memo), `src/repository/keys/registry.rs` (`list_shared_keys` ~329 and the key verbs ~143, ~413, ~525, ~594), the other `migrate_registry` callers (`src/repository/registration_removal.rs` ~332, `src/repository/bridges/abandon.rs` ~789/~981, `src/repository/mutation/records.rs` ~922, `src/repository/context_retirement/execution.rs` ~273, `src/repository.rs` ~3662, ~3688, ~3707, ~3783, ~3854, ~3971, ~4078, ~4098, ~4190, ~4211, ~11347), `src/repository/local_sync/record.rs` (`audit` ~469, `unfinished_context` ~1091, `require_no_pending` ~1104, `refresh_allowed` ~1116), `src/repository/local_sync/image_proof.rs` (`audit` ~237), `src/repository/closure/record.rs` (`save_record` ~552)
- Test: library tests in `src/repository/keys/` and `src/repository/local_sync/`

**Interfaces:**
- Produces: `fn open_migrated_registry(&self, operation: RepositoryOperation) -> Result<rusqlite::Connection, RepositoryError>` on `RepositoryService`: migrates only when the memo (the registry file's device and inode, and `PRAGMA schema_version`) differs from the file now; every caller above uses it. `list_shared_keys` takes a read guard. `local_sync::record::audit_schema(db)` (markers, schema SQL, foreign keys, integrity) runs once per memo; `local_sync::record::audit_unfinished(db)` audits every unfinished row (all items) and is what every guard calls; a completed record is audited when a read uses it (`src/repository/read/status.rs` ~302). Registry open audits the schema and the unfinished rows (decision 9).

- [ ] **Step 1: Write failing tests** `key_listing_after_open_runs_no_migration` (a migration counter; registry open's own migration is excluded), `a_replaced_registry_file_migrates_again`, `a_guard_refuses_a_malformed_unfinished_row_of_another_item`, `registry_open_audits_unfinished_rows_and_not_completed_ones`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and the local-sync audit tests in `src/repository/local_sync/record.rs` by name (listed in the ledger). Expected: PASS. Commit.

### Task 21: Two previews per attempt

**Files:**
- Modify: `src/repository/promotion/run.rs` (`promote_inner` ~427: the observation from ~492 also decides `cleanup_only` ~659; ~679 and ~710 read from the integration preview), `src/repository/promotion/integrate.rs` (`prepare_integration_for` ~110: no observation when `resume_written` wins), `src/repository/closure/observe.rs` (`preview` ~268, `prepare_observation_at` ~289-300), `src/repository/promotion/observe.rs` (`prepare_for` ~199-215), `src/repository/promotion/run_tests.rs` (E-11)
- Test: `src/repository/closure/run_tests.rs`, `src/repository/promotion/run_tests.rs` (`Fixture::remote()`)

**Interfaces:**
- Produces (P-5): `fn item_preview_facts(service: &RepositoryService, root: &Path, item: &ItemId) -> Result<PreviewFacts, RepositoryError>` with `pub(super) struct PreviewFacts { kind_mismatch: bool, item_path: Option<PathBuf> }` (closure `not_ticket` and `ticket_path`, promotion `not_document` and `document_path`), from the item's own context observation and the primary's index rows after `moved_contexts` (observing the primary when it moved), replacing `observe_root` in both previews; a counter hook `set_preview_facts_hook`.

- [ ] **Step 1: Write failing tests** `a_confirmed_close_takes_at_most_two_previews_and_no_whole_repository_observation` and the same for promotion (counting `set_preview_facts_hook` and `set_scan_hook` separately), and `a_resumed_attempt_rechecks_under_the_lease_without_a_consent_preview`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement**, and E-11.
- [ ] **Step 4: Run** the new tests, the six `spec4_*` tests, and the re-confirmation tests from the ledger. Expected: PASS. Commit.

### Task 22: One index update per exit; cleanup checks (P-1)

**Files:**
- Modify: `src/repository/promotion/run.rs` (`child` ~902-1025 records the child's index debt; every exit: `finish_integrated` ~1136-1160, abandon ~687/~718/~758, interrupted ~656/~675/~702, PrimaryAdvanced ~755, the reconciliation in `run_observed` ~348-408; cleanup checks ~1261, ~1340, ~1432-1458), `src/repository/cleanup.rs` (`remove_context_worktree` ~265-272)
- Test: `src/repository/closure/run_tests.rs`, `src/repository/promotion/run_tests.rs`

**Interfaces:**
- Produces: on every exit the parent calls `index_contexts` once with its own contexts and those its children recorded in its record (Task 16), under the same row it uses today (a whole-repository-scoped `refresh` row with target `''`, claimed as `refresh_repository_unauthored` claims it, promotion/run.rs ~1699-1716; Owner question 2), and runs no whole-repository refresh. Folder create does the same under its request's row (folder.rs ~569-590). P-1: the full `require_cleanup_clean` check runs under the lease inside `remove_context_worktree`; a head-and-status check runs before the remote delete (~1340); the checks at ~1261 and ~1432 go.

- [ ] **Step 1: Write failing tests** `every_lifecycle_exit_indexes_the_contexts_its_children_changed_once` (a table over finish, abandon, cancel, a stop at awaiting publication and its later finish, interrupted, primary advanced; each asserts exactly one index step and no whole-repository refresh), `a_dirty_worktree_stops_the_remote_delete`, `a_nested_repository_created_after_the_remote_delete_stops_the_removal_at_remote_deleted` (P-1's changed outcome: cleanup pending, the work merged, the retry finishes once it is gone), `a_refused_removal_leaves_the_close_at_worktree_removing_and_resumable`; add the close and promote rows to `each_command_indexes_only_its_contexts`.
- [ ] **Step 2: Run.** Expected: FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and the no-new-socket checks (`tests/ticket_closure/journeys.rs` ~222, ~1372; `tests/document_promotion/advanced.rs` ~247), by name. Expected: PASS. Commit.

### Task 23: Phase 3 close and the skills

**Files:**
- Modify: `docs/RFC/cli-contract.md` (the C5 known limits and follow-ups this change settles), `skills/manyhands-finding-work/SKILL.md` ("Is what I see current"), `skills/manyhands-polling-and-index/SKILL.md`, `skills/manyhands-recovery/SKILL.md` (a crashed write no longer blocks refresh and poll; `context sync-local`, close, promotion and folder create still do)
- Test: `tests/cli/skills.rs` drift tests

- [ ] **Step 1:** Write the amendments. Each skill change says which build has the new behaviour and keeps the old guidance for older builds (as vh-bhepg's skills did). Verify each changed claim with this branch's build in a scratch registry (isolated `HOME`, `MANYHANDS_DATA_DIR`, `GIT_CONFIG_GLOBAL`).
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::`. Expected: PASS.
- [ ] **Step 3: Measure** every command with `scripts/measure-writes`; record against every target; profile a save, a sync, a promote and a close and record that `image_proof::audit`, `latest_commit_touching` and repeated previews are absent.
- [ ] **Step 4:** A fresh reviewer reviews the Phase 3 diff; fix and re-review; checkpoint comment with the full before/after table. Commit and publish.

### Task 24: Review-ready

- [ ] **Step 1:** `git fetch origin && git merge origin/main` if main moved; `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] **Step 2: Static gates:** `devenv shell -- cargo check --all-features --locked`; `devenv shell -- cargo fmt --check`; `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. Expected: clean.
- [ ] **Step 3: Fast tier** exactly as AGENTS.md lists it (ending `-- --skip repository::local_sync::`), then `--lib repository::local_sync::`. Expected: PASS.
- [ ] **Step 4: SSH tier:** every SSH-harness target (`ssh_fixture`, `cli_remote`, `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `mutation_remote`, `comment_publication`, `remote_polling`, `document_promotion`, `ticket_closure`), each as `cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5:** A fresh whole-branch reviewer against the design (with P-1–P-5), Tables A and B and the ticket's Acceptance; fix and re-review the fixes.
- [ ] **Step 6:** Set the status to `review_ready`; add the review-ready comment (gates, tiers, measurements against targets, review result, and for the merge note: the registry migrates one-way; native CI should watch connection reuse on Windows). Ask the owner whether to open a PR.
- [ ] **Step 7:** After the owner's approval: ask which status the ticket ends with and save it; prepare `ticket close`, show the preview and wait; confirm only after the owner's OK, with the key loaded in ssh-agent (AGENTS.md). Native CI runs after merge on the owner's dispatch.

## Plan self-review

| Design item | Task |
| --- | --- |
| Decision 1 targets, Measurement | 1, 15, 19, 23 |
| Decisions 2–3, Goals 1–2, the command table | 5, 7, 10, 11, 12, 16, 22 |
| Decision 4, Goal 3, R-1, R-2 | 4, 5, 7 |
| Table A, R-4 | 2, 3, 6, 7 |
| Table B, R-3, decisions 10 and 14, Owner question 2 | 3, 14, 16 |
| Freshness on read, decision 8, P-2 | 8, 9 |
| Reads that decide on other items, decision 11 | 13, 15 (measured), 21 |
| Crash and retry, R-7, decision 12 | 10, 22 |
| Phase 2, Goal 4, P-4 | 16, 17, 18 |
| Phase 3, Goals 5–6, decision 9, R-5, R-6, P-1, P-5 | 20, 21, 22; R-6 measured in 15 |
| P-3 | 10, 12, 16 |
| Document amendments | 15, 19, 23 |
| Risks: one-way migration | Global Constraints, 1, 2, 23, 24 |
| Ticket Acceptance: measurements | 1, 15, 19, 23 |
| Ticket Acceptance: activity unchanged | 4 |
| Ticket Acceptance: commits or diffs per refresh | 12 |
| Ticket Acceptance: profiles | 15, 19, 23 |
| Ticket Acceptance: recovery and index results (amended, saved in `26e3a01c` and `f1c1c6f3`) | 3, 10, 14 |
