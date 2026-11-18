---
manyhands_managed: true
manyhands_kind: comment
id: "01M61NDJDK2RKJQKE1651TQGCY"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T16:27:45Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 17: complete

Task 17, release on leave, is complete. Its commits are 0a78378, 48c6422
and 34b86ec. The docs are 8b04988, 7a75efa, 677af4c, ff3fde9 and this
commit.

- **What changed.** An item's private copy is released when a tab leaves
  it clean, or when a draft of it is discarded, only when all four of the
  design's conditions hold:
  - this run made the copy;
  - the tab left clean or a draft was discarded;
  - no save is in flight;
  - the draft store lists no draft of the item.

  A refusal shows in the operation area. Quitting releases nothing.
- **Outcome rows.** All four of Task 17's required-outcome rows answered
  as the design's table says (R-76).
- **Owner decision, 2026-10-28.** R-78 is narrowed: a release that the
  library refused, touching nothing on every attempt, no longer blocks
  the earlier Edit's Resume. A release with a lost answer still blocks.
- **Rulings.**
  - No copy is released while a tab of this run is editing the item.
  - A Resume that R-78 blocks says why.
  - Discard waits for a draft that is still beginning.
- **Review.** Approved. The first fix round closed four minor findings.
  A re-review found one Important gap, in the narrowing, and the second
  round fixed it. The second re-review approved, with no new issues.
- **Tests.** The whole desktop binary: 852 passed, 0 failed. No main test
  changed.
- **Checks.** Clippy and fmt are clean.
