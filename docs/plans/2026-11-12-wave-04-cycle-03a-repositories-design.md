---
title: "Wave 04 Cycle 03a Repositories Panel And Sharing Status Design"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7888X5VZ7PXGT4XWGS9TMJS"
---

# Wave 04 Cycle 03a Repositories Panel And Sharing Status Design

## Intent And Approval Boundary

This design covers [Cycle 03a](../Cycles/wave-04-cycle-03a-repositories.md):
the Repositories panel rebuilt to `docs/design/03-panels.md`, with one
plain-language state per repository, a toolbar, a context menu, and the
footer's "N to share" and "Checked for updates …" items. The repository
details block leaves the item area, and its facts and actions move to the
panel. Approval covers the mechanisms, rulings, named test edits and task
order. It does not authorize implementation, pushes beyond the ticket
branch, merging or closing.

**Draft.** Grounded at the ticket branch `7034522f`, which holds
origin/main `37be101d` (Cycle 01, Cycle 02 and Wave 03 D6 merged). Not yet
reviewed.

Conventions:

- `D/` is `src/desktop/`; `L/` is the library under `src/`.
- `C/` and `B/` are `gpui-component-0.6.6/src/` and `gpui-base-0.6.6/src/`
  under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (the
  locked crates).
- "Fact n" refers to Baseline Facts. "P-n", "R-n", "E-n", "K-n" and "U-n"
  are this design's Required Outcomes, Rulings, Named Edits, Known Limits
  and unverified facts. "Decision n" is a Cycle 03a decision; "Wave
  decision n" is Wave 04's; "C1 R-n" and "C2 R-n" are rulings of the Cycle
  01 and Cycle 02 designs; "D6 X" is a section of the Wave 03 D6 design.
- "The handoff" is `docs/design/`; `02` is `02-layout-and-docking.md`,
  `03` is `03-panels.md`, `06` is `06-language-and-states.md`.
- `{repo}` is the repository's display name, the one the panel and the
  window title show (C1 R-14): `RepositoryEntry::name()`, the folder name.
- A "check" is one poll of a repository's shared copy, automatic or by
  Check now (D6). "The schedule" is D6's `PollSchedule`.

## Baseline Facts At `7034522f`

**The panel today.**

1. **No panel type.** The Repositories panel is the generic `RegionPanel`
   with `Slot::Panel(PanelKind::Repositories)` (`D/view/dock/panels.rs:58`,
   `:67`); its body is `Workspace::repositories_region`, which calls
   `sidebar::repositories(&self.navigation, &self.repositories_pane, cx)`
   (`D/view/workspace/render.rs:68-73`). The header count is the row count
   (`D/view/dock/panels.rs:171`).
2. **`sidebar::repositories`** (`D/view/sidebar.rs:69`) draws an in-body
   "Repositories" heading (`REPOSITORIES_HEADING`, `:35`), the Kit list
   (`"repositories-list"`, `:332`), a state line (`"repositories-state"`
   `:87`, `"repositories-last-read"` `:97` "Showing the last list read",
   `"repositories-failure"` `:112`, `"repositories-retry"` `:121`), and a
   ghost Settings button (`"sidebar-settings"`, `:146`) that opens
   `SettingsPage::Repositories`.
3. **Rows.** `RepositoryRow { pub key, name, unavailable, selected }`
   (`:161`); `RepositoryDelegate { rows, highlighted }` (`:196`) with
   `type Item = ListItem` (`:207`) and no `render_empty` override, so an
   empty list draws the Kit's Inbox icon (`C/list/delegate.rs:74`). A row
   shows the folder name, a "Selected" badge (`("repo-selected", i)`,
   `:233`), an "Unavailable" badge (`("repo-unavailable", i)`, `:225`) and
   the root path (`("repo-root", i)`, `:262`); `("repo-row", i)` (`:244`)
   carries `aria_description(SELECTED)` when chosen and no
   `aria_selected`. No icon, sharing state or tooltip.
4. **`RepositoriesPane`** (`:286`) owns the Kit `ListState`; `set(rows,
   window, cx)` (`:308`) returns early on equal rows, keeps the highlight
   by key and notifies. It is called from `Workspace::render` on every
   frame (`D/view/workspace/render.rs:205-206`). The Confirm subscription
   (`D/view/workspace.rs:267-274`) calls `select_repository(key)`.
5. **The repository details block.** `sidebar::selection` (`:343`) draws
   the name (`"selection-name"`), root, and by `Resolution`: "Opening",
   the unavailable code's message with Retry and Remove, busy with Retry,
   a failure with Retry, or the configuration label, primary branch,
   problems' guidance, index state and Share main
   (`"selection-share-main"`, `ShareMain`, `:534`). It is drawn in the
   item area when no tab is shown (`render.rs:121-130`;
   `D/view/workspace/content.rs:671-675`) and at the end of every item
   tab's scroll region (`content.rs:623-629`, `repository_details`,
   attached by `open_item` `:310` and `follow_tab` `:330`).
6. **Constants imported elsewhere.** `RETRY`, `SHARING` and
   `configuration_label` from `sidebar.rs` are used by `drafts.rs:43`,
   `item/draft.rs:56`, `item/status.rs:19`, `operations.rs:630,644,775`,
   `operations/outcomes.rs:45`, `item/share.rs:57`,
   `settings/repositories.rs:68` and tests.

**Navigation.**

7. `Resolution { Unknown, Resolved(ResolvedRepository), Unavailable(ResultCode),
   Busy, Failed(ReadFailure) }` (`D/model/navigation.rs:35`); `Unavailable`
   covers `InvalidPath`, `NotRepository`, `RepositoryInaccessible`,
   `RepositoryNotRegistered` (`:26`). Only the selected repository is
   resolved (`select` `:348`, `retry_resolution` `:383`; the reads in
   `D/view/workspace/repositories.rs:111-131`).
8. `RepositoryEntry { summary: RepositorySummaryDto, resolution }`
   (`:63`). `RepositorySummaryDto { root, enabled_at, accessibility,
   configuration: ConfigurationDto, index, problem_count }` and
   `ConfigurationDto { state: Valid|Invalid|Missing, primary_branch,
   publication_remote: Option<String>, problems }`
   (`L/repository/read/dto.rs:119`, `:131`) come from `list_repositories`
   for every registered repository. `accessibility` is never written as
   anything but `accessible` (open ticket `01M4EHGE2KDCBRZ9DMXBPP30R0`).
9. `Navigation.repositories` is a plain `ListState<Vec<RepositoryEntry>>`
   with `not_reloaded: Option<NotReloaded { Busy, Failed }>` (`:152`,
   `:175`), not Cycle 02's `KeptList`.

**Cycle 02's shared parts.**

10. `ListRow` (`D/view/rows.rs:52`): `new`, `indent`, `gap`, `focused`,
    `expanded`, `label`, `description`, `tooltip`, `on_click`,
    `Selectable::selected`; it always sets `aria_selected(selected)`
    (`:175`). Helpers `icon` (`:216`), `named_icon` (`:226`),
    `skeleton_rows(id, count, label, cx)` (`:252`), `alert_row(id, &Alert,
    on_action, cx)` (`:285`, action id `(id, "action")`), `empty_state(id,
    text, Option<EmptyAction>, cx)` (`:351`).
11. `list_body` (`D/model/list_view.rs:195`) takes a `KeptList` and
    `BodyInput { kind: ListKind, repo, … }`; `ListKind` is `Tickets |
    Documents` (`:24`) and its texts are written for those lists.
    `list_state::LABEL_READY` is "Up to date" (`D/model/list_state.rs:17`).
12. **Tests cannot read a tooltip or an accessible description.** The Kit
    `ElementSnapshot` holds role, path, checked, selected, expanded,
    value, bounds, visible, focused, disabled and label only
    (`B/test_support.rs:18-32`); the Kit `Tooltip` is not registered with
    test support. Tooltip and description text is tested through its pure
    function (C2 fact 11; `D/view/polling_privacy_tests.rs:16-17`).

**Toolbars, focus and menus.**

13. `ToolbarButton { NewTicket, NewDocument, NewFolder, CollapseAll }` with
    `ALL: [Self; 4]`, `id`, `index`, `region`
    (`D/view/workspace/focus.rs:111-145`); `toolbar_focus: [FocusHandle;
    4]` (`D/view/workspace.rs:135`, `:338`, getter `:438`).
    `ToolbarButton::of(kind)` returns `&[]` for Repositories and Activity
    (`D/view/dock/header/toolbar.rs:65-69`). Buttons are base `Button`s on
    Workspace-owned handles in a wrapper with `NAVIGATION_CONTEXT` that
    stops mouse-down (`:77-140`); `press_toolbar` returns when
    `modal_open` or disabled (`:156`). `focused_region` counts a focused
    toolbar handle as its region (`focus.rs:182-192`); `focus_region` for
    Repositories focuses the Kit list only when it has rows (`:204`).
14. **The dock menu** (C1) is the pattern for a keyboard-reachable popup:
    `DockMenu::open(panel, current, toggle, placement, restore, window,
    cx)` builds a `PopupMenu` with `action_context(toggle)` and subscribes
    to `DismissEvent` (`D/view/footer/menu.rs:60`); `layer()` draws it in
    `deferred(anchored()…)` at `POPUP_PRIORITY` (`:98`). It opens on a
    right button through `capture_any_mouse_down` with `prevent_default`
    at the pointer (`D/view/workspace/footer.rs:74-140`), and by `menu`
    and `shift-f10` bound to `OpenDockMenu` in `PANEL_TOGGLE_CONTEXT`
    (`D/keys.rs:261-262`) at bounds recorded by a `canvas` during prepaint
    (`D/view/footer.rs:268-281`; `open_focused_dock_menu`,
    `workspace/footer.rs:187`). `dock_menu_layer` drops it under a modal
    (`:244`). Tests open it with `right_click(id)` or by dispatching the
    action and read items by index (`footer_tests/mod.rs:142-233`).
15. The Kit has `ContextMenuExt::context_menu(f)`
    (`C/menu/context_menu.rs:13-33`), opened by a right click only; the
    app does not use it.
16. `unless_dialog` and `modal_open` (`D/view/workspace/render.rs:168`,
    `:174`) refuse actions behind a dialog or sheet.

**Repository actions.**

17. `share_main(root, remote, window, cx)` (`D/view/workspace/requests.rs:313`)
    submits `Intent::RepoSync`, or with no publication remote opens
    Settings › Remotes (`:285`); `sharing_main(root)` (`:303`) is true
    while one is queued or running. `repo sync` synchronizes main only;
    an item is shared by `item sync` (`docs/RFC/cli-contract.md:180`).
18. `remove_repository(root, window, cx)` (`requests.rs:357`) guards on
    `has_active_dialog` only, sets `last_region = Content` and confirms
    `Intent::RepoRemove` returning focus to `remove_focus`; the dialog's
    title is "Remove repository" (`D/services/requests.rs:424`) with the
    note "Local files remain: only the registration is removed. Nothing in
    the folder or in Git is changed." (`D/view/confirm.rs:74`).
19. `maintain_index(rebuild, cx)` (`requests.rs:338`) submits
    `IndexRefresh` or `IndexRebuild` for the selected repository.
20. `open_settings(page, window, cx)` (`D/view/workspace/content.rs:188`);
    `SettingsPage { Repositories, Remotes, Keys, Hosts, Polling }`
    (`D/model/stamp.rs:19`), `Polling` being "Checking for updates".
21. `item::lock::open_folder(folder, cx)` (`D/view/item/lock.rs:81`)
    opens a folder with the system and, in tests, records it in
    `OpenedFolders` (`:96`).

**D6's model.**

22. `PollSchedule::repo(root) -> Option<&RepoSchedule>`
    (`D/model/polling.rs:99`); `RepoSchedule { policy, phase, last:
    Option<AttemptSummary>, last_completed, conflict_pending, … }`
    (`D/model/polling/state.rs:100`); `Phase { Idle, Queued, Running,
    Waiting, Preempted, Attention(Reason), Retiring, Stopped }` (`:66`);
    `Reason { KeyLocked(KeyId), KeyProblem(KeyId), NoKeySelected(Root),
    HostUnapproved(Authority), HostUnidentified(Root),
    ConfigurationRequired(Root), RecoveryHeld(Root) }` (`:51`). The model's
    methods are declared `pub fn` on `pub(crate)` types.
23. `AttemptSummary { answer, completed_at, brought_in, up_to_date,
    not_shared, listed, retire, index_pending }`
    (`D/model/polling/attempt.rs:159`). `of` counts `not_shared` for
    `LocalAhead` only; `Unpublished` goes to `listed` (`:196-208`).
    `AttemptTarget { item_id, kind, disposition }` (`:133`) carries no
    title.
24. `status::row(&Facts) -> Row { status, detail, pending, shape }`
    (`D/model/polling/status.rs:250`) with `Facts { name, schedule,
    unplaced, holding, checking_now, now }` (`:80`) and `Shape {
    NoSharedCopy, Pending, Off, Held, Polling(Controls) }` (`:96`);
    `Controls { paused, interval_seconds, retry, checking }` (`:111`).
    `checked` writes "Checked {ago} · up to date" and "· N not shared yet"
    (`:154`); `after_last` maps an `Unreachable` last answer to
    `cant_reach(name)` "Can't reach {repo}. Your work is safe." (`:63`,
    `:174`); `since_library` writes "Checked {ago}" or "Not checked yet"
    when this process has seen no attempt (`:197`). `ago` is private
    (`:134`).
25. `Unplaced { Reading, Unreadable }` (`:72`): the scheduler is reading
    the repository's remotes, or the read failed.

**The scheduler.**

26. `Scheduler::schedule()`, `holding(root)`, `unplaced(root)`,
    `refreshing(root)`, `utc()` (`D/services/scheduler/entity.rs:178-217`);
    test-only `schedule_mut()` (`:228`). `unplaced` is `None` while polling
    is off, which it is unless `Services::with_polling()`
    (`D/services/mod.rs:847`) was called; production calls it
    (`D/mod.rs:143`).
27. `registration::read` (`D/services/scheduler/registration.rs:101`)
    resolves the repository, reads its publication remotes with
    `Backend::publication_remotes` (the library's `list_remotes_redacted`)
    and its `polling_status`; `policy_of` (`:124`) keeps the policy and
    drops `latest_outcome`. A repository is scheduled only when a remote is
    `selected_for_publication && publication_eligible` (SSH fetch and push,
    `ssh_publication`, `:38`). `registered` keeps a side map `authorities:
    BTreeMap<Root, Option<Authority>>` (`entity.rs:110`, `:360`).
28. `PollingStatusDto.latest_outcome: Option<PollingOutcome>`
    (`L/repository/read/dto.rs:715`), one of `Completed,
    ConfigurationRequired, SelectedKeyUnavailable, UnlockRequired,
    HostApprovalRequired, TransportUnavailable, ProtocolRejected,
    Cancelled, RepositoryUnavailable` (`:689`), with no time stored. The
    desktop maps replayed rows by category (`D/services/poll_convert.rs:390`)
    but nothing maps the DTO's `latest_outcome`.
29. `heard(event, operations) -> Option<Heard>`
    (`D/services/scheduler/reconfigure.rs:280`) turns worker events into
    scheduler events; a finished `ItemSync` or `RepoSync` is none of them,
    so a share changes nothing in the schedule.
30. Check now: `Intent::PollOnce { root }` on `Lane::User`, sent by
    Settings' `check_now` (`D/view/settings/polling.rs:225`) unless a
    dialog is open or one is in flight; `checking_now` is the worker's
    in-flight `PollOnce` for the root (`:199`). The test double
    `PollingBackend` passes `polling_status` through to the library
    (`D/test_support/polling.rs:404`).

**The footer.**

31. `Control { Dock(Side), Panel(PanelKind), Attention, Settings }` with
    `ALL: [Control; 9]` and ids `footer-…` (`D/view/footer.rs:42-66`);
    `plan(layout, attention) -> Plan { left, right }` puts `[Attention]`
    (when above 0), the right dock group and `[Settings]` on the right
    (`:132`). `Workspace::footer` fills a Kit `StatusBar`, whose centre is
    the spacer (`D/view/workspace/footer.rs:40`); `press` refuses under a
    dialog or sheet and matches every control (`:141`). Exhaustive matches
    on `Control`: `footer.rs:66,82,213`, `workspace/footer.rs:82,150`,
    `keyboard_tests/shell/support.rs:93`.

**Library reads for titles.**

32. `ReadKind::Item` reads `show_item`; `ReadKind::MadeReady` is D5's
    precedent for reading titles apart from the tabs' item reads
    (`D/services/mod.rs:91-125`).

## Part A: Sharing Facts The Scheduler Keeps

Rows read D6's model, never their own guess (D6 W-5). Four small additions
let the model say what the panel needs.

- **The stored outcome.** `registration::read` already holds
  `PollingStatusDto`; the scheduler keeps its `latest_outcome` in a side
  map beside `authorities`, `stored: BTreeMap<Root, PollingOutcome>`,
  filled wherever the status is read (`registered`, `answered`), and
  dropped with the repository (R-1). `Policy` is not changed, so its six
  literals stay as they are. `status::Facts` gains `stored:
  Option<PollingOutcome>`, read only when `last` is `None`
  (`since_library`), so Settings and the panel say the same (decisions
  9, 10):

  | Stored outcome, no attempt this session | Settings line | Panel state |
  | --- | --- | --- |
  | `TransportUnavailable`, `ProtocolRejected` | `cant_reach(name)` | Can't reach |
  | `UnlockRequired` | "Needs unlocking" | Locked |
  | `SelectedKeyUnavailable`, `HostApprovalRequired`, `ConfigurationRequired` | "Choose a key", "Needs host approval", "Can't check" | Needs attention |
  | `RepositoryUnavailable` | "Can't check" | Can't reach |
  | `Completed`, `Cancelled`, none | today's text | no state from it |

- **The shared copy.** `registered` also keeps the selected publication
  remote's name and redacted push location,
  `shared_copies: BTreeMap<Root, SharedCopy { name, location }>`, from the
  same `publication_remotes` read (the library's `list_remotes_redacted`,
  fact 27). No new read (R-2).
- **What is not shared yet.** `AttemptSummary` gains `to_share:
  Vec<AttemptTarget>`: every `LocalAhead` and `Unpublished` target of the
  attempt, main included (a target with no item). The `not_shared` field
  becomes a method returning `to_share.len()`, so Settings' "· N not
  shared yet" counts the same
  items as the panel (decision 10). `Unpublished` targets stay in `listed`
  too, so Activity's attempt entry is unchanged.
- **A finished share.** `heard` gains `Heard::Shared { root, item:
  Option<ItemId> }` for an `ItemSync` (its item) or a `RepoSync` (main,
  `None`) that finished with `Outcome::Success`. The scheduler removes the
  matching target from `last.to_share` (R-3; decision 11). A failed or
  partial share removes nothing.
- **Seam.** `PollingBackend` gains `store_outcome(root, outcome)`, which
  its `polling_status` applies to the library's answer, so a test sets a
  stored outcome without SQL (R-4).

## Part B: Repository State

One pure function decides each row's state, in the order the owner agreed
(decision 4), with one change for feedback (decision 8):

`repository_state(&RepoFacts) -> RepoState`, in `D/model/repository_state.rs`.

`RepoFacts<'a>` holds what the Workspace already has: the `RepositoryEntry`
(summary and resolution), the scheduler's `RepoSchedule`, `Unplaced`,
`Holding`, stored outcome and `SharedCopy` for the root, `checking_now`,
`sharing_main`, whether polling is on, the titles known for the
repository's items (Part C), and `now`.

| # | When | State | Icon, tone | Action (leads the menu) |
| --- | --- | --- | --- | --- |
| 1 | The selected repository's `Resolution` is `Unavailable` or `Failed`; or the scheduler's `Unplaced::Unreadable` | Can't open | `circle-alert`, red | Retry |
| 2 | `configuration.state` is `Invalid` or `Missing` | Needs fixing | `triangle-alert`, amber | Settings… |
| 3 | Phase `Attention(KeyLocked(_))`; or no attempt this session and stored `UnlockRequired` | Locked | `lock`, amber | Unlock… (live); Check now (stored, R-5) |
| 4 | Phase `Attention(_)` for any other reason; `Shape::Held`; or no attempt and a stored outcome Part A maps to Needs attention | Needs attention | `circle-alert`, red | D6's attention action for the reason (Choose a key…, Approve host…, Open remote settings…, Resume checking); Check now for a stored one |
| 5 | `checking_now`, or phase `Running` | Checking… | `loader-circle`, teal | — |
| 6 | Last answer `Transport(Unreachable)` or `Transport(Unavailable)`; or no attempt and a stored outcome mapped to Can't reach | Can't reach | `circle-alert`, red | Retry now |
| 7 | Last answer `Completed` with `to_share` not empty | {n} to share | `cloud-upload`, amber | — |
| 8 | `policy.paused` | Paused | `pause`, muted | Check now |
| 9 | Last answer `Completed`, nothing to share | Up to date | `circle-check`, green | — |
| 10 | Polling on, no schedule and not `Unplaced` (no SSH shared copy) | This computer | `hard-drive`, secondary | Settings… |
| 11 | Anything else: reading, not checked yet, checking soon, waiting, off, polling off | no state | — | — |

- The first row that holds wins. Decision 8 moves Checking… above Can't
  reach, so a Retry now shows that it is working.
- A row with no state (11) shows no label; its tooltip still carries D6's
  sentence (R-6; decision 12).
- `RepoState { kind, label, icon, action: Option<RowAction>, sentence }`.
  The label and sentence are constants or formatting functions beside the
  function (Copy). `RowAction` names D6's attention actions by reason;
  today's `Action` and `action_of` sit in the view
  (`D/view/operations/attention.rs:73`), so Task 2 moves what the model
  needs into `D/model/polling/attention.rs` and records where.

**Tooltip and description.** `repository_tooltip(&RepoFacts, &RepoState)
-> Vec<String>`, joined with new lines for the tooltip and with ". " for
the row's accessible description (C2 R-14):

1. The state's sentence:

   | State | Sentence |
   | --- | --- |
   | Can't open | The unavailable code's `message()`, then "Its files are unchanged. Retry, or remove it from Manyhands." |
   | Needs fixing | "Its configuration needs fixing:" and each problem's `guidance()` |
   | Locked | "Sharing from {repo} is waiting for your key's passphrase." |
   | Needs attention | D6's `status::row(…).status`, and its `detail` when present |
   | Checking… | "Checking {repo} for updates…" |
   | Can't reach | `cant_reach(name)`, then "Manyhands tries again later." |
   | {n} to share | "Not shared yet: " and the items' titles (Part C), main as "main"; then "Share each from its tab, or Share main for main." |
   | Paused | "Checking for updates is paused." |
   | Up to date | D6's `checked` line ("Checked {ago} · up to date") |
   | This computer | "No shared copy, so changes stay on this computer." |
   | no state | D6's `status::row(…).status`, or nothing while polling is off |

2. "Folder: {root}".
3. "Main branch: {primary_branch}", when known.
4. "Shared copy: {name} · {location}" from `SharedCopy`, when there is one;
   else "Shared copy: {publication_remote}" when the configuration names
   one the scheduler does not schedule (K-5).

Nothing else: no key, passphrase, fingerprint, or URL that is not the
library's redacted one (Part I).

## Part C: Rows, Titles And List States

- **Rows** use Cycle 02's `ListRow` in the Kit list (C2 R-2), 28 px:
  `folder-git-2` (14 px, `secondary`), the name with ellipsis, and the
  state at the right (icon 12 px and label, 11.5 px, in the state's tone),
  per `03`. The row's label is `{repo}`; its description is
  `repository_tooltip` joined (R-6); its tooltip the same lines. The
  "Selected" and "Unavailable" badges and the path line go; the state
  label has the id `("repo-state", i)` and its text as accessible name.
- **One meaning of selected** (decision 3, D1 polish). `ListRow`'s
  `aria_selected` is the chosen repository, the scope; the Kit cursor is
  drawn as `ListRow::focused`'s ring only. Enter or a click chooses, as
  today (R-7).
- **No notify during render** (decision 3, D1 polish). `Workspace::render`
  still builds the rows, and when they differ from the pane's it applies
  them in `window.defer`, after the frame, instead of calling `set` during
  the render (R-8; U-4).
- **Titles of what is not shared.** A session cache in the Workspace,
  `titles: HashMap<(RepoKey, ItemId), String>`, filled from the Tickets and
  Documents lists whenever they load, and otherwise by one `show_item`
  read per unknown item named by a `to_share` list, under a new
  `ReadKind::ShareTitles` (fact 32's `MadeReady` pattern). A read that
  fails or is busy leaves the item named by its ID (R-9; K-3).
- **List states** of the repositories read, by Cycle 02's helpers, in a
  small body function of the panel's own (fact 11; R-10):

  | List | Shows |
  | --- | --- |
  | First load | `skeleton_rows` labelled "Loading repositories" |
  | Ready, no rows | `empty_state` "No repositories yet." with "Add a repository…" (Part E's Add) |
  | Rows, a reload busy | rows kept, `alert_row` "Repositories were not updated: Manyhands is busy." with Retry |
  | Rows, a reload failed | rows kept, `alert_row` with the failure's message and Retry |
  | No rows, failed | `alert_row` with the failure's message and Retry |

  The Kit list is drawn in every state (C2 R-34). The alert's Retry is
  `load_repositories`, as today's `repositories-retry`.
- The in-body heading goes: the header already names the panel (fact 2).
  The Settings button stays below the list until Cycle 04 removes it
  (Wave Cycle 04; decision 15).

## Part D: The Repository Menu

One Workspace-owned popup, built as C1's dock menu is (fact 14; R-11),
opens for a row:

- by a right click on the row, at the pointer, for that row's repository,
  without changing the choice;
- by `menu` or `shift-f10` in the `Repositories` key context, at the
  highlighted row's bounds (recorded by a `canvas` in each row during
  prepaint, keyed by root), for the highlighted repository, or the chosen
  one when nothing is highlighted (U-1, U-2).

Items, in order, each enabled as stated, come from a pure `menu_items(&RepoFacts, &RepoState) -> Vec<MenuItem>` beside `repository_state`; the More menu's from `more_items(chosen: Option<&RepoKey>)`:

| Item | Does | Disabled when |
| --- | --- | --- |
| The state's action, when it has one not listed below (Retry, Unlock…, Choose a key…, Approve host…, Open remote settings…, Resume checking) | As Activity's and Settings' buttons do today: `retry_resolution`, `Scheduler::unlock`, `attend` | — |
| Check now, or Retry now when the state is Can't reach | `Intent::PollOnce { root }`, user lane, as Settings' Check now | no schedule, `Unplaced`, or `checking_now` |
| Share main, or "Share main…" with no publication remote | `share_main(root, remote)` | `sharing_main(root)` ("Sharing…") |
| Open folder | `open_folder(root)` | — |
| Settings… | `open_settings(SettingsPage::Repositories)` | — |
| separator, Remove from Manyhands… | `remove_repository(root)` | — |

- Every handler is refused behind a dialog or sheet (`unless_dialog`;
  `modal_open`); the layer is dropped under a modal, as the dock menu's.
- Choosing an item closes the menu; Esc closes it and returns focus where
  it was (the list for a key-opened menu).
- **Remove from the panel.** `remove_repository` refuses under a dialog or
  a sheet, and its dialog returns focus to the Repositories list, not to
  the item area (R-12). The dialog is today's, with its "Local files
  remain" note (fact 18; decision 14).

## Part E: Toolbar

`02` puts the toolbar at the right of the panel header; Cycle 02's
mechanism carries three more buttons (fact 13; C2 Part G):

| Id | Icon | Name and tooltip | Does | Disabled when |
| --- | --- | --- | --- | --- |
| `add-repository-button` | `plus` | "Add or create a repository…" | `open_settings(SettingsPage::Repositories)`, today's flow, until Cycle 05's sheet | — |
| `check-all-button` | `refresh-cw` | "Check all for updates now" | `PollOnce` for every repository with a schedule that is not `Unplaced` and not already checking, in list order (R-13) | none qualifies |
| `repositories-more-button` | `ellipsis` | "More" | Opens the More menu below the button | — |

- **More menu**, the same popup mechanism as Part D, for the chosen
  repository: Refresh (`maintain_index(false)`), Rebuild the search index
  (`maintain_index(true)`), separator, Remove from Manyhands…. Each is
  disabled with no repository chosen.
- `ToolbarButton` gains `AddRepository`, `CheckAll`, `RepositoriesMore`,
  all with `region()` Repositories; `ALL` and `toolbar_focus` grow to 7.
  `secondary-n` from a Repositories button does what it does from the
  list today (R-14).

## Part F: The Details Block Leaves The Item Area

Decision 5 and the folded-in vh-d78zh:

- `sidebar::selection` and its two call sites go (fact 5): nothing about
  the repository is drawn in the item area or at the end of an item tab.
- With no tab shown, the item area shows "Select a repository." when none
  is chosen, as today, and otherwise "Open a ticket or document from the
  lists", the empty pane's text from `04` without its icon, which Cycle 05
  adds (R-15; decision 13).
- Where each part of the block goes:

  | Today in the block | Now |
  | --- | --- |
  | Name, root | The row's label; "Folder:" in its tooltip |
  | Opening (`Resolution::Unknown`) | No state while resolving (Part B row 11) |
  | Unavailable code, Retry, Remove | Can't open, its sentence; Retry and Remove from Manyhands… in the menu |
  | Busy resolution, Retry | No state; "Opening was busy" sentence, Retry in the menu (R-16) |
  | Failed resolution, Retry | Can't open with the failure's message |
  | Configuration label, problems | Needs fixing and its sentence; nothing for a valid configuration |
  | Primary branch | "Main branch:" in the tooltip |
  | Index state | The Tickets and Documents lists' states, which already say stale, never refreshed or unavailable (C2 decision 11) |
  | Share main | The menu |

- `REPOSITORIES_HEADING`, `SELECTED`, `UNAVAILABLE`, `SHOWING_LAST_READ`,
  `SHARE_MAIN_BUTTON` and `configuration_label` go once nothing uses them;
  `RETRY`, `SHARING`, `SHARE_MAIN` and `SHARE_MAIN_SET_UP`, which other
  files import, stay where they are (fact 6).

## Part G: Footer Status Items

`02` "Footer" items 3 and 4, between the spacer and Needs attention
(fact 31):

| Control | Id | Shows | Name and tooltip | Activates |
| --- | --- | --- | --- | --- |
| `UpdateCheck` | `footer-update-check` | `refresh-cw` (teal) and "Checking for updates…" while any repository is checking; otherwise "Checked for updates {ago}" from the latest `last_completed` of any scheduled repository; hidden when there is none | The text, then "Background check: fetch and fast-forward only. Never shares your changes." | Settings › Checking for updates (`SettingsPage::Polling`) |
| `ToShare` | `footer-to-share` | `cloud-upload` (amber) and "{n} to share", the sum of every repository's `to_share`; hidden at 0 | "{n} to share. Select {repo}." naming the repository it selects next | Chooses the next repository in list order, after the chosen one, whose `to_share` is not empty (decision 6) |

- `{ago}` is D6's `ago`, made `pub(crate)` (fact 24).
- `plan(layout, &FooterStatus)` replaces `plan(layout, attention)`, with
  `FooterStatus { update: Option<String>, to_share: usize, attention:
  usize }`; the right side's first group is `[UpdateCheck?, ToShare?,
  Attention?]`, empty ones dropped. `Control::ALL` grows to 11.
- Both are focusable, reached by Tab, activated by Enter or Space, and
  refused under a dialog or sheet by `press` (C1 R-29).
- Choosing a repository by `ToShare` keeps focus on the control.

## Part H: Keyboard And Focus

- **Unchanged:** F6 and Shift F6, `secondary-1` (focuses Repositories),
  up and down in the list with Enter choosing, the region ring.
- **New:** `menu` and `shift-f10` open the repository menu in the
  `Repositories` context (`OpenRepositoryMenu`), whose name, today the
  literal `"Repositories"` (`D/view/workspace/focus.rs:100`), becomes the
  constant `REPOSITORIES_CONTEXT` in `D/keys.rs`; up, down, Enter and Esc
  in the menu (the Kit's). The toolbar buttons and the two footer items
  are Tab stops with accessible names.
- **Focus after an action:** choosing a menu item leaves focus on the list
  (Remove's dialog returns it there, R-12); a toolbar button keeps it; the
  More menu returns it to its button.

## Part I: Privacy

Every new text is built from repository names, paths, branch names,
remote names, the library's redacted remote locations, item titles and
fixed copy. No key, passphrase, fingerprint or unredacted URL is read into
`RepoFacts`.

- `polling_privacy` (`D/view/polling_privacy_tests.rs:406`) gains
  surfaces for `repo-` and `footer-` ids, and sweeps
  `repository_tooltip` for every repository and the footer's names
  through their functions, because the snapshot cannot read a tooltip
  (fact 12). Its passphrase and private-key sentinels must not appear.
- A remote whose URL carries a password sentinel: its tooltip line holds
  only the library's redacted location, or, if such a remote is not
  publication eligible, only its name (U-3).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Each row of Part B's table, alone | Its state, label, icon, action and sentence |
| P2 | Several rows of the table hold at once | The first wins, in Part B's order |
| P3 | A stored outcome and no attempt this session; then an attempt | The stored outcome's state; then the attempt's; Settings' line agrees at each step |
| P4 | An attempt with `LocalAhead` and `Unpublished` targets and main ahead | "{n} to share" counts all of them; Settings' "N not shared yet" says the same |
| P5 | A successful `ItemSync` of one of them; a failed one; a successful `RepoSync` | The count drops by one; unchanged; main leaves the count |
| P6 | Titles known from a loaded list; unknown, read; the read fails | Named by title; by title after the read; by ID |
| P7 | The tooltip and description of each state | Part B's lines, and nothing else |
| P8 | The repositories read: loading, ready empty, busy reload, failed reload, failed with no rows | Part C's state, with its action; the rows kept where it says |
| P9 | A click or Enter on a row; arrows | Chooses it; moves the cursor only; `aria_selected` is the chosen repository |
| P10 | The menu by right click and by `shift-f10` and `menu` | Opens for that row; each item does what Part D says and is disabled as it says |
| P11 | Remove from Manyhands… from the menu, then Cancel; then OK | The dialog with its note; focus back on the list; the repository removed and its row gone |
| P12 | Any menu item, toolbar button or footer item behind a dialog or sheet | Refused; the menu closes under a modal |
| P13 | The toolbar by Tab and Enter | Add opens Settings › Repositories; Check all sends one `PollOnce` per qualifying repository; More opens its menu; disabled as Part E says; the region holds focus |
| P14 | No tab shown | "Select a repository." or "Open a ticket or document from the lists"; nothing about the repository in the item area or at the end of a tab |
| P15 | The footer with checks done, one running, none; with work to share in two repositories | "Checked for updates {ago}", "Checking for updates…", hidden; "{n} to share", and each activation chooses the next repository with work |
| P16 | Rows change while nothing else redraws | The pane is updated after the frame; nothing notifies during render |
| P17 | Every text P1–P15 shows, with passphrase, private-key and URL-password sentinels in play | No sentinel |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.

- Between tasks, only the tests a task wrote or changed run, by exact name
  (AGENTS.md, Testing Strategy; decision 18).
- Behaviour, not appearance: no tests for colours, icons, tones, sizes or
  that a view renders. Icons and tones in Part B's table are not tested.
- Decision logic lives in pure functions with one table-driven test each;
  window tests are few, each a journey that also checks what is sent.
- Tooltip and description text is tested through its function (fact 12).
- Stored outcomes come from `PollingBackend::store_outcome` (Part A);
  attempts, phases and attention from `schedule_mut()` or the existing
  `polling_support::Rig` and `PollingBackend` answers; window tests that
  need the scheduler opt in with `with_polling()` (fact 26). No SQL on the
  registry is needed. No real waits; `now` is passed in.

| Test | What |
| --- | --- |
| `desktop::model::repository_state::tests::each_state_and_its_precedence` | One table over Part B's rows, alone and in pairs, with stored outcomes and live attempts (P1–P3) |
| `desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts` | One table of `repository_tooltip` per state, with and without branch, shared copy and titles (P6, P7) |
| `desktop::model::polling::tests::schedule::to_share_counts_and_finished_shares` | `AttemptSummary::of` with `LocalAhead`, `Unpublished` and main; `Heard::Shared` success, failure and `RepoSync` (P4, P5) |
| `desktop::model::polling::status::tests::status_text_table` (E-9) | Gains the stored-outcome rows of Part A (P3) |
| `desktop::services::scheduler::entity_tests::the_stored_outcome_and_shared_copy_are_kept` | With `store_outcome` and a fixture remote: kept on `registered`, replaced on a later read, dropped with the repository (P3) |
| `desktop::services::scheduler::reconfigure::tests::finished_shares_are_heard` | A finished `ItemSync` and `RepoSync`, success and failure, give `Heard::Shared` or nothing (P5) |
| `desktop::model::repository_state::tests::menu_items_follow_the_state` | One table of `menu_items` and `more_items`: order, labels and enablement per state (P10, P13) |
| `desktop::view::repositories::tests::rows_states_and_titles` | A window journey with polling on: rows with states from the schedule; a finished share drops the count; titles from a loaded list and from a `ShareTitles` read; list states busy and failed with Retry (P4–P6, P8, P9, P16) |
| `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard` | Right click and `shift-f10`; each item's request or effect; disabled items; Remove with Cancel and OK and focus; refused under a dialog (P10–P12) |
| `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter` (E-11) | Gains the three Repositories buttons (P12, P13) |
| `desktop::view::footer::tests::status_items_say_and_choose` | `plan` with `FooterStatus`; the two items' texts and names by function; Enter on To share cycles the choice; Update check opens Settings › Checking for updates (P15) |
| `desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository` | No tab: the two texts; a tab: no repository details at its end (P14) |
| `desktop::view::polling_privacy_tests::polling_privacy` (E-15) | Gains the `repo-` and `footer-` surfaces and the tooltip sweep (P17) |

Pre-existing tests change as the named edits say.

## Named Edits To Pre-Existing Tests

At `7034522f`. Task 0 re-counts with `git grep`, over tests and
production code, and records any other site as a further edit (decision
17). Edits that remove assertions are marked **Removes**.

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `D/view/workspace_tests.rs`: `selecting_a_repository_resolves_it_and_shows_its_configuration_and_index` (:709), `a_renamed_away_repository_is_unavailable_until_retry_finds_it_again` (:753), `a_registration_removed_by_a_second_actor_shows_unavailable` (:793), `a_list_reload_after_a_second_actor_removes_the_selection_shows_unavailable` (:819), `a_held_resolution_of_a_never_replaces_b` (:939), `reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor` (:1098) | Its row part goes with the rows (Part C): `("repo-unavailable", i)` (:776, :785) becomes `("repo-state", i)` reading "Can't open". Its details part goes with Part F: `selection-name` becomes the chosen row's `selected()`; `selection-unavailable*` becomes `("repo-state", i)` reading "Can't open"; `selection-retry` becomes the menu's Retry; `selection-configuration` becomes the row's state (Needs fixing, or none) with the configuration checked through `repository_tooltip`. **Removes** the "Configuration valid" and index-state assertions (the index state is the lists', fact 11) | C, F |
| E-2 | `D/view/workspace_tests.rs`: `a_busy_list_reload_keeps_the_last_rows_with_a_label` (:900); `D/view/settings/repositories_manage_tests.rs` `tabs_close_once_the_list_no_longer_gives_their_root` (:647) | `repositories-state`, `repositories-last-read` and `repositories-retry` become the busy alert row and its action `(id, "action")`; the row labels are unchanged. **Removes** "Showing the last list read" | C |
| E-3 | `D/view/workspace_tests.rs`: `the_fixtures_repositories_are_listed_by_name` (:696), `the_repository_selector_selects_with_arrows_and_enter` (:1039), `a_repository_rows_selected_state_has_one_meaning` (:1142); `D/view/keyboard_tests.rs` helper `select_a_repository` (:262) | `("repo-root", 0)` goes (the folder is in the tooltip, P7); `("repo-selected", i)` becomes `selected() == Some(true)`; the one-meaning test asserts `selected()` is the chosen repository and the cursor is not. **Removes** the root-path label | C |
| E-4 | `D/view/tabs_tests.rs` `opening_a_tab_does_not_change_the_selection` (:190) | Reads the chosen row's `selected()` instead of `selection-name` | F |
| E-5 | `D/view/keyboard_tests/actions.rs`: helper `unavailable` (:173), `retry_an_unavailable_repository_by_keyboard` (:187), `remove_an_unavailable_repository_by_keyboard` (:203) | The steps go to the Repositories region and open the menu with `shift-f10`; Retry and Remove from Manyhands… are chosen there | D, F |
| E-6 | `D/view/settings/repositories_manage_tests.rs` (path `desktop::view::settings::repositories::manage_tests::…`): `removing_a_registration_whose_folder_was_renamed_away_succeeds` (:533), `the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go` (:731) | Remove is reached through the menu; the focus after the dialog is the list | D, F |
| E-7 | `D/view/item/share_tests.rs`: `share_main_in_the_repository_detail_submits_repo_sync` (:232), `share_main_without_a_remote_reads_share_main_and_opens_remotes` (:280), `an_accepted_share_offers_stop_and_stopping_signals_it` (:537), `a_stopped_share_says_what_completed_and_offers_share` (:599), `share_opens_remotes_on_its_own_repository_after_another` (:789); `share_main_at_the_end_of_an_item_tab_submits_repo_sync` (:299) | Share main is chosen from the menu, its label read from the menu item. `share_main_at_the_end_of_an_item_tab_…` is deleted: no tab carries the details. **Removes** | D, F |
| E-8 | `D/view/keyboard_tests/discussion.rs` `tab_order_runs_header_share_chips_page_discussion_composer` (:297) and its `SHARE_MAIN` constant (:34) | The Tab order ends at the composer; the steps to and from `selection-share-main` go. **Removes** | F |
| E-9 | `D/model/polling_schedule_tests.rs` `attempt_summary_counts` (:10; path `desktop::model::polling::tests::schedule::…`), which now expects 2, its fixture having one `LocalAhead` and one `Unpublished`; `D/model/polling/status_tests.rs` `status_text_table` (:73) and every `status::Facts` literal (not `retire_loop::Facts`); `D/view/settings/polling_tests.rs` `settings_polling_rows` (:43; path `desktop::view::settings::polling_tests::…`) | `not_shared` is a method returning `to_share.len()`, so it counts `Unpublished` too; `status::Facts` literals gain `stored: None`; the table gains Part A's stored rows; texts with "not shared yet" follow the new count. `PillState::EditingSaved { not_shared }` (`D/view/item/pill.rs`) is unrelated and unchanged | A |
| E-10 | `D/view/footer_tests/mod.rs`: `a_secondary_click_opens_the_dock_menu_with_its_side_checked` (:191-199), `toggle_names_follow_their_state` (:445-464); `D/view/keyboard_tests/shell.rs`: `each_side_is_reached_by_the_dock_menu_keys` (:353-356), `footer_controls_are_reached_by_tab` (:409-415); `keyboard_tests/shell/support.rs` `said` (:93) | `plan` takes `FooterStatus`; `said` gains the two controls; the Tab walk compares with the controls the plan shows, so the two status items, hidden without a schedule, are left out there and walked in `status_items_say_and_choose` | G |
| E-11 | `D/view/dock_tests/toolbars.rs` `panel_toolbars_work_by_tab_and_enter` and `with_no_repository` | The three Repositories buttons join the loop; `with_no_repository` names them with their enablement (Add enabled) | E |
| E-12 | `D/assets_tests.rs` `every_item_view_icon_loads` and its fixed-length `ICONS` array | Each icon Parts B, E and G use is checked first; one that is neither in `ItemIcons` nor a Kit built-in is added to both, and the array's length follows (`hard-drive`, `pause`, `lock`, `loader-circle`, `refresh-cw`, `ellipsis`, `plus`, `triangle-alert` are not there at `7034522f`) | B, E, G |
| E-13 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | Gains `menu` and `shift-f10` → `OpenRepositoryMenu` in `Repositories` | H |
| E-14 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour and stops for the owner. **Removes** | — |
| E-15 | `D/view/polling_privacy_tests.rs` `polling_privacy` | Gains Part I's surfaces and sweep | I |

## Document Amendments

To apply in the approval commit:

1. **Wave Cycle 03.** Split into "Cycle 03a: Repositories Panel And
   Sharing Status" (this Cycle: toolbar, rows, states, context menu,
   footer "to share" and "Checked for updates", the repository details out
   of the item area) and "Cycle 03b: Activity And Needs Attention"
   (Activity, the popover, the three K-19 rows, whether an unreachable
   repository counts); the Cycles table and planned document paths with
   them. Remove "when D6 provides it" and "once D6 has merged" (D6 is
   merged) and "proposed" before K-19 (accepted 2026-11-09). Exit
   evidence for 03a: each state from real service state; every action by
   keyboard; no secret in a row, tooltip or footer item.
2. **Handoff `03`, Repositories.** The state table gains Can't open, Needs
   fixing, Locked and Needs attention (decision 4) and puts Checking…
   above Can't reach (decision 8); "N to share" counts items not shared
   yet, not checkpoint commits (decision 2); the context menu leads with
   the state's action and adds More's Refresh and Rebuild (Part D, E).
3. **Handoff `08`.** The repository context menu is a Workspace popup
   like the dock menu, not the Kit `ContextMenu` (R-11).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | The stored `latest_outcome` lives in a scheduler side map, not in `Policy`; it informs only a repository with no attempt this session | A stored outcome older than a later success by another process is shown until this process checks (K-2) |
| R-2 | The shared copy's name and location come from the `publication_remotes` read the scheduler already makes | — |
| R-3 | A successful `ItemSync` or `RepoSync` removes its target from `last.to_share` at once; nothing else changes the attempt | A share by another process or the CLI shows only after the next check (K-1) |
| R-4 | The stored-outcome seam is a desktop test double, not SQL on the registry | — |
| R-5 | A stored `UnlockRequired` offers Check now, not Unlock…: the stored row names no key, and a user-lane check asks for the passphrase (decision 9; U-6) | One more step before unlocking after a relaunch (K-4) |
| R-6 | A row with no known state shows no label; its tooltip and description still carry D6's sentence | — |
| R-7 | `aria_selected` is the chosen repository; the Kit cursor is the focus ring only | A screen reader hears the cursor through focus, not selection |
| R-8 | Rows are built at render and applied in `window.defer` when they changed (U-4) | Rows lag one frame |
| R-9 | Titles: from loaded lists, else one `show_item` read per unknown item, cached for the session; IDs when a read fails | A renamed item keeps its old title until the lists reload |
| R-10 | The repositories list's states are a small body function of the panel's own, using `rows.rs`'s helpers; `list_body` is not widened | — |
| R-11 | The menu is a Workspace-owned `PopupMenu` layer like the dock menu, opened by right click and by `menu` and `shift-f10`; not the Kit `ContextMenu`, which has no keyboard path (facts 14, 15) | — |
| R-12 | `remove_repository` refuses under a dialog or sheet and returns focus to the Repositories list | — |
| R-13 | Check all sends one `PollOnce` per scheduled, placed repository not already checking, paused ones included (decision 16) | A person who paused a repository sees it checked once |
| R-14 | `secondary-n` from a Repositories toolbar button does what it does from the list | — |
| R-15 | The item area's empty text is `04`'s, without its icon, until Cycle 05 | — |
| R-16 | A busy resolution shows no state and offers Retry in the menu | — |
| R-17 | The row follows the scheduler, not the configuration, for whether there is a shared copy; Share main follows the configuration, as today | A non-SSH publication remote reads "This computer" while Share main is offered (K-5) |

## Known Limits

- **K-1.** "{n} to share" is known only after a check this session, and
  follows this app's shares at once but another process's only at the
  next check.
- **K-2.** A stored outcome is shown until this process checks, even if
  another process has since checked successfully.
- **K-3.** Items not shared yet are named by ID when their title cannot be
  read.
- **K-4.** After a relaunch a locked key is offered Check now, which asks
  for the passphrase, rather than Unlock….
- **K-5.** A publication remote that is not SSH reads "This computer"; its
  name is in the tooltip; Share main is still offered (R-17).
- **K-6.** Tooltips show on hover only; their text is also the row's
  accessible description (C2 K-1).
- **K-7.** "Checked for updates {ago}" is worked out when the footer is
  drawn.
- **K-8.** The panel's Settings button stays until Cycle 04.

## Unverified

- **U-1.** `menu` and `shift-f10` pressed in the Kit list reach a binding
  in the enclosing `Repositories` context (the Kit `List` binds neither).
  Task 4 checks it first; if not, the binding moves to the `List` context
  with a `Repositories` parent.
- **U-2.** A right button press on a Kit list row reaches the row's
  `capture_any_mouse_down`. Task 4 checks it first.
- **U-3.** Whether a remote whose SSH URL carries a password is
  publication eligible. Task 8 settles which branch of Part I's test runs.
- **U-4.** `window.defer` from `Workspace::render` applies the rows after
  the frame without a second render loop. Task 3 checks it first; if
  not, rows are pushed from the scheduler observer and the navigation
  updates instead.
- **U-5.** `keyboard_tests.rs`'s `TAB_LIMIT` (40) still reaches Settings
  with two more footer stops. Task 7 checks it.
- **U-6.** A user-lane `PollOnce` of a repository whose key is locked asks
  for the passphrase, as the `PollingBackend::ask_in_check_now` seam
  suggests (`D/test_support/polling.rs:251`) and
  `unlock_after_cancelled_prompt` shows through the worker's provider
  (`D/services/scheduler_attention_tests.rs:227`). Task 1 checks it; if not,
  R-5's Check now becomes Settings › Sharing & keys and decision 9 is
  raised with the owner.

## Alternatives Rejected

- **A library read of unshared checkpoints.** Counts commits without a
  check, but needs a library ticket first (owner, decision 2).
- **Share all… from the row.** Clears the count in one go, but is new
  batch behaviour (owner, decision 7).
- **A Details… sheet.** Keeps today's block whole, but is a sheet the
  design does not have (owner, decision 5).
- **The Kit `ContextMenu`.** Opens by right click only (fact 15).
- **`Policy.stored`.** Breaks six `Policy` literals for a value the
  schedule does not decide by (R-1).

## Risks

| Risk | Control |
| --- | --- |
| The state table and D6's Settings line drift apart | Both read `stored` and `to_share` from one model; P3 and P4 check both |
| Removing the details block loses a recovery route | Part F's table maps every part; E-1 to E-8 keep each journey through the menu |
| The footer's new stops break the Tab walks | E-10; U-5 |
| Rows and the menu read Workspace state while panels are built inside `workspace.update` | Rows are computed by the Workspace and pushed to the pane (R-8); the menu is a Workspace layer |
| D1, D4 and D5 follow-up tickets edit the same files | They have no commits on their branches; the D1 items this Cycle absorbs are noted on that ticket at closure |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | Split, "to share" source, Activity's shape, fold-ins | Owner decisions | Decisions 1–3 | Session, 2026-11-12 |
| Owner, 2026-11-12 | Row labels, the details block, the footer, what "to share" offers | Owner decisions | Decisions 4–7 | Session, 2026-11-12 |
| Planning research | `repo sync` shares main only; no library count of unshared checkpoints; the stored outcome dropped; a share changes nothing in the schedule; tooltips unreadable by tests; the details block drawn at the end of every tab | Facts, rulings, decisions (proposed) | Facts 5, 17, 27–29, 12; Parts A, F; decisions 8–16 | This document |
