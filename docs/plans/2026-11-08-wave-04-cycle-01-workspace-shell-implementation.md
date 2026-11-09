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

> **For agentic workers:** Draft, 2026-11-08. Not approved; nothing here
> is authorized yet. Once the owner approves and authorizes
> implementation, use `implementing-a-cycle` task by task, re-read the
> design, repeat the ticket preflight, and record every checkpoint on
> ticket `01M6V8RK8B0SBCH0YZXFVSR9E3`. Steps use checkbox (`- [ ]`)
> syntax.

**Goal:** Replace the fixed four-region layout with a title bar, a dock
area of four panels around the item area, and a footer that shows, hides
and moves them, by mouse and keyboard, with the layout kept across
restarts.

**Architecture:** Part A tokens; Part B a pure layout model; Part C the
Kit dock area, whose panels draw the Workspace's existing regions; Parts
D–F the title bar, footer and keys; Part G the layout file; Part H the
quit. Serial.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
`zorite-editor` `=0.10.0`; Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-01-workspace-shell.md) and
[design](2026-11-08-wave-04-cycle-01-workspace-shell-design.md), and
`docs/design/02-layout-and-docking.md` for anatomy and copy. "Fact n",
"P-n", "R-n", "E-n", "K-n" and "U-n" refer to the design; "decision n" to
the Cycle.

## Global Constraints

**Precondition.** The owner has approved the three documents and
authorized implementation, and chosen the execution method (decision 12
covers test runs only). Task 0 does not start before.

**Where.** Every task runs in `.manyhands/worktrees/01M6V8RK8B0SBCH0YZXFVSR9E3`
on `manyhands/ticket/01M6V8RK8B0SBCH0YZXFVSR9E3`. No lane branches. Every
Rust command is `devenv shell -- cargo …` with `--locked`.

**Boundary.**

- Changes are confined to `src/desktop/` and its tests, plus the
  documents in the design's Document Amendments.
- These stay untouched: `src/lib.rs`, `src/repository.rs`,
  `src/repository/`, `src/results.rs`, `src/drafts/` (its `Folder` is
  used, not changed), `src/runtime.rs`, `src/bin/`, `Cargo.toml` and
  `Cargo.lock`.
- So do `src/desktop/view/operations.rs`, `src/desktop/view/operations/`
  and `src/desktop/model/operations.rs`. Check each task with
  `git diff --stat <base> -- src/desktop/view/operations.rs src/desktop/view/operations src/desktop/model/operations.rs`,
  which must print nothing.
- A gap stops the task for the owner.

**Main.** Merge `origin/main` at Task 0, at Tasks 4b and 9, and whenever
main moves. Never rebase. If Wave 03 D6 (`01M4CC0VN3VPVVQ3RNWJKXWEVX`)
has merged, the merge stops for a short design checkpoint with the owner
on adding its attention states to `needs_attention` (decision 3).

**GPUI** only through `gpui_kit::*`, never a glob import.

**Test isolation.**

- Each test uses its fixture's own temporary repositories and data
  directory. No test reads or writes the real data directory (the lesson
  of D5 Task 17).
- No network.
- No real waits: move time with `cx.executor().advance_clock` (fact 32).

**Editor tests** follow D3's rules:

- the cursor, focus and a drawn frame come before typing;
- use window-level `simulate_input` and `simulate_keystrokes`;
- call `cx.executor().allow_parking()` when waiting on the worker.

**Test runs** (AGENTS.md, Testing Strategy; decision 12).

- Desktop runs: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`,
  written `… <filter>`. For one test, give its full path and add
  `-- --exact`. One filter per run, and each run must report more than
  zero tests.
- Between tasks, run only the tests the task wrote or changed, by exact
  name.
- Decision 12: the whole desktop bin suite (`…` with no filter) once at
  Task 4a, once at Task 4b and once at Task 9. Each run records its pass
  and fail counts and its wall time in the ledger, and any that runs past
  15 minutes is noted there too.
- AGENTS.md's fast tier runs once, at Task 12. No SSH target, since the
  change is desktop-only.

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

**Existing tests** change only as E-1 to E-8. E-8's deletions are
appearance tests only, each named in the ledger with the assertion it
made. Anything else stops for the owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.**

- Add a comment with `manyhands-cli comment add` against the main
  checkout after Tasks 4b, 9 and 12. Add it only while the ticket worktree
  is clean, and record commits, tests with counts and rulings.
- Before `ticket save`, commit any owned `ticket.md` edit.

**Execution ledger.**
`docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-execution.md`,
created in Task 0.

**File size.** New source files stay below about 500 lines.

## Review Focus

1. **Hiding loses nothing.** A hidden panel keeps its state; a request
   finishes while Activity is hidden; a dock reload keeps focus and tabs
   (P2, P28, P30; Task 5).
2. **Centre refusal.** A drop on the item area is undone (P10; Task 5).
3. **R-60.** Every new workspace key is refused behind a sheet or dialog
   (P18; Task 8, E-6).
4. **Bold stays Bold.** Ctrl B in an editor is Bold (P17; Task 8).
5. **Fallback.** Each bad file restores the default, with the notice
   where decision 5 says, and no other file changes (P24; Task 9).
6. **The quit.** An exit keeps the saved layout from before the reveal;
   a quit back to running keeps Activity shown (P26, P27; Task 10).
7. **Contents unchanged.** The operation files' diff is empty at every
   task (Boundary).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, re-check references, ledger. |
| 1 | A | Chrome tokens; item view reads `accent` and `faint`; E-7. |
| 2 | B | Layout model, default and F6 stops (pure). |
| 3 | B, E | Needs attention count and window title (pure). |
| 4a | C | Regions split; Navigation split; Find to Tickets; E-1 to E-4. |
| 4b | C | Dock area, panels, renderer, default layout; E-8; checkpoint. |
| 5 | C | Hiding, moves, centre refusal, focus. |
| 6 | D | Title bar, window title, icons. |
| 7 | E | Footer and dock menu by mouse. |
| 8 | F | Keys; E-5, E-6. |
| 9 | G | Layout file, save, restore, notice; checkpoint. |
| 10 | H | Quit with Activity hidden. |
| 11 | — | Keyboard journey; on-screen checklist. |
| 12 | — | Gates, review-ready, closure. |

New files:

- `D/model/layout.rs`, `D/model/attention.rs`
- `D/services/layout.rs`
- `D/view/shell.rs`, `D/view/shell/{tokens,title}.rs`
- `D/view/dock.rs`, `D/view/dock/{panels,header}.rs`
- `D/view/footer.rs`, `D/view/footer/menu.rs`

Each has a `_tests.rs` beside it, included with `#[path]` as
`D/assets.rs:85-87` does.

Order: 0 → 1 → 2 → 3 → 4a → 4b → 5 → … → 12, one at a time.

---

## Task 0: Baseline, References And Ledger

- [ ] **Step 1: Merge and baseline.**
```sh
git fetch origin
git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```
  A failure is resolved, or accepted by the owner, before Task 1. No test
  suite runs here (AGENTS.md: the tiers run once, at review-ready).
- [ ] **Step 2: Re-check facts.**
  `git diff --stat 2b1dc4d5 HEAD -- src/desktop docs/Waves docs/RFC docs/design Cargo.lock`.
  Re-read each fact whose file changed and correct the design first. If
  `Cargo.lock` moved `gpui-kit`, `gpui-base`, `gpui-component` or
  `zorite-editor`, stop for the owner: facts 1–19 rest on the locked
  versions.
- [ ] **Step 3: Sweep the E-list.** Re-run the design's E-1 to E-4 counts:
```sh
git grep -c -e 'Region::Navigation' -e 'region-navigation' -- src/desktop
git grep -c -e 'pane-tickets' -e 'pane-documents' -e 'pane-showing' -e 'SHOWING_' -- src/desktop
```
  Compare them with E-1 to E-3. A new site stops for the owner.
- [ ] **Step 4: Ledger, commit, ticket comment** recording the base SHA.

---

## Task 1: Chrome Tokens (Part A)

- [ ] **Test** (`D/view/shell/tokens_tests.rs`): `chrome_tokens_give_07s_values_in_both_modes`
  compares each `ChromeColors` field with `07`'s table in dark and light.
  E-7: `view/item/tokens_tests.rs` also asserts that `accent` and `faint`
  equal the chrome tokens in both modes.
- [ ] **Code:** add `D/view/shell.rs` and `D/view/shell/tokens.rs`
  (`ChromeColors`, `chrome_colors(cx)`). `view/item/tokens.rs` `colors`
  reads `accent` and `faint` from `ChromeColors`.
- [ ] **Verify:** `… shell::tokens`; `… item::tokens`; the four lines.
- [ ] **Commit.**

## Task 2: Layout Model (Part B)

- [ ] **Tests** (`D/model/layout_tests.rs`):
  - `the_default_is_1b` (P1);
  - `toggles_follow_the_rules`: one table over Part B's operations, with
    columns start state, operation, then `on`, `shown`, on screen and
    open; it covers P2–P9 and P13's reveal;
  - `f6_stops_follow_the_arrangement` (P14), including a hidden panel, a
    hidden dock and a tabbed group.
- [ ] **Code:** `D/model/layout.rs`. Its pieces:
  - the types: `PanelKind`, `Side` and `Layout`;
  - stable names, labels and icons;
  - the operations and derived states;
  - `Layout::default()`;
  - `f6_stops(&Layout) -> Vec<Stop>`;
  - `arrangement_from(&DockAreaState)`, which reads each dock's leaf
    `panel_name`s in order.

  No GPUI entity or window.
- [ ] **Verify:** `… model::layout`; the four lines.
- [ ] **Commit.**

## Task 3: Needs Attention Count And Window Title

- [ ] **Tests:**
  - `D/model/attention_tests.rs` `counts_what_awaits_the_person` is one
    table: `Finished` with each `Outcome`, `Dismissed`, `NotSent`, flush
    failures, drafts unavailable, pending rows, and a failed pending read
    (R-13). The decision 3 sources are built as `operations_tests.rs`
    builds requests.
  - `D/view/shell/title_tests.rs` `title_names_the_selected_repository`
    checks a selected repository, none, and a root without a directory
    name (P22, fact 29).
- [ ] **Code:**
  - `D/model/attention.rs` `needs_attention(&Operations, &DraftsRegistry) -> usize`.
    It reads only.
  - `D/view/shell/title.rs` `window_title(Option<&RepositoryEntry>) -> String`.
- [ ] **Verify:** `… model::attention`; `… shell::title`; the four
  lines; the operation-files diff is empty.
- [ ] **Commit.**

## Task 4a: Regions And The Navigation Split (Part C, Fixed Layout)

Still the fixed row, now with five regions. This lands the region and list
changes on their own, before the dock.

- [ ] **Code:**
  - `Region` becomes `{ Repositories, Documents, Content, Tickets,
    Operations }` (R-7), with `REGIONS` in that order, which is F6's
    default (decision 9). `regions` becomes `[FocusHandle; 5]`. The new
    ids are `region-tickets` and `region-documents`.
  - Both lists keep the "Navigation" key context.
  - `navigation_pane` splits into a Tickets body (Find first,
    decision 4) and a Documents body, each its own region in the row.
  - The choice and `pane-showing` go.
  - `Pane` is set when either region takes focus (R-8).
  - `FocusNavigation` focuses `Pane`'s list.
  - Add `Shell::use_list(Pane, cx)` to `D/view/harness.rs` (E-2).
- [ ] **Edits:** E-1, E-2, E-3, E-4, exactly as the design lists them.
- [ ] **Verify:** each edited test by exact name; then the whole desktop
  bin suite (decision 12). A failure outside E-1 to E-4 stops for the
  owner. Finish with the four lines.
- [ ] **Commit.**

## Task 4b: Dock Area And Panels (Part C)

- [ ] **Step 1: Design checkpoint, before code.** In the ledger, write
  down:
  - the panel entity shape;
  - the `register_panel` builders;
  - `ShellRenderer`'s delegation to DockSkin;
  - how `PanelHeader` wires drag and drop through `TabGroupContext`;
  - how the Workspace builds the default layout with `DockLayout`.

  Then write one test, `dock_tests::each_panel_draws_its_region`. It
  asserts that the five region ids and `operation-area` are present, and
  that `secondary-1` to `secondary-4` focus their regions. That proves
  U-4. If it cannot pass, stop for the owner.
- [ ] **Tests** (`D/view/dock_tests.rs`):
  - `each_panel_draws_its_region`;
  - `the_default_layout_is_1b` (P1). It reads `DockArea::dump`: the left
    dock holds Repositories over Documents, sized in proportion 0.6 : 1.4
    (U-6); the right dock holds Tickets; the bottom dock Activity; the
    dock sizes are 290, 440 and 150 px.

  The existing `item::sheet_tests::no_workspace_key_acts_behind_the_sheet`
  must pass unchanged with the sheet inside the item area panel (R-29).
- [ ] **Code:**
  - `D/view/dock.rs`, `D/view/dock/panels.rs` and
    `D/view/dock/header.rs` (Part C, R-1, R-2, R-22, R-25, R-27);
  - the Workspace holds the area and the panels;
  - `render.rs`'s fixed row is replaced by the area, with the item area
    as its centre;
  - Root's sheet layer stays in the item area's body (fact 21);
  - `cycle_region` uses `f6_stops`.
- [ ] **Verify:** the two tests and the sheet test by exact name; then
  the whole desktop bin suite
  (decision 12). For each failure:
  - an appearance assertion (a size, bound or position) is deleted under
    E-8 and named in the ledger;
  - anything else stops for the owner.

  Finish with the four lines.
- [ ] **Commit; merge main; ticket comment (dock checkpoint).**

## Task 5: Hiding, Moves, Refusal, Focus (Part C)

- [ ] **Tests** (`D/view/dock_tests.rs`), driven through the Workspace's
  layout operations, which the footer and keys call later:
  - `hiding_a_panel_gives_its_space_to_its_dock` (P2);
  - `hiding_the_last_panel_takes_its_dock_out` (P4);
  - `a_dock_toggled_back_keeps_its_panels_groups_and_sizes` (P5, P30):
    the dumped trees and sizes are equal before and after, and so are
    focus and the item area's tabs;
  - `toggling_a_dock_with_every_panel_off_shows_them` (P6);
  - `a_side_with_no_panels_has_no_dock` (P7);
  - `a_panel_moved_by_dock_panel_lands_last_in_its_new_dock` (P8): into
    an open dock, a dock taken out, and a dock with no panels;
  - `docking_a_panel_where_it_is_changes_nothing` (P9);
  - `a_drop_on_the_item_area_is_undone` (P10): `move_panel` with
    `InsertTarget::Tabs` on the centre's group is the edit a drop commits
    (fact 10), and afterwards the dump equals the accepted state;
  - `hiding_the_focused_panel_focuses_the_item_area` (P12);
  - `a_hidden_panel_keeps_its_state` (P2): Tickets with a filter, Find
    text and a selection is hidden and shown again;
  - `a_request_finishes_while_activity_is_hidden` (P28): its entry shows
    when Activity does, and no tab opened, closed or reloaded.
- [ ] **Code:** R-3, R-5, R-6 and R-9 in `D/view/dock.rs`. Handle
  `DockEvent::LayoutChanged`: check the dump, keep it as the accepted
  state, and read the arrangement from it. Define the `DockPanel` action
  (R-12, U-5).
- [ ] **Verify:** each test by exact name; the four lines.
- [ ] **Commit.**

## Task 6: Title Bar And Window Title (Part D)

- [ ] **Test:** `shell::title_tests::the_window_title_follows_the_selected_repository`.
  Select a repository, select another, then remove it, and read
  `VisualTestContext::window_title()` after each (P22, fact 18).
- [ ] **Code:**
  - `window_options` (`D/mod.rs:76-85`) becomes
    `TitleBar::window_options()` with the bounds and client decorations
    (R-15).
  - The title bar is built in `D/view/shell.rs`: app icon (R-16), title
    text, and `on_close_window` → `request_quit`.
  - `set_window_title` is called when the title changes.
  - Icons (R-23): add `PanelLeft`, `PanelRight`, `PanelBottom`,
    `Settings` and `Activity` to `ItemIcons`. Add the app icon to
    `DesktopAssets` at `brand/manyhands.png`.
- [ ] **Verify:** the test; `… assets`; the four lines.
- [ ] **Commit.**

## Task 7: Footer And Dock Menu By Mouse (Part E)

- [ ] **Tests** (`D/view/footer_tests.rs`):
  - `each_panel_toggle_shows_and_hides_its_panel` (P2, P3). The
    accessible toggled state and the tooltip text follow on screen, as
    R-4 says.
  - `dock_toggles_follow_the_rules` (P4–P7).
  - `a_moved_panel_takes_its_toggle_to_its_new_side` (P8).
  - `a_secondary_click_opens_the_dock_menu_with_its_side_checked`. It
    chooses "Dock right", then the current side (P8, P9).
  - `needs_attention_counts_and_leads_to_activity` (P20): hidden at 0;
    "1 needs attention"; activating it reveals and focuses Activity.
  - `settings_opens_the_settings_page` (P21).
- [ ] **Code:** `D/view/footer.rs`, `D/view/footer/menu.rs` (R-4, R-11,
  R-13, R-24). The workspace root renders the footer below the area.
- [ ] **Verify:** each test by exact name; the four lines.
- [ ] **Commit.**

## Task 8: Keyboard (Part F)

- [ ] **Tests** (`D/view/keyboard_tests/shell.rs`, registered in
  `keyboard_tests.rs`):
  - `dock_keys_toggle_each_dock` (P17);
  - `ctrl_b_in_an_editor_is_bold` (P17, decision 2). It follows D3's
    editor rules and asserts the editor's text gains bold markup and the
    left dock is unchanged.
  - `region_keys_and_find_reveal_hidden_panels` (P13, P15);
  - `f6_skips_hidden_panels` (P14);
  - `the_dock_menu_opens_and_moves_by_keyboard` (P19). It uses `menu`,
    then `shift-f10`, then arrows and Enter; Escape returns focus to the
    toggle.
  - `secondary_n_creates_in_its_own_list` (P16);
  - `title_bar_and_footer_controls_are_reached_by_tab` (P29). Each
    control is reached by Tab with its focus ring, and has its accessible
    name.
- [ ] **Edits:** E-5 and E-6.
- [ ] **Code:**
  - `D/keys.rs` gains `ToggleLeftDock`, `ToggleRightDock`,
    `ToggleBottomDock` and `OpenDockMenu`, with their bindings (Part F,
    R-10);
  - each handler is wrapped in `unless_dialog`;
  - each panel toggle has the "PanelToggle" key context;
  - the reveals go in `focus_region` and `focus_find`.
- [ ] **Verify:** each test by exact name; `keys_tests::each_key_is_bound_to_its_action`;
  `item::sheet_tests::no_workspace_key_acts_behind_the_sheet`; the four
  lines.
- [ ] **Commit.**

## Task 9: Persistence (Part G)

- [ ] **Tests** (`D/view/dock/restore_tests.rs`):
  - `each_layout_file_restores_or_falls_back` is the one table-driven test
    (P24, P25). It has one row for each line of Part G's table, plus a
    valid file with a hidden dock, a moved panel and changed dock and
    group sizes.
    - Each row seeds the fixture's data directory and launches.
    - It asserts the arrangement, the on flags, the shown docks and the
      sizes, and that the notice is present or absent.
    - It checks that every other file in the data directory is
      byte-for-byte unchanged and that the repositories' `git status`
      and refs are unchanged.
    - The notice row also dismisses the notice and checks that the count
      is unchanged.
  - `the_layout_survives_a_restart` (P23, A3, A4, A14). Hide the bottom
    dock, move Tickets left and resize a group. Advance the clock past
    `LAYOUT_SAVE_DELAY`, drop the shell, then open a fresh one over the
    same data directory, as `D/view/item/posting_tests/relaunch.rs`
    does. The file is at `<data>/layout/layout.json`.
- [ ] **Code:**
  - `D/services/layout.rs` `LayoutStore` (R-17), opened in
    `library_services` and carried in `Services`, as `ResolutionStore` is;
  - validation (R-18), restore, save and the notice (R-19, R-21, R-28).
- [ ] **Verify:** the two tests; then the whole desktop bin suite
  (decision 12); the four lines.
- [ ] **Commit; merge main; ticket comment (persistence checkpoint).**

## Task 10: Quit (Part H)

- [ ] **Tests** (`D/view/workspace_tests.rs`, beside the existing quit
  tests):
  - `a_waiting_quit_shows_activity_and_keeps_the_saved_layout` (P26).
    With Activity hidden and a job running, quit. Activity and its dock
    are shown, Quit now is focused, and the file still has Activity
    hidden after the clock advances.
  - `a_quit_back_to_running_keeps_activity_shown` (P27) uses the
    `saving_drafts` failure recipe. Activity stays shown, its Retry is
    focused, and the file saves Activity shown.
- [ ] **Code:** R-20 in `wait_to_quit` and `drafts_flushed`
  (`D/view/workspace/lifecycle.rs`).
- [ ] **Verify:** the two tests and the existing quit tests by name; the
  four lines.
- [ ] **Commit.**

## Task 11: Keyboard Journey And Checklist

- [ ] **Test:** `keyboard_tests::shell::arrange_the_workspace_by_keyboard_alone`.
  With no clicks and no Workspace calls:
  - hide Documents from the footer;
  - move Tickets to the left dock with its dock menu;
  - hide and show the bottom dock with `secondary-j`;
  - focus a hidden Activity with `secondary-4`;
  - restart;
  - find the same layout (A2, A4, A10).
- [ ] **Write** `docs/plans/2026-11-08-wave-04-cycle-01-on-screen-checklist.md`
  (design, Testing), in D5's format, grounded in the app's strings.
- [ ] **Verify:** the test by exact name; the four lines.
- [ ] **Commit.**

## Task 12: Gates, Review-Ready, Closure

- [ ] Merge main. Run:
```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
# AGENTS.md fast tier, exactly as main's AGENTS.md lists it at Task 12
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
git merge-tree --write-tree HEAD origin/main
git diff --stat 2b1dc4d5 HEAD -- src/desktop/view/operations.rs src/desktop/view/operations src/desktop/model/operations.rs
```
  The last command must print nothing.
- [ ] Do a whole-branch spec review and a whole-branch quality review.
  Fill the Cycle's Acceptance Status column. The owner runs the
  checklist, and Claude records only what the owner reports.
- [ ] Ticket comment: review-ready; `ticket save`.
- [ ] After code-review approval, and only with the owner's go-ahead,
  close the ticket from the registered main checkout:
  `manyhands-cli ticket close --repo /home/vhodges/work/src/manyhands --id 01M6V8RK8B0SBCH0YZXFVSR9E3`.
  The close performs the merge, publishes main, and removes the branch
  and worktree. With a passphrase-protected key, the owner runs it at
  their terminal. Native CI runs after merge, on main, only when the
  owner dispatches it.

---

## Plan Self-Review

| Acceptance / outcome / ruling / decision | Task |
| --- | --- |
| A1, P1 | 2, 4b |
| A2, P2–P9 | 2, 5, 7, 8, 11 |
| A3 | 9 (sizes restored); checklist (resizing) |
| A4, P23 | 9, 11 |
| A5, P24, P25 | 9 |
| A6, P22 | 3, 6 |
| A7, P26, P27 | 10 |
| A8 | 4a, 4b, 9 (whole suite), 12 |
| A9, P20 | 3, 7 |
| A10, P13, P14, P17 | 2, 8 |
| A11, P15 | 4a, 8 |
| A12, P18 | 8 (E-6) |
| A13, P29 | 8 |
| A14 | 9 |
| A15 | 1 |
| A16 | 11 (checklist), after merge |
| P10, P12, P28, P30 | 5 |
| P11 | checklist |
| P16, P19 | 8 |
| P21 | 7 |
| R-1, R-2, R-22, R-25–R-27, R-29 | 4b |
| R-3, R-5, R-6, R-9, R-12 | 5 |
| R-4, R-11, R-13, R-24 | 7 |
| R-7, R-8 | 4a |
| R-10 | 8 |
| R-14–R-16, R-23 | 6 |
| R-17–R-19, R-21, R-28 | 9 |
| R-20 | 10 |
| E-1–E-4 | 4a |
| E-5, E-6 | 8 |
| E-7 | 1 |
| E-8 | 4b |
| U-3 | checklist |
| U-4, U-6 | 4b |
| U-5 | 5 |
| Decisions 1, 11 | Scope |
| Decision 2 | 8 |
| Decision 3 | 3, 7 |
| Decision 4 | 4a, 8 |
| Decision 5 | 9 |
| Decision 6 | 2, 5 |
| Decision 7 | 10 |
| Decision 8 | 7, 8 |
| Decision 9 | 2, 4a |
| Decision 10 | 5 |
| Decision 12 | 4a, 4b, 9 |
