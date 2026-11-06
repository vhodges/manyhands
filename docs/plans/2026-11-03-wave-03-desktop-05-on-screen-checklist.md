# D5 On-Screen Checklist

This list is for checking the D5 Cycle (conflict recovery, promotion and
closure) by hand on Linux Wayland, before the Cycle is called
review-ready. It is the native evidence the design asks for (design,
"Native Checklist (Linux Wayland, Release Build, Product Owner)"), with
the owner rulings that changed the UI since: R-27 to R-32. Work through
sections 1 to 6 in order. Every journey is keyboard-only: use no pointer
unless a step says so. Write each step's result in its row's last column,
and a one-line summary per section in section 7. Claude records only what
you report.

Quoted text is the app's own wording. `{remote}` stands for the remote's
name, `main` for the repository's main branch, `{id}` for an item's ID,
and `{short}` for a ticket's short code (or an item's last eight ID
characters when it has none).

## Before you start

- Start the app from the ticket worktree
  (`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN2K8DBHNMZ46NTX5JJ`)
  with a release build:
  `devenv shell -- cargo run --release --locked --features desktop --bin manyhands`
  It needs an active desktop display. Run it while no long test suite or
  build is running on the machine.
- In the keys below, `secondary` is Ctrl on Linux. `secondary-4` is the
  operation area, called **Activity** in the dialogs.
- Steps marked **(rare)** need a state that is hard to make by hand. Try
  each one; if you cannot reach it, write "skipped". Each is also covered
  by an automated test.
- Run the CLI against the same repositories from the ticket worktree:
  `devenv shell -- cargo run --locked --bin manyhands-cli -- …`. It shares
  the app's repository records.

### Two scratch repositories

- **Local scratch.** A fresh repository with **no remote**: Settings ›
  Repositories › **Create repository**. Leave Settings › Remotes empty for
  it. Sections 2 (local-only) and 3 (local repair) use it.
- **Shared scratch.** A second fresh repository, set up with a remote as
  in D4:
  1. Create or import a passphrase-protected key and **select it before**
     configuring the remote (Settings › Shared SSH keys) (D4 K-4).
  2. Make a bare repository on an SSH host you control
     (`git init --bare scratch.git` on the host). Add it as the remote and
     select it for publication (Settings › Remotes).
  3. Commit the selection with `git`: `git status`, then commit the changed
     Manyhands configuration file (D4 K-2).
  4. Push main: `git push {remote} main`. Approve the host on the Hosts
     page when first asked.
  5. **Second clone.** `git clone` the bare repository somewhere else. It
     stands for someone else's work and is changed only with plain `git`
     (edit, commit, push).
- **Stalling the remote.** Sections 4 and 6 need the remote stalled: on the
  host, `kill -STOP` the `sshd` process serving your connection, and
  `kill -CONT` to let it go. Have the commands ready.

### Making a ticket with plain `git`

Several scenarios need a ticket committed straight to main, as the tests'
fixtures do. Get an ID with
`devenv shell -- cargo run --locked --bin manyhands-cli -- id new`, then
write `.manyhands/tickets/{id}/ticket.md` in the repository's folder:

```
---
manyhands_managed: true
manyhands_kind: ticket
id: {id}
title: Base
type: task
status: open
---
Body of Base.
```

Commit it on main (`git add` and `git commit`), then press `secondary-r`
(Refresh) in the app. A ticket's own working copy, once the app has saved
it, is `.manyhands/worktrees/{id}/` on the branch
`manyhands/ticket/{id}`.

### Making a local conflict (Local scratch)

This is what the tests' `Fixture::conflict_locally_with_body` does, by
hand:

1. Commit a ticket titled "Base" with the body "Body of Base." on main, as
   above, and Refresh.
2. In the app, open it, Edit, change the title to "Mine" and the body to
   "Body of Mine.", and Save (`secondary-s`). This saving gives the ticket
   its own working copy and branch.
3. In the repository's folder, on main, change the same file's title to
   "Theirs" and its body to "Body of Theirs.", and commit.
4. The conflict appears when main is brought into the ticket's working
   copy: step 3.7 does it with **Repair locally…** from Close ticket….

Both sides changed the title and the body, so the sheet has two parts to
choose.

### Getting a close stopped mid-run (Shared scratch)

1. Edit and save a ticket in the app, so its branch has work to share.
2. Stall the remote.
3. Press **Close ticket…**, then **Close ticket** to confirm. The dialog
   shows "Share with {remote}" as Running and stays there; Activity reads
   "Closing · Connecting…" or the next stage.
4. Now either press **Stop at a safe point** (a stop), or kill the app with
   `kill -9 <pid>` (an interrupted run). Do not quit with `secondary-q`
   for the second: quitting stops the run at a safe point instead.
5. `kill -CONT` the remote before the next step that needs it.

A stall at "Share {main} with {remote}" (after the close is recorded and
merged) needs the stall to start after "Share with {remote}" is Done:
stop the host's listening `sshd` (the parent, not your connection's) while
"Share with {remote}" runs, so the next connection hangs. That is
**(rare)**.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure.

- **K-24.** A conflict while sharing **main** (Share main, no item) offers
  no Review changes. Its entry reads "Needs attention" and it is resolved
  outside Manyhands.
- **K-26.** A close or merge stopped while "Merging into main…", then sent
  again after main has moved, can show "Merged into main" and "Not tidied
  up yet: …" with no Repair locally…, though main never got the merge
  (`git log main` shows it missing). The CLI reports the same. Library
  ticket `01M6R79XHD9MTQCBDF27JAC351`.
- **K-25.** A close interrupted right at its end (after it finished, before
  its request was settled) reads closed with no Activity row and no lock;
  its request stays accepted and nothing in the app settles it.
- **K-20.** After a restart, a ticket whose close got past "Mark as
  closed" reads closed, offers no Close ticket…, and stays read-only with
  "Closing…" until you press **Resume** on its row in Activity.
- **K-13.** A merged item whose tidy-up was refused because its working
  copy, ignored files or remote branch changed stays editable and untidied
  ("Merged, but not tidied up: {reason}") until you undo that change
  outside the app; then **Check again** retries.
- **K-14.** A conflict on a document whose path has non-ASCII characters
  or a control character goes to the outside guidance ("This file needs
  fixing outside Manyhands"), local or remote.
- **D4 K-16 / D5 K-5.** A stopped share may keep the repository's
  reservation, so every share answers busy. Recover with
  `manyhands-cli -- operation resume <operation-id>` as in D4.

## 1. Smoke

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 1.1 | Start the app with the command above. | A window opens with no error in the terminal. | |
| 1.2 | Open a ticket of the Shared scratch (`secondary-p`, its ID, Enter). Press `secondary-3` and Tab along the header. | Comments, Edit, Share, then **Close ticket…**, then the chips. | |
| 1.3 | Open a document. Tab along its header. | Edit, Share, then **Approve and merge…**, then the chips. | |
| 1.4 | Quit (`secondary-q`). | The window closes, with no error in the terminal. | |

## 2. Close and promote

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 2.1 | **Remote.** On a Shared scratch ticket, Edit, change a line, and leave it **unsaved**. Tab to Close ticket… and press Enter. | The dialog opens: kind icon, title "Close ticket", subtitle "{short} · {title} · {repository}", and the branch. "Saving your latest edits…" shows briefly, then "Preparing…". | |
| 2.2 | Read the confirm stage. | Lead "Closing runs these steps in order. If one fails, it stops there and nothing after it happens." Steps: "Save your latest edits", "Share with {remote}", "Mark as closed" ("Closed by {you}"), "Merge into main", "Share main with {remote}", "Tidy up". Then "Files that will be merged into main ({n})" with one row per path, each tagged ticket, document or comment; a "Git: merge … into main · push {remote} · delete …" line; "Expires {time}". Buttons **Close ticket** and **Cancel**. | |
| 2.3 | Press Enter. | Each step goes Not started → Running → Done in order. "It continues in the background if you close this, and shows up in Activity." and **Stop at a safe point** show while it runs. Activity's entry reads "Closing · …" with the stage ("Connecting…", "Marking it closed…", "Merging into main…", "Sharing main…", "Tidying up…"). | |
| 2.4 | When it ends, read it. | "Done", every step Done, then "Merged into main", "Shared", "Tidied up". **Close** only. Press Enter: the dialog closes and focus is back on the header. | |
| 2.5 | Look at the ticket. In the second clone, `git pull`. | The tab shows it from main: Closed, with Closed at and Closed by, apart from Status. No Close ticket…. In the second clone the ticket's file has `closed_at` and `closed_by`, and `git branch -r` lists no `manyhands/ticket/{id}`. | |
| 2.6 | **Local-only.** On a Local scratch ticket, Edit, change a line, Save, and press Close ticket…. | Steps: "Save your latest edits", "Keep on this computer (sharing pending)" ("This repository has no remote, so nothing is sent."), "Mark as closed", "Merge into main", "Tidy up". No "Share main" step. | |
| 2.7 | Confirm with Enter. | "Done", then "Merged into main", "Sharing pending (no remote)", "Tidied up" (R-7). No "Sharing main…" stage in Activity. | |
| 2.8 | **Code on the branch.** On another Local scratch ticket, Edit and Save once. Then, with `git`, add a file such as `notes.txt` in `.manyhands/worktrees/{id}/` and commit it there. Press Close ticket…. | The file is listed with the tag `code`, and the note "This working copy includes code as well as the ticket. All of it is merged." shows. No line counts (R-3). | |
| 2.9 | Confirm and let it finish. Run `git log --stat -1 main`. | "Done". The merge on main includes `notes.txt` and the ticket file. | |
| 2.10 | **Made ready.** In the Local scratch, commit ticket A by `git`, then ticket B with `deps:` and `- {A's id}` lines in its front matter (as this repository's tickets have). Refresh. Edit and Save A once, then close A. | On Done, below the outcome lines: "Now ready:", then "{B's short} · {B's title}". | |
| 2.11 | **Promote.** On a Shared scratch document, Edit, Save, and press Approve and merge…. Confirm. | Title "Approve and merge", lead "Merging runs these steps in order. …", no "Mark as closed" step; button **Approve and merge**. Ends "Done", "Merged into main", "Shared", "Tidied up". Activity reads "Merging · …". | |
| 2.12 | **Status text `closed`.** On a Local scratch ticket, set its Status to `closed` and Save. Press Close ticket… and confirm. | It runs (P13). Afterwards the lifecycle Closed shows apart from the Status field. | |
| 2.13 | While a close runs (use the stall from "Before you start"), try Edit, Save and Share on that ticket, also from a second tab on it. | The tab reads "Closing…" ("Merging…" for a document) and Edit, Save, Share, Move… and Repair… are unavailable. Typing in a comment composer still works (R-23). | |

## 3. Conflict

### Share and post (Shared scratch)

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 3.1 | On a Shared scratch ticket, Edit and Save a line of its body (do not Share). In the second clone, on main, change the **same** line of that ticket's file; commit and push main. Press Share in the app. | The banner reads "You and someone else both changed this since it was last shared. Your work is safe." with **Review changes**. The pill reads "Needs attention". Activity's entry offers Review changes too. | |
| 3.2 | Press Review changes on the banner. | The sheet opens, focused, while "Reading the conflict…" shows briefly. Title "Choose what to keep · {title}". Description "You and someone else both changed this since it was last shared. Pick a version for each part, or keep both." A meta line: kind icon, `{short}`, the repository, and "0 of {n} chosen" in amber. The note "Saving finishes the share that stopped." | |
| 3.3 | **Look (R-27).** Read a part. | A field part names its field, with **Keep mine** and **Keep theirs** and "Choose one". The body part is "Description", with Keep mine, Keep theirs, **Keep both**. Before, Yours and Theirs are tinted monospaced lines (not editors); changed lines carry `~` and added lines `+` in the gutter. A field changed on one side only says "Only yours changed it; yours is kept." or "Only theirs changed it; theirs is kept." Under the Result editor: "Saving keeps the result on this computer. You can still edit it in the item afterwards, and Share sends it to everyone." At the foot, "Git: conflict in {path} · ours {8 chars} · theirs {8 chars}". No line counts, no author. | |
| 3.4 | Choose Keep both on the body, and one side on each field. | Each part's state reads "Keeping both", "Keeping yours" or "Keeping theirs"; the count turns green at "{n} of {n} chosen"; **Save resolution** becomes available. With a mixed choice, "This file will be saved in the standard layout." shows on the card. | |
| 3.5 | Save resolution. | The sheet closes, the banner clears, and the share's outcome follows ("Shared"). In the second clone, after `git pull`, the file holds your choices. | |
| 3.6 | Repeat 3.1 but **post a comment** in place of Share. | The comment is saved; the same banner with Review changes shows. Review changes, choose, Save resolution: the share finishes and the comment is in the second clone once. | |

### Local repair (Local scratch, R-29)

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 3.7 | Make a local conflict ("Before you start"). Press Close ticket…. | The dialog stops before confirm: "Nothing was changed. Each thing below stands in the way; deal with it, then prepare again." with "This item's changes conflict with main.", the guidance "Bring main into this ticket's working copy, then prepare again.", and **Repair locally…**, **Prepare again**, **Close**. | |
| 3.8 | Press Repair locally…. | "Bringing main into this ticket's working copy…" shows; then the dialog closes and the sheet opens. Description "This item and main both changed since they last met. Pick a version for each part, or keep both." Note "Saving finishes bringing main into this item." Two parts: the title and Description. | |
| 3.9 | Choose both parts and Save resolution. | The sheet closes and the **lifecycle dialog opens again** (R-29) with "Main is in this ticket's working copy now." and Prepare again and Close. | |
| 3.10 | Press Prepare again, then confirm. | The confirm stage shows; the close runs to "Done". | |
| 3.11 | **(rare)** Make another local conflict, press Close ticket…, then Repair locally…; press Cancel in the sheet. Quit, start again, press `secondary-4`. | Under "Pending in repositories" the local synchronization's row offers **Review changes** (R-28). It opens the same sheet. | |

### Lifecycle child (Shared scratch)

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 3.12 | Repeat 3.1's setup (save a body line in the app; change the same line on main in the second clone and push), but press **Close ticket…** and confirm. | "Share with {remote}" fails: "Failed", with "This item's working copy has a conflict to resolve." and "A conflict on this item is waiting to be resolved.", the guidance "This ticket's shared copy has a conflict to resolve first, then retry.", and **Resolve conflict…** and **Retry**. Nothing was merged ("Nothing was merged into main."). If the prepare stops first with a blocker, follow its guidance and write it down. | |
| 3.13 | Press Resolve conflict…. | The dialog closes; the sheet opens with the note "Saving lets the close continue." | |
| 3.14 | Choose and Save resolution. | The sheet closes; the lifecycle dialog shows Running again (the same request, sent again), then "Done". | |
| 3.15 | Repeat 3.12-3.14 with a document and Approve and merge…. | The sheet's note reads "Saving lets the merge continue."; the merge ends "Done". | |
| 3.16 | Make 3.12's child conflict again; when it fails, press Escape, quit, start again, press `secondary-4`. | The close's row offers Resume, and Review changes is offered on the close, not on its child's own synchronization row (R-28). From it the sheet opens; after Save resolution, the dialog shows Running, then the answer. | |
| 3.17 | **(rare, K-14)** Make 3.1's conflict on a document whose file name has a non-ASCII character (for example `é`). Review changes. | One card per path: "This file needs fixing outside Manyhands", with its reason, **Open folder** and **Check again**; the lead "Fix these files in the folder, commit the merge, then check again."; no Save resolution. | |
| 3.18 | **(K-24)** In the working clone, commit a change to a line of a ticket file straight on main; change the same line on main in the second clone and push. Press Share main. | "Needs attention", with **no** Review changes. Write down how you resolved it outside. | |

## 4. Failures

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 4.1 | **Key locked.** Start the app fresh. Close a Shared scratch ticket and choose **Not now** at "Unlock key". | "Failed" at "Share with {remote}", with **Unlock…** and **Open Sharing & keys**. Press Unlock…, enter the passphrase: one prompt, then the run goes on to "Done". | |
| 4.2 | **Key refused.** Remove your public key from the host's `authorized_keys`. Close a ticket. | "Failed" at "Share with {remote}", with **Retry** and **Open Sharing & keys**; write down the code under the message (expect `key_rejected`). Restore the key and press Retry: "Done". | |
| 4.3 | **Remote deletion refused (P6).** On the host, add an `update` hook to the bare repository that refuses deleting `refs/heads/manyhands/*` (exit 1 when the new value is all zeros; `chmod +x`). Close a ticket. | "Failed", with "Merged into main", "Shared", and "Not tidied up yet: the remote refused to delete the shared branch", and **Retry**. The ticket stays read-only ("Closing…"). | |
| 4.4 | Try Edit on that ticket. Then remove the hook and press Retry in the dialog (or Open… in Activity, then Retry). | Edit is unavailable (P23). Retry ends "Done" with "Tidied up"; no new merge or publication commit on main (`git log main` in the second clone after a pull) (P7). | |
| 4.5 | **Stop.** Get a close stopped mid-run ("Before you start", Stop at a safe point). | The button reads "Stopping…", then "Stopped at a safe point", "The cancellation finished: nothing after the completed steps ran.", "Completed: {steps}.", "Nothing was merged into main.", and **Retry**. If it had already finished: "It finished before it could stop." | |
| 4.6 | Look at the ticket's tab, then let the remote go and press Retry. | Before Retry the ticket is editable again (stopped before "Mark as closed", L-1). Retry sends the same request: "Done". | |
| 4.7 | **Quit and relaunch with Resume (P5).** Get a close stopped mid-run with `kill -9`. Let the remote go. Start the app, press `secondary-4`. | The close is listed under "Pending in repositories" with **Resume**. | |
| 4.8 | Press Resume. | The dialog opens on Running and the same close goes on to "Done". | |
| 4.9 | **(rare, K-20)** Make the same interruption with the stall at "Share main with {remote}". Start the app again and open the ticket. | It reads closed, offers no Close ticket…, and is read-only with "Closing…" (K-20). Resume on its row finishes it; then it unlocks. | |
| 4.10 | **Already running (P22).** While a close is stalled, press Escape, then open the run again from Activity (**Open…**). | Escape hides the dialog; the run goes on. Open… reopens it on Running. Resume and Continue are not offered while it runs; a resend answered busy reads "Still running…". | |
| 4.11 | **Held (P12).** Stop a close (4.5) and press Escape. Press Close ticket… again. | Either "An earlier run of this is not finished, and it holds the item. Nothing new was started." with **Continue** (or **Resume**), or a fresh confirm. Write down which. | |
| 4.12 | **Dirty main (P19).** In the Local scratch, change a tracked file in the repository's folder without committing. Press Close ticket…. | Blocked: "Main has unsaved changes outside Manyhands." with **Open folder**. Nothing merged. Undo the change afterwards. | |
| 4.13 | **(rare) Stale (P2).** At the confirm stage of a close, commit a new file in the ticket's working copy with `git`, then press Enter. | "Something changed since you confirmed." with **Prepare again**, which shows the new summary. | |
| 4.14 | **(K-13)** In the Local scratch, commit a `.gitignore` with `*.log` to main first. Edit and Save a ticket, put `scratch.log` in its working copy, and close it. | The close merges, then stops at the tidy-up: "Not tidied up yet: the working copy holds ignored files". The ticket is editable and shows "Merged, but not tidied up: the working copy holds ignored files" with **Open folder** and **Check again**. Delete the file and press Check again: "Tidied up". | |

## 5. Republish

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 5.1 | **A share's deleted branch.** Share a Shared scratch ticket once. In the second clone, `git push origin --delete manyhands/ticket/{id}`. Edit and save the ticket in the app; press Share. | "The shared copy of this item was deleted. Republish it to share it again." While its record is held: **Stop and republish…** and **Stop trying to share**; once finished: **Republish…** (R-30). Write down which you got. | |
| 5.2 | Press Republish… (or Stop and republish…). | A Republish confirmation with Branch, Last published and Local commit. After Confirm the entry reads "Republishing…", and Republish… and Stop and republish… are hidden on the entry and the item while it runs. It ends shared; `git branch -r` in the second clone lists the branch again. | |
| 5.3 | **(rare) Main's deleted branch.** On the host, `git config receive.denyDeleteCurrent ignore` in the bare repository; from the second clone, `git push origin --delete main`. Press Share main in the app. | "The shared copy of main was deleted." with **Stop trying to share** only: no Republish… (R-30). Push main back from the second clone afterwards. | |
| 5.4 | **A lifecycle's deleted branch (P21).** Share a ticket once; delete its branch in the second clone as in 5.1; Edit and Save it in the app (do not Share or Refresh). Close ticket… and confirm. | "Failed" at "Share with {remote}", with "This ticket's shared copy was deleted: share it again, then retry." and "Stop this run first, then share this ticket again.", and **Stop and republish…**. If the prepare stops first on "This item's shared copy was deleted.", write it down. | |
| 5.5 | Press Stop and republish…. | "Stopped; ready to republish." and the Republish confirmation opens over the dialog. Confirm: the dialog reads "Republishing…", then the close is sent again and runs to "Done". No loop of busy answers. | |
| 5.6 | **(rare)** If the run could not be given up, or the republish stopped. | "This run could not be stopped now." or "The republish stopped; send it again from its entry in Activity." | |

## 6. Keyboard only

Use no pointer at all in this section.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 6.1 | Open Close ticket… by Tab and Enter. At confirm, press Escape. | The dialog closes; focus is back on Close ticket…. | |
| 6.2 | Open it again; press Enter at confirm. While it runs, press Escape. | Enter confirms. Escape hides the dialog and the run goes on; Activity's **Open…** reopens it. | |
| 6.3 | On a Failed or stopped run, press Enter. On a Done run, press Enter. | Enter retries the failed or stopped run; on Done it closes the dialog, focus back on the header. | |
| 6.4 | **R-32.** Stall a close; Tab to Stop at a safe point and press Enter. Then Tab and Shift-Tab. | When Stop goes, focus stays in the dialog: Tab moves among the dialog's controls only, never to the page behind. | |
| 6.5 | **R-32.** Start the app fresh (key locked). On a stopped close, press Retry by keyboard. At "Unlock key", choose Not now with the keyboard. | The prompt opens over the dialog; after Not now focus is back inside the lifecycle dialog. | |
| 6.6 | Open a conflict sheet (3.1 or 3.7) by keyboard. Tab through it. | Every stop shows a focus ring: each part's Keep mine, Keep theirs (Keep both on the body), then the **Result** box (a tab stop after the choices), then Save resolution and Cancel. Shift-Tab runs back. | |
| 6.7 | **R-31.** On the Result box, press Enter. Type, press Tab, press Enter. Press Escape. | Enter puts focus in the editor. Tab indents (focus stays); Enter makes a newline. Escape returns to the Result box, with the sheet still open and the text kept; the part reads "Keeping the result as edited". | |
| 6.8 | Choose every part by keyboard, go into the Result editor again, and press `secondary-enter`. | The resolution is saved from inside the editor. | |
| 6.9 | Open a sheet again; on the Result box (not in the editor), press Escape. | The sheet closes; focus returns to the control that opened it (Review changes, or the header after Repair locally…). | |
| 6.10 | **R-32.** In an outside-guidance sheet (3.17), press Check again by keyboard. | When the control re-renders, focus stays inside the sheet. | |

## 7. Results

Write the build (commit), display server, scale factor, and one line per
section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| 1 | Smoke | |
| 2 | Close and promote (remote, local-only, code, made ready; P13, P20, R-7) | |
| 3 | Conflict (share, post, local repair, lifecycle child; P8, P9; R-27, R-28, R-29; K-14, K-24) | |
| 4 | Failures (key, remote deletion, stop, quit and Resume, held, dirty main, stale; P2, P4-P7, P12, P19, P22, P23; K-13, K-20) | |
| 5 | Republish (P17, P21; R-30) | |
| 6 | Keyboard only (R-31, R-32) | |
| — | Known limits hit (K-numbers, steps) | |
| — | D4 K-16 recovery used? (step and operation ID) | |
