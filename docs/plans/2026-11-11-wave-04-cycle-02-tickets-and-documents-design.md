---
title: "Wave 04 Cycle 02 Tickets And Documents Panels Design"
date: 2026-11-11
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M75C77FPAF6TY24045SBF4AW"
---

# Wave 04 Cycle 02 Tickets And Documents Panels Design

## Intent And Approval Boundary

This design covers [Cycle 02](../Cycles/wave-04-cycle-02-tickets-and-documents.md):
the Tickets and Documents panels rebuilt to `docs/design/03-panels.md`.
That means the common row anatomy and the list states, then the Tickets
toolbar, filter bar, column header, rows and footnote, then the Documents
toolbar and tree. Approval covers the mechanisms, rulings, named test
edits and task order. It does not authorize implementation, pushes beyond
the ticket branch, merging or closing.

**Draft, revision 2.** Grounded at the ticket branch `a0a734a4`, which
holds origin/main `48653d01` (Cycle 01 and Wave 03 D6 both merged).
Revision 1 (`563763e2`) was reviewed by two independent reviewers, one for
source and feasibility and one for contract and consistency; revision 2
applies their findings (Review Record).

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
  Where this design departs from it, the ruling says so.
- `{repo}` is the repository's display name, the one the Repositories
  panel and the window title show (C1 R-14).

## Baseline Facts At `a0a734a4`

**Kit lists (0.6.6).**

1. **Kit `List`.** `ListState::new(delegate, window, cx)`
   (`C/list/list.rs:94`). The delegate's `type Item: Selectable +
   IntoElement` need not be `ListItem` (`C/list/delegate.rs:11`). Every
   row has the height measured from one row (`C/list/list.rs:69`,
   `:429-451`). Each row is wrapped in `div().overflow_hidden()`, given
   `.selected()`, and drawn with nothing of the Kit's own on top
   (`:490-502`, `:666-716`); the container has `Role::List` and the
   wrapper `Role::ListItem` with position and selected state (`:474-501`,
   `:790`). A click selects and emits `Confirm` (`:503-514`). Keys in the
   "List" context are escape, enter, secondary-enter, up and down; up and
   down wrap (`:26-35`; `C/list/cache.rs:107-170`). There is no Home, End
   or Page key. `set_selected_index` emits no event (`:184-192`);
   `scroll_to_selected_item` exists (`:263`). The list's focus handle is
   not a tab stop, and a clone cannot make it one (`:102`;
   `G/window.rs:546-547`; `G/tab_stop.rs:78-89`).
2. **Kit `ListItem`** applies its selected background (`list_active`, or
   `accent`) after the caller's style, so a caller cannot replace it
   (`C/list/list_item.rs:193`, `:234-246`). Calling `.hover()` on it is
   unsupported (`:165-167`).
3. **`DataTable`** (`C/table/`) has a sticky header and per-column sort,
   but: sorting is by clicking a small icon only, with no keyboard path
   (`C/table/state.rs:1561-1580`); Enter has no binding
   (`C/table/data_table.rs:17-29`); Tab and Shift-Tab move between
   columns inside the table, so Tab cannot leave it
   (`C/table/state.rs:1034-1109`; `G/keymap.rs:154-159`); Home and End
   select columns (`C/table/state.rs:925-967`).
4. **Tree.** `gpui_kit::base::Tree::new(&state).item(Fn(usize, &TreeEntry,
   TreeEntryState, &mut Window, &mut App) -> AnyElement)` takes any element
   and draws nothing of its own (`B/tree.rs:466-497`). The styled
   `component::tree::tree` forces a `ListItem` row (`C/tree.rs:18-23`,
   `:83-90`) and adds `.vertical_scrollbar(&scroll_handle)` around the
   base tree (`:111`); `component::scroll::ScrollableElement` is public
   (`C/scroll/mod.rs:9`) and `UniformListScrollHandle` is a
   `ScrollbarHandle` (`B/scrollbar.rs:102`). Rows are a `uniform_list`
   (`B/tree.rs:423`). `TreeEntry::depth()` (`:66`);
   `TreeEntryState::is_selected()` (`:164-178`). Each row's wrapper has
   `Role::TreeItem`, `aria_label`, `aria_selected` and, for folders,
   `aria_expanded` (`:432-441`). Its focus handle is not a tab stop, and
   has no getter (`:199`).
5. **Tree expansion and selection.** Expansion lives in each `TreeItem`
   (`B/tree.rs:34-46`, `:121-124`); `TreeItem::expanded(bool)` sets it.
   `is_folder()` is `!children.is_empty()`, so a folder with no child
   nodes is a leaf that cannot toggle (`:131-134`). `set_items` rebuilds
   rows, clears the selection and emits nothing (`:214-219`).
   `TreeEvent::{Expanded(id), Collapsed(id)}` are emitted on toggles by
   key or mouse (`:93-96`, `:305`, `:331-335`). Keys in the "Tree" context
   are up, down, left and right only; up and down wrap; left collapses and
   right expands without moving (`:20-27`, `:365-411`). `toggle_expand`
   and the `Confirm` handler are private, and enter is not bound
   (`:320`, `:350-363`). Mouse-down selects and toggles a folder, and
   does nothing else (`:413-417`, `:443-447`). `set_selected_index`,
   `index_of(id)`, `scroll_to_item(ix, strategy)` exist (`:221-266`).
6. **Segmented controls.** `ButtonGroup`, `ToggleGroup::segmented` and
   `TabBar::segmented` exist, but none has arrow keys; `TabBar` tabs are
   not focusable at all (`B/tabs.rs:17-21`). Any focused element with
   `on_click` is activated by Enter and Space (`G/elements/div.rs:3010-3066`).
7. **Input.** `InputState::new(window, cx).placeholder(…)`;
   `InputEvent::{Change, PressEnter, Focus, Blur}`; `set_value` needs a
   `Window` and emits no Change; `.clean_on_escape()` clears on Escape,
   else Escape propagates (`B/input/base/state.rs:766`, `:897-915`,
   `:1086`, `:2010-2044`). `Input::new(&state).prefix(…).cleanable(true)`
   (`C/input/input.rs:240`, `:281`).
8. **Menus.** `Button::dropdown_menu(f)` opens a `PopupMenu` on left
   mouse-down or on Enter and Space, and the menu takes focus
   (`C/menu/dropdown_menu.rs:12-34`, `:150`; `B/popover.rs:17-22`,
   `:288-309`). The popover saves the focused handle when it opens and
   restores it when it closes, by Escape or by an item
   (`B/popover.rs:98-124`). `PopupMenuItem::new(label).checked(bool).on_click(…)`
   (`C/menu/popup_menu.rs:180-210`). A checked item is not exposed to
   accessibility (`C/menu/menu_item.rs:99-101`).
9. **Skeleton.** `component::skeleton::Skeleton` pulses with a repeating
   two-second animation (`C/skeleton.rs:39-57`).
10. **Tooltips** appear on hover only, never on keyboard focus
    (`G/elements/div.rs:1674-1702`; `C/tooltip.rs:247-292`).
    `Button::tooltip_with_action` shows the platform's key for an action
    (`C/button/button.rs:404-418`).
11. **Accessibility.** `aria_label`, `aria_description`, `aria_selected`,
    `aria_expanded`, `aria_toggled` exist on stateful elements
    (`G/elements/div.rs:1304-1497`). `aria_sort` does not exist. The test
    snapshot exposes role, label, checked, selected, expanded, value,
    bounds, visible, focused and disabled, but **no description**
    (`B/test_support.rs:17-31`, `:302-318`).
12. **Icons.** Of the design's icons, `repeat`, `bug`, `sparkles`,
    `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`,
    `arrow-up-down`, `list-filter` and `circle-x` are in the Kit catalog
    but not loaded by the app; `file-warning` does not exist, and
    `file-exclamation-point` does (`A/build.rs:19-67`). `circle-check` and
    `sliders-horizontal` are already loaded. The app serves extra icons
    through `icon_assets!(ItemIcons, […])` (`D/assets.rs:17-58`); an icon
    not listed there or in the Kit defaults fails to load
    (`A/src/native_assets.rs:29`).

**Tickets today.**

13. **Body.** Find (drawn whether or not a repository is selected), then,
    with a repository, New ticket…, the closure choices
    `filter-open|closed|all` with a "Showing …" line, the readiness
    choices `filter-any-readiness|ready|blocked` with their line, the list
    state line, the list `tickets-list`, and Why blocked under Blocked
    (`D/view/lists.rs:53-152`). Each choice is a `Role::Tab` div with its
    own focus handle around a Kit `Button` (`:155-177`;
    `D/view/workspace.rs:301-303`).
14. **Rows.** A Kit `List` of `ListItem`s: the title on line 1; short code
    or "No short code", type, status, "Open" or "Closed", and the raw
    RFC 3339 change time on line 2; a reasons line on every row whenever
    any row has reasons (`D/view/tickets.rs:112-210`, `:265`). The row's
    `aria_label` is the title, or the path for a nonconforming entry, and
    its `aria_description` the reasons joined by "; " (`:96-110`,
    `:197-204`). `LIFECYCLE_OPEN`, `LIFECYCLE_CLOSED` and
    `lifecycle_label` are also used by the item view, the details sheet
    and the relationships panel (`D/view/item/details.rs:31`,
    `item/sheet/ticket.rs:31`, `D/view/relationships.rs:29`).
15. **Query and read.** `TicketQuery { closure, readiness }` is `Copy`,
    maps onto the library's `TicketFilter` through `to_filter`, and every
    filter change reads again (`D/model/tickets.rs:33-57`;
    `D/view/workspace/lists.rs:150-168`). Under Blocked a second read lists
    every open ticket to name blockers (`D/model/tickets.rs:59-67`). The
    desktop does not sort (`:310-322`). `open_ready_tickets` sets Open and
    Ready and reads (`D/view/workspace/lists.rs:174-199`).
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
    (`L/repository/read/dto.rs:347-356`, `:425-478`). Active means the
    item has its own worktree (`items.rs:1526-1553`); in this repository
    every open ticket has one. `TicketRow` keeps neither `readiness.state`
    nor `context.kind` (`D/model/tickets.rs:145-194`). A nonconforming
    entry has no ID, title or metadata, only a path and problem codes, and
    its field is not named (`items.rs:528-532`, `:965-980`;
    `L/results.rs:785-799`).
18. **List states.** `ListState::{Loading, Ready { rows, complete, index },
    Busy, Failed }`, with labels for loading, empty, never refreshed,
    stale, incomplete, busy, failed and index unavailable
    (`D/model/list_state.rs:16-52`, `:101-125`). `Tickets::load` resets the
    list (`forget`, `D/model/tickets.rs:279-284`); `apply_keeping` keeps
    rows over a busy read and replaces them over a failure (ruling R-B5,
    `D/model/list_state.rs:72-98`). A job-driven reload keeps the old rows
    on screen while the model says Loading
    (`D/view/workspace/lists.rs:36-46`, `:117-127`). Today a failure's
    message is a visible line under the state label
    (`D/view/lists.rs:208-222`).
19. **Find** sits at the top of the Tickets panel (C1 decision 4).
    `secondary-p` reveals Tickets and focuses it. Enter on a ULID opens
    that item, ticket or document, closed or not, through an asynchronous
    `show_item` read that a stamp drops when the selection changes, with
    "Finding", "No match", busy and failure states; other text is looked
    up as a short code with `find_tickets_by_slug`, and every match is
    listed as a card with its context and an Open button
    (`D/model/find.rs:32-116`; `D/view/workspace/find.rs:24-85`;
    `D/view/find.rs:44-171`). Focus stays in the input after a ULID opens
    (`D/view/keyboard_tests.rs:439-443`).
20. **Panel header.** The header is the dock's tab group renderer, outside
    every region's focus subtree; `region()` puts `track_focus` and the
    `Navigation` key context on the panel body only
    (`D/view/workspace/focus.rs:226-255`). The renderer draws icon, title
    and count (`title_suffix`, with no element id) and no toolbar; the Kit
    `Panel` has a `toolbar_buttons` hook the app does not draw
    (`D/view/dock/header/tabs.rs:116-165`; `C/dock/panel.rs:101-107`).
    `header()` holds the Workspace's `WeakEntity`; `single` and `tabs` do
    not receive it (`D/view/dock/header.rs:79-118`). The count is the
    number of rows of `tickets().state()` (C1 R-22;
    `D/view/dock/panels.rs:166-180`).
21. **Region focus.** `focused_region` is the region whose handle holds
    focus, else none (`D/view/workspace/focus.rs:134-138`); F6 from none
    goes to the first stop (`:204-221`). The keyboard walk asserts that
    exactly one region holds focus and draws its ring
    (`D/view/keyboard_tests.rs:150-158`, `:618-629`).

**Documents today.**

22. **Tree.** The styled Kit `tree()` of `ListItem`s, indented 12 px per
    level, with no chevrons, icons or tooltips (`D/view/documents.rs:129-221`).
    A document shows its title and its path on a second line; a
    nonconforming file its path, "Nonconforming" and its first problem
    code; an empty folder "Empty folder" (`:36-48`, `:148-199`). A document
    opens through `ListItem::on_click` (`:203-213`). Every folder is
    rebuilt expanded on every reload (`:286`), so expansion does not
    survive one. Selection is restored by id (`:81-96`).
23. **Model.** `TreeNode::{Folder { path, empty, children }, Document(ItemDto)}`,
    folders first in the folder read's order, documents in the library's
    path order; a document whose folder was not listed goes under the
    nearest listed ancestor (`D/model/documents.rs:23-127`).
    `document_count` counts documents, not folders (`:37-46`). Two reads,
    folders and documents, combine into one state (`:227-253`); a failed or
    busy reload clears the tree (`D/view/workspace/lists.rs:95-103`).
24. **Empty folders.** `DocumentFolderDto.empty` means the directory holds
    nothing at all, not even a non-document file or a subfolder; folders
    come from the primary worktree's index only
    (`L/repository/read/dto.rs:389-417`).
25. **Keys.** Up, down, left and right are the Kit's; Enter is the app's
    `OpenSelected` in the `Documents` context and opens a document,
    ignoring a folder (`D/keys.rs:145-147`, `:255`;
    `D/view/workspace/lists.rs:201-205`). New document… and New folder…
    are body buttons (`D/view/documents.rs:226-242`).

**Shared.**

26. **Unsaved work** is not on `ItemDto`. In memory,
    `DraftRegistry::held_by(repo, kind, id) -> Held { Yes, No, Unknown }`
    answers whether a live session or the cached store listing holds a
    draft of the item **or of a comment on it**, with no I/O
    (`D/services/drafts/held.rs:126-170`).
27. **Pending conflicts** are kept per repository as
    `Workspace.pending_conflicts: HashMap<RepoKey, Vec<PendingConflict>>`,
    each naming its item's ID and kind and a source that is a sync, a
    local merge, a close or a promotion (`D/view/workspace.rs:141-145`;
    `D/services/pending_conflicts.rs:40-67`).
28. **Times and statuses.** `relative_time(at, now)` gives "just now",
    "{n} min ago" … "{n} days ago", then the date
    (`D/view/item/chips.rs:180-206`), private to `view::item`. The item
    header tints a status by `tint(status)`: "open", "todo", "in
    progress" and kin as working; "done", "closed", "complete" and kin as
    done; "draft", "review", "in review" as draft; "blocked"; else none
    (`chips.rs:155-167`, `:354-357`).
29. **Index maintenance.** `Workspace::maintain_index(rebuild, cx)` submits
    `IndexRefresh` or `IndexRebuild` for the selected repository as a user
    job (`D/view/workspace/requests.rs:333-350`), bound to `secondary-r` and
    `secondary-shift-r` (`D/keys.rs:240-241`). A finished job reloads the
    lists.
30. **Unreadable files.** A list's `complete` is false only when a
    *directory* holding its kind could not be read; an unreadable `.md`
    file is neither a list entry nor an incomplete list
    (`L/repository/read/items.rs:117-120`, `:864-883`). The index status
    read, `index_status`, is the only read that reports it, as an
    `IndexProblemDto { code: SourceUnreadable, path, worktree }`
    (`L/repository/read/dto.rs:643-668`; `L/repository/read/status.rs:669`).
    The desktop does not call it today. A refresh meets the same
    unreadable place again (`dto.rs:376-385`).
31. **Chrome tokens** `selection`, `accent` (amber), `hover`, `muted`,
    `secondary`, `text`, `border`, `teal`, `green` and `red` exist in both
    modes (`D/view/shell/tokens.rs:12-76`).

## Part A: Common Row Anatomy And List States

**Row (`ListRow`).** One element in `D/view/rows.rs` for both panels. It
implements the Kit's `Selectable` (`B/component_traits.rs:3-10`), so the
Kit `List` can use it as its item (fact 1), and the tree draws it directly
(fact 4).

- 28 px tall, one line, padding `0 10 0 12` (the tree sets its own left
  indent), gap 8 in Tickets and 5 in Documents (prototype), 13 px text.
- Hover: `hover` background.
- Selected: `selection` background, a 2 px inset left line in `accent`,
  the label at weight 600, and `aria_selected`.
- Keyboard focus: while its list has focus, the selected row also draws a
  2 px inset ring in `teal` (`07`:58; the Kit's outset ring would be
  clipped by the list's `overflow_hidden` wrapper, fact 1). The region's
  own focus ring from Cycle 01 stays.
- It takes an `on_click` (a tree row opens a document through it; fact 5).
- The row keeps the panel's existing id and `aria_label` (`ticket-row`,
  `doc-node`), so a screen reader reads the same name as today and tests
  keep finding rows by label.

**States.** A pure function in `D/model/list_view.rs` turns a list's
state into what the panel body shows. Each state has its own text, so
none is shown by colour alone, and D1's distinct states stay distinct.
`listed` is the number of rows the panel would draw: conforming,
nonconforming and unreadable rows in Tickets; every node in Documents.

| State | Body | Text (Tickets; Documents reads "documents") | Action |
| --- | --- | --- | --- |
| No repository selected | message | "Select a repository." | — |
| First load (Loading, nothing kept) | six static skeleton rows | accessible "Loading tickets" | — |
| Reload (Loading, rows kept) | the kept rows; the header count reads "Updating…" | — | — |
| Ready, current, `listed` > 0 | rows | — | — |
| Ready, `listed` = 0, no narrowing filter | empty state | "No open tickets in {repo}." / "No closed tickets in {repo}." / "No tickets in {repo} yet."; Documents: "No documents in {repo} yet." | New ticket… (Open and All); New document… |
| Ready, `listed` = 0, narrowed by text or readiness | empty state | "No tickets match these filters." | Clear filters |
| Never refreshed | alert row above rows | "{repo} hasn't been scanned yet, so this list may be empty." | Refresh |
| Stale | alert row above rows | "The list for {repo} may be out of date." | Refresh |
| Incomplete | alert row above rows | "A folder in {repo} couldn't be read, so this list may be missing items. Fix its permissions, then refresh." | Refresh |
| Busy, rows kept | alert row above the kept rows | "{repo} was busy, so this list wasn't updated." | Retry |
| Busy, nothing kept | alert row | "{repo} is busy right now." | Retry |
| Failed | alert row with a second, visible line, no rows | "Couldn't read the tickets in {repo}. Nothing was changed." then the failure's message | Retry |
| Index unavailable | alert row, no rows | "The search index for {repo} can't be read. Nothing was changed." | Rebuild the search index |

- The alert row is one line, a second for a failure's message, with an
  icon and one action button: `circle-alert` in amber for never
  refreshed, stale, incomplete and busy; `circle-x` in red for failed and
  unavailable (`06`). Never refreshed, stale and incomplete are checked in
  that order, as `ListState::label` does (fact 18).
- Refresh and Rebuild the search index call `maintain_index` (fact 29);
  Retry reads the list again, as today's Retry does; Clear filters empties
  the text and sets Any readiness; New ticket… and New document… are the
  toolbar's actions.
- After an action whose element goes away, focus moves to the panel's
  list or tree for Refresh, Retry and Rebuild, and to the filter input for
  Clear filters and Show all (R-28).
- Skeleton rows are plain bars in `theme.skeleton`, not the Kit
  `Skeleton`, which pulses (fact 9): `07` lets nothing but the spinner
  animate (R-6).
- "Updating…" replaces the header count while a reload runs over kept
  rows, so a reload is told apart from a settled list (RFC 57-61)
  without moving the rows. On a first load the count is absent, as
  today (C1 R-22).

**Kept rows (R-17).** Each list model keeps the rows of its last ready
read for its repository, in a small `KeptList<T>` beside `ListState` in
`D/model/list_state.rs`. A reload shows them until it lands; a busy reload
keeps them under the busy alert; a failed reload replaces them with the
failure, as D1's ruling R-B5 says. A change of repository drops them.
Documents applies its combined tree state (from its two reads) to its
`KeptList`, not each read.

## Part B: The Tickets Model

**One read (R-1).** The panel reads `list_tickets` once per load, with
`TicketFilter::default()`: closure All, no readiness (fact 16). Lifecycle,
readiness, the text filter and the sort are applied in the desktop by a
pure function. Changing any of them never reads again and never leaves a
stale read to drop. The second read under Blocked goes: the blockers are
named from the same rows, which hold every ticket.

**`TicketRow`** gains `readiness` (Ready, Blocked or Closed), `context`
(its kind, branch and worktree, fact 17) and `reasons` for every blocked
row, named from all the read's rows; the other fields are unchanged.
Unreadable files join as rows too (Part I).

**`TicketQuery`** becomes the view's filter: `closure` (default Open),
`readiness` (default none), `text` (default empty) and `sort` (default
Updated, newest first). It is no longer `Copy` and no longer maps onto a
library filter; `query()` returns a reference.

- Choosing Ready or Blocked while Closed is chosen chooses Open, and
  choosing Closed sets Any readiness, since only open tickets are ready
  or blocked (R-26).

**`derive(rows, query) -> TicketView`** (`D/model/ticket_view.rs`):

1. **Text (R-16).** A conforming row matches when the trimmed text is
   empty, is a case-insensitive substring of its title or short code, or
   equals its ID ignoring case. Nonconforming and unreadable rows always
   match (R-9).
2. **Counts.** `open`, `closed` and `all` count the conforming tickets
   that match the text, by lifecycle, so Open + Closed = All. The readiness
   choice does not change them (decision 6).
3. **Elsewhere.** `elsewhere` counts the conforming tickets that match a
   non-blank text but are hidden by the lifecycle choice (decision 2).
4. **Lifecycle and readiness.** Rows of the chosen lifecycle (All keeps
   both), then, when a readiness is chosen, those whose readiness state is
   that one. Nonconforming and unreadable rows are kept under every
   choice (RFC 67-71, "Filtering does not hide malformed entries").
5. **Sort (R-10).** A stable sort of the conforming rows over the library's
   order (fact 16), by the chosen column:
   - Type, Code, Title, Status: case-insensitive text; a row without a
     code, or with an empty status, has no value.
   - Updated: by age, so ascending is newest first (the prototype's
     `asc: true` with `arrow-up`, `Workspace.dc.html:732`, `:1005-1025`).
   - Descending reverses the order of the rows that have the value; rows
     without it stay last either way.
   - Nonconforming, then unreadable rows follow, in their read order.
6. **`listed`** is every row the panel draws; `shown` the conforming ones.
7. **Footnote** (decision 7), from `shown`, `n`:
   - `{n} open tickets in {repo}`, `{n} closed tickets in {repo}`, or
     `{n} tickets in {repo}` under All; "ticket" when `n` is 1.
   - Under Ready or Blocked the word is "ready" or "blocked":
     `{n} ready tickets in {repo}`.
   - With text: `… in {repo} match "{text}"` ("matches" when `n` is 1),
     the text trimmed.
   - Then ` · 1 needs fixing` or ` · {k} need fixing` for nonconforming
     rows, and ` · 1 can't be read` or ` · {k} can't be read` for
     unreadable ones.

**Hint (decision 2).** When `elsewhere` is above zero, a line above the
rows reads "1 closed ticket also matches." or "{k} open tickets also
match." with a Show all action that chooses All. So every match of a
short code is reachable, whatever the lifecycle chosen (RFC relationships
146-148).

**Age** (`age_label(changed_at, now)`, R-20), as the prototype writes it
(`Workspace.dc.html:715`): whole minutes under an hour (`0m` to `59m`),
then hours rounded (`{h}h`) under a day, then days rounded (`{d}d`). A time
after `now` is `0m`; an unreadable or missing time is blank. `now` is
passed in, so tests set it.

**Markers** (`row_markers(…)` in `D/model/markers.rs`, shared with
Documents; R-8), from in-memory state only:

- **Unsaved** (`pencil`, amber) when `held_by` answers `Yes`: a draft of
  the item or of a comment on it (fact 26). `Unknown` shows none (R-8).
  A private working copy is not a marker: every open ticket in a
  dogfooding repository has one (fact 17); it is context in the tooltip
  (decision 8).
- **Conflict** (`circle-alert`, red) when the repository's pending
  conflicts name the row's ID.
- **Closed** (`circle-check`, `secondary`) when the row is
  lifecycle-closed and the lifecycle choice is All (decision 5).

**Status colour** (decision 20). One mapping for the list and the item
header: the header's `tint` (fact 28), moved to `D/model/status.rs`, with
`03`'s "ready" added as done and "waiting" and "not started" as none. The
pill shows the status as written; an untinted status is neutral
(`secondary`). A nonconforming row shows "needs fixing" in amber, an
unreadable one "can't be read" in red. The pill shows project status,
never lifecycle or readiness (RFC 94-95).

**Tooltip** (`ticket_tooltip(row, repo)`, R-14), lines in this order,
each only when it applies:

- `{code or "No short code"} · {repo}`, then `ID {ULID}` (decision 4;
  PRD MH-CONTENT-006).
- `Type {type}`.
- The copy it lists from: "Main copy", "Private working copy" or "Working
  copy not checked yet" for Primary, Active and Unverified, then
  `Git: branch {branch} · worktree {worktree}` (PRD MH-CONTENT-004;
  `06` keeps Git detail to tooltips; RFC relationships 146-148's
  "context").
- Each blocked reason, one per line, as today's reasons read ("Waits for
  open ticket: vh-aaaaa Dock layout").
- `Needs fixing: {guidance} Open it to fix.` for a nonconforming entry,
  with `ProblemCode::guidance()` of its first problem (decision 9), or
  `Can't be read: {guidance}` for an unreadable one; neither has the
  other lines.
- `Closed` for a closed ticket.

The row's `aria_description` is the same lines joined by "; ". Marker
tooltips are also their accessible names:

- Unsaved: "You have unsaved work on this."
- Conflict: "This has a conflict to resolve. Open it to choose what to
  keep." (fact 27: not every conflict is with the shared copy).
- Closed: "Closed".

## Part C: The Tickets Panel

Top to bottom inside `region-tickets`:

1. **Filter bar** (padding 6 8, bottom border):
   - **Segments.** The existing choices `filter-open`, `filter-closed` and
     `filter-all` (R-4), drawn as one segmented control: `Open 8 | Closed
     4 | All 12`, counts in `muted`, the active segment in `border` with
     `text`. Each stays a tab stop activated by Enter or Space, keeps
     `Role::Tab` and `aria_selected`, and its accessible name gains the
     count ("Open, 8 tickets"; "Open, 1 ticket").
   - **Readiness menu** (R-5; Wave decision 5). A `sliders-horizontal`
     icon button `filter-readiness` with `.dropdown_menu`: "Any
     readiness", "Ready", "Blocked", the current one checked. When Ready or
     Blocked is chosen the button also shows that word. Its accessible
     name is "Readiness: any", "Readiness: ready" or "Readiness: blocked",
     since the Kit does not expose the checked item (fact 8). Tooltip
     "Show ready or blocked tickets". Focus returns to it when the menu
     closes (fact 8).
   - **Filter input** (decision 2; Part D). 24 px high, `list-filter`
     prefix icon (prototype :110), placeholder "Filter by title or code",
     clearable.
2. **Hint line** (Part B), when there is one.
3. **Column header** (24 px, bottom border; R-2): a 14 px type button
   showing only its arrow, then buttons reading `CODE`, `TITLE`, `STATUS`
   and `UPD.` in 11 px `muted`. Ids `sort-type`, `sort-code`,
   `sort-title`, `sort-status`, `sort-updated`. The active one is `text`
   with `arrow-up` or `arrow-down`; the others show a faint
   `arrow-up-down`. Activating a new column sorts it ascending; activating
   the active one reverses it. Each is a tab stop activated by Enter or
   Space, with `aria_selected` on the active one and an accessible name
   that says the order (no `aria_sort`, fact 11): the active column reads
   "Sort by code, A to Z" or "Sort by code, Z to A" (type, title and
   status alike), or "Sort by updated, newest first" or "Sort by updated,
   oldest first"; an inactive one reads "Sort by code". Tooltip "Sort by
   {column}". The header sits outside the scrolling list, so it stays in
   place.
4. **Body**: the state of Part A, or the rows.
5. **Rows**, a Kit `List` of `ListRow`s (R-2), inside a div with the
   `TicketList` key context. Cells, in order:
   - **Type** (14 px): `repeat` for cycle, `bug` for defect, `sparkles`
     for feature, `list-checks` for task, `ticket` for any other type,
     `triangle-alert` in amber for a nonconforming row, `circle-x` in red
     for an unreadable one. Id `ticket-type`, accessible name and tooltip
     "Type {type}", "Needs fixing" or "Can't be read".
   - **Code** (78 px, mono 11 px, `secondary`): the short code as stored,
     lowercase (decision 3), or `—` with accessible name "No short code"
     (decision 4). Id `ticket-slug`.
   - **Title** (flex, at least 96 px, ellipsis; R-25), `secondary` for a
     closed ticket; a nonconforming or unreadable row shows its path. Then
     the markers, each an icon with its own id, tooltip and accessible
     name: `ticket-unsaved`, `ticket-conflict`, `ticket-closed`.
   - **Status** (92 px): the pill, id `ticket-status`, its text the
     status as written.
   - **Updated** (30 px, right-aligned, `muted`): the age, id
     `ticket-changed`, accessible name "Updated {relative_time}" (fact 28,
     made reachable from the list).
   - Code and Status shrink and clip before the title drops below 96 px
     (R-25).
6. **Footnote** (11 px, `muted`, top border, ellipsis, id
   `tickets-footnote`): Part B's text.
7. **Why blocked**, under Blocked only, as today (fact 13), below the
   list and above the footnote (decision 19).

**Rows are one line** (decision 19). Today's reasons line under each row
goes: the 28 px row has no room for it. A blocked ticket's reasons are in
Why blocked for the selected row, in its tooltip, and in its accessible
description.

**Header count** (R-11): the number of rows listed (`listed`, nonconforming
and unreadable included), as Cycle 01 counted rows listed under the
filters; "Updating…" during a reload over kept rows (Part A).

**Selection** (R-7) stays the list's own: set by a click (which also
opens, fact 1) or the arrows, kept by ID across reloads and filter
changes, and shown with the selected treatment. When a filter or reload
hides the selected row, the selection is cleared, so Enter never opens a
different ticket (Review Focus 1). It does not follow the item area's
tab, unlike the prototype (`:1017`).

**Home and End** (R-15) select and scroll to the first and last row; the
Workspace is notified, so Why blocked follows (fact 1).

**Markers and the view** (R-32): `TicketView` is derived when the read or
the query changes and kept until then. Markers are worked out on every
Workspace render for the rows listed, so a new draft or pending conflict
shows at once; the pane is updated only when its rows or markers changed.

## Part D: The Filter Input Replaces Find

Decision 2. The design draws no Find in the Tickets panel, and its filter
"matches title and short code. Duplicate short codes show all matches"
(`03`:50). The input takes over Find's place, key and ULID path:

- It is the Workspace's existing `find_input` entity, kept under its name,
  with the new placeholder. It is drawn whether or not a repository is
  selected, as Find is (fact 13). `secondary-p` still reveals Tickets and
  focuses it (C1 P15).
- **One source** (R-30). `Workspace::set_filter_text(text, window, cx)`
  sets both the input and `TicketQuery.text`. Typing (Change) sets the
  query; the list, counts, hint and footnote follow each keystroke.
- **Enter:**
  - text that parses as a ULID opens that item through Find's existing
    stamped `show_item` read, ticket or document, closed or not, whatever
    the filters (fact 19; RFC 67-71). A status line under the filter bar,
    id `filter-status`, says "Opening…" while it is held, then nothing
    once the tab opens, "Nothing in {repo} has that ID." when it is not
    found, "{repo} is busy right now." with Retry when busy, or the
    failure's message with Retry. A read held across a change of
    repository is dropped, as today (RFC 341-342). Focus stays in the
    input (fact 19);
  - otherwise, when exactly one conforming ticket matches the text in any
    lifecycle, it opens, even if the lifecycle choice hides it;
  - otherwise, when rows are listed, focus moves to the list with its
    first row selected; with none, nothing happens.
- **Escape** clears the text (`clean_on_escape`, fact 7).
- **Repository change.** The text is cleared when the selected repository
  changes, not on a Retry or a reload of the same one. The clear is
  applied at the next render, which has the window `set_value` needs
  (fact 7). Lifecycle, readiness and sort carry over, as the filters do
  today.
- With no repository selected, Enter says "Select an available
  repository to find in" in `filter-status`, as Find does.
- **Goes:** the match cards, their Open buttons and texts, the
  `find_tickets_by_slug` lookup and its results. **Stays:** the ID parse
  (`parse_id`), the stamped ID read and its states.

## Part E: The Documents Model

**Expansion (R-12).** `Documents` keeps, per repository for the session,
the set of folder paths the person has collapsed. A folder not in the set
is expanded, so a new folder starts expanded, as today (decision 13; the
prototype starts with most folders collapsed, `:733`). The tree's
`TreeEvent::Expanded` and `Collapsed`, mapped from the `folder:{path}`
ids, update the set through a subscription in `Workspace::new` (fact 5).
Enter on a folder (R-24) and Collapse all change the set and rebuild the
tree, since the Kit's toggle is private (fact 5). Building the tree sets
each `TreeItem::expanded` from the set, so a reload or a hidden panel
keeps it. It is not written to disk (K-5).

**Selection after a rebuild** (R-27): restored by ID; when the row is now
hidden in a collapsed folder, its nearest visible ancestor is selected.
New folder… expands the new folder's ancestors, so the folder it made is
visible.

**Row facts** (`doc_row(node, expanded) -> DocRow`):

| Node | Label | Expanded | Trailing | Tooltip and description |
| --- | --- | --- | --- | --- |
| Folder with child nodes, expanded | last path segment | yes | — | its path |
| Folder with child nodes, collapsed | last path segment | no | its document count | its path |
| Folder with no child nodes, not empty (fact 24) | last path segment | no | its document count (`0`) | its path |
| Folder, empty (fact 24) | last path segment | no | "empty" (decision 14) | "Empty folder. Shared once it contains a document." |
| Document | its title | — | the unsaved marker | its path |
| Nonconforming, adoptable | file name | — | `triangle-alert` | "Not a managed document yet. Open to adopt it." then its path |
| Nonconforming, any other | file name | — | `triangle-alert` | "Needs fixing: {guidance} Open it to fix." then its path |
| Unreadable (Part I) | file name | — | `circle-x` | "Can't be read: {guidance}" then its path |

- The adoptable test is the item view's own `unmanaged(item)`
  (`D/view/item/repair.rs:102-114`), moved unchanged to `D/model/item.rs`
  with what it needs; its callers in `item/banners.rs` and
  `repair/adopt.rs` follow. The tree and the banner then agree
  (decision 9).
- The count counts documents under the folder, nonconforming included
  (`document_count`, fact 23).
- The icon follows the row: `folder-open` when expanded, else `folder`;
  `file-text`; `file-exclamation-point` with an amber label for
  nonconforming; `circle-x` with a red label for unreadable.
- Order is unchanged: folders first, then documents in path order
  (fact 23).

## Part F: The Documents Panel

- **Tree** (R-3): the base `Tree` with `ListRow` rows and
  `.vertical_scrollbar` (fact 4), in `region-documents`.
- **Row**: indent `8 + 14 × depth` px; a 12 px chevron (`chevron-right`
  or `chevron-down`) for a folder with child nodes, else a 12 px spacer;
  the icon at 14 px; the label; the trailing cell. Ids: `doc-node` (the
  row, `aria_label` the label, `aria_description` the tooltip text,
  `aria_expanded` from the tree), `doc-count`, `doc-empty`,
  `doc-nonconforming`, `doc-unsaved`.
- **Mouse**: a click on a folder toggles it (the tree's own mouse-down,
  fact 5); a click on a document opens it (`ListRow::on_click`).
- **Keys**: up and down (the tree's); Left collapses and Right expands
  (the tree's); Enter opens a document and toggles a folder (R-24); Home
  and End select the first and last row (R-15).
- **States**: Part A, with the Documents texts. The header count stays
  documents listed (C1 R-22), "Updating…" during a reload.

## Part G: Panel Toolbars

`02` puts each panel's toolbar at the right of its header, and Cycle 01
left the header without one (fact 20; C1 Exclusions, "Panel toolbars go
into this Cycle's panel header").

- The header draws the toolbar of the panel it shows: for a single panel
  at the right of its title row; for a tabbed group, the active tab's at
  the right of the tab strip. `single` and `tabs` receive the Workspace
  handle `header()` already holds, and ask it for the panel's toolbar
  (R-13).
- **Focus counts as the panel's region** (decision 18). The Workspace owns
  each button's focus handle. `focused_region` counts a focused toolbar
  button as its panel's region, so the region's ring shows, F6 and
  Shift F6 move on from that region, and the keyboard walk's checks hold
  (fact 21). The toolbar carries the `Navigation` key context, so
  `secondary-n` works there and makes the panel's own kind.
- Buttons: 26 × 24 px, radius 4, 14 px icon in `secondary`; hover `hover`
  with `text`. Each is a tab stop activated by Enter or Space, with an
  accessible name and a tooltip:

| Panel | Id | Icon | Name and tooltip | Does | Disabled when |
| --- | --- | --- | --- | --- | --- |
| Tickets | `new-ticket-button` | `plus` | "New ticket…", with its key (`tooltip_with_action`) | New ticket…, as today's body button | no repository resolved |
| Documents | `new-document-button` | `file-plus` | "New document…", with its key | New document…, as today | no repository resolved |
| Documents | `new-folder-button` | `folder-plus` | "New folder…" | New folder…, as today | no repository resolved |
| Documents | `collapse-all-button` | `chevrons-down-up` | "Collapse all folders" | Part E's Collapse all | no folder is expanded |

- The ids are today's body buttons' ids, so tests that click them keep
  working; the body buttons and their wrapper divs go.
- No More button in this Cycle: nothing is designed to go in it, and the
  Wave leaves out what would be a stub (decision 12).
- Each handler is guarded by `unless_dialog`, like the footer's (C1 R-29),
  so nothing acts behind a dialog or sheet (C1 contract 4).

## Part H: Keyboard

- **Unchanged:** F6 and Shift F6, `secondary-1` to `secondary-4`,
  `secondary-n` (New ticket… or New document…, the focused panel's),
  `secondary-r` and `secondary-shift-r`, up and down in both lists, Left
  and Right in the tree, Enter opening a ticket or document.
- **`secondary-p`** focuses the filter input (Part D).
- **Home and End** (R-15): new actions `SelectFirst` and `SelectLast`,
  bound in the new `TicketList` context and the existing `Documents`
  context, each guarded by `unless_dialog`. In the filter input Home and
  End stay the input's, whose context is deeper.
- **Enter on a folder** toggles it (R-24).
- **Tab order** in Tickets: the toolbar, the segments, the readiness
  menu, the filter input, Show all when shown, the five column headers,
  Why blocked when shown. In Documents: the toolbar. The list and the tree
  are not tab stops (facts 1, 4), as today: they are reached by F6,
  `secondary-2`, a click, or Enter in the filter input (Part D; K-11).
- Every new control shows focus: Kit buttons and the input draw the Kit
  ring; segments and column headers draw the app's ring as today's
  choices do; the list's selected row draws Part A's inset ring.

## Part I: Unreadable Files

Decision 10. The Wave's exit evidence asks for inaccessible items to stay
visible with their reason, and the read that reports them exists
(fact 30).

- Each load reads `index_status` once beside the lists, through the
  desktop's read service, under the lists' stamp.
- `unreadable_rows(status) -> (Vec<UnreadableFile>, Vec<UnreadableFile>)`
  (`D/model/unreadable.rs`) picks the `SourceUnreadable` problems whose
  path names a `.md` file under `.manyhands/tickets/` (Tickets) or `docs/`
  (Documents), with the worktree each was found in. Other problems are
  not this Cycle's (Activity's, Cycle 03).
- Tickets lists them after the nonconforming rows; Documents places each
  in its folder, after the documents, as it places a nonconforming one.
- A row shows its path (Tickets) or file name (Documents), "can't be
  read" (Tickets' pill), the `circle-x` icon, and the tooltip
  `Can't be read: {guidance}` with the path. Enter or a click opens it by
  path, as a nonconforming entry opens (an inspection tab, which shows
  why it cannot be read).
- A failed or busy status read leaves the lists as they are, with no
  unreadable rows, and does not fail them: the lists still answer.
- The footnote counts them (Part B). The panel header count includes
  them (R-11).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | A repository's tickets load | One `list_tickets` read with the default filter and one `index_status` read; no second read under Blocked |
| P2 | Open, Closed or All chosen; Ready or Blocked chosen; text typed; a column sorted | The list, counts, hint and footnote follow at once; no read is sent |
| P3 | The default | Open, Any readiness, no text, Updated newest first: the library's order |
| P4 | A ticket whose status text is `closed` but lifecycle open | Listed under Open, its pill reads `closed` |
| P5 | Under All | A lifecycle-closed row has a `secondary` title and the Closed marker |
| P6 | Nonconforming and unreadable rows | Listed under every lifecycle, readiness and text, last in read order under every sort, with their reason |
| P7 | Two tickets share a short code; the code is typed | Both listed when both are in the lifecycle chosen; otherwise the hint names the others and Show all lists them; each opens its own ticket by ID |
| P8 | A ULID typed, then Enter | That ticket or document opens, closed or not, whatever the filters; "Opening…" while held; "Nothing in {repo} has that ID." when not found; busy and failure with Retry; dropped when the repository changes first; focus stays in the input |
| P9 | Enter with exactly one match in any lifecycle; with several | It opens; focus moves to the list with the first row selected |
| P10 | Segment counts | Conforming tickets matching the text, by lifecycle; Open + Closed = All; readiness does not change them |
| P11 | Ready or Blocked chosen through the menu, by mouse and by keyboard | The rows of that readiness; the button names it; Why blocked under Blocked; focus back on the button; Closed becomes Open |
| P12 | A column header activated, by click and by Enter | Sorted ascending; again, reversed; the name says the order |
| P13 | A ticket with no short code | `—`; the tooltip and description carry its ID |
| P14 | A ticket with unsaved work (item or comment draft); with a pending conflict | The unsaved marker; the conflict marker; they appear without a reload |
| P15 | Each list state of Part A, in both panels | Its own text and action; Retry reads, Refresh and Rebuild submit their jobs, Clear filters clears and focuses the input |
| P16 | A reload of a listed repository | The kept rows stay until it lands, the header reads "Updating…"; a busy reload keeps them under the busy alert; a failed one shows the failure with its message visible |
| P17 | A change of repository | The text is cleared; lifecycle, readiness and sort kept; kept rows dropped. A Retry of the same repository keeps the text |
| P18 | Home and End in the Tickets list and in the tree | First and last row selected and scrolled to |
| P19 | A folder collapsed, then the list reloaded, then the panel hidden and shown | Still collapsed |
| P20 | Collapse all | Every folder collapsed; the hidden selected row's top-level folder selected |
| P21 | Enter, a click, Left and Right on a folder | Toggle, toggle, collapse, expand |
| P22 | A collapsed folder; a leaf folder with no documents; an empty folder | Its count; `0`; "empty" with its explanation |
| P23 | A nonconforming document | Its file name in amber, the marker, and the adopt or fix text by its problem |
| P24 | A toolbar button by Tab and Enter, in a single panel and in a tabbed group | The same action as its click; its panel's region holds focus, F6 moves on, `secondary-n` makes its kind; disabled with no repository; refused behind a dialog |
| P25 | Any filter, sort or expansion change, or Collapse all | No tab opens, closes or reloads; nothing is saved, synchronized or sent (C1 contract 7) |
| P26 | Hiding and showing either panel | Filters, text, sort, selection and expansion kept (C1 P2) |
| P27 | A list holding only nonconforming or unreadable rows; a tree of folders only | Rows, never the empty state |
| P28 | An unreadable ticket or document file | A row with its reason in its list; it opens by path |
| P29 | The header count | Rows listed under the filters; "Updating…" during a reload |
| P30 | A filter hides the selected row, then Enter in the list | Nothing opens |

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
- Tooltips and descriptions are tested through their pure functions:
  the test snapshot cannot read a description (fact 11).
- No real waits; `now` is passed in.

| Test | What |
| --- | --- |
| `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts` | One table: open, closed, ready, blocked, no-code, duplicate-code, status `closed` while open, empty status, nonconforming and unreadable rows under every lifecycle, readiness, text and sort it needs; rows in order, counts, `elsewhere`, `listed`, `shown` and footnote (P2–P7, P10, P13, P27) |
| `desktop::model::ticket_view::tests::ages_read_as_the_prototype_writes_them` | One table of `age_label` (R-20) |
| `desktop::model::ticket_view::tests::tooltips_carry_the_id_context_and_reasons` | One table of `ticket_tooltip`: coded, no code, each context kind, blocked with an open dependency and on a cycle, nonconforming, unreadable, closed (P13) |
| `desktop::model::markers::tests::markers_follow_drafts_conflicts_and_closure` | One table of `row_markers`: `Held` Yes, No and Unknown, a conflict, closed under each lifecycle (P5, P14) |
| `desktop::model::list_view::tests::each_state_has_its_own_body` | One table: every row of Part A's state table for each panel, `listed` 0 and above, with its text and action (P15, P27) |
| `desktop::model::list_state::tests::kept_rows_follow_reloads` | One table of `KeptList`: reload, then ok, busy or failed; a combined state; a change of repository (P16, P17) |
| `desktop::model::tickets::tests::a_read_lands_only_for_its_repository_and_load` | One table (E-1): a read for the load in flight lands; one for an earlier load, another repository or after a clear is dropped; a query change reads nothing (P2, P17) |
| `desktop::model::unreadable::tests::unreadable_files_go_to_their_list` | One table of `unreadable_rows`: ticket file, document file, a directory, a file elsewhere, another problem code (P28) |
| `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all` | Toggle, rebuild with a new folder and with a folder gone and back, Collapse all, another repository; the set and the built `expanded` flags; the selection rule (P19, P20) |
| `desktop::model::documents::tests::rows_read_as_the_design_writes_them` | One table of `doc_row` over Part E's eight kinds, label, expanded, trailing and tooltip (P22, P23) |
| `desktop::view::tickets::tests::the_filter_bar_works_without_a_read` | A journey: one read on load; segments by click and key; the readiness menu by keyboard; text with the hint and Show all; sorts by click and by Enter; Home and End; a filter hiding the selected row; header count, footnote and segment names at each step; no read or request sent after the load (P1, P2, P7, P10–P12, P18, P25, P29, P30) |
| `desktop::view::tickets::tests::list_states_act_and_a_reload_keeps_the_rows` | Retry reads; Refresh submits `IndexRefresh`; Clear filters clears and focuses the input; a job-driven reload keeps the rows with "Updating…"; a held busy reload keeps them under the busy alert (P15, P16) |
| `desktop::view::documents::tests::folders_keep_their_state_and_toggle_by_key` | Left, Right, Enter, a click; a reload; hide and show; Collapse all; Home and End; Enter on a document opens it (P18–P21, P25, P26) |
| `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter` | Each toolbar button reached by Tab and run by Enter, single and tabbed; region, F6 and `secondary-n` from a button; disabled with no repository; refused behind a dialog (P24) |
| `desktop::view::find::tests::*` (rewritten, E-4) | ULID of a closed ticket under Open, of a document, unknown, held across a repository change; one match in another lifecycle; several matches; focus (P8, P9, P17) |
| `desktop::view::tickets::tests::an_unreadable_file_is_listed_with_its_reason` (`#[cfg(unix)]`) | A ticket file made unreadable, then a refresh: its row with "can't be read"; Enter opens it by path (P28) |

Pre-existing tests change as the named edits say.

## Named Edits To Pre-Existing Tests

At `a0a734a4`. Task 0 re-counts with `git grep`, over tests and
production code, and records any other site as a further edit
(decision 16).

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `D/model/tickets_tests.rs`: `the_query_maps_closure_and_readiness_and_nothing_else`, `a_filter_change_drops_the_last_filters_read`, `set_query_keeps_the_query_and_drops_the_last_read`, `a_readiness_change_drops_the_last_readinesss_read`, `rows_read_under_blocked_carry_their_reasons`; the two `blocked_reasons` tests' query set-up (`:259-266`, `:305-312`) | The five become one table test, `a_read_lands_only_for_its_repository_and_load`, keeping the repository-change, clear and generation guards (`:157-169`) and the query kept before a load; the mapping to a library filter and "a filter change drops the read" are **removed** (R-1). Blocked reasons move to `derive_filters_counts_and_sorts`. `the_default_query_lists_open_tickets` asserts the default query and `read_filter() == TicketFilter::default()`. The `blocked_reasons` tests build `TicketFilter` literals | B |
| E-2 | `D/view/tickets_tests.rs`: `a_filter_change_drops_the_held_read_of_the_last_filter`, `a_readiness_change_drops_the_held_read_of_the_last_one`; helpers `library()` (`:49-57`) and `blocked_list` (`:449-457`); `arrows_move_in_the_list_and_enter_opens` | The two tests are **deleted**: a filter change no longer reads; the repository-change guard is E-1's table and P17's view step. The helpers take `TicketFilter` literals | B |
| E-3 | `D/view/tickets_tests.rs`: `each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`, `ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry`, `the_why_blocked_panel_gives_the_selected_rows_reasons_in_full`, `the_panel_says_incomplete_where_the_library_does`, `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text`, `a_long_why_blocked_panel_scrolls_by_keyboard` | Each still compares the rows shown under each choice with `list_tickets(TicketFilter { closure, readiness })` (contract 1). `ticket-lifecycle` becomes the `ticket-closed` marker under All; readiness is chosen through `filter-readiness`; `ticket-reason-{i}-{n}` becomes Why blocked's lines; the `row_description` check becomes `ticket_tooltip`'s; the "Showing …" texts go, so the label test checks the segments' names with counts and the menu button's name. **The "Showing …" and inline-reason assertions are removed** | C |
| E-4 | `D/view/find_tests.rs` (8) | Rewritten for the filter input. `secondary_p_focuses_find`, `a_ulid_opens_its_ticket_closed_or_not`, `find_does_nothing_while_a_dialog_is_open`, `a_held_search_never_lands_after_the_selection_changed` (the ULID read) and `find_needs_a_selected_repository_and_a_new_selection_clears_it` (the text is cleared) keep their intent; `no_match_says_so_and_opens_nothing` checks "Nothing in {repo} has that ID." and, for text, "No tickets match these filters."; `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` lists both as rows, one closed through the hint. Steps for a document's ULID and one match are added. `a_matchs_open_is_reached_by_tab_and_enter_opens_it` is **deleted** (no match cards), and the cards' context labels are **no longer asserted** (the context is in the tooltip, tested by its function) | D |
| E-5 | `D/model/find_tests.rs` | The short-code tests go; `an_id_found_opens_by_its_id_and_one_not_found_is_no_match` and `an_earlier_search_is_dropped` stay, rewritten to `parse_id` and the ID read's states. **This removes the short-code assertions** | D |
| E-6 | `D/view/keyboard_tests.rs` `change_the_filters` (`:345`), `find` and `find_by_short_code_and_ulid` (`:430`, `:450`) | New Tab order (Part H); readiness through the menu; Find steps use the filter input. **The "Showing …" checks and the match-card Tab steps are removed** | C, D |
| E-7 | `D/view/keyboard_tests/editing.rs` `create_by_keyboard_alone` (`:709-735`), `move_and_new_folder_by_keyboard_alone` (`:850-886`); `D/view/documents_tests.rs` `new_folder_opens_its_dialog` | Shift-Tab from the list or tree no longer reaches the body buttons; the steps reach `new-ticket-button`, `new-document-button` and `new-folder-button` in the header. The region and ring checks are unchanged (decision 18) | G |
| E-8 | `D/view/documents_tests.rs` `the_tree_shows_what_the_library_reads_give`, `arrows_move_in_the_tree_and_enter_opens` (Enter on a folder now toggles it), `a_held_tree_of_a_never_replaces_bs` (`:239`, `:245`); `D/view/new_folder_tests.rs` (`doc-empty` text); `D/view/move_document_tests.rs` helper `tree_paths` (`:274-279`) | The nonconforming label is the file name; "Empty folder" becomes "empty"; `doc-path`, `doc-nonconforming` and `doc-problem` lines go, and `tree_paths` reads document paths through a `#[cfg(test)]` accessor on `DocumentsPane`. **The second-line assertions ("Empty folder", "Nonconforming", the problem code, the path) are removed**; `doc_row`'s table carries their content | E, F |
| E-9 | Keyboard helpers that open a nonconforming file by its path label: `keyboard_tests/actions.rs` `a_nonconforming_entry_opens_by_keyboard`, `keyboard_tests/editing.rs` `repair_and_adopt_by_keyboard_alone` | They look for the file name | E |
| E-10 | `D/view/dock_tests/state.rs` `a_hidden_panel_keeps_its_state`; `D/view/lifecycle/made_ready_tests.rs` (`filter-ready`), `made_ready_review_tests.rs` (`filter-ready`, `:70` `filter-blocked`) | Find text is the filter text; readiness is read from `filter-readiness`'s name. `open_ready_tickets` is also checked to clear typed text (R-29) | C, D |
| E-11 | `D/assets_tests.rs` `every_item_view_icon_loads` | The table and its array length gain the new icons of fact 12 (`circle-check` and `sliders-horizontal` are already there) | A |
| E-12 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | The table gains `home` → `SelectFirst` and `end` → `SelectLast` | H |
| E-13 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour and stops for the owner | — |

## Document Amendments

To apply in the approval commit:

1. **Wave Cycle 02.**
   - In scope, Tickets: the footnote reads as decision 7; "Ready and
     Blocked filters keep working, in a filter menu" adds "Why blocked
     stays under Blocked; a row's reasons move to Why blocked and its
     tooltip (decision 19)"; add "Find becomes the filter input (decision
     2)"; the toolbar reads "New ticket (More waits for an action,
     decision 12)".
   - In scope, list states: "failed or stale scan with Retry" reads
     "failed with Retry, stale and never refreshed with Refresh"
     (decision 11).
   - Exit evidence: "nonconforming and inaccessible items stay visible
     with their reason" adds "unreadable files from the index status read
     (decision 10)".
2. **Wave Deferred Work.** Add: saving the ticket sort, filters and folder
   expansion with the layout (K-5).
3. **Cycle 01 document, decision 4.** Add: "Superseded in part by Wave 04
   Cycle 02 decision 2: Find is now the Tickets filter input; `secondary-p`
   and the open-by-ID path are kept."
4. **Handoff `08`.** The Tickets list is a Kit `List` with an app row and
   an app column header, not `DataTable` (fact 3); the Documents tree is
   the base `Tree` with app rows (fact 4); the segmented control is the
   app's own (fact 6); `file-warning` is `file-exclamation-point`
   (fact 12).
5. **Handoff `03`, Tickets.** Short codes appear as stored, in lowercase
   (decision 3); the status pill uses the item header's mapping
   (decision 20); "Ctrl N" is the platform's New key.
6. **Ticket relationships RFC, Desktop (318-331).** "A find-by-slug entry
   lists every match with its context" reads: "The Tickets filter lists
   every ticket with that short code in the lifecycle chosen and names
   the matches in the other, each with its context in its tooltip." "Is
   shown by its ULID" (132-134) adds: "in the desktop list, `—` with the
   ULID in the row's tooltip" (decisions 2, 4).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | One `list_tickets` read with the default filter per load; lifecycle, readiness, text and sort are applied in the desktop; the names read goes (fact 16) | A large repository reads its closed tickets on every reload (K-10) |
| R-2 | Tickets: a Kit `List` with `ListRow` items and the app's column header outside it; not `DataTable` (fact 3) | — |
| R-3 | Documents: the base `Tree` with `ListRow` rows and the wrapper's scrollbar; not `component::tree`, whose `ListItem` overrides the selected background (facts 2, 4) | The Kit's context-menu hook on tree rows is not used (none is configured today) |
| R-4 | The segments are the app's existing choice controls, restyled; the Kit's segmented controls have no keyboard path (fact 6) | — |
| R-5 | The readiness menu is a `Button::dropdown_menu` with checked items; the button names the choice (fact 8) | A screen reader hears the choice on the button, not on the item (K-6) |
| R-6 | Skeleton rows are static bars in `theme.skeleton`, not the pulsing Kit `Skeleton` (fact 9; `07`:68) | — |
| R-7 | `ListRow` draws hover, the selected treatment and, while its list has focus, an inset ring on the selected row. Selection is the list's own, not the open tab (unlike the prototype, `:1017`); a hidden selected row is cleared | The open item is not highlighted in the list unless selected there |
| R-8 | Markers come from in-memory state only: `held_by`, the repository's pending conflicts, the row's closure. `Held::Unknown` shows no marker, so a store that cannot be read does not mark every row (unlike R-36's retirement, which must assume the worst) | A draft in an unreadable store shows no marker |
| R-9 | Nonconforming and unreadable rows pass every filter, the text included, and sort last in read order | — |
| R-10 | The sort is stable over the library's order; rows without the value sort last in either direction | — |
| R-11 | The header count is the rows listed, nonconforming and unreadable included, as C1 R-22 counted rows listed under the filters | — |
| R-12 | Expansion per repository, session only, as a set of collapsed folder paths; absent means expanded; `TreeEvent` updates it; `set_items` builds each item's `expanded` from it | Expansion is lost at restart (K-5) |
| R-13 | `single` and `tabs` receive the Workspace handle and draw `Workspace::panel_toolbar(kind)` for the panel shown | — |
| R-14 | Every tooltip's text is also the element's accessible name or description; tooltips are hover-only (fact 10) | A sighted keyboard user does not see a tooltip (K-1) |
| R-15 | `SelectFirst` and `SelectLast` on `home` and `end` in the `TicketList` and `Documents` contexts, guarded by `unless_dialog`, notifying the Workspace | — |
| R-16 | Text matching: trimmed, case-insensitive substring of title or short code, or the whole ID | — |
| R-17 | Each list keeps its last ready rows for its repository: shown during a reload and over a busy read; replaced by a failure (R-B5); dropped on a change of repository | — |
| R-18 | Why blocked stays as today under Blocked; reasons come with every blocked row | — |
| R-19 | Short codes as stored, lowercase (RFC relationships :103-107) | Unlike the handoff's frames |
| R-20 | Ages as the prototype writes them: `{m}m`, rounded `{h}h`, rounded `{d}d`; blank when unknown | — |
| R-21 | New icons join `ItemIcons`; `file-exclamation-point` stands for `file-warning` | — |
| R-22 | The footnote counts conforming rows shown and names needs-fixing and can't-be-read separately (decision 7) | — |
| R-23 | An empty folder shows "empty" where a count would be (decision 14) | — |
| R-24 | Enter on a folder toggles it, as a click does, through the model and a rebuild | — |
| R-25 | The title keeps at least 96 px; Code and Status shrink and clip first | In a narrow dock codes and pills are cut (K-2) |
| R-26 | Ready or Blocked with Closed chooses Open; Closed sets Any readiness | — |
| R-27 | After a rebuild a hidden selected row gives the selection to its nearest visible ancestor; New folder… expands the new folder's ancestors | — |
| R-28 | Focus after a transient action: the list or tree for Refresh, Retry and Rebuild; the filter input for Clear filters and Show all | — |
| R-29 | `open_ready_tickets` chooses Open and Ready, clears the text, keeps the sort and reads once | — |
| R-30 | The filter text has one source: `set_filter_text` sets the input and the query; a repository change clears it at the next render | — |
| R-31 | Find's stamped ID read and its states stay; only the short-code lookup goes | — |
| R-32 | `TicketView` is derived when the read or the query changes; markers on every Workspace render; the pane is updated only when its rows or markers changed | Markers cost a little per render |

## Known Limits

- **K-1.** Tooltips show on hover only (fact 10). Their text is also each
  element's accessible name or description, and the opened item shows the
  ID, path and context.
- **K-2.** In a narrow dock, such as the left one at 290 px, short codes
  and status pills are clipped so the title keeps 96 px (R-25).
- **K-3.** Ages are worked out when a row is drawn, so an idle list's ages
  do not tick on their own.
- **K-4.** Unreadable files are read from the last refresh's index
  status; one made unreadable since shows after the next refresh.
- **K-5.** Folder expansion, the filters and the sort last for the session
  only; saving them with the layout is Deferred Work (amendment 2).
- **K-6.** The readiness menu's checked item is not exposed to
  accessibility; the button's name says the choice (fact 8).
- **K-7.** The sort order is in the header button's name; there is no
  `aria_sort` (fact 11).
- **K-8.** Up and Down wrap at both ends, as the Kit does (facts 1, 5).
- **K-9.** New document… still starts in `docs`, not the selected folder,
  as today.
- **K-10.** Every reload reads every ticket of the repository, closed ones
  included (R-1).
- **K-11.** The list and the tree are not Tab stops, as today (facts 1,
  4): they are reached by F6, `secondary-2`, a click, or Enter in the
  filter.
- **K-12.** Short codes longer than about 11 characters are clipped in
  the Code column; the tooltip shows them whole.

## Unverified

- **U-3.** That `single` and `tabs` can draw Workspace-built buttons whose
  focus `focused_region` counts as the panel's region (decision 18,
  R-13). Task 8's test proves it.
- **U-5.** That the Workspace renders when the draft registry or the
  pending conflicts change (fact 26, R-32), as the tab strip's unsaved dot
  needs. Task 3 checks it in the source before relying on it.

U-1, U-2 and U-4 were settled in review (facts 1, 4, 8).

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
- **A pencil for every private working copy:** every open ticket in a
  dogfooding repository has one, so it would mark nearly every row
  (decision 8).
- **The toolbar as a row inside the panel body:** simpler for focus, but
  the design puts it in the header and the header has the room
  (decision 18).
- **Accepting the unreadable-file gap:** the read that reports them
  exists, and the Wave's exit evidence asks for them (decision 10).

## Risks

| Risk | Control |
| --- | --- |
| The redesign reaches many existing tests (labels, ids, Tab order) and the `TicketQuery` change reaches production code | E-1 to E-13 with their sites; ids kept where the element survives; Task 0's sweep over tests and production; each task runs the tests it touches by exact name |
| One unfiltered read is slower on a large repository | The library already loads every stored item for any filter (`items.rs:1712-1724`); K-10; the performance work the owner put next can add a count read |
| Custom rows lose the Kit's list accessibility | The Kit `List` and base `Tree` keep their roles, positions and selected state on the row wrapper (facts 1, 4); rows keep their ids and names |
| Find's removal loses a path to closed tickets and documents by ID | Part D keeps the stamped ULID read, its feedback and its test (P8) |
| Header focus counted as a region breaks C1's focus rules | Decision 18 keeps one region per focus; the toolbar test runs F6 and the keyboard walk's ring check from a button |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | The design's segment counts need every lifecycle; today each filter is its own read | Ruling | R-1 | Fact 16 |
| Planning research | Find and the design's filter input both match short codes | Decision (proposed) | Decision 2, Part D | Fact 19; `03`:50 |
| Planning research | Upper- vs lowercase codes; `—` vs ULID; closed by colour alone; counts and footnote; nonconforming wording; list states; expansion | Decisions (proposed) | Decisions 3–9, 11, 13, 14 | Revision 1 Review Record |
| Planning research | Kit list, tree, table, segmented, menu and skeleton facts | Rulings | R-2 to R-6 | Facts 1–9 |
| Review round 1, source and feasibility (`563763e2`) | 4 Blocking: header buttons outside every region's focus; `TicketQuery` change breaks unnamed sites; removing `LIFECYCLE_*` breaks the item view; the snapshot cannot read a description | Decision (proposed), named edits, rulings | Decision 18 and Part G; E-1, E-2 sites and the plan's Task 2; `LIFECYCLE_*` kept; descriptions tested through pure functions and a `#[cfg(test)]` accessor (E-8) | Facts 11, 14, 20, 21 |
| Review round 1, source and feasibility | 15 Important: header count, interim reasons, stale markers, unreachable `waiting_on` and `relative_time`, description contents, the window-less clear, the ULID read, E-1's lost guard, uncovered test sites, `ListRow` without `aria_selected` and `on_click`, `listed` undefined, the working-copy pencil, Tab stops, the RFC duplicate rule, `KeptList` for the tree | Rulings, settled | R-11, R-32, Part B tooltip from reasons, R-30, R-31, E-1, E-2, E-8, E-10, Part A, decision 8, K-11, decision 2's hint, `KeptList` combined state | Reviewer report, 2026-11-11 |
| Review round 1, source and feasibility | 14 Minor, including closing U-1, U-2 and U-4 from the source | Settled | Facts 1, 3, 4, 8, 12, 18, 20; plan file lists | Reviewer report |
| Review round 1, contract and consistency (`563763e2`) | 4 Blocking: duplicate codes across lifecycles and their context; unreadable files rest on a false fact; a list of only nonconforming rows replaced by the empty state; the ULID path's feedback and guard | Decisions (proposed), rulings | Decision 2's hint and the tooltip's context; decision 10 and Part I; `listed` (Part A, P27); R-31 | Fact 30; RFC relationships 146-148 |
| Review round 1, contract and consistency | 22 Important, including the header count, Tab reach, `open_ready_tickets`, conflict and draft wording, failure detail made hover-only, inline reasons, type visibility, one status mapping, focus after actions, toolbar enablement, folders without documents, the reload state, narrow docks | Decisions (proposed), rulings | Decisions 8, 19, 20; R-11, R-25, R-28, R-29; Part A, C, E, G | Reviewer report, 2026-11-11 |
| Review round 1, contract and consistency | Minor: grammar, copy per `06`, focus line width, prototype values, the type column's sort, amendments | Settled | Parts A–D; amendments 1, 2, 5, 6 | Reviewer report |
