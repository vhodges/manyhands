---
manyhands_managed: true
manyhands_kind: comment
id: "01M63YN25YGC1AVTCSW613C4A1"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T13:47:37Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
Task 25 (Repair) is complete.

- **Commits:** `862c82e` and `44c8c0a` (Repair from the tab, through the one confirmation); fix round 1 `9e16a94` and `7743407`; fix round 2 `74ae2e5`.
- **Tests:** the whole desktop binary passed 1023 tests with 0 failures at `862c82e`. After the fixes: `repair` 21, `confirm` 24, `view::item` 312, `view::tabs` 36. Both clippy runs and fmt are clean. No library tests were run, since no library code changed.
- **Review:** the first review found 1 Important and 7 Minor issues. The Important one: Adopt… added none of the title and ID its banner promises. Round 1 fixed all eight, and its re-review approved, with three new Minor issues. Round 2 fixed two of them, and the controller checked its diff. The third is parked.
- **Behavior:**
  - Repair… is offered from "Needs fixing", and Adopt… from an unmanaged document's banner.
  - Adopt… opens the file under front matter holding a new ID and a title. The ID is made once for each read of the file.
  - A repair made makes the tab follow the item in its private copy, because a repair never writes the shared copy.
  - After a lost answer, the view waits for that same request.
  - A conforming file's Inspect tab offers no Repair.
- **New Known Limits,** to be added at Task 27:
  - Repair… inserts no ID.
  - A repair refused because the file changed starts again from the file as it is now.
  - The operation area's Prepare again for that refusal is refused the same way.
- Rulings, owner copy and parked items are in the execution ledger.
