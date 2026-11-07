---
manyhands_managed: true
manyhands_kind: comment
id: "01M5WTYRKSR7S7MN56GB4KB0ZY"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-26T20:04:06Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Planning checkpoint: the item-view design

**Base.** On 2026-10-26 this branch merged `origin/main` at `0464c42` as
`6f1463c`, with no conflicts and no rebase. It brought in C3 and the
owner's item-view design (`docs/design/item_view/`). Of library source it
changed only one test file, so the design's library facts stand.

**Product-owner decisions (2026-10-26):**

12. **The item-view design in D3:** the tab treatment, front matter in a
    GPUI Kit `Sheet`, the fixed header, inline comment display and the
    editor toolbar.
13. **Shape:** a new frame Part A; the old Parts A–D become B–E.
14. **Other front-matter keys** (a document's status, date and author
    included) stay read-only; a follow-up ticket covers editing them.
15. **Done keeps the draft** and returns to viewing.
16. **Share, Close ticket… and Approve and merge…** are shown disabled.
17. **The read view is the editor, read-only,** so text can be copied. It
    supersedes decision 6.
18. **Release only contexts this desktop session created.** It replaces
    ruling R-45.

**Source audits.** GPUI Kit 0.6.6 and zorite-editor/zorite-markdown were
audited read-only. They became design facts 49–80, a Design Deviations
table and a Not Built In D3 table.

**Review round 4, at `4a2628e`.**
- **Source and feasibility:** 2 blocking, 6 important, 9 minor.
- **Contract, consistency and adversarial:** 3 blocking, 14 important,
  5 minor, with 6 questions.
- Each finding was checked against source. Question 1 became decision 18,
  and the other five were settled as rulings R-59 and R-68 to R-74.
- The plan now has 30 tasks, with Tasks 5, 10 and 14 split.

**Not reviewed.** The round-4 edits, decision 18 included, have had no
independent review. A scoped re-review is owed before approval.

Nothing has been built or tested.
