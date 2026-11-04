---
slug: vh-30s0x
id: 01M6K06PC2HDVPJQ496ZK2DCZX
title: item sync answers recovery_required after a refused first publication
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Found by C5 Task 12 (ledger, "Task 12 rulings: the two blockers, and the C4 finding"); C4 behaviour, not walked further in C5. When the remote refuses a context's first publication, the same `item sync` request, sent again once the remote accepts, answers `recovery_required` and publishes nothing.

Expected: the same request publishes once the remote accepts, as C4's retry contract says.

Acceptance: reproduce through the binary over SSH (two clones), establish whether the fixture's unborn HEAD contributes, fix, and pin the retry with one push.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
