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

Revision 2, 2026-11-08, after review round 1 (Review Record) and the
owner's answers that became decisions 12–15. Revision 2 has not yet been
re-reviewed. Grounded at the ticket branch `2b1dc4d5`, which holds
origin/main `8da9afd4`.

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
- The **operation area** is the code's name for today's strip at the foot
  of the window. The **Activity panel** is the dockable panel that wraps it
  unchanged.

## Baseline Facts At `2b1dc4d5`

**Kit dock (0.6.6).**

1. **Names.** The dock is a rewrite of the API the handoff names.
   `DockItem`, `StackPanel`, `TabPanel` and `set_left_dock` do not exist;
   "StackPanel" and "TabPanel" survive only as container names in a saved
   file (`B/dock/state_convert.rs:8-9`). The types are reached as
   `gpui_kit::component::dock::{DockArea, DockAreaRenderer, DockAreaState,
   DockEvent, DockLayout, DockPlacement, DockState, PanelState, PanelInfo,
   PanelEvent, PanelId, InsertTarget, TabGroupRenderer, TabGroupContext,
   register_panel, PanelBuildContext, …}` (`C/dock/mod.rs:48-55`), with
   `BasePanel` (`:32`), `Panel`, `PanelHandle` and `panel_handle` (`:56`)
   and `DockSkin` (`:122`). gpui-kit re-exports whole crates only
   (`gpui-kit-0.6.6/src/lib.rs:106,143`).
2. **DockArea.**
   - `DockArea::new(id, version: Option<usize>, window, cx)`
     (`B/dock/dock_area.rs:126`) and
     `with_renderer(Rc<dyn DockAreaRenderer>)` (`:154`).
   - `set_center(DockLayout, …)` (`:232`). `set_dock(placement,
     DockLayout, …)` takes no size or open argument (`:246`); a new dock is
     200 px and open (`:262`).
   - `remove_dock` (`:271`), `has_dock` (`:286`), `is_dock_open` and
     `toggle_dock` (`:293`, `:301`), and `set_dock_size`, floored at 100 px
     (`:348`).
   - `add_panel_view(panel, placement, Option<Pixels>, …)` merges into the
     region's first tab group, or creates the dock (`:389-447`).
   - `remove_panel` (`:469`), `move_panel(PanelId, InsertTarget, …)`
     (`:480`) and `select_panel(PanelId, …)` (`:558`).
   - `layout(placement) -> Option<&PaneTree>` (`:189`).
     `is_empty(placement, cx)` calls each panel's `visible` (`:211-214`).
   - `InsertTarget` is `Tabs { node, ix, activate }` or
     `Split { node, placement, size }` (`B/dock/layout/edit.rs:12-25`).
     `Split` beside a dock's root wraps the root (`:205-283`).
3. **DockLayout.**
   - `DockLayout::h_split()`, `v_split()`, `tabs()`
     (`B/dock/layout/builder.rs:41-51`), `.child(DockLayout,
     Option<Pixels>)` (`:71`), `.panel_view(Arc<dyn PanelView>, &App)`
     (`:101`), `.active_index(ix)` (`:114`).
   - A panel added with `.panel(entity)` shows its `panel_name` as its
     title (`C/dock/panel.rs:313-327`).
   - Sizes are pixels, with no flex type. Inference: when every slot of a
     split is sized, the sizes are rescaled to the container in proportion
     (`B/dock/dock_area.rs:1470-1481`), so the left dock's 0.6 : 1.4 is given
     as `Some(px(600.))` and `Some(px(1400.))`.
   - Each slot is at least 100 px (`B/resizable/panel.rs:259`). A new
     resizable state rescales on its first prepaint, so restored sizes may
     differ by a fraction of a pixel (`B/resizable/mod.rs:353-385`).
4. **dump.** `dump(&self, cx) -> DockAreaState` (`B/dock/dock_area.rs:794`)
   writes `version`, `center: PanelState` and
   `left_dock`/`right_dock`/`bottom_dock: Option<DockState>`
   (`B/dock/state.rs:9-23`).
   - `DockState` has private fields `panel`, `placement`, `size` and
     `open`, with a public `new` and getters (`:27-58`).
   - `PanelState { panel_name, children, info }` is public (`:67-71`).
     `PanelInfo` is `Stack { sizes, axis }`, `Tabs { active_index }` or
     `Panel(_)`.
   - Panels hidden by `visible()` are written.
   - A split's sizes are its measured sizes, else its stored sizes, else
     100 px; a zero is never written (`B/dock/dock_area.rs:827-848`).
   - A dock that is not in the area is not written (`:812`).
5. **load.** `load(DockAreaState, …) -> Result<()>` (`:716-760`) does
   the following:
   - It sets the area's version from the state without checking it,
     rebuilds every tree through the registry, gives each dock its saved
     size and open flag, emits a queued `LayoutChanged`, and always
     returns `Ok`.
   - It clears every group and split and mints new node ids from a global
     counter (`:724-729`; `B/dock/layout/tree.rs:28-32`). Element ids
     carry those ids (`B/dock/dock_area.rs:1232-1234`, `:1773`), and keyed
     element state, such as a Kit button's focus handle
     (`C/button/button.rs:586-588`), is looked up by the full id path
     (`G/window.rs:4164-4181`). After a `load`, therefore, focused Kit
     buttons anywhere in the area, the item area included, have new
     handles.
   - An unregistered name becomes the renderer's placeholder
     (`C/dock/dock.rs:83-93`). A panel missing from the state is simply
     absent.
   - Registry builders run inside `load`, while the area is being updated
     (`B/dock/dock_area.rs:1593-1611`).
6. **Events.** `DockEvent::{LayoutChanged, DragDrop { item, target }}`
   (`:39-45`).
   - `LayoutChanged` follows `set_center`, `set_dock`, `remove_dock`,
     `toggle_dock`, `set_dock_size` and `load`.
   - It also follows every committed edit (add, remove, move, drop, close,
     `select_panel`, tab switches, `:1112-1121`) and the end of a
     split-divider drag (`:1072-1073`).
   - With DockSkin's renderer only, it follows mouse-up after a dock-edge
     resize (`C/dock/dock.rs:219-231`).
   - A window resize and a change in a panel's `visible()` emit nothing.
     `DragDrop` is for host-owned drags only; a panel drag is a committed
     edit.
7. **Panel traits.** `BasePanel` (`B/dock/panel.rs:21-93`):
   - `panel_name` is required;
   - `visible`, `closable` and `zoomable` default to true;
   - `dump` defaults to `PanelState::new(panel_name)`;
   - supertraits `EventEmitter<PanelEvent> + Render + Focusable`;
   - nothing in the Kit subscribes to `PanelEvent`.

   The component `Panel` (`C/dock/panel.rs:71-149`) adds `title`,
   `title_suffix`, `toolbar_buttons`, `dropdown_menu`, `zoom_control`
   (default `Some(PanelControl::Menu)`) and `title_bar` (default true).
   `PanelHandle::of` recovers them from a view (`:233`).
8. **Hiding and placement.**
   - A slot whose panels are all `visible() == false` gives its space to its
     siblings (`B/dock/dock_area.rs:1238-1254`, `:1315-1323`).
   - A dock whose panels are all hidden keeps its size: `render_dock` sizes
     it from open and size only (`:1325-1354`).
   - A closed side dock takes 0 px. A closed bottom dock keeps a 29 px
     strip, `CLOSED_BOTTOM_STRIP` (`:1702-1715`).
   - The side docks run the full height. The bottom dock sits in the
     centre column, below the centre (`:1415-1431`).
   - A dock emptied by a drag keeps its empty root and is drawn at full
     size (`B/dock/layout/normalize.rs:254-268`).
   - A tab group falls back to its first visible panel when its active one
     is hidden (`B/dock/tab_group.rs:202-208`).
9. **Header and renderer.**
   - DockSkin's group renderer, `TabGroupSkin`, is `pub(crate)`
     (`C/dock/tab_panel.rs:185`). It always draws a menu button whose menu
     holds Zoom (disabled at most) and, when closable, Close (`:269-351`).
     It draws the Kit's own dock toggle buttons (`:205-265`) and caches the
     active panel's view (`:733`).
   - The replacement point is `DockAreaRenderer::tab_group_renderer`, the
     trait's only required method (`B/dock/dock_area.rs:1747-1840`). It
     returns a `TabGroupRenderer` (`B/dock/tab_group.rs:894-953`) with
     `frame`, `content_frame`, `render_tab_bar` (required),
     `render_active_panel` (default uncached, `:926-934`),
     `render_drop_indicator` (default `None`, `:936-943`) and
     `render_empty`.
   - `TabGroupContext` offers `panels`, `active_panel`, `is_collapsed`,
     `is_draggable`, `is_droppable`, `select_tab`, `drag_panel` and
     `drop_panel` (`:788-885`).
   - DockSkin implements `DockAreaRenderer`; its `render_dock` adds the
     dock-edge resize handle and tracker (`C/dock/dock.rs:35-97`). Its
     `tab_group_renderer()` is public (`:95-98`). `DragPanelPreview`'s
     field is private (`C/dock/tab_panel.rs:51-53`).
10. **Dragging and dropping.** A tab or single-panel title is a drag
    source only while `!locked && !is_last_panel`; a group alone in its
    tree with at most one visible panel is the last panel
    (`B/dock/tab_group.rs:444-461`). Base attaches content-area drop
    targets to every group while `!locked` (`:729-741`). There is no drop
    predicate. `set_locked` locks dragging, dropping and closing in the
    whole area (`B/dock/dock_area.rs:217`, `:891-894`).
11. **Registry.** `register_panel(cx, name, Fn(PanelBuildContext, &mut
    Window, &mut App) -> Arc<dyn PanelView>)` (`B/dock/registry.rs:111-119`)
    keeps one global map per App, by name; registering a name again
    replaces it. `PanelBuildContext::dock_area()` names the area being
    loaded (`:27-37`).

**GPUI and Kit chrome.**

12. **Views.** GPUI renders a child view after its parent's `render` has
    returned (`G/view.rs:324-345`, `:422-445`), so a view's render can
    update an ancestor's entity. A cached view is reused unless its own
    entity is dirty (`G/view.rs:484-499`), and dirtiness is marked on
    ancestors, not descendants (`G/window.rs:2148-2160`). Reading an entity
    that is being updated panics.
13. **TitleBar.** `gpui_kit::component::TitleBar` (`C/title_bar.rs`):
    - `TITLE_BAR_HEIGHT` is 34 px (`:15`). It has `new()` (`:50`),
      `on_close_window` (`:95`), and is `Styled` and `ParentElement`
      (`:296`, `:302`). `window_options()` sets only `titlebar` and
      `app_owns_titlebar_drag` (`:81`).
    - It moves the window on drag (`:353-370`) and maximizes on a double
      click on Linux (`:344-346`).
    - It draws window controls on Linux only with `Decorations::Client`
      (`:267-270`). GPUI asks for server decorations unless
      `window_decorations` is set (`G/window.rs:1596-1597`).
    - Root draws `window_border` by default (`C/root.rs:110`, `:604-611`).
      It gives a client-decorated window its resize edges and shadow
      (`C/window_border.rs:25`, `:89-148`, `:189-200`).
    - The test window reports server decorations, so tests draw no window
      controls (`G/platform.rs:1020-1022`).
14. **StatusBar.** `gpui_kit::component::status_bar::StatusBar`
    (`C/status_bar.rs:41-95`): `new`, `left`, `right`, and a centre slot
    through `ParentElement`; `Styled`; no fixed height and no role.
15. **Menus.**
    - `PopupMenu::build(window, cx, f)` (`C/menu/popup_menu.rs:358`),
      `action_context(FocusHandle)` (`:371`), `menu_with_check(label,
      checked, Box<dyn Action>)` (`:565`) and `separator()` (`:680`).
    - Dismissing a menu emits `DismissEvent` and returns focus to its
      action context (`:1055-1074`).
    - `ContextMenuExt::context_menu` opens on a right mouse-down only and
      has no public open (`C/menu/context_menu.rs:296-300`, `:117-127`). It
      draws its menu deferred and anchored, and closes on `DismissEvent`
      (`:185-215`, `:325-350`).
    - `NativeMenu::show(position, …)` shows the operating system's menu on
      macOS and Windows, and the Kit's popup elsewhere
      (`C/native_menu/mod.rs:201-222`).
    - The `Action` derive works through `gpui_kit`, and
      `#[action(no_json)]` is valid (`G/action.rs:70-79`;
      `C/native_menu/mod.rs:363`).
16. **Buttons and tooltips.**
    - `Selectable::selected` styles a button; `toggled` sets only its
      accessible toggled state (`C/button/button.rs:483`, `:521`,
      `:766-772`).
    - A button's focus handle is internal keyed state, with no getter
      (`:586-588`).
    - Buttons are Tab stops by default (`:458-466`) and take no focus on a
      mouse-down (`:790-791`).
    - `tooltip(text)` (`:389`) and `tooltip_with_action(text, action,
      context)` (`:404`) show on hover only.
    - Root binds Tab and Shift Tab to the next and previous focus
      (`C/root.rs:24-26`).
17. **Accessibility.** GPUI 0.3.6 builds an AccessKit tree
    (`G/gpui.rs:89-91`); stateful elements take `role`, `aria_label` and
    `aria_toggled` (`G/elements/div.rs:1304-1400`). The Kit's test
    snapshots expose an element's `role()`, `label()` and `checked()`,
    taken from its accessible toggled state
    (`gpui-base-0.6.6/src/test_support.rs:65-112`, `:299-321`).
18. **Window title.** `Window::set_window_title(&str)`
    (`G/window.rs:2884-2887`). The test window records it
    (`G/platform/test/window.rs:24`, `:427-429`), and
    `VisualTestContext::window_title() -> Option<String>` reads it
    (`G/app/test_context.rs:861-864`). This settles the Cycle's U-1.
19. **Keys.**
    - `Keystroke::parse` accepts any key name
      (`G/platform/keystroke.rs:120`, `:184-188`).
    - `secondary` is Ctrl on Linux and Windows and Cmd on macOS
      (`:143-149`).
    - On Linux a key is named by its xkb keysym name in lower case
      (`gpui-pre-linux-0.3.6/src/linux/platform.rs:1083`, `:1158`).
      Inference: the Menu keysym (0xff67) arrives as `menu` (Cycle U-3).
    - zorite binds `cmd-b` and `ctrl-b` to Bold in "Editor"
      (`zorite-editor-0.10.0/src/lib.rs:59`, `:137-138`).
    - Nothing in the Kit or zorite binds `secondary-alt-b`, `secondary-j`,
      `menu` or `shift-f10`.

**The app.**

20. **Windows.**
    - `window_options` sets only the bounds, 1280 × 840
      (`D/mod.rs:76-85`). The startup-failure window uses the same options
      (`:115-137`).
    - Nothing in `src/` sets a window title.
    - Root is created over the Workspace in `root_over_workspace`
      (`D/mod.rs:89-92`); tests use `open_test_window_over`
      (`D/mod.rs:292-304`).
21. **Workspace render.** `D/view/workspace/render.rs:47-236`:
    - It builds a fixed row of Repositories (260 px), Navigation (320 px)
      and Content (flex), over the operation area (`id("operation-area")`,
      260 px, `overflow_y_scroll`) (`:103-141`).
    - It computes `finishing` (`:56`) and calls `note_focus` after the
      Navigation pane (`:73-75`). It prunes the abandon and control focus
      handles after the session and pending sections are built
      (`:118-120`; `D/view/workspace/requests.rs:549-554`).
    - Root's sheet layer is drawn inside the content region (`:83-93`).
      The sheet places itself over the whole window. The dialog and
      notification layers are children of the workspace root
      (`:234-235`).
    - `unless_dialog` (`:34-45`) drops a workspace action while a dialog
      or sheet is active (`has_active_sheet` is
      `Root.active_sheet.is_some()`, `C/window_ext.rs:129`). This is R-60's
      guard, and it does not depend on where the sheet is drawn.
    - Quit and QuitNow are not guarded (`:146-149`).
22. **Regions.**
    - `Region { Repositories, Navigation, Content, Operations }` and
      `REGIONS: [Region; 4]` (`D/view/workspace/focus.rs:11-24`).
    - Key contexts "Repositories", "Navigation", "Content" and
      "Operations" (`:52-59`); ids `region-*` and `region-*-focus-ring`
      (`:39`, `:48`).
    - `Workspace::region` (`:153-183`), `cycle_region` (`:137-151`),
      `focus_region` (`:82-109`) and `show_pane` (`:63-74`).
    - `focus_lost` falls back to `last_region` (`:117-135`).
    - The Workspace holds `regions: [FocusHandle; 4]`
      (`D/view/workspace.rs:84`) and focuses Repositories at start
      (`:165`).
23. **Key bindings.** `D/keys.rs:203-295`:
    - `f6`/`shift-f6` → `NextRegion`/`PreviousRegion`;
    - `secondary-1`…`4` → `FocusRepositories`, `FocusNavigation`,
      `FocusContent`, `FocusOperations`;
    - `secondary-p` → `Find`; `secondary-,` → `OpenSettings`;
      `secondary-q` → `Quit`;
    - all of these in no context. `secondary-n` → `NewItem` only in
      "Navigation". `QuitNow` has no key.

    `keys_tests.rs:12-56` checks a 17-row table. Tests run under each
    module's `tests` submodule, so this one is
    `desktop::keys::tests::each_key_is_bound_to_its_action` (`:297-299`).
24. **Navigation pane.** `D/view/lists.rs:39-180`:
    - Find, then the choice `PANES` (`pane-documents`, `pane-tickets`,
      `:25-28`), then `pane-showing` ("Showing documents" / "Showing
      tickets", `:23-24`), then the chosen list.
    - `Pane { Documents (default), Tickets }`
      (`D/model/navigation.rs:145-150`), with `pane()`/`set_pane()`
      (`:211-217`). `secondary-2` focuses the chosen list
      (`focus.rs:98-103`).
    - `new_item` picks the kind from `pane()`, and both New ticket… and
      New document… call it (`D/view/create.rs:183-213`).
25. **Find.** The bar is `D/view/find.rs:44-82`; `focus_find` is
    `D/view/workspace/find.rs:21-25`.
26. **Operation area.** It has no type of its own: `session_section`
    (`D/view/operations.rs:641`), `pending_section` (`:699`) and
    `finishing` (`D/view/quit.rs:87`) are Workspace methods, composed in
    `render.rs:118-141`. Its controls carry no `ContentScroll` markers, so
    nothing scrolls a focused one into view (`D/view/scroll.rs:1-22`).
27. **Quit.**
    - `QuitPhase { Running, SavingDrafts, Finishing(id), Quit }`
      (`D/view/quit.rs:55-64`).
    - `wait_to_quit` closes dialogs and the sheet, focuses Quit now and
      sets the last region to Operations
      (`D/view/workspace/lifecycle.rs:170-181`). It runs for both
      `SavingDrafts` (`:185-200`) and `Finishing` (`:152-168`).
    - When drafts cannot be saved, the phase goes back to `Running` and the
      Drafts not saved dialog opens. Its Cancel focuses Content; Quit
      anyway quits (`:128-136`, `:202-229`).
28. **Item tokens.** `D/view/item/tokens.rs` defines `accent` and `faint`
    by mode (`:47-51`: dark `#e9ad36`, `#4a5059`; light `#a86f00`,
    `#b5b0a8`), which equal `07`'s. `tokens_tests.rs:7-33` pins the header
    and page tokens, not these two.
29. **Repository name.** `RepositoryEntry::name()` gives the root's
    directory name, or the root itself (`D/model/navigation.rs:74-80`).
    The sidebar's Settings entry is `sidebar-settings`
    (`D/view/sidebar.rs:144-157`).
30. **App data.**
    - The service's data directory is given by
      `src/repository/read/credentials.rs:272-279`.
    - Stores live on the `Backend` trait (`D/services/mod.rs:174-189`,
      `:236-270`). `LibraryBackend::new` opens them in the app (through
      `library_services`, `D/mod.rs:98-106`), and in tests
      `DelayingBackend::with_resolutions` and the fixture open them lazily
      over its data directory (`D/test_support/delaying.rs:105`;
      `D/test_support/fixtures.rs:99-131`).
    - `ResolutionStore::open_in` uses `Folder::open_named(data, name)`
      (`D/services/resolutions.rs:218-224`), whose `replace` and `read`
      (`src/drafts/mod.rs:87`, `:125`, `:144`) give an owner-only folder.
      The names `layout` and `layout.json` pass its rules.
    - `D/mod_tests.rs:193-198` asserts the stores sit beside the service.
31. **Attention sources.**
    - `Operations::requests()` (`D/model/operations.rs:188`) holds
      `DesktopRequest`s. Their `RequestState::Finished(Outcome, …)`
      (`D/services/requests.rs:793`) carries
      `Outcome { Success, Noop, Partial, Blocked, Cancelled, Error }`
      (`src/results.rs:48-55`); `Dismissed` is its own state.
    - `Operations::pending()` (`D/model/operations.rs:584`) maps each
      repository to a `ListState` of its "Pending in repositories" rows.
      Busy and Failed reads show a Retry (`D/view/operations.rs:716-745`).
    - `DraftRegistry::flush_failures()` (`D/services/drafts.rs:136`,
      `:438-449`) and `unavailable()` (`:222`).
    - `model/operations.rs` has no attention entries.
32. **Time in tests.** Desktop tests move time with
    `cx.executor().advance_clock` (for example `D/view/tabs_tests.rs:575`);
    the drafts registry waits with `background_executor().timer`
    (`D/services/drafts/flush.rs:168-186`).
33. **Harness.**
    - `Shell` (`D/view/harness.rs:62-67`) owns its fixture and opens a
      1440 × 1400 window (`:119`). It has no relaunch helper.
      `posting_tests/relaunch.rs` seeds the store before a single launch.
    - Lookup by `.test_support()` ids panics on an ambiguous id
      (`gpui-base-0.6.6/src/test_support.rs:159-167`). A click needs a
      visible target (`:358-366`).
    - Helpers such as `focused_region` (`D/view/workspace_tests.rs:876-892`)
      `find` every region id.
34. **Icons and CI.**
    - `panel-left`, `panel-right`, `panel-bottom` and `settings` are Kit
      default icons (`gpui-kit-assets-0.6.6/default-icons.txt:67-73`,
      `:84`). `activity` is not.
    - CI runs desktop tests as `--features desktop-test-support --bin
      manyhands` (`.github/workflows/build.yml:129`); no new target is
      needed.

## Part A: Chrome Tokens

`D/view/shell/tokens.rs` holds `ChromeColors`. It has `07`'s 14 chrome
tokens (`chrome`, `panel`, `border`, `border_strong`, `hover`, `faint`,
`muted`, `secondary`, `text`, `selection`, `accent`, `teal`, `green`,
`red`) for both modes, chosen by `cx.theme().is_dark()` as `item_colors`
does. The title bar, panel headers and footer use it. `ItemColors.accent`
and `.faint` are read from it, so one source holds them; the rest of
`ItemColors` is unchanged.

## Part B: The Layout Model

`D/model/layout.rs` is pure: no entities, no window.

- `PanelKind { Repositories, Tickets, Documents, Activity }`, each with a
  stable name (`RepositoriesPanel`, `TicketsPanel`, `DocumentsPanel`,
  `ActivityPanel`), a label, a short label (`Repos`, `Tickets`, `Docs`,
  `Activity`; `02`) and an icon. The item area's stable name is
  `ItemAreaPanel`.
- `Side { Left, Right, Bottom }`.
- `Layout` holds:
  - per panel, whether it is **on** (the person wants it);
  - per side, whether the dock is **shown**;
  - the **arrangement**, read from the dock area: each side's groups in
    order, each group's panels and its active panel;
  - the **kept** state of each dock the shell has taken out (R-3);
  - each side's last size;
  - the **quit override** (R-20).
- Derived:
  - a panel is **on screen** when it is on, its dock is shown, and it is
    its group's active panel (or alone in its group), or when the quit
    override names it;
  - a dock is **open** when it is shown and holds a panel that is on, or
    when the quit override names a panel in it.

| Operation | Effect |
| --- | --- |
| Panel toggle on a panel on screen | It goes off. |
| Panel toggle on a panel not on screen | Reveal it. |
| Dock toggle on an open dock | Not shown. Which panels are on does not change (`02`). |
| Dock toggle on a dock that is not open | Shown; if none of its panels is on, all of them go on (decision 6). |
| Dock toggle on a side with no panels | None: that side has no toggle (decision 6). |
| Reveal (a toggle, a region key, Find, Needs attention) | The panel goes on, its dock is shown and its tab is selected. |
| Dock menu: another side | The panel moves there (Part C), is revealed, and its new side is shown (R-5). |
| Dock menu: its current side | Nothing. |
| Any operation while a quit waits | Refused (R-20). |

Every operation that changes the layout schedules a save (Part G). The
quit override is never saved.

`Layout::default()` is Wave decision 1's 1b: left Repositories over
Documents, right Tickets, bottom Activity, every panel on and every dock
shown, with sizes 290, 440 and 150 px (decision 15). The rules and the
default are unit tested (Testing).

## Part C: Dock Area And Panels

**Panels (R-1).** `D/view/dock/panels.rs` defines `RepositoriesPanel`,
`TicketsPanel`, `DocumentsPanel`, `ActivityPanel` and `ItemAreaPanel`.
Each is a thin entity holding a `WeakEntity<Workspace>`, its kind, its own
`visible` flag, and a clone of the Workspace's focus handle for its
region. All of these are set by the Workspace.

- `visible` and `focus_handle` read only the panel's own fields. The Kit
  calls them inside its own updates (facts 2 and 5), so they never read
  the Workspace.
- `render`, `title` and `title_suffix` build from the Workspace (fact 12).
  The body comes from the same Workspace methods as today, decorated by
  `Workspace::region`, so the region ids, focus rings and key contexts
  stay (fact 22).
- `closable` and `zoomable` are false, and `zoom_control` is `None`.
  `ItemAreaPanel`'s `title_bar` is false.
- Each panel observes the Workspace and notifies itself, so a change in
  the Workspace redraws it whatever caching the dock uses (fact 12).

**Bookkeeping that moves with the bodies (R-33).** Render-time steps that
depend on a section being drawn move into the body that draws it:

- the Activity body computes `finishing` and prunes the abandon and
  control focus handles after its sections are built (fact 21);
- each list body calls `note_focus` after drawing.

While Activity is hidden its pruning does not run, so no handle is
dropped.

**Builders (R-34).** Each stable name is registered once per App. The
builder finds the panel through the area being loaded
(`PanelBuildContext::dock_area()`, fact 11), using an app-global map from
each dock area to its Workspace's panels. Two windows therefore never swap
panels. A builder returns the existing panel's handle, so a `load` never
makes a second copy.

**Regions (R-7).**

- `Region` gains `Tickets` and `Documents` in place of `Navigation`, so
  `REGIONS` has five entries in F6's default order.
- Both lists keep the key context "Navigation", so `secondary-n` still
  works in each (fact 23). Their ids become `region-tickets` and
  `region-documents`.
- `Operations` keeps its name and ids, and `operation-area` keeps its id,
  which privacy tests sweep.
- `focus_lost` skips regions that are not on screen (R-9).

**Navigation split.** `navigation_pane` (fact 24) becomes two bodies.

- The Tickets body is Find, then New ticket…, the filters, the list and
  Why blocked.
- The Documents body is New document…, New folder… and the tree.
- The choice and the `pane-showing` line go (Cycle Scope 4).
- `Pane` stays as "the list used last". It is set when either panel takes
  focus, and `secondary-2` focuses it (R-8).
- New ticket… and New document… each pass their own kind. `NewItem` takes
  the kind from the focused list, not from `Pane` (fact 24).

**The area.**

- The Workspace holds an `Entity<DockArea>`, built with
  `DockArea::new("workspace-dock", Some(LAYOUT_VERSION), …)` and
  `with_renderer(ShellRenderer)`.
- Its centre is `ItemAreaPanel` alone: the content region with its tab
  strip, item view, Settings and Drafts pages, empty state, and Root's
  sheet layer, drawn where it is today (fact 21).
- R-60 holds through `unless_dialog`, which every new workspace action and
  every footer handler uses (R-29).

**Renderer (R-2).** `ShellRenderer` (`D/view/dock/header.rs`) implements
`DockAreaRenderer`. It hands `frame`, `center_frame`, `split_frame`,
`render_dock` and `build_placeholder` to an inner `DockSkin`, which keeps
the Kit's dock-edge resizing and its resize `LayoutChanged` (facts 6 and
9). Its `tab_group_renderer` returns the app's `PanelHeader`, which:

- draws a single panel as icon, title and count, per `02`, "Panel header";
- draws a tabbed group as one tab per visible panel, the active one with
  the `panel` background and the 2 px amber line (`accent`), using short
  labels when the group has more than two tabs (`02`);
- makes tabs and titles drag sources and drop targets through
  `TabGroupContext` (`drag_panel`, `drop_panel`, `select_tab`), as the
  Kit's skin wires them (`C/dock/tab_panel.rs:400-412`, `:523-604`), with
  its own drag preview (fact 9);
- hands `render_drop_indicator` to the skin's own group renderer
  (`DockSkin::tab_group_renderer()`, fact 9);
- draws no menu button, zoom, close or dock toggle buttons;
- draws no tab bar for a group whose active panel's `title_bar` is false
  (the item area);
- draws the active panel uncached (the default, fact 9).

**Hiding (R-3).**

- A panel that is off has its `visible` flag cleared, so its slot gives
  its space to the rest of its dock (fact 8).
- A dock that is not open is taken out of the area with `remove_dock`,
  after its `DockState` is kept in the layout.
- When the dock opens again, the shell rebuilds a `DockLayout` from the
  kept `PanelState` with its own panel handles. Containers `StackPanel`
  and `TabPanel` become splits and tab groups with their sizes and active
  index. The shell then calls `set_dock` and `set_dock_size`.
- Nothing else in the area is rebuilt, so the centre, the other docks and
  their focus and element state are untouched (fact 5). `load` is used
  only at launch and for R-6's refusal.
- After a drag, any dock whose tree holds no panel is taken out (fact 8).
- The Kit's own dock closing is not used: a closed bottom dock keeps a
  29 px strip, and a dock whose panels are all hidden keeps its width
  (fact 8).

**Moves (R-5).** When a panel's dock menu chooses another dock:

1. If the target side has a kept dock, it is put back first, as above.
2. If the target dock is now in the area, `move_panel` puts the panel at
   its end as a new group: `InsertTarget::Split` beside the dock's root,
   below it in a side dock and to its right in the bottom dock.
3. If the side has no panels at all, the shell takes the panel out of its
   dock (`remove_panel`) and adds it with `add_panel_view` at the side's
   last size, or its default.
4. The panel is revealed and its side shown.

A source dock left with nothing open is taken out, as above. Drags are the
Kit's own edits (fact 10); the shell reads the new arrangement after each
one.

**After each `LayoutChanged` (R-6).** The shell's own edits (R-3 and R-5)
run inside a guard and are accepted at their end, so the handler ignores
the events they cause. For any other event, the shell dumps the area,
joins the kept docks to the dump, and checks the result.

- **If the centre holds `ItemAreaPanel` alone and each of the four panels
  appears exactly once,** the joined state becomes the **accepted** state,
  the arrangement is read from it, empty docks are taken out, and a save
  is scheduled (Part G).
- **Otherwise** a drop landed on the item area. The shell loads the
  accepted state, so the panel is back where it was. This reload resets
  focus and element state in the area (K-2). It settles the Cycle's U-2,
  since the Kit has no drop predicate (fact 10).

**Focus (R-9).**

- When the focused panel stops being on screen (it is hidden, its dock is
  hidden, or another tab is selected), focus moves to the item area.
- At start, focus goes to Repositories if it is on screen, otherwise to
  the first F6 stop.

**F6 order (decision 9).** F6 visits the left dock's on-screen panels top
to bottom, the item area, the right dock's, then the bottom dock's from
left to right. Only the active tab of a tabbed group is a stop.
`cycle_region` steps this list, which the layout derives from the
arrangement.

**Tests and Activity's height (decision 15).** Activity opens at 150 px
and scrolls. Nothing reveals a focused control below its fold (fact 26).
Clicks in tests need visible targets (fact 33), so the harness opens
Activity at 400 px right after launch (R-26). Tests of the default layout,
of restore and of the restart use `Shell::open_as_launched`, which skips
that.

## Part D: Title Bar And Window Title

**Window options (R-15).** The main window uses `TitleBar::window_options()`
with the existing bounds and `window_decorations:
Some(WindowDecorations::Client)`, so Linux draws the Kit's window controls
(fact 13). Root's `window_border` gives the resize edges. The
startup-failure window keeps today's options (fact 20).

**Title bar.** `TitleBar::new()` at 38 px (`07`) holds the app icon
(R-16) and the title text. Its close control quits through
`on_close_window` → `request_quit`, as the window's close does today
(`D/view/workspace.rs:177-181`).

**Title (R-14).** `window_title(Option<&RepositoryEntry>) -> String` is
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

A side with no panels has no dock toggle (decision 6).

**Controls (R-4, R-30).**

- Each footer control is a Workspace-owned focus wrapper around a Kit
  button that is not itself a Tab stop. The pattern follows D5's
  `item-lifecycle-focus`.
- The wrapper holds the role, the accessible name and the toggled state
  (fact 17). Its handle is keyed by control, not by footer group, so it
  survives a move (fact 16).
- Ids: `footer-dock-{left,right,bottom}`,
  `footer-panel-{repositories,tickets,documents,activity}`,
  `footer-attention` and `footer-settings`.
- Enter, Space and a click activate a control. A control does nothing
  while a dialog or sheet is open (R-30); the Kit's overlays cover the
  footer then as well.

**Toggles (R-4, R-24).**

- A panel toggle reads **on** when its panel is on screen. It is
  `selected` and `toggled` together, and its accessible name and tooltip
  are "Hide {panel}" when on and "Show {panel}" when off.
- A dock toggle reads on when its dock is open. Its name is "Hide left
  dock" or "Show left dock", and the same for right and bottom; its
  tooltip adds the key through `tooltip_with_action`.
- Names come from a pure function, which is tested. Tooltips are drawn on
  hover only (fact 16).
- Activating a toggle applies Part B's rule.

**Needs attention (decisions 3 and 14, R-13).**

- It shows `circle-alert` and "1 needs attention" or "N need attention" in
  `red`, and is hidden at 0.
- Activating it reveals and focuses Activity.
- The count is `D/model/attention.rs`'s
  `needs_attention(&Operations, &DraftRegistry)`. It counts:
  - each request whose state is `Finished` with `Partial`, `Blocked` or
    `Error`;
  - each draft flush failure;
  - one when drafts are unavailable;
  - each pending row listed;
  - one for each repository whose pending read is `Failed`
    (decision 14).
- It reads only; `model/operations.rs` is unchanged.

**Settings.** `settings` icon, with the accessible name "Settings" and a
tooltip carrying `secondary-,` through `tooltip_with_action`. It opens
today's Settings page, as `secondary-,` does.

**Dock menu (decision 8, R-11).** The Workspace builds a Kit `PopupMenu`
and draws it deferred and anchored at the toggle, as the Kit's
`ContextMenu` does internally (fact 15).

- The items are "Dock left", "Dock right" and "Dock bottom", each a
  `menu_with_check` with the current side checked.
- Each dispatches `DockPanel { panel, side }` (R-12). Its action context
  is the toggle wrapper's handle.
- A secondary click on a toggle opens the menu at the pointer.
- On a focused toggle, `menu` or `shift-f10`, in the toggle's
  "PanelToggle" key context, opens the menu under the toggle with focus
  in it.
- On `DismissEvent` the menu closes and focus returns to the toggle,
  which after a move is in the panel's new footer group.

## Part F: Keyboard

| Key | Action | Context | Guard |
| --- | --- | --- | --- |
| `secondary-b` | `ToggleLeftDock` | none | `unless_dialog`; Bold wins in "Editor" (decision 2, fact 19) |
| `secondary-alt-b` | `ToggleRightDock` | none | `unless_dialog` |
| `secondary-j` | `ToggleBottomDock` | none | `unless_dialog` |
| `menu`, `shift-f10` | `OpenDockMenu` | "PanelToggle" | `unless_dialog` |

Existing actions (R-10):

- `FocusRepositories`, `FocusNavigation` (`secondary-2`) and
  `FocusOperations` reveal their panel before focusing it.
- `Find` reveals Tickets before focusing Find.
- `NextRegion` and `PreviousRegion` follow Part C's F6 order.
- `FocusContent` is unchanged.

A reveal is a layout change and is saved; the quit override is the one
exception (R-20). There are no per-panel toggle keys (Wave decision 3).
Footer controls are Tab stops with focus shown. Title-bar controls exist
only with client decorations, so they are checked natively (fact 13).

## Part G: Persistence

**Store (R-17).** `D/services/layout.rs` defines `LayoutStore::open_in(data)`.

- It is reached through a new `Backend::layout()`. `LibraryBackend` opens
  it in the service's data directory, and tests open it lazily over the
  fixture's data directory, as the resolution store is (fact 30).
- It uses `Folder::open_named(data, "layout")`, and the file is
  `layout.json` in that folder.
- It is never written into a repository and never touches another file.

The file is JSON:

- `version`: `LAYOUT_VERSION`, 1;
- `area`: a `DockAreaState` holding every dock that has panels, with the
  kept docks joined in; `open` is written and ignored on read;
- `on`: each panel's on flag;
- `shown`: each side's shown flag.

**Restore and fallback (decision 5, R-18).**
`validate(bytes) -> Result<SavedLayout, Fallback>` is pure. At launch the
shell reads the file and validates it. On success it loads the area
without the docks that are not open, which it keeps; otherwise it builds
the default.

| Case | Result | Notice |
| --- | --- | --- |
| No file (first launch) | Default | None |
| The folder cannot be opened; the file cannot be read | Default | Restore notice |
| Not JSON; JSON of another shape | Default | Restore notice |
| `version` other than 1 | Default | Restore notice |
| A leaf name other than the five; a panel missing or repeated; a panel in the centre; `ItemAreaPanel` outside it; a container other than `StackPanel` or `TabPanel`; a dock with no panels | Default | Restore notice |
| `on` or `shown` missing a key | Default | Restore notice |
| Valid | Restored with on flags, shown docks, groups, active tabs and sizes | None |

**Notices (decision 5, decision 13, R-21).** At the top of the Activity
panel, above the operation area and outside `view/operations/`:

- the restore notice, "Your panel layout couldn't be restored, so the
  default layout is back.";
- the save notice, "Your panel layout couldn't be saved.", shown after a
  failed write and removed by a later successful one.

Each has a Dismiss button and lasts for the session. Neither is counted by
Needs attention.

**Saving (R-19).**

- Every accepted layout change and every Part B operation schedules a
  save `LAYOUT_SAVE_DELAY` (500 ms) later, through
  `background_executor().timer` (fact 32). The write runs off the UI
  thread.
- When a quit starts, a pending save is written at once, on the UI
  thread, before anything else.
- While a quit waits, nothing is saved.

## Part H: Quit

**Reveal (decision 7, R-20).** When a quit starts waiting, for either
`SavingDrafts` or `Finishing` (fact 27), `wait_to_quit`:

1. writes any pending layout save;
2. stops saving, locks the area (`set_locked`, fact 10) and refuses every
   layout operation;
3. sets the quit override on Activity, which reveals it on screen without
   changing its saved on flag or its dock's shown flag, and selects its
   tab;
4. focuses Quit now, as today.

On exit, the saved layout is the person's own, from before the quit. If
the quit goes back to `Running` because drafts could not be saved, the
area unlocks and saving resumes, and the Drafts not saved dialog opens as
today. The override stays, so Activity stays on screen with the failure,
until the person toggles, hides or moves Activity or its dock. Their action
then applies to their own state. The override is never saved, so if they
then choose Quit anyway, the file still holds their layout.

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | First launch | Default layout (Part B); focus on Repositories; no notice |
| P2 | Panel toggle on a panel on screen | Off; the rest of its dock takes its space; its contents, filters, Find text and selection are kept; saved |
| P3 | Panel toggle on a panel not on screen, including an inactive tab | Revealed: on, dock shown, tab selected |
| P4 | The last on-screen panel of a dock goes off | The dock leaves the area and the item area widens; the dock toggle reads off; nothing else in the area is rebuilt |
| P5 | Dock toggle on an open dock, then again | Hidden, then back with the same panels, groups, active tabs and sizes (within 1 px) |
| P6 | Dock toggle on a dock with every panel off | Every panel in it on, dock shown |
| P7 | A side with no panels | No dock toggle, no dock in the area |
| P8 | Dock menu, each panel to each other side (into an open dock, a hidden dock, a side with no panels) | The panel at the end of that dock as a new group, revealed; its toggle in that side's footer group, keeping focus; the source dock leaves if nothing in it is open |
| P9 | Dock menu, current side | Nothing changes |
| P10 | A panel dropped onto the item area | Refused: the layout as before the drop |
| P11 | A panel dragged into another dock's tab group | The Kit's result; the footer follows; saved; restored after a restart |
| P12 | The focused panel stops being on screen | Focus on the item area |
| P13 | `secondary-1`, `-2`, `-4` with the panel hidden or an inactive tab | Revealed and focused; `secondary-2` picks the list used last |
| P14 | F6 and Shift F6 | Only on-screen stops, in decision 9's order |
| P15 | `secondary-p` with Tickets hidden or an inactive tab | Tickets revealed; Find focused |
| P16 | `secondary-n`, and clicking New ticket… and New document… | A ticket from the Tickets panel, a document from the Documents panel, whichever list was used last; nothing from the item area |
| P17 | `secondary-b`, `secondary-alt-b`, `secondary-j` | Toggle the left, right and bottom docks; Bold while an editor has focus |
| P18 | Any workspace key, or a footer control, with a sheet or dialog open, including with panels hidden | Refused (R-60); the layout unchanged |
| P19 | `menu` or `shift-f10` on a focused panel toggle | Menu open, focus in it; arrows and Enter choose; Escape closes it with focus back on the toggle |
| P20 | Needs attention | Decisions 3 and 14's count; hidden at 0; activating it, by click or by Enter, reveals and focuses Activity |
| P21 | Settings button | Today's Settings page |
| P22 | Repository selected, changed, none, removed | Window title and title bar `Manyhands — {name}`, or `Manyhands` |
| P23 | Layout change, then a restart | Same arrangement, on flags, shown docks, active tabs and sizes |
| P24 | Layout file per Part G's table | That row's result; `drafts/`, `resolutions/` and the data directory's entry list unchanged; no repository changes |
| P25 | Restore notice dismissed | Gone for the session; not counted |
| P26 | A quit that waits, with Activity hidden | Activity on screen, Quit now focused, layout operations refused; on exit the file holds the earlier layout |
| P27 | That quit returns to running (drafts not saved), then Cancel | Activity stays on screen with the failure; the file never holds the override |
| P28 | Hiding or moving any panel while a request runs | The request finishes and its entry is in Activity when shown; no tab opens, closes or reloads; nothing is sent |
| P29 | Tab through the footer | Every control reached, with focus shown, its accessible name and toggled state |
| P30 | A dock taken out and put back (R-3) | Focus on a Kit button elsewhere (for example the item header's Edit) is kept; tabs unchanged |
| P31 | A change, then quit at once | The change is in the file at the next launch |
| P32 | A layout write fails | The save notice; nothing else changes; a later successful write removes it |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.
Between tasks, only the tests a task wrote or changed run, by exact name
(decision 12). Behaviour, not appearance (Wave Testing): no tests for
colours, sizes, icons or that a view renders. Every test uses its
fixture's data directory, since D5's tests once swept the real one (D5
ledger, Task 17). No test waits in real time: saves are reached with
`advance_clock`. A test that hides a panel uses `try_find`, since the
existing region helpers `find` every region (fact 33). Sizes are compared
within 1 px (fact 3).

| Test module | What |
| --- | --- |
| `D/model/layout_tests.rs` | One table-driven test of Part B's operations and derived states (P2–P9, P13, the override); the default (P1); F6 stops from an arrangement (P14) |
| `D/model/layout_tests.rs` | **The one table-driven restore test** (Wave Testing): `validate` over every case in Part G's table (P24) |
| `D/model/attention_tests.rs` | One table-driven count test (P20): each source, `Dismissed`, `Cancelled` and `NotSent` not counted, a failed and a busy pending read, the sum |
| `D/view/shell/title_tests.rs` | Title derivation, and the window title through `window_title()` (P22) |
| `D/view/item/tokens_tests.rs` | E-7: `accent` and `faint` come from `ChromeColors` in both modes (A15) |
| `D/view/dock_tests.rs` | Through the area: hiding and a dock out and back (P2, P4, P5, P30), moves of each panel to each side through `DockPanel` (P8, P9), a refused centre drop through `move_panel` into the centre (P10), a move into a tab group through `move_panel` with `InsertTarget::Tabs` (P11), focus (P12), state kept (P2, P28), New ticket… and New document… by click (P16) |
| `D/view/footer_tests.rs` | One test per kind of control: a panel toggle, a dock toggle, the dock menu by secondary click, Needs attention, Settings (P3, P6, P8, P20, P21); names and toggled state read with `label()` and `checked()` |
| `D/view/dock/restore_tests.rs` | A seeded valid file restores (P23, P24); an unreadable file shows the notice and leaves the other stores alone (P24, P25); a change survives `Shell::relaunch` after the save delay, and after an immediate quit (P23, P31); a failed write shows the save notice (P32) |
| `D/view/keyboard_tests/shell.rs` | Dock keys (P17), region keys and Find revealing (P13, P15), F6 skipping (P14), a panel toggle by Tab and Enter, moves to each side by `menu` and `shift-f10` (P19), Needs attention by Enter (P20), Tab reach (P29), Ctrl B Bold in an editor following D3's editor-test rules, and the arranging journey |
| `D/view/item/sheet_tests.rs` | E-6 (P18) |
| `D/view/workspace_tests.rs` | Quit with Activity hidden (P26), and drafts not saved during the quit (P27), beside the existing quit tests |

`Shell::relaunch(self, cx)` removes the window, runs until parked, and
opens a new one over the same fixture (R-34 keeps the builders apart).

**On-screen checklist** (Linux Wayland, release build, the owner), in
D5's format, written in the final task:

- the title bar: drag, double-click, the three controls by mouse and by
  Tab, resize edges, and closing with an unsaved draft;
- dragging panels between docks and into a tabbed group, and onto the
  item area;
- resizing docks and groups;
- the context-menu key (U-3);
- Ctrl B outside and inside an editor;
- the layout after a restart;
- building 1a, 1b and 1c (without Projects and Teams) from the footer and
  by dragging (Wave Integration Gate);
- Orca reading one panel toggle's name and on state (RFC 349-351);
- AGENTS.md's desktop smoke run.

## Named Edits To Pre-Existing Tests

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `Region::Navigation` in `view/find_tests.rs` (2), `view/keyboard_tests.rs` (12), `view/keyboard_tests/actions.rs` (2), `view/keyboard_tests/editing.rs` (8), `view/keyboard_tests/frame.rs` (2), `view/lifecycle/made_ready_tests.rs` (1), `view/workspace_tests.rs` (3); `"region-navigation"` in `view/item/page_tests.rs` (4) | Each becomes `Region::Tickets` or `Region::Documents` (`region-tickets`, `region-documents`), whichever list the step is in. Find's steps are Tickets (decision 4). Nothing else changes | C |
| E-2 | Setup that clicks `pane-tickets` or `pane-documents` in mouse and model tests: `view/create_tests.rs` (7), `view/lifecycle/made_ready_review_tests.rs` (1), `view/operations_tests.rs` (2), `view/tabs_tests.rs` (1), `view/tickets_tests.rs` (7) | A new harness helper `Shell::use_list(Pane, cx)` calls `Workspace::show_pane`, which sets the list used last and focuses it, as the choice did. The rest of each test is unchanged | C |
| E-3 | Keyboard journeys that choose a list with the keys: `view/keyboard_tests.rs` `switch_panes` (`:276`) and `change_the_filters` (`:349`), `view/keyboard_tests/editing.rs` (`:701`, `:1106`); and `view/tickets_tests.rs:572` `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text` | The journeys reach the list by keys: F6 to Tickets, or `secondary-2` to the list used last. `switch_panes` checks that `secondary-2` follows the list used last. The choice and the "Showing …" line are removed by design (Cycle Scope 4), so their assertions go: `switch_panes`'s `pane-showing` checks, and the label test's `pane-documents`, `pane-tickets` and `pane-showing` ids. **These remove assertions; approving this design approves them** | C |
| E-4 | `view/workspace_tests.rs:897` `f6_cycles_the_regions_and_secondary_digits_focus_each`; Content → F6 → Operations at `view/keyboard_tests/frame.rs:403-404`, `view/keyboard_tests/discussion.rs:358-359`, `:373-374`, `:404-405`, `view/keyboard_tests/editing.rs:189-190` | Decision 9 changes the expected order, so the assertions change. F6 from Repositories: Documents, Content, Tickets, Operations, Repositories; Shift F6 the reverse. The digits table adds `Region::Tickets` and `Region::Documents` for `secondary-2`. Content → F6 now lands on `Region::Tickets` | C |
| E-5 | `desktop::keys::tests::each_key_is_bound_to_its_action` (`keys_tests.rs:12`) | The table grows from 17 to 22 rows: `secondary-b`, `secondary-alt-b`, `secondary-j`, and `menu` and `shift-f10` in "PanelToggle" | F |
| E-6 | `view/item/sheet_tests.rs:673` `no_workspace_key_acts_behind_the_sheet` | It starts with Documents off and the bottom dock hidden. The key list adds `secondary-b`, `secondary-alt-b` and `secondary-j`. After every key it asserts that the layout is unchanged, and it activates one footer control | F |
| E-7 | `view/item/tokens_tests.rs:7-33` | Also asserts that `accent` and `faint` equal `ChromeColors`'s in both modes | A |
| E-8 | Appearance tests broken by the shell's deliberate layout change, found in any task from 4a on. Known candidate: `view/item/frame_tests.rs:188` `the_page_fills_the_region_and_the_discussion_starts_below_it`, whose 760 px column is wider than the item area at 1440 px with the default docks (1440 − 290 − 440 = 710 px) | A whole test whose purpose is appearance is deleted, not updated (Wave Testing; AGENTS.md), and named in the ledger with what it asserted. The task's reviewer confirms each deletion. A test that checks a control is visible in order to reach it is behaviour: it stops for the owner | C |
| E-9 | `D/mod_tests.rs:193-198` (stores beside the service) | Also asserts `layout/` beside the service | G |

## Document Amendments

Applied in the approval commit:

1. **Wave Cycle 01, Persistence.** "report it in Activity, when the file
   is missing, unreadable or of another version" now reads: a missing
   file restores the default quietly, and other failures report
   (decision 5). The Risks row "unknown panels dropped" now reads: a file
   naming any panel other than the five, or missing one, restores the
   default with the notice (decision 5).
2. **Wave Cycle 01, Default layout.** "Rows are 28 px" is Cycle 02's row
   default (decision 11).
3. **Wave Cycle 01, Dock menu.** It also opens with Shift F10
   (decision 8).
4. **Wave Deferred Work.** It now lists saving open tabs, the focused
   pane and the scope with the layout (`02`, "Persistence"; Cycle
   Exclusions).
5. **Wave Cycle 01, Split line.** One Cycle (decision 1).
6. **Handoff `08`.** The dock names are verified against 0.6.6 (fact 1):
   `DockLayout`, `DockSkin`, `DockAreaRenderer`, `TabGroupRenderer`; there
   is no `DockItem`.
7. **Wave Cycle 01, Footer, and Risks row 514.** The Needs attention count
   is now read from the operations and drafts models by a new function,
   since `model/operations.rs` has no attention entries. D6's states join
   it when the second of D6 and this Cycle merges (decisions 3 and 14).
8. **Wave Cycle 01, Behavior, and `02`, "Footer".** A dock toggle on a
   dock whose panels are all off turns them all on (decision 6).
9. **Wave Cycle 01, Keyboard.** Ctrl B stays Bold while an editor has
   focus (decision 2).
10. **Ticket.** The Scope Summary's "28 px rows" defers to decision 11.
    The D6 Coordination's "attention entries" becomes the new count
    function. This is a `ticket.md` edit, committed before `ticket save`.

After approval, also comment on D6's ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`
(decision 3, Amendment 7): whichever merges second adds D6's states to
`needs_attention`.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Panels are thin views. Their bodies and state stay in the Workspace, which `render`, `title` and `title_suffix` read and each panel observes. `visible` and `focus_handle` read only the panel's own fields | Every Workspace change redraws every panel |
| R-2 | The app's own group renderer, inside a `DockAreaRenderer` that delegates everything else to DockSkin, with the drop indicator from the skin's own group renderer. Panels are not closable or zoomable, and there are no Kit dock toggles | A later Kit header feature is not picked up |
| R-3 | A panel that is off is not visible. A dock that is not open is taken out with `remove_dock`, keeping its `DockState`, and put back with `set_dock` from a `DockLayout` built from that state. `load` only at launch and for R-6 | A conversion bug changes a group on show (P5 tests it) |
| R-4 | A panel toggle reads on when its panel is on screen, an inactive tab included; one for a panel not on screen reveals it. A dock toggle reads on when its dock is open | A panel that is on but behind another tab reads off |
| R-5 | A menu move puts the panel at the end of the target dock as a new group, revealed, after putting back a kept target dock | A tabbed group needs a drag |
| R-6 | The shell's own edits are guarded and accepted at their end. Any other `LayoutChanged` is checked against the dump joined with the kept docks; a centre holding anything but the item area loads the last accepted state | A refused drop resets focus in the area (K-2) |
| R-7 | `Region::Navigation` becomes `Tickets` and `Documents`. `Operations`, its ids, `operation-area` and the action names (`FocusNavigation` and the rest) are kept | Internal names differ from panel names |
| R-8 | `secondary-2` focuses the list used last (`Pane`, default Documents), set when either list takes focus. New ticket… and New document… pass their own kind | — |
| R-9 | Focus in a panel that stops being on screen goes to the item area; `focus_lost` skips regions that are not on screen. Start focus is Repositories if on screen, else the first F6 stop | — |
| R-10 | Dock keys are `secondary-b`, `secondary-alt-b`, `secondary-j` (Cmd on macOS, as Zed) in no context, guarded by `unless_dialog`; region keys, Find and Needs attention reveal | — |
| R-11 | The dock menu is a Kit `PopupMenu` the Workspace draws anchored to the toggle, opened by a secondary click or `menu`/`shift-f10`. Not `ContextMenuExt`, which has no keyboard path, and not `NativeMenu`, an OS menu on macOS and Windows that desktop tests cannot drive | The menu looks like the Kit's, not the system's, on macOS and Windows |
| R-12 | `DockPanel { panel, side }` is a data action with `#[action(no_json)]`, defined in Task 7 | — |
| R-13 | "1 needs attention" / "N need attention". Pending rows count one each, and a `Failed` pending read one per repository (decision 14). A `Busy` read is not counted: it retries | A busy repository is visible only in Activity |
| R-14 | Title `Manyhands — {name}` with U+2014, set only when it changes | — |
| R-15 | Client decorations for the main window (fact 13). The startup-failure window keeps today's options | A compositor that refuses them draws its own bar too (K-4) |
| R-16 | The app icon is a small PNG made from `packaging/icons/manyhands-1024.png`, kept under `D/` and served by `DesktopAssets` at `brand/manyhands.png`; only the `activity` icon is added to `ItemIcons` (fact 34) | — |
| R-17 | The layout lives at `<data>/layout/layout.json`, through `Folder::open_named`, reached by `Backend::layout()` | — |
| R-18 | Part G's validation table, as a pure function | A file a later build could partly use is replaced |
| R-19 | Saves 500 ms after the last change, off the UI thread; at a quit's start, at once on the UI thread; none while a quit waits | The UI thread waits on one small write at quit |
| R-20 | A waiting quit writes the pending save, locks the area, refuses layout operations, and reveals Activity through an override that is never saved. After a return to running, the override lasts until the person changes Activity or its dock | Activity stays shown after a failed quit until the person hides it |
| R-21 | The restore and save notices sit in the Activity panel above the operation area; dismissible, session only, not counted | — |
| R-22 | Header counts: repositories listed, tickets listed under the current filters, documents listed. Activity has none, and there is no count until the list is loaded | — |
| R-23 | Footer panel icons: `folder-git-2`, `ticket`, `file-text`, `activity` | — |
| R-24 | Toggle names "Show {panel}" / "Hide {panel}" and "Show left dock" / "Hide left dock" (right, bottom), from a pure function; tooltips are the same text, with the key for dock toggles | — |
| R-25 | Withdrawn (decision 15) | — |
| R-26 | The harness opens Activity at 400 px after launch; tests of the default layout, restore and restart use `Shell::open_as_launched`. The window stays 1440 × 1400 | Existing journeys run with a taller Activity than a person sees |
| R-27 | A hidden panel stays in the area with all its state; only its dock may be taken out | — |
| R-28 | The file's own `version` decides; the area's `version` is set to it and not trusted (fact 5) | — |
| R-29 | Every new workspace action and footer handler is wrapped in `unless_dialog` | — |
| R-30 | Footer controls are Workspace-owned focus wrappers around Kit buttons, keyed by control, and do nothing while a dialog or sheet is open | — |
| R-31 | Every Part B operation schedules a save, not only `LayoutChanged` (fact 6) | — |
| R-32 | Short tab labels when a group has more than two tabs (`02`) | — |
| R-33 | `finishing`, the focus-handle pruning and `note_focus` move into the bodies that draw their sections | — |
| R-34 | Registry builders resolve panels through the area being loaded, so windows never swap panels | — |

## Known Limits

- **K-1.** Docks and groups resize by mouse only; the Kit's handles take
  no focus (`B/resizable/panel.rs`).
- **K-2.** A refused drop reloads the area, so focus returns to the item
  area and focused Kit buttons in the area lose focus (R-6, fact 5). The
  Kit's drop indicator shows over the item area before the refusal.
- **K-3.** Only a tabbed group's active tab is an F6 stop (decision 9).
- **K-4.** A compositor that refuses client decorations draws its own
  title bar above the Kit's (R-15).
- **K-5.** That the context-menu key arrives as `menu` under Wayland is
  inferred (U-3); Shift F10 works either way.
- **K-6.** Open tabs, the focused pane and the scope are not saved
  (Exclusions).
- **K-7.** Withdrawn: a failed write now shows the save notice
  (decision 13).
- **K-8.** Tests read accessible names and states from the Kit's
  snapshots (fact 17); what a screen reader announces is checked once,
  natively (checklist).
- **K-9.** A window resize does not save; the saved pixels are those of
  the last change (fact 6).
- **K-10.** D6's attention states are not counted until the second of D6
  and this Cycle merges (decision 3).
- **K-11.** A dock's only panel cannot be dragged out (fact 10); its dock
  menu moves it.
- **K-12.** Tabbed groups are made by dragging only; the dock menu makes a
  new group (R-5).
- **K-13.** Nothing scrolls a focused control in Activity into view; at
  150 px some controls need scrolling (decision 15, fact 26).

## Unverified

- **U-3** (Cycle). The context-menu key's name under Wayland (fact 19).
  The native checklist confirms it.
- **U-4.** That a panel's `render` can update the Workspace entity to
  build its body. This is inferred from fact 12, and Task 4b's first test
  proves it.
- **U-6.** The 0.6 : 1.4 rescale (fact 3), proved by the default-layout
  test reading the dumped sizes.

U-5 was settled in review (fact 15).

## Alternatives Rejected

- **DockSkin's header:** it always offers Zoom and, when closable, Close,
  and draws its own dock toggles (fact 9).
- **The Kit's dock closing for hiding:** a 29 px bottom strip, and full
  width for a dock whose panels are all hidden (fact 8).
- **`load` for every dock hide:** it rebuilds the whole area and drops
  focus and element state (fact 5).
- **Taking hidden panels out of the area:** the panel loses its place in
  its group.
- **Drawing the quit feedback outside any panel:** two homes for the same
  controls, while D6 adds to them in the operation area.
- **`set_locked` to refuse centre drops:** it locks every dock (fact 10).
- **An app model that rebuilds the area on every change:** it fights the
  Kit's own drag edits.
- **Scroll-into-view markers in the operation area:** they would edit the
  files D6 changes (decision 15).

## Risks

| Risk | Control |
| --- | --- |
| The 0.6.6 dock differs from the handoff's names and examples | Facts 1–11 read from the locked source; Task 4b opens with a design checkpoint |
| The region change reaches most desktop tests, and with decision 12 an unlisted break shows only at Task 12's fast tier | E-1 to E-9; each task runs the existing tests it touches by name; an unlisted break stops for the owner |
| Panel methods called inside Kit updates read a leased entity | R-1: `visible` and `focus_handle` read only the panel |
| A dock put back from kept state differs from the one taken out | P5 compares the dumps |
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
| Review round 1 (2 reviewers, `bb9b3cc9`) | Source and feasibility: 3 Blocking, 14 Important, 16 Minor. Contract: 2 Blocking, 17 Important, 11 Minor | — | All verified against source and authorities; applied in revision 2, except where noted below | Reviewer reports, 2026-11-08 |
| Round 1 | R-6 would undo every dock hide and move (both reviewers) | Ruling | R-6 guard and joined check | `B/dock/dock_area.rs:812`, `:770` |
| Round 1 | `load` resets focus and element state across the area | Ruling | R-3 uses `remove_dock` and `set_dock`; P30 tests a Kit button's focus | `B/dock/dock_area.rs:724-729`; `C/button/button.rs:586-588` |
| Round 1 | Activity at 150 px and no focus reveal | Owner decision | Decision 15: 150 px, scrolls, no reveal; R-25 withdrawn; R-26 test height | Owner, 2026-11-08 |
| Round 1 | R-20 saved the quit reveal after a return to running, against decision 7 | Ruling | R-20 override, never saved | Decision 7 |
| Round 1 | Token test compared colours | Settled | Test dropped; E-7 only | AGENTS.md Testing Strategy |
| Round 1 | A failed layout write was never reported | Owner decision | Decision 13: save notice | RFC 52-53 |
| Round 1 | A failed pending read counted nothing | Owner decision | Decision 14 | Decision 3 |
| Round 1 | Whole-suite runs beyond AGENTS.md | Owner decision | Decision 12: AGENTS.md as is | AGENTS.md Testing Strategy |
| Round 1 | Clicking New ticket… could create a document | Ruling | R-8; P16 | `D/view/create.rs:183-213` |
| Round 1 | Stores live on `Backend`, not `Services`; no relaunch helper; builders are global | Ruling | R-17, R-34, `Shell::relaunch` | Facts 11, 30, 33 |
| Round 1 | Tabbed groups and the quit reveal; quit wait not protected | Ruling | Part B on screen; R-20 lock | Contract 2 |
| Round 1 | Panel methods read a leased Workspace | Ruling | R-1 | Facts 2, 5 |
| Round 1 | Render bookkeeping order | Ruling | R-33 | Fact 21 |
| Round 1 | Unrecorded departures for decisions 2, 3, 6 | Settled | Amendments 7–10 | Wave Cycle 01 |
