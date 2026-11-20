# Fast writes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each write cost about its own work: a write indexes only the contexts it changed, a whole refresh re-reads only what moved, a share uses 4–5 SSH connections, and a close or promotion previews twice and indexes once.

**Architecture:** Each context's index rows come from that context alone, with a stored basis (head, branch, file stats and hashes) and a dirty mark; which copy of an item is presented is decided at read. Writes index their contexts under the lease they already hold; a whole refresh scans outside the lease and stores by basis compare under it. Activity keeps today's values through short per-item walks and an incremental map for the primary. Phase 2 reuses one SSH connection per remote call and drops repeated listings, inspections and the post-push refresh; Phase 3 migrates the registry once per process, audits by use, and cuts a lifecycle's previews and refreshes.

**Tech Stack:** Rust, git2 0.20.4 (libgit2 with libssh2), rusqlite registry, serde_json, the SSH test harness in `tests/support/ssh_*`.

**Spec:** `docs/plans/2026-11-19-fast-writes-design.md` (approved 2026-11-19; decisions 1–14, rulings R-1–R-7, Tables A and B). Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts): its "How to measure" section and planning comments 1–3.

## Global Constraints

- Run Rust only as `devenv shell -- cargo <cmd>` with `CARGO_TARGET_DIR=/home/vhodges/work/src/manyhands/target`, so no `target/` is created inside the ticket worktree.
- Between steps run only the tests you wrote or changed, by exact name (`--lib <module>::<test> -- --exact`, or `--test <target> <name> -- --exact`; an SSH target takes the same filter). Never a whole SSH target between steps. If one test command takes over about 15 minutes, stop, narrow it, and note the target in the ledger.
- Static gates, the fast tier and the SSH tier run once, in Task 20. This work touches `src/repository/transport/`, so the SSH tier is every SSH-harness target.
- Tests at the cheapest layer: library unit tests by default, table-driven; non-SSH integration only for CLI or public-API contracts; exactly one new SSH case (Task 15). No real waits: inject the clock, and use the existing hooks to park work.
- Named edits only: an existing test changes only if this plan names it (section "Named edits") or a task's inventory step lists it in the ledger and it falls inside Table A or B. Any other test that breaks: stop and ask the owner.
- No change to JSON shapes, codes or recovery actions (design decision 8); every golden under `tests/fixtures/` stays byte-identical.
- The registry migration is one-way: never run a build from this branch against the owner's registry (`~/.local/share/manyhands`). Scratch registries only: isolated `HOME`, `MANYHANDS_DATA_DIR`, `GIT_CONFIG_GLOBAL`. Ticket operations (comments, saves, syncs) use the CLI built from main, from the main checkout, as AGENTS.md says.
- Merge `origin/main` into the ticket worktree (never rebase) at the start of a task if main moved; publish the ticket branch after each task with `manyhands-cli item sync` (or the task's `comment add`) from the main checkout.
- Desktop code is not expected to change (the desktop reads through the library). If a task touches `src/desktop/`, also run `check` and `clippy` with `--features desktop` on its own.
- Manyhands content (tickets, comments, documents) is one line per paragraph, no hard wrapping.
- Commit trailer: `Co-Authored-By:` the executing model, `<noreply@anthropic.com>`.

## Review Focus

- An item with a live worktree must be listed once, from the worktree, and not read as stale; released or retired, it must come back from main at the next read (Task 5 Table A rows, Task 6).
- A write must never answer `discovery: current` while one of its contexts is dirty; a crash between the commit and its index step must leave the context dirty and the row resumable to the same commit (Task 10, `a_crash_after_the_commit_leaves_the_context_dirty_and_resume_completes_it`).
- An editor or an outside `git` command writing while a refresh or a write observes must not be stored as current (Task 4 `an_edit_during_observation_keeps_the_previous_rows_and_marks_the_context_dirty`, Task 11 `an_outside_change_between_the_refresh_steps_leaves_the_context_dirty_and_answers_busy`).
- A crashed save whose commit landed must not block refresh, poll or other items, and must still close when resumed (Task 8 Table B rows, Task 9).
- A registry written by an older build must open, read as stale where it must, and be brought current by one refresh (Task 2 `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty`, Task 11 `the_first_refresh_after_migration_observes_every_context_once`).

## Named edits

Each changes only as stated; anything else in these tests stays.

- **E-1** `tests/discovery_rebuild.rs`: `refresh_scan_race_retains_previous_rows_then_converges_on_retry`, `no_mutation_scan_race_retry_preserves_state_after_external_change`, `refresh_does_not_persist_private_markdown_while_detecting_source_races`, `refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes`, `refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes`, and the helper `assert_active_worktree_race_preserves_rows`: the race is injected between the refresh's two steps (Task 11's hook) instead of between two observations; the asserted outcomes (previous rows kept, `busy`, convergence on retry, no private Markdown persisted) stay. `discovery_public_types_hold_metadata_only`: add the basis fields to the expected public shape only if a public type gains them.
- **E-2** `tests/read_items.rs`: `a_folder_made_while_a_refresh_observes_makes_the_refresh_try_again` (hook moves to between the steps; the outcome stays); `refreshed_at_is_set_only_when_a_refresh_completes` (still true for whole refreshes; add that a per-item write does not set it).
- **E-3** `tests/read_status.rs`: `index_status_of_an_interrupted_refresh_is_stale_and_says_why` (stale now also from a dirty context; the reason text stays); `index_status_is_stale_when_the_index_holds_an_item_twice` (a primary copy plus its valid worktree copy is not stale; two copies that Table A does not explain still are).
- **E-4** Duplicate-copy reads, replaced by Table A: `tests/read_comments.rs` `comments_of_an_item_the_index_holds_twice_come_from_the_chosen_copy_and_are_stale`; `tests/read_items.rs` `an_item_the_index_holds_twice_is_listed_once_and_read_from_the_copy_that_is_there` and `a_ticket_the_index_holds_twice_shows_only_its_effective_rows_relationships`, with their helpers `insert_item_worktree_row` and `insert_second_ticket_row`: "chosen copy" stays; "stale" is asserted only for the cases Table A says.
- **E-5** `tests/mutation_replay/reenter.rs` `writers_on_two_tickets_both_complete_while_one_is_unfinished`: `index refresh` now succeeds and indexes the unfinished ticket instead of answering `recovery_required`; the retry still finishes with the same commit and no pending rows remain.
- **E-6** `src/repository/recovery_scope_tests.rs`: the rows for item rows against whole-repository calls, per Table B.
- **E-7** Observation hooks that fire between a refresh's two observations: `src/repository/remote/sync_tests.rs` (~1295, ~1358, ~1400) and `src/repository/comment_publication_tests.rs` (~1612) move to the step hook of Task 11 or the per-context index step of Task 10; their assertions stay.
- **E-8** Transport reconnects: `tests/ssh_transport/exact.rs` `tracking_boundaries` and `reference_status_rejection`, `tests/ssh_transport/synchronization.rs` `cancel_before_transfer`, and `tests/remote_synchronization.rs` `remote_change_during_fetch` (~1029): they hook the per-transfer reconnect or the listing before the fetch; they move to the transfer checkpoint Task 14 defines, and keep their assertions (moved-remote detected, no leak, `Interrupted` with no new authentication).
- Expected unchanged, re-run in their task: `a_hand_off_no_operation_owes_is_offered_as_a_refresh` (`outcome_tests.rs:1396`); the no-new-socket checks in `tests/ticket_closure/journeys.rs` (~222, ~1372) and `tests/document_promotion/advanced.rs` (~247); `first_parent_history_respects_the_requested_inspection_bound` (`discovery.rs` ~2548); every golden.

---

### Task 0: Preflight, ledger, start

**Files:**
- Create: `docs/plans/2026-11-19-fast-writes-execution.md` (execution ledger: one row per task with base SHA, tests run, results, deviations)

- [ ] **Step 1:** In the ticket worktree, `git fetch origin && git merge origin/main` (never rebase). Record the base SHA in the ledger.
- [ ] **Step 2:** Re-check every file, function and line this plan and the design cite (`grep -n`); record drift in the ledger and use the current names.
- [ ] **Step 3:** From the main checkout, set the ticket's status to `implementing` (`manyhands-tickets`), then add the start comment (`manyhands-comments`): base SHA, the task list, and for each Acceptance item the test or measurement that proves it. Commit the ledger first.

### Task 1: Measurement script and baseline

**Files:**
- Create: `scripts/measure-writes` (bash, executable)

**Interfaces:**
- Produces: `scripts/measure-writes [--local-only] [--runs N] --out <csv>` prints and writes `command,run,seconds,build_head` rows. Later phase tasks run it unchanged.

- [ ] **Step 1: Write the script.** It builds the release CLI from the current tree into `CARGO_TARGET_DIR`; makes a scratch `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary directory; clones the main checkout with every `origin/manyhands/*` branch into a replica, and a bare copy as its remote at `ssh://$USER@127.0.0.1/<bare path>` (or an unreachable `ssh://git@127.0.0.1:1/...` with `--local-only`); runs `repo enable`, `poll once` (materializing every open context) and `index rebuild`; then times, N runs each (default 3), on fixed items: `ticket save`, `document save`, `ticket create`, `comment add`, `item sync`, `document promote --prepare`/`--confirm`, `ticket close --prepare`/`--confirm`, `index refresh`, `poll once`. Close and promote use fresh items it creates. `comment add` also records its local-checkpoint time from the CLI's `progress:` lines on stderr.
- [ ] **Step 2: Hand over the scratch trust steps.** Check `ssh -o BatchMode=yes 127.0.0.1 true`. The scratch registry needs a key selection (`key select`, the agent runs it) and a host approval for `127.0.0.1` (`host approve`: show the owner the fingerprint and wait). If localhost SSH with the owner's key does not work, ask the owner; until then run `--local-only`.
- [ ] **Step 3: Run the baseline** on this branch's base (before any code change), `--runs 3`, and record the medians in the ledger and in a ticket comment. Commit the script.

## Phase 1: the index

### Task 2: Registry migration for basis, dirty marks, owed contexts and the invalid kind

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` ~1555, table creation ~1599-1664), `src/repository/recovery.rs` (`migrate_operation_records` ~597-775), `src/repository.rs` (`context_kind_name` ~6929, stored kind readers ~7519), `src/repository/read/items.rs` (kind reader ~540)
- Test: `src/repository/discovery.rs` tests

**Interfaces:**
- Produces: `contexts` gains `basis TEXT` (JSON, NULL = unknown), `indexed_at INTEGER` (unix nanoseconds), `dirty INTEGER NOT NULL DEFAULT 1 CHECK (dirty IN (0,1))`; new table `primary_activity(repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE, path TEXT NOT NULL, commit_oid TEXT NOT NULL, committed_at INTEGER NOT NULL, UNIQUE(repository_id, path))` plus `contexts.activity_base TEXT` (the primary's map base OID); `operation_records.owed_contexts TEXT` (JSON array of context keys, NULL = unknown). Migration marker `cycle_context_basis`, ALTER ADD COLUMN pattern of `migrate_operation_attribution` (recovery.rs ~775), marker-based, never through `migrate_item_read_schema` (which would mark every registration stale).
- Produces: context kind `invalid` accepted by every kind reader. Step 1 decides whether the existing `unverified` kind already means this.
- Produces: `pub(super) enum ContextKey { Primary, Item(canonical::ItemId) }` with `fn encode(&self) -> String` (`"primary"` or the ULID) and `fn decode(&str) -> Option<Self>`, in `src/repository/discovery.rs`.

- [ ] **Step 1:** Read what writes and reads `unverified` and record in the ledger whether `invalid` is a new kind or a rename; use the answer below.
- [ ] **Step 2: Write failing tests** `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty` (create the tables without the new columns, insert a context, open with the new code: columns exist, the row has `dirty=1` and NULL basis, the marker is set once, a second open changes nothing) and `the_invalid_context_kind_round_trips_through_every_kind_reader`.
- [ ] **Step 3: Run** both by exact name. Expected: FAIL.
- [ ] **Step 4: Implement** the migration, `ContextKey`, and the kind.
- [ ] **Step 5: Run** both, and `first_parent_history_respects_the_requested_inspection_bound`. Expected: PASS. Commit.

### Task 3: Activity without per-item history walks

**Files:**
- Create: `src/repository/activity.rs` (declared in `src/repository.rs`)
- Modify: `src/repository/discovery.rs` (`observe_activity` ~668 calls the new functions; `latest_commit_touching`, `first_parent_commits`, `commit_touches_paths` move under `#[cfg(test)]` as the oracle)
- Test: `src/repository/activity.rs` tests (helpers copied from `discovery.rs` tests: `repository_on_main`, `checkpoint`, `checkpoint_reference`, `active_worktree_at`)

**Interfaces:**
- Produces: `pub(super) struct CommitActivity { pub oid: git2::Oid, pub at: OffsetDateTime }`.
- Produces: `pub(super) fn item_commit_activity(repository: &git2::Repository, head: git2::Oid, owned: &[PathBuf]) -> Result<Option<CommitActivity>, git2::Error>`: first-parent walk from `head`, a diff against the first parent restricted to `owned` (`DiffOptions::pathspec` per path with `disable_pathspec_match(true)`), stopping at the first match, at most `MAX_ACTIVITY_HISTORY_COMMITS` (1024) commits; a root commit diffs against the empty tree.
- Produces: `pub(super) struct PrimaryActivityMap { base: Option<git2::Oid>, newest: BTreeMap<PathBuf, CommitActivity> }` with `fn load(tx: &rusqlite::Transaction<'_>, repository_id: i64) -> Result<Self, RepositoryError>`, `fn store(&self, tx: &rusqlite::Transaction<'_>, repository_id: i64) -> Result<(), RepositoryError>`, `fn advance(&mut self, repository: &git2::Repository, head: git2::Oid) -> Result<Advance, git2::Error>` and `fn newest_over(&self, owned: &[PathBuf]) -> Option<CommitActivity>`; `pub(super) enum Advance { Unchanged, Incremental { diffed: usize }, Rebuilt { diffed: usize } }`. `advance` goes incremental only when `base` is an ancestor of `head` (`graph_descendant_of`) and found on `head`'s first-parent chain; otherwise one full first-parent walk, one unfiltered diff per commit, mapped to every touched path. No commit window (R-1).

- [ ] **Step 1: Write the failing table test** `activity_matches_latest_commit_touching_for_every_history`: rows for direct commits, a backdated first-parent commit, a side-branch-only change, a merge of primary into an item branch, comment added and comment invalid (parent missing), a mode-only change, a path with `*` and `?` in its name, a root commit, a merged item on main; each row asserts `item_commit_activity(head, owned)` equals the oracle and records the commits examined (at most the oracle's).
- [ ] **Step 2: Write the failing tests** `the_primary_map_advances_incrementally_over_new_first_parent_commits` (diffed equals the new commits), `the_primary_map_rebuilds_after_a_reset_amend_or_second_parent_fast_forward` (three rows, `Rebuilt`, values equal the oracle at the new head), `the_primary_map_drops_a_path_that_leaves_the_owned_set` and `the_primary_map_keeps_an_item_older_than_the_window` (1030 commits; today's walk drops the item, the map keeps its time: R-1).
- [ ] **Step 3: Run** them by exact name. Expected: FAIL.
- [ ] **Step 4: Implement** `activity.rs`, and switch `observe_activity` to `item_commit_activity` for item contexts (the primary's use comes in Task 4). The uncommitted part stays as `observe_activity` computes it today.
- [ ] **Step 5: Run** the new tests and the existing activity tests in `discovery.rs` (`observation_uses_a_newer_comment_commit_for_item_activity`, `observation_uses_the_newer_first_parent_commit_when_its_timestamp_is_backdated`, `observation_ignores_a_newer_side_branch_only_item_change`, `observation_uses_newer_uncommitted_item_or_comment_mtime`, `observation_excludes_invalid_comments_from_trees_and_activity`). Expected: PASS. Commit.

### Task 4: Per-context observation with a basis

**Files:**
- Modify: `src/repository/discovery.rs` (`observe_root` ~318 and `observe_active_contexts` ~403 split; `leave_active_items_to_their_contexts` ~776 no longer applied to stored rows; `collect_authoring_item_sources` ~952 unchanged)
- Create: `src/repository/basis.rs`
- Test: `src/repository/discovery.rs` tests, `src/repository/basis.rs` tests

**Interfaces:**
- Produces in `basis.rs`: `pub(super) struct Basis { head: Option<git2::Oid>, branch: Option<String>, worktree: PathBuf, config_blob: Option<git2::Oid>, files: Vec<BasisFile>, dirs: Vec<BasisDir> }`, `BasisFile { path, size, inode, ctime_ns, mtime_ns, hash: git2::Oid }`, `BasisDir { path, inode, ctime_ns, mtime_ns }` (inode and ctime absent on Windows), with `fn encode(&self) -> String`, `fn decode(&str) -> Option<Self>`, and `pub(super) fn check(stored: &Basis, current_stat: &Basis, indexed_at_ns: i128) -> BasisCheck` where `pub(super) enum BasisCheck { Same, Racy(Vec<PathBuf>), Moved }` (racy: an entry whose mtime is within 2 s of `indexed_at_ns`; a racy file is settled by comparing its content hash, and a confirmed one is recorded clean).
- Produces in `discovery.rs`: `pub(super) struct ListedContext { key: ContextKey, worktree: PathBuf }`, `pub(super) fn list_contexts(repository: &git2::Repository, root: &Path) -> Result<Vec<ListedContext>, RootObservationProblem>`, `pub(super) fn stat_basis(repository: &git2::Repository, root: &Path, listed: &ListedContext) -> Basis` (stat only, no reads), and `pub(super) fn observe_context(repository: &git2::Repository, root: &Path, listed: &ListedContext, activity: &mut PrimaryActivityMap) -> ContextObservation`. `ContextObservation` holds today's per-context fields (context, head, sources, validation, items, folders for the primary, problems) plus `basis: Basis`, `valid: bool` and `left_dirty: bool`. Stat, read and parse, stat again; on a difference observe once more; if still different keep `left_dirty = true` and no rows are to be stored. An invalid item context yields its own observation with `valid = false` and its problems (today they go to the primary). The primary's observation uses `PrimaryActivityMap::advance` and `newest_over` for every root item, hidden or not.
- `observe_root` remains for `index rebuild` and local sync until Tasks 10–11, composed from `list_contexts` and `observe_context`.

- [ ] **Step 1: Write the failing table test** `observation_and_basis_detect_every_kind_of_change`: rows for an editor write during the read (`an_edit_during_observation_keeps_the_previous_rows_and_marks_the_context_dirty`), a same-second same-size edit (racy, settled by hash), an mtime-preserving copy (`#[cfg(unix)]`), a branch switch at the same OID, a new comment file, a removed comment file, a folder made outside Manyhands (primary), a configuration edit, a racy entry confirmed clean once and not hashed again.
- [ ] **Step 2: Write the failing tests** `an_invalid_worktree_is_observed_as_its_own_invalid_context` and `the_primary_observation_does_not_depend_on_active_contexts` (adding, invalidating and removing a worktree leaves the primary's observation equal).
- [ ] **Step 3: Run** them. Expected: FAIL.
- [ ] **Step 4: Implement** `basis.rs` and the split.
- [ ] **Step 5: Run** the new tests and `refresh_keeps_primary_and_active_contexts_with_active_item_precedence` (`tests/discovery_rebuild.rs` ~736). Expected: the new ones PASS; record the existing one's state in the ledger (it is expected to fail until Task 6). Commit.

### Task 5: Table A tests first — design checkpoint

**Files:**
- Test: `src/repository/read/items_tests.rs` (extend; a tempdir root for `context_exists`), `src/repository/read/comments_tests.rs`, `src/repository/read/status_tests.rs`, `src/repository/read/graph_tests.rs`

- [ ] **Step 1: Write one failing test per row of the design's Table A**, named for the row (`an_item_only_on_the_primary_is_presented_from_it`, `a_primary_item_with_a_valid_worktree_is_presented_once_from_it_and_not_stale`, `a_released_or_removed_worktree_presents_the_primary_copy_again`, `an_invalid_worktree_presents_the_primary_copy_and_its_problems`, `an_item_only_in_a_worktree_is_presented_from_it`, `an_invalid_configuration_presents_no_item_contexts`, `relationships_and_short_codes_come_from_presented_copies_only`), each over lists, `show`, comments, graph reads and `index status` as the row says, with stored rows built through the existing `context()`/`stored()` helpers.
- [ ] **Step 2: Run** them. Expected: FAIL. Commit them as failing tests in their own commit, marked `#[ignore = "Table A: design checkpoint"]` so the tree stays green.
- [ ] **Step 3: Checkpoint.** Set the ticket's status to `waiting`, add a comment listing each test and the row it pins, publish, and stop until the owner answers. A row the owner changes changes the design first.

### Task 6: Read-time presentation

**Files:**
- Modify: `src/repository/read/items.rs` (`effective_rows` ~1567, `effective_copy` ~1616, `row_rank` ~1546, `stored_problems` ~804, `stored_index_state` ~572), `src/repository/read/status.rs` (`index_problems` ~871, context count ~929), `src/repository/read/admin.rs` (`list_repositories` ~224), `src/repository/read/graph.rs` (~884-891), `src/repository.rs` (`read_repository_snapshot` ~6983, the "not globally unique" failure ~7220, branch parsing ~7200, ~7555), `src/repository/remote/state.rs` (~2766, ~2816, ~7040, `remote_snapshot` ~7194), `src/repository/local_sync/observe.rs` (~65)
- Test: Task 5's tests; E-3, E-4

**Interfaces:**
- Consumes: Task 2's `invalid` kind, Task 4's per-context rows.
- Produces: `effective_rows` and `effective_copy` are the one place a copy is chosen, and apply Table A; the problem readers drop the primary's problems under a presented worktree's item directories; `read_repository_snapshot` chooses copies instead of failing; every `kind='active'` reader excludes `invalid`.

- [ ] **Step 1:** Remove the `#[ignore]` from Task 5's tests. Run them. Expected: FAIL.
- [ ] **Step 2: Implement** Table A in the readers listed.
- [ ] **Step 3: Run** Task 5's tests, E-3, E-4 and `refresh_keeps_primary_and_active_contexts_with_active_item_precedence`. Expected: PASS. Commit.

### Task 7: Per-context store and freshness

**Files:**
- Modify: `src/repository.rs` (`persist_context_refresh` ~6413, `persist_context` ~6792, `persist_retry_problem` ~6514, `mark_document_refresh_required` ~11284 and its callers ~4008, `checkpoint_owned_paths` ~3907/~3941), `src/repository/bridges/folder.rs` (~514), `src/repository/closure/checkpoint.rs` (~559), `src/repository/promotion/run.rs` (~1694-1699), `src/repository/promotion/lifecycle_record.rs` (~941), `src/repository/remote/state.rs` (~6530), `src/repository/context_retirement/` (`execution.rs` ~392, `preserve.rs` ~324, `reentry.rs` ~247, `run.rs` ~583, `authority.rs` ~421), `src/repository/mutation/bind/lifecycle.rs` (~1235, ~1386), `src/repository/read/mod.rs` (`index_state` ~92), `src/repository/read/items.rs` (`newer_than` callers ~1792, ~1884), `src/repository/read/comments.rs` (~313, ~349)
- Test: library tests next to `persist_context_refresh` and in `src/repository/read/`

**Interfaces:**
- Produces: `pub(super) fn persist_context_index(tx: &rusqlite::Transaction<'_>, repository_id: i64, observation: &ContextObservation, indexed_at_ns: i128) -> Result<(), RepositoryError>` (one context's rows, basis, `indexed_at`, `dirty = 0`; for the primary also configuration observations, folders and the activity map).
- Produces: `pub(super) fn mark_contexts_dirty(registry_path: &Path, root: &Path, contexts: &[ContextKey], operation: RepositoryOperation) -> Result<(), RepositoryError>`, used by every former `refresh_required` writer except whole-repository facts (`reconcile_registration`, the item-read migration, poll's mark, `persist_rebuild_retry`).
- Produces: `index_state(refresh_required, refreshed_at, has_contexts, any_dirty: bool)`; reads compare the content hash of the file they read with the stored basis hash, and report stale when the context is dirty; `ItemFile::newer_than` is removed.

- [ ] **Step 1: Write failing tests** `a_checkpoint_marks_only_its_context_dirty`, `indexing_a_context_clears_its_dirty_mark_and_stores_its_basis`, `reads_report_stale_from_a_dirty_context_or_a_changed_file_hash_only` (rows for `show` by ID and path, the comment reads, lists' `data.index`, `index status`, and a row from an older build), `the_lifecycle_reads_the_dirty_marks_of_the_contexts_it_changed`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and E-2's `refreshed_at_is_set_only_when_a_refresh_completes`. Expected: PASS. Commit.

### Task 8: Table B tests first — design checkpoint

**Files:**
- Test: `src/repository/recovery_scope_tests.rs` (extend `Seed` with `completed_step` and `item_id` so a row can owe only its hand-off; add `Call` variants for poll, other items' sync reservation, `context sync-local` admission and retirement), `src/repository/remote/reservation.rs` tests, `src/repository/context_retirement/` tests, `src/repository/local_sync/` tests

- [ ] **Step 1: Inventory** (no code): every existing test that asserts a refusal behind a pending local row or a remote `index_pending` row (grep tests for `require_no_pending_local`, `blocking_debt`, `LocalOperationPending`, `index_pending`, `insert_local`, `insert_remote_index_pending`). List them in the ledger with the Table B cell each falls under. Edits are allowed only where the cell changes.
- [ ] **Step 2: Write one failing test row per cell of the design's Table B** against the real checks: `is_in_the_way` (`recovery.rs` ~2029), `require_no_pending_local` (~1119) through each of its callers' entry points (`reservation.rs` ~317, ~1308, ~1557, ~1970, ~3153; `poll.rs` ~1674; `context_retirement/execution.rs` ~330, `source.rs` ~116, `authority.rs` ~306/~401/~518), `local_sync/record.rs` `admit` ~1017, `context_retirement/pass.rs` `blocking_debt` ~382, `authority.rs` `local_owner` ~135, `reentry.rs` ~144, `record.rs` ~1734, and the reservation's `busy` behind another remote row whose only debt is `index_pending` (`reservation.rs` ~1309-1318).
- [ ] **Step 3: Run.** Expected: FAIL where Table B changes today's answer. Commit them `#[ignore = "Table B: design checkpoint"]`.
- [ ] **Step 4: Checkpoint**, as in Task 5.

### Task 9: Per-item holds

**Files:**
- Modify: the checks listed in Task 8; `src/repository/recovery.rs` (`ITEM_ACTIONS` ~1085, `INDEX_OWNER_STATES` ~2024, `owes_only_its_hand_off` ~2425, the comment binding check ~182-190); `src/repository/remote/reservation.rs` (`validate_index_identity` ~3643)
- Test: Task 8's tests; E-5, E-6

**Interfaces:**
- Produces: `pub(super) const LOCAL_ROW_HOLDS_OTHERS: &str` (an SQL predicate: a row that is pending and does not owe only its hand-off) used by every SQL check in Task 8; `is_in_the_way` applies `INDEX_OWNER_STATES` to repository-scoped rows only.
- Produces: action `index_context`, added to `ITEM_ACTIONS`, with target `index-context-v1/<kind>/<item>` (or `index-context-v1/primary`): the per-item index hand-off row Task 13's syncs create. The binding check and `validate_index_identity` accept it.

- [ ] **Step 1:** Remove the `#[ignore]` from Task 8's tests. Run them. Expected: FAIL.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run** Task 8's tests, the inventoried tests, E-5 and E-6. Expected: PASS. Commit.

### Task 10: Writes index their own contexts

**Files:**
- Modify: `src/repository.rs` (`handoff_post_authoritative` ~4012 and its callers ~1922, ~2509, ~2863, ~3376, ~4391, ~4451, ~4539, ~4592, ~4727, ~4842, ~5418, ~5454, ~5627; `src/repository/bridges/repair.rs` ~594; `src/repository/context_release.rs` ~335), `src/repository/recovery.rs` (row creation), `src/repository/mutation/bind/resume.rs` (`hand_off` ~1111-1132), `src/repository/local_sync/run.rs` (`index` ~886-907)
- Test: library tests next to `handoff_post_authoritative`; `tests/mutation_replay/reenter.rs`

**Interfaces:**
- Consumes: Tasks 4, 7, 9.
- Produces: `fn record_owed_contexts(&self, root: &Path, record: RecoveryRecord, contexts: &[ContextKey]) -> Result<(), RepositoryError>`, written before the commit; `fn index_contexts(&self, root: &Path, record: RecoveryRecord, lease: &RepositoryLease, contexts: &[ContextKey]) -> Result<IndexOutcome, RepositoryError>` with `pub(super) enum IndexOutcome { Current, LeftDirty }` (observes each context once, stores it, deletes rows of removed contexts, completes the row; `LeftDirty` maps to today's `RetryRequired` answer). `handoff_post_authoritative` keeps the lease and calls `index_contexts` with the design's command-table contexts; repository-level commands (`remote add/select/remove`, `repo enable`) keep a whole-repository refresh. `hand_off` in `resume.rs` indexes the row's owed contexts, treating a context that no longer exists as indexed. `local_sync::run::index` indexes its context with `index_contexts`.

- [ ] **Step 1: Write the failing table test** `each_command_indexes_only_its_contexts`: one row per row of the design's command table, asserting the contexts observed (a per-context counter hook) and that other contexts' rows and basis are unchanged.
- [ ] **Step 2: Write failing tests** `a_crash_after_the_commit_leaves_the_context_dirty_and_resume_completes_it` (`FailurePoint::BeforeIndexTransactionCommit`; refresh and poll then proceed and index it; `operation resume` completes the row with the same commit; replay answers `ok`), `a_resume_whose_context_was_retired_completes`, `an_abandon_leaves_the_dirty_marks`, and `a_save_holds_the_lease_through_its_index_step` (a second process's lease attempt during the step is refused within the 250 ms budget, using `set_lifecycle_lease_hook_for_testing`).
- [ ] **Step 3: Run.** Expected: FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the new tests, E-5, E-7's comment case and `a_hand_off_no_operation_owes_is_offered_as_a_refresh`. Expected: PASS. Commit.

### Task 11: The whole-repository refresh and reads that decide on other items

**Files:**
- Modify: `src/repository.rs` (`refresh_repository_observed` ~1568, `refresh_repository_claimed` ~1512, `reconcile_disappeared_contexts` ~6563, `rebuild_repository` ~1690-1747, the relationship check reached at ~695-708), `src/repository/read/items.rs` (`check_ticket_relationships` ~2033)
- Test: library tests next to `refresh_repository_observed`; E-1, E-2, E-7

**Interfaces:**
- Consumes: Tasks 4, 7.
- Produces: the two-step refresh of the design (step 1 outside the lease: `list_contexts`, `stat_basis`, `check`, `observe_context` for changed, dirty, new or transiently-problematic contexts; step 2 under the lease: re-list, re-stat, store matches with `persist_context_index`, leave moved ones dirty, delete disappeared; `RetryRequired` if any left dirty; `refreshed_at` = step-1 start). The repository-wide indexing row is held from before step 1 to the end. A test hook `set_refresh_step_hook_for_testing(hook: impl FnOnce() + Send + 'static)` fires between the steps (replacing `set_observation_hook_for_testing`'s role).
- Produces: `fn fresh_relationships(&self, root: &Path) -> Result<Vec<StoredItem>, RepositoryError>` for the relationship check: the same stored ticket rows `check_ticket_relationships` reads today, with the rows of any context whose basis moved replaced by its tickets read from disk (not written to the index).

- [ ] **Step 1: Write failing tests** `a_refresh_observes_only_changed_contexts` (counted), `an_outside_change_between_the_refresh_steps_leaves_the_context_dirty_and_answers_busy`, `a_refresh_runs_no_observation_under_the_lease`, `a_refresh_reconciles_a_removed_worktree`, `the_first_refresh_after_migration_observes_every_context_once`, `a_relationship_check_reads_a_moved_context_from_disk`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**; `rebuild_repository` composes the new functions and keeps its two observations.
- [ ] **Step 4: Run** the new tests, E-1, E-2, E-7. Expected: PASS. Commit.

### Task 12: Phase 1 close

**Files:**
- Modify: `src/repository.rs` (`first_parent_configuration_entries` ~11204 stops at the first match; the discarded `repository_snapshot` in the hand-off is skipped), `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/RFC/desktop-information-architecture-and-editor.md`, `docs/RFC/cli-contract.md` (the `effects` definition's `discovery` on a write; the `index refresh` and `operation resume` rows)

- [ ] **Step 1:** Make the two code changes; run `ensure_configuration_path_clean`'s existing tests by name. Write the RFC amendments listed in the design (Phase 1 parts), dated, each naming this ticket.
- [ ] **Step 2: Measure:** `scripts/measure-writes --runs 3`; record the medians against the baseline and decision 1's targets in the ledger; take one sampled profile of a save (the ticket's method) and record that `latest_commit_touching` and a second observation are absent. If R-6's primary observation exceeds about 300 ms, record it and ask the owner before Phase 2.
- [ ] **Step 3:** A fresh reviewer reviews the Phase 1 diff against the design and Tables A and B; fix what it finds; add a checkpoint comment with the numbers and the review result. Commit and publish.

## Phase 2: publishing

### Task 13: A sync indexes what it changed, and no refresh after a push

**Files:**
- Modify: `src/repository/remote/sync.rs` (`synchronization_refresh` ~7081 and its callers ~5916, ~5997, ~6051, ~6817, ~7867; the local-update lease block ~6576-6679), `src/repository/remote/reservation.rs` (`finish_synchronization_index` ~3628), `src/repository/comment_publication.rs` (~392-420)
- Test: `src/repository/remote/sync_tests.rs`, `src/repository/comment_publication_tests.rs`; E-7

**Interfaces:**
- Consumes: Task 9's `index_context` row, Task 10's `index_contexts`.
- Produces: `fn synchronization_index_hand_off(&self, request: &SynchronizeRemoteRequest, changed: &[ContextKey], outcome: SynchronizationOutcome) -> Result<SynchronizationResult, SynchronizationError>`: indexes the contexts a merge or fast-forward changed inside the local-update lease block; for a push-only or already-current outcome takes the lease briefly and records the hand-off when the context's stored basis equals the current one and it is not dirty, else indexes it. The comment receipt is proved once: `publish_comment_with_config` skips `prove_comment_receipt` when the receipt is already recorded with the same commit.

- [ ] **Step 1: Write failing tests** in `sync_tests.rs`: `a_sync_that_merged_main_indexes_only_its_context`, `a_push_only_sync_records_its_hand_off_without_observing`, `a_push_only_sync_indexes_a_context_whose_basis_moved`, `a_crashed_sync_hand_off_blocks_no_other_item` (Table B's sync row); in `comment_publication_tests.rs`: `a_comment_receipt_is_proved_once`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and E-7's sync cases. Expected: PASS. Commit.

### Task 14: One SSH connection per remote call

**Files:**
- Modify: `src/repository/transport/remote.rs` (`transfer` ~347, `transfer_inner` ~356, `fresh_advertisement` ~120, `fetch_exact` ~133, `push_exact` ~201, `delete_exact` ~221), `src/repository/transport/operation.rs` (`connect_ssh` ~289), `src/repository/transport/callbacks.rs` (`CallbackAttempt` ~12), `src/repository/transport/operation_tests.rs` (`Checkpoint` ~31)
- Test: `tests/ssh_transport/exact.rs`, `tests/ssh_transport/synchronization.rs`, `tests/remote_synchronization.rs`; E-8

**Interfaces:**
- Produces: a remote call lists and transfers on the connection `connect_ssh` opened: the transfer runs inside its closure and reuses its `CallbackAttempt`, so `key_submissions()` stays 1 per connection and any reconnect inside libgit2 fails closed. `recheck_ssh` and `recheck_trust` still run before each transfer; a failed recheck refuses. `fetch_exact` lists and downloads on one connection. No automatic reconnect on error. `Checkpoint::Reconnected` is replaced by `Checkpoint::TransferStarted` at the point a transfer begins on the open connection; E-8 moves to it.
- `download_push_target` keeps its own upload-pack remote (sync.rs ~7005-7006 forbids reusing a listing for it).

- [ ] **Step 1: Write failing tests** in `tests/ssh_transport/exact.rs`: `a_listing_and_its_transfer_share_one_connection` (counted with `accepted_sockets()` around one `fetch_exact` and one `push_exact`), and `a_second_key_submission_on_one_connection_fails_closed`.
- [ ] **Step 2: Run** them by exact name. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and E-8, each by exact name. Expected: PASS. Commit.

### Task 15: Fewer listings and inspections in a sync; the SSH connection-count case

**Files:**
- Modify: `src/repository/remote/sync.rs` (`fetch_and_observe` ~7925-7974: drop the "before" listing, keep "after"; `synchronization_push_observation` ~6953 shares the push connection when nothing local changed since; `inspect_synchronization_local` call sites: merge ~6013 with ~6187, drop ~7934/~7944/~7954 inside the fetch window, keep one before each local update and before the push), `src/repository.rs` (`validate_context_tree` ~9741, cached by tree OID with kind, item, configuration blob and build)
- Test: `src/repository/remote/sync_tests.rs`; `tests/remote_synchronization.rs` (the one SSH case)

- [ ] **Step 1: Write failing tests**: `a_tree_validation_is_cached_by_tree_kind_item_configuration_and_build` (library); and the SSH case `a_pushing_sync_uses_four_connections_and_a_merging_one_five` in `tests/remote_synchronization.rs`, modelled on `context_first_current_fast_forward_local_ahead` (~386) and `context_divergence(true)` (~706), counting `accepted_sockets()`. It needs SSH because connection reuse is transport behaviour.
- [ ] **Step 2: Run** them by exact name. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests, `remote_change_during_fetch`, and E-7's sync cases. Expected: PASS. Commit.

### Task 16: Phase 2 close

**Files:**
- Modify: `docs/RFC/git-workflow-and-conflict-recovery.md` (the sync refresh steps; the upload-pack count)

- [ ] **Step 1:** Write the Git RFC amendments.
- [ ] **Step 2: Measure** with `scripts/measure-writes` (SSH mode); record `item sync` and `comment add` against the 2–4 s target and the connection counts. Profile one `comment add`.
- [ ] **Step 3:** A fresh reviewer reviews the Phase 2 diff; fix; checkpoint comment. Commit and publish.

## Phase 3: promote and close

### Task 17: Registry migration once per process; audits by use

**Files:**
- Modify: `src/repository.rs` (registry open ~4251-4274; `RepositoryService` gains the memo), `src/repository/keys/registry.rs` (`list_shared_keys` ~329 and the other key verbs ~143, ~413, ~525, ~594), the other `migrate_registry` callers (`registration_removal.rs` ~332, `bridges/abandon.rs` ~789/~981, `mutation/records.rs` ~922, `context_retirement/execution.rs` ~273, `src/repository.rs` ~3662, ~3688, ~3707, ~3783, ~3854, ~3971, ~4078, ~4098, ~4190, ~4211, ~11347), `src/repository/local_sync/record.rs` (`audit` ~469, `unfinished_context` ~1091, `require_no_pending` ~1104, `refresh_allowed` ~1116), `src/repository/local_sync/image_proof.rs` (`audit` ~237)
- Test: library tests in `src/repository/keys/` and `src/repository/local_sync/`

**Interfaces:**
- Produces: `fn open_migrated_registry(&self, operation: RepositoryOperation) -> Result<rusqlite::Connection, RepositoryError>` on `RepositoryService`: migrates only when the memo (registry file device and inode, and `PRAGMA schema_version`) differs from the file now; every caller above uses it. `list_shared_keys` takes a read guard. `local_sync::record::audit_schema(db)` (markers, schema SQL, foreign keys, integrity) runs once per memo; `local_sync::record::audit_unfinished(db)` audits every unfinished row (all items) and is what every guard calls; completed rows are audited when a read uses them (`read/status.rs` ~302 audits what it reads). Registry open audits the schema and the unfinished rows (decision 9).

- [ ] **Step 1: Write failing tests** `key_listing_runs_no_migration` (a counter in `migrate_registry`), `a_replaced_registry_file_migrates_again`, `a_guard_refuses_a_malformed_unfinished_row_of_another_item`, `registry_open_audits_unfinished_rows_and_not_completed_ones`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and the existing local-sync audit tests by name (`--lib repository::local_sync::record::` filter, exact names from the module). Expected: PASS. Commit.

### Task 18: Two previews, one index update per exit, one full cleanup check

**Files:**
- Modify: `src/repository/promotion/run.rs` (`promote_inner` ~427: reuse the observation from ~492 for `cleanup_only` ~659; ~679 and ~710 read from the integration preview; `child` ~1025 records the child's index debt; every exit: `finish_integrated` ~1136-1160, abandon ~687/~718/~758, interrupted ~656/~675/~702, PrimaryAdvanced ~755, the reconciliation in `run_observed` ~348-408; cleanup prechecks ~1261, ~1340, ~1432), `src/repository/promotion/integrate.rs` (`prepare_integration_for` ~110: no observation when `resume_written` wins), `src/repository/closure/observe.rs` (`preview` ~268, `prepare_observation_at` ~289-300), `src/repository/promotion/observe.rs` (`prepare_for` ~199-215), `src/repository/cleanup.rs` (`remove_context_worktree` ~265-272)
- Test: `src/repository/closure/run_tests.rs`, `src/repository/promotion/run_tests.rs` (`Fixture::remote()`, `set_scan_hook` counting as `spec4_assert_unleased_scans` ~3371 does)

**Interfaces:**
- Produces: `fn item_preview_facts(service: &RepositoryService, root: &Path, item: &ItemId) -> Result<PreviewFacts, RepositoryError>` replacing `observe_root` in both previews, with `pub(super) struct PreviewFacts { kind_mismatch: bool, item_path: Option<PathBuf>, other_item_blockers: Vec<RootObservationProblem> }`: the two facts each preview takes today (closure `not_ticket` and `ticket_path`, promotion `not_document` and `document_path`) from the item's own context and the primary branch, and the blockers that depend on other items (duplicate IDs, nonconforming content) from the index after the basis check of `fresh_relationships`. The parent indexes, on every exit, its own contexts and those its children changed (`index_contexts` under the lease).
- Ruling P-1 (cleanup): one full `precheck_context` under the lease before the remote branch is deleted (~1340), and a head-and-status re-check immediately before worktree removal (~1432), replacing four full checks. The design says one check immediately before removal; it moves earlier because the remote delete comes first and a dirty worktree must stop it. Recorded for the owner in the plan's approval message.

- [ ] **Step 1: Write failing tests**: `a_confirmed_close_takes_at_most_two_previews_and_no_whole_repository_observation` and the same for promotion (counted with `set_scan_hook`); `every_lifecycle_exit_indexes_the_contexts_its_children_changed` (a table over finish, abandon, cancel, a stop at awaiting publication and its later finish, interrupted, primary advanced); `a_dirty_worktree_stops_the_remote_delete`; `a_preview_reads_a_moved_other_item_from_disk`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and the re-run list for the no-new-socket checks (`tests/ticket_closure/journeys.rs` ~222, ~1372; `tests/document_promotion/advanced.rs` ~247, each by exact name). Expected: PASS. Commit.

### Task 19: Phase 3 close and the skills

**Files:**
- Modify: `docs/RFC/cli-contract.md` (the C5 known limits and follow-ups this change settles), `skills/manyhands-finding-work/SKILL.md` ("Is what I see current"), `skills/manyhands-polling-and-index/SKILL.md`, `skills/manyhands-recovery/SKILL.md` (a crashed write no longer blocks refresh and poll; `context sync-local` still does)
- Test: `tests/cli/skills.rs` drift tests

- [ ] **Step 1:** Write the amendments; verify each changed skill claim on this branch's build in a scratch repository.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::`. Expected: PASS.
- [ ] **Step 3: Measure** all commands with `scripts/measure-writes`; record against every target; profile a close and record that `image_proof::audit` and the repeated previews are absent.
- [ ] **Step 4:** A fresh reviewer reviews the Phase 3 diff; fix; checkpoint comment with the full before/after table. Commit and publish.

### Task 20: Review-ready

- [ ] **Step 1:** `git fetch origin && git merge origin/main` if main moved; `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] **Step 2: Static gates:** `devenv shell -- cargo check --all-features --locked`; `devenv shell -- cargo fmt --check`; `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. Expected: clean.
- [ ] **Step 3: Fast tier** exactly as AGENTS.md lists it (with `-- --skip repository::local_sync::`), then `--lib repository::local_sync::`. Expected: PASS.
- [ ] **Step 4: SSH tier:** every SSH-harness target (`ssh_fixture`, `cli_remote`, `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `mutation_remote`, `comment_publication`, `remote_polling`, `document_promotion`, `ticket_closure`), each as `cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5:** A fresh whole-branch reviewer against the design, Tables A and B, and the Acceptance; fix and re-review the fixes.
- [ ] **Step 6:** Set the ticket's status to `review_ready`; add the review-ready comment (gates, tiers, measurements against targets, review result, the one-way migration note for the merge). Ask the owner whether to open a PR.
- [ ] **Step 7:** After the owner's approval: ask which status the ticket ends with, save it, prepare `ticket close`, show the preview and wait; confirm only after the owner's OK (it merges into main and publishes). Native CI runs after merge, by the owner's dispatch.

## Plan self-review

| Design item | Task |
| --- | --- |
| Decision 1 targets, Measurement | 1, 12, 16, 19 |
| Decisions 2–3, Goals 1–2, Phase 1 table | 4, 7, 10, 11 |
| Decision 4, Goal 3, R-1, R-2 | 3 |
| Table A, R-4 | 2, 5, 6 |
| Table B, R-3, decision 10, decision 14 | 8, 9, 13 |
| Freshness on read, decision 8 | 7 |
| Reads that decide on other items, decision 11 | 11, 18 |
| Crash and retry, R-7, decision 12 | 10 |
| Phase 2, Goal 4 | 13, 14, 15 |
| Phase 3, Goals 5–6, decision 9, R-5, R-6 | 17, 18 |
| Document amendments | 12, 16, 19 |
| Risks: one-way migration | Global Constraints, 2, 20 |
| Acceptance (ticket) | 3 (activity), 11 (refresh counts), 12/16/19 (profiles, numbers) |
