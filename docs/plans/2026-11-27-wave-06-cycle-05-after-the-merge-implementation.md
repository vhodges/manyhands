---
id: 01M8G1CY7XE733CHN0XEF52J78
title: 'Wave 06 Cycle 05 Implementation Plan: After The Merge'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 05 Implementation Plan: After The Merge

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After a forge merge in any style, record when each item reached the primary at every Manyhands fast-forward of the local primary, report the poll's arrivals with the dependents they made ready, classify every context as done or in progress, end a done context through the confirmed `context cleanup`, and warn when the registered checkout is not on the primary.

**Architecture:** One function, `item_merges::arrivals`, walks a fast-forward's first-parent range and finds the items whose done state arrived; each move of the local primary records them in a new registry table, `item_merges`, which `ticket show` and `document show` read and the poll's binding lists (Parts A to C). The poll's context classification reads the local primary after the primary moved and adds the dispositions `merged` and `unheld` (Part D). `context cleanup` is a new confirmed verb over a local record, whose behaviour is the required outcomes CC1 to CC22 and whose mechanism Task 7's checkpoint settles (Part E). `repo inspect` and `poll once`'s human output use the poll's own checkout guard (Part F).

**Tech Stack:** Rust, `git2` 0.20.4 (`Revwalk`), `rusqlite`, the SSH test harness for steps added to one existing case.

**Spec:** [Cycle 05 design](2026-11-27-wave-06-cycle-05-after-the-merge-design.md) (revision 1), under the [Cycle document](../Cycles/wave-06-cycle-05-after-the-merge.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft**, revision 1, for review. Not approved.

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the named test edits, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M876851FT3NPSH9KDXKXR1AJ` on `manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop compile checks run with `--features desktop-test-support --tests`, on their own.

**Registry.** The registry migrations are one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. A golden written by an SSH case is rewritten by that case alone, by exact name, with `MANYHANDS_UPDATE_GOLDEN=1`. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds no SSH case; Task 11 adds steps to Cycle 03's one case (design R-15), because the deletion is a transport write and no non-SSH remote can be selected (design Baseline Fact 20).
- The fast tier and the SSH tier run once, in Task 13; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: clocks are injected, and no test sleeps to age anything.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger with the stub's description. A table test's rows are each shown red, by one run per stub (a table reports every failing row). Only a row that pins unchanged behaviour (the "as today" rows of design Part D, the five-field decoding of E-2) is exempt, and is recorded as such.

**Test edits.** The design's named edits E-1 to E-19, and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-20 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner. A golden rewrite is logged with its diff's summary, and a rewrite that changes more than the fields of design Part G stops for the owner.

**Intermediate commits.** Every commit compiles and passes the static gates and the registration tests: a function with no production caller yet is added in the task that calls it, an enum value's exhaustive matches are given arms in the task that adds it, a schema and its registrations land with its first golden, and a schema field and the goldens that carry it land together.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 5 and 13, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`, or the implementer model's own trailer when another model wrote it.

**Static gates** (Task 13, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-ssepw after Tasks 0, 3, 6, 8 and 11, and at review-ready. Each comment covers every task since the last one: its commits, the tests that ran, the design checkpoints and the edits applied. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M876851FT3NPSH9KDXKXR1AJ --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- `merged_at` comes from the range only: no commit outside `before..after`'s first-parent chain is read, and a merge commit, a squash and a fast-forward each give the time design R-3 names. Tasks 2 and 9 pin it.
- Every move of the local primary records, and a move never fails or changes its answer because of the record. Task 3 pins it.
- A context whose item is done is never retired while its remote branch survives or its proof fails, and always gets `merged` or `unheld`. Tasks 5 and 9 pin it.
- `context cleanup` never deletes a branch that moved since the last fetch, never deletes without a consent whose `unheld` list it still matches, never retires before its deletion was observed, and never lets a synchronization or a poll bring the branch back between the two. Tasks 7, 8 and 11 pin it.
- Old stored poll answers still decode and replay, and a retiring poll's frozen report is unchanged. Task 6 pins it.
- Direct mode behaves as before, except the records, the named edits and `unheld` (design R-10). The fast tier and the SSH tier pin it in Task 13.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger | — |
| 1 | `src/repository/item_merges.rs` (new; the table's functions), `src/repository.rs` (the module), `src/repository/discovery.rs` (`migrate_item_merges`, its call, its unit test); E-1 | 0 |
| 2 | `src/repository/item_merges.rs` (`arrivals`, the counter), its tests (`item_merges_tests.rs`) | 1 |
| 3 | `src/repository/item_merges.rs` (`record_after_move`), `remote/poll.rs` (`poll_fast_forward_unlocked`), `remote/sync.rs` (`reconcile_pending_candidate`, `integrate_divergence`, `synchronize_remote_observed_inner`), `promotion/run.rs` (after `L::apply`), `setup/poll.rs` (`activate`), their tests | 2 |
| 4 | `src/repository/read/dto.rs` (`ClosureDto`, `ItemDto`), `read/items.rs` (`fill_complete_read`, the literals), `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs`, the literals and goldens of E-5 and E-6, `tests/read_items.rs` | 3 |
| 5 | `remote/poll.rs` (`classify_local_target`, `completed_deleted_context`, `PollDisposition`, `guidance`, `decode_frozen`), `remote/state.rs` (the catalog and its upgrade), `mutation/bind/poll.rs`, `mutation/dto.rs` (`PollTargetDisposition`), `schemas/v1/poll_mutation.schema.json`, `remote_history.schema.json`, the desktop's arms, `remote/poll_tests.rs`, `remote/state_tests.rs`; E-7, E-8, E-9 (dispositions), E-15 to E-18 | 4 |
| 6 | `mutation/bind/poll.rs` (`poll_data`, `reported`), `mutation/bind/poll_retirement.rs`, `mutation/dto.rs` (`PollMutationDto`, `PollMergedDto`, `from_stored`), `schemas/v1/poll_mutation.schema.json`, `src/bin/manyhands-cli/polling.rs`, the literals and goldens of E-4 and E-6, `bind/poll_tests.rs`; E-2, E-3 | 5 |
| 7 | `mutation/command.rs`, `mutation/mod.rs`, `mutation/bind/cleanup.rs` (new), `mutation/bind/mod.rs`, `mutation/dto.rs` (the preview, the data, `LifecycleBlocker`), `mutation/settle.rs`, `mutation/records.rs`, `mutation/progress.rs`, `recovery.rs` (the action and steps), `src/repository/context_cleanup.rs` (new; the domain), `schemas/v1/context_cleanup_preview.schema.json`, `context_cleanup_mutation.schema.json`, the blocker and operation schemas, the CLI's `catalog.rs`, `dispatch.rs`, handler and human renderer, the desktop's arms, `tests/cli/inventory.rs`, `tests/mutation_contract.rs`, `tests/support/conformance.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/support/confirmation.rs`, `tests/cli/skills.rs`, the `cli_v1` goldens; E-9 (blockers, effects), E-10 to E-13 | 6 |
| 8 | `src/repository/context_cleanup.rs` (confirm, the remote step, the local retirement), the seam and its test fake, `context_cleanup_tests.rs` | 7 |
| 9 | `src/repository/context_cleanup_tests.rs` or a lifecycle test module beside the Wave 06 lifecycle tests (P8) | 8 |
| 10 | `remote/poll.rs` (`root_checkout`), `read/admin.rs`, `results.rs` (`ProblemCode`), `schemas/v1/problem.schema.json`, `index_problem.schema.json`, `src/bin/manyhands-cli/polling.rs`, `tests/read_repository.rs`, `polling_tests.rs`; E-9 (problem codes) | 9 |
| 11 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/support/mutation_golden.rs`, the `mutation_v1` goldens; E-12 (`CASES`), E-19 | 10 |
| 12 | `docs/RFC/cli-contract.md`, `docs/RFC/pull-request-mode.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`, `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`; E-14 | 11 |
| 13 | — | 12 |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-27-wave-06-cycle-05-after-the-merge-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and `HEAD` before and after in the ledger.
- [ ] **Step 2:** Re-check every name, path and line of design Baseline Facts 1 to 22 and Parts A to G at the new `HEAD`, in a ledger table with its current file and line. A name that changed shape stops for the owner.
- [ ] **Step 3:** Confirm the test module paths: `repository::item_merges` tests, `repository::remote::poll_tests`, `repository::remote::state_tests`, `repository::mutation::bind::poll` tests, `repository::mutation::dto` tests, the CLI's human-output test modules (`human_tests`, `polling_tests`), and the targets `read_items`, `read_repository`, `read_contract`, `mutation_contract`, `mutation_confirmation`, `cli`, `repository_enablement`, `recovery_foundation_gate`, `mutation_remote`, `cli_remote`, `remote_polling`.
- [ ] **Step 4: Premise sweep.** Grep for the design's sweep list (Named Edits, last paragraph). List every golden that carries a poll answer or an item, with the test that writes it and whether that test is an SSH case. Add each hit beyond E-1 to E-19 to the ledger as E-20 onwards. Settle U-6. An edit that reverses an intent is listed for the owner before its task.
- [ ] **Step 5:** Read the rows of E-7 and E-8 and write down, per row, the disposition it pins today and the one Part D gives.
- [ ] **Step 6:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 7:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Arrival Table (Design Part A)

**Files:** `src/repository/item_merges.rs` (new: `ItemMerge`, `merge_of`, `merges_by`, `record`), `src/repository.rs` (the module), `src/repository/discovery.rs` (`ITEM_MERGES_MARKER`, `ITEM_MERGES_TABLE`, `migrate_item_merges` after `migrate_pull_request_links` in `migrate_registry`, and its unit test). Named edit: E-1.

**Interfaces:** Produces `ItemMerge { kind: AuthoringKind, item_id: ItemId, commit: Oid, merged_at: OffsetDateTime, operation: OperationId }`, `merge_of(&Connection, i64, AuthoringKind, &ItemId) -> rusqlite::Result<Option<ItemMerge>>`, `merges_by(&Connection, i64, &OperationId) -> rusqlite::Result<Vec<ItemMerge>>`, `record(&Transaction, i64, &OperationId, &[Arrival]) -> rusqlite::Result<()>` (`Arrival` from Task 2 is declared here with its fields, and filled there).

- [ ] **Step 1: Write the failing test** `discovery.rs` `item_merges_migration_creates_its_table_once` (P1), copying `pull_request_links_migration_creates_its_table_once`'s six checks for `item_merges`, and a `record` then `merge_of` round trip with an upsert replacing the row.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --lib repository::discovery::tests::item_merges_migration_creates_its_table_once -- --exact` (Task 0 confirms the path). Expected: FAIL (no table).
- [ ] **Step 3: Implement** the table, the migration and the functions.
- [ ] **Step 4: Run** the test again. Expected: PASS. Then E-1: `devenv shell -- cargo test --all-features --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema -- --exact`, red before the list gains `item_merges`, green after.
- [ ] **Step 5: Test the rebuild:** a `read_items` or `discovery_rebuild` test (Task 0 picks the target that already drives `index rebuild`) that records a row, rebuilds, and reads it back. Red against a stub that deletes `item_merges` in `persist_observation`; green without it.
- [ ] **Step 6: Commit** "repository: the item_merges table".

## Task 2: The Arrival Rule (Design Part A, R-1 to R-4)

**Files:** `src/repository/item_merges.rs` (`arrivals`, the test-only counter), `src/repository/item_merges_tests.rs` (new).

**Interfaces:** Produces `arrivals(&git2::Repository, before: Oid, after: Oid) -> Result<Vec<Arrival>, git2::Error>`, with `Arrival { kind, item_id, commit: Oid, merged_at: OffsetDateTime }`, and `#[cfg(test)] take_walked() -> usize`.

- [ ] **Step 1: Write the failing table test** `arrivals_follow_the_range` (P2), with a local `git2` repository: each row builds `before` and `after` and names the arrivals and times expected. Rows: a merge commit, a squash and a fast-forward, each for a ticket's closure, a new document and an edit of a merged document; two arrivals in one range at different commits; a ticket file changed with no `closed_at`; a ticket whose `before` already had `closed_at`; a document deleted; a document moved; `before` not an ancestor of `after`; `before` equal to `after`; a candidate no first-parent commit changed (R-4); a fast-forward whose first-parent chain does not pass through `before`. Each row also asserts `take_walked()` equals the number of first-parent commits of `after` not reachable from `before`, never more.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --lib repository::item_merges::tests::arrivals_follow_the_range -- --exact`. Expected: FAIL (no function).
- [ ] **Step 3: Implement** `arrivals` as design Part A's steps 1 to 4.
- [ ] **Step 4: Run** again. Expected: PASS. Then show each row's red run against two stubs, recorded in the ledger: one that walks every parent (the merge commit rows give the branch commit's time), one that takes the oldest commit instead of the newest (the two-arrival and ticket-save rows fail).
- [ ] **Step 5: Commit** "repository: arrivals over a fast-forward's range".

## Task 3: Recording At Every Move (Design Part A, R-5, R-6)

**Files:** `src/repository/item_merges.rs` (`record_after_move`), `remote/poll.rs` (`poll_fast_forward_unlocked`), `remote/sync.rs` (`reconcile_pending_candidate`, `integrate_divergence`, `synchronize_remote_observed_inner`), `promotion/run.rs` (after `L::apply`), `setup/poll.rs` (`activate`); tests beside each site's existing tests.

**Interfaces:** Produces `record_after_move(service: &RepositoryService, root: &Path, operation: &OperationId, before: Oid, after: Oid)`, which returns nothing and drops its own failures.

- [ ] **Step 0: Design checkpoint.** Write in the ledger, per site: the call, the operation ID it passes, how it knows the moved ref is `refs/heads/<primary>`, and its `before`; for the lifecycle runner, how the old primary survives `write_commit` (U-3). The orchestrator approves it before code.
- [ ] **Step 1: Write the failing test** `every_move_of_the_primary_records_its_arrivals` (P3): one row per site (the poll through `PollFixture` and `synthetic_fetch`; `repo sync`'s clean path and its `integrate_divergence` stages through the existing `remote/sync.rs` test support Task 0 names; a direct close through the closure run tests' fixture; a setup's activation through `setup/test_world.rs`), each asserting one `item_merges` row with that operation's ID; and a row where a deliberate failure injected into `record` leaves the verb's answer unchanged.
- [ ] **Step 2: Run** it by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** `record_after_move` and the calls.
- [ ] **Step 4: Run** again. Expected: PASS. Each row shown red once by removing its site's call.
- [ ] **Step 5:** Run by exact name the existing tests of each site that Task 0 listed as exercising the move (no whole module). Expected: PASS.
- [ ] **Step 6: Commit** "repository: every move of the primary records its arrivals". The orchestrator adds the checkpoint comment for Tasks 1 to 3.

## Task 4: `merged_at` In Reads (Design Part B, R-7, R-26)

**Files:** `src/repository/read/dto.rs` (`ClosureDto.merged_at`, `ItemDto.merged_at`), `read/items.rs` (`fill_complete_read`, its literals), `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs` (`item_show`), the literals of E-6 (`ClosureDto`, `ItemDto`), the goldens of E-5, `tests/read_items.rs`, `bin/manyhands-cli/human_tests.rs`. Named edits: E-5, E-6 (`ClosureDto`, `ItemDto`).

- [ ] **Step 1: Write the failing tests** in `tests/read_items.rs`: `ticket_show_gives_merged_at_on_its_closure` and `document_show_gives_merged_at` (P4), each recording a row through `item_merges::record`, then asserting `show`, `show --path` and the list (null), a closing ticket (null) and an item with no row (null); and in `human_tests.rs` a `Merged at:` line.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the fields, the schema, the fill and the human line; apply E-6's literals.
- [ ] **Step 4:** Rewrite E-5's goldens with `MANYHANDS_UPDATE_GOLDEN=1` by the tests that write them (Task 0's list), each by exact name; read every diff, which must add only `merged_at`. Run `read_contract`'s schema tests by exact name (`every_published_object_is_closed_and_requires_all_its_properties` and the golden tests Task 0 named). Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS.
- [ ] **Step 6: Commit** "read: merged_at on a ticket's closure and a document".

## Task 5: Detection Per Context (Design Part D, R-10 to R-13)

**Files:** `remote/poll.rs` (`classify_local_target`, `completed_deleted_context`, the pre-filter, `PollDisposition`, `guidance`, `decode_frozen`), `remote/state.rs` (`POLL_DISPOSITION_SCHEMA_18`, the current catalog, `migrate_poll_dispositions`, `disposition_name`, `disposition`), `mutation/bind/poll.rs` (`dto_disposition`), `mutation/dto.rs` (`PollTargetDisposition`), `schemas/v1/poll_mutation.schema.json`, `remote_history.schema.json`, the desktop's arms (design Baseline Fact 6), `skills/manyhands-polling-and-index/references/poll-results.md` (left to Task 12), `remote/poll_tests.rs`, `remote/state_tests.rs`. Named edits: E-7, E-8, E-9 (dispositions), E-15 to E-18.

- [ ] **Step 0: Merge main** if it moved. **Design checkpoint:** write in the ledger the catalog upgrade against `migrate_poll_dispositions` and the `unpublished` upgrade (U-5), its marker, and every place of design Baseline Fact 6 the task changes. The orchestrator approves it before code.
- [ ] **Step 1: Write the failing table test** `detection_per_context` (P5) over design Part D's rows, through `LocalFixture` with tracking refs it installs and the primary moved first, per kind, with the pre-filter's misses (a document edited after its merge; a ticket whose primary copy is open; a ticket whose branch holds a later code commit). Write `the_poll_disposition_catalog_upgrades_from_18` in `state_tests.rs`, from a table at the `_18` catalog holding rows of every old value.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** Part D, the catalog and its upgrade, the two values at every place, and the desktop's compile-level arms (the desktop maps them as it maps `current`; Cycle 06 designs their display).
- [ ] **Step 4:** Apply E-7, E-8 (each row as Task 0 recorded it), E-9's disposition pins, E-15 to E-18. Run each by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS. Show the "new" rows red against a stub that keeps today's classification, and the `unheld` rows red against a stub that skips the proof.
- [ ] **Step 6:** Desktop compile check: `devenv shell -- cargo check --locked --features desktop-test-support --tests`.
- [ ] **Step 7: Commit** "poll: merged and unheld contexts".

## Task 6: The Poll's `merged` List (Design Part C, R-8, R-9)

**Files:** `mutation/bind/poll.rs` (`poll_data`, `reported`), `mutation/bind/poll_retirement.rs`, `mutation/dto.rs` (`PollMutationDto.merged`, `PollMergedDto`, `from_stored`), `schemas/v1/poll_mutation.schema.json`, `src/bin/manyhands-cli/polling.rs` (the lines, `fields_without`), the literals of E-6 (`PollMutationDto`), the goldens of E-4, `bind/poll_tests.rs`, `polling_tests.rs`. Named edits: E-2, E-3, E-4, E-6 (`PollMutationDto`).

- [ ] **Step 0: Design checkpoint.** Find where a retiring poll's answer is composed, its cached summaries included (U-4), and write down where `merged` is added for an ordinary and a retiring poll. The orchestrator approves it.
- [ ] **Step 1: Write the failing tests** (P6): `a_poll_lists_its_arrivals_with_the_dependents_made_ready` (the entries and their order, a ticket's `made_ready` with `current`, `index_pending` when the refresh is pending, a document's nulls, a poll that did not move the primary listing none, a retiring poll's `poll`), driven through the Wave 06 lifecycle forge (design Baseline Fact 19); and in `dto.rs` a test that a five-field stored answer decodes with `merged` empty and a six-field one round-trips (E-2's new form); a human-output test of the `merged:` and `made ready:` lines.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** Part C; apply E-2, E-3 and E-6's `PollMutationDto` literals.
- [ ] **Step 4:** E-4: rewrite the poll goldens with `MANYHANDS_UPDATE_GOLDEN=1` by the cases that write them, each by exact name (SSH cases one at a time), reading each diff, which must add only `merged`. Run `mutation_contract`'s golden-schema test by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS. Show the `made_ready` rows red against a stub that reads before the refresh.
- [ ] **Step 6: Commit** "poll: the merged list". The orchestrator adds the checkpoint comment for Tasks 4 to 6.

## Task 7: `context cleanup`: Checkpoint, Prepare, Preview And Registrations (Design Part E CC1 To CC9, Part G Items 1 To 3 And 9, R-19 To R-22)

**Files:** `mutation/command.rs` (`Command::ContextCleanup` and every classifier), `mutation/mod.rs` (`Mutation::ContextCleanup`), `mutation/bind/cleanup.rs` (new, a `Confirmed` binding), `mutation/bind/mod.rs`, `mutation/dto.rs` (`ContextCleanupPreviewDto`, `ContextCleanupMutationDto`, `LifecycleBlocker::NotDoneOnPrimary`, `PreviewDetailsDto`, `MutationDataDto`, `from_stored`), `mutation/settle.rs`, `mutation/records.rs`, `mutation/progress.rs`, `recovery.rs` (the local action and its steps), `src/repository/context_cleanup.rs` (new; the observation and prepare), the two schemas and the blocker, effect and operation schemas, the CLI (`catalog.rs`, `dispatch.rs`, the handler, `human_admin.rs`), the desktop's arms, and the registrations of E-9 to E-13 with the `cli_v1` goldens `context_cleanup_preview` and `context_cleanup_blocked` (JSON and text, no network). Named edits: E-9 (blockers, effects), E-10, E-11, E-12 (`HOMES`, conformance, `preview_schema`), E-13.

- [ ] **Step 0: Design checkpoint (U-1, U-2).** A designer on the most capable model writes in the ledger, before any code, the mechanism for every row CC1 to CC22, starting from design Part E's recommendation: the record and where it keeps `L`, `R`, the remote and its step (U-2); the reservation and whether `synchronize_context` serves without a change to the reservation's rules (U-1); the seam and its fake; the local retirement; and, per CC row, the code path that gives it. A separate checker on the most capable model re-reads the source for each claim. A row that cannot be met, or a mechanism that needs a change to the reservation's rules, a new remote action, a lifecycle table rebuild or the conflict verbs, stops for the owner. The orchestrator approves it before code.
- [ ] **Step 1: Write the failing table test** `context_cleanup_prepare_outcomes` (P7, CC1 to CC9) in `context_cleanup_tests.rs`: each row builds a repository with a context, a local primary and tracking refs, and asserts the answer, the preview's fields and effects, the blockers in order, and the digest's response to each field of CC8; nothing recorded where the row says so.
- [ ] **Step 2: Run** it by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the command, the binding's prepare, preview, `row` and `observe_again`, the DTOs and their schemas, the registrations and the CLI. Until Task 8 the confirm answers `recovery_required` behind an interim guard that no test reaches through the CLI.
- [ ] **Step 4:** Write the `cli_v1` goldens (preview and blocked, JSON and text) through `tests/cli` (no network) and their conformance rows; apply E-9 to E-13. Run each by exact name, and the registration tests by exact name: `tests/cli/inventory.rs`'s three counts and `DATA`, `tests/mutation_contract.rs`'s `DATA_ROOTS` and reachability tests, `tests/cli/skills.rs` `every_mutation_sits_in_one_autonomy_column`. Expected: PASS.
- [ ] **Step 5: Run** the table again. Expected: PASS. Each row red once against a stub recorded in the ledger.
- [ ] **Step 6:** Desktop compile check, as in Task 5.
- [ ] **Step 7: Commit** "context cleanup: prepare and preview".

## Task 8: `context cleanup`: Confirm, Stops And Concurrency (Design Part E CC10 To CC22, R-23 To R-25)

**Files:** `src/repository/context_cleanup.rs` (confirm, the remote step, the local retirement, the re-run), the seam and its fake, `mutation/bind/cleanup.rs` (`call`, `stop`, replay), `context_cleanup_tests.rs`.

- [ ] **Step 1: Write the failing table tests** (P7): `context_cleanup_confirm_outcomes` (CC9 to CC15, CC21), `context_cleanup_stops_and_crashes` (CC16 to CC18, CC20, each crash point by the test hooks the checkpoint named, as `setup/cancel.rs` `STOP_AFTER`), and `context_cleanup_holds_the_remote_between_delete_and_retirement` (CC19: a poll, an `item sync` and a comment publication of the repository attempted at each point between the advertisement and the retirement answer `busy` or yield, and none republishes or materializes the branch; CC22). The seam's fake records every delete and refuses a second one at another tip.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the checkpoint's mechanism; remove Task 7's interim guard.
- [ ] **Step 4: Run** again. Expected: PASS. Each row red once against a stub recorded in the ledger (among them: a delete not guarded by `R`; a retirement before the observation; a reservation released before the retirement).
- [ ] **Step 5: Commit** "context cleanup: confirm". The orchestrator adds the checkpoint comment for Tasks 7 and 8.

## Task 9: The Owner's Lifecycles (Design P8; Acceptance A1 To A3)

**Files:** a lifecycle test module beside the Wave 06 lifecycle tests (Task 0 names it), reusing their `Forge`, `Merge`, `poll` and `Peer`.

- [ ] **Step 1: Write the table test** `after_the_merge_lifecycles`: for a ticket closed in a pull-request repository, a new document and an edit of a merged document, each merge style, the branch deleted at merge and kept: the poll's `merged` entry with `merged_at` as design R-3 names it and a ticket's made-ready dependent; a deleted branch reading `closed` or `promoted` and retired by the retiring poll; a kept branch reading `merged`, then `context cleanup` deleting it and retiring the context, and the next poll finding nothing to do; a squash after which the primary moved on, for which a ticket reads `unheld` and a document as today (K-6), and `context cleanup`'s preview lists what the primary does not hold, then its confirm ends the context; and a second clone made after the merge, which has no `merged_at` and shows `closed_at`.
- [ ] **Step 2: Run** it by exact name. Expected: PASS on the code of Tasks 1 to 8. Show it red against two stubs recorded in the ledger: the poll's pre-Cycle classification, and `merged_at` from every parent.
- [ ] **Step 3: Commit** "test: the lifecycles after a merge".

## Task 10: The Checkout Warning (Design Part F, R-16 To R-18)

**Files:** `remote/poll.rs` (`root_checkout`, `RootCheckout`), `read/admin.rs` (`inspect_selected`), `results.rs` (`ProblemCode::PrimaryNotCheckedOut`, `PrimaryDirty`, their guidance), `schemas/v1/problem.schema.json`, `index_problem.schema.json`, `src/bin/manyhands-cli/polling.rs` (the warning line), `tests/read_repository.rs`, `polling_tests.rs`. Named edit: E-9 (problem codes).

- [ ] **Step 1: Write the failing tests** (P9): `repo_inspect_warns_when_the_root_is_off_the_primary` in `tests/read_repository.rs` (another branch, detached, dirty with an untracked file, an operation in progress, clean, and a repository with no publication remote, which is not warned); `root_checkout_is_the_poll_s_guard` in `poll_tests.rs`, asserting for each case that `root_checkout` and the poll's primary disposition agree; a human-output test of the poll's warning line under an `inaccessible`, a `dirty` and a `conflicted` primary.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** Part F.
- [ ] **Step 4:** E-9's problem-code pins and `every_problem_matches_the_problem_schema` by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS.
- [ ] **Step 6: Commit** "inspect and poll: warn when the root is off the primary".

## Task 11: The SSH Steps And The Goldens (Design P10, R-15)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (`a_pull_request_close_publishes_only_the_ticket_branch`), `tests/support/mutation_golden.rs` (`CASES`), the `mutation_v1` goldens `context_cleanup_ok` and a poll golden with a `merged` entry. Named edits: E-12 (`CASES`), E-19.

- [ ] **Step 1:** Add the steps after the case's last: write a merge commit of the ticket's branch into the server's bare repository (as `tests/mutation_remote.rs` about 5418-5430 writes a squash), keeping the branch; poll (the ticket reads `merged`, and the `merged` list has it); `context cleanup --prepare` and `--confirm`; then assert, from a new cursor, that the only server ref update is the deletion of the ticket's branch (its new OID zero, accepted), that the server's primary is the forge's merge, and that a last poll finds nothing to do.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`, first with `MANYHANDS_UPDATE_GOLDEN=1` to write the goldens, then without. Expected: PASS. Show it red once against a stub that deletes without the guard's expected tip, recorded in the ledger.
- [ ] **Step 3: Commit** "test: context cleanup over SSH". The orchestrator adds the checkpoint comment for Tasks 9 to 11.

## Task 12: The Contracts, RFCs, Audit And Skills (Design D-1 To D-5)

**Files:** `docs/RFC/cli-contract.md` (D-1), `docs/RFC/pull-request-mode.md` (D-2), `docs/RFC/repository-index-persistence-and-refresh.md` (D-3), `skills/` and `.agents/skills/` (D-4), `docs/research/wave-03-api-audit.md` (D-5). The RFCs are managed documents; as Cycles 01 to 04 did, they are edited on the ticket branch with Git and merge with the ticket, one line per paragraph. Tests: `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`. Named edit: E-14.

- [ ] **Step 1:** Write D-1 to D-5, each amendment dated and naming vh-ssepw.
- [ ] **Step 2: Run**, each by exact name in `--test cli`: `skills_are_well_formed`, `skill_links_stay_inside_the_set_and_resolve`, `skill_commands_are_catalogued`, `skill_json_examples_match_their_input_schema` and `every_mutation_sits_in_one_autonomy_column` (module `skills`), and the tests of `recovery_tables`, `collaboration_exits` and `inventory` that parse the contract (Task 0 named them). Expected: PASS.
- [ ] **Step 3: Commit** "docs: after the merge".

## Task 13: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved. Then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written, with `--no-fail-fast`. Expected: PASS, apart from the known desktop-scheduler load flakes, each then passed alone. Record the time.
- [ ] **Step 4: The SSH tier**, derived from `git diff --name-only origin/main...HEAD` against AGENTS.md's table; at planning `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`, `mutation_remote`, `document_promotion` and `ticket_closure` (about 95 minutes at Cycle 04's times). Each runs once as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5: Final review.** A fresh reviewer (the most capable model) reviews the whole branch against the design; a fix wave follows if needed, and after it only the targets whose code or tests changed run again.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses (through `manyhands-documents`) and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH steps, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then, through `manyhands-documents` and with the owner's go-ahead for each save and promotion (D-6): tick Cycle 05's boxes in the Wave 06 document; set this Cycle's three documents' status to done; promote the three. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3 | 9 |
| A4 | 2 |
| A5 | 3 |
| A6 | 1, 4 |
| A7 | 6 |
| A8 | 5 |
| A9 | 7, 8 |
| A10 | 10 |
| A11 | 11 |
| A12 | 7 |
| A13 | 12 |
| A14 | 13 |
| R-1, R-2, R-3, R-4 | 2 |
| R-5, R-6 | 3 |
| R-7, R-26 | 4 |
| R-10, R-11, R-12, R-13 | 5 |
| R-8, R-9 | 6 |
| R-14 | 5, 6, 7 |
| R-15 | 11 |
| R-16, R-17, R-18 | 10 |
| R-19, R-20, R-21, R-22 | 7 |
| R-23, R-24, R-25 | 8 |
| Review Focus lines | 2, 3, 5, 6, 7, 8, 9, 11, 13 |

Checked: every design Part has a task (A: 1 to 3; B: 4; C: 6; D: 5; E: 7 and 8; F: 10; G: 4 to 7 and 10); every CC row is in Task 7's or Task 8's table; the interfaces named across tasks agree (`Arrival`, `ItemMerge`, `arrivals`, `record_after_move`, `merges_by`, `PollMergedDto`, `RootCheckout`, `ContextCleanupPreviewDto`, `ContextCleanupMutationDto`); no placeholder steps; the plan adds no test target, so no CI list changes.
