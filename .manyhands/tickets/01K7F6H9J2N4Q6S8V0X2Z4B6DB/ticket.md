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
- [ ] Implement and record each task's verification and decisions as comments.

Implementation status: authorized and starting with subagent-driven development.
The user approved scope, design, and plan on 2026-10-05, including Q1–Q3 and
10,000/30,000 ms timeout defaults. Resolve safe timeout initialization and backend
coverage before the connection-driver task. Track task/review state in the
plan-specific execution ledger and ticket comments; keep the ticket open.

## Entry Gate

The source RFCs and Wave are approved. Cycle 02's closing comment records
approved PR #8, 480 passing tests, and successful native CI on all five targets.
Before implementation, approve these Cycle 03 artifacts, repeat the current-main
preflight, and run the required local baseline checks. Prior recorded evidence
does not replace a fresh baseline against the implementation checkout.

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
