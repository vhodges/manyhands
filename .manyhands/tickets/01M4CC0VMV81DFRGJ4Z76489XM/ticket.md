---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMV81DFRGJ4Z76489XM"
title: "Wave 03 C3: CLI Authoring And Discovery Maintenance"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "C3"
---

Make local document/ticket lifecycle and explicit discovery repair
available for agents and project dogfooding.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c3-cli-authoring-and-discovery-maintenance).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-cli-03-authoring-and-discovery.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and C1–C2.

## Exit Evidence

Offline checkpoints/no-ops, stale writes, invalid/malformed
repair, adoption retry, move collision, index loss/corruption and commit-success/
refresh-failure preserve actual Git/filesystem state. New JSON schemas and
documented command examples pass. Switch ticket create/update dogfooding to the
CLI when those operations meet this gate.
