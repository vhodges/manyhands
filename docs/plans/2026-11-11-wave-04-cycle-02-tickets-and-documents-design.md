---
title: "Wave 04 Cycle 02 Tickets And Documents Panels Design"
date: 2026-11-11
status: "approved"
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
toolbar and tree, and unreadable files shown in their lists. Approval
covers the mechanisms, rulings, named test edits and task order. It does
not authorize implementation, pushes beyond the ticket branch, merging or
closing.

**Approved 2026-11-11** by the owner (revision 3 with round 3's fixes),
with decision 10's narrowing (R-38) accepted; Document Amendments 1–6 were
applied in the approval commit. Grounded at the ticket branch `a0a734a4`, which
holds origin/main `48653d01` (Cycle 01 and Wave 03 D6 both merged); `src/`
is unchanged since. Revision 1 (`563763e2`) had two reviews in round 1;
revision 2 (`4c32325f`, decisions confirmed at `787c5345`) had a scoped
re-review and an adversarial review in round 2. Revision 3 applies round
2's findings, and a scoped re-review of revision 3 (round 3, `ba5da196`)
found them resolved; its fixes are applied here (Review Record).

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
   only the index as its id, with `Role::TreeItem`, `aria_label`,
   `aria_selected` and, for folders, `aria_expanded` (`:432-441`). Its
   focus handle is not a tab stop, and has no getter (`:199`).
5. **Tree expansion and selection.** Expansion lives in each `TreeItem`
   (`B/tree.rs:34-46`, `:121-124`); `TreeItem::expanded(bool)` sets it.
   `is_folder()` is `!children.is_empty()`, so a folder with no child
   nodes is a leaf that cannot toggle (`:131-134`). `set_items` rebuilds
   rows, clears the selection and emits nothing (`:214-219`).
   `TreeEvent::{Expanded(id), Collapsed(id)}` are emitted on toggles by
   key or mouse (`:93-96`, `:331-335`), and also by `expand_ancestors`
   (`:305`), which `set_selected_item` and `reveal_item` call
   (`:229-240`, `:268-277`); `set_selected_index` expands nothing
   (`:221-228`). Keys in the "Tree" context are up, down, left and right
   only; up and down wrap; left collapses and right expands without
   moving (`:20-27`, `:365-411`). `toggle_expand` and the `Confirm`
   handler are private, and enter is not bound (`:320`, `:350-363`).
   Mouse-down selects and toggles a folder, and does nothing else
   (`:413-417`, `:443-447`). `index_of(id)` and `scroll_to_item(ix,
   strategy)` exist (`:243-266`).
6. **Segmented controls.** `ButtonGroup`, `ToggleGroup::segmented` and
   `TabBar::segmented` exist, but none has arrow keys; `TabBar` tabs are
   not focusable at all (`B/tabs.rs:17-21`). Any focused element with
   `on_click` is activated by Enter and Space (`G/elements/div.rs:3010-3066`).
7. **Input.** `InputState::new(window, cx).placeholder(…)`;
   `InputEvent::{Change, PressEnter, Focus, Blur}`; `set_value` needs a
   `Window` and emits no Change; `set_clean_on_escape(true)` clears on
   Escape, else Escape propagates (`B/input/base/state.rs:766`,
   `:897-915`, `:1086-1091`, `:2010-2044`). `Input::new(&state).prefix(…).cleanable(true)`
   (`C/input/input.rs:240`, `:281`). Today's `find_input` does not clear
   on Escape (`D/view/workspace.rs:226-227`).
8. **Menus.** `Button::dropdown_menu(f)` opens a `PopupMenu` on left
   mouse-down or on Enter and Space, and the menu takes focus with no item
   highlighted (`C/menu/dropdown_menu.rs:12-34`, `:150`;
   `C/menu/popup_menu.rs:934-936`; `B/popover.rs:17-22`, `:288-309`). The
   popover saves the focused handle when it opens and restores it when it
   closes, by Escape or by an item (`B/popover.rs:98-124`).
   `PopupMenuItem::new(label).checked(bool).on_click(…)`
   (`C/menu/popup_menu.rs:180-210`). A checked item is not exposed to
   accessibility (`C/menu/menu_item.rs:99-101`).
9. **Skeleton.** `component::skeleton::Skeleton` pulses with a repeating
   two-second animation (`C/skeleton.rs:39-57`).
10. **Tooltips and buttons.** Tooltips appear on hover only, never on
    keyboard focus (`G/elements/div.rs:1674-1702`; `C/tooltip.rs:247-292`).
    The component `Button` mints a focus handle keyed by its element id
    path (`C/button/button.rs:586-588`); the base
    `gpui_kit::base::Button::track_focus(&handle)` uses a caller-owned
    handle instead (`B/button.rs:93-96`, `:235-241`), and takes a
    `.tooltip(…)` built from `Tooltip::new(…).action(…)`, which shows the
    platform's key (`C/tooltip.rs:69-72`). Neither button stops mouse-down
    from reaching its parent (`C/button/button.rs:783-793`).
11. **Accessibility.** `aria_label`, `aria_description`, `aria_selected`,
    `aria_expanded`, `aria_toggled` exist on stateful elements
    (`G/elements/div.rs:1304-1497`). `aria_sort` does not exist. The test
    snapshot exposes role, path, label, checked, indeterminate, selected,
    expanded, value, bounds, visible, focused (whether the element's
    tracked handle contains focus), focus action and disabled, but **no
    description** (`B/test_support.rs:17-31`, `:302-318`, `:368-371`).
12. **Icons.** Of the design's icons, `repeat`, `bug`, `sparkles`,
    `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`,
    `arrow-up-down` and `list-filter` are in the Kit catalog but not loaded
    by the app; `file-warning` does not exist, and
    `file-exclamation-point` does (`A/build.rs:19-67`). `circle-x` is a
    Kit default; `circle-check` and `sliders-horizontal` are already loaded
    by the app. The app serves extra icons through
    `icon_assets!(ItemIcons, […])` (`D/assets.rs:17-58`); an icon not
    listed there or in the Kit defaults fails to load
    (`A/src/native_assets.rs:29`).

**Tickets today.**

13. **Body.** Find, then the closure choices `filter-open|closed|all` with
    a "Showing …" line and the readiness choices
    `filter-any-readiness|ready|blocked` with their line, all drawn
    whether or not a repository is selected; with one, also New ticket…,
    the list state line, the list `tickets-list` (always drawn, even with
    no rows) and Why blocked under Blocked (`D/view/lists.rs:53-152`).
    Each choice is a `Role::Tab` div with its own focus handle around a
    Kit `Button` (`:155-177`; `D/view/workspace.rs:301-303`).
14. **Rows.** A Kit `List` of `ListItem`s: the title on line 1; short code
    or "No short code", type, status, "Open" or "Closed", and the raw
    RFC 3339 change time on line 2; a reasons line on every row whenever
    any row has reasons (`D/view/tickets.rs:112-210`, `:265`). The row's
    `aria_label` is the title, or the path for a nonconforming entry, and
    its `aria_description` the reasons joined by "; " (`:96-110`,
    `:197-204`). Selection is kept by ID, or by path for an entry with no
    ID (`:253-270`). `LIFECYCLE_OPEN`, `LIFECYCLE_CLOSED` and
    `lifecycle_label` are also used by the item view, the details sheet and
    the relationships panel (`D/view/item/details.rs:31`,
    `item/sheet/ticket.rs:31`, `D/view/relationships.rs:29`).
15. **Query and read.** `TicketQuery { closure, readiness }` is `Copy`,
    maps onto the library's `TicketFilter` through `to_filter`, and every
    filter change reads again (`D/model/tickets.rs:33-57`;
    `D/view/workspace/lists.rs:150-168`). Under Blocked a second read lists
    every open ticket to name blockers (`D/model/tickets.rs:59-67`). The
    desktop does not sort (`:310-322`). `open_ready_tickets` sets Open and
    Ready and reads (`D/view/workspace/lists.rs:174-199`). The ticket
    details sheet suggests field values from the loaded rows,
    `tickets.state().rows()` (`D/view/item/sheet/fields.rs:336-344`).
16. **What one read gives.** Every ticket of every `list_tickets` read
    carries its closure, its readiness state and its blocked reasons,
    whatever the filter (`L/repository/read/items.rs:1689-1696`,
    `:1007-1048`; `L/repository/read/dto.rs:443-450`, `:505-508`); a closed
    ticket's readiness state is `closed` (`dto.rs:328-336`). The library
    orders tickets by change time, latest first, then ID, and puts
    nonconforming entries last in path order (`items.rs:1745-1757`); it
    orders documents by path, nonconforming ones among them
    (`:1729-1738`). A filter never removes a nonconforming entry (`:63`).
17. **Fields.** `ItemDto` has `slug` (lowercase), `title`, `ticket_type`,
    `status`, `closure`, `readiness`, `changed_at` (RFC 3339 UTC, from the
    last index refresh), `context { kind: Primary|Unverified|Active,
    branch: Option<String>, worktree }` and `problems`
    (`L/repository/read/dto.rs:347-356`, `:425-478`, `:536-542`). Active
    means the item has its own worktree (`items.rs:1526-1553`); in this
    repository every open ticket has one. `TicketRow` keeps neither
    `readiness.state` nor `context.kind` (`D/model/tickets.rs:145-194`). The
    details sheet labels the kinds "Primary", "Active item worktree" and
    "Unverified", and a missing branch `NO_BRANCH`
    (`D/model/item.rs:25-28`). A nonconforming entry has no ID, title or
    metadata, only a path and problem codes, and its field is not named
    (`items.rs:528-532`, `:965-980`; `L/results.rs:785-799`).
18. **List states.** `ListState::{Loading, Ready { rows, complete, index },
    Busy, Failed }`, with labels for loading, empty, never refreshed,
    stale, incomplete, busy, failed and index unavailable
    (`D/model/list_state.rs:16-52`, `:101-125`). `Tickets::load` resets the
    list (`forget`, `D/model/tickets.rs:279-284`); `apply_keeping` keeps
    rows over a busy read and replaces them over a failure (ruling R-B5,
    `D/model/list_state.rs:72-98`). Today a failure's message is a visible
    line under the state label, beside Retry (`D/view/lists.rs:208-230`).
19. **Reloads and Retry.** `load_lists` calls `clear_lists` first, which
    clears both list models, both panes and Find, then reads; Retry,
    `select_repository` and `retry_resolution` go through it.
    `reload_lists` keeps rows and selection until the replacement arrives,
    and finished jobs use it (`D/view/workspace/lists.rs:27-58`;
    `D/view/lists.rs:226-230`; `D/view/workspace/repositories.rs:111-122`;
    `D/view/workspace/requests.rs:147-162`).
20. **Find** sits at the top of the Tickets panel (C1 decision 4).
    `secondary-p` reveals Tickets and focuses it. Enter on a ULID opens
    that item, ticket or document, closed or not, through an asynchronous
    `show_item` read that a stamp drops when the selection changes, with
    "Finding", "No match", busy and failure states; other text is looked
    up as a short code with `find_tickets_by_slug`, and every match is
    listed as a card with its context and an Open button
    (`D/model/find.rs:32-140`; `D/view/workspace/find.rs:24-85`;
    `D/view/find.rs:44-171`). Focus stays in the input after a ULID opens
    (`D/view/keyboard_tests.rs:439-443`).
21. **Panel header.** The header is the dock's tab group renderer, outside
    every region's focus subtree; `region()` puts `track_focus` and the
    `Navigation` key context on the panel body only
    (`D/view/workspace/focus.rs:226-255`; `D/view/workspace/render.rs:53-70`).
    `tabs::header` holds the Workspace's `WeakEntity`; `single` and `tabs`
    do not receive it, and `tabs` takes no window
    (`D/view/dock/header/tabs.rs:50`, `:116`, `:168`). `single` draws icon,
    title and count (`title_suffix`, with no element id) inside one row
    that is both a drag source and a drop target (`:138-163`); `tabs` draws
    no count (`:168-219`). The Kit `Panel` has a `toolbar_buttons` hook the
    app does not draw (`C/dock/panel.rs:101-107`). The count is the number
    of rows of `tickets().state()` (C1 R-22; `D/view/dock/panels.rs:166-180`).
22. **Region focus.** `focused_region` is the region whose handle contains
    focus, else none (`D/view/workspace/focus.rs:134-138`); F6 from none
    goes to the first stop (`:204-221`); `focus_region` sends Tickets and
    Documents focus to the list and tree when a repository is set
    (`:165-170`). `NewItem` takes its kind from `focused_region`
    (`D/view/workspace/render.rs:226-231`). The keyboard walk's `ringed`
    asserts, from the snapshot, that exactly one region element holds
    focus and draws its ring (`D/view/keyboard_tests.rs:150-158`,
    `:618-629`).

**Documents today.**

23. **Tree.** The styled Kit `tree()` of `ListItem`s, indented 12 px per
    level, with no chevrons, icons or tooltips (`D/view/documents.rs:129-221`).
    A document shows its title and its path on a second line; a
    nonconforming file its path, "Nonconforming" and its first problem
    code; an empty folder "Empty folder" (`:36-48`, `:148-199`). A document
    opens through `ListItem::on_click` (`:203-213`). Every folder is
    rebuilt expanded on every reload (`:286`), so expansion does not
    survive one. Selection is restored by id with `set_selected_index`
    (`:81-96`).
24. **Model.** `TreeNode::{Folder { path, empty, children }, Document(ItemDto)}`,
    folders first in the folder read's order, documents in the library's
    path order; a document whose folder was not listed goes under the
    nearest listed ancestor (`D/model/documents.rs:23-127`).
    `document_count` counts documents, not folders (`:37-46`). Two reads,
    folders and documents, combine into one state (`:227-253`); a failed or
    busy reload clears the tree (`D/view/workspace/lists.rs:95-103`).
25. **Empty folders.** `DocumentFolderDto.empty` means the directory holds
    nothing at all, not even a non-document file or a subfolder; folders
    come from the primary worktree's index only
    (`L/repository/read/dto.rs:389-417`).
26. **Keys.** Up, down, left and right are the Kit's; Enter is the app's
    `OpenSelected` in the `Documents` context and opens a document,
    ignoring a folder (`D/keys.rs:145-147`, `:255`;
    `D/view/workspace/lists.rs:201-205`). New document… and New folder…
    are body buttons (`D/view/documents.rs:226-242`); New folder… and Move…
    finish through `new_folder.rs:302` and `requests.rs:161`.

**Shared.**

27. **Unsaved work** is not on `ItemDto`. In memory,
    `DraftRegistry::held_by(repo, kind, id) -> Held { Yes, No, Unknown }`
    answers whether a live session or the cached store listing holds a
    draft of the item **or of a comment on it**, with no I/O
    (`D/services/drafts/held.rs:126-170`). Entering edit mode starts a
    session. The Workspace observes the drafts service
    (`D/view/workspace.rs:193`).
28. **Pending conflicts** are kept per repository as
    `Workspace.pending_conflicts: HashMap<RepoKey, Vec<PendingConflict>>`,
    each naming its item's ID and kind and a source that is a sync, a
    local merge, a close or a promotion; setting them notifies
    (`D/view/workspace.rs:141-145`; `D/services/pending_conflicts.rs:40-67`;
    `D/view/workspace/requests.rs:113-116`).
29. **Times and statuses.** `relative_time(at, now)` gives "just now",
    "{n} min ago" … "{n} days ago", then the date; it is `pub(crate)` in
    the private `view::item::chips` module, and a header test imports it
    there (`D/view/item/chips.rs:180-206`; `header_tests.rs:21`). The item
    header tints a status by `tint(status)`: "open", "todo", "in progress"
    and kin as working; "done", "closed", "complete" and kin as done;
    "draft", "review", "in review" as draft; "blocked"; else none
    (`chips.rs:155-167`, `:354-357`).
30. **Index maintenance.** `Workspace::maintain_index(rebuild, cx)` submits
    `IndexRefresh` or `IndexRebuild` for the selected repository as a user
    job (`D/view/workspace/requests.rs:333-350`), bound to `secondary-r` and
    `secondary-shift-r` (`D/keys.rs:240-241`). A finished job reloads the
    lists.
31. **Unreadable files.** A list's `complete` is false only when a
    *directory* holding its kind could not be read; an unreadable or
    non-UTF-8 `.md` file is neither a list entry nor an incomplete list,
    and neither is an unreadable ticket directory
    (`L/repository/read/items.rs:117-120`, `:864-883`;
    `L/repository/discovery.rs:1503-1508`). The index status read,
    `index_status`, is the only read that reports such a file, as an
    `IndexProblemDto { code: SourceUnreadable, path: Option, worktree: Option }`,
    the path relative to its worktree when it can be, else absolute; it
    also reads every stored item and pending operation
    (`L/repository/read/dto.rs:643-668`; `L/repository/read/status.rs:669-697`).
    The desktop does not call it today. A refresh meets the same
    unreadable place again (`dto.rs:376-385`). Opening such a file by path
    answers `repository_inaccessible`, "The repository cannot be read.",
    for a permission failure (`items.rs:1295-1318`; `L/results.rs:555`).
32. **Reads in the desktop** run through `D/services/reads.rs` by
    `ReadKind`, which the test backend can hold (`hold_read`) and count
    (`reads_seen`) (`D/services/mod.rs:91-121`;
    `D/test_support/delaying.rs:135-147`, `:269-284`). There is no
    `ReadKind` for the index status.
33. **Chrome tokens** `selection`, `accent` (amber), `hover`, `muted`,
    `secondary`, `text`, `border`, `teal`, `green` and `red` exist in both
    modes (`D/view/shell/tokens.rs:12-76`).

## Part A: Common Row Anatomy And List States

**Row (`ListRow`).** One element in `D/view/rows.rs` for both panels. It
implements the Kit's `Selectable` (`B/component_traits.rs:3-10`), so the
Kit `List` can use it as its item (fact 1), and the tree draws it directly
(fact 4). It calls `.test_support()`.

- 28 px tall, one line, padding `0 10 0 12` (the tree sets its own left
  indent), gap 8 in Tickets and 5 in Documents (prototype), 13 px text.
- Hover: `hover` background.
- Selected: `selection` background, a 2 px inset left line in `accent`,
  the label at weight 600, and `aria_selected`.
- Keyboard focus: while its list has focus, the selected row also draws a
  2 px inset ring in `teal` (`07`:58; the Kit's outset ring would be
  clipped by the list's `overflow_hidden` wrapper, fact 1). "Its list has
  focus" is the Tickets list's handle being focused, or the Documents
  region containing focus (the tree's handle has no getter, fact 4). The
  region's own focus ring from Cycle 01 stays.
- It takes `expanded(Option<bool>)` for a tree folder's `aria_expanded`,
  since the tree's own wrapper is found only by index (fact 4), and an
  `on_click` (a tree row opens a document through it; fact 5).
- The row keeps the panel's existing id and `aria_label` (`ticket-row`,
  `doc-node`), so a screen reader reads the same name as today and tests
  keep finding rows by label.

**The list stays drawn** (R-34). Whenever a repository is selected, the
Kit `List` and the base `Tree` are drawn in every state, with zero rows
when there are none, and the skeleton, empty state or alert is drawn above
them. So `focus_region`, F6, `secondary-2` and the focus targets after an
action always have their element (fact 22; C1 decision 9).

**States.** A pure function in `D/model/list_view.rs` turns a list's
state into what the panel body shows. Each state has its own text, so
none is shown by colour alone, and D1's distinct states stay distinct.
`listed` is the number of rows the panel would draw: conforming,
nonconforming and unreadable rows in Tickets; every node in Documents.

| State | Body | Text (Tickets; Documents reads "documents") | Action |
| --- | --- | --- | --- |
| No repository selected | message | "Select a repository." | — |
| First load (Loading, nothing kept) | six static skeleton rows | accessible "Loading tickets" | — |
| Reload (Loading, rows kept) | the kept rows; "Updating…" (below) | — | — |
| Ready, current, `listed` > 0 | rows | — | — |
| Ready, `listed` = 0, no narrowing filter | empty state | "No open tickets in {repo}." / "No closed tickets in {repo}." / "No tickets in {repo} yet."; Documents: "No documents in {repo} yet." | New ticket… (Open and All); New document… |
| Ready, `listed` = 0, narrowed, no match elsewhere | empty state | "No tickets match these filters." | Clear filters |
| Ready, `listed` = 0, narrowed, matches elsewhere | empty state | "No tickets here match. 1 more ticket matches with other filters." ("{k} more tickets match …") | Show all |
| Busy, rows kept | alert row above the kept rows | "{repo} was busy, so this list wasn't updated." | Retry |
| Never refreshed | alert row above rows | "{repo} hasn't been scanned yet, so this list may be empty." | Refresh |
| Stale | alert row above rows | "The list for {repo} may be out of date." | Refresh |
| Incomplete | alert row above rows | "A folder in {repo} couldn't be read, so this list may be missing items." | Refresh |
| Busy, nothing kept | alert row | "{repo} is busy right now." | Retry |
| Failed | alert row with a second, visible line, no rows | "Couldn't read the tickets in {repo}. Nothing was changed." then the failure's message | Retry |
| Index unavailable | alert row, no rows | "The search index for {repo} can't be read. Nothing was changed." | Rebuild the search index |

- **Alert precedence.** Busy over kept rows first; then never refreshed,
  stale and incomplete of the rows shown, in that order, as
  `ListState::label` does (fact 18). One alert at a time.
- The alert row is one line, a second for a failure's message, with an
  icon and one action button: `triangle-alert` in amber for busy, never
  refreshed, stale and incomplete; `circle-x` in red for failed and
  unavailable (`06` keeps `circle-alert` for failure and conflict).
- Refresh and Rebuild the search index call `maintain_index` (fact 30).
  Retry reads the list again without clearing it (`reload_lists`, R-35),
  so the filter text, the kept rows and a held ID read stay. Clear
  filters empties the text and sets Any readiness. Show all chooses All
  and Any readiness. New ticket… and New document… are the toolbar's
  actions.
- **Focus after an action** (R-28): the panel's list or tree for Refresh,
  Retry and Rebuild; the filter input for Clear filters and Show all.
  Both are always drawn (R-34).
- Skeleton rows are plain bars in `theme.skeleton`, not the Kit
  `Skeleton`, which pulses (fact 9): `07` lets nothing but the spinner
  animate (R-6).
- **"Updating…"** (R-36) marks a reload over kept rows, so it is told
  apart from a settled list (RFC 57-61) without moving the rows: in a
  single panel's header it replaces the count; in a tabbed group, where
  there is no count (fact 21), the panel's tab reads "{title} · Updating…".
  On a first load there is no count, as today (C1 R-22).

**Kept rows (R-17).** Each list model keeps the rows of its last ready
read for its repository, in a small `KeptList<T>` beside `ListState` in
`D/model/list_state.rs`. A reload shows them until it lands; a busy reload
keeps them under the busy alert; a failed reload replaces them with the
failure, as D1's ruling R-B5 says. A change of repository drops them.
Documents applies its combined tree state (from its two reads) to its
`KeptList`, not each read.

**Repository changed** (R-35). Each list model remembers the last
repository it loaded, which a clear does not reset. A load for another
repository is a change: it drops the kept rows and the unreadable rows,
and (Tickets) clears the filter text and drops a held ID read. A Retry, a
finished job's reload and `retry_resolution` of the same repository are
not changes.

## Part B: The Tickets Model

**One read (R-1).** The panel reads `list_tickets` once per load, with
`TicketFilter::default()`: closure All, no readiness (fact 16). Lifecycle,
readiness, the text filter and the sort are applied in the desktop by a
pure function. Changing any of them never reads again and never leaves a
stale read to drop. The second read under Blocked goes: the blockers are
named from the same rows, which hold every ticket. The details sheet's
suggestions come from the read's open conforming tickets, whatever the
filters, as they did under the default Open filter (R-33; fact 15).

**`TicketRow`** gains `readiness` (Ready, Blocked or Closed), `context`
(its kind, branch and worktree, fact 17) and `reasons` for every blocked
row, named from all the read's rows when the read is applied; the other
fields are unchanged. Unreadable files join as rows (Part I). A row with
no ID is matched for selection by its worktree and path.

**`TicketQuery`** becomes the view's filter: `closure` (default Open),
`readiness` (default none), `text` (default empty) and `sort` (default
Updated, newest first). It is no longer `Copy` and no longer maps onto a
library filter; `query()` returns a reference, and `state()` stays as a
pass-through to the `KeptList`'s state.

- Choosing Ready or Blocked while Closed is chosen chooses Open, and
  choosing Closed sets Any readiness, since only open tickets are ready
  or blocked (R-26).
- The model derives the view whenever the query or the rows change, so
  the view is current the moment a key is handled (R-32).

**`derive(rows, query) -> TicketView`** (`D/model/ticket_view.rs`):

1. **Text (R-16).** A conforming row matches when the trimmed text is
   empty, is a case-insensitive substring of its title or short code, or
   equals its ID ignoring case. Nonconforming and unreadable rows always
   match (R-9).
2. **Counts.** `open`, `closed` and `all` count the conforming tickets
   that match the text, by lifecycle, so Open + Closed = All. The readiness
   choice does not change them (decision 6).
3. **Elsewhere.** `elsewhere` counts the conforming tickets that match a
   non-blank text but are not listed under the current lifecycle and
   readiness (decision 2).
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
8. **Segment counts with nothing to count.** With no repository, on a
   first load, or after a failure with nothing kept, the segments show
   their names without counts; over kept rows they count the kept rows.

**Hint (decision 2).** When rows are listed and `elsewhere` is above zero,
a line above the rows reads "1 more ticket matches with other filters."
or "{k} more tickets match with other filters." with a Show all action
that chooses All and Any readiness. When no row is listed, the empty
state says the same (Part A). So every match of a short code is
reachable, whatever the filters (RFC relationships 146-148).

**Age** (`age_label(changed_at, now)`, R-20), as the prototype writes it
(`Workspace.dc.html:715`): whole minutes under an hour (`0m` to `59m`),
then hours rounded (`{h}h`) under a day, then days rounded (`{d}d`). A time
after `now` is `0m`; an unreadable or missing time is blank. `now` is
passed in, so tests set it.

**Markers** (`row_markers(…)` in `D/model/markers.rs`, shared with
Documents; R-8), from in-memory state only:

- **Unsaved** (`pencil`, amber) when `held_by` answers `Yes`: a draft of
  the item or of a comment on it (fact 27). `Unknown` shows none (R-8).
  A private working copy is not a marker: every open ticket in a
  dogfooding repository has one (fact 17); it is context in the tooltip
  (decision 8).
- **Conflict** (`circle-alert`, red) when the repository's pending
  conflicts name the row's ID.
- **Closed** (`circle-check`, `secondary`) when the row is
  lifecycle-closed and the lifecycle choice is All (decision 5).

**Status colour** (decision 20). One mapping for the list and the item
header: the header's `tint` (fact 29), moved to `D/model/status.rs`, with
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
- The copy it lists from, in the details sheet's words (fact 17):
  "Primary", "Active item worktree" or "Unverified", then
  `Git: branch {branch} · worktree {worktree}`, the branch read as
  `NO_BRANCH` when there is none (PRD MH-CONTENT-004; `06` keeps Git
  detail to tooltips; RFC relationships 146-148's "context").
- Each blocked reason, one per line, as today's reasons read ("Waits for
  open ticket: vh-aaaaa Dock layout").
- `Needs fixing: {guidance} Open it to fix.` for a nonconforming entry,
  with `ProblemCode::guidance()` of its first problem (decision 9).
- `Closed` for a closed ticket.

An unreadable row has its own lines (Part I). The row's `aria_description`
is the same lines joined by "; ". Marker tooltips are also their
accessible names:

- Unsaved: "You have unsaved work on this."
- Conflict: "This has a conflict to resolve. Open it to choose what to
  keep." (fact 28: not every conflict is with the shared copy).
- Closed: "Closed".

## Part C: The Tickets Panel

Top to bottom inside `region-tickets`:

1. **Filter bar** (padding 6 8, bottom border), drawn whether or not a
   repository is selected, as today's choices and Find are (fact 13):
   - **Segments.** The existing choices `filter-open`, `filter-closed` and
     `filter-all` (R-4), drawn as one segmented control: `Open 8 | Closed
     4 | All 12`, counts in `muted` (Part B, item 8), the active segment in
     `border` with `text`. Each stays a tab stop activated by Enter or
     Space, keeps `Role::Tab` and `aria_selected`, and its accessible name
     gains the count when there is one ("Open, 8 tickets"; "Open, 1
     ticket").
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
2. **Filter status** (`filter-status`, Part D), when there is one.
3. **Hint line** (`tickets-hint` with `tickets-show-all`, Part B), when
   there is one.
4. **Column header** (24 px, bottom border; R-2): a 14 px type button
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
   place. It is drawn when a repository is selected.
5. **Body**: the state of Part A above the list, which is always drawn
   (R-34).
6. **Rows**, a Kit `List` of `ListRow`s (R-2), inside a div with the
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
     `ticket-changed`, accessible name "Updated {relative_time}" (fact 29,
     re-exported for the list).
   - Code and Status shrink and clip before the title drops below 96 px
     (R-25).
7. **Footnote** (11 px, `muted`, top border, ellipsis, id
   `tickets-footnote`): Part B's text.
8. **Why blocked**, under Blocked only, as today (fact 13), below the
   list and above the footnote (decision 19).

**Rows are one line** (decision 19). Today's reasons line under each row
goes: the 28 px row has no room for it. A blocked ticket's reasons are in
Why blocked for the selected row, in its tooltip, and in its accessible
description.

**Header count** (R-11): the number of rows listed (`listed`, nonconforming
and unreadable included), as Cycle 01 counted rows listed under the
filters; "Updating…" during a reload over kept rows (Part A). The count's
element gets the id `("panel-count", node)` and its text as `aria_label`,
so tests can read it.

**An empty list draws nothing of the Kit's.** The Kit `List` draws an
Inbox icon when it has no rows (`C/list/delegate.rs:74-85`); under R-34
the ticket delegate's `render_empty` draws nothing, so only Part A's
state shows.

**Selection** (R-7) stays the list's own: set by a click (which also
opens, fact 1) or the arrows, kept by ID (or worktree and path) across
reloads and filter changes, and shown with the selected treatment. When a
filter or reload hides the selected row, the selection is cleared, so
Enter never opens a different ticket (Review Focus 1). It does not follow
the item area's tab, unlike the prototype (`:1017`).

**Home and End** (R-15) select and scroll to the first and last row; the
Workspace is notified, so Why blocked follows (fact 1).

**Markers and the pane** (R-32): `TicketView` is derived in the model when
the read or the query changes. Markers are worked out on every Workspace
render for the rows listed, so a new draft or pending conflict shows at
once (facts 27, 28); the pane is updated only when its rows or markers
changed, and `submit_filter` updates it before it selects (Part D).

## Part D: The Filter Input Replaces Find

Decision 2. The design draws no Find in the Tickets panel, and its filter
"matches title and short code. Duplicate short codes show all matches"
(`03`:50). The input takes over Find's place, key and ULID path:

- It is the Workspace's existing `find_input` entity, kept under its name,
  with the new placeholder and `set_clean_on_escape(true)` (fact 7). It is
  drawn whether or not a repository is selected, as Find is.
  `secondary-p` still reveals Tickets and focuses it (C1 P15).
- **One source** (R-30). `Workspace::set_filter_text(text, window, cx)`
  sets both the input and `TicketQuery.text`. Typing (Change) sets the
  query; the list, counts, hint and footnote follow each keystroke.
- **Enter** (`submit_filter`), after bringing the pane up to date:
  - text that parses as a ULID opens that item through Find's existing
    stamped `show_item` read, ticket or document, closed or not, whatever
    the filters (fact 20; RFC 67-71). `filter-status` says "Opening…"
    while it is held, then nothing once the tab opens, "Nothing in {repo}
    has that ID." when it is not found, "{repo} is busy right now." when
    busy, or the failure's message; Enter again tries again, as with Find
    today, so there is no Retry control. A read held
    across a change of repository is dropped, as today (RFC 341-342).
    Focus stays in the input (fact 20);
  - otherwise, with non-blank text, when exactly one conforming ticket
    matches it under any lifecycle and readiness, that ticket opens and
    focus stays in the input, even if the filters hide it (the hint has
    named it);
  - otherwise, when rows are listed, focus moves to the list with its
    first row selected; with none, nothing happens.
- **Escape** clears the text (fact 7).
- **Repository change** (R-35). The text is cleared when the selected
  repository changes, not on a Retry or a reload of the same one. The
  clear is applied at the next render, which has the window `set_value`
  needs (fact 7). Lifecycle, readiness and sort carry over, as the filters
  do today.
- With no repository selected, Enter says "Select an available
  repository to find in" in `filter-status`, as Find does.
- **Goes:** the match cards, their Open buttons and texts, the
  `find_tickets_by_slug` lookup and its results. **Stays:** the ID parse
  (`parse_id`), the stamped ID read and its states (R-31).

## Part E: The Documents Model

**Expansion (R-12).** `Documents` keeps, per repository for the session,
the set of folder paths the person has collapsed. A folder with child
nodes that is not in the set is expanded, so a new folder starts
expanded, as today (decision 13; the prototype starts with most folders
collapsed, `:733`). A folder with no child nodes cannot toggle (fact 5)
and is never "expanded". The tree's `TreeEvent::Expanded` and
`Collapsed`, mapped from the `folder:{path}` ids, update the set through a
subscription in `Workspace::new` (fact 5). Enter on a folder (R-24) and
Collapse all change the set and rebuild the tree, since the Kit's toggle
is private (fact 5). Building the tree sets each `TreeItem::expanded`
from the set, so a reload or a hidden panel keeps it. It is not written to
disk (K-5).

**Selection after a rebuild** (R-27): restored with
`set_selected_index(index_of(id))`, never `set_selected_item` or
`reveal_item`, which would expand ancestors and write that back through
the subscription (fact 5). When the row is now hidden in a collapsed
folder, its nearest visible ancestor is selected. New folder… and Move…
expand the destination folder's ancestors, so what they made or moved is
visible (`new_folder.rs:302`, `requests.rs:161`).

**Row facts** (`doc_row(node, expanded) -> DocRow`):

| Node | Label | Expanded | Trailing | Tooltip and description |
| --- | --- | --- | --- | --- |
| Folder with child nodes, expanded | last path segment | yes | — | its path |
| Folder with child nodes, collapsed | last path segment | no | its document count | its path |
| Folder with no child nodes, not empty (fact 25) | last path segment | — | its document count (`0`) | its path |
| Folder, empty (fact 25) | last path segment | — | "empty" (decision 14) | "Empty folder. Shared once it contains a document." |
| Document | its title | — | the unsaved marker | its path |
| Nonconforming, adoptable | file name | — | `triangle-alert` | "Not a managed document yet. Open to adopt it." then its path |
| Nonconforming, any other | file name | — | `triangle-alert` | "Needs fixing: {guidance} Open it to fix." then its path |
| Unreadable (Part I) | file name | — | `circle-x` | Part I's lines |

- The adoptable test is the item view's own `unmanaged(item)`
  (`D/view/item/repair.rs:102-114`), moved unchanged to `D/model/item.rs`
  with what it needs; its callers in `item/banners.rs` and
  `repair/adopt.rs` follow. The tree and the banner then agree
  (decision 9).
- The count counts documents under the folder, nonconforming and
  unreadable included (`document_count`, fact 24), as the header count
  does.
- The icon follows the row: `folder-open` when expanded, else `folder`;
  `file-text`; `file-exclamation-point` with an amber label for
  nonconforming; `circle-x` with a red label for unreadable.
- Order is unchanged: folders first, then documents in path order, an
  unreadable file among them by its path (fact 24).

## Part F: The Documents Panel

- **Tree** (R-3): the base `Tree` with `ListRow` rows and
  `.vertical_scrollbar` (fact 4), in `region-documents`, always drawn when
  a repository is selected (R-34).
- **Row**: indent `8 + 14 × depth` px; a 12 px chevron (`chevron-right`
  or `chevron-down`) for a folder with child nodes, else a 12 px spacer;
  the icon at 14 px; the label; the trailing cell. Ids: `doc-node` (the
  row, `aria_label` the label, `aria_description` the tooltip text,
  `aria_expanded` through `ListRow::expanded`), `doc-count`, `doc-empty`,
  `doc-nonconforming`, `doc-unreadable`, `doc-unsaved`.
- **Mouse**: a click on a folder toggles it (the tree's own mouse-down,
  fact 5); a click on a document opens it (`ListRow::on_click`).
- **Keys**: up and down (the tree's); Left collapses and Right expands
  (the tree's); Enter opens a document and toggles a folder (R-24); Home
  and End select the first and last row (R-15).
- **States**: Part A, with the Documents texts. The header count stays
  documents listed (C1 R-22), "Updating…" during a reload (R-36).

## Part G: Panel Toolbars

`02` puts each panel's toolbar at the right of its header, and Cycle 01
left the header without one (fact 21; C1 Exclusions, "Panel toolbars go
into this Cycle's panel header").

- **Placement.** The header draws the toolbar of the panel it shows: for a
  single panel at the right of its title row; for a tabbed group, the
  active tab's at the right of the tab strip. `single` and `tabs` receive
  the Workspace handle `tabs::header` holds (and the window), and ask it
  for the panel's toolbar (R-13). The drag source and drop target is the
  title area (flex 1), which keeps the id `("panel-title", node)` the dock
  tests drag from (`dock_tests/moves.rs:196`), not the whole row, and the
  toolbar wrapper stops
  mouse-down from propagating, so pressing a button never drags the panel
  (R-37; fact 10, fact 21).
- **Buttons.** The base `gpui_kit::base::Button` with
  `.track_focus(&handle)` on a Workspace-owned focus handle, a Lucide
  icon and `.tooltip(…)` from `Tooltip::new(…)` (with `.action(&NewItem,
  Some(NAVIGATION_CONTEXT))` for the New buttons, so the platform's key
  shows), because the component `Button` keys its own handle to its
  element path, which a dock `load` or move changes (fact 10; R-37).
- **Focus counts as the panel's region** (decision 18). `focused_region`
  and `cycle_region` count a focused toolbar handle as its panel's region,
  so the region's ring shows, F6 and Shift F6 move on from that region
  (C1 decision 9), and `NewItem`, which takes its kind from
  `focused_region` (fact 22), makes the panel's kind. The toolbar carries
  the `Navigation` key context, so `secondary-n` reaches `NewItem` there.
  The keyboard walk's `ringed` helper reads the snapshot's element focus,
  which a header button is never inside (fact 22), so it is edited to ask
  the Workspace's `focused_region` as well (E-14).
- Buttons: 26 × 24 px, radius 4, 14 px icon in `secondary`; hover `hover`
  with `text`. Each is a tab stop activated by Enter or Space, with an
  accessible name and a tooltip:

| Panel | Id | Icon | Name and tooltip | Does | Disabled when |
| --- | --- | --- | --- | --- | --- |
| Tickets | `new-ticket-button` | `plus` | "New ticket…", with its key | New ticket…, as today's body button | no repository resolved |
| Documents | `new-document-button` | `file-plus` | "New document…", with its key | New document…, as today | no repository resolved |
| Documents | `new-folder-button` | `folder-plus` | "New folder…" | New folder…, as today | no repository resolved |
| Documents | `collapse-all-button` | `chevrons-down-up` | "Collapse all folders" | Part E's Collapse all | no folder is expanded |

- When a button becomes disabled while it holds focus (Collapse all after
  it runs), focus moves to the tree.
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
- **Tab order** in Tickets: the toolbar; the segments, the readiness menu
  and the filter input; the hint's
  Show all when shown; the five column headers; the alert's or empty
  state's action when shown; Why blocked when shown. In Documents: the
  toolbar, then the alert's or empty state's action when shown. The list
  and the tree are not tab stops (facts 1, 4), as today: they are reached
  by F6, `secondary-2`, a click, or Enter in the filter input (Part D;
  K-11).
- Every new control shows focus: buttons and the input draw the Kit ring;
  segments and column headers draw the app's ring as today's choices do;
  the list's selected row draws Part A's inset ring.

## Part I: Unreadable Files

Decision 10. The Wave's exit evidence asks for inaccessible items to stay
visible with their reason, and the read that reports them exists
(fact 31).

- **The read.** Each load reads `index_status` once, beside the lists,
  through `D/services/reads.rs` under a new `ReadKind::IndexStatus`
  (fact 32). Its completion carries both lists' stamps and is applied to
  each list whose stamp is still current.
- **The mapping.** `unreadable_rows(status, root) -> (Vec<UnreadableFile>, Vec<UnreadableFile>)`
  (`D/model/unreadable.rs`) picks the `SourceUnreadable` problems whose
  path is relative and names a `.md` file under `.manyhands/tickets/`
  (Tickets) or `docs/` (Documents), with the worktree each was found in.
  A problem with no path, no worktree, or an absolute path is skipped, as
  are other codes and directories: those are Activity's (Cycle 03) or the
  incomplete alert's.
- **Kept beside the rows.** Each list model holds its unreadable files
  beside its `KeptList`, under the same rules: replaced when a status read
  lands, kept over a busy or failed status read and during a reload,
  dropped on a change of repository (R-35). They merge into the rows when
  the view is derived (Tickets) or the tree is built (Documents); the tree
  rebuilds when they change. A failed or busy status read never fails the
  lists.
- **The row.** Tickets lists them after the nonconforming rows; Documents
  places each in its folder by path. A row shows its path (Tickets) or
  file name (Documents), "can't be read" (Tickets' pill), the `circle-x`
  icon (`doc-unreadable` in Documents), and the tooltip and description
  lines `Can't be read: {guidance}`, its path, and, when it is not the
  primary worktree, `Git: worktree {worktree}`. The same path can appear
  once per worktree; selection keys on worktree and path.
- **It does not open** (R-38). Opening it by path would answer "The
  repository cannot be read." for a permission failure (fact 31), which
  blames the whole repository. Enter and a click select it; its reason and
  path are in its tooltip and description. This narrows decision 10's
  "opens by path".
- The footnote counts them (Part B). The panel header count includes
  them (R-11), and so does a folder's document count.
- Its cost: one more read per load, which reads every stored item and
  pending operation again (K-10).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | A repository's lists load | One `list_tickets` read with the default filter, the two document reads and one `index_status` read; no second ticket read under Blocked |
| P2 | Open, Closed or All chosen; Ready or Blocked chosen; text typed; a column sorted | The list, counts, hint and footnote follow at once; no read is sent |
| P3 | The default | Open, Any readiness, no text, Updated newest first: the library's order |
| P4 | A ticket whose status text is `closed` but lifecycle open | Listed under Open, its pill reads `closed` |
| P5 | Under All | A lifecycle-closed row has a `secondary` title and the Closed marker |
| P6 | Nonconforming and unreadable rows | Listed under every lifecycle, readiness and text, last in read order under every sort, with their reason |
| P7 | A short code typed that tickets outside the filters also carry | The hint, or the empty state, names how many more match; Show all lists them all; each opens its own ticket by ID |
| P8 | A ULID typed, then Enter | That ticket or document opens, closed or not, whatever the filters; "Opening…" while held; "Nothing in {repo} has that ID." when not found; busy and failure said, Enter again retrying; dropped when the repository changes first; focus stays in the input |
| P9 | Enter with non-blank text and exactly one match under any filters; with several | It opens; focus moves to the list with the first row selected |
| P10 | Segment counts | Conforming tickets matching the text, by lifecycle; Open + Closed = All; readiness does not change them; none shown with nothing to count |
| P11 | Ready or Blocked chosen through the menu, by mouse and by keyboard | The rows of that readiness; the button names it; Why blocked under Blocked; focus back on the button; Closed becomes Open |
| P12 | A column header activated, by click and by Enter | Sorted ascending; again, reversed; the name says the order |
| P13 | A ticket with no short code | `—`; the tooltip and description carry its ID |
| P14 | A ticket with unsaved work (item or comment draft); with a pending conflict | The unsaved marker; the conflict marker; they appear without a list read |
| P15 | Each list state of Part A, in both panels | Its own text and action; Retry reads without clearing, Refresh and Rebuild submit their jobs, Clear filters and Show all change the filters and focus the input |
| P16 | A reload of a listed repository | The kept rows stay until it lands, marked "Updating…"; a busy reload keeps them under the busy alert; a failed one shows the failure with its message visible |
| P17 | A change of repository | The text is cleared; lifecycle, readiness and sort kept; kept and unreadable rows dropped; a held ID read dropped. A Retry of the same repository keeps all of them |
| P18 | Home and End in the Tickets list and in the tree | First and last row selected and scrolled to |
| P19 | A folder collapsed, then the list reloaded, then the panel hidden and shown | Still collapsed |
| P20 | Collapse all | Every folder collapsed; the hidden selected row's top-level folder selected; focus on the tree |
| P21 | Enter, a click, Left and Right on a folder | Toggle, toggle, collapse, expand |
| P22 | A collapsed folder; a leaf folder with no documents; an empty folder | Its count; `0`; "empty" with its explanation |
| P23 | A nonconforming document | Its file name in amber, the marker, and the adopt or fix text by its problem |
| P24 | A toolbar button by Tab and Enter, in a single panel and in a tabbed group | The same action as its click; its panel's region holds focus and draws its ring, F6 moves on, `secondary-n` makes its kind; pressing it never drags the panel; disabled with no repository; refused behind a dialog |
| P25 | Any filter, sort or expansion change, or Collapse all | No tab opens, closes or reloads; nothing is saved, synchronized or sent (C1 contract 7) |
| P26 | Hiding and showing either panel | Filters, text, sort, selection and expansion kept (C1 P2) |
| P27 | A list holding only nonconforming or unreadable rows; a tree of folders only | Rows, never the empty state |
| P28 | An unreadable ticket or document file | A row with its reason in its list; it does not open; a failed or busy status read leaves the lists as they were |
| P29 | The header count | Rows listed under the filters; "Updating…" during a reload, on the tab in a tabbed group |
| P30 | A filter hides the selected row, then Enter in the list | Nothing opens |
| P31 | F6 or `secondary-2` into a list that is empty, loading or failed | Its region holds focus and draws its ring |
| P32 | The details sheet's suggestions | Values from the repository's open conforming tickets, whatever the filters |

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
- Reads are counted with `reads_seen` and held with `hold_read`
  (fact 32); busy and failed reads are made as the existing tests make
  them (`a_busy_list_reload_keeps_the_last_rows_with_a_label`; the
  failed read through `DelayingBackend::replace_service`
  (`D/test_support/delaying.rs:257`); a state no seam can make stops the
  task for the owner.
- No real waits; `now` is passed in.

| Test | What |
| --- | --- |
| `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts` | One table: open, closed, ready, blocked, no-code, duplicate-code, status `closed` while open, empty status, nonconforming and unreadable rows under every lifecycle, readiness, text and sort it needs; rows in order, counts, `elsewhere`, `listed`, `shown` and footnote (P2–P7, P10, P13, P27) |
| `desktop::model::ticket_view::tests::ages_read_as_the_prototype_writes_them` | One table of `age_label` (R-20) |
| `desktop::model::ticket_view::tests::tooltips_carry_the_id_context_and_reasons` | One table of `ticket_tooltip`: coded, no code, each context kind with and without a branch, blocked with an open dependency and on a cycle, nonconforming, unreadable in an item worktree, closed (P13) |
| `desktop::model::markers::tests::markers_follow_drafts_conflicts_and_closure` | One table of `row_markers`: `Held` Yes, No and Unknown, a conflict, closed under each lifecycle (P5, P14) |
| `desktop::model::list_view::tests::each_state_has_its_own_body` | One table: every row of Part A's state table for each panel, `listed` 0 and above, `elsewhere` 0 and above, the alert precedence, with text and action (P15, P27) |
| `desktop::model::list_state::tests::kept_rows_follow_reloads` | One table of `KeptList`: reload, then ok, busy or failed; a combined state (P16) |
| `desktop::model::tickets::tests::a_read_lands_only_for_its_repository_and_load` | One table (E-1): a read for the load in flight lands, with blocked reasons named from all its rows; one for an earlier load, another repository or after a clear is dropped; a query change reads nothing and derives at once; a load of the same repository after a clear is not a change, of another one is (P2, P17) |
| `desktop::model::unreadable::tests::unreadable_files_go_to_their_list` | One table of `unreadable_rows`: ticket file, document file, a file in an item worktree, a directory, a file elsewhere, an absolute path, no path, another code (P28) |
| `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all` | Toggle, rebuild with a new folder and with a folder gone and back, Collapse all, `expand_ancestors`, another repository; the set and the built `expanded` flags; the selection rule (P19, P20) |
| `desktop::model::documents::tests::rows_read_as_the_design_writes_them` | One table of `doc_row` over Part E's eight kinds: label, expanded, trailing and tooltip (P22, P23) |
| `desktop::view::tickets::tests::the_filter_bar_works_without_a_read` | A journey: one ticket read on load; segments by click and key; the readiness menu by keyboard; text with the hint and Show all, under a readiness choice too; sorts by click and by Enter; Home and End; a filter hiding the selected row; header count, footnote and segment names at each step; no read or request after the load (P1, P2, P7, P10–P12, P18, P25, P29, P30) |
| `desktop::view::tickets::tests::list_states_act_and_a_reload_keeps_the_rows` | Retry reads without clearing and keeps the text; Refresh submits `IndexRefresh`; Rebuild the search index submits `IndexRebuild`; Clear filters clears and focuses the input; the empty state's New ticket… opens the dialog; a job-driven reload keeps the rows with "Updating…"; a held busy reload keeps them under the busy alert; a failed reload shows the failure and its message; F6 into the empty and the failed list keeps the region's ring (P15, P16, P31) |
| `desktop::view::tickets::tests::markers_appear_without_a_read` | Start editing a ticket (D3's editor-test rules): its row shows `ticket-unsaved` with no list read (P14) |
| `desktop::view::tickets::tests::an_unreadable_file_is_listed_with_its_reason` | A ticket file and a document file written as invalid UTF-8, then a refresh job: each listed with "can't be read"; Enter does not open it; the footnote ends `· 1 can't be read`; a held then busy status read leaves the lists unchanged (P1, P28) |
| `desktop::view::documents::tests::folders_keep_their_state_and_toggle_by_key` | Left, Right, Enter, a click; a reload; hide and show; Collapse all; Home and End; Enter on a document opens it; a Documents stale or never-refreshed alert's Refresh (P15, P18–P21, P25, P26) |
| `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter` | Each toolbar button reached by Tab and run by Enter, single and tabbed; region, ring, F6 and `secondary-n` from a button; a press and small move drags nothing; disabled with no repository; refused behind a dialog (P24) |
| `desktop::view::find::tests::*` (rewritten, E-4) | ULID of a closed ticket under Open, of a document, unknown, held across a repository change; one match outside the filters; several matches; focus (P8, P9, P17) |

Pre-existing tests change as the named edits say.

## Named Edits To Pre-Existing Tests

At `a0a734a4`. Task 0 re-counts with `git grep`, over tests and
production code, and records any other site as a further edit
(decision 16). Edits that remove assertions are marked **removes**.

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `D/model/tickets_tests.rs`: `the_query_maps_closure_and_readiness_and_nothing_else`, `a_filter_change_drops_the_last_filters_read`, `set_query_keeps_the_query_and_drops_the_last_read`, `a_readiness_change_drops_the_last_readinesss_read`, `rows_read_under_blocked_carry_their_reasons`; the query set-up of `blocked_reasons_name_the_blocker_the_missing_id_and_the_cycle` (`:259-266`) and `an_untitled_blocker_is_named_by_its_short_code` (`:305-312`) | The five become one table test, `a_read_lands_only_for_its_repository_and_load`, keeping the repository-change, clear and generation guards (`:157-169`), the query kept before a load, and the blocked reasons filled when a read is applied; the mapping to a library filter and "a filter change drops the read" go (R-1). `the_default_query_lists_open_tickets` asserts the default query and `read_filter() == TicketFilter::default()`. The `blocked_reasons` tests build `TicketFilter` literals. **Removes** | B |
| E-2 | `D/view/tickets_tests.rs`: `a_filter_change_drops_the_held_read_of_the_last_filter`, `a_readiness_change_drops_the_held_read_of_the_last_one`; helpers `library()` (`:49-57`) and `blocked_list` (`:449-457`) and the query at `:322-326`; their users `arrows_move_in_the_list_and_enter_opens`, `each_closure_filter_…` (`:122`), `the_panel_says_incomplete_…` (`:552`) | The two tests are deleted: a filter change no longer reads; the repository-change guard is E-1's table and P17's view step. The helpers and `:322-326` take `TicketFilter` literals. **Removes** | B |
| E-3 | `D/view/tickets_tests.rs`: `each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`, `ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry`, `the_why_blocked_panel_gives_the_selected_rows_reasons_in_full`, `the_panel_says_incomplete_where_the_library_does`, `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text`, `a_long_why_blocked_panel_scrolls_by_keyboard` | Each still compares the rows shown under each choice with `list_tickets(TicketFilter { closure, readiness })` (contract 1). `ticket-lifecycle` becomes the `ticket-closed` marker under All; the nonconforming row's `ticket-nonconforming` and `ticket-problem` cells (`:142-154`) become its "needs fixing" `ticket-status`, and `expected()` changes with it; readiness is chosen through `filter-readiness`; `ticket-reason-{i}-{n}` becomes Why blocked's lines; the `row_description` check becomes `ticket_tooltip`'s; the "Showing …" texts go, so the label test checks the segments' names with counts and the menu button's name. **Removes** the "Showing …", inline-reason and problem-code assertions | C |
| E-4 | `D/view/find_tests.rs` (8) | Rewritten for the filter input. `secondary_p_focuses_find`, `a_ulid_opens_its_ticket_closed_or_not`, `find_does_nothing_while_a_dialog_is_open`, `a_held_search_never_lands_after_the_selection_changed` (the ULID read) and `find_needs_a_selected_repository_and_a_new_selection_clears_it` (the text is cleared) keep their intent; `no_match_says_so_and_opens_nothing` checks "Nothing in {repo} has that ID." and, for text, "No tickets match these filters."; `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` lists both as rows, one through Show all. Steps for a document's ULID and one match are added. `a_matchs_open_is_reached_by_tab_and_enter_opens_it` is deleted (no match cards), and the cards' context labels are no longer asserted (the context is in the tooltip, tested by its function). **Removes** | D |
| E-5 | `D/model/find_tests.rs` | The short-code tests go; `a_ulid_is_an_id_and_anything_else_is_a_short_code` (`:17`) becomes a `parse_id` table; `an_id_found_opens_by_its_id_and_one_not_found_is_no_match` and `an_earlier_search_is_dropped` stay, rewritten to the ID read's states. **Removes** the short-code assertions | D |
| E-6 | `D/view/keyboard_tests.rs` `change_the_filters` (`:345`), `find` and `find_by_short_code_and_ulid` (`:430`, `:450`) | New Tab order (Part H): edited in Task 3 for the filter bar and again in Task 8 for the toolbar; readiness through the menu; Find steps use the filter input. **Removes** the "Showing …" checks and the match-card Tab steps | C, D, G |
| E-7 | `D/view/keyboard_tests/editing.rs` `create_by_keyboard_alone` (`:709-736`), `move_and_new_folder_by_keyboard_alone` (`:847-886`); `D/view/documents_tests.rs` `new_folder_opens_its_dialog` | Shift-Tab from the list or tree no longer reaches the body buttons; the steps reach `new-ticket-button`, `new-document-button` and `new-folder-button` in the header, where `walk.on(…, region)` holds through E-14 | G |
| E-8 | `D/view/documents_tests.rs` `the_tree_shows_what_the_library_reads_give`, `arrows_move_in_the_tree_and_enter_opens` (Enter on a folder now toggles it), `a_held_tree_of_a_never_replaces_bs` (`:239`, `:245`), and the exhaustive `match` at `:100-111` (Task 7); `D/view/new_folder_tests.rs` (`doc-empty` text); `D/view/move_document_tests.rs` helper `tree_paths` (`:274-279`) | The nonconforming label is the file name; "Empty folder" becomes "empty"; `doc-path`, `doc-nonconforming` and `doc-problem` lines go, and `tree_paths` reads document paths through a `#[cfg(test)]` accessor on `DocumentsPane`. **Removes** the second-line assertions ("Empty folder", "Nonconforming", the problem code, the path); `doc_row`'s table carries their content | E, F, I |
| E-9 | Keyboard helpers that open a nonconforming file by its path label: `keyboard_tests/actions.rs` `a_nonconforming_entry_opens_by_keyboard`, `keyboard_tests/editing.rs` `repair_and_adopt_by_keyboard_alone` | They look for the file name | E |
| E-10 | `D/view/dock_tests/state.rs` `a_hidden_panel_keeps_its_state` (also chooses a sort and Ready before hiding, and checks they are kept, P26); `D/view/lifecycle/made_ready_tests.rs` (`:288-289`, `filter-ready`), `made_ready_review_tests.rs` (`:65-70`, `filter-ready`, `filter-blocked`) | The kept Find text becomes filter text that matches both rows; the made-ready set-up, which R-26 makes unreachable (Closed with Blocked), starts from Closed with typed text, and asserts that `open_ready_tickets` gives Open, Ready and no text (R-29); readiness is read from `filter-readiness`'s name | C, D |
| E-11 | `D/assets_tests.rs` `every_item_view_icon_loads` | The table and its array length gain `repeat`, `bug`, `sparkles`, `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`, `arrow-up-down`, `list-filter` and `file-exclamation-point` | A |
| E-12 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | The table gains `home` → `SelectFirst` and `end` → `SelectLast` | H |
| E-13 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour and stops for the owner. **Removes** | — |
| E-14 | `D/view/keyboard_tests.rs` `ringed` (`:618-629`) and `on` (`:150-158`) | A region "holds focus" when its element's snapshot is focused or the Workspace's `focused_region` names it (a header toolbar button); the ring-element checks are unchanged. This loosens Cycle 01's helper for decision 18 | G |

## Document Amendments

To apply in the approval commit:

1. **Wave Cycle 02.**
   - In scope, Tickets: the footnote reads as decision 7; "Ready and
     Blocked filters keep working, in a filter menu" adds "Why blocked
     stays under Blocked; a row's reasons move to Why blocked and its
     tooltip (decision 19)"; add "Find becomes the filter input (decision
     2)"; the toolbar reads "New ticket (More waits for an action,
     decision 12)"; "status pills by the `03` color table" reads "status
     pills by the item header's mapping (decision 20)".
   - In scope, list states: "failed or stale scan with Retry" reads
     "failed with Retry, stale and never refreshed with Refresh"
     (decision 11).
   - Exit evidence: "nonconforming and inaccessible items stay visible
     with their reason" adds "unreadable files from the index status read
     (decision 10); they do not open (design R-38)".
2. **Wave Deferred Work.** Add: saving the ticket sort, filters and folder
   expansion with the layout (K-5).
3. **Cycle 01 document, decision 4.** Add: "Superseded in part by Wave 04
   Cycle 02 decision 2: Find is now the Tickets filter input; `secondary-p`
   and the open-by-ID path are kept."
4. **Handoff `08`.** The Tickets list is a Kit `List` with an app row and
   an app column header, not `DataTable` (fact 3); the Documents tree is
   the base `Tree` with app rows (fact 4); the segmented control is the
   app's own (fact 6); toolbar buttons are the base `Button` with
   caller-owned focus (fact 10); `file-warning` is
   `file-exclamation-point` (fact 12).
5. **Handoff `03`, Tickets.** Short codes appear as stored, in lowercase
   (decision 3); the status pill uses the item header's mapping
   (decision 20); "Ctrl N" is the platform's New key.
6. **Ticket relationships RFC, Desktop (318-331).** "A find-by-slug entry
   lists every match with its context" reads: "The Tickets filter lists
   every ticket with that short code under the filters chosen and says
   how many more match outside them, each with its context in its
   tooltip." "Is shown by its ULID" (132-134) adds: "in the desktop list,
   `—` with the ULID in the row's tooltip" (decisions 2, 4).

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
| R-8 | Markers come from in-memory state only: `held_by`, the repository's pending conflicts, the row's closure. `Held::Unknown` shows no marker, so a store that cannot be read does not mark every row (unlike the retirement path, for which `held.rs:131` counts an unknown listing as a draft) | A draft in an unreadable store shows no marker |
| R-9 | Nonconforming and unreadable rows pass every filter, the text included, and sort last in read order | — |
| R-10 | The sort is stable over the library's order; descending reverses the rows that have the value; rows without it sort last in either direction | — |
| R-11 | The header count is the rows listed, nonconforming and unreadable included, as C1 R-22 counted rows listed under the filters | — |
| R-12 | Expansion per repository, session only, as a set of collapsed folder paths; absent means expanded for a folder with child nodes; `TreeEvent` updates it; `set_items` builds each item's `expanded` from it | Expansion is lost at restart (K-5) |
| R-13 | `single` and `tabs` receive the Workspace handle and window and draw `Workspace::panel_toolbar(kind)` for the panel shown | — |
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
| R-27 | After a rebuild the selection is restored by index, never by an expanding call; a hidden selected row gives it to its nearest visible ancestor; New folder… and Move… expand the destination's ancestors | — |
| R-28 | Focus after a transient action: the list or tree for Refresh, Retry and Rebuild; the filter input for Clear filters and Show all; the tree when Collapse all disables itself | — |
| R-29 | `open_ready_tickets` chooses Open and Ready, clears the text, keeps the sort and reads once | — |
| R-30 | The filter text has one source: `set_filter_text` sets the input and the query; a repository change clears it at the next render | — |
| R-31 | Find's stamped ID read and its states stay; only the short-code lookup goes | — |
| R-32 | `TicketView` is derived in the model when the read or the query changes; markers on every Workspace render; the pane is updated only when its rows or markers changed, and before `submit_filter` selects | Markers cost a little per render |
| R-33 | The details sheet suggests values from the read's open conforming tickets, whatever the filters, so `suggestions_come_from_the_loaded_tickets_values` holds unchanged (fact 15) | A value used only on closed tickets is not suggested, as under today's default |
| R-34 | The Kit `List` and the base `Tree` are drawn in every state while a repository is selected, with zero rows when there are none | — |
| R-35 | Retry reloads without clearing. Each list remembers the last repository it loaded, which a clear does not reset; only a load of another repository is a change | — |
| R-36 | "Updating…" replaces a single panel's header count, or follows its tab's title in a tabbed group, while a reload runs over kept rows | — |
| R-37 | Toolbar buttons are base `Button`s with Workspace-owned focus handles, outside the header's drag source, in a wrapper that stops mouse-down propagation | — |
| R-38 | An unreadable row selects but does not open; its reason and path are in its tooltip and description (fact 31) | The person must find the file themselves |

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
  included (R-1), and the index status read reads every stored item and
  pending operation again (fact 31).
- **K-11.** The list and the tree are not Tab stops, as today (facts 1,
  4): they are reached by F6, `secondary-2`, a click, or Enter in the
  filter.
- **K-12.** Short codes longer than about 11 characters are clipped in
  the Code column; the tooltip shows them whole.
- **K-13.** An unreadable ticket *directory* hides its ticket without a
  row or an incomplete alert (fact 31); the index status read reports it
  as a directory, which this Cycle leaves to Activity.
- **K-14.** An unreadable row does not open (R-38).

## Unverified

- **U-3.** That `single` and `tabs` can draw Workspace-built base buttons
  whose focus `focused_region` counts as the panel's region, outside the
  drag source (decision 18, R-13, R-37). Task 8's test proves it.

U-1, U-2, U-4 and U-5 were settled in review (facts 1, 4, 8, 27, 28).

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
- **The component `Button` in the toolbar:** its focus handle is keyed to
  its element path, which a dock `load` changes (R-37).
- **Accepting the unreadable-file gap:** the read that reports them
  exists, and the Wave's exit evidence asks for them (decision 10).
- **Opening an unreadable file by path:** the read answers that the whole
  repository cannot be read (R-38).
- **Taking the list out of the tree in the empty and failed states:** focus
  would have no element (R-34).

## Risks

| Risk | Control |
| --- | --- |
| The redesign reaches many existing tests (labels, ids, Tab order) and the `TicketQuery` change reaches production code | E-1 to E-14 with their sites; ids kept where the element survives; Task 0's sweep over tests and production; each task runs the tests it touches by exact name |
| One unfiltered read and the status read are slower on a large repository | The library already loads every stored item for any filter (`items.rs:1712-1724`); K-10; the performance work the owner put next can add a count read |
| Custom rows lose the Kit's list accessibility | The Kit `List` and base `Tree` keep their roles, positions and selected state on the row wrapper (facts 1, 4); rows keep their ids and names, and `ListRow` adds `aria_expanded` |
| Find's removal loses a path to closed tickets and documents by ID | Part D keeps the stamped ULID read, its feedback and its test (P8) |
| Header focus counted as a region breaks C1's focus rules | Decision 18 keeps one region per focus; E-14 states the helper's change; the toolbar test runs F6, the ring and `secondary-n` from a button |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | The design's segment counts need every lifecycle; today each filter is its own read | Ruling | R-1 | Fact 16 |
| Planning research | Find and the design's filter input both match short codes | Decision | Decision 2, Part D | Fact 20; `03`:50 |
| Planning research | Upper- vs lowercase codes; `—` vs ULID; closed by colour alone; counts and footnote; nonconforming wording; list states; expansion | Decisions | Decisions 3–9, 11, 13, 14 | Revision 1 Review Record |
| Planning research | Kit list, tree, table, segmented, menu and skeleton facts | Rulings | R-2 to R-6 | Facts 1–9 |
| Review round 1, source and feasibility (`563763e2`) | 4 Blocking (header focus, `TicketQuery` sites, `LIFECYCLE_*`, descriptions unreadable), 15 Important, 14 Minor | Decisions, named edits, rulings | Revision 2: decision 18, E-1, E-2, fact 14, E-8, and the rest | Round 1 reports |
| Review round 1, contract and consistency (`563763e2`) | 4 Blocking (duplicate codes across lifecycles, unreadable files, nonconforming-only lists, the ULID path), 22 Important, Minor | Decisions, rulings | Revision 2: decisions 2, 8, 10, 19, 20; Parts A–I | Round 1 reports |
| Owner, 2026-11-11 | Decisions 1–20 confirmed; review round 2 requested | Owner decision | Recorded in the Cycle document | Session |
| Review round 2, scoped re-review (`85210ab0`) | Round 1 resolved except S-B1, S-B2, S-I6 partly; 4 new Important: toolbar focus fails `ringed`; Retry clears the text; the details sheet's suggestions change; R-26 makes the made-ready set-up unreachable; Minor: fact citations, file lists, E-list gaps, `ListRow` interface, `clean_on_escape`, menu keys, P26 for sort and readiness | Named edits, rulings, settled | E-14, R-35, R-33 (open tickets only, so its test holds), E-10; facts 5, 11, 12, 13, 21; plan file lists; E-1, E-2, E-3, E-5, E-7, E-8; Part A `ListRow`; Part D | Reviewer report, 2026-11-11 |
| Review round 2, adversarial (`85210ab0`) | 3 Blocking: E-7 rests on `ringed` being unchanged; no list element in the non-row states; outcomes without proof (P1, P14, P15, P16, Part I's busy read); 9 Important: Retry, Show all under a readiness, the hint against the empty state, an unreadable file's open, the status read's stamps, buttons inside the drag source, the component `Button`'s keyed focus, "Updating…" in tabbed groups, Enter on stale rows; Minor | Rulings, named edits, settled; one narrowing of decision 10 | E-14; R-34; new test steps and `markers_appear_without_a_read`; R-35; Show all sets Any readiness; the empty state with matches elsewhere; R-38; Part I's read and kept files; R-37; R-36; R-32; Minor in Parts A–I and K-10, K-13 | Reviewer report, 2026-11-11 |
| Review round 3, scoped re-review (`ba5da196`) | Round 2's Blocking and Important all resolved; the new mechanisms and the invalid-UTF-8 fixture hold against the source. 1 Important: Task 3's tests need the filter input that Task 4 placed, and R-26 reaches the made-ready set-up at Task 2. 12 Minor: a test path, `ReadKind::FindSlug` left unused, R-8's wording, the status read in Task 3's journey, failed status reads, the filter status's Retry, the tabbed "Updating…" step and the count's label, the Kit's empty-list icon, signatures, E-10's task, the drag id, the acceptance order | Plan task moves; settled | The input and its one-source wiring move to Task 3; E-10's made-ready edit to Task 2; Part C header count and empty list; Part D without a Retry control; Part G drag id; Testing seams; E-1's test names; plan fixes | Reviewer report, 2026-11-11 |
