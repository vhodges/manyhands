---
id: 01M7V8NT3T405PN74G4RAJHK4J
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-20T01:21:02.795366785Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint 4, 2026-11-19. The implementation plan `docs/plans/2026-11-19-fast-writes-implementation.md` is at `b68db56c` on this branch, beside the approved design. Nothing has been built or run; only documents changed.

**Research:** five read-only agents mapped the index storage and reads, the write path and holds, activity, transport and sync, and lifecycle and registry audits before the plan was written.

**Plan review rounds:** 1 on `97e4dc6c` (source and feasibility; contract and consistency): 7 blocking, 25 important, about 19 minor. 2 on `476f5529` (scoped, adversarial on P-3): 2, 9, 9. 3 on `c5c5c5ab` (scoped): 1, 8, about 11. The round-3 fixes in this commit have had no independent review; the hold rule, which drew findings in every round, is now stated as Table B itself and enumerated by one test, and Task 3's checkpoint has a fresh reviewer check the Table A and B tests before any code.

**Proposed for the owner's approval with the plan:** planning amendments P-1 (cleanup checks: one full check inside removal as the design says, plus a cheap check before the remote delete), P-2 (reads keep today's newer-file staleness rule, against the context's own time), P-3 (a write's index step observes outside the lease and stores under it after a basis re-check), P-4 (a sync's hand-off keeps the `refresh` action with a per-item `synchronization-index-v1/` target), P-5 (previews add no blockers); owner questions 1–4 (approve P-1–P-5; a primary sync's hand-off row as the primary's own item, with close, promotion and folder create keeping repository scope as a second exception to decision 10; the measurement script's scratch-only setup; named edits E-1–E-11).

**Proposed execution method:** subagent-driven, task by task with a reviewer per task, because the tasks hand interfaces forward (basis, observation, store, holds) and a mistake in recovery or holds is costly.