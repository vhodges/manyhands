---
title: "Wave 04 Cycle 05b Implementation Plan: Close Ticket And Resolve Conflict"
date: 2026-11-15
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7FACCS575B9W0PRRCDW9N0A"
---

# Wave 04 Cycle 05b Implementation Plan: Close Ticket And Resolve Conflict

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task.
>
> **Draft, revision 4** (review rounds 1–3 and decisions 11 and 12
> applied; revision 4 had no independent review), for the owner's review. Nothing here is approved, and
> execution waits until Cycle 05a (`01M7D5Z7FZTGTC1HE25KQ1MYQY`) is
> merged to main (the ticket's Entry Gate).
>
> Where focus goes around the Resolve sheet is a table of required
> outcomes (design B3, H1–H16), and it is the contract. Task 6 writes its
> rows test-first under a design checkpoint. Re-read the design before
> each task, and record every checkpoint on ticket
> `01M7D7JK5MSY6J471JEKG3CZXR`. Steps use checkbox (`- [ ]`) syntax.

**Goal:**

- the lifecycle dialog (Close ticket… and Approve and merge…) on
  Cycle 05a's frame, with `05`'s stages, step rows, outcome box and
  footers, its behaviour unchanged;
- Resolve conflict as a right-side 920 px sheet the workspace keeps, with
  its header and footer fixed and its parts named in words;
- the copy items of the D5 follow-ups.

**Architecture.**

- **Lifecycle** (design Part A): `LifecycleDialog::stage` groups the
  phases into Ask, Running, Waiting, Done, Failed and Stopped; the dialog's
  builder puts `frame::dialog`'s header in the Kit title slot at each
  render, and sets the backdrop from the stage. One step-row drawer
  serves every stage. The ended lines go into an outcome box. Footers
  are `frame::footer`, drawn as the body's last child.
- **Resolve** (Part B): `SheetOwner::Resolve`; `Workspace::show_resolve`
  is the only way to open it, and every way in routes to it;
  `conflict::open` opens a Kit sheet with `frame::sheet_header` and
  `frame::sheet_footer`.
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-05b-close-and-resolve.md) and
[design](2026-11-15-wave-04-cycle-05b-close-and-resolve-design.md), with
`docs/design/05-sheets-and-dialogs.md` and `06`.

- "Fact n", "H-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer to the
  design; "A1"–"A8" and "B1"–"B6" are its parts' sections, except
  "A1"–"A10" in Self-Review, which are the Cycle's acceptance rows.
  "Decision n" refers to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method.
- **Cycle 05a is closed and merged to main** (the ticket's Entry Gate).
- Task 0 does not start before both.
- The design's Document Amendments 2 to 6 are applied by the planner in
  the approval commit; Amendment 1 is applied in Task 0.

**Roles.**

- The orchestrator runs the tasks, or dispatches one implementer per task
  and then a fresh reviewer, one at a time.
- It alone writes the ledger and ticket comments and pushes the ticket
  branch.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files.

**Where.**

- Every task runs in `.manyhands/worktrees/01M7D7JK5MSY6J471JEKG3CZXR` on
  `manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`.
- No lane branches and no extra worktrees.
- Every Rust command is `devenv shell -- cargo …` with `--locked`.

**Boundary.** A task changes `src/desktop/` and its tests. After each
task's commit, this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` before Tasks 0 and 5, and whenever main
moves. Never rebase.

**Push.**

- After each task's commit, the orchestrator pushes the ticket branch
  with `item sync` from the main checkout.
- It then checks that `HEAD` equals
  `origin/manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`.
- `item sync` and `comment add` merge main first when it has moved. Any
  such merge is recorded in the ledger, and the task's tests are run again
  by exact name.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.**

- Every user-visible string is a `pub(crate) const` (or a formatting
  `fn`), verbatim from the design or kept from today. Tests compare
  constants.
- Sentence case and no exclamation marks.
- `…` (U+2026) on an action that opens a confirmation or sheet (`06`).

**Test runs** (`AGENTS.md`, Testing Strategy):

- **Command.** Desktop tests run as
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- **Paths** are module paths, for example
  `desktop::view::lifecycle::outcome_tests::<name>`,
  `desktop::view::conflict::tests::<name>`,
  `desktop::view::keyboard_tests::conflict::<name>`.
- **New test files** are included as
  `#[cfg(all(test, feature = "desktop-test-support"))] #[path = "…_tests.rs"] mod …;`.
- **Each run reports more than zero tests.** A command with an empty
  filter is stopped at once.
- **Between tasks,** only the tests the task wrote or changed run, by
  exact name, each named edit included, listed in full in the ledger.
- **Module runs** (a named doubt). Tasks 1–7 change code that tests
  reach without naming it. At the end of each, the orchestrator runs
  once, by module (the command without `-- --exact`), the modules the
  task lists, and fixes what fails before the commit.
- **No whole-suite run** happens before Task 9.
- **Limits.** A command past about 15 minutes is stopped, narrowed and
  noted. Never build while a test run is in progress. A new window test
  runs alone first.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.**

- Each new behaviour's test is written and seen to fail before its code.
- Table-driven where the design gives a table (P1, P2, P6, P9).
- A row a later task builds is written in that task, not left red.
- **Focus is read after at least one draw.** A read inside the update
  that moved focus does not prove an H row.
- Guards are tested with dispatched actions, never pointer clicks, which
  the scrim takes.
- No real waits.

**No appearance tests** (Wave Testing). Nothing asserts widths, colours,
icons, tints, dividers, the header's layout or that a view renders.

**Existing tests.**

- They change only as E-1 to E-15 say.
- Any other edit is pre-authorized (decision 11). It is recorded in the
  ledger as E-16 onward with its reason, checked by the task's reviewer,
  and called out to the owner at review-ready.
- An assertion removal is called out in the ledger and the review-ready
  summary (E-3 is one).
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`, and Task N removes it.
Test-only helpers and accessors are `#[cfg(test)]`. Allows go on items,
never on a module.

**Commits.** One per task, with a conventional message, ending with the
session's own `Co-Authored-By:` trailer.

**Ticket records.**

- After Tasks 0 and 4, at Task 6's design checkpoint and its end, and
  after Task 9, a comment through
  `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
  run against the main checkout while the ticket worktree is clean.
- It records the commits, the tests run with counts (copied from the
  ledger), and the rulings.
- `comment add` publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-execution.md`,
created in Task 0 and written only by the orchestrator. Only the ledger
assigns E-numbers past E-15. Rulings take the form "Ruling: decision —
why — cost if wrong".

**File size.** New source files stay below about 500 lines. The
Workspace's new code goes in `D/view/workspace/resolve_sheet.rs`; the
lifecycle's new drawing in `D/view/lifecycle/stage.rs` and
`D/view/lifecycle/rows.rs` if `view.rs` or `running.rs` would pass about
500 lines.

**The real registry.** `manyhands-cli` runs only from the up-to-date main
checkout, never from a build of this branch.


## Review Focus

Inputs no task's main journey exercises, each pinned by a test in the
task named:

- **Task 2:** a run reopened from Activity, with no preview (P1: the
  stage from the answer); a submitted request shown as a message only
  (P1: Ask, K-10); a `busy` answer (P1: Waiting).
- **Task 3:** a Waiting answer (P4: the message in the box, no step
  failed); a failure whose blockers name the merge (P4: the message
  under Merge); a run held at a stage without its preview (P4: the stage
  words, no other notes).
- **Task 4:** Stop pressed (P2: "Stopping…" in Stop's place, the hint
  only); Done after Stop (P2: Done last); a failed run offering Retry
  and Unlock… (P2: both last, Retry first); Waiting (P2: Close first, no
  backdrop hide, P3).
- **Task 5:** each way in, the lifecycle's Repair locally… returning
  the sheet for `reopen_after`, and the lifecycle's way in under a
  prompt (P5).
- **Task 6:** a save whose `applied` reopens the lifecycle dialog (H5);
  the origin gone by close time (H6); an Unlock prompt over the sheet
  (H7), and a save answered under it (H14); Save turning to Retry by
  itself (H8); a save answered after Escape with the entity held (H12);
  quitting (H11); a draft discarded behind the sheet while typing in the
  Result (H15); Resume (H16); Ctrl+, before and right after Escape, Add
  repository, and Republish… on both paths (P7).
- **Task 7:** an external set (P8: one row of actions); an unknown front
  matter key (P8: the key as written); a typed Result (P8: "Edited").

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/model/activity/copy.rs` (`REVIEW_CHANGES`, `OPEN_SHARING_AND_KEYS`, `CHECK_HOST`, `KEEP_MINE`, `share_main`, `CHOSEN`, `EDITED`; `RESUME` re-exported), its copies' files (`D/model/review.rs`, `D/model/edit/save_state.rs`, `D/view/operations.rs`, `D/services/requests.rs`, `D/view/item/posting.rs`), and `D/model/activity_tests/` (E-8) | 1 |
| `D/model/conflict/names.rs` (new, `field_name`) | 1 |
| `D/view/lifecycle.rs` (`RESOLVE_CONFLICT` goes; `RESUME`, `share_main` re-exported; the builder), `D/view/lifecycle/outcome.rs` (`OPEN_REPOSITORY_DETAILS`) | 1, 2, 4 |
| `D/view/conflict.rs`, `conflict/render.rs`, `render/pieces.rs` (`KEEPING_*` go; `choice_of`) | 1, 5, 6, 7 |
| `D/assets.rs`, `D/assets_tests.rs` (`CircleX`, `GitMerge`; E-13) | 1 |
| `D/view/frame.rs` (`Tone`'s allows) | 2 |
| `D/view/lifecycle/view.rs`, `stage.rs` (new), `stages_tests.rs` (new) | 2, 3, 4 |
| `D/view/lifecycle/running.rs`, `confirm.rs`, `outcome.rs`, `made_ready.rs`, `rows.rs` (new) | 3, 4 |
| `D/view/keyboard_tests/lifecycle.rs` (P13) | 4 |
| `D/view/workspace/resolve_sheet.rs` (new), `resolve_sheet_tests.rs` (new) | 5, 6, 7 |
| `D/view/workspace/content.rs` (`SheetOwner`, `sync_sheet`, the details request), `settings_sheet.rs` (`SheetOwnerKind`), `requests.rs` (`ItemRoute::Resolve` in `owned()` and `open_route`), `D/view/item.rs`, `item/review.rs`, `D/view/operations/review.rs`, `D/view/conflict/entry.rs`, `D/view/conflict/submit.rs` (`close`), `D/view/lifecycle/actions.rs` (`in_workspace`), `lifecycle/repair.rs`; the tests of E-1, E-2, E-11 | 5 |
| `D/view/workspace/settings_sheet.rs` (Ctrl+, and `show_settings`), `render.rs` (the keep-focus gate), `content.rs` (`focus_content`, `apply_review`), `D/view/workspace/add_sheet.rs` (05a's `show_add_repository`), `D/view/operations/republish.rs` (R-18), `D/view/conflict/submit.rs` (`close_resolve`, R-26) | 6 |
| `D/model/conflict/merge.rs` (`PathConflict::item_kind`) | 7 |
| `D/view/keyboard_tests/conflict.rs`, `keyboard_tests/conflict/sheet.rs` (E-11, E-12) | 1, 5, 7 |
| `D/view/lifecycle/privacy_secret_tests.rs` (new, P12) | 8 |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Fetch, then the precondition.** `git fetch origin`; then
  confirm Cycle 05a is closed and merged: `git log origin/main` shows
  "Integrate closed ticket 01M7D5Z7FZTGTC1HE25KQ1MYQY". If not, stop.
- [ ] **Step 2: Merge and baseline.**

```sh
git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 3: Re-map Cycle 05a's names (U-1).** Run
  `git diff --stat ac50ba1a HEAD -- src/desktop docs/design Cargo.lock`.
  - Read the merged `SheetOwner`, `SheetOwnerKind`, `sync_sheet`,
    `show_settings`, the Ctrl+, handler, `settings_open` and how 05a
    widened it, the keep-focus checks and their gate in
    `workspace/render.rs`, `frame.rs` (`Tone`, `Footer`, `sheet_footer`),
    `wrapped_kit`, `show_add_repository`, `wait_to_quit`, and 05a's
    execution ledger (its rulings).
  - Re-read design facts 2, 13–24 and 31–34 against them. Correct the
    design first, in a commit of its own noted in the ledger.
  - If 05a's code cannot carry B1–B3 (an owner case `sync_sheet` keeps,
    a refusal in the Ctrl+, handler and in `show_add_repository`, a
    keep-focus gate that can take a second predicate), stop for the
    owner.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner.
- [ ] **Step 4: Wave amendment.** Apply the design's Document Amendment 1
  to `docs/Waves/wave-04-workspace-design.md`'s Cycle 05b section, in a
  commit of its own.
- [ ] **Step 5: Sweep the E-list (U-6) and the dialog-only guards
  (U-7),** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'lifecycle-kind' -e 'lifecycle-branch' -e 'lifecycle-title' -e 'lifecycle-ending' \
  -e 'lifecycle-stop' -e 'lifecycle-message' -e 'lifecycle-state' -e 'lifecycle-step-state' \
  -e 'STARTED' -e 'Closed by' -e 'RESOLVE_CONFLICT' -e 'KEEPING_' -e 'Review changes' \
  -e 'Open Sharing & keys' -e 'Check host' -e 'Open repository details' -e 'close_dialog' -- $P
git grep -n -e 'conflict::open' -e 'open_sheet_in' -e 'in_dialog' -e 'conflict-field' \
  -e 'conflict-choice' -e 'conflict-cancel' -e 'conflict-save' -e 'SURFACES' -e 'holds_none' -- $P
git grep -n 'has_active_dialog' -- $P ':!*_tests.rs' ':!*_tests/*'
```

  - Record each test site against its E-n, or as E-16 onward (decision
    11).
  - Classify each `has_active_dialog`-only production site: a person's
    action (needs nothing) or something that opens by itself (waits
    while `resolve_open`, R-18). `apply_review` is one to decide. List
    the second kind for Task 6.
  - List every test file each task will touch, for the module runs.
  - A production site no task names goes into the task that breaks it.
- [ ] **Step 6: Ledger, commit, push, ticket comment.** Record the base
  SHA, 05a's merge commit, Step 3's findings and Step 5's lists.

---

## Task 1: Copy And Constants (Part C)

**Interfaces.**

- `D/model/activity/copy.rs`: `REVIEW_CHANGES` "Review changes…",
  `OPEN_SHARING_AND_KEYS` "Open Sharing & keys…", `CHECK_HOST` "Check
  host…" (decision 12), which a comment's offers
  (`D/view/item/posting.rs:389-390`) now use; new `KEEP_MINE`,
  `fn share_main(primary)`, `CHOSEN` ("Chosen"), `EDITED` ("Edited").
  Each copy design Part C item 4 lists re-exports or calls the one
  definition; `RESUME` is the one `copy/states.rs:18` already defines
  (R-16).
- `D/view/lifecycle/outcome.rs`: `OPEN_REPOSITORY_DETAILS` "Open
  repository details…".
- `D/view/lifecycle.rs`: `RESOLVE_CONFLICT` goes; the button that used it
  reads `REVIEW_CHANGES`. `REPAIR_LOCALLY` stays.
- `D/view/conflict.rs`: `KEEPING_*` go; `field_state` and `body_state`
  (`render/pieces.rs`) return `CHOOSE_ONE`, `CHOSEN` or, for a typed
  Result, `EDITED`. Test-only `ResolveSheet::choice_of(&self, part)`
  gives the model's choice of a field part or the body (R-20).
- `D/model/conflict/names.rs` (new): `fn field_name(key: &str) -> Cow<'static, str>`
  (Part C table).
- `D/assets.rs`: `GitMerge` and `CircleX` in `icon_assets!`.
- The new lifecycle words as constants (design Part C, A1–A7), each
  `#[allow(dead_code)] // Used from Task N`.

- [ ] **Step 1: P9 first.** Write
  `desktop::view::conflict::copy_tests::close_and_resolve_copy_uses_the_vocabulary`
  (new `D/view/conflict/copy_tests.rs`): a table of every constant Part
  C and A1–A7 name against its words. See it fail.
- [ ] **Step 2: The constants and `field_name`,** with a unit test of
  the table's rows and an unknown key (the model half of P8).
- [ ] **Step 3: Named edits** E-7, E-8, E-9, E-11 (`choose_by_keyboard`
  only), E-13.
- [ ] **Verify:** by exact name, P9, the `field_name` test,
  `desktop::assets::tests::every_item_view_icon_loads`, and every test
  E-7, E-8, E-9 and E-11 name; then the module runs
  `desktop::model::activity::tests`, `desktop::view::conflict` (every
  `conflict::*` module), `desktop::view::keyboard_tests::conflict`,
  `desktop::view::activity::tests`, `desktop::view::needs_attention::tests`,
  `desktop::view::lifecycle::credential_tests`,
  `desktop::view::item::posting::tests` (a comment's offers),
  `desktop::view::item::edit_tests::review` (the item review's "Keep
  mine");
  the last Verify lines.
- [ ] **Commit:** `feat(desktop): Close ticket and Resolve conflict copy`.

---

## Task 2: Lifecycle Stages, Header And Git Line (A1, A2; A5's and A6's Git line)

**Interfaces.**

```rust
/// Where the dialog stands (design A1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stage { Ask, Running, Waiting, Done, Failed, Stopped }

impl LifecycleDialog {
    pub(super) fn stage(&self, cx: &App) -> Stage;
    /// The header for the Kit's title slot.
    pub(super) fn header(&self, cx: &App) -> frame::Header;
}
```

- `stage` and `submitted_body` share one helper for a submitted run, so
  they cannot disagree (R-1).
- `lifecycle::open`'s builder: `frame::dialog(base, dialog.read(cx).header(cx), cx)`
  at each render.
- `render` drops the old header; `lifecycle-kind` and `lifecycle-branch`
  go. `lifecycle-ending` stays where it is until Task 3 (R-17).
- The Git line (`lifecycle-git`, `confirm::git_line(plan)`) is drawn on
  running and ended stages when the dialog holds the preview, so the
  branch the old header named is still shown (A5, A6).
- `frame::Tone::{Danger, Success}` lose their `#[allow(dead_code)]`.

- [ ] **Step 1: P1 first.** Write
  `desktop::view::lifecycle::stages_tests::each_stage_has_its_title`
  (new `D/view/lifecycle/stages_tests.rs`), table-driven over both
  commands and the stages: confirm, running, Waiting (`busy`), done,
  failed, stopped, a reopened run without its preview, and a removed
  request (Ask, K-10); each asserts `lifecycle-title` and
  `lifecycle-subtitle`. Reuse `outcome_tests::stopped_at_integrating` and
  `outcome_canned_tests::reopen`. See it fail.
- [ ] **Step 2: The stage and the header.**
- [ ] **Step 3: Named edits** E-3, E-4, E-5.
- [ ] **Verify:** by exact name, P1 and the edited tests; then the
  module runs `desktop::view::lifecycle::tests`,
  `desktop::view::lifecycle::outcome_tests`,
  `desktop::view::lifecycle::outcome_canned_tests`,
  `desktop::view::keyboard_tests::lifecycle`,
  `desktop::view::lifecycle::privacy_tests`,
  `desktop::view::lifecycle::privacy_promote_tests`,
  `desktop::view::lifecycle::privacy_remote_tests`; the last Verify
  lines.
- [ ] **Commit:** `feat(desktop): lifecycle stages in the frame's header`.

---

## Task 3: Lifecycle Steps And Bodies (A3, A4, A6)

**Interfaces.**

- One step-row drawer for every stage (`D/view/lifecycle/rows.rs`):
  the mark (number, `CircleCheck`, `Spinner`, `CircleX`), the title, the
  note. Past confirm, the mark's wrapper keeps
  `("lifecycle-step-state", ix)` and today's words as its label (R-2).
- Notes: the preview's own notes when held; the Running step's note is
  `stage_words(stage, primary)` once a stage is known
  (`RequestState::Running { stage }`), with or without the preview; the
  Failed step's note is `lifecycle-message`; with no Failed step, the
  message goes in the box.
- The outcome box (A6), `lifecycle-ending` first; the made-ready box
  with "Now ready to start".
- Confirm: "Closed by {name} · today" for a closure to record, "Closed
  by {name}" for one already recorded (`closed_by` up to " <").
- Running: the lead moves above the steps, with the new words
  (`lifecycle-state`).

- [ ] **Step 1: P4 first.** Write
  `desktop::view::lifecycle::stages_tests::steps_show_their_state_and_progress`:
  a run held at a stage shows the stage's words as the Running step's
  note and today's state words on each mark, with and without the
  preview; a failure whose blockers name the merge shows its message
  under Merge; a Waiting answer shows it in the box. See it fail.
- [ ] **Step 2: Rows, bodies and box.**
- [ ] **Step 3: Named edit** E-6. E-14 needs none.
- [ ] **Verify:** by exact name, P4, E-6's tests and
  `desktop::view::lifecycle::outcome_tests::running_lights_each_step_from_its_stage`;
  then the module runs, all under `desktop::view::lifecycle::`,
  `outcome_tests`, `outcome_canned_tests`, `made_ready_tests`,
  `made_ready_review_tests`, `confirm_tests`, `interruption_tests`,
  `no_record_tests`, `after_tests`, `privacy_tests`,
  `privacy_promote_tests`, `privacy_remote_tests`; and
  `desktop::view::operations::resume::tests`,
  `desktop::view::operations::republish::follow_tests`,
  `desktop::view::keyboard_tests::lifecycle`; the last Verify lines.
- [ ] **Commit:** `feat(desktop): lifecycle step rows and outcome box`.

---

## Task 4: Lifecycle Footers, Backdrop And The Failed Journey (A5, A7; P13)

**Interfaces.**

- `button()` and the body helpers (`submitted_body`, `ended_buttons`,
  `republish_buttons`, `repair_buttons`, `held_action`, `blocked_body`)
  return `(primary, element)` pairs, so the footer can apply A7's rule.
- `render` draws `frame::footer(Footer { hint, buttons }, cx).id("lifecycle-footer")`
  as the body's last child, by A7's table: the dismiss button
  (`lifecycle-cancel`, labelled by the stage) first, today's non-primary
  buttons in today's order, today's primary buttons in today's order; on
  Done, Done last and primary; while running, Stop, or after it
  "Stopping…" as an inert Tab stop with a handle of its own, keeping the
  id `lifecycle-stopping` (R-23).
- `LifecycleDialog::hides_on_backdrop(&self, cx) -> bool`, true only in
  the Running stage; the builder sets `overlay_closable` from it.

- [ ] **Step 1: P2, P3 and P13 first.**
  - `desktop::view::lifecycle::stages_tests::each_stage_orders_its_footer`:
    a row per line of A7's table and per Review Focus input; each
    asserts the ids inside `lifecycle-footer` sorted by their bounds'
    left edge (design Testing), the dismiss label and the hint; Tab
    reaches the buttons in that order. The Stop-pressed row also presses
    Tab and Shift+Tab three times each and asserts focus stays inside
    `lifecycle-dialog`, then Escape hides the dialog and focus is on the
    origin.
  - `::the_backdrop_hides_only_a_running_dialog`: a click outside the
    dialog, below the title bar (`simulate_click`), hides a running
    dialog, the run goes on and focus is on the origin; on confirm, on
    Waiting and on a failure it does nothing.
  - `desktop::view::keyboard_tests::lifecycle::a_failed_run_is_retried_by_keyboard`
    (P13), for both commands: the library's failure point
    `BeforeRequestDomainCall` fails the run with `internal_error` and
    offers Retry, set up as `outcome_tests::a_stopped_run_with_no_record_offers_prepare`
    does (`D/view/lifecycle/outcome_tests.rs:299-323`); the normal
    service is put back without moving the branch; Tab to Retry, Enter;
    then Done by Enter; focus on the origin.

  See them fail.
- [ ] **Step 2: Footers and backdrop.**
- [ ] **Verify:** by exact name, P2, P3, P13; then the module runs
  `desktop::view::keyboard_tests::lifecycle`,
  `desktop::view::lifecycle::{tests, confirm_tests, stale_tests, outcome_tests, outcome_canned_tests, credential_tests, action_tests, check_now_tests}`,
  `desktop::view::operations::republish::tests` and
  `desktop::view::operations::republish::stop_tests` (the module of
  `republish_stop_tests.rs`, whose `lifecycle-stopping` reads stay),
  `desktop::view::workspace::settings_sheet::tests`; the last Verify
  lines.
- [ ] **Commit:** `feat(desktop): lifecycle footers, action last`.
- [ ] **Ticket comment** (Tasks 1–4).

---

## Task 5: Resolve As A Sheet: Owner And Ways In (B1, B2)

**Interfaces.**

- `SheetOwner::Resolve(WeakEntity<ResolveSheet>)`, `SheetOwnerKind::Resolve`
  (`settings_sheet.rs:41`); `sync_sheet`'s two cases (B1); a requested
  details sheet dropped while `resolve_open`; `Workspace::resolve_open`
  and `Workspace::workspace_sheet_open`.
- `Workspace::show_resolve` in `D/view/workspace/resolve_sheet.rs` (B2):
  refusals, `conflict::open`, the owner in the same update, then the
  sheet's own handle focused.
- `conflict::open` opens a Kit sheet: `window.open_sheet`, sized
  `min(920 px, window width − 48 px)` (R-28), `overlay_closable(false)`;
  the title slot holds today's header inside `conflict-header`; the
  footer slot holds today's Git line and buttons inside `conflict-footer`
  with `RESOLVE_SHEET_CONTEXT`, `SaveResolution` and `footer_focus`; the
  body laid over the Kit's, keeping today's own scroll without
  `max_h(px(720.))` (R-12); `on_close` focuses the origin. It becomes
  `pub(in crate::desktop::view)`.
- The ways in (B2's table): `ItemRoute::Resolve(Box<conflict::Opening>)`
  (`Opening: Clone`; an arm in `owned()` and one in `open_route`);
  `Entry::opening`; `in_workspace` generic over its return value, and
  `open_sheet` without its own close. R-24 on the lifecycle's ways in: a
  press goes on only when focus was inside the lifecycle dialog before
  `on` moved it; Repair locally…'s sheet, opened when the local
  synchronization answers, waits on the dialog until a render finds
  focus back inside it (U-10).
- Cancel is re-pointed to `ResolveSheet::close`, which Cancel, Resume
  and `applied` all use; it calls `close_sheet` for now, and Task 6
  guards it.

- [ ] **Step 1: P5 first.** Write
  `desktop::view::workspace::resolve_sheet::tests::resolve_is_a_sheet_the_workspace_keeps`:
  from each way in (the banner, an Activity row, a pending row, a
  popover row, the lifecycle's Review changes… and Repair locally…), a
  sheet is active, no dialog, the owner is `Resolve`, and it survives
  renders; Escape, the close button, Cancel, Resume and a save each
  clear the owner; with a prompt opened over the lifecycle dialog, its
  Review changes… press does nothing, and a Repair locally… answer opens
  the sheet only once the prompt has closed. See it fail.
- [ ] **Step 2: Owner, opening, ways in.**
- [ ] **Step 3: Named edits** E-1, E-2, E-11 (`in_sheet`, R-19).
- [ ] **Verify:** by exact name, P5 and the edited tests; then the
  module runs `desktop::view::conflict` (every `conflict::*` module),
  `desktop::view::keyboard_tests::conflict`,
  `desktop::view::workspace::tests`,
  `desktop::view::workspace::settings_sheet::tests`,
  `desktop::view::activity::tests`, `desktop::view::needs_attention::tests`,
  `desktop::view::lifecycle::{tests, outcome_canned_tests, credential_tests, privacy_tests, privacy_remote_tests}`
  (every `in_workspace` caller), `desktop::view::operations::resume::tests`;
  the last Verify lines.
- [ ] **Commit:** `feat(desktop): Resolve conflict as a sheet the workspace keeps`.

---

## Task 6: Resolve: Closing, Focus And Refusals (B1–B3; design checkpoint)

**Interfaces.**

- `Workspace::close_resolve(&Entity<ResolveSheet>, window, cx) -> bool`:
  closes the Kit sheet only while the owner is that sheet.
  `ResolveSheet::close` reaches it through Root, as `in_workspace` does;
  on false nothing more runs (no focus, no lifecycle reopen, no
  `follow_resume`) (R-22, R-27).
- An `applied` that lands while a dialog is open keeps its answer on
  the sheet and sets the Workspace's `resolve_waiting`;
  `run_waiting_resolve_close`, called from `Workspace::render`, runs the
  rest through `cx.defer_in` once no dialog is open, as 05a's
  `run_waiting_add_close` does (R-26).
- Footer presses focus the sheet's own handle first; `conflict-footer`
  tracks `footer_focus`; body presses do not move focus (B3).
- `focus_content` and `apply_review`'s refocus skip while
  `workspace_sheet_open` (05a's `sheet_held`, or `resolve_open`);
  `apply_review` keeps its Drafts and Settings requests until the sheet
  closes (R-25).
- `keep_focus_in_resolve`, gated in `Workspace::render` beside 05a's
  `keep_focus_in_sheet`, on `resolve_open`; `keep_focus_in_sheet` and
  `sheet_keeps_lost_focus` do not take Resolve (R-11).
- `settings_key`, `show_settings` and 05a's `show_add_repository` refuse
  while `resolve_open` (decision 9); `republish_once_stopped` and
  `open_deferred_republish` wait while it, and each site Task 0 listed
  (R-18, U-7, U-9).

- [ ] **Step 1: Design checkpoint, tests first.** Write
  `desktop::view::workspace::resolve_sheet::tests::{focus_around_the_resolve_sheet,
  nothing_opens_over_the_resolve_sheet}` (P6 with one row per H1–H16;
  P7) before any production code.
  - P7: Ctrl+, does nothing, and opens Settings right after Escape;
    `show_settings` and `show_add_repository` refuse; a details sheet
    asked for does not open, then or after the sheet closes; a workspace
    action (a region focus key) and an Activity action are refused;
    `show_resolve` refuses over a dialog and over a sheet; a share's
    Stop and republish…, its share stopped while the sheet is open,
    opens Republish… only once it closes, on each of the two paths.
  - If a row cannot be written against the code, or the code would
    answer it differently, **stop and bring it to the owner.**
  - Comment on the ticket with the rows written and any stop.
  See them fail.
- [ ] **Step 2: Closing, waiting under a dialog, presses, keep-focus.**
- [ ] **Step 3: Refusals, focus moves behind, R-18.**
- [ ] **Verify:** by exact name, P6 and P7; then the module runs
  `desktop::view::conflict`, `desktop::view::keyboard_tests::conflict`,
  `desktop::view::workspace::settings_sheet::tests`,
  `desktop::view::workspace::tests`,
  `desktop::view::operations::republish`,
  `desktop::view::tabs::tests`, `desktop::view::item::draft_actions::tests`
  and `desktop::view::create::tests` (the discard, close-with-draft and
  `open_created` paths `focus_content` serves), and the Add sheet's module (05a's
  `desktop::view::workspace::add_sheet::tests`); the last Verify lines.
- [ ] **Commit:** `feat(desktop): Resolve conflict's closes, focus and refusals`.
- [ ] **Ticket comment** (Tasks 5–6).

---

## Task 7: Resolve Header, Cards And Footer (B4–B6)

**Interfaces.**

- The title slot: `frame::sheet_header` with `conflict-title` and
  `conflict-description`, inside `conflict-header`.
- The body: meta line, note, status, cards (B4).
- `PathConflict::item_kind()` (`D/model/conflict/merge.rs`), from the
  kind its parse finds.
- Cards (B5): the name (`field_name`), the kind
  (`("conflict-field-kind", k)`), the state, the choices on the right
  with a check on the chosen one; Before with "last shared copy";
  outside cards in the card frame, one shared row of actions.
- The footer slot: `frame::sheet_footer(Footer { hint: None, buttons: [git line, Cancel, primary] })`
  inside `conflict-footer` (B6, R-15).

- [ ] **Step 1: P8 first.** Write
  `desktop::view::workspace::resolve_sheet::tests::parts_are_named_in_words`:
  a ticket conflict on `status` and an unknown key reads "Status" and
  the key, each with "Ticket detail"; a choice reads "Chosen"; a typed
  Result reads "Edited"; an external set draws one card per path and one
  row of actions. See it fail.
- [ ] **Step 2: Header, cards, footer.**
- [ ] **Step 3: Named edits** E-10, E-12.
- [ ] **Verify:** by exact name, P8, E-10's and E-12's tests; then the
  module runs `desktop::view::conflict`,
  `desktop::view::keyboard_tests::conflict`,
  `desktop::view::workspace::resolve_sheet::tests`; the last Verify
  lines.
- [ ] **Commit:** `feat(desktop): Resolve conflict's header, cards and footer`.

---

## Task 8: No Secret, And The Keyboard Journeys (P12; A1, A3)

- [ ] **Step 1: P12 first.** Write
  `desktop::view::lifecycle::privacy_secret_tests::close_and_resolve_show_no_secret`
  (new `D/view/lifecycle/privacy_secret_tests.rs`), reading every label
  and value as `passphrase_tests`' `window_text` does
  (`D/view/passphrase_tests.rs:537-547`): a key imported with
  `Fixture::import_key`, whose private file holds `PRIVATE_KEY_SENTINEL`
  (`D/test_support/fixtures.rs:908`, `:940-985`); a close answered
  `unlock_required`; Unlock… opens the prompt, where a sentinel
  passphrase is typed and supplied, as `credential_tests`' `ask` and
  `a_resend_without_a_secret_takes_the_unlock_path` drive it
  (`D/view/lifecycle/credential_tests.rs:157`, `:204`); Retry, then
  Done, by canned answers; then a remote conflict's sheet. At each stage
  no label or value in the window, the dialog's and sheet's slots
  included, holds either sentinel. It
  passes on Task 7's code; record that it was seen to fail with the
  sentinel planted in a label, then the plant removed.
- [ ] **Step 2: Run** by exact name the keyboard journeys
  (`desktop::view::keyboard_tests::lifecycle::{close_a_ticket_by_keyboard, approve_and_merge_a_document_by_keyboard, a_failed_run_is_retried_by_keyboard}`,
  `desktop::view::keyboard_tests::conflict::*`) and the a16 privacy tests
  (`desktop::view::lifecycle::privacy_tests::a16_a_close_broken_refused_repaired_and_done_shows_no_item_text`,
  `::privacy_promote_tests::a16_a_promotion_stopped_untidied_and_done_shows_no_item_text`,
  `::privacy_remote_tests::a16_a_shared_conflict_and_its_resolution_show_no_item_text`).
- [ ] **Verify:** the last Verify lines.
- [ ] **Commit:** `test(desktop): no secret on the close and resolve surfaces`.

---

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: Merge main.** `git fetch origin && git merge origin/main`;
  `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] **Step 2: Gates** (`AGENTS.md`): `cargo check --all-features --locked`,
  `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`;
  then the fast tier once. No SSH target (desktop-only).
- [ ] **Step 3: On-screen checklist**
  `docs/plans/2026-11-15-wave-04-cycle-05b-on-screen-checklist.md`:
  frames 2c, 2d, 2e and 2g against the app, in the dark theme, by the
  owner; each Known Limit named.
- [ ] **Step 4: Acceptance Status** in the Cycle document, from the
  ledger and the tests by exact name; the ticket's checklist ticked.
- [ ] **Step 5: Review-ready.** Ledger, commit, push, ticket comment
  with the review-ready summary (every E-n, each assertion removal,
  every ruling).
- [ ] **Step 6: After review approval,** the owner sets the resolution
  status and closes the ticket with `ticket close` from the main
  checkout, which merges it. Before the close, `target/` and `.devenv/`
  are removed from the worktree (a close stops on ignored files). Not
  this session's.

## Plan Self-Review

| Acceptance row / ruling | Task |
| --- | --- |
| A1 (close and approval by keyboard) | 2, 3, 4 (P13), 8 |
| A2 (stages, steps, box, footers) | 2, 3, 4 |
| A3 (resolving by keyboard) | 5, 6, 7, 8 |
| A4 (a sheet the workspace keeps) | 5 |
| A5 (focus returns) | 4, 5, 6 |
| A6 (names, Chosen, Edited; nothing behind) | 1, 6, 7 |
| A7 (copy) | 1 |
| A8 (no secret) | 8 |
| A9 (D5's behaviour) | every task's module runs; 9 |
| A10 (on screen) | 9 |
| R-1, R-17 | 2, 3 |
| R-2, R-3, R-4 | 3 |
| R-5 | 4 |
| R-8, R-9, R-12, R-19, R-21, R-24, R-28 | 5 |
| R-11, R-18, R-22, R-25, R-26, R-27 | 6 |
| R-23 | 4 |
| R-13, R-14, R-15 | 7 |
| R-16, R-20 | 1 |
