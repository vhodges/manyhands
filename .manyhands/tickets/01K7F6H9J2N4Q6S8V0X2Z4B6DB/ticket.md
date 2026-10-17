---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
title: "Wave 02 Cycle 03: Authenticated SSH Transport"
type: "cycle"
status: "closed"
project: "manyhands"
team: "core"
wave: "02"
cycle: "03"
---

Bind the selected shared key and approved host-verification policy to `git2`
remote callbacks and prove authenticated transport against a real fixture.

## Planning

- [x] Prepare the [Cycle contract](../../../docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md).
- [x] Prepare the [design](../../../docs/plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-design.md).
- [x] Prepare the [implementation plan](../../../docs/plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-implementation.md)
  and planning checkpoint comment.
- [x] Obtain approval of the Cycle, design, and plan: user approved on 2026-10-05
  and selected subagent-driven development.
- [x] Implement and record each task's verification and decisions as comments.
- [x] Complete local final gates and independent whole-branch review/fix review.
- [x] Obtain native CI evidence after authorized publication: all five targets
  passed on `e75e768` in run 37390262561.

Implementation status: locally verified, independently reviewed, and native CI
verified on all five targets. Temporary investigation diagnostics are removed; permanent regressions remain.
The user approved scope, design, and plan on 2026-10-05, including Q1–Q3 and
10,000/30,000 ms timeout defaults and the documented per-address/per-blocking-call
backend limits. The pre-thread bootstrap and test-host design were implemented
before the connection-driver task. Task/review state is recorded in
the plan-specific execution ledger and ticket comments; the ticket is closed
after the merged-PR reconciliation below.

Latest Rust revision `bbeef25` passes all four required Devenv gates: 585 tests,
including 31 Unix SSH fixture and 45 transport cases, with no failures or ignored
tests. CLI smoke exits zero; desktop launched on an active display and was
deliberately stopped after successful startup. All task and final review findings
are resolved. See the Cycle's final verification table for contract evidence.

## Entry Gate

The source RFCs and Wave are approved. Cycle 02's closing comment records
approved PR #8, 480 passing tests, and successful native CI on all five targets.
Cycle 03 artifacts were approved, the current-main preflight repeated, and all
required local baseline checks passed before implementation (480 tests).

Planning rebase: fetched origin/main at `89ce24d`; included local main at
`a21a31a` (four additional tooling/skills/research commits). Ticket HEAD moved
from `edb5ea9` to `305c0f0` without conflicts; both ancestry checks passed.

## Exit Evidence

A real authenticated fetch or equivalent approved server fixture succeeds with
the selected key; wrong key, locked key, cancelled unlock, host-verification,
and network failures are typed, redacted, and non-mutating before transfer.
The plan additionally proves real push, host-pin precedence, imported backend
validation, rejection-driven session eviction, and native encrypted Ed25519 use.
Post-transfer failure recovery belongs to subsequent lifecycle Cycles.

Publication: [PR #9](https://github.com/vhodges/manyhands/pull/9) merged into
`main` as `5f5bac0` on 2026-10-06. All five native targets passed on the final
published head `dbda637` in [run 37391879620](https://github.com/vhodges/manyhands/actions/runs/37391879620).
The macOS signal-isolation correction passed 90 investigative probe cases and
three permanent regressions before temporary diagnostic cleanup. Cleanup
`bbeef25` passed independent review and all required local gates; the final
native log confirms the permanent regressions remain and temporary diagnostics
are absent. The ticket is closed in the post-merge reconciliation requested by
the user.
