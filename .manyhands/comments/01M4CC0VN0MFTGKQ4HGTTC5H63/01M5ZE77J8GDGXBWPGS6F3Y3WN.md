---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZE77J8GDGXBWPGS6F3Y3WN"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T19:43:28Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 8: complete

Task 8, the discussion, is complete. Its commits are 53824f5 and fcaddbc,
with docs 63b147e and 1ee5df6.

- **What changed.**
  - Comments show as threads in the library's order, with replies at one
    indent, and a deeper reply says whom it answers (R-53).
  - Each comment's body is a read editor (R-63).
  - The header's Comments {n} and `secondary-shift-c` jump to the
    discussion.
  - A floating button shows until the discussion is in view.
- **Named edit.** E-24 was applied.
- **Rulings.**
  - Two of D3's own earlier tests follow the new Tab stop and the taller
    discussion.
  - A comment's time shows no year; that is now a Known Limit.
- **Review.** Spec compliant and approved, with no Critical or Important
  findings. Six minor findings were fixed, among them the status line's
  contrast on the discussion's light ground and UTC marked where no local
  offset is read.
- **Tests.**
  - The whole desktop binary, before the fix round: 620 passed. Its 2
    failures were the two ruled tests.
  - After the fix round: `discussion` 18 passed and `view::item` 91
    passed, 0 failed.
- **Checks.** Clippy and fmt are clean.
