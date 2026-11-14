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
| 0 | done | merge `82df6942`; design citations `9f89a3bb`; ledger `003eb518`; ticket comment `e54fe9b5` | yes |
| 1 | done, review approved first time (Minors parked) | code `0d5501e7` (base `e54fe9b5`); ledger `023ebf47` | yes |
| 2 | done, design checkpoint passed, review approved after fix round 1 | code `5cddad28` (base `023ebf47`), fix `6894d70d`; ledger commit follows | with the ledger commit |

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

Comment `01M7D6V5B8M8DW7JXXXAF7T8R1`, published with the branch at `e54fe9b5`.

## Task 1: the swap (A1, A2 without origins, A3, B1 with five entries, B2, C3, Part F)

Implementer: one fresh Opus agent. Base `e54fe9b5`, code commit `0d5501e7`
(the trailer was amended from Opus 5.5 to Sonnet 5.5 before review, unpushed;
Ruling 1). Reviewer: one fresh Opus agent, spec compliant, quality approved,
no Critical or Important findings. Boundary diff prints nothing. 38 files
changed.

### Rulings (Task 1)

Ruling 12: **the origin lives on `SettingsHost`** (`set_origin`, `take_origin`,
a weak handle) in Task 1. `workspace.rs` may gain only `settings_origin` and
`settings_return`, which are Task 2's and mean something else. Task 2 may move
it — cost if wrong: Task 2 moves a field.

Ruling 13: **A2 step 3 (Settings already open) is done now, in its smallest
form**: a route raised while Settings is open arrives in place, focuses its
target or the entry, and keeps the origin. Without it a re-open would close
Settings itself and record an origin inside the sheet. The selection is set
before the already-open branch so that `Hosts(root)` arrives on its root.
Consequence: Task 2's test for step 3 will pass on its first run, so its RED
will not be seen; Task 2's tests for step 3 are kept as regression rows —
cost if wrong: one test never seen red.

Ruling 14: **R-39 (fields fill the body, up to 480 px) is taken early from
Task 4.** B1's frame makes the body about 477 px wide, and today's fixed 480 px
fields pushed Browse past the window (`browse_fills_the_field_and_a_dismissal_leaves_it_unchanged`).
A shared `settings::field_column()` (`w_full().max_w(480)`), net 0 lines in
`repositories.rs` and `remotes.rs`. Task 4 keeps its other R-39 work (the key
list's fields) — cost if wrong: none; it was needed.

Ruling 15: **`in_window` (`workspace/requests.rs`) was a latent production
bug**, named by no task. It updated the workspace through
`WindowHandle<Root>::update`, which leases Root, while `show_settings` reads
and updates Root (`has_active_dialog`, `open_sheet`): every item route
(Share…, posting, the composer) would have panicked. It now updates through
`AnyWindowHandle`, which passes the root view without leasing it. Its only
caller is `item_route`. Accepted — cost if wrong: none.

Ruling 16: the page methods' signatures changed as the brief directs
(`manage_root` loses `focus: bool` and returns the name field's handle;
`ask_again` and `HostsPage::check_host` return `Option<FocusHandle>`),
touching `keys/commands.rs`, `keys/render.rs`, `hosts.rs` and
`repositories/manage.rs`, which the plan's file list omits. The wrappers stay
where the entry functions were (`content.rs`, `requests.rs`). `open_settings_on`
keeps its signature. `SettingsRoute::AddRepository` and `::Keys` carry
`#[allow(dead_code)] // Used from Task 6`; `Workspace::settings_chosen` carries
`// Used from Task 3`.

Ruling 17: **F14 is read** as the focused element's key context containing
"Dialog", after two draws, with the EditsNotKept dialog open and no other (the
base dialog host's element is not in the snapshots). It uses a symlink for
"recovery off", so **the EditsNotKept row is unix-only**; the Windows native
target skips it. Recorded for the Task 9 summary and for native CI after merge
— cost if wrong: a Windows row missing until native CI.

Ruling 18: P7's RED was its owner precondition; no action row was seen to fail
before the swap. The rows are discriminating (before the swap no modal was
open, so each action would have moved focus or layout) but that evidence is
inferred — cost if wrong: a P7 row that cannot fail.

Ruling 19: **the stale module doc of `settings/mod.rs`** (D1's R-B4 paragraph,
"shown in the main window's content region in place of the tabs") stays until
Task 8 Step 2, which owns its rewrite.

### Parked review Minors (Task 1)

1. `settings_sheet_tests.rs` is 950 lines. Before Task 2, its helpers
   (`:60-368`, about 310 lines) move into `settings_sheet_tests/support.rs`;
   the tests stay at `workspace::settings_sheet::tests::<name>`. Done in Task
   2's dispatch as its first step, as a pure move.
2. P7's RED (Ruling 18) and F14's unix-only setup (Ruling 17), above.
3. Two test names now say the opposite of their bodies:
   `drafts_routing_tests.rs` `back_to_tabs_from_settings_over_drafts_shows_the_tabs`
   (now asserts Drafts shown and the tab not) and
   `the_drafts_key_empties_a_typed_passphrase` (Escape now does the
   emptying). E-4 does not authorize renames; **a rename is parked for Task 8**.
4. E-2's removed `assert_eq!(settings, Shown::Tabs)` was still true for every
   remaining key (a free check that no workspace key opens Settings behind the
   details sheet). Optional to restore; parked.
5. The F-rows read after a draw in Task 1: F3 (P3), F11 (P14), F12, F13, F14 (P6).

### Tests that ran (Task 1)

- RED, eight single-test runs (`0 passed; 1 failed` each): P1
  `settings_is_a_sheet_the_workspace_keeps`, P2 `settings_leaves_the_item_as_it_was`,
  P3 `settings_replaces_the_details_sheet_and_keeps_its_edits`, P6
  `settings_closes_every_way`, P7 `nothing_acts_behind_settings`, P8
  `settings_scrolls_on_its_own`, P14
  `show_in_activity_closes_settings_and_focuses_activity`, P15
  `settings_does_not_open_behind_a_dialog_or_while_quitting`, all under
  `desktop::view::workspace::settings_sheet::tests::`. GREEN: each alone,
  `1 passed`.
- The 25 E-tests by exact name, one command: 24 passed, 1 failed
  (`desktop::view::item::share::tests::share_opens_remotes_on_its_own_repository_after_another`,
  "cannot read Root while it is already being updated", the `in_window` fix,
  Ruling 15); rerun alone, 1 passed. Both E-20 tests passed again after their
  edit (2 passed).
- Module run 1 (once, every touched file plus Task 0's Task 1 list): 305
  tests, 303 passed, 2 failed, 380.81 s. Failures:
  `settings::repositories::tests::browse_fills_the_field_and_a_dismissal_leaves_it_unchanged`
  (fixed by Ruling 14) and
  `repositories::tests::the_more_menu_acts_on_the_chosen_repository` (the
  harness's 180 s wait exceeded under parallel load; passed alone twice at
  2.35 s and 2.27 s; the reviewer found nothing in the diff that reaches it).
- Module run 2 (only what changed after run 1): 101 passed, 0 failed, 62.00 s.
- `cargo fmt --check`, `cargo check --locked --features desktop`, `cargo clippy
  --all-targets --all-features --locked -- -D warnings`: exit 0.
- The binary has 1920 tests: 1914 before, less 2 (E-1), plus 8.

### The test edits (Task 1)

Removals are in bold. Numbers E-21 to E-23 are Task 6, 7 and 4's (Task 0);
further edits found by tasks are numbered **E-24 onward** (the Task 1
implementer's two are E-24 and E-25).

Each entry gives the E-number, the test, the old assertion, the new
assertion and the reason. **Removals are in bold.**

- **E-1**, `settings/mod_tests.rs`:
  - `the_settings_key_shows_the_settings_and_back_keeps_the_tabs` (`:73`):
    - **Removes** `assert!(!shell.present(("tab", 0usize), cx))`: the strip
      stays drawn under the sheet.
    - **Removes** the `label("settings-close") == BACK_TO_TABS` assertion.
    - `shell.click("settings-close")` becomes `shell.keys("escape")`.
    - The `settings-page-shown` check stays until Task 5.
    - The doc comment was rewritten, and the `BACK_TO_TABS` import dropped.
  - **Removes the test** `opening_an_item_shows_the_tabs_again` (`:137`).
  - **Removes the test** `focusing_the_content_region_focuses_the_settings`
    (`:168`).
  - The module doc was rewritten for the sheet.
- **E-2**, `item/sheet_tests.rs` `no_workspace_key_acts_behind_the_sheet`:
  - **Removes** `"secondary-,"` from the refused keys.
  - **Removes** the `workspace.settings_shown(cx)` read and
    `assert_eq!(settings, Shown::Tabs, "{key}")`. P3 asserts the new
    behaviour.
  - The doc comment was updated, and the unused `Shown` import dropped.
- **E-3**, `item/frame_tests.rs`
  `page_keys_scroll_the_items_region_and_settings_keep_theirs`:
  - The `column_offset` reads for Settings became `settings_offset` (a new
    helper, `workspace.settings_offset(cx)`).
  - Added `assert_eq!(column_offset, 0, "the column did not move")`, which
    is the design's "not `content_offset`".
  - `click("settings-close")` became Escape.
  - The doc comments were updated.
  - The window change is **E-24**.
- **E-4**, `drafts_routing_tests.rs`:
  - `back_to_tabs_from_settings_over_drafts_shows_the_tabs` (`:66`):
    - **Removes** `assert!(!drafts_shown)`, `assert!(!present("drafts"))`
      (after Ctrl+,), `assert!(!drafts_shown)` (after the close) and
      `assert!(present(("tab", 1)))`.
    - They are replaced by their inversions: `drafts_shown`,
      `present("drafts")`, and after Escape `drafts_shown` and
      `!present(("tab", 1))` (R-31).
    - The close button became Escape.
  - `the_drafts_key_empties_a_typed_passphrase` (`:83`):
    - Escape now closes Settings before the drafts key.
    - Added `assert_eq!(settings_shown, Shown::Tabs)` after the Escape.
    - **Removed** the extra `shell.frame` and its comment ("The next frame
      of the content region empties them").
    - The emptiness assertions are unchanged.
  - The module doc was updated.
- **E-5's walk**, `keyboard_tests.rs` `Walk::open_settings_and_each_page`:
  - The `tab_to("shift-tab", "settings-close")`,
    `on("settings-close", Content)` and `enter` steps were replaced by
    `keys("escape")`.
  - **Removes** `on(("tab", shown), Content)`. It is replaced by
    `on_handle(&origin, Content)`, where `origin` is the handle focused
    before Ctrl+,.
  - The doc comment was updated.
  - The page helpers keep today's indices.
- **E-6**, Escape instead of the close button:
  - `keys_tests.rs`, both sites;
  - `item/share_tests.rs`;
  - `create_tests.rs` (`settings-close-button`);
  - `hosts_tests.rs`;
  - `remotes_tests.rs`, inside the E-13 test.
- **E-7**, `operations/attention_tests.rs` `attention_actions_hit`:
  `rig.shell.keys("escape")` after each of the three Settings-opening
  presses, with a comment.
- **E-8**, `settings/attention_tests.rs`
  `settings_polling_attention_actions`: added an assertion after Ctrl+,
  that `settings_shown == Settings(Keys)`, the page Choose a key… left.
  The comment was updated.
- **E-12**, Escape before the next action outside Settings:
  - `repositories_tests.rs` `rows_and_list_states`, after the
    Settings(Repositories) assertion;
  - `dock_tests/toolbars/repositories.rs`, after the
    `Page::Settings(Repositories)` assertion;
  - `repositories_tests/menu.rs`, after the Remotes assertions and after
    the Settings… on `b` assertions;
  - `lifecycle/credential_tests.rs`, after
    `matches!(shown, Settings(Keys))` in row 0;
  - `keyboard_tests/shell/polling_journey.rs`, before the Activity steps
    (`:266`); `:323` follows with Settings already closed;
  - `polling_privacy_tests.rs`, after
    `seen.surfaces("settings row") > 0`, before `press_button`;
  - `operations/attention_tests.rs` `attention_actions_by_reason`, after
    each `assert_eq!(shown, page)` in the loop. The second iteration's
    Escape is harmless.
- **E-13, first half**:
  - `remotes_tests.rs`
    `opening_remotes_on_a_repository_shows_that_repository_even_after_another`:
    - `host.open_remotes_on(b)` is replaced by
      `workspace.show_settings(SettingsRoute::Remotes(b), None)`;
    - the close is now Escape;
    - "Settings keeps b" is still asserted.
  - `settings/repositories_tests.rs`
    `a_form_control_below_the_fold_is_scrolled_into_view`:
    - `find("content").bounds()` became `find("settings-scroll").bounds()`;
    - `content_offset()` became `settings_offset(cx)`;
    - the doc was updated.
  - The window change is **E-25**.
- **E-19**, `needs_attention_tests/actions.rs` `holds`: `shell.keys("escape")`
  after the CHOOSE_A_KEY, APPROVE_HOST and OPEN_REMOTE_SETTINGS assertions.
- **E-20**: both tests passed unedited (the seam bypasses `modal_open`). The
  ledger's honest edit was made anyway:
  - `activity_tests/cases.rs` `requests`: Escape after the Settings(Keys)
    assertion;
  - `repositories_manage_tests.rs`
    `a_release_that_failed_during_removal_completes_once_retry_stores_the_text`:
    Escape before `act(RETRY)`, plus an added
    `assert_eq!(settings_shown, Shown::Tabs)`, so the Escape cannot silently
    miss.
- **E-24 (new)**, `item/frame_tests.rs`
  `page_keys_scroll_the_items_region_and_settings_keep_theirs`:
  - `shell(SHORT_WINDOW, …)` became `shell(SETTINGS_WINDOW, …)`, with the
    new const `SETTINGS_WINDOW = 400.`.
  - Added `shell.set_activity_height(SETTINGS_ACTIVITY = 100.)`.
  - Reason: the sheet is as tall as the window. At 700 its body (about
    927 px) holds the whole Repositories page (769 px), so Settings'
    PageDown cannot move. Activity is shortened so the item area keeps a
    region to page.
  - No assertion changed.
- **E-25 (new)**, `settings/repositories_tests.rs`
  `a_form_control_below_the_fold_is_scrolled_into_view`: the window height
  went from 640 to 360. Same reason: Create repository must start below the
  sheet body's fold. No assertion changed.

Scripted edits: the function lists of all 22 edited existing test files
were compared before (from `HEAD`) and after. The only differences are the
two E-1 removals (`opening_an_item_shows_the_tabs_again` and
`focusing_the_content_region_focuses_the_settings`) and the new helper
`settings_offset` in `frame_tests.rs`. Test-attribute counts per file are
unchanged, except `settings/mod_tests.rs`, which went from 5 to 3. The
binary has 1920 tests: 1914 before, less 2, plus 8.

- **E-21, E-22 and E-23** are untouched (Tasks 6, 7 and 4).

## Task 2: where focus goes (A2's origin, steps 2-3 and 5, A3, A4, A5; design checkpoint on A6)

Implementer: one fresh Opus agent, two phases. Phase A wrote A6's rows as
failing tests before any focus code (the design checkpoint). Phase B built the
mechanism. Base `023ebf47`, code `5cddad28`, fix round 1 `6894d70d`. Reviewer:
one fresh Opus agent, spec ❌ with one Important finding; scoped re-review
(Sonnet) of the fix: addressed, no new Critical or Important breakage.
Boundary diff prints nothing. No existing test was edited (no E-26).

### The design checkpoint (owner's gate), decision

All 19 A6 rows were written as tests first and read after a draw. Nothing was
foreseen unmeetable, and no row was edited or weakened. **One part could not
be expressed against today's code: F2's "a field not drawn".** No route that
exists now names a field the page does not draw in the next frame; A6's Test
column gives F2 to P4 (Task 6, with R-22's key list while the keys are still
being read, the plan's Review Focus for Task 6). It is a documented deferral,
not a stop: **Task 6's P4 must carry an F2 "field not drawn" row**, and the
review-ready summary tells the owner so. Two sub-rows cannot fail after a draw
by their nature (the P5 save-banner F9 row; the P22 Hosts F17 after-draw
read), and carry comments saying so. A discriminating F9 row was added (the
tickets list gone after a removal in Settings) and stays.

### Rulings (Task 2)

Ruling 20: R-7 ("the close is detected in `sync_sheet`, not by `on_close`") is
a mechanism sketch and loses to A3 rev 4 and A7.10 ("must survive"): the
close runs `settings_closed` from the sheet's `on_close` at once (Escape, the
close button, the scrim), and `sync_sheet` is the backstop for a direct
`window.close_sheet` — cost if wrong: none; the owner guard keeps the double run
idempotent (reviewer verified).

Ruling 21 (accepted at the checkpoint): `settings_return` is cleared on the
second render after it is armed, not "by the render after the close" (a render
runs before `focus_lost` in a draw, `window.rs:3304` against `:3363`). Nothing
uses `on_next_frame` (A7.9).

Ruling 22: **Step 5's "focus found outside the sheet goes to the page's entry"
is a post-frame check with positive containment**, not a render-time check and
not the focus-out listener Ruling 21's plan named. GPUI empties the focus-event
paths while the window is inactive (`gpui-pre window.rs:3354-3393`), so a
listener never fires in the test windows and would miss a landing while the
app is in the background. The check is deferred from render with `cx.defer_in`
(`render.rs:174-182`, the dock clamp's pattern), so it reads the frame just
drawn. It moves focus only when it positively finds it behind the sheet (in a
region outside the layer, a toolbar button or a footer control), never because
focus is merely "not in the sheet" (A7.1); Root's overlay menu and Kit popups
are in none of those (A7.2, A7.3) — cost if wrong: a landing onto a control that
is in no region would stay behind the sheet.

Ruling 23: a menu that was already chosen returns focus to its context, not to
its pointer `restore` (the repository menu's Settings… by pointer would
otherwise return to wherever focus was before the right-click). A refinement
of A2 case 3, consistent with F8; the reviewer agreed.

Ruling 24: Drafts' `apply_refocus` drops its refocus while any sheet is
active, as the Workspace's `landed_refocus` does (fix round 1). The details
sheet is covered by the same test.

### F1 to F19, as the implementer wrote them (phase A)

## 3. The F1 to F19 table

| F | Row(s), test | Verdict | Read after a draw | Now, and why |
| --- | --- | --- | --- | --- |
| F1 | Ruling 3 test: `Identity(root)` → `identity-name`, `CheckHost(root, a)` → `hosts-check`, `RetryGeneration(id)` → `key-generate-passphrase` | EXPRESSED | `holds(target)` after two draws, route raised through `show_settings` | Passes: no render-time rule yet. This guards A7 item 1 for Step 5. Task 6 folds it into P4. |
| F2 | Ruling 3 test: `Page(Repositories)`, `Page(Polling)`, `Manage(root)` → `("settings-page", ix)` | EXPRESSED for "a route naming no field". **CANNOT BE EXPRESSED now** for "a field not drawn". | as F1 | Passes. No route that exists now names a field the page does not draw in the next frame: the identity form draws whenever a root is managed (`D/view/settings/repositories/manage.rs:559-590`), Check host always draws once a root is chosen (`hosts.rs:729-737`), and the Generate form always draws today. P4 (Task 6) owns it, with R-22's key list while it is still being read. Rows are limited to routes whose target Task 6 does not change. |
| F3 | P3 `settings_replaces_the_details_sheet_and_keeps_its_edits` (Task 1) | EXPRESSED (Task 1) | `holds` after two draws | Passes. Rerun after the mechanism. |
| F4 | P5: footer item; ticket list row; item header's Share… (Tab, Enter; W-11); lifecycle dialog's Open Sharing & keys (its origin, Activity's list); W-31 inside the sheet (the first origin) | EXPRESSED | `holds_handle` / `holds` after Escape and two draws | All pass: the origin is the focused handle, and the Kit gives it back. |
| F5 | P5: by its button (focused, Enter); by its row menu; by Enter; the row dismissed while open (cursor rule) | EXPRESSED | list `holds_handle` and `highlighted` after two draws | **By button: RED.** Focus goes back to `("activity-button", 1)`, because today's origin is the focused handle, the button (`activity_panel.rs:126-143`), and nothing sets `settings_origin` (R-43). The other three pass. |
| F6 | P5: a popover row, the item still shown | EXPRESSED | `holds_handle(item)` | Passes (`press_attention` focuses the item first, `needs_attention.rs:278-281`). |
| F7 | P5: a popover row, the row dismissed while open so the item hides | EXPRESSED | `holds_handle(attention_restore_target)` | Passes (`keep_focus_in_footer`, `footer_status.rs:184-208`). |
| F8 | P5: Ctrl+, in the popover; in a row menu by keyboard; in a repository menu by keyboard; in a dock menu by keyboard; in ⋯ (Ruling 8) | EXPRESSED | `holds_handle(restore, else context)` | Popover passes (`focus_lost`'s popover rule, `focus.rs:301-312`). **The four Kit menus are RED**: content region. The origin today is the menu's own PopupMenu handle. The layer drops at the next render (`activity_menu.rs:134-137`, `repository_menu.rs:307-310`, `footer.rs:270-273`, `activity_more.rs:105-108`), so both weak records, the Kit's (`K/root.rs:379-423`) and the host's (`settings/mod.rs:225-233`), fail to upgrade, and `settings_closed` falls to `Region::Content` (`settings_sheet.rs:122-125`). "If that is gone, F9" has no row of its own. |
| F9 | P5: the save banner's Open Settings (W-30), its tab closed while open; and **(added)** a ticket list row whose list goes when the repository is removed inside Settings | EXPRESSED | `region()` after two draws | Banner passes, and **cannot fail**: its region is Content, where every fallback lands anyway. The added row is **RED**: Content, not Tickets, because `settings_return` is missing and `focus_lost`'s target comes from the closed sheet. See issue (c) for the banner's wording. |
| F10 | P5: route raised with nothing focused (`blur` in the same update) | EXPRESSED | `region()` | Passes. |
| F11 | P14 and P6 (Task 1) | EXPRESSED (Task 1) | `holds_handle` | Passes. |
| F12, F13 | P6 (Task 1) | EXPRESSED (Task 1) | `holds` / `holds_handle` | Pass. |
| F14 | P6 (Task 1; unix only, Ruling 17) | EXPRESSED (Task 1) | `in_context("Dialog")` | Passes. |
| F15 | P23: Remove repository…'s confirmation closed by Escape; reopened, and Remove gone while it is open (a fabricated `RepoRemove` landing); a page's dialog within 0.25 s of Drafts' deferred close | EXPRESSED | `holds_handle` plus `in_context("Sheet")`; `holds(entry)` | Basic passes. **Origin gone is RED**: nothing is focused. `focus_lost`'s modal branch has no target and does nothing (`focus.rs:287-292`; A7 item 6). Within 0.25 s passes: the app's confirmation focuses its own origin on close (`confirm.rs:255`), which hides A7 item 4's pending-handle record. |
| F16 | P23: Drafts' Discard… by Enter, a draw, Ctrl+,, clock +250 ms (`advance_clock`, no real wait), draws; and the restore firing after a page's dialog | EXPRESSED | `holds(entry)` and not `holds(drafts row)` | **Both RED**: the Kit's restore (`K/root.rs:341-363`) puts focus on the Drafts row behind the sheet. |
| F17 | P22: Add pressed with its check held, Escape, the check lands; the Hosts `refocus_gone` landing with the page closed and focus empty; reopen rows. Key removal is P24's (Task 4). | EXPRESSED (Add row). The Hosts sub-row **cannot fail after a draw**. | `has_dialog` plus `holds_handle(origin)`; Hosts: in-update fact, then a read after two draws | **Add: RED**. A confirmation opens after the close and takes focus (`repositories.rs:420-428`, `:461-479`; no open gate). Reopen rows pass. **Hosts: the handler's own focus call is RED** (it focuses Check host in the closed sheet, `hosts.rs:576-582`). That read is in-update, so it is not an A6 proof (Testing allows it for the update's own call). The read after two draws passes and cannot fail: see issue (b). |
| F18 | P23: `open_created` landing; a review landing (`draft_discarded`) | EXPRESSED | `holds("add-path")` after two draws | **Both RED**: focus goes to `("tab", 0)` behind the sheet (`content.rs:131-133`, `:162-164`, `:590-602`; R-34 missing). |
| F19 | P23: interval select by Tab and Enter, then Escape; Root's context menu over Add folder by right-click, then Escape (Linux fallback overlay; not on macOS or Windows) | EXPRESSED | `in_context("List")` plus `"Sheet"`; `in_context("PopupMenu")`; `holds` afterwards | All pass. Diagnostic run: the select's list is `[Root > Workspace > Content > Sheet > Select > List]`, inside the sheet. The context menu is `[Root > PopupMenu]`, region None: outside the Workspace (A7 item 2 confirmed). |

Also in P22, for A2 step 2: a route raised in the same update as a direct
Kit `close_sheet`. **The passphrase is not emptied: RED.**
`show_settings` does not run a quiet `settings_closed` when the owner is
Settings but no sheet is active (`settings_sheet.rs:68-79`), and `shown`
stays Keys, so `forget_passphrase` keeps the text (`settings/mod.rs:207-213`).
The new origin is recorded, and that row passes.

Summary:

- All 19 rows are expressed (F3 and F11 to F14 by Task 1's tests).
- One part cannot be expressed now: F2's "field not drawn", which goes to
  Task 6.
- Two sub-rows are written but cannot fail after a draw: the P5 banner
  (F9) and the P22 Hosts sub-row (F17). The added F9 row and the Add row
  discriminate instead.
- No row is foreseen as unmeetable.

### A7, how each item is met

| A7 | How it is met | Code |
| --- | --- | --- |
| 1. Focus reads lag a frame | No check treats "not in the sheet in the last frame" as outside. The render check moves only focus it positively finds behind the sheet in the frame drawn last: in a region (outside the layer), a toolbar button or a footer control. A route's target drawn for the first time this frame is in none of them, so it is left alone. The check runs AFTER the frame, deferred from render with `cx.defer_in` (fix round 1), so it reads the frame just drawn. The F1/F2 test guards this. | `settings_sheet.rs:293-305`, `:316-325`; `render.rs:174-182` |
| 2. Root's overlays sit outside the Workspace | Root's fallback menu (`[Root > PopupMenu]`) is in no region, toolbar button or footer control, so it is left there. | `settings_sheet.rs:293-305` |
| 3. Kit popups move focus | A popup in the sheet is a deferred child inside the tracked layer (`[… Sheet > Select > List]`), so `drawn_behind` is false. Focus is never taken from the select's trigger while its list is open. | `render.rs:121-130`; `settings_sheet.rs:296` |
| 4. The Kit's deferred restore | A restore onto a control drawn behind the sheet is moved to the entry at the next render (F16). A restore onto an undrawn handle goes through `focus_lost`'s Settings rule: the restore target when the layer holds it (usually the Kit sheet host, which carries the `Sheet` key context and the focus trap), else the entry (corrected by the Task 2 review). A dialog opened within 0.25 s gets back its own origin from the app's confirmation (`D/view/confirm.rs:255`). | `settings_sheet.rs:316-325`, `:328-345` |
| 5. `close_sheet` and `close_all_dialogs` move focus | `settings_closed` closes the Kit sheet before any focus move or dialog, quiet closes included. The owner is never cleared while the Kit's Settings sheet stays active. Quit paths keep "dialogs, then Settings quietly" (R-52, Task 1). | `settings_sheet.rs:251-258` |
| 6. `focus_lost` may have no target | While Settings is open and no dialog is: no target, or a target outside the layer, focuses the page's entry (F15 "origin gone"). | `settings_sheet.rs:328-345`; `focus.rs:287-290` |
| 7. A layer's `restore` may be empty | Restore, else context. A chosen menu gives its context. The ⋯ menu and dock menu gained the context fields they lacked (Ruling 8). The layer is read and closed inside `show_settings`. | `settings_sheet.rs:88-90`, `:205-222`; `activity/more.rs:24`; `footer/menu.rs:57` |
| 8. `region_of` defaults to Content | `origin_region` returns `None` outside every region and toolbar button. It is read at the open, when the origin is drawn. A footer origin then leaves the decision to `keep_focus_in_footer` (F7). | `settings_sheet.rs:166-168`, `:227-238` |
| 9. `on_next_frame` never runs in tests | `settings_return` is disarmed by renders (the second after arming) and taken by `focus_lost`. Test frames drive both, and nothing uses `on_next_frame`. | `settings_sheet.rs:70-82`; `render.rs:168`; `focus.rs:297-301` |
| 10. The `open` flag is only as fresh as the close | It is cleared by `settings_closed`, which runs at once in `on_close` (Escape, close button, scrim). `OpenFlag::is_open` also requires an active sheet, so a direct Kit close without `on_close` counts as closed at once. | `settings_sheet.rs:179-191`, `:259-263`; `settings/mod.rs:96-100` |

### Where the code departs from the sketch (A2-A4, R-43 to R-47)

## 4. Where the code departs from the sketch, and why

1. **Step 5's check: phase A planned a listener; the code has a post-frame
   check (positive containment). This departs from Ruling 21's accepted
   plan (Ruling 22).** (First built as a check inside `render`, which the
   review found misses a move made inside a child render; fix round 1
   defers it to after the frame.)
   - GPUI empties both focus paths in its focus events while the window is
     inactive (`gpui-pre window.rs:3354-3393`). The test windows are
     inactive, so `on_focus_out` never fired. A landing in production while
     the app's window is in the background would be missed the same way.
   - The render check is the brief's Step 5 sketch with one change. It
     moves only focus it finds behind the sheet in the frame drawn last; it
     never treats "not in the sheet" as outside. That avoids A7 item 1,
     which was why A7 rejects a plain render check.
   - Limit: a control drawn behind the sheet for the first time, and
     focused in the same update, is moved only at a later render. No
     extra frame is requested. No landing today does this: they go through
     `focus_content` (R-34) or focus controls that are already drawn.
   - The Kit's title-bar controls are not in the "behind" set. The
     Workspace does not own their focus, and nothing focuses them.
2. **`settings_return` is cleared on the second render after arming, not
   by "the render after the close"** (A3, R-45; Ruling 21). The render runs
   before the frame swap and before `focus_lost` (`window.rs:3304`, `:3354`,
   `:3363-3368`).
3. **The close runs in `on_close`; R-7 said `sync_sheet`** (Ruling 20).
4. **A2 case 3 ("its restore, else its context")**: a menu already chosen
   returns to its context, not to its pointer `restore`. That matches what
   each menu's own dismissal does after a choice, and what the Kit has
   focused (`repository_menu.rs:291`, `activity_menu.rs:120`). Without this,
   the repository menu's Settings… opened by pointer would return focus to
   wherever focus was before the right-click.
5. **R-43**: `settings_origin` is set in `choose_activity`, around its one
   `run_activity_action` call. It is not set in `run_activity_action`
   itself, whose other direct callers (Enter on a row with no button) open
   no Settings (ledger 3.4).
6. **R-47**: `OpenFlag::is_open` also requires `has_active_sheet`, on top of
   being cleared at once (A7 item 10).
7. **The origin stays on `SettingsHost`** (Ruling 12). `workspace.rs` gains
   only `settings_origin` and `settings_return` and their initial values.

### Tests that ran (Task 2)

- Phase A, each alone by exact name (all
  `desktop::view::workspace::settings_sheet::tests::`): P5
  `focus_returns_to_what_opened_settings` (19 rows; 6 red, F5 by button, F8 x4,
  the added F9), P22 `late_work_moves_no_focus` (9; 5 red), P23
  `focus_stays_inside_settings` (13; 5 red), and
  `focus_on_open_survives_the_sheet_keeping_focus` (6; green before, as
  expected). The helper move into `settings_sheet_tests/support.rs` is a pure
  move: the eight Task 1 tests passed by exact name after it (8 passed).
- Phase B GREEN, each alone: P5 19 of 19, P22 9 of 9, P23 13 of 13, the F1/F2
  test 6 of 6; Task 1's eight tests 8 passed.
- Module runs, once: run 1 (`keyboard_tests`, `footer::tests`,
  `repositories::tests`, `operations::attention_tests`,
  `operations::outcomes::tests`, `operations::outcomes::route_tests`,
  `lifecycle::credential_tests`, `lifecycle::outcome_canned_tests`,
  `settings::keys::tests`, `settings::repositories::manage_tests`,
  `create::tests`, `item::share::tests`): 181 passed, 158.2 s. Run 2
  (`settings::hosts::tests`, `settings::repositories::tests`,
  `workspace::tests`, `tabs::tests`, `needs_attention::tests`,
  `activity::tests`, `drafts::routing_tests`,
  `workspace::settings_sheet::tests`): 146 passed, 147.3 s. Run 3 (a named
  doubt: Settings-touching modules from Task 1's list): 54 passed, 59.3 s.
- Fix round 1: new test `a_landing_drawn_behind_settings_leaves_focus_inside`
  (`settings_sheet_tests/behind.rs`): RED before (`0 passed; 1 failed`, focus in
  `[Root > Workspace > Content]` with the sheet open), GREEN after (`1 passed`),
  and it reads focus with no forced draw. Its fixture holds Drafts' refocus
  behind a probe dialog and closes the dialog in one update. It passes with
  either the deferred check or the Drafts gate alone, so it pins the pair.
  Covering: 13 settings_sheet tests by exact name (13 passed, 28.7 s); by
  module `drafts::` (42), `workspace::tests::` (29), `footer::tests::` (9),
  `item::sheet_tests::` (16): 96 passed, 69.6 s.
- fmt, `cargo check --locked --features desktop` and clippy `-D warnings`:
  clean at each commit. The binary has 1925 tests (1920 + 4 + 1).

### Parked review Minors (Task 2) and notes for later

1. `focus_lost`-moved focus asks for no redraw (`settings_sheet.rs:282-286`,
   `:338-341`): add `cx.notify()` (cannot loop).
2. `settings_return` lingers one extra frame after a close found by
   `sync_sheet` (`render.rs:168` against `:173`); no production path closes the
   sheet directly today.
3. P5's F10 row (`returns.rs`, `no_origin`) cannot fail and is unlabelled.
4. P5 closes every row by Escape; F4 also names the close button and the scrim,
   with no focus read after either (they share the Kit's `close(request,
   on_close)` path).
5. `table.rs`'s `done` checks no expected row count (P23 has 11 rows on macOS
   and Windows, 13 on Linux, since the context-menu rows are Linux-only,
   `inside.rs:75`): pass a count, per platform.
6. Untested sub-cases: F8's "if that is gone, F9"; F8 with a pointer-opened
   menu (where `restore` is set); R-48's `inspect_root` clearing.
7. **For the owner:** after a page dialog closes within 0.25 s of an earlier
   deferred close, focus goes to the dialog's origin (F15) and then jumps to the
   page's entry when the Kit's restore fires (F16). The tests assert it
   (`inside.rs:294-305`). A6 says so; it may be unexpected on screen.
8. **Platform gaps for native CI after merge:** F14's EditsNotKept row is
   unix-only (Ruling 17), and P23's context-menu rows run only on Linux.
9. `HostsPage::land_refocus_gone` is a test-only hook (`hosts.rs`, gated
   `cfg(all(test, feature = "desktop-test-support"))`).
10. The Task 2 module-run list was taken from Task 0's ledger entry; no whole
    suite ran.
