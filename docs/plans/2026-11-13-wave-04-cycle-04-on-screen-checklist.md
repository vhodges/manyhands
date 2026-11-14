# Wave 04 Cycle 04 On-Screen Checklist

This list is for checking the Cycle 04 Settings sheet (the 720 px sheet on the
right, its header and three-entry menu, Repositories, Sharing & keys with its
key rows and Trusted servers, Checking for updates, the ways in and the ways
out, and where focus goes) by hand on Linux Wayland, in the dark theme, before
the Cycle is called review-ready. It is the native evidence the design asks
for (design, "Testing" and "Unverified" U-3; Cycle document, A15, "On screen";
implementation plan, Task 9 Step 1), and it carries what no test can see: the
sheet against frames 1e and 1f, the header's two lines beside the Kit's close
button, the sheet's top edge against the title bar, fields inside the body's
width, a real focus ring, a real click outside, and the focus moves the
ledger reports as "may look unexpected". Work through sections S-1 to S-11 in
order. Write each step's result in its row's last column, and a one-line
summary per section in "Results" at the end. The owner runs every row: no row
is marked passed here, and Claude records only what you report.

Quoted text is the app's own wording. `{repo}` stands for a repository's
display name (its directory's name), `{label}` for a key's label, `{ago}` for
a time such as "just now" or "2 minutes ago". "Ctrl" is the `secondary` key of
the keymap on Linux. A key written `Ctrl ,` or `Shift Tab` is held together.
"The design" is
`docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`; "A6 F4" is
its Part A, section A6, row F4 (the focus contract); "W-7" is a row of its Part
D (a way in); "R-39", "K-6" and "U-3" are its rulings, known limits and
unverified facts. "The Cycle document" is
`docs/Cycles/wave-04-cycle-04-settings-sheet.md`. "The ledger" is
`docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`. "The
frames" are `docs/design/screenshots/1e-settings-sharing-keys.png` (Settings on
Sharing & keys) and `1f-settings-checking-for-updates.png` (Settings on
Checking for updates), read with `docs/design/05-sheets-and-dialogs.md`
(Settings sheet) and `06-language-and-states.md` (the vocabulary). Both frames
draw the sheet on the right over the dimmed workspace, its header "Settings"
with "Applies to this computer only." under it and a close button at the right,
a menu column at the left whose active entry has a lighter background, and no
footer.

The **Supports** column names the Cycle document's Acceptance row (A1 to A15,
"Acceptance And Exit Evidence") and the design's ruling, known limit or
unverified fact the step is evidence for. A step with no automated twin says
so. Steps marked **(rare)** need a state that is hard to make by hand: try each
one, and if you cannot reach it write "skipped". Steps marked **Owner decides**
ask you to look and say what you want; they are not pass or fail. Steps marked
**Watch** name something that has never been seen on a real display, or that
the ledger says may look unexpected: write what you saw, either way.

## Minimum pass

If you cannot do every row, these cover each part of Task 9 Step 1 at least
once. Nothing below is removed from the list; the other rows add depth.

- **A15, the sheet against frames 1e and 1f in the dark theme (the owner's
  eye, no automated twin):** S-1.2, S-1.4 to S-1.7, S-2.1, S-2.2, S-2.10,
  S-3.1, S-3.2.
- **The header and its description (U-3):** S-1.3.
- **The sheet's top edge against the title bar (K-6):** S-1.5.
- **The menu, the key rows and their radio, the forms, the info box, Trusted
  servers:** S-1.6, S-2.2 to S-2.4, S-2.8, S-2.10, S-2.11.
- **Fields within the body's width (R-39):** S-6.1, S-6.2.
- **Escape, the close button and a click outside:** S-7.1 to S-7.3.
- **Focus after each kind of way in and each close:** S-9.1 to S-9.12, S-9.17.
- **The ledger's on-screen items:** S-10.1, S-10.5, S-10.9; S-2.12 and S-5.4 to
  S-5.7 (Owner decides).
- **A3, a typed edit in the details sheet is kept:** S-7.5.

Mostly covered by automated tests, or rare, so **optional** if time is short:
S-2.13, S-2.14, S-2.15, S-4.5, S-5.6, S-7.9, S-9.13 to S-9.16, S-10.8,
S-10.11, S-11.5.

## Before you start

- **Use a scratch data directory for everything.** This checklist registers
  repositories, adds and removes keys, makes a draft and stops a remote. Run
  all of it against a throwaway data directory so your real one is never
  touched, and run only from a build of this branch. Make it, check it took
  and launch with it exactly as Cycle 03a's checklist does
  (`docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md`, "Before
  you start": the desktop reads `XDG_DATA_HOME`, so the scratch data directory
  is `$SCRATCH/manyhands`; the CLI reads `MANYHANDS_DATA_DIR`).
  - **Make it** (one terminal, kept open for the whole checklist):

    ```sh
    export SCRATCH=$(mktemp -d)
    touch "$SCRATCH/.start"
    mkdir "$SCRATCH/repos"
    ```

  - **Smoke test (S-1.1), by you, on Wayland.** From the ticket worktree
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7AMAPNHHTNECXBF4CA5WPKV`,
    the debug build the Agents file names, with the scratch directory:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --locked --features desktop --bin manyhands`
    (without the scratch variable it is
    `devenv shell -- cargo run --locked --features desktop --bin manyhands`).
    Quit it (Ctrl Q) when the window is up.
  - **Launch with it** for every step after S-1.1: the release build,
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`
    It needs an active desktop display. A development build lags. Run it
    while no long test suite or build is running on the machine.
  - **Check it took.** `ls "$SCRATCH/manyhands"` lists `layout` and the
    registry's files.
  - **Your real data stays untouched,** except keys: Shared SSH keys live in
    `~/.ssh/manyhands`, outside the scratch directory (Cycle 03a says the
    same). The key rows below add and remove keys there, and **Delete key
    file…** deletes a file. Use only keys you make for this list, never your
    own. At the end
    `find ~/.local/share/manyhands -newer "$SCRATCH/.start"` prints nothing
    (adjust the path if your `XDG_DATA_HOME` is normally set).
- **Repositories** (make them with Settings › Repositories › **Create
  repository**, as Cycle 02's and 03a's checklists do; give each two or three
  tickets):
  - `shared`: a shared copy over SSH with a passphrase-protected key, as Cycle
    03a's "Before you start" makes it (S-2.11, S-9, S-10);
  - `local`: no remote (S-3.3, S-4, S-9);
  - `second`: a second repository, any state (S-4.2, S-4.3);
  - `ident`: a repository with **no Git name or email** set, for the save
    banner (S-9.11; this machine has no global Git identity, so a repository
    made with plain `git init` has none).
- **Keys.** At least three keys in Sharing & keys for S-2 and S-10: make two
  with **Make a new key** (one with a passphrase, one without) and import one
  with **Use a key I already have…** from a throwaway key you make with
  `ssh-keygen -t ed25519 -f "$SCRATCH/imported" -N ""`. Give one of them a
  label of about 60 characters (S-6.3).
- **Stalling the remote** (S-7.6, S-9.8): as Cycle 03a does, stop the host's
  listening `sshd` with `sudo kill -STOP <listener>` and release it with
  `sudo kill -CONT <listener>`.
- **A draft** for S-10.9: Edit a ticket in `local`, change its body and leave
  it unsaved, then open Drafts (the way Cycle 02's checklist does).
- **A window size.** The app opens at 1280 x 840 in the default layout: the
  Repositories panel and Documents at the left, Tickets at the right, Activity
  at the foot. The sheet is 720 px wide.
- **The dark theme.** Run in it, and write down in "Results" which theme and
  which scale factor you ran.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure. The
first group is this Cycle's design "Known Limits" (K-1 to K-7); the second is
what the frames draw and this Cycle does not build; the third is what is not
run on this machine.

- **K-1.** Key rows show no passphrase note, creation date or algorithm, and
  trusted servers no date. The library records none of them.
- **K-2.** There is no "Pause all checking" switch.
- **K-3.** `Shown::Tabs` means "Settings is closed" (no screen effect).
- **K-4.** The design's General, Your name & email and Appearance pages are
  not shown. The menu has three entries. Name and email stay per repository on
  the Repositories page.
- **K-5.** A trusted server whose key changed shows the old and new
  fingerprints side by side in Check host and Replace host key…; the frames
  do not draw this.
- **K-6.** The Kit sheet's top margin is the Kit title bar's 34 px, not the
  app's 38 px, as for the details sheet today. S-1.5 is where you look.
- **K-7.** Ctrl `,` over the details sheet keeps every edit. A relationship
  picker's typed search text is not an edit, and is dropped when the details
  sheet closes, as it is today on any close.
- **Frames, not failures.** The frames draw: General, Your name & email and
  Appearance in the menu; "Has a passphrase · created Oct 4" and "No passphrase
  · added Oct 6" under a key; the tag "From ~/.ssh" (the app says "Imported");
  "Trusted Oct 4" beside a server; a teal tint on the in-use key's row; "Pause
  all checking until I turn it back on"; and, in frame 1e, the info box's text
  and a server's fingerprint running past the sheet's right edge (the app is
  meant to keep them inside, S-6). Also `MH-VH-…` upper-case short codes, the
  Repos tab strip and the title bar's scope button.
- **Not run on this machine.** macOS and Windows are not run locally. Two
  automated focus rows run on Linux or unix only (design P23's context-menu
  rows and A6 F14's EditsNotKept row); native CI covers the other platforms
  after merge (ledger, Task 2 concerns 5 and 8). S-10.10 is the on-screen look
  at the context-menu row; S-7.9 touches the quit rows, and no row reaches
  F14's EditsNotKept.

## S-1 Launch, the header and the menu

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-1.1 | Run the smoke test above, then the "Check it took" listing. | A window opens with no error in the terminal; the title is "Manyhands"; `$SCRATCH/manyhands` lists `layout`. Ctrl Q closes it. | A1 (no automated twin) | |
| S-1.2 | In the release build, with a ticket open in `shared` or `local`, press Ctrl `,`. Look at the whole window against frame 1e. | A sheet 720 px wide slides in from the right over a dimmed window: the open ticket, the panels and the footer stay visible behind a scrim. **No footer** at the foot of the sheet. The sheet has a header, a menu column at the left and a body at the right. | A15, A1 (appearance: no automated twin) | |
| S-1.3 | **Watch (U-3).** Look at the header. The design's text: "A two-line element in `Sheet::title` lays out above the Kit's close button." | Two lines at the left: "Settings" (15 px, semibold), then "Applies to this computer only." (smaller, `secondary`). The close button (an X) is at the header's right. A pass: both lines whole and not clipped, the close button fully visible and not covered by or covering the text, the header's height not too tight for two lines. Write what you see, including any overlap, clipping, a pushed-out button or a stray gap. | A15, U-3, R-25 (no automated twin) | |
| S-1.4 | Look at the colours against frame 1e. | Dark theme: the sheet and its header the same family as the frame's, thin borders, the menu column and the body readable, nothing unreadable. Write what differs from the frame. | A15 (no automated twin for the look) | |
| S-1.5 | **Watch (K-6).** Look at the sheet's top edge against the title bar's bottom edge (the bar with File, Edit, the search box and the window buttons). Then close Settings, click the item's chip (its details sheet), and compare its top edge. | The frames start the sheet at the title bar's lower edge. The design says the Kit's top margin is 34 px and the app's title bar is 38 px, so the edge may sit about 4 px higher than the bar's bottom. Write: flush, a gap or an overlap, and by how much (by eye); whether the title bar's buttons and the search box stay visible and clickable; whether the scrim also covers the bar. Write whether the details sheet's top edge is the same (K-6 says it is today). | A15, K-6 (no automated twin) | |
| S-1.6 | Look at the menu column against frames 1e and 1f. | Three entries, in this order: **Repositories** (`folder-git-2`), **Sharing & keys** (`key-round`), **Checking for updates** (`refresh-cw`), each with a 14 px icon and its label on one line (no wrap at 214 px). No "Shown" tag. The frames also draw General, Your name & email and Appearance: not built (K-4). | A6, A15, K-4 (twin: `settings_has_three_pages`; the look has none) | |
| S-1.7 | Click each entry in turn. Look at the active entry. | The active entry has a lighter background (the frames' `border` fill) and no other entry does; each click shows that page in the body. Write what the menu's own background looks like against the body's. | A6, A15 (twin: `settings_has_three_pages`; the look has none) | |
| S-1.8 | Press the header's close button once, reopen Settings with Ctrl `,`. | The button closes the sheet (S-7 looks at where focus goes). Ctrl `,` opens it again on Repositories. | A1 | |

## S-2 Sharing & keys (frame 1e)

Open Sharing & keys with at least the three keys of "Before you start". The
frame draws two keys (one in use), the buttons, the info box and one trusted
server.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-2.1 | Open Settings and click Sharing & keys. Read the page from the top, against frame 1e. | In this order: the heading "Key used for sharing" with its intro ("Manyhands uses one key for every repository it shares through. Your private key stays on this computer."); the key list in a bordered box with rounded corners; under it **Stop using this key** (only while a key is in use); the two buttons **Make a new key** (the primary, amber) and **Use a key I already have…**; the info box; **Trusted servers**. Neither key form is shown. | A6, A15 (twin: `the_key_forms_open_from_their_buttons`; the look has none) | |
| S-2.2 | Look at one key row, then at a generated and an imported key side by side. | From the left: a radio; the label; a tag, "Made by Manyhands" on a generated key, "Imported" otherwise; an "In use" tag on the key in use; the fingerprint in monospace (or "No fingerprint recorded"); **Copy public key**; a trash icon button at the right. **Delete key file…** appears on generated keys only. Every state has words, never colour alone. The frames' notes (dates, "Has a passphrase") are not built (K-1). | A6, A15, K-1 (twin: `key_rows_send_what_todays_controls_sent`; the look has none) | |
| S-2.3 | **Owner decides (R-38).** Look at the radio: the key in use against the others, against the frame's. | The radio is drawn by the app (the Kit's radio could not take focus from a route): a filled ring on the key in use, an empty ring on the others. Say whether it looks right next to the frame's. | R-38 (no automated twin for the look) | |
| S-2.4 | Click an unselected key's radio, then the radio of the key in use. | The first makes that key the one in use: "In use" moves to it (write whether Activity shows an entry). The second does nothing (no change). | A7 (twin: `key_rows_send_what_todays_controls_sent`) | |
| S-2.5 | Hover the trash button on an imported key, then press it. | Tooltip: "Remove from Manyhands. Keeps the key file." The key leaves the list; its key file is still on disk (`ls` the imported file, `$SCRATCH/imported`). Make it again with Use a key I already have…. | A7, A14 (twin: `key_rows_send_what_todays_controls_sent`) | |
| S-2.6 | With a key in use, press **Stop using this key**. | No key is in use: no "In use" tag, and Stop using this key goes away. The key stays in the list. | A7 (twin: `key_rows_send_what_todays_controls_sent`) | |
| S-2.7 | On a generated key you made for this list, press **Delete key file…**, read the confirmation, press Cancel, then do it again and confirm. | Cancel keeps the key and its file. Confirm removes the key from the list and its file from `~/.ssh/manyhands`. Only the generated keys offer it. | A7 (twin: `key_rows_send_what_todays_controls_sent`) | |
| S-2.8 | Press **Make a new key**, then **Use a key I already have…**, then the shown form's own button. | Neither form shows at first. Make a new key shows the Generate form below the buttons; Use a key I already have… hides it and shows the Import form; pressing the shown form's button hides it. The Import form carries "Importing uses the files where they are. Removing a key from Manyhands leaves its files." Generating or importing now takes one more press than before: say if that bothers you. | A6, A7 (twin: `the_key_forms_open_from_their_buttons`) | |
| S-2.9 | Show the Generate form, type a passphrase and its confirmation, then press **Make a new key** again to hide it, and show it again. | The passphrase fields are empty. | A11 (twin: `a_typed_passphrase_lives_only_on_sharing_and_keys`) | |
| S-2.10 | Look at the info box against frame 1e. | A box with a `lock` icon and the text "If your key has a passphrase, Manyhands asks for it once each time you open the app and never stores it." It wraps and stays inside the body. | A15 (no automated twin for the look) | |
| S-2.11 | In Repositories choose `shared` (Manage), then back on Sharing & keys read **Trusted servers**. Press **Check host**. | One row per trusted server: the host and its fingerprint in monospace (the frame's "Trusted Oct 4" is not built, K-1); with none, "No server is trusted yet."; while reading, "Reading the trusted servers". Under it the note "Check host connects to the server this repository's shared copy is on and shows the key it presents. It trusts nothing yet." and the result: "This server is not trusted yet.", "The server presents the key you trusted." or "The server presents a different key from the one you trusted." with **Approve host…** or **Replace host key…**, each opening a confirmation; a trusted key reads "Trusted key". | A6, A15 (twin: hosts tests; the look has none) | |
| S-2.12 | **Owner decides.** Look between the intro and the key list. | A line "Selected key: {label}" (or "No key is selected.") between the intro and the list. It repeats the "In use" tag and frame 1e has no such line. The ledger kept it only because removing it needs test removals no named edit allows (Ruling 49). Say keep or remove. | A15, ledger Ruling 49 (no automated twin) | |
| S-2.13 | Choose `local` (no shared copy) on Repositories, then look at Trusted servers. | Check host is unavailable with "Check host is unavailable: this repository's shared copy is not reached over SSH. Choose one under Repositories › Sharing." With no repository managed: "Manage a repository on the Repositories page to check the server it shares through." | A10 (twin: hosts tests) | |
| S-2.14 | **(rare) Retry of a protected key generation.** Make a passphrase-protected generation retry (a generation that failed, with Retry in Activity), so Settings opens on the Generate form asking again. Then press **Make a new key**, and look at **Use a key I already have…**. | The Generate form is shown while the page is retrying, with its passphrase field focused. **Make a new key** stays pressable and **does nothing** (its own button cannot hide the form). **Use a key I already have…** is unavailable with the reason "Finish or cancel the retry first." Write which of the two shows that line. | A6, ledger Ruling 34 (owner's eye; twin: `the_key_forms_open_from_their_buttons`) | |
| S-2.15 | Press **Make a new key** and show the Generate form, close Settings, open it again, and go to Sharing & keys. | The form is still shown (form visibility is kept across a close). "Neither form at first" meant the first time the page is made. Say if you would rather it started closed each time. | A6, ledger Ruling 35 (owner's eye; no automated twin) | |

## S-3 Checking for updates (frame 1f)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-3.1 | After a check has finished, click the footer's "Checked for updates {ago}". Read the page against frame 1f. | Settings opens on Checking for updates (the third entry active). Heading "Checking for updates" and the intro "While Manyhands is open it checks each repository's shared copy and brings in other people's changes when it's safe. It never shares your changes, and never touches anything you're still editing." One row per repository. | A1, A15 (no automated twin for the look) | |
| S-3.2 | Look at a row with a shared copy, a row with none and a Can't reach one. | A shared copy: the name, a status line ("Checked {ago} · up to date"), an interval select ("Every 5 minutes" and so on), **Check now**, a switch. No shared copy: "No shared copy, nothing to check" and **Set up sharing…**. Can't reach: the status line in red and **Retry now**. The frame draws a global "Pause all checking until I turn it back on" switch and its status lines cut off by the select: the switch is not built (K-2). Write whether any status line is cut off in the app. | A15, K-2 (no automated twin for the look) | |
| S-3.3 | On `local`'s row press **Set up sharing…**. Then press Esc. | The menu moves to Repositories and its Sharing section for `local`, with focus on the Add a remote form's Name field; the sheet stays open. Esc closes the sheet and focus goes to where Settings was opened from, not to a control inside. | A5 (twin: `each_route_lands_on_its_page_and_control`, W-31) | |
| S-3.4 | Choose an interval for a repository by keyboard (Tab to the select, Enter, arrow, Enter), then switch checking off and on for it. | The setting changes and shows. The select's popup is covered in S-10.10. | A7 | |

## S-4 Repositories

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-4.1 | Choose `local` in the Repositories panel, press Ctrl `,`. Read the page. | In this order: Repositories (each registration with **Manage**; for the chosen one its Git name and email form and **Remove repository…**), then **Sharing**, then **Add an existing repository** and **Create a repository** as before. | A6 (twin: `settings_has_three_pages`, `one_repository_is_chosen`) | |
| S-4.2 | **Owner decides (R-14).** Open Settings with `local` chosen in the panel: is its identity form and Remove already showing? Then choose `second` in the panel, close and reopen. | A plain open shows the panel's chosen repository's form and Remove at once, and a reopen follows a choice made in the panel since the last open. A reopen with no change keeps the repository you managed. Say whether that is what you want. | A6, A13, R-14 (twin: `one_repository_is_chosen`) | |
| S-4.3 | **Owner decides (R-15).** Press **Manage** on `second`, then read **Sharing**. | Sharing now shows `second`'s remotes. There is no separate list to choose from in Sharing: Manage is the one way to choose. Say if that is clear. | A6, R-15 (twin: `one_repository_is_chosen`) | |
| S-4.4 | Read Sharing for `shared` (and a repository with a non-SSH remote if you have one). | A heading "Sharing", a line "Shared copy: {name}"; the chosen remote reads "The shared copy"; a remote over SSH that is not chosen "Can be the shared copy" with **Use as the shared copy**; a non-SSH one "Can't be the shared copy: sharing needs a remote reached over SSH"; **Keep changes on this computer**; **Remove remote…**; with none chosen "No shared copy is chosen, so changes stay on this computer." With nothing managed: "Manage a repository above to see how it shares." | A10 (twin: `settings_copy_uses_the_vocabulary`) | |
| S-4.5 | Press **Remove repository…** for a scratch repository (`second`), read the confirmation, press Cancel. | A confirmation (the label ends in "…"). Cancel keeps it. Do not confirm unless you want `second` gone; confirming does not touch its files. | A7 (focus: S-10.8) | |

## S-5 Copy

Read the words, not the layout. The design's vocabulary is `06`: no
"publication", "publish", "pin", "SSH key", "Remotes page", "Hosts page",
"sidebar" or "Back to tabs" in Settings.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-5.1 | Read the menu and the header. | "Repositories", "Sharing & keys", "Checking for updates"; under "Settings": "Applies to this computer only." No "Back to tabs" button anywhere. | A10 (twin: `settings_copy_uses_the_vocabulary`) | |
| S-5.2 | Read Sharing & keys: heading, empty list, tags, buttons and tooltips. | "Key used for sharing"; with no keys "No key is set up for sharing yet."; "Made by Manyhands" and "Imported"; "In use"; **Stop using this key**; trash tooltip "Remove from Manyhands. Keeps the key file."; **Delete key file…**; **Make a new key**; **Use a key I already have…**. Every action that opens a confirmation or a form ends in "…". | A10 (twin: `settings_copy_uses_the_vocabulary`) | |
| S-5.3 | Read Trusted servers and Repositories › Sharing (S-2.11, S-4.4). | "Trusted servers", "Trusted key", "Approve host…", "Replace host key…"; "The shared copy", "Use as the shared copy", "Keep changes on this computer", "Remove remote…", "Remove repository…". | A10 (twin: `settings_copy_uses_the_vocabulary`) | |
| S-5.4 | Walk every page once more, then Activity after you use a key and a shared copy, looking for the old terms. | None of "publication", "publish", "pin", "pinned", "SSH key", "Remotes page", "Hosts page", "sidebar". The ledger says a few strings were left alone (a confirmation's "Last published", an "Unpublished" label, the old `keys` selection line): write where you meet any of them. | A10 (twin: `settings_copy_uses_the_vocabulary`; the walk has none) | |
| S-5.5 | **Owner decides.** From Activity, press **Open remote settings…** on a row that has it. | It lands in Repositories › Sharing (S-9.7). The label still says "remote settings": a replaced term is not involved, so it was left. Say if you want "Open sharing settings…". | A10, ledger Task 7 Minor 4 (no automated twin) | |
| S-5.6 | **(rare) The key-store wording.** Make the key store unavailable if you can (for example a key page opened with the store directory unreadable), then look at Settings and at Activity. | Settings and the dialogs say "Manyhands could not find your key store."; Activity's own row still says "your SSH key store." Write both. Say whether to align Activity. | A10, ledger Ruling 50 (no automated twin) | |
| S-5.7 | **Activity row titles.** Use **Use as the shared copy**, **Approve host…** and a key choice, then read Activity. | The rows keep the old vocabulary, for example "Select remote" and "Approve host": unchanged by this Cycle, not a failure. Write the titles you read. | A10, ledger Task 6 and Task 7 (Ruling 10; no automated twin) | |

## S-6 Fields within the body's width (R-39)

The body is the sheet's 720 px less the 214 px menu and its padding, which is
less than a field's 480 px maximum, so fields fill the body's width and no
more. Frame 1e's info box and server fingerprint run past the edge in the
mockup; the app is not meant to.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-6.1 | At the default 1280 x 840 window, on Sharing & keys, show each form (Generate, then Import) and look at every field and button. | No input, button, tag or text reaches or crosses the body's right edge or the sheet's edge: the key row (label, both tags, fingerprint, **Copy public key**, trash), the Generate form's fields, the Import form's path fields and buttons, the info box, a server's fingerprint and **Check host** with its result. No horizontal scrollbar; the vertical one only when the page is taller than the sheet. | A15, R-39 (no automated twin: layout is not asserted) | |
| S-6.2 | On Repositories, look at every field and button: a registration row and its Manage, the identity form's Name and Email, the Add a remote form's Name and URL, the Add folder field, the Create form. Then on Checking for updates look at a row's select, button and switch. | Each fills the body's width up to 480 px and stops inside it; nothing is clipped on its right; no horizontal scroll. | A15, R-39 | |
| S-6.3 | With the 60-character key label and a repository with a long name, look at their rows on each page. | The long text wraps or is shortened with an ellipsis; the tags, **Copy public key**, trash and buttons stay inside the body. | A15, R-39 | |
| S-6.4 | Make the window narrower in steps (down to the smallest the app allows) and repeat S-6.1 on Sharing & keys and S-6.2 on Repositories at each step. Write the width at which anything breaks. | The header, the menu and the body stay usable; fields shrink with the body, not past it; the close button stays reachable. | A15, R-39 (no automated twin) | |

## S-7 Closing: Escape, the close button and a click outside

Do each of S-7.1 to S-7.3 from the same starting state: a ticket open,
focus on one control of your choice (write which), Settings opened with Ctrl
`,`.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-7.1 | Press Esc. | The sheet closes, the scrim goes, the ticket is as you left it, and focus is back on the control that opened Settings, with its ring (write what holds it). No dialog opens. | A1, A4 (twin: `settings_closes_every_way`, `focus_returns_to_what_opened_settings`; the ring is appearance) | |
| S-7.2 | Open it again and press the header's close button. | The same: the sheet closes and focus returns to the same control. | A1, A4 (twin: `settings_closes_every_way`) | |
| S-7.3 | Open it again and click outside the sheet, on the dimmed item area. Then once more on a dimmed panel row. | The sheet closes; focus returns to the same control. The design says the scrim takes the click: write whether the click also acted on what was under it (a ticket opened, a row chosen). | A1, A4 (twin: `settings_closes_every_way`; a real click has none) | |
| S-7.4 | With a ticket open and scrolled halfway down, press Ctrl `,`, press PageDown on a menu entry, then Esc. | The same tab is shown at the same scroll offset, and the footer, panels and tabs are as they were. The PageDown scrolled the sheet, not the ticket. | A2, A13 (twin: `settings_leaves_the_item_as_it_was`, `settings_scrolls_on_its_own`) | |
| S-7.5 | Open the item's details sheet (its chip), type a change in a field and do not save. Press Ctrl `,`. Then Esc. Then reopen the details sheet. | Settings replaces the details sheet (one sheet, no second one); focus is inside Settings. Esc closes it and focus goes to the chip. The reopened details sheet still holds what you typed. | A3, K-7 (twin: `settings_replaces_the_details_sheet_and_keeps_its_edits`) | |
| S-7.6 | With `shared` managed, stall the remote ("Stalling the remote" above), press **Check host** on Sharing & keys, then Esc at once. Release the remote. Reopen Settings. | Escape is not blocked by the running request: the sheet closes. Nothing is cancelled: Activity reports the result when it lands. No confirmation opens and focus does not move by itself. Reopening shows the result. | A1, A4 (twin: `late_work_moves_no_focus`) | |
| S-7.7 | Type a passphrase in the Generate form, then close Settings by each of Esc, the close button and a click outside (one at a time), reopening each time. Then type one and click another menu entry and back. | Each time the passphrase fields are empty. A passphrase typed on Sharing & keys survives only while Sharing & keys is the page. | A11 (twin: `a_typed_passphrase_lives_only_on_sharing_and_keys`, `settings_closes_every_way`) | |
| S-7.8 | On Checking for updates, find a row with a **Show in Activity** link or button (a repository that cannot be reached, or one waiting on a key) and press it, with Activity hidden. | Settings closes, Activity is shown, and its list has focus with its ring. | A12 (twin: `show_in_activity_closes_settings_and_focuses_activity`) | |
| S-7.9 | **(rare)** Press Ctrl Q with Settings open, once with Activity displayed and once with it hidden. | The sheet closes first, then the app asks or quits as it does today: with Activity displayed focus goes to Quit now; hidden, to where Settings was opened from. | A1, A4 (twin: `settings_closes_every_way`) | |

## S-8 Behind the sheet

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-8.1 | With Settings open on Sharing & keys press Ctrl 1, Ctrl 4, Ctrl N, Ctrl Tab, your close-tab key and Ctrl `,`. Try the footer's toggles with a click. | Nothing acts behind the sheet: no panel takes focus, no dialog opens, no tab changes, Ctrl `,` leaves Settings as it was; the footer cannot be pressed through the scrim. | A9 (twin: `nothing_acts_behind_settings`) | |
| S-8.2 | Tab through a long page (Sharing & keys with the Generate form shown) to a control below the sheet's fold. Press PageDown, PageUp, End and Home with focus on a menu entry, then on a page control. | The control is scrolled into view inside the sheet. The page keys scroll the sheet's body, not the item area, from an entry or a control (inside a text input, Home and End move the cursor). | A13 (twin: `settings_scrolls_on_its_own`) | |
| S-8.3 | **Owner decides (R-31).** Open Drafts, then Ctrl `,`, then Esc. | Drafts stays shown under the sheet, and closing Settings returns to Drafts, not to the tabs. Say whether that is what you want. | A1, R-31 (twin: `settings_is_a_sheet_the_workspace_keeps`) | |
| S-8.4 | With Settings open, let a background check or a share finish (or press **Check now** on Checking for updates). Watch the item area and Activity while the sheet is open. | Nothing opens or takes focus behind the sheet. Late results show in Activity, and on the page the next time it opens. | A9 (twin: `focus_stays_inside_settings`, `late_work_moves_no_focus`) | |

## S-9 Focus after each way in and each close

Each row: do the **way in**, read the **page** and the **control with the
ring** (a route that names a field focuses it; one that does not focuses the
page's menu entry), then press Esc and read where focus **returns**. "Entry"
means the page's menu entry. Part D's lines are in brackets. Write what you
see where a row says so. If you cannot reach a state write "skipped".

| # | Do (way in) | Expect on arrival, then after Esc | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-9.1 | **Footer Settings** [W-2]: Tab to the footer's gear and press Enter. | Repositories page, ring on its **entry**. Esc: focus back on the footer's Settings. | A1, A4 (twin: `focus_returns_to_what_opened_settings`) | |
| S-9.2 | **Ctrl `,` from a list** [W-1]: focus a ticket row, press Ctrl `,`. | Repositories page, ring on its entry. Esc: focus back on the ticket row. | A1, A4 (twin: `focus_returns_to_what_opened_settings`) | |
| S-9.3 | **Footer "Checked for updates"** [W-3]: Tab to it, Enter. | Checking for updates page, ring on its entry. Esc: back on that footer item (if it hides while Settings is open, on the next control to its right). | A1, A4 (twin: `each_route_lands_on_its_page_and_control`, `focus_returns_to_what_opened_settings`) | |
| S-9.4 | **Repository menu Settings…** [W-7]: right-click a repository row, choose Settings…. | Repositories page with **that** repository managed, ring on its entry. Esc: focus back where the menu was opened (the list or the row). | A5 (twin: `each_route_lands_on_its_page_and_control`) | |
| S-9.5 | **Repository menu Share main…** on `local` [W-10]. | Repositories page, Sharing section, ring on the Add a remote form's **Name** field. Esc: back where the menu was opened. | A5 (twin: `each_route_lands_on_its_page_and_control`) | |
| S-9.6 | **Activity Choose a key…** [W-23]: with a share that needs a key, press the button in the Activity row (also try its row menu and Enter). | Sharing & keys, ring on the **radio of the key in use**, else the first key's, else **Make a new key**. Esc: focus on the Activity list with the same row highlighted. | A5 (twin: `routes_by_keyboard`, `focus_returns_to_what_opened_settings`) | |
| S-9.7 | **Activity Open remote settings…** [W-24]. | Repositories › Sharing for that row's repository, ring on **Name**. Esc: the Activity list, the row highlighted. | A5 (twin: `routes_by_keyboard`) | |
| S-9.8 | **Activity Approve host…** [W-25] with the remote stalled or a host not trusted yet (Check host shows it): press the row's button. | Sharing & keys › Trusted servers for **that row's repository**, ring on **Check host**. Esc: the Activity list. | A5 (twin: `each_route_lands_on_its_page_and_control`, R-23) | |
| S-9.9 | **An item header Share…** [W-11]: in `local` open a ticket, press **Share…** (the item has no shared copy). | Repositories › Sharing for `local`, ring on **Name**. Esc: focus on the item header's Share…. | A5 (twin: `routes_by_keyboard`) | |
| S-9.10 | **The lifecycle dialog's Open Sharing & keys** [W-21]: make **Close ticket…** fail for a key reason (a refused or locked key) so the dialog offers it, then press it. | The dialog closes; Sharing & keys, ring on the key in use's radio (else the first, else Make a new key). Esc: focus on the control that opened the dialog (**Close ticket…**), the dialog's own origin. | A5 (twin: `routes_by_keyboard`) | |
| S-9.11 | **The save banner's Open Settings** [W-30]: in `ident` (no Git name) edit a ticket and Save so the banner "initials required" shows, then press its **Open Settings**. | Repositories, ring on the identity form's **name** field. Set a name and email; Esc. The banner has gone: focus goes to the **content region** (a control of the item area), not to a button that no longer exists. If you do not set the identity first, focus goes to the banner's button. | A5 (twin: `routes_by_keyboard`, `focus_returns_to_what_opened_settings`) | |
| S-9.12 | **Add… from the empty panel and from the toolbar** [W-5, W-6]: with no repositories, press **Add or create a repository…**; later press the Repositories toolbar's `plus`. | Repositories, ring on the **Add folder** field. Esc: back on the button that opened it, or, if that has gone (the empty panel's button once a repository exists), the panel's region. | A5, A8 (twin: `each_route_lands_on_its_page_and_control`) | |
| S-9.13 | **(optional) A popover row** [W-23 and others]: open the footer's "N need attention" popover and press a row's button that opens Settings. | The popover closes, Settings opens on the row's page and field. Esc: focus on the footer item, or if it has hidden, the control to its right. | A4 (twin: `focus_returns_to_what_opened_settings`) | |
| S-9.14 | **(optional) Ctrl `,` over a menu:** open a repository menu (Shift F10 on a row) or a dock menu, then press Ctrl `,`. Then Esc. | The menu closes and Settings opens. Esc: focus on the control that menu returns to (its row or the list). | A4 (twin: `focus_returns_to_what_opened_settings`) | |
| S-9.15 | **A route replacing the item's sheet:** open a ticket's details sheet (chip), then press Ctrl `,`. Then Esc. | Settings replaces the details sheet; Esc returns focus to the **chip**. (Same as S-7.5; write the focus.) | A3, A4 (twin: `settings_replaces_the_details_sheet_and_keeps_its_edits`) | |
| S-9.16 | With Settings open press Ctrl `,` again, and with a dialog open press it. | Nothing happens (Settings stays as it was; no second sheet, nothing behind a dialog). | A9 (twin: `settings_does_not_open_behind_a_dialog_or_while_quitting`) | |
| S-9.17 | **Close each of three ways from a route with a field:** from S-9.5, close once by Esc, once by the close button, once by a click outside. | Each returns focus to the same control (the one that opened the menu). Write where each puts it. | A4 (twin: `focus_returns_to_what_opened_settings`; the button and scrim share the Kit's close path) | |

## S-10 Focus on screen: rings and the focus moves the ledger reports

These rows look at things no test sees: whether a ring is drawn and whether a
move looks wrong. Write what you saw, either way.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-10.1 | **Watch (the Kit buttons' key-ID).** Have three keys (A, B, C, in that order). Open Sharing & keys, Tab to B's trash ("Remove from Manyhands") and press Enter, **by keyboard only**. Look for a focus ring. | B leaves; focus goes to **C's radio**, even if the list's order changes, with a **visible ring** on that control. The ledger warns that the Kit buttons' IDs are keyed by key ID but their wrapper's ID is the row's index, so when a key changes row, focus may fall to a wrapper with no ring until you press Tab. Write exactly what has the ring (if anything) and whether a Tab was needed. If it shows no ring, the fix is to key the wrappers by key ID. | A13, ledger Ruling 37 (twin: `removing_a_key_returns_focus_by_key`, which passes through that fallback; the ring has none) | |
| S-10.2 | Remove the last key of three by keyboard, then the only key left. | After the last of three: focus on the **previous** key's radio, with a ring. After the only key: focus on **Make a new key**, with a ring. | A13 (twin: `removing_a_key_returns_focus_by_key`) | |
| S-10.3 | On a generated key press **Delete key file…** by keyboard, Cancel with Esc, then again and confirm with Enter. | Cancel: focus back on that key's Delete key file… button, with a ring. Confirm: the key goes and focus is on the next key's radio, else the previous one's, else **Make a new key**. | A4, A13 (twin: `removing_a_key_returns_focus_by_key`, F15) | |
| S-10.4 | **The Kit button focus ring on the menu entries.** Tab (and Shift Tab) to each of the three menu entries. | Each shows a visible focus ring (the Kit's, in the sheet's colours) that fits the entry and is not clipped by the menu column. Write what the ring looks like and any entry where none shows. | A6, ledger Task 5 concern 3 (no automated twin) | |
| S-10.5 | **The icons and the menu's background.** Look at the trash icon on a key row and the `key-round` icon in the menu, then at the menu column's background against the body, against frames 1e and 1f. | `trash` reads as a trash can (not the Kit's backspace-like glyph); `key-round` a key; both at 14 px, grey, like their neighbours. The menu's background is the frames'. Write what differs. | A6, A15, ledger Task 4 Ruling 39 and Task 5 (no automated twin) | |
| S-10.6 | Tab through every control on Sharing & keys: the radios, **Copy public key**, the trash, **Delete key file…**, **Stop using this key**, **Make a new key**, **Use a key I already have…**, a shown form's fields and buttons, **Check host**. | Each stop shows a ring. Write any that shows none. | A5 (twin: `sharing_and_keys_by_keyboard`; the ring is appearance) | |
| S-10.7 | Do the same on Repositories (Manage, the identity fields, Remove repository…, the remotes' buttons, Add folder) and Checking for updates (a select, Check now, a switch, Set up sharing…). | Each stop shows a ring. Write any that shows none. | A5 | |
| S-10.8 | Press **Remove repository…** (for `second`), then Cancel with Esc. Open it again, and Cancel with the button. Confirm once if you want `second` gone. | Esc closes **only the confirmation**, not the sheet (the dialog takes it first); focus is back on **Remove repository…**, inside the sheet, with a ring. After a confirmed removal the button is gone: focus goes to the page's menu entry (the sheet's entry), not to something behind it. | A4 (twin: `focus_stays_inside_settings`, F15) | |
| S-10.9 | **Watch (F15, then F16).** Open Drafts with a draft (a row with **Discard…**), press **Discard…**, confirm with **Enter**, and press Ctrl `,` within a quarter of a second of the confirmation closing. Do it again, a second apart. | The second time: Settings opens with its ring on the **Repositories entry**. The first (fast) time: the ledger says focus may go to the dialog's origin and then jump to the page's entry when the Kit's deferred restore fires, 0.25 s after the dialog closes. Look for: a ring flashing on the Drafts row behind the sheet, a ring on nothing, a second Tab needed, or a flicker. Say whether it looks broken. | A4, ledger Task 2 item 7 (twin: `focus_stays_inside_settings`, F15 and F16; the look has none) | |
| S-10.10 | **Watch (F19).** On Checking for updates open an interval select by keyboard (Enter), then Esc. Then right-click inside a text field (the identity Name) and open its context menu, then Esc. | The select's list holds focus while open; after it closes focus is back on the select. The context menu keeps focus while open, then focus is back in the field. Neither lets focus drop behind the sheet. (The context-menu row runs on Linux only.) | A4 (twin: `focus_stays_inside_settings`, F19) | |
| S-10.11 | **(rare) Late landing (F17).** On Repositories type a folder path in Add folder and press **Add** with a slow check (a large folder or a stalled network location), press Esc at once and watch. | No confirmation opens afterward and no focus moves; reopening Settings shows no pending Add. | A4 (twin: `late_work_moves_no_focus`) | |

## S-11 By keyboard alone

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-11.1 | With Settings open on each page, press Tab until you pass the last control, then Shift Tab until you pass the first. | Tab from the last control reaches the first; Shift Tab from the first reaches the last. Focus never reaches a panel, the footer or the title bar behind the sheet. | A4, A9 (twin: `focus_stays_inside_settings`) | |
| S-11.2 | Focus each of the three menu entries and press Enter, then Space; then click one. | Each opens its page (an entry opens its first page: Repositories, Sharing & keys, Checking for updates). The entry in use stays marked. | A6 (twin: `settings_has_three_pages`) | |
| S-11.3 | Do Sharing & keys by keys only: choose a key (Space or Enter on its radio), Stop using this key, Make a new key and generate, Use a key I already have… and import, remove a key, Delete key file… through its confirmation, Copy public key. | Every action is reachable and works with Tab, Enter, Space and Esc. | A5, A7 (twin: `sharing_and_keys_by_keyboard`) | |
| S-11.4 | **(optional) Orca.** With Orca on, move to a key's radio, the trash and a menu entry. | The radio is read as a radio button named "Use {label} for sharing", checked for the key in use; the trash as "Remove from Manyhands"; the active menu entry as selected. Write what you hear. | A6, R-38 (no automated twin for the reading) | |
| S-11.5 | **(optional) Platform note.** Read "Not run on this machine" in the known limits. | Nothing to do: macOS and Windows are not run here, and the Linux or unix-only focus rows are for native CI. | none | |

## Results

Write the build (commit), display server, theme, scale factor, and one line
per section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, theme, scale factor | |
| S-1 | Launch, the header and the menu (A1, A6, A15; K-6, U-3) | |
| S-2 | Sharing & keys (A6, A7, A10, A11, A14, A15) | |
| S-3 | Checking for updates (A1, A5, A15) | |
| S-4 | Repositories (A6, A7, A13) | |
| S-5 | Copy (A10) | |
| S-6 | Fields within the body's width (A15; R-39) | |
| S-7 | Closing (A1, A2, A3, A4, A11, A12) | |
| S-8 | Behind the sheet (A1, A9, A13) | |
| S-9 | Focus after each way in and each close (A1, A4, A5, A8, A9) | |
| S-10 | Focus on screen (A4, A13; ledger items) | |
| S-11 | By keyboard alone (A4 to A7, A9) | |
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
| Display server, theme, scale factor | |
| Result (all rows run, rows skipped, rows failed) | |
| Notes | |
