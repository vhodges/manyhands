---
title: "Wave 04 Cycle 02 Tickets And Documents Panels Design"
date: 2026-11-11
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
---

# Wave 04 Cycle 02 Tickets And Documents Panels Design

## Intent And Approval Boundary

This design covers [Cycle 02](../Cycles/wave-04-cycle-02-tickets-and-documents.md):
the Tickets and Documents panels rebuilt to `docs/design/03-panels.md`.
That means the common row anatomy and the four list states, then the
Tickets toolbar, filter bar, column header, rows and footnote, then the
Documents toolbar and tree. Approval covers the mechanisms, rulings, named
test edits and task order. It does not authorize implementation, pushes
beyond the ticket branch, merging or closing.

**Draft, revision 1.** Grounded at the ticket branch `a0a734a4`, which
holds origin/main `48653d01` (Cycle 01 and Wave 03 D6 both merged).

Conventions:

- `D/` is `src/desktop/`; `L/` is the library under `src/`.
- `B/`, `C/` and `G/` are `gpui-base-0.6.6/src/`, `gpui-component-0.6.6/src/`
  and `gpui-pre-0.3.6/src/` under
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (the locked
  crates); `A/` is `gpui-kit-assets-0.6.6/`.
- "Fact n" refers to Baseline Facts. "P-n", "R-n", "E-n", "K-n" and "U-n"
  are this design's Required Outcomes, Rulings, Named Edits, Known Limits
  and unverified facts. "Decision n" is a Cycle decision; "Wave decision n"
  is Wave 04's; "C1 R-n" is a ruling of Cycle 01's design.
- "The handoff" is `docs/design/`; `03` is `03-panels.md`, `06` is
  `06-language-and-states.md`, `07` is `07-design-tokens.md`.
- "The prototype" is `docs/design/mockups/source/Workspace.dc.html`, which
  governs exact values where the specs are silent (handoff README:15).
- `{repo}` is the repository's display name, the one the Repositories
  panel and the window title show (C1 R-14).

## Baseline Facts At `a0a734a4`

**Kit lists (0.6.6).**

1. **Kit `List`.** `ListState::new(delegate, window, cx)`
   (`C/list/list.rs:94`). The delegate's `type Item: Selectable +
   IntoElement` need not be `ListItem` (`C/list/delegate.rs:11`). Every
   row has the height measured from one row (`C/list/list.rs:69`,
   `:429-451`). Rows are wrapped in `div().overflow_hidden()` and given
   `.selected()` (`:490-502`); the container has `Role::List` and rows
   `Role::ListItem` with position and selected state (`:474-501`, `:790`).
   A click selects and emits `Confirm` (`:503-514`). Keys in the "List"
   context are escape, enter, secondary-enter, up and down; up and down
   wrap (`:26-35`; `C/list/cache.rs:107-170`). There is no Home, End or
   Page key. `scroll_to_selected_item` exists (`C/list/list.rs:263`). The
   list's focus handle is not a tab stop (`:102`; `G/window.rs:546-547`).
2. **Kit `ListItem`** applies its selected background (`list_active`, or
   `accent`) after the caller's style, so a caller cannot replace it
   (`C/list/list_item.rs:193`, `:234-246`). Calling `.hover()` on it is
   unsupported (`:165-167`).
3. **`DataTable`** (`C/table/`) has a sticky header and per-column sort,
   but: sorting is by clicking a small icon only, with no keyboard path
   (`C/table/state.rs:1561-1580`); Enter has no binding
   (`C/table/data_table.rs:17-29`); Tab and Shift-Tab move between
   columns inside the table, so Tab cannot leave it
   (`:1034-1109`; `G/keymap.rs:154-159`); Home and End select columns
   (`:925-967`).
4. **Tree.** `gpui_kit::base::Tree::new(&state).item(Fn(usize, &TreeEntry,
   TreeEntryState, &mut Window, &mut App) -> AnyElement)` takes any element
   and draws nothing of its own (`B/tree.rs:466-497`). The styled
   `component::tree::tree` forces a `ListItem` row (`C/tree.rs:18-23`,
   `:83-90`) and adds `.vertical_scrollbar(&scroll_handle)` around the
   base tree (`:111`). Rows are a `uniform_list` (`B/tree.rs:423`).
   `TreeEntry::depth()` (`:66`); `TreeEntryState::is_selected()`
   (`:164-178`). Each row's wrapper has `Role::TreeItem`, `aria_label`,
   `aria_selected` and, for folders, `aria_expanded` (`:432-441`).
5. **Tree expansion and selection.** Expansion lives in each `TreeItem`
   (`B/tree.rs:34-46`, `:121-124`); `TreeItem::expanded(bool)` sets it.
   `is_folder()` is `!children.is_empty()`, so an empty folder is a leaf
   that cannot toggle (`:131-134`). `set_items` rebuilds rows, clears the
   selection and emits nothing (`:214-219`). `TreeEvent::{Expanded(id),
   Collapsed(id)}` are emitted on toggles (`:93-96`, `:305`, `:331-335`).
   Keys in the "Tree" context are up, down, left and right only; up and
   down wrap; left collapses and right expands without moving
   (`:20-27`, `:365-411`). A `Confirm` handler toggles folders but enter
   is not bound (`:350-363`). Mouse-down selects and toggles a folder
   (`:443-447`). `set_selected_index`, `index_of(id)`,
   `scroll_to_item(ix, strategy)` exist (`:221-266`).
6. **Segmented controls.** `ButtonGroup`, `ToggleGroup::segmented` and
   `TabBar::segmented` exist, but none has arrow keys; `TabBar` tabs are
   not focusable at all (`B/tabs.rs:17-21`). Any focused element with
   `on_click` is activated by Enter and Space (`G/elements/div.rs:3010-3066`).
7. **Input.** `InputState::new(window, cx).placeholder(…)`;
   `InputEvent::{Change, PressEnter, Focus, Blur}`; `set_value` emits no
   Change; `.clean_on_escape()` clears on Escape, else Escape propagates
   (`B/input/base/state.rs:766`, `:897-915`, `:1086`, `:2010-2044`).
   `Input::new(&state).prefix(…).cleanable(true)` (`C/input/input.rs:240`,
   `:281`). The placeholder is its accessible name when none is given
   (`:665-669`).
8. **Menus.** `Button::dropdown_menu(f)` opens a `PopupMenu` on left
   mouse-down or on Enter and Space, and the menu takes focus
   (`C/menu/dropdown_menu.rs:12-34`, `:150`; `B/popover.rs:17-22`,
   `:288-309`). `PopupMenuItem::new(label).checked(bool).on_click(…)`
   (`C/menu/popup_menu.rs:180-210`). A checked item is not exposed to
   accessibility (`C/menu/menu_item.rs:99-101`).
9. **Skeleton.** `component::skeleton::Skeleton` pulses with a repeating
   two-second animation (`C/skeleton.rs:39-57`).
10. **Tooltips** appear on hover only, never on keyboard focus
    (`G/elements/div.rs:1674-1702`; `C/tooltip.rs:247-292`).
11. **Accessibility.** `aria_label`, `aria_description`, `aria_selected`,
    `aria_expanded`, `aria_toggled` exist on stateful elements
    (`G/elements/div.rs:1304-1497`). `aria_sort` does not exist.
12. **Icons.** Of the design's icons, `repeat`, `bug`, `sparkles`,
    `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`,
    `arrow-up-down` and `list-filter` are in the Kit catalog but not loaded
    by the app; `file-warning` does not exist, and
    `file-exclamation-point` does (`A/build.rs:19-67`). The app serves
    extra icons through `icon_assets!(ItemIcons, […])`
    (`D/assets.rs:17-58`); an icon not listed there or in the Kit defaults
    fails to load (`A/src/native_assets.rs:29`).

**Tickets today.**

13. **Body.** Find, then New ticket…, the closure choices
    `filter-open|closed|all` with a "Showing …" line, the readiness
    choices `filter-any-readiness|ready|blocked` with their line, the list
    state line, the list `tickets-list`, and Why blocked under Blocked
    (`D/view/lists.rs:53-152`). Each choice is a `Role::Tab` div with its
    own focus handle around a Kit `Button` (`:155-177`).
14. **Rows.** A Kit `List` of `ListItem`s: the title on line 1; short code
    or "No short code", type, status, "Open" or "Closed", and the raw
    RFC 3339 change time on line 2; under Blocked a reasons line on every
    row (`D/view/tickets.rs:112-210`). The row's `aria_label` is the title,
    or the path for a nonconforming entry (`:96-110`, `:197-204`).
15. **Query and read.** `TicketQuery { closure, readiness }` maps onto the
    library's `TicketFilter`, and every filter change reads again
    (`D/model/tickets.rs:33-57`; `D/view/workspace/lists.rs:150-168`).
    Under Blocked a second read lists every open ticket to name blockers
    (`D/model/tickets.rs:59-67`). The desktop does not sort
    (`:310-322`).
16. **What one read gives.** Every ticket of every `list_tickets` read
    carries its closure, its readiness state and its blocked reasons,
    whatever the filter (`L/repository/read/items.rs:1689-1696`,
    `:1007-1048`; `L/repository/read/dto.rs:443-450`, `:505-508`). The
    library orders tickets by change time, latest first, then ID, and puts
    nonconforming entries last in path order (`items.rs:1745-1757`). A
    filter never removes a nonconforming entry (`:63`).
17. **Fields.** `ItemDto` has `slug` (lowercase), `title`, `ticket_type`,
    `status`, `closure`, `readiness`, `changed_at` (RFC 3339 UTC, from the
    last index refresh), `context { kind: Primary|Unverified|Active,
    branch, worktree }` and `problems`
    (`L/repository/read/dto.rs:347-356`, `:425-478`). `TicketRow` keeps
    neither `readiness.state` nor `context.kind`
    (`D/model/tickets.rs:145-194`). A nonconforming entry has no ID, title
    or metadata, only a path and problem codes, and its field is not named
    (`items.rs:528-532`, `:965-980`; `L/results.rs:785-799`).
18. **List states.** `ListState::{Loading, Ready { rows, complete, index },
    Busy, Failed }`, with labels for loading, empty, never refreshed,
    stale, incomplete, busy, failed and index unavailable
    (`D/model/list_state.rs:16-52`, `:101-125`). Tickets apply a reload
    with `from_read`, so a busy reload clears the rows
    (`D/model/tickets.rs:302`); `apply_keeping` keeps rows over a busy read
    and replaces them over a failure (ruling R-B5,
    `D/model/list_state.rs:72-98`). A job-driven reload keeps the old rows
    on screen while the model says Loading
    (`D/view/workspace/lists.rs:36-46`, `:117-127`).
19. **Find** sits at the top of the Tickets panel (C1 decision 4).
    `secondary-p` reveals Tickets and focuses it. Enter on a ULID opens that
    item, ticket or document, closed or not, through `show_item`; other
    text is looked up as a short code with `find_tickets_by_slug`, and
    every match is listed as a card with its context and an Open button
    (`D/model/find.rs:32-57`; `D/view/workspace/find.rs:24-80`;
    `D/view/find.rs:44-171`).
20. **Header count** is the number of rows listed under the current
    filters, absent until the list is loaded (C1 R-22;
    `D/view/dock/panels.rs:166-180`). The header draws icon, title and
    count only; there is no toolbar slot (`D/view/dock/header/tabs.rs:116-165`).

**Documents today.**

21. **Tree.** The styled Kit `tree()` of `ListItem`s, indented 12 px per
    level, with no chevrons, icons or tooltips (`D/view/documents.rs:129-221`).
    A document shows its title and its path on a second line; a
    nonconforming file its path, "Nonconforming" and its first problem
    code; an empty folder "Empty folder" (`:36-48`, `:148-199`). Every
    folder is rebuilt expanded on every reload (`:286`), so expansion does
    not survive one. Selection is restored by id (`:81-96`).
22. **Model.** `TreeNode::{Folder { path, empty, children }, Document(ItemDto)}`,
    folders first in the folder read's order, documents in the library's
    path order; a document whose folder was not listed goes under the
    nearest listed ancestor (`D/model/documents.rs:23-127`).
    `document_count` counts documents, not folders (`:37-46`). Two reads,
    folders and documents, combine into one state (`:227-253`); a failed or
    busy reload clears the tree (`D/view/workspace/lists.rs:95-103`).
23. **Empty folders.** `DocumentFolderDto.empty` means the directory holds
    nothing at all; folders come from the primary worktree's index only
    (`L/repository/read/dto.rs:389-417`).
24. **Keys.** Up, down, left and right are the Kit's; Enter is the app's
    `OpenSelected` in the `Documents` context and opens a document,
    ignoring a folder (`D/keys.rs:145-147`, `:255`;
    `D/view/workspace/lists.rs:201-205`). New document… and New folder…
    are body buttons (`D/view/documents.rs:226-242`).

**Shared.**

25. **Unsaved and editing state** is not on `ItemDto`. In memory:
    `DraftRegistry::held_by(repo, kind, id) -> Held { Yes, No, Unknown }`
    over live sessions and the cached store listing, with no I/O
    (`D/services/drafts/held.rs:126-170`). The item's own private working
    copy is `context.kind == Active` (fact 17).
26. **Pending conflicts** are kept per repository as
    `Workspace.pending_conflicts: HashMap<RepoKey, Vec<PendingConflict>>`,
    each naming its item's ID and kind (`D/view/workspace.rs:141-145`;
    `D/services/pending_conflicts.rs:55-67`).
27. **Times.** `relative_time(at, now)` gives "just now", "{n} min ago" …
    "{n} days ago", then the date (`D/view/item/chips.rs:180-206`). The
    header's "waiting on" text for a blocked ticket is built beside it
    (`:208` on).
28. **Index maintenance.** `Workspace::maintain_index(rebuild, cx)` submits
    `IndexRefresh` or `IndexRebuild` for the selected repository as a user
    job (`D/view/workspace/requests.rs:333-350`), bound to `secondary-r` and
    `secondary-shift-r` (`D/keys.rs:240-241`). A finished job reloads the
    lists.
29. **Chrome tokens** `selection`, `accent` (amber), `hover`, `muted`,
    `secondary`, `text`, `border`, `teal`, `green` and `red` exist in both
    modes (`D/view/shell/tokens.rs:12-76`).

## Part A: Common Row Anatomy And List States

**Row (`ListRow`).** One element in `D/view/rows.rs` for both panels. It
implements the Kit's `Selectable` (`B/component_traits.rs:3-10`), so the
Kit `List` can use it as its item (fact 1), and the tree draws it directly
(fact 4).

- 28 px tall, one line, padding `0 10 0 12` (the tree sets its own left
  indent), gap 8, 13 px text.
- Hover: `hover` background.
- Selected: `selection` background, a 2 px inset left line in `accent`,
  and the label at weight 600.
- Keyboard focus: while the row's list has focus, the selected row also
  draws a 1 px inset ring in `teal` (the Kit's outset ring would be
  clipped by the list's `overflow_hidden` wrapper, fact 1). The region's
  own focus ring from Cycle 01 stays.
- Cells are its children. The row keeps the panel's existing id and
  `aria_label` (`ticket-row`, `doc-node`), so a screen reader reads the
  same name as today.

**States.** A pure function in `D/model/list_view.rs` turns a list's state
into what the panel body shows. Each state has its own text, so none is
shown by colour alone, and D1's distinct states stay distinct.

| State | Body | Text (Tickets; Documents reads "documents") | Action |
| --- | --- | --- | --- |
| No repository selected | message | "Select a repository." | — |
| First load (Loading, nothing kept) | six static skeleton rows | accessible "Loading tickets" | — |
| Reload (Loading, rows kept) | the kept rows | — | — |
| Ready, current, rows | rows | — | — |
| Ready, no rows, no narrowing filter | empty state | "No open tickets in {repo}." / "No closed tickets in {repo}." / "No tickets in {repo} yet."; Documents: "No documents in {repo} yet." | New ticket… (Open and All); New document… |
| Ready, no rows, narrowed by text or readiness | empty state | "No tickets match these filters." | Clear filters |
| Never refreshed | alert row above rows | "{repo} hasn't been scanned yet, so this list may be empty." | Refresh |
| Stale | alert row above rows | "The list for {repo} may be out of date." | Refresh |
| Incomplete | alert row above rows | "Some files in {repo} couldn't be read, so this list may be missing some." | Refresh |
| Busy, rows kept | alert row above the kept rows | "{repo} was busy, so this list wasn't updated." | Retry |
| Busy, nothing kept | alert row | "{repo} is busy right now." | Retry |
| Failed | alert row, no rows | "Couldn't read the tickets in {repo}. Nothing was changed." with the failure's message as its tooltip and description | Retry |
| Index unavailable | alert row, no rows | "The search index for {repo} can't be read. Nothing was changed." | Rebuild index |

- The alert row is one `ListRow`-height line with a `circle-alert` icon
  (amber for never refreshed, stale, incomplete and busy; red for failed
  and unavailable) and one action button. Never refreshed, stale and
  incomplete are checked in that order, as `ListState::label` does
  (fact 18).
- Refresh and Rebuild index call `maintain_index` (fact 28); Retry reads
  the list again, as today's Retry does; Clear filters empties the text
  and sets Any readiness; New ticket… and New document… are the toolbar's
  actions.
- Skeleton rows are plain bars in `theme.skeleton`, not the Kit
  `Skeleton`, which pulses (fact 9): `07` lets nothing but the spinner
  animate (R-6).

**Kept rows (R-17).** Each list model keeps the rows of its last ready
read for its repository, in a small `KeptList<T>` beside `ListState` in
`D/model/list_state.rs`. A reload shows them until it lands; a busy reload
keeps them under the busy alert; a failed reload replaces them with the
failure, as D1's ruling R-B5 says. A change of repository drops them.
Documents keeps its combined tree this way, not each of its two reads.

## Part B: The Tickets Model

**One read (R-1).** The panel reads `list_tickets` once per load, with
`TicketFilter::default()`: closure All, no readiness (fact 16). Lifecycle,
readiness, the text filter and the sort are applied in the desktop by a
pure function. Changing any of them never reads again and never leaves a
stale read to drop. The second read under Blocked goes: the blockers are
named from the same rows, which hold every ticket.

**`TicketRow`** gains `readiness` (Ready, Blocked or Closed),
`working_copy: Option<{ branch, worktree }>` (set when `context.kind` is
Active, fact 17) and `reasons` for every blocked row, not only under
Blocked. Its other fields are unchanged.

**`TicketQuery`** becomes the view's filter: `closure` (default Open),
`readiness` (default none), `text` (default empty) and `sort` (default
Updated, newest first). It no longer maps onto a library filter.

**`derive(rows, query) -> TicketView`** (`D/model/ticket_view.rs`):

1. **Text.** A row matches when the trimmed text is empty, or is a
   case-insensitive substring of its title or short code, or equals its ID
   ignoring case. Nonconforming entries always match (R-9).
2. **Counts.** `open`, `closed` and `all` count the conforming tickets
   that match the text, by lifecycle, so Open + Closed = All. The readiness
   choice does not change them (decision 6).
3. **Lifecycle and readiness.** Rows of the chosen lifecycle (All keeps
   both), then, when a readiness is chosen, those whose readiness state is
   that one. Nonconforming entries are kept under every choice (RFC
   67-71, "Filtering does not hide malformed entries").
4. **Sort (R-10).** A stable sort of the conforming rows over the library's
   order (fact 16), by the chosen column:
   - Code: by short code, then rows with none.
   - Title: case-insensitive.
   - Status: case-insensitive.
   - Updated: by age, so ascending is newest first (the prototype's
     `asc: true` with `arrow-up`, `Workspace.dc.html:732`, `:1005-1025`).
   - Descending reverses the order of the rows that have the value; rows
     missing it stay last either way.
   - Nonconforming entries follow, in the library's path order.
5. **Footnote** (decision 7), from the conforming rows shown, `n`:
   - `{n} open tickets in {repo}`, `{n} closed tickets in {repo}`, or
     `{n} tickets in {repo}` under All; "ticket" when `n` is 1.
   - Under Ready or Blocked the word is "ready" or "blocked":
     `{n} ready tickets in {repo}`.
   - With text: `… in {repo} match "{text}"`, the text trimmed.
   - Then ` · 1 needs fixing` or ` · {k} need fixing` when there are
     nonconforming entries.

**Age** (`age_label(changed_at, now)`, R-20): "now" under a minute,
`{m}m` under an hour, `{h}h` under a day, `{d}d` under 365 days, then
`{y}y`. A time after `now` is "now"; an unreadable or missing time is
blank. `now` is passed in, so tests set it.

**Markers** (`row_markers(…)` in `D/model/markers.rs`, shared with
Documents; R-8), from in-memory state only:

- **Editing** when the row has a working copy, or `held_by` answers
  `Yes`. `Unknown` shows no marker (decision 8).
- **Conflict** when the repository's pending conflicts name the row's ID.
- **Closed** when the row is lifecycle-closed and the lifecycle choice is
  All (decision 5).

**Status tone.** The status text is shown as written. Its pill takes a
tone from a case-insensitive exact match: "in progress" teal, "ready"
green, "blocked" red, "in review" amber, "closed" muted, any other value
neutral (`secondary`). A nonconforming entry shows "needs fixing" in
amber. The pill shows project status, never lifecycle or readiness (RFC
94-95).

## Part C: The Tickets Panel

Top to bottom inside `region-tickets`:

1. **Filter bar** (padding 6 8, bottom border):
   - **Segments.** The existing choices `filter-open`, `filter-closed` and
     `filter-all` (R-4), drawn as one segmented control: `Open 8 | Closed
     4 | All 12`, counts in `muted`, the active segment in `border` with
     `text`. Each stays a tab stop activated by Enter or Space, keeps
     `Role::Tab` and `aria_selected`, and its accessible name gains the
     count ("Open, 8 tickets").
   - **Readiness menu** (R-5; Wave decision 5). A `list-filter` icon button
     `filter-readiness` with `.dropdown_menu`: "Any readiness", "Ready",
     "Blocked", the current one checked. When Ready or Blocked is chosen
     the button also shows that word. Its accessible name is "Readiness:
     any", "Readiness: ready" or "Readiness: blocked", since the Kit does
     not expose the checked item (fact 8). Tooltip "Show ready or blocked
     tickets".
   - **Filter input** (decision 2; Part D). 24 px high, `search` prefix
     icon, placeholder "Filter by title or code", clearable.
2. **Column header** (24 px, bottom border; R-2): a blank 14 px type
   column, then buttons `sort-code`, `sort-title`, `sort-status` and
   `sort-updated` reading `CODE`, `TITLE`, `STATUS` and `UPD.` in 11 px
   `muted`. The active one is `text` with `arrow-up` or `arrow-down`;
   the others show a faint `arrow-up-down`. Activating a new column sorts
   it ascending; activating the active one reverses it. Each is a tab stop
   activated by Enter or Space, with `aria_selected` on the active one and
   an accessible name that says the order (no `aria_sort`, fact 11): the
   active column reads "Sort by code, A to Z" or "Sort by code, Z to A"
   (title and status alike), or "Sort by updated, newest first" or "Sort
   by updated, oldest first"; an inactive one reads "Sort by code".
   Tooltip "Sort by {column}". The header sits outside the scrolling
   list, so it stays in place.
3. **Body**: the state of Part A, or the rows.
4. **Rows**, a Kit `List` of `ListRow`s (R-2). Cells, in order:
   - **Type** (14 px): `repeat` for cycle, `bug` for defect, `sparkles`
     for feature, `list-checks` for task, `ticket` for any other type,
     `triangle-alert` in amber for a nonconforming entry. Id
     `ticket-type`, accessible name "Type {type}" or "Needs fixing".
   - **Code** (78 px, mono 11 px, `secondary`): the short code as stored,
     lowercase (decision 3), or `—` with accessible name "No short code"
     (decision 4). Id `ticket-slug`.
   - **Title** (flex, ellipsis), `secondary` for a closed ticket; a
     nonconforming entry shows its path. Then the markers, each an icon
     with its own id, tooltip and accessible name: `ticket-editing`
     (`pencil`, amber), `ticket-conflict` (`circle-alert`, red),
     `ticket-closed` (`circle-check`, `secondary`).
   - **Status** (92 px): the pill, id `ticket-status`, its text the
     status as written.
   - **Updated** (30 px, right-aligned, `muted`): the age, id
     `ticket-changed`, accessible name "Updated {relative_time}".
5. **Footnote** (11 px, `muted`, top border, ellipsis): Part B's text.
6. **Why blocked**, under Blocked only, as today (fact 13), below the
   list and above the footnote. The inline reasons line goes: rows are one
   line now, and the reasons are in Why blocked, the tooltip and the row's
   accessible description.

**Row tooltip** (R-14), in this order, each line only when it applies:

- `{code or "No short code"} · {repo}`, then `ID {ULID}` (decision 4;
  PRD MH-CONTENT-006 "alongside access to the canonical identifier").
- `Waiting on {…}` for a blocked ticket, from the header's existing text
  (fact 27).
- `Needs fixing: {guidance} Open it to fix.` for a nonconforming entry,
  with `ProblemCode::guidance()` of its first problem (decision 9).
- `Closed` for a closed ticket.

The same text is the row's `aria_description`. Marker tooltips:

- Editing, working copy: "Being edited in a private working copy on this
  computer." then `Git: branch {branch} · worktree {worktree}` (PRD
  MH-CONTENT-004; `06` keeps Git detail to tooltips).
- Editing, unsaved draft only: "You have unsaved changes to this."
- Both: the working-copy text, then "You also have unsaved changes."
- Conflict: "Changed here and in the shared copy. Open it to choose what
  to keep."
- Closed: "Closed".

**Selection** (R-7) stays the list's own: set by a click (which also
opens, fact 1) or the arrows, kept by ID across reloads and filter
changes, and shown with the selected treatment. It does not follow the
item area's tab. A filter that hides the selected row clears the
selection.

## Part D: The Filter Input Replaces Find

Decision 2. The design draws no Find in the Tickets panel, and its filter
"matches title and short code. Duplicate short codes show all matches"
(`03`:50). The input takes over Find's place, key and ULID path:

- It is the Workspace's existing `find_input` entity, kept under its name,
  with the new placeholder. `secondary-p` still reveals Tickets and focuses
  it (C1 P15).
- **Typing** sets `TicketQuery.text` at once (Change event), so the list,
  the counts and the footnote follow each keystroke. Rows matching a short
  code are all listed, duplicates included.
- **Enter:**
  - text that parses as a ULID opens that item through `show_item`,
    ticket or document, closed or not, whatever the filters, as Find did
    (fact 19; RFC "Opening a closed ticket directly by ID remains
    supported");
  - otherwise, when exactly one conforming row is listed, it opens;
  - otherwise focus moves to the list with its first row selected.
- **Escape** clears the text (`clean_on_escape`, fact 7).
- A change of repository clears the text, as it cleared Find. Lifecycle,
  readiness and sort carry over, as the filters do today.
- **Goes:** the match cards, their Open buttons and texts, the
  `find_tickets_by_slug` lookup, and `FindResult` (`D/view/find.rs`,
  `D/model/find.rs` apart from the ULID parse). `FindQuery::parse` keeps
  its ULID rule.

## Part E: The Documents Model

**Expansion (R-12).** `Documents` keeps, per repository for the session,
the set of folder paths the person has collapsed. A folder not in the set
is expanded, so a new folder starts expanded, as today (decision 13). The
tree's `TreeEvent::Expanded` and `Collapsed` update the set (fact 5).
Collapse all puts every folder of the current tree in it. Building the
tree sets each `TreeItem::expanded` from the set, so a reload or a hidden
panel keeps it. It is not written to disk (K-5).

**Row facts** (`doc_row(node, collapsed) -> DocRow`):

| Node | Label | Icon | Trailing | Tooltip and description |
| --- | --- | --- | --- | --- |
| Folder, expanded | last path segment | `folder-open` | — | its path |
| Folder, collapsed | last path segment | `folder` | descendant document count, `muted` | its path |
| Folder, empty (fact 23) | last path segment | `folder` | "empty", `muted` (decision 14) | "Empty folder. Shared once it contains a document." |
| Document | its title | `file-text` | markers | its path |
| Nonconforming, `missing_front_matter` alone | file name | `file-exclamation-point`, amber label | `triangle-alert` | "Not a managed document yet. Open to adopt it." then its path |
| Nonconforming, any other | file name | `file-exclamation-point`, amber label | `triangle-alert` | "Needs fixing: {guidance} Open it to fix." then its path |

- The adoptable test is the item view's own `unmanaged(item)`
  (`D/view/item/repair.rs:102-114`), moved unchanged to `D/model/item.rs`
  so the model can call it, so the tree and the banner agree (decision 9).
- The count counts documents under the folder, nonconforming included,
  and is shown only while the folder is collapsed, as in the prototype
  (`Workspace.dc.html:1037-1054`). It is `document_count` (fact 22).
- Markers: editing as in Part B; no conflict or closed marker (`03` lists
  editing and nonconforming only).
- Order is unchanged: folders first, then documents in path order
  (fact 22).

## Part F: The Documents Panel

- **Tree** (R-3): the base `Tree` with `ListRow` rows and the scrollbar
  the styled wrapper adds (fact 4), in `region-documents`.
- **Row**: indent `8 + 14 × depth` px; a 12 px chevron (`chevron-right`
  or `chevron-down`) for a folder with children, else a 12 px spacer;
  the icon at 14 px; the label; the trailing cell. Ids: `doc-node` (the
  row, `aria_label` the label, `aria_description` the tooltip text),
  `doc-count`, `doc-empty`, `doc-nonconforming`, `doc-editing`.
- **Mouse**: a click on a folder toggles it (the tree's own mouse-down,
  fact 5); a click on a document opens it.
- **Keys**: up and down (the tree's); Left collapses and Right expands
  (the tree's); Enter opens a document and now toggles a folder (R-24);
  Home and End select the first and last row (R-15).
- **States**: Part A, with the Documents texts. The header count stays
  documents listed (C1 R-22).
- **Selection** is restored by ID after a rebuild, as today (fact 21).
  When Collapse all hides the selected row, its top-level folder is
  selected instead.

## Part G: Panel Toolbars

`02` puts each panel's toolbar at the right of its header, and Cycle 01
left a header with no toolbar slot (fact 20; C1 Exclusions, "Panel
toolbars go into this Cycle's panel header").

- The header draws the toolbar of the panel it shows: for a single panel
  at the right of its title row; for a tabbed group, the active tab's at
  the right of the tab strip. The tab group renderer asks the panel for
  it (R-13).
- Buttons: 26 × 24 px, radius 4, 14 px icon in `secondary`; hover `hover`
  with `text`. Each is a tab stop activated by Enter or Space, with an
  accessible name and a tooltip:

| Panel | Id | Icon | Name and tooltip | Does |
| --- | --- | --- | --- | --- |
| Tickets | `new-ticket-button` | `plus` | "New ticket…", tooltip with Ctrl N | New ticket…, as today's body button |
| Documents | `new-document-button` | `file-plus` | "New document…" | New document…, as today |
| Documents | `new-folder-button` | `folder-plus` | "New folder…" | New folder…, as today |
| Documents | `collapse-all-button` | `chevrons-down-up` | "Collapse all folders" | Part E's Collapse all |

- The ids are today's body buttons' ids, so the tests that click them
  keep working; the body buttons go.
- No More button in this Cycle: nothing is designed to go in it, and the
  Wave leaves out what would be a stub (decision 12).
- Each handler is guarded by `unless_dialog`, like the footer's (C1 R-29),
  so nothing acts behind a dialog or sheet (C1 contract 4).

## Part H: Keyboard

- **Unchanged:** F6 and Shift F6, `secondary-1` to `secondary-4`,
  `secondary-n` (New ticket… or New document…, the focused list's),
  `secondary-r` and `secondary-shift-r`, up and down in both lists, Left
  and Right in the tree, Enter opening a ticket or document.
- **`secondary-p`** focuses the filter input (Part D).
- **Home and End** (R-15): new actions `SelectFirst` and `SelectLast`,
  bound in a new `TicketList` context around the Tickets list and in the
  existing `Documents` context. They select and scroll to the first or
  last row. In the filter input, Home and End stay the input's (its
  context is deeper).
- **Enter on a folder** toggles it (R-24).
- **Tab order** in Tickets: the toolbar, the segments, the readiness
  menu, the filter input, the four column headers, the list, Why blocked
  when shown. In Documents: the toolbar, then the tree.
- Every new control shows focus: Kit buttons and the input draw the Kit
  ring; segments and column headers draw the app's ring as today's
  choices do; the list's selected row draws Part A's inset ring.

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | A repository's tickets load | One `list_tickets` read with the default filter; no second read under Blocked |
| P2 | Open, Closed or All chosen; Ready or Blocked chosen; text typed; a column sorted | The list, counts and footnote follow at once; no read is sent |
| P3 | The default | Open, Any readiness, no text, Updated newest first: the library's order |
| P4 | A ticket whose status text is `closed` but lifecycle open | Listed under Open, its pill reads `closed` |
| P5 | Under All | A lifecycle-closed row has a `secondary` title and the Closed marker |
| P6 | Nonconforming entries | Listed under every lifecycle, readiness and text, last in path order under every sort, with "needs fixing" and the guidance |
| P7 | Two tickets share a short code; the code is typed | Both listed; each opens its own ticket by ID |
| P8 | A ULID typed, then Enter | That ticket or document opens, closed or not, whatever the filters |
| P9 | Enter with exactly one match; with several | It opens; focus moves to the list with the first row selected |
| P10 | Segment counts | Conforming tickets matching the text, by lifecycle; Open + Closed = All; readiness does not change them |
| P11 | Ready or Blocked chosen through the menu, by mouse and by keyboard | The rows of that readiness; the button names it; Why blocked under Blocked |
| P12 | A column header activated, by click and by Enter | Sorted ascending; again, reversed; the name says the order |
| P13 | A ticket with no short code | `—`; the tooltip and description carry its ID |
| P14 | A ticket with a private working copy; with only an unsaved draft; with a pending conflict | The editing marker with the right text; the conflict marker |
| P15 | Each list state of Part A, in both panels | Its own text and action; the action does what Part A says |
| P16 | A reload of a listed repository | The kept rows stay until it lands; a busy reload keeps them under the busy alert; a failed one shows the failure |
| P17 | A change of repository | The text filter is cleared; lifecycle, readiness and sort kept; kept rows dropped |
| P18 | Home and End in the Tickets list and in the tree | First and last row selected and scrolled to |
| P19 | A folder collapsed, then the list reloaded, then the panel hidden and shown | Still collapsed |
| P20 | Collapse all | Every folder collapsed; the selection moves to the hidden row's top-level folder |
| P21 | Enter, a click, Left and Right on a folder | Toggle, toggle, collapse, expand |
| P22 | A collapsed folder; an empty folder | Its document count; "empty" with its tooltip and description |
| P23 | A nonconforming document | Its file name in amber, the marker, and the adopt or fix text by its problem |
| P24 | A toolbar button by Tab and Enter, in a single panel and in a tabbed group | The same action as its click; refused behind a dialog |
| P25 | Any filter, sort or expansion change, or a toolbar Collapse all | No tab opens, closes or reloads; nothing is saved, synchronized or sent (C1 contract 7) |
| P26 | Hiding and showing either panel | Filters, text, sort, selection and expansion kept (C1 P2) |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.

- Between tasks, only the tests a task wrote or changed run, by exact
  name (AGENTS.md, Testing Strategy; decision 17).
- Behaviour, not appearance (Wave Testing): no tests for colours, sizes,
  icons, tones or that a view renders. The tone and icon mappings are not
  tested.
- Decision logic lives in pure functions with table-driven tests; the
  desktop tests are few, each a journey that also checks that nothing is
  read or sent.
- No real waits; `now` is passed in.

| Test | What |
| --- | --- |
| `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts` | One table: a fixture of open, closed, ready, blocked, no-code, duplicate-code and nonconforming rows under every lifecycle, readiness, text and sort it needs; the expected rows in order, counts, needs-fixing count and footnote (P2–P7, P10, P13) |
| `desktop::model::ticket_view::tests::ages_read_as_short_units` | One table of `age_label` (R-20) |
| `desktop::model::markers::tests::markers_follow_the_copy_drafts_and_conflicts` | One table of `row_markers`: working copy, `Held` Yes, No and Unknown, a conflict, closed under each lifecycle (P5, P14) |
| `desktop::model::ticket_view::tests::tooltips_carry_the_id_and_the_reason` | One table of `ticket_tooltip`: a row with and without a code, blocked, nonconforming, closed (P13) |
| `desktop::model::list_view::tests::each_state_has_its_own_body` | One table: every row of Part A's state table for each panel, with its text and action (P15) |
| `desktop::model::list_state::tests::kept_rows_follow_reloads` | One table of `KeptList`: reload, then ok, busy or failed; a change of repository (P16, P17) |
| `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all` | Toggle, rebuild with a new folder and with a folder gone and back, Collapse all; the set and the built `expanded` flags (P19, P20) |
| `desktop::model::documents::tests::rows_read_as_the_design_writes_them` | One table of `doc_row` over Part E's six kinds (P22, P23) |
| `desktop::view::tickets::tests::the_filter_bar_works_without_a_read` | A journey: segments, the readiness menu by keyboard, text, a sort by Enter, Home and End; rows, counts and footnote at each step; no read or request sent (P2, P11, P12, P18, P25) |
| `desktop::view::find::tests::enter_in_the_filter_opens_an_id_or_the_only_match` | A closed ticket's ULID under Open; a document's ULID; one match; several matches (P8, P9) |
| `desktop::view::tickets::tests::a_reload_keeps_the_rows_and_a_busy_one_says_so` | A job-driven reload; a held busy reload (P16) |
| `desktop::view::documents::tests::folders_keep_their_state_and_toggle_by_key` | Left, Right, Enter, a click; a reload; hide and show; Collapse all from the toolbar; Home and End (P18–P21, P26) |
| `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter` | Each toolbar button reached by Tab and run by Enter, single and tabbed; refused behind a dialog (P24) |

Pre-existing tests change as the named edits say.

## Named Edits To Pre-Existing Tests

At `a0a734a4`. Task 0 re-counts each with `git grep` and records any
other site as a further edit (decision 16).

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `D/model/tickets_tests.rs`: `the_query_maps_closure_and_readiness_and_nothing_else`, `a_filter_change_drops_the_last_filters_read`, `set_query_keeps_the_query_and_drops_the_last_read`, `a_readiness_change_drops_the_last_readinesss_read`, `rows_read_under_blocked_carry_their_reasons` | The query no longer maps to a library filter and filter changes no longer read (R-1). These five are **deleted**; their behaviour moves to `derive_filters_counts_and_sorts` (rows and reasons) and to P2's "no read" check. `the_default_query_lists_open_tickets` asserts the default query and that the read's filter is `TicketFilter::default()`. **This removes assertions** | B |
| E-2 | `D/view/tickets_tests.rs`: `a_filter_change_drops_the_held_read_of_the_last_filter`, `a_readiness_change_drops_the_held_read_of_the_last_one` | Same reason; **deleted**. The held-read guard stays tested for a change of repository by the existing stamp tests. **This removes assertions** | B |
| E-3 | `D/view/tickets_tests.rs`: `each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`, `ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry`, `the_why_blocked_panel_gives_the_selected_rows_reasons_in_full`, `the_panel_says_incomplete_where_the_library_does`, `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text`, `a_long_why_blocked_panel_scrolls_by_keyboard` | `ticket-lifecycle` text becomes the `ticket-closed` marker's presence under All; readiness is chosen through `filter-readiness`'s menu; `ticket-reason-{i}-{n}` becomes the row's description and Why blocked's lines; the "Showing …" texts are gone, so the label test checks the segments' names with counts and the menu button's name. **The "Showing …" assertions are removed** | C |
| E-4 | `D/view/find_tests.rs` (8) | Rewritten for the filter input: `secondary_p_focuses_find` and `find_does_nothing_while_a_dialog_is_open` keep their checks; `a_ulid_opens_its_ticket_closed_or_not`, `no_match_says_so_and_opens_nothing` (now the "No tickets match these filters." state) and `find_needs_a_selected_repository_and_a_new_selection_clears_it` (the text is cleared) keep their intent; `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` lists both as rows. `a_held_search_never_lands_after_the_selection_changed` and `a_matchs_open_is_reached_by_tab_and_enter_opens_it` are **deleted** (no slug lookup, no match cards); the cards' context labels are not asserted any more. **This removes assertions** | D |
| E-5 | `D/model/find_tests.rs` | The slug half goes with `find_tickets_by_slug`'s use; the ULID parse tests stay. **This removes assertions** | D |
| E-6 | `D/view/keyboard_tests.rs` `change_the_filters` (`:345`), `find` and `find_by_short_code_and_ulid` (`:430`, `:450`) | New Tab order (Part H); readiness through the menu; "Showing …" checks go; Find steps use the filter input | C, D |
| E-7 | `D/view/keyboard_tests/editing.rs` `create_by_keyboard_alone` (`:709-730`), `move_and_new_folder_by_keyboard_alone`; `D/view/documents_tests.rs` `new_folder_opens_its_dialog` | Shift-Tab from the list now reaches the column headers or the tree's toolbar in the header; the steps reach `new-ticket-button`, `new-document-button` and `new-folder-button` in the header | G |
| E-8 | `D/view/documents_tests.rs` `the_tree_shows_what_the_library_reads_give`; `D/view/new_folder_tests.rs` (`doc-empty` text); `D/view/move_document_tests.rs` helper `tree_paths` (`:274-279`) | The nonconforming label is the file name; `doc-path`, `doc-nonconforming` and `doc-problem` lines become the row's description and marker; "Empty folder" becomes "empty"; `tree_paths` reads each `doc-node`'s description | E, F |
| E-9 | Keyboard helpers that open a nonconforming file by its path label: `keyboard_tests/actions.rs` `a_nonconforming_entry_opens_by_keyboard`, `keyboard_tests/editing.rs` `repair_and_adopt_by_keyboard_alone` | They look for the file name | E |
| E-10 | `D/view/dock_tests/state.rs` `a_hidden_panel_keeps_its_state`; `D/view/lifecycle/made_ready_tests.rs`, `made_ready_review_tests.rs` (`filter-ready` selected) | Find text is the filter text; Ready is read from the menu button's name, or the model | C, D |
| E-11 | `D/assets_tests.rs` `every_item_view_icon_loads` | The table gains Part A–G's new icons (fact 12) | A |
| E-12 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | The table gains `home` and `end` in `TicketList` and `Documents` | H |
| E-13 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour and stops for the owner | — |

## Document Amendments

To apply in the approval commit:

1. **Wave Cycle 02.**
   - In scope, Tickets: the footnote reads as decision 7; "Ready and
     Blocked filters keep working, in a filter menu" adds "Why blocked
     stays under Blocked"; add "Find becomes the filter input (decision 2)".
   - Exit evidence: "nonconforming and inaccessible items stay visible with
     their reason" adds "an inaccessible file shows as the incomplete
     alert; per-item inaccessible rows need a library read (decision 10)".
   - Toolbar: "New ticket, More" reads "New ticket (More waits for an
     action, decision 12)".
2. **Cycle 01 document, decision 4.** Add: "Superseded in part by Wave 04
   Cycle 02 decision 2: Find is now the Tickets filter input; `secondary-p`
   and the ULID path are kept."
3. **Handoff `08`.** The Tickets list is a Kit `List` with an app row and
   an app column header, not `DataTable` (fact 3); the Documents tree is
   the base `Tree` with app rows (fact 4); the segmented control is the
   app's own (fact 6); `file-warning` is `file-exclamation-point`
   (fact 12).
4. **Handoff `03`, Tickets.** Short codes appear as stored, in lowercase
   (decision 3).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | One `list_tickets` read with the default filter per load; lifecycle, readiness, text and sort are applied in the desktop; the names read goes (fact 16) | A large repository reads its closed tickets on every reload (K-10) |
| R-2 | Tickets: a Kit `List` with `ListRow` items and the app's column header outside it; not `DataTable` (fact 3) | — |
| R-3 | Documents: the base `Tree` with `ListRow` rows and the wrapper's scrollbar; not `component::tree`, whose `ListItem` overrides the selected background (facts 2, 4) | The Kit's context-menu hook on tree rows is not used (none is configured today) |
| R-4 | The segments are the app's existing choice controls, restyled; the Kit's segmented controls have no keyboard path (fact 6) | — |
| R-5 | The readiness menu is a `Button::dropdown_menu` with checked items; the button names the choice (fact 8) | A screen reader hears the choice on the button, not on the item (K-6) |
| R-6 | Skeleton rows are static bars in `theme.skeleton`, not the pulsing Kit `Skeleton` (fact 9; `07`:68) | — |
| R-7 | `ListRow` draws hover, the selected treatment and, while its list has focus, an inset ring on the selected row. Selection is the list's own, not the open tab (prototype `:1002`) | The open item is not highlighted in the list unless selected there |
| R-8 | Markers are computed when a row is drawn, from in-memory state only: the row's working copy, `held_by`, the repository's pending conflicts. `Held::Unknown` shows none | A draft in an unreadable store shows no marker |
| R-9 | Nonconforming entries pass every filter, the text included, and sort last in path order | — |
| R-10 | The sort is stable over the library's order; rows missing the value sort last in either direction | — |
| R-11 | Header counts stay C1 R-22 (rows listed) | — |
| R-12 | Expansion per repository, session only, as a set of collapsed folder paths; absent means expanded; `TreeEvent` updates it; `set_items` builds each item's `expanded` from it | Expansion is lost at restart (K-5) |
| R-13 | The tab group renderer asks the shown panel for its toolbar; `RegionPanel` builds it from the Workspace, like its body (C1 R-1). The ids are the old body buttons' | — |
| R-14 | Every tooltip's text is also the element's accessible name or description; tooltips are hover-only (fact 10) | A sighted keyboard user does not see a tooltip (K-1) |
| R-15 | `SelectFirst` and `SelectLast` on `home` and `end` in the `TicketList` and `Documents` contexts | — |
| R-16 | Text matching: trimmed, case-insensitive substring of title or short code, or the whole ID | — |
| R-17 | Each list keeps its last ready rows for its repository: shown during a reload and over a busy read; replaced by a failure (R-B5); dropped on a change of repository | — |
| R-18 | Why blocked stays as today under Blocked; reasons come with every blocked row | — |
| R-19 | Short codes as stored, lowercase (RFC relationships :103-107) | Unlike the handoff's frames |
| R-20 | Ages: now, `{m}m`, `{h}h`, `{d}d` under 365 days, `{y}y`; blank when unknown | — |
| R-21 | New icons join `ItemIcons`; `file-exclamation-point` stands for `file-warning` | — |
| R-22 | The footnote counts conforming rows shown and names needs-fixing separately (decision 7) | — |
| R-23 | An empty folder shows "empty" where a count would be (decision 14) | — |
| R-24 | Enter on a folder toggles it, as a click does | — |

## Known Limits

- **K-1.** Tooltips show on hover only (fact 10). Their text is also each
  element's accessible name or description, and the opened item shows the
  ID, path and context.
- **K-2.** Columns have fixed widths. In a narrow dock, such as the left
  one at 290 px, the title column is short and ellipsized.
- **K-3.** Ages are worked out when a row is drawn, so an idle list's ages
  do not tick on their own.
- **K-4.** An unreadable file is not a row: the library lists none, so it
  shows as the incomplete alert (decision 10).
- **K-5.** Folder expansion and the sort last for the session only; they
  are not saved with the layout (Wave Deferred Work).
- **K-6.** The readiness menu's checked item is not exposed to
  accessibility; the button's name says the choice (fact 8).
- **K-7.** The sort order is in the header button's name; there is no
  `aria_sort` (fact 11).
- **K-8.** Up and Down wrap at both ends, as the Kit does (facts 1, 5).
- **K-9.** New document… still starts in `docs`, not the selected folder,
  as today.
- **K-10.** Every reload reads every ticket of the repository, closed ones
  included (R-1).

## Unverified

- **U-1.** That a Kit `List` with a non-`ListItem` item draws its own
  hover and selected treatment with nothing of the Kit's on top (fact 1,
  inferred from `C/list/list.rs:490-502`). Task 4's first test proves it.
- **U-2.** That `.vertical_scrollbar` is reachable for the base tree from
  the app's `gpui_kit` imports, as the wrapper uses it (fact 4). Task 7
  checks it.
- **U-3.** That the tab group renderer can draw Workspace-built toolbar
  buttons, reached by Tab before the panel body (R-13). Task 6's test
  proves it.
- **U-4.** That a `dropdown_menu` button returns focus to itself when its
  menu closes by Escape (fact 8). Task 5's test proves it.

## Alternatives Rejected

- **`DataTable` for Tickets:** no keyboard sort, no Enter, Tab trapped
  inside, Home and End select columns (fact 3).
- **The styled `tree()` for Documents:** its `ListItem` overrides the
  design's selected background (facts 2, 4).
- **Keeping Find beside the filter input:** two inputs that both match
  short codes, and the design draws none (decision 2).
- **Reading again for each filter:** no counts for the other segments, a
  second read under Blocked, and a stale-read race per change (R-1).
- **The Kit `Skeleton`:** it animates, which `07` reserves for the
  spinner (R-6).
- **Following the open tab with the list selection:** the selection would
  jump when the person switches tabs while using the arrows (R-7).

## Risks

| Risk | Control |
| --- | --- |
| The redesign reaches many existing tests (labels, ids, Tab order) | E-1 to E-13; ids kept where the element survives; Task 0's sweep; each task runs the tests it touches by exact name |
| One unfiltered read is slower on a large repository | The library already loads every stored item for any filter (`items.rs:1712-1724`); K-10; the performance work the owner put next can add a count read |
| Custom rows lose the Kit's list accessibility | The Kit `List` and base `Tree` keep their roles, positions and selected state on the row wrapper (facts 1, 4); rows keep their ids and names |
| Find's removal loses a path to closed tickets and documents by ID | Part D keeps the ULID path on Enter (P8) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | The design's segment counts need every lifecycle; today each filter is its own read | Ruling | R-1 | Fact 16 |
| Planning research | Find and the design's filter input both match short codes | Decision (proposed) | Decision 2, Part D | Fact 19; `03`:50 |
| Planning research | The handoff shows upper-case codes; the RFC displays lowercase | Decision (proposed) | Decision 3 | RFC relationships :103-107; frames |
| Planning research | No short code: the design's `—` vs the RFC's ULID | Decision (proposed) | Decision 4 | `03`:58; RFC relationships :132-134 |
| Planning research | Closed shown by colour alone under All | Decision (proposed) | Decision 5 | `03`:63; README :40; D1 design :414-416 |
| Planning research | Header count, segment counts and footnote disagree between Wave, design, prototype and C1 | Decision (proposed), ruling | Decisions 6, 7; R-11 | Prototype :988-1030; C1 R-22 |
| Planning research | "Not shared yet" is not known per item; the PRD asks for the context's branch and worktree | Decision (proposed) | Decision 8 | PRD MH-CONTENT-004; RFC IA :270-274 |
| Planning research | A nonconforming entry's field is not named; adopt fits only `missing_front_matter` | Decision (proposed) | Decision 9 | Fact 17; D3 design :2560-2562; `D/view/item/repair.rs:102-114` |
| Planning research | No per-item inaccessible entries exist | Decision (proposed) | Decision 10 | Fact 17; `items.rs:118-120` |
| Planning research | `03`'s four states vs D1's eight; no copy in `06` | Decision (proposed) | Decision 11, Part A | Fact 18; D1 :113-115 |
| Planning research | Kit list, tree, table, segmented and menu facts | Rulings | R-2 to R-6 | Facts 1–9 |
| Planning research | Expansion lost on every reload | Decision (proposed), ruling | Decision 13, R-12 | Fact 21 |
