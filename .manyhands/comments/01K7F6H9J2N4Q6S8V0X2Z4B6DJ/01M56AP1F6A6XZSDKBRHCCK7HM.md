---
id: 01M56AP1F6A6XZSDKBRHCCK7HM
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DJ
created_at: 2026-10-18T01:40:39Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 5 checkpoint: the `Close` reservation arms, ticket retirement and
closure completion. The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Reservation.** A `Close` slot holds on cancel or yield like `Promote`.
  `finish_parent` finishes a closure only when it is complete and its ticket
  context is retired at the merged context (adopted, else checkpoint, else
  synchronized context). `restart_remote_lifecycle` fences the owner epoch
  and refuses an ended closure parent.
- **Child fence.** `ChildExecution` admits a ticket context target only for
  a closure parent, and reads that parent from `ticket_closures`.
- **Retirement.** `RetirementEvidence::matches(kind, ..)` replaces
  `matches_promotion`. Closure completion uses it with `Ticket` and the
  merged context.
- **Remote delete.** Named edit 5: the exact delete mapping is generic over
  document and ticket contexts. The runner's cleanup uses the merged
  context. The delete guard is the context child's publication, or, when no
  child ran, the observed tracking OID (R-T3-3), through a record accessor.
  Promotion is unchanged.
- **Gate.** fmt and Clippy are clean. `--lib remote::reservation` passes 52,
  `cleanup` 45, `promotion` 199, `closure` 126 and `transport` 38.
  `remote_reservation` passes 13, `remote_polling` 84 SSH cases and
  `document_promotion` 74 SSH cases, unchanged.
- **To review.** I added a closure-specific remote-deletion rule: no context
  attempt and no tracking context means no delete. Without it, a contained,
  open closure with no remote branch would stop at recovery.
