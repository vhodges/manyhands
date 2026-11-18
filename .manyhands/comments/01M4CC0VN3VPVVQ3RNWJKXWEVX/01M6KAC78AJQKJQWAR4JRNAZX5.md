---
manyhands_managed: true
manyhands_kind: comment
id: "01M6KAC78AJQKJQWAR4JRNAZX5"
item_id: "01M4CC0VN3VPVVQ3RNWJKXWEVX"
created_at: "2026-11-04T13:01:06Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## What Wave 03 C5 delivered for D6

C5 (`01M4CC0VMXTNXNH78C11DYXS8H`) merged to main at `2d912d10` on 2026-11-04. Track rule 2, as C5 amended it, has D6 wait for that merge, so it is now satisfied. Read C5's Cycle (`docs/Cycles/wave-03-cli-05-completion-and-polling.md`), the design's "Polling Verbs" section and the RFC's "C5 evidence and known limits" before planning.

- **Polling bindings at the shared mutation boundary:** `PollOnce`, `PollConfigure`, `PollPause`, `PollResume`. D6 consumes these, not `set_remote_polling`, which now has test callers only.
- **Policy writes (decision 15):** each verb writes one field in the request's own transaction. Last commit wins: a policy request that never committed is a new write when resent.
- **`poll resume` (decision 11):** clears the pause and `next_eligible_at`, so an eligible poll is due. It leaves the recovery suspension and the stored backoff value alone.
- **Recovery suspension (decision 5):** left to D6. `resume_remote_polling_after_recovery` has no caller and no CLI verb, and D6 owns clearing it.
- **A held lease (decision 24):** a poll's fetch that meets a held repository lease answers `busy` and interrupts its row, with no stored outcome and no backoff. For automatic polls nothing restarts such a row, so D6's scheduler must.
- **Stops (R-16, R-18):** a completed poll with dirty, diverged, malformed or deleted targets is `ok`, with dispositions in data and `PollTargetDisposition::guidance` for display. A transport stop after reservation is final, and a new request polls again.
- **Already-contained branches (decision 28):** a remote context whose tip primary already holds is reported `current` and not materialized.
- **Worker (`src/desktop/services/worker.rs:790`):** a policy verb reports `writing` only and never `accepted`, so the "not yet accepted" branch must handle one-transaction commands.
- **Known limits:** `poll once` holds the remote reservation, so it is `busy` beside other remote operations. A domain-level automatic poll's plan failure raises the generation, which can strand a retained conflict (see the RFC's limits).
