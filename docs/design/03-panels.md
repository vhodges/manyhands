# 03 · Panels

Common row anatomy: height 28 px (comfortable) or 24 px (compact), padding 0 10 px 0 12 px, gap 8, font 13 px.
- **Hover:** `hover` background.
- **Selected:** background `selection` (`#29363a` dark / `#fbefd6` light), a 2 px inset **left** line in `accent`, and the label weight goes to 600.
- **Focus (keyboard):** the same as selected, plus the GPUI Kit focus ring. Focus must always be visible.

All lists are virtualized (`VirtualList` or `List`) and keyboard navigable (↑ ↓, Enter opens, Home/End).

Every panel distinguishes **four states**, as the RFC requires:
- **Loading:** the scan is running. Show skeleton rows.
- **Empty:** "No tickets in project manyhands" plus the primary action.
- **Failed or stale scan:** a row-level alert with Retry.
- **Normal.**

---

## Repositories

- **Toolbar:** `plus` "Add or create a repository" (opens the Add repository sheet) · `refresh-cw` "Check all for updates now" · `ellipsis` More (rebuild index, remove…).
- **Row:** `folder-git-2` (14 px, `secondary`) · name (ellipsis) · right-aligned state (11.5 px, icon 12 px + label).

| State | Label | Color | Icon |
| --- | --- | --- | --- |
| Unpublished checkpoints | `2 to share` | amber | `cloud-upload` |
| In sync | `Up to date` | green | `circle-check` |
| No publication remote | `This computer` | secondary | `hard-drive` |
| Poll/auth/network failure | `Can't reach` | red | `circle-alert` |
| Paused | `Paused` | muted | `pause` |
| Checking now | `Checking…` | teal | spinner |

- **Tooltip:** path · main branch · remote URL · numeric detail ("2 checkpoint commits not pushed"). This is where Git vocabulary lives.
- **Click:** scope = this repository.
- **Context menu:** Check now, Share main, Open folder, Settings…, Remove from Manyhands… (confirm; explain that the files stay).

## Projects and Teams

- **Toolbar:** `list-filter` Filter · `ellipsis` More.
- **Row:** `folder-kanban` (projects) or `users` (teams) · name · meta (11.5 px, `muted`): `8 open · 2 repos` for projects, `8 open` for teams.
- **Tooltip:** `Project: everything tagged "manyhands" across all repositories`.
- **Click:** scope = this project or team.
- Counts only. **No progress bars, no status breakdowns** (rollups are a PRD non-goal).

## Tickets (flat, dense, sortable)

**Toolbar:** `plus` New ticket (Ctrl N) · `ellipsis` More.

**Filter bar** (padding 6 8, bottom border):
- A segmented control: `Open 8 | Closed 4 | All 12`, counts in `muted`. The active segment is `border`-colored with `text`. The default is **Open** (RFC). Closure means lifecycle closure, not the status text.
- A filter input: "Filter by title or code". It matches title and short code. Duplicate short codes show all matches.
- Ready / Blocked filters (RFC ticket relationships) are not designed. Add them as more segments or a menu later.

**Column header** (24 px, 11 px uppercase, letter-spacing .04em, `muted`). Clicking a column sorts by it; clicking again reverses. The active column is `text` with an arrow; inactive columns show a faint `arrow-up-down`.

| Column | Width | Content |
| --- | --- | --- |
| (type) | 14 px | type icon 13 px: cycle `repeat`, defect `bug`, feature `sparkles`, task `list-checks`, missing `triangle-alert` (amber) |
| Code | 78 px | short code, mono 11 px, `secondary`; `—` when none |
| Title | flex | ellipsis. After it, inline markers: `pencil` (amber, "You're editing this. Saved on this computer, not shared yet."), `circle-alert` (red, conflict) |
| Status | 92 px | pill: 11 px, padding 1 7, radius 9, tinted bg + colored text (table below) |
| Upd. | 30 px | right-aligned age `12m`, `5h`, `3d` (`muted`) |

The default sort is **Updated, newest first**. Closed tickets use `secondary` title color.

**Status pill colors.** Status is a free-form project value. These are the known values; unknown values fall back to neutral.

| Status | Text | Background |
| --- | --- | --- |
| in progress | teal | teal 13% |
| ready | green | green 13% |
| blocked | red | red 13% |
| in review | amber | amber 13% |
| not started / waiting / unknown | secondary | secondary 12% |
| closed | muted | muted 12% |
| needs fixing (nonconforming) | amber | amber 15% |

**Nonconforming ticket rows stay visible.** They show a `triangle-alert` type icon, the status pill "needs fixing" and the tooltip "Needs fixing: Missing type. Open it to add one."

**Footnote** (11 px, `muted`, top border): `{n} {open|closed|} tickets in {kind} {name} · from {k} repositories`.

## Documents (tree)

- **Toolbar:** `file-plus` New document · `folder-plus` New folder · `chevrons-down-up` Collapse all.
- **Row:** indent 8 + 14 × depth px · chevron (12 px; `chevron-right` or `chevron-down`, or a blank spacer for leaves) · icon 14 px · label · trailing markers.
  - Folder icon: `folder` or `folder-open` (`secondary`). A collapsed folder shows its descendant count on the right.
  - Document: `file-text`. The label is the document **title**, not the filename. The tooltip shows the path.
  - Editing: a trailing `pencil` (amber).
  - Nonconforming: `file-warning` icon, amber label, trailing `triangle-alert`. The label falls back to the filename. Tooltip: "Not a managed document yet. Open to adopt it."
- **Multi-repo scope** (project or team): top-level nodes are repositories (`folder-git-2` in amber, 600 weight). **Single-repo scope:** the repo's tree directly.
- Click a folder to toggle it. Click a document to open it in the focused pane. ← → collapse and expand.
- Empty folders: the RFC says they aren't portable. Show them with the tooltip "Shared once it contains a document."

## Activity

- **Toolbar:** `list-filter` "Show only things that need attention" · `ellipsis`.
- **Row** (min height 24, 12.5 px): time (mono 11 px, `muted`, 40 px) · icon 13 px (colored by kind) · message (ellipsis) · repository tag (11 px, 1 px `border`, radius 4) · optional action button ("Retry now", "Set up sharing", "Review", "Unlock…").
- **Tooltip:** the Git or technical detail ("fetch origin · fast-forward main 4e1a2c0..e49f978"). **Never include secrets** (NFR-003).
- Newest first. Persistent for the session at least; failures remain until resolved (RFC: failures must not exist only in transient notifications).
- Icon colors: info `secondary`, success green, sharing teal, failure red, locked amber.

Example copy (use this register):
- "Checked manyhands for updates. Brought in 2 changes from Jo Smith."
- "Saved "Wave 03 D3: Rich-text editor integration" on this computer"
- "Couldn't reach zorite's shared copy. Your work is safe; we'll try again in 5 minutes." [Retry now]
- "team-handbook has no shared copy, so changes stay on this computer" [Set up sharing]
- "Closed "…". Merged into main and tidied up. MH-VH-8FN is now ready."
- "Rebuilt the search index (4 repositories, 1,455 files)"
