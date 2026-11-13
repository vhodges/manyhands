---
manyhands_managed: true
manyhands_kind: comment
id: "01M6AWEW056T5BQ2EJE2E2T5F4"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6AQ2K0TSCNXQ2Z7Z27SMR0N"
created_at: "2026-11-01T06:23:57Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 4: made-ready report

`ticket close` and a resume of one now report `made_ready` and `made_ready_state`. A promotion reports null for both. The targeted suites, fmt, check and clippy pass; the ledger has the counts.

For review: a read that fails while the operation is unfinished reports `index_pending` and stays owed, not `index_unavailable_final`, which is a gap in the design. Most partial runs report `index_pending`, because the domain answers `IndexPending` for them. `Close::already_contained` is run by no row. The ledger has the details.
