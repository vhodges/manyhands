---
manyhands_managed: true
manyhands_kind: comment
id: "01M62DYZWDC8G9QQQ5CVJCE2TV"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M626HSQ75577337XJN16EQNR"
created_at: "2026-10-29T00:14:36Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2a complete; the task review is pending

- **Synchronizer.** Comment submission and retry have crate-private variants,
  `submit_comment_with` and `retry_comment_publication_with`. Each takes a
  synchronizer closure, which makes the publication's one domain call. The
  submission variant also takes `restart` and `release_unwritten`. The public
  `submit_comment` and `retry_comment_publication` pass a closure over
  `synchronize_remote` and are unchanged. A synchronizer that makes no call
  leaves the publication pending as `recovery_required`.
- **The binding's semantics (`release_unwritten`).** A parent that is not a
  comment of the item, or that is the comment itself, is refused before a
  binding row is inserted. On `IdentityRequired` with nothing written, one
  registry transaction deletes the binding and marks the row `rejected`. The
  same operation then binds afresh, even after another checkpoint lands. A
  written file keeps the binding. The public path keeps today's order and
  standing, as fact 73 requires.
- **`Watch::new`** is a `pub(super)` constructor in `bind/sync.rs`, and the
  synchronization binding now uses it. Task 2b builds the `comment add`
  watch with it.
- **Rulings.** The controller moved the comment author to Task 2b: its type,
  validation, parameter and domain cases. Nothing in this task would call
  them, so the library's dead-code lint would fail. The controller also kept
  the early parent check off the public path.
- **Verification.** There are five new `c4_comment_domain_*` SSH cases. The
  `mutation_remote -- c4_comment_domain` run passed 5;
  `comment_publication` 41; `remote_merge_recovery` 25; `--lib repository`
  1344. `clippy --all-targets --all-features` and `fmt --check` pass.
  `origin/main` is still `a372edb`, and `merge-tree` is clean.
