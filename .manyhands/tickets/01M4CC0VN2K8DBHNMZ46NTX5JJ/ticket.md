---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VN2K8DBHNMZ46NTX5JJ"
title: "Wave 03 D5: Desktop Conflict Recovery, Promotion And Closure"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "D5"
---

Complete deliberate desktop collaboration lifecycles.

Desktop track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#d5-desktop-conflict-recovery-promotion-and-closure).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-desktop-05-recovery-and-completion.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 06, 09 and 10 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and D1–D4.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC reaches this branch when it is
rebased onto main.

A completed close lists the tickets it made ready.

## From F1 Planning (2026-10-07)

Conflict inspection was deferred out of F1, because no conflict record exists
before Wave 02 Cycle 06. Whichever of C4 or D5 is planned first lands that
read in the headless library as a shared-library change under track rule 5,
with the same DTO and redaction rules as the F1 reads.

Decided by the product owner on 2026-10-07 during F1 planning; the
RFC and Wave amendments are on the F1 branch
(`manyhands/ticket/01M4CC0VMQ7R15A7M9SPN3KB67`) and reach main when it merges.

## Exit Evidence

Native keyboard conflict and completion journeys; stale
resolution/confirmation, dirty primary, local-only close/promotion, remote
cleanup rejection, interruption and cleanup-only replay. Preserved drafts and
contexts remain inspectable and no retry repeats a merge or closure checkpoint.
