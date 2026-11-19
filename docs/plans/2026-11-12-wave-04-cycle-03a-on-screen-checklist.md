# Wave 04 Cycle 03a On-Screen Checklist

This list is for checking the Cycle 03a Repositories panel and footer (one
state per repository row, the repository menu, the panel toolbar, the
footer's "Checked for updates" and "N to share", the empty item area) by
hand on Linux Wayland, in the dark theme, before the Cycle is called
review-ready. It is the native evidence the design asks for (design,
"Testing"; Cycle document, A13, "On screen"; implementation plan, Task 10),
and it carries what no test can see: the panel against the frames, the
states against a real shared copy, a real right click and a real menu key,
a screen reader, and a focus move the footer makes while it draws. Work
through sections S-1 to S-9 in order. Write each step's result in its row's
last column, and a one-line summary per section in "Results" at the end.
Claude records only what you report.

Quoted text is the app's own wording. `{repo}` stands for a repository's
display name (its directory's name), `{n}` for a count, `{ago}` for a time
such as "just now" or "2 minutes ago". "Ctrl" is the `secondary` key of the
keymap on Linux. A key written `Ctrl Shift R` is held together; `Shift F10`
likewise; "the menu key" is the context-menu key (`menu`). "The frames" are
`docs/design/screenshots/1a-layout-classic.png` (the classic layout) and
`1b-layout-docked-tabs.png` (the default layout), read with
`docs/design/03-panels.md` (Repositories) and `06-language-and-states.md`.
Both frames draw four rows in the Repositories panel: "manyhands" reading "2
to share" in amber, "manyhands-site" reading "Up to date" in green,
"team-handbook" reading "This computer" in `secondary`, and "zorite" reading
"Can't reach" in red.

The **Supports** column names the Acceptance row (A1 to A13, Cycle document,
"Acceptance And Exit Evidence") and the design's Required Outcome (P1 to
P44, design, "Required Outcomes") the step is evidence for. A step with no
automated twin says so; a step whose twin is only a pure function (the
snapshot cannot read a tooltip or a description) says "pure-function twin".
Steps marked **(rare)** need a state that is hard to make by hand: try each
one, and if you cannot reach it write "skipped". Each is also covered by an
automated test. Steps marked **Owner decides** ask you to look and say what
you want; they are not pass or fail. Steps marked **Watch** name something
that has never been seen on a real display: write what you saw, either way.

## Minimum pass

If you cannot do every row, these cover each Acceptance row that has
on-screen evidence at least once. Nothing below is removed from the list; the
other rows add depth.

- **A13, the panel and footer against frames 1a and 1b in the dark theme
  (the owner's eye, no automated twin):** S-1.3 to S-1.6, S-2.1, S-2.2,
  S-2.3, S-2.4, S-2.5, S-2.8, S-6.1 and S-6.2.
- **A1, each state comes from real service state:** S-2.1 to S-2.14, S-3.1.
- **A2, "N to share" counts, follows a save and a share, and agrees with
  Settings:** S-2.3, S-2.4, S-6.5 to S-6.7.
- **A3, every action works by keyboard and the reason is readable without
  hovering:** S-4.1 to S-4.4, S-4.9, S-5.1, S-5.2, S-6.4.
- **A6, the details block has left the item area:** S-7.1 to S-7.3.
- **A7, removal confirms and keeps the files:** S-4.10, S-4.11.
- **A9, one meaning of selected:** S-1.6, S-4.5.
- **A10, navigation sends nothing:** S-4.5, S-7.4.
- **A11, Check now and Check all only check; Check all skips what waits on a
  key or host:** S-4.6, S-5.4 to S-5.7.
- **A12, colour is never the only signal:** S-1.6, S-2.1 to S-2.14.
- **A4, no secret on screen:** S-8.1 to S-8.4.
- **A3 and A12, a screen reader:** S-9.1 to S-9.5.

Mostly covered by automated tests, or rare, so **optional** if time is
short: S-1.7, S-2.12, S-2.14, S-3.2, S-4.12, S-5.8, S-6.9 and S-9.6.

## Before you start

- **Use a scratch data directory for everything.** This checklist makes
  repositories, registers a shared copy, renames a repository's folder and
  stops a remote. Run all of it against a throwaway data directory so your
  real one is never touched.
  - **What the app honours.** The desktop reads `XDG_DATA_HOME`, not
    `MANYHANDS_DATA_DIR`; only `manyhands-cli` reads that (Cycle 01
    checklist, "Before you start"). The scratch data directory is then
    `$SCRATCH/manyhands`.
  - **Make it** (one terminal, kept open for the whole checklist):

    ```sh
    export SCRATCH=$(mktemp -d)
    touch "$SCRATCH/.start"
    mkdir "$SCRATCH/repos"
    ```

  - **Smoke test (S-1.1).** First, once, the debug build the Agents file
    names, to see that it starts (from the ticket worktree,
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y8C73B39GR4JWENXE5D0`):
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --locked --features desktop --bin manyhands`
    Quit it (Ctrl Q) when the window is up.
  - **Launch with it**, the release build, under Wayland, for every step
    after S-1.1:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`
    It needs an active desktop display. A development build lags. Run it
    while no long test suite or build is running on the machine.
  - **Check it took (S-1.1).** After the first launch,
    `ls "$SCRATCH/manyhands"` lists `layout` and the registry's files. If it
    does not, the variable did not reach the app: quit and fix that first.
  - **Your real data stays untouched,** except Shared SSH keys, which live in
    `~/.ssh/manyhands`, outside the scratch directory; that is expected (the
    Wave 03 D6 checklist says the same). At the end,
    `find ~/.local/share/manyhands -newer "$SCRATCH/.start"` prints nothing
    (adjust the path if your `XDG_DATA_HOME` is normally set).
  - **Where the CLI looks** (only S-2.9 and S-2.13 need it):
    `devenv shell -- env MANYHANDS_DATA_DIR="$SCRATCH/manyhands" cargo run --locked --bin manyhands-cli -- …`
- **Six scratch repositories**, all under `$SCRATCH/repos`. Make each with
  Settings (Ctrl ,) › Repositories › **Create repository**, as in Cycle 02's
  checklist: its folder `$SCRATCH/repos/{name}`, primary branch `main`. If the
  form asks for a Git name and email, give any. Give each two or three
  tickets with Cycle 02's `commit` and `ticket` shell helpers ("Before you
  start", "The helpers"), from the repository's folder, with its own
  26-character ID prefix (`01ARZ3NDEKTSV4RRFFQ69G5H` for `shared`,
  `…G5J` for `second`, `…G5K` for the rest), then press Ctrl R with that
  repository selected. Make the two SSH repositories as the Wave 03 D6
  checklist's "Two scratch repositories", "Shared scratch" does
  (`docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md`):
  1. Create or import a **passphrase-protected** key and **select it before**
     configuring the remote (Settings › Shared SSH keys).
  2. On an SSH host you control make a bare repository
     (`git init --bare shared.git`, and `second.git` for the second one).
     Settings › Remotes › Add a remote (name `origin`, the SSH location), then
     **Select for publication**.
  3. Commit the changed Manyhands configuration file in the repository's
     folder, then `git push origin main`. Approve the host when first asked.

  | Name | What it is | Used for |
  | --- | --- | --- |
  | `shared` | SSH shared copy `shared.git`, at least seven tickets | To share, Up to date, Checking…, Can't reach, Locked, Needs attention, Check all |
  | `second` | SSH shared copy `second.git`, three tickets | A second repository with work to share, Paused, Check all's skipped ones |
  | `local` | No remote | This computer |
  | `plain` | A remote that is **not** SSH, such as a `file://` path to a bare repository you make with `git init --bare` (S-2.8 says how) | The non-SSH state |
  | `away` | No remote; its folder is renamed in S-2.9 | Can't open, the kept row |
  | `fixing` | No remote; its configuration is broken in S-2.11 | Needs fixing |

- **Stalling and stopping the remote** (Can't reach, Checking…): stop the
  host's **listening** `sshd` as the Wave 03 D6 checklist's "Stalling the
  remote" does: `pgrep -a sshd`, then `sudo kill -STOP <listener>` to stall
  (a check hangs, then fails after about 30 s) and `sudo kill -CONT
  <listener>` to release. Have both commands ready in a terminal on the host.
- **Checking interval.** The app checks a repository by itself on its
  interval (Settings › Checking for updates: every 1, 5, 10, 15, 30 or 60
  minutes). Set `shared` and `second` to Every 1 minute where a step needs a
  check soon, and to Every 60 minutes where a step needs it not to check
  (S-3).
- **A window size.** The app opens at 1280 x 840 in the default layout (Cycle
  01, "The default layout"): Repositories over Documents in the left dock
  (290 px), Tickets alone in the right dock (440 px), Activity at the foot.
  Keep Activity visible: several steps read it.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure. The
first group is this Cycle's design "Known Limits" (K-1 to K-11); the second
is from the earlier Cycles; the third is what the frames draw and this Cycle
does not build.

- **K-1.** "N to share" follows this app's saves and shares at once, from the
  first save of a session, even before any completed check, and a completed
  check sets it whole. Another process's or the CLI's work, a resumed share,
  a comment's publication and a conflict resolution's share show at the next
  check.
- **K-2.** After a relaunch a stored outcome stands until this app checks.
- **K-3.** An unshared item is named by its ID until its title is read, for
  the session after three failed tries, and when a local conflict resolution
  added it. **K-11.** A row names at most five items, then "and {n} more".
- **K-4.** After a relaunch a locked key is offered **Check now** rather than
  **Unlock…**.
- **K-5.** A publication remote that is not SSH shows no label; its sentence
  says why.
- **K-6.** Tooltips show on hover only. Their text is also each row's
  accessible description, and the state's sentence alone is its menu's first
  line.
- **K-7.** "Checked for updates {ago}" is worked out when the footer is drawn,
  so an idle footer's age does not tick on its own.
- **K-8.** The panel's own Settings button stays below the list until Cycle
  04.
- **K-9.** The footer's "need attention" does not yet count Can't open, Needs
  fixing, Can't reach or the Needs attention this Cycle adds; Cycle 03b
  decides.
- **K-10.** "Checked for updates {ago}" is the latest check of any
  repository, even while another cannot be reached.
- **Cycle 01 K-3.** Only a tabbed group's displayed tab is an F6 stop.
- **Frames, not failures.** The frames draw things this Cycle does not build:
  the Repos tab strip with Projects and Teams, the scope button in the title
  bar, a repository as a root node of the Documents tree, "from {k}
  repositories" in the Tickets footnote, and `MH-VH-…` upper-case short
  codes. The empty item area's icon waits for Cycle 05.

## S-1 Launch, an empty panel and the first look

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-1.1 | Run the smoke test and the "Check it took" listing above. | A window opens with no error in the terminal; the title is "Manyhands"; `$SCRATCH/manyhands` lists `layout`. | A5 (no automated twin) | |
| S-1.2 | Before any repository exists, look at the Repositories panel, then its header. | The body says "No repositories yet." with a button **Add or create a repository…** (no "Loading repositories" bars left, no old heading in the body); the header shows "Repositories" and its three buttons (`plus`, `refresh-cw`, `ellipsis`). Press the empty state's button: Settings opens on its Repositories page. Close Settings. | P8 (twin: `rows_and_list_states`) | |
| S-1.3 | Make the six repositories as in "Before you start", press Ctrl R in each. Look at the panel against frame 1b with `shared` chosen. | One 28 px row per repository: a `folder-git-2` icon (14 px, `secondary`), the name, and at the right the state (a 12 px icon and a label in 11.5 px, in the state's colour). No "Unavailable" badge and no path line under any row. The look is the frame's, with the "Frames, not failures" differences. | A13, A1 (appearance: no automated twin) | |
| S-1.4 | Look at the chosen row against an unchosen one. | The chosen row has the `selection` background, a 2 px amber line at its left edge and its name in a heavier weight. The others are unchanged. | A13, A9 (appearance: no automated twin) | |
| S-1.5 | Press Ctrl 1 (focus the Repositories panel), then Down twice. | The panel draws its region ring; the keyboard cursor shows its focus ring on the row it rests on; the chosen row keeps its treatment and does not move until you press Enter. | A9, A3 (twin: `a_repository_rows_selected_state_has_one_meaning`; the ring is appearance) | |
| S-1.6 | Look at the colours, then at one row's state with the colour taken away in your head (read the words). | Dark theme: chrome (header) darker than the panel body, thin borders, nothing unreadable. Amber for "to share", Needs fixing and Locked; green for Up to date; teal for Checking…; red for Can't reach, Can't open and Needs attention; `secondary` for This computer; muted for Paused. **Every state has a word and an icon**, so none depends on its colour. | A12, A13 (no automated twin for the look) | |
| S-1.7 | **(rare)** **First load.** Select a repository from a cold start and look at the panel the instant it appears. | Six grey static bars read as "Loading repositories" where the rows will be; too fast to see on a small registry. If you cannot catch it write "skipped". | P8 (twin: `rows_and_list_states`) | |
| S-1.8 | Compare the header against frame 1a. | Icon, "Repositories", the count (one per row drawn; the kept row of S-2.9 counts), and at the right three 26 x 24 px buttons with 14 px grey icons: `plus`, `refresh-cw`, `ellipsis`. | A13 (appearance), P8 | |

## S-2 Each state, one row each

Put `shared` and `second` on Every 1 minute for this section unless a step
says otherwise. For each row, read three things: the state **in the row**
(label, icon, colour), the **tooltip** (hover about a second; K-6), and the
**first line of its menu** (S-4 describes opening the menu; here press Shift
F10 on the row). The tooltip's lines, after the state's sentence, are
`Folder: {path}`, `Main branch: main` and, for a repository with a shared
copy, `Shared copy: {remote name} · {redacted address}`. Write down what you
read where the Expect column says "write down".

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-2.1 | **This computer.** Look at `local`. | `hard-drive`, "This computer", `secondary`. Tooltip begins "No shared copy, so changes stay on this computer."; no "Shared copy:" line. Menu first line (greyed, not chosen): the same sentence; then **Set up sharing…**, then Check now (dimmed: there is no shared copy to check), Share main… (the label with the dots, since there is no publication remote), Open folder, Settings…, Remove from Manyhands…. | A1, A12, P1, P7 (pure-function twin: `each_state_and_its_precedence`, `tooltips_say_the_state_and_the_facts`) | |
| S-2.2 | **Up to date.** In `shared`, with nothing unshared, press Check now (menu) and wait. | `circle-check`, "Up to date", green. Tooltip begins "Checked {ago} · up to date" (write the age). Menu first line: that sentence, then Check now (no state action), Share main, Open folder, Settings…, Remove from Manyhands…. | A1, A12, P1, P7 | |
| S-2.3 | **{n} to share, by a save.** In `shared`, open a ticket, Edit, change its body and Save (Ctrl S) but do **not** Share. Look at the row at once, without waiting for a check. | `cloud-upload`, "1 to share", amber, **at once** (decision 24; K-1), never "Up to date" after a save. Tooltip begins "Not shared yet: {the ticket's title}. Share each from its tab or from Activity." Menu first line: the same. The footer reads "1 to share" too. | A1, A2, A12, P4, P30 (twin: `unshared_follows_attempts_shares_and_saves`) | |
| S-2.4 | **Several, and main.** In `shared`, Edit and Save five more different tickets without sharing. Then add a commit to `main` with plain `git` in its folder (a new ticket by Cycle 02's `ticket` helper) and press Check now. | "7 to share" (six items and main; main counts as one). Tooltip: "Not shared yet: " then **at most five** names, "main" among them as its own word, then " and {k} more." (K-11); then "Share each from its tab or from Activity." and "Share main shares main.". Names are titles; an item whose title is not read yet is named by its ID (K-3). | A2, P4, P6, P7 (pure-function twin: `to_share_counts_and_finished_shares`) | |
| S-2.5 | **A share leaves the count.** Press Share on one of the saved tickets (its tab, header). Look at the row as the share ends. | The count drops by one at once, before any check; the ticket is gone from the tooltip's names. A Share that fails (for example a stalled remote) removes nothing. Press Share main in the menu: "main" leaves. | A2, P5 (twin: `unshared_follows_attempts_shares_and_saves`, `finished_shares_and_saves_are_heard`) | |
| S-2.6 | **Checking….** In `shared`, stall the remote and press Check now. | At once `loader-circle` (it may spin), "Checking…", teal. Tooltip begins "Checking shared for updates…" (with the repository's name). The menu's first line says the same and Check now is dimmed. The footer's Update check reads "Checking for updates…" (S-6). The state stays until the transport gives up (about 30 s). | A1, A12, P1, P2 (pure-function twin: `each_state_and_its_precedence`) | |
| S-2.7 | **Can't reach.** Let S-2.6's check fail, with the remote still stalled. | `circle-alert`, "Can't reach", red. Tooltip begins "Can't reach shared. Your work is safe." Menu first line: the same; then **Retry now** (not "Check now"), Share main, Open folder, Settings…, Remove from Manyhands…. Release the remote (`kill -CONT`) and press Retry now: the row goes through Checking… to Up to date or "to share". | A1, A12, P1, P3 | |
| S-2.8 | **A remote that is not SSH.** In `plain`: make a bare repository with `git init --bare "$SCRATCH/plain.git"`. Settings › Remotes › Add a remote for `plain` (name `origin`, location `file://$SCRATCH/plain.git`), then **Select for publication**. Settings says it is "Not eligible for publication: publishing needs a remote reached over SSH" and may still let you select it. If the library refuses the selection write "skipped" and what it said. | **No label** at the right of the row. Tooltip begins "origin isn't an SSH shared copy, so Manyhands can't check or share it." Menu first line: the same; then **Set up sharing…** (it opens Settings › Remotes with `plain` chosen), Check now (write whether it is dimmed), **Share main dimmed**, Open folder, Settings…, Remove from Manyhands…. | A1, P20 (twin: `each_state_and_its_precedence`, `the_menu_by_pointer_and_keyboard`) | |
| S-2.9 | **Can't open.** Choose `away`. Then, in a terminal, `mv "$SCRATCH/repos/away" "$SCRATCH/repos/away.gone"` and press Ctrl R. Look at the row. Then `mv` it back and press **Retry** in its menu. | After the rename: `circle-alert`, "Can't open", red. Tooltip begins with the library's reason ("The repository folder is missing" or its own wording; write it), then "Its files are unchanged. Retry, or remove it from Manyhands." Menu first line: the same; then **Retry**, then the fixed items with Share main and Open folder **dimmed** (write whether Check now is). After the `mv` back and Retry the row returns to its own state (This computer). | A1, P1, P38 (twin: `each_state_and_its_precedence`, E-1 `a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`) | |
| S-2.10 | **The kept row.** Rename `away` again and press Ctrl R so it reads Can't open, with `away` still chosen. Then, with the CLI, remove its registration as a second actor: `… -- repo remove --repo "$SCRATCH/repos/away.gone" --prepare`, then the same command with `--confirm <ULID>` using the confirmation ULID the preview printed (`… repo remove --help` lists the options: `--repo`, `--prepare`, `--confirm`, `--non-interactive`, `--json`). Back in the app make the panel read its list again (Ctrl R; if the row does not change try More › Refresh, and write which worked). | `away` **stays in the panel**, drawn last, as the chosen row reading Can't open. Its tooltip begins "away is no longer registered with Manyhands. Its files are unchanged. Retry, or remove it from Manyhands." Its menu offers **Retry** and **Remove from Manyhands…**, Share main and Open folder dimmed. The header counts it. Choose another repository: the kept row goes. | A6, A1, P18, P38 (twin: `each_state_and_its_precedence`, `rows_and_list_states`) | |
| S-2.11 | **Needs fixing.** In `fixing` overwrite `.manyhands/config.toml` with text that is not valid TOML (or delete it), commit it, and press Ctrl R with `fixing` chosen. If the row does not change, say what it showed. | `triangle-alert`, "Needs fixing", amber. Tooltip begins "Its configuration needs fixing:" followed by each problem's guidance (write it). Menu first line: the same; then **Settings…** (it opens Settings › Repositories for `fixing`, and the item is not repeated lower down), then the fixed items (Check now, Share main, Open folder, Remove from Manyhands…). | A1, A12, P1 (twin: `each_state_and_its_precedence`) | |
| S-2.12 | **(rare) Needs attention, from a check's problems.** Make a check list a problem for `shared`, for example a ticket the app has shared whose branch another person deletes without closing it: in a second clone run `git push origin --delete manyhands/ticket/{id}`, then Check now. If you cannot make one write "skipped". | `circle-alert`, "Needs attention", red. Tooltip and menu first line begin "1 item needs a look. Activity lists them." ("{n} items need a look…" for more). The menu has no state action (Activity has them), then Check now and the fixed items. | A1, A12, P1, P3 (twin: `each_state_and_its_precedence`) | |
| S-2.13 | **Paused.** Open Settings › Checking for updates and turn `second`'s switch off. Close Settings. | `pause`, "Paused", muted. Tooltip begins "Checking for updates is paused." Menu first line: the same; then **Check now** (the state's action; it checks once and leaves the repository paused), Share main, Open folder, Settings…, Remove from Manyhands…. A paused repository with items to share reads "{n} to share" instead, as Settings does. | A1, A12, P1 | |
| S-2.14 | **(rare) Locked, live.** Quit (Ctrl Q) and launch again, so the key is locked; set `shared` to Every 1 minute first. Wait about two minutes without pressing anything. | **No passphrase dialog opens by itself.** The row reads `lock`, "Locked", amber. Tooltip begins "Checking is waiting for your key to be unlocked." Menu first line: the same; then **Unlock…** (it opens the "Unlock key" dialog; **Not now** leaves it), then Share main, Open folder, Settings…, Remove from Manyhands…. | A1, A12, P1, P2 (Wave 03 D6 checklist 4.1) | |

## S-3 A relaunch with a stored outcome

The library stores each repository's last check outcome. After a relaunch,
until this app has checked a repository, that outcome sets its state (K-2).

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-3.1 | **Can't reach survives a relaunch.** Set `shared` to Every 60 minutes, stall the remote, press Check now and wait for "Can't reach" (S-2.7). Quit (Ctrl Q) with the remote still stalled, and launch again. Select `shared` and look at once, before anything is checked. | The row reads "Can't reach" straight away, with Retry now in its menu, and Settings › Checking for updates says "Can't reach shared. Your work is safe." for it too, not "Checked 2 hours ago". Release the remote. Press Retry now: Checking…, then a result. | A1, P3, P34 (twin: `the_stored_outcome_and_shared_copy_are_kept`; the relaunch itself has no automated twin) | |
| S-3.2 | **(rare) A stored Locked.** After S-2.14 (key locked, "Locked" shown), set `shared` to Every 60 minutes, quit and launch again. Look at the row before pressing anything. | "Locked" from the stored outcome, with **Check now** (not Unlock…, K-4) in the menu; pressing it asks for the passphrase (your press, so it may prompt). If the row shows nothing special write what it showed. | P3, P43 (twin: `each_state_and_its_precedence`) | |
| S-3.3 | **A stored outcome never outranks Checking….** From S-3.1's relaunch with the remote still stalled, press Retry now. | The row reads "Checking…" at once, and never "Can't reach" while the check runs. | P2 (twin: `each_state_and_its_precedence`) | |

## S-4 The repository menu

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-4.1 | Right-click the `shared` row (not the chosen one: choose `local` first). | A menu opens at the pointer for `shared`. **The choice does not change**: `local` keeps its chosen treatment and the Tickets list stays as it was. Its first line is `shared`'s state sentence, dimmed, followed by the state's action, then the fixed items. | A3, A10, P10, P22 (twin: `the_menu_by_pointer_and_keyboard`; a real right click has no automated twin) | |
| S-4.2 | Press Esc. | The menu closes; focus is where it was before the click. | A3, P10 | |
| S-4.3 | Press Ctrl 1, Down to a row, then **Shift F10**. Then Esc. Do it again with **the menu key** (if your keyboard has none, write "no key"). | The menu opens beside the highlighted row, for that row's repository, with Up, Down and Enter working in it. Esc closes it and focus is on the **list**. Both keys do the same. | A3, P10, P21 (twin: `the_menu_by_pointer_and_keyboard`; the real key has no automated twin) | |
| S-4.4 | Read the first line of the menu of every state you made in S-2 once more. | It is always the state's own sentence, dimmed, never chosen; in a state with **no label** (a busy opening, a non-SSH remote, anything not yet known) it still reads its sentence. Press Up from the first action: write whether the cursor lands on the sentence. | A3, P10 | |
| S-4.5 | Choose another repository by Enter, then move the cursor with arrows and open a menu and close it. Look at Activity and at the chosen row. | Opening a menu and moving the cursor save, check and send nothing: Activity gains no entry from them, and only Enter or a click moves the chosen treatment. | A10, A9 (twin: `the_menu_by_pointer_and_keyboard`, P22) | |
| S-4.6 | In the menu of `shared`, choose **Check now**. Then choose it while a check is running. | The first sends one check (Activity shows "Checking · …"; the footer says "Checking for updates…") and nothing is shared. The second does nothing: the item is dimmed while that repository is checking. Focus goes back to where the menu was opened. | A11, P23 (twin: `check_all_checks_each_repository`) | |
| S-4.7 | In the menu of a state whose Check now reads **Retry now** (S-2.7), choose it. | It checks again. In a state where Retry now has become Resume checking or Unlock… by the time you choose, nothing happens. | A11, P10 | |
| S-4.8 | In the menu of `shared` with a ticket saved and not shared, choose **Share main**. | Share main shares main only and reads "Sharing…" (dimmed) while it runs; the unshared ticket stays counted; "main" leaves the tooltip's names. With no remote (`local`) the item reads "Share main…" (with the dots); write what it opens. | P5, P10 | |
| S-4.9 | In the menu of `local`, choose **Open folder**. Then **Settings…** (on a state whose action is not Settings…). | Open folder opens the repository's folder in your file manager. Settings… opens Settings › Repositories with **that row's** repository selected, not the chosen one. | A3, P10 | |
| S-4.10 | Choose **Remove from Manyhands…** on a row that is not chosen. Read the dialog. Press Cancel. | A confirmation dialog: its preview carries "Local files remain: only the registration is removed. Nothing in the folder or in Git is changed." and "Drafts of this repository's items stay in Drafts." Cancel closes it and **focus is back where the menu was opened** (the list). The repository is still listed. | A7, P11 (twin: `the_menu_by_pointer_and_keyboard`) | |
| S-4.11 | Open the same dialog from the **More** menu of the toolbar for the chosen repository (S-5.3), press Cancel, then do it again and **Confirm** on `fixing` (it is a scratch repository). | After Cancel, focus is on the More button. After Confirm the row is gone, and `ls "$SCRATCH/repos/fixing"` still lists its files and `.git`. | A7, P11 | |
| S-4.12 | **(rare)** Open a menu, then trigger something that removes its repository (for example, the CLI's `repo remove` of the row you opened, then Ctrl R). | The menu closes by itself when its repository leaves the rows. | P10 (twin: `the_menu_by_pointer_and_keyboard`) | |
| S-4.13 | Open a dialog (for example New ticket…) and try Shift F10 and a right click on a row. Close it, open Settings (Ctrl ,) and try the same. | Nothing opens behind a dialog. Neither opens behind Settings, a sheet. | A8 (twin: P12 in the menu, toolbar and footer tests, each with a dialog and a sheet) | |

## S-5 The toolbar, by keyboard, with focus visible

Use no pointer in this section. Every stop must show a ring.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-5.1 | Press Ctrl 1, then Shift Tab until focus is on the header. Write down the order of the stops. | Three buttons, in this order left to right: **Add or create a repository…** (`plus`), **Check all for updates now** (`refresh-cw`), **More** (`ellipsis`). Each shows its ring; the Repositories region draws its ring while a button holds focus (write down which rings you see). | A3, A13 (twin: `panel_toolbars_work_by_tab_and_enter`; the ring is appearance) | |
| S-5.2 | Press Enter, then Space, on **Add or create a repository…**. Close what opens. | Settings opens on its Repositories page (until Cycle 05's sheet), both times. | A3, P24 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| S-5.3 | Press Enter on **More**. | A menu opens under the button: **Refresh**, **Rebuild the search index**, and (after a separator) **Remove from Manyhands…**, for the chosen repository; each is dimmed while no repository is chosen. Esc closes it and focus is **on the More button**. Choose Refresh: a job shows in Activity. | A3, P13 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| S-5.4 | With `shared` and `second` both set to Every 60 minutes and neither checking, press Enter on **Check all for updates now**. | One check per repository that can check, in list order: Activity shows a "Checking · …" entry for each (`shared`, and `second` even if it is paused; `local`, `plain` and the rest have nothing to check and show none). Nothing is shared. Every repository is then checking, so the button dims at once: if it held focus, focus moves to the list (write where it went). | A11, P23 (twin: `check_all_checks_each_repository`) | |
| S-5.5 | **A repository waiting for a key.** Quit and launch again so the key is locked; make `shared` read "Locked" (S-2.14). Un-pause `second` if it is paused, and wait until it too reads Locked. Press Check all. | **No passphrase dialog opens, one after another or at all.** Check all skips `shared` (waiting on its key): Activity shows no new check for it. It reaches its own row only through Unlock… in its menu. `second` waits on the same key, so the button has nothing to take and dims; S-5.4 is the control that Check all does check. | A11, P24 (owner ruling 2026-11-12; twin: `check_all_checks_each_repository`) | |
| S-5.6 | **A paused repository with a stored locked-key outcome.** Make `second` read Locked once (as `shared` did), then turn its switch off in Settings › Checking for updates so it reads Paused, quit, launch again (key locked) and look at `second`. Press Check all. | `second` reads "Paused" (a paused repository's stored outcome does not set its state), and Check all **skips** it too: no check starts for it and no prompt opens. A paused repository **without** a stored locked-key outcome is checked by Check all (S-5.4). | A11, P24 (owner ruling 2026-11-12; twin: `check_all_checks_each_repository`) | |
| S-5.7 | **Check all dims while it has focus.** Stall the remote, put focus on **Check all**, press Enter, and look at the button and at focus for the 30 s the checks last. | The button is dimmed while no repository qualifies, and focus has moved from it to the **list** (it does not stay on a dimmed button). Release the remote afterwards. | P24, R-22 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| S-5.8 | **(rare)** With a dialog open (New ticket…), press Ctrl N and click each toolbar button; then open Settings and try them. | Nothing acts behind the dialog: no second dialog, no check, no menu. | A8, P12 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| S-5.9 | On a toolbar button press **Shift F10**, then Esc. | The menu is the **chosen** repository's even when the cursor rests on another row, and Esc returns focus to **that button**. | P10, R-40 (twin: `the_menu_by_pointer_and_keyboard`) | |
| S-5.10 | Hover each button. | `plus`: "Add or create a repository…". `refresh-cw`: "Check all for updates now". `ellipsis`: "More". No key is shown beside them (Ctrl N does nothing from the Repositories toolbar). | A3, P24 (no automated twin for the tooltip) | |

## S-6 The footer's two items

The footer reads, from the left, the dock and panel toggles, then a spacer,
then **Update check**, **To share**, the Needs attention count when there is
one, and the right-hand toggles and Settings.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-6.1 | Before any save or completed check (a fresh scratch directory, or `local` alone), look at the footer. | Neither the Update check nor "N to share" is drawn (hidden, not "Checked … never"). | A13, P15 (twin: `status_items_say_and_choose`) | |
| S-6.2 | After a check completed (S-2.2), look at the footer against frame 1b. | At the right, before "need attention": `refresh-cw` in teal with "Checked for updates {ago}" (write the age), then, when something is not shared, `cloud-upload` in amber with "{n} to share". | A13, A2 (appearance: no automated twin) | |
| S-6.3 | Hover the Update check. | A tooltip: "Checked for updates {ago}", then the line "Background check: fetch and fast-forward only. Never shares your changes." | A3, P15 (pure-function twin: `status_items_say_and_choose`) | |
| S-6.4 | Press Tab until the Update check has focus, then press Enter, then Space. | A ring shows on it. Both open **Settings › Checking for updates**. | A3, P15 | |
| S-6.5 | Stall the remote and press Check now in `shared`. Look at the Update check, then release the remote and wait. | While a repository is checking it reads `refresh-cw` and "Checking for updates…"; after the check it reads "Checked for updates just now". Press Check all with two repositories able to check: it stays "Checking for updates…" until both end. | A2, P15 (twin: `status_items_say_and_choose`) | |
| S-6.6 | With work to share in **two** repositories (`shared` and `second`; K-1), choose `local`. Tab to **To share** and press Enter. Hover it first. | Tooltip and name: "{n} to share. Select {repo}." naming the next repository with work **after the chosen one, in list order**. Enter chooses it, **focus stays on the control**, and a hidden Repositories panel stays hidden (the layout never changes). Enter again chooses the next one, wrapping at the end. | A2, A3, P15 (twin: `status_items_say_and_choose`) | |
| S-6.7 | With work to share **only in the chosen repository**, press Enter on **To share**. | Nothing happens: no reload, no change of choice. The tooltip and the name read "{n} to share in {repo}." | A2, P15 (decision 23; twin: `status_items_say_and_choose`) | |
| S-6.8 | Share the last unshared item of the chosen repository (S-2.5) while the footer's **To share has focus**. **Watch.** | When the count reaches 0 the control hides and focus moves to the **next shown control to its right**: the Needs attention count if shown, else the first control of the right dock group, else Settings. This focus move is made while the footer is drawn and **has never been seen on a real display**: write what you saw, including a focus that vanished, a ring on nothing, or a second Tab press needed. | A3, P15, R-38 (twin: `status_items_say_and_choose`; the real display has none) | |
| S-6.9 | **(rare)** With two repositories sharing a display name (create `$SCRATCH/repos2/shared` too), look at To share's name. | It adds the parent folder: "{n} to share. Select shared in repos2." (or the parent's name). | P15 (twin: `status_items_say_and_choose`) | |
| S-6.10 | Open a dialog and try to press each footer item by keyboard and pointer. | Neither acts behind a dialog or a sheet. | A8, P12 | |

## S-7 The item area with no tab

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-7.1 | Look at the item area with no repository chosen and no tab open (as first seen at S-1.2, or after a launch where you have not chosen one yet; if a launch restores a choice, say so). | It reads "Select a repository." | A6, P14 (twin: `the_item_area_holds_nothing_of_the_repository`) | |
| S-7.2 | Choose a repository with no tab open. | It reads "Open a ticket or document from the lists", with no icon (Cycle 05 adds it). **Nothing of the old repository block** is there: no name, path, branch, configuration, index state or Share main button. | A6, P14 | |
| S-7.3 | Open a ticket tab and scroll it to the end. | Nothing about the repository follows the ticket's body: the block that used to end every tab is gone. Each thing it offered is somewhere now: the row's tooltip (folder, main branch), the menu (Retry, Share main, Remove from Manyhands…), the Tickets and Documents lists (index states). | A6, P14 (design Part F's table) | |
| S-7.4 | Close the tab, choose another repository, then another, and watch Activity. | The item area flips between the two texts as the choice changes; choosing sends nothing beyond the reads a choice makes today. | A10, P22 | |

## S-8 Tooltips

Hover with the pointer and wait about a second. A tooltip never shows from
keyboard focus (K-6). The tooltip's lines are the state's sentence, then
`Folder: {path}`, then `Main branch: {branch}`, then, when a shared copy is
selected, `Shared copy: {name} · {address}`. Write down the exact text you
read.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-8.1 | Hover `shared`. | Four lines (write them): the state's sentence; `Folder: $SCRATCH/repos/shared`; `Main branch: main`; `Shared copy: origin · ssh://…` with **no user name's password, key, passphrase or fingerprint**; the address is the library's redacted one. | A4, P7 (pure-function twin: `tooltips_say_the_state_and_the_facts`, `polling_privacy`) | |
| S-8.2 | Hover `local`, then a repository with no `main` yet known (a brand-new one just after Create repository). | `local`: no "Shared copy:" line. A repository whose main branch is not known yet has no "Main branch:" line. | A4, P7 | |
| S-8.3 | With six or more items not shared in `shared` (S-2.4), hover it. | At most **five** items named, then " and {k} more." (the sixth and later are not named); main appears as the word "main". | A2, P6 (K-11) | |
| S-8.4 | Compare each tooltip you read in S-2 with its menu's first line. | The same sentence. Nothing else shows a key, a passphrase, a fingerprint or a URL with a password. | A4, P7 | |
| S-8.5 | Hover the Update check and To share (S-6.3, S-6.6). | As quoted there. | A3, P15 | |

## S-9 Orca

Turn on Orca (Super Alt S, or your desktop's switch). Tests read accessible
names and states from the Kit's snapshots only; what a screen reader
announces is checked only here (K-6). Write down what you hear, word for
word.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| S-9.1 | Focus the panel (Ctrl 1) and move down the rows. | Orca reads each repository's name, its position in the list, and its description: the state's sentence, then "Folder: …", "Main branch: …" and the shared copy, with the same words as the tooltip. The state is also read as the label of its own element (for example "To share" with its count, "Up to date"). | A3, A12 (twin: names and descriptions asserted in `tooltips_say_the_state_and_the_facts`; the reading is not) | |
| S-9.2 | Move to the chosen row. | The row is read as chosen through its "Selected" mark and its description. **It is not announced as selected twice** (the Kit's cursor selects; the row itself sets no selected state). Write down what you hear. | A9, A12 (no automated twin for the reading) | |
| S-9.3 | Press Shift F10 on a row. | Orca announces the menu; the first item is the state's sentence, read as text that is not available to choose; Down reads each item's name and whether it is dimmed. | A3 (no automated twin for the reading) | |
| S-9.4 | Tab to each toolbar button and the footer's two items. | "Add or create a repository…", "Check all for updates now" and "More", as buttons; the footer's Update check reads its text, then **write down whether the tooltip's line "Background check: fetch and fast-forward only. Never shares your changes." is read with it** (as its description or not); "To share" reads "{n} to share. Select {repo}." as a button. | A3, A12 (no automated twin for the reading) | |
| S-9.5 | Move to the empty item area's text and to a state with **no label** (S-2.8). | The item area's text is read; the row with no label still reads its sentence as its description. | A3, A12 | |
| S-9.6 | **(rare)** Move to a dimmed menu item (Share main on `away`, or Check now while checking). | It is announced as unavailable; write how. | A3 | |

## Results

Write the build (commit), display server, scale factor, and one line per
section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| S-1 | Launch, an empty panel, the first look (A1, A5, A9, A12, A13; P8) | |
| S-2 | Each state, one row each (A1, A2, A6, A12; P1 to P7, P18 to P20, P30, P38) | |
| S-3 | A relaunch with a stored outcome (A1; P2, P3, P34, P43) | |
| S-4 | The repository menu (A3, A7 to A11; P10 to P13, P21, P22) | |
| S-5 | The toolbar, by keyboard, with focus visible (A3, A8, A11; P12, P13, P23, P24) | |
| S-6 | The footer's two items (A2, A3, A8, A13; P5, P12, P15) | |
| S-7 | The item area with no tab (A6, A10; P14, P22) | |
| S-8 | Tooltips (A2, A4; P6, P7; K-6, K-11) | |
| S-9 | Orca (A3, A9, A12; K-6) | |
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
