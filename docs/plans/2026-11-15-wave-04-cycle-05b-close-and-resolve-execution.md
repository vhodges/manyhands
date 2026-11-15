---
title: "Wave 04 Cycle 05b Execution Ledger"
date: 2026-11-15
status: "in progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
---

# Wave 04 Cycle 05b Execution Ledger

Ticket `01M7D7JK5MSY6J471JEKG3CZXR` (`vh-qjw5e`), branch
`manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`. Written only by the
orchestrator. Plan:
[implementation](2026-11-15-wave-04-cycle-05b-close-and-resolve-implementation.md);
brief: [handoff](2026-11-15-wave-04-cycle-05b-handoff.md). Subagent-driven and
serial (Cycle decision 13). Implementers: Task 0 on Opus, others per task.

## State

| Task | Status | Commits | Review | Pushed |
| --- | --- | --- | --- | --- |
| 0 | complete | `1fbb19b3..5be247cf` and this ledger commit | clean (Sonnet reviewer) | with this commit |
| 1 | complete | `eaf18125..30aaa015` | clean (Sonnet reviewer), 3 Minor deferred | with the ledger commit after it |
| 2 | complete | `ec6a3110..25335162` | clean (Sonnet reviewer), 3 Minor deferred | with the ledger commit after it |
| 3 | complete | `1df89a09..9a9a6e30` | clean (Sonnet reviewer), 5 Minor deferred | with the ledger commit after it |
| 4 | complete | `aa66a5ab..5f729d5d` (feature `4401609c`, fix `5f729d5d`) | clean (Opus reviewer, Sonnet re-review of the fix), 7 Minor deferred | with the ledger commit after it |
| 5–9 | not started | | | |

## Base and merges

- Cycle 05a (`01M7D5Z7FZTGTC1HE25KQ1MYQY`) closed and merged: `origin/main`
  `1f30d58c` "Integrate closed ticket 01M7D5Z7FZTGTC1HE25KQ1MYQY" (Task 0
  Step 1 met).
- Ticket branch before the merge: `8756ac56`. Merge of `origin/main` at
  `1fbb19b3` (no conflicts; `git merge-tree` was clean). The branch's
  HEAD at the start of Task 0 is **`1fbb19b3`**.
- `Cargo.lock` unchanged since `ac50ba1a`; `gpui-kit`, `gpui-base` and
  `gpui-component` all `0.6.6`.
- Boundary check after Task 0: `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing.

## Branches published

None yet. (`comment add` and `item sync` publish this ticket's branch only.)

## Pre-flight scan (orchestrator, before Task 1)

Pairs of tasks that share a file or an interface:

| Tasks | Shared file or interface | Produces → consumes | Found |
| --- | --- | --- | --- |
| 1 → 2, 4 | `view/lifecycle.rs` | T1 drops `RESOLVE_CONFLICT`, re-exports `RESUME`, `share_main`; T2 the builder; T4 the footers | Compatible. New words are `#[allow(dead_code)] // Used from Task N`, removed by Task N |
| 1 → 5, 6, 7 | `view/conflict.rs`, `render/pieces.rs` | T1 `CHOSEN`/`EDITED`, `choice_of`, `KEEPING_*` gone; T5 `conflict::open` as a sheet; T6 `close_resolve`; T7 cards | Compatible. T1's P9 compares constants only, so no card is needed yet |
| 1, 5, 7 | `keyboard_tests/conflict.rs`, `conflict/sheet.rs` | E-11 is split: T1 (`choose_by_keyboard`, its callers, the import), T5 (`in_dialog` → `in_sheet`), E-12 in T7 | Compatible. Until T5 the dialog is still a dialog, so `in_dialog` holds. Split recorded as Ruling T-4 |
| 2 → 3 → 4 | `lifecycle/view.rs`, `stage.rs`, `running.rs`, `confirm.rs`, `outcome.rs`, `made_ready.rs` | T2 stage and header; T3 rows and box; T4 footers and `(primary, element)` pairs | Compatible. `lifecycle-ending` stays inside the dialog throughout (R-17) |
| 3 → 4 | button/body helpers | T3 draws bodies as plain elements; T4 changes their return to pairs | Compatible, T4 is last to touch them. `prepare.rs`, `republish.rs`, `repair.rs`, `resumed.rs` are Task 4's but missing from the File Map (Ruling O-2) |
| 2 → 4 | `resumed.rs` busy resume | A1 classes a resume answered with no data as Ask, yet `resume_answered` draws "Still running…" for `busy` | Not a contract break. P1 must pin it (Ruling O-2) |
| 5 → 6 | `content.rs`, `settings_sheet.rs`, `submit.rs` | T5 `ResolveSheet::close` calls `close_sheet`, a details request dropped while `resolve_open`; T6 guards with `close_resolve`, adds refusals | Compatible. T5's P5 closes by Escape, Cancel, Resume and save, so it does not depend on T6's guard |
| 5, 6, 7 | `workspace/resolve_sheet_tests.rs` | P5 (T5), P6 and P7 (T6), P8 (T7) | Compatible. Each writes its own rows; T6's rows are written before any T6 code under the design checkpoint |
| 6 | `apply_review`'s refocus | Plan: skip while `workspace_sheet_open`. Merged code: an owner match (`Settings` or `AddRepository`) | The H rows govern. See Ruling O-1 |
| 4 → 8 | `keyboard_tests/lifecycle.rs`, P13 | T4 adds P13; T8 runs it by name | Compatible |
| 7 → 8 | `lifecycle/privacy_*` reads of the sheet | T7 moves the header and footer; T8's P12 reads them | Compatible. P12 "seen to fail by planting" is a plan-mandated variation of test-first, recorded there |
| 8 | private test helpers | `window_text` (`passphrase_tests.rs:538`), `ask` (`credential_tests.rs:157`) | Needs widening or a copy (Ruling O-3) |

Each task's own text against itself:

| Task | Consistent? |
| --- | --- |
| 0 | Yes. Step 6 is the orchestrator's; the implementer writes no ledger |
| 1 | Yes. Its "Verify" module run uses brace lists in places that `cargo test` does not accept; they expand to one run per module (Ruling O-4) |
| 2 | Yes. The `lifecycle-ending` read stays until Task 3 by R-17 |
| 3 | Yes. E-14 needs no edit |
| 4 | Yes. Brace lists as in Task 1. `republish_stop_tests.rs` is `operations::republish::stop_tests` |
| 5 | Yes. The lifecycle's Repair locally… wait (U-10) is checked here |
| 6 | Yes, with Ruling O-1. The design checkpoint precedes any production code |
| 7 | Yes. E-10 and E-12 are named |
| 8 | Yes, apart from the helper visibility (Ruling O-3) |
| 9 | Yes. Steps 3, 4 and 6 involve the owner; none is this session's without asking |

## Rulings

- **Ruling T-1** (commit trailer): both Task 0 commits carry
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` — the
  session's own trailer, set by the harness for this session, not the
  implementer's model. The implementer had written Opus 5.5 under the
  handoff's "your own session's trailer" rule. The two commits were
  unpushed, so the orchestrator rewrote their messages
  (`git filter-branch --msg-filter`; `da932979`/`03601ed5` became
  `6873f2c6`/`5be247cf`). Every later implementer is told to use the Sonnet
  trailer. Cost if wrong: the trailer line is amended again before the
  push.
- **Ruling T-2** (the implementer's): the design's corrections are
  annotations marked "at the merge" beside the `ac50ba1a` text, not
  rewrites. Why: Conventions pin line numbers to `ac50ba1a`, and most facts
  cite files 05a never touched. Cost if wrong: two line numbers for 05a's
  names.
- **Ruling T-3** (the implementer's): U-9 is answered "no code for 05a's
  openers"; Amendment 6's B3 note keeps the design's wording ("waits"). Why:
  the return to Settings cannot meet an open Resolve sheet (one sheet slot,
  the same-cycle deferral, the `modal_open` drop at `add_sheet.rs:313`).
  The design's wording is vacuously true. Cost if wrong: a Settings return
  is dropped by `show_settings`' new refusal instead of kept, in a case no
  way in produces. Flagged for the review-ready summary.
- **Ruling T-4** (the implementer's): E-11's Task 1 part is
  `choose_by_keyboard`'s callers (`keyboard_tests/conflict.rs:150`, `:258`)
  and the import at `:31`; its Task 5 part is the helper `tab_in_sheet`
  (`:57-60`) and the other `in_dialog` calls. Cost if wrong: a reviewer
  numbers them E-16 onward.
- **Ruling T-5** (the implementer's, U-7): `open_created` and
  `close_with_draft` need nothing beyond `focus_content`'s Task 6 gate.
  `apply_review`, `republish_once_stopped` and `open_deferred_republish` are
  Task 6's "opens by itself" list. Cost if wrong: a focus move lands behind
  the sheet and `keep_focus_in_resolve` (H10) pulls it back one frame later.
- **Ruling O-1** (`apply_review`'s refocus, Task 6): the contract is the
  H rows, not a particular predicate. The merged refocus is an owner match
  (`content.rs:140-147`). Task 6 chooses the smallest change that makes
  H-rows pass without changing 05a's behaviour for Settings and Add: add
  `Resolve(_)` to the match or OR it with `workspace_sheet_open`. R-25's
  kept Drafts and identity requests key on `resolve_open` only. Cost if
  wrong: one frame of focus behind a closing Settings or Add sheet.
- **Ruling O-2** (File Map gaps): Task 4 may edit `lifecycle/prepare.rs`,
  `republish.rs`, `repair.rs` and `resumed.rs`; Task 1 may edit
  `repair.rs:38`, `:360` (`RESOLVE_CONFLICT` becomes `REVIEW_CHANGES`);
  Task 2's P1 table gains a row for a busy resume with no lifecycle data
  (Ask, with "Still running…" kept in the body). They are named by function
  in the task text. Cost if wrong: a reviewer queries an off-map file.
- **Ruling O-3** (Task 8's helpers): Task 8 widens `window_text` and `ask`
  to `pub(super)` where needed, recorded as E-20 and E-21 (visibility only,
  no assertion changes), in preference to copying them. Why: decision 11
  pre-authorizes edits with no assertion change, and a copy would drift.
  Cost if wrong: the edit is reverted and a copy used.
- **Ruling O-4** (module runs): "the modules the task lists" are run as one
  `cargo test` per module path, brace lists expanded. The modules Task 0's
  sweep found beyond the plan's lists (the brief: "every file Task 0's sweep
  listed for it") are added to each task's end-of-task module run; they
  are in the dispatch. Cost if wrong: a longer end-of-task run.

## Task 0: baseline, references, ledger

Implementer: Opus. Report: `.superpowers/sdd/…/task-0-report.md` (not
committed). Commits: `6873f2c6` re-maps the design; `5be247cf`
applies Amendments 1 and 6.

**Gates** (`devenv shell -- cargo … --locked`, HEAD `1fbb19b3`): `check
--all-features` exit 0 in 54 s; `fmt --check` exit 0; `clippy --all-targets
--all-features -D warnings` exit 0 in 1 m 14 s. No tests run.

**U-1** (Cycle 05a's names): every name exists. 05a's code can carry B1–B3
on each of the four points: `sync_sheet` takes a `Resolve` case
(`content.rs:281-337`); `settings_key` (`settings_sheet.rs:400-410`),
`show_settings` (`:143-218`) and `show_add_repository`
(`add_sheet.rs:179-258`) each take a one-term refusal; the keep-focus gate
(`render.rs:175-183`) takes a second `if`; the waiting close
(`add_sheet.rs:111-112`, `:280-288`, `:347-358`, called at `render.rs:173`)
can be copied. Differences from the design: `settings_open` was **not**
widened (05a added `sheet_held`, `settings_sheet.rs:116-120`, beside it);
`keep_focus_in_settings` and `settings_keeps_lost_focus` are now
`keep_focus_in_sheet` and `sheet_keeps_lost_focus`; `apply_review`'s refocus is
an owner match, not `sheet_held`; `Accent` and `sheet_footer` are already in
use; `ICONS` has 57 names, not 58 (E-13: 57 to 59). All corrected in the
design, in `6873f2c6`, with the line numbers.

**U-6** (the named edits against the merged code): sites match E-1 to E-15 as
planned. No E-16 onward was found by the sweep (729 hit lines). The one
change from the design's lines: `entry_tests.rs:407-408` for E-7 (`:406-407`
before 05a added a line). The file-by-file list is in Task 0's report; each
task's dispatch carries its own files.

**U-7** (production sites that check only a dialog): 55 dialog-only sites
(fact 31's "about 50"). Those that open by themselves: `apply_review`,
`republish_once_stopped`, `open_deferred_republish` (Task 6, R-18) and
`sync_sheet`'s details request (Task 5, B1). A person's action, needs
nothing: every other site, including `open_created` and `close_with_draft`
(Ruling T-5).

**U-9** (05a's openers that wait for no dialog): the return to Settings
after Add (`add_closed`, `add_sheet.rs:316-338`), Browse…'s focus landing and
the Unlock prompt. None can meet an open Resolve sheet (Ruling T-3).
Task 6 adds no code to them.

**U-10** stays with Task 5.

**Amendments:** 1 in `docs/Waves/wave-04-workspace-design.md` (Cycle 05b,
Close ticket bullet); 6 in 05a's design B2 step 1 and B3 step 3.

**Tests run:** none (static gates only, as the plan says).

**Review:** clean. Spec compliant, no Critical or Important findings. The reviewer checked far more than 8 line citations against the tree, confirmed that no contract text changed, that E-13's "57 to 59" is true, and that the two amendments match the design. Two items it could not check from the diff are the sweep's count of 55 (the raw count of 65 `has_active_dialog` lines, 9 of them also naming `has_active_sheet`, is consistent with it) and "`content.rs` changed only in the empty pane" (plausible from the hunks).

- **Ruling O-5** (Task 4, P2 rows): the plan's Review Focus input "a failed run offering Retry and Unlock… (both last, Retry first)" cannot occur: `lifecycle/actions.rs:192-198` offers Unlock… only for a locked key and then withholds Retry (pinned at `:253-255` and in `credential_tests`), and Retry never sits beside Prepare again or Republish…. A7's table says "each when offered" and never claims the pair, so this is a plan wording defect, not a contract row. Three P2 rows replace it: Retry alone (Not now · Retry); a locked key (Not now · Open Sharing & keys… · Unlock…, no Retry); `unlock_required` with `request.prepare` (Not now · Open Sharing & keys… · Prepare again · Unlock…). Cost if wrong: the owner wants the pair possible, which is a code change in D5's rules.
- **Ruling O-6** (Task 4, P13): after Done on a closed ticket the origin control no longer exists (a closed ticket does not offer Close ticket…), so D5's `focus_lost` fallback puts focus on the closed ticket's tab in the content region (unchanged behaviour, design A8). P13 asserts focus on the action for Approve and merge… and, for Close ticket…, that the action is gone and focus is on its tab, with a comment saying why. The Wave's "focus returned to the invoking control" cannot hold literally for a close. Flag at review-ready. Cost if wrong: a production change outside A7 to keep focus on a control that is gone.
- **Ruling O-7** (Task 4, the footer wraps): the blocked prepare's footer holds five buttons (about 730 px) in a 580 px dialog with 538 px free. Without a fix the frame's right-aligned row spilled past the dialog's left edge: Close sat at x 337–407 (hidden) and Check now at 415–524 (its middle outside the dialog at 506), so a click on it hit the backdrop (`check_now_tests::way_out_check_now` failed, found by the orchestrator's module run, not by the implementer). Fix `5f729d5d`: the lifecycle footer (`lifecycle/view.rs`) wraps onto a second line, buttons still in A7's order with the action last; `frame.rs` unchanged, so no other dialog changes. Also ruled: the hint gets the id `frame-hint` in `frame.rs` (no collision: the 14 other `Footer` sites pass `hint: None`); "Still running…" stays in the footer after Close, as today. Flag at review-ready and in the on-screen checklist: the owner's eye on a two-line footer in the blocked state (frame 2e shows one line). Cost if wrong: the owner wants the dialog wider or the buttons shorter, a visual change.

## Test edits (E-n)

| E | Task | Status |
| --- | --- | --- |
| E-1 | 5 | not started |
| E-2 | 5 | not started |
| E-3 (assertion removal: `lifecycle-kind` pair) | 2 | done (Task 2): the two `lifecycle-kind` asserts deleted, nothing else weakened |
| E-4, E-5 | 2 | done (Task 2) |
| E-6 | 3 | not started |
| E-7, E-8, E-9, E-13 | 1 | done (Task 1) |
| E-10, E-12 | 7 | not started |
| E-11 | 1 (part), 5 (part) | Task 1 part done (`choose_by_keyboard`, callers, import); Task 5 part open |
| E-14 | none | no edit needed |
| E-15 | none | no edit needed (privacy sweeps keep reading the same surfaces) |
| E-16 (Task 2) | 2 | Removed the dead `Heading::branch` field and the `branch: None,` line in `operations/resume.rs`, `conflict/entry.rs` and `conflict_tests.rs`. Why: `lifecycle-branch` goes (E-3), so the field is dead and clippy `-D warnings` fails on it. No assertion changed |
| E-17 (Task 2) | 2 | `privacy_promote_tests::saved_document` made `pub(super)` so P1 reuses it. Visibility only |
| E-18 (Task 3) | 3 | `made_ready_tests.rs` compares `READY_TO_START` instead of the deleted `MADE_READY_LEAD` (Task 1 left both; retired here). The assertion is otherwise unchanged |
| E-19 (Task 4) | 4 | `keyboard_tests::lifecycle::Walk::reach_the_action` hands its steps to a new `reach_the_action_in(.., tab, ..)` so P13 can open the document as tab 1 (the original passes `("tab", 0)`). Behaviour identical, no assertion changed |
| E-20, E-21 (Ruling O-3; renumbered once Tasks 2 to 4 took E-16 to E-19) | 8 | reserved for `window_text` and `ask` visibility, if Task 8 needs them |

## Tests run, by checkpoint

| Checkpoint | Tests | Result |
| --- | --- | --- |
| Task 0 | none (static gates only: check, fmt, clippy, all green) | |
| Task 1, implementer | one `--exact` run of 12 names (P9 `conflict::copy_tests::close_and_resolve_copy_uses_the_vocabulary`, the `field_name` test, `assets::tests::every_item_view_icon_loads`, and the E-7, E-8, E-9, E-11 tests); fmt --check, check --features desktop, clippy -D warnings | 12 passed; static gates clean. P9 RED was a compile failure on the missing names (E0432), then green |
| Task 2, implementer | exact names: P1 `lifecycle::stages_tests::each_stage_has_its_title`, E-3 `lifecycle::tests::the_header_names_item_repository_and_context`, E-4 `outcome_tests::stop_at_a_safe_point_names_completed_steps`, E-5 `outcome_tests::the_operation_entry_reopens_the_dialog`; fmt, check --features desktop, clippy | 4 passed, 1 each; static gates clean. P1 RED seen (assertion: the old title "Close ae-c10s3?" against "Close ticket") |
| Task 2, orchestrator module run | one `cargo test` with filters `view::lifecycle::{tests, outcome_tests, outcome_canned_tests, privacy_tests, privacy_promote_tests, privacy_remote_tests, action_tests, save_first_tests, lock_tests, stages_tests}`, `view::keyboard_tests::lifecycle`, `view::frame::tests` | 52 passed, 0 failed, 95 s |
| Task 3, implementer | exact names: P4 `lifecycle::stages_tests::steps_show_their_state_and_progress`, the two E-6 confirm tests, `outcome_tests::running_lights_each_step_from_its_stage`, `made_ready_tests::done_lists_made_ready_with_slug_and_title`; fmt, check --features desktop, clippy | P4 RED seen (empty notes where the stage words were expected), then 4 passed on the others; static gates clean |
| Task 3, orchestrator module run | one `cargo test` over the 20 module filters the plan and Task 0 list for Task 3 (`view::lifecycle::{outcome_tests, outcome_canned_tests, made_ready_tests, made_ready_review_tests, confirm_tests, interruption_tests, no_record_tests, after_tests, privacy_tests, privacy_promote_tests, privacy_remote_tests, save_first_tests, lock_tests, steps_tests, stages_tests, tests}`, `view::operations::{resume::tests, republish::follow_tests}`, `view::keyboard_tests::lifecycle`, `view::conflict::lifecycle_resume_tests`) | 93 passed, 0 failed, 127 s. The privacy sweeps stayed green with the step notes now drawn on running and ended stages |
| Task 4, implementer | exact names: P2 `lifecycle::stages_tests::each_stage_orders_its_footer`, P3 `::the_backdrop_hides_only_a_running_dialog`, P13 `keyboard_tests::lifecycle::a_failed_run_is_retried_by_keyboard`, the 4 walk tests sharing `reach_the_action`, the 2 frame tests that draw footers, and after the fix `check_now_tests::way_out_check_now`; fmt, check --features desktop, clippy | P2, P3, P13 each RED first (no footer yet; the backdrop never hid; the dismiss read "Close", not "Not now"), then green together; the others green; static gates clean |
| Task 4, orchestrator module run 1, at `4401609c` | 28 module filters: the plan's list (`keyboard_tests::lifecycle`, `view::lifecycle::{tests, confirm_tests, stale_tests, outcome_tests, outcome_canned_tests, credential_tests, action_tests, check_now_tests}`, `operations::republish::{tests, stop_tests}`, `workspace::settings_sheet::tests`) plus Task 0's and the Task 3 modules | **1 failed** (`check_now_tests::way_out_check_now`, deterministic: left 0, right 1 at `check_now_tests.rs:74`), 142 passed, 215 s. Same failure alone by exact name. Cause and fix: Ruling O-7 |
| Task 4, orchestrator module run 2, at `5f729d5d` | the same 28 filters | 143 passed, 0 failed, 219 s. The privacy sweeps and the Tab-count test (`settings_sheet_tests/keyboard.rs:200`) stayed green |
| Task 1, orchestrator module run | one `cargo test` with the filters `desktop::model::activity::tests`, `view::conflict`, `view::keyboard_tests::conflict`, `view::activity::tests`, `view::needs_attention::tests`, `view::lifecycle::{credential_tests, tests, outcome_tests, outcome_canned_tests}`, `view::item::{posting::tests, edit_tests::review, share::tests}`, `desktop::assets::tests`, `desktop::model::{review::tests, conflict}` | 256 passed, 0 failed, 284 s |

## Task 1: copy and constants

Implementer: Sonnet. Commit `30aaa015` `feat(desktop): Close ticket and Resolve conflict copy`. Reviewer: Sonnet, spec compliant, no Critical or Important findings. Every Part C and A1–A7 constant is present with the design's words and one definition; P9 is one table over them; `field_name` has its table test with unknown keys. `choice_of` and `PartAt` are `cfg(test)`. The new lifecycle words (design A1–A7) are in `view/lifecycle.rs`, the B5 words (`TICKET_DETAIL`, `DOCUMENT_DETAIL`, `COMMENT_DETAIL`, `LAST_SHARED_COPY`) in `view/conflict.rs`, and `field_name` in `model/conflict/names.rs`, each with `#[allow(dead_code)] // Used from Task N`. `RESOLVE_CONFLICT` is gone, including `lifecycle/repair.rs:38`, `:360` (Ruling O-2).

**Test edits.** E-7, E-8, E-9, E-11 (Task 1 part) and E-13 as named. No E-16 onward, no assertion removed (E-9 and E-11 swap the state-word read for `CHOSEN` and add a `choice_of` read). The scripted-edit files' test function lists are identical before and after, by the implementer's report and the reviewer's reading of the diff.

**Carried to Task 3:** `MADE_READY_LEAD` ("Now ready:", `made_ready.rs:39`) is still the live lead; Task 1 staged `READY_TO_START` ("Now ready to start") beside it. Task 3 switches `made_ready.rs:194` to it, updates `made_ready_tests.rs:205` (which compares the old constant as a constant: an E-16 edit, no assertion change) and deletes `MADE_READY_LEAD`. The confirm's "Closed by {}" at `confirm.rs:125` also stays until Task 3 (E-6).

## Task 2: lifecycle stages, header and Git line

Implementer: Sonnet. Commit `25335162` `feat(desktop): lifecycle stages in the frame's header`. Reviewer: Sonnet, spec compliant, no Critical or Important findings. New `view/lifecycle/stages.rs` (`Stage`, `stage()`, `header()`); `running.rs` has `Shown` and `shown()`, which `stage` and `submitted_body` share (R-1); the builder calls `frame::dialog(base, header, cx)` at each render; `confirm::git_row` draws `lifecycle-git` on confirm, running and ended stages when the preview is held. Ruling O-2 applied: a resume answered with no lifecycle data (busy included) is Ask, with its body and buttons unchanged, and P1 has the row. `Tone::{Danger, Success}` lost their allows.

**Assertion removal (E-3):** the two `lifecycle-kind` asserts in `lifecycle_tests.rs`, as the design says. Call out at review-ready.

**What the owner will see.** Blocked, Held, Saving, Preparing and any reopened or resumed run without a preview no longer name the context branch anywhere, because the branch is in the Git line only and that line needs the preview (A2, K-15, E-3). Not a defect against the design; the owner's eye marker on A2 covers it. Include in the review-ready summary.

**Carried to Tasks 3 and 4.** `render` is wrapped in `frame::body(...)` for now, because `frame::dialog` zeroes the Kit padding. A2's final shape is `lifecycle-dialog` holding `frame::body(stage body)` and then `frame::footer` as its last child: Task 4 re-nests it (Task 3 moves the Git line per A6; it sits at the end of an ended body until then).

## Task 3: lifecycle step rows and outcome box

Implementer: Sonnet. Commit `9a9a6e30` `feat(desktop): lifecycle step rows and outcome box`. Reviewer: Sonnet, spec compliant, no Critical or Important findings. New `view/lifecycle/rows.rs` (211 lines): one drawer for every stage, keeping `("lifecycle-step-state", ix)` and today's words (R-2). `running.rs` 432 lines, `confirm.rs` 500. The outcome box has `lifecycle-ending` first; the message goes in the box only when no step failed (`Note::Failure` carries the rule in the type, so it is drawn once); the made-ready box says "Now ready to start". `MADE_READY_LEAD` and `STARTED` are gone. The confirm note reads "Closed by {name} · today" (or without "· today" once recorded) and no longer shows the closer's email (E-6, A4). Two ids added in production, unlabelled containers: `("lifecycle-step-row", ix)` and `lifecycle-box`, so P4 can say where `lifecycle-message` sits.

**Test edits.** E-6 (two confirm tests) and E-18 (above). No assertion removed. **Carried to Task 4:** "Open ready tickets" stays in the button row until A7's table moves it; the interim `frame::body(...)` wrapper of `render` is Task 4's to re-nest; the Git line sits where A6 puts it.

## Task 4: lifecycle footers, backdrop and the failed journey

Implementer: Opus (resumed twice: once for rulings O-5 and O-6, once for the regression). Commits `4401609c` `feat(desktop): lifecycle footers, action last` and `5f729d5d` `fix(desktop): keep Check now working in the lifecycle footer`. Reviewers: Opus on `4401609c` (spec compliant, no Critical or Important), Sonnet re-review on `5f729d5d` (approved). `footer_buttons` applies A7's rule in one place with a stable partition, so order within each group is today's by construction; `hides_on_backdrop` is read at each render; "Stopping…" is an inert Tab stop with its own handle and keeps `lifecycle-stopping`; the body and footer are re-nested as A2 says. Helper files outside the File Map (Ruling O-2): `prepare.rs`, `republish.rs`, `repair.rs`, `resumed.rs`, `actions.rs`, `made_ready.rs`.

**Test edits.** E-19 only (above). No assertion removed. P2, P3 and P13 as the plan names; O-5 and O-6 above; the Review Focus "Stop pressed", "Done after Stop", "Waiting with Close first" rows are in P2.

## Deferred minor findings

Task 0: minor (deferred): the design's U-1 closing paragraph says "Every name above exists as named" and then lists corrections; wording polish only.
Task 1: minor (deferred): `DONE` ("Done") is a second spelling beside `ENDING_DONE`, `STATE_DONE` and `outcome_label(Success)`; the roles differ and the design names it, so it stays.
Task 1: minor (deferred): `choice_of` folds a field's Mine or Theirs into `BodyChoice` (R-20 as written); a future test of `Both` or `Edited` on a field would read as `None`, which cannot arise.
Task 1: minor (deferred): `MADE_READY_LEAD` and `READY_TO_START` coexist until Task 3 retires the old one (carried to Task 3 above).
Task 2: minor (deferred): P1 has no `NotSent` or `Dismissed` rows; both sit in the same `Shown::Message` arm as `Removed`, so a reordering that moved one out would go unseen. Task 3 or the final review may add a row.
Task 2: minor (deferred): the interim `frame::body(...)` wrap of the whole render (carried to Task 4).
Task 2: minor (deferred): the context branch is no longer shown on Blocked, Held, Saving or no-preview stages (design-mandated; for the review-ready summary).
Task 3: minor (deferred): `rows.rs` draws the Running spinner at the Kit's default size where A3 says 16 px (appearance only; the report discloses it). Set the size if the Kit offers one.
Task 3: minor (deferred): P4's `message_home` searches rows `(0..16)`; deriving the index from the path would be sturdier.
Task 3: minor (deferred): the `split_once(" <")` fallback (no email in `closed_by`) has no test; E-6 covers the email form only.
Task 3: minor (deferred): P4's failure and Waiting rows use a ticket; the document kind is covered only by the held-with-preview row. The drawer is shared.
Task 3: minor (deferred): `running.rs`'s `ended()` is long, since it builds the box's `lines` vector.
Task 4: minor (deferred): P2's "no footer control drawn off the dialog" check uses `visible()`, which is true for a control only partly past the dialog's edge (the original Check now spill was partial, so it passed that check and only Close was caught). It proves "not wholly clipped", not "inside the dialog's bounds". Tighten to each wrapper's bounds within the `lifecycle-dialog` bounds if the final review wants it. `check_now_tests::way_out_check_now` pins the actual regression.
Task 4: minor (deferred): P2 groups wrapped lines by exact `f32` centre equality (a false failure, never a false pass, if rounding split a line); quantize the centre if it ever flakes.
Task 4: minor (deferred): the report says 16 other `Footer` sites; there are 14, all `hint: None`.
Task 4: minor (deferred): P2 has no row for Done after a primary (Open ready tickets · Republish… · Done); P3's failed row does not assert its stage itself; nothing pins that Enter and Space on "Stopping…" do nothing.
Task 4: minor (deferred): `stages_tests.rs` is 1434 lines and `each_stage_orders_its_footer` about 430 of them, with a conditional `if shell.present("item-done")`. The plan fixes the exact test paths, so no split.
