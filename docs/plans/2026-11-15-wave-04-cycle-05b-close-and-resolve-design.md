---
title: "Wave 04 Cycle 05b Design: Close Ticket And Resolve Conflict"
date: 2026-11-15
status: "draft"
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
copy (decision 5 and Part C). It changes `src/desktop/` only. Draft:
nothing here is approved.

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
    (`settings_sheet.rs:93-96`), which C5a widens to "Settings or Add
    repository owns an active sheet" (C5a's ledger, Task 0).
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
    `GitMerge`. `ICONS` in `D/assets_tests.rs:8` lists 58 names
    (`every_item_view_icon_loads`). The Kit has `Spinner`
    (`GC/spinner.rs:10`).

31. **Dialog-only guards.** About 50 production sites in `D/view` check
    `window.has_active_dialog(cx)` without `has_active_sheet`. Most refuse
    a person's action (Done, Preview, Review changes, Resume, Open the
    lifecycle). The deferred Republish… opens by itself once no dialog is
    open (`D/view/operations/republish.rs:205-230`).
32. **Where a sheet's focus counts.** Root's sheet layer is drawn inside
    the Content region (`D/view/workspace/render.rs:114-131`), so focus in
    a sheet counts as the Content region's; Settings' keyboard tests
    assert `Region::Content` (`D/view/keyboard_tests/settings.rs:25`). The
    keyboard walk's `in_dialog` (`D/view/keyboard_tests/drafts.rs:73-87`)
    asserts that no region holds focus, which a sheet fails.
33. **The harness reads window-wide.** `Shell::label`, `present` and
    `rows` find an id anywhere in the window (`window.try_find`,
    `D/view/harness.rs:344-481`); the privacy sweeps file each labelled
    element under the first surface id in its path (`SURFACES`,
    `D/view/lifecycle/privacy_tests.rs:112-113`, with
    `("lifecycle-dialog", "dialog message")` and
    `("conflict-sheet", "sheet message")`).

### Data the library does not give

25. **No line counts.** The preview's paths carry only a change kind
    (`LifecyclePathChangeDto`, `src/repository/mutation/dto.rs:294-336`);
    nothing in the library computes diff stats (D5 R-3, K-7).
26. **No ready list before the close.** The library's made-ready report
    is read only after the merge (`src/repository/read/graph.rs:285-298`,
    `pub(super)`; `bind/lifecycle.rs:456-499`), and D5 decision 6 shows it
    on Done only (decision 2).
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

Decision 1 leaves facts 25, 27, 28 and 30 to a follow-up library ticket.

## Part A: The Lifecycle Dialog On The Frame

### A1. Stages

The dialog's phases group into five stages, which set the header:

| Stage | Phases | Close ticket title | Approve and merge title | Tile |
| --- | --- | --- | --- | --- |
| Ask | `Saving`, `Preparing`, `Confirm`, `Blocked`, `Held`, `Stale`, `Answered`, `Message`, `Repaired`; a submitted request shown as a message only (`Removed`, `NotSent`, `Dismissed`, a resume answered without lifecycle data) | "Close {short}?" | "Approve and merge {short}?" | `Archive` (close) or `GitMerge` (promote), Accent |
| Running | `Submitted` with no answer yet; an answer whose `Ending` is `StillRunning` | "Closing…" | "Merging…" | as Ask |
| Done | `Ending::Done` | "Ticket closed" | "Merged into {primary}" | `CircleCheck`, Success |
| Failed | `Ending::Failed` | "Couldn't finish closing" | "Couldn't finish merging" | `CircleX`, Danger |
| Stopped | `Ending::Stopped` | "Stopped at a safe point" | "Stopped at a safe point" | as Ask |

`{short}` is `Heading::short`; `{primary}` is `LifecycleDialog::primary()`
(R-35). `LifecycleDialog::stage(&self, cx) -> Stage` computes it from the
phase and, for `Submitted`, from what `submitted_body` reads, so the two
never disagree (R-1).

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
- `lifecycle-kind` and `lifecycle-branch` go. The tile replaces the kind
  icon; the branch is in the Git line (confirm) and the subtitle names
  the item. **(owner's eye)**
- `lifecycle-ending` stays, with today's words, as the outcome box's
  first line (A6): about 25 tests and four shared helpers wait on it or
  read it, and the privacy sweeps read its words inside
  `lifecycle-dialog` (R-17).
- The header holds no control (C5a A1), so nothing about focus or Tab
  changes. The harness reads ids window-wide (`Shell::label`, `present`
  and `rows` use `window.try_find`, `D/view/harness.rs:344-481`), so tests
  still find `lifecycle-title` and `lifecycle-subtitle` in the title slot.

`render` keeps the `lifecycle-dialog` element, which tracks the dialog's
own focus (fact 11), and draws `frame::body(stage body)` followed by
`frame::footer(..)` as its last child (C5a R-19), the footer element
carrying the id `lifecycle-footer` so a test reads its buttons in drawn
order (P2).

### A3. Step rows

One row drawer serves every stage, in the bordered list `05` draws:

- **The mark,** 20 px, left: the step's number in a muted circle on
  confirm, and on running and ended for a step not started or not done;
  `CircleCheck` (green, 18 px) for Done; a Kit `Spinner` (16 px, teal)
  for Running; `CircleX` (red) for Failed. On running and ended, the
  mark's wrapper keeps the id `("lifecycle-step-state", ix)` and is
  labelled with today's state words (`StepState::words`), so a test reads
  the same words it reads today and a screen reader hears them (R-2).
- **The title,** `("lifecycle-step", ix)`, red for a Failed step.
- **The note,** `("lifecycle-step-note", ix)`, drawn in every stage when
  the step has one (fact 5: a run shown without its preview has none):
  - running: the Running step's note is
    `stage_words(stage, primary)` (fact 27) for the request's last stage,
    and the step's own note before any stage is known **(owner's eye)**;
  - failed: the Failed step's note is the answer's message, with the id
    `lifecycle-message` (fact 8), in red; the step's own note is not
    drawn. With no Failed step, the message goes in the outcome box (A6).

### A4. Confirm

As today (fact 6), except:

- the Mark as closed note reads "Closed by {name} · today", where
  `{name}` is `closed_by` up to its " <" (the whole value when it has
  none) **(owner's eye)** (R-3);
- the footer: hint "Enter to confirm · Esc to cancel", then Cancel ·
  **{command}** (decision 4);
- the Expires line stays (`lifecycle-expires`), muted, under the Git
  line (R-4).

No line counts (decision 1), no "Becomes ready to start" (decision 2).

### A5. Running

- The lead goes above the steps: "You can keep working. This continues
  in the background and shows up in Activity." (`lifecycle-state`,
  replacing `STARTED`).
- The steps (A3).
- **Footer:** hint "Esc hides this window"; the only button is Stop at a
  safe point (`lifecycle-stop`), or "Stopping…" (`lifecycle-stopping`)
  once pressed, which leaves the footer with the hint only. There is no
  Close **(owner's eye)** (decision 4).
- **Backdrop:** clicking it hides the dialog, as Escape does: the run
  goes on and Activity keeps it. The builder sets
  `overlay_closable(dialog.read(cx).hides_on_backdrop(cx))`, true only in
  the Running stage (decision 4, R-5). Every other stage keeps today's
  inert backdrop.

### A6. Ended

Below the steps, an **outcome box** (radius 6, 1 px border in the tone,
the tone at 8% behind), holding today's ended lines in today's order
(fact 8), each with its id:

- Done: `CircleCheck`, green; Failed: `ShieldCheck`, red; Stopped and
  Running (`StillRunning`): `ShieldCheck`, amber.
- Contents: the ending word (`lifecycle-ending`: "Done", "Failed",
  "Stopped at a safe point", "Still running…", semibold; R-17); the
  cancellation and completed lines (stop); "It finished
  before it could stop." (done after Stop); the outcome lines; the
  blockers; the no-record line; the republish line; the guidance; "Still
  running…"; the message when no step shows it (A3); the code.

The made-ready part (fact 12) follows the box in its own teal-tinted
box, its lead reading "Now ready to start" (decision 2, Part C).

### A7. Footers

The rule (decision 4): the dismiss button first, then today's other
non-primary buttons in today's order, then today's primary buttons in
today's order (R-6). The dismiss button keeps the id `lifecycle-cancel`
and the behaviour of today's Cancel and Close (close, focus the origin):

| Stage, phase | Dismiss label | Then | Hint |
| --- | --- | --- | --- |
| Ask: `Saving`, `Preparing` | Cancel | — | — |
| Ask: `Confirm` | Cancel | **{command}** | "Enter to confirm · Esc to cancel" |
| Ask: `Stale` | Cancel | **Prepare again** | — |
| Ask: `Blocked` | Close | the way-out actions, **Prepare again** (not for `ticket_closed`) | — |
| Ask: `Held` | Close | Continue or Resume | — |
| Ask: `Answered` | Close | **Retry** (on `busy`) | — |
| Ask: `Repaired` | Close | **Prepare again** | — |
| Ask: `Message`, and a submitted message only | Close | — | — |
| Running | none | Stop at a safe point | "Esc hides this window" |
| Failed, Stopped | Not now | today's others, then **Retry**, **Unlock…**, **Prepare again** where offered and primary | — |
| Done | **Done**, last | today's others (Republish…, Open ready tickets) come first | — |
| `StillRunning` | Close | today's others | — |

Bold marks a primary button. On Done the dismiss button is the primary
and comes last, reading "Done". **(owner's eye)** for every row: today
the primary comes first and the row is left-aligned.

### A8. What does not change

Escape closes the dialog in every stage and cancels nothing (fact 2;
C5a decision 7). Enter does what it does today (fact 10). Every press
still focuses the dialog first (fact 11). Focus returns to the origin on
every close. The stale, Retry, Stop and made-ready behaviours are D5's.

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

and the test mirror `SheetOwnerKind` gains `Resolve`. `sync_sheet` gains
two cases:

1. **Owner `Resolve`, and a sheet is active:** kept, whatever item is
   shown.
2. **Owner `Resolve`, and no sheet is active:** the sheet closed (Escape,
   its close button, Cancel, a save, or the Kit). The owner is cleared.
   Focus is already where B3 put it.

A requested details sheet never opens while the owner is `Resolve`, as
C4's case 5 says for Settings.

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
   14), while any sheet is active (defensive: no way in can fire behind a
   sheet, fact 21), and, through `conflict::open`, while quitting or
   saving drafts (fact 13);
2. calls `conflict::open`, which now opens a Kit sheet on the right
   (`window.open_sheet`), `size(px(920.))`, `overlay_closable(false)`
   (R-7), with:
   - the title slot: `frame::sheet_header` (B4), wrapped in an element
     with id `conflict-header`;
   - the footer slot: `frame::sheet_footer` (B6), wrapped in an element
     with id `conflict-footer` that carries `RESOLVE_SHEET_CONTEXT` and
     the `SaveResolution` action, so Ctrl+Enter still saves from the
     footer's buttons (R-8);
   - the body: the `ResolveSheet` view;
   - `on_close`: focus the origin, as today;
   - the builder reads the entity at each render, as the lifecycle's
     does (fact 2);
3. sets the owner to `Resolve(sheet.downgrade())`.

**The ways in** (fact 14) all go through it:

| Way | Today | After |
| --- | --- | --- |
| Item banner, Review changes… | `conflict::open` in `ItemPane::review_conflict` | emits `ItemRoute::Resolve(Box<conflict::Opening>)`; `Workspace::open_route` calls `show_resolve` |
| Activity row or pending row, Review changes… (and Needs attention, which runs them) | `Entry::open` from `review_request`, `review_pending` | `Entry::opening(..) -> Option<Opening>`, then `self.show_resolve(opening)` |
| Lifecycle dialog, Repair locally… and Review changes… | `conflict::open` after `self.close` | `self.in_workspace(.., |ws, w, cx| ws.show_resolve(opening, w, cx))` (fact: `in_workspace`, `actions.rs:126-140`, closes the dialog first) |

`conflict::open` becomes `pub(in crate::desktop::view)` and its only
caller is `show_resolve` (R-9). The test helper `open_sheet_in`
(fact 14) opens through `show_resolve` (E-1).

**Settings while it is open.** `settings_key` refuses when the owner is
`Resolve`, and `show_settings` refuses too (defensive): opening Settings
would close the sheet and lose the choices, and today the Resolve dialog
stops both (fact 22, R-10).

**What waited for the dialog.** About 50 production sites check
`has_active_dialog` alone (fact 31). A site that refuses a person's
action needs nothing: nothing behind the sheet can be pressed (fact 21).
A site that opens something by itself waits while the owner is
`Resolve`, as it waited for the dialog (R-18). The one found is the
deferred Republish… (`republish_once_stopped` and
`open_deferred_republish`, `D/view/operations/republish.rs:211`, `:225`):
pressed from the lifecycle dialog, it waits for that dialog to close,
and the dialog's Review changes… closes it to open this sheet. Task 0
classifies every site again (U-7).

### B3. Closing and focus

The sheet closes by Escape (outside the Result editor), its header's
close button (new: the Kit's `x`), Cancel, or a successful save
(`applied`, `D/view/conflict/submit.rs:350-406`). Escape and the close
button are the Kit's and run `on_close`; Cancel and `applied` call
`window.close_sheet(cx)` (today `close_dialog`) and focus the origin
themselves, as today, since `on_close` runs only on the Kit's own paths
(C5a lesson). `applied`'s reopening of the lifecycle dialog is unchanged.
The scrim does not close it (R-7).

**Where focus goes** (contract; each row read after a draw, P-rows in
Testing):

| Row | Situation | Focus after |
| --- | --- | --- |
| H1 | Opened by any way in | inside the sheet; Tab reaches the first enabled choice. Root draws its sheet layer inside the Content region (`D/view/workspace/render.rs:114-131`), so, as for Settings, the Content region counts as holding it |
| H2 | Escape, not in the Result editor | the origin |
| H3 | Escape in the Result editor | the Result stop; the sheet stays open |
| H4 | Cancel, or the close button | the origin |
| H5 | A save that closes it | the origin, or the reopened lifecycle dialog when `applied` reopens it |
| H6 | The origin is no longer drawn when it closes | the origin's region, else the content region (`focus_lost`, fact 21) |
| H7 | A dialog opened over the sheet (for example Unlock key) closes | back inside the sheet |
| H8 | The focused control goes by itself (Save turns to Retry, a choice disables while saving) | inside the sheet (`focus_lost`, fact 21) |
| H9 | Tab and Shift+Tab, repeatedly | never leaves the sheet, or its dialog over it |
| H10 | Something behind takes focus while it is open | back inside the sheet: `keep_focus_in_settings`'s check, widened by C5a to Add repository, widens to `Resolve` (R-11) |

Origins: the banner's `review_focus`; the Activity list, or the footer's
Needs attention item for a popover row (as today); the lifecycle
dialog's origin.

### B4. Header and body

- **Title slot** (`frame::sheet_header`): title "Choose what to keep ·
  {title}" (or "Choose what to keep") with id `conflict-title`;
  description as today, id `conflict-description`; no tile.
- **Body**, the `conflict-sheet` element, which keeps its focus,
  `RESOLVE_SHEET_CONTEXT`, `SaveResolution` and Escape handling, but
  drops its own `max_h` and scroll: the Kit's sheet body scrolls (R-12).
  In order: the meta line (fact 15, unchanged), `conflict-note`
  (unchanged), the status block, the cards.

### B5. Cards

Each part's card (`05`, 2g), bordered, radius 6:

- **Header row:** the part's name (600) · its kind (muted) · its state ·
  then, right-aligned, the choices.
  - **Name** (`("conflict-field", k)`): the field's name in words (Part
    C table), not its key **(owner's eye)**; the body part stays
    "Description" (`("conflict-body", i)`).
  - **Kind**, new, `("conflict-field-kind", k)`: "Ticket detail",
    "Document detail" or "Comment detail", by the path's item kind. The
    body part has no kind label: `05` names a body *region* and calls its
    kind "Description", but the body is one part here, already named
    "Description" (K-14).
  - **State** (`("conflict-choice", k)`, `("conflict-body-choice", i)`):
    "Choose one" (amber) or "Chosen" (green) **(owner's eye)** (Part C).
  - **Choices:** Keep mine, Keep theirs, and Keep both for the body, as
    today. The selected one stays the Kit's `selected` style (K-8).
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

`frame::sheet_footer` with `Footer { hint: None, buttons }`, where
`buttons` is, in order:

1. the Git line (`conflict-git`, unchanged text), drawn with `flex_1`,
   `min_w_0` and `truncate` so it fills the left and is cut short (R-15);
2. Cancel (`conflict-cancel`);
3. the primary, as today: Resume, Retry, the lost line, or **Save
   resolution**, enabled only when every part is chosen.

Cancel before Save is decision 4 **(owner's eye)**; Tab now reaches
Cancel, then Save resolution (E-list).

## Part C: Copy

From the D5 follow-ups ticket (`01M6RAS63EWVHWWVBSZ836RXG2`, decision 5):

1. **One name for the way into Resolve conflict:** "Review changes…",
   with `06`'s ellipsis on an action that opens a sheet, for the banner,
   Activity's rows and Needs attention (`REVIEW_CHANGES`,
   `D/model/activity/copy.rs:40`) and the lifecycle's Resolve conflict…
   (`RESOLVE_CONFLICT`, `D/view/lifecycle.rs:119`, which goes). Repair
   locally… keeps its name: it does something first. The sheet's title
   stays "Choose what to keep".
2. **Keep mine beside Yours:** the buttons keep "Keep mine", "Keep
   theirs", "Keep both" and the columns "Yours" and "Theirs", as `05`
   draws them; the state words "Keeping yours" and the like give way to
   `05`'s "Chosen" (`KEEPING_*` go).
3. **Shared label constants:** one definition each of "Resume", "Keep
   mine" and `share_main`, in `D/model/activity/copy.rs` beside
   `REVIEW_CHANGES`, re-exported where today's copies are (R-16).

Field names (B5):

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

New lifecycle words (A1–A7): the stage titles; "Closed by {name} ·
today"; the running lead; the two hints; "Not now"; "Done" for the
dismiss button; "Now ready to start".

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Each stage's header title, for both commands (A1) | `each_stage_has_its_title` (new, table-driven) |
| P2 | Each stage's footer: the buttons, in order, with the dismiss label; none but Stop while running (A7) | `each_stage_orders_its_footer` (new, table-driven); Tab reaches them in that order |
| P3 | The backdrop hides the dialog only while running, and the run goes on (A5) | `the_backdrop_hides_only_a_running_dialog` (new) |
| P4 | Step rows: state words on the marks; the running step's note is its stage words; a failed step's note is the message (A3) | `steps_show_their_state_and_progress` (new); `running_lights_each_step_from_its_stage` unchanged |
| P5 | Resolve conflict is a sheet the workspace keeps, through every way in (B1, B2) | `resolve_is_a_sheet_the_workspace_keeps` (new) |
| P6 | Focus rows H1–H10 (B3) | `focus_around_the_resolve_sheet` (new, table-driven, one row per H) |
| P7 | Nothing opens Settings or a details sheet over it; it does not open over a dialog or sheet; a deferred Republish… waits for it to close (B2, R-10, R-18) | `nothing_opens_over_the_resolve_sheet` (new) |
| P8 | Field names, kinds and the Chosen state (B5, Part C) | `parts_are_named_in_words` (new) |
| P9 | Copy (Part C, A1–A7) | `close_and_resolve_copy_uses_the_vocabulary` (new copy table) |
| P10 | No item text in the new header and footer elements, and none of the existing privacy guarantees lost (Wave exit) | the privacy tests, their surfaces extended to `conflict-header`, `conflict-footer` and the lifecycle's title slot (E-list) |
| P11 | D5's behaviour: stale confirmations, Retry reusing the operation, the safe-point stop, no automatic winner (Wave risk row) | D5's existing tests, unchanged in what they assert about behaviour (named edits change copy, ids and order only) |

## Testing

- **Layers.** All new tests are desktop tests in existing modules, in
  `TestAppContext` with the existing `Shell` and fixtures; no new target,
  no SSH case (the Cycle touches no transport). Per `AGENTS.md`, they test
  behaviour: titles, labels, order, enablement, focus and what a service
  receives, not colours or layout.
- **New tests:** P1–P4 in `D/view/lifecycle/stages_tests.rs` (new module
  under `lifecycle`); P5–P8 in `D/view/workspace/resolve_sheet_tests.rs`;
  P9 in `D/view/conflict/copy_tests.rs`.
- **Focus is read after a draw** (`shell.run(cx)`, then
  `Shell::holds_focus`/`holds_control`), as C4 and C5a do. "Inside the
  sheet" is the walk's `in_sheet` (R-19): focus inside `conflict-sheet`,
  `conflict-header` or `conflict-footer`, and the Content region holding
  it.
- **Footer order** (P2) is read from the drawn order of the ids inside
  `lifecycle-footer` (or `conflict-footer`), as `confirm_tests`'
  `row_elements` reads a path row (`lifecycle/confirm_tests.rs:62`).
- **The backdrop** (P3): a click on the overlay at a point outside the
  dialog, if the harness can place one (U-4); else `hides_on_backdrop`
  per stage and the builder's use of it, which the stage table tests.
- **Fast tier** at review-ready; no SSH target (`AGENTS.md`: desktop-only).

## Named Edits To Pre-Existing Tests

From the test sweep at `ac50ba1a` (U-6). Each edit changes copy, an id,
an order or how the sheet is opened or read, never what a behaviour test
asserts about behaviour. Paths are under `D/view/` unless named.

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
  `lifecycle-kind` asserts (`:538`, `:568`) and the `lifecycle-branch`
  assert (`:547`) go. **Assertion removal:** the kind is the tile's
  icon, which no test reads (appearance), and the branch is in the Git
  line, which `confirm_tests` reads.
- **E-4** (Task 2) `lifecycle::outcome_tests::stop_at_a_safe_point_names_completed_steps`
  (`lifecycle/outcome_tests.rs:224`): the `lifecycle-branch` assert goes
  (**assertion removal**, as E-3).
- **E-5** (Task 2) `lifecycle::outcome_tests::the_operation_entry_reopens_the_dialog`
  (`:484-485`): the title reads "Stopped at a safe point".
- **E-6** (Task 3) `lifecycle::confirm_tests::closure_shows_closed_by_from_the_preview`
  (`lifecycle/confirm_tests.rs:292-293`) and
  `::existing_closure_and_already_contained_read_already` (`:437`): the
  note reads "Closed by {name} · today".
- **E-7** (Task 1) `conflict::entry_tests::resolve_conflict_opens_the_childs_sheet`
  (`conflict/entry_tests.rs:406-407`): `RESOLVE_CONFLICT` becomes
  `REVIEW_CHANGES` (Part C item 1). The button's id stays.
- **E-8** (Task 1) the activity copy tables, whose literal "Review
  changes" becomes the constant: `model::activity::tests::share_copy_table`
  (`D/model/activity_tests/tables.rs:113-114`),
  `::each_source_makes_its_rows` (`states_pending.rs:174`, `:197`),
  `::logs::attention::attention_counts_each_thing_once`
  (`attention_states.rs:194`).
- **E-9** (Task 1) `conflict::tests::save_is_disabled_until_every_part_is_chosen`
  (`conflict_tests.rs:328-329`), `::choices_wait_while_a_save_is_out`
  (`:669-675`), `conflict::stale_tests::a_stale_observation_refreshes_the_comparison`
  (`conflict/stale_tests.rs:81-82`): `KEEPING_*` becomes `CHOSEN`, and
  each asserts which choice was made through `ResolveSheet::choice_of`
  (R-20).
- **E-10** (Task 6) `conflict::tests::save_is_disabled_until_every_part_is_chosen`
  (`conflict_tests.rs:300`): `["title"]` becomes `["Title"]`.
- **E-11** (Tasks 1 and 5) `keyboard_tests/conflict.rs`'s walk:
  `choose_by_keyboard` (`:75-83`) asserts `CHOSEN` and the choice through
  `choice_of` (Task 1); `in_dialog` becomes `in_sheet` (R-19) at every
  call in `resolve_a_local_conflict_by_keyboard`,
  `resolve_a_shared_conflict_by_keyboard` and
  `sheet::{check_again_by_keyboard_keeps_focus_in_the_sheet,
  edit_the_result_by_keyboard, ctrl_enter_with_a_part_unchosen_keeps_focus_in_the_editor}`
  (Task 5).
- **E-12** (Task 6) `keyboard_tests::conflict::sheet::edit_the_result_by_keyboard`
  (`:134-135`): Tab from the Result stop reaches `conflict-cancel`, then
  `conflict-save` (decision 4).
- **E-13** (Task 1) `desktop::assets::tests::every_item_view_icon_loads`
  (`D/assets_tests.rs:8`): `ICONS` gains `circle-x` and `git-merge` (58 to
  60).
- **E-14** (Task 3) the `lifecycle-ending` waits and reads need no edit
  (R-17). Listed so a reviewer does not look for them: the sweep found
  about 25.
- **E-15** (Task 7) the privacy tests' `SURFACES`
  (`lifecycle/privacy_tests.rs:112-113`) gain `conflict-header` and
  `conflict-footer` (Resolve) and `lifecycle-title` (the lifecycle's
  title slot), and the a16 tests' `holds_none` lists
  (`lifecycle/privacy_tests.rs:513-521`,
  `lifecycle/privacy_promote_tests.rs:196-202`,
  `lifecycle/privacy_remote_tests.rs:281-287` and `:185`) name them, so
  the moved header and footer stay swept (P10, U-8).

Sites the sweep checked that need no edit: the `lifecycle-cancel` tab in
`keyboard_tests::lifecycle::Walk::open_the_dialog` (`:170`, order-free);
`STARTED` (no test reads it); the step-note count (confirm only);
`MADE_READY_LEAD` (compared as a constant);
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
2. **`05`, as built** (the approval commit): under "Close ticket dialog"
   and "Resolve conflict sheet", what this Cycle built differently: the
   step order and Share main (D5 R-8), no line counts, no ready list
   before the close, the stage words as progress, the message as the
   cause, the Expires line, the dismiss labels, the backdrop rule;
   no names or times, no base, Keep theirs, the Result editor and its
   note (D5 R-27, R-34), the shared outside actions, the scrim.
3. **`08`** (the approval commit): the Sheets row and the mapping table's
   `draft_comparison.rs` row note that Resolve conflict is a sheet the
   workspace owns (`SheetOwner::Resolve`).

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
- **R-6:** the footer rule is mechanical (dismiss, others, primaries) —
  today's order already reflects D5's choices; only the dismiss and the
  primaries move — an order the owner would arrange by hand.
- **R-7:** Resolve's scrim does not close it — closing loses the choices,
  and today's dialog ignores its backdrop — none.
- **R-8:** the footer wrapper carries `RESOLVE_SHEET_CONTEXT` and
  `SaveResolution` — Ctrl+Enter saves from the buttons today — none.
- **R-9:** every way in goes through `show_resolve` — the owner must be
  set as the sheet opens, and a view inside a Workspace update cannot
  reach the Workspace again — a route event and a refactor of `Entry`.
- **R-10:** Ctrl+, and `show_settings` refuse while the owner is
  `Resolve` — today the dialog stops both; Settings would close the sheet
  and lose the choices — none.
- **R-11:** the "something behind took focus" check covers `Resolve` —
  R-60 — none.
- **R-12:** the Kit's sheet body scrolls, not the view — one scroll, and
  the header and footer stay put as `05` draws them — none.
- **R-13:** the gutter stays inside the line's text — in mono it already
  reads as a column, and the tests read lines with it — a column of its
  own later.
- **R-14:** Open folder and Check again stay one row — both act on the
  whole set; per card they would repeat — differs from `05`'s sketch,
  which is "not designed visually".
- **R-15:** the Git line is the footer's first element, filling the left
  — `Footer`'s hint is a fixed string — none.
- **R-16:** shared constants live in `D/model/activity/copy.rs` — the
  model already holds `REVIEW_CHANGES` there (Cycle 03b R-41) — none.
  `D/model/review.rs`'s `KEEPING_MINE` ("Keeping mine") and `CHOOSE_ONE`
  belong to the item's own review of a remote change, not this sheet,
  and stay.
- **R-17:** `lifecycle-ending` stays as the outcome box's first line —
  removing it would touch about 25 tests and four helpers that wait on
  it, and drop the privacy sweeps' read of the ending inside the dialog;
  the header says the stage in other words — the stage is said twice.
- **R-18:** an automatic opener that waits for no dialog also waits
  while the owner is `Resolve` — today the Resolve dialog held it back;
  over the sheet it would cover the choices — a Republish… shown later
  than it could be.
- **R-19:** the conflict keyboard tests hold focus "in the sheet" by a
  walk helper `in_sheet` that asserts focus is inside the sheet's
  elements and the Content region holds it, as Settings' tests do (fact
  32) — `in_dialog` cannot hold for a sheet — none.
- **R-20:** a test tells Keep mine from Keep theirs by the sheet's model
  (`ResolveSheet::choice_of`, test-only), since both states read
  "Chosen" — the state word no longer says which — none.

## Known Limits

- K-1 No line counts per file (decision 1, fact 25).
- K-2 No ready list before the close (decision 2, fact 26).
- K-3 Progress is the stage's words, not counts (fact 27).
- K-4 A failure's cause is the library's message, which names no key
  (fact 28).
- K-5 "today" is assumed, not read (R-3).
- K-6 No author names or times; "Keep theirs" and "Theirs" (fact 30).
- K-7 The Git line names ours and theirs, no base (fact 30).
- K-8 The selected choice is the Kit's `selected` style, not `05`'s teal
  fill.
- K-9 The files heading keeps its count in parentheses.
- K-10 A submitted request shown as a message only takes the Ask title
  (A1).
- K-11 Resolve's scrim does nothing (R-7).
- K-12 Outside cards share one row of actions (R-14).
- K-13 Facts 25, 27, 28 and 30 go to the follow-up library ticket
  (decision 1).
- K-14 The body is one part named "Description", not `05`'s regions, so
  its card has no kind label (B5).

## Unverified

- **U-1:** Cycle 05a's names after it merges: `SheetOwner::AddRepository`,
  the widened `settings_open`, `sheet_footer` and `Tone::Accent` in use,
  `wrapped_kit`, and where C5a moved `keep_focus_in_settings`. Task 0.
- **U-2:** the Result editor's Escape (`RESOLVE_SHEET_EDITOR`) still wins
  over the Kit sheet's Escape. `keyboard_tests::conflict::sheet::edit_the_result_by_keyboard`
  proves it once the sheet moves (Task 5).
- **U-3:** the Kit sheet's footer slot takes an element with its own key
  context and action (R-8). Task 5.
- **U-4:** whether a test can click the dialog's overlay (P3). Task 2.
- **U-5:** the Kit `Spinner` in a test window (it animates). Task 2.
- **U-6:** the named edits, from the test sweep at `ac50ba1a`. Task 0
  re-checks them against 05a's merged code.
- **U-7:** the classification of fact 31's sites (R-18). Task 0.
- **U-8:** that a privacy sweep files an element under its own id when
  that id is in `SURFACES` (E-15). Task 7.

## Alternatives Rejected

- A whole-file Result preview (`05`): it would replace D5 R-27's editor,
  a behaviour change.
- Per-card outside actions (R-14).
- Keeping Resolve conflict a dialog (decision 3).
- Line counts, authors and the base in this Cycle (decision 1).
- A provisional ready list at confirm (decision 2; D5 decision 6).

## Risks

| Risk | Mitigation |
| --- | --- |
| The sheet slot's interplay (Settings, Add repository, details, dialogs over it) | B1–B3 follow C4 and C5a; P5–P7 and H1–H10 are written first (plan Task 4's design checkpoint) |
| A D5 behaviour test edited into meaning less | Named edits change copy, ids, order or the open helper only; the reviewer of each task checks each edit |
| 05a's Tasks 4–10 change what this design cites | Task 0 re-maps (U-1); execution waits for 05a's merge |

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning research, 2026-11-15 | The lifecycle dialog, Resolve conflict and the library's data, read at `ac50ba1a` | Facts | Baseline Facts 1–30 |
| Owner, 2026-11-15 | Missing data; ready list; sheet or dialog; button order; D5 follow-ups; the outline | Owner decisions | Decisions 1–6 |
