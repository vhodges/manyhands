---
manyhands_managed: true
manyhands_kind: ticket
id: "01M57SBA7MKR9BZR7PYECSJ05G"
title: "2.10 follow up"
type: "task"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
---

Loose ends from Wave 02 Cycle 10 (confirmed ticket closure, ticket
`01K7F6H9J2N4Q6S8V0X2Z4B6DJ`). None of them blocked closing that Cycle. The
reasoning for each one is in the "Follow-ups" section of the
[Cycle 10 execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

## Items

- [ ] **Closure wording in child execution (R-T6B-3).** `ChildExecution`
  mismatch errors use promotion wording when the parent is a ticket closure.
  Word them for the actual parent.
- [ ] **`save_ticket` refusal order.** With a document target, `save_ticket`
  provisions a document context before it refuses. It should refuse first.
  This predates Cycle 10.
- [ ] **Duplicated record logic.** The `terminal` and busy-supersede logic is
  duplicated between `promotion/record.rs` and `closure/record.rs`. Move it
  into `promotion/lifecycle_record.rs` with the rest of the shared code.
- [ ] **Raw-store negative control.** The ticket-closure privacy test checks
  only that the email is absent from raw storage. Extend it to the other
  fields records must never hold: `closed_by`, titles, bodies, endpoints and
  secrets.
- [ ] **Differing closures in a true race (R-T8-1 alternative).** When two
  sides close the same ticket with different closures, merge/sync classifies
  the conflict `EligibleCanonical` and every in-product resolution fails
  `ValidationFailed`. Classify it `ExternalResolutionRequired` instead. This
  touches Cycle 06 merge/sync code.
- [ ] **Local context synchronization.** Add a local (no-remote) "synchronize
  context with primary" operation, so a user has an in-product way out of
  every checkpoint-merge refusal. Today the documented way out is a manual
  local merge of primary into the ticket context.

## Exit Evidence

Each item is either done with a test that pins it, or explicitly dropped by
the owner with the reason recorded on this ticket.
