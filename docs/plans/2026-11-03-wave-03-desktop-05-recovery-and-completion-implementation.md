---
title: "Wave 03 D5 Desktop Conflict Recovery, Promotion And Closure Implementation Plan"
date: 2026-11-03
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6HPQ3PV69Z8FA294YW507N3"
---

# Desktop Conflict Recovery, Promotion And Closure Implementation Plan

> **For agentic workers:** Draft (revision 3), 2026-11-03. Not approved; implementation
> is not authorized. Once approved, use `implementing-a-cycle` task by
> task, re-read the design, repeat the ticket preflight, and record every
> checkpoint on ticket `01M4CC0VN2K8DBHNMZ46NTX5JJ`. Steps use checkbox
> (`- [ ]`) syntax.

**Goal:** Close tickets and approve and merge documents from the desktop
with the whole branch's effects confirmed, resolve conflicts part by part,
repair and republish in the app, and recover from any partial result
without repeating a merge or a closure checkpoint.

**Architecture:** Part 0 adds the library part (L1 way out, L2 lifecycle
resume listing, L3 lifecycle progress). Part A builds the lifecycle dialog
over the existing confirmation boundary. Part B builds the conflict model
and sheet for remote and local conflicts. Part C adds republish and
lifecycle resume. Part D proves the journeys. All serial (decision 8).

**Tech stack:** Rust 2024; `gpui-kit` through `gpui_kit::*` only;
`zorite-editor` `=0.10.0` (locked; no new package); Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-05-recovery-and-completion.md) and
[design](2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md),
and `docs/design/05-sheets-and-dialogs.md` for sizes and copy. "Fact n",
"U-n", "R-n", "E-n", "P-n" and "K-n" refer to the design.

## Global Constraints

**Precondition.** C5 (`01M4CC0VMXTNXNH78C11DYXS8H`) is merged to main and
the documents are approved against it. Task 0 does not start before.

**Roles.** The **orchestrator** is the session executing this plan. It
dispatches one implementer per task and a fresh reviewer after it, one at
a time; writes the ledger and ticket comments; and pushes the ticket
branch. Implementers never dispatch subagents and never edit ticket,
comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M4CC0VN2K8DBHNMZ46NTX5JJ`
on `manyhands/ticket/01M4CC0VN2K8DBHNMZ46NTX5JJ`. No lane branches
(decision 8). Every Rust command is `devenv shell -- cargo …` with
`--locked`.

**Library boundary.** Only Tasks 1–3 change library files, and only those
the design's Part 0 lists (Task 3 also adds `stage_text` arms in
`src/desktop/model/operations.rs` so the tree stays green). No other task changes `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. A gap stops
the task for the owner.

**Main.** Merge `origin/main` at Task 0, at each Part checkpoint and
whenever main moves; never rebase.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding title, body, comment, side or result text
derives `Debug` or `Display`.

**Test isolation.** Own temporary repositories, data directory and Git
home per test; no network; no desktop test initializes the transport.
Remote outcomes in desktop tests are canned answers built from library
types (decision 9); each names the library test that produces the real one.

**Editor tests** follow D3's rules (cursor, focus and a drawn frame before
typing; window-level `simulate_input`/`simulate_keystrokes`;
`cx.executor().allow_parking()` when waiting on the worker).

**Test runs.**
- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`
  (written `… <filter>`). One filter per run; each run must report more
  than zero tests.
- Library (Tasks 1–3): `devenv shell -- cargo test --all-features --locked --test <target> <filter>`,
  `--lib <filter>` or `--bin manyhands-cli <filter>`.
- Targeted tests between tasks; the whole desktop bin suite at each Part
  checkpoint; the full suite in Task 0.
- Part 0 changes `src/repository/mutation/`, `promotion/`, `closure/` and
  CLI lifecycle output, so AGENTS.md's SSH tier applies to it:
  `mutation_remote`, `document_promotion`, `ticket_closure` and
  `cli_remote`, at Task 4 and again in Task 16. If U-4 needs a file under
  `src/repository/remote/` or `local_sync/`, the task stops for the owner
  (design K-11).

**Each task's last Verify lines.**
```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Dead code.** An item first called by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Called from Task N.`; Task N
removes it.

**Test-first.** Each behaviour's test is written before its code.

**Existing tests** change only as E-1 to E-7; anything else stops for the
owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment after each Part checkpoint with
`manyhands-cli comment add` against the main checkout, only while the
ticket worktree is clean; it records commits, tests with counts and
rulings. Before `ticket save`, commit any owned `ticket.md` edit first.

**Execution ledger.**
`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-execution.md`,
created in Task 0.

**File size.** New source files stay below about 500 lines.

## Review Focus

1. **No repeat.** A Retry and a Resume after a partial result add no
   checkpoint, merge or publication commit (Tasks 7, 15b).
2. **Exact confirmation.** A refused consent never runs; Prepare again
   shows the new summary (Task 6b).
3. **Save first.** A failed save prepares nothing (Task 6a).
4. **No automatic winner.** Save resolution stays disabled until every
   part is chosen; external sets have no Save (Tasks 11b, 12).
5. **L1 parity.** C5's CLI renderer tests and goldens unchanged (Task 1).
6. **L3 order.** A cleanup-only replay reports `cleaning_up` alone; a
   local-only run no `publishing` (Task 3).
7. **Resume after restart** resends the named request (Task 14).
8. **The lock** never strands a close: read-only after integration until
   tidied, across a restart; unlocked on every pre-acceptance answer
   (Task 9).
9. **Kept resolution sets** are per request ID, survive a restart and are
   deleted on finish (Task 11a).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge main with C5, baseline, U-1–U-4, sweep, ledger. |
| 1 | 0 | L1 way out; CLI renders it. |
| 2 | 0 | L2 resume for lifecycle parents. |
| 3 | 0 | L3 lifecycle progress (test-first, design checkpoint). |
| 4 | 0 | Part 0 checkpoint. |
| 5 | A | Intents; header live; test seams. |
| 6a | A | Dialog: save first, prepare, blocked, held. |
| 6b | A | Dialog: confirm summary, stale, read-only tabs. |
| 7 | A | Running, failed, done; outcomes; way out; credentials. |
| 8 | A | Made-ready on Done. |
| 9 | A | After a lifecycle; the lock; Part A checkpoint. |
| 10 | B | Conflict intents, reads and model. |
| 11a | B | Kept resolution sets store. |
| 11b | B | Resolve sheet and submission. |
| 12 | B | External guidance. |
| 13 | B | Entry points; local sync and Resume; Part B checkpoint. |
| 14 | C | Republish; lifecycle Resume and resend. |
| 15a | D | Keyboard journeys. |
| 15b | D | Interruption and commit counts. |
| 15c | D | Privacy and consent. |
| 16 | D | Checklist, gates, review-ready. |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6a → 6b → 7 → … → 16, one at a time.

---

## Task 0: Baseline, References And Ledger

- [ ] **Step 1: Merge and baseline (entry-gate item 7).**
```sh
git fetch origin
git merge origin/main          # never rebase; C5 must be in it
git merge-base --is-ancestor origin/main HEAD
git merge-base --is-ancestor f59aceb6 HEAD || echo "C5 tip moved: re-check every (C5) fact"
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- sh scripts/check-cli-no-gpui.sh
```
  Run the test line detached with a three-hour budget. A failure is
  resolved or accepted by the owner before Task 1.
- [ ] **Step 2: Re-check facts and amendments.**
  `git diff --stat e39bb004 HEAD -- src/repository src/results.rs src/desktop src/bin schemas docs/RFC docs/Waves`.
  Re-read each fact whose file changed, especially Facts 2–9 (C5), and
  correct the design first. Refresh the Wave line citations. Confirm
  Document Amendments 1–6 are applied.
- [ ] **Step 3: Settle U-1 to U-5** by reading the source; record each in
  the ledger. An answer that differs stops for the owner.
- [ ] **Step 4: Sweep.** `src/desktop` for `LATER`, `LATER_NOTE`,
  `header_later`, `later_action`, `CONFLICT_BANNER`, `BRANCH_DELETED`,
  `RemoteBranchDeleted`, `SYNCHRONIZATION_UNAVAILABLE`, `resume_offer`,
  `is_synchronization`, `stage_text`, `JobKind::`, `PreviewDetailsDto::`,
  `MutationDataDto::`, `PrepareDataDto::`; `tests/` and
  `src/bin/manyhands-cli/` for `"connecting"`, `next_action` and
  `ProgressStage` near lifecycle commands. Compare each hit with E-1–E-7;
  an unlisted breakage stops for the owner.
- [ ] **Step 5: Ledger, commit, ticket comment.**

---

# Part 0: Library

## Task 1: L1 Way Out

- [ ] **Tests first** (`src/repository/mutation/way_out_tests.rs`, lib
  unit tests): one per `LifecycleWayOut` variant from a constructed
  envelope; `a_held_prepare_names_the_holding_request`;
  `a_deleted_branch_cancels_then_sends_again`;
  `a_documents_integration_conflict_is_repair_outside`;
  `prepare_routing_takes_the_items_kind`;
  `mixed_blockers_keep_the_cli_order`.
- [ ] **Code:** `mutation/way_out.rs`, `mutation/mod.rs` export;
  `src/bin/manyhands-cli/human_lifecycle.rs` renders the returned values
  (design, L1).
- [ ] **Verify:** `--lib way_out`; `--bin manyhands-cli lifecycle` (C5's
  `lifecycle_tests.rs`, unchanged); `--test cli`; `--test cli_remote
  lifecycle`; no golden changes; the four lines.
- [ ] **Commit; review.**

## Task 2: L2 Resume For Lifecycle Parents

- [ ] **Tests first** (`tests/read_status.rs`):
  `an_unfinished_promotion_lists_resume`,
  `an_unfinished_closure_lists_resume`,
  `a_held_parent_still_lists_resume`,
  `a_refused_parent_lists_no_resume`.
- [ ] **Code:** `read/status.rs`, `mutation/bind/resume.rs` (share
  `lifecycle_standing`) (design, L2). E-1, E-2.
- [ ] **Verify:** `--test read_status`; `--test cli`; `--test cli_remote
  operation`; the four lines.
- [ ] **Commit; review.**

## Task 3: L3 Lifecycle Progress (test-first, design checkpoint)

- [ ] **Design checkpoint:** before code, the implementer writes the
  observer threading for `run_lifecycle` and U-4's mapping into the ledger;
  the orchestrator reviews it against the design's L3.
- [ ] **Tests first** (`tests/mutation_remote/d5_progress.rs`, a module of
  `tests/mutation_remote.rs`):
  `a_promotion_reports_its_stages_in_order`,
  `a_closure_reports_recording_before_integrating`,
  `publishing_follows_integrating`,
  `the_primary_childs_safe_points_are_not_forwarded`,
  `a_cleanup_only_replay_reports_accepted_observing_cleaning_up`,
  `a_local_only_run_never_reports_connecting_or_publishing`.
- [ ] **Code:** `mutation/progress.rs`, `mutation/command.rs` (and its
  stage table test), `schemas/v1/progress_event.schema.json`,
  `tests/mutation_remote.rs` (`mod d5_progress;`),
  `promotion/run.rs`, `closure/lifecycle.rs`, `mutation/bind/lifecycle.rs`;
  `stage_text` arms in `src/desktop/model/operations.rs`. E-3.
- [ ] **Verify:** `--test mutation_remote d5_progress`; `--test
  read_contract`; `--test document_promotion`; `--test ticket_closure`;
  `--test cli_remote lifecycle`; the four lines.
- [ ] **Commit; review.**

## Task 4: Part 0 Checkpoint

- [ ] Merge main. AGENTS.md's fast tier, and the SSH tier:
  `mutation_remote`, `document_promotion`, `ticket_closure`, `cli_remote`.
- [ ] Ledger; ticket comment; push the ticket branch.

---

# Part A: Close Ticket… And Approve And Merge…

## Task 5: Intents, Header, Test Seams

- [ ] **Tests first** (`services/requests_tests.rs`,
  `view/item/header_tests.rs`, `model/operations_tests.rs`):
  `document_promote_builds_its_mutation`, `ticket_close_builds_its_mutation`,
  `close_ticket_is_live_on_a_ticket`, `approve_and_merge_is_live_on_a_document`,
  `a_closed_ticket_offers_no_close`,
  `a_lifecycle_row_is_not_a_synchronization`.
- [ ] **Code:** `Intent`, `JobKind`, build arms, labels; `header.rs`,
  `model/item.rs`; `resume_offer` lifecycle arm; canned prepare and
  conflict-read seams, and a worker progress-sink seam that holds at a
  named stage, in `test_support/` (design, A1, Testing). E-4.
- [ ] **Verify:** `… header`; `… requests`; `… operations`; the four lines.
- [ ] **Commit; review.**

## Task 6a: Dialog — Save First, Prepare, Blocked

- [ ] **Tests first** (`view/lifecycle_tests.rs`):
  `unsaved_edits_are_saved_before_prepare`,
  `a_failed_save_prepares_nothing` (P1),
  `a_dirty_primary_blocks_at_prepare` (P19, real repository),
  `a_blocked_prepare_lists_blockers_and_way_out`,
  `a_held_item_offers_continue` (P12),
  `the_first_run_submits_with_its_confirmation`,
  `already_closed_runs_nothing` (P14),
  `the_header_names_item_repository_and_context` (A22).
- [ ] **Code:** `view/lifecycle.rs`, `lifecycle/prepare.rs` (design, A2).
- [ ] **Verify:** `… lifecycle`; the four lines.
- [ ] **Commit; review.**

## Task 6b: Dialog — Confirm, Stale, Read-Only

- [ ] **Tests first** (`view/lifecycle/confirm_tests.rs`):
  `the_summary_lists_every_changed_path_with_its_tag` (A19),
  `code_on_the_branch_adds_the_code_note`,
  `closure_steps_mark_closed_before_merge`,
  `local_only_reads_keep_on_this_computer`,
  `closure_shows_closed_by_from_the_preview`,
  `a_new_path_after_prepare_offers_prepare_again` (P2, A4),
  `expired_consent_offers_prepare_again` (P3),
  `status_text_closed_still_closes` (P13),
  `enter_confirms_esc_cancels_and_focus_returns` (A21).
- [ ] **Code:** `lifecycle/confirm.rs`, read-only hook in the item tab
  (design, A2).
- [ ] **Verify:** `… lifecycle`; the four lines.
- [ ] **Commit; review.**

## Task 7: Running, Failed, Done

- [ ] **Tests first** (`view/lifecycle/outcome_tests.rs`):
  `running_lights_each_step_from_its_stage`,
  `stop_at_a_safe_point_names_completed_steps` (P4, A22),
  `local_only_with_cleanup_complete_is_done`,
  `cleanup_refused_is_failed_with_retry` (P6, canned),
  `retry_sends_the_same_request_id` (P7),
  `a_stopped_run_with_no_record_offers_prepare` (P18),
  `a_dirty_primary_stop_at_execute_merges_nothing` (P19, canned),
  `each_way_out_offers_its_button`,
  `a_refused_key_offers_unlock_and_sharing_and_keys`,
  `a_resend_without_a_secret_takes_the_unlock_path`,
  `the_operation_entry_reopens_the_dialog`.
- [ ] **Code:** `lifecycle/outcome.rs`, operation area entry (design, A3).
- [ ] **Verify:** `… lifecycle`; `… operations`; the four lines.
- [ ] **Commit; review.**

## Task 8: Made Ready On Done

- [ ] **Tests first** (`view/lifecycle/made_ready_tests.rs`):
  `done_lists_made_ready_with_slug_and_title`,
  `a_ticket_without_a_slug_shows_its_short_id`,
  `index_pending_says_the_list_may_be_incomplete`,
  `not_computed_offers_open_ready_tickets`,
  `a_promotion_shows_no_made_ready`.
- [ ] **Code:** `lifecycle/made_ready.rs` (design, A3).
- [ ] **Verify:** `… made_ready`; the four lines.
- [ ] **Commit; review.**

## Task 9: After A Lifecycle; Part A Checkpoint

- [ ] **Tests first** (`view/lifecycle/after_tests.rs`):
  `tabs_reopen_from_primary_after_cleanup`,
  `the_closure_display_stays_apart_from_status` (characterization, A20),
  `comment_drafts_stay_on_the_drafts_page` (A10),
  `a_failed_close_keeps_its_context_and_drafts` (A10),
  `editing_a_promoted_document_starts_a_fresh_context`;
  (`view/lifecycle/lock_tests.rs`) `every_tab_of_the_item_is_read_only_from_saving` (P20),
  `the_lock_survives_a_restart_from_the_l2_row` (P20),
  `after_integration_a_refused_cleanup_stays_locked` (P23, real local-only with a failure point),
  `each_pre_acceptance_answer_unlocks` (P25).
- [ ] **Code:** design, A2 (lock) and A4.
- [ ] **Verify:** `… lifecycle`; whole desktop bin suite; the four lines.
- [ ] **Commit; review;** merge main; ledger; ticket comment; push.

---

# Part B: Resolve Conflict Sheet

## Task 10: Conflict Reads And Model

- [ ] **Tests first** (`services/requests_tests.rs`):
  `conflict_resolve_builds_its_mutation`,
  `local_context_sync_and_resolve_build_their_mutations`;
  (`model/conflict_tests.rs`):
  `a_field_changed_by_both_needs_a_choice`,
  `a_field_changed_by_one_side_is_taken`,
  `the_body_offers_keep_both`,
  `all_mine_submits_mines_bytes`,
  `a_chosen_merge_submits_a_normalized_file`,
  `a_noncanonical_path_makes_the_set_external`,
  `incompatible_closures_are_external`,
  `a_local_unsupported_reason_is_external`,
  `a_local_entry_is_classified_by_its_path` (U-2).
- [ ] **Code:** `Intent::ConflictResolve`, `LocalContextSync`,
  `LocalContextResolve` with `JobKind`s and build arms; `model/conflict.rs`;
  reads (design, B1–B3).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 11a: Kept Resolution Sets

- [ ] **Tests first** (`services/resolutions_tests.rs`):
  `a_set_is_written_before_submission`,
  `a_set_is_read_back_after_reopening_the_store` (P24),
  `a_set_is_deleted_when_its_request_finishes`,
  `sets_are_kept_per_request_id`,
  `a_set_has_no_debug_and_never_reaches_a_message`.
- [ ] **Code:** `services/resolutions.rs` beside the D2 store (U-5;
  design, B3).
- [ ] **Verify:** `… resolutions`; the four lines.
- [ ] **Commit; review.**

## Task 11b: Resolve Sheet And Submission

- [ ] **Tests first** (`view/conflict_tests.rs`):
  `save_is_disabled_until_every_part_is_chosen`,
  `ctrl_enter_saves_and_enter_is_a_newline`,
  `save_submits_every_path_with_the_observation`,
  `a_local_resolution_uses_the_inspections_observation`,
  `a_stale_observation_refreshes_the_comparison` (P11, A3, real local),
  `the_note_follows_the_source`,
  `original_request_required_retries_with_that_requests_kept_set`,
  `reopening_after_a_partial_local_resolution_offers_retry_not_new`,
  `a_lifecycle_child_resolution_sends_the_lifecycle_again` (P8, canned),
  `after_a_restart_a_child_resolution_resumes_the_parent_row`,
  `an_interrupted_remote_resolution_resumes` (P15, canned).
- [ ] **Code:** `view/conflict.rs`, `conflict/` (design, B3).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 12: External Guidance

- [ ] **Tests first** (`view/conflict/external_tests.rs`):
  `an_external_set_has_no_save`,
  `check_again_offers_resume_only_when_the_library_does`,
  `incompatible_closures_offer_no_winner` (P10, A12).
- [ ] **Code:** `view/conflict/external.rs` (design, B4).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 13: Entry Points, Local Sync; Part B Checkpoint

- [ ] **Tests first** (`view/conflict/entry_tests.rs`):
  `the_banner_offers_review_changes`,
  `a_pending_resolve_row_offers_review_changes_after_restart`,
  `repair_locally_syncs_and_opens_the_sheet_on_conflict` (P9, A13, real),
  `a_refused_local_sync_shows_its_answer_and_resume` (P16),
  `a_local_synchronization_offers_resume`.
- [ ] **Code:** banners, operation area, `resume_offer` (design, B5–B6).
  E-5, E-7.
- [ ] **Verify:** `… conflict`; `… operations`; whole desktop bin suite;
  the four lines.
- [ ] **Commit; review;** merge main; ledger; ticket comment; push.

---

# Part C: Republish And Resume

## Task 14: Republish; Lifecycle Resume

- [ ] **Tests first** (`view/operations/resume_tests.rs`,
  `view/operations/republish_tests.rs`):
  `republish_builds_its_mutation`,
  `a_deleted_branch_offers_republish`,
  `republish_confirms_then_shares` (canned),
  `a_stopped_republish_sends_the_same_request` (P17),
  `stop_and_republish_cancels_republishes_then_resends` (P21, A24, canned),
  `a_lifecycle_row_offers_resume_after_restart` (A15),
  `resume_resends_the_named_request_and_opens_running` (P5),
  `resume_is_not_offered_while_in_flight_and_busy_reads_still_running` (P22).
- [ ] **Code:** `Intent::ItemRepublish` with `JobKind` and build arm;
  design, A3 (deleted branch) and Part C. E-6.
- [ ] **Verify:** `… operations`; the four lines.
- [ ] **Commit; review.**

---

# Part D: Evidence

## Task 15a: Keyboard Journeys

- [ ] **Tests:** `view/keyboard_tests/lifecycle.rs` (A2, A6: close and
  promote by keyboard, local-only, real) and
  `view/keyboard_tests/conflict.rs` (A1: local real, remote canned).
- [ ] **Verify:** `… keyboard_tests`; the four lines.
- [ ] **Commit; review.**

## Task 15b: Interruption And Commit Counts

- [ ] **Tests** (`view/lifecycle/interruption_tests.rs`, real local-only
  repositories): a close held at `integrating` by the progress-sink seam,
  stopped at a safe point, then Retry (P4); a close broken by a library
  failure point after `replace_service`, the services reopened as after a
  relaunch, then Resume → `original_request_required` → resend (P5); a
  cleanup-only replay. Commits on primary and context are counted before
  and after each (A8, A9).
- [ ] **Verify:** `… interruption`; the four lines.
- [ ] **Commit; review.**

## Task 15c: Privacy And Consent

- [ ] **Tests** (`view/lifecycle/privacy_tests.rs`): sentinel title, body,
  comment and side text never in an operation entry or message (A16);
  navigation, Save, Done, close and quit start no lifecycle (A17).
- [ ] **Verify:** `… privacy`; whole desktop bin suite; the four lines.
- [ ] **Commit; review.**

## Task 16: Checklist, Gates, Review-Ready

- [ ] Write `docs/plans/2026-11-03-wave-03-desktop-05-on-screen-checklist.md`
  (design, Native Checklist).
- [ ] Merge main. Run:
```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
# AGENTS.md fast tier, exactly as main's AGENTS.md lists it at Task 16
devenv shell -- cargo test --all-features --locked --test mutation_remote
devenv shell -- cargo test --all-features --locked --test document_promotion
devenv shell -- cargo test --all-features --locked --test ticket_closure
devenv shell -- cargo test --all-features --locked --test cli_remote
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
git merge-tree --write-tree HEAD origin/main
```
- [ ] Whole-branch SPEC and QUALITY reviews; fill the Acceptance Status
  column; the owner runs the checklist.
- [ ] Ticket comment: review-ready; `ticket save`.
- [ ] After code-review approval, close the ticket by a canonical
  ticket-file edit (AGENTS.md; `ticket close` is not used on D5's own
  ticket) in the final push before merge. Native CI runs after merge, on
  main, only when the owner dispatches it.

---

## Plan Self-Review

| Acceptance / ruling / decision | Task |
| --- | --- |
| A1 | 13, 15a |
| A2, A6 | 6a–9, 15a |
| A3 | 11b |
| A4, A19 | 6b |
| A5 | 6a, 7 |
| A7 | 7 |
| A8, A9 | 7, 14, 15b |
| A24 | 14 |
| A10 | 9 |
| A11 | 8 |
| A12 | 12 |
| A13 | 13 |
| A14 | 14 |
| A15 | 2, 13, 14 |
| A16, A17 | 15c |
| A18 | 1–4 |
| A20 | 9 |
| A21 | 6b, 15a |
| A22 | 6a, 7 |
| A23 | 9 |
| P1, P12, P14, P19 | 6a, 7 |
| P2, P3, P13 | 6b |
| P20, P23, P25 | 9 |
| P21, P22 | 14 |
| P24 | 11a |
| P4, P6, P7, P18 | 7 |
| P5, P17 | 14 |
| P8, P11, P15 | 11b |
| P9, P16 | 13 |
| P10 | 12 |
| R-1–R-3, R-7, R-8 | 6a, 6b, 7 |
| R-4–R-6, R-9 | 10, 12 |
| R-10, R-11 | 11b |
| R-12 | 11a |
| Decisions 2, 3, 10, 11 | 1–3, 14 |
| Decision 6 | 8 |
| Decision 9 | 5, 7, 11, 14 |
| Decisions 12, 18 | 9 |
| Decision 19 | 11a, 11b |
| Decisions 1, 17 | 0 |
| Decision 4 | 10, 11b |
| Decision 5 | 14 |
| Decision 7 | 16 |
| Decision 8 | Order |
| Decision 13 | 6a |
| Decision 14 | 9 |
| Decisions 15, 16 | 11b, 13 |
