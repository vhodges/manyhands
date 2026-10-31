---
manyhands_managed: true
manyhands_kind: comment
id: "01M64AGF41CC4ZXHNQV6QZHQ7P"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M63W9P4E23MX26BHBJQY25T0"
created_at: "2026-10-29T17:43:33Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5 complete; the task review is pending

- **`item republish` (decisions 4, 7 and 11).** It is a confirmed command
  that pushes an item's remotely deleted branch again, at exactly the local
  commit its preview showed. It integrates nothing.
  - The preview reads what is stored, with no reservation and no
    connection (fact 55). That is a `remotely_deleted` fetch record, or the
    push evidence `item sync` classifies, now one shared function.
  - Only a recorded deletion gives a preview. Anything else is the no-op.
  - Execution reserves as a synchronization of the context (fact 78). It
    fetches, proves the branch absent at both endpoints, pushes the
    consented commit, verifies it and records the publication.
  - It leaves out the push-absence boundary and the per-record refusal.
    It keeps the reservation's checks, the unfinished local sync among
    them (fact 80).
- **Refusals.**
  - A change since the preview, or a push the remote refused, ends the
    operation with nothing on the remote. The record is finished, and a new
    preview is offered.
  - A transport, unlock or host stop leaves the operation for the same
    request, which verifies a push that landed rather than pushing again.
  - A lifecycle-closed ticket is `ticket_closed` (decision 17).
- **Resume.** `operation resume` on an operation an `item republish` request
  holds answers `original_request_required`, naming that request (P3).
- **Other changes.**
  - The desktop arm shows "Branch", "Last published" and "Local commit".
  - The CLI gains its two preview arms.
  - New schema `item_republish_preview`; the effect `branch_republished`.
  - Named edits E-5 to E-9.
  - The owner-authorized conformance split: one case per row.
- **Verification.**
  - 23 `c4_republish` cases and 16 `c4_resume`; the whole `mutation_remote`
    target (see the ledger's Task 5 entry).
  - `mutation_confirmation` 183, `mutation_bridges` 129, `mutation_contract`
    44, `read_contract` 72.
  - `--lib` 123, 477, 164 and 26.
  - clippy and fmt; the desktop gate, at main's 58 errors exactly.
  - `origin/main` is still `a372edb`, and `merge-tree` is clean.
- **For review.** These are in the report and the ledger:
  - a refused push finishes the record;
  - a refused republish's operation reads `cancelled`;
  - push-only evidence cannot tell a present branch from a deleted one;
  - the busy and no-op messages are human output, for Part B.
