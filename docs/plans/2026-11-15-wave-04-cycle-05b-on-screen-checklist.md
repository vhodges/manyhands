# Wave 04 Cycle 05b: On-Screen Checklist

Ticket `01M7D7JK5MSY6J471JEKG3CZXR`, Close Ticket And Resolve Conflict.
This is acceptance row A10: the surfaces match frames 2c, 2d, 2e and 2g as
amended, in the dark theme. Tests deliberately do not check appearance
(layout, widths, colours, icons, wrapping, the step-row drawer, the footer
order as drawn). You do. Tick each box as you see it.

Frames (open beside the app): `docs/design/screenshots/2c-close-ticket-confirm.png`
(2c, the Confirm stage), `2d-close-ticket-running.png` (2d),
`2e-close-ticket-failed.png` (2e) and `2g-resolve-conflict-sheet.png` (2g).

Source: `2026-11-15-wave-04-cycle-05b-close-and-resolve-design.md` (the
"design"). Section letters (A1 to A7, B4 to B6) and Known Limits (K-n) are
its. Strings in quotes are the design's words. Where a line says "Not
derived from the design: ask the implementer", the design does not say.

## Before you start

- Run: `devenv shell -- cargo run --locked --features desktop --bin manyhands`
  (needs an active desktop display; Wayland is the Linux smoke test).
- Use the dark theme.
- Per `AGENTS.md`, run the app on real data only from up-to-date main; this
  branch's build migrates the registry one way. Use a throwaway repository.
- **For a close (2c, 2d, 2e):** you need a ticket that can be closed from the
  item header's lifecycle action (Close ticket…), or Approve and merge… on a
  document. The design does not state what repository state to prepare. Not
  derived from the design: ask the implementer (what makes the confirm show
  files, a remote and a Git line, and how to reach Failed, Waiting and
  Stopped).
- **For a conflict (2g):** you need an item with a conflict, so that Review
  changes… is offered. The design does not state how to create one. Not
  derived from the design: ask the implementer. It names these ways in (B2):
  the item banner's Review changes…; an Activity row, a pending row or a
  Needs attention entry, Review changes…; the lifecycle dialog's Repair
  locally… or Review changes….
- Optional: Windows or macOS, for K-18 only.

Tick when ready: [ ] app is running, dark theme, throwaway data.

## Frame 2c: Close ticket, confirm

Shows the dialog in its Ask stage, phase Confirm (A1, A4): header with a tile,
a title, the subtitle; the lead; the bordered numbered steps; the files that
will be merged; the code note; the Git line; the Expires line; the footer.

Reach it: open a closable ticket and choose Close ticket…. For a document,
choose Approve and merge… and check the "Approve and merge {short}?" title
instead.

- [ ] Dialog is 580 px wide (the Cycle's table: width 580, unchanged); a header
      tile sits left of the title.
- [ ] Title reads "Close {short}?" with the ticket's short id (document:
      "Approve and merge {short}?"). Tile is `Archive` (document: `GitMerge`),
      accent colour.
- [ ] The subtitle reads "{short} · {title} · {repository}"; no separate kind
      icon and no branch line in the header (A2).
- [ ] Steps are a bordered, numbered list, number in a muted circle. A "Mark
      as closed" step note reads "Closed by {name} · today" (or "Closed by
      {name}" when already marked closed).
- [ ] The Git line shows the branch; the muted Expires line follows it.
- [ ] Footer, left to right: hint "Enter to confirm · Esc to cancel", then
      Cancel, then the bold command ("Close ticket"). The command is last.
- [ ] No line counts per file and no "Becomes ready to start" at confirm (K-1,
      K-2, below).
- [ ] The file rows show an icon, a path and a tag; not drawn from the design:
      `+n −m` counts (do not expect them). Anything else about the file list:
      see the design frame 2c.

## Frame 2d: Close ticket, running

Shows the Running stage (A5): the lead above the steps, step marks, the Git
line, and a footer with only Stop.

Reach it: press the command on confirm and watch while the close runs.

- [ ] Title reads "Closing…" (document: "Merging…").
- [ ] The lead above the steps reads "You can keep working. This continues in
      the background and shows up in Activity."
- [ ] Marks: a green check for done steps, a teal spinner for the running one,
      a muted number for steps not started.
- [ ] The running step carries a note in the stage's words (K-3), not counts.
- [ ] The Git line follows the steps.
- [ ] Footer: hint "Esc hides this window", then Stop at a safe point. There
      is no Close or Cancel button.
- [ ] Press Stop at a safe point: "Stopping…" takes its place; Tab still stays
      in the dialog and Escape still hides it.
- [ ] Press Escape, and in a second run click the backdrop: the dialog hides,
      the run goes on, Activity keeps it, and focus returns to where you
      started.

## Frame 2e: Close ticket, ended

Shows the ended stages (A6, A7): step rows, the outcome box, the made-ready
box, the Git line, and the footer in the dismiss, non-primary, primary order.
Stages: Done "Ticket closed" (document: "Merged into {primary}"), Failed
"Couldn't finish closing" (document: "Couldn't finish merging"), Stopped
"Stopped at a safe point", Waiting "Still running…".

Reach it: let a close finish for Done. Failed, Stopped and Waiting: Not
derived from the design: ask the implementer. (Stopped follows Stop at a safe
point on 2d; Waiting follows an answer of "Still running…".)

- [ ] Done: title "Ticket closed", `CircleCheck` tile in green.
- [ ] Done: the outcome box (1 px border and faint tint in the tone, radius
      6) has a green `CircleCheck` and the ending word "Done" in semibold.
- [ ] Done: a teal made-ready box whose lead reads "Now ready to start", with
      the Git line below.
- [ ] Done: footer is Open ready tickets (when offered), Republish… (when
      offered), then **Done**. Done is last.
- [ ] Failed: title "Couldn't finish closing", `CircleX` tile in red; the
      Failed step's mark is a red `CircleX` and its title is red, with the
      message in red in place of its note; the outcome box is red.
- [ ] Failed: footer starts with "Not now", then the fixes (Repair locally…,
      Review changes…, and the like), and ends with the primary (Republish…,
      Retry, Prepare again or Unlock…). The primary is last.
- [ ] Stopped: title "Stopped at a safe point"; amber box reading "Stopped at
      a safe point". Waiting: title "Still running…"; amber box; footer
      starts with Close.
- [ ] Wrapping: in a state with many buttons (the blocked prepare is the
      case, five buttons), the footer wraps to a second line, buttons still in
      order and none cut off. The design draws one line; this is the
      implementer's O-7 ruling (execution ledger), not the design. Is a
      two-line footer acceptable? [ ] yes [ ] no, make the dialog wider or
      the labels shorter.
- [ ] Blocked, Held, Saving, Preparing and any run reopened without its
      preview do not name the branch anywhere, since the branch lives in the
      Git line (A2, K-15). Accept? [ ] yes [ ] no.

## Frame 2g: Resolve conflict, sheet

Shows Resolve conflict as a right-side sheet (B1 to B6), up to 920 px wide
(narrower in a small window, the window width less 48 px), over a scrim, with
a header, a scrolling body of cards, and a pinned footer.

Reach it: open a conflicted item and choose Review changes… (or one of the
other ways in above).

- [ ] The sheet opens on the right; the header and footer stay put while the
      body scrolls.
- [ ] Header title reads "Choose what to keep · {title}", the description
      beneath it, and the Kit's close button is present.
- [ ] Below the header: the meta line with "{n} of {m} chosen" (amber, then
      green when all are chosen), then the note.
- [ ] Each card's header row reads: the part's name in words (such as "Title",
      "Status", "Closed by"; the body part is "Description"), its kind
      ("Ticket detail", "Document detail" or "Comment detail"; the body has no
      kind label, K-14), its state ("Choose one" amber; "Chosen" or "Edited"
      green), then Keep mine, Keep theirs (and Keep both for the body) at the
      right.
- [ ] Beneath the header row, three equal mono columns: Before ("last shared
      copy", muted), Yours, Theirs, with line gutters inside the text.
- [ ] The chosen button shows the selected style and a check icon before its
      label (K-8).
- [ ] Footer, left to right: the Git line (fills the left, cut short when
      long), Cancel, then **Save resolution**, enabled only when every part is
      chosen. Save is last. Tab reaches Cancel, then Save resolution.
- [ ] Clicking the scrim does nothing (K-11). Escape, the header's close
      button and Cancel close the sheet.
- [ ] An outside set (if you can reach one): one card per path with its
      reason; Open folder and Check again are one shared row below (K-12).

## Known Limits

Tick one box per line. "Accept" means you accept it as shown.

- K-1 No line counts per file. [ ] I accept this as shown [ ] I want this changed
- K-2 No ready list before the close. [ ] accept [ ] change
- K-3 Progress is the stage's words, not counts. [ ] accept [ ] change
- K-4 A failure's cause is the library's message, which names no key. [ ] accept [ ] change
- K-5 "today" is assumed, not read. [ ] accept [ ] change
- K-6 No author names or times; "Keep theirs" and "Theirs". [ ] accept [ ] change
- K-7 The Git line names ours and theirs, no base. [ ] accept [ ] change
- K-8 The chosen choice is the Kit's `selected` style with a check, not
  `05`'s teal fill. [ ] accept [ ] change
- K-9 The files heading keeps its count in parentheses. [ ] accept [ ] change
- K-10 A submitted request shown as a message only takes the Ask title. [ ] accept [ ] change
- K-11 Resolve's scrim does nothing. [ ] accept [ ] change
- K-12 Outside cards share one row of actions. [ ] accept [ ] change
- K-13 Facts 25 and 30 (counts; names and times) go to the follow-up library
  ticket `01M7FBRBN04QACSCZV6KX0WMR3`. [ ] accept [ ] change
- K-14 The body is one part named "Description", not `05`'s regions, so its
  card has no kind label. [ ] accept [ ] change
- K-15 A run shown without its preview (reopened from Activity, or after a
  restart) draws no Git line and no step notes but the running step's.
  [ ] accept [ ] change
- K-16 Ctrl+Enter does not save while focus is on the Kit's close button,
  which sits outside the sheet's key context. [ ] accept [ ] change
- K-17 The Git line is cut short with no tooltip, as today. [ ] accept [ ] change
- K-18 The backdrop flag is read when the dialog is drawn: a click in the
  frame after a run ends hides the ended dialog. On Windows and macOS the Kit
  marks the backdrop as a window-drag area, which may take the click. This
  checklist checks it there: [ ] checked on Windows/macOS, backdrop hides a
  running dialog [ ] could not check [ ] accept [ ] change
- K-19 Tab to a part below the fold does not scroll it into view, as today.
  [ ] accept [ ] change
- K-20 Root's focus trap tries 100 stops: with more Tab stops than that
  behind the sheet, a wrap from the last control can land behind it, and
  focus returns to the sheet's handle, not the close button. [ ] accept [ ] change
- K-21 Quitting closes the sheet in the window that quits; another window's
  sheet stays until that window quits. [ ] accept [ ] change

## Verdict

Frames 2c, 2d, 2e and 2g, in the dark theme, against the design as amended.

- [ ] **Accept.** The surfaces and every Known Limit are accepted as shown.
- [ ] **Changes wanted.** The ticks above marked "change" or "no", and the
      notes below, are the changes.

Date and platform seen on: ______________________

Notes:

&nbsp;

&nbsp;

&nbsp;
