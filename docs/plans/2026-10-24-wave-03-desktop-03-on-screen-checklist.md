# D3 On-Screen Checklist

This list is for checking the D3 Cycle (rich-text and source authoring) by hand on Linux Wayland, before the Cycle is called review-ready. Work through sections 1 to 3 in order. Tick each box as you go. Write every result in the table in section 4.

## Before you start

- Start the app from the worktree
  (`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN0MFTGKQ4HGTTC5H63`)
  with:
  `devenv shell -- cargo run --locked --features desktop --bin manyhands`
  It needs an active desktop display.
- While Claude's final gate is running, that command may wait for
  cargo's build lock. If it does, run the same build directly instead:
  `devenv shell -- target/owner-check/manyhands`
  It was built with the command above from the code at `74ae378`.
- Use a throwaway branch of this project's repository (the Manyhands repository).
- Nothing here needs a network connection. The `https` link in step 2.3 only has to open the browser; the page does not have to load.
- Have these ready before the steps that need them:
  - a fixture document under `docs/` with a words-mixed `$$` pair, an image, and the three refused images of step 2.2, committed on the throwaway branch;
  - a 200 KiB document, and a ticket with 60 comments (step 2.10);
  - the three repair files of step 3.10;
  - IBus or Fcitx, if you have it (step 2.5).
- In the keys below, `secondary` is Ctrl on Linux.
- Steps marked **(rare)** need a state that is hard to make by hand. Try each one; if you cannot reach it, write "skipped" for it in the table. Each is also covered by an automated test.
- Some steps need crash recovery off, which happens when the draft store cannot open. To switch it off, quit the app, then run:
  `mv ~/.local/share/manyhands/drafts ~/.local/share/manyhands/drafts.keep && ln -s /tmp ~/.local/share/manyhands/drafts`
  To switch it back on, quit the app, then run:
  `rm ~/.local/share/manyhands/drafts && mv ~/.local/share/manyhands/drafts.keep ~/.local/share/manyhands/drafts`

## 1. Smoke test

- [ ] **Do:** Start the app with the command above. **Expect:** A window opens with no error in the terminal.
- [ ] **Do:** Open the throwaway repository. **Expect:** Its tickets and documents list.
- [ ] **Do:** Open a document. **Expect:** It opens in its own tab.
- [ ] **Do:** Open a ticket. **Expect:** It opens in its own tab.
- [ ] **Do:** Quit the app. **Expect:** The window closes, with no error in the terminal.

## 2. Native checklist

### 2.1 Frame

Open a ticket and a document, in the dark and the light theme.

- [ ] **Do:** Scroll the page. **Expect:** The header stays while the page scrolls, and the page is white in both themes.
- [ ] **Do:** Click each chip. **Expect:** Each chip opens the details sheet at its field.
- [ ] **Do:** Press Escape in the sheet. **Expect:** Focus returns to the chip.
- [ ] **Do:** Show the discussion. **Expect:** The floating comments button hides once the discussion shows.
- [ ] **Do:** Look at Share and the primary action. **Expect:** Both are disabled, with their tooltip.
- [ ] **Do:** Scroll to the end of the item. **Expect:** The selected repository's details sit at the end of the item's scroll, after the discussion, and the page still fills the region.

### 2.2 Read view

Commit a fixture document under `docs/` with a words-mixed `$$` pair, an image beside it, and three images that are refused: `![r](https://example.com/r.png)`, `![s](s.svg)` with an SVG file beside it, and `![t](../../t.png)` (it leaves the repository). Then open it.

- [ ] **Do:** Open the document. **Expect:** It renders with its image. The three refused images each show as their source text, and nothing is fetched (no network activity). (Task 27 review)
- [ ] **Do:** Select a paragraph and copy it, then paste elsewhere. **Expect:** The paste is its Markdown.
- [ ] **Do:** Type, paste, middle-click paste, click a checkbox, drag a table column, and right-click. **Expect:** None of these changes anything.
- [ ] **Do:** Hover over blocks and tables. **Expect:** The editor's block grip and table handles show. Using them changes nothing.
- [ ] **Do:** Look at each image, then drag its corner resize grip. **Expect:** Each image shows a corner resize grip. Dragging it changes nothing.
- [ ] **Do:** Arrow through the `$$` block. **Expect:** The caret passes it.
- [ ] **Do:** Click the sheet's Inspect link. **Expect:** It shows the file.
- [ ] **Do:** Run `git status`. **Expect:** It is unchanged.

### 2.3 Links

Click each of these and check the result.

- [ ] **Do:** Click a relative link. **Expect:** It opens its tab.
- [ ] **Do:** Click a `file:` link. **Expect:** Nothing happens.
- [ ] **Do:** Click a `javascript:` link. **Expect:** Nothing happens.
- [ ] **Do:** Click a missing link. **Expect:** It says not found.
- [ ] **Do:** Click an `https` link. **Expect:** It opens the browser.

### 2.4 Edit and undo

Edit (`e`) the fixture document of step 2.2.

- [ ] **Do:** Press `e`. **Expect:** The caret is where you last clicked in the read view.
- [ ] **Do:** Press Done at once, then run `git status` and `git branch`. **Expect:** Nothing is written, `git status` is unchanged, and `git branch` shows the context gone.
- [ ] **Do:** Edit again. Type in rich mode, switch mode (`secondary-shift-m`), type, then undo past the switch. **Expect:** The tab is clean: the pill reads "Editing · saved on this computer", and the tab in the strip shows no unsaved dot. (Task 27 review)
- [ ] **Do:** Type again, then press Done. **Expect:** The saved copy shows, with the kept-draft banner.
- [ ] **Do:** Press Continue editing. **Expect:** The text and undo are back.
- [ ] **Do:** Discard, then run `git branch`. **Expect:** `git branch` shows the context gone.

### 2.5 Input method

- [ ] **Do:** Edit again and compose a non-ASCII character, and CJK through IBus or Fcitx if available. **Expect:** The text and cursor are right.
- [ ] **Do:** In the read view, compose text. **Expect:** Composing changes nothing.

### 2.6 Toolbar and focus

- [ ] **Do:** In rich mode, use every toolbar control by keyboard, with the arrows along the toolbar. **Expect:** Every control works.
- [ ] **Do:** Switch to Markdown mode and use every toolbar control by keyboard, with the arrows along the toolbar. **Expect:** Every control works.
- [ ] **Do:** Press Tab in the editor. **Expect:** Tab indents.
- [ ] **Do:** Press Escape in the editor. **Expect:** Focus moves to the header's Done.
- [ ] **Do:** Press F6 in the editor. **Expect:** F6 leaves the editor.
- [ ] **Do:** Right-click in the editor, then press Escape. **Expect:** The menu closes.

### 2.7 Save

- [ ] **Do:** Save offline (`secondary-s`). **Expect:** The pill reads "Editing · saved on this computer".
- [ ] **Do:** Run `git log` in the worktree. **Expect:** It shows one commit.
- [ ] **Do:** Type again and save again, then run `git log`. **Expect:** Two commits.

### 2.8 Review

- [ ] **Do:** While editing, commit a change to the item's file with `git` in its worktree, then press Refresh. **Expect:** The change banner shows.
- [ ] **Do:** Press Save. **Expect:** The review opens.
- [ ] **Do:** Take theirs for the title and keep mine for the body, then Save. **Expect:** The save goes through with those choices.

### 2.9 New ticket

- [ ] **Do:** Create a new ticket by keyboard, with a dependency chosen in the sheet's picker, and Save. **Expect:** The ticket is created with that dependency.
- [ ] **Do:** Make the dependency's parent this ticket. **Expect:** The cycle is explained, and Save is unavailable.

### 2.10 Large document

- [ ] **Do:** Open a 200 KiB document, then edit it. **Expect:** Note any lag on opening, while scrolling and while typing.
- [ ] **Do:** Open a ticket with 60 comments. **Expect:** Note any lag on opening and while scrolling.

### 2.11 Record

- [ ] **Do:** Write down the build, display server, scale factor and result for each step. **Expect:** All four are recorded in section 4.

## 3. Words and looks

Each line quotes the app's words. Tell Claude about any wording that differs, or that you would word differently. The "(Task N)" tag says where each one came from.

### 3.1 Tab strip and header

- [ ] **Do:** Open a ticket or document and press Edit, before typing. **Expect:** The pill reads "Editing · saved on this computer". (Task 14)
- [ ] **Do:** Leave a draft kept on a ticket that needs attention, then on a closed ticket, and read the pill. **Expect:** "Needs attention · unsaved changes kept" and "Closed · unsaved changes kept", each with its icon. (Part A review)
- [ ] **Do:** Press Edit and look at the pill while the private copy is made (rare; it shows only when the app has to wait). **Expect:** "Getting a private copy ready…", Edit disabled, and a banner with Resume. (Task 14)
- [ ] **Do:** Open a file that is not a valid ticket or document. **Expect:** The "Needs fixing" banner shows the same reason as before, not "This ticket has no type, so Manyhands can't save it yet." (Task 5b)
- [ ] **Do:** Press Tab from the tab strip. **Expect:** Focus goes to Comments {n}, then the first chip. (Task 8)

### 3.2 Details sheet

- [ ] **Do:** Press Edit on an item, press Done, and open the details sheet. **Expect:** "Viewing your private copy. Choose Edit to change these." (Part A review)
- [ ] **Do:** Open a ticket's details sheet while viewing. **Expect:** Waiting on shows its dependency rows. (Part A review)
- [ ] **Do:** Edit a ticket, empty its Type, and look at the sheet. **Expect:** "A ticket needs a type before it can be saved." and a red border on the field. The other required fields give their own messages. (Task 20)
- [ ] **Do:** Type in a Type, Status, Project or Team field. **Expect:** Suggestion buttons show. Choosing one replaces the text, and undo brings the old text back. (Task 20)
- [ ] **Do:** Open the review (step 2.8) with the sheet open. **Expect:** The sheet keeps "Saved with the ticket (Ctrl S)" while its fields are read only. (Task 20)
- [ ] **Do:** Empty a required field, then press Done. **Expect:** The prompt disables Save without saying why. (Task 20)
- [ ] **Do:** Click a row's label in the sheet. **Expect:** Focus goes to the row, not its input. (Task 20)
- [ ] **Do:** Put focus on the sheet's own close button and press `secondary-s`. **Expect:** Nothing happens. (Task 20)
- [ ] **Do:** Open the sheet of a new, never-saved item while its private copy is made. **Expect:** "Not created yet. These can be changed once its private copy is ready." and "Read only until then". The sheet has no Inspect link. (Task 23 review)

### 3.3 Read view and discussion

- [ ] **Do:** Open a ticket with comments that hold an image. **Expect:** Each image in a comment shows the editor's resize grip. Dragging it changes nothing. (Part B review)
- [ ] **Do:** Press Edit, then press Refresh and look at the open tabs of the repository. **Expect:** A clean tab reloads with the caret kept in place. (Task 7)

### 3.4 Editing and the toolbar

- [ ] **Do:** Press Edit and look under the edit toolbar. **Expect:** A muted line reads "Esc: Done · F6: next region". (Part B review)
- [ ] **Do:** Keep a draft and watch the toolbar's status for a while. **Expect:** It updates about once a second with no flicker. Note any lag. (Task 14)

### 3.5 Saving, Done and drafts

- [ ] **Do:** Type in an edit tab, then quit at once (or with the draft store unavailable). **Expect:** "1 tab has unsaved changes that are not kept as a draft", with Quit anyway and Cancel. Cancel is the default and shows that tab. With two such tabs it reads "N tabs have unsaved changes that are not kept as drafts". (Part C)
- [ ] **Do:** Do the same, then remove the repository from the list. **Expect:** The confirmation adds "1 tab has unsaved changes that are not kept as a draft. Removing closes it." With several tabs: "N tabs … Removing closes them." (Part C)
- [ ] **Do:** Close an edit tab that has a draft but no unsaved changes. **Expect:** The dialog is titled "Close a tab with a draft" and reads "This tab has a draft, and no unsaved changes. Keep it to continue later, or discard it." (Part C)
- [ ] **Do:** Open the Drafts page and look at a draft's Restore. **Expect:** The hint reads "Open in its tab to continue editing". Restore is disabled while a tab in this window edits that item. (Part C)
- [ ] **Do:** With the review open (step 2.8), close the tab. **Expect:** The close dialog stays open and says "Cancel to review the changes." (Task 19)
- [ ] **Do:** Open the review (step 2.8). **Expect:** Its intro reads "This item changed since your edits began." (Task 19)
- [ ] **Do:** Press Resume when a release of the item was sent since (rare). **Expect:** The banner reads "Your private copy cannot be made ready again from this request: a release of this item was sent since." It goes on with either "Abandon its pending operation in the operation area, then press Edit." or "Press Edit to try again." The operation area's Retry note reads "Not sent again after a later Edit or release". (Task 17)
- [ ] **Do:** Close a tab with a restored draft, and press Save in its dialog by Enter. **Expect:** While the editor opens behind it, focus stays in the dialog, and Escape still closes it. Note where focus goes once it closes: it should be on a visible control or in the content region, and one Tab or a click reaches the editor. (Task 27 review)
- [ ] **Do:** Restore a ticket draft whose Waiting on or Parent you changed, and press Save in the restored section. Do it again from the close dialog's Save. **Expect:** Each time the draft saves, perhaps after a moment while the relationships are checked; from the close dialog, the tab then closes. (Task 27 review)

### 3.6 Images and links

- [ ] **Do:** Open a document with images, edit the file outside the app, and press Refresh. **Expect:** Each image redraws after the reload. (Task 27)

### 3.7 Relationships

- [ ] **Do:** Edit a ticket and look at its Waiting on and Parent rows. **Expect:** Each chip shows an icon, short code and title, state, ID and a Remove icon. Add…, Change… and Clear are small outlined controls. (Task 21)
- [ ] **Do:** Click a chip while editing. **Expect:** The chip does not open its ticket. (Task 21)
- [ ] **Do:** Press Add… and look at the picker. **Expect:** The picker sits under its row, with the filter "Filter by title, or type a short code". (Task 21)
- [ ] **Do:** Leave the filter empty. **Expect:** The picker shows the first eight tickets in list order, with no hint that more exist. (Task 21)
- [ ] **Do:** Type a filter that matches nothing. **Expect:** "No ticket matches". (Task 21)
- [ ] **Do:** Choose a parent that makes a cycle (step 2.9). **Expect:** The explanation reads "This makes a cycle: {names}." in the danger color, under the field you changed. Save reads "A ticket cannot be saved with a relationship cycle." (Task 21)
- [ ] **Do:** Look at the end of the relationships. **Expect:** The index note reads "The index may be missing tickets. Save checks again." (Task 21)
- [ ] **Do:** Look at a dependency that cannot be found (rare). **Expect:** It reads "not found, kept". An item that is not a ticket reads "Not a ticket: {names}." and Save reads "A ticket's dependencies and parent must be tickets." (Task 21)
- [ ] **Do:** Open the picker when the ticket list cannot load (rare). **Expect:** "Could not list the tickets". (Task 21)
- [ ] **Do:** Change Parent or Waiting on, and look at Save at once. **Expect:** For a moment Save is unavailable, with "Save waits while the relationships are checked." It may be too quick to see. (Task 27 review)

### 3.8 Creating documents and tickets

- [ ] **Do:** Look at the documents list and the tickets list. **Expect:** "New document…" sits beside New folder…, and "New ticket…" sits in the same place in the ticket list. `secondary-n` opens the dialog from the navigation region. (Task 23)
- [ ] **Do:** Open each dialog. **Expect:** The titles are "New document" and "New ticket". The labels are "Title", "Type", "Folder" and "File name", with "Cancel" and "Create". Check the layout. (Task 23)
- [ ] **Do:** Open a dialog with an empty Title. **Expect:** The required-field words show at once. (Task 23)
- [ ] **Do:** Leave File name empty in the document dialog. **Expect:** "A document's file name must end in .md." shows. (Task 23)
- [ ] **Do:** Type a Folder outside `docs`. **Expect:** "A document's path must be under docs." (Task 23)
- [ ] **Do:** Type the path of a file that exists. **Expect:** "Another file is already at {path}." If it appears only at Save, it reads "Another file is already at {path}. The draft is kept." (Task 23)
- [ ] **Do:** Create a ticket in a repository with no initials. **Expect:** "This repository needs initials before it can create tickets." with an "Open Settings" button. The button lands on the identity form with focus in its name field. (Task 23)
- [ ] **Do:** Create with the draft store unavailable (rare). **Expect:** "Crash recovery is off, so a new item cannot be created." If the draft cannot be written: "The new item's draft could not be written, so nothing was created." plus the store's error text. (Task 22, Task 23)
- [ ] **Do:** Press Create and look at the new tab. **Expect:** Its title, chips, pill and strip come from the draft, with no not-found line. Focus lands in the content region, not the editor. (Task 23)
- [ ] **Do:** Open the Drafts page with a draft whose item was never created. **Expect:** Its state reads "The item is not created yet". (Task 22)
- [ ] **Do:** Press Done, then Save on a new item. **Expect:** The header keeps its title until the read lands, with no "not found" line. The pill reads "Viewing your private copy". (Part D review)
- [ ] **Do:** Create a new ticket with Project emptied and Status set to `closed`. **Expect:** Both are saved as typed. (Part D review)
- [ ] **Do:** Create a new item at a path whose file cannot be checked (rare). **Expect:** The library's own refusal shows, with no path words. (Part D review)
- [ ] **Do:** Discard the draft from the Drafts page while New ticket… is preparing. **Expect:** The new tab closes. (Part D review)
- [ ] **Do:** Create a document at a path that turns out to be taken, then look at the operation area. **Expect:** The first prepare request still shows as failed while the flow goes on. (Task 22 review)
- [ ] **Do:** Hover the header's pill on a new item's tab before it is created, then after. **Expect:** Before, no tooltip. After, the tooltip names where the item was read from. (Part D review)

### 3.9 Move and folders

- [ ] **Do:** Open a document's details sheet while viewing. **Expect:** "Move…" sits to the right of the Folder box, with its note under the row. (Task 24)
- [ ] **Do:** Start an edit or a save, then look at Move…. **Expect:** It is disabled, and the note reads "Move is available while viewing, with no draft or save in progress." While a private copy is being removed it reads "Move waits while a private copy is removed." (Task 24)
- [ ] **Do:** Press Move…. **Expect:** The dialog is titled "Move document", with Folder and File name (the document's own), "Cancel" and "Move". (Task 24)
- [ ] **Do:** Leave the path unchanged. **Expect:** "The document is already at this path." (Task 24)
- [ ] **Do:** Type the path of an existing file. **Expect:** "Another file is already at {path}." The refusal goes when a field changes. (Task 24)
- [ ] **Do:** Change the document in another place, then press Move (rare). **Expect:** "This document changed since it was read. Close this and try again." Move is no longer offered in that dialog. (Task 24)
- [ ] **Do:** Press New folder… beside New document…. **Expect:** The heading is "New folder". The explanation reads "A new folder stays on this computer until a document is saved in it." The field is "Folder name", with the path shown. The parent is the selected folder, else `docs`. (Task 24)
- [ ] **Do:** Create a folder that exists. **Expect:** "Folder exists", and Create is not offered for that name. (Task 24)
- [ ] **Do:** Type an empty name, a name with `/` or `\`, `.`, `..`, or a leading `.`. **Expect:** "This is not a valid folder name." (Task 24)
- [ ] **Do:** Press Move, New folder's Create, or a create dialog's Create, and watch Cancel while it works. **Expect:** Cancel is disabled until the dialog closes or shows a refusal. Escape still closes the dialog. It may be too quick to see. (Part E review)
- [ ] **Do:** Start the app and open a document's details sheet at once (rare). **Expect:** Until the drafts are listed, Move… is disabled with "Move is available while viewing, with no draft or save in progress." (Part E review)
- [ ] **Do:** With crash recovery off (see "Before you start"), open a document's details sheet while viewing. **Expect:** Move… is offered, and pressing it opens the dialog. (Part E review)

### 3.10 Repair

For these steps, put on the throwaway branch:
- a document under `docs/` whose front matter is broken;
- a plain Markdown file under `docs/` with no front matter;
- a file under `docs/` that is not UTF-8.

- [ ] **Do:** Open the broken document. **Expect:** The "Needs fixing" banner shows D1's reason, with "Repair…" enabled. (Task 25)
- [ ] **Do:** Press Repair…. **Expect:** A plain text area holds the whole file exactly as written, with "Preview" and "Cancel". Tab and Shift-Tab leave the text area. (Task 25)
- [ ] **Do:** Fix the front matter and press Preview. **Expect:** The confirmation shows Root, Path and ID. (Task 25)
- [ ] **Do:** Confirm. **Expect:** The tab now shows the item in its private copy, in the same place in the strip. If a tab of that item was already open, that tab shows instead and this one closes. (Task 25)
- [ ] **Do:** Open the plain Markdown file. **Expect:** An amber banner reads "This file isn't a managed document yet. Adopting it adds a title and an ID at the top; the rest stays exactly as written.", with "Adopt…". (Task 25)
- [ ] **Do:** Press Adopt…. **Expect:** The text starts with front matter holding a new ID and a title. The title is the file's first heading, or else the file name. Below it, the file is exactly as written. (Task 25)
- [ ] **Do:** Press Preview. **Expect:** The confirmation adds "A new ID will be assigned". (Task 25)
- [ ] **Do:** Press Cancel, then Adopt… again. **Expect:** The same ID as before. (Task 25)
- [ ] **Do:** Type in a repair, then look at the header's pill. **Expect:** "Editing · unsaved changes", also while Confirm runs. Delete what you typed, or press Cancel and then Discard, and the pill shows viewing again. (Part E review)
- [ ] **Do:** Press Preview, then Escape from the confirmation. **Expect:** Focus is back on Preview. If you press Confirm and then Escape while the repair runs, focus is in the repair's text area instead. (Part E review; Task 27 review)
- [ ] **Do:** After a repair, open the same file again from the Documents list. **Expect:** It still shows "Needs fixing". The repair is in the item's private copy, not on the main branch, until that work is merged. Repairing it again is refused. This is a Known Limit. (Part E review)
- [ ] **Do:** Type in a repair, then close the tab. **Expect:** "Discard your repair?" and "Your changes to this file are not saved anywhere.", with "Discard" and "Cancel". Cancel is the default. Closing without typing anything asks nothing. (Task 25)
- [ ] **Do:** Type in a repair, then quit. **Expect:** The quit dialog counts that tab among those that "have unsaved changes that are not kept as drafts". (Task 25)
- [ ] **Do:** Open the file that is not UTF-8. **Expect:** "This file is not UTF-8 text, so it cannot be repaired here.", and no Repair…. (Task 25)
- [ ] **Do:** From an item's details sheet, open its file by Inspect. **Expect:** No Repair… anywhere on that tab. (Task 25)
- [ ] **Do:** Open Repair…, change the file outside the app and commit it, then press Preview. **Expect:** The library's refusal, with its message and code. The dialog offers no Prepare again, and your text is kept. (Task 25)
- [ ] **Do:** Press Confirm, then close the tab while the repair runs (rare). **Expect:** "Discard your repair?" reads "Your repair was sent and may already be made. Discarding does not undo it." Discard closes the tab, and the repair is made once. (Task 27 review)

### 3.11 Keyboard

- [ ] **Do:** Press Create or Move by Enter in a dialog. **Expect:** While it works, the focus ring stays on the disabled button. If it is refused, focus moves to the field with the note, or else to the button. (Task 26)
- [ ] **Do:** Press `secondary-shift-e` with focus in the tab strip. **Expect:** Nothing happens. It works from the header and the page. (Task 26)
- [ ] **Question:** In the review, Tab stops at the body's result editor, because that editor keeps Tab. The other fields are reached by Shift-Tab from below, or by Back to top and then Shift-Tab. Should the other changed fields come before the body's result editor? (Task 26)

## 4. Results

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| 1 | Smoke test | |
| 2.1 | Frame | |
| 2.2 | Read view | |
| 2.3 | Links | |
| 2.4 | Edit and undo | |
| 2.5 | Input method | |
| 2.6 | Toolbar and focus | |
| 2.7 | Save | |
| 2.8 | Review | |
| 2.9 | New ticket | |
| 2.10 | Large document | |
| 2.11 | Record (build, display server, scale factor) | |
| 3.1 | Tab strip and header | |
| 3.2 | Details sheet | |
| 3.3 | Read view and discussion | |
| 3.4 | Editing and the toolbar | |
| 3.5 | Saving, Done and drafts | |
| 3.6 | Images and links | |
| 3.7 | Relationships | |
| 3.8 | Creating documents and tickets | |
| 3.9 | Move and folders | |
| 3.10 | Repair | |
| 3.11 | Keyboard (and the review-order question) | |
