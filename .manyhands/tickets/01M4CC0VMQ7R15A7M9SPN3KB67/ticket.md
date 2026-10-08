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

- [x] Create `docs/Cycles/wave-03-foundation-01-read-boundary-and-results.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.
- [x] Independent review of the three documents; revisions made.
- [x] Product owner decides the six open decisions in the Cycle document
  (2026-10-07, all as recommended).
- [x] Product owner approves the Cycle, design and plan (2026-10-07).
- [x] Defect ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` (one effective copy per item) merged to main (`60b0324`, 2026-10-07).
- [x] Implementation authorization (2026-10-07: implement by subagent-driven
  development; local commits only. 2026-10-08: push of this ticket branch).

Planning artifacts:
[Cycle](../../../docs/Cycles/wave-03-foundation-01-read-boundary-and-results.md),
[design](../../../docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md),
[plan](../../../docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-implementation.md).

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

## Review-Ready (2026-10-08)

Implementation is complete at `8d05ab2` and the Cycle is ready for the
product owner's review. The full gate passed there, every acceptance row is
met on Linux, and the whole branch was independently reviewed. The
[execution ledger](../../../docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md)
holds the rulings, the acceptance table and the evidence.

Not authorized and not done: pull request, merge, ticket closure, worktree
cleanup.

## Open Obligations

- Native execution on Windows and macOS for the eight read test targets, and
  native path matching. The non-Unix file and configuration readers have been
  compiled once and never run.
- Conflict inspection: the first of C4 and D5.
- Polling: `next_eligible_at` is always null; no outcome time or history.
- Empty folders are not listed: F2.
- `created_by` is read and nothing writes it: F2.
- `operation.resume` and new result codes join the registries: F2.

## Open With The Product Owner

- A compatibility rule for the closed v1 schemas.
- Nested or flat comment replies.
- Amendments to the Cycle, Wave and RFC documents that the rulings made
  inaccurate.
- Ten defect and follow-up tickets listed in the ledger, not yet raised.
