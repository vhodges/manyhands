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
- [x] Explicit implementation authorization (2026-10-11): Tasks 0–8, rebase,
  local commits and canonical ticket/comment/ledger updates; execute Task 0
  only in this session, then return for independent spec and quality reviews.

The owner approved the planning documents and D1–D4 on 2026-10-11, then
separately authorized implementation and local commits. The
[execution ledger](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md)
records the preflight, the decision matrix, the internal rulings and Task 0
execution evidence.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current. Cycles 01–07 are
present on the planning base `d1ca887`. Implementation Task 0 repeats the
rebase, records the decisions, runs the characterizations named in the plan
and the baseline.

## Implementation Progress

- [x] Task 0: fresh fetch/rebase and ancestry/clean-status checks; authorization
  and D1–D4 recorded; caller compatibility re-scan; all five throwaway
  characterizations executed and removed; all five Devenv baseline commands
  passed. No design contradiction or blocker found.
- [x] Independent Task 0 spec review (2026-10-11):
  `ses_ed4470e17ffeiE6Bsc6av876RR`; spec compliant, no blockers.
- [x] Independent Task 0 quality review (2026-10-11):
  `ses_ed443ab4effebJy3h12aBe4kYO`; approved, zero Critical/Important/Minor issues.

Task 0 is completed and independently reviewed. **Paused after Task 0 and
before Task 1**, per owner instruction on 2026-10-11. Task 1 has not started;
wait for owner direction to resume. Ticket remains open. Native matrix evidence
remains pending separate authorization.

## Exit Evidence

Real remote tests prove clean updates and new-context materialization occur
once, while dirty, divergent, conflicted, malformed, inaccessible,
unrecognized, renamed, and remotely deleted contexts remain unchanged and
actionable.
