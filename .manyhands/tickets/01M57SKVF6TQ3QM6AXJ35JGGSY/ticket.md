---
manyhands_managed: true
manyhands_kind: ticket
id: "01M57SKVF6TQ3QM6AXJ35JGGSY"
title: "2.10 follow up: closure recovery gaps before C4, C5 and D5"
type: "task"
status: "waiting"
project: "manyhands"
team: "core"
wave: "02"
---

Library changes left over from Wave 02 Cycle 10 (confirmed ticket closure,
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`) that Wave 03 front ends depend on. This
ticket was split from the "2.10 follow up" ticket (`01M57SBA7MKR9BZR7PYECSJ05G`),
which keeps the internal cleanup items. The reasoning for each item is in the
"Follow-ups" section of the
[Cycle 10 execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

## Why This Gates Wave 03

Wave 03 track rule 5 forbids a front-end Cycle from working around a library
gap. The fix lands on main first, as its own library-only change. Without
these items:

- C4 (CLI) and D5 (desktop) conflict recovery would offer an in-product
  resolution for a conflict no resolution can fix.
- C5 and D5 ticket close would leave a user with a manual Git merge as the
  only way out of a refused checkpoint merge. The Wave 03 outcome promises
  closing tickets "without operating Git manually".

Land it before the first of C4, C5 or D5 starts. C1–C3 and D1–D4 do not need
it. Wave 03 rule 6 has whichever of C5 or D5 comes first land the closure
binding as a shared-library change. This ticket can go ahead of that binding
or alongside it.

## Items

- [ ] **Differing closures in a true race (R-T8-1 alternative).** When two
  sides close the same ticket with different closures, merge/sync classifies
  the conflict `EligibleCanonical`, and every in-product resolution fails
  `ValidationFailed`. Classify it `ExternalResolutionRequired`, so front ends
  show outside-repair guidance. This touches Cycle 06 merge/sync code.
- [ ] **Local context synchronization.** Add a local (no-remote)
  "synchronize context with primary" operation, so a user has an in-product
  way out of every checkpoint-merge refusal. Today the documented way out is
  a manual local merge of primary into the ticket context. Resume already
  adopts a merged context (R-T4-1).
- [ ] **Closure wording in child execution (R-T6B-3).** `ChildExecution`
  mismatch errors use promotion wording when the parent is a ticket closure.
  Word them for the actual parent; C5 and D5 users would see this text.

## Exit Evidence

A race in which two clones close the same ticket differently classifies as
`ExternalResolutionRequired`, with no in-product resolution offered. A
closure refused for a checkpoint-merge conflict completes after the new local
synchronization, with no manual Git step. A closure child mismatch error
names ticket closure, not promotion.
