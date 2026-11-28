---
id: 01M8G1CY7XE733CHN0XEF52J78
title: 'Wave 06 Cycle 05 Implementation Plan: After The Merge'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 05 Implementation Plan: After The Merge

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After a forge merge in any style, record when each item reached the primary at every Manyhands fast-forward of the local primary, report the poll's arrivals with the dependents they made ready, classify every context as done or in progress, end a done context through the confirmed `context cleanup`, end a closed ticket's link, and warn when the registered checkout is not on the primary.

**Architecture:** One function, `item_merges::arrivals`, walks a fast-forward's first-parent range and finds the items whose done state arrived; each move of the local primary records them in a new registry table, `item_merges`, which `ticket show` and `document show` read and the poll lists (Parts A to C). The poll's context classification reads the local primary after the primary moved and adds the dispositions `merged` and `unheld` (Part D). `context cleanup` is a cleanup-only path of the close and promote lifecycles: a record of the action `cleanup` that the shared runner takes straight to the direct cleanup step (Part E). `repo inspect` and `poll once`'s human output use the poll's own checkout guard (Part F).

**Tech Stack:** Rust, `git2` 0.20.4 (`Revwalk`), `rusqlite`, the SSH test harness for steps added to one existing case.

**Spec:** [Cycle 05 design](2026-11-27-wave-06-cycle-05-after-the-merge-design.md) (revision 2), under the [Cycle document](../Cycles/wave-06-cycle-05-after-the-merge.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft**, revision 2, for review. Not approved.

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the named test edits, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M876851FT3NPSH9KDXKXR1AJ` on `manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop compile checks run with `--features desktop-test-support --tests`, on their own.

**Registry.** The registry migrations are one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `devenv shell -- cargo test --all-features --locked --lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. A test whose golden or fixture the step rewrote counts as changed. Task 0 confirms every module path this plan names, and the ledger records any correction. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. A golden written by an SSH case is rewritten by that case alone, by exact name, with `MANYHANDS_UPDATE_GOLDEN=1`. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds no SSH case; Task 13 adds steps to Cycle 03's one case (design R-15), because the deletion is a transport write and no non-SSH remote can be selected (design Baseline Fact 26).
- The fast tier and the SSH tier run once, in Task 15; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: clocks are injected, and no test sleeps to age anything.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger with the stub's description. A table test's rows are each shown red, by one run per stub (a table reports every failing row). Only a row that pins unchanged behaviour (the "as today" rows of design Part D, E-2's five-field decoding, the old frozen report) is exempt, and is recorded as such.

**Test edits.** The design's named edits E-1 to E-22, and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-23 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner. A golden rewrite is logged with its diff's summary, and a rewrite that changes more than the fields of design Part G stops for the owner.

**Intermediate commits.** Every commit compiles and passes the static gates and the registration tests: a function is added in the task that first calls it from production code, an enum value's exhaustive matches are given arms in the task that adds it, a schema and its registrations land with its first golden, a schema field and the goldens that carry it land together, and a new verb lands with its taxonomy row, its `autonomy.md` cell and the `help_root` goldens.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 4, 7 and 15, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M876851FT3NPSH9KDXKXR1AJ`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`, or the implementer model's own trailer when another model wrote it.

**Static gates** (Task 15, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-ssepw after Tasks 0, 3, 6, 7 (the design checkpoint's record, before any code of Part E), 10 and 13, and at review-ready. Each comment covers every task since the last one: its commits, the tests that ran, the design checkpoints and the edits applied. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M876851FT3NPSH9KDXKXR1AJ --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- `merged_at` comes from the range only: only the first-parent commits of `before..after` are diffed, and a merge commit, a squash and a fast-forward each give the time design R-3 names. Tasks 1 and 11 pin it.
- Every move of the local primary records, and a move never fails or changes its answer because of the record. Tasks 1 and 2 pin it.
- A context whose item is done is never retired while its remote branch survives or its proof finds a mismatch; it reads `merged` or `unheld` when its local and remote tips agree, as design Part D lists (K-6, K-9 and K-14 are its exceptions). Tasks 5 and 11 pin it.
- `context cleanup` never deletes a branch that moved since the last fetch, never deletes or retires without a consent whose `L`, `R` and `unheld` it still matches, never loses a commit made after consent, and never lets a synchronization or a poll bring the branch back while it holds the reservation. Tasks 8, 9 and 13 pin it.
- A stuck cleanup always has a way out: the same request, or `operation abandon` at the steps CC24 allows, and no other lifecycle record becomes abandonable. Task 10 pins it.
- Old stored poll answers still decode and replay, and a retiring poll's frozen report and an old frozen image are unchanged. Tasks 5 and 6 pin it.
- Every direct close, promote and branch record behaves as before. The runner tests, and the `ticket_closure` and `document_promotion` targets in Task 15, pin it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger | — |
| 1 | `src/repository/item_merges.rs` (new: `Arrival`, `arrivals`, `record`, `record_after_move`, the counter), `item_merges_tests.rs`, `src/repository.rs` (the module), `src/repository/discovery.rs` (`migrate_item_merges`, its call, its unit test), `remote/poll.rs` (`poll_after_fetch_into`), `remote/poll_tests.rs`; E-1 | 0 |
| 2 | `remote/sync.rs` and its tests, `promotion/run.rs`, `closure/run_tests.rs`, `promotion/run_tests.rs`, `setup/poll.rs` and its tests | 1 |
| 3 | `item_merges.rs` (`ItemMerge`, `merge_of`), `read/dto.rs` (`ClosureDto`, `ItemDto`), `read/items.rs` (`fill_complete_read`, the literals), `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs`, the literals and goldens of E-4 and E-5, `tests/read_items.rs`, `tests/discovery_rebuild.rs`; E-4, E-5 (`ClosureDto`, `ItemDto`), E-21 | 2 |
| 4 | `remote/state.rs` (`POLL_DISPOSITION_SCHEMA_18`, the current catalog, `migrate_poll_dispositions`), `remote/state_tests.rs`; E-14, E-15 (the disposition marker) | 3 |
| 5 | `remote/poll.rs` (`classify_local_target`, `completed_deleted_context`, the pre-filter, `PollDisposition`, `guidance`, `decode_frozen`), `remote/state.rs` (`disposition_name`, `disposition`), `mutation/bind/poll.rs` (`dto_disposition`), `mutation/dto.rs` (`PollTargetDisposition`), `schemas/v1/poll_mutation.schema.json`, `remote_history.schema.json`, the desktop's arms, `remote/poll_tests.rs`; E-6, E-7, E-18, E-19 (dispositions) | 4 |
| 6 | `item_merges.rs` (`merges_by`), `mutation/bind/poll.rs` (`reported`), `context_retirement/pass_report.rs` (`finish`), `mutation/dto.rs` (`PollMutationDto`, `PollMergedDto`, `from_stored`), `schemas/v1/poll_mutation.schema.json`, `src/bin/manyhands-cli/polling.rs`, the literals and goldens of E-3 and E-5, the lifecycle tests; E-2, E-3, E-5 (`PollMutationDto`), E-8 (the poll's enums) | 5 |
| 7 | `closure/record.rs` and `promotion/record.rs` (the cleanup action, columns, CHECKs, steps, rebuilds, holders), `closure/record_tests.rs`, `promotion/record_tests.rs`, `tests/recovery_foundation_gate.rs`; E-15 (the lifecycle markers), E-16 | 6 |
| 8 | `mutation/command.rs`, `mutation/mod.rs`, `mutation/bind/lifecycle_verb.rs` (`CleanupTicket`, `CleanupDocument`), `mutation/bind/lifecycle.rs`, `mutation/bind/resume.rs`, `mutation/dto.rs` (the preview, the data, `LifecycleBlocker::NotDoneOnPrimary`), `recovery.rs` (`LIFECYCLE_COMMANDS`), `mutation/progress.rs`, `closure/lifecycle.rs` and `promotion/lifecycle.rs` (the cleanup lifecycles, the observation), `promotion/run.rs` (the third path's local part), the two schemas and the blocker schema, the CLI (`catalog.rs`, `dispatch.rs`, the handler, `human_admin.rs`), `docs/RFC/cli-contract.md` (the taxonomy row), `skills/manyhands-overview/references/autonomy.md` and `.agents/skills/…` (the cell), the desktop's arms, the registrations and goldens of E-8 to E-12 | 7 |
| 9 | `promotion/run.rs` (the third path's reservation, delete and re-run checks), `promotion/lifecycle_record.rs` (`complete_record`), `remote/reservation.rs` (`finish_parent`'s join), `make_outcome`, the holders' sites, `closure/run_tests.rs`, `promotion/run_tests.rs` | 8 |
| 10 | `remote/reservation.rs` (`abandonable`), `mutation/bind/abandon.rs`, the cleanup records' abandon, their tests; E-17 | 9 |
| 11 | `closure/run_pull_request_lifecycle_tests.rs`, `promotion/run_pull_request_lifecycle_tests.rs` | 10 |
| 12 | `remote/poll.rs` (`root_checkout`, `RootCheckout`), `remote/mod.rs` (the re-export), `read/admin.rs`, `results.rs` (`ProblemCode`), `schemas/v1/problem.schema.json`, `index_problem.schema.json`, `src/bin/manyhands-cli/polling.rs`, `tests/read_repository.rs`, `tests/read_contract.rs` (its fixture), `polling_tests.rs`; E-8 (the problem codes), E-20 | 11 |
| 13 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/support/mutation_golden.rs`, the `mutation_v1` goldens; E-11 (`CASES`), E-22 | 12 |
| 14 | `docs/RFC/cli-contract.md`, `docs/RFC/pull-request-mode.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`, `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`; E-13 | 13 |
| 15 | — | 14 |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-27-wave-06-cycle-05-after-the-merge-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and `HEAD` before and after in the ledger.
- [ ] **Step 2:** Re-check every name, path and line of design Baseline Facts 1 to 27 and Parts A to G at the new `HEAD`, in a ledger table with its current file and line. A name that changed shape stops for the owner.
- [ ] **Step 3:** Confirm every module path this plan names (among them `repository::discovery`'s tests, `repository::item_merges::tests`, `repository::remote::poll::tests`, `repository::remote::sync`'s and `repository::remote::state`'s tests, `repository::closure::run_tests`, `repository::promotion::run_tests`, `repository::closure::record_tests`, `repository::promotion::record_tests`, `repository::setup::poll`'s tests, `repository::closure::run_pull_request_lifecycle_tests`, `repository::promotion::run_pull_request_lifecycle_tests`, `repository::mutation::dto`'s tests, the CLI's `human_tests` and `polling_tests`), and the targets `read_items`, `read_repository`, `read_contract`, `discovery_rebuild`, `mutation_contract`, `mutation_confirmation`, `cli`, `repository_enablement`, `recovery_foundation_gate`, `mutation_remote`, `cli_remote`, `remote_polling`. Record corrections.
- [ ] **Step 4: Premise sweep.** Grep for the design's sweep list (Named Edits, last paragraph). List every golden that carries a poll answer or an item, with the test that writes it and whether that test is an SSH case. Settle U-7. Add each hit beyond E-1 to E-22 to the ledger as E-23 onwards. An edit that reverses an intent is listed for the owner before its task.
- [ ] **Step 5:** Read the rows of E-6, E-7 and E-21 and write down, per row, what it pins today and what the design gives.
- [ ] **Step 6:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 7:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Arrival Record At The Poll's Move (Design Part A)

**Files:** `src/repository/item_merges.rs` (new: `Arrival`, `arrivals`, `record`, `record_after_move`, `#[cfg(test)] take_walked`), `src/repository/item_merges_tests.rs` (new), `src/repository.rs` (the module), `src/repository/discovery.rs` (`ITEM_MERGES_MARKER`, `ITEM_MERGES_TABLE`, `migrate_item_merges` after `migrate_pull_request_links`, its unit test), `remote/poll.rs` (`poll_after_fetch_into`, after the primary's `FastForwarded`), `remote/poll_tests.rs`. Named edit: E-1.

**Interfaces:** Produces `Arrival { kind: AuthoringKind, item_id: ItemId, commit: Oid, merged_at: OffsetDateTime }`, `arrivals(&git2::Repository, before: Oid, after: Oid) -> Result<Vec<Arrival>, git2::Error>`, `record(&Transaction, i64, &OperationId, &[Arrival]) -> rusqlite::Result<()>`, and `record_after_move(service: &RepositoryService, root: &Path, operation: &OperationId, before: Oid, after: Oid)`, which returns nothing and drops its own failures.

- [ ] **Step 1: Write the failing tests.** `discovery.rs` `item_merges_migration_creates_its_table_once` (P1); `item_merges_tests.rs` `arrivals_follow_the_range` (P2), every row of design Testing's P2 with its expected arrivals, times and exact `take_walked()` count; `remote/poll_tests.rs` `the_poll_records_its_arrivals` (P3's poll row, through `PollFixture` and `synthetic_fetch`, asserting one row per arrival with the poll's operation ID, and that a failure injected into `record` leaves the poll's answer unchanged).
- [ ] **Step 2: Run** each:
  `devenv shell -- cargo test --all-features --locked --lib repository::discovery::tests::item_merges_migration_creates_its_table_once -- --exact`
  `devenv shell -- cargo test --all-features --locked --lib repository::item_merges::tests::arrivals_follow_the_range -- --exact`
  `devenv shell -- cargo test --all-features --locked --lib repository::remote::poll::tests::the_poll_records_its_arrivals -- --exact`
  Expected: FAIL (they do not compile until the functions exist; then FAIL on behaviour).
- [ ] **Step 3: Implement** the table, the migration, the four functions and the poll's call.
- [ ] **Step 4: Run** the three again. Expected: PASS. Show `arrivals_follow_the_range` red against two stubs, recorded in the ledger: one that diffs every parent (the counts fail, and the merge rows read a branch commit), one that takes the oldest commit (the two-arrival and ticket-save rows fail). Then E-1: `devenv shell -- cargo test --all-features --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema -- --exact`, red before the list gains `item_merges`, green after.
- [ ] **Step 5: Commit** "repository: arrivals over the poll's fast-forward".

## Task 2: Recording At Every Other Move (Design Part A, R-5, R-6)

**Files:** `remote/sync.rs` (`reconcile_pending_candidate`, `integrate_divergence`, `synchronize_remote_observed_inner`, for a `Primary` target) and its tests, `promotion/run.rs` (after `L::apply` answers `Merged`, with `prepared.evidence().old_primary` read before `write_commit`), `closure/run_tests.rs`, `promotion/run_tests.rs`, `setup/poll.rs` (`activate`, with a new `OperationId`) and its tests.

- [ ] **Step 0: Design checkpoint.** Write in the ledger, per site, the call, the operation ID it passes, and how it knows the moved ref is `refs/heads/<primary>`. The orchestrator approves it before code.
- [ ] **Step 1: Write the failing tests** (P3): `repository::remote::sync::tests::repo_sync_records_its_arrivals` (a table over the clean path, a restart and both `integrate_divergence` stages, through the existing `remote/sync.rs` test support Task 0 names), `repository::closure::run_tests::a_direct_close_records_its_arrival`, `repository::promotion::run_tests::a_direct_promote_records_its_arrival`, `repository::setup::poll::tests::a_setup_activation_records_its_arrivals`; each asserts one row per arrival with that site's operation ID.
- [ ] **Step 2: Run** each by exact name with `devenv shell -- cargo test --all-features --locked --lib <path> -- --exact`. Expected: FAIL.
- [ ] **Step 3: Implement** the calls.
- [ ] **Step 4: Run** again. Expected: PASS. Each row shown red once by removing its site's call.
- [ ] **Step 5: Commit** "repository: every move of the primary records its arrivals".

## Task 3: `merged_at` And A Closed Ticket's Link In Reads (Design Part B, R-7, R-26, Decision 10)

**Files:** `item_merges.rs` (`ItemMerge`, `merge_of`), `src/repository/read/dto.rs` (`ClosureDto.merged_at`, `ItemDto.merged_at`), `read/items.rs` (`fill_complete_read`, its literals), `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs` (`item_show`), the literals of E-5 (`ClosureDto`, `ItemDto`), the goldens of E-4, `tests/read_items.rs`, `tests/discovery_rebuild.rs`, `bin/manyhands-cli/human_tests.rs`. Named edits: E-4, E-5 (`ClosureDto`, `ItemDto`), E-21.

- [ ] **Step 1: Write the failing tests** (P4, P1's rebuild): `tests/read_items.rs` `ticket_show_gives_merged_at_on_its_closure`, `document_show_gives_merged_at` and `a_closed_ticket_s_link_is_no_longer_offered`, each writing the `item_merges` or `pull_request_links` row with `rusqlite` and asserting `show`, `show --path`, the list (null), a closing ticket and an item with no row; `tests/discovery_rebuild.rs` `an_item_merge_survives_an_index_rebuild`; a `human_tests.rs` test of the `Merged at:` line.
- [ ] **Step 2: Run** each by exact name: `devenv shell -- cargo test --all-features --locked --test read_items <name> -- --exact`, `--test discovery_rebuild an_item_merge_survives_an_index_rebuild -- --exact`, `--bin manyhands-cli <human_tests path>::<name> -- --exact`. Expected: FAIL.
- [ ] **Step 3: Implement** the fields, the schema, the fill, the link rule and the human line; apply E-5's literals and E-21.
- [ ] **Step 4:** Rewrite E-4's goldens with `MANYHANDS_UPDATE_GOLDEN=1` by the tests that write them (Task 0's list), each by exact name; read every diff, which must add only `merged_at`. Run, by exact name, the schema tests whose goldens changed (Task 0 named them). Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS.
- [ ] **Step 6: Commit** "read: merged_at, and a closed ticket's link ends". The orchestrator adds the checkpoint comment for Tasks 1 to 3.

## Task 4: The Disposition Catalog (Design Part D's Catalog, R-11)

**Files:** `remote/state.rs` (`POLL_DISPOSITION_SCHEMA_18`, the current catalog with `merged` and `unheld`, `migrate_poll_dispositions` with the marker `wave_06_poll_dispositions_merged_v1`), `remote/state_tests.rs`, `tests/recovery_foundation_gate.rs`. Named edits: E-14, E-15 (the disposition marker).

- [ ] **Step 0: Merge main** if it moved. **Design checkpoint:** write in the ledger the upgrade against design Part D's truth table, its marker checks, and each test of E-14 with the change it needs. The orchestrator approves it before code.
- [ ] **Step 1: Write the failing test** `the_poll_disposition_catalog_upgrades_by_its_truth_table` (P5), one row per line of the truth table, from tables holding rows of every old value.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --lib repository::remote::state::tests::the_poll_disposition_catalog_upgrades_by_its_truth_table -- --exact`. Expected: FAIL.
- [ ] **Step 3: Implement** the catalog and its upgrade. No Rust enum changes yet: the CHECK accepts the values before anything writes them.
- [ ] **Step 4:** Apply E-14 and E-15's disposition marker; run each edited test by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new test again. Expected: PASS.
- [ ] **Step 6: Commit** "registry: the poll disposition catalog gains merged and unheld".

## Task 5: Detection Per Context (Design Part D, R-10, R-12, R-13, Decision 8)

**Files:** `remote/poll.rs` (`classify_local_target`, `completed_deleted_context`, the pre-filter, `PollDisposition`, `guidance`, `decode_frozen`'s list), `remote/state.rs` (`disposition_name`, `disposition`), `mutation/bind/poll.rs` (`dto_disposition`), `mutation/dto.rs` (`PollTargetDisposition`), `schemas/v1/poll_mutation.schema.json`, `remote_history.schema.json`, the desktop's arms (design Baseline Fact 6), `remote/poll_tests.rs`, `tests/remote_polling.rs` (its guidance table). Named edits: E-6, E-7, E-18, E-19 (dispositions).

- [ ] **Step 1: Write the failing tests** (P5): `repository::remote::poll::tests::detection_per_context` (design Part D's rows through `LocalFixture` with tracking refs it installs and the primary moved first, among them a ticket whose branch holds a later code commit, a shallow repository, an unreadable `L`, and a root off the primary for which nothing is recorded or detected until the root is back) and `repository::remote::poll::tests::an_old_frozen_report_still_decodes`.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL (the second is exempt from red if it passes at once: it pins unchanged behaviour).
- [ ] **Step 3: Implement** Part D, the two values at every place, and the desktop's compile-level arms (the desktop maps them as it maps `current`; Cycle 06 designs their display).
- [ ] **Step 4:** Apply E-6, E-7 (each row as Task 0 recorded it), E-18 and E-19's disposition arms. Run each by exact name (the `tests/remote_polling.rs` guidance case alone, as an SSH case). Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS. Show the new rows red against a stub that keeps today's classification, and the `unheld` rows red against a stub that skips the proof.
- [ ] **Step 6:** Desktop compile check: `devenv shell -- cargo check --locked --features desktop-test-support --tests`.
- [ ] **Step 7: Commit** "poll: merged and unheld contexts".

## Task 6: The Poll's `merged` List (Design Part C, R-8, R-9)

**Files:** `item_merges.rs` (`merges_by`), `mutation/bind/poll.rs` (`reported`), `context_retirement/pass_report.rs` (`finish`), `mutation/dto.rs` (`PollMutationDto.merged`, `PollMergedDto`, `from_stored`), `schemas/v1/poll_mutation.schema.json`, `src/bin/manyhands-cli/polling.rs` (the lines, `fields_without`), the literals of E-5 (`PollMutationDto`), the goldens of E-3, `closure/run_pull_request_lifecycle_tests.rs`, `promotion/run_pull_request_lifecycle_tests.rs`, `polling_tests.rs`. Named edits: E-2, E-3, E-5 (`PollMutationDto`), E-8 (the poll's enums).

- [ ] **Step 0: Design checkpoint (U-4).** Write down where `merged` is added for an ordinary poll and for a retiring poll, inside `pass_report.rs` `finish`'s transaction, and where its `made_ready` is read after that transaction. The orchestrator approves it.
- [ ] **Step 1: Write the failing tests** (P6): `repository::closure::run_pull_request_lifecycle_tests::a_poll_lists_its_arrivals_with_the_dependents_made_ready` and `…::a_retiring_poll_lists_its_arrivals`, `repository::promotion::run_pull_request_lifecycle_tests::a_poll_lists_a_document_s_arrival`, the two `mutation/dto.rs` decoding tests (E-2's new form and an old `poll_retirement`), and a `polling_tests.rs` test of the `merged:` and `made ready:` lines.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** Part C; apply E-2's library edits and E-5's `PollMutationDto` literals; add the poll's enums to `contract_enumerations` (E-8).
- [ ] **Step 4:** E-3: rewrite the poll goldens with `MANYHANDS_UPDATE_GOLDEN=1` by the cases that write them, each by exact name (SSH cases one at a time, among them `tests/cli_remote/retirement.rs`'s, with its assertion of six fields), reading each diff, which must add only `merged`. Run, by exact name, `mutation_contract`'s golden-schema test and `an_opted_poll_matches_its_distinct_closed_schema_and_golden`. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS. Show the `made_ready` rows red against a stub that reads before the refresh.
- [ ] **Step 6: Commit** "poll: the merged list". The orchestrator adds the checkpoint comment for Tasks 4 to 6.

## Task 7: `context cleanup`: Design Checkpoint And The Records (Design Part E, Items 1, 4 And 5; R-19, R-28, R-29)

**Files:** `closure/record.rs` and `promotion/record.rs` (the action `cleanup`, the columns, CHECKs and step graph, the rebuilds under `wave_06_closure_cleanup_v1` and `wave_06_promotion_cleanup_v1`, the immutables, the holder predicates), `closure/record_tests.rs`, `promotion/record_tests.rs`, `tests/recovery_foundation_gate.rs`. Named edits: E-15 (the lifecycle markers), E-16.

- [ ] **Step 0: Merge main** if it moved. **Design checkpoint (U-2, U-5, U-6).** A designer on the most capable model writes in the ledger, before any code, the mechanism for every row CC1 to CC26, as design Part E's last section lists: the rebuilt tables' columns, CHECKs and step graph and a cleanup record's immutables; the runner's third path, its consent re-check and its head check; `finish_parent`'s join, `make_outcome` and `complete_record` for a cleanup record; the holder predicates for CC3 and CC26; the two `LifecycleVerb`s and the resume discriminator; `abandonable` and the abandon's end. A separate checker on the most capable model re-reads the source for each claim. The orchestrator approves it and posts it on the ticket before any code. A row that cannot be met, or a mechanism that needs a change to the remote reservation's rules beyond `finish_parent`'s join and `abandonable` for a cleanup record, a new remote action, a change to the transport or a change to the conflict verbs, stops for the owner.
- [ ] **Step 1: Write the failing tests** `repository::closure::record_tests::cleanup_records_rebuild_once` and `repository::promotion::record_tests::cleanup_records_rebuild_once` (P7): each rebuilds a table at today's schema holding rows of every action and step, checks every old row reads its old action, a cleanup row's CHECKs and step graph (accepted and refused transitions), idempotence and a half-done migration failing closed.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the records and their migrations.
- [ ] **Step 4:** Apply E-15's lifecycle markers and E-16; run each edited test by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS.
- [ ] **Step 6: Commit** "lifecycle: cleanup records". The orchestrator adds the checkpoint's comment if not yet posted.

## Task 8: `context cleanup`: Prepare, Preview, Registrations And A Confirm With No Remote (Design Part E Item 3, CC1 To CC8, CC14, CC15; Part G Items 1 To 3, 9 And 11; R-20 To R-22, R-30)

**Files:** `mutation/command.rs` (`Command::ContextCleanup` and every classifier, `setup_gate`), `mutation/mod.rs`, `mutation/bind/lifecycle_verb.rs` (`CleanupTicket`, `CleanupDocument`), `mutation/bind/lifecycle.rs`, `mutation/bind/resume.rs`, `mutation/dto.rs` (`ContextCleanupPreviewDto`, `ContextCleanupMutationDto`, `LifecycleBlocker::NotDoneOnPrimary`, `PreviewDetailsDto`, `MutationDataDto`, `from_stored`), `recovery.rs` (`LIFECYCLE_COMMANDS`), `mutation/progress.rs`, `closure/lifecycle.rs` and `promotion/lifecycle.rs` (the cleanup lifecycles and their observation), `promotion/run.rs` (the third path's local part), `schemas/v1/context_cleanup_preview.schema.json`, `context_cleanup_mutation.schema.json` and the blocker schema, the CLI (`catalog.rs`, `dispatch.rs`, the handler, `human_admin.rs`), `docs/RFC/cli-contract.md` (the taxonomy row only), the `autonomy.md` cell in both skill trees, the desktop's arms, the goldens `cli_v1` `context_cleanup_preview`, `context_cleanup_blocked` (JSON and text) and `context_cleanup_ok_no_remote`, and `help_root` and `help_root_text`. Named edits: E-8 (the cleanup's enums), E-9 to E-12.

- [ ] **Step 1: Write the failing table tests** `repository::closure::run_tests::context_cleanup_prepare_outcomes` and `repository::promotion::run_tests::context_cleanup_prepare_outcomes` (P7, CC1 to CC8, and CC14 and CC15 with no publication remote): each row builds a repository with a context, a local primary and tracking refs, and asserts the answer, the preview's fields and effects, the blockers in order, the digest's response to each field of CC8, nothing recorded where the row says so, and, with no remote, the local retirement and its data.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the command, the two verbs, the observation and preview, the DTOs and schemas, the registrations, the CLI, and the third path for a record with no publication remote. A confirm of a record with a remote answers `recovery_required` until Task 9; no test reaches it before then, since a confirm with a remote needs the SSH harness.
- [ ] **Step 4:** Write the `cli_v1` goldens through `tests/cli` (no network) and their conformance rows; add the taxonomy row and the `autonomy.md` cell; rewrite `help_root` and `help_root_text`; apply E-8 to E-12. Run each by exact name, and the registration tests by exact name: `tests/cli/inventory.rs` `the_catalog_and_the_rfc_taxonomies_name_the_same_verbs` and the mutating-verb, input and `DATA` tests Task 0 named; `tests/mutation_contract.rs`'s `DATA_ROOTS`, reachability and data-root tests; `tests/cli/skills.rs` `every_mutation_sits_in_one_autonomy_column`; `tests/cli/help.rs`'s root tests. Expected: PASS.
- [ ] **Step 5: Run** the tables again. Expected: PASS. Each row red once against a stub recorded in the ledger.
- [ ] **Step 6:** Desktop compile check, as in Task 5.
- [ ] **Step 7: Commit** "context cleanup: prepare, preview and a cleanup with no remote".

## Task 9: `context cleanup`: The Remote Confirm, Stops, Holds And Replay (Design Part E Item 2, CC9 To CC23, CC25, CC26; R-23, R-24, R-31)

**Files:** `promotion/run.rs` (the third path's reservation, delete, consent re-check and head check), `promotion/lifecycle_record.rs` (`complete_record`), `remote/reservation.rs` (`finish_parent`'s join for a cleanup record), `make_outcome`, the holder sites the checkpoint named, `closure/run_tests.rs`, `promotion/run_tests.rs`.

- [ ] **Step 1: Write the failing table tests** (P7), in both run-test modules, with `FakeIo` scripting the delete: `context_cleanup_confirm_outcomes` (CC9 to CC16, CC21, CC25), `context_cleanup_stops_and_crashes` (CC17 to CC20, CC23, each crash point by the runner's existing safe points and hooks), and `context_cleanup_holds_its_item_and_the_repository` (CC22: a local write, an `index refresh`, a poll, an `item sync`, a `repo sync` and a comment's publication attempted while it holds its reservation answer as CC22 says, and none republishes or materializes the branch; CC26: each retirement path skips or refuses its item, and retires it once the record finishes).
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the checkpoint's mechanism; remove Task 8's interim answer.
- [ ] **Step 4: Run** again. Expected: PASS. Each row red once against a stub recorded in the ledger (among them: a delete not guarded by `R`; a retirement before the observation; a re-run that skips the consent check; a retirement at the live tip rather than the consented `L`).
- [ ] **Step 5: Commit** "context cleanup: the confirm with a remote".

## Task 10: `context cleanup`: Abandon (Design Part E Item 5, CC24, Decision 9)

**Files:** `remote/reservation.rs` (`abandonable`), `mutation/bind/abandon.rs`, the cleanup records' abandon and its end of the reservation, `closure/run_tests.rs`, `promotion/run_tests.rs`, the abandon tests of E-17. Named edit: E-17.

- [ ] **Step 1: Write the failing table test** `context_cleanup_abandon` (P7, CC24) in both run-test modules: each allowed step abandons (the record `abandoned`, the reservation ended, the context and its branches as they were), each refused step refuses (a delete in flight; every other lifecycle record, as today).
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** it; apply E-17 and run each edited test by exact name.
- [ ] **Step 4: Run** again. Expected: PASS. The refused rows red against a stub that lets every lifecycle record be abandoned.
- [ ] **Step 5: Commit** "context cleanup: abandon". The orchestrator adds the checkpoint comment for Tasks 7 to 10.

## Task 11: The Owner's Lifecycles (Design P8; Acceptance A1 To A3)

**Files:** `closure/run_pull_request_lifecycle_tests.rs`, `promotion/run_pull_request_lifecycle_tests.rs`, each reusing its own `Forge`.

- [ ] **Step 1: Write the table tests** `repository::closure::run_pull_request_lifecycle_tests::a_ticket_after_its_merge` and `repository::promotion::run_pull_request_lifecycle_tests::a_document_after_its_merge`, as design Testing's P8 lists them, the second clone's row in the closure tests with `Peer`.
- [ ] **Step 2: Run** each by exact name. Expected: PASS on the code of Tasks 1 to 10. Show each red against two stubs recorded in the ledger: the poll's pre-Cycle classification, and `merged_at` from every parent.
- [ ] **Step 3: Commit** "test: the lifecycles after a merge".

## Task 12: The Checkout Warning (Design Part F, R-16 To R-18)

**Files:** `remote/poll.rs` (`root_checkout`, `RootCheckout`), `remote/mod.rs` (the re-export), `read/admin.rs` (`inspect_selected`), `results.rs` (`ProblemCode::PrimaryNotCheckedOut`, `PrimaryDirty`, their guidance), `schemas/v1/problem.schema.json`, `index_problem.schema.json`, `src/bin/manyhands-cli/polling.rs` (the warning line), `tests/read_repository.rs`, `tests/read_contract.rs` (`add_fixture_remotes`), `polling_tests.rs`. Named edits: E-8 (the problem codes), E-20.

- [ ] **Step 1: Write the failing tests** (P9): `tests/read_repository.rs` `repo_inspect_warns_when_the_root_is_off_the_primary`; `repository::remote::poll::tests::root_checkout_is_the_poll_s_guard`, asserting for each case that `root_checkout` and the poll's primary disposition agree; a `polling_tests.rs` test of the warning line under a root off the primary and its absence when `inaccessible` comes from the obstruction scan.
- [ ] **Step 2: Run** each by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** Part F; apply E-20 so that `repo_inspect.json` keeps `"problems": []`.
- [ ] **Step 4:** E-8's problem codes and `every_problem_matches_the_problem_schema` and `repo_inspect_matches_its_schema_and_golden` by exact name. Expected: PASS.
- [ ] **Step 5: Run** the new tests again. Expected: PASS.
- [ ] **Step 6: Commit** "inspect and poll: warn when the root is off the primary".

## Task 13: The SSH Steps And The Goldens (Design P10, R-15)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (`a_pull_request_close_publishes_only_the_ticket_branch`), `tests/support/mutation_golden.rs` (`CASES`), the `mutation_v1` goldens `context_cleanup_ok` and `poll_once_merged`. Named edits: E-11 (`CASES`), E-22.

- [ ] **Step 1:** Add the steps after the case's last: write a merge commit of the ticket's branch into the server's bare repository (as `tests/mutation_remote.rs` about 5418-5430 writes a squash), keeping the branch; poll (the ticket reads `merged`, and the `merged` list has it); `context cleanup --prepare` and `--confirm`; then assert, from a new cursor, that the only server ref update is the deletion of the ticket's branch (its new OID zero, accepted), that the server's primary is the forge's merge, and that a last poll finds nothing to do.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`, first with `MANYHANDS_UPDATE_GOLDEN=1` to write the goldens, then without. Expected: PASS. Show it red once against a stub the server can see, recorded in the ledger: a cleanup that skips the delete (the assertion of one deletion fails), or one that also pushes the ticket's branch.
- [ ] **Step 3: Commit** "test: context cleanup over SSH". The orchestrator adds the checkpoint comment for Tasks 11 to 13.

## Task 14: The Contracts, RFCs, Audit And Skills (Design D-1 To D-5)

**Files:** `docs/RFC/cli-contract.md` (D-1, beyond Task 8's taxonomy row), `docs/RFC/pull-request-mode.md` (D-2), `docs/RFC/repository-index-persistence-and-refresh.md` (D-3), `skills/` and `.agents/skills/` (D-4, beyond Task 8's cell), `docs/research/wave-03-api-audit.md` (D-5). The RFCs are managed documents; as Cycles 01 to 04 did, they are edited on the ticket branch with Git and merge with the ticket, one line per paragraph. Tests: `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`. Named edit: E-13.

- [ ] **Step 1:** Write D-1 to D-5, each amendment dated and naming vh-ssepw.
- [ ] **Step 2: Run**, each by exact name in `--test cli`: `skills_are_well_formed`, `skill_links_stay_inside_the_set_and_resolve`, `skill_commands_are_catalogued`, `skill_json_examples_match_their_input_schema` and `every_mutation_sits_in_one_autonomy_column` (module `skills`), and the tests of `recovery_tables`, `collaboration_exits` and `inventory` that parse the contract (Task 0 named them). Expected: PASS.
- [ ] **Step 3: Commit** "docs: after the merge".

## Task 15: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved. Then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written, with `--no-fail-fast`. Expected: PASS, apart from the known desktop-scheduler load flakes, each then passed alone. Record the time.
- [ ] **Step 4: The SSH tier**, derived from `git diff --name-only origin/main...HEAD` against AGENTS.md's table; at planning `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`, `mutation_remote`, `document_promotion` and `ticket_closure` (about 95 minutes at Cycle 04's times; the transport does not change, so no row adds more). Each runs once as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5: Final review.** A fresh reviewer (the most capable model) reviews the whole branch against the design; a fix wave follows if needed, and after it only the targets whose code or tests changed run again.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses (through `manyhands-documents`) and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH steps, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then, through `manyhands-documents` and with the owner's go-ahead for each save and promotion (D-6): tick Cycle 05's boxes and extend Cycle 06's list in the Wave 06 document; set this Cycle's three documents' status to done; promote the three. Leave the approval notes of the Cycle document on vh-p7st2, vh-ew8pt and vh-tmghg if not yet left. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3 | 11 |
| A4 | 1 |
| A5 | 1, 2 |
| A6 | 1, 3 |
| A7 | 6 |
| A8 | 4, 5 |
| A9 | 7, 8, 9 |
| A10 | 10 |
| A11 | 12 |
| A12 | 13 |
| A13 | 8 |
| A14 | 14 |
| A15 | 15 |
| R-1, R-2, R-3, R-4, R-5 | 1 |
| R-6 | 2 |
| R-7, R-26 | 3 |
| R-11 | 4 |
| R-10, R-12, R-13, R-27 | 5 |
| R-8, R-9 | 6 |
| R-14 | 5, 6, 8 |
| R-15 | 13 |
| R-16, R-17, R-18 | 12 |
| R-19, R-28, R-29 | 7, 9 |
| R-20, R-21, R-22, R-30 | 8 |
| R-23, R-24, R-25, R-31 | 9 |
| R-32 | 14 |
| Review Focus lines | 1, 2, 5, 6, 8, 9, 10, 11, 13, 15 |

Checked: every design Part has a task (A: 1 and 2; B: 3; C: 6; D: 4 and 5; E: 7 to 10; F: 12; G: 3, 5, 6, 8 and 12); every CC row is in Task 8's, 9's or 10's tables; each function is added with its first production caller (`arrivals`, `record` and `record_after_move` in Task 1, `merge_of` in Task 3, `merges_by` in Task 6); the interfaces named across tasks agree (`Arrival`, `ItemMerge`, `arrivals`, `record_after_move`, `merges_by`, `PollMergedDto`, `RootCheckout`, `ContextCleanupPreviewDto`, `ContextCleanupMutationDto`, `CleanupTicket`, `CleanupDocument`); no placeholder steps; the plan adds no test target, so no CI list changes.
