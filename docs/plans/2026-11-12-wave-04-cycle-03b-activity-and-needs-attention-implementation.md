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
> **Draft, revision 1.** Not approved; Task 0 does not start before the
> owner approves it, authorizes implementation and chooses the execution
> method, and Cycle 03a has closed. Re-read the design before each task
> and record every checkpoint on ticket `01M784Y94Q3AXT6XJX38F5VK1V`.
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild Activity to `03-panels.md` as one timed log of one-line
rows, each with the time the app learned of it, a plain-language message,
a repository tag, one button and a menu with the rest. Add the Needs
attention popover from `05`, a filtered view of the same rows, opened from
the footer, whose count is their number. Fold in the operations split, a
share classified once, and the running stage shown.

**Architecture.**

- **The store.** A pure derivation turns today's models into rows. Its
  sources are requests, checking holds, Cycle 03a's repository states,
  the unshared list, retirements, pending operations, drafts and notes,
  plus a new session history of checks that brought something in (design
  Parts A, B).
- **The log.** The Workspace's `ActivityLog` gives each row the time it
  was first learned. It keeps events, lets states go when they resolve,
  holds a repository row through a check, and counts the attention rows
  once each (A3, A4).
- **The panel.** Activity draws the log in its own `uniform_list`, with a
  filter, ⋯ "Clear finished", a row menu like Cycle 03a's, and keys (C,
  D).
- **The popover.** It is a Workspace-owned layer above the footer item
  (E).
- **Tests.** Existing tests move first onto a seam that acts by row key
  and label, so the drawing can be swapped without touching them
  (Testing).
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-03b-activity-and-needs-attention.md) and
[design](2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md),
with `docs/design/03-panels.md` "Activity", `05` "Needs attention
popover", `02` Footer item 5 and `06`.

- "Fact n", "S-n", "N-n", "L-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer
  to the design.
- "Decision n" refers to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method.
- Cycle 03a (`01M784Y8C73B39GR4JWENXE5D0`) is closed and merged to main.
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

**Main.** Merge `origin/main` at Task 0, before Task 8, and whenever main
moves. Never rebase.

**Push.** After each task's commit, the orchestrator pushes the ticket
branch with `item sync` from the main checkout (owner, 2026-11-11). It
then checks that `HEAD` equals
`origin/manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is design Part B's, verbatim, held in
a `pub(crate) const`, or in a formatting function in
`D/model/activity/copy.rs` for strings with values. Tests compare
constants. Sentence case, no exclamation marks, `…` (U+2026) on an action
that opens a dialog, sheet or page, typographic quotes around a title.

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
  Unchanged tests are not rerun. There is no whole-suite run before Task
  11.
- **One module run.** Task 1 moves code without changing tests, and runs
  `desktop::model::operations` once, by module, because its tests'
  subject moved. That is a doubt the plan names.
- **Limits.**
  - A command past about 15 minutes is stopped, narrowed and noted.
  - Never build while a test run is in progress.
  - A new window test runs alone first.
- **Unverified facts** (U-1 to U-7) are checked with a scratch test that
  is discarded, not committed. Its result goes in the ledger.

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
  `attention_counts_each_thing_once` from the design's table N1–N16 before
  any of its code. If a row cannot be expressed against the code, the
  implementer stops and reports to the orchestrator. The orchestrator
  settles it with the design before implementation goes on.

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
- **The seam.** From Task 6, `D/view/activity_support.rs`'s functions
  (design Testing).

**Existing tests.**

- They change only as E-1 to E-16 say.
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

**Ticket records.** After Tasks 0, 7 and 11, a comment through
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

- `D/view/workspace.rs` (488 lines) gains nothing: the Workspace's new
  code goes in `D/view/workspace/activity.rs` and
  `D/view/workspace/needs_attention.rs`.
- `D/model/operations.rs` shrinks in Task 1, and `D/view/operations.rs`
  in Task 8.

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **A failed share retried.** Its row keeps its time and place, its
   stored result is cleared and classified again, and its count is one
   throughout (Tasks 2, 5: `shares_are_classified_once`,
   `the_log_keeps_times_and_events`).
2. **An unreachable repository that the scheduler retries.** Its row and
   the count hold through each check, so the popover does not flicker
   (Tasks 5, 8: `the_log_keeps_times_and_events`,
   `a_checking_repository_keeps_its_row`).
3. **A failed request whose operation is also a pending row, and two
   requests of one operation.** Each counts once (Task 5,
   `attention_counts_each_thing_once`, N1 and N2).
4. **"Clear finished" with a failure and a state present.** Both stay
   (Task 8, `clear_finished_keeps_failures_and_states`).
5. **A popover action that opens a dialog, with Activity hidden.** Focus
   comes back to the footer item, or, if that hid, to the control before
   it (Task 9, `popover_actions_by_keyboard`, `the_popover_closes`).
6. **A row menu choice after the row changed.** Nothing runs (Task 8,
   `the_row_menu_by_pointer_and_keyboard`).
7. **A title sentinel in a row.** It is allowed, while a body sentinel
   in the same row is not (Task 10, the D5 sweeps).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main (with 03a), baseline gates, re-check facts, E-sweep, ledger |
| 1 | F1 | Split `model/operations.rs`; re-exports; no behaviour change |
| 2 | F2, F3 | A share classified once; the stage shown; E-1, E-2 |
| 3 | A1, B1–B3, B5, B6 | Row types, request rows and their copy |
| 4 | A2, A5, A7, B4 | State rows, the brought-in history, `each_repository` |
| 5 | A3, A4, A6 | The log, attention test-first, the count, the Workspace's refresh points; E-3, E-4 |
| 6 | A1 actions, Testing | `ActivityMenuChoice`, `run_activity_action`, the seam; E-5 |
| 7 | Testing | The other files onto the seam; E-6; checkpoint |
| 8 | C, D, F4, H | The Activity panel and its row menu replace the columns; E-7 to E-12, E-16 |
| 9 | E | The footer item and the popover; E-9 part, E-13 |
| 10 | G | Privacy sweeps; E-14 |
| 11 | — | On-screen checklist, gates, review-ready; checkpoint |

**New source files:**

- `D/model/activity.rs`, with `activity/{rows,requests,states,log,copy}.rs`;
- `D/model/operations/{progress,recovery,pending,lifecycle}.rs`;
- `D/view/activity.rs`, with `activity/{row,menu,actions}.rs`;
- `D/view/needs_attention.rs`;
- `D/view/workspace/{activity,needs_attention}.rs`;
- `D/view/activity_support.rs` (test only).

**New test files:** `D/model/activity_tests.rs`, `D/view/activity_tests.rs`,
`D/view/needs_attention_tests.rs`. New tests in existing files are named
in their tasks.

**Order:** 0 → 1 → … → 11, one at a time. Tasks 0–8 form 03b-1 if the
owner splits the Cycle (design Risks).

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
- [ ] **Step 2: Re-check facts (U-6).** Run
  `git diff --stat b37d2c2b HEAD -- src/desktop docs/Waves docs/RFC docs/design docs/plans Cargo.lock`.
  - Re-read facts 20–23, 28–30 and 46, and each fact whose file changed,
    and correct the design first.
  - Record in the ledger how Cycle 03a's deferred Minor 2 ended (fact 46).
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner: facts 31–34 and U-2 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list**, over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'request_id(' -e 'recovery_id(' -e 'draft_flush_id(' -e '"operation-area"' \
  -e '"pending-' -e 'PENDING_REVIEW' -e 'attention-action' -e '("attention",' \
  -e 'attempt-' -e 'record-note' -e 'recovered-drafts' -e 'abandon_focus' -- $P
git grep -n -e 'needs_attention' -e 'share_result(' -e 'FINISHING' -e 'state_label' \
  -e 'outcome_label' -e 'effects_text' -e 'scope_name' -e 'ToolbarButton::ALL' \
  -e 'Control::Attention' -e 'focus_after_hiding' -e 'keep_focus_in_footer' \
  -e 'footer-attention' -e 'WorkerEvent::Finished(' -e 'ICONS' -- $P
```

  - Record each test site in the ledger against its E-n.
  - A test site no E-n names is a pre-authorized edit (decision 14),
    recorded as E-17 onward with its task.
  - A production site no task names goes into the task that breaks it.
  - List every caller of the helpers E-5 to E-7 name (`assert_row`,
    `close_row`, `row_of`, `under_a_resume`, `body_sheet`,
    `press_retire`, `recovery_labels`, `retry_control`, `entry_texts`,
    `reopen`, `open_abandon`, the `Walk` methods), since a helper's edit
    reaches them.
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
}
pub(crate) fn stage_words(stage: ProgressStage, primary: Option<&str>) -> &'static str;
```

- [ ] **Step 1: Write the classification test (P9).**
  `shares_are_classified_once` is a table with these rows:
  - a share noted Accepted, then finished;
  - the worker's record changing after `Finished`: the stored result does
    not change;
  - a retry `Queued`: the result is cleared;
  - a fabricated `Finished` with no note: `Unknown`, no Give up.

  It fails.
- [ ] **Step 2: Implement F2.**
  - Add `share_attempts` and `share_results` to `Operations`.
  - The drain calls `note_share_attempt` beside `note_record`, before
    `apply` (fact 13).
  - `apply(Finished)` classifies once, and `apply(Queued)` clears both
    maps (R-39).
  - The view's `share_result`, `finished_share`, `run_share_offer` and the
    item tab's `share_outcome` read the stored result.
  - `share_words` and `shown_offers` keep their render-time inputs (R-9).
- [ ] **Step 3: Edit E-1.** Its two tests note the attempt, then read the
  stored result.
- [ ] **Step 4: Write the stage test (P10).** `stages_are_shown` covers:
  - `state_label` for each `ProgressStage` with and without `accepted`;
  - `stage_words` for each stage;
  - "Finishing" for accepted with no stage.

  It fails.
- [ ] **Step 5: Implement F3.**
  - `stage_words` lives in `progress.rs`.
  - `state_label`'s stage arm comes before the accepted arm.
  - The disabled "Finishing" control is unchanged for now. It becomes the
    row's button in Task 8.
- [ ] **Step 6: Edit E-2,** then run:
  - `… desktop::model::operations::tests::shares_are_classified_once`;
  - `… desktop::model::operations::tests::stages_are_shown`;
  - E-1's and E-2's tests, by name;
  - any settings test that reads `state_label` (fact 14), found by Task 0's
    sweep and recorded as E-17 onward.

  Verify lines, commit, push.

---

## Task 3: Row Types, Request Rows And Their Copy (A1, S1, B1–B3, B5, B6)

**Files:**

- Create: `D/model/activity.rs` (with `mod activity;` in `D/model/mod.rs`),
  `activity/rows.rs`, `activity/requests.rs`, `activity/copy.rs`.
- Tests: `D/model/activity_tests.rs`.

**Interfaces** (design A1):

```rust
pub(crate) struct Derived { pub row: ActivityRow /* at unset */, pub kept: bool }
pub(crate) struct RequestInputs<'a> {
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
}
pub(crate) fn request_rows(operations: &Operations, inputs: &RequestInputs<'_>) -> Vec<Derived>;
pub(crate) fn activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) -> (String, String);
```

- [ ] **Step 1: Check U-1 and U-7** with a scratch test.
  - U-1: the derives `ActivityMenuChoice` will need. Add each missing
    derive to a desktop type.
  - U-7: finding an open tab from `(root, kind, id)`.

  Record both.
- [ ] **Step 2: Write the request tables.** Write `request_copy_table` (P4:
  every intent of B2 in each state), `share_copy_table` (P5: each
  `SyncOutcome` with records Accepted, Finished and Unknown and after a
  Give up, including the button and menu order) and
  `activity_time_formats` (P7). Then write the S1 rows of
  `each_source_makes_its_rows` (P1: key, kind, button, menu, `finished`,
  `item`, detail lines). All fail.
- [ ] **Step 3: Implement.**
  - The types: `RowKey`, `Kind`, `Offer`, `ActivityRow` and
    `ActivityAction`.
  - `request_rows`, following B2's table, B3 and B5.
  - The title lookup of B1 is a closure the Workspace supplies in Task 5.
    The tests pass their own.
  - `activity_time`.
- [ ] **Step 4: Run** the four tests by name. Verify lines, commit, push.

---

## Task 4: State Rows, The Brought-In History, `each_repository` (A2, A5, A7, B4)

**Files:**

- Create: `D/model/activity/states.rs`.
- Modify: `D/model/polling.rs` (`Brought`, `brought()`), `after`,
  `checked_now`, `D/view/repositories.rs` (`each_repository`).
- Tests:
  - `D/model/activity_tests.rs`;
  - the polling model's tests: `brought_in_is_kept_for_the_session` (P8).

**Interfaces:**

```rust
pub(crate) struct StateInputs<'a> {
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],   // root, name, RepoState, tooltip lines, checking, unshared
    pub operations: &'a Operations,            // pending, record notes
    pub drafts: DraftsInput,                   // flush failures, unavailable, recovered
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
}
pub(crate) fn state_rows(inputs: &StateInputs<'_>) -> Vec<Derived>;
pub(crate) fn each_repository<R>(workspace: &Workspace, cx: &App, f: impl FnMut(&RepoFacts<'_>) -> R) -> Vec<R>;
```

- [ ] **Step 1: Check U-5.** Does `checked_now`'s in-flight branch return a
  summary? Record it.
- [ ] **Step 2: Write `brought_in_is_kept_for_the_session` (P8).**
  - An automatic check and a Check now that brought in, through `after`
    and `checked_now`.
  - Ones that brought in nothing, and a failed one.
  - The sequence numbers and `automatic`.

  It fails. Implement A5.
- [ ] **Step 3: Move `RepoFacts` building into `each_repository`** (A7;
  R-5), unchanged. `rows()` calls it. Cycle 03a's panel tests are not
  rerun: their code moved but they did not change (P12 is proved in
  Step 4).
- [ ] **Step 4: Write the state tables.** Write `state_copy_table` (P6:
  each row of B4, the dispositions, the total pending-action table, the
  next actions). Write the S2–S14 rows of `each_source_makes_its_rows`
  (P1, P12), including:
  - each S4 state that makes a row and each that does not (R-14, R-15);
  - S5 and S6 from the unshared list (R-17);
  - S8's `kept` once final (R-19).

  All fail.
- [ ] **Step 5: Implement `state_rows`.** Implement it from design A2 and
  B4. Run the tests by name. Verify lines, commit, push.

---

## Task 5: The Log, Attention, The Count (A3, A4, A6)

**Files:**

- Create: `D/model/activity/log.rs`, `D/view/workspace/activity.rs`.
- Modify:
  - `D/view/workspace.rs` (the observers call `refresh_activity`; the
    field `activity: ActivityLog`);
  - `D/view/workspace/footer.rs` (`attention()` reads the log);
  - the navigation and title refresh points (A4).
- Delete: `D/model/attention.rs` with its tests (E-3).
- Tests:
  - `D/model/activity_tests.rs`: `attention_counts_each_thing_once` (P2)
    and `the_log_keeps_times_and_events` (P3);
  - `D/view/activity_tests.rs`: `count_and_popover_agree` (P13, its
    count half);
  - E-3, E-4.

**Interfaces:**

```rust
impl ActivityLog {
    pub(crate) fn refresh(&mut self, derived: Vec<Derived>, checking: &BTreeSet<RepoKey>, now: OffsetDateTime) -> bool;
    pub(crate) fn rows(&self) -> &[ActivityRow];
    pub(crate) fn attention_count(&self) -> usize;
    pub(crate) fn dismiss(&mut self, key: &RowKey);
    pub(crate) fn finished(&self) -> Vec<RowKey>;
}
impl Workspace {
    pub(in crate::desktop::view) fn refresh_activity(&mut self, cx: &mut Context<Self>);
    pub(crate) fn activity(&self) -> &ActivityLog;
}
```

- [ ] **Step 1: Design checkpoint: the N-table test first.** Write
  `attention_counts_each_thing_once` from N1–N16, one table row per N-row,
  over `derive` and the log. If a row cannot be expressed against the
  code, stop and report to the orchestrator. It fails.
- [ ] **Step 2: Write `the_log_keeps_times_and_events` (P3).** Cover L1–L9:
  - a retry keeps time and place;
  - a dismissed request retried returns;
  - a state that leaves and returns gets a new time;
  - a repository row through a check (L3);
  - a kept retirement line;
  - equal times;
  - "Clear finished".

  It fails.
- [ ] **Step 3: Implement the log.** Implement `ActivityLog` and the
  attention rules in `state_rows` and `request_rows`: N1–N4 need the
  operation grouping today's `needs_attention` does (fact 24).
- [ ] **Step 4: Wire the Workspace.**
  - `refresh_activity` builds the inputs: titles from tabs and
    `share_title`, `each_repository`, and the clock's `utc()`.
  - It is called from every refresh point of A4.
  - `attention()` returns `attention_count()`.
- [ ] **Step 5: Delete the old count.** Delete `needs_attention` and its
  tests (E-3); move their cases into Step 1's table, listed in the ledger.
  Apply E-4, recording any changed count with its N-row.
- [ ] **Step 6: Write the count journey.** `count_and_popover_agree`
  (P13), for now comparing the footer count with the log's attention rows
  through a journey of each counted kind, each then resolved. It runs
  alone first.
- [ ] **Step 7: Run** the new tests and E-3's and E-4's by name. Verify
  lines, commit, push.

---

## Task 6: Actions And The Test Seam (E-5)

**Files:**

- Modify:
  - `D/keys.rs` (`ActivityMenuChoice`);
  - `D/view/workspace/render.rs` (its `on_action`, through
    `unless_dialog`).
- Create:
  - `D/view/activity/actions.rs` (`run_activity_action`); the declaration
    of `D/view/activity.rs` itself can come here;
  - `D/view/activity_support.rs` (test only).
- Tests:
  - `D/view/activity_tests.rs`: `each_action_runs_its_handler` (P18,
    dispatch half);
  - the files of E-5.

**Interfaces:**

```rust
#[derive(Clone, PartialEq, Debug, gpui_kit::Action)]
#[action(namespace = manyhands, no_json)]
pub(crate) struct ActivityMenuChoice { pub key: RowKey, pub action: ActivityAction, pub shown: String }

impl Workspace {
    pub(in crate::desktop::view) fn run_activity_action(&mut self, action: &ActivityAction, origin: FocusHandle, window: &mut Window, cx: &mut Context<Self>);
}
// activity_support.rs
pub(crate) fn row(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) -> Option<ActivityRow>;
pub(crate) fn rows(shell: &Shell, cx: &mut TestAppContext) -> Vec<ActivityRow>;
pub(crate) fn offered(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) -> Option<Offer>;
pub(crate) fn act(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext);
pub(crate) fn request(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> DesktopRequest;
pub(crate) fn pending_key(shell: &Shell, root: &str, action: &str, cx: &mut TestAppContext) -> RowKey;
```

- [ ] **Step 1: Write the dispatch test.** `each_action_runs_its_handler`
  covers each `ActivityAction` kind, dispatched as a choice. Each sends
  exactly what today's control sends, read from the recording backend or
  the dialog or sheet it opens. It fails.
- [ ] **Step 2: Implement the choice.**
  - `ActivityMenuChoice` checks the row again (design D): an enabled
    offer with the same action and label.
  - `run_activity_action` maps each action to its handler (A1's table),
    passing the origin.
  - For now the origin is the Operations region's handle. Task 8 replaces
    it with the list's.
- [ ] **Step 3: Write the seam** functions above.
- [ ] **Step 4: Move E-5's tests onto the seam.** Move them file by file,
  following E-5's rule. Before each file, record its test function list
  in the ledger; after, record it again, and check that they are equal.
  Keyboard and focus steps are untouched.
- [ ] **Step 5: Run** every E-5 test by name, and the new test. All pass,
  with the old columns still drawn. Verify lines, commit, push.

---

## Task 7: The Other Files Onto The Seam (E-6)

**Files:** the files of E-6.

- [ ] **Step 1: Move the tests.** Move E-6's tests file by file, as in
  Task 6 Step 4, with the function lists before and after in the ledger.
  - `settings/repositories_tests.rs`' literal id (`:569`) goes with the
    rest.
  - D5's sweeps keep reading the drawn area until Task 10.
- [ ] **Step 2: Run** every E-6 test by name. All pass. Verify lines,
  commit, push.
- [ ] **Step 3: Checkpoint.** Ticket comment: Tasks 1–7, the moved tests
  with counts, the rulings so far.

---

## Task 8: The Activity Panel And Its Row Menu (C, D, F4, H)

The largest task. Before it, merge `origin/main`.

**Files:**

- Create:
  - `D/view/activity.rs` (the pane, its list and toolbar wiring);
  - `D/view/activity/row.rs`;
  - `D/view/activity/menu.rs`.
- Modify:
  - `D/keys.rs` (`ACTIVITY_CONTEXT` and its bindings);
  - `D/view/dock/header/toolbar.rs` and `D/view/workspace/focus.rs`
    (`ToolbarButton::ActivityFilter`, `ActivityMore`);
  - `D/view/workspace/render.rs` (`activity_region`; mount the row menu
    and the ⋯ menu layers);
  - `D/view/workspace/activity.rs` (the filter, the menu, focus);
  - `D/view/workspace/requests.rs`, `D/view/workspace/lifecycle.rs` and
    `D/view/operations/review.rs` (origins: C3);
  - `D/view/operations.rs` and its submodules (the drawing goes: F4);
  - `D/view/workspace.rs` (the focus maps of fact 8 go).
- Tests:
  - `D/view/activity_tests.rs`: `rows_draw_from_the_log_and_filter`
    (P14), `activity_by_keyboard` (P15), `rows_by_pointer` (P16),
    `the_row_menu_by_pointer_and_keyboard` (P17);
  - `each_action_runs_its_handler` extended to the drawn button and the
    menu (P18);
  - `clear_finished_keeps_failures_and_states` (P19),
    `a_dialog_from_a_row_returns_focus_to_its_row` (P20),
    `rows_fit_a_narrow_activity` (P21), `activity_hidden_keeps_the_log`
    (P29), `a_checking_repository_keeps_its_row` (P30);
  - E-7 to E-12, E-16.

- [ ] **Step 1: Check U-2 and U-4** with a scratch test:
  - `uniform_list` with a tracked scroll handle draws and scrolls the
    rows;
  - a key bound in `ACTIVITY_CONTEXT` reaches its handler while the list
    holds focus.

  Record both. If U-2 fails, use the Kit's `VirtualList` with the same
  keys, and record the amendment.
- [ ] **Step 2: Write the list and key tests.** Write P14, P15, P16, P20,
  P21, P29 and P30. Each runs alone first. They fail.
- [ ] **Step 3: Build the pane and rows** (C1, C2).
  - The rows are applied after the frame, and the highlight follows the
    key.
  - The toolbar's filter and ⋯ ("Clear finished" through the log, L8).
  - The empty states.
  - The container keeps `"operation-area"` (R-20).
  - Mount it in `activity_region` below the notices and the quit row
    (R-18). Remove the old columns' drawing (F4).
- [ ] **Step 4: Write the menu tests.** Write P17 and P18's menu half.
  They fail.
- [ ] **Step 5: Build the row menu** (Part D), on Cycle 03a's pattern:
  - a right press;
  - `shift-f10` and `menu`;
  - the detail lines first;
  - the re-check;
  - dropped under a modal and when its row leaves;
  - one layer at a time (R-13).
- [ ] **Step 6: Wire focus (C3).**
  - Origins and restores become the list's handle, with the highlight on
    the row's key.
  - "Show it" and Drafts not saved's Cancel highlight their rows.
  - Delete the focus maps, `prune_abandon_focus` and
    `Workspace::abandon_focus`.
- [ ] **Step 7: Edit E-7 to E-12 and E-16.** For E-7, rewrite the keyboard
  and focus steps to Part H's keys. Record each rewritten test's steps in
  the ledger.
- [ ] **Step 8: Run** every new and edited test by name. Verify lines,
  commit, push.

---

## Task 9: The Footer Item And The Popover (E)

**Files:**

- Create: `D/view/needs_attention.rs`, `D/view/workspace/needs_attention.rs`.
- Modify:
  - `D/view/footer.rs` (Attention's bounds, its tooltip,
    `focus_after_hiding`);
  - `D/view/workspace/footer.rs` (`press` toggles);
  - `D/view/workspace/footer_status.rs` (`keep_focus_in_footer`);
  - `D/keys.rs` (`ATTENTION_POPOVER_CONTEXT`);
  - `D/view/workspace/render.rs` (mount the layer).
- Tests:
  - `D/view/needs_attention_tests.rs`:
    `the_popover_opens_lists_and_closes` (P22),
    `popover_actions_by_keyboard` (P23),
    `the_popover_works_with_activity_hidden` (P24),
    `the_popover_closes` (P25);
  - `count_and_popover_agree` extended to the popover's rows (P13);
  - `nothing_acts_behind_a_dialog_or_sheet` (P27: Activity, its menus,
    the footer item and the popover, each with a dialog and a sheet);
  - E-9's popover part, and E-13, which carries P26 in the footer's
    status table.

- [ ] **Step 1: Check U-3** (the order of a press outside and the footer
  item's click). Record it.
- [ ] **Step 2: Write the popover tests.** Write P22–P25 and P27, and
  extend P13. Each runs alone first. They fail.
- [ ] **Step 3: Build the footer item** (E1):
  - bounds;
  - tooltip;
  - the toggle;
  - focus after hiding (R-37).
- [ ] **Step 4: Build the layer** (E2):
  - anchoring (R-38);
  - the header, rows and link;
  - keys, with Tab inside (R-12);
  - running an action after closing, with the footer item's handle as
    origin;
  - "Show everything in Activity";
  - closing at 0, under a modal, when another layer opens, and on a press
    outside (R-11).
- [ ] **Step 5: Edit E-13 and E-9's popover part.** Run every new and
  edited test by name. Verify lines, commit, push.

---

## Task 10: Privacy (G)

**Files:**

- Modify: `D/view/lifecycle/privacy_tests.rs` (the title-only allowance,
  used by `privacy_promote_tests.rs` and `privacy_remote_tests.rs`),
  `D/view/polling_privacy_tests.rs`.

- [ ] **Step 1: Extend the sweeps** (E-14, P28):
  - **D5's `Seen`:** `TITLE` and `READY` are allowed inside
    `"operation-area"`, `"attention-popover"` and the row menu; the other
    sentinels stay forbidden everywhere.
  - **`polling_privacy`:** gains the log's rows read from the model, the
    popover and an open row menu.
  - **The positive control:** the title sentinel is seen in an Activity
    row, and a body sentinel planted in the same item is not.
- [ ] **Step 2: Run** `polling_privacy` and D5's four `a16_` tests by name.
  Verify lines, commit, push.

---

## Task 11: Checklist, Gates, Review-Ready

- [ ] **Step 1: Write the on-screen checklist.** Write
  `docs/plans/2026-11-12-wave-04-cycle-03b-on-screen-checklist.md` for
  the owner, from Cycle 03a's. It covers frames 1a and 2f in the dark
  theme:
  - the rows' copy;
  - the filter and ⋯;
  - the row menu;
  - the popover's anchor and keys;
  - Activity hidden;
  - a narrow Activity;
  - the K-19 rows.
- [ ] **Step 2: Run the gates** (`AGENTS.md`):
  - the static gates;
  - the fast tier once, with `--no-fail-fast`. No SSH target, since this
    is desktop only;
  - `git merge-tree --write-tree HEAD origin/main`.
- [ ] **Step 3: Fill in the evidence.** Fill the Cycle's Acceptance
  Status from the tests by exact name and the ledger. On-screen rows wait
  for the owner.
- [ ] **Step 4: Mark the ticket review-ready.** Ticket comment and push.
  After review approval, the owner closes the ticket with `ticket close`,
  which performs the merge (`AGENTS.md`).

## Plan Self-Review

| Acceptance | Task |
| --- | --- |
| A1 | 3, 4, 5 (P1, P3), 9 (P13) |
| A2 | 9 (P22, P23) |
| A3 | 8 (P29), 9 (P24) |
| A4 | 10 (P28) |
| A5 | 3, 4 (P4–P7), 8 (P14) |
| A6 | 3, 4 (P1), 6, 7 (E-5, E-6) |
| A7 | 5 (P2, P13) |
| A8 | 8 (P21) |
| A9 | 4 (P12) |
| A10 | 8 (P15, P17, E-7) |
| A11 | 6, 8 (P18) |
| A12 | 9 (P27) |
| A13 | 1 (P11), 2 (P9, P10) |
| A14 | 11 (on screen) |

**Rulings by task:**

- **Task 2:** R-8, R-9, R-35, R-39.
- **Task 3:** R-3, R-26, R-30, R-31.
- **Task 4:** R-4, R-5, R-14, R-15, R-16, R-17, R-19, R-32.
- **Task 5:** R-1, R-2, R-6, R-7, R-22, R-23, R-24, R-28.
- **Task 6:** R-21.
- **Task 8:** R-13, R-18, R-20, R-25, R-27, R-29, R-33, R-36.
- **Task 9:** R-10, R-11, R-12, R-37, R-38.
- **Task 1:** R-34.

**Named edits by task:**

- E-1, E-2: Task 2.
- E-3, E-4: Task 5.
- E-5: Task 6.
- E-6: Task 7.
- E-7, E-8, E-10, E-11, E-12, E-16: Task 8.
- E-9: Tasks 8 and 9.
- E-13: Task 9.
- E-14: Task 10.
- E-15: any task.

**Unverified facts by task:** U-1 and U-7 in Task 3; U-5 in Task 4; U-2
and U-4 in Task 8; U-3 in Task 9; U-6 in Task 0.

**Placeholders:** none. Every test, file and interface the tasks name is
in the design.
