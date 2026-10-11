---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
title: "Wave 02 Cycle 08: One-Shot Polling And Materialization"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "08"
---

Safely observe a remote once, update clean local state, and materialize newly
discovered conforming shared contexts.

## Planning

- [x] Create [Cycle](../../../docs/Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md).
- [x] Create [design](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md)
  and [detailed implementation plan](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md).
- [x] Record planning/rebase and skill-based self-review checkpoints as comments.
- [x] Owner answers to the four decisions in the Cycle document, all as
  recommended (2026-10-11): keep stale tracking refs; resume only a
  branch-only materialization; refuse to poll during a pending conflict; a
  local action during an automatic poll requests yield and gets `RepositoryBusy`.
- [x] User approval of Cycle, design, and implementation plan (2026-10-11).
- [ ] Explicit implementation authorization.

The owner approved the planning documents on 2026-10-11 and authorized the
local planning commit. Implementation is not yet authorized. The
[execution ledger](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md)
records the preflight, the decision matrix and the internal rulings.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current. Cycles 01–07 are
present on the planning base `d1ca887`. Implementation Task 0 repeats the
rebase, records the decisions, runs the characterizations named in the plan
and the baseline.

## Exit Evidence

Real remote tests prove clean updates and new-context materialization occur
once, while dirty, divergent, conflicted, malformed, inaccessible,
unrecognized, renamed, and remotely deleted contexts remain unchanged and
actionable.
