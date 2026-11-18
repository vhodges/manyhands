---
manyhands_managed: true
manyhands_kind: ticket
id: "01M57SKVF6TQ3QM6AXJ35JGGSY"
title: "2.10 follow up: closure recovery gaps before C4, C5 and D5"
type: "task"
status: "closed"
project: "manyhands"
team: "core"
closed_at: 2026-10-27T12:51:51Z
closed_by: Vince Hodges <vince@imbas.ca>
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

- [x] **Differing closures in a true race (R-T8-1 alternative).** Fresh
  incompatible all-side closure pairs now make the whole conflict set
  `ExternalResolutionRequired`, with no write or false in-product resolution
  offer. Immutable provenance validation remains; no winning-closure policy is added.
- [x] **Local context synchronization.** Production library observation,
  start, inspection, explicit canonical resolution and resume synchronize committed
  local primary into an existing document/ticket context without network work or
  primary mutation. The approved supported checkpoint-conflict workflow completes
  through local synchronization and explicit original closure resume, preserving
  closure authority (R-T4-1). Unsupported/immutable conflicts and ambiguous backend
  states retain their documented limits; frontend adapters remain downstream.
- [x] **Closure wording in child execution (R-T6B-3).** `ChildExecution`
  mismatch paths now name their actual lifecycle parent, retaining promotion
  wording for promotion and closure wording for closure.

## Exit Evidence

A race in which two clones close the same ticket differently classifies as
`ExternalResolutionRequired`, with no in-product resolution offered. A
closure refused for a checkpoint-merge conflict completes after the new local
synchronization, with no manual Git step. A closure child mismatch error
names ticket closure, not promotion.

## Review-Ready Checkpoint (2026-10-27)

Tasks 0–9 are complete. The controller reports whole-branch SPEC and QUALITY
approval with **0 Blocking / 0 Important / 0 Minor** findings over
`0464c42dae19a6dde000c7e7660e82762a41fada..f92bc00912444cdd2774a00f521248f9be8ece1d`.
The final live-prerequisite inspection correction is included. Local A13 gates
and independent review are met; full evidence and separately scoped pre/post-fix
counts are in the
[execution ledger](../../../docs/plans/2026-10-25-closure-recovery-gate-execution.md#task-9-final-review-ready-checkpoint).

This status edit follows the authorized `working-on-a-ticket` metadata exception.
The ticket remains **open**. No owner final acceptance, lifecycle closure or merge
is claimed. C4/C5/D5's merge prerequisite remains in force. CLI verbs, human-mode
conflict artifacts, desktop in-app editing and the remaining remote/lifecycle
adapters are downstream. Runtime evidence is Linux-only, with cooperative writers,
operator proof for ambiguous backend effects and exact original R source when
required. Full-suite/native verification was not run. PR, lifecycle close/merge,
native dispatch and cleanup require separate authorization.

## Owner Acceptance And Closure (2026-10-27)

The owner accepted the review-ready change in chat and instructed:
"Mark the ticket closed, commit, merge back to main locally and push to origin".
The canonical closure fields above record that acceptance under the
`working-on-a-ticket` metadata exception. The closure commit changes metadata
and records only; the independently reviewed and verified Rust tree is unchanged.
Local main integration and publication are authorized by the same instruction.
See comment `01M5YPNHG6HW9GJJJJMGR6AQ68` for the pre-merge checkpoint.
