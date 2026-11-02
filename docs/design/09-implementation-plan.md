# 09 · Suggested implementation plan

Follow `AGENTS.md`'s dogfooding workflow. Each Cycle below gets a Manyhands ticket before its plan, works in the ticket's worktree, records checkpoints as ticket comments, and passes the Devenv gates (`check`, `fmt`, `clippy`, relevant tests). Keep each Cycle reviewable on its own.

## Cycle 0: Decisions (no code)

- Amend `docs/RFC/desktop-information-architecture-and-editor.md`. It currently rules out a configurable docking system and describes a fixed layout. Add: the dockable panels, the single-scope model, the footer toggles, Projects and Teams as filters (explicitly not rollups), Activity + Needs attention as the persistent operation area, and the plain-language vocabulary (`06`). Get product-owner approval.
- Decide the default layout (recommendation: 1b) and the default row density.
- Check the Dock API at the locked `gpui-kit` version. If it's needed, plan the upgrade as Cycle 1a.
- Confirm the keyboard bindings for panel and dock toggles.

## Cycle 1: Theme + shell skeleton

- Custom dark and light themes from `07`. Theme switch in Settings › Appearance (Dark / Light / Match system).
- `TitleBar` with the menu, a scope switcher placeholder, search (non-functional) and window controls.
- `StatusBar` footer with the sync status items (static wiring to existing state is fine).
- `DockArea` with empty placeholder panels registered by name; layout persistence and the 1b fallback.
- **Accept:** the app starts with layout 1b; docks resize; the layout survives a restart; the theme toggles live; the keyboard reaches every footer control.

## Cycle 2: Scope model + Repositories, Projects, Teams panels

- `Scope { Repository(id) | Project(name) | Team(name) }` in desktop model state, persisted.
- Library: read-only index queries for distinct project and team values with open counts and repo counts.
- The three panels, with their row specs, sharing-state labels (`03`) and tooltips.
- **Accept:** selecting a row changes the scope everywhere; scope persists; the four list states render; keyboard navigation works.

## Cycle 3: Tickets and Documents panels

- Move the existing list and tree views into panels; filter by scope.
- Tickets: Open/Closed/All, filter input, sortable columns, status pills, editing and conflict markers, nonconforming rows, footnote.
- Documents: the tree with repo grouping for multi-repo scopes, editing and nonconforming markers, collapse all.
- **Accept:** PRD MH-CONTENT-001/003/004 list behaviors still pass; nonconforming items stay visible; Open is the default filter.

## Cycle 4: Footer toggles + tabbed groups

- Per-panel and per-dock toggles with the on/off states; toggling a panel on reveals its dock; empty docks collapse.
- Drag panels between docks and into tabbed groups (toolkit behavior); footer buttons follow their panel's dock.
- **Accept:** layouts 1a, 1b and 1c can all be built by the user and are restored after relaunch.

## Cycle 5: Item area

- Tab groups keyed by item key; split right (max 3 panes); focused-pane treatment; unsaved dot with an accessible label.
- Item header: breadcrumb, title, chips, state pill with Git tooltip, Save / Share / primary lifecycle button, banner slot. The editor body stays a placeholder or the current body.
- **Accept:** re-opening focuses the existing tab; viewing never provisions a worktree; the banners appear for conflict, missing metadata and unmanaged documents.

## Cycle 6: Activity + Needs attention

- Move the operations area into `ActivityPanel` (bottom dock). Rows with actions and Git tooltips; no secrets in rows or tooltips.
- Footer counts ("N to share", "N need attention") from the same state; the popover lists unresolved recovery items with one action each.
- **Accept:** failures persist until resolved; every Attention action works by keyboard; Activity can be hidden and the attention popover still works.

## Cycle 7: Sheets and dialogs

- Add repository sheet (existing + create, live checks, main branch, remote, identity step).
- Settings sheet: Sharing & keys, Checking for updates, Appearance. Fold the existing settings pages in.
- Close ticket / Approve and merge dialog: confirm (full effect summary incl. code files and newly-ready tickets) → running (steps, safe-point stop) → failed (retry reuses the operation) → done.
- Unlock key dialog.
- Resolve conflict sheet for canonical paths; the external-recovery card for others.
- **Accept:** the PRD end-to-end journeys (offline edit then later sync; conflict recovery; confirmed promotion; confirmed close with retryable failure) run through the new UI. `MH-NFR-005` keyboard-only runs pass.

## Open decisions to raise with the owner

- Default layout and row density.
- Fonts (IBM Plex vs toolkit default).
- Ready/Blocked filters in the Tickets panel (from the ticket relationships RFC): segments vs a menu.
- Whether Projects and Teams should also offer "Unassigned".
- The command palette (Ctrl K) design.
- First-run with no repositories.
- The ticket and document editor round (zorite-editor), comments and discussion.
