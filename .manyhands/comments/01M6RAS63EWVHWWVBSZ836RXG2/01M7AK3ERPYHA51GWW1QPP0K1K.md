---
id: 01M7AK3ERPYHA51GWW1QPP0K1K
item_id: 01M6RAS63EWVHWWVBSZ836RXG2
created_at: 2026-11-13T13:56:09.976771474Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 04 Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`, `vh-dt0aq`) takes this ticket's split of `src/desktop/model/operations.rs` (Cycle decision 3; approved 2026-11-13, not yet implemented).

**The split.**
- The lifecycle answers and lock bookkeeping move to `model/operations/lifecycle.rs`, beside new `progress.rs`, `recovery.rs` and `pending.rs`.
- Every item is re-exported, so no path changes (03b design F1).
- `view/operations.rs` loses its drawing: Activity is rebuilt as one list of rows, and only the action handlers stay (design F4).

**Left on this ticket:**
- the drain move to `services/drain.rs`;
- path-keyed lifecycle locks;
- library L1 and L4;
- the harness waits and fixtures, and the 30 s sleep in `view/operations_tests.rs`;
- the copy, comment and doc items;
- K-28 to K-33;
- the worker-thread leak.

**Check after 03b merges:**
- Per-row `show_conflict` caching may be moot once the old entries are gone.
- K-28 still applies: Activity's Retry of a failed plain item save or comment resubmits without the lifecycle-lock check. The Retry now comes from a row's button or menu.
