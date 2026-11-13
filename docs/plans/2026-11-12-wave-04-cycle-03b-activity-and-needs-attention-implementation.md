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
> **Draft, revision 2.** It applies review round 1 and decisions 21–24;
> not yet re-reviewed. Not approved; Task 0 does not start before the
> owner approves it, authorizes implementation and chooses the execution
> method, and Cycle 03a has closed. Re-read the design before each task
> and record every checkpoint on ticket `01M784Y94Q3AXT6XJX38F5VK1V`.
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild Activity to `03-panels.md` as one timed log of one-line
rows, each with the time the app learned of it, a plain-language message,
a repository tag, at most one button and a menu with the rest. Add the
Needs attention popover from `05`, a filtered view of the same rows,
opened from the footer, whose count is their number. Fold in the
operations split, a share classified once, and the running stage shown.

**Architecture.**

- **The store.** A pure derivation, `derive`, turns today's models into
  rows, then marks the attention rows (`assign_attention`). Its sources
  are requests, checking holds, Cycle 03a's repository states, the
  unshared list, retirements, pending operations, drafts and notes, plus
  a new session history of checks that brought something in (design
  Parts A, B).
- **The log.** The Workspace's `ActivityLog` gives each row the time it
  was first learned. It keeps events, lets states go when they resolve,
  holds a repository row through a check, and counts the attention rows
  once each (A3, A4). Refreshes are coalesced, one per effect cycle.
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

- "Fact n", "S-n", "N-n", "L-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer
  to the design.
- "Decision n" and "A-n" (an Acceptance row) refer to the Cycle.
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
  1's module, Task 4's Cycle 03a tests). There is no whole-suite run
  before Task 13.
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
  `attention_counts_each_thing_once` from the design's table N1–N17, over
  `derive`, before any of its code. If a row cannot be expressed against
  the code, the implementer stops and reports to the orchestrator. The
  orchestrator settles it with the design before implementation goes on.

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
  `act` calls `Workspace::choose_activity` directly inside
  `workspace.update`; it does not dispatch a window action (design
  Testing; R-45).

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
4. **"Clear finished" with a failure and a state present.** Both stay
   (Task 10, `clear_finished_keeps_failures_and_states`).
5. **A popover action that opens a dialog, with Activity hidden.** Focus
   comes back to the footer item, or, if that hid before the dialog
   closed, falls back as Cycle 01's `focus_lost` does (Task 11,
   `popover_actions_by_keyboard`, and `the_popover_closes` with the
   hidden-item case).
6. **A row menu choice after the row changed.** Nothing runs (Task 9,
   `the_row_menu_by_pointer_and_keyboard`).
7. **A title sentinel in a row.** It is allowed, while a body sentinel
   in the same row is not (Task 9, the D5 sweeps' title allowance; Task
   12, the positive control).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main (with 03a), baseline gates, re-check facts, E-sweep, ledger; checkpoint |
| 1 | F1 | Split `model/operations.rs`; re-exports; no behaviour change |
| 2 | F2, F3 | A share classified once (`classify_share`, the new getter); the stage shown (`stage_words -> String`); E-1, E-2 |
| 3 | A1, B0–B3, B5, B6 | Row types (`Derived`, `ActivityRow`, `ItemRef`, `Target`), request rows and their copy; the pure text helpers move into the model (R-41) |
| 4 | A2, A5, A7, B4 | State rows, `RepoState.cause`, the brought-in history, `each_repository`; Cycle 03a's panel tests by name (P12); P32 |
| 5 | A2, A3, A4, A6 | `derive`, `assign_attention`, the log, coalesced refresh and its points, the count; creates `D/view/activity.rs`; E-3, E-4; checkpoint |
| 6 | A1 actions, Testing | `ActivityMenuChoice`, `choose_activity`, `run_activity_action`, handler signatures, the seam; E-5 part 1 |
| 7 | Testing | E-5 part 2 onto the seam |
| 8 | Testing | The other files onto the seam; E-6; checkpoint |
| 9 | C, D, F4, G, H | The swap: the pane, rows, keys, the row menu, focus; the old drawing goes; the sweeps at the swap; E-7, E-8, E-11, E-12, E-16 |
| 10 | C1, C2 | The toolbar, the filter, ⋯ and "Clear finished"; E-9's Activity part, E-10; checkpoint |
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
  - Rename today's three-argument `share_result(id, record,
    after_give_up)` (`D/model/operations.rs:477`) to `classify_share`,
    kept as `#[cfg(test)]`, and add the getter `share_result(&self, id)`.
  - The view's `share_result` (same signature), `finished_share`,
    `run_share_offer` and the item tab's `share_outcome` read the stored
    result.
  - `share_words` and `shown_offers` keep their render-time inputs (R-9).
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
  - E-1's and E-2's tests, by name;
  - any settings test that reads `state_label` (fact 14), found by Task 0's
    sweep and recorded as E-17 onward.

  Verify lines, commit, push.

---

## Task 3: Row Types, Request Rows And Their Copy (A1, S1, B0–B3, B5, B6)

**Files:**

- Create: `D/model/activity.rs` (with `mod activity;` in `D/model/mod.rs`),
  `activity/rows.rs`, `activity/requests.rs`, `activity/copy.rs`.
- Modify: `D/view/operations.rs` and `D/view/lifecycle/outcome.rs` (the
  pure text helpers move out, re-exported from their old paths; R-41).
- Tests: `D/model/activity_tests.rs`.

**Interfaces** (design A1, A2):

```rust
pub(crate) enum Target { Main, Item(ItemId), Unnamed(u32) }
pub(crate) struct ItemRef { pub root: RepoKey, pub kind: AuthoringKind, pub id: ItemId }
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
    pub kept: bool,
}
pub(crate) struct ActivityRow { pub at: OffsetDateTime, pub row: Derived }
pub(crate) struct RequestFacts {
    pub stop_offer: StopOffer,
    pub shown_offers: Vec<SyncOffer>,
    pub share_words: Option<String>,
    pub already_finished: bool,
    pub review: bool,
    pub recoveries: Vec<(usize, String, bool)>,
    pub lifecycle_lines: Vec<String>,
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
  `outcome_label`, `effects_text`, `scope_name` and the lifecycle outcome
  lines into `D/model/activity/copy.rs` (or `model/operations`), with no
  GPUI import, and re-export each from its old path. No caller and no
  test changes. `cargo check` and `clippy`.
- [ ] **Step 3: Write the request tables.**
  - `request_copy_table` (P4): every intent of B2 in each state, with
    the templates that do not repeat the subject, and no path.
  - `share_copy_table` (P5): each `SyncOutcome` with records Accepted,
    Finished and Unknown and after a Give up, including B2's button
    precedence (risky actions never the button) and the menu's order.
  - `activity_time_formats` (P7).
  - The S1 rows of `each_source_makes_its_rows` (P1): key, kind, title,
    button, menu, `clearable`, `item`, detail lines. A save or create of
    an item with no title known reads by its short code, never the
    draft's own title (B1). The detail holds no time line (B5).

  All fail.
- [ ] **Step 4: Implement.**
  - The types of design A1.
  - `request_rows`, following B1, B2's table and rules, B3 and B5.
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
  - `D/model/repository_state.rs` (`AttentionCause`, `RepoState.cause`);
  - `D/view/repositories.rs` (`each_repository`).
- Tests:
  - `D/model/activity_tests.rs`, with `reminders_match_to_share` (P32);
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
    pub tooltip: Vec<String>,
    pub checking: bool,
    pub primary: Option<String>,
    pub unshared: Unshared,
    pub sharing: BTreeMap<Target, ShareState>,
    pub retiring: BTreeSet<ItemId>,
}
pub(crate) enum ShareState { Ready, Sharing, Locked(String) }
pub(crate) struct DraftsInput {
    pub flush_failures: Vec<(DraftId, String, DraftErrorKind)>,
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
    pending-action table, the next actions.
  - The S2–S14 rows of `each_source_makes_its_rows` (P1), including:
    - each S4 state and cause that makes a row and each that does not
      (R-14, R-15, R-40);
    - S5 and S6 from the unshared list, keyed by `Target` (R-17);
    - S6's `ShareState` (Sharing… and locked, disabled) and S7's
      `retiring`;
    - S8's `kept` once final (R-19), and `clearable` on its "Retired"
      and "Finished" lines only;
    - S13's and S14's Dismiss as `DismissRecovered` and `DismissNote`.
  - `reminders_match_to_share` (P32): S6's reminder rows are exactly the
    items Cycle 03a's count function counts in "N to share", main as one,
    for the same repository and moment.

  All fail.
- [ ] **Step 6: Implement `state_rows`.** Implement it from design A2 and
  B4. Run the tests by name. Verify lines, commit, push.

---

## Task 5: Derivation, Attention, The Log, The Count (A2, A3, A4, A6)

**Files:**

- Create:
  - `D/model/activity/log.rs`;
  - `D/view/workspace/activity.rs`;
  - `D/view/activity.rs`, declared in `D/view/mod.rs`, with its test file
    `D/view/activity_tests.rs`, for `count_and_popover_agree`.
- Modify:
  - `D/model/activity.rs` (`Inputs`, `derive`, `assign_attention`);
  - `D/view/workspace.rs` (the observers call `refresh_activity`; the
    field `activity: ActivityLog`);
  - `D/view/workspace/footer.rs` (`attention()` reads the log);
  - the refresh points of A4: `D/view/workspace/requests.rs` (pending
    conflicts land), the open item panes' title loads, `ShareTitles`, the
    navigation updates.
- Delete: `D/model/attention.rs` with its tests (E-3).
- Tests:
  - `D/model/activity_tests.rs`: `attention_counts_each_thing_once` (P2)
    and `the_log_keeps_times_and_events` (P3);
  - `D/view/activity_tests.rs`: `count_and_popover_agree` (P13, its
    count half);
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
}
pub(crate) fn derive(inputs: &Inputs<'_>) -> Vec<Derived>;
pub(crate) fn assign_attention(rows: &mut [Derived]);
impl ActivityLog {
    pub(crate) fn refresh(&mut self, derived: Vec<Derived>, checking: &BTreeSet<RepoKey>, now: OffsetDateTime) -> bool;
    pub(crate) fn rows(&self) -> &[ActivityRow];
    pub(crate) fn attention_count(&self) -> usize;
    pub(crate) fn dismiss(&mut self, key: &RowKey);
    pub(crate) fn clearable(&self) -> Vec<RowKey>;
}
impl Workspace {
    pub(in crate::desktop::view) fn refresh_activity(&mut self, cx: &mut Context<Self>);
    pub(crate) fn activity(&self) -> &ActivityLog;
}
```

- [ ] **Step 1: Design checkpoint: the N-table test first.** Write
  `attention_counts_each_thing_once` from N1–N17, one table row per
  N-row, with an N4 row for each failure class, over `derive` and the
  log. If a row cannot be expressed against the code, stop and report to
  the orchestrator. It fails.
- [ ] **Step 2: Write `the_log_keeps_times_and_events` (P3).** Cover L1–L9:
  - a retry keeps time and place;
  - a dismissed request retried returns;
  - a state that leaves and returns gets a new time;
  - an S2 row's time is its `Brought.at`;
  - a repository row through a check (L3);
  - a kept retirement line;
  - S13's and S14's Dismiss through their sources;
  - equal times;
  - "Clear finished" over `clearable` rows only.

  It fails.
- [ ] **Step 3: Implement the derivation and the log.**
  - `derive`: `request_rows` plus `state_rows`, dropping the S1 row of a
    ContextRetire whose retirement has an S8 line (R-42), then
    `assign_attention`, which applies N1–N17 over all rows. N1 and N2
    group by `operation_of` (fact 18); N4 follows `check_failed` (fact
    21).
  - `ActivityLog`, by A4's rules.
- [ ] **Step 4: Wire the Workspace.**
  - Create `D/view/activity.rs`, declared in `D/view/mod.rs`, holding for
    now only its test module.
  - `refresh_activity` marks the log dirty; one refresh runs per effect
    cycle (`cx.defer`), so a burst of notifications derives once (R-43).
  - That refresh builds `Inputs`: `RequestFacts` from Services and the
    worker; titles from an open item's `last_title()` and `share_title`;
    `each_repository`; `conflict_waits` from `pending_conflicts`; the
    clock's `utc()`.
  - It is called from every refresh point of A4, including pending
    conflicts landing and an open item's title loading.
  - `attention()` returns `attention_count()`.
- [ ] **Step 5: Delete the old count.** Delete `needs_attention` and its
  tests (E-3); move their cases into Step 1's table, listed in the ledger.
  `retire_resume_resends_original` builds its count from `derive` over
  the `World`'s operations and schedule, with no repository inputs. Apply
  E-4 to every reader it names, recording any changed count with its
  N-row.
- [ ] **Step 6: Write the count journey.** `count_and_popover_agree`
  (P13), for now comparing the footer count with the log's attention rows
  through a journey of each counted kind, each then resolved. It runs
  alone first.
- [ ] **Step 7: Run** the new tests and every E-3 and E-4 test by name.
  Verify lines, commit, push.
- [ ] **Step 8: Checkpoint.** Ticket comment: Tasks 1–5, the N-table and
  anything settled with the design at its checkpoint, the tests run with
  counts, the rulings so far.

---

## Task 6: Actions And The Test Seam (E-5 Part 1)

**Files:**

- Modify:
  - `D/keys.rs` (`ActivityMenuChoice`);
  - the handlers whose signatures change (design A1): `D/view/operations.rs`
    (`open_lifecycle`), `D/view/operations/review.rs` (`review_request`,
    `review_pending`), `D/view/workspace/requests.rs` (`abandon`,
    `run_recovery`), `D/view/operations/retire.rs` (`retire_chosen`) and
    `D/view/workspace/repository_menu.rs` (`row_action`).
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
pub(crate) struct ActivityMenuChoice { pub key: RowKey, pub action: ActivityAction, pub shown: String }

impl Workspace {
    pub(in crate::desktop::view) fn choose_activity(&mut self, choice: &ActivityMenuChoice, origin: FocusHandle, window: &mut Window, cx: &mut Context<Self>);
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

- [ ] **Step 1: Write the seam test.** `each_action_runs_its_handler`
  covers each `ActivityAction` kind through `choose_activity`. Each sends
  exactly what today's control sends, read from the recording backend or
  the dialog or sheet it opens. It fails.
- [ ] **Step 2: Implement the choice.**
  - `choose_activity` checks the row again (design D): an enabled offer
    with the same action and label. Otherwise nothing runs.
  - `run_activity_action` maps each action to its handler (A1's table),
    passing the origin.
  - The handlers of A1's signature changes gain an `origin: FocusHandle`
    parameter, or are made reachable (`pub(in crate::desktop::view)`).
  - For now the origin is today's control handle for that action
    (`control_focus_for` and `abandon_focus_for` by the old id), so
    converted tests that read focus stay green (R-45). Task 9 replaces it
    with the list's.
- [ ] **Step 3: Write the seam** functions above. `act` builds the
  `ActivityMenuChoice` for the offer and calls `choose_activity` inside
  `workspace.update`. It does not dispatch a window action:
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
  assertions in the ledger.
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
  - `D/keys.rs` (`ACTIVITY_CONTEXT` and its bindings);
  - `D/view/workspace/render.rs` (`activity_region`; mount the row menu
    layer; the menu's `ActivityMenuChoice` handler, through
    `unless_dialog`, calls `choose_activity` with the list's handle);
  - `D/view/workspace/activity.rs` (the row menu, focus);
  - `D/view/workspace/requests.rs`, `D/view/workspace/lifecycle.rs` and
    `D/view/operations/review.rs` (origins: C3);
  - `D/view/operations.rs` and its submodules (the drawing goes: F4);
  - `D/view/workspace.rs` (the focus maps of fact 8 go);
  - the sweeps at the swap: `D/view/lifecycle/privacy_tests.rs`,
    `D/view/polling_privacy_tests.rs`, `D/view/drafts_privacy_tests.rs`,
    `D/view/drafts_entry_tests.rs`;
  - `D/assets_tests.rs` (E-16).
- Tests:
  - `D/view/activity_tests.rs`: `rows_draw_from_the_log_and_filter`
    without the filter (P14), `activity_by_keyboard` (P15),
    `rows_by_pointer` (P16), `the_row_menu_by_pointer_and_keyboard`
    (P17), `a_dialog_from_a_row_returns_focus_to_its_row` (P20);
  - `each_action_runs_its_handler` extended to the drawn button and the
    menu (P18);
  - E-7, E-8, E-11, E-12, E-16.

- [ ] **Step 1: Check U-2 and U-4** with a scratch test:
  - `uniform_list` with a tracked scroll handle draws and scrolls the
    rows;
  - a key bound in `ACTIVITY_CONTEXT` reaches its handler while the list
    holds focus.

  Record both. If U-2 fails, use the Kit's `VirtualList` with the same
  keys, and record the amendment.
- [ ] **Step 2: Write the list and key tests.** Write P14 (without the
  filter), P15 (including Enter on the focused row button running its
  action once, with the list's `enter` not also firing, and Enter doing
  nothing on a disabled button or a row with neither), P16 and P20 (from
  the button, the menu and Enter; decision 23). Each runs alone first.
  They fail.
- [ ] **Step 3: Build the pane and rows** (C1, C2).
  - One-line rows, 28 px; the message and the tag end in an ellipsis,
    and the button never leaves the row (decision 22).
  - The time line is prepended at draw (B6).
  - The rows are applied after the frame, and the highlight follows the
    key; moving it calls `scroll_to_item`.
  - The empty state "Nothing yet this session".
  - The container keeps `"operation-area"` (R-20).
  - Mount it in `activity_region` below the notices and the quit row
    (R-18). Remove the old columns' drawing (F4).
- [ ] **Step 4: Write the menu tests.** Write P17 and P18's button and
  menu halves. They fail.
- [ ] **Step 5: Build the row menu** (Part D), on Cycle 03a's pattern:
  - a right press;
  - `shift-f10` and `menu`;
  - the detail lines first;
  - the re-check through `choose_activity`;
  - dropped under a modal and when its row leaves;
  - one layer at a time (R-13).
- [ ] **Step 6: Wire focus (C3; decision 23).**
  - Origins and restores become the list's handle, with the highlight on
    the row's key, from the button, the menu and Enter alike. R-45's
    interim origins end.
  - "Show it" and Drafts not saved's Cancel highlight their rows.
  - Delete the focus maps, `prune_abandon_focus` and
    `Workspace::abandon_focus`.
- [ ] **Step 7: The sweeps at the swap** (E-11, E-12; design Part G).
  - D5's sweeps: `TITLE` and `READY` are allowed inside
    `"operation-area"`; the other sentinels stay forbidden everywhere.
  - `polling_privacy`: the same title allowance; `SURFACES` updated
    explicitly; the model-rows surface, which keeps `read_at`'s
    `("operation entry", "worktree_not_clean")` pair, now from the row's
    detail (`:997`).
  - The drafts sweeps expect S11's title.
- [ ] **Step 8: Edit E-7, E-8 and E-16.** For E-7, rewrite the keyboard
  and focus steps to Part H's keys. Record each rewritten test's steps in
  the ledger.
- [ ] **Step 9: Check the size.** `D/view/operations.rs` is below 500
  lines (A13); record its count.
- [ ] **Step 10: Run** every new and edited test by name, with
  `polling_privacy` and D5's four `a16_` tests. Verify lines, commit,
  push.

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
    `navigation_sends_nothing` (P31, its Activity part);
  - E-9's Activity part, E-10.

- [ ] **Step 1: Write the tests.** Write P14's filter part, P19, P21
  (Activity moved into the right dock and set to 320 px with the layout
  model's API, as Cycle 01's dock tests do, with the longest message of
  each kind, a long repository name and "+N"), P29, P30 and P31's
  Activity part (the highlight, the filter, the row menu and ⋯, with the
  recording backend). Each runs alone first. They fail.
- [ ] **Step 2: Build the toolbar** (C1).
  - The filter, with its state as `aria_toggled` on a wrapper, and its
    empty state "Nothing needs attention".
  - ⋯ with "Clear finished" through the log (L8), disabled when no row
    is `clearable`.
  - One layer at a time (R-13).
- [ ] **Step 3: Edit E-9's Activity part and E-10.** Run every new and
  edited test by name. Verify lines, commit, push.
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
    carries P26 in the footer's status table.

- [ ] **Step 1: Check U-3** (the order of a press outside and the footer
  item's click). Record it.
- [ ] **Step 2: Write the popover tests.**
  - P22: by click, twice; by Enter, then Escape; by Space, then Escape.
  - P23: a table over design P23's sources and buttons, driven through
    the scheduler `World`, `PollingBackend` answers and fabricated
    `WorkerEvent`s, not a journey per action.
  - P24, P25 (including a dialog from the popover that closes after the
    item hid: Cycle 01's `focus_lost` fallback), P27 and P31's popover
    part; extend P13.

  Each runs alone first. They fail.
- [ ] **Step 3: Build the footer item** (E1):
  - bounds;
  - tooltip;
  - the toggle;
  - focus after hiding moves right (R-37).
- [ ] **Step 4: Build the layer** (E2):
  - anchoring, 6 px above the footer (R-38);
  - the ids `"needs-attention-…"`;
  - the header, rows and link;
  - keys, with Tab's three stops inside (R-12), and Enter on the link;
  - running an action after closing, through `choose_activity` with the
    footer item's handle as origin, and focus after an action that opens
    no dialog;
  - "Show everything in Activity";
  - closing at 0, under a modal, when another layer opens (R-13), and on
    a press outside (R-11).
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
| A10 | 9 (P15, P16, P17, P20, E-7) |
| A11 | 6, 9 (P18), 6–8 (E-5, E-6), 10, 11 (P31) |
| A12 | 11 (P27) |
| A13 | 1 (P11), 2 (P9, P10), 9 (`D/view/operations.rs` below 500 lines) |
| A14 | 13 (on screen) |
| A15 | 4 (P12, P32) |
| A16 | 3, 4 (`each_source_makes_its_rows`), 9, 10 (`rows_draw_from_the_log_and_filter`) |

**Rulings by task:**

- **Task 1:** R-34.
- **Task 2:** R-8, R-9, R-35, R-39.
- **Task 3:** R-3, R-26, R-30, R-31, R-41, R-44.
- **Task 4:** R-4, R-5, R-14, R-15, R-16, R-17, R-19, R-32, R-40, R-44.
- **Task 5:** R-1, R-2, R-6, R-7, R-22, R-23, R-24, R-28, R-42, R-43.
- **Task 6:** R-21, R-45.
- **Task 9:** decision 23 (C3), R-13, R-18, R-20, R-27, R-29, R-33, R-36.
- **Task 10:** R-13, R-23.
- **Task 11:** R-10, R-11, R-12, R-13, R-37, R-38.

**Named edits by task:**

- E-1, E-2: Task 2.
- E-3, E-4: Task 5.
- E-5: Task 6 (part 1) and Task 7 (part 2).
- E-6: Task 8.
- E-7, E-8, E-11, E-12, E-16: Task 9.
- E-9: Tasks 10 and 11.
- E-10: Task 10.
- E-13: Task 11.
- E-14: Task 12.
- E-15: any task.

**Required outcomes by task:** P1 in Tasks 3 and 4; P2, P3 and P13 in
Task 5 (P13 extended in 11); P4, P5 and P7 in Task 3; P6, P8, P12 and P32
in Task 4; P9 and P10 in Task 2; P11 in Task 1; P14 in Tasks 9 and 10;
P15, P16, P17 and P20 in Task 9; P18 in Tasks 6 and 9; P19, P21, P29 and
P30 in Task 10; P22–P27 in Task 11; P28 in Tasks 9 and 12; P31 in Tasks
10 and 11.

**Unverified facts by task:** U-1 and U-7 in Task 3; U-5 in Task 4; U-2
and U-4 in Task 9; U-3 in Task 11; U-6 in Task 0.

**Placeholders:** none. Every test, file and interface the tasks name is
in the design, and every interface is written out in full.
