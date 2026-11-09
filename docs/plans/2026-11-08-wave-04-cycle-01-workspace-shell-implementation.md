---
title: "Wave 04 Cycle 01 Workspace Shell Implementation Plan"
date: 2026-11-08
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6YVNFE6AB0RBGQK72CFJJ4X"
---

# Wave 04 Cycle 01 Workspace Shell Implementation Plan

> **For agentic workers:** Revision 5, 2026-11-08, after review rounds 1
> to 4, decision 16 and decision 7 as revised.
> Not approved, and nothing here is authorized yet. Once the owner
> approves and authorizes implementation, use `implementing-a-cycle` task
> by task, re-read the design, repeat the ticket preflight, and record
> every checkpoint on ticket `01M6V8RK8B0SBCH0YZXFVSR9E3`. Steps use
> checkbox (`- [ ]`) syntax.

**Goal:** Replace the fixed four-region layout with a title bar, a dock
area of four panels around the item area, and a footer that shows, hides
and moves them by mouse and keyboard. The layout is kept across restarts.

**Architecture:**

- Part A: tokens.
- Part B: a pure layout model.
- Part C: the Kit dock area, whose panels draw the Workspace's existing
  regions.
- Parts D to F: the title bar, the footer and the keys.
- Part G: the layout file.
- Part H: the quit.

The tasks run one at a time.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
`zorite-editor` `=0.10.0`; Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-01-workspace-shell.md) and
[design](2026-11-08-wave-04-cycle-01-workspace-shell-design.md), and
`docs/design/02-layout-and-docking.md` for anatomy and copy. "Fact n",
"P-n", "R-n", "E-n", "K-n" and "U-n" refer to the design; "decision n"
refers to the Cycle.

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized
implementation, and chosen the execution method. Task 0 does not start
before that.

**Roles.** The **orchestrator** is the session executing this plan.

- It runs the tasks, or dispatches one implementer per task and then a
  fresh reviewer, one at a time, if the owner chooses that method.
- It alone writes the ledger and the ticket comments, and pushes the
  ticket branch.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M6V8RK8B0SBCH0YZXFVSR9E3`,
on `manyhands/ticket/01M6V8RK8B0SBCH0YZXFVSR9E3`. There are no lane
branches and no extra worktrees. In D5, an extra worktree made
`ticket create` answer `recovery_required` (D5 ledger, Task 16). Every Rust
command is `devenv shell -- cargo …` with `--locked`.

**Boundary.**

- The task changes `src/desktop/` and its tests, plus the documents in the
  design's Document Amendments.
- `src/drafts/` and `packaging/` are read, not changed.
- After a task's commit, both of these must print nothing. They are two
  commands because an exclude applies to the whole pathspec set, so the
  operation files need a command of their own.
```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' Cargo.toml Cargo.lock packaging
git diff --stat origin/main...HEAD -- src/desktop/view/operations.rs \
  src/desktop/view/operations src/desktop/model/operations.rs
```
- A gap stops the task for the owner.

**Main.** Merge `origin/main` at Task 0, at Tasks 4b and 9, and whenever
main moves. Never rebase. If Wave 03 D6 (`01M4CC0VN3VPVVQ3RNWJKXWEVX`)
has merged, the merge stops for a short design checkpoint with the owner,
on adding its attention states to `needs_attention` (decision 3).

**GPUI** only through `gpui_kit::*`, never a glob import.

**Test isolation.**

- Each test uses its fixture's own temporary repositories and data
  directory. No test reads or writes the real data directory (D5 ledger,
  Task 17).
- No network.
- No real waits: move time with `cx.executor().advance_clock` (fact 32).
- A test that hides a panel looks things up with `try_find` (fact 33).

**Editor tests** follow D3's rules:

- the cursor, focus and a drawn frame come before typing;
- use window-level `simulate_input` and `simulate_keystrokes`;
- call `cx.executor().allow_parking()` when waiting on the worker.

**Test runs** (AGENTS.md, Testing Strategy; decision 12).

- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written `… <path>`.
- A test's exact path is the path of the module that includes its file,
  so check that file's `mod` line. New `_tests.rs` files are included as
  `#[path = "…_tests.rs"] mod tests;`, so
  `desktop::view::dock::tests::the_default_layout_is_1b`. Existing ones
  vary: `desktop::keys::tests::each_key_is_bound_to_its_action`,
  `desktop::view::item::sheet_tests::…` (`D/view/item.rs:592-594`),
  `desktop::view::keyboard_tests::<module>::…`.
- Each run must report more than zero tests.
- Between tasks, run only the tests the task wrote or changed, by exact
  name. That includes each named edit (E-n) a task applies.
- No whole-suite run happens before Task 12's fast tier, which includes
  `--bins`. A break found there in a test outside E-1 to E-10 stops for the
  owner, or, if it is an appearance test, is deleted under E-8.
- Any single command that runs past about 15 minutes is stopped. Narrow
  the filter and note it in the ticket.
- Never build while a test run is in progress. In D5, tests that re-run
  their own binary failed when a build replaced it (D5 ledger, Task 14).

**Each task's last Verify lines.**
```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Dead code.** An item first called by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Called from Task N.`; Task N
removes it.

**Test-first.** Each behaviour's test is written, and seen to fail,
before its code.

**Existing tests** change only as E-1 to E-10. Any other edit stops for the
owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.**

- Add a comment with `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
  run against the main checkout, after Tasks 0, 4b, 9 and 12. Add it only
  while the ticket worktree is clean. Record commits, tests with counts,
  and rulings.
- Before `ticket save`, commit any owned `ticket.md` edit.

**Execution ledger.**
`docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-execution.md`,
created in Task 0 and written only by the orchestrator.

**File size.** New source files stay below about 500 lines.

## Review Focus

1. **Hiding loses nothing.** A hidden panel keeps its state; a request
   finishes while Activity is hidden; closing and opening a dock keeps
   focus on a Kit button elsewhere (P2, P28, P30; Task 5).
2. **The handler settles.** Every `LayoutChanged` is handled
   idempotently. Moves and hides stay as made, a drop on the item area is
   undone, and closed docks stay closed (P4, P8, P10; Task 5).
3. **R-60.** Every new key and footer control is refused behind a sheet or
   dialog, with panels hidden (P18; Task 8, E-6).
4. **Bold stays Bold.** Ctrl B in an editor is Bold (P17; Task 8).
5. **Fallback.** Each bad file restores the default, with the notice
   where decision 5 says, and no other store changes (P24; Task 9).
6. **The quit.** A waiting quit changes no layout and never focuses a
   control that is not displayed; the quit's write is the newest (P26,
   P27, P31, P35; Tasks 9, 10).
7. **Contents unchanged.** The Boundary diff is empty at every task.

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, re-check references, ledger, approval follow-ups. |
| 1 | A | Chrome tokens; item view reads `accent` and `faint`; E-7. |
| 2 | B | Layout model, default and F6 stops (pure). |
| 3 | B, E | Needs attention count and window title (pure). |
| 4a | C | Regions split; Navigation split; Find to Tickets; New ticket… and New document… kinds; E-1 to E-4. |
| 4b | C | Dock area, panels, builders, delegating renderer, default layout; harness; checkpoint. |
| 4c | C | Panel header per `02`. |
| 5 | C | Layout operations: hiding, moves, refusal, focus. |
| 6 | D | Title bar, window title, icons. |
| 7 | E | Footer and dock menu by mouse; the new actions. |
| 8 | F | Key bindings and reveals; E-5, E-6. |
| 9 | G | Layout file, saving, restore, notices; E-9; checkpoint. |
| 10 | H | Quit with Activity hidden. |
| 11 | — | Keyboard journey; on-screen checklist. |
| 12 | — | Gates, review-ready, closure. |

New source files:

- `D/model/layout.rs`, `D/model/attention.rs`
- `D/services/layout.rs`
- `D/view/shell.rs`, `D/view/shell/{tokens,title}.rs`
- `D/view/dock.rs`, `D/view/dock/{panels,header}.rs`
- `D/view/footer.rs`, `D/view/footer/menu.rs`

New test files, each included with `#[path = "…_tests.rs"] mod tests;` as
`D/assets.rs:85-87` does:

- `D/model/layout_tests.rs`, `D/model/attention_tests.rs`;
- `D/view/shell/title_tests.rs`, `D/view/dock_tests.rs`,
  `D/view/footer_tests.rs`;
- `D/view/dock/restore_tests.rs`, included from `dock.rs` as
  `#[path = "dock/restore_tests.rs"] mod restore_tests;`, so its tests
  run as `desktop::view::dock::restore_tests::<name>`;
- `D/view/keyboard_tests/shell.rs`, a `mod shell;` in `keyboard_tests.rs`.

`D/view/shell.rs`, `D/view/dock/{panels,header}.rs`,
`D/view/footer/menu.rs` and `D/services/layout.rs` are tested through
those files.

Order: 0 → 1 → 2 → 3 → 4a → 4b → 4c → 5 → … → 12, one at a time.

---

## Task 0: Baseline, References, Ledger And Follow-Ups

- [ ] **Step 1: Merge and baseline.**
```sh
git fetch origin
git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```
  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 2: Re-check facts.** Run
  `git diff --stat 2b1dc4d5 HEAD -- src/desktop docs/Waves docs/RFC docs/design Cargo.lock`.
  - Re-read each fact whose file changed, and correct the design first.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base`, `gpui-component` or
    `zorite-editor`, stop for the owner: facts 1–19 rest on the locked
    versions.
- [ ] **Step 3: Sweep the E-list.** Count the test sites only:
```sh
git grep -c -e 'Region::Navigation' -e 'region-navigation' -- \
  'src/desktop/*_tests.rs' 'src/desktop/view/keyboard_tests*'
git grep -c -e 'pane-tickets' -e 'pane-documents' -e 'pane-showing' \
  -e 'SHOWING_DOCUMENTS' -e 'SHOWING_TICKETS' -- \
  'src/desktop/*_tests.rs' 'src/desktop/view/keyboard_tests*'
```
  At `2b1dc4d5` the first command prints 8 files: `find_tests` 2,
  `item/page_tests` 4, `keyboard_tests.rs` 12, `actions` 2, `editing` 8,
  `frame` 2, `made_ready_tests` 1 and `workspace_tests` 3. The second prints
  7 files: `create_tests` 7, `keyboard_tests.rs` 10, `editing` 2,
  `made_ready_review_tests` 1, `operations_tests` 2, `tabs_tests` 1 and
  `tickets_tests` 11. These counts include the label test's lines in
  `tickets_tests` and the `pane-showing` checks, both E-3's. Any other
  count is a new test site, and it stops for the owner. Also run
  `git grep -n simulate_resize -- src/desktop`: at `2b1dc4d5` it prints the
  harness (`harness.rs:119`) and E-10's ten sites. Any other site stops
  for the owner.
- [ ] **Step 4: Approval follow-ups.**
  - Confirm that the approval commit applied Document Amendments 1–9
    and 11.
  - Make Amendments 10 and 11's `ticket.md` edits, commit them, and
    `ticket save`.
  - Comment on D6's ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`: whichever of D6
    and this Cycle merges second adds D6's states to `needs_attention`
    (decision 3).
- [ ] **Step 5: Ledger, commit, ticket comment** recording the base SHA.

---

## Task 1: Chrome Tokens (Part A)

- [ ] **Edit E-7, first:** `view/item/tokens_tests.rs` also asserts that
  `accent` and `faint` equal `ChromeColors`'s in both modes. It fails until
  `ChromeColors` exists. No new colour test (AGENTS.md).
- [ ] **Code:** add `D/view/shell.rs` and `D/view/shell/tokens.rs`
  (`ChromeColors`, `chrome_colors(cx)`, `07`'s values). `view/item/tokens.rs`
  then reads `accent` and `faint` from it.
- [ ] **Verify:** the edited test by exact name; the four lines.
- [ ] **Commit.**

## Task 2: Layout Model (Part B)

- [ ] **Tests** (`D/model/layout_tests.rs`):
  - `the_default_is_1b` (P1).
  - `operations_follow_the_rules` is one table. Its columns are the start
    state, the operation, then `on`, `shown`, displayed, open, and the
    tab selected. Its rows cover P2–P9 and P13. They include an inactive
    tab revealed and decision 6.
  - `f6_stops_follow_the_arrangement` (P14), including a hidden panel, a
    hidden dock and a tabbed group.
- [ ] **Code:** `D/model/layout.rs`. It holds:
  - `PanelKind`, `Side` and `Layout`;
  - names, short labels and icons;
  - the operations and the derived states;
  - `Layout::default()`;
  - `f6_stops(&Layout) -> Vec<Stop>`;
  - `arrangement_from(&DockAreaState)`, which reads each dock's leaf
    `panel_name`s in order, with each group's active index.

  No GPUI entity and no window.
- [ ] **Verify:** each test by exact name; the four lines.
- [ ] **Commit.**

## Task 3: Needs Attention Count And Window Title

- [ ] **Tests:**
  - `D/model/attention_tests.rs` `counts_what_awaits_the_person` is one
    table (decisions 3 and 14, R-13). Its rows: `Finished` with each
    `Outcome`; `Dismissed`; `NotSent`; flush failures; drafts unavailable;
    pending rows; a `Failed` pending read; and a `Busy` one. Build the
    requests as `operations_tests.rs` does.
  - `D/view/shell/title_tests.rs` `title_names_the_selected_repository`
    covers a selected repository, none, and a root without a directory
    name (P22, fact 29).
- [ ] **Code:**
  - `D/model/attention.rs` `needs_attention(&Operations, &DraftRegistry) -> usize`,
    which reads only;
  - `D/view/shell/title.rs` `window_title(Option<&RepositoryEntry>) -> String`.
- [ ] **Verify:** each test by exact name; the four lines; the Boundary
  diff.
- [ ] **Commit.**

## Task 4a: Regions, The Navigation Split And Item Kinds (Fixed Layout)

The row stays fixed for now, with five regions. Documents sits under
Repositories in the 260 px column, Tickets fills the 320 px column, and
Content is unchanged. This lands the region and list changes on their own,
before the dock.

- [ ] **Test first:** `desktop::view::create::tests::new_ticket_and_new_document_create_their_own_kind`
  (P16). Click New ticket… while `Pane` is Documents, and New document…
  while it is Tickets. Each opens its own kind.
- [ ] **Code:**
  - `Region` becomes `{ Repositories, Documents, Content, Tickets,
    Operations }` (R-7), with `REGIONS` in that order (decision 9), and
    `regions` becomes `[FocusHandle; 5]`. The new ids are `region-tickets`
    and `region-documents`.
  - Both lists keep the "Navigation" key context.
  - `navigation_pane` splits into a Tickets body (Find first, decision 4)
    and a Documents body. The choice and `pane-showing` go.
  - `Pane` is set when either list's region takes focus (R-8), and
    `FocusNavigation` focuses `Pane`'s list.
  - New ticket… and New document… pass their own kind. `NewItem` takes the
    kind from the focused list.
  - Add `Shell::use_list(Pane, cx)` to `D/view/harness.rs` (E-2). It sets
    the list used last and focuses that list's region. `show_pane` alone
    focuses only from Navigation (fact 33).
- [ ] **Edits:** E-1, E-2, E-3 and E-4, exactly as the design lists them.
  An appearance test the new column breaks is handled under E-8.
- [ ] **Verify:** the new test and every edited test by exact name; the
  four lines.
- [ ] **Commit.**

## Task 4b: Dock Area, Panels And Default Layout (Part C)

- [ ] **Step 1: Design checkpoint, before code.** Write in the ledger:
  - the panel entity shape, and which fields `visible` and
    `focus_handle` read (R-1);
  - the builder map (R-34);
  - `ShellRenderer`'s delegation to DockSkin, and the drop indicator;
  - the bookkeeping that moves into the bodies (R-33);
  - how the Workspace builds the default layout with `DockLayout`.
- [ ] **Tests** (`D/view/dock_tests.rs`), written first:
  - `each_panel_draws_its_region_inside_the_dock`. Every region id and
    `operation-area` is found within the dock area's scope (`within`),
    each in its panel's group, and the fixed row's container is gone. It
    fails before the code and proves U-4.
  - `the_default_layout_is_1b` (P1), using `Shell::open_as_launched`. From
    `DockArea::dump`: Repositories is over Documents in proportion
    0.6 : 1.4 within 1 px (U-6); Tickets is on the right and Activity at
    the bottom; the dock sizes are 290, 440 and 150 px; focus is on
    Repositories.

  The existing `desktop::view::item::sheet_tests::no_workspace_key_acts_behind_the_sheet`
  must pass unchanged with the sheet inside the item area panel (R-29).
- [ ] **Code:**
  - `D/view/dock.rs` and `D/view/dock/panels.rs` (R-1, R-27, R-33, R-34);
  - `D/view/dock/header.rs`: `ShellRenderer` and a plain `PanelHeader`
    that draws titles only (R-2), finished in Task 4c;
  - the Workspace holds the area and the panels, and `render.rs`'s fixed
    row is replaced by the area, with the item area as its centre and
    Root's sheet layer still in the item area's body (fact 21);
  - `cycle_region` uses `f6_stops`. `focus_lost` skips regions that are
    not displayed and falls back to the item area (R-9);
  - `ItemAreaPanel`'s body calls the content region's `note_focus`, and
    the Activity body computes `finishing` and prunes after its sections
    (R-33);
  - `ShellRenderer` built inside the area's constructor, as
    `DockSkin::dock_area` does (R-2);
  - the harness (R-26):
    - the window becomes 1590 × 1726;
    - `Shell::open` and `Shell::launch` set the bottom dock to 520 px
      after launch;
    - `Shell::resize_keeping_content(width, height, cx)` is added;
    - `Shell::open_as_launched` resizes nothing.
- [ ] **Edits:** E-10. Several sites are shared helpers (`frame_tests.rs`'s
  `shell()`, `posting_tests/support.rs`, `keyboard_tests/discussion.rs`), so
  run every test that reaches each site, by exact name, and list them in
  the ledger.
- [ ] **Verify:** the two tests and the sheet test by exact name; the
  existing tests that use the region helpers in `workspace_tests.rs` and
  `keyboard_tests.rs` (`focused_region`, `ringed`), by exact name; the
  four lines.
- [ ] **Commit; merge main; ticket comment (dock checkpoint).**

## Task 4c: Panel Header (Part C)

- [ ] **Test:** `desktop::view::dock::tests::clicking_a_tab_selects_its_panel`.
  Put Documents in Repositories' group with `move_panel` and
  `InsertTarget::Tabs { activate: false }`, so Repositories stays active.
  Then click `panel-tab-documents`. Documents is now the active panel in
  the dump, and its region is drawn.
- [ ] **Code:** `PanelHeader`, per `02` and Part C:
  - icon, title and count (R-22);
  - tabs with the amber line, with string ids `panel-tab-{kind}` and short
    labels when a group has more than two tabs (R-32);
  - drag sources and drop targets through `TabGroupContext`, with the
    app's own drag preview;
  - the drop indicator from the skin's group renderer;
  - no tab bar for the item area;
  - a collapsed group drawn as one tab per panel, with no active panel
    drawn and no drag or drop on its tabs (R-35). Its tabs reveal in Task
    5.

  Appearance gets no tests (Wave Testing). Dragging is checked on the
  checklist.
- [ ] **Verify:** the test by exact name; the four lines.
- [ ] **Commit.**

## Task 5: Layout Operations (Part C)

The Workspace gains `toggle_panel(kind)`, `toggle_dock(side)`,
`reveal(kind)` and `dock_panel(kind, side)`. Tasks 7, 8 and 10 call them.

This task is test-first, with a design checkpoint: the Required Outcomes
P2–P12, P30, P33 and P34 are its contract. Before code, write in the
ledger the reconcile step, the `LayoutChanged` handler's three steps
(design Part C) and how each test reaches its state.

- [ ] **Tests** (`D/view/dock_tests.rs`). Save checks are marked
  `// Completed in Task 9.` until the store exists.
  - `hiding_a_panel_gives_its_space_to_its_dock` (P2).
  - `hiding_the_last_panel_closes_its_dock` (P4). Covers a side dock and
    the bottom dock, which keeps its strip, and a drag that leaves only
    panels that are off.
  - `a_dock_closed_and_opened_keeps_its_panels_groups_and_sizes` (P5,
    P30). Focus is on the item header's Edit button, a Kit button, while
    the left dock closes and opens. The dumped trees are equal and the
    sizes within 1 px. Focus, the item area's tabs and the centre's node
    ids are unchanged.
  - `toggling_a_dock_with_every_panel_off_shows_them` (P6).
  - `a_side_with_no_panels_has_no_dock` (P7).
  - `each_panel_docks_to_each_side` (P8) is one table: 4 panels × 3 sides
    through `dock_panel`. It covers an open target, a closed target and a
    side with no panels, from an open source and from a closed one. It
    checks the arrangement, the panel displayed, and the source dock
    removed or closed as P8 says.
  - `docking_a_panel_where_it_is_changes_nothing` (P9).
  - `a_drop_on_the_item_area_is_undone` (P10). `move_panel` with
    `InsertTarget::Tabs` on the centre's group is the edit a drop commits
    (fact 10). One row has no closed dock and one has the left dock
    closed. Afterwards the dump equals the accepted state, and the closed
    dock is still closed.
  - `a_panel_moved_into_a_tab_group_is_kept` (P11). `move_panel` puts
    Tickets in Repositories' group. The arrangement follows and Tickets is
    displayed. A save check and a `Shell::relaunch` follow in Task 9.
  - `revealing_an_inactive_tab_selects_it` (P3, P13).
  - `hiding_the_active_tab_selects_the_next` (Part B).
  - `the_closed_strip_names_and_reveals_every_panel` (P33). Cover a
    single group, a bottom dock split side by side, and a dock with every
    panel off.
  - `opening_the_strip_by_its_edge_counts_as_its_toggle` (P34). The Kit's
    `toggle_dock` on the closed bottom dock stands in for the edge drag.
  - `focus_leaves_a_panel_that_is_not_displayed` (P12). Cover its toggle,
    its dock (the bottom one included), another tab selected by toggle,
    and a click on the closed strip outside its tabs. A tab click keeps
    focus on the panel it selects.
  - `a_hidden_panel_keeps_its_state` (P2). Tickets, with a filter, Find
    text and a selection, is hidden and shown again.
  - `a_request_finishes_while_activity_is_hidden` (P28). Its entry shows
    when Activity does. No tab opened, closed or reloaded, and the worker
    received nothing from hiding or showing.
- [ ] **Code** in `D/view/dock.rs`:
  - R-3 reconcile (with R-35's visible flags while the bottom dock is
    closed), R-5, R-6's idempotent handler, R-9's focus check (after
    reconcile and on focus-in), R-31, R-35's reveal and R-36;
  - R-38 for the default launch: the accepted state and arrangement are
    set from a dump when the area is built. Task 9 adds the restored
    launch;
  - R-10's `show_holder` reveal, with the test
    `show_it_reveals_a_hidden_activity`;
  - the four operations, each ending with reconcile, the focus check and
    a scheduled save.
- [ ] **Verify:** each test by exact name; the four lines.
- [ ] **Commit.**

## Task 6: Title Bar And Window Title (Part D)

- [ ] **Test:** `desktop::view::shell::title::tests::the_window_title_follows_the_selected_repository`.
  Select a repository, select another, then remove it, and read
  `VisualTestContext::window_title()` after each (P22, fact 18).
- [ ] **Code:**
  - The main window's options become `TitleBar::window_options()` with
    the bounds and client decorations (R-15). The startup-failure window
    keeps today's options.
  - Build the title bar in `D/view/shell.rs`: app icon, title text, and
    `on_close_window` → `request_quit`.
  - Call `set_window_title` when the title changes.
  - Icons (R-16, R-23): add `Activity` to `ItemIcons`. Add the small app
    icon PNG under `D/` and serve it from `DesktopAssets` at
    `brand/manyhands.png`.
- [ ] **Verify:** the test and `desktop::assets::tests::…`'s existing
  tests by exact name; the four lines.
- [ ] **Commit.**

## Task 7: Footer And Dock Menu By Mouse (Part E)

- [ ] **Tests** (`D/view/footer_tests.rs`), one per kind of control:
  - `a_panel_toggle_hides_and_reveals_its_panel` (P2, P3). Its `label()`
    and `checked()` follow whether it is displayed (R-4); an inactive tab reads off.
  - `a_dock_toggle_reveals_its_hidden_panels` (P6).
  - `a_secondary_click_opens_the_dock_menu_with_its_side_checked` (P8,
    P9). Choose "Dock right": the toggle is now in the right group.
  - `needs_attention_counts_and_leads_to_activity` (P20). It is hidden at
    0, reads "1 needs attention", and a click reveals and focuses
    Activity.
  - `settings_opens_the_settings_page` (P21).
  - `footer_controls_do_nothing_behind_a_dialog` (P18). With a dialog
    open (dialogs keep their overlay, fact 33), clicking a panel toggle and
    a dock toggle leaves the layout unchanged and the dialog open.
  - `toggle_names_follow_their_state`: the pure name function (R-24).
- [ ] **Code:**
  - `D/view/footer.rs` and `D/view/footer/menu.rs` (R-4, R-11, R-13,
    R-24, R-29, R-30);
  - `D/keys.rs` gains the actions `ToggleLeftDock`, `ToggleRightDock`,
    `ToggleBottomDock`, `OpenDockMenu` and `DockPanel` (R-12), without
    bindings yet;
  - the workspace root renders the footer below the area.
- [ ] **Verify:** each test by exact name; the four lines.
- [ ] **Commit.**

## Task 8: Keyboard (Part F)

- [ ] **Tests** (`D/view/keyboard_tests/shell.rs`, registered in
  `keyboard_tests.rs`):
  - `dock_keys_toggle_each_dock` (P17).
  - `ctrl_b_in_an_editor_is_bold` (P17, decision 2). Following D3's editor
    rules, the editor's text gains bold markup and the left dock is
    unchanged.
  - `region_keys_and_find_reveal_hidden_panels` (P13, P15), including an
    inactive tab.
  - `f6_skips_hidden_panels` (P14).
  - `a_panel_toggle_works_by_tab_and_enter` (A2).
  - `each_side_is_reached_by_the_dock_menu_keys` (P19). Use `menu` for one
    move and `shift-f10` for the others, with arrows and Enter: one panel
    to each side. Escape returns focus to the toggle.
  - `needs_attention_works_by_enter` (P20).
  - `footer_controls_are_reached_by_tab` (P29). Each control is focused in
    turn, with its ring, and has its `label()` and, for toggles,
    `checked()`.
- [ ] **Edits:** E-5 and E-6.
- [ ] **Code:**
  - bindings for the new actions (Part F, R-10), each handler wrapped in
    `unless_dialog`;
  - the "PanelToggle" key context on each toggle wrapper;
  - reveals in `focus_region` and `focus_find`.
- [ ] **Verify:** each test, `desktop::keys::tests::each_key_is_bound_to_its_action`
  and `desktop::view::item::sheet_tests::no_workspace_key_acts_behind_the_sheet`
  by exact name; the four lines.
- [ ] **Commit.**

## Task 9: Persistence (Part G)

- [ ] **Tests:**
  - `desktop::model::layout::tests::restore_restores_or_falls_back` is
    **the one table-driven restore test** (Wave Testing; P24). It calls
    `restore` with one row per case of Part G's table:
    - `Ok(None)` (no file), and an `Err` (cannot be read);
    - not JSON, and JSON of another shape;
    - version 0 and version 2;
    - a dock whose `placement` differs from its slot;
    - an unknown leaf, a missing panel, a repeated panel, a panel in the
      centre, and `ItemAreaPanel` in a dock;
    - `info` and name disagreeing: `TabPanel` with `Panel` info, and a
      leaf name with `Stack` info;
    - an unknown container, a `StackPanel` holding a leaf, a `TabPanel`
      holding a container, a `sizes` count different from the children,
      a split size of 0, a negative split size, a `DockState` size of
      99 px, and a dock with no panels;
    - `on` and `shown` each missing a key, and each with an unknown one;
    - a valid file with a closed dock, a moved panel, a tab group and
      changed sizes.

    Each row asserts the result and whether the notice shows.
  - `D/view/dock/restore_tests.rs`:
    - `a_valid_file_restores_the_layout` (P23, P24, P37). Seed a file,
      then `Shell::open_as_launched`. The dump, the on flags and the shown
      docks match within 1 px, and after the clock advances no save has
      happened.
    - `a_dock_too_large_for_the_window_is_clamped` (P36). Seed side
      docks wider than the window. Once the area has bounds, both are
      scaled so the item area has 320 px, and that is saved. A closed
      oversized dock is scaled when it opens.
    - `a_file_that_cannot_be_read_restores_the_default_and_says_so` (P24,
      P25). `layout.json` is a directory, so the read fails on every
      platform. Use `Shell::open_as_launched` and advance no clock, so no
      save is attempted. Every later write would fail too, and show the
      save notice. The notice shows, and `footer-attention` is absent.
      Dismiss removes the notice. The sentinel files in `drafts/` and
      `resolutions/`, and the data directory's entry list, are unchanged,
      and so are the repositories' refs and status.
    - `the_layout_survives_a_restart` (P23, A3, A4, A14). Hide Documents
      (P2), close the bottom dock, move Tickets left and `set_dock_size`
      the left dock. Advance past `LAYOUT_SAVE_DELAY`, then
      `Shell::relaunch`. The file is at `<data>/layout/layout.json`, and
      the layout matches.
    - `a_change_just_before_a_quit_that_exits_at_once_is_kept` (P31). Make
      a change, quit with no job running and no clock advance, then
      relaunch.
    - `a_change_during_a_quit_wait_is_kept` (P31). Start a job and quit;
      while it waits, open Activity; release the job, then relaunch.
      Activity is open.
    - `a_stale_write_loses_to_the_quits` (P35). Hold a background write
      with `hold_writes_for_testing`, make another change and quit, then
      release the held write. The file holds the newer layout.
    - `a_failed_write_shows_the_save_notice` (P32). With
      `fail_writes_for_testing`, a write fails and the save notice shows;
      a later successful write removes it.
  - Complete Task 5's marked save checks, and the relaunch in
    `a_panel_moved_into_a_tab_group_is_kept`.
- [ ] **Edits:** E-9.
- [ ] **Code:**
  - `D/services/layout.rs` `LayoutStore` (R-17), reached through
    `Backend::layout()`. `LibraryBackend` opens it; `DelayingBackend` and
    the fixture open it lazily over the fixture's data directory.
  - Store-allocated generations behind one `Arc` that every clone
    shares, one write in flight, and the two test seams (R-19). `hold_writes_for_testing` parks before the store's lock.
  - `restore` in `D/model/layout.rs` (R-18, R-28).
  - Restore at launch (Part G's four steps, R-38) and the clamp (R-37).
  - Saving (R-19, R-31), including the write in `quit_now`.
  - The two notices (R-21).
  - `Shell::relaunch` in the harness, which resizes nothing (R-26).
- [ ] **Verify:** each test by exact name, and Task 5's completed tests;
  the four lines.
- [ ] **Commit; merge main; ticket comment (persistence checkpoint).**

## Task 10: Quit (Part H)

- [ ] **Tests** (`D/view/workspace_tests.rs`, beside the existing quit
  tests), each written and seen to fail first:
  - `a_waiting_quit_with_activity_hidden_changes_nothing` (P26). Hide
    Activity, start a job, focus the item area, then quit.
    - The layout is unchanged, and focus is still in the item area.
    - `secondary-4` reveals Activity, which shows "Finishing {job}" and
      Quit now.
  - With Activity displayed, the existing
    `finishing_closes_the_dialog_and_focuses_quit_now`
    (`workspace_tests.rs:1064`) still covers Quit now's focus (P26).
  - `desktop::view::workspace::tests::saving_drafts::drafts_not_saved_with_activity_hidden_focuses_the_item_area`
    (P27), `#[cfg(unix)]` like the recipe it uses. Use the `saving_drafts`
    failure recipe with Activity hidden. On
    Cancel, the item area is focused and Needs attention counts the
    failure. With Activity displayed, the existing
    `a_draft_that_cannot_be_saved_asks_before_quitting` (`:476-524`) still
    shows Retry focused.
- [ ] **Code:** R-20 in `wait_to_quit` and `not_saved_chosen`
  (`D/view/workspace/lifecycle.rs`): Quit now and Retry take focus only
  when Activity is displayed.
- [ ] **Verify:** the new tests, `finishing_closes_the_dialog_and_focuses_quit_now`,
  the existing quit tests in `workspace_tests.rs`'s `saving_drafts`
  module, and the Edits not kept tests in `view/item/edit_quit_tests.rs`
  (P27's other return), each by exact name; the four lines.
- [ ] **Commit.**

## Task 11: Keyboard Journey And Checklist

- [ ] **Test:** `desktop::view::keyboard_tests::shell::arrange_the_workspace_by_keyboard_alone`
  (A2, A4, A10, A18). With no clicks and no Workspace calls:
  - hide Documents from the footer;
  - move Tickets to the left dock with its dock menu;
  - hide and show the bottom dock with `secondary-j`;
  - hide Activity from the footer, then focus it with `secondary-4`, which
    reveals it;
  - relaunch, and find the same layout.

  The worker receives no request at any step (RFC 80-84; D5's A17
  pattern).
- [ ] **Write** `docs/plans/2026-11-08-wave-04-cycle-01-on-screen-checklist.md`
  (design, Testing), in D5's format, grounded in the app's strings.
- [ ] **Verify:** the test by exact name; the four lines.
- [ ] **Commit.**

## Task 12: Gates, Review-Ready, Closure

- [ ] Fetch and merge main, then run the gates:
```sh
git fetch origin
git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib --bins \
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
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
git merge-tree --write-tree HEAD origin/main
```
  The fast tier above is AGENTS.md's at the time of writing; use main's
  AGENTS.md as it is at Task 12. The Boundary diff must print nothing. A
  fast-tier failure outside E-1 to E-10 stops for the owner, unless it is an
  appearance test deleted under E-8.
- [ ] Whole-branch spec and quality reviews. Fill the Cycle's Acceptance
  Status column.
- [ ] The owner runs the checklist, including AGENTS.md's desktop smoke run
  (`devenv shell -- cargo run --locked --features desktop --bin manyhands`).
  Claude records only what the owner reports.
- [ ] Ticket comment: review-ready; `ticket save`.
- [ ] After code-review approval, and only with the owner's go-ahead:
  - set the ticket's status text to its resolution and `ticket save`;
  - close it from the registered main checkout with
    `devenv shell -- cargo run --locked --bin manyhands-cli -- ticket close --repo /home/vhodges/work/src/manyhands --id 01M6V8RK8B0SBCH0YZXFVSR9E3`.

  The close performs the merge, publishes main, and removes the branch and
  worktree. With a passphrase-protected key, the owner runs it at their
  terminal. Native CI runs after merge, on main, only when the owner
  dispatches it.

---

## Plan Self-Review

| Acceptance / outcome / ruling / decision | Task |
| --- | --- |
| A1, P1 | 2, 4b |
| A2, P2–P9, P19 | 2, 5, 7, 8, 11 |
| A3 | 5 (P5), 9 (sizes restored); checklist (resizing) |
| A4, P23, P31, P35 | 9, 11 |
| A5, P24, P25 | 9 |
| A6, P22 | 3, 6 |
| A7, P26, P27 | 10 |
| A8 | 4a (E-1 to E-4), 8 (E-5, E-6), 12 (fast tier) |
| A9, P20 | 3, 7, 8 |
| A10, P13, P14, P17 | 2, 5, 8 |
| A11, P15 | 4a, 8 |
| A12, P18 | 8 (E-6) |
| A13, P29 | 8; title bar on the checklist |
| A14 | 9 |
| A15 | 1 (E-7) |
| A16 | 11 (checklist), after merge |
| A17, P2, P28, P30 | 5; Boundary diff in every task |
| A18 | 5 (P28), 11 |
| A19, P32 | 9 |
| A20, P11 | 4c, 5, 9 |
| P10, P12, P33, P34 | 5 |
| P36, P37 | 9 |
| P16 | 4a |
| P18 | 7, 8 |
| P21 | 7 |
| R-1, R-26, R-27, R-33, R-34 | 4b |
| R-2 | 4b, 4c |
| R-35 | 4c, 5 |
| R-38 | 5, 9 |
| R-3, R-5, R-6, R-31, R-36 | 5 |
| R-37 | 9 |
| R-4, R-11, R-12, R-13, R-24, R-30 | 7 |
| R-7, R-8 | 4a |
| R-9 | 4b, 5 |
| R-10 | 8 |
| R-14–R-16, R-23 | 6 |
| R-17–R-19, R-21, R-28 | 9 |
| R-20 | 10 |
| R-22, R-32 | 4c |
| R-29 | 4b, 7, 8 |
| E-1–E-4 | 4a |
| E-5, E-6 | 8 |
| E-7 | 1 |
| E-8 | any task from 4a; 12 |
| E-9 | 9 |
| E-10 | 4b |
| U-3 | checklist |
| U-4, U-6 | 4b |
| Decisions 1, 11 | Scope |
| Decision 2 | 8 |
| Decisions 3, 14 | 3, 7; Task 0 (D6 comment) |
| Decision 4 | 4a, 8 |
| Decision 5 | 9 |
| Decision 6 | 2, 5, 7 |
| Decision 7 | 10 |
| Decision 8 | 7, 8 |
| Decision 9 | 2, 4a |
| Decision 10 | 5 |
| Decision 12 | Global Constraints |
| Decision 13 | 9 |
| Decision 15 | 4b (R-26) |
| Decision 16 | 4c, 5 (R-3, R-35) |
