---
id: 01M6W9BGDGSYZWE2Y95674N6YS
item_id: 01M66HFV44YXNQ7ANJVA16TQ1H
created_at: 2026-11-08T00:36:25.307346958Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**Implementation started (Task 0 checkpoint), 2026-11-07.**

- The owner approved the design and the implementation plan on 2026-11-07.
  Execution is subagent-driven: one Sonnet 5.5 implementer per task, then
  an independent reviewer. The ledger is
  `docs/plans/2026-11-07-remote-history-repair-execution.md`.
- Preflight: origin/main `d70142b6` was already merged. Branch head
  `1a1b2c55`.
- Baseline is green:
  - `cargo check`;
  - `remote_polling poll_history_unknown_after_registry_loss` (one case, by
    exact name);
  - the key-source fence case in `remote::poll::tests`;
  - `remote::state::persistence_tests` (68);
  - `remote::refs::tests` (10).
- All four test-only `history_unknown=0` resets are dead (checked with a
  temporary overflow guard, then reverted). Task 11 removes them.
- Plan correction: the lib test filters it names don't match. The ledger
  lists the real module paths.
