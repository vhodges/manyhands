---
title: "Wave 04 Cycle 05b Design: Close Ticket And Resolve Conflict"
date: 2026-11-15
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7FACCK2EN0W8KS5HMV751RB"
---

# Wave 04 Cycle 05b Design: Close Ticket And Resolve Conflict

Ticket `01M7D7JK5MSY6J471JEKG3CZXR`.
[Cycle document](../Cycles/wave-04-cycle-05b-close-and-resolve.md)
and [implementation plan](2026-11-15-wave-04-cycle-05b-close-and-resolve-implementation.md).

## Intent And Approval Boundary

This design puts Wave 03 D5's two surfaces on Cycle 05a's common frame
and lays them out as `05` draws them:

- the lifecycle dialog (Close ticket… and Approve and merge…), whose
  stages get the design's header, step rows, outcome box and footer;
- Resolve conflict, which moves from a centered dialog to a right-side
  920 px sheet the workspace keeps (decision 3).

Behaviour, services, requests and recovery routes stay as they are. The
named exceptions are the footer order (decision 4), the running stage's
buttons and backdrop (decision 4), the sheet itself (decision 3) and the
copy (decisions 5 and 12, Part C). It changes `src/desktop/` only.
Approved 2026-11-15 (revision 4, Cycle decision 13; revision 4 had no
independent review), with implementation authorized, subagent-driven and
serial, from the handoff brief `2026-11-15-wave-04-cycle-05b-handoff.md`.
Execution waits until Cycle 05a has merged. Approval covers the
mechanisms, rulings, named test edits and task order. It does not
authorize a pull request, merging, closing the ticket, worktree cleanup,
native CI or creating tickets.

Planned against:

- the ticket branch at `ac50ba1a`: main `ea6d899b`, with Cycle 04's
  branch and Cycle 05a's Tasks 0–3 merged in through 05a's branch
  (`2c398b03`);
- **Cycle 05a's approved design** for what 05a's Tasks 4–10 have not
  built yet: the Add repository sheet's owner (C5a B1–B3), the widened
  "a sheet is open" check (C5a fact 20, R-46), `frame::sheet_footer` in
  use (C5a B10) and `Tone::Accent` in use (C5a Part D). Task 0 re-maps
  every name this design takes from 05a against its merged code (U-1).

## Conventions

- `D/` is `src/desktop/`; `GC/` is the locked `gpui-component-0.6.6/src/`
  under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. The app
  reaches it only as `gpui_kit::…`.
- "C5a B1" and similar name a part of Cycle 05a's design; "C5a R-n" its
  rulings. "C4" is Cycle 04's design. "D5" is Wave 03 D5's design
  (`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md`),
  "D5 decision n" its Cycle document's.
- "Decision n" is this Cycle document's. "Fact n", "R-n", "H-n" (Resolve
  focus rows), "P-n", "E-n", "K-n" and "U-n" are this design's.
- **(owner's eye)** marks a change people see today or a departure from
  an authority.
- Line numbers are at `ac50ba1a`.

## Baseline Facts

### The lifecycle dialog today

1. **One view, two commands.** `LifecycleDialog` (`D/view/lifecycle.rs`,
   625 lines, with `lifecycle/` holding `view.rs`, `confirm.rs`,
   `running.rs`, `outcome.rs`, `actions.rs`, `prepare.rs`,
   `made_ready.rs`, `repair.rs`, `republish.rs`, `resumed.rs`) serves
   `Intent::TicketClose` and `Intent::DocumentPromote`. Its title is
   `intent.label()`, "Close ticket" or "Approve and merge"
   (`D/services/requests.rs:460-461`).
2. **Opening.** `lifecycle::open` (`lifecycle.rs:214-244`) builds a Kit
   `Dialog`: `base.w(px(580.)).close_button(false).overlay_closable(false)`,
   `on_ok` calls `enter()` and returns false, `on_cancel` returns true in
   every phase ("Escape: close; what was sent goes on"), and `on_close`
   focuses the origin. The builder closure runs at every render of Root's
   dialog layer (`GC/root.rs:262`), so anything it reads from the entity
   is current.
3. **The phases** (`lifecycle.rs:314-353`): `Saving`, `Preparing`,
   `Confirm { preview, confirmation }`, `Blocked { code, blockers,
   way_out, shared }`, `Held { way_out }`, `Stale { code, message }`,
   `Answered { code, message }`, `Message(&'static str)`, `Repaired`,
   `Submitted(DesktopRequestId)`. There is no running, failed or done
   phase: `submitted_body` (`running.rs:93-113`) draws `running` until an
   answer is known, then `ended`, whose `Ending` is `Done`, `Failed`,
   `Stopped` or `StillRunning` (`D/model/activity/lifecycle_lines.rs:84-126`).
   `Removed`, `NotSent` and `Dismissed` requests draw a message only.
4. **The drawing** (`view.rs:156-255`). One `v_flex` with id
   `lifecycle-dialog` tracks the dialog's own focus. It holds, in order:
   - the header (`view.rs:64-111`): a 20 px kind icon (`lifecycle-kind`,
     labelled "ticket" or "document"), `lifecycle-title`
     (`intent.label()`), `lifecycle-subtitle` ("{short} · {title} ·
     {repository}", `Heading::subtitle`, `lifecycle.rs:176-184`) and
     `lifecycle-branch`;
   - the phase's body;
   - `repair_line`;
   - the buttons, `h_flex().gap_2()`, not right-aligned.

   The Kit dialog's title and footer slots are not used, and
   `D/view/frame.rs` is not used.
5. **Steps.** `confirm::steps` (`confirm.rs:92-153`) gives the preview's
   steps in the library's order (D5 R-8): Save your latest edits; "Share
   with {remote}" or `KEEP_LOCAL` "Keep on this computer (sharing
   pending)"; Mark as closed or Already marked closed (a closure only,
   note "Closed by {closed_by}", `closed_by` being "Name <email>",
   `src/repository/closure/observe.rs:44-47`); "Merge into
   {primary}" or "Already in {primary}"; "Share {primary} with {remote}"
   (when publishing); Tidy up. Each `Step` has `key`, `title` and `note`
   (`confirm.rs:67-71`). `steps_by_kind` (`outcome.rs:92-130`) gives the
   steps of a run shown without its preview, with empty notes.
6. **Confirm** (`confirm.rs:355-497`): the lead (`lifecycle-lead`), the
   bordered numbered steps (`("lifecycle-step", ix)` and
   `("lifecycle-step-note", ix)`), "Files that will be merged into {primary}
   ({n})" (`lifecycle-paths-heading`), each path row
   (`("lifecycle-path-row", ix)`) with exactly three elements: change icon,
   path and tag (`ticket`, `document`, `comment` or `code`, from
   `item_path_kind`, D5 R-2; `confirm_tests.rs:62` `row_elements` pins
   three, D5 R-3), the code note (`lifecycle-code-note`), the Git line
   (`lifecycle-git`, `confirm.rs:178-204`), and "Expires {time}"
   (`lifecycle-expires`). Buttons: the command (primary,
   `lifecycle-confirm`), then Cancel (`lifecycle-cancel`).
7. **Running** (`running.rs:146-175`): each step as a state word in an
   84 px column (`("lifecycle-step-state", ix)`: "Done", "Running", "Not
   started") and its title, no note; then `STARTED` ("It continues in the
   background if you close this, and shows up in Activity.",
   `lifecycle-state`). Buttons: Stop at a safe point (`lifecycle-stop`,
   not primary), or "Stopping…" (`lifecycle-stopping`) once pressed; and
   Close (`lifecycle-cancel`), which `render` adds to every submitted
   stage (`view.rs:248-252`).
8. **Ended** (`running.rs:177-281`): the ending word (`lifecycle-ending`:
   "Done", "Failed", "Stopped at a safe point", "Still running…"); for a
   stop, the cancellation and completed lines; the step rows with states
   ("Done", "Not done", "Failed"); the outcome lines
   (`("lifecycle-outcome", ix)`, `lifecycle_lines.rs:250-299`); the
   made-ready part ("Now ready:", `made_ready.rs:38`); blockers; the
   no-record line; the republish line; the guidance
   (`("lifecycle-way-out", ix)`); "Still running…"; for Failed or
   StillRunning, the message (`lifecycle-message`) and code
   (`lifecycle-code`). No box, tint or icon.
9. **Ended buttons** (`running.rs:277-352`), in this order: republish
   buttons, Repair locally… and Resolve conflict…, Retry (primary),
   Prepare again (primary only without Retry), Resume, Unlock… (primary)
   or Check host, Open Sharing & keys, Open repository details, Open
   folder, Open ready tickets; then Close.
10. **Enter** (`confirm.rs:273-280`, `actions.rs:221-256`): Confirm on
    confirm; Prepare again on stale or repaired; on a sent run, Done
    closes, else Retry, else Unlock…, else Prepare again. A focused
    button's wrapper takes Enter first (`view.rs:38-60`).
11. **Focus.** Every press focuses the dialog's own handle first
    (`view.rs:121-142`, A21). `cancel` and `on_close` focus the origin
    (`view.rs:113-119`, `lifecycle.rs:241`). The origin is the item
    header's lifecycle action, or the Activity list when reopened from
    it.
12. **Made ready** (`made_ready.rs`): only on an ended closure that
    reached main (D5 R-20, D5 decision 6): "Now ready:" and a row per
    ticket, "{short} · {title}" (`("lifecycle-made-ready", ix)`), the
    incomplete note, or "Could not work out which tickets are now ready."
    with Open ready tickets.

### Resolve conflict today

13. **A centered dialog.** `conflict::open` (`D/view/conflict.rs:191-217`)
    refuses while quitting or saving drafts, builds `ResolveSheet`, and
    opens a Kit `Dialog`: `w(px(920.))`, no close button, the overlay not
    closable, `on_ok` false, `on_cancel` true, `on_close` focuses the
    origin. Its root (`render.rs:500-517`) is `v_flex().id("conflict-sheet")`
    tracking the sheet's focus, with `key_context(RESOLVE_SHEET_CONTEXT)`,
    the `SaveResolution` action (Ctrl+Enter), `max_h(px(720.))` and its
    own vertical scroll, holding the header, status, cards, Git line and
    buttons.
14. **Its ways in.** Three production callers:
    - the item banner's Review changes (`ItemPane::review_conflict`,
      `D/view/item/review.rs:81-111`), refused while a dialog is open,
      origin `review_focus`;
    - Activity's Review changes on a request or a pending row
      (`Workspace::review_request`, `review_pending`,
      `D/view/operations/review.rs:64-106`, through `Entry::open`,
      `D/view/conflict/entry.rs:134-163`), refused while a dialog is
      open; Needs attention runs the same actions (`choose_activity`);
    - the lifecycle dialog's Repair locally… and Resolve conflict…
      (`LifecycleDialog::open_sheet`, `D/view/lifecycle/repair.rs:300-328`),
      which closes the lifecycle dialog first.

    Tests open it through `open_sheet_in`
    (`D/view/conflict_tests.rs:100-127`), which calls `conflict::open` in a
    full `Shell`; 33 call sites use it or `open_sheet`.
15. **The header** (`render.rs:66-127`): `conflict-title` "Choose what to
    keep · {title}" (or without a title), `conflict-description`
    (`DESCRIPTION_REMOTE` or `description_local`), a meta line
    (`conflict-kind` icon, `conflict-short`, `conflict-repository`,
    `conflict-progress` "{n} of {m} chosen" amber then green), and
    `conflict-note` (D5 decision 15's note by source).
16. **Cards** (`render.rs:208-445`): per path, `("conflict-path", i)`;
    taken rows (`("conflict-taken", n)` "{key}: Only theirs changed it;
    theirs is kept."); per field part a bordered card whose header row is
    `("conflict-field", k)` (the **raw key**, `"title"`), Keep mine, Keep
    theirs, and `("conflict-choice", k)`; the body part's card,
    `("conflict-body", i)` "Description", Keep mine, Keep theirs, Keep
    both, `("conflict-body-choice", i)`. The state words are `CHOOSE_ONE`
    "Choose one" and `KEEPING_MINE` "Keeping yours", `KEEPING_THEIRS`,
    `KEEPING_BOTH`, `KEEPING_EDITED` "Keeping the result as edited"
    (`conflict.rs:99-103`). Columns Before, Yours, Theirs
    (`render/pieces.rs`); Yours and Theirs lines are `conflict-line` with
    the gutter inside the text ("~ Mine"). The body's Result is an editor
    in a tab stop (`("conflict-result", i)`, D5 R-27, R-31) with
    `RESULT_NOTE` (D5 R-34).
17. **Footer** (`render.rs:129-158`, `448-497`): the Git line
    (`conflict-git`, "Git: conflict in {paths} · ours {8} · theirs {8}", no
    base, D5 R-27), then a right-aligned row: Resume, Retry, the lost
    line or Save resolution (primary), then Cancel, which calls
    `close_dialog` and focuses the origin.
18. **Data.** Neither `ConflictDto` (`src/repository/remote/conflict_read.rs:135-182`)
    nor the local inspection (`src/repository/read/local_sync.rs:47-80`)
    names an author, a time or a base commit. A field part's `key` is the
    front matter key (`D/model/conflict/merge.rs:31-41`); the known keys are
    listed at `merge.rs:103-114`.
19. **Keys.** `secondary-enter` → `SaveResolution` and `enter` →
    `NoAction` in `RESOLVE_SHEET_CONTEXT`; `escape` → `LeaveResult` in
    `RESOLVE_SHEET_EDITOR` (`D/keys.rs:455-465`). Tab goes in draw order:
    the choices, the Result stop, the primary button, Cancel
    (`keyboard_tests::conflict::sheet::edit_the_result_by_keyboard`).

### The workspace's sheet slot

20. **One slot, one owner.** Root keeps one `active_sheet`
    (`GC/root.rs:40-41`); `open_sheet_at` replaces it and focuses a new
    handle of Root's (`GC/root.rs:379-405`), as `open_dialog` does for a
    dialog (`GC/root.rs:297-311`). The workspace records who keeps it in
    `content.sheet: Option<SheetOwner>`, today `Details(WeakEntity<ItemPane>)`
    or `Settings` (`D/view/workspace/content.rs:44-50`), and C5a B1 adds
    `AddRepository`. `sync_sheet` (`content.rs:270-324`) closes any active
    sheet its owner does not keep.
21. **Nothing acts behind a sheet** (Wave 03 D3 R-60): `modal_open` is
    `has_active_dialog || has_active_sheet` (`D/view/workspace/render.rs:149-151`),
    and Activity, its menus and Needs attention refuse under it.
    `focus_lost` (`D/view/workspace/focus.rs:285-296`) restores the lost
    focus target while a dialog or sheet is active. C4's
    `keep_focus_in_settings` (`settings_sheet.rs:303-313`) pulls focus back
    into Settings when something behind takes it, gated on `settings_open`
    (`settings_sheet.rs:93-96`). On 05a's branch (`d7b39f62`, its Task 6)
    `settings_open` stays Settings' and a new `sheet_held` (Settings or
    Add repository) gates the renamed `keep_focus_in_sheet`,
    `sheet_keeps_lost_focus`, `focus_content` and `settings_key`
    (`settings_sheet.rs:116-120`, `:347-371` there).
22. **Ctrl+, and Settings routes.** `settings_key`
    (`settings_sheet.rs:350-360`) refuses while a dialog or Settings is
    open; `show_settings` (`:117-185`) refuses while a dialog is open, and
    otherwise closes any active sheet before opening Settings (`:143-147`).
    Today the Resolve dialog stops both; as a sheet, it would not.
23. **Cycle 05a's frame** (`D/view/frame.rs`, 05a Task 1): `Tone { Accent,
    Danger, Success }` (`Danger` and `Success` marked "Used from Cycle
    05b", `Accent` "Used from Task 8"); `Header { icon: Option<(IconName,
    Tone)>, title, description, ids }`; `Footer { hint: Option<&'static
    str>, buttons: Vec<AnyElement> }` (buttons drawn in order, right-aligned,
    the hint on the left); `dialog(base, header, cx)` puts the header in a
    Kit dialog's title slot and zeroes its padding; `body(content)`;
    `footer(footer, cx)`, drawn as the body's last child (C5a R-19);
    `sheet_header` for a Kit sheet's title slot; `sheet_footer` for its
    footer slot ("Used from Task 4", the Add sheet).
24. **Icons.** `icon_assets!` (`D/assets.rs:17-79`) holds `Archive`,
    `CircleCheck`, `ShieldCheck` and `CircleAlert`; not `CircleX` or
    `GitMerge`. `CircleX` loads anyway, from the Kit's default bundle
    through the `Assets` fallback (`D/assets.rs:103-108`); `GitMerge` must
    be listed. Both are in the Kit's `IconName`. `ICONS` in
    `D/assets_tests.rs:8` lists 58 names (`every_item_view_icon_loads`).
    The Kit has `Spinner` (`GC/spinner.rs:10`), already drawn by
    `D/view/item/pill.rs:234`.

### Data the library does not give

25. **No line counts.** The preview's paths carry only a change kind
    (`LifecyclePathChangeDto`, `src/repository/mutation/dto.rs:294-336`);
    nothing in the library computes diff stats (D5 R-3, K-7).
26. **No ready list before the close.** The library's made-ready report
    is read only after the merge (`src/repository/read/graph.rs:285-298`,
    `pub(super)`; `bind/lifecycle.rs:456-499`), and D5 decision 6 and R-20
    show it only on a closure that reached main, Done or Failed
    (decision 2).
27. **No progress counts.** `ProgressEvent` carries a stage and no count
    (`src/repository/mutation/progress.rs:95-129`); the desktop's stage
    words are `stage_words(stage, primary)`
    (`D/model/operations/progress.rs:139-151`), for example "Sending your
    saved changes…".
28. **No key name in a failure.** A refused key reaches the desktop as a
    code and a fixed message ("The remote rejected the selected key.");
    the key's ID stays in the transport error (`bind/lifecycle.rs:1304`).
29. **No closing time before the close.** `closed_by` is in the preview;
    `closed_at` is taken when the close runs
    (`src/repository/closure/checkpoint.rs:70`).
30. **No authors, times or base commit for a conflict** (fact 18).

Decision 1 sends facts 25 and 30 (line counts, authors and times, the
base) to the follow-up library ticket `01M7FBRBN04QACSCZV6KX0WMR3`
(`vh-nkp8c`); facts 27 and 28 stay Known Limits (K-3, K-4).

### Guards, focus and the tests

31. **Dialog-only guards.** About 50 production sites in `D/view` check
    `window.has_active_dialog(cx)` without `has_active_sheet`. Most refuse
    a person's action (Done, Preview, Review changes, Resume, open the
    lifecycle dialog). Two open something by themselves:
    - the deferred Republish… (`D/view/operations/republish.rs:205-230`):
      a share entry's Stop and republish… (`outcomes.rs:270`, no dialog)
      waits for the share to stop, then opens Republish… unless a dialog
      is open, else waits for none to be (`open_deferred_republish`, run
      at each Workspace render). One pressed from the lifecycle dialog
      names that dialog and opens over it at once (`over_dialog`,
      `D/view/lifecycle/republish.rs:234-243`), and its wait goes with
      the dialog (`republish.rs:157-166`);
    - `apply_review` (`D/view/workspace/content.rs:127-148`), which opens
      Drafts or an identity form at the next frame.
32. **Where a sheet's focus counts.** Root's sheet layer is drawn inside
    the Content region (`D/view/workspace/render.rs:114-131`), so focus in
    a sheet counts as the Content region's; Settings' keyboard tests
    assert `Region::Content` (`D/view/keyboard_tests/settings.rs:25`). The
    keyboard walk's `in_dialog` (`D/view/keyboard_tests/drafts.rs:73-87`)
    asserts that no region holds focus, which a sheet fails.
    - The Kit's `sheet-host` wraps the title row, body and footer, tracks
      Root's handle and holds the focus trap
      (`gpui-base-0.6.6/src/sheet.rs:118-140`). An element in the title or
      footer slot whose control goes has `sheet-host` as its nearest
      focusable ancestor, which `focus_lost` restores to, unless the slot
      tracks a handle of its own.
    - The Kit sheet always draws a close button in its title row, a Tab
      stop that dispatches `Cancel` (`GC/sheet.rs:195-207`).
    - The Kit's sheet body scrolls on a handle of its own and does not
      reveal the focused control; the details sheet lays its own scroll
      over it for that (`D/view/item/sheet.rs:254-278`), and the keyboard
      walk asserts that each focused control is visible
      (`D/view/keyboard_tests.rs:157-166`).
33. **The harness reads window-wide.** `Shell::label`, `present` and
    `rows` find an id anywhere in the window (`window.try_find`,
    `D/view/harness.rs:344-481`). The privacy sweeps file each labelled
    element under the first `SURFACES` entry, in array order, whose id is
    the element's own or an ancestor's (`D/view/lifecycle/privacy_tests.rs:112-113`,
    `:143-150`, `:206-219`); an element under none is "drawn", which the
    sweeps also read. `ALLOWED` (`:78-89`) is by id: item text may appear
    only in `lifecycle-subtitle`, `conflict-title`, `conflict-before`,
    `conflict-line` and `conflict-result`. None of the privacy tests
    sweeps passphrase or key text over these two surfaces.
34. **Snapshots have no order.** `gpui_kit::base::test_support::snapshots`
    iterates a map (`gpui-base-0.6.6/src/test_support.rs:173-187`);
    `confirm_tests`' `row_elements` sorts ids by their Debug text
    (`lifecycle/confirm_tests.rs:62-80`). Drawn order is read from each
    element's bounds.

## Part A: The Lifecycle Dialog On The Frame

### A1. Stages

The dialog's phases group into six stages, which set the header, the
footer and the backdrop:

| Stage | Phases | Close ticket title | Approve and merge title | Tile |
| --- | --- | --- | --- | --- |
| Ask | `Saving`, `Preparing`, `Confirm`, `Blocked`, `Held`, `Stale`, `Answered`, `Message`, `Repaired`; a submitted request shown as a message only (`Removed`, `NotSent`, `Dismissed`, a resume answered without lifecycle data) | "Close {short}?" | "Approve and merge {short}?" | `Archive` (close) or `GitMerge` (promote), Accent |
| Running | `Submitted`, no answer yet | "Closing…" | "Merging…" | as Ask |
| Waiting | an answer whose `Ending` is `StillRunning` | "Still running…" (`STILL_RUNNING`) | "Still running…" | as Ask |
| Done | `Ending::Done` | "Ticket closed" | "Merged into {primary}" | `CircleCheck`, Success |
| Failed | `Ending::Failed` | "Couldn't finish closing" | "Couldn't finish merging" | `CircleX`, Danger |
| Stopped | `Ending::Stopped` | "Stopped at a safe point" | "Stopped at a safe point" | as Ask |

`{short}` is `Heading::short`; `{primary}` is `LifecycleDialog::primary()`
(D5 R-35). `LifecycleDialog::stage(&self, cx) -> Stage` computes it from the
phase and, for `Submitted`, through the same helper `submitted_body`
uses, so the header, the footer and the backdrop never disagree (R-1).
The titles are decision 7.

### A2. The header

`lifecycle::open` builds the dialog with `frame::dialog(base, header, cx)`,
where the builder reads `dialog.read(cx).header(cx)` at each render
(fact 2), so the title and tile follow the stage:

```rust
frame::Header {
    icon: Some((stage_icon, stage_tone)),
    title: stage_title.into(),
    description: Some(self.heading.subtitle().into()),
    ids: ("lifecycle-title", Some("lifecycle-subtitle")),
}
```

- `lifecycle-subtitle` keeps today's text.
- `lifecycle-kind` and `lifecycle-branch` go **(owner's eye)**. The
  title's command words say the kind ("Close … ?" for a ticket, "Approve
  and merge … ?" for a document); the branch is in the Git line, which
  every stage with the preview now draws (A4–A6).
- `lifecycle-ending` stays, with today's words, as the outcome box's
  first line (A6, R-17).
- The header holds no control (C5a A1). The harness reads ids
  window-wide, and the privacy sweeps read the title slot as "drawn",
  with the subtitle allowed by id (fact 33), so no test needs the header
  inside `lifecycle-dialog`.

`render` keeps the `lifecycle-dialog` element, which tracks the dialog's
own focus (fact 11), and draws `frame::body(stage body)` followed by
`frame::footer(..).id("lifecycle-footer")` as its last child (C5a R-19).

### A3. Step rows

One row drawer serves every stage, in the bordered list `05` draws:

- **The mark,** 20 px, left: the step's number in a muted circle on
  confirm, and on later stages for a step not started or not done;
  `CircleCheck` (green, 18 px) for Done; a Kit `Spinner` (16 px, teal)
  for Running; `CircleX` (red) for Failed. Past confirm, the mark's
  wrapper keeps the id `("lifecycle-step-state", ix)` and is labelled
  with today's state words (`StepState::words`), so tests and screen
  readers read the same words (R-2).
- **The title,** `("lifecycle-step", ix)`, red for a Failed step.
- **The note,** `("lifecycle-step-note", ix)`, drawn when it is not
  empty, in every stage:
  - a step's own note comes from the preview the dialog holds; a run
    shown without it has none (fact 5);
  - the Running step's note is `stage_words(stage, primary)` (fact 27)
    once the request has reported a stage, with or without the preview
    **(owner's eye)**;
  - the Failed step's note is the answer's message, with the id
    `lifecycle-message`, in red, in place of its own note. With no Failed
    step (Waiting; a failure whose steps are all done), the message goes
    in the outcome box (A6).

### A4. Confirm

As today (fact 6), except:

- the Mark as closed note reads "Closed by {name} · today" when the
  closure is to be recorded (`LifecycleRecording::Record`), and "Closed
  by {name}" when it already is ("Already marked closed",
  `LifecycleRecording::Existing`, which may be from an earlier day);
  `{name}` is `closed_by` up to its " <" (the whole value when it has
  none) (decision 8, R-3);
- the footer (A7): hint "Enter to confirm · Esc to cancel", then Cancel ·
  **{command}** (decision 4);
- the Expires line stays (`lifecycle-expires`), muted, after the Git
  line (R-4).

No line counts (decision 1), no "Becomes ready to start" (decision 2).

### A5. Running

- The lead goes above the steps: "You can keep working. This continues
  in the background and shows up in Activity." (`lifecycle-state`,
  replacing `STARTED`).
- The steps (A3), then the Git line (`lifecycle-git`, from the preview
  the dialog holds; none without it, K-15), as frame 2d draws it.
- **Footer:** hint "Esc hides this window", then Stop at a safe point
  (`lifecycle-stop`). Once pressed, "Stopping…" (`lifecycle-stopping`)
  takes Stop's place, as today. It is drawn as an inert Tab stop with a
  handle of its own, labelled "Stopping…", which Enter and Space do
  nothing on, so the dialog keeps a Tab stop and Escape keeps working
  (R-23).
- **No dismiss button** **(owner's eye)** (decision 4).
- **Backdrop:** clicking it hides the dialog as Escape does: `on_cancel`,
  then the close, then `on_close`, which focuses the origin
  (`gpui-base-0.6.6/src/dialog.rs:600-621`); the run goes on and Activity
  keeps it. The builder sets
  `overlay_closable(dialog.read(cx).hides_on_backdrop(cx))`, true only in
  the Running stage (R-5). Every other stage keeps today's inert
  backdrop. The Kit reads the flag when it draws, so a click in the one
  frame after the run ends hides the ended dialog (K-18).

### A6. Ended (Waiting, Done, Failed, Stopped)

In order: the step rows (A3), the **outcome box**, the made-ready box,
the Git line (`lifecycle-git`, from the preview the dialog holds; none
without it, K-15), as frame 2e draws it.

**The outcome box** (radius 6, a 1 px border in the tone, the tone at 8%
behind): Done, `CircleCheck`, green; Failed, `ShieldCheck`, red;
Stopped and Waiting, `ShieldCheck`, amber. It holds today's ended lines
(fact 8) in today's order, each with its id:

1. the ending word (`lifecycle-ending`: "Done", "Failed", "Stopped at a
   safe point", "Still running…", semibold; R-17);
2. for a stop, the cancellation and completed lines; for a done run
   after Stop, "It finished before it could stop.";
3. the outcome lines;
4. the blockers; the no-record line; the republish line; the guidance;
   "Still running…" (`lifecycle-still-running`, when Continue or Resume
   waits);
5. the message, when no step shows it (A3), and the code.

**The made-ready box** (fact 12; D5 R-20: a closure that reached main,
Done or Failed), teal-tinted, its lead reading "Now ready to start"
(Part C).

### A7. Footers

The rule (decision 4; decision 7): the dismiss button first, then
today's non-primary buttons in today's order, then today's primary
buttons in today's order; on Done, Done comes last. "Today's order" is
fact 9's for an ended run and `render`'s for the other phases
(`view.rs:156-255`); "primary" is today's flag on each button. The
dismiss button keeps the id `lifecycle-cancel` and today's Cancel and
Close behaviour (close, focus the origin). Every stage keeps at least
one Tab stop (R-23).

| Stage: phase | Footer, in order (primary in bold) | Hint |
| --- | --- | --- |
| Ask: `Saving`, `Preparing` | Cancel | — |
| Ask: `Confirm` | Cancel · **{command}** | "Enter to confirm · Esc to cancel" |
| Ask: `Stale` | Cancel · **Prepare again** | — |
| Ask: `Blocked` | Close · the way-out actions, non-primary then primary · **Prepare again** (none for `ticket_closed`) | — |
| Ask: `Held` | Close · **Continue** or **Resume** | — |
| Ask: `Answered` | Close · **Retry** (on `busy` only) | — |
| Ask: `Repaired` | Close · **Prepare again** | — |
| Ask: `Message`, a submitted message only | Close | — |
| Running | Stop at a safe point, or the inert "Stopping…" stop | "Esc hides this window" |
| Waiting | Close · today's other buttons, by the rule | — |
| Failed, Stopped | Not now · Repair locally…, Review changes…, Prepare again (when Retry is offered), Resume, Check host…, Open Sharing & keys…, Open repository details…, Open folder, Open ready tickets (each when offered) · **Republish…** or **Stop and republish…**, **Retry**, **Prepare again** (when no Retry), **Unlock…** (each when offered) | — |
| Done | Open ready tickets (when offered) · **Republish…** (when offered) · **Done** | — |

**(owner's eye)** for every row: today the primary comes first and the
row is left-aligned. The labels Cancel, Close, Not now and Done are
decision 7's. Building the rule needs each button's primary flag: the
helpers return `(primary, element)` pairs instead of bare elements
(Task 4).

### A8. What does not change

Escape closes the dialog in every stage and cancels nothing (fact 2;
C5a decision 7). Enter does what it does today (fact 10). Every press
still focuses the dialog first (fact 11). Focus returns to the origin on
every close, the backdrop's included. The stale, Retry, Stop and
made-ready behaviours are D5's.

## Part B: Resolve Conflict As A Sheet

### B1. The owner

`SheetOwner` (fact 20) gains a case:

```rust
pub(super) enum SheetOwner {
    Details(WeakEntity<ItemPane>),
    Settings,
    AddRepository,
    /// Resolve conflict (Cycle 05b).
    Resolve(WeakEntity<ResolveSheet>),
}
```

and the test mirror `SheetOwnerKind` (`D/view/workspace/settings_sheet.rs:41`)
gains `Resolve`. `sync_sheet` gains two cases:

1. **Owner `Resolve`, and a sheet is active:** kept, whatever item is
   shown.
2. **Owner `Resolve`, and no sheet is active:** the sheet closed. The
   owner is cleared. Focus is already where B3 put it.

While `resolve_open`, a requested details sheet is dropped
(`content.details = None`), as C4 does while Settings is open
(`content.rs:307-308`), so none opens after the sheet closes.

Two predicates, beside C4's `settings_open`, which stays Settings' alone
(R-11):

- `Workspace::resolve_open(&self, window, cx) -> bool`: the owner is
  `Resolve` and a sheet is active. Every refusal and wait in B2 reads it,
  so an Escape followed at once by Ctrl+, is not refused by an owner
  `sync_sheet` has not yet cleared.
- `Workspace::workspace_sheet_open(&self, window, cx) -> bool`: Cycle
  05a's `sheet_held` (Settings or Add repository, `settings_sheet.rs:116-120`
  on 05a's branch at `d7b39f62`) or `resolve_open` (R-25). 05a's
  `keep_focus_in_sheet` and `sheet_keeps_lost_focus` (`:347-371` there)
  do not take Resolve: they send focus to Settings' or Add's first
  control; Resolve has its own (B3).

### B2. Opening

```rust
impl Workspace {
    /// Opens Resolve conflict on `opening`. `None` when refused.
    pub(in crate::desktop::view) fn show_resolve(
        &mut self,
        opening: conflict::Opening,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ResolveSheet>>;
}
```

in a new `D/view/workspace/resolve_sheet.rs`. It:

1. **refuses** while a dialog is open (as every way in does today, fact
   14), while any sheet is active, and, through `conflict::open`, while
   quitting or saving drafts (fact 13). No way in can fire with a sheet
   open (fact 21), so refusing loses nothing; C4 and C5a drop a details
   sheet instead, because their ways in can (R-21);
2. calls `conflict::open`, which now opens a Kit sheet on the right
   (`window.open_sheet`), `overlay_closable(false)` (decision 9), sized
   `min(920 px, the window's width less 48 px)`: the Kit sheet does not
   clamp itself (`GC/sheet.rs:176-186`) and the app sets no minimum
   window width (R-28). It has:
   - the title slot: `frame::sheet_header` (B4) inside an element with id
     `conflict-header`;
   - the footer slot: `frame::sheet_footer` (B6) inside an element with
     id `conflict-footer`, which carries `RESOLVE_SHEET_CONTEXT` and the
     `SaveResolution` action, so Ctrl+Enter saves from the footer's
     buttons, as the details sheet's footer does (`D/view/item/sheet.rs:225-239`,
     R-8), and tracks a handle of the sheet's own, `footer_focus` (B3);
   - the body: the `ResolveSheet` view, laid over the Kit's body with
     its own scroll (B4);
   - `on_close`: focus the origin, as today;
   - the builder reads the entity at each render, as the lifecycle's
     does (fact 2);
3. sets the owner to `Resolve(sheet.downgrade())`, in the same update,
   so no render of `sync_sheet` sees the sheet without its owner;
4. focuses the sheet's own handle (`conflict-sheet`) after
   `open_sheet` has focused Root's (fact 20; H1).

**The ways in** (fact 14) all go through it (R-9):

| Way | Today | After |
| --- | --- | --- |
| Item banner, Review changes… | `conflict::open` in `ItemPane::review_conflict` | emits `ItemRoute::Resolve(Box<conflict::Opening>)`; `Workspace::item_route` (`D/view/workspace/requests.rs:181-190`) clones it in `owned()`, and `open_route` (`:198-217`) gains the arm that calls `show_resolve`. `conflict::Opening` derives `Clone`. The route runs a frame later (`cx.defer`, `:298`): a dialog opened meanwhile refuses it, as today's guard would have refused the press |
| Activity row or pending row, Review changes… (and Needs attention, which runs them) | `Entry::open` from `review_request`, `review_pending` | `Entry::opening(..) -> Option<Opening>`, then `self.show_resolve(opening)` |
| Lifecycle dialog, Repair locally… and Review changes… | `conflict::open` after `self.close` | `self.in_workspace(.., |ws, w, cx| ws.show_resolve(opening, w, cx))`; `in_workspace` (`D/view/lifecycle/actions.rs:126-140`) becomes generic over its closure's return value, so Repair locally… still gets the sheet for `reopen_after` (`repair.rs:260-262`), and `open_sheet` drops its own `self.close`, which `in_workspace` does. The Kit cannot say which dialog is on top, and `in_workspace` closes the top one, so the dialog goes on only while focus is inside it: a press checks this before `on` moves focus, and does nothing otherwise; Repair locally…'s sheet, which opens when the local synchronization answers (`repair.rs:214-227`), waits instead, kept on the dialog, until a render finds focus back inside it, that is, until the prompt over it has closed (R-24) |

`conflict::open` becomes `pub(in crate::desktop::view)` and its only
production caller is `show_resolve`. The test helper `open_sheet_in`
(fact 14) opens through `show_resolve` (E-1).

**Settings and Add repository while it is open** (decision 9).
`settings_key`, `show_settings` and Cycle 05a's `show_add_repository`
refuse while `resolve_open`: opening Settings or Add repository would
close the sheet and lose the choices, and today the Resolve dialog
stops them (fact 22). For `show_add_repository` this is a defensive line
added to C5a B2 step 1 (Document Amendment 6).

**What opens by itself** (fact 31, R-18). Both sites that open
Republish… unless a dialog is open also wait while `resolve_open`:
`republish_once_stopped` (`D/view/operations/republish.rs:211`), reached
when a share's Stop and republish… finds the share stopped, and
`open_deferred_republish` (`:225`), which retries each render. A share
that stops while the sheet is open opens Republish… only once it closes.
`apply_review` is B3's (R-25).

### B3. Closing and focus

The sheet closes by Escape (outside the Result editor), its header's
close button (new, the Kit's), Cancel, Resume, a save that finishes it
(`applied`, `D/view/conflict/submit.rs:350-406`), or quitting
(`wait_to_quit` closes every sheet of its window,
`D/view/workspace/lifecycle.rs:221-240`, as it closed the dialog). The
scrim does not close it (decision 9).

- Escape and the close button are the Kit's and run `on_close`.
- **One way to close it from the sheet.** `ResolveSheet::close`
  (`submit.rs:428-431`), which Resume (`:409-413`) and `applied` use, and
  Cancel too once Task 5 re-points it (today Cancel is its own closure,
  `render.rs:488-495`), calls `Workspace::close_resolve(&sheet, window, cx) -> bool`,
  found through Root as `in_workspace` finds it. `close_resolve` closes
  the Kit sheet and returns true only while the owner is `Resolve(this)`.
  On false, the caller does nothing more: no focus move, no lifecycle
  reopen, no `follow_resume` (R-22, R-27). On true, it focuses the
  origin, as today, since `on_close` runs only on the Kit's own paths,
  and `applied` goes on to reopen the lifecycle dialog as today.
- **An answer that lands under a dialog** (an Unlock prompt over the
  sheet): `applied` (run from `ResolveSheet::operations_changed`,
  `submit.rs:254-284`) keeps the answer on the sheet and records the
  sheet in the Workspace's `resolve_waiting`; `run_waiting_resolve_close`,
  called from `Workspace::render`, does nothing while a dialog is open,
  and otherwise takes it and, through `cx.defer_in`, has the sheet run
  the rest of `applied` (close, focus, reopen). This is Cycle 05a's
  `add.waiting` and `run_waiting_add_close` (`add_sheet.rs:276-280`,
  `:339-350` on 05a's branch at `d7b39f62`; its G23), and the deferral
  keeps the Workspace from being entered twice (R-26).
- **Footer presses keep focus inside.** Each footer button's press
  focuses the sheet's own handle first, as `LifecycleDialog::on` does
  (`D/view/lifecycle/view.rs:121-142`); a footer control that goes by
  itself leaves focus on `footer_focus` (fact 32). Body presses do not
  move focus, as today: a body control that goes leaves focus on
  `conflict-sheet`, its nearest tracked ancestor, through `focus_lost`.
- **Focus moves that would land behind it.** `focus_content`
  (`D/view/workspace/content.rs:598-600`) and `apply_review`'s refocus
  (`:127-148`), which skip today while Settings is open and on 05a's
  branch while `sheet_held`, skip while `workspace_sheet_open`; the moves that run through them (a discarded
  draft's `DraftDiscarded`, `close_with_draft`, `open_created`) leave
  focus where it is. `apply_review` keeps its Drafts and Settings
  requests until the sheet has closed, rather than losing them to a
  refusal (R-25).
- **Something behind takes focus anyway:** `keep_focus_in_resolve`, run
  from the Workspace's render beside `keep_focus_in_settings` (05a's
  `keep_focus_in_sheet`; `D/view/workspace/render.rs:173-182`), on
  `resolve_open`, moves focus back to the sheet's own handle; like
  C4's, it leaves a context menu over the sheet alone (R-11).

**Where focus goes** (contract; each row read after a draw; P6):

| Row | Situation | Focus after |
| --- | --- | --- |
| H1 | Opened by any way in | the sheet's own handle (`conflict-sheet`); Tab reaches the body's first Tab stop (the first enabled choice of a canonical set; Open folder for an external set; Read again after a failed read); Shift+Tab reaches the Kit's close button, drawn before the body |
| H2 | Escape, not in the Result editor | the origin |
| H3 | Escape in the Result editor | the Result stop; the sheet stays open |
| H4 | Cancel, or the close button | the origin |
| H5 | A save that closes it | the origin, or the lifecycle dialog that `applied` reopens |
| H6 | The origin is no longer drawn when it closes | `focus_lost`'s fallback: the origin's region, else the last region, else Content (`D/view/workspace/focus.rs:320-333`) |
| H7 | A dialog opened over the sheet (for example Unlock key) closes | the control that held focus when it opened (Root restores it) |
| H8 | A body control goes (a choice disabled while a save is out); a footer control goes by itself (Save turns to Retry) | `conflict-sheet`'s handle, through `focus_lost`; `footer_focus` |
| H9 | Tab and Shift+Tab, repeatedly | never leaves `sheet-host`: the close button, the body, the footer (K-20) |
| H10 | Something behind takes focus while it is open, the Kit's deferred restore of a dialog closed just before it opened included (`GC/root.rs:341-363`) | the sheet's own handle |
| H11 | Quitting while it is open | it closes; Quit now takes focus when Activity is displayed, as `wait_to_quit` says |
| H12 | A save's answer lands after the sheet closed (Escape first), with the sheet's entity still held | nothing moves; no sheet closes; no lifecycle dialog reopens (R-22, R-27) |
| H13 | Root's context menu over the Result editor | the menu keeps focus until it closes, then the editor |
| H14 | A save's answer lands while a dialog is over the sheet | nothing until that dialog closes; then H5 (R-26) |
| H15 | A focus move the workspace makes while it is open (a draft discarded behind it, typing in the Result editor) | unchanged: still in the editor (R-25) |
| H16 | Resume | the origin |

Origins: the banner's `review_focus`; the Activity list, or the footer's
Needs attention item for a popover row (as today); the lifecycle
dialog's origin.

"Inside the sheet" in a test is the walk's `in_sheet(id)`: focus inside
the element `id` and the Content region holding it (fact 32, R-19).

### B4. Header and body

- **Title slot** (`frame::sheet_header`): title "Choose what to keep ·
  {title}" (or "Choose what to keep"), id `conflict-title`; description
  as today, id `conflict-description`; no tile.
- **Body:** the `conflict-sheet` element keeps its focus,
  `RESOLVE_SHEET_CONTEXT`, `SaveResolution` and Escape handling and its
  own vertical scroll, drops its `max_h(px(720.))`, and is laid over the
  Kit's body, out of its flow, as the details sheet lays its body
  (`D/view/item/sheet.rs:254-278`), so the header and footer stay put
  (R-12). As today, Tab to a part below the fold does not scroll it into
  view (K-19). In order: the meta line (fact 15, unchanged),
  `conflict-note` (unchanged), the status block, the cards.

### B5. Cards

Each part's card (`05`, 2g), bordered, radius 6:

- **Header row:** the part's name (600) · its kind (muted) · its state ·
  then, right-aligned, the choices.
  - **Name** (`("conflict-field", k)`): the field's name in words (Part
    C), not its key **(owner's eye)** (decision 10); the body part stays
    "Description" (`("conflict-body", i)`).
  - **Kind** (`("conflict-field-kind", k)`, new): "Ticket detail",
    "Document detail" or "Comment detail", by the kind of the item at
    that path. `PathConflict` keeps the kind its parse already finds
    (`D/model/conflict/merge.rs:129`) and gives it as
    `PathConflict::item_kind()`. The body part has no kind label: `05`
    names a body *region* and calls its kind "Description", but the body
    is one part here, already named "Description" (K-14).
  - **State** (`("conflict-choice", k)`, `("conflict-body-choice", i)`):
    "Choose one" (amber), "Chosen" (green), or "Edited" (green) once the
    Result was typed in **(owner's eye)** (decisions 5 and 10).
  - **Choices:** Keep mine, Keep theirs, and Keep both for the body, as
    today. The chosen one keeps the Kit's `selected` style and shows a
    check icon before its label, so the choice is not told by colour
    alone (Wave 04, "colour is never the only signal") (K-8).
- **Columns:** Before (with "last shared copy", muted, after the label),
  Yours, Theirs; three equal columns, mono lines. Lines keep their
  gutter inside the text (fact 16, R-13).
- **Body part:** the Result editor, its stop and note, as today (D5 R-27,
  R-31, R-34).
- **Taken rows:** "{name}: …", the name as above.
- **Outside cards** (an external set): one card per path, in the same
  frame, with its reason; Open folder and Check again stay one shared row
  below the cards, since both act on the whole set (R-14).

### B6. Footer

`frame::sheet_footer` with `Footer { hint: None, buttons }`, inside
`conflict-footer` (B2), where `buttons` is, in order:

1. the Git line (`conflict-git`, unchanged text), drawn with `flex_1`,
   `min_w_0` and `truncate` so it fills the left and is cut short (R-15);
2. Cancel (`conflict-cancel`);
3. exactly one of today's primary chain (`render.rs:448-487`): Resume,
   Retry, the lost line, or **Save resolution**, enabled only when every
   part is chosen; none for an external set without Resume.

Cancel before Save is decision 4 **(owner's eye)**; Tab reaches Cancel,
then Save resolution (E-12).

## Part C: Copy

From the D5 follow-ups ticket (`01M6RAS63EWVHWWVBSZ836RXG2`, decision 5)
and decision 12:

1. **One name for the way into Resolve conflict:** "Review changes…" for
   the banner, Activity's rows and Needs attention (`REVIEW_CHANGES`,
   `D/model/activity/copy.rs:40`) and the lifecycle's Resolve conflict…
   (`RESOLVE_CONFLICT`, `D/view/lifecycle.rs:119`, which goes). Repair
   locally… keeps its name: it does something first. The sheet's title
   stays "Choose what to keep".
2. **The ellipsis rule** (`06`: "…" on an action that opens a
   confirmation or sheet; decision 12): "Open Sharing & keys…",
   "Check host…" (`OPEN_SHARING_AND_KEYS`, `CHECK_HOST`,
   `D/model/activity/copy.rs:52-53`) and "Open repository details…"
   (`OPEN_REPOSITORY_DETAILS`, `D/view/lifecycle/outcome.rs:67`),
   wherever they appear: the lifecycle dialog, Activity's rows, and a
   comment's offers, whose literal labels (`D/view/item/posting.rs:389-390`)
   take the constants.
3. **Keep mine beside Yours:** the buttons keep "Keep mine", "Keep
   theirs", "Keep both" and the columns "Yours" and "Theirs", as `05`
   draws them; the state words "Keeping yours" and the like give way to
   `05`'s "Chosen", and "Edited" for a typed Result (`KEEPING_*` go).
4. **Shared label constants** (R-16), each defined once in
   `D/model/activity/copy.rs` (or the file it already re-exports) and
   re-exported or called where the copies are today:
   - "Resume": the model's (`D/model/activity/copy/states.rs:18`); the
     copies at `D/view/lifecycle.rs:103`, `D/view/conflict.rs:78`,
     `D/model/edit/save_state.rs:11`, the literal at
     `D/model/operations/recovery.rs:293` and the test-only
     `D/view/operations.rs:48` re-export or use it;
   - "Keep mine": new in `copy.rs`; `D/view/conflict/render.rs:43` and
     `D/model/review.rs:28` re-export it;
   - `share_main(primary)`: new in `copy.rs`; `D/view/lifecycle.rs:113-115`,
     the step title at `D/view/lifecycle/outcome.rs:122` and
     `Intent::label_in` (`D/services/requests.rs:410-413`) call it.

Field names (B5), as the details sheet and the header's fields name them
(`D/view/item/details.rs:222-229`, `D/view/item/sheet/ticket.rs:35-38`):

| Key | Name |
| --- | --- |
| `id` | ID |
| `title` | Title |
| `type` | Type |
| `status` | Status |
| `project` | Project |
| `team` | Team |
| `closed_at` | Closed at |
| `closed_by` | Closed by |
| `item_id` | Item |
| `parent_id` | Reply to |
| `created_at` | Created at |
| `created_by` | Created by |
| any other key | the key as written |

`fn field_name(key: &str) -> Cow<'static, str>` lives in a new
`D/model/conflict/names.rs`.

New words, each a constant: the stage titles (A1); "Closed by {name}
· today" and "Closed by {name}" (A4); the running lead; the two hints;
"Not now"; "Done" for the dismiss button; "Now
ready to start"; "Ticket detail", "Document detail", "Comment detail",
"last shared copy", "Chosen" and "Edited" (B5).

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Each stage's header title, for both commands (A1) | `each_stage_has_its_title` (new, table-driven) |
| P2 | Each stage's footer: the buttons in drawn order, the dismiss label and the hint; none but Stop while running; after Stop is pressed, Tab and Shift+Tab stay in the dialog and Escape still hides it (A5, A7, R-23) | `each_stage_orders_its_footer` (new, table-driven); Tab reaches them in that order |
| P3 | A click on the backdrop hides the dialog only while running; the run goes on; focus returns to the origin (A5) | `the_backdrop_hides_only_a_running_dialog` (new) |
| P4 | Step rows: state words on the marks; the running step's stage words, with and without the preview; a failed step's message; the message in the box with no failed step (A3) | `steps_show_their_state_and_progress` (new); `running_lights_each_step_from_its_stage` unchanged |
| P5 | Resolve conflict is a sheet the workspace keeps, through every way in; each way out clears the owner, Resume's included; the lifecycle's way in does nothing under a prompt (B1–B3, R-24) | `resolve_is_a_sheet_the_workspace_keeps` (new) |
| P6 | Focus rows H1–H16 (B3) | `focus_around_the_resolve_sheet` (new, table-driven, one row per H) |
| P7 | Nothing opens over or acts behind it: Ctrl+, , `show_settings`, `show_add_repository`, a details sheet (dropped, not kept for later), a workspace key, an Activity action; it opens over neither a dialog nor a sheet; Republish… waits for it on both paths; Ctrl+, right after Escape opens Settings (B1, B2, decision 9, R-18) | `nothing_opens_over_the_resolve_sheet` (new) |
| P8 | Field names, kinds, Chosen and Edited; an external set's shared row (B5) | `parts_are_named_in_words` (new); the `field_name` unit test |
| P9 | Copy (Part C, A1–A7) | `close_and_resolve_copy_uses_the_vocabulary` (new copy table) |
| P10 | No item text in the moved headers and footers (fact 33) | the existing a16 privacy tests, unchanged: the slots are swept as "drawn", and item text is allowed by id only |
| P11 | D5's behaviour: stale confirmations, Retry reusing the operation, the safe-point stop, no automatic winner (Wave risk row) | D5's existing tests, unchanged in what they assert about behaviour |
| P12 | No passphrase or private key text in any label or Git line of either surface (Wave exit); this Cycle adds no tooltip | `close_and_resolve_show_no_secret` (new), reading every label and value as `passphrase_tests`' `window_text` does (`D/view/passphrase_tests.rs:537-547`): a key imported with `Fixture::import_key`, whose private file holds `PRIVATE_KEY_SENTINEL` (`D/test_support/fixtures.rs:908`, `:940-985`); a close answered `unlock_required`, then Unlock… opens the prompt, where a sentinel passphrase is typed and supplied, as `credential_tests`' `ask` and `a_resend_without_a_secret_takes_the_unlock_path` drive it (`D/view/lifecycle/credential_tests.rs:157`, `:204`); Retry, then Done, by canned answers; then a remote conflict's sheet. At each stage no label or value in the window, the dialog's and sheet's slots included, holds either sentinel |
| P13 | A close and an approval that fail, retried by keyboard to Done, focus back on the origin (Wave exit) | `a_failed_run_is_retried_by_keyboard` (new, both commands): the library's failure point `BeforeRequestDomainCall` fails the run with `internal_error` and offers Retry, as `outcome_tests::a_stopped_run_with_no_record_offers_prepare` sets it up (`D/view/lifecycle/outcome_tests.rs:299-323`); the normal service is put back without moving the branch; Tab to Retry, Enter, then Done by Enter; focus on the origin |

## Testing

- **Layers.** All new tests are desktop tests in existing modules, in
  `TestAppContext` with the existing `Shell` and fixtures; no new target,
  no SSH case (the Cycle touches no transport). Per `AGENTS.md`, they test
  behaviour: titles, labels, order, enablement, focus and what a service
  receives, not colours or layout.
- **New tests:** P1–P4 in `D/view/lifecycle/stages_tests.rs` (new module
  under `lifecycle`); P5–P8 in `D/view/workspace/resolve_sheet_tests.rs`;
  P9 in `D/view/conflict/copy_tests.rs`; P12 in
  `D/view/lifecycle/privacy_secret_tests.rs`; P13 in
  `D/view/keyboard_tests/lifecycle.rs`.
- **Focus is read after a draw** (`shell.run(cx)`, then
  `Shell::holds_focus`/`holds_control`), as C4 and C5a do.
- **Drawn order** (P2) is the ids inside `lifecycle-footer` (or
  `conflict-footer`) sorted by their bounds' left edge (fact 34), not by
  `row_elements`.
- **The backdrop** (P3) is clicked with `simulate_click` at a point
  outside the 580 px dialog and below the title bar
  (`D/view/harness.rs:315-321`; the Kit ignores clicks above
  `TITLE_BAR_HEIGHT`).
- **Fast tier** at review-ready; no SSH target (`AGENTS.md`: desktop-only).

## Named Edits To Pre-Existing Tests

From the test sweep at `ac50ba1a` and the reviews (U-6). Each edit
changes copy, an id, an order or how the sheet is opened or read, never
what a behaviour test asserts about behaviour. Paths are under `D/view/`
unless named. Edits the sweep missed are pre-authorized by decision 11.

- **E-1** (Task 5) `conflict_tests.rs` `open_sheet_in` (`:100-127`):
  opens through `Workspace::show_resolve` (B2, R-9). Its 33 callers do
  not change.
- **E-2** (Task 5) `conflict::sheet_tests::the_note_follows_the_source`
  (`conflict/sheet_tests.rs:35`) and
  `conflict::answer_tests::a_dismissed_refusal_is_not_another_windows_set`
  (`conflict/answer_tests.rs:482`): `WindowExt::close_dialog` becomes
  `close_sheet`.
- **E-3** (Task 2) `lifecycle::tests::the_header_names_item_repository_and_context`
  (`lifecycle_tests.rs:525-580`): the `lifecycle-title` asserts read the
  Ask titles ("Close {SLUG}?", "Approve and merge {short}?"); the
  `lifecycle-branch` assert (`:547`) reads the branch inside
  `lifecycle-git` (this test's dialog is on confirm, where it is drawn
  today). The
  `lifecycle-kind` asserts (`:538`, `:568`) go. **Assertion removal:**
  the title's command words now say the kind (A2).
- **E-4** (Task 2) `lifecycle::outcome_tests::stop_at_a_safe_point_names_completed_steps`
  (`lifecycle/outcome_tests.rs:224`): the `lifecycle-branch` assert reads
  the branch inside `lifecycle-git`, which the stopped run now draws from
  the preview it was confirmed with (A6; Task 2 draws the Git line on
  running and ended stages, as it removes the branch line).
- **E-5** (Task 2) `lifecycle::outcome_tests::the_operation_entry_reopens_the_dialog`
  (`:484-485`): the title reads "Stopped at a safe point".
- **E-6** (Task 3) `lifecycle::confirm_tests::closure_shows_closed_by_from_the_preview`
  (`lifecycle/confirm_tests.rs:292-293`) and
  `::existing_closure_and_already_contained_read_already` (`:437`): the
  first reads "Closed by {name} · today"; the second, an existing
  closure, "Closed by Ada Example" (A4).
- **E-7** (Task 1) `conflict::entry_tests::resolve_conflict_opens_the_childs_sheet`
  (`conflict/entry_tests.rs:406-407`): `RESOLVE_CONFLICT` becomes
  `REVIEW_CHANGES` (Part C item 1). The button's id stays.
- **E-8** (Task 1) the activity copy tables, whose literal labels become
  the new words: "Review changes" in
  `D/model/activity_tests/tables.rs:113-114`,
  `D/model/activity_tests/states_pending.rs:174`, `:197` and
  `D/model/activity_tests/attention_states.rs:194`; "Open Sharing & keys"
  and "Check host" in `tables.rs:133-142` (decision 12).
- **E-9** (Task 1) `conflict::tests::save_is_disabled_until_every_part_is_chosen`
  (`conflict_tests.rs:328-329`), `::choices_wait_while_a_save_is_out`
  (`:669-675`), `conflict::stale_tests::a_stale_observation_refreshes_the_comparison`
  (`conflict/stale_tests.rs:81-82`): `KEEPING_*` becomes `CHOSEN`, and
  each asserts which choice was made through `ResolveSheet::choice_of`
  (R-20).
- **E-10** (Task 7) `conflict::tests::save_is_disabled_until_every_part_is_chosen`
  (`conflict_tests.rs:300`): `["title"]` becomes `["Title"]`.
- **E-11** (Tasks 1 and 5) `keyboard_tests/conflict.rs`'s walk:
  `choose_by_keyboard` (`:75-83`) asserts `CHOSEN` and the choice through
  `choice_of` (Task 1); `in_dialog` becomes `in_sheet` (R-19) at every
  call in `resolve_a_local_conflict_by_keyboard`,
  `resolve_a_shared_conflict_by_keyboard` and
  `sheet::{check_again_by_keyboard_keeps_focus_in_the_sheet,
  edit_the_result_by_keyboard, ctrl_enter_with_a_part_unchosen_keeps_focus_in_the_editor}`
  (Task 5). The walk's Tab loops need no other edit: Tab from the
  sheet's handle reaches the first choice (H1).
- **E-12** (Task 7) `keyboard_tests::conflict::sheet::edit_the_result_by_keyboard`
  (`:134-135`): Tab from the Result stop reaches `conflict-cancel`, then
  `conflict-save` (decision 4).
- **E-13** (Task 1) `desktop::assets::tests::every_item_view_icon_loads`
  (`D/assets_tests.rs:8`): `ICONS` gains `circle-x` and `git-merge` (58 to
  60). `circle-x` already loads from the Kit's default bundle; listing it
  is harmless.
- **E-14** the `lifecycle-ending` waits and reads need no edit (R-17).
  Listed so a reviewer does not look for them: the sweep found about 25.
- **E-15** the a16 privacy tests need no edit (P10, fact 33).

Sites checked that need no edit: the `lifecycle-cancel` tab in
`keyboard_tests::lifecycle::Walk::open_the_dialog` (`:170`, order-free);
`STARTED` (no test reads it); the step-note count (confirm only);
`MADE_READY_LEAD` (compared as a constant); `lifecycle-stopping`
(`keyboard_tests/lifecycle.rs:237`, `operations/republish_stop_tests.rs:120`,
`:181-185`, `lifecycle/outcome_tests.rs:126`: still drawn, A5);
`settings_sheet_tests/keyboard.rs:200`'s `tab_reaches` count (Open
Sharing & keys moves earlier); the tests that open the lifecycle dialog
after the sheet (`answer_tests.rs:366-387`, `followup_tests.rs:190-196`,
`lifecycle_resume_tests.rs:138-151`, `repair_tests.rs:144-169`,
`:273-280`, `entry_tests.rs:417`, `privacy_tests.rs:486`), which Task 5
runs.

## Document Amendments

1. **The Wave** (`docs/Waves/wave-04-workspace-design.md`, Cycle 05b): in
   scope, "the files to be merged with their tags and line counts" and
   "Becomes ready to start" amended by decisions 1 and 2; "Running shows
   each step's state and progress" reads "its stage" (fact 27). Applied
   in Task 0.
2. **`05`, as built** (the approval commit):
   - under the common frame's as-built notes, that the lifecycle dialog
     and Resolve conflict also put the action last (decision 4), and
     that Resolve conflict is a sheet that closes on Escape, its close
     button, Cancel, Resume, a save or quitting, not its scrim (decision
     9);
   - under "Close ticket dialog": the step order and Share main (D5
     R-8); no line counts; no ready list before the close; the stage
     words as progress; the message as the cause; the Expires line; the
     Waiting stage and the dismiss labels; the backdrop rule; the
     `GitMerge` tile for Approve and merge; the ending word in the box
     (R-17); the file count in parentheses (K-9); message-only states
     (K-10);
   - under "Resolve conflict sheet": no names or times, no base, Keep
     theirs; the Result editor and its note (D5 R-27, R-34); the gutter
     in the text (R-13); the Kit's selected style with a check (K-8);
     "Edited"; the shared outside actions; the scrim;
   - in the Needs attention popover's example, "Review changes…".
3. **`08`** (the approval commit): the Sheets row notes that Resolve
   conflict is a sheet the workspace owns (`SheetOwner::Resolve`); the
   Banners, outcome boxes row notes that the lifecycle's outcome box is
   the app's own, not `Alert`.
4. **`06`** (the approval commit): the writing rule's example action
   reads "Review changes…".
5. **Cycle 03b's copy** (the approval commit, a dated note in its
   design's Part B): "Review changes…", "Open Sharing & keys…" and
   "Check host…" (decision 12).
6. **Cycle 05a's design** (Task 0, once 05a has merged: its branch is
   still moving, so an edit now would meet its own; a dated note under B2
   and B3): `show_add_repository` also refuses while Resolve conflict is
   open, and `add_closed`'s return to Settings waits while it is
   (decision 9, U-9).

## Rulings

Each: decision — why — cost if wrong.

- **R-1:** one `stage()` drives the header, the footer and the backdrop —
  three readers of one state cannot disagree — a second match to keep in
  step.
- **R-2:** step marks keep `("lifecycle-step-state", ix)` and today's
  words as their label — tests and screen readers keep reading the state
  — none.
- **R-3:** "Closed by {name} · today": `closed_at` is taken as the close
  runs, which is now; the confirmation expires well within the day — a
  confirmation left open across midnight says "today" for tomorrow.
- **R-4:** the Expires line stays — it tells why a confirmation went
  stale; `05` does not draw it — one more line than the frame.
- **R-5:** the backdrop hides only in the Running stage — `05` says so
  for running only; a click elsewhere would drop a confirmation or a
  failure's actions — none.
- **R-6:** withdrawn: the footer order is decision 7's, with A7's table.
- **R-7:** withdrawn: the scrim is decision 9's.
- **R-8:** the footer wrapper carries `RESOLVE_SHEET_CONTEXT` and
  `SaveResolution` — Ctrl+Enter saves from the buttons today — none.
- **R-9:** every way in goes through `show_resolve` — the owner must be
  set in the same update that opens the sheet, and `sync_sheet` closes an
  unowned sheet at the next render — a route event and a refactor of
  `Entry`.
- **R-10:** withdrawn: the refusals are decision 9's.
- **R-11:** `resolve_open` is a predicate of its own, with
  `keep_focus_in_resolve` aiming at the sheet's own handle; C4's
  `settings_open` stays Settings' — widening `settings_open` would aim
  `keep_focus_in_settings`, `settings_keeps_lost_focus` and `focus_content`
  at Settings' page entry (`settings_sheet.rs:270-334`,
  `content.rs:598-600`) — none.
- **R-12:** the body keeps today's scroll of its own, laid over the
  Kit's body as the details sheet lays its own — the header and footer
  stay put, and nothing is lost: today's dialog does not reveal a
  focused part below the fold either (K-19) — a `ContentScroll` region
  with markers later, as the details sheet and Settings have.
- **R-13:** the gutter stays inside the line's text — in mono it already
  reads as a column, and the tests read lines with it — a column of its
  own later.
- **R-14:** Open folder and Check again stay one row — both act on the
  whole set; per card they would repeat — differs from `05`'s sketch,
  which is "not designed visually".
- **R-15:** the Git line is the footer's first element, filling the left
  — `Footer`'s hint is a fixed string — none.
- **R-16:** shared constants live in `D/model/activity/copy.rs` — the
  model already holds `REVIEW_CHANGES` and `RESUME` there (Cycle 03b
  R-41) — none. `D/model/review.rs`'s `KEEPING_MINE` ("Keeping mine"),
  `CHOOSE_ONE` and `REVIEW_TITLE` belong to the item's own review of a
  remote change, not this sheet, and stay.
- **R-17:** `lifecycle-ending` stays as the outcome box's first line —
  removing it would touch about 25 tests and four helpers that wait on
  it, and the privacy sweeps read its words inside the dialog — the stage
  is said twice.
- **R-18:** an automatic opener that waits for no dialog also waits
  while `resolve_open` — today the Resolve dialog held it back; over the
  sheet it would cover the choices — a Republish… shown later than it
  could be.
- **R-19:** the conflict keyboard tests hold focus "in the sheet" with
  a walk helper `in_sheet(id)`: focus inside `id` and the Content region
  holding it — `in_dialog` cannot hold for a sheet — none.
- **R-20:** a test tells Keep mine from Keep theirs by the sheet's model
  (`ResolveSheet::choice_of`, test-only), since both states read
  "Chosen" — the state word no longer says which — none.
- **R-21:** `show_resolve` refuses over any sheet rather than dropping a
  details sheet — no way in can fire with one open — none.
- **R-22:** `ResolveSheet::close` (Cancel, Resume, `applied`) closes the
  sheet only while the owner is this sheet (`close_resolve`) — an answer
  that lands after Escape must not close a sheet opened since — none.
- **R-23:** every stage of the lifecycle dialog keeps a Tab stop; after
  Stop, "Stopping…" is an inert one — with none, Root's Tab handler lets
  focus out of the dialog, and Escape, bound inside it, stops working
  (`GC/root.rs:494-520`, `gpui-base-0.6.6/src/dialog.rs:552-559`) — none.
- **R-24:** the lifecycle's way into the sheet goes on only while focus
  is inside the lifecycle dialog: a press checks before `on` moves focus
  and is dropped otherwise; Repair locally…'s sheet, opened when an
  answer lands, waits on the dialog until focus is back inside it — the
  Kit cannot name the top dialog, `in_workspace` closes whichever is on
  top, and closing a prompt opened over the dialog would drop it
  unanswered — a press in the frame a prompt opens is lost; the waiting
  sheet opens late.
- **R-25:** one predicate, `workspace_sheet_open` (05a's `sheet_held`,
  or `resolve_open`), gates the workspace's own focus moves
  (`focus_content`, `apply_review`'s refocus); `apply_review` keeps its
  requests until the sheet closes — C4 skips for Settings alone, and
  05a's G19 and C4's F18 promise focus unchanged under their sheets —
  one more caller to keep in step.
- **R-26:** an `applied` that lands under a dialog waits in the
  Workspace's `resolve_waiting`, drained from its render through
  `cx.defer_in` once no dialog is open, then closes, focuses and reopens
  — closing the sheet under a prompt would focus behind it, and the
  reopened lifecycle dialog would stack above the prompt; 05a's
  `add.waiting` waits the same way — a close shown late.
- **R-27:** `close_resolve` returns whether it closed; on false, nothing
  more runs — a test holds the entity, so `applied` can run after the
  sheet closed, and production may one day hold it too — none.
- **R-28:** the sheet is `min(920 px, window width − 48 px)` wide — the
  Kit does not clamp a sheet, and the app sets no minimum window size,
  so a narrow window would hide its left side — a narrower sheet than
  `05` draws in a small window.

## Known Limits

- K-1 No line counts per file (decision 1, fact 25).
- K-2 No ready list before the close (decision 2, fact 26).
- K-3 Progress is the stage's words, not counts (fact 27).
- K-4 A failure's cause is the library's message, which names no key
  (fact 28).
- K-5 "today" is assumed, not read (R-3).
- K-6 No author names or times; "Keep theirs" and "Theirs" (fact 30).
- K-7 The Git line names ours and theirs, no base (fact 30).
- K-8 The chosen choice is the Kit's `selected` style with a check, not
  `05`'s teal fill.
- K-9 The files heading keeps its count in parentheses.
- K-10 A submitted request shown as a message only takes the Ask title
  (A1).
- K-11 Resolve's scrim does nothing (decision 9).
- K-12 Outside cards share one row of actions (R-14).
- K-13 Facts 25 and 30 go to the follow-up library ticket
  `01M7FBRBN04QACSCZV6KX0WMR3` (decision 1).
- K-14 The body is one part named "Description", not `05`'s regions, so
  its card has no kind label (B5).
- K-15 A run shown without its preview (reopened from Activity, or after
  a restart) draws no Git line and no step notes but the running step's.
- K-16 Ctrl+Enter does not save while focus is on the Kit's close button,
  which sits outside `RESOLVE_SHEET_CONTEXT`.
- K-17 The Git line is cut short with no tooltip, as today.
- K-18 The backdrop flag is read when the dialog is drawn: a click in
  the frame after a run ends hides the ended dialog. On Windows and
  macOS the Kit marks the backdrop as a window-drag area, which may take
  the click; the on-screen checklist checks it there.
- K-19 Tab to a part below the fold does not scroll it into view, as
  today (R-12).
- K-20 Root's focus trap tries 100 stops: with more Tab stops than that
  behind the sheet, a wrap from the last control can land behind it,
  and the keep-focus check then brings focus back to the sheet's handle,
  not to the close button.
- K-21 Quitting closes the sheet in the window that quits, as it closed
  the dialog; another window's sheet stays until that window quits.

## Unverified

- **U-1:** Cycle 05a's names after it merges: `SheetOwner::AddRepository`,
  `sheet_held`, `keep_focus_in_sheet`, `sheet_keeps_lost_focus`,
  `add.waiting` and `run_waiting_add_close`, `sheet_footer` and
  `Tone::Accent` in use, `wrapped_kit`, `show_add_repository`. Read at
  `d7b39f62` (05a's Task 6) for revision 4; Task 0 reads the merged code.
- **U-6:** the named edits, from the sweep at `ac50ba1a`. Task 0
  re-checks them against 05a's merged code.
- **U-7:** the classification of fact 31's sites (R-18, R-25), with
  `apply_review` (whose identity request `show_settings` would now
  refuse after it is taken, `content.rs:145-147`) and `close_with_draft`
  and `open_created` (`content.rs:166`, `:503-510`). Task 0.
- **U-9:** Cycle 05a's own openers that wait for no dialog (its
  `add_closed` step 3) also wait while `resolve_open`. Task 0, once 05a
  has merged.
- **U-10:** Repair locally…'s waiting sheet (R-24) opens at the
  lifecycle dialog's first render with focus back inside it. Task 5.

Checked in review round 1 (source review, 2026-11-15) and no longer
open: U-2 (the Result editor's Escape wins over the Kit sheet's, as over
the dialog's), U-3 (a footer slot element takes a key context and
action, as the details sheet's does), U-4 (the dialog's overlay can be
clicked in a test), U-5 (the Kit `Spinner` runs in a test window, as
`D/view/item/pill.rs:234` does), U-8 (a privacy sweep files an element
under its own id).

## Alternatives Rejected

- A whole-file Result preview (`05`): it would replace D5 R-27's editor,
  a behaviour change.
- Per-card outside actions (R-14).
- Keeping Resolve conflict a dialog (decision 3).
- Line counts, authors and the base in this Cycle (decision 1).
- A provisional ready list at confirm (decision 2; D5 decision 6).
- Widening `settings_open` to cover Resolve (R-11).
- The Kit's own sheet-body scroll (R-12).
- A `ContentScroll` region with markers, which would reveal a focused
  part below the fold: new behaviour beyond today's dialog (R-12, K-19).
- Moving focus to the sheet's handle on every press, body included: Tab
  would restart at the first choice after each choice (B3).

## Risks

| Risk | Mitigation |
| --- | --- |
| The sheet slot's interplay (Settings, Add repository, details, dialogs over it) | B1–B3 follow C4 and C5a; P5 is written first in Task 5, P6 (H1–H16) and P7 first in Task 6, under a design checkpoint |
| A D5 behaviour test edited into meaning less | Named edits change copy, ids, order or the open helper only; the reviewer of each task checks each edit |
| 05a's Tasks 4–10 change what this design cites | Task 0 re-maps (U-1); execution waits for 05a's merge |

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning research, 2026-11-15 | The lifecycle dialog, Resolve conflict and the library's data, read at `ac50ba1a`; the test sweep | Facts | Baseline Facts; Named Edits |
| Owner, 2026-11-15 | Missing data; ready list; sheet or dialog; button order; D5 follow-ups; the outline | Owner decisions | Decisions 1–6 |
| Review round 1, source and feasibility (`702e569f`) | 1 Blocking: a control gone from the Kit's footer slot leaves focus on `sheet-host`. Important: P2 read no order; StillRunning in two stages; the Republish… scenario; `settings_open` would aim at Settings; P4's input; the Kit body's scroll | Blocking, Important, Minor | Revision 2: `footer_focus` and presses focusing the sheet (B3, H8); bounds order (fact 34); the Waiting stage (A1); fact 31, R-18; R-11; P4; R-12; and the Minors |
| Review round 1, contract and consistency (`702e569f`) | 2 Blocking: no proof for the passphrase exit item; no Failed-then-Retry keyboard journey. Important: the Git line in 2d/2e; StillRunning; "Stopping…"; A7's order as an owner decision; decision 1's follow-up; the ellipsis rule; colour alone for the choice; the close paths (quit, guarded closes, late answers); "inside the sheet"; scrolling; contract 5's proof and 05a's `show_add_repository`; amendments; unlisted test edits; privacy order; Task 5's size; P3 | Blocking, Important, Minor; owner questions | Revision 2: P12, P13; A5, A6; A7 into decision 7; K-13; decision 12; the check and "Edited" (B5); B3, H11–H13, R-22; `in_sheet`, H1–H13; R-12; P7, Amendment 6; Amendments 2–6; decision 11; fact 33; the plan's Task 5 split into Tasks 5 and 6 |
| Owner, 2026-11-15, after review round 1 | Further test edits; the ellipsis rule | Owner decisions | Decisions 11 and 12 |
| Review round 2, scoped re-review (revision 2, `874bb66b`) | 1 Blocking: P13's fixture offers no Retry. Important: H1's Tab order; presses in the body moving focus; the scroll named reveals nothing; E-4 left Task 2 red. Round 1's findings otherwise resolved | Blocking, Important, Minor | Revision 3: P13 on `BeforeRequestDomainCall`; H1; B3's footer-only rule; R-12, K-19; E-4 into Task 2; and the Minors (Resume, H12, copies, comment offers, "today" for an existing closure, P12, P9's words, stale text) |
| Review round 2, adversarial on the sheet and the dialog's stages (revision 2, `aaf6b475`) | 1 Blocking: after Stop, no Tab stop is left in the dialog. Important: H1's Tab order; an answer under a dialog closing the sheet; a second Republish… path; workspace focus moves behind the sheet | Blocking, Important, Minor | Revision 3: R-23; H1; R-26, H14; R-18's two sites; R-25, H15; and the Minors (Resume, R-27, K-18, R-28, `resolve_open` everywhere, R-24, K-20, K-21, H10) |
| Review round 3, scoped re-review (revision 3, `3964af7c`) | 0 Blocking, 2 Important, 7 Minor. Important: P12's fixture never prompts; R-24 had no mechanism and dropped Repair locally's late sheet. Round 2's findings resolved | Important, Minor | Revision 4, **not independently reviewed**: P12 on `import_key` and the prompt; R-24's focus check and wait (U-10); R-26's drain; 05a's `sheet_held`; H1 by set kind; Cancel through `close`; the copies; the Verify lines; Amendments 2 and 6 |
| Owner, 2026-11-15 | Approval (Cycle decision 13) | Owner decision | Approved at revision 4; Amendments 2–5 applied in the approval commit, 1 and 6 in Task 0 |
