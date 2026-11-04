---
slug: vh-ew8pt
id: 01M6K06B7H8V7347SRM3XGV7CB
title: Delete a contained context's stale remote branch, guarded by push evidence
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A contained context whose remote branch has no tracking ref keeps that branch on the remote: the lifecycle run has no OID to guard the delete with, so the preview honestly says no remote deletion (C5's withdrawn decision 32; decision 28 makes polls ignore such branches).

Expected: the run deletes that branch, guarded by the context's stored push evidence, and the preview says so.

Acceptance: the deletion is guarded against a branch that moved, a cache-loss recovery row still passes, and the preview matches what the run does.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
