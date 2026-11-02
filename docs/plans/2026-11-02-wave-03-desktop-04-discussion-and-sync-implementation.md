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

> **For agentic workers:** Draft, not approved. Implementation starts only
> once the owner has approved the documents and authorized it. Then use
> `implementing-a-cycle` task by task, re-read the design, repeat the
> ticket preflight, and record every checkpoint on ticket
> `01M4CC0VN1KTQFSVR99378TDHV`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Comment, reply, post and share from the desktop, with truthful
progress and outcomes, safe-point stop, credential recovery and recoverable
comment drafts, consuming C4's bindings unchanged.

**Architecture:** Part 0 adds the seams (intents, safe-point stop, give up,
outcome model, placeholders removed, `ItemPane` slots, keys). Lanes A
(comment drafts), B (share, progress, recovery) and C (composer widget)
then run in parallel on lane branches. Lane D wires them into the
discussion. Part E proves the keyboard, privacy and native journeys.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
`zorite-editor` `=0.10.0` (already locked; no new package); Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-04-discussion-and-sync.md) and
[design](2026-11-02-wave-03-desktop-04-discussion-and-sync-design.md), and
`docs/design/item_view/04-comments.md` for sizes and copy. "Fact n",
"U-n", "R-n", "E-n" and "K-n" refer to the design.

## Global Constraints

**Where and how to run.**
- Part 0, Lane D and Part E run in the ticket worktree,
  `.manyhands/worktrees/01M4CC0VN1KTQFSVR99378TDHV`, on branch
  `manyhands/ticket/01M4CC0VN1KTQFSVR99378TDHV`.
- Lanes A, B and C each run in their lane worktree
  (`<main checkout>/.worktrees/d4-lane-{a,b,c}`, branch `d4/lane-{a,b,c}`),
  created at the Part 0 checkpoint (decision 1). Lane branches are never
  pushed.
- Run every Rust command as `devenv shell -- cargo …` with `--locked`.

**No library change.** No task changes `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. A gap stops
the task: file a library ticket with `file-ticket` and ask the owner.

**File ownership.** Each task changes only files in its owner's row of the
design's File ownership table. Anything else stops the task.

**GPUI.** Only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding comment text derives `Debug`. Comment text
never enters an operation entry, notification, dialog message, log or
error.

**Test isolation.** Own temporary repositories, data directory and Git
home per test. No network. No desktop test initializes the transport.

**Editor tests** follow D3's rules: set the cursor, focus and draw before
typing; window-level `simulate_input`/`simulate_keystrokes`; never
Ctrl-Cmd-Space; `cx.executor().allow_parking()` when waiting on the worker.

**Test runs.**
- `cargo test` takes one positional filter; each Verify line uses one and
  checks that more than zero tests ran.
- Desktop tests:
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`.
- Targeted tests between tasks. The full suite runs once, in Task 16.
- Each lane checkpoint runs the whole desktop bin suite
  (`--features desktop-test-support --bin manyhands`, no filter).

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

**Existing tests** change only as E-1 to E-11. Any other change stops for
the owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Git identity is
passed per command if none is configured
(`git -c "user.name=Vince Hodges" -c user.email=vince@imbas.ca`).

**Merging.** Merge `origin/main` into the ticket branch at the Part 0
checkpoint and before Task 12; never rebase. Lane branches merge into the
ticket branch with `--no-ff`. Before the final checkpoint, run
`git merge-tree --write-tree HEAD origin/main`.

**Ticket records.** After each task, a ticket comment records the commit,
tests run with counts, and any ruling. Use `manyhands-cli comment add`
against the main checkout (C4 is merged), or a canonical file under
`.manyhands/comments/01M4CC0VN1KTQFSVR99378TDHV/` if the CLI is not
usable from a lane. Take ULIDs from `manyhands-cli id new` and
`created_at` from the UTC clock. Lane tasks record their comments on the
ticket branch when the lane merges.

**Execution ledger.**
`docs/plans/2026-11-02-wave-03-desktop-04-discussion-and-sync-execution.md`,
created in Task 0, with a section per lane.

**File size.** New desktop source files stay below about 500 lines.

**Implementers never dispatch subagents.** The orchestrating session
dispatches lane implementers and reviewers, merges lanes and runs
checkpoints.

## Review Focus

1. **One comment, once.** Post with an unreachable remote, Retry sharing
   twice, quit, relaunch, Resume: exactly one comment file and one
   checkpoint commit for it. Pinned in Tasks 13 and 14.
2. **Stop at a safe point** on an accepted share: the token is cancelled
   and the outcome reports what completed. Pinned in Tasks 2 and 9.
3. **Dirty body, post comment:** the body file in the context is
   unchanged and the body draft still holds the text. Pinned in Task 13.
4. **Unlock cancelled:** no second prompt until Unlock… is pressed.
   Pinned in Task 9.
5. **A comment draft survives a dropped registry** and reopens in its
   thread with the same comment ID. Pinned in Tasks 6 and 12.

## File Map And Dependency Order

| Task | Owner | Files |
| --- | --- | --- |
| 0 | Ticket | Ledger. |
| 1 | Part 0 | `services/requests.rs`, `services/requests/build.rs`, `services/requests_tests.rs`, `services/mod.rs`, `test_support/delaying.rs` |
| 2 | Part 0 | `services/worker.rs`, `services/worker_tests.rs`, `view/workspace/requests.rs` |
| 3 | Part 0 | `model/publication.rs`, `model/publication_tests.rs`, `model/mod.rs` (one `mod` line) |
| 4 | Part 0 | `model/item.rs`, `model/item_tests.rs`, `view/item.rs`, `view/item_tests.rs`, `view/item/header.rs`, `view/item/header_tests.rs`, `view/item/discussion.rs`, `view/item/discussion_tests.rs`, `view/item/{share,composer,posting}.rs` (stubs), `keys.rs` |
| 5 | Part 0 | Checkpoint, lane worktrees. |
| 6, 7 | Lane A | Design table, Lane A row. |
| 8, 9, 10 | Lane B | Design table, Lane B row. |
| 11a, 11b | Lane C | Design table, Lane C row. |
| 12, 13, 14 | Lane D | Design table, Lane D row. |
| 15, 16 | Part E | Design table, Part E row; ledger; ticket. |

Order: 0 → 1 → 2 → 3 → 4 → 5 → {6–7 ∥ 8–10 ∥ 11a–11b} → lane merges →
12 → 13 → 14 → 15 → 16.

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
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands
```
  The full suite is not run here: D3's ledger recorded it at over two
  hours, and D4 changes no library code. The desktop bin suite is the
  baseline for every lane. A failure is resolved or accepted by the owner
  before Task 1.
- [ ] **Step 2: Re-check facts.** `git diff --stat 00bef3da HEAD -- src/repository src/results.rs src/drafts src/desktop .github`. Re-read each (research) fact and each fact whose file changed; correct the design before coding.
- [ ] **Step 3: Settle U-1 to U-3** by reading the source (no new tests):
  U-1 in `bind/sync.rs` and the domain's context lookup; U-2 by tracing
  `RuntimeUninitialized` through `outcome.rs`; U-3 in
  `services/mod.rs`'s handling of `changes_items`. Record each answer in
  the ledger. An answer that changes a ruling stops for the owner.
- [ ] **Step 4: Sweep for unlisted test edits.** Search every `*.rs` under
  `src/desktop/` for `LATER`, `LATER_NOTE`, `header_later`,
  `later-action`, `later-note`, `COMMENT_LATER`, `COMMENT_ON`,
  `SYNCHRONIZATION_UNAVAILABLE`, `CancelResult::`, `JobKind::`,
  `Intent::`, `RecoveryStep::Cancel`, `"discussion"`, `"Comment"`,
  `"Share"`, `stored_for`. Compare each hit with E-1 to E-11; an unlisted
  breakage stops for the owner. Record the sweep.
- [ ] **Step 5: API audit refresh.** Record in the ledger the file and
  line of each binding in facts 2–12 at the merged base.
- [ ] **Step 6: Ledger, commit, ticket comment.**

---

# Part 0: Seams

## Task 1: Intents

**Files:** Part 0 row, Task 1.

- [ ] **Step 1: Tests first** in `services/requests_tests.rs`:
  `comment_add_builds_its_mutation_with_no_author`,
  `item_and_repo_sync_build_their_mutations`,
  `the_three_stop_at_safe_points_and_others_do_not`,
  `a_comment_label_never_holds_its_text`.
- [ ] **Step 2:** Add `CommentBody` (no `Debug`), the three `Intent`
  variants (design, Intents), `build` arms, labels ("Post comment on
  {title}" from the tab's title passed in the intent's label field as the
  existing intents do; "Share {title}"; "Share main"), `job_kind`
  (`JobKind::Comment`, `JobKind::Synchronize`, added in `services/mod.rs`
  and accepted by `DelayingBackend`), `changes_items`, and
  `stops_at_safe_points`.
- [ ] **Verify:**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands requests`
  then the four lint/check lines.
- [ ] **Commit; ticket comment.**

## Task 2: Safe-Point Stop And Give Up

**Files:** Part 0 row, Task 2.

- [ ] **Step 1: Tests first** in `services/worker_tests.rs`:
  `cancelling_an_accepted_sync_signals_its_token` (hold a `RepoSync` job
  past `Accepted` with the existing gate; `cancel` returns `Signalled`;
  the backend's token is cancelled);
  `cancelling_an_accepted_local_job_is_still_too_late` (as today);
  `giving_up_cancels_the_request_then_sends_it_again_under_its_id`.
- [ ] **Step 2:** `Running.stops_at_safe_points`; the `cancel` rule
  (R-1); `WorkerHandle::give_up(id)` (R-2), which runs on the worker
  thread, calls `service().cancel_request`, and re-admits the job from
  `Submitted` under the same request ID; the workspace's
  `RecoveryStep::Cancel` calls `give_up` when the request is finished.
- [ ] **Verify:** `… --bin manyhands worker` and `… --bin manyhands workspace`, then the lint/check lines.
- [ ] **Commit; ticket comment.**

## Task 3: Outcome Model

**Files:** Part 0 row, Task 3.

- [ ] **Step 1: Tests first** in `model/publication_tests.rs`: one test per
  row of both outcome tables (design, Outcomes), from constructed
  `Envelope<MutationDataDto>` values. Copy envelope construction from
  `model/operations_tests.rs`.
- [ ] **Step 2:** `comment_outcome`, `sync_outcome`, and their enums.
- [ ] **Verify:** `… --bin manyhands publication`, then the lint/check lines.
- [ ] **Commit; ticket comment.**

## Task 4: Placeholders, Slots And Keys

**Files:** Part 0 row, Task 4. Named edits E-1 to E-7 (E-6 first half).

- [ ] **Step 1:** Apply E-1 to E-7 exactly as the design lists them.
- [ ] **Step 2:** `LATER` and `header_later` lose Share and Comment; the
  discussion's disabled Comment is removed.
- [ ] **Step 3:** New stub files `view/item/share.rs`,
  `view/item/composer.rs`, `view/item/posting.rs`, each with its type,
  `Default`, and an empty render; the three `ItemPane` fields and their
  single render calls.
- [ ] **Step 4:** `keys.rs`: the `CommentComposer` context with
  `PostComment` (`secondary-enter`) and `CancelComposer` (`escape`), and
  the `ShareItem` action, with stub handlers.
- [ ] **Verify:** `… --bin manyhands item` (covers `model::item`,
  `view::item` and its children), `… --bin manyhands keyboard`, then the
  lint/check lines.
- [ ] **Commit; ticket comment.**

## Task 5: Part 0 Checkpoint And Lane Worktrees

- [ ] Whole desktop bin suite; the lint/check lines.
- [ ] Fresh reviewer of Tasks 1–4 against the design.
- [ ] `git fetch origin`; `git merge origin/main`; re-check facts whose
  files moved (C5 may have merged); `git merge-tree --write-tree HEAD origin/main`.
- [ ] Push the ticket branch.
- [ ] Create the lane worktrees from the ticket branch's HEAD:
```sh
main=/home/vhodges/work/src/manyhands
for lane in a b c; do
  git -C "$main" worktree add -b "d4/lane-$lane" "$main/.worktrees/d4-lane-$lane" manyhands/ticket/01M4CC0VN1KTQFSVR99378TDHV
done
```
- [ ] Ticket comment: Part 0 checkpoint, base and lane SHAs; notify the
  owner.

---

# Lane A: Comment Drafts (worktree `d4-lane-a`)

## Task 6: Comment Draft API

- [ ] **Tests first** in `services/drafts/comments_tests.rs`:
  `beginning_a_comment_draft_allocates_one_comment_id`,
  `each_thread_keeps_its_own_draft`,
  `a_comment_draft_flushes_within_a_second`,
  `a_dropped_registry_leaves_the_draft_to_adopt_with_its_id`,
  `retiring_removes_the_draft`,
  `stored_for_still_ignores_comment_drafts`.
- [ ] **Code:** `services/drafts/comments.rs` (design, Lane A), one `mod`
  line in `services/drafts.rs`.
- [ ] **Verify:** `… --bin manyhands drafts`, then the lint/check lines.
- [ ] **Commit** on `d4/lane-a`.

## Task 7: The Drafts Page Opens Comment Drafts

- [ ] **Tests first:** E-8, E-9; new `a_comment_rows_open_opens_its_item`.
- [ ] **Code:** Open on comment rows in `view/drafts.rs` and
  `model/drafts/rows.rs`; `COMMENT_LATER` removed. Open navigates to the
  item tab (Lane D's composer picks the draft up; until then the tab
  opens).
- [ ] **Verify:** `… --bin manyhands drafts`, then the lint/check lines.
- [ ] **Lane A checkpoint:** whole desktop bin suite; fresh reviewer;
  merge into the ticket branch (design, Lane mechanics); ticket comments
  for Tasks 6–7.

---

# Lane B: Share, Progress And Recovery (worktree `d4-lane-b`)

## Task 8: Share Controls

- [ ] **Tests first** in `view/item/share_tests.rs`:
  `share_follows_edit_and_is_enabled_with_a_context`,
  `share_without_a_context_is_disabled_with_its_note`,
  `share_without_a_remote_reads_share_and_opens_remotes`,
  `a_second_press_while_sharing_queues_nothing`,
  `share_main_in_the_repository_detail_submits_repo_sync`;
  E-6, second half.
- [ ] **Code:** `view/item/share.rs`, `view/item/header.rs`,
  `view/sidebar.rs`.
- [ ] **Verify:** `… --bin manyhands share` (the repository-detail test
  lives in `share_tests.rs` too; `view/sidebar.rs` has no test file of its
  own), `… --bin manyhands header`, then the lint/check lines.

## Task 9: Stage, Stop And Unlock

- [ ] **Tests first:** `each_stage_shows_its_text` (DelayingBackend holds
  the job; progress events forwarded); `an_accepted_share_offers_stop_and_stopping_signals_it`;
  `a_stopped_share_says_what_completed`;
  `cancelling_the_prompt_finishes_unlock_required_and_prompts_no_more`;
  `unlock_retries_the_same_request_and_prompts_once` (protected key
  fixture; copy from `view/passphrase_tests.rs`);
  `the_dialog_names_what_waits_and_says_not_now`.
- [ ] **Code:** `view/item/pill.rs`, `view/operations.rs`,
  `model/operations.rs`, `view/passphrase.rs`.
- [ ] **Verify:** `… --bin manyhands share`, `… --bin manyhands operations`,
  `… --bin manyhands passphrase`, then the lint/check lines.

## Task 10: Outcomes And Recovery Routes

- [ ] **Tests first:** one test per `SyncOutcome` row (design, Lane B,
  Outcomes) through a real repository where reachable (no remote:
  `Pending`; `publish_over_ssh`: the failure path) and `DelayingBackend`
  otherwise; `no_selected_key_opens_sharing_and_keys`;
  `a_host_code_checks_the_host_on_its_root`;
  `give_up_is_offered_only_when_the_library_offers_cancel`;
  E-10 and `resume_publishes_a_pending_comment_row`.
- [ ] **Code:** `model/operations.rs`, `view/operations.rs`,
  `view/item/banners.rs` (conflict banner), `view/item/share.rs`.
- [ ] **Verify:** `… --bin manyhands operations`, `… --bin manyhands share`
  (the conflict banner's tests live in `share_tests.rs`), then the
  lint/check lines.
- [ ] **Lane B checkpoint:** whole desktop bin suite; fresh reviewer;
  merge; ticket comments for Tasks 8–10.

---

# Lane C: The Composer Widget (worktree `d4-lane-c`)

## Task 11a: Editor, Toolbar And Footer

- [ ] **Tests first** in `view/item/composer_tests.rs`:
  `the_composer_is_a_compact_editor_with_the_mode_switch`,
  `the_mini_toolbar_formats_the_selection`,
  `labels_follow_the_remote_and_the_kind`,
  `the_explanation_names_ticket_or_document` (R-9),
  `the_draft_indicator_shows_kept_and_not_kept`,
  `a_restored_text_with_math_fences_is_not_changed` (R-10).
- [ ] **Code:** `view/item/composer.rs`, `view/item/composer/*`; additive
  changes to `view/editor.rs` only if `open_editor` needs a compact mode.
- [ ] **Verify:** `… --bin manyhands composer`, `… --bin manyhands editor`,
  then the lint/check lines.

## Task 11b: Events, States And The Discard Prompt

- [ ] **Tests first:** `ctrl_enter_submits_and_an_empty_composer_does_not`,
  `posting_is_read_only_and_submits_nothing`,
  `failed_shows_the_step_and_retry_submits`,
  `esc_with_text_asks_to_discard_and_keep_editing_keeps_it`,
  `esc_when_empty_cancels_silently`.
- [ ] **Code:** `view/item/composer.rs`.
- [ ] **Verify:** `… --bin manyhands composer`, then the lint/check lines.
- [ ] **Lane C checkpoint:** whole desktop bin suite; fresh reviewer;
  merge; ticket comments for Tasks 11a–11b.

---

# Lane D: Discussion Integration (ticket worktree)

Before Task 12: all three lanes merged; `git merge origin/main`;
re-check facts; whole desktop bin suite green; lane worktrees and
branches removed (`git worktree remove`, `git branch -d`).

## Task 12: Placement And Drafts

- [ ] **Tests first** in `view/item/posting_tests.rs` and
  `discussion_tests.rs`: `the_new_comment_composer_follows_the_threads`
  (replaces E-7's test), `reply_opens_one_composer_in_its_thread`,
  `reply_on_a_reply_targets_its_root_and_names_the_comment`,
  `moving_the_reply_keeps_each_threads_draft`,
  `an_items_comment_drafts_reopen_in_their_threads`,
  `a_draft_begins_on_the_first_keystroke_not_on_focus`; E-11.
- [ ] **Code:** `view/item/discussion.rs`, `discussion/rows.rs`,
  `model/discussion.rs`, `view/item/composer.rs` placement in
  `view/item.rs`.
- [ ] **Verify:** `… --bin manyhands discussion`, `… --bin manyhands posting`,
  then the lint/check lines.

## Task 13: Posting And Outcomes

- [ ] **Tests first:** `saving_locally_without_a_remote_shows_saved_here`
  (real repository and library); `an_unreachable_remote_saves_once_and_offers_retry_sharing`
  (U-2; asserts one comment file and one checkpoint commit after two
  retries); `a_failed_checkpoint_keeps_the_text_and_adds_no_row`;
  `posting_shows_a_pending_row_until_the_outcome`;
  `a_second_submit_while_posting_queues_nothing`;
  `posting_never_commits_the_dirty_body` (A12);
  `a_restored_draft_already_saved_is_retired` (R-8).
- [ ] **Code:** `view/item/posting.rs`, `view/item/discussion/rows.rs`.
- [ ] **Verify:** `… --bin manyhands posting`, then the lint/check lines.

## Task 14: Reload, Contexts And Order

- [ ] **Tests first:** `a_committed_comment_reloads_the_thread`;
  `a_pending_discovery_refreshes_then_reloads`;
  `a_committed_comment_keeps_its_context_from_release` (R-4);
  `order_and_parents_hold_after_rebuild_and_reopen` (A6: two roots, three
  replies posted out of order, `IndexRebuild`, close and reopen);
  `the_chip_survives_a_reload_and_clears_when_shared`.
- [ ] **Code:** `view/item/reads.rs`, `services/created_contexts.rs`,
  `view/item/posting.rs`.
- [ ] **Verify:** `… --bin manyhands posting`, `… --bin manyhands created_contexts`,
  then the lint/check lines.
- [ ] **Lane D checkpoint:** whole desktop bin suite; fresh reviewer;
  ticket comments.

---

# Part E: Keyboard, Privacy, Native Evidence And The Gate

## Task 15: Keyboard Journey, Privacy And The Checklist

- [ ] **Tests:** `view/keyboard_tests/discussion.rs`:
  `a_keyboard_only_user_comments_replies_and_posts` (A1, no remote);
  `tab_order_runs_header_share_chips_page_discussion_composer`.
  `editing_privacy_tests.rs`: `comment_text_never_reaches_an_entry_or_message`.
- [ ] **Checklist:** write
  `docs/plans/2026-11-02-wave-03-desktop-04-on-screen-checklist.md`
  (design, Native checklist), in D3's format.
- [ ] **Verify:** `… --bin manyhands keyboard`, `… --bin manyhands privacy`,
  then the lint/check lines.
- [ ] Ask the owner to run the checklist on a release build; record only
  what the owner reports.

## Task 16: Gates And Review-Ready

- [ ] `git fetch origin`; `git merge origin/main`;
  `git merge-tree --write-tree HEAD origin/main` shows no conflict.
- [ ] Gates:
```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```
  The full suite runs once here (AGENTS.md lets a `working-on-a-ticket`
  run skip it; this Cycle is planned, so it runs it), detached, with a
  two-hour budget.
- [ ] Fill the Cycle's Acceptance table with evidence; mark the ticket
  review-ready with a comment; push the ticket branch.
- [ ] After review approval, close the ticket as a canonical ticket-file
  edit (or `ticket close` if C5 has merged) in the final push before
  merge.

## Plan Self-Review

| Acceptance / ruling | Task |
| --- | --- |
| A1 | 12, 15; native §2 |
| A2 | native §3 |
| A3 | 10, 13; native §4 |
| A4 | 13; native §4 |
| A5 | 8, 11b, 13 |
| A6 | 14 |
| A7 | 9; native §5 |
| A8 | 9, 13; native §6 |
| A9 | 2, 9; native §6 |
| A10 | 3, 10; native §7 |
| A11 | 6, 12 |
| A12 | 13 |
| A13 | 10; native §5 |
| A14 | 10 |
| A15 | 15 |
| R-1, R-2 | 2 |
| R-3 | 9 |
| R-4 | 14 |
| R-5, R-6 | 8 |
| R-7 | 10 |
| R-8 | 13 |
| R-9, R-10 | 11a |
| R-11 | 13, 14 |
| E-1 – E-7 | 4 (E-6 also 8) |
| E-8, E-9 | 7 |
| E-10 | 10 |
| E-11 | 12 |
