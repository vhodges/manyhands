---
wave: '03'
cycle: D6
deps:
- 01M4CC0VN2K8DBHNMZ46NTX5JJ
id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
title: 'Wave 03 D6: Desktop Background Polling And Shutdown'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---

Keep discovery current inside the desktop process while preserving
manual priority and unsaved work.

Desktop track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#d6-desktop-background-polling-and-shutdown).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 08 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and D1–D5. Runtime readiness reconciles the final Wave 02 transport startup and cancellation contract before implementation.

## Exit Evidence

Real SSH background update/materialization journey,
exceptional-context preservation, pause versus unlock state, safety against a
concurrent explicit operation from a second process calling the library,
stalled transport with responsive stopping status, configuration changes and
shutdown/restart. No background push/checkpoint/merge/cleanup; no remaining
worker after graceful process exit.
