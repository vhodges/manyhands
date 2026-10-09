---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4H33R34Z7C7EEKTY1ZCT950"
title: "Let the user abandon a pending synchronization conflict"
type: "enhancement"
status: "open"
project: "manyhands"
team: "core"
---

Once a synchronization has installed a conflicted merge, the only way forward is
to complete that exact merge. There is no way to give up on it, and while it is
pending every other synchronization in the repository is refused.

## Evidence

Found on 2026-10-09 by the whole-change review of Wave 02 Cycle 06 (ticket
`01K7F6H9J2N4Q6S8V0X2Z4B6DE`, branch source `50d7de3`). Read from the code, not
yet reproduced by a test; writing that failing test is the first step.

- A new synchronization of any target returns `Busy` while any integration
  step in the repository is `conflict_pending`
  (`src/repository/remote/reservation.rs`, `state::has_pending_conflict`).
- `cancel_remote_operation` is a silent no-op for a released conflict, because
  the operation is parked `interrupted`, which is not an active phase.
- A cancel honoured at a safe point is deliberately non-terminal for a pending
  conflict (Cycle 06 Task 5 fix), so that resolution and external repair stay
  possible.
- If the user runs `git merge --abort`, the same-ID restart returns
  `RecoveryRequired`, inspection returns `ExternalChange`, authoring in that
  context is refused, and all other targets stay `Busy`. The only exit is to
  redo the exact ordered two-parent merge by hand.

The Cycle 06 design says a pending record is kept "so other safe work is not
blocked forever"; today it can be.

## Proposed direction

Add an explicit abandon transition for a pending conflict. It marks the
operation terminal only after observing, under the lease, that the target is
clean at the recorded local parent — that is, the merge was aborted and nothing
of the operation's effect remains — and it retires the operation's own recorded
merge metadata by the existing proven-ownership rule. It never resets, aborts
or deletes on the user's behalf.

## Open points for planning

- Whether abandon also covers a step still `applying` with an installed but
  unrecorded conflict (crash-restart route), which today stays terminal on
  cancel with the merge files left for the user.
- Whether `Busy` should narrow to the same target, so a pending conflict in one
  context does not block synchronizing another.
- The read/status surface: a pending conflict is listed as `interrupted` with
  `Resume` and is not distinguishable from other interrupted operations.
- CLI and desktop entry points for abandon.
