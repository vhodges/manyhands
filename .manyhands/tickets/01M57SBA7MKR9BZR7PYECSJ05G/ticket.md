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

Internal cleanup left over from Wave 02 Cycle 10 (confirmed ticket closure,
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`). None of it blocks either Wave 03 track.
The items Wave 03 depends on were split out on 2026-10-18 into
"2.10 follow up: closure recovery gaps before C4, C5 and D5"
(`01M57SKVF6TQ3QM6AXJ35JGGSY`). The reasoning for each item is in the
"Follow-ups" section of the
[Cycle 10 execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

## Items

- [ ] **`save_ticket` refusal order.** With a document target, `save_ticket`
  provisions a document context before it refuses. It should refuse first.
  This predates Cycle 10, and no front end reaches this path.
- [ ] **Duplicated record logic.** The `terminal` and busy-supersede logic is
  duplicated between `promotion/record.rs` and `closure/record.rs`. Move it
  into `promotion/lifecycle_record.rs` with the rest of the shared code.
- [ ] **Raw-store negative control.** The ticket-closure privacy test checks
  only that the email is absent from raw storage. Extend it to the other
  fields records must never hold: `closed_by`, titles, bodies, endpoints and
  secrets.

## Exit Evidence

Each item is either done with a test that pins it, or explicitly dropped by
the owner with the reason recorded on this ticket.
