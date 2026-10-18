---
manyhands_managed: true
manyhands_kind: comment
id: "01M56T5F94DBEE3E9FAFBKQMM1"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T06:11:13Z"
---

Task 8 checkpoint: closure is proven end to end over real SSH with two
clones. The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md).

- **Cases.** The new `ticket_closure` target has 72 SSH cases, all
  passing, plus 5 child-only output controls:
  - 23 journeys;
  - 21 blockers;
  - 6 checkpoint seams;
  - 2 races;
  - 17 exits;
  - 3 privacy cases.

  Every case is under 20 s. `document_promotion` still passes all 74 after
  named edit 4.
- **Acceptance mapping.**
  - Journeys:
    - the happy path, where B polls, sees the closed primary and reports
      nothing for the context, and the remote context never held the
      checkpoint;
    - local-only, then publication;
    - existing closure fields;
    - contained-and-closed cleanup, both by a collaborator and after cache
      loss with the remote branch gone;
    - contained-and-open closure with no context child;
    - a conflict, its resolution, then the same ID closing once;
    - a poll advancing `T(P)`;
    - a primary-only advance;
    - a changed remote context;
    - a rejected publication, then a retry;
    - a rejected deletion, then a retry;
    - a deletion race;
    - offline before and after integration;
    - a dirty, untracked, ignored or moved context;
    - poll priority;
    - a pending comment publication;
    - the C5 guard;
    - discoverability from both clones.
  - Every `ClosureBlocker`, each with full preservation snapshots.
  - The three checkpoint seams, each exited by a child process, then retried
    after an advanced or a conflicting primary.
  - The sequential and true (distinct identities) double closure.
  - F1–F12, the closure seams and both F10/F11 seams, with reopen, the
    identical retry and effect counters. The T6-1 lock case uses a
    fixture-only operator repair.
  - `closed_by` privacy, with positive and negative controls (checkpointed
    and pinned WAL).
- **Named edit 4.** The column-scoped scanner moved unchanged to
  `tests/support/privacy_rows.rs`. `document_promotion` calls it with its
  old policy.
- **No production change.** The 7 first-run failures were test-side and are
  recorded in the ledger. fmt and Clippy are clean. The workflow test list
  gains `--test ticket_closure` (not dispatched).
