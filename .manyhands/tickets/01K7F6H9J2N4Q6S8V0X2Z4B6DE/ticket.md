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

## Task 1 checkpoint

Task 1 backend/API characterization is committed at
`6f65a79c9a0be0f52b5891d5a9b098b6f2ed3682`. Independent review found the
mempack fixture's single HEAD-target assertion insufficient to prove that no
other destination ref storage changed. Valid P1 remediation is committed at
`b9931fd21c5a724028c503d5f3c09bb4b696879f` and
`b80ba384bb0393b0ea9c7e4b0582937783050b50`: byte-for-byte layout-aware
snapshots now cover common and worktree Git directories' `HEAD`, loose `refs`,
optional `packed-refs`, and reflogs (including absence/presence and symlinks)
after both initial and reset/recomputed worker merges.
The ticket remains open. Task 2 has not started; native five-target behavior
and all orchestration/recovery work remain pending. See the Task 1 comments and
execution ledger for exact commands, isolation observations and residual gaps.

Task 1 validation remediation is committed at
`e5a3ab2a8c5a4b6e39149c25bbe07844c4393422`: the linked-worktree test now
passes its configured `WorktreeAddOptions` by immutable reference to
`Repository::worktree`, resolving `clippy::unnecessary_mut_passed` without
altering its ODB/mempack or conflict assertions. Required Devenv formatting,
merge-test (9 tests), all-target/all-feature clippy, and diff checks passed.

## Tasks 0–3 checkpoint

Task 0 baseline/preflight is committed at `456b509`. Task 1's private mempack
isolation and Task 2's ordered durable evidence, including their review fixes,
are committed through `50fea1c`. Task 3's initial ordered integration/conflict
recovery source and verification are `3a9c197` and `03fe4d6`; its subsequent
P1 remediation is ready for a local checkpoint commit.

Task 3 now persists a candidate before preparation, reconciles an owned
applying candidate on exact old/candidate HEAD only, re-fetches before final
candidate-envelope completion, and refuses any third-head mismatch. It
rechecks frozen configuration/tracking/clean-target inputs under the mutation
lease. Conflict side reads bind opaque tokens to the current operation/index
state, reject unsafe paths and all non-regular stage modes before materializing
blobs, and never read `current` from the worktree (preventing symlink follow).

Final independent review reports no blocking findings. Required local Devenv
validation passed: all-feature check, formatting, all-target/all-feature
clippy with warnings denied, all-feature tests (including 103 SSH cases), and
the headless CLI smoke test. One P2 test-depth note remains: no separate public
SSH failure-point test stops exactly while a candidate child is `Applying`; the
controller-level restart regression covers the mandatory fresh fetch and
finalization transition. Ticket remains open. Tasks 4–7, native five-target
verification, code-review/PR approval, delivery, and closure remain pending.

## Task 4 approved protocol revision — 2026-10-08

Task 3 remediation checkpoint was `6bd1f18`, now `51fe6f2` after the
conflict-free current-main rebase; Task 4 is still unaccepted. The explicitly
unaccepted preservation WIP is `45a139b`, based on main `ceb1be4`.
Owner approved cooperative-writer concurrency, operator recovery for
interrupted/failed libgit2 ref/reflog effects (including partial logs and
ambiguous locks), and the
[revised protocol](../../../docs/plans/2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md).
Resume sequential subagent implementation and independent review locally.
Existing agent-owned work was preserved and rebased without Rust/Cargo tree
changes. See the ledger for accepted-task mapping; do not replay Tasks 0–3.
No validation/native waiver or delivery, closure, or cleanup authority was granted.

Milestone 1 stable-sentinel/evidence/libgit2-serialization changes have independent
review **OK with notes**, with no delivered-seam blockers. Check/fmt/strict clippy,
150 remote and 9/50/73 reservation/foundation/enablement tests passed. This is
milestone evidence only; Task 4 stays unaccepted and uncommitted beyond its WIP.
Milestone 2a canonical write-observation replay and all-side closure/immutable
validation also received independent review OK with notes, with 70 sync, 9 merge
and 112 local-authoring tests plus check/fmt/strict clippy passing. Milestone 2b
stock-API characterization is independently reviewed, with seven new tests and
163 remote tests passing. Owner then approved broader operator-controlled
backend ref/log recovery; automatic log repair/rewriting remains forbidden.
Implement baseline/complete-image proof for safe same-operation retry, preserving
uncertain effects rather than blindly reappending. Milestone2c writer timed out
at 30 minutes without an attributed handoff; partial changes are preserved, not
accepted. Owner requested pause/fresh session; see
`docs/plans/2026-10-08-wave-02-cycle-06-task-04-handoff.md`. Use targeted tests and
longer future validation budgets. Native helpers, real process death and final
gates remain blocking. See ledger and milestone
comments for exact evidence; no whole-task acceptance.

## Entry Gate

### Current implementation checkpoint — 2026-10-09

Tasks 0–4 are accepted (Task 4 source a68eba7, native run
[37877620596](https://github.com/vhodges/manyhands/actions/runs/37877620596)).
Task 5 is implemented and independently reviewed in four milestones; its
acceptance rests on the exact-source native run recorded in the execution ledger.
Tasks 6/7 remain pending. Owner directs best-effort robustness; targeted local
tests of changed code precede each native dispatch. Ticket remains
open; no PR/main merge/closure/cleanup authority. See execution ledger for evidence.

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current.

## Exit Evidence

Real remote divergence tests prove clean merges retain both histories, conflicts
retain the affected worktree and markers, resolution retry does not repeat
completed network or merge steps, and no local work is lost.
