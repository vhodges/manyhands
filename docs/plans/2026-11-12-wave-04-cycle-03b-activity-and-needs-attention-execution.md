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
| 5 | done, two reviewers, approved after fix round 1; checkpoint 2 posted | code `110aff6f` (base `3b377441`), fix `484b0f60`; ledger `8830224f` | yes |
| 6 | done, review approved first time, Minors parked | code `35bdd07f` (base `8b960272`); ledger `fe6ced71` | yes |
| 7 | done, review approved first time, Minors parked | code `f6559d70` (base `fe6ced71`); ledger `32eba29f` | yes |
| 8 | done in two parts, review approved after fix round 1; checkpoint 3 posted | part A `1b3081e4` and fix `386b179b`, part B `2eb033d7` (base `32eba29f`); ledger commit follows | yes |
| 9 | done in three milestones, review approved after fix round 1 | M1 `77494d11`, M2 `19c26ca9`, M3 `719b027d`, re-export `e22879bb`, fix `eaa34112` (base `c0734a5a`); ledger commit follows | yes |
| 10 | done, review approved after fix round 1 | `e6a5285a`, `f36dea31`, `7a4159ad`, fix `54976f45`, `437e5cfe` (base `ba39f24f`); ledger `8fcae607`; checkpoint 4 posted `189e077b` | yes |
| 11 | done, two reviewers, approved after fix round 1 | M1 `8bb8c88d`, M2 `e2a75e19`, M3 `413fded7`, fix `baf3b360` (base `189e077b`); ledger `0dccc834` | yes |
| 12 | done, review approved first time, Minors parked | `33be3c66` (base `0dccc834`); ledger `619ffb64` | yes |
| 13 | done: gates, checklist, final review and one fix wave (scoped re-review approved); review-ready comment 5 follows | checklist `daf5bd71`, Acceptance Status `47306e39`, fix wave `f815b793`, `e68c5516`, `2f8a1f31` (base `47306e39`) | yes |

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

## Task 6 (actions and the test seam; E-5 part 1)

`ActivityMenuChoice` (`keys.rs`), `Workspace::choose_activity` (re-checked through
`ActivityLog::still_offered`) and `run_activity_action` (an exhaustive match over the 21
`ActivityAction` kinds) in `view/activity/actions.rs`; handlers gained an `origin: FocusHandle`
parameter or were made `pub(in crate::desktop::view)` (`open_lifecycle`, `review_request`,
`review_pending`, `run_recovery`, `run_share_offer`; reachable: `retire_chosen`, `resume_lifecycle`,
`row_action`, `publication_remote`, `share_unpublished`). The seam is `view/activity_support.rs`
(test only). For now every action passes today's control handle (`control_focus_for`,
`abandon_focus_for`), through `activity_support::origin_of`. TDD: compile-red, then assertion-red
(a no-op `choose_activity` left a Cancel request Queued), then green.

- Ruling T6-1 (binds Task 9): Task 9 replaces `activity_support::origin_of` with the list's
  handle, deletes the focus maps (`control_focus`, `abandon_focus`), deletes the kept helpers
  `operations_tests::recovery_of` and `outcomes_tests::outcome` with the E-7 tests that use them
  (`dismiss_and_retry_from_the_keyboard...`, `a_recovery_dialog_gives_focus_back...`,
  `show_it_focuses_the_entry_of_the_possible_holder`), and reruns E-5 and E-6 by name; it also owns the
  menu's "still marked chosen" step and closing the menu (R5-8). `Share { item: Some }` runs
  `share_unpublished(Intent::ItemSync)`; `Share { item: None }` runs `share_main(root,
  publication_remote(root))`.
- Pruned seven test-only re-exports from `model/operations.rs` (`main_branch_deleted`,
  `CHOOSE_A_KEY`, `CURRENT`, `KEY_LOCKED`, `NEEDS_IDENTITY`, `NOWHERE_TO_SHARE`,
  `check_host_text`): only the converted tests used them; Task 7 may restore one — the converted tests
  now read `SyncOutcome` — none.
- **Parked Minors:** (1) T12's "gone" offer is not exercised through `choose_activity` directly (only
  absence, and a disabled offer, are), so `still_offered`'s action and label clauses have no direct
  test; `act_refused` also passes when the row itself is missing; (2) `RetryFlush` is checked only by
  a `DraftWrite` read count rising (the earlier version, waiting for the failure to clear, timed out;
  the reviewer judged the handler identical to the old control's `retry_draft_flush`, so a fixture
  artefact is likelier; the same pattern is used in `keyboard_tests/drafts/quit.rs:96-101`); (3)
  `check_tests.rs` now calls `progress_words` and `stage_words` directly rather than reading the row's
  description; (4) weaker substitutes: `a_lease_held...` asserts the row kind is not Success only;
  `busy_without_a_known_holder...` asserts no Pending or PendingList row; `a_give_up_left_waiting_on_unlock...`
  dropped `!present(gave-up)`; (5) `outcomes_tests.rs` 525 → 605, `give_up_tests.rs` 548 → 596,
  `operations_tests.rs` 892 → 927, `outcomes_route_tests.rs` 377 → 431 lines (not split); (6) the Share
  and Offer cases assert request counts rather than the sent intent's root, kind and id.

**E-5 part 1: test function lists, before and after** (taken from the implementer's report):

Before and after lists were captured by `fn` name per file. The tests (`#[test]` and `#[gpui_kit::test]` functions) are equal in all six. Helper differences are listed.

- `operations_tests.rs`: tests equal (the six E-5 tests plus the untouched ones). Helpers added: `message`, `pending_commands`, `pending_operations`, `recovery_index`, `recovery_offer`. `recovery_of` kept unchanged (returns the element id) because the E-7 keyboard tests `dismiss_and_retry_from_the_keyboard...` and `a_recovery_dialog_gives_focus_back...` Tab to that control; `recovery_offer` is the label-returning helper the converted test uses.
- `give_up_tests.rs`: tests equal. Helper `words_of` replaced by `outcome_for(code, record) -> SyncOutcome`; helper `code` now reads the request model; `give_up` presses through `act`.
- `outcomes_tests.rs`: tests equal (13). Helpers `offered` and `assert_row` rewritten (`assert_row` now takes a `SyncOutcome`); added `outcome_of`, `code_of`, `conflict_next`. `outcome` (drawn words) kept, only for the E-7 test `show_it_focuses_the_entry_of_the_possible_holder`.
- `outcomes_route_tests.rs`: equal (helpers unchanged). `show_it_focuses...` (E-7) untouched; `a_hostless_comment_stop_opens_the_roots_hosts_page` untouched.
- `check_tests.rs`: equal; helper `stage_of` added.
- `record_tests.rs`: equal.

**Removes (drawn-text assertions removed or turned into model reads), per file:**

- `operations_tests.rs`
  - `launch_refreshes...`: label of `request_id(id,"state")` == "Success" text; `!present(message)`. Replaced by Finished `recovery` empty, row kind Success, not an attention row.
  - `a_failed_refresh_and_rebuild...`: state label (Error text) once; code label twice; message label (kept as the model's `message()` == `code.message()`); `pending-command` labels "refresh"/"rebuild" (now `pending_commands()` from the model); `pending-empty` text (now the pending list is empty); `message` presence twice and `entry` presence twice (now `row(..)` presence); dismiss label (now `offered(.., DISMISS)`).
  - `a_lease_held...`: state label vs outcome label; code label; message label (kept via `message()`); message presence; state != Success label. Now: row stays and its kind is not Success.
  - `a_pending_local_operation...`: labels of `pending-command`, `pending-operation`, `pending-next` (now the pending `OperationDto`), `pending-scope` (now the row's `repo` tag), `pending-empty` text; presence of `pending-resume` / `pending-resume-unavailable` (now `offered("Resume")` enabled).
  - `resume_finishes...` and `an_abandon_by_a_second_actor...`: presence of `pending-row` only (now `row(..)`); nothing else removed.
- `give_up_tests.rs`
  - `giving_up_a_finished_record_says_already_finished`: label of `gave-up` == ALREADY_FINISHED (covered by the existing `gave_up(share) == Some(AlreadyFinished)` assertion and by P5).
  - `a_give_up_left_waiting_on_unlock...`: `!present(gave-up)` (covered by the existing `gave_up(id) == Some(Resent)`).
  - `a_stopped_resume_over_an_unsettled_push...`: `stop` label == STOP/STOPPING (now `offered`), state label == "Blocked" and != CANCELLED (now `Finished(Outcome::Blocked, ..)`), `outcome` label not in the stop words (now `share_result(resume)` is None, since a resume is no share), `!present(stop)`.
  - every `assert_row` word argument (outcome words: KEY_LOCKED, main-branch-deleted text, CHOOSE_A_KEY, check-host text, NEEDS_IDENTITY, STOPPED_BEFORE_SENDING, None) became a `SyncOutcome`.
- `outcomes_tests.rs`: all outcome words in `assert_row` calls (SHARED, CURRENT, STOPPED_BEFORE_SENDING, NEEDS_ATTENTION, KEY_LOCKED x2, CHOOSE_A_KEY x3, check-host text (named and unnamed), NEEDS_IDENTITY, ANOTHER_SHARE_HOLDS, NOWHERE_TO_SHARE, main_branch_deleted("main"), NOTHING_TO_SHARE, None) are now `share_result(id).outcome`; plus the drawn `named` label read "Check git.example.com before sharing" (its content is the model's `NeedsHost { authority }`, the copy is P5's); `code` labels "item_not_found" and "push_rejected" (now Finished code); `outcome(main) == None` for a failed main share (now `Failed { ItemNotFound, retry: true }`); `present(dismiss)`, `present(recovery_id(id,0))`, `present(Share)` (now `offered`/row menu).
- `outcomes_route_tests.rs`: `outcome(..) == ANOTHER_SHARE_HOLDS` in `busy_without_a_known_holder...` (now `SyncOutcome::Busy` and `busy_holder == None`); `label("pending-empty")` (now no Pending/PendingList row); `present(Retry)` (now `offered`). Presses by element id became `act`.
- `check_tests.rs`: the drawn state label "Checking · {text}" (7 stages) and "Running: Saving…" (3 verbs). Now the stage is read from `request(..).state` and the words from the model's `progress_words(Progressing::Check, Some(stage), remote)` (remote read from navigation) and `stage_words(Writing, None)`, with the same literals.
- `record_tests.rs`: label of `(NOTE,0)` == `record_note_words(Unreadable)` and the non-empty copy assertion; `present((NOTE,1))`. Now the `RowKey::Note(Unreadable)` row exists, appears once, and `act(.., "Dismiss")` removes it.

Keyboard and focus steps were left alone (`Region::Operations.ring_id()` assertions in the pending abandon test and the busy test stay; they stay green because the seam passes today's control handle).

## Task 7 (the rest of `D/view/operations/` onto the seam; E-5 part 2)

Seven test files converted (`attempts_tests.rs`, `attention_tests.rs`, `republish_follow_tests.rs`,
`republish_stop_tests.rs`, `republish_tests.rs`, `resume_tests.rs`, `retire_tests.rs`); no production file
changed. The old attempt entry is now Problem and Unshared rows (design S4–S6, R-17), so
`dispositions_listed` pins exact `RowKey` lists and Share only on the Unpublished row and none on the
Problem rows; `retiring_tests.rs` was out of the list (ruling P-F11).

- Ruling T7-1: seam additions `act_unsettled`, `share_outcome`, `label_of` (test-only,
  `activity_support.rs`, 249 → 296 lines). `act_unsettled` skips the settle after the choice so
  `attention_recovery_held` keeps its "library call not inside the handler" assertion — plain `act`
  would hide it — the exactly-one-enabled-offer rule is shared with `act` — none.
- E-17: `resume_activity_tests.rs` `a_running_resume_makes_the_stranded_row_still_running` (Task 5's fix
  round test): `close_row` now takes the root and returns a `RowKey`, so its one press became `act`; no
  assertion removed.
- **Parked Minors:** (1) `dispositions_listed` no longer pins which rows offer Share beyond
  Problem rows and the Unpublished row: an enabled Share on the Diverged and the quiet LocalAhead
  Unshared row would pin decision 17 and R-17; (2) its message "twelve lines, in the poll's order" is
  inaccurate (problems in poll order, then the two Unshared); (3) `resume_tests.rs` "the row is gone"
  checks `row(&entry).is_none()` for that pending key; confirm `RowKey::Pending` keys the operation,
  not a per-render value; (4) `attempt_entry_share_waits_for_the_lock`: `!tab_reaches(action, 20)` could
  pass vacuously if the drawn element disappeared; add `assert!(rig.shell.present(action.clone(), cx))`
  before it, until E-7 removes the step; (5) `republish_confirms_then_shares` no longer touches the row
  itself (it reads the request model only); (6) the `until_not_offered` doc comment says "enabled" but it
  waits for no such offer at all.

**Test seam additions:**

- `act_unsettled`: `act` without the final `shell.run`; `act` and `act_unsettled` share a private
  `press`, and `choose` takes a `settle` flag. Needed by `attention_recovery_held`, whose "not
  inside the click" assertion (the library call is made on a background task, not in the
  handler) cannot be seen after `act`'s run.
- `share_outcome(shell, id)`: the finished share's `SyncOutcome` (panics if none).
- `label_of(shell, key, &ActivityAction)`: the label of the offer that runs an action, after a
  refresh (for Recovery offers, whose labels come from the library).

**Test-function lists, before and after:**

- `attempts_tests.rs`: tests equal (4). Helpers: added `brought`, `entry`, `keys`, `problem`,
  `unshared`; removed `line`.
- `attention_tests.rs`: tests equal (6). Helpers: added `hold`, `holds`, `offers`; removed `names`;
  consts `ENTRY` and `ROOTS` removed (unused); `TestWindowExt` import dropped (unused).
- `republish_follow_tests.rs`: tests equal (7). Helper added: `press_recovery`.
- `republish_stop_tests.rs`: tests equal (7). Helpers unchanged (`under_a_resume` body edited).
- `republish_tests.rs`: tests equal (8). Helpers added: `until_offered`, `until_not_offered`.
- `resume_tests.rs`: tests equal (7). Helpers added: `listed_close`, `offer`; `close_row` kept by
  name but now `close_row(shell, root, cx) -> RowKey` (was `(shell, cx) -> usize`).
- `retire_tests.rs`: tests equal (7). Helper added: `retire_key`.

**Removes (drawn-text assertions) and what replaced them, per file:**

- `attempts_tests.rs`
  - `dispositions_listed`:
    - `present(attempt-..-repository)` (entry exists/absent) is now the rows of Problem or
      Unshared keys for the root (`entry`).
    - `line-i` label == `PollDisposition::X.guidance()` is now the Problem row's `detail[0]`
      ends with the same `guidance()` for the ten problem dispositions, with kind Failure.
    - The `line-10` label == UNPUBLISHED text and the Diverged line's text are removed; the
      model copy (`unshared_detail`) is P-test territory. Their rows are asserted to exist, in
      order.
    - `action-i` labels are now `offered(.., SHARE)`: none on a problem row, enabled on the
      Unpublished row.
    - `!present(line-12)` is now the entry's key list being exactly the twelve expected keys.
    - `label(brought-in) == None` is now zero Brought rows; `"Brought in 1 change"` is now one
      Brought row.
    - `!present(line-1)` ("replaced") is now the key list being exactly `[Problem(new id)]`.
  - `attempt_entry_share_waits_for_the_lock`:
    - Share label == SHARE is now `offered(..).enabled`.
    - The drawn "Share… plus lock words" label is now the offer present and disabled, and the
      row's `detail` contains `lock_words(Ticket)`.
    - Click on the waiting Share… sending nothing is now `act_refused` (which asserts the row
      is there, no enabled offer, and an unchanged footprint) plus the old `sent().is_empty()`.
    - After unlock: offered enabled, then `act`. The drawn `tab_reaches(.., "not focusable")`
      check is kept (a keyboard step).
- `attention_tests.rs`
  - `attention_locked_key`: `label(ENTRY) == headline(KeyLocked)` is now the Hold row's kind
    `Locked` (headline copy is P-tested); `label(ROOTS) == "Holding back a, b"` is now
    `row.group.roots == roots`; `label(ACTION) == UNLOCK` is now `offers(.., UNLOCK)` enabled;
    `!present(ENTRY 1)` is now exactly one Hold row.
  - `attention_host`: label APPROVE_HOST is now `offers`; I added "not Unlock…" as
    `offered(UNLOCK).is_none()` (the old doc said it; nothing asserted it); click is now `act`;
    `!present(ENTRY 0)` is now zero Hold rows and the host's row is None.
  - `attention_actions_by_reason`: labels CHOOSE_A_KEY and OPEN_REMOTE_SETTINGS are now `offers`
    on each reason's Hold row; clicks are now `act` on the row chosen by reason.
  - `attention_recovery_held`: `label(ENTRY) == "Checking is held until recovery"` is removed
    (copy is P-tested); label RESUME_CHECKING is now `offers`; the click is now `act_unsettled`
    and the "not inside the choice" assertion is kept.
- `republish_tests.rs`
  - `a_deleted_branch_offers_republish`: outcome label == BRANCH_DELETED is now
    `share_outcome == SyncOutcome::BranchDeleted`; the "later release" assertion on the const
    stays; Republish… label is now `offered(..).enabled`; the stop-and-republish and share
    presence checks are now `offered(..).is_none()` (the row exists, the Republish… offer
    proved it).
  - `republish_confirms_then_shares`: drawn "Done" state and "Republish" command labels are
    removed. The `Finished(Success, Ok)` assertion that was already there carries "Done"; the
    command word is now `done[0].intent.label() == "Republish"`.
  - `a_stopped_republish_sends_the_same_request`: waits and presses the Recovery(id, 0) offer
    (`label_of` then `act`) instead of the element.
  - `an_item_shares_stop_and_republish_opens_republish`, `republish_waits_for_another_dialog_to_close`,
    `republish_opens_when_the_dialog_it_waits_for_is_cancelled`,
    `an_unreadable_record_offers_give_up_and_it_sends_the_give_up`: presence and label reads of
    the row's controls are now `until_offered` / `offered`; presses are `act`. In the last,
    Give up is waited for, then Republish… and Stop and republish… are `is_none()` with the row
    existing.
- `republish_follow_tests.rs`
  - `a_share_offers_no_republish_while_one_runs`: `label(request_id(id,"republish")) == REPUBLISH`
    is now `offered(..).enabled`. `!present(republish)` is now `until_not_offered` (the row must
    still exist). `label(outcome) == REPUBLISHING` is now `offered(REPUBLISHING)` that is disabled
    and whose action is `Offer(id, Republish)`. "Republish… again" is now `until_offered`.
    `item-pill` label reads stay (item pane, not Activity).
  - `a_waiting_republish_hides_republish_elsewhere`: presence of dismiss, republish and
    stop-and-republish on the share are now `offered` (Dismiss enabled, the other two `None`).
    `lifecycle-republish-stopped` and dialog reads stay.
  - Presses by id (recovery 0, dismiss, open) are now `act` through `press_recovery`, `DISMISS`
    and `OPEN_LIFECYCLE`.
  - `a_removed_republish_offers_republish_again` needed no edit: its controls are the lifecycle
    dialog's (`lifecycle-republish`), not rows.
- `republish_stop_tests.rs`: `under_a_resume` and `a_stop_that_cannot_be_queued_says_so` press
  Open… with `act(OPEN_LIFECYCLE)`. No assertion removed. Dialog labels (`lifecycle-stop-refused`,
  `lifecycle-code`) stay.
- `resume_tests.rs`
  - `close_row` waits until the close is listed in `Operations::pending_shown` and returns
    `pending_key(.., "close")`.
  - `a_lifecycle_row_offers_resume_after_restart`:
    - `label(pending-next) == "resume"` is now the pending `OperationDto.next_action`.
    - `label(pending-resume) == RESUME` is now `offered(RESUME)`, enabled, with action
      `ResumeLifecycle`.
    - `!present(pending-resume-unavailable)` and `!present(pending-still-running)` are now
      `offered(STILL_RUNNING).is_none()` and "every offer on the row is enabled".
  - `resume_is_not_offered_while_in_flight_and_busy_reads_still_running` and
    `a_running_resume_counts_as_in_flight`: `present`/`label(pending-still-running)` is now
    `offered(STILL_RUNNING)` that is disabled; `!present(pending-resume)` is now
    `offered(RESUME).is_none()` with the row existing.
  - `resume_resends_the_named_request_and_opens_running`: "the row is gone" (`pending-command`
    rows lack "close") is now `row(key).is_none()`.
  - All Resume presses are `act(.., RESUME)`.
- `retire_tests.rs`: presses of `attempt-..-retire-..` in the five tests are `act(RETIRE)` on
  `RowKey::RetireOffer`. No drawn-text assertion removed; dialog reads
  (`retire-unkept-message` and the like) stay, since those dialogs are not Activity.

## Task 8 (the other files onto the seam; E-6)

- Ruling T8-1: Task 8 was run as two commits, part A (`D/view/conflict/*`, `confirm_tests.rs`,
  `find_tests.rs`, `tabs_tests.rs`, `D/view/item/*`) and part B (`D/view/lifecycle/*`,
  `lifecycle_tests.rs`, `dock_tests/state.rs`, `drafts_entry_tests.rs`, `D/services/*`,
  `D/view/settings/*`), reviewed by two fresh reviewers — E-6 spans ~25 files and one pass would not
  have been reviewable — the plan says one commit per task — none.
- Ruling T8-2: a test E-6 names whose body presses or reads nothing in Activity needs no
  conversion and stays unchanged. Applied to `item/posting_tests/review.rs`
  `an_operation_area_retry_is_followed_and_posts_one_comment` (it calls `worker.retry` directly, and
  decision 26 hides Retry for that unsaved post), the two `a17_` tests of `privacy_consent_tests.rs`
  and the two tests of `drafts_entry_tests.rs` that read no Activity — E-6's counts ("two a17_ tests",
  "its two tests") name the ones that do — none.
- Ruling T8-3: `services/drafts_tests.rs::a_failed_flush_shows_an_entry_with_retry_and_no_draft_text`
  had a drawn whole-window scan for "Recovery copy of" that E-11 does not name; it was converted to
  model row reads, and the drawn SENTINEL-absence scan over all window labels is kept, guarded by
  `Region::Operations.id()` (also `remotes_tests.rs`), plus a model scan of every row's title,
  description and detail. **Task 9 must keep or update the `Region::Operations.id()` guards** — the
  sweep would pass vacuously once the area is renamed.
- Fix round 1 (part A, `386b179b`, scoped re-review: addressed, no breakage): `no_stop` in
  `item/share_tests.rs` now rejects any offer labelled STOP or STOPPING (enabled or not) across the
  menu and button; a new `no_stop_action` rejects any `ActivityAction::Stop` once the share has ended.
  The post-without-a-remote site checks labels only, since a running accepted post with no safe point
  legitimately offers a disabled "Finishing" carrying a Stop action (the old drawn check allowed it).
- Seam additions (test-only): `until_offered` (part A), `until_pending(shell, root, action, cx) ->
  RowKey` (part B; `conflict/entry_tests.rs::row_of` is a one-line wrapper over it). The duplicate
  `republish_tests::until_offered`/`until_not_offered` (Task 7) is not folded.
- **Accepted loss to look at in the review:** the disabled Retry's reason text "Not sent again after
  a later Edit or release" (`NOT_SENT_AGAIN`, `model/operations.rs:73`, used only by the old view,
  `view/operations.rs:546-547`) is no longer shown or asserted anywhere once Task 9 removes the old
  view — the model's `Offer` carries no reason and the design has no such copy. The behaviour
  (disabled Retry, `attempt == 1`, `operations.not_sent_again`) is still tested. If the new Activity
  should explain a disabled Retry, that is a design gap.
- **Parked Minors:** (A) the row title's stage suffix is not pinned for a post-kind request or for
  share stages other than Fetching (`each_stage_shows_its_text_for_share_and_post`); the model's
  `stages_are_shown` pins `stage_words` for all stages; (A) `a_stopped_share_says_what_completed...` builds
  its expected label from `offer_label(SyncOffer::Share)` (saved by the next `act` on the literal
  label); (A) duplicate `until_offered`; (B) `drafts_tests.rs:239-250`'s kept drawn scan is guarded only
  by `Region::Operations` being drawn (the old guard proved the entry's own text was among the labels
  swept; Task 9 should guard on a label containing the row's title); (B) `outcome_tests.rs:1154-1164`: the
  removed command label ("Close ticket") and item short id have no model replacement (assert
  `running.row.item` is the ticket and the intent is `TicketClose`); (B) `dock_tests/state.rs:466-499`:
  the drawn "hidden, so not drawn" check after the finish is now only the layout flag; add
  `assert!(!shell.present(Region::Operations.id(), cx))`, and the "row equal to its hidden-time self"
  check pins that derivation is stable, not that Show leaves the log intact (both reads follow a forced
  `refresh_activity_now`); it rests on the log keeping the row's revision while nothing changes — loosen
  to key and kind if a later task moves the revision.

**Part A: test-function lists, before and after:**

| file | tests | helpers |
| --- | --- | --- |
| `conflict/entry_tests.rs` | equal (7) | `row_of` kept by name, now `row_of(shell, root, command, cx) -> RowKey` (was `(shell, command, cx) -> usize`); waits on `Operations::pending_shown` and returns `pending_key` |
| `conflict/repair_tests.rs` | equal (5) | none |
| `confirm_tests.rs` | equal (10) | added `abandon(shell, root, cx)` = `act(pending_key(.., "save_ticket"), ABANDON)`; `stranded` unchanged (the E-7 escape test still uses `("pending-abandon", 0)`) |
| `find_tests.rs` | equal (14) | none |
| `tabs_tests.rs` | equal (39) | none |
| `item/share_tests.rs` | equal (16) | added `entry`, `no_stop`, `says_shared`, `assert_staged` |
| `item/edit_release_tests.rs` | equal (30) | `retry_control` kept by name; now returns the log's `Offer` (was the element id `String`); const `NOT_SENT_AGAIN` removed |
| `posting_tests/outside.rs` | equal (5) | `recovery_labels` kept by name; reads the row's offers |
| `posting_tests/decisions.rs` | equal (3) | none |
| `posting_tests/extras.rs` | equal (10) | none |
| `posting_tests/recovery.rs` | equal (10) | none |
| `posting_tests/review.rs` | unchanged (see concern) | none |

**Part A: Removes (drawn-text assertions) and what replaced them, per file:**

Presses by id became `act(.., LABEL)` with the label constant; presence checks became `offered` or
`row`; every absence check is made on a row that is proved listed. Only differences from a plain
mapping are listed.

- `conflict/entry_tests.rs`
  - `the_banner_offers_review_changes`: `label(request_id(share,"review")) == REVIEW_CHANGES` is now
    `offered(Request(share), REVIEW_CHANGES)` with action `Review(share)` (after `until_offered`).
    `present(entry)` is `row(..).is_some()`; `!present(review)` on main is `offered(..).is_none()`
    (row listed). Banner reads (`item-conflict`, `item-review-changes`) stay: item pane, not Activity.
  - `a_pending_resolve_row_offers_review_changes_after_restart`: `label(pending-review) ==
    REVIEW_CHANGES` is now `offered(.., REVIEW_CHANGES)` with action `ReviewPending{..}`. No text removed.
  - `a_local_synchronization_offers_resume`: `present(pending-resume)` and `!present(pending-resume-unavailable)`
    are now `offered(RESUME)` enabled with action `Resume{..}` (disabled is the "unavailable" case).
  - `resolve_conflict_opens_the_childs_sheet`: review label is `offered` with action `Review(close)`;
    `click(open)` is `act(OPEN_LIFECYCLE)`.
  - `an_applied_resolution_clears_the_banner_and_its_review` and
    `a_resume_that_settles_the_conflict_clears_its_review`: `!present(request_id(share,"review"))` is
    `row(entry).is_some()` and `offered(.., REVIEW_CHANGES).is_none()`; the earlier presence is
    `until_offered`/`offered(..).enabled`; Dismiss is `act(DISMISS)`.
- `conflict/repair_tests.rs` (three tests: `repair_locally_syncs...`, `a_refused_local_sync...`,
  `repair_locally_waits_for_a_queued_sync`): same mappings; the close's row `!present(pending-review)`
  is `row(close).is_some()` plus `offered(REVIEW_CHANGES).is_none()`. No text removed.
- `confirm_tests.rs`, `find_tests.rs`, `tabs_tests.rs`: only the press of `("pending-abandon", 0)` became
  `act(pending_key(.., "save_ticket"), ABANDON)` (so the row is looked up by key, not index). Dialog
  assertions untouched. No text removed.
- `item/share_tests.rs` (the nine tests that read or press the entry):
  - `each_stage_shows_its_text_for_share_and_post`: REMOVED the drawn `label(request_id(id,"state")) ==
    "Sharing · {stage text}"` / `"Posting · ..."` for each stage. Replaced by `assert_staged`: the request's
    state is `Running { stage: Some(stage) }`, and the log entry is a `Kind::Working` row whose title opens
    with "Sharing" / "Posting". The model row's title is "Sharing “Shared”… · Connecting…" (a different
    copy from the old drawn "Sharing · Connecting to origin…"), so equating the words is not possible;
    the entry's stage copy is P4-P7's (`stages_are_shown`). The pill's `"Sharing · {text}"` assertion
    stays (item pane).
  - `a_post_without_a_remote_never_says_sharing`: the drawn state label `starts_with("Posting · ")` and
    `!contains("Sharing")` is now the entry's title `starts_with("Posting")` and `!contains("Sharing")`;
    `!present(stop)` is `no_stop` (no enabled `Stop` offer on a listed entry).
  - `an_accepted_share_offers_stop_and_stopping_signals_it`: STOP label is `offered(STOP) ==
    Offer(Stop(id), STOP, true)`; `!present(cancel)` is `offered(CANCEL).is_none()`; STOPPING label is
    `offered(STOPPING) == Offer(Stop(id), STOPPING, false)` (and no STOP); `!present(stop)` is `no_stop`.
    REMOVED the code label `"cancelled"` (now `Finished(_, ResultCode::Cancelled, ..)`) and the
    `STOPPED_BEFORE_SENDING` outcome words (now `share_outcome == Stopped { completed: false }`).
  - `a_stopped_share_says_what_completed_and_offers_share`: REMOVED `STOPPED_BEFORE_SENDING` and
    `STOPPED_AFTER_PART` outcome words, now `share_outcome == Stopped { completed }` (false, then true);
    Share label is `offered(SHARE) == Offer(Offer(id, SyncOffer::Share), ..., true)`; the press is `act(SHARE)`.
  - `the_conflict_banner_says_work_is_safe_and_offers_review_changes`: Dismiss is `act(DISMISS)`. No text
    removed.
  - `a_finished_share_says_its_outcome_in_the_pill`, `a_kept_draft_after_a_shared_outcome_reads_draft_kept`,
    `a_shared_outcome_leaves_the_pill_once_the_item_reads_otherwise`,
    `sharing_private_work_shows_shared_until_the_item_reads_differently`: REMOVED the entry's drawn
    outcome label `== SHARED`; now `says_shared`: the entry is listed and `share_outcome ==
    SyncOutcome::Published`. Pill reads stay.
- `item/edit_release_tests.rs` (`retry_control` and its three tests): REMOVED the drawn disabled-Retry
  label `"Retry (Not sent again after a later Edit or release)"` (`NOT_SENT_AGAIN`), asserted three
  times, and the click on the unavailable text. Now `retry_control` is `("Retry", enabled)` on the
  entry's `Recovery(id, n)` offer: `("Retry", true)` when offered; `("Retry", false)` after the later
  prepare/release (listed, disabled), and `act_refused(.., "Retry")` (which asserts no enabled offer and
  an unchanged footprint). `attempt == 1` and the worker's refusal are kept. The "not sent again"
  reason text exists only in the old view (`view/operations.rs:546`); the model's rows and the design do
  not carry it, so nothing replaces the words.
- `posting_tests/outside.rs` (`recovery_labels` and three tests): `recovery_labels` now lists every offer
  label on the post's entry (asserting the entry is listed and offers Dismiss, so an empty list cannot
  mean "no entry"); "no Retry/Resume/Prepare" and "Retry sharing, not Retry" assertions are unchanged
  in meaning. No text removed.
- `posting_tests/decisions.rs`: Dismiss is `act(DISMISS)`. No text removed.
- `posting_tests/extras.rs` `quitting_during_an_accepted_post_leaves_it_saved_and_cancelled`:
  `!present(("pending-resume", 0))` is now: the post's entry is listed and NO row in the log offers
  RESUME. No text removed.
- `posting_tests/recovery.rs` `p3_a_stopped_post_offers_share_not_retry`: `click(stop)` is `offered(STOP)`
  enabled then `act(STOP)`. No text removed (the chip reads are the item pane's).

**Part B: test-function lists, before and after:**

| file | tests | helpers |
| --- | --- | --- |
| `lifecycle/outcome_tests.rs` | equal (8) | equal (`at_stage`'s inner `use RequestState` dropped, the module imports it now) |
| `lifecycle/outcome_canned_tests.rs` | equal (5) | equal; `reopen(shell, id, cx)` keeps its signature, now presses with `act(.., Open…)` |
| `lifecycle/interruption_tests.rs` | equal (4) | equal |
| `lifecycle/made_ready_tests.rs` | equal (6) | `entry_texts` kept by name and signature; now the log row's title, description, repo tag and detail lines (was the drawn snapshot labels under the entry's id) |
| `lifecycle/privacy_consent_tests.rs` | equal (4) | equal |
| `lifecycle/privacy_promote_tests.rs` | equal (1) | equal |
| `lifecycle_tests.rs` | equal (8) | equal |
| `dock_tests/state.rs` | equal (3) | equal |
| `drafts_entry_tests.rs` | equal (4) | equal |
| `services/drafts_tests.rs` | equal (14) | equal |
| `services/drafts/save_tests.rs` | equal (25) | equal |
| `settings/keys_tests.rs` | equal (18) | equal |
| `settings/remotes_tests.rs` | equal (18) | equal (local `row` (remote name -> index) is unrelated to the seam's, which is imported `as log_row`) |
| `settings/repositories_tests.rs` | equal (17) | equal |
| `settings/repositories_manage_tests.rs` | equal (20) | equal |
| `conflict/entry_tests.rs` (Part A file, `row_of` only) | equal (7) | `row_of` body folded into `until_pending` |
| `activity_support.rs` | n/a | + `until_pending` |

**Part B: Removes (drawn-text assertions) and what replaced them, per file:**

Presses by id became `act(.., LABEL)`; presence checks became `offered` / `row(..).is_some()`; every absence check is made on a row proved listed in the same test.

- `lifecycle/outcome_tests.rs` `the_operation_entry_reopens_the_dialog`:
  - REMOVED drawn entry words: command `"Close ticket"`, item `short_id(TICKET)`, state `"Closing · Merging into main…"`.
    Replaced by: the entry row is listed with `Kind::Working`, and the request's state is
    `Running { stage: Some(Integrating), accepted: true }`. The stage copy is P4-P7's.
  - `!present(open)` "not while running" is `offered(.., OPEN_LIFECYCLE).is_none()` on the listed running row.
  - After the run: the `open` label read is `offered(..).action == Open(id)`; "no raw recovery action 0..4" is: no offer on the
    (listed) row has a `Recovery(..)` action; `click(open)` is `act(.., OPEN_LIFECYCLE)`.
- `lifecycle/outcome_canned_tests.rs`:
  - `reopen`: `click(request_id(id,"open"))` is `act(shell, &RowKey::Request(id), OPEN_LIFECYCLE, cx)` (so
    `each_way_out_offers_its_button` and `busy_after_the_merge_is_failed_not_still_running` change only through it; no text removed).
  - `a_lifecycle_entry_keeps_the_recovery_its_dialog_does_not_own`: `present(open)` is Open… offered and enabled; `present(recovery 0)`
    ("Set identity") is an offer with action `Recovery(id, 0)`; `!present(recovery 1)` / `!present(recovery 2)` are no offer with
    `Recovery(id,1)` / `Recovery(id,2)`; all on the proved-listed row. Labels are not asserted (they were not before).
- `lifecycle/interruption_tests.rs`:
  - `p5_a_close_stranded_at_closing_resumes_after_relaunch`: pending row by drawn `pending-command` position is
    `until_pending(.., "close")` (the pending key); the `RESUME` label read is `offered(..)` enabled with action
    `ResumeLifecycle { .. }` (S9); `click(pending-resume)` is `act(.., "Resume")`. No text removed.
  - `a_close_lost_before_settlement_is_complete_and_stays_accepted`: `!rows("pending-command").contains(close)` is: the repository's
    pending list is `Ready` (read as the model's `pending()` rows, which must exist) with no `close` operation, and Activity lists no
    `RowKey::Pending` row of that repository. (The old test never asserted the list was non-empty; the Ready wait is kept.)
- `lifecycle/made_ready_tests.rs` `done_lists_made_ready_with_slug_and_title` (via `entry_texts`): the drawn snapshot scan of the
  entry (no `DEPENDENT_TITLE`, non-empty) is the same two assertions over the log row's title, description, repo tag and detail.
  No assertion removed; the source of the texts is now the model row.
- `lifecycle/privacy_consent_tests.rs`: `a17_a_stranded_close_...`: the drawn wait for the close's `pending-command` row is
  `until_pending`. `a17_a_conflict_waiting_...`: row + `present(pending-resume)` wait is `until_pending` then `until_offered(RESUME)`;
  "still listed" is `row(..).is_some()`; "still offered" is `offered(..)` enabled. No text removed.
- `lifecycle/privacy_promote_tests.rs` `a16_...`: `click(request_id(retried,"open"))` is `act(.., OPEN_LIFECYCLE)`. D5's `Seen`
  sweeps (`seen.window`, `holds_none`) still read the drawn area, untouched (Task 9). No text removed.
- `lifecycle_tests.rs` `a_blocked_prepare_lists_blockers_and_way_out`: REMOVED drawn command label `"Share trunk"` on the share's
  entry (`request_id(share,"command")`). Replaced by: the share's entry row is listed and its first detail line (the model's
  `"{intent label} · {scope}"`) starts with `"Share trunk"`, so the R-35b fact (the entry names trunk) is still asserted, from the model.
- `dock_tests/state.rs` `a_request_finishes_while_activity_is_hidden`:
  - `present(entry)` while running is the entry row listed with `Kind::Working`.
  - `!present(entry)` "hidden, so not drawn" is REMOVED as an id read (it would pass vacuously once Task 9 renames the id). Replaced
    by `!layout.is_displayed(Activity)` (the model's hidden state) AND the row still listed (the P28 carried check "nothing pruned
    while hidden"), with `Kind::Success` and the request `Finished(..)`.
  - After Show: `present(entry)` and the drawn state label non-empty are REMOVED. Replaced by `is_displayed(Activity)`, the
    `Region::Operations` panel drawn, and the row equal to its hidden-time self (same row, same revision).
  - `show_it_reveals_a_hidden_activity` (E-7, same file) untouched and not run (shares no changed helper).
- `drafts_entry_tests.rs`:
  - `the_launch_offers_its_recovered_drafts_and_restores_nothing`: drawn label `recovered-drafts == "Recovered drafts: 2"` REMOVED
    (the `recovered_drafts(2)` string assert on the model fn stays); replaced by `RowKey::Recovered` listed with title
    `recovered_title(2)`, Review offered enabled with action `ReviewDrafts`. `click(review)` / `click(dismiss)` are
    `act(REVIEW)` / `act(DISMISS)`. `!present("recovered-drafts")` after dismiss, and again after `reload_listing` ("for the session"),
    are `row(Recovered).is_none()` (the row was proved listed earlier in the test).
  - `an_operation_entry_names_the_item_never_the_draft`: drawn labels `draft_flush_failed("One"|TWO, Access)` REMOVED; replaced by,
    for each of the two sessions, the `RowKey::Flush(id)` row: `Kind::Failure`, title
    `flush_title(item_name(Some("One") | None, Ticket, id))`, description `DraftError{Access}`, Retry offered; plus no row's title,
    description or detail contains the SENTINEL. The `"operation-area"` scan is left (E-11).
- `services/drafts_tests.rs`:
  - `a_store_that_cannot_be_protected_still_opens_the_window`: drawn `drafts-unavailable == "{DRAFTS_UNAVAILABLE}: The drafts folder
    could not be protected."` REMOVED; replaced by the `DraftStore` row: `Kind::Failure`, title `DRAFTS_UNAVAILABLE`, description
    `"The drafts folder could not be protected."` (the fixed reason is still pinned). The `session-section` label scan ("entry drawn,
    nothing reads Dismiss") is REMOVED; replaced by `offered(DraftStore, DISMISS).is_none()` on the listed row.
  - `a_failed_flush_shows_an_entry_with_retry_and_no_draft_text`: drawn entry text `"Recovery copy of Item title not saved: Access to the
    drafts folder was denied."` (and its literal pin) REMOVED; replaced by the `Flush(id)` row: `Kind::Failure`, title
    `flush_title(item_name(Some("Item title"), kind, id))`, description `"Access to the drafts folder was denied."`, Retry offered
    enabled. The drawn scan "some label starts with `Recovery copy of`" REMOVED (replaced by the row reads above); the SENTINEL scans:
    all rows' texts (model) and all drawn labels (kept, with the Activity panel asserted drawn). Retry is `act(RETRY)` and the
    entry gone is `row(Flush).is_none()` (listed before).
- `services/drafts/save_tests.rs`: `click(request_id(request,"dismiss"))` is `act(Request(request), DISMISS)`. No text removed.
- `settings/keys_tests.rs`: `click(recovery_id(id, retry))` is `act` with the label read from the row's offer
  (`label_of(Recovery(id, retry))`). No text removed.
- `settings/remotes_tests.rs`:
  - `an_added_url_with_a_password_is_kept_out_of_everything_shown`: drawn command label `== Intent::RemoteAdd.label()` REMOVED; replaced
    by the request's intent is `RemoteAdd` and the entry row is listed; its title, description and detail hold no SENTINEL (new, model);
    the drawn `window_text` SENTINEL scan stays, with `Region::Operations` asserted drawn.
  - `selecting_a_remote_that_is_not_eligible_shows_its_typed_failure`: drawn `request_id(id,"code") == code` is the row's detail has
    a line starting `"invalid_remote: "` (the model's `"{code}: {message}"` line). Not a removal of the fact.
  - `a_partial_add_is_completed_by_retry_under_the_same_request`: drawn code and effects labels are the row's detail lines
    `"{code}: .."` and `"Effects: {effects}"` (effects text from the model's `copy::effects_text`); `present(recovery)` is the retry
    offer enabled (label from `label_of`); `click` is `act`.
- `settings/repositories_tests.rs` `a_partial_create_is_completed_by_retry_under_the_same_request`: the literal
  `format!("request-{}-code")` read (and `recovery_id`) goes: the row's detail has a `"{code}: .."` line, retry offered enabled,
  `act` for the press.
- `settings/repositories_manage_tests.rs` `a_release_that_failed_during_removal_...`: `present(retry)` is the `Flush(draft)` row listed
  and Retry offered enabled; `click` is `act(RETRY)`; `!present(entry)` is `row(Flush).is_none()` after it was proved listed.

Sentences no longer asserted anywhere (drawn-only copy; model tests P4-P7 own the new copy): `"Closing · Merging into main…"`,
`"Recovery copy of {title} not saved: {reason}"` as one string (title and reason are asserted separately, in the S11 form),
`"Recovered drafts: 2"` as a drawn label (the model fn string is still asserted), `"{DRAFTS_UNAVAILABLE}: {reason}"` as one string
(the two parts asserted separately), the drawn `"Share trunk"` command label (the model's detail line is asserted instead).

## Task 9 (the swap: the Activity list, the row menu, the sweeps; E-7, E-8, E-9, E-11, E-12, E-16)

- Run as three milestone commits (ruling T9-1: the task is ~70 files; each milestone left the tree compiling, and the E-7 tests that needed the row menu failed at M1 and passed from M2 — recorded in the report; cost if wrong: M1 alone is not a green tree): M1 `77494d11` (pane, rows, keys, tooltip, F4 removal), M2 `19c26ca9` (row menu, focus wiring, accessor), M3 `719b027d` (sweeps, reruns), then `e22879bb` (the parked Task 3-8 Minor: `view/lifecycle/outcome.rs` re-exports the model's `STOP`, `STOPPING`, `UNLOCK`, `OPEN_SHARING_AND_KEYS`, `CHECK_HOST`). Two reviewers (M1; M2+M3 and the re-export), then a fix round and a scoped re-review. Base `c0734a5a`.
- U-2 and U-4 both passed in a scratch test (deleted): no `VirtualList` amendment. The list is `uniform_list` plus `track_scroll`; `"operation-area"` is the list container's id.
- Review: M2+M3 approved, no Critical or Important. M1 found two Important, fixed in round 1 (`eaa34112`, scoped re-review: both addressed, no new breakage):
  - T8 checks (P15, P16, P17) could not tell the drawn revision from the log's current one: `change_row` removed the Resume offer, so `still_offered` refused on the missing offer. Now an A-B-A in one update (offer goes, comes back, the revision bumps, offers equal what the pane drew), plus a stale Enter on a button-less row (`is_current`, previously untested). Mutations seen failing: the `row.revision == revision` comparison removed; the revision taken from the log's row; `is_current` dropped.
  - `retire_left_offer` was weaker than `!present(action)`: now `row(..).is_none()` for done, pending, running; for the marked item the row exists with Retire… disabled.
  - Also: unused `window` parameter removed from `action_button`/`element`; stale comments; two E-8 removals added to the Removes list.
- Ruling T9-2: the highlight starts as None; `focus_region(Operations)` focuses the list and starts it on the first row; a Tab into the list does not start it (Enter and shift-f10 do nothing until an arrow). The design is silent on the start. Cost if wrong: one Task 10 change (start the highlight when the list gains focus by any route). **Task 10's P15 Tab clause must settle it.**
- Ruling T9-3: P15/P16 call `cx.executor().allow_parking()` (a real worker job; flaked once under load without it; same call as `ticket_shell`).
- Size: `view/operations.rs` 954 → 154 lines (A13: below 500). `view/workspace/requests.rs` 636 → 611 (not split). New: `activity/{menu,row}.rs`, `workspace/{activity_menu,activity_panel}.rs`, `activity_tests/{list,list_support,menu,focus,drawn}.rs`.
- Interfaces for later tasks: the filter lives in `Workspace` and the panel header; derive the shown rows before `ActivityPane::apply` (the pane compares shown rows) and turn the filter off in `show_holder` where its comment says. `ActivityLog::clearable` carries the one `// Used from Task 10` marker (Clear finished). The popover can reuse `view::activity::row::choice_of`, `Workspace::choose_activity` and the apply-time focus rule; `choose_activity` highlights the applied row, so a popover choice also moves the Activity pane's highlight (note for Task 11). The row menu's accessors are `activity_menu_open()`, `activity_menu_key()`, `activity_menu_at()` (test); the sweep reads the menu's labels, never the shared `"popup-menu"` id. Tab order: toolbar (Task 10), the list, then the highlighted row's enabled button only. **Task 12 uses `activity_menu_open()`.**
- **Task 10 must land P21 `rows_fit_a_narrow_activity`** reading `("activity-button", ix)` bounds against `find("operation-area")` after `scroll_to_item`: E-8 removed seven bounds/resize steps (`retire_left_offer`, `retire_pending_row_by_keyboard`, `retire_rows_hit`, `attention_actions_hit`, `attempt_entry_actions_hit`, `a_draft_not_saved_on_quit_by_keyboard`, `polling_keyboard`), and until P21 exists nothing pins a row's button inside the list. That gap must not reach Task 13.
- Sweeps (Part G, E-11, E-12, P28): the D5 `a16_` tests allow TITLE and READY inside `"operation-area"` (scrubbed to `{title}` and recorded as seen), BODY, COMMENT_TEXT, BASE and THEIRS stay forbidden, `holds_none` gains a positive control. `polling_privacy` has an explicit `SURFACES` list (17 → 12) with a "model row" surface (title, description, detail, tag, every offer label) and an "activity button" surface; FOLDER and REMOTE_PATH are added (9 → 11 sentinels); each old `read_at` pair maps to a new one (S7 wording: "is closed"/"was promoted" → "is closed"/"is approved"; "Holding back a" → "Your key is locked"). The drafts and remotes drawn sentinel scans are guarded by a drawn label containing the row's title (T8-3's `Region::Operations.id()` guard is replaced, not lost).
- Forced edits outside the E-list (not numbered; E-17 and E-18 stay unassigned candidates for Tasks 10 and 11): `activity_tests/pending.rs` helpers made `pub(super)`; `republish_tests` `ticket_shell` and `finished_deleted_share` made `pub(in crate::desktop::view)`; `attempt_entry_share_waits_for_the_lock` (the drawn-id is-not-focusable check becomes "Tab from the list does not reach the button handle"; one Tab, not the old 20-Tab walk); the `drafts_tests` and `remotes_tests` guards; `model/operations.rs` `NOT_AVAILABLE_YET`, `NOT_SENT_AGAIN`, `draft_flush_failed`, `Operations::retry_not_sent_again` deleted (the accepted loss of the disabled Retry's reason text stands).
- Removes (drawn-text and appearance assertions; none listed here is a behaviour assertion): `retire_left_offer`: the drawn `words()` equality with `RetireOffer::text()`, `!present(action)`, the bounds; `retire_rows_hit`: the thirty-retirement setup and its four drawn retirement-line assertions (R-51), the bounds, the resize; `retire_pending_row_by_keyboard`: `present` by drawn id, three bounds checks; `a_draft_not_saved_on_quit_by_keyboard`: the Retry wrapper's drawn ring, the resize; `attempt_entry_share_waits_for_the_lock`: above; `resume_by_keyboard`, `cancel_a_queued_request_by_keyboard`: drawn id reads replaced by row reads; `attention_actions_hit` no longer pins the order of the five hold rows (no model test pins the order of several holds among themselves; only the generic L7 sort covers it); `attention_actions_by_keyboard` no longer pins Tab order 0..4. No appearance test was deleted under E-15.
- E-9 (Activity part): `each_key_is_bound_to_its_action` 26 → 33. E-16: `ICONS` 49 → 53, `WifiOff` added. The focus maps (`control_focus`, `abandon_focus`, `abandon_focus_for`, `Workspace::abandon_focus`, `prune_abandon_focus`) are gone; every origin is the list's handle.
- Parked Minors (Task 9; for the final review): a Tab into the list does not start the highlight (T9-2); the explicit R-13 clears in `workspace/activity_menu.rs`, `repository_menu.rs`, `footer.rs` stop the Kit's focus-loss `DismissEvent` stealing focus from a just-opened menu, and no test pins them (P17 checks `menu_open` booleans; a mutation removing the repository menu's clear passed; the dock-menu direction is untested); `highlight_when_shown`'s `want` stays set if its key never reaches the log (Cancel's `RowKey::Flush`) and can hijack a later apply; the `polling_privacy` positive control "TITLE is drawn in Activity's rows" is satisfied by model rows alone and no control shows a row naming the second repository's remote; `show_it_focuses_the_entry_of_the_possible_holder` compares `contains` plus Busy, not equality; arrow keys while the button holds focus leave `button_focus` untracked when the new row has no enabled button (relies on the workspace's focus-lost fallback, untested), and the Enter-once guard (`list_focus.is_focused`) is unpinned; `kind_icon` picks `wifi-off` by comparing `derived.title` with `cant_reach_title` (a model flag would be sturdier); `activity_list` compares the whole row vector every render; `menu.rs` ~150 comment says "its Resume went" where the Resume now goes and returns.

## Task 10 (the toolbar and the rest of Activity; E-10)

- Commits (base `ba39f24f`): `e6a5285a` (toolbar, filter, ⋯, tests, E-10), `f36dea31` (R-13 clears), `7a4159ad` (Show it and the filter), fix `54976f45` (P21), `437e5cfe` (debug print removed). One reviewer, one fix round with a scoped re-review (addressed, no new breakage).
- Built: `ToolbarButton::{ActivityFilter, ActivityMore}` (`ALL` 7 → 9); the filter (state `aria_toggled` on the wrapper `activity-filter`; rows filtered to attention before `ActivityPane::apply`; empty state "Nothing needs attention"); ⋯ with "Clear finished" through the log (L8: requests through `dismiss_request`, other keys through `log.dismiss`; disabled when nothing is clearable at open); `show_holder` turns the filter off for a hidden holder. `ActivityLog::clearable`'s `// Used from Task 10` marker is gone; `grep "Used from Task" src` finds nothing. Activity's toolbar has no key context (a `NAVIGATION_CONTEXT` would make `secondary-n` a New item there).
- Ruling T10-1 (settles T9-2): the list starts its highlight on the first shown row whenever it holds focus, by any route, without stealing an existing highlight. It runs in `activity_list` at render (`ActivityPane::start_highlight`, no notify) and is pinned by P15's Tab clause (Tab into the list, then Enter presses the first row's Resume and shift-f10 opens its menu; the precondition "nothing highlighted" is asserted). The handler-side calls were tried, saw no test depend on them, and were removed. Cost if wrong: the hook moves to an `on_focus` listener.
- Found and fixed (Cycle 03a code, `footer.rs`, `repository_menu.rs`): the repository menus and the dock menu did not close each other (R-13); the Kit's focus-based dismissal hid it. The all-pairs layers test (20 ordered pairs of five layers, state read in the same update before any draw) found it; seven single-clear mutations each fail it. This pins Task 9's parked R-13 Minor in every direction. **Task 11 adds the popover as a sixth layer to `activity_tests/layers.rs`'s `Layer` table and must clear `activity.more` and every other layer when it opens, and each of them must clear it (R-13, E2).**
- Process deviation, stated plainly: for the toolbar the implementer built before writing the tests; P29, P30 and P32 were green first run (the behaviour existed since Tasks 4, 5 and 9); RED for the others is by mutation of the line each test pins (filter ignored; empty text constant; render hook removed; Clear takes every row; republish wait forced false; item always enabled; Tab order swapped; `show_holder` without the filter step; refresh while hidden; L3 keep off; S6 skips main; the filter toggle starts a check). The reviewer judged from the code that each would fail.
- P19: the window test sets the stopped-share and stopped-republish facts with `note_share_attempt` and `resends().after_republish` (the facts the Workspace reads), not a real SSH share; the model test `log_more::clearing` pins the same table. After the fix round the fixture holds eleven kinds: the ten of the plan plus an S2 brought-in row (through `log.dismiss`); only the success, the settled failure, the two S8 events, the S2 row and the cancelled close go. The disabled-item and late-row steps are pinned.
- P21 (`rows_fit_a_narrow_activity`), after the fix round: Activity in the right dock at 320 px in a 1280×360 window (list about 97 px, about three rows), two windows (A: a hold with a long name and "+1", Retry and Share… requests, pending Resume rows; B: reminders, a problem, an S2 row, an S7 retire offer, an S8 Retired event, a lifecycle Resume row, a state row "Retry now"), every kind asserted present; the last row is asserted not drawn before it is scrolled to; each row is scrolled to and its row and `("activity-button", ix)` must lie inside `operation-area` on all four edges (a row with no button must draw none). Mutations seen failing: `min_w_0` and `overflow_hidden` removed from the message for state rows alone, and for reminder rows alone. This closes the gap Task 9 recorded; nothing now depends on it. "Longest message of each kind" is covered by a mixed-kind `longest > 100` precondition and long repository names, not per kind (reviewer: acceptable, the text is model-generated).
- E-10 (`dock_tests/toolbars.rs::panel_toolbars_work_by_tab_and_enter`): additions only. `with_no_repository` gains two table rows; `single_panels` gains the Activity buttons (Shift-Tab to ⋯ then the filter, Enter on ⋯ opens `[Clear finished]` and Escape returns focus, Enter toggles the filter, `secondary-n` makes no dialog); `behind_a_dialog`'s loop over `ALL` (candidate E-17, `toolbars.rs:323`) passes unchanged and now also asserts the filter stays off and no menu opened. E-17 therefore needs no edit; E-18 remains for Task 11.
- Edits outside the E-list (additions only): `attention_support` and `Rig` widened to `pub(in crate::desktop::view)` (P21 uses the attention rig); `show_holder` visibility; test accessors `dock_menu_open()`, `activity_more_open()`; `follow::{found, item, main_branch}` and `clear::finished_after` made `pub(super)`.
- Interfaces for Tasks 11 and 12: `Workspace::{activity_filter, toggle_activity_filter, show_row_through_filter}`; "Show everything in Activity" sets `self.activity.filter = false` then `focus_region(Operations)`; the sweep ids are `activity-filter-button`, `activity-more-button`, `activity-filter`, with fixed copy (`SHOW_ONLY_ATTENTION`, `MORE`, `CLEAR_FINISHED`).
- Sizes: all new files below 500 (`activity/more.rs` 61, `workspace/activity_more.rs` 121, `activity_tests/{toolbar 288, clear ~330, layers ~210, follow 310, narrow ~150}`); `requests.rs` 611 (grew by 6 lines).
- Parked Minors (Task 10): the layers test cannot tell `Repository` from `RepositoriesMore` for the one pair that shares a field; `start_highlight` is a state change at render without notify (the drawn highlight is in the same frame, so the report's "frame after" is pessimistic); P19 fakes the stopped share and republish facts rather than using real records.

## Task 11 (the footer item and the Needs attention popover; E-9 popover part, E-13)

- Commits (base `189e077b`): M1 `8bb8c88d` (footer item E1, key actions and bindings, the popover tests written first and seen failing), M2 `e2a75e19` (the layer), M3 `413fded7` (E-13 readers, E-9, P13, P31, the sixth R-13 layer), fix `baf3b360`. Two reviewers (production code: approved, no Critical or Important; tests: three Important), one fix round, scoped re-review approved.
- U-8 holds: `Workspace::focus_lost` sees `window.focused(cx)` equal to the Workspace-kept handle when a prompt or dock menu opened from the popover closes onto it, so R-50's fallback is in place and the "move focus to the footer item first" fallback is not needed. The check sits in `focus_lost` before its region fallback.
- Ruling T11-1 (a design deviation, reviewed and judged sound): the popover reuses Activity's `ActivityPane` (its own instance) as its state. The design's cursor, focus and restore fields become the pane's highlight by `RowKey`, the Workspace's handles and the footer item's handle; the C2 apply rule (a button holding focus whose row leaves, or whose row has no enabled button, sends focus to the list) comes for free. Cost if wrong: a second copy of the rule to maintain.
- Ruling T11-2: `attention_focus` is three Workspace-owned handles (`AttentionHandles {list, button, link}`, kept for the window's life), not one; `focus_lost` falls back to the footer item (else E1's control) when `window.focused` is one of them and the popover is not drawing it. A press outside needs no deferred focus code: it keeps the focus it gave, else the fallback applies.
- Ruling T11-3: a popover choice also highlights the applied row in Activity's pane (`choose_activity`'s `highlight_applied_row`): where the person lands in Activity. Wanted, left. It also moves Activity's highlight and scroll when Activity is hidden.
- Built: footer item bounds, tooltip, `press(Attention)` toggles, `focus_after_hiding(Attention)` moves right (R-37), `keep_focus_in_footer` covers Attention (R-49); `view/needs_attention.rs` (357) and `view/workspace/needs_attention.rs` (457), `ATTENTION_POPOVER_CONTEXT = "NeedsAttention"`, eight root key handlers through `unless_dialog`, ids `needs-attention-…`; anchored `BottomRight` 6 px above the footer (R-38), 440 px; Tab's three stops (R-12); a choice is built from the popover's drawn row (R-55) and checked by `still_offered` before the popover closes and `choose_activity` runs with the footer item as origin; "Show everything in Activity" closes the popover, turns the filter off, highlights the first row, focuses Operations (P24); closing at 0, under a modal, when another layer opens (R-13: row menu, ⋯, both repository-menu openers, dock menu) and on a press outside (ignored inside the item's bounds, fact 47). The popover is the sixth layer in `activity_tests/layers.rs` (30 ordered pairs).
- P23 after the fix round: split by source group into several tests, each press asserting the button label, that the popover closed, the action's effect as P18 asserts it, and focus back on the footer item for dialog and sheet kinds. Real-worker rows (no SSH, through `outcomes_tests` helpers widened visibility-only): Unlock… and Retry (same request id, attempt + 1), Share (a finished-record failure, because a stopped share is information, not attention, per the design's table), Check host, Set identity. S4: Retry now, Check now, Resume checking (the state's own Attend), Reread, Retry, Settings…; S9 Review changes; S11 flush Retry. The state row's Set up sharing is an information row (R-15): a test asserts it is in the log and not in the popover. Remaining recovery kinds beyond `IndexRefresh` and `RequestPrepare` go through the same dispatch P18 pins.
- Stale rows (T8, R-55): the A-B-A case reads the drawn revision from the popover's own applied row and fails if the revision is taken from the log's current row (4 vs 3 requests); the offer branch is isolated by a same-revision choice with a label the row does not offer (refused, the offered label then runs); a row that stays with its Resume disabled and a row that is gone both assert the press arrived. P25's two halves: the highlighted row leaves (`the_highlighted_row_leaves`) and the row stays highlighted with no enabled button (`the_row_at_the_position_has_no_button`); in `ActivityPane::apply` a different row at the position is the same `!kept` branch as the row leaving, so no branch is unpinned.
- E-13 Removes (each justified by E1, a press now toggles the popover instead of revealing Activity): `status_table.rs` the assertion `focus_after_hiding(Attention, ..) == None` (replaced by a four-case table and a positive control); `needs_attention_counts_and_leads_to_activity` (renamed `needs_attention_counts_and_opens_the_popover`): after the press, `is_displayed(Activity)` and `focused(Region::Operations)` (replaced by: the popover is open with one row, Activity stays hidden, a second press closes it); `needs_attention_works_by_enter`: after Enter, `is_displayed(Activity)`, `present(Region::Operations)`, `ringed_on(Operations)` (replaced by: the popover is drawn, Activity hidden, Escape closes it and the item has focus and ring); the two "Away" rows each get a window. Additions only: `status.rs` (P26), `footer_controls_do_nothing_behind_a_dialog`, E-9 (keys table 33 → 41), P13 (hidden Activity and a quiet row so "list every row" fails), P31. Candidate E-18 (`keyboard_tests/shell/support.rs:102`): no edit needed.
- Mutations seen failing and restored: about 15 in M2/M3 and 6 in the fix round (revision from the log's current row; no item-bounds check on a press outside; no `focus_lost` fallback; `keep_focus_in_footer` without Attention; dock menu leaves the popover open; popover lists every row; opening the popover submits a refresh; closing the popover without running the choice; `still_offered` ignoring the offer; `!kept || !button` reduced to `!kept`; reverse order). Two tests initially survived (P27 behind the press guard, P13 with "list every row") and were strengthened.
- Edits outside the E-list (visibility and plumbing only): `activity_tests` helpers and modules, `operations/attention_tests` (`every_reason`, `hold_rows`, `hold_with`, `shown`), `operations.rs`, `operations/outcomes.rs`, `outcomes_tests.rs`, `activity.rs` (`scroll()`, `rows_applied()`), `row.rs` (`action_button` split into `button`; `kind_icon`, `tooltip` widened; behaviour-preserving, reviewer checked).
- For Task 12: the popover's ids are `needs-attention-popover`, `-list`, `("needs-attention-row", ix)`, `("needs-attention-button", ix)`, `needs-attention-show-all`; fixed copy `NEEDS_ATTENTION_TITLE`, `NEEDS_ATTENTION_SAFE`, `SHOW_EVERYTHING`, `ATTENTION_TOOLTIP`; the footer item's tooltip is new. `Workspace::activity_menu_open()`, `activity_more_open()` and the popover's accessors are the layers' probes. The popover's rows are the log's attention rows, so the model-row surface of `polling_privacy` already covers their text; the drawn rows, tooltips and the header need their own surface.
- Parked Minors (Task 11): the `focus_lost` fallback fires on a lost popover handle even with the popover open, so a wheel scroll that virtualizes the highlighted row's button out of the `uniform_list` moves focus to the footer item with the popover still open and Escape dead (fix: focus the popover's list while it is open); global region shortcuts (F6, region navigation) can move focus out with the popover still open (R-12 asks only that Tab stays inside); `press_attention`'s `still_offered` duplicates `choose_activity`'s (harmless; wants a comment); the anchor's y assumes the footer sits at the window's bottom edge and `CHROME_HEIGHT = 118` is a magic number; `footer_tests/mod.rs` 493 → 518 lines; `open_attention_item`'s `is_current` stale branch has no popover case; `ATTENTION_POPOVER_CONTEXT` is not asserted in `keys_tests`; `#[cfg(test)]` `pressed` counters sit in `workspace/needs_attention.rs`; the root `unless_dialog` guards on the popover's keys are not independently pinned (the layer drops under a modal first); `requests.rs` doc comments misplaced on `share` and `popover_actions_real_requests`, and the held second `RepoSync` in `share` is not awaited; the popover's own `open.rs` test no longer exercises the item and disabled quiet-row kinds (P13 covers them).

## Task 12 (privacy of the new layers; E-14, P28)

- Commit `33be3c66` (base `0dccc834`), one reviewer, approved first time (no Critical or Important).
- New shared test module `D/view/privacy_layers_tests.rs` (under `cfg(all(test, feature = "desktop-test-support"))`, one `mod` line in `view/mod.rs`): `in_row_menu(path, activity_menu_open)` gives the TITLE and READY allowance to items under the Kit's shared `"popup-menu"` id only while the row menu is the open menu (read through `activity_menu_open()` in the same update, never by the id alone), so the allowance cannot reach the dock, repository or More menus (R-13 keeps two menus from being open at once); `popover_texts` reads each popover row's tooltip lines and joined description plus the fixed copy; `row_menu_texts` reads the open menu's items from its row. A unit test, `the_allowance_reaches_only_the_row_menu`, fails if the open flag is ignored.
- `polling_privacy`: new `polling_privacy_tests/layers.rs` holds the drive and the controls (the main file grew by 51 lines, 1141 → 1192, rather than more). `SURFACES` 12 → 15 (popover row, popover button, popover, plus a "row menu" attribution); the surfaces asserted read at the end 17 → 25; none dropped. It now runs real Check nows of the second and third repositories, leaving failed requests that need attention, so the popover lists rows naming b and c, and FOLDER, REMOTE_PATH and PASSWORD are swept through them. The popover is opened twice and one row's menu once, with the sweep running while each is open.
- D5 (`lifecycle/privacy_tests.rs`): two new surfaces (popover row, popover) plus the row-menu attribution; `Seen::title_only` allows TITLE and READY in the operation area, popover rows and the row menu and nothing else; the shared-conflict `a16_` test opens the popover and the row menu on the item's conflict row and asserts the title is drawn there.
- Task 9's two parked Minors in these files are fixed: drawn and model title sightings are noted apart (`activity-row` drawn, `model-row` model), so model rows alone no longer satisfy the "TITLE is drawn in Activity's rows" control, which also asserts the drawn Activity row and the drawn menu were read with the title scrubbed out; and a control shows the second repository's row ("origin isn't an SSH shared copy…", tag `b`) drawn and in the model, naming the remote by name only, with REMOTE_PATH and PASSWORD in no row.
- Mutations seen failing and restored: polling_privacy — popover header plus BODY, header plus TITLE, popover row plus BODY, REMOTE_PATH in `tooltip()` for b, FOLDER in `tooltip()` for c, row menu plus BODY, a drawn row stripped of its title; D5 — header plus BODY, header plus TITLE, row menu plus BODY, `tooltip()` plus COMMENT, popover row plus BASE; unit test — `in_row_menu` without the open flag.
- Tests: `polling_privacy`, D5's four `a16_` tests and the new unit test, 6 passed together by exact name (24 s); fmt, `cargo check --all-features --locked`, clippy `-D warnings` clean; boundary diff empty. Fast tier not run (Task 13). No production code changed.
- Parked Minors (Task 12): the popover header's two text divs have no id, so no snapshot reads them and the "popover copy" sweep reads compile-time constants (tautological; a leak there would need a code change to `NEEDS_ATTENTION_TITLE`/`NEEDS_ATTENTION_SAFE`; fix: give the divs ids `needs-attention-heading` and `needs-attention-safe` with one mutation); nothing pins that the call sites pass the real `activity_menu_open()` flag (a hard-coded `true` would pass every sweep), and no sweep opens the dock, repository or More menu with a planted title (those menus carry fixed copy and repository names, so an end-to-end case would be vacuous); in D5 the "popover row" surface also contains the row's buttons, so the title allowance reaches popover button labels there (polling_privacy is stricter; harmless, only TITLE and READY are allowed); in `polling_privacy` no popover row carries a title (D5's shared-conflict test supplies that control); `lifecycle/privacy_tests.rs` is 526 lines (was 488; one method, `Seen::title_only`) and `polling_privacy_tests.rs` is 1192 (pre-existing over 500).

## Task 13 (checklist, gates, final review, review-ready)

- On-screen checklist `docs/plans/2026-11-12-wave-04-cycle-03b-on-screen-checklist.md` (commit `daf5bd71`): 85 rows in S-1 to S-10, every Result cell blank; written by a subagent, read by the orchestrator. Owner items it leaves open: B4 descriptions, the ⋯ tooltip, the filter's on look, the popover's 6 px and 440 px to measure, setup methods for several rows.
- Gates at `619ffb64` (no source change until the fix wave): `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` all rc=0. CLI smoke test ran, rc=0, no window. `grep -rn "Used from Task" src` prints nothing. `git merge-tree --write-tree HEAD origin/main` clean (`origin/main` still `ba95e1b7`). Boundary diff prints nothing.
- Fast tier once, `--no-fail-fast`: 5,998 passed, 2 failed across 79 test-result lines (about 40 min). The failures are Cycle 03a tests the branch did not edit: `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard` (menu.rs:339, "Can't reach" where "Needs attention" expected) and `check_all_checks_each_repository` (toolbar.rs:144, `raise(KeyLocked)` returned false). Each passed alone, and the module filter passed three times (5 each). Ruling T13-1: a load flake of the family Cycle 03a already fixed in `595667cd` (the ledger had named the second at Task 4); the final review judged both pre-existing waits missing an observable condition (I-1) and one fix wave hardened them. The fast tier was not rerun: AGENTS.md reruns only a target whose code changed, and these two tests were rerun by name.
- Final whole-branch review (most capable model, read-only): 0 Critical, 2 Important, 7 Minor; verdict needs a small fix wave. Report in the SDD scratch directory (`final-review-report.md`). Its triage of the 67 parked Minors: 1 to fix (Task 11's focus-lost fallback, which is I-2), 6 already fixed by later tasks, the rest left with a reason each.
- Fix wave (one implementer, `f815b793`, `e68c5516`, `2f8a1f31`; one scoped re-review, all addressed, no new breakage):
  - I-1: waits added in `repositories_tests/toolbar.rs` (`c` scheduled and not in flight before `raise(KeyLocked)`) and `menu.rs` (`b` not in flight and not refreshing before the hand-made `after` and `raise(RecoveryHeld)`). No assertion removed (E-19, E-20).
  - I-2: `view/workspace/focus.rs`: a focus loss on a popover row button while the popover is open focuses the popover's list, not the footer item. New case `the_button_scrolls_out_of_the_list` in `the_popover_closes`, seen failing first (`focus moved to its list`, left `Item`, right `List`).
  - m-1: the stale `NotSavedChoice::Cancel` comment.
- Tests run after the fix wave: the two flaky tests by exact name and the five-test module filter; `the_popover_closes`, `popover_actions_by_keyboard`, `the_popover_opens_lists_and_closes`, `the_popover_works_with_activity_hidden`, `a_dialog_from_a_row_returns_focus_to_its_row`, `activity_by_keyboard`: all passed. Static gates rerun at `2f8a1f31`: clean.
- Ruling T13-2: the I-2 test puts the new failed request on another repository than the six pending rows, because a failed request on the same repository dropped that repository's pending rows from the log. Not investigated (the implementer did not; the re-reviewer judged the test sound). Possibly the intended replacement of a pending listing by the request's own row; recorded for the owner as an open question, not a finding.
- Final-review items left, for the owner and the close: m-2 (the owner-approved rulings C1 to C7 and C9, T9-2, T10-1 and T11-1 to T11-3 are recorded only here, not in the design or Cycle documents; the ledger said "to be listed as document amendments at close"; not edited because those documents are the owner's), m-3 (duplicated helpers `sharing`/`item_sharing`, `dto_kind`, the `TargetKind` match), m-4 (`model/repository_state/menu.rs` imports three constants from `view::sidebar`), m-5 (F4's dead drawing code kept under `cfg(test)`), m-6 (P13's journey covers three counted kinds, not every kind), m-7 (file sizes: `model/operations.rs` 668, `footer_tests/mod.rs` 518, `lifecycle/privacy_tests.rs` 526, `workspace/lists.rs` 502).
- Acceptance Status filled in the Cycle document (`47306e39`): A1 to A13, A15 and A16 proven by tests by exact name (A9 and A15 with the noted flake); A14 awaits the owner.

## Tests run

| Checkpoint | Tests run |
| --- | --- |
| Task 0 | none (static gates only) |
| Task 1 | `desktop::model::operations` by module: 32 passed, 0 failed (the one module run the plan names). fmt, `cargo check --features desktop` and clippy `-D warnings` clean. |
| Task 2 | By exact path, 1 passed each: `desktop::model::operations::tests::shares_are_classified_once`; `desktop::model::operations::tests::stages_are_shown`; `desktop::view::operations::tests::state_label_shows_stages`; E-1 `desktop::view::operations::outcomes::give_up_tests::a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`; E-2 `desktop::view::operations::check_tests::a_running_check_now_reads_its_stage_words`. fmt, check and clippy clean. |
| Task 3 | By exact path, 1 passed each, red on stubs first: `desktop::model::activity::tests::activity_time_formats`; `...::request_copy_table`; `...::share_copy_table`; `...::each_source_makes_its_rows`. fmt, check and clippy clean. No existing test run or edited. |
| Task 4 | 1 passed each: `desktop::model::polling::tests::brought::brought_in_is_kept_for_the_session` (red on stubs first); `desktop::model::activity::tests::each_source_makes_its_rows` (S1 + S2–S14); `desktop::model::activity::tests::states::copy::state_copy_table`; `desktop::model::repository_state::tests::causes::each_attention_cause_is_set`; Cycle 03a, unchanged and green: `desktop::model::repository_state::tests::{each_state_and_its_precedence, tooltips_say_the_state_and_the_facts, menu_items_follow_the_state, check_all_takes_each_repository_that_can_check}` and `desktop::view::repositories::tests::{rows_and_list_states, titles_of_what_is_not_shared, the_menu_by_pointer_and_keyboard, check_all_checks_each_repository (one flake in a combined run, then green alone twice), the_more_menu_acts_on_the_chosen_repository}`. fmt, check, clippy clean. |
| Task 5 | Each alone, count 1, ok: `desktop::model::activity::tests::logs::attention::attention_counts_each_thing_once` (97 rows) and `...::logs::log::the_log_keeps_times_and_events` (written first, every row red on stubs); `...::s1::a_share_title_that_begins_with_its_subject_is_capitalised`; `desktop::model::polling::tests::brought::completed_attempts_are_counted_per_root` (red first); `desktop::services::scheduler::attention_answer_tests::a_failed_check_now_is_recorded_when_its_answer_applies` (red first, then red for the retry row); `desktop::services::resends::tests::waits_on_says_when_forgetting_would_drop_something`; `desktop::view::activity::tests::count_and_popover_agree`; `desktop::view::activity::tests::refreshes_are_coalesced` (red with coalescing off: 6 derivations vs 4); `desktop::view::operations::resume::activity_tests::a_running_resume_makes_the_stranded_row_still_running`; changed: `each_source_makes_its_rows`, `request_copy_table`, `share_copy_table`, `activity_time_formats`, `brought_in_is_kept_for_the_session`; E-3 `retire_resume_resends_original`; the nine E-4 tests above. fmt, check and clippy clean after each commit. |
| Task 6 | Each alone, count 1, ok (35 tests): `desktop::view::activity::tests::each_action_runs_its_handler` (new; 21 kinds, T11, T12); the six E-5 `desktop::view::operations::tests::*` tests; the seven `desktop::view::operations::outcomes::give_up_tests::*`; the thirteen `...::outcomes::tests::*`; the six `...::outcomes::route_tests::*` plus the untouched E-7 `show_it_focuses_the_entry_of_the_possible_holder` (it shares the changed helpers; green); `desktop::view::operations::check_tests::a_running_check_now_reads_its_stage_words`; `desktop::view::operations::record_tests::an_unreadable_record_is_reported_once_in_activity`. (Full names in the Task 6 test lists above.) fmt, check and clippy clean. |
| Task 7 | Each alone, count 1, ok: the 31 E-5 part 2 tests (exact paths in the Task 7 report: `attempts_tests` 2, `attention_tests` 4, `republish::follow_tests` 7, `republish::stop_tests` 4, `republish::tests` 7, `resume::tests` 7, `retire_tests` 5 — Task 7 ran them by `desktop::view::operations::…` paths); sharing a changed helper and green: E-7 `attention_actions_hit`, `attention_actions_by_keyboard`, `attempt_entry_actions_hit`, `attempt_entry_ids_follow_the_repository`, `retire_left_offer`, `retire_pending_row_by_keyboard`; `resume::activity_tests::a_running_resume_makes_the_stranded_row_still_running`; `activity::tests::each_action_runs_its_handler`. fmt, check and clippy clean. |
| Task 8 part A | All via `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`. 35 converted tests, each 1 passed, 0 failed (the one first-run failure, `each_stage_shows_its_text_...`, was my assertion on the row's words; fixed as above and rerun green):  - `desktop::view::conflict::entry_tests::` `the_banner_offers_review_changes`,   `a_pending_resolve_row_offers_review_changes_after_restart`, `a_local_synchronization_offers_resume`,   `resolve_conflict_opens_the_childs_sheet`, `an_applied_resolution_clears_the_banner_and_its_review`,   `a_resume_that_settles_the_conflict_clears_its_review` (6) - `desktop::view::conflict::repair_tests::` `repair_locally_syncs_and_opens_the_sheet_on_conflict`,   `a_refused_local_sync_shows_its_answer_and_resume`, `repair_locally_waits_for_a_queued_sync` (3) - `desktop::view::confirm::tests::` `pressing_enter_twice_on_confirm_submits_one_request`,   `region_and_refresh_keys_do_not_escape_the_dialog`, `quit_works_while_a_dialog_is_open`,   `a_stale_preview_is_shown_with_prepare_again`, `a_busy_preview_offers_retry_which_prepares_again`,   `an_answer_to_an_earlier_attempt_is_not_shown_after_prepare_again` (6) - `desktop::view::find::tests::find_does_nothing_while_a_dialog_is_open` (1) - `desktop::view::tabs::tests::the_tab_keys_do_nothing_while_a_dialog_is_open` (1) - `desktop::view::item::share::tests::` `each_stage_shows_its_text_for_share_and_post`,   `a_post_without_a_remote_never_says_sharing`, `an_accepted_share_offers_stop_and_stopping_signals_it`,   `a_stopped_share_says_what_completed_and_offers_share`,   `the_conflict_banner_says_work_is_safe_and_offers_review_changes`,   `a_finished_share_says_its_outcome_in_the_pill`, `a_kept_draft_after_a_shared_outcome_reads_draft_kept`,   `a_shared_outcome_leaves_the_pill_once_the_item_reads_otherwise`,   `sharing_private_work_shows_shared_until_the_item_reads_differently` (9) - `desktop::view::item::edit_tests::release::` `a_release_is_not_retried_once_the_item_is_prepared_again`,   `a_refused_release_lets_the_earlier_prepare_resume`,   `a_prepare_is_not_sent_again_once_its_context_is_released` (3) - `desktop::view::item::posting::tests::outside::` `an_unsaved_posts_operation_entry_offers_no_retry`,   `a_saved_unshared_posts_operation_entry_offers_retry_sharing`,   `an_unsaved_posts_operation_entry_offers_no_resume_or_prepare` (3) - `desktop::view::item::posting::tests::decisions::a_post_stopped_on_a_conflict_raises_the_items_banner` (1) - `desktop::view::item::posting::tests::extras::quitting_during_an_accepted_post_leaves_it_saved_and_cancelled` (1) - `desktop::view::item::posting::tests::recovery::p3_a_stopped_post_offers_share_not_retry` (1)  Helper-sharing or untouched-by-conversion tests run (green, 1 passed each): - E-7 `desktop::view::confirm::tests::escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon`   (shares `stranded`; still presses by id) - `desktop::view::item::posting::tests::review::an_operation_area_retry_is_followed_and_posts_one_comment`   (the unchanged E-6 name, see concern)  `row_of` is used only by `entry_tests` and `repair_tests` (all callers converted and run); `retry_control` only by the three release tests; `recovery_labels` only by the three outside tests. No whole-module or whole-suite runs.  Verify lines (all pass): `devenv shell -- cargo fmt --check`; `devenv shell -- cargo check --locked --features desktop`; `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. Fix round: `an_accepted_share_offers_stop_and_stopping_signals_it` and `a_post_without_a_remote_never_says_sharing`, 1 passed each. fmt, check, clippy clean. |
| Task 8 part B | `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`. No failures; no reruns needed.  Converted (23): - `desktop::view::lifecycle::outcome_tests::the_operation_entry_reopens_the_dialog` - `desktop::view::lifecycle::outcome_canned_tests::` `each_way_out_offers_its_button`, `busy_after_the_merge_is_failed_not_still_running`,   `a_lifecycle_entry_keeps_the_recovery_its_dialog_does_not_own` - `desktop::view::lifecycle::interruption_tests::` `p5_a_close_stranded_at_closing_resumes_after_relaunch`,   `a_close_lost_before_settlement_is_complete_and_stays_accepted` - `desktop::view::lifecycle::made_ready_tests::done_lists_made_ready_with_slug_and_title` - `desktop::view::lifecycle::privacy_consent_tests::` `a17_a_stranded_close_is_not_resumed_without_consent`,   `a17_a_conflict_waiting_after_a_restart_is_not_resumed_or_resolved` - `desktop::view::lifecycle::privacy_promote_tests::a16_a_promotion_stopped_untidied_and_done_shows_no_item_text` - `desktop::view::lifecycle::tests::a_blocked_prepare_lists_blockers_and_way_out` - `desktop::view::dock::tests::state::a_request_finishes_while_activity_is_hidden` - `desktop::view::drafts::entry_tests::` `the_launch_offers_its_recovered_drafts_and_restores_nothing`, `an_operation_entry_names_the_item_never_the_draft` - `desktop::services::drafts::tests::` `a_store_that_cannot_be_protected_still_opens_the_window`, `a_failed_flush_shows_an_entry_with_retry_and_no_draft_text` - `desktop::services::drafts::save::tests::dismissing_a_pending_save_in_the_operation_area_lets_another_be_sent` - `desktop::view::settings::keys::tests::the_operation_area_retry_of_a_protected_generation_asks_on_the_page` - `desktop::view::settings::remotes::tests::` `an_added_url_with_a_password_is_kept_out_of_everything_shown`,   `selecting_a_remote_that_is_not_eligible_shows_its_typed_failure`, `a_partial_add_is_completed_by_retry_under_the_same_request` - `desktop::view::settings::repositories::tests::a_partial_create_is_completed_by_retry_under_the_same_request` - `desktop::view::settings::repositories::manage_tests::a_release_that_failed_during_removal_completes_once_retry_stores_the_text`  Helper-sharing, not E-6-named (6, all passed): callers of the changed `reopen` and `row_of`/`until_pending`: - `desktop::view::lifecycle::made_ready_tests::index_pending_says_the_list_may_be_incomplete` - `desktop::view::lifecycle::credential_tests::a_refused_key_offers_unlock_and_sharing_and_keys` - `desktop::view::lifecycle::credential_tests::enter_on_a_locked_key_unlocks` - `desktop::view::conflict::entry_tests::a_pending_resolve_row_offers_review_changes_after_restart` - `desktop::view::conflict::entry_tests::a_local_synchronization_offers_resume` - `desktop::view::conflict::repair_tests::a_refused_local_sync_shows_its_answer_and_resume`  Not run (unchanged, touch no Activity): the other two `a17_` tests, `review_in_drafts_opens_the_page_on_the_drafts_row`, `review_in_drafts_waits_for_an_open_dialog`. No whole-module or whole-suite runs.  Verify (all pass): `devenv shell -- cargo fmt --check` (exit 0); `devenv shell -- cargo check --locked --features desktop`; `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. fmt, check, clippy clean. |
| Task 9 | `cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`. New/edited, 43 names passed (tooltip, keys, assets, P14-P16, P17, P18 `each_action_runs_its_handler`, P20, the 26 E-7 tests, `attempt_entry_share_waits_for_the_lock`, the four D5 `a16_` tests, `polling_privacy`, the drafts privacy test). E-5/E-6 reruns by name: 176 tests in four batches, all passed. 125 others (quit, editing privacy, footer, dock, keyboard shell, settings polling/hosts, workspace, drafts) passed, none broken. Fix round: 26 by exact name (activity tests, `retire_tests` 7, `attention_tests` 6, `attempts_tests` 4, `polling_keyboard`) passed; three mutations seen failing and restored. fmt, check and clippy `-D warnings` clean at every milestone and after the fix. Fast tier not run (Task 13). |
| Task 10 | `cargo test --locked --features desktop-test-support --bin manyhands -- --exact <name>`, each alone. 13 new or edited passed: P14 filter (`rows_draw_from_the_log_and_filter`), P15 Tab clause (`activity_by_keyboard`), P19 (`clear_finished_keeps_failures_and_states`), the R-13 layers test (`one_layer_at_a_time_in_activity`), P31 (`navigation_sends_nothing`), P29 (`activity_hidden_keeps_the_log`), P30 (`a_checking_repository_keeps_its_row`), P32 (`reminders_match_to_share`), P21 (`rows_fit_a_narrow_activity`), `show_it_turns_the_filter_off_for_a_hidden_holder`, E-10 (`panel_toolbars_work_by_tab_and_enter`), and the reruns `show_it_focuses_the_entry_of_the_possible_holder` and `each_key_is_bound_to_its_action`. Also by exact name for the named doubt (toolbar tab stops before the list; R-13 clears changed): 30 E-7, keyboard, footer-Tab and P18 tests, and 17 menu, toolbar, footer and dock tests, all passed. After the fix round: P21, P19 and `activity_by_keyboard` passed, and P21 again after the debug print was removed. fmt, check and clippy `-D warnings` clean. A module-prefix test run was refused by the permission layer and not retried. Fast tier not run (Task 13). |
| Task 11 | `cargo test --locked --features desktop-test-support --bin manyhands -- --exact <names>`. At M3, 22 passed: the five popover tests (P22 `the_popover_opens_lists_and_closes`, P25 `the_popover_closes`, P24 `the_popover_works_with_activity_hidden`, P27 `nothing_acts_behind_a_dialog_or_sheet`, P23 `popover_actions_by_keyboard`), `count_and_popover_agree` (P13), the layers test (30 ordered pairs), `navigation_sends_nothing` (P31), the keys table (E-9), the eight E-13 readers, and five Activity tests touched by the `row.rs` and opener changes. Fix round: 22 passed (28.6 s): the P23 groups and stale tests plus the ten tests of the widened `outcomes` helpers; one flake in `press_from_popover` (S3 holds reorder while polls answer) fixed by re-reading the order at each arrow step and rerun three times. fmt, check `--all-features` and clippy `-D warnings` clean at every commit. Fast tier not run (Task 13). |
| Task 12 | `cargo test --locked --features desktop-test-support --bin manyhands -- --exact <names>`: `polling_privacy`, D5's four `a16_` tests and the new unit test `desktop::view::privacy_layers_tests::tests::the_allowance_reaches_only_the_row_menu`: 6 passed together (24 s). fmt, `cargo check --all-features --locked` and clippy `-D warnings` clean. Fast tier not run (Task 13). |
| Task 13 | Fast tier once at `619ffb64` (`--no-fail-fast`): 5,998 passed, 2 failed (the two Cycle 03a load flakes, each passing alone and in a module run). After the fix wave, by exact name: both flaky tests, the `repositories::tests::` module (5 passed), `the_popover_closes` (with the new case, red first), `popover_actions_by_keyboard`, `the_popover_opens_lists_and_closes`, `the_popover_works_with_activity_hidden`, `a_dialog_from_a_row_returns_focus_to_its_row`, `activity_by_keyboard`. check, fmt and clippy clean at `2f8a1f31`. No SSH target (desktop only). |

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
- E-5 part 1 (Task 6): `operations_tests.rs`, `give_up_tests.rs`, `outcomes_tests.rs`, `outcomes_route_tests.rs`, `check_tests.rs`, `record_tests.rs` moved onto the seam; the test-function lists are equal in all six; the removed drawn-text assertions are listed under Task 6 above. No E-17+ outside these files.
- E-5 part 2 (Task 7): the seven files above moved onto the seam; test lists equal; Removes listed under Task 7. E-17 (above).
- E-6 (Task 8): the files above moved onto the seam; test lists equal; Removes listed under Task 8. E-17 candidates: `republish_tests::until_offered`/`until_not_offered` duplicate the seam's; `conflict/entry_tests.rs::row_of` became a wrapper over `until_pending` (no assertion removed).
- E-7, E-8, E-9 (Activity part), E-11, E-12, E-16 (Task 9): done; steps and Removes under Task 9. The forced edits outside the E-list are listed there (unnumbered). E-10 and E-13 remain for Tasks 10 and 11; E-14 for Task 12.
- E-10 (Task 10): done, additions only; E-17 (`toolbars.rs:323`) needs no edit. Details under Task 10. E-13 and E-18 remain for Task 11; E-14 for Task 12.
- E-9 popover part and E-13 (Task 11): done; E-18 needs no edit. Removes and replacements under Task 11. E-14 remains for Task 12.
- E-14 (Task 12): done (D5 and `polling_privacy` read the popover and the row menu, title-only). Details under Task 12.

- E-19, E-20 (Task 13 fix wave): waits added to `check_all_checks_each_repository` and `the_menu_by_pointer_and_keyboard` (Cycle 03a tests, I-1); no assertion removed.
