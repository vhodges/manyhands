---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DA"
title: "Wave 02 Cycle 02: Generated Keys And Session Unlock"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "02"
---

Create and safely use generated or imported key material while retaining
passphrases only for the application session.

## Planning

- [x] Create `docs/Cycles/wave-02-cycle-02-generated-keys-and-unlock.md`.
- [x] Create the implementation plan and record its checkpoints as ticket
  comments.
- [x] Approve the Cycle, detailed design, and implementation plan.

Approved planning documents:

- [Cycle scope](../../../docs/Cycles/wave-02-cycle-02-generated-keys-and-unlock.md)
- [Detailed design](../../../docs/plans/2026-10-05-wave-02-cycle-02-generated-keys-and-unlock-design.md)
- [Implementation plan](../../../docs/plans/2026-10-05-wave-02-cycle-02-generated-keys-and-unlock-implementation.md)

The existing ticket branch was rebased onto main at `21eefa4` before planning;
its resulting checkpoint is `d546264`. The user authorized subagent-driven implementation on 2026-10-05.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Tests prove key files follow the approved protection model, passphrases do not
enter persistent state or diagnostics, session unlock is reused only within its
session, and key access failures preserve registrations and report actionable
recovery.
