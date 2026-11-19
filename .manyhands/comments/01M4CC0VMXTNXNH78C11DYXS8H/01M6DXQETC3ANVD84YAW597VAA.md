---
manyhands_managed: true
manyhands_kind: comment
id: "01M6DXQETC3ANVD84YAW597VAA"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6D09X1EKQVEKMVP4H95ET9E"
created_at: "2026-11-02T10:43:51Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Decision 28: the latest copy is authoritative

`poll once` no longer materializes a remote context branch whose tip primary already holds. It reports that target `current`, reusing the disposition, since a new one would change an audited CHECK constraint. Library and CLI rows pin it, and a control shows a branch with a commit primary lacks still materializes. S2: `item sync` of a closed ticket's copy doesn't publish the stale copy. It answers `remote_branch_deleted` when the branch is gone, and when the remote still holds it, it publishes primary's closure. `mutation_contract` has a failure that predates this change, from Task 7, and the review-fix commit adds its reason. The ledger has the detail.
