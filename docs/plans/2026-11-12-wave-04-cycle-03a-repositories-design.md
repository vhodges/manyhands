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

**Draft, revision 2.** Grounded at the ticket branch `7034522f`, which
holds origin/main `37be101d` (Cycle 01, Cycle 02 and Wave 03 D6 merged).
Revision 1 (`f6d57bd4`) had two reviews in round 1 (source and
feasibility; contract and consistency). Revision 2 applies their findings
and the owner's second-round answers (decisions 19–22). Not yet
re-reviewed.

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
  Check now (D6). "The schedule" is D6's `PollSchedule`. "No attempt this
  session" means exactly `last.is_none()` (fact 33).
- "The kept row" is the chosen repository the list no longer gives
  (decision 22; fact 35).

## Baseline Facts At `7034522f`

**The panel today.**

1. **No panel type.** The Repositories panel is the generic `RegionPanel`
   with `Slot::Panel(PanelKind::Repositories)` (`D/view/dock/panels.rs:58`,
   `:67`); its body is `Workspace::repositories_region`, which calls
   `sidebar::repositories(&self.navigation, &self.repositories_pane, cx)`
   (`D/view/workspace/render.rs:68-73`). The header count is the listed
   row count (`D/view/dock/panels.rs:171-177`).
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
   the selected repository's name (`"selection-name"`), root, and by
   `Resolution`: "Opening", the unavailable code's message with Retry and
   Remove, busy with Retry, a failure with Retry, or the configuration
   label, primary branch, problems' guidance, index state and Share main
   (`"selection-share-main"`, `ShareMain`, `:534`). It is the selected
   repository's, not an item's, and it is drawn in the item area when no
   tab is shown (`render.rs:121-130`; `D/view/workspace/content.rs:671-675`)
   and at the end of every item tab's scroll region (`content.rs:623-629`,
   `repository_details`, attached by `open_item` `:310` and `follow_tab`
   `:330`).
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
    stops mouse-down (`:77-140`); `toolbar_enabled(&self, button)` takes
    no `cx` (`:143`); `press_toolbar` returns when `modal_open` or
    disabled (`:156`). `focused_region` counts a focused toolbar handle as
    its region (`focus.rs:182-192`); `focus_region` for Repositories
    focuses the Kit list only when it has rows (`:204`).
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
    action (`footer_tests/mod.rs:142-233`).
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
    (`D/model/stamp.rs:19`), `Polling` being "Checking for updates" and
    `Keys` "Shared SSH keys".
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
    `LocalAhead` only; `Unpublished`, `Diverged` and every other
    disposition but `Current`, `FastForwarded`, `Materialized`, `LocalAhead`
    and a retirable `Closed` or `Promoted` go to `listed` (`:196-208`).
    `AttemptTarget { item_id, kind, disposition }` (`:133`) carries no
    title; main is a target with no item.
24. `status::row(&Facts) -> Row { status, detail, pending, shape }`
    (`D/model/polling/status.rs:250`) with `Facts { name, schedule,
    unplaced, holding, checking_now, now }` (`:80`) and `Shape {
    NoSharedCopy, Pending, Off, Held, Polling(Controls) }` (`:96`);
    `Controls { paused, interval_seconds, retry, checking }` (`:111`).
    `checked` (private) writes "Checked {ago} · up to date" and "· N not
    shared yet" (`:154`); `after_last` maps an `Unreachable` last answer
    to `cant_reach(name)` "Can't reach {repo}. Your work is safe." (`:63`,
    `:174`); `since_library` writes "Checked {ago}" or "Not checked yet"
    when this process has seen no attempt (`:197`); `phase_status`
    (`:215`) and `after_last` take the schedule, not `Facts`. `ago` is
    private (`:134`).
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
    BTreeMap<Root, Option<Authority>>` (`entity.rs:110`, `:360`). The
    module `registration` is private to the scheduler.
28. `PollingStatusDto.latest_outcome: Option<PollingOutcome>`
    (`L/repository/read/dto.rs:715`), one of `Completed,
    ConfigurationRequired, SelectedKeyUnavailable, UnlockRequired,
    HostApprovalRequired, TransportUnavailable, ProtocolRejected,
    Cancelled, RepositoryUnavailable` (`:689`), with no time stored. The
    desktop maps replayed rows by category (`D/services/poll_convert.rs:390`)
    but nothing maps the DTO's `latest_outcome`.
29. `heard(event, operations) -> Option<Heard>`
    (`D/services/scheduler/reconfigure.rs:280`) turns worker events into
    scheduler events, before the request model applies them, so the
    request's intent is still readable; a finished `ItemSync` or
    `RepoSync` is none of them, so a share changes nothing in the
    schedule.
30. Check now: `Intent::PollOnce { root }` on `Lane::User`, sent by
    Settings' `check_now` (`D/view/settings/polling.rs:225`) unless a
    dialog is open or one is in flight; `checking_now` is the worker's
    in-flight `PollOnce` for the root (`:199`). Settings offers Check now
    only for `Shape::Polling`. The test double `PollingBackend` passes
    `polling_status` through to the library (`D/test_support/polling.rs:404`).

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

**Found in review round 1.**

33. **`last` is not restored at launch.** `PollSchedule::add` builds
    `RepoSchedule::new(policy, last_completed)` with `last: None`
    (`D/model/polling.rs:80-86`; `D/model/polling/state.rs:127-131`). The
    comment "restored from `remote_snapshot` at launch" refers to
    `last_completed`, which `registration::read` takes from
    `latest_observed_at` (`registration.rs:119`).
34. **The Kit list owns "selected".** The wrapper `list-item-{ix}` has
    `Role::ListItem` and `aria_selected` of the Kit cursor, and calls
    `item.selected(cursor)` on the delegate's row after `render_item`
    (`C/list/list.rs:489-502`), so a row cannot carry another meaning of
    selected through `Selectable`. `ListRow` draws its ring only when
    `selected && focused` (`D/view/rows.rs:201`) and has only `on_click`.
    Today's repository row carries no `aria_selected`, and
    `a_repository_rows_selected_state_has_one_meaning`
    (`D/view/workspace_tests.rs:1142`) pins that: decision 3's D1 item is
    already met at `7034522f`.
35. **The kept selection.** `Navigation.kept` is the selected entry when
    the last list read did not give it (`D/model/navigation.rs:155-158`),
    carried through reloads (`:289-330`) until another is selected
    (`:349-350`); today only the details block shows it.
36. **`Phase::Running` is never entered in production.** `mark_running` is
    not called: "`Queued` stands for queued and running alike"
    (`D/model/polling.rs:263-271`). `RepoSchedule::in_flight()` covers both
    (`state.rs:142-144`); Settings says Checking… for `Queued | Running`
    (`status.rs:218`).
37. **`TransportFailure::Unavailable` is a local fault** (registry
    unavailable or runtime uninitialized, `attempt.rs:46-47`); D6 says
    "Can't check" for it and for `ConfigurationRequired`
    (`status.rs:184-185`).
38. `open_settings` takes its selection from `navigation.selected()`
    (`content.rs:188-197`); `open_remotes` sets
    `set_selection(Some(root))` (`requests.rs:285-298`). Both move focus
    to the item area.
39. **A failed registration read waits for the window.** `Scheduler::listed`
    reads only newly added roots (`entity.rs:269-273`), and the tick skips
    `failed_reads` (`:413-418`); `read_registrations` is `pub(super)`
    (`:312`).
40. **The drain's "item shared".** A finished `ItemSync`, `ItemRepublish`,
    `TicketClose` or `DocumentPromote` whose `effects.publication` is
    `Published` or `Current` posts `ItemShared` (`D/services/mod.rs:636-652`).
    A `discovery_pending` answer is `Outcome::Partial`
    (`L/results.rs:667`, `:693`), so a share can publish and not be a
    success.
41. **Toolbar context.** The toolbar wrapper sets `NAVIGATION_CONTEXT` for
    every panel (`toolbar.rs:134`); `NewItem` falls back to
    `navigation.pane()` when the region has none (`render.rs:232-238`;
    `focus.rs:69-75`).
42. **Poll doubles.** `PollingBackend::poll_automatic` calls the library
    when nothing is queued (`D/test_support/polling.rs:446-458`); a
    user-lane `PollOnce` runs the real job unless `answer_job` or
    `hold_job` is set for it (`D/test_support/delaying.rs:149`,
    `:186-198`). The fixture's SSH remote is
    `ssh://git@example.invalid/team/repo.git`
    (`D/test_support/fixtures.rs:975`, `:983`).
43. **Menu items in tests.** `MenuItemElement` sets no disabled flag
    (`C/menu/menu_item.rs:95-133`), so a snapshot cannot read an item's
    enablement; separators take an index (`C/menu/popup_menu.rs:1225-1254`).
44. **The privacy sweep.** `polling_privacy` collects but does not assert
    text inside its `ALLOWED` ids (`[&str; 12]`,
    `D/view/polling_privacy_tests.rs:127`), which include
    `region-repositories` (`:136`); it plants its `TITLE` sentinel in every
    item title (`:9-10`, `:92`); `SURFACES` is `[(&str, &str, &str); 15]`
    (`:144`).
45. **Reading titles outside the selection.** D5's made-ready titles are
    read by one `read_job` that resolves the repository and then calls
    `show_item` (`D/view/lifecycle/made_ready.rs:133-171`). `ViewScope`
    (`D/model/stamp.rs:32`) has no variant for this.
46. **Attention actions.** `Action` and `action_of` are in the model
    (`D/model/polling/attention.rs:20`, `:46`); the view's
    `attention_action` (`D/view/operations/attention.rs:73`) disables
    Unlock… when `unlock_target` is `None` (`:81-92`); `attend` is private
    (`:120`).
47. `WorkerHandle::in_flight(pick)` (`D/services/worker.rs:744`) is
    reached through `services.worker()`.
48. Settings' own Remove is `submit_remove`
    (`D/view/settings/repositories/manage.rs:362`), a separate path from
    `remove_repository`.

## Part A: Sharing Facts The Scheduler Keeps

Rows read D6's model, never their own guess (D6 W-5). These additions let
the model say what the panel needs.

- **The stored outcome.** `registration::read` already holds
  `PollingStatusDto`; the scheduler keeps its `latest_outcome` in a side
  map beside `authorities`, `stored: BTreeMap<Root, PollingOutcome>`,
  filled wherever the status is read (`registered`, `answered`), and
  dropped with the repository (R-1). `Policy` is not changed, so its six
  literals stay as they are. `status::Facts` gains `stored:
  Option<PollingOutcome>`; `phase_status` and `after_last` take `&Facts`
  so `since_library` can read it. A stored outcome counts only when there
  is no attempt this session (`last.is_none()`, fact 33) and the
  repository is not checking (`checking_now` or `in_flight()`), so it
  never outranks Checking… (decision 9). Settings and the panel then say
  the same (decision 10):

  | Stored outcome, no attempt this session, not checking | Settings line | Panel state |
  | --- | --- | --- |
  | `TransportUnavailable`, `ProtocolRejected` | `cant_reach(name)` | Can't reach |
  | `UnlockRequired` | "Needs unlocking" | Locked |
  | `SelectedKeyUnavailable` | "Choose a key" | Needs attention |
  | `HostApprovalRequired` | "Needs host approval" | Needs attention |
  | `ConfigurationRequired`, `RepositoryUnavailable` | "Can't check" | Needs attention |
  | `Completed`, `Cancelled`, none | today's text | no state from it |

- **The shared copy.** `registered` also keeps the selected publication
  remote's name and redacted push location,
  `shared_copies: BTreeMap<Root, SharedCopy { name, location }>`, from the
  same `publication_remotes` read (the library's `list_remotes_redacted`,
  fact 27). No new read (R-2). `SharedCopy` is re-exported from the
  scheduler for the model (fact 27).
- **What is not shared yet.** `AttemptSummary` gains `to_share:
  Vec<AttemptTarget>`: every `LocalAhead`, `Unpublished` and `Diverged`
  target of the attempt, main included (decision 19). Main counts as one
  item (R-18). It also gains `problems: Vec<AttemptTarget>`: every
  `Dirty`, `Conflicted`, `Inaccessible`, `Nonconforming`,
  `BlockedByLocalState`, `MaterializationIncomplete`, `Malformed`,
  `RemotelyDeleted`, `HistoryUnknown` and `NotReached` target (decision
  19). The `not_shared` field becomes a method returning
  `to_share.len()`, so Settings' "· N not shared yet" counts the same
  items as the panel (decision 10). Every target that is in `listed` today
  stays there, so Activity's attempt entry is unchanged.
- **A finished share.** `heard` gains `Heard::Shared { root, item:
  Option<ItemId> }` for a finished `ItemSync`, `ItemRepublish`,
  `TicketClose` or `DocumentPromote` (its item), or `RepoSync` (main,
  `None`), whose `effects.publication` is `Published` or `Current`: the
  drain's own "item shared" (fact 40). The scheduler removes the matching
  target from `last.to_share` (R-3; decision 11). An answer that did not
  publish removes nothing.
- **Reading one repository again.** `Scheduler::reread(&mut self, root:
  Root, cx)` drops `root` from `failed_reads` and reads its registration
  again through `read_registrations` (fact 39), for Retry on a row the
  scheduler could not read (R-19).
- **Seam.** `PollingBackend` gains `store_outcome(root, outcome)`, which
  its `polling_status` applies to the library's answer, so a test sets a
  stored outcome without SQL (R-4).

## Part B: Repository State

One pure function decides each row's state, in the order the owner agreed
(decisions 4, 19, 20), with Checking… moved above Locked, Needs attention
and Can't reach for feedback (decision 8):

`repository_state(&RepoFacts) -> RepoState`, in `D/model/repository_state.rs`.

`RepoFacts<'a>` holds what the Workspace already has: the `RepositoryEntry`
(summary and resolution) and whether it is the kept row, the scheduler's
`RepoSchedule`, `Unplaced`, `Holding`, stored outcome and `SharedCopy` for
the root, `checking_now`, `sharing_main`, whether polling is on, the
titles known for the repository's items (Part C), and `now`.

| # | When | State | Icon, tone | Action |
| --- | --- | --- | --- | --- |
| 1 | The kept row; the chosen repository's `Resolution` is `Unavailable` or `Failed`; or `Unplaced::Unreadable` | Can't open | `circle-alert`, red | Retry: `retry_resolution` for the kept row and a resolution; `Scheduler::reread(root)` for `Unreadable` |
| 2 | The chosen repository's `Resolution` is `Busy` | no label | — | Retry (`retry_resolution`) |
| 3 | `configuration.state` is `Invalid` or `Missing` | Needs fixing | `triangle-alert`, amber | Settings… (Settings › Repositories for this repository) |
| 4 | `checking_now`, or `in_flight()` (fact 36) | Checking… | `loader-circle`, teal | — |
| 5 | Phase `Attention(KeyLocked(_))`; or stored `UnlockRequired` (Part A's condition) | Locked | `lock`, amber | Unlock… (live; disabled when `unlock_target` is `None`, fact 46); Check now (stored, R-5) |
| 6 | Phase `Attention(_)` for any other reason; `Shape::Held`; a stored outcome Part A maps here; last answer `Transport(Unavailable)` or `Transport(ConfigurationRequired)`; or last answer `Completed` with `problems` not empty | Needs attention | `circle-alert`, red | D6's `action_of(reason)` (Choose a key…, Approve host…, Open remote settings…, Resume checking); Check now for a stored one; Retry now for "Can't check"; none for problems |
| 7 | Last answer `Transport(Unreachable)`; or stored `TransportUnavailable` or `ProtocolRejected` | Can't reach | `circle-alert`, red | Retry now |
| 8 | Last answer `Completed` with `to_share` not empty | {n} to share | `cloud-upload`, amber | — |
| 9 | `policy.paused` | Paused | `pause`, muted | Check now |
| 10 | Last answer `Completed`, nothing to share and no problems | Up to date | `circle-check`, green | — |
| 11 | Polling on, no schedule, not `Unplaced`, and the configuration names no publication remote | This computer | `hard-drive`, secondary | Set up sharing… (Settings › Remotes for this repository) |
| 12 | Polling on, no schedule, not `Unplaced`, and the configuration names a publication remote (not SSH, decision 20) | no label | — | Set up sharing… |
| 13 | Anything else: reading, not checked yet, checking soon, waiting, off, polling off | no label | — | — |

- The first row that holds wins. Decision 8 moves Checking… above Locked,
  Needs attention and Can't reach, so an Unlock…, Retry now or Check now
  shows that it is working, as D6's `status::row` puts `checking_now`
  before the phase (fact 24); so neither a stored outcome (decision 9) nor
  a live attention outranks Checking….
- Rows 2, 12 and 13 have no label; their sentence is still in the tooltip,
  the description and the menu's first line (R-6; decision 12). Every
  state with a tone has a label and an icon, so colour is never the only
  signal.
- `RepoState { kind, label, icon, tone, action: Option<RowAction>,
  sentence: Vec<String> }`. The label and sentence are constants or
  formatting functions beside the function (Copy). `RowAction` names
  D6's attention actions through `Action` and `action_of`, which are
  already in the model (fact 46).

**Sentences.** The sentence is the first part of the tooltip and
description, and the menu's first line (decision 21):

| State | Sentence |
| --- | --- |
| Can't open, the kept row | "{repo} is no longer registered with Manyhands. Its files are unchanged. Retry, or remove it from Manyhands." |
| Can't open, `Unavailable(code)` | The code's `message()`, then "Its files are unchanged. Retry, or remove it from Manyhands." |
| Can't open, `Failed(failure)` | The failure code's `message()`, then the same |
| Can't open, `Unreadable` | "Manyhands couldn't read {repo}'s settings for sharing. Its files are unchanged. Retry, or remove it from Manyhands." |
| Busy resolution | "Manyhands was busy opening {repo}. Its files are unchanged. Retry." |
| Needs fixing | "Its configuration needs fixing:" and each problem's `guidance()` |
| Locked | "Sharing from {repo} is waiting for your key's passphrase." |
| Needs attention, a reason, Held, a stored outcome or Can't check | D6's `status::row(…).status`, and its `detail` when present |
| Needs attention, problems | "{n} items need a look. Activity lists them." ("1 item needs a look. Activity lists them.") |
| Checking… | "Checking {repo} for updates…" |
| Can't reach | D6's `status::row(…).status` (`cant_reach(name)`, which says "Your work is safe.") |
| {n} to share | "Not shared yet: " and the titles (Part C), main as "main", joined with ", " and ending "."; then "Share each from its tab or from Activity." when an item is listed; then "Share main shares main." when main is listed |
| Paused | "Checking for updates is paused." |
| Up to date | D6's `status::row(…).status` ("Checked {ago} · up to date") |
| This computer | "No shared copy, so changes stay on this computer." |
| Not SSH (row 12) | "{remote} isn't an SSH shared copy, so Manyhands can't check or share it." |
| Row 13 | D6's `status::row(…).status`, or nothing while polling is off |

**Tooltip and description.** `repository_tooltip(&RepoFacts, &RepoState)
-> Vec<String>`, joined with new lines for the tooltip and with " " for
the row's accessible description (C2 R-14):

1. The state's sentence.
2. "Folder: {root}".
3. "Main branch: {primary_branch}", when known.
4. "Shared copy: {name} · {location}" from `SharedCopy`, when there is one.

Nothing else: no key, passphrase, fingerprint, or URL that is not the
library's redacted one (Part I).

## Part C: Rows, Titles And List States

- **Rows are the panel's own element**, drawn to `03`'s anatomy, not
  `ListRow`: the Kit overwrites `selected` with its cursor (fact 34). Each
  row, `("repo-row", i)`, 28 px: `folder-git-2` (14 px, `secondary`), the
  name with ellipsis, and the state at the right (icon 12 px and label,
  11.5 px, in the state's tone) with id `("repo-state", i)` and its label
  as accessible name. The row's label is `{repo}`; its description and
  tooltip are `repository_tooltip`'s lines. The "Unavailable" badge and
  the path line go.
- **One meaning of selected** (decision 3; R-7). The Kit cursor stays the
  Kit's selection, which its wrapper announces; the row sets no
  `aria_selected`. The chosen repository carries the selected treatment
  (the `selection` background, the 2 px amber left line, weight 600); the
  line is a mark element with id `("repo-selected", i)` and accessible
  label "Selected" (`SELECTED`, kept). The cursor shows the focus ring
  while the list has focus. Enter or a click chooses, as today.
- **Pointer hook.** Each row has `capture_any_mouse_down`: a right button
  opens the repository menu at the pointer for that row (Part D), with
  `prevent_default`, and changes no choice.
- **The kept row** (decision 22) is drawn last, after the listed rows,
  while `Navigation.kept` holds an entry the list does not give
  (fact 35); it reads Can't open. The header counts the rows drawn, the
  kept row included (R-20).
- **Applied after the frame** (decision 3, D1 polish). `Workspace::render`
  still builds the rows, and when they differ from the pane's it applies
  them in `window.defer`, after the frame, instead of calling `set`
  during the render (R-8; U-4). The journey checks that the rows follow
  the model after the frame (P16); that nothing notifies during render is
  checked in code review, since no test can observe it.
- **Titles of what is not shared** (R-9). A session cache in the
  Workspace, `titles: HashMap<(RepoKey, ItemId), String>`, is filled from
  the Tickets and Documents lists whenever they load. When the scheduler
  notifies and a repository's `last.to_share` holds item IDs neither
  cached nor tried, the Workspace sends one read job for that repository,
  copying D5's made-ready reads (fact 45): resolve it, then `show_item`
  for each such ID, under `ReadKind::ShareTitles` and a new
  `ViewScope::ShareTitles(RepoKey)` with its stamp, so a stale answer is
  dropped. Each ID sent goes into a tried set for the session, so a
  failed or busy read is not repeated on every notify; such an item is
  named by its ID (K-3).
- **List states** of the repositories read, by Cycle 02's helpers, in a
  small body function of the panel's own (fact 11; R-10):

  | List | Shows |
  | --- | --- |
  | First load | `skeleton_rows` labelled "Loading repositories" |
  | First load busy, no rows | `alert_row` "Repositories were not updated: Manyhands is busy." with Retry |
  | Ready, no rows | `empty_state` "No repositories yet." with "Add or create a repository…" (Part E's Add) |
  | Ready but incomplete | rows, and `alert_row` "The list of repositories is incomplete." with Retry |
  | Rows, a reload busy | rows kept, `alert_row` "Repositories were not updated: Manyhands is busy." with Retry |
  | Rows, a reload failed | rows kept, `alert_row` with the failure's message and Retry |
  | No rows, failed | `alert_row` with the failure's message and Retry |

  The Kit list is drawn in every state (C2 R-34), and the delegate
  overrides `render_empty` so the Kit's Inbox icon is not drawn (fact 3).
  Retry is `load_repositories`, as today's `repositories-retry`.
- The in-body heading goes: the header already names the panel (fact 2).
  The Settings button stays below the list until Cycle 04 removes it
  (Wave Cycle 04; decision 15).

## Part D: The Repository Menu

One Workspace-owned popup, built as C1's dock menu is (fact 14; R-11),
opens for a row:

- by a right click on the row, at the pointer, for that row's repository,
  without changing the choice;
- by `menu` or `shift-f10` in the `Repositories` context, at the
  highlighted row's bounds (recorded by a `canvas` in each row during
  prepaint, keyed by root), for the highlighted repository, or the chosen
  one when nothing is highlighted (U-1, U-2).

The anchor type is `MenuAnchor { Pointer(Point<Pixels>),
Bounds(Bounds<Pixels>) }`, named so it does not clash with
`gpui_kit::Anchor`, which the layer uses.

Items, in order, come from a pure `menu_items(&RepoFacts, &RepoState) ->
Vec<MenuItem>` beside `repository_state`; the More menu's from
`more_items(chosen: Option<&RepoKey>)`:

| Item | Does | Disabled when |
| --- | --- | --- |
| The state's sentence, as one line (decision 21) | Nothing | Always (it is read, not chosen); left out when the sentence is empty |
| The state's action, when it has one (Retry, Unlock…, Choose a key…, Approve host…, Open remote settings…, Resume checking, Settings…, Set up sharing…, Check now, Retry now) | As Part B says; `attend` widened for D6's actions (fact 46) | Unlock… when `unlock_target` is `None` |
| Check now, or Retry now when the state is Can't reach or "Can't check"; left out when the state's action is the same | `Intent::PollOnce { root }`, user lane, as Settings' Check now | D6's shape is not `Polling`, or the repository is checking |
| Share main, "Share main…" with no publication remote, "Sharing…" while sharing | `share_main(root, remote)` | `sharing_main(root)`; under Can't open from a resolution or the kept row |
| Open folder | `open_folder(root)` | under Can't open from a resolution or the kept row |
| Settings…, left out when it is the state's action | Settings › Repositories for this repository | — |
| separator, Remove from Manyhands… | `remove_repository(root, restore)` | — |

- `menu_items` reuses `sidebar.rs`'s `SHARE_MAIN`, `SHARE_MAIN_SET_UP` and
  `SHARING` (fact 6).
- **Settings for a row.** Settings…, Set up sharing… and Needs fixing's
  Settings… open the page with the row's repository selected, as
  `open_remotes` does (fact 38; R-21), not the chosen one.
- **Guards.** The three entry points, the `OpenRepositoryMenu` handler,
  the row's right-click opener and the `RepositoryMenuChoice` handler, are
  each wrapped in `unless_dialog`, which refuses behind a dialog or a
  sheet; the layer is dropped under a modal, as the dock menu's. The menu
  closes when its repository leaves the rows (R-23).
- **Remove from the panel.** `remove_repository(root, restore, window,
  cx)` takes the focus to give back from its caller: the list for the row
  menu, the More button for More. It still refuses only under a dialog
  itself, as today; the entry points refuse under a sheet, and Settings'
  own Remove is a separate path (fact 48). `last_region` is the caller's
  region (R-12). The dialog is today's, with its "Local files remain"
  note (fact 18; decision 14).

## Part E: Toolbar

`02` puts the toolbar at the right of the panel header; Cycle 02's
mechanism carries three more buttons (fact 13; C2 Part G):

| Id | Icon | Name and tooltip | Does | Disabled when |
| --- | --- | --- | --- | --- |
| `add-repository-button` | `plus` | "Add or create a repository…" | `open_settings(SettingsPage::Repositories)`, today's flow, until Cycle 05's sheet | — |
| `check-all-button` | `refresh-cw` | "Check all for updates now" | `PollOnce` for each repository whose D6 shape is `Polling`, that is not checking and whose phase is not `Attention` for a key or host (`KeyLocked`, `KeyProblem`, `NoKeySelected`, `HostUnapproved`, `HostUnidentified`), in list order; paused ones included (R-13; decision 16) | none qualifies |
| `repositories-more-button` | `ellipsis` | "More" | Opens the More menu at the button's bounds, recorded by a `canvas` | — |

- **More menu**, the same popup mechanism as Part D, for the chosen
  repository: Refresh (`maintain_index(false)`), Rebuild the search index
  (`maintain_index(true)`), separator, Remove from Manyhands…. Each is
  disabled with no repository chosen.
- `ToolbarButton` gains `AddRepository`, `CheckAll`, `RepositoriesMore`,
  all with `region()` Repositories; `ALL` and `toolbar_focus` grow to 7.
  `toolbar_enabled` takes `cx`, since Check all reads the scheduler.
- **Context.** The Repositories toolbar's wrapper carries
  `REPOSITORIES_CONTEXT`, not `NAVIGATION_CONTEXT` (fact 41), so
  `secondary-n` there does nothing, as from the list (R-14).
- When Check all becomes disabled while it holds focus, focus moves to the
  list, as Cycle 02's Collapse all does (R-22).

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
  | Opening (`Resolution::Unknown`) | No state while resolving (Part B row 13) |
  | Unavailable code, Retry, Remove | Can't open and its sentence; Retry and Remove from Manyhands… in the menu |
  | A selection the list no longer gives | The kept row, Can't open, Retry and Remove from Manyhands… (decision 22) |
  | Busy resolution, Retry | No label, its sentence, Retry in the menu (Part B row 2; R-16) |
  | Failed resolution, Retry | Can't open with the failure's message |
  | Configuration label, problems | Needs fixing and its sentence; nothing for a valid configuration |
  | Primary branch | "Main branch:" in the tooltip |
  | Index state | The Tickets and Documents lists' states, which already say stale, never refreshed or unavailable (C2 decision 11) |
  | Share main | The menu |

- Constants and imports that go once unused: `REPOSITORIES_HEADING`,
  `UNAVAILABLE`, `SHOWING_LAST_READ`, `SHARE_MAIN_BUTTON`, `RESOLVING`,
  `REMOVE`, and `sidebar.rs`'s `index_label` and `ConfigurationDto`
  imports. `NOTHING_SELECTED` moves beside the item area's text.
  `SELECTED` stays (the mark's label), and `RETRY`, `SHARING`,
  `SHARE_MAIN`, `SHARE_MAIN_SET_UP` and `configuration_label`, which other
  files or the menu use, stay where they are (fact 6).

## Part G: Footer Status Items

`02` "Footer" items 3 and 4, between the spacer and Needs attention
(fact 31):

| Control | Id | Shows | Name and tooltip | Activates |
| --- | --- | --- | --- | --- |
| `UpdateCheck` | `footer-update-check` | `refresh-cw` (teal) and "Checking for updates…" while any repository is checking (`checking_now` or `in_flight()`); otherwise "Checked for updates {ago}" from the latest `last_completed` of any scheduled repository; hidden when there is none | The text, then "Background check: fetch and fast-forward only. Never shares your changes." | Settings › Checking for updates (`SettingsPage::Polling`) |
| `ToShare` | `footer-to-share` | `cloud-upload` (amber) and "{n} to share": the sum of `last.to_share.len()` over repositories whose last answer is `Completed`, whatever their row shows (R-24); hidden at 0 | "{n} to share. Select {repo}." naming the repository it chooses next; "{n} to share in {repo}." when the only one with work is already chosen | Chooses the next such repository in list order after the chosen one, wrapping (decision 6); does nothing when the only one is already chosen (decision 23) |

- `{ago}` is D6's `ago`, made `pub(crate)` (fact 24).
- `plan(layout, &FooterStatus)` replaces `plan(layout, attention)`, with
  `FooterStatus { update: Option<String>, to_share: usize, attention:
  usize }`; the right side's first group is `[UpdateCheck?, ToShare?,
  Attention?]`, empty ones dropped. `Control::ALL` grows to 11.
- Both are focusable, reached by Tab, activated by Enter or Space, and
  refused under a dialog or sheet by `press` (C1 R-29).
- Choosing a repository by `ToShare` keeps focus on the control and never
  changes the layout: a hidden Repositories panel stays hidden
  (decision 23).

## Part H: Keyboard And Focus

- **Unchanged:** F6 and Shift F6, `secondary-1` (focuses Repositories),
  up and down in the list with Enter choosing, the region ring.
- **New:** `menu` and `shift-f10` open the repository menu in the
  `Repositories` context (`OpenRepositoryMenu`), whose name, today the
  literal `"Repositories"` (`D/view/workspace/focus.rs:100`), becomes the
  constant `REPOSITORIES_CONTEXT` in `D/keys.rs`; up, down, Enter and Esc
  in the menu (the Kit's). The toolbar buttons and the two footer items
  are Tab stops with accessible names.
- **Focus** (R-25):

  | Event | Focus |
  | --- | --- |
  | Esc in a menu opened by key | The list |
  | Esc in a menu opened by pointer | What held focus before |
  | Esc in the More menu | The More button |
  | Settings…, Set up sharing…, "Share main…", Open remote settings… | The page they open in the item area, as today (fact 38) |
  | Check now, Retry now, Retry, Share main, Open folder, Unlock…, Resume checking, Choose a key…, Approve host… | Where the menu was opened from (the list, or the More button), unless the action opens its own dialog, which gives focus back there when it closes |
  | Remove from Manyhands…, Cancel or OK | Where the menu was opened from (R-12) |
  | A toolbar button | Stays on it; Check all moves it to the list when it disables itself (R-22) |
  | The footer's To share | Stays on it |

## Part I: Privacy

Every new text is built from repository names, paths, branch names,
remote names, the library's redacted remote locations, item titles and
fixed copy. No key, passphrase, fingerprint or unredacted URL is read into
`RepoFacts`.

- `polling_privacy` (`D/view/polling_privacy_tests.rs:406`) no longer
  allows `region-repositories` (fact 44), so the row labels are swept, and
  gains surfaces for `repo-` and `footer-` ids (`SURFACES` grows to 17).
- It sweeps `repository_tooltip`, `menu_items`' labels and the footer's
  names for every repository through their functions, built from the live
  workspace's facts (through accessors on the Workspace), against the
  passphrase, private-key and URL-password sentinels; not against the
  title sentinel, since titles appear by design (fact 44).
- **Positive control:** at least one "Shared copy:" line exists for the
  fixture's eligible SSH remote, so the sweep reads real tooltips.
- A remote whose URL carries a password sentinel: the sentinel appears
  nowhere, whether or not such a remote is publication eligible (U-3).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Each row of Part B's table, alone | Its state, label, icon, action and sentence |
| P2 | Several rows hold at once, including a stored outcome while checking and a live attention while checking | The first wins, in Part B's order; neither a stored outcome nor a live attention outranks Checking… |
| P3 | A stored outcome and no attempt this session; then an attempt | The stored outcome's state; then the attempt's; Settings' line agrees at each step |
| P4 | An attempt with `LocalAhead`, `Unpublished` and `Diverged` targets and main ahead | "{n} to share" counts all of them, main as one; Settings' "N not shared yet" says the same |
| P5 | A finished `ItemSync` whose publication is `Published` (also when `Partial`); an `ItemRepublish`, `TicketClose` or `DocumentPromote` that published; one that did not publish; a `RepoSync` that published | The item leaves the count; the same; unchanged; main leaves the count |
| P6 | Titles known from a loaded list; unknown, read; the read fails; another notify | Named by title; by title after the read; by ID; the failed read is not sent again; a stale answer is dropped |
| P7 | The tooltip and description of each state | Part B's lines, and nothing else |
| P8 | The repositories read: loading, first load busy, ready empty, ready incomplete, busy reload, failed reload, failed with no rows | Part C's state, with its action; the rows kept where it says |
| P9 | A click or Enter on a row; arrows | Chooses it, which carries the mark; moves the Kit cursor only; the row sets no `aria_selected` |
| P10 | The menu by right click and by `shift-f10` and `menu` | Opens for that row; its items and enablement as `menu_items` says; each does what Part D says |
| P11 | Remove from Manyhands… from the row menu and from More, then Cancel; then OK | The dialog with its note; focus back where the menu was opened; the repository removed and its row gone |
| P12 | A menu entry point, a toolbar button or a footer item behind a dialog; behind a sheet | Refused; an open menu closes under a modal |
| P13 | The toolbar by Tab and Enter | Add opens Settings › Repositories; More opens its menu and Refresh submits `IndexRefresh`; disabled as Part E says; the region holds focus |
| P14 | No tab shown | "Select a repository." or "Open a ticket or document from the lists"; nothing about the repository in the item area or at the end of a tab |
| P15 | The footer with checks done, one running, none; with work in two repositories | "Checked for updates {ago}", "Checking for updates…", hidden; "{n} to share", and each activation chooses the next repository with work |
| P16 | The model changes | The rows follow after the frame |
| P17 | Every text P1–P15 shows, with passphrase, private-key and URL-password sentinels in play | No sentinel; a "Shared copy:" line exists |
| P18 | The chosen repository's registration removed by another process | The kept row reads Can't open; Retry and Remove from Manyhands… work from its menu; it goes when removed or when another is chosen |
| P19 | A completed check that lists a `Dirty` or `Conflicted` item and nothing to share | Needs attention, "1 item needs a look. Activity lists them."; not Up to date |
| P20 | A publication remote that is not SSH | No label; decision 20's sentence; Set up sharing… opens Settings › Remotes for that repository |
| P21 | The menu of each state | Its first line is the state's sentence |
| P22 | Opening the menu, moving the cursor, Esc | No read beyond those a choice makes, no request (contract 2) |
| P23 | Check now and Check all | Send only `PollOnce`; nothing is shared (contract 3) |
| P24 | Check all with a paused, a locked and an ordinary repository | Checks the paused and the ordinary one; skips the locked one |
| P25 | The footer's To share when the only repository with work is already chosen | Nothing happens, no read; its name says "in {repo}" |
| P26 | Retry on a row that is not chosen and reads Can't open because the scheduler could not read it | That repository's registration is read again |
| P27 | Settings… on a row that is not chosen | Settings opens on that repository |
| P28 | `secondary-n` from a Repositories toolbar button | Nothing opens |
| P29 | A reload that drops the repository of an open menu | The menu closes |

## Testing

Desktop-only, so AGENTS.md's fast tier at review-ready and no SSH target.

- Between tasks, only the tests a task wrote or changed run, by exact name
  (AGENTS.md, Testing Strategy; decision 18).
- Behaviour, not appearance: no tests for colours, icons, tones, sizes or
  that a view renders. Icons and tones in Part B's table are not tested.
- Decision logic lives in pure functions with one table-driven test each;
  window tests are few, each a journey that also checks what is sent.
- Tooltip and description text is tested through its function (fact 12);
  menu enablement through `menu_items` and `more_items` (fact 43). Window
  tests read menu items by label and check that activating a disabled one
  sends nothing.
- Stored outcomes come from `PollingBackend::store_outcome` (Part A);
  attempts, phases and attention from `schedule_mut()` or the existing
  `polling_support::Rig` and `PollingBackend` answers; window tests that
  need the scheduler opt in with `with_polling()` (fact 26). **Every poll
  is answered explicitly**: `answer_poll` for each launch poll, and
  `answer_job(JobKind::PollOnce, …)` or `hold_job` for each check, so no
  test reaches `example.invalid` (fact 42). No SQL on the registry. No
  real waits; `now` is passed in.
- Guard tests cover a sheet (the item details sheet) as well as a dialog,
  with key presses or dispatched actions (a pointer press behind a dialog
  lands on its overlay).

| Test | What |
| --- | --- |
| `desktop::model::repository_state::tests::each_state_and_its_precedence` | One table over Part B's rows, alone and in pairs, with stored outcomes, live attempts, problems, the kept row and a non-SSH remote (P1–P3, P18–P20) |
| `desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts` | One table of `repository_tooltip` per state, with and without branch, shared copy and titles (P6, P7) |
| `desktop::model::repository_state::tests::menu_items_follow_the_state` | One table of `menu_items` and `more_items`: first line, order, labels and enablement per state (P10, P13, P21) |
| `desktop::model::polling::tests::schedule::to_share_counts_and_finished_shares` | `AttemptSummary::of` with `LocalAhead`, `Unpublished`, `Diverged`, problems and main; `shared` for an item, main, unknown and twice (P4, P5, P19) |
| `desktop::model::polling::status::tests::status_text_table` (E-9) | Gains the stored-outcome rows of Part A (P3) |
| `desktop::services::scheduler::reconfigure::tests::finished_shares_are_heard` | A finished `ItemSync` (`Success`, `Partial` with `Published`, `Error`), `ItemRepublish`, `TicketClose`, `DocumentPromote` and `RepoSync` with `Published`, `Current` and `NotRequested` publication give `Heard::Shared` or nothing (P5) |
| `desktop::services::scheduler::entity_tests::the_stored_outcome_and_shared_copy_are_kept` | With `store_outcome` and a fixture remote: kept on `registered`, replaced on a later read, dropped with the repository; `reread` of a failed root reads it again (P3, P26) |
| `desktop::view::repositories::tests::rows_and_list_states` | A window journey with polling on: rows with states from the schedule; a finished share drops the count; the kept row; list states with Retry; click, Enter and arrows; the rows after the frame (P5, P8, P9, P16, P18) |
| `desktop::view::repositories::tests::titles_of_what_is_not_shared` | Titles from a loaded list and from a `ShareTitles` read (held, answered), one failed and not read again, a stale answer dropped (P6) |
| `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard` | Right click and `shift-f10`; the first line; each item's request or effect, by label; a disabled item sends nothing; Remove with Cancel and OK and focus; Retry on an unread row; Settings… on another row; refused under a dialog and a sheet; no read after opening, moving and Esc; the menu closing when its row goes (P10–P12, P21, P22, P26, P27, P29) |
| `desktop::view::repositories::tests::check_all_checks_each_repository` | With a paused, a locked and an ordinary repository: two `PollOnce`, none for the locked one; nothing else sent (P23, P24) |
| `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter` (E-11) | Gains Add and More; `secondary-n` from a Repositories button opens nothing (P12, P13, P28) |
| `desktop::view::footer::tests::status_items_say_and_choose` | `plan` with `FooterStatus`; the two items' texts and names by function; Enter on To share cycles the choice, and does nothing for the only chosen one; Update check opens Settings › Checking for updates; refused under a dialog and a sheet (P12, P15, P25) |
| `desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository` | No tab: the two texts; a tab: no repository details at its end (P14) |
| `desktop::view::polling_privacy_tests::polling_privacy` (E-15) | Part I's sweep and its positive control (P17) |

Pre-existing tests change as the named edits say.

## Named Edits To Pre-Existing Tests

At `7034522f`. Task 0 re-counts with `git grep`, over tests and
production code, and records any other site as a further edit (decision
17). Edits that remove assertions are marked **Removes**.

| E | Tests | Edit | Part |
| --- | --- | --- | --- |
| E-1 | `D/view/workspace_tests.rs`: `selecting_a_repository_resolves_it_and_shows_its_configuration_and_index` (:709), `a_renamed_away_repository_is_unavailable_until_retry_finds_it_again` (:753), `a_registration_removed_by_a_second_actor_shows_unavailable` (:793), `a_list_reload_after_a_second_actor_removes_the_selection_shows_unavailable` (:819), `a_held_resolution_of_a_never_replaces_b` (:939), `reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor` (:1098) | Its row part goes with the rows (Part C): `("repo-unavailable", i)` (:776, :785) becomes `("repo-state", i)` reading "Can't open". Its details part goes with Part F: `selection-name` becomes the chosen row's `("repo-selected", i)` mark; `selection-unavailable*` becomes `("repo-state", i)` reading "Can't open", on the kept row for the two second-actor tests (decision 22); `selection-retry` becomes the menu's Retry; `selection-configuration` becomes the row's state (Needs fixing, or none) with the configuration checked through `repository_tooltip`. **Removes** the "Configuration valid" and index-state assertions (the index state is the lists', fact 11) | C, F |
| E-2 | `D/view/workspace_tests.rs` `a_busy_list_reload_keeps_the_last_rows_with_a_label` (:900); `D/view/settings/repositories_manage_tests.rs` `tabs_close_once_the_list_no_longer_gives_their_root` (:647) | `repositories-state`, `repositories-last-read` and `repositories-retry` become the busy alert row and its action `(id, "action")`; the row labels are unchanged; the first test's `selection-name` read (:927) becomes the chosen row's mark with Part F. **Removes** "Showing the last list read" | C, F |
| E-3 | `D/view/workspace_tests.rs`: `the_fixtures_repositories_are_listed_by_name` (:696); the `selection-name` read in `the_repository_selector_selects_with_arrows_and_enter` (:1055) | `("repo-root", 0)` goes (the folder is in the tooltip, P7); `selection-name` becomes the chosen row's mark with Part F. `("repo-selected", i)` stays as the mark, so `the_repository_selector_…`'s other steps, `a_repository_rows_selected_state_has_one_meaning` and `keyboard_tests.rs`'s `select_a_repository` are unchanged. **Removes** the root-path label | C, F |
| E-4 | `D/view/tabs_tests.rs` `opening_a_tab_does_not_change_the_selection` (:190) | Reads the chosen row's mark instead of `selection-name` | F |
| E-5 | `D/view/keyboard_tests/actions.rs`: helper `unavailable` (:173), `retry_an_unavailable_repository_by_keyboard` (:187), `remove_an_unavailable_repository_by_keyboard` (:203) | The steps go to the Repositories region and open the menu with `shift-f10`; Retry and Remove from Manyhands… are chosen there | D, F |
| E-6 | `D/view/settings/repositories_manage_tests.rs` (path `desktop::view::settings::repositories::manage_tests::…`): `removing_a_registration_whose_folder_was_renamed_away_succeeds` (:533), `the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go` (:731) | Remove is reached through the menu, for the second test the kept row's (decision 22); the focus after the dialog is the list | D, F |
| E-7 | `D/view/item/share_tests.rs`: `share_main_in_the_repository_detail_submits_repo_sync` (:232), `share_main_without_a_remote_reads_share_main_and_opens_remotes` (:280), `an_accepted_share_offers_stop_and_stopping_signals_it` (:537), `a_stopped_share_says_what_completed_and_offers_share` (:599), `share_opens_remotes_on_its_own_repository_after_another` (:789); `share_main_at_the_end_of_an_item_tab_submits_repo_sync` (:299) | Share main is chosen from the menu, by label, and its label checked through `menu_items`. `share_main_at_the_end_of_an_item_tab_…` is deleted: no tab carries the details. **Removes** | D, F |
| E-8 | `D/view/keyboard_tests/discussion.rs` `tab_order_runs_header_share_chips_page_discussion_composer` (:297) and its `SHARE_MAIN` constant (:34) | The Tab order ends at the composer; the steps to and from `selection-share-main` go. **Removes** | F |
| E-9 | `D/model/polling_schedule_tests.rs` `attempt_summary_counts` (:10; path `desktop::model::polling::tests::schedule::…`), whose fixture lists every disposition once, so `not_shared()` is now 3 (`LocalAhead`, `Unpublished`, `Diverged`) and `problems` 10; `D/model/polling/status_tests.rs` `status_text_table` (:73) and every `status::Facts` literal (not `retire_loop::Facts`); `D/view/settings/polling_tests.rs` `settings_polling_rows` (:43; path `desktop::view::settings::polling_tests::…`) | `not_shared` is a method returning `to_share.len()`; `status::Facts` literals gain `stored: None`; the table gains Part A's stored rows; texts with "not shared yet" follow the new count. `PillState::EditingSaved { not_shared }` (`D/view/item/pill.rs`) is unrelated and unchanged | A |
| E-10 | `D/view/footer_tests/mod.rs`: `a_secondary_click_opens_the_dock_menu_with_its_side_checked` (:191-199), `toggle_names_follow_their_state` (:445-464); `D/view/keyboard_tests/shell.rs`: `each_side_is_reached_by_the_dock_menu_keys` (:353-356), `footer_controls_are_reached_by_tab` (:409-415); `keyboard_tests/shell/support.rs` `said` (:93) | `plan` takes `FooterStatus`; `said` gains the two controls; the Tab walk keeps an explicit expected list for its fixture, in which nothing is scheduled, so the two status items are absent there and walked in `status_items_say_and_choose` | G |
| E-11 | `D/view/dock_tests/toolbars.rs` `panel_toolbars_work_by_tab_and_enter` and `with_no_repository` | Add and More join the loop that asserts enablement with a chosen repository; Check all is left out of it, since its window has no schedule, and is tested in `check_all_checks_each_repository`; `with_no_repository` names the three with their enablement (Add enabled, Check all disabled, More enabled with its items disabled) | E |
| E-12 | `D/assets_tests.rs` `every_item_view_icon_loads` and its fixed-length `ICONS` array | Each icon Parts B, E and G use is checked first; one that is neither in `ItemIcons` nor a Kit built-in is added to both, and the array's length follows (`hard-drive`, `pause`, `lock`, `loader-circle`, `refresh-cw`, `ellipsis`, `plus`, `triangle-alert` are not there at `7034522f`) | B, E, G |
| E-13 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | Gains `menu` and `shift-f10` → `OpenRepositoryMenu`; the table becomes 26 long; it checks that the binding exists | H |
| E-14 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour and stops for the owner. **Removes** | — |
| E-15 | `D/view/polling_privacy_tests.rs` `polling_privacy` | `region-repositories` leaves `ALLOWED`; Part I's surfaces, sweep and positive control | I |

## Document Amendments

Applied by the planner in the approval commit, not by an implementation
task:

1. **Wave Cycle 03.** Split into "Cycle 03a: Repositories Panel And
   Sharing Status" (this Cycle: toolbar, rows, states, context menu,
   footer "to share" and "Checked for updates", the repository details out
   of the item area) and "Cycle 03b: Activity And Needs Attention"
   (Activity, the popover, the three K-19 rows, the rest of Cycle 03's
   exit evidence, and which repository states the Needs attention count
   includes); the Cycles table and planned document paths with them; the
   "Design section | Owning Cycle" table (`03` Repositories → 03a, `03`
   Activity → 03b, `05` Needs attention popover → 03b). Remove "when D6
   provides it" and "once D6 has merged" (D6 is merged) and "proposed"
   before K-19 (accepted 2026-11-09). Exit evidence for 03a: each state
   from real service state; every action by keyboard; no secret in a row,
   tooltip, menu or footer item.
2. **Handoff `03`, Repositories.** The state table gains Can't open, Needs
   fixing, Locked and Needs attention (decision 4), puts Checking… above
   Locked, Needs attention and Can't reach (decision 8), counts a diverged item as to share and
   reports other problems as Needs attention (decision 19), and shows no
   label for a non-SSH remote (decision 20); "N to share" counts items not
   shared yet, main as one, not checkpoint commits (decision 2); the
   context menu opens with the state's sentence and action (decision 21);
   More holds Refresh, Rebuild the search index and Remove from
   Manyhands….
3. **Handoff `02`, Footer item 4.** "N to share" is the total across
   repositories; each activation chooses the next repository with work
   (decisions 6, 23).
4. **Handoff `06`.** "Unpublished checkpoints → N to share" counts items
   not shared yet.
5. **Handoff `08`.** The repository context menu is a Workspace popup like
   the dock menu, not the Kit `ContextMenu` (R-11); repository rows are
   the panel's own element (R-7).
6. **Cycle 01 document.** Its "until Cycle 03" footer text points to Cycle
   03b.
7. **Wave Cycle 05.** The empty item area's text is delivered by Cycle 03a
   (decision 13); Cycle 05 adds the icon.
8. **Wave 03 D6 design.** A4's counts: "N not shared yet" counts
   `LocalAhead`, `Unpublished` and `Diverged`; A2: `last` is not restored
   at launch, only `last_completed` (fact 33); C4's status sources gain the
   stored outcome (decision 10).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | The stored `latest_outcome` lives in a scheduler side map, not in `Policy`; it informs only a repository with no attempt this session that is not checking | A stored outcome older than a later success by another process is shown until this process checks (K-2) |
| R-2 | The shared copy's name and location come from the `publication_remotes` read the scheduler already makes | — |
| R-3 | A finished `ItemSync`, `ItemRepublish`, `TicketClose`, `DocumentPromote` or `RepoSync` whose publication is `Published` or `Current` removes its target from `last.to_share` at once, by the drain's own predicate (fact 40) | A share by another process, the CLI or a comment's publication shows only after the next check (K-1) |
| R-4 | The stored-outcome seam is a desktop test double, not SQL on the registry | — |
| R-5 | A stored `UnlockRequired` offers Check now, not Unlock…: the stored row names no key, and a user-lane check asks for the passphrase (decision 9; U-6) | One more step before unlocking after a relaunch (K-4) |
| R-6 | A row with no known state shows no label; its tooltip, description and menu still carry its sentence | — |
| R-7 | Rows are the panel's own element, not `ListRow`, because the Kit overwrites `selected` with its cursor (fact 34): the Kit cursor is the list's selection, announced by its wrapper; the chosen repository carries the selected treatment and the `("repo-selected", i)` mark labelled "Selected"; the cursor draws the focus ring | A screen reader hears the chosen repository through the mark and description, not through selection |
| R-8 | Rows are built at render and applied in `window.defer` when they changed (U-4) | Rows lag one frame |
| R-9 | Titles: from loaded lists, else one read job per repository (resolve, then `show_item` per unknown ID) under `ViewScope::ShareTitles`, triggered by the scheduler's notify, with a tried set for the session; IDs when a read fails | A renamed item keeps its old title until the lists reload; a title that failed once is not retried this session |
| R-10 | The repositories list's states are a small body function of the panel's own, using `rows.rs`'s helpers; `list_body` is not widened | — |
| R-11 | The menu is a Workspace-owned `PopupMenu` layer like the dock menu, opened by right click and by `menu` and `shift-f10`; not the Kit `ContextMenu`, which has no keyboard path (facts 14, 15) | — |
| R-12 | `remove_repository` takes the focus to give back from its caller and keeps its own dialog guard; the menu and toolbar entry points refuse under a sheet; Settings' Remove is unaffected (fact 48) | — |
| R-13 | Check all sends one `PollOnce` per repository whose D6 shape is `Polling`, not checking and not waiting on a key or host, paused ones included (decision 16) | A person who paused a repository sees it checked once |
| R-14 | The Repositories toolbar's wrapper carries `REPOSITORIES_CONTEXT`, so `secondary-n` there does nothing, as from the list (fact 41) | — |
| R-15 | The item area's empty text is `04`'s, without its icon, until Cycle 05 | — |
| R-16 | A busy resolution shows no label, its sentence, and Retry as its action (Part B row 2) | — |
| R-17 | A publication remote that is not SSH: decision 20 (Part B row 12) | — |
| R-18 | Main counts as one item in "to share" | — |
| R-19 | Retry on a row the scheduler could not read calls `Scheduler::reread(root)` (fact 39) | — |
| R-20 | The header counts the rows drawn, the kept row included | — |
| R-21 | Settings…, Set up sharing… and Needs fixing's Settings… open on the row's repository, as `open_remotes` does (fact 38) | — |
| R-22 | When Check all disables itself while focused, focus moves to the list | — |
| R-23 | The repository menu closes when its repository leaves the rows | — |
| R-24 | The footer's "to share" is the sum of `last.to_share.len()` over repositories whose last answer is `Completed`, whatever their row shows; the next repository is chosen among those | The footer can count a repository whose row says Checking… or Locked |
| R-25 | Focus follows Part H's table | — |
| R-26 | Checking… is `checking_now` or `in_flight()`, in the row and the footer (fact 36) | — |
| R-27 | Under Can't open from a resolution or the kept row, Share main and Open folder are disabled | — |

## Known Limits

- **K-1.** "{n} to share" is known only after a check this session, and
  follows this app's shares at once but another process's, the CLI's and
  a comment's publication only at the next check.
- **K-2.** A stored outcome is shown until this process checks, even if
  another process has since checked successfully.
- **K-3.** Items not shared yet are named by ID when their title cannot be
  read; a failed title read is not tried again this session.
- **K-4.** After a relaunch a locked key is offered Check now, which asks
  for the passphrase, rather than Unlock….
- **K-5.** A publication remote that is not SSH shows no label; its
  sentence says Manyhands can't check or share it (decision 20).
- **K-6.** Tooltips show on hover only; their text is also the row's
  accessible description and the menu's first line (decision 21).
- **K-7.** "Checked for updates {ago}" is worked out when the footer is
  drawn.
- **K-8.** The panel's Settings button stays until Cycle 04.
- **K-9.** The footer's Needs attention does not yet count Can't open,
  Needs fixing, a stored Locked or Needs attention, or a check's
  problems; Cycle 03b decides.

## Unverified

- **U-1.** `menu` and `shift-f10` pressed in the Kit list reach a binding
  in the enclosing `Repositories` context (the Kit `List` binds neither).
  Task 5 checks it first; if not, the binding moves to the `List` context
  with a `Repositories` parent.
- **U-2.** A right button press on a row inside the Kit list reaches the
  row's `capture_any_mouse_down`. Task 5 checks it first.
- **U-3.** Whether a remote whose SSH URL carries a password is
  publication eligible. Task 9 records which way it goes; Part I's
  assertions hold either way.
- **U-4.** `window.defer` from `Workspace::render` applies the rows after
  the frame without a second render loop: the pane holds the new rows
  after the frame. Task 3 checks it first; if not, rows are pushed from
  the scheduler observer and the navigation updates instead.
- **U-5.** `keyboard_tests.rs`'s `TAB_LIMIT` (40) still reaches Settings
  with two more footer stops. Task 8 checks it.
- **U-6.** A user-lane `PollOnce` of a repository whose key is locked asks
  for the passphrase, as the `PollingBackend::ask_in_check_now` seam
  suggests (`D/test_support/polling.rs:251`) and
  `unlock_after_cancelled_prompt` shows through the worker's provider
  (`D/services/scheduler_attention_tests.rs:227`). Task 1 checks it; if
  not, R-5's Check now becomes Settings › Shared SSH keys
  (`SettingsPage::Keys`) and decision 9 is raised with the owner.

## Alternatives Rejected

- **A library read of unshared checkpoints.** Counts commits without a
  check, but needs a library ticket first (owner, decision 2).
- **Share all… from the row.** Clears the count in one go, but is new
  batch behaviour (owner, decision 7).
- **A Details… sheet.** Keeps today's block whole, but is a sheet the
  design does not have (owner, decision 5).
- **Dropping a selection the list no longer gives.** Simpler, but loses
  Retry and Remove from the panel (owner, decision 22).
- **`ListRow` for repository rows.** Shared with Cycle 02, but the Kit's
  cursor would become the chosen repository's selection (fact 34).
- **The Kit `ContextMenu`.** Opens by right click only (fact 15).
- **`Policy.stored`.** Breaks six `Policy` literals for a value the
  schedule does not decide by (R-1).

## Risks

| Risk | Control |
| --- | --- |
| The state table and D6's Settings line drift apart | Both read `stored`, `to_share` and `problems` from one model; P3 and P4 check both |
| Removing the details block loses a recovery route | Part F's table maps every part, the kept row included; E-1 to E-8 keep each journey through the menu |
| The footer's new stops break the Tab walks | E-10; U-5 |
| Rows and the menu read Workspace state while panels are built inside `workspace.update` | Rows are computed by the Workspace and pushed to the pane (R-8); the menu is a Workspace layer |
| A window test reaches the network | Every poll and check is answered explicitly (Testing) |
| D1, D4 and D5 follow-up tickets edit the same files | They have no commits on their branches; the D1 items this Cycle absorbs are noted on that ticket at closure |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | Split, "to share" source, Activity's shape, fold-ins | Owner decisions | Decisions 1–3 | Session, 2026-11-12 |
| Owner, 2026-11-12 | Row labels, the details block, the footer, what "to share" offers | Owner decisions | Decisions 4–7 | Session, 2026-11-12 |
| Planning research | `repo sync` shares main only; no library count of unshared checkpoints; the stored outcome dropped; a share changes nothing in the schedule; tooltips unreadable by tests; the details block drawn at the end of every tab | Facts, rulings, decisions (proposed) | Facts 5, 17, 27–29, 12; Parts A, F; decisions 8–18 | This document |
| Review round 1, contract and consistency (`f6d57bd4`) | 1 Blocking, 17 Important, 11 Minor; among them Retry lost for a busy resolution, stored outcomes outranking Checking…, `RepositoryUnavailable` as Can't reach, Retry on an unread row, Remove's focus, guard tests without a sheet, contracts without acceptance rows, the footer's sum and single repository, Check all and locked keys, Task 3's size | Rulings, revised proposals, owner questions | R-12, R-16, R-19, R-22–R-27; decisions 9, 11, 16 revised, 23 proposed; owner questions answered as decisions 19–21; Task 3 split | This document; Cycle Review Record |
| Review round 1, source and feasibility (`f6d57bd4`) | 2 Blocking, 13 Important, 13 Minor; among them `ListRow` unable to carry one meaning of selected, the kept selection without a row, `Phase::Running` never entered, the share predicate, Settings… on the chosen repository, the network in window tests, the privacy sweep allowing the panel, menu enablement unreadable | Facts, rulings, owner question | Facts 33–48; R-3, R-7, R-9, R-14, R-21, R-26; decision 22; Testing | This document |
| Owner, 2026-11-12, second round | Problems a check lists, non-SSH remotes, the reason for keyboard users, the kept selection; and what the details block was | Owner decisions; clarification | Decisions 19–22 | Session, 2026-11-12 |
