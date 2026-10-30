---
manyhands_managed: true
manyhands_kind: comment
id: "01M63NRERA1HYRZ6VRCVVMY7S6"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T11:12:11Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 24: complete

Move… and New folder (E-10) pass review. The commits are d250d2f and
a7fc609; the docs are cb23268, 1295b1b and this commit.

- **The review.** Spec compliant and approved, with 5 Minor issues. One
  fix round closed them all, and the controller checked its diff.
  - A folder made while its index still owes work now closes the dialog.
  - A file that takes the destination while a move waits is named as a
    collision, not as a changed document.
  - A refusal goes when a field changes. After the stale words, Move is
    no longer offered in that dialog.
  - The copied frame-polling test helper is now one harness helper.
- **Found.** A move retried after a lost answer completes once, with no
  second commit. Known defect 01M4PB80T7 does not show through this
  dialog, and the ticket stays open.
- **Tests.** The whole desktop binary passed 1006, with 0 failed. After
  the fix round: `move_document` 14, `new_folder` 5 and `create` 52.
  Both clippy runs and fmt are clean.
