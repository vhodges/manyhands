---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DE"
title: "Wave 02 Cycle 06: Merge And Conflict Recovery"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "06"
---

Extend deliberate synchronization to safely merge divergent shared history and
preserve conflicts for explicit resolution.

## Planning

- [x] Prepare the [Cycle contract](../../../docs/Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md).
- [x] Prepare the [detailed design](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md).
- [x] Prepare the [implementation plan](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md)
  and record planning checkpoints as ticket comments.
- [x] Self-review using `review-cycle-docs`; record decisions/rulings in the
  [planning/execution ledger](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md).
- [x] Owner approves the Cycle, design and implementation plan: 2026-10-07.
- [x] Obtain explicit implementation authorization and execution method: 2026-10-07; sequential subagent-driven execution with local checkpoint commits.
- [ ] Implement, verify, review and record per-task checkpoints.
- [ ] Obtain code-review/PR approval and separate delivery/closure authority.

Cycle, design and implementation plan approved by the owner on 2026-10-07.
Implementation authorization followed in the present session: execute Tasks 0–7
sequentially in this existing worktree with subagent-driven development and local
checkpoint commits. Ticket remains open. Push/PR, CI dispatch, merge, closure
and cleanup remain separately unauthorized.

## Planning Evidence And Decision

Fresh fetched main `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9` is an ancestor
of rebased ticket `8fb768e39835acb7b84cde81df7083a03995b645`; before rebase
HEAD was `2dbd3b4777674e7cd0fab584239e62202f677f9c`. Rebase was conflict-free,
status clean before document edits, changed AGENTS reread, unrelated checkouts
preserved. Full details are in the planning comment and ledger.

Owner answered the self-review scope question on 2026-10-07: mixed canonical
and code/binary/unsupported conflicts require **whole-merge external recovery**.
Manyhands writes no canonical resolution portion of mixed sets; external tools
resolve/commit the merge, then deliberate resume verifies it. Accepted cost:
those canonical portions cannot be resolved in-process. That scoped answer
alone did not grant artifact or implementation approval; subsequent approval
of all three artifacts is recorded above. No material question remains open.

Internal rulings cover two-parent candidates, ordered stage/attempt digests,
auxiliary identity-confirmation replay, strict external-parent proof and a
bounded current-ref integration pass. Backend source self-review found merge
preparation writes ODB blobs; the plan now requires a separate transient-memory
ODB handle and verifies its isolation before import/application under the lease.

Cycle 05 source is on current main; its historical local evidence is not a fresh
Cycle 06 baseline. CI is manual-only; actual native evidence remains pending.
Cycle 05's native deferral does not automatically authorize a Cycle 06 deferral.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Real remote divergence tests prove clean merges retain both histories, conflicts
retain the affected worktree and markers, resolution retry does not repeat
completed network or merge steps, and no local work is lost.
