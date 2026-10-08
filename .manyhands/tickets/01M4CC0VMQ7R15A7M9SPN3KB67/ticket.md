---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMQ7R15A7M9SPN3KB67"
title: "Wave 03 F1: Headless Read Boundary And Result Model"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "F1"
---

Establish the headless front-end boundary both tracks read
through, so neither front end scrapes SQLite, Git or the filesystem itself.

Foundation of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#f1-headless-read-boundary-and-result-model).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-foundation-01-read-boundary-and-results.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: None within Wave 03.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC reaches this branch when it is
rebased onto main.

Ticket DTOs carry `slug`, `parent`, `deps`, readiness and relationship
problems. Read services cover ready, blocked, dependencies in both directions,
children, cycles, plan, critical path and find by short code, over index edge
records rebuilt from canonical files. Exit evidence adds: these queries proven
across primary and active worktrees with an unresolved dependency, a merged-in
cycle and a duplicate short code, each reported and none repaired.

## Exit Evidence

Library integration tests against real repositories cover
every read service, typed malformed rows, deterministic ordering and complete
lists. Golden DTO schemas and redaction fixtures are published. An open ticket
whose status text is `closed` is listed as lifecycle-open. No display or GPUI
dependency, no canonical mutation and no resident process.
