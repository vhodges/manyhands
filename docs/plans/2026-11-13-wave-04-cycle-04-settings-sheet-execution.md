---
title: "Wave 04 Cycle 04 Execution Ledger"
date: 2026-11-14
status: "draft"
author: "Claude"
---

# Wave 04 Cycle 04 Execution Ledger

Written only by the orchestrator. Ticket `01M7AMAPNHHTNECXBF4CA5WPKV`,
branch `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`. Execution is
subagent-driven and serial (decision 14): one fresh implementer per task,
then a fresh reviewer.

Rulings are recorded as "Ruling: decision — why — cost if wrong". Test edits
are E-1 to E-18 (named) and E-19 onward (further, decision 7), each with its
reason. Commit trailer: `Co-Authored-By: Claude Sonnet 5.5
<noreply@anthropic.com>` (Ruling 1).

## Task state

| Task | Status | Commit | Pushed |
| --- | --- | --- | --- |
| 0 | done | merge `82df6942`; design citations `9f89a3bb`; ledger commit follows | with the ledger commit |

## Branches published

- `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`, after each task (authorized,
  owner 2026-11-13). No other branch published yet.

## Pre-flight scan rulings

Ruling 1: the trailer is `Co-Authored-By: Claude Sonnet 5.5
<noreply@anthropic.com>`, not the plan's Opus 5.5 — the handoff says "your
own session's trailer", and this session is Sonnet 5.5 — cost if wrong:
cosmetic.

Ruling 2: the plan's "Focus assertions read focus inside the same update that
causes the change" gives way to design A6 where they conflict: every focus
assertion that is an A6 row is read after at least one draw (a second after
`focus_lost` moves it); a read inside the update that moved focus does not
count (A6; handoff). Other focus assertions keep today's style — cost if
wrong: one test reads late; none reads early.

Ruling 3: Task 2 also writes F1 and F2 as one test
(`focus_on_open_survives_the_sheet_keeping_focus`, in
`settings_sheet_tests.rs`), from the routes that exist at that point, read
after two draws. A7.1 is created by Task 2's Step 5 (focus found outside the
sheet goes to the page's entry), while P4, which owns F1 and F2, is not
written until Task 6. Task 6 folds it into P4 — cost if wrong: one extra
test.

Ruling 4: for Tasks 4 to 5, while the menu still has five entries, the
design's B1 and C2 govern. Where they are silent, the Keys and Hosts entries
both show the combined Sharing & keys body (Keys view, then Trusted servers),
the menu highlights by `nav()`, and Task 5 collapses the entries. The Task 4
implementer records what it did — cost if wrong: Task 5 reworks the interim.

Ruling 5: Task 0 is split. The orchestrator merged and ran the static gates;
the implementer did Steps 3 and 4 read-only (a design-citation commit allowed)
and built nothing; the orchestrator writes this ledger — cost if wrong: none.

Ruling 6: module runs follow the handoff ("Tasks 1 to 6 each run, once by
module, the test files they touched and those Task 0's sweep listed"), which
post-dates and cites AGENTS.md's Testing Strategy. 15-minute cap, each run
recorded — cost if wrong: a longer test cycle.

## Task 0 rulings (after the implementer's report)

Ruling 7: W-36 and E-19 to E-23 live in this ledger, not in the design's
Part D and E table. The plan says "Task 0's W-36 onward" and "Only the ledger
assigns E-numbers past E-18" — cost if wrong: an implementer misses the row.
Every dispatch that touches them carries the pointer.

Ruling 8: the Activity ⋯ menu (`activity.more`) is a fifth Workspace layer
that F8 does not list, and `ActivityMore` and `DockMenu` expose no `restore`
or `context`. Task 2 treats both as F8 layers: the control a layer returns to,
else its row or list, else F9; it adds an accessor to the layer where it needs
one (desktop code, no library change). The row of A6 is unchanged — cost if
wrong: Task 2 adds two accessors that a later review removes.

Ruling 9: two production sites no task named are assigned:
`D/view/settings/attention.rs:11-33` (W-32) and `settings/polling_attention.rs:85`
to Task 6; `D/view/confirm.rs:71` `PINNED_KEY` "Pinned key" to Task 7, with
E-22's tests following the constant — cost if wrong: a stale word survives
until review.

Ruling 10: `services/requests.rs:424-442` (`Select remote`, `Remove remote`,
`Remove repository`, `Select key`, `Unregister key`, `Delete key`, `Approve
host`, `Replace host key`) and `model/operations/recovery.rs:296-297` carry
replaced terms into Activity row titles. They are not Settings copy constants
and are outside P18. Left unchanged and listed for the owner in the
review-ready summary — cost if wrong: Activity rows keep the old words.

Ruling 11: R-3 and "Alternatives Rejected" keep "21 test files" (rulings are
not edited); fact 3 says 24 — cost if wrong: none.

## Task 0: baseline, references, ledger

- **Precondition (Step 1):** met. `git log origin/main` shows "Integrate closed
  ticket 01M784Y94Q3AXT6XJX38F5VK1V" (`ea6d899b`), which is origin/main.
- **Merge (Step 2):** branch `de9a2095` merged `origin/main` `ea6d899b` into
  `82df6942`. One conflict, `docs/design/08-gpui-kit-mapping.md`, resolved by
  keeping both amendments: 03b's Needs attention row and Cycle 04's Settings
  layout row. `git merge-base --is-ancestor origin/main HEAD` holds.
  `Cargo.lock` and `Cargo.toml` are unchanged against `ba95e1b7`:
  gpui-kit, gpui-base and gpui-component all 0.6.6.
- **Static gates at `82df6942`:** `cargo check --all-features --locked`,
  `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked
  -- -D warnings` all exit 0 (13:56 to 13:58). No test ran.
- **Boundary:** `git diff --stat origin/main...HEAD -- src ':!src/desktop'
  tests Cargo.toml Cargo.lock packaging` prints nothing.
- **Re-map (Step 3):** commit `9f89a3bb` edits only the design: 86 citation and
  fact edits, no ruling, contract, A6 row or decision changed. 47 facts
  re-checked (20 cite changed files, all claims hold); 35 W rows re-checked (23
  re-lined, none changed meaning). W-36 is new. **U-5 answered: 03b's merged
  code carries what the design needs** (an origin through `choose_activity`;
  the popover closing on a sheet), see 3.4 and 3.6 below.

The implementer's findings follow, as written, for Tasks 1 to 8. Paths are
under `src/desktop/` unless they start `docs/`; "D/view" is
`src/desktop/view`.

### Step 3.7 Cargo.lock

`git diff ba95e1b7 HEAD -- Cargo.lock` is empty (0 bytes); `Cargo.toml` is
unchanged. gpui-kit 0.6.6, gpui-base 0.6.6, gpui-component 0.6.6, as at
`ba95e1b7`. Facts 26 and 33-39 stand. Nothing outside `src/desktop`, `docs/`
and `.manyhands/` changed.

### Step 3.2 Re-mapping of Baseline Facts 1-47

Holds = the claim still holds at HEAD. 20 facts cite changed files; 27 do not.

Facts that cite only unchanged files (claim and citation unchanged):
1, 2, 4, 5, 6, 7, 9, 11, 12, 15, 16, 17, 18, 19, 20, 28, 32, 33, 34, 35, 36, 37,
38, 39, 40, 43, 45. (Fact 12's tests `mod_tests.rs:73-118`, `frame_tests.rs:148-178`
are unchanged files.)

| Fact | Old citation | New | Holds? |
| --- | --- | --- | --- |
| 3 | "21 test files" call `settings_shown` | 24 files (adds `activity_support.rs`, `activity_tests/cases.rs`, `needs_attention_tests/hidden.rs`) | Holds, count corrected in fact 3 only (R-3 and Alternatives still say 21; left, they are rulings) |
| 8 | `workspace.rs:71`, `:285-289`, `:91`, `:290-296` | `:74`, `:288-292`, `:94`, `:293-299` | Holds |
| 10 | `requests.rs:285-297`, `:217-225` | `:305-317` (fn to :318), `:220-228` | Holds |
| 13 | `render.rs:121-122` | `:108-109` | Holds |
| 14 | `keys.rs:308-311`; `render.rs:286-297` | `:396-399`; `:276-287` | Holds |
| 21 | `repository_menu.rs:243`; `operations/attention.rs:119-152` | `:251`; `:27-63` (file was rewritten, `attend` is now 64 lines) | Holds. "33 triggers" is a `ba95e1b7` count; there is now a 36th (W-36) |
| 22 | `lifecycle.rs:504-507`; `requests.rs:177-186` | `:505-508`; `:180-189` | Holds |
| 23 | `footer.rs:77`, `:92`, `:44`, `:327`; `workspace/footer.rs:161-178` | `:81`, `:96`, `:44`, `:328`; `:156-173` | Holds (guard `:162-164`, gear `:169`, update item `:170`) |
| 24 | `keys.rs:281`; `render.rs:262-264` | `:347`; `:252-254` | Holds |
| 25 | 03b design `:1044 :2269 :1936 :1941 :2218-2226 :2408` | merged code (see 3.4/3.6) | Holds; the citations now name code |
| 26 | `render.rs:117-152`, `:130-135` | `:104-139`, `:117-122` | Holds |
| 27 | `render.rs:155-158`, `:160-171`; `footer.rs:167`; `toolbar.rs:209` | `:142-145`, `:147-158`; `workspace/footer.rs:162`; `toolbar.rs:241` | Holds. 03b's new layers also drop under `modal_open` (3.6) |
| 29 | `workspace.rs:91`; `render.rs:179`; `lifecycle.rs:226-238` | `:94`; `:166`; `:225-237` | Holds (order unchanged; Quit now focus only while Activity is displayed, `:233-236`) |
| 30 | `retire.rs:208-222` | `:159-172` (`focus_region(Content)` at `:171`) | Holds |
| 31 | `focus.rs:275-295` | `:285-325` (modal branch `:287-292`, popover rule `:301-312`) | Holds; 03b's rule comes after the modal branch |
| 41 | `05-sheets-and-dialogs.md:40-66`, `:46-59`, `:61-66` | `:40-68`, `:48-61`, `:63-68` (the branch's own Cycle 04 note inserted 2 lines at :46) | Holds |
| 42 | `06...md:24`, `:25`, `:36` | `:25`, `:26`, `:39` | Holds |
| 44 | `wave-03-desktop-06-polling-and-shutdown.md:428-430` | `:434-436` | Holds |
| 46 | `quit.rs:259-262`; `operations/retire_asks.rs:159` | `:260-263`; `D/view/retire_asks.rs:159` (the old path was wrong at `ba95e1b7` too) | Holds |
| 47 | `focus.rs:275-295`; `workspace/lifecycle.rs:86-99` | `:285-325`; `:85-98` | Holds |

Also corrected in place: A1 `workspace.rs:91`, "`workspace.rs` is 488 lines" (now
485), A3 "`render.rs:179`, then `footer.rs:51`" (now `render.rs:166`, then
`workspace/footer.rs:46`; definition `footer_status.rs:184`), A7 items 7-9
(`repository_menu.rs:280-293`, `:307-310`, `activity/menu.rs:43`,
`focus.rs:216-228`, `render.rs:168-170`), Part D intro, the Conventions bullet
about "03b design". No claim failed. Total edits: 86 (list: `git show 9f89a3bb`).
Not edited, because they would alter meaning or are rulings: R-3 and
"Alternatives Rejected" still say "21 test files"; W-36 (below) is not added
to Part D.

### Step 3.2 Part D W-1..W-35

Unchanged files (citation and claim hold): W-4 `body.rs:139`, W-5 `body.rs:113`,
W-16 to W-19 (`posting.rs:366-371`, `composer/place/actions.rs:78`; W-11's
`item/share.rs:400`), W-20 `running.rs:317`, W-21 `running.rs:329`, W-22
`lifecycle/actions.rs:143-150`, W-30 `tabs.rs:329-330` -> `content.rs:145-146`,
W-31 `settings/mod.rs:284-285`, W-32 `settings/attention.rs:11`.

| W | Old | New | Holds? |
| --- | --- | --- | --- |
| W-1 | `render.rs:262-264` | `:252-254` | Yes |
| W-2, W-3 | `workspace/footer.rs:174`, `:175` | `:169`, `:170` | Yes |
| W-6 | `toolbar.rs:217-218` | `:249-250` | Yes |
| W-7 | `repository_menu.rs:221-222` | `:229-230` | Yes |
| W-8 | `repository_menu.rs:245-246` | `:253-254` | Yes |
| W-9 | `repository_menu.rs:248` | `:256` | Yes |
| W-10 | `requests.rs:320-321` | `:340-341` (inside `share_main`, `:333-356`) | Yes; new caller, see W-36 |
| W-11 | `requests.rs:201` | `:204` | Yes |
| W-12 | `outcomes.rs:211` -> `requests.rs:202` | `:179` -> `:205` | Yes |
| W-13 | `outcomes.rs:212` -> `requests.rs:203-210` | `:180` -> `:206-213` | Yes |
| W-14 | `outcomes.rs:219` -> `requests.rs:211` | `:187` -> `:214` | Yes |
| W-15 | `outcomes.rs:220` -> `requests.rs:201` | `:188` -> `:204` | Yes |
| W-23 | `attention.rs:147` | `:54` | Yes |
| W-24 | `attention.rs:148` | `:55` | Yes |
| W-25 | `attention.rs:149-151` | `:56-58` | Yes. The defect (Approve host… with no server opens Hosts on the panel's selection, `:58`) is still there, so R-23 still applies |
| W-26 | `requests.rs:615-616` | `:590-591` | Yes |
| W-27 | `requests.rs:627` | `:602` | Yes |
| W-28 | `requests.rs:628` | `:603` | Yes |
| W-29 | `requests.rs:629-630` | `:604-605` | Yes |
| W-33 | `repository_menu.rs:243` -> `attention.rs:119-152` | `:251` -> `:27-63` | Yes |
| W-34 | 03b design `:1936` | `model/activity/states/repositories.rs:78-83`, `:120-126`; `model/repository_state.rs:266`; handler `repository_menu.rs:253-254` | Yes |
| W-35 | 03b design `:1941` | `states/repositories.rs:114-126`; `repository_state.rs:352`, `:357`; handler `repository_menu.rs:256` | Yes |

No W row changed meaning. W-12 to W-15 and W-23 to W-29 are now raised from
Activity rows and popover rows (3.4) as well as from where the table says.

### Step 3.4 How `choose_activity` reaches `attend`, `row_action`, `open_route`

For Tasks 2 and 6.

- `Workspace::choose_activity(&mut self, choice, origin: FocusHandle, window, cx)`,
  `D/view/activity/actions.rs:32-49`. `origin` is a plain `FocusHandle` (not
  `Option`). It returns early at `:39-46` when `still_offered` is false, then
  `highlight_applied_row` (`:47`), then `self.run_activity_action(&choice.action,
  origin, window, cx)` at `:48`.
- `run_activity_action`, `:53-140`, is a synchronous `match` on the action:
  - `ActivityAction::Attend(action, reason, root)` -> `self.attend(*action, reason, root, window, cx)` at `:82-84`
    (`attend`: `operations/attention.rs:27-63`; Settings at `:54`, `:55`, `:57`, `:58`).
  - `ActivityAction::Repository(root, action)` -> `self.row_action(action, root.clone(), window, cx)` at `:85-87`
    (`row_action`: `workspace/repository_menu.rs:237-258`, now `pub(in crate::desktop::view)`;
    `RowAction::Attend` `:251`, `Settings` `:253-254`, `SetUpSharing` `:256`).
  - `ActivityAction::Offer(id, offer)` -> `run_share_offer(*id, *offer, origin, ...)` at `:65-67`
    (`operations/outcomes.rs:231-280`) -> `self.open_route(route, window, cx)` at `outcomes.rs:275`
    (`open_route`: `workspace/requests.rs:197-217`). `open_route` is reached only from
    there, from the lifecycle dialog (`lifecycle/actions.rs:120-123`) and from
    `item_route` (`requests.rs:180-189`, deferred through `in_window`).
  - `ActivityAction::Recovery(id, n)` -> `run_recovery(*id, *n, origin, ...)` at `:68`
    (`requests.rs:559-610`). It calls `window.focus(&origin, cx)` at `:579` first,
    then `retry_generation_in_settings` `:591`, `inspect_in_settings` `:602`,
    `identity_in_settings` `:603`, `check_host_in_settings` `:604-606`. It does not use `open_route`.
  - `Open`, `ResumeLifecycle`, `ReviewPending`, `Abandon` take `origin` and open a
    dialog; the dialog later routes through `in_workspace` (`lifecycle/actions.rs:126-140`)
    after `close()` has focused its own origin (`lifecycle.rs:505-508`).
  - `ActivityAction::Share { item: None }` -> `share_main` at `:107-110`
    (new route to Settings, W-36).
- Origin passed to `choose_activity`: the Activity list's handle
  `activity.pane.list_focus()` for a button and Enter (`workspace/activity_panel.rs:126-143`,
  `:140`; Enter `:101-120` -> `press_activity`) and for a row-menu choice
  (`menu.context`, `workspace/activity_menu.rs:97-105`; `context` is always the list,
  `:32`, `:55`); the footer Needs attention item for a popover row
  (`workspace/needs_attention.rs:278-281`, after `popover = None` and
  `window.focus(&origin)`); the test seam passes the list handle
  (`activity_support.rs:324-334`). Enter on a row with no button
  (`activity_panel.rs:147-163`, `needs_attention.rs:288-313`) calls
  `run_activity_action(OpenItem)` directly: no Settings.
- `attend`, `row_action` and `open_route` take no origin and none is passed.
- Is a `settings_origin` hook around each handler call feasible: yes. Every
  handler above opens Settings synchronously inside `run_activity_action`, with
  `origin` in scope. Set it at the top of `run_activity_action` (`:53`), around the
  `match` (this also covers direct callers `activity_panel.rs:162` and
  `needs_attention.rs:311`, which do not open Settings), or around `:48`.
  Clear it on every exit (a guard; `choose_activity` returns early at `:45`).
  Limits Task 2 must know: (a) the deferred routes (`item_route` -> `in_window`
  `requests.rs:280-301`, and a dialog's `in_workspace`) run after the call, so the
  hook is not set there; they rely on the focused handle, which is right (item
  routes: the chip or composer; lifecycle dialog: `close()` focuses its origin);
  (b) for `run_recovery` the focused handle already is `origin` (`:579`); (c) for a
  popover row the focused handle already is the footer item (`:280`); (d) for an
  Activity button press the focused handle is the row's button, not the list
  (`press_activity` does not focus the list), so the hook is needed there.

### Step 3.5 Every 03b trigger that opens Settings

03b surfaces and their Settings-opening reach. All run through
`choose_activity` (button, row menu entry, Enter) or `press_attention` (popover
button, Enter) -> `choose_activity`.

| Trigger | Route and call (HEAD) | Part D |
| --- | --- | --- |
| Hold row "Choose a key…" (`model/activity/states.rs:210`) | `attend` `attention.rs:54` | W-23 |
| Hold row "Open remote settings…" | `attend` `:55` | W-24 |
| Hold row "Approve host…" | `attend` `:56-58` | W-25 |
| S4 attention-cause row (Choose a key…, Approve host…, Open remote settings…) (`states/repositories.rs:97-125`, `repository_state/cause.rs:70`) | `row_action` `repository_menu.rs:251` -> `attend` | W-33 |
| S4 Needs fixing row, Settings… (`repository_state.rs:266`) | `row_action` `:253-254` | W-34 |
| S4 This computer / non-SSH row, Set up sharing… (`repository_state.rs:352`, `:357`) | `row_action` `:256` | W-35 |
| S1 share-outcome rows: Open Sharing & keys, Check host, Set identity, Set up sharing (`requests/offers.rs:99`) | `run_share_offer` `outcomes.rs:231-280` -> `open_route` `:275` | W-12 to W-15 |
| Recovery rows (`offers.rs:110`): protected generation Retry, `repo.inspect`/`remote.history`, `repo.identity_set`, `host.approve`/`host.replace` | `run_recovery` `requests.rs:590-606` | W-26 to W-29 |
| Lifecycle rows Open…, Resume, Review changes, Abandon open a dialog; the dialog's Check host / Open Sharing & keys / Open repository details route with the dialog's own origin | `lifecycle/running.rs:317`, `:329`; `lifecycle/actions.rs:143-150` | W-20 to W-22 |
| Footer gear, "Checked for updates" | `press` `workspace/footer.rs:169`, `:170` | W-2, W-3 |
| **S6 "Share main" row (an unshared main, `states/items.rs:120-127`)** | `ActivityAction::Share{item:None}` `actions.rs:107-110` -> `share_main` -> `open_remotes` when `publication_remote(root)` is false (`requests.rs:340-341`) | **Missed** |

Other 03b surfaces (filter, ⋯ Clear finished, "Show everything in Activity",
Show it, Open item, Retire, Resume checking, Unlock, Flush retry, Review drafts)
open no Settings route. Callers of every Settings-opening handler were grepped
(`open_settings`, `open_settings_on`, `open_remotes`, `open_hosts`,
`*_in_settings`, `ItemRoute::*`, `settings.open`): no other call site exists.

Misses, numbered on from W-35:

- **W-36**: Activity S6 "Share main" row (button, row menu entry, Enter), when
  the repository has no publication remote at press time (a remote removed after
  the row was derived; `still_offered` does not re-check the remote, and the repo
  menu's Share main… is the same handler, W-10). Route `Remotes(root)`, Shown
  `Settings(Remotes)`, focus `remote-add-name`, by analogy with W-10. Origin: the
  Activity list, through `settings_origin`. Probably rare in practice (an
  unshared-main row normally implies a remote) but the code path exists. A test
  row of P4 should raise it through `choose_activity`.

No other miss. W-33's text is incomplete in one respect only: its routes are now
also raised from Activity S4 rows and popover rows (design Part D already says
this in prose).

### Step 3.6 03b's footer and popover against Part A (A2, A6 F6-F8, A7)

- `press` (`workspace/footer.rs:156-173`): returns for a dialog or any sheet
  (`:162-164`); Settings `:169`, "Checked for updates" `:170`, Needs attention
  -> `toggle_attention_popover` `:168` (`needs_attention.rs:81-109`, which sets
  the Activity menu, ⋯, repository menu and dock menu to `None` and focuses the
  popover list). A5 ("the footer cannot be pressed while any sheet is open")
  holds.
- `focus_after_hiding` (`D/view/footer.rs:273-282`): next shown control to the
  right (To share after Update check, then Needs attention, else the right dock
  group's first control, else Settings); `None` for other controls.
  `attention_restore_target` (`needs_attention.rs:59-76`) uses it and falls back
  to the Settings handle; it is `pub(in crate::desktop::view)`.
- `keep_focus_in_footer` (`workspace/footer_status.rs:184-208`, called from
  `workspace/footer.rs:46`, which render calls at `render.rs:198`, after
  `sync_sheet` at `:166`): **returns early while a dialog or any sheet is active
  (`:190-192`)**; otherwise, for each of Update check, To share, Needs attention
  that is hidden and holds focus, focuses the next control "now, not deferred".
  So after a Kit close the next render, with no sheet active, moves a hidden
  footer origin to E1's control: F7 and A3's "same render" claim hold. While
  Settings is open nothing keeps the footer item, which A3 expects.
- `focus_lost` (`workspace/focus.rs:285-325`): the modal branch (dialog or sheet:
  refocus the restore target, return) is first, `:287-292`; 03b's popover rule
  (`:301-312`: if the focused handle is one of the popover's three handles, go to
  the popover list while it is open, else `attention_restore_target`) is after it.
  So with Settings open the popover rule never runs, and after the close it runs
  only for focus left on the popover's handles.
- Popover close on a sheet: `attention_layer` (`needs_attention.rs:125-133`) sets
  `popover = None` when `modal_open` is true, at the render after the sheet
  opens, and moves no focus. The row menu (`activity_menu.rs:134-137`), ⋯
  (`activity_more.rs:105-108`), repository menu (`repository_menu.rs:307-310`) and
  dock menu (`footer.rs:270-273`) do the same. `press_attention`
  (`:253-283`), `show_activity_menu`, `press_activity` and the dock-menu opener
  refuse under `modal_open`.

Against the design:

- **A2 "an origin through `choose_activity`":** can be carried (3.4).
- **A2 case 3 / F8 (a Workspace layer's `restore`, else its context):** readable
  at `show_settings` time, before the next render drops the layer:
  popover -> `attention_restore_target(cx)` (footer item or E1's control);
  Activity row menu -> `activity.menu.restore` (Some only for a pointer menu,
  `activity_menu.rs:33-36`; `None` for `menu`/`shift-f10`, `:56`) else
  `context` (the list; `activity/menu.rs:40-43`, both `pub(crate)`);
  repository menu -> `restore`/`context` (`repositories/menu.rs:88-91`);
  dock menu -> `footer.menu.restore` is `pub`, but `DockMenu` keeps no toggle or
  context field (`footer/menu.rs:43-53`), so its "context" needs a new field or
  a derivation from the menu's panel kind. **The Activity ⋯ menu
  (`activity.more`) is a fifth layer F8 does not list; `ActivityMore` exposes
  neither `restore` nor `context` (`activity/more.rs:18-23`).** Task 2 must
  decide it (the focused handle, or add an accessor).
- **A6 F6 (popover row, footer item still shown):** `press_attention` focuses the
  footer item before `choose_activity` (`needs_attention.rs:278-280`), so the
  focused handle is already the origin. F7 as above.
- **A6 F5 (Activity row):** the origin is the list handle, but the focused handle
  at press time is the row's button, which can go; `settings_origin` is needed.
- **A7 item 7:** matches (`activity_menu.rs:33-36`, `repository_menu.rs:48-52`,
  `:307-310`). **A7 item 9** (no `on_next_frame` in tests) unchanged.
- The one popover-close timing fact the design should keep in view: the popover
  is dropped at the next render, not when the sheet opens, and silently. A
  focus that sat on the popover's handles is then lost; `show_settings` has
  already moved focus (A2 step 5), and the modal branch of `focus_lost` covers
  the remainder.

Nothing found means the contract cannot be carried by 03b's code.

### Step 4 Sweep (both commands run exactly as written, over `src/desktop`)

Sweep 1: 516 hits in 75 files. Sweep 2: 617 hits in 145 files (almost all
`publication` outside Settings: Activity copy, item header, model, services).
Only the Settings-relevant hits are classified.

#### Sweep 1 test sites against E-n

| Test site | E-n | Task |
| --- | --- | --- |
| `settings/mod_tests.rs` (`:11` import, `:73`, `:92`, `:108`, `:123-129`, `:137`, `:168`) | E-1 | 1; 5 (`:91-93`); 8 (`:123`) |
| `item/sheet_tests.rs:716-739` | E-2 | 1 |
| `item/frame_tests.rs:77`, `:165`, `:177` | E-3 | 1 |
| `drafts_routing_tests.rs:66-86` | E-4 (and E-16 `:86`) | 1; 5 |
| `keyboard_tests.rs:561-577`, `:704`; `keyboard_tests/settings.rs:22` | E-5 | 1; 5 |
| `settings/repositories_tests.rs:41`; `hosts_tests.rs:61`; `remotes_tests.rs:52`; `keys_tests.rs:52`; `polling_support.rs:104` | E-5 helpers | 5 |
| `keys_tests.rs:351`, `:833`/`:855`; `share_tests.rs:817`/`:834`; `create_tests.rs:930`; `hosts_tests.rs:275`; `remotes_tests.rs:769`/`:779` | E-6 | 1 |
| `operations/attention_tests.rs:411`, `:430-440` | E-7 | 1 |
| `settings/attention_tests.rs:83-86` | E-8 | 1 |
| `remotes_tests.rs:755-760`, `:851-855` (Choose rows) | E-9 | 3 |
| `keys_tests.rs:586`, `:639`, `:657`, `:681`, `:713`, `:734`, `:795`, `:919`, `:927`, `:935` (key-select/clear/unregister/delete ids) | E-10 | 4 |
| `repositories_tests.rs:146`, `:213`; `dock_tests/toolbars/repositories.rs:72`, `:76`; `repositories_tests/menu.rs:406-422`; `credential_tests.rs:128`; `outcome_canned_tests.rs:227`; `polling_journey.rs:266`, `:323`; `polling_privacy_tests.rs:964-1013`; `attention_tests.rs:249` | E-12 | 1 |
| `remotes_tests.rs:786-795`; `settings/repositories_tests.rs:660-678` | E-13 | 1; 3 |
| `polling_tests.rs:338-343`; `keyboard_tests/settings.rs:84` | E-14 | 3 |
| `keys_tests.rs` Generate/Import form tests (`key-generate-` 18 hits, `key-import-` 7); `keyboard_tests/settings.rs:107-111` | E-15 | 4 |
| `drafts_routing_tests.rs:86`, `polling_tests.rs:69`, `polling_privacy_tests.rs:965`, `hosts_tests.rs:278`, `remotes_tests.rs:795`, `keys_tests.rs:358`/`:361`, `settings/attention_tests.rs:84`, `polling_journey.rs:169-177` | E-16 | 5 |
| `settings/repositories_tests.rs:715-718` | E-17 | 6 |
| `footer_tests/mod.rs:332-347`; `footer_tests/status.rs:428-433`; `repositories_tests.rs:53`; `toolbars/repositories.rs:49`; `repositories_tests/menu.rs:408`, `:417`; `item/share_tests.rs:205`, `:330`, `:817`; `outcomes_route_tests.rs:59`, `:80`; `outcomes_tests.rs:462`; `credential_tests.rs:54`; `outcome_canned_tests.rs:274`; `operations/attention_tests.rs:126`, `:193`, `:411`; `keys_tests.rs:833`; `settings/repositories_tests.rs:698`; `repositories_manage_tests.rs:616`; `hosts_tests.rs:415`; `create_tests.rs:886`; `polling_tests.rs:308`; `settings/attention_tests.rs:34` | E-18 | 6 |
| `keys_tests.rs:10`, `:31` (the keymap binds `secondary-,`) | none | none needed |
| `activity_support.rs:358`, `needs_attention_tests/hidden.rs:118`, `repositories_tests/support.rs:427` (read `settings_shown`) | none | none needed (the accessor stays, R-3) |
| `Region::Content` hits in `keyboard_tests/{actions,conflict,discussion,drafts,editing,frame,lifecycle}.rs`, `dock_tests/{strip,moves}.rs`, `relationships_tests.rs`, `scroll_tests.rs:120`, `tabs_tests.rs`, `workspace_tests.rs`, `needs_attention_tests/closes.rs:138` | none | none; Task 2 module run (focus) |

#### Candidate E-19 onward

| E | Test | Why it breaks, or why it is low risk | Task |
| --- | --- | --- | --- |
| **E-19** | `needs_attention_tests/actions.rs` `holds` (`:122-149`, inside `popover_actions_by_keyboard`): `:139-147` | **Definite break (E-12's kind).** After `press_from_popover(CHOOSE_A_KEY)` opens Settings (`:139-140`) the next `press_from_popover` focuses the footer item and presses Enter (`actions.rs:47-100`); the footer press is refused behind the sheet (`workspace/footer.rs:162`), so "Enter on the item opens it" fails. Edit: Escape after each Settings-opening press (`:140`, `:142`, `:145`) | 1 |
| E-20 | `activity_tests/cases.rs` `requests` (`:231-237`, then `act` at `:277`, `:288`, `:316`, `:331`); `repositories_manage_tests.rs` `a_release_that_failed_during_removal_completes_once_retry_stores_the_text` (Settings opens `:898`, `act(RETRY)` `:917`) | E-12's kind but through the `choose_activity` seam (`activity_support.rs:324-334`), which bypasses the scrim and `modal_open`, so probably passes. The second test was a real click on the flush Retry at `ba95e1b7` (the design missed it). Escape before the outside action is the honest edit; run to see | 1 |
| E-21 | The 03b route tests not in E-18: `needs_attention_tests/actions.rs:140`, `:142`, `:145`; `needs_attention_tests/requests.rs` `shares` (`:126-129`), `routes` (`:303-331`); `needs_attention_tests/states.rs` `needs_fixing` (`:380-383`); `activity_tests/cases.rs:237`; `activity_tests/states.rs:172`; `keyboard_tests/shell.rs:298-303` (footer Enter, W-2) | Each reads `Shown::Settings(page)` only; they gain E-18's one assertion (the focused element id Part D names). W-34 (`needs_fixing`) lands on the entry; W-35 (`activity_tests/states.rs:172`) on `remote-add-name`; W-23 to W-25 (`actions.rs`) on the key list, `hosts-check`, `remote-add-name`; W-12/W-15 (`requests.rs`, `cases.rs`) likewise | 6 |
| E-22 | `settings/hosts_tests.rs:318`, `:348`, `:381`, `:652` read `confirm::PINNED_KEY` | Follow the constant when Task 7 rewrites it (see the unnamed production sites) | 7 |
| E-23 (check, not a known break) | `drafts_routing_tests.rs:88-101` (`the_drafts_key_empties_a_typed_passphrase`) sets the Generate inputs through `host.keys(window, cx).input(id)` | Reaches Task 4's hidden forms only if the inputs exist only while drawn; the inputs are page entities today. Check in Task 4 | 4 |

#### Tests that, after opening Settings, act outside it before closing (E-12's kind)

Named by the design (E-12, all Task 1): `repositories_tests.rs` `rows_and_list_states`
(`:146` opens, `:220` clicks a repo row); `dock_tests/toolbars/repositories.rs`
`repositories_toolbar` (`:69` opens, `:74` `secondary-n`); `repositories_tests/menu.rs`
`journey` (`:406`, `:415` open; `:415`, `:427` right-click rows); `lifecycle/credential_tests.rs`
`a_refused_key_offers_unlock_and_sharing_and_keys` (`:128` opens; `outcome_canned_tests.rs:227`
`reopen`); `keyboard_tests/shell/polling_journey.rs` `polling_keyboard` (`:168` opens;
Activity steps `:266-277`, `:323-331`); `polling_privacy_tests.rs` `polling_privacy`
(`:964` opens; `press_button` real clicks at `:997`, `:1013`); `operations/attention_tests.rs`
`attention_actions_by_reason` (`:249`, second iteration).
Also E-7 `attention_actions_hit` (`:411-440`, real clicks on `activity-button`) and
E-6/E-13 tests that already close Settings.
Not named by the design: E-19 (definite), E-20 (seam, probable pass).
Checked and not E-12: `settings/repositories_manage_tests.rs` removal tests
(`:388`, `:451`, `:477`, `:506`, `:739`, `:825`; all act inside Settings),
`shell/title_tests.rs:81`, `footer_tests/*`, `item/share_tests.rs:214`, `:338`,
`outcomes_*_tests`, `activity_tests/states.rs`, `needs_attention_tests/requests.rs`
(each uses a fresh window per case).

#### Production sites no task names

- `D/view/settings/attention.rs:11-33` (the host's `attend`: W-32; `ApproveHost`
  with no authority already uses `attend.root`) and `settings/polling_attention.rs:85`
  are in no task's file list, though A2 "Routes inside the sheet" and W-31/W-32
  change them -> **Task 6** (arrive routes), origin kept by Task 2.
- `D/view/confirm.rs:71` `PINNED_KEY` "Pinned key" (uses `:704`, `:972`) is the text
  of the Approve host and Replace host key confirmations. Part E rewrites only
  `hosts.rs:86` `PINNED` -> "Trusted key"; the dialog would still say "Pinned key"
  -> **Task 7** (E-22 tests follow the constant).
- Not Settings constants, so outside P18, but they carry the replaced terms into
  Activity row titles: `services/requests.rs:424-442` (`Select remote`,
  `Remove remote`, `Remove repository`, `Select key`, `Unregister key`, `Delete key`,
  `Approve host`, `Replace host key`) and `model/operations/recovery.rs:296-297`.
  For the owner to decide; no task names them.
- Named already: `workspace/content.rs`, `render.rs`, `focus.rs`, `workspace.rs`,
  `lifecycle.rs`, `requests.rs`, `footer.rs`, `repository_menu.rs`,
  `operations/attention.rs` (the plan's file map), `toolbar.rs`/`body.rs` (6, 8),
  `settings/{mod,keys,remotes,hosts,repositories,repositories/manage,polling}.rs`.
  `operations/retire.rs:171` and `lifecycle/actions.rs:148` need no edit.

#### Callers of the E-5 helpers

- `settings` (`settings/repositories_tests.rs:41`): 15 calls in
  `repositories_tests.rs` (`:220`, `:266`, `:315`, `:345`, `:388`, `:420`, `:445`,
  `:472`, `:500`, `:536`, `:612`, `:658`, `:725`, `:792`, `:824`) and 7 in
  `repositories_manage_tests.rs` (`:140`, `:179`, `:252`, `:296`, `:322`, `:348`,
  `:643`).
- `hosts` (`hosts_tests.rs:61`): 15 calls, same file.
- `remotes` (`remotes_tests.rs:52`): 18 calls, same file.
- `keys` (`keys_tests.rs:52`): 18 calls, same file.
- `Rig::open` (`polling_support.rs:104`): 7 callers: `attention_tests.rs:49`,
  `polling_tests.rs:65`, `:190`, `:313`, `:361`, `polling_real_tests.rs:63`, `:167`.
- `open_page` (`keyboard_tests/settings.rs:22`): 5 callers, `:57`, `:87`, `:110`,
  `:136`, `:171` (indices 0 to 4).
- `Walk::open_settings_and_each_page` (`keyboard_tests.rs:561`): 1 caller,
  `every_core_action_works_by_keyboard_alone`, `:704`.
- Direct `secondary-,` opens that bypass the helpers: `repositories_manage_tests.rs`
  (7), `shell/title_tests.rs:81`, `polling_privacy_tests.rs:964`,
  `settings/attention_tests.rs:83`, `hosts_tests.rs:277`, `drafts_routing_tests.rs:69`/`:85`,
  `item/frame_tests.rs:165`, `polling_journey.rs:168`.

#### Test files per task, for the module runs

Module paths are `desktop::view::` plus the path shown (resolved from the
`#[path]` declarations; run as `--bin manyhands <path>`).

**Task 1** touches: `workspace::settings_sheet::tests` (new); `settings::tests`
(`mod_tests.rs`, E-1); `item::sheet_tests` (E-2); `item::frame_tests` (E-3);
`drafts::routing_tests` (E-4); `keyboard_tests` (E-5 walk); `settings::keys::tests`,
`item::share::tests`, `create::tests`, `settings::hosts::tests`,
`settings::remotes::tests` (E-6; remotes also E-13); `operations::attention_tests`
(E-7, E-12); `settings::attention_tests` (E-8); `repositories::tests`,
`dock::tests::toolbars::repositories`, `repositories::tests::menu`,
`lifecycle::credential_tests`, `lifecycle::outcome_canned_tests`,
`keyboard_tests::shell::polling_journey`, `polling_privacy_tests` (E-12);
`settings::repositories::tests` (E-13); new: `needs_attention::tests::actions` (E-19),
`activity::tests::cases` and `settings::repositories::manage_tests` (E-20).
Reached by the sweep (they open Settings; not edited): `keyboard_tests::settings`,
`keyboard_tests::shell`, `footer::tests`, `footer::tests::status`,
`operations::outcomes::tests`, `operations::outcomes::route_tests`,
`settings::polling_tests`, `settings::polling_real_tests`, `shell::title::tests`,
`needs_attention::tests::{requests,states,hidden}`, `activity::tests::state_cases`.

**Task 2** touches `workspace::settings_sheet::tests` (P5, P22, P23). Reruns P3, P6,
P14. Reaches (plan: E-5 walk, E-18 files, `keys_tests`, `repositories_manage_tests`,
`create_tests`) plus the focus tests the sweep found (`focus_lost`, `focus_content`,
`landed_refocus`, `apply_review`): `keyboard_tests` (walk), `footer::tests`,
`footer::tests::status`, `repositories::tests`, `repositories::tests::menu`,
`repositories::tests::toolbar`, `operations::attention_tests`,
`operations::outcomes::{tests,route_tests}`, `lifecycle::credential_tests`,
`lifecycle::outcome_canned_tests`, `settings::keys::tests`,
`settings::repositories::manage_tests`, `create::tests`, `item::share::tests`,
`settings::hosts::tests`, `settings::repositories::tests`, `workspace::tests`,
`tabs::tests`, `needs_attention::tests` (all submodules: actions, closes, hidden,
open, requests, stale, states), `activity::tests` (focus and the rest),
`drafts::routing_tests`.

**Task 3** touches `settings::remotes::tests` (E-9, E-13), `settings::polling_tests`
(E-14), `keyboard_tests::settings` (E-14); reaches `settings::repositories::tests`,
`settings::repositories::manage_tests`, `settings::hosts::tests` (Check host reads
the chosen repository), `settings::attention_tests` (`set_up_sharing` selects a
root), `shell::title::tests` (manage and remove), `settings::polling_real_tests`.

**Task 4** touches `settings::keys::tests` (E-10, E-15), `keyboard_tests::settings`
(E-15); reaches `settings::hosts::tests` (Trusted servers move under Sharing &
keys), `settings::tests` (`forget_passphrase`), `drafts::routing_tests` (E-23),
`settings::attention_tests` (Keys route).

**Task 5** touches every E-5 helper file and E-16 site: `settings::repositories::tests`,
`settings::repositories::manage_tests`, `settings::hosts::tests`,
`settings::remotes::tests`, `settings::keys::tests`, `settings::polling_tests`,
`settings::polling_real_tests`, `settings::attention_tests`, `keyboard_tests`,
`keyboard_tests::settings`, `keyboard_tests::shell::polling_journey`,
`drafts::routing_tests`, `polling_privacy_tests`, `settings::tests` (E-1 `:91-93`);
reruns the P tests of Tasks 1-4.

**Task 6** touches `workspace::settings_sheet::tests` (P4, P20, P2); E-17
(`settings::repositories::tests`); E-18 files (`footer::tests`, `footer::tests::status`,
`repositories::tests`, `dock::tests::toolbars::repositories`, `repositories::tests::menu`,
`item::share::tests`, `operations::outcomes::route_tests`, `operations::outcomes::tests`,
`lifecycle::credential_tests`, `lifecycle::outcome_canned_tests`,
`operations::attention_tests`, `settings::keys::tests`,
`settings::repositories::manage_tests`, `settings::hosts::tests`, `create::tests`,
`settings::polling_tests`, `settings::attention_tests`); new E-21 files
(`needs_attention::tests::{actions,requests,states}`, `activity::tests::cases`,
`activity::tests::state_cases`, `keyboard_tests::shell`).

#### Did 03b move or change any E-n citation

Moved only, meaning unchanged: E-5 (`remotes` `:47`->`:52`, `settings` `:40`->`:41`,
`keys` `:51`->`:52`); E-6 (`keys_tests` `:350`->`:351`, `:832`->`:833`; `share_tests`
`:754`/`:771`->`:817`/`:834`; `remotes_tests` `:731`->`:769`); E-7 (`:367`->`:411`;
the row clicks are still real pointer clicks, now on `activity-button`); E-12 menu
journey `+13`; E-13 (`+38`, `+10`); E-16 (`polling_privacy` `:825`->`:965`, `remotes`
`:757`->`:795`, `keys` `:357`/`:360`->`:358`/`:361`, `polling_journey`
`:152-160`->`:169-177`); E-17 `:705-708`->`:715-718`; E-18 (all, see the design row).
Unchanged files: E-1, E-2, E-3, E-4, E-8, E-9 (`remotes_tests` is changed but the Choose
clicks at `:760`, `:855` are intact), E-10, E-14, E-15 lines.
**Changed meaning:** E-12 `outcome_canned_tests.rs:223` (a click on the entry's Open…
is now the `choose_activity` seam, `:227`); `polling_journey.rs:247`/`:305`
(`secondary-4` keys are gone; Activity is reached through `walk.region` and
`to_row`, `:266`, `:323`); `polling_privacy_tests.rs:846`/`:872` (the old
`attention-action` click is now `press_button`, a real click on `activity-button`,
`:997`, `:1013`: still breaks behind the scrim); `attention_tests`
`attention_actions_by_reason` `:224-225` (the second loop iteration's `act` now runs
through the seam). The design's E-12/E-6/E-7/E-16/E-17/E-18 rows were updated to the
new lines in `9f89a3bb`; the E-12 row describes the seam sites.

### The further test edits (decision 7), assigned

E-19 to E-23 are in "Candidate E-19 onward" above. E-19 is a definite break
and E-20 probably passes; both belong to Task 1. E-21 is Task 6, E-22 Task 7,
and E-23 a check in Task 4. They are recorded as edits only when made, with
the old and new assertion.

### Ticket comment, Task 0

Posted after this commit: see the next commit.
