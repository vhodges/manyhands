---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DF"
title: "Wave 02 Cycle 07: Comment Publication"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "07"
---

Make comment submission the required compound local checkpoint and immediate
item-context synchronization action.

## Planning

- [x] Create [Cycle](../../../docs/Cycles/wave-02-cycle-07-comment-publication.md).
- [x] Create [design](../../../docs/plans/2026-10-07-wave-02-cycle-07-comment-publication-design.md)
  and [detailed implementation plan](../../../docs/plans/2026-10-07-wave-02-cycle-07-comment-publication-implementation.md).
- [x] Record planning/rebase and skill-based self-review checkpoints as comments.
- [x] User approval of Cycle, design, and implementation plan (2026-10-07).
- [x] Rebase onto merged Cycle 06 and amend the plan against it (2026-10-09).
- [x] Ticket `01M4GD0KKXW684QBA49F6EX3WE` (save leaves the Git index current) merged to the base (PR #16, `842fd16`).
- [x] Explicit implementation authorization after dependency gates pass (owner hand-off, 2026-10-10).

The user approved the planning documents on 2026-10-07 and authorized the
approval-state updates and local planning commit. The 2026-10-10 hand-off
authorizes Task 0 rebase, Tasks 1–6 implementation, local commits, local gates
and ticket comments. Task 0 dependency and baseline checks pass; the owner
approved the affected Cycle conflict-scope paragraph on 2026-10-10. This
ticket stays open. The
[execution ledger](../../../docs/plans/2026-10-07-wave-02-cycle-07-comment-publication-execution.md)
records the decision matrix, internal rulings and verification/lifecycle gates.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current. Cycle 06's approved,
reviewed merge/conflict implementation is present on the base since 2026-10-09
(main `5e4fad6`). The index ticket above is merged into the refreshed base
`f87ce81`; the public save → synchronize real-SSH check passes without an
index refresh. No Cycle 06 work will be
duplicated here. Native evidence remains pending
separately authorized execution or explicit owner deferral.

Task 0 found that the Cycle's "saved locally ... for every item" promise during
a pending conflict needs qualification: Cycle 06 refuses new comments inside
the context that owns the pending merge before checkpointing. The proposed
paragraph in the execution ledger preserves that guard and the decided
saved-local/Busy mapping for checkpointable comments in other contexts. The
owner approved that affected paragraph on 2026-10-10; the Task 0 gate is met.

## Implementation Progress

- [x] Task 0 entry gates and owner-approved conflict-scope qualification.
- [x] Task 1 API/checkpoint extraction and independent review (`82623fa`).
- [x] Task 2 durable binding/receipt recovery and independent review (`41422f1`).
- [x] Task 3 context delegation/index mapping and independent review (`b07ca41`).
- [x] Task 4 retry/cancellation and owner-authorized narrow Cycle 06
  applied-but-unreleased cancellation repair; independent source review.
- [x] Task 5 complete real-SSH acceptance and fault/privacy matrix; independent review.
- [x] Task 6 full local gates and independent whole-branch code review.
- [ ] Task 6 authorized five-target native evidence and review-ready handoff.

The public SSH regression exposed terminal cancellation after the resolution
checkpoint but before cleanup. The owner-authorized shared predicate repair now
preserves the recoverable stop, same child/candidate, and terminal clean/released
negatives. Task 5's 41 real-SSH cases and independent review pass. Task 6 reproduced
and fixed the redundant binding-migration marker write behind the concurrent
opening failure; all final local gates and whole-branch source review pass
(library 504, read-items 67, all integration/SSH targets and CLI smoke).
Five-target native execution remains pending separate push/dispatch authorization.
The ticket is open and not review-ready.

## Exit Evidence

A new root comment and reply are published when possible, remain discoverable
locally when no remote or remote recovery failure exists, and publish on a
later retry without duplicate files or commits.
