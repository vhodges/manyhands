---
manyhands_managed: true
manyhands_kind: comment
id: "01M5X7ED31ZHEGJ5Z97GX8ZYHS"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
created_at: "2026-10-26T23:06:34Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Planning drafts

On 2026-10-26 the ticket branch merged `origin/main` at `0464c42` as
`219ec57`. The three drafts are written:

- `docs/Cycles/wave-03-cli-04-discussion-and-sync.md`
- `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-design.md`
- `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-implementation.md`

The owner's decisions 1–10 (2026-10-26), recorded in the Cycle document:

1. The library bindings for `comment add`, `conflict show` and
   `conflict resolve` land in C4, as an exception to track rule 5.
2. The gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` lands first. C4's
   implementation starts after it merges.
3. Divergent `item sync` and `repo sync` merge, and their conflicts are
   recoverable.
4. Republish consent is added in C4.
5. D4 waits for all of C4.
6. Abandoning a pending remote conflict stays on `01M4H33R34Z7C7EEKTY1ZCT950`.
7. Republish is a new confirmed verb, `item republish`.
8. A conflict's next step is typed data. The recovery-action enum is
   unchanged.
9. `comment add` takes an optional, validated `created_by`.
10. C4 adds one desktop arm for the republish preview.

Status: drafts, before independent review. Nothing was built or tested for
planning.
