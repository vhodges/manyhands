# 08 · GPUI Kit mapping and fit with the current code

Verify every API name below against the **locked** `gpui-kit` version (0.6.6 at the time of writing) before planning. These names come from the v0.8.0 docs.

## Component per element

| Element | GPUI Kit |
| --- | --- |
| Window chrome, Linux controls | `TitleBar` (client-side decorations on Wayland) |
| Scope switcher, menus, ⋯ | `Popover` / `Menu` / `DropdownButton` |
| Search field | `Input` + `Kbd`; opens `Command` (palette, future) |
| Docks, groups, tabbed groups, splits | `Dock`: `DockArea`, `DockSkin`, `DockLayout::{h_split, v_split, tabs}`, `DockPlacement`, `DockAreaState` (persist), `register_panel` |
| Each panel | a struct implementing `BasePanel` + `Panel` (title, tab, toolbar), wrapped with `panel_handle` |
| Panel toolbars | `Toolbar` + `Button` (ghost, icon) + `Tooltip` |
| Repos, projects, teams, activity lists | `List` / `VirtualList` |
| Tickets list | `Table` / `DataTable` (sortable columns), or `VirtualList` with a custom header. Needs keyboard row navigation. |
| Documents tree | `Tree` |
| Status pills, tags | `Tag` / `Badge` |
| Segmented Open/Closed/All | `Tabs` (segmented variant) or a `Toggle` group |
| Item tabs + split | `Dock` center tab groups, or `Tabs` with close buttons |
| Banners, outcome boxes | `Alert` |
| Footer | `StatusBar` |
| Sheets | `Sheet` (right placement) via `Root::render_sheet_layer` |
| Dialogs | `Dialog` / `AlertDialog` via `Root::render_dialog_layer` |
| Needs attention | `Popover` anchored to the footer item |
| Settings layout | `Settings` component or `Sidebar` + content |
| Switches, radios, selects | `Switch`, `Radio`, `Select` |
| Spinner, loading rows | `Spinner`, `Skeleton` |
| Empty states | `Empty` |
| Icons | `Icon` (Lucide names) |
| Theme | `Theme` (custom dark and light, see `07`) |

## Mapping onto today's `src/desktop/view`

The current `Workspace::render` (`view/workspace/render.rs`) builds a fixed h-flex: Repositories (260 px) | Navigation (320 px) | Content (flex), plus a 260 px Operations area at the bottom. It already renders the `Root` sheet, dialog and notification layers, and it has region focus actions. The new design reorganizes these, rather than adding to them:

| Today | Becomes |
| --- | --- |
| `Region::Repositories` (`sidebar.rs`) | `RepositoriesPanel` |
| `Region::Navigation` (`documents.rs`, `tickets.rs`, `lists.rs`) | `TicketsPanel` + `DocumentsPanel`, now scope-driven |
| (none) | `ProjectsPanel`, `TeamsPanel` (new index queries: distinct project and team values with counts) |
| `Region::Content` (`item/*`, `tabs.rs`) | Center item area: tab groups + split |
| `Region::Operations` (`operations.rs`) | `ActivityPanel` (bottom dock) + footer status + Needs attention popover |
| `settings/*` (repositories, remotes, keys, hosts) | Settings sheet sections (copy changes; behavior mostly unchanged) |
| `confirm.rs` (promotion and closure confirm) | Close ticket / Approve and merge dialog stages |
| `passphrase.rs` | Unlock key dialog |
| `draft_comparison.rs`, conflict views | Resolve conflict sheet (canonical paths) |
| `keys.rs` region focus actions | Keep. `FocusRepositories`, `FocusNavigation` and friends map to panel focus. Add panel and dock toggles. |

Keep the existing headless services and models untouched where possible. This is a view-layer reorganization plus:
- a **scope** model (new),
- project and team index queries (new; read-only, in the library),
- layout persistence (new; app-local data, not in repositories).
