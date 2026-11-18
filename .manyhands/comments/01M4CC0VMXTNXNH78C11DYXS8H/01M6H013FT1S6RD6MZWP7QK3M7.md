---
manyhands_managed: true
manyhands_kind: comment
id: "01M6H013FT1S6RD6MZWP7QK3M7"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6GN7BBXWH8N1DTKPBZJFPZP"
created_at: "2026-11-03T15:21:47Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Decision 34: the conflict verbs reach a lifecycle's child

`conflict show` and `conflict resolve` now take a promotion's or closure's operation ID for its context child, as a comment's names its synchronization. The child's own ID works too, and a request is the same by either. C4's read and resolution are otherwise unchanged. The request that holds the parent is the child's holder, so its `request cancel` stops the resolution's publication. The lifecycle's human answer names both verbs by the parent's ID.

- **Walk.** Task 13b's retained-conflict close now finishes through the binary over SSH: `conflict show`, `conflict resolve`, then the same `ticket close`, `ok`, with one merge and the closure fields on primary and the remote.
- **Poll plan failure.** `poll once` doesn't fence, so the same route finishes the close after it.
- **Key select.** Still stuck, and kept as a Known Limit: a conflict is reacquired only under its generation.
- **Not reproduced.** Task 13b's report that `conflict show` of the child answered `invalid_input` doesn't happen on this tree or the old one.

Verified: 12 library rows for both verbs, 3 walks, the fast tier (4919), `cli_remote` (160), `mutation_remote` (696), and the five remote targets.
