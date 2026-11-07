# D4 On-Screen Checklist

This list is for checking the D4 Cycle (discussion and sync) by hand on
Linux Wayland, before the Cycle is called review-ready. It is the native
evidence for acceptance rows A1 to A14 (Cycle doc, "Acceptance And Exit
Evidence"; design, "Native checklist"). Work through sections 1 to 7 in
order. Every journey is keyboard-only: use no pointer unless a step says
so. Write each step's result in its row's last column, and a one-line
summary per section in section 8. Claude records only what you report.

## Before you start

- Start the app from the ticket worktree
  (`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN1KTQFSVR99378TDHV`)
  with a release build:
  `devenv shell -- cargo run --release --locked --features desktop --bin manyhands`
  It needs an active desktop display. A development build lags, as D3's
  checklist found.
- Run it while no long test suite or build is running on the machine.
- Use a throwaway clone of this project's repository, or a fresh
  Manyhands repository, as the **working clone**. Have a ticket in it with
  at least one comment by someone else.
- **The remote.** An SSH remote you control, reachable from this machine,
  set up in this order:
  1. Create or import a passphrase-protected key and **select it before**
     configuring the remote (Settings › Shared SSH keys). Selecting it
     afterwards does not take effect (K-4, `01M66HFV44YXNQ7ANJVA16TQ1H`).
  2. Add the remote and select it for publication (Settings › Remotes).
  3. Commit the selection with `git`: a fresh selection leaves the
     repository's index dirty (K-2, `01M669FJR27FRHCNPWAMD6VH4V`). Run
     `git status`, then commit the changed Manyhands configuration file.
  4. Approve the remote's host on the Hosts page when first asked.
- **The second clone.** First push the working clone's `main` to the
  remote (`git push <remote> main`, after step 3 above). Then make a
  second clone of the remote, changed only with plain `git` (edit, commit,
  push). It stands for someone else's
  work; it is not a second Manyhands front end.
- **Making the remote unreachable and stalled.** Sections 4 and 6 need the
  remote unreachable (for example, stop its SSH server, or block its port)
  and then stalled (for example, `kill -STOP` the server's SSH process for
  your connection, `kill -CONT` to let it go). Have the commands ready.
- **Where comments are written.** A post writes the comment file under the
  item's private copy,
  `.manyhands/worktrees/<item-id>/.manyhands/comments/<item-id>/`, and
  commits one checkpoint whose message starts "Checkpoint comment ". To
  count them: `git -C .manyhands/worktrees/<item-id> log --oneline` in
  the working clone, and `git log` in the second clone after a pull.
- In the keys below, `secondary` is Ctrl on Linux.
- Steps marked **(rare)** need a state that is hard to make by hand. Try
  each one; if you cannot reach it, write "skipped". Each is also covered
  by an automated test.
- **If every share and post answers busy after a restart.** A share whose
  transport failed after a resume can keep the repository's sharing
  reservation, listed as reconciling with no Resume or Give up (K-16,
  library ticket `01M6GHWZS03WCADS1A1JXV4PMP`). Nothing in the desktop
  frees it after a restart. Once the remote is reachable, run
  `devenv shell -- cargo run --locked --bin manyhands-cli -- operation resume <operation-id>`
  against the working clone, with the operation ID from the reconciling
  row. Write down that it happened, and in which step.

## 1. Smoke

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 1.1 | Start the app with the command above. | A window opens with no error in the terminal. | |
| 1.2 | Select the working clone (`secondary-1`, the arrows, Enter) and open the ticket by Find (`secondary-p`, its ID, Enter). | It opens in its own tab. | |
| 1.3 | Press `secondary-3`, then Tab along the header. | Comments, Edit, then **Share** (enabled), then the chips. Share's tooltip reads "Brings in others' changes and shares this item's saved work through {remote}. Unsaved edits are not included." | |
| 1.4 | Press `secondary-shift-c`. | Focus is on the Discussion heading. After the threads there is one "Write a comment…" box, not an open editor (decision 22). | |
| 1.5 | Quit (`secondary-q`). | The window closes, with no error in the terminal. | |

## 2. Discussion

Use the ticket with a comment by someone else. Every step is by keyboard.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 2.1 | From the tab strip, Tab through the item. | Tab strip, header (Comments, Edit, Share), chips, the page, Back to top, then each comment and its Reply, then "Write a comment…", then the repository's details (Share main). Shift-Tab runs back the same way. Every stop shows a focus ring. | |
| 2.2 | On "Write a comment…", press Enter. Press Escape, then open it again with Space. | Enter opens the composer with focus in its editor. Escape with no text closes it again at once, with no question, focus back on "Write a comment…". Space opens it too. | |
| 2.3 | In the open composer, look at it, then Shift-Tab once from the editor. | A toolbar (Bold, Italic, Inline code, Link, Bulleted list, Rich text / Markdown, and the draft indicator) is always shown above the editor (decision 21). The placeholder reads "Add a comment. Markdown works here." The footer reads "Posting also shares other saved work in this ticket. Unsaved edits above are not included." with **Post and sync** and **Save comment locally**, and the hint "Ctrl Enter to post · Esc to cancel · F6: next region". Shift-Tab in the editor outdents; it does not leave. | |
| 2.4 | Type a bulleted line and a second line, then press Tab, then Shift-Tab. | Tab indents the line and Shift-Tab outdents it; focus stays in the composer (decision 19). | |
| 2.5 | Press F6, then come back (`secondary-3`, `secondary-shift-c`, Tab to the composer). | F6 leaves the composer for the next region. The composer, holding text, stays open. Tab reaches its toolbar's one stop, then its editor. | |
| 2.6 | On the toolbar, move with Left, Right, Home and End, and use Bold by Enter. | The arrows move along the controls; Enter applies Bold to the selection. | |
| 2.7 | Select a word in the editor and press `secondary-k`. Type an address and Enter. Then press `secondary-k` again and Escape. | A "Link address" field opens, focused. Enter makes the word a link and focus returns to the editor. Escape closes the field and nothing changes. | |
| 2.8 | Press `secondary-shift-m` twice in the composer. | The mode switches to Markdown (monospace) and back to Rich text; the text and focus stay. | |
| 2.9 | Write a first root comment and press `secondary-enter`. The first post asks for your key's passphrase ("Unlock key"): enter it. | One "Unlock key" prompt; then "Posting · …" shows, then the comment is listed after the threads, once. The composer is empty again. | |
| 2.10 | Write a second root comment and post it with `secondary-enter`. | Listed once, after the first. | |
| 2.10a | Write a comment and press `secondary-enter` twice, quickly. | The second press is refused with "Already posting"; the comment is listed once (A5). | |
| 2.11 | Tab to the first new comment's Reply and press Enter. Write a reply, post it with `secondary-enter`. Do it again for a second reply. | The reply composer opens in that thread, after its last reply, focused, with "Replying to {you}", placeholder "Write a reply. Markdown works here." and the hint "Ctrl Enter to post · Esc to cancel · F6: next region". Each reply is listed in that thread, in order. | |
| 2.12 | Press Reply on one of those replies, write, and post. | The composer opens in the same thread and names the reply's author. The new reply lands in that thread: on disk its `parent_id` is the thread's root, not the reply (decision 5). | |
| 2.13 | Open a reply composer, type, press Escape. Press Enter. Press Escape again, Shift-Tab, Enter. | Escape asks "Discard this reply?" with Discard · Keep editing, focus on Keep editing. Enter keeps the text. The second time, Shift-Tab reaches Discard, and Enter drops the text and closes the composer, focus back on its Reply. | |
| 2.14 | In the new-comment composer, type, press Escape, and choose Discard. | "Discard this comment?". After Discard the composer stays open, empty, focused. Escape then closes it with no question. | |
| 2.15 | Open a reply composer, type a reply, wait two seconds, then kill the app (`kill -9 <pid>`). Start it again and open the ticket. | The reply composer is back in its thread, with the text, and the indicator reads "Draft kept". "Replying to" names the thread root's author, even if you pressed Reply on a reply (K-11). The avatar is a placeholder (K-11). | |
| 2.16 | Post that restored reply with `secondary-enter`. | Listed once. | |
| 2.17 | Press `secondary-shift-r` (Rebuild index) and wait for it to finish. Close the tab (`secondary-w`) and open the ticket again. | Every comment and reply is listed once, oldest first, each reply in its root's thread, as before. | |
| 2.18 | In the working clone, run `git -C .manyhands/worktrees/<item-id> log --oneline`, and list that folder's `.manyhands/comments/<item-id>/`. | One file and one "Checkpoint comment" commit per comment and reply you posted. | |
| 2.19 | While a reply is posting (rare: needs a slow remote), press Reply in another thread. | Reply elsewhere is unavailable, with "Finish or cancel the reply being posted". | |

## 3. Share

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 3.1 | On the ticket, Tab to Share and press Enter. | The pill reads "Sharing · …" with each stage ("Connecting to {remote}…", "Bringing in changes…", "Sending your saved changes…", …), then "Shared". Press `secondary-4`: the operation entry says the same. While it runs, Share reads "Sharing…" and is disabled. | |
| 3.2 | In the second clone, `git pull`. | Your comments from section 2 are there, each once. | |
| 3.3 | Press Share again at once. | "Same as the shared copy". | |
| 3.4 | Tab to the end of the item, to the repository's details, and press Share main. | It runs as a share of the main branch, and ends "Shared" or "Same as the shared copy". | |
| 3.5 | In the second clone, change another ticket with `git` (edit, commit, push). Then press Share main, or Share on that ticket, in the app. Refresh (`secondary-r`). | The change is brought in and merged; the app shows it after the refresh. | |
| 3.5a | Edit the ticket and type in its body, but do not save. Post a comment from its discussion (`secondary-enter`). Then, in the second clone, `git pull` and look at the ticket's file. | The comment arrives; the ticket's body in the second clone has none of the unsaved text (A12). Back in the app, the body still holds your unsaved text. | |
| 3.6 | On an item never edited in this app, press Share. | "Nothing to share yet: this item has no saved changes." (decision 18). | |

## 4. Offline

Use a ticket, the remote **unreachable**.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 4.1 | Post a comment. | The comment is listed with an amber chip "Saved on this computer · not shared yet" and **Retry sharing**. The operation entry offers Retry sharing too. | |
| 4.2 | Make the remote reachable. Press Retry sharing (from the chip, by Tab and Enter). | "Shared"; the chip goes. In the second clone, after a pull, the comment is there once. | |
| 4.3 | Make it unreachable again. Post another comment and wait until it has failed to share (the chip shows). Quit. | The app quits. | |
| 4.4 | Start the app again, make the remote reachable, and open the ticket. Look at the operation area (`secondary-4`) and at the comment. | The comment is listed, once. There is **no Resume row** for it in the operation area (K-15, library ticket `01M6GHWZ5CXXFC7YB3EVJ6Q4A7`) and no chip (the chip lasts for the session, K-1). | |
| 4.5 | If the composer shows that comment again on reopening (its draft re-adopted): press Retry in it. | It reads "This comment was being posted when Manyhands closed. Retry to finish posting it." or "This comment is not saved yet. Retry to finish posting it.". Retry sends the same request again: the comment is listed once and the composer is cleared. If the composer is empty, write "not shown". | |
| 4.6 | Press Share on the ticket. | "Shared". In the second clone, after a pull, the comment is there once. | |
| 4.7 | With the remote reachable but slow enough to see "Posting · …" (or stalled as in section 6), post once more and quit **while it is sending** (`secondary-q`). | The app stops the post at a safe point and quits; "Finishing" may show first, with Quit now (decision 15, K-10). | |
| 4.8 | Start the app again and open the ticket. | If the comment was saved: it is listed once, with no Resume row and no chip; Share publishes it (K-10). If its composer comes back reading "This comment was being posted when Manyhands closed. Retry to finish posting it.", Retry replays its saved result and the comment shows once. | |
| 4.9 | Press Share. Then, in the working clone and the second clone, count the comment files and "Checkpoint comment" commits as in step 2.18. | Shared. One comment file and one checkpoint per comment through this whole section; no comment appears twice. | |
| 4.9a | Stall the remote (see "Before you start"). Press Share on the ticket and wait until "Sharing · …" stays at a stage. Kill the app (`kill -9 <pid>`): do not quit, since quitting stops the share as final (decision 15). | The app is gone. | |
| 4.9b | Start the app again and press `secondary-4`. | The operation area lists the pending share with **Resume** (A14). | |
| 4.9c | Let the remote go (`kill -CONT`) and press Resume. | It ends "Shared". If the row stays stuck as reconciling, with no Resume or Give up, use K-16's recovery from "Before you start" and write that down. | |
| 4.10 | (rare) When a post is answered "not saved" (for example "This comment is not saved yet. Retry to finish posting it."), look at its operation entry. | The operation entry offers no Retry or other re-send for it; its composer is the one place to retry (decision 26). A saved post that is not shared keeps Retry sharing in both places. | |

## 5. Credentials

Start the app fresh, so the key is locked.

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 5.1 | Press Share on the ticket. | One passphrase prompt, "Unlock key", whose subtitle reads "Sharing {repository} is waiting for your passphrase.", with **Not now**. | |
| 5.2 | Choose Not now. | No other prompt. The outcome reads "Your key is locked" with **Unlock…** and **Stop trying to share**. Your work stays saved. | |
| 5.3 | Press Share again (a second request). | It does not prompt; it answers "Your key is locked" again. | |
| 5.4 | Press Unlock… and enter the right passphrase. | Exactly one prompt; the share goes on and ends "Shared". | |
| 5.5 | Quit, start again, Share, and enter a **wrong** passphrase. | It reads as an unreachable remote; the dialog shows no inline error (K-7, `01M5E6DYJ6GN7DMW4424TH3V78`). Write down what it says. | |
| 5.6 | Remove the remote's host from the pins (Settings › Hosts). Post a comment, or press Share. | For a share: "Check {host} before sharing", with **Check host**. For a comment's post: "Check the host before sharing" (a comment's stop names no host, K-3), with **Check host**. | |
| 5.7 | Press Check host, then Approve on the Hosts page, then come back and press Retry (or Retry sharing on the comment's chip). | Check host opens the Hosts page (for a comment it opens the repository's Hosts page, K-3, `01M67BN4C1SY2HQBGNE9NBYMW8`). After Approve, Retry shares it: "Shared". | |

## 6. Responsiveness

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 6.1 | Stall the remote (see "Before you start"). Press Share. | "Sharing · …" stays at a stage; **Stop at a safe point** shows in the operation entry. | |
| 6.2 | While it is stalled, Edit the ticket and type in the body; then type in a comment composer. | Typing keeps up in both. Saving may wait behind the share (K-8): write down if it does. | |
| 6.3 | Press Stop at a safe point. | The control reads "Stopping…" (the pill may read "Sharing · Stopping at a safe point…"), then "Stopped before anything was sent. Your saved work is still on this computer." or "Stopped after part of the work finished. Your saved work is on this computer; Share sends the rest.", with Share. | |
| 6.4 | Let the remote go (`kill -CONT`) and press Share. | "Shared". | |

## 7. Conflict

| # | Do | Expect | Result / notes |
| --- | --- | --- | --- |
| 7.1 | Edit and save a line of the ticket's body in the app (do not Share). In the second clone, on `main`, change the **same** line of the ticket's file with `git`, commit and push `main`. | Both saved. | |
| 7.2 | Press Share in the app. | The item shows the banner "You and someone else both changed this since it was last shared. Your work is safe. Resolving it in Manyhands arrives in a later release." with no action (decision 13). The pill reads "Needs attention". It is not retried by itself. | |
| 7.3 | Look at the body in the app. | Your saved text, unchanged. | |
| 7.4 | Look at the operation area. | The conflict is listed and cannot be abandoned (K-6, `01M4H33R34Z7C7EEKTY1ZCT950`). | |

## 8. Results

Write the build (commit), display server, scale factor, and one line per
section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| 1 | Smoke | |
| 2 | Discussion (A1, A6, A11; A5 at 2.10a and 2.19) | |
| 3 | Share (A2; A5 at 3.1; A12 at 3.5a) | |
| 4 | Offline (A3, A4; A14 at 4.9a-4.9c) | |
| 5 | Credentials (A7, A13) | |
| 6 | Responsiveness (A8, A9) | |
| 7 | Conflict (A10) | |
| — | K-16 recovery used? (step and operation ID) | |

The worktree-switch step of the PRD journey is G1's (Wave line 72:
"ordering/rebuild recovery in G1").
