---
manyhands_managed: true
manyhands_kind: comment
id: "01M55T9XS2SXV322TXSDYD9RZG"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-17T20:54:24Z"
---

Task 1 checkpoint: the lifecycle seam, with no behavior change. The details
are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Seam.** The new `promotion/lifecycle.rs` holds `LifecycleKind`
  (`DocumentPromotion`, `TicketClosure`), the `Lifecycle` trait (one method
  per record, observe and integrate call the runner makes, plus the
  `pre_integration` hook) and `DocumentPromotionLifecycle`, which forwards to
  the unchanged promotion functions.
- **Kind threading.** The runner, integration (context ref, subject-keyed
  reconciliation), observation (guard directory and ref), `ChildExecution`,
  `finish_parent` and `restart_remote_lifecycle` all take the kind. A
  `TicketClosure` kind reaching any of them returns `recovery_required` until
  its own task.
- **Gate.** All 13 baseline targets keep their Task 0 counts. Promotion
  gains 3 new lifecycle tests. Clippy and fmt are clean. No existing test
  changed.
- **Open.** The runner's bound still pins the promotion record, step and
  error types. Task 6 must lift them.
- **Ledger.** The Task 0 count is corrected (12 `record::` functions over 23
  sites), and named edit 5 is recorded.
