---
title: "Wave 04 Cycle 01 Workspace Shell Design"
date: 2026-11-08
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6YVNFD5D8C8P15EC464R5PM"
---

# Wave 04 Cycle 01 Workspace Shell Design

## Intent And Approval Boundary

This design covers the workspace shell of
[Cycle 01](../Cycles/wave-04-cycle-01-workspace-shell.md): chrome tokens,
the title bar and window title, the dock area with four panels around the
item area, the footer and its dock menus, keyboard reach, layout
persistence, and the quit path with Activity hidden. Approval covers the
mechanisms, rulings, named test edits and task order. It does not
authorize implementation, pushes beyond the ticket branch, merging or
closing.

Draft, 2026-11-08, not yet independently reviewed. Grounded at the ticket
branch `2b1dc4d5`, which holds origin/main `8da9afd4`.

Conventions:

- `D/` is `src/desktop/`.
- `B/`, `C/` and `G/` are `gpui-base-0.6.6/src/`, `gpui-component-0.6.6/src/`
  and `gpui-pre-0.3.6/src/` under
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (the locked
  crates).
- "Fact n" refers to Baseline Facts. "P-n", "R-n", "E-n", "K-n" and "U-n"
  are this design's Required Outcomes, Rulings, Named Edits, Known Limits
  and unverified facts. "Decision n" is a Cycle decision; "Wave decision n"
  is Wave 04's.
- "The handoff" is `docs/design/`; `02` is `02-layout-and-docking.md`.

## Baseline Facts At `2b1dc4d5`

**Kit dock (0.6.6).**

1. **Names.** The dock is a rewrite of the API the handoff names.
   `DockItem`, `StackPanel`, `TabPanel` and `set_left_dock` do not exist;
   "StackPanel" and "TabPanel" survive only as names in a saved file
   (`B/dock/state_convert.rs:8-9`). The types are reached as
   `gpui_kit::component::dock::{DockArea, DockAreaRenderer, DockAreaState,
   DockEvent, DockLayout, DockPlacement, DockState, PanelState, PanelInfo,
   PanelEvent, PanelId, InsertTarget, TabGroupRenderer, TabGroupContext,
   register_panel, PanelBuildContext, …}` (`C/dock/mod.rs:48-55`), with
   `BasePanel` (`:32`), `Panel`, `PanelHandle` and `panel_handle` (`:56`)
   and `DockSkin` (`:122`). gpui-kit re-exports whole crates only
   (`gpui-kit-0.6.6/src/lib.rs:106,143`).
2. **DockArea.** `DockArea::new(id, version: Option<usize>, window, cx)`
   (`B/dock/dock_area.rs:126`), `with_renderer(Rc<dyn DockAreaRenderer>)`
   (`:154`), `set_center(DockLayout, …)` (`:232`), `set_dock(placement,
   DockLayout, …)` with no size or open argument (`:246`; a new dock is
   200 px and open, `:262`), `remove_dock` (`:271`), `has_dock` (`:286`),
   `is_dock_open` and `toggle_dock` (`:293`, `:301`), `set_dock_size`,
   floored at 100 px (`:348`), `add_panel_view(panel, placement,
   Option<Pixels>, …)`, which merges into the region's first tab group or
   creates the dock (`:389-441`), `remove_panel` (`:469`),
   `move_panel(PanelId, InsertTarget, …)` (`:480`), and
   `layout(placement) -> Option<&PaneTree>` (`:189`). `InsertTarget` is
   `Tabs { node, ix, activate }` or `Split { node, placement, size }`
   (`B/dock/layout/edit.rs:12-25`).
3. **DockLayout.** `DockLayout::h_split()`, `v_split()`, `tabs()`
   (`B/dock/layout/builder.rs:41-51`), `.child(DockLayout, Option<Pixels>)`
   (`:71`), `.panel_view(Arc<dyn PanelView>, &App)` (`:101`). A panel added
   with `.panel(entity)` shows its `panel_name` as its title; titles need
   `panel_view(panel_handle(entity), cx)` (`C/dock/panel.rs:313-327`). Sizes
   are pixels, with no flex type. Inference: when every slot of a split is
   sized, the sizes are rescaled to the container in proportion
   (`B/dock/dock_area.rs:1470-1481`), so the left dock's 0.6 : 1.4 is given
   as `Some(px(600.))` and `Some(px(1400.))`. Each slot is at least 100 px
   (`B/resizable/panel.rs:259`).
4. **dump.** `dump(&self, cx) -> DockAreaState` (`B/dock/dock_area.rs:794`)
   writes `version`, `center: PanelState` and
   `left_dock`/`right_dock`/`bottom_dock: Option<DockState>`
   (`B/dock/state.rs:9-23`). `DockState` has private fields `panel`,
   `placement`, `size`, `open`, with a public `new` and getters
   (`:27-58`). `PanelState { panel_name, children, info }` is public
   (`:67-71`). Panels hidden by `visible()` are written. A split's sizes
   are its measured sizes, else its stored sizes, else 100 px; a zero is
   never written (`B/dock/dock_area.rs:827-848`). A dock that is not in
   the area is not written.
5. **load.** `load(DockAreaState, …) -> Result<()>` (`:716-760`) sets the
   area's version from the state without checking it, rebuilds every tree
   through the registry, gives each dock its saved size and open flag,
   emits `LayoutChanged` and always returns `Ok`. An unregistered name
   becomes the renderer's placeholder, which DockSkin draws as "The `X`
   panel type is not registered" (`C/dock/dock.rs:83-93`,
   `C/dock/invalid_panel.rs:66-69`). A panel missing from the state is
   simply absent.
6. **Events.** `DockEvent::{LayoutChanged, DragDrop { item, target }}`
   (`B/dock/dock_area.rs:39-45`). `LayoutChanged` follows `set_center`,
   `set_dock`, `remove_dock`, `toggle_dock`, `set_dock_size`, `load`,
   every committed edit (add, remove, move, drop, close, `select_panel`,
   tab switches, `:1112-1121`), the end of a split-divider drag
   (`:1072-1073`) and, with DockSkin's renderer only, mouse-up after a
   dock-edge resize (`C/dock/dock.rs:219-231`). A window resize emits
   nothing. `DragDrop` is for host-owned drags only; a panel drag is a
   committed edit.
7. **Panel traits.** `BasePanel` (`B/dock/panel.rs:21-93`): `panel_name`
   is required; `visible`, `closable` and `zoomable` default to true;
   `dump` defaults to `PanelState::new(panel_name)`; supertraits
   `EventEmitter<PanelEvent> + Render + Focusable`. Nothing in the Kit
   subscribes to `PanelEvent`. The component `Panel` (`C/dock/panel.rs:71-149`)
   adds `title`, `title_suffix`, `toolbar_buttons`, `dropdown_menu`,
   `zoom_control` (default `Some(PanelControl::Menu)`) and `title_bar`
   (default true). `PanelHandle::of` recovers them from a view (`:233`).
8. **Hiding.** A slot whose panels are all `visible() == false` gives its
   space to its siblings (`B/dock/dock_area.rs:1238-1254`, `:1315-1323`).
   A dock whose panels are all hidden keeps its size: `render_dock` sizes
   it from open and size only (`:1325-1354`). A closed side dock takes
   0 px; a closed bottom dock keeps a 29 px strip, `CLOSED_BOTTOM_STRIP`
   (`:1702-1715`).
9. **Header and renderer.** DockSkin's group renderer, `TabGroupSkin`, is
   `pub(crate)` (`C/dock/tab_panel.rs:185`). It always draws a menu
   button whose menu holds Zoom (disabled at most) and, when closable,
   Close (`:269-351`); it draws the Kit's own dock toggle buttons
   (`:205-265`); and it caches the active panel's view (`:733`). The
   replacement point is `DockAreaRenderer::tab_group_renderer`, the trait's
   only required method (`B/dock/dock_area.rs:1747-1840`), returning a
   `TabGroupRenderer` (`B/dock/tab_group.rs:894-953`: `frame`,
   `content_frame`, `render_tab_bar` (required), `render_active_panel`
   (default uncached, `:926-934`), `render_drop_indicator`,
   `render_empty`). `TabGroupContext` offers `panels`, `active_panel`,
   `is_collapsed`, `is_draggable`, `is_droppable`, `select_tab`,
   `drag_panel`, `drop_panel` (`:788-885`). DockSkin implements
   `DockAreaRenderer`, and its `render_dock` adds the dock-edge resize
   handle and tracker (`C/dock/dock.rs:35-97`).
10. **Dragging and dropping.** A tab or single-panel title is a drag
    source only while `!locked && !is_last_panel`; a group alone in its
    tree with at most one visible panel is the last panel
    (`B/dock/tab_group.rs:444-461`). Base attaches content-area drop
    targets to every group while `!locked` (`:729-741`). There is no drop
    predicate; `set_locked` locks the whole area (`B/dock/dock_area.rs:217`,
    `:891-894`).
11. **Registry.** `register_panel(cx, name, Fn(PanelBuildContext, &mut
    Window, &mut App) -> Arc<dyn PanelView>)` (`B/dock/registry.rs:111-119`)
    keeps a global map by name; registering a name again replaces it.

**GPUI and Kit chrome.**

12. **Views.** GPUI renders a child view after its parent's `render` has
    returned (`G/view.rs:324-345`, `:422-445`), so a view's render can
    update an ancestor's entity. A cached view is reused unless its own
    entity is dirty (`G/view.rs:484-499`), and dirtiness is marked on
    ancestors, not descendants (`G/window.rs:2148-2160`).
13. **TitleBar.** `gpui_kit::component::TitleBar` (`C/title_bar.rs`):
    `TITLE_BAR_HEIGHT` 34 px (`:15`), `new()` (`:50`), `window_options()`
    sets `titlebar` and `app_owns_titlebar_drag` only (`:81`),
    `on_close_window` (`:95`), `Styled` and `ParentElement` (`:296`,
    `:302`). It moves the window on drag (`:353-370`) and maximizes on a
    double click on Linux (`:344-346`). It draws window controls on Linux
    only with `Decorations::Client` (`:267-270`). GPUI asks for server
    decorations unless `window_decorations` is set
    (`G/window.rs:1596-1597`). Root draws `window_border`, which gives a
    client-decorated window its resize edges and shadow
    (`C/window_border.rs:25`, `:89-148`, `:189-200`). The test window
    reports server decorations (`G/platform.rs:1020-1029`).
14. **StatusBar.** `gpui_kit::component::status_bar::StatusBar`
    (`C/status_bar.rs:41-95`): `new`, `left`, `right`, and a centre slot
    through `ParentElement`; `Styled`; no fixed height and no role.
15. **Menus.** `PopupMenu::build(window, cx, f)`
    (`C/menu/popup_menu.rs:358`), `action_context(FocusHandle)` (`:371`),
    `menu_with_check(label, checked, Box<dyn Action>)` (`:565`),
    `separator()` (`:680`); `set_previous_focus` is `pub(crate)` (`:394`).
    `ContextMenuExt::context_menu` opens on a right mouse-down only and
    has no public open (`C/menu/context_menu.rs:296-300`, `:117-127`); it
    draws its menu deferred and anchored and closes on `DismissEvent`
    (`:185-215`, `:325-350`). `NativeMenu::show(position, …)` shows the
    operating system's menu on macOS and Windows and the Kit's popup
    elsewhere (`C/native_menu/mod.rs:201-222`).
16. **Buttons and tooltips.** `Selectable::selected` styles a button;
    `toggled` sets only its accessible toggled state
    (`C/button/button.rs:483`, `:521`, `:766-772`). Buttons are Tab stops
    by default (`:458-466`) and are not focused by a mouse-down
    (`:790-791`). `tooltip(text)` (`:389`) and `tooltip_with_action(text,
    action, context)` (`:404`) show on hover only. Root binds Tab and
    Shift Tab to the next and previous focus (`C/root.rs:24-26`).
17. **Accessibility.** GPUI 0.3.6 builds an AccessKit tree
    (`G/gpui.rs:89-91`); stateful elements take `role`, `aria_label` and
    `aria_toggled` (`G/elements/div.rs:1304-1400`). Inference: the test
    platform's `a11y_init` is a no-op (`G/platform.rs:1070`), so tests see
    no tree.
18. **Window title.** `Window::set_window_title(&str)`
    (`G/window.rs:2884-2887`). The test window records it
    (`G/platform/test/window.rs:24`, `:427-429`) and
    `VisualTestContext::window_title() -> Option<String>` reads it
    (`G/app/test_context.rs:861-864`). This settles the Cycle's U-1.
19. **Keys.** `Keystroke::parse` accepts any key name
    (`G/platform/keystroke.rs:120`, `:184-188`); `secondary` is Ctrl on
    Linux and Windows and Cmd on macOS (`:143-149`). On Linux a key is
    named by its xkb keysym name in lower case
    (`gpui-pre-linux-0.3.6/src/linux/platform.rs:1083`, `:1158`).
    Inference: the Menu keysym (0xff67) arrives as `menu` (Cycle U-3).
    zorite binds `cmd-b` and `ctrl-b` to Bold in "Editor"
    (`zorite-editor-0.10.0/src/lib.rs:59`, `:137-138`). Nothing in the Kit
    or zorite binds `secondary-alt-b`, `secondary-j`, `menu` or
    `shift-f10`.

**The app.**

20. **Window.** `window_options` sets only the bounds, 1280 × 840
    (`D/mod.rs:76-85`). Nothing in `src/` sets a window title. Root is
    created over the Workspace in `root_over_workspace` (`D/mod.rs:89-92`);
    tests use `open_test_window_over` (`D/mod.rs:292-304`).
21. **Workspace render.** `D/view/workspace/render.rs:47-236` builds a
    fixed row of Repositories (260 px), Navigation (320 px) and Content
    (flex), over the operation area (`id("operation-area")`, 260 px)
    (`:103-141`). Root's sheet layer is drawn inside the content region
    (`:83-93`); the dialog and notification layers are children of the
    workspace root (`:234-235`). `unless_dialog` (`:34-45`) drops a
    workspace action while a dialog or sheet is active
    (`has_active_sheet` is `Root.active_sheet.is_some()`,
    `C/window_ext.rs:129`). This is R-60's guard; it does not depend on
    where the sheet is drawn. Quit and QuitNow are not guarded (`:146-149`).
22. **Regions.** `Region { Repositories, Navigation, Content, Operations }`
    and `REGIONS: [Region; 4]` (`D/view/workspace/focus.rs:11-24`); key
    contexts "Repositories", "Navigation", "Content", "Operations"
    (`:52-59`); ids `region-*` and `region-*-focus-ring` (`:39`, `:48`);
    the decorator `Workspace::region` (`:153-183`); `cycle_region`
    (`:137-151`); `focus_region` (`:82-109`); `show_pane` (`:63-74`). The
    Workspace holds `regions: [FocusHandle; 4]` (`D/view/workspace.rs:84`)
    and focuses Repositories at start (`:165`).
23. **Key bindings.** `D/keys.rs:203-295`. `f6`/`shift-f6` → `NextRegion`/
    `PreviousRegion`; `secondary-1`…`4` → `FocusRepositories`,
    `FocusNavigation`, `FocusContent`, `FocusOperations`; `secondary-p` →
    `Find`; `secondary-,` → `OpenSettings`; `secondary-q` → `Quit`; all in
    no context. `secondary-n` → `NewItem` only in "Navigation". `QuitNow`
    has no key. `keys_tests.rs:12-56` checks a 17-row table.
24. **Navigation pane.** `D/view/lists.rs:39-180`: Find, then the choice
    `PANES` (`pane-documents`, `pane-tickets`, `:25-28`), then
    `pane-showing` ("Showing documents" / "Showing tickets", `:23-24`),
    then the chosen list. `Pane { Documents (default), Tickets }`
    (`D/model/navigation.rs:145-150`), `pane()`/`set_pane()` (`:211-217`).
    `secondary-2` focuses the chosen list (`focus.rs:98-103`).
25. **Find.** The bar is `D/view/find.rs:44-82`; `focus_find` is
    `D/view/workspace/find.rs:21-25`.
26. **Operation area.** It has no type of its own: `session_section`
    (`D/view/operations.rs:641`), `pending_section` (`:699`) and
    `finishing` (`D/view/quit.rs:87`) are Workspace methods composed in
    `render.rs:118-141`.
27. **Quit.** `QuitPhase { Running, SavingDrafts, Finishing(id), Quit }`
    (`D/view/quit.rs:55-64`). `wait_to_quit` closes dialogs and the sheet,
    focuses Quit now and sets the last region to Operations
    (`D/view/workspace/lifecycle.rs:170-181`). A `SavingDrafts` quit goes
    back to `Running` when a draft cannot be saved (`:202-215`).
28. **Item tokens.** `D/view/item/tokens.rs` defines `accent` and `faint`
    by mode (`:47-51`: dark `#e9ad36`, `#4a5059`; light `#a86f00`,
    `#b5b0a8`). `tokens_tests.rs:7-33` pins the header and page tokens, not
    these two; `tabs_tests.rs:1587-1588` pins where they are used.
29. **Repository name.** `RepositoryEntry::name()`: the root's directory
    name, or the root (`D/model/navigation.rs:74-80`). The sidebar's
    Settings entry is `sidebar-settings` (`D/view/sidebar.rs:144-157`).
30. **App data.** The service's data directory
    (`src/repository/read/credentials.rs:272-279`). `library_services`
    opens the draft and resolution stores in it (`D/mod.rs:98-106`);
    `ResolutionStore::open_in` uses `Folder::open_named(data, name)`
    (`D/services/resolutions.rs:218-224`), whose `replace` and `read`
    (`src/drafts/mod.rs:87`, `:125`, `:144`) give an owner-only folder.
    Tests use the fixture's temporary data directory
    (`D/test_support/fixtures.rs:76-90`).
31. **Attention sources.** `Operations::requests()`
    (`D/model/operations.rs:188`) holds `DesktopRequest`s whose
    `RequestState::Finished(Outcome, …)` (`D/services/requests.rs:793`)
    carries `Outcome { Success, Noop, Partial, Blocked, Cancelled, Error }`
    (`src/results.rs:48-55`); `Dismissed` is its own state.
    `Operations::pending()` (`D/model/operations.rs:584`) maps each
    repository to its "Pending in repositories" rows.
    `DraftsRegistry::flush_failures()` (`D/services/drafts.rs:438-449`) and
    `unavailable()` (`:222`). `model/operations.rs` has no attention
    entries.
32. **Time in tests.** Desktop tests move time with
    `cx.executor().advance_clock` (for example `D/view/tabs_tests.rs:575`);
    the drafts registry waits with `background_executor().timer`
    (`D/services/drafts/flush.rs:168-186`).
33. **Scrolling a focused control into view.** `D/view/scroll.rs:1-22`:
    GPUI does not scroll a focused element into view; the content
    region's `ContentScroll` markers do.
34. **Harness and CI.** `Shell` (`D/view/harness.rs:62`) opens a
    1440 × 1400 window (`:119`). Lookup by `.test_support()` ids panics on
    an ambiguous id (`gpui-base-0.6.6/src/test_support.rs:159-167`). CI
    runs desktop tests as `--features desktop-test-support --bin manyhands`
    (`.github/workflows/build.yml:129`); no new target is needed.

## Part A: Chrome Tokens

`D/view/shell/tokens.rs` holds `ChromeColors` with `07`'s 14 chrome
tokens (`chrome`, `panel`, `border`, `border_strong`, `hover`, `faint`,
`muted`, `secondary`, `text`, `selection`, `accent`, `teal`, `green`,
`red`) for both modes, chosen by `cx.theme().is_dark()` as
`item_colors` does. The title bar, panel headers and footer use it.
`ItemColors.accent` and `.faint` are read from it, so the item view's
values stay as fact 28 lists them; the rest of `ItemColors` is unchanged.

## Part B: The Layout Model

`D/model/layout.rs` is pure: no entities, no window.

- `PanelKind { Repositories, Tickets, Documents, Activity }`, each with a
  stable name (`RepositoriesPanel`, `TicketsPanel`, `DocumentsPanel`,
  `ActivityPanel`), a label and an icon. The item area's stable name is
  `ItemAreaPanel`.
- `Side { Left, Right, Bottom }`.
- `Layout` holds, per panel, whether it is **on** (the person wants it);
  per side, whether the dock is **shown**; the **arrangement** (each
  side's panels in order, read from the dock area); the **kept** state of
  each dock the shell has taken out of the area (R-3); and each side's
  last size.
- Derived: a panel is **on screen** when it is on and its dock is shown;
  a dock is **open** when it is shown and holds a panel that is on.

| Operation | Effect |
| --- | --- |
| Panel toggle on a panel on screen | It goes off. |
| Panel toggle on a panel not on screen | It goes on and its dock is shown (`02`). |
| Dock toggle on an open dock | Not shown. Which panels are on does not change (`02`). |
| Dock toggle on a dock that is not open | Shown; if none of its panels is on, all of them go on (decision 6). |
| Dock toggle on a side with no panels | None: that side has no toggle (decision 6). |
| Reveal (a region key, Find, Needs attention) | The panel goes on and its dock is shown. |
| Moved to another side | It is on, and the new side is shown (R-5). |

`Layout::default()` is decision-1's 1b: left Repositories over Documents,
right Tickets, bottom Activity, every panel on and every dock shown, with
sizes 290, 440 and 150 px. Both the rules and the default are unit tested
(Testing).

## Part C: Dock Area And Panels

**Panels (R-1).** `D/view/dock/panels.rs` defines `RepositoriesPanel`,
`TicketsPanel`, `DocumentsPanel`, `ActivityPanel` and `ItemAreaPanel`.
Each is a thin entity holding a `WeakEntity<Workspace>` and its kind:

- its `render` asks the Workspace for that region's body, built as today
  by the same Workspace methods and decorated by `Workspace::region`, so
  the region ids, focus rings and key contexts stay (fact 22);
- its `focus_handle` is the Workspace's handle for that region;
- `visible` reads whether the layout says the panel is on;
- `closable` and `zoomable` are false, `zoom_control` is `None`;
  `title`, `title_suffix` (the count, R-22) and the icon serve the header;
  `ItemAreaPanel`'s `title_bar` is false;
- it observes the Workspace and notifies itself, so a change in the
  Workspace redraws it whatever caching the dock uses (fact 12).

Each is registered by its stable name with `register_panel`, and the
builder returns the Workspace's existing entity, so a `load` never makes a
second copy (fact 11).

**Regions (R-7).** `Region` gains `Tickets` and `Documents` in place of
`Navigation`, so `REGIONS` has five entries in F6's default order (R-9).
Both lists keep the key context "Navigation", so `secondary-n` still works
in each (fact 23). Their ids become `region-tickets` and `region-documents`.
`Operations` keeps its name, its ids and the `operation-area` id, which
privacy tests sweep (E-1).

**Navigation split.** `navigation_pane` (fact 24) becomes two bodies.
The Tickets body is Find, then New ticket…, the filters, the list and Why
blocked. The Documents body is New document…, New folder… and the tree.
The choice and the `pane-showing` line go (Cycle Scope 4). `Pane` stays
as "the list used last": it is set when either panel takes focus, and
`secondary-2` focuses it (R-8).

**The area.** The Workspace holds an `Entity<DockArea>`, built with
`DockArea::new("workspace-dock", Some(LAYOUT_VERSION), …)` and
`with_renderer(ShellRenderer)`. Its centre is `ItemAreaPanel` alone: the
content region with its tab strip, item view, Settings and Drafts pages,
empty state and Root's sheet layer, drawn where it is today (fact 21).
R-60 still holds through `unless_dialog`, which every new workspace action
uses (R-29).

**Renderer (R-2).** `ShellRenderer` (`D/view/dock/header.rs`) implements
`DockAreaRenderer`. It hands `frame`, `center_frame`, `split_frame`,
`render_dock` and `build_placeholder` to an inner `DockSkin`, which keeps
the Kit's dock-edge resizing and its resize `LayoutChanged` (facts 6 and
9). Its `tab_group_renderer` returns the app's `PanelHeader`:

- a single panel: icon, title and count per `02`, "Panel header";
- a tabbed group: one tab per visible panel, the active one with the
  `panel` background and the 2 px amber line (`accent`);
- tabs and titles are drag sources and drop targets through
  `TabGroupContext` (`drag_panel`, `drop_panel`, `select_tab`), as the
  Kit's skin wires them (`C/dock/tab_panel.rs:400-412`, `:523-604`);
- no menu button, zoom, close or dock toggle buttons;
- no tab bar for a group whose active panel has `title_bar` false (the
  item area);
- the active panel drawn uncached (the default, fact 9).

**Hiding (R-3).** A panel that is off reports `visible() == false`, so
its slot gives its space to the rest of its dock (fact 8). A dock that is
not open is taken out of the area: the shell dumps the area, keeps that
dock's `DockState` in the layout, and loads the state without it. When the
dock opens again, the shell loads the state with the kept dock back. A
dock with no panels is not in the area. The Kit's own dock closing is not
used, because a closed bottom dock keeps a 29 px strip and a dock whose
panels are all hidden keeps its width (fact 8).

**Moves (R-5).** Choosing another dock in a panel's dock menu:

- if the target dock is in the area, `move_panel` puts the panel at its
  end as a new group: `InsertTarget::Split` beside the dock's root,
  below it in a side dock and to its right in the bottom dock;
- otherwise the shell takes the panel out of its dock (`remove_panel`)
  and adds it to the target with `add_panel_view`, at the side's last
  size, or its default.

The panel is then on and its side shown. A source dock left with no panel
on screen is taken out, as above. Drags are the Kit's own edits (fact 10);
the shell reads the new arrangement after each.

**After each `LayoutChanged` (R-6).** The shell dumps the area and checks
that its centre holds `ItemAreaPanel` alone and that each of the four
panels appears exactly once.

- **If it does,** the dump becomes the **accepted** state, the layout's
  arrangement is read from it, and a save is scheduled (Part G).
- **If not,** a drop landed on the item area: the shell loads the
  accepted state, and the panel is back where it was. This settles the
  Cycle's U-2, since the Kit has no drop predicate (fact 10).

**Focus (R-9).** When the focused panel or its dock is hidden, focus
moves to the item area. At start, focus goes to Repositories if it is on
screen, otherwise to the first F6 stop.

**F6 order (decision 9).** The left dock's on-screen panels top to
bottom, the item area, the right dock's, then the bottom dock's left to
right. In a tabbed group only the active tab is a stop. `cycle_region`
steps this list, which the layout derives from the arrangement.

## Part D: Title Bar And Window Title

- **Window options (R-15).** `TitleBar::window_options()` with the
  existing bounds and `window_decorations: Some(WindowDecorations::Client)`,
  so Linux draws the Kit's window controls (fact 13). Root's
  `window_border` gives the resize edges.
- **Title bar.** `TitleBar::new()` at 38 px (`07`) holds the app icon
  (R-16) and the title text. Its close control quits through
  `on_close_window` → `request_quit`, as the window's close does today
  (`D/view/workspace.rs:177-181`).
- **Title (R-14).** `window_title(Option<&RepositoryEntry>) -> String` is
  `Manyhands — {name}` (U+2014 with a space either side), or `Manyhands`
  when no repository is selected. The Workspace calls `set_window_title`
  whenever the derived title changes, and the title bar draws the same
  string.

## Part E: Footer And Dock Menu

**Footer.** `StatusBar` at 28 px, per `02`, "Footer", without its
update-check and "to share" items (Cycle 03):

- left: the left dock toggle (`panel-left`), one toggle per left panel, a
  separator, the bottom dock toggle (`panel-bottom`), one toggle per
  bottom panel;
- right: Needs attention, a separator, one toggle per right panel, the
  right dock toggle (`panel-right`), a separator, then Settings.

A side with no panels has no dock toggle (decision 6). Ids:
`footer-dock-{left,right,bottom}`, `footer-panel-{repositories,tickets,documents,activity}`,
`footer-attention`, `footer-settings`.

**Toggles (R-4).**

- A panel toggle reads **on** when its panel is on screen. It is
  `selected` and `toggled` together, with the tooltip and accessible name
  "Hide {panel}" when on and "Show {panel}" when off.
- A dock toggle reads on when its dock is open. Its tooltip is "Hide left
  dock" or "Show left dock" (and the same for right and bottom), with the
  key added by `tooltip_with_action` (R-24).
- Activating a toggle applies Part B's rule.

**Needs attention (decision 3, R-13).** `circle-alert` and "1 needs
attention" or "N need attention" in `red`, hidden at 0. Activating it
reveals and focuses Activity. The count is `D/model/attention.rs`'s
`needs_attention(&Operations, &DraftsRegistry)`. It counts:

- each request whose state is `Finished` with `Partial`, `Blocked` or
  `Error`;
- each draft flush failure;
- one when drafts are unavailable;
- each pending row listed for a repository.

It reads only; `model/operations.rs` is unchanged.

**Settings.** `settings` icon, tooltip "Settings (Ctrl ,)" through
`tooltip_with_action`. It opens today's Settings page, as `secondary-,`
does.

**Dock menu (decision 8, R-11).** The Workspace builds a Kit `PopupMenu`
and draws it deferred and anchored at the toggle, as the Kit's
`ContextMenu` does internally (fact 15).

- The items are "Dock left", "Dock right" and "Dock bottom", each a
  `menu_with_check`, with the current side checked. Each dispatches
  `DockPanel { panel, side }` (R-12), with the toggle's focus as the
  action context.
- A secondary click on a toggle opens it at the pointer.
- On a focused toggle, `menu` or `shift-f10`, in the toggle's
  "PanelToggle" key context, opens it under the toggle, with focus in the
  menu.
- On `DismissEvent` the menu closes and focus returns to the toggle.
- Choosing the current side changes nothing.

## Part F: Keyboard

| Key | Action | Context | Guard |
| --- | --- | --- | --- |
| `secondary-b` | `ToggleLeftDock` | none | `unless_dialog`; Bold wins in "Editor" (decision 2, fact 19) |
| `secondary-alt-b` | `ToggleRightDock` | none | `unless_dialog` |
| `secondary-j` | `ToggleBottomDock` | none | `unless_dialog` |
| `menu`, `shift-f10` | `OpenDockMenu` | "PanelToggle" | `unless_dialog` |

The existing actions change as follows (R-10):

- `FocusRepositories`, `FocusNavigation` and `FocusOperations` reveal
  their panel before focusing it.
- `Find` reveals Tickets before focusing Find.
- `NextRegion` and `PreviousRegion` follow Part C's F6 order.
- `FocusContent` is unchanged.

A reveal is a layout change and is saved, apart from the quit's (R-20).
No per-panel toggle keys (Wave decision 3). Title-bar and footer controls
are Kit buttons: Tab stops, with focus shown (Root's Tab order, fact 16).

## Part G: Persistence

**Store (R-17).** `D/services/layout.rs` defines `LayoutStore::open_in(data)`,
opened by `library_services` beside the draft and resolution stores, over
`Folder::open_named(data, "layout")`. The file is `layout.json` in that
folder (fact 30). It is never written into a repository. Its JSON shape:

- `version`: `LAYOUT_VERSION`, 1;
- `area`: a `DockAreaState`, with each dock the shell took out put back
  as its kept `DockState`;
- `on`: each panel's on flag;
- `shown`: each side's shown flag.

**Restore and fallback (decision 5, R-18).** At launch the shell reads
the file, validates it, and loads the area from it with the closed docks
kept apart. Otherwise it builds the default.

| File | Result | Notice |
| --- | --- | --- |
| Missing (first launch) | Default | None |
| Folder cannot be opened, file unreadable, JSON malformed | Default | Shown |
| `version` other than 1 | Default | Shown |
| A panel name other than the five, a panel missing or repeated, a panel in the centre, `ItemAreaPanel` outside it, `on` or `shown` incomplete | Default | Shown |
| Valid | Restored, with hidden docks, moved panels and sizes | None |

The notice, "Your panel layout couldn't be restored, so the default
layout is back.", sits at the top of the Activity panel, above the
operation area and outside `view/operations/`. It has a Dismiss button,
lasts for the session, and is not counted by Needs attention (R-21). The
store never reads or writes any other file, and nothing else in the data
directory changes.

**Saving (R-19).** A save is scheduled `LAYOUT_SAVE_DELAY` (500 ms) after
the last accepted layout change, through `background_executor().timer`
(fact 32). It writes off the UI thread. A quit's start writes at once.
A failed write is not reported; the next change writes again (K-7).

## Part H: Quit

**Reveal (decision 7, R-20).** `wait_to_quit` (fact 27):

1. writes any pending layout save;
2. stops later saves;
3. reveals Activity and its dock;
4. focuses Quit now, as today.

If the app exits, the saved layout is the one from before the reveal. If
the quit returns to `Running` because a draft could not be saved, saving
resumes: Activity stays on screen, with the failure and its Retry, and
that state is saved like any other change.

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | First launch | Default layout (Part B); focus on Repositories; no notice |
| P2 | Panel toggle on a panel on screen | Off; the rest of its dock takes its space; its contents, filters, Find text and selection are kept |
| P3 | Panel toggle on a panel not on screen | On; its dock shown |
| P4 | The last on-screen panel of a dock goes off | The dock leaves the area and the item area widens; the dock toggle reads off |
| P5 | Dock toggle on an open dock, then again | Hidden, then back with the same panels, groups and sizes |
| P6 | Dock toggle on a dock with every panel off | Every panel in it on, dock shown |
| P7 | A side with no panels | No dock toggle, no dock in the area |
| P8 | Dock menu, another side | The panel at the end of that dock as a new group, on; its toggle in that side's footer group; the source dock leaves if nothing in it is on screen |
| P9 | Dock menu, current side | Nothing changes |
| P10 | A panel dragged onto the item area | Refused: the layout as before the drag |
| P11 | A panel dragged to another dock or into a tabbed group | The Kit's result; the footer follows; saved |
| P12 | Focus in a panel that is hidden (its toggle, its dock's toggle or its dock key) | Focus on the item area |
| P13 | `secondary-1`, `-2`, `-4` with the panel hidden | Revealed and focused; `secondary-2` picks the list used last |
| P14 | F6 and Shift F6 | Only on-screen stops, in decision 9's order |
| P15 | `secondary-p` with Tickets hidden | Tickets revealed; Find focused |
| P16 | `secondary-n` | New ticket… in Tickets, New document… in Documents, nothing in the item area |
| P17 | `secondary-b`, `secondary-alt-b`, `secondary-j` | Toggle the left, right and bottom docks; Bold while an editor has focus |
| P18 | Any workspace key with a sheet or dialog open | Refused (R-60), the new keys included |
| P19 | `menu` or `shift-f10` on a focused panel toggle | Menu open, focus in it; arrows and Enter choose; Escape closes it with focus back on the toggle |
| P20 | Needs attention | Decision 3's count; hidden at 0; activating it reveals and focuses Activity |
| P21 | Settings button | Today's Settings page |
| P22 | Repository selected, changed, none, removed | Window title and title bar `Manyhands — {name}`, or `Manyhands` |
| P23 | Layout change, then a restart | Same arrangement, on flags, shown docks and sizes |
| P24 | Layout file per Part G's table | That row's result; no other file in the data directory changes; no repository changes |
| P25 | Restore notice dismissed | Gone for the session; not counted |
| P26 | Quit that waits, Activity hidden | Activity and its dock shown, Quit now focused; on exit the saved layout is the earlier one |
| P27 | That quit returns to running | Activity stays shown with the failure and Retry; saved |
| P28 | Hiding or moving any panel while a request runs | The request finishes; its entry is in Activity when shown; no tab opens, closes or reloads |
| P29 | Tab through the title bar and footer | Every control reached, with focus shown and an accessible name |
| P30 | Hidden dock taken out and put back (R-3) | Focus, the panels' states and the item area's tabs unchanged |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.
Behaviour, not appearance (Wave Testing): no tests for colours, sizes,
icons or that a view renders.

| Target | What |
| --- | --- |
| `D/model/layout_tests.rs` | One table-driven test of Part B's operations and derived states (P2–P9, P13's reveal); the default (P1); F6 order from an arrangement (P14) |
| `D/model/attention_tests.rs` | One table-driven count test (P20): each source, Dismissed and Cancelled not counted, the sum |
| `D/view/shell/title_tests.rs` | Title derivation (P22) |
| `D/view/shell/tokens_tests.rs` | Both modes give `07`'s values for the tokens `ItemColors` reads (A15; E-7) |
| `D/view/dock_tests.rs` | Through the area: hiding, a dock taken out and back (P4, P5, P30), moves through the menu's action (P8, P9), the centre refusal through `move_panel` into the centre, the same edit a drop commits (P10), focus on hide (P12), state kept (P2, P28) |
| `D/view/footer_tests.rs` | Toggles by click and their accessible state and tooltip (P2–P7), the dock menu by secondary click (P8), Needs attention (P20), Settings (P21) |
| `D/view/dock/restore_tests.rs` | **One table-driven test** of Part G's table (P24, P25; Wave Testing), and the restart test (P23): change, advance the clock, drop the shell, open a fresh one over the same data directory, as `D/view/item/posting_tests/relaunch.rs` does |
| `D/view/keyboard_tests/shell.rs` | Dock keys (P17), region keys and Find revealing (P13, P15), F6 skipping (P14), the dock menu by `menu` and `shift-f10` (P19), Tab reach (P29), and Ctrl B Bold in an editor, following D3's editor-test rules |
| `D/view/item/sheet_tests.rs` | E-6 (P18) |
| `D/view/workspace_tests.rs` | Quit with Activity hidden (P26) and a failed draft save during quit (P27), next to the existing quit tests |

The title is read with `VisualTestContext::window_title()` (fact 18). No
test waits in real time; saves are reached with `advance_clock`. Every
test uses its fixture's data directory. D5's tests once swept the real one
(D5 ledger, Task 17).

**On-screen checklist** (Linux Wayland, release build, the owner), in
D5's format, written in the final task:

- the title bar: drag, double-click, the three controls, resize edges;
- dragging panels between docks and into a tabbed group, and onto the
  item area;
- resizing docks and groups;
- the context-menu key (U-3);
- Ctrl B outside and inside an editor;
- the layout after a restart;
- building 1a, 1b and 1c (without Projects and Teams) from the footer and
  by dragging (Wave Integration Gate).

## Named Edits To Pre-Existing Tests

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `Region::Navigation` in `view/find_tests.rs` (2), `view/keyboard_tests.rs` (12), `view/keyboard_tests/actions.rs` (2), `view/keyboard_tests/editing.rs` (8), `view/keyboard_tests/frame.rs` (2), `view/lifecycle/made_ready_tests.rs` (1), `view/workspace_tests.rs` (3); `"region-navigation"` in `view/item/page_tests.rs` (4) | Each becomes `Region::Tickets` or `Region::Documents` (`region-tickets`, `region-documents`), whichever list the step is in; Find's steps are Tickets (decision 4). Nothing else changes | C |
| E-2 | Setup that clicks or chooses `pane-tickets` or `pane-documents`: `view/create_tests.rs` (7), `view/keyboard_tests.rs` (3), `view/keyboard_tests/editing.rs` (3), `view/lifecycle/made_ready_review_tests.rs` (1), `view/operations_tests.rs` (2), `view/tabs_tests.rs` (1), `view/tickets_tests.rs` (7) | A new harness helper `Shell::use_list(Pane, cx)` calls `Workspace::show_pane`, which sets the list used last and focuses it as the choice did. The rest of each test is unchanged | C |
| E-3 | `view/keyboard_tests.rs` `switch_panes` (`:276`, in `every_core_action_works_by_keyboard_alone`); `view/tickets_tests.rs:572` `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text` | The choice and the "Showing …" line are removed by design (Cycle Scope 4). `switch_panes` reaches Tickets and back to Documents by F6 and Shift F6, and checks that `secondary-2` then focuses the list used last. The label test drops `pane-documents`, `pane-tickets` and `pane-showing` and keeps its other ids. **These remove assertions; owner approval of this design approves them** | C |
| E-4 | `view/workspace_tests.rs:897` `f6_cycles_the_regions_and_secondary_digits_focus_each`; Content → F6 → Operations at `view/keyboard_tests/frame.rs:403-404`, `view/keyboard_tests/discussion.rs:358-359`, `:373-374`, `:404-405`, `view/keyboard_tests/editing.rs:189-190` | F6 from Repositories: Documents, Content, Tickets, Operations, Repositories; Shift F6 the reverse; the digits table adds `Region::Tickets` and `Region::Documents` for `secondary-2` (decision 9). Content → F6 lands on Tickets | C |
| E-5 | `keys_tests.rs:12` `each_key_is_bound_to_its_action` | The table grows from 17 to 22 rows: `secondary-b`, `secondary-alt-b`, `secondary-j`, and `menu` and `shift-f10` in "PanelToggle" | F |
| E-6 | `view/item/sheet_tests.rs:673` `no_workspace_key_acts_behind_the_sheet` | The key list adds `secondary-b`, `secondary-alt-b` and `secondary-j`. Each also asserts that the layout is unchanged | F |
| E-7 | `view/item/tokens_tests.rs:7-33` | Also asserts that `accent` and `faint` equal the chrome tokens in both modes (A15) | A |
| E-8 | Appearance tests broken by the shell's deliberate layout change. Candidate: `view/item/frame_tests.rs:188` `the_page_fills_the_region_and_the_discussion_starts_below_it`, whose 760 px column is wider than the item area at 1440 px with the default docks (1440 − 290 − 440 = 710 px) | Deleted, not updated (Wave Testing; AGENTS.md). Each is named in the ledger with the assertion it made. A test that fails on behaviour is not deleted: it stops for the owner unless it is listed here | C |

The harness keeps its 1440 × 1400 window (R-26). E-2 to E-4 change
journeys that the Wave says must keep passing. They change how a step is
reached, not what the step proves; the step's own assertions stay.

## Document Amendments

Applied in the approval commit:

1. **Wave Cycle 01, Persistence.** "report it in Activity, when the file
   is missing, unreadable or of another version" now reads: a missing
   file restores the default quietly; other failures report (decision 5).
   The Risks row "unknown panels dropped" now reads: a file naming any
   panel other than the five, or missing one, restores the default with
   the notice (decision 5).
2. **Wave Cycle 01, Default layout.** "Rows are 28 px" is Cycle 02's row
   default (decision 11).
3. **Wave Cycle 01, Dock menu.** It also opens with Shift F10
   (decision 8).
4. **Wave Deferred Work.** It now lists saving open tabs, the focused
   pane and the scope with the layout (`02`, "Persistence"; Cycle
   Exclusions).
5. **Wave Cycle 01, Split line.** One Cycle (decision 1).
6. **Handoff `08`.** The dock names verified against 0.6.6 (fact 1):
   `DockLayout`, `DockSkin`, `DockAreaRenderer`, `TabGroupRenderer`; no
   `DockItem`.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Panels are thin views: their bodies, state and focus handles stay in the Workspace, which each panel renders from and observes | Every Workspace change redraws every panel |
| R-2 | The app's own group renderer inside a `DockAreaRenderer` that delegates everything else to DockSkin; panels not closable or zoomable; no Kit dock toggles | A later Kit header feature is not picked up |
| R-3 | A panel that is off reports not visible. A dock that is not open is taken out of the area (dump, keep its `DockState`, load without it) and loaded back when it opens | Each dock change reloads the area once |
| R-4 | A panel toggle reads on when its panel is on screen; one for a panel not on screen shows it and its dock. A dock toggle reads on when its dock is open | A panel that is on inside a hidden dock reads off |
| R-5 | A menu move puts the panel at the end of the target dock as a new group, on, and its side shown | A tabbed group needs a drag |
| R-6 | After each `LayoutChanged` the dump is checked; a centre holding anything but the item area loads the last accepted state | The item area briefly shows the dropped panel |
| R-7 | `Region::Navigation` becomes `Tickets` and `Documents`. `Operations`, its ids, `operation-area` and the action names (`FocusNavigation` and the rest) are kept | Internal names differ from panel names |
| R-8 | `secondary-2` focuses the list used last (`Pane`, default Documents), set when either panel takes focus | — |
| R-9 | Focus in a panel being hidden goes to the item area. Start focus is Repositories if on screen, else the first F6 stop | — |
| R-10 | Dock keys are `secondary-b`, `secondary-alt-b`, `secondary-j` (Cmd on macOS, as Zed) in no context, guarded by `unless_dialog`; region keys, Find and Needs attention reveal | — |
| R-11 | The dock menu is a Kit `PopupMenu` the Workspace draws anchored to the toggle, opened by a secondary click or `menu`/`shift-f10`. Not `ContextMenuExt` (no keyboard path) and not `NativeMenu` (an OS menu on macOS and Windows that desktop tests cannot drive) | The menu looks like the Kit's, not the system's, on macOS and Windows |
| R-12 | `DockPanel { panel, side }` is a data action with `#[action(no_json)]` (`G/action.rs:70-79`) | — |
| R-13 | "1 needs attention" / "N need attention". Pending rows count one each; a repository whose pending read failed counts nothing | A failed pending read is visible only in Activity |
| R-14 | Title `Manyhands — {name}` with U+2014, set only when it changes | — |
| R-15 | Client decorations on every platform's options (fact 13) | A compositor that refuses them draws its own bar too (K-4) |
| R-16 | The app icon is `packaging/icons/manyhands-1024.png`, served by `DesktopAssets` | 55 KB in the binary |
| R-17 | The layout lives at `<data>/layout/layout.json` through `Folder::open_named`, opened in `library_services` | — |
| R-18 | Part G's validation table | A file a later build could partly use is replaced |
| R-19 | Saves 500 ms after the last change, off the UI thread; at a quit's start, at once; a failed write is not reported | One change lost silently (K-7) |
| R-20 | A waiting quit writes the pending save, stops saving and reveals Activity. A quit that returns to running resumes saving and keeps Activity on screen | Activity stays shown after a failed quit |
| R-21 | The restore notice sits in the Activity panel above the operation area, dismissible, session only, not counted | — |
| R-22 | Header counts: repositories listed, tickets listed under the current filters, documents listed; Activity has none; no count until the list is loaded | — |
| R-23 | Footer icons: `folder-git-2`, `ticket`, `file-text`, `activity`; `panel-left`, `panel-right`, `panel-bottom`, `settings` and `activity` are added to `ItemIcons` (`D/assets.rs:15-55`) | — |
| R-24 | Dock toggle tooltips "Show left dock" / "Hide left dock" (right, bottom) with the key from `tooltip_with_action`; accessible names are the same text without the key | — |
| R-25 | The Activity body scrolls, and a focused control in it is revealed with `ContentScroll` markers (fact 33), since the bottom dock is 150 px high | — |
| R-26 | The harness window stays 1440 × 1400 | Appearance tests that assumed a wider item area are deleted (E-8) |
| R-27 | A hidden panel stays in the area with all its state; only its dock may be taken out | — |
| R-28 | The file's own `version` decides; the area's `version` is set to it and not trusted (fact 5) | — |
| R-29 | Every new workspace action is wrapped in `unless_dialog` | — |

## Known Limits

- **K-1.** Docks and groups resize by mouse only; the Kit's handles take
  no focus (`B/resizable/panel.rs`).
- **K-2.** The Kit's drop indicator shows over the item area, though the
  drop is refused (R-6).
- **K-3.** Only a tabbed group's active tab is an F6 stop (decision 9).
- **K-4.** A compositor that refuses client decorations draws its own
  title bar above the Kit's (R-15).
- **K-5.** That the context-menu key arrives as `menu` under Wayland is
  inferred (U-3); Shift F10 works either way.
- **K-6.** Open tabs, the focused pane and the scope are not saved
  (Exclusions).
- **K-7.** A failed layout write is not reported (R-19).
- **K-8.** Toggles and tabs carry roles and names, but the test platform
  builds no accessibility tree (fact 17). Keyboard reach is tested;
  screen-reader output is not (RFC 344-351).
- **K-9.** A window resize does not save; the saved pixels are those of
  the last change (fact 6).
- **K-10.** D6's attention states are not counted until the second of D6
  and this Cycle merges (decision 3).
- **K-11.** A dock's only panel cannot be dragged out (fact 10); its dock
  menu moves it.

## Unverified

- **U-3** (Cycle). The context-menu key's name under Wayland (fact 19).
  The native checklist confirms it.
- **U-4.** That a panel's `render` can update the Workspace entity to
  build its body. Inferred from fact 12. The first panel test in Task 4b
  proves it; if it fails, the task stops with a design checkpoint.
- **U-5.** That `gpui_kit` re-exports the `Action` derive used by R-12.
  Copy the derive and imports from `C/native_menu/mod.rs:363`.
- **U-6.** The 0.6 : 1.4 rescale (fact 3), proved by the default-layout
  test reading the dumped sizes.

## Alternatives Rejected

- **DockSkin's header:** it always offers Zoom and, when closable, Close,
  and draws its own dock toggles (fact 9).
- **The Kit's dock closing for hiding:** a 29 px bottom strip, and a full
  width for a dock whose panels are all hidden (fact 8).
- **Taking hidden panels out of the area:** the panel loses its place in
  its group.
- **Drawing the quit feedback outside any panel:** two homes for the same
  controls, while D6 adds to them in the operation area.
- **`set_locked` to refuse centre drops:** it locks every dock (fact 10).
- **An app model that rebuilds the area on every change:** it fights the
  Kit's own drag edits.

## Risks

| Risk | Control |
| --- | --- |
| The 0.6.6 dock differs from the handoff's names and examples | Facts 1–11 read from the locked source; Task 4b opens with a design checkpoint |
| The region change reaches most desktop tests | E-1 to E-8; decision 12's whole-desktop runs at Tasks 4a, 4b and 9 |
| Activity at 150 px hides controls that keyboard tests reach | R-25 |
| A dock reload (R-3) loses focus or panel state | P30, tested in Task 5 |
| D6 changes the operation area and the quit path at the same time | No edit to `view/operations/` or `model/operations.rs`; the second to merge adapts (Cycle, In flight) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | Handoff dock names absent in 0.6.6 | Settled | Fact 1; amendment 6 | `B/dock/state_convert.rs:8-9`; `C/dock/mod.rs:48-55` |
| Planning research | A hidden bottom dock keeps a strip; an all-hidden dock keeps its width | Ruling | R-3 | `B/dock/dock_area.rs:1325-1354`, `:1702-1715` |
| Planning research | No drop predicate | Ruling | R-6; Cycle U-2 settled | `B/dock/tab_group.rs:729-741` |
| Planning research | The test window records the title | Settled | Cycle U-1 settled | `G/app/test_context.rs:861-864` |
| Planning research | The context menu has no keyboard path; NativeMenu is native on macOS and Windows | Ruling | R-11 | `C/menu/context_menu.rs:296-300`; `C/native_menu/mod.rs:201-222` |
| Planning research | TitleBar draws Linux controls only with client decorations | Ruling | R-15 | `C/title_bar.rs:267-270`; `G/window.rs:1596-1597` |
| Planning research | A cached panel would not see Workspace changes | Ruling | R-1 | `G/view.rs:484-499`; `C/dock/tab_panel.rs:733` |
| Planning research | A waiting quit can return to running | Ruling | R-20 | `D/view/workspace/lifecycle.rs:202-215` |
| Planning research | Region ids, the pane choice and F6 order are pinned by tests | Named edits | E-1 to E-4 | Facts 22–24; test inventory |
| Planning research | Activity's 150 px versus keyboard tests' visibility checks | Ruling | R-25 | `D/view/keyboard_tests.rs:157` |
