---
title: "Wave 04 Cycle 04 Settings Sheet Design"
date: 2026-11-13
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7APBY77T6T3S9G4SE55Y6FW"
---

# Wave 04 Cycle 04 Settings Sheet Design

## Intent And Approval Boundary

This design covers [Cycle 04](../Cycles/wave-04-cycle-04-settings-sheet.md).
Settings stops being a page drawn in place of the open item and becomes
the design's 720 px sheet on the right. Its left menu has three pages:

- **Repositories:** today's Repositories and Remotes pages, with the Git
  identity form.
- **Sharing & keys:** today's Shared SSH keys page, in `05`'s row layout
  (decision 2), and Trusted servers from today's Hosts page.
- **Checking for updates:** D6's page.

Every existing way into Settings opens the sheet at its page and control,
including those Cycle 03b adds. Opening and closing Settings leaves the
open item, its scroll position and any draft as they were. The Cycle also
takes five items from ticket `01M5F3GVDYTH1Q5V5FCS9YF7V3` that sit in the
code it rewrites (decisions 3 and 12; Part G).

Approval covers the mechanisms, rulings, named test edits and task order.
It does not authorize implementation, merging or closing.
**Implementation waits until Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is
merged to main** (decision 1). Task 0 then merges main and re-checks every
citation below against it (U-5). After approval, a note goes on
`01M5F3GVDYTH1Q5V5FCS9YF7V3` saying which items this Cycle takes. That is
the planner's step 9 work, not a Document Amendment.

Status: **draft**, revision 3.

- Revision 2 applied review round 1 (source and feasibility: 0 Blocking,
  10 Important, 9 Minor; contract and consistency: 2 Blocking, 16
  Important, 17 Minor) and decisions 12 and 13.
- Revision 3 applies review round 2 (scoped re-review: 0 Blocking, 5
  Important, 18 Minor; adversarial review of the close and focus
  mechanics: 1 Blocking, 7 Important, 8 Minor).
- Most of round 2 sits in where focus goes, so that contract is now a
  table of required outcomes (A6), which Task 2 writes test-first.
- **Revision 3 has not been independently reviewed.**

## Conventions

- `D/` is `src/desktop/`.
- `K/` is `gpui-component-0.6.6/src` and `KB/` is `gpui-base-0.6.6/src`,
  both under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.
  `gpui_kit::component` re-exports gpui-component
  (`gpui-kit-0.6.6/src/lib.rs:143`).
- Citations are at main `ba95e1b7`. This branch's `0b7a8f3d` changes only
  `ticket.md`.
- Cycle 03b changes `D/view/operations/`, the footer, the keymap,
  `D/view/workspace/focus.rs` and `render.rs`. Citations of 03b are to its
  approved design on its branch ("03b design"). Task 0 re-maps every
  citation in those files after 03b merges.
- "Fact n", "W-n" (a way in), "P-n" (a required outcome), "E-n", "R-n",
  "K-n" and "U-n" refer to this design. "Decision n" and "A-n" refer to
  the Cycle document.

## Baseline Facts At `ba95e1b7`

### Settings today

1. **The host.** `SettingsHost` (`D/view/settings/mod.rs:90-109`) holds:
   - `services` and a `scroll: ContentScroll`;
   - `shown: Shown`;
   - one focus handle per page-list entry (`entries`);
   - `close`, the wrapper around "Back to tabs";
   - five lazily made page entities (`repositories`, `remotes`, `keys`,
     `hosts`, `polling`) and the polling subscription;
   - `selection: Option<PathBuf>`.

   Its module doc (`:1-18`) records D1's ruling R-B4: Settings is shown
   "in the main window's content region in place of the tabs".
2. **Pages.** `SettingsPage` (`D/model/stamp.rs:18-26`) is `Repositories,
   Remotes, Keys, Hosts, Polling`.
   - `PAGES` (`mod.rs:55-61`) orders them.
   - `page_title` (`mod.rs:66-70`) gives "Repositories", "Remotes",
     "Shared SSH keys", "Hosts" and "Checking for updates".
   - `ViewScope::Settings(SettingsPage)` is one result-generation scope
     per page (`stamp.rs:1-6`).
3. **What tests read.** `Shown` (`mod.rs:76-79`) is `Tabs |
   Settings(SettingsPage)`. Tests read it through the test-only
   `Workspace::settings_shown` (`D/view/workspace/content.rs:745-747`),
   called from 21 test files, some through a `Page` alias.
   `SettingsEvent` (`mod.rs:82-89`) is `Closed | ShowActivity`.
4. **The host's API:**
   - `open(page)` (`mod.rs:145-169`) sets `shown` and calls `opened(…)` on
     Remotes, Keys, Hosts and Polling. `RepositoriesPage` has no
     `opened`; only `remotes.rs:281`, `keys.rs:275`, `hosts.rs:313` and
     `polling.rs:159` do. Its registrations are read by
     `load_registrations` (`repositories/manage.rs:230`).
   - `open_remotes_on` (`:171-183`), `set_selection` (`:185-187`),
     `forget_passphrase` (`:191-198`), `hide` (`:202-207`).
   - `first_focus` (`:211-218`) returns the shown page's entry.
   - `set_up_sharing` (`:299-305`) focuses the first entry.
   - `check_host` (`:309-320`), `retry_generation` (`:324-335`),
     `inspect` (`:354-364`), `identity` (`:367-376`), `back`
     (`:378-381`).
5. **The page list** (`mod.rs:384-428`) is an `h_flex().flex_wrap()` of
   plain divs:
   - each has `Role::Tab`, `aria_selected` and the id
     `("settings-page", ix)`;
   - the shown entry carries a "Shown" tag (`settings-page-shown`).

   It does not use the Kit's `Settings` component, by ruling R-T12a (D1
   comment
   `.manyhands/comments/01M4CC0VMY5RBMP70QVBXWXM6A/01M5CRVEAP42DDKYBC63A66TRV.md:80`).
6. **Render** (`mod.rs:431-474`) first calls `forget_passphrase`. It then
   draws the heading "Settings", the "Back to tabs" button
   (`settings-close`, `settings-close-button`), the page list and the
   page.
7. **Wrapped controls.** `wrapped(id, label, focus, scroll, control)`
   (`mod.rs:482-497`) puts a control in a focus-tracking wrapper with a
   scroll marker. A control taking focus below the fold is then revealed.
8. **Owner.** The Workspace owns the host:
   - `ContentArea.settings: Entity<SettingsHost>` (`D/view/workspace.rs:71`)
     is built at `:285-289` with `tabs.scroll().clone()`;
   - `ContentArea.sheet` is typed at `:91`;
   - the subscription at `:290-296` focuses `Region::Content` on
     `Closed` and `Region::Operations` on `ShowActivity`.
9. **The content page.** `ContentPage { Tabs, Settings, Drafts }`
   (`content.rs:39-43`) is chosen by `page()` (`:46-54`), Settings first.
   - `show_tabs` (`:56-59`) hides Settings and Drafts.
   - `content_body` (`:662-700`) draws only the `SettingsHost` while
     Settings is shown. On the Tabs and Drafts branches it calls
     `forget_passphrase`.
   - `shown_item` (`:630-635`) is `None` while Settings is shown.
   - `focus_content` (`:605-621`) sends `Region::Content` focus into
     Settings while it is shown.
10. **Opening.**
    - `open_settings(page)` (`content.rs:194-202`) uses the Repositories
      panel's selection.
    - `open_settings_on(page, root)` (`:206-220`) also calls
      `manage_root(root, false)` for Repositories.
    - Both go through `open_settings_at` (`:222-235`), which hides
      Drafts, sets the selection, opens the page and focuses
      `Region::Content`. `inspect_in_settings` (`:237-248`) focuses
      `Region::Content` too.
    - `identity_in_settings` (`:250-261`), `check_host_in_settings`
      (`:263-275`) and `retry_generation_in_settings` (`:277-288`) only
      set `last_region`, and the page focuses a field:
      - `manage_root(…, true)` focuses `identity-name`
        (`repositories/manage.rs:322-324`);
      - `ask_again` focuses `key-generate-passphrase`
        (`keys/commands.rs:111`);
      - the Hosts check focuses `hosts-check`.
    - `open_remotes` (`D/view/workspace/requests.rs:285-297`) and the
      private `open_hosts` (`:217-225`) open Settings directly.

    That is eight entry functions, plus the save banner's deferred
    `settings_for` (`content.rs:115-120`, `:145-146`).
11. **Leaving.** "Back to tabs" runs `back`, which hides and emits
    `Closed`. `open_item` (`content.rs:336-342`), `show_tab`, ctrl-tab and
    `open_drafts` (`:77-91`) also hide Settings. Nothing binds Escape for
    Settings.
12. **What survives today.**
    - Tabs, the shown tab, each `ItemPane` (its edit session and draft)
      and its own scroll region are untouched by opening Settings
      (`D/view/settings/mod_tests.rs:73-118`;
      `D/view/item/frame_tests.rs:148-178`).
    - The Keys page's masked inputs are emptied whenever Keys is not the
      shown page (fact 9; `mod.rs:191-198`).
13. **One scroll.** Settings, the Drafts page and the empty pane scroll
    the content region's column, `tabs.scroll()` (`content.rs:625-627`;
    `D/view/scroll.rs:1-22`).
    - An item tab scrolls a region made by `ContentScroll::region()`
      (`scroll.rs:59-68`), which shares the column's focus-reveal state.
    - The column's `note_focus` runs once a frame in `item_area_region`
      (`D/view/workspace/render.rs:121-122`).
    - This is the shared offset that `01M5F3GVDYTH1Q5V5FCS9YF7V3`
      reports.
14. **Page keys.** PageUp, PageDown, Home and End are bound in the
    `Content` context (`D/keys.rs:308-311`). The Workspace handles them
    through `unless_dialog` (`render.rs:286-297`), scrolling
    `scroll_target` (`content.rs:639-644`): the shown item's region, else
    the column.

### The pages

15. **Repositories** (`D/view/settings/repositories.rs`,
    `repositories/manage.rs`). Render order (`repositories.rs:776-794`):
    - the managed section: the registrations, each with Manage
      `("managed-choose-button", ix)`, then the identity form and Remove
      for the managed root;
    - the Add form;
    - the Create form.

    `manage_root(root, focus)` (`manage.rs:308-327`) sets the managed
    root, **empties the identity inputs every time**, reads the identity
    and focuses `identity-name` when `focus` is set. The identity read's
    prefill (`apply_prefill`, `:299-304`) sets both fields
    unconditionally. `inspect_root` (`repositories.rs:436`) fills and
    checks `add-path` without moving focus. Dialogs go through
    `view::confirm::confirm`.
16. **Remotes** (`D/view/settings/remotes.rs`):
    - its own registrations list (`:608-640`), each row with Choose
      (`choose_root`, `:486`), read at `new` (`:238`) and again at
      `opened` (`:282`);
    - the chosen repository's remotes;
    - the Add form (`add_form`, `:793-822`), drawn whenever a root is
      chosen, with `remote-add-name` and `remote-add-url`.

    `opened(selection)` (`:281-289`) switches root only when it has none;
    `open_on(root)` (`:292-298`) always switches. Remove goes through
    `confirm(RemoteRemove)`. Copy is in `:73-99`, and
    `"Publication remote: {name}"` is at `:114`.
17. **Shared SSH keys** (`D/view/settings/keys.rs`, `keys/render.rs`,
    `keys/commands.rs`, `keys/reads.rs`).
    - Each key row (`render.rs:91-163`) shows a kind ("Generated by
      Manyhands" or "Imported"), Select, Unregister, and Delete key on
      generated keys only.
    - "Clear selection" sits under the list (`render.rs:165-200`).
    - Below are the inline Generate form (label, passphrase,
      confirmation) and Import form (label, private and public file).
    - Text fields are 480 px wide (`keys/render.rs:87`; also
      `remotes.rs:584`, `repositories.rs:616`).
    - Delete's focus fallback is keyed by row index
      (`01M5F3GVDYTH1Q5V5FCS9YF7V3`, Keys page).
    - Copy is in `keys.rs:37-74`.
18. **Hosts** (`D/view/settings/hosts.rs`).
    - Render (`:825-835`): the pin list ("Host pins"), then Check host
      for the selected repository.
    - There is no repository picker: `opened(selection)` (`:313-320`)
      works on the host's `selection`.
    - `check_host(root, authority)` (`:433-449`) returns early under a
      dialog, else chooses the root and starts a check with focus on
      `hosts-check`. `refocus_gone` (`:576`) moves focus back to Check
      host.
    - Copy is in `hosts.rs:67-86`.
19. **Checking for updates** (`D/view/settings/polling.rs`) emits
    `PollingEvent::{SetUpSharing, Attend, ShowActivity}` (`:85-92`).
    - A row with a pending operation draws "Show in Activity"
      (`SHOW_IN_ACTIVITY`, `:76`; button at `:418-427`).
    - The host answers `Attend` in `D/view/settings/attention.rs:11`:
      Keys, `check_host`, Hosts, or `set_up_sharing`.
20. **Handlers check dialogs, not sheets.** Every Settings page handler
    tests `window.has_active_dialog`, which counts dialogs only
    (`K/window_ext.rs:161`).

### Ways in

21. **Calls into Settings** outside tests are listed in Part D with their
    file and line: 33 triggers at `ba95e1b7`, through the eight entry
    functions of fact 10, the save banner's `settings_for` and the
    Settings pages' own routes. They include:
    - the Repositories panel's own Settings button (`sidebar-settings`,
      `D/view/repositories/body.rs:129-141`);
    - the repository menu's `RowAction::Attend`
      (`D/view/workspace/repository_menu.rs:243`), which calls
      `Workspace::attend` (`D/view/operations/attention.rs:119-152`).
22. **Routes from a dialog close the dialog first.**
    - The lifecycle dialog's `in_workspace`
      (`D/view/lifecycle/actions.rs:126-140`) calls `close`
      (`D/view/lifecycle.rs:504-507`: `close_dialog`, then focus its
      `origin`) before calling the Workspace.
    - Item routes go through `in_window` (`requests.rs:177-186`).
    - The save banner's Open Settings goes through `settings_for` and
      opens on the next frame.
23. **The footer.** `Control::Settings` (`D/view/footer.rs:77`, id
    `footer-settings` at `:92`, label `SETTINGS` at `:44`, tooltip action
    `OpenSettings` at `:327`).
    - `press` (`D/view/workspace/footer.rs:161-178`) returns early while a
      dialog or sheet is open.
    - It then sends Settings to `open_settings(Repositories)` and the
      "Checked for updates" item to `open_settings(Polling)`.
24. **The key.** `KeyBinding::new("secondary-,", OpenSettings, None)`
    (`D/keys.rs:281`), handled at `render.rs:262-264` inside
    `unless_dialog`.
25. **Cycle 03b's routes** (03b design, on its branch):
    - **Activity rows and the popover** run today's `attend` and
      `open_route` handlers through
      `Workspace::choose_activity(&ActivityMenuChoice, origin, …)` (03b
      design `:1044`, `:2269`). That covers the button, the row menu and
      Enter.
    - **Repository-state rows** (S4) also offer Settings… and Set up
      sharing… (03b design `:1936`, `:1941`).
    - **Origins.** A row's origin is the Activity list's focus handle; the
      row counts as the invoking control (03b decision 23, design C3,
      `:2218-2226`). A popover action's origin is the footer item. When
      that item has hidden, `keep_focus_in_footer` moves focus to "E1's
      control" in render (03b R-49). A lost popover focus goes to the
      footer item, else E1's control (03b R-50).
    - **The popover** closes when a dialog or sheet opens (03b design
      `:2408`).

### The sheet layer, R-60 and focus

26. **Where the sheet is drawn.** `item_area_region`
    (`render.rs:117-152`) appends `Root::render_sheet_layer` inside the
    `Region::Content` element, deferred.
    - Its comment (`:130-135`) says the sheet's focus "counts as the
      region's" and its keys pass the region's context first.
    - A deferred draw keeps its parent dispatch node
      (`gpui-pre-0.3.6/src/window.rs:3739`, `:3782`, `:4338`), and
      `contains_focused` reads the rendered frame's dispatch tree
      (`:504-520`). So focus in the sheet is `Region::Content`, and keys
      bound in the `Content` context reach elements in the sheet before
      the Workspace's handlers. (U-1 of revision 1, settled by review.)
27. **R-60** (Wave 03 D3's ruling, kept by Cycle 01).
    - `modal_open` is `has_active_dialog || has_active_sheet`
      (`render.rs:155-158`). `unless_dialog` (`:160-171`) drops every
      Workspace action under it except Quit and Quit now.
    - The footer, the dock menu and the toolbar check it too
      (`footer.rs:167`; `D/view/dock/header/toolbar.rs:209`).
    - The test `no_workspace_key_acts_behind_the_sheet`
      (`D/view/item/sheet_tests.rs:670-741`) asserts that `secondary-,`
      behind the details sheet leaves `Shown::Tabs`.
28. **The details sheet.**
    - `open_details` (`D/view/item/sheet.rs:145-163`) focuses the chip
      first, because Root returns focus on close to what held it when the
      sheet opened. It then calls `window.open_sheet(…)`, then focuses
      the field.
    - Its body is its own absolute `inset_0` scroll over the Kit body,
      with its own `ScrollHandle` and scrollbar (`sheet.rs:251-275`). It
      reveals focused controls through a `scroll.region()`
      (`D/view/item.rs:313`; `item/sheet.rs:227`).
    - Its title is 15 px semibold (`TITLE_SIZE`, `item/sheet.rs:82`).
29. **Who owns the sheet.** `ContentArea.sheet:
    Option<WeakEntity<ItemPane>>` (`workspace.rs:91`).
    - `Workspace::sync_sheet` (`content.rs:307-334`), called on every
      render (`render.rs:179`), **closes any active sheet not owned by the
      shown item pane**.
    - It then opens a requested details sheet (`content.details`) while
      no dialog is open (`:321-327`).
    - `open_item` clears the owner (`content.rs:340`). It can run while
      Settings is shown, from an async landing such as the filter's ID
      read (`D/view/workspace/find.rs:104`).
    - `wait_to_quit` (`D/view/workspace/lifecycle.rs:226-238`) clears the
      owner, closes dialogs and any sheet, then focuses Quit now's
      `quit_focus`.
30. **Async refocus.** These move focus into the content region on a
    landing:
    - `apply_review`'s `refocus` (`content.rs:135-137`), not guarded
      against a sheet, unlike `landed_refocus`;
    - the `close_with_draft` landing (`:515`);
    - `open_created` (`:166-168`);
    - the retire flow's `show_item_tab` (`D/view/operations/retire.rs:208-222`).
31. **Focus lost.** `focus_lost` (`D/view/workspace/focus.rs:275-295`):
    - with a modal open, it refocuses `window.focus_lost_restore_target`;
    - otherwise it moves focus to the nearest displayed region.
    - 03b adds its popover rule (fact 25).
32. **Details sheet edits are drafts.**
    - Each field change goes through `change_fields` into the shared
      drafts registry (`D/view/item/sheet/fields.rs:163-198`;
      `D/view/item/edit/feed.rs:30-63`).
    - The sheet's builder holds only a `WeakEntity<ItemPane>`, so closing
      the sheet drops no edit.
    - `each_named_control_edits_its_field_and_marks_the_tab_dirty`
      (`D/view/item/sheet/fields_tests.rs:255-339`) closes with Escape
      and finds every field in the draft.
    - An open relationship picker's typed filter is not draft content.
      It is dropped when the details sheet closes, by any route
      (`picker_follows_sheet`, `D/view/item/relations_edit/picker.rs:266-275`).

### GPUI Kit 0.6.6

33. **Calls.** `open_sheet`, `open_sheet_at(placement)`,
    `has_active_sheet` and `close_sheet` (`K/window_ext.rs:14-27`,
    `:111-138`). Root keeps one slot, `active_sheet:
    Option<ActiveSheet>` (`K/root.rs:40`).
34. **Replacing and closing.**
    - `open_sheet_at` (`K/root.rs:379-406`) keeps the replaced sheet's
      `previous_focused_handle`, or else the focused handle, then focuses
      a new Root-owned handle.
    - `close_sheet` (`:410-423`) focuses that previous handle if it still
      exists, then clears the slot. It does not look at dialogs.
35. **Rebuilt each frame.** `render_sheet_layer` calls the stored builder
    on every render (`K/root.rs:215-240`), so a sheet's state must live in
    entities its builder captures.
36. **The builder** (`K/sheet.rs:59-119`):
    - `title(impl IntoElement)` and `footer`;
    - `size` (default 350 px) and `resizable` (stored, unused);
    - `overlay` and `overlay_closable` (both default true): a left
      mouse-down on the overlay closes through `request_close`, then
      `on_close`;
    - `on_close`.

    The header always draws a ghost close button that dispatches `Cancel`
    (`K/sheet.rs:200-208`). There is no description builder. The top
    margin is the theme's `sheet.margin_top`, by default the Kit title
    bar's 34 px (`K/sheet.rs:32-37`, `:144`). The app's title bar is
    38 px (`D/view/shell.rs:18`).
37. **Escape and the trap.**
    - `escape` is bound to `Cancel` in the `Sheet` context
      (`KB/sheet.rs:23-25`).
    - The host traps focus (`KB/sheet.rs:133-134`; Root's Tab handlers,
      `K/root.rs:492-560`). It handles `Cancel` by propagating, then
      closing (`KB/sheet.rs:135-138`).
    - Closing runs `request_close` (`window.close_sheet`), then
      `on_close`. A direct `window.close_sheet` does not run `on_close`
      (`KB/sheet.rs:17-21`).
    - No code under `D/` and no Kit dock handler takes `Cancel`.
38. **Dialogs over a sheet.**
    - `open_dialog` does not check for a sheet. It records the focused
      handle and focuses its own (`K/root.rs:297-323`).
    - Dialogs are deferred at priority 10 and above (`KB/dialog.rs:541`);
      the app's sheet layer is at 0 (fact 26).
    - `close_dialog` focuses the dialog's previous handle
      (`K/root.rs:325-339`).
    - So a Settings page's confirmation opens above the sheet and gives
      focus back inside it.
39. **Radio is unusable here.**
    - `Radio` (`K/radio.rs:19-130`) keeps its focus handle as keyed state
      made during render (`:174-177`) and passes it to the base radio, so
      a caller cannot reach or focus it.
    - A checked radio has no click action.
    - The accessibility builders are `role(accesskit::Role)` and
      `aria_toggled` (`gpui-pre-0.3.6/src/elements/div.rs:1304`,
      `:1400`).

### Data and authorities

40. **What the library gives a key row.** `KeyDto`
    (`src/repository/read/dto.rs:247-258`): `id`, `label`, `ownership`,
    `selected`, `fingerprint`, the source and metadata states and the file
    paths. It has no passphrase flag, no creation date and no algorithm.
    `HostPinDto` (`:294-300`): `host`, `port`, `pinned_algorithm`,
    `pinned_fingerprint`, `reapproval_required`; no date.
41. **The design.** `05` "Settings sheet"
    (`docs/design/05-sheets-and-dialogs.md:40-66`):
    - 720 px, opened from the footer gear and Ctrl ,;
    - a 214 px left nav whose designed pages are Sharing & keys, Checking
      for updates and Appearance;
    - the key list, the info box and Trusted servers (`:46-59`);
    - Checking for updates, with a global "Pause all" switch (`:61-66`).

    The common rules (`:3-5`) and frame (`:7-12`) are: Escape closes
    "unless an operation is running"; a header with title, description
    and close; a scrolling body; a footer.

    Frames `1e` and `1f` draw Settings with no footer and with the header
    description "Applies to this computer only."
    (`docs/design/mockups/source/Workspace.dc.html:1123`). Every page but
    keys, polling and appearance is "not mocked this round" (`:475`).
42. **Vocabulary.** `06` (`docs/design/06-language-and-states.md:8`,
    `:15`, `:24`, `:25`) maps:
    - publication remote to **shared copy**;
    - SSH key to **key** / "Key used for sharing";
    - known host pin to **Trusted servers**;
    - background poll to **Checking for updates**.

    It asks for `…` on any action that opens a confirmation or sheet
    (`:36`).
43. **Passphrases.** The credential RFC keeps a protected key's passphrase
    "only in that provider's memory for the remaining session" and asks
    for it on first use in a session
    (`docs/RFC/authentication-and-credential-handling.md:145`, `:275`).
    So `05`'s info box, "asks for it once each time you open the app and
    never stores it", is true of today's handling.
44. **No Pause all.** D6 decision 8 rejected a global pause switch and
    recorded the handoff's as a Known Limit
    (`docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md:428-430`).
45. **Key settings behaviour.** The desktop IA RFC requires generation,
    import, selection, clearing and unregistration. Generated-file
    deletion is "a separate confirmed action", and there is no
    private-key display or copy
    (`docs/RFC/desktop-information-architecture-and-editor.md:321-328`).
    Key management must work without a pointer (`:343-345`).

### Focus restores

46. **The Kit's deferred restore.** `defer_close_dialog`
    (`K/root.rs:341-363`) closes a dialog at once, then focuses its
    previous handle 0.25 s later if the dialog count is unchanged. It does
    not look at sheets. It runs when a dialog's `on_ok` returns true:
    Drafts' Discard… (`D/view/drafts/actions.rs:248`) and quit's Not saved
    (`D/view/quit.rs:259-262`). The app's confirmation and passphrase
    dialogs close at once.
47. **`focus_lost`'s target.** GPUI fires `focus_lost` when focus moves
    from something to nothing (`gpui-pre-0.3.6/src/window.rs:3360-3369`).
    `focus_lost_restore_target` is the nearest focusable ancestor on the
    previous frame's focus path (`:2291-2300`). The Workspace's
    `focus_lost` (`D/view/workspace/focus.rs:275-295`) focuses that target
    while a dialog or sheet is open, else the nearest displayed region.
    The quit path's EditsNotKept ask (`D/view/workspace/lifecycle.rs:86-99`)
    opens a dialog before `wait_to_quit` runs.

## Part A: The Sheet

### A1. Who owns the sheet

`ContentArea.sheet` (`workspace.rs:91`) becomes:

```rust
/// The sheet on Root's one sheet layer, and who keeps it open (fact 29).
pub(super) enum SheetOwner {
    /// The shown item's details sheet, as today.
    Details(WeakEntity<ItemPane>),
    /// Settings (Part B).
    Settings,
}
```

`sync_sheet` changes as follows (R-2, R-33):

1. **Owner `Settings`, and a sheet is active:** kept, whatever item is
   shown.
2. **Owner `Settings`, and no sheet is active:** the Kit closed Settings.
   `settings_closed` runs (A3).
3. **The host is open, but the owner is not `Settings`:** a defensive
   case. `settings_closed` runs quietly.
4. **`Details(pane)`:** as today's `Some(pane)`.
5. **A requested details sheet** (`content.details`): as today. It cannot
   be requested while Settings is open, since its chip is behind the
   sheet, and `show_settings` drops one requested before it (A2 step 4;
   R-51).

Owner changes:

- `open_item` clears the owner only when it is `Details` (fact 29).
- `wait_to_quit`, and every quit path before it asks anything (EditsNotKept,
  NotSaved), closes Settings quietly first (A3; R-52).

The details sheet and Settings never coexist: Root has one slot (fact 33;
Wave decision 6).

### A2. One way in

Every way in calls one function:

```rust
/// Where Settings opens and what it does on arrival (Part D).
pub(crate) enum SettingsRoute {
    /// A page, focused on its menu entry.
    Page(SettingsPage),
    /// The Repositories page, on its Add folder field (W-5, W-6).
    AddRepository,
    /// The Repositories page managing `root` (the repository menu's Settings…).
    Manage(PathBuf),
    /// The Repositories page, checking `root` in its Add folder (repo.inspect).
    Inspect(PathBuf),
    /// The Repositories page's identity form for `root`.
    Identity(PathBuf),
    /// Repositories › Sharing for `root`, on the add-remote Name field.
    Remotes(PathBuf),
    /// Sharing & keys, on the key list (R-22).
    Keys,
    /// Sharing & keys › Trusted servers for `root`, on Check host.
    Hosts(PathBuf),
    /// As `Hosts`, checking `authority` (R-T16a).
    CheckHost(PathBuf, SshAuthority),
    /// Sharing & keys, asking again for request `id`'s passphrase (R-T15a).
    RetryGeneration(DesktopRequestId),
}

impl Workspace {
    /// Opens Settings at `route` (Part D). `origin` is where focus returns
    /// on close (A2, "The origin").
    pub(in crate::desktop::view) fn show_settings(
        &mut self,
        route: SettingsRoute,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );
}

impl SettingsHost {
    /// Switches to the route's page and applies it: chooses the
    /// repository, shows a form, starts a check. Moves no focus. Returns
    /// the control to focus, only when the page draws it in the next
    /// frame; `None` means the page's menu entry (Part D).
    pub(crate) fn arrive(
        &mut self,
        route: &SettingsRoute,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<FocusHandle>;
}
```

`show_settings` and `settings_closed` live in a new
`D/view/workspace/settings_sheet.rs` (`workspace.rs` is 488 lines).
`workspace.rs` changes only `ContentArea.sheet`'s type, the subscription,
two fields (`settings_origin`, `settings_return`) and the new module's
declaration.

**Wrappers** (R-5). Today's entry functions stay as one-line wrappers that
build a route and pass `None`: `open_settings`, `open_settings_on`,
`inspect_in_settings`, `identity_in_settings`, `check_host_in_settings`,
`retry_generation_in_settings`, `open_remotes` and `open_hosts`. Their
callers change only where Part D says. W-5 and W-6 call
`show_settings(AddRepository, None, …)` directly.

The page methods that focus a field today stop focusing when called from
`arrive`, and give back the handle instead (fact 10): `manage_root`,
`ask_again`, the Hosts `check_host` and `set_up_sharing`.

**The origin** (R-6, R-43). The control focus returns to on close is the
first of:

1. the `origin` argument;
2. the Workspace's pending origin, `settings_origin`. Cycle 03b's
   `choose_activity` sets it to its own `origin` (the Activity list's
   handle, or the footer item for a popover row) for the length of one
   handler call, and clears it after. So `attend`, `row_action` and
   `open_route` keep their signatures;
3. when focus is in one of the Workspace's own layers (03b's popover, a
   row menu, a dock menu), the handle that layer gives focus back to (its
   `restore`), and the layer closes;
4. the focused handle.

**Open** means the owner is `Settings` and a sheet is active (R-36).
`show_settings`:

1. **Refuses** while a dialog is open, or while the app is quitting
   (R-9). Nothing happens.
2. **If the owner is `Settings` but no sheet is active** (the Kit closed
   it, and `sync_sheet` has not yet seen it), runs `settings_closed`
   quietly first.
3. **If Settings is open**, calls `arrive` and focuses its target, else
   the entry. The origin recorded at opening is kept.
4. **If a sheet is active and the owner is not `Settings`** (the details
   sheet), sets the owner to `None` and calls `window.close_sheet`. That
   gives focus back to the sheet's chip (facts 28, 34). The sheet's edits
   are already in the draft (fact 32). A requested details sheet
   (`content.details`) is dropped (R-51).
5. **Records the origin** (above), and the region it is in
   (`focused_region`; `None` for the footer) as `settings_return`'s
   region (R-45). If the origin is not the focused handle, focuses it,
   so that the Kit's own record (fact 34) is the origin too (R-44).
6. **Sets the chosen repository** (R-14; C1).
7. **Opens the sheet.**
   - Sets the owner to `SheetOwner::Settings` and the host's `open` flag
     (A4).
   - Calls `window.open_sheet(cx, builder)`. The builder captures the
     `Entity<SettingsHost>` and draws Part B's frame.
   - Calls `arrive(route)` and focuses the returned handle, else the
     page's entry. That focus comes after `open_sheet` has focused Root's
     own handle (fact 34), as `open_details` does (fact 28).

The Drafts page is not hidden (R-31): whatever the item area shows stays
under the sheet.

**Routes inside the sheet** (W-31, W-32). The Checking for updates page's
Set up sharing… and attention actions are handled by `SettingsHost`
itself. It calls `arrive` and focuses the result, keeping the recorded
origin.

### A3. Closing

Settings closes by any of:

- Escape;
- the header's close button;
- a click on the scrim (decision 13; fact 36);
- "Show in Activity" (Part C3);
- quitting.

The Kit's `on_close` does not run on every path (fact 37), so the close is
detected in `sync_sheet` (A1 case 2). Then `settings_closed(quiet)`:

1. sets the owner to `None` and clears the host's `open` flag (A4);
2. hides the host (`Shown::Tabs`). That empties a typed passphrase (B3)
   and a pending Add (A4);
3. unless `quiet`, or a dialog or sheet is now open, focuses the origin
   and sets `settings_return` (below).

A2 step 5 focused the origin before `open_sheet`, so the Kit's own
`close_sheet` already gives focus back to it on every close the Kit makes
(fact 34). Step 3 does the same for closes the Kit did not make.

**Quietly.** Steps 1 and 2 only:

- every quit path, which then focuses Quit now or asks (A1; R-52);
- Show in Activity, which then focuses Activity (C3);
- A2 step 2 and A1 case 3.

**An origin that is no longer drawn** (R-45). GPUI then fires
`focus_lost` in the same draw. Its restore target comes from the previous
frame's focus path, which ran through the sheet, inside the Content region
(facts 26, 47). Left alone, focus would land in the item area. Instead:

- `settings_return` holds the origin's region from A2 step 5. The
  Workspace's `focus_lost` takes it, if it is set, and focuses that
  region (`focus_region`). It is cleared at the end of the next frame
  either way.
- A footer origin has no region. `keep_focus_in_footer` (03b R-49) runs
  in the same render, after `sync_sheet` (`render.rs:179`, then
  `footer.rs:51`). It moves focus to E1's control when the footer item
  has hidden. Cycle 03b's R-50 does not apply, since no origin is its
  `attention_focus` (A2, "The origin", case 3).
- When the origin's handle was dropped, its region is focused, else
  `Region::Content`.

**The sheet keeps focus** (R-46). On each render, the Workspace checks.
While Settings is open and no dialog is open, if focus is not inside the
sheet layer, it focuses the shown page's entry.

- "Inside the sheet layer" means the focus is inside the div that
  `item_area_region` wraps around Root's sheet layer (fact 26), which
  tracks a focus handle of its own (`sheet_layer`). Deferred draws keep
  their parent dispatch node, so `sheet_layer.contains_focused` holds
  for any focus in the sheet, the Kit's close button included.
- This catches:
  - the Kit's deferred focus restore after a confirmed dialog (fact 46);
  - a route target that is not drawn (facts 46, 47);
  - any landing that slips past A4.
- U-6 checks that a focus change brings a render.

### A4. Escape, and work that finishes after a close

- **Escape always closes Settings**, also while a request it started is
  running (decision 6; R-10). This departs from `05`'s "unless an
  operation is running" (fact 41), and Document Amendments 1 and 2 record
  it.
- **Nothing is cancelled.** The request finishes on the worker and is
  reported in Activity. The page shows its result when Settings next opens
  on the same repository, since the page entities are kept and `opened`
  reads again.
- **Settings knows whether it is open** (R-47). `SettingsHost` holds
  `open: Rc<Cell<bool>>`, set by `show_settings` and `settings_closed`,
  and gives each page a clone when it makes it.
- **While it is false, a landing moves no focus and opens nothing:**
  - Repositories' `resume_submit` (`repositories.rs:418-427`,
    `:461-478`). A folder check that lands after the close opens no
    confirmation and moves no focus. `submit_pending` is cleared when the
    host hides and by `inspect_root`, so a later Inspect cannot open the
    Add confirmation for a folder nobody pressed Add for (R-48);
  - Hosts' `refocus_gone` (`hosts.rs:575-586`);
  - the focus by key ID after a removal (C2.1; P24);
  - R-22's key-list retarget.
- **`Cancel` goes nowhere else.** The Kit propagates `Cancel` after
  closing (fact 37), and nothing in the app handles it. A Kit input,
  select or dialog inside the sheet that handles Escape first keeps doing
  so.

### A5. The guard (R-60), the key and focus behind the sheet

- **R-60.** Settings is a sheet, so `modal_open` (fact 27) holds while it
  is open. No workspace action, footer item, toolbar button or dock menu
  acts behind it.
- **Ctrl+, (`OpenSettings`).** Its handler leaves `unless_dialog` for a
  guard of its own:
  - a dialog open: nothing (today);
  - Settings open: nothing (decision 6);
  - another sheet active (the details sheet): Settings opens, closing it
    first (Wave decision 6; A2 step 4);
  - otherwise: `SettingsRoute::Page(Repositories)`.

  This is the one exception to R-60 (contract 7).
- **The footer** cannot be pressed while any sheet is open (fact 23; its
  scrim takes the click). So Settings is never opened from the footer over
  the details sheet: Ctrl+, is the way.
- **The page keys** are Settings' own while it is open (B2).
- **Nothing pulls focus behind the sheet** (R-34):
  - while Settings is open, `focus_content` does nothing;
  - `apply_review`'s `refocus` is dropped while the owner is `Settings`,
    as `landed_refocus` is under any sheet (`content.rs:139-141`); under
    the details sheet it behaves as today;
  - the other landings of fact 30 go through `focus_content`;
  - R-46 catches the rest.
- **Quitting.** `show_settings` refuses while quitting (R-29).

### A6. Where Focus Goes: Required Outcomes

Focus has been the contract's most-reviewed part, so its outcomes are a
table. Task 2 writes them test-first, before the mechanism. A row the
implementer cannot express against the code goes back to the orchestrator
and the owner.

| F | When | Focus goes to | Test |
| --- | --- | --- | --- |
| F1 | Settings opens on a route naming a field the page draws | That field | P4 |
| F2 | Settings opens on a route naming no field, or a field not drawn | The page's menu entry | P4 |
| F3 | Settings opens over the details sheet; later it closes | F1 or F2; on close, the chip | P3 |
| F4 | Settings closes (Escape, close button, scrim) and the origin is drawn | The origin | P5 |
| F5 | F4, the origin being an Activity row (button, menu or Enter) | The Activity list, the row still highlighted | P5 |
| F6 | F4, the origin being a popover row, the footer item still shown | The footer item | P5 |
| F7 | F6, the footer item hidden by then | E1's control (03b R-49) | P5 |
| F8 | F4, the origin being 03b's popover itself or a row menu (Ctrl+, pressed there) | The control that layer returns to | P5 |
| F9 | Settings closes, the origin no longer drawn | The origin's region | P5 |
| F10 | Settings closes, the origin's handle dropped | Its region, else the content region | P5 |
| F11 | Show in Activity | The Activity list | P14 |
| F12 | Quitting with Activity displayed | Quit now | P6 |
| F13 | Quitting with Activity hidden | Where quitting puts it today, not inside a closed sheet | P6 |
| F14 | A dialog opened from Settings closes | Back inside the sheet, on its origin | P23 |
| F15 | The Kit's deferred restore, or any landing, puts focus behind the open sheet | The shown page's entry, on the next render | P23 |
| F16 | A request or read lands after Settings closed | Unchanged | P22 |
| F17 | `FocusContent` or a review landing while Settings is open | Unchanged, inside the sheet | P23 |

## Part B: The Frame

### B1. Header, menu, body

- **Sheet.** `.size(px(720.))`, right placement (the default), and the
  Kit's overlay and close button. No footer (decision 4).
- **Header.** `title(…)` holds a `v_flex` of:
  - "Settings", 15 px semibold, as the details sheet's title (fact 28;
    `07`'s sheet title);
  - "Applies to this computer only." (`SETTINGS_NOTE`, secondary text).

  The Kit has no description builder (fact 36; R-25).
- **Menu.** A 214 px column of three entries, in order: Repositories,
  Sharing & keys, Checking for updates (`SettingsNav`, R-4).
  - Plain controls, as today (fact 5; R-T12a): each is a div with
    `Role::Tab`, `aria_selected`, `track_focus` on its entry handle, and
    id `("settings-page", ix)` for `ix` in 0..3.
  - Each has a 14 px icon (`folder-git-2`, `key-round`, `refresh-cw`, as
    the mockup's nav, fact 41) and its label.
  - The active entry has the `border` background and is announced
    selected. The "Shown" text tag goes, since `aria_selected` announces
    it and the label is unchanged.
  - Enter, Space or a click opens the entry's page.
- **Body.** The shown page's view, in an absolute `inset_0` scroll laid
  over the Kit body, as the details sheet does (fact 28; R-12). It has
  its own `ScrollHandle` and vertical scrollbar.
- **Fields** fill the body's width up to today's 480 px. The body is
  720 − 214 − padding wide, less than 480 px (fact 17; R-39).

```rust
/// The menu's three entries (decision 6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SettingsNav {
    Repositories,
    SharingAndKeys,
    Checking,
}

impl SettingsPage {
    /// The menu entry that shows this page: Repositories and Remotes under
    /// Repositories; Keys and Hosts under Sharing & keys; Polling under
    /// Checking for updates.
    pub(crate) fn nav(self) -> SettingsNav;
}
```

`SettingsPage` keeps its five variants:

- they name the page entities and their result scopes (fact 2);
- tests read them through `Shown` (fact 3; R-3).

An entry opens its first page (`Repositories`, `Keys`, `Polling`). The
frame lives in a new `D/view/settings/frame.rs`, and `arrive` in
`D/view/settings/arrive.rs` (`mod.rs` is 517 lines).

### B2. Scrolling and the page keys

- **Its own scroll** (R-12).
  - `SettingsHost` holds a `ContentScroll` made by
    `ContentScroll::region()` from the column's. It shares the column's
    focus-reveal state, whose `note_focus` already runs every frame
    (fact 13).
  - So a control taking focus below the fold is revealed in the sheet, as
    the details sheet's are (fact 28).
  - Every page, made with the host's scroll today, gets this one.
- **The page keys** (R-40).
  - The frame's root element, which holds both the menu and the body,
    handles `ScrollPageUp`, `ScrollPageDown`, `ScrollToTop` and
    `ScrollToBottom` on the body's scroll, without propagating.
  - The keys are bound in the `Content` context, which the sheet's focus
    is inside (fact 26). So they reach the frame from a menu entry or a
    page control before the Workspace's guarded handlers. The Kit's close
    button sits in the sheet's header, outside the frame
    (`K/sheet.rs:190-208`), so page keys pressed on it do nothing.
  - Inside a text input, its own Home and End win, as today.
  - The menu entries carry no scroll marker of the body's.
- **The item keeps its place.** The column and the item's region are no
  longer Settings'. A control revealed in Settings moves only Settings'
  scroll, because reveal moves the scroll whose marker is focused
  (fact 13).

### B3. A typed passphrase

The D2 rule is "a typed passphrase does not outlive the page being shown"
(fact 12). It becomes: the Keys page's masked inputs are emptied when
(R-13):

- Settings closes (A3 step 2, quiet closes included);
- the shown page leaves Sharing & keys, that is `shown.nav() !=
  SettingsNav::SharingAndKeys`;
- the Generate form is hidden, by either button (C2.2).

`SettingsHost::hide`, the page change and hiding the Generate form call
`forget_passphrase`, whose test becomes the `nav()` one. The render-time call stays with the same
test. So a passphrase typed while Trusted servers' Check host is the
route's page (`Shown::Settings(Hosts)`) is kept, because both draw on
Sharing & keys. `content_body`'s calls go (fact 9).

## Part C: The Pages

### C1. Repositories

One page, in this order:

1. **Repositories** (today's managed section): the registrations, each
   with Manage, then, for the chosen repository, its identity form and
   Remove.
2. **Sharing** (today's Remotes page, without its own registrations
   list): the chosen repository's remotes, which one is the shared copy,
   and the Add a remote form.
3. **Add an existing repository** and **Create a repository**, as today,
   until Cycle 05 moves them (Wave decision 10).

**One chosen repository** (R-14). `SettingsHost.selection` becomes
`chosen: Option<PathBuf>`: the repository that both the Repositories page
and Trusted servers' Check host work on. At each open:

- the route's root, when the route names one, except `Inspect` (R-55);
- else the Repositories panel's selection, when it has changed since
  Settings last opened (fold-in, Part G);
- else the previous choice, kept with its typed text.

Every open, whatever its route, records the panel's selection it saw, so
"changed since Settings last opened" compares with the latest open.
Manage on a row also sets the choice.

**`Inspect` does not choose** (R-55). It fills and checks the Add folder as
today (`inspect_root`, fact 15). The folder may not be registered, and
choosing it would empty another repository's typed identity text.

**Unchoosing** (R-56). Today, removing the chosen repository, or a
registrations read that no longer lists it, calls the Repositories page's
own `forget_root` (`repositories/manage.rs:212-213`, `:255`, `:420`). It
now emits a `RepositoriesEvent::Forgot(root)`. The host answers with
`choose(None)` for both pages, so neither Sharing nor Check host stays on
a repository that is no longer registered.

Every choice goes through one `SettingsHost::choose(root)`:

- **`root` is already chosen:** the identity inputs are left alone, so
  text being typed is kept (the 03a ledger's R5-2 lesson).
- **Otherwise:** it does what `manage_root(root, false)` does today
  (fact 15), and points the Sharing section at `root` (today's
  `open_on`, fact 16).
- **The identity read's prefill fills only empty fields**, so a slow read
  never overwrites typing (decision 12).

Every page is read again when shown (R-32). `RepositoriesPage` gains
`opened`, which calls `load_registrations` (fact 4).

The Remotes page's own registrations list, its Choose and its read at
`new` go (R-15). Manage is the one way to choose. With them goes the
ticket's "first open reads the list twice" (decision 12).

### C2. Sharing & keys

One page, in this order:

1. **Key used for sharing.** `05`'s intro, then the key list (bordered,
   radius 8), one row per key (decision 2; R-17):
   - **a radio**, checked when the key is in use (R-38). The Kit's
     `Radio` cannot take focus from a route and has no click when checked
     (fact 39), so the radio is a wrapped div drawn as a radio. It has
     `role(Role::RadioButton)`, `aria_toggled`, a tab stop and the
     accessible name "Use {label} for sharing". Click, Space or Enter
     sends `KeySelect`, as Select does today, and does nothing when the
     key is already in use;
   - the label, an origin tag ("Made by Manyhands" for a generated key,
     "Imported" otherwise) and an "In use" tag on the selected key, which
     uses words, not colour alone;
   - the fingerprint in mono, or today's "No fingerprint recorded";
   - today's source-state line when the private file was missing or
     unreadable (`SOURCE_MISSING`, `SOURCE_UNAVAILABLE`);
   - **Copy public key**, as today;
   - **a trash icon button**, named "Remove from Manyhands", sending
     `KeyRemove` (today's Unregister). Its tooltip is "Remove from
     Manyhands. Keeps the key file." (decision 11);
   - on generated keys only, **Delete key file…**, which opens today's
     `KeyDelete` confirmation.

   Under the list, while a key is in use, **Stop using this key** sends
   `KeyClear` (today's Clear selection). After a key is removed or its
   file deleted, focus goes to the next key's radio by key ID, else the
   previous one's, else Make a new key (decision 12). The passphrase
   note, the dates and the algorithm are left out (K-1).
2. **Make a new key** (primary) and **Use a key I already have…** (R-18;
   labels verbatim from `05`):
   - neither form is shown at first;
   - each button shows its form below (today's Generate or Import form)
     and hides the other, emptying the Generate form's passphrase fields
     when it hides;
   - pressing the button of the shown form hides it;
   - the Generate form is shown whenever the Keys page is retrying a
     protected generation, however the retry began: `ask_again`, or
     `operations_changed` (`keys/commands.rs:269-275`), since the retry
     prompt is drawn inside that form (`keys/render.rs:259-300`). A
     route's retry (W-26) also focuses its passphrase field (R-T15a;
     R-54).
3. **The info box** (`lock` icon): "If your key has a passphrase,
   Manyhands asks for it once each time you open the app and never stores
   it." (fact 43; R-19).
4. **Trusted servers** (today's Hosts page):
   - each server with its host, mono fingerprint and the reapproval
     notice;
   - then **Check host** for the chosen repository, with its result,
     Approve… and Replace…, as today.

   The design's "Trusted Oct 4" is left out (K-1).

### C3. Checking for updates

D6's page is kept as it is, apart from copy (Part E). There is no "Pause
all checking" switch (fact 44; K-2). Its Set up sharing… and attention
actions stay inside the sheet: they switch the menu and arrive as W-31 and
W-32.

**Show in Activity** (decision 5; R-20):

- it closes Settings quietly (A3);
- it then reveals and focuses Activity, showing the panel if hidden, with
  today's `focus_region(Region::Operations)` (fact 8). After Cycle 03b,
  that focuses Activity's list.

`SettingsEvent::ShowActivity` keeps its name. The Workspace's answer now
closes the sheet first.

## Part D: Ways In

`SettingsHost::arrive(route)` switches the menu to the route's page and
applies the route. It returns the focus target, and `None` means the
page's entry. "Shown" is what `settings_shown` then reads (R-3). Lines are
at `ba95e1b7`; Task 0 re-maps them after Cycle 03b merges (U-5).

| W | Trigger | Today's call | Route | Shown | Focus |
| --- | --- | --- | --- | --- | --- |
| W-1 | Ctrl+, (`render.rs:262-264`) | `open_settings(Repositories)` | `Page(Repositories)` | `Settings(Repositories)` | entry |
| W-2 | Footer Settings (`workspace/footer.rs:174`) | same | same | same | entry |
| W-3 | Footer "Checked for updates" (`workspace/footer.rs:175`) | `open_settings(Polling)` | `Page(Polling)` | `Settings(Polling)` | entry |
| W-4 | Repositories panel Settings button (`repositories/body.rs:139`) | `open_settings(Repositories)` | removed (Part F) | — | — |
| W-5 | Empty Repositories panel "Add or create a repository…" (`repositories/body.rs:113`) | `open_settings(Repositories)` | `AddRepository` | `Settings(Repositories)` | `add-path` (R-24) |
| W-6 | Repositories toolbar Add (`dock/header/toolbar.rs:217-218`) | same | same | same | `add-path` |
| W-7 | Repository menu Settings… (`workspace/repository_menu.rs:221-222`) | `open_settings_on(Repositories, root)` | `Manage(root)` | `Settings(Repositories)` | entry |
| W-8 | Repository row action Settings… (Needs fixing; `repository_menu.rs:245-246`) | same | same | same | entry |
| W-9 | Repository row action Set up sharing… (`repository_menu.rs:248`) | `open_remotes(root)` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
| W-10 | Share main… with no remote (`requests.rs:320-321`) | `open_remotes(root)` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
| W-11 | Item Share… with no remote (`item/share.rs:400` → `requests.rs:201`) | `open_remotes(root)` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
| W-12 | Share outcome Open Sharing & keys (`operations/outcomes.rs:211` → `requests.rs:202`) | `open_settings(Keys)` | `Keys` | `Settings(Keys)` | key list (R-22) |
| W-13 | Share outcome Check host (`outcomes.rs:212` → `requests.rs:203-210`) | `check_host_in_settings` or `open_hosts` | `CheckHost(root, a)` or `Hosts(root)` | `Settings(Hosts)` | `hosts-check` |
| W-14 | Share outcome Set identity (`outcomes.rs:219` → `requests.rs:211`) | `identity_in_settings` | `Identity(root)` | `Settings(Repositories)` | `identity-name` |
| W-15 | Share outcome Set up sharing (`outcomes.rs:220` → `requests.rs:201`) | `open_remotes` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
| W-16 | Comment offer Open Sharing & keys (`item/posting.rs:366`) | `open_settings(Keys)` | `Keys` | `Settings(Keys)` | key list |
| W-17 | Comment offer Check host (`posting.rs:367-370`; no authority) | `open_hosts(root)` | `Hosts(root)` | `Settings(Hosts)` | `hosts-check` |
| W-18 | Comment offer Set identity (`posting.rs:371`) | `identity_in_settings` | `Identity(root)` | `Settings(Repositories)` | `identity-name` |
| W-19 | Composer Set identity (`item/composer/place/actions.rs:78`) | same | same | same | `identity-name` |
| W-20 | Lifecycle dialog Check host (`lifecycle/running.rs:317`) | via `open_route` | `CheckHost` or `Hosts` | `Settings(Hosts)` | `hosts-check` |
| W-21 | Lifecycle dialog Open Sharing & keys (`running.rs:329`) | `open_settings(Keys)` | `Keys` | `Settings(Keys)` | key list |
| W-22 | Lifecycle dialog Open repository details (`lifecycle/actions.rs:143-150`) | `inspect_in_settings` | `Inspect(root)` | `Settings(Repositories)` | `add-path` |
| W-23 | Activity Choose a key… (`operations/attention.rs:147`) | `open_settings(Keys)` | `Keys` | `Settings(Keys)` | key list |
| W-24 | Activity Open remote settings… (`attention.rs:148`) | `open_remotes(path)` | `Remotes(path)` | `Settings(Remotes)` | `remote-add-name` |
| W-25 | Activity Approve host… (`attention.rs:149-151`) | `check_host_in_settings`, or **`open_settings(Hosts)` on the panel's selection** | `CheckHost(path, a)`, or `Hosts(path)` (R-23) | `Settings(Hosts)` | `hosts-check` |
| W-26 | Recovery Retry of a protected key generation (`requests.rs:615-616`) | `retry_generation_in_settings` | `RetryGeneration(id)` | `Settings(Keys)` | `key-generate-passphrase` |
| W-27 | Recovery `repo.inspect` / `remote.history` (`requests.rs:627`) | `inspect_in_settings` | `Inspect(root)` | `Settings(Repositories)` | `add-path` |
| W-28 | Recovery `repo.identity_set` (`requests.rs:628`) | `identity_in_settings` | `Identity(root)` | `Settings(Repositories)` | `identity-name` |
| W-29 | Recovery `host.approve` / `host.replace` (`requests.rs:629-630`) | `check_host_in_settings` | `CheckHost(root, a)` | `Settings(Hosts)` | `hosts-check` |
| W-30 | Save banner Open Settings, initials required (`tabs.rs:329-330` → `content.rs:145-146`) | `identity_in_settings` on the next frame | `Identity(root)` | `Settings(Repositories)` | `identity-name` |
| W-31 | Checking for updates, Set up sharing… (inside; `settings/mod.rs:284-285`) | `set_up_sharing` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
| W-32 | Checking for updates, attention actions (inside; `settings/attention.rs:11`) | `attend` | `Keys`, `CheckHost`, `Hosts(root)` or `Remotes(root)` | as their rows | as their rows |
| W-33 | Repository row action for an attention cause: Choose a key…, Approve host…, Open remote settings… (`repository_menu.rs:243` → `attention.rs:119-152`) | `attend` | as W-23 to W-25 (R-23 applies) | as W-23 to W-25 | as W-23 to W-25 |
| W-34 | Cycle 03b: an Activity or popover repository-state row's Settings… (S4 Needs fixing; 03b design `:1936`) | 03b's `choose_activity` → `row_action` | `Manage(root)` | `Settings(Repositories)` | entry |
| W-35 | Cycle 03b: an Activity or popover repository-state row's Set up sharing… (S4 This computer, non-SSH; 03b design `:1941`) | same | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |

**After Cycle 03b,** the triggers of W-12 to W-15 and W-23 to W-29 are
Activity rows. So are 03b's S4 "Needs attention" repository rows, whose
actions go through `attend` (W-33's routes). Each can be raised three
ways: its button, its row menu (`shift-f10`), and, when it counts, its
popover row. W-34 and W-35 are new triggers for existing routes. Task 0
maps every row to 03b's merged code and adds any 03b trigger this table
misses (U-5).

**Origins** (R-6):

| Trigger | Origin |
| --- | --- |
| Footer, Ctrl+, from a list, a panel control or a menu | The focused control |
| An item header or banner control | The focused control |
| A lifecycle dialog (W-20 to W-22) | The dialog's own origin, which it focuses before routing (fact 22) |
| An Activity row (button, menu or Enter) | The Activity list's handle (03b decision 23), through `settings_origin` |
| A popover row | The footer item, through `settings_origin` |
| Ctrl+, while 03b's popover or a row menu holds focus | The control that layer returns to (A2, "The origin", case 3) |
| W-31, W-32 | The origin recorded when Settings opened |

Notes:

- **Focus targets** (decision 8; R-24). Today, routes with no field focus
  the page's entry (fact 10). The Wave asks for "the right page and
  field", so routes that name a field focus it. `Page` and `Manage`
  routes keep the entry.
- **The key list** (W-12, W-16, W-21, W-23; R-22) focuses the in-use
  key's radio, else the first key's, else Make a new key. While the keys
  are still being read, focus goes to the entry. When the read lands, it
  moves to the target only if the entry still holds focus and Settings is
  open.
- **`hosts-check`** for `Hosts(root)` (W-13, W-17, W-20, W-25) is Check
  host for the chosen repository. It is always drawn once a repository is
  chosen, and disabled, with `NO_SSH_PUBLICATION` beside it, when the
  repository has no shared copy over SSH (`hosts.rs:733-742`). At arrival
  the shared copy is still being read, so focus always goes there (R-53).
- **W-25's fix** (decision 9; R-23). Today, Approve host… with no server
  named opens Hosts on the Repositories panel's selection, not the
  attention row's repository (`attention.rs:151`). It now uses the row's
  repository, as W-32's path does. The same holds on W-33's path.

## Part E: Copy

Copy follows `06` (fact 42). Every string is a `pub(crate) const`, and
tests compare constants. The changes:

| Today (constant, file) | New |
| --- | --- |
| `BACK_TO_TABS` "Back to tabs" (`settings/mod.rs:52`) | removed: the header's close button replaces it |
| `page_title` (`mod.rs:66-70`) | the menu: "Repositories", "Sharing & keys", "Checking for updates" |
| — | `SETTINGS_NOTE` "Applies to this computer only." |
| `REMOTES_HEADING` "Remotes" (`remotes.rs:74`) | `SHARING_HEADING` "Sharing" |
| `READING_REGISTRATIONS`, `NO_REGISTRATIONS`, `CHOOSE`, `CHOSEN` (`remotes.rs:76-79`) | removed with the Remotes page's list (R-15). `REPOSITORY_HEADING` stays: `chosen_line` (`remotes.rs:118-119`) names the chosen repository as `remotes-root` |
| `NO_REPOSITORY` (`remotes.rs:80`) | "Manage a repository above to see how it shares." |
| `ELIGIBLE` "Eligible for publication" (`remotes.rs:85`) | "Can be the shared copy" |
| `NOT_ELIGIBLE` (`remotes.rs:86-87`) | "Can't be the shared copy: sharing needs a remote reached over SSH" |
| `SELECTED` "Selected for publication" (`remotes.rs:88`) | "The shared copy" |
| `SELECTION_NONE` (`remotes.rs:89`) | "No shared copy is chosen, so changes stay on this computer." |
| `SELECT` "Select for publication" (`remotes.rs:90`) | "Use as the shared copy" |
| `SELECT_NONE` "Publish to no remote" (`remotes.rs:91`) | "Keep changes on this computer" |
| "Publication remote: {name}" (`remotes.rs:114`) | "Shared copy: {name}" |
| `KEYS_HEADING` "Shared SSH keys" (`keys.rs:37`) | "Key used for sharing", with `05`'s intro sentence (`KEYS_INTRO`) |
| `NO_KEYS` "No SSH key is registered yet." (`keys.rs:41`) | "No key is set up for sharing yet." |
| `KIND_GENERATED` "Generated by Manyhands" (`keys.rs:42`) | "Made by Manyhands" |
| `SELECTED` "Selected" (`keys.rs:48`) | "In use" |
| `SELECT` "Select" (`keys.rs:52`) | `USE_KEY`, the radio's accessible name, "Use {label} for sharing" (a formatting function) |
| `CLEAR` "Clear selection" (`keys.rs:53`) | "Stop using this key" |
| `UNREGISTER` "Unregister" (`keys.rs:54`) | "Remove from Manyhands"; tooltip `REMOVE_TOOLTIP` "Remove from Manyhands. Keeps the key file." |
| `DELETE` "Delete key" (`keys.rs:55`) | "Delete key file…" |
| — | `MAKE_KEY` "Make a new key", `USE_EXISTING` "Use a key I already have…" (verbatim from `05`); the forms keep their headings |
| — | `KEYS_NOTE`, the info box (C2.3) |
| `IMPORT_NOTE` (`keys.rs:72-73`) | "Importing uses the files where they are. Removing a key from Manyhands leaves its files." |
| `PINS_HEADING` "Host pins" (`hosts.rs:67`) | "Trusted servers" |
| `READING_PINS` (`hosts.rs:69`) | "Reading the trusted servers" |
| `NO_PINS` (`hosts.rs:70`) | "No server is trusted yet." |
| `REAPPROVAL` (`hosts.rs:71-72`) | "The list of trusted servers was lost: approve each server again." |
| `NO_REPOSITORY` (`hosts.rs:73-74`) | "Manage a repository on the Repositories page to check the server it shares through." |
| `NO_SSH_PUBLICATION` (`hosts.rs:76-77`) | "Check host is unavailable: this repository's shared copy is not reached over SSH. Choose one under Repositories › Sharing." |
| `CHECK_NOTE` (`hosts.rs:79-80`) | "Check host connects to the server this repository's shared copy is on and shows the key it presents. It trusts nothing yet." |
| `NOT_PINNED` (`hosts.rs:83`) | "This server is not trusted yet." |
| `MATCHES` (`hosts.rs:84`) | "The server presents the key you trusted." |
| `DIFFERENT` (`hosts.rs:85`) | "The server presents a different key from the one you trusted." |
| `PINNED` "Pinned key" (`hosts.rs:86`) | "Trusted key" |
| `APPROVE` "Approve host", `REPLACE` "Replace host key" (`hosts.rs:87-88`) | "Approve host…", "Replace host key…": each opens a confirmation (`06` `:36`) |
| `REMOVE` "Remove remote" (`remotes.rs:92`) | "Remove remote…" |
| `REMOVE` "Remove repository" (`repositories/manage.rs:73`) | "Remove repository…" |

The rule for anything Task 0's sweep finds beyond this table (R-24's copy
half):

- a Settings string that uses a term `06` replaces is rewritten with
  `06`'s term and the new place names. The terms are: publication or
  publish, pin, SSH key, host pin, the Hosts page, the Remotes page, the
  sidebar;
- a string that does not is unchanged;
- Git detail (a remote's name, a branch) may stay in Settings, which is a
  sheet (`06` header).

Task 7 lists every rewritten constant in the ledger. P18 checks, by whole
words, that no Settings constant still uses a replaced term.

## Part F: What Leaves

- `ContentPage::Settings`, its branch in `content_body`, `focus_content`
  and `shown_item` (fact 9), and the column's use by Settings.
- `SettingsEvent::Closed`, `back`, the `close` handle, "Back to tabs"
  (fact 6) and the "Shown" tag (fact 5).
- The Repositories panel's Settings button, `sidebar-settings`
  (`repositories/body.rs:129-141`). This is in the Wave's scope and Cycle
  03a's decision 15.
- `open_item`, `show_tab`, ctrl-tab and `open_drafts` no longer hide
  Settings: `show_tabs` and `open_drafts` stop calling `settings.hide`
  (`content.rs:56-59`, `:84-86`). None of them can be pressed while the
  sheet is open (A5). `open_item` can still run from an async landing,
  which leaves Settings open (A1).
- The module doc's R-B4 paragraph (fact 1) is rewritten for the sheet.

The empty Repositories panel's action and the toolbar's Add keep opening
Settings › Repositories until Cycle 05 moves Add repository to its own
sheet (W-5, W-6).

## Part G: Folded-In Work

From `01M5F3GVDYTH1Q5V5FCS9YF7V3` (decisions 3 and 12), the items in code
this Cycle rewrites:

1. **"Settings and tabs share one scroll offset."** Settings scrolls its
   own region (B2). P8.
2. **"Reopening Settings ignores a sidebar selection made since the last
   open."** An open follows a newer panel selection (C1). P10.
3. **"The first open reads the list twice"** (Remotes). The Remotes page's
   list and its reads go (C1; R-15).
4. **"The focus fallback for Delete is keyed by row index."** It is keyed
   by key ID (C2.1). P24.
5. **"A slow identity read can overwrite text the user is typing."** The
   prefill fills only empty fields (C1). P25.

**The planner's own fix** (R-32): Repositories is read again when shown,
so a registration made while Settings was closed is listed. This was
presented to the owner as one of the ticket's items, which it is not;
decision 12 corrects the record.

The ticket's Add repository items (Browse, folder inspection, non-UTF-8
roots) wait for Cycle 05. The rest stays in that ticket.

## Part H: Keyboard, Focus And Privacy

- Every control in the sheet is reached by Tab within the Kit's trap
  (fact 37). Tab from the last control wraps to the first, and Shift-Tab
  from the first wraps to the last. P23.
- Focus goes where A6's table says. P5 covers each origin of Part D's
  table.
- A dialog a page opens (a confirmation, the passphrase prompt) sits above
  the sheet and gives focus back inside it (fact 38).
- Every key-management action works by keyboard (fact 45). P19.
- No new text names a passphrase, private key, remote URL or file path
  beyond what today's pages show. No action displays or copies a private
  key (fact 45). P21.

## Required Outcomes

Each is a test, written first. Window tests use the shell fixtures of
today's `settings/*_tests.rs`. Test-only accessors:

- `Workspace::sheet_owner() -> Option<SheetOwnerKind>`, a copyable
  mirror of `SheetOwner` without the pane;
- `Workspace::settings_chosen(cx) -> Option<PathBuf>`.

| P | Test | Asserts |
| --- | --- | --- |
| P1 | `settings_is_a_sheet_the_workspace_keeps` | After Ctrl+,, the owner is Settings and a sheet is active across frames: with an item shown, with Drafts shown under it, and after `open_item` runs (an async landing). A details sheet requested in the same frame as Ctrl+, is dropped (A1 case 5; R-51) |
| P2 | `settings_leaves_the_item_as_it_was` | With an item open, its region scrolled and a field edited into a draft: Ctrl+,, Tab to a control below the fold, the `AddRepository` route (W-6's) to `add-path`, then Escape. The same tab is shown, `tab_keys` unchanged, the item's offset unchanged, and the draft holds the edit (Wave exit evidence) |
| P3 | `settings_replaces_the_details_sheet_and_keeps_its_edits` | Details sheet open with a field edited: Ctrl+, opens Settings, the details sheet is gone and the draft holds the edit. Escape returns focus to the chip (Wave decision 6; R-6) |
| P4 | `each_route_lands_on_its_page_and_control` | A table over the ten routes, through `show_settings`: `settings_shown`, the focused element's id and `settings_chosen` (A6 F1, F2). Rows include: the key list while still being read (the entry, then the radio when the read lands); a route naming no root after the panel's selection changed and after it did not (R-14); `Inspect` leaving the choice alone (R-55); `Hosts(path)` with the panel's selection on another repository (R-23). For each W no existing test raises (W-9, W-14, W-16 to W-20, W-33 to W-35), a row raises it through its own handler (the wrapper, `row_action`, `attend` or `open_route`) |
| P5 | `focus_returns_to_what_opened_settings` | A table, one row per A6 row F4 to F10: the footer item; a ticket list row; an item header's Share…; the save banner; a lifecycle dialog route (the dialog's origin); an Activity row; a popover row; a popover row whose footer item has hidden by close time; Ctrl+, pressed while 03b's popover holds focus, and while a row menu does; W-31 inside the sheet (the first origin); a ticket row filtered out while Settings was open (the Tickets panel's region); an origin whose handle was dropped |
| P6 | `settings_closes_every_way` | A table: Escape, the close button, the scrim, Show in Activity, quitting with Activity displayed, quitting with Activity hidden, quitting with a confirmation open inside Settings, quitting with unkept edits. Each leaves `Shown::Tabs`, the owner `None`, the host closed and a typed passphrase empty. Focus is as A6 F11 to F13 say |
| P7 | `nothing_acts_behind_settings` | With Settings open on Sharing & keys, by dispatched actions: the panel actions, `FocusContent`, `NewItem`, ctrl-tab, close tab and `OpenDrafts` change nothing; `OpenSettings` leaves the page as it was. Written in Task 1, before the swap, when it fails |
| P8 | `settings_scrolls_on_its_own` | PageDown from a menu entry and End from a page control move Settings' offset, not the column's or the item's; Tab to a control below the fold reveals it |
| P9 | `a_typed_passphrase_lives_only_on_sharing_and_keys` | A table: typed, then closed (emptied); typed, then the menu leaves Sharing & keys (emptied); typed after a `Hosts(path)` route (kept while on Sharing & keys); typed, then Use a key I already have… (emptied); typed, then Make a new key pressed again, hiding its form (emptied) |
| P10 | `one_repository_is_chosen` | Manage b shows b's remotes under Sharing; `Remotes(a)` chooses a; a footer open after the panel's selection changed to c chooses c; a footer open with no change since a route open keeps the route's choice and its typed identity text; choosing the chosen repository again keeps typed identity text; removing the chosen repository unchooses it for Sharing and Check host (R-56) |
| P11 | `every_page_is_read_again_on_open` | A registration added while Settings is closed is listed on the next open of Repositories |
| P12 | `key_rows_send_what_todays_controls_sent` | The radio, Stop using this key, the trash and Delete key file… send `KeySelect`, `KeyClear` and `KeyRemove`, and open the `KeyDelete` confirmation. Delete key file… appears only on generated keys, In use only on the selected key, and the radio of the key in use sends nothing |
| P13 | `the_key_forms_open_from_their_buttons` | Neither form at first; each button shows its form and hides the other; pressing it again hides it; W-26 shows the Generate form with focus in its passphrase |
| P14 | `show_in_activity_closes_settings_and_focuses_activity` | With Activity hidden: the button closes Settings, shows Activity and focuses it |
| P15 | `settings_does_not_open_behind_a_dialog_or_while_quitting` | A route raised under an open dialog and one raised while quitting open nothing |
| P16 | `settings_has_three_pages` | The menu has three entries in order; each opens its first page; Enter and Space on an entry open it; the active one is announced selected |
| P17 | `the_old_settings_places_are_gone` | No `sidebar-settings`, no `settings-close`; the item area never shows Settings |
| P18 | `settings_copy_uses_the_vocabulary` | No Settings copy constant contains, as a whole word, "publication", "publish", "pin", "pinned", "pins", "sidebar", or the phrases "SSH key", "Remotes page", "Hosts page", "Back to tabs" |
| P19 | `sharing_and_keys_by_keyboard` | By keys only: choose a key, stop using it, Make a new key and generate, Use a key I already have…, remove a key, Delete key file… through its confirmation, Copy public key |
| P20 | `routes_by_keyboard` | One trigger of each kind, focused and pressed with Enter: the footer item, an Activity row's button, an Activity row's menu entry, a popover row, a repository row's menu entry, an item header's Share…, a lifecycle dialog's Open Sharing & keys, the save banner's Open Settings. Each lands as Part D says |
| P21 | `settings_copy_names_no_secret` | With a key whose label, path and fingerprint are sentinels: the new constants, tooltips and the radio's accessible name hold the label and fingerprint only, never a path or passphrase |
| P22 | `late_work_moves_no_focus` | With the panel's selection unchanged throughout: type a folder and press Add while its check is pending, press Escape, let the check land (no confirmation opens, focus unchanged, and reopening shows no pending Add); remove a key, press Escape, let it land (focus unchanged); start a Check host, press Escape, move focus to nothing, let it land (focus not moved into the closed sheet); reopen and see each result. A route raised after the Kit closed the sheet but before `sync_sheet` saw it empties a typed passphrase first and records the new origin (A2 step 2) |
| P23 | `focus_stays_inside_settings` | Tab from the last control reaches the first, Shift-Tab the reverse (Kit trap); `FocusContent` and a review landing leave focus inside the sheet (A6 F17); a dialog opened from a page gives focus back inside (F14); confirm Drafts' Discard… with Enter, press Ctrl+, at once, advance the clock 250 ms: focus is on the page's entry, not the Drafts row (F15; R-46) |
| P24 | `removing_a_key_returns_focus_by_key` | Remove the middle of three keys: focus goes to the next key's radio, even if the list's order changes on the read |
| P25 | `a_slow_identity_read_fills_only_empty_fields` | Type a name, let the identity read land: the typed name stays and the empty email is filled |

## Testing

- **Command.** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
- **Layer.** Every test is a desktop unit test. Nothing touches the
  library, so no integration or SSH target changes (`AGENTS.md`).
- **Behaviour, not appearance** (Wave Testing). No test asserts widths,
  colours, icons or the header's layout. P16 checks entries and order, not
  their look.
- **Focus** is read inside the same update that moves it (Cycle 01 and 03a
  ledgers).
- **Guards** are tested with dispatched actions, not key presses bound in
  another context and not pointer clicks, which land on the scrim (Cycle
  02 ledger).
- **Window tests with polling on** answer every poll (03a handoff).
- **Helpers.** The five shared helpers that open a page (E-5) change in
  one place each, so their callers need no edit.
- **P4 and the existing route tests.** P4 checks each route through
  `show_settings`. The existing route tests (E-18) raise the triggers and
  gain one landing assertion each. For each W that no existing test
  raises (W-9, W-14, W-16 to W-20, W-33 to W-35), P4 calls the trigger's
  own handler, so it proves the trigger builds its route. P20 raises one
  trigger of each kind by keyboard.
- **A6 first.** Task 2 writes P5's and P23's rows from A6's table before
  any of the focus mechanism (A6).

## Named Edits To Pre-Existing Tests

At `ba95e1b7`. Task 0 re-counts them with the plan's sweep after Cycle 03b
merges, since 03b edits several of the same tests (its E-4, E-6, E-7,
E-12 and E-13). Edits that remove assertions are marked **Removes**.

| E | Tests | Edit | Task |
| --- | --- | --- | --- |
| E-1 | `D/view/settings/mod_tests.rs`: `the_settings_key_shows_the_settings_and_back_keeps_the_tabs` (`:73`), `opening_an_item_shows_the_tabs_again` (`:137`), `focusing_the_content_region_focuses_the_settings` (`:168`), `the_sidebar_entry_shows_the_settings` (`:123`) | `:73`: Escape instead of Back; **Removes** the tab-strip-hidden assertion (`:98`), since the strip stays drawn under the sheet, and the `BACK_TO_TABS` label assertion (`:106-109`). `:137` **Removes** the test: no item opens behind the sheet (P7). `:168` **Removes** the test: `secondary-3` is refused behind the sheet (P7). `:123` **Removes** the test in Task 8, with the button (P17). `:73`'s `settings-page-shown` check (`:91-93`) **Removes** in Task 5. `the_settings_key_does_nothing_while_a_dialog_is_open` (`:154`) is unchanged | 1; 5 (`:91-93`); 8 (`:123`) |
| E-2 | `D/view/item/sheet_tests.rs` `no_workspace_key_acts_behind_the_sheet` (`:670-741`) | **Removes** `secondary-,` from the refused keys (`:719`, `:731-739`); P3 asserts the new behaviour | 1 |
| E-3 | `D/view/item/frame_tests.rs` `page_keys_scroll_the_items_region_and_settings_keep_theirs` (`:148-178`) | Settings' PageDown moves Settings' own offset, not `content_offset`; it closes by Escape | 1 |
| E-4 | `D/view/drafts_routing_tests.rs` (`:66`, `:83`) | `:66`: Settings no longer hides Drafts, and it closes by Escape. **Removes** `!present("drafts")` and the `!drafts_shown` and `present(("tab",1))` checks (`:70`, `:76`, `:77`), which invert. `:83`: the drafts key cannot act behind the sheet; the passphrase is emptied by closing Settings, then the drafts key opens Drafts | 1 |
| E-5 | Helpers: `settings` (`D/view/settings/repositories_tests.rs:40`), `hosts` (`hosts_tests.rs:61`), `remotes` (`remotes_tests.rs:47`), `keys` (`keys_tests.rs:51`), `Rig::open` (`polling_support.rs:104`), `open_page` (`D/view/keyboard_tests/settings.rs:22`) and its five callers, `Walk::open_settings_and_each_page` (`D/view/keyboard_tests.rs:561-577`) | Task 1: the walk closes by Escape and finds focus on the control that opened Settings, not the shown tab (`:574-576`). Task 5: menu indices 0–2 instead of 0–4, with `remotes` choosing its root by Manage | 1, 5 |
| E-6 | Tests that leave Settings by `settings-close` or `settings-close-button`: `keys_tests.rs` `a_typed_passphrase_is_emptied_when_the_page_is_hidden` (`:350`) and `the_operation_area_retry_of_a_protected_generation_asks_on_the_page` (`:832`); `D/view/item/share_tests.rs` `share_opens_remotes_on_its_own_repository_after_another` (`:754`, `:771`); `D/view/create_tests.rs` `initials_required_points_to_settings` (`:886`, `:930-935`); `hosts_tests.rs` (`:263`); `remotes_tests.rs` (`:731`) | Escape instead of the button | 1 |
| E-7 | `D/view/operations/attention_tests.rs` `attention_actions_hit` (`:367`, `:387-391`) | Closes Settings by Escape between the row clicks, which the scrim would otherwise take | 1 |
| E-8 | `D/view/settings/attention_tests.rs` `settings_polling_attention_actions` (`:34`, `:83`) | Ctrl+, with Settings open now does nothing; the assertion that follows reads the page the attention action left | 1 |
| E-9 | `D/view/settings/remotes_tests.rs`: every test that clicks the Remotes page's Choose row | Chooses by Manage on the Repositories page (R-15) | 3 |
| E-10 | `D/view/settings/keys_tests.rs`: tests pressing Select, Clear selection, Unregister or Delete key by element id | The radio, Stop using this key, the trash and Delete key file…, by their new ids | 4 |
| E-11 | Tests comparing a literal Settings string rather than its constant | The new string or its constant (Part E) | 7 |
| E-12 | Tests that act outside Settings while it is open: `D/view/repositories_tests.rs` `rows_and_list_states` (`:146`, `:213`); `D/view/dock_tests/toolbars/repositories.rs` `repositories_toolbar` (`:72`, `:76`); `D/view/repositories_tests/menu.rs` `journey` (`:393`, `:401`, `:409`); `D/view/lifecycle/credential_tests.rs` `a_refused_key_offers_unlock_and_sharing_and_keys` (`:128`; `outcome_canned_tests.rs:223`); `D/view/keyboard_tests/shell/polling_journey.rs` `polling_keyboard` (`secondary-4` at `:247`, `:305`); `D/view/polling_privacy_tests.rs` `polling_privacy` (`:824`, `:846`, `:872`); `D/view/operations/attention_tests.rs` `attention_actions_by_reason` (`:224-225`) | Close Settings by Escape before the next action outside it | 1 |
| E-13 | `D/view/settings/remotes_tests.rs` `opening_remotes_on_a_repository_shows_that_repository_even_after_another` (`:748`, `:755-757`); `D/view/settings/repositories_tests.rs` `a_form_control_below_the_fold_is_scrolled_into_view` (`:650-668`) | Task 1: the first opens through the route, not `host.open_remotes_on`, closes by Escape, and keeps "Settings keeps b". Task 3: its last assertion follows R-14 (a reopen after the panel's selection changed follows it), a **Removes** of "Settings keeps b". Task 1: the second reads Settings' own scroll and the sheet body's bounds, not `content` and `content_offset` | 1, 3 |
| E-14 | `D/view/settings/polling_tests.rs` `settings_polling_states` (`:338-343`); `D/view/keyboard_tests/settings.rs` `add_a_remote_by_keyboard` (`:84`) | The first reads the chosen repository through `settings_chosen`, not `remotes-registration` and `remotes-chosen`. The second chooses by Manage | 3 |
| E-15 | `D/view/settings/keys_tests.rs`: every test that fills the Generate or Import form; `D/view/keyboard_tests/settings.rs` `generate_a_key_by_keyboard` (`:107`) | Press Make a new key or Use a key I already have… first | 4 |
| E-16 | Literal page indices: `drafts_routing_tests.rs:86`, `polling_tests.rs:69`, `polling_privacy_tests.rs:825`, `hosts_tests.rs:278`, `remotes_tests.rs:757`, `keys_tests.rs:357`, `:360`, `settings/attention_tests.rs:84`, `polling_journey.rs:152-160` | The three-entry indices | 5 |
| E-17 | Tests asserting a route focuses the page's entry where Part D now names a field: `repositories_tests.rs` `the_inspect_recovery_opens_the_page_with_its_root_checked` (`:705-708`), and those Task 0's sweep finds among the share, create and attention tests; Cycle 03b's handler test row for Approve host… with no server named (decision 9) | The field Part D names; for 03b's row, the attention row's repository | 6 |
| E-18 | The existing route tests: `footer_tests/mod.rs:309`, `footer_tests/status.rs` (`:404-409`), `repositories_tests.rs:53`, `dock_tests/toolbars/repositories.rs:49`, `repositories_tests/menu.rs` (`:395`, `:404`), `item/share_tests.rs` (`:164`, `:289`, `:754`), `operations/outcomes_route_tests.rs` (`:57`, `:76`), `operations/outcomes_tests.rs:407`, `lifecycle/credential_tests.rs:54`, `lifecycle/outcome_canned_tests.rs:270`, `operations/attention_tests.rs` (`:109`, `:171`, `:367`), `keys_tests.rs:832`, `repositories_tests.rs:688`, `repositories_manage_tests.rs:616`, `hosts_tests.rs:415`, `create_tests.rs:886`, `polling_tests.rs:308`, `settings/attention_tests.rs:34` | Each gains one assertion: the focused element's id that Part D names | 6 |

Further sites the sweep finds are recorded in the ledger as E-19 onward
(decision 7).

## Document Amendments

Applied by the planner in the approval commit:

1. **Wave 04, Cycle 04 section.** Add "Amended 2026-11-13 by Cycle 04":
   - the sheet has a header with "Applies to this computer only." and no
     footer (decision 4);
   - the menu has three pages; the design's General, Your name & email
     and Appearance pages are not shown;
   - Sharing & keys follows `05`'s row layout with today's actions, and
     without the passphrase note, dates and algorithm (decision 2);
   - no Pause all switch (D6 decision 8);
   - Show in Activity closes Settings (decision 5);
   - Escape closes Settings also while a request runs (decision 6).
2. **Wave 04, Cycle 05 section.** A note under "Common frame": Settings
   (Cycle 04) closes on Escape also while a request runs (decision 6). It
   lists what Cycle 05 adds when Add repository replaces Settings (the
   Cycle document's Downstream Obligations).
3. **`docs/design/05-sheets-and-dialogs.md`, Settings sheet.** An "As
   built, Wave 04 Cycle 04" note listing amendment 1's points, decision
   11's trash wording and the "Imported" tag in place of "From ~/.ssh".
4. **`docs/design/08-gpui-kit-mapping.md:32`.** Settings layout: plain
   controls, not the `Settings` component (D1 R-T12a; this Cycle's
   R-27).
5. **Wave 03 D2 design**
   (`docs/plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md:631-638`).
   A note: Settings is a sheet from Wave 04 Cycle 04, so it no longer
   excludes the Drafts page. A typed passphrase is emptied when Settings
   closes or leaves Sharing & keys.
6. **Wave 03 D6's Cycle document**, where it describes Show in Activity:
   a note that it closes Settings first (decision 5).
7. **Cycle 03a's Cycle document:**
   - decision 15 and K-8: "Done in Cycle 04" beside the panel's Settings
     button;
   - decision 20's "Settings › Remotes" becomes "Settings › Repositories
     › Sharing".

D1's R-B4 lives only in a ticket comment, which is not amended. Fact 1 and
Part F record that it is superseded.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Settings is a Kit `Sheet` on Root's one sheet layer (`08`:29), not a layer of the Workspace's own | The Kit's close and focus quirks (facts 34, 37) are handled in A3; a Workspace layer would need its own trap and scrim |
| R-2 | `SheetOwner` records who holds the sheet; `sync_sheet` keeps a Settings sheet (A1) | Without it Settings closes on the next frame (fact 29) |
| R-3 | `Shown` keeps its variants: `Tabs` now means Settings is closed, and `Settings(page)` the page entity the sheet last arrived at | The name `Tabs` reads oddly (K-3); renaming would touch 21 test files for no behaviour |
| R-4 | `SettingsPage` keeps five variants; `SettingsNav` groups them into three menu entries | Two enums to keep in step; `nav()` is one match |
| R-5 | Every way in goes through `show_settings`; today's entry functions stay as wrappers | One more layer of calls; the alternative edits 33 call sites and Cycle 03b's |
| R-6 | The origin is A2's first of: the argument, `settings_origin`, a Workspace layer's `restore`, the focused handle. After a details sheet closes, the focused handle is its chip (A2 steps 4–5; Part D's origins) | When the origin is gone, focus follows A3's fallbacks |
| R-7 | The close is detected in `sync_sheet`, not by `on_close` | A close in a frame that does not render would be seen one frame late; every close path notifies |
| R-8 | Ctrl+, opens over the details sheet, and does nothing over Settings or a dialog (A5) | — (decision 6) |
| R-9 | `show_settings` refuses under a dialog and while quitting | A route raised under a dialog is lost; today none is reachable, since routes from dialogs close them first (fact 22) |
| R-10 | Escape always closes Settings, also while a request it started runs | — (decision 6; Amendments 1–3 record the departure from `05`) |
| R-11 | Superseded by R-40 | — |
| R-12 | Settings scrolls its own `ContentScroll::region()`, laid over the Kit body as the details sheet does | — |
| R-13 | A typed passphrase is emptied on close, when the page leaves Sharing & keys, and when the other form is chosen (B3) | — (D2 rule; decision 6) |
| R-14 | One chosen repository: the route's root, else a changed panel selection, else the previous choice (C1) **(owner's eye)** | A plain open shows the chosen repository's identity form and Remove at once, which today's Repositories page did not |
| R-15 | The Remotes page's own repository list goes; Manage chooses for both sections **(owner's eye; decision 10)** | Choosing a repository for its remotes is now in the Repositories list |
| R-16 | Sharing & keys holds the key section, then Trusted servers with Check host on the chosen repository | Check host's repository is chosen on the other page |
| R-17 | Key rows as decision 2, with the trash's wording of decision 11 | — |
| R-18 | Make a new key and Use a key I already have… show today's forms, one at a time, neither at first (C2.2) **(owner's eye)** | Generating or importing takes one more press than today |
| R-19 | The info box uses `05`'s sentence, true by the credential RFC (fact 43) | If session handling changes, the sentence must change with it |
| R-20 | Show in Activity closes Settings quietly, then focuses Activity, showing it if hidden | — (decision 5) |
| R-21 | The panel's Settings button goes; the empty state and Add keep opening Repositories until Cycle 05 | — |
| R-22 | Key-list routes focus the in-use or first key's radio, else Make a new key, retargeting once when the read lands if focus has not moved | A slow read leaves focus on the entry |
| R-23 | Approve host… with no server named uses the attention row's repository (W-25, W-33) | — (decision 9) |
| R-24 | Routes naming a field focus it (Part D; decision 8); copy follows Part E and its rule | A copy change can break a test reading a literal (E-11) |
| R-25 | The header's description is part of the Kit title element | — |
| R-26 | No sheet footer | — (decision 4) |
| R-27 | The menu is three plain `Role::Tab` entries with ids `("settings-page", 0..3)` (R-T12a) | — |
| R-28 | The scrim closes Settings (Kit default), with A3's close | A stray click outside closes Settings and empties a typed passphrase (decision 13) |
| R-29 | Settings closes quietly when quitting and does not open while quitting | — (R-T17b for dialogs; Cycle 01 decision 7's Quit now focus) |
| R-30 | `SettingsHost` stays in `ContentArea.settings` | — |
| R-31 | Opening Settings leaves Drafts shown under the sheet **(owner's eye)** | Closing Settings opened from Drafts returns to Drafts, not the tabs |
| R-32 | `open` calls `opened` on every page; `RepositoriesPage` gains `opened` | One more read per open |
| R-33 | `open_item` clears only a `Details` owner; `wait_to_quit` closes Settings quietly | — (review round 1) |
| R-34 | While Settings is open, `focus_content` does nothing, and `apply_review`'s refocus is dropped while the owner is Settings; under the details sheet it behaves as today | A landing that should focus the item is dropped while Settings is open, as `landed_refocus` is under any sheet |
| R-35 | A page handler that moves focus on a landing does so only while Settings is open | — |
| R-36 | "Open" is owner `Settings` and an active sheet; a route arriving after the Kit closed it closes it first | — |
| R-37 | Superseded by R-51 | — |
| R-38 | The key radio is a wrapped div with `Role::RadioButton` and `aria_toggled`, not the Kit `Radio` (fact 39) | Its look is ours to keep close to the Kit's; on-screen check |
| R-39 | Fields fill the body's width up to 480 px | — |
| R-40 | The page keys are handled on the frame's root, which holds the menu and the body | — |
| R-41 | Pushes use `item sync` (owner, 2026-11-11). A main merge that it or `comment add` performs is recorded in the ledger, and the task's tests are rerun | A merge mid-Cycle can change what was tested; the rerun catches it |
| R-42 | Decision 12's two items are taken (Part G items 4 and 5) | — |
| R-43 | Cycle 03b's `choose_activity` sets `settings_origin` to its origin for one handler call, so `attend`, `row_action` and `open_route` keep their signatures | An action that opens Settings outside `choose_activity`'s call returns focus to the focused handle instead |
| R-44 | `show_settings` focuses the origin before `open_sheet`, so the Kit's own record is the origin on every Kit close | Focus visits the origin for one frame before the sheet takes it |
| R-45 | `settings_return` keeps the origin's region; `focus_lost` uses it when the origin is no longer drawn | Focus lands on the region's own target, not the control nearest the old origin |
| R-46 | While Settings is open and no dialog is, the Workspace pulls focus found outside the sheet layer back to the page's entry on each render | A control behind the sheet can hold focus until the next render |
| R-47 | `SettingsHost`'s `open` flag is shared with every page; a landing moves focus or opens a dialog only while it is set | — |
| R-48 | `submit_pending` is cleared when the host hides and by `inspect_root` | Add pressed and then Settings closed before the folder check lands adds nothing; the person presses Add again |
| R-49 | Not used, to avoid confusion with Cycle 03b's R-49 | — |
| R-50 | Not used, to avoid confusion with Cycle 03b's R-50 | — |
| R-51 | `show_settings` drops a requested details sheet | A chip clicked in the same frame as Ctrl+, opens nothing |
| R-52 | Every quit path closes Settings quietly before it asks or waits | — |
| R-53 | `Hosts(root)` routes always focus Check host, drawn disabled when it cannot run | — |
| R-54 | The Generate form is shown whenever the Keys page is retrying | — (R-T15a) |
| R-55 | `Inspect` does not change the chosen repository | — (today's `inspect` leaves the managed root alone) |
| R-56 | Forgetting the chosen repository (removed, or no longer listed) unchooses it for every page | — |

## Known Limits

- **K-1.** Key rows show no passphrase note, creation date or algorithm,
  and trusted servers no date. The library records none of them (fact 40;
  decision 2).
- **K-2.** There is no "Pause all checking" switch (fact 44).
- **K-3.** `Shown::Tabs` means "Settings is closed" (R-3).
- **K-4.** The design's General, Your name & email and Appearance pages
  are not shown. Name and email stay per repository on the Repositories
  page.
- **K-5.** A trusted server whose key changed is handled by today's Check
  host and Replace…, which show the old and new fingerprints side by side.
  The design leaves this undrawn.
- **K-6.** The Kit sheet's top margin is the Kit title bar's 34 px, not
  the app's 38 px (fact 36), as for the details sheet today. It is checked
  on screen.
- **K-7.** Ctrl+, over the details sheet keeps every edit. A relationship
  picker's typed search text is not an edit, and is dropped when the
  details sheet closes, as it is today on any close (fact 32).

## Unverified

- **U-1.** Settled by review round 1 (fact 26).
- **U-2.** Settled by review round 1 (fact 39; R-38).
- **U-3.** A two-line element in `Sheet::title` lays out above the Kit's
  close button. Checked on screen (Task 9's checklist).
- **U-4.** Settled by review round 1 (fact 28's reveal).
- **U-6.** A focus change made by the Kit's deferred restore (fact 46)
  brings a Workspace render, so R-46 runs on it. Task 2 checks it with
  P23's last row; if not, the Workspace observes the window's focus and
  notifies itself.
- **U-7.** A focus handle tracked on the div around Root's sheet layer
  contains the sheet's focus, the Kit's close button included (fact 26).
  Task 2 checks it.
- **U-5.** After Cycle 03b merges:
  - where `choose_activity` calls `attend`, `row_action` and
    `open_route`, so that it can set `settings_origin` around them (R-43);
  - that its popover closes when a sheet opens;
  - that W-12 to W-35's lines and triggers still hold, and which 03b
    triggers Part D misses.

  Checked in Task 0.

## Alternatives Rejected

- **A Workspace-owned layer** like Cycle 03b's popover. It would need its
  own trap, scrim and Escape, which the Kit sheet has; `08` maps Settings
  to the Kit sheet.
- **The Kit's `Settings` component** (`K/setting/settings.rs`). Ruling
  R-T12a forbids it, and its keyboard model is unread.
- **The Kit `Radio`.** Its focus is private (fact 39).
- **Renaming `Shown`'s variants.** That touches 21 test files for no
  behaviour (R-3).
- **A library ticket for the key data first.** The owner chose to leave
  the data out (decision 2).
- **A split line.** The Wave defines none for Cycle 04, and a split
  would need its own gates and review.

## Risks

| Risk | Control |
| --- | --- |
| Cycle 03b changed the footer, `attend`, `render.rs` and `focus.rs` after these citations | Implementation starts after 03b merges; Task 0 re-maps every citation and re-runs the E sweep (U-5) |
| Test churn larger than listed | The helpers change in one place (E-5). Task 0's sweep records every further site (decision 7). Tasks 1 and 5 run every touched test file once by module, a doubt the plan names |
| Focus lands somewhere unexpected after a close | P5's table, read inside the same update |
| A landing or `focus_content` pulls focus behind the sheet | R-34, R-35; P22, P23 |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-13 | Plan Cycle 04 now; execute after 03b lands | Owner direction | Decision 1 | Session, 2026-11-13 |
| Planning research (four read-only agents, checked against source) | Facts that shaped the questions: the workspace closes foreign sheets; the R-60 guard and its test conflict with Wave decision 6; one shared scroll; passphrase clearing is tied to the content page; the Kit's focus record when a sheet is replaced; key and host data the library lacks; the design's footer, nav and Pause all, against the Wave and D6. | Facts; owner questions; rulings | Facts; decisions 2–5; R-2, R-6, R-7, R-8, R-12, R-13 | Session, 2026-11-13 |
| Owner, 2026-11-13 | Key list; fold-in; footer; Show in Activity | Owner decisions | Decisions 2–5 | Session, 2026-11-13 |
| Owner, 2026-11-13 | The outline, with its stated constraints | Owner decision | Decision 6 | Session, 2026-11-13 |
| Review round 1, source and feasibility (revision 1, `65c898a5`) | 0 Blocking, 10 Important, 9 Minor. Important: quitting and `open_item` bypass the close; the chip origin; route focus lost to `open_sheet`; the passphrase wiped on Hosts routes; where the page keys are handled; unnamed test breaks; W-33; `focus_content` and async refocus; test accessors. Minor: the details request over Settings, Show in Activity's timing, `RepositoriesPage::opened`, the copy table, the radio, field widths, P7's keys, `workspace.rs`, fact 29's picker | Important, Minor | Revision 2: A1 cases 1–5, A2 steps 2–7, A3 "Quietly", A5, B1–B3, C1–C2, Part D W-33, Part E; R-33 to R-40; facts 4, 10, 17, 21, 26, 29–32, 39; E-12 to E-16 | Review report, 2026-11-13 |
| Review round 1, contract and consistency (revision 1, `65c898a5`) | 2 Blocking, 16 Important, 17 Minor. Blocking: no keyboard proof of the routes; Part D missed 03b's triggers. Important: 03b's focus fallbacks; quit focus; the passphrase predicate; Cycle 05's dependency; Escape against `05`; interruptions; contract 6's exceptions; decision 3's attribution; privacy proof; focus trap; scroll leak; named edits; Task 1's size; pushes; a pending details request; decisions 9 and 11. | Blocking, Important, Minor; owner questions | Revision 2: P19–P23, W-34, W-35, Part D's origins, A3, A4, B3, contract 6, Part G, Amendments 1–7, E-12 to E-18, R-41; Task 1 split into Tasks 1 and 2; the split line dropped; decisions 12 and 13 | Review report, 2026-11-13 |
| Owner, 2026-11-13, after review round 1 | Decision 3's attribution and two more items; the scrim | Owner decisions | Decisions 12 and 13 | Session, 2026-11-13 |
| Review round 2, scoped re-review (revision 2, `2574a277`) | 0 Blocking, 5 Important, 18 Minor. Important: P3 needs the key's guard in Task 1; E-13 needs R-14 in Task 3; no route for W-5 and W-6; origins from Activity, the popover and inside Settings not wired; Task 5 and later behaviour changes run no module. Every round-1 Blocking and Important finding was resolved | Important, Minor | Revision 3: `AddRepository`; A2's origin order and `settings_origin`; W-31 and W-32 inside the host; the plan's task moves and module runs; Part E's kept `REPOSITORY_HEADING`, `IMPORT_NOTE` and `…`; facts 32, K-7; E-1, E-12, E-13 | Review report, 2026-11-13 |
| Review round 2, adversarial on the close and focus mechanics (revision 2, `2574a277`) | 1 Blocking, 7 Important, 8 Minor. Blocking: focus can land behind the open sheet (the Kit's deferred restore; a route target not drawn) and the keyboard cannot get it back. Important: the Kit's record is not the origin; the wrappers drop the origin; an undrawn origin always lands in Content; late landings beyond A4's list; the chosen repository never unchosen; the `Hosts` focus undecidable at arrival; `Inspect` changing the choice | Blocking, Important, Minor | Revision 3: A6's table of required outcomes; R-43 to R-48, R-51 to R-56; facts 46, 47; U-6, U-7; P1, P4–P7, P9, P10, P22, P23 | Review report, 2026-11-13 |
