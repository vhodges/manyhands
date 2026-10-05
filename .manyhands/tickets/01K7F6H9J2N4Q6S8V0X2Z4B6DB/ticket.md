---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
title: "Wave 02 Cycle 03: Authenticated SSH Transport"
type: "cycle"
status: "open"
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
- [ ] Obtain native CI evidence after authorized publication.

Implementation status: locally verified and independently reviewed; native CI pending.
The user approved scope, design, and plan on 2026-10-05, including Q1–Q3 and
10,000/30,000 ms timeout defaults and the documented per-address/per-blocking-call
backend limits. The pre-thread bootstrap and test-host design were implemented
before the connection-driver task. Task/review state is recorded in
the plan-specific execution ledger and ticket comments; the ticket remains open.

Latest Rust revision `ee8153e` passes all four required Devenv gates: 582 tests,
including 28 SSH fixture and 45 transport cases, with no failures or ignored
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

Publication: [PR #9](https://github.com/vhodges/manyhands/pull/9) is open. Native
CI follow-up fixes and an unexplained intermittent macOS lost-response assertion
are tracked in the Cycle and comments. Windows native acceptance remains pending.
