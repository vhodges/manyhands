---
slug: vh-6ba06
id: 01M73V5N1RK32TJPTW219S4KD6
title: Match a pending retirement row to its in-flight entry by item
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The desktop's in-flight retirement entry has no exit if it never learned its operation: after a hard crash between submitting a retirement and its first progress event, or when a request is left Accepted with no data. Reconciliation skips such an entry, and Resume and Abandon match on the operation. The item stays "Retiring…" and read-only across relaunches.

Follow-up from Wave 03 D6 (ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`); see its execution ledger (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`) and design Known Limits. Filed on the owner's approval, 2026-11-10. Known Limit K-59 (the final Task 12 review's I-1).

## Evidence

- Ending such entries at launch would risk dropping the hold on a row the library created but the desktop has not seen, so D6 left them alone.
- The library's pending-operation list (`OperationDto`) carries the item ID, which D6's design (Fact 33) records as unused.

## Expected

- Reconciliation, Resume and Abandon also match a pending row to an entry by the item (root, kind, id) when the entry has no operation.

## Acceptance

- A desktop test: an entry with no operation, and a complete fresh pending list that lists its item, keeps the entry; one that lists none ends it once the worker and request model hold nothing for it.
- Resume and Abandon work for such an entry after a relaunch.