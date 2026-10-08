---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
title: "Wave 03 C1: CLI Shell And Inspection"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "C1"
---

Make existing state inspectable through the CLI without enabling
mutation commands prematurely.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c1-cli-shell-and-inspection).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-cli-01-shell-and-inspection.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1 and F2. C1 consumes only F1 and may start when F1 exits if capacity allows.

## Exit Evidence

Real-repository read tests through the binary, golden
schemas and human/JSON error tests; no display or desktop dependency; no
canonical mutation or resident process.
