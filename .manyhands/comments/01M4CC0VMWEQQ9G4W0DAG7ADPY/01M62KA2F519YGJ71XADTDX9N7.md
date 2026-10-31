---
manyhands_managed: true
manyhands_kind: comment
id: "01M62KA2F519YGJ71XADTDX9N7"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M62DYZWDC8G9QQQ5CVJCE2TV"
created_at: "2026-10-29T02:02:41Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2b complete; the task review is pending

- **`comment add` is bound.** `Mutation::CommentAdd(CommentAddInput)`,
  command `"comment add"`, with the Command Predicates table's values. The
  binding calls `submit_comment_with` with the full input and
  `release_unwritten: true`, and with `restart` on re-entry. Its synchronizer
  runs the child synchronization through item sync's own call, with
  `MERGING` and a `Watch` whose subject and owner are the `comment add`
  request, so the child's progress, cancellation, unsettled push, fence and
  conflict data are item sync's.
- **Answers and standing (R-6, R-34).** Published, current, or no remote is
  `ok`. A stopped publication gives the child's code, owed, except a final
  child or `remote_branch_deleted`, which finish. `identity_required` keeps
  the record. The data is the new `CommentMutationDto`, with the
  `comment_mutation` schema. `comment_synchronization` resolves either ID
  through the binding (R-5).
- **The author (moved from Task 2a).** `CommentAuthor::parse` checks R-7's
  form. A given author is written verbatim and once; a commit identity is
  still needed.
- **Named edits.** E-6, E-7, E-8, E-9 and E-10, as the design words them.
  `the_stopping_commands_and_their_stages_and_confirmation` is renamed now.
- **Verification.** There are 27 new `c4_comment_*` SSH cases, one per table
  row, plus `MERGING`, Review Focus 1 and 5, `request_mismatch` and the
  carried no-context identity case. There are also 2 new
  `c4_comment_domain_*` cases. `mutation_remote` passed 148,
  `comment_publication` 41, `mutation_replay` 383, `mutation_bridges` 129,
  `mutation_contract` 44, `read_contract` 72 and `--lib mutation` 123.
  `clippy` and `fmt --check` pass. `origin/main` is still `a372edb`, and
  `merge-tree` is clean.
- **For a ruling.** After `remote_branch_deleted` the finished record leaves
  the child's reservation held. `request cancel` then asks nothing, and a
  new `item sync` is `busy`. The ledger's Task 2b findings give this and
  four smaller ones.
