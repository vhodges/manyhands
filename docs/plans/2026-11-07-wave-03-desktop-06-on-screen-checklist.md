# D6 On-Screen Checklist

This list is for checking the D6 Cycle (desktop background polling and
shutdown, with the retirement of completed contexts) by hand on Linux
Wayland, before the Cycle is called review-ready. It is the native evidence
the design asks for (design, "Native Checklist (Linux Wayland, Release
Build, Product Owner)"; Cycle acceptance rows A1, A5, A7, A9, A15, A19, A21,
A22 and A23), with the owner rulings that changed the behaviour since:
Document Amendment 11 (Task 11b: decision 28 covers any running retirement;
an unreadable draft entry is unknown; a failed Check now stops the loop; a
successful Check now is the latest attempt and may start or replace a loop)
and Amendment 12 (the in-flight entry's exits). It also carries what the
execution ledger put on the screen because no automated test can reach it:
U-9's observer row, K-29, K-60, K-17 and the Activity layout change. Work
through sections 1 to 7 in order. Every journey is keyboard-only: use no
pointer unless a step says so. Write each step's result in its row's last
column, and a one-line summary per section in section 8. Claude records only
what you report.

Quoted text is the app's own wording. `{repository}` stands for a
repository's name (its directory's name), `{remote}` for the remote's name
and `{id}` for an item's ID. `{last8}` stands for the **last eight
characters** of an item's ID: D6's rows name an item by that, never by its
title (K-46). `secondary` is Ctrl on Linux.

**What a step may not see.** No D6 text names a title, a body, a comment, a
path or a key. If one does, write it down as a failure. An item is only ever
its `{last8}`.

## Before you start

- Start the app from the ticket worktree
  (`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN3VPVVQ3RNWJKXWEVX`)
  with a release build:
  `devenv shell -- cargo run --release --locked --features desktop --bin manyhands`
  It needs an active desktop display. Run it while no long test suite or
  build is running on the machine. The window opens at 1280 x 840 with
  Activity at the foot, 150 px tall (the "launched size").
- **Use a scratch data directory.** D6's app polls and retires on its own,
  and this build migrates the registry one way, so keep your real data
  directory out of it. The desktop honours only `XDG_DATA_HOME`; the CLI
  honours `MANYHANDS_DATA_DIR`:

  ```sh
  export SCRATCH=$(mktemp -d)
  mkdir "$SCRATCH/repos"
  # the app:
  devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands
  # the CLI, same data (the data directory is "$SCRATCH/manyhands"):
  devenv shell -- env MANYHANDS_DATA_DIR="$SCRATCH/manyhands" cargo run --locked --bin manyhands-cli -- …
  ```

  Drafts are `"$SCRATCH/manyhands/drafts/{id}.json"`. Every launch below is
  this command. Shared SSH keys live in `~/.ssh/manyhands`, outside the
  scratch directory; that is expected.
- In the keys below, `secondary-4` is the operation area, called
  **Activity**. `secondary-,` opens Settings, `secondary-shift-d` opens
  Drafts, `secondary-r` is Refresh, `secondary-q` quits. Keep Activity
  visible all the time (K-13).
- Steps marked **(rare)** need a state that is hard to make by hand. Try
  each one; if you cannot reach it, write "skipped". Each is also covered by
  an automated test, **except** section 5's U-9 steps (5.14 to 5.20), which
  nothing else covers: do those.
- Where a row says "(wording not checked)", the words come from the library
  or from a page I did not read, so write down what you saw.

### Two scratch repositories

Set them up as D5's checklist does (`docs/plans/2026-11-03-wave-03-desktop-05-on-screen-checklist.md`,
"Two scratch repositories"); the short form:

- **Local scratch.** Settings (`secondary-,`) › Repositories › **Create
  repository**, with no remote. It stands for a repository that is never
  polled.
- **Shared scratch**, in `$SCRATCH/repos/shared`, called `$REPO` below:
  1. Create or import a passphrase-protected key and **select it before**
     configuring the remote (Settings › Shared SSH keys) (D4 K-4).
  2. Make a bare repository on an SSH host you control
     (`git init --bare scratch.git` on the host). Add it as the remote and
     select it for publication (Settings › Remotes).
  3. Commit the changed Manyhands configuration file (D4 K-2), then
     `git push {remote} main`. Approve the host when first asked.

### The second clone (the other person)

```sh
git clone {user}@{host}:scratch.git "$SCRATCH/clone2"     # once; $CLONE2 below
cd "$CLONE2"
git config user.name Peer && git config user.email peer@example.com
```

It is changed with plain `git` only. Paste these helpers into the shell you
use for it:

```sh
# Another person edits a ticket the app has shared: one line on its branch.
peer_edit() {   # $1 = ticket ID, $2 = text
  git fetch -q origin
  git switch -q --detach origin/manyhands/ticket/$1
  echo "$2" >> .manyhands/tickets/$1/ticket.md
  git commit -qam "peer edit $1" && git push -q origin HEAD:manyhands/ticket/$1
}

# Another person shares a brand-new ticket on a context branch.
peer_new_ticket() {   # $1 = new ID (from `manyhands-cli id new`), $2 = title
  git fetch -q origin
  git switch -q --detach origin/main
  mkdir -p .manyhands/tickets/$1
  printf -- '---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: %s\ntitle: %s\ntype: task\nstatus: open\n---\nBody of %s.\n' "$1" "$2" "$2" > .manyhands/tickets/$1/ticket.md
  git add .manyhands/tickets/$1 && git commit -qm "peer ticket $2"
  git push -q origin HEAD:manyhands/ticket/$1
}

# Another person closes a ticket and deletes its branch: the merge into
# main carries what the app shared; the two closing fields are what the
# library reads as closed; main is pushed; the shared branch is deleted.
close_ticket() {   # $1 = ticket ID
  git fetch -q origin && git switch -q main && git pull -q --ff-only origin main
  git merge -q --no-ff -m "Merge $1" origin/manyhands/ticket/$1
  sed -i "s/^status: .*/&\nclosed_at: $(date -u +%Y-%m-%dT%H:%M:%SZ)\nclosed_by: Peer <peer@example.com>/" .manyhands/tickets/$1/ticket.md
  git commit -qam "Close $1" && git push -q origin main
  git push -q origin --delete manyhands/ticket/$1
}
```

`close_ticket` is plain `git`, so it needs no key unlock and registers
nothing. The alternative is `manyhands-cli ticket close --repo "$CLONE2" --id
{id}` (it asks you to confirm at the terminal, and needs `$CLONE2`
registered with its own data directory and an unlocked key); use it only if
`close_ticket` fails.

### Seven tickets, made ready once

All seven must be shared from the app before any step needs them: the branch
and working copy must exist locally and on the remote, or there is nothing to
retire. For each of `T1` to `T7` (the checklist names them by purpose below):

1. Get an ID: `manyhands-cli id new` (the CLI line above). In `$REPO` write
   `.manyhands/tickets/{id}/ticket.md` as D5's "Making a ticket with plain
   `git`" does (front matter with `manyhands_managed: true`,
   `manyhands_kind: ticket`, `id`, `title`, `type: task`, `status: open`),
   `git add` and `git commit` it, and `git push {remote} main`.
2. In the app press `secondary-r`, open it (`secondary-p`, its ID, Enter),
   **Edit**, change its body, **Save** (`secondary-s`), then **Share**
   (`secondary-3`, Tab along the header). The first Share asks for the
   passphrase ("Unlock key").
3. Write down each ID and its `{last8}`:

| Name | Used for | ID | `{last8}` |
| --- | --- | --- | --- |
| T1 | the app's own retirement (5.1) | | |
| T2 | the journey's edited ticket (2.7, 2.9, 2.10), then the open-edit ticket (5.5 to 5.8) | | |
| T3 | a kept draft, warned and retired (5.9 to 5.12) | | |
| T4 | a locked key and Check now (4.5 to 4.9) | | |
| T5 | the unreadable draft (5.13) | | |
| T6 | U-9: stalled Retire…, quit, Resume (5.14 to 5.20) | | |
| T7 | the stall's Save (3.3) and spare | | |

4. **Kept drafts for T3 and T6.** Open T3, Edit, type a line in the body and
   do **not** Save. Do the same on T6. Quit with `secondary-q` ("Saving drafts
   for recovery" may flash). Start the app again; `secondary-shift-d` lists
   both drafts. The app is now freshly launched, with the key locked.

### Stalling the remote

Polls and retirements open a new connection each time, so stall the host's
**listening** `sshd` (the parent, not a per-connection child; D5 used the
same for connections made afterwards): the connection is accepted by the
kernel and nothing ever answers. On the host:

```sh
pgrep -a sshd                 # the listener is the parent of the others (-D, or "[listener]")
sudo kill -STOP <listener>    # stall
sudo kill -CONT <listener>    # release
```

Have both commands ready in a terminal on the host. Stopping the `sshd` that
serves an already open connection (D5's "Stalling the remote") also works for
a call already in flight; the listener covers new ones.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure.
"Design" numbers are the design's; "ledger" numbers are the execution
ledger's.

- **Design K-1.** A Save or Share submitted during a stalled poll waits for
  the poll's safe point, up to the transport timeout (about 30 s).
- **Design K-12.** Until a context is retired, the item's own copy is what a
  tab reads, so a ticket closed elsewhere still reads open until the poll
  after which it is retired.
- **Design K-13.** With Activity hidden, a quit that waits shows nothing
  until Activity is opened.
- **Design K-14.** A Retire… offer comes from the last attempt and can be
  stale; the library proves everything again and may answer
  `already_absent`.
- **Design K-15.** Each retired item costs its own observation, so a loop
  over several items takes several connections.
- **Design K-17.** A Save or Share submitted while a retirement runs waits
  for that one retirement, up to one observation and its local removal.
- **Design K-18.** A graceful quit lets the running retirement finish and
  drops the queued ones; **Quit now** leaves it pending, and its pending row
  offers Resume at the next launch. The repository's polls wait until it is
  settled.
- **Design K-19.** An item being edited or with kept drafts is not retired
  until you press Retire….
- **Design K-16 (reworded by the Activity change, ledger Task 11b).** The
  three older unwrapped Activity rows (an unsaved draft's entry, a
  repository read failure, a pending row with several controls: the Wave 04
  Cycle 01 checklist's W-10.1 to W-10.3) now clip or overflow inside their
  column instead of widening Activity. D6's own rows wrap.
- **Design K-22.** A poll killed by Quit now or a crash, in a repository that
  is not eligible, keeps its row until it is eligible again; the row says why.
- **Design K-23.** With two protected keys, a job using one evicts the
  other's passphrase.
- **Design K-27.** A pending retirement with no in-flight entry (one a second
  instance submitted) offers Abandon but not Resume, unless it owes only its
  index hand-off.
- **Ledger K-29.** Resume can still be offered on a row the desktop cannot
  continue (another process's, or the U-9 observer row).
- **Ledger K-33.** A Check now's failure cannot tell host-verification or
  protocol failures from other causes; no attention entry then.
- **Ledger K-46.** `{item}` in D6's rows is `{last8}`, never the ticket's
  short code.
- **Ledger K-47, K-52.** A hung draft read parks the repository's polls and
  a Retire… press until a Check now, a reconfiguration or a quit.
- **Ledger K-59.** An in-flight entry that never learned its operation
  keeps its item "Retiring…" across relaunches (rare).
- **Ledger K-61.** A discarded reconcile read is retried only at the next
  window activation.

## 1. Smoke

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 1.1 | Start the app with the scratch command above. | A window opens with no error in the terminal. | |
| 1.2 | Press `secondary-,`, and Tab through the page list. | Five pages: "Repositories", "Remotes", "Shared SSH keys", "Hosts", "Checking for updates". | |
| 1.3 | Open "Checking for updates". | The heading "Checking for updates" and the intro "Manyhands checks your shared copies for other people's changes and brings them in when it's safe. It never shares anything. It also removes this computer's copies of tickets closed and documents promoted elsewhere, unless you have unsaved or kept work in them." Then one row per repository. | |
| 1.4 | Quit (`secondary-q`). | The window closes with no error in the terminal. | |

## 2. The page, and a real SSH journey

Start the app. The key is locked: Settings › Checking for updates, press
**Check now** on the Shared scratch row, and unlock at the prompt (4.5 later
needs it locked again).

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 2.1 | Read the Local scratch row. | The status "No shared copy, nothing to check" and a **Set up sharing…** button. No interval, no Check now, no switch. | |
| 2.2 | Read the Shared scratch row (name, status, controls), by Tab. | Its name, a status line, then **How often** (a select), **Check now** and a switch labelled **Check automatically**, which is on. | |
| 2.3 | Press Check now. Watch the status and Activity. | While it runs the status reads "Checking…", and the Check now button is unavailable (and cannot be reached by Tab). Activity's entry reads "Checking · …" with stage words (for example "Connecting to {remote}…", "Bringing in changes…"). It ends "Checked just now · up to date". | |
| 2.4 | Open **How often**. | Six choices: "Every 1 minute", "Every 5 minutes", "Every 10 minutes", "Every 15 minutes", "Every 30 minutes", "Every 60 minutes". Choose "Every 1 minute" with the keyboard. | |
| 2.5 | Quit, start again, open the page. | The interval still reads "Every 1 minute" (the library stored it). | |
| 2.6 | Turn **Check automatically** off, quit, start again, open the page. | The status reads "Paused"; the switch is off after the restart (the library stored it). Turn it on again. | |
| 2.7 | **The journey: a change.** Open T2 in the app (a clean tab, no edit). In `$CLONE2` run `peer_edit {T2 id} "A line from the other clone."`. Do not touch the app. | Within the interval (about a minute) the status reads "Checking…", then "Checked just now · Brought in 1 change". Activity, under "This session", shows an entry for `{repository}` with "Brought in 1 change". The open tab shows the line "A line from the other clone." (if it does not, press `secondary-r` and write that down). | |
| 2.8 | **The journey: a new context.** In `$CLONE2` run `peer_new_ticket {new id} "From the other clone"` (an ID from `manyhands-cli id new`). | Within the interval the Tickets list gains "From the other clone". Its tab opens with "Body of From the other clone." The Activity entry's line for it is the library's (the context was materialized; "The remote context was materialized locally." is the library's text, wording not checked). | |
| 2.9 | **Never shares.** In the app, Edit T2, add a word, and **Save** but do **not** Share. Note `git ls-remote origin refs/heads/manyhands/ticket/{T2 id}` in `$CLONE2`. Wait for the next poll. Run it again. | The remote's line is unchanged: the poll pushed nothing (and nothing was merged or checkpointed). The status may add "· 1 not shared yet". Now press **Share**, so the saved line is on the remote: a ticket whose work is only local is refused when it is retired later. | |
| 2.10 | **A dirty tab is kept.** Edit T2 again and type a line, unsaved. In `$CLONE2` run `peer_edit {T2 id} "Another line."`. Wait for the next poll. | The tab keeps what you typed. Activity's entry for the repository lists T2 with the library's guidance for a dirty context (wording not checked). Nothing you typed is lost. Discard the edit afterwards (the tab's own discard control, wording not checked). | |
| 2.11 | **Retry now.** Stall the remote. Wait for the next poll to fail (up to the transport timeout, about 30 s). Release it. | The status reads "Can't reach {repository}. Your work is safe." and the button reads **Retry now** in place of Check now. Press it after releasing: "Checked just now · …". | |
| 2.12 | Make sure the last states agree: read the Shared scratch row. | A fresh status, no leftover "Checking…". | |

## 3. Stall

Keep the interval at "Every 1 minute". Have the stall and release commands
ready.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 3.1 | Stall the remote. Wait for the next automatic poll to start. | The status reads "Checking…" (and stays so until the transport gives up, about 30 s). The window stays responsive: open a tab, Tab around, scroll Activity, switch Settings pages, type in an editor. Nothing freezes. | |
| 3.2 | While it is stalled, open the Settings page and press **Check now**. | Check now is accepted at once; Activity's entry reads "Queued", then "Checking · …" when the stalled poll gives way (a user job cancels the poll at its safe point, which can take up to the timeout: K-1). The button is unavailable while this Check now runs. | |
| 3.3 | While a poll is stalled, Edit T7, change a line and press **Save**. | The Save is accepted: Activity reads "Queued", then runs once the poll has stopped (up to about 30 s, K-1). The window is responsive throughout. | |
| 3.4 | Stalled again with a poll running, press `secondary-q`. Watch Activity. | At the top of Activity: "Finishing a check for updates" and a focused **Quit now**. Nothing new starts. After ten seconds the words change to "Still stopping — waiting for Manyhands to reach a safe point. Your work is saved." The window is still interactive. | |
| 3.5 | Wait without pressing Quit now (up to the timeout). | The window closes when the stalled call gives way. Release the remote afterwards. | |
| 3.6 | After 3.5, at once: `pgrep -a manyhands; ps -eLo pid,tid,comm \| grep manyhands-worker` and `pgrep -af ssh`. | No `manyhands` process, no `manyhands-worker` thread, no leftover child. (A8's graceful exit joined the worker.) | |
| 3.7 | **(rare, K-13)** Repeat 3.4 with Activity hidden (`secondary-j`). | Nothing shows and the layout does not change; the window waits. `secondary-4` shows Activity with the words. Quit now ends it. | |

## 4. Lock and host

Start the app fresh. The key is locked (no step above unlocked it). The
interval is "Every 1 minute".

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 4.1 | Do nothing for two minutes. | **No passphrase dialog ever opens by itself.** The Shared scratch status reads "Needs unlocking". Activity, at the top of "This session", has the entry "Checking is waiting for your key to be unlocked" with "Holding back {repository}" and **Unlock…**. The footer reads "1 needs attention". The Settings row also has **Unlock…**. | |
| 4.2 | Turn **Check automatically** off. | The status reads "Paused · Needs unlocking". The Activity entry stays. Pausing did not unlock anything. | |
| 4.3 | Press **Unlock…** in Activity, then **Not now**. | The "Unlock key" dialog (passphrase input, **Unlock**, **Not now**) opens. After Not now the entry stays, and no second dialog opens on its own. | |
| 4.4 | Press Unlock… again, enter the passphrase, **Unlock**. | The entry and the footer count go. The status reads "Paused" (a repository that is paused stays paused; unlocking did not resume it). No "Checking…" starts at the next due time. Turn the switch on: the status reads "Checking…", then "Checked just now · …", with no second prompt. | |
| 4.5 | **A locked key and a closed ticket (T4).** Quit and start again (locked). Make T4 closed: in `$CLONE2` run `close_ticket {T4 id}`. Press **Check now** in Settings. | A prompt opens (Check now is your action, so it may prompt): "Unlock key", with no line "Sharing {repository} is waiting for your passphrase." Choose **Not now**. | |
| 4.6 | Read the result of 4.5. | The Check now fails and **the loop stops**: nothing is retired, T4's tab still reads open, `git -C "$REPO" branch --list 'manyhands/ticket/*'` still lists T4's branch. The status reads "Needs unlocking" and the Activity entry of 4.1 is back. | |
| 4.7 | Press **Unlock…** in Activity, enter the passphrase. | One prompt. The unlock runs a Check now: the status reads "Checking…", then "Removing finished copies…" (no second prompt) and Activity's entry for the repository reads "Retired the copy of {last8} on this computer." The attention entry and footer count go. | |
| 4.8 | Look at T4's tab and the repository. | T4's tab reads Closed (pill "Closed", with "Closed at" and "Closed by"); `git branch --list 'manyhands/ticket/*'` no longer lists it, and `ls "$REPO/.manyhands/worktrees"` no longer has its folder. | |
| 4.9 | **(rare) Host.** Remove the host's approval in Settings › Hosts if the page offers it (wording not checked) and wait for a poll. | The status reads "Needs host approval". Activity: "Checking needs a host to be approved", "Holding back {repository}", **Approve host…**. It opens the Hosts page; after approving, a Check now follows. No dialog opened by itself. | |

## 5. Retire on two clones

The key is unlocked at the start of each step that needs it (Settings ›
Checking for updates › Check now, then the passphrase). Interval "Every 1
minute". Watch Activity's entry for the repository ("This session").

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 5.1 | **The app's own retirement.** Open T1's tab (no edit). In `$CLONE2` run `close_ticket {T1 id}`. Do not touch the app. Before the next poll, read T1's tab. | **K-12:** the tab still reads open (it shows its own copy). | |
| 5.2 | Wait for the next automatic poll. | The status reads "Checking…", then "Removing finished copies…", then "Checked just now · …". **No passphrase dialog opens.** Activity's entry reads "Retiring {last8}…" and then "Retired the copy of {last8} on this computer." | |
| 5.3 | Read T1's tab; run `git -C "$REPO" branch --list 'manyhands/ticket/*'` and `ls "$REPO/.manyhands/worktrees"`. | The tab reads Closed ("Closed at", "Closed by"); the Tickets list shows it Closed. T1's branch and folder are gone. Main's files are untouched. | |
| 5.4 | **The retirement is per item and bounded.** Look at the poll's attempt entry once more. | One line per retired item; no title, body or path anywhere in Activity or Settings. | |
| 5.5 | **An edit blocks the retirement (the unkept ask).** First `chmod -w "$SCRATCH/manyhands/drafts"`. Open T2, **Edit**, type a line, no Save. In `$CLONE2` run `close_ticket {T2 id}`. Wait for the poll. | T2 is **not** retired. Activity's entry reads "{last8} is closed. Its copy on this computer was not retired." with **Retire…**. (An entry for the unsaved draft may also show "could not be saved"; wording not checked.) | |
| 5.6 | Press `secondary-4`, Tab to **Retire…**, Enter. | The dialog "Retire local copy": "1 tab has unsaved changes that are not kept as a draft, so this item is not retired. Keep or discard them first." with **Show tab** (the default) and **Cancel**. There is no "Retire anyway". Nothing was submitted. | |
| 5.7 | Press Enter (Show tab). | The dialog closes and T2's tab with the typed text is shown. | |
| 5.8 | `chmod +w "$SCRATCH/manyhands/drafts"` and wait about five seconds (the draft is then kept). Press Retire… again. | The other ask: "This ticket has 1 kept draft. After it is retired you can still export them, but not restore them into it." with **Cancel** (the default) and **Retire**. Press Escape (Cancel): nothing changes. Then discard the edit in the tab (its own control, wording not checked) and press Retire… once more: no ask; T2 reads "Retiring…", then is retired ("Retired the copy of {last8} on this computer."). | |
| 5.9 | **Kept draft, the warning.** T3 has a kept draft and no tab open. In `$CLONE2` run `close_ticket {T3 id}`. Wait for the poll. | T3 is **not** retired (the app never retires an item with a kept draft). The entry reads "{last8} is closed. Its copy on this computer was not retired." with **Retire…**. | |
| 5.10 | Retire…: Tab to it, Enter. In the dialog press Enter at once. | The kept-draft dialog (as in 5.8). Enter is **Cancel**: nothing is retired and the offer stays. | |
| 5.11 | Press Retire… again, Tab to **Retire**, Enter. | T3's tab reads "Retiring…" (and is read-only), Activity's entry "Retiring {last8}…", then "Retired the copy of {last8} on this computer." | |
| 5.12 | `secondary-shift-d`. | The draft is still listed. Its row says "The ticket is closed" and offers **Export…** but no usable Restore. Export… writes the draft's text to the file you choose (open it and check the text). The branch and folder are gone. | |
| 5.13 | **An unreadable draft blocks retirement (T5).** Make T5 closed: `close_ticket {T5 id}` in `$CLONE2`. In the shell: `chmod 000 "$SCRATCH"/manyhands/drafts/*.json` (T6's draft is there). Press Check now. | T5 is **not** retired, though nothing else holds it: an unreadable draft counts as unknown, so no item of any repository is retired. The entry offers **Retire…** for it. Drafts (`secondary-shift-d`) lists "This draft cannot be read" for the files. Press Retire… and read the dialog: "This ticket may have kept drafts. After it is retired you can still export them, but not restore them into it." Cancel. Now **restore** the files: `chmod 600 "$SCRATCH"/manyhands/drafts/*.json`, and press Check now again: "Removing finished copies…", then "Retired the copy of {last8} on this computer." (Running as root ignores chmod: write "skipped".) | |
| 5.14 | **U-9: set up.** Make T6 closed: `close_ticket {T6 id}`. Press Check now. | T6 is not retired (kept draft). The entry offers **Retire…**. | |
| 5.15 | Stall the remote (the listener). Press Retire… and, at the kept-draft dialog, press **Retire**. | T6's tab reads "Retiring…" and cannot be edited. Activity's request reads "Retiring · …" with stage words (for example "Connecting to {remote}…"). It is stalled. | |
| 5.16 | **K-17.** While it is stalled, Edit T7 and press **Save**. | The Save is accepted but waits: Activity reads "Queued" for the Save until the retirement ends (up to about 30 s). The window stays responsive. | |
| 5.17 | **K-17 and K-18: quit during the retirement.** Press `secondary-q`. | Activity reads "Finishing Retire local copy" with a focused **Quit now**. The quit **waits** for the retirement: it is not cancelled. After ten seconds: "Still stopping — waiting for Manyhands to reach a safe point. Your work is saved." Press Quit now: the window closes at once. Then release the remote. | |
| 5.18 | Start the app again. Press `secondary-4`. Open T6's tab. | T6's tab reads "Retiring…". Under "Pending in repositories" a row for the repository's retirement (its action and state are the library's, wording not checked) with **Resume** and **Abandon**. The Settings status reads "Waiting for an unfinished operation" with **Show in Activity**. | |
| 5.19 | **K-60.** Switch to another window and back (a window activation). Look again. | T6 **still** reads "Retiring…" and the pending row still has Resume. (If either is gone before you pressed Resume, the entry lost its hold at activation: write that down; it is K-60 failing.) | |
| 5.20 | **U-9 / K-29: Resume.** Press **Resume** on the row. Unlock at the prompt ("Unlock key") if it opens. | The pending row goes. T6's tab stops reading "Retiring…" and reads Closed. Activity's entry reads "Retired the copy of {last8} on this computer." (or "Finished retiring {last8}." if the row owed only its hand-off: write down which). The branch and folder are gone; the draft is on the Drafts page. The repository's status goes back to "Checked …" and its polls resume. If Resume is missing, or fails, or the row stays: write down exactly what the row and status read (K-29, K-59). | |
| 5.21 | **(rare) Abandon.** Make another retirement pending as in 5.14 to 5.17 and press **Abandon**. | A confirmation with the retirement's details (library text, wording not checked). After confirming, the row goes and the item stops reading "Retiring…". | |
| 5.22 | **(rare) Check now replaces a loop.** With three tickets closed by `close_ticket`, wait for "Removing finished copies…" and press Check now at once. | No error; each ticket is retired exactly once ("Retired the copy of {last8} on this computer." for each); Check now's own attempt takes over the rest; the status ends "Checked just now · …". | |
| 5.23 | **(rare) A promoted document.** In `$CLONE2`, promote a document the app has edited and shared (`manyhands-cli document promote`, with `$CLONE2` registered in its own data directory; wording of that flow not checked), leave a kept draft on it, wait for the poll. | Like T3, except that Retire… reads: "You can still restore or export the document's drafts. Drafts of comments can only be exported." | |

## 6. Restart and shutdown

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 6.1 | **The stored deadline.** Set "Every 5 minutes". Press Check now and note the time of "Checked just now · …". After about a minute, quit and start again at once. Watch the status without touching anything. | At launch **no** "Checking…" starts: the status reads "Checked 1 minute ago · …" (or 2). The next automatic poll starts about five minutes after the first check, not at launch: "Checking…" then "Checked just now · …". | |
| 6.2 | **L-4: quit now during a Check now.** Stall the remote. Press Check now. When Activity reads "Checking · Connecting to {remote}…", press `secondary-q`. | Activity reads "Finishing Check now" with a focused **Quit now**; after ten seconds "Still stopping — waiting for Manyhands to reach a safe point. Your work is saved." Press Quit now. Release the remote. | |
| 6.3 | Start the app again. Press `secondary-4`. | The dead Check now's row is under "Pending in repositories" with **Resume**. The status reads "Waiting for an unfinished operation" with **Show in Activity**. | |
| 6.4 | Press **Resume**. | The row goes; a Check now runs ("Checking · …"), then "Checked just now · …". If Resume is not offered or fails (design risk U-15: the CLI recovers it), write down what the row and status read. | |
| 6.5 | **Nothing new starts once quitting is asked (R-33).** `chmod -w "$SCRATCH/manyhands/drafts"`, Edit a ticket, type a line, press `secondary-q`. Leave the dialog open for about 90 seconds (interval "Every 1 minute"). | The dialog is the unkept ask ("1 tab has unsaved changes that are not kept as a draft"). Press **Cancel**: the status reads "Checked 1 minute ago" or older, never "just now" (no poll ran while the ask was open); a poll starts shortly after. Restore `chmod +w`. | |
| 6.6 | **Graceful quit, idle.** With nothing running, press `secondary-q`; then `pgrep -a manyhands; ps -eLo pid,tid,comm \| grep manyhands-worker; pgrep -af ssh`. | The window closes. No `manyhands` process, no `manyhands-worker` thread, no child process remains. (The stalled-poll exit of 3.6 is the other path.) | |

## 7. Keyboard only, and Activity at the launched size

Use no pointer at all in the keyboard rows. Start at the launched size:
1280 x 840, Activity 150 px tall.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 7.1 | `secondary-,`, Tab to the "Checking for updates" page, Tab along the Shared scratch row. | Every stop shows a focus ring and scrolls into view: **How often**, **Check now**, **Check automatically**, then any **Show in Activity**. | |
| 7.2 | Open the interval select, choose another with the arrows and Enter. | It changes, and the status keeps showing; focus stays on the select. | |
| 7.3 | Press Space or Enter on the switch, twice. | "Paused", then back on; the focus ring stays on the switch. | |
| 7.4 | Press Enter on **Check now**. | One Check now runs (the button becomes unavailable and is no longer a Tab stop while it runs). | |
| 7.5 | `secondary-4`, Tab to **Unlock…** (make one: relaunch locked and wait) and **Retire…**, press Enter on each. | Each is a stop with a ring, scrolled into view in the 150 px panel. Enter on Unlock… opens "Unlock key"; Enter on Retire… sends one request (or its dialog, see 5.6 and 5.10). | |
| 7.6 | In the Retire… dialogs (5.6, 5.10): Tab, Shift-Tab, Enter, Escape. | Show tab is the unkept ask's default (Enter), Cancel is the kept ask's default (Enter); Escape is Cancel in both; Tab stays inside the dialog; focus returns to **Retire…** when it closes without retiring. | |
| 7.7 | `secondary-q` during a stalled poll (3.4): Tab. | Focus is on **Quit now** (ring); Enter presses it; the window closes. | |
| 7.8 | **K-16 / the Activity change.** At the launched size with Activity 150 px tall, make these show together: an attention entry (4.1), an attempt entry with a retirement line and a **Retire…** offer (5.9), a pending retirement row (5.18). | Activity's two columns ("This session" and "Pending in repositories") stay inside the panel: **no horizontal scrolling**, no control outside what you can see. D6's own rows (the attention entry, the attempt entry's lines, the Retire… offer, the retirement's line) wrap inside their column; Unlock…, Retire…, Resume and Abandon are all reachable and visible (the operation area scrolls vertically). | |
| 7.9 | **(rare) The three older rows.** Make the three Wave 04 W-10 rows (an unsaved-draft entry with Retry: `chmod -w "$SCRATCH/manyhands/drafts"` while you edit; a repository read failure; a pending row with several controls). | Per the reworded design K-16, a control after long text may now clip inside its column instead of widening Activity. Write down which of the three clip. | |

## 8. Results

Write the build (commit), display server, scale factor, and one line per
section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| 1 | Smoke | |
| 2 | The page (interval, switch, Check now, status words) and the real SSH journey (change, new context, never shares, dirty tab, Retry now) (A1) | |
| 3 | Stall (responsive, Queued, still stopping at ten seconds, process list after a graceful quit) (A5, A9; K-1, K-13) | |
| 4 | Lock and host (no prompt, Unlock…, pause and unlock separate, a failed Check now stops the loop, Check now may start a loop) (A3, A13; K-23) | |
| 5 | Retire on two clones (the app's own retirement, the two asks, an unreadable draft, Drafts and Export…; U-9, K-17, K-18, K-29, K-60) (A19, A21, A22) | |
| — | U-9 observer row: quit during a retirement, relaunch, Resume (5.15 to 5.20): finished, or what the row read | |
| 6 | Restart and shutdown (the stored deadline, L-4 Resume, nothing new once quitting, no worker left) (A7, A9, A23) | |
| 7 | Keyboard only (A15), and Activity at the launched size (K-16) | |
| — | Known limits hit (numbers, steps) | |
| — | Wording I could not find (hosts page, pending row, a dirty context's guidance, materialized) | |
