# Wave 04 Cycle 03b On-Screen Checklist

This list is for checking the Cycle 03b Activity panel (one timed log, its
rows' copy, the filter and ⋯, the row menu, the keyboard, one-line rows at
any width) and the Needs attention footer item and popover by hand on Linux
Wayland, in the dark theme, before the Cycle is called review-ready. It is
the native evidence the design asks for (design, "Testing"; Cycle document,
A14, "On screen"; implementation plan, Task 13 Step 1), and it carries what
no test can see: the panel and the popover against frames 1a and 2f, a real
right click and a real menu key, where the popover sits above the footer,
and a focus move made while the footer draws. Work through sections S-1 to
S-10 in order. Write each step's result in its row's last column, and a
one-line summary per section in "Results" at the end. The owner marks each
row: Claude records only what you report and never assumes a result.

Quoted text is the app's own wording, from the design's Part B tables.
`{repo}` stands for a repository's display name (its directory's name),
`{primary}` for its main branch, `{title}` for an item's title, `{n}` for a
count. "Ctrl" is the `secondary` key of the keymap on Linux. A key written
`Shift F10` is held together; "the menu key" is the context-menu key
(`menu`). "The design" is
`docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md`;
"design B4" is its Part B, section B4, and "design S4" its copy table row
S4 (not this list's S-4 section). Where a row says "see design", the design
gives the wording and this list does not repeat it. "The frames" are
`docs/design/screenshots/1a-layout-classic.png` (Activity in the bottom
dock, with the filter and ⋯ at the right of its header, one line per row:
time, icon, message, tag, button) and `2f-needs-attention-popover.png` (the
popover above the footer's "3 need attention", 440 px wide, three rows and
the link "Show everything in Activity"), read with `docs/design/03-panels.md`
(Activity), `05-sheets-and-dialogs.md` (Needs attention popover) and
`06-language-and-states.md`.

The **Supports** column names the Acceptance row (A1 to A16, Cycle document,
"Acceptance And Exit Evidence") and the design's Required Outcome (P1 to
P32, design, "Required Outcomes") the step is evidence for. A step with no
automated twin says so. Steps marked **(rare)** need a state that is hard
to make by hand: try each one, and if you cannot reach it write "skipped".
Each is also covered by an automated test. Steps marked **Watch** name
something that has never been seen on a real display: write what you saw,
either way. Steps marked **Owner decides** are not pass or fail: look and
say what you want.

## Minimum pass

If you cannot do every row, these cover each Acceptance row that has
on-screen evidence at least once. Nothing below is removed from the list;
the other rows add depth.

- **A14, Activity, the popover and the footer item against frames 1a and 2f
  in the dark theme (the owner's eye, no automated twin):** S-1.2 to S-1.4,
  S-6.1 to S-6.3, S-6.6 to S-6.9.
- **A5, rows of time, icon, message, tag, button; the copy:** S-2.1 to
  S-2.8, S-2.15.
- **A8 and the K-19 rows (decision 22):** S-8.1 to S-8.6.
- **A10, Activity and its row menu by keyboard:** S-4.1 to S-4.4, S-5.1 to
  S-5.6.
- **A2, every popover action by keyboard:** S-6.11 to S-6.14.
- **A3, Activity hidden and the popover still works:** S-7.1 to S-7.4.
- **A12, nothing acts behind a dialog or sheet:** S-9.1, S-9.2.
- **A1 and A7, one count, each thing once:** S-3.2, S-6.8.
- **A15, "N to share" agrees with the reminder rows:** S-10.1, S-10.2.
- **A4, no secret on screen:** S-2.14, S-2.16.

Mostly covered by automated tests, or rare, so **optional** if time is
short: S-1.5, S-2.9 to S-2.12, S-4.7, S-5.8, S-6.17, S-6.19, S-8.8 and
S-9.4.

## Before you start

- **Use a scratch data directory for everything.** This checklist makes
  repositories, stalls a remote, locks a key and makes a draft unsaveable.
  Run all of it against a throwaway data directory so your real one is
  never touched. Make it, check it took, and launch with it exactly as
  Cycle 03a's checklist does
  (`docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md`,
  "Before you start": the desktop reads `XDG_DATA_HOME`, so the scratch
  data directory is `$SCRATCH/manyhands`, and the CLI reads
  `MANYHANDS_DATA_DIR`). Your real data stays untouched, except Shared SSH
  keys in `~/.ssh/manyhands`.
  - **Smoke test (S-1.1).** From the ticket worktree
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y94Q3AXT6XJX38F5VK1V`,
    once, on Wayland:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --locked --features desktop --bin manyhands`
    Quit it (Ctrl Q) when the window is up.
  - **Launch with it** for every step after S-1.1: the release build,
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`
    It needs an active desktop display. Run it while no long test suite or
    build is running on the machine.
- **Scratch repositories.** Make them as Cycle 03a's "Before you start"
  does, with Settings (Ctrl ,) › Repositories › **Create repository** and
  its table of names; this list needs only:
  - `shared` and `second`, SSH shared copies with a passphrase-protected
    key (several tickets in `shared`);
  - `local`, no remote;
  - `away`, no remote, whose folder is renamed (Can't open);
  - `a-repository-with-a-very-long-name-for-a-narrow-activity`, no remote
    (S-8.2).
- **How the app produces a state's row.** Where Cycle 03a's checklist
  already says how, follow it: a stalled remote for Can't reach and a
  running check (its "Stalling and stopping the remote", S-2.6, S-2.7); a
  locked key by quitting and launching again (its S-2.14); Can't open by
  renaming the folder (its S-2.9); Needs fixing by breaking
  `.manyhands/config.toml` (its S-2.11); work not shared by saving without
  sharing (its S-2.3, S-2.4); Paused in Settings › Checking for updates
  (its S-2.13). For anything else (a failed share, a stopped share, a
  pending Resume, a pending list that fails, a closed ticket whose copy
  remains, a problem item from a check) this list says what state the row
  needs and leaves the method to you; if you cannot reach it write
  "skipped".
- **Activity at the start.** The app opens at 1280 x 840 in the default
  layout (Cycle 01, "The default layout"): Activity in the bottom dock,
  Tickets in the right dock. Several steps move it. Activity is a session
  log: it starts with whatever the launch itself did (launch refreshes are
  rows, design decision 13), so it may not be empty.
- **The dark theme.** Run in it, and write down in "Results" which theme
  and which scale factor you ran.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure.
The first group is this Cycle's Known Limits (design K-1 to K-13); the
second is what the frames draw and this Cycle does not build.

- **K-1.** Rows and their times last for the session. After a relaunch,
  pending operations, stored check outcomes, recovered drafts and the
  unshared list come back from their own stores, timed by the new session.
- **K-2.** An item is named by its short code when no title is known ("the
  ticket {short}", "the document {short}"). A new item reads so until a
  list read gives its title.
- **K-3.** A failure's reason is the library's fixed sentence for its code,
  and some name CLI commands.
- **K-4.** Nothing is capped or pruned within a session; "Clear finished"
  is the relief.
- **K-5.** Where the local offset is unknown (non-Unix), times are UTC and
  the tooltip's time says " UTC".
- **K-6.** No names: "Brought in N changes" names no one, and a conflict is
  "you and someone else".
- **K-7.** "Tries again" gives no minutes.
- **K-8.** Tooltips show on hover only. The full message and the detail are
  also each row's accessible description and its menu's first lines.
- **K-9.** A pending conflict with no pending row loaded has no Activity
  row.
- **K-10.** A pending row with no operation ID can change its time.
- **K-11.** A press outside the popover also acts on what was pressed.
- **K-12.** A busy share's "May be waiting for …" can change while shown.
- **K-13.** An unreachable repository with problems from an earlier check
  shows the problems, not Can't reach.
- **Frames, not failures.** The frames draw things this Cycle does not
  build or words it words otherwise: names ("Jo Smith") where the app says
  "someone else"; "Last tried 7 minutes ago" and "tries again in 5" where
  the app gives no minutes; the straight quotes of the mockup where the app
  uses “ ”; `MH-VH-…` upper-case short codes; the Repos tab strip and the
  Tickets footnote (Cycle 03a's list).

## S-1 Launch and the first look

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-1.1 | Run the smoke test above. | A window opens with no error in the terminal; the title is "Manyhands"; Activity is in the bottom dock. Ctrl Q closes it. | A14 (no automated twin) | |
| S-1.2 | Look at Activity's header against frame 1a. | "Activity" at the left and, at the right, two buttons: the filter (`list-filter`) and ⋯ (`ellipsis`). None of today's old headings in the body: no "This session", no "No requests yet", no "Pending in repositories", no "No pending operations". | A14, A6 (appearance: no automated twin) | |
| S-1.3 | Look at the rows against frame 1a. Make a few: save a ticket (Ctrl S), press Check now on `shared`. | Newest first. One line per row, 28 px high, in this order: the time ("HH:MM", monospace, muted), the kind's icon, the message, the repository tag, at most one button. | A14, A5, P14 (appearance: no automated twin) | |
| S-1.4 | Look at the colours and then at each row with the colour taken away in your head. | Dark theme: chrome (header) darker than the body, thin borders, nothing unreadable. **Every row has an icon and words**, so none depends on its colour. | A14, A16 (no automated twin for the look) | |
| S-1.5 | **(rare)** Make a layout notice show (Cycle 01 checklist, W-8.10) and, with a job running, press Ctrl Q to show the quit row (its W-9.1). | The notices and the quit row sit **above** the list, as today, with their words unchanged, and do not scroll away. | A6 (design B0 and C1; no automated twin for the look) | |
| S-1.6 | Hover the filter and ⋯. | The filter: "Show only things that need attention". ⋯: write its tooltip (the design gives it no words; see design C1). | A14, P14 (no automated twin for the tooltip) | |

## S-2 What each row says

Put `shared` and `second` on Every 1 minute where a step needs a check soon.
For each row read three things: the **line** (time, message, tag, button),
the **tooltip** (hover about a second; K-8) and the **menu's first lines**
(S-4 opens the menu). The line's message is the title, then ". " and the
description when there is one. The tag is `{repo}`. The tooltip is the full
message, the time in full ("{YYYY-MM-DD HH:MM}"), then the detail. Write
down what you read where a row says so.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-2.1 | **A save.** Edit a ticket in `shared` and Save (Ctrl S). | “Saved “{title}” on this computer”, no button, tagged `shared`. The title is the item's, never the draft's own; a new item reads "the ticket {short}" until a list read gives its title (K-2). | A5, P4, P1 (twin: `request_copy_table`; the look has none) | |
| S-2.2 | **A check that brought something in.** In a second clone push a ticket change to `shared`'s shared copy; wait for the automatic check, then do it again and press Check now. | Automatic: "Checked {repo} for updates. Brought in 1 change." or "… Brought in {n} changes.". Check now: its own row "Checked {repo} for updates" and a row "Brought in 1 change to {repo}" or "Brought in {n} changes to {repo}". Neither names a person (K-6). A quiet automatic check that brings nothing in adds **no** row. | A5, P8 (twin: `brought_in_is_kept_for_the_session`) | |
| S-2.3 | **A running request.** Stall the remote and press Check now in `shared` (Cycle 03a's checklist). | At once: "Checking {repo} for updates…" and, when a stage is known, " · {stage words}". A button: Cancel, or "Stop at a safe point" when offered, or the disabled "Stopping…" or "Finishing". | A5, P4, P10 (twin: `request_copy_table`, `stages_are_shown`) | |
| S-2.4 | **A failed request.** Let S-2.3's check fail (about 30 s). | The row reads "Couldn't check {repo} for updates" and a description ending "Your work is safe on this computer." (write the whole line). It stays until fixed or dismissed. | A1, A5, P4 (twin: `request_copy_table`) | |
| S-2.5 | **A share that did not finish.** Make a share fail or stop, for example a locked key with work saved, or Share with the remote stalled (method yours). | The title and description are design B3's row for the outcome, for example "Couldn't share “{title}”: your key is locked" with "Unlock it, then share again." and Unlock…. The button is the first non-risky offer; "Stop trying to share" is never the button. Write the row you got. | A1, A5, P5 (twin: `share_copy_table`) | |
| S-2.6 | **A locked key.** Quit, launch again so the key is locked, set `shared` to Every 1 minute and wait. | One row, however many repositories it holds: "Your key is locked", description "Checking {repos} is waiting for the passphrase.", button Unlock… (it opens the "Unlock key" dialog; **Not now** leaves it). Tag `{repo} +{n}` when it holds several. **No passphrase dialog opens by itself.** | A1, A5, P6 (twin: `state_copy_table`) | |
| S-2.7 | **A repository's state.** With the remote stalled and `shared` failing, then `away` renamed and Ctrl R, then `fixing` or `local` as Cycle 03a's checklist makes them. | Can't reach: "Couldn't reach {repo}'s shared copy", "Your work is safe, and Manyhands tries again.", Retry now. Can't open: "Can't open {repo}" with Retry. Needs fixing: "{repo} needs fixing" with Settings…. A repository with no shared copy: "{repo} has no shared copy, so changes stay on this computer" with Set up sharing…. The descriptions of Can't open, Needs fixing and the non-SSH case are the state's own sentence (see design B4); write each. | A1, A5, P6 (twin: `state_copy_table`) | |
| S-2.8 | **Work not shared yet.** In `shared`, Save a ticket without sharing, and add a commit to `main` with plain `git`; press Check now. | A row “{title}” isn't shared yet with Share…, and a row "{primary} in {repo} isn't shared yet" with "Share main". Never counted in Needs attention. | A5, A15, P6 (twin: `state_copy_table`, `reminders_match_to_share`) | |
| S-2.9 | **(rare) A closed ticket whose copy stays.** Close a ticket so its copy remains on this computer. | “{title}” is closed, but its copy is still on this computer, with Retire…; a promoted document reads "is approved, but …". | P6 (twin: `state_copy_table`) | |
| S-2.10 | **(rare) A pending operation.** Leave an operation unfinished (a stranded share). | "{action words} didn't finish in {repo}", or "{action words} “{title}” didn't finish" when the item is known; the description is the next action's sentence (see design B4); the button is Resume or Review changes. Write the row. | A6, P6 (twin: `state_copy_table`) | |
| S-2.11 | **(rare) A problem from a check.** Make a check list a problem item (Cycle 03a's S-2.12). | One row per item: “{title}” needs a look, with the disposition's sentence as its description (see design B4, "S5 dispositions") and Open. | A1, P6 (twin: `state_copy_table`) | |
| S-2.12 | **(rare) Recovered drafts.** Launch again after a crash with a draft left over (Cycle 01 checklist, W-2.10). | "1 draft recovered from an earlier session" or "{n} drafts recovered from an earlier session", with Review. | P6 (twin: `state_copy_table`) | |
| S-2.13 | Look at the times. | "HH:MM" in local time; the tooltip's time line is "{YYYY-MM-DD HH:MM}" (K-5: " UTC" only where the offset is unknown). Around midnight nothing reads out of order. | P7 (twin: `activity_time_formats`) | |
| S-2.14 | Hover a repository's row (S-2.7) and read its tooltip and menu. | The detail is "Main branch: {primary}" and "Shared copy: {remote}", the remote's **name only**. No "Folder:" line, no path, no address (unlike Cycle 03a's panel tooltip, which is unchanged). | A4, P28 (twin: `polling_privacy`; the reading has none) | |
| S-2.15 | Hover a request's row (S-2.1, S-2.4). | Line 1 is the full message; then the time in full; then "{intent label} · {scope}", the outcome ("Done", "Failed"), "{code}: {message}" when it did not succeed, "Operation {id}" when known. | A5, P14 (twin: `rows_draw_from_the_log_and_filter`) | |
| S-2.16 | Read every row, tooltip and menu you made for a body, a comment, a remote URL, a folder path, a key's label or path, a passphrase. | None appears. A title may (decisions 12 and 21). | A4, P28 (twin: `polling_privacy`) | |

## S-3 The filter and ⋯

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-3.1 | Look at the filter at launch, then press it, then press it again. | Off at launch. On: the button shows it is on (write how) and the list shows only attention rows. Off again: all rows. | A5, P14 (twin: `rows_draw_from_the_log_and_filter`; the look has none) | |
| S-3.2 | With the filter on and a failed request, a locked key and a Can't reach row (S-2), count the rows and read the footer's count. | The same number: the footer's count is the attention rows, each thing once. Successes, brought-in rows, reminders, Retire… offers and information rows are hidden by the filter. | A1, A7, P2, P13 (twin: `attention_counts_each_thing_once`, `count_and_popover_agree`) | |
| S-3.3 | Fix everything that needs attention (unlock the key, release the remote, dismiss the failure), with the filter on. | "Nothing needs attention", in place of the list. With the filter off, rows (successes, history) still show. | P14 (twin: `rows_draw_from_the_log_and_filter`) | |
| S-3.4 | Hide Activity (footer toggle) and show it again with the filter on. | The filter is still on: it lasts for the session. | P14 | |
| S-3.5 | **(rare)** A session with no rows at all and the filter off. | "Nothing yet this session". If launch refreshes always leave a row write "skipped". | P14 | |
| S-3.6 | Open ⋯ by pointer, then by keyboard (Tab to it, Enter). Press Esc. | A menu with one item, "Clear finished". Esc closes it and focus is on the ⋯ button. | A10, P19 (no automated twin for the real menu) | |
| S-3.7 | With nothing finished to clear (every row a failure or a state), open ⋯. | "Clear finished" is disabled. | P19 (twin: `clear_finished_keeps_failures_and_states`) | |
| S-3.8 | With successes, a failure, a locked-key row, a Can't reach row and a share you stopped (it offers Give up) showing, choose **Clear finished**. | Successes and information events go. The failure, the states, the attention rows and the stopped share **stay**. The footer's count does not change. | A1, P19 (twin: `clear_finished_keeps_failures_and_states`) | |

## S-4 The row menu

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-4.1 | Right-click a row that is not highlighted. | The row is highlighted and a menu opens at the pointer. Nothing runs. The first items are the full message, then the detail lines, **all disabled**; a separator; the row's offers, the button's first; Dismiss when the row can be dismissed. | A10, P16, P17 (twin: `rows_by_pointer`, `the_row_menu_by_pointer_and_keyboard`; a real right click has none) | |
| S-4.2 | Press Esc. | The menu closes; focus is where it was before the click. | A10, P17 | |
| S-4.3 | Press Ctrl 4, Down to a row, then **Shift F10**; Esc; again with **the menu key** (if your keyboard has none, write "no key"). | The menu opens beside the highlighted row, with Up, Down and Enter working in it. Esc closes it and focus is back on the **list**. Both keys do the same. | A10, P17 (the real key has no automated twin) | |
| S-4.4 | Open the menu of a row with a button and choose its first offer. | It runs once, the handler the button runs. The menu closes. | A10, A11, P18 (twin: `each_action_runs_its_handler`) | |
| S-4.5 | Open the menu of a share that can be abandoned or republished (a stopped or failed share, a pending operation). | **Abandon…**, "Stop trying to share" and "Stop and republish…" are in the menu and are **never the row's button**. | A10, P18 (design B2) | |
| S-4.6 | Choose **Dismiss** on a failed request. | The row leaves Activity and the footer's count drops by one. | A1, P19 (twin: `clear_finished_keeps_failures_and_states`) | |
| S-4.7 | **(rare)** Open a menu, then make its row leave (Clear finished or the state resolving). | The menu closes by itself and focus goes to the list. | P17 (twin: `the_row_menu_by_pointer_and_keyboard`) | |

## S-5 Activity by keyboard

Use no pointer in this section unless a step says so. Every stop must show a
ring.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-5.1 | Press Ctrl 4 (or F6 to Activity). | Activity's region ring shows and the list has focus, the first row highlighted. | A10, P15 (twin: `activity_by_keyboard`; the ring is appearance) | |
| S-5.2 | Press Down to the last row, then Down again; Up to the first, then Up again; then End and Home. | The highlight stops at each end, **no wrap**. End highlights the last row and Home the first. The list scrolls to keep it in view. | A10, P15 | |
| S-5.3 | From the list press Shift Tab until focus is on the header, and write down the order of the stops; then Tab forward. | Tab runs: the filter, ⋯, the list, then the highlighted row's button. Other rows' buttons are not tab stops. | A10, P15 (twin: `activity_by_keyboard`) | |
| S-5.4 | Highlight a row with a button and press Enter on the list; then Tab to its button and press Enter. | The first runs the button once. The second runs it once: one action, not two (one dialog, not two). | A10, P15 | |
| S-5.5 | Press Enter on a row with **no** button (a success), and on a row whose button is disabled ("Checking…", "Finishing", "Still running…"). | The first opens the row's item when it has one; the second does nothing (decisions 11 and 24). | A10, P15 | |
| S-5.6 | Move the highlight, toggle the filter, open and close a row's menu, ⋯ and the popover. Then look at Activity. | Nothing is sent: Activity gains no row from any of them. A left click on a row highlights it and focuses the list and runs nothing; a click on its button runs it once. | A11, P16, P31 (twin: `navigation_sends_nothing`, `rows_by_pointer`) | |
| S-5.7 | From a row's button, menu or Enter, choose an action that opens a dialog (Review changes, Republish…, Open…, Abandon… from the menu). Press Cancel. | Focus is back on the **list**, with the **same row** still highlighted, whichever way you opened it (decision 23). | A10, P20 (twin: `a_dialog_from_a_row_returns_focus_to_its_row`; a real display has none) | |
| S-5.8 | **(rare) Watch.** Focus a row's button, then make its row leave (the state resolving). | Focus moves to the list, so Esc and the arrows still work. Write what you saw, including a focus that vanished. | P15 (twin: `activity_by_keyboard`; the real display has none) | |

## S-6 The footer item and the popover

Make something that needs attention first: a failed request, a locked key
and a Can't reach row give the frame's three kinds (frame 2f).

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-6.1 | With nothing needing attention, look at the footer; then make one thing need it. | At 0 the item is not drawn. At 1: "1 needs attention"; at more: "{n} need attention", in the footer's right group beside "Checked for updates" and "N to share", as in frames 1a and 2f. | A14, A7 (appearance: no automated twin) | |
| S-6.2 | Hover the item. | The tooltip: "Things that need you. Your work is safe until you act." | A14, A2 (no automated twin for the tooltip) | |
| S-6.3 | Press Tab until the item has focus (a ring shows). Press Enter, then Esc, then Space, then Esc. | Each press toggles the popover and does not reveal Activity. Esc returns focus to the footer item. | A2, P22 (twin: `the_popover_opens_lists_and_closes`) | |
| S-6.4 | Click the item, then click it again. | The first opens the popover, the second closes it. | A2, P22 | |
| S-6.5 | With the popover open, click the footer item once. | It closes **and stays closed** (it does not close and reopen). | P25 (twin: `the_popover_closes`; a real click has none) | |
| S-6.6 | Open the popover against frame 2f. Measure, or judge by eye, its edges against the footer item and the footer's top edge. Write what you measured. | 440 px wide, right-aligned to the footer item. Its bottom edge sits **6 px** above the footer's top edge. It is no taller than the window less the title bar and footer, and scrolls if there are more rows. | A14, A2 (no automated twin) | |
| S-6.7 | Read the header. | "Needs attention", then "Your work is safe on this computer. Nothing here happens until you choose." | A14 (no automated twin) | |
| S-6.8 | Read the rows against frame 2f. | One row per attention row, **newest first**, each with icon, title, description, its button and its tag. The count equals the number of rows (S-3.2). Repeats about one operation show once here and all in Activity. Hover a row: the same tooltip as its Activity row. Last: the link "Show everything in Activity". | A14, A1, A7, P13, P22 (twin: `count_and_popover_agree`) | |
| S-6.9 | Look at the popover in the dark theme. | Nothing unreadable; readable against the item area behind it; colour is never the only signal (every row has an icon and words). | A14, A16 (no automated twin) | |
| S-6.10 | Open it by keyboard (Enter on the footer item). | The popover's list has focus, the first row highlighted. | A2, P22 | |
| S-6.11 | Press Down to the last row and again; Up to the first and again; End; Home. | The highlight stops at the ends, no wrap. | A2, P23 (twin: `popover_actions_by_keyboard`) | |
| S-6.12 | Press Tab, Tab, Tab, then Shift Tab. | Three stops, wrapping: the popover's list, the highlighted row's button, the link "Show everything in Activity". Shift Tab reverses. Focus never leaves the popover. | A2, P23 | |
| S-6.13 | Highlight a row with a button and press Enter. | The popover closes, then the button's action runs (for example Retry now). Focus goes to the footer item if it is still shown, else to the next control to its right. | A2, P23, P25 (twin: `popover_actions_by_keyboard`) | |
| S-6.14 | Highlight a row with no button, press Enter; then one with a disabled button. | The first opens its item and the popover closes. The second does nothing and the popover stays open. | A2, P23 | |
| S-6.15 | Open it, press Esc. | It closes; focus is on the footer item. | A2, P22 | |
| S-6.16 | Open it, then press on the item area, a list row or a toolbar button outside it. | It closes **and** the press acts on what was under it (K-11). The press keeps the focus it gave; if it gave none, focus goes to the footer item. | A2, P25 (twin: `the_popover_closes`) | |
| S-6.17 | **(rare) Watch.** Resolve the last attention row while the popover is open, and again while the footer item has focus. | The popover closes at 0 and the item hides. Focus moves to the next shown control to its right: the right dock group's first control, else Settings. This move is made while the footer draws and has never been seen on a real display: write what you saw, including a focus that vanished or a ring on nothing. | A2, P25, P26 (twin: `the_popover_closes`; the real display has none) | |
| S-6.18 | From a popover row choose an action that opens a dialog (Review changes, Unlock…). Cancel. | Focus returns to the footer item, or, if it has hidden meanwhile, to the next control to its right (E1). | A2, P25 (twin: `the_popover_closes`) | |
| S-6.19 | **(rare)** With the popover open, press Shift F10 on a row elsewhere or right-click one, close that menu, and press Esc. | Focus is on the footer item (or its right neighbour if hidden), not lost. | P25 | |

## S-7 Activity hidden

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-7.1 | Hide Activity with its footer toggle. Look at the footer. | The Needs attention count is unchanged (it does not depend on Activity being drawn). | A3, P24 (twin: `the_popover_works_with_activity_hidden`) | |
| S-7.2 | Open the popover (click, then by keyboard). | It opens and lists the same rows as before; the keys of S-6 work. | A3, P24 | |
| S-7.3 | Run a popover row's button (for example Retry now or Unlock…). | It runs. Activity stays hidden. | A3, P24 | |
| S-7.4 | Turn the filter **on**, hide Activity, open the popover and press **Show everything in Activity** (and again by Tab to the link and Enter). | The popover closes; Activity is revealed (its dock opens if closed); the filter is **off**; the list has focus with the first row highlighted. | A3, P24 (twin: `the_popover_works_with_activity_hidden`) | |
| S-7.5 | Hide Activity, save a ticket (Ctrl S), wait a minute, show Activity. | The save's row is there with the time of the save, not of showing, and in the right place. | A3, P29 (twin: `activity_hidden_keeps_the_log`) | |

## S-8 Narrow Activity and the K-19 rows

Every row is one line and its button never leaves the row: the message
shrinks first (decision 22). This is the on-screen twin of
`rows_fit_a_narrow_activity`.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-8.1 | At the default 1280 px window, Activity in the bottom dock (about 550 px wide there), look at every kind of row you made in S-2. | Every row is one 28 px line. The message ends in an ellipsis when it is long. Every button is fully inside the list. | A8, P21 (twin: `rows_fit_a_narrow_activity`; the look has none) | |
| S-8.2 | Move Activity to the right dock (right-click its footer toggle, Dock right), then drag the dock's edge until it is 320 px wide. Make the longest message of each kind you can, with the long-named repository (a share failure, a locked key, a Can't reach row, a hold over two repositories so its tag reads `{repo} +{n}`). | At 320 px **no row's button is clipped outside the list**; the message is ellipsized; the tag shrinks with an ellipsis after the message. Scroll each row into view to check. Write which row, if any, loses its button. | A8, P21 (twin: `rows_fit_a_narrow_activity`) | |
| S-8.3 | At 320 px, hover an ellipsized row. | The tooltip carries the full message (title, then description), then the time, then the detail. | A8, P14 (twin: `rows_draw_from_the_log_and_filter`; the tooltip has none) | |
| S-8.4 | **K-19: the draft-flush row.** Make a draft that cannot be saved (`chmod -w "$SCRATCH/manyhands/drafts"` while you edit a ticket; `chmod +w` afterwards). | A one-line row "The recovery copy of “{title}” wasn't saved" (or "the ticket {short}", K-2), with its reason, and **Retry**, Retry fully visible at 550 px and at 320 px. | A8, P21 (Cycle 01 checklist, W-10.1) | |
| S-8.5 | **K-19: a pending repository row.** Make a repository whose unfinished-operations read fails (Cycle 01 checklist, W-10.2: rename its folder, or use the long-named one). | A one-line row "Couldn't read the unfinished operations of {repo}" (or the "busy" form) with **Retry**, fully visible at 550 px and at 320 px. | A8, P21 | |
| S-8.6 | **K-19: a pending row with several controls.** Make a pending operation that offers Resume, Review changes and Abandon… (Cycle 01 checklist, W-10.3: a stranded share). | One line. The button is Resume (or Review changes) and is fully visible at 550 px and at 320 px; the other controls are in the row's menu, and Abandon… is never the button. | A8, P18, P21 | |
| S-8.7 | **(rare) K-19: recovered drafts.** With a recovered-drafts row (S-2.12), look at it at 320 px. | One line, with **Review** fully visible. | A8, P21 | |
| S-8.8 | **(rare)** Shrink Activity's height to its default of about 150 px with both layout notices and a pending row showing. | The notices and the quit row stay at the top; the list below scrolls vertically; nothing wraps. | A8 (Cycle 01 checklist, W-10.4) | |

## S-9 Dialogs, sheets and one layer at a time

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-9.1 | Open a dialog (Close ticket…, or New ticket…) with a row highlighted. Press Enter, Shift F10 and the menu key with focus where it can be; right-click a row; click a row's button, the filter, ⋯ and the footer item; press the popover's keys. Close the dialog. | Nothing acts behind the dialog: no menu, no popover, no filter change, no action run. | A12, P27 (twin: `nothing_acts_behind_a_dialog_or_sheet`) | |
| S-9.2 | Do the same behind Settings (Ctrl ,), a sheet. | Nothing acts behind the sheet either. | A12, P27 | |
| S-9.3 | Open a row menu, then open the popover (footer item); close it, open ⋯, then right-click a row; then open a dock menu (right-click a footer toggle) and Cycle 03a's repository menu. | Only **one layer is open at a time**: opening one closes any other. The popover also closes when ⋯, a row menu, a dock menu or the repository menu opens. | A2, P25 (twin: `the_popover_closes`) | |
| S-9.4 | **(rare)** With the popover open, trigger a dialog (a passphrase prompt that a background check raises, or Ctrl N). | The popover closes (it is dropped under a dialog); focus lands on the footer item or its right neighbour once the dialog closes. | A12, P25 | |

## S-10 "N to share" and the reminder rows

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-10.1 | In `shared` save several tickets without sharing and add a commit to `main`; count the rows reading "isn't shared yet" (S-2.8) in Activity, with the filter off, and read the footer's "N to share". | The same number: one reminder row per item not shared yet, **main counted as one**. | A15, P32 (twin: `reminders_match_to_share`) | |
| S-10.2 | Share one of the saved tickets, then Share main. | The footer's count drops by one each time and so does the number of reminder rows. | A15, P32 | |
| S-10.3 | With work to share in two repositories (`shared` and `second`), compare once more. | The footer's count is the total across repositories; the reminder rows add up to it. | A15, P32 | |
| S-10.4 | With reminder rows and a "no shared copy" information row showing and nothing else wrong, read the Needs attention footer item and turn the filter on. | The item is not drawn and the filter shows "Nothing needs attention": reminders and information rows are never counted. The `local` information row stays all session. | A1, A15, P2 (twin: `attention_counts_each_thing_once`) | |

## Results

Write the build (commit), display server, theme, scale factor, and one line
per section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, theme, scale factor | |
| S-1 | Launch and the first look (A6, A14, A16; P14) | |
| S-2 | What each row says (A1, A4, A5, A6, A15; P1, P4 to P8, P28) | |
| S-3 | The filter and ⋯ (A1, A7, A10; P2, P13, P14, P19) | |
| S-4 | The row menu (A1, A10, A11; P16 to P19) | |
| S-5 | Activity by keyboard (A10, A11; P15, P16, P20, P31) | |
| S-6 | The footer item and the popover (A1, A2, A7, A14, A16; P13, P22, P23, P25, P26) | |
| S-7 | Activity hidden (A3; P24, P29) | |
| S-8 | Narrow Activity and the K-19 rows (A8; P21) | |
| S-9 | Dialogs, sheets, one layer at a time (A2, A12; P25, P27) | |
| S-10 | "N to share" and the reminder rows (A1, A15; P2, P32) | |
| — | Known limits hit (K-numbers, steps) | |

## Findings

Fill this in as you go. Claude records only what you report.

| # | Step | What differs from the frames, or what you would change in the design | Where it goes (design amendment, follow-up ticket, nothing) |
| --- | --- | --- | --- |
| 1 | | | |
| 2 | | | |
| 3 | | | |

## Not reached

List here anything the checklist could not reach, and why.
