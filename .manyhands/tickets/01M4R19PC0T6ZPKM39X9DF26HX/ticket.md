---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4R19PC0T6ZPKM39X9DF26HX"
title: "A blocked synchronization leaves its reservation held until its request is cancelled"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

When a synchronization stops on a blocking cause after it has reserved the
repository (or an item's branch), it keeps the reservation. While the
reservation is held, saves are refused and other synchronizations are busy.
Sending the same request again does not release it while the cause stands. The
exit is to cancel the request and call it once more, or to deal with the cause
and send the same request. For two of the causes the result offers no recovery
action at all.

## Evidence

Found on 2026-10-08 as open case 3 of "The Journal Defect" in the design
`docs/plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md`
(also listed under its known limits), and reproduced by Task 13 of Wave 03
Cycle F2 (ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`, branch
`manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`). The behaviour is that of the
synchronization code as it stood before the Cycle.

**Reproduced by a test that was run.** `pin_a_held_reservation`, in
`tests/mutation_remote.rs`, runs the same sequence after five causes:
`merge_required`, `remote_branch_deleted`, `push_rejected`,
`host_approval_required` and `unlock_required`. Task 13's report
(`task-13-report.md`, "Open case 3 — reproduced") records the same answers for
all five:

1. The result is `blocked`, publication `pending`, and the request record stays
   `accepted`. The reservation is still active.
2. A save on the repository is `recovery_required` with nothing recorded, when
   the reservation is the primary branch's (four of the five). An item's
   reservation (`remote_branch_deleted`) refuses only that item's saves with
   `RecoveryRequired`; a save of another item goes through. That is the rule of
   `recovery.rs`, not of the F2 boundary.
3. Another synchronization is `busy`, with nothing recorded.
4. The same request sent again does not release the reservation while the cause
   stands: same code, same operation ID, reservation still held.
5. `request cancel` followed by the same request releases it. The cancel itself
   reports `requested: true, acknowledged: false` and the reservation is still
   held, because no call is running. The next call of the same request returns
   `cancelled`, the reservation is then free and the refused save succeeds.

The exit is therefore `request cancel` plus one more call of the same request,
or the same request once the cause is dealt with (shown for an approved host, a
server that takes the key again, a stall that clears, and a yield).
`operation abandon` is not an exit and is not offered.

**By the design and the report, not by a separate run:** today the
`merge_required` and `remote_branch_deleted` results offer no recovery action
at all (`first.recovery` is asserted empty for the merge case in the test), although the
reservation is held. Manyhands F2 never merges, so `merge_required` has no
forward path inside the F2 boundary: the only ways on are to cancel the request,
or to merge by other means and send the request again.

## Who is affected today

Anyone who runs a repository or item synchronization that stops on one of these
causes, and any front end that follows the offered actions. For `merge_required`
and `remote_branch_deleted` a client has no action to follow, and a user who
does not know the cancel-and-call-again sequence finds saves refused with
`recovery_required` and other synchronizations `busy`, with no stated reason
tied to the earlier request.

## Proposed direction

Decide where the release belongs. A blocked synchronization that cannot go on
without outside action could release its reservation when it returns, and be
taken up again by the same request when the cause is gone. Whatever the choice,
every blocked result that leaves the reservation held should offer the way out
that works, so that no result of a held reservation is without an action.

## Open points for planning

- Whether a blocked synchronization should release its reservation itself.
- Whether the `merge_required` and `remote_branch_deleted` results should offer
  `request.cancel`.
- What a save should answer while the reservation is held, and whether it should
  name the request that holds it.
- Whether `operation abandon` should ever apply to a held synchronization
  reservation.
