# Wave 04 Cycle 05a On-Screen Checklist

This list is for checking Cycle 05a (the common dialog frame, the Add
repository sheet, the Unlock key dialog and the empty item area) by hand on
Linux Wayland before the Cycle is called review-ready. It is the native
evidence the design asks for (design, "Testing", Part E, A2 and "Unverified"
U-2 and U-7; Cycle document, A13, "On screen"; implementation plan, Task 10
Step 1), and it carries what no test sees: the Add sheet against frame 1d, the
Unlock dialog against frame 2h, the frame on each dialog with its two
hand-drawn rings, the theme slots, the empty pane's icon, and every item the
ledger marks "on screen" or "owner's eye". Work through sections V-1 to V-9
in order. Write each step's result in its row's last column, and a one-line
summary per section in "Results" at the end. The owner runs every row: **no
row is marked passed here, every result cell is empty, and Claude records only
what you report.**

Quoted text is the app's own wording. "Ctrl" is the `secondary` key of the
keymap on Linux. A key written `Ctrl ,` or `Shift Tab` is held together.
"The design" is
`docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md`;
"G13" is a row of its B9 (the focus contract), "AW-3" a way into the Add
sheet (B2), "R-27", "K-3" and "U-2" its rulings, known limits and unverified
facts, "Part D" the Unlock dialog and "Part E" the empty pane. "The Cycle
document" is `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`
("A13" is its on-screen acceptance row, "decision 16" one of its decisions).
"The ledger" is
`docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md`
("Ruling 12" and "Task 8 Minor 1" name its entries). "The frames" are
`docs/design/screenshots/1d-add-repository-sheet.png` and
`2h-unlock-key-dialog.png`, read with `docs/design/05-sheets-and-dialogs.md`
(the common frame, the Add repository sheet, the Unlock key dialog),
`04-item-area.md` ("Empty pane") and `06-language-and-states.md` (the
vocabulary).

The **Supports** column names the Cycle document's Acceptance row and the
design's ruling, known limit or unverified fact the step is evidence for. A
step with no automated twin says so. Steps marked **(rare)** need a state that
is hard to make by hand: try each one, and if you cannot reach it write
"skipped". Steps marked **Owner decides** ask you to look and say what you
want; they are not pass or fail. Steps marked **Watch** name something that has
never been seen on a real display: write what you saw, either way.

## Minimum pass

If you cannot do every row, these cover each part of Task 10 Step 1 at least
once. Nothing below is removed from the list; the other rows add depth.

- **A13, the Add sheet against frame 1d (owner's eye, no automated twin):**
  V-2.1 to V-2.5.
- **A13, the Unlock dialog against frame 2h:** V-5.1 to V-5.4.
- **The frame on each A3 dialog, and its two hand-drawn rings:** V-6.1,
  V-6.3, V-6.4, V-6.9, V-6.10, V-6.12.
- **The theme slots and the mode the app shows (U-2, A2):** V-1.2, V-1.3,
  V-8.1 to V-8.4.
- **The empty item area's icon and its 12 px gap (Part E):** V-7.1.
- **The ledger's on-screen items:** V-3.1 (Task 1, top padding), V-8.2
  (Ruling 12), V-5.5 (Task 8), V-4.6 (P23 at 150 px), V-4.7 (Ruling 22),
  V-4.1 (one mode at a time), V-5.3 (width 440), V-2.1 (width 470), V-3.4
  (AW-3 and Escape), V-8.3 (Task 1 U-7).

Mostly covered by automated tests, or rare, so **optional** if time is short:
V-3.5, V-4.9, V-4.10, V-6.2, V-6.11, V-6.13, V-9.3.

## Before you start

- **Use a scratch data directory for everything.** This checklist registers
  and creates repositories, makes drafts and opens the Unlock prompt. Run all
  of it against a throwaway data directory so your real registry is never
  touched, and run only from a build of this branch. Make it, launch with it
  and check it took exactly as Cycle 04's checklist does
  (`docs/plans/2026-11-13-wave-04-cycle-04-on-screen-checklist.md`, "Before you
  start": the desktop reads `XDG_DATA_HOME`, so the scratch data directory is
  `$SCRATCH/manyhands`).
  - **Make it** (one terminal, kept open for the whole checklist):

    ```sh
    export SCRATCH=$(mktemp -d)
    touch "$SCRATCH/.start"
    mkdir "$SCRATCH/repos"
    ```

  - **Smoke test (V-1.1), by you, on Wayland.** From the ticket worktree
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7D5Z7FZTGTC1HE25KQ1MYQY`,
    the debug build the Agents file names, with the scratch directory:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --locked --features desktop --bin manyhands`.
    Quit it (Ctrl Q) when the window is up.
  - **Launch with it** for every step after V-1.1: the release build,
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`.
    It needs an active desktop display. A development build lags. Run it while
    no long test suite or build is running on the machine.
  - **Check it took.** `ls "$SCRATCH/manyhands"` lists `layout` and the
    registry's files.
  - **Your real data stays untouched,** except keys: Shared SSH keys live in
    `~/.ssh/manyhands`, outside the scratch directory. Use only a key you make
    for this list, never your own. At the end
    `find ~/.local/share/manyhands -newer "$SCRATCH/.start"` prints nothing
    (adjust the path if your `XDG_DATA_HOME` is normally set).
- **Folders and repositories** (under `$SCRATCH/repos`; make them with plain
  `git init` and, where a commit is needed, a commit made with
  `git -c user.name=x -c user.email=x@x commit` since this machine has no
  global Git identity):
  - `plain`: a Git repository with one commit on `main`, no Manyhands
    configuration (V-2, V-3, V-4);
  - `ident`: a repository set up by Manyhands (add `plain` once, or create
    one) whose Git has an identity, for "Already added" and the Open its
    settings row (V-4.3);
  - `noident`: a repository with a **Valid configuration and no Git name or
    email** (set up by Manyhands, then `git config --unset user.name` and
    `user.email`), for the held share select (V-4.8);
  - `tworemotes`: a repository with two remotes, one SSH address
    (`git@example.com:x/y.git`) and one HTTPS, for the share select (V-4.8);
  - `empty`: a Git repository with no commit, for "no branch" (V-4.2);
  - `notgit`: a plain empty directory (V-4.2, the Create one here row);
  - `many`: a few more registered repositories, so the Repositories panel has
    rows and a selection to change (V-3.7).
- **A shared SSH key with a passphrase** and a repository that shares through
  it, as Cycle 03a's "Before you start" makes it
  (`docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md`), for the
  Unlock prompt (V-5). Lock it by restarting the app, then start a share (an
  Edit saved on a shared ticket, or Check now on Checking for updates).
- **A draft** (V-6): Edit a ticket, change its body and leave it unsaved.
- **The dark theme and the scale factor.** Run in the mode the app shows (see
  U-2 at V-1.2) and write down in "Results" which mode and which scale factor
  you ran.
- **A window size.** The app opens at 1280 x 840. V-4.6 needs a window as short
  as the app allows.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure. The
first group is this Cycle's design "Known Limits"; the second is what the
frames draw and this Cycle does not build; the third is what is not run on this
machine.

- **K-1.** No "You can save changes here" row: no read tells the desktop
  whether a folder is writable.
- **K-2.** No inline error for a wrong passphrase: Unlock closes as today and
  the job fails (decision 4). The inline `REFUSED` covers only an empty or
  NUL-holding passphrase.
- **K-3.** One scrim colour, `#0a0b0d99`; a dialog over a sheet draws two
  scrims.
- **K-4.** The Kit's focus ring: 3 px at 50%, in teal, not `05`'s 2 px at 18%.
- **K-5.** A sheet's header and footer dividers stop short of its edges.
- **K-6.** The Unlock note makes no "until you quit" promise
  (`01M70CF14M3YC6SR8T9JJ6XQBG`).
- **K-8.** The mode control's two options are two Tab stops with Enter and
  Space, not arrow keys.
- **K-9.** No File › Add repository entry.
- **K-10.** A chosen remote waiting for its add is lost if the app quits first.
- **K-11.** A dialog taller than the window scrolls its footer with its body.
- **K-12.** Create one here is offered on any folder that is not a repository;
  a folder that is not empty is refused by the confirmation.
- **K-13.** The secondary button, input background and sheet border and
  shadow keep the Kit's where its theme has no key (see V-8).
- **K-14, K-15.** A failed share selection after an add offers no new action;
  a folder already set up but with no Git identity cannot choose its sharing in
  the sheet.
- **K-16 (ledger, Ruling 19 C-2).** A confirmation stacked under an Unlock
  prompt cannot close itself: its close reaches the topmost dialog, the prompt.
  It predates this Cycle and affects every confirmation. Do not try to
  reproduce it.
- **Frames, not failures.** Frame 1d draws a "You can save changes here" row
  (K-1) and a header description and a note that run past the sheet's right
  edge in the mockup (the app wraps them). Frame 2h draws a show and hide eye at
  the right of the field (Wave decision 9: none), the note "Kept in memory until
  you quit Manyhands. It's never written to disk. If you choose Not now, you
  can…" (the app's wording is Part F's, K-6) and the card's "Vince's laptop /
  ED25519 · SHA256:…" (the app draws the key's label and fingerprint). Both
  frames also draw the title bar's menus and scope button, the Repos tab strip
  and `MH-VH-…` upper-case short codes.
- **Not run on this machine.** macOS and Windows are not run locally; native CI
  covers them after merge (ledger, Task 6 Minor 1 and Task 7 Minor 6 name two
  path-separator rows).

## V-1 Launch and the theme (A2, U-2)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-1.1 | Run the smoke test above, then the "Check it took" listing. | A window opens with no error in the terminal; the title is "Manyhands"; `$SCRATCH/manyhands` lists `layout`. Ctrl Q closes it. | A13 (no automated twin) | |
| V-1.2 | **Watch (U-2).** In the release build, say which mode the app shows: dark or light. Write it. | The design set both modes' slots and left the current mode in force; the mode the app shows must not have changed from what it showed before this Cycle (the ledger's test `applying_the_theme_keeps_its_mode` pins the order, not what you see). Write the mode, and if the window flickers between modes at start. | A13, R-2, U-2 (twin: `applying_the_theme_keeps_its_mode`; which mode is shown has none) | |
| V-1.3 | Look at every primary button you can reach (Save, Close ticket…, a dialog's Create) and at a focus ring on any input (click into the search box or a field). | The primary button is amber with dark text, in the mode shown. A focused input shows a teal ring and a `border-strong` border. Write any primary button that is not amber and any ring that is not teal. | A13, A2, K-4 (no automated twin) | |

## V-2 The Add sheet against frame 1d

Open the sheet with the Repositories toolbar's `+` (or, with no repository
registered, the panel's "Add or create a repository…"), in the existing-folder
mode, with `plain`'s path typed and checked. The frame draws a sheet on the
right, a header, a mode control, the folder field with Browse…, the checks, the
main branch select, "Share changes through", a note, and a footer.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-2.1 | **Watch (width 470).** Open the sheet and look at the whole window against frame 1d. | A sheet 470 px wide slides in from the right over a dimmed window; the item, the panels and the footer stay visible behind a scrim. Its header, the Kit's close button at the header's right and a footer with Cancel and Add repository at the foot. Write the width you judge it to be, and anything that differs from the frame. | A13, A2 (no automated twin) | |
| V-2.2 | Read the header. | "Add a repository" (15 px, semibold) and under it "Bring a folder you already work in, or start a new one. Manyhands keeps its tickets and documents in the repository." Both whole and wrapped inside the sheet, not clipped; the close button fully visible. The frame's description is cut off at the edge (R-23). | A13, R-23, K-5 (no automated twin) | |
| V-2.3 | Look at the mode control, the Folder row and Browse…. | Two segments, "Add an existing folder" (`folder-input`, chosen) and "Create a new repository" (`folder-plus`); under them the label "Folder", a field with a `folder` icon and a **Browse…** button beside it. The chosen segment has a lighter background. | A13, B5 (no automated twin for the look) | |
| V-2.4 | With `plain` typed and checked, read the check rows, "Main branch", "Share changes through" and the note against the frame. | Rows of 12 px text, each an icon and a sentence: "This folder is a Git repository." and "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change." (the frame's "You can save changes here" row is not built, K-1). "Main branch" is a select with a `git-branch` icon, `main` chosen and a teal "Suggested" tag; under it the help "The version everyone shares. Finished tickets and approved documents are merged here." "Share changes through" is a select with a `cloud` icon (with no remote: "Keep on this computer"), and under it the help "You can also choose "Keep on this computer" and set this up later. Only SSH addresses can be used for sharing." Then a box with a teal `shield-check` and "Removing a repository from Manyhands later never deletes, moves or changes its files." Write what differs from the frame (spacing, fonts, icon sizes, colours). | A13, A7 (twins: `folder_checks_are_rows_with_their_actions`, `the_main_branch_is_the_checked_out_one`, `share_changes_through_lists_the_folders_remotes`; the look has none) | |
| V-2.5 | Look at the footer and the sheet's dividers. | Cancel then Add repository, right-aligned; Add repository amber with dark text. The header's and footer's divider lines stop short of the sheet's edges (K-5); say whether that looks wrong. The note box and the other rows are not clipped on the right. | A13, K-5 (no automated twin) | |
| V-2.6 | Switch to "Create a new repository". | **One form at a time** (V-4.1): the existing form hides and the Create form shows: Name, Location with Browse…, a line "Empty folder will be created at {location}/{name}" once both are filled, Main branch (a text field, `main`), a short code prefix, "Your name & email" with its note, and the shield note. Neither mode loses its typed text when you switch back. | A13, B5, B8 (twin: `create_joins_location_and_name`; the look has none) | |

## V-3 Ways in, Settings and the ways out

Each row: do the **way in**, look at the sheet and the control with the focus
ring, then close it and read where focus goes. If you cannot reach a state
write "skipped".

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-3.1 | **Watch (Task 1).** Open any dialog with a body (V-6.1 is the easiest) and look at the gap between the title and the first line of the body. | The Kit draws a fixed 8 px under the title; the frame's body takes a top padding of 10 (not the design's 18) to make up the difference. Write whether the gap under the title looks right, too tight or too loose against the gap above the footer. If it looks wrong, say 10 or 18. | A13, ledger Task 1 (no automated twin) | |
| V-3.2 | AW-2: press the Repositories toolbar's `+`. Then Esc. | The Add sheet opens in the existing-folder mode with the ring on the folder field. Esc closes it and the ring is back on `+`. | A3, A5 (twins: `each_way_in_opens_the_add_sheet`, `focus_returns_to_what_opened_add`) | |
| V-3.3 | AW-1: with no repository registered (use a fresh scratch directory), press the empty panel's "Add or create a repository…". Then Esc. | The sheet opens with the ring on the folder field. After Esc the ring is on the panel (the button has gone once a repository exists; on the panel's region then). | A3, A5 (twins as above) | |
| V-3.4 | **Watch (AW-3, return to Settings).** Open Settings (Ctrl `,`), on Repositories press "Add or create a repository…". Look at the screen, then close the sheet once by each of Esc, the header's close button, a click on the dimmed area and Cancel, reopening the way in each time. | Settings closes and the Add sheet opens in its place (one sheet, never two at once). Each close brings Settings back at Repositories with the ring on "Add or create a repository…". Then Esc closes Settings and focus goes to where Settings was first opened from. Write where each close puts the ring, and any flash of a ring on a control behind the sheet. | A4, A5, AW-3, G4 to G6 (twin: `add_from_settings_returns_to_settings`; the ring and any flicker have none) | |
| V-3.5 | **(optional) Settings after the move (B11).** Open Settings on Repositories and read the page. | The Add and Create forms are gone. Where they were the page draws one button, **Add or create a repository…**; the managed section (each registration with Manage, the identity form, Remove repository…) is as in Cycle 04. A passphrase typed on Sharing & keys is already emptied by leaving that page, so the sheet's own emptying cannot be seen by hand: `add_from_settings_returns_to_settings` (G3) covers it. | A4, B11 (twin: `add_from_settings_returns_to_settings`; the look has none) | |
| V-3.6 | **Owner decides (decision 19).** With the Add sheet open press Ctrl `,`. | Nothing happens: Settings does not open over the sheet. Escape first, then Ctrl `,`. Say whether you want it. | A5, decision 19 (twin: `nothing_acts_behind_the_add_sheet`) | |
| V-3.7 | **Owner decides (Task 4 Minor 5).** Choose a different repository in the Repositories panel than the one Settings last showed, press `+`, then Esc, then Ctrl `,`. | Settings opens on the repository it had shown before, not the panel's new choice: opening the Add sheet counts as having seen the panel's selection (Ruling 15, realised on every open, `+` included). In Cycle 04 `+` opened Settings, which chose the panel's repository. Say whether that is what you want. | A5, Ruling 15, ledger Task 4 Minor 5 (no automated twin) | |
| V-3.8 | **Owner decides (decision 16).** Type a folder in the sheet, close it by Esc, open it again; then add a repository successfully and open it again. | After Esc the typed folder is kept. After a successful add the sheet closes and the next open is empty. Say whether that is right. | A3, A6, decision 16 (twin: `the_add_sheet_closes_every_way`) | |
| V-3.9 | Press Add repository on a valid folder, then confirm in the confirmation. | The confirmation opens over the sheet; Confirm adds, the confirmation closes, then the sheet closes; opened from Settings, Settings comes back at Repositories with the new repository chosen (its form and Remove showing). Opened any other way, focus is back on what opened the sheet. Write any moment the sheet closes under the confirmation or any flicker. | A5, G5, G14 (twin: `focus_returns_to_what_opened_add`) | |
| V-3.10 | Press Add repository, and while the confirmation runs the add, look at the sheet's button, then press Esc. | While the add is pending the button reads "Adding…" and is disabled. Esc closes the confirmation (Escape never cancels the work), and the ring is on the button's own place. Write whether a ring shows on the button, and where focus is. | A5, R-27, Ruling 18 (twin: `focus_returns_to_what_opened_add`, row `while_adding`; the ring has none) | |
| V-3.11 | **Owner decides (Ruling 24; rare).** Press Add repository and confirm an add whose answer offers **Retry** or **Prepare again** (as V-6.2: let its preview go stale before you confirm). Close the confirmation and look at the sheet's footer. Then, one at a time: retry from Activity; dismiss the request in Activity; close the sheet and open it again. | After the confirmation closes, Add repository and Create repository still read "Adding…" and are disabled until you retry from Activity, dismiss the request, or close and reopen the sheet: the add is still waiting (B10, R-27), by design. A retry that succeeds closes the sheet; dismissing, or the next open, gives the buttons back. Say whether that is what you want. | A5, B10, R-27, Ruling 24 (twin: `focus_returns_to_what_opened_add`, row `while_adding`, pins "Adding…" while an add is pending; the stay after a Retry or Prepare again answer has none) | |

## V-4 The forms on screen (Add and Create)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-4.1 | **One form at a time.** Switch the mode control by Tab (to a segment), Enter and Space, and by click. | Exactly one form shows at a time; each segment is its own Tab stop; Enter and Space choose it; arrow keys do nothing (K-8). | A6, B5, K-8 (twin: `add_and_create_by_keyboard`; the look has none) | |
| V-4.2 | **Watch (the mode control's focus ring).** Tab to each of the two segments. | A visible ring that fits each segment and is not clipped. It is the control's own ring, drawn separately from the filter bar's (ledger Task 5 Minor). Say what it looks like. | A13, ledger Task 5 (no automated twin) | |
| V-4.3 | Type each of these into the folder field and press Tab away (or Enter), one at a time: `notgit` (not a Git repository), `empty` (no commit), `ident` (registered). Read the rows. | `notgit`: "Not a Git repository." with a **Create one here** button. `empty`: "No branch is checked out. Check one out, then check again." with **Check again**. `ident` (registered): "Already added to Manyhands." with its configuration's state and problems and **Open its settings**; Add repository disabled; no main branch or share select. Add repository is disabled for each. Write the wording and the buttons you see. | A6, A7, A12 (twins: `folder_checks_are_rows_with_their_actions`, `add_is_enabled_only_when_it_can_go_ahead`; the look has none) | |
| V-4.4 | On `notgit`'s row press **Create one here**. | The mode control switches to Create with Location the parent folder and Name the last part, and the ring on Name. The confirmation, when you press Create repository, refuses a folder that is not empty (K-12). | A6, G20 (twin: `folder_checks_are_rows_with_their_actions`) | |
| V-4.5 | **Watch (Task 6).** In Create, look at the two fields labelled "Name" (the folder's name near the top, the identity's below). | Two fields with the same label; the identity block's heading "Your name & email" tells them apart. Say whether that is clear. | A13, ledger Task 6 Minor 3 (no automated twin) | |
| V-4.6 | **Watch (P23 at 150 px).** Make the window as short as the app allows (or about 150 px of sheet body). In the Add form press Page Down, Page Up, End and Home with focus on a control, then Tab to a field below the fold. | The page keys scroll the **sheet's body**, not the item area behind it; the control you Tab to is scrolled into view, with its ring. Inside a text input Home and End move the cursor. Write what you see. | A5, P23, G18 (twin: `the_add_sheet_scrolls_on_its_own`; the look has none) | |
| V-4.7 | **Watch (Ruling 22, Retry and Check again focus).** By keyboard only: with `empty` checked press Tab until **Check again** has the ring, press Enter. Do the same for a Choose another row (a failed check that is not NotRepository) and for Retry (a busy check; rare, skip if you cannot make one). | Check again, Retry and Choose another each leave the ring on the **folder field** (`add-path`), not on nothing: Enter in the field checks again and Tab moves on. Write where the ring is after each, and whether Enter then does something. | A5, A6, Ruling 22 (twin: `add_and_create_by_keyboard`; the ring has none) | |
| V-4.8 | **Share changes through.** On `tworemotes` open the select; then on `noident` read the select. | `tworemotes`: one item per remote with its name and, in mono, its address (a password in an address never shows); the HTTPS one greyed with "Only SSH addresses can be used for sharing." and refused when you press Enter on it; "Keep on this computer" last; chosen: the folder's configured remote, else Keep. `noident` (Valid configuration, no Git name or email): the select is held on "Keep on this computer" and disabled, with the line "Set your name and email in Settings first, then choose how it shares." Write how the held select looks (R-30) and whether the configured remote should show instead of Keep. | A7, A8, R-30, decision 20 (twins: `share_changes_through_lists_the_folders_remotes`, `a_chosen_remote_is_selected_after_the_add_succeeds`; the look has none) | |
| V-4.9 | **(optional) A Valid configuration with no identity.** Choose Keep, then Tab through the form past the held select. | The select's wrapper may stay a Tab stop while held and disabled (ledger Task 7 Minor 3). Say whether Tab stops on it. | A13, ledger Task 7 (twin: P13's Tab test passes) | |
| V-4.10 | **Owner decides (ledger, Task 5 design gap).** For a folder whose Valid configuration names a publication remote that is not among the folder's remotes (edit the settings file to do it), type it and read the rows and the Add button. | No row is drawn and Add stays enabled; the confirmation's preview then refuses it. The design says Add is disabled (C2, decision 9). Say what you would want. | A7, decision 9 (no automated twin) | |
| V-4.11 | **(rare) Prefix.** Type a folder that is already set up (Valid configuration) and look for the "Short code prefix" field. | Not drawn: the configuration holds the prefix (R-31). A set-up folder with Missing configuration shows it. Say whether not seeing the prefix is a problem. | A7, R-31 (twin: `folder_checks_are_rows_with_their_actions`) | |

## V-5 The Unlock key dialog against frame 2h

Make the prompt appear: the key with a passphrase, locked (restart the app), then
start a share. If the dialog appears with a subtitle ("Sharing from {repository}
is waiting for your passphrase."), a share is waiting; one from Check now on
Checking for updates has none.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-5.1 | Look at the dialog against frame 2h. | A dialog centred over a dimmed window: a header with a 32 px tile holding a lock (amber), "Unlock your key" and, while a share waits, the subtitle; a key card; a "Passphrase" field; a note with a `shield-check`; a footer "Not now" then "Unlock" (amber, right-aligned). Write what differs from the frame. | A9, A13, Part D (no automated twin for the look) | |
| V-5.2 | **Watch (the card).** Look at the key card. | The key's label in the card's first line and, under it, its fingerprint in monospace (`SHA256:…`), with a `key-round` icon. Before the read lands (or with no matching key or no fingerprint) the label alone. Write what you see, and whether a long fingerprint is shortened or runs past the card's edge. | A9, A13, R-16 (twin: `the_unlock_dialog_names_the_key`; the look has none) | |
| V-5.3 | **Watch (width 440).** Judge the dialog's width against the frame's. | About 440 px, and the same width with and without the subtitle. Write the width you judge it to be. | A13, ledger Task 8 (no automated twin) | |
| V-5.4 | Read the field and the note. | The label "Passphrase" (no key name in it: the card names the key); the field is masked and focused; **no show and hide eye** (Wave decision 9: the frame draws one). The note, always shown: "Kept in memory only, never written to disk. If you choose Not now, you can keep working and unlock later from Needs attention." | A9, A10, decision 14, K-6 (twins: `the_unlock_dialog_names_the_key`, `frame_copy_uses_the_vocabulary`; the look has none) | |
| V-5.5 | **Watch (Task 8 Minor 1).** Press Unlock with the field empty. | `REFUSED` shows under the field **in red**, focus is back in the field (with its ring, also if you pressed the Unlock button), and no reply is sent: the dialog stays open. Today's `REFUSED` was not red; the frame names no colour. Say whether red is right. | A9, A13, R-17, Task 8 Minor 1 (twin: `a_refused_passphrase_keeps_focus_in_the_field`; the colour has none) | |
| V-5.6 | Press Tab from the field. Then Shift Tab. Then Enter in the field with a passphrase of a wrong key. | Tab reaches **Not now** before **Unlock** (the action last, decision 15); Enter in the field submits once. A wrong passphrase closes the dialog and the job fails, as today (K-2). | A9, decision 15 (twins: `footer_buttons_tab_in_drawn_order`, `unlock_by_tab_and_enter_submits_once`) | |
| V-5.7 | Press **Not now** (or Esc). | The dialog closes, the key stays locked and the share waits in Activity and Needs attention with **Unlock…**; focus returns where it did before this Cycle (the control that was focused). Write where. | A9 (twins: `cancel_by_tab_and_enter_cancels_and_the_key_stays_locked`, `unlock_clears_the_declined_key_and_retries_the_same_request`) | |

## V-6 The frame on each A3 dialog

For each dialog look at: the header (the title, 15 px, semibold, on its own
band with a line under it), the body, the footer (a line above it, the buttons
**right-aligned**, in today's order) and, for the two with a hand-drawn ring,
that ring. Every dialog keeps its keys, focus and return (A1; twin
`every_dialog_keeps_its_keys_and_focus` and the dialogs' own tests); the rows
below look at the frame, not the logic. Write "skipped" for a dialog you cannot
reach.

| # | Do (open the dialog) | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-6.1 | **Confirmation.** Press Add repository on a valid folder (V-3.9). | Header with the command's title; body with the `Root:` line and the preview; footer **Cancel · Confirm** (the action last), right-aligned; Confirm is amber. Tab from the body reaches Cancel before Confirm; Enter still presses Confirm. | A1, A13, decision 15 (twins: `every_dialog_keeps_its_keys_and_focus`, `footer_buttons_tab_in_drawn_order`) | |
| V-6.2 | **(rare) Confirmation, Retry or Prepare again.** Make a preview go stale (change the folder after its preview) and look at the footer. | **Close · Prepare again** or **Cancel · Retry**: the action last. | A1 (twin: P1's Confirmation rows) | |
| V-6.3 | **New ticket and New document.** Press the Tickets panel's `+`, then the Documents panel's new-document button. | Header "New ticket" or "New document"; the form in the body; footer **Cancel · Create**, right-aligned. | A1 (twin: `every_dialog_keeps_its_keys_and_focus`) | |
| V-6.4 | **Create's hand-drawn ring.** In the New ticket dialog fill the form, Tab to **Create** and press Enter (or Space). | While the write runs the button disables and the ring stays drawn **around the Create button** (a 2 px ring in the theme's ring colour), which is where the keyed press parked focus. Write whether the ring shows and whether it sits snug on the button. Repeat on New folder and Move (V-6.5, V-6.6). | A1, A3, `MH-NFR-005`, ledger Task 0 U-3 (twin: `every_dialog_keeps_its_keys_and_focus` for the keyed close; the ring has none) | |
| V-6.5 | **New folder.** Press the Documents panel's new-folder button, fill it and press Create by keyboard. | Header "New folder"; footer **Cancel · Create**; Create's ring as V-6.4. | A1, A3 (as V-6.4) | |
| V-6.6 | **Move.** Choose a document, Move…, pick a folder and press Move by keyboard. | Header "Move…"; footer **Cancel · Move**; Move's ring as V-6.4. | A1, A3 (as V-6.4) | |
| V-6.7 | **Close a tab with a draft.** With the draft (Before you start) open in its tab, press Ctrl W. | Header with the ask; footer **Keep draft · Save · Retry · Discard · Cancel** (as many as apply; "Retry" and the unavailable Save only in their states), right-aligned: they were left-aligned before. | A1 (twin: `every_dialog_keeps_its_keys_and_focus`) | |
| V-6.8 | **The quit asks.** With the draft still unsaved press Ctrl Q (and, if you can, with a request running). | A quit ask: footer **Cancel · Quit anyway**, right-aligned. (Quit now is not a dialog's button: it keeps its own ring where quitting waits, unchanged by this Cycle.) | A1, ledger Task 0 (twin: `every_dialog_keeps_its_keys_and_focus`) | |
| V-6.9 | **Discard a draft.** Open Drafts (Ctrl Shift D), press **Discard…** on the draft. | Footer **Cancel · Discard**. Esc closes it with the draft kept. | A1 (twin: as V-6.7) | |
| V-6.10 | **The composer discard and its hand-drawn ring.** In a ticket's discussion start a reply or comment, type something and press Esc (or Cancel). | Header "Discard this reply?" or "Discard this comment?"; footer **Discard · Keep editing**, right-aligned. **A ring is drawn around Keep editing** as the prompt opens (the focused stop). Write whether the ring shows and sits snug. | A1, A3, `MH-NFR-005` (twin: `every_dialog_keeps_its_keys_and_focus`; the ring has none) | |
| V-6.11 | **Watch (Task 2 Minor).** Open the composer discard from a composer that has no owner note. | The body between header and footer is empty (about 28 px of padding): look at whether the gap looks wrong. | A13, ledger Task 2 (no automated twin) | |
| V-6.12 | **Retire local copy.** In Activity, on an item the loop left alone, press **Retire…**: once for a draft that is kept and once for one that is unkept if you can. | Unkept: footer **Show tab · Cancel**. Kept: **Cancel · Retire**. Both right-aligned. | A1 (twin: `every_dialog_keeps_its_keys_and_focus`) | |
| V-6.13 | **(rare) The repair discard.** Make a repair draft, then discard it. | "Discard your repair?" footer **Cancel · Discard**. | A1 (twin: as V-6.7) | |
| V-6.14 | **(rare) Recovery off and Replace your edits.** With crash recovery off (a scratch data directory you make read-only) Edit asks first. For Replace your edits, let a review bring changes that would overwrite your unsaved edits. | Recovery off: **Continue · Cancel**. "Replace your edits?": **Keep my edits · Replace**. Both on the frame. | A1 (twin: `every_dialog_keeps_its_keys_and_focus`) | |
| V-6.15 | **Two scrims.** Open a Confirmation over the Add sheet (V-3.9) and look at the dimming. | The sheet is dimmed by one scrim and the window by another: two layers of the one colour (K-3). Say whether it looks wrong. | A13, K-3 (no automated twin) | |

## V-7 The empty item area (Part E)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-7.1 | **Watch (icon and the 12 px gap).** Close every tab so the item area is empty. Look at its centre. | The app's icon, 56 px, **grey** (greyscale, 35% opacity), centred above today's empty-pane text, with about **12 px** between the icon and the text. Write the gap you judge, whether the icon is centred and whether it looks too faint or too dark. Also run it in the other mode if you can switch. | A13, Part E (no automated twin; `the_item_area_holds_nothing_of_the_repository` pins the text) | |

## V-8 The theme slots

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-8.1 | Compare the app's amber, teal and the borders with the design tokens (`docs/design/07-design-tokens.md`) in the mode shown. | Primary buttons amber with `#1a1c20` text (dark) or white (light); the ring teal; input borders `border-strong`. Write any slot that looks off. | A13, A2 (no automated twin) | |
| V-8.2 | **Owner decides (Ruling 12).** In the dark mode, hover and press any secondary (default) button: Cancel in a dialog, the Add sheet's Cancel, a panel's toolbar button. | The hover colour is a lighter step (`#353a41`) and the press the same; without the extra `hover_strong` token the hover would be darker than the resting colour. This change reaches **every Default-variant button in the app**. Say whether to keep it. | A13, Ruling 12 (no automated twin) | |
| V-8.3 | **Watch (U-7).** Look at the text inputs (the folder field, the search box, a form's fields) in the dark mode. | The input background is the Kit's, computed from `input.border`, so the new `border_strong` shifts dark inputs a little lighter than before. Say whether the inputs now look wrong against the sheet's body. | A13, U-7, K-13 (no automated twin) | |
| V-8.4 | Open a sheet and a dialog and compare the scrim. | One scrim colour for both (`#0a0b0d99`); a sheet's left border and shadow are the Kit's, not `border-strong` (K-13). Write what you see. | A13, K-3, K-13 (no automated twin) | |

## V-9 By keyboard alone, and copy

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| V-9.1 | Add `plain` by keyboard alone: open the sheet from `+` (Enter), Tab to the field, type the path, Tab to Add repository, Enter, (the identity step if asked), Confirm. | Every step is reachable and works with Tab, Enter, Space and Esc; no step needs the mouse; the ring is always visible. Write any control with no ring. | A6 (twin: `add_and_create_by_keyboard`; the ring has none) | |
| V-9.2 | Create a repository by keyboard alone: switch the mode, fill Name, Location and Main branch, Create repository, Confirm. | The same. Create repository stays disabled until Name, Location and Main branch are filled. | A6, A7 (twins: `add_and_create_by_keyboard`, `create_joins_location_and_name`) | |
| V-9.3 | **(optional) Copy.** Read the Add sheet, the Unlock dialog and Settings' "Add or create a repository…" button against `06`. | Sentence case, no `!`, `…` on **Browse…** and **Add or create a repository…**. **Owner decides (Ruling 21):** "Choose another" and "Open its settings" carry no `…`, though `06` would put one on the second; Part F gives both without it. Say whether to add them. | A10, Ruling 21 (twin: `frame_copy_uses_the_vocabulary`) | |

## Results

Write the build (commit), display server, theme mode shown, scale factor, and
one line per section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, theme mode shown (U-2), scale factor | |
| V-1 | Launch and the theme (A2, U-2) | |
| V-2 | The Add sheet against frame 1d (A13) | |
| V-3 | Ways in, Settings and the ways out (A3 to A5) | |
| V-4 | The forms on screen (A6, A7) | |
| V-5 | The Unlock dialog against frame 2h (A9, A13) | |
| V-6 | The frame on each A3 dialog (A1) | |
| V-7 | The empty item area (Part E) | |
| V-8 | The theme slots (A2) | |
| V-9 | By keyboard alone, and copy (A6, A10) | |
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

## Reported by the owner

| Item | Report |
| --- | --- |
| Date | |
| Build (commit) | |
| Display server, theme mode shown, scale factor | |
| Result (all rows run, rows skipped, rows failed) | |
| Notes | |
