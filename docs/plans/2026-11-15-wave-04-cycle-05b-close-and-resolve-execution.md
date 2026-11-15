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
| 1–9 | not started | | | |

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
  to `pub(super)` where needed, recorded as E-16 and E-17 (visibility only,
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

## Test edits (E-n)

| E | Task | Status |
| --- | --- | --- |
| E-1 | 5 | not started |
| E-2 | 5 | not started |
| E-3 (assertion removal: `lifecycle-kind` pair) | 2 | not started |
| E-4, E-5 | 2 | not started |
| E-6 | 3 | not started |
| E-7, E-8, E-9, E-13 | 1 | not started |
| E-10, E-12 | 7 | not started |
| E-11 | 1 (part), 5 (part) | not started |
| E-14 | none | no edit needed |
| E-15 | none | no edit needed (privacy sweeps keep reading the same surfaces) |
| E-16 onward | | none yet |

## Tests run, by checkpoint

| Checkpoint | Tests | Result |
| --- | --- | --- |
| Task 0 | none (static gates only: check, fmt, clippy, all green) | |

## Deferred minor findings

Task 0: minor (deferred): the design's U-1 closing paragraph says "Every name above exists as named" and then lists corrections; wording polish only.
