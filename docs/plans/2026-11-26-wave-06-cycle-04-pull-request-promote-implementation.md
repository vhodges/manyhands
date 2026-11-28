---
id: 01M8BXXH54RS9M6NX5XW9ARX9A
title: 'Wave 06 Cycle 04 Implementation Plan: Document Promote In Pull-Request Mode'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 04 Implementation Plan: Document Promote In Pull-Request Mode

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a pull-request repository, promote a document by synchronizing and publishing its branch and giving the pull request link; promote again by synchronizing again and giving the same link; count a document as promoted only by the content proof, in promote, `document show` and `item sync`.

**Architecture:** The pull-request promote is a branch record of the existing `document_promotions` lifecycle (decision 1): the record gains its mode and stored answer (`promotion/record.rs`), prepare takes a pull-request observation with its own digest (`promotion/observe.rs`), and Cycle 03's branch path in the runner (`promotion/run.rs` `promote_branch`) runs the context child and lets promotion's `branch_step` and `end_branch` answer and finish the record. One function decides whether the primary holds the document by the content proof, used by promote and `item sync` (`bind/sync.rs`); `document show` (`read/items.rs`) uses a cheaper twin (decision 9). New data and preview shapes carry the link.

**Tech Stack:** Rust, `git2`, `rusqlite`, `src/forge.rs`, the SSH test harness for steps added to one existing case.

**Spec:** [Cycle 04 design](2026-11-26-wave-06-cycle-04-pull-request-promote-design.md) (revision 7), under the [Cycle document](../Cycles/wave-06-cycle-04-pull-request-promote.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **approved** by the owner on 2026-11-26, at revision 7 (Cycle document decision 16). Execution: subagent-driven, with Sonnet 5.5 at high effort as the implementer. Implementation: **done**: the owner approved the review, and the ticket was closed and merged on 2026-11-27 at `a1aba5d5`.

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the named test edits, authorized implementation and chosen the execution method; and Cycle 03 (vh-gy8gz) has merged to main (decision 5; it was integrated at `6bc4b69c`, and main `8e5d9832` was merged into the ticket branch at `0c5027e0`). Task 0 does not start before the owner's approval. Met on 2026-11-26 (decision 16).

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87684C9GM9BJP5QQ429KMAK` on `manyhands/ticket/01M87684C9GM9BJP5QQ429KMAK`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop compile checks run with `--features desktop-test-support --tests`, on their own.

**Registry.** The registry migrations are one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds no SSH case; Task 8 adds steps to Cycle 03's one case (decision 6).
- The fast tier and the SSH tier run once, in Task 10; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: the runner takes its clock from the request.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. Only a row that pins unchanged behaviour (C24, the direct rows of P1, P2 and P13) is exempt, and is recorded as such; a new row that passes at once is run red against a deliberate stub.

**Test edits.** The design's named edits E-1 to E-14, and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-15 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner.

**Intermediate commits.** Every commit compiles and passes the static gates and the registration tests: a function with no production caller yet is added in the task that calls it, an enum value's exhaustive matches are given arms in the task that adds it, and a schema and its registrations land with its first golden. Until Task 5, a pull-request promote may answer without data, and its preview without details; direct mode passes throughout.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 3 and 10, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87684C9GM9BJP5QQ429KMAK`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 10, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-rphcs after Tasks 0, 2, 4, 6 and 8, and at review-ready. Each comment covers every task since the last one: its commits, the tests that ran, the design checkpoints and the edits applied. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M87684C9GM9BJP5QQ429KMAK --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- No pull-request promote moves the local primary or synchronizes the primary. Tasks 3, 4 and 7 pin it with the shared fake publisher that refuses a primary target, and Task 8 with the server's primary.
- A document is promoted only by the content proof: an edit of a merged document is not promoted before its own merge, and a squash is promoted after it. Tasks 2 and 7 pin it.
- Promoting again never gives a second, different link and never leaves a save behind. Task 3 pins it (C4 to C7).
- Every direct promotion behaves as before: every existing row reads `direct`. Task 1's migration test, Task 4's C24 and the `document_promotion` target pin it.
- A same-request retry after each crash point repeats no publication and answers as before. Task 4 pins it.
- `document show` stops offering the link once the primary holds the document, and its cheap check agrees with the proof. Task 6 pins it.
- A stopped pull-request promote never waits for good and never publishes what was not previewed; retirement waits for it. Task 4 pins it (C20, C22, C25, P13).

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger | — |
| 1 | `src/repository/promotion/record.rs`, `promotion/lifecycle.rs` (`impl LifecycleRecord for PromotionRecord`), `record_tests.rs`, `tests/recovery_foundation_gate.rs`; E-3, E-4 | 0 |
| 2 | `src/repository/promotion/observe.rs`, `observe_tests.rs`, `promotion/mod.rs`, `promotion/lifecycle.rs` (`Preview`, `runs_context_child`, `require_identity`), `bind/lifecycle_verb.rs` (`Promote::observe`, `Ready`), `bind/lifecycle.rs` (the preview's effects, the digest); E-12 | 1 |
| 3 | `src/repository/promotion/lifecycle.rs` (`branch_step`, `end_branch`, `Outcome`), `run.rs` (`RunEvent::Recording`), `mod.rs` (`DocumentPromotionResult::Branch`, `BranchPromotionOutcome`), `lifecycle_record.rs` (the second slot's readers), `closure/record.rs` and `closure/checkpoint.rs` (the shared link helper), `bind/lifecycle_verb.rs` (`Called::Branch`), `bind/lifecycle.rs`, the shared fake publisher (a test-support module), `promotion/run_tests.rs`; E-5 | 2 |
| 4 | `src/repository/promotion/lifecycle.rs` and `run.rs` (Part G item 6's hook), `promotion/record.rs` (the fence's query), `src/repository/context_retirement/source.rs`, `authority.rs` and `run.rs` (`capture_target`, `claim_initial`, `claim_scoped`, the absent-context branch; the fence, Part G item 14), `promotion/run_tests.rs`, `context_retirement/tests.rs` or `mutation_tests.rs` | 3 |
| 5 | `src/repository/mutation/dto.rs`, `mutation/way_out.rs`, `mutation/replay.rs`, `bind/resume.rs`, `schemas/v1/pull_request_promotion_preview.schema.json`, `bind/lifecycle.rs` and `bind/lifecycle_verb.rs` (the answers' data), `src/bin/manyhands-cli/human_lifecycle.rs`, `human_admin.rs`, `mutate.rs`, `collaboration.rs` and `lifecycle_tests.rs`, the desktop's arms, `tests/cli/lifecycle.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/mutation_contract.rs`, `tests/support/cli.rs`, `tests/support/mutation_golden.rs`, `tests/cli/inventory.rs`, `tests/support/mutation.rs`, `tests/support/repair.rs`, `tests/read_contract.rs`, `replay_tests.rs`, the goldens; E-2 and E-8 (preview parts), E-6, E-7, E-9 (`primary_holds`), E-11 | 4 |
| 6 | `src/repository/read/items.rs`, `read/dto.rs`, `schemas/v1/item.schema.json`, `src/repository/promotion/observe.rs` (`document_link_outstanding`), `src/repository/mutation/bind/sync.rs`, `tests/read_items.rs`, `promotion/observe_tests.rs`, `bind/sync_tests.rs`, `schemas/v1/pull_request_link.schema.json` (its description); E-1, E-14 | 5 |
| 7 | `src/repository/promotion/run_tests.rs` (the lifecycle table) | 6 |
| 8 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/support/mutation_golden.rs`, `schemas/v1/pull_request_promotion_mutation.schema.json`, `tests/mutation_contract.rs`, `tests/cli/inventory.rs`, the `mutation_v1` golden; E-2 and E-8 (mutation parts), E-9 (`ok`), E-13 | 7 |
| 9 | `docs/RFC/cli-contract.md`, `docs/RFC/pull-request-mode.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`, `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`; E-10 | 8 |
| 10 | — | 9 |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-execution.md`.

- [ ] **Step 1:** Confirm that vh-gy8gz is still closed and merged on `origin/main` (integrated at `6bc4b69c`). In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: Cycle 03's names (U-1).** They were re-checked at main `8e5d9832` for revision 6; find them again at the new HEAD, each name of design Baseline Facts 6 to 10, 13, 14 and 16: `RecordPath`, `BranchStep`, `BranchEnd`, `promote_branch`, `ends_context_child`, `give_link`, `branch_link`, `abandon_branch_record`, `Called::Branch`, `branch_code`, `ClosureOutcome`, `pull_request_links`, `PullRequestLinkDataDto`, `pull_request_link.schema.json` and the golden `item_sync_pull_request_link`, `item_sync_gives_the_link_of_a_closing_ticket`, the withdraw prepare's `already_applied` shape, `PullRequestIo` and the test-local `Forge`, and the SSH case `a_pull_request_close_publishes_only_the_ticket_branch`. Record each as found, renamed (with the new name) or changed in shape. A change of shape stops for the owner.
- [ ] **Step 3:** Re-check every other name, path and line the design cites (Baseline Facts 1 to 25, Parts A to G) at the new HEAD, in a ledger table with its current file. Confirm the test module paths: `repository::promotion::record` tests, `repository::promotion::observe` tests, `repository::promotion::run_tests` and its new `pull_request` module, `repository::mutation::bind::sync` tests, the CLI's human-output test module, `results::tests`, and the targets `read_items`, `read_contract`, `mutation_contract`, `cli`, `mutation_remote`. Name the tables `tests/cli/recovery_tables.rs` and `tests/cli/collaboration_exits.rs` parse (E-10).
- [ ] **Step 4: Premise sweep.** Grep for the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-15 onwards. An edit that reverses an intent is listed for the owner before its task.
- [ ] **Step 5:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 6:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Record's Mode (Design Part A)

**Files:** `src/repository/promotion/record.rs` (`SCHEMA_V2` frozen and used by the `promotion_refusal_blockers_v2` step; the rebuild under `wave_06_promotion_modes_v1` with an explicit column list; `integration_mode`, `answer_code`, `consented_context_oid`, `situation`, `primary_branch` and `endpoint_digest`; the CHECKs per mode; `PromotionStep::permits` and `validate` per mode; `PromotionAnswer`), `promotion/lifecycle.rs` (`impl LifecycleRecord for PromotionRecord`: `path()` and `answered()`; the callers of `begin_record`). Tests: `src/repository/promotion/record_tests.rs`, `tests/recovery_foundation_gate.rs`. Named edits: E-3, E-4.

**Interfaces:** Produces `PromotionAnswer { Promoted, AlreadyPromoted, PrimaryHolds, RemoteBranchDeleted, HistoryUnresolved, ContextGone }` (stored names `promoted`, `already_promoted`, `primary_holds`, `remote_branch_deleted`, `history_unresolved`, `context_gone`), and `PromotionRecord`'s `integration_mode: Integration` (Cycle 01's), `answer_code: Option<PromotionAnswer>`, and its immutable `consented_context_oid`, `situation`, `primary_branch` and `endpoint_digest`.

- [ ] **Step 0: Design checkpoint.** Write in the ledger, against the RR and F tables of design Part C, why each of Part A's new columns is needed, or the outcome that shows one is not; the rebuild is made once, so its columns are settled here, except that Task 4's checkpoint may amend this migration on the branch if a row needs a column (the migration never runs on a real registry before the merge). The orchestrator approves it before code.

- [ ] **Step 1: Write the failing tests** in `record_tests.rs`:
  - `promotion_modes_migration_rebuilds_a_v2_table_once`: a registry holding a v2 `document_promotions` table with rows of several steps migrates once, keeps every row as `direct`, records the marker, and a second migration changes nothing; a fresh registry is created with the new DDL and both markers.
  - `pull_request_promotion_steps_and_checks`: a table over (mode, step, columns) asserting `permits`, `validate` and the SQL CHECKs: a `pull_request` row holds its four begin-time columns from `confirmed` and `context_oid` from `context_synchronized`, moves `confirmed` → `context_synchronized` → `complete` (with a `published` or `already_current` category, an answer, and no integration column), `confirmed` → `abandoned` with the refusal `reconfirmation_required` and an answer, and `confirmed` → `abandoned` as a refused record (`blocked`, `external_change` or `reconfirmation_required`, no answer, in the confirm's mode); it refuses every integration and cleanup step and every integration column; `answer_code` is required at a pull-request `complete`, allowed at `abandoned`, and refused elsewhere and on a `direct` row; `save_record` refuses a change of any of the four begin-time columns once set, and still sets `context_oid` once at `context_synchronized`.
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part A. Apply E-3 and E-4 (E-4 rewrites `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once` from `SCHEMA_V2` with an explicit column list).
- [ ] **Step 4: Run** the new tests and E-4's edited tests by exact name, and `devenv shell -- cargo test --all-features --locked --test recovery_foundation_gate failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly -- --exact`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(promotion): the record's mode and stored answer".

## Task 2: The Pull-Request Observation (Design Part B)

**Files:** `src/repository/promotion/observe.rs` (`primary_holds_document`; the pull-request observation, its situations, effects, blockers and digest under `manyhands pull request promotion v1`), `promotion/mod.rs` (`preview_document_promotion` reads the mode), `promotion/lifecycle.rs` (promotion's `type Preview` carries the mode and the pull-request facts; `runs_context_child` true and `require_identity` passing in pull-request mode), `bind/lifecycle_verb.rs` (`Promote::observe`'s dispatch, `Ready`, no `identity_required` in pull-request mode, the prepare's `already_applied` through Cycle 03's withdraw-prepare shape), `bind/lifecycle.rs` (the preview's effects, the digest). Tests: `src/repository/promotion/observe_tests.rs`. Named edits: E-12.

**Interfaces:** Produces `primary_holds_document` (the repository, the tip to judge, the tracking primary's OID or none, the primary branch's name, the publication remote's name, the item, and whether to read `R` as absent; it answers a boolean; the exact signature is written at Step 1), and a pull-request observation carrying the mode, the situation (`first` or `again`), `L`, `R` and the link.

- [ ] **Step 1: Design checkpoint.** Write in the ledger, against the source: `primary_holds_document`'s signature and how it reads `L`, `R`, the local and the tracking primary with no fetch; Part G items 1, 5, 10, 12 and 15 (the held shape's own digest): the observation type's shape (the primary OID optional or absent in pull-request mode) and each `DocumentPromotionPreflight` match it changes (E-12), how the mode reaches `begin_record`, and how a prepare's `already_applied` is answered through Cycle 03's withdraw-prepare shape before any record. The orchestrator approves it before code.
- [ ] **Step 2: Write the failing tests** in `observe_tests.rs`:
  - `primary_holds_document_by_the_content_proof` (P3, A6): a table over a new document and an edit of a merged document × before the merge, a merge commit, a squash, a fast-forward × the merge on the tracking primary, and the local primary equal to, behind, ahead of and beside the tracking primary × `R` absent, equal to `L`, behind `L`, ahead of `L`, beside `L`, and `R` after `remote_branch_deleted`; plus a shallow repository and two merge bases. Before the merge it is never held; after it, held exactly when the merge is on the tracking primary, or on a local primary that is the tracking primary or behind it, and `R` is absent, equal or behind; a merge only on a local primary ahead of the tracking primary is not held; a document present on the primary with older content is never held; every refusal reads not held.
  - `pull_request_promotion_observation` (P2, A7): a table over the situations in Part B's order, the effects (all three, always), each blocker kept, each dropped (a dirty root, a root on another branch, a primary ahead of its remote, no identity), `history_unknown` not blocking, `primary_missing` with no tracking primary, and `remote_context_deleted`; the dispatch per mode (a direct repository's observation is today's).
  - `pull_request_promotion_digest_binds_each_field`, beside `digest_binds_each_field_change_blob_and_cleanup_member`: each of the domain, item, path, branch, primary, remote, `L`, `R` when ahead of or beside `L`, the situation and the previewed link changes the digest; `R` absent, equal to `L` and behind `L` give the same digest; the tracking and local primary do not change it.
- [ ] **Step 3: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 4: Implement** Part B and Part G items 1, 5, 10, 12 and 15. Apply E-12.
- [ ] **Step 5: Run** the new tests, E-12's edited tests, and `digest_binds_each_field_change_blob_and_cleanup_member` (unchanged), each by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(promotion): the pull-request observation and the content-proof rule".

## Task 3: Confirm, The Answer Tables (Design Part C, C1 To C11)

**Files:** `src/repository/promotion/lifecycle.rs` (`branch_step`, `end_branch`, `type Outcome` as an enum), `promotion/run.rs` (no `RunEvent::Recording` for a promotion), `promotion/mod.rs` (`DocumentPromotionResult::Branch` and its index-pending twin, `BranchPromotionOutcome`), `promotion/lifecycle_record.rs` (the second slot's readers), `closure/record.rs` and `closure/checkpoint.rs` (`give_link` and `branch_link` take the kind), `bind/lifecycle_verb.rs` (`Called::Branch` over either lifecycle; the promotion answer's code), `bind/lifecycle.rs` (`reached_*`, the recorded effects, the link row read for the answer), the shared fake publisher (Part G item 13). Tests: a `pull_request` module of `src/repository/promotion/run_tests.rs`. Named edits: E-5.

**Interfaces:** Consumes Task 1's record and Task 2's observation and `primary_holds_document`. Produces `BranchPromotionOutcome { answer: PromotionAnswer, publication: Option<PromotionPublication> }` (the binding reads the link from the row), and the shared fake publisher. `branch_step` reads the child's category through `child_state` (`Complete(stored)`) and the situation from the record, and judges the first table on the child's authority OID with the tracking primary as it is when it answers (design R-12).

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Design checkpoint.** Write in the ledger, against the source, Part G items 2 to 4, 7, 9, 11 and 13, and settle U-2 (lift Cycle 03's `PullRequestIo`, or write a promotion twin). The orchestrator approves it before code.
- [ ] **Step 3: Write the failing tests** in the `pull_request` module, each asserting that the local primary did not move and that no synchronization targeted the primary:
  - `first_promote_publishes_and_gives_the_link` (C1 to C3): a table over a new document, an edit of a merged one, and a branch already published by `item sync`.
  - `promoting_again_synchronizes_and_gives_the_same_link` (C4 to C7, A4): nothing new, after a save, after the remote's primary moved, after the forge setting changed; `publication` and `given_at` as Part C says, and the row's link every time.
  - `a_held_document_gives_no_link` (C8 to C10): at prepare; a squash between prepare and confirm with the branch kept (`complete`, the child's merge published); a merge with the branch deleted (`abandoned`, judged on the consented tip with the tracking primary the child's fetch moved).
  - `a_declined_pull_request_answers_remote_branch_deleted` (C11): the record `abandoned` with the refusal `reconfirmation_required` and its answer; the link row deleted; after a republish, the next promote answers `ok`.
- [ ] **Step 4: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 5: Implement** Part C's two tables and Part G items 2 to 4, 7, 9, 11 and 13. Apply E-5.
- [ ] **Step 6: Run** the new tests by exact name, and these unchanged: `cache_loss_contained_context_cleans_without_another_merge_or_child`, `already_current_publication_is_stored_and_replayed_without_children_or_deletion`, `contained_cleanup_needs_no_identity_or_new_merge` (promotion), `d1_records_publishes_and_completes_without_touching_the_primary` and `d2_an_existing_closure_publishes_pending_work_and_is_already_closing` (`closure/run_pull_request_tests.rs`, which share the branch path and the link helper), and E-5's edited tests. Expected: PASS.
- [ ] **Step 7: Commit** "feat(promotion): promote in a pull-request repository publishes and gives the link".

## Task 4: Re-Runs, Stops, Crashes, Holds, Consent And The Fence (Design Part C: The RR And F Tables, C12 To C28; Part G Items 6 And 14)

This task is test-first against required outcomes: three review rounds found blockers in the prose mechanisms of decisions 13 and 14, so the design states them as the RR and F tables, and this task's checkpoint chooses the mechanism (design R-28).

**Files:** `src/repository/promotion/lifecycle.rs` and `run.rs` (Part G item 6: the defaulted hook that gives the RR table for a promotion branch record), `promotion/record.rs` (the fence's query, a twin of the closure's hold), `src/repository/context_retirement/source.rs` `capture_target`, `authority.rs` `claim_initial` and `claim_scoped`, `context_retirement/run.rs`'s absent-context branch to `retire_context_scoped` (Part G item 14), and whatever the checkpoint finds the rows need. Tests: the `pull_request` module of `src/repository/promotion/run_tests.rs`; `src/repository/context_retirement/tests.rs` or `mutation_tests.rs` (P13).

- [ ] **Step 1: Write the failing tests first**, one row per outcome, each asserting that the local primary did not move:
  - `re_run_outcomes` (RR1 to RR8 and their order; C20, C25, C27 to C29): a table with a row for each RR row, covering what the tests below do not: the child's fast-forward and merges before a stop, then the same request (continues); a completed child before `context_synchronized`, with its branch present and gone, a cancellation after it, a remote change and an unreadable configuration after it (finishes `complete`); an attempt that may have pushed (continued or replayed, never refused); a save, a commit with Git, a poll's fast-forward to another clone's publication, and a save between the check and the child's start (refused, nothing published); a change of remote name and of fetch or push endpoint before any push (refused); a remote change and a save while a conflict holds the child, then the conflict resolved (finishes, decision 15); the branch removed before the child (RR7, both outcomes); a finish with the context absent (no row).
  - `stops_of_the_context_child` (C12, C13), `crash_and_replay` (C14, C15), `holds_cancellation_and_listing` (C16, C17, C21), `consent_differences` (C18, C19, C23, through the library's revalidation).
  - `retirement_waits_for_a_stopped_promote` (C22): for the poll's retirement and for `context retire` (the desktop's retirement is the same library path), the fence holds, the same request answers `already_applied` with `primary_holds`, and the next poll retires the document.
  - `the_retirement_fence` (P13, A14, F1 to F4) in `context_retirement/tests.rs` or `mutation_tests.rs`: a table over `capture_target`, `claim_initial`, `claim_scoped` and the absent-context branch, each skipping or refusing a document with an unfinished pull-request promotion record and retiring it once the record finishes; a retirement claimed before the record began (F3); a document without one, and a ticket, retire as today.
- [ ] **Step 2: Run, expect FAIL**, each by exact name; a row that already passes is run red against a deliberate stub (Global Constraints). Record the runs.
- [ ] **Step 3: Design checkpoint.** Write in the ledger, against the source and the red tests, Part G items 6 and 14 and the mechanism of every RR and F row: how a call tells its child's own moves from a commit it did not make, how it knows an attempt may have pushed, which columns of Part A it reads (and any column it adds to Task 1's migration), how a retained child is left to its way out (decision 15), whether the absent-context branch needs its own fence, and which check gives F3 (U-3, U-4). A row that cannot be met, or a change to the closure's path beyond the defaulted hook, stops for the owner. The orchestrator approves it before code.
- [ ] **Step 4: Implement** Part G items 6 and 14 and what Step 3 chose.
- [ ] **Step 5: Run** the new tests by exact name, the direct tests Task 3 Step 6 ran (C24), and the retirement tests the fence's call sites already have (Task 0 names them). Expected: PASS.
- [ ] **Step 6: Commit** "feat(promotion): a stopped pull-request promote re-runs, ends or waits by its outcomes, and fences retirement".

## Task 5: The Data, The Preview, The Way Out And The CLI Contract (Design Part D)

**Files:** `src/repository/mutation/dto.rs` (`PullRequestPromotionMutationDto`, `PullRequestPromotionPreviewDto`, the `from_stored` arm, the lifecycle arm's guard, `document promote` in the synchronization-conflict and host-data arms), `mutation/way_out.rs` (a promotion twin of `branch_route` for decision 11's way out, and `document promote` in the route to `child_conflict`), `mutation/replay.rs` (`result_commit_held` for the pull-request promote, K-18), `bind/resume.rs` (its branch scope and `DocumentPromote` arm for a promotion branch record), `src/bin/manyhands-cli/mutate.rs` and `collaboration.rs` (the retry guard and `continued_a_lifecycle`), `schemas/v1/pull_request_promotion_mutation.schema.json`, `schemas/v1/pull_request_promotion_preview.schema.json`, `bind/lifecycle.rs` and `bind/lifecycle_verb.rs` (the answers carry the data, the preview its details), `src/bin/manyhands-cli/human_lifecycle.rs` and `human_admin.rs` (the human output) and `lifecycle_tests.rs`, the desktop's compile-level arms (`src/desktop/view/confirm.rs` and the data matches). Tests: `tests/cli/lifecycle.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/mutation_contract.rs`, `src/repository/mutation/replay_tests.rs`, `dto.rs`'s `from_stored` tests, the goldens. Named edits: E-2 and E-8 (the preview's part), E-6, E-7, E-9 (the `primary_holds` golden), E-11. The mutation schema `pull_request_promotion_mutation.schema.json`, its `DATA_ROOTS` entry and its inventory entry land in Task 8, with the first golden that carries the data.

- [ ] **Step 1: Write the failing tests:**
  - in `tests/cli/lifecycle.rs`, in a `format_version = 2` pull-request repository with a selected key, a tracking primary written by the fixture, and a publication remote on a closed port, beside `a_context_primary_already_holds_is_only_cleaned_up`: `a_pull_request_promote_previews_its_link` (golden `document_promote_pull_request_preview`, JSON and text); a human-output check that a pull-request prepare's `already_applied` says the primary holds the document; `a_pull_request_prepare_is_blocked` (golden `document_promote_pull_request_blocked`, JSON and text: a dirty context, and none of the dropped blockers appear); `a_pull_request_confirm_stops_at_its_child` (the transport stop answers as `item sync`'s, the same request is offered as the way out, and a replay of the stored envelope keeps its data);
  - in `tests/mutation_confirmation/conformance.rs`, beside `withdraw_with_no_closure_is_already_applied`: `pull_request_promote_with_the_primary_holding_it_is_already_applied` (golden `mutation_v1` `document_promote_pull_request_primary_holds`, no network);
  - in `tests/mutation_confirmation/conformance.rs`: `a_held_document_at_confirm_is_external_change` (Part G item 15, C19): a merge fetched between prepare and confirm; the confirm answers `external_change` with nothing recorded, as Cycle 03's withdraw answers `NothingToWithdraw` at confirm, and a new prepare answers `already_applied` (the runner's revalidation half is Task 4's `consent_differences`);
  - promote rows in `replay_tests.rs`'s table `a_result_s_commit_is_held_where_its_request_left_it` (about 50; K-18): a merge commit and a fast-forward replay the stored envelope after the branch is gone; a squash answers as Cycle 03's K-13 says;
  - a row in `dto.rs`'s `from_stored` tests: `operation resume` with `pull_request_promotion` data reads it back;
  - `pull_request_promotion_human_lines` (P9) in `src/bin/manyhands-cli/lifecycle_tests.rs`: `publication: <state>`, the link's line, and `the primary branch already holds this document`;
  - a `way_out.rs` test beside its existing ones: the new data's `remote_branch_deleted` gives `Republish` of its item, then `PrepareAgain`; its `recovery_required` with `history_unresolved` gives `item resolve-history`; a stopped promote's conflict data gives `child_conflict`; a lifecycle `remote_branch_deleted` and a close's keep today's way outs.
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part D. Write the goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read the diff. Apply E-2's and E-8's preview parts, E-6, E-7, E-9's `primary_holds` part and E-11.
- [ ] **Step 4: Run** the new tests, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `every_schema_is_reachable_from_a_published_root` (`--test mutation_contract`), `every_verb_s_data_has_an_output_schema_and_its_goldens_match_it` and `fixture_directory_is_exactly_the_registered_cases` (`--test cli`), `the_fixture_directories_are_exactly_the_registered_cases` (`--test mutation_contract`) and `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`), each by exact name. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(promotion): the pull-request promote's data, preview, way out and output".

## Task 6: The Document's Link In Reads And `item sync` (Design Parts E And F)

**Files:** `src/repository/promotion/observe.rs` (`document_link_outstanding`, the cheap twin), `src/repository/read/items.rs` (`fill_complete_read` for a document), `read/dto.rs` and `schemas/v1/item.schema.json` (the documentation of `pull_request`), `src/repository/mutation/bind/sync.rs` (`link_given`'s document arm, beside Cycle 03's ticket arm), `schemas/v1/pull_request_link.schema.json`'s description and `PullRequestLinkDataDto`'s documentation. Tests: `promotion/observe_tests.rs`, `tests/read_items.rs`, `src/repository/mutation/bind/sync_tests.rs`. Named edits: E-1, E-14.

- [ ] **Step 1: Write the failing tests:**
  - `the_cheap_check_agrees_with_the_proof` (P6) in `observe_tests.rs`: `document_link_outstanding` against `primary_holds_document` on every row of P3's table.
  - `document_show_gives_the_link_while_it_is_outstanding` (P6, A8) in `tests/read_items.rs`, beside `ticket_show_gives_the_link_and_the_publication`: a document read from its active context with a link row gives the link in `document show` and `show --path`; null once the primary holds the document; null without a row, and without an active context; null in `document list`.
  - `item_sync_gives_an_outstanding_document_link` (P7, A9) beside `item_sync_gives_the_link_of_a_closing_ticket`: published and already current with an outstanding link give the link; no row, the primary holding the document, or a direct repository give today's answer; no row is written and `given_at` is unchanged.
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Parts E and F. Apply E-1 and E-14.
- [ ] **Step 4: Run** the new tests, `ticket_show_gives_the_link_and_the_publication` (E-1) and `item_sync_gives_the_link_of_a_closing_ticket` (unchanged), each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(read): a document's outstanding link in show and item sync".

## Task 7: The Owner's Lifecycles (Design P5)

**Files:** `src/repository/promotion/run_tests.rs`, the `pull_request` module.

- [ ] **Step 1: Write the failing test** `pull_request_promote_lifecycles` (A1 to A3): a table over a new document and an edit of a merged document × a forge merge commit, a squash and a fast-forward. For each row, with a local bare repository driven by `git2` as the forge and the shared fake publisher: promote gives the link; a poll (`synthetic_fetch`, after the objects and tracking refs are fetched with plain `git2`) leaves the document unpromoted; promote again gives the same link and `document show`'s `pull_request` is the link; a save and promote again publish the save; the forge merges by the row's style and deletes the branch; the poll fast-forwards the primary and reports `Promoted`; promote's prepare answers `already_applied`; `document show`'s `pull_request` is null; `execute(Mutation::PollOnceRetiring)` retires the context and the link row is gone. The local primary moves only by the poll's fast-forward.
- [ ] **Step 2: Run, expect FAIL** against a stub `primary_holds_document` that answers presence (the edit rows fail before the merge). Record the red run.
- [ ] **Step 3: Run** it against the real rule. Expected: PASS.
- [ ] **Step 4: Commit** "test(promotion): the pull-request promote's lifecycles".

## Task 8: The SSH Steps And The Golden (Design P10)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (Cycle 03's `a_pull_request_close_publishes_only_the_ticket_branch` gains the document steps), `tests/support/mutation_golden.rs` (`CASES`), the golden under `tests/fixtures/mutation_v1/`, `schemas/v1/pull_request_promotion_mutation.schema.json`, `tests/mutation_contract.rs` (`DATA_ROOTS`), `tests/cli/inventory.rs`. Named edits: E-2 and E-8 (the mutation's parts), E-9 (the `ok` golden), E-13.

- [ ] **Step 1: Add the steps** to the case, after its own (decision 6): create and save a document; `document promote --prepare` and `--confirm` answer `ok` with the link; promote again answers `already_applied` with the same link; the server's primary is unchanged after each step, and those steps push only the document's branch; write the golden `document_promote_pull_request_ok`, with commit IDs replaced as existing goldens replace them. Apply E-9's `ok` part and E-13 (the case's "only the ticket branch" assertions scoped to its own steps).
- [ ] **Step 2: Run it red** against a stub promote that answers `ok` without publishing (the server check fails): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`. Record the red run.
- [ ] **Step 3: Run it** against the real code, by the same command. Expected: PASS. Then `the_fixture_directories_are_exactly_the_registered_cases` and `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root` (`--test mutation_contract`) and `every_verb_s_data_has_an_output_schema_and_its_goldens_match_it` (`--test cli`), each by exact name. Expected: PASS.
- [ ] **Step 4: Commit** "test(promotion): the pull-request promote over SSH and its golden".

## Task 9: The Contracts, RFCs, Audit And Skills (Design D-1 To D-5)

**Files:** `docs/RFC/cli-contract.md` (D-1), `docs/RFC/pull-request-mode.md` (D-2), `docs/RFC/repository-index-persistence-and-refresh.md` (D-3), `skills/` and `.agents/skills/` (D-4), `docs/research/wave-03-api-audit.md` (D-5). The RFCs are managed documents; as Cycles 01 to 03 did, they are edited on the ticket branch with Git and merge with the ticket, one line per paragraph. Tests: `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs`. Named edits: E-10.

- [ ] **Step 1:** Write D-1 to D-5, each amendment dated 2026-11-26 and naming vh-rphcs. D-1 extends the contract's sentence that a pull-request close's other recoverable answers have no row of their own to the pull-request promote, so no table row is added (E-10 expects none).
- [ ] **Step 2: Run**, each by exact name in `--test cli`: `skills_are_well_formed`, `skill_links_stay_inside_the_set_and_resolve`, `skill_commands_are_catalogued`, `skill_json_examples_match_their_input_schema` and `every_mutation_sits_in_one_autonomy_column` (module `skills`), the tests of `recovery_tables`, `collaboration_exits` and `inventory` that parse the contract (Task 0 named them). Expected: PASS.
- [ ] **Step 3: Commit** "docs: document promote in pull-request mode".

## Task 10: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved. Then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier** (about 80 minutes at Cycle 03's review-ready for the five targets below), derived from `git diff --name-only origin/main...HEAD` against AGENTS.md's table, plus `remote_polling` (design Testing); at planning it is `document_promotion`, `mutation_remote`, `ticket_closure`, `cli_remote` and `remote_polling`, and any target the diff adds joins it. Each runs once as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5: Final review.** A fresh reviewer (the most capable model) reviews the whole branch against the design; a fix wave follows if needed, and after it only the targets whose code or tests changed run again.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses (through `manyhands-documents`) and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH steps, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then, through `manyhands-documents` and with the owner's go-ahead for each save and promotion (D-6): tick Cycle 04's boxes in the Wave 06 document; set this Cycle's three documents' status to done; promote the three. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3 | 7 |
| A4 | 3 |
| A5 | 3, 4 |
| A6, A7 | 2 |
| A8, A9 | 6 |
| A10 | 3, 4, 7, 8 |
| A11 | 5, 8 |
| A12 | 9 |
| A13 | 10 |
| A14 | 4 |
| R-1, R-11, R-12, R-13, R-16, R-17, R-19, R-30 | 3 |
| R-2, R-3, R-4, R-5, R-6, R-7, R-18, R-24, R-29 | 2 |
| R-8, R-14, R-22, R-23, R-25 | 5 |
| R-9, R-10 | 6 |
| R-15 | 8 |
| R-20, R-21, R-26, R-28 | 4 |
| R-27 | 1 |
| Review Focus lines | 1, 2, 3, 4, 6, 7, 8 |

Checked: every design Part has a task (A: 1; B: 2; C: 3 and 4; D: 5; E and F: 6; G: 2, 3 and 4); the types named in Interfaces match across tasks (`PromotionAnswer`, `BranchPromotionOutcome`, `PullRequestPromotionMutationDto`, `PullRequestPromotionPreviewDto`, `primary_holds_document`, `document_link_outstanding`); Part G's items map to Tasks 2 (1, 5, 10, 12, 15), 3 (2 to 4, 7, 9, 11, 13), 4 (6, 14) and 5 (item 4's replay, resume, retry guard and `from_stored` parts); the RR and F tables map to Task 4; no placeholder steps; the plan adds no test target, so no CI list changes.
