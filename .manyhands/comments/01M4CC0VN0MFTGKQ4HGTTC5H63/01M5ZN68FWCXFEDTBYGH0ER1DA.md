---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZN68FWCXFEDTBYGH0ER1DA"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T21:45:17Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 9: complete

Task 9, the editor host, is complete. Its commits are a8e3cd8 and
a5152e8, with docs 55eb453 and 3f3d0c6.

- **What changed.** `EditorHost` is the tab's edit editor:
  - Rich and Markdown modes on one entity;
  - dirty tracking against the baseline;
  - `mark_saved`, which keeps undo (R-69);
  - `rebase` through `set_exact` (R-18, R-79);
  - `reload_clean` (R-19);
  - Escape to the header's Done, after a menu just opened (R-59);
  - the caret stepping over math and property blocks (R-66);
  - the caret kept in view (R-67).

  The read view and the comment bodies now share one caret-reveal helper,
  and their tests pass unedited.
- **Review.** It found one Important issue, fixed: a clean reload of the
  editor's own saved text no longer loads it again, which would have
  cleared undo for fact 81's shapes. Two small items were fixed with it.
- **Tests.**
  - The whole desktop binary, before the fix: 650 passed, 0 failed.
  - After the fix: `editor` 53 passed, `keys` 28 passed.
- **Checks.** Clippy, fmt and the CLI isolation check are clean.
