---
manyhands_managed: true
manyhands_kind: comment
id: "01M62XBG3E9FFXQR2BP08C1CSM"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M62KA2F519YGJ71XADTDX9N7"
created_at: "2026-10-29T04:05:40Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 3 complete; the task review is pending

- **The read.** `RepositoryService::show_conflict(repo, operation_id)` is
  in `remote/conflict_read.rs` and returns `ConflictDto`, with
  `ConflictPathDto` and the design's enums. Three more contract enums
  close the target, stage and eligibility. It takes a synchronization's ID
  or a comment's operation (R-5). A local sync's ID, an operation that is
  no synchronization, and a synchronization with no recorded conflict are
  `invalid_input`. An unknown ID is `operation_not_found`.
- **States.** The library's inspection gives a live conflict's observation
  and tokens. `next_action` is `resolve` exactly when `conflict_next`
  says so. At `ResolutionPrepared` the inspection still issues the same
  observation, tokens and sides (fact 86 holds), with `retry_resolution`
  and `historical: true`. Resolved, cancelled and complete come from the
  journal. When Git no longer holds the recorded conflict, an outside
  repair is judged read-only, under the lease, by fact 35's rule:
  `repaired` and `resume`, or else `repair_refused` and `external_repair`.
  `request_id` stays null until Task 4a.
- **Paths.** Kind and structure come from `classify_conflict_entry`
  (`incompatible_closures` included). Sides come from the library's side
  read, so they are there only where it reads text. Display paths follow
  R-11. String tokens follow R-9, through `pub(crate)` `path_tokens` and
  `observation_text`.
- **Contract.** `conflict` and `conflict_path` schemas are added, with
  `conflict` as a data root (E-7) and the enums in
  `contract_enumerations` (E-8). Three read goldens are added:
  `conflict_show_conflict_pending`, `_repaired` and `_complete`.
- **Verification.**
  - 21 `c4_conflict_read_*` SSH cases pass: one per table row, plus R-5,
    R-12 (the registry's bytes are unchanged), Review Focus 2 (hostile
    path), R-11 (display path) and R-40.
  - `read_contract` 72, `mutation_contract` 44, `--lib remote` 477,
    `--lib local_sync` 164 and `--lib integration` 26 pass, as do clippy
    and `fmt --check`.
  - `origin/main` is still `a372edb`, and `merge-tree` is clean.
- **For review.** Five decisions are in the execution ledger's Task 3
  entry. One is a known gap: the repair judgement does not mirror the
  restart's check for leftover Git locks.
