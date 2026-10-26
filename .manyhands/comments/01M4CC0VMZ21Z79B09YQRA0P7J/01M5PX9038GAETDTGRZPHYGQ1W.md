---
manyhands_managed: true
manyhands_kind: comment
id: "01M5PX9038GAETDTGRZPHYGQ1W"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T12:13:25Z"
---

Task 11 (Inspect, Restore, Export and Discard) is complete. Commits:
`7ffe7f2`, the fix `2710f12`, and a comment fix `cd82674`.

**What it adds.**

- **Inspect.** A row's Inspect opens the comparison view, which lists
  each change as text. Escape closes it and returns focus to Inspect.
- **Restore.** Restore from the page opens the item's tab through
  `open_item`, with the draft attached. The page does not hide itself:
  `content.rs` routes the move (R23).
  - If another window holds the draft, Restore is refused with "…just
    now. Try Restore again." The listing is then reloaded, so a draft
    that really is held elsewhere stops offering Restore.
- **Export.** Export… writes `markdown(draft)`, and Export raw… writes an
  unreadable file's bytes.
  - It writes a temporary file in the target folder, syncs it, renames it
    into place and syncs the folder.
  - A failure shows a fixed sentence and leaves any existing file
    unchanged.
- **Discard.** Discard… opens a dialog that names the draft:
  - by its own title;
  - "Comment on <item title>" for a comment;
  - a fixed sentence for an unreadable file.

  The default is Cancel. Enter, Escape and Cancel discard nothing; only
  Confirm discards, and only that draft. If the discard fails, the file
  is kept, the session is released and "Draft not discarded" is shown.
  After a discard, focus moves to the neighbouring row, or to the tab's
  content.
- **Shared code.** The tab's restored-draft section gains Export… and
  Discard…, which use the same code as the page.
  - That code is in `view/drafts/actions.rs`.
  - The page's own actions are in `view/drafts/row_actions.rs`.
  - The tab's are in `view/item/draft_actions.rs`.
- **Open elsewhere.** A draft whose claim another process holds reads
  "Open in another Manyhands window". It offers Inspect and Export only.

**Ruling.** The Discard dialog defaults to Cancel. It matches D2's other
dialogs, which default to Keep draft (closing a tab) and Cancel
(quitting). D1's confirmation dialog confirms on Enter, but it submits a
command the user has already previewed; here Enter would lose text.

**Tests.**

| Filter | Passed |
| --- | --- |
| `view::drafts` | 38 |
| `view::item` | 30 |
| `services::drafts` | 38 |
| `view::workspace` | 25 |
| `view::tabs` | 21 |
| keyboard | 12 |

- The open-elsewhere test runs a child process that holds the claim.
- The failed-discard tests make the drafts folder read-only. A drop guard
  restores its mode, even if a test fails.
- Each fix's test fails with the fix reverted. Check and clippy are clean
  with `--features desktop` alone and with `--all-targets --all-features`,
  and fmt is clean.

**Review.**

- **Task review.** The spec is met, but the review asked for fixes, with
  two Important findings:
  - no failed-discard test;
  - Enter in the Discard dialog discarded the draft.
- **Fix round.** It closed both, and nine minors:
  - the folder sync after the export's rename;
  - focus after a discard, or when the row being inspected goes away;
  - the reload after an open-elsewhere refusal;
  - the Restore focus check;
  - naming a draft with an empty title;
  - the quitting and dialog gates;
  - two drop guards.
- **Re-review.** A scoped re-review approved the fix.
- **Deferred to the final review:**
  - on the page, the focus move after a discard does not check for an
    open dialog;
  - the "Try Restore again" notice can outlive the other window's hold;
  - a failed discard in a tab relies on the workspace's focus-lost
    handler.

Task 12 follows.
