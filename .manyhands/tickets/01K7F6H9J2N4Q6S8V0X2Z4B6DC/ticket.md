---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DC"
title: "Wave 02 Cycle 04: Remote Observation And Recovery Model"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "04"
---

Make the approved remote-ref protocol and remote lifecycle state durable and
observable before any operation changes local branches.

## Planning

- [x] Create `docs/Cycles/wave-02-cycle-04-remote-observation-and-recovery.md`.
- [x] Create the implementation plan and record its checkpoints as ticket
  comments.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Fixtures prove deterministic remote observations, no secret records, retry
after interruption, safe manual-versus-poll arbitration, and visible
unmaterialized, malformed, and remotely deleted remote states.
