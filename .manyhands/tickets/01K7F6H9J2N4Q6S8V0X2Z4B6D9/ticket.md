---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6D9"
title: "Wave 02 Cycle 01: Shared-Key Registry"
type: "cycle"
status: "closed"
project: "manyhands"
team: "core"
wave: "02"
cycle: "01"
---

Establish application-local, non-secret SSH-key registration and single
shared-key selection without using a network transport.

## Planning

- [x] Create `docs/Cycles/wave-02-cycle-01-shared-key-registry.md`.
- [x] Create the implementation plan and record its checkpoints as ticket
  comments.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Tests prove canonical metadata uniqueness, one shared selection, imported-key
non-deletion, generated-key deletion confirmation, corrupt or missing key
recovery guidance, and absence of secret persistence.
