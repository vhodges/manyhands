---
manyhands_managed: true
manyhands_kind: comment
id: "01M561TBM3WZ13YE2FMAS112KM"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-17T23:05:43Z"
---

Task 3 checkpoint: the closure preflight and preview. The details are in
the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Preview.** `preview_ticket_closure` returns `Ready(TicketClosurePreview)`
  or `Blocked { blockers, identity_configuration }`. It takes no lease and
  makes no network call, SQL write, ref change or worktree change. All 21
  blockers are reported in fixed order. `AlreadyClosed` replaces
  `ContextMissing`. `IdentityRequired` is reported alone and only for a
  `Record` closure.
- **Observation.** It follows the integration table, checks C12 pending
  local work and comment publications (also under the lease), and selects
  the identity with effective configuration winning (C2). The digest uses
  `manyhands closure v1` and binds `closed_by`, the `existing` `closed_at`
  text and the T2-1 change set. D6 excludes moving OIDs when there is a
  remote.
- **`closed_ticket_bytes`** is complete and shared by preview and execution
  (C10). It compares after canonical normalization (R-T0-2).
- **Seam.** The shared promotion observation helpers were extracted with no
  behavior change: `--lib promotion` still passes 197 and
  `document_promotion` passes 74 SSH cases.
- **Gate.** 52 new observation tests (`--lib closure::` 67); fmt and Clippy
  are clean.
- **Choices to review.** The guard scope stays `.manyhands/tickets`. D5 is
  skipped for contained contexts. A comment's own unfinished local row
  reports both C12 blockers. The execution-side public types are left to
  Task 6.
