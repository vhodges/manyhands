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
| 0 | done (no source change; no reviewer, see ruling R0-1) | ledger commit | see below |

## Base

- `origin/main` is `ba95e1b7`. `git merge origin/main` was a no-op: main was
  merged at `81789331`.
- Start of execution: branch HEAD `396db2a4`. Gates ran at `ff381f35`; the two
  later commits (`3367b9a7`, `396db2a4`) change only docs: the Cycle document
  and the handoff brief.
- `src/desktop` is unchanged since `81789331`. `Cargo.lock` is unchanged;
  `gpui-kit`, `gpui-base` and `gpui-component` are all 0.6.6 (U-6 holds).

## Authority note

`3367b9a7` (a docs commit on the branch) amends the Cycle document and the
brief to allow pushing other branches. The owner's message that started this
session still says that pushing any other branch, and `comment add` or
`ticket save` on another ticket, are not authorized without asking.
Ruling: the session's own instruction governs; this session pushes only the
ticket branch and comments only on this ticket — the amendment is not relied
on — it is the stricter reading — cost if wrong: one extra question to the
owner.

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

## Tests run

| Checkpoint | Tests run |
| --- | --- |
| Task 0 | none (static gates only) |

## Test edits (E-n)

None yet.
