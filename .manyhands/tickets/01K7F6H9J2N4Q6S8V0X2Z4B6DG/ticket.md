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
- [x] Owner resume authorization conveyed by controller (2026-10-13): remaining
  execution resumed; this implementer assigned Task 1 only, inline, no subagents,
  local task commit and bookkeeping, then independent spec and quality review.
- [x] Fresh Task 2 assignment (2026-10-13): Task 2 only in this ticket worktree,
  no subagents, one local task commit and canonical bookkeeping, then return
  for the controller's independent reviews. Resume rebase evidence trusted.

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
- [x] Task 1: nullable UTC attempt/deadline migration and accessors, pure policy
  and retry-deadline functions, transactional persisted rollback clamp, existing
  status deadline field and privacy scan. Red recorded before behavior; focused
  tests and all four required Rust checks passed (1,581 tests, 1 existing ignored).
- [x] Independent Task 1 spec review: `ses_eca3c077effeAOiqWMn8w5xeR5`, compliant.
- [x] Independent Task 1 quality review: `ses_eca397d08ffeZI2d2YhlbUJ99h`, approved;
  minor direct corrupt timestamp read test carried into Task 2.
- [x] Task 2: poll-kind reservation/local stage, atomic timestamps, D3/D4 gates;
  prior minor direct corrupt timestamp read coverage resolved. Recorded RED then
  focused GREEN and all four Rust checks (1,594 passed, 1 existing ignored).
- [x] Independent Task 2 spec review: `ses_ec9ebcd99ffekVz7ijwoL63NJx`, compliant.
- [x] Independent Task 2 quality review: `ses_ec9e9909cffeS4kt0Z0ENmyjss`, approved;
  minor suggested direct-SQL kind immutability regression carried to final review.
- [x] Task 3: exact parsed poll fetch set, guarded tracking refs and sticky stop
  hook; RED and corrected stop-fixture behavioral RED recorded. GREEN: 31 focused
  library, 9 poll SSH, 114 entire transport cases; four Rust checks passed
  (1,606 tests, 0 failures, 1 existing ignored). Combined frozen/automatic scope.
- [x] Independent Task 3 spec review: `ses_ec9b35ee6ffeBdb0HkkfCCp9R4`, passed.
- [x] Independent Task 3 quality review: `ses_ec9b0a1d9ffeWP162PBxjUK6l4`,
  controller reports cumulative approval after atomic-absence correction through
  `34b97c178a48b39b6ae32e30de52ff5722d1dd06`; prior Important resolved.
- [ ] Task 4: blocked for owner amendment (2026-10-13). Meaningful test-first
  discovery-agreement RED: 8 passed, 1 failed, 523 filtered. Existing discovery
  accepts a valid executable document tree that poll must reject; two-way
  agreement conflicts with visibility-only changes. Probe removed, Rust unchanged.

Task 0 is completed and independently reviewed. The owner resumed execution on
2026-10-13; the controller's fresh fetch/rebase was a clean no-op at main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`, before/after ticket HEAD
`4c1590d73b9dbab17030670989bd8825328f1f55`. Task 1 is implemented and independently
reviewed. Task 2 is implemented and independently reviewed. Fresh Task 3 assignment
authorizes Task 3 only, inline, no subagents, one local commit and bookkeeping.
Controller reports Task 3 spec and quality approval of the cumulative corrected
range. Fresh implementer assigned Task 4 only, no subagents. Task 4 stopped at
confirmed discovery-agreement contradiction before production work; owner
amendment required, recorded in the ledger and comment `01M4VD7G36JQ3T0D3VXEX7YDTQ`.
Deferred Task 2 direct-SQL kind-immutability minor remains for final review.
Tasks 5–8 have not started. Ticket remains
open; native matrix evidence remains pending separate authorization.

## Exit Evidence

Real remote tests prove clean updates and new-context materialization occur
once, while dirty, divergent, conflicted, malformed, inaccessible,
unrecognized, renamed, and remotely deleted contexts remain unchanged and
actionable.
