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

- [x] Create `docs/Cycles/wave-03-cli-03-authoring-and-discovery.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and C1–C2.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC is present through main merged into
this ticket branch; ticket branches are merged, not rebased.

Ticket create and save take `deps` and `parent`; C3 adds `ticket slug-assign`
and optional initials on ticket creation/slug assignment. Initials on
`repo identity-set` and the prefix on `repo create`/`enable` shipped in C2.
The six Wave 03 dependency saves, value checks and target pushes completed with
owner authorization; evidence is recorded in C3 commit `8da3890`. The dogfooding
workflow switch is a separate Task 9 documentation step with its own review.

## Exit Evidence

Offline checkpoints/no-ops, stale writes, invalid/malformed
repair, adoption retry, move collision, index loss/corruption and commit-success/
refresh-failure preserve actual Git/filesystem state. New JSON schemas and
documented command examples pass. Switch ticket create/update dogfooding to the
CLI when those operations meet this gate.
