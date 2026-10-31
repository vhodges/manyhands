---
manyhands_managed: true
manyhands_kind: comment
id: "01M61SP1SQ6YEVREFF9XQAE1QJ"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T17:42:17Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 18: Restored drafts, close-dialog Save and late results

Commit 1e4d4c7 holds the code and tests; this commit holds the ledger row.

- **What changed.**
  - Continue editing of a stored or restored draft claims the draft,
    prepares the context and opens the editor on the draft's own text,
    exactly.
  - The draft's base takes the context's token only when its source and
    fields match; otherwise the review opens.
  - Save from a restored draft, and the close dialog's Save, take the
    same steps and then save. The dialog stays open while it saves, shows
    a pending checkpoint with Retry or a refusal with its code, and closes
    the tab once the draft retires.
  - D2's draft banner is now in the header's banner slot, with Continue
    editing.
  - A late read never replaces a dirty or drafted editor. A clean editor
    reloads only a newer body, keeping its caret.
- **Outcome row.** The close dialog's Save answering a pending checkpoint
  keeps the dialog open with Retry, and the tab stays, as the design's
  table says.
- **Tests**, desktop-test-support, each run once at the end:
  - `view::item`: 217 passed;
  - `view::tabs`: 34 passed;
  - `drafts`: 140 passed;
  - `keyboard_tests`: 23 passed;
  - the whole desktop binary: 866 passed, 0 failed.

  Both clippy runs and fmt are clean.
- **Rulings.**
  - The controller approved the edit to Task 14's
    `edit_reads_continue_editing_while_a_draft_is_stored`. It needed one
    line more, `allow_parking`.
  - The controller confirmed five route choices.
  - D2's words stay.
  - A new item's draft waits for Task 22.
