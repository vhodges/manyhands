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
> **Draft**, for the owner's review. Nothing here is approved, and
> execution waits until Cycle 05a (`01M7D5Z7FZTGTC1HE25KQ1MYQY`) is
> merged to main (the ticket's Entry Gate).
>
> Where focus goes around the Resolve sheet is a table of required
> outcomes (design B3, H1–H10), and it is the contract. Task 5 writes its
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
  phases into Ask, Running, Done, Failed and Stopped; the dialog's
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
- The design's Document Amendments 2 and 3 are applied by the planner in
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
- **Module runs** (a named doubt). Tasks 2–6 change code that tests
  reach without naming it. At the end of each, the orchestrator runs
  once, by module (the command without `-- --exact`), the modules the
  task lists, and fixes what fails before the commit.
- **No whole-suite run** happens before Task 8.
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
- Any other edit is recorded in the ledger as E-16 onward with its
  reason, checked by the task's reviewer, and called out to the owner
  at review-ready.
- An assertion removal is called out in the ledger and the review-ready
  summary (E-3 and E-4 are two).
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`, and Task N removes it.
Test-only helpers and accessors are `#[cfg(test)]`. Allows go on items,
never on a module.

**Commits.** One per task, with a conventional message, ending with the
session's own `Co-Authored-By:` trailer.

**Ticket records.**

- After Tasks 0, 4, 6 and 8, a comment through
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
  (P1: Ask, K-10).
- **Task 3:** a failed run whose blockers name no step (P4: the message
  in the box, not under a step); a run shown without its preview (P4:
  no notes drawn).
- **Task 4:** Stop pressed (P2: the footer holds the hint only); Done
  after Stop (P2: Done last); a Failed run with both Retry and Unlock…
  (P2: both primaries last, in today's order).
- **Task 5:** Review changes… from the lifecycle dialog with a
  Republish… deferred (P7, R-18); a save whose `applied` reopens the
  lifecycle dialog (P6: H5); the origin gone by close time (P6: H6); an
  Unlock prompt over the sheet (P6: H7); Ctrl+, while it is open (P7).
- **Task 6:** an external set (P8: one row of actions); an unknown front
  matter key (P8: the key as written).

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/model/activity/copy.rs` (`REVIEW_CHANGES`, `RESUME`, `KEEP_MINE`, `share_main`, `CHOSEN`) | 1 |
| `D/view/lifecycle.rs` (`RESOLVE_CONFLICT` goes; `RESUME`, `share_main` re-exported; builder) | 1, 2, 4 |
| `D/view/conflict.rs`, `conflict/render.rs`, `render/pieces.rs` (`KEEPING_*` go; `choice_of`) | 1, 5, 6 |
| `D/assets.rs`, `D/assets_tests.rs` (`CircleX`, `GitMerge`; E-13) | 1 |
| `D/view/frame.rs` (`Tone`'s allows) | 2 |
| `D/view/lifecycle/view.rs`, `stage.rs` (new), `stages_tests.rs` (new) | 2, 3, 4 |
| `D/view/lifecycle/running.rs`, `confirm.rs`, `outcome.rs`, `made_ready.rs`, `rows.rs` (new) | 3, 4 |
| `D/view/workspace/resolve_sheet.rs` (new), `resolve_sheet_tests.rs` (new) | 5 |
| `D/view/workspace/content.rs` (`SheetOwner`, `sync_sheet`), `settings_sheet.rs` (Ctrl+, and `show_settings` refusals; the keep-focus check), `requests.rs` (`open_route`), `D/view/operations/republish.rs` (R-18) | 5 |
| `D/view/item.rs` (`ItemRoute::Resolve`), `item/review.rs`, `D/view/operations/review.rs`, `D/view/conflict/entry.rs`, `D/view/lifecycle/repair.rs` | 5 |
| `D/view/conflict/submit.rs` (`close_sheet`) | 5 |
| `D/view/keyboard_tests/conflict.rs` (E-11, E-12) | 1, 5, 6 |
| `D/view/lifecycle/privacy_tests.rs`, `privacy_promote_tests.rs`, `privacy_remote_tests.rs` (E-15) | 7 |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8.

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
    `show_settings`, the Ctrl+, handler, the widened `settings_open`, the
    keep-focus check (`keep_focus_in_settings` or its successor),
    `frame.rs` (`Tone`, `Footer`, `sheet_footer`), `wrapped_kit`, the Add
    sheet's `show_add_repository`, and 05a's execution ledger (its
    rulings).
  - Re-read design facts 2, 13–24 and 31–33 against them. Correct the
    design first, in a commit of its own noted in the ledger.
  - If 05a's code cannot carry B1–B3 (an owner case `sync_sheet` keeps,
    a refusal in the Ctrl+, handler, a keep-focus check that can take a
    third owner), stop for the owner.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner.
- [ ] **Step 4: Wave amendment.** Apply the design's Document Amendment 1
  to `docs/Waves/wave-04-workspace-design.md`'s Cycle 05b section, in a
  commit of its own.
- [ ] **Step 5: Sweep the E-list (U-6) and the dialog-only guards
  (U-7),** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'lifecycle-kind' -e 'lifecycle-branch' -e 'lifecycle-title' -e 'Closed by' \
  -e 'RESOLVE_CONFLICT' -e 'KEEPING_' -e '"Review changes' -e 'close_dialog' -- $P
git grep -n -e 'conflict::open' -e 'open_sheet_in' -e 'in_dialog' -e 'conflict-field' \
  -e 'SURFACES' -e 'holds_none' -- $P
git grep -n 'has_active_dialog' -- $P ':!*_tests.rs' ':!*_tests/*'
```

  - Record each test site against its E-n, or as E-16 onward.
  - Classify each `has_active_dialog`-only production site: a person's
    action (needs nothing) or something that opens by itself (waits
    while the owner is `Resolve`, R-18). List the second kind for Task 5.
  - List every test file each task will touch, for the module runs.
  - A production site no task names goes into the task that breaks it.
- [ ] **Step 6: Ledger, commit, push, ticket comment.** Record the base
  SHA, 05a's merge commit, Step 3's findings and Step 5's lists.

---

## Task 1: Copy And Constants (Part C)

**Interfaces.**

- `D/model/activity/copy.rs`: `REVIEW_CHANGES` becomes "Review
  changes…"; new `RESUME`, `KEEP_MINE`, `fn share_main(primary)`,
  `CHOSEN` ("Chosen"); today's copies re-export them (R-16).
- `D/view/lifecycle.rs`: `RESOLVE_CONFLICT` goes; the button that used it
  reads `REVIEW_CHANGES`. `REPAIR_LOCALLY` stays.
- `D/view/conflict.rs`: `KEEPING_MINE`, `KEEPING_THEIRS`, `KEEPING_BOTH`,
  `KEEPING_EDITED` go; `field_state` and `body_state`
  (`render/pieces.rs`) return `CHOOSE_ONE` or `CHOSEN`. Test-only
  `ResolveSheet::choice_of(&self, part: PartRef) -> Option<…>` gives the
  model's choice of a field part or the body (R-20).
- `D/assets.rs`: `CircleX` and `GitMerge` in `icon_assets!` (used from
  Task 2: `#[allow]` not needed, the macro registers assets).
- The new lifecycle words as constants (design Part C, A1–A7), each
  `#[allow(dead_code)] // Used from Task N`.

- [ ] **Step 1: P9 first.** Write
  `desktop::view::conflict::copy_tests::close_and_resolve_copy_uses_the_vocabulary`
  (new `D/view/conflict/copy_tests.rs`): a table of every constant Part
  C and A1–A7 name against its words. See it fail.
- [ ] **Step 2: The constants and the field-name table** (Part C), as a
  `fn field_name(key: &str) -> Cow<'static, str>` in
  `D/model/conflict/merge.rs` or a new `D/model/conflict/names.rs`, with
  a unit test of the table's rows and an unknown key (part of P8, the
  model half).
- [ ] **Step 3: Named edits** E-7, E-8, E-9, E-11 (`choose_by_keyboard`
  only), E-13.
- [ ] **Verify:** by exact name, P9, the field-name unit test,
  `desktop::assets::tests::every_item_view_icon_loads`,
  `desktop::view::conflict::entry_tests::resolve_conflict_opens_the_childs_sheet`,
  `desktop::view::conflict::tests::{save_is_disabled_until_every_part_is_chosen, choices_wait_while_a_save_is_out}`,
  `desktop::view::conflict::stale_tests::a_stale_observation_refreshes_the_comparison`,
  `desktop::view::keyboard_tests::conflict::{resolve_a_local_conflict_by_keyboard, resolve_a_shared_conflict_by_keyboard}`,
  and the three model tests of E-8. Then the last Verify lines.
- [ ] **Commit:** `feat(desktop): Close ticket and Resolve conflict copy`.

---

## Task 2: Lifecycle Stages And Header (A1, A2)

**Interfaces.**

```rust
/// Where the dialog stands (design A1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stage { Ask, Running, Done, Failed, Stopped }

impl LifecycleDialog {
    pub(super) fn stage(&self, cx: &App) -> Stage;
    /// The header for the Kit's title slot.
    pub(super) fn header(&self, cx: &App) -> frame::Header;
    /// True only while running (A5, R-5).
    pub(super) fn hides_on_backdrop(&self, cx: &App) -> bool;
}
```

- `stage` reads what `submitted_body` reads (R-1); `submitted_body`
  and `stage` share one helper so they cannot disagree.
- `lifecycle::open`'s builder: `frame::dialog(base, dialog.read(cx).header(cx), cx)`
  at each render; the `hides_on_backdrop` part is Task 4.
- `render` drops the old header; `lifecycle-kind` and `lifecycle-branch`
  go. `lifecycle-ending` stays (R-17; Task 3 moves it into the box).
- `frame::Tone::{Danger, Success}` lose their `#[allow(dead_code)]`.

- [ ] **Step 1: P1 first.** Write
  `desktop::view::lifecycle::stages_tests::each_stage_has_its_title`
  (new `D/view/lifecycle/stages_tests.rs`), table-driven over both
  commands and the stages: confirm, running, done, failed, stopped, a
  reopened run without its preview, and a removed request (Ask, K-10);
  each asserts `lifecycle-title` and `lifecycle-subtitle`. Reuse the
  fixtures of `outcome_tests` (`stopped_at_integrating`) and
  `outcome_canned_tests` (`reopen`). See it fail.
- [ ] **Step 2: The stage and the header.**
- [ ] **Step 3: Named edits** E-3, E-4, E-5.
- [ ] **Verify:** by exact name, P1 and the three edited tests; then the
  module runs `desktop::view::lifecycle::tests`,
  `desktop::view::lifecycle::outcome_tests`,
  `desktop::view::lifecycle::outcome_canned_tests`,
  `desktop::view::keyboard_tests::lifecycle`,
  `desktop::view::lifecycle::privacy_tests`,
  `desktop::view::lifecycle::privacy_promote_tests`; the last Verify
  lines.
- [ ] **Commit:** `feat(desktop): lifecycle stages in the frame's header`.

---

## Task 3: Lifecycle Steps And Bodies (A3, A4, A6)

**Interfaces.**

- One step-row drawer for every stage (`D/view/lifecycle/rows.rs`):
  the mark (number, `CircleCheck`, `Spinner`, `CircleX`), the title,
  the note. The mark's wrapper keeps `("lifecycle-step-state", ix)` and
  today's words as its label on running and ended (R-2).
- Running: the Running step's note is `stage_words(stage, primary)` for
  the request's last stage (`RequestState::Running { stage }`), else its
  own note.
- Failed: the Failed step's note is `lifecycle-message`; with no Failed
  step, the message goes in the box.
- The outcome box (A6), with `lifecycle-ending` first; the made-ready
  box with "Now ready to start".
- Confirm: "Closed by {name} · today" (`closed_by` up to " <").
- Running: the lead moves above the steps, with the new words
  (`lifecycle-state`).

- [ ] **Step 1: P4 first.** Write
  `desktop::view::lifecycle::stages_tests::steps_show_their_state_and_progress`:
  a run held at a stage shows the stage's words as the Running step's
  note and today's state words on each mark; a failed run shows its
  message as the Failed step's note; a failed run with no Failed step
  shows it in the box; a run without its preview draws no notes. See it
  fail.
- [ ] **Step 2: Rows, bodies and the box.**
- [ ] **Step 3: Named edit** E-6. E-14 needs none.
- [ ] **Verify:** by exact name, P4, E-6's two tests and
  `desktop::view::lifecycle::outcome_tests::running_lights_each_step_from_its_stage`;
  then the module runs `desktop::view::lifecycle::outcome_tests`,
  `outcome_canned_tests`, `made_ready_tests`, `made_ready_review_tests`,
  `confirm_tests`, `interruption_tests`, `no_record_tests`,
  `after_tests`, `privacy_tests`, `privacy_promote_tests`,
  `privacy_remote_tests` (all under `desktop::view::lifecycle::`),
  `desktop::view::operations::resume::tests`,
  `desktop::view::operations::republish::follow_tests`,
  `desktop::view::keyboard_tests::lifecycle`; the last Verify lines.
- [ ] **Commit:** `feat(desktop): lifecycle step rows and outcome box`.

---

## Task 4: Lifecycle Footers And Backdrop (A5, A7)

**Interfaces.**

- `render` draws `frame::footer(Footer { hint, buttons }, cx).id("lifecycle-footer")`
  as the body's last child. `buttons` follows A7's rule (R-6): the
  dismiss button first (`lifecycle-cancel`, labelled by the stage), today's
  other non-primary buttons in today's order, today's primary buttons in
  today's order; on Done, Done last and primary; while running, Stop
  only, no dismiss.
- The builder sets `overlay_closable(dialog.read(cx).hides_on_backdrop(cx))`.

- [ ] **Step 1: P2 and P3 first.** Write
  `desktop::view::lifecycle::stages_tests::each_stage_orders_its_footer`
  (table over A7's rows: the ids inside `lifecycle-footer` in drawn
  order, the dismiss label, the hint; Tab reaches them in that order)
  and `::the_backdrop_hides_only_a_running_dialog` (U-4: a click on the
  overlay if the harness can place one, else `hides_on_backdrop` per
  stage; the run goes on after the hide, as Escape's). See them fail.
- [ ] **Step 2: Footers and backdrop.**
- [ ] **Verify:** by exact name, P2, P3; then the module runs
  `desktop::view::keyboard_tests::lifecycle`,
  `desktop::view::lifecycle::{tests, confirm_tests, stale_tests, outcome_tests, outcome_canned_tests, credential_tests, action_tests, check_now_tests}`,
  `desktop::view::workspace::settings_sheet::tests` (Open Sharing & keys
  from the dialog); the last Verify lines.
- [ ] **Commit:** `feat(desktop): lifecycle footers, action last`.
- [ ] **Ticket comment** (Tasks 1–4).

---

## Task 5: Resolve As A Sheet: Owner, Ways In, Focus (B1–B3; design checkpoint)

**Interfaces.**

- `SheetOwner::Resolve(WeakEntity<ResolveSheet>)`, `SheetOwnerKind::Resolve`;
  `sync_sheet`'s two cases (B1); no details sheet while `Resolve`.
- `Workspace::show_resolve(opening, window, cx) -> Option<Entity<ResolveSheet>>`
  in `D/view/workspace/resolve_sheet.rs` (B2).
- `conflict::open` opens a Kit sheet: `window.open_sheet`, `size(px(920.))`,
  `overlay_closable(false)`, the title slot (today's header for now,
  inside `conflict-header`), the footer slot (today's Git line and
  buttons for now, inside `conflict-footer` with `RESOLVE_SHEET_CONTEXT`
  and `SaveResolution`), `on_close` focusing the origin. It becomes
  `pub(in crate::desktop::view)`; its one caller is `show_resolve`.
- The ways in (B2's table): `ItemRoute::Resolve(Box<conflict::Opening>)`
  emitted by `ItemPane::review_conflict` and handled in `open_route`;
  `Entry::opening` used by `review_request` and `review_pending`;
  `LifecycleDialog::open_sheet` through `in_workspace`.
- Cancel and `applied` (`conflict/submit.rs:429`, `render.rs:493`) call
  `close_sheet`.
- `settings_key` and `show_settings` refuse while the owner is `Resolve`
  (R-10); the keep-focus check covers `Resolve` (R-11); the deferred
  Republish… and any site Task 0 listed wait while it is `Resolve`
  (R-18).
- The view drops its own `max_h` and scroll (R-12).

- [ ] **Step 1: Design checkpoint, tests first.** Write
  `desktop::view::workspace::resolve_sheet::tests::{resolve_is_a_sheet_the_workspace_keeps,
  focus_around_the_resolve_sheet, nothing_opens_over_the_resolve_sheet}`
  (P5, P6 with one row per H1–H10, P7), before any production code.
  - P5: from each way in, a sheet is active, no dialog, the owner is
    `Resolve`, and it survives renders; each way out clears the owner.
  - P7: Ctrl+, does nothing; `show_settings` refuses; a details sheet
    asked for does not open; `show_resolve` refuses over a dialog and
    over a sheet; a deferred Republish… waits until the sheet closes.
  - If a row cannot be written against the code, or the code would
    answer it differently, **stop and bring it to the owner.**
  See them fail.
- [ ] **Step 2: Owner, opening and ways in.**
- [ ] **Step 3: Closing, refusals, keep-focus, R-18.**
- [ ] **Step 4: Named edits** E-1, E-2, E-11 (`in_sheet`, R-19).
- [ ] **Verify:** by exact name, P5–P7 and the edited tests; then the
  module runs `desktop::view::conflict` (every `conflict::*` module),
  `desktop::view::keyboard_tests::conflict`,
  `desktop::view::workspace::settings_sheet::tests`,
  `desktop::view::workspace::tests`, `desktop::view::activity::tests`,
  `desktop::view::needs_attention::tests`,
  `desktop::view::operations::republish`,
  `desktop::view::lifecycle::privacy_tests`,
  `desktop::view::lifecycle::privacy_remote_tests`; the last Verify
  lines.
- [ ] **Commit:** `feat(desktop): Resolve conflict as a sheet the workspace keeps`.

---

## Task 6: Resolve Header, Cards And Footer (B4–B6)

**Interfaces.**

- The title slot: `frame::sheet_header` with `conflict-title` and
  `conflict-description`, inside `conflict-header`.
- The body: meta line, note, status, cards (B4).
- Cards (B5): name in words (`field_name`), kind
  (`("conflict-field-kind", k)`), state, choices on the right; Before
  with "last shared copy"; outside cards in the card frame.
- The footer slot: `frame::sheet_footer(Footer { hint: None, buttons: [git line, Cancel, primary] })`
  inside `conflict-footer` (B6, R-15).

- [ ] **Step 1: P8 first.** Write
  `desktop::view::workspace::resolve_sheet::tests::parts_are_named_in_words`:
  a ticket conflict on `status` and an unknown key reads "Status" and
  the key, with "Ticket detail"; a choice reads "Chosen"; an external
  set draws one card per path and one row of actions. See it fail.
- [ ] **Step 2: Header, cards, footer.**
- [ ] **Step 3: Named edits** E-10, E-12.
- [ ] **Verify:** by exact name, P8, E-10's and E-12's tests; then the
  module runs `desktop::view::conflict`,
  `desktop::view::keyboard_tests::conflict`; the last Verify lines.
- [ ] **Commit:** `feat(desktop): Resolve conflict's header, cards and footer`.
- [ ] **Ticket comment** (Tasks 5–6).

---

## Task 7: Privacy And The Keyboard Journeys (P10, A1, A3)

- [ ] **Step 1: E-15.** Extend `SURFACES` and the a16 `holds_none` lists
  with `conflict-header`, `conflict-footer` and `lifecycle-title`. Check
  U-8 first: a labelled element whose own id is a surface is filed under
  it; if not, wrap the header in an element with a surface id and record
  the ruling.
- [ ] **Step 2: Run** by exact name the privacy tests
  (`desktop::view::lifecycle::privacy_tests::a16_a_close_broken_refused_repaired_and_done_shows_no_item_text`,
  `::privacy_promote_tests::a16_a_promotion_stopped_untidied_and_done_shows_no_item_text`,
  `::privacy_remote_tests::a16_a_shared_conflict_and_its_resolution_show_no_item_text`
  and the rest of those three modules) and the keyboard journeys
  (`desktop::view::keyboard_tests::lifecycle::{close_a_ticket_by_keyboard, approve_and_merge_a_document_by_keyboard}`,
  `desktop::view::keyboard_tests::conflict::*`).
- [ ] **Verify:** the last Verify lines.
- [ ] **Commit:** `test(desktop): privacy sweeps cover the moved headers and footers`.

---

## Task 8: Checklist, Gates, Review-Ready

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
  ledger and the tests by exact name.
- [ ] **Step 5: Review-ready.** Ledger, commit, push, ticket comment
  with the review-ready summary (every E-n, each assertion removal,
  every ruling).
- [ ] **Step 6: After review approval,** the owner sets the resolution
  status and closes the ticket with `ticket close` from the main
  checkout, which merges it. Not this session's.

## Plan Self-Review

| Acceptance row / ruling | Task |
| --- | --- |
| A1 (close and approval by keyboard) | 2, 3, 4, 7 |
| A2 (stages, steps, box, footers) | 3, 4 |
| A3 (resolving by keyboard) | 5, 6, 7 |
| A4 (a sheet the workspace keeps) | 5 |
| A5 (focus returns) | 4, 5 |
| A6 (names, Chosen) | 1, 6 |
| A7 (copy) | 1 |
| A8 (privacy) | 7 |
| A9 (D5's behaviour) | every task's module runs; 8 |
| A10 (on screen) | 8 |
| R-1, R-17 | 2, 3 |
| R-2, R-3, R-4 | 3 |
| R-5, R-6 | 4 |
| R-7 to R-12, R-18, R-19 | 5 |
| R-13 to R-15, R-20 | 1, 6 |
| R-16 | 1 |
