# 02 · Layout and docking

Inspired by Zed: edge docks around a central editor area, panels that can share a dock as tabs, and footer buttons that show or hide panels.

## Regions

- **Left dock**, **right dock**, **bottom dock** (GPUI Kit `DockPlacement::Left | Right | Bottom`).
- **Center** = the item area (`DockLayout` h-split of tab groups). It always fills the remaining space, `minmax(0, 1fr)`.
- The bottom dock spans only the center column (its left and right edges align with the center). Side docks run full height between the title bar and the footer.
- Within a side dock, **groups are stacked vertically** with a 1 px border between them. Within the bottom dock, groups sit side by side.
- **A group with more than one panel becomes a tabbed group.** The tabs sit in the group header (see "Panel header" below).
- Group sizes are proportional (flex weights). Docks and groups are resizable (GPUI Kit `Resizable`). The mockups don't show drag handles; use the toolkit default.

## Explored layouts

| | Left dock | Right dock | Bottom dock | Rows |
| --- | --- | --- | --- | --- |
| **1a Classic** | 264 px: Repositories (1.1) / Projects (0.9) / Teams (0.7), stacked | 400 px: Tickets (1.3) / Documents (1.0), stacked | 170 px: Activity | 28 px |
| **1b Docked as tabs** | 290 px: **tabs [Repos · Projects · Teams]** (0.75) / Documents (1.4) | 440 px: Tickets | 150 px: Activity | 24 px |
| **1c Single sidebar** | 380 px: tabs [Repos · Projects · Teams] (0.6) / tabs [Tickets · Documents] (1.6) | none | Activity, hidden by default | 24 px |

**Recommended default: 1b.** Comfortable rows from 1a (28 px) are a good candidate setting ("Row density: Comfortable / Compact", not designed yet). The owner said "I like this direction" about the whole set, so confirm the default layout with the owner. All three must be reachable just by the user rearranging panels; they are not separate modes.

When a tabbed group has more than 2 tabs, use short labels: `Repos`, `Projects`, `Teams`, `Tickets`, `Docs`, `Activity`.

## Panel header (every panel group)

- Height 32 px, background `chrome`, bottom border 1 px `border`.
- **Single panel:** icon (13 px) + title (12 px / 500) + count (11 px, `muted`), no tab treatment.
- **Tabbed group:** each tab is a button with padding 0 10 px. The active tab has `panel` background, `text` color and a 2 px inset bottom line in `accent` (amber). Inactive tabs use `secondary` text on a transparent background.
- Right side: toolbar icon buttons, 26×24 px, radius 4, icon 14 px `secondary`. Hover: `hover` background and `text` color. Per-panel actions are listed in `03-panels.md`.

## Footer (global status bar), 28 px

Left to right:

1. **Left dock toggle** (`panel-left`) · one toggle per panel in the left dock · separator · **bottom dock toggle** (`panel-bottom`) · one toggle per bottom panel.
2. Spacer.
3. **Update check status**: `refresh-cw` icon (teal) + "Checked for updates 2 min ago". Tooltip: "Background check: fetch and fast-forward only. Never shares your changes."
4. **Unshared work**: `cloud-upload` + "2 to share" (amber). Click sets scope to the repository that has the unshared work.
5. **Needs attention**: `circle-alert` + "3 need attention" (red). Click toggles the Needs attention popover (`05-sheets-and-dialogs.md`). Hide it when the count is 0.
6. Separator · one toggle per panel in the right dock · **right dock toggle** (`panel-right`) · separator · **Settings** (`settings`, Ctrl ,).

Toggle buttons are 26×22 px, radius 4, icon 14 px.
- **On:** `hover` background, `text` icon color.
- **Off:** transparent, `muted`.
- **Dock toggles:** `secondary` when the dock is shown, `faint` (`#4a5059`) when hidden.
- Tooltip text: "Hide Tickets" or "Show Tickets".

Behavior:
- Toggling a panel **on** also shows its dock if the dock was hidden.
- When every panel in a dock is hidden, the dock collapses and the center widens.
- Toggling the dock button hides or shows the dock without changing which panels are enabled inside it.
- Panels moved by the user to another dock carry their footer button to that side.

## Center: item area

- An h-split of up to **3 panes**. Each pane is a tab group (`04-item-area.md`). Its "Split right" button (`columns-2`) clones the active tab into a new pane to the right.
- Closing the last tab of a non-only pane removes the pane.
- One pane is **focused**. Its active tab shows a 2 px **amber** top line. Other panes' active tabs show a 2 px `faint` line. Lists open items into the focused pane.

## Persistence

- Persist the `DockAreaState` (or equivalent): dock sizes, groups, panel order, active tab per group, visible panels, item panes and open tabs (by item key), the focused pane, and the active scope.
- Save on every `LayoutChanged` event. Restore on launch. Fall back to layout 1b if restore fails or the schema version changed.
- Register every panel by a stable name: `RepositoriesPanel`, `ProjectsPanel`, `TeamsPanel`, `TicketsPanel`, `DocumentsPanel`, `ActivityPanel`. Register item tabs as `TicketItem` and `DocumentItem` with their key.
- Open tabs whose items no longer exist restore as a recoverable "not found" tab, never as a silent drop (RFC: removed repositories leave drafts recoverable).

## Keyboard (in addition to existing bindings)

Bindings already in `src/desktop/keys.rs` (region focus, tabs, find, settings) remain. Add:
- Toggle each panel (for example Ctrl Shift R / P / T / K / D / A for Repos, Projects, Teams, tickets, docs, activity; confirm against existing bindings).
- Toggle the left, right and bottom docks (Ctrl B, Ctrl Alt B, Ctrl J, as in Zed).
- Split right, and move focus between panes.
- Each panel's toolbar actions must be focusable and reachable by Tab.
