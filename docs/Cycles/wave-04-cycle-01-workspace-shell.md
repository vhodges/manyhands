---
title: "Wave 04 Cycle 01: Workspace Shell"
date: 2026-11-08
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6YCHWDYY8ZCFZ62VX0ES1JM"
---

# Wave 04 Cycle 01: Workspace Shell

Ticket `01M6V8RK8B0SBCH0YZXFVSR9E3` (`vh-jge24`). Design (to follow):
`docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-design.md`. Plan (to
follow): `docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-implementation.md`.

Status: **draft, 2026-11-08.** Not reviewed and not approved. Drafted on the
ticket branch at `cc2af8d0`, which merged origin/main `8da9afd4`. The design
and implementation plan follow this document, and all three go through
independent review before the owner is asked to approve them. Approval of
the documents does not authorize implementation, which needs its own
go-ahead.

## Purpose And Authority

Replace the fixed four-region layout with the design's window anatomy: a
title bar that names the selected repository, left, right and bottom docks
around the item area, and a footer that shows, hides and moves the panels.
Today's region contents move into the new frame unchanged. After this Cycle
a person can arrange Repositories, Tickets, Documents and Activity the way
they work, by mouse or keyboard, and find the same arrangement after a
restart.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md): Cycle 01 (lines
   203–291), Owner Direction (65–101), Planning Baseline (103–138), Explicit
   Exclusions (169–183), Testing (478–487), Risks And Controls (505–517) and
   Owner Decisions 1–10 (519–549).
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   the Wave 04 amendment (40–55), tab identity (73–75), navigation that never
   saves or synchronizes (80–84), and accessibility (344–351).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-NFR-005` keyboard core
   workflows, `MH-UX-001` desktop interface, `MH-NFR-002` platforms (Linux
   in a Wayland session), `MH-NFR-004` responsiveness.
5. The design handoff: [`01-information-architecture.md`](../design/01-information-architecture.md)
   (window anatomy), [`02-layout-and-docking.md`](../design/02-layout-and-docking.md)
   (regions, panel header, footer, behavior, persistence, keyboard),
   [`07-design-tokens.md`](../design/07-design-tokens.md) (chrome colors),
   [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md), and the
   frame `screenshots/1b-layout-docked-tabs.png`.
6. Predecessors: Wave 03 D3 ruling R-60 (no workspace key acts behind a
   sheet), D1's operation area and quit path, and D5's execution ledger,
   whose lessons the plan carries as rules.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`cc2af8d0`) |
| --- | --- |
| Wave 04 approved, desktop IA RFC amended | Done 2026-11-07, main `846af679`. |
| Cycle ticket before the plan (`AGENTS.md`) | `01M6V8RK8B0SBCH0YZXFVSR9E3`, open. |
| Latest main merged into the ticket branch | origin/main `8da9afd4` merged as `cc2af8d0`, no conflicts. |
| GPUI Kit 0.6.6 names verified (ticket Planning) | Read from the locked source during planning: `DockArea`, `DockAreaState`, `DockLayout`, `register_panel`, `TitleBar`, `StatusBar`, `PopupMenu`, `Popover`, `h_resizable`. Facts in the design's Baseline Facts. |
| Wave 03 D6 | Not required (ticket, Entry Gate). D6 is drafted, not reviewed and not merged. |
| Required checks at baseline | Not run during planning. Task 0 runs them. |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge (Wave decision 4). |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents named in the design's Document Amendments. It
changes nothing in `src/lib.rs`, `src/repository.rs`, `src/repository/`,
`src/results.rs`, `src/drafts/`, `src/runtime.rs` or the CLI, and adds no
Cargo dependency (`serde` and `serde_json` are already direct dependencies).
It does not edit `src/desktop/view/operations.rs`, `src/desktop/view/operations/`
or `src/desktop/model/operations.rs`: the Activity panel wraps the operation
area as it is (Wave 04 Cycle 01; ticket, D6 Coordination).

**In flight.**

- **Wave 03 D6** (`01M4CC0VN3VPVVQ3RNWJKXWEVX`). Its Cycle document, design
  and plan are drafted (revision 1, 2026-11-07), unreviewed, and exist only
  as uncommitted files in its worktree, which is 87 commits behind main. D6
  adds a polling model whose attention states (key locked, host not
  approved, key missing, configuration required, recovery hold) live in that
  model, with one operation-area entry each; a fifth Settings page; and a
  "still stopping" line in the quit feedback. See decision 3 and Exclusions
  for how the Needs attention count meets it.
- **Ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`** changes only `view/confirm.rs`
  and its tests under `src/desktop/`. No overlap.
- **Follow-up tickets `01M6RAS63EWVHWWVBSZ836RXG2` and
  `01M6H7VXJJHRYHTW9S8TJ88921`** plan to split `model/operations.rs` and
  `view/operations.rs`. This Cycle does not edit either file, so a split that
  lands first changes only the paths the Activity panel calls.

Whichever of these merges second merges main and adapts, per `AGENTS.md`.

## Scope

1. **Chrome tokens.** A workspace token module with `07`'s chrome colors
   (`chrome`, `panel`, `border`, `border-strong`, `hover`, `faint`, `muted`,
   `secondary`, `text`, `selection`, `accent`, `teal`, `green`, `red`) in
   both modes, used by the title bar, panel headers and footer. The item
   view's `accent` and `faint` come from it instead of `view/item/tokens.rs`.
2. **Title bar and window title.** Kit `TitleBar` on every platform (Wave
   decision 4) with the app icon, the title text and the platform window
   controls. The window title is `Manyhands — {repository}` while a
   repository is selected and `Manyhands` when none is, using the name the
   Repositories panel shows. It is set on the platform window and drawn in
   the title bar.
3. **Dock area and item area.** Kit `DockArea` with the item area as its
   center: today's content region with its tab strip, item view, Settings
   and Drafts pages and empty state. Root's sheet layer stays inside the
   center, so a sheet's focus still counts as the item area's and R-60 still
   refuses workspace keys behind it.
4. **Panels**, registered by stable name:
   - `RepositoriesPanel`: today's sidebar.
   - `TicketsPanel`: today's Tickets list with its New ticket… button,
     filters and Why blocked, and Find at its top (decision 4).
   - `DocumentsPanel`: today's Documents tree with New document….
   - `ActivityPanel`: today's operation area, wrapped as it is, with the
     layout notice (decision 5) above it.

   Navigation's Documents | Tickets choice and its "Showing …" line go,
   since each list is now its own panel. Panel headers follow `02`, "Panel
   header": icon, title and count for a single panel, and a tab per panel
   with the amber active line for a tabbed group. Panels cannot be closed or
   zoomed; the footer is how a panel is hidden.
5. **Default layout** (Wave decision 1): left dock 290 px with Repositories
   (0.6) stacked over Documents (1.4); right dock 440 px with Tickets; bottom
   dock 150 px with Activity.
6. **Visibility rules** from `02`: showing a panel shows its dock; when
   every panel in a dock is hidden the dock collapses and the item area
   widens; a dock toggle hides or shows the dock without changing which
   panels are on; a dock toggle on a dock whose panels are all hidden shows
   them all (decision 6).
7. **Footer** (Kit `StatusBar`, 28 px), per `02`, "Footer":
   - the left dock toggle, one toggle per left panel, a separator, the
     bottom dock toggle and one toggle per bottom panel;
   - **Needs attention**: `circle-alert` and "N need attention", red, hidden
     at 0, counting what decision 3 lists; activating it shows and focuses
     the Activity panel until Cycle 03 adds the popover;
   - a separator, one toggle per right panel and the right dock toggle;
   - a separator and **Settings** (`settings`, tooltip "Settings (Ctrl ,)"),
     opening today's Settings page in the item area.

   Panel toggles show on and off states with the tooltips "Show {panel}" and
   "Hide {panel}". A panel moved to another dock takes its toggle to that
   dock's group in the footer.
8. **Dock menu** on every panel toggle, opened by secondary click and by
   the keyboard (decision 8): "Dock left", "Dock right" and "Dock bottom",
   with the current placement checked.
9. **Dragging and resizing.** Docks and stacked groups resize with the
   toolkit's handles. Dragging a panel to another dock or into a tabbed group
   is the toolkit's. A drop onto the item area is refused (decision 10).
10. **Persistence.** The layout (the dock area's state, which panels are
    visible and which docks are shown) is saved to a versioned file in the
    app data directory shortly after each change, and restored at launch.
    Never in a repository. Fallback per decision 5.
11. **Keyboard.**
    - Ctrl B, Ctrl Alt B and Ctrl J toggle the left, right and bottom docks,
      as in Zed. While an editor has focus Ctrl B stays Bold (decision 2).
    - No per-panel toggle keys (Wave decision 3).
    - `secondary-1` focuses Repositories, `secondary-2` whichever of Tickets
      and Documents was used last, `secondary-3` the item area and
      `secondary-4` Activity. Focusing a hidden panel shows it, and its dock.
    - `secondary-p` shows the Tickets panel if it is hidden and focuses Find
      (decision 4).
    - `secondary-n` is New ticket… in Tickets and New document… in
      Documents.
    - F6 and Shift F6 cycle the visible panels and the item area only, in
      the order of decision 9.
    - Every title-bar and footer control is reachable by Tab, has an
      accessible name and shows focus.
12. **Quit feedback.** A quit that has to wait shows the Activity panel and
    its dock, so "Finishing …" and Quit now stay visible and focused
    (decision 7).

## Explicit Exclusions And Downstream Obligations

- New panel contents, toolbars and row rendering, including the design's
  28 px rows (decision 11): Cycles 02 and 03.
- The Needs attention popover, and the footer's "to share" and update-check
  items: Cycle 03.
- The Settings sheet. Settings stays a page in the item area, and the
  sidebar's Settings entry stays, until Cycle 04 moves and removes them.
- Saving open tabs, the focused pane or the scope with the layout (`02`,
  "Persistence"). The Wave's Cycle 01 persists the dock layout and the
  visible panels only. Not scheduled; recorded as a Document Amendment
  candidate for the Wave's Deferred Work.
- A dock menu for the item area, item-area splits and the focused-pane
  treatment (Wave Deferred Work).
- Search, the command palette, the Projects and Teams panels, the scope
  switcher, the identity avatar and title-bar menus (Wave exclusions).
- A theme switch and light-mode polish. The tokens carry both modes.
- **D6's attention states in the Needs attention count.** They live in D6's
  polling model, not in the operation area this Cycle counts. Whichever of
  D6 and this Cycle merges second adds them to the count (decision 3).
- Native evidence on macOS and Windows: native CI after merge (Wave
  decision 4).

## Required Contract

1. **Contents unchanged.** Every panel and the item area keep their
   current behavior, actions and services. No domain behavior changes, and
   the operation area's files are not edited.
2. **Hiding loses nothing.** Hiding a panel or a dock never cancels a
   request, discards a draft or dismisses an operation entry. An unresolved
   failure stays in Activity and in the Needs attention count while Activity
   is hidden, and a quit that has to wait is never hidden.
3. **Layout is app-local and never blocks.** The layout file lives only in
   the app data directory. A missing, unreadable or incompatible file never
   stops startup and never touches other app data (drafts, kept resolution
   sets).
4. **R-60 holds.** No workspace key acts behind a sheet or dialog, including
   the dock toggles and the region keys that now show panels.
5. **Keyboard.** Every panel is reachable by key even when hidden, every
   footer and title-bar control by Tab, and the dock menu without a
   pointer. Focus stays visible.
6. **Color is never the only signal.** Toggle states are also given by
   their accessible state and tooltip, and the Needs attention count by its
   text.
7. **Item identity unchanged.** Tabs stay keyed by repository, kind and
   ULID, and moving panels never opens, closes or reloads an item.

## Acceptance And Exit Evidence

Planned proof. Nothing has been run.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Starts in the default layout | Model test of the default; desktop test that a first launch shows it | Planned |
| A2 | Each panel hidden, shown and moved to every offered dock from the footer, by mouse and by keyboard | Model tests of the visibility rules and moves (table-driven); desktop tests through the footer toggles and dock menu by click and by key | Planned |
| A3 | Docks and groups resize | Restore test keeps changed dock and group sizes; resizing itself on the on-screen checklist | Planned |
| A4 | Layout survives a restart, including a hidden dock and a moved panel | Desktop test: change, save, open a fresh workspace over the same data directory | Planned |
| A5 | A corrupt or old layout file restores the default without losing anything else | One table-driven test: missing, unreadable, malformed, other version, unknown panel, missing panel; other files in the data directory untouched; notice shown only where decision 5 says | Planned |
| A6 | The window title follows the selected repository | Model test of title derivation; desktop test that selecting a repository sets it | Planned |
| A7 | A quit that has to wait shows its feedback with Activity hidden | Desktop test (contract 2, decision 7) | Planned |
| A8 | Every existing keyboard journey still passes | Existing keyboard tests, with the named edits the design lists | Planned |
| A9 | Needs attention counts what decision 3 lists and leads to Activity | Model test of the count; desktop test of the footer item by key | Planned |
| A10 | Dock keys, region keys and F6 | Key-binding table test; desktop tests that `secondary-1`–`4` show a hidden panel, F6 skips hidden ones, and Ctrl B in an editor is Bold | Planned |
| A11 | Find in the Tickets panel | Existing Find tests, edited: `secondary-p` shows a hidden Tickets panel and focuses Find | Planned |
| A12 | R-60 holds for the new keys (contract 4) | `no_workspace_key_acts_behind_the_sheet` extended with the dock toggles | Planned |
| A13 | Title-bar and footer controls reachable by Tab with accessible names (contracts 5, 6) | Desktop keyboard test | Planned |
| A14 | Layout never written into a repository (contract 3) | Persistence test asserts the file's location and that no repository's tree changed | Planned |
| A15 | Item view reads `accent` and `faint` from the chrome tokens, both modes | Existing token test, edited | Planned |
| A16 | Native title bar | Linux Wayland on the on-screen checklist by the owner; macOS and Windows in native CI after merge (Wave decision 4) | Planned |

The on-screen checklist follows D5's format, run by the owner on a release
build under Wayland. Claude records only what the owner reports.

## Product-Owner Decisions (2026-11-08)

Wave 04 decisions 1–10 (2026-11-07) govern and are not repeated.

Answered by the owner during planning:

1. **One Cycle.** The Wave's split line (01a / 01b) does not apply.
2. **Ctrl B stays Bold in the editor.** While an editor has focus Ctrl B is
   Bold, as today. Everywhere else it toggles the left dock. From an editor,
   Esc first or the footer's left dock toggle.
3. **Needs attention counts everything awaiting the user.** Until Cycle 03:
   requests that finished blocked, partial or with an error and are not
   dismissed; draft save failures; drafts unavailable; and the unfinished
   operations listed under "Pending in repositories". A new read-only
   function counts them from the existing models, so `model/operations.rs`
   is not edited. D6's attention states join the count in whichever of D6
   and this Cycle merges second.
4. **Find moves to the top of the Tickets panel.** It still finds tickets
   and documents. `secondary-p` shows the Tickets panel if it is hidden and
   focuses Find.

Proposed by the planner, to be confirmed with the documents' approval:

5. **Fallback reporting.** A missing layout file (first launch) starts in
   the default layout without a notice. An unreadable or malformed file, one
   of another version, or one whose panels are not exactly the four known
   ones restores the default and shows one dismissible notice at the top of
   the Activity panel: "Your panel layout couldn't be restored, so the
   default layout is back." The notice is not counted in Needs attention.
   This departs from the Wave's wording, which also reports a missing file
   (line 262).
6. **A dock toggle on a dock whose panels are all hidden** shows every
   panel in that dock, so the toggle always does something visible. A dock
   with no panels at all has no toggle.
7. **A quit that has to wait shows Activity.** The Activity panel and its
   dock are shown when a quit starts waiting, and this is not saved into the
   layout.
8. **The dock menu by keyboard** opens with the context-menu key and with
   Shift F10 on a focused panel toggle, since the Kit's context menu opens
   on right click only.
9. **F6 order:** the left dock's visible panels top to bottom, the item
   area, the right dock's, then the bottom dock's, left to right. In a
   tabbed group only the active tab is a stop.
10. **A panel dropped on the item area** is refused, and the panel stays
    where it was.
11. **Rows.** The Wave's "Rows are 28 px" (Cycle 01, line 237) is read as
    the default for the row anatomy Cycle 02 builds. Cycle 01 leaves rows
    as they are, since row rendering is out of its scope (line 272).

## Open Questions

None for the owner beyond confirming proposals 5–11. Three facts are
unverified and are settled in Task 0 by reading or trying the source; an
answer that changes a proposal stops for the owner:

- U-1: whether the GPUI test platform records a window title the tests can
  read (A6).
- U-2: whether the dock area can refuse a drop onto the center, or the drop
  has to be undone on the Kit's `DragDrop` event (proposal 10).
- U-3: whether the keyboard's context-menu key reaches GPUI as `menu` under
  Wayland (proposal 8). Shift F10 is the fallback; the native check
  confirms.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | Does the plan fit one reviewable Cycle? | Owner decision | Decision 1. | Wave 288–291; ticket Planning. |
| Planning research | Ctrl B is Bold in zorite's `Editor` context | Owner decision | Decision 2. | `zorite-editor-0.10.0/src/lib.rs:137-138`. |
| Planning research | No "attention entries" exist in `model/operations.rs` | Owner decision | Decision 3. | Wave 244, 514; `services/requests.rs:782-793`. |
| Planning research | Find sits above both lists in the Navigation pane | Owner decision | Decision 4. | `view/lists.rs:39`; `view/find.rs:44`. |
| Planning research | D6's attention states live in its own polling model; its documents are uncommitted | Settled | Exclusions and decision 3: the second to merge wires them in. | D6 design (uncommitted), Phase and Reason; D6 worktree `git status`. |
| Planning research | The Kit hides a panel only by `visible()`, and a dock with every panel hidden keeps its width | Ruling | The layout model closes a dock with no visible panel (design). | gpui-base `dock/panel.rs:25-30`, `dock/dock_area.rs:1325-1341`. |
| Planning research | The Kit's panel header always draws a menu with Zoom and Close | Ruling | Own header renderer per `02`; panels not closable or zoomable (design). | gpui-component `dock/tab_panel.rs:326-338`. |
| Planning research | `DockArea::load` ignores the version and keeps unknown panels as placeholders | Ruling | The layout file is validated before loading (design). | gpui-base `dock/dock_area.rs:716-722`; gpui-component `dock/dock.rs:80-86`. |
| Planning research | The Kit's context menu opens on right click only | Proposal | Decision 8. | gpui-component `menu/context_menu.rs:294-300`. |
| Planning research | The Wave reports a missing layout file | Proposal | Decision 5. | Wave 259–263. |
| Planning research | The Wave puts 28 px rows in Cycle 01 and row rendering out of it | Proposal | Decision 11. | Wave 235–237, 272–273. |
