---
id: 01M6V05RX2GZ4FFW8VMKTWFBTN
title: 'Wave 04: Workspace Design'
date: 2026-11-07
status: draft
author: "Vince Hodges <vince@imbas.ca> && Claude"
manyhands_managed: true
manyhands_kind: document
---
# Wave 04: Workspace Design

## Outcome

Wave 04 rebuilds the desktop workspace around the design handoff in
[`docs/design/`](../design/README.md). At its exit the fixed four-region layout
is gone. The window has a title bar that names what you're looking at.
Repositories, Tickets, Documents and Activity are dockable, resizable panels
that the user shows, hides and moves from a footer bar that also carries
Needs attention and Settings, and the layout survives a restart. Each panel's
rows, toolbars and states follow `03-panels.md` and the plain-language vocabulary in
`06-language-and-states.md`. Settings opens as a sheet instead of taking over
the item area, and the existing sheets and dialogs follow the design too.

Each Cycle is small and leaves the app visibly better than before: the shell
first, then the two lists people use most, then the sharing and recovery
surfaces, then Settings, then the remaining sheets and dialogs. No Cycle
changes domain behavior. This Wave is a
view-layer reorganization over the existing headless services, plus a desktop
layout model and its app-local persistence.

The item view (the center pane's header, page, editor, toolbar, details sheet
and inline discussion from `docs/design/item_view/`) is largely in place from
Wave 03 and is not reworked here, apart from hosting it inside the new center
area.

## Authority And Traceability

This Wave implements the workspace design handoff, `docs/design/01` to `08`,
under [PRD v0.6](../PRD/mvp.md) and the
[desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md),
as amended below. `docs/design/09-implementation-plan.md` was the designer's
suggested sequence. This Wave replaces it with the owner's priorities from
2026-11-07, and where the two disagree this Wave governs.

| Design section | Owning Cycle |
| --- | --- |
| `01` window anatomy: title bar, dock area, footer | Cycle 01 |
| `01` scope model, Repository scope only | Cycle 01 (title), Cycle 02 (lists) |
| `02` regions, docks, groups, panel header, footer toggles, persistence, keyboard | Cycle 01 |
| `03` Tickets, Documents | Cycle 02 |
| `03` Repositories, Activity | Cycle 03 |
| `05` Needs attention popover | Cycle 03 |
| `05` Settings sheet frame, Sharing & keys | Cycle 04 |
| `05` common sheet and dialog frame, Add repository, Unlock key, Close ticket and Approve and merge, Resolve conflict | Cycle 05 |
| `04` empty item area | Cycle 05 |
| `06` copy and state vocabulary | every Cycle, for the surfaces it touches |
| `07` color, type, spacing tokens (both modes) | Cycle 01 (chrome tokens), then each Cycle's surfaces |
| `08` GPUI Kit mapping | every Cycle, verified against the locked version |

Every requirement the existing desktop meets still holds: failures never
exist only in transient notifications, every action works by keyboard, color
is never the only signal, tabs key on repository, kind and ULID, and viewing
never provisions a working copy.

## Owner Direction, 2026-11-07

The owner set the scope and priority order of this Wave:

1. Title bar. The window title shows Manyhands and the selected
   repository's name (later a project or team name).
2. Footer bar. Buttons toggle each panel (Repositories, Tickets, Documents,
   Activity). Each button has a menu that sets its panel's dock location, as
   Zed does. Toggles show or hide the left and right dock areas.
3. Dockable, resizable panels. Each current region can be docked, shown or
   hidden, and docked panels can be resized.
4. Each panel's contents and rendering rebuilt to the design: Repositories,
   Tickets, Documents, Activity.
5. Settings moves into a sheet.
6. Later, remedial work, to be defined: light and dark mode switching, other
   dialogs and panels, moving some content from the bottom of the item view
   into its details sheet.

Items 1 to 3 form Cycle 01, item 4 spans Cycles 02 and 03, and item 5 is
Cycle 04. Later the same day the owner added Cycle 05 for the sheets and
dialogs that already exist but were left out: the "other dialogs" part of
item 6, for the surfaces the app has today. The rest of item 6 is
[Deferred Work](#deferred-work).

These parts of the design are aspirational and are **not** built in this
Wave: Search (the title bar field and Ctrl K), the Teams panel and the
Projects panel, with the Project and Team scope kinds. Anything that would
show them is left out, not stubbed.

Later on 2026-11-07 the owner settled two points:

- **Settings and Needs attention stay where the design puts them**, in the
  footer (`02`, "Footer", items 5 and 6), with the Needs attention popover
  anchored above its footer item. An earlier direction moved them to the
  title bar; the owner withdrew it.
- **There is no top dock.** The dock menu offers left, right and bottom, as
  the handoff does and GPUI Kit 0.6.6 supports.

## Planning Baseline

Read on 2026-11-07 against main at `1e5616af`.

- **Layout today.** `Workspace::render` (`src/desktop/view/workspace/render.rs`)
  builds a fixed h-flex: Repositories (260 px) | Navigation (320 px, Documents
  and Tickets together) | Content (flex), above a 260 px Operations area.
  `Region` (`Repositories`, `Navigation`, `Content`, `Operations`) drives
  focus, F6 cycling and `secondary-1` to `secondary-4`.
- **Sheet layer placement.** Root's sheet layer is drawn inside the content
  region on purpose, so that a sheet's focus counts as the content region's
  and workspace keys are refused behind it (R-60). The dock migration must
  keep this property.
- **Settings today** is a page shown in the content region by `SettingsHost`
  (`src/desktop/view/settings/mod.rs`), with pages Repositories, Remotes,
  Shared SSH keys and Hosts, opened with `secondary-,` or the sidebar's
  Settings entry (ruling R-B4). The Repositories page also carries the Git
  identity form.
- **GPUI Kit 0.6.6** (locked) already has what Cycle 01 needs, so no upgrade
  Cycle is planned: `DockArea` with `set_dock`, `toggle_dock`,
  `set_dock_size`, `add_panel`, `move_panel`, `remove_panel`, `dump` and
  `load` of a versioned `DockAreaState`, `DockEvent::LayoutChanged`,
  stacked and tabbed groups, and drag between docks; `h_resizable` and
  `v_resizable`; `TitleBar`, `StatusBar`, `Sheet`, `Popover`, `Menu`, `Tree`,
  `Table` and `VirtualList`. Each Cycle still verifies the names it uses.
- **No top dock.** `DockPlacement` at 0.6.6 is `Center`, `Left`, `Bottom`,
  `Right`, and its serde tags are a frozen on-disk schema. The owner accepts
  left, right and bottom only (decision 2).
- **Theme.** The Kit theme is closed to new slots. `src/desktop/view/item/tokens.rs`
  already keeps the item view's own colors for both modes, plus the
  handoff's `accent` and `faint` chrome tokens. There is no workspace-wide
  token module yet.
- **Wave 03 still open.** D6 (desktop background polling) and G1 are not
  merged. The design's "Checked for updates", "Checking…" and "Paused"
  states, and the Settings "Checking for updates" page, depend on D6. Ready
  and Blocked ticket filters already exist (`src/desktop/view/tickets.rs`).

## Required Source Amendments

Made when this Wave is approved, on its ticket:

- **Desktop IA RFC.** Replace "No boards, project rollups or configurable
  docking system are required" and the fixed-layout paragraph with: a title
  bar; left, right and bottom docks around the item area; dockable,
  resizable Repositories, Tickets, Documents and Activity panels shown,
  hidden and moved from the footer; layout persisted in app-local data,
  never in a repository; Activity plus Needs attention as the persistent
  operation area; Settings as a sheet; and the plain-language vocabulary of
  `docs/design/06`. Boards and rollups stay excluded.
- **`docs/design/09-implementation-plan.md`.** Note that this Wave supersedes
  its sequence.

## Scope

- A desktop layout model: which panels exist, where each is docked, which
  are visible, dock sizes and open state, and its app-local persistence with
  a fallback default.
- A workspace chrome token module from `07-design-tokens.md`, both modes,
  absorbing the chrome tokens now in `item/tokens.rs`.
- The title bar, the footer, the dock area and the four panels.
- Panel contents per `03`, for Repositories, Tickets, Documents and
  Activity, in Repository scope.
- The Needs attention popover.
- The Settings sheet, holding the existing settings pages.
- The existing sheets and dialogs, in the design's frame and layout.

## Explicit Exclusions

- Search, the command palette, Projects and Teams panels and scopes (owner,
  aspirational).
- The title bar's File, Edit, View and Help menus. Every command they would
  hold already has a key binding or a control.
- Item-area splits and the focused-pane treatment (`02`, "Center";
  `04`). The center keeps today's single tab group.
- Sheets and dialogs the app does not have yet, such as the design's
  General settings page.
- A theme switch, the Appearance settings page and light-mode polish. Tokens
  carry both modes so that this is later work, not rework.
- A row density setting and font changes (decisions 7 and 8).
- Any library, repository-format or CLI change. A gap found in the headless
  API goes to a separate library ticket merged to main first, as in Wave 03.

## Cycles

Five Cycles, in priority order. Each gets a Manyhands ticket before its plan,
and its Cycle document at the path named below.

| Cycle | Delivers | Depends on |
| --- | --- | --- |
| 01 Workspace shell | Title bar, docks and panels, footer toggles, layout persistence, chrome tokens | Wave approval and RFC amendment |
| 02 Tickets and Documents panels | The two lists rebuilt to `03` | 01 |
| 03 Repositories, Activity and Needs attention | Sharing states, the activity log, the popover, the footer's "to share" | 01; D6 for the polling states only |
| 04 Settings sheet | Settings moved from the item area into a sheet | 01 |
| 05 Existing sheets and dialogs | Add repository, Unlock key, Close ticket and Approve and merge, Resolve conflict and the other dialogs in the design's frame | 01, 04 |

Cycles 02, 03 and 04 each depend only on 01. Cycle 05 also depends on 04,
because it moves Add repository out of the Settings sheet's Repositories
page. Run them in order unless the owner asks to overlap them. 02 and 03 both touch panel headers and shared row
helpers, so overlapping them means more merging.

### Cycle 01: Workspace Shell

**Planned document:** `docs/Cycles/wave-04-cycle-01-workspace-shell.md`

**Purpose:** Replace the fixed layout with the design's window anatomy, using
today's region contents unchanged inside the new frame.

**In scope:**

- **Chrome tokens.** A workspace token module for `07`'s chrome colors
  (`chrome`, `panel`, `border`, `border-strong`, `hover`, `faint`, `muted`,
  `secondary`, `text`, `selection`, `accent`, `teal`, `green`, `red`), both
  modes, applied to the title bar, panel headers and footer. The item view
  reads `accent` and `faint` from it.
- **Title bar** (Kit `TitleBar`, 38 px): app icon, the title text and the
  platform window controls.
- **Window title** set through the window and shown in the title bar:
  `Manyhands — {repository}` for the selected repository, `Manyhands` when
  none is selected. The repository name is the one shown in the
  Repositories panel.
- **Dock area** (Kit `DockArea`) with the item area as its center, holding
  the current content region with its tab strip, item view, Settings and
  Drafts pages and empty state. Root's sheet layer stays inside the center
  (R-60).
- **Panels**, registered by stable name: `RepositoriesPanel` (today's
  sidebar), `TicketsPanel` and `DocumentsPanel` (today's Navigation pane,
  split in two, each with its current contents), `ActivityPanel` (today's
  Operations area). Panel headers follow `02`, "Panel header": single panel
  with icon, title and count; tabbed group with the amber active line.
  Toolbars stay as they are until 02 and 03.
- **Default layout** (decision 1, recommended): left dock 290 px with
  Repositories (0.6) stacked over Documents (1.4); right dock 440 px with
  Tickets; bottom dock 150 px with Activity.
- **Resizing** of docks and of stacked groups, with the toolkit's handles.
- **Footer** (Kit `StatusBar`, 28 px), per `02`, "Footer": the left dock
  toggle and one toggle per left panel; the bottom dock toggle and one per
  bottom panel; one toggle per right panel and the right dock toggle. On and
  off states, tooltips "Show Tickets" and "Hide Tickets". Also, per `02`:
  the **Needs attention** item (`circle-alert` + "N need attention", red,
  hidden at 0, counting the unresolved recovery states the operations area
  already shows; until Cycle 03 it shows and focuses the Activity panel),
  and after a separator the **Settings** button (`settings`, tooltip
  "Settings (Ctrl ,)") opening today's Settings page. The other status
  items are Cycle 03's.
- **Dock menu** on every panel toggle (secondary click and the keyboard's
  context-menu key, as in Zed): "Dock left", "Dock right", "Dock bottom",
  current placement checked.
  Moving a panel carries its footer button to that side.
- **Behavior** from `02`: showing a panel shows its dock; hiding every panel
  in a dock collapses it and the center widens; a dock toggle hides or shows
  the dock without changing which panels are on; dragging a panel to
  another dock or into a tabbed group is the toolkit's and is persisted like
  any other change.
- **Persistence.** Save the dumped `DockAreaState` and the visible-panel set
  to a versioned layout file in the app data directory, debounced on
  `LayoutChanged`. Restore at launch. Fall back to the default layout, and
  report it in Activity, when the file is missing, unreadable or of another
  version. Never write layout into a repository.
- **Keyboard.** Dock toggles Ctrl B (left), Ctrl Alt B (right), Ctrl J
  (bottom), as in Zed. One toggle per panel (decision 3). The region
  actions keep working: `secondary-1` focuses Repositories, `secondary-2`
  the last used of Tickets and Documents, `secondary-3` the item area,
  `secondary-4` Activity, and F6 cycles only visible panels. Focusing a
  hidden panel shows it. Every title-bar and footer control is reachable by
  Tab and has an accessible name.

**Out of scope:** New panel contents, toolbars or row rendering, the Needs
attention popover, the Settings sheet, the footer's sharing and update-check
status items, and the
dock-location menu for the item area itself.

**Exit evidence:** The app starts in the default layout; each panel can be
hidden, shown and moved to every offered dock from the footer, by mouse and
by keyboard; docks and groups resize; the layout, including a hidden dock and
a moved panel, survives a restart; a corrupt or old layout file restores the
default without losing anything else; the window title follows the selected
repository; every existing keyboard journey still passes. Model tests cover
the visibility rules, menu moves, restore fallback and title derivation.
Native check of the title bar on Linux Wayland, with macOS and Windows after
merge (decision 4).

**Split line:** if the plan does not fit one reviewable Cycle, split it into
01a (tokens, title bar, dock area with the four panels, persistence, and a
footer with only Needs attention and Settings) and 01b (footer toggles, dock
menus, keyboard toggles), and update this Wave.

### Cycle 02: Tickets And Documents Panels

**Planned document:** `docs/Cycles/wave-04-cycle-02-tickets-and-documents.md`

**Purpose:** Rebuild the two lists people use most to `03-panels.md`.

**In scope:**

- **Common row anatomy** (28 px, hover, the selected treatment with the
  amber left line and 600 weight, visible keyboard focus) and the four
  states (loading skeleton rows, empty with its primary action, failed or
  stale scan with Retry, normal). Lists stay virtualized and keyboard
  navigable.
- **Tickets:** toolbar (New ticket, More); filter bar with the
  `Open | Closed | All` segmented control and counts, Open the default; the
  "Filter by title or code" input; sortable column header (type, code,
  title, status, updated; default updated, newest first); type icons; status
  pills by the `03` color table, unknown values neutral; editing and
  conflict markers after the title; nonconforming rows kept visible with
  "needs fixing"; closed titles in `secondary`; the footnote
  `{n} open tickets in {repository}`. Ready and Blocked filters keep working,
  placed per decision 5.
- **Documents:** toolbar (New document, New folder, Collapse all); tree rows
  with depth indent, chevrons, folder and file icons, collapsed descendant
  counts, the document title as the label and the path in the tooltip;
  editing marker; nonconforming rows with the amber label and "Not a
  managed document yet. Open to adopt it."; empty folders with "Shared once
  it contains a document."; ← and → collapse and expand.
- Repository scope only, so the Documents tree shows the selected
  repository's tree directly, with no repository grouping.

**Out of scope:** Project and Team scopes and multi-repository grouping;
column resizing or choosing columns.

**Exit evidence:** PRD `MH-CONTENT-001/003/004` list behaviors still pass;
nonconforming and inaccessible items stay visible with their reason; Open is
the default; sort, filter and the four states are covered by model tests;
duplicate short codes list every match; keyboard-only navigation and opening
work in both panels.

### Cycle 03: Repositories, Activity And Needs Attention

**Planned document:** `docs/Cycles/wave-04-cycle-03-repositories-and-activity.md`

**Purpose:** Make sharing state and recovery visible in plain language, in
the places the design puts them.

**In scope:**

- **Repositories:** toolbar (Add or create a repository, opening today's
  flow; Check all for updates now, when D6 provides it; More with Rebuild
  and Remove); rows with `folder-git-2`, name and the right-aligned state
  from `03` ("2 to share", "Up to date", "This computer", "Can't reach");
  the Git detail in the tooltip; the context menu (Share main, Open folder,
  Settings…, Remove from Manyhands… with its confirmation saying the files
  stay). "Checking…" and "Paused" and the Check now action appear only once
  D6 has merged.
- **Activity:** rows of time, colored icon, message in the `06` register,
  repository tag and at most one action; the Git detail in the tooltip;
  newest first; failures kept until resolved; the "Show only things that
  need attention" filter. Today's operations-area content (finishing,
  session and pending sections) is carried into these rows, not dropped.
- **Needs attention popover** (`05`, 2f), anchored above the footer item:
  one row per unresolved recovery state with its single action, and "Show
  everything in Activity". It is a filtered view of the same store as
  Activity.
- **Footer status:** "N to share" (`cloud-upload`, amber), selecting the
  repository with unshared work. "Checked for updates …" only once D6 has
  merged.

**Out of scope:** The Add repository sheet (Cycle 05); background polling
itself (D6).

**Exit evidence:** Failures persist until resolved and appear in both
Activity and the popover; every popover action works by keyboard; Activity
can be hidden and the popover still works; each repository state comes from
real service state, with no fabricated rows; no secret appears in a row,
tooltip or popover (extend the existing privacy tests).

### Cycle 04: Settings Sheet

**Planned document:** `docs/Cycles/wave-04-cycle-04-settings-sheet.md`

**Purpose:** Move Settings out of the item area into the design's sheet, so
opening it no longer hides the open item.

**In scope:**

- The sheet frame from `05` (right side, 720 px, header, scrolling body,
  footer) with the 214 px left nav, opened from the footer's Settings
  button and Ctrl ,; focus trapped, Esc closes, focus returns to the
  invoking control.
- The existing pages folded into the design's nav: **Repositories** (today's
  Repositories and Remotes pages, with the identity form), **Sharing &
  keys** (today's Shared SSH keys page, plus Trusted servers from today's
  Hosts page), and **Checking for updates** only once D6 has merged.
  Behavior unchanged; copy moves to the `06` vocabulary, and Sharing & keys
  follows `05`'s layout.
- Every existing route into Settings (recovery actions such as
  `repo.identity_set`, Share… with no remote, Open Sharing & keys from a
  failed close) opens the sheet at the right page and field.
- Settings removed from the content region and from the sidebar.

**Out of scope:** The General and Appearance pages, and new settings.

**Exit evidence:** Every existing settings journey and recovery route passes
through the sheet by keyboard; opening and closing Settings leaves the open
item, its scroll position and any draft untouched; the rule for an item
details sheet that is already open (decision 6) is tested.

### Cycle 05: Existing Sheets And Dialogs

**Planned document:** `docs/Cycles/wave-04-cycle-05-sheets-and-dialogs.md`

**Purpose:** Bring the sheets and dialogs that already exist up to the
design, so that the Wave leaves no surface in the old style. Every one keeps
its current behavior, services, request handling and recovery routes. Only
its frame, layout, copy and states change.

**Check first.** Wave 03 D5 built the lifecycle dialog and the resolve
conflict sheet from the same handoff (frames 2c to 2e and 2g), and the owner
has not yet compared the result with the design. Before planning any work
on either, compare each with `05` and its screenshots, and record in the
Cycle document what already matches and what differs. Only the differences
are in scope. If both already match, they leave the Cycle apart from the
shared tokens and frame, and the split line below is unlikely to be needed.

**In scope:**

- **Common frame** from `05`: the sheet and dialog frames (header, body,
  footer), primary and secondary buttons, 32 px inputs with the teal focus
  ring, scrims, and focus that is trapped, closes on Esc unless an operation
  is running, and returns to the invoking control. It applies to every
  existing dialog, including those the design does not draw on its own: the
  confirmation dialog (`confirm.rs`), New ticket… and New document…, New
  folder…, Move…, the close-draft dialog and the quit dialogs.
- **Add repository sheet** (`05`, 1d, 470 px): today's Add an existing
  repository and Create a repository, moved out of the Settings
  Repositories page into the sheet, with the mode segmented control, the
  live folder checks and their recovery actions, the main branch select
  with its Suggested tag, the sharing remote select (non-SSH remotes listed
  but disabled, with the reason), the "Removing a repository … never
  deletes" note, and the identity step when Git has none. It opens from
  the Repositories panel's `+`, from the Settings Repositories page and
  from the empty Repositories panel.
- **Unlock key dialog** (`05`, 2h, 440 px): today's passphrase dialog with
  the design's header, key card, note, Not now and Unlock, the inline error
  for a wrong passphrase, and Not now moving the prompt into Needs
  attention. The show and hide toggle follows decision 9.
- **Close ticket and Approve and merge dialog** (`05`, 2c to 2e, 580 px),
  where the check finds differences: today's lifecycle dialog laid out as
  the design's stages. Confirm shows
  the numbered step list, the files to be merged with their tags and line
  counts (and the note when code is included), "Becomes ready to start" and
  the Git detail line. Running shows each step's state and progress, with
  Stop at a safe point. Failed names the step and cause, shows the outcome
  box, the contextual fix and Retry. Done shows the outcome and the newly
  ready tickets. The local-only wording for step 2 is kept.
- **Resolve conflict sheet** (`05`, 2g, 920 px): already built from the
  design in Wave 03 D5. This Cycle moves it onto the shared tokens and
  frame and corrects only the differences the check found, without
  changing how parts are chosen or saved.
- **Empty item area** (`04`, "Empty pane"): the faded app icon and "Open a
  ticket or document from the lists". The "Ctrl K to search" line waits
  for Search.

**Out of scope:** New behavior in any of these surfaces, the General and
Appearance settings pages, and theme switching.

**Exit evidence:** Every existing journey through these surfaces still
passes by keyboard: adding and creating a repository, including the
identity step and each failed check's recovery action; unlocking, Not now,
a wrong passphrase; a close and an approval through confirm, running,
failed with Retry, and done, including a local-only repository; resolving
a conflict. Focus returns to the invoking control after each. No
passphrase or private key text reaches any label, tooltip or Git detail
line (extend the existing privacy tests).

**Split line:** if the plan does not fit one reviewable Cycle, split it into
05a (common frame, Add repository sheet, Unlock key dialog, empty item area)
and 05b (Close ticket and Approve and merge dialog, Resolve conflict sheet),
and update this Wave.

## Testing

Per `AGENTS.md`, "Testing Strategy". This Wave changes `src/desktop/` only,
so each Cycle runs the fast tier at review-ready and no SSH target. Tests
check behavior, not appearance: layout-model rules, visibility, persistence
and fallback, actions and their enablement, focus and keyboard reach, what a
service receives, list sort and filter, and the states each surface shows.
No tests for colors, sizes, icons or that a view renders. Appearance tests
broken by the deliberate visual changes in this Wave are deleted, not
updated. The layout file's restore and fallback is one table-driven test.

## Integration Gate

Wave 04 is complete when its Cycles have approved exit evidence and:

- The desktop starts in the default layout, and a user can build at least
  the design's 1a, 1b and 1c layouts (without Projects and Teams) from the
  footer and by dragging, and get them back after relaunch.
- Every Wave 03 desktop journey still passes by keyboard through the new
  shell.
- Settings, Needs attention, the four panels, the window title and the
  existing sheets and dialogs follow the design as amended here, in the dark theme, with both modes' tokens in
  place.
- The static gates and the fast tier pass on the integrated tree, and the
  desktop smoke test runs on Linux Wayland. Native CI on all four targets
  runs after merge, as `AGENTS.md` sets out.

## Risks And Controls

| Risk | Control |
| --- | --- |
| Moving regions into `DockArea` breaks focus, R-60 sheet refusal or key contexts | Cycle 01 keeps the sheet layer in the center and the region actions mapped to panels; the existing keyboard and dialog tests are the regression net. |
| Client-side title bar behaves differently per platform (drag, double-click, controls, Wayland decorations) | Use Kit `TitleBar` and its window options; check Linux locally, macOS and Windows in native CI after merge (decision 4). |
| A layout file from an older build, or a removed panel, breaks startup | Versioned file, fallback default reported in Activity, unknown panels dropped; one table-driven test. |
| D6 lands while Cycles 01 and 03 move and rework the operations area | Cycle 01 moves the area unchanged; 03 bases its plan on whatever D6 has merged and leaves the polling states out until then. Merge main often, per `AGENTS.md`. |
| Cycle 01 or 05 grows too large to review | The split line in each. |
| Restyling the lifecycle dialog and the resolve conflict sheet regresses D5 behavior (stale confirmations, retry reusing the operation, safe-point stop, no automatic winner) | Cycle 05 changes layout and copy only; D5's existing tests stay and must pass unchanged in what they assert about behavior. |
| The design's list specs drop a Wave 03 behavior (Ready and Blocked, duplicate short codes, inaccessible entries) | Cycle 02's exit evidence names them; decision 5 places Ready and Blocked. |

## Open Decisions For The Owner

1. **Default layout.** Recommended: 1b without Projects and Teams (left:
   Repositories over Documents; right: Tickets; bottom: Activity).
2. **Top dock.** Settled 2026-11-07: no top dock. The owner accepts left,
   right and bottom, which GPUI Kit 0.6.6 supports.
3. **Panel toggle keys.** The handoff's Ctrl Shift R and Ctrl Shift D clash
   with Rebuild (`secondary-shift-r`) and Open drafts (`secondary-shift-d`).
   Recommended: no per-panel keys this Wave beyond the
   existing `secondary-1` to `secondary-4` (which now also show a hidden
   panel), plus the three Zed dock toggles.
4. **Title bar on macOS and Windows.** Recommended: Kit `TitleBar` on every
   platform, so the title bar looks and behaves the same everywhere.
5. **Ready and Blocked filters.** Recommended: a filter menu beside the
   Open/Closed/All control, keeping that control as designed.
6. **Settings with a details sheet open.** Root has one sheet layer.
   Recommended: opening Settings closes the details sheet first (its edits
   live in the shared draft, so nothing is lost).
7. **Row density.** Recommended: 28 px rows, no setting yet.
8. **Fonts.** Recommended: the toolkit's UI font and one monospace font, at
   the design's size scale.
9. **Passphrase show and hide.** The design's Unlock key dialog has an `eye`
   toggle, but today's dialog has none on purpose: the Kit's masked input
   refuses copy and hides its value from accessibility. Recommended: keep it
   without the toggle.
10. **Add repository from Settings.** Root has one sheet layer, so the Add
   repository sheet can't sit over the Settings sheet. Recommended: opening
   it from Settings replaces the Settings sheet, and Cancel or Add returns to
   the Settings Repositories page.

## Deferred Work

Owner item 6 and the excluded design parts, to be planned later:

- Theme switching and the Appearance page; light-mode polish.
- Other dialogs and panels the app does not have yet.
- Moving content from the bottom of the item view into its details sheet.
- Item-area splits and the focused-pane treatment.
- Search and the command palette; Projects and Teams panels and scopes;
  title-bar menus.

## Approval And Remaining Readiness

Drafted 2026-11-07 from the owner's direction of that date. Not yet
approved. Approval needs the owner's answers to the open decisions, or
acceptance of their recommendations, and the source amendments above. Each
Cycle then needs its own ticket, Cycle document, plan and approval before
implementation.
