---
title: "Wave 04 Cycle 05a Implementation Plan: Common Frame, Add Repository And Unlock Key"
date: 2026-11-14
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7D7V15PAMQPTK7QZP9Q234D"
---

# Wave 04 Cycle 05a Implementation Plan: Common Frame, Add Repository And Unlock Key

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task.
>
> **Draft** (revision 1). Execution waits until Cycle 04
> (`01M7AMAPNHHTNECXBF4CA5WPKV`) is merged to main (Cycle decision 1),
> and until the owner has approved the three documents, authorized
> implementation and chosen the execution method.
>
> Where focus goes around the Add sheet is a table of required outcomes
> (design B9, G1–G14), and it is the contract. Task 3 writes its rows
> test-first under a design checkpoint. Re-read the design before each
> task, and record every checkpoint on ticket
> `01M7D5Z7FZTGTC1HE25KQ1MYQY`. Steps use checkbox (`- [ ]`) syntax.

**Goal:**

- every dialog except the two Cycle 05b surfaces on `05`'s frame, with
  its behaviour unchanged;
- Add repository as its own 470 px sheet, opened from every way in,
  replacing Settings when opened from it, with `05`'s layout, the main
  branch select and "Share changes through";
- the Unlock key dialog in `05`'s layout;
- the empty item area's icon.

**Architecture.**

- **The frame** (design Part A): `D/view/frame.rs` builds the header for
  a dialog's or sheet's title slot, and the body and footer each dialog's
  own view draws. `frame::theme::apply` sets the Kit theme's primary,
  scrim, ring and input slots once, through `apply_config`.
- **The Add sheet** (Part B): a new `AddRepositorySheet` entity holds
  what today's two forms hold in `RepositoriesPage`. Cycle 04's
  `SheetOwner` gains `AddRepository`; `Workspace::show_add_repository`
  opens it from every way in; `add_closed` brings Settings back when it
  came from there.
- **Sharing** (Part C): the Workspace keeps the chosen remote per add
  request and sends today's `RemoteSelect` when the add succeeds.
- **Unlock** (Part D): the passphrase dialog on the frame, with the key's
  fingerprint read from `list_keys`.
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md)
and [design](2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md),
with `docs/design/05-sheets-and-dialogs.md` and `06`.

- "Fact n", "A-n", "G-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer to
  the design, except "A1"–"A13" in Self-Review, which are the Cycle's
  acceptance rows. "Decision n" refers to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method.
- **Cycle 04 is closed and merged to main** (decision 1).
- Task 0 does not start before both.
- The design's Document Amendment 2 is applied by the planner in the
  approval commit. Amendment 1 is applied in Task 0, amendments 3 and 4
  in Task 9.

**Roles.**

- The orchestrator runs the tasks, or dispatches one implementer per task
  and then a fresh reviewer, one at a time.
- It alone writes the ledger and ticket comments and pushes the ticket
  branch.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files.

**Where.**

- Every task runs in `.manyhands/worktrees/01M7D5Z7FZTGTC1HE25KQ1MYQY` on
  `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`.
- No lane branches and no extra worktrees: an extra worktree once made
  `ticket create` answer `recovery_required`.
- Every Rust command is `devenv shell -- cargo …` with `--locked`.

**Boundary.** A task changes `src/desktop/` and its tests. After each
task's commit, this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` before Tasks 0 and 6, and whenever main
moves. Never rebase.

**Push.**

- After each task's commit, the orchestrator pushes the ticket branch
  with `item sync` from the main checkout (owner, 2026-11-11).
- It then checks that `HEAD` equals
  `origin/manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`.
- `item sync` and `comment add` merge main first when it has moved. Any
  such merge is recorded in the ledger, and the task's tests are run again
  by exact name.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.**

- Every user-visible string is a `pub(crate) const` (or a formatting
  `fn`), verbatim from design Part F or kept from today. Tests compare
  constants.
- Sentence case and no exclamation marks.
- `…` (U+2026) on an action that opens a chooser, confirmation or sheet
  (`06`).

**Test runs** (`AGENTS.md`, Testing Strategy):

- **Command.** Desktop tests run as
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- **Paths.** A test's path is its module's, for example
  `desktop::view::add_repository::tests::<name>` for the new
  `D/view/add_repository_tests.rs`, and
  `desktop::view::frame::tests::<name>` for `D/view/frame_tests.rs`.
- **New test files** are included as
  `#[cfg(all(test, feature = "desktop-test-support"))] #[path = "…_tests.rs"] mod tests;`.
- **Each run reports more than zero tests.** A command with an empty
  filter is stopped at once.
- **Between tasks,** only the tests the task wrote or changed run, by
  exact name, each named edit included, listed in full in the ledger.
- **Module runs** (a named doubt). Tasks 1–4 change code that tests
  reach without naming it: the dialogs' layout, the Add forms' host, and
  the routes. At the end of each, the orchestrator runs once, by module,
  every test file the task touched and every file Task 0's sweep listed
  for it, and fixes what fails before the commit.
- **No whole-suite run** happens before Task 9.
- **Limits.**
  - A command past about 15 minutes is stopped, narrowed and noted.
  - Never build while a test run is in progress.
  - A new window test runs alone first.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.**

- Each new behaviour's test is written and seen to fail before its code.
  A P test that pins behaviour this Cycle keeps (P1) is written first and
  seen to pass on today's code, then kept green through the change.
- Table-driven where the design gives a table (P1, P2, P4, P5, P9,
  P11).
- **Focus is read after at least one draw** (design B9; Cycle 04's
  design A6). A read inside the update that moved focus does not prove a
  G row.
- Guards are tested with dispatched actions, never pointer clicks, which
  the scrim takes, and never keys bound in another context (Cycle 02
  ledger).
- Window tests with polling on answer every poll (Cycle 03a handoff).
- No real waits: the 0.25 s deferred restore is reached by advancing the
  test clock.

**No appearance tests** (Wave Testing). Nothing asserts widths, colours,
icons, dividers, the header's layout, the theme slots or that a view
renders.

**Existing tests.**

- They change only as E-1 to E-12 say.
- Any other edit is pre-authorized (decision 8). It is recorded in the
  ledger as E-13 onward with its reason, and checked by the task's
  reviewer.
- An assertion removal is called out in the ledger and the review-ready
  summary.
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`, and Task N removes it.
Test-only helpers and accessors are `#[cfg(test)]`. Allows go on items,
never on a module.

**Commits.** One per task, with a conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.**

- After Tasks 0, 3, 6 and 9, a comment through
  `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
  run against the main checkout while the ticket worktree is clean.
- It records the commits, the tests run with counts (copied from the
  ledger), and the rulings.
- Before `ticket save`, commit any owned `ticket.md` edit. `comment add`
  publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md`.

- It is created in Task 0 and written only by the orchestrator.
- Only the ledger assigns E-numbers past E-12.
- Rulings take the form "Ruling: decision — why — cost if wrong".

**File size.** New source files stay below about 500 lines.
`D/view/settings/repositories.rs` (802 lines) shrinks by the forms;
`D/view/workspace.rs` changes only its fields and module declarations,
and the Workspace's new code goes in `D/view/workspace/add_sheet.rs`.

**The real registry.** `manyhands-cli` runs only from the up-to-date main
checkout, never from a build of this branch.

## Review Focus

Inputs no task's main journey exercises, each pinned by a test in the
task named:

- **Task 2:** a keyed Create, New folder and Move that close after their
  write with the footer drawn in the body (P1, R-19).
- **Task 3:**
  - Add repository opened from Settings with a passphrase typed on
    Sharing & keys (P6: emptied);
  - Add pressed, then Escape before the folder check lands (P8);
  - an `Inspect` route while the sheet is already open (P4: the route
    applies, the origin kept);
  - the confirmation's Escape returning inside the sheet within 0.25 s
    of an earlier deferred close (P5, G10);
  - quitting with the confirmation open above the sheet (P7, G13).
- **Task 4:** a `Confirm` naming a refused branch (P10); a check landing
  for text the person has since changed (P9: ignored).
- **Task 6:** the sheet closed while the add runs, then the add succeeds
  (P14: still sent); the add fails (P14: nothing sent).
- **Task 7:** a `list_keys` read that answers Busy while the prompt waits
  (P18: label only).

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/view/frame.rs` (new), `D/view/frame/theme.rs` (new), `D/view/frame_tests.rs` (new) | 1, 2 |
| `D/mod.rs` (`run`: `frame::theme::apply`) | 1 |
| `D/view/shell/tokens.rs` (`accent_hover`, `on_accent`) | 1 |
| `D/assets.rs` (icons `FolderInput`, `KeyRound`, `GitBranch`, `Cloud`, `X`) | 1 |
| `D/view/confirm.rs` | 1 |
| `D/view/create.rs`, `create/form.rs`, `new_folder.rs`, `move_document.rs`, `close_draft.rs`, `quit.rs`, `retire_asks.rs`, `drafts/actions.rs`, `item/repair/prompt.rs`, `item/composer/discard.rs`, `item/edit/recovery_off.rs`, `item/edit/review.rs`, `item/sheet.rs` | 2 |
| `D/view/add_repository.rs` (new), `add_repository/checks.rs`, `render.rs`, `share.rs` (new), `D/view/add_repository_tests.rs` (new) | 3, 4, 5, 6 |
| `D/view/workspace/add_sheet.rs` (new), `D/view/workspace/add_sheet_tests.rs` (new) | 3, 6 |
| `D/view/workspace/content.rs` (`SheetOwner`, `sync_sheet`), `workspace.rs`, `workspace/lifecycle.rs` (quit paths) | 3 |
| `D/view/workspace/settings_sheet.rs`, `D/view/settings/arrive.rs`, `D/view/settings/mod.rs` (Cycle 04's) | 3 |
| `D/view/settings/repositories.rs`, `repositories_tests.rs` | 3, 4 |
| `D/view/repositories/body.rs`, `D/view/dock/header/toolbar.rs`, `D/view/workspace/requests.rs`, `D/view/lifecycle/actions.rs` | 3 |
| `D/model/stamp.rs` (`ViewScope::AddRepository`) | 3 |
| `D/view/passphrase.rs`, `D/view/workspace/lifecycle.rs` (`show_prompt`) | 7 |
| `D/view/workspace/content.rs` (`empty_pane`) | 8 |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Precondition.** Confirm Cycle 04 is closed and merged:
  `git log origin/main` shows "Integrate closed ticket
  01M7AMAPNHHTNECXBF4CA5WPKV". If not, stop.
- [ ] **Step 2: Merge and baseline.**

```sh
git fetch origin && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 3: Re-map Cycle 04's names (U-1).** Run
  `git diff --stat ea6d899b HEAD -- src/desktop docs/Waves docs/design Cargo.lock`.
  - Read the merged `SheetOwner`, `sync_sheet`, `show_settings`,
    `settings_closed`, `SettingsRoute`, `SettingsHost::arrive`,
    `settings_origin`, `settings_return` and the host's `open` flag, and
    Cycle 04's execution ledger (its rulings and W-36).
  - Re-read design facts 9, 16, 18–24 against them. Correct the design
    first, in a commit of its own noted in the ledger.
  - If Cycle 04's code cannot carry B2's or B3's steps (an origin for
    A-3, a quiet close that empties the passphrase, a `Manage(root)`
    route), stop for the owner.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner. Facts 32–41 rest on the locked versions.
- [ ] **Step 4: Wave amendment.** Apply the design's Document Amendment 1
  to `docs/Waves/wave-04-workspace-design.md`, over Cycle 04's own
  amendment of the Cycle 05 section, in a commit of its own.
- [ ] **Step 5: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'inspect_in_settings' -e 'SettingsRoute::Inspect' -e 'SettingsRoute::AddRepository' \
  -e 'inspect_root' -e '"add-' -e '"create-' -e 'add_form' -e 'create_form' \
  -e 'Inspection::' -e 'submit_pending' -e 'resume_submit' -e 'NEED_FOLDER' -e 'NEED_BRANCH' \
  -e 'browse-unavailable' -e 'ADD_HEADING' -e 'CREATE_HEADING' -e 'CHECKED_OUT' -e 'NOT_ADDED' -- $P
git grep -n -e 'passphrase-' -e 'label_for' -e 'NOT_NOW_NOTE' -e 'TITLE' -- $P/view/passphrase.rs $P/view/passphrase_tests.rs $P/view/keyboard_tests
git grep -n -e 'confirm-ok' -e 'confirm-cancel' -e 'tab_reaches' -e '"tab"' -- $P/view/*_tests.rs $P/view/keyboard_tests
```

  - Record each test site against its E-n, or as E-13 onward (decision 8).
  - Confirm design A3's footer table against each dialog's code (U-3).
  - List every caller of the `settings()` helper of
    `D/view/settings/repositories_tests.rs` and of Cycle 04's helpers that
    open Settings on Repositories, and every test file each task will
    touch, for the module runs.
  - A production site no task names goes into the task that breaks it.
- [ ] **Step 6: Ledger, commit, push, ticket comment.** Record the base
  SHA, Cycle 04's merge commit, Step 3's findings and Step 5's lists.

---

## Task 1: The Frame And Theme (A1, A2, A6; the Confirmation dialog)

**Interfaces.**

```rust
// D/view/frame.rs
pub(crate) enum Tone { Accent, Danger, Success }
pub(crate) struct Header { /* design A1 */ }
pub(crate) struct Footer { /* design A1 */ }
pub(crate) fn dialog(base: Dialog, header: Header, cx: &App) -> Dialog;
pub(crate) fn body(content: impl IntoElement) -> Div;
pub(crate) fn footer(footer: Footer, cx: &App) -> Div;
pub(crate) fn sheet_header(header: Header, cx: &App) -> impl IntoElement;
pub(crate) fn sheet_footer(footer: Footer, cx: &App) -> impl IntoElement;
// D/view/frame/theme.rs
pub(crate) fn apply(cx: &mut App);
```

- [ ] **Step 1: P1 and P2 rows for the Confirmation dialog, first.** In
  the new `D/view/frame_tests.rs`, write `every_dialog_keeps_its_keys_and_focus`
  with its Confirmation row (focus on open, Enter confirms once, Escape
  closes with nothing sent, focus back on the origin) and
  `footer_buttons_tab_in_drawn_order` with its Confirmation row (Tab from
  the body reaches `confirm-cancel` before `confirm-ok`). Copy the window
  set-up from `D/view/confirm_tests.rs`. P1's row passes on today's code;
  P2's fails.
- [ ] **Step 2: Theme.** Add `accent_hover` and `on_accent` to
  `ChromeColors` (`07`'s values, both modes). Write `frame::theme::apply`
  (design A2) with `Theme::apply_config` for both modes, then
  `Theme::sync_base`, and call it in `D/mod.rs`'s `run` right after
  `gpui_kit::init`.
- [ ] **Step 3: Icons.** Add `FolderInput`, `KeyRound`, `GitBranch`,
  `Cloud` and `X` to `icon_assets!(ItemIcons, …)` (`D/assets.rs`). Run
  `desktop::assets::tests::every_item_view_icon_loads` (`D/assets_tests.rs:66`)
  by exact name (U-6); check the module path in Task 0.
- [ ] **Step 4: The frame.** Write `D/view/frame.rs` (design A1). Header
  title and description keep the ids the caller passes.
- [ ] **Step 5: The Confirmation dialog.** Its header in the Kit's title
  slot through `frame::dialog`; its body and footer drawn by
  `ConfirmDialog::render` through `frame::body` and `frame::footer`,
  Cancel or Close first, then the action (design A3). `confirm-command`
  stays in the body. Run P1, P2 and every test of
  `desktop::view::confirm::tests` and `desktop::view::keyboard_tests::confirm`
  by module (module run).
- [ ] **Step 6: Verify, commit, push.**

---

## Task 2: Every Other Dialog On The Frame (A3, A5)

- [ ] **Step 1: P1 and P2 rows, first,** for every other dialog in
  design A3's table: New ticket, New document, New folder, Move, Close a
  tab with a draft, the quit asks, the retire asks, Discard a draft, the
  repair discard, the composer discard, Recovery off and Replace your
  edits. Each P1 row includes, for Create, New folder and Move, that a
  keyed action closes the dialog after its write (R-19). Copy each
  dialog's set-up from its own test file. All P1 rows pass on today's
  code; P2's rows pass too, since the order is kept (they guard R-19).
- [ ] **Step 2: Move each dialog onto the frame,** one file at a time:
  the header into the Kit's title slot, the body into `frame::body`, the
  footer into `frame::footer` as the last child of the dialog's own view,
  in today's order. Replace your edits moves `review-replace-message`
  into its header with its id. Bodies that carry their title as an aria
  label keep it.
- [ ] **Step 3: The details sheet's header** (A5):
  `frame::sheet_header` with `sheet-heading` and `sheet-description`;
  the body loses `sheet-description`; the footer is unchanged. Run
  `desktop::view::item::sheet_tests` and
  `desktop::view::keyboard_tests::frame::the_details_sheet_opens_by_enter_traps_focus_and_gives_it_back`.
- [ ] **Step 4: Module runs** over every touched dialog's test file and
  Task 0's list for this task, including `keyboard_tests::drafts`,
  `keyboard_tests::editing`, `editing_privacy_tests`,
  `lifecycle::privacy_tests` and `polling_privacy_tests`.
- [ ] **Step 5: Verify, commit, push.**

---

## Task 3: The Add Sheet: Owner, Ways In, Settings And Back, Focus (B1–B4, B9, B11, B12; design checkpoint)

**Interfaces.**

```rust
// D/view/workspace/content.rs (Cycle 04's enum)
pub(super) enum SheetOwner { Details(WeakEntity<ItemPane>), Settings, AddRepository }
// D/view/workspace/add_sheet.rs
pub(crate) enum AddRoute { New, Inspect(PathBuf) }
impl Workspace {
    pub(in crate::desktop::view) fn show_add_repository(&mut self, route: AddRoute,
        origin: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>);
    pub(super) fn add_closed(&mut self, quiet: bool, window: &mut Window, cx: &mut Context<Self>);
    #[cfg(test)] pub(crate) fn add_sheet(&self) -> Option<Entity<AddRepositorySheet>>;
}
// D/view/add_repository.rs
pub(crate) struct AddRepositorySheet { /* design B4 */ }
impl AddRepositorySheet {
    pub(crate) fn arrive(&mut self, route: &AddRoute, window: &mut Window, cx: &mut Context<Self>);
    pub(crate) fn reset(&mut self, cx: &mut Context<Self>);
}
// D/model/stamp.rs
ViewScope::AddRepository
```

- [ ] **Step 1: B9's rows, first.** In `D/view/workspace/add_sheet_tests.rs`,
  write `focus_returns_to_what_opened_add` (P5),
  `add_from_settings_returns_to_settings` (P6),
  `the_add_sheet_closes_every_way` (P7),
  `late_work_moves_no_focus_after_add_closes` (P8),
  `add_repository_is_a_sheet_the_workspace_keeps` (P3),
  `each_way_in_opens_the_add_sheet` (P4) and
  `nothing_acts_behind_the_add_sheet` (P15), table-driven over B9's G
  rows. Each fails.
- [ ] **Step 2: Design checkpoint.** If a G row cannot be expressed, or
  Cycle 04's merged mechanism answers it differently, stop and report to
  the orchestrator, who asks the owner. Never edit a row to fit.
- [ ] **Step 3: Move the forms' state.** Create `AddRepositorySheet` with
  everything fact 9 lists, unchanged in behaviour, its reads under
  `ViewScope::AddRepository`, its own `ScrollHandle`, and the `open`
  flag. Today's two forms render stacked inside it for now (Tasks 4 and 5
  restyle them). `RepositoriesPage` keeps the manage section and the
  shared helpers, and draws `settings-add-repository` where the forms
  were (B11).
- [ ] **Step 4: The owner and the sheet.** Add `SheetOwner::AddRepository`
  and `sync_sheet`'s cases (B1); `show_add_repository` and `add_closed`
  (B2, B3) in `add_sheet.rs`; the quit paths close it quietly.
- [ ] **Step 5: Ways in.** A-1 (`repositories/body.rs`), A-2
  (`press_toolbar`), A-3 (`settings-add-repository`), A-4
  (`lifecycle/actions.rs`), A-5 (`workspace/requests.rs`). Remove
  `SettingsRoute::Inspect` and `inspect_in_settings` (R-5); repoint
  `SettingsRoute::AddRepository` (R-6).
- [ ] **Step 6: Named edits** E-1 (move only: the Add and Create tests
  move to `D/view/add_repository_tests.rs` and open the sheet, their
  assertions otherwise as today), E-3, E-4, E-5, E-6, E-7, E-8, E-12.
- [ ] **Step 7: Run** P3–P8 and P15, every moved test, and the module
  runs: `desktop::view::settings::repositories`,
  `desktop::view::settings::repositories::manage_tests`,
  `desktop::view::repositories`, `desktop::view::dock::tests::toolbars`,
  `desktop::view::lifecycle::outcome_canned_tests`,
  `desktop::view::keyboard_tests::settings`, Cycle 04's
  `desktop::view::workspace::settings_sheet` tests, and Task 0's list.
- [ ] **Step 8: Verify, commit, push, ticket comment.**

---

## Task 4: Add An Existing Folder (B5, B6, B7, B10; Part G)

- [ ] **Step 1: Tests first:** `folder_checks_are_rows_with_their_actions`
  (P9, a table over B6's rows, canned checks where a real repository
  cannot give the case), `the_main_branch_is_the_checked_out_one` (P10),
  `add_is_enabled_only_when_it_can_go_ahead` (P11),
  `browse_says_why_beside_its_button` (P21) and
  `a_folder_that_is_not_text_is_named` (P22, `#[cfg(unix)]`).
- [ ] **Step 2: Header and mode** (B5): `frame::sheet_header`; the
  `ToggleGroup::segmented()` kept to one checked toggle. Test first that
  its toggles take Enter and Space inside the sheet (U-5).
- [ ] **Step 3: The folder and its checks** (B6), in
  `add_repository/checks.rs`: the rows and their actions; Browse… and its
  note line; Retry, Check again, Choose another and Create one here.
- [ ] **Step 4: The main branch** (B7): a `SelectState` whose delegate
  draws the Suggested tag and the greyed reason, refuses other branches
  in `on_will_change`, and ignores a `Confirm` naming one.
- [ ] **Step 5: The footer and enablement** (B10): `frame::sheet_footer`
  with Cancel and Add repository; R-T12b's check-first kept.
- [ ] **Step 6: Named edits** E-1 (the row assertions) and E-2.
- [ ] **Step 7: Run** P9–P11, P21, P22, the moved Add tests, and the
  module run of `desktop::view::add_repository`.
- [ ] **Step 8: Verify, commit, push.**

---

## Task 5: Create A New Repository (B8)

- [ ] **Step 1: Test first:** `create_joins_location_and_name` (P12).
- [ ] **Step 2:** Name, Location and its Browse…, `create-target`, the
  branch text field, the prefix, the optional identity and the note (B8).
  Create one here (B6) fills Location and Name.
- [ ] **Step 3: Named edit** E-1 (the Create tests fill Name and
  Location).
- [ ] **Step 4: Run** P12, P9's Create one here row, and the moved Create
  tests.
- [ ] **Step 5: Verify, commit, push.**

---

## Task 6: Share Changes Through (Part C)

- [ ] **Step 1: Merge `origin/main`** (Global Constraints, Main).
- [ ] **Step 2: Tests first:** `share_changes_through_lists_the_folders_remotes`
  (P13) and `a_chosen_remote_is_selected_after_the_add_succeeds` (P14).
  P14 reads what the worker received, as
  `D/view/settings/remotes_tests.rs` does for `RemoteSelect`.
- [ ] **Step 3: The select** (C1), in `add_repository/share.rs`.
- [ ] **Step 4: After the add** (C2): `pending_shares` in
  `add_sheet.rs`, filled on `Followed::Confirmed`, sent on the request's
  success, dropped otherwise.
- [ ] **Step 5: Run** P13, P14 and P9's registered-folder row.
- [ ] **Step 6: Verify, commit, push, ticket comment.**

---

## Task 7: The Unlock Key Dialog (Part D)

- [ ] **Step 1: Tests first:** `the_unlock_dialog_names_the_key` (P18,
  including a Busy and a failed `list_keys` read, U-4) and
  `a_refused_passphrase_keeps_focus_in_the_field` (P19, by Enter and by
  the Unlock button).
- [ ] **Step 2:** The dialog on the frame (D1–D7): header with the lock
  tile; the key card and its read through the `Services` `show_prompt`
  passes; `PASSPHRASE`; the refusal's focus; the note always; Not now ·
  Unlock; width 440 px.
- [ ] **Step 3: Named edit** E-9. Confirm E-10 (no change expected).
- [ ] **Step 4: Run** P18, P19, P1's and P2's Unlock rows, every test of
  `desktop::view::passphrase::tests` and
  `desktop::view::keyboard_tests::passphrase`, and the passphrase sweep of
  `desktop::view::polling_privacy_tests`.
- [ ] **Step 5: Verify, commit, push.**

---

## Task 8: Empty Pane, Copy, Privacy, Keyboard Journeys (Parts E, F; P16, P17, P20)

- [ ] **Step 1: Tests first:** `add_and_create_by_keyboard` (P16),
  `add_sheet_names_no_secret` (P17) and `frame_copy_uses_the_vocabulary`
  (P20).
- [ ] **Step 2:** The empty pane's icon (Part E). No test (Wave Testing).
- [ ] **Step 3:** Part F's constants, where an earlier task did not
  already write them.
- [ ] **Step 4: Run** P16, P17, P20 and
  `desktop::view::workspace_tests` at the empty pane's test
  (`workspace_tests.rs:611`).
- [ ] **Step 5: Verify, commit, push.**

---

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-14-wave-04-cycle-05a-on-screen-checklist.md` for
  the owner: frames 1d and 2h, the frame on each A3 dialog, the theme
  slots in the mode the app shows (U-2), the empty pane. Record only what
  the owner reports.
- [ ] **Step 2: Gates** (`AGENTS.md`): the static gates across the tree,
  then the fast tier once with `--no-fail-fast`. No SSH target: this
  Cycle changes `src/desktop/` only.
- [ ] **Step 3:** `git merge-tree --write-tree HEAD origin/main`; merge
  main again if it reports conflicts.
- [ ] **Step 4: Document Amendments 3 and 4:** Cycle 04's Cycle document's
  Downstream Obligations marked done; a comment on
  `01M5F3GVDYTH1Q5V5FCS9YF7V3` that its four Add repository items are
  done (that comment publishes that ticket's branch).
- [ ] **Step 5:** Fill the Cycle document's Status column from the
  ledger; mark the ticket review-ready with a comment.
- [ ] **Step 6: After review approval only:** set the ticket's status to
  `done` with `ticket save`, then the owner runs `ticket close` from the
  main checkout, which merges.

## Plan Self-Review

**Acceptance rows to tasks.**

| Cycle row | Tasks | Tests |
| --- | --- | --- |
| A1 | 1, 2 | P1, P2 |
| A2 | 3 | P3, P15 |
| A3 | 3 | P4 |
| A4 | 3 | P6 |
| A5 | 3 | P5–P8 |
| A6 | 3, 4, 5, 8 | P9, P16; moved tests |
| A7 | 4, 5, 6 | P9–P13 |
| A8 | 6 | P14 |
| A9 | 7 | P18, P19 |
| A10 | 8 | P20 |
| A11 | 7, 8 | P17; existing privacy tests |
| A12 | 4 | P9, P21, P22 |
| A13 | 9 | On screen |

**Rulings to tasks.** R-1, R-4, R-19: Tasks 1–2. R-2, R-3: Task 1. R-5 to
R-9, R-14: Task 3. R-10 to R-12, R-18: Task 4. R-13: Task 5. R-15:
Task 6. R-16, R-17: Task 7.

**Placeholders.** None. The interfaces marked "design A1" and "design
B4" name the design's sketch, which the task settles.

**Unverified names.** U-1 (Cycle 04's merged names, Task 0), U-3 (A3's
table, Task 0), U-4 (Task 7), U-5 (Task 4), U-6 (Task 1). Each step that
uses one names the existing code or test to copy from.
