# 01 · Information architecture

## Window anatomy (top to bottom)

1. **Title bar** (38 px). App icon, menu (File, Edit, View, Help), **scope switcher**, centered **search** (Ctrl K), Git identity avatar, Linux client-side window controls (minimize, maximize, close).
2. **Dock area**. Left, right and bottom docks around the **center item area**.
3. **Footer / status bar** (28 px). Panel visibility toggles on the left and right edges. Sync status in the middle-right.

## Panels

| Panel | Role | Content source |
| --- | --- | --- |
| Repositories | Lists enabled repositories, each with a sharing state. Clicking one sets scope = that repository. | Registry + remote observation |
| Projects | Lists distinct `project` values across all repos. Clicking one sets scope = that project. | Index (union across repos) |
| Teams | Lists distinct `team` values. Clicking one sets scope = that team. | Index (union) |
| Tickets | Flat, dense, sortable list of tickets **in scope**. | Index, filtered by scope |
| Documents | Tree of managed documents **in scope**. Grouped by repository when the scope spans more than one. | Index, filtered by scope |
| Activity | Persistent event log: polls, saves, shares, closes, failures with actions. | Operation journal / runtime events |
| Item area | Open tickets and documents as tabs; up to 3 side-by-side panes. | Item reads + drafts |

## The scope model (one active scope)

- Exactly **one** scope is active at a time: `Repository(id)`, `Project(name)` or `Team(name)`. There is no intersection of filters.
- Selecting a row in Repositories, Projects or Teams **replaces** the scope. The selected row shows the selection treatment (`03-panels.md`). Rows in the other two panels lose selection.
- The scope is shown in the title-bar switcher as `[icon] Kind name ▾`, for example `Project manyhands`. The switcher is a shortcut (a popover listing all three groups) for when the scope panels are hidden.
- The Tickets footnote restates the scope: `8 open tickets in project manyhands · from 2 repositories`.
- Default scope at first launch: the first enabled repository. On later launches: the last scope (persist it with the dock layout).
- Project and team values are free text from metadata. Normalize case for grouping but display the most common spelling. Items without a project or team only appear under their repository scope.

## Item identity (unchanged from the RFC)

- Tabs are keyed by `(repository id, kind, canonical ULID)`. Opening an item that is already open focuses its existing tab, in whichever pane it lives.
- The **short code** (`MH-VH-7Q3`) is shown wherever a ticket is named: list rows, tabs, dialogs. The ULID is reachable from tooltips and detail views.
- Opening a list row opens it in the **focused pane**.

## What drives what

```
Repositories ─┐
Projects ─────┼─► scope ─► Tickets list ─┐
Teams ────────┘           Documents tree ─┼─► open tab in focused pane
                                          │
Activity ◄── runtime events ◄── item actions (save, share, close, approve)
Needs attention (footer) ◄── unresolved recovery states (subset of Activity)
```

## Out of scope this round

- Ticket and document editor bodies (zorite-editor), comments and discussion.
- Command palette contents (only the search field is shown).
- First run with no repositories (empty states are specified in `03-panels.md`, but the first-run flow is not designed).
- Boards, rollups, assignment.
