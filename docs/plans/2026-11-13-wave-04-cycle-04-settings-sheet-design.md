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

This design covers [Cycle 04](../Cycles/wave-04-cycle-04-settings-sheet.md):
Settings stops being a page drawn in place of the open item and becomes the
design's 720 px sheet on the right. Its left menu has three pages:

- **Repositories:** today's Repositories and Remotes pages, with the Git
  identity form.
- **Sharing & keys:** today's Shared SSH keys page, in `05`'s row layout
  (decision 2), and Trusted servers from today's Hosts page.
- **Checking for updates:** D6's page.

Every existing way into Settings opens the sheet at its page and control.
Opening and closing Settings leaves the open item, its scroll position and
any draft as they were. The Cycle also takes three items from ticket
`01M5F3GVDYTH1Q5V5FCS9YF7V3` that sit in the code it rewrites (decision 3,
Part G).

Approval covers the mechanisms, rulings, named test edits and task order.
It does not authorize implementation, pushes beyond the ticket branch,
merging or closing. **Implementation waits until Cycle 03b
(`01M784Y94Q3AXT6XJX38F5VK1V`) is merged to main** (decision 1). Task 0
then merges main and re-checks every citation below against it (U-5).
After approval, a note goes on `01M5F3GVDYTH1Q5V5FCS9YF7V3` saying which
items this Cycle takes; that is the planner's step 9 work, not a Document
Amendment.

Status: **draft**, revision 1, not yet independently reviewed.

## Conventions

- `D/` is `src/desktop/`. `K/` is
  `gpui-component-0.6.6/src` and `KB/` is `gpui-base-0.6.6/src`, both under
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. `gpui_kit::component`
  re-exports gpui-component (`gpui-kit-0.6.6/src/lib.rs:143`).
- Citations are at main `ba95e1b7` (this branch `0b7a8f3d` changes only
  `ticket.md`). Cycle 03b changes `D/view/operations/`, the footer, the
  keymap, `D/view/workspace/focus.rs` and `render.rs`, so Task 0 re-maps
  every citation in those files after 03b merges.
- "Fact n", "W-n" (a way in), "P-n" (a required outcome), "E-n", "R-n",
  "K-n" and "U-n" refer to this design. "Decision n" and "A-n" refer to
  the Cycle document.

## Baseline Facts At `ba95e1b7`

### Settings today

1. **The host.** `SettingsHost` (`D/view/settings/mod.rs:90-109`) holds
   `services`, a `scroll: ContentScroll`, `shown: Shown`, one focus handle
   per page-list entry (`entries`), `close` (the wrapper around "Back to
   tabs"), five lazily made page entities (`repositories`, `remotes`,
   `keys`, `hosts`, `polling`), the polling subscription and
   `selection: Option<PathBuf>`. Its module doc (`:1-18`) records ruling
   R-B4: Settings is shown "in the main window's content region in place of
   the tabs".
2. **Pages.** `SettingsPage` (`D/model/stamp.rs:18-26`) is `Repositories,
   Remotes, Keys, Hosts, Polling`. `PAGES` (`mod.rs:55-61`) orders them, and
   `page_title` gives "Repositories", "Remotes", "Shared SSH keys", "Hosts"
   and "Checking for updates" (`mod.rs:66-70`). `ViewScope::Settings(SettingsPage)`
   is one result-generation scope per page (`stamp.rs:1-6`).
3. **What tests read.** `Shown` (`mod.rs:76-79`) is `Tabs |
   Settings(SettingsPage)`, read through the test-only
   `Workspace::settings_shown` (`D/view/workspace/content.rs:745-747`),
   called from 21 test files. `SettingsEvent` (`mod.rs:82-89`) is `Closed |
   ShowActivity`.
4. **The host's API.** `open(page)` (`mod.rs:145-169`) sets `shown` and
   calls `opened(…)` on Remotes, Keys, Hosts and Polling, but not on
   Repositories. `open_remotes_on` (`:171-183`), `set_selection` (`:185-187`),
   `forget_passphrase` (`:191-198`), `hide` (`:202-207`), `first_focus`
   (`:211-218`, the shown page's entry), `set_up_sharing` (`:299-305`),
   `check_host` (`:309-320`), `retry_generation` (`:324-335`), `inspect`
   (`:354-364`), `identity` (`:367-376`), `back` (`:378-381`).
5. **The page list** (`mod.rs:384-428`) is an `h_flex().flex_wrap()` of
   plain divs with `Role::Tab`, `aria_selected`, a "Shown" tag and ids
   `("settings-page", ix)`. It does not use the Kit's `Settings` component,
   by ruling R-T12a (D1 comment
   `.manyhands/comments/01M4CC0VMY5RBMP70QVBXWXM6A/01M5CRVEAP42DDKYBC63A66TRV.md:80`).
6. **Render** (`mod.rs:431-474`) first calls `forget_passphrase`, then draws
   the heading "Settings", the "Back to tabs" button (`settings-close`,
   `settings-close-button`), the page list and the page.
7. **Wrapped controls.** `wrapped(id, label, focus, scroll, control)`
   (`mod.rs:482-497`) puts a control in a focus-tracking wrapper with a
   scroll marker, so a control taking focus below the fold is revealed.
8. **Owner.** `ContentArea.settings: Entity<SettingsHost>`
   (`D/view/workspace.rs:71`) is built at `:285-289` with
   `tabs.scroll().clone()`. The Workspace subscribes at `:290-296`: `Closed`
   focuses `Region::Content`, `ShowActivity` focuses `Region::Operations`.
9. **The content page.** `ContentPage { Tabs, Settings, Drafts }`
   (`content.rs:39-43`) is chosen by `page()` (`:46-54`), Settings first.
   `show_tabs` (`:56-59`) hides Settings and Drafts. `content_body`
   (`:662-700`) draws only the `SettingsHost` while Settings is shown, and
   on the Tabs and Drafts branches calls `forget_passphrase`. `shown_item`
   (`:630-635`) is `None` while Settings is shown.
10. **Opening.** `open_settings(page)` (`content.rs:194-202`) uses the
    Repositories panel's selection; `open_settings_on(page, root)`
    (`:206-220`) also calls `manage_root(root, false)` for Repositories;
    both go through `open_settings_at` (`:222-235`), which hides Drafts,
    sets the selection, opens the page and focuses `Region::Content`.
    `inspect_in_settings` (`:237-248`) focuses `Region::Content` too;
    `identity_in_settings` (`:250-261`), `check_host_in_settings`
    (`:263-275`) and `retry_generation_in_settings` (`:277-288`) only set
    `last_region` and leave focus to the page. `open_remotes`
    (`D/view/workspace/requests.rs:285-297`) and the private `open_hosts`
    (`:217-225`) open Settings directly, bypassing `open_settings_at`.
11. **Leaving.** "Back to tabs" runs `back`, which hides and emits `Closed`.
    `open_item` (`content.rs:336-342`), `show_tab`, ctrl-tab and
    `open_drafts` (`:77-91`) also hide Settings. Nothing binds Escape for
    Settings.
12. **What survives today.** Tabs, the shown tab, each `ItemPane` (its edit
    session and draft) and its own scroll region are untouched by opening
    Settings (`D/view/settings/mod_tests.rs:73-118`;
    `D/view/item/frame_tests.rs:148-178`). The Keys page's masked inputs are
    emptied whenever Keys is not the shown page (fact 9; `mod.rs:191-198`).
13. **One scroll.** Settings, the Drafts page and the empty pane scroll the
    content region's column, `tabs.scroll()` (`content.rs:625-627`;
    `D/view/scroll.rs:1-22`). An item tab scrolls a region made by
    `ContentScroll::region()` (`scroll.rs:59-68`), which shares the column's
    focus-reveal state. The column's `note_focus` runs once a frame in
    `item_area_region` (`D/view/workspace/render.rs:121-122`). This is the
    shared offset `01M5F3GVDYTH1Q5V5FCS9YF7V3` reports.
14. **Page keys.** PageUp, PageDown, Home and End are bound in the
    `Content` context (`D/keys.rs:308-311`). The Workspace handles them
    through `unless_dialog` (`render.rs:286-297`), scrolling
    `scroll_target` (`content.rs:639-644`): the shown item's region, else
    the column.

### The pages

15. **Repositories** (`D/view/settings/repositories.rs`,
    `repositories/manage.rs`). Render order (`repositories.rs:776-794`):
    the managed section (registrations, each with Manage
    `("managed-choose-button", ix)`, then the identity form and Remove for
    the managed root), the Add form, the Create form. `manage_root(root,
    focus)` (`manage.rs:308-327`) sets the managed root and **empties the
    identity inputs every time**, then focuses `identity-name` when `focus`.
    `inspect_root` (`repositories.rs:436`) fills and checks `add-path`
    without moving focus. Dialogs go through `view::confirm::confirm`.
16. **Remotes** (`D/view/settings/remotes.rs`). Its own registrations list
    (`:608-640`), each row with Choose (`choose_root`, `:486`); the chosen
    repository's remotes; the Add form (`add_form`, `:793-822`), drawn
    whenever a root is chosen, with `remote-add-name` and `remote-add-url`.
    `opened(selection)` (`:281-289`) switches root only when it has none;
    `open_on(root)` (`:292-298`) always switches. Remove goes through
    `confirm(RemoteRemove)`.
17. **Shared SSH keys** (`D/view/settings/keys.rs`, `keys/render.rs`,
    `keys/commands.rs`, `keys/reads.rs`). Each key row (`render.rs:91-163`)
    shows a kind ("Generated by Manyhands" / "Imported"), Select,
    Unregister, and Delete key on generated keys only; "Clear selection"
    sits under the list (`render.rs:165-200`). Below are the inline Generate
    form (label, passphrase, confirmation) and Import form (label, private
    and public file). `ask_again(id)` (`commands.rs:111`) focuses
    `key-generate-passphrase` for a retry, returning early under a dialog.
    Copy is in `keys.rs:37-74`.
18. **Hosts** (`D/view/settings/hosts.rs`). Render (`:825-835`): the pin
    list ("Host pins"), then Check host for the selected repository. There
    is no repository picker: `opened(selection)` (`:313-320`) works on the
    host's `selection`. `check_host(root, authority)` (`:433-449`) returns
    early under a dialog, else chooses the root and starts a check with
    focus on `hosts-check`. Copy is in `hosts.rs:67-83`;
    `NO_REPOSITORY` says "Choose a repository in the sidebar…" and
    `NO_SSH_PUBLICATION` says "… Choose one on the Remotes page."
19. **Checking for updates** (`D/view/settings/polling.rs`). Emits
    `PollingEvent::{SetUpSharing, Attend, ShowActivity}` (`:85-92`). A row
    with a pending operation draws "Show in Activity" (`SHOW_IN_ACTIVITY`,
    `:76`; button at `:418-427`). The host answers `Attend` in
    `D/view/settings/attention.rs:11` (Keys, `check_host`, Hosts, or
    `set_up_sharing`).
20. **Handlers check dialogs, not sheets.** Every Settings page handler
    tests `window.has_active_dialog`, which counts dialogs only
    (`K/window_ext.rs:161`).

### Ways in

21. **Every call into Settings** outside tests is listed in Part D with its
    file and line: 32 triggers through 9 entry functions. The Repositories
    panel's own Settings button (`sidebar-settings`,
    `D/view/repositories/body.rs:129-141`) is one of them.
22. **Routes from a dialog close the dialog first.** The lifecycle dialog's
    `in_workspace` (`D/view/lifecycle/actions.rs:126-140`) calls `close`
    (`D/view/lifecycle.rs:504-507`: `close_dialog`, then focus its
    `origin`) before calling the Workspace. Item routes go through
    `in_window` (`requests.rs:177-186`); the save banner's Open Settings
    goes through `settings_for` and opens on the next frame
    (`content.rs:115-120`, `:145-146`).
23. **The footer.** `Control::Settings` (`D/view/footer.rs:77`, id
    `footer-settings` at `:92`, label `SETTINGS` at `:44`, tooltip action
    `OpenSettings` at `:327`). `press` (`D/view/workspace/footer.rs:161-178`)
    returns early while a dialog or sheet is open, then sends Settings to
    `open_settings(Repositories)` and the "Checked for updates" item to
    `open_settings(Polling)`.
24. **The key.** `KeyBinding::new("secondary-,", OpenSettings, None)`
    (`D/keys.rs:281`), handled at `render.rs:262-264` inside
    `unless_dialog`.

### The sheet layer and R-60

25. **Where the sheet is drawn.** `item_area_region`
    (`render.rs:117-152`) appends `Root::render_sheet_layer` inside the
    `Region::Content` element, deferred. Its comment (`:130-135`): the
    sheet's focus "counts as the region's", its keys pass the region's
    context first, and the sheet places itself over the whole window.
26. **R-60.** `modal_open` is `has_active_dialog || has_active_sheet`
    (`render.rs:155-158`); `unless_dialog` (`:160-171`) drops every
    Workspace action under it except Quit and Quit now. The footer, the
    dock menu and the toolbar check it too (`footer.rs:167`;
    `D/view/dock/header/toolbar.rs:209`). The test
    `no_workspace_key_acts_behind_the_sheet`
    (`D/view/item/sheet_tests.rs:670-741`) asserts that `secondary-,`
    behind the details sheet leaves `Shown::Tabs`.
27. **The details sheet.** `open_details` (`D/view/item/sheet.rs:145-163`)
    focuses the chip first, because Root returns focus on close to what
    held it when the sheet opened, then `window.open_sheet(…)`, then the
    field. Its body is its own absolute `inset_0` scroll over the Kit body,
    with its own `ScrollHandle` and scrollbar (`sheet.rs:251-275`).
28. **Who owns the sheet.** `ContentArea.sheet:
    Option<WeakEntity<ItemPane>>`. `Workspace::sync_sheet`
    (`content.rs:307-334`), called on every render (`render.rs:179`),
    **closes any active sheet not owned by the shown item pane**, then opens
    a requested details sheet while no dialog is open. `open_item` clears
    the owner (`content.rs:340`); `wait_to_quit`
    (`D/view/workspace/lifecycle.rs:226-238`) closes dialogs and any sheet.
29. **Details sheet edits are drafts.** Each field change goes through
    `change_fields` into the shared drafts registry
    (`D/view/item/sheet/fields.rs:163-198`; `D/view/item/edit/feed.rs:30-63`),
    and the sheet's builder holds only a `WeakEntity<ItemPane>`, so closing
    the sheet drops no edit. `each_named_control_edits_its_field_and_marks_the_tab_dirty`
    (`D/view/item/sheet/fields_tests.rs:255-339`) closes with Escape and
    finds every field in the draft. An open relationship picker's typed
    filter is dropped on close (`D/view/item/relations_edit/picker.rs:266-275`).

### GPUI Kit 0.6.6

30. **Calls.** `open_sheet`, `open_sheet_at(placement)`, `has_active_sheet`
    and `close_sheet` (`K/window_ext.rs:14-27`, `:111-138`). Root keeps one
    slot, `active_sheet: Option<ActiveSheet>` (`K/root.rs:40`).
31. **Replacing and closing.** `open_sheet_at` (`K/root.rs:379-406`) keeps
    the replaced sheet's `previous_focused_handle`, or else the focused
    handle, then focuses a new Root-owned handle. `close_sheet`
    (`:410-423`) focuses that previous handle if it still exists, then
    clears the slot. It does not look at dialogs.
32. **Rebuilt each frame.** `render_sheet_layer` calls the stored builder
    on every render (`K/root.rs:215-240`), so a sheet's state must live in
    entities its builder captures.
33. **The builder** (`K/sheet.rs:59-119`): `title(impl IntoElement)`,
    `footer`, `size` (default 350 px), `resizable` (stored, unused),
    `overlay` and `overlay_closable` (both default true), `on_close`. The
    header always draws a ghost close button that dispatches `Cancel`
    (`K/sheet.rs:200-208`). There is no description builder. The top margin
    is the theme's `sheet.margin_top`, default the Kit title bar's 34 px
    (`K/sheet.rs:32-37`, `:144`); the app's title bar is 38 px
    (`D/view/shell.rs:18`).
34. **Escape and the trap.** `escape` is bound to `Cancel` in the `Sheet`
    context (`KB/sheet.rs:23-25`). The host traps focus
    (`KB/sheet.rs:133-134`; Root's Tab handlers, `K/root.rs:492-560`) and
    handles `Cancel` by propagating, then closing (`KB/sheet.rs:135-138`).
    Closing runs `request_close` (`window.close_sheet`) then `on_close`; a
    direct `window.close_sheet` does not run `on_close` (`KB/sheet.rs:17-21`).
35. **No app `Cancel` handler.** No code under `D/` handles the Kit's
    `Cancel` action (grep of `_: &Cancel`), so the propagated `Cancel`
    reaches nothing of the app's.
36. **Dialogs over a sheet.** `open_dialog` does not check for a sheet; it
    records the focused handle and focuses its own (`K/root.rs:297-323`).
    Dialogs are deferred at priority 10 and above (`KB/dialog.rs:541`), the
    app's sheet layer at 0 (fact 25). `close_dialog` focuses the dialog's
    previous handle (`K/root.rs:325-339`). A Settings page's confirmation
    therefore opens above the sheet and gives focus back inside it.
37. **Radio.** `Radio` (`K/radio.rs:19-130`) has `checked`, `on_click`,
    `accessibility_label`, `tab_stop` and its own keyed focus handle
    (`:174-201`).

### Data and authorities

38. **What the library gives a key row.** `KeyDto`
    (`src/repository/read/dto.rs:247-258`): `id`, `label`, `ownership`,
    `selected`, `fingerprint`, the source and metadata states and the file
    paths. No passphrase flag, no creation date, no algorithm. `HostPinDto`
    (`:294-300`): `host`, `port`, `pinned_algorithm`, `pinned_fingerprint`,
    `reapproval_required`; no date.
39. **The design.** `05` "Settings sheet" (`docs/design/05-sheets-and-dialogs.md:40-66`):
    720 px, opened from the footer gear and Ctrl ,, a 214 px left nav whose
    designed pages are Sharing & keys, Checking for updates and Appearance;
    the key list, the info box and Trusted servers (`:46-59`); Checking for
    updates with a global "Pause all" switch (`:61-66`). The common frame
    (`:7-12`) has a header with title, description and close, a scrolling
    body and a footer. Frames `1e` and `1f` draw Settings with no footer
    and the header description "Applies to this computer only."
    (`docs/design/mockups/source/Workspace.dc.html:1123`); every page but
    keys, polling and appearance is "not mocked this round" (`:475`).
40. **Vocabulary.** `06` maps publication remote to **shared copy**, SSH
    key to **key** / "Key used for sharing", known host pin to **Trusted
    servers**, background poll to **Checking for updates**
    (`docs/design/06-language-and-states.md:8`, `:15`, `:24`, `:25`), and
    asks for `…` on any action that opens a confirmation or sheet (`:36`).
41. **Passphrases.** The credential RFC keeps a protected key's passphrase
    "only in that provider's memory for the remaining session" and asks for
    it on first use in a session
    (`docs/RFC/authentication-and-credential-handling.md:145`, `:275`). So
    `05`'s info box, "asks for it once each time you open the app and never
    stores it", is true of today's handling.
42. **No Pause all.** D6 decision 8 rejected a global pause switch and
    recorded the handoff's as a Known Limit
    (`docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md:428-430`).
43. **Key settings behaviour.** The desktop IA RFC requires generation,
    import, selection, clearing and unregistration, with generated-file
    deletion "a separate confirmed action" and no private-key display or
    copy (`docs/RFC/desktop-information-architecture-and-editor.md:321-328`).

## Part A: The Sheet

### A1. Who owns the sheet

`ContentArea.sheet` becomes:

```rust
/// The sheet on Root's one sheet layer, and who keeps it open (fact 28).
pub(super) enum SheetOwner {
    /// The shown item's details sheet, as today.
    Details(WeakEntity<ItemPane>),
    /// Settings (Part B).
    Settings,
}
```

`sync_sheet` keeps a sheet owned by `Settings` whatever item is shown, and
treats `Details(pane)` exactly as today's `Some(pane)`. Every other place
that sets or clears the owner (fact 28) keeps its meaning. The details
sheet and Settings never coexist: Root has one slot (fact 30; Wave
decision 6).

### A2. One way in

Every way in calls one function:

```rust
/// Where Settings opens and what it does on arrival (Part D).
pub(crate) enum SettingsRoute {
    /// A page, focused on its menu entry, on the chosen repository.
    Page(SettingsPage),
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
    /// on close; `None` takes the focused handle.
    pub(in crate::desktop::view) fn show_settings(
        &mut self,
        route: SettingsRoute,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );
}
```

`show_settings` lives in a new `D/view/workspace/settings_sheet.rs`
(`workspace.rs` is 488 lines). Today's entry functions (`open_settings`,
`open_settings_on`, `inspect_in_settings`, `identity_in_settings`,
`check_host_in_settings`, `retry_generation_in_settings`, `open_remotes`,
`open_hosts`) stay as one-line wrappers that build a route and pass
`None`, so their callers, Cycle 03b's included, change only where Part D
says (R-5).

`show_settings`:

1. **Refuses** while a dialog is open, or while the app is quitting
   (R-9). Nothing happens.
2. **If Settings is already open**, applies the route inside it (Part D)
   and keeps the original origin. Ctrl+, and the footer item are handled
   in A5, not here.
3. **Records the origin.** The given handle, else `window.focused(cx)`.
   When the details sheet is open, its owner pane's chip for the field it
   was opened on is the origin instead, since the focused field leaves
   with that sheet (R-6).
4. **Closes the details sheet** if one is open (Wave decision 6): sets the
   owner to `None` and calls `window.close_sheet`. Its edits are already in
   the draft (fact 29).
5. **Sets the chosen repository** (R-14): the route's root, else the
   Repositories panel's selection.
6. **Opens the sheet**: sets the owner to `SheetOwner::Settings`, tells the
   host the route (`SettingsHost::arrive(route)`, Part D), then
   `window.open_sheet(cx, builder)`, whose builder captures the
   `Entity<SettingsHost>` and draws Part B's frame. Focus then goes to the
   route's target (Part D), after `open_sheet` has focused Root's own
   handle (fact 31), as `open_details` does (fact 27).

The Drafts page is not hidden (R-31): whatever the item area shows stays
under the sheet.

### A3. Closing

Settings closes by Escape, the header's close button, a click on the scrim
(the Kit's default, fact 33), "Show in Activity" (Part C3), quitting, or
`show_settings` replacing it (never: see A2 step 2). The Kit's `on_close`
does not run on every path (fact 34), so the close is detected in
`sync_sheet`: the owner is `Settings` and no sheet is active. Then
`settings_closed`:

1. sets the owner to `None`;
2. hides the host (`Shown::Tabs`), which empties a typed passphrase
   (Part B3);
3. focuses the origin if it is still drawn and no dialog or sheet is now
   open, else focuses `Region::Content` as "Back to tabs" does today
   (fact 11). A close made by "Show in Activity" skips this step (C3).

Focus moves inside the frame that detects the close, not deferred: a
focused control leaving the tree fires `focus_lost` in the same draw
(Cycle 03a ledger).

The Kit focuses its own recorded handle in `close_sheet` before
`settings_closed` runs (fact 31); step 3 then sets focus to the origin, so
the Kit's record matters only when the origin is gone.

### A4. Escape

Escape always closes Settings, also while a request it started is running
(R-10). Nothing is cancelled: the request finishes on the worker and is
reported in Activity, and the page shows its result when Settings next
opens, since the page entities are kept. The Kit propagates `Cancel` after
closing (fact 34); nothing in the app handles it (fact 35). A Kit input,
select or dialog inside the sheet that handles Escape first keeps doing so.

### A5. The guard (R-60) and the key

- Settings is a sheet, so `modal_open` (fact 26) holds while it is open:
  no workspace action, footer item, toolbar button or dock menu acts behind
  it. That is today's R-60, unchanged.
- **Ctrl+, (`OpenSettings`).** Its handler leaves `unless_dialog` for a
  guard of its own:
  - a dialog open: nothing (today);
  - Settings open: nothing (decision 6);
  - the details sheet open: Settings opens, closing it first (Wave
    decision 6; A2 step 4);
  - otherwise: `SettingsRoute::Page(Repositories)`.
- **The footer** cannot be pressed while any sheet is open (fact 23; its
  scrim takes the click), so Settings is never opened from the footer over
  the details sheet. Ctrl+, is the way.
- **The page keys** are Settings' own while it is open (Part B2).
- **Quitting.** `wait_to_quit` already closes any sheet (fact 28); the
  close is detected as A3. `show_settings` refuses while quitting (R-29).

## Part B: The Frame

### B1. Header, menu, body

- **Sheet.** `.size(px(720.))`, right placement (the default), the Kit's
  overlay and close button. No footer (decision 4).
- **Header.** `title(…)` holds a `v_flex` of "Settings" (15 px, semibold,
  as the details sheet's title, `D/view/item/sheet.rs:82`; `07`'s sheet
  title) and "Applies to this computer only." (`SETTINGS_NOTE`, secondary
  text). The Kit has no description
  builder (fact 33; R-25).
- **Menu.** A 214 px column of three entries, in order: Repositories,
  Sharing & keys, Checking for updates (`SettingsNav`, R-4). Plain controls
  as today (fact 5; R-T12a): each a div with `Role::Tab`, `aria_selected`,
  `track_focus` on its entry handle, id `("settings-page", ix)` for `ix` in
  0..3, a 14 px icon (`folder-git-2`, `key-round`, `refresh-cw`, as the
  mockup's nav, fact 39) and its label. The active entry has the `border`
  background and is announced selected; the "Shown" text tag goes, since
  the background and `aria_selected` carry it (colour is not the only
  signal: the selected state is announced). Enter, Space or a click opens
  the entry's page.
- **Body.** The shown page's view, in an absolute `inset_0` scroll laid
  over the Kit body with its own `ScrollHandle` and vertical scrollbar, as
  the details sheet does (fact 27; R-12).

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

`SettingsPage` keeps its five variants: they name the page entities and
their result scopes (fact 2), and tests read them through `Shown` (fact 3;
R-3). An entry opens its first page (`Repositories`, `Keys`, `Polling`).

### B2. Scrolling and the page keys

- **Its own scroll.** `SettingsHost` holds a `ContentScroll` made by
  `ContentScroll::region()` from the column's (R-12). It shares the
  column's focus-reveal state, whose `note_focus` already runs every frame
  (fact 13), so a control taking focus below the fold is revealed in the
  sheet. Every page, made with the host's scroll today, gets this one.
- **The page keys.** The body element handles `ScrollPageUp`,
  `ScrollPageDown`, `ScrollToTop` and `ScrollToBottom` itself, on its own
  scroll, without propagating (R-11). The keys are bound in the `Content`
  context, which the sheet's focus is inside (fact 25), so they reach the
  body before the Workspace's guarded handlers. Inside a text input, its
  own Home and End win as they do today.
- **The item keeps its place.** The column and the item's region are no
  longer Settings', so nothing in Settings moves them (fold-in, Part G).

### B3. A typed passphrase

The D2 rule, "a typed passphrase does not outlive the page being shown"
(fact 12), becomes: the Keys page's masked inputs are emptied when Settings
closes (A3 step 2) and when the menu leaves Sharing & keys (R-13).
`SettingsHost::hide` and the menu's page change call `forget_passphrase`;
the render-time call stays, so a hidden Keys page is emptied at the next
draw, as today. `content_body`'s calls go (fact 9).

## Part C: The Pages

### C1. Repositories

One page, in this order:

1. **Repositories** (today's managed section): the registrations, each
   with Manage, then for the chosen repository its identity form and
   Remove.
2. **Sharing** (today's Remotes page, without its own registrations list):
   the chosen repository's remotes and the Add a remote form.
3. **Add an existing repository** and **Create a repository**, as today
   until Cycle 05 moves them (Wave decision 10).

**One chosen repository** (R-14). `SettingsHost.selection` becomes
`chosen: Option<PathBuf>`, the repository both the Repositories page and
the Trusted servers' Check host work on. It is set:

- by every way in (A2 step 5): the route's root, else the panel's
  selection, read fresh at each open (fold-in: reopening follows a newer
  selection, Part G);
- by Manage on a row.

Each page reads it when it is shown (`opened`), the Repositories page
included (R-32). Manage and every route call one `choose(root)`, which:

- does nothing to the identity inputs when `root` is already chosen, so
  text being typed is kept (the 03a ledger's R5-2 lesson);
- otherwise does what `manage_root(root, false)` does today (fact 15) and
  points the Sharing section at `root` (today's `open_on`, fact 16).

The Remotes page's own registrations list and Choose are removed; Manage
is the one way to choose (R-15).

### C2. Sharing & keys

One page, in this order:

1. **Key used for sharing.** `05`'s intro, then the key list (bordered,
   radius 8), one row per key (decision 2; R-17):
   - a `Radio` (fact 37), checked when the key is in use; choosing it
     sends `KeySelect`, as Select does today;
   - the label, an origin tag ("Made by Manyhands" for a generated key,
     "Imported" otherwise) and an "In use" tag (with the word, not colour
     alone) on the selected key;
   - the fingerprint in mono, or today's "No fingerprint recorded";
   - today's source-state line when the private file was missing or
     unreadable (`SOURCE_MISSING`, `SOURCE_UNAVAILABLE`);
   - **Copy public key**, as today;
   - a trash icon button, "Remove from Manyhands", sending `KeyRemove`
     (today's Unregister), with the tooltip "Remove from Manyhands. The key
     file is never deleted.";
   - on generated keys only, **Delete key file…**, which opens today's
     `KeyDelete` confirmation.

   Under the list, while a key is in use, **Stop using this key** sends
   `KeyClear` (today's Clear selection). The passphrase note, the dates
   and the algorithm are left out (K-1).
2. **Make a new key** (primary) and **Use a key I already have…**: each
   shows today's Generate or Import form below, and hides the other (R-18).
   A retry of a protected generation shows the Generate form with focus on
   its passphrase field (R-T15a; W-26).
3. **The info box** (`lock` icon): "If your key has a passphrase,
   Manyhands asks for it once each time you open the app and never stores
   it." (fact 41).
4. **Trusted servers** (today's Hosts page): each pin with its host, mono
   fingerprint and the reapproval notice; then **Check host** for the
   chosen repository, its result, Approve… and Replace… as today. The
   design's "Trusted Oct 4" is left out (K-1).

### C3. Checking for updates

D6's page as it is, copy aside (Part E). No "Pause all checking" switch
(fact 42; K-2). Its Set up sharing… and attention actions stay inside the
sheet: they switch the menu and arrive as Part D's W-31 and W-32.

**Show in Activity** (decision 5): closes Settings without A3's focus step,
then reveals and focuses Activity, showing the panel if hidden, as today's
`focus_region(Region::Operations)` does (fact 8). `SettingsEvent::ShowActivity`
keeps its name; the Workspace's answer closes the sheet first (R-20).

## Part D: Ways In

`SettingsHost::arrive(route)` switches the menu to the route's page,
applies it (choosing the repository, revealing a form, starting a check)
and names the focus target. "Entry" is the page's menu entry. "Shown" is
what `settings_shown` then reads (R-3). Lines are at `ba95e1b7`; Task 0
re-maps them after Cycle 03b merges.

| W | Trigger | Today's call | Route | Shown | Focus |
| --- | --- | --- | --- | --- | --- |
| W-1 | Ctrl+, (`render.rs:262-264`) | `open_settings(Repositories)` | `Page(Repositories)` | `Settings(Repositories)` | entry |
| W-2 | Footer Settings (`workspace/footer.rs:174`) | same | same | same | entry |
| W-3 | Footer "Checked for updates" (`workspace/footer.rs:175`) | `open_settings(Polling)` | `Page(Polling)` | `Settings(Polling)` | entry |
| W-4 | Repositories panel Settings button (`repositories/body.rs:139`) | `open_settings(Repositories)` | removed (Part F) | — | — |
| W-5 | Empty Repositories panel "Add or create a repository…" (`repositories/body.rs:113`) | `open_settings(Repositories)` | `Page(Repositories)` | `Settings(Repositories)` | `add-path` (R-24) |
| W-6 | Repositories toolbar Add (`dock/header/toolbar.rs:217-218`) | same | same | same | `add-path` |
| W-7 | Repository menu Settings… (`workspace/repository_menu.rs:221-222`) | `open_settings_on(Repositories, root)` | `Manage(root)` | `Settings(Repositories)` | entry |
| W-8 | Row action Settings… (Needs fixing; `repository_menu.rs:245-246`) | same | same | same | entry |
| W-9 | Row action Set up sharing… (`repository_menu.rs:248`) | `open_remotes(root)` | `Remotes(root)` | `Settings(Remotes)` | `remote-add-name` |
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

Notes:

- **Focus targets.** Today, routes with no field focus the page's entry
  (fact 10). The Wave asks for "the right page and field", so the routes
  above that name a field focus it (R-24). `Page` routes keep the entry.
- **The key list** (W-12, W-16, W-21, W-23; R-22). Focus goes to the
  in-use key's radio, else the first key's, else Make a new key. While the
  keys are still being read, focus goes to the entry, and moves to the
  target when the read lands only if the entry still holds focus.
- **`hosts-check`** for `Hosts(root)` (W-13, W-17, W-20, W-25): Check host
  for the chosen repository, or the Trusted servers heading's first
  control when the repository has no shared copy over SSH (today's
  `NO_SSH_PUBLICATION` line, Part E).
- **W-25's fix** (R-23). Today, Approve host… with no server named opens
  Hosts on the Repositories panel's selection, not the attention row's
  repository (`attention.rs:151`). It now uses the row's repository, as
  W-32's path does (`settings/attention.rs`).
- **Inside the sheet** (W-31, W-32), the origin is the one recorded when
  Settings opened (A2 step 2).
- **Cycle 03b.** Its popover and Activity rows run today's `attend` and
  `open_route` handlers through `choose_activity`, passing their own focus
  handle as origin (03b design C3, E2). Task 0 confirms how its merged code
  passes that origin, and those calls pass it to `show_settings` instead of
  `None` (U-5). The popover closes when a sheet opens (03b design E2), and
  a sheet it opened returns focus to the footer item.

## Part E: Copy

Copy follows `06` (fact 40). Every string is a `pub(crate) const`; tests
compare constants. The changes:

| Today (constant, file) | New |
| --- | --- |
| `BACK_TO_TABS` "Back to tabs" (`settings/mod.rs:52`) | removed: the header's close button replaces it |
| `page_title` "Remotes", "Shared SSH keys", "Hosts" (`mod.rs:66-70`) | menu: "Repositories", "Sharing & keys", "Checking for updates"; section headings "Sharing" and "Trusted servers" |
| — | `SETTINGS_NOTE` "Applies to this computer only." |
| `KEYS_HEADING` "Shared SSH keys" (`keys.rs:37`) | "Key used for sharing", with `05`'s intro sentence |
| `KIND_GENERATED` "Generated by Manyhands" (`keys.rs:42`) | "Made by Manyhands" |
| `SELECTED` "Selected" (`keys.rs:48`) | "In use" |
| `SELECT` "Select" (`keys.rs:52`) | the radio, labelled "Use {label} for sharing" for its accessible name |
| `CLEAR` "Clear selection" (`keys.rs:53`) | "Stop using this key" |
| `UNREGISTER` "Unregister" (`keys.rs:54`) | "Remove from Manyhands" (trash button's name and tooltip text) |
| `DELETE` "Delete key" (`keys.rs:55`) | "Delete key file…" |
| `GENERATE_HEADING`, `IMPORT_HEADING` (`keys.rs:38-39`) | the buttons "Make a new key" and "Use a key I already have…"; the forms keep their headings |
| — | `KEYS_NOTE`, the info box (C2.3) |
| `PINS_HEADING` "Host pins" (`hosts.rs:67`) | "Trusted servers" |
| `NO_PINS` "No host is pinned yet." (`hosts.rs:70`) | "No server is trusted yet." |
| `REAPPROVAL` (`hosts.rs:71-72`) | "The list of trusted servers was lost: approve each server again." |
| `NO_REPOSITORY` "…in the sidebar…" (`hosts.rs:73-74`) | "Choose a repository on the Repositories page to check the server it shares through." |
| `NO_SSH_PUBLICATION` "…Choose one on the Remotes page." (`hosts.rs:76-77`) | "Check host is unavailable: this repository's shared copy is not reached over SSH. Choose one under Repositories › Sharing." |
| `CHECK_NOTE` "…the publication remote…" (`hosts.rs:79-80`) | "Check host connects to the server this repository's shared copy is on and shows the key it presents. It trusts nothing yet." |
| Strings in `remotes.rs` saying "publication remote" | "shared copy", by the rule below |

The rule for the rest (R-24's copy half): a Settings string that uses a
term `06` replaces (publication remote, SSH key, host pin, Hosts page,
Remotes page, sidebar) is rewritten with `06`'s term and the new place
names; a string that does not is unchanged. Git detail (a remote's name,
a branch) may stay in Settings, which is a sheet (`06` header). Task 7
lists every rewritten constant in the ledger, and P18 checks that no
Settings constant still uses a replaced term.

## Part F: What Leaves

- `ContentPage::Settings`, its branch in `content_body`, `focus_content`
  and `shown_item` (fact 9), and the column's use by Settings.
- `SettingsEvent::Closed`, `back`, the `close` handle and "Back to tabs"
  (fact 6).
- The Repositories panel's Settings button, `sidebar-settings`
  (`repositories/body.rs:129-141`) (Wave scope; 03a decision 15).
- `open_item`, `show_tab`, ctrl-tab and `open_drafts` no longer hide
  Settings: none can act while the sheet is open (A5).
- The module doc's R-B4 paragraph (fact 1) is rewritten for the sheet.

The empty Repositories panel's action and the toolbar's Add keep opening
Settings › Repositories until Cycle 05 moves Add repository to its own
sheet (W-5, W-6).

## Part G: Folded-In Work

From `01M5F3GVDYTH1Q5V5FCS9YF7V3` (decision 3), the items in code this
Cycle rewrites:

1. **"Settings and tabs share one scroll offset."** Settings scrolls its
   own region (B2). P8.
2. **"Reopening Settings ignores a sidebar selection made since the last
   open."** Every open sets the chosen repository from the route or the
   panel's selection (C1). P10.
3. **Repositories is not read again on reopen** (fact 4). `open` calls
   `opened` on every page (R-32). P11.

The ticket's Add repository items (Browse, folder inspection, non-UTF-8
roots) wait for Cycle 05; the rest stays in that ticket.

## Part H: Keyboard, Focus And Privacy

- Every control in the sheet is reached by Tab within the Kit's trap
  (fact 34). Shift-Tab from the first control wraps to the last.
- Focus returns as A3 says. P5 covers each kind of origin.
- A dialog a page opens (a confirmation, the passphrase prompt) sits above
  the sheet and gives focus back inside it (fact 36).
- No new text names a passphrase, private key, remote URL or file path
  beyond what today's pages show. No action displays or copies a private
  key (fact 43). The trash and Delete key file… tooltips name no path.

## Required Outcomes

Each is a test, written first. Window tests use the shell fixtures of
today's `settings/*_tests.rs`.

| P | Test | Asserts |
| --- | --- | --- |
| P1 | `settings_is_a_sheet_the_workspace_keeps` | After Ctrl+,, a sheet is active and stays active across frames with an item shown, another tab shown, and Drafts shown under it; the details sheet is still closed when its tab is not shown (fact 28) |
| P2 | `settings_leaves_the_item_as_it_was` | With an item open, its region scrolled and a field edited into a draft: Ctrl+,, then Escape. The same tab is shown, `tab_keys` unchanged, the item's offset unchanged, the draft holds the edit (Wave exit evidence) |
| P3 | `settings_replaces_the_details_sheet_and_keeps_its_edits` | Details sheet open with a field edited: Ctrl+, opens Settings, the details sheet is gone, the draft holds the edit; Escape returns focus to the chip (Wave decision 6; R-6) |
| P4 | `each_way_in_lands_on_its_page_and_control` | Table over Part D, W-1 to W-32 less W-4: `settings_shown`, the focused element's id, and the chosen repository |
| P5 | `focus_returns_to_what_opened_settings` | Table: the footer item; a list row (Ctrl+, from the ticket list); a lifecycle dialog route (focus returns to the dialog's origin); an origin that left the tree (the content region) |
| P6 | `settings_closes_every_way` | Table: Escape, the close button, the scrim, Show in Activity, quitting. Each leaves `Shown::Tabs`, the owner `None` and a typed passphrase empty |
| P7 | `nothing_acts_behind_settings` | With Settings open, by dispatched actions and key presses: the panel keys, F6, `secondary-n`, ctrl-tab, `secondary-w`, the drafts key and Ctrl+, change nothing; Ctrl+, behind a dialog opens nothing |
| P8 | `settings_scrolls_on_its_own` | PageDown and End move Settings' offset, not the column's or the item's; Tab to a control below the fold reveals it |
| P9 | `a_typed_passphrase_is_emptied_on_close_and_on_leaving_sharing_and_keys` | Extends `a_typed_passphrase_is_emptied_when_the_page_is_hidden` (E-6) |
| P10 | `one_repository_is_chosen` | Manage b shows b's remotes under Sharing; `Remotes(a)` chooses a; a footer open after selecting c in the panel chooses c; choosing the chosen repository again keeps typed identity text |
| P11 | `every_page_is_read_again_on_open` | A registration added while Settings is closed is listed on the next open of Repositories |
| P12 | `key_rows_send_what_todays_controls_sent` | Radio, Stop using this key, the trash and Delete key file… send `KeySelect`, `KeyClear`, `KeyRemove` and open the `KeyDelete` confirmation; Delete key file… only on generated keys; In use on the selected key |
| P13 | `the_key_forms_open_from_their_buttons` | Make a new key and Use a key I already have… each show their form and hide the other; W-26 shows the Generate form with focus in its passphrase |
| P14 | `show_in_activity_closes_settings_and_focuses_activity` | With Activity hidden: the button closes Settings, shows Activity and focuses it |
| P15 | `settings_does_not_open_behind_a_dialog_or_while_quitting` | A route raised under an open dialog and one raised while quitting open nothing |
| P16 | `settings_has_three_pages` | The menu has three entries in order; each opens its pages; Enter and Space on an entry open it |
| P17 | `the_old_settings_places_are_gone` | No `sidebar-settings`, no `settings-close`; the item area never shows Settings |
| P18 | `settings_copy_uses_the_vocabulary` | No Settings copy constant contains "publication remote", "SSH key", "pin", "Hosts page", "Remotes page", "sidebar" or "Back to tabs" |

## Testing

- **Command.** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
- **Layer.** Every test is a desktop unit test; nothing touches the
  library, so no integration or SSH target changes (`AGENTS.md`).
- **Behaviour, not appearance** (Wave Testing): no test asserts widths,
  colours, icons or the header's layout. P16 checks entries and order, not
  their look.
- **Focus** is read inside the same update that moves it (Cycle 01 and
  03a ledgers).
- **Guards** are tested with dispatched actions and key presses, not
  pointer clicks, which land on the scrim (Cycle 02 ledger).
- **Window tests with polling on** answer every poll (03a handoff).
- **Helpers.** The five shared helpers that open a page (E-5) change in
  one place each, so their ~80 callers need no edit.

## Named Edits To Pre-Existing Tests

At `ba95e1b7`. Task 0 re-counts them with the plan's sweep after Cycle 03b
merges, since 03b edits several of the same tests (its E-4, E-6, E-7,
E-12, E-13). Edits that remove assertions are marked **Removes**.

| E | Tests | Edit | Task |
| --- | --- | --- | --- |
| E-1 | `D/view/settings/mod_tests.rs`: `the_settings_key_shows_the_settings_and_back_keeps_the_tabs` (`:73`), `the_sidebar_entry_shows_the_settings` (`:123`), `opening_an_item_shows_the_tabs_again` (`:137`), `focusing_the_content_region_focuses_the_settings` (`:168`) | `:73`: Escape instead of Back; the tab-strip-hidden assertion (`:98`) **Removes** (the strip stays drawn under the sheet). `:123` **Removes** the test (the button is gone; P17). `:137` **Removes** the test (no item opens behind the sheet; P7). `:168` **Removes** the test (`secondary-3` is refused behind the sheet; P7). `the_settings_key_does_nothing_while_a_dialog_is_open` (`:154`) is unchanged | 1 |
| E-2 | `D/view/item/sheet_tests.rs` `no_workspace_key_acts_behind_the_sheet` (`:670-741`) | `secondary-,` leaves the refused keys (`:719`, `:731-739`) **Removes**; P3 asserts the new behaviour | 1 |
| E-3 | `D/view/item/frame_tests.rs` `page_keys_scroll_the_items_region_and_settings_keep_theirs` (`:148-178`) | Settings' PageDown moves Settings' own offset, not `content_offset`; closes by Escape | 2 |
| E-4 | `D/view/drafts_routing_tests.rs` (`:66`, `:83`) | `:66`: Settings no longer hides Drafts; Escape instead of Back; **Removes** `!present("drafts")`. `:83`: the drafts key cannot act behind the sheet; the passphrase is emptied by closing Settings, then the drafts key opens Drafts | 1 |
| E-5 | Helpers: `settings` (`D/view/settings/repositories_tests.rs:40`), `hosts` (`hosts_tests.rs:61`), `remotes` (`remotes_tests.rs:47`), `keys` (`keys_tests.rs:51`), `Rig::open` (`polling_support.rs:104`), `open_page` (`D/view/keyboard_tests/settings.rs:22`) and its five callers, `Walk::open_settings_and_each_page` (`D/view/keyboard_tests.rs:561-577`) | Menu indices 0–2 instead of 0–4; `remotes` chooses its root by Manage, not the Remotes page's Choose; the walk closes by Escape and finds focus on the control that opened Settings, not the shown tab (`:574-576`) | 1 (indices for five entries), 5 (three entries) |
| E-6 | Tests that leave Settings by `settings-close` or `settings-close-button`: `keys_tests.rs` `a_typed_passphrase_is_emptied_when_the_page_is_hidden` (`:350`) and `the_operation_area_retry_of_a_protected_generation_asks_on_the_page` (`:832`); `D/view/item/share_tests.rs` `share_opens_remotes_on_its_own_repository_after_another` (`:754`, `:771`); `D/view/create_tests.rs` `initials_required_points_to_settings` (`:886`, `:930-935`); `hosts_tests.rs` (`:263`); `remotes_tests.rs` (`:731`) | Escape instead of the button | 1 |
| E-7 | `D/view/operations/attention_tests.rs` `attention_actions_hit` (`:367`, `:387-391`) | Closes Settings by Escape between the row clicks, which the scrim would otherwise take | 1 |
| E-8 | `D/view/settings/attention_tests.rs` `settings_polling_attention_actions` (`:34`, `:83`) | Ctrl+, with Settings open now does nothing; the assertion that follows reads the page the attention action left | 1 |
| E-9 | `D/view/settings/remotes_tests.rs`: every test that clicks the Remotes page's Choose row | Chooses by Manage on the Repositories page (R-15) | 3 |
| E-10 | `D/view/settings/keys_tests.rs`: tests pressing Select, Clear selection, Unregister or Delete key by element id | The radio, Stop using this key, the trash and Delete key file… by their new ids | 4 |
| E-11 | Tests comparing a literal Settings string rather than its constant | The new string or its constant (Part E) | 7 |

Further sites the sweep finds are recorded in the ledger as E-12 onward
(decision 7).

## Document Amendments

Applied by the planner in the approval commit:

1. **Wave 04, Cycle 04 section.** "Amended 2026-11-13 by Cycle 04":
   - the sheet has a header with "Applies to this computer only." and no
     footer (decision 4);
   - the menu has three pages; the design's General, Your name & email
     and Appearance pages are not shown (Wave scope and exclusions);
   - Sharing & keys follows `05`'s row layout with today's actions and
     without the passphrase note, dates and algorithm (decision 2);
   - no Pause all switch (D6 decision 8);
   - Show in Activity closes Settings (decision 5).
2. **`docs/design/05-sheets-and-dialogs.md`, Settings sheet.** An "As
   built, Wave 04 Cycle 04" note listing amendment 1's points.
3. **Wave 03 D2 design** (`docs/plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md:631-638`).
   A note: Settings is a sheet from Wave 04 Cycle 04, so it no longer
   excludes the Drafts page; a typed passphrase is emptied when Settings
   closes or leaves Sharing & keys.
4. **Cycle 03a's Cycle document**, decision 15 and K-8: "Done in Cycle 04"
   beside the panel's Settings button.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Settings is a Kit `Sheet` on Root's one sheet layer (`08`:29), not a layer of the Workspace's own | The Kit's close and focus quirks (facts 31, 34) are handled in A3; a Workspace layer would need its own trap and scrim |
| R-2 | `SheetOwner` records who holds the sheet; `sync_sheet` keeps a Settings sheet (A1) | Without it Settings closes on the next frame (fact 28) |
| R-3 | `Shown` keeps its variants: `Tabs` now means Settings is closed, and `Settings(page)` the page entity the sheet last opened at | The name `Tabs` reads oddly (K-3); renaming would touch 21 test files for no behaviour |
| R-4 | `SettingsPage` keeps five variants; `SettingsNav` groups them into three menu entries | Two enums to keep in step; `nav()` is one match |
| R-5 | Every way in goes through `show_settings`; today's entry functions stay as wrappers | One more layer of calls; the alternative edits 32 call sites and Cycle 03b's |
| R-6 | The origin is the given handle, else the focused one; over the details sheet, that sheet's chip | Focus can land on a control the person did not expect when the origin is gone; it then falls back to the content region |
| R-7 | The close is detected in `sync_sheet`, not by `on_close` | A close in a frame that does not render would be seen one frame late; every close path notifies |
| R-8 | Ctrl+, opens over the details sheet, does nothing over Settings or a dialog (A5) | — (decision 6) |
| R-9 | `show_settings` refuses under a dialog and while quitting | A route raised under a dialog is lost; today none is reachable (routes from dialogs close them first, fact 22) |
| R-10 | Escape always closes Settings, also while a request it started runs; `05`'s "unless an operation is running" is for dialogs whose work stops when they close | Someone could close Settings mid-generation; the result is in Activity and on the page next time |
| R-11 | Settings handles the page keys on its own body and does not propagate | Inside a control that consumes PageDown nothing scrolls, as today |
| R-12 | Settings scrolls its own `ContentScroll::region()`, laid over the Kit body as the details sheet does | — |
| R-13 | A typed passphrase is emptied when Settings closes or leaves Sharing & keys | — (D2 rule, decision 6) |
| R-14 | One chosen repository for Settings, set by every open and by Manage | A person who chose b inside Settings and reopens from the footer starts on the panel's selection, not b |
| R-15 | The Remotes page's own repository list goes; Manage chooses for both sections | Choosing a repository for its remotes is now in the Repositories list |
| R-16 | Sharing & keys holds the key section, then Trusted servers with Check host on the chosen repository | Check host's repository is chosen on the other page |
| R-17 | Key rows as decision 2: radio, tags, fingerprint, Copy public key, trash (Remove from Manyhands), Delete key file… on generated keys; Stop using this key under the list | The trash's tooltip differs from `05`'s for generated keys, which promised to delete the file |
| R-18 | Make a new key and Use a key I already have… show today's forms; one at a time | — |
| R-19 | The info box uses `05`'s sentence, true by the credential RFC (fact 41) | If session handling changes, the sentence must change with it |
| R-20 | Show in Activity closes Settings, then focuses Activity, showing it if hidden | — (decision 5) |
| R-21 | The panel's Settings button goes; the empty state and Add keep opening Repositories until Cycle 05 | — |
| R-22 | Key-list routes focus the in-use or first key's radio, else Make a new key, retargeting once when the read lands if focus has not moved | A slow read leaves focus on the entry |
| R-23 | Approve host… with no server named uses the attention row's repository (W-25) | — (fixes a wrong repository) |
| R-24 | Routes naming a field focus it (Part D); copy follows Part E and its rule | A copy change can break a test reading a literal (E-11) |
| R-25 | The header's description is part of the Kit title element | — |
| R-26 | No sheet footer | — (decision 4) |
| R-27 | The menu is three plain `Role::Tab` entries with ids `("settings-page", 0..3)` (R-T12a) | — |
| R-28 | The scrim closes Settings (Kit default), with A3's close | A stray click outside closes Settings; nothing typed is lost but a passphrase |
| R-29 | Settings closes when quitting and does not open while quitting | — (R-T17b for dialogs) |
| R-30 | `SettingsHost` stays in `ContentArea.settings` | — |
| R-31 | Opening Settings leaves Drafts shown under the sheet | — |
| R-32 | `open` calls `opened` on every page, Repositories included | One more read per open |

## Known Limits

- **K-1.** Key rows show no passphrase note, creation date or algorithm,
  and trusted servers no date: the library records none (fact 38;
  decision 2).
- **K-2.** No "Pause all checking" switch (fact 42).
- **K-3.** `Shown::Tabs` means "Settings is closed" (R-3).
- **K-4.** The design's General, Your name & email and Appearance pages are
  not shown; name and email stay per repository on the Repositories page.
- **K-5.** A trusted server whose key changed is handled by today's Check
  host and Replace…, which show the old and new fingerprints side by side;
  the design leaves this undrawn.
- **K-6.** The Kit sheet's top margin is the Kit title bar's 34 px, not
  the app's 38 px (fact 33), as for the details sheet today. Checked on
  screen.

## Unverified

- **U-1.** Focus inside the sheet counts as `Region::Content` for
  `focused_region` and the tests' `walk.on(…, Region::Content)` (fact 25's
  comment). Task 1 checks it with a scratch test; if not, E-5's helpers
  read the focused id without the region.
- **U-2.** A route can focus a key row's `Radio`, and Space then chooses
  it. Task 4 checks it; if the Kit's private focus handle cannot be
  focused from outside, the row's radio is a wrapped control with
  `role(accesskit::Role::RadioButton)` and `aria_toggled`
  (`gpui-pre-0.3.6/src/elements/div.rs:1304`, `:1400`), drawn like a
  radio.
- **U-3.** A two-line element in `Sheet::title` lays out above the Kit's
  close button. Checked on screen in Task 1.
- **U-4.** `ContentScroll::region()`'s shared reveal reveals a control in
  a deferred sheet (B2). Task 2 checks it with P8.
- **U-5.** After Cycle 03b merges: how `choose_activity` passes an origin
  to `attend` and `open_route`, and that W-12 to W-29's lines still hold.
  Task 0.

## Alternatives Rejected

- **A Workspace-owned layer** like Cycle 03b's popover: it would need its
  own trap, scrim and Escape, which the Kit sheet has; `08` maps Settings
  to the Kit sheet.
- **The Kit's `Settings` component** (`K/setting/settings.rs`): ruling
  R-T12a, and its keyboard model is unread.
- **Renaming `Shown`'s variants**: 21 test files for no behaviour (R-3).
- **A library ticket for the key data first**: the owner chose to leave
  the data out (decision 2).

## Risks

| Risk | Control |
| --- | --- |
| Cycle 03b changed the footer, `attend`, `render.rs` and `focus.rs` after these citations | Implementation starts after 03b merges; Task 0 re-maps every citation and re-runs the E sweep (U-5) |
| Test churn larger than listed | The helpers change in one place (E-5); Task 0's sweep records every further site (decision 7) |
| Focus lands somewhere unexpected after a close | P5's table, read inside the same update |
| The Cycle grows too large to review | Split line after Task 2: the sheet working with today's five pages (04-1), then the regrouping, routes and copy (04-2) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-13 | Plan Cycle 04 now, execute after 03b lands | Owner direction | Decision 1 | Session, 2026-11-13 |
| Planning research (four read-only agents, checked against source) | `sync_sheet` closes foreign sheets; the R-60 guard and its test conflict with Wave decision 6; one shared scroll; passphrase clearing tied to the content page; the Kit sheet's focus record on replacement; key and host data the library lacks; the design's footer, nav and Pause all against the Wave and D6 | Facts; owner questions; rulings | Facts 1–43; decisions 2–5; R-2, R-6, R-7, R-8, R-12, R-13 | Session, 2026-11-13 |
| Owner, 2026-11-13 | Key list; fold-in; sheet footer; Show in Activity | Owner decisions | Decisions 2–5 | Session, 2026-11-13 |
| Owner, 2026-11-13 | The outline, with its stated constraints | Owner decision | Decision 6 | Session, 2026-11-13 |
