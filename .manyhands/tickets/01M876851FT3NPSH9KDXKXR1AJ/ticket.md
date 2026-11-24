---
slug: vh-ssepw
deps:
- 01M87683QK8E34EYY8KS4VQG9Y
- 01M87684C9GM9BJP5QQ429KMAK
id: 01M876851FT3NPSH9KDXKXR1AJ
title: 'Wave 06 Cycle 05: After the merge: detection, merged_at, cleanup and surviving branches'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Wave 06 Cycle 05. Design: docs/RFC/pull-request-mode.md (approved 2026-11-24, merged as 0db9124d), section "After the merge" (including "When it reached the primary" and "Checkout not on the primary"); decisions 10, 11, 13, 14 and 22. Design ticket: vh-k12ka. Depends on Cycles 03 and 04.

Scope:

- `poll once` reports items that became done on the primary, and for a ticket the dependents made ready.
- `merged_at`: the committer time of the first primary commit that holds an item's done state, taken only from the commits a poll fast-forwards over, stored in the registry and kept across an index rebuild. Direct mode fills it from its own integration commit. It is shown on a ticket's closure and on a promoted document.
- A confirmed deletion of a remote branch that survives the merge, guarded by the observed tip (related: vh-ew8pt).
- A confirmed manual cleanup when the content proof fails for an item that is done, with a preview of what the primary does not hold.
- A warning from `poll once` and `repo inspect` when the registered checkout is not on the primary.

Acceptance: a merge commit, a squash and a fast-forward each end with the item done and cleaned up, in one table-driven test per lifecycle; `merged_at` comes from the fast-forward range with no other history walk, and is absent in a clone that never saw the item arrive; a squash after which the primary moved on ends in the confirmed manual cleanup.

Plan it with the planning-a-cycle skill against current main. Tests follow AGENTS.md's Testing Strategy: library tests first, a local remote for CLI contracts, and no new SSH case without a written reason, since the transport does not change.