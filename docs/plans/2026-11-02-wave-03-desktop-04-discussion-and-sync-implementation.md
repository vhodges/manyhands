---
title: "Wave 03 D4 Desktop Discussion And Deliberate Synchronization Implementation Plan"
date: 2026-11-02
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6E696JQ4NAKG1RT7103FQJ3"
---

# Desktop Discussion And Deliberate Synchronization Implementation Plan

> **For agentic workers:** Draft, revision 5, not approved. Implementation
> starts only once the owner has approved the documents and authorized it.
> Then use `implementing-a-cycle` task by task, re-read the design, repeat
> the ticket preflight, and record every checkpoint on ticket
> `01M4CC0VN1KTQFSVR99378TDHV`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Comment, reply, post and share from the desktop, with truthful
progress and outcomes, safe-point stop, credential recovery and recoverable
comment drafts, consuming C4's bindings unchanged.

**Architecture:** Part 0 adds the seams: intents, safe-point stop, Give
up, declined keys, the `Submissions` registry, the outcome model, the
placeholders removed, `ItemPane` slots and keys. Lanes A (comment drafts),
B (share, progress, recovery) and C (composer widget) run in parallel on
lane branches. Lane D wires them into the discussion. Part E proves the
keyboard, privacy and native journeys.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
`zorite-editor` `=0.10.0` (locked; no new package); Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-04-discussion-and-sync.md) and
[design](2026-11-02-wave-03-desktop-04-discussion-and-sync-design.md), and
`docs/design/item_view/04-comments.md` for sizes and copy. "Fact n",
"U-n", "R-n", "E-n" and "K-n" refer to the design.

## Global Constraints

**Roles.** The **orchestrator** is the session executing this plan. It
dispatches a fresh implementer for each task and then a fresh reviewer
before the next task in that lane starts, merges branches, runs
checkpoints, writes the ledger and ticket comments, and pushes.
Implementers never dispatch subagents and never edit ticket, comment or
ledger files.

**Where.**
- Part 0, Lane D and Part E run in the ticket worktree,
  `.manyhands/worktrees/01M4CC0VN1KTQFSVR99378TDHV`, branch
  `manyhands/ticket/01M4CC0VN1KTQFSVR99378TDHV`.
- Lanes A, B and C each run in `<main checkout>/.worktrees/d4-lane-{a,b,c}`
  on `d4/lane-{a,b,c}` (decision 1). Lane branches are never pushed.
- Every Rust command is `devenv shell -- cargo …` with `--locked`.

**No library change.** No task changes `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. A gap stops
the task; the orchestrator files a library ticket with `file-ticket` after
asking the owner.

**File ownership.** A task changes only files in its owner's row of the
design's table. A need outside the row stops the task; the orchestrator
makes that change as its own reviewed commit on the ticket branch, and
running lanes merge the ticket branch at their next task boundary.

**Main.** The orchestrator merges `origin/main` into the ticket branch at
the Part 0 checkpoint, at each lane merge, before Task 12, and whenever
main moves; never rebase. A lane takes main by merging the ticket branch
into itself at a task boundary.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding comment text derives `Debug` or `Display`.

**Test isolation.** Own temporary repositories, data directory and Git
home per test; no network; no desktop test initializes the transport.

**Editor tests** follow D3's rules (cursor, focus and a drawn frame before
typing; window-level `simulate_input`/`simulate_keystrokes`; never
Ctrl-Cmd-Space; `cx.executor().allow_parking()` when waiting on the
worker).

**Test runs.**
- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`
  (written `… <filter>` below). One filter per run; each run must report
  more than zero tests.
- Targeted tests between tasks; the whole desktop bin suite (no filter) at
  each checkpoint and lane merge; the full suite in Tasks 0 and 16.

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

**Existing tests** change only as E-1 to E-11; anything else stops for the
owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. With no Git
identity configured, pass it per command:
`git -c "user.name=Vince Hodges" -c user.email=vince@imbas.ca commit …`.

**Ticket records.** The orchestrator adds a comment after each task with
`manyhands-cli comment add` against the main checkout (built from current
main), only while the ticket worktree is clean and no merge is in
progress, and records the commit, the tests with counts, and any ruling.
Lane task comments are added when the lane's task passes review, naming
the lane commit. Before `ticket save`, commit any owned `ticket.md` edit
first (AGENTS.md).

**Execution ledger.**
`docs/plans/2026-11-02-wave-03-desktop-04-discussion-and-sync-execution.md`,
created in Task 0 and committed by the orchestrator on the ticket branch,
with a section per lane.

**File size.** New desktop source files stay below about 500 lines.

## Review Focus

1. **One comment, once.** `publish_over_ssh`, post, Retry sharing twice:
   one comment file, one checkpoint commit (Task 13); quit, relaunch,
   Resume (native §4).
2. **Stop at a safe point** on an accepted share cancels the token (Task
   2) and the outcome says what completed (Task 9).
3. **Dirty body, post:** the body file in the context is unchanged
   (Task 13).
4. **Not now, then a second request:** no second prompt (Task 2); Unlock…
   prompts once (native §5).
5. **A restored draft** reopens in its thread with its comment ID (Tasks
   6, 12) and is retired, not reposted, if already saved (Task 13).
6. **A cancelled post** offers Share, never a same-ID retry (Tasks 3, 13).

## File Map And Dependency Order

| Task | Owner | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge, baseline, U-1–U-5, sweep, ledger. |
| 1 | Part 0 | Intents. |
| 2 | Part 0 | Safe-point stop, Give up, declined keys, `Submissions`. |
| 3 | Part 0 | Outcome model. |
| 4 | Part 0 | Placeholders, slots, keys; E-1–E-7. |
| 5 | Orchestrator | Part 0 checkpoint; lane worktrees. |
| 6–7 | Lane A | Comment drafts; Drafts page; E-8, E-9. |
| 8–10b | Lane B | Share, stages and stop, outcomes and routes, Resume; E-6 (second half), E-10. |
| 11a–11b | Lane C | Composer widget. |
| 12–14 | Lane D | Placement, posting, reload; E-11. |
| 15 | Part E | Keyboard, privacy, checklist. |
| 16 | Orchestrator | Gates, reviews, review-ready. |

Order: 0 → 1 → 2 → 3 → 4 → 5 → {6–7 ∥ 8–10b ∥ 11a–11b} → 12 → 13 → 14 →
15 → 16.

---

## Task 0: Baseline, References And Ledger

- [ ] **Step 1: Merge and baseline (entry-gate item 7).**
```sh
git fetch origin
git merge origin/main          # never rebase
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- sh scripts/check-cli-no-gpui.sh
```
  Run the test line detached with a two-hour budget. A failure is resolved
  or accepted by the owner before Task 1.
- [ ] **Step 2: Re-check facts and amendments.**
  `git diff --stat 00bef3da HEAD -- src/repository src/results.rs src/drafts src/desktop .github docs/RFC docs/Waves`.
  Re-read each fact whose file changed and correct the design first.
  Confirm Document Amendments 1–6 are applied.
- [ ] **Step 3: Settle U-1 to U-5** by reading the source; record each in
  the ledger. An answer that differs from the design's expectation stops
  for the owner.
- [ ] **Step 4: Sweep** `src/desktop` for `LATER`, `LATER_NOTE`,
  `header_later`, `later-action`, `later-note`, `NoteAt::BelowOnPage`,
  `COMMENT_LATER`, `COMMENT_ON`, `SYNCHRONIZATION_UNAVAILABLE`,
  `resume_offer`, `CancelResult::`, `JobKind::`, `RecoveryStep::Cancel`,
  `CANCEL`, `Finishing`, `stored_for`, `"discussion"`, `"Comment"`,
  `"Share"`. Compare each hit with E-1–E-11; an unlisted breakage stops for
  the owner.
- [ ] **Step 5: Ledger, commit, ticket comment.**

---

# Part 0: Seams (ticket worktree)

## Task 1: Intents

- [ ] **Tests first** (`services/requests_tests.rs`):
  `comment_add_builds_its_mutation_with_no_author`,
  `item_and_repo_sync_build_their_mutations`,
  `labels_are_fixed_text`,
  `syncs_comments_and_resume_stop_at_safe_points_and_saves_do_not`.
- [ ] **Code:** `CommentBody`; the three intents and `build` arms;
  `JobKind::Comment`, `JobKind::Synchronize` (`services/mod.rs`) and
  `DelayingBackend` support; `changes_items`; `stops_at_safe_points`
  (design, Intents).
- [ ] **Verify:** `… requests`, then the four lines.
- [ ] **Commit.**

## Task 2: Stop, Give Up, Declined Keys And Submissions

- [ ] **Tests first:**
  - `services/worker_tests.rs`: `cancelling_an_accepted_sync_signals_its_token`
    (repository with `publish_over_ssh`; hold the job after acceptance with
    `set_request_hook_for_testing`; assert `Signalled` and the token
    cancelled); `cancelling_an_accepted_local_job_is_still_too_late`;
    `a_later_stage_marks_a_retry_accepted` (R-14);
    `giving_up_an_accepted_record_cancels_then_sends_the_same_request`;
    `giving_up_a_finished_record_reports_already_finished`;
    `give_up_reports_whatever_the_resent_request_answers` (fact 14d;
    `hold_job_return` with a constructed answer);
    `give_up_and_retry_of_one_request_refuse_each_other`;
    `a_finished_attempt_carries_its_record_state` (`show_request`);
    `submit_with_request_id_uses_the_given_id` (R-16).
  - `services/prompts_tests.rs` (the declined set shared by the handle
    and the provider, `Arc<Mutex<HashSet<…>>>`):
    `a_declined_key_answers_cancelled_without_a_prompt`,
    `clearing_a_declined_key_through_the_handle_prompts_again`.
  - `services/submissions_tests.rs`: `a_second_submit_while_posting_is_refused`,
    `a_posts_request_id_is_its_comment_id` (R-16),
    `retry_keeps_the_request_id`, `restart_moves_to_a_new_comment_id`,
    and, as a `#[gpui_kit::test]` through the services harness
    (`start(&fixture, cx)`, as `services/drafts_tests.rs` does),
    `a_finished_event_with_no_tab_open_updates_the_submission` (P9).
- [ ] **Code:** `services/worker.rs` (R-1, R-14, `Work::GiveUp`),
  `services/prompts.rs` (R-13), `services/submissions.rs`, and in
  `services/mod.rs` its registration and its update from the event drain.
- [ ] **Verify:** `… worker`, `… prompts`, `… submissions`, then the four lines.
- [ ] **Commit.**

## Task 3: Outcome Model

- [ ] **Tests first** (`model/publication_tests.rs`): one test per row of
  both tables, plus `rows_match_in_precedence_order` (an envelope matching
  two rows takes the earlier), `a_stop_after_the_push_is_shared`,
  `internal_error_is_maybe_saved_only_for_an_adopted_first_post`,
  `request_mismatch_is_maybe_saved`,
  `an_owed_unshared_comment_offers_retry_and_a_final_one_offers_share`
  (rows 9a/9b), and `identity_required_routes_to_set_identity` (row 9c,
  `NeedsIdentity`). Build envelopes as
  `model/operations_tests.rs` does.
- [ ] **Code:** `model/publication.rs`; one `mod` line in `model/mod.rs`.
- [ ] **Verify:** `… publication`, then the four lines.
- [ ] **Commit.**

## Task 4: Placeholders, Slots And Keys

- [ ] **Step 1:** E-1 to E-7 and E-6's first half, as the design lists
  them.
- [ ] **Step 2:** `LATER`, `COMMENT`, `SHARE` (marked for Task 8),
  `header_later`, the discussion's disabled Comment and
  `NoteAt::BelowOnPage` (design, Removing the placeholders).
- [ ] **Step 3:** Stub `view/item/{share,composer,posting}.rs`; the three
  `ItemPane` fields and render calls.
- [ ] **Step 4:** `keys.rs`: the `CommentComposer` context and its four
  bindings, `ShareItem`, stub handlers.
- [ ] **Verify:** `… item`, `… keyboard`, then the four lines.
- [ ] **Commit.**

## Task 5: Part 0 Checkpoint And Lane Worktrees

- [ ] Whole desktop bin suite; the four lines.
- [ ] Fresh reviewer of Tasks 1–4 together against the design.
- [ ] `git fetch origin`; merge `origin/main`; re-check moved facts;
  `git merge-tree --write-tree HEAD origin/main`; push the ticket branch.
- [ ] Create the lanes from the ticket branch's HEAD:
```sh
main=/home/vhodges/work/src/manyhands
for lane in a b c; do
  git -C "$main" worktree add -b "d4/lane-$lane" "$main/.worktrees/d4-lane-$lane" manyhands/ticket/01M4CC0VN1KTQFSVR99378TDHV
done
```
- [ ] Ledger and ticket comment with the base and lane SHAs; notify the
  owner.

---

# Lane A: Comment Drafts (`d4-lane-a`)

## Task 6: Comment Draft API

- [ ] **Tests first** (`services/drafts/comments_tests.rs`):
  `begin_keeps_the_callers_comment_id`, `each_thread_keeps_its_own_draft`,
  `an_edit_flushes_after_the_idle_delay` (advance the test clock past
  `IDLE`), `a_dropped_registry_leaves_the_draft_to_adopt_with_its_id`,
  `comment_drafts_waits_for_the_first_listing`,
  `retiring_removes_the_draft`, `stored_for_still_ignores_comment_drafts`.
- [ ] **Code:** `services/drafts/comments.rs` (design, Lane A), one `mod`
  line in `services/drafts.rs`.
- [ ] **Verify:** `… drafts`, then the four lines.
- [ ] **Commit; review.**

## Task 7: The Drafts Page Opens Comment Drafts

- [ ] **Tests first:** E-8, E-9; `a_comment_rows_open_opens_its_item`.
- [ ] **Code:** `view/drafts/rows.rs`, `view/drafts/row_actions.rs`,
  `view/drafts.rs`, `model/drafts/rows.rs`; `COMMENT_LATER` removed.
- [ ] **Verify:** `… drafts`, then the four lines.
- [ ] **Commit; review. Lane A checkpoint** (design, Lane merge).

---

# Lane B: Share, Progress And Recovery (`d4-lane-b`)

## Task 8: Share Controls

- [ ] **Tests first** (`view/item/share_tests.rs`):
  `share_follows_edit_when_a_remote_exists`,
  `share_without_a_remote_reads_share_and_opens_remotes`,
  `a_second_press_while_sharing_queues_nothing`,
  `share_main_in_the_repository_detail_submits_repo_sync`; E-6, second
  half.
- [ ] **Code:** `view/item/share.rs`, `view/item/header.rs` (removes
  `SHARE`'s allowance), `view/sidebar.rs`, the Remotes route in
  `view/workspace/requests.rs`.
- [ ] **Verify:** `… share`, `… header`, then the four lines.
- [ ] **Commit; review.**

## Task 9: Stages And Stop

- [ ] **Tests first:** `each_stage_shows_its_text_for_share_and_post`
  (apply constructed `ProgressEvent`s through `Operations::apply`);
  `a_post_without_a_remote_never_says_sharing`;
  `an_accepted_share_offers_stop_and_stopping_signals_it`;
  `a_stopped_share_says_what_completed_and_offers_share`;
  `the_dialog_names_what_waits_and_says_not_now` (provider-raised prompt,
  `passphrase_tests.rs` pattern).
- [ ] **Code:** `view/item/pill.rs`, `view/operations.rs`,
  `model/operations.rs`, `view/passphrase.rs`.
- [ ] **Verify:** `… share`, `… operations`, `… passphrase`, then the four lines.
- [ ] **Commit; review.**

## Task 10a: Outcomes And Routes

- [ ] **Tests first:** one test per `SyncOutcome` row from constructed
  envelopes; `unlock_clears_the_declined_key_and_retries_the_same_request`;
  `no_selected_key_opens_sharing_and_keys`;
  `a_host_code_checks_the_host_with_its_authority`;
  `a_hostless_comment_stop_opens_the_roots_hosts_page` (K-3);
  `busy_may_name_an_accepted_holder_of_this_session`;
  `busy_ignores_requests_that_hold_no_reservation`;
  `busy_without_a_known_holder_points_to_pending_operations`;
  `a_deleted_branch_offers_give_up_and_no_share`;
  `the_conflict_banner_says_work_is_safe_and_offers_nothing`.
- [ ] **Code:** `model/operations.rs`, `view/operations.rs`,
  `view/item/banners.rs`, `view/item/share.rs`, Keys and Hosts routes in
  `view/workspace/requests.rs`.
- [ ] **Verify:** `… share`, `… operations`, then the four lines.
- [ ] **Commit; review.**

## Task 10b: Give Up And Resume

- [ ] **Tests first:** `give_up_is_offered_exactly_when_the_record_is_accepted`
  (including `unlock_required` after Not now);
  `give_up_is_offered_on_exactly_the_rows_that_name_it`;
  `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up` (P14);
  `stop_is_not_offered_without_a_remote` (R-15); E-10;
  `resume_is_offered_for_a_comment_publication_row`;
  `resume_is_not_offered_for_a_local_context_sync`;
  `an_accepted_resume_offers_stop`.
- [ ] **Code:** `model/operations.rs`, `view/operations.rs`,
  `view/workspace/requests.rs`.
- [ ] **Verify:** `… operations`, `… workspace`, then the four lines.
- [ ] **Commit; review. Lane B checkpoint.**

---

# Lane C: The Composer Widget (`d4-lane-c`)

## Task 11a: Editor, Toolbar And Footer

- [ ] **Tests first** (`view/item/composer_tests.rs`):
  `the_composer_is_a_compact_editor_with_the_mode_switch`,
  `ctrl_shift_m_switches_mode_in_a_composer`,
  `the_mini_toolbar_formats_the_selection_and_arrows_move_along_it`,
  `labels_and_footer_follow_the_remote_and_the_kind`,
  `placeholders_differ_for_new_and_reply`,
  `the_draft_indicator_shows_kept_and_not_kept`,
  `a_restored_text_with_math_fences_is_not_changed`.
- [ ] **Code:** `view/item/composer.rs`, `view/item/composer/*`; additive
  `view/editor.rs` only if `open_editor` needs a compact mode.
- [ ] **Verify:** `… composer`, `… editor`, then the four lines.
- [ ] **Commit; review.**

## Task 11b: Events, States And The Discard Prompt

- [ ] **Tests first:** `ctrl_enter_submits_and_an_empty_composer_does_not`,
  `posting_is_read_only_and_submits_nothing`,
  `failed_offers_exactly_the_actions_its_owner_sets`,
  `esc_with_text_asks_to_discard_and_keep_editing_keeps_it`,
  `esc_when_empty_cancels_silently`, `tab_leaves_the_composer`.
- [ ] **Code:** `view/item/composer.rs`.
- [ ] **Verify:** `… composer`, then the four lines.
- [ ] **Commit; review. Lane C checkpoint.**

---

# Lane D: Discussion Integration (ticket worktree)

Before Task 12: Lanes A, B and C merged; `origin/main` merged; facts
re-checked; whole desktop bin suite green; lane worktrees and branches
removed.

## Task 12: Placement And Drafts

- [ ] **Tests first** (`view/item/posting_tests.rs`, `discussion_tests.rs`):
  `the_new_comment_composer_follows_the_threads`,
  `reply_opens_one_composer_in_its_thread`,
  `reply_on_a_reply_targets_its_root_and_names_the_comment`,
  `moving_the_reply_keeps_each_threads_draft`,
  `an_items_comment_drafts_reopen_in_their_threads`,
  `a_draft_begins_on_the_first_keystroke_after_the_listing`; E-11.
- [ ] **Code:** `view/item/discussion.rs`, `discussion/rows.rs`,
  `model/discussion.rs`, `view/item/posting.rs`, `view/item.rs`.
- [ ] **Verify:** `… discussion`, `… posting`, then the four lines.
- [ ] **Commit; review.**

## Task 13: Posting And Outcomes (test-first, design checkpoint)

Task 13 implements the design's **Required outcomes of posting recovery**
(P1–P10). Step 1 writes one test per row; Step 2, before any code, the
implementer proposes the posting state machine to the orchestrator, who
checks it against P1–P10 and the outcome table and approves it or asks the
owner. Only then Step 3 writes the code.

- [ ] **Tests first** (`view/item/posting_tests.rs`), one per P-row:
  `p1_a_shared_post_is_one_file_one_checkpoint_shown_once`,
  `p2_retry_sharing_writes_nothing_more` (`publish_over_ssh`, U-2),
  `p3_a_stopped_post_offers_share_not_retry`,
  `p4_an_adopted_draft_is_listed_first_then_posted_as_the_same_request`,
  `p5_post_as_new_needs_no_record_and_no_listed_comment`,
  `p6_edited_text_after_a_finished_failure_posts_under_a_new_comment_id`,
  `p7_an_owed_failure_offers_retry_discard_and_copy`,
  `p8_occupied_path_keeps_the_text_and_offers_discard`,
  `p9_closing_the_tab_while_posting_still_retires_the_draft`,
  `p10_a_second_submit_while_posting_is_refused`,
  `p11_a_relaunch_resends_the_same_request_before_listing`,
  `p12_a_mismatch_keeps_the_text_and_offers_no_retry`,
  `p13_edited_text_retargets_the_draft_before_posting`,
  `p15_internal_error_on_any_resend_is_maybe_saved`,
  `p16_post_as_new_needs_a_complete_current_listing`,
  `p17_an_unknown_record_offers_retry_and_check_only`,
  `p18_discard_after_a_written_file_says_it_stays`;
  and:
  `saving_locally_without_a_remote_shows_saved_here` (real library);
  `a_failed_checkpoint_keeps_the_text_and_adds_no_row`;
  `posting_shows_a_pending_row_until_the_outcome`;
  `posting_never_commits_the_dirty_body`;
  `the_reply_composer_cannot_move_while_posting`;
  `an_outcome_applies_only_to_its_comments_composer`;
  `an_outcome_after_a_tab_switch_or_reopen_never_replaces_newer_text` (A17);
  `quitting_during_an_accepted_post_leaves_it_saved_and_cancelled` (decision 15,
  K-10: request hook holds the post after acceptance; quit; the envelope is
  `cancelled` with the checkpoint committed; no Resume row is offered);
  `navigation_save_done_close_and_quit_submit_no_publication` (A16).
- [ ] **Design checkpoint** (Step 2 above), recorded in the ledger.
- [ ] **Code:** `view/item/posting.rs`, `view/item/discussion/rows.rs`,
  the drain hook in `services/mod.rs` (P9).
- [ ] **Verify:** `… posting`, then the four lines.
- [ ] **Commit; review.**

## Task 14: Reload, Contexts And Order

- [ ] **Tests first:** `a_committed_comment_reloads_the_thread`,
  `a_pending_discovery_refreshes_then_reloads`,
  `a_committed_comment_keeps_its_context_from_release` (R-4),
  `order_and_parents_hold_after_rebuild_and_reopen` (A6),
  `the_chip_survives_a_reload_and_clears_when_shared`.
- [ ] **Code:** `view/item/reads.rs`, `services/created_contexts.rs`,
  `view/item/posting.rs`.
- [ ] **Verify:** `… posting`, `… created_contexts`, then the four lines.
- [ ] **Commit; review.** Whole desktop bin suite; ticket comment.

---

# Part E: Keyboard, Privacy, Native Evidence And The Gate

## Task 15: Keyboard Journey, Privacy And The Checklist

- [ ] **Tests:** `view/keyboard_tests/discussion.rs`:
  `a_keyboard_only_user_comments_replies_and_posts`,
  `tab_order_runs_header_share_chips_page_discussion_composer`;
  `view/editing_privacy_tests.rs`:
  `comment_text_never_reaches_an_entry_quit_dialog_or_message`.
- [ ] **Checklist:**
  `docs/plans/2026-11-02-wave-03-desktop-04-on-screen-checklist.md`
  (design, Native checklist), in D3's format.
- [ ] **Verify:** `… keyboard`, `… privacy`, then the four lines.
- [ ] **Commit; review.** Ask the owner to run the checklist on a release
  build (`devenv shell -- cargo run --release --locked --features desktop --bin manyhands`);
  record only what the owner reports.

## Task 16: Gates, Reviews And Review-Ready

- [ ] `git fetch origin`; merge `origin/main`;
  `git merge-tree --write-tree HEAD origin/main` shows no conflict. If the
  merge brought desktop changes, ask the owner to re-run the affected
  checklist sections.
- [ ] Gates, each count recorded in the ledger:
```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
devenv shell -- cargo run --locked --features desktop --bin manyhands
```
  (The desktop smoke test needs a display.)
- [ ] Whole-branch SPEC and QUALITY reviews by fresh reviewers; fixes as
  their own commits.
- [ ] Fill the Cycle's Acceptance Status column; review-ready: commit the
  `ticket.md` status edit, then `manyhands-cli ticket save`; ticket
  comment; push the ticket branch.
- [ ] After review approval: close the ticket (`ticket close` if C5 has
  merged, else a canonical ticket-file edit) in the final push before
  merge.

## Plan Self-Review

| Acceptance / ruling / decision | Task |
| --- | --- |
| A1 | 12, 15; native §2 |
| A2 | native §3 |
| A3 | 13; native §4 |
| A4 | 13; native §4 |
| A5 | 2, 8, 11b, 13 |
| A6 | 14; native §2 |
| A7 | 2, 9, 10a; native §5 |
| A8 | 13; native §6 |
| A9 | 2, 9, 10b; native §6 |
| A10 | 3, 10a, 13; native §7 |
| A11 | 6, 12; native §2 |
| A12 | 13 |
| A13 | 10a; native §5 |
| A14 | 10b; native §4 |
| A15 | 15 |
| A16, A17 | 13 |
| R-1 | 2, 10b |
| R-3, R-13 | 2, 10a |
| R-4 | 14 |
| R-8 | 13 |
| R-9, R-10 | 11a |
| R-12 | 3, 11b, 13 |
| Decision 5 | 12 |
| Decision 7 | 8, 11a |
| Decision 8 | 2, 10a, 10b |
| Decisions 9, 10 | 8 |
| Decision 11 | 10b |
| Decision 12 | K-8; native §6 |
| Decision 13 | 10a |
| Decision 14 | 13, 14 |
| Decision 15 | 13; native §4 |
| R-16 | 2, 13 |
| R-14 | 2 |
| R-15 | 10b |
| P1–P18 | 13 (P14 also 10b) |
| E-1 – E-7 | 4 (E-6 also 8) |
| E-8, E-9 | 7 |
| E-10 | 10b |
| E-11 | 12 |
