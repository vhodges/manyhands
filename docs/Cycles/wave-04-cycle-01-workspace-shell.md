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

Ticket `01M6V8RK8B0SBCH0YZXFVSR9E3` (`vh-jge24`).
[Design](../plans/2026-11-08-wave-04-cycle-01-workspace-shell-design.md) and
[implementation plan](../plans/2026-11-08-wave-04-cycle-01-workspace-shell-implementation.md).

Status: **draft revision 5, 2026-11-08.** Not approved.

- This document was drafted on the ticket branch at `cc2af8d0`, which
  merged origin/main `8da9afd4`. The design and plan were drafted at
  `2b1dc4d5`.
- All three were reviewed independently at `bb9b3cc9` (round 1),
  `fbca53f4` (round 2) and `32b82b9c` (round 3), each with two reviewers,
  and at `d035d90b` (round 4, a scoped re-review). They were revised after
  each round (Review Record).
- Revision 5's fixes after round 4 have had no independent review.

Approval of the documents does not authorize implementation, which needs
its own go-ahead.

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
| GPUI Kit 0.6.6 names verified (ticket Planning) | Read from the locked source during planning: `DockArea`, `DockAreaState`, `DockState`, `DockLayout`, `DockSkin`, `DockAreaRenderer`, `TabGroupRenderer`, `register_panel`, `TitleBar`, `StatusBar`, `PopupMenu`. The handoff's `DockItem`, `StackPanel` and `TabPanel` types are not in 0.6.6 (design, fact 1). Facts 1–19 in the design. |
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
   - `DocumentsPanel`: today's Documents tree with New document… and New
     folder….
   - `ActivityPanel`: today's operation area, wrapped as it is, with the
     layout notice (decision 5) above it.

   Navigation's Documents | Tickets choice and its "Showing …" line go,
   since each list is now its own panel. Panel headers follow `02`, "Panel
   header": icon, title and count for a single panel, and a tab per panel
   with the amber active line for a tabbed group. Panels cannot be closed or
   zoomed; the footer is how a panel is hidden.
5. **Default layout** (Wave decision 1, decision 15): left dock 290 px
   with Repositories (0.6) stacked over Documents (1.4); right dock 440 px
   with Tickets; bottom dock 150 px with Activity, whose contents scroll.
6. **Visibility rules** from `02`:
   - Showing a panel shows its dock, and selects its tab.
   - When every panel in a dock is hidden, the dock collapses and the item
     area widens. A hidden bottom dock keeps a 29 px strip naming its
     panels (decision 16).
   - A dock toggle hides or shows the dock without changing which panels
     are on.
   - A dock toggle on a dock whose panels are all hidden shows them all
     (decision 6).
7. **Footer** (Kit `StatusBar`, 28 px), per `02`, "Footer":
   - the left dock toggle, one toggle per left panel, a separator, the
     bottom dock toggle and one toggle per bottom panel;
   - **Needs attention**: `circle-alert` and "N need attention", red,
     hidden at 0, counting what decisions 3 and 14 list. Until Cycle 03
     adds the popover, activating it by click or key shows and focuses the
     Activity panel;
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
10. **Persistence.** The layout is saved to a versioned file in the app
    data directory shortly after each change, and restored at launch. It
    holds the dock area's state, which panels are on, and which docks are
    shown. It is never written into a repository.
    - Fallback per decision 5.
    - A failed save shows a notice (decision 13).
11. **Keyboard.**
    - Ctrl B, Ctrl Alt B and Ctrl J toggle the left, right and bottom docks,
      as in Zed. While an editor has focus Ctrl B stays Bold (decision 2).
    - No per-panel toggle keys (Wave decision 3).
    - `secondary-1` focuses Repositories, `secondary-2` whichever of Tickets
      and Documents was used last, `secondary-3` the item area and
      `secondary-4` Activity. Focusing a hidden panel shows it, its dock
      and its tab.
    - `secondary-p` shows the Tickets panel if it is hidden and focuses Find
      (decision 4).
    - `secondary-n` is New ticket… in Tickets and New document… in
      Documents.
    - F6 and Shift F6 cycle the displayed panels and the item area only, in
      the order of decision 9.
    - Every footer control is reachable by Tab, has an accessible name and
      shows focus. The title bar's window controls exist only with client
      decorations, so they are checked natively.
12. **Quit feedback.** A quit that has to wait changes no layout. Its
    entries ("Finishing …", Quit now) appear in Activity as today. If
    Activity is hidden, the quit simply waits, and the person can open
    Activity to see why (decision 7).

## Explicit Exclusions And Downstream Obligations

- New panel contents, toolbars and row rendering, including the design's
  28 px rows (decision 11): Cycles 02 and 03.
  - Panel toolbars go into this Cycle's panel header.
  - Cycle 03's rebuilt Activity keeps the layout notices above its rows.
- The Needs attention popover, and the footer's "to share" and
  update-check items: Cycle 03. The popover reads the same count
  function.
- The Settings sheet. Settings stays a page in the item area, and the
  sidebar's Settings entry stays, until Cycle 04 moves and removes them.
- Saving open tabs, the focused pane or the scope with the layout (`02`,
  "Persistence"). The Wave's Cycle 01 persists the dock layout and the
  visible panels only. This is not scheduled; design Document Amendment 4
  adds it to the Wave's Deferred Work.
- A dock menu for the item area, item-area splits and the focused-pane
  treatment (Wave Deferred Work).
- Search, the command palette, the Projects and Teams panels, scopes and
  title-bar menus (Wave Explicit Exclusions). The title bar's identity
  avatar (`01`) is not planned in Wave 04.
- A theme switch and light-mode polish. The tokens carry both modes.
- **D6's attention states in the Needs attention count.** They live in D6's
  polling model, not in the operation area this Cycle counts. Whichever of
  D6 and this Cycle merges second adds them to the count (decision 3).
- Native evidence on macOS and Windows: native CI after merge (Wave
  decision 4).
- **Later work, from the owner (design Document Amendment 4):**
  - a footer line showing the latest Activity status, such as
    "Saving…" or "Syncing…";
  - quitting with work at risk: one blocking dialog that lists the
    unsaved or unsaveable work and offers going back or discarding it and
    quitting, with save failures handled per item and Activity's wording
    revisited. It replaces Quit now and Quit anyway;
  - a watchdog for pending operations: keep waiting, or abandon the job
    as far as the request can safely be stopped.

## Required Contract

1. **Contents unchanged.** Every panel and the item area keep their
   current behavior, actions and services, except as Scope 4 and decision
   4 change the Navigation pane. No domain behavior changes, and the
   operation area's files are not edited.
2. **Hiding loses nothing.** Hiding a panel or a dock never cancels a
   request, discards a draft or dismisses an operation entry. An
   unresolved failure stays in Activity and in the Needs attention count
   while Activity is hidden. Hiding Activity never stops or cancels a
   quit; its entries stay in Activity.
3. **Layout is app-local and never blocks.** The layout file lives only in
   the app data directory. A missing, unreadable or incompatible file never
   stops startup and never touches other app data (drafts, kept resolution
   sets).
4. **R-60 holds.** No workspace key or footer control acts behind a sheet
   or dialog. That includes the dock toggles and the region keys that now
   show panels.
5. **Keyboard.** Every panel is reachable by key even when hidden, every
   footer and title-bar control by Tab, and the dock menu without a
   pointer. Focus stays visible.
6. **Color is never the only signal.** Toggle states are also given by
   their accessible state and tooltip, and the Needs attention count by its
   text.
7. **Item identity unchanged, and navigation sends nothing.** Tabs stay
   keyed by repository, kind and ULID. Showing, hiding, moving or focusing
   a panel never opens, closes or reloads an item, and never saves,
   synchronizes or promotes anything (RFC 80-84).

## Acceptance And Exit Evidence

Planned proof. Nothing has been run. "P-n" are the design's Required
Outcomes.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Starts in the default layout | Model test of the default; desktop test of a first launch (P1) | Planned |
| A2 | Each panel hidden, shown and moved to every offered dock from the footer, by mouse and by keyboard | Model table of the operations; a desktop table of each panel to each side; footer by click; a toggle by Tab and Enter; moves to each side by the menu keys (P2–P9, P19) | Planned |
| A3 | Docks and groups resize | Hide-and-show and restore keep dock and group sizes (P5, P23); resizing itself on the on-screen checklist | Planned |
| A4 | Layout survives a restart, including a hidden dock and a moved panel | Relaunch over the same data directory, after the save delay and after an immediate quit (P23, P31) | Planned |
| A5 | A corrupt or old layout file restores the default without losing anything else | One table-driven test of every case; a desktop test that the notice shows and the other stores are untouched (P24, P25) | Planned |
| A6 | The window title follows the selected repository | Title derivation; the window's title through selection changes (P22) | Planned |
| A7 | A quit that has to wait, with Activity hidden (decision 7 revised; design Amendment 11) | Desktop tests: the layout is unchanged and the quit's entries are in Activity when opened; drafts not saved focuses the item area (P26, P27) | Planned |
| A8 | Every existing keyboard journey still passes | Existing keyboard tests with the design's named edits; the fast tier at review-ready | Planned |
| A9 | Needs attention counts what decisions 3 and 14 list and leads to Activity | Model test of the count; the footer item by click and by Enter (P20) | Planned |
| A10 | Dock keys, region keys and F6 | Key-binding table; region keys reveal, F6 skips, Ctrl B in an editor is Bold (P13, P14, P17) | Planned |
| A11 | Find in the Tickets panel | Find's existing tests with E-1; a new test that `secondary-p` reveals Tickets (P15) | Planned |
| A12 | R-60 holds for the new keys and the footer (contract 4) | `no_workspace_key_acts_behind_the_sheet` extended, with panels hidden (E-6, P18) | Planned |
| A13 | Footer controls reachable by Tab with accessible names and states (contracts 5, 6) | Desktop keyboard test reading names and toggled states (P29); title-bar controls and a screen reader on the checklist | Planned |
| A14 | Layout never written into a repository (contract 3) | Restart test checks the file's location; restore test checks repositories unchanged (P23, P24) | Planned |
| A15 | Item view reads `accent` and `faint` from the chrome tokens, both modes | Existing token test, extended (E-7) | Planned |
| A16 | Native title bar | Linux Wayland on the on-screen checklist by the owner; macOS and Windows in native CI after merge (Wave decision 4) | Planned |
| A17 | Contents unchanged; hiding loses nothing (contracts 1, 2) | A hidden panel keeps its state; a request finishes while Activity is hidden; a dock put back keeps focus (P2, P28, P30); the operation files' diff is empty | Planned |
| A18 | Navigation sends nothing (contract 7) | The keyboard journey and P28 assert the worker received no request | Planned |
| A19 | A failed layout save is shown (decision 13) | Desktop test (P32) | Planned |
| A20 | A drag into a tabbed group is kept like any other change | Desktop test through the area's move and a relaunch (P11) | Planned |

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

Proposed by the planner and confirmed by the owner:

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
7. **A quit that has to wait changes no layout** (revised by the owner
   after review round 3; first proposed as "shows Activity, not saved").
   Its entries are in Activity as today, visible if Activity is. Quit now
   and Retry take focus only when Activity is displayed.
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

The owner confirmed 5–11 on 2026-11-08 ("Proposals looks fine"), before
the design and plan were written.

Answered by the owner after review round 1:

12. **Test runs follow AGENTS.md as is.** Between tasks, only the tests a
    task wrote or changed run, by exact name. The fast tier runs once, at
    review-ready. There are no extra whole-suite runs, and no standing
    approval for unlisted test edits, which stop for the owner.
13. **A failed layout save is shown.** One dismissible notice in Activity,
    "Your panel layout couldn't be saved.", in the restore notice's place.
    It is not counted in Needs attention, and a later successful save
    removes it.
14. **A failed pending read counts.** Each repository whose "Pending in
    repositories" list failed to load adds one to Needs attention, beside
    decision 3's sources.
15. **Activity keeps the Wave's 150 px.** The default height matters
    little, and the panel's contents scroll, so content below the fold is
    acceptable. Nothing scrolls a focused control into view in this Cycle.
    Tests that need a control low in Activity open it taller.

Answered by the owner after review round 2:

16. **A hidden bottom dock keeps the Kit's strip, for now.** Docks open
    and close through the Kit itself. The Kit cannot close the bottom dock
    completely: it keeps a 29 px strip under the item area, which names
    the dock's panels; activating one shows it. Left and right docks close
    completely. Hiding the bottom dock completely is to be revisited later
    (design Document Amendment 4 adds it to the Wave's Deferred Work).

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- Docks and groups resize by mouse only (K-1).
- Tabbed groups are made by dragging only; the dock menu makes a new group
  (K-12). A dock's only panel is moved by its menu, not by dragging (K-11).
- A refused drop on the item area moves focus to the item area (K-2).
- A compositor that refuses client decorations draws its own title bar too
  (K-4).
- Open tabs, the focused pane and the scope are not saved with the layout
  (K-6).
- A window resize alone is not saved (K-9).
- A focused control low in Activity may need scrolling (K-13,
  decision 15).
- A hidden bottom dock keeps a 29 px strip (K-15, decision 16).
- With Activity hidden, a quit that has to wait shows nothing until
  Activity is opened (K-18, decision 7).
- A quit's layout write runs on the UI thread, so a hung file system
  delays the quit (K-14).

## Open Questions

None for the owner. U-1 and U-2 were settled by reading the source:

- **U-1, settled.** The test window records the title, and
  `VisualTestContext::window_title()` reads it (design, fact 18).
- **U-2, settled.** The Kit has no drop predicate, so a drop onto the item
  area is undone after the fact (design, fact 10 and R-6).

One fact is still unverified:

- **U-3.** Whether the keyboard's context-menu key reaches GPUI as `menu`
  under Wayland (decision 8). Shift F10 is the fallback, and the native
  checklist confirms it.

The design's U-4 and U-6 are checked in the tasks that first rely on
them; U-5 was settled in review.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | Does the plan fit one reviewable Cycle? | Owner decision | Decision 1. | Wave 288–291; ticket Planning. |
| Planning research | Ctrl B is Bold in zorite's `Editor` context | Owner decision | Decision 2. | `zorite-editor-0.10.0/src/lib.rs:137-138`. |
| Planning research | No "attention entries" exist in `model/operations.rs` | Owner decision | Decision 3. | Wave 244, 514; `services/requests.rs:782-793`. |
| Planning research | Find sits above both lists in the Navigation pane | Owner decision | Decision 4. | `view/lists.rs:39`; `view/find.rs:44`. |
| Planning research | D6's attention states live in its own polling model; its documents are uncommitted | Settled | Exclusions and decision 3: the second to merge wires them in. | D6 design (uncommitted), Phase and Reason; D6 worktree `git status`. |
| Planning research | The Kit hides a panel only by `visible()`; a dock with every panel hidden keeps its width; a closed bottom dock keeps a 29 px strip | Ruling, then owner decision | Design R-3: a dock with nothing on closes through the Kit; the bottom keeps its strip (decision 16). | gpui-base `dock/panel.rs:21-93`, `dock/dock_area.rs:1325-1354`, `:1702-1715`. |
| Planning research | The Kit's panel header always draws a menu with Zoom and Close | Ruling | Design R-2: own header renderer per `02`; panels not closable or zoomable. | gpui-component `dock/tab_panel.rs:269-351`. |
| Planning research | `DockArea::load` ignores the version and keeps unknown panels as placeholders | Ruling | Design R-18, R-28: the file is validated before loading. | gpui-base `dock/dock_area.rs:716-760`; gpui-component `dock/dock.rs:83-93`. |
| Planning research | The Kit's context menu opens on right click only | Owner decision (proposed, confirmed 2026-11-08) | Decision 8; design R-11. | gpui-component `menu/context_menu.rs:296-300`. |
| Planning research | The Wave reports a missing layout file, and its Risks row drops unknown panels | Owner decision (proposed, confirmed 2026-11-08) | Decision 5; design Document Amendment 1. | Wave 259–263; Risks And Controls. |
| Planning research | The Wave puts 28 px rows in Cycle 01 and row rendering out of it | Owner decision (proposed, confirmed 2026-11-08) | Decision 11; design Document Amendment 2. | Wave 235–237, 272–273. |
| Planning research | The test window records the title (U-1); no drop predicate exists (U-2) | Settled | Open Questions. | Design facts 10, 18. |
| Review round 1 (two reviewers, `bb9b3cc9`) | Source and feasibility: 3 Blocking, 14 Important, 16 Minor. Contract and consistency: 2 Blocking, 17 Important, 11 Minor | — | Each verified against the source or authority and applied in revision 2; detail in the design's Review Record | Reviewer reports, 2026-11-08 |
| Review round 1 | Whole-suite runs beyond AGENTS.md (proposed decision 12) | Owner decision | Decision 12: AGENTS.md as is | Owner, 2026-11-08 |
| Review round 1 | A failed layout save was never shown | Owner decision | Decision 13 | RFC 52-53 |
| Review round 1 | A failed pending read counted nothing | Owner decision | Decision 14 | Decision 3 |
| Review round 1 | Activity at 150 px with no focus reveal | Owner decision | Decision 15 | Design fact 26 |
| Review round 2 (scoped re-review and adversarial review, `fbca53f4`) | Re-review: 1 Blocking, 9 Important, 10 Minor. Adversarial: 4 Blocking, 9 Important, 5 Minor | — | Each verified against the source and applied in revision 3; detail in the design's Review Record | Reviewer reports, 2026-11-08 |
| Review round 2 | Taking hidden docks out of the area and rebuilding them was the source of all five Blocking findings | Owner decision | Decision 16: the Kit's own dock closing, with its bottom strip for now | Design Review Record |
| Review round 3 (scoped re-review and adversarial, `32b82b9c`) | Re-review: 7 Important, 8 Minor. Adversarial: 1 Blocking, 6 Important, 8 Minor. The Kit-closing mechanism held | — | Each verified against the source and applied in revision 4; detail in the design's Review Record | Reviewer reports, 2026-11-08 |
| Review round 3 | The quit override drew findings in every round, including round 3's Blocking (a tab click during a quit wait hid Quit now) | Owner decision | Decision 7 revised: a waiting quit changes no layout; the override is gone | Owner, 2026-11-08 |
| Owner, 2026-11-08 | A footer status line; no Quit now or Quit anyway buttons | Owner direction | Later work (Exclusions; design Amendment 4) | Owner, 2026-11-08 |
| Review round 4 (scoped re-review, `d035d90b`) | 4 Important, 10 Minor, none Blocking | — | Verified and applied in revision 5, which had no independent review | Reviewer report, 2026-11-08 |
| Owner, 2026-11-08 | Quitting with work at risk as one blocking dialog; a watchdog for pending operations | Owner direction | Later work (Exclusions; design Amendment 4) | Owner, 2026-11-08 |
