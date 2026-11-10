---
title: "Wave 04 Cycle 01 Workspace Shell Design"
date: 2026-11-08
status: "approved"
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

**Approved 2026-11-08** by the owner, with the named edits E-1 to E-10 and
decision 17 (further existing-test edits pre-authorized). Revision 5, after
review rounds 1 to 4 (Review Record) and the owner's decisions 12–16. Decision 16 replaced the mechanism round 2
attacked: docks now open and close through the Kit itself, which held in
round 3. The quit override, which drew findings in every round, is gone:
the owner revised decision 7 so a waiting quit changes no layout (Part H).
Round 4, a scoped re-review of revision 4, found nothing Blocking;
revision 5's fixes for its findings have had no independent review. It is grounded at the ticket branch `2b1dc4d5`,
which holds origin/main `8da9afd4`.

**Amended 2026-11-09, owner-approved:** the changes marked "amended
2026-11-09" below record where the built code differs from this design (the
execution ledger, Task 12, has the evidence). The approval status above is
unchanged.

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
   - A closed side dock takes 0 px, and its tree is not drawn. A closed
     bottom dock keeps a 29 px strip, `CLOSED_BOTTOM_STRIP` (`:1702-1715`).
     Its groups are drawn collapsed, with a tab bar and no content space,
     but the renderer is still asked for the active panel
     (`B/dock/tab_group.rs:713-748`).
   - `toggle_dock` opens or closes a dock in place, keeping its tree,
     sizes and panels, and emits `LayoutChanged`
     (`B/dock/dock_area.rs:301-320`). Dragging a closed dock's edge handle
     opens it (`C/dock/dock.rs:206-215`).
   - `DockSizing::clamp` bounds a dock's size against the area and the
     opposite dock (`B/dock/dock_placement.rs:20-92`).
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
    - Every quit starts at `request_quit` (`:26-50`): the Quit action and
      the window's close both call it (`render.rs:146-149`;
      `D/view/workspace.rs:177-181`). With no job running, it quits at once
      through `finish_job` and `quit_now` (`:152-157`), without
      `wait_to_quit`.
    - The phase changes only through `set_quit` (`:141-147`). It goes back
      to `Running` in two places:
      - when every edit begin has landed, before Edits not kept is asked
        (`:101-108`, `:84-95`);
      - when drafts cannot be saved, before the Drafts not saved dialog
        opens (`:202-229`).
    - The Drafts not saved dialog's Cancel focuses the first failed
      draft's Retry, and its Quit anyway quits (`:232-262`). Edits not
      kept's Cancel focuses Content (`:128-136`).
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
      1440 × 1400 window (`:119`), sized so that item actions and
      operation entries can be clicked (`:115-118`). Today that gives the
      content region 860 × 1140 px and the operation area 1440 × 260 px
      (fact 21). It has no relaunch helper; `posting_tests/relaunch.rs`
      seeds the store before a single launch.
    - `show_pane` focuses the list only when the Navigation region
      already has focus (`D/view/workspace/focus.rs:63-74`).
    - The details sheet closes when its overlay is clicked
      (`C/sheet.rs:71`, `:254`); dialogs do not.
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
  - the **arrangement**, read from the dock area after each accepted
    change: each side's groups in order, each group's panels and its
    active panel;
- Derived:
  - A dock is **open** when it is shown and holds a panel that is on.
    The Kit's open flag follows this (R-3).
  - A panel is **displayed** when its dock is open, it is on, and it is
    its group's active panel. If the active panel
    is off, the group's first panel that is on counts, which is the Kit's
    own fallback (fact 8).
  - "On screen" means displayed.

| Operation | Effect |
| --- | --- |
| Panel toggle on a displayed panel | It goes off. If it was its group's active panel, the next panel that is on becomes active. |
| Panel toggle on a panel not displayed | Reveal it. |
| Dock toggle on an open dock | Not shown. Which panels are on does not change (`02`). |
| Dock toggle on a dock that is not open | Shown; if none of its panels is on, all of them go on (decision 6). |
| Dock toggle on a side with no panels | None: that side has no toggle (decision 6). |
| Reveal (a toggle, a region key, Find, Needs attention, a strip tab, the item view's "Show it" link) | The panel goes on, its dock is shown and its tab is selected. |
| Dock menu: another side | The panel moves there (Part C) and is revealed (R-5). |
| Dock menu: its current side | Nothing. |
| A panel dragged to another side | Revealed (R-36). |

Every operation ends with reconcile, the focus check (Part C) and a
scheduled save (Part G).

`Layout::default()` is Wave decision 1's 1b: left Repositories over
Documents, right Tickets, bottom Activity, every panel on and every dock
shown, with sizes 290, 440 and 150 px (decision 15). The rules and the
default are unit tested (Testing).

## Part C: Dock Area And Panels

**Panels (R-1).** `D/view/dock/panels.rs` defines `RepositoriesPanel`,
`TicketsPanel`, `DocumentsPanel`, `ActivityPanel` and `ItemAreaPanel`.
Each is a thin entity holding a `WeakEntity<Workspace>`, its kind, its own
`visible` flag, and a clone of the Workspace's focus handle for its
region. All of these are set by the Workspace. (Amended 2026-11-09,
owner-approved: one `RegionPanel` entity with a slot, `Slot { Panel(PanelKind),
ItemArea }`, replaces the five separate panel types. The five stable names
are unchanged: they are what persists and what `panel_name` returns. The
`impl Workspace` for the dock operations lives in `D/view/workspace/docks.rs`
and for the footer in `D/view/workspace/footer.rs`, because the Workspace's
fields are private to `view::workspace`.)

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

- The Activity body computes `finishing`, then prunes the abandon and
  control focus handles after its sections are built (fact 21). While
  Activity is not displayed its pruning does not run, so no handle is
  dropped.
- `ItemAreaPanel`'s body calls the content region's `note_focus` before
  `content_body`, as `render.rs:73-75` does today. It is the item area's
  per-frame focus note, which its scroll-into-view needs.

**Builders (R-34).**

- Each stable name is registered once per App.
- An app-global map goes from each dock area's `EntityId` to the five
  panel handles (`Arc<dyn PanelView>`). The Workspace inserts its entry
  before the area's first `load` and removes it when it is released.
- A builder looks its handle up by `PanelBuildContext::dock_area()`'s
  entity id, without upgrading the area or reading the Workspace (fact
  5). Two windows therefore never swap panels, and a `load` never makes a
  second copy.

**Regions (R-7).**

- `Region` gains `Tickets` and `Documents` in place of `Navigation`, so
  `REGIONS` has five entries in F6's default order.
- Both lists keep the key context "Navigation", so `secondary-n` still
  works in each (fact 23). Their ids become `region-tickets` and
  `region-documents`.
- `Operations` keeps its name and ids, and `operation-area` keeps its id,
  which privacy tests sweep.
- `focus_lost` skips regions that are not displayed, and its last
  fallback is the item area (R-9).

**Navigation split.** `navigation_pane` (fact 24) becomes two bodies.

- The Tickets body is Find, then New ticket…, the filters, the list and
  Why blocked.
- The Documents body is New document…, New folder… and the tree.
- The choice and the `pane-showing` line go (Cycle Scope 4).
- `Pane` stays as "the list used last". It is set when either list's
  region takes focus, and `secondary-2` focuses it (R-8).
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
`DockAreaRenderer`.

- It is built inside the area's constructor, as `DockSkin::dock_area`
  does, since `DockSkin::new` needs the area's own context
  (`C/dock/mod.rs:132-158`).
- It hands `frame`, `center_frame`, `split_frame`, `render_dock` and
  `build_placeholder` to an inner `DockSkin`. That keeps the Kit's
  dock-edge resizing and its resize `LayoutChanged` (facts 6 and 9).

Its `tab_group_renderer` returns the app's `PanelHeader`, which:

- draws a single panel as icon, title and count, per `02`, "Panel header";
- draws a tabbed group as one tab per visible panel, the active one with
  the `panel` background and the 2 px amber line (`accent`), using short
  labels when the group has more than two tabs (`02`);
- gives each tab a string id, `panel-tab-{kind}` (for example
  `panel-tab-documents`);
- makes tabs and titles drag sources and drop targets through
  `TabGroupContext` (`drag_panel`, `drop_panel`, `select_tab`), as the
  Kit's skin wires them (`C/dock/tab_panel.rs:400-412`, `:523-604`), with
  its own drag preview (fact 9);
- hands `render_drop_indicator` to the skin's own group renderer
  (`DockSkin::tab_group_renderer()`, fact 9);
- draws no menu button, zoom, close or dock toggle buttons;
- draws no tab bar for a group whose active panel's `title_bar` is false
  (the item area);
- draws the active panel uncached (the default, fact 9);
- draws a **collapsed** group (a closed bottom dock's strip, fact 8) as
  one tab per panel in it (R-35):
  - activating a tab reveals that panel;
  - strip tabs are neither drag sources nor drop targets;
  - in a collapsed group, `render_active_panel` draws nothing, so no
    hidden control can take focus.

**Opening and closing docks (R-3, decision 16).**

- A panel that is off has its `visible` flag cleared, so its slot gives
  its space to the rest of its dock (fact 8).
- A dock opens and closes in place through the Kit's `toggle_dock`
  (fact 8). Its tree, groups, sizes and panels stay in the area. The
  shell's **reconcile** step compares each dock's `is_dock_open` with the
  layout's open state and toggles the docks that differ.
- A closed side dock takes no space. A closed bottom dock keeps the Kit's
  29 px strip under the item area, drawn as above.
- While the bottom dock is closed, reconcile marks every panel in it
  visible, so the strip names each one even in a split dock (fact 8: the
  Kit hides a slot whose panels are all invisible). The strip draws no
  panel bodies. When the dock opens, reconcile sets the flags back from
  `on`.
- A dock with no panels at all is removed with `remove_dock`, and its
  last size is kept in the layout.
- Dock sizes are clamped by R-37: when the dock area's bounds are first
  non-zero, and whenever a dock opens (including a dock the Kit opens by
  its edge drag), not on every reconcile. (Amended 2026-11-09,
  owner-approved: this replaces "clamped once at launch".)
- `load` is used only at launch and for R-6's refusal. Nothing else
  rebuilds the area, so its focus and element state are untouched
  (fact 5).

**Moves (R-5).** When a panel's dock menu chooses another side:

1. If the target side has a dock, `move_panel` puts the panel at its end
   as a new group: `InsertTarget::Split` beside the dock's root, below it
   in a side dock and to its right in the bottom dock. The target dock
   may be closed.
2. If the target side has no dock, the shell takes the panel out of its
   dock (`remove_panel`) and adds it with `add_panel_view`, at the side's
   last size or its default.
3. The panel is revealed.
4. Reconcile runs: a source dock left with no panels is removed, and one
   with none on is closed.

**After each `LayoutChanged` (R-6).** The handler is idempotent. GPUI
delivers the event after the edit that caused it (`G/app/context.rs:762-776`;
`G/app.rs:1170-1177`), so it cannot tell the shell's own edits from the
person's. It handles every event the same way.

1. Dump the area. If the dump equals the accepted state, stop. (Amended
   2026-11-09, owner-approved: "equals" is the `same_layout` comparison,
   not strict equality: version, centre, per-dock presence, placement and
   open flag, dock size within 1 px, names, children, a Tabs group's
   active index, a Stack's axis, and a Stack's sizes compared as shares
   within 0.5%.)
2. If the centre holds anything but `ItemAreaPanel`, a drop landed on the
   item area. Load the accepted state, so the panel goes back where it
   was. The accepted state is a full dump with closed docks marked closed,
   so nothing hidden comes back. This reload resets focus and element
   state in the area (K-2). It settles the Cycle's U-2, since the Kit has
   no drop predicate (fact 10).
3. Otherwise the dump becomes the accepted state, and the arrangement is
   read from it. Any panel whose side changed is revealed (R-36).
   - If the Kit opened a dock that the layout has closed, the shell
     treats that as the dock toggle. That happens when the person drags
     the closed bottom strip's edge (fact 8). (Amended 2026-11-09,
     owner-approved: a dock is Kit-opened only when
     `!accepted.open && dump.open && !layout.is_open`, evaluated before
     `set_arrangement`. The naive wording would mistake a dock left open
     with only off panels, or a shell-initiated toggle, for a Kit-opened
     dock and turn its panels on, which breaks P4.)
   - Reconcile runs, then the focus check.
   - A save is scheduled (Part G).

Reconcile toggles only docks that differ, and step 1 stops once nothing
changes, so the events settle.

**Launch (R-38).** The launch builds or loads the area, then, in the same
update, sets the accepted state and the arrangement from a dump of it.
The launch's own `LayoutChanged` events arrive after that update ends
(fact 6), so step 1 absorbs them: no reveal, no Kit-opened dock and no
save follow from a launch. No flag is needed.

**Focus (R-9).**

- The focus check runs after every reconcile. If focus is in a region
  that is not displayed (its panel is hidden, its dock is closed, or
  another tab is selected), it moves to the item area. The strip's group
  frame tracks the active panel's handle (fact 8), so the check, not
  `focus_lost`, catches a closed bottom dock.
- The same check runs when focus enters a region that is not displayed,
  for example a click on the closed strip outside its tabs, whose frame
  tracks the active panel's handle (`B/dock/tab_group.rs:675-711`).
- A tab click is the exception: the Kit focuses the panel it selects, and
  focus stays there.
- A refused operation moves no focus.
- At start, focus goes to Repositories if it is displayed, otherwise to
  the first F6 stop.

**F6 order (decision 9).** F6 visits the left dock's displayed panels top
to bottom, the item area, the right dock's, then the bottom dock's from
left to right. Only the displayed tab of a tabbed group is a stop.
`cycle_region` steps this list, which the layout derives from the
arrangement.

**Tests and geometry (decision 15, R-26).** Activity opens at 150 px and
scrolls. Nothing reveals a focused control below its fold (fact 26).
Clicks in tests need visible targets (fact 33), so the harness keeps
today's click geometry:

- The window is 1590 × 1726, so the item area is 860 × 1140, as today's
  content region is. That is 1590 − 290 − 440 wide and
  1726 − 38 − 28 − 520 high.
- `Shell::open` and `Shell::launch` open Activity at 520 px. Activity sits
  in the centre column (fact 8), 860 px wide against today's 1440, so its
  two sections are about half as wide; twice today's 260 px keeps their
  wrapped entries in view.
- A test that resizes the window itself uses
  `Shell::resize_keeping_content(width, height)` (E-10). It takes the size
  the test asks for today and opens the window 150 px wider and 326 px
  taller, so the item area keeps the size it had.
- `Shell::open_as_launched` and `Shell::relaunch` never resize anything.
  Tests of the default layout, restore and the restart use them.

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
  button that is not itself a Tab stop. The pattern follows Quit now's
  wrapper (`D/view/quit.rs:87-129`).
- The wrapper holds the role, the accessible name and the toggled state
  (fact 17). Its handle is keyed by control, not by footer group, so it
  survives a move (fact 16).
- Ids: `footer-dock-{left,right,bottom}`,
  `footer-panel-{repositories,tickets,documents,activity}`,
  `footer-attention` and `footer-settings`.
- Enter, Space and a click activate a control. A control does nothing
  while a dialog or sheet is open (R-30). A click on the footer while the
  details sheet is open lands on the sheet's overlay and closes the sheet
  (fact 33).

**Toggles (R-4, R-24).**

- A panel toggle reads **on** when its panel is displayed. It is
  `selected` and `toggled` together, and its accessible name and tooltip
  are "Hide {panel}" when on and "Show {panel}" when off. (Amended
  2026-11-09, owner-approved: `selected` is on the inner Kit button and
  `aria_toggled` is on the Workspace-owned wrapper, so assistive
  technology sees one toggle, not two.)
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
  is the toggle wrapper's handle, so choosing an item focuses the toggle
  first. After a move, the toggle sits in the panel's new footer group.
- A secondary click on a toggle opens the menu at the pointer.
- On a focused toggle, `menu` or `shift-f10`, in the toggle's
  "PanelToggle" key context, opens the menu above the toggle with focus
  in it. (Amended 2026-11-09, owner-approved: the dock menu opens above
  the toggle, not under it, because the footer is at the window foot. Its
  bottom-left is at the pointer or the toggle's top-left, kept inside the
  window with an 8 px margin.)
- On `DismissEvent` the menu closes. Opened by key, focus is back on the
  toggle. Opened by mouse, focus is where it was before (fact 15).
- Choosing the current side changes nothing.

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
- `show_holder`, which the item view's "Show it" link reaches
  (`D/view/workspace/requests.rs:225-247`), reveals Activity before it
  focuses the entry or the region.
- `Find` reveals Tickets before focusing Find.
- `NextRegion` and `PreviousRegion` follow Part C's F6 order.
- `FocusContent` is unchanged.

A reveal is a layout change and is saved. There are no per-panel toggle
keys (Wave decision 3).
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
- The app has one main window, so there is one layout. A second window,
  which only tests open, shares the store, and the last write wins (K-16).

The file is JSON:

- `version`: `LAYOUT_VERSION`, 1;
- `area`: the accepted `DockAreaState`. Each dock's `open` flag is written
  and ignored on read.
- `on`: each panel's on flag;
- `shown`: each side's shown flag.

**Restore and fallback (decision 5, R-18).** The decision is pure:
`restore(read: Result<Option<Vec<u8>>, LayoutStoreError>) -> Restore`. It
returns either a saved layout or the default, and says whether the
restore notice shows. At launch the shell:

1. builds the area with the default layout and keeps a dump of it as the
   **default state**;
2. reads the file and calls `restore`;
3. if a saved layout came back, loads it, with each dock's open flag set
   from `on` and `shown`, and runs R-6's centre check once. If the check
   fails, it loads the default state and shows the restore notice;
4. sets the accepted state from a dump (R-38).

So the launch's own events schedule no save.

**Clamp (R-37).** The Workspace's render reads `DockArea::bounds()`
(`B/dock/dock_area.rs:177-180`, recorded in prepaint at `:1404-1405`).
The first time they are non-zero it defers the clamp, and it does the
same whenever reconcile opens a dock. The clamp works as follows:

- If the side docks that are open, or about to open, leave the item area
  less than 320 px wide, both are scaled down in proportion until it has
  320 px.
- A too-tall bottom dock is scaled likewise, until the item area has
  200 px.
- It is a layout change and is saved: a size from a larger screen is
  reduced.
- It never runs while the bounds are still zero.

The hook does not use `on_next_frame`, which tests never run
(`G/window.rs:2610-2647`).

| Case | Result | Notice |
| --- | --- | --- |
| No file (first launch) | Default | None |
| The folder cannot be opened; the file cannot be read | Default | Restore notice |
| Not JSON; JSON of another shape | Default | Restore notice |
| `version` other than 1 | Default | Restore notice |
| A dock whose `placement` differs from its slot | Default | Restore notice |
| A leaf name other than the five; a panel missing or repeated; a panel in the centre; `ItemAreaPanel` outside it | Default | Restore notice |
| A node whose `info` and name disagree (the Kit reads by `info`, `B/dock/state_convert.rs:126-185`): `Stack` must be `StackPanel`, `Tabs` must be `TabPanel`, `Panel` a leaf | Default | Restore notice |
| A `StackPanel` holding a leaf; a `TabPanel` holding anything but leaves; a `sizes` count that differs from the children; a split size of 0 or less; a `DockState` size under 100 px, which the Kit's `load` does not floor and a person could not drag back (`B/dock/dock_area.rs:748-750`; `B/dock/dock_placement.rs:150-158`); a dock with no panels | Default | Restore notice |
| `on` or `shown` missing a key or holding an unknown one | Default | Restore notice |
| A file over 1 MiB (amended 2026-11-09, owner-approved; K-21) | Default | Restore notice |
| A split size or a dock size over 1,000,000 px (amended 2026-11-09, owner-approved) | Default | Restore notice |
| A `Stack` whose `axis` is other than 0 or 1; an empty `TabPanel`; a bare leaf as a dock root; a `Tabs` `active_index` at or past the child count; a non-finite size (amended 2026-11-09, owner-approved: shapes the Kit's dump never writes) | Default | Restore notice |
| Valid | Restored with on flags, shown docks, groups, active tabs and sizes | None |

(Amended 2026-11-09, owner-approved: the last three failure rows were
added in Task 9. The file cap is checked after `Folder::read` has read the
file, so it does not bound that one read's memory (K-21).)

**Notices (decisions 5 and 13, R-21).** They sit at the top of the
Activity panel, above the operation area and outside `view/operations/`:

- the restore notice, "Your panel layout couldn't be restored, so the
  default layout is back.";
- the save notice, "Your panel layout couldn't be saved.", shown after a
  failed write and removed by a later successful one.

Each has a Dismiss button and lasts for the session. Neither is counted by
Needs attention. If the store cannot be opened, saving is off for the
session and only the restore notice shows.

**Saving (R-19).**

- **Debounced.** Every accepted change and every Part B operation restarts
  one timer, `LAYOUT_SAVE_DELAY` (500 ms), through
  `background_executor().timer` (fact 32). When it fires, it takes the
  layout as it is then.
- **Generations.** The store hands out the generation when a write is
  requested, from a counter it owns. The counter, the newest generation
  written, the lock and both test seams sit behind one `Arc` that every
  clone of the store shares, as `ResolutionStore` shares its folder
  (`D/services/resolutions.rs:213-215`; `D/test_support/fixtures.rs:97-107`).
  So a relaunch or a second Workspace never starts behind. It writes one save at a time, off the UI thread.
  Under its lock it skips any save older than the newest it has written,
  as the drafts registry keeps one write in flight
  (`D/services/drafts/flush.rs:192-215`).
- **The quit's write.** `quit_now` (`D/view/workspace/lifecycle.rs:268`)
  writes the layout at once on the UI thread, with the next generation,
  before it calls `cx.quit()`. Every exit passes through it: a quit that
  exits at once, Quit now, a job that finishes and a successful flush
  (`:152-157`, `:213-215`, `:268-281`; `render.rs:149`). A change made
  during a wait is therefore kept. A background write still in flight is
  skipped as stale. (Amended 2026-11-09, owner-approved: the write happens
  only when a save is pending, that is, one was requested and has not yet
  been confirmed written. An unchanged session writes nothing, so a newer
  build's file is not clobbered at quit; a failed earlier write leaves the
  save pending, so the quit retries it.)
- **Every write** uses `on` and `shown` from the model.

## Part H: Quit

**No reveal (decision 7, R-20).** A quit that has to wait leaves the layout
as it is. Its entries ("Saving drafts for recovery", "Finishing {job}" and
Quit now) appear in the Activity panel, as today in the operation area.
If Activity is displayed, the person sees them. If not, the quit simply
waits, and the person can open Activity (`secondary-4`, its footer toggle,
or Needs attention when a draft fails) to see why.

- `wait_to_quit` (fact 27) focuses Quit now and sets the last region to
  Operations only when Activity is displayed. Otherwise focus stays where
  closing the dialogs and the sheet left it.
- When the quit returns to `Running`, Drafts not saved's Cancel focuses
  the failed draft's Retry if Activity is displayed, otherwise the item
  area. Edits not kept's Cancel focuses the item area, as today.
- Layout operations stay available throughout. The layout is written in
  `quit_now`, the single funnel every exit passes through, and only when
  a save is pending (Part G); a change made during the wait is saved as
  usual. (Amended 2026-11-09, owner-approved: this replaces "written at
  `request_quit`'s entry", which would miss a change made during a quit
  wait.)

Removing Quit now and Quit anyway, and a footer line that shows the latest
Activity status, are recorded as later work (Document Amendment 4).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | First launch | Default layout (Part B); focus on Repositories; no notice; no save unless the clamp (R-37) ran |
| P2 | Panel toggle on a displayed panel | Off; the rest of its dock takes its space; its contents, filters, Find text and selection are kept; saved |
| P3 | Panel toggle on a panel not displayed, including an inactive tab | Revealed: on, dock shown, tab selected |
| P4 | The last panel that is on in a dock goes off, or a drag leaves only panels that are off | The dock closes: a side dock takes no space, the bottom dock keeps its strip; the dock toggle reads off |
| P5 | Dock toggle on an open dock, then again | Closed, then open with the same panels, groups, active tabs and sizes (within 1 px) |
| P6 | Dock toggle on a dock with every panel off | Every panel in it on, dock shown |
| P7 | A side with no panels | No dock toggle, no dock in the area |
| P8 | Dock menu, each panel to each other side (into an open dock, a closed dock, a side with no panels; from an open dock and from a closed one) | The panel at the end of that dock as a new group, revealed; its toggle in that side's footer group; a source dock left with no panels removed, with none on closed |
| P9 | Dock menu, current side | Nothing changes |
| P10 | A panel dropped onto the item area, with and without a closed dock | Refused: the layout as before the drop, closed docks still closed |
| P11 | A panel dragged into another dock's tab group | The Kit's result; the panel revealed; the footer follows; saved; restored after a restart |
| P12 | The focused panel stops being displayed: hidden, its dock closed (the bottom one included), or another tab selected by key or toggle | Focus on the item area. A tab click focuses the panel it selects |
| P13 | `secondary-1`, `-2`, `-4` with the panel hidden or an inactive tab | Revealed and focused; `secondary-2` picks the list used last |
| P14 | F6 and Shift F6 | Only displayed stops, in decision 9's order |
| P15 | `secondary-p` with Tickets hidden or an inactive tab | Tickets revealed; Find focused |
| P16 | `secondary-n`, and clicking New ticket… and New document… | A ticket from the Tickets panel, a document from the Documents panel, whichever list was used last; nothing from the item area |
| P17 | `secondary-b`, `secondary-alt-b`, `secondary-j` | Toggle the left, right and bottom docks; Bold while an editor has focus |
| P18 | Any workspace key with a sheet or dialog open, with panels hidden; a footer control with a dialog open | Refused (R-60); the layout unchanged |
| P19 | `menu` or `shift-f10` on a focused panel toggle | Menu open, focus in it; arrows and Enter choose; Escape closes it with focus back on the toggle |
| P20 | Needs attention | Decisions 3 and 14's count; hidden at 0; activating it, by click or by Enter, reveals and focuses Activity |
| P21 | Settings button | Today's Settings page |
| P22 | Repository selected, changed, none, removed | Window title and title bar `Manyhands — {name}`, or `Manyhands` |
| P23 | Layout change, then a restart | Same arrangement, on flags, shown docks, active tabs and sizes |
| P24 | Layout file per Part G's table | That row's result; `drafts/`, `resolutions/` and the data directory's entry list unchanged; no repository changes |
| P25 | Restore notice dismissed | Gone for the session; not counted |
| P26 | A quit that waits, with Activity hidden | The layout is unchanged; the quit proceeds; its entries are in Activity when opened; focus stays where it was. With Activity displayed, Quit now is focused, as today |
| P27 | That quit returns to running, by Drafts not saved or by Edits not kept | Cancel focuses Retry if Activity is displayed, else the item area (Drafts not saved), or the item area (Edits not kept) |
| P28 | Hiding or moving any panel while a request runs | The request finishes and its entry is in Activity when shown; no tab opens, closes or reloads; nothing is sent |
| P29 | Tab through the footer | Every control reached, with focus shown, its accessible name and toggled state |
| P30 | A dock closed and opened (R-3) | Focus on a Kit button elsewhere (for example the item header's Edit) is kept; tabs unchanged; nothing in the area is rebuilt |
| P31 | A change, then a quit that exits at once; a change made during a quit wait, then the job finishes | The change is in the file at the next launch |
| P32 | A layout write fails | The save notice; nothing else changes; a later successful write removes it |
| P33 | A tab on the closed bottom strip, including in a bottom dock split side by side and with every panel off | Every panel in the dock is named; activating one reveals it, opening the dock |
| P34 | Dragging the closed bottom strip's edge | The dock opens, counted as its dock toggle |
| P35 | A background write in flight when a quit writes | The quit's newer layout is the one in the file |
| P36 | A restored dock size too large for the window | Clamped once after the first frame (R-37), and saved; nothing happens before the first frame |
| P37 | Launch, with or without a file | No reveal, no Kit-opened dock and no save from the launch's own events (R-38) |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.

- Between tasks, only the tests a task wrote or changed run, by exact
  name (decision 12).
- Behaviour, not appearance (Wave Testing): no tests for colours, sizes,
  icons or that a view renders.
- Every test uses its fixture's data directory. D5's tests once swept the
  real one (D5 ledger, Task 17).
- No test waits in real time: saves are reached with `advance_clock`.
- A test that hides a panel uses `try_find`, since the existing region
  helpers `find` every region (fact 33).
- Sizes are compared within 1 px (fact 3).

| Test module | What |
| --- | --- |
| `D/model/layout_tests.rs` | One table-driven test of Part B's operations and derived states (P2–P9, P13); the default (P1); F6 stops from an arrangement (P14) |
| `D/model/layout_tests.rs` | **The one table-driven restore test** (Wave Testing): `restore` over every case in Part G's table (P24) |
| `D/model/attention_tests.rs` | One table-driven count test (P20): each source; `Dismissed`, `Cancelled` and `NotSent` not counted; a failed and a busy pending read; the sum |
| `D/view/shell/title_tests.rs` | Title derivation, and the window title through `window_title()` (P22) |
| `D/view/item/tokens_tests.rs` | E-7: `accent` and `faint` come from `ChromeColors` in both modes (A15) |
| `D/view/create_tests.rs` | New ticket… and New document… by click create their own kind (P16) |
| `D/view/dock_tests.rs` | Through the area: hiding, closing and opening a dock (P2, P4, P5, P30); moves of each panel to each side through `DockPanel` (P8, P9); a refused centre drop, through `move_panel` into the centre, with and without a closed dock (P10); a move into a tab group through `move_panel` with `InsertTarget::Tabs` (P11); the strip's tabs, in a split bottom dock and with every panel off (P33); a Kit-opened dock (P34); focus, including a closed bottom dock and a tab click (P12); state kept and nothing sent (P2, P28); a tab click selecting its panel |
| `D/view/footer_tests.rs` | One test per kind of control: a panel toggle, a dock toggle, the dock menu by secondary click, Needs attention, Settings (P3, P6, P8, P20, P21); a footer control refused behind a dialog (P18); names and toggled state read with `label()` and `checked()` |
| `D/view/dock/restore_tests.rs`, included from `dock.rs` as `mod restore_tests` | A seeded valid file restores, with no save after the launch (P23, P24, P37). A file that cannot be read shows the notice and leaves the other stores alone (P24, P25). A change survives `Shell::relaunch` after the save delay and after a quit that exits at once (P23, P31). A failed write shows the save notice (P32). A stale background write loses to the quit's (P35). A too-large saved size is clamped after the first frame, and saved (P36) |
| `D/view/keyboard_tests/shell.rs` | Dock keys (P17); region keys and Find revealing (P13, P15); F6 skipping (P14); a panel toggle by Tab and Enter; moves to each side by `menu` and `shift-f10` (P19); Needs attention by Enter (P20); Tab reach (P29); Ctrl B Bold in an editor, following D3's editor-test rules; the arranging journey |
| `D/view/item/sheet_tests.rs` | E-6 (P18) |
| `D/view/workspace_tests.rs` | A waiting quit with Activity hidden leaves the layout alone and keeps focus; with Activity displayed it focuses Quit now (P26); both returns to running (P27); beside the existing quit tests |

**Amendments during implementation** (amended 2026-11-09, owner-approved;
the ledger has the detail). Beyond E-1 to E-10, these existing tests were
edited under decision 17 and recorded as E-11 entries:

- Task 4a: `keyboard_tests::frame::the_strip_header_chips_and_page_run_in_tab_order`
  (`shift-f6` and a region check).
- Task 4b: `keyboard_tests::drafts::quit::a_draft_not_saved_on_quit_by_keyboard`
  (the window made 400 px wider; the owner approved widening the test and
  recording K-19).
- Task 6: `assets_tests::every_item_view_icon_loads` (the `"activity"` icon).
- Tasks 8, 9 and 11: visibility-only widenings of test helpers.

E-3's approved assertion removals are the only removals. The plan's test
names were merged into tables: Task 5's plan tests became table-driven
tests (for example `hiding_a_panel_gives_its_space_to_its_dock` and
`dock_toggles_follow_the_rules`; ledger, Task 5 design checkpoint, ruling D5).

**Harness.**

- `Shell::relaunch(self, cx)` removes the window, runs until parked, and
  opens a new one over the same fixture without resizing (R-26). R-34
  keeps the builders apart.
- A file that cannot be read is a directory named `layout.json`, which
  works the same on every platform.
- The store's `fail_writes_for_testing` seam makes writes fail (P32).
- Its `hold_writes_for_testing` seam keeps a background write in flight
  (P35). It parks the write before the store's lock, as `Backend::read_gate`
  is awaited before a blocking call (`D/services/mod.rs:190-193`), so the
  quit's write on the test thread never waits on it.

**On-screen checklist** (Linux Wayland, release build, the owner), in
D5's format, written in the final task:

- the title bar: drag, double-click, the three controls by mouse and by
  Tab, resize edges, and closing with an unsaved draft;
- dragging panels between docks and into a tabbed group, and onto the
  item area;
- resizing docks and groups, and dragging the closed bottom strip's edge;
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
| E-2 | Setup that clicks `pane-tickets` or `pane-documents` in mouse and model tests: `view/create_tests.rs` (7), `view/lifecycle/made_ready_review_tests.rs` (1), `view/operations_tests.rs` (2), `view/tabs_tests.rs` (1), `view/tickets_tests.rs` (4: `:45`, `:267`, `:549`, `:631`) | A new harness helper `Shell::use_list(Pane, cx)` sets the list used last and focuses its region, as clicking the choice did. `show_pane` alone focuses only from Navigation (fact 33). The rest of each test is unchanged | C |
| E-3 | Keyboard journeys that choose a list with the keys: `view/keyboard_tests.rs` `switch_panes` (`:276`) and `change_the_filters` (`:349`), `view/keyboard_tests/editing.rs` (`:701`, `:1106`); and `view/tickets_tests.rs:572` `every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text` | The journeys reach the list by keys: F6 to Tickets, or `secondary-2` to the list used last. `switch_panes` checks that `secondary-2` follows the list used last. The choice and the "Showing …" line are removed by design (Cycle Scope 4), so their assertions go: `switch_panes`'s `pane-showing` checks, and the label test's `pane-documents`, `pane-tickets` and `pane-showing` ids. **These remove assertions; approving this design approves them** | C |
| E-4 | `view/workspace_tests.rs:897` `f6_cycles_the_regions_and_secondary_digits_focus_each`; Content → F6 → Operations at `view/keyboard_tests/frame.rs:403-404`, `view/keyboard_tests/discussion.rs:358-359`, `:373-374`, `:404-405`, `view/keyboard_tests/editing.rs:189-190` | Decision 9 changes the expected order, so the assertions change. F6 from Repositories goes Documents, Content, Tickets, Operations, Repositories; Shift F6 goes the reverse. The digits table adds `Region::Tickets` and `Region::Documents` for `secondary-2`. Content → F6 now lands on `Region::Tickets` | C |
| E-5 | `desktop::keys::tests::each_key_is_bound_to_its_action` (`keys_tests.rs:12`) | The table grows from 17 to 22 rows: `secondary-b`, `secondary-alt-b`, `secondary-j`, and `menu` and `shift-f10` in "PanelToggle" | F |
| E-6 | `view/item/sheet_tests.rs:673` `no_workspace_key_acts_behind_the_sheet` | It starts with Documents off and the bottom dock closed. The key list adds `secondary-b`, `secondary-alt-b` and `secondary-j`. After every key it asserts that the layout is unchanged. Footer controls are tested behind a dialog instead, since a footer click closes the sheet (fact 33) | F |
| E-7 | `view/item/tokens_tests.rs:7-33` | Also asserts that `accent` and `faint` equal `ChromeColors`'s in both modes | A |
| E-8 | Appearance tests broken by the shell's deliberate layout change, found in any task from 4a on | A whole test whose purpose is appearance is deleted, not updated (Wave Testing; AGENTS.md), and named in the ledger with what it asserted. The task's reviewer confirms each deletion. A test that checks a control is visible in order to reach it is behaviour: it stops for the owner. With the harness geometry of R-26 no deletion is expected | C |
| E-9 | `D/mod_tests.rs:193-198` (stores beside the service) | Also asserts `layout/` beside the service | G |
| E-10 | The ten `simulate_resize` calls that tests make after opening a shell: `view/scroll_tests.rs:35`; `view/settings/repositories_tests.rs:650`; `view/keyboard_tests/discussion.rs:69`; `view/editing_privacy_tests.rs:779`; `view/item/posting_tests.rs:142`; `view/item/posting_tests/support.rs:96`; `view/item/frame_tests.rs:51`, `:220`; `view/item/discussion_tests.rs:108`; `view/drafts_tests.rs:317` | Each becomes `shell.resize_keeping_content(width, height, cx)` with the same numbers. The window opens 150 px wider and 326 px taller, so the item area keeps the size the test chose (R-26). Nothing else changes | C |

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
4. **Wave Deferred Work.** It gains five items:
   - saving open tabs, the focused pane and the scope with the layout
     (`02`, "Persistence"; Cycle Exclusions);
   - hiding the bottom dock completely, without the Kit's 29 px strip
     (decision 16);
   - a footer line showing the latest Activity status, such as "Saving…"
     or "Syncing…" (owner, 2026-11-08);
   - **quitting with work at risk** (owner, 2026-11-08). One blocking
     dialog lists the unsaved or unsaveable work and offers going back or
     discarding it and quitting. Save failures are handled per item
     ("discard changes") rather than globally, and Activity's quit and
     save-failure wording is revisited. This replaces Quit now and Quit
     anyway;
   - **a watchdog for pending operations** (owner, 2026-11-08). After a
     time, the person chooses to keep waiting or to abandon the job,
     offering only what the request can safely do: a stop at a safe point
     or Give up where they exist, otherwise keep waiting, or quit and
     Resume later. It is also the way out of a quit that hangs on a job.
     D6's "still stopping" line after ten seconds is a first step;
5. **Wave Cycle 01, Split line.** One Cycle (decision 1).
6. **Handoff `08`.** The dock names are verified against 0.6.6 (fact 1):
   `DockLayout`, `DockSkin`, `DockAreaRenderer`, `TabGroupRenderer`; there
   is no `DockItem`.
7. **Wave Cycle 01, Footer, and Risks row 514.** The Needs attention count
   is now read from the operations and drafts models by a new function,
   since `model/operations.rs` has no attention entries. D6's states join
   it when the second of D6 and this Cycle merges (decisions 3 and 14).
8. **Wave Cycle 01, Behavior, and `02`, "Footer".**
   - A dock toggle on a dock whose panels are all off turns them all on
     (decision 6).
   - A hidden bottom dock keeps a 29 px strip naming its panels
     (decision 16).
9. **Wave Cycle 01, Keyboard.** Ctrl B stays Bold while an editor has
   focus (decision 2).
10. **Ticket.** The Scope Summary's "28 px rows" defers to decision 11.
    The D6 Coordination's "attention entries" becomes the new count
    function. This is a `ticket.md` edit, committed before `ticket save`.
11. **Wave Cycle 01, Panels, Exit Evidence, and Risks row 513**
    (decision 7):
    - Panels: "Quit feedback stays visible with Activity hidden: the panel
      is shown when a quit starts waiting, or the feedback is drawn outside
      any panel" now reads: "A quit that has to wait changes no layout; its
      feedback is in the Activity panel, visible when Activity is shown."
    - Exit evidence: "a quit that has to wait shows its feedback with
      Activity hidden" now reads: "a quit that has to wait proceeds with
      Activity hidden, and its feedback is in Activity when opened."
    - Risks row 513's control now reads: "Cycle 01 keeps the feedback in
      the Activity panel and changes no layout when a quit waits; a
      person who wonders why opens Activity."
    - The ticket's D6 Coordination item "Quit feedback stays visible"
      changes the same way (with Amendment 10).

After approval, also comment on D6's ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`
(decision 3, Amendment 7): whichever merges second adds D6's states to
`needs_attention`.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Panels are thin views. Their bodies and state stay in the Workspace, which `render`, `title` and `title_suffix` read and each panel observes. `visible` and `focus_handle` read only the panel's own fields | Every Workspace change redraws every panel |
| R-2 | The app's own group renderer, inside a `DockAreaRenderer` that delegates everything else to DockSkin, with the drop indicator from the skin's own group renderer. Panels are not closable or zoomable, and there are no Kit dock toggles | A later Kit header feature is not picked up |
| R-3 | A panel that is off is not visible. Docks open and close in place through `toggle_dock`, reconciled with the layout. A dock with no panels is removed. `load` only at launch and for R-6 (decision 16) | A closed bottom dock keeps a 29 px strip (K-15) |
| R-4 | A panel toggle reads on only when its panel is displayed, so a panel behind another tab reads off; activating a toggle for a panel not displayed reveals it. A dock toggle reads on when its dock is open | A panel that is on but behind another tab reads off |
| R-5 | A menu move puts the panel at the end of the target dock as a new group, then reveals it | A tabbed group needs a drag (K-12) |
| R-6 | Every `LayoutChanged` is handled the same way and idempotently: a dump equal to the accepted state stops; a centre holding anything but the item area loads the accepted state; otherwise it is accepted, reconciled and saved | A refused drop resets focus in the area (K-2) |
| R-7 | `Region::Navigation` becomes `Tickets` and `Documents`. `Operations`, its ids, `operation-area` and the action names (`FocusNavigation` and the rest) are kept | Internal names differ from panel names |
| R-8 | `secondary-2` focuses the list used last (`Pane`, default Documents), set when either list takes focus. New ticket… and New document… pass their own kind | — |
| R-9 | Focus in a panel that stops being displayed goes to the item area; a refused operation moves no focus; `focus_lost` skips regions not displayed and falls back to the item area. Start focus is Repositories if displayed, else the first F6 stop | — |
| R-10 | Dock keys are `secondary-b`, `secondary-alt-b`, `secondary-j` (Cmd on macOS, as Zed) in no context, guarded by `unless_dialog`; region keys, Find and Needs attention reveal | — |
| R-11 | The dock menu is a Kit `PopupMenu` the Workspace draws anchored to the toggle, opened by a secondary click or `menu`/`shift-f10`. Not `ContextMenuExt`, which has no keyboard path, and not `NativeMenu`, an OS menu on macOS and Windows that desktop tests cannot drive | The menu looks like the Kit's, not the system's, on macOS and Windows |
| R-12 | `DockPanel { panel, side }` is a data action with `#[action(no_json)]`, defined in Task 7 | — |
| R-13 | "1 needs attention" / "N need attention". Pending rows count one each, and a `Failed` pending read one per repository (decision 14). A `Busy` read is not counted: it retries | A busy repository is visible only in Activity |
| R-14 | Title `Manyhands — {name}` with U+2014, set only when it changes | — |
| R-15 | Client decorations for the main window (fact 13). The startup-failure window keeps today's options | A compositor that refuses them draws its own bar too (K-4) |
| R-16 | The app icon is a small PNG made from `packaging/icons/manyhands-1024.png`, kept under `D/` and served by `DesktopAssets` at `brand/manyhands.png`; only the `activity` icon is added to `ItemIcons` (fact 34) | — |
| R-17 | The layout lives at `<data>/layout/layout.json`, through `Folder::open_named`, reached by `Backend::layout()`; one layout per app | — |
| R-18 | Part G's validation table, inside the pure `restore`; a launch whose loaded area fails R-6's check falls back to the default with the notice | A file a later build could partly use is replaced |
| R-19 | One debounced save, 500 ms after the last change; one write at a time off the UI thread, with generations from a counter every store clone shares, so a stale write is skipped; in `quit_now`, at once on the UI thread, when a save is pending (amended 2026-11-09, owner-approved), before every exit | The UI thread waits on one small write at quit (K-14) |
| R-20 | A waiting quit changes no layout; its entries live in Activity; Quit now and Retry take focus only when Activity is displayed (decision 7, revised) | A person with Activity hidden sees no sign of a waiting quit until they open Activity |
| R-21 | The restore and save notices sit in the Activity panel above the operation area; dismissible, session only, not counted. With no store, saving is off and only the restore notice shows | — |
| R-22 | Header counts: repositories listed, tickets listed under the current filters, documents listed. Activity has none, and there is no count until the list is loaded | — |
| R-23 | Footer panel icons: `folder-git-2`, `ticket`, `file-text`, `activity` | — |
| R-24 | Toggle names "Show {panel}" / "Hide {panel}" and "Show left dock" / "Hide left dock" (right, bottom), from a pure function; tooltips are the same text, with the key for dock toggles | — |
| R-25 | Withdrawn (decision 15) | — |
| R-26 | The harness window is 1590 × 1726. `Shell::open` and `Shell::launch` open Activity at 520 px. `Shell::resize_keeping_content` keeps each test's own item-area size (E-10). `Shell::open_as_launched` and `Shell::relaunch` resize nothing | Existing journeys run with a taller Activity than a person sees |
| R-27 | A hidden panel stays in the area with all its state | — |
| R-28 | The file's own `version` decides; the area's `version` is set to it and not trusted (fact 5) | — |
| R-29 | Every new workspace action and footer handler is wrapped in `unless_dialog` | — |
| R-30 | Footer controls are Workspace-owned focus wrappers around Kit buttons, keyed by control, and do nothing while a dialog or sheet is open | — |
| R-31 | Every Part B operation schedules a save, not only `LayoutChanged` (fact 6) | — |
| R-32 | Short tab labels when a group has more than two tabs (`02`) | — |
| R-33 | `finishing` and the focus-handle pruning move into the Activity body; the content region's `note_focus` moves into `ItemAreaPanel`'s body | — |
| R-34 | Registry builders look up panel handles by the loading area's entity id in an app-global map, inserted before the first `load` and removed when the Workspace is released | — |
| R-35 | While the bottom dock is closed, every panel in it is marked visible, so the strip names each one; activating one reveals it. Strip tabs are not dragged or dropped on. A collapsed group draws no active panel | A strip in a bottom dock split top and bottom shows only its top group (K-17) |
| R-36 | A panel whose side changes, by the menu or by a drag, is revealed; a Kit-opened dock counts as its dock toggle | — |
| R-37 | When the dock area's bounds are first non-zero, and whenever a dock opens, side docks are scaled down together so the item area keeps 320 px, and the bottom dock so it keeps 200 px; the result is saved | A size from a larger screen is lost |
| R-38 | The launch sets the accepted state and the arrangement from a dump in the same update that builds or loads the area, so its own events are absorbed by R-6's first step | — |
| R-39 | Withdrawn (decision 7, revised) | — |

## Known Limits

- **K-1.** Docks and groups resize by mouse only; the Kit's handles take
  no focus (`B/resizable/panel.rs`).
- **K-2.** A refused drop reloads the area, so focus returns to the item
  area and focused Kit buttons in the area lose focus (R-6, fact 5). The
  Kit's drop indicator shows over the item area before the refusal.
- **K-3.** Only a tabbed group's displayed tab is an F6 stop (decision 9).
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
- **K-14.** The quit's write runs on the UI thread and can wait for a
  write already in flight; on a hung file system the quit waits with it.
- **K-15.** A hidden bottom dock keeps a 29 px strip under the item area
  (decision 16; Deferred Work).
- **K-16.** Two windows over the same services share one layout file, and
  the last write wins. The app opens one main window; only tests open
  two.
- **K-17.** A bottom dock split top and bottom by a drag shows only its top
  group's tabs in the closed strip (R-35).
- **K-18.** With Activity hidden, a quit that has to wait shows nothing
  until Activity is opened (decision 7, R-20).
- **K-19.** (Added 2026-11-09, accepted by the owner: "it should be okay,
  contents will probably get revisited".) Activity is as wide as the item
  area, because the bottom dock sits in the centre column (fact 8): about
  550 px at the app's default 1280 px window. Three unwrapped single-line
  rows in `operations.rs`, which this Cycle does not edit, can sit outside
  the visible Activity: the draft-flush entry's Retry, a pending
  repository row's Retry, and the pending controls row's later controls,
  such as Abandon. Activity scrolls vertically only. The rows are fixed
  where `operations.rs` is rebuilt (Cycle 03's Activity, or D6).
- **K-20.** (Added 2026-11-09.) The app being killed or terminated by the
  operating system bypasses `quit_now`, so a change made after the last
  debounced save and before the kill is lost.
- **K-21.** (Added 2026-11-09.) The layout file's 1 MiB cap is checked
  after `Folder::read` has read the file, because it has no size query
  and `src/drafts` is outside the Boundary, so the cap does not bound that
  one read's memory.

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
- **Taking a hidden dock out of the area and rebuilding it on show**
  (revision 2): round 2 found it could double, lose or resurrect panels,
  and loop. The owner chose the Kit's own closing for now (decision 16).
- **`load` for every dock change:** it rebuilds the whole area and drops
  focus and element state (fact 5).
- **Taking hidden panels out of the area:** the panel loses its place in
  its group.
- **Drawing the quit feedback outside any panel:** two homes for the same
  controls, while D6 adds to them in the operation area.
- **`set_locked` to refuse centre drops:** it locks every dock (fact 10).
- **A guard around the shell's own edits:** GPUI delivers events after the
  edit (R-6).
- **Scroll-into-view markers in the operation area:** they would edit the
  files D6 changes (decision 15).

## Risks

| Risk | Control |
| --- | --- |
| The 0.6.6 dock differs from the handoff's names and examples | Facts 1–11 read from the locked source; Task 4b opens with a design checkpoint |
| The region change reaches most desktop tests, and with decision 12 an unlisted break shows only at Task 12's fast tier | E-1 to E-10; R-26 keeps today's click geometry; each task runs the existing tests it touches by name; an unlisted break stops for the owner |
| Panel methods called inside Kit updates read a leased entity | R-1: `visible` and `focus_handle` read only the panel |
| Reconcile and the handler chase each other | R-6 stops on an unchanged dump, and reconcile toggles only docks that differ; P4, P8 and P10 run with closed docks |
| D6 changes the operation area and the quit path at the same time | No edit to `view/operations/` or `model/operations.rs`; the second to merge adapts (Cycle, In flight) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | Handoff dock names absent in 0.6.6 | Settled | Fact 1; amendment 6 | `B/dock/state_convert.rs:8-9`; `C/dock/mod.rs:48-55` |
| Planning research | No drop predicate | Ruling | R-6; Cycle U-2 settled | `B/dock/tab_group.rs:729-741` |
| Planning research | The test window records the title | Settled | Cycle U-1 settled | `G/app/test_context.rs:861-864` |
| Planning research | The context menu has no keyboard path; NativeMenu is native on macOS and Windows | Ruling | R-11 | `C/menu/context_menu.rs:296-300`; `C/native_menu/mod.rs:201-222` |
| Planning research | TitleBar draws Linux controls only with client decorations | Ruling | R-15 | `C/title_bar.rs:267-270`; `G/window.rs:1596-1597` |
| Planning research | A cached panel would not see Workspace changes | Ruling | R-1 | `G/view.rs:484-499`; `C/dock/tab_panel.rs:733` |
| Review round 1 (two reviewers, `bb9b3cc9`) | Source and feasibility: 3 Blocking, 14 Important, 16 Minor. Contract: 2 Blocking, 17 Important, 11 Minor | — | All verified against source and authorities; applied in revision 2 | Reviewer reports, 2026-11-08 |
| Round 1 | Activity at 150 px and no focus reveal | Owner decision | Decision 15 | Owner, 2026-11-08 |
| Round 1 | A failed layout write was never reported; a failed pending read counted nothing; whole-suite runs | Owner decisions | Decisions 13, 14, 12 | Owner, 2026-11-08 |
| Round 1 | R-20 saved the quit reveal; the token test compared colours; New ticket… could create a document; stores live on `Backend`; builders are global; panel methods read a leased Workspace; bookkeeping order; unrecorded departures | Rulings, settled | R-20, E-7, R-8, R-17, R-34, R-1, R-33, amendments 7–10 | Round 1 reports |
| Review round 2 (scoped re-review and adversarial, `fbca53f4`) | Re-review: 1 Blocking, 9 Important, 10 Minor. Adversarial: 4 Blocking, 9 Important, 5 Minor | — | Verified; applied in revision 3 | Reviewer reports, 2026-11-08 |
| Round 2 | Taking a dock out and rebuilding it could resurrect hidden docks on a refused drop, loop, panic on moves from a hidden dock, and save files validate rejects (5 Blocking across both reviews) | Owner decision | Decision 16: the Kit's own closing, keeping its bottom strip for now; R-3 and R-5 rewritten | `B/dock/dock_area.rs:748-770`, `:974-1013` |
| Round 2 | The guard cannot see queued events | Ruling | R-6 idempotent, no guard | `G/app/context.rs:762-776`; `G/app.rs:1170-1177` |
| Round 2 | A return to `Running` without Drafts not saved left the area locked | Ruling, later superseded | R-20 followed `set_quit`; withdrawn with decision 7 revised | `D/view/workspace/lifecycle.rs:101-108`, `:141-147` |
| Round 2 | Override toggles and its active index leaked; "on screen" disagreed with the Kit's fallback | Ruling | Part B and R-4 (the override was later withdrawn) | `B/dock/tab_group.rs:199-208` |
| Round 2 | The immediate quit skipped the quit's write; a write in flight could overwrite a newer one | Ruling | R-19 at `request_quit`, generations | `D/view/workspace/lifecycle.rs:26-58`; `src/drafts/mod.rs:124-137` |
| Round 2 | `note_focus` placement; Cancel's focus target; harness geometry; footer click behind a sheet; the Boundary command; validate's gaps; clamping; two windows | Ruling, settled | R-33, fact 27, R-26, E-6, plan Boundary, Part G, R-37, K-16 | Round 2 reports |
| Review round 3 (scoped re-review and adversarial, `32b82b9c`) | Re-review: 7 Important, 8 Minor. Adversarial: 1 Blocking, 6 Important, 8 Minor. The Kit-closing mechanism held: the handler settles, refusals keep closed docks closed, moves from a closed dock are safe | — | Verified; applied in revision 4 | Reviewer reports, 2026-11-08 |
| Round 3 | A tab click during a quit wait hid Activity and Quit now; a strip edge drag flapped; a second quit recorded the override's own tab; the recorded index went stale | Owner decision | Resolved by removing the quit override (decision 7 revised, below); R-39 withdrawn | `B/dock/tab_group.rs:232-242`; `C/dock/dock.rs:206-215` |
| Round 3 | "No save from the launch" needed a flag | Ruling | R-38 | `G/app.rs:1170-1177` |
| Round 3 | The clamp had no bounds before the first frame, ran on every reconcile, and depended on order | Ruling | R-37: once, after the first frame, both side docks together | `B/dock/dock_area.rs:137` |
| Round 3 | Store generations per Workspace; the hold seam inside the lock | Ruling | R-19; Testing, Harness | `D/test_support/fixtures.rs:99-131` |
| Round 3 | The strip skipped groups whose panels are all off | Ruling | R-35; K-17 | `B/dock/dock_area.rs:1238-1254` |
| Round 3 | Ten test sites resize the window themselves (the re-review found nine; `settings/repositories_tests.rs:650` is the tenth) | Named edit | E-10 | `git grep simulate_resize -- src/desktop` |
| Round 3 | Validation by name, not by `info`; `set_quit` has no window; focus on a closed strip; strip drops | Ruling, settled | Part G table; Part C focus check; R-35 (the lock was later withdrawn) | `B/dock/state_convert.rs:126-185`; `D/view/workspace/lifecycle.rs:141-147` |
| Owner, after round 3 | "It doesn't matter if the panel is shown or not when quitting"; then "take option two: if it's visible the user will see it, if not they're waiting and if they wonder why they can open the activity panel" | Owner decision | Decision 7 revised: a waiting quit changes no layout; the quit override, the lock, its outcomes table and R-39 withdrawn, which also resolves round 3's quit findings; Amendment 11 | Owner, 2026-11-08 |
| Review round 4 (scoped re-review, `d035d90b`) | 4 Important, 10 Minor, none Blocking | — | Verified and applied in revision 5, which had no independent review | Reviewer report, 2026-11-08 |
| Round 4 | A change made during a quit wait was never written; "Show it" focused a hidden Activity; a strip click focused a hidden region; the clamp had no named hook and skipped closed docks; store clones; tiny dock sizes; amendment wording | Ruling, settled | R-19 (`quit_now`), Part F `show_holder`, Part C focus-in check, R-37, Part G | `D/view/workspace/lifecycle.rs:268-281`; `D/view/workspace/requests.rs:225-247` |
| Owner, 2026-11-09 | The built code differs from the approved text in recorded ways (quit write in `quit_now` gated on a pending save; R-37 clamp; Part G extra rows, file cap and size bound; `same_layout` and the three-term Kit-opened test; `RegionPanel`; menu above the toggle; `selected`/`aria_toggled` split); proposed K-19, K-20, K-21 | Owner decision | The amendments were applied on 2026-11-09 with owner approval; K-19 accepted; K-20 and K-21 added | Owner, 2026-11-09; execution ledger, Tasks 4b, 5, 7, 9 and 12 |
