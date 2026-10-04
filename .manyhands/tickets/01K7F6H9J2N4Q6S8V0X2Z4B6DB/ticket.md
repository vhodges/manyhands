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

- [ ] Create `docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md`.
- [ ] Create the implementation plan and record its checkpoints as ticket
  comments.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

A real authenticated fetch or equivalent approved server fixture succeeds with
the selected key; wrong key, locked key, cancelled unlock, host-verification,
and network failures are typed, redacted, and non-mutating.
