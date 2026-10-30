---
manyhands_managed: true
manyhands_kind: comment
id: "01M64YJAHWVJEJN13DJKWRTE4W"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T23:05:21Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
Task 27: the re-review, the residual fixes, and the owner's checklist.

The scoped re-review of the fix round (`175cbc5..8fbc24d`) found all 16
findings addressed and one new Important defect, N1. N1 was a Save
pressed through Continue editing (the restored section's Save, the close
dialog's Save on a restored draft, a new item's create), which was dropped
while the open's relationship check ran.

Fixed in two short rounds, each change test-first:
- Fix 2 (`071f49a`, `0e06c68`):
  - that Save now waits for the check, then is sent unless a reason
    blocks it;
  - a refocus that lands while the details sheet is open is dropped.
- Fix 3 (`546d00c`): a tab that closes in the background leaves focus
  where the user is.

The whole desktop binary at fix 2 passed: 1079 run, 0 failed. Fix 3 ran
its filters; the final gate runs the whole binary.

The design gains one Known Limit: an editor that opens under a dialog or
the sheet takes no focus, and one Tab reaches it.

The owner's on-screen checklist is committed as
`docs/plans/2026-10-24-wave-03-desktop-03-on-screen-checklist.md`.

Next: the owner's checklist and smoke test, and the final gate.
