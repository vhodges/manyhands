---
manyhands_managed: true
manyhands_kind: comment
id: "01M6HD1AXTZMAR7CAG99WYZJSF"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6H3M46PYCZ5SDT4N8MJS2F1"
created_at: "2026-11-03T19:09:06Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Decision 34's review applied

All seven review items are fixed on top of Task 14's commit.

- **Conflict lines.** They now print only when the blockers name the item's own context as conflicted as well as `conflict_pending`. Their note always goes with them.
- **Unconsented resolution.** If the resolution takes the peer's side, the resolution itself is published, but the same request answers `external_change` and ends. There is no merge, and a new prepare is free. This is correct under consent, and the documents and note now say so.
- **External-only conflicts.** A Git repair in the context with a committed merge, then the same request, gives `ok` with one merge.
- **Deleted remote branch after a resolution.** The route loops: the request is cancelled but the child is kept, so `item republish` answers `busy`. This is recorded as a Known Limit. The resolution's answer now offers `request cancel` of the lifecycle request.
- **Stored IDs.** A stored lifecycle ID that doesn't parse now answers `recovery_required`.

Verified:

- the fast tier: 4931 passed
- `cli_remote`: 160
- `mutation_remote`: 700
- the five `remote/` targets
- `--test cli`: 474
