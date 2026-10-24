---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMZ21Z79B09YQRA0P7J"
title: "Wave 03 D2: Recoverable Drafts And External-Change Handling"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "D2"
---

Establish preservation of unsaved work before editor integration.

Desktop track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#d2-recoverable-drafts-and-external-change-handling).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [x] Create `docs/Cycles/wave-03-desktop-02-recoverable-drafts.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and D1. D2 consumes only F1 and may start when F1 exits if capacity allows.

## Exit Evidence

Real-file crash/partial-write/access/migration cases preserve
the last successfully flushed draft. Save completing while newer text exists
retires only the saved revision. Repository/cache removal does not delete drafts;
stale or vanished targets remain recoverable and never silently overwritten.
