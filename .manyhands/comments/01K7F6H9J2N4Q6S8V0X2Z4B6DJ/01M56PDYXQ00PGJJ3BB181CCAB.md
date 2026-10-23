---
id: 01M56PDYXQ00PGJJ3BB181CCAB
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DJ
created_at: 2026-10-18T05:05:49Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 7 checkpoint: the domain refuses new writes to a closed ticket (C5).
The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Guard.** `require_ticket_open` reads the ticket from the local primary
  branch tip and, when it exists, the context branch tip. It never reads a
  working tree. It decides "closed" with the closure observation's own
  `file_closure`, so the guard and `AlreadyClosed` cannot disagree. A
  refusal is `RepositoryErrorKind::TicketClosed`, which maps to
  `ticket_closed` in mutations and reads.
- **Call sites.** Ticket saves (relationship and short-code writes go
  through the same path), public `prepare_context` and comment checkpoints
  for a ticket with Edit intent. Each is checked under the lease, after
  `begin_lifecycle`, for a new row only. A refused row ends `rejected` with
  no branch, worktree or file made. Resumed rows, creation, repair and
  writes to tickets that depend on a closed ticket are not refused.
- **Named edits.** `ticket_edit_preserves_unknown_closure_and_exact_body`
  is now a refusal test (edit 1). Both error-kind lists gain `TicketClosed`
  (edit 2).
- **Gate.** See the ledger for per-target counts. fmt and Clippy are clean.
