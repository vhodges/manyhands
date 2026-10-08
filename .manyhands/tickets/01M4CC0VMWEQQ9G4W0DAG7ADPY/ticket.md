---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
title: "Wave 03 C4: CLI Discussion, Synchronization And Conflict Recovery"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "C4"
---

Let CLI users collaborate deliberately and recover incomplete work.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c4-cli-discussion-synchronization-and-conflict-recovery).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-cli-04-discussion-and-sync.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 06 and 07 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and C1–C3.

## Exit Evidence

Real two-clone SSH tests cover comment publish success/failure,
ordered replies, one saved comment after retry, clean/divergent sync, stale
resolutions, external noncanonical repair and no secret/raw-server output.
Ctrl-C and lost stdout preserve recoverable effects and meaningful exit status.
Switch ticket comment dogfooding to the CLI when supported.
