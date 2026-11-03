---
cycle: C1
wave: '03'
id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
title: 'Wave 03 C1: CLI Shell And Inspection'
type: cycle
status: closed
project: manyhands
team: core
closed_at: 2026-10-19T17:08:51Z
closed_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: ticket
---

Make existing state inspectable through the CLI without enabling
mutation commands prematurely.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c1-cli-shell-and-inspection).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [x] Create `docs/Cycles/wave-03-cli-01-shell-and-inspection.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1 and F2. C1 consumes only F1 and may start when F1 exits if capacity allows.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC reaches this branch when it is
rebased onto main.

Add the relationship read verbs: `ticket ready`, `blocked`, `deps`,
`children`, `cycles`, `plan`, `critical-path` and `find`. A short code passed
as `--id` is rejected as an invalid ID.

## Exit Evidence

Real-repository read tests through the binary, golden
schemas and human/JSON error tests; no display or desktop dependency; no
canonical mutation or resident process.

## Closure — 2026-10-19

The owner approved marking this ticket done, committing its closure, merging
the ticket branch into local main and pushing main, removing the local and
remote ticket branches and local worktree, and dispatching one native Build
run on GitHub. The owner will monitor that run.

Tasks 0–8 are complete. Independent whole-branch spec and quality reviews
passed with no remaining findings. The final Devenv gate on `309c9e0` passed
3,468 tests/cases, with zero failures and one existing opt-in test ignored;
check, fmt and all-target/all-feature Clippy passed. CLI and bounded desktop
launch smoke checks passed. The approved fixture-only correction is recorded
as Cycle decision 14. The execution ledger contains the full evidence.

Native CI runs after the merge on main; this closure does not claim a native
pass. The CLI's ticket-closure and comment-writing commands are still planned,
so this lifecycle update uses the canonical filesystem records.
