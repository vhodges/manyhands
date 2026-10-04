---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DF"
title: "Wave 02 Cycle 07: Comment Publication"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "07"
---

Make comment submission the required compound local checkpoint and immediate
item-context synchronization action.

## Planning

- [ ] Create `docs/Cycles/wave-02-cycle-07-comment-publication.md`.
- [ ] Create the implementation plan and record its checkpoints as ticket
  comments.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

A new root comment and reply are published when possible, remain discoverable
locally when no remote or remote recovery failure exists, and publish on a
later retry without duplicate files or commits.
