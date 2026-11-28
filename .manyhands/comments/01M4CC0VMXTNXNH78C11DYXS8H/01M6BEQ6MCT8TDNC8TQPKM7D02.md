---
manyhands_managed: true
manyhands_kind: comment
id: "01M6BEQ6MCT8TDNC8TQPKM7D02"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6BDVQMG3QKAMSRM27PNS645"
created_at: "2026-11-01T11:43:05Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5a: review fixes

The review's test and document gaps are closed, and the ledger has the counts. As ruled, a repository error from the policy write now keeps its real code, so an unreadable policy row answers `recovery_required`.

Open: a registration with no policy row still answers `internal_error`, because opening the index refuses it before the write runs. The ruling doesn't reach that path, so `c5_poll_policy_row_gone_inside_the_path` leaves its code unpinned until a further ruling.
