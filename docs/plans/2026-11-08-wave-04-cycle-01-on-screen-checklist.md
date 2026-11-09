# Wave 04 Cycle 01 On-Screen Checklist

This list is for checking the Cycle 01 workspace shell (title bar, dock
area and panels, footer, dock menu, dock keys, layout file, notices and the
waiting quit) by hand on Linux Wayland, before the Cycle is called
review-ready. It is the native evidence the design asks for (design,
"Testing", "On-screen checklist"), for the Acceptance rows A1 to A20 (Cycle
document, "Acceptance And Exit Evidence"), and it carries the items the
execution ledger left for the screen: the real mouse drags, a real active
window, the context-menu key (U-3), a screen reader and the Wayland title
bar (A16). Work through sections W-1 to W-12 in order. Write each step's
result in its row's last column, and a one-line summary per section in
"Results" at the end. Claude records only what you report.

Quoted text is the app's own wording. `{name}` stands for a repository's
name (its directory's name). "Ctrl" is the `secondary` key of the keymap on
Linux. A key written `Ctrl Alt B` is held together; `Shift F10` likewise.

The **Supports** column names the Acceptance row (A1 to A20) the step is
evidence for. A step with no automated twin says so. Steps marked **(rare)**
need a state that is hard to make by hand: try each one, and if you cannot
reach it write "skipped". Each is also covered by an automated test.

## Minimum pass

If you cannot do every row, these cover each Acceptance row at least once.
Nothing below is removed from the list; the other rows add depth.

- **A16, Wayland title bar (native evidence only):** W-2.1 to W-2.5, W-2.7,
  W-2.9.
- **A1, default look:** W-1.1 to W-1.3.
- **A10 and U-3, keys and the context-menu key:** W-4.1, W-4.5, W-4.6 and
  W-5.1 to W-5.3.
- **A2 and A20, moving and dragging panels:** W-6.1, W-6.3, W-6.4, W-6.7,
  W-6.8.
- **A3, resizing:** W-6.9, W-6.10, W-6.13, W-6.16, and W-8.15 or W-8.16.
- **A4, restart:** W-8.1, W-8.3, W-8.9.
- **A5, a bad layout file and the notices:** W-8.6 to W-8.9, W-8.12.
- **A7, a waiting quit:** W-9.1 to W-9.3.
- **A9, Needs attention:** W-3.8, W-3.9.
- **A13, a screen reader:** W-12.1, W-12.2.
- **Wave Integration Gate, layouts 1a and 1c:** W-7.2, W-7.3.
- **A2 and A18, the arranging journey on screen:** W-11.1 to W-11.5.

Mostly covered by automated tests, or rare, so **optional** if time is short:
W-2.10, W-2.11, W-3.3, W-3.4, W-3.6, W-3.10, W-3.13, W-4.7, W-4.8, W-5.5,
W-5.8, W-5.9, W-6.14, W-6.15, W-7.5, W-8.13, W-8.17, W-8.18, W-9.4, W-9.5,
W-10.5 and W-12.3 to W-12.5.

## Before you start

- **Use a scratch data directory for everything.** This checklist changes
  the layout many times, breaks the layout file on purpose, and creates
  repositories, drafts and unwritable folders. Run all of it against a
  throwaway data directory so your real one is never touched.
  - **What the app honours.** The desktop opens its data through
    `RepositoryService::open_default` (`src/desktop/mod.rs:64`), which uses
    `directories::ProjectDirs::from("com", "manyhands", "Manyhands")`
    (`src/repository.rs:7440-7445`). On Linux that is `$XDG_DATA_HOME/manyhands`
    when `XDG_DATA_HOME` is an absolute path, else `~/.local/share/manyhands`
    (`directories` 5.0.1, `src/lin.rs:69` and `:94-96`). **The desktop does
    not read `MANYHANDS_DATA_DIR`; only `manyhands-cli` does**
    (`src/bin/manyhands-cli.rs:90`). So the desktop is pointed at the scratch
    directory with `XDG_DATA_HOME`.
  - **Make it** (one terminal, kept open for the whole checklist):

    ```sh
    export SCRATCH=$(mktemp -d)
    touch "$SCRATCH/.start"
    mkdir "$SCRATCH/repos"
    ```

    The scratch data directory is then `$SCRATCH/manyhands` (created by the
    first launch); the layout file is `$SCRATCH/manyhands/layout/layout.json`,
    beside `drafts/` and `resolutions/`. Whenever this list says "the data
    directory", it means `$SCRATCH/manyhands`.
  - **Launch with it** (from the ticket worktree,
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M6V8RK8B0SBCH0YZXFVSR9E3`).
    The release build, for every step except W-1.1:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`
    W-1.1 is AGENTS.md's smoke run, also against the scratch directory:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --locked --features desktop --bin manyhands`
    Both need an active desktop display. A development build lags, as D3's
    checklist found. Run it while no long test suite or build is running on
    the machine.
  - **Check it took (W-1.0).** After the first launch, `ls "$SCRATCH/manyhands"`
    lists `layout` (and `drafts`, `resolutions` and the registry's files). If it does not, the variable
    did not reach the app: quit and fix that before going on.
  - **Your real data stays untouched.** At the end,
    `find ~/.local/share/manyhands -newer "$SCRATCH/.start"` prints nothing
    (adjust the path if your `XDG_DATA_HOME` is normally set). The only real
    place a step could touch is `~/.ssh/manyhands`, where Settings › Shared
    SSH keys keeps keys: **do not use Shared SSH keys or Remotes** in this
    checklist, except the optional stalled share in W-9.1.
  - **Smoke against the real directory (only if you want).** Start the
    release build once **without** `XDG_DATA_HOME`, look at the window, and
    press Ctrl Q at once. Change nothing: a launch with no layout change
    writes no layout file, and a good existing file is only read.
  - **Where the CLI looks.** To make tickets and documents in the scratch
    repositories, run the CLI against the same scratch directory:
    `devenv shell -- env MANYHANDS_DATA_DIR="$SCRATCH/manyhands" cargo run --locked --bin manyhands-cli -- …`
    The layout is never written into a repository (A14).
- **Two scratch repositories**, both under `$SCRATCH/repos`. Settings
  (Ctrl ,) › Repositories › **Create repository**, as in D5: a normal one
  named `scratch` (folder `$SCRATCH/repos/scratch`), with a few tickets and
  documents (make them with `manyhands-cli` or plain `git`, as D5
  describes); and a second whose directory has a very long name, such as
  `$SCRATCH/repos/a-repository-with-a-name-that-keeps-going-and-going-and-going-until-it-is-too-long`,
  for W-2.8.
- **A window size.** The app opens at 1280 x 840. Steps that need a narrower
  window say so; resize it by dragging an edge.
- Some steps need something to need attention, a draft, or a job that waits
  (W-3, W-9, W-10). Their rows say how to make it. A stalled share is made
  as in D5: `kill -STOP` the `sshd` process serving your connection, and
  `kill -CONT` to let it go.

### The default layout

The app starts in layout 1b, without Projects and Teams:

- **Left dock** (290 px): Repositories over Documents (about 0.3 to 0.7 of
  its height).
- **Right dock** (440 px): Tickets.
- **Bottom dock** (150 px): Activity, in the centre column under the item
  area.
- **Title bar** 38 px at the top, **footer** 28 px at the foot.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure.

- **K-1.** Docks and groups resize by mouse only; the Kit's handles take no
  keyboard focus.
- **K-2.** A refused drop (on the item area) reloads the dock area, so focus
  returns to the item area. The drop indicator shows over the item area
  before the refusal.
- **K-3.** Only a tabbed group's displayed tab is an F6 stop.
- **K-4.** A compositor that refuses client decorations draws its own title
  bar above the app's.
- **K-5.** That the context-menu key arrives as `menu` under Wayland is
  inferred; Shift F10 works either way (W-4).
- **K-6.** Open tabs, the focused pane and the scope are not saved.
- **K-8.** Tests read accessible names and states from the Kit's snapshots;
  what a screen reader announces is checked only here (W-12).
- **K-9.** A window resize does not save; the saved pixels are those of the
  last layout change.
- **K-10.** D6's attention states (key locked, host approval, recovery hold)
  are not counted in Needs attention until D6 and this Cycle are both on
  main.
- **K-11.** A dock's only panel cannot be dragged out of it; its dock menu
  moves it.
- **K-12.** Tabbed groups are made by dragging only; the dock menu makes a
  new group.
- **K-13.** Nothing scrolls a focused control in Activity into view; at the
  default 150 px some controls need scrolling.
- **K-14.** The quit's layout write runs on the UI thread and can wait for
  a write already in flight; on a hung file system the quit waits with it.
- **K-15.** A closed bottom dock keeps a thin strip (about 29 px) under the
  item area.
- **K-17.** A bottom dock split top and bottom by a drag shows only its top
  group's tabs in the closed strip.
- **K-18.** With Activity hidden, a quit that has to wait shows nothing
  until Activity is opened.
- **Proposed K-19 (not yet in the design).** Activity is as wide as the item
  area (about 550 px at the default window). Three rows in the operations
  area are single unwrapped lines, so at narrow widths a control after the
  text can sit outside the visible Activity (W-10).
- **Proposed (ledger, Task 9).** The operating system ending the app without
  a quit (`kill -9`, a power cut) loses a layout change made in the last
  half second.

## W-1 Smoke and the default look

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-1.1 | Run AGENTS.md's smoke command (the development build) from the worktree. Quit with Ctrl Q. | A window opens with no error in the terminal, and Ctrl Q closes it with no error. | A1, A8 | |
| W-1.2 | Check the scratch directory took (see "Check it took"), then quit and start the release build again over the **same scratch directory**, which has no `layout/layout.json` yet. Look at the window without touching it. | Title bar with the app icon and the title "Manyhands" (no repository is selected). Left: **Repositories** over **Documents**. Right: **Tickets**. Bottom, under the item area: **Activity**. Footer along the foot. Each panel has a header with its icon, its title and (once its list has loaded) a count beside the title; Activity has no count. Keyboard focus starts on Repositories (R-9): in an active window its ring shows, and no other region has one. | A1, A6 | |
| W-1.3 | Select `scratch` (click it in Repositories). | The title reads "Manyhands — scratch" (an em dash with spaces). Tickets and Documents list the repository; their counts are the number of rows listed (the ticket count follows the filters). | A6, A1 | |
| W-1.4 | Look at the proportions by eye: left dock about 290 px wide, right about 440 px, Activity about 150 px tall; Repositories about a third of the left dock's height; title bar 38 px; footer 28 px. | They look as described, within a few pixels. No panel is a sliver. The footer is thinner than the title bar. | A1 | |
| W-1.5 | Look at the colours. | The chrome (title bar, footer, panel headers) is the dark chrome colour, the item area the editor colour, with thin borders between panels. Nothing is unreadable. (Appearance: no automated test.) | A15 | |

## W-2 Title bar (A16)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-2.1 | Press on empty title-bar space and drag. | The window moves with the pointer. | A16 | |
| W-2.2 | Double-click empty title-bar space, then again. | The window maximizes, then returns to its size. | A16 | |
| W-2.3 | Look at the window controls at the bar's right end. Click minimize, restore the window from your task switcher. Click maximize, then restore. | Three controls: minimize, maximize or restore, close. Each does what it says. No second title bar is drawn above the app's (K-4: write down if one is). | A16 | |
| W-2.4 | Press Tab repeatedly from the item area, watching the title bar. | Write down whether the window controls take keyboard focus and in what order; if they do, each shows a focus ring and Enter presses it. (Their keyboard reach is the Kit's: this step records what it does, it is not asserted by a test.) | A13, A16 | |
| W-2.5 | Hover each window edge and corner; drag one edge and one corner to resize. | The pointer changes to a resize pointer at each edge; the window resizes smoothly. | A16 | |
| W-2.6 | Look at the window's shadow and rounded corners, restored and maximized. | A soft shadow around the window when restored; none when maximized. No black or white corner artefact. | A16 | |
| W-2.7 | Look closely at the app icon in the title bar's left (18 px, from a 64 px image). | It is sharp enough to recognise, on the dark chrome, with **no white or opaque square corners**. | A16 | |
| W-2.8 | Select the long-named repository. | The title reads "Manyhands — {name}" with the whole name; the bar does not wrap, and the window controls stay visible and clickable. Write down whether a very long name runs under the controls (known: the title is not truncated). | A6, A16 | |
| W-2.9 | Select `scratch`, open a ticket, press **Edit**, type a few words in the body, and do not save. Click the title bar's **close** control. | The app quits through its normal quit: "Saving drafts for recovery" may flash in Activity, then the window closes. Start it again and open Drafts (Ctrl Shift D): the unsaved text is there as a draft. | A16, A7 | |
| W-2.10 | **(rare)** Crash recovery is off when the draft store cannot open. Quit, then in the scratch data directory: `rm -r "$SCRATCH/manyhands/drafts" && touch "$SCRATCH/manyhands/drafts"`. Start the app, select `scratch`, open a ticket and press **Edit**. | A dialog first says crash recovery is off, with **Continue** (the default) and **Cancel**. Continue; type a few words and do not save; press the title bar's close control. The "Quit" dialog reads "1 tab has unsaved changes that are not kept as a draft", with **Cancel** (the default) and **Quit anyway**. Cancel keeps the window and shows the tab. When done, quit with Quit anyway and put the folder back: `rm "$SCRATCH/manyhands/drafts"` (the app recreates it). This changes only the scratch directory. | A7 | |
| W-2.11 | Select `scratch` and look at your desktop's task list, window overview or window switcher. | The window's title there is "Manyhands — scratch"; with nothing selected, "Manyhands". Select another repository: it follows. | A6 | |

## W-3 Footer, toggles, tooltips, Needs attention and Settings

The footer, left to right: the **left dock** toggle, then the panels that sit
in the left dock (Repositories, Documents); a separator; the **bottom dock**
toggle and Activity. At the right: **Needs attention** (only when above 0),
the right dock's panels (Tickets) and its **right dock** toggle, a separator,
and **Settings**.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-3.1 | Look at the footer. | The controls above, in that order, each an icon (a folder, a ticket, a document, an activity pulse for the panels). Needs attention is absent at 0. | A2, A9 | |
| W-3.2 | Click the **Documents** toggle. | Documents disappears; Repositories takes the whole left dock. The toggle looks off. Click it again: Documents returns with its list state as it was. | A2, A17 | |
| W-3.3 | In Tickets, type in the filter or select a row, then hide Tickets with its toggle and show it again. | The filter, the Find text and the selected row are as you left them. | A17 | |
| W-3.4 | Hide **Repositories**, then **Documents**, so the left dock holds no visible panel. | The left dock closes entirely (no empty column). Show Documents: the left dock opens again with it. | A2 | |
| W-3.5 | Click the **left dock** toggle, then again. Do the same for the right dock and the bottom dock. | Each dock closes and opens in place with its panels, groups and sizes kept. A closed bottom dock leaves a thin strip (K-15). | A2, A3 | |
| W-3.6 | Close the left dock with its toggle while it holds panels you hid by their own toggles (Documents off). Open the dock again with its toggle. | The panels that were off are shown again (a dock toggle turns its panels on). | A2 | |
| W-3.7 | Hover each footer control for a moment. | A tooltip: "Hide Documents" / "Show Documents" and so on for panels; "Hide left dock" / "Show left dock" (and right, bottom) with the key beside it (Ctrl B, Ctrl Alt B, Ctrl J); on Settings, "Settings" with Ctrl ,. The tooltip text equals the control's name and follows the state. | A13 | |
| W-3.8 | Make something need attention (a failed share or recovery as in D5 section 4; or a repository whose folder you renamed). | The footer shows a red "1 needs attention" (with two or more, "2 need attention"). Nothing else changes. | A9 | |
| W-3.9 | Hide Activity, then click Needs attention. | Activity is revealed and focused (its ring shows); the footer item stays. | A9, A2 | |
| W-3.10 | Resolve the problem. | The count falls; at 0 the control disappears. | A9 | |
| W-3.11 | Click **Settings**. | The Settings page opens on Repositories, as Ctrl , does. | A9, A2 | |
| W-3.12 | With the keyboard only: Ctrl 3, then Tab until the Documents toggle has a ring. Press Enter, then Space. | A ring shows on each stop. Enter hides Documents, Space shows it; focus stays on the toggle. | A2, A13 | |
| W-3.13 | (a) Open a **dialog**: Ctrl Q with an unsaved edit and crash recovery off (W-2.10), or press **Close ticket…** on a ticket. With the dialog open, click the footer toggles and press Ctrl B, Ctrl Alt B and Ctrl J. (b) Open the ticket's **details sheet** (a header chip), and press Ctrl B, Ctrl Alt B and Ctrl J with the keys only. Then click a footer toggle. | (a) Nothing in the layout changes and the dialog stays open. (b) The keys change nothing. The first footer click lands on the sheet's overlay: it closes the sheet and changes nothing in the layout; a second click then works as usual. | A12 | |

## W-4 The dock menu and the context-menu key (U-3)

The dock menu has three items: **Dock left**, **Dock right**, **Dock bottom**.
The panel's current side is checked.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-4.1 | Right-click the **Tickets** toggle. | A menu opens **above** the toggle (the footer is at the foot of the window) with "Dock left", "Dock right" (checked), "Dock bottom". Keyboard focus is in the menu. | A2 | |
| W-4.2 | Click **Dock left**. | Tickets moves to the left dock as a new group (under or beside Repositories and Documents), is shown, and its footer toggle moves to the left group. The new group has a sensible share of the dock, neither a sliver nor all of it. | A2 | |
| W-4.3 | Right-click **Tickets** again, then press Escape. | The menu closes; nothing moves; focus is back where it was. | A2 | |
| W-4.4 | Move Tickets to **Dock bottom** and then **Dock right** with the menu. | Each move shows Tickets in that dock, revealed, with its footer toggle in the matching group. Choosing the side it is already on does nothing. | A2 | |
| W-4.5 | Tab to the Documents toggle (Ctrl 3, then Tab). Press the **context-menu key** (the key beside right Ctrl, if your keyboard has one). | **U-3:** write down whether the dock menu opens. If it does, the menu opens above the toggle with focus in it. | A2, A10 | |
| W-4.6 | Press **Shift F10** on the same toggle. | The menu opens (this must work either way). Down, Down, Enter: the panel moves; focus is on its toggle. | A2, A10 | |
| W-4.7 | Press the context-menu key and Shift F10 with focus on a list, not a toggle. | Nothing opens. | A10 | |
| W-4.8 | Open the dock menu, then open the ticket's details sheet (a header chip) or the **Close ticket…** dialog by the keyboard. | The menu closes before the dialog or sheet is shown, and the dialog or sheet has focus. | A12 | |

## W-5 Keys

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-5.1 | Click in the item area (not an editor). Press **Ctrl B**, again, then **Ctrl Alt B** and again, then **Ctrl J** and again. | Ctrl B closes and opens the left dock; Ctrl Alt B the right dock; Ctrl J the bottom dock. Each is toggled with its panels intact. | A10 | |
| W-5.2 | Open a ticket, press **Edit**, type a line in the body, select a word, press **Ctrl B**. | The word becomes bold (`**word**` in the source). The left dock does not move. Ctrl B again removes it. | A10 | |
| W-5.3 | Press Escape to leave the editor (focus on Done), then Ctrl B. | The left dock toggles. | A10 | |
| W-5.4 | Press **Ctrl P** (Find, "Find by short code or ID"), then Ctrl B with the caret in it. | The left dock toggles (accepted: Find is a plain input, not an editor). The text typed stays. | A10, A11 | |
| W-5.5 | **(rare)** In an editor, start a `$$` formula and press Ctrl B inside it. | The left dock toggles (the editor clears its key context there). | A10 | |
| W-5.6 | Hide Tickets, Documents, Repositories and Activity one at a time with their toggles, and bring each back with its key: Ctrl P, Ctrl 2, Ctrl 1, Ctrl 4. | Each key reveals its panel (and opens its dock if closed) and focuses it, with a ring. Ctrl 2 follows the list you used last (Tickets after Find). | A10, A11 | |
| W-5.7 | Close the bottom dock with Ctrl J, then press Ctrl 4. | The bottom dock opens and Activity is focused. | A10 | |
| W-5.8 | Press F6 and Shift F6 repeatedly, with some panels hidden and a dock closed. | The ring visits only the regions drawn, in order, and never a hidden panel. Only the displayed tab of a tabbed group is a stop (K-3). | A10 | |
| W-5.9 | Hide the panel that holds focus (focus Tickets with Ctrl 2, then hide it from the footer by mouse). | Focus moves to the item area (its ring shows); no ring is left on a vanished panel. | A2, A17 | |

## W-6 Dragging, resizing and the closed strip

Drag by the panel's header (the row with its icon, title and count).

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-6.1 | Drag the **Documents** header onto the right dock, over **Tickets**' header, and release. | A drop indicator shows while you hold. After release Documents sits in the right dock with Tickets (as the indicator showed: a tab beside Tickets, or a new group). It is shown, and its footer toggle moves to the right group. | A2, A20 | |
| W-6.2 | Drag the Documents tab back to the left dock, near the bottom edge of Repositories' group. | A new group below Repositories, or a tab, as the indicator showed. | A2 | |
| W-6.3 | Drag a panel onto another panel's header so they form a tabbed group (for example Tickets onto Documents). | A tab bar with one tab per panel (icon and title; with more than two tabs the short names "Repos", "Tickets", "Docs"). The active tab is marked by an underline. Clicking another tab selects it and focuses its panel. | A20, A2 | |
| W-6.4 | In a group of three or more tabs, drag a tab to the empty space **past the last tab** in the tab bar. | The tab lands after the last tab, not before it. Repeat in a group where one panel is hidden (its toggle off): the drop still lands after the last visible tab. | A20 | |
| W-6.5 | Move a panel into a tabbed group, wait a second, and quit with Ctrl Q. Start the app again. | The tab group is as you left it (same panels, same active tab). | A20, A4 | |
| W-6.6 | Try to drag **Tickets**' header out of the right dock when it is the dock's only panel. | It cannot be dragged out (K-11). Use its dock menu instead. | A2 | |
| W-6.7 | Drag a panel over the **item area** (the big middle area) and hold. | The Kit's drop indicator shows over the item area (K-2). Write down what it looks like. | A2 | |
| W-6.8 | Release the panel over the item area. | The drop is refused: the panel returns to where it was, every other panel and dock is as before, and nothing is saved. Focus is on the item area (its ring), and a focused button inside the area may lose its focus (K-2). | A2, A20 | |
| W-6.9 | Drag the **edge between the left dock and the item area**, and then the right dock's edge, and the bottom dock's top edge. | The dock resizes with the pointer and does not go below about 100 px. (Mouse only: K-1.) | A3 | |
| W-6.10 | Drag a **divider between two stacked groups** in a dock (for example between Repositories and Documents). | The groups resize. Hide one of them with its toggle and show it again: the divider is where you left it. | A3 | |
| W-6.11 | Close the bottom dock with Ctrl J. Look at the strip. Click the **Activity** tab in the strip. | The strip names every panel in the bottom dock, one tab each. The click opens the dock, shows Activity and focuses it (its ring shows). | A2, A17 | |
| W-6.12 | Close the bottom dock again. In an **active window**, click the strip **outside its tabs** (the empty part of the strip). | The dock does not open by itself. Keyboard focus is not left in a hidden panel: press Tab or F6 and write down where the ring goes (it should be a drawn region). | A2, A17 | |
| W-6.13 | With the bottom dock closed, press on the strip's **top edge** and drag it upward. | The dock opens as it would by its toggle: the footer's bottom toggle reads "Hide bottom dock", Activity is shown, and the panels you had turned off stay off. (Mouse only: K-1.) | A2, A3 | |
| W-6.14 | Hide every panel in the bottom dock, close it with Ctrl J, and open it by dragging the strip edge. | The dock may open but shows no panel turned on that you turned off. Write down what shows. | A2 | |
| W-6.15 | Drag a panel into the bottom dock beside Activity, then split the bottom dock top and bottom by a drag; close the dock. | The closed strip shows only the top group's tabs (K-17). | A2 | |
| W-6.16 | Move a divider and a dock edge by only a few pixels (3 or 4), wait a second, quit, start again. | The size you left is restored within a few pixels. If it is not, write down by how much (the layout's change test ignores a shift under about 0.5% until the next real change). | A3, A4 | |

## W-7 Building layouts 1a, 1b and 1c (Wave Integration Gate)

Without Projects and Teams. Start each from the default (1b), using the
footer, the dock menu and dragging. Relaunch after each to see it back.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-7.1 | **1b** (the default). Quit, `mv "$SCRATCH/manyhands/layout/layout.json" "$SCRATCH/manyhands/layout/layout.json.bak"` and start the app, or arrange the default by hand: left Repositories over Documents (290 px), right Tickets (440 px), bottom Activity (150 px). | Looks as in "The default layout". Quit, start again: the same. | A1, A4 | |
| W-7.2 | **1a** (classic). Start from 1b. Move **Documents** to the right dock with its dock menu ("Dock right"), so the right dock stacks Tickets over Documents. Drag the left dock's edge to about 264 px, the right dock's edge to about 400 px and the bottom dock's edge to about 170 px (the sizes of frame 1a in `docs/design/02-layout-and-docking.md`). | Left dock: Repositories alone. Right dock: Tickets over Documents. Bottom: Activity at about 170 px. Quit and start again: the same, sizes within a few pixels. | A2, A3, A4 | |
| W-7.3 | **1c** (single sidebar). Start from 1b. Move **Tickets** to the left dock ("Dock left"), then drag Tickets' header onto Documents' header so they form the tabs **Tickets · Documents**. Hide **Activity** with its toggle, and drag the left dock's edge to about 380 px (frame 1c's size in `docs/design/02-layout-and-docking.md`). | One dock on the left: Repositories above one tabbed group of Tickets and Documents (the tab order is whatever the drop indicator showed). The right dock is gone (no empty column); the bottom dock is gone or closed (a hidden Activity). The footer shows the left dock toggle with its panels, no right dock toggle. Quit and start again: the same. | A2, A4, A20 | |
| W-7.4 | From 1c, get back to 1b with the footer and the dock menu only (Tickets "Dock right"; Activity shown). | The default look again. | A2 | |
| W-7.5 | From 1b, put all three of Repositories, Documents and Tickets in one tabbed group by dragging. | One group of three tabs, named "Repos", "Docs", "Tickets" (short names). Relaunch: same. | A20, A4 | |

## W-8 The layout file, restart and the notices

All of this is in the scratch data directory (see "Before you start"); `L`
below is `$SCRATCH/manyhands/layout`. Before W-8.6, once a good file exists,
copy it: `cp "$L/layout.json" "$L/layout.json.good"`. Each step that quits
uses Ctrl Q.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-8.1 | Hide Documents, close the bottom dock (Ctrl J), move Tickets to the left dock, drag the left dock to about 330 px. Wait a second. Quit and start again. | Documents is off (its toggle reads "Show Documents"), the bottom dock closed, Tickets on the left, the left dock about 330 px. No notice shows. | A4 | |
| W-8.2 | Make one change (hide Documents) and press Ctrl Q **at once**, within half a second. Start again. | The change is there (the quit writes the pending layout). | A4 | |
| W-8.3 | Close the left dock with Ctrl B, restart. | The left dock is still closed (the footer's left toggle reads "Show left dock"); Ctrl B opens it with its panels, groups and sizes. | A4 | |
| W-8.4 | Resize the window narrower and wider only, then quit and start again. | No layout change is saved by a resize alone (K-9); the layout is as before the resize. | A4 | |
| W-8.5 | Hide Repositories, restart. | Hidden. In an active window the first keyboard focus (the ring at launch) is on the first displayed stop, Documents (R-9), not on a hidden panel; the default launch (W-1.2) focuses Repositories. | A4, A10 | |
| W-8.6 | Quit. `cp "$L/layout.json" "$L/layout.json.good"`, then replace `layout.json` with the text `not json` (`echo 'not json' > "$L/layout.json"`). Start the app. | The default layout comes back with this notice at the top of Activity: "Your panel layout couldn't be restored, so the default layout is back." with a **Dismiss** button. Drafts, resolutions and repositories are untouched. "Needs attention" does not count it. | A5, A14 | |
| W-8.7 | Click **Dismiss** with the mouse. | The notice goes. | A5 | |
| W-8.8 | Quit without changing anything and start again. | The bad file is still there and the notice shows again (a failed restore with no change leaves the file). | A5 | |
| W-8.9 | Change something (hide Documents), wait a second, quit and start again. | The file is rewritten as a good one: no notice. | A5, A4 | |
| W-8.10 | Quit. Replace `layout.json` with an **empty folder** of that name: `mv "$L/layout.json" "$L/layout.json.bak" && mkdir "$L/layout.json"`. Start the app. | The same restore notice shows, and the default layout. | A5 | |
| W-8.11 | With the folder in place, hide Documents and wait a second. | A second notice: "Your panel layout couldn't be saved." with **Dismiss**, in Activity under the first. Neither counts in Needs attention. | A19, A5 | |
| W-8.12 | By keyboard only: Ctrl 3, Tab to the **Dismiss** of the restore notice (a ring shows; a screen reader says "Dismiss"), press Enter. Then Tab to the save notice's Dismiss and press Space. | Each notice goes. The ring, the name and the keys work as the automated journey asserts; this step confirms them in a real window. | A19, A5, A13 | |
| W-8.13 | Make another change while the folder is still there. | The save notice shows again (a new failure). Dismiss it. | A19 | |
| W-8.14 | Quit. `rmdir "$L/layout.json" && cp "$L/layout.json.good" "$L/layout.json"`. Start again. | No notice; the layout is the saved one. | A4, A5 | |
| W-8.15 | **(rare)** Set the two side docks to sizes that cannot fit. Quit, then in `$L` (both docks exist in the default layout; the file is one minified line, so do not edit it by hand): `jq '.area.left_dock.size=900 \| .area.right_dock.size=900' layout.json > layout.new && mv layout.new layout.json`. Without `jq`: `python3 -c "import json;d=json.load(open('layout.json'));d['area']['left_dock']['size']=900;d['area']['right_dock']['size']=900;json.dump(d,open('layout.new','w'))" && mv layout.new layout.json`. Start the app. | **No restore notice** (the file is valid). Once the window is drawn the docks are scaled down together so the item area keeps about 320 px, and the app is usable. The next save holds the smaller sizes. A dock missing from the file would make the file invalid, so restore it with `cp layout.json.good layout.json` if the notice shows. | A3, A4 | |
| W-8.16 | Close the right dock. Shrink the window to about 700 px wide. Open the right dock with Ctrl Alt B. | The right dock opens at a width that leaves the item area about 320 px (a closed oversized dock is scaled when it opens); the left dock is scaled with it if both do not fit. | A3 | |
| W-8.17 | **(rare)** Shrink the window very small (about 500 x 500) with both side docks and the bottom dock open. | Docks keep at least about 100 px; the item area keeps what it can; nothing is unreachable by key. Widen the window again. | A3 | |
| W-8.18 | **(rare, proposed limit)** Make a layout change and `kill -9` the app within half a second. Start again. | The change may be missing (a killed app writes nothing). A change older than a second is there. | A4 | |

## W-9 Quit with a pending change and with Activity hidden

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-9.1 | Start a job that takes a while: Refresh (Ctrl R) on a large repository (a clone of a big project) under `$SCRATCH/repos`, or **(rare)** a share to a stalled remote as in D5 (that needs Shared SSH keys, which use `~/.ssh/manyhands` outside the scratch directory). Leave Activity visible. Press Ctrl Q. | Activity reads "Finishing {the job}" with **Quit now** focused (a ring). Press Quit now, or let the job end: the app quits. | A7 | |
| W-9.2 | Start the same job, hide Activity with its footer toggle, press Ctrl Q. | **Nothing appears** and the layout does not change: Activity stays hidden, no focus moves to a control you cannot see (K-18). The window just waits. | A7 | |
| W-9.3 | While it waits, press Ctrl 4. | Activity is revealed and shows "Finishing {the job}" with **Quit now**. Quit now (or release the job) quits. | A7 | |
| W-9.4 | While the quit waits with Activity hidden, hide Documents. Let the job end. Start again. | The change was kept (the quit writes the layout when it exits). | A7, A4 | |
| W-9.5 | Edit a ticket without saving, with crash recovery off as in W-2.10 (the scratch `drafts` replaced by a file). Hide Activity, press Ctrl Q, and choose **Cancel** in the dialog. | The dialog closes; focus is in the item area on the tab with the unsaved text; the layout is unchanged; Activity is still hidden. | A7 | |

## W-10 Activity at the default width (proposed K-19)

Use the default 1280 px window. Activity is about 550 px wide there.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-10.1 | Make a **draft that cannot be saved** (`chmod -w "$SCRATCH/manyhands/drafts"` while you edit a ticket; `chmod +w` afterwards). Look at its entry in Activity. | Write down whether the entry's **Retry** is visible at 550 px or sits outside Activity (proposed K-19). | A17 | |
| W-10.2 | Make a repository whose read fails and whose row has a long name or state (rename its folder, or use the long-named one). Look at its row under "Pending in repositories". | Write down whether **Retry** is visible or clipped. | A17 | |
| W-10.3 | Make a pending row with several controls (Resume, Review changes, Abandon: a stranded share). | Write down which of its controls are visible; usually the later ones, such as Abandon, can be clipped. Widen the window (or the item area) until they show. | A17 | |
| W-10.4 | At the default 150 px Activity height, make both layout notices show (W-8.10, W-8.11) and a pending row. | The notices stay at the top and do not scroll away; the operation area below scrolls vertically. Each notice takes about 36 px (K-13). | A5, A19 | |
| W-10.5 | Drag the bottom dock's top edge up until Activity is about 300 px tall. | More of Activity shows; the notices stay at the top. | A17 | |

## W-11 The arranging journey on a real screen, keyboard only

Do not use the pointer in this section. Start from the default layout, with
one repository selected and a ticket open in a tab. This is the on-screen
twin of the automated `arrange_the_workspace_by_keyboard_alone`.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-11.1 | Ctrl 3, then Tab to the **Documents** toggle; press Enter. | A ring on each stop. Documents is hidden; the toggle reads "Show Documents". Nothing is reloaded: the open ticket tab stays as it is. | A2, A18 | |
| W-11.2 | Tab to the **Tickets** toggle; press the context-menu key (or Shift F10). Down, Enter. | The menu opens with focus in it; Tickets moves to the left dock and is shown; focus is on its toggle. | A2, A10, A18 | |
| W-11.3 | Ctrl J, then Ctrl J. | The bottom dock closes and opens. | A10, A18 | |
| W-11.4 | Tab to the **Activity** toggle; press Enter. Then Ctrl 4. | Activity is hidden, then Ctrl 4 shows it and focuses it (its ring shows). | A2, A10, A18 | |
| W-11.5 | Wait a second, Ctrl Q, start again. | The same arrangement: Documents hidden, Tickets on the left, Activity shown. The ticket tab is not restored (K-6). | A4 | |

## W-12 Orca (A13)

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| W-12.1 | Turn on Orca (Super Alt S, or your desktop's switch). Tab to the **Documents** toggle in the footer. | Orca reads its name and state: "Hide Documents", a toggle button, pressed or checked (on). It is one control, not two. | A13 | |
| W-12.2 | Press Enter on it. | Orca reads "Show Documents" and the state off. Press Space: back to "Hide Documents", on. | A13 | |
| W-12.3 | Tab to a dock toggle and to Needs attention (make one), and to Settings. | "Hide left dock" (or "Show…") with its state; "1 needs attention"; "Settings". | A13, A9 | |
| W-12.4 | Open the restore notice (W-8.10) and Tab to its Dismiss. | Orca reads "Dismiss". | A13, A5 | |
| W-12.5 | Close the bottom dock (Ctrl J), then find the tabs in the **closed strip**. | Write down whether Tab reaches the strip tabs. Orca reads each one's name (the panel's short title, such as "Activity"), by Tab if they are stops or by its pointer review otherwise. Clicking one opens the dock and shows that panel. | A13, A2 | |

## Results

Write the build (commit), display server, scale factor, whether the
compositor draws its own title bar (K-4), and one line per section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| W-1 | Smoke and the default look (A1, A6, A8, A15) | |
| W-2 | Title bar (A6, A7, A13, A16; K-4) | |
| W-3 | Footer, toggles, tooltips, Needs attention, Settings (A2, A9, A12, A13, A17) | |
| W-4 | Dock menu and the context-menu key (A2, A10, A12; U-3, K-5) | |
| W-5 | Keys (A10, A11, A17) | |
| W-6 | Dragging, resizing, the closed strip (A2, A3, A20; K-1, K-2, K-11, K-17) | |
| W-7 | Layouts 1a, 1b, 1c (A1 to A4, A20; Wave Integration Gate) | |
| W-8 | The layout file, restart and the notices (A3 to A5, A14, A19) | |
| W-9 | Quit with a pending change and with Activity hidden (A4, A7; K-18) | |
| W-10 | Activity at the default width (A17; proposed K-19) | |
| W-11 | The arranging journey, keyboard only (A2, A4, A10, A18) | |
| W-12 | Orca (A13) | |
| — | Known limits hit (K-numbers, steps) | |
| — | U-3: the context-menu key arrives as `menu` under Wayland? (yes / no / no such key) | |
| — | Anything the checklist could not reach | |
