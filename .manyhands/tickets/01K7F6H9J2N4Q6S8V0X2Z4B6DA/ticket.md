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

- [ ] Create `docs/Cycles/wave-02-cycle-02-generated-keys-and-unlock.md`.
- [ ] Create the implementation plan and record its checkpoints as ticket
  comments.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Tests prove key files follow the approved protection model, passphrases do not
enter persistent state or diagnostics, session unlock is reused only within its
session, and key access failures preserve registrations and report actionable
recovery.
