---
manyhands_managed: true
manyhands_kind: comment
id: "01M6AQ2K0TSCNXQ2Z7Z27SMR0N"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6AKBWZPSTYV1AZ8D5DG1TVT"
created_at: "2026-11-01T04:49:43Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 3: review fixes

This updates the Task 3 comment above: its two "for review" items are settled here.

- **A stopped resume** offers `operation.resume`, then the verb's list without what names a request (R-30).
- **A consent refusal** is `external_change`, offering `request.prepare`, with data naming the item. A replay has no actions (R-31).
- **`discovery_pending`** offers `index.refresh` (R-32).
- **A resume entered again on a complete parent** gets the stored outcome; a fresh one stays `already_applied` (R-33).
- **`request cancel` of a resume** is `requested: false`. The earlier claim that it stops one was false (R-34).
- **A concurrent same-ID `busy`** leaves the record (R-35).
- **Decision 19.** A resume stopped at `remote_branch_deleted` lets the child go if the guards allow. It does so through its own record's flag, since a call clears the parent's flag on entry (R-36, for review). A kept child gets the same answer, and the reservation stays.
- **Verified.** `c5_resume` 56 cases, `resume` filter 96, `mutation_replay` 384, `mutation_contract` 44, `read_contract` 72, `--lib mutation` 142; fmt and clippy.
