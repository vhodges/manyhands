---
title: "Wave 04 Cycle 03b Execution Ledger"
date: 2026-11-13
status: "draft"
author: "Claude"
---

# Wave 04 Cycle 03b Execution Ledger

Written only by the orchestrator. Ticket `01M784Y94Q3AXT6XJX38F5VK1V`
(`vh-dt0aq`), branch `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`.
Execution is subagent-driven and serial (decision 26): one fresh implementer
per task, then a fresh reviewer. Subagents run on Sonnet 5.5 (owner's
instruction for this session).

Rulings are recorded as "Ruling: decision — why — cost if wrong". Test edits
are E-1 to E-16 (named) and E-17 onward (further, decision 14), each with its
reason.

## Task state

| Task | Status | Commit | Pushed |
| --- | --- | --- | --- |
| 0 | done (no source change; no reviewer, see ruling R0-1) | `f5e01cb9` (ledger); `a5b5ac29` (ticket comment) | yes |
| 1 | done, review approved first time | code `ec67fea7` (base `a5b5ac29`); ledger `df8fa9cf` | yes |
| 2 | done, review approved first time | code `879da113` (base `df8fa9cf`); ledger `622dfaa5` | yes |
| 3 | done, review approved with Minors parked | code `bbaa6663` (base `622dfaa5`); ledger `f487e094` | yes |
| 4 | done, review approved with Minors parked | code `94c7faab` (base `f487e094`); ledger `3b377441` | yes |
| 5 | done, two reviewers, approved after fix round 1; checkpoint comment follows | code `110aff6f` (base `3b377441`), fix `484b0f60`; ledger commit follows | yes |

## Base

- `origin/main` is `ba95e1b7`. `git merge origin/main` was a no-op: main was
  merged at `81789331`.
- Start of execution: branch HEAD `396db2a4`. Gates ran at `ff381f35`; the two
  later commits (`3367b9a7`, `396db2a4`) change only docs: the Cycle document
  and the handoff brief.
- `src/desktop` is unchanged since `81789331`. `Cargo.lock` is unchanged;
  `gpui-kit`, `gpui-base` and `gpui-component` are all 0.6.6 (U-6 holds).

## Authority note

`3367b9a7` (a docs commit on the branch) amended the Cycle document and the
brief to allow pushing other branches. The owner's first message of this
session still said otherwise, so the orchestrator did not rely on the
amendment. The owner then confirmed it in the session (2026-11-13): pushing
branches is fine, this ticket's after each task and others when a comment on
another ticket publishes them, naming which. A pull request, merging into
main, closing the ticket, worktree cleanup, native CI and creating tickets
still need the owner's go-ahead at the time.

**Branches published so far:** `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`
only.

## Task 0

Gates at `ff381f35`, through Devenv, `--locked`: `cargo check --all-features`
passed (54 s); `cargo fmt --check` passed; `cargo clippy --all-targets
--all-features -- -D warnings` passed (1 m 10 s). No test ran.

**Fact 46** (Cycle 03a's deferred Minor 2), as the design records it: Cycle
03a's ledger recorded that Activity could list an item the count no longer
had, when an older check is rejected after a share. It was fixed in
`43f411a0`, item 2: `unshared_after` keeps the list for an older attempt and
still removes from `last.listed` the shares heard after it. (Verbatim text:
design lines 756-775; copied in the SDD report `task-0-report.md`.)

**E-list sweep** (full per-line lists in the SDD workspace's `task-0-report.md`,
git-ignored): 597 matches. 376 test sites are named by an E-n and 151
production sites are named by a task. 7 production and 16 test matches are
false positives (a local `request_id` helper returning the library
`RequestId`; `FINISHING_POLL`; names containing "needs_attention"). 24 test
sites feed `WorkerEvent::Finished(` and need no edit.

Candidate further edits (decision 14), to be assigned when the task that
breaks them runs:

- E-17 (candidate, Task 10): `view/dock_tests/toolbars.rs:323`
  (`behind_a_dialog` loops `ToolbarButton::ALL`; Task 10 adds buttons to it).
  Probably passes unchanged.
- E-18 (candidate, Task 11): `view/keyboard_tests/shell/support.rs:102` (the
  `said` helper reads `Control::Attention`). Reaches E-13 if the item's text
  changes.
- `credential_tests.rs:114,275` call E-6's `reopen`; no E-n names the file.
  It follows `reopen` in Task 8 (E-6) and is recorded then.
- `footer_tests/mod.rs` `toggle_names_follow_their_state` and
  `status_items_say_and_choose` read `Control::Attention`: covered by E-4
  ("every other reader", fact 41).

Unnamed production sites (21, ten files): `WorkerEvent::Finished(` consumers in
`services/scheduler/{reconfigure,retire}.rs` and `services/worker.rs` (no task
changes the enum); users of `scope_name`, `outcome_label` and `state_label`
that survive only if Task 3 re-exports them from their old paths
(`view/conflict/entry.rs`, `view/lifecycle.rs`); `workspace.rs:135,138`, sized
by `ToolbarButton::ALL` (Task 10); `workspace/requests.rs:241` (`show_holder`),
which the plan does not name — assigned to Task 9.

E-8's cite of a "window resize" at `polling_journey.rs:133` is loose:
that line is `set_activity_height(150., cx)`, the same kind of edit.

## Pre-flight scan

The scan table is in the SDD workspace (`preflight.md`): 16 findings, none
blocking, no assertion-free test and no mandated verbatim duplication. Every
finding is ruled below; the rulings are handed to each implementer and
reviewer as a file.

- Ruling R0-1: no reviewer for Task 0 — it changed no source or document and
  its evidence is gate output and grep lists — a misreported gate would show
  at Task 1's `cargo check` — cost if wrong: one wasted task start.
- Ruling P-F1: `model/operations.rs` ends near 600 lines after Task 1, as design
  F1 says; the plan's "below 500" applies to new files and to
  `view/operations.rs` (Task 9, A13) — design is binding — a larger file than
  hoped; split later.
- Ruling P-F2: the `stage_words` half of `stages_are_shown` is a model test
  (`desktop::model::operations::tests::stages_are_shown`); the `state_label`
  half is a view test named `state_label_shows_stages` — `state_label` is a
  view function — a renamed test.
- Ruling P-F3: one private non-test helper holds the share classification body;
  `classify_share` (`cfg(test)`) and `apply(Finished)` both call it — avoids
  copying the body — none.
- Ruling P-F4: where the plan omits a signature (`RowKey`, `Kind`, `Offer`,
  `ActivityAction`, `ActivityLog` fields, `Brought`), the design (A1, A4) is the
  signature of record — Cycle governs design governs plan — a mismatch is
  reported, not guessed.
- Ruling P-F5: new files stay below about 500 lines; an implementer checks
  `wc -l` of each file it creates or grows past 500 and reports; a file that
  would exceed is split along the design's module lines and recorded.
  Pre-existing large files (`polling.rs`, 697) are not split unless a task
  grows them materially — late splits are the cost.
- Ruling P-F6: Task 3 builds `RowKey` per design A1 in full, `RetireNote` and
  `Retired(seq)` included, marked `#[allow(dead_code)] // Used from Task 4` —
  A1 defines the type — none.
- Ruling P-F7: `Inputs.drafts` follows the design; if it is silent, a reference
  matching `StateInputs.drafts` — consistency — a trivial change.
- Ruling P-F8: where the plan names no module path or exact name (the
  completed-attempt count, the applied-check record,
  `brought_in_is_kept_for_the_session`, P26), the implementer takes the name from
  the design's test list, else a descriptive one, and reports the full exact
  path for the ledger — the exact-name rule needs names — none.
- Ruling P-F9: Task 6 reads the old tests' cases before writing its Step 1 table;
  the later step only deletes — table first — none.
- Ruling P-F10: each implementer lists every `#[allow(dead_code)] // Used from
  Task N` it adds and removes those marked for its own task; Task 13 greps
  `Used from Task` and expects none — none.
- Ruling P-F11: `retiring_tests.rs` is dropped from Task 7's file list — its
  only test is out of scope per the plan — none.
- Ruling P-F12: implementers commit (one per task) and report; the orchestrator
  writes the ledger, pushes and posts ticket comments — Global Constraints,
  Roles — none.
- Ruling P-F13: Task 9 adds `activity_menu_open()` (or the design's named
  accessor); Task 11's popover element id follows the design's Part E literal
  and Task 12 uses the same one; a mismatch is reported — design is binding —
  an id rename.
- Ruling P-F14: line cites into `operations.rs` are re-grepped after Task 1 —
  none.
- Ruling P-F15: the commit trailer is `Co-Authored-By: Claude Sonnet 5.5
  <noreply@anthropic.com>` (this session's attribution), not the plan's Opus
  5.5 — the session's attribution rule governs — none.
- Ruling P-F16: `row.menu` holds every offer, the button's included, button
  first; the Goal's "a menu with the rest" is loose prose — design A1 and the
  seam agree — none.

## Task 1

`operations.rs` 1276 → 619 lines; new files `progress.rs` 127, `recovery.rs` 313,
`pending.rs` 213, `lifecycle.rs` 64. The reviewer compared moved lines with
`-M --color-moved`: only imports, re-exports and three visibility widenings
differ (`lifecycle_answered`, `takes_up_remote_work`, `check_now_left_active`,
private → `pub(super)`).

- Ruling T1-1: `RETRY_SHARING` is not re-exported and `recovery_label` is
  re-exported under `#[cfg(test)]` only — an unused `pub(crate) use` fails
  `-D warnings`; the only outside caller of `recovery_label` is
  `operations_tests.rs:15` and nothing resolves `operations::RETRY_SHARING` —
  a later task that wants either imports it from `operations::recovery`.
- P-F14 stands: line cites into `operations.rs` elsewhere in the docs are
  stale and are re-grepped by the task that needs one.

## Task 2

F2 (a share classified once) and F3 (the stage shown). The drain's
`note_share_attempt` (`services/mod.rs`, now 1150 lines, was 1126) runs before
`apply`; `Queued` clears `share_attempts` and `share_results`; one private
`classified()` serves `apply(Finished)` and the `cfg(test)` `classify_share`.

- Ruling T2-1: `stage_words` falls back to "main" for Integrating and Publishing
  (F3, P10, the plan), while `stage_text` keeps "the primary branch" for the
  lifecycle copy that has a primary — the design says so — `state_label` has no
  primary, so a generic running row reads "Merging into main…" even where the
  primary has another name; reachable only through that fallback.
- Ruling T2-2: the P9 table has a fourth row (a share noted after a give up) and
  asserts the private maps are empty after the retry — a pin on R-39 — none.
- Tests were first seen failing as compile errors, not assertions (the whole test
  binary shares one build).

## Task 3

Row types (A1, `RowKey` in full), request rows (S1), their copy (B1–B3, B5),
`activity_time`, and the R-41 move of the pure text helpers and the lifecycle
outcome lines into `model/activity/{copy,lifecycle_lines,intent_words}.rs` with
re-exports from the old view paths. New files all under 500 lines (largest
`activity_tests.rs` 490, `requests.rs` 418); `request_rows` is split into
`requests/offers.rs`.

- **U-1:** holds. Only `Report` lacked `PartialOrd, Ord`; both were added
  (a desktop type). **U-7:** holds: `Workspace::tab_of(&TabKey::Item{..})`, then
  `TabContent::pane(id)`, then `pane.read(cx).model.last_title()`; it needs an
  `AuthoringKind` → `ItemDtoKind` conversion, which Task 5's title closure supplies.
- **Dead-code markers (9, all plain):** Task 4: `RowKey`, `Target`,
  `Kind::ToShare`, `ActivityAction`. Task 5: `Subject::Operation`, `ActivityRow`,
  `RequestInputs`, `request_rows`, `activity_time`.
- Ruling T3-1: the label and state constants beyond R-41's list moved into
  `copy.rs` and are re-exported from their view paths; `LifecycleRow` stays for
  Task 4 — the model needs them for `request_rows` — none.
- Ruling T3-2: Task 5 contracts the interpretations bind (reviewer-confirmed
  against A1): (1) `RequestFacts.recoveries` is the already-shown set — the Workspace
  removes dialog-owned and hidden actions; the model finds Abandon from
  `recovery[n].action`. (2) `shown_offers` arrives already filtered of
  Republish… and Stop and republish… while a republish runs. (3)
  `share_words: Some` on a BranchDeleted answer means "republish running" and
  shows a disabled "Republishing…". (4) A succeeded share offers only Dismiss.
  (5) An Owed retirement row is `Kind::Failure`; its `candidate` follows the
  envelope. — design A1 field docs — Task 5 would build the facts wrongly.
- **Parked Minors** (carried to the final review, none blocks):
  1. `view/lifecycle/outcome.rs` still defines `STOP`, `STOPPING`, `UNLOCK`,
     `OPEN_SHARING_AND_KEYS` and `CHECK_HOST` that `copy.rs:26-60` now defines
     again; the strings can drift. Task 9 (the old drawing goes) re-exports the
     model's.
  2. R-41's "re-export each" was not kept for `APPLICATION`, `ALREADY_SHARED` and
     the offer-label constants (`UNLOCK`, `GIVE_UP`, `OPEN_SHARING_AND_KEYS`,
     `CHECK_HOST`, `SET_IDENTITY`, `SHOW_IT`, `SET_UP_SHARING`): nothing outside
     uses them, and an unused re-export fails `-D warnings`.
  3. An Owed row with a failed envelope offers Dismiss only (`offers.rs:44`);
     today's entry would show that envelope's recovery actions. The design is
     silent: confirm at the final review that this is intended.
  4. A finished Success share with no `share_result` falls back to the Doing text
     as its title; unreachable today.
  5. A lifecycle Noop, Pending or "Not shared" outcome line is covered only through
     `request_copy_table`, not `request_rows`.

## Task 4

State rows (S2–S14, `model/activity/states*`), `AttentionCause` and `RepoState.cause`
(`repository_state/cause.rs`), the brought-in history (A5), the retirement events
(`RetireEvent`, `next_retire_event`, appended by `apply(Finished)` just before the
dismissal for Retired, Attention, Unreachable and Retry only), `LifecycleRow` (with
`new`) moved to the model, `each_repository`. The reviewer confirmed that nothing
reads `PollSchedule`'s retirement lines. U-5: `checked_now`'s in-flight branch
returns a summary (both branches append). Largest new file 462 lines;
`polling.rs` grew to 740 (it was over 500) and `operations.rs` to 688.

- Ruling T4-1: the pending row's recovery label is "Abandon" (today's, R-44), not
  the "Abandon…" the design's A2, B0, P20 and Part G write — R-44 keeps today's
  labels verbatim — if the owner wants the ellipsis, one constant and the tables
  change; Tasks 9 and 12 use "Abandon".
- Ruling T4-2: S4 non-SSH and S14 titles drop the trailing full stop (B's rule: a
  title never ends with a full stop); an item name that starts a title is
  capitalised — the design is silent — Task 5 makes B3's titles match.
- Ruling T4-3 (binds Task 5): `lifecycle_running(root, item)` must also hold true for
  an in-flight `OperationResume` for a stranded lifecycle, as today's view
  (`view/operations/resume.rs:51-62`) adds the row's own operation to `held`; Task 5
  builds the closure from the Close/Promote pending operations of `(root, item)` in
  `Operations::pending_shown`, and says so in its report — otherwise P22 and
  `p5_a_close_stranded_at_closing_resumes_after_relaunch` regress.
- Ruling T4-4 (binds Task 5): the log reads `Group.sent` on `RetireNote` rows to
  detect a replaced note (T18); the model sets it.
- State tables were not seen failing before their code (no compile-failure RED); the
  implementer's mutation checks on the green tree (swapped causes, a removed
  latest-note rule) failed as expected, and the reviewer judged them adequate. The P8
  test had a real RED.
- `check_all_checks_each_repository` failed once in a 13-test combined run (window
  focus, `raise` returned false) and passed alone twice; treated as the known
  window-test flake. If it shows again at Task 13, it is a doubt to name.
- `repository_state_tests.rs`, `polling_tests.rs` and `activity_tests.rs` gained `mod`
  declarations and `each_source_makes_its_rows` the S2–S14 call: no existing assertion
  edited, no E-17.
- **Parked Minors** (carried to the final review): S9's button is the first enabled
  non-Abandon offer, so a still-running lifecycle with an enabled Review changes would
  show that instead of the disabled "Still running…" (not reachable with today's
  data); a `RetireNote` stays shown after a later Retired event for the same item
  (R-59 allows it); a doc line is duplicated in `repository_state.rs` ~160;
  `UNNAMED_ITEM` is defined in `copy/states.rs` and `repository_state/words.rs:43`;
  `states_pending.rs`'s header says "S9 to S14" but covers S9.

## Task 5 (design checkpoint)

**The checkpoint.** The implementer wrote the two tables first
(`attention_counts_each_thing_once`, 52 rows; `the_log_keeps_times_and_events`, 19 rows) against
stubs and saw every row fail. Of the 40 ledger lines (24 N, 16 T) 36 were expressible and answered as the
design says; six needed a ruling. **The owner approved all six, 2026-11-13** ("Approve all six"). The design
text was not edited; these rulings amend it (to be listed as document amendments at close):

- Ruling C1: S1 sets `Group.outcome = None` for an Owed retirement row, so N22 clause 1 never settles from it (Task 3 line, test updated) — N22 clause 1 against R-51 — the Owed row counts again if wrong.
- Ruling C2: absorption is sticky per attempt: a failure absorbed while its covering row is kept stays absorbed after the poll answers (N19 after the poll answers: count stays 1) — avoids a 1→2 flicker — a flicker.
- Ruling C3: N22 clause 3 reads "left, or no longer covers its root" for hold and repository absorbers — a hold stays for another repository — a failure counted beside the hold.
- Ruling C4: an absorbing row records itself in `absorbed` for every member of an absorbed group — N2 with clause 3 — an older member counts when the pending row leaves.
- Ruling C5: nothing counts for an operation while its lifecycle runs elsewhere (N25); implemented by the S9 lifecycle row setting `Group.running_operation` so N21 silences its operation (and Task 4's N25 row group carries it) — N1's fallback named only N12 — a failure counted while its lifecycle runs. A1's `running_operation` comment ("S1: …") needs the S9 clause.
- Ruling C6: `dismissed` is `BTreeMap<RowKey, Option<u64>>` holding `Group.sent`; a `RetireNote` with a different `sent` shows again (T18) — A4's set could not tell events apart — a dismissed note stays hidden.
- Ruling C7: `pending_held(&Operations, listed roots)` is a model function; T7 calls it and the Workspace uses it — L3b's trigger was prose only — an untested trigger.
- Ruling C9 (owner, after the review): N19 absorbs by "kept" only a failure first evaluated while its covering row was kept (`first_kept`, per attempt, cleared when `Group.sent` changes) — the review found that a failure counted on its own (count 2) dropped to 1 on any routine poll and was lost if the row then cleared — the original count drop returns.

**E-3** (the old count): `model/attention.rs` and its three test files are deleted; their cases moved into
the tables as rows named "E-3 …" (`attention_moved.rs`, `attention_retired.rs`), each under its N-row;
`retire_resume_resends_original` builds an `ActivityLog` from `derive` over the `World` with no repository
inputs and reads `attention_count()` (still 1). The model reviewer found every old case present (16 retire
cases, 16 R-23 rows) except "polling: the sum" (the combination of polling holds with requests, flush
failures, pending rows and a failed read; the parts are covered by N5/N6 rows) and the Background/User lane
variants (the model never reads the lane).

**E-4** (readers of the count), all run green: `footer::tests::{needs_attention_counts_and_leads_to_activity (edited), status_items_say_and_choose}`;
`keyboard_tests::shell::{needs_attention_works_by_enter, footer_controls_are_reached_by_tab}`;
`keyboard_tests::journey::arrange_the_workspace_by_keyboard_alone`;
`workspace::tests::saving_drafts::drafts_not_saved_with_activity_hidden_focuses_the_item_area`;
`dock::restore_tests::{a_file_that_cannot_be_read_restores_the_default_and_says_so, a_failed_write_shows_the_save_notice}`.
**Changed counts:** N12 — pending rows with no next action were each counted (old 3), now 0; R-23's cases were ported with rows that wait.
N2 — `needs_attention_counts_and_leads_to_activity` finished two failed requests of one subject and read "2 need attention"; it now reads
"1 needs attention". The plural text is `attention_text`'s own test, so the footer no longer shows "2 need attention".

**Removed assertion (called out):** `counts_what_awaits_the_person` built real `DraftRegistry` failures (two failing sessions, an unavailable store) and read the count from them; the model test now feeds `DraftsInput` (N14 rows). The registry path is still asserted end to end by `workspace::tests::saving_drafts::drafts_not_saved_with_activity_hidden_focuses_the_item_area` (and `workspace_tests.rs:538-583`). No other removal.

**Other changed earlier-task assertions** (each required by a ruling): Task 3's Owed `outcome` expects `None` (C1); Task 4's N25 row group carries `running_operation` (C5); `request_copy_table`/`share_copy_table`/`activity_time_formats`/`brought_in_is_kept_for_the_session` gained fixtures only.

**Fix round 1** (commit `484b0f60`; scoped re-review: all addressed, no Critical or Important breakage): C9; kept L3/L3b rows keep their place (L7);
the scheduler drops a stale `checks_applied` entry when a retry is heard; the duplicate `moved_rows()` call is removed; a test that a running
resume makes a stranded Close row read "Still running…", disabled and uncounted (`desktop::view::operations::resume::activity_tests::a_running_resume_makes_the_stranded_row_still_running`;
it passes first time, fails if both the lock operations and the pending-lifecycle lookup are neutralised; the lookup T4-3 mandates is redundant in this fixture and unguarded alone).
The attention table now has 97 rows (the earlier 52 plus the moved E-3 rows plus the C9 rows, no longer doubled).

**Tests that exist but the builder does not reach until Tasks 6/9:** share offers and words through the Workspace builder, `ShareState`, the open-pane title closure, `DraftsInput.unavailable`/`recovered` from the real registry.

- **Deferred Minors** (carried to the final review): (1) a kept row's insert index (`log.rs:104-106`) is the old index applied to the new vector, so with rows that leave beside it a tie can land after a neighbour; (2) `first_kept` is set only when a failure reaches `remaining`; (3) `follow_void` clears the whole latch, including one N22 set; (4) `settles` and `subjects` are O(unsettled failures × rows); (5) hand-indented blocks in `attention_more.rs` and `log.rs` that rustfmt does not reformat, and a duplicated doc comment at `table.rs:11-13`; (6) the pane-title map `ActivityState.titles` never shrinks and closing an item tab triggers no refresh; (7) `facts_words` restates `share_words`, and `flush_failure_items` duplicates `flush_failures`; (8) `count_and_popover_agree` covers three kinds (N3, N12, N13), not an N14 or hold step; (9) the T11 "outcome changed, offers same" case is untested; (10) `checks_applied`, `checks_heard` and `completed` grow for the session.

## Tests run

| Checkpoint | Tests run |
| --- | --- |
| Task 0 | none (static gates only) |
| Task 1 | `desktop::model::operations` by module: 32 passed, 0 failed (the one module run the plan names). fmt, `cargo check --features desktop` and clippy `-D warnings` clean. |
| Task 2 | By exact path, 1 passed each: `desktop::model::operations::tests::shares_are_classified_once`; `desktop::model::operations::tests::stages_are_shown`; `desktop::view::operations::tests::state_label_shows_stages`; E-1 `desktop::view::operations::outcomes::give_up_tests::a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`; E-2 `desktop::view::operations::check_tests::a_running_check_now_reads_its_stage_words`. fmt, check and clippy clean. |
| Task 3 | By exact path, 1 passed each, red on stubs first: `desktop::model::activity::tests::activity_time_formats`; `...::request_copy_table`; `...::share_copy_table`; `...::each_source_makes_its_rows`. fmt, check and clippy clean. No existing test run or edited. |
| Task 4 | 1 passed each: `desktop::model::polling::tests::brought::brought_in_is_kept_for_the_session` (red on stubs first); `desktop::model::activity::tests::each_source_makes_its_rows` (S1 + S2–S14); `desktop::model::activity::tests::states::copy::state_copy_table`; `desktop::model::repository_state::tests::causes::each_attention_cause_is_set`; Cycle 03a, unchanged and green: `desktop::model::repository_state::tests::{each_state_and_its_precedence, tooltips_say_the_state_and_the_facts, menu_items_follow_the_state, check_all_takes_each_repository_that_can_check}` and `desktop::view::repositories::tests::{rows_and_list_states, titles_of_what_is_not_shared, the_menu_by_pointer_and_keyboard, check_all_checks_each_repository (one flake in a combined run, then green alone twice), the_more_menu_acts_on_the_chosen_repository}`. fmt, check, clippy clean. |
| Task 5 | Each alone, count 1, ok: `desktop::model::activity::tests::logs::attention::attention_counts_each_thing_once` (97 rows) and `...::logs::log::the_log_keeps_times_and_events` (written first, every row red on stubs); `...::s1::a_share_title_that_begins_with_its_subject_is_capitalised`; `desktop::model::polling::tests::brought::completed_attempts_are_counted_per_root` (red first); `desktop::services::scheduler::attention_answer_tests::a_failed_check_now_is_recorded_when_its_answer_applies` (red first, then red for the retry row); `desktop::services::resends::tests::waits_on_says_when_forgetting_would_drop_something`; `desktop::view::activity::tests::count_and_popover_agree`; `desktop::view::activity::tests::refreshes_are_coalesced` (red with coalescing off: 6 derivations vs 4); `desktop::view::operations::resume::activity_tests::a_running_resume_makes_the_stranded_row_still_running`; changed: `each_source_makes_its_rows`, `request_copy_table`, `share_copy_table`, `activity_time_formats`, `brought_in_is_kept_for_the_session`; E-3 `retire_resume_resends_original`; the nine E-4 tests above. fmt, check and clippy clean after each commit. |

## Test edits (E-n)

- E-1 (Task 2): `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`
  (`view/operations/outcomes/give_up_tests.rs:344`) calls `classify_share`
  instead of `share_result`: it re-classifies under another record on purpose.
  No assertion removed.
- E-2 (Task 2): `a_running_check_now_reads_its_stage_words`
  (`view/operations/check_tests.rs`) expects "Running: Saving…" instead of
  `FINISHING` at the Writing stage (F3); the unused `FINISHING` import and the
  module header comment were reworded. No assertion removed.
- E-3 (Task 5): the old count `needs_attention`, its module and three test files deleted; cases moved into the Task 5 tables; `retire_resume_resends_original` reads `attention_count()` from an `ActivityLog` built over `derive`. Removed: the model test's real-`DraftRegistry` assertions (covered by an E-4 test).
- E-4 (Task 5): nine readers of the count, one edit: `needs_attention_counts_and_leads_to_activity` expects "1 needs attention" (N2). The others unedited.
- Candidates E-17 (`dock_tests/toolbars.rs:323`) and E-18 (`keyboard_tests/shell/support.rs:102`) are still unassigned: Task 10 and Task 11.
