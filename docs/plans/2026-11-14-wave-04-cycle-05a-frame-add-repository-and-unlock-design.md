---
title: "Wave 04 Cycle 05a Design: Common Frame, Add Repository And Unlock Key"
date: 2026-11-14
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7D7V0Q1D39V8GQ28EEY61CW"
---

# Wave 04 Cycle 05a Design: Common Frame, Add Repository And Unlock Key

Ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`.
[Cycle document](../Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md)
and [implementation plan](2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-implementation.md).

## Intent And Approval Boundary

This design moves today's dialogs onto `05`'s common frame, moves Add
repository out of Settings into its own sheet, and restyles the Unlock
key dialog and the empty item area. It changes `src/desktop/` only.

It is a draft for the owner's approval. Approving it does not authorize
implementation, which also waits until Cycle 04 has merged (Cycle
decision 1).

Planned against:

- main at `ea6d899b` for everything Cycle 04 does not touch;
- **Cycle 04's approved design** (`docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`
  on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`, approved at
  `ded8402f`) for the Settings sheet, the sheet owner and the routes into
  Settings. Cycle 04 is in execution. Task 0 re-maps every name this
  design takes from it against the merged code (U-1).

## Conventions

- `D/` is `src/desktop/`; `GC/` is the locked
  `gpui-component-0.6.6/src/`, `GB/` is `gpui-base-0.6.6/src/` and `GP/`
  is `gpui-pre-0.3.6/src/`, under
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. The app
  reaches them only as `gpui_kit::…`.
- "C4 A6" and similar name a part of Cycle 04's design; "C4 R-n" and
  "C4 W-n" its rulings and ways in.
- "Decision n" is the Cycle document's. "Fact n", "R-n", "G-n" (focus
  rows), "AW-n" (ways into the Add sheet), "P-n", "E-n", "K-n" and "U-n"
  are this design's. "A1"–"A6" are Part A's sections.
- **(owner's eye)** marks a change people see today or a departure from
  an authority.

## Baseline Facts

Read on 2026-11-14 at `ea6d899b`, and in Cycle 04's approved design.

### The dialogs today

1. **Fifteen dialogs, one kind.** Every dialog opens with
   `window.open_dialog` as a plain Kit `Dialog`, with
   `.close_button(false).overlay_closable(false)`. None uses
   `AlertDialog`, the Kit `footer` slot or a description. Every button is
   a child of the Kit's scrolling body. The sites:

   | Dialog | Opens at | Width | Title today |
   | --- | --- | --- | --- |
   | Confirmation | `D/view/confirm.rs:240` | 448 (Kit default) | `intent.label()` |
   | New ticket, New document | `D/view/create.rs:148` | 448 | Kit title |
   | New folder | `D/view/new_folder.rs:121` | 448 | Kit title |
   | Move | `D/view/move_document.rs:213` | 448 | Kit title |
   | Close a tab with a draft | `D/view/close_draft.rs:315` | 448 | four titles by case |
   | Quit asks | `D/view/quit.rs:253` | 448 | "Quit" |
   | Unlock key | `D/view/passphrase.rs:85` | 448 | "Unlock key" |
   | Retire local copy | `D/view/retire_asks.rs:152` | 448 | Kit title |
   | Discard a draft | `D/view/drafts/actions.rs:241` | 448 | Kit title |
   | Discard your repair? | `D/view/item/repair/prompt.rs:88` | 448 | Kit title |
   | Discard this reply/comment? | `D/view/item/composer/discard.rs:57` | 448 | Kit title |
   | Recovery off | `D/view/item/edit/recovery_off.rs:50` | 448 | `RECOVERY_OFF` |
   | Replace your edits | `D/view/item/edit/review.rs:255` | 448 | drawn in the body |
   | Lifecycle (Cycle 05b) | `D/view/lifecycle.rs:226` | 580 | own header |
   | Resolve conflict (Cycle 05b) | `D/view/conflict.rs:203` | 920 | own header |

2. **Escape today.** Every dialog's `on_cancel` returns `true`, so Escape
   closes it at once, except: New ticket, New document, New folder and
   Move refuse while their write runs (`!…creating`, `!running`;
   `D/view/create.rs:148`, `new_folder.rs:121`, `move_document.rs:213`),
   and the composer discard closes itself and returns `false`
   (`D/view/item/composer/discard.rs:57`).
3. **Focus return today.** Most dialogs focus a recorded origin in
   `on_close`. Create, New folder, Move, the quit asks, the retire asks
   and the passphrase dialog's Escape rely on the Kit's own restore.
   `on_close` runs only on the Kit's Cancel, Confirm and backdrop paths,
   not on a direct `window.close_dialog` (GB `dialog.rs:559-588`).
4. **Enter** is `Confirm` in the `Dialog` context (GB `dialog.rs:18`,
   `:89-93`). When `on_ok` returns `true` the Kit uses
   `defer_close_dialog`, which restores focus 0.25 s later
   (GC `root.rs:341-363`).
5. **Shared pieces.** `D/view/create/form.rs` holds `field`, `button`,
   `held_button`, `keep_focus_in` and `focus_the_note`, used by Create,
   New folder and Move. Eleven dialogs have their own
   "button in a div with `on_action(Confirm)`" wrapper (`confirm.rs:762`,
   `passphrase.rs:288`, `quit.rs:272`, `close_draft.rs:365` and others).
   Three hand-draw a focus ring (`form.rs:93-104`, `quit.rs:139-148`,
   `composer/discard.rs:122-128`).
6. **The details sheet** (`D/view/item/sheet.rs:174-276`) is the only Kit
   `Sheet`: 440 px; title `sheet-heading` in the Kit title slot; the
   description `sheet-description` in the body; a footer with the note
   `sheet-note` and Done (`sheet-done`).

### The theme today

7. **The chrome tokens** (`D/view/shell/tokens.rs:48-70`) hold `07`'s
   `chrome`, `panel`, `border`, `border_strong`, `hover`, `faint`,
   `muted`, `secondary`, `text`, `selection`, `accent`, `teal`, `green`
   and `red` for both modes, chosen by `cx.theme().is_dark()`. There is no
   `editor`, `surface`, `on_accent` or `teal_strong`.
8. **The Kit theme is never changed by the app.** `D/mod.rs:57-72` calls
   `gpui_kit::init`, which runs `Theme::change(ThemeMode::Light, …)`
   (GC `theme/mod.rs:36-42`). No code under `D/` calls `Theme::change`,
   `global_mut`, `apply_config` or `sync_system_appearance` outside tests.
   Which mode the app shows on screen is U-2.

### Add and Create today

9. **Both forms live in Settings › Repositories,**
   `D/view/settings/repositories.rs` (802 lines). About 560 lines serve
   them:
   - copy constants `:71-99`;
   - `enum Form { Add, Create }` `:101-175`, which names every id:
     `add-path`, `add-branch`, `add-prefix`, `add-name`, `add-email`,
     `add-browse`, `add-submit`, `add-message`, and the same with
     `create-`;
   - `enum Inspection { None, Checking(String), Checked { path,
     inspection }, Busy(String), Failed(ResultCode) }` `:177-205`;
   - the page's fields `inspection`, `failed_path`, `generations`,
     `inspect_read`, `pending_branch`, `messages`, `browsing`,
     `browse_unavailable`, `submit_pending`, `window` (`:207-239`);
   - `input_event`, `forget_inspection`, `inspect` (`:383-433`),
     `inspect_root` (`:436`), `add_checked`, `resume_submit` (`:461`),
     `submit` (`:483-536`), `browse` (`:539`), `chosen` (`:567`);
   - `folder` (`:640`), `inspection` (`:667`), `message` (`:718`),
     `add_form` (`:724-750`) and `create_form` (`:752-772`).

   `field`, `button`, `heading`, `text`, `NAME` and `EMAIL` are shared
   with the manage section (`repositories/manage.rs`).
10. **The folder check** reads
    `services.read(ReadKind::Inspect, stamp, move |service| service.inspect_repository(&root), …)`
    (`:393-398`) under `ViewScope::Settings(SettingsPage::Repositories)`,
    a scope the manage section's reads share (`D/model/stamp.rs:32-46`).
    It runs on Enter, on blur after a change, and after Browse. A
    successful check fills an empty branch from `head_branch`
    (`:406-408`). A failure keeps only its code: `Failed(failure.code)`
    (`:417`) drops the read's recovery actions. Busy shows a label and no
    Retry.
11. **What the check returns.** `RepositoryInspectionDto`
    (`src/repository/read/dto.rs:153-162`): `selected_path`, `root`,
    `registered`, `head_branch: Option<String>`,
    `local_branches: Vec<String>`, `configuration` (`state`
    Valid/Invalid/Missing, `primary_branch`, `publication_remote`,
    `problems`), `identity_state` (Available/Required) and
    `remotes: Vec<RemoteDto>`. `RemoteDto` (`:200-206`) has `name`,
    `fetch_location`, a redacted `push_location`, `publication_eligible`
    and `selected_for_publication`; both locations are redacted
    (`src/repository/read/admin.rs:140-157`). There is no writability
    field and no reason for an ineligible remote.
11a. **A repository with no commits.** `head_branch` comes from the
    symbolic HEAD, so it names a branch that has no commits yet
    (`src/repository.rs:9965-9991`), while `local_branches` lists only
    branches with commits (`:9993-10015`). Enable accepts it
    (`src/repository/mutation/bind/repo.rs:784-791`). Enable writes no
    publication remote (`src/repository.rs:10215`); `RemoteSelect` with
    no name clears one, as Settings' `remotes-select-none` does
    (`D/view/settings/remotes.rs`).
12. **What Add and Create send.** Both are confirmed intents
    (`D/services/requests.rs:357-372`). `submit` builds
    `Intent::RepoEnable` or `RepoCreate` and calls
    `confirm(intent, origin, &services, window, cx)`
    (`D/view/confirm.rs:161`), which opens the confirmation dialog with
    the library's preview. Their inputs (`src/repository/mutation/bind/repo.rs:76-101`):
    `RepoEnableInput { root, primary_branch, identity, slug_prefix }`,
    whose branch "must be the one checked out" (refused as
    `WrongBranch`, `:793`); `RepoCreateInput` the same, whose root "is
    absent or empty, in a parent that exists". Neither takes a remote.
13. **How a page learns the outcome.** No callback reaches the page. The
    worker's `Finished` event sets
    `RequestState::Finished(Outcome, ResultCode, Effects, Vec<RecoveryAction>, String)`
    (`D/services/requests.rs:855`) in `services.operations()`, which holds
    no scope. On `Ok` the worker submits a background `IndexRefresh` for
    the root (`D/services/worker.rs:1352-1375`).
    `confirm_followed(intent, origin: Origin, …) -> Option<Entity<ConfirmDialog>>`
    (`D/view/confirm.rs:174-182`), where
    `Origin = Rc<dyn Fn(&App) -> FocusHandle>` (`:151`), emits the event
    `Followed::Confirmed { request, details }` or
    `Followed::Answered(code)`; `D/view/item/repair.rs:314` and
    `D/view/operations/republish.rs:84` use it. Each `Request` carries an
    `attempt` (`D/services/requests.rs:835-838`): 1, then one more for each
    retry or Prepare again.
13a. **The confirmation closes itself.** `ConfirmDialog`'s own operations
    observer closes the dialog on any finish that is not stale, then
    focuses its origin, with a direct `close_dialog`, not the deferred one
    (`D/view/confirm.rs:510-545`). The Workspace's operations observer was
    registered at startup (`D/view/workspace.rs:214`), before any dialog's,
    so it sees `Finished` while the confirmation is still open. Prepare
    again re-confirms the same request id and emits
    `Followed::Confirmed` again (`:466-472`), and the model can still hold
    the earlier attempt's state until the new one is queued (`:521-525`).
    A failed add is shown by the Activity row, not by the dialog.
14. **Browse** (`:539-587`) calls `cx.prompt_for_paths` for one
    directory. While it is open the button is disabled with no text;
    after a platform failure it stays disabled (`:556`), and the reason
    (`browse-unavailable`) is drawn once, at the bottom of the page
    (`:782-784`). A chosen path that is not text gives the message
    `BROWSE_NOT_TEXT` for that choice and leaves Browse enabled
    (`:577-580`). `inspect_root` ignores a root that is not valid UTF-8
    (`:442-444`).
15. **Remote selection today** (`D/view/settings/remotes.rs`).
    `submit_select(root, name, …)` (`:438-451`) builds
    `Intent::RemoteSelect { root, name }`, unconfirmed
    (`D/services/requests.rs:374`), and submits it with
    `worker().submit(intent, Lane::User, None, None)` (`:453`). Its
    Activity row says "Choosing how {repo} shares", then "{repo} now
    shares through {name}", or on failure "Couldn't choose how {repo}
    shares" with the message and "Your work is safe on this computer."
    (`D/model/activity/intent_words.rs:106-113`,
    `D/model/activity/requests.rs:296-298`). `NOT_ELIGIBLE` is "Not
    eligible for publication: publishing needs a remote reached over SSH"
    (`remotes.rs:86-87`). The page lists `fetch_location` beside each
    name.
16. **The ways in today** all open Settings › Repositories in the item
    area:
    - the empty Repositories panel's "Add or create a repository…"
      (`D/view/repositories/body.rs:108-119`);
    - the toolbar's `+`, `ToolbarButton::AddRepository`, id
      `add-repository-button` (`D/view/dock/header/toolbar.rs:235-255`);
    - the `repo.inspect` and `remote.history` recoveries
      (`D/view/workspace/requests.rs:602`) and the lifecycle dialog's
      "Open repository details" (`D/view/lifecycle/actions.rs:144-153`),
      through `inspect_in_settings` (`D/view/workspace/content.rs:237-248`),
      `SettingsHost::inspect` (`D/view/settings/mod.rs:354-363`) and
      `inspect_root`.
17. **Tests reach the forms** through `settings()` and the helpers of
    `D/view/settings/repositories_tests.rs:41-170`, which the manage tests
    import (`repositories_manage_tests.rs:22`).

### Cycle 04 (approved design, not yet merged)

18. **The sheet owner** (C4 A1): `ContentArea.sheet` becomes
    `Option<SheetOwner>`, `enum SheetOwner { Details(WeakEntity<ItemPane>), Settings }`,
    in `D/view/workspace/content.rs`. `sync_sheet` keeps Settings while a
    sheet is active, runs `settings_closed` when the Kit has closed it,
    and closes quietly a host left open under another owner.
19. **Opening Settings** (C4 A2): one function,
    `Workspace::show_settings(route: SettingsRoute, origin: Option<FocusHandle>, window, cx)`,
    in `D/view/workspace/settings_sheet.rs`. `SettingsRoute` has
    `Page`, `AddRepository` ("the Repositories page, on its Add folder
    field"), `Manage(PathBuf)`, `Inspect(PathBuf)`, `Identity`, `Remotes`,
    `Keys`, `Hosts`, `CheckHost` and `RetryGeneration`. The origin is the
    first of the argument, the Workspace's `settings_origin`, an open
    layer's `restore`, and the focused handle. `show_settings` refuses
    while a dialog is open or the app is quitting.
20. **Closing Settings** (C4 A3, A4): `settings_closed(quiet)` closes the
    Kit sheet if it is still active, clears the owner and the host's
    `open` flag, hides the host (which empties a typed passphrase and a
    pending Add), and unless `quiet` focuses the origin and arms
    `settings_return`. Every quit path closes Settings quietly. Escape
    always closes Settings, also while a request runs (C4 decision 6).
21. **The routes Cycle 04 moves to this Cycle** (C4 Part D):
    W-5 (empty panel), W-6 (toolbar `+`), both
    `show_settings(AddRepository, None, …)` landing on `add-path`; W-22
    (lifecycle "Open repository details") and W-27 (the recoveries),
    `Inspect(root)` landing on `add-path`. C4 R-48: `submit_pending` is
    cleared when the host hides and by `inspect_root`. C4 R-55: `Inspect`
    does not change the chosen repository.
22. **Cycle 04's focus contract** is its A6 table (F1–F19), each row read
    after at least one draw, and A7's ten facts any mechanism must
    survive: focus reads lag a frame; Root's overlays sit outside the
    Workspace; Kit popups move focus; the 0.25 s deferred restore;
    `close_sheet` and `close_all_dialogs` move focus without looking;
    `focus_lost` may have no target; a layer's `restore` may be empty;
    `region_of` defaults to Content; `on_next_frame` never runs in tests;
    an `open` flag is only as fresh as the close that clears it.
23. **Cycle 04's Downstream Obligations "For Cycle 05"**: an owner variant
    for the Add repository sheet; emptying a typed passphrase when it
    replaces Settings; reopening Settings at Repositories after Cancel or
    Add with the origin Settings recorded first; the routes W-5, W-6,
    W-22 and W-27; the common frame keeps Settings' Escape rule.
23a. **Cycle 04 keeps focus inside Settings only.** Its R-34
    (`focus_content` does nothing and `apply_review`'s refocus is
    dropped) and R-46 (a focus landing behind the open sheet goes to the
    page's entry) are keyed on the owner `Settings` (C4 A3, A5; F16,
    F18). Its Ctrl+, handler opens Settings over the details sheet (C4
    A5). It adds `D/view/settings/frame.rs` for the Settings header.
24. **Cycle 04's execution has started.** Its branch holds Task 0
    (`9f89a3bb`, `003eb518`), whose ledger adds a way in (W-36) and
    rulings 7 and 8. Task 0 of this Cycle reads the merged result.

### The Unlock key dialog today

25. **`D/view/passphrase.rs`** (372 lines). Constants `TITLE` ("Unlock
    key"), `UNLOCK`, `CANCEL` ("Not now"), `NOT_NOW_NOTE`, `REFUSED`;
    `waiting_subtitle(repository)` and `label_for(label)` ("Passphrase for
    {label}"). `open_dialog(prompt, window, cx)` (`:76-110`) opens a Kit
    dialog with the title, `on_ok` submitting and `on_cancel` answering
    `Cancelled`, then focuses the masked input. Ids: `passphrase-dialog`,
    `passphrase-subtitle`, `passphrase-label`, `passphrase-reason`,
    `passphrase-key` ("Key {id}"), `passphrase-input`,
    `passphrase-refused`, `passphrase-note`, `passphrase-ok`,
    `passphrase-cancel`. Buttons are drawn Unlock, then Not now.
26. **A refused passphrase** (`:174-183`): an empty one or one with a NUL
    sets `refused`, keeps the dialog and the typed text, and moves no
    focus.
27. **What a prompt knows.** `PromptRequest { key_id, label, reason, reply }`
    (`D/services/prompts.rs:62-67`), built from the library's
    `UnlockRequest { key_id, label, source, reason }`
    (`src/repository/keys/session.rs:139-150`). No fingerprint, and no
    sign that an earlier attempt failed. `Workspace::show_prompt`
    (`D/view/workspace/lifecycle.rs:354-379`) opens it.
28. **The key list.** `list_keys()` (`src/repository/read/credentials.rs:190`)
    returns each `KeyDto` (`src/repository/read/dto.rs:247-258`) with
    `id`, `label` and `fingerprint: Option<String>`, without opening a
    key file. The Keys page reads it through
    `services.read(ReadKind::Keys, …)` (`D/view/settings/keys/reads.rs:14-33`).
    `Services::read` (`D/services/reads.rs:151`) runs on the background
    executor and retries Busy. The dialog holds no `Services`;
    `show_prompt` has the Workspace's.
29. **A declined unlock** surfaces in Activity and Needs attention: a
    share as "Couldn't share {subject}: your key is locked" with Unlock…
    (`D/model/activity/copy.rs:278-282`), a check as "Your key is locked"
    with Unlock… (`D/model/activity/states.rs:186-240`). A background job
    never prompts (`D/services/prompts.rs:27-34`).
30. **A wrong passphrase** reaches the desktop as `transport_unavailable`
    (ticket `01M5E6DYJ6GN7DMW4424TH3V78`, open). Nothing tells the dialog
    an attempt failed.

### The empty item area

31. `empty_pane()` (`D/view/workspace/content.rs:646-660`) draws the text
    `item-area-empty` only; its constant (`:35-37`) notes "without its
    icon until Cycle 05". The app icon at runtime is
    `src/desktop/brand/manyhands.png` (64×64), served as `APP_ICON_PATH`
    (`D/assets.rs:74-76`) and drawn by the title bar with
    `img(APP_ICON_PATH)` (`D/view/shell.rs:15`, `:62`).

### GPUI Kit 0.6.6

32. **Dialog slots** (GC `dialog/dialog.rs`): `title(impl IntoElement)`
    (`:323`), `footer(impl IntoElement)` (`:339`), `w` (`:405`),
    `overlay_closable` (`:433`), `keyboard` (`:439`). The title and body
    share an inner block with an 8 px gap; the footer is a sibling outside
    the scrolling body (`:633-684`). The Dialog's own padding sets each
    side (`:540-552`); with `.p_0()` an element in the title or footer
    slot can draw its own padding and border.
33. **Sheet slots** (GC `sheet.rs`): `title(impl IntoElement)` takes any
    element (`:77`) and sits in a row with the close button
    (`:191-208`), which dispatches `Cancel`; `footer` is drawn outside the
    body in a `justify_between` row with fixed padding (`:221-230`);
    `size` (`:89`). The title and footer rows' padding cannot be changed.
34. **Root's layers** (GC `root.rs`). One sheet slot and a stack of
    dialogs (`:40-41`). `open_dialog` records the focused handle and
    focuses its own (`:297-323`); `close_dialog` focuses that record
    (`:325-339`). `open_sheet_at` replaces a sheet and keeps the first
    record (`:379-408`); `close_sheet` focuses it (`:410-423`). Dialogs
    draw at priority 10 and above, the app's sheet layer at 0, so a
    dialog opened from a sheet draws above it and gives focus back inside
    it on close.
35. **Escape in a sheet** (GB `sheet.rs:135-138`): the sheet's `Cancel`
    handler propagates and then closes, unconditionally. A sheet cannot
    refuse Escape.
36. **Theme slots** (GC `theme/theme_color.rs`, `theme/schema.rs`):
    `overlay` (`theme_color.rs:309`) is the one scrim colour for dialogs
    and sheets; `ring` (`:211`) colours every focus ring; `input` (`:173`)
    the input border; the primary button reads `tokens.button_primary`,
    `_hover` and `_active` and `colors.button_primary_foreground`
    (GC `button/button.rs:936`, `:954`, `:1086`, `:1170`).
    `Theme::change` re-applies the stored light or dark config and wipes
    values set through `Theme::global_mut` (`theme/mod.rs:261`;
    `schema.rs:1103`). `theme.apply_config(&config)` (`schema.rs:1060`, a
    `&mut self` method) stores a config so a later change re-applies it,
    rebuilds the tokens and **sets the theme's mode to the config's**
    (`:1104-1106`). Its keys include `button.primary.background`,
    `overlay`, `ring` and `input.border` (`schema.rs:306-315`, `:426`,
    `:483`, `:630`). A colour must be `#hex` or a scale name
    (`color.rs:693-723`); one that does not parse falls back to the
    default silently (`schema.rs:690-695`).
37. **The focus ring** is 3 px at 50% of `ring`, both private constants
    (GC `styled.rs:11-12`). A medium Kit `Input` is 32 px high
    (GC `sizing.rs:261-268`).
38. **Select** (GC `select.rs`): `SelectState::new(delegate, selected, …)`
    (`:149`), `selected_value` (`:369`), `SelectEvent::Confirm` (`:70-75`).
    A delegate item can draw its own row (`render`, `render_item`;
    `searchable_list/delegate.rs:33`, `:102`). A `disabled` item is
    greyed only: a click, Enter or the arrows still reach it
    (`list/list.rs:368-386`, `:500-514`; `list/cache.rs:107`, `:139`).
    `on_will_change` (`delegate.rs:167`; `select.rs:190-193`) can refuse
    a change, but `Confirm` is still emitted and the menu closes
    (`:201-203`). Escape in an open select stops there; on a closed one
    it reaches the sheet (GB `select.rs:294-301`).
39. **The Kit's segmented controls have no keyboard path.** Inside a
    `ToggleGroup`, each toggle's click only records its index and the
    group's `on_click` is a pointer listener (GC `button/toggle.rs:373-397`);
    the Kit's own test asserts that Enter reaches neither
    (`legacy_toggle_group_keyboard_click_does_not_reach_the_group_callback`,
    `:566-575`). Cycle 02 found the same and built its own segmented
    choice for the Tickets filter bar: focusable segments with
    `Role::Tab`, each a Tab stop that Enter or Space activates
    (`D/view/tickets/filter_bar.rs:1-7`, `:109-126`; `08`, amended
    2026-11-11).
40. **Icons.** The app's `icon_assets!(ItemIcons, …)` list
    (`D/assets.rs:17-86`) lacks `FolderInput`, `KeyRound`, `GitBranch`,
    `Cloud` and `X`, which `gpui_kit::assets::IconName` names and
    `gpui-kit-assets-0.6.6` holds. `DesktopAssets::load` falls back to the
    Kit's default bundle (`D/assets.rs:88-96`), which has `folder`,
    `info`, `circle-x` and `loader-circle` but not those five.
    `desktop::assets::tests::every_item_view_icon_loads` checks its own
    `ICONS` list.
41. **`img`** (GP `elements/img.rs`) takes `grayscale(bool)` (`:153`) and
    `Styled`'s `opacity` and `size` (`:513`).

## Part A: The Common Frame

### A1. One frame

A new module `D/view/frame.rs` builds the parts `05` draws, for dialogs
and sheets alike. It is a sketch; Task 1 settles the names.

```rust
/// A dialog's or sheet's header (`05`, "Common sheet frame").
pub(crate) struct Header {
    /// A 32 px icon tile, as `05` draws for Unlock key (2h).
    pub icon: Option<(IconName, Tone)>,
    pub title: SharedString,
    pub description: Option<SharedString>,
    /// The element ids the title and description keep (tests find them).
    pub ids: (&'static str, Option<&'static str>),
}

/// A footer: an optional hint on the left, then the buttons, right-aligned.
pub(crate) struct Footer {
    pub hint: Option<&'static str>,
    /// Drawn in this order (A3).
    pub buttons: Vec<AnyElement>,
}

/// Puts a header into a Kit `Dialog`'s title slot and zeroes its padding,
/// so the dialog's own view draws `05`'s body and footer edge to edge.
pub(crate) fn dialog(base: Dialog, header: Header, cx: &App) -> Dialog;
/// The body part of a dialog's own view: `05`'s padding and gap.
pub(crate) fn body(content: impl IntoElement) -> Div;
/// The footer part of a dialog's own view, drawn as its last child:
/// a top border, the hint on the left, the buttons right-aligned.
pub(crate) fn footer(footer: Footer, cx: &App) -> Div;
/// The header element for a Kit `Sheet`'s title slot.
pub(crate) fn sheet_header(header: Header, cx: &App) -> impl IntoElement;
/// A right-aligned footer element for a Kit `Sheet`'s footer slot.
pub(crate) fn sheet_footer(footer: Footer, cx: &App) -> impl IntoElement;
```

- **The footer stays in the dialog's own view** (R-19). New ticket, New
  document, New folder and Move close themselves only while focus is
  inside their body element (`D/view/create.rs:450`,
  `new_folder.rs:303`, `move_document.rs:396`), a keyed press parks focus
  inside it (`D/view/create/form.rs:158-171`), and the close-draft
  dialog's save does the same (`close_draft.rs:280`). Privacy sweeps and
  `focused_in` find buttons by their path inside the body
  (`editing_privacy_tests.rs:211`, `:736`; `keyboard_tests/frame.rs:150`).
  The Kit's `footer` slot is outside the body (fact 32), so a footer
  there would break all of these. The header holds no control, so the
  Kit's title slot is safe for it.
- **Padding and borders.** `dialog` sets the Kit dialog's padding to
  zero (fact 32). The header draws 18 20 14 with a bottom border
  (`border`); `body` 18 20; `footer` 12 20 with a top border. In a dialog
  taller than the window the footer scrolls with the body (K-11). On a
  sheet, the Kit's title and footer rows keep their own padding (fact
  33), so the dividers stop short of the edges (K-5).
- **Text.** Title 15 px, weight 600; description 12.5 px in `secondary`
  (chrome tokens). The Kit's title slot wraps the header in its
  semibold `DialogTitle`; the description sets its own weight.
- **Ids.** Every element id a dialog draws today is kept, also where an
  element moves into the header (R-4). A body that carries its title as
  its aria label keeps it (`close-draft-dialog`, `repair-discard-dialog`,
  `composer-discard-dialog`).
- **What it does not do.** It sets no `on_ok`, `on_cancel` or `on_close`:
  each dialog keeps its own (A3).

### A2. Theme slots

`frame::theme::apply(cx)` runs once in `D/mod.rs`'s `run`, right after
`gpui_kit::init`. It builds a `ThemeConfig` for each mode from the chrome
tokens and gives it to `theme.apply_config` on `Theme::global_mut(cx)`, so
a later `Theme::change` re-applies it (fact 36; R-2):

- `apply_config` sets the theme's mode to the config's, so the config of
  the mode the app is **not** in goes first and the current mode's last.
  The mode the app shows does not change (U-2).
- Then `Theme::sync_base(cx)`, as the Kit's own doc asks
  (GC `theme/mod.rs:197-205`; U-7).
- Every colour is written as `#rrggbb` or `#rrggbbaa` (fact 36).

| Slot | Dark | Light | From `07` |
| --- | --- | --- | --- |
| `button.primary.background` | accent | accent | `accent` |
| primary hover and active | accent-hover | accent-hover | `accent-hover` |
| primary foreground | `#1a1c20` | `#ffffff` | `on-accent` |
| `overlay` | `#0a0b0d99` | `#0a0b0d99` | `05`'s dialog scrim (K-3) |
| `ring` | teal | teal | `teal` |
| `input.border` | border-strong | border-strong | `border-strong` |

`D/view/shell/tokens.rs` gains `accent_hover` and `on_accent` with `07`'s
values for both modes. **(owner's eye):** every primary button in the app
turns amber, and every Kit focus ring teal (decision 6).

`05`'s secondary button (`hover` background, `border-strong` border), the
inputs' `editor` background, and the sheet's `border-strong` left border
and shadow are set the same way where `ThemeConfig` has a key for them
(U-7); where it has none they stay the Kit's (K-13).

### A3. Each dialog on the frame

Each dialog in fact 1's table, except the two Cycle 05b surfaces, builds
its header and footer with A1. For each:

- **Header:** today's title, in the Kit's title slot. Replace your edits,
  which has no Kit title, moves its `review-replace-message` into the
  header with its id. The Confirmation dialog keeps its `confirm-command`
  line in the body as well, which tests read. No dialog gains new text.
  Only Unlock key gains an icon tile and a description (Part D).
- **Body:** today's body, inside `frame::body`.
- **Footer:** today's buttons, in `frame::footer`, drawn by the dialog's
  own view as its last child, right-aligned (R-19).
- **Order:** today's, except where `05` draws the order: the
  Confirmation dialog and Unlock key put the action last (R-1).
  - Several dialogs put the safe choice first on purpose, and their code
    and tests say so: the quit asks (`quit.rs:309-310`), Discard a draft
    (`drafts/actions.rs:296`), the repair discard
    (`item/repair/prompt.rs:152`), the retire asks' `AskIds`, the
    close-draft dialog's Keep draft before Save (its R-16) and the
    composer discard (`05`'s own order). They keep it.
  - New ticket, New document, New folder and Move already draw Cancel
    first.
- **Unchanged:** what Enter does, what Escape does (fact 2), the control
  focused on open, where focus returns (fact 3), and every id.

The footers, left to right:

| Dialog | Footer | Changes |
| --- | --- | --- |
| Confirmation | Cancel (or Close) · Confirm, Retry or Prepare again | the action moves last (R-1) |
| New ticket, New document | Cancel · Create | — |
| New folder | Cancel · Create | — |
| Move | Cancel · Move | — |
| Close a tab with a draft | Keep draft · Save · Retry · Discard · Cancel · the unavailable Save | — |
| Quit asks | Cancel · Quit anyway | — |
| Unlock key | Not now · Unlock | the action moves last (Part D) |
| Retire local copy, unkept | Show tab · Cancel | — |
| Retire local copy, kept | Cancel · Retire | — |
| Discard a draft | Cancel · Discard | — |
| Discard your repair? | Cancel · Discard | — |
| Discard this reply or comment? | Discard · Keep editing | — |
| Recovery off | Continue · Cancel | — |
| Replace your edits | Keep my edits · Replace | — |

**(owner's eye):** on the Confirmation dialog and Unlock key, Tab from
the body now reaches Cancel or Not now before the action (decision 15).
Enter still presses the action.

**Hand-drawn focus rings.** The three rings the app draws today (fact 5:
Create's held button, Quit now, the composer's Keep editing) move with
their buttons into the footer and keep drawing. The on-screen checklist
names them (`MH-NFR-005`).

### A4. Escape and focus stay as they are

`05` says dialogs close on Escape "unless an operation is running". This
Cycle keeps fact 2's rules on every dialog, and `05` is amended (decision
7). Fact 3's focus returns are kept as they are; this Cycle does not move
a dialog from the Kit's restore to an `on_close` origin or back.

### A5. The details sheet's header

The details sheet's title slot takes `frame::sheet_header` with today's
heading and, as description, today's `sheet-description` text moved out
of the body. Both ids stay. The footer and body do not change. The header
holds no control, so the sheet's Tab stops are as today
(`keyboard_tests/frame.rs:302`).

Cycle 04's Settings header (`D/view/settings/frame.rs`, fact 23a) takes
`frame::sheet_header` too, with its heading and note and their ids
(R-20).

### A6. Inputs

Kit inputs are already 32 px (fact 37). A2's `ring` and `input.border`
give them `05`'s teal focus and `border-strong` border. The app draws no
ring of its own around Kit inputs (R-3; K-4).

## Part B: The Add Repository Sheet

### B1. The owner

Cycle 04's `SheetOwner` gains a case, and its test mirror
`SheetOwnerKind` (C4 implementation plan, Task 1) gains
`AddRepository`:

```rust
pub(super) enum SheetOwner {
    Details(WeakEntity<ItemPane>),
    Settings,
    /// The Add repository sheet (this Cycle).
    AddRepository,
}
```

`sync_sheet` (C4 A1) gains three cases:

1. **Owner `AddRepository`, and a sheet is active:** kept, whatever item
   is shown.
2. **Owner `AddRepository`, and no sheet is active:** the Kit closed it.
   `add_closed(Kept)` runs (B3), the backstop for a close the Kit made
   without `on_close`.
3. **The Add sheet's `open` flag is set, but the owner is not
   `AddRepository`:** a defensive case. `add_closed(Quiet)` runs.

Every quit path that closes Settings quietly closes the Add sheet with
`add_closed(Quiet)` too, at the same point (C4 R-52).

### B2. Ways in

```rust
/// Where the Add repository sheet opens.
pub(crate) enum AddRoute {
    /// The existing-folder mode, as last left (decision 16).
    New,
    /// The existing-folder mode, `root` filled and checked (AW-4, AW-5).
    Inspect(PathBuf),
}

impl Workspace {
    /// Opens the Add repository sheet. `origin` is where focus returns.
    pub(in crate::desktop::view) fn show_add_repository(
        &mut self,
        route: AddRoute,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );
}
```

It lives in a new `D/view/workspace/add_sheet.rs`. The ways in:

| Way | Trigger | Today (fact 16) | After |
| --- | --- | --- | --- |
| AW-1 (C4 W-5) | Empty Repositories panel "Add or create a repository…" | Settings › Repositories | `show_add_repository(New, None)` |
| AW-2 (C4 W-6) | Toolbar `+` (`add-repository-button`) | Settings › Repositories | `show_add_repository(New, None)` |
| AW-3 (new) | Settings › Repositories "Add or create a repository…" (B11) | — | `show_add_repository(New, None)`, replacing Settings (B3) |
| AW-4 (C4 W-22) | Lifecycle "Open repository details" | Settings, `Inspect(root)` | `show_add_repository(Inspect(root), None)` |
| AW-5 (C4 W-27) | `repo.inspect` / `remote.history` recovery, from an Activity row's button, its row menu or a popover row (Cycle 03b's `choose_activity`) | Settings, `Inspect(root)` | `show_add_repository(Inspect(root), None)` |

`SettingsRoute::Inspect` and `inspect_in_settings` go, and their callers
call `show_add_repository` (R-5). `SettingsRoute::AddRepository` is
renamed `SettingsRoute::AddButton` and lands on Settings' "Add or create
a repository…" button (B11; R-6).

**Opening** (a sketch; B9 is the contract):

1. **Refuses** while a dialog is open or the app is quitting, as
   `show_settings` does (fact 19).
2. **If Settings is open** (owner `Settings`): keep Settings' own
   recorded origin as `add_return_to_settings`, then close Settings
   quietly (`settings_closed(true)`), which empties a typed passphrase
   (fact 20).
3. **If the details sheet is active:** drop it as `show_settings` does
   (C4 A2 step 4), which gives focus back to its chip; its edits are in
   the draft. (No way in can fire while the details sheet is open, since
   the scrim takes clicks and `modal_open` refuses actions; this step is
   defensive, R-21.)
4. **The origin**, by C4 A2's rules, recorded after steps 2 and 3: the
   argument, `settings_origin` (set by Cycle 03b's `choose_activity` for
   AW-5), an open layer's `restore`, the focused handle. For AW-3 the
   origin is Settings' `settings-add-repository` button, used only to
   find Settings again (B3).
5. **If the Add sheet is already open:** apply the route to it, focus
   `add-path`, keep the recorded origin.
6. **Open:** set the owner to `AddRepository` and the sheet's `open`
   flag; `window.open_sheet` with `.size(px(470.))`, the builder
   capturing the `Entity<AddRepositorySheet>`; apply the route; focus
   `add-path` after `open_sheet` has focused Root's handle (fact 34).

**Settings opened while the Add sheet is open.** A recovery chosen in
the Add confirmation (for example `repo.identity_set`, or Check host)
closes the confirmation and calls `show_settings`. `show_settings` gains
a first step: when the owner is `AddRepository`, `add_closed(Quiet)`
runs before Settings opens (G17; R-22). Ctrl+, does nothing while the
Add sheet is open: Cycle 04's Ctrl+, handler (C4 A5) refuses when the
owner is `AddRepository` (R-14).

### B3. Closing and coming back

The sheet closes by Escape, the header's close button, the scrim, Cancel,
a successful add (B10), "Open its settings" (B6), a route into Settings
(B2), or quitting. Escape, the close button and the scrim are the Kit's
(`on_close`), which call `add_closed(Kept)`. Every other close calls
`add_closed` directly; its step 0 closes the Kit sheet.

```rust
pub(super) enum AddClose {
    /// Escape, the close button, the scrim, Cancel: typed text is kept.
    Kept,
    /// A successful add of `root`: typed text is cleared.
    Added(PathBuf),
    /// Open its settings: Settings opens on `root`.
    ToSettings(PathBuf),
    /// Quitting, or Settings opening over it: no focus step.
    Quiet,
}
```

`add_closed(how)`, in `add_sheet.rs`:

0. if the Kit sheet is still active, `window.close_sheet` first;
1. clears the owner and the sheet's `open` flag;
2. clears the pending Add (`submit_pending`) and any check in flight's
   landing (C4 R-48's rule). On `Added`, also clears both modes' typed
   text, the check and the choices; on `Kept`, `ToSettings` and `Quiet`,
   typed text, the check and the choices are kept (decision 16). Quitting
   loses them with the app;
3. unless `Quiet`:
   - **`ToSettings(root)`:** `show_settings(Manage(root), the recorded
     origin)`;
   - **opened from Settings** (`add_return_to_settings` is set):
     `show_settings(route, that origin)`, where `route` is
     `Manage(root)` after `Added(root)` (decision 12) and
     `SettingsRoute::AddButton` otherwise;
   - **otherwise:** focuses the origin, else its region, else the content
     region, by Cycle 04's mechanism for an origin no longer drawn
     (`settings_return`'s rule, C4 R-45).

`show_settings` from step 3 runs only once no dialog is open: B10 defers
the success close until the confirmation has closed.

### B4. The sheet's state

`AddRepositorySheet`, a new entity in `D/view/add_repository.rs`, takes
over everything fact 9 lists for the two forms from `RepositoriesPage`,
unchanged in behaviour unless this Part says otherwise, plus:

- `mode: Mode` (`Existing` or `New`);
- `open: Rc<Cell<bool>>`, set on open and cleared at once by
  `add_closed`; while it is false a landing moves no focus and opens no
  dialog (C4 R-47's rule; G12);
- the branch and remote choices (B7, Part C);
- its own `ScrollHandle` for the body (R-8).

As its own entity it has its own `Generations`, so its reads no longer
share them with the manage section (fact 10). Its read stamps take a new
`ViewScope::AddRepository` (R-9). The module splits as
`add_repository.rs` (state and actions), `add_repository/checks.rs`
(B6's rows), `add_repository/render.rs` and `add_repository/share.rs`
(Part C), each below about 500 lines.

`RepositoriesPage` keeps the manage section, `field`, `button`,
`heading`, `text`, `NAME` and `EMAIL`. Its Add and Create code goes.

### B5. Header and mode

- **Header** (A1): "Add a repository" (`add-sheet-title`); description
  "Bring a folder you already work in, or start a new one. Manyhands
  keeps its tickets and documents in the repository."
  (`add-sheet-description`; R-23).
- **Mode** (`add-mode`): the app's own segmented choice, built as Cycle
  02's filter bar segments are (fact 39): two focusable segments with
  `Role::Tab` and `aria_selected`, `folder-input` "Add an existing
  folder" (`add-mode-existing`) and `folder-plus` "Create a new
  repository" (`add-mode-new`). Each is a Tab stop that Enter or Space
  chooses; exactly one is chosen (R-10). Each mode keeps its typed text
  when the other is shown.

### B6. Add an existing folder

In order:

1. **Folder:** the field `add-path` with `folder` icon, and Browse…
   (`add-browse`) beside it, with a note line under it
   (`add-browse-note`):
   - while the chooser is open: Browse… disabled, the note "Choosing a
     folder…";
   - after a platform failure: Browse… disabled for the session, the note
     today's `BROWSE_UNAVAILABLE`;
   - after a choice that is not text: Browse… stays enabled, the note
     today's `BROWSE_NOT_TEXT` until the next choice (Part G).
2. **Checks** (`add-checks`), 12 px rows, each an icon and a sentence,
   from the check (fact 11):

   | Check state | Rows | Action |
   | --- | --- | --- |
   | `Checking` | ⟳ "Checking the folder…" | — |
   | `Checked`, registered | ✓ "Already added to Manyhands." | Open its settings |
   | `Checked`, not registered | ✓ "This folder is a Git repository." | — |
   | … then configuration Missing | ℹ "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change." | — |
   | … then configuration Valid | ✓ "Manyhands is already set up here." and, when not registered, "It will be added to this computer." | — |
   | … then configuration Invalid | ✗ "Manyhands' settings here need fixing." then each problem's guidance, as today | — |
   | … then no branch checked out (`head_branch` is none) | ✗ "No branch is checked out. Check one out, then check again." | Check again |
   | `Failed(NotRepository)` | ✗ "Not a Git repository." | Create one here |
   | `Failed(other)` | ✗ the code's message, with the code in the tooltip | Choose another |
   | `Busy` | ✗ "Manyhands is busy. Try the check again in a moment." | Retry |
   | A route's root is not text | ✗ "This folder's name can't be used here. Choose another." | Choose another |

   The configuration rows follow the registered row too, so a registered
   repository shows its configuration's state and problems, as today's
   form does (decision 18).
   - **Open its settings** (`add-open-settings`) closes the sheet with
     `add_closed(ToSettings(root))` (B3), which opens Settings ›
     Repositories on that repository (decision 18).
   - **Create one here** (`add-create-here`) switches to the new mode
     with Location the folder's parent and Name its last part, and
     focuses Name (`create-folder-name`). The library's preview still
     refuses a folder that is not empty (R-11; K-12).
   - **Choose another** (`add-choose`) runs Browse… when it is available,
     else focuses `add-path`.
   - **Retry** and **Check again** (`add-retry`) run the check again.
   - A check that lands for text the person has since changed is
     ignored, as today (`generations`).
   - The "Checked out: {branch}" line goes; the branch shows in B7.
   - **(owner's eye):** the rows replace today's "A Git repository, not
     yet added to Manyhands" line and the "Configuration …" label.
3. **Main branch** (B7).
4. **Share changes through** (Part C).
5. **Short code prefix (optional)** (`add-prefix`), as today.
6. **Your name & email** (`add-identity`), only when the check says
   Required: today's `IDENTITY_REQUIRED` text and `add-name`,
   `add-email`.
7. **Note** (`add-note`), `shield-check` in teal: "Removing a repository
   from Manyhands later never deletes, moves or changes its files."

### B7. The main branch

In the existing-folder mode, a Kit `Select` (`add-branch`) with
`git-branch` icon. Its items, built when the check lands:

- the checked-out branch (`head_branch`) first, chosen, tagged
  "Suggested" in teal, also when it has no commits yet and so is missing
  from `local_branches` (fact 11a);
- then every other branch of `local_branches`, drawn greyed with the
  line "Check it out first. Manyhands starts from the branch you have
  checked out." The delegate's `on_will_change` refuses it, and a
  `Confirm` naming it is ignored (fact 38; R-12);
- before a check lands, or when no branch is checked out, the select
  shows nothing chosen (B10 disables Add).

Help under it: "The version everyone shares. Finished tickets and
approved documents are merged here." Tooltip: "Git: the primary branch
used for promotion and ticket closure". **(owner's eye)** (decision 10).

### B8. Create a new repository

In order:

1. **Name** (`create-folder-name`): the new folder's name.
2. **Location** (`create-location`) with Browse… (`create-browse`) and
   its note line (`create-browse-note`), as B6 item 1.
3. **Where** (`create-target`): ✓ "Empty folder will be created at
   {location}/{name}", shown when both are filled. The path is built with
   `Path::join` and shown as typed; the sheet does not check it (R-13).
4. **Main branch** (`create-branch`): today's text field, "main" by
   default, with B7's help and tooltip. It is required (RFC 318).
5. **Short code prefix (optional)** (`create-prefix`), as today.
6. **Your name & email (optional)**: today's `CREATE_IDENTITY` note and
   `create-name`, `create-email`.
7. **Note**, as B6 item 7.

The request is today's `RepoCreate` with `root = location.join(name)`.
**(owner's eye)** (decision 11).

### B9. Where focus goes

**This table is the contract** (Cycle contract 4). B2 and B3 sketch one
way to meet it; Cycle 04's A7 lists what a mechanism must survive, and
all of it applies here. Task 4 writes every row it owns as a test before
it changes any focus code; a row it cannot meet goes back to the
orchestrator and the owner.

Each row's focus is read after at least one draw, and after a second
draw when `focus_lost` or a render-time rule moves focus on the first
(C4 A6's rule).

| G | When | Focus goes to | Test |
| --- | --- | --- | --- |
| G1 | The sheet opens by AW-1 to AW-5 | `add-path`, still there after the next draw | P4 |
| G2 | (defensive, by a direct call) The sheet opens over the details sheet; later it closes | G1; on close, the chip | P3 |
| G3 | The sheet opens from Settings (AW-3) | G1; Settings is closed and a passphrase typed on Sharing & keys is empty | P6 |
| G4 | Opened from Settings, closed by Escape, the close button, the scrim or Cancel | Settings, at Repositories, on `settings-add-repository` | P6 |
| G5 | Opened from Settings, closed by a successful add | Settings at Repositories, its chosen repository the new one, on the control C4's `Manage` route lands on | P6 |
| G6 | G4 or G5, then Settings closes | The origin Settings recorded when it first opened | P6 |
| G7 | Not from Settings, closed by Escape, the close button, the scrim, Cancel or a successful add, the origin drawn | The origin | P5 |
| G8 | G7, the origin an Activity row (AW-5 by button, menu or Enter) | The Activity list, as C4 F5 | P5 |
| G9 | G7, the origin a popover row (AW-5), the footer item still shown or hidden | The footer item, or E1's control (C4 F6, F7; 03b R-49) | P5 |
| G10 | G7, the origin no longer drawn | The origin's own region | P5 |
| G11 | G7 with no origin, or one with no region | The content region | P5 |
| G12 | A check, a Browse answer or the add's outcome lands after the sheet closed | Unchanged; no dialog opens | P8 |
| G13 | The confirmation opened from the sheet closes (Escape, Cancel or Close, or a finish) | Inside the sheet: the button that opened it, else `add-path` | P5 |
| G14 | A successful add whose finish closes the confirmation in the same batch | As G5 or G7, after the confirmation has closed | P5, P6 |
| G15 | Quitting with the sheet, or the sheet and its confirmation, open | As Cycle 04's F12–F14 for Settings | P7 |
| G16 | A select's list is open in the sheet | That list, until it closes; then its trigger | P10 |
| G17 | A route into Settings while the sheet is open (a recovery from its confirmation) | Settings' route, as C4 F1 or F2; the sheet closed quietly | P6 |
| G18 | Any landing (the Kit's deferred restore, `open_created`, a review landing, `focus_lost`) would put focus on a control drawn behind the open sheet | `add-path`, as C4 F16 | P15 |
| G19 | A focus landing while the sheet is open | Unchanged, inside the sheet, as C4 F18 | P15 |
| G20 | Create one here | `create-folder-name` | P9 |

G18 and G19 extend Cycle 04's R-34 and R-46 to the owner
`AddRepository` (fact 23a; R-24).

### B10. The footer and adding

**Footer** (A1): Cancel (`add-cancel`) · Add repository (`add-submit`),
or Create repository (`create-submit`) in the new mode.

**Add repository is enabled** (decision 9) when the folder field is
filled and the state of its check is:

| State | Enabled | On press |
| --- | --- | --- |
| None yet for this text | yes | checks first, then goes on when the check passes (D1 R-T12b) |
| `Checking` | yes | goes on when the check passes (`submit_pending`) |
| `Checked`, not registered, configuration Missing or Valid, a branch checked out | yes | opens the confirmation; with identity Required and its fields empty, focuses `add-name`, as today |
| `Checked`, registered | no | — |
| `Checked`, configuration Invalid | no | — |
| `Checked`, no branch checked out | no | — |
| `Failed(_)`, `Busy`, a root that is not text | no | — |

**Create repository is enabled** when Name, Location and Main branch are
filled. **(owner's eye):** `NEED_FOLDER` and `NEED_BRANCH` go, since the
buttons are disabled instead.

**Adding.** Pressing the enabled button builds today's intent and calls
`confirm_followed` (fact 13) with an origin closure giving the button's
wrapper, else `add-path`. The confirmation opens above the sheet (fact
34). The Workspace keeps the add:

```rust
/// An add confirmed from the sheet, until its request finishes.
struct PendingAdd {
    /// The attempt current when it was recorded; an older one's finish is
    /// ignored (fact 13a).
    attempt: u32,
    /// The root the check named, for Settings and the share.
    root: PathBuf,
    /// The share choice to send on success (Part C), if any.
    share: Option<Option<String>>,
}
// Workspace: pending_adds: HashMap<DesktopRequestId, PendingAdd>
```

- On `Followed::Confirmed { request, .. }`, the sheet records or
  replaces the entry with the request's current attempt (a Prepare again
  replaces it).
- The Workspace's existing operations observer (`operations_changed`,
  fact 13a) looks at each entry's request. A `Finished` of an older
  attempt is ignored. When the recorded attempt has finished, it defers
  its handling with `cx.defer_in(window, …)` until after the
  confirmation's own observer has closed it (fact 13a; R-25), then:
  - **`Finished(Success, Ok, …)`:** sends the share (Part C); and if the
    sheet is open and this add came from it, `add_closed(Added(root))`
    (B3, G5, G7, G14);
  - **anything else:** drops the entry. The sheet stays open with its
    text; Activity shows the failure as today, and focus is where the
    confirmation's close put it (G13).
- An entry recorded for a request that has already finished is handled
  the same way at once.
- Once a quit has started, entries are dropped and nothing is sent.

### B11. Settings › Repositories after the move

The Add and Create forms leave the Repositories page. At their place
the page draws one button, "Add or create a repository…"
(`settings-add-repository`), which runs AW-3. The managed section, the
identity form and Remove are unchanged.

### B12. Keyboard, focus and R-60

- Tab stays in the sheet (the Kit sheet's focus trap); a confirmation
  opened from it traps Tab in turn.
- Escape closes the sheet as Cancel, also while a request runs; nothing
  is cancelled (decision 7). An open select takes Escape first (fact 38).
- Nothing in the workspace acts behind the sheet: `modal_open` already
  counts any active sheet. Ctrl+, does nothing while it is open (R-14).
  Nothing pulls focus behind it (G18, G19).
- The body scrolls on its own handle; Page Up, Page Down, Home and End
  scroll it, as Cycle 04's Settings frame does (P23).
- Enter in `add-path` checks the folder, as today. A Kit button in a
  sheet has no `Confirm` binding (GB `sheet.rs:24`), so the sheet's
  footer buttons and row actions use today's form wrappers, which handle
  Enter (`wrapped`, fact 9).

## Part C: Share Changes Through

### C1. The select

In the existing-folder mode, after B7, a Kit `Select` (`add-share`) with
`cloud` icon:

- one item per remote in the check's `remotes`: the name and, in mono,
  its redacted `fetch_location`, as the Remotes page shows it (facts 11,
  15);
- a remote that is not `publication_eligible` is greyed with "Only SSH
  addresses can be used for sharing." and refused as B7's branches are;
- last, "Keep on this computer".

**Chosen when the check lands** (decision 17): the configuration's
`publication_remote` when it names an eligible remote; otherwise "Keep on
this computer".

Help under it: "You can also choose "Keep on this computer" and set
this up later. Only SSH addresses can be used for sharing."

Not drawn in the new mode (a new repository has no remotes), nor when
the folder is registered.

### C2. After the add

When the add is confirmed (B10), the entry's `share` is:

- `Some(Some(name))` when the chosen remote differs from the
  configuration's `publication_remote`;
- `Some(None)` when "Keep on this computer" is chosen and the
  configuration names a remote, so that the result agrees with what the
  sheet showed (decision 17);
- `None` when the choice equals the configuration's.

On the add's success (B10), a `share` of `Some(name)` submits
`Intent::RemoteSelect { root, name }` on `Lane::User`, as `submit_select`
does (fact 15), with `root` the check's `root`.

The entry lives in the Workspace, so the share is sent also when the
sheet was closed while the add ran (decision 12). It is not kept across
a restart, and it is dropped once a quit starts (K-10).

A failed selection is reported by its own Activity row (fact 15). The
repository stays added; the person fixes sharing in Settings ›
Repositories › Sharing. The row offers whatever recovery the result
carries, as today, and no new action (K-14). **(owner's eye)** (decision
3).

## Part D: The Unlock Key Dialog

`05`'s 2h, built with A1. In order:

1. **Header:** `lock` tile in amber; title "Unlock your key"
   (`passphrase-title`); as description, `waiting_subtitle(repository)`
   only while a share waits, as today (`passphrase-subtitle`).
2. **Key card** (`passphrase-key`): `key-round`, the key's label
   (`passphrase-key-label`) and its fingerprint in mono
   (`passphrase-fingerprint`).
   - `open_dialog` takes the Workspace's `Services` as a new argument,
     which `show_prompt` passes (fact 27). The dialog keeps a `Generations`
     and stamps its read with a new `ViewScope::Unlock`.
   - On open it reads `list_keys` through `services.read(ReadKind::Keys, …)`
     and takes the entry whose `id` is the prompt's `key_id` (fact 28).
   - Until the read lands, or if it fails, finds no entry or has no
     fingerprint, the card shows the label only (R-16). `Services::read`
     retries Busy itself.
   - Today's "Key {id}" line goes.
3. **The reason's guidance** (`passphrase-reason`), as today.
4. **Passphrase:** the label "Passphrase" (`passphrase-label`) and
   today's masked input (`passphrase-input`), focused on open. No show
   and hide toggle (Wave decision 9).
5. **Refused** (`passphrase-refused`): today's `REFUSED` under the field.
   On a refusal, focus goes to the input, also when the person pressed
   Unlock (R-17).
6. **Note** (`passphrase-note`), always, with `shield-check`: "Kept in
   memory only, never written to disk. If you choose Not now, you can
   keep working and unlock later from Needs attention."
7. **Footer:** Not now (`passphrase-cancel`) · Unlock (`passphrase-ok`,
   primary).

Width 440 px. Everything else is today's: Enter in the field submits;
Escape and Not now answer `Cancelled`; a declined key then waits in
Activity and Needs attention with Unlock… (fact 29); prompts are held
while drafts save; quitting answers `Unavailable`; background jobs never
prompt. **(owner's eye)** (decision 14).

## Part E: The Empty Item Area

`empty_pane()` draws, centred above today's text,
`img(APP_ICON_PATH).size(px(56.)).grayscale(true).opacity(0.35)` (facts
31, 41). The text and its id do not change. No test checks it (Wave
Testing); the owner checks it on screen.

## Part F: Copy

Every string is a `pub(crate) const`. New or changed:

| Where | Constant | Text |
| --- | --- | --- |
| Add sheet | `ADD_TITLE` | "Add a repository" |
| | `ADD_DESCRIPTION` | "Bring a folder you already work in, or start a new one. Manyhands keeps its tickets and documents in the repository." |
| | `MODE_EXISTING` | "Add an existing folder" |
| | `MODE_NEW` | "Create a new repository" |
| | `FOLDER` | "Folder" |
| | `BROWSE` | "Browse…" (was "Browse"; `06`'s ellipsis rule, R-18) |
| | `CHOOSING` | "Choosing a folder…" |
| | `CHECKING` | "Checking the folder…" |
| | `IS_REPOSITORY` | "This folder is a Git repository." |
| | `ALREADY_ADDED` | "Already added to Manyhands." |
| | `NOT_SET_UP` | "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change." |
| | `ALREADY_SET_UP` | "Manyhands is already set up here." |
| | `WILL_BE_ADDED` | "It will be added to this computer." |
| | `NEEDS_FIXING` | "Manyhands' settings here need fixing." |
| | `NO_BRANCH` | "No branch is checked out. Check one out, then check again." |
| | `NOT_REPOSITORY` | "Not a Git repository." |
| | `BUSY` | "Manyhands is busy. Try the check again in a moment." |
| | `NOT_TEXT` | "This folder's name can't be used here. Choose another." |
| | `CREATE_HERE`, `CHOOSE_ANOTHER`, `RETRY`, `CHECK_AGAIN`, `OPEN_SETTINGS` | "Create one here", "Choose another", "Retry", "Check again", "Open its settings" |
| | `MAIN_BRANCH` | "Main branch" (was "Primary branch") |
| | `MAIN_BRANCH_HELP` | "The version everyone shares. Finished tickets and approved documents are merged here." |
| | `MAIN_BRANCH_TIP` | "Git: the primary branch used for promotion and ticket closure" |
| | `SUGGESTED` | "Suggested" |
| | `CHECK_OUT_FIRST` | "Check it out first. Manyhands starts from the branch you have checked out." |
| | `SHARE_THROUGH` | "Share changes through" |
| | `SHARE_HELP` | "You can also choose "Keep on this computer" and set this up later. Only SSH addresses can be used for sharing." |
| | `KEEP_LOCAL` | "Keep on this computer" |
| | `SSH_ONLY` | "Only SSH addresses can be used for sharing." |
| | `IDENTITY_HEADING` | "Your name & email" |
| | `REMOVE_NOTE` | "Removing a repository from Manyhands later never deletes, moves or changes its files." |
| | `NAME_FIELD`, `LOCATION` | "Name", "Location" |
| | `WILL_CREATE` | fn: "Empty folder will be created at {path}" |
| | `CANCEL`, `ADD`, `CREATE` | "Cancel", "Add repository", "Create repository" |
| Settings | `ADD_OR_CREATE` (reused) | "Add or create a repository…" |
| Unlock | `TITLE` | "Unlock your key" (was "Unlock key") |
| | `PASSPHRASE` | "Passphrase" (replaces `label_for`) |
| | `NOT_NOW_NOTE` | "Kept in memory only, never written to disk. If you choose Not now, you can keep working and unlock later from Needs attention." |

`06`'s rules: sentence case, no exclamation marks, `…` on an action that
opens a chooser, confirmation or sheet. `ADD` and `CREATE` are the one
exception: `05` draws "Add repository" and "Create repository" without
it, and P20 names them. Kept from today: `IDENTITY_REQUIRED`,
`CREATE_IDENTITY`, `PREFIX`, `NAME`, `EMAIL`, `BROWSE_UNAVAILABLE`,
`BROWSE_NOT_TEXT`, `REFUSED`, `UNLOCK`, `CANCEL` ("Not now").

## Part G: Folded-In Work

From `01M5F3GVDYTH1Q5V5FCS9YF7V3`, the four items Cycle 04 left for this
Cycle (its Part G):

1. **The Browse reason beside each Browse** (B6 item 1, B8 item 2), not
   once at the page bottom (fact 14). P21.
2. **Text while the chooser is open** ("Choosing a folder…"). P21.
3. **Retry for a busy check** (B6's `Busy` row). P9.
4. **A root that is not text** is named with a row, not ignored (B6's
   last row). P22.

Decisions 12 and 16 also settle the ticket's "the forms are not cleared
after a success" and its product question: the sheet clears after a
successful add and keeps text otherwise. The ticket keeps its other
items, with a note (Amendment 4).

## Required Outcomes

| P | Test (module) | Proves |
| --- | --- | --- |
| P1 | `every_dialog_keeps_its_keys_and_focus` (`frame`) | A table over A3's dialogs, Unlock key included: the control focused on open, what Enter does, what Escape does (closes, or waits while a write runs), where focus returns; and that a keyed Create, New folder and Move still close after their write (R-19). Read after a draw |
| P2 | `footer_buttons_tab_in_drawn_order` (`frame`) | On the Confirmation dialog and Unlock key, Tab from the body reaches Cancel or Not now before the action (fails before Task 1 and Task 8) |
| P3 | `add_repository_is_a_sheet_the_workspace_keeps` (`workspace::add_sheet`) | Owner `AddRepository` across frames and after `open_item`; G2 by a direct call |
| P4 | `each_way_in_opens_the_add_sheet` (`workspace::add_sheet`) | AW-1 to AW-5 in a table, AW-5 raised three ways through `choose_activity`: the folder value, a check started for `Inspect`, focus on `add-path` (G1); `Inspect` leaves Settings' chosen repository alone |
| P5 | `focus_returns_to_what_opened_add` (`workspace::add_sheet`) | G7–G11, G13 and G14, one row per origin kind |
| P6 | `add_from_settings_returns_to_settings` (`workspace::add_sheet`) | G3–G6 and G17: a direct `show_add_repository` call while Sharing & keys holds a typed passphrase empties it; each close returns to Settings, success with the new repository chosen; Settings' own close returns to its first origin; a route into Settings closes the sheet quietly |
| P7 | `the_add_sheet_closes_every_way` (`workspace::add_sheet`) | Every close leaves owner `None`, no sheet, `open` false, no pending Add; Kept closes keep typed text and Added clears it (decision 16); quitting closes quietly (G15) |
| P8 | `late_work_moves_no_focus_after_add_closes` (`workspace::add_sheet`) | Press Add while its check is pending, press Escape, let the check land: no confirmation, focus unchanged, reopening shows no pending Add; a Browse answer and an add's success after the close (G12) |
| P9 | `folder_checks_are_rows_with_their_actions` (`add_repository`) | B6's table row by row (a registered folder with its configuration rows; `Busy` set through a test accessor); Create one here (G20), Choose another, Retry, Check again and Open its settings do what B6 says |
| P10 | `the_main_branch_is_the_checked_out_one` (`add_repository`) | The checked-out branch chosen and tagged, also with no commits; another refused (by `on_will_change` and by a `Confirm` naming it); none checked out; G16 |
| P11 | `add_is_enabled_only_when_it_can_go_ahead` (`add_repository`) | B10's table row by row; pressing on an unchecked folder checks first, then opens the confirmation (R-T12b) |
| P12 | `create_joins_location_and_name` (`add_repository`) | `create-target`'s path; the `RepoCreate` root sent; Create's enablement, Main branch required |
| P13 | `share_changes_through_lists_the_folders_remotes` (`add_repository`) | Items and greying from canned checks; decision 17's default; not drawn in the new mode or for a registered folder |
| P14 | `a_chosen_remote_is_selected_after_the_add_succeeds` (`workspace::add_sheet`) | One `RemoteSelect` after success; `name: None` for Keep over a configured remote; none for an unchanged choice or a failed add; one after Prepare again then success; sent also when the sheet closed meanwhile; none once a quit has started; a failed selection leaves the repository registered with its Activity row |
| P15 | `nothing_acts_behind_the_add_sheet` (`workspace::add_sheet`) | Dispatched workspace actions, Ctrl+, among them, change nothing while it is open; G18 and G19 (a deferred restore by the test clock, `open_created`, a review landing) |
| P16 | `add_and_create_by_keyboard` (`add_repository`) | Add with the identity step, and Create, by Tab and Enter only; each recovery action of B6 by keyboard, in a table |
| P17 | `add_sheet_names_no_secret` (`add_repository`) | A remote whose fetch URL carries userinfo: no window text while the sheet and its confirmation are open holds the userinfo, a sentinel passphrase or a private key path |
| P18 | `the_unlock_dialog_names_the_key` (`passphrase`) | Label and fingerprint after the read; the label alone before it (`hold_read(ReadKind::Keys)`), when it fails, with no matching key, or with no fingerprint |
| P19 | `a_refused_passphrase_keeps_focus_in_the_field` (`passphrase`) | An empty passphrase submitted by the Unlock button: `REFUSED` shown, focus in `passphrase-input`, no reply |
| P20 | `frame_copy_uses_the_vocabulary` (`add_repository`) | Part F's constants: sentence case, no `!`, `…` where `06` asks, `ADD` and `CREATE` named as `05`'s exception |
| P21 | `browse_says_why_beside_its_button` (`add_repository`) | The note under each Browse while choosing, when unavailable, and after a choice that is not text (Browse still enabled) |
| P22 | `a_folder_that_is_not_text_is_named` (`add_repository`) | An `Inspect` route with a non-UTF-8 root shows the row and its action (`#[cfg(unix)]`) |
| P23 | `the_add_sheet_scrolls_on_its_own` (`workspace::add_sheet`) | Page Down and End move the sheet's body, not the item area |

Module paths: `desktop::view::frame::tests`,
`desktop::view::workspace::add_sheet::tests`,
`desktop::view::add_repository::tests`, `desktop::view::passphrase::tests`.

## Testing

- **Command.** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`;
  a module run drops `-- --exact`.
- **Layer.** Every test is a desktop unit test. Nothing touches the
  library, so no integration or SSH target changes (`AGENTS.md`).
- **Behaviour, not appearance** (Wave Testing). No test asserts widths,
  colours, icons, dividers, the header's layout or that a view renders.
  The icon tile, the empty pane's icon and the theme slots are checked
  on screen only. P2 checks Tab order, which is behaviour.
- **Focus** is read after at least one draw (B9; C4 A6). Cycle 04's plan
  disagreed with its own design on this; its design governs here.
- **Guards** are tested with dispatched actions, never pointer clicks,
  which the scrim takes, and never keys bound in another context (Cycle
  02 ledger).
- **Window tests with polling on** answer every poll (Cycle 03a handoff).
- **No real waits.** The 0.25 s deferred restore is reached with
  `advance_clock`.
- **Canned checks.** `RepositoryInspectionDto`, `ConfigurationDto` and
  `RemoteDto` have public fields (fact 11), so P9, P10, P13 and P14 build
  them where a real repository cannot give the case (an ineligible
  remote, a configured remote). `Busy` is a read result, set through a
  `#[cfg(test)]` accessor. Real `git2` repositories in a tempdir
  otherwise, as today's form tests do; P10's no-commits row uses a fresh
  `git init` with no commit.
- **Keys.** P18 makes its key with `fixture.generate_key`
  (`D/test_support/fixtures.rs:920`) and asks with that key's id.
- **Moved tests.** Today's Add and Create tests move with the code to
  `D/view/add_repository_tests.rs`; the helpers the manage tests import
  stay where they are (E-1).
- **Unlock evidence** (Wave exit): unlocking and Not now are today's
  `desktop::view::keyboard_tests::passphrase` walks and
  `desktop::view::passphrase::tests::{submit_supplies_the_passphrase_and_releases_the_input, cancel_replies_cancelled}`;
  the declined key then waiting in Activity with Unlock… is today's
  `unlock_clears_the_declined_key_and_retries_the_same_request` and
  `share_after_a_finished_locked_key_clears_its_declined_key`
  (`D/view/operations/outcomes_route_tests.rs:242`, `:287`) and
  `unlock_after_cancelled_prompt`
  (`D/services/scheduler_attention_tests.rs:227`); a refused passphrase is
  P19; a wrong passphrase is today's journey (decision 4), which the
  library pins (`tests/ssh_transport/session.rs:185`, an SSH target this
  Cycle does not run) and the desktop shows as any
  `transport_unavailable` (`D/model/publication_tests.rs:334`). Task 0
  confirms their module paths.

## Named Edits To Pre-Existing Tests

Each existing test this Cycle changes, with why. Any further edit is
recorded in the ledger as E-16 onward (decision 8).

| E | Tests | Change | Task |
| --- | --- | --- | --- |
| E-1 | The Add and Create tests of `D/view/settings/repositories_tests.rs`: `create_in_an_empty_directory_previews_nothing_then_creates_and_refreshes`, `adding_a_repository_without_identity_asks_for_one_and_keeps_it`, `a_repository_with_an_identity_shows_no_identity_fields`, `create_in_a_non_empty_directory_shows_its_typed_failure`, `adding_a_folder_that_is_not_a_repository_shows_its_typed_failure`, `escape_on_the_preview_makes_nothing`, `a_create_whose_directory_changed_since_its_preview_offers_prepare_again`, `pressing_confirm_twice_creates_one_repository`, `a_partial_create_is_completed_by_retry_under_the_same_request`, `browse_fills_the_field_and_a_dismissal_leaves_it_unchanged`, `escape_on_an_add_preview_makes_nothing`, `an_add_whose_repository_was_enabled_since_its_preview_offers_prepare_again`, `add_pressed_at_once_checks_first_and_asks_for_the_identity`, `add_pressed_at_once_on_a_repository_with_an_identity_opens_the_preview` | **Task 3:** move to `add_repository_tests.rs` unchanged but for the helper that opens them (still in Settings). **Task 4:** open the sheet. **Task 5:** see E-1a to E-1e. **Task 6:** Create fills Name and Location | 3, 4, 5, 6 |
| E-1a | `adding_a_folder_that_is_not_a_repository_shows_its_typed_failure` (`:396-411`) | It presses Add after a failed check to see the preview's typed failure; Add is now disabled there. **Assertion removed:** the preview's code; the row's text and the code in its tooltip are asserted instead | 5 |
| E-1b | `adding_a_repository_without_identity_asks_for_one_and_keeps_it` (`:281`, `:303-308`), `add_pressed_at_once_checks_first_and_asks_for_the_identity` (`:808`) | `value("add-branch")` reads the select's chosen branch; the re-check after success (`:303-308`) reopens the sheet, which a success cleared (decision 16) | 4, 5 |
| E-1c | `browse_fills_the_field_and_a_dismissal_leaves_it_unchanged` (`:609-635`) | Add's and Create's Browse are tested one mode at a time; `browse-unavailable` becomes `add-browse-note` | 5, 6 |
| E-1d | The tests that assert `NOT_ADDED`, `CHECKED_OUT` or the configuration label | Assert B6's rows | 5 |
| E-1e | `a_form_control_below_the_fold_is_scrolled_into_view` (`:657`) | Reads the sheet's own scroll and body bounds (R-8) | 4 |
| E-2 | `submitting_with_no_folder_says_so` (`:419`) | Becomes: Add (Task 5) and Create (Task 6) are disabled with nothing typed (decision 9). **Assertion removed:** `NEED_FOLDER` | 5, 6 |
| E-4 | `the_inspect_recovery_opens_the_page_with_its_root_checked` (`:698`) | The recovery opens the Add sheet with the root checked and focus on `add-path` (Task 4); `ALREADY_ADDED` as a row (Task 5) | 4, 5 |
| E-5 | `desktop::view::keyboard_tests::settings::add_an_existing_repository_by_keyboard` (`:52`) | Opens the sheet by the toolbar's `+`; its last step (`:78`) expects focus back on `+`, not the content region (G7) | 4 |
| E-6 | `desktop::view::repositories::tests::rows_and_list_states` (`D/view/repositories_tests.rs:146-151`) | The empty state's action opens the Add sheet, not `Shown::Settings(Repositories)` | 4 |
| E-7 | `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`, through its helper `repositories_toolbar` (`D/view/dock_tests/toolbars/repositories.rs:49`, `:66-70`) | Enter on `+` opens the Add sheet | 4 |
| E-8 | `desktop::view::lifecycle::outcome_canned_tests::each_way_out_offers_its_button` (`:411-417`) | "Open repository details" opens the Add sheet. A D5 test's asserted route changes, against the Wave's risk control for D5's tests; decision 13 is the reason | 4 |
| E-9 | `desktop::view::passphrase::tests::submit_supplies_the_passphrase_and_releases_the_input`, `the_dialog_names_what_waits_and_says_not_now`, `a_finished_confirmation_and_a_prompt_in_one_batch_close_only_the_confirmation` (`:676-677`, `label_for`), `a_prompt_for_a_check_now_names_no_share` (`:785`, `!present("passphrase-note")`) | The label reads `PASSPHRASE`; the note is always shown with its new text, so `:785` asserts it present; the absent toggle stays asserted | 8 |
| E-10 | (none expected) The keyboard walks of the Confirmation dialog and Unlock key (`D/view/keyboard_tests/confirm.rs`, `keyboard_tests/passphrase.rs`) loop with `tab_to` (sweep, 2026-11-14) | Task 0 confirms | 1, 8 |
| E-11 | (none expected) The fixed-count Tab walks (`tabs_tests.rs:1280`, `keyboard_tests/drafts.rs:261`, `:369`, `keyboard_tests/drafts/quit.rs:114`, `item/edit_restored_tests.rs:741`) are on dialogs A3 leaves in order | Task 0 confirms | 2 |
| E-12 | Cycle 04's tests for W-5, W-6, W-22 and W-27 (its E-18 and P4 rows), its P2 `settings_leaves_the_item_as_it_was` (the AddRepository route to `add-path`) and P22 `late_work_moves_no_focus` (Add pressed in Settings with a check pending), and any of its P8 or P2 steps that rely on the forms' length | Expect the Add sheet (AW-1, AW-2, AW-4, AW-5); P2 takes `SettingsRoute::AddButton`; P22's Add case moves to this Cycle's P8 | 4 |
| E-13 | `D/view/keyboard_tests/lifecycle.rs:415`, which calls `open_dialog(prompt, window, cx)` | Passes the `Services` Part D adds | 8 |
| E-14 | `desktop::assets::tests::every_item_view_icon_loads` | Its `ICONS` list gains the five icons (fact 40) | 1 |
| E-15 | `desktop::view::workspace::tests` that pin Settings' routes Part B moves (found by Task 0's sweep) | Follow AW-1 to AW-5 | 4 |

E-3 is not used (revision 1's E-3 is E-1e).

## Document Amendments

Applied by the planner, after the owner's approval:

1. **Wave 04** (applied in Task 0, after Cycle 04's own amendment of the
   same section has merged):
   - the Cycle 05 section split into 05a and 05b (decision 2), the
     Cycles table and dependencies updated, the planned documents' paths
     and both tickets named;
   - 05a's exit evidence: "a wrong passphrase" is today's journey
     (decision 4);
   - "Add repository sheet": the remote select sends today's selection
     after the add (decisions 3, 17); the inspect routes land on the sheet
     with Open its settings (decisions 13, 18);
   - "Common frame": each dialog keeps its Escape rule and its button
     order, except the Confirmation dialog and Unlock key (decisions 7,
     15); the Add sheet closes on Escape as Cancel;
   - "Unlock key dialog": the note's wording (decision 14).
2. **`docs/design/05-sheets-and-dialogs.md`** (approval commit): "As
   built, Wave 04 Cycle 05a" notes:
   - common frame: button order as today, except the Confirmation dialog
     and Unlock key (action last); one scrim colour; the Kit's ring in
     teal; the footer drawn in the dialog's body, so it scrolls with it
     (K-11); a sheet's dividers stop short of its edges (K-5); Escape per
     decision 7; the slots K-13 leaves to the Kit;
   - Add repository: decisions 3, 9–13 and 16–18, the rows' wording, no
     writability check, no File menu entry;
   - Unlock key: decisions 4 and 14.
3. **`docs/design/08-gpui-kit-mapping.md`** (approval commit): the Add
   sheet's mode control is the app's own segmented choice, as Cycle 02's
   (fact 39).
4. **Cycle 04's documents** (Task 10, once they are on main): its Cycle
   document's Downstream Obligations marked "Done in Cycle 05a"; a note
   on its design's A2 (`SettingsRoute::AddRepository` renamed
   `AddButton`, `Inspect` removed) and Part D rows W-5, W-6, W-22 and
   W-27 (moved to the Add sheet).
5. **Ticket `01M5F3GVDYTH1Q5V5FCS9YF7V3`** (Task 10): a comment that its
   four Add repository items, and its "forms are not cleared after
   success" item with its product question, are done here (Part G).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Button order stays as today, except the Confirmation dialog and Unlock key (decision 15) | — (owner's decision) |
| R-2 | Theme slots set once through `apply_config`, both modes, the current mode last | A future theme switch must rebuild the config |
| R-3 | No app-drawn ring around Kit inputs; the Kit's ring in teal | `05`'s 2 px at 18% is not met (K-4) |
| R-4 | Every element id stays | — |
| R-5 | `SettingsRoute::Inspect` and `inspect_in_settings` go; their callers call `show_add_repository` | Cycle 04's tests for those routes change (E-12) |
| R-6 | `SettingsRoute::AddRepository` is renamed `AddButton` and lands on Settings' Add button | Cycle 04's tests naming it change (E-12) |
| R-7 | Settled by decision 16 | — |
| R-8 | The sheet body has its own scroll handle | — |
| R-9 | The sheet's read stamps take `ViewScope::AddRepository` | — |
| R-10 | The mode control is the app's own segmented choice, as Cycle 02's | — |
| R-11 | Create one here does not check that the folder is empty; the preview refuses it | The person learns it from the confirmation (K-12) |
| R-12 | Branches other than the checked-out one are listed, greyed and refused | The Kit select still lets the highlight land on them |
| R-13 | `create-target` shows the joined path unchecked | A bad path is found by the preview |
| R-14 | Ctrl+, does nothing while the Add sheet is open **(owner's eye)** | Settings is reached by closing the sheet first |
| R-15 | Settled by decision 17 | — |
| R-16 | The fingerprint is read from `list_keys` when the dialog opens | One extra read per prompt |
| R-17 | A refusal focuses the input | — |
| R-18 | "Browse" becomes "Browse…" | Tests reading the literal change (E-1) |
| R-19 | A dialog's footer is drawn by its own view, as the last child of its body element; the Kit's `footer` slot is not used | The footer scrolls with the body in a dialog taller than the window (K-11) |
| R-20 | Cycle 04's Settings header takes `frame::sheet_header` | — |
| R-21 | Opening over the details sheet is defensive only; no way in reaches it | — |
| R-22 | `show_settings` closes an open Add sheet quietly first | Text typed in the sheet is kept for its next open (decision 16) |
| R-23 | The header's description completes frame 1d's cut-off sentence | The owner may word it differently |
| R-24 | Cycle 04's R-34 and R-46 apply to the owner `AddRepository` | — |
| R-25 | An add's success is handled after the confirmation has closed (`defer_in`) | — |

## Known Limits

- **K-1:** no "You can save changes here" check (fact 11).
- **K-2:** no inline error for a wrong passphrase (decision 4; fact 30).
- **K-3:** one scrim colour, `#0a0b0d99`; a dialog over a sheet draws two
  scrims (fact 36).
- **K-4:** the Kit's 3 px focus ring at 50%, in teal (fact 37).
- **K-5:** a sheet's header and footer dividers stop short of its edges
  (fact 33).
- **K-6:** the Unlock note promises no "until you quit"
  (`01M70CF14M3YC6SR8T9JJ6XQBG`).
- **K-7:** "Open repository details" lands on the Add sheet's check, with
  Open its settings (decisions 13, 18).
- **K-8:** the mode control is two Tab stops, not one with arrow keys,
  as Cycle 02's segments are.
- **K-9:** no File › Add repository entry (no title-bar menus).
- **K-10:** a share choice waiting for its add is lost if the app quits
  before the add finishes; the person chooses again in Settings.
- **K-11:** a dialog's footer scrolls with its body when the dialog is
  taller than the window (R-19).
- **K-12:** Create one here is offered on any folder that is not a
  repository, though the library refuses one that is not empty (R-11).
- **K-13:** any of the secondary button, input background or sheet
  border and shadow that `ThemeConfig` has no key for stays the Kit's
  (A2; U-7).
- **K-14:** a failed share selection after an add offers no new action;
  the person fixes it in Settings › Repositories › Sharing (C2).

## Unverified

- **U-1:** Cycle 04's merged names, signatures and lines (facts 18–24,
  23a): `SheetOwner`, `SheetOwnerKind`, `show_settings`,
  `settings_closed`, `SettingsRoute`, `settings_origin`,
  `settings_return`, the host's `open` flag, its Ctrl+, handler, its
  `frame.rs`, W-36. Task 0 re-maps them and corrects this design first.
- **U-2:** which theme mode the app shows on screen, given fact 8. A2
  sets both modes and leaves the current one in force; the owner's
  on-screen check confirms.
- **U-3:** A3's footer table, per dialog, against the code at Task 0
  (the sweep of 2026-11-14 read it at `ea6d899b`).
- **U-4:** the exact module paths of the Unlock evidence tests (Testing).
  Task 0 confirms them.
- **U-5:** (not used; revision 1's ToggleGroup question is settled by
  fact 39).
- **U-6:** the five icons load once added (fact 40); E-14's test proves
  it.
- **U-7:** `Theme::sync_base` and the `ThemeConfig` keys for the
  secondary button, the input background and the sheet border (A2).
  Task 1 reads the schema; a slot with no key is K-13.

## Alternatives Rejected

- **A custom 2 px focus ring around every input.** Every input in the
  app would need a wrapper reading its focus; the Kit ring in teal is one
  setting (R-3).
- **The Kit's `footer` slot for dialogs.** It sits outside the body, so
  the focus-gated closes of Create, New folder and Move and the path-based
  privacy sweeps would break (R-19).
- **The Kit's `ToggleGroup` for the mode.** It has no keyboard path
  (fact 39).
- **Showing only the checked-out branch.** `05` and the RFC ask for a
  select that shows the choice; listing the others with the reason tells
  the person why they cannot pick them (R-12).
- **`origin` as the default remote.** The owner chose the configured
  remote, else Keep on this computer (decision 17).
- **Leaving Add in Settings and adding a sheet beside it.** Two ways to
  add would disagree.
- **Inspect routes to Settings' chosen repository.** The routes show the
  folder check today, and the chosen repository's page does not show the
  configuration's problems; Open its settings reaches it in one step
  (decisions 13, 18).

## Risks

| Risk | Control |
| --- | --- |
| Cycle 04's merged code differs from its design | Task 0 re-maps every fact 18–24 name and corrects this design before Task 1 |
| Focus between the Add sheet, Settings and the confirmation goes wrong (Cycle 04's three Blocking rounds were all focus) | B9 is a table of required outcomes; Task 4 writes it test-first under a design checkpoint |
| A changed Tab order breaks keyboard tests nobody named | Only two dialogs change order (decision 15); Task 0's sweep; decision 8 records each extra edit |
| The success close races the confirmation's own close | R-25 defers it; P5 and P6 test the same-batch case (G14) |
| An older attempt's finish drops the share or the close | `PendingAdd.attempt` (B10); P14's Prepare again row |
| The Kit select's disabled items still confirm | `on_will_change` refusal plus ignoring a refused `Confirm` (R-12); P10 tests both |
| The theme config switches the app's mode or ignores a colour | The current mode's config goes last; hex colours only (A2) |
| Moving 560 lines breaks the manage section's shared helpers | The helpers stay in `RepositoriesPage`; the manage tests' imports stay (E-1); Task 3 moves the state without changing where it is drawn |

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning research, 2026-11-14 | Facts 1–41 | Facts | Revision 1 |
| Test sweep, 2026-11-14 | Footers in the Kit slot break focus-gated closes and privacy sweeps; several dialogs put the safe choice first on purpose | Facts; owner question | R-19; decision 15 |
| Review round 1, source and feasibility (revision 1, `9e22c765`) | 1 Blocking (the Kit `ToggleGroup` has no keyboard path), 9 Important (the success close races the confirmation; an older attempt's finish; no-commit repositories; Keep over a configured remote; `apply_config` switches the mode and drops `rgba`; task order; E-1's hidden removals; missed tests; P6 could not fail), 12 Minor | Blocking, Important, Minor | Revision 2: facts 11a, 13a, 36, 39, 40; A2; B2, B3, B5, B7, B10; Part C; Part D; P-list; E-list; plan Tasks 3–6 |
| Review round 1, contract and consistency (revision 1, `9e22c765`) | 2 Blocking (Unlock's exit evidence unproven; nothing kept focus from behind the Add sheet), 19 Important (B7 against B10; Create's branch; registered folders' configuration; R-7; success tracking; Cycle 04's origins F5–F8; origin order; Settings over the sheet; Keep; the as-built note; the mode control; secondary slots; Task 3's size; P16; decision 6 against decision 5; decision 12's reading; contract 1), 22 Minor | Blocking, Important, Minor; owner questions | Revision 2: Testing's Unlock evidence; G8, G9, G17–G20; R-22, R-24, R-25; B6, B8, B10; Part C; A2; amendments; plan Task 3 split; decisions 15–18 |
| Owner, 2026-11-14, after round 1 | Button order; typed text on close; the default remote and Keep; registered folders on the inspect routes | Owner decisions | Decisions 15–18 |
