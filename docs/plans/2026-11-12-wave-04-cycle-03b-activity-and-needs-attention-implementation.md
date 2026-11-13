---
title: "Wave 04 Cycle 03b Activity And Needs Attention Implementation Plan"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M79CDMK18N4YHMWP1TV498D0"
---

# Wave 04 Cycle 03b Activity And Needs Attention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task.
> **Draft, revision 5.** Revision 2 applied review round 1 and decisions
> 21–24. Revision 3 applied review round 2 (scoped re-review and
> adversarial review of `b830e8e3`): the log and the attention count are
> tables of required outcomes (design A3, A4), which Task 5 writes
> test-first with a design checkpoint. Revision 4 applied review round 3
> (scoped re-review and adversarial review of `4c596645`) and decision
> 25: retirements are redesigned, with no row derived from the attempt's
> retirement lines (R-51), and a fixed failure stops counting (N22).
> Revision 5 applies review round 4 (scoped re-review and adversarial
> review of `05246630`): "fixed" and "followed" are latched per attempt
> (R-56), subjects are exact (R-57), and the plan is re-grounded on Cycle
> 03a's merged code at `81789331`. **Revision 5 had no independent
> review.** The attention count's contract is the N-table with its
> precedence, latches and T-rows; Task 5 writes it test-first, and a row
> its implementer cannot express goes back to the orchestrator and the
> owner. Not approved; Task 0 does not start before the owner approves
> it, authorizes implementation and chooses the execution method. Re-read
> the design before each task and record every checkpoint on ticket
> `01M784Y94Q3AXT6XJX38F5VK1V`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild Activity to `03-panels.md` as one timed log of one-line
rows, each with the time the app learned of it, a plain-language message,
a repository tag, at most one button and a menu with the rest. Add the
Needs attention popover from `05`, a filtered view of the same rows,
opened from the footer, whose count is their number. Fold in the
operations split, a share classified once, and the running stage shown.

**Architecture.**

- **The store.** A pure derivation, `derive`, turns today's models into
  rows, each with the `Group` the attention pass reads. Its sources are
  requests, checking holds, Cycle 03a's repository states, the unshared
  list, retirements, pending operations, drafts and notes, plus a new
  session history of checks that brought something in (design Parts A,
  B).
- **The log.** The Workspace's `ActivityLog` gives each row the time it
  was first learned and a revision. It keeps events, lets states go when
  they resolve, holds a repository row through a check and a pending row
  while its list is unread (a Busy read, or one that removed it). Last,
  it marks the attention rows (`ActivityLog::assign_attention`), each
  thing once, with "fixed" and "followed" latched per attempt (A3, A4;
  R-56). Refreshes are coalesced, one per effect cycle.
- **The panel.** Activity draws the log in its own `uniform_list`, with a
  filter, ⋯ "Clear finished", a row menu like Cycle 03a's, and keys (C,
  D).
- **The popover.** It is a Workspace-owned layer above the footer item
  (E).
- **Tests.** Existing tests move first onto a seam that calls
  `choose_activity` by row key and label, so the drawing can be swapped
  without touching them (Testing).
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-03b-activity-and-needs-attention.md) and
[design](2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md),
with `docs/design/03-panels.md` "Activity", `05` "Needs attention
popover", `02` Footer item 5 and `06`.

- "Fact n", "S-n", "N-n", "T-n", "L-n", "P-n", "R-n", "E-n", "K-n" and
  "U-n" refer to the design.
- "Decision n" and "A-n" (an Acceptance row) refer to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method.
- Cycle 03a (`01M784Y8C73B39GR4JWENXE5D0`) is closed and merged to main.
  This is met: main `ba95e1b7`, merged into the ticket branch at
  `81789331`. Task 0 still merges main and re-checks.
- Task 0 does not start before both.
- The design's Document Amendments are applied by the planner in the
  approval commit, not by a task.

**Roles.**

- The orchestrator runs the tasks, or dispatches one implementer per task
  and then a fresh reviewer, one at a time.
- It alone writes the ledger and ticket comments and pushes the ticket
  branch.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files.

**Where.**

- Every task runs in `.manyhands/worktrees/01M784Y94Q3AXT6XJX38F5VK1V` on
  `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`.
- No lane branches and no extra worktrees: an extra worktree once made
  `ticket create` answer `recovery_required`.
- Every Rust command is `devenv shell -- cargo …` with `--locked`.

**Boundary.** A task changes `src/desktop/` and its tests. After each
task's commit, this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` before Tasks 0 and 9, and whenever main
moves. Never rebase.

**Push.** After each task's commit, the orchestrator pushes the ticket
branch with `item sync` from the main checkout (owner, 2026-11-11). It
then checks that `HEAD` equals
`origin/manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is design Part B's, verbatim, by its
copy rules, held in a `pub(crate) const`, or in a formatting function in
`D/model/activity/copy.rs` for strings with values. Tests compare
constants. Sentence case, no exclamation marks, `…` (U+2026) on a new
action that opens a dialog, sheet or page, typographic quotes around a
title. Today's offer and recovery labels are kept verbatim (R-44). No row
names a draft's own title or a file path (decision 21).

**Test runs** (`AGENTS.md`, Testing Strategy; decision 15):

- **Command.** Desktop tests run as
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- **Paths.** A test's path is its module's:
  - `desktop::model::activity::tests::<name>` for
    `D/model/activity_tests.rs`;
  - `desktop::view::activity::tests::<name>` for
    `D/view/activity_tests.rs`;
  - `desktop::view::needs_attention::tests::<name>` for
    `D/view/needs_attention_tests.rs`.
- **New test files** are included as
  `#[cfg(test)] #[path = "…_tests.rs"] mod tests;`. Use
  `all(test, feature = "desktop-test-support")` when they open a window.
- **Each run reports more than zero tests.** A command with an empty
  filter is stopped at once.
- **Between tasks**, only the tests the task wrote or changed run, by
  exact name, each named edit included, listed in full in the ledger.
  Unchanged tests are not rerun, except where a task names a doubt (Task
  1's module, Task 4's Cycle 03a tests, Task 9's E-5 and E-6 tests, whose
  origins and focus maps the swap changes). There is no whole-suite run
  before Task 13.
- **One module run.** Task 1 moves code without changing tests, and runs
  `desktop::model::operations` once, by module, because its tests'
  subject moved. That is a doubt the plan names.
- **Limits.**
  - A command past about 15 minutes is stopped, narrowed and noted.
  - Never build while a test run is in progress.
  - A new window test runs alone first.
- **Unverified facts** (U-1, U-2 and U-4 to U-8; U-3 is now fact 47)
  are checked with a scratch test that is discarded, not committed. Its
  result goes in the ledger.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.**

- Each behaviour's test is written and seen to fail before its code.
- Model tests are table-driven: one table per function.
- Focus assertions read focus inside the same update that causes the
  change (Cycle 01 ledger).
- **Task 5 has a design checkpoint.** Its implementer writes
  `attention_counts_each_thing_once` from the design's table N1–N25 (N20
  withdrawn), with N22's clauses and the latch, and
  `the_log_keeps_times_and_events` from its table T1–T18 less T8 and
  T10, which are window tests, over `derive` and `ActivityLog::refresh`,
  before any of their code. Revision 5 had no independent review, so
  these tables are the contract. If a row cannot be expressed against the
  code, the implementer stops and reports to the orchestrator, who takes
  it to the owner. The orchestrator settles it with the design before
  implementation goes on.

**No appearance tests** (Wave Testing).

- Nothing asserts colours, icons, sizes or that a view renders.
- Copy and detail are tested through their pure functions.
- The one bounds check is P21's: the reachability of a control.

**Test seams.**

- **Time.** The clock comes from `Services::with_clock` with `TestClock`
  (fact 22).
- **Polling.** Holds, attempts and the unshared list come from
  `schedule_mut()`, the scheduler `World` or `PollingBackend` answers
  (Cycle 03a's seams).
- **Window tests with polling on** answer every poll explicitly, with
  `answer_poll` and `answer_job(JobKind::PollOnce, …)` or `hold_job`
  (03a ledger).
- **Guards.** A pointer press behind a dialog lands on its overlay, so
  guard tests use key presses or dispatched actions, with a dialog and
  with a sheet.
- **Fixtures.** Fixture data directories only. No network, no real waits,
  no `chmod` fixtures.
- **The seam.** From Task 6, `D/view/activity_support.rs`'s functions.
  `act` refreshes the log synchronously (`refresh_activity_now`), panics
  unless exactly one enabled offer in `row.menu` (every offer, the
  button's included) has the label, and calls
  `Workspace::choose_activity` directly inside `workspace.update`, with
  the row's revision; it does not dispatch a window action. `act_refused`
  asserts a refusal (design Testing; R-45).

**Existing tests.**

- They change only as E-1 to E-16 say.
- Every E-7 test is left out of E-5 and E-6: it keeps pressing by element
  id until Task 9 rewrites it.
- A drawn-text read that becomes a model read (design Testing, the read
  conversion rule) is a **Removes**, listed per file in the ledger
  (decision 14).
- Any other edit is pre-authorized (decision 14). It is recorded in the
  ledger as E-17 onward with its reason and checked by the task's
  reviewer. An assertion removal is called out in the ledger and the
  review-ready summary.
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`, because the `cfg_attr(not(test))`
form warns in the test build (Cycle 02 ledger). Task N removes it.
Test-only helpers are `#[cfg(test)]`.

**Commits.** One per task, with a conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** After Tasks 0, 5, 8, 10 and 13, a comment through
`devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean. It
records:

- the commits;
- the tests run, with counts, copied from the ledger (`AGENTS.md`);
- the rulings.

Before `ticket save`, commit any owned `ticket.md` edit. `comment add`
publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-execution.md`,
created in Task 0 and written only by the orchestrator. Only the ledger
assigns E-numbers past E-16.

**File size.** New source files stay below about 500 lines.

- `D/view/workspace.rs` (488 lines) gains only the `activity` field and
  the observers' calls: the Workspace's new code goes in
  `D/view/workspace/activity.rs` and `D/view/workspace/needs_attention.rs`.
- `D/model/operations.rs` shrinks in Task 1, and `D/view/operations.rs`
  in Task 9, below 500 lines, which Task 9 checks (A13).

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **A failed share retried.** Its row keeps its time and place, its
   stored result is cleared and classified again, and it counts once
   while it is a counted failure (Tasks 2, 5: `shares_are_classified_once`,
   `the_log_keeps_times_and_events`).
2. **An unreachable repository that the scheduler retries.** Its row and
   the count hold through each check, so the popover does not flicker
   (Tasks 5, 10: `the_log_keeps_times_and_events`,
   `a_checking_repository_keeps_its_row`).
3. **A failed request whose operation is also a pending row, and two
   requests of one operation.** Each counts once (Task 5,
   `attention_counts_each_thing_once`, N1 and N2).
4. **"Clear finished" with a failure, a state, a stopped share that
   offers Give up and a republish a lifecycle waits on.** All stay (Tasks
   5, 10: `the_log_keeps_times_and_events` T9,
   `clear_finished_keeps_failures_and_states`).
5. **A popover action that opens a dialog, with Activity hidden.** Focus
   comes back to the footer item, or, if that hid before the dialog
   closed, lands on E1's control (R-49; Task 11,
   `popover_actions_by_keyboard`, and `the_popover_closes` with the
   hidden-item case).
6. **A row menu choice, click or Enter after the row changed.** Nothing
   runs; a running request's Stop stays choosable while its stage words
   change (T8, T11; R-46, R-55; Tasks 5, 6 and 9:
   `the_log_keeps_times_and_events`, `each_action_runs_its_handler`,
   `activity_by_keyboard`, `rows_by_pointer`,
   `the_row_menu_by_pointer_and_keyboard`).
7. **A title sentinel in a row.** It is allowed, while a body sentinel
   in the same row is not (Task 9, the D5 sweeps' title allowance; Task
   12, the positive control).
8. **A failure fixed another way.** A failed share followed by a later
   share of the same item that succeeds, a Resume answered
   `original_request_required` and resent, and a Can't reach row that
   leaves after it absorbed a failed Check now: each failure stops
   counting and stays shown. It stays uncounted when what fixed it is
   cleared, and counts again on a retry or when its follow-up was
   cancelled while queued (decision 25; R-56; Task 5,
   `attention_counts_each_thing_once`, N22 and N23, and
   `the_log_keeps_times_and_events`, T14–T17).
9. **A retirement.** While it runs it is its request's row, with Cancel
   or Stop; a dismissed answer is one event, never counted; any other
   answer keeps the request's row, counted (R-51; Tasks 3, 4 and 5:
   `each_source_makes_its_rows`, `attention_counts_each_thing_once`).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main (with 03a), baseline gates, re-check facts, E-sweep, ledger; checkpoint |
| 1 | F1 | Split `model/operations.rs`; re-exports; no behaviour change |
| 2 | F2, F3 | A share classified once (`classify_share`, the new getter); the stage shown (`stage_words -> String`); E-1, E-2 |
| 3 | A1, B0–B3, B5, B6 | Row types (`Group`, `Subject` with `target`, `Derived`, `ActivityRow`, `ItemRef`, `Target`), request rows and their copy, `Operations::sent`; the pure text helpers, `outcome_lines` with `Reach` and its constants, and `SHARE_MAIN` move into the model (R-41) |
| 4 | A2, A5, A7, B4 | State rows, `RepoState.cause`, `repository_detail`, the brought-in history, the retirement events with `next_retire_event` and `RetireNote` (R-51, R-59), S9's lifecycle Resume with `LifecycleRow::new` (N25), `each_repository`; Cycle 03a's panel tests by name (P12) |
| 5 | A2, A3, A4, A6 | `derive`, the log with `ActivityLog::assign_attention` last, the latch (R-56), L3b, revisions, coalesced refresh and its points, `Resends::waits_on`, the scheduler's applied-check record (R-58) and the completed-attempt count, the count; creates `D/view/activity.rs`; E-3, E-4; checkpoint |
| 6 | A1 actions, Testing | `ActivityMenuChoice`, `choose_activity`, `run_activity_action`, handler signatures, the seam; E-5 part 1 |
| 7 | Testing | E-5 part 2 onto the seam |
| 8 | Testing | The other files onto the seam; E-6; checkpoint |
| 9 | C, D, F4, G, H | The swap: the pane, rows, keys, the row menu, focus; the old drawing and the model text only it used go; the sweeps at the swap; E-5 and E-6 rerun; E-7, E-8, E-9's Activity part, E-11, E-12, E-16 |
| 10 | C1, C2 | The toolbar, the filter, ⋯ and "Clear finished"; P15's Tab clause; P32; E-10; checkpoint |
| 11 | E | The footer item and the popover; E-9's popover part, E-13 |
| 12 | G | Privacy surfaces of the new layers; E-14 |
| 13 | — | On-screen checklist, gates, review-ready; checkpoint |

**New source files:**

- `D/model/activity.rs`, with `activity/{rows,requests,states,log,copy}.rs`;
- `D/model/operations/{progress,recovery,pending,lifecycle}.rs`;
- `D/view/activity.rs` (Task 5), with `activity/{row,menu,actions}.rs`;
- `D/view/needs_attention.rs`;
- `D/view/workspace/{activity,needs_attention}.rs`;
- `D/view/activity_support.rs` (test only).

**New test files:** `D/model/activity_tests.rs`, `D/view/activity_tests.rs`
(Task 5), `D/view/needs_attention_tests.rs`. New tests in existing files
are named in their tasks.

**Order:** 0 → 1 → … → 13, one at a time. If the owner splits the Cycle,
03b-1 is Tasks 0–10 and 03b-2 is Tasks 11–13 (design Risks).

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Merge and baseline.**

```sh
git fetch origin && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 2: Re-check facts (U-6).** Revision 5 re-read Cycle 03a's
  final code at `81789331`. Run
  `git diff --stat 81789331 HEAD -- src/desktop docs/Waves docs/RFC docs/design docs/plans Cargo.lock`.
  - Re-read each fact whose file changed after `81789331`, and correct
    the design first.
  - Copy fact 46's account of Cycle 03a's deferred Minor 2 into the
    ledger.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner: facts 31–34 and U-2 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list**, over tests and production code alike:

```sh
P='src/desktop'
git grep -n -E '(^|[^_[:alnum:]])request_id\(' -- $P
git grep -n -e 'recovery_id(' -e 'draft_flush_id(' -e '"operation-area"' \
  -e '"pending-' -e 'PENDING_REVIEW' -e 'attention-action' -e '("attention",' \
  -e 'attempt-' -e 'record-note' -e 'recovered-drafts' -e 'abandon_focus' \
  -e '"drafts-unavailable"' -e 'NOTHING_PENDING' -e '"session-empty"' \
  -e '"session-section"' -- $P
git grep -n -e 'needs_attention' -e 'share_result(' -e 'FINISHING' -e 'state_label' \
  -e 'outcome_label' -e 'effects_text' -e 'scope_name' -e 'ToolbarButton::ALL' \
  -e 'Control::Attention' -e 'focus_after_hiding' -e 'keep_focus_in_footer' \
  -e 'footer-attention' -e 'WorkerEvent::Finished(' -e 'ICONS' -- $P
```

  - The first command matches calls of `request_id(` only.
    `submit_with_request_id` and the `RequestId` helpers are not sites.
    The literal patterns `request_id(id` and `request_id(request` would
    miss calls such as `request_id(refresh` and `request_id(share`.
  - Record each test site in the ledger against its E-n.
  - A test site no E-n names is a pre-authorized edit (decision 14),
    recorded as E-17 onward with its task.
  - A production site no task names goes into the task that breaks it.
  - List every caller of the helpers E-5 to E-7 name (`assert_row`,
    `close_row`, `row_of`, `under_a_resume`, `body_sheet`,
    `recovery_labels`, `retry_control`, `entry_texts`, `reopen`,
    `open_abandon`, the `Walk` methods), of the retire helpers `action`
    and `words` (`D/view/operations/retire_tests.rs:114-121`), and of
    `press_retire` (`D/view/polling_privacy_tests.rs:376`), since a
    helper's edit reaches them.
- [ ] **Step 4: Ledger, commit, push, ticket comment,** recording the base
  SHA.

---

## Task 1: Splitting `model/operations.rs` (F1)

**Files:**

- Modify: `D/model/operations.rs`.
- Create: `D/model/operations/progress.rs`, `recovery.rs`, `pending.rs`,
  `lifecycle.rs`.

**Interfaces:** none new. Every moved item is re-exported from
`operations.rs` with `pub(crate) use`, so no caller or test path changes
(R-34).

- [ ] **Step 1: Move the items.** Move the items of design F1's table to
  their files, unchanged. `pub(super)` helpers stay reachable by their
  callers.
- [ ] **Step 2: Build.** `cargo check` and `clippy`. No test file
  changes; `git diff --stat` lists only the five source files.
- [ ] **Step 3: Run the moved code's tests.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::model::operations`
  (a module filter; the one module run Global Constraints names). It
  passes, with its count recorded.
- [ ] **Step 4: Check sizes.** `operations.rs` is near 600 lines, and
  each new file is below 500. Verify lines, commit, push.

---

## Task 2: A Share Classified Once; The Stage Shown (F2, F3)

**Files:**

- Modify:
  - `D/model/operations.rs`, `D/model/operations/progress.rs`;
  - `D/services/mod.rs` (the drain);
  - `D/view/operations/outcomes.rs`, `D/view/item/share.rs`;
  - `D/view/operations.rs` (`state_label`).
- Tests:
  - `D/model/operations_tests.rs`: `shares_are_classified_once` (P9) and
    `stages_are_shown` (P10);
  - E-1, E-2.

**Interfaces:**

```rust
impl Operations {
    pub(crate) fn note_share_attempt(&mut self, id: DesktopRequestId, record: RecordState, after_give_up: bool);
    pub(crate) fn share_result(&self, id: DesktopRequestId) -> Option<&SyncResult>;
    #[cfg(test)]
    pub(crate) fn classify_share(&self, id: DesktopRequestId, record: RecordState, after_give_up: bool) -> Option<SyncResult>;
}
pub(crate) fn stage_words(stage: ProgressStage, primary: Option<&str>) -> String;
```

- [ ] **Step 1: Write the classification test (P9).**
  `shares_are_classified_once` is a table with these rows:
  - a share noted Accepted, then finished: classified from the note;
  - a retry `Queued`: the result is cleared;
  - a fabricated `Finished` with no note: `Unknown`, no Give up.

  No row changes the worker's record after `Finished`: the drain notes
  the attempt before `apply`, so such a row could not fail after F2
  (R-8).

  It fails.
- [ ] **Step 2: Implement F2.**
  - Add `share_attempts` and `share_results` to `Operations`.
  - The drain calls `note_share_attempt` beside `note_record`, before
    `apply` (fact 13).
  - `apply(Finished)` classifies once, and `apply(Queued)` clears both
    maps (R-39).
  - Rename today's three-argument `share_result(id, record,
    after_give_up)` (`D/model/operations.rs:477`) to `classify_share`,
    kept as `#[cfg(test)]`, and add the getter `share_result(&self, id)`.
  - The view's `share_result` (same signature), `finished_share`,
    `run_share_offer` and the item tab's `share_outcome` read the stored
    result.
  - `share_words` and `shown_offers` keep their live inputs, worked out
    at each refresh into `RequestFacts` from Task 5 (R-9).
- [ ] **Step 3: Edit E-1.** Its one test,
  `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`, calls
  `classify_share`, since it re-classifies under another record on
  purpose. `outcomes_route_tests.rs:91` needs no edit.
- [ ] **Step 4: Write the stage test (P10).** `stages_are_shown` covers:
  - `state_label` for each `ProgressStage` with and without `accepted`;
  - `stage_words` for each stage, with and without `primary`
    (Integrating and Publishing name it, else "main");
  - "Finishing" for accepted with no stage.

  It fails.
- [ ] **Step 5: Implement F3.**
  - `stage_words` lives in `progress.rs`.
  - `state_label`'s stage arm comes before the accepted arm.
  - The disabled "Finishing" control is unchanged for now. It becomes the
    row's button in Task 9.
- [ ] **Step 6: Edit E-2,** then run:
  - `… desktop::model::operations::tests::shares_are_classified_once`;
  - `… desktop::model::operations::tests::stages_are_shown`;
  - E-1's and E-2's tests, by name.

  Verify lines, commit, push.

---

## Task 3: Row Types, Request Rows And Their Copy (A1, S1, B0–B3, B5, B6)

**Files:**

- Create: `D/model/activity.rs` (with `mod activity;` in `D/model/mod.rs`),
  `activity/rows.rs`, `activity/requests.rs`, `activity/copy.rs`.
- Modify:
  - `D/view/operations.rs`, `D/view/lifecycle/outcome.rs`,
    `D/view/lifecycle.rs` (`STILL_RUNNING`) and `D/view/sidebar.rs` (the
    pure text helpers, `outcome_lines` with what it needs, and
    `SHARE_MAIN` move out, re-exported from their old paths; R-41);
  - `D/model/operations/retire.rs` (the read-only getters
    `running_operation` and `poll_runs`, design A2);
  - `D/model/operations.rs` (the read-only getter `sent`: the field is
    private, `:117`).
- Tests: `D/model/activity_tests.rs`.

**Interfaces** (design A1, A2):

```rust
pub(crate) enum Target { Main, Item(ItemId), Unnamed(u32) }
pub(crate) struct ItemRef { pub root: RepoKey, pub kind: AuthoringKind, pub id: ItemId }
pub(crate) struct Group {
    pub candidate: bool,
    pub operation: Option<String>,
    pub roots: Vec<RepoKey>,
    pub check: bool,
    pub check_applied: bool,
    pub superseded: bool,
    pub subject: Option<Subject>,
    pub sent: Option<u64>,
    pub outcome: Option<Outcome>,
    pub running_operation: Option<String>,
    pub followed: bool,
    pub follow_void: bool,
    pub resend_waits: bool,
}
pub(crate) enum Subject {
    Operation(String),
    Intent { family: Family, root: Option<RepoKey>, item: Option<ItemId>, target: Option<String> },
}
pub(crate) enum Family { ItemShare, MainShare, Check, Retire, Save, Own(JobKind) }
pub(crate) struct Derived {
    pub key: RowKey,
    pub kind: Kind,
    pub title: String,
    pub description: Option<String>,
    pub repo: Option<String>,
    pub button: Option<Offer>,
    pub menu: Vec<Offer>,
    pub detail: Vec<String>,
    pub attention: bool,
    pub item: Option<ItemRef>,
    pub clearable: bool,
    pub at: Option<OffsetDateTime>,
    pub group: Group,
}
pub(crate) struct ActivityRow { pub at: OffsetDateTime, pub revision: u64, pub row: Derived }
pub(crate) struct RequestFacts {
    pub stop_offer: StopOffer,
    pub shown_offers: Vec<SyncOffer>,
    pub share_words: Option<String>,
    pub already_finished: bool,
    pub review: bool,
    pub recoveries: Vec<(usize, String, bool)>,
    pub resend_waits: bool,
    pub followed: bool,
    pub follow_void: bool,
    pub check_applied: bool,
    pub superseded: bool,
}
impl Operations {
    pub(crate) fn running_operation(&self, id: DesktopRequestId) -> Option<&str>; // is_running's first source
    pub(crate) fn poll_runs(&self, operation: &str) -> bool;                       // is_running's second source
    pub(crate) fn sent(&self, id: DesktopRequestId) -> Option<u64>;                // Group.sent; the field is private
}
pub(crate) struct RequestInputs<'a> {
    pub operations: &'a Operations,
    pub requests: &'a dyn Fn(DesktopRequestId) -> RequestFacts,
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
}
pub(crate) fn request_rows(inputs: &RequestInputs<'_>) -> Vec<Derived>;
pub(crate) fn activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) -> (String, String);
```

`RowKey`, `Kind`, `Offer` and `ActivityAction` are design A1's, as
written there.

- [ ] **Step 1: Check U-1 and U-7** with a scratch test.
  - U-1: the derives `ActivityMenuChoice` will need. Add each missing
    derive to a desktop type.
  - U-7: finding an open item's model from `(root, kind, id)`, for
    `last_title()`.

  Record both.
- [ ] **Step 2: Move the pure text helpers (R-41).** Move
  `outcome_label`, `effects_text`, `scope_name`, the lifecycle outcome
  lines (`outcome_lines`, `D/view/lifecycle/outcome.rs:364`) and
  `SHARE_MAIN` (`D/view/sidebar.rs:12`) into `D/model/activity/copy.rs`
  (or `model/operations`), and re-export each from its old path.
  - With `outcome_lines` go, from `outcome.rs`: `Reach` with `Reach::of`
    (`:252-341`) and the users of its private fields (`states`, `done`),
    `Ending` and `StepState` (which `Reach::states` reads), `StepKey`,
    `lifecycle_data`, `blocked_step`, the line words (`merged`,
    `not_merged`, `SHARED`, `ALREADY_SHARED`, `SHARING_PENDING`,
    `TIDIED_UP`) and `publication_reason` and `cleanup_reason` (design
    A1).
  - The constants their `words` use go too: `Ending::words` reads
    `super::STILL_RUNNING` and the `ENDING_*` constants, and
    `StepState::words` reads `STATE_*` (`outcome.rs:48-55`, `:110-121`,
    `:232-239`). Move them into the model with the types, and re-export
    them from the view (`STILL_RUNNING` from `D/view/lifecycle.rs`).
  - The model then depends on no view module (layering).
    `D/view/lifecycle/outcome.rs` and `D/view/sidebar.rs` import no GPUI.

  No caller and no test changes. `cargo check` and `clippy`.
- [ ] **Step 3: Write the request tables.**
  - `request_copy_table` (P4): every intent of B2 in each state, with
    the templates that do not repeat the subject, and no path.
  - `share_copy_table` (P5): each `SyncOutcome` with records Accepted,
    Finished and Unknown and after a Give up, including B2's button
    precedence (risky actions never the button) and the menu's order.
  - `activity_time_formats` (P7).
  - The S1 rows of `each_source_makes_its_rows` (P1): key, kind, title,
    button, menu, `clearable`, `item`, `group`, detail lines. Including:
    - a save or create of an item with no title known reads by its short
      code, never the draft's own title (B1); the detail holds no time
      line (B5);
    - Cancel for Queued and for Running not accepted; Stop or the
      disabled "Stopping…" or "Finishing" once accepted (B2; P1);
    - a ContextRetire Queued or Running is its own S1 row, with Cancel
      or Stop (R-51); an Owed answer's row, Success envelope included,
      reads "Retiring “{title}” didn't finish", is not clearable, and
      carries "The lists are still catching up." in its detail when its
      lists are behind (A2, B2, B5);
    - a lifecycle entry's shown recovery action, one its dialog does not
      own, outranks Open… (B2);
    - the lifecycle outcome lines, from `lifecycle_answer(id)` with
      `Reach::of(envelope, None)`;
    - `clearable` withheld when `resend_waits` is set, or when an enabled
      offer other than Dismiss remains, Open… aside: a cancelled close is
      clearable (R-47). A failure's settled half is the log's (Task 5);
    - `group`: `subject`, the `Intent` subject of every request, with A3's
      families and each intent's `target` (R-52, R-57); `sent` from
      `Operations::sent(id)` and `outcome`; `operation` (an
      OperationResume's or OperationAbandon's own when `operation_of`
      gives none); `running_operation` for a Running request whose
      progress named an operation and for an operation an automatic poll
      runs (N21); `followed` and `follow_void` (N23); `check_applied` (N4)
      and `superseded` (N22 clause 4); `resend_waits`.

  All fail.
- [ ] **Step 4: Implement.**
  - The types of design A1.
  - `request_rows`, following B1, B2's table and rules, B3 and B5, with
    each row's `Group` (A1). It hides no request (R-51).
  - The getters `running_operation`, `poll_runs` and `sent` (design A2),
    read-only.
  - The title and primary lookups are closures the Workspace supplies in
    Task 5. The tests pass their own.
  - `activity_time`.
- [ ] **Step 5: Run** the four tests by name. Verify lines, commit, push.

---

## Task 4: State Rows, The Attention Cause, The Brought-In History, `each_repository` (A2, A5, A7, B4)

**Files:**

- Create: `D/model/activity/states.rs`.
- Modify:
  - `D/model/polling.rs` (`Brought`, `brought()`), `after`,
    `checked_now`;
  - `D/model/operations/retire.rs` (`RetireEvent`, `retire_events()`) and
    `D/model/operations.rs` (the counter `next_retire_event`;
    `apply(Finished)` appends the event just before its dismissal,
    `:366-368`; R-51). `PollSchedule`'s retirement lines do not change;
  - `D/view/operations/resume.rs` (`LifecycleRow` and its `of` move to the
    model, re-exported, for S9; its fields are private, `:41-45`, so the
    move adds `LifecycleRow::new`);
  - `D/model/repository_state.rs` (`AttentionCause`, `RepoState.cause`);
  - `D/model/activity/copy.rs` (`repository_detail`, R-48);
  - `D/view/repositories.rs` (`each_repository`).
- Tests:
  - `D/model/activity_tests.rs`;
  - the polling model's tests: `brought_in_is_kept_for_the_session` (P8).

**Interfaces:**

```rust
pub(crate) enum AttentionCause {
    Phase(Reason), Held, Conflict, WaitingFailed, Unreadable, Transport, Stored, Problems,
}
// RepoState gains `pub cause: Option<AttentionCause>`, set by `attention_cause`.
pub(crate) struct RepositoryInput {
    pub root: RepoKey,
    pub name: String,
    pub state: RepoState,
    pub remote: Option<String>,
    pub checking: bool,
    pub primary: Option<String>,
    pub unshared: Unshared,
    pub sharing: BTreeMap<Target, ShareState>,
    pub retiring: BTreeSet<ItemId>,
    pub retire_offers: Vec<RetireOffer>,
}
pub(crate) enum ShareState { Ready, Sharing, Locked(String) }
pub(crate) struct DraftsInput {
    pub flush_failures: Vec<(DraftId, ItemRef, String, DraftErrorKind)>,
    pub unavailable: Option<DraftStoreUnavailable>,
    pub recovered: Option<usize>,
}
pub(crate) struct StateInputs<'a> {
    pub operations: &'a Operations,
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],
    pub drafts: &'a DraftsInput,
    pub conflict_waits: &'a dyn Fn(&RepoKey, &str) -> bool,
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
    pub unlock_target: &'a dyn Fn(&KeyId) -> bool,
    pub lifecycle_running: &'a dyn Fn(&RepoKey, &ItemId) -> bool,
}
pub(crate) fn state_rows(inputs: &StateInputs<'_>) -> Vec<Derived>;
pub(crate) fn repository_detail(primary: Option<&str>, remote: Option<&str>) -> Vec<String>;
pub(crate) struct RetireEvent {
    pub seq: u64,  // from Operations.next_retire_event
    pub sent: u64, // the producing request's Operations::sent (N22 clause 5)
    pub root: RepoKey,
    pub kind: AuthoringKind,
    pub item: ItemId,
    pub verdict: Verdict,
    pub changed: bool,
}
// Operations gains the field `next_retire_event: u64`, RetireEvent.seq's counter.
impl Operations {
    pub(crate) fn retire_events(&self) -> &[RetireEvent];
}
impl LifecycleRow {
    pub(crate) fn new(kind: ItemDtoKind, item: String, operation: String) -> Self; // ResumeLifecycle rebuilds the row
}
// RowKey gains `RetireNote { root: RepoKey, item: ItemId }` (design A1; R-59).
pub(crate) fn each_repository<R>(workspace: &Workspace, cx: &App, f: impl FnMut(&RepoFacts<'_>) -> R) -> Vec<R>;
```

- [ ] **Step 1: Check U-5.** Does `checked_now`'s in-flight branch return a
  summary? Record it.
- [ ] **Step 2: Write `brought_in_is_kept_for_the_session` (P8).**
  - An automatic check and a Check now that brought in, through `after`
    and `checked_now`.
  - Ones that brought in nothing, and a failed one.
  - The sequence numbers, `at` and `automatic`.

  It fails. Implement A5.
- [ ] **Step 3: Add the attention cause** (A7; R-5). `RepoState` gains
  `cause`, set by `attention_cause`, one `AttentionCause` variant per
  cause of fact 23. Cycle 03a's model tests compare `RepoState` field by
  field, so they need no edit. Run them by name:
  - `… desktop::model::repository_state::tests::each_state_and_its_precedence`;
  - `… desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts`;
  - `… desktop::model::repository_state::tests::menu_items_follow_the_state`;
  - `… desktop::model::repository_state::tests::check_all_takes_each_repository_that_can_check`.

  Any edit one needs is recorded as E-17 onward.
- [ ] **Step 4: Move `RepoFacts` building into `each_repository`** (A7;
  R-5), unchanged. `rows()` calls it. Rerun Cycle 03a's panel tests by
  name (P12), since their code moved:
  - `… desktop::view::repositories::tests::rows_and_list_states`;
  - `… desktop::view::repositories::tests::titles_of_what_is_not_shared`;
  - `… desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard`;
  - `… desktop::view::repositories::tests::check_all_checks_each_repository`;
  - `… desktop::view::repositories::tests::the_more_menu_acts_on_the_chosen_repository`.

  They pass unchanged.
- [ ] **Step 5: Write the state tables.**
  - `state_copy_table` (P6): each row of B4, the dispositions, the total
    pending-action table, the next actions, and `repository_detail`:
    starting at "Main branch: {primary}" when known, without the state's
    sentence, which is the description, then "Shared copy: {remote}" by
    name only, and never the folder or the location (R-48).
  - The S2–S14 rows of `each_source_makes_its_rows` (P1), including:
    - each S4 state and cause that makes a row and each that does not
      (R-14, R-15, R-40), with Locked split by its `RowAction`:
      `Unlock(_)` makes no row, `CheckNow` makes one;
    - S3's Unlock… enabled or disabled by `unlock_target`, and its
      `group.roots` naming every root the hold covers;
    - S5 and S6 from the unshared list, keyed by `Target` (R-17);
    - S6's `ShareState` (Sharing… and locked, disabled), S7 from
      `retire_offers`, and S7's `retiring`;
    - S8 from `retire_events()`, through `Operations::apply(Finished)`
      for each verdict: Retired, Attention, Unreachable and Retry each
      append one event with its `seq` and `sent`; Owed, refused,
      preserved and failed append none; nothing read from
      `PollSchedule`'s retirement lines (R-51). Retired: keyed
      `Retired(seq)`, a later retirement of the same item a new key, a
      success row, clearable, never a candidate, with `Group { subject:
      Intent { Retire, root, item, None }, sent, outcome: Success }`
      (N22 clause 5). Attention, Unreachable and Retry: keyed
      `RetireNote { root, item }`, the latest per item, information rows,
      clearable, never candidates (R-59);
    - S9's lifecycle parent (`LifecycleRow::of`): Resume as
      `ResumeLifecycle`, or a disabled "Still running…" and no candidate
      while `lifecycle_running` holds (N25);
    - S11 named by B1's rule from `item_title()`, "the ticket {short}"
      when it is the ID;
    - S13's and S14's Dismiss as `DismissRecovered` and `DismissNote`.

  All fail.
- [ ] **Step 6: Implement `state_rows`.** Implement it from design A2 and
  B4, with `repository_detail`, the retirement events (`RetireEvent`,
  appended by `apply` with `next_retire_event`; no `PollSchedule`
  change; R-51, R-59) and S9's lifecycle Resume, with `LifecycleRow`
  moved to the model and given `LifecycleRow::new`. Run the tests by
  name. Verify lines, commit, push.

---

## Task 5: Derivation, Attention, The Log, The Count (A2, A3, A4, A6)

**Files:**

- Create:
  - `D/model/activity/log.rs`;
  - `D/view/workspace/activity.rs`;
  - `D/view/activity.rs`, declared in `D/view/mod.rs`, with its test file
    `D/view/activity_tests.rs`, for `count_and_popover_agree` and
    `refreshes_are_coalesced`.
- Modify:
  - `D/model/activity.rs` (`Inputs`, `derive`);
  - `D/services/resends.rs` (`Resends::waits_on`, read-only; design A2);
  - `D/model/polling.rs` (the per-root count of completed attempts,
    incremented in `after` and `checked_now` on a `Completed` answer,
    and its reader; N22 clause 4);
  - `D/services/scheduler/entity.rs` (the `Scheduler` struct, `:77-139`:
    the applied-check set, the count noted per failed Check now, and
    their readers) and `D/services/scheduler/attention.rs` (`check_failed`
    adds to the set beside `self.schedule.after`, `:120`, and notes the
    count; R-58);
  - `D/view/workspace.rs` (the observers call `refresh_activity`; the
    field `activity: ActivityLog`);
  - `D/view/workspace/footer.rs` (`attention()` reads the log);
  - the refresh points of A4: `D/view/workspace/requests.rs` (pending
    conflicts land), the open item panes' title loads, `ShareTitles`, the
    navigation updates.
- Delete: `D/model/attention.rs` with its tests (E-3).
- Tests:
  - `D/model/activity_tests.rs`: `attention_counts_each_thing_once` (P2,
    the N-table) and `the_log_keeps_times_and_events` (P3, the T-table
    less T8 and T10);
  - the polling model's and the scheduler's tests: the completed-attempt
    count and the applied-check record, each seen to fail first;
  - `D/view/activity_tests.rs`: `count_and_popover_agree` (P13, its
    count half) and `refreshes_are_coalesced` (P3's T10);
  - E-3, E-4.

**Interfaces:**

```rust
pub(crate) struct Inputs<'a> {
    pub operations: &'a Operations,
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],
    pub drafts: DraftsInput,
    pub requests: &'a dyn Fn(DesktopRequestId) -> RequestFacts,
    pub conflict_waits: &'a dyn Fn(&RepoKey, &str) -> bool,
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
    pub unlock_target: &'a dyn Fn(&KeyId) -> bool,
    pub lifecycle_running: &'a dyn Fn(&RepoKey, &ItemId) -> bool,
}
pub(crate) fn derive(inputs: &Inputs<'_>) -> Vec<Derived>;
// ActivityLog gains `settled: BTreeMap<RowKey, u64>`; `absorbed` is per attempt (design A3, A4; R-56).
impl ActivityLog {
    // Called only by `refresh`, last, over the final rows; reads and writes `absorbed` and `settled` (design A3).
    fn assign_attention(&mut self);
    pub(crate) fn refresh(&mut self, derived: Vec<Derived>, checking: &BTreeSet<RepoKey>, pending_held: &BTreeSet<RepoKey>, now: OffsetDateTime) -> bool;
    pub(crate) fn rows(&self) -> &[ActivityRow];
    pub(crate) fn attention_count(&self) -> usize;
    pub(crate) fn dismiss(&mut self, key: &RowKey);
    pub(crate) fn clearable(&self) -> Vec<RowKey>;
    pub(crate) fn still_offered(&self, key: &RowKey, revision: u64, action: &ActivityAction, label: &str) -> bool; // T11, T12
}
impl Resends {
    pub(crate) fn waits_on(&self, id: DesktopRequestId) -> bool; // forget(id) would drop something
}
impl PollSchedule {
    pub(crate) fn completed_attempts(&self, root: &Root) -> u64; // completed attempts applied to `root` (N22 clause 4)
}
// Scheduler gains `checks_applied: BTreeSet<DesktopRequestId>`, added to in `check_failed` when it
// applies the answer, and `checks_heard: BTreeMap<DesktopRequestId, u64>`, the root's
// `completed_attempts` when `check_failed` heard the failure.
impl Scheduler {
    pub(crate) fn check_applied(&self, id: DesktopRequestId) -> bool;    // RequestFacts.check_applied (R-58)
    pub(crate) fn check_superseded(&self, id: DesktopRequestId) -> bool; // RequestFacts.superseded: the count grew since
}
impl Workspace {
    pub(in crate::desktop::view) fn refresh_activity(&mut self, cx: &mut Context<Self>);
    pub(in crate::desktop::view) fn refresh_activity_now(&mut self, cx: &mut Context<Self>);
    pub(crate) fn activity(&self) -> &ActivityLog;
    #[cfg(test)]
    pub(in crate::desktop::view) fn activity_derivations(&self) -> usize; // T10's counter
}
```

- [ ] **Step 1: Design checkpoint: both tables' tests first.**
  - Write `attention_counts_each_thing_once` from N1–N25 (N20
    withdrawn), one table row per N-row, with an N4 row for each failure
    class, over `derive` and `ActivityLog::refresh`. It includes A3's
    precedence, N2 by operation else subject, N22's five clauses with the
    latch (decision 25; R-56, R-57), N23's resent Resume and its undoing,
    N24, N25, and N9's repository row beside its problems.
  - Write `the_log_keeps_times_and_events` (P3) from T1–T18 less T8 and
    T10, one table row per T-row, over `ActivityLog::refresh` with an
    injected `now`. T5 and T7 (L3b) check every offer disabled, and that
    the kept row leaves with its repository; T9 includes a stopped share
    whose record is Accepted, a stopped republish a lifecycle waits on
    (R-47), a failure settled under N22, an S8 Retired event, an S8
    information event, a cancelled close and an Owed retirement; T11 and
    T12 run through the log's `still_offered`, which `choose_activity`
    calls in Task 6, and T11 includes a Stop whose stage words changed
    (the revision holds); T14–T17 are the latch's cases and T18 a
    replaced information event (R-59).

  Revision 5 had no independent review, so these tables are the
  contract. If a row of either table cannot be expressed against the
  code, stop and report to the orchestrator. Both fail.
- [ ] **Step 2: Record the checkpoint.** The orchestrator takes any
  reported row to the owner and settles it with the design before Step
  3, and records each in the ledger.
- [ ] **Step 3: Implement the derivation and the log.**
  - `derive`: `request_rows` plus `state_rows`. It hides no row (R-51)
    and assigns no attention.
  - The completed-attempt count in `PollSchedule` and the scheduler's
    applied-check record (R-58), each with its test seen to fail first.
  - `ActivityLog`, by A4's rules L1–L9, L3b included with its two
    triggers (R-54), with the log-wide revisions (R-46) and `settled` and
    `absorbed` per attempt (R-56), then `ActivityLog::assign_attention`
    last over the final rows, reading each row's `Group` (N1–N25, in A3's
    precedence; R-52, R-53, R-57), then `clearable`'s first condition and
    its settled half (R-47).
  - `Resends::waits_on(id)`: whether `forget(id)` would drop a chain link
    or a waiting republish (design A2; `D/services/resends.rs:173-178`).
- [ ] **Step 4: Wire the Workspace.**
  - Create `D/view/activity.rs`, declared in `D/view/mod.rs`, holding for
    now only its test module.
  - `refresh_activity` marks the log dirty and defers one
    `refresh_activity_now` per effect cycle (`cx.defer`), so a burst of
    notifications derives once (R-43; T10).
  - `refresh_activity_now` builds `Inputs`: `RequestFacts` from Services
    and the worker, with the Busy words and republish state worked out
    here (R-9), `resend_waits` from `Services::resends().waits_on(id)`,
    `followed` and `follow_void` from `Services::resends().follow(id)`
    and the follow-up's state (N23), `check_applied` from
    `Scheduler::check_applied(id)` (N4; R-53, R-58) and `superseded` from
    `Scheduler::check_superseded(id)` (N22 clause 4); titles from an open
    item's `last_title()` and `share_title`; `each_repository`;
    `conflict_waits` from `pending_conflicts`; `unlock_target` from
    `Scheduler::unlock_target`; `lifecycle_running` from
    `worker.in_flight` with `lifecycle_of` (S9); the clock's `utc()`. It
    also builds `refresh`'s `checking` set from `RepositoryInput.checking`
    and `pending_held` from L3b's triggers, both over the listed
    repositories only. It counts its derivations in a test-only counter
    (T10).
  - It is called from every refresh point of A4, including pending
    conflicts landing and an open item's title loading.
  - `attention()` returns `attention_count()`.
- [ ] **Step 5: Delete the old count.** Delete `needs_attention` and its
  tests (E-3); move their cases into Step 1's table, listed in the ledger.
  `retire_resume_resends_original` builds an `ActivityLog` from `derive`
  over the `World`'s operations and schedule, with no repository inputs,
  calls `refresh`, and reads `attention_count()`. Apply
  E-4 to every reader it names, recording any changed count with its
  N-row.
- [ ] **Step 6: Write the window tests.** `count_and_popover_agree`
  (P13), for now comparing the footer count with the log's attention rows
  through a journey of each counted kind, each then resolved. Each step
  asserts the expected list of attention `RowKey`s, written from the
  N-table, as well as the count. Then `refreshes_are_coalesced` (T10):
  a burst of notifications in one effect cycle makes one derivation,
  read through `activity_derivations()`. Each runs alone first.
- [ ] **Step 7: Run** the new tests and every E-3 and E-4 test by name.
  Verify lines, commit, push.
- [ ] **Step 8: Checkpoint.** Ticket comment: Tasks 1–5, the N-table, the
  T-table and anything settled with the owner and the design at its
  checkpoint, the tests run with counts, the rulings so far.

---

## Task 6: Actions And The Test Seam (E-5 Part 1)

**Files:**

- Modify:
  - `D/keys.rs` (`ActivityMenuChoice`);
  - the handlers whose signatures change (design A1): `D/view/operations.rs`
    (`open_lifecycle`), `D/view/operations/review.rs` (`review_request`,
    `review_pending`), `D/view/workspace/requests.rs` (`run_recovery`;
    `abandon` already takes an origin, `:498-503`),
    `D/view/operations/outcomes.rs` (`run_share_offer`, whose Republish…
    origin comes today from `control_focus_for`, `:317`),
    `D/view/operations/retire.rs` (`retire_chosen`),
    `D/view/operations/resume.rs` (`resume_lifecycle`, which already takes
    an origin, made reachable) and
    `D/view/workspace/repository_menu.rs` (`row_action`);
  - `D/view/activity.rs` (`mod actions;`);
  - `D/view/mod.rs` (`activity_support`).
- Create:
  - `D/view/activity/actions.rs` (`choose_activity`,
    `run_activity_action`);
  - `D/view/activity_support.rs` (test only).
- Tests:
  - `D/view/activity_tests.rs`: `each_action_runs_its_handler` (P18,
    through `choose_activity`);
  - E-5 part 1: `operations_tests.rs`, `give_up_tests.rs`,
    `outcomes_tests.rs`, `outcomes_route_tests.rs`, `check_tests.rs`,
    `record_tests.rs`.

**Interfaces:**

```rust
#[derive(Clone, PartialEq, Debug, gpui_kit::Action)]
#[action(namespace = manyhands, no_json)]
pub(crate) struct ActivityMenuChoice { pub key: RowKey, pub action: ActivityAction, pub shown: String, pub revision: u64 }

impl Workspace {
    pub(in crate::desktop::view) fn choose_activity(&mut self, choice: &ActivityMenuChoice, origin: FocusHandle, window: &mut Window, cx: &mut Context<Self>);
    pub(in crate::desktop::view) fn run_activity_action(&mut self, action: &ActivityAction, origin: FocusHandle, window: &mut Window, cx: &mut Context<Self>);
}
// activity_support.rs
pub(crate) fn row(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) -> Option<ActivityRow>;
pub(crate) fn rows(shell: &Shell, cx: &mut TestAppContext) -> Vec<ActivityRow>;
pub(crate) fn offered(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) -> Option<Offer>;
pub(crate) fn act(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext);
pub(crate) fn act_refused(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext);
pub(crate) fn request(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> DesktopRequest;
pub(crate) fn pending_key(shell: &Shell, root: &str, action: &str, cx: &mut TestAppContext) -> RowKey;
```

- [ ] **Step 1: Write the seam test.** `each_action_runs_its_handler`
  covers each `ActivityAction` kind through `choose_activity`,
  `ResumeLifecycle` included. Each sends exactly what today's control
  sends, read from the recording backend or the dialog or sheet it opens.
  It also has:
  - a choice whose revision is stale (T11): an `ActivityMenuChoice`
    built with an old revision and passed to `choose_activity` directly,
    not through `act_refused`;
  - one whose offer is gone or disabled (T12), through `act_refused`.

  Nothing runs for either. It fails.
- [ ] **Step 2: Implement the choice.**
  - `choose_activity` checks the row again (design D) through
    `ActivityLog::still_offered`: the same revision, and an enabled offer
    with the same action and label. Otherwise nothing runs (R-46).
  - `run_activity_action` maps each action to its handler (A1's table),
    passing the origin.
  - The handlers of A1's signature changes gain an `origin: FocusHandle`
    parameter, or are made reachable (`pub(in crate::desktop::view)`).
  - For now the origin is today's control handle for that action
    (`control_focus_for` and `abandon_focus_for` by the old id), so
    converted tests that read focus stay green (R-45). Task 9 replaces it
    with the list's.
- [ ] **Step 3: Write the seam** functions above. `act` cannot pass
  vacuously:
  - it first refreshes the log synchronously, calling
    `refresh_activity_now` directly;
  - it panics unless exactly one enabled offer in `row.menu` has that
    label. `row.menu` holds every offer, the button's included, so the
    button is not counted twice;
  - it builds the `ActivityMenuChoice` for the offer, with the row's
    current revision, and calls `choose_activity` inside
    `workspace.update`.

  `act_refused` asserts that the offer is absent or disabled, and that
  `choose_activity` runs nothing. Neither dispatches a window action:
  `Window::dispatch_action` runs deferred on the focused node, and
  `FocusHandle::dispatch_action` does nothing for an undrawn handle
  (design Testing).
- [ ] **Step 4: Move E-5 part 1's tests onto the seam.** Move them file
  by file, following design Testing's read conversion rule. Before each
  file, record its test function list in the ledger; after, record it
  again, check that they are equal, and list each removed drawn-text
  assertion. Keyboard and focus steps, and every E-7 test, are untouched.
- [ ] **Step 5: Run** every E-5 part 1 test by name, and the new test.
  All pass, with the old columns still drawn. Verify lines, commit, push.

---

## Task 7: The Rest Of `D/view/operations/` Onto The Seam (E-5 Part 2)

**Files:** E-5 part 2: `attempts_tests.rs`, `attention_tests.rs`,
`republish_follow_tests.rs`, `republish_stop_tests.rs`,
`republish_tests.rs`, `resume_tests.rs`, `retire_tests.rs` and
`retiring_tests.rs`, their tests outside E-7 only. (`retiring_tests.rs`'
one test, `retire_rows_hit`, is in E-7, so Task 0's sweep confirms it has
nothing here.)

- [ ] **Step 1: Move the tests.** Move E-5 part 2's tests file by file,
  as in Task 6 Step 4, with the function lists and removed drawn-text
  assertions in the ledger. `resume_tests.rs`' lifecycle Resume presses
  become `act(.., "Resume")` on the pending row (S9's
  `ResumeLifecycle`); their meaning is unchanged.
- [ ] **Step 2: Run** every E-5 part 2 test by name. All pass. Verify
  lines, commit, push.

---

## Task 8: The Other Files Onto The Seam (E-6)

**Files:** the files of E-6.

- [ ] **Step 1: Move the tests.** Move E-6's tests file by file, as in
  Task 6 Step 4, with the function lists and removed drawn-text
  assertions in the ledger.
  - `settings/repositories_tests.rs`' literal id (`:569`) goes with the
    rest.
  - `p5_a_close_stranded_at_closing_resumes_after_relaunch` presses
    Resume with `act(.., "Resume")` on the pending row (S9's
    `ResumeLifecycle`); its meaning is unchanged.
  - D5's sweeps keep reading the drawn area until Task 9.
  - Every E-7 test is left out.
- [ ] **Step 2: Run** every E-6 test by name. All pass. Verify lines,
  commit, push.
- [ ] **Step 3: Checkpoint.** Ticket comment: Tasks 6–8, the moved tests
  with counts, the removed drawn-text assertions, the rulings so far.

---

## Task 9: The Swap: The Activity Panel And Its Row Menu (C, D, F4, G, H)

The largest task. Before it, merge `origin/main`.

**Files:**

- Create:
  - `D/view/activity/row.rs`;
  - `D/view/activity/menu.rs`.
- Modify:
  - `D/view/activity.rs` (the pane and its list);
  - `D/model/activity/copy.rs` (`tooltip_lines`, B5);
  - `D/keys.rs` (`ACTIVITY_CONTEXT` and its bindings), with
    `D/keys_tests.rs` (E-9's Activity part);
  - `D/view/workspace/render.rs` (`activity_region`; mount the row menu
    layer; the menu's `ActivityMenuChoice` handler, through
    `unless_dialog`, calls `choose_activity` with the list's handle);
  - `D/view/workspace/activity.rs` (the row menu, focus);
  - `D/view/workspace/requests.rs`, `D/view/workspace/lifecycle.rs`,
    `D/view/operations/review.rs`, `D/view/operations/outcomes.rs` and
    `D/view/operations/resume.rs` (origins: C3, `run_share_offer`'s
    Republish… and `resume_lifecycle` included);
  - `D/view/activity_support.rs` (the seam's origins become the list's
    handle);
  - `D/view/operations.rs` and its submodules (the drawing goes: F4);
  - `D/model/polling/entry.rs` (the text only the old drawing used goes,
    or is gated `#[cfg(test)]`: F4);
  - `D/view/workspace.rs` (the focus maps of fact 8 go);
  - the sweeps at the swap: `D/view/lifecycle/privacy_tests.rs`,
    `D/view/polling_privacy_tests.rs`, `D/view/drafts_privacy_tests.rs`,
    `D/view/drafts_entry_tests.rs`;
  - `D/assets_tests.rs` (E-16).
- Tests:
  - `D/view/activity_tests.rs`: `rows_draw_from_the_log_and_filter`
    without the filter (P14), `activity_by_keyboard` (P15, all but its
    Tab clause, which is Task 10's), `rows_by_pointer` (P16),
    `the_row_menu_by_pointer_and_keyboard` (P17),
    `a_dialog_from_a_row_returns_focus_to_its_row` (P20);
  - `each_action_runs_its_handler` extended to the drawn button and the
    menu (P18);
  - E-7, E-8, E-9's Activity part, E-11, E-12, E-16;
  - E-5's and E-6's tests, rerun by name: a named doubt (Global
    Constraints, Test runs), since the swap changes the origins `act`
    passes and deletes the focus maps.

- [ ] **Step 1: Check U-2 and U-4** with a scratch test:
  - `uniform_list` with a tracked scroll handle draws and scrolls the
    rows;
  - a key bound in `ACTIVITY_CONTEXT` reaches its handler while the list
    holds focus.

  Record both. If U-2 fails, use the Kit's `VirtualList` with the same
  keys, and record the amendment.
- [ ] **Step 2: Write the list and key tests.** Write P14 (without the
  filter; the tooltip text through `tooltip_lines`), P15 (without its Tab
  clause; including Enter on the focused row button running its action
  once, with the list's `enter` not also firing, Enter doing nothing on a
  disabled button or a row with neither, and focus moving to the list
  when the highlighted row leaves while its button holds focus), P16, the
  drawn revision in
  both (T8: a click or Enter on a row drawn before the log's row took a
  newer revision runs nothing; R-55) and P20 (Republish…, a recovery that
  confirms, Open… and a lifecycle's Resume from the button, the menu and
  Enter; Abandon… from the menu; decision 23). Each runs alone first.
  They fail.
- [ ] **Step 3: Build the pane and rows** (C1, C2).
  - One-line rows, 28 px; the message and the tag end in an ellipsis,
    and the button never leaves the row (decision 22).
  - The tooltip and accessible description are `tooltip_lines`: the full
    message, the time line (B6), then the detail (B5).
  - The rows are applied after the frame, and the highlight follows the
    key, else the row now at its position, else the last row; moving it
    calls `scroll_to_item`. A click or Enter builds its choice from the
    drawn row, the pane's applied one, with its key and revision, not
    the log's current row (C2; R-55).
  - When the highlighted row's button held focus and the row left, or
    the row now at its position has no button, focus moves to the list
    (C2).
  - The empty state "Nothing yet this session".
  - The container keeps `"operation-area"` (R-20).
  - Mount it in `activity_region` below the notices and the quit row
    (R-18). Remove the old columns' drawing (F4).
  - **Dead code** (F4). The removal leaves these unused in the non-test
    build: `AttemptEntry`, `attempt_entry`, `EntryLine`, `LineAction`;
    `RetireLine::text`, `retiring_text`, `finished_text`,
    `unfinished_text`; `offer_text`, `RetireOffer::text`. Outside tests
    only `D/view/operations/attempts.rs` (`:40`, `:74`, `:99`) and
    `D/view/operations/retire.rs:120` use them. Delete each, or gate it
    `#[cfg(test)]` where tests still import it
    (`D/view/operations/retiring_tests.rs:18`,
    `D/view/operations/retire_tests.rs:21`,
    `D/model/polling_retiring_tests.rs:122`,
    `D/model/polling/entry_tests.rs`,
    `D/services/scheduler_retire_tests.rs:1300`). Any that S7 or S8
    reuse moves to `D/model/activity/copy.rs` instead.
- [ ] **Step 4: Write the menu tests.** Write P17 and P18's button and
  menu halves. They fail.
- [ ] **Step 5: Build the row menu** (Part D), on Cycle 03a's pattern:
  - a right press;
  - `shift-f10` and `menu`;
  - the full message, then the detail lines, first;
  - the choice carries the row's revision when the menu opened (R-46);
  - the re-check through `choose_activity`;
  - dropped under a modal and when its row leaves;
  - one layer at a time (R-13).
- [ ] **Step 6: Wire focus (C3; decision 23).**
  - Origins and restores become the list's handle, with the highlight on
    the row's key, from the button, the menu and Enter alike,
    `run_share_offer`'s Republish… included. R-45's interim origins end,
    in the seam too.
  - "Show it" and Drafts not saved's Cancel highlight their rows.
  - Delete the focus maps, `prune_abandon_focus` and
    `Workspace::abandon_focus`.
- [ ] **Step 7: The sweeps at the swap** (E-11, E-12; design Part G).
  - D5's sweeps: `TITLE` and `READY` are allowed inside
    `"operation-area"`; the other sentinels stay forbidden everywhere.
  - `polling_privacy`: the same title allowance; `SURFACES` updated
    explicitly; the model-rows surface, which keeps `read_at`'s
    `("operation entry", "worktree_not_clean")` pair, now from the row's
    detail (`:997`); the folder sentinel and the remote URL's path and
    user-info sentinel (P28).
  - The drafts sweeps expect S11's title, `D/view/drafts_entry_tests.rs:207-212`
    included (E-11).
- [ ] **Step 8: Edit E-7, E-8, E-9's Activity part and E-16.** For E-7,
  rewrite the keyboard and focus steps to Part H's keys. Record each
  rewritten test's steps in the ledger. In `retire_rows_hit`, the drawn
  retirement-line text assertions (`retiring_tests.rs:113-126`) are
  **Removes**, listed in the ledger, since R-51 no longer draws those
  lines; its Tab and Dismiss step moves to the refused request's S1 row.
- [ ] **Step 9: Check the size.** `D/view/operations.rs` is below 500
  lines (A13); record its count.
- [ ] **Step 10: Run** every new and edited test by name, with
  `polling_privacy` and D5's four `a16_` tests, and every E-5 and E-6
  test by name (the named doubt above). Verify lines, commit, push.

---

## Task 10: The Toolbar And The Rest Of Activity (C1, C2)

**Files:**

- Modify:
  - `D/view/dock/header/toolbar.rs` and `D/view/workspace/focus.rs`
    (`ToolbarButton::ActivityFilter`, `ActivityMore`);
  - `D/view/workspace/activity.rs` (the filter, the ⋯ menu);
  - `D/view/workspace/render.rs` (mount the ⋯ menu layer).
- Tests:
  - `D/view/activity_tests.rs`: `rows_draw_from_the_log_and_filter`
    extended to the filter (P14), `clear_finished_keeps_failures_and_states`
    (P19), `rows_fit_a_narrow_activity` (P21),
    `activity_hidden_keeps_the_log` (P29),
    `a_checking_repository_keeps_its_row` (P30),
    `navigation_sends_nothing` (P31, its Activity part),
    `reminders_match_to_share` (P32);
  - `activity_by_keyboard` extended to P15's Tab clause (filter, ⋯, list,
    highlighted row's button);
  - E-10.

- [ ] **Step 1: Write the tests.** Write P14's filter part, P15's Tab
  clause, P19 (a success, a stopped share whose record is Accepted, a
  stopped republish a lifecycle waits on, a failure, a failure settled
  under N22 that still offers Retry, an attention row, a state, an S8
  Retired event, an S8 information event and a cancelled close, which
  offers Open…: only the success, the settled failure, the two S8 events
  and the cancelled close go; R-47),
  P21 (Activity moved into the right dock and set to 320
  px with the layout model's API, as Cycle 01's dock tests do, with the
  longest message of each kind, a long repository name and "+N", each
  row scrolled into view with `scroll_to_item` before its bounds are
  read), P29, P30, P31's Activity part (the highlight, the filter, the
  row menu and ⋯, with the recording backend) and P32 (the footer's "N
  to share", Cycle 03a's `ShareFacts` total,
  `D/view/workspace/footer_status.rs:59`, equals the number of S6 rows of
  that moment, main counted as one). Each runs alone first. They fail.
- [ ] **Step 2: Build the toolbar** (C1).
  - The filter, with its state as `aria_toggled` on a wrapper, and its
    empty state "Nothing needs attention".
  - ⋯ with "Clear finished" through the log (L8), disabled when no row
    is `clearable`.
  - One layer at a time (R-13).
- [ ] **Step 3: Edit E-10.** Run every new and edited test by name.
  Verify lines, commit, push.
- [ ] **Step 4: Checkpoint.** Ticket comment: Tasks 9–10, the tests run
  with counts, the rulings so far. This is the 03b-1 line if the owner
  splits the Cycle.

---

## Task 11: The Footer Item And The Popover (E)

**Files:**

- Create: `D/view/needs_attention.rs`, `D/view/workspace/needs_attention.rs`.
- Modify:
  - `D/view/footer.rs` (Attention's bounds, its tooltip,
    `focus_after_hiding`);
  - `D/view/workspace/footer.rs` (`press` toggles);
  - `D/view/workspace/footer_status.rs` (`keep_focus_in_footer`);
  - `D/view/workspace/focus.rs` (`focus_lost`'s fallback for
    `attention_focus`, R-50);
  - `D/keys.rs` (`ATTENTION_POPOVER_CONTEXT`);
  - `D/view/workspace/render.rs` (mount the layer).
- Tests:
  - `D/view/needs_attention_tests.rs`:
    `the_popover_opens_lists_and_closes` (P22),
    `popover_actions_by_keyboard` (P23),
    `the_popover_works_with_activity_hidden` (P24),
    `the_popover_closes` (P25),
    `nothing_acts_behind_a_dialog_or_sheet` (P27: Activity, its menus,
    the footer item and the popover, each with a dialog and a sheet);
  - `count_and_popover_agree` extended to the popover's rows (P13);
  - `navigation_sends_nothing` extended to the popover and "Show
    everything in Activity" (P31);
  - E-9's popover part, and E-13 with every reader it names, which
    carries P26 in the footer's status table, and
    `footer_controls_do_nothing_behind_a_dialog`
    (`D/view/footer_tests/mod.rs:334`), which presses `Control::Attention`
    behind a dialog.

- [ ] **Step 1: Check U-8** with a scratch test: a passphrase prompt or a
  right-press menu opened from the popover closes onto `attention_focus`,
  and `focus_lost` can tell. Record it. If it does not hold, opening such
  a layer first moves focus to the footer item (R-50).
- [ ] **Step 2: Write the popover tests.**
  - P22: by click, twice; by Enter, then Escape; by Space, then Escape.
  - P23: a table over design P23's sources and buttons, a lifecycle's
    Resume (`ResumeLifecycle`) included, driven through the scheduler
    `World`, `PollingBackend` answers and fabricated `WorkerEvent`s, not
    a journey per action. A click or Enter on a row drawn before the
    log's row took a newer revision, or whose offer changed, runs
    nothing, and the popover stays open with the row refreshed (T8;
    R-55; design E2).
  - P25, each case of design P25: a full click on the footer item while
    the popover is open leaves it closed (fact 47); a press outside keeps
    the focus it gave, else the footer item; a dialog from the popover
    that closes after the item hid lands on E1's control (R-49); a dialog
    or menu opened while the popover had focus, then closed, lands on the
    footer item, or E1's control (R-50); the highlighted row leaving, or
    the row now at its position having no button, while its button holds
    focus moves focus to the popover's list, where Escape and the arrows
    still work.
  - P24, P27 and P31's popover part; extend P13 to the popover's rows.

  Each runs alone first. They fail.
- [ ] **Step 3: Build the footer item** (E1):
  - bounds;
  - tooltip;
  - the toggle;
  - focus after hiding moves right (R-37), `keep_focus_in_footer`
    covering Attention, which also decides focus after a dialog (R-49).
- [ ] **Step 4: Build the layer** (E2):
  - anchoring, 6 px above the footer (R-38);
  - the ids `"needs-attention-…"`;
  - the header, rows and link;
  - keys, with Tab's three stops inside (R-12), and Enter on the link;
  - the cursor following its `RowKey`, and a click or Enter building its
    choice from the drawn row, the popover's applied one, with its key and
    revision (R-55); when the highlighted row changes while its button
    holds focus, focus moves to the popover's list (`attention_focus`);
  - running an action: first `still_offered` checks the row's offer and
    revision, and on failure nothing runs and the popover stays open with
    the row refreshed; otherwise the popover closes and the action runs
    through `choose_activity` with the footer item's handle as origin; an
    action that places focus itself wins, and otherwise focus goes to the
    footer item if shown, else E1's control;
  - `attention_focus`, kept across opens, and `focus_lost`'s fallback
    for it (R-50);
  - "Show everything in Activity";
  - closing at 0, under a modal, when another layer opens (R-13), and on
    a press outside (R-11), whose handler ignores a press inside the
    Attention control's recorded bounds (fact 47).
- [ ] **Step 5: Edit E-13 and E-9's popover part.** Run every new and
  edited test, and each E-13 reader, by name. Verify lines, commit, push.

---

## Task 12: Privacy Of The New Layers (G)

**Files:**

- Modify: `D/view/lifecycle/privacy_tests.rs` (used by
  `privacy_promote_tests.rs` and `privacy_remote_tests.rs`),
  `D/view/polling_privacy_tests.rs`.

- [ ] **Step 1: Extend the sweeps** (E-14, P28):
  - **The popover:** the `"needs-attention-popover"` surface, with the
    title allowance.
  - **The row menu:** read through `activity_menu_open()` and the menu
    entity, not the shared `"popup-menu"` id, so the allowance never
    reaches the dock, repository or More menus.
  - **The positive control:** the title sentinel is seen in an Activity
    row, and a body sentinel planted in the same item is not.
  - **The folder and remote sentinels** of E-12 (a repository root's
    parent folder name; a remote URL's path and user-info, since host
    names are allowed), swept through the popover.
- [ ] **Step 2: Run** `polling_privacy` and D5's four `a16_` tests by name.
  Verify lines, commit, push.

---

## Task 13: Checklist, Gates, Review-Ready

- [ ] **Step 1: Write the on-screen checklist.** Write
  `docs/plans/2026-11-12-wave-04-cycle-03b-on-screen-checklist.md` for
  the owner, from Cycle 03a's. It covers frames 1a and 2f in the dark
  theme:
  - the desktop smoke test, run by the owner on Wayland:
    `devenv shell -- cargo run --locked --features desktop --bin manyhands`;
  - the rows' copy;
  - the filter and ⋯;
  - the row menu;
  - the popover's anchor and keys;
  - Activity hidden;
  - a narrow Activity;
  - the K-19 rows.
- [ ] **Step 2: Run the gates** (`AGENTS.md`):
  - the static gates:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  - the fast tier once, with `--no-fail-fast`. No SSH target, since this
    is desktop only:

```sh
devenv shell -- cargo test --all-features --locked --no-fail-fast --lib --bins \
  --test canonical_foundation --test cli --test discovery_rebuild \
  --test drafts --test journal_rejection --test key_material \
  --test key_storage --test local_authoring --test mutation_bridges \
  --test mutation_confirmation --test mutation_contract \
  --test mutation_relationships --test mutation_replay \
  --test read_boundary --test read_characterization --test read_comments \
  --test read_contract --test read_credentials --test read_items \
  --test read_relationships --test read_repository --test read_status \
  --test recovery_foundation_gate --test remote_reservation \
  --test repository_enablement --test session_credentials \
  --test shared_key_registry -- --skip repository::local_sync::
```

  - the CLI smoke test: `devenv shell -- cargo run --locked --bin manyhands-cli`;
  - `git fetch origin`, then `git merge-tree --write-tree HEAD origin/main`.

  Native CI is dispatched only with the owner's approval, after merge.
- [ ] **Step 3: Fill in the evidence.** Fill the Cycle's Acceptance
  Status from the tests by exact name and the ledger. On-screen rows wait
  for the owner.
- [ ] **Step 4: Mark the ticket review-ready.** Ticket comment and push.
  After review approval, the owner closes the ticket with `ticket close`,
  which performs the merge (`AGENTS.md`).

## Plan Self-Review

| Acceptance | Task |
| --- | --- |
| A1 | 3, 4 (P1), 5 (P3, P13), 10 (P19, P30), 11 (P13) |
| A2 | 11 (P22, P23, P25, P26) |
| A3 | 10 (P29), 11 (P24) |
| A4 | 9 (E-12, P28), 12 (E-14, P28) |
| A5 | 3, 4 (P4–P7), 9, 10 (P14) |
| A6 | 3, 4 (P1), 6, 7 (E-5), 8 (E-6) |
| A7 | 5 (P2, P13) |
| A8 | 10 (P21) |
| A9 | 4 (P12) |
| A10 | 9 (P15, P16, P17, P20, E-7), 10 (P15's Tab clause) |
| A11 | 6, 9 (P18), 6–8 (E-5, E-6), 10, 11 (P31) |
| A12 | 11 (P27) |
| A13 | 1 (P11), 2 (P9, P10), 9 (`D/view/operations.rs` below 500 lines) |
| A14 | 13 (on screen) |
| A15 | 4 (P12), 10 (P32) |
| A16 | 3, 4 (`each_source_makes_its_rows`), 9, 10 (`rows_draw_from_the_log_and_filter`) |

**Rulings by task:**

- **Task 1:** R-34.
- **Task 2:** R-8, R-9, R-35, R-39.
- **Task 3:** R-3, R-26, R-30, R-31, R-41, R-44, R-47 (the source's
  conditions, Open… aside), R-51 (a running retirement is its S1 row; the
  Owed copy), R-52 and R-57 (the subject with its target, set by S1).
- **Task 4:** R-4, R-5, R-14, R-15, R-16, R-17, R-32, R-40, R-44, R-48,
  R-51 (the retirement events), R-59 (S8's keys).
- **Task 5:** R-1, R-2, R-6, R-7, R-9, R-22, R-23, R-24, R-28, R-43,
  R-46 (the log's revisions), R-47 (the log's conditions), R-52 (N2,
  N22), R-53, R-54, R-56 (the latch), R-57 (N22's clause 1), R-58 (the
  applied-check record), R-59 (a replaced event's time).
- **Task 6:** R-21, R-45, R-46 (the choice's check).
- **Task 9:** decision 23 (C3), R-13, R-18, R-20, R-27, R-29, R-33, R-36,
  R-55 (the pane's drawn revision).
- **Task 10:** R-13, R-23, R-47 ("Clear finished", settled failures
  included).
- **Task 11:** R-10, R-11, R-12, R-13, R-37, R-38, R-49, R-50, R-55 (the
  popover's drawn revision and its check before closing).
- **Withdrawn in revision 4:** R-19 and R-42 (replaced by R-51).

**Named edits by task:**

- E-1, E-2: Task 2.
- E-3, E-4: Task 5.
- E-5: Task 6 (part 1) and Task 7 (part 2).
- E-6: Task 8.
- E-7, E-8, E-11, E-12, E-16: Task 9, which also reruns E-5's and E-6's
  tests by name.
- E-9: Tasks 9 (Activity's part) and 11 (the popover's).
- E-10: Task 10.
- E-13: Task 11.
- E-14: Task 12.
- E-15: any task.

**Required outcomes by task:** P1 in Tasks 3 and 4; P2, P3 and P13 in
Task 5 (P3's T8 in Task 9; P13 extended in 11); P4, P5 and P7 in Task
3; P6, P8 and P12 in Task 4; P9 and P10 in Task 2; P11 in Task 1; P14
in Tasks 9 and 10; P15 in Tasks 9 and 10 (its Tab clause); P16, P17 and
P20 in Task 9; P18 in Tasks 6 and 9; P19, P21, P29, P30 and P32 in Task
10; P22–P27 in Task 11; P28 in Tasks 9 and 12; P31 in Tasks 10 and 11.

**Tables by task:** N1–N25 (N20 withdrawn; N25 new in revision 5) and
T1–T18 (T14–T18 new in revision 5) in Task 5, test-first, under its
design checkpoint, except two window tests: T10,
`refreshes_are_coalesced`, also in Task 5, and T8 in Task 9 (P15–P17).
T11 also through `choose_activity` with an old revision, and T12
through `act_refused`, in Task 6. N22's five clauses and the latch
(R-56) are rows of Task 5's two tables.

**Unverified facts by task:** U-1 and U-7 in Task 3; U-5 in Task 4; U-2
and U-4 in Task 9; U-8 in Task 11; U-6 in Task 0, for what changes after
`81789331`. U-3 is settled (fact 47).

**Placeholders:** none. Every test, file and interface the tasks name is
in the design, and every interface is written out in full.
