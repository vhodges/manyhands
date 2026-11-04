---
slug: vh-xnhq8
id: 01M6K08364Z6VX2TYVZD86PG1Q
title: Decide whether effective_identity takes one level's complete pair
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
`effective_identity` (`src/repository/remote/sync.rs`) reads `user.name` and `user.email` separately through Git's cascade, while `resolve_identity` takes the first configuration level holding both. The two can choose different identities for one repository. `01M5MK57JKCSQFXBNZ8DBE548K`, folded into C5, asked for this decision and said changing it needs the owner's approval; C5 kept the cascade (R-6).

Task: the owner decides which rule applies; if a change is chosen, align the selectors with rows for split configurations, preserving recovery records and replays.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
