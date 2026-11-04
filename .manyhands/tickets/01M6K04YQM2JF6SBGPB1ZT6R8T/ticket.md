---
slug: vh-6rrjs
id: 01M6K04YQM2JF6SBGPB1ZT6R8T
title: Reads should take the latest copy of an item by Git history
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Today a read takes an item's own worktree copy as its effective copy (F1's rule). When copies disagree, the owner's rule (2026-11-02, C5 decision 28's discussion) is that the latest copy is authoritative. C5 applied that only to `poll once` (decision 28).

Expected: reads choose the latest copy of an item by Git history across primary and every context. Diverged copies need a defined answer (report a conflict, or keep the current preference). Unsaved and unpublished edits stay newest.

Acceptance: rows for a primary newer than a context, a context newer than primary, and a diverged pair, through `ticket show`, `ticket ready` and made-ready, with the desktop's reads unchanged except where the rule says so. The index stores no commit per copy today, so the design must say how reads learn history.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
