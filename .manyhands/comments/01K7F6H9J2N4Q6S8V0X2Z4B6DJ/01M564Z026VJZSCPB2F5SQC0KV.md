---
manyhands_managed: true
manyhands_kind: comment
id: "01M564Z026VJZSCPB2F5SQC0KV"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T00:00:41Z"
---

Task 4 checkpoint: the closure checkpoint commit, its integration and the
post-checkpoint path. The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Checkpoint.** `Close ticket <ULID>` is built in the ODB, unreferenced,
  from the one shared `closed_ticket_bytes` with the mode kept. `closed_at`
  is whole UTC seconds and is also the commit time, so a rebuild makes the
  same commit. The context moves in `fast_forward` order, and refresh
  required is marked.
- **Failure points.** `ClosureAfterClosingRecorded`,
  `ClosureDuringCheckpointCheckout` and `AfterClosureCheckpoint` are at the
  design's positions. A test shows each one leaves the matching
  reconciliation row.
- **C9 resume.** It runs before any consent check. Authority comes from the
  checkpoint commit, never from configuration. The base is the current
  primary (D2). The merge is in memory even with a remote, and a conflict
  is `RecoveryRequired` with the record kept.
- **Seam.** `prepare_integration_for` is generic over expectation and error
  (R-T1-6). The runner reloads the record after a checkpoint and integrates
  the merged context (R-T1-4). Promotion never returns a checkpoint, so its
  behavior is unchanged.
- **Gate.** fmt and Clippy are clean. `--lib closure` passes 108 (31 new), `--lib promotion` 197 and `remote_reservation` 13. `document_promotion` passes 74 SSH cases, the same as before.
- **To review.** `adopt_written_closure` updates `base_oid` and
  `closure_oid` while `closing`, for the C9 base and an adopted descendant.
  The design does not cover this.
