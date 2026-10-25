---
id: 01M5AMJS6653J5BH3XMRA3F1MS
item_id: 01M4R19P943QZKHHKRXV2D2TP2
created_at: 2026-10-19T17:54:32Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

## Implementation plan prepared

The owner requested a written implementation plan after agreeing to preserve
blocks for unrelated unfinished operations and reservations.

- [Implementation plan](../../../docs/plans/2026-10-19-missing-repository-registration-removal-implementation.md)

This defect has no assigned Wave/Cycle number. The single plan includes the
behavior contract, internal design, ordered tasks, acceptance map and decision
review. It is proposed and awaiting implementation authorization.

Refreshed the existing ticket worktree: fetched `origin/main` at
`021f68425db6d4be38bcfbee0ee151829e06df41`, then ran `git merge origin/main`.
Already up to date; before/after ticket HEAD remains
`cdd797cba21878eecb83beed6023c118be198256`. Ancestry check succeeded. The
previous investigation comment and scope decision were preserved.

### Planning rulings

- Missing-root removal uses the exact stored root. Fresh unknown missing
  paths retain their existing errors; matched persisted requests can replay
  after both directory and registration have gone.
- Keep the existing confirmation and schema. A private checked domain method
  rechecks the observed registration before deletion; public request structs
  remain source-compatible.
- Specify bootstrap → optional common-Git lease → exclusive cache guard.
  Reappearance while acquiring guards returns busy rather than reversing lock
  order. Existing per-lock timeouts remain; there is no internal retry loop.
- The new registry-only path conservatively preserves all unfinished evidence,
  including context synchronization, parent promotion/closure state and accepted
  requests not yet in a domain journal. The ordinary local-operation predicate
  alone does not cover all of these cases. Live-route policy stays unchanged.
- A Git NotFound error alone is insufficient for the fallback; distinguish
  positive absence from broken indirection, permission failures and corruption.

### Review and evidence limits

Audited source selection, scope/replay, confirmation, coordination, journal
preservation, rollback, platform fixtures, native-test coverage and lifecycle.
The plan names focused tests at each checkpoint and the four final Devenv
gates. Existing native targets already include all planned test targets;
dispatch remains separately authorized after merge on main.

One existing limitation is explicit for owner review: the current registration
identity is an integer row ID plus second-resolution `enabled_at`. SQLite can
reuse the ID, so a same-second replacement can repeat the stamp. The plan
preserves current semantics and adds the missing guarded comparison; it does
not claim to repair that separate ABA identity problem. A generation-token
migration would need a plan amendment.

No implementation or regression tests were added in this planning step.
Earlier diagnostic results remain investigation evidence, not fix verification.

### Approval and execution start — 2026-10-19T18:09:48Z

The owner approved the plan. Sequential implementation has started with
Tasks 0–2 as the first review batch. See the
[execution ledger](../../../docs/plans/2026-10-19-missing-repository-registration-removal-execution.md).
Fresh fetch/merge was a no-op at the same base and HEAD recorded above.
Focused baseline passed: repository enablement removal 2 tests, mutation
confirmation removal 8 tests, recovery registration removal 3 tests.
