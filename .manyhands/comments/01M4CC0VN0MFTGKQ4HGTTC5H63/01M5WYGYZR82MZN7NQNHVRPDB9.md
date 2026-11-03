---
manyhands_managed: true
manyhands_kind: comment
id: "01M5WYGYZR82MZN7NQNHVRPDB9"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-26T20:30:41Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Planning checkpoint: review round 5

**Review round 5, a scoped and adversarial re-review of `9ce38e7`:**
1 blocking, 9 important and 12 minor findings. Each was checked against
source.

**The blocker.** After a save that retires the draft, the next draft's base
was to come from the tab's own later read. That read can carry another
actor's change and hide it. The fix: D2's registry reports its own
post-save base (`DraftSaved`, R-69), which is unverified when another actor
changed the item, so the next Save reviews.

**The edit session's lifecycle as a table.** It had blockers in rounds 4 and
5. So the design now states it as a table of required outcomes (R-76).
Tasks 15–19 write their rows as tests first, and stop for a ruling on any
difference.

**The other changes:**
- **Task order.** Save now comes before Done and release, as Tasks 14–17,
  and the plan has 30 tasks numbered 0–27.
- **Releases.** A process-wide `CreatedContexts` set, consulted by every
  discard path, so decision 18 holds when a tab is gone (R-75).
- **Done** stops when a waited save does not commit. Done and Save are
  disabled during the review.
- **The Drafts page.** Its Discard waits for that draft's save, and a
  never-created item's tab closes when its draft is discarded.
- **D2's banner** keeps Restore and Review in Drafts.
- **The sheet** closes when one of its rows opens a ticket.
- **Dependencies** are compared in ID order through `normalized` (R-72).
- **Named edits** E-7, E-24 and E-29 are completed.

**Not reviewed.** The round-5 edits have had no independent review.

Nothing has been built or tested.
