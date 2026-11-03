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

> **For agentic workers:** Draft, 2026-11-03. Not approved; implementation
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
the design's Part 0 lists. No other task changes `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. A gap stops
the task for the owner.

**Main.** Merge `origin/main` at Task 0, at each Part checkpoint and
whenever main moves; never rebase.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding title, body, comment, side or result text
derives `Debug` or `Display`.

**Test isolation.** Own temporary repositories, data directory and Git
home per test; no network beyond the SSH fixture; no desktop test
initializes the transport.

**Editor tests** follow D3's rules (cursor, focus and a drawn frame before
typing; window-level `simulate_input`/`simulate_keystrokes`;
`cx.executor().allow_parking()` when waiting on the worker).

**Test runs.**
- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`
  (written `… <filter>`). One filter per run; each run must report more
  than zero tests.
- Library (Tasks 1–3): `devenv shell -- cargo test --all-features --locked --test <target> <filter>`.
- Targeted tests between tasks; the whole desktop bin suite at each Part
  checkpoint; the full suite in Task 0.
- Part 0 changes `src/repository/mutation/`, `promotion/`, `closure/` and
  CLI lifecycle output, so AGENTS.md's SSH tier applies to it:
  `mutation_remote`, `document_promotion`, `ticket_closure` and
  `cli_remote`, at Task 4 and again in Task 16.

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

1. **No repeat.** A retry and a resume after a partial result add no
   checkpoint, merge or publication commit (Tasks 7, 15).
2. **Exact confirmation.** A change after prepare never runs under the
   old consent (Task 6).
3. **Save first.** A failed save prepares nothing (Task 6).
4. **No automatic winner.** Save resolution stays disabled until every
   part is chosen; external sets have no Save (Tasks 11, 12).
5. **L1 parity.** CLI goldens unchanged by the move (Task 1).
6. **L3 order.** A cleanup-only replay reports `cleaning_up` alone (Task 3).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge main with C5, baseline, U-1–U-4, sweep, ledger. |
| 1 | 0 | L1 way out; CLI renders it. |
| 2 | 0 | L2 resume for lifecycle parents. |
| 3 | 0 | L3 lifecycle progress (test-first, design checkpoint). |
| 4 | 0 | Part 0 checkpoint. |
| 5 | A | Intents; header actions live. |
| 6 | A | Lifecycle dialog: save first, prepare, confirm, stale. |
| 7 | A | Running, failed, done; outcomes; way out; operation area. |
| 8 | A | Made-ready on Done. |
| 9 | A | After a lifecycle; Part A checkpoint. |
| 10 | B | Conflict reads and model. |
| 11 | B | Resolve sheet and submission. |
| 12 | B | External guidance. |
| 13 | B | Entry points; local sync and Resume; Part B checkpoint. |
| 14 | C | Republish; lifecycle Resume. |
| 15 | D | Journeys, interruption, privacy, consent. |
| 16 | D | Checklist, gates, review-ready. |

Order: 0 → 1 → 2 → 3 → 4 → 5 → … → 16, one at a time.

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
  `git diff --stat e39bb004 HEAD -- src/repository src/results.rs src/desktop src/bin docs/RFC docs/Waves`.
  Re-read each fact whose file changed, especially Facts 2–9 (C5), and
  correct the design first. Confirm Document Amendments 1–4 are applied.
- [ ] **Step 3: Settle U-1 to U-4** by reading the source; record each in
  the ledger. An answer that differs stops for the owner.
- [ ] **Step 4: Sweep.** `src/desktop` for `LATER`, `LATER_NOTE`,
  `header_later`, `later_action`, `CONFLICT_BANNER`, `BRANCH_DELETED`,
  `SYNCHRONIZATION_UNAVAILABLE`, `resume_offer`, `JobKind::`,
  `PreviewDetailsDto::`, `MutationDataDto::`, `PrepareDataDto::`; `tests/`
  for `"connecting"`, `next_action`, `ProgressStage` near lifecycle
  commands. Compare each hit with E-1–E-7; an unlisted breakage stops for
  the owner.
- [ ] **Step 5: Ledger, commit, ticket comment.**

---

# Part 0: Library

## Task 1: L1 Way Out

- [ ] **Tests first** (`tests/mutation_remote/d5_way_out.rs`, added to
  the `mutation_remote` target): one test per `LifecycleWayOut` variant
  from a constructed envelope; `held_prepare_names_the_holding_request`;
  `mixed_blockers_give_inspect`.
- [ ] **Code:** `src/repository/mutation/way_out.rs`, `mutation/mod.rs`
  export; `src/bin/manyhands-cli/human_lifecycle.rs` renders the
  returned values (design, L1).
- [ ] **Verify:** `--test mutation_remote d5_way_out`; `--test cli`;
  `--test cli_remote lifecycle` with no golden change; the four lines.
- [ ] **Commit; review.**

## Task 2: L2 Resume For Lifecycle Parents

- [ ] **Tests first** (`tests/read_status.rs`):
  `unfinished_promotion_lists_resume`, `unfinished_closure_lists_resume`,
  `a_held_parent_lists_no_resume`.
- [ ] **Code:** `src/repository/read/status.rs` (design, L2). E-1, E-2.
- [ ] **Verify:** `--test read_status`; `--test cli`; the four lines.
- [ ] **Commit; review.**

## Task 3: L3 Lifecycle Progress (test-first, design checkpoint)

- [ ] **Design checkpoint:** before code, the implementer writes the sink
  threading for `promote_document_observed` and `close_ticket_observed`
  into the ledger; the orchestrator reviews it against the design's L3.
- [ ] **Tests first** (`tests/mutation_remote/d5_progress.rs`):
  `a_promotion_reports_its_stages_in_order`,
  `a_closure_reports_recording_before_integrating`,
  `a_cleanup_only_replay_reports_cleaning_up_alone`,
  `a_local_only_run_reports_no_pushing`.
- [ ] **Code:** `mutation/progress.rs`, `promotion/lifecycle.rs`,
  `promotion/mod.rs`, `closure/mod.rs`, `mutation/bind/lifecycle.rs`. E-3.
  CLI RFC stage list (Document Amendment 3) if not applied at approval.
- [ ] **Verify:** `--test mutation_remote d5_progress`; `--test
  document_promotion`; `--test ticket_closure`; the four lines.
- [ ] **Commit; review.**

## Task 4: Part 0 Checkpoint

- [ ] Merge main. AGENTS.md's fast tier, and the SSH tier:
  `mutation_remote`, `document_promotion`, `ticket_closure`, `cli_remote`.
- [ ] Ledger; ticket comment; push the ticket branch.

---

# Part A: Close Ticket… And Approve And Merge…

## Task 5: Intents And Header

- [ ] **Tests first** (`services/requests_tests.rs`, `view/item/header_tests.rs`):
  `document_promote_builds_its_mutation`, `ticket_close_builds_its_mutation`,
  `close_ticket_is_live_on_a_ticket`, `approve_and_merge_is_live_on_a_document`,
  `a_closed_ticket_offers_no_close`.
- [ ] **Code:** `Intent`, `JobKind`, build arms, labels; `header.rs`,
  `model/item.rs` (design, A1). E-4. Exhaustive matches take C5's new
  variants.
- [ ] **Verify:** `… header`; `… requests`; the four lines.
- [ ] **Commit; review.**

## Task 6: Lifecycle Dialog, Confirm

- [ ] **Tests first** (`view/lifecycle_tests.rs`):
  `unsaved_edits_are_saved_before_prepare`, `a_failed_save_prepares_nothing` (P1),
  `the_summary_lists_every_changed_path_with_its_tag`,
  `code_on_the_branch_adds_the_code_note`,
  `local_only_reads_keep_on_this_computer`,
  `closure_shows_closed_by_from_the_preview`,
  `a_change_after_prepare_offers_prepare_again` (P2),
  `expired_consent_offers_prepare_again` (P3),
  `a_blocked_prepare_lists_blockers_and_way_out` (A5),
  `a_held_item_names_its_request` (P12),
  `already_closed_runs_nothing` (P14),
  `status_text_closed_still_closes` (P13),
  `enter_confirms_and_esc_cancels`.
- [ ] **Code:** `view/lifecycle.rs` and `lifecycle/` (design, A2).
- [ ] **Verify:** `… lifecycle`; the four lines.
- [ ] **Commit; review.**

## Task 7: Running, Failed, Done

- [ ] **Tests first** (`view/lifecycle/outcome_tests.rs`):
  `running_shows_stages_as_they_arrive`, `stop_at_a_safe_point_stops` (P4),
  `merge_publication_and_cleanup_are_reported_apart`,
  `cleanup_refused_keeps_failed_with_retry` (P6),
  `retry_sends_the_same_request_id` (P7),
  `each_way_out_offers_its_button`,
  `the_operation_entry_reopens_the_dialog`.
- [ ] **Code:** `view/lifecycle/outcome.rs`, operation area entry (design,
  A3).
- [ ] **Verify:** `… lifecycle`; `… operations`; the four lines.
- [ ] **Commit; review.**

## Task 8: Made Ready On Done

- [ ] **Tests first:** `done_lists_made_ready_with_slug_and_title`,
  `index_pending_says_the_list_may_be_incomplete`,
  `not_computed_offers_open_ready_tickets`,
  `a_promotion_shows_no_made_ready`.
- [ ] **Code:** `view/lifecycle/made_ready.rs` (design, A3).
- [ ] **Verify:** `… made_ready`; the four lines.
- [ ] **Commit; review.**

## Task 9: After A Lifecycle; Part A Checkpoint

- [ ] **Tests first:** `tabs_reopen_from_primary_after_cleanup`,
  `a_closed_ticket_shows_closed_by_apart_from_status`,
  `comment_drafts_stay_on_the_drafts_page` (A10),
  `a_failed_close_keeps_its_context_and_drafts` (A10),
  `editing_a_promoted_document_starts_a_fresh_context`.
- [ ] **Code:** design, A4.
- [ ] **Verify:** `… lifecycle`; whole desktop bin suite; the four lines.
- [ ] **Commit; review;** merge main; ledger; ticket comment; push.

---

# Part B: Resolve Conflict Sheet

## Task 10: Conflict Reads And Model

- [ ] **Tests first** (`model/conflict_tests.rs`):
  `a_field_changed_by_both_needs_a_choice`,
  `a_field_changed_by_one_side_is_taken`,
  `the_body_offers_keep_both`,
  `keep_mine_everywhere_submits_mine_unchanged` (U-4),
  `a_noncanonical_path_makes_the_set_external`,
  `incompatible_closures_are_external`,
  `a_local_entry_is_classified_by_its_path` (U-2).
- [ ] **Code:** `model/conflict.rs`; reads in `services/reads.rs` callers
  (design, B1–B2).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 11: Resolve Sheet And Submission

- [ ] **Tests first** (`view/conflict_tests.rs`):
  `save_is_disabled_until_every_part_is_chosen`,
  `save_submits_every_path_with_the_observation`,
  `a_stale_observation_refreshes_the_comparison` (P11, A3),
  `original_request_required_offers_retry`,
  `a_lifecycle_child_conflict_finishes_the_same_request` (P8).
- [ ] **Code:** `view/conflict.rs`, `conflict/` (design, B3).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 12: External Guidance

- [ ] **Tests first:** `an_external_set_has_no_save`,
  `check_again_offers_resume_only_when_the_library_does`,
  `incompatible_closures_offer_no_winner` (P10, A12).
- [ ] **Code:** `view/conflict/external.rs` (design, B4).
- [ ] **Verify:** `… conflict`; the four lines.
- [ ] **Commit; review.**

## Task 13: Entry Points, Local Sync; Part B Checkpoint

- [ ] **Tests first:** `the_banner_offers_review_changes`,
  `a_pending_resolve_row_offers_review_changes_after_restart`,
  `repair_locally_syncs_and_opens_the_sheet_on_conflict` (P9, A13),
  `a_local_synchronization_offers_resume`.
- [ ] **Code:** banners, operation area, `resume_offer` (design, B5–B6).
  E-5, E-7.
- [ ] **Verify:** `… conflict`; `… operations`; whole desktop bin suite;
  the four lines.
- [ ] **Commit; review;** merge main; ledger; ticket comment; push.

---

# Part C: Republish And Resume

## Task 14: Republish; Lifecycle Resume

- [ ] **Tests first:** `a_deleted_branch_offers_republish`,
  `republish_confirms_then_shares`,
  `a_lifecycle_row_offers_resume_after_restart` (P5, A15),
  `resume_opens_the_dialog_running`.
- [ ] **Code:** design, Part C. E-6.
- [ ] **Verify:** `… republish`; `… operations`; the four lines.
- [ ] **Commit; review.**

---

# Part D: Evidence

## Task 15: Journeys

- [ ] **Tests:** `view/keyboard_tests/lifecycle.rs` (A2: close and
  promote by keyboard, local-only included, A6) and
  `view/keyboard_tests/conflict.rs` (A1: remote and local); SSH-fixture
  tests for remote cleanup rejection (A7) and republish (A14); a stop and
  a second-process interruption, then Retry and Resume, counting commits
  on primary and context (A8, A9); sentinel privacy (A16); no lifecycle
  from navigation, Save, Done, close or quit (A17).
- [ ] **Verify:** each filter; whole desktop bin suite; the four lines.
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
- [ ] Ticket comment: review-ready; `ticket save`; close the ticket only
  after approval, in the final push before merge.

---

## Plan Self-Review

| Acceptance / ruling / decision | Task |
| --- | --- |
| A1 | 13, 15 |
| A2, A6 | 6–9, 15 |
| A3 | 11 |
| A4 | 6 |
| A5 | 6 |
| A7 | 7, 15 |
| A8, A9 | 7, 14, 15 |
| A10 | 9 |
| A11 | 8 |
| A12 | 12 |
| A13 | 13 |
| A14 | 14, 15 |
| A15 | 2, 13, 14 |
| A16, A17 | 15 |
| A18 | 1–4 |
| P1–P3, P12–P14 | 6 |
| P4, P6, P7 | 7 |
| P5 | 14 |
| P8, P11 | 11 |
| P9 | 13 |
| P10 | 12 |
| R-1–R-3, R-7 | 6, 7 |
| R-4–R-6 | 10, 12 |
| Decisions 2, 3, 9, 10 | 1–3 |
| Decision 6 | 8 |
| Decision 11 | 6 |
| Decision 12 | 9 |
| Decisions 13, 14 | 11, 13 |
