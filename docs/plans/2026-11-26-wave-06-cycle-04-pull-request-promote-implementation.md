---
id: 01M8BXXH54RS9M6NX5XW9ARX9A
title: 'Wave 06 Cycle 04 Implementation Plan: Document Promote In Pull-Request Mode'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 04 Implementation Plan: Document Promote In Pull-Request Mode

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a pull-request repository, promote a document by synchronizing and publishing its branch and giving the pull request link; promote again by synchronizing again and giving the same link; count a document as promoted only by the content proof, in promote, `document show` and `item sync`.

**Architecture:** The pull-request promote is a branch record of the existing `document_promotions` lifecycle (decision 1): the record gains its mode and stored answer (`promotion/record.rs`), prepare takes a pull-request observation with its own digest (`promotion/observe.rs`), and Cycle 03's branch path in the runner (`promotion/run.rs` `promote_branch`) runs the context child and lets promotion's `branch_step` answer and finish the record. One function decides whether the primary holds the document by the content proof, and promote, `document show` (`read/items.rs`) and `item sync` (`bind/sync.rs`) use it. New data and preview shapes carry the link.

**Tech Stack:** Rust, `git2`, `rusqlite`, `src/forge.rs`, the SSH test harness for steps added to one existing case.

**Spec:** [Cycle 04 design](2026-11-26-wave-06-cycle-04-pull-request-promote-design.md) (revision 1), under the [Cycle document](../Cycles/wave-06-cycle-04-pull-request-promote.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft**, revision 1, 2026-11-26. Not yet reviewed or approved.

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the named test edits, authorized implementation and chosen the execution method; and Cycle 03 (vh-gy8gz) has merged to main (decision 5). Task 0 does not start before both.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87684C9GM9BJP5QQ429KMAK` on `manyhands/ticket/01M87684C9GM9BJP5QQ429KMAK`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop compile checks run with `--features desktop-test-support --tests`, on their own.

**Registry.** The registry migrations are one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds no SSH case; Task 7 adds steps to Cycle 03's one case (decision 6).
- The fast tier and the SSH tier run once, in Task 9; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: the runner takes its clock from the request.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour (C20, the direct rows of P1 and P2) is exempt, and is recorded as such.

**Test edits.** The design's named edits E-1 to E-5, and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-6 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner.

**Intermediate commits.** Every commit compiles and passes the static gates and the registration tests: a function with no production caller yet is added in the task that calls it, an enum value's exhaustive matches are given arms in the task that adds it, and a schema and its registrations land with its first golden. Until Task 5, a pull-request promote may answer without data; direct mode passes throughout.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 3 and 9, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87684C9GM9BJP5QQ429KMAK`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 9, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-rphcs after Tasks 0, 3, 6 and 7, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M87684C9GM9BJP5QQ429KMAK --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- No pull-request promote moves the local primary or synchronizes the primary. Tasks 3 and 4 pin it with the publisher that refuses a primary target, and Task 7 with the server's primary.
- A document is promoted only by the content proof: an edit of a merged document is not promoted before its own merge, and a squash is promoted after it. Tasks 2 and 4 pin it.
- Promoting again never gives a second, different link and never leaves a save behind. Task 3 pins it.
- Every direct promotion behaves as before: every existing row reads `direct`. Task 1's migration test, Task 3's C20 and the `document_promotion` target pin it.
- A same-request retry after each crash point repeats no publication and answers as before. Task 3 pins it.
- `document show` stops offering the link once the primary holds the document. Task 6 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger | — |
| 1 | `src/repository/promotion/record.rs`, `record_tests.rs`, `tests/recovery_foundation_gate.rs`; E-3, E-4 | 0 |
| 2 | `src/repository/promotion/observe.rs`, `observe_tests.rs`, `promotion/mod.rs`, `bind/lifecycle_verb.rs` (`Promote::observe`'s dispatch), `bind/lifecycle.rs` (the preview's effects) | 1 |
| 3 | `src/repository/promotion/lifecycle.rs`, `run.rs`, `mod.rs`, `lifecycle_record.rs` (the second slot's readers), `closure/record.rs` and `closure/checkpoint.rs` (the shared link helper), `bind/lifecycle_verb.rs` (`Called::Branch`), `bind/lifecycle.rs`, `mutation/way_out.rs`; `promotion/run_tests.rs`; E-5 | 2 |
| 4 | `src/repository/promotion/run_tests.rs` (the lifecycle table) | 3 |
| 5 | `src/repository/mutation/dto.rs`, `schemas/v1/pull_request_promotion_mutation.schema.json`, `schemas/v1/pull_request_promotion_preview.schema.json`, `src/bin/manyhands-cli/human_lifecycle.rs`, `human_admin.rs`, the desktop's arms, `tests/cli/lifecycle.rs`, `tests/mutation_contract.rs`, the `cli_v1` goldens; E-2 | 3 |
| 6 | `src/repository/read/items.rs`, `read/dto.rs`, `schemas/v1/item.schema.json`, `src/repository/mutation/bind/sync.rs`, `tests/read_items.rs`, `bind/sync_tests.rs`; E-1 | 2, 5 |
| 7 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/support/mutation_golden.rs`, the `mutation_v1` goldens | 5, 6 |
| 8 | `docs/RFC/cli-contract.md`, `docs/RFC/pull-request-mode.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `skills/`, `.agents/skills/` | 7 |
| 9 | — | 8 |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-execution.md`.

- [ ] **Step 1:** Confirm that vh-gy8gz is closed and merged on `origin/main`. In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: Cycle 03's names (U-3).** Find, at the new HEAD, each name of design Baseline Facts 6 to 10 and 14: `RecordPath`, `BranchStep`, `BranchEnd`, `promote_branch`, `ends_context_child`, `give_link`, `branch_link`, `Called::Branch`, `branch_code`, `pull_request_links`, `PullRequestLinkDataDto` and `pull_request_link.schema.json`, `item_sync_gives_the_link_of_a_closing_ticket`, and the SSH case `a_pull_request_close_publishes_only_the_ticket_branch`. Record each as found, renamed (with the new name) or changed in shape. A change of shape stops for the owner.
- [ ] **Step 3:** Re-check every other name, path and line the design cites (Baseline Facts 1 to 20, Parts A to G) at the new HEAD, in a ledger table with its current file. Confirm the test module paths: `repository::promotion::record` tests, `repository::promotion::observe` tests, `repository::promotion::run_tests` and its new `pull_request` module, `repository::mutation::bind::sync` tests, `results::tests`, and the targets `read_items`, `read_contract`, `mutation_contract`, `cli`, `mutation_remote`.
- [ ] **Step 4: Premise sweep.** Grep for the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-6 onwards. An edit that reverses an intent is listed for the owner before its task.
- [ ] **Step 5:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 6:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Record's Mode (Design Part A)

**Files:** `src/repository/promotion/record.rs` (the rebuild under `wave_06_promotion_modes_v1` with an explicit column list, `SCHEMA_V2` frozen, `integration_mode` and `answer_code`, the CHECKs per mode, `PromotionStep::permits` and `validate` per mode, `PromotionAnswer`, `PromotionRecord::path()` and `answered()`, `begin_record` and `refuse_record` taking the mode). Tests: `src/repository/promotion/record_tests.rs`. Named edits: E-3, E-4.

**Interfaces:** Produces `PromotionAnswer { Promoted, AlreadyPromoted, PrimaryHolds, RemoteBranchDeleted, HistoryUnresolved, ExternalChange }` (stored names `promoted`, `already_promoted`, `primary_holds`, `remote_branch_deleted`, `history_unresolved`, `external_change`), and `PromotionRecord`'s `integration_mode: Integration` and `answer_code: Option<PromotionAnswer>`. Reuses Cycle 01's `Integration`.

- [ ] **Step 1: Write the failing tests** in `record_tests.rs`:
  - `promotion_modes_migration_rebuilds_a_v2_table_once`: a registry holding a v2 `document_promotions` table with rows of several steps migrates once, keeps every row as `direct`, records the marker, and a second migration changes nothing; a fresh registry is created with the new DDL and both markers.
  - `pull_request_promotion_steps_and_checks`: a table over (mode, step, columns) asserting `permits`, `validate` and the SQL CHECKs: a `pull_request` row moves `confirmed` → `context_synchronized` → `complete` and to `abandoned` from the first two, refuses every integration and cleanup step and every integration column, and has `answer_code` exactly at `complete` and `abandoned`; a `direct` row with `answer_code` is refused.
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part A. Apply E-3 and E-4.
- [ ] **Step 4: Run** the new tests and E-4's edited tests by exact name, and `devenv shell -- cargo test --all-features --locked --test recovery_foundation_gate failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly -- --exact`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(promotion): the record's mode and stored answer".

## Task 2: The Pull-Request Observation (Design Part B)

**Files:** `src/repository/promotion/observe.rs` (`primary_holds_document`; the pull-request observation, its situations `first` and `again`, its blockers, its digest under `manyhands pull request promotion v1`), `promotion/mod.rs` (`preview_document_promotion` reads the mode), `bind/lifecycle_verb.rs` (`Promote::observe`'s dispatch, no `identity_required` in pull-request mode), `bind/lifecycle.rs` (the preview's effects `context_synchronized`, `context_published`, `pull_request_link`). Tests: `src/repository/promotion/observe_tests.rs`.

**Interfaces:** Produces `primary_holds_document(repository, plan, kind, item) -> Result<bool, …>` (the signature is written at Step 1), and a pull-request observation carrying the situation, `L`, `R` and the link it would give.

- [ ] **Step 1: Design checkpoint.** Write in the ledger, against the source: `primary_holds_document`'s signature, how it reads `L`, `R`, the local and the tracking primary with no fetch, and how it maps `ProofRefusal` (every refusal is "not held"); how the pull-request observation reaches the binding without the Task 5 preview type (the existing preview envelope with its effects, and no details or data until Task 5); and that a prepare's `already_applied` is answered from the observation before any record. The orchestrator approves it before code.
- [ ] **Step 2: Write the failing tests** in `observe_tests.rs`:
  - `primary_holds_document_by_the_content_proof` (P3, A6): a table over a new document and an edit of a merged document × before the merge, a merge commit, a squash, a fast-forward × the merge on the local primary only, the tracking primary only, both × `R` absent, equal to `L`, behind `L`, ahead of `L`. Before the merge it is never held; after it, held exactly when `R` is absent, equal or behind; a document present on the primary with older content is never held.
  - `pull_request_promotion_observation` (P2, A7): a table over the situations in Part B's order, each blocker kept, each dropped (a dirty root, a root on another branch, a primary ahead of its remote, no identity), `history_unknown` not blocking, and `remote_context_deleted`; the dispatch per mode (a direct repository's observation is today's).
  - `pull_request_promotion_digest_binds_each_field`, beside `digest_binds_each_field_change_blob_and_cleanup_member`: each of the domain, item, path, branch, primary, remote, `L`, `R` and the situation changes the digest; the tracking and local primary do not.
- [ ] **Step 3: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 4: Implement** Part B.
- [ ] **Step 5: Run** the new tests and `digest_binds_each_field_change_blob_and_cleanup_member` (unchanged) by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(promotion): the pull-request observation and the content-proof rule".

## Task 3: Confirm As A Mode Of The Lifecycle (Design Parts C And G)

**Files:** `src/repository/promotion/lifecycle.rs` (`DocumentPromotionLifecycle`'s `branch_step` and `end_branch`), `promotion/run.rs` (`promote_branch`'s remote check for a promotion, U-2), `promotion/mod.rs` (`DocumentPromotionResult::Branch` and its index-pending twin, `BranchPromotionOutcome`), `promotion/lifecycle_record.rs` (the second slot's readers for a promotion branch record), `closure/record.rs` and `closure/checkpoint.rs` (`give_link` and `branch_link` take the kind, Part G item 9), `bind/lifecycle_verb.rs` (`Promote` builds `Called::Branch`; the promotion answer's code), `bind/lifecycle.rs` (`reached_*` and the recorded effects by path), `mutation/way_out.rs` (`remote_branch_deleted` → `item republish`). Tests: a `pull_request` module of `src/repository/promotion/run_tests.rs`, with `FakeIo` refusing a primary target. Named edits: E-5.

**Interfaces:** Consumes Task 1's record and Task 2's observation and `primary_holds_document`. Produces `BranchPromotionOutcome { answer: PromotionAnswer, link: Option<PullRequestLink>, publication: Option<PromotionPublication> }`.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Design checkpoint.** Write in the ledger, against the source, each of design Part G's nine items, and settle U-1 (does the context child's fetch move the tracking primary before a `remote_branch_deleted` stop?) and U-2 (how `promote_branch`'s remote check ends a pull-request promotion with `external_change`). If U-2 needs a change to the closure's path, stop for the owner. The orchestrator approves the checkpoint before code.
- [ ] **Step 3: Write the failing tests** in the `pull_request` module, each asserting that the local primary did not move and that no synchronization targeted the primary:
  - `first_promote_publishes_and_gives_the_link` (C1, C2): table over a new document and an edit of a merged one.
  - `promoting_again_synchronizes_and_gives_the_same_link` (C3 to C5, A4): nothing new, after a save, after the remote's primary moved; `publication` and `given_at` as Part C says.
  - `a_held_document_gives_no_link` (C6 to C8): at prepare; a squash between prepare and confirm with the branch kept; a merge with the branch deleted (C8 per U-1's answer).
  - `ends_and_stops_of_the_context_child` (C9 to C11, C18): a declined pull request's deleted branch; each stop of C10; a conflict resolved by the record's ID and continued; the remote changed after the record began.
  - `crash_and_replay` (C12, C13): the failure points after the child and after the record finished; the same request; a replay answering from `answer_code`.
  - `holds_cancellation_and_listing` (C14, C15, C19).
  - `consent_differences` (C16, C17): a mode switch either way; a save, a fetched publication, a link given between prepare and confirm.
- [ ] **Step 4: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 5: Implement** Parts C and G. Apply E-5.
- [ ] **Step 6: Run** the new tests by exact name, and these direct tests unchanged (C20): `cache_loss_contained_context_cleans_without_another_merge_or_child`, `already_current_publication_is_stored_and_replayed_without_children_or_deletion`, `contained_cleanup_needs_no_identity_or_new_merge`, and Cycle 03's `d1_records_publishes_and_completes_without_touching_the_primary` and `d2_an_existing_closure_publishes_pending_work_and_is_already_closing` (`closure/run_pull_request_tests.rs`, which share the branch path and the link helper). Expected: PASS.
- [ ] **Step 7: Commit** "feat(promotion): promote in a pull-request repository publishes and gives the link".

## Task 4: The Owner's Lifecycles (Design P5)

**Files:** `src/repository/promotion/run_tests.rs`, the `pull_request` module.

- [ ] **Step 1: Write the failing test** `pull_request_promote_lifecycles` (A1 to A3): a table over a new document and an edit of a merged document × a forge merge commit, a squash and a fast-forward. For each row, with a local bare repository driven by `git2` as the forge, `promotion_test_drive_local_bound` for the children's stages and `poll::synthetic_fetch` for the poll: promote gives the link; a poll leaves the document unpromoted; promote again gives the same link and `document show`'s `pull_request` is the link; a save and promote again publish the save; the forge merges by the row's style and deletes the branch; the poll fast-forwards the primary and reports `Promoted`; promote answers `already_applied` with `primary_holds`; `document show`'s `pull_request` is null; `poll once --retire-completed`'s retirement succeeds and the link row is gone. The local primary moves only by the poll's fast-forward.
- [ ] **Step 2: Run, expect FAIL** against a stub `primary_holds_document` that answers presence (the edit rows fail before the merge). Record the red run.
- [ ] **Step 3: Run** it against the real rule. Expected: PASS.
- [ ] **Step 4: Commit** "test(promotion): the pull-request promote's lifecycles".

## Task 5: The Data, The Preview And The CLI Contract (Design Part D)

**Files:** `src/repository/mutation/dto.rs` (`PullRequestPromotionMutationDto`, `PullRequestPromotionPreviewDto`, the `from_stored` arm, the lifecycle arm's guard), `schemas/v1/pull_request_promotion_mutation.schema.json`, `schemas/v1/pull_request_promotion_preview.schema.json`, `bind/lifecycle.rs` and `bind/lifecycle_verb.rs` (the answers and the prepare's `already_applied` carry the data), `src/bin/manyhands-cli/human_lifecycle.rs` and `human_admin.rs` (the human output), the desktop's compile-level arms (`src/desktop/view/confirm.rs` and the data matches). Tests: `tests/cli/lifecycle.rs`, `tests/mutation_contract.rs`, goldens in `tests/fixtures/cli_v1/`. Named edits: E-2.

- [ ] **Step 1: Write the failing tests** in `tests/cli/lifecycle.rs`, in a pull-request repository whose publication remote is on an unreachable URL, beside `a_context_primary_already_holds_is_only_cleaned_up`:
  - `a_pull_request_promote_previews_its_link` (golden `document_promote_pull_request_preview`, JSON and text);
  - `a_pull_request_prepare_says_the_primary_holds_the_document` (golden `document_promote_pull_request_primary_holds`, JSON and text);
  - `a_pull_request_prepare_is_blocked` (a dirty context; the dropped blockers do not appear);
  - `a_pull_request_confirm_stops_at_its_child` (the transport stop answers as `item sync`'s, and the same request is offered as the way out).
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part D. Write the goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read the diff. Apply E-2.
- [ ] **Step 4: Run** the new tests, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `every_schema_is_reachable_from_a_published_root` (`--test mutation_contract`) and `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`), each by exact name. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(promotion): the pull-request promote's data, preview and output".

## Task 6: The Document's Link In Reads And `item sync` (Design Parts E And F)

**Files:** `src/repository/read/items.rs` (`fill_complete_read` for a document), `read/dto.rs` and `schemas/v1/item.schema.json` (the documentation of `pull_request`), `src/repository/mutation/bind/sync.rs` (the link for a document, beside Cycle 03's for a closing ticket). Tests: `tests/read_items.rs`, `src/repository/mutation/bind/sync_tests.rs`. Named edits: E-1.

- [ ] **Step 1: Write the failing tests:**
  - `document_show_gives_the_link_while_it_is_outstanding` (P6, A8) in `tests/read_items.rs`, beside `ticket_show_gives_the_link_and_the_publication`: a document with a link row and an active context gives the link in `document show` and `show --path`; null once the primary holds the document; null without a row, and without a context; null in `document list`.
  - `item_sync_gives_the_link_of_a_promoted_document` (P7, A9) beside `item_sync_gives_the_link_of_a_closing_ticket`: published and already current with an outstanding link give the link; no row, or the primary holding the document, give today's answer; no row is written.
- [ ] **Step 2: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 3: Implement** Parts E and F. Apply E-1.
- [ ] **Step 4: Run** the new tests, E-1's test, and `item_sync_gives_the_link_of_a_closing_ticket` (unchanged), each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(read): a document's outstanding link in show and item sync".

## Task 7: The SSH Steps And The Goldens (Design P9)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (Cycle 03's `a_pull_request_close_publishes_only_the_ticket_branch` gains the document steps), `tests/support/mutation_golden.rs` (`CASES`), the goldens under `tests/fixtures/mutation_v1/`.

- [ ] **Step 1: Add the steps** to the case, after its own (decision 6): create and save a document; `document promote --prepare` and `--confirm` answer `ok` with the link; promote again answers `already_applied` with the same link; `item sync` of the document answers with the link; the server's primary is unchanged after each step; write the goldens `document_promote_pull_request_ok`, `document_promote_pull_request_already_applied` and `item_sync_document_pull_request_link`, with commit IDs replaced as existing goldens replace them.
- [ ] **Step 2: Run it red** against a stub promote that answers `ok` without publishing (the server check fails): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`. Record the red run.
- [ ] **Step 3: Run it** against the real code, by the same command. Expected: PASS. Then `every_bound_command_has_a_golden` (`--test mutation_contract`) by exact name. Expected: PASS.
- [ ] **Step 4: Commit** "test(promotion): the pull-request promote over SSH and its goldens".

## Task 8: The Contracts, RFCs And Skills (Design D-1 To D-4)

**Files:** `docs/RFC/cli-contract.md` (D-1), `docs/RFC/pull-request-mode.md` (D-2), `docs/RFC/repository-index-persistence-and-refresh.md` (D-3), `skills/` and `.agents/skills/` (D-4). These are files on the ticket branch, edited with Git, not managed documents.

- [ ] **Step 1:** Write D-1 to D-4, each amendment dated 2026-11-26 and naming vh-rphcs.
- [ ] **Step 2: Run** `skills_are_well_formed`, `skill_links_stay_inside_the_set_and_resolve`, `skill_commands_are_catalogued`, `skill_json_examples_match_their_input_schema` and `every_mutation_sits_in_one_autonomy_column` (`--test cli`, module `skills`), each by exact name. Expected: PASS.
- [ ] **Step 3: Commit** "docs: document promote in pull-request mode".

## Task 9: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved. Then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier**: `document_promotion`, `mutation_remote`, `ticket_closure` and `cli_remote` (design Testing), each once as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times.
- [ ] **Step 5: Final review.** A fresh reviewer (the most capable model) reviews the whole branch against the design; a fix wave follows if needed, and after it only the targets whose code or tests changed run again.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses (through `manyhands-documents`) and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH steps, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then tick Cycle 04's boxes in the Wave 06 document through `manyhands-documents` (D-5), with the owner's go-ahead for its save and promotion. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3 | 4 |
| A4, A5 | 3 |
| A6, A7 | 2 |
| A8, A9 | 6 |
| A10 | 3, 4, 7 |
| A11 | 5, 7 |
| A12 | 8 |
| A13 | 9 |
| R-1, R-12, R-13, R-16, R-17 | 3 |
| R-2, R-3, R-4, R-5, R-6, R-7 | 2 |
| R-8, R-14 | 5 |
| R-9 | 6 |
| R-10 | 6 |
| R-11 | 3 |
| R-15 | 7 |
| Review Focus lines | 1, 2, 3, 4, 6, 7 |

Checked: every design Part has a task (A: 1; B: 2; C: 3; D: 5; E and F: 6; G: 3); the types named in Interfaces match across tasks (`PromotionAnswer`, `BranchPromotionOutcome`, `PullRequestPromotionMutationDto`, `PullRequestPromotionPreviewDto`, `primary_holds_document`); no placeholder steps; the plan adds no test target, so no CI list changes.
