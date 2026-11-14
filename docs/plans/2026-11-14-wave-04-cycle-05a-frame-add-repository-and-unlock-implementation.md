---
title: "Wave 04 Cycle 05a Implementation Plan: Common Frame, Add Repository And Unlock Key"
date: 2026-11-14
status: "approved"
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
> **Approved 2026-11-14** (revision 3: review rounds 1 and 2 and
> decisions 15–18 applied; **revision 3 had no independent review**).
> Execution is subagent-driven and serial, in a fresh session from the
> handoff brief `docs/plans/2026-11-14-wave-04-cycle-05a-handoff.md`
> (Cycle decision 21). It waits until Cycle 04
> (`01M7AMAPNHHTNECXBF4CA5WPKV`) is merged to main (Cycle decision 1).
>
> Where focus goes around the Add sheet is a table of required outcomes
> (design B9, G1–G23), and it is the contract. Task 4 writes its rows
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
  came from there. The Workspace keeps each confirmed add
  (`pending_adds`) and closes the sheet once it succeeds.
- **Sharing** (Part C): the same entry holds the share choice, sent as
  today's `RemoteSelect` when the add succeeds.
- **Unlock** (Part D): the passphrase dialog on the frame, with the key's
  fingerprint read from `list_keys`.
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md)
and [design](2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md),
with `docs/design/05-sheets-and-dialogs.md` and `06`.

- "Fact n", "G-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer to the
  design; "A1"–"A6" are its Part A sections, except "A1"–"A13" in
  Self-Review, which are the Cycle's acceptance rows. "AW-n" are the
  design's ways into the Add sheet.
  "Decision n" refers to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method (2026-11-14, decision 21).
- **Cycle 04 is closed and merged to main** (decision 1).
- Task 0 does not start before both.
- The design's Document Amendments 2 and 3 were applied by the planner
  in the approval commit. Amendment 1 is applied in Task 0, amendments 4
  and 5 in Task 10.

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

**Main.** Merge `origin/main` before Tasks 0 and 7, and whenever main
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
- **Module runs** (a named doubt). Tasks 1–5 change code that tests
  reach without naming it: the dialogs' layout, the Add forms' host, and
  the routes. At the end of each, the orchestrator runs once, by module
  (the command without `-- --exact`), every test file the task touched
  and every file Task 0's sweep listed for it, and fixes what fails
  before the commit.
- **No whole-suite run** happens before Task 10.
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
  P11, P16).
- A row a later task builds is written in that task, not left red.
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

- They change only as E-1 to E-16 say.
- Any other edit is pre-authorized (decision 8). It is recorded in the
  ledger as E-17 onward with its reason, and checked by the task's
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

- After Tasks 0, 4, 7 and 10, a comment through
  `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
  run against the main checkout while the ticket worktree is clean.
- It records the commits, the tests run with counts (copied from the
  ledger), and the rulings.
- Before `ticket save`, commit any owned `ticket.md` edit. `comment add`
  publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md`.

- It is created in Task 0 and written only by the orchestrator.
- Only the ledger assigns E-numbers past E-16.
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
- **Task 4:**
  - Add repository opened by a direct call while Sharing & keys holds a
    typed passphrase (P6: emptied);
  - Add pressed, then Escape before the folder check lands (P8);
  - an `Inspect` route while the sheet is already open (P4: the route
    applies, the origin kept);
  - AW-5 raised from an Activity row, its menu and a popover row whose
    footer item has hidden by close time (P4, P5);
  - a recovery in the confirmation that opens Settings (P6, G17);
  - an add's success whose finish closes the confirmation in the same
    batch (P5, P6, G14), and one that lands while an Unlock prompt is
    open (P5, G23);
  - a success for an earlier open of the sheet after it was reopened
    (P8);
  - quitting with the confirmation open above the sheet (P7, G15);
  - the Kit's deferred restore landing behind the sheet (P15, G18).
- **Task 5:** a `Confirm` naming a refused branch (P10); a check landing
  for text the person has since changed (P9: ignored); a repository with
  no commits (P10).
- **Task 7:** the sheet closed while the add runs, then the add succeeds
  (P14: still sent); Prepare again, then success, and a Partial then
  Retry (P14: one selection each); the add ends (P14: nothing sent); a
  Valid configuration with no identity (P14: the select held on Keep).
- **Task 8:** a `list_keys` read held, then failing (P18: label only).

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/view/mod.rs` (`mod frame;`, `mod add_repository;`) | 1, 3 |
| `D/view/frame.rs` (new), `D/view/frame/theme.rs` (new), `D/view/frame_tests.rs` (new) | 1, 2, 8 |
| `D/mod.rs` (`run`: `frame::theme::apply`) | 1 |
| `D/view/shell/tokens.rs` (`accent_hover`, `on_accent`) | 1 |
| `D/assets.rs`, `D/assets_tests.rs` (five icons, E-14) | 1 |
| `D/view/confirm.rs` | 1 |
| `D/view/create.rs`, `create/form.rs`, `new_folder.rs`, `move_document.rs`, `close_draft.rs`, `quit.rs`, `retire_asks.rs`, `drafts/actions.rs`, `item/repair/prompt.rs`, `item/composer/discard.rs`, `item/edit/recovery_off.rs`, `item/edit/review.rs`, `item/sheet.rs`, `settings/frame.rs` (Cycle 04's) | 2 |
| `D/view/add_repository.rs` (new), `add_repository/checks.rs`, `render.rs`, `share.rs` (new), `D/view/add_repository_tests.rs` (new) | 3, 5, 6, 7, 9 |
| `D/view/settings/repositories.rs`, `repositories_tests.rs` | 3, 4 |
| `D/model/stamp.rs` (`ViewScope::AddRepository`, `ViewScope::Unlock`) | 3, 8 |
| `D/view/workspace/add_sheet.rs` (new), `D/view/workspace/add_sheet_tests.rs` (new) | 4, 7 |
| `D/view/workspace/content.rs` (`SheetOwner`, `sync_sheet`), `workspace.rs`, `workspace/lifecycle.rs` (quit paths), `workspace/settings_sheet.rs` and Cycle 04's Ctrl+, handler, `D/view/settings/arrive.rs` | 4 |
| `D/view/repositories/body.rs`, `D/view/dock/header/toolbar.rs`, `D/view/workspace/requests.rs`, `D/view/lifecycle/actions.rs` | 4 |
| `D/view/passphrase.rs`, `D/view/workspace/lifecycle.rs` (`show_prompt`), `D/view/keyboard_tests/lifecycle.rs` (E-13) | 8 |
| `D/view/workspace/content.rs` (`empty_pane`) | 9 |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Fetch, then the precondition.** `git fetch origin`; then
  confirm Cycle 04 is closed and merged: `git log origin/main` shows
  "Integrate closed ticket 01M7AMAPNHHTNECXBF4CA5WPKV". If not, stop.
- [ ] **Step 2: Merge and baseline.**

```sh
git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 3: Re-map Cycle 04's names (U-1).** Run
  `git diff --stat ea6d899b HEAD -- src/desktop docs/Waves docs/design Cargo.lock`.
  - Read the merged `SheetOwner`, `SheetOwnerKind`, `sync_sheet`,
    `show_settings`, `settings_closed`, `SettingsRoute`,
    `SettingsHost::arrive`, `settings_origin`, `settings_return`, the
    host's `open` flag, the Ctrl+, handler, `settings/frame.rs`, and
    Cycle 04's execution ledger (its rulings and W-36).
  - Re-read design facts 9, 16, 18–24 and 23a against them. Correct the
    design first, in a commit of its own noted in the ledger.
  - If Cycle 04's code cannot carry B2's or B3's steps (an origin for
    AW-3, a quiet close that empties the passphrase, a `Manage(root)`
    route, a refusal in its Ctrl+, handler), stop for the owner.
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
git grep -n -e 'passphrase-' -e 'label_for' -e 'NOT_NOW_NOTE' -e 'open_dialog(prompt' -e 'Followed::Confirmed' -- $P
git grep -n -e 'confirm-ok' -e 'confirm-cancel' -e 'tab_reaches' -e 'tab_to' -- $P/view/*_tests.rs $P/view/keyboard_tests
```

  - Record each test site against its E-n, or as E-17 onward (decision 8).
  - Confirm design A3's footer table against each dialog's code (U-3).
  - Confirm the module paths of the Unlock evidence tests (design
    Testing; U-4) and of `every_item_view_icon_loads`, and look in
    `D/test_support/` for a read double P18 can fail (U-8).
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
- [ ] **Step 2: Theme.** Read `ThemeConfig`'s schema for the secondary
  button, input background and sheet border keys and `Theme::sync_base`
  (U-7); record what exists in the ledger. Add `accent_hover` and
  `on_accent` to `ChromeColors` (`07`'s values, both modes). Write
  `frame::theme::apply` (design A2): both configs through
  `apply_config`, the current mode's last, hex colours only, then
  `sync_base`; call it in `D/mod.rs`'s `run` right after
  `gpui_kit::init`. Check by a unit test that the theme's mode is
  unchanged by `apply` (`Theme::global(cx).mode` before and after).
- [ ] **Step 3: Icons.** Add `FolderInput`, `KeyRound`, `GitBranch`,
  `Cloud` and `X` to `icon_assets!(ItemIcons, …)` (`D/assets.rs`), and to
  the `ICONS` list of `desktop::assets::tests::every_item_view_icon_loads`
  (E-14). Run it by exact name.
- [ ] **Step 4: The frame.** Write `D/view/frame.rs` (design A1). Header
  title and description keep the ids the caller passes.
- [ ] **Step 5: The Confirmation dialog.** Its header in the Kit's title
  slot through `frame::dialog`; its body and footer drawn by
  `ConfirmDialog::render` through `frame::body` and `frame::footer`,
  Cancel or Close first, then the action (design A3). `confirm-command`
  stays in the body.
- [ ] **Step 6: Run** P1, P2, E-14, and by module
  `desktop::view::confirm::tests` and `desktop::view::keyboard_tests::confirm`
  (E-10).
- [ ] **Step 7: Verify, commit, push.**

---

## Task 2: Every Other Dialog On The Frame (A3, A5)

- [ ] **Step 1: P1 rows, first,** for New ticket, New document, New
  folder, Move, Close a tab with a draft, the quit asks, the retire asks,
  Discard a draft, the repair discard, the composer discard, Recovery off
  and Replace your edits. For Create, New folder and Move each row
  includes that a keyed action closes the dialog after its write (R-19).
  Copy each dialog's set-up from its own test file. They pass on today's
  code and must stay green.
- [ ] **Step 2: Move each dialog onto the frame,** one file at a time:
  the header into the Kit's title slot, the body into `frame::body`, the
  footer into `frame::footer` as the last child of the dialog's own view,
  in today's order. Replace your edits moves `review-replace-message`
  into its header with its id. Bodies that carry their title as an aria
  label keep it. The hand-drawn rings move with their buttons.
- [ ] **Step 3: Sheet headers** (A5): the details sheet's
  `frame::sheet_header` with `sheet-heading` and `sheet-description`,
  the body losing `sheet-description`; Cycle 04's Settings header onto
  `frame::sheet_header` (R-20).
- [ ] **Step 4: Run** P1, and by module every touched dialog's test file
  and Task 0's list for this task, including
  `desktop::view::item::sheet_tests`,
  `desktop::view::keyboard_tests::frame`,
  `desktop::view::keyboard_tests::drafts`,
  `desktop::view::keyboard_tests::editing`,
  `desktop::view::editing_privacy_tests`,
  `desktop::view::lifecycle::privacy_tests`,
  `desktop::view::polling_privacy_tests` and Cycle 04's Settings sheet
  tests (E-11).
- [ ] **Step 5: Verify, commit, push.**

---

## Task 3: Extract The Add Forms' State (B4)

No visible change: the forms move into their own entity, still drawn on
Settings › Repositories.

**Interfaces.**

```rust
// D/view/add_repository.rs
pub(crate) struct AddRepositorySheet { /* design B4 */ }
// D/model/stamp.rs
ViewScope::AddRepository
```

- [ ] **Step 1: Move the state.** Create `AddRepositorySheet` with
  everything fact 9 lists, unchanged in behaviour, its reads stamped
  under `ViewScope::AddRepository`, its own `ScrollHandle`. The
  `RepositoriesPage` holds an `Entity<AddRepositorySheet>` and draws it
  where the forms were. The manage section and the shared helpers stay.
- [ ] **Step 2: Named edit** E-1 (Task 3's part: the tests move to
  `D/view/add_repository_tests.rs`, opening Settings as today through a
  helper there; the helpers the manage tests import stay in
  `repositories_tests.rs`). The ledger lists the test functions before
  and after the move.
- [ ] **Step 3: Run** by module `desktop::view::add_repository::tests`,
  `desktop::view::settings::repositories::tests`,
  `desktop::view::settings::repositories::manage_tests` and
  `desktop::view::keyboard_tests::settings`.
- [ ] **Step 4: Verify, commit, push.**

---

## Task 4: The Add Sheet: Owner, Ways In, Settings And Back, Adding, Focus (B1–B3, B9, B10's "Adding", B11, B12; design checkpoint)

**Interfaces.**

```rust
// D/view/workspace/content.rs (Cycle 04's enum)
pub(super) enum SheetOwner { Details(WeakEntity<ItemPane>), Settings, AddRepository }
// D/view/workspace/add_sheet.rs
pub(crate) enum AddRoute { New, Inspect(PathBuf) }
pub(in crate::desktop::view) enum AddClose { Kept, Added(PathBuf), ToSettings(PathBuf), Quiet }
struct PendingAdd { attempt: u32, session: u64, share: Option<Option<String>> }
impl Workspace {
    pub(in crate::desktop::view) fn show_add_repository(&mut self, route: AddRoute,
        origin: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>);
    pub(in crate::desktop::view) fn add_closed(&mut self, how: AddClose, window: &mut Window, cx: &mut Context<Self>);
    #[cfg(test)] pub(crate) fn add_sheet(&self) -> Option<Entity<AddRepositorySheet>>;
}
// D/view/settings/arrive.rs (Cycle 04's enum)
SettingsRoute::AddButton // renamed from AddRepository; Inspect removed
// D/view/confirm.rs
Followed::Confirmed { request, attempt, details } // attempt added (R-28)
// D/services/requests.rs
Request::registered_root: Option<PathBuf> // from the Finished envelope's scope (R-29)
```

- [ ] **Step 1: B9's rows, first.** In `D/view/workspace/add_sheet_tests.rs`,
  write P3, P4, P5, P6, P7, P8, P15 and P23, table-driven over B9's G
  rows except G16 and G22 (Task 5) and G20 (Task 6). Each fails.
- [ ] **Step 2: Design checkpoint.** If a G row cannot be expressed, or
  Cycle 04's merged mechanism answers it differently, stop and report to
  the orchestrator, who asks the owner. Never edit a row to fit.
- [ ] **Step 3: The owner and the sheet.** `SheetOwner::AddRepository`,
  `SheetOwnerKind::AddRepository` and `sync_sheet`'s three cases (B1);
  `show_add_repository` and `add_closed` (B2, B3) in `add_sheet.rs`; the
  sheet drawn in a Kit `Sheet` of 470 px with `frame::sheet_header` and
  `frame::sheet_footer` (Cancel, and today's Add and Create buttons
  moved there); the quit paths close it quietly; `show_settings` closes
  it quietly first (R-22); the Ctrl+, handler refuses while it is open
  (R-14); Cycle 04's R-34 and R-46 extended to it (G18, G19; R-24).
- [ ] **Step 4: Ways in.** AW-1 (`repositories/body.rs`), AW-2
  (`press_toolbar`), AW-3 (`settings-add-repository`, replacing the
  forms on Settings › Repositories, B11), AW-4 (`lifecycle/actions.rs`),
  AW-5 (`workspace/requests.rs`). Remove `SettingsRoute::Inspect` and
  `inspect_in_settings` (R-5); rename `SettingsRoute::AddRepository` to
  `AddButton` (R-6).
- [ ] **Step 5: Adding** (B10's "Adding"): `Followed::Confirmed` gains
  `attempt` (R-28; E-16); `Request` gains `registered_root` (R-29);
  `confirm_followed` with the origin closure, the sheet holding only the
  subscription; `pending_adds` recorded on `Followed::Confirmed` with the
  event's attempt and the open number (R-26); B10's states in
  `operations_changed`, the entry taken out in that pass and handled
  with `defer_in` (R-25); added closes the sheet of the same open with
  `add_closed(Added(root))`, or forgets the check and clears the text;
  the close and any return to Settings wait for dialogs (G23); "Adding…"
  while pending (R-27). Shares are Task 7's; the `share` field stays
  `None` until then.
- [ ] **Step 6: Named edits** E-1 (Task 4's part: the moved tests open
  the sheet), E-1b (Task 4's part), E-1e, E-4 (Task 4's part), E-5, E-6,
  E-7, E-8, E-12, E-15, E-16.
- [ ] **Step 7: Run** P3–P8, P15, P23 and every edited test, and by module
  `desktop::view::add_repository::tests`,
  `desktop::view::settings::repositories`,
  `desktop::view::repositories`, `desktop::view::dock::tests::toolbars`,
  `desktop::view::lifecycle::outcome_canned_tests`,
  `desktop::view::keyboard_tests::settings`, Cycle 04's
  `desktop::view::workspace::settings_sheet::tests`, and Task 0's list.
- [ ] **Step 8: Verify, commit, push, ticket comment.**

---

## Task 5: Add An Existing Folder (B5, B6, B7, B10's enablement; Part G)

- [ ] **Step 1: Tests first:** P9 (B6's table; `Busy` through a
  `#[cfg(test)]` accessor; G20 is Task 6's), P10 (with G16 and the
  no-commits row), P11 (B10's table and G22), P21's Add rows and P22
  (`#[cfg(unix)]`), in `D/view/add_repository_tests.rs`.
- [ ] **Step 2: Header and mode** (B5): `frame::sheet_header` with the
  title and description; the mode control as Cycle 02's segments
  (`D/view/tickets/filter_bar.rs`), each a Tab stop with Enter and Space.
- [ ] **Step 3: The folder and its checks** (B6), in
  `add_repository/checks.rs`: the rows and their actions, Open its
  settings and the two Valid-configuration rows included; the prefix not
  drawn for a Valid configuration (R-31); Browse… and its note line;
  Retry, Check again, Choose another and Create one here (its focus row
  G20 is Task 6's).
- [ ] **Step 4: The main branch** (B7): a `SelectState` whose items are
  the checked-out branch first, then the others; the delegate draws the
  Suggested tag and the greyed reason, refuses other branches in
  `on_will_change`, and ignores a `Confirm` naming one.
- [ ] **Step 5: Enablement** (B10's table) for Add repository, and
  `resume_submit` going on only when the landed check enables Add (G22).
- [ ] **Step 6: Named edits** E-1a, E-1b (Task 5's part), E-1c (Add's
  part), E-1d, E-2 (Add's part), E-4 (the row).
- [ ] **Step 7: Run** P9–P11, P21, P22, the edited tests, and by module
  `desktop::view::add_repository::tests`.
- [ ] **Step 8: Verify, commit, push.**

---

## Task 6: Create A New Repository (B8)

- [ ] **Step 1: Tests first:** P12, P9's Create one here row (G20), and
  P21's Create rows.
- [ ] **Step 2:** Name, Location and its Browse…, `create-target`, the
  required branch field, the prefix, the optional identity and the note
  (B8); Create repository's enablement (B10); Create one here fills
  Location and Name and focuses Name.
- [ ] **Step 3: Named edits** E-1 (Task 6's part: the Create tests fill
  Name and Location), E-1c (Create's part), E-2 (Create's part).
- [ ] **Step 4: Run** P12, P9, and the edited tests.
- [ ] **Step 5: Verify, commit, push.**

---

## Task 7: Share Changes Through (Part C)

- [ ] **Step 1: Merge `origin/main`** (Global Constraints, Main).
- [ ] **Step 2: Tests first:** P13 (`add_repository`) and P14
  (`workspace::add_sheet`). P14 reads what the worker received, as
  `D/view/settings/remotes_tests.rs` does for `RemoteSelect`, and checks
  that a failed selection leaves the repository registered with its
  Activity row.
- [ ] **Step 3: The select** (C1), in `add_repository/share.rs`, with
  decision 17's default.
- [ ] **Step 4: After the add** (C2): the `share` field of `PendingAdd`
  filled on confirmation by C2's three cases; sent to the registered root
  when the add is added; not sent once the app is finishing; the select
  held on Keep for a Valid configuration with no identity (R-30).
- [ ] **Step 5: Run** P13, P14 and P9's registered-folder row.
- [ ] **Step 6: Verify, commit, push, ticket comment.**

---

## Task 8: The Unlock Key Dialog (Part D)

- [ ] **Step 1: Tests first:** P18 (the key from `fixture.generate_key`,
  `D/test_support/fixtures.rs:920`; "before the read" with
  `hold_read(ReadKind::Keys)`, `D/test_support/delaying.rs:135`; a
  failed read through U-8's seam; a missing key; a read landing after
  the prompt was withdrawn), P19 (by the Unlock button), and P1's and
  P2's Unlock rows (P2's fails).
- [ ] **Step 2:** The dialog on the frame (Part D 1–7): `open_dialog`
  takes `Services` and `show_prompt` passes it; the header with the lock
  tile; the key card and its read under `ViewScope::Unlock`;
  `PASSPHRASE`; the refusal's focus; the note always; Not now · Unlock;
  width 440 px.
- [ ] **Step 3: Named edits** E-9 and E-13. Confirm E-10.
- [ ] **Step 4: Run** P18, P19, P1, P2, the edited tests, and by module
  `desktop::view::passphrase::tests`,
  `desktop::view::keyboard_tests::passphrase`,
  `desktop::view::keyboard_tests::lifecycle` and the Unlock evidence
  tests named in the design's Testing.
- [ ] **Step 5: Verify, commit, push.**

---

## Task 9: Empty Pane, Copy, Privacy, Keyboard Journeys (Parts E, F; P16, P17, P20)

- [ ] **Step 1: Tests first:** P16 (a table over each B6 recovery
  action, and both modes with the identity step), P17 (a remote whose
  fetch URL carries userinfo) and P20.
- [ ] **Step 2:** The empty pane's icon (Part E). No test (Wave Testing).
- [ ] **Step 3:** Part F's constants, where an earlier task did not
  already write them.
- [ ] **Step 4: Run** P16, P17, P20 and
  `desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository`
  (the empty pane's text, `D/view/workspace_tests.rs:595`).
- [ ] **Step 5: Verify, commit, push.**

---

## Task 10: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-14-wave-04-cycle-05a-on-screen-checklist.md` for
  the owner: frames 1d and 2h; the frame on each A3 dialog with its
  hand-drawn rings; the theme slots in the mode the app shows (U-2); the
  empty pane. Record only what the owner reports.
- [ ] **Step 2: Gates** (`AGENTS.md`): the static gates across the tree,
  then the fast tier once with `--no-fail-fast`. No SSH target: this
  Cycle changes `src/desktop/` only. Then the desktop smoke test
  (`devenv shell -- cargo run --locked --features desktop --bin manyhands`),
  run by the owner on Linux Wayland, against a scratch data directory,
  never the real registry.
- [ ] **Step 3:** `git merge-tree --write-tree HEAD origin/main`; merge
  main again if it reports conflicts.
- [ ] **Step 4: Document Amendments 4 and 5:** Cycle 04's documents
  annotated; a comment on `01M5F3GVDYTH1Q5V5FCS9YF7V3` (that comment
  publishes that ticket's branch).
- [ ] **Step 5:** Fill the Cycle document's Status column from the
  ledger; mark the ticket review-ready with a comment. Native CI runs
  after merge, dispatched only with the owner's approval (`AGENTS.md`).
- [ ] **Step 6: After review approval only:** set the ticket's status to
  `done` with `ticket save`; remove `target/` and `.devenv/` from the
  ticket worktree, which otherwise stop `ticket close`'s cleanup; then the
  owner runs `ticket close` from the main checkout, which merges.

## Plan Self-Review

**Acceptance rows to tasks.**

| Cycle row | Tasks | Tests |
| --- | --- | --- |
| A1 | 1, 2, 8 | P1, P2 |
| A2 | 4 | P3, P15 |
| A3 | 4 | P4 |
| A4 | 4 | P6 |
| A5 | 4, 5, 6 | P3–P8, P9 (G20), P10 (G16), P11 (G22), P15, P23 |
| A6 | 3–6, 9 | P9, P16; moved tests |
| A7 | 5, 6, 7 | P9–P13 |
| A8 | 7 | P14 |
| A9 | 8 | P1, P2, P18, P19; the Unlock evidence tests |
| A10 | 9 | P20 |
| A11 | 8, 9 | P17; existing privacy tests |
| A12 | 5 | P9, P21, P22 |
| A13 | 10 | On screen |

**Rulings to tasks.** R-1, R-4, R-19: Tasks 1–2. R-2, R-3: Task 1. R-20:
Task 2. R-8, R-9: Task 3. R-5, R-6, R-14, R-21, R-22, R-24, R-25: Task 4.
R-26 to R-29: Task 4. R-10 to R-12, R-18, R-23, R-31: Task 5. R-13: Task
6. R-30: Task 7. R-16, R-17: Task 8.
Decisions 15–18: Tasks 1–2 (15), 4 (16), 7 (17), 5 (18).

**Placeholders.** None. The interfaces marked "design A1" and "design
B4" name the design's sketch, which the task settles.

**Unverified names.** U-1 (Cycle 04's merged names, Task 0), U-3 (A3's
table, Task 0), U-4 (the Unlock evidence tests' paths, Task 0), U-6 (Task
1, by E-14), U-7 (Task 1). Each step that uses one names the existing
code or test to copy from.
