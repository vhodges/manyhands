---
manyhands_managed: true
manyhands_kind: comment
id: "01M56MQQ0SWWSD05G2GJSG7E5W"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T04:36:19Z"
---

Task 6B checkpoint: the consolidation and the guard tightenings are done.
The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Shared record logic.** The table-agnostic parent code that promotion and
  closure duplicated now lives in `promotion/lifecycle_record.rs`. Each
  table keeps its schema, steps, invariants, insert and save. Promotion's
  results and errors are unchanged, and no caller or older test changed.
- **Delete guard (R-T5-2).** The tracking-OID fallback applies only when no
  context attempt was minted. A minted attempt that never reserved stops
  cleanup with `RecoveryRequired` before any delete.
- **Context evidence (R-T5-3).** A closure records the tracking OID only
  with the context synchronization.
- **Poll materialization.** This was reachable. A closure without a remote
  delete holds no slot, so at `branch_deleted` a poll rebuilt the ticket's
  worktree and branch after a collaborator's push. A poll now refuses that
  with `BlockedByLocalState` until the closure is retired.
- **Gate.** fmt and Clippy are clean. `--lib closure` passes 168 (+6),
  `promotion` 199, `remote::reservation` 54, `cleanup` 47,
  `recovery` 50, `transport` 38 and `remote::poll` 79.
  `document_promotion` (74), `remote_reservation` (13) and
  `remote_polling` (84) pass.
- **To review.** After the fenced poll, the same ID stops
  `RecoveryRequired` at `branch_deleted`, because the tracking ref it
  would retire changed. Nothing is rebuilt or lost.
